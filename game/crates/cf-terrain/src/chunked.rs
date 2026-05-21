//! M2 chunked pixel terrain.
//!
//! Per the canonical roadmap (M2 — Pixel Terrain And Materials) this module owns:
//!
//! - The chunked storage (`ChunkedTerrain`, `Chunk`, `ChunkCoord`) with sparse
//!   lazy allocation: chunks that match the default material are not stored.
//! - The DR-007 launch material set (8 ids) and the `MaterialAffordance` lookup.
//! - Carve / blast / fill primitives the engine drives from `act.player.dig`,
//!   blast outcomes, and scenario seeding.
//! - Dirty-region tracking the renderer consumes (one frame latency).
//! - Layout-stable `checksum_bytes` and JSON-serializable snapshot for M3A.
//! - Physics integration helpers (`aabb_overlaps_solid`, `column_top_solid_y`).
//!
//! Anti-scope (lands at M5.6 Material Kernel):
//! - Active CA / reaction table / phase change.
//! - Fluid/gas pressure or atmospherics.
//! - Heat conduction.
//!
//! Anti-scope (lands at M2's GPU path follow-up):
//! - GPU-assisted carving compute shader. The M2 Universal Enhancement row
//!   ("GPU compute path investigation") is satisfied by `notes.md` plus the
//!   deterministic CPU baseline implemented here. The CPU path stays the source
//!   of truth for replay determinism per DR-054.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::integrity::{
    apply_damage_formula, normalized_hardness, CascadeEvent, DamageKind, IntegrityBand, PenetrationOutcome, PixelMeta,
    PixelMetaGrid, PixelMetaKey, DEFAULT_CASCADE_DECAY_PCT, DEFAULT_CASCADE_DEPTH, DEFAULT_CASCADE_THRESHOLD,
};

/// One pixel's material id. The DR-007 launch set ships 8 ids; the runtime stays
/// `u8` so future expansion (M5.6 active material kernel) can fit additional ids
/// without changing the storage layout.
pub type MaterialId = u16;

pub const MATERIAL_AIR: MaterialId = 0;
pub const MATERIAL_DIRT: MaterialId = 1;
pub const MATERIAL_CONCRETE: MaterialId = 2;
pub const MATERIAL_METAL_NOHOOK: MaterialId = 3;
pub const MATERIAL_HAZARD: MaterialId = 4;
pub const MATERIAL_LOOSE_FILL: MaterialId = 5;
pub const MATERIAL_REPAIR_FILL: MaterialId = 6;
pub const MATERIAL_ANCHOR: MaterialId = 7;
/// **M14E** § Per-pixel structural integrity. Player-placed load-bearing
/// reinforcement (T1 craftable, 2 iron + 1 wood per beam). Anchorable,
/// non-piling, hardness=200 — locks the ±8-pixel integrity field around
/// it to integrity 500 so cave-in roll is suppressed.
pub const MATERIAL_SUPPORT_BEAM: MaterialId = 8;

/// 256x256 chunk size — matches the canonical roadmap M2 scope ("256×256
/// chunks; per-pixel material id; sparse storage"). Stored as `u32` so chunk
/// math doesn't sign-extend through `usize` casts.
pub const CHUNK_SIZE: u32 = 256;
const CHUNK_PIXELS: usize = (CHUNK_SIZE as usize) * (CHUNK_SIZE as usize);

/// Per-material affordance the renderer + AI + physics + tool dispatcher read.
/// The canonical roadmap (M2 scope) names this as: hardness, anchorability,
/// hazard flags, path-cost contribution, plus a tool-validity refusal reason
/// for the (intentionally non-diggable) `metal_nohook` and `anchor` materials.
///
/// Extended at M2 with the full OpenLieroX / CCCP affordance flag taxonomy
/// (drillable, blastable, beam_cuttable, projectile_passable, actor_passable,
/// blocks_line_of_sight, damage_on_touch). Future-compat fields read by
/// M5.6 active material kernel ride on `MaterialDef` in `cf-material`; the
/// runtime affordance table here stays Copy-friendly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialAffordance {
    pub id: MaterialId,
    pub name: &'static str,
    /// True if the material blocks actor / projectile motion.
    pub solid: bool,
    /// True if a digger tool can carve through it.
    pub diggable: bool,
    /// HP per pixel for diggable materials. Engine deducts `tool_strength`
    /// from this per dig call against pixels in the carve mask. Also used
    /// by the projectile penetration formula as the per-pixel integrity
    /// (`impulse_squared > integrity_squared` per CCCP `SceneMan.cpp:571`).
    pub hardness: f32,
    /// True if the material can support an anchor / climbing tool.
    pub anchorable: bool,
    /// True if the material damages actors that touch / occupy it.
    pub hazard: bool,
    /// Damage applied per tick of contact when `hazard == true`.
    pub damage_per_tick: f32,
    /// True if a drill tool can carve through it (mirrors OpenLieroX
    /// `material.h:17` flag). At M2 mirrors `diggable`; future drill
    /// presets may diverge.
    pub drillable: bool,
    /// True if an explosive blast can clear it given `force >= hardness`.
    pub blastable: bool,
    /// True if a beam / laser cutter can carve it (M5+ tool).
    pub beam_cuttable: bool,
    /// True if a projectile can pass through this material without stopping.
    /// At M2 mirrors `!solid` (air = passable, solids = block); future
    /// presets may set this for one-way breakables.
    pub projectile_passable: bool,
    /// True if an actor can walk / climb through this material (mirrors
    /// CCCP `Material.h` flag). At M2 mirrors `!solid`.
    pub actor_passable: bool,
    /// True if this material blocks line-of-sight raycasts for AI vision +
    /// future fog-of-war. M2 ships solids=true, air=false.
    pub blocks_line_of_sight: bool,
    /// Per-pixel stickiness chance (0..=1). When a projectile fails to
    /// penetrate, a roll < stickiness draws it into the terrain instead of
    /// bouncing (CCCP `Material.Stickiness`). M2 uses engine RNG.
    ///
    /// **M14 audit pass 3 (GAP-M3-01)**: the canonical source for these
    /// values is `content/materials/material_registry.json` (each material
    /// entry carries `"stickiness": <f32>`). The const `MATERIAL_TABLE`
    /// below mirrors the JSON. When editing one, edit the other —
    /// M15+ active material kernel will collapse the two into a single
    /// JSON-driven loader.
    pub stickiness: f32,
    /// Restitution coefficient for bouncing projectiles (0..=1). M2 wires
    /// this into `try_penetrate` for ricochets.
    pub restitution: f32,
    /// Friction coefficient (0..=1). Pairs with `restitution` for bounce
    /// physics; M5.5 collision matrix consumes it.
    pub friction: f32,
    /// Density (kg / pixel). Drives spawn-debris mass.
    pub density: f32,
    /// Optional debris material spawned when a projectile penetrates (CCCP
    /// `Material.SpawnMaterial`). `None` = no debris.
    pub spawn_material: Option<MaterialId>,
    /// AI path-cost contribution (1.0 = nominal floor; >1.0 = expensive).
    pub path_cost: f32,
    /// Material-overlay color (sRGB, alpha 0xFF). Pure black `[0,0,0,0]` for
    /// `air` so the overlay shows nothing on empty space.
    pub overlay_rgba: [u8; 4],
    /// Stable refusal reason emitted on `terrain.tool_refused` when the dig
    /// targets this material. `None` for diggable materials.
    pub refusal_reason: Option<&'static str>,
}

impl MaterialAffordance {
    pub fn is_solid(&self) -> bool {
        self.solid
    }
}

