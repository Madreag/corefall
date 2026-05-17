//! **M4B § "Tamper-evident competitive replays"** — per-event BLAKE3 chain.
//!
//! The ledger chain is a Merkle-style linked list: every event records the
//! BLAKE3 of (`prev_event_hash` || canonical-JSON of the current event
//! payload), keyed by a 32-byte derivation of `manifest.run_id + scenario
//! seed`. That keying binds the chain to its run: splicing events from a
//! different run produces a hash mismatch the verifier detects.
//!
//! ## Anchor
//!
//! `run_manifest.json.ledger_chain_anchor` is the BLAKE3 chain hash of the
//! LAST event in the run. A tournament organizer publishes the anchor; any
//! third party with the bundle + the anchor can verify the chain is
//! intact (see [`verify_chain`]).
//!
//! ## Dev mode
//!
//! Tournament mode REQUIRES the chain. Dev mode (`cf-headless replay
//! --skip-chain-verify`) is allowed to skip the check — but the CI gate
//! enforces "always on" for tournament-mode bundles.

use serde::{Deserialize, Serialize};

use crate::checksum;

/// One event in the chain. The encoder writes [`ChainedEvent`] structs out
/// to the run-bundle's `events.jsonl` (cf-replay carries the canonical
/// envelope; M4B adds the `prev_event_hash` field).
///
/// We intentionally keep this minimal: cf-replay's full `Event` carries
/// many envelope-level fields. The chain operates over a (run_id, event_id,
/// canonical payload) triple to avoid coupling to the recorder envelope's
/// optional fields (which can change without breaking the chain).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainedEvent {
    pub event_id: String,
    /// Canonical JSON of the payload. The chain hashes this directly so
    /// platform float-representation cannot leak (DR-052).
    pub payload_canonical_json: String,
    /// `Some(hash)` for every event after the first; `None` for the first
    /// event in the chain.
    pub prev_event_hash: Option<String>,
    /// The BLAKE3-keyed hash for THIS event (so the verifier can compare
    /// the recorded value against its recomputation).
    pub chained_hash_hex: String,
}

/// Per-run encoder state. Construct once per run with [`new_encoder`], call
/// [`Encoder::append`] for every event in tick order, finalize with
/// [`Encoder::anchor`].
#[derive(Debug)]
pub struct Encoder {
    key: [u8; 32],
    last_hash: Option<String>,
    events_appended: u64,
}

impl Encoder {
    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    pub fn events_appended(&self) -> u64 {
        self.events_appended
    }

    pub fn append(&mut self, event_id: &str, payload_canonical_json: &str) -> ChainedEvent {
        let chained_hash_hex = self.compute_hash(payload_canonical_json);
        let prev_event_hash = self.last_hash.clone();
        self.last_hash = Some(chained_hash_hex.clone());
        self.events_appended += 1;
        ChainedEvent {
            event_id: event_id.to_string(),
            payload_canonical_json: payload_canonical_json.to_string(),
            prev_event_hash,
            chained_hash_hex,
        }
    }

    /// Returns the final chain anchor — the `chained_hash_hex` of the last
    /// event appended. None when no events have been appended.
    pub fn anchor(&self) -> Option<String> {
        self.last_hash.clone()
    }

    fn compute_hash(&self, payload_canonical_json: &str) -> String {
        let prefix = self.last_hash.as_deref().unwrap_or("");
        let mut buf = Vec::with_capacity(prefix.len() + 1 + payload_canonical_json.len());
        buf.extend_from_slice(prefix.as_bytes());
        buf.push(b'|');
        buf.extend_from_slice(payload_canonical_json.as_bytes());
        checksum::blake3_keyed_hex_of(&self.key, &buf)
    }
}

/// Build the per-run BLAKE3 key. The material is `run_id + "|" + seed`
/// so chains from two different runs (or the same run with a different
/// seed) produce different keys.
pub fn new_encoder(run_id: &str, seed: u64) -> Encoder {
    let material = format!("{run_id}|{seed}");
    Encoder {
        key: checksum::derive_chain_key(&material),
        last_hash: None,
        events_appended: 0,
    }
}

/// Outcome of a chain verification pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum VerifyOutcome {
    Clean {
        events_verified: u64,
        anchor: String,
    },
    Tampered {
        first_break: ChainBreak,
    },
    EmptyChain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainBreak {
    pub event_id: String,
    pub expected_hash: String,
    pub actual_hash: String,
}

