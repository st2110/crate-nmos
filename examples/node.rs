//! A Node that announces itself, serves IS-04, and can be connected over IS-05.
//!
//! ```text
//! cargo run --example node -- [PORT] [ADDRESS]
//! ```
//!
//! Give it the address of the interface this Node lives on. Without one it
//! announces every interface the mDNS daemon can see, which on a machine with a
//! VPN or a virtual bridge includes addresses nothing can reach — and a
//! controller that picks one of those sees a Node that will not answer. Real
//! products ask the operator which interface to use for exactly this reason.
//!
//! It exposes two Devices — one that receives video, audio and metadata, one
//! that sends them — announces `_nmos-node._tcp` over mDNS, and accepts the
//! `PATCH` a controller uses to connect a Receiver.
//!
//! Serving HTTP and announcing over mDNS are not the library's job: it models
//! the protocol and leaves the plumbing to whoever is doing the plumbing. What
//! is worth reading here is [`apply`], where a patch meets the four states of
//! [`Param`] — that is the part no test can explain as clearly.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{Json, Router, routing::get};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use nmos::is05::{
    Activation, ActivationMode, Constraint, Param, ReceiverStagedPatch,
    ReceiverTransportParamsPatch, SenderStagedPatch, SenderTransportParamsPatch, TransportFile,
};
use nmos::{
    ApiEndpoint, Component, ComponentName, Control, Device, Flow, FlowCore, InterlaceMode,
    MediaCaps, Node, NodeApi, Protocol, Rate, Receiver, ReceiverCaps, ReceiverSubscription,
    Reception, ResourceCore, ResourceId, Sender, SenderSubscription, Source, SourceCore,
    Transmission, Version, VideoCore,
};
use serde_json::{Value, json};

/// The Node API version served here.
const VERSION: &str = "v1.3";
/// The Connection API version served here.
const CONNECTION_VERSION: &str = "v1.1";
/// The service type every NMOS Node advertises.
const SERVICE_TYPE: &str = "_nmos-node._tcp.local.";
/// The name this Node answers to on the local network.
const HOSTNAME: &str = "example-node.local.";
/// RTP over multicast, which is what ST 2110 uses.
const TRANSPORT: &str = "urn:x-nmos:transport:rtp.mcast";
/// The control a controller looks for to find the Connection API.
const CONTROL_KIND: &str = "urn:x-nmos:control:sr-ctrl/v1.1";
/// The interface this Node would receive and send on.
const INTERFACE_IP: [u8; 4] = [10, 0, 0, 1];

const NODE_ID: &str = "0a1b2c3d-0000-4000-8000-000000000001";
const INGEST_ID: &str = "0a1b2c3d-0000-4000-8000-000000000002";
const PLAYOUT_ID: &str = "0a1b2c3d-0000-4000-8000-000000000003";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(8080u16);
    let announced: Option<std::net::IpAddr> = std::env::args()
        .nth(2)
        .and_then(|argument| argument.parse().ok());
    // Whatever the operator named, or the Node's mDNS name when they named
    // nothing. Guessing by the default route is the one thing that must not
    // happen: on a machine with a VPN that route is a tunnel, and the Node ends
    // up published at an address no controller can open.
    let authority = announced.map_or_else(
        || HOSTNAME.trim_end_matches('.').to_owned(),
        |address| address.to_string(),
    );
    let base = format!("http://{authority}:{port}");

    let model = Arc::new(Model::new(&base)?);
    let announcement = announce(&model, port, announced)?;

    let listener = tokio::net::TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await?;
    println!("Node API   {base}/x-nmos/node/{VERSION}/");
    println!("Connection {base}/x-nmos/connection/{CONNECTION_VERSION}/");
    match announced {
        Some(address) => println!("announced as {SERVICE_TYPE} on {address}"),
        None => println!(
            "announced as {SERVICE_TYPE} on every interface — pass an address to \
             narrow it if a controller cannot reach this Node"
        ),
    }
    println!("press ctrl-c to stop");

    axum::serve(listener, routes(Arc::clone(&model))).await?;
    drop(announcement);
    Ok(())
}

