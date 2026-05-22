//! M8B § Frame v0.1 — locked byte-stable wire encoding.
//!
//! Per M8B spec § Notes for the implementer: frame v0.1 layout MUST be
//! expressible as a single `#[repr(C, packed)]`-equivalent serde encoder
//! with fixed-int + little-endian options. Any byte that flips here MUST
//! bump `PROTOCOL_SEMVER` minor and add a new fixture; the byte-pin CI
//! gate (`game/tools/ci/m8b_protocol_byte_pin.sh`) enforces this.
//!
//! Implementation: **`bincode` 2.x** with
//! `Configuration::standard().with_fixed_int_encoding().with_little_endian()`
//! and the serde-compat adapter. The
//! `serde_with::DisplayFromStr` adapter is explicitly banned in this
//! crate per spec Notes (it produces variable-length encodings).
//!
//! Bincode 2.x byte layout (with the above config):
//!
//! - Integers: fixed-size little-endian (u8=1B, u16=2B, u32=4B, u64=8B).
//! - Bool: 1 byte (0 or 1).
//! - `String`: u64 LE length prefix + UTF-8 bytes (no NUL).
//! - `Vec<T>`: u64 LE length prefix + serialized elements.
//! - `Vec<u8>`: u64 LE length prefix + raw bytes.
//! - Fixed-size arrays `[u8; N]`: N raw bytes (no length prefix).
//! - Tagged enums (no `#[serde(tag)]`): u32 LE variant index + variant
//!   fields in declared order.
//!
//! No `f64` permitted in the wire encoding (M8B Notes § "Mixed f32 / f64
//! in serialization paths is a common source of cross-OS divergence").

use bincode::config::{Configuration, Fixint, LittleEndian, Limit};
use serde::{Deserialize, Serialize};

use crate::loss_recovery::redundant_input::RedundantInputTail;

/// headers). Inherits from M8A. Avoids IP fragmentation.
pub const NET_FRAME_V01_MAX_SIZE_BYTES: usize = 1450;

/// payload size for a minimal InputCommand alone MUST be ≤ 96 bytes.
pub const INPUT_COMMAND_PAYLOAD_MAX_BYTES: usize = 96;

/// LOCKED — never reorder, never drop, never reuse a removed slot.
/// Adding a new variant means: append a new u32 at the end + bump
/// PROTOCOL_SEMVER minor + add a fixture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum PayloadKind {
    Handshake = 0,
    HandshakeAck = 1,
    InputCommand = 2,
    SnapshotDelta = 3,
    EventBatch = 4,
    ChecksumProbe = 5,
    Ping = 6,
    Pong = 7,
    Disconnect = 8,
    /// datagram). See `loss_recovery::redundant_input`.
    InputCommandRedundant = 9,
    /// < 8 kB. See `loss_recovery::fec`.
    FecShard = 10,
    /// See `nat::*`.
    NatTraversalOutcome = 11,
    /// observability + replay record). See `rollback::resimulate`.
    RollbackWindow = 12,
}

impl PayloadKind {
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    pub fn from_u32(v: u32) -> Option<Self> {
        Some(match v {
            0 => Self::Handshake,
            1 => Self::HandshakeAck,
            2 => Self::InputCommand,
            3 => Self::SnapshotDelta,
            4 => Self::EventBatch,
            5 => Self::ChecksumProbe,
            6 => Self::Ping,
            7 => Self::Pong,
            8 => Self::Disconnect,
            9 => Self::InputCommandRedundant,
            10 => Self::FecShard,
            11 => Self::NatTraversalOutcome,
            12 => Self::RollbackWindow,
            _ => return None,
        })
    }
}

/// Locked v0.1 frame envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetFrameV01 {
    pub semver_packed: u16,
    pub seq: u32,
    pub timestamp_ms: u64,
    pub payload: NetPayloadV01,
}

