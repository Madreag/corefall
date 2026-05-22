//! M8B § TURN relay fallback.
//!
//! Per M8B Notes: "The TURN relay is a deliberate operational dependency.
//! Self-hosted servers (M42) MUST be able to operate without it; the
//! M42 docs include 'running without TURN' guidance for community
//! hosts. The first-party launch infra includes a TURN cluster."

use serde::{Deserialize, Serialize};

use crate::nat::{NatTraversalMethod, NatTraversalPath};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRelayClient {
    pub turn_server_addr: String,
    pub turn_server_port: u16,
    pub username: String,
    pub realm: String,
}

impl TurnRelayClient {
    pub fn new(addr: &str, port: u16, username: &str, realm: &str) -> Self {
        Self {
            turn_server_addr: addr.to_string(),
            turn_server_port: port,
            username: username.to_string(),
            realm: realm.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnRelayOutcome {
    pub method: NatTraversalMethod,
    pub path: NatTraversalPath,
    pub relayed_address: String,
    pub relayed_port: u16,
    pub elapsed_ms: u32,
}

impl TurnRelayClient {
    /// allocate a relayed address; in test scenarios the caller
    /// supplies the allocation outcome.
    pub fn allocate<F>(&self, mut allocate_fn: F) -> TurnRelayOutcome
    where
        F: FnMut(&TurnRelayClient) -> (String, u16, u32),
    {
        let (addr, port, elapsed) = allocate_fn(self);
        TurnRelayOutcome {
            method: NatTraversalMethod::TurnRelay,
            path: NatTraversalPath::Relay,
            relayed_address: addr,
            relayed_port: port,
            elapsed_ms: elapsed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_allocation_returns_relay_outcome() {
        let client = TurnRelayClient::new("turn.corefall.example", 3478, "anon", "corefall");
        let outcome = client.allocate(|_| ("198.51.100.7".into(), 50000, 800));
        assert!(matches!(outcome.method, NatTraversalMethod::TurnRelay));
        assert!(matches!(outcome.path, NatTraversalPath::Relay));
        assert_eq!(outcome.relayed_address, "198.51.100.7");
        assert_eq!(outcome.relayed_port, 50000);
        assert_eq!(outcome.elapsed_ms, 800);
    }
}