/// Everything this Node has to say about itself.
struct Model {
    node: Node,
    devices: Vec<Device>,
    sources: Vec<Source>,
    flows: Vec<Flow>,
    /// The Senders and Receivers as they were declared. Their `subscription` is
    /// not kept here — see [`Model::receivers_document`].
    senders: Vec<Sender>,
    receivers: Vec<Receiver>,
    /// The only mutable thing in this Node.
    ///
    /// Both APIs are views of it: IS-05 serves the documents almost verbatim,
    /// IS-04 projects the part of them it describes. Nothing is stored twice,
    /// so nothing can disagree.
    ///
    /// Where this state lives is the application's business. The library models
    /// the documents and has no opinion about who holds them, which is why a
    /// `BTreeMap` behind a `Mutex` is enough here and a real Node would put the
    /// same facts in its own configuration.
    connections: Mutex<BTreeMap<String, Connection>>,
}

/// The `staged` and `active` documents of one Sender or Receiver.
#[derive(Clone)]
struct Connection {
    staged: Endpoint,
    active: Endpoint,
}

/// One IS-05 document.
#[derive(Clone)]
struct Endpoint {
    master_enable: bool,
    /// The resource at the other end: a Sender for a Receiver, and the reverse.
    peer: Option<ResourceId>,
    activation: Activation,
    /// Receivers carry the SDP they were given; Senders publish theirs instead.
    transport_file: Option<TransportFile>,
    params: Legs,
}

/// The transport parameters of one resource, which differ by direction.
#[derive(Clone)]
enum Legs {
    Receiver(Vec<ReceiverTransportParamsPatch>),
    Sender(Vec<SenderTransportParamsPatch>),
}

impl Endpoint {
    fn receiver() -> Self {
        Self {
            master_enable: false,
            peer: None,
            activation: Activation::default(),
            transport_file: Some(TransportFile::sdp(None)),
            params: Legs::Receiver(vec![ReceiverTransportParamsPatch {
                interface_ip: Param::Set(INTERFACE_IP.into()),
                // Nothing joined yet. `Null` rather than absent: the parameter
                // exists and has no value, which is not the same as unmentioned.
                multicast_ip: Param::Null,
                source_ip: Param::Null,
                destination_port: Param::Auto,
                rtp_enabled: Param::Set(true),
                ..ReceiverTransportParamsPatch::default()
            }]),
        }
    }

    fn sender() -> Self {
        Self {
            master_enable: false,
            peer: None,
            activation: Activation::default(),
            transport_file: None,
            params: Legs::Sender(vec![SenderTransportParamsPatch {
                source_ip: Param::Set(INTERFACE_IP.into()),
                destination_ip: Param::Null,
                destination_port: Param::Auto,
                rtp_enabled: Param::Set(true),
                ..SenderTransportParamsPatch::default()
            }]),
        }
    }

    /// The document as IS-05 writes it.
    fn document(&self, peer_field: &str) -> Value {
        let params = match &self.params {
            Legs::Receiver(legs) => serde_json::to_value(legs),
            Legs::Sender(legs) => serde_json::to_value(legs),
        }
        .unwrap_or(Value::Array(Vec::new()));

        let mut document = json!({
            "master_enable": self.master_enable,
            peer_field: self.peer.as_ref().map(ToString::to_string),
            "activation": self.activation,
            "transport_params": params,
        });
        if let Some(file) = &self.transport_file
            && let Some(object) = document.as_object_mut()
        {
            object.insert(
                "transport_file".to_owned(),
                serde_json::to_value(file).unwrap_or(Value::Null),
            );
        }
        document
    }
}

