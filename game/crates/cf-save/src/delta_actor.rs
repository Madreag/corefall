//! **M4B § "per-actor delta"** — actor-specific delta wrapper.
//!
//! Convenience layer over [`crate::delta`] for per-actor diffs. The encoder
//! lifts a [`crate::SaveBlob`] into a `serde_json::Value` then delegates
//! to the generic JSON differ. This keeps the per-actor path uniform with
//! the per-chunk + per-projectile paths and lets mods extend `SaveBlob`
//! without changing the delta encoder.

use crate::{
    delta::{diff, encode_delta, BaselineSnapshot, DeltaSnapshot},
    SaveBlob, SaveError,
};

/// Build a per-actor baseline. The actor's full SaveBlob is serialized into
/// a canonical JSON value; subsequent deltas chain from this baseline.
pub fn actor_baseline(actor_id: u64, tick: u64, blob: &SaveBlob) -> Result<BaselineSnapshot, SaveError> {
    let event_id = format!("actor_baseline:{actor_id}:{tick}");
    let state = serde_json::to_value(blob).map_err(SaveError::SerializeJson)?;
    BaselineSnapshot::compute(tick, event_id, state)
}

/// Encode a per-actor delta. Returns `Ok(None)` when the two blobs are
/// identical (the encoder elides empty deltas — they cost a line in the
/// event log but carry no information).
pub fn actor_delta(
    actor_id: u64,
    tick: u64,
    baseline_event_id: String,
    previous: &SaveBlob,
    current: &SaveBlob,
) -> Result<Option<DeltaSnapshot>, SaveError> {
    let prev_value = serde_json::to_value(previous).map_err(SaveError::SerializeJson)?;
    let curr_value = serde_json::to_value(current).map_err(SaveError::SerializeJson)?;
    let ops = diff(&prev_value, &curr_value);
    if ops.is_empty() {
        return Ok(None);
    }
    let event_id = format!("actor_delta:{actor_id}:{tick}");
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
    use crate::{V1_0_0, V2_0_0};
    use std::collections::BTreeMap;

    fn blob_at(version: crate::SaveSchemaVersion) -> SaveBlob {
        SaveBlob {
            schema_version: version,
            actor_id: 7,
            team: "blue".to_string(),
            origin_id: "human".to_string(),
            position: [10.0, 20.0],
            velocity: [0.0, 0.0],
            aim: [1.0, 0.0],
            hp: 100.0,
            hp_max: 100.0,
            on_ground: true,
            status: "stable".to_string(),
            selected_slot: 0,
            rifle_preset: None,
            rifle_ammo: None,
            rifle_reload_remaining_ticks: None,
            chassis: None,
            gear_dropped_by_limb_loss: false,
            chassis_detached: false,
            afflictions: vec![],
            crouch_active: false,
            climb_active: false,
            jet_active: false,
            mod_payload: BTreeMap::new(),
        }
    }

    #[test]
    fn baseline_and_delta_round_trip() {
        let prev = blob_at(V2_0_0);
        let mut curr = prev.clone();
        curr.hp = 75.0;
        curr.position = [12.0, 20.5];
        let baseline = actor_baseline(7, 0, &prev).unwrap();
        let delta = actor_delta(7, 1, baseline.event_id.clone(), &prev, &curr)
            .unwrap()
            .expect("non-identical state must produce a delta");
        let frames = crate::delta::reconstruct_chain(&baseline, std::slice::from_ref(&delta)).unwrap();
        let recovered = serde_json::from_value::<SaveBlob>(frames[1].clone()).unwrap();
        assert_eq!(recovered, curr);
    }

    #[test]
    fn identical_state_emits_no_delta() {
        let a = blob_at(V2_0_0);
        let baseline = actor_baseline(7, 0, &a).unwrap();
        let delta = actor_delta(7, 1, baseline.event_id, &a, &a).unwrap();
        assert!(delta.is_none());
    }

    #[test]
    fn schema_version_change_surfaces_in_delta() {
        let mut a = blob_at(V1_0_0);
        let mut b = blob_at(V2_0_0);
        b.actor_id = a.actor_id;
        let baseline = actor_baseline(7, 0, &a).unwrap();
        a.hp = 80.0;
        let _ = a;
        let delta = actor_delta(7, 1, baseline.event_id.clone(), &blob_at(V1_0_0), &b).unwrap();
        assert!(delta.is_some(), "version bump must surface as a delta op");
    }
}