/// **M4B § "Ledger chain rejects tampered bundle"** — walk the chain
/// front-to-back, recompute every event's hash, and compare against the
/// recorded `chained_hash_hex`. Returns [`VerifyOutcome::Tampered`] with
/// the first break + the run-end anchor when clean.
pub fn verify_chain(run_id: &str, seed: u64, events: &[ChainedEvent]) -> VerifyOutcome {
    if events.is_empty() {
        return VerifyOutcome::EmptyChain;
    }
    let mut encoder = new_encoder(run_id, seed);
    for event in events {
        let recomputed = encoder.append(&event.event_id, &event.payload_canonical_json);
        if recomputed.prev_event_hash != event.prev_event_hash
            || recomputed.chained_hash_hex != event.chained_hash_hex
        {
            return VerifyOutcome::Tampered {
                first_break: ChainBreak {
                    event_id: event.event_id.clone(),
                    expected_hash: recomputed.chained_hash_hex,
                    actual_hash: event.chained_hash_hex.clone(),
                },
            };
        }
    }
    VerifyOutcome::Clean {
        events_verified: u64::try_from(events.len()).unwrap_or(u64::MAX),
        anchor: encoder.anchor().unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn payload(i: u32) -> String {
        serde_json::to_string(&serde_json::json!({"i": i, "msg": "ok"})).unwrap()
    }

    #[test]
    fn encoder_chains_consecutive_events_and_anchor_matches_last_hash() {
        let mut enc = new_encoder("run-abc", 42);
        let e1 = enc.append("ev1", &payload(1));
        let e2 = enc.append("ev2", &payload(2));
        let e3 = enc.append("ev3", &payload(3));
        assert!(e1.prev_event_hash.is_none());
        assert_eq!(e2.prev_event_hash.as_deref(), Some(e1.chained_hash_hex.as_str()));
        assert_eq!(e3.prev_event_hash.as_deref(), Some(e2.chained_hash_hex.as_str()));
        assert_eq!(enc.anchor().unwrap(), e3.chained_hash_hex);
        assert_eq!(enc.events_appended(), 3);
    }

    #[test]
    fn verify_clean_chain_returns_anchor_and_event_count() {
        let mut enc = new_encoder("run-abc", 42);
        let mut events = Vec::new();
        for i in 0..5 {
            events.push(enc.append(&format!("ev{i}"), &payload(i)));
        }
        let outcome = verify_chain("run-abc", 42, &events);
        match outcome {
            VerifyOutcome::Clean { events_verified, anchor } => {
                assert_eq!(events_verified, 5);
                assert_eq!(anchor, enc.anchor().unwrap());
            }
            other => panic!("expected clean, got {other:?}"),
        }
    }

    #[test]
    fn verify_tampered_event_payload_returns_first_break_at_that_event() {
        let mut enc = new_encoder("run-abc", 42);
        let mut events = Vec::new();
        for i in 0..5 {
            events.push(enc.append(&format!("ev{i}"), &payload(i)));
        }
        // Tamper with event index 2's payload AFTER the hash was computed.
        events[2].payload_canonical_json = serde_json::to_string(&serde_json::json!({"i": 999, "msg": "tampered"})).unwrap();
        let outcome = verify_chain("run-abc", 42, &events);
        match outcome {
            VerifyOutcome::Tampered { first_break } => {
                assert_eq!(first_break.event_id, "ev2");
                assert_ne!(first_break.expected_hash, first_break.actual_hash);
            }
            other => panic!("expected tampered, got {other:?}"),
        }
    }

    #[test]
    fn verify_with_wrong_run_id_or_seed_returns_tampered() {
        let mut enc = new_encoder("run-abc", 42);
        let mut events = Vec::new();
        for i in 0..3 {
            events.push(enc.append(&format!("ev{i}"), &payload(i)));
        }
        let outcome = verify_chain("run-abc", 99, &events);
        assert!(matches!(outcome, VerifyOutcome::Tampered { .. }));
        let outcome = verify_chain("run-xyz", 42, &events);
        assert!(matches!(outcome, VerifyOutcome::Tampered { .. }));
    }

    #[test]
    fn verify_empty_chain_returns_empty_outcome() {
        let outcome = verify_chain("run-abc", 42, &[]);
        assert!(matches!(outcome, VerifyOutcome::EmptyChain));
    }

    #[test]
    fn anchor_is_deterministic_across_repeated_encode() {
        let mut a = new_encoder("run-abc", 42);
        let mut b = new_encoder("run-abc", 42);
        for i in 0..10 {
            a.append(&format!("ev{i}"), &payload(i));
            b.append(&format!("ev{i}"), &payload(i));
        }
        assert_eq!(a.anchor(), b.anchor());
    }
}
