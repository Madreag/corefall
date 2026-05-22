//! Outcomes emitted by the per-tick sim: [`StepReport`], [`ActorTickOutcome`],
//! [`HitOutcome`], [`SpawnedProjectile`], [`ExpiredProjectile`],
//! [`SettledLooseItem`], plus the [`zone_from_hit`] helper.
//!
//! Extracted from [`crate::sim`] for file size; re-exported from `crate::sim`
//! so existing `cf_actor::sim::X` paths continue to work.

use serde::{Deserialize, Serialize};

use crate::{ActorId, IntentSource, ItemSlot, Status, Vec2};

/// One actor's outcome for the tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorTickOutcome {
    pub actor: ActorId,
    pub source: IntentSource,
    pub previous_status: Status,
    pub new_status: Status,
    pub move_x: f32,
    pub aim: Vec2,
    pub jump_accepted: bool,
    pub reload_started: bool,
    pub reload_completed: bool,
    /// **M1 re-audit pass 4 (2026-05-13)**: reload duration in ticks at the
    /// moment the reload was initiated. Zero unless `reload_started=true`.
    #[serde(default)]
    pub reload_ticks_total: u32,
    /// **M1 re-audit pass 4 (2026-05-13)**: stable per-rifle magazine
    /// counter AFTER any reload completion this tick. Engine uses this
    /// (+ the rifle preset id) to synthesize a stable `magazine_id`.
    #[serde(default)]
    pub magazine_index_after_reload: u32,
    /// **M1 re-audit pass 4 (2026-05-13)**: latched when the player
    /// pressed fire this tick but the rifle was reloading. Engine emits
    /// `control.command_rejected reason="reloading"`.
    #[serde(default)]
    pub fire_denied_reloading: bool,
    /// **M6**: latched when the player pressed fire this tick but the
    /// actor's weapon swap was still in flight. Engine emits
    /// `actor.action_rejected reason="swap_in_progress"`.
    #[serde(default)]
    pub fire_denied_by_swap: bool,
    /// **M6**: bipod state at fire time. When deployed, the engine reports
    /// reduced recoil/bloom via [`outcome.recoil_applied`] +
    /// [`outcome.bloom_factor`] (already multiplied by the bipod factors)
    /// and surfaces the flag in the `equipment.weapon_fired` event for
    /// replay consumers.
    #[serde(default)]
    pub bipod_deployed_at_fire: bool,
    /// **M6**: suppressor state at fire time. When attached, the engine
    /// already multiplied `loudness_radius` by the suppressor factor; this
    /// flag is surfaced so the `equipment.alarm_registered` event can
    /// carry `suppressed=true`.
    #[serde(default)]
    pub suppressor_attached_at_fire: bool,
    /// **M6**: round popped from the magazine on this tick's shot. None
    /// when no shot fired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub popped_round: Option<cf_equipment::PoppedRound>,
    /// **M6**: shell ejection emitted by the shot. None when no shot fired.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_ejection: Option<cf_equipment::ShellEjection>,
    pub fired: bool,
    pub dry_fire: bool,
    pub muzzle_origin: Option<Vec2>,
    pub recoil_applied: f32,
    pub selection_changed: Option<ItemSlot>,
    pub reset: bool,
    pub landed_impulse: f32,
    /// **M5**: set to true on the tick a destroyed `Backpack`/`Jet`-bound zone
    /// disables the jet stance. Engine emits `chassis.jet_failed_due_to_limb_loss`.
    #[serde(default)]
    pub jet_disabled_by_limb_loss: bool,
    /// **M5**: set to true on the tick a destroyed grip-side zone forces gear
    /// drop. Engine emits `actor.gear_dropped` and clears the rifle slot.
    #[serde(default)]
    pub gear_dropped_by_limb_loss: bool,
    /// **M1**: latched when the actor entered DYING this tick. Engine emits
    /// `actor.inventory_dropped` once per DYING entry with the rifle preset id
    /// and hand position.
    #[serde(default)]
    pub entered_dying: bool,
    /// **M1**: position used as the inventory drop hand origin (latched at the
    /// time of DYING entry).
    #[serde(default)]
    pub inventory_drop_position: Option<Vec2>,
    /// **M1**: hand-position toss velocity at DYING entry.
    #[serde(default)]
    pub inventory_drop_velocity: Option<Vec2>,
    /// **M1**: dropped item label (e.g. "rifle"), populated when the dying
    /// actor was carrying a rifle.
    #[serde(default)]
    pub inventory_drop_label: Option<String>,
    /// **M1**: latched when DYING dwell elapsed this tick and the actor
    /// transitioned to DEAD. Engine emits `actor.actor_status_changed` with
    /// from=dying, to=dead and cause="dying_dwell_elapsed".
    #[serde(default)]
    pub dying_dwell_elapsed: bool,
    /// **M1 (Gap C3)**: surfaced from `ActorState::last_lethal_cause_event_id`
    /// so the engine emits `actor.inventory_dropped`,
    /// `actor.actor_status_changed(DYING)`, and
    /// `actor.actor_status_changed(DEAD)` with the lethal event id as parent
    /// even when the DYING dwell elapses on a tick after the killing hit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lethal_cause_event_id: Option<String>,
    /// **M1**: latched when sharp aim was invalidated this tick. Engine emits
    /// `actor.sharp_aim_invalidated` with this reason.
    #[serde(default)]
    pub sharp_aim_invalidation_reason: Option<String>,
    /// **M1**: latched when knockdown began this tick. Engine emits
    /// `physics.authority_changed` with from=animation, to=ragdoll.
    #[serde(default)]
    pub knockdown_started: bool,
    /// **M1**: latched when knockdown recovered this tick. Engine emits
    /// `physics.authority_changed` with from=ragdoll, to=animation.
    #[serde(default)]
    pub knockdown_recovered: bool,
    /// **M1**: noise/alarm radius emitted by a fire event. Zero when not
    /// firing; non-zero only when `fired=true`. Engine consumes this to
    /// emit `equipment.alarm_registered`.
    #[serde(default)]
    pub loudness_radius: f32,
    /// **M1**: most-recent bloom factor (mirrored from
    /// `ActorState::bloom_factor`). Engine writes this into events the HUD
    /// reticle widget reads.
    #[serde(default)]
    pub bloom_factor: f32,
    /// **M1 audit pass 6 (2026-05-13)**: latched when travel-impulse
    /// damage was applied this tick to an UNSTABLE actor (per CCCP
    /// `Actor.cpp:1199`). Engine emits `actor.actor_status_changed
    /// cause="travel_impulse"` AND `cf_audio::AudioCue::BodyHit` from
    /// this flag.
    #[serde(default)]
    pub travel_impulse_damage: bool,
}