/// **M15** § Material affordances for the full active-material set.
/// Originally a launch-9 table (air, dirt, concrete, metal_nohook,
/// hazard, loose_fill, repair_fill, anchor, support_beam) for the M2
/// terrain milestone. M15 grew the active material registry to 89 ids
/// (water, oil, acid, lava, iron, wood, ore, gases, etc.).
///
/// **This table is the RENDER + PHYSICS source of truth** — the
/// JSON-driven `cf_material::MaterialRegistry` provides id/name/color
/// for chemistry, but display + physics fields (hardness, friction,
/// density, blastable, etc.) live HERE. Adding a new material to the
/// JSON registry alone makes it kernel-reactive but INVISIBLE to the
/// renderer + treated as air by physics; you must also add an entry
/// to this table.
///
/// Adding new materials: pick a unique `id` not already in this table,
/// add a `pub const MATERIAL_X: MaterialId = N;` constant above, then
/// add the affordance entry below. Bump the table size in the array
/// declaration.
const MATERIAL_TABLE: [MaterialAffordance; 21] = [
    MaterialAffordance {
        id: MATERIAL_AIR,
        name: "air",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 0.0,
        spawn_material: None,
        path_cost: 1.0,
        overlay_rgba: [0, 0, 0, 0],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_DIRT,
        name: "dirt",
        solid: true,
        diggable: true,
        // CCCP earth normalized; spec M2 baseline = 10.
        hardness: 10.0,
        anchorable: true,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: true,
        blastable: true,
        beam_cuttable: true,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.05,
        restitution: 0.05,
        friction: 0.6,
        density: 1.5,
        spawn_material: Some(MATERIAL_LOOSE_FILL),
        path_cost: 1.0,
        overlay_rgba: [120, 80, 50, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_CONCRETE,
        name: "concrete",
        solid: true,
        diggable: true,
        // CCCP concrete=200 normalized. M3 audit pass 7 (2026-05-13):
        // bumped from 40 → 50 so the dirt/concrete hardness ratio is
        // exactly 5x per spec literal ("concrete carves in 5-10x the dirt
        // time"). 50/10 = 5x lower bound; spec allows up to 10x.
        hardness: 50.0,
        anchorable: true,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: true,
        blastable: true,
        beam_cuttable: true,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.0,
        restitution: 0.30,
        friction: 0.7,
        density: 2.3,
        spawn_material: Some(MATERIAL_LOOSE_FILL),
        path_cost: 1.0,
        overlay_rgba: [180, 180, 180, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_METAL_NOHOOK,
        name: "metal_nohook",
        solid: true,
        diggable: false,
        // Spec: hardness=100 (CCCP metal=400 normalized; refuse-by-default
        // high integrity). The digger refuses regardless of hardness via
        // `diggable=false`; the value is used by projectile penetration +
        // blast force gate. M5.6 may add a drill-strength override.
        hardness: 100.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: true,
        beam_cuttable: false,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.70,
        restitution: 0.40,
        friction: 0.5,
        density: 7.8,
        spawn_material: None,
        path_cost: 999.0,
        overlay_rgba: [80, 100, 140, 0xFF],
        // M3 audit pass 5 (2026-05-13): spec literal demands
        // `reason="material_not_diggable"` for all non-diggable carve refusals.
        // Concrete `material_<name>` strings are kept as the structured
        // `material` field on the event payload; this is the stable reason
        // vocabulary the spec contract specifies.
        refusal_reason: Some("material_not_diggable"),
    },
    MaterialAffordance {
        id: MATERIAL_HAZARD,
        name: "hazard",
        // Spec: hazard is solid + damage_on_touch=true. M2 treats hazard as
        // refusal-only for the digger (diggable=false), still a hazard
        // surface for the actor (damage_per_tick=2.0), and blastable when
        // `force >= 50`. The previous "f32::INFINITY for refusal-only at
        // BP2" decision is superseded by the M2 spec — hazard hardness=50
        // and blastable=true so explosives can clear it (the M5.6 active
        // material kernel will later add a dispersal/reaction path).
        solid: true,
        diggable: false,
        hardness: 50.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 2.0,
        drillable: false,
        blastable: true,
        beam_cuttable: false,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.10,
        friction: 0.5,
        density: 3.0,
        spawn_material: Some(MATERIAL_LOOSE_FILL),
        path_cost: 10.0,
        overlay_rgba: [200, 60, 60, 0xFF],
        // M3 audit pass 5 (2026-05-13): non-diggable carve refusals route
        // through the stable `material_not_diggable` reason; the specific
        // material is on the payload's `material` field.
        refusal_reason: Some("material_not_diggable"),
    },
    MaterialAffordance {
        id: MATERIAL_LOOSE_FILL,
        name: "loose_fill",
        solid: true,
        diggable: true,
        // Spec: hardness=5 (CCCP earth_rubble=25 normalized; soft fill).
        hardness: 5.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: true,
        blastable: true,
        beam_cuttable: true,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.10,
        restitution: 0.0,
        friction: 0.4,
        density: 1.2,
        spawn_material: None,
        path_cost: 2.0,
        overlay_rgba: [200, 170, 90, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_REPAIR_FILL,
        name: "repair_fill",
        solid: true,
        diggable: true,
        // Spec: hardness=15 (player-placed repair foam; medium).
        hardness: 15.0,
        anchorable: true,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: true,
        blastable: true,
        beam_cuttable: true,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.20,
        restitution: 0.15,
        friction: 0.6,
        density: 0.8,
        spawn_material: Some(MATERIAL_LOOSE_FILL),
        path_cost: 1.0,
        overlay_rgba: [120, 200, 140, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: MATERIAL_ANCHOR,
        name: "anchor",
        solid: true,
        diggable: false,
        // Spec: hardness=60 (CCCP stone=140 normalized; harder than dirt,
        // anchorable). Digger refuses via `diggable=false`; blast clears
        // at force >= 60.
        hardness: 60.0,
        anchorable: true,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: true,
        beam_cuttable: false,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.50,
        restitution: 0.30,
        friction: 0.7,
        density: 2.6,
        spawn_material: Some(MATERIAL_LOOSE_FILL),
        path_cost: 1.0,
        overlay_rgba: [60, 60, 200, 0xFF],
        // M3 audit pass 5 (2026-05-13): non-diggable carve refusals route
        // through the stable `material_not_diggable` reason; the specific
        // material is on the payload's `material` field.
        refusal_reason: Some("material_not_diggable"),
    },
    // **M14E** § Per-pixel structural integrity. Player-placed support
    // beam (T1 craftable). Hardness=200 + anchorable=true; non-piling so
    // it doesn't pile-fill its neighbors. Locks the ±8-pixel structural
    // integrity field around itself to 500 (load-bearing). Diggable so the
    // demolish-beam scenario can remove it; blast clears at force >= 200.
    MaterialAffordance {
        id: MATERIAL_SUPPORT_BEAM,
        name: "support_beam",
        solid: true,
        diggable: true,
        hardness: 200.0,
        anchorable: true,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: true,
        blastable: true,
        beam_cuttable: true,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.20,
        restitution: 0.20,
        friction: 0.6,
        density: 2.0,
        spawn_material: Some(MATERIAL_LOOSE_FILL),
        path_cost: 1.0,
        overlay_rgba: [110, 70, 30, 0xFF],
        refusal_reason: None,
    },
    // ===== M15 ACTIVE MATERIAL AFFORDANCES =====
    // The following 12 entries cover the M15B precipitation chain +
    // the most-emitted reaction products so they render visibly +
    // physics-interact instead of being silently treated as air.
    MaterialAffordance {
        id: 13, // water
        name: "water",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.05,
        restitution: 0.0,
        friction: 0.10,
        density: 1.0,
        spawn_material: None,
        path_cost: 2.0,
        overlay_rgba: [40, 100, 220, 200],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 19,
        name: "oil",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.10,
        restitution: 0.0,
        friction: 0.20,
        density: 0.88,
        spawn_material: None,
        path_cost: 2.0,
        overlay_rgba: [26, 20, 16, 220],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 21, // acid
        name: "acid",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 2.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.05,
        restitution: 0.0,
        friction: 0.10,
        density: 1.2,
        spawn_material: None,
        path_cost: 3.0,
        overlay_rgba: [180, 240, 80, 200],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 26, // lava
        name: "lava",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 12.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.15,
        restitution: 0.0,
        friction: 0.30,
        density: 2.8,
        spawn_material: None,
        path_cost: 8.0,
        overlay_rgba: [220, 80, 20, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 68,
        name: "iron",
        solid: true,
        diggable: true,
        hardness: 80.0,
        anchorable: true,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: true,
        blastable: true,
        beam_cuttable: true,
        projectile_passable: false,
        actor_passable: false,
        blocks_line_of_sight: true,
        stickiness: 0.0,
        restitution: 0.30,
        friction: 0.50,
        density: 7.87,
        spawn_material: Some(MATERIAL_LOOSE_FILL),
        path_cost: 1.0,
        overlay_rgba: [120, 120, 130, 0xFF],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 43, // co2
        name: "co2",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 0.18,
        spawn_material: None,
        path_cost: 1.0,
        overlay_rgba: [200, 200, 210, 60],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 50, // steam
        name: "steam",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 0.06,
        spawn_material: None,
        path_cost: 1.0,
        overlay_rgba: [220, 220, 240, 100],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 62, // smoke
        name: "smoke",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: true,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 0.12,
        spawn_material: None,
        path_cost: 1.0,
        overlay_rgba: [50, 50, 55, 160],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 65, // fire_intense
        name: "fire_intense",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 8.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 0.20,
        spawn_material: None,
        path_cost: 6.0,
        overlay_rgba: [240, 120, 20, 220],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 71, // cloud
        name: "cloud",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 0.08,
        spawn_material: None,
        path_cost: 1.0,
        overlay_rgba: [240, 240, 250, 140],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 87, // rain
        name: "rain",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: false,
        damage_per_tick: 0.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.10,
        density: 1.0,
        spawn_material: None,
        path_cost: 2.0,
        overlay_rgba: [60, 120, 220, 200],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 88, // acid_droplet
        name: "acid_droplet",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 2.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.05,
        restitution: 0.0,
        friction: 0.10,
        density: 1.2,
        spawn_material: None,
        path_cost: 4.0,
        overlay_rgba: [180, 240, 100, 200],
        refusal_reason: None,
    },
];

/// Look up a material affordance by id. `None` if the id is outside the launch
/// set; callers should treat unknown ids as `air` and emit a structured warning.
#[must_use]
pub fn material_affordance(id: MaterialId) -> Option<&'static MaterialAffordance> {
    MATERIAL_TABLE.iter().find(|m| m.id == id)
}

/// Resolve the canonical material name for a `MaterialId`. Returns `"unknown"`
/// for ids outside the launch set (callers should treat as `air`).
#[must_use]
pub fn material_name_from_id(id: MaterialId) -> &'static str {
    material_affordance(id).map(|m| m.name).unwrap_or("unknown")
}

/// Resolve a material name (case-sensitive) from a scenario manifest. Names
/// match the DR-007 launch set verbatim. `concrete_soft` is a deprecated M1.5
/// alias of `concrete` retained for backward compat with `micro_breach.ron`.
///
/// **M15B** extension: scenarios can now stamp the precipitation chain
/// materials directly (`water`, `steam`, `cloud`, `rain`, `acid_droplet`)
/// so the m15b_water_cycle_demo + m15b_acid_rain_vulcan scenarios load
/// without engine-side seeding.
#[must_use]
pub fn material_id_from_name(name: &str) -> Option<MaterialId> {
    match name {
        "air" => Some(MATERIAL_AIR),
        "dirt" => Some(MATERIAL_DIRT),
        "concrete" | "concrete_soft" => Some(MATERIAL_CONCRETE),
        "metal_nohook" => Some(MATERIAL_METAL_NOHOOK),
        "hazard" => Some(MATERIAL_HAZARD),
        "loose_fill" => Some(MATERIAL_LOOSE_FILL),
        "repair_fill" => Some(MATERIAL_REPAIR_FILL),
        "anchor" => Some(MATERIAL_ANCHOR),
        "support_beam" => Some(MATERIAL_SUPPORT_BEAM),
        // **M15B** § precipitation chain stampable ids.
        "water" => Some(13),
        "steam" => Some(50),
        "cloud" => Some(71),
        "rain" => Some(87),
        "acid_droplet" => Some(88),
        // **M15** § active material set — names from
        // `content/materials/material_registry.json`.
        "oil" => Some(16),
        "acid" => Some(21),
        "lava" => Some(26),
        "iron" => Some(29),
        "co2" => Some(43),
        "smoke" => Some(62),
        "fire_intense" => Some(65),
        _ => None,
    }
}

/// Material schema version stamped into `run_manifest.json.material_schema_version`
/// so future schema migrations can identify legacy bundles.
pub const MATERIAL_SCHEMA_VERSION: &str = "cf-terrain-launch-v1";

/// One chunk in the terrain. Stored densely as a row-major array of `MaterialId`s.
/// Sparse: chunks with all pixels equal to the terrain's `default_material` are
/// NOT stored at all — `ChunkedTerrain::material_at` returns the default in
/// that case.
///
/// **M3 re-open (2026-05-13) fix #7**: forward-compat fields for M14 (sub-rect
/// upload) + M15 (active material kernel). All four extra fields are
/// `serde(default)` so legacy v0.1 snapshots round-trip cleanly:
/// - `active_region: bool` — M15 active-cell hint; true when any cell in this
///   chunk participates in an ongoing reaction (per CCCP active-region pattern)
/// - `last_modified_tick: u64` — bumped by every successful `set_pixel`; used
///   by M22 pathfinder + M15 active-region eviction policy
/// - `color_grid: Option<Vec<u32>>` — M14 per-pixel color cache (Noita-grade
///   visual variation); None at M3 means "render the material's color_hex"
/// - `dirty_rect: Option<DirtyRect>` — M14 sub-rect upload bound. None at M3
///   means "uploader assumes whole-chunk dirty"
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    pixels: Vec<MaterialId>,
    #[serde(default)]
    pub active_region: bool,
    #[serde(default)]
    pub last_modified_tick: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_grid: Option<Vec<u32>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dirty_rect: Option<DirtyRect>,
}

/// **M3 re-open (2026-05-13) fix #7 + fix #6**: per-chunk dirty AABB for
/// sub-rect upload (M14 forward-compat). Coordinates are LOCAL to the chunk
/// (0..CHUNK_SIZE). `min`/`max` inclusive. None means "no pending dirty
/// region"; the chunk is considered up-to-date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirtyRect {
    pub min: [u32; 2],
    pub max: [u32; 2],
}

impl DirtyRect {
    pub fn single(lx: u32, ly: u32) -> Self {
        Self {
            min: [lx, ly],
            max: [lx, ly],
        }
    }

    pub fn extend(&mut self, lx: u32, ly: u32) {
        self.min[0] = self.min[0].min(lx);
        self.min[1] = self.min[1].min(ly);
        self.max[0] = self.max[0].max(lx);
        self.max[1] = self.max[1].max(ly);
    }
}

impl Chunk {
    fn uniform(material: MaterialId) -> Self {
        Self {
            pixels: vec![material; CHUNK_PIXELS],
            active_region: false,
            last_modified_tick: 0,
            color_grid: None,
            dirty_rect: None,
        }
    }

    /// Material at `(lx, ly)` inside this chunk; `lx`/`ly` are local 0..CHUNK_SIZE.
    pub fn pixel(&self, lx: u32, ly: u32) -> MaterialId {
        debug_assert!(lx < CHUNK_SIZE && ly < CHUNK_SIZE);
        self.pixels[(ly as usize) * (CHUNK_SIZE as usize) + (lx as usize)]
    }

    /// Set the material at `(lx, ly)`; returns true if the cell changed.
    /// **M3 re-open fix #6/#7**: also extends `dirty_rect` to cover this pixel
    /// and bumps `last_modified_tick` (caller is responsible for passing the
    /// current tick via `set_pixel_at_tick`). For backward compat, this
    /// 3-arg form leaves `last_modified_tick` alone.
    pub fn set_pixel(&mut self, lx: u32, ly: u32, mat: MaterialId) -> bool {
        debug_assert!(lx < CHUNK_SIZE && ly < CHUNK_SIZE);
        let idx = (ly as usize) * (CHUNK_SIZE as usize) + (lx as usize);
        let prev = self.pixels[idx];
        if prev == mat {
            return false;
        }
        self.pixels[idx] = mat;
        // Extend the dirty rect so the M14 sub-rect upload bridge can re-upload
        // only the affected sub-region instead of the whole 256×256 chunk.
        match &mut self.dirty_rect {
            Some(rect) => rect.extend(lx, ly),
            None => self.dirty_rect = Some(DirtyRect::single(lx, ly)),
        }
        true
    }

    /// **M3 re-open fix #6/#7**: M15 forward-compat — set the material AND
    /// stamp `last_modified_tick`. Returns true if the cell changed.
    pub fn set_pixel_at_tick(&mut self, lx: u32, ly: u32, mat: MaterialId, tick: u64) -> bool {
        if self.set_pixel(lx, ly, mat) {
            self.last_modified_tick = tick;
            true
        } else {
            false
        }
    }

    /// **M3 re-open fix #6**: drain the per-chunk dirty rect (consumes it).
    /// Returns `None` when the chunk has nothing pending. Caller uploads the
    /// returned sub-rect through the render bridge.
    pub fn take_dirty_rect(&mut self) -> Option<DirtyRect> {
        self.dirty_rect.take()
    }

    /// True if every pixel equals `mat`. Used to compress chunks back to the
    /// default after a fill operation.
    pub fn is_uniform(&self, mat: MaterialId) -> bool {
        self.pixels.iter().all(|m| *m == mat)
    }

    /// Number of solid pixels in this chunk. Used by `aabb_overlaps_solid` and
    /// the run-bundle summary to track how much of the world is currently solid.
    pub fn solid_count(&self, registry: &MaterialRegistry) -> u32 {
        self.pixels.iter().filter(|m| registry.is_solid(**m)).count() as u32
    }
}

/// Chunk coordinate in chunk-space (not pixel-space). `(0, 0)` covers pixels
/// `[0..256, 0..256]`; `(1, 0)` covers `[256..512, 0..256]`; etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub cx: i32,
    pub cy: i32,
}

