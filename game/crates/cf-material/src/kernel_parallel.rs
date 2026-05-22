//! Snapshot-then-apply parallel dispatch for M15 kernel reactions +
//! phase transitions. Used when MaterialKernel::with_parallel(true).
//! Determinism preserved via 4-color chunk pattern for reactions and
//! independent-chunk parallelism for phase transitions.

use cf_terrain::chunked::{ChunkedTerrain, CHUNK_SIZE};
use cf_terrain::heat::HeatField;

use crate::phase::{phase_transition_event, PhaseRegistry, PhaseTransitionEvent};
use crate::reactions::{
    reaction_event_with_emissions, ReactionLookup, ReactionRegistry, ReactionTriggeredEvent,
    EMISSION_DROPPED,
};
use crate::MaterialId;

pub(crate) fn dispatch_phase_parallel(
    terrain: &mut ChunkedTerrain,
    scan_chunks: &[(i32, i32)],
    phase: &PhaseRegistry,
    phase_materials: &std::collections::BTreeSet<MaterialId>,
    heat: &HeatField,
    prev_heat: &HeatField,
    tick: u64,
    cap: u32,
) -> (Vec<PhaseTransitionEvent>, u32) {
    use rayon::prelude::*;
    let width = terrain.width_px as i64;
    let height = terrain.height_px as i64;

    let snapshots: Vec<(i32, i32, Vec<MaterialId>)> = scan_chunks
        .iter()
        .filter_map(|(cx, cy)| terrain.chunk_pixels_clone(*cx, *cy).map(|p| (*cx, *cy, p)))
        .collect();

    type PhaseResult = (i32, i32, Vec<MaterialId>, Vec<PhaseTransitionEvent>, u32);
    let results: Vec<PhaseResult> = snapshots
        .into_par_iter()
        .map(|(cx, cy, mut pixels)| {
            let mut events: Vec<PhaseTransitionEvent> = Vec::new();
            let mut fired = 0u32;
            process_chunk_phase_to_snap(
                &mut pixels, cx, cy, width, height, phase, phase_materials, heat, prev_heat, tick,
                &mut events, &mut fired,
            );
            (cx, cy, pixels, events, fired)
        })
        .collect();

    let mut all_events: Vec<PhaseTransitionEvent> = Vec::new();
    let mut remaining = cap;
    for (cx, cy, pixels, mut events, _fired) in results {
        if remaining == 0 {
            break;
        }
        let dirty = terrain.replace_chunk_pixels_at_tick(cx, cy, pixels, tick);
        if dirty {
            let chunk_size = CHUNK_SIZE as i64;
            terrain.add_updated_material_area(
                [(cx as i64 * chunk_size) as f32, (cy as i64 * chunk_size) as f32],
                [((cx as i64 + 1) * chunk_size) as f32, ((cy as i64 + 1) * chunk_size) as f32],
            );
            terrain.wake_chunk_neighborhood(cx, cy);
        }
        let allowed = (remaining as usize).min(events.len());
        events.truncate(allowed);
        remaining = remaining.saturating_sub(allowed as u32);
        all_events.extend(events);
    }
    (all_events, remaining)
}

fn process_chunk_phase_to_snap(
    snap: &mut Vec<MaterialId>,
    cx: i32,
    cy: i32,
    width: i64,
    height: i64,
    phase: &PhaseRegistry,
    phase_materials: &std::collections::BTreeSet<MaterialId>,
    heat: &HeatField,
    prev_heat: &HeatField,
    tick: u64,
    events: &mut Vec<PhaseTransitionEvent>,
    fired: &mut u32,
) {
    let chunk_origin_x = (cx as i64) * (CHUNK_SIZE as i64);
    let chunk_origin_y = (cy as i64) * (CHUNK_SIZE as i64);
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
            let material = snap[(ly as usize) * (CHUNK_SIZE as usize) + (lx as usize)];
            if material == 0 || !phase_materials.contains(&material) {
                continue;
            }
            let curr_t = heat.temperature_at_world(world_x as f32, world_y as f32);
            let prev_t = prev_heat.temperature_at_world(world_x as f32, world_y as f32);
            if let Some((transition, direction)) = phase.evaluate(material, prev_t, curr_t) {
                let (product, _state) = transition.resolve(direction);
                if product != material {
                    snap[(ly as usize) * (CHUNK_SIZE as usize) + (lx as usize)] = product;
                }
                events.push(phase_transition_event(
                    transition, direction, [world_x as i32, world_y as i32], curr_t, tick,
                ));
                *fired += 1;
            }
        }
    }
}