/// Hit applied to an actor by a projectile this tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HitOutcome {
    pub projectile_id: u64,
    pub shooter: ActorId,
    pub target: ActorId,
    pub hit_position: Vec2,
    pub damage: f32,
    pub previous_status: Status,
    pub new_status: Status,
    /// **M5**: body zone resolved from `hit_position` relative to the target's
    /// AABB. Engine consumers route chassis-grade hits through
    /// `cf_chassis::ChassisState::apply_zone_damage` using this zone label.
    #[serde(default = "default_hit_zone")]
    pub zone: String,
    /// **M5**: chassis zone damage outcome when the target had a chassis attached.
    /// `None` when the target has no chassis. The engine reads this to emit
    /// `chassis.armor_layer_damaged` / `module_state_changed` events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chassis_outcome: Option<cf_chassis::ZoneDamageOutcome>,
    /// **M14 audit pass 3 (Finding 4)**: entry parameter `t` from the
    /// swept-segment intersection, in [0, 1]. Lets the engine emit
    /// accurate `combat.swept_collision.entry_t` even when the hit
    /// resolves multiple ticks after the projectile spawn.
    #[serde(default)]
    pub entry_t: f32,
    /// **M14 audit pass 3 (Finding 4)**: world-space ray-origin position
    /// (projectile start of this tick). Lets the engine emit
    /// `combat.swept_collision.ray_origin` accurately even for delayed hits.
    #[serde(default)]
    pub ray_origin: Vec2,
    /// **M14 audit pass 3 (Finding 4)**: normalized ray direction at the
    /// instant of hit. Lets the engine emit
    /// `combat.swept_collision.ray_direction` accurately for delayed hits.
    #[serde(default = "default_ray_direction")]
    pub ray_direction: Vec2,
    /// **M14 audit pass 3 (Finding 4)**: distance from ray_origin to the
    /// hit_position along the ray. Lets the engine emit
    /// `combat.swept_collision.distance_traveled` without reconstruction.
    #[serde(default)]
    pub distance_traveled: f32,
}