/// Where a patch meets the four states of a transport parameter.
///
/// This is the whole reason [`Param`] is not `Option<Option<T>>`:
///
/// * [`Param::Absent`] — the controller did not mention the field, so it keeps
///   whatever it had;
/// * [`Param::Null`] — the controller cleared it;
/// * [`Param::Auto`] — the controller handed the decision to this Node;
/// * [`Param::Set`] — use this.
///
/// Only the first is special here. The other three are stored as they arrived,
/// and that is not fussiness: `destination_port` may be a number or `"auto"`
/// and the schema forbids `null` outright, so a Node that folded `auto` into
/// "no value" would answer with a document its own specification rejects.
///
/// [`Param::value`] does perform that fold, for callers that only want to know
/// whether a value was imposed. Storing state is not one of those callers.
fn apply<T>(current: &mut Param<T>, patch: Param<T>) {
    if !patch.is_absent() {
        *current = patch;
    }
}

fn apply_receiver_leg(
    current: &mut ReceiverTransportParamsPatch,
    patch: ReceiverTransportParamsPatch,
) {
    apply(&mut current.multicast_ip, patch.multicast_ip);
    apply(&mut current.source_ip, patch.source_ip);
    apply(&mut current.destination_port, patch.destination_port);
    apply(&mut current.rtp_enabled, patch.rtp_enabled);
    // `interface_ip` is deliberately not applied. Which wire a multicast group
    // arrives on belongs to this machine, and a controller naming somebody
    // else's address would not make the traffic appear there. The constraints
    // say so too: one permitted value, and it is ours.
}

fn apply_sender_leg(current: &mut SenderTransportParamsPatch, patch: SenderTransportParamsPatch) {
    apply(&mut current.destination_ip, patch.destination_ip);
    apply(&mut current.destination_port, patch.destination_port);
    apply(&mut current.rtp_enabled, patch.rtp_enabled);
}

impl Model {
    fn new(base: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let now = Version::now()?;
        let node_id: ResourceId = NODE_ID.parse()?;
        let ingest_id: ResourceId = INGEST_ID.parse()?;
        let playout_id: ResourceId = PLAYOUT_ID.parse()?;

        // The format a Receiver accepts is carried by its capabilities rather
        // than by a field of its own, which is why these differ by variant.
        let receivers: Vec<Receiver> = [
            (
                "video",
                "0011",
                ReceiverCaps::Video {
                    caps: media_caps(&["video/raw"]),
                },
            ),
            (
                "audio",
                "0012",
                ReceiverCaps::Audio {
                    caps: media_caps(&["audio/L24", "audio/L16"]),
                },
            ),
            (
                "metadata",
                "0013",
                ReceiverCaps::Data {
                    caps: media_caps(&["video/smpte291"]),
                },
            ),
        ]
        .into_iter()
        .map(|(what, tail, caps)| {
            Ok(Receiver {
                core: core(&resource_id(tail)?, &format!("Ingest {what}"), now),
                device_id: ingest_id.clone(),
                transport: TRANSPORT.to_owned(),
                interface_bindings: vec!["eth0".to_owned()],
                subscription: ReceiverSubscription::new(Reception::Unsubscribed, None),
                caps,
            })
        })
        .collect::<Result<_, nmos::ParseError>>()?;

        let sources = sources(&playout_id, now)?;
        let flows = flows(&playout_id, &sources, now)?;

        let senders: Vec<Sender> = ["video", "audio", "metadata"]
            .into_iter()
            .zip(["0021", "0022", "0023"])
            .zip(&flows)
            .map(|((what, tail), flow)| {
                Ok(Sender {
                    core: core(&resource_id(tail)?, &format!("Playout {what}"), now),
                    caps: BTreeMap::new(),
                    flow_id: Some(flow.core().core.id.clone()),
                    transport: TRANSPORT.to_owned(),
                    device_id: playout_id.clone(),
                    manifest_href: Some(format!("{base}/sdp/{tail}.sdp")),
                    interface_bindings: vec!["eth0".to_owned()],
                    subscription: SenderSubscription::new(Transmission::Idle, None),
                })
            })
            .collect::<Result<_, nmos::ParseError>>()?;

        // Both Devices advertise the Connection API. Without this a controller
        // finds the resources and has no way to ask that they be connected.
        let control = Control {
            href: format!("{base}/x-nmos/connection/{CONNECTION_VERSION}/"),
            kind: CONTROL_KIND.to_owned(),
            authorization: false,
        };
        let devices = vec![
            Device {
                core: core(&ingest_id, "Ingest", now),
                kind: "urn:x-nmos:device:generic".to_owned(),
                node_id: node_id.clone(),
                senders: Vec::new(),
                receivers: receivers.iter().map(|r| r.core.id.clone()).collect(),
                controls: vec![control.clone()],
            },
            Device {
                core: core(&playout_id, "Playout", now),
                kind: "urn:x-nmos:device:generic".to_owned(),
                node_id: node_id.clone(),
                senders: senders.iter().map(|s| s.core.id.clone()).collect(),
                receivers: Vec::new(),
                controls: vec![control],
            },
        ];

        // Every field is spelled out. `Node` has no `Default` on purpose: a Node
        // without an identity or an address is not a lesser Node, it is not one.
        let (host_text, host_port) = split_authority(base);
        let node = Node {
            core: core(&node_id, "nmos example node", now),
            href: format!("{base}/"),
            hostname: Some(HOSTNAME.trim_end_matches('.').to_owned()),
            api: NodeApi {
                versions: vec!["v1.2".to_owned(), VERSION.to_owned()],
                endpoints: vec![ApiEndpoint {
                    host: host_text,
                    port: host_port,
                    protocol: Protocol::Http,
                    authorization: false,
                }],
            },
            caps: BTreeMap::new(),
            services: Vec::new(),
            clocks: Vec::new(),
            interfaces: Vec::new(),
        };

        let mut connections = BTreeMap::new();
        for receiver in &receivers {
            let endpoint = Endpoint::receiver();
            connections.insert(
                receiver.core.id.to_string(),
                Connection {
                    staged: endpoint.clone(),
                    active: endpoint,
                },
            );
        }
        for sender in &senders {
            let endpoint = Endpoint::sender();
            connections.insert(
                sender.core.id.to_string(),
                Connection {
                    staged: endpoint.clone(),
                    active: endpoint,
                },
            );
        }

        Ok(Self {
            node,
            devices,
            sources,
            flows,
            senders,
            receivers,
            connections: Mutex::new(connections),
        })
    }
}

