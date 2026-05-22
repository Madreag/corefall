//! **M4B § "Save files are smaller"** — baseline + delta-chain encoder.
//!
//! The encoder treats each snapshot as a `serde_json::Value` and emits:
//!
//! 1. **Baseline snapshots** every `cadence_ticks` ticks. These contain the
//!    full world state and are the only points the decoder can resume from
//!    cold.
//! 2. **Delta snapshots** every other snapshot-bearing tick. Each delta
//!    references the most recent baseline by `baseline_event_id` (so the
//!    chain is self-describing) and stores the JSON-Patch-style diff
//!    between the previous tick and the current tick.
//!
//! The encoder is intentionally JSON-Patch-shaped rather than binary so
//! mods can add fields without breaking older deltas. Binary delta is
//! reserved for a future optimization (Powder-Toy-style bitstream); M4B
//! ships the JSON form so the determinism + introspection contracts hold.
//!
//! ## Reconstructor
//!
//! [`reconstruct_chain`] walks `(baseline, [delta_1, delta_2, ...])` and
//! produces a `Vec<Value>` where index `i` is the snapshot at tick `i +
//! baseline_tick`. The reconstructor is the canonical inverse of
//! [`encode_delta`]; replay tooling MUST call this rather than re-implement
//! the JSON patch logic.

use serde::{Deserialize, Serialize};

use crate::{checksum, SaveError};

/// Default cadence: 600 ticks (10 s at 60 Hz). Tuned to keep delta chain
/// depth <= 600 (reconstruction cost stays sub-millisecond). Configurable
/// via `run_manifest.json.delta_baseline_cadence_ticks` when a scenario
/// needs tighter cadence.
pub const DEFAULT_BASELINE_CADENCE_TICKS: u64 = 600;

/// A baseline snapshot in the delta chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BaselineSnapshot {
    /// The tick this baseline was captured at.
    pub tick: u64,
    /// The event id of the `snapshot.baseline_emitted` event that recorded
    /// this baseline; deltas reference this id.
    pub event_id: String,
    /// The full snapshot payload at this tick.
    pub state: serde_json::Value,
    /// Canonical-JSON BLAKE3 of `state`. Pinned on the baseline so the
    /// reconstructor can audit that the recorded payload matches the live
    /// payload at every baseline boundary.
    pub state_checksum_hex: String,
}

impl BaselineSnapshot {
    /// Compute the baseline's `state_checksum_hex` over the canonical JSON
    /// of `state` (matching the cf-replay record-side hash).
    pub fn compute(tick: u64, event_id: String, state: serde_json::Value) -> Result<Self, SaveError> {
        let state_checksum_hex = checksum::canonical_blake3_hex(&state).map_err(SaveError::SerializeJson)?;
        Ok(Self {
            tick,
            event_id,
            state,
            state_checksum_hex,
        })
    }
}

/// A delta snapshot referencing the most recent baseline. The delta is a
/// JSON-Patch-style sequence of operations sufficient to transform
/// `previous_state` into the current tick's state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeltaSnapshot {
    pub tick: u64,
    pub event_id: String,
    /// Event id of the most recent [`BaselineSnapshot`] this delta chains
    /// from. The reconstructor walks back to that baseline + replays every
    /// intermediate delta in order.
    pub baseline_event_id: String,
    /// JSON-Patch operations: each entry is one of:
    /// - `{"op": "set", "path": [..], "value": ...}` — assign a new value.
    /// - `{"op": "remove", "path": [..]}` — drop a key from a map.
    /// - `{"op": "list_push", "path": [..], "value": ...}` — append to a vec.
    /// - `{"op": "list_set", "path": [..], "index": n, "value": ...}` — replace.
    /// - `{"op": "list_trunc", "path": [..], "length": n}` — truncate a vec.
    pub ops: Vec<DeltaOp>,
}

/// A single delta operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum DeltaOp {
    Set {
        path: Vec<String>,
        value: serde_json::Value,
    },
    Remove {
        path: Vec<String>,
    },
    ListPush {
        path: Vec<String>,
        value: serde_json::Value,
    },
    ListSet {
        path: Vec<String>,
        index: usize,
        value: serde_json::Value,
    },
    ListTrunc {
        path: Vec<String>,
        length: usize,
    },
}

/// Compute the JSON-Patch-style delta between `previous` and `current`.
///
/// The diff is structural: object key adds + removes + value changes, list
/// length increases + index-wise replacements + truncations. The output is
/// the minimal sequence of ops that, when applied to `previous`, yields
/// `current`. For the determinism contract this matters: the encoder
/// produces a stable byte representation for a given input pair.
pub fn diff(previous: &serde_json::Value, current: &serde_json::Value) -> Vec<DeltaOp> {
    let mut ops = Vec::new();
    diff_recursive(&mut ops, &mut Vec::new(), previous, current);
    ops
}

