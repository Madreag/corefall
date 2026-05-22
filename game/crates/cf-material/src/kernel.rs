//! **M15** § The active-material kernel orchestration loop.
//!
//! Per the M15 spec § "Canonical ownership":
//! > This spec is the **canonical owner of the active-material kernel
//! > orchestration loop** — the per-tick CPU cellular-automata stepper,
//! > the Margolus checker-pattern ordering, the `AddUpdatedMaterialArea`
//! > dirty-path contract, the **active-chunk wake/sleep gating**, the
//! > **phase-transition + alchemy + flask glue**, and the **per-tick
//! > reaction-evaluator dispatch**.
//!
//! [`KernelStep`] is the top-level entry point invoked once per sim
//! tick. It:
//!
//! 1. Runs the Margolus CA movement pass (`cf_terrain::ca::step_ca` or
//!    `step_ca_filtered` when wake/sleep gating is enabled).
//! 2. Dispatches per-pixel **reactions** between adjacent pixels against
//!    the [`crate::reactions::ReactionRegistry`] (M15D registry).
//! 3. Dispatches per-pixel **phase transitions** against the
//!    [`crate::phase::PhaseRegistry`].
//! 4. Wakes the 3×3 chunk neighborhood for every chunk that saw a write
//!    (movement, reaction, or phase change) per Preservation rule 4.
//! 5. Transitions chunks idle past
//!    [`SLEEP_IDLE_THRESHOLD_TICKS`] back to
//!    `active_region = false`.
//!
//! The kernel emits structured [`KernelStepReport`] data for the engine
//! to turn into `material.cellular_step`,
//! `material.reaction_triggered`, and `material.phase_transition`
//! replay events.

use serde::{Deserialize, Serialize};

use cf_terrain::ca::{step_ca_filtered, CaMovementClass, CaStepReport, CaStepperState};
use cf_terrain::chunked::{ChunkedTerrain, CHUNK_SIZE};
use cf_terrain::heat::HeatField;

use crate::phase::{phase_transition_event, PhaseRegistry, PhaseTransitionEvent};
use crate::reactions::{ReactionLookup, ReactionRegistry, ReactionTriggeredEvent};
use crate::MaterialId;

/// Per M8A `CHUNK_SLEEP_IDLE_THRESHOLD_TICKS`; the kernel transitions
/// chunks idle this many ticks back to sleeping.
pub const SLEEP_IDLE_THRESHOLD_TICKS: u64 = 300;

/// Reactions-per-step cap. Per-tick reaction firings are capped to keep
/// the bench within budget; events past the cap are dropped (the engine
/// layer would surface a `material.reactions_capped` event in a future
/// extension). Mods can override via [`KernelStep::reaction_cap`].
pub const DEFAULT_REACTION_CAP: u32 = 4096;

/// Phase-transitions-per-step cap.
pub const DEFAULT_PHASE_CAP: u32 = 4096;

/// One per-tick driver of the active material kernel. Holds the CA
/// stepper state across ticks and accumulates the wake/sleep ledger.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialKernel {
    pub stepper: CaStepperState,
    pub awake_only: bool,
    pub reaction_cap: u32,
    pub phase_cap: u32,
    /// **Parallel dispatch** opt-in. When `true`, the per-chunk reaction
    /// + phase passes use rayon's `par_iter` over the 4-color chunk
    /// pattern (chunks colored by `(cx % 2, cy % 2)`). Chunks of the
    /// same color don't share pixel writes — they're at least 2 chunks
    /// apart — so within a color phase the work is data-race-free.
    /// Between colors, writes are flushed serially in deterministic
    /// `(cx, cy)` order so the per-tick blake3 sim_checksum is byte-
    /// identical to the serial path. Default `false` preserves the
    /// existing M15 acceptance behavior; set via [`Self::with_parallel`].
    #[serde(default)]
    pub parallel: bool,
}

impl Default for MaterialKernel {
    fn default() -> Self {
        Self {
            stepper: CaStepperState::default(),
            // Default is full-scan; wake/sleep gating is opt-in so scenarios
            // that haven't pre-seeded `active_region=true` chunks still simulate.
            awake_only: false,
            reaction_cap: DEFAULT_REACTION_CAP,
            phase_cap: DEFAULT_PHASE_CAP,
            parallel: false,
        }
    }
}

impl MaterialKernel {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle wake/sleep gating. When `true`, the CA stepper visits
    /// only chunks with `active_region == true` (huge perf win for
    /// scenes with sleeping geometry). Defaults to `false` so scenarios
    /// run without pre-seeding active chunks.
    #[must_use]
    pub fn with_wake_sleep_gating(mut self, on: bool) -> Self {
        self.awake_only = on;
        self
    }

    /// **Opt-in parallel per-chunk dispatch**. Per-chunk reaction +
    /// phase passes run on a rayon thread pool using the 4-color chunk
    /// pattern. Determinism is preserved because:
    /// 1. Chunks within a color phase don't share writes (they're
    ///    2+ chunks apart in both axes).
    /// 2. Writes are flushed in deterministic `(cx, cy)` order between
    ///    phases.
    /// 3. Within-chunk pixel cascade still runs sequentially per chunk
    ///    (each parallel worker maintains a local pixel-override map
    ///    that subsequent cascade reads consult before the immutable
    ///    terrain reference).
    ///
    /// Expected speedup: ~Nx on N-core machines for >256-chunk scenes.
    /// For the M15 CA burst bench (100K active pixels, 256 chunks),
    /// drops p99 from ~56 ms (single-threaded) toward the M15 4 ms
    /// HARD GATE budget.
    #[must_use]
    pub fn with_parallel(mut self, on: bool) -> Self {
        self.parallel = on;
        self.stepper.parallel = on;
        self
    }