impl ChunkCoord {
    pub const fn new(cx: i32, cy: i32) -> Self {
        Self { cx, cy }
    }

    /// Pixel-space origin of this chunk's `(0, 0)` corner.
    pub fn pixel_origin(&self) -> [i64; 2] {
        [
            (self.cx as i64) * (CHUNK_SIZE as i64),
            (self.cy as i64) * (CHUNK_SIZE as i64),
        ]
    }
}

/// Wrapper around `[MaterialAffordance; 8]` so consumers can ask "is this id
/// solid?" / "is this id diggable?" without touching the constants directly.
#[derive(Debug, Clone, Copy)]
pub struct MaterialRegistry;

impl MaterialRegistry {
    pub fn affordance(&self, id: MaterialId) -> Option<&'static MaterialAffordance> {
        material_affordance(id)
    }

    pub fn is_solid(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(MaterialAffordance::is_solid)
    }

    pub fn is_diggable(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(|m| m.diggable)
    }

    pub fn refusal_reason(&self, id: MaterialId) -> Option<&'static str> {
        self.affordance(id).and_then(|m| m.refusal_reason)
    }

    pub fn name(&self, id: MaterialId) -> &'static str {
        self.affordance(id).map(|m| m.name).unwrap_or("unknown")
    }

    pub fn hardness(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.hardness).unwrap_or(0.0)
    }

    pub fn is_hazard(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(|m| m.hazard)
    }

    pub fn damage_per_tick(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.damage_per_tick).unwrap_or(0.0)
    }

    pub fn is_anchorable(&self, id: MaterialId) -> bool {
        self.affordance(id).is_some_and(|m| m.anchorable)
    }

    pub fn path_cost(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.path_cost).unwrap_or(1.0)
    }

    pub fn stickiness(&self, id: MaterialId) -> f32 {
        self.affordance(id).map(|m| m.stickiness).unwrap_or(0.0)
    }

    pub fn spawn_material(&self, id: MaterialId) -> Option<MaterialId> {
        self.affordance(id).and_then(|m| m.spawn_material)
    }
}

/// One outcome of a `try_carve` / `try_blast` call.
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkedCarveOutcome {
    Carved(ChunkedCarveStats),
    Refused(ChunkedCarveRefusal),
    NoOp(ChunkedCarveNoOp),
}

/// One successful carve. `bbox_min/max` are inclusive pixel-space bounds
/// covering every affected pixel; `count` is the number of pixels removed.
/// `material_*` records the dominant (mode) material the carve removed so the
/// HUD + replay viewer can label the action ("dug 28 dirt pixels", etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedCarveStats {
    pub bbox_min: [i64; 2],
    pub bbox_max: [i64; 2],
    pub count: u32,
    pub dominant_material: MaterialId,
    pub dirty_chunks: Vec<ChunkCoord>,
    pub refusal_reason: Option<&'static str>,
}

/// Carve refused. Distinct from `NoOp` because refusals must be visible in the
/// HUD + replay (player learns why their dig didn't work). M2 surfaces three
/// refusal reasons in the launch set: `material_metal_nohook`, `material_hazard`,
/// and the legacy alias `out_of_range` for empty carves.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedCarveRefusal {
    pub reason: &'static str,
    pub probe_at: [i64; 2],
    pub material: MaterialId,
}

/// Carve hit only `air` (or some other non-solid material). Engine emits this
/// silently — no `tool_refused`, no `terrain_carved`. The HUD displays an
/// "out of range" cue.
#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedCarveNoOp {
    pub probe_at: [i64; 2],
}

/// One terrain stamp from the scenario manifest. Stamps run in declaration
/// order; later stamps overwrite earlier ones. Convenient for hand-authoring
/// test scenarios without a full level editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TerrainStamp {
    /// Fill an axis-aligned box in pixel space with `material`.
    FillAabb {
        min: [f32; 2],
        max: [f32; 2],
        material: String,
    },
    /// Fill a circular region in pixel space with `material`.
    FillCircle {
        center: [f32; 2],
        radius: f32,
        material: String,
    },
}

/// JSON-serializable snapshot of the chunked terrain. Used by the M3A snapshot
/// pipeline + the replay viewer + the cf-headless replay verifier. Layout is
/// stable: future migrations bump `schema` and register a converter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkedTerrainSnapshot {
    pub schema: String,
    pub width_px: u32,
    pub height_px: u32,
    pub anchor: [f32; 2],
    pub default_material: MaterialId,
    pub carve_count: u64,
    pub refusal_count: u64,
    pub material_counts: BTreeMap<String, u64>,
    pub chunks: Vec<ChunkedTerrainSnapshotChunk>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChunkedTerrainSnapshotChunk {
    pub coord: ChunkCoord,
    /// Per-pixel material ids, row-major, length `CHUNK_SIZE * CHUNK_SIZE`.
    pub pixels: Vec<MaterialId>,
}

/// Chunked terrain core. Owns the chunk map, dirty regions, perf counters, and
/// snapshot/checksum APIs. The engine wraps this in `RwLock` so the per-tick
/// dig handler can mutate while the renderer reads.
#[derive(Debug, Clone)]
pub struct ChunkedTerrain {
    pub anchor: [f32; 2],
    pub width_px: u32,
    pub height_px: u32,
    pub default_material: MaterialId,
    chunks: BTreeMap<ChunkCoord, Chunk>,
    dirty_chunks: BTreeSet<ChunkCoord>,
    pub carve_count: u64,
    pub refusal_count: u64,
    pub registry: MaterialRegistry,
    /// **M3 re-audit pass 4 (2026-05-13)**: most-recent sim tick stamped
    /// onto chunks that were modified this tick. Engine calls
    /// `set_current_tick` from `drive_tick` so every subsequent pixel
    /// write updates the affected chunk's `last_modified_tick`. Default
    /// 0 (chunks never modified). Not serialized (transient runtime state).
    pub current_tick: u64,
    /// **M9 § Destructible terrain — 5-tier per-pixel integrity grid**.
    /// Sparse: only damaged pixels appear. Air pixels are never tracked.
    /// Pristine (1.0) pixels are implicit — they only get an entry once
    /// damage lands. Layout-stable across runs (BTreeMap ordering) so
    /// snapshot round-trips preserve determinism for M4's `sim_state_v1`
    /// checksum scope.
    pub pixel_meta_grid: PixelMetaGrid,
}

impl ChunkedTerrain {
    /// New empty terrain with `width_px x height_px` extent and a uniform
    /// default material across every cell.
    pub fn new(width_px: u32, height_px: u32, default_material: MaterialId) -> Self {
        Self {
            anchor: [0.0, 0.0],
            width_px,
            height_px,
            default_material,
            chunks: BTreeMap::new(),
            dirty_chunks: BTreeSet::new(),
            carve_count: 0,
            refusal_count: 0,
            registry: MaterialRegistry,
            current_tick: 0,
            pixel_meta_grid: PixelMetaGrid::new(),
        }
    }

    /// **M3 re-audit pass 4 (2026-05-13)**: engine calls this each tick so
    /// the next pixel write stamps the right `last_modified_tick` on the
    /// affected chunk.
    pub fn set_current_tick(&mut self, tick: u64) {
        self.current_tick = tick;
    }

    /// Apply a list of [`TerrainStamp`]s. Returns the number of pixels written.
    /// Stamps run in declaration order; later stamps overwrite earlier ones.
    pub fn apply_stamps(&mut self, stamps: &[TerrainStamp]) -> u64 {
        let mut written: u64 = 0;
        for stamp in stamps {
            match stamp {
                TerrainStamp::FillAabb { min, max, material } => {
                    let Some(mat) = material_id_from_name(material) else {
                        tracing::warn!(
                            target: "cf::terrain",
                            material = %material,
                            "fill_aabb: unknown material; skipping"
                        );
                        continue;
                    };
                    written += self.fill_aabb(*min, *max, mat);
                }
                TerrainStamp::FillCircle {
                    center,
                    radius,
                    material,
                } => {
                    let Some(mat) = material_id_from_name(material) else {
                        tracing::warn!(
                            target: "cf::terrain",
                            material = %material,
                            "fill_circle: unknown material; skipping"
                        );
                        continue;
                    };
                    written += self.fill_circle(*center, *radius, mat);
                }
            }
        }
        // Stamps are an init step; clear dirty so the renderer doesn't think
        // these are interactive carves to flash.
        self.dirty_chunks.clear();
        written
    }

    /// Fill a closed pixel-space AABB with `mat`. Returns the number of pixels
    /// changed. Coordinates are clamped to the terrain extent.
    pub fn fill_aabb(&mut self, min: [f32; 2], max: [f32; 2], mat: MaterialId) -> u64 {
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        let mut written: u64 = 0;
        for py in y0..y1 {
            for px in x0..x1 {
                if self.set_pixel_internal(px, py, mat) {
                    written += 1;
                }
            }
        }
        written
    }

