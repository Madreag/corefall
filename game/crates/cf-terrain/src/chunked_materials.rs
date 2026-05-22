//! Material registry for the M2 chunked pixel terrain.
//!
//! Owns the DR-007 launch material set (8 ids), the M14E support-beam
//! addition, and the M15 active-material affordances (water, oil, acid,
//! lava, iron, co2, steam, smoke, fire_intense, cloud, rain, acid_droplet).
//!
//! Split out of `chunked.rs` purely as code motion; every public symbol
//! is re-exported from the `chunked` module to preserve the existing
//! `cf_terrain::chunked::<symbol>` API.

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
/// reinforcement (T1 craftable, 2 iron + 1 wood per beam). Anchorable,
/// non-piling, hardness=200 — locks the ±8-pixel integrity field around
/// it to integrity 500 so cave-in roll is suppressed.
pub const MATERIAL_SUPPORT_BEAM: MaterialId = 8;

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

/// Render + physics affordance table. Lookup by id via
/// `material_affordance(id)`. Add entries here when introducing new
/// kernel-reactive materials; absence yields transparent render +
/// air-like physics.
const MATERIAL_TABLE: [MaterialAffordance; 27] = [
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
        // through the stable `material_not_diggable` reason; the specific
        // material is on the payload's `material` field.
        refusal_reason: Some("material_not_diggable"),
    },
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
        id: 53, // co2
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
    MaterialAffordance {
        id: 60,
        name: "chlorine",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 3.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 3.21,
        spawn_material: None,
        path_cost: 3.0,
        overlay_rgba: [180, 240, 130, 120],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 61,
        name: "ammonia",
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
        stickiness: 0.0,
        restitution: 0.0,
        friction: 0.0,
        density: 0.73,
        spawn_material: None,
        path_cost: 3.0,
        overlay_rgba: [220, 240, 170, 110],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 63,
        name: "electric_arc",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 15.0,
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
        path_cost: 5.0,
        overlay_rgba: [200, 230, 255, 220],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 64,
        name: "lightning",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 30.0,
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
        path_cost: 8.0,
        overlay_rgba: [255, 255, 255, 240],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 66,
        name: "polluted_water",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 1.0,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.06,
        restitution: 0.0,
        friction: 0.12,
        density: 1.05,
        spawn_material: None,
        path_cost: 3.0,
        overlay_rgba: [70, 90, 40, 200],
        refusal_reason: None,
    },
    MaterialAffordance {
        id: 25,
        name: "mercury",
        solid: false,
        diggable: false,
        hardness: 0.0,
        anchorable: false,
        hazard: true,
        damage_per_tick: 1.5,
        drillable: false,
        blastable: false,
        beam_cuttable: false,
        projectile_passable: true,
        actor_passable: true,
        blocks_line_of_sight: false,
        stickiness: 0.02,
        restitution: 0.0,
        friction: 0.05,
        density: 13.5,
        spawn_material: None,
        path_cost: 4.0,
        overlay_rgba: [180, 180, 185, 230],
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
        "water" => Some(13),
        "steam" => Some(50),
        "cloud" => Some(71),
        "rain" => Some(87),
        "acid_droplet" => Some(88),
        "oil" => Some(19),
        "acid" => Some(21),
        "alkali" => Some(22),
        "lava" => Some(26),
        "iron" => Some(68),
        "ore_iron" => Some(34),
        "ore_gold" => Some(35),
        "ore_copper" => Some(36),
        "ore_uranium" => Some(37),
        "gold" => Some(73),
        "copper" => Some(74),
        "obsidian" => Some(70),
        "salt" => Some(42),
        "sugar" => Some(43),
        "gunpowder" => Some(48),
        "fabric" => Some(49),
        "ash" => Some(40),
        "charcoal" => Some(41),
        "oxygen" => Some(51),
        "nitrogen" => Some(52),
        "co2" => Some(53),
        "methane" => Some(54),
        "hydrogen" => Some(55),
        "nitrous_oxide" => Some(56),
        "helium" => Some(57),
        "ozone" => Some(58),
        "ethanol_vapor" => Some(59),
        "chlorine" => Some(60),
        "ammonia" => Some(61),
        "smoke" => Some(62),
        "electric_arc" => Some(63),
        "lightning" => Some(64),
        "fire_intense" => Some(65),
        "polluted_water" => Some(66),
        "neutralized_brine" => Some(67),
        "frozen_blood" => Some(72),
        "blood" => Some(23),
        "alcohol" => Some(24),
        "mercury" => Some(25),
        "rust" => Some(38),
        "mud" => Some(39),
        "paper" => Some(47),
        "wood" => Some(8),
        "cloth" => Some(9),
        "glass" => Some(10),
        "cardboard" => Some(11),
        "snow" => Some(12),
        "sand" => Some(14),
        "ice" => Some(15),
        "rock" => Some(16),
        "foam_insulation" => Some(17),
        "vegetation" => Some(18),
        "slime" => Some(27),
        "basalt" => Some(28),
        "brick" => Some(29),
        "marble" => Some(30),
        "sandstone" => Some(31),
        "granite" => Some(32),
        "coal" => Some(33),
        "rubber" => Some(44),
        "plastic" => Some(45),
        "leather" => Some(46),
        "pollutant_x" => Some(83),
        _ => None,
    }
}

/// Material schema version stamped into `run_manifest.json.material_schema_version`
/// so future schema migrations can identify legacy bundles.
pub const MATERIAL_SCHEMA_VERSION: &str = "cf-terrain-launch-v1";