    /// Toggle parallel dispatch on an existing kernel.
    pub fn set_parallel(&mut self, on: bool) {
        self.parallel = on;
        self.stepper.parallel = on;
    }

    /// Current sim tick.
    pub fn tick(&self) -> u64 {
        self.stepper.tick
    }
}

/// One tick's outcome from the orchestrator. Holds the CA report plus
/// the dispatched reaction + phase-transition events. The engine layer
/// records these via cf-replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct KernelStepReport {
    pub ca: CaStepReport,
    pub reactions: Vec<ReactionTriggeredEvent>,
    pub phase_transitions: Vec<PhaseTransitionEvent>,
    pub awake_chunks_after: u32,
    pub slept_chunks: Vec<(i32, i32)>,
}

/// material kernel concern in one deterministic call.
///
/// ## Order of operations (locked by the M15 acceptance scenarios)
///
/// 1. **Phase transitions** — per-pixel temperature-crossing → new
///    material id. Runs first so phase products participate in the
///    same-tick reaction + movement passes.
/// 2. **Reaction dispatch** — per-pixel adjacency. Runs second so the
///    reaction pass sees the post-phase material state (a water tile
///    that just boiled to steam can react as steam this tick).
/// 3. **CA movement** — Margolus gravity / buoyancy / sideways flow.
///    Runs last so reaction byproducts (e.g. steam from `water+fire`)
///    can rise this tick and the iron pixel that just rusted stays in
///    place during the reaction's recording.
/// 4. **Sleep-idle bookkeeping** — chunks idle past
///    [`SLEEP_IDLE_THRESHOLD_TICKS`] go back to
///    `active_region = false`.
///
/// `prev_heat` is the heat field snapshot from the previous tick (used
/// to detect threshold crossings). Callers wishing not to track phase
/// transitions can pass `None`.
pub fn kernel_step(
    terrain: &mut ChunkedTerrain,
    kernel: &mut MaterialKernel,
    reactions: &ReactionRegistry,
    phase: &PhaseRegistry,
    heat: &HeatField,
    prev_heat: Option<&HeatField>,
) -> KernelStepReport {
    let tick_before = kernel.tick();

    // The set of chunks to scan for reactions + phase transitions is
    // computed once and shared across both passes. Includes every
    // allocated chunk (or every awake chunk under wake/sleep gating).
    let scan_chunks = if kernel.awake_only {
        terrain.awake_chunk_coords()
    } else {
        terrain.allocated_chunk_coords()
    };

    let phase_materials = phase.phase_material_set();
    let (phase_events, _phase_remaining) = if let Some(prev) = prev_heat {
        if kernel.parallel {
            dispatch_phase_parallel(
                terrain,
                &scan_chunks,
                phase,
                &phase_materials,
                heat,
                prev,
                tick_before,
                kernel.phase_cap,
            )
        } else {
            let mut events: Vec<PhaseTransitionEvent> = Vec::new();
            let mut remaining = kernel.phase_cap;
            for (cx, cy) in &scan_chunks {
                if remaining == 0 {
                    break;
                }
                let n = dispatch_phase_in_chunk(
                    terrain,
                    *cx,
                    *cy,
                    phase,
                    &phase_materials,
                    heat,
                    prev,
                    tick_before,
                    &mut events,
                    &mut remaining,
                );
                if n > 0 {
                    terrain.wake_chunk_neighborhood(*cx, *cy);
                }
            }
            (events, remaining)
        }
    } else {
        (Vec::new(), kernel.phase_cap)
    };

    let reaction_lookup = reactions.build_lookup();
    let (reaction_events, reactions_remaining) =
        if kernel.parallel {
            dispatch_reactions_4color_parallel(
                terrain,
                &scan_chunks,
                reactions,
                &reaction_lookup,
                heat,
                tick_before,
                kernel.reaction_cap,
            )
        } else {
            let mut events: Vec<ReactionTriggeredEvent> = Vec::new();
            let mut remaining = kernel.reaction_cap;
            for (cx, cy) in &scan_chunks {
                if remaining == 0 {
                    break;
                }
                let n = dispatch_reactions_in_chunk(
                    terrain,
                    *cx,
                    *cy,
                    reactions,
                    &reaction_lookup,
                    heat,
                    tick_before,
                    &mut events,
                    &mut remaining,
                );
                if n > 0 {
                    terrain.wake_chunk_neighborhood(*cx, *cy);
                }
            }
            (events, remaining)
        };
    let _ = reactions_remaining;

    // --- 3. CA movement (gravity / buoyancy / sideways).
    let ca_report = step_ca_filtered(terrain, &mut kernel.stepper, kernel.awake_only);

    // --- 4. Sleep idle chunks (wake/sleep bookkeeping).
    let slept = terrain.sleep_idle_chunks(kernel.tick(), SLEEP_IDLE_THRESHOLD_TICKS);
    let awake_after = terrain.awake_chunk_coords().len() as u32;

    KernelStepReport {
        ca: ca_report,
        reactions: reaction_events,
        phase_transitions: phase_events,
        awake_chunks_after: awake_after,
        slept_chunks: slept,
    }
}