    /// Fill a circular region in pixel space with `mat`. Returns the number of
    /// pixels changed.
    pub fn fill_circle(&mut self, center: [f32; 2], radius: f32, mat: MaterialId) -> u64 {
        let r = radius.max(0.0);
        let r2 = r * r;
        let cx = center[0];
        let cy = center[1];
        let min = [cx - r, cy - r];
        let max = [cx + r, cy + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        // Bugbot 3212180092: convert center world-space → pixel-space so
        // the circle test matches the iteration bounds (which are
        // pixel-space coming out of `aabb_to_pixels`).
        let center_px_x = cx - self.anchor[0];
        let center_px_y = cy - self.anchor[1];
        let mut written: u64 = 0;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy <= r2 && self.set_pixel_internal(px, py, mat) {
                    written += 1;
                }
            }
        }
        written
    }

    /// Material at the given pixel coordinate. Returns the terrain default for
    /// out-of-range or unallocated chunks.
    pub fn material_at(&self, px: i64, py: i64) -> MaterialId {
        if !self.in_bounds(px, py) {
            return self.default_material;
        }
        let (coord, lx, ly) = chunk_split(px, py);
        match self.chunks.get(&coord) {
            Some(c) => c.pixel(lx, ly),
            None => self.default_material,
        }
    }

    /// Material at world-space coordinates (continuous floats). The terrain
    /// anchor + integer flooring resolve world space to pixel space.
    pub fn material_at_world(&self, world_x: f32, world_y: f32) -> MaterialId {
        let px = (world_x - self.anchor[0]).floor() as i64;
        let py = (world_y - self.anchor[1]).floor() as i64;
        self.material_at(px, py)
    }

    /// Try to carve a circular region. Tool semantics:
    ///
    /// - Pixels matching the carve mask AND `is_diggable(mat) = true` become
    ///   `default_material` (typically `air`).
    /// - Pixels matching the mask AND `is_diggable(mat) = false` AND with a
    ///   `refusal_reason` short-circuit the carve and return
    ///   [`ChunkedCarveOutcome::Refused`] before any other pixel changes.
    /// - When the mask hits only non-solid (`air`) pixels, we return
    ///   [`ChunkedCarveOutcome::NoOp`] so callers can label "out_of_range".
    ///
    /// This is the same vocabulary as `cf-terrain::BreachStrip::try_dig` so
    /// existing M1.5 consumers don't need migration.
    pub fn try_carve(&mut self, origin: [f32; 2], radius: f32) -> ChunkedCarveOutcome {
        let r = radius.max(0.0);
        let r2 = r * r;
        let min = [origin[0] - r, origin[1] - r];
        let max = [origin[0] + r, origin[1] + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        // Bugbot 3212180092: `aabb_to_pixels` subtracts the terrain anchor
        // when converting world → pixel space, so `px` / `py` iterate in
        // pixel-space. `origin` is world-space. Compare in pixel-space by
        // pre-subtracting the anchor from the origin so the carve circle
        // isn't off-center for non-zero anchors.
        let center_px_x = origin[0] - self.anchor[0];
        let center_px_y = origin[1] - self.anchor[1];

        // First pass: probe for refusal-reason materials. A refusal short-circuits
        // the whole carve so the player gets a clean "this won't work" event.
        let mut probe = [center_px_x.round() as i64, center_px_y.round() as i64];
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if let Some(reason) = self.registry.refusal_reason(mat) {
                    self.refusal_count += 1;
                    return ChunkedCarveOutcome::Refused(ChunkedCarveRefusal {
                        reason,
                        probe_at: [px, py],
                        material: mat,
                    });
                }
                probe = [px, py];
            }
        }

        // Second pass: carve diggable pixels. Track the mode material removed
        // so the event payload can label the carve.
        let mut counts: BTreeMap<MaterialId, u32> = BTreeMap::new();
        let mut count: u32 = 0;
        let mut bbox_min = [i64::MAX, i64::MAX];
        let mut bbox_max = [i64::MIN, i64::MIN];
        let mut dirty: BTreeSet<ChunkCoord> = BTreeSet::new();
        let air = self.default_material;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if !self.registry.is_diggable(mat) {
                    continue;
                }
                if self.set_pixel_internal(px, py, air) {
                    *counts.entry(mat).or_insert(0) += 1;
                    count += 1;
                    bbox_min[0] = bbox_min[0].min(px);
                    bbox_min[1] = bbox_min[1].min(py);
                    bbox_max[0] = bbox_max[0].max(px);
                    bbox_max[1] = bbox_max[1].max(py);
                    let (coord, _, _) = chunk_split(px, py);
                    dirty.insert(coord);
                }
            }
        }
        if count == 0 {
            return ChunkedCarveOutcome::NoOp(ChunkedCarveNoOp { probe_at: probe });
        }
        self.carve_count += 1;
        let dominant_material = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(m, _)| *m)
            .unwrap_or(self.default_material);
        ChunkedCarveOutcome::Carved(ChunkedCarveStats {
            bbox_min,
            bbox_max,
            count,
            dominant_material,
            dirty_chunks: dirty.into_iter().collect(),
            refusal_reason: None,
        })
    }

    /// Larger blast carve: same semantics as `try_carve` but with a wider
    /// default radius and ignores refusal-reason materials when `force` is
    /// strictly above the registered hardness. M2 ships a CPU baseline; the
    /// GPU compute path lands as a follow-up.
    pub fn try_blast(&mut self, origin: [f32; 2], radius: f32, force: f32) -> ChunkedCarveOutcome {
        let r = radius.max(0.0);
        let r2 = r * r;
        let min = [origin[0] - r, origin[1] - r];
        let max = [origin[0] + r, origin[1] + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        // Bugbot 3212180092: same world→pixel anchor subtraction as
        // try_carve. The blast circle test must compare px (pixel-space)
        // against an anchor-subtracted origin so non-zero anchors don't
        // off-center the blast.
        let center_px_x = origin[0] - self.anchor[0];
        let center_px_y = origin[1] - self.anchor[1];

        let mut counts: BTreeMap<MaterialId, u32> = BTreeMap::new();
        let mut count: u32 = 0;
        let mut bbox_min = [i64::MAX, i64::MAX];
        let mut bbox_max = [i64::MIN, i64::MIN];
        let mut dirty: BTreeSet<ChunkCoord> = BTreeSet::new();
        let mut hardest_blocked: Option<(MaterialId, &'static str)> = None;
        let air = self.default_material;
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if mat == air {
                    continue;
                }
                let aff = match self.registry.affordance(mat) {
                    Some(a) => a,
                    None => continue,
                };
                if !aff.diggable {
                    if force >= aff.hardness {
                        // Force overcomes the hardness; the blast clears it.
                    } else {
                        if let Some(reason) = aff.refusal_reason {
                            hardest_blocked = Some((mat, reason));
                        }
                        continue;
                    }
                }
                if self.set_pixel_internal(px, py, air) {
                    *counts.entry(mat).or_insert(0) += 1;
                    count += 1;
                    bbox_min[0] = bbox_min[0].min(px);
                    bbox_min[1] = bbox_min[1].min(py);
                    bbox_max[0] = bbox_max[0].max(px);
                    bbox_max[1] = bbox_max[1].max(py);
                    let (coord, _, _) = chunk_split(px, py);
                    dirty.insert(coord);
                }
            }
        }
        if count == 0 {
            if let Some((mat, reason)) = hardest_blocked {
                self.refusal_count += 1;
                return ChunkedCarveOutcome::Refused(ChunkedCarveRefusal {
                    reason,
                    probe_at: [center_px_x.round() as i64, center_px_y.round() as i64],
                    material: mat,
                });
            }
            return ChunkedCarveOutcome::NoOp(ChunkedCarveNoOp {
                probe_at: [center_px_x.round() as i64, center_px_y.round() as i64],
            });
        }
        self.carve_count += 1;
        let dominant_material = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(m, _)| *m)
            .unwrap_or(self.default_material);
        ChunkedCarveOutcome::Carved(ChunkedCarveStats {
            bbox_min,
            bbox_max,
            count,
            dominant_material,
            dirty_chunks: dirty.into_iter().collect(),
            refusal_reason: None,
        })
    }

    /// True if any pixel inside the closed AABB `min..=max` is solid (per
    /// `MaterialRegistry::is_solid`). Used by the physics integration so an
    /// actor cannot stand on top of an air column or walk through a concrete
    /// pillar.
    pub fn aabb_overlaps_solid(&self, min: [f32; 2], max: [f32; 2]) -> bool {
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        for py in y0..y1 {
            for px in x0..x1 {
                if self.registry.is_solid(self.material_at(px, py)) {
                    return true;
                }
            }
        }
        false
    }

    /// Highest solid pixel y-coordinate inside the closed pixel-x range
    /// `[x0..x1]`. `None` when the column is entirely air. Used by physics to
    /// resolve actor "stand on terrain" without per-tick contact manifold work.
    pub fn column_top_solid_y(&self, x0: i64, x1: i64, y_max: i64) -> Option<i64> {
        let mut best: Option<i64> = None;
        for px in x0..=x1 {
            for py in (0..=y_max).rev() {
                if self.registry.is_solid(self.material_at(px, py)) {
                    best = Some(best.map(|b| b.max(py)).unwrap_or(py));
                    break;
                }
            }
        }
        best
    }

    /// Iterate over the dirty chunks (consumed by render). Caller must call
    /// [`Self::clear_dirty`] after consuming.
    pub fn dirty_chunks(&self) -> impl Iterator<Item = ChunkCoord> + '_ {
        self.dirty_chunks.iter().copied()
    }

    pub fn dirty_chunk_count(&self) -> usize {
        self.dirty_chunks.len()
    }

    /// Clear the dirty set without touching pixel data.
    pub fn clear_dirty(&mut self) {
        self.dirty_chunks.clear();
    }

    /// **M3 re-open (2026-05-13) fix #6**: take + clear the per-chunk
    /// `dirty_rect`. Returns `None` when the chunk has been reclaimed
    /// (uniform default) OR when no pixel writes have landed since the last
    /// take. Caller uses this to upload only the affected sub-rect through
    /// the render bridge instead of the entire 256×256 chunk.
    pub fn take_chunk_dirty_rect(&mut self, cx: i32, cy: i32) -> Option<DirtyRect> {
        let coord = ChunkCoord::new(cx, cy);
        self.chunks.get_mut(&coord).and_then(Chunk::take_dirty_rect)
    }

    /// **M2 contract**: mark every chunk whose pixels intersect the closed
    /// pixel-space AABB as dirty. This is the canonical "I just touched the
    /// terrain at this region" path — any caller that mutates pixels via a
    /// shortcut (door stamp, fluid kernel, future repair tool that doesn't
    /// route through `try_carve` / `try_fill_or_repair` / `try_blast`) MUST
    /// call this so the renderer + AI pathfinder + replay see the edit.
    /// Per CCCP `SLTerrain.cpp:397-481` lesson: raw `set_material_pixel`
    /// without `add_updated_material_area` causes stale pathfinding —
    /// we will not repeat that mistake.
    ///
    /// The AABB is clamped to the terrain extent; out-of-bounds calls
    /// are silently no-op.
    pub fn add_updated_material_area(&mut self, min: [f32; 2], max: [f32; 2]) {
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        if x1 <= x0 || y1 <= y0 {
            return;
        }
        let cs = CHUNK_SIZE as i64;
        let cx0 = x0.div_euclid(cs);
        let cy0 = y0.div_euclid(cs);
        let cx1 = (x1 - 1).div_euclid(cs);
        let cy1 = (y1 - 1).div_euclid(cs);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                self.dirty_chunks.insert(ChunkCoord::new(cx as i32, cy as i32));
            }
        }
    }

    /// Per-chunk blake3 checksum (forward-compat for M3A determinism feed).
    /// Returns `None` for unallocated chunks; callers should treat that as
    /// "uniform default_material" and hash the default id instead.
    pub fn chunk_checksum(&self, cx: i32, cy: i32) -> Option<String> {
        let coord = ChunkCoord::new(cx, cy);
        let chunk = self.chunks.get(&coord)?;
        let mut hasher = blake3::Hasher::new();
        hasher.update(&cx.to_le_bytes());
        hasher.update(&cy.to_le_bytes());
        for p in &chunk.pixels {
            hasher.update(&p.to_le_bytes());
        }
        Some(hex::encode(hasher.finalize().as_bytes()))
    }

    /// Per-chunk material grid copy. Used by `inspect.terrain.chunk` so
    /// cfctl + AI consumers can read the exact pixel layout.
    pub fn chunk_pixels(&self, cx: i32, cy: i32) -> Vec<MaterialId> {
        let coord = ChunkCoord::new(cx, cy);
        match self.chunks.get(&coord) {
            Some(c) => c.pixels.clone(),
            None => vec![self.default_material; CHUNK_PIXELS],
        }
    }

    /// **M3 re-audit pass 4 (2026-05-13)**: read the chunk's current
    /// `dirty_rect` without taking it. Returns `None` when the chunk is
    /// unallocated OR has no pending dirty rect. Used by
    /// `inspect.terrain.chunk` so cfctl consumers can see in-flight dirt
    /// state without affecting the next render-bridge drain.
    #[must_use]
    pub fn chunk_dirty_rect(&self, cx: i32, cy: i32) -> Option<DirtyRect> {
        self.chunks.get(&ChunkCoord::new(cx, cy)).and_then(|c| c.dirty_rect)
    }

    /// **M3 re-audit pass 4 (2026-05-13)**: read the chunk's
    /// `last_modified_tick` stamp. Returns 0 for unallocated chunks (they
    /// have never been modified).
    #[must_use]
    pub fn chunk_last_modified_tick(&self, cx: i32, cy: i32) -> u64 {
        self.chunks
            .get(&ChunkCoord::new(cx, cy))
            .map(|c| c.last_modified_tick)
            .unwrap_or(0)
    }

    /// Try to fill / repair a circular region with `material`. Mirrors
    /// `try_carve` semantics: the operation refuses when the mask overlaps a
    /// refusal-reason material (e.g., metal_nohook — can't repaint over
    /// undiggable metal). Used by future repair-fill tools and the M2 spec
    /// `try_fill_or_repair` surface.
    pub fn try_fill_or_repair(&mut self, origin: [f32; 2], radius: f32, material: MaterialId) -> ChunkedCarveOutcome {
        let r = radius.max(0.0);
        let r2 = r * r;
        let min = [origin[0] - r, origin[1] - r];
        let max = [origin[0] + r, origin[1] + r];
        let (x0, y0, x1, y1) = self.aabb_to_pixels(min, max);
        let center_px_x = origin[0] - self.anchor[0];
        let center_px_y = origin[1] - self.anchor[1];
        let mut probe = [center_px_x.round() as i64, center_px_y.round() as i64];

        // Refusal probe: if the target overlaps a refusal-only material
        // (metal_nohook / anchor) we cannot overpaint without consent.
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let mat = self.material_at(px, py);
                if let Some(reason) = self.registry.refusal_reason(mat) {
                    self.refusal_count += 1;
                    return ChunkedCarveOutcome::Refused(ChunkedCarveRefusal {
                        reason,
                        probe_at: [px, py],
                        material: mat,
                    });
                }
                probe = [px, py];
            }
        }

        let mut counts: BTreeMap<MaterialId, u32> = BTreeMap::new();
        let mut count: u32 = 0;
        let mut bbox_min = [i64::MAX, i64::MAX];
        let mut bbox_max = [i64::MIN, i64::MIN];
        let mut dirty: BTreeSet<ChunkCoord> = BTreeSet::new();
        for py in y0..y1 {
            for px in x0..x1 {
                let dx = (px as f32 + 0.5) - center_px_x;
                let dy = (py as f32 + 0.5) - center_px_y;
                if dx * dx + dy * dy > r2 {
                    continue;
                }
                let prev = self.material_at(px, py);
                if self.set_pixel_internal(px, py, material) {
                    *counts.entry(prev).or_insert(0) += 1;
                    count += 1;
                    bbox_min[0] = bbox_min[0].min(px);
                    bbox_min[1] = bbox_min[1].min(py);
                    bbox_max[0] = bbox_max[0].max(px);
                    bbox_max[1] = bbox_max[1].max(py);
                    let (coord, _, _) = chunk_split(px, py);
                    dirty.insert(coord);
                }
            }
        }
        if count == 0 {
            return ChunkedCarveOutcome::NoOp(ChunkedCarveNoOp { probe_at: probe });
        }
        self.carve_count += 1;
        let dominant_source = counts
            .iter()
            .max_by_key(|(_, n)| **n)
            .map(|(m, _)| *m)
            .unwrap_or(self.default_material);
        ChunkedCarveOutcome::Carved(ChunkedCarveStats {
            bbox_min,
            bbox_max,
            count,
            dominant_material: dominant_source,
            dirty_chunks: dirty.into_iter().collect(),
            refusal_reason: None,
        })
    }

    /// True if the terrain has any allocated chunks; cheap to check before
    /// emitting an empty checksum.
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Number of allocated chunks (sparse storage proof).
    pub fn allocated_chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// **M15** § enumerate every allocated chunk's `(cx, cy)` in
    /// `(cx, cy)` ascending order. The CA stepper iterates this list
    /// per tick to apply Margolus rules.
    pub fn allocated_chunk_coords(&self) -> Vec<(i32, i32)> {
        self.chunks.keys().map(|c| (c.cx, c.cy)).collect()
    }

    /// Clone a chunk's pixel buffer for parallel snapshot-then-apply.
    pub fn chunk_pixels_clone(&self, cx: i32, cy: i32) -> Option<Vec<MaterialId>> {
        self.chunks.get(&ChunkCoord::new(cx, cy)).map(|c| c.pixels.clone())
    }

    /// Replace a chunk's pixel buffer atomically; updates dirty_rect to whole-chunk
    /// and bumps last_modified_tick. Used by the parallel CA stepper after off-chunk
    /// processing. Returns true if the chunk existed.
    pub fn replace_chunk_pixels_at_tick(
        &mut self,
        cx: i32,
        cy: i32,
        new_pixels: Vec<MaterialId>,
        tick: u64,
    ) -> bool {
        let coord = ChunkCoord::new(cx, cy);
        let chunk = match self.chunks.get_mut(&coord) {
            Some(c) => c,
            None => return false,
        };
        if new_pixels.len() != chunk.pixels.len() {
            return false;
        }
        if chunk.pixels == new_pixels {
            return false;
        }
        chunk.pixels = new_pixels;
        chunk.last_modified_tick = tick;
        chunk.dirty_rect = Some(DirtyRect {
            min: [0, 0],
            max: [CHUNK_SIZE - 1, CHUNK_SIZE - 1],
        });
        true
    }

    /// **M15** § enumerate awake chunks only (`active_region == true`)
    /// in `(cx, cy)` ascending order. Per the M15 spec § "Per-pixel
    /// cellular automata (Noita chunking)" rule: "Per-tick: only active
    /// chunks simulated (dirty regions + nearby chunks)". The CA stepper
    /// uses this when wake/sleep gating is active.
    pub fn awake_chunk_coords(&self) -> Vec<(i32, i32)> {
        self.chunks
            .iter()
            .filter(|(_, c)| c.active_region)
            .map(|(coord, _)| (coord.cx, coord.cy))
            .collect()
    }

    /// **M15** § set the `active_region` flag on a single allocated
    /// chunk. Per M15 spec § Preservation rule 4: "M3 always writes
    /// false; M15 sets true for chunks with falling materials". Returns
    /// true if the chunk existed; false if no chunk is allocated at
    /// `(cx, cy)` (the caller can pre-allocate via a `set_material_pixel`
    /// call first if needed).
    pub fn set_chunk_active_region(&mut self, cx: i32, cy: i32, value: bool) -> bool {
        let coord = ChunkCoord::new(cx, cy);
        match self.chunks.get_mut(&coord) {
            Some(chunk) => {
                chunk.active_region = value;
                true
            }
            None => false,
        }
    }

    /// **M15** § read the `active_region` flag on a chunk. Returns
    /// `false` for unallocated chunks (they are implicitly sleeping).
    #[must_use]
    pub fn chunk_active_region(&self, cx: i32, cy: i32) -> bool {
        self.chunks
            .get(&ChunkCoord::new(cx, cy))
            .map(|c| c.active_region)
            .unwrap_or(false)
    }

    /// **M15** § wake the chunk at `(cx, cy)` plus its 1-chunk-radius
    /// neighborhood (3×3 = 9 chunks total) to `active_region = true`.
    /// Per M15 spec § "active-chunk wake/sleep gating" + M8A's
    /// `wake_chunk_and_neighbors` semantics. Only updates already-
    /// allocated chunks; missing chunks are left as-is. Returns the
    /// number of chunks transitioned.
    pub fn wake_chunk_neighborhood(&mut self, cx: i32, cy: i32) -> u32 {
        let mut transitioned = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let coord = ChunkCoord::new(cx + dx, cy + dy);
                if let Some(chunk) = self.chunks.get_mut(&coord) {
                    if !chunk.active_region {
                        chunk.active_region = true;
                        transitioned += 1;
                    }
                }
            }
        }
        transitioned
    }

    /// **M15** § transition chunks that have been idle for at least
    /// `idle_threshold_ticks` to `active_region = false`. Per the M15
    /// spec § "Per Noita pattern: most of world sleeping; only chunks
    /// with falling materials wake up". Returns the chunks that just
    /// went to sleep.
    pub fn sleep_idle_chunks(&mut self, current_tick: u64, idle_threshold_ticks: u64) -> Vec<(i32, i32)> {
        let mut transitioned = Vec::new();
        for (coord, chunk) in &mut self.chunks {
            if chunk.active_region
                && current_tick.saturating_sub(chunk.last_modified_tick) >= idle_threshold_ticks
            {
                chunk.active_region = false;
                transitioned.push((coord.cx, coord.cy));
            }
        }
        transitioned
    }

    /// **M15** § set the material at world-space pixel `(px, py)` with
    /// an explicit tick stamp. Routes through the canonical
    /// `set_pixel_at_tick` path so the per-chunk `last_modified_tick`,
    /// `dirty_rect`, and `dirty_chunks` set all stay coherent. Per the
    /// M3 preservation rules (M15 spec § "Preservation rules from M3"
    /// rule 1) every CA / reaction / phase-change pixel write MUST go
    /// through this entry point (not the lower-level chunk APIs).
    /// Returns true if the pixel changed.
    pub fn set_material_pixel(&mut self, px: i64, py: i64, mat: MaterialId, tick: u64) -> bool {
        let prev_tick = self.current_tick;
        self.current_tick = tick;
        let changed = self.set_pixel_internal(px, py, mat);
        self.current_tick = prev_tick;
        changed
    }

    /// **M3 audit pass 5 (2026-05-13)**: per-chunk blake3 hex summaries for
    /// every allocated chunk. Used by the engine to populate the
    /// `chunk_summary` field on `determinism.sim_checksum` payloads per
    /// spec literal "And it appears in the determinism.sim_checksum
    /// payload's chunk-summary field". Returns `(cx, cy, blake3_hex)`
    /// triples ordered by (cx, cy) for deterministic JSON output.
    ///
    /// **Performance**: parallelized via rayon's `par_iter` over the chunk
    /// map's entries. Per-chunk blake3 hashing is CPU-bound + independent
    /// (no shared writes), so this scales linearly with core count. The
    /// final collect() preserves BTreeMap iteration order so the output
    /// is byte-identical to the serial path (determinism per DR-052).
    pub fn chunk_summary_entries(&self) -> Vec<(i32, i32, String)> {
        use rayon::iter::{IntoParallelIterator, ParallelIterator};
        // Snapshot ordered entries first so par_iter operates on a
        // canonical-ordered slice; final result preserves (cx, cy)
        // ascending order without needing a post-sort.
        let entries: Vec<(&ChunkCoord, &Chunk)> = self.chunks.iter().collect();
        entries
            .into_par_iter()
            .map(|(coord, chunk)| {
                let mut hasher = blake3::Hasher::new();
                hasher.update(&coord.cx.to_le_bytes());
                hasher.update(&coord.cy.to_le_bytes());
                for p in &chunk.pixels {
                    hasher.update(&p.to_le_bytes());
                }
                (coord.cx, coord.cy, hex::encode(hasher.finalize().as_bytes()))
            })
            .collect()
    }

    /// Per-material pixel counts across every allocated chunk + the implicit
    /// default-material area. Used for snapshot + summary stats.
    pub fn material_counts(&self) -> BTreeMap<String, u64> {
        let mut counts: BTreeMap<String, u64> = BTreeMap::new();
        // Total covered pixels.
        let total = (self.width_px as u64) * (self.height_px as u64);
        // Pixels in allocated chunks: sum each chunk's pixel array.
        let mut allocated_pixels: u64 = 0;
        for (coord, chunk) in &self.chunks {
            // Skip chunks fully outside the terrain extent.
            let origin = coord.pixel_origin();
            if origin[0] >= self.width_px as i64 || origin[1] >= self.height_px as i64 {
                continue;
            }
            for ly in 0..CHUNK_SIZE {
                for lx in 0..CHUNK_SIZE {
                    let px = origin[0] + lx as i64;
                    let py = origin[1] + ly as i64;
                    if !self.in_bounds(px, py) {
                        continue;
                    }
                    let mat = chunk.pixel(lx, ly);
                    let name = self.registry.name(mat);
                    *counts.entry(name.to_string()).or_insert(0) += 1;
                    allocated_pixels += 1;
                }
            }
        }
        // Pixels NOT in allocated chunks default to `default_material`.
        let default_name = self.registry.name(self.default_material);
        let default_pixels = total.saturating_sub(allocated_pixels);
        if default_pixels > 0 {
            *counts.entry(default_name.to_string()).or_insert(0) += default_pixels;
        }
        counts
    }

    /// Layout-stable bytes used by the determinism checksum. M2 appends after
    /// `cf-actor::sim::ActorSimState::checksum_bytes()` which appends after the
    /// M0 `tick_counter || rng_state` prefix; the layout is `(M0 prefix) ||
    /// (M1 actor bytes) || (M2 terrain bytes)`. M3+ snapshots hash this
    /// directly.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(64 + self.chunks.len() * 32);
        out.extend_from_slice(b"cf-terrain-chunked-v2");
        out.extend_from_slice(&self.width_px.to_le_bytes());
        out.extend_from_slice(&self.height_px.to_le_bytes());
        out.extend_from_slice(&self.default_material.to_le_bytes());
        out.extend_from_slice(&self.carve_count.to_le_bytes());
        out.extend_from_slice(&(self.chunks.len() as u32).to_le_bytes());
        let mut chunk_hasher = blake3::Hasher::new();
        for (coord, chunk) in &self.chunks {
            chunk_hasher.update(&coord.cx.to_le_bytes());
            chunk_hasher.update(&coord.cy.to_le_bytes());
            for p in &chunk.pixels {
                chunk_hasher.update(&p.to_le_bytes());
            }
        }
        out.extend_from_slice(chunk_hasher.finalize().as_bytes());
        out
    }

    /// Full snapshot, suitable for the M3A `snapshot.snapshot_terrain_chunk`
    /// event payload. JSON-serializable. Only allocated chunks are emitted;
    /// the default material covers the rest.
    pub fn snapshot(&self) -> ChunkedTerrainSnapshot {
        let chunks = self
            .chunks
            .iter()
            .map(|(coord, chunk)| ChunkedTerrainSnapshotChunk {
                coord: *coord,
                pixels: chunk.pixels.clone(),
            })
            .collect();
        ChunkedTerrainSnapshot {
            schema: MATERIAL_SCHEMA_VERSION.to_string(),
            width_px: self.width_px,
            height_px: self.height_px,
            anchor: self.anchor,
            default_material: self.default_material,
            carve_count: self.carve_count,
            refusal_count: self.refusal_count,
            material_counts: self.material_counts(),
            chunks,
        }
    }

    /// **M9 § Destructible terrain — 5-tier per-pixel integrity**: read
    /// the integrity at `(px, py)`. Returns `1.0` (Pristine) when the
    /// pixel has no metadata entry — Pristine pixels are implicit. Air
    /// pixels also return `1.0` so callers don't need to special-case them
    /// before applying damage (the `try_penetrate_pixel` path filters air
    /// itself).
    #[must_use]
    pub fn pixel_integrity(&self, px: i64, py: i64) -> f32 {
        if !self.in_bounds(px, py) {
            return 1.0;
        }
        let (coord, lx, ly) = chunk_split(px, py);
        match self.pixel_meta_grid.get(&PixelMetaKey::new(coord.cx, coord.cy, lx, ly)) {
            Some(meta) => meta.integrity,
            None => 1.0,
        }
    }

    /// **M9 § Destructible terrain — 5-tier per-pixel integrity**: derive
    /// the band at `(px, py)`. Untouched pixels report Pristine.
    #[must_use]
    pub fn pixel_band(&self, px: i64, py: i64) -> IntegrityBand {
        IntegrityBand::from_integrity(self.pixel_integrity(px, py))
    }

    /// **M9 § Destructible terrain — per-pixel integrity damage**.
    ///
    /// Apply `impact_energy` damage to a single pixel under the per-material
    /// hardness curve (`damage = impact_energy * (1 - hardness) / hardness`,
    /// clamped to `[0, 1]`). Tracks integrity in `pixel_meta_grid` and
    /// emits a `PenetrationOutcome` describing band crossings, pixel
    /// removal, and any cascade decay applied to direct neighbors when
    /// the pixel reaches Destroyed.
    ///
    /// Caller is responsible for emitting `terrain.material_state_changed`
    /// plus `terrain.pixel_removed` plus `terrain.cascade_triggered` based
    /// on the outcome. cf-control's projectile-vs-terrain handler does this
    /// inside its event loop with the right `parent_event_id` chain.
    ///
    /// Returns `None` when the pixel is out of bounds OR air — air pixels
    /// receive no damage state.
    pub fn try_penetrate_pixel(
        &mut self,
        px: i64,
        py: i64,
        impact_energy: f32,
        cause: DamageKind,
        damage_source: Option<String>,
    ) -> Option<PenetrationOutcome> {
        if !self.in_bounds(px, py) {
            return None;
        }
        let mat = self.material_at(px, py);
        if mat == self.default_material || mat == MATERIAL_AIR {
            return None;
        }
        let aff = self.registry.affordance(mat)?;
        let (coord, lx, ly) = chunk_split(px, py);
        let key = PixelMetaKey::new(coord.cx, coord.cy, lx, ly);
        let integrity_before = self.pixel_meta_grid.get(&key).map(|m| m.integrity).unwrap_or(1.0);
        let band_before = IntegrityBand::from_integrity(integrity_before);
        let hardness = normalized_hardness(mat);
        let integrity_after = apply_damage_formula(integrity_before, impact_energy, hardness);
        let band_after = IntegrityBand::from_integrity(integrity_after);
        let band_crossed = band_before != band_after;
        let destroyed = integrity_after <= 0.0;
        if destroyed {
            // Clear damage state + remove pixel from the world. Cascade
            // decay applies to surviving neighbors only — never to air.
            self.pixel_meta_grid.remove(&key);
            self.set_pixel_internal(px, py, self.default_material);
        } else {
            self.pixel_meta_grid.insert(
                key,
                PixelMeta {
                    integrity: integrity_after,
                    last_damage_tick: self.current_tick,
                    damage_kind: cause,
                    damage_source: damage_source.clone(),
                },
            );
        }
        let cascades = if destroyed {
            self.apply_cascade_decay(px, py, damage_source.as_deref())
        } else {
            Vec::new()
        };
        Some(PenetrationOutcome {
            pos: [px, py],
            material_id: mat,
            material_name: aff.name,
            integrity_before,
            integrity_after,
            band_before,
            band_after,
            band_crossed,
            destroyed,
            cascades,
        })
    }

    /// **M9 § Cascade rule** for digger / blast carves: after a multi-pixel
    /// carve, walk the perimeter of the destroyed bbox and apply cascade
    /// decay to any solid pixel adjacent to a freshly-cleared pixel whose
    /// normalized hardness is at or below the cascade threshold. Returns
    /// one `CascadeEvent` per affected neighbor (caller emits
    /// `terrain.cascade_triggered` with the right `parent_event_id`).
    ///
    /// `bbox_min` / `bbox_max` are inclusive pixel-space bounds (matching
    /// `ChunkedCarveStats`). `source` annotates the cascade source for the
    /// cause chain.
    #[must_use]
    pub fn apply_cascade_to_carve_perimeter(
        &mut self,
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        source: Option<&str>,
    ) -> Vec<CascadeEvent> {
        let mut events: Vec<CascadeEvent> = Vec::new();
        let mut seen: BTreeSet<(i64, i64)> = BTreeSet::new();
        // Walk a 1-pixel halo around the destroyed bbox. Any air pixel
        // inside the carve volume is a candidate "destroyed source"; any
        // solid pixel adjacent to it is a candidate "affected neighbor".
        for py in (bbox_min[1] - 1)..=(bbox_max[1] + 1) {
            for px in (bbox_min[0] - 1)..=(bbox_max[0] + 1) {
                if !self.in_bounds(px, py) {
                    continue;
                }
                let mat = self.material_at(px, py);
                if mat != self.default_material && mat != MATERIAL_AIR {
                    continue;
                }
                // Cleared pixel inside or at the edge of the carve. Check
                // its 4-neighbors for cascade candidates.
                for (dx, dy) in [(-1_i64, 0_i64), (1, 0), (0, -1), (0, 1)] {
                    let nx = px + dx;
                    let ny = py + dy;
                    if !self.in_bounds(nx, ny) {
                        continue;
                    }
                    if !seen.insert((nx, ny)) {
                        continue;
                    }
                    if let Some(ev) = self.cascade_pixel(nx, ny, [px, py], source) {
                        events.push(ev);
                    }
                }
            }
        }
        events
    }

    /// **M9 § Cascade rule**: when a pixel reaches Destroyed, decay direct
    /// 4-neighbors whose normalized hardness is at or below
    /// `cascade_threshold` (default 0.6). Each affected neighbor loses
    /// `DEFAULT_CASCADE_DECAY_PCT` (default 0.1) integrity, clamped to
    /// `[0, 1]`. Neighbors that reach 0 under cascade are removed in turn
    /// (still capped to `cascade_depth=1` — we do NOT recurse).
    ///
    /// Returns the cascade events so the engine can emit
    /// `terrain.cascade_triggered` (one per affected neighbor) and
    /// `terrain.pixel_removed` (when cascade kills the neighbor).
    fn apply_cascade_decay(&mut self, source_x: i64, source_y: i64, source: Option<&str>) -> Vec<CascadeEvent> {
        let mut events = Vec::with_capacity(4);
        for (dx, dy) in [(-1_i64, 0_i64), (1, 0), (0, -1), (0, 1)] {
            let nx = source_x + dx;
            let ny = source_y + dy;
            if let Some(ev) = self.cascade_pixel(nx, ny, [source_x, source_y], source) {
                events.push(ev);
            }
        }
        events
    }

    /// Apply cascade decay to a single candidate neighbor pixel from
    /// `from_pos`. Returns the event when decay landed; `None` when:
    ///   - out of bounds
    ///   - neighbor is air / default material
    ///   - neighbor hardness > cascade threshold
    ///   - neighbor integrity unchanged (already at 0)
    fn cascade_pixel(&mut self, nx: i64, ny: i64, from_pos: [i64; 2], source: Option<&str>) -> Option<CascadeEvent> {
        if !self.in_bounds(nx, ny) {
            return None;
        }
        let nmat = self.material_at(nx, ny);
        if nmat == self.default_material || nmat == MATERIAL_AIR {
            return None;
        }
        let nhardness = normalized_hardness(nmat);
        if nhardness > DEFAULT_CASCADE_THRESHOLD {
            return None;
        }
        let (ncoord, nlx, nly) = chunk_split(nx, ny);
        let nkey = PixelMetaKey::new(ncoord.cx, ncoord.cy, nlx, nly);
        let integrity_before = self.pixel_meta_grid.get(&nkey).map(|m| m.integrity).unwrap_or(1.0);
        let integrity_after = (integrity_before - DEFAULT_CASCADE_DECAY_PCT).clamp(0.0, 1.0);
        if (integrity_after - integrity_before).abs() < f32::EPSILON {
            return None;
        }
        let band_before = IntegrityBand::from_integrity(integrity_before);
        let band_after = IntegrityBand::from_integrity(integrity_after);
        let destroyed_neighbor = integrity_after <= 0.0;
        if destroyed_neighbor {
            self.pixel_meta_grid.remove(&nkey);
            self.set_pixel_internal(nx, ny, self.default_material);
        } else {
            self.pixel_meta_grid.insert(
                nkey,
                PixelMeta {
                    integrity: integrity_after,
                    last_damage_tick: self.current_tick,
                    damage_kind: DamageKind::NeighborDestroyed,
                    damage_source: source.map(str::to_owned),
                },
            );
        }
        let nname = self.registry.affordance(nmat).map(|a| a.name).unwrap_or("unknown");
        Some(CascadeEvent {
            from_pos,
            to_pos: [nx, ny],
            material_id: nmat,
            material_name: nname,
            integrity_before,
            integrity_after,
            from_band: band_before,
            to_band: band_after,
            destroyed_neighbor,
            depth: DEFAULT_CASCADE_DEPTH,
            threshold: DEFAULT_CASCADE_THRESHOLD,
        })
    }

    /// Reverse of [`Self::snapshot`]: rebuild a terrain from a snapshot. Used
    /// by the cf-headless replay verifier so a replayed bundle's per-tick
    /// state matches the live run pixel-for-pixel.
    pub fn from_snapshot(snapshot: &ChunkedTerrainSnapshot) -> Self {
        let mut t = Self::new(snapshot.width_px, snapshot.height_px, snapshot.default_material);
        t.anchor = snapshot.anchor;
        t.carve_count = snapshot.carve_count;
        t.refusal_count = snapshot.refusal_count;
        for c in &snapshot.chunks {
            if c.pixels.len() != CHUNK_PIXELS {
                tracing::warn!(
                    target: "cf::terrain",
                    coord = ?c.coord,
                    expected = CHUNK_PIXELS,
                    actual = c.pixels.len(),
                    "from_snapshot: chunk pixel array has wrong length; skipping"
                );
                continue;
            }
            t.chunks.insert(
                c.coord,
                Chunk {
                    pixels: c.pixels.clone(),
                    active_region: false,
                    last_modified_tick: 0,
                    color_grid: None,
                    dirty_rect: None,
                },
            );
        }
        t
    }

    /// Reset every chunk to the default material. Used by `scenario.reset` so
    /// the engine can rewind the terrain without rebuilding from disk.
    pub fn reset_to_default(&mut self) {
        self.chunks.clear();
        self.dirty_chunks.clear();
        self.carve_count = 0;
        self.refusal_count = 0;
        self.pixel_meta_grid.clear();
    }

    fn aabb_to_pixels(&self, min: [f32; 2], max: [f32; 2]) -> (i64, i64, i64, i64) {
        let x0 = ((min[0] - self.anchor[0]).floor() as i64).max(0);
        let y0 = ((min[1] - self.anchor[1]).floor() as i64).max(0);
        let x1 = ((max[0] - self.anchor[0]).ceil() as i64)
            .max(0)
            .min(self.width_px as i64);
        let y1 = ((max[1] - self.anchor[1]).ceil() as i64)
            .max(0)
            .min(self.height_px as i64);
        (x0, y0, x1.max(x0), y1.max(y0))
    }

    fn in_bounds(&self, px: i64, py: i64) -> bool {
        // Devin BUG_pr-review-job (yellow): compare in i64 space so a
        // pathological caller passing px >= 2^32 doesn't truncate-wrap to a
        // small u32 and falsely report "in bounds". All current call sites
        // derive (px, py) from f32 world coords (max ~16M) or
        // `aabb_to_pixels` which clamps to the terrain extent, so this is
        // defensive — but the contract is now branch-truthful regardless of
        // the caller's coordinate source.
        px >= 0 && py >= 0 && px < (self.width_px as i64) && py < (self.height_px as i64)
    }

    fn set_pixel_internal(&mut self, px: i64, py: i64, mat: MaterialId) -> bool {
        if !self.in_bounds(px, py) {
            return false;
        }
        let (coord, lx, ly) = chunk_split(px, py);
        let entry = self
            .chunks
            .entry(coord)
            .or_insert_with(|| Chunk::uniform(self.default_material));
        // M3 re-audit pass 4 (2026-05-13): route through `set_pixel_at_tick`
        // so the chunk's `last_modified_tick` stamp tracks the engine's
        // current tick. `inspect.terrain.chunk` reads this stamp.
        let changed = entry.set_pixel_at_tick(lx, ly, mat, self.current_tick);
        if changed {
            self.dirty_chunks.insert(coord);
            // Reclaim chunks that fully match the default to keep storage sparse.
            if entry.is_uniform(self.default_material) {
                self.chunks.remove(&coord);
            }
        }
        changed
    }
}