/// One Source per thing this Node sends.
fn sources(device_id: &ResourceId, now: Version) -> Result<Vec<Source>, nmos::ParseError> {
    let core_of = |tail: &str, label: &str| -> Result<SourceCore, nmos::ParseError> {
        Ok(SourceCore {
            core: core(&resource_id(tail)?, label, now),
            grain_rate: Some(Rate {
                numerator: 50,
                denominator: 1,
            }),
            caps: BTreeMap::new(),
            device_id: device_id.clone(),
            parents: Vec::new(),
            clock_name: None,
        })
    };
    Ok(vec![
        Source::Video {
            core: core_of("0031", "Playout video source")?,
        },
        Source::Audio {
            core: core_of("0032", "Playout audio source")?,
            channels: vec![
                nmos::Channel {
                    label: "Left".to_owned(),
                    symbol: Some("L".to_owned()),
                },
                nmos::Channel {
                    label: "Right".to_owned(),
                    symbol: Some("R".to_owned()),
                },
            ],
        },
        Source::Data {
            core: core_of("0033", "Playout metadata source")?,
            event_type: None,
        },
    ])
}

/// One Flow per Source, which is what a Sender actually carries.
fn flows(
    device_id: &ResourceId,
    sources: &[Source],
    now: Version,
) -> Result<Vec<Flow>, nmos::ParseError> {
    let core_of =
        |tail: &str, label: &str, source: &Source| -> Result<FlowCore, nmos::ParseError> {
            Ok(FlowCore {
                core: core(&resource_id(tail)?, label, now),
                source_id: source.id().clone(),
                device_id: device_id.clone(),
                parents: Vec::new(),
                grain_rate: Some(Rate {
                    numerator: 50,
                    denominator: 1,
                }),
            })
        };
    let (video, audio, data) = match sources {
        [video, audio, data] => (video, audio, data),
        _ => return Ok(Vec::new()),
    };
    Ok(vec![
        Flow::VideoRaw {
            core: core_of("0041", "Playout video flow", video)?,
            video: VideoCore {
                frame_width: 1920,
                frame_height: 1080,
                interlace_mode: InterlaceMode::Progressive,
                colorspace: "BT709".to_owned(),
                transfer_characteristic: Some("SDR".to_owned()),
            },
            media_type: "video/raw".parse()?,
            components: vec![
                component(ComponentName::Y, 1920, 1080),
                component(ComponentName::Cb, 960, 1080),
                component(ComponentName::Cr, 960, 1080),
            ],
        },
        Flow::AudioRaw {
            core: core_of("0042", "Playout audio flow", audio)?,
            sample_rate: Rate {
                numerator: 48_000,
                denominator: 1,
            },
            media_type: "audio/L24".parse()?,
            bit_depth: Some(24),
        },
        Flow::Data {
            core: core_of("0043", "Playout metadata flow", data)?,
            media_type: "video/smpte291".parse()?,
        },
    ])
}

