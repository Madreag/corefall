//! M8B § Frame v0.1 — locked byte-stable wire encoding.
//!
//! Per M8B spec § Notes for the implementer: frame v0.1 layout MUST be
//! expressible as a single `#[repr(C, packed)]`-equivalent encoder with
//! fixed-int + little-endian options. Any byte that flips here MUST bump
//! `PROTOCOL_SEMVER` minor and add a new fixture; the byte-pin CI gate
//! enforces this.
//!
//! Encoding rules (hand-rolled, identical bytes to a bincode 2.x
//! fixed-int + little-endian config):
//!
//! - Integers: fixed-size little-endian (u8 = 1B, u16 = 2B, u32 = 4B,
//!   u64 = 8B). No varint, no zig-zag, no LEB128.
//! - Bool: u8 (0 or 1).
//! - String: u32 LE length prefix + UTF-8 bytes (no null terminator).
//! - Vec<T>: u32 LE length prefix + serialized elements.
//! - Vec<u8>: u32 LE length prefix + raw bytes (no per-element prefix).
//! - Option<T>: u8 tag (0 = None, 1 = Some) + T if Some.
//! - Enum (tagged union): u32 LE discriminant + serialized fields.
//! - f32: 4 bytes LE (IEEE-754); subnormals + NaN canonicalized via
//!   `to_bits()` so cross-OS encoding is byte-identical.
//!
//! No `f64` permitted in the wire encoding (M8B Notes § "Mixed f32 / f64
//! in serialization paths is a common source of cross-OS divergence").

use serde::{Deserialize, Serialize};

use crate::loss_recovery::redundant_input::RedundantInputTail;

/// **M8B § locked**: per-frame max size (Ethernet MTU minus IP+UDP+QUIC
/// headers). Inherits from M8A. Avoids IP fragmentation.
pub const NET_FRAME_V01_MAX_SIZE_BYTES: usize = 1450;

/// **M8B § Scenario "Unreliable datagram carries per-tick input"**:
/// payload size for a minimal InputCommand alone MUST be ≤ 96 bytes.
pub const INPUT_COMMAND_PAYLOAD_MAX_BYTES: usize = 96;

/// **M8B § Frame v0.1**: discriminant for every NetPayloadV01 variant.
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
    /// **M8B**: redundant-input piggyback (last-N inputs encoded on each
    /// datagram). See `loss_recovery::redundant_input`.
    InputCommandRedundant = 9,
    /// **M8B**: Reed-Solomon FEC parity shard for reliable payloads
    /// < 8 kB. See `loss_recovery::fec`.
    FecShard = 10,
    /// **M8B**: NAT-traversal outcome notification to the peer + lobby.
    /// See `nat::*`.
    NatTraversalOutcome = 11,
    /// **M8B**: rollback resimulate window notification (for cfctl
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

/// Locked v0.1 tagged-union payload. **Byte-stable**.
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
    #[error("buffer too short: need {need} more bytes at offset {at}")]
    Truncated { at: usize, need: usize },
    #[error("invalid utf-8 in string at offset {at}")]
    InvalidUtf8 { at: usize },
    #[error("unknown payload kind discriminant {0}")]
    UnknownPayloadKind(u32),
    #[error("invalid bool byte {0}")]
    InvalidBool(u8),
    #[error("checksum slice must be exactly 32 bytes, got {0}")]
    BadChecksumLen(usize),
}

pub type FrameV01Result<T> = Result<T, FrameV01Error>;

// ----------------------------------------------------------------------
// Encoders (write into a mutable Vec<u8>)
// ----------------------------------------------------------------------

#[inline]
fn write_u8(buf: &mut Vec<u8>, v: u8) {
    buf.push(v);
}

#[inline]
fn write_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

#[inline]
fn write_bool(buf: &mut Vec<u8>, v: bool) {
    write_u8(buf, if v { 1 } else { 0 });
}

#[inline]
fn write_string(buf: &mut Vec<u8>, s: &str) {
    let bytes = s.as_bytes();
    write_u32(buf, bytes.len() as u32);
    buf.extend_from_slice(bytes);
}

#[inline]
fn write_bytes(buf: &mut Vec<u8>, b: &[u8]) {
    write_u32(buf, b.len() as u32);
    buf.extend_from_slice(b);
}