/// **Test/debug helper**: same as [`kernel_step`] but skips the CA
/// movement pass. Used by acceptance tests that need to verify
/// reaction + phase outputs without the materials drifting under
/// gravity in the same tick.
pub fn kernel_step_no_movement(
    terrain: &mut ChunkedTerrain,
    kernel: &mut MaterialKernel,
    reactions: &ReactionRegistry,
    phase: &PhaseRegistry,
    heat: &HeatField,
    prev_heat: Option<&HeatField>,
) -> KernelStepReport {
    let tick_before = kernel.tick();
    let scan_chunks = if kernel.awake_only {
        terrain.awake_chunk_coords()
    } else {
        terrain.allocated_chunk_coords()
    };

    let mut phase_events: Vec<PhaseTransitionEvent> = Vec::new();
    let mut phase_remaining = kernel.phase_cap;
    let phase_materials = phase.phase_material_set();
    if let Some(prev) = prev_heat {
        for (cx, cy) in &scan_chunks {
            if phase_remaining == 0 {
                break;
            }
            let n = dispatch_phase_in_chunk(
                terrain,
                *cx,
                *cy,
                phase,
                &phase_materials,
                heat,
                prev,
                tick_before,
                &mut phase_events,
                &mut phase_remaining,
            );
            if n > 0 {
                terrain.wake_chunk_neighborhood(*cx, *cy);
            }
        }
    }
    let mut reaction_events: Vec<ReactionTriggeredEvent> = Vec::new();
    let mut reactions_remaining = kernel.reaction_cap;
    let reaction_lookup = reactions.build_lookup();
    for (cx, cy) in &scan_chunks {
        if reactions_remaining == 0 {
            break;
        }
        let n = dispatch_reactions_in_chunk(
            terrain,
            *cx,
            *cy,
            reactions,
            &reaction_lookup,
            heat,
            tick_before,
            &mut reaction_events,
            &mut reactions_remaining,
        );
        if n > 0 {
            terrain.wake_chunk_neighborhood(*cx, *cy);
        }
    }
    kernel.stepper.advance();
    let ca = CaStepReport {
        tick: tick_before,
        parity: kernel.stepper.parity ^ 1,
        pixels_moved: 0,
        dirty_chunks: vec![],
    };
    let slept = terrain.sleep_idle_chunks(kernel.tick(), SLEEP_IDLE_THRESHOLD_TICKS);
    let awake_after = terrain.awake_chunk_coords().len() as u32;
    KernelStepReport {
        ca,
        reactions: reaction_events,
        phase_transitions: phase_events,
        awake_chunks_after: awake_after,
        slept_chunks: slept,
    }
}

/// Scan a single chunk's pixels and fire reactions whose `(a, b)` pair
/// matches an adjacent (4-neighbor: right + below) pair. Per the M15
/// spec § "per-tick reaction-evaluator dispatch".
///
/// Fast-path optimization: pixels whose material is NOT in the
/// registry's `primary_reactive_set` are skipped (air, dirt, concrete,
/// etc. never appear in any reaction's `(input_a, input_b)` pair, so
/// no neighbor check is needed). This drops the bench cost from
/// O(pixels × neighbors) to O(reactive_pixels × neighbors) — a 20×+
/// speedup for typical scenarios where >95% of pixels are inert.
///
/// Returns the number of reactions fired this chunk.
fn dispatch_reactions_in_chunk(
    terrain: &mut ChunkedTerrain,
    cx: i32,
    cy: i32,
    reactions: &ReactionRegistry,
    reaction_lookup: &ReactionLookup,
    heat: &HeatField,
    tick: u64,
    out_events: &mut Vec<ReactionTriggeredEvent>,
    remaining: &mut u32,
) -> u32 {
    let chunk_origin_x = (cx as i64) * (CHUNK_SIZE as i64);
    let chunk_origin_y = (cy as i64) * (CHUNK_SIZE as i64);
    let width = terrain.width_px as i64;
    let height = terrain.height_px as i64;

    let mut fired: u32 = 0;
    // Iterate every pixel in the chunk; check right + below neighbors
    // (no need to check left/up since those pairs are covered by the
    // chunk to the left/above's iteration).
    //
    // **Perf** § O(1) reactive-material bitmap check + O(1) lookup
    // table for the (a, b) pair → reaction match. Replaces BTreeSet
    // log-N lookup + linear scan over the reactions Vec.
    for ly in 0..CHUNK_SIZE {
        if *remaining == 0 {
            break;
        }
        let world_y = chunk_origin_y + (ly as i64);
        if world_y >= height {
            break;
        }
        for lx in 0..CHUNK_SIZE {
            if *remaining == 0 {
                break;
            }
            let world_x = chunk_origin_x + (lx as i64);
            if world_x >= width {
                break;
            }
            let pa = terrain.material_at(world_x, world_y);
            if !reaction_lookup.is_reactive(pa) {
                continue;
            }
            // Right neighbor.
            if world_x + 1 < width {
                let pb = terrain.material_at(world_x + 1, world_y);
                if try_fire_reaction(
                    terrain,
                    [world_x, world_y],
                    [world_x + 1, world_y],
                    pa,
                    pb,
                    reactions,
                    reaction_lookup,
                    heat,
                    tick,
                    out_events,
                ) {
                    fired += 1;
                    *remaining = remaining.saturating_sub(1);
                    continue;
                }
            }
            // Below neighbor.
            if world_y + 1 < height {
                let pb = terrain.material_at(world_x, world_y + 1);
                if try_fire_reaction(
                    terrain,
                    [world_x, world_y],
                    [world_x, world_y + 1],
                    pa,
                    pb,
                    reactions,
                    reaction_lookup,
                    heat,
                    tick,
                    out_events,
                ) {
                    fired += 1;
                    *remaining = remaining.saturating_sub(1);
                }
            }
        }
    }
    fired
}

