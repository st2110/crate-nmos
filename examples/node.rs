//! A Node that announces itself and answers the IS-04 Node API.
//!
//! Run it and a controller on the same network will find it:
//!
//! ```text
//! cargo run --example node
//! ```
//!
//! It exposes two Devices — one that receives video, audio and metadata, one
//! that sends them — and serves the resources over HTTP while announcing
//! `_nmos-node._tcp` over mDNS.
//!
//! Serving HTTP and announcing over mDNS are not the library's job: it models
//! the protocol and leaves the plumbing to whoever is doing the plumbing. That
//! is what makes this worth reading — everything below is the plumbing, and the
//! resources it serves come straight out of `nmos`.

use std::collections::BTreeMap;
use std::net::SocketAddr;

use axum::{Json, Router, routing::get};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use nmos::{
    ApiEndpoint, Device, MediaCaps, Node, NodeApi, Protocol, Receiver, ReceiverCaps,
    ReceiverSubscription, Reception, ResourceCore, ResourceId, Sender, SenderSubscription,
    Transmission, Version,
};

/// The API version served here.
const VERSION: &str = "v1.3";
/// The service type every NMOS Node advertises.
const SERVICE_TYPE: &str = "_nmos-node._tcp.local.";
/// The name this Node answers to on the local network.
const HOSTNAME: &str = "example-node.local.";
/// RTP over multicast, which is what ST 2110 uses.
const TRANSPORT: &str = "urn:x-nmos:transport:rtp.mcast";

/// Identifiers are fixed rather than random so that a restart does not look
/// like a different Node to whoever is watching.
const NODE_ID: &str = "0a1b2c3d-0000-4000-8000-000000000001";
const INGEST_ID: &str = "0a1b2c3d-0000-4000-8000-000000000002";
const PLAYOUT_ID: &str = "0a1b2c3d-0000-4000-8000-000000000003";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let port = std::env::args()
        .nth(1)
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(8080u16);
    // The address is the daemon's business, not ours. Picking one by the
    // default route sends the announcement down whatever the default route is
    // — on a machine with a VPN, that is a tunnel no multicast crosses, and the
    // Node is announced where nobody is listening.
    let base = format!("http://{}:{port}", HOSTNAME.trim_end_matches('.'));

    let model = Model::new(&base)?;
    let announcement = announce(&model, port)?;

    let address = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("this Node is at {base}/x-nmos/node/{VERSION}/");
    println!("announced as {SERVICE_TYPE} — a controller on this network will find it");
    println!("press ctrl-c to stop");

    axum::serve(listener, routes(model)).await?;
    drop(announcement);
    Ok(())
}

