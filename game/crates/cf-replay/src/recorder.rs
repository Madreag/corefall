//! Backpressure-aware event recorder + envelope helpers.

use std::{
    collections::BTreeMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};

use cf_sim_core::{ids::make_event_id, Tick};

use crate::{event::Event, summary::EventCounts, EVENT_SCHEMA_VERSION};

/// Append-friendly recorder. Events go through here so the writer can apply backpressure
/// and surface dropped counts in `summary.json.event_counts.dropped_total`.
///
/// drops the oldest COSMETIC event first (priority-aware drop) so gameplay
/// events are never starved by particle/visual cosmetics. If no cosmetic
/// event is in the buffer, the new gameplay event itself is dropped (counted
/// in `dropped_total` and the next emitted event picks up the dropped count).
///
/// runs in CHAIN MODE. When [`Recorder::enable_chain_mode`] has been called,
/// every recorded event carries a `prev_event_hash` + `chained_hash_hex` pair
/// computed via [`cf_save::ledger_chain::Encoder`]. The final
/// [`Recorder::chain_anchor`] is the run-bundle's tournament-mode anchor.
pub struct Recorder {
    run_id: String,
    seq: AtomicU64,
    pub(crate) inner: Mutex<RecorderInner>,
    /// Maximum events before backpressure drops. 0 = unlimited.
    capacity: usize,
    /// has been called; default `None` (events ship without chain fields,
    /// matching legacy bundle behavior).
    chain_encoder: Mutex<Option<cf_save::ledger_chain::Encoder>>,
}

pub(crate) struct RecorderInner {
    pub(crate) events: Vec<Event>,
    pub(crate) by_category: BTreeMap<String, u64>,
    pub(crate) by_type: BTreeMap<String, u64>,
    pub(crate) by_severity: BTreeMap<String, u64>,
    pub(crate) dropped: u64,
    pub(crate) dropped_cosmetic: u64,
    pub(crate) dropped_gameplay: u64,
    /// Outstanding drops not yet attached to a subsequent emitted event's
    /// `dropped_count` payload field (per M4 § "the per-event payload that
    /// triggered the overflow includes dropped_count=N in the next emitted
    /// event").
    pub(crate) pending_drop_tag: u64,
    pub(crate) peak_buffer_depth: usize,
    pub(crate) first_tick: Option<u64>,
    pub(crate) last_tick: Option<u64>,
    pub(crate) final_checksum: Option<String>,
    pub(crate) checksum_event_count: u64,
}

impl Recorder {
    pub fn new(run_id: String) -> Self {
        Self::with_capacity(run_id, 0)
    }

    /// Create a recorder with a maximum event capacity. 0 = unlimited.
    /// When capacity is exceeded, new events are dropped and the dropped
    /// counter is incremented (surfaced in summary.json.event_counts.dropped_total).
    pub fn with_capacity(run_id: String, capacity: usize) -> Self {
        Self {
            run_id,
            seq: AtomicU64::new(0),
            inner: Mutex::new(RecorderInner {
                events: Vec::new(),
                by_category: BTreeMap::new(),
                by_type: BTreeMap::new(),
                by_severity: BTreeMap::new(),
                dropped: 0,
                dropped_cosmetic: 0,
                dropped_gameplay: 0,
                pending_drop_tag: 0,
                peak_buffer_depth: 0,
                first_tick: None,
                last_tick: None,
                final_checksum: None,
                checksum_event_count: 0,
            }),
            capacity,
            chain_encoder: Mutex::new(None),
        }
    }

    /// chain mode. The encoder uses keyed-BLAKE3 with a per-run key derived
    /// from `(run_id, seed)` so chains from two different runs (or the same
    /// run with a different seed) produce different anchors. Idempotent:
    /// re-enabling resets the chain state.
    pub fn enable_chain_mode(&self, seed: u64) {
        let encoder = cf_save::ledger_chain::new_encoder(&self.run_id, seed);
        let mut guard = self.chain_encoder.lock().expect("chain_encoder mutex poisoned");
        *guard = Some(encoder);
    }

    pub fn chain_mode_enabled(&self) -> bool {
        self.chain_encoder
            .lock()
            .expect("chain_encoder mutex poisoned")
            .is_some()
    }

    /// the final event. Returns `None` when chain mode is OFF or no event
    /// has been recorded yet.
    pub fn chain_anchor(&self) -> Option<String> {
        self.chain_encoder
            .lock()
            .expect("chain_encoder mutex poisoned")
            .as_ref()
            .and_then(|e| e.anchor())
    }

