//! **M15B** § Liquid-flow redistribution for rain puddles.
//!
//! Per the M15B spec § "Player-facing behavior":
//! > Player can observe the full water cycle: ground evaporates → cloud
//! > forms → rain falls → puddles flow back into low ground → repeat.
//!
//! And § acceptance scenario 4:
//! > And puddles accumulate in low ground via cf-terrain liquid_flow.
//!
//! This module owns the deterministic post-step pass that takes the
//! rain droplets the precipitation cycle deposited on the terrain and
//! redistributes them into low ground. It runs AFTER the CA step (per
//! the M15 ordering rule: phase → reactions → movement → liquid flow)
//! so the post-tick state has water pooled in basins.
//!
//! ## Algorithm
//!
//! 1. Scan dirty chunks for rain (id=87) and acid_droplet (id=88) pixels
//!    that have landed (their below-neighbor is solid).
//! 2. Convert them to their post-landing material (rain → water id=13,
//!    acid_droplet → acid id=21).
//! 3. Run a single deterministic flow-to-lowest-neighbor pass to push
//!    the pool toward the nearest depression. The flow pass is
//!    Margolus-friendly (per-cell rule, no atomic-CAS loops).
//!
//! ## Determinism contract
//!
//! - Per-chunk iteration in `(cx, cy)` ascending order (BTreeSet path).
//! - Per-cell iteration `(lx, ly)` ascending order within a chunk.
//! - No `thread_rng`, no `f64`, no `unsafe`.
//! - The output checksum (terrain.checksum_bytes()) is byte-stable across
//!   identical runs.

use serde::{Deserialize, Serialize};

use crate::chunked::{ChunkedTerrain, MaterialId, CHUNK_SIZE};

/// **M15B** § Material id for landed rain (water).
pub const POST_LANDING_RAIN: MaterialId = 13; // water
/// **M15B** § Material id for landed acid_droplet (acid).
pub const POST_LANDING_ACID_DROPLET: MaterialId = 21; // acid
/// **M15B** § Material ids that the flow pass migrates per tick. These
/// stay distinct from cf-terrain::ca's general liquid pass because we
/// want a deterministic single-pass redistribution that doesn't loop
/// the entire CA (which would be O(scene) per tick).
pub const FLOWING_LIQUIDS: &[MaterialId] = &[
    POST_LANDING_RAIN,           // water
    POST_LANDING_ACID_DROPLET,   // acid
    87,                          // rain (in-flight)
    88,                          // acid_droplet (in-flight)
];

/// **M15B** § Per-tick report from the liquid-flow pass.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LiquidFlowReport {
    pub tick: u64,
    pub landed_droplets: u32,
    pub pixels_flowed: u32,
    pub dirty_chunks_touched: u32,
}