/// 4-color reaction dispatch. Chunks colored by (cx%2, cy%2); within a color,
/// chunks are 2+ apart so cross-chunk emission targets belong to different
/// colors. Each color phase: snapshot → parallel process → serial writeback.
pub(crate) fn dispatch_reactions_4color_parallel(
    terrain: &mut ChunkedTerrain,
    scan_chunks: &[(i32, i32)],
    reactions: &ReactionRegistry,
    lookup: &ReactionLookup,
    heat: &HeatField,
    tick: u64,
    cap: u32,
) -> (Vec<ReactionTriggeredEvent>, u32) {
    use rayon::prelude::*;
    let width = terrain.width_px as i64;
    let height = terrain.height_px as i64;
    let mut all_events: Vec<ReactionTriggeredEvent> = Vec::new();
    let mut remaining = cap;

    for color in 0..4u8 {
        if remaining == 0 {
            break;
        }
        let color_chunks: Vec<(i32, i32)> = scan_chunks
            .iter()
            .filter(|(cx, cy)| chunk_color(*cx, *cy) == color)
            .copied()
            .collect();
        if color_chunks.is_empty() {
            continue;
        }

        let snapshots: Vec<(i32, i32, Vec<MaterialId>)> = color_chunks
            .iter()
            .filter_map(|(cx, cy)| terrain.chunk_pixels_clone(*cx, *cy).map(|p| (*cx, *cy, p)))
            .collect();

        let terrain_ref: &ChunkedTerrain = &*terrain;
        type ChunkResult = (
            i32,
            i32,
            Vec<MaterialId>,
            Vec<ReactionTriggeredEvent>,
            Vec<(i64, i64, MaterialId)>,
        );
        let results: Vec<ChunkResult> = snapshots
            .into_par_iter()
            .map(|(cx, cy, mut pixels)| {
                let mut events = Vec::new();
                let mut cross_writes: Vec<(i64, i64, MaterialId)> = Vec::new();
                let mut cross_occupied: std::collections::BTreeSet<(i64, i64)> =
                    std::collections::BTreeSet::new();
                process_chunk_reactions_to_snap(
                    &mut pixels, cx, cy, width, height, reactions, lookup, heat, terrain_ref, tick,
                    &mut events, &mut cross_writes, &mut cross_occupied,
                );
                (cx, cy, pixels, events, cross_writes)
            })
            .collect();

        for (cx, cy, pixels, mut events, cross_writes) in results {
            if remaining == 0 {
                break;
            }
            let chunk_dirty = terrain.replace_chunk_pixels_at_tick(cx, cy, pixels, tick);
            for (x, y, mat) in cross_writes {
                terrain.set_material_pixel(x, y, mat, tick);
            }
            if chunk_dirty {
                let chunk_size = CHUNK_SIZE as i64;
                terrain.add_updated_material_area(
                    [(cx as i64 * chunk_size) as f32, (cy as i64 * chunk_size) as f32],
                    [((cx as i64 + 1) * chunk_size) as f32, ((cy as i64 + 1) * chunk_size) as f32],
                );
                terrain.wake_chunk_neighborhood(cx, cy);
            }
            let allowed = (remaining as usize).min(events.len());
            events.truncate(allowed);
            remaining = remaining.saturating_sub(allowed as u32);
            all_events.extend(events);
        }
    }

    (all_events, remaining)
}