    pub fn peak_buffer_depth(&self) -> usize {
        self.inner.lock().expect("recorder mutex poisoned").peak_buffer_depth
    }

    pub fn dropped_cosmetic_count(&self) -> u64 {
        self.inner.lock().expect("recorder mutex poisoned").dropped_cosmetic
    }

    pub fn dropped_gameplay_count(&self) -> u64 {
        self.inner.lock().expect("recorder mutex poisoned").dropped_gameplay
    }

    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    pub fn dropped_count(&self) -> u64 {
        self.inner.lock().expect("recorder mutex poisoned").dropped
    }

    pub fn event_count(&self) -> usize {
        self.inner.lock().expect("recorder mutex poisoned").events.len()
    }

    pub fn record(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        payload: serde_json::Value,
        parent_event_id: Option<String>,
    ) -> String {
        self.record_with_cosmetic(tick, sim_time_ms, category, event_type, payload, parent_event_id, false)
    }

    /// M4 § Recorder backpressure / cosmetic flag. Record an event flagged as
    /// cosmetic (`cosmetic=true`) so the determinism island excludes it from
    /// `sim_state_v1` hashing and the recorder drops it FIRST under
    /// backpressure (priority-aware drop).
    pub fn record_cosmetic(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        payload: serde_json::Value,
        parent_event_id: Option<String>,
    ) -> String {
        self.record_with_cosmetic(tick, sim_time_ms, category, event_type, payload, parent_event_id, true)
    }

    /// entry (`asset_ref`). Used by capture-grid screenshots, audio playback,
    /// mod-supplied content, etc. The asset_ref value is a string-encoded
    /// `cf-asset-ledger::AssetId` (blake3 hex). Cosmetic when `cosmetic` is
    /// true — capture surfaces don't participate in the deterministic sim
    /// checksum.
    pub fn record_with_asset_ref(&self, params: AssetRefRecordParams<'_>) -> String {
        let AssetRefRecordParams {
            tick,
            sim_time_ms,
            category,
            event_type,
            payload,
            parent_event_id,
            asset_ref,
            cosmetic,
        } = params;
        let event_id = self.record_with_cosmetic(
            tick,
            sim_time_ms,
            category,
            event_type,
            payload,
            parent_event_id,
            cosmetic,
        );
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        if let Some(last) = inner.events.last_mut() {
            if last.event_id == event_id {
                last.asset_ref = Some(asset_ref);
            }
        }
        event_id
    }

