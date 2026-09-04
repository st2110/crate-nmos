//! What a Sender and a Receiver are each doing, in the project's own words.
//!
//! IS-04 says `active` at both ends. That word stops here. A Sender that is
//! putting its Flow on the network and a Receiver that is taking a stream are
//! different facts, and neither of them means "connected" — only a *pair* is
//! connected, and answering that needs every Node, which is the engine's job,
//! not this crate's. See `DESIGN.md`, "Vocabulary".

use std::fmt;

/// What a Sender is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Transmission {
    /// Putting its Flow on the network.
    ///
    /// Says nothing about whether anyone is listening: on the bench, three
    /// Senders reported this while nothing on the network took their streams.
    Transmitting,

    /// Not transmitting.
    Idle,
}

impl Transmission {
    /// Whether the Sender is putting its Flow on the network.
    #[must_use]
    pub fn is_transmitting(self) -> bool {
        matches!(self, Transmission::Transmitting)
    }
}

impl fmt::Display for Transmission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Transmission::Transmitting => "transmitting",
            Transmission::Idle => "idle",
        })
    }
}

/// What a Receiver is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Reception {
    /// Taking a stream. Which Sender it takes may still be unresolved — that is
    /// a separate question, answered by the engine across the whole network.
    Subscribed,

    /// Taking nothing.
    Unsubscribed,
}

impl Reception {
    /// Whether the Receiver is taking a stream.
    #[must_use]
    pub fn is_subscribed(self) -> bool {
        matches!(self, Reception::Subscribed)
    }
}

impl fmt::Display for Reception {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Reception::Subscribed => "subscribed",
            Reception::Unsubscribed => "unsubscribed",
        })
    }
}