#[inline]
fn chunk_color(cx: i32, cy: i32) -> u8 {
    ((cx.rem_euclid(2) as u8) << 1) | (cy.rem_euclid(2) as u8)
}

#[inline]
fn snap_read_at(
    snap: &[MaterialId],
    terrain: &ChunkedTerrain,
    cx: i32,
    cy: i32,
    wx: i64,
    wy: i64,
) -> MaterialId {
    let lx = wx - (cx as i64) * CHUNK_SIZE as i64;
    let ly = wy - (cy as i64) * CHUNK_SIZE as i64;
    if lx >= 0 && lx < CHUNK_SIZE as i64 && ly >= 0 && ly < CHUNK_SIZE as i64 {
        snap[ly as usize * CHUNK_SIZE as usize + lx as usize]
    } else {
        terrain.material_at(wx, wy)
    }
}

#[inline]
fn snap_write_at(
    snap: &mut [MaterialId],
    cross_writes: &mut Vec<(i64, i64, MaterialId)>,
    cross_occupied: &mut std::collections::BTreeSet<(i64, i64)>,
    cx: i32,
    cy: i32,
    wx: i64,
    wy: i64,
    mat: MaterialId,
) {
    let lx = wx - (cx as i64) * CHUNK_SIZE as i64;
    let ly = wy - (cy as i64) * CHUNK_SIZE as i64;
    if lx >= 0 && lx < CHUNK_SIZE as i64 && ly >= 0 && ly < CHUNK_SIZE as i64 {
        snap[ly as usize * CHUNK_SIZE as usize + lx as usize] = mat;
    } else {
        cross_writes.push((wx, wy, mat));
        cross_occupied.insert((wx, wy));
    }
}

fn process_chunk_reactions_to_snap(
    snap: &mut Vec<MaterialId>,
    cx: i32,
    cy: i32,
    width: i64,
    height: i64,
    reactions: &ReactionRegistry,
    lookup: &ReactionLookup,
    heat: &HeatField,
    terrain: &ChunkedTerrain,
    tick: u64,
    events: &mut Vec<ReactionTriggeredEvent>,
    cross_writes: &mut Vec<(i64, i64, MaterialId)>,
    cross_occupied: &mut std::collections::BTreeSet<(i64, i64)>,
) {
    let chunk_origin_x = (cx as i64) * CHUNK_SIZE as i64;
    let chunk_origin_y = (cy as i64) * CHUNK_SIZE as i64;
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
            let pa = snap_read_at(snap, terrain, cx, cy, world_x, world_y);
            if !lookup.is_reactive(pa) {
                continue;
            }
            if world_x + 1 < width {
                let pb = snap_read_at(snap, terrain, cx, cy, world_x + 1, world_y);
                if try_fire_reaction_snap(
                    snap, cx, cy, width, height, terrain,
                    [world_x, world_y], [world_x + 1, world_y], pa, pb,
                    reactions, lookup, heat, tick, events, cross_writes, cross_occupied,
                ) {
                    continue;
                }
            }
            if world_y + 1 < height {
                let pb = snap_read_at(snap, terrain, cx, cy, world_x, world_y + 1);
                try_fire_reaction_snap(
                    snap, cx, cy, width, height, terrain,
                    [world_x, world_y], [world_x, world_y + 1], pa, pb,
                    reactions, lookup, heat, tick, events, cross_writes, cross_occupied,
                );
            }
        }
    }
}