/// Locked v0.1 tagged-union payload. **Byte-stable**. No
/// `#[serde(tag = ...)]` attribute so bincode encodes via its native
/// variant-index u32 representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NetPayloadV01 {
    Handshake {
        semver_packed: u16,
        client_version: String,
        content_hash: String,
        supported_features: Vec<String>,
    },
    HandshakeAck {
        accepted: bool,
        granted_features: Vec<String>,
        session_id: String,
        server_semver_packed: u16,
        reject_reason: String,
        download_url: String,
    },
    InputCommand {
        tick: u64,
        intent_event_id: String,
        control_command_bytes: Vec<u8>,
    },
    SnapshotDelta {
        from_tick: u64,
        to_tick: u64,
        delta_bytes: Vec<u8>,
    },
    EventBatch {
        tick: u64,
        events_bytes: Vec<u8>,
    },
    ChecksumProbe {
        tick: u64,
        checksum_bytes: [u8; 32],
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
    InputCommandRedundant {
        head_tick: u64,
        tail: RedundantInputTail,
    },
    FecShard {
        group_id: u64,
        shard_index: u8,
        k: u8,
        m: u8,
        shard_bytes: Vec<u8>,
    },
    NatTraversalOutcome {
        session_id: String,
        method: String,
        path: String,
        elapsed_ms: u32,
    },
    RollbackWindow {
        from_tick: u64,
        to_tick: u64,
        resim_us: u32,
        cause: String,
    },
}

impl NetPayloadV01 {
    pub fn kind(&self) -> PayloadKind {
        match self {
            Self::Handshake { .. } => PayloadKind::Handshake,
            Self::HandshakeAck { .. } => PayloadKind::HandshakeAck,
            Self::InputCommand { .. } => PayloadKind::InputCommand,
            Self::SnapshotDelta { .. } => PayloadKind::SnapshotDelta,
            Self::EventBatch { .. } => PayloadKind::EventBatch,
            Self::ChecksumProbe { .. } => PayloadKind::ChecksumProbe,
            Self::Ping { .. } => PayloadKind::Ping,
            Self::Pong { .. } => PayloadKind::Pong,
            Self::Disconnect { .. } => PayloadKind::Disconnect,
            Self::InputCommandRedundant { .. } => PayloadKind::InputCommandRedundant,
            Self::FecShard { .. } => PayloadKind::FecShard,
            Self::NatTraversalOutcome { .. } => PayloadKind::NatTraversalOutcome,
            Self::RollbackWindow { .. } => PayloadKind::RollbackWindow,
        }
    }
}

/// Errors emitted by the v0.1 codec.
#[derive(Debug, thiserror::Error)]
pub enum FrameV01Error {
    #[error("frame too large: {0} bytes > {NET_FRAME_V01_MAX_SIZE_BYTES} max")]
    FrameTooLarge(usize),
    #[error("bincode encode: {0}")]
    BincodeEncode(String),
    #[error("bincode decode: {0}")]
    BincodeDecode(String),
}

pub type FrameV01Result<T> = Result<T, FrameV01Error>;

/// options"**: the LOCKED encoder config. Any code path that wants to
/// produce or consume the v0.1 wire bytes MUST use this exact config.
const fn bincode_v01_config() -> Configuration<LittleEndian, Fixint, Limit<{ NET_FRAME_V01_MAX_SIZE_BYTES }>> {
    bincode::config::standard()
        .with_fixed_int_encoding()
        .with_little_endian()
        .with_limit::<{ NET_FRAME_V01_MAX_SIZE_BYTES }>()
}

/// Encode a NetFrameV01 to its canonical byte form. Asserts the result
/// is within the locked 1450-byte per-frame max.
pub fn encode_frame(frame: &NetFrameV01) -> FrameV01Result<Vec<u8>> {
    let cfg = bincode_v01_config();
    let bytes = bincode::serde::encode_to_vec(frame, cfg)
        .map_err(|e| FrameV01Error::BincodeEncode(format!("{e}")))?;
    if bytes.len() > NET_FRAME_V01_MAX_SIZE_BYTES {
        return Err(FrameV01Error::FrameTooLarge(bytes.len()));
    }
    Ok(bytes)
}

/// Encode a payload alone (no frame envelope). Useful for size checks
/// and fixture vectors.
pub fn encode_payload_to_vec(p: &NetPayloadV01) -> Vec<u8> {
    // No size cap — payload alone is bounded by the frame envelope check
    // in `encode_frame`. Per the v0.1 contract, the payload bytes are
    // identical whether encoded standalone or inside a frame.
    let cfg = bincode::config::standard()
        .with_fixed_int_encoding()
        .with_little_endian();
    bincode::serde::encode_to_vec(p, cfg).expect("payload encode (no limit)")
}

