//! M8B § Stream routing — which transport (reliable bidi stream vs
//! unreliable datagram) carries each NetPayloadV01 variant.
//!
//! Per M8B § Acceptance:
//! - "Reliable QUIC stream carries the canonical event log" → EventBatch
//!   travels over the reliable bidi stream `event_log`.
//! - "Unreliable datagram carries per-tick input" → InputCommand travels
//!   over a QUIC unreliable datagram and the unreliable path never
//!   blocks the reliable event stream.
//!
//! Stream names are locked at v0.1. The CI byte-pin gate prevents
//! silent re-naming; the transport-bind matrix below is what M9+'s real
//! QUIC wiring will consume.

use serde::{Deserialize, Serialize};

use crate::protocol::frame_v01::{NetPayloadV01, PayloadKind};

/// Locked v0.1 name for the reliable bidi stream carrying the canonical
/// event log (EventBatch payloads + canonical run-bundle event stream).
pub const RELIABLE_STREAM_EVENT_LOG: &str = "event_log";

/// Locked v0.1 name for the reliable bidi stream carrying the handshake +
/// session-level control traffic.
pub const RELIABLE_STREAM_CONTROL: &str = "control";

/// Locked v0.1 name for the reliable bidi stream carrying FEC-protected
/// reliable payloads (small EventBatch shards).
pub const RELIABLE_STREAM_FEC: &str = "fec";

/// Which QUIC transport binding a payload uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportBinding {
    /// Reliable bidi stream by name (e.g. "event_log").
    ReliableStream(&'static str),
    /// Unreliable QUIC datagram (no ack, no retransmit, no head-of-line
    /// blocking on the reliable stream).
    UnreliableDatagram,
}

impl TransportBinding {
    pub fn is_reliable(self) -> bool {
        matches!(self, Self::ReliableStream(_))
    }

    pub fn stream_name(self) -> Option<&'static str> {
        match self {
            Self::ReliableStream(name) => Some(name),
            Self::UnreliableDatagram => None,
        }
    }
}

/// **M8B § locked**: per-payload transport binding. This is the only
/// canonical mapping consumed by the M9+ QUIC wiring; any new payload
/// variant MUST be added here at the same time as PayloadKind is
/// extended.
pub fn binding_for(kind: PayloadKind) -> TransportBinding {
    match kind {
        // Reliable stream — control plane (handshake + handshake ack +
        // disconnect). Loss / re-order would break session semantics.
        PayloadKind::Handshake | PayloadKind::HandshakeAck | PayloadKind::Disconnect => {
            TransportBinding::ReliableStream(RELIABLE_STREAM_CONTROL)
        }
        // Reliable stream — the canonical event log (per spec §
        // Acceptance "Reliable QUIC stream carries the canonical event
        // log"). No event ever dropped / reordered / duplicated.
        PayloadKind::EventBatch => TransportBinding::ReliableStream(RELIABLE_STREAM_EVENT_LOG),
        // Reliable stream — FEC-protected small reliable payloads.
        PayloadKind::FecShard => TransportBinding::ReliableStream(RELIABLE_STREAM_FEC),
        // Unreliable datagram — per-tick input + redundant tail. The
        // unreliable path never blocks the reliable stream per spec
        // § Acceptance "Unreliable datagram carries per-tick input".
        PayloadKind::InputCommand | PayloadKind::InputCommandRedundant => TransportBinding::UnreliableDatagram,
        // Unreliable datagram — delta snapshots are time-sensitive +
        // a dropped delta is recovered from the next keyframe.
        PayloadKind::SnapshotDelta => TransportBinding::UnreliableDatagram,
        // Unreliable datagram — per-cadence checksum probe (fire and
        // forget; mismatch triggers a snapshot resync).
        PayloadKind::ChecksumProbe => TransportBinding::UnreliableDatagram,
        // Unreliable datagram — ping/pong RTT probe.
        PayloadKind::Ping | PayloadKind::Pong => TransportBinding::UnreliableDatagram,
        // Reliable stream — session-level outcome notifications.
        PayloadKind::NatTraversalOutcome | PayloadKind::RollbackWindow => {
            TransportBinding::ReliableStream(RELIABLE_STREAM_CONTROL)
        }
    }
}

/// Convenience: return the binding for a payload value (vs. a kind).
pub fn binding_for_payload(p: &NetPayloadV01) -> TransportBinding {
    binding_for(p.kind())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_log_stream_is_locked_at_event_log() {
        assert_eq!(RELIABLE_STREAM_EVENT_LOG, "event_log");
    }

    #[test]
    fn event_batch_routes_to_reliable_event_log_stream() {
        let binding = binding_for(PayloadKind::EventBatch);
        assert!(binding.is_reliable());
        assert_eq!(binding.stream_name(), Some("event_log"));
    }

    #[test]
    fn input_command_routes_to_unreliable_datagram() {
        let binding = binding_for(PayloadKind::InputCommand);
        assert!(!binding.is_reliable(), "InputCommand must NOT use reliable stream");
        assert_eq!(binding.stream_name(), None);
        assert_eq!(binding, TransportBinding::UnreliableDatagram);
    }

    #[test]
    fn input_command_redundant_also_routes_to_datagram() {
        let binding = binding_for(PayloadKind::InputCommandRedundant);
        assert_eq!(binding, TransportBinding::UnreliableDatagram);
    }

    #[test]
    fn handshake_routes_to_reliable_control_stream() {
        let binding = binding_for(PayloadKind::Handshake);
        assert_eq!(binding.stream_name(), Some("control"));
    }

    #[test]
    fn fec_shard_routes_to_reliable_fec_stream() {
        let binding = binding_for(PayloadKind::FecShard);
        assert!(binding.is_reliable());
        assert_eq!(binding.stream_name(), Some("fec"));
    }

    #[test]
    fn every_payload_kind_has_a_canonical_binding() {
        // Iterate every PayloadKind discriminant. The match in
        // `binding_for` is total over PayloadKind, so this only
        // asserts the function does not panic + returns a stable
        // binding for every variant.
        for disc in 0u32..=12u32 {
            let kind = PayloadKind::from_u32(disc).expect("known disc");
            let _binding = binding_for(kind);
        }
    }
}