fn component(name: ComponentName, width: i64, height: i64) -> Component {
    Component {
        name,
        width,
        height,
        bit_depth: 10,
    }
}

/// The fields every resource carries.
fn core(id: &ResourceId, label: &str, version: Version) -> ResourceCore {
    ResourceCore {
        id: id.clone(),
        version,
        label: label.to_owned(),
        description: String::new(),
        tags: BTreeMap::new(),
    }
}

/// The media types a Receiver will take.
fn media_caps(media_types: &[&str]) -> MediaCaps {
    MediaCaps {
        media_types: media_types
            .iter()
            .filter_map(|text| text.parse().ok())
            .collect(),
        ..MediaCaps::default()
    }
}

/// Split `http://example-node.local:8080` into its host and port.
fn split_authority(base: &str) -> (String, u16) {
    let authority = base.trim_start_matches("http://");
    match authority.rsplit_once(':') {
        Some((host, port)) => (host.to_owned(), port.parse().unwrap_or(80)),
        None => (authority.to_owned(), 80),
    }
}

/// # Errors
///
/// Returns an error if the identifier does not parse. It cannot happen with the
/// literals above, but an example that reaches for `expect` teaches the habit
/// this crate exists to avoid.
fn resource_id(tail: &str) -> Result<ResourceId, nmos::ParseError> {
    format!("0a1b2c3d-0000-4000-8000-00000000{tail}").parse()
}

/// What a controller may put in a Receiver's transport parameters.
fn receiver_constraints() -> Value {
    let interface = std::net::IpAddr::from(INTERFACE_IP).to_string();
    json!([{
        "source_ip": Constraint::default(),
        "multicast_ip": Constraint::default(),
        // One permitted value, and it is ours: which wire the traffic arrives
        // on is this machine's business.
        "interface_ip": Constraint::one_of([interface]),
        "destination_port": Constraint::port(),
        "rtp_enabled": Constraint::default(),
    }])
}

fn sender_constraints() -> Value {
    let interface = std::net::IpAddr::from(INTERFACE_IP).to_string();
    json!([{
        "source_ip": Constraint::one_of([interface]),
        "destination_ip": Constraint::default(),
        "destination_port": Constraint::port(),
        "rtp_enabled": Constraint::default(),
    }])
}

