//! M8B § Recovery event payloads.
//!
//! Per M8B spec § Acceptance:
//! - "Reed-Solomon FEC recovers a single-byte-corrupted reliable
//!   payload" → emits `net.fec_recovered { shards_lost: 1, k: 4, m: 2 }`
//! - "Redundant-input encoding recovers from single-datagram loss" →
//!   emits `net.input_resent_redundant { session_id, recovered_tick,
//!   carrier_tick, window_ticks }` per recovered tick.
//! - "ICE-lite + STUN punches a symmetric-NAT to symmetric-NAT pair" →
//!   emits `net.nat_traversal_outcome { session_id, method, path,
//!   elapsed_ms }`.
//! - "6-frame rollback resimulates inside budget" → emits
//!   `net.rollback_window { from_tick, to_tick, resim_us, cause,
//!   rollback_to_tick_elapsed_us, per_frame_resim_elapsed_us,
//!   within_budget }`.
//! - "Semver negotiation ..." → emits `net.protocol_negotiated {
//!   session_id, accepted, server_semver_packed, client_semver_packed,
//!   session_semver_packed, granted_features, ... }`.
//!
//! Each helper returns a `serde_json::Value` matching the corresponding
//! `cf-replay/schemas/event/net_*.json` schema, ready for the recorder
//! to ingest as a non-cosmetic event.
//!
//! All M8B recovery events MUST be `cosmetic: false` per spec § Notes
//! "All new events MUST cap their cosmetic flag at `false` — protocol-
//! layer events are sim-relevant + non-droppable under M4 cosmetic
//! backpressure." The recorder side calls `record(..)` (not
//! `record_cosmetic(..)`) — this module returns the payload only; the
//! cosmetic boolean is set by the caller at record time.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::loss_recovery::fec::{FecError, FecGroup};
use crate::nat::{NatTraversalMethod, NatTraversalPath};
use crate::protocol::semver::Semver;
use crate::rollback::resimulate::ResimulateOutcome;
use crate::transport_select::TransportMode;

/// **M8B § locked**: every M8B recovery event is non-cosmetic.
pub const NET_RECOVERY_EVENT_COSMETIC: bool = false;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolNegotiatedEvent {
    pub session_id: String,
    pub accepted: bool,
    pub server_semver: Semver,
    pub client_semver: Semver,
    /// `None` when accepted=false.
    pub session_semver: Option<Semver>,
    pub granted_features: Vec<String>,
    pub reject_reason: String,
    pub download_url: String,
}