/// Attempt to fire one reaction between pixels `pa_pos` (material `pa`)
/// and `pb_pos` (material `pb`). Returns true if a reaction fired.
///
/// into an adjacent air cell. The search walks NESW deterministically
/// starting at `input_a`'s neighbors, then `input_b`'s neighbors. Each
/// emission consumes one air cell; subsequent emissions don't overlap.
/// Emissions that find no air cell are dropped (the reaction still
/// fires, but the emission's position is the [`EMISSION_DROPPED`]
/// sentinel in the event payload).
fn try_fire_reaction(
    terrain: &mut ChunkedTerrain,
    pa_pos: [i64; 2],
    pb_pos: [i64; 2],
    pa: MaterialId,
    pb: MaterialId,
    reactions: &ReactionRegistry,
    reaction_lookup: &ReactionLookup,
    heat: &HeatField,
    tick: u64,
    out_events: &mut Vec<ReactionTriggeredEvent>,
) -> bool {
    if pa == pb {
        return false;
    }
    let temp = heat.temperature_at_world(pa_pos[0] as f32, pa_pos[1] as f32);
    // **Perf** § O(1) table lookup instead of linear scan over registry.
    let Some(rxn) = reaction_lookup.evaluate(reactions, pa, pb, temp) else {
        return false;
    };
    // Identify which pixel is `input_a` (gets `output`) and which is
    // `input_b` (gets `byproduct` if Some, else stays).
    let (a_pos, b_pos) = if pa == rxn.input_a {
        (pa_pos, pb_pos)
    } else {
        (pb_pos, pa_pos)
    };
    terrain.set_material_pixel(a_pos[0], a_pos[1], rxn.output, tick);
    if let Some(byproduct) = rxn.byproduct {
        terrain.set_material_pixel(b_pos[0], b_pos[1], byproduct, tick);
    }
    let mut emission_positions: Vec<[i32; 2]> = Vec::with_capacity(rxn.emissions.len());
    let mut occupied: std::collections::BTreeSet<(i64, i64)> = std::collections::BTreeSet::new();
    // Don't spawn an emission ON the pixels we just rewrote.
    occupied.insert((a_pos[0], a_pos[1]));
    occupied.insert((b_pos[0], b_pos[1]));
    for emission_mat in &rxn.emissions {
        let candidate = find_adjacent_air_cell(terrain, a_pos, b_pos, &occupied);
        match candidate {
            Some((ex, ey)) => {
                terrain.set_material_pixel(ex, ey, *emission_mat, tick);
                occupied.insert((ex, ey));
                emission_positions.push([ex as i32, ey as i32]);
            }
            None => {
                emission_positions.push([cf_material_internal::EMISSION_DROPPED_REEXPORT, cf_material_internal::EMISSION_DROPPED_REEXPORT]);
            }
        }
    }
    // M3 preservation rule 1: AddUpdatedMaterialArea is the canonical
    // dirty path. The per-pixel set_material_pixel calls already mark
    // dirty_chunks, but we also call add_updated_material_area to be
    // explicit + cover any future viewer/AI consumer that watches the
    // function call surface.
    let mut min_x = a_pos[0].min(b_pos[0]);
    let mut min_y = a_pos[1].min(b_pos[1]);
    let mut max_x = a_pos[0].max(b_pos[0]);
    let mut max_y = a_pos[1].max(b_pos[1]);
    for emission_pos in &emission_positions {
        if emission_pos[0] == cf_material_internal::EMISSION_DROPPED_REEXPORT
            && emission_pos[1] == cf_material_internal::EMISSION_DROPPED_REEXPORT
        {
            continue;
        }
        min_x = min_x.min(emission_pos[0] as i64);
        min_y = min_y.min(emission_pos[1] as i64);
        max_x = max_x.max(emission_pos[0] as i64);
        max_y = max_y.max(emission_pos[1] as i64);
    }
    let world_min = [min_x as f32, min_y as f32];
    let world_max = [(max_x + 1) as f32, (max_y + 1) as f32];
    terrain.add_updated_material_area(world_min, world_max);
    out_events.push(crate::reactions::reaction_event_with_emissions(
        rxn,
        [a_pos[0] as i32, a_pos[1] as i32],
        tick,
        emission_positions,
    ));
    true
}

/// the orchestrator can spell it without leaking the crate-public name
/// into every match arm. Kept private to this module.
mod cf_material_internal {
    pub const EMISSION_DROPPED_REEXPORT: i32 = crate::reactions::EMISSION_DROPPED;
}

/// search walks NESW (north → east → south → west) starting at
/// `a_pos`'s neighbors, then `b_pos`'s neighbors. `occupied` lists
/// cells already taken by THIS reaction's output/byproduct/prior
/// emission so we don't double-write the same cell.
///
/// Determinism: the order is fixed; same inputs → same output cell
/// across runs.
fn find_adjacent_air_cell(
    terrain: &ChunkedTerrain,
    a_pos: [i64; 2],
    b_pos: [i64; 2],
    occupied: &std::collections::BTreeSet<(i64, i64)>,
) -> Option<(i64, i64)> {
    let width = terrain.width_px as i64;
    let height = terrain.height_px as i64;
    // NESW around a_pos first, then b_pos. (-1y up first per Margolus
    // convention where +y is down.)
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
        let (cx, cy) = (c[0], c[1]);
        if cx < 0 || cy < 0 || cx >= width || cy >= height {
            continue;
        }
        if occupied.contains(&(cx, cy)) {
            continue;
        }
        if terrain.material_at(cx, cy) == 0 {
            return Some((cx, cy));
        }
    }
    None
}

pub(crate) use crate::kernel_parallel::{dispatch_phase_parallel, dispatch_reactions_4color_parallel};


