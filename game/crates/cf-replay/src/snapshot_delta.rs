//! **M4B § "snapshot.delta_emitted"** — delta snapshot writer.
//!
//! Paired with [`crate::snapshot_baseline`]. Between baselines, the
//! recorder emits one `snapshot.delta_emitted` event per cadence
//! containing a JSON-Patch-style diff against the previous tick's state,
//! referencing the most recent baseline event id.
//!
//! The delta encoder is owned by `cf-save::delta`; this module is the
//! cf-replay-side wrapper that adapts that encoder to the cf-replay
//! recorder envelope.

use serde::{Deserialize, Serialize};

use crate::Recorder;
use cf_sim_core::Tick;

pub const EVENT_CATEGORY: &str = "snapshot";
pub const EVENT_TYPE_DELTA: &str = "delta_emitted";

/// Payload shape for `snapshot.delta_emitted`. References the most recent
/// baseline by event id; carries the JSON-Patch ops between
/// previous-tick and current-tick world state. The cadence is repeated
/// here so the bundle is self-describing (consumers don't have to walk
/// the manifest to know it).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeltaPayload {
    pub world_tick: u64,
    pub baseline_event_id: String,
    pub baseline_tick: u64,
    pub ops: serde_json::Value,
}

/// Emit a `snapshot.delta_emitted` event into `recorder`. `ops` is the
/// serialized `Vec<cf_save::delta::DeltaOp>` as `serde_json::Value`
/// (typed at the cf-save edge so cf-replay stays free of the cf-save
/// dependency).
pub fn emit_delta(
    recorder: &Recorder,
    tick: Tick,
    sim_time_ms: f64,
    world_tick: u64,
    baseline_event_id: String,
    baseline_tick: u64,
    ops: serde_json::Value,
) -> Result<String, DeltaError> {
    let payload = DeltaPayload {
        world_tick,
        baseline_event_id: baseline_event_id.clone(),
        baseline_tick,
        ops,
    };
    let value = serde_json::to_value(payload).map_err(DeltaError::SerializePayload)?;
    let event_id = recorder.record(
        tick,
        sim_time_ms,
        EVENT_CATEGORY,
        EVENT_TYPE_DELTA,
        value,
        Some(baseline_event_id),
    );
    Ok(event_id)
}

#[derive(Debug, thiserror::Error)]
pub enum DeltaError {
    #[error("delta payload serialization failed: {0}")]
    SerializePayload(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_delta_records_one_event_chained_to_baseline_via_parent_event_id() {
        let recorder = Recorder::new("m4b_test_delta".to_string());
        let baseline_id = "baseline_ev_0".to_string();
        let ops = serde_json::json!([{"op":"set","path":["hp"],"value":90}]);
        let id = emit_delta(&recorder, Tick(1), 16.6, 1, baseline_id.clone(), 0, ops).unwrap();
        let events = recorder.snapshot_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, id);
        assert_eq!(events[0].category, EVENT_CATEGORY);
        assert_eq!(events[0].event_type, EVENT_TYPE_DELTA);
        assert_eq!(events[0].parent_event_id.as_deref(), Some(baseline_id.as_str()));
        let payload = &events[0].payload;
        assert_eq!(
            payload.get("baseline_event_id").and_then(|v| v.as_str()),
            Some(baseline_id.as_str())
        );
    }
}
