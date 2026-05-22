//! M8B § NAT punch-through — ICE-lite + STUN discovery + TURN relay
//! fallback + parallel candidate-pair connectivity check.
//!
//! Per M8B Notes for the implementer:
//!
//! - ICE-lite (not full ICE) per IETF RFC 5245: the cf-net client is
//!   the controlling agent; the cf-net server is the controlled agent
//!   and always has a server-reflexive candidate. This is the simplest
//!   workable profile for a client-server game; full ICE is overkill.
//! - The TURN relay is a deliberate operational dependency. Self-hosted
//!   servers (M42) MUST be able to operate without it; the M42 docs
//!   include "running without TURN" guidance for community hosts.

pub mod candidate_pair;
pub mod ice_lite;
pub mod stun_client;
pub mod turn_relay;

pub use candidate_pair::{CandidatePair, CandidatePairOutcome, PairConnectivityCheck};
pub use ice_lite::{IceLiteAgent, IceLiteOutcome, IceRole, NatBehavior};
pub use stun_client::{StunBindingResponse, StunClient};
pub use turn_relay::{TurnRelayClient, TurnRelayOutcome};

use serde::{Deserialize, Serialize};

/// have 4 seconds to find a working pair before TURN relay engages.
pub const ICE_LITE_TIMEOUT_MS: u32 = 4000;

/// Method tag for `nat_traversal_outcome` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatTraversalMethod {
    IceLite,
    TurnRelay,
}

impl NatTraversalMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IceLite => "ice_lite",
            Self::TurnRelay => "turn_relay",
        }
    }
}

/// Path tag for `nat_traversal_outcome` events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NatTraversalPath {
    Direct,
    Relay,
}

impl NatTraversalPath {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Relay => "relay",
        }
    }
}

pub const JOIN_TIME_TARGET_MS: u32 = 6000;