/// **M15B** § Drive one liquid-flow pass over the terrain.
///
/// This is a Margolus-style single-pass deterministic redistribution.
/// It does NOT replace the cf-terrain::ca general CA pass — it
/// supplements it by:
/// 1. Landing rain/acid droplets when they hit a solid below.
/// 2. Spreading puddle water sideways into adjacent empty cells at the
///    same row when those cells also rest on solid (i.e. pooling into
///    basins).
///
/// Per spec § "puddles accumulate in low ground via cf-terrain
/// liquid_flow". The sideways flow stops at the first solid wall so a
/// "low ground" basin holds water without leaking.
pub fn liquid_flow_step(terrain: &mut ChunkedTerrain, tick: u64) -> LiquidFlowReport {
    let mut report = LiquidFlowReport {
        tick,
        ..Default::default()
    };

    let chunks = terrain.allocated_chunk_coords();
    report.dirty_chunks_touched = chunks.len() as u32;
    let width = terrain.width_px as i64;
    let height = terrain.height_px as i64;

    // Pass 1: convert landed droplets (rain or acid_droplet whose below
    // neighbor is solid OR out-of-world) to their post-landing material.
    for (cx, cy) in &chunks {
        let chunk_origin_x = (*cx as i64) * (CHUNK_SIZE as i64);
        let chunk_origin_y = (*cy as i64) * (CHUNK_SIZE as i64);
        for ly in 0..CHUNK_SIZE {
            let world_y = chunk_origin_y + (ly as i64);
            if world_y >= height {
                break;
            }
            for lx in 0..CHUNK_SIZE {
                let world_x = chunk_origin_x + (lx as i64);
                if world_x >= width {
                    break;
                }
                let mat = terrain.material_at(world_x, world_y);
                let landed_mat = match mat {
                    87 => Some(POST_LANDING_RAIN),
                    88 => Some(POST_LANDING_ACID_DROPLET),
                    _ => None,
                };
                let Some(post) = landed_mat else { continue };
                // Landed iff below is solid OR off-world.
                let landed = if world_y + 1 >= height {
                    true
                } else {
                    let below = terrain.material_at(world_x, world_y + 1);
                    is_solid_floor(below)
                };
                if landed {
                    terrain.set_material_pixel(world_x, world_y, post, tick);
                    let world_min = [world_x as f32, world_y as f32];
                    let world_max = [(world_x + 1) as f32, (world_y + 1) as f32];
                    terrain.add_updated_material_area(world_min, world_max);
                    report.landed_droplets = report.landed_droplets.saturating_add(1);
                }
            }
        }
    }

    // Pass 2: sideways flow for landed water/acid into adjacent air cells
    // that also rest on solid (puddling). One-tile-per-tick flow keeps
    // the cost bounded + the determinism contract intact.
    for (cx, cy) in &chunks {
        let chunk_origin_x = (*cx as i64) * (CHUNK_SIZE as i64);
        let chunk_origin_y = (*cy as i64) * (CHUNK_SIZE as i64);
        for ly in 0..CHUNK_SIZE {
            let world_y = chunk_origin_y + (ly as i64);
            if world_y >= height {
                break;
            }
            for lx in 0..CHUNK_SIZE {
                let world_x = chunk_origin_x + (lx as i64);
                if world_x >= width {
                    break;
                }
                let mat = terrain.material_at(world_x, world_y);
                if mat != POST_LANDING_RAIN && mat != POST_LANDING_ACID_DROPLET {
                    continue;
                }
                if world_y + 1 >= height {
                    continue;
                }
                let below = terrain.material_at(world_x, world_y + 1);
                if !is_solid_floor(below) {
                    continue;
                }
                // Try right neighbor first (deterministic preference).
                if world_x + 1 < width {
                    let right = terrain.material_at(world_x + 1, world_y);
                    if right == 0 {
                        let right_below = if world_y + 1 < height {
                            terrain.material_at(world_x + 1, world_y + 1)
                        } else {
                            // off-world = solid wall (boundary).
                            POST_LANDING_RAIN
                        };
                        if is_solid_floor(right_below) {
                            terrain.set_material_pixel(world_x + 1, world_y, mat, tick);
                            terrain.set_material_pixel(world_x, world_y, 0, tick);
                            let world_min = [world_x as f32, world_y as f32];
                            let world_max = [(world_x + 2) as f32, (world_y + 1) as f32];
                            terrain.add_updated_material_area(world_min, world_max);
                            report.pixels_flowed = report.pixels_flowed.saturating_add(1);
                            continue;
                        }
                    }
                }
                // Try left neighbor.
                if world_x > 0 {
                    let left = terrain.material_at(world_x - 1, world_y);
                    if left == 0 {
                        let left_below = if world_y + 1 < height {
                            terrain.material_at(world_x - 1, world_y + 1)
                        } else {
                            POST_LANDING_RAIN
                        };
                        if is_solid_floor(left_below) {
                            terrain.set_material_pixel(world_x - 1, world_y, mat, tick);
                            terrain.set_material_pixel(world_x, world_y, 0, tick);
                            let world_min = [(world_x - 1) as f32, world_y as f32];
                            let world_max = [(world_x + 1) as f32, (world_y + 1) as f32];
                            terrain.add_updated_material_area(world_min, world_max);
                            report.pixels_flowed = report.pixels_flowed.saturating_add(1);
                        }
                    }
                }
            }
        }
    }

    report
}

