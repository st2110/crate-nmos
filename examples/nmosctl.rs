//! A small controller: find the Nodes on this network and say what they hold.
//!
//! ```text
//! cargo run --example nmosctl -- list [SECONDS]
//! ```
//!
//! The other side of `examples/node.rs`, and of this crate. Where the Node
//! composes documents, this reads them: it browses `_nmos-node._tcp` over mDNS,
//! takes the API versions a Node advertises in its TXT record, negotiates one
//! both ends understand, and fetches the resource tree.
//!
//! It is deliberately small. A real controller — jackfield, in this project's
//! case — keeps the inventory in memory, follows changes, and draws a screen.
//! What is worth taking from here is the shape of the client: discover, agree a
//! version, read, and report each Node's failure without giving up on the rest.

use std::collections::BTreeMap;
use std::time::Duration;

use mdns_sd::{ServiceDaemon, ServiceEvent};
use nmos::{ApiVersion, Device, NodeApiClient};

/// The service type every NMOS Node advertises.
const SERVICE_TYPE: &str = "_nmos-node._tcp.local.";
/// How long to listen before reporting, unless told otherwise.
const DEFAULT_SECONDS: u64 = 4;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let command = arguments.next().unwrap_or_else(|| "list".to_owned());
    let seconds = arguments
        .next()
        .and_then(|argument| argument.parse().ok())
        .unwrap_or(DEFAULT_SECONDS);

    match command.as_str() {
        "list" => list(Duration::from_secs(seconds)).await,
        other => {
            eprintln!("nmosctl: no such command `{other}`");
            eprintln!("usage: nmosctl list [SECONDS]");
            // Exit rather than return an error: `main` would print its `Debug`
            // underneath the usage line, and a person asking for help does not
            // need it repeated as a Rust value.
            std::process::exit(2);
        }
    }
}

/// Find every Node that answers, and describe it.
async fn list(listen_for: Duration) -> Result<(), Box<dyn std::error::Error>> {
    let found = discover(listen_for).await?;
    if found.is_empty() {
        println!("no NMOS Nodes answered in {listen_for:?}");
        return Ok(());
    }

    let client = NodeApiClient::builder()
        .user_agent(concat!("nmosctl/", env!("CARGO_PKG_VERSION")))
        .request_timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(2))
        .build()?;

    for node in found {
        // One unreachable Node must not cost the others their line: an operator
        // asking what is on the network needs the whole answer, including which
        // part of it is broken.
        match client.fetch_tree(&node.base, &node.versions).await {
            Ok(tree) => {
                println!("{} — {} ({})", name_of(&tree.node), node.base, tree.version);
                for device in &tree.devices {
                    println!("    {}", describe(device, &tree.senders, &tree.receivers));
                }
            }
            Err(reason) => println!("{} — {} — {reason}", node.name, node.base),
        }
    }
    Ok(())
}

/// What to call a Node on screen.
///
/// The library has no such function on purpose: which of a Node's several names
/// to show is a judgement about an audience, not a fact about the protocol.
/// Labels repeat across equipment, hostnames are absent as often as not, and
/// the identifier is unreadable — so the choice, and the blame for it, belong to
/// whoever is doing the displaying.
fn name_of(node: &nmos::Node) -> String {
    if !node.core.label.is_empty() {
        return node.core.label.clone();
    }
    node.hostname
        .clone()
        .unwrap_or_else(|| node.core.id.to_string())
}

/// A Node as the network described it, before anything was read from it.
struct Found {
    name: String,
    base: String,
    versions: Vec<ApiVersion>,
}

/// Browse for Nodes until the time is up.
async fn discover(listen_for: Duration) -> Result<Vec<Found>, Box<dyn std::error::Error>> {
    let daemon = ServiceDaemon::new()?;
    let events = daemon.browse(SERVICE_TYPE)?;

    // Keyed by name so that a Node answering twice is one Node.
    let mut found: BTreeMap<String, Found> = BTreeMap::new();
    let deadline = tokio::time::Instant::now() + listen_for;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, events.recv_async()).await {
            Ok(Ok(ServiceEvent::ServiceResolved(service))) => {
                if let Some(node) = read_announcement(&service) {
                    found.insert(node.name.clone(), node);
                }
            }
            // Other events say a Node appeared or went away; only a resolved
            // one carries an address to talk to.
            Ok(Ok(_)) => {}
            Ok(Err(_)) | Err(_) => break,
        }
    }
    Ok(found.into_values().collect())
}

/// What an announcement says, or nothing if it says too little to use.
fn read_announcement(service: &mdns_sd::ResolvedService) -> Option<Found> {
    let properties = &service.txt_properties;
    // A Node that does not say how to speak to it is assumed to speak plain
    // HTTP: that guess is what every controller makes, and saying nothing is
    // far more common than saying `https`.
    let scheme = match properties.get_property_val_str("api_proto") {
        Some("https") => "https",
        _ => "http",
    };
    let versions: Vec<ApiVersion> = properties
        .get_property_val_str("api_ver")?
        .split(',')
        .filter_map(|version| version.trim().parse().ok())
        .collect();
    if versions.is_empty() {
        return None;
    }

    let address = service
        .addresses
        .iter()
        .map(|address| address.to_ip_addr())
        .find(std::net::IpAddr::is_ipv4)
        .or_else(|| {
            service
                .addresses
                .iter()
                .map(|address| address.to_ip_addr())
                .next()
        })?;

    Some(Found {
        name: service.fullname.clone(),
        base: format!("{scheme}://{address}:{}", service.port),
        versions,
    })
}

/// One line about a Device: what it has.
///
/// Counted from the Senders and Receivers themselves rather than from the
/// Device's own lists. Both should agree, and when they do not it is the
/// resources that exist.
fn describe(device: &Device, senders: &[nmos::Sender], receivers: &[nmos::Receiver]) -> String {
    let sending = senders
        .iter()
        .filter(|sender| sender.device_id == device.core.id)
        .count();
    let receiving = receivers
        .iter()
        .filter(|receiver| receiver.device_id == device.core.id)
        .count();
    format!(
        "{} — {sending} sender{}, {receiving} receiver{}",
        device.core.label,
        plural(sending),
        plural(receiving)
    )
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