fn try_fire_reaction_snap(
    snap: &mut Vec<MaterialId>,
    cx: i32,
    cy: i32,
    width: i64,
    height: i64,
    terrain: &ChunkedTerrain,
    pa_pos: [i64; 2],
    pb_pos: [i64; 2],
    pa: MaterialId,
    pb: MaterialId,
    reactions: &ReactionRegistry,
    lookup: &ReactionLookup,
    heat: &HeatField,
    tick: u64,
    events: &mut Vec<ReactionTriggeredEvent>,
    cross_writes: &mut Vec<(i64, i64, MaterialId)>,
    cross_occupied: &mut std::collections::BTreeSet<(i64, i64)>,
) -> bool {
    if pa == pb {
        return false;
    }
    let temp = heat.temperature_at_world(pa_pos[0] as f32, pa_pos[1] as f32);
    let Some(rxn) = lookup.evaluate(reactions, pa, pb, temp) else {
        return false;
    };
    let (a_pos, b_pos) = if pa == rxn.input_a {
        (pa_pos, pb_pos)
    } else {
        (pb_pos, pa_pos)
    };
    if !rxn.fires_at(tick, a_pos[0], a_pos[1], temp, cf_terrain::air::AMBIENT_PRESSURE_KPA) {
        return false;
    }
    snap_write_at(snap, cross_writes, cross_occupied, cx, cy, a_pos[0], a_pos[1], rxn.output);
    if let Some(by) = rxn.byproduct {
        snap_write_at(snap, cross_writes, cross_occupied, cx, cy, b_pos[0], b_pos[1], by);
    }
    let mut emission_positions: Vec<[i32; 2]> = Vec::with_capacity(rxn.emissions.len());
    let mut occupied: std::collections::BTreeSet<(i64, i64)> = std::collections::BTreeSet::new();
    occupied.insert((a_pos[0], a_pos[1]));
    occupied.insert((b_pos[0], b_pos[1]));
    for emission_mat in &rxn.emissions {
        let candidate = find_adjacent_air_cell_snap(
            snap, cx, cy, width, height, terrain, cross_occupied, a_pos, b_pos, &occupied,
        );
        match candidate {
            Some((ex, ey)) => {
                snap_write_at(snap, cross_writes, cross_occupied, cx, cy, ex, ey, *emission_mat);
                occupied.insert((ex, ey));
                emission_positions.push([ex as i32, ey as i32]);
            }
            None => {
                emission_positions.push([EMISSION_DROPPED, EMISSION_DROPPED]);
            }
        }
    }
    events.push(reaction_event_with_emissions(
        rxn,
        [a_pos[0] as i32, a_pos[1] as i32],
        tick,
        emission_positions,
    ));
    true
}

fn find_adjacent_air_cell_snap(
    snap: &[MaterialId],
    cx: i32,
    cy: i32,
    width: i64,
    height: i64,
    terrain: &ChunkedTerrain,
    cross_occupied: &std::collections::BTreeSet<(i64, i64)>,
    a_pos: [i64; 2],
    b_pos: [i64; 2],
    occupied: &std::collections::BTreeSet<(i64, i64)>,
) -> Option<(i64, i64)> {
    let candidates: [[i64; 2]; 8] = [
        [a_pos[0], a_pos[1] - 1],
        [a_pos[0] + 1, a_pos[1]],
        [a_pos[0], a_pos[1] + 1],
        [a_pos[0] - 1, a_pos[1]],
        [b_pos[0], b_pos[1] - 1],
        [b_pos[0] + 1, b_pos[1]],
        [b_pos[0], b_pos[1] + 1],
        [b_pos[0] - 1, b_pos[1]],
    ];
    for c in &candidates {
        let (wx, wy) = (c[0], c[1]);
        if wx < 0 || wy < 0 || wx >= width || wy >= height {
            continue;
        }
        if occupied.contains(&(wx, wy)) {
            continue;
        }
        if cross_occupied.contains(&(wx, wy)) {
            continue;
        }
        if snap_read_at(snap, terrain, cx, cy, wx, wy) == 0 {
            return Some((wx, wy));
        }
    }
    None
}
