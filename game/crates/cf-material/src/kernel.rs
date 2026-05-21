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
use crate::reactions::{reaction_event, ReactionRegistry, ReactionTriggeredEvent};

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

/// **M15** § the canonical per-tick orchestrator. Drives every active-
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

    // --- 1. Phase transition dispatch (pixel-identity changes by temp).
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

    // --- 2. Reaction dispatch (per-pixel adjacency).
    let mut reaction_events: Vec<ReactionTriggeredEvent> = Vec::new();
    let mut reactions_remaining = kernel.reaction_cap;
    let reactive_materials = reactions.primary_reactive_set();
    for (cx, cy) in &scan_chunks {
        if reactions_remaining == 0 {
            break;
        }
        let n = dispatch_reactions_in_chunk(
            terrain,
            *cx,
            *cy,
            reactions,
            &reactive_materials,
            heat,
            tick_before,
            &mut reaction_events,
            &mut reactions_remaining,
        );
        if n > 0 {
            terrain.wake_chunk_neighborhood(*cx, *cy);
        }
    }

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
    let reactive_materials = reactions.primary_reactive_set();
    for (cx, cy) in &scan_chunks {
        if reactions_remaining == 0 {
            break;
        }
        let n = dispatch_reactions_in_chunk(
            terrain,
            *cx,
            *cy,
            reactions,
            &reactive_materials,
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
    reactive_materials: &std::collections::BTreeSet<u8>,
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
            if !reactive_materials.contains(&pa) {
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
fn try_fire_reaction(
    terrain: &mut ChunkedTerrain,
    pa_pos: [i64; 2],
    pb_pos: [i64; 2],
    pa: u8,
    pb: u8,
    reactions: &ReactionRegistry,
    heat: &HeatField,
    tick: u64,
    out_events: &mut Vec<ReactionTriggeredEvent>,
) -> bool {
    if pa == pb {
        return false;
    }
    let temp = heat.temperature_at_world(pa_pos[0] as f32, pa_pos[1] as f32);
    let Some(rxn) = reactions.evaluate(pa, pb, temp) else {
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
    // M3 preservation rule 1: AddUpdatedMaterialArea is the canonical
    // dirty path. The per-pixel set_material_pixel calls already mark
    // dirty_chunks, but we also call add_updated_material_area to be
    // explicit + cover any future viewer/AI consumer that watches the
    // function call surface.
    let world_min = [(a_pos[0].min(b_pos[0])) as f32, (a_pos[1].min(b_pos[1])) as f32];
    let world_max = [
        (a_pos[0].max(b_pos[0]) + 1) as f32,
        (a_pos[1].max(b_pos[1]) + 1) as f32,
    ];
    terrain.add_updated_material_area(world_min, world_max);
    out_events.push(reaction_event(rxn, [a_pos[0] as i32, a_pos[1] as i32], tick));
    true
}

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
    phase_materials: &std::collections::BTreeSet<u8>,
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
    /// water pixel becomes steam, the fire pixel becomes air.
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
        assert_eq!(terrain.material_at(4, 3), 0, "fire pixel → air (extinguished)");
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
        // Fire pixel is gone (extinguished to air).
        assert_eq!(terrain.material_at(4, 3), 0, "fire extinguished");
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
        fn run() -> Vec<u8> {
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
}
