//! M8A § Network protocol v0.1 — locked wire frames + M8B byte-stable
//! v0.1 frame layout + semver gate + downgrade-attack detection.
//!
//! ```text
//! NetFrame {
//!   version: u16  // protocol_version, locked to 1 at v0.1
//!   seq: u32      // monotonic per-sender frame counter
//!   timestamp_ms: u64  // sender's monotonic clock at send time
//!   payload: NetPayload  // tagged union
//! }
//! ```
//!
//! M8A shipped the JSON-encoded scaffold. M8B promotes it to a
//! byte-pinned v0.1 layout via the `frame_v01` submodule + adds the
//! semver gate via `semver`.

use serde::{Deserialize, Serialize};

pub mod byte_pinning_tests;
pub mod frame_v01;
pub mod semver;

/// scaffold field). M8B introduces the byte-pinned `semver_packed` u16
/// in `NetFrameV01` for the canonical wire.
pub const PROTOCOL_VERSION: u16 = 1;

/// headers). Avoids fragmentation.
pub const NET_FRAME_MAX_SIZE_BYTES: usize = 1450;

/// `cfctl srv.set_cvar net.snapshot.cadence_ticks <n>`).
pub const SNAPSHOT_CADENCE_TICKS: u64 = 64;

pub const CHECKSUM_PROBE_CADENCE_TICKS: u64 = 64;

/// Server-initiated handshake response: accept or reject. Reject reasons
/// MUST include `protocol_version_mismatch` for stale-client kicks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HandshakeResponse {
    Accept {
        session_id: String,
        server_version: String,
        content_hash: String,
    },
    Reject {
        reason: String,
    },
}

/// The canonical wire frame for cf-net.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetFrame {
    pub version: u16,
    pub seq: u32,
    pub timestamp_ms: u64,
    pub payload: NetPayload,
}

impl NetFrame {
    pub fn new(seq: u32, timestamp_ms: u64, payload: NetPayload) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            seq,
            timestamp_ms,
            payload,
        }
    }
}

/// Tagged union of per-frame payloads. M8A locks the variant set;
/// downstream milestones extend by adding variants (additive-only).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NetPayload {
    Handshake {
        client_version: String,
        capabilities: Vec<String>,
        session_token: Option<String>,
        content_hash: String,
    },
    HandshakeAck(HandshakeResponse),
    InputCommand {
        tick: u64,
        intent_event_id: String,
        control_command_json: String,
    },
    SnapshotDelta {
        from_tick: u64,
        to_tick: u64,
        delta_bytes: Vec<u8>,
    },
    EventBatch {
        tick: u64,
        events_jsonl: Vec<String>,
    },
    ChecksumProbe {
        tick: u64,
        checksum_hex: String,
    },
    Ping {
        send_ms: u64,
    },
    Pong {
        send_ms: u64,
        recv_ms: u64,
    },
    Disconnect {
        reason: String,
    },
}

/// before any sim frame.
pub fn verify_handshake_version(client_version: u16) -> crate::NetResult<()> {
    if client_version != PROTOCOL_VERSION {
        return Err(crate::NetError::ProtocolVersionMismatch {
            server: PROTOCOL_VERSION,
            client: client_version,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handshake_accepts_matched_version() {
        assert!(verify_handshake_version(PROTOCOL_VERSION).is_ok());
    }

    #[test]
    fn handshake_rejects_mismatched_version() {
        let err = verify_handshake_version(PROTOCOL_VERSION + 1).unwrap_err();
        assert!(matches!(err, crate::NetError::ProtocolVersionMismatch { .. }));
    }

    #[test]
    fn net_frame_round_trip() {
        let f = NetFrame::new(42, 123456, NetPayload::Ping { send_ms: 100 });
        let s = serde_json::to_string(&f).unwrap();
        let back: NetFrame = serde_json::from_str(&s).unwrap();
        assert_eq!(f, back);
    }

    #[test]
    fn protocol_version_is_locked_at_one() {
        assert_eq!(
            PROTOCOL_VERSION, 1,
            "M8A v0.1 wire protocol is LOCKED at PROTOCOL_VERSION=1"
        );
    }
}
