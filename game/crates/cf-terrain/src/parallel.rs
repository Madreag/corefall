//! M8A § Files / cf-terrain — per-chunk parallel mutation + deterministic
//! merge.
//!
//! Per M8A spec § Acceptance criteria — "Per-chunk parallel terrain
//! mutation": the chunked terrain mutation runs `par_iter_mut` over the
//! dirty-chunk set. Inter-chunk boundary post-pass runs single-threaded
//! in `(cx, cy)` ascending order to preserve determinism across thread
//! schedules.
//!
//! M8A ships the scaffold (per-chunk dirty-rect application; single-
//! threaded boundary post-pass). M9+ wires this through the engine
//! scheduler.

use serde::{Deserialize, Serialize};

/// semantic event with `post_state_checksum`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkMutation {
    pub chunk_coords: (i32, i32),
    pub bbox: (u16, u16, u16, u16),
    pub delta_materials: Vec<MaterialChange>,
    pub post_state_checksum: [u8; 32],
    pub cause: TerrainCause,
    pub tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterialChange {
    pub px: u16,
    pub py: u16,
    pub old_material: u8,
    pub new_material: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerrainCause {
    Dig,
    Blast,
    Fill,
    Settle,
    Stamp,
    DoorToggle,
    HazardBurn,
}

/// order. The actual per-chunk mutation in the engine runs in
/// `par_iter_mut` (no shared writes); this function provides the
/// canonical sort order for the inter-chunk boundary post-pass.
pub fn sort_mutations_canonical(mutations: &mut [ChunkMutation]) {
    mutations.sort_by_key(|m| (m.chunk_coords.0, m.chunk_coords.1, m.tick));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_mutations_by_chunk_then_tick() {
        let mut mutations = vec![
            ChunkMutation {
                chunk_coords: (1, 0),
                bbox: (0, 0, 1, 1),
                delta_materials: vec![],
                post_state_checksum: [0; 32],
                cause: TerrainCause::Dig,
                tick: 10,
            },
            ChunkMutation {
                chunk_coords: (0, 0),
                bbox: (0, 0, 1, 1),
                delta_materials: vec![],
                post_state_checksum: [0; 32],
                cause: TerrainCause::Dig,
                tick: 15,
            },
            ChunkMutation {
                chunk_coords: (0, 0),
                bbox: (0, 0, 1, 1),
                delta_materials: vec![],
                post_state_checksum: [0; 32],
                cause: TerrainCause::Dig,
                tick: 5,
            },
        ];
        sort_mutations_canonical(&mut mutations);
        assert_eq!(mutations[0].chunk_coords, (0, 0));
        assert_eq!(mutations[0].tick, 5);
        assert_eq!(mutations[1].chunk_coords, (0, 0));
        assert_eq!(mutations[1].tick, 15);
        assert_eq!(mutations[2].chunk_coords, (1, 0));
    }
}