fn diff_recursive(
    ops: &mut Vec<DeltaOp>,
    path: &mut Vec<String>,
    previous: &serde_json::Value,
    current: &serde_json::Value,
) {
    if previous == current {
        return;
    }
    match (previous, current) {
        (serde_json::Value::Object(prev_map), serde_json::Value::Object(curr_map)) => {
            // Removed keys.
            for key in prev_map.keys() {
                if !curr_map.contains_key(key) {
                    path.push(key.clone());
                    ops.push(DeltaOp::Remove { path: path.clone() });
                    path.pop();
                }
            }
            // Added + changed keys.
            for (key, curr_value) in curr_map {
                path.push(key.clone());
                match prev_map.get(key) {
                    Some(prev_value) => diff_recursive(ops, path, prev_value, curr_value),
                    None => ops.push(DeltaOp::Set {
                        path: path.clone(),
                        value: curr_value.clone(),
                    }),
                }
                path.pop();
            }
        }
        (serde_json::Value::Array(prev_arr), serde_json::Value::Array(curr_arr)) => {
            let common = prev_arr.len().min(curr_arr.len());
            for (i, (prev_item, curr_item)) in prev_arr.iter().zip(curr_arr.iter()).take(common).enumerate() {
                if prev_item != curr_item {
                    ops.push(DeltaOp::ListSet {
                        path: path.clone(),
                        index: i,
                        value: curr_item.clone(),
                    });
                }
            }
            if curr_arr.len() > prev_arr.len() {
                for item in &curr_arr[prev_arr.len()..] {
                    ops.push(DeltaOp::ListPush {
                        path: path.clone(),
                        value: item.clone(),
                    });
                }
            } else if curr_arr.len() < prev_arr.len() {
                ops.push(DeltaOp::ListTrunc {
                    path: path.clone(),
                    length: curr_arr.len(),
                });
            }
        }
        _ => {
            ops.push(DeltaOp::Set {
                path: path.clone(),
                value: current.clone(),
            });
        }
    }
}

/// Apply a single delta op to a snapshot value. Errors when the path or
/// shape doesn't match (the reconstructor should never see this in a
/// well-formed delta chain).
pub fn apply_op(target: &mut serde_json::Value, op: &DeltaOp) -> Result<(), DeltaError> {
    match op {
        DeltaOp::Set { path, value } => set_at(target, path, value.clone()),
        DeltaOp::Remove { path } => remove_at(target, path),
        DeltaOp::ListPush { path, value } => list_push_at(target, path, value.clone()),
        DeltaOp::ListSet { path, index, value } => list_set_at(target, path, *index, value.clone()),
        DeltaOp::ListTrunc { path, length } => list_trunc_at(target, path, *length),
    }
}

/// Encode a delta from `previous` to `current` referencing `baseline_event_id`.
pub fn encode_delta(
    tick: u64,
    event_id: String,
    baseline_event_id: String,
    previous: &serde_json::Value,
    current: &serde_json::Value,
) -> DeltaSnapshot {
    DeltaSnapshot {
        tick,
        event_id,
        baseline_event_id,
        ops: diff(previous, current),
    }
}

/// Apply every delta op in order, mutating `target` in place.
pub fn apply_delta(target: &mut serde_json::Value, delta: &DeltaSnapshot) -> Result<(), DeltaError> {
    for op in &delta.ops {
        apply_op(target, op)?;
    }
    Ok(())
}

/// Reconstruct every snapshot in `[baseline, baseline + deltas.len()]`.
/// Returns a `Vec<serde_json::Value>` where element `i` corresponds to
/// `baseline.tick + i`. Useful for replay tooling that wants the full
/// frame-by-frame reconstruction.
pub fn reconstruct_chain(
    baseline: &BaselineSnapshot,
    deltas: &[DeltaSnapshot],
) -> Result<Vec<serde_json::Value>, DeltaError> {
    // Sanity: every delta MUST reference this baseline.
    for delta in deltas {
        if delta.baseline_event_id != baseline.event_id {
            return Err(DeltaError::ChainMismatch {
                baseline_event_id: baseline.event_id.clone(),
                delta_baseline_event_id: delta.baseline_event_id.clone(),
            });
        }
    }
    let mut frames = Vec::with_capacity(deltas.len() + 1);
    let mut cursor = baseline.state.clone();
    frames.push(cursor.clone());
    for delta in deltas {
        apply_delta(&mut cursor, delta)?;
        frames.push(cursor.clone());
    }
    Ok(frames)
}

