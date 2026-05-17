//! **M4B § "per-projectile delta"** — projectile-specific delta wrapper.
//!
//! Convenience layer over [`crate::delta`] for per-projectile diffs.
//! Projectiles are opaque JSON so mods can attach extra fields (e.g.,
//! attached spalling state, energy reservoir, etc.).

use crate::{
    delta::{diff, encode_delta, BaselineSnapshot, DeltaSnapshot},
    ProjectileSnapshot, SaveError,
};

pub fn projectile_baseline(
    projectile_id: u64,
    tick: u64,
    snapshot: &ProjectileSnapshot,
) -> Result<BaselineSnapshot, SaveError> {
    let event_id = format!("projectile_baseline:{projectile_id}:{tick}");
    let state = serde_json::to_value(snapshot).map_err(SaveError::SerializeJson)?;
    BaselineSnapshot::compute(tick, event_id, state)
}

pub fn projectile_delta(
    projectile_id: u64,
    tick: u64,
    baseline_event_id: String,
    previous: &ProjectileSnapshot,
    current: &ProjectileSnapshot,
) -> Result<Option<DeltaSnapshot>, SaveError> {
    let prev_value = serde_json::to_value(previous).map_err(SaveError::SerializeJson)?;
    let curr_value = serde_json::to_value(current).map_err(SaveError::SerializeJson)?;
    let ops = diff(&prev_value, &curr_value);
    if ops.is_empty() {
        return Ok(None);
    }
    let event_id = format!("projectile_delta:{projectile_id}:{tick}");
    Ok(Some(encode_delta(
        tick,
        event_id,
        baseline_event_id,
        &prev_value,
        &curr_value,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn snap(id: u64, body: serde_json::Value) -> ProjectileSnapshot {
        ProjectileSnapshot { id, state: body }
    }

    #[test]
    fn baseline_then_delta_round_trip() {
        let a = snap(42, json!({"pos": [10.0, 20.0], "vel": [50.0, 0.0], "ttl": 60}));
        let b = snap(42, json!({"pos": [11.0, 20.0], "vel": [50.0, -1.0], "ttl": 59}));
        let baseline = projectile_baseline(42, 0, &a).unwrap();
        let delta = projectile_delta(42, 1, baseline.event_id.clone(), &a, &b)
            .unwrap()
            .expect("non-identical state must produce a delta");
        let frames = crate::delta::reconstruct_chain(&baseline, std::slice::from_ref(&delta)).unwrap();
        let recovered: ProjectileSnapshot = serde_json::from_value(frames[1].clone()).unwrap();
        assert_eq!(recovered, b);
    }
}
