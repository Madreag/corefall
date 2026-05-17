//! **M4B § "per-terrain-chunk delta"** — chunk-specific delta wrapper.
//!
//! Convenience layer over [`crate::delta`] for per-terrain-chunk diffs.
//! Chunks are opaque JSON (the chunk-side RLE encoder lives in cf-terrain);
//! M4B uses the canonical Value form so the same JSON-Patch differ works
//! uniformly across actors, chunks, and projectiles.

use crate::{
    delta::{diff, encode_delta, BaselineSnapshot, DeltaSnapshot},
    SaveError, TerrainChunkSnapshot,
};

/// Build a per-chunk baseline.
pub fn chunk_baseline(
    chunk_id: &str,
    tick: u64,
    chunk: &TerrainChunkSnapshot,
) -> Result<BaselineSnapshot, SaveError> {
    let event_id = format!("chunk_baseline:{chunk_id}:{tick}");
    let state = serde_json::to_value(chunk).map_err(SaveError::SerializeJson)?;
    BaselineSnapshot::compute(tick, event_id, state)
}

/// Encode a per-chunk delta. Returns `Ok(None)` when the chunks are
/// identical (the encoder elides empty deltas).
pub fn chunk_delta(
    chunk_id: &str,
    tick: u64,
    baseline_event_id: String,
    previous: &TerrainChunkSnapshot,
    current: &TerrainChunkSnapshot,
) -> Result<Option<DeltaSnapshot>, SaveError> {
    let prev_value = serde_json::to_value(previous).map_err(SaveError::SerializeJson)?;
    let curr_value = serde_json::to_value(current).map_err(SaveError::SerializeJson)?;
    let ops = diff(&prev_value, &curr_value);
    if ops.is_empty() {
        return Ok(None);
    }
    let event_id = format!("chunk_delta:{chunk_id}:{tick}");
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

    fn chunk(id: &str, body: serde_json::Value) -> TerrainChunkSnapshot {
        TerrainChunkSnapshot {
            chunk_id: id.to_string(),
            state: body,
        }
    }

    #[test]
    fn baseline_then_delta_round_trip() {
        let a = chunk("0,0", json!({"materials": [1, 1, 2, 3], "dirty": false}));
        let b = chunk("0,0", json!({"materials": [1, 1, 2, 9, 9], "dirty": true}));
        let baseline = chunk_baseline("0,0", 0, &a).unwrap();
        let delta = chunk_delta("0,0", 1, baseline.event_id.clone(), &a, &b)
            .unwrap()
            .expect("non-identical state must produce a delta");
        let frames = crate::delta::reconstruct_chain(&baseline, std::slice::from_ref(&delta)).unwrap();
        let recovered: TerrainChunkSnapshot = serde_json::from_value(frames[1].clone()).unwrap();
        assert_eq!(recovered, b);
    }
}