/// The Node API and the Connection API, as far as a controller needs them.
fn routes(model: Arc<Model>) -> Router {
    let node = format!("/x-nmos/node/{VERSION}");
    let connection = format!("/x-nmos/connection/{CONNECTION_VERSION}");

    let mut router = Router::new()
        .route("/x-nmos/", get(|| async { Json(["connection/", "node/"]) }))
        .route(
            "/x-nmos/node/",
            get(move || async move { Json([format!("{VERSION}/")]) }),
        )
        .route(
            "/x-nmos/connection/",
            get(move || async move { Json([format!("{CONNECTION_VERSION}/")]) }),
        );

    // IS-04. Sources and Flows are no longer empty: a Sender that carries
    // nothing is not a Sender a controller can reason about.
    for (path, body) in [
        ("self", serde_json::to_value(&model.node)),
        ("devices", serde_json::to_value(&model.devices)),
        ("sources", serde_json::to_value(&model.sources)),
        ("flows", serde_json::to_value(&model.flows)),
    ] {
        let body = body.unwrap_or(Value::Null);
        // Both spellings: the specification writes trailing slashes and clients
        // do not always send them.
        for suffix in ["", "/"] {
            let body = body.clone();
            router = router.route(
                &format!("{node}/{path}{suffix}"),
                get(move || async move { Json(body) }),
            );
        }
    }
    for path in ["senders", "receivers"] {
        for suffix in ["", "/"] {
            router = router.route(
                &format!("{node}/{path}{suffix}"),
                // Read at request time, not at start-up: these change when a
                // controller connects something.
                get(move |State(model): State<Arc<Model>>| async move {
                    Json(if path == "senders" {
                        model.senders_document()
                    } else {
                        model.receivers_document()
                    })
                }),
            );
        }
    }

    router = router.route(
        &format!("{node}/"),
        get(|| async {
            Json([
                "self/",
                "devices/",
                "senders/",
                "receivers/",
                "sources/",
                "flows/",
            ])
        }),
    );

    // IS-05.
    let senders: Vec<String> = model
        .senders
        .iter()
        .map(|sender| format!("{}/", sender.core.id))
        .collect();
    let receivers: Vec<String> = model
        .receivers
        .iter()
        .map(|receiver| format!("{}/", receiver.core.id))
        .collect();
    router = router
        .route(
            &format!("{connection}/"),
            get(|| async { Json(["bulk/", "single/"]) }),
        )
        .route(
            &format!("{connection}/single/"),
            get(|| async { Json(["receivers/", "senders/"]) }),
        )
        .route(
            &format!("{connection}/single/senders/"),
            get(move || async move { Json(senders) }),
        )
        .route(
            &format!("{connection}/single/receivers/"),
            get(move || async move { Json(receivers) }),
        );

    for kind in ["senders", "receivers"] {
        router = router
            .route(
                &format!("{connection}/single/{kind}/{{id}}/"),
                get(|| async { Json(["active/", "constraints/", "staged/", "transporttype/"]) }),
            )
            .route(
                &format!("{connection}/single/{kind}/{{id}}/transporttype"),
                get(|| async { Json(TRANSPORT) }),
            )
            .route(
                &format!("{connection}/single/{kind}/{{id}}/constraints"),
                get(move || async move {
                    Json(if kind == "senders" {
                        sender_constraints()
                    } else {
                        receiver_constraints()
                    })
                }),
            )
            .route(
                &format!("{connection}/single/{kind}/{{id}}/active"),
                get(read_active),
            )
            .route(
                &format!("{connection}/single/{kind}/{{id}}/staged"),
                get(read_staged).patch(patch_staged),
            );
    }

    router.with_state(model)
}

fn peer_field(id: &str, model: &Model) -> &'static str {
    if model
        .senders
        .iter()
        .any(|sender| sender.core.id.to_string() == id)
    {
        "receiver_id"
    } else {
        "sender_id"
    }
}

