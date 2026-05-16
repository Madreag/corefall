//! M8A § cf-net::client — input prediction + reconciliation scaffold.
//!
//! Per M8A spec § Server authority model: client sends inputs only;
//! receives server snapshot deltas; reconciles. Triggers rollback on
//! prediction mismatch.

use serde::{Deserialize, Serialize};

/// **M8A § client**: net mode at connect time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetMode {
    /// LAN deterministic lockstep. All clients sim-step the same tick
    /// locally with identical inputs.
    Lockstep,
    /// Internet rollback. Client locally predicts forward N frames;
    /// server-confirmed inputs trigger rollback to first divergent
    /// frame + resim forward.
    #[default]
    Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub server_addr: String,
    pub server_port: u16,
    pub net_mode: NetMode,
}

#[derive(Debug)]
pub struct Client {
    pub config: ClientConfig,
}

impl Client {
    pub fn new(config: ClientConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_constructs_with_default_rollback_mode() {
        let cfg = ClientConfig {
            server_addr: "127.0.0.1".into(),
            server_port: 4040,
            net_mode: NetMode::default(),
        };
        let _client = Client::new(cfg.clone());
        assert_eq!(cfg.net_mode, NetMode::Rollback);
    }
}