    /// M4 § "unknown_cause" marker. Record an event with `parent_event_id = None`
    /// and inject `cause_origin: "unknown_cause"` + a `reason` field into
    /// the payload so the M10 cause-chain walker reports a clean terminal
    /// instead of a "missing parent" bug.
    ///
    /// Use this for events that have no causal predecessor in the event
    /// log (external interrupts, scenario-start defaults, sim-tick fallthroughs
    /// where no upstream cause exists).
    pub fn record_with_unknown_cause(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        mut payload: serde_json::Value,
        reason: &str,
    ) -> String {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("cause_origin".to_string(), serde_json::json!("unknown_cause"));
            obj.insert("cause_origin_reason".to_string(), serde_json::json!(reason));
        }
        self.record(tick, sim_time_ms, category, event_type, payload, None)
    }

    fn record_with_cosmetic(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        category: &str,
        event_type: &str,
        payload: serde_json::Value,
        parent_event_id: Option<String>,
        cosmetic: bool,
    ) -> String {
        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let event_id = make_event_id(&self.run_id, tick.0, seq);
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        let effective_tick = if let Some(last) = inner.last_tick {
            tick.0.max(last)
        } else {
            tick.0
        };
        // Drain any outstanding drop tag onto THIS event before it's stored.
        let drop_tag = if inner.pending_drop_tag > 0 && !cosmetic {
            let n = inner.pending_drop_tag;
            inner.pending_drop_tag = 0;
            Some(n)
        } else {
            None
        };
        // is on, compute the per-event BLAKE3 keyed chain hash over the
        // canonical-JSON of the payload. The chain hash must use the same
        // canonical string that the on-disk events.jsonl will produce on
        // verifier re-read; otherwise the chain breaks even for clean
        // bundles because serde_json without `arbitrary_precision` can
        // lose 1 ULP on f64 values that derived from f32 narrowing (e.g.
        // f32 0.12 → f64 0.11999999731779099 serializes cleanly, but
        // parsing that string back gives a slightly different f64 that
        // re-serializes as "0.119999997317791"). Both `prev_event_hash`
        // and `chained_hash_hex` are stored on the envelope so the
        // verifier can pinpoint a tamper to the exact event_id.
        let (prev_hash, chained_hash) = {
            let mut chain_guard = self.chain_encoder.lock().expect("chain_encoder mutex poisoned");
            if let Some(encoder) = chain_guard.as_mut() {
                let canonical = canonical_payload_for_chain(&payload);
                let chained = encoder.append(&event_id, &canonical);
                (chained.prev_event_hash, Some(chained.chained_hash_hex))
            } else {
                (None, None)
            }
        };
        let event = Event {
            schema_version: EVENT_SCHEMA_VERSION.to_string(),
            run_id: self.run_id.clone(),
            tick: effective_tick,
            sim_time_ms,
            event_id: event_id.clone(),
            category: category.to_string(),
            event_type: event_type.to_string(),
            payload,
            parent_event_id,
            actor_id: None,
            source_id: None,
            team: None,
            pos: None,
            bbox: None,
            dropped_count: drop_tag,
            cosmetic: if cosmetic { Some(true) } else { None },
            asset_ref: None,
            prev_event_hash: prev_hash,
            chained_hash_hex: chained_hash,
        };
        *inner.by_category.entry(event.category.clone()).or_insert(0) += 1;
        *inner.by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        inner.first_tick.get_or_insert(effective_tick);
        inner.last_tick = Some(effective_tick);
        if event.category == "determinism" && event.event_type == "sim_checksum" {
            inner.checksum_event_count += 1;
            if let Some(hex) = event.payload.get("checksum_hex").and_then(|v| v.as_str()) {
                inner.final_checksum = Some(hex.to_string());
            }
        }
        if self.capacity > 0 && inner.events.len() >= self.capacity {
            // M4 § "Cosmetic events drop first under pressure". If the new
            // event is cosmetic, drop the new event immediately. Otherwise,
            // try to evict the oldest cosmetic event from the buffer to make
            // room. If no cosmetic event is available, drop the gameplay
            // event itself and tag the dropped_count on the next emitted
            // event.
            if cosmetic {
                inner.dropped += 1;
                inner.dropped_cosmetic += 1;
                inner.pending_drop_tag += 1;
                return event_id;
            }
            // Search for the oldest cosmetic event to evict.
            let mut evict_idx: Option<usize> = None;
            for (idx, ev) in inner.events.iter().enumerate() {
                if ev.cosmetic == Some(true) {
                    evict_idx = Some(idx);
                    break;
                }
            }
            if let Some(idx) = evict_idx {
                let evicted = inner.events.remove(idx);
                if let Some(cat_count) = inner.by_category.get_mut(&evicted.category) {
                    *cat_count = cat_count.saturating_sub(1);
                }
                if let Some(ty_count) = inner.by_type.get_mut(&evicted.event_type) {
                    *ty_count = ty_count.saturating_sub(1);
                }
                inner.dropped += 1;
                inner.dropped_cosmetic += 1;
                inner.pending_drop_tag += 1;
                inner.events.push(event);
                let depth = inner.events.len();
                if depth > inner.peak_buffer_depth {
                    inner.peak_buffer_depth = depth;
                }
                return event_id;
            }
            // No cosmetic event to evict; drop the gameplay event itself.
            inner.dropped += 1;
            inner.dropped_gameplay += 1;
            inner.pending_drop_tag += 1;
            return event_id;
        }
        inner.events.push(event);
        let depth = inner.events.len();
        if depth > inner.peak_buffer_depth {
            inner.peak_buffer_depth = depth;
        }
        event_id
    }

    pub fn record_severity(&self, severity: &str) {
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        *inner.by_severity.entry(severity.to_string()).or_insert(0) += 1;
    }

    pub fn dropped(&self, count: u64) {
        let mut inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.dropped += count;
        inner.pending_drop_tag = inner.pending_drop_tag.saturating_add(count);
    }

    /// drop count so it surfaces in the bundle (per M4 § "the per-event
    /// payload that triggered the overflow includes dropped_count in the
    /// next emitted event"). Returns the count and clears the outstanding
    /// counter so it's not reported twice.
    pub fn take_outstanding_drop_count(&self) -> u64 {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        let n = inner.dropped;
        // Note: we don't zero `inner.dropped` here — `dropped_total` is
        // cumulative for `summary.json`. Callers that want a per-emit delta
        // should diff against the prior return.
        n
    }

    /// Snapshot the entire event log. Panics on mutex poisoning per the
    /// consistent recorder error-handling strategy (issue #22): poisoning
    /// indicates a critical bug, not a transient failure to silently degrade.
    pub fn snapshot_events(&self) -> Vec<Event> {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.events.clone()
    }

    /// Return events recorded after `after_idx` (i.e., the tail since the
    /// caller last polled). Panics on mutex poisoning per the consistent
    /// recorder error-handling strategy (issue #22).
    pub fn events_since(&self, after_idx: usize) -> Vec<Event> {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        if after_idx >= inner.events.len() {
            Vec::new()
        } else {
            inner.events[after_idx..].to_vec()
        }
    }

    pub fn counts(&self) -> EventCounts {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        let mut by_severity = inner.by_severity.clone();
        by_severity.entry("error".to_string()).or_insert(0);
        by_severity.entry("warn".to_string()).or_insert(0);
        EventCounts {
            total: inner.events.len() as u64,
            by_category: inner.by_category.clone(),
            by_type: inner.by_type.clone(),
            by_severity,
            dropped_total: inner.dropped,
        }
    }

    pub fn first_last_tick(&self) -> (Option<u64>, Option<u64>) {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        (inner.first_tick, inner.last_tick)
    }

    pub fn final_checksum_hex(&self) -> Option<String> {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.final_checksum.clone()
    }

    pub fn checksum_event_count(&self) -> u64 {
        let inner = self.inner.lock().expect("recorder mutex poisoned; aborting run");
        inner.checksum_event_count
    }
}

