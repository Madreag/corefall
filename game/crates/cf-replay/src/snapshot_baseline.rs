//! **M4B § "snapshot.baseline_emitted"** — baseline snapshot writer.
//!
//! Paired with [`crate::snapshot_delta`]. The recorder emits one
//! `snapshot.baseline_emitted` event every `delta_baseline_cadence_ticks`
//! ticks (default 600) containing the full world state at that boundary;
//! between baselines it emits `snapshot.delta_emitted` events referencing
//! the most recent baseline event id.

use serde::{Deserialize, Serialize};

use crate::Recorder;
use cf_sim_core::Tick;

pub const EVENT_CATEGORY: &str = "snapshot";
pub const EVENT_TYPE_BASELINE: &str = "baseline_emitted";

/// Payload shape for `snapshot.baseline_emitted`. Embeds the full state
/// blob plus the canonical-JSON BLAKE3 of that state (so the
/// reconstructor can audit per-baseline integrity).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselinePayload {
    pub world_tick: u64,
    pub state: serde_json::Value,
    pub state_checksum_hex: String,
    pub cadence_ticks: u64,
}

/// Emit a `snapshot.baseline_emitted` event into `recorder` at `tick`.
/// Returns the recorder's assigned event id so subsequent delta payloads
/// can chain from it.
pub fn emit_baseline(
    recorder: &Recorder,
    tick: Tick,
    sim_time_ms: f64,
    world_tick: u64,
    state: serde_json::Value,
    cadence_ticks: u64,
) -> Result<String, BaselineError> {
    let canonical = serde_json::to_string(&state).map_err(BaselineError::SerializeState)?;
    let state_checksum_hex = hex::encode(blake3::hash(canonical.as_bytes()).as_bytes());
    let payload = BaselinePayload {
        world_tick,
        state,
        state_checksum_hex,
        cadence_ticks,
    };
    let value = serde_json::to_value(payload).map_err(BaselineError::SerializePayload)?;
    let event_id = recorder.record(tick, sim_time_ms, EVENT_CATEGORY, EVENT_TYPE_BASELINE, value, None);
    Ok(event_id)
}

#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    #[error("baseline state serialization failed: {0}")]
    SerializeState(#[source] serde_json::Error),
    #[error("baseline payload serialization failed: {0}")]
    SerializePayload(#[source] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emit_baseline_records_one_event_with_state_checksum_in_payload() {
        let recorder = Recorder::new("m4b_test_run".to_string());
        let state = serde_json::json!({"actors": [], "tick": 0});
        let id = emit_baseline(&recorder, Tick(0), 0.0, 0, state.clone(), 600).unwrap();
        let events = recorder.snapshot_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, id);
        assert_eq!(events[0].category, EVENT_CATEGORY);
        assert_eq!(events[0].event_type, EVENT_TYPE_BASELINE);
        let payload = &events[0].payload;
        assert_eq!(payload.get("world_tick").and_then(|v| v.as_u64()), Some(0));
        assert_eq!(payload.get("cadence_ticks").and_then(|v| v.as_u64()), Some(600));
        let cs = payload
            .get("state_checksum_hex")
            .and_then(|v| v.as_str())
            .expect("state_checksum_hex present");
        assert_eq!(cs.len(), 64);
    }
}