/// Everything this Node has to say about itself.
#[derive(Clone)]
struct Model {
    node: Node,
    devices: Vec<Device>,
    senders: Vec<Sender>,
    receivers: Vec<Receiver>,
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
                // Nothing is taking anything yet: this Node is idle until a
                // controller connects it.
                subscription: ReceiverSubscription::new(Reception::Unsubscribed, None),
                caps,
            })
        })
        .collect::<Result<_, nmos::ParseError>>()?;

        let senders: Vec<Sender> = [("video", "0021"), ("audio", "0022"), ("metadata", "0023")]
            .into_iter()
            .map(|(what, tail)| {
                Ok(Sender {
                    core: core(&resource_id(tail)?, &format!("Playout {what}"), now),
                    caps: BTreeMap::new(),
                    flow_id: None,
                    transport: TRANSPORT.to_owned(),
                    device_id: playout_id.clone(),
                    manifest_href: Some(format!("{base}/sdp/{tail}.sdp")),
                    interface_bindings: vec!["eth0".to_owned()],
                    subscription: SenderSubscription::new(Transmission::Idle, None),
                })
            })
            .collect::<Result<_, nmos::ParseError>>()?;

        let devices = vec![
            Device {
                core: core(&ingest_id, "Ingest", now),
                kind: "urn:x-nmos:device:generic".to_owned(),
                node_id: node_id.clone(),
                senders: Vec::new(),
                receivers: receivers.iter().map(|r| r.core.id.clone()).collect(),
                controls: Vec::new(),
            },
            Device {
                core: core(&playout_id, "Playout", now),
                kind: "urn:x-nmos:device:generic".to_owned(),
                node_id: node_id.clone(),
                senders: senders.iter().map(|s| s.core.id.clone()).collect(),
                receivers: Vec::new(),
                controls: Vec::new(),
            },
        ];

        // Every field is spelled out. `Node` has no `Default` on purpose: a
        // Node without an identity or an address is not a lesser Node, it is
        // not one at all.
        let (host_text, port) = split_authority(base);
        let node = Node {
            core: core(&node_id, "nmos example node", now),
            href: format!("{base}/"),
            hostname: Some("example-node".to_owned()),
            api: NodeApi {
                versions: vec!["v1.2".to_owned(), VERSION.to_owned()],
                endpoints: vec![ApiEndpoint {
                    host: host_text,
                    port,
                    protocol: Protocol::Http,
                    authorization: false,
                }],
            },
            caps: BTreeMap::new(),
            services: Vec::new(),
            clocks: Vec::new(),
            interfaces: Vec::new(),
        };

        Ok(Self {
            node,
            devices,
            senders,
            receivers,
        })
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

/// Split `http://10.0.0.1:8080` into its host and port.
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

/// The Node API, as far as a browsing controller needs it.
fn routes(model: Model) -> Router {
    let prefix = format!("/x-nmos/node/{VERSION}");
    let mut router = Router::new()
        .route("/x-nmos/", get(|| async { Json(["node/"]) }))
        .route(
            "/x-nmos/node/",
            get(move || async move { Json([format!("{VERSION}/")]) }),
        );

    for (path, body) in [
        ("self", serde_json::to_value(&model.node)),
        ("devices", serde_json::to_value(&model.devices)),
        ("senders", serde_json::to_value(&model.senders)),
        ("receivers", serde_json::to_value(&model.receivers)),
        // Nothing here makes Flows or Sources, so the collections are empty
        // rather than absent: a controller must find them and read nothing.
        ("sources", Ok(serde_json::Value::Array(Vec::new()))),
        ("flows", Ok(serde_json::Value::Array(Vec::new()))),
    ] {
        let body = body.unwrap_or(serde_json::Value::Null);
        // Both spellings, because the specification writes trailing slashes and
        // clients do not always send them.
        for suffix in ["", "/"] {
            let body = body.clone();
            router = router.route(
                &format!("{prefix}/{path}{suffix}"),
                get(move || async move { Json(body) }),
            );
        }
    }

    router.route(
        &format!("{prefix}/"),
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
    )
}

/// Tell the network this Node is here.
fn announce(model: &Model, port: u16) -> Result<ServiceDaemon, Box<dyn std::error::Error>> {
    let daemon = ServiceDaemon::new()?;
    let properties = [
        ("api_proto", "http".to_owned()),
        ("api_ver", format!("v1.2,{VERSION}")),
        ("api_auth", "false".to_owned()),
        // The counters a controller watches to notice a change without polling
        // the whole tree.
        ("ver_slf", "0".to_owned()),
        ("ver_dvc", model.devices.len().to_string()),
        ("ver_snd", model.senders.len().to_string()),
        ("ver_rcv", model.receivers.len().to_string()),
        ("ver_src", "0".to_owned()),
        ("ver_flw", "0".to_owned()),
    ];
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        "nmos-example-node",
        HOSTNAME,
        "",
        port,
        &properties[..],
    )?
    // Every interface the daemon can reach, rather than one this program
    // guessed at.
    .enable_addr_auto();
    daemon.register(service)?;
    Ok(daemon)
}