fn default_ray_direction() -> Vec2 {
    Vec2::new(1.0, 0.0)
}

fn default_hit_zone() -> String {
    "torso".to_string()
}

/// **M5**: derive a body zone from a hit position relative to the target AABB.
/// Used by both projectile and explicit damage paths so every chassis hit
/// carries a zone label without engine-side guessing.
///
/// The 14-zone resolution maps the actor's AABB into five horizontal bands
/// (head / upper torso+arms / mid forearms / lower hands+thighs / shins / feet)
/// and three lateral lanes (left arm/leg, torso/center, right arm/leg) so the
/// granular M5 body graph receives meaningful per-hit damage.
///
/// **DR-033 forward-hook**: thin `Vec2` adapter that delegates to
/// `cf_physics::zone_from_hit` so M5.5's per-zone collision routing can reach
/// the resolver from the physics crate without depending on `cf-actor`.
pub fn zone_from_hit(target_position: Vec2, half_extents: Vec2, hit_position: Vec2) -> cf_chassis::BodyZone {
    cf_physics::zone_from_hit(
        (target_position.x, target_position.y),
        (half_extents.x, half_extents.y),
        (hit_position.x, hit_position.y),
    )
}

/// Spawned projectile metadata for the recorder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnedProjectile {
    pub id: u64,
    pub owner: ActorId,
    pub origin: Vec2,
    pub velocity: Vec2,
    pub damage: f32,
    /// Loudness radius in world units. AI guards within this radius
    /// can detect the shot for awareness/alert purposes.
    pub loudness_radius: f32,
    /// **M1**: tracer flag for this projectile (CCCP `Magazine.RTTRatio`).
    /// Applies uniformly to every particle of a multi-pellet shot — tracer
    /// is per-shot, not per-particle.
    #[serde(default)]
    pub is_tracer: bool,
    /// **M1**: index of this projectile within the same shot (0..particle_count-1).
    #[serde(default)]
    pub particle_index: u32,
    /// **M1**: total particles in this shot. =1 for single-round weapons.
    #[serde(default = "default_particle_count_in_shot")]
    pub particle_count: u32,
    /// **M14C** § round-kind discriminator for this spawned projectile so
    /// downstream M14C HEAT / APFSDS producers can route on the right path
    /// (HEAT → `heat_impact_producer`, APFSDS → `apfsds_impact_producer`,
    /// other kinds → existing M14 traversal path).
    #[serde(default = "default_round_kind_in_shot")]
    pub round_kind: cf_equipment::RoundKind,
}

fn default_particle_count_in_shot() -> u32 {
    1
}

fn default_round_kind_in_shot() -> cf_equipment::RoundKind {
    cf_equipment::RoundKind::Regular
}

/// Projectile that flew off the map / outlasted its budget without hitting anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpiredProjectile {
    pub id: u64,
    pub owner: ActorId,
    pub last_position: Vec2,
}

/// **M1 R2 / Gap G1**: outcome row for a `LooseItem` that just settled this
/// tick. The engine consumes this to fire one-shot
/// `actor.inventory_settled` events with parent_event_id = the originating
/// `actor.inventory_dropped`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SettledLooseItem {
    pub id: u64,
    pub source_event_id: String,
    pub item_label: String,
    pub position: Vec2,
}

/// All structured outcomes from one [`crate::sim::step`]. The engine turns these
/// into recorder events.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StepReport {
    pub actor_outcomes: Vec<ActorTickOutcome>,
    pub spawned_projectiles: Vec<SpawnedProjectile>,
    pub hits: Vec<HitOutcome>,
    pub expired_projectiles: Vec<ExpiredProjectile>,
    /// **M1 R2 / Gap G1**: loose items that came to rest on this tick.
    /// Engine emits one `actor.inventory_settled` event per entry.
    #[serde(default)]
    pub settled_loose_items: Vec<SettledLooseItem>,
}