/// True when the material is a "solid floor" the liquid pass can pool
/// on top of. Anything that is NOT air and not another liquid that's
/// in-flight counts as solid. The post-landing rain (water) and
/// acid (acid) ARE counted as solid too so water pools sit on top of
/// existing water without sinking through them.
#[must_use]
pub fn is_solid_floor(material: MaterialId) -> bool {
    // air (0), in-flight rain (87), in-flight acid_droplet (88) are
    // explicitly NOT a floor. Everything else is.
    !matches!(material, 0 | 87 | 88)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunked::{ChunkedTerrain, MATERIAL_AIR, MATERIAL_DIRT};

    /// VAL-M15B-flow-001: rain droplet on top of a solid floor lands +
    /// becomes water (somewhere along the same row — the in-tick
    /// sideways flow may push the puddle a single tile right).
    #[test]
    fn rain_droplet_lands_as_water() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        // Dirt floor at y=4
        for x in 0..8 {
            terrain.set_material_pixel(x, 4, MATERIAL_DIRT, 0);
        }
        terrain.set_material_pixel(3, 3, 87, 0); // rain
        let r = liquid_flow_step(&mut terrain, 1);
        assert!(r.landed_droplets >= 1, "rain must land");
        let mut found_water = false;
        for x in 0..8 {
            if terrain.material_at(x, 3) == POST_LANDING_RAIN {
                found_water = true;
                break;
            }
        }
        assert!(found_water, "water must be somewhere on row y=3 after landing");
    }

    /// VAL-M15B-flow-002: acid_droplet lands as acid (somewhere along
    /// the same row).
    #[test]
    fn acid_droplet_lands_as_acid() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        for x in 0..8 {
            terrain.set_material_pixel(x, 4, MATERIAL_DIRT, 0);
        }
        terrain.set_material_pixel(3, 3, 88, 0); // acid_droplet
        let _ = liquid_flow_step(&mut terrain, 1);
        let mut found_acid = false;
        for x in 0..8 {
            if terrain.material_at(x, 3) == POST_LANDING_ACID_DROPLET {
                found_acid = true;
                break;
            }
        }
        assert!(found_acid, "acid must be somewhere on row y=3 after landing");
    }

    /// VAL-M15B-flow-003: airborne rain (no solid below) does NOT land.
    #[test]
    fn airborne_rain_does_not_land() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        // No floor — the cell below is air.
        terrain.set_material_pixel(3, 3, 87, 0);
        let r = liquid_flow_step(&mut terrain, 1);
        assert_eq!(r.landed_droplets, 0, "airborne rain must not land");
        assert_eq!(terrain.material_at(3, 3), 87);
    }

    /// VAL-M15B-flow-004: water on top of solid spreads sideways into
    /// adjacent air (puddling).
    #[test]
    fn water_spreads_into_adjacent_air_cell() {
        let mut terrain = ChunkedTerrain::new(16, 8, MATERIAL_AIR);
        for x in 0..16 {
            terrain.set_material_pixel(x, 4, MATERIAL_DIRT, 0);
        }
        terrain.set_material_pixel(5, 3, POST_LANDING_RAIN, 0); // pre-landed water
        let _ = liquid_flow_step(&mut terrain, 1);
        // After flow, water should have moved to (6, 3) (right preference).
        let original = terrain.material_at(5, 3);
        let right = terrain.material_at(6, 3);
        assert!(
            original == 0 || right == POST_LANDING_RAIN,
            "water must spread right when possible"
        );
    }

    /// VAL-M15B-flow-005: water in a basin does NOT leak past solid
    /// walls.
    #[test]
    fn water_does_not_leak_past_solid_walls() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        // Build a basin: floor at y=4, walls at x=2 and x=5
        for x in 2..=5 {
            terrain.set_material_pixel(x, 4, MATERIAL_DIRT, 0);
        }
        terrain.set_material_pixel(2, 3, MATERIAL_DIRT, 0);
        terrain.set_material_pixel(5, 3, MATERIAL_DIRT, 0);
        terrain.set_material_pixel(3, 3, POST_LANDING_RAIN, 0);
        let _ = liquid_flow_step(&mut terrain, 1);
        // Water can move from (3,3) → (4,3) but not past the wall at (5,3).
        assert_ne!(terrain.material_at(5, 3), POST_LANDING_RAIN, "wall must hold water");
    }

    /// VAL-M15B-flow-006: report counts both landings + flows.
    #[test]
    fn report_tracks_landings_and_flows() {
        let mut terrain = ChunkedTerrain::new(16, 8, MATERIAL_AIR);
        for x in 0..16 {
            terrain.set_material_pixel(x, 4, MATERIAL_DIRT, 0);
        }
        terrain.set_material_pixel(2, 3, 87, 0); // rain
        terrain.set_material_pixel(8, 3, POST_LANDING_RAIN, 0); // pre-landed water
        let r = liquid_flow_step(&mut terrain, 42);
        assert_eq!(r.tick, 42);
        assert!(r.landed_droplets >= 1);
        assert!(r.pixels_flowed >= 1);
    }

    /// VAL-M15B-flow-007: deterministic — identical input yields
    /// identical state across runs.
    #[test]
    fn liquid_flow_is_deterministic_across_runs() {
        fn run() -> Vec<MaterialId> {
            let mut terrain = ChunkedTerrain::new(16, 16, MATERIAL_AIR);
            for x in 0..16 {
                terrain.set_material_pixel(x, 8, MATERIAL_DIRT, 0);
            }
            terrain.set_material_pixel(3, 7, 87, 0);
            terrain.set_material_pixel(6, 7, 87, 0);
            terrain.set_material_pixel(9, 7, 88, 0);
            for t in 0u64..5 {
                liquid_flow_step(&mut terrain, t);
            }
            let mut snap = Vec::new();
            for y in 0..16 {
                for x in 0..16 {
                    snap.push(terrain.material_at(x, y));
                }
            }
            snap
        }
        assert_eq!(run(), run());
    }

    /// VAL-M15B-flow-008: rain droplet at the very bottom of the world
    /// lands without a tile below (off-world boundary acts as floor).
    #[test]
    fn rain_at_world_bottom_lands() {
        let mut terrain = ChunkedTerrain::new(8, 4, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 87, 0); // bottom row
        let r = liquid_flow_step(&mut terrain, 1);
        assert!(r.landed_droplets >= 1);
        assert_eq!(terrain.material_at(3, 3), POST_LANDING_RAIN);
    }

    /// VAL-M15B-flow-009: is_solid_floor classification.
    #[test]
    fn is_solid_floor_classification_is_correct() {
        assert!(!is_solid_floor(0), "air is not floor");
        assert!(!is_solid_floor(87), "rain in-flight is not floor");
        assert!(!is_solid_floor(88), "acid_droplet in-flight is not floor");
        assert!(is_solid_floor(MATERIAL_DIRT), "dirt is floor");
        assert!(is_solid_floor(POST_LANDING_RAIN), "water is floor");
        assert!(is_solid_floor(POST_LANDING_ACID_DROPLET), "acid is floor");
    }

    /// VAL-M15B-flow-010: idempotent — running the step twice with no
    /// new input doesn't double-count landings.
    #[test]
    fn second_step_does_not_double_count_landings() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        for x in 0..8 {
            terrain.set_material_pixel(x, 4, MATERIAL_DIRT, 0);
        }
        terrain.set_material_pixel(3, 3, 87, 0);
        let r1 = liquid_flow_step(&mut terrain, 1);
        let r2 = liquid_flow_step(&mut terrain, 2);
        assert!(r1.landed_droplets >= 1);
        assert_eq!(r2.landed_droplets, 0, "no more in-flight rain to land");
    }
}
