//! M8A § Recorder shard merge contract.
//!
//! Per-thread `RecorderShard`s buffer events with shard-local
//! `(tick, monotonic_seq_in_shard)` ids. At the `RecorderMerge` stage:
//!
//! 1. Each shard sorts its events by `monotonic_seq_in_shard` (already
//!    in insertion order).
//! 2. Cross-shard merge orders events by `(tick, shard_id,
//!    monotonic_seq_in_shard)`. `shard_id` is a stable per-system-stage
//!    assignment, NOT the runtime thread id.
//! 3. After merge, every event is re-stamped with canonical `event_id =
//!    (tick, canonical_seq)` where `canonical_seq` is the post-merge
//!    position.
//! 4. `parent_event_id` references are re-mapped from shard-local ids to
//!    canonical ids during merge.
//!
//! This preserves determinism across thread-scheduling variance: same
//! input → same per-shard event sequence → same merge order → same
//! canonical event stream → same blake3 checksum.

use serde::{Deserialize, Serialize};

/// Stable per-system-stage shard identifier; assigned at engine init
/// based on the SimStage's position in the canonical dependency graph,
/// not on runtime thread id.
pub type ShardId = u8;

/// Shard-local monotonic event sequence number within a tick.
pub type MonotonicSeq = u32;

/// Post-merge canonical event identifier within a tick.
pub type CanonicalSeq = u64;

/// A shard-local recorder event before canonical re-stamping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShardEvent {
    pub tick: u64,
    pub shard_id: ShardId,
    pub monotonic_seq: MonotonicSeq,
    pub category: String,
    pub event_type: String,
    pub payload: Vec<u8>,
    pub parent_shard_event: Option<(ShardId, MonotonicSeq)>,
}

/// A post-merge canonical event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalEvent {
    pub tick: u64,
    pub canonical_seq: CanonicalSeq,
    pub category: String,
    pub event_type: String,
    pub payload: Vec<u8>,
    pub parent_canonical_seq: Option<CanonicalSeq>,
}

/// Per-thread event buffer. Events insert in monotonic order; the merge
/// reads the buffer once.
#[derive(Debug, Clone, Default)]
pub struct RecorderShard {
    pub shard_id: ShardId,
    pub events: Vec<ShardEvent>,
}

impl RecorderShard {
    pub fn new(shard_id: ShardId) -> Self {
        Self {
            shard_id,
            events: Vec::new(),
        }
    }

    pub fn push(&mut self, tick: u64, category: impl Into<String>, event_type: impl Into<String>, payload: Vec<u8>) -> MonotonicSeq {
        let seq = self.events.len() as MonotonicSeq;
        self.events.push(ShardEvent {
            tick,
            shard_id: self.shard_id,
            monotonic_seq: seq,
            category: category.into(),
            event_type: event_type.into(),
            payload,
            parent_shard_event: None,
        });
        seq
    }

    pub fn push_with_parent(
        &mut self,
        tick: u64,
        category: impl Into<String>,
        event_type: impl Into<String>,
        payload: Vec<u8>,
        parent: (ShardId, MonotonicSeq),
    ) -> MonotonicSeq {
        let seq = self.events.len() as MonotonicSeq;
        self.events.push(ShardEvent {
            tick,
            shard_id: self.shard_id,
            monotonic_seq: seq,
            category: category.into(),
            event_type: event_type.into(),
            payload,
            parent_shard_event: Some(parent),
        });
        seq
    }
}

/// Merge per-thread shards into a canonical event stream. Sorts by
/// `(tick, shard_id, monotonic_seq)` and re-stamps `canonical_seq` for
/// each event; re-maps `parent_event_id` references.
pub fn merge_shards_canonical(shards: &[RecorderShard]) -> Vec<CanonicalEvent> {
    let mut events: Vec<&ShardEvent> = shards.iter().flat_map(|s| s.events.iter()).collect();
    events.sort_by_key(|e| (e.tick, e.shard_id, e.monotonic_seq));

    let mut canonical_seq_for: std::collections::BTreeMap<(ShardId, MonotonicSeq), CanonicalSeq> =
        std::collections::BTreeMap::new();
    let mut out = Vec::with_capacity(events.len());
    for (i, ev) in events.iter().enumerate() {
        let canonical_seq = i as CanonicalSeq;
        canonical_seq_for.insert((ev.shard_id, ev.monotonic_seq), canonical_seq);
        let parent_canonical_seq = ev
            .parent_shard_event
            .as_ref()
            .and_then(|pid| canonical_seq_for.get(pid).copied());
        out.push(CanonicalEvent {
            tick: ev.tick,
            canonical_seq,
            category: ev.category.clone(),
            event_type: ev.event_type.clone(),
            payload: ev.payload.clone(),
            parent_canonical_seq,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_orders_by_tick_then_shard_then_seq() {
        let mut s0 = RecorderShard::new(0);
        s0.push(2, "actor", "moved", vec![]);
        s0.push(0, "actor", "moved", vec![]);
        let mut s1 = RecorderShard::new(1);
        s1.push(0, "actor", "fired", vec![]);
        s1.push(1, "actor", "fired", vec![]);
        let merged = merge_shards_canonical(&[s0, s1]);
        assert_eq!(merged.len(), 4);
        assert_eq!((merged[0].tick, merged[0].canonical_seq), (0, 0));
        assert_eq!((merged[1].tick, merged[1].canonical_seq), (0, 1));
        assert_eq!((merged[2].tick, merged[2].canonical_seq), (1, 2));
        assert_eq!((merged[3].tick, merged[3].canonical_seq), (2, 3));
    }

    #[test]
    fn merge_remaps_parent_event_id() {
        let mut s0 = RecorderShard::new(0);
        let parent_seq = s0.push(0, "actor", "fired", vec![]);
        let _child_seq = s0.push_with_parent(0, "actor", "hit", vec![], (0, parent_seq));
        let merged = merge_shards_canonical(&[s0]);
        assert_eq!(merged[0].parent_canonical_seq, None);
        assert_eq!(merged[1].parent_canonical_seq, Some(0));
    }

    #[test]
    fn merge_is_byte_identical_across_orderings() {
        let mut s0 = RecorderShard::new(0);
        s0.push(0, "actor", "a", b"a".to_vec());
        s0.push(0, "actor", "b", b"b".to_vec());

        let mut s1 = RecorderShard::new(1);
        s1.push(0, "ai", "c", b"c".to_vec());

        let merged1 = merge_shards_canonical(&[s0.clone(), s1.clone()]);
        let merged2 = merge_shards_canonical(&[s1, s0]);
        assert_eq!(merged1, merged2, "shard input order must not affect canonical output");
    }
}