/// returns the reconstructed snapshot at `tick` from the supplied
/// `(baseline, deltas)` pair, by walking forward from the baseline and
/// applying every delta whose tick is <= `tick`.
pub fn reconstruct_at(
    baseline: &BaselineSnapshot,
    deltas: &[DeltaSnapshot],
    tick: u64,
) -> Result<serde_json::Value, DeltaError> {
    if tick < baseline.tick {
        return Err(DeltaError::TickBeforeBaseline {
            baseline_tick: baseline.tick,
            requested_tick: tick,
        });
    }
    let mut cursor = baseline.state.clone();
    for delta in deltas {
        if delta.tick > tick {
            break;
        }
        if delta.baseline_event_id != baseline.event_id {
            return Err(DeltaError::ChainMismatch {
                baseline_event_id: baseline.event_id.clone(),
                delta_baseline_event_id: delta.baseline_event_id.clone(),
            });
        }
        apply_delta(&mut cursor, delta)?;
    }
    Ok(cursor)
}

#[derive(Debug, thiserror::Error)]
pub enum DeltaError {
    #[error("delta path resolves to non-object/non-array: {0:?}")]
    PathShapeMismatch(Vec<String>),
    #[error("delta path traverses missing key: {0:?}")]
    MissingPath(Vec<String>),
    #[error("delta list index {index} out of bounds (length {length}) at path {path:?}")]
    ListIndexOutOfBounds {
        path: Vec<String>,
        index: usize,
        length: usize,
    },
    #[error(
        "delta chain mismatch: baseline event_id {baseline_event_id} != delta.baseline_event_id {delta_baseline_event_id}"
    )]
    ChainMismatch {
        baseline_event_id: String,
        delta_baseline_event_id: String,
    },
    #[error("reconstruct_at: requested tick {requested_tick} precedes baseline tick {baseline_tick}")]
    TickBeforeBaseline { baseline_tick: u64, requested_tick: u64 },
}

fn set_at(target: &mut serde_json::Value, path: &[String], value: serde_json::Value) -> Result<(), DeltaError> {
    if path.is_empty() {
        *target = value;
        return Ok(());
    }
    let parent = navigate_mut(target, &path[..path.len() - 1])?;
    let key = path
        .last()
        .ok_or_else(|| DeltaError::PathShapeMismatch(path.to_vec()))?;
    match parent {
        serde_json::Value::Object(map) => {
            map.insert(key.clone(), value);
            Ok(())
        }
        serde_json::Value::Null => {
            // Auto-create object container at the parent (rare; happens when
            // the predecessor snapshot was `null`).
            let mut map = serde_json::Map::new();
            map.insert(key.clone(), value);
            *parent = serde_json::Value::Object(map);
            Ok(())
        }
        _ => Err(DeltaError::PathShapeMismatch(path.to_vec())),
    }
}

fn remove_at(target: &mut serde_json::Value, path: &[String]) -> Result<(), DeltaError> {
    if path.is_empty() {
        *target = serde_json::Value::Null;
        return Ok(());
    }
    let parent = navigate_mut(target, &path[..path.len() - 1])?;
    let key = path
        .last()
        .ok_or_else(|| DeltaError::PathShapeMismatch(path.to_vec()))?;
    match parent {
        serde_json::Value::Object(map) => {
            map.remove(key);
            Ok(())
        }
        _ => Err(DeltaError::PathShapeMismatch(path.to_vec())),
    }
}

fn list_push_at(
    target: &mut serde_json::Value,
    path: &[String],
    value: serde_json::Value,
) -> Result<(), DeltaError> {
    let parent = navigate_mut(target, path)?;
    match parent {
        serde_json::Value::Array(arr) => {
            arr.push(value);
            Ok(())
        }
        _ => Err(DeltaError::PathShapeMismatch(path.to_vec())),
    }
}

fn list_set_at(
    target: &mut serde_json::Value,
    path: &[String],
    index: usize,
    value: serde_json::Value,
) -> Result<(), DeltaError> {
    let parent = navigate_mut(target, path)?;
    match parent {
        serde_json::Value::Array(arr) => {
            if index >= arr.len() {
                Err(DeltaError::ListIndexOutOfBounds {
                    path: path.to_vec(),
                    index,
                    length: arr.len(),
                })
            } else {
                arr[index] = value;
                Ok(())
            }
        }
        _ => Err(DeltaError::PathShapeMismatch(path.to_vec())),
    }
}

fn list_trunc_at(target: &mut serde_json::Value, path: &[String], length: usize) -> Result<(), DeltaError> {
    let parent = navigate_mut(target, path)?;
    match parent {
        serde_json::Value::Array(arr) => {
            arr.truncate(length);
            Ok(())
        }
        _ => Err(DeltaError::PathShapeMismatch(path.to_vec())),
    }
}