/// Split a pixel coordinate into (chunk coord, local x, local y).
fn chunk_split(px: i64, py: i64) -> (ChunkCoord, u32, u32) {
    let cs = CHUNK_SIZE as i64;
    let cx = px.div_euclid(cs) as i32;
    let cy = py.div_euclid(cs) as i32;
    let lx = px.rem_euclid(cs) as u32;
    let ly = py.rem_euclid(cs) as u32;
    (ChunkCoord::new(cx, cy), lx, ly)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_world() -> ChunkedTerrain {
        let mut t = ChunkedTerrain::new(512, 256, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [512.0, 16.0], MATERIAL_CONCRETE);
        t.fill_aabb([200.0, 16.0], [400.0, 96.0], MATERIAL_DIRT);
        t.fill_aabb([460.0, 16.0], [492.0, 96.0], MATERIAL_METAL_NOHOOK);
        t
    }

    /// VAL-M14E-010: support_beam material has id=8, integrity (hardness) = 200,
    /// anchorable=true, piling=false. The piling=false invariant is asserted
    /// by cf-material's MaterialDef accessor; here we verify the cf-terrain
    /// affordance row.
    #[test]
    fn support_beam_affordance_matches_m14e_spec_table() {
        let aff =
            material_affordance(MATERIAL_SUPPORT_BEAM).expect("support_beam registered");
        assert_eq!(aff.id, MATERIAL_SUPPORT_BEAM);
        assert_eq!(aff.id, 8);
        assert_eq!(aff.name, "support_beam");
        assert!((aff.hardness - 200.0).abs() < 1e-3);
        assert!(aff.anchorable);
        assert!(aff.solid);
        assert!(aff.diggable, "support_beam must be diggable so demolish works");
    }

    #[test]
    fn material_id_from_name_covers_launch_set() {
        for name in [
            "air",
            "dirt",
            "concrete",
            "concrete_soft",
            "metal_nohook",
            "hazard",
            "loose_fill",
            "repair_fill",
            "anchor",
        ] {
            assert!(material_id_from_name(name).is_some(), "{name}");
        }
        assert!(material_id_from_name("granite_unknown").is_none());
    }

    #[test]
    fn fill_aabb_writes_dense_pixels() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let written = t.fill_aabb([0.0, 0.0], [10.0, 10.0], MATERIAL_DIRT);
        assert_eq!(written, 100);
        assert_eq!(t.material_at(0, 0), MATERIAL_DIRT);
        assert_eq!(t.material_at(9, 9), MATERIAL_DIRT);
        assert_eq!(t.material_at(10, 10), MATERIAL_AIR);
    }

    #[test]
    fn fill_circle_writes_radius_pixels() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let written = t.fill_circle([5.0, 5.0], 3.0, MATERIAL_DIRT);
        assert!(written > 0 && written < 64);
        // (5,5) is the centre — must be filled.
        assert_eq!(t.material_at(5, 5), MATERIAL_DIRT);
        // (15,15) is well outside — still air.
        assert_eq!(t.material_at(15, 15), MATERIAL_AIR);
    }

    #[test]
    fn try_carve_into_dirt_succeeds() {
        let mut t = small_world();
        let outcome = t.try_carve([300.0, 60.0], 8.0);
        match outcome {
            ChunkedCarveOutcome::Carved(stats) => {
                assert!(stats.count > 0);
                assert_eq!(stats.dominant_material, MATERIAL_DIRT);
                assert!(!stats.dirty_chunks.is_empty());
            }
            other => panic!("expected Carved, got {other:?}"),
        }
        assert_eq!(t.carve_count, 1);
    }

    #[test]
    fn try_carve_into_metal_refuses() {
        let mut t = small_world();
        let outcome = t.try_carve([476.0, 60.0], 8.0);
        match outcome {
            ChunkedCarveOutcome::Refused(refusal) => {
                assert_eq!(refusal.reason, "material_not_diggable");
                assert_eq!(refusal.material, MATERIAL_METAL_NOHOOK);
            }
            other => panic!("expected Refused, got {other:?}"),
        }
        assert_eq!(t.refusal_count, 1);
    }

    #[test]
    fn try_carve_against_air_is_noop() {
        let mut t = small_world();
        let outcome = t.try_carve([100.0, 200.0], 4.0);
        assert!(matches!(outcome, ChunkedCarveOutcome::NoOp(_)));
        assert_eq!(t.carve_count, 0);
    }

    #[test]
    fn try_carve_increments_carve_count_only_on_success() {
        let mut t = small_world();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        let _ = t.try_carve([320.0, 60.0], 8.0);
        let _ = t.try_carve([476.0, 60.0], 8.0); // refused
        assert_eq!(t.carve_count, 2);
        assert_eq!(t.refusal_count, 1);
    }

    #[test]
    fn aabb_overlaps_solid_detects_floor() {
        let t = small_world();
        assert!(t.aabb_overlaps_solid([0.0, 0.0], [10.0, 16.0]));
        assert!(!t.aabb_overlaps_solid([0.0, 100.0], [10.0, 110.0]));
    }

    #[test]
    fn column_top_solid_y_finds_floor_height() {
        let t = small_world();
        let top = t.column_top_solid_y(50, 51, 200).unwrap();
        // Floor occupies [0..16) so the top solid pixel y is 15.
        assert_eq!(top, 15);
    }

    #[test]
    fn try_carve_breaks_through_full_width_with_repeated_calls() {
        let mut t = small_world();
        let mut carved_total: u32 = 0;
        for _ in 0..6 {
            if let ChunkedCarveOutcome::Carved(stats) = t.try_carve([300.0, 60.0], 8.0) {
                carved_total += stats.count;
            }
        }
        assert!(carved_total > 0);
        // Eventually the carve circle hits all-air and turns into a no-op.
        let _ = t.try_carve([300.0, 60.0], 8.0);
    }

    #[test]
    fn dirty_chunks_track_carves_until_cleared() {
        let mut t = small_world();
        t.clear_dirty();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        assert!(t.dirty_chunk_count() > 0);
        t.clear_dirty();
        assert_eq!(t.dirty_chunk_count(), 0);
    }

    #[test]
    fn checksum_changes_when_terrain_changes() {
        let mut t = small_world();
        let before = t.checksum_bytes();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        let after = t.checksum_bytes();
        assert_ne!(before, after);
    }

    #[test]
    fn snapshot_round_trip_preserves_pixel_values() {
        let mut t = small_world();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        let snap = t.snapshot();
        let restored = ChunkedTerrain::from_snapshot(&snap);
        assert_eq!(restored.checksum_bytes(), t.checksum_bytes());
        assert_eq!(restored.material_at(300, 60), MATERIAL_AIR);
    }

    #[test]
    fn reset_clears_chunks_and_counters() {
        let mut t = small_world();
        let _ = t.try_carve([300.0, 60.0], 8.0);
        t.reset_to_default();
        assert_eq!(t.allocated_chunk_count(), 0);
        assert_eq!(t.carve_count, 0);
        assert_eq!(t.refusal_count, 0);
        assert_eq!(t.material_at(300, 60), MATERIAL_AIR);
    }

    #[test]
    fn material_counts_balances_total_pixels() {
        let t = small_world();
        let counts = t.material_counts();
        let total: u64 = counts.values().sum();
        assert_eq!(total, 512 * 256);
    }

    #[test]
    fn try_blast_clears_circle_through_diggable() {
        let mut t = small_world();
        let outcome = t.try_blast([300.0, 60.0], 16.0, 100.0);
        match outcome {
            ChunkedCarveOutcome::Carved(stats) => {
                assert!(stats.count > 0);
            }
            other => panic!("expected Carved, got {other:?}"),
        }
    }

    #[test]
    fn try_blast_refuses_when_force_below_metal_hardness() {
        let mut t = small_world();
        let outcome = t.try_blast([476.0, 60.0], 16.0, 1.0);
        assert!(matches!(outcome, ChunkedCarveOutcome::Refused(_)));
    }

    #[test]
    fn try_blast_against_hazard_obeys_spec_hardness_gate() {
        // M2 spec: hazard.hardness=50. Blasts below 50 refuse; blasts at or
        // above 50 clear (M5.6 active material kernel will later add a
        // dispersal/reaction path; M2 ships the basic force-gate symmetry
        // with `try_blast` against any blastable material).
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_HAZARD);
        // Force below hardness must refuse.
        let outcome = t.try_blast([16.0, 16.0], 8.0, 10.0);
        assert!(
            matches!(outcome, ChunkedCarveOutcome::Refused(_)),
            "expected hazard to refuse blast with force 10, got {outcome:?}"
        );
        // Force at hardness clears.
        let outcome = t.try_blast([16.0, 16.0], 8.0, 50.0);
        assert!(
            matches!(outcome, ChunkedCarveOutcome::Carved(_)),
            "expected hazard to yield to blast with force 50, got {outcome:?}"
        );
    }

    #[test]
    fn try_fill_or_repair_paints_into_air() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let outcome = t.try_fill_or_repair([32.0, 32.0], 6.0, MATERIAL_REPAIR_FILL);
        assert!(matches!(outcome, ChunkedCarveOutcome::Carved(_)));
        assert_eq!(t.material_at(32, 32), MATERIAL_REPAIR_FILL);
    }

    #[test]
    fn try_fill_or_repair_refuses_over_metal_nohook() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_METAL_NOHOOK);
        let outcome = t.try_fill_or_repair([16.0, 16.0], 6.0, MATERIAL_REPAIR_FILL);
        match outcome {
            ChunkedCarveOutcome::Refused(refusal) => {
                assert_eq!(refusal.reason, "material_not_diggable");
            }
            other => panic!("expected refusal, got {other:?}"),
        }
    }

    #[test]
    fn add_updated_material_area_marks_chunks_dirty() {
        let mut t = ChunkedTerrain::new(1024, 512, MATERIAL_AIR);
        t.clear_dirty();
        t.add_updated_material_area([100.0, 100.0], [200.0, 200.0]);
        assert!(t.dirty_chunk_count() > 0);
    }

    #[test]
    fn chunk_checksum_changes_with_pixel_edit() {
        let mut t = small_world();
        let before = t.chunk_checksum(1, 0);
        let _ = t.try_carve([300.0, 60.0], 8.0);
        let after = t.chunk_checksum(1, 0);
        assert!(before.is_some());
        assert!(after.is_some());
        assert_ne!(before, after);
    }

    #[test]
    fn val_m15b_material_affordances_cover_active_set() {
        // M15B added 12 active-material affordances (water, oil, acid,
        // lava, iron, co2, steam, smoke, fire_intense, cloud, rain,
        // acid_droplet). Without these, the renderer treats them as
        // transparent black + physics treats them as air.
        for (id, expected_name) in [
            (13u16, "water"),
            (19, "oil"),
            (21, "acid"),
            (26, "lava"),
            (43, "co2"),
            (50, "steam"),
            (62, "smoke"),
            (65, "fire_intense"),
            (68, "iron"),
            (71, "cloud"),
            (87, "rain"),
            (88, "acid_droplet"),
        ] {
            let aff = material_affordance(id)
                .unwrap_or_else(|| panic!("M15 material id={id} ({expected_name}) missing affordance"));
            assert_eq!(aff.name, expected_name, "id={id}");
        }
    }

    #[test]
    fn val_m15b_hazardous_materials_damage_actors() {
        // Per M15 chem doc: acid, lava, fire_intense, acid_droplet all
        // emit per-tick damage to actors in contact.
        for (id, min_dpt) in [(21u16, 1.0_f32), (26, 5.0), (65, 5.0), (88, 1.0)] {
            let aff = material_affordance(id).unwrap();
            assert!(aff.hazard, "id={id} should be hazard");
            assert!(
                aff.damage_per_tick >= min_dpt,
                "id={id} damage_per_tick {} < {min_dpt}",
                aff.damage_per_tick
            );
        }
    }

    #[test]
    fn dirt_to_concrete_hardness_ratio_matches_spec() {
        // Spec: concrete carves in 5-10x dirt time (hardness=50 vs hardness=10).
        // M3 audit pass 7 (2026-05-13): concrete bumped to 50 so the ratio
        // hits the 5x lower bound exactly.
        let dirt = material_affordance(MATERIAL_DIRT).unwrap();
        let concrete = material_affordance(MATERIAL_CONCRETE).unwrap();
        assert!((dirt.hardness - 10.0).abs() < f32::EPSILON);
        assert!((concrete.hardness - 50.0).abs() < f32::EPSILON);
        assert!(concrete.hardness >= 5.0 * dirt.hardness);
        assert!(concrete.hardness <= 10.0 * dirt.hardness);
    }

    #[test]
    fn launch_set_baseline_hardness_matches_spec() {
        for (id, expected) in [
            (MATERIAL_AIR, 0.0_f32),
            (MATERIAL_DIRT, 10.0),
            (MATERIAL_LOOSE_FILL, 5.0),
            // M3 audit pass 7 (2026-05-13): concrete bumped to 50 so the
            // 5x dirt-ratio spec floor is satisfied exactly.
            (MATERIAL_CONCRETE, 50.0),
            (MATERIAL_METAL_NOHOOK, 100.0),
            (MATERIAL_ANCHOR, 60.0),
            (MATERIAL_HAZARD, 50.0),
            (MATERIAL_REPAIR_FILL, 15.0),
        ] {
            let aff = material_affordance(id).unwrap_or_else(|| panic!("id {id} present"));
            assert!(
                (aff.hardness - expected).abs() < 1e-3,
                "{} hardness expected {} got {}",
                aff.name,
                expected,
                aff.hardness
            );
        }
    }

    #[test]
    fn in_bounds_rejects_extreme_i64_coordinates() {
        // Devin BUG_pr-review-job (yellow) regression: `in_bounds` cast
        // `px as u32` which truncates for px >= 2^32 (e.g., 4294967296
        // truncates to 0 and would be reported in-bounds). The fix
        // compares in i64 space.
        let t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        // Inside bounds.
        assert!(t.in_bounds(0, 0));
        assert!(t.in_bounds(63, 63));
        // Outside bounds — truncation-prone values.
        assert!(!t.in_bounds(-1, 0));
        assert!(!t.in_bounds(64, 0));
        assert!(!t.in_bounds(64_000_000, 0));
        // Values that would truncate-wrap on a u32 cast — must remain out.
        assert!(!t.in_bounds(1_i64 << 32, 0));
        assert!(!t.in_bounds(0, 1_i64 << 33));
        assert!(!t.in_bounds(i64::MAX, 0));
    }

    #[test]
    fn fill_aabb_clamped_to_terrain_extent() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let written = t.fill_aabb([-10.0, -10.0], [200.0, 200.0], MATERIAL_DIRT);
        assert_eq!(written, 64 * 64);
    }

    #[test]
    fn chunk_uniformity_compresses_storage() {
        let mut t = ChunkedTerrain::new(512, 512, MATERIAL_AIR);
        // Fill an entire chunk with dirt, then carve it back to air.
        t.fill_aabb([0.0, 0.0], [256.0, 256.0], MATERIAL_DIRT);
        assert_eq!(t.allocated_chunk_count(), 1);
        // Carving reverts each pixel back to air; once the chunk is uniform
        // again, storage is reclaimed.
        for y in 0..256 {
            for x in 0..256 {
                t.set_pixel_internal(x, y, MATERIAL_AIR);
            }
        }
        assert_eq!(t.allocated_chunk_count(), 0);
    }

    // -- M9 § Destructible terrain — per-pixel integrity tests --

    #[test]
    fn pixel_integrity_starts_pristine_for_untouched_pixels() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        // No prior damage — pristine 1.0.
        assert!((t.pixel_integrity(10, 10) - 1.0).abs() < f32::EPSILON);
        assert_eq!(t.pixel_band(10, 10), IntegrityBand::Pristine);
    }

    #[test]
    fn try_penetrate_pixel_dirt_light_hit_drops_to_scratched() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        let outcome = t
            .try_penetrate_pixel(10, 10, 0.05, DamageKind::ProjectileHit, None)
            .expect("dirt pixel exists");
        assert_eq!(outcome.material_id, MATERIAL_DIRT);
        assert_eq!(outcome.band_before, IntegrityBand::Pristine);
        // 0.05 * (1 - 0.2) / 0.2 = 0.2 → integrity 0.8 → still Pristine.
        // Use a larger impact to land in Scratched.
        let outcome2 = t
            .try_penetrate_pixel(10, 10, 0.05, DamageKind::ProjectileHit, None)
            .expect("dirt pixel exists");
        assert_eq!(outcome2.band_after, IntegrityBand::Scratched);
        assert!(outcome2.band_crossed);
        assert!(!outcome2.destroyed);
    }

    #[test]
    fn try_penetrate_pixel_sand_hit_destroys_immediately() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_LOOSE_FILL);
        let outcome = t
            .try_penetrate_pixel(10, 10, 0.5, DamageKind::ProjectileHit, None)
            .expect("sand pixel exists");
        assert!(outcome.destroyed);
        assert_eq!(outcome.band_after, IntegrityBand::Destroyed);
        // Pixel removed from the world.
        assert_eq!(t.material_at(10, 10), MATERIAL_AIR);
    }

    #[test]
    fn try_penetrate_pixel_metal_resists_high_impact() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_METAL_NOHOOK);
        let outcome = t
            .try_penetrate_pixel(10, 10, 0.5, DamageKind::ProjectileHit, None)
            .expect("metal pixel exists");
        assert!(!outcome.destroyed);
        // (1.0 - 0.5 * 0.1 / 0.9) ≈ 0.944
        assert!(
            (outcome.integrity_after - 0.944).abs() < 0.02,
            "metal integrity after one hit at impact=0.5 should be ~0.94, got {}",
            outcome.integrity_after
        );
        assert_eq!(outcome.band_after, IntegrityBand::Pristine);
    }

    #[test]
    fn try_penetrate_pixel_against_air_returns_none() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        let outcome = t.try_penetrate_pixel(10, 10, 0.5, DamageKind::ProjectileHit, None);
        assert!(outcome.is_none());
    }

    #[test]
    fn try_penetrate_pixel_progresses_through_all_5_bands() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        let mut bands_observed: Vec<IntegrityBand> = vec![IntegrityBand::Pristine];
        for _ in 0..40 {
            if let Some(outcome) = t.try_penetrate_pixel(10, 10, 0.05, DamageKind::ProjectileHit, None) {
                if outcome.band_crossed {
                    bands_observed.push(outcome.band_after);
                }
                if outcome.destroyed {
                    break;
                }
            } else {
                break;
            }
        }
        // Must have observed every band on the path Pristine → Destroyed.
        assert!(bands_observed.contains(&IntegrityBand::Pristine));
        assert!(bands_observed.contains(&IntegrityBand::Scratched));
        assert!(bands_observed.contains(&IntegrityBand::Cracked));
        assert!(bands_observed.contains(&IntegrityBand::Critical));
        assert!(bands_observed.contains(&IntegrityBand::Destroyed));
    }

    #[test]
    fn cascade_decay_affects_low_hardness_neighbors() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        // Pre-damage the destroyed-target pixel to integrity 0.05 so a small
        // impact pushes it to 0 + triggers cascade.
        let outcome = t
            .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
            .expect("dirt pixel");
        assert!(outcome.destroyed);
        // 4-neighbors are dirt (hardness 0.2 < 0.6) → all 4 cascade.
        assert_eq!(outcome.cascades.len(), 4);
        for ev in &outcome.cascades {
            assert!(ev.integrity_after < ev.integrity_before);
            assert_eq!(ev.depth, DEFAULT_CASCADE_DEPTH);
            assert_eq!(ev.threshold, DEFAULT_CASCADE_THRESHOLD);
        }
    }

    #[test]
    fn cascade_skips_hard_neighbors() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        // Soft center pixel surrounded by hard concrete.
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_CONCRETE);
        t.set_pixel_internal(10, 10, MATERIAL_LOOSE_FILL);
        // Destroy the soft center.
        let outcome = t
            .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
            .expect("loose fill pixel");
        assert!(outcome.destroyed);
        // No cascade — every neighbor is concrete (hardness 0.7 > threshold 0.6).
        assert!(outcome.cascades.is_empty());
    }

    #[test]
    fn cascade_decay_can_destroy_neighbor_with_low_integrity() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        // Bring the east neighbor (11, 10) down to ~0.05 integrity via several
        // light hits. dirt hardness=0.2 — impact_energy 0.01 yields damage=0.04
        // per hit, so 24 hits drops integrity from 1.0 to ~0.04 without
        // destroying it (so it stays a valid cascade target).
        for _ in 0..24 {
            let _ = t.try_penetrate_pixel(11, 10, 0.01, DamageKind::ProjectileHit, None);
        }
        let int_pre = t.pixel_integrity(11, 10);
        assert!(int_pre < 0.1, "expected low pre-cascade integrity, got {int_pre}");
        assert_eq!(t.material_at(11, 10), MATERIAL_DIRT);
        // Destroy the source pixel — cascade decay (0.1) pushes neighbor to 0.
        let outcome = t
            .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
            .expect("dirt pixel");
        assert!(outcome.destroyed);
        let neighbor_event = outcome
            .cascades
            .iter()
            .find(|ev| ev.to_pos == [11, 10])
            .expect("east neighbor cascade event");
        assert!(neighbor_event.destroyed_neighbor);
        assert_eq!(t.material_at(11, 10), MATERIAL_AIR);
    }

    #[test]
    fn cascade_does_not_recurse_beyond_depth_1() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        // Pre-damage neighbor (11, 10) to 0 via cascade is allowed, but a
        // cascade-killed neighbor at (12, 10) must NOT cascade further to
        // (13, 10). Force (11, 10) and (12, 10) close to destruction.
        let _ = t.try_penetrate_pixel(11, 10, 0.95, DamageKind::ProjectileHit, None);
        let _ = t.try_penetrate_pixel(12, 10, 0.95, DamageKind::ProjectileHit, None);
        let int_13_before = t.pixel_integrity(13, 10);
        let outcome = t
            .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
            .expect("dirt pixel");
        // (11, 10) cascade may destroy it, but (12, 10) and (13, 10) integrity
        // should not be affected by a recursive cascade — they're only
        // adjacent to a cascade-affected pixel, not the original destroyed.
        let int_13_after = t.pixel_integrity(13, 10);
        assert!((int_13_after - int_13_before).abs() < f32::EPSILON);
        let _ = outcome;
    }

    #[test]
    fn pixel_meta_grid_is_sparse_only_for_damaged_pixels() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        // Pristine pixels never get meta entries.
        assert!(t.pixel_meta_grid.is_empty());
        let _ = t.try_penetrate_pixel(5, 5, 0.05, DamageKind::ProjectileHit, None);
        // One damaged pixel → exactly one entry.
        assert_eq!(t.pixel_meta_grid.len(), 1);
    }

    #[test]
    fn destroyed_pixel_removes_meta_entry() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        let _ = t.try_penetrate_pixel(5, 5, 0.05, DamageKind::ProjectileHit, None);
        assert_eq!(t.pixel_meta_grid.len(), 1);
        let _ = t.try_penetrate_pixel(5, 5, 10.0, DamageKind::ProjectileHit, None);
        // Pixel destroyed → meta entry cleared.
        let damaged_keys: Vec<_> = t.pixel_meta_grid.keys().filter(|k| k.lx == 5 && k.ly == 5).collect();
        assert!(damaged_keys.is_empty());
    }

    #[test]
    fn reset_to_default_clears_pixel_meta() {
        let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
        t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
        let _ = t.try_penetrate_pixel(5, 5, 0.05, DamageKind::ProjectileHit, None);
        assert!(!t.pixel_meta_grid.is_empty());
        t.reset_to_default();
        assert!(t.pixel_meta_grid.is_empty());
    }
}
