//! M8A § Snapshot field contract (Powder Toy-derived, Corefall-localized).
//!
//! Per M8A spec § Snapshot field contract: `tick + rng_state + chunks +
//! actors + projectiles + mission` is the minimum byte set required to
//! byte-identically restore the sim. Everything else is forward-compat
//! for later milestones.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// anchors (tick + rng_state + engine_build + content_hashes); chunks /
/// actors / projectiles / mission stay as opaque payloads at M8A
/// (M9+ wires the typed inner snapshots).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// Determinism anchor — primary key. Must round-trip byte-identical
    /// across server topologies.
    pub tick: u64,
    /// 32-byte RNG state snapshot. Restored before any sim step on
    /// rollback or replay.
    pub rng_state: [u8; 32],
    /// Semver of the engine binary that wrote the snapshot.
    pub engine_build: String,
    /// Mod / scenario / weapon / actor content hashes.
    pub content_hashes: BTreeMap<String, String>,
    /// Per-chunk opaque payload (chunk_index → blake3-hex string of the
    /// chunk's serialized state). M9+ wires the typed inner snapshot.
    pub chunks: BTreeMap<(i32, i32), String>,
    /// Per-actor opaque payload (actor_id → blake3-hex of serialized
    /// state). M9+ types the inner snapshot.
    pub actors: BTreeMap<u32, String>,
    /// Per-projectile opaque payload (projectile_id → blake3-hex).
    pub projectiles: BTreeMap<u32, String>,
    /// Mission director state snapshot (blake3-hex).
    pub mission: String,
    /// Engine + map + mod authoring metadata.
    pub authored_by: Vec<String>,
}

impl Snapshot {
    pub fn new(tick: u64, rng_state: [u8; 32], engine_build: impl Into<String>) -> Self {
        Self {
            tick,
            rng_state,
            engine_build: engine_build.into(),
            content_hashes: BTreeMap::new(),
            chunks: BTreeMap::new(),
            actors: BTreeMap::new(),
            projectiles: BTreeMap::new(),
            mission: String::new(),
            authored_by: Vec::new(),
        }
    }

    /// stream. Used by the cross-OS gate to detect divergence.
    pub fn determinism_checksum(&self) -> [u8; 32] {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.tick.to_le_bytes());
        hasher.update(&self.rng_state);
        for ((cx, cy), hex) in &self.chunks {
            hasher.update(&cx.to_le_bytes());
            hasher.update(&cy.to_le_bytes());
            hasher.update(hex.as_bytes());
        }
        for (id, hex) in &self.actors {
            hasher.update(&id.to_le_bytes());
            hasher.update(hex.as_bytes());
        }
        for (id, hex) in &self.projectiles {
            hasher.update(&id.to_le_bytes());
            hasher.update(hex.as_bytes());
        }
        hasher.update(self.mission.as_bytes());
        *hasher.finalize().as_bytes()
    }
}

/// as a delta against a base snapshot.
///
/// `Forward(old) → new` for applying server updates;
/// `Restore(new) → old` for rollback.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SnapshotDelta {
    Forward {
        from_tick: u64,
        to_tick: u64,
        /// Encoded as a list of `(actor_id, new_hex)` updates. Empty new
        /// hex means delete.
        actor_changes: Vec<(u32, Option<String>)>,
        chunk_changes: Vec<((i32, i32), Option<String>)>,
    },
    Restore {
        from_tick: u64,
        to_tick: u64,
        actor_changes: Vec<(u32, Option<String>)>,
        chunk_changes: Vec<((i32, i32), Option<String>)>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_round_trip() {
        let s = Snapshot::new(42, [7u8; 32], "0.0.1");
        let json = serde_json::to_string(&s).unwrap();
        let back: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn determinism_checksum_stable() {
        let s = Snapshot::new(42, [7u8; 32], "0.0.1");
        assert_eq!(s.determinism_checksum(), s.determinism_checksum());
    }

    #[test]
    fn determinism_checksum_differs_on_tick_change() {
        let mut s = Snapshot::new(42, [7u8; 32], "0.0.1");
        let h1 = s.determinism_checksum();
        s.tick = 43;
        let h2 = s.determinism_checksum();
        assert_ne!(h1, h2);
    }
}