fn navigate_mut<'a>(target: &'a mut serde_json::Value, path: &[String]) -> Result<&'a mut serde_json::Value, DeltaError> {
    let mut cursor = target;
    for key in path {
        cursor = match cursor {
            serde_json::Value::Object(map) => map
                .get_mut(key)
                .ok_or_else(|| DeltaError::MissingPath(path.to_vec()))?,
            _ => return Err(DeltaError::PathShapeMismatch(path.to_vec())),
        };
    }
    Ok(cursor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn diff_empty_when_states_equal() {
        let a = json!({"hp": 100, "team": "blue"});
        assert!(diff(&a, &a).is_empty());
    }

    #[test]
    fn diff_set_for_changed_primitive() {
        let a = json!({"hp": 100});
        let b = json!({"hp": 90});
        let ops = diff(&a, &b);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], DeltaOp::Set { path, value } if path == &["hp".to_string()] && value == &json!(90)));
    }

    #[test]
    fn diff_remove_for_dropped_key() {
        let a = json!({"hp": 100, "ammo": 30});
        let b = json!({"hp": 100});
        let ops = diff(&a, &b);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], DeltaOp::Remove { path } if path == &["ammo".to_string()]));
    }

    #[test]
    fn diff_add_for_new_key() {
        let a = json!({"hp": 100});
        let b = json!({"hp": 100, "ammo": 30});
        let ops = diff(&a, &b);
        assert_eq!(ops.len(), 1);
        assert!(matches!(&ops[0], DeltaOp::Set { path, .. } if path == &["ammo".to_string()]));
    }

    #[test]
    fn diff_list_push_and_set_and_trunc() {
        let a = json!([1, 2, 3]);
        let b = json!([1, 2, 4, 5]);
        let mut target = a.clone();
        let ops = diff(&a, &b);
        for op in &ops {
            apply_op(&mut target, op).unwrap();
        }
        assert_eq!(target, b);

        let c = json!([1]);
        let mut target2 = b.clone();
        let ops = diff(&b, &c);
        for op in &ops {
            apply_op(&mut target2, op).unwrap();
        }
        assert_eq!(target2, c);
    }

    #[test]
    fn apply_chain_round_trips() {
        let frames = [
            json!({"hp": 100, "ammo": 30, "team": "blue"}),
            json!({"hp": 90, "ammo": 28, "team": "blue"}),
            json!({"hp": 75, "ammo": 25, "team": "blue", "afflictions": ["bleeding"]}),
            json!({"hp": 60, "ammo": 20, "team": "blue", "afflictions": ["bleeding", "concussion"]}),
        ];
        let baseline = BaselineSnapshot::compute(0, "ev-baseline".to_string(), frames[0].clone()).unwrap();
        let mut deltas = Vec::new();
        for (i, pair) in frames.windows(2).enumerate() {
            let delta = encode_delta(
                u64::try_from(i + 1).unwrap(),
                format!("ev-delta-{}", i + 1),
                "ev-baseline".to_string(),
                &pair[0],
                &pair[1],
            );
            deltas.push(delta);
        }
        let reconstructed = reconstruct_chain(&baseline, &deltas).unwrap();
        assert_eq!(reconstructed.len(), frames.len());
        for (i, frame) in frames.iter().enumerate() {
            assert_eq!(&reconstructed[i], frame, "frame {i} differs");
        }
    }

    #[test]
    fn reconstruct_at_walks_forward_only_to_requested_tick() {
        let frames = [
            json!({"hp": 100}),
            json!({"hp": 90}),
            json!({"hp": 75}),
            json!({"hp": 60}),
        ];
        let baseline = BaselineSnapshot::compute(0, "b0".to_string(), frames[0].clone()).unwrap();
        let mut deltas = Vec::new();
        for (i, pair) in frames.windows(2).enumerate() {
            deltas.push(encode_delta(
                u64::try_from(i + 1).unwrap(),
                format!("d{}", i + 1),
                "b0".to_string(),
                &pair[0],
                &pair[1],
            ));
        }
        let at1 = reconstruct_at(&baseline, &deltas, 1).unwrap();
        assert_eq!(at1, frames[1]);
        let at3 = reconstruct_at(&baseline, &deltas, 3).unwrap();
        assert_eq!(at3, frames[3]);
    }

    #[test]
    fn chain_mismatch_detected_on_wrong_baseline_event_id() {
        let baseline = BaselineSnapshot::compute(0, "b0".to_string(), json!({"hp": 100})).unwrap();
        let delta = encode_delta(1, "d1".to_string(), "wrong-baseline".to_string(), &json!({"hp": 100}), &json!({"hp": 90}));
        let err = reconstruct_chain(&baseline, std::slice::from_ref(&delta)).err().unwrap();
        assert!(matches!(err, DeltaError::ChainMismatch { .. }));
    }
}