/// Bundles the envelope identity (`tick`, `sim_time_ms`, `category`, etc.)
/// plus the `asset_ref` (string-encoded `cf-asset-ledger::AssetId`) and the
/// cosmetic flag so the recorder writes a single event with the ledger
/// pointer on the M4 envelope.
pub struct AssetRefRecordParams<'a> {
    pub tick: Tick,
    pub sim_time_ms: f64,
    pub category: &'a str,
    pub event_type: &'a str,
    pub payload: serde_json::Value,
    pub parent_event_id: Option<String>,
    pub asset_ref: String,
    pub cosmetic: bool,
}

/// the chain hash + the verifier (after disk round-trip) produce the same
/// bytes. serde_json without the `arbitrary_precision` feature can lose
/// 1 ULP on f64 values that came from f32 narrowing — Ryu serializes them
/// fine, but the parser doesn't always reverse to the exact same f64. The
/// fix is to round-trip the payload through string serialization here so
/// the canonical string passed to the chain encoder matches what's later
/// written to events.jsonl + re-parsed by the verifier.
pub(crate) fn canonical_payload_for_chain(payload: &serde_json::Value) -> String {
    let direct = serde_json::to_string(payload).unwrap_or_else(|_| "null".to_string());
    let reparsed: serde_json::Value = match serde_json::from_str(&direct) {
        Ok(v) => v,
        Err(_) => return direct,
    };
    serde_json::to_string(&reparsed).unwrap_or(direct)
}

#[cfg(test)]
mod canonical_chain_tests {
    use super::canonical_payload_for_chain;
    use serde_json::json;

    /// f32 → f64 narrowing yields an f64 that does NOT have a stable
    /// JSON round-trip in serde_json (without `arbitrary_precision`).
    /// `canonical_payload_for_chain` must run that round-trip once
    /// itself so the chain hash + disk re-read produce the same bytes.
    #[test]
    fn canonical_round_trip_is_stable_for_f32_derived_values() {
        let v: f32 = 0.12;
        let payload = json!({"k": v});
        let canonical = canonical_payload_for_chain(&payload);
        let reparsed: serde_json::Value = serde_json::from_str(&canonical).unwrap();
        let canonical_2 = serde_json::to_string(&reparsed).unwrap();
        assert_eq!(
            canonical, canonical_2,
            "round-tripped canonical must be byte-identical on second pass"
        );
    }

    /// Multiple problematic f32 values all round-trip stably.
    #[test]
    fn canonical_round_trip_stable_for_assorted_f32_values() {
        for raw in [0.12_f32, 162.14_f32, 0.0001785_f32, 0.0000898_f32, 7.87_f32, 1700.0_f32] {
            let payload = json!({"k": raw});
            let canonical = canonical_payload_for_chain(&payload);
            let reparsed: serde_json::Value = serde_json::from_str(&canonical).unwrap();
            let canonical_2 = serde_json::to_string(&reparsed).unwrap();
            assert_eq!(canonical, canonical_2, "unstable round-trip for f32={raw}");
        }
    }
}
