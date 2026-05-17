//! M8B § STUN client — server-reflexive candidate discovery.
//!
//! ICE-lite needs a STUN service to discover the public-facing port the
//! NAT has bound for the agent. This module models the STUN BindingRequest
//! → BindingResponse exchange.
//!
//! No real network IO; instead, callers inject a `respond_with` callback
//! that produces the expected response. Production wires `quinn` + a
//! UDP socket to the STUN server here at M9+.

use serde::{Deserialize, Serialize};

/// Locked STUN bind-request identifier. M8B uses a per-agent random
/// transaction id; in tests we use a fixed value for determinism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StunTransactionId(pub [u8; 12]);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StunBindingResponse {
    pub transaction_id: StunTransactionId,
    pub mapped_address: String,
    pub mapped_port: u16,
}

pub struct StunClient {
    pub server_addr: String,
    pub server_port: u16,
}

impl StunClient {
    pub fn new(server_addr: &str, server_port: u16) -> Self {
        Self {
            server_addr: server_addr.to_string(),
            server_port,
        }
    }

    /// **M8B § ICE-lite**: discover the server-reflexive candidate via
    /// STUN. The callback simulates the response (production swaps it
    /// for a real UDP exchange).
    pub fn discover<F>(&self, tx_id: StunTransactionId, mut respond_with: F) -> StunBindingResponse
    where
        F: FnMut(StunTransactionId, &str, u16) -> StunBindingResponse,
    {
        respond_with(tx_id, &self.server_addr, self.server_port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stun_discover_round_trips_transaction_id() {
        let client = StunClient::new("stun.corefall.example", 3478);
        let tx = StunTransactionId([0u8; 12]);
        let response = client.discover(tx, |tx, _addr, _port| StunBindingResponse {
            transaction_id: tx,
            mapped_address: "203.0.113.50".into(),
            mapped_port: 49001,
        });
        assert_eq!(response.transaction_id, tx);
        assert_eq!(response.mapped_address, "203.0.113.50");
        assert_eq!(response.mapped_port, 49001);
    }
}