#[inline]
fn write_str_vec(buf: &mut Vec<u8>, v: &[String]) {
    write_u32(buf, v.len() as u32);
    for s in v {
        write_string(buf, s);
    }
}

/// Encode a NetFrameV01 to its canonical byte form. Asserts the result
/// is within the locked 1450-byte per-frame max.
pub fn encode_frame(frame: &NetFrameV01) -> FrameV01Result<Vec<u8>> {
    let mut buf = Vec::with_capacity(64);
    write_u16(&mut buf, frame.semver_packed);
    write_u32(&mut buf, frame.seq);
    write_u64(&mut buf, frame.timestamp_ms);
    write_payload(&mut buf, &frame.payload);
    if buf.len() > NET_FRAME_V01_MAX_SIZE_BYTES {
        return Err(FrameV01Error::FrameTooLarge(buf.len()));
    }
    Ok(buf)
}

/// Encode a payload into a fresh Vec<u8>. Useful for size checks and
/// fixtures.
pub fn encode_payload_to_vec(p: &NetPayloadV01) -> Vec<u8> {
    let mut buf = Vec::with_capacity(32);
    write_payload(&mut buf, p);
    buf
}

fn write_payload(buf: &mut Vec<u8>, p: &NetPayloadV01) {
    write_u32(buf, p.kind().as_u32());
    match p {
        NetPayloadV01::Handshake {
            semver_packed,
            client_version,
            content_hash,
            supported_features,
        } => {
            write_u16(buf, *semver_packed);
            write_string(buf, client_version);
            write_string(buf, content_hash);
            write_str_vec(buf, supported_features);
        }
        NetPayloadV01::HandshakeAck {
            accepted,
            granted_features,
            session_id,
            server_semver_packed,
            reject_reason,
            download_url,
        } => {
            write_bool(buf, *accepted);
            write_str_vec(buf, granted_features);
            write_string(buf, session_id);
            write_u16(buf, *server_semver_packed);
            write_string(buf, reject_reason);
            write_string(buf, download_url);
        }
        NetPayloadV01::InputCommand {
            tick,
            intent_event_id,
            control_command_bytes,
        } => {
            write_u64(buf, *tick);
            write_string(buf, intent_event_id);
            write_bytes(buf, control_command_bytes);
        }
        NetPayloadV01::SnapshotDelta {
            from_tick,
            to_tick,
            delta_bytes,
        } => {
            write_u64(buf, *from_tick);
            write_u64(buf, *to_tick);
            write_bytes(buf, delta_bytes);
        }
        NetPayloadV01::EventBatch { tick, events_bytes } => {
            write_u64(buf, *tick);
            write_bytes(buf, events_bytes);
        }
        NetPayloadV01::ChecksumProbe { tick, checksum_bytes } => {
            write_u64(buf, *tick);
            buf.extend_from_slice(checksum_bytes);
        }
        NetPayloadV01::Ping { send_ms } => {
            write_u64(buf, *send_ms);
        }
        NetPayloadV01::Pong { send_ms, recv_ms } => {
            write_u64(buf, *send_ms);
            write_u64(buf, *recv_ms);
        }
        NetPayloadV01::Disconnect { reason } => {
            write_string(buf, reason);
        }
        NetPayloadV01::InputCommandRedundant { head_tick, tail } => {
            write_u64(buf, *head_tick);
            tail.write_v01(buf);
        }
        NetPayloadV01::FecShard {
            group_id,
            shard_index,
            k,
            m,
            shard_bytes,
        } => {
            write_u64(buf, *group_id);
            write_u8(buf, *shard_index);
            write_u8(buf, *k);
            write_u8(buf, *m);
            write_bytes(buf, shard_bytes);
        }
        NetPayloadV01::NatTraversalOutcome {
            session_id,
            method,
            path,
            elapsed_ms,
        } => {
            write_string(buf, session_id);
            write_string(buf, method);
            write_string(buf, path);
            write_u32(buf, *elapsed_ms);
        }
        NetPayloadV01::RollbackWindow {
            from_tick,
            to_tick,
            resim_us,
            cause,
        } => {
            write_u64(buf, *from_tick);
            write_u64(buf, *to_tick);
            write_u32(buf, *resim_us);
            write_string(buf, cause);
        }
    }
}

