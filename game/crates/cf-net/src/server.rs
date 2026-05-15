//! M8A § cf-net::server — authoritative server loop scaffold.
//!
//! Per M8A spec § Server authority model: server is the only authority.
//! Client sends inputs only; server simulates and broadcasts canonical
//! events + snapshot deltas. cf-headless `serve` subcommand wires this
//! into a binary that runs with no Bevy render plugin.

use serde::{Deserialize, Serialize};

/// **M8A § server**: per-client bandwidth target (heavy combat p99).
pub const PER_CLIENT_BANDWIDTH_P99_KBPS: u32 = 50;

/// Server runtime configuration. M8A's cvars are additive to the cfctl
/// JSON-RPC surface (`srv.set_cvar / srv.get_cvar / srv.list_cvars`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub bind_addr: String,
    pub port: u16,
    pub scenario_path: String,
    pub snapshot_cadence_ticks: u64,
    pub checksum_probe_cadence_ticks: u64,
    pub max_clients: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0".into(),
            port: 4040,
            scenario_path: String::new(),
            snapshot_cadence_ticks: crate::protocol::SNAPSHOT_CADENCE_TICKS,
            checksum_probe_cadence_ticks: crate::protocol::CHECKSUM_PROBE_CADENCE_TICKS,
            max_clients: 8,
        }
    }
}

/// **M8A § server**: minimal authoritative-server scaffold. The actual
/// QUIC transport + drive-tick integration lands at M9+.
#[derive(Debug)]
pub struct Server {
    pub config: ServerConfig,
    pub clients_connected: u32,
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            clients_connected: 0,
        }
    }

    /// **M8A § server**: returns the canonical server-side run bundle
    /// path. Per spec § Acceptance — 8-client LAN session writes a
    /// single server-side run bundle to
    /// `prototype_runs/server/<run-id>/`.
    pub fn run_bundle_path(&self, run_id: &str) -> String {
        format!("prototype_runs/server/{run_id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_server_config_uses_locked_cadence() {
        let cfg = ServerConfig::default();
        assert_eq!(cfg.snapshot_cadence_ticks, 64);
        assert_eq!(cfg.checksum_probe_cadence_ticks, 64);
    }

    #[test]
    fn run_bundle_path_format() {
        let cfg = ServerConfig::default();
        let srv = Server::new(cfg);
        assert_eq!(srv.run_bundle_path("abc123"), "prototype_runs/server/abc123");
    }
}
