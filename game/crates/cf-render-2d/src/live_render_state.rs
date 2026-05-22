use bevy::prelude::*;

use cf_actor::ActorObservation;

/// Snapshot of the engine's actor world, written each frame by the `cf-app` bridge
/// system. The render layer reads this and updates Bevy entities without ever owning
/// authoritative state.
#[derive(Resource, Debug, Clone, Default)]
pub struct ActorRenderState {
    pub actors: Vec<ActorObservation>,
    pub player_actor_id: Option<u64>,
    pub region_width: f32,
    pub region_height: f32,
    /// Bottom-left anchor of the play region in world space. Mirrors
    /// `M0EngineConfig::region_anchor_{x,y}` so the render layer can centre the
    /// floor + camera over the actual region for scenarios that don't anchor at
    /// the world origin.
    pub region_anchor_x: f32,
    pub region_anchor_y: f32,
    pub floor_y: f32,
    /// M1.5 breach strips (id + bbox + hp). Empty for non-breach scenarios.
    pub breaches: Vec<BreachRender>,
    /// M1.5 extraction zone if the scenario carries a `ReachZone` objective.
    pub extraction_zone: Option<ExtractionRender>,
    /// attached actors (legs alternate at ~8-tick cadence during Walking /
    /// Running stances). Without this, the silhouette stands still even
    /// while the position moves — which would re-introduce the
    /// "static sliding pawn" M5-DC-3 failure mode the chassis pips exist
    /// to close.
    pub tick: u64,
    /// `ObserveFrame::tool_validity::valid`. Drives the reticle color (red
    /// when false). `None` = no tool-validity tracking active (default).
    pub tool_valid: Option<bool>,
    /// cf-app writes this when `equipment.weapon_fired` fires; cleared each
    /// frame after rendering.
    pub muzzle_flash: Option<MuzzleFlashRender>,
}

#[derive(Debug, Clone)]
pub struct MuzzleFlashRender {
    pub origin: Vec2,
    pub remaining_ticks: u32,
}

/// M1.5 render-side projection of a breach strip.
#[derive(Debug, Clone)]
pub struct BreachRender {
    pub id: String,
    pub bbox_min: [f32; 2],
    pub bbox_max: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    pub broken: bool,
    pub refusal_reason: Option<String>,
}

/// M1.5 render-side projection of the extraction zone.
#[derive(Debug, Clone)]
pub struct ExtractionRender {
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub completed: bool,
}

/// Spawned per actor; carries the actor id so the render system can update or
/// despawn the right entity when the world changes.
#[derive(Component, Debug, Clone, Copy)]
pub struct ActorRenderTag {
    pub id: u64,
}

/// Marker for the floor sprite (M1 stand-in for chunked terrain).
#[derive(Component, Debug)]
pub struct FloorRenderTag;

/// Marker for the aim reticle sprite that follows the player's aim direction.
#[derive(Component, Debug)]
pub struct ReticleRenderTag;

/// Marker for the M1.5 extraction zone sprite (the green goal box).
#[derive(Component, Debug)]
pub struct ExtractionZoneTag;

/// Marker for one M1.5 breach strip rendered as a colored block.
#[derive(Component, Debug, Clone)]
pub struct BreachRenderTag {
    pub id: String,
}

/// chassis-attached actor, one per body zone (Head / Torso / ArmLeft / ArmRight
/// / ForearmLeft / ForearmRight / HandLeft / HandRight / LegLeft / LegRight /
/// ShinLeft / ShinRight / FootLeft / FootRight / Backpack). Each pip's color
/// reflects the zone's external_integrity + destroyed flag so a player watching
/// a wreck_eject scenario can SEE the right forearm pip turn black when the
/// chassis zone gets destroyed — the visible-limb proof for M5-DC-3 that
/// closes the "static sliding pawn" gap from the audit. The pip's position
/// follows the actor's transform + stance offset.
#[derive(Component, Debug, Clone)]
pub struct ChassisZoneRenderTag {
    pub actor_id: u64,
    pub zone: String,
}

/// entry per chassis-attached actor (jet, shield, sensor, weapon_mount,
/// repair_drone). Color reflects module state (Nominal → green, Degraded
/// → yellow, Warning → orange, Failed → red, NotPresent → not rendered).
/// Position follows the bound zone + kind-specific offset. Surfaces sim
/// depth Cortex doesn't have: every chassis carries 5 damageable modules
/// whose health drives gameplay (jet failure grounds the actor, sensor
/// failure blanks the HUD radar, weapon-mount failure jams the rifle).
#[derive(Component, Debug, Clone)]
pub struct ChassisModuleRenderTag {
    pub actor_id: u64,
    pub module_id: String,
}

/// resolves to a rifle. Position follows the right-hand zone (or torso for
/// chassis-less actors) + aim vector; rotation matches aim direction so the
/// rifle visibly points where the player aims. Without this, the actor has
/// NO visible weapon despite firing (only projectiles + muzzle flash hint
/// at the rifle's existence).
#[derive(Component, Debug, Clone)]
pub struct HeldRifleRenderTag {
    pub actor_id: u64,
}

#[derive(Component, Debug)]
pub struct MuzzleFlashTag;

/// without depending on cf-actor's exact struct. Matches the field set we
/// read from `ActorObservation.chassis.zones[]`.
pub(crate) struct ActorChassisZoneView {
    pub zone: String,
    pub external_integrity: f32,
    pub internal_integrity: f32,
    pub core_integrity: f32,
    pub wound_integrity: f32,
    pub destroyed: bool,
}