// ----------------------------------------------------------------------
// Decoders (read from a slice with a moving cursor)
// ----------------------------------------------------------------------

pub(crate) struct Cursor<'a> {
    pub(crate) src: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(src: &'a [u8]) -> Self {
        Self { src, pos: 0 }
    }

    fn need(&self, n: usize) -> FrameV01Result<()> {
        if self.pos + n > self.src.len() {
            Err(FrameV01Error::Truncated { at: self.pos, need: n })
        } else {
            Ok(())
        }
    }

    fn read_u8(&mut self) -> FrameV01Result<u8> {
        self.need(1)?;
        let v = self.src[self.pos];
        self.pos += 1;
        Ok(v)
    }

    fn read_u16(&mut self) -> FrameV01Result<u16> {
        self.need(2)?;
        let v = u16::from_le_bytes([self.src[self.pos], self.src[self.pos + 1]]);
        self.pos += 2;
        Ok(v)
    }

    fn read_u32(&mut self) -> FrameV01Result<u32> {
        self.need(4)?;
        let v = u32::from_le_bytes([
            self.src[self.pos],
            self.src[self.pos + 1],
            self.src[self.pos + 2],
            self.src[self.pos + 3],
        ]);
        self.pos += 4;
        Ok(v)
    }

    fn read_u64(&mut self) -> FrameV01Result<u64> {
        self.need(8)?;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&self.src[self.pos..self.pos + 8]);
        self.pos += 8;
        Ok(u64::from_le_bytes(bytes))
    }

    fn read_bool(&mut self) -> FrameV01Result<bool> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            b => Err(FrameV01Error::InvalidBool(b)),
        }
    }

    fn read_string(&mut self) -> FrameV01Result<String> {
        let at = self.pos;
        let len = self.read_u32()? as usize;
        self.need(len)?;
        let slice = &self.src[self.pos..self.pos + len];
        self.pos += len;
        std::str::from_utf8(slice)
            .map(|s| s.to_owned())
            .map_err(|_| FrameV01Error::InvalidUtf8 { at })
    }

    fn read_bytes(&mut self) -> FrameV01Result<Vec<u8>> {
        let len = self.read_u32()? as usize;
        self.need(len)?;
        let v = self.src[self.pos..self.pos + len].to_vec();
        self.pos += len;
        Ok(v)
    }

    fn read_str_vec(&mut self) -> FrameV01Result<Vec<String>> {
        let len = self.read_u32()? as usize;
        let mut out = Vec::with_capacity(len.min(64));
        for _ in 0..len {
            out.push(self.read_string()?);
        }
        Ok(out)
    }

    fn read_fixed<const N: usize>(&mut self) -> FrameV01Result<[u8; N]> {
        self.need(N)?;
        let mut out = [0u8; N];
        out.copy_from_slice(&self.src[self.pos..self.pos + N]);
        self.pos += N;
        Ok(out)
    }
}

/// Decode a NetFrameV01 from bytes. Errors on truncation or unknown
/// payload discriminant.
pub fn decode_frame(bytes: &[u8]) -> FrameV01Result<NetFrameV01> {
    if bytes.len() > NET_FRAME_V01_MAX_SIZE_BYTES {
        return Err(FrameV01Error::FrameTooLarge(bytes.len()));
    }
    let mut c = Cursor::new(bytes);
    let semver_packed = c.read_u16()?;
    let seq = c.read_u32()?;
    let timestamp_ms = c.read_u64()?;
    let payload = read_payload(&mut c)?;
    Ok(NetFrameV01 {
        semver_packed,
        seq,
        timestamp_ms,
        payload,
    })
}