/// Scan a single chunk's pixels and fire phase transitions whose
/// temperature crossing fires for the pixel's material. Per the M15
/// spec § "Material state transitions (water → steam → cloud → rain)".
///
/// Fast-path optimization: pixels whose material is not in the
/// registry's `phase_material_set` (inert solids: dirt, concrete,
/// metal) are skipped. Bench cost drops O(pixels) → O(phase_pixels).
fn dispatch_phase_in_chunk(
    terrain: &mut ChunkedTerrain,
    cx: i32,
    cy: i32,
    phase: &PhaseRegistry,
    phase_materials: &std::collections::BTreeSet<MaterialId>,
    heat: &HeatField,
    prev_heat: &HeatField,
    tick: u64,
    out_events: &mut Vec<PhaseTransitionEvent>,
    remaining: &mut u32,
) -> u32 {
    let chunk_origin_x = (cx as i64) * (CHUNK_SIZE as i64);
    let chunk_origin_y = (cy as i64) * (CHUNK_SIZE as i64);
    let width = terrain.width_px as i64;
    let height = terrain.height_px as i64;

    let mut fired: u32 = 0;
    for ly in 0..CHUNK_SIZE {
        if *remaining == 0 {
            break;
        }
        let world_y = chunk_origin_y + (ly as i64);
        if world_y >= height {
            break;
        }
        for lx in 0..CHUNK_SIZE {
            if *remaining == 0 {
                break;
            }
            let world_x = chunk_origin_x + (lx as i64);
            if world_x >= width {
                break;
            }
            let material = terrain.material_at(world_x, world_y);
            // Skip air (no phase transitions for vacuum).
            if material == 0 {
                continue;
            }
            if !phase_materials.contains(&material) {
                continue;
            }
            let curr_t = heat.temperature_at_world(world_x as f32, world_y as f32);
            let prev_t = prev_heat.temperature_at_world(world_x as f32, world_y as f32);
            if let Some((transition, direction)) = phase.evaluate(material, prev_t, curr_t) {
                let (product, _state) = transition.resolve(direction);
                if product != material {
                    terrain.set_material_pixel(world_x, world_y, product, tick);
                    let world_min = [world_x as f32, world_y as f32];
                    let world_max = [(world_x + 1) as f32, (world_y + 1) as f32];
                    terrain.add_updated_material_area(world_min, world_max);
                }
                out_events.push(phase_transition_event(
                    transition,
                    direction,
                    [world_x as i32, world_y as i32],
                    curr_t,
                    tick,
                ));
                fired += 1;
                *remaining = remaining.saturating_sub(1);
            }
        }
    }
    fired
}