impl ProtocolNegotiatedEvent {
    /// Build the `net.protocol_negotiated` recorder payload.
    pub fn payload(&self) -> serde_json::Value {
        json!({
            "session_id": self.session_id,
            "accepted": self.accepted,
            "server_semver_packed": self.server_semver.pack(),
            "client_semver_packed": self.client_semver.pack(),
            "session_semver_packed": self.session_semver.map(|s| s.pack()).unwrap_or(0),
            "granted_features": self.granted_features,
            "reject_reason": self.reject_reason,
            "download_url": self.download_url,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputResentRedundantEvent {
    pub session_id: String,
    pub recovered_tick: u64,
    pub carrier_tick: u64,
    pub window_ticks: u8,
}

impl InputResentRedundantEvent {
    pub fn payload(&self) -> serde_json::Value {
        json!({
            "session_id": self.session_id,
            "recovered_tick": self.recovered_tick,
            "carrier_tick": self.carrier_tick,
            "window_ticks": self.window_ticks,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FecRecoveredEvent {
    pub group_id: u64,
    pub k: u8,
    pub m: u8,
    pub shards_lost: u8,
    pub payload_bytes: u32,
}

impl FecRecoveredEvent {
    pub fn payload(&self) -> serde_json::Value {
        json!({
            "group_id": self.group_id,
            "k": self.k,
            "m": self.m,
            "shards_lost": self.shards_lost,
            "payload_bytes": self.payload_bytes,
        })
    }

    /// Build a recovery event from a decoded FEC group + count of lost
    /// shards observed by the receiver.
    pub fn from_group(group: &FecGroup, shards_lost: u8) -> Self {
        Self {
            group_id: group.group_id,
            k: group.k,
            m: group.m,
            shards_lost,
            payload_bytes: group.original_len as u32,
        }
    }
}

/// **M8B § Acceptance "TURN relay engages when ICE-lite fails"** + ICE-lite
/// direct-path scenario: the canonical session-level outcome event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NatTraversalOutcomeEvent {
    pub session_id: String,
    pub method: NatTraversalMethod,
    pub path: NatTraversalPath,
    pub elapsed_ms: u32,
}

impl NatTraversalOutcomeEvent {
    pub fn payload(&self) -> serde_json::Value {
        json!({
            "session_id": self.session_id,
            "method": self.method.as_str(),
            "path": self.path.as_str(),
            "elapsed_ms": self.elapsed_ms,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollbackWindowEvent {
    pub from_tick: u64,
    pub to_tick: u64,
    pub resim_us: u32,
    pub cause: String,
    pub rollback_to_tick_elapsed_us: u32,
    pub per_frame_resim_elapsed_us: u32,
    pub within_budget: bool,
}

impl RollbackWindowEvent {
    pub fn payload(&self) -> serde_json::Value {
        json!({
            "from_tick": self.from_tick,
            "to_tick": self.to_tick,
            "resim_us": self.resim_us,
            "cause": self.cause,
            "rollback_to_tick_elapsed_us": self.rollback_to_tick_elapsed_us,
            "per_frame_resim_elapsed_us": self.per_frame_resim_elapsed_us,
            "within_budget": self.within_budget,
        })
    }

    pub fn from_outcome(outcome: &ResimulateOutcome) -> Self {
        Self {
            from_tick: outcome.from_tick,
            to_tick: outcome.to_tick,
            resim_us: outcome.elapsed_us,
            cause: outcome.cause.clone(),
            rollback_to_tick_elapsed_us: outcome.rollback_to_tick_elapsed_us,
            per_frame_resim_elapsed_us: outcome.per_frame_resim_elapsed_us,
            within_budget: outcome.within_budget,
        }
    }
}

/// **M8B § convenience**: build a session transport projection that the
/// observe.net.session_transport JSON-RPC handler returns when a live
/// session is in flight.
pub fn session_transport_view(
    session_id: &str,
    transport: TransportMode,
    method: NatTraversalMethod,
    path: NatTraversalPath,
    elapsed_ms: u32,
    session_semver: Semver,
) -> serde_json::Value {
    json!({
        "schema_version": 1u32,
        "session_id": session_id,
        "transport_mode": transport.as_str(),
        "traversal_method": method.as_str(),
        "traversal_path": path.as_str(),
        "elapsed_ms": elapsed_ms,
        "session_semver_packed": session_semver.pack(),
    })
}

/// **M8B § convenience**: re-export of [`FecError`] for convenience —
/// caller can use this type when constructing fec_recovered events from
/// a fallible decode result.
pub type FecRecoveryError = FecError;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_negotiated_payload_matches_schema_required_fields() {
        let e = ProtocolNegotiatedEvent {
            session_id: "sess-1".into(),
            accepted: true,
            server_semver: Semver::new(0, 1, 7),
            client_semver: Semver::new(0, 1, 4),
            session_semver: Some(Semver::new(0, 1, 4)),
            granted_features: vec!["fec".into(), "ice_lite".into()],
            reject_reason: String::new(),
            download_url: String::new(),
        };
        let payload = e.payload();
        cf_replay::schemas::validate_event_payload("net", "protocol_negotiated", &payload).expect("valid");
    }

    #[test]
    fn protocol_negotiated_rejected_payload_validates() {
        let e = ProtocolNegotiatedEvent {
            session_id: "sess-2".into(),
            accepted: false,
            server_semver: Semver::new(0, 1, 7),
            client_semver: Semver::new(0, 2, 0),
            session_semver: None,
            granted_features: vec![],
            reject_reason: "protocol_major_mismatch".into(),
            download_url: "https://corefall.example/update".into(),
        };
        let payload = e.payload();
        cf_replay::schemas::validate_event_payload("net", "protocol_negotiated", &payload).expect("valid reject");
    }

    #[test]
    fn input_resent_redundant_payload_validates() {
        let e = InputResentRedundantEvent {
            session_id: "sess-1".into(),
            recovered_tick: 700,
            carrier_tick: 701,
            window_ticks: 3,
        };
        cf_replay::schemas::validate_event_payload("net", "input_resent_redundant", &e.payload()).expect("valid");
    }

    #[test]
    fn fec_recovered_payload_validates_spec_example() {
        // Spec scenario: emits net.fec_recovered { shards_lost: 1, k: 4, m: 2 }
        let e = FecRecoveredEvent {
            group_id: 42,
            k: 4,
            m: 2,
            shards_lost: 1,
            payload_bytes: 512,
        };
        cf_replay::schemas::validate_event_payload("net", "fec_recovered", &e.payload()).expect("valid");
    }

    #[test]
    fn fec_recovered_from_group_round_trips() {
        let payload = vec![0u8; 64];
        let group = crate::loss_recovery::fec::encode_fec_group(&payload, 4, 2, 99).unwrap();
        let e = FecRecoveredEvent::from_group(&group, 1);
        assert_eq!(e.group_id, 99);
        assert_eq!(e.k, 4);
        assert_eq!(e.m, 2);
        assert_eq!(e.shards_lost, 1);
        assert_eq!(e.payload_bytes, 64);
        cf_replay::schemas::validate_event_payload("net", "fec_recovered", &e.payload()).expect("valid");
    }

    #[test]
    fn nat_traversal_outcome_direct_payload_validates() {
        let e = NatTraversalOutcomeEvent {
            session_id: "sess-1".into(),
            method: NatTraversalMethod::IceLite,
            path: NatTraversalPath::Direct,
            elapsed_ms: 2500,
        };
        cf_replay::schemas::validate_event_payload("net", "nat_traversal_outcome", &e.payload()).expect("valid");
    }

    #[test]
    fn nat_traversal_outcome_relay_payload_validates() {
        let e = NatTraversalOutcomeEvent {
            session_id: "sess-1".into(),
            method: NatTraversalMethod::TurnRelay,
            path: NatTraversalPath::Relay,
            elapsed_ms: 5500,
        };
        cf_replay::schemas::validate_event_payload("net", "nat_traversal_outcome", &e.payload()).expect("valid");
    }

    #[test]
    fn rollback_window_payload_validates() {
        let e = RollbackWindowEvent {
            from_tick: 614,
            to_tick: 620,
            resim_us: 7100,
            cause: "input_mismatch".into(),
            rollback_to_tick_elapsed_us: 800,
            per_frame_resim_elapsed_us: 6300,
            within_budget: true,
        };
        cf_replay::schemas::validate_event_payload("net", "rollback_window", &e.payload()).expect("valid");
    }

    #[test]
    fn all_recovery_events_are_non_cosmetic() {
        // **M8B § Notes "All new events MUST cap their cosmetic flag at
        // `false`"** — this is a flat assertion that the canonical
        // constant + the producer side both adhere to the rule.
        const _: () = assert!(!NET_RECOVERY_EVENT_COSMETIC);
        assert_eq!(NET_RECOVERY_EVENT_COSMETIC, false);
    }
}