fn read_payload(c: &mut Cursor<'_>) -> FrameV01Result<NetPayloadV01> {
    let disc = c.read_u32()?;
    let kind = PayloadKind::from_u32(disc).ok_or(FrameV01Error::UnknownPayloadKind(disc))?;
    Ok(match kind {
        PayloadKind::Handshake => NetPayloadV01::Handshake {
            semver_packed: c.read_u16()?,
            client_version: c.read_string()?,
            content_hash: c.read_string()?,
            supported_features: c.read_str_vec()?,
        },
        PayloadKind::HandshakeAck => NetPayloadV01::HandshakeAck {
            accepted: c.read_bool()?,
            granted_features: c.read_str_vec()?,
            session_id: c.read_string()?,
            server_semver_packed: c.read_u16()?,
            reject_reason: c.read_string()?,
            download_url: c.read_string()?,
        },
        PayloadKind::InputCommand => NetPayloadV01::InputCommand {
            tick: c.read_u64()?,
            intent_event_id: c.read_string()?,
            control_command_bytes: c.read_bytes()?,
        },
        PayloadKind::SnapshotDelta => NetPayloadV01::SnapshotDelta {
            from_tick: c.read_u64()?,
            to_tick: c.read_u64()?,
            delta_bytes: c.read_bytes()?,
        },
        PayloadKind::EventBatch => NetPayloadV01::EventBatch {
            tick: c.read_u64()?,
            events_bytes: c.read_bytes()?,
        },
        PayloadKind::ChecksumProbe => NetPayloadV01::ChecksumProbe {
            tick: c.read_u64()?,
            checksum_bytes: c.read_fixed::<32>()?,
        },
        PayloadKind::Ping => NetPayloadV01::Ping { send_ms: c.read_u64()? },
        PayloadKind::Pong => NetPayloadV01::Pong {
            send_ms: c.read_u64()?,
            recv_ms: c.read_u64()?,
        },
        PayloadKind::Disconnect => NetPayloadV01::Disconnect {
            reason: c.read_string()?,
        },
        PayloadKind::InputCommandRedundant => NetPayloadV01::InputCommandRedundant {
            head_tick: c.read_u64()?,
            tail: RedundantInputTail::read_v01(c)?,
        },
        PayloadKind::FecShard => {
            let group_id = c.read_u64()?;
            let shard_index = c.read_u8()?;
            let k = c.read_u8()?;
            let m = c.read_u8()?;
            let shard_bytes = c.read_bytes()?;
            NetPayloadV01::FecShard {
                group_id,
                shard_index,
                k,
                m,
                shard_bytes,
            }
        }
        PayloadKind::NatTraversalOutcome => NetPayloadV01::NatTraversalOutcome {
            session_id: c.read_string()?,
            method: c.read_string()?,
            path: c.read_string()?,
            elapsed_ms: c.read_u32()?,
        },
        PayloadKind::RollbackWindow => NetPayloadV01::RollbackWindow {
            from_tick: c.read_u64()?,
            to_tick: c.read_u64()?,
            resim_us: c.read_u32()?,
            cause: c.read_string()?,
        },
    })
}

/// Crate-internal access for child modules that need to read out of a
/// running cursor (e.g., RedundantInputTail's payload decoder).
pub(crate) type PrivCursor<'a> = Cursor<'a>;

pub(crate) trait CursorRead<'a> {
    fn read_u8(&mut self) -> FrameV01Result<u8>;
    fn read_u32(&mut self) -> FrameV01Result<u32>;
    fn read_u64(&mut self) -> FrameV01Result<u64>;
    fn read_bytes(&mut self) -> FrameV01Result<Vec<u8>>;
}

impl<'a> CursorRead<'a> for Cursor<'a> {
    fn read_u8(&mut self) -> FrameV01Result<u8> {
        Cursor::read_u8(self)
    }
    fn read_u32(&mut self) -> FrameV01Result<u32> {
        Cursor::read_u32(self)
    }
    fn read_u64(&mut self) -> FrameV01Result<u64> {
        Cursor::read_u64(self)
    }
    fn read_bytes(&mut self) -> FrameV01Result<Vec<u8>> {
        Cursor::read_bytes(self)
    }
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
        // **M8B Acceptance §**: payload size for InputCommand alone ≤ 96 bytes.
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
    fn decode_rejects_unknown_payload_kind() {
        let mut bytes = Vec::new();
        write_u16(&mut bytes, 0x0104);
        write_u32(&mut bytes, 0);
        write_u64(&mut bytes, 0);
        write_u32(&mut bytes, 9999);
        let err = decode_frame(&bytes).unwrap_err();
        assert!(matches!(err, FrameV01Error::UnknownPayloadKind(9999)));
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