impl Model {
    /// The Receivers, with their subscription taken from the live connection.
    ///
    /// This is the projection that keeps the two APIs honest. The subscription
    /// is not stored anywhere: it is what the `active` document means, said in
    /// the vocabulary IS-04 uses, and computed at the moment somebody asks.
    fn receivers_document(&self) -> Value {
        let Ok(connections) = self.connections.lock() else {
            return Value::Array(Vec::new());
        };
        let receivers: Vec<Value> = self
            .receivers
            .iter()
            .map(|receiver| {
                let mut receiver = receiver.clone();
                if let Some(connection) = connections.get(&receiver.core.id.to_string()) {
                    // The schema is explicit: the identifier is set only while
                    // the subscription is active, and null otherwise. Stating
                    // that once, here, is the advantage of projecting rather
                    // than storing — every writer would otherwise have to
                    // remember it.
                    let (reception, peer) = if connection.active.master_enable {
                        (Reception::Subscribed, connection.active.peer.clone())
                    } else {
                        (Reception::Unsubscribed, None)
                    };
                    receiver.subscription = ReceiverSubscription::new(reception, peer);
                }
                serde_json::to_value(receiver).unwrap_or(Value::Null)
            })
            .collect();
        Value::Array(receivers)
    }

    /// The Senders, likewise.
    fn senders_document(&self) -> Value {
        let Ok(connections) = self.connections.lock() else {
            return Value::Array(Vec::new());
        };
        let senders: Vec<Value> = self
            .senders
            .iter()
            .map(|sender| {
                let mut sender = sender.clone();
                if let Some(connection) = connections.get(&sender.core.id.to_string()) {
                    let (transmission, peer) = if connection.active.master_enable {
                        (Transmission::Transmitting, connection.active.peer.clone())
                    } else {
                        (Transmission::Idle, None)
                    };
                    sender.subscription = SenderSubscription::new(transmission, peer);
                }
                serde_json::to_value(sender).unwrap_or(Value::Null)
            })
            .collect();
        Value::Array(senders)
    }
}

async fn read_active(State(model): State<Arc<Model>>, Path(id): Path<String>) -> impl IntoResponse {
    document(&model, &id, |connection| &connection.active)
}

async fn read_staged(State(model): State<Arc<Model>>, Path(id): Path<String>) -> impl IntoResponse {
    document(&model, &id, |connection| &connection.staged)
}

fn document(
    model: &Model,
    id: &str,
    pick: impl Fn(&Connection) -> &Endpoint,
) -> (StatusCode, Json<Value>) {
    let Ok(connections) = model.connections.lock() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "state is poisoned",
            )),
        );
    };
    match connections.get(id) {
        Some(connection) => (
            StatusCode::OK,
            Json(pick(connection).document(peer_field(id, model))),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(error(StatusCode::NOT_FOUND, "no such resource")),
        ),
    }
}