/// Decode a NetFrameV01 from bytes. Errors on truncation, oversize, or
/// unknown payload discriminant (bincode rejects out-of-range variant
/// indices).
pub fn decode_frame(bytes: &[u8]) -> FrameV01Result<NetFrameV01> {
    if bytes.len() > NET_FRAME_V01_MAX_SIZE_BYTES {
        return Err(FrameV01Error::FrameTooLarge(bytes.len()));
    }
    let cfg = bincode_v01_config();
    let (frame, _consumed): (NetFrameV01, usize) = bincode::serde::decode_from_slice(bytes, cfg)
        .map_err(|e| FrameV01Error::BincodeDecode(format!("{e}")))?;
    Ok(frame)
}

/// Decode a standalone payload (no frame envelope). Inverse of
/// [`encode_payload_to_vec`].
pub fn decode_payload(bytes: &[u8]) -> FrameV01Result<NetPayloadV01> {
    let cfg = bincode::config::standard()
        .with_fixed_int_encoding()
        .with_little_endian();
    let (p, _consumed): (NetPayloadV01, usize) = bincode::serde::decode_from_slice(bytes, cfg)
        .map_err(|e| FrameV01Error::BincodeDecode(format!("{e}")))?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_kind_round_trips_all_discriminants() {
        for disc in 0..=12u32 {
            let kind = PayloadKind::from_u32(disc).expect("known disc");
            assert_eq!(kind.as_u32(), disc);
        }
        assert!(PayloadKind::from_u32(99).is_none());
    }

    #[test]
    fn ping_round_trip_is_byte_stable() {
        let frame = NetFrameV01 {
            semver_packed: 0x0104,
            seq: 42,
            timestamp_ms: 1_000_000,
            payload: NetPayloadV01::Ping { send_ms: 7 },
        };
        let bytes = encode_frame(&frame).unwrap();
        // 2 (semver) + 4 (seq) + 8 (ts) + 4 (kind disc) + 8 (send_ms) = 26
        assert_eq!(bytes.len(), 26);
        let back = decode_frame(&bytes).unwrap();
        assert_eq!(frame, back);
    }

    #[test]
    fn input_command_minimal_fits_within_96_byte_payload_budget() {
        let payload = NetPayloadV01::InputCommand {
            tick: 612,
            intent_event_id: "m1_r:612:0".to_string(),
            control_command_bytes: vec![0u8; 32], // typical 32-byte intent
        };
        let bytes = encode_payload_to_vec(&payload);
        assert!(
            bytes.len() <= INPUT_COMMAND_PAYLOAD_MAX_BYTES,
            "InputCommand payload {} bytes exceeds 96-byte budget",
            bytes.len()
        );
    }

    #[test]
    fn handshake_round_trip() {
        let payload = NetPayloadV01::Handshake {
            semver_packed: crate::protocol::semver::pack(0, 1, 4),
            client_version: "cf-app 0.1.4".into(),
            content_hash: "abcd".to_string(),
            supported_features: vec!["redundant_input".into(), "fec".into()],
        };
        let frame = NetFrameV01 {
            semver_packed: crate::protocol::semver::pack(0, 1, 4),
            seq: 1,
            timestamp_ms: 0,
            payload,
        };
        let bytes = encode_frame(&frame).unwrap();
        let back = decode_frame(&bytes).unwrap();
        assert_eq!(frame, back);
    }

    #[test]
    fn decode_rejects_oversized_frame() {
        let oversized = vec![0u8; NET_FRAME_V01_MAX_SIZE_BYTES + 1];
        let err = decode_frame(&oversized).unwrap_err();
        assert!(matches!(err, FrameV01Error::FrameTooLarge(_)));
    }

    #[test]
    fn fec_shard_round_trip() {
        let frame = NetFrameV01 {
            semver_packed: 0x0104,
            seq: 7,
            timestamp_ms: 1234,
            payload: NetPayloadV01::FecShard {
                group_id: 99,
                shard_index: 4,
                k: 4,
                m: 2,
                shard_bytes: vec![0xDE, 0xAD, 0xBE, 0xEF],
            },
        };
        let bytes = encode_frame(&frame).unwrap();
        let back = decode_frame(&bytes).unwrap();
        assert_eq!(frame, back);
    }

    #[test]
    fn rollback_window_round_trip() {
        let frame = NetFrameV01 {
            semver_packed: 0x0104,
            seq: 0,
            timestamp_ms: 0,
            payload: NetPayloadV01::RollbackWindow {
                from_tick: 614,
                to_tick: 620,
                resim_us: 7100,
                cause: "input_mismatch".into(),
            },
        };
        let bytes = encode_frame(&frame).unwrap();
        let back = decode_frame(&bytes).unwrap();
        assert_eq!(frame, back);
    }
}