/// Smoke-check helper: is the pixel a "movable" CA class? Used by tests.
#[must_use]
pub fn pixel_is_dynamic(class: CaMovementClass) -> bool {
    !matches!(class, CaMovementClass::Static | CaMovementClass::Air)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_terrain::chunked::{ChunkedTerrain, MATERIAL_AIR};
    use cf_terrain::heat::HeatField;

    use crate::phase::default_phase_registry;
    use crate::reactions::default_reaction_registry;

    fn empty_heat() -> HeatField {
        HeatField::default()
    }

    /// VAL-M15-kernel-001: acid + iron reaction fires through the
    /// orchestrator. The iron pixel transforms to rust (spec gherkin
    /// "And the iron pixel transforms"). Uses
    /// [`kernel_step_no_movement`] so the test verifies pixel positions
    /// stable across the orchestrator's reaction pass.
    #[test]
    fn kernel_step_acid_iron_to_rust_pixel_transform() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        // Place iron at (4, 4) and acid at (5, 4) — adjacent.
        terrain.set_material_pixel(4, 4, 68, 0); // iron
        terrain.set_material_pixel(5, 4, 21, 0); // acid
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        let mut kernel = MaterialKernel::new();
        let report =
            kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        assert!(!report.reactions.is_empty(), "at least one reaction fires");
        let iron_now = terrain.material_at(4, 4);
        let acid_now = terrain.material_at(5, 4);
        assert_eq!(iron_now, 38, "iron pixel must transform to rust");
        assert_eq!(acid_now, 55, "acid pixel must transform to hydrogen byproduct");
    }

    /// VAL-M15-kernel-002: water + fire extinguishes the fire. The
    /// water pixel becomes steam, the fire pixel becomes smoke (per real
    /// chemistry: incomplete-combustion residue escapes as smoke when
    /// the fire dies — fire never goes straight to clean air).
    #[test]
    fn kernel_step_water_fire_extinguish() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 13, 0); // water
        terrain.set_material_pixel(4, 3, 65, 0); // fire
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        let mut kernel = MaterialKernel::new();
        let report =
            kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        assert!(report.reactions.iter().any(|e| e.reaction_id == "rxn.extinguish.water_fire"));
        assert_eq!(terrain.material_at(3, 3), 50, "water pixel → steam");
        assert_eq!(terrain.material_at(4, 3), 62, "fire pixel → smoke (extinguished)");
    }

    /// VAL-M15-kernel-003: water tile in a hot cell transforms to
    /// steam via the phase transition state machine.
    #[test]
    fn kernel_step_water_to_steam_phase_transition() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 13, 0); // water
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        // Two heat fields: prev = 360 K everywhere, curr = 380 K
        // everywhere → water crosses boil threshold.
        let mut prev = HeatField::default();
        let mut curr = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                prev.set_temperature(cx, cy, 360.0);
                curr.set_temperature(cx, cy, 380.0);
            }
        }
        let mut kernel = MaterialKernel::new();
        let report =
            kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &curr, Some(&prev));
        assert!(
            !report.phase_transitions.is_empty(),
            "water at 380K must trigger phase transition"
        );
        assert_eq!(terrain.material_at(3, 3), 50, "water pixel → steam after boil crossing");
    }

    /// VAL-M15-kernel-004: steam tile in a cold cell condenses back
    /// to water (reverse phase transition).
    #[test]
    fn kernel_step_steam_to_water_condensation() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 50, 0); // steam
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let mut prev = HeatField::default();
        let mut curr = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                prev.set_temperature(cx, cy, 380.0);
                curr.set_temperature(cx, cy, 360.0);
            }
        }
        let mut kernel = MaterialKernel::new();
        let report =
            kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &curr, Some(&prev));
        assert!(!report.phase_transitions.is_empty());
        assert_eq!(terrain.material_at(3, 3), 13, "steam pixel → water (condensed)");
    }

    /// VAL-M15-kernel-004b: full kernel_step with CA movement —
    /// reaction fires AND the steam product rises via the CA pass.
    /// Verifies the canonical order phase → reactions → movement.
    #[test]
    fn kernel_step_full_water_fire_then_steam_rises() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        // Dirt floor at row y=4 so liquids don't fall.
        for x in 0..8 {
            terrain.set_material_pixel(x, 4, 1, 0); // dirt
        }
        terrain.set_material_pixel(3, 3, 13, 0); // water on floor
        terrain.set_material_pixel(4, 3, 65, 0); // fire on floor
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        let mut kernel = MaterialKernel::new();
        let report = kernel_step(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        // Reaction must have fired.
        assert!(report.reactions.iter().any(|e| e.reaction_id == "rxn.extinguish.water_fire"));
        // Steam (product of water) should now be somewhere in the upper
        // area — either at the original water position or risen.
        let mut found_steam = false;
        for y in 0..4 {
            for x in 0..8 {
                if terrain.material_at(x, y) == 50 {
                    found_steam = true;
                }
            }
        }
        assert!(found_steam, "steam product must exist in upper rows");
        // Fire pixel is gone (extinguished to smoke per real chemistry).
        // After the CA pass smoke (a gas) may have risen one row.
        let mut found_smoke = false;
        for y in 0..8 {
            for x in 0..8 {
                if terrain.material_at(x, y) == 62 {
                    found_smoke = true;
                }
            }
        }
        assert!(found_smoke, "smoke product (from extinguished fire) must exist");
        assert_ne!(terrain.material_at(4, 3), 65, "fire pixel no longer fire");
    }

    /// VAL-M15-kernel-005: kernel respects the reaction cap.
    #[test]
    fn kernel_step_reaction_cap_limits_events() {
        let mut terrain = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        // Fill the upper-left quadrant with alternating iron/acid columns.
        for y in 0..32 {
            for x in 0..32 {
                let mat = if x % 2 == 0 { 68 } else { 21 };
                terrain.set_material_pixel(x, y, mat, 0);
            }
        }
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        let mut kernel = MaterialKernel::new();
        kernel.reaction_cap = 5;
        let report = kernel_step(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        assert!(report.reactions.len() <= 5, "cap respected (got {})", report.reactions.len());
    }

    /// VAL-M15-kernel-006: chunks that saw movement/reaction wake up.
    #[test]
    fn kernel_step_wakes_chunks_with_activity() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 13, 0);
        terrain.set_material_pixel(4, 3, 65, 0);
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        assert!(!terrain.chunk_active_region(0, 0));
        let mut kernel = MaterialKernel::new();
        let _ = kernel_step(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        assert!(terrain.chunk_active_region(0, 0), "chunk with reaction must be awake");
    }

    /// VAL-M15-kernel-007: idle chunks sleep after SLEEP_IDLE_THRESHOLD_TICKS.
    #[test]
    fn kernel_step_sleeps_idle_chunks() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        // Touch chunk (0,0) once, then idle for 500 ticks.
        terrain.set_material_pixel(3, 3, 1, 0);
        terrain.set_chunk_active_region(0, 0, true);
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        let mut kernel = MaterialKernel::new();
        // Advance the kernel tick far past the idle threshold.
        for _ in 0..400 {
            kernel.stepper.advance();
        }
        // Now run a step — the sleep pass should fire.
        let report = kernel_step(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        assert!(
            report.slept_chunks.iter().any(|c| *c == (0, 0)),
            "chunk (0,0) must sleep after idle threshold"
        );
        assert!(!terrain.chunk_active_region(0, 0));
    }

    /// VAL-M15-kernel-008: with awake_only=true and no awake chunks,
    /// the CA pass does no work.
    #[test]
    fn kernel_step_wake_sleep_gating_skips_sleeping_chunks() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(4, 2, 14, 0); // sand
        // Don't wake the chunk.
        assert!(!terrain.chunk_active_region(0, 0));
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        let mut kernel = MaterialKernel::new().with_wake_sleep_gating(true);
        let report = kernel_step(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        assert_eq!(report.ca.pixels_moved, 0, "sleeping chunks skipped");
        assert_eq!(terrain.material_at(4, 2), 14, "sand unmoved");
    }

    /// VAL-M15-kernel-009: deterministic — same input → same output.
    #[test]
    fn kernel_step_is_deterministic_across_runs() {
        fn run() -> Vec<MaterialId> {
            let mut t = ChunkedTerrain::new(16, 16, MATERIAL_AIR);
            t.set_material_pixel(5, 5, 68, 0); // iron
            t.set_material_pixel(6, 5, 21, 0); // acid
            t.set_material_pixel(8, 8, 14, 0); // sand
            let reactions = default_reaction_registry();
            let phase = default_phase_registry();
            let heat = HeatField::default();
            let mut k = MaterialKernel::new();
            for _ in 0..30 {
                kernel_step(&mut t, &mut k, &reactions, &phase, &heat, None);
            }
            let mut snapshot = Vec::new();
            for y in 0..16 {
                for x in 0..16 {
                    snapshot.push(t.material_at(x, y));
                }
            }
            snapshot
        }
        let a = run();
        let b = run();
        assert_eq!(a, b, "kernel must be deterministic across runs");
    }

    /// VAL-M15-kernel-010: AddUpdatedMaterialArea contract — after
    /// reaction firing, the affected chunk is in the dirty set.
    #[test]
    fn kernel_step_marks_dirty_chunk_on_reaction() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        terrain.set_material_pixel(3, 3, 68, 0); // iron
        terrain.set_material_pixel(4, 3, 21, 0); // acid
        terrain.clear_dirty();
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let heat = empty_heat();
        let mut kernel = MaterialKernel::new();
        let _ = kernel_step(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        assert!(terrain.dirty_chunk_count() > 0, "dirty chunk set must be populated");
    }

    /// smoke + CO2 into adjacent air cells (the cascade-friendly
    /// tertiary-output path). The wood pixel becomes charcoal; the
    /// fire pixel stays as fire (cascade); smoke + CO2 appear in
    /// adjacent air cells.
    #[test]
    fn kernel_step_wood_fire_emits_smoke_and_co2() {
        let mut terrain = ChunkedTerrain::new(16, 16, MATERIAL_AIR);
        // Surround the wood + fire pair with air on all 4 sides.
        terrain.set_material_pixel(5, 5, 8, 0); // wood at (5,5)
        terrain.set_material_pixel(6, 5, 65, 0); // fire at (6,5)
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        // Heat field above the rxn.ignition.wood_fire 573K gate.
        let mut heat = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                heat.set_temperature(cx, cy, 800.0);
            }
        }
        let mut kernel = MaterialKernel::new();
        let report = kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);

        // Reaction fired.
        let rxn = report
            .reactions
            .iter()
            .find(|e| e.reaction_id == "rxn.ignition.wood_fire")
            .expect("wood+fire reaction must fire");
        assert_eq!(rxn.emissions.len(), 2, "wood+fire emits 2 tertiary products");
        assert_eq!(rxn.emissions[0], 62, "first emission must be smoke");
        assert_eq!(rxn.emissions[1], 53, "second emission must be co2");
        assert_eq!(
            rxn.emission_positions.len(),
            rxn.emissions.len(),
            "emission_positions length must match emissions"
        );
        // The wood pixel became charcoal; fire pixel stays as fire.
        assert_eq!(terrain.material_at(5, 5), 41, "wood → charcoal");
        assert_eq!(terrain.material_at(6, 5), 65, "fire pixel stays as fire (cascade)");
        // Both emission positions are non-dropped (room to spawn).
        for ep in &rxn.emission_positions {
            assert_ne!(ep[0], crate::reactions::EMISSION_DROPPED, "emission must place");
        }
        // The actual world cells at the emission positions carry the
        // emitted materials.
        let p0 = rxn.emission_positions[0];
        let p1 = rxn.emission_positions[1];
        assert_eq!(terrain.material_at(p0[0] as i64, p0[1] as i64), 62, "smoke placed");
        assert_eq!(terrain.material_at(p1[0] as i64, p1[1] as i64), 53, "co2 placed");
    }

    /// emissions don't kill the fire cascade. The CRITICAL property:
    /// after a wood+fire reaction fires (with emissions), the fire
    /// pixel MUST still be fire (so adjacent wood pixels can also
    /// react on subsequent ticks). This is the test that would have
    /// failed if I had set `byproduct: Some(smoke)` to emit smoke —
    /// it would have killed the cascade by replacing fire with smoke.
    #[test]
    fn kernel_step_wood_fire_cascade_preserves_fire_pixel() {
        let mut terrain = ChunkedTerrain::new(16, 16, MATERIAL_AIR);
        terrain.set_material_pixel(5, 5, 8, 0); // wood
        terrain.set_material_pixel(6, 5, 65, 0); // fire
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let mut heat = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                heat.set_temperature(cx, cy, 800.0);
            }
        }
        let mut kernel = MaterialKernel::new();
        let report = kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        let rxn = report
            .reactions
            .iter()
            .find(|e| e.reaction_id == "rxn.ignition.wood_fire")
            .expect("wood+fire reaction must fire");
        assert_eq!(rxn.emissions.len(), 2, "wood+fire must emit smoke + co2");
        // CRITICAL: fire pixel survived the reaction. If a future
        // refactor moved smoke into `byproduct` instead of `emissions`,
        // this assertion would fail because fire → smoke.
        assert_eq!(terrain.material_at(6, 5), 65, "fire pixel must survive the reaction");
        assert_eq!(terrain.material_at(5, 5), 41, "wood pixel became charcoal");
    }

    /// when no adjacent air cell is available, the emission is dropped
    /// + the event records the sentinel position.
    #[test]
    fn kernel_step_emissions_drop_when_no_adjacent_air() {
        let mut terrain = ChunkedTerrain::new(8, 8, MATERIAL_AIR);
        // Surround the wood + fire pair with dirt (no air around them).
        for y in 0..8 {
            for x in 0..8 {
                terrain.set_material_pixel(x, y, 1, 0); // dirt everywhere
            }
        }
        terrain.set_material_pixel(3, 3, 8, 0); // wood
        terrain.set_material_pixel(4, 3, 65, 0); // fire
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let mut heat = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                heat.set_temperature(cx, cy, 800.0);
            }
        }
        let mut kernel = MaterialKernel::new();
        let report = kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        let rxn = report
            .reactions
            .iter()
            .find(|e| e.reaction_id == "rxn.ignition.wood_fire")
            .expect("wood+fire reaction must fire");
        // Every emission position must be the sentinel because no air.
        for ep in &rxn.emission_positions {
            assert_eq!(
                ep[0],
                crate::reactions::EMISSION_DROPPED,
                "emission must drop when no air cell"
            );
            assert_eq!(ep[1], crate::reactions::EMISSION_DROPPED);
        }
    }

    /// emits a dense smoke cloud (2× smoke + CO2). The reaction's
    /// emissions vec has length 3.
    #[test]
    fn kernel_step_gunpowder_emits_dense_smoke_cloud() {
        let mut terrain = ChunkedTerrain::new(16, 16, MATERIAL_AIR);
        terrain.set_material_pixel(5, 5, 48, 0); // gunpowder
        terrain.set_material_pixel(6, 5, 65, 0); // fire
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        // Heat above 573K gate.
        let mut heat = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                heat.set_temperature(cx, cy, 800.0);
            }
        }
        let mut kernel = MaterialKernel::new();
        let report = kernel_step_no_movement(&mut terrain, &mut kernel, &reactions, &phase, &heat, None);
        let rxn = report
            .reactions
            .iter()
            .find(|e| e.reaction_id == "rxn.explosion.gunpowder_fire")
            .expect("gunpowder+fire reaction must fire");
        assert!(rxn.emissions.starts_with(&[62u16, 62, 53]), "spec literal: 2× smoke + CO2 (plus fire emissions for violence)");
    }

    fn seed_reactive_world() -> ChunkedTerrain {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        for i in 0..10 {
            t.set_material_pixel(5 + (i * 4), 5, 68, 0);
            t.set_material_pixel(6 + (i * 4), 5, 21, 0);
        }
        for i in 0..5 {
            t.set_material_pixel(5 + (i * 6), 10, 13, 0);
            t.set_material_pixel(6 + (i * 6), 10, 65, 0);
        }
        for i in 0..5 {
            t.set_material_pixel(5 + (i * 6), 15, 8, 0);
            t.set_material_pixel(6 + (i * 6), 15, 65, 0);
        }
        t
    }

    /// VAL: parallel phase dispatch produces byte-identical output to serial.
    #[test]
    fn val_parallel_phase_match_serial() {
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let mut heat = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                heat.set_temperature(cx, cy, 400.0);
            }
        }
        let prev = HeatField::default();

        let seed = |t: &mut ChunkedTerrain| {
            for x in 0..32 {
                for y in 0..16 {
                    if (x + y) % 4 == 0 {
                        t.set_material_pixel(x, y, 13, 0);
                    } else if (x + y) % 5 == 1 {
                        t.set_material_pixel(x, y, 15, 0);
                    } else if (x + y) % 7 == 2 {
                        t.set_material_pixel(x, y, 19, 0);
                    }
                }
            }
        };

        let mut a = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let mut b = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        seed(&mut a);
        seed(&mut b);

        let mut ka = MaterialKernel::new();
        let mut kb = MaterialKernel::new().with_parallel(true);
        for tick in 0..10u64 {
            let ra = kernel_step_no_movement(&mut a, &mut ka, &reactions, &phase, &heat, Some(&prev));
            let rb = kernel_step_no_movement(&mut b, &mut kb, &reactions, &phase, &heat, Some(&prev));
            assert_eq!(
                ra.phase_transitions.len(),
                rb.phase_transitions.len(),
                "phase count tick {tick}"
            );
            for (ea, eb) in ra.phase_transitions.iter().zip(rb.phase_transitions.iter()) {
                assert_eq!(ea.material, eb.material, "material tick {tick}");
                assert_eq!(ea.pos, eb.pos, "pos tick {tick}");
                assert_eq!(ea.product_material, eb.product_material, "product tick {tick}");
            }
            for y in 0..64i64 {
                for x in 0..64i64 {
                    assert_eq!(a.material_at(x, y), b.material_at(x, y), "pixel ({x},{y}) tick {tick}");
                }
            }
        }
    }

    /// VAL: parallel reaction dispatch produces byte-identical output to serial.
    #[test]
    fn val_parallel_reactions_match_serial() {
        let reactions = default_reaction_registry();
        let phase = default_phase_registry();
        let mut heat = HeatField::default();
        for cy in 0..cf_terrain::air::AIR_GRID_SIZE {
            for cx in 0..cf_terrain::air::AIR_GRID_SIZE {
                heat.set_temperature(cx, cy, 800.0);
            }
        }

        let mut a = seed_reactive_world();
        let mut b = seed_reactive_world();
        let mut ka = MaterialKernel::new();
        let mut kb = MaterialKernel::new().with_parallel(true);
        for tick in 0..15u64 {
            let ra = kernel_step_no_movement(&mut a, &mut ka, &reactions, &phase, &heat, None);
            let rb = kernel_step_no_movement(&mut b, &mut kb, &reactions, &phase, &heat, None);
            assert_eq!(ra.reactions.len(), rb.reactions.len(), "reaction count tick {tick}");
            for (ea, eb) in ra.reactions.iter().zip(rb.reactions.iter()) {
                assert_eq!(ea.reaction_id, eb.reaction_id, "reaction_id tick {tick}");
                assert_eq!(ea.pos, eb.pos, "pos tick {tick}");
                assert_eq!(ea.output, eb.output, "output tick {tick}");
                assert_eq!(ea.byproduct, eb.byproduct, "byproduct tick {tick}");
                assert_eq!(ea.emissions, eb.emissions, "emissions tick {tick}");
            }
            for y in 0..64i64 {
                for x in 0..64i64 {
                    let ma = a.material_at(x, y);
                    let mb = b.material_at(x, y);
                    assert_eq!(ma, mb, "pixel ({x},{y}) tick {tick}: serial={ma} parallel={mb}");
                }
            }
        }
    }
}