/// Take a controller's patch, and activate it if that is what was asked.
///
/// The whole of IS-05 writing is here: parse, apply, decide. A patch that
/// cannot be honoured leaves no trace — the specification requires the state to
/// be unchanged after a rejection, and a controller that got 200 for a patch it
/// did not get is worse than one that got an error.
async fn patch_staged(
    State(model): State<Arc<Model>>,
    Path(id): Path<String>,
    body: String,
) -> impl IntoResponse {
    let is_sender = peer_field(&id, &model) == "receiver_id";

    let Ok(mut connections) = model.connections.lock() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "state is poisoned",
            )),
        );
    };
    let Some(connection) = connections.get(&id).cloned() else {
        return (
            StatusCode::NOT_FOUND,
            Json(error(StatusCode::NOT_FOUND, "no such resource")),
        );
    };
    let mut staged = connection.staged.clone();

    let mode = if is_sender {
        match serde_json::from_str::<SenderStagedPatch>(&body) {
            Ok(patch) => {
                if let Some(enable) = patch.master_enable {
                    staged.master_enable = enable;
                }
                if let Some(peer) = patch.receiver_id.value() {
                    staged.peer = peer;
                }
                if let (Legs::Sender(legs), Some(incoming)) =
                    (&mut staged.params, patch.transport_params)
                {
                    for (leg, patch) in legs.iter_mut().zip(incoming) {
                        apply_sender_leg(leg, patch);
                    }
                }
                patch.activation.and_then(|activation| activation.mode)
            }
            Err(reason) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(error(StatusCode::BAD_REQUEST, &reason.to_string())),
                );
            }
        }
    } else {
        match serde_json::from_str::<ReceiverStagedPatch>(&body) {
            Ok(patch) => {
                if let Some(enable) = patch.master_enable {
                    staged.master_enable = enable;
                }
                if let Some(peer) = patch.sender_id.value() {
                    staged.peer = peer;
                }
                if let Some(file) = patch.transport_file {
                    staged.transport_file = Some(TransportFile::sdp(file.data));
                }
                if let (Legs::Receiver(legs), Some(incoming)) =
                    (&mut staged.params, patch.transport_params)
                {
                    for (leg, patch) in legs.iter_mut().zip(incoming) {
                        apply_receiver_leg(leg, patch);
                    }
                }
                patch.activation.and_then(|activation| activation.mode)
            }
            Err(reason) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(error(StatusCode::BAD_REQUEST, &reason.to_string())),
                );
            }
        }
    };

    match mode {
        // Scheduling needs a clock shared with the controller and a queue of
        // pending edits. Refusing plainly beats accepting and forgetting.
        Some(
            ActivationMode::ActivateScheduledAbsolute | ActivationMode::ActivateScheduledRelative,
        ) => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(error(
                    StatusCode::NOT_IMPLEMENTED,
                    "this Node activates immediately or not at all",
                )),
            );
        }
        Some(ActivationMode::ActivateImmediate) => {
            let Ok(at) = Version::now() else {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "this Node has no usable clock",
                    )),
                );
            };
            staged.activation = Activation::immediate(at);
            let mut active = staged.clone();
            // In the resource itself the mode goes out once it has happened;
            // only the time remains.
            active.activation = staged.activation.settled();
            staged.activation = staged.activation.settled();
            connections.insert(
                id.clone(),
                Connection {
                    staged: staged.clone(),
                    active: active.clone(),
                },
            );
        }
        // `ActivationMode` is `#[non_exhaustive]`: a mode this Node has never
        // heard of is refused rather than guessed at.
        Some(_) => {
            return (
                StatusCode::NOT_IMPLEMENTED,
                Json(error(
                    StatusCode::NOT_IMPLEMENTED,
                    "unrecognised activation mode",
                )),
            );
        }
        None => {
            connections.insert(
                id.clone(),
                Connection {
                    staged: staged.clone(),
                    active: connection.active,
                },
            );
        }
    }

    let field = peer_field(&id, &model);
    (StatusCode::OK, Json(staged.document(field)))
}

/// The error body IS-05 specifies. The code inside it must match the status of
/// the response carrying it, or the two disagree about what happened.
fn error(status: StatusCode, message: &str) -> Value {
    json!({ "code": status.as_u16(), "error": message, "debug": Value::Null })
}

/// Tell the network this Node is here.
fn announce(
    model: &Model,
    port: u16,
    announced: Option<std::net::IpAddr>,
) -> Result<ServiceDaemon, Box<dyn std::error::Error>> {
    let daemon = ServiceDaemon::new()?;
    let properties = [
        ("api_proto", "http".to_owned()),
        ("api_ver", format!("v1.2,{VERSION}")),
        ("api_auth", "false".to_owned()),
        // The counters a controller watches to notice a change without
        // re-reading the whole tree.
        ("ver_slf", "0".to_owned()),
        ("ver_dvc", model.devices.len().to_string()),
        ("ver_snd", model.senders.len().to_string()),
        ("ver_rcv", model.receivers.len().to_string()),
        ("ver_src", model.sources.len().to_string()),
        ("ver_flw", model.flows.len().to_string()),
    ];
    let addresses = announced
        .map(|address| address.to_string())
        .unwrap_or_default();
    let mut service = ServiceInfo::new(
        SERVICE_TYPE,
        "nmos-example-node",
        HOSTNAME,
        &addresses[..],
        port,
        &properties[..],
    )?;
    if announced.is_none() {
        // Nobody said which interface, so offer them all and hope. This is the
        // guess a real deployment replaces with a setting.
        service = service.enable_addr_auto();
    }
    daemon.register(service)?;
    Ok(daemon)
}
