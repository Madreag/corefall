//! M1 actor simulation step: per-tick actor + rifle + projectile pipeline.
//!
//! This module owns the deterministic per-tick state transition for the M1 actor world:
//!
//! 1. Snapshot the inbound [`ControlIntent`]s.
//! 2. Apply movement/aim/jump/reload/fire intent.
//! 3. Step kinematics through `cf-physics`.
//! 4. Step the rifle state machine (`cf-equipment`).
//! 5. Spawn projectiles when fire fired this tick.
//! 6. Step projectiles + check actor hits.
//! 7. Resolve status changes from accumulated damage.
//!
//! Output is a [`StepReport`] of structured outcomes the engine turns into recorder
//! events. The sim itself never touches the recorder so we keep `cf-actor` decoupled
//! from `cf-replay`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use cf_equipment::{tick_rifle, RifleState, RifleTickInputs, TickOutcomes as RifleOutcomes};
use cf_physics::{
    apply_horizontal_motion, apply_jump, apply_recoil, step_kinematics, HorizontalInputs, JumpInputs, StepInputs,
};

use crate::{quantize_f32, ActorId, ActorWorld, ControlIntent, IntentSource, InventoryItem, ItemSlot, Stance, Status, Vec2};

/// Per-actor rifle state tracked alongside the actor world. Keyed by [`ActorId`]; only
/// actors carrying a rifle in their inventory get an entry.
pub type RifleStates = BTreeMap<ActorId, RifleState>;

/// Movement tuning. Hard-coded for the M1 actor; M5 will move these into the chassis
/// grammar so different chassis have different feel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActorTuning {
    pub max_speed: f32,
    pub ground_acceleration: f32,
    pub air_acceleration: f32,
    pub ground_friction: f32,
    pub jump_impulse: f32,
    pub terminal_velocity: f32,
}

impl Default for ActorTuning {
    fn default() -> Self {
        Self {
            max_speed: 220.0,
            ground_acceleration: 1500.0,
            air_acceleration: 600.0,
            ground_friction: 1200.0,
            jump_impulse: 420.0,
            terminal_velocity: -1800.0,
        }
    }
}

/// Inputs for one [`step`] call.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepDeps {
    pub tick_dt: f32,
    pub region_min_x: f32,
    pub region_max_x: f32,
    /// Upper Y bound (in world units) for projectile out-of-bounds expiry. Derived
    /// from the scenario region height by the engine; the X-axis already uses
    /// `region_min_x` / `region_max_x`, and this mirrors the same data-driven
    /// pattern on the Y-axis instead of a hardcoded constant.
    pub region_max_y: f32,
    pub auto_reload_when_empty: bool,
}

/// One projectile in flight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    pub id: u64,
    pub owner: ActorId,
    pub origin: Vec2,
    pub position: Vec2,
    pub velocity: Vec2,
    pub damage: f32,
    pub remaining_ticks: u32,
}

/// Entire mutable sim state owned by the engine across ticks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorSimState {
    pub world: ActorWorld,
    pub rifles: RifleStates,
    pub projectiles: Vec<Projectile>,
    next_projectile_id: u64,
}

impl ActorSimState {
    pub fn new(world: ActorWorld) -> Self {
        Self {
            world,
            rifles: BTreeMap::new(),
            projectiles: Vec::new(),
            next_projectile_id: 0,
        }
    }

    pub fn ensure_rifle_for(&mut self, actor_id: ActorId, state: RifleState) {
        self.rifles.entry(actor_id).or_insert(state);
    }

    /// Current value of the projectile-id counter. Exposed so callers that rebuild
    /// the sim state (e.g. `scenario.reset`) can carry the counter forward and keep
    /// `projectile_id` globally unique across resets in the (monotonic) event log.
    pub fn next_projectile_id(&self) -> u64 {
        self.next_projectile_id
    }

    /// Override the projectile-id counter. The next allocated projectile id will
    /// be `id`. Used by `scenario.reset` to preserve uniqueness of `projectile_id`
    /// across the reset boundary; the event log's `combat.projectile_*` cause-chain
    /// would otherwise alias pre-reset and post-reset projectiles.
    pub fn set_next_projectile_id(&mut self, id: u64) {
        self.next_projectile_id = id;
    }

    fn allocate_projectile_id(&mut self) -> u64 {
        let id = self.next_projectile_id;
        self.next_projectile_id += 1;
        id
    }

    /// Hash bytes for the deterministic checksum. Layout-stable; future milestones
    /// append projectile + RNG state without changing earlier slots.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = self.world.checksum_bytes();
        out.extend_from_slice(&(self.rifles.len() as u64).to_le_bytes());
        for (id, state) in &self.rifles {
            out.extend_from_slice(&id.0.to_le_bytes());
            out.extend_from_slice(&state.ammo_in_mag.to_le_bytes());
            out.extend_from_slice(&state.fire_cooldown_ticks.to_le_bytes());
            out.extend_from_slice(&state.reload_remaining_ticks.to_le_bytes());
        }
        out.extend_from_slice(&(self.projectiles.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.next_projectile_id.to_le_bytes());
        for p in &self.projectiles {
            out.extend_from_slice(&p.id.to_le_bytes());
            out.extend_from_slice(&p.owner.0.to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.position.x).to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.position.y).to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.velocity.x).to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.velocity.y).to_le_bytes());
            out.extend_from_slice(&p.remaining_ticks.to_le_bytes());
        }
        out
    }
}

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
}

fn default_particle_count_in_shot() -> u32 {
    1
}

/// Projectile that flew off the map / outlasted its budget without hitting anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpiredProjectile {
    pub id: u64,
    pub owner: ActorId,
    pub last_position: Vec2,
}

/// All structured outcomes from one [`step`]. The engine turns these into recorder
/// events.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct StepReport {
    pub actor_outcomes: Vec<ActorTickOutcome>,
    pub spawned_projectiles: Vec<SpawnedProjectile>,
    pub hits: Vec<HitOutcome>,
    pub expired_projectiles: Vec<ExpiredProjectile>,
}

/// Run one fixed-tick step for every actor in `state.world`.
///
/// `intents` maps actor → their [`ControlIntent`] for this tick. Actors without an
/// entry are stationary (idle move, no fire/jump/reload). Actors whose `status` does
/// not [`Status::accepts_input`] ignore movement/fire/reload intents but still take
/// physics steps (so a downed actor falls).
///
/// `rng` is the engine's seeded source consumed for multi-particle spread cones.
/// Pass any `FnMut() -> u64`; in production it forwards to
/// `cf_sim_core::Rng::next_u64`. Single-particle weapons never call it.
pub fn step<R: FnMut() -> u64>(
    state: &mut ActorSimState,
    intents: &mut BTreeMap<ActorId, ControlIntent>,
    deps: StepDeps,
    rng: &mut R,
) -> StepReport {
    let mut report = StepReport::default();

    let actor_ids: Vec<ActorId> = state.world.actors.keys().copied().collect();
    for actor_id in actor_ids {
        let intent = intents
            .remove(&actor_id)
            .unwrap_or_else(|| ControlIntent::new(actor_id, IntentSource::Cfctl));
        let outcome = step_one_actor(state, actor_id, intent, deps, &mut report, rng);
        report.actor_outcomes.push(outcome);
    }

    step_projectiles(state, deps, &mut report);

    report
}

/// Convenience wrapper for tests / callers that don't need per-tick RNG (no
/// multi-particle weapons in play). Internally seeds a zero state; safe for
/// deterministic single-particle scenarios.
pub fn step_no_rng(
    state: &mut ActorSimState,
    intents: &mut BTreeMap<ActorId, ControlIntent>,
    deps: StepDeps,
) -> StepReport {
    let mut counter: u64 = 0x6b67c9_8a7f_3d1ad9_u64;
    step(state, intents, deps, &mut || {
        counter = counter.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        counter
    })
}

fn step_one_actor<R: FnMut() -> u64>(
    state: &mut ActorSimState,
    actor_id: ActorId,
    intent: ControlIntent,
    deps: StepDeps,
    report: &mut StepReport,
    rng: &mut R,
) -> ActorTickOutcome {
    let floor_y = state.world.floor_y;
    let gravity = state.world.gravity;

    // First pass: mutate the actor (movement, physics, intent application). The borrow
    // on `state.world.actors` ends here so we can touch `state.projectiles` / `state.rifles`
    // afterward without aliasing.
    struct ActorPass {
        outcome: ActorTickOutcome,
        accepted_input: bool,
        early_exit: bool,
    }

    let pass: ActorPass = {
        let actor = state
            .world
            .actors
            .get_mut(&actor_id)
            .expect("actor id exists by construction");

        let previous_status = actor.status;
        let accepted_input = actor.status.accepts_input() && actor.knockdown_ticks_remaining == 0;
        let mut outcome = ActorTickOutcome {
            actor: actor_id,
            source: intent.source,
            previous_status,
            new_status: previous_status,
            move_x: 0.0,
            aim: actor.aim,
            jump_accepted: false,
            reload_started: false,
            reload_completed: false,
            fired: false,
            dry_fire: false,
            muzzle_origin: None,
            recoil_applied: 0.0,
            selection_changed: None,
            reset: false,
            landed_impulse: 0.0,
            jet_disabled_by_limb_loss: false,
            gear_dropped_by_limb_loss: false,
            entered_dying: false,
            inventory_drop_position: None,
            inventory_drop_velocity: None,
            inventory_drop_label: None,
            dying_dwell_elapsed: false,
            lethal_cause_event_id: None,
            sharp_aim_invalidation_reason: None,
            knockdown_started: false,
            knockdown_recovered: false,
            loudness_radius: 0.0,
            bloom_factor: actor.bloom_factor,
        };

        if intent.reset {
            actor.reset();
            outcome.reset = true;
            outcome.new_status = actor.status;
            outcome.aim = actor.aim;
            ActorPass {
                outcome,
                accepted_input,
                early_exit: true,
            }
        } else {
            if accepted_input {
                if let Some(slot) = intent.selected_item {
                    if actor.inventory.try_select(slot) {
                        outcome.selection_changed = Some(slot);
                    }
                }
            }
            if accepted_input && intent.aim != Vec2::ZERO {
                actor.aim = intent.aim.normalize_or_x();
                outcome.aim = actor.aim;
            }
            // Refresh ground contact from the current resting position so a jump pressed
            // on the first tick after spawn or reset (where `on_ground` defaults to false)
            // is honored when the actor is already standing on the floor. Mirrors the
            // `was_on_ground` check inside `step_kinematics`.
            if !actor.on_ground
                && actor.velocity.y <= 0.0
                && (actor.position.y - (floor_y + actor.half_extents.y)).abs() < 1e-3
            {
                actor.on_ground = true;
            }
            // Tuning is read once and shared across jump/horizontal-motion/kinematics so
            // any change to ActorTuning (e.g. M5 chassis grammar) propagates uniformly
            // instead of leaving stale hardcoded values in jump-impulse space.
            //
            // **M5**: when a chassis is attached, the BodyGraph's destroyed-zone
            // movement-contribution multipliers scale max_speed + jump impulse,
            // and the `disables_rifle_when_destroyed` flag gates fire/reload.
            // The `forces_crawl_when_destroyed` and `disables_jet_when_destroyed`
            // flags route through stance derivation + jet command rejection.
            let mut tuning = ActorTuning::default();
            // Mass-based physics feel: heavier chassis accelerate/decelerate
            // slower and jump lower, making them feel appropriately weighty.
            // 80kg = 1.0x, 200kg = 0.63x, 600kg = 0.37x.
            let mass_factor = (80.0 / actor.mass_kg.max(1.0)).sqrt().min(1.0);
            tuning.ground_acceleration *= mass_factor;
            tuning.air_acceleration *= mass_factor;
            tuning.ground_friction *= mass_factor;
            tuning.jump_impulse *= mass_factor;
            let (move_factor, jump_factor, _disable_rifle, force_crawl, drop_gear, disable_jet) =
                if let Some(chassis) = actor.chassis.as_ref() {
                    chassis.body_graph.movement_factor(&chassis.destroyed_zones())
                } else {
                    (1.0_f32, 1.0_f32, false, false, false, false)
                };
            if disable_jet && actor.jet_active {
                actor.jet_active = false;
                outcome.jet_disabled_by_limb_loss = true;
            }
            if drop_gear && !actor.gear_dropped_by_limb_loss {
                actor.gear_dropped_by_limb_loss = true;
                outcome.gear_dropped_by_limb_loss = true;
            }
            let effective_jump_impulse = tuning.jump_impulse * jump_factor;
            let effective_max_speed = if force_crawl {
                tuning.max_speed * 0.25
            } else {
                tuning.max_speed * move_factor
            };
            if accepted_input && intent.jump {
                let (new_vy, accepted) = apply_jump(JumpInputs {
                    velocity_y: actor.velocity.y,
                    on_ground: actor.on_ground,
                    jump_impulse: effective_jump_impulse,
                });
                actor.velocity.y = new_vy;
                if accepted {
                    actor.on_ground = false;
                }
                outcome.jump_accepted = accepted;
            }
            let move_x_input = if accepted_input { intent.move_x } else { 0.0 };
            outcome.move_x = move_x_input;

            let h_out = apply_horizontal_motion(HorizontalInputs {
                position_x: actor.position.x,
                velocity_x: actor.velocity.x,
                move_x: move_x_input,
                max_speed: effective_max_speed,
                ground_acceleration: tuning.ground_acceleration,
                air_acceleration: tuning.air_acceleration,
                ground_friction: tuning.ground_friction,
                on_ground: actor.on_ground,
                tick_dt: deps.tick_dt,
                min_x: deps.region_min_x,
                max_x: deps.region_max_x,
            });
            actor.position.x = h_out.position_x;
            actor.velocity.x = h_out.velocity_x;

            let kin = step_kinematics(StepInputs {
                position_y: actor.position.y,
                velocity_y: actor.velocity.y,
                gravity,
                tick_dt: deps.tick_dt,
                floor_y,
                half_extent_y: actor.half_extents.y,
                terminal_velocity_y: tuning.terminal_velocity,
            });
            actor.position.y = kin.position_y;
            actor.velocity.y = kin.velocity_y;
            actor.on_ground = kin.on_ground;
            outcome.landed_impulse = kin.landed_impulse;

            outcome.new_status = actor.status;

            ActorPass {
                outcome,
                accepted_input,
                early_exit: false,
            }
        }
    };

    let mut outcome = pass.outcome;
    if pass.early_exit {
        if let Some(rifle) = state.rifles.get_mut(&actor_id) {
            rifle.reset();
        }
        // Mirror `ControlCommand::ScenarioReset` semantics for the resetting actor's
        // own projectiles: drain any of their pre-reset shots from the projectile
        // pool so a player who fires and then immediately resets cannot have their
        // pre-reset projectile hit the dummy after respawn. Only the resetting
        // actor's projectiles are cleared (other actors' shots still fly), and
        // each cleared projectile is reported as expired so the engine emits a
        // matching `combat.projectile_expired` event and every spawned projectile
        // in the run-bundle has a balanced termination event.
        state.projectiles.retain(|p| {
            if p.owner == actor_id {
                report.expired_projectiles.push(ExpiredProjectile {
                    id: p.id,
                    owner: p.owner,
                    last_position: p.position,
                });
                false
            } else {
                true
            }
        });
        return outcome;
    }
    if !pass.accepted_input {
        return outcome;
    }

    // Tick the rifle (separate borrow from the actor world). Fire/reload intent only
    // applies when the actor's currently selected inventory slot is the rifle AND the
    // chassis grammar still permits weapon handling (right arm chain intact); otherwise
    // the rifle still ticks (so cooldowns advance) but ignores the pressed edges.
    //
    // **M5**: a destroyed `HandRight` / `ForearmRight` / `ArmRight` zone with
    // `disables_rifle_when_destroyed=true` in the BodyGraph movement contribution
    // gates the fire path so a player with a blown-off rifle arm cannot keep shooting.
    let (rifle_selected, rifle_disabled_by_limb_loss, weapon_jammed) = {
        let actor = state.world.actors.get(&actor_id);
        let selected = actor.is_some_and(|a| a.inventory.selected_item().is_rifle());
        let (rifle_off, jammed) = actor.and_then(|a| a.chassis.as_ref()).map_or((false, false), |c| {
            let (_, _, disable_rifle, _, _, _) = c.body_graph.movement_factor(&c.destroyed_zones());
            (disable_rifle, c.weapon_jammed)
        });
        (selected, rifle_off, jammed)
    };
    let can_fire = rifle_selected && !rifle_disabled_by_limb_loss && !weapon_jammed;
    let rifle_outcomes = if let Some(rifle) = state.rifles.get_mut(&actor_id) {
        let inputs = RifleTickInputs {
            // Fire is honored on either the edge `intent.fire` or the sticky
            // `intent.fire_held`. The rifle's fire_mode controls semantics:
            // Semi latches after one shot, FullAuto auto-repeats at cadence.
            fire_pressed: (intent.fire || intent.fire_held) && can_fire,
            reload_pressed: intent.reload && can_fire,
            auto_reload_when_empty: deps.auto_reload_when_empty && can_fire,
        };
        tick_rifle(rifle, inputs)
    } else {
        RifleOutcomes::empty()
    };
    outcome.reload_started = rifle_outcomes.reload_started;
    outcome.reload_completed = rifle_outcomes.reload_completed;
    outcome.dry_fire = rifle_outcomes.dry_fire;

    if rifle_outcomes.fired_this_tick {
        outcome.fired = true;
        outcome.recoil_applied = rifle_outcomes.recoil_impulse_applied;
        let (spec, max_flight) = state
            .rifles
            .get(&actor_id)
            .map(|r| (r.spec.clone(), r.projectile_max_flight_ticks()))
            .expect("fired rifle must have a state");

        // Reborrow actor briefly to apply recoil + read aim/position.
        let (muzzle, aim, base_velocity, damage) = {
            let actor = state
                .world
                .actors
                .get_mut(&actor_id)
                .expect("actor id exists by construction");
            actor.velocity.x = apply_recoil(actor.velocity.x, actor.aim.x, rifle_outcomes.recoil_impulse_applied);
            let aim = if actor.aim == Vec2::ZERO {
                Vec2::new(1.0, 0.0)
            } else {
                actor.aim.normalize_or_x()
            };
            // Project the forward offset along the aim direction so the muzzle origin
            // tracks vertical/diagonal aim (e.g. straight up no longer collapses to the
            // actor's centre x because `signum(0)` is 0). The vertical rifle offset is
            // independent of aim — it represents where the rifle sits on the chassis.
            let muzzle = Vec2::new(
                actor.position.x + aim.x * spec.muzzle_forward_offset,
                actor.position.y + spec.muzzle_vertical_offset + aim.y * spec.muzzle_forward_offset,
            );
            // M1: projectile velocity inheritance is data-driven per
            // `RifleSpec::inherits_firer_velocity` (CCCP `HDFirearm.cpp:752`).
            // True => half actor velocity is added so running-and-gunning shots
            // arc. False (mortar-style) => pure muzzle velocity.
            let inherit_fraction = if spec.inherits_firer_velocity { 0.5_f32 } else { 0.0_f32 };
            let base_velocity = Vec2::new(
                aim.x * spec.projectile_speed + actor.velocity.x * inherit_fraction,
                aim.y * spec.projectile_speed + actor.velocity.y * inherit_fraction,
            );
            (muzzle, aim, base_velocity, spec.damage_per_hit)
        };
        outcome.muzzle_origin = Some(muzzle);
        // Loudness radius (CCCP HDFirearm.cpp:948): scaled by spec.loudness too.
        let loudness_radius = 480.0_f32 * (damage / 10.0).max(1.0).min(3.0) * spec.loudness.max(0.1);
        outcome.loudness_radius = loudness_radius;

        // Per-particle spawn loop. Particle count >= 1; >1 produces a spread
        // cone of `spread_radians` around the aim direction. Each particle
        // gets a unique projectile_id. The tracer flag from `tick_rifle`
        // applies to ALL particles of this shot (CCCP `Round.RTTRatio` is
        // per-shot, not per-particle).
        let particle_count = spec.particle_count.max(1);
        let half_spread = spec.spread_radians * 0.5;
        // Angle of the base aim (radians).
        let base_angle = aim.y.atan2(aim.x);
        let base_speed = (base_velocity.x * base_velocity.x + base_velocity.y * base_velocity.y).sqrt();
        let inherit_fraction = if spec.inherits_firer_velocity { 0.5_f32 } else { 0.0_f32 };
        // Reborrow actor.velocity for inheritance addition per particle.
        let actor_velocity = state
            .world
            .actors
            .get(&actor_id)
            .map(|a| a.velocity)
            .unwrap_or(Vec2::ZERO);
        for particle_idx in 0..particle_count {
            // Deterministic in-cone offset for particles 2..N. The first
            // particle always flies on the base aim line so single-particle
            // weapons match historical behaviour byte-for-byte.
            let angle = if particle_count == 1 || half_spread <= 0.0 {
                base_angle
            } else {
                // Sample a uniform[-1, 1] from the engine's seeded RNG.
                let raw = (rng() as f64) / (u64::MAX as f64); // [0, 1]
                let unit = (raw as f32) * 2.0 - 1.0; // [-1, +1]
                base_angle + unit * half_spread
            };
            let projectile_id = state.allocate_projectile_id();
            // Speed mirrors the base projectile speed (no per-particle inherit
            // skew so all pellets follow the spec'd muzzle profile).
            let speed = if base_speed > 0.0 { base_speed } else { spec.projectile_speed };
            // Decompose: pure muzzle vector along the angle plus inherited actor velocity.
            let dir = Vec2::new(angle.cos(), angle.sin());
            let velocity = Vec2::new(
                dir.x * spec.projectile_speed + actor_velocity.x * inherit_fraction,
                dir.y * spec.projectile_speed + actor_velocity.y * inherit_fraction,
            );
            let _ = speed; // suppress unused warning when needed
            state.projectiles.push(Projectile {
                id: projectile_id,
                owner: actor_id,
                origin: muzzle,
                position: muzzle,
                velocity,
                damage,
                remaining_ticks: max_flight,
            });
            report.spawned_projectiles.push(SpawnedProjectile {
                id: projectile_id,
                owner: actor_id,
                origin: muzzle,
                velocity,
                damage,
                loudness_radius,
                is_tracer: rifle_outcomes.fired_is_tracer,
                particle_index: particle_idx,
                particle_count,
            });
        }
        // M1: alternating recoil accumulator (CCCP HDFirearm.cpp:891) — each
        // shot pushes the muzzle drift toward the opposite sign of the last
        // one so the climb pattern feels predictable, not chaotic.
        {
            let actor = state
                .world
                .actors
                .get_mut(&actor_id)
                .expect("actor id exists by construction");
            let sign = if actor.recoil_alternation_sign >= 0 {
                1.0
            } else {
                -1.0
            };
            let contribution = rifle_outcomes.recoil_impulse_applied / 100.0;
            actor.recoil_accumulator += sign * contribution;
            actor.recoil_alternation_sign = -actor.recoil_alternation_sign;
            if actor.recoil_alternation_sign == 0 {
                actor.recoil_alternation_sign = 1;
            }
        }
    }

    // W1.3 + M1: stability tracking, knockdown, sharp aim, recoil decay,
    // bloom factor, DYING dwell, travel-impulse damage.
    {
        let actor = state
            .world
            .actors
            .get_mut(&actor_id)
            .expect("actor id exists by construction");

        // Mass-scaled stability: heavier actors resist destabilization.
        // 80kg infantry = 1.0x cost; 200kg powered armor = 0.4x; 600kg mech = 0.13x.
        let mass_resistance = (80.0 / actor.mass_kg.max(1.0)).min(2.0);

        // Recoil destabilizes proportional to impulse strength, scaled by mass.
        if outcome.recoil_applied > 0.0 {
            let recoil_cost = (outcome.recoil_applied / 200.0).min(0.3) * mass_resistance;
            actor.stability = (actor.stability - recoil_cost).max(0.0);
        }

        // Landing impact destabilizes based on vertical impulse magnitude.
        if outcome.landed_impulse > 0.0 {
            let impact_cost = (outcome.landed_impulse / 1000.0).min(0.5) * mass_resistance;
            actor.stability = (actor.stability - impact_cost).max(0.0);
        }

        // Recovery toward 1.0 when on ground and no disruption this tick.
        if actor.on_ground && outcome.recoil_applied == 0.0 && outcome.landed_impulse == 0.0 {
            actor.stability = (actor.stability + actor.stability_recovery_rate).min(1.0);
        }

        // M1: recoil accumulator decay (CCCP HDFirearm.cpp:891) — angular drift
        // exponentially decays toward zero each tick the actor isn't firing.
        if outcome.recoil_applied == 0.0 && actor.recoil_accumulator.abs() > 1e-4 {
            let decay = actor.recoil_decay_rate * actor.recoil_accumulator.signum();
            let next = actor.recoil_accumulator - decay;
            if next.signum() != actor.recoil_accumulator.signum() {
                actor.recoil_accumulator = 0.0;
            } else {
                actor.recoil_accumulator = next;
            }
        }

        // M1: sharp aim build / invalidation (CCCP AHuman.cpp:1779).
        // Build only when ALL conditions hold: STABLE status (not Unstable, not
        // knockdown), grounded, slow, equipped (rifle selected), and player
        // holds the sharp_aim sticky input.
        let rifle_equipped_for_sharp = actor.inventory.selected_item().is_rifle();
        let horizontal_speed = actor.velocity.x.abs();
        let walk_threshold = actor.walk_threshold;
        let prior_sharp = actor.sharp_aim_progress;
        let mut invalidation: Option<&'static str> = None;
        let mut invalidation_reason: Option<String> = None;
        if outcome.fired {
            // Firing doesn't invalidate by itself; the reticle bloom handles it.
        }
        if outcome.reload_started {
            invalidation = Some("reloading");
        }
        if outcome.jump_accepted {
            invalidation = Some("jumped");
        }
        if outcome.selection_changed.is_some() {
            invalidation = Some("item_swap");
        }
        if !rifle_equipped_for_sharp && prior_sharp > 0.0 {
            invalidation = invalidation.or(Some("unequipped"));
        }
        if horizontal_speed > walk_threshold && prior_sharp > 0.0 {
            invalidation = invalidation.or(Some("moved"));
        }
        if !matches!(actor.status, Status::Stable) && prior_sharp > 0.0 {
            invalidation = invalidation.or(Some("unstable"));
        }
        if actor.knockdown_ticks_remaining > 0 && prior_sharp > 0.0 {
            invalidation = invalidation.or(Some("knockdown"));
        }
        if invalidation.is_some() {
            actor.sharp_aim_progress = 0.0;
            actor.sharp_aim_active = false;
        } else if intent.sharp_aim
            && rifle_equipped_for_sharp
            && actor.on_ground
            && horizontal_speed <= walk_threshold
            && matches!(actor.status, Status::Stable)
            && actor.knockdown_ticks_remaining == 0
        {
            let build_step = if actor.sharp_aim_build_ticks > 0 {
                1.0 / (actor.sharp_aim_build_ticks as f32)
            } else {
                1.0
            };
            actor.sharp_aim_progress = (actor.sharp_aim_progress + build_step).min(1.0);
            actor.sharp_aim_active = true;
        } else if !intent.sharp_aim {
            actor.sharp_aim_active = false;
            // Releasing the hold without an invalidation cause decays sharp aim.
            actor.sharp_aim_progress = (actor.sharp_aim_progress - 0.05).max(0.0);
        }
        if let Some(reason) = invalidation {
            invalidation_reason = Some(reason.to_string());
        }
        outcome.sharp_aim_invalidation_reason = invalidation_reason;

        // M1: movement accuracy bloom (OpenSoldat Sprites.pas:4870).
        // standing/walking = 1.0×; running/jumping/jetting = 7.0×;
        // airborne/prone-transition = 3.0×. Sharp aim multiplies the bloom by
        // (1 - 0.6 * sharp_aim_progress) so a full sharp aim cuts the reticle
        // down to 40% of its baseline.
        let speed = actor.velocity.x.abs();
        let mut bloom: f32 = if !actor.on_ground {
            3.0
        } else if speed >= Stance::RUN_THRESHOLD {
            7.0
        } else {
            1.0
        };
        if actor.jet_active {
            bloom = bloom.max(7.0);
        }
        if outcome.recoil_applied > 0.0 {
            bloom *= 1.5;
        }
        if outcome.reload_started || rifle_outcomes.reload_completed {
            bloom *= 1.2;
        }
        // Sharp aim tightens the reticle (scaled by progress).
        let sharp_tighten = 1.0 - 0.6 * actor.sharp_aim_progress;
        bloom *= sharp_tighten.max(0.4);
        actor.bloom_factor = bloom;
        outcome.bloom_factor = bloom;

        // Knockdown: when stability is critically low and the actor just took
        // a destabilizing event, trigger a knockdown stun. The actor cannot act
        // for knockdown_ticks_remaining ticks (similar to Downed but recoverable).
        // At 60Hz, 18 ticks = 0.3s stun — enough to feel the impact without
        // being frustrating. Only triggers once per knockdown (ticks_remaining == 0).
        const KNOCKDOWN_STABILITY_THRESHOLD: f32 = 0.1;
        const KNOCKDOWN_DURATION_TICKS: u32 = 18;
        let took_hit = outcome.recoil_applied > 0.0 || outcome.landed_impulse > 100.0;
        let was_in_knockdown = actor.knockdown_ticks_remaining > 0;
        if actor.stability < KNOCKDOWN_STABILITY_THRESHOLD
            && took_hit
            && actor.knockdown_ticks_remaining == 0
            && actor.status.accepts_input()
        {
            actor.knockdown_ticks_remaining = KNOCKDOWN_DURATION_TICKS;
            outcome.knockdown_started = true;
        }
        // M1: travel-impulse damage on UNSTABLE actor (CCCP Actor.cpp:1199).
        // STABLE actors do NOT take travel-impulse damage — they only become
        // UNSTABLE first via the stability scalar.
        if matches!(actor.status, Status::Unstable) {
            const TRAVEL_IMPULSE_THRESHOLD: f32 = 100.0;
            const GIB_IMPULSE_LIMIT: f32 = 1000.0;
            let impulse = outcome.landed_impulse.max(outcome.recoil_applied);
            if impulse > TRAVEL_IMPULSE_THRESHOLD {
                let raw = (impulse - TRAVEL_IMPULSE_THRESHOLD)
                    / (GIB_IMPULSE_LIMIT - TRAVEL_IMPULSE_THRESHOLD);
                let damage = (raw * actor.hp_max).max(0.0).min(actor.hp_max);
                let _ = actor.apply_damage(damage);
            }
        }

        // Tick down knockdown recovery.
        if actor.knockdown_ticks_remaining > 0 {
            actor.knockdown_ticks_remaining -= 1;
            if actor.knockdown_ticks_remaining == 0 && was_in_knockdown {
                outcome.knockdown_recovered = true;
            }
        }

        // M1: DYING dwell countdown (CCCP Actor.cpp:1229). When dwell expires,
        // transition to DEAD (unless mission_critical, which caps at Downed).
        if matches!(actor.status, Status::Dying) && actor.dying_dwell_ticks_remaining > 0 {
            actor.dying_dwell_ticks_remaining -= 1;
            if actor.dying_dwell_ticks_remaining == 0 && !actor.mission_critical {
                actor.status = Status::Dead;
                outcome.dying_dwell_elapsed = true;
                outcome.new_status = Status::Dead;
                // Surface the lethal cause so the engine can parent the
                // dwell-elapsed DEAD event to the projectile_hit even though
                // the kill happened ticks earlier (Gap C3).
                outcome.lethal_cause_event_id = actor.last_lethal_cause_event_id.clone();
            }
        }

        // M1: one-shot inventory drop on DYING entry. Captures hand position
        // and an outward toss velocity for the engine to emit on the recorder.
        if matches!(actor.status, Status::Dying) && !actor.inventory_dropped_on_dying {
            let label = actor.inventory.selected_item().label().to_string();
            let drop_pos = Vec2::new(
                actor.position.x + actor.aim.x * actor.half_extents.x,
                actor.position.y + actor.half_extents.y * 0.5,
            );
            let toss_vel = Vec2::new(actor.aim.x * 30.0 + actor.velocity.x * 0.25, 60.0);
            outcome.entered_dying = true;
            outcome.inventory_drop_position = Some(drop_pos);
            outcome.inventory_drop_velocity = Some(toss_vel);
            outcome.inventory_drop_label = Some(label);
            // Gap C3: surface the latched lethal cause id so the engine emits
            // inventory_dropped + actor_status_changed(DYING) with the right
            // parent_event_id chain.
            if outcome.lethal_cause_event_id.is_none() {
                outcome.lethal_cause_event_id = actor.last_lethal_cause_event_id.clone();
            }
            // Replace inventory slots with Empty so subsequent fire intent is
            // rejected ("no weapon"). We retain the slot count so observation
            // shapes remain stable.
            for slot in &mut actor.inventory.items {
                *slot = InventoryItem::Empty;
            }
            actor.inventory_dropped_on_dying = true;
        }
    }

    outcome
}

fn step_projectiles(state: &mut ActorSimState, deps: StepDeps, report: &mut StepReport) {
    let mut survivors: Vec<Projectile> = Vec::with_capacity(state.projectiles.len());
    let projectiles = std::mem::take(&mut state.projectiles);
    for mut projectile in projectiles {
        let start = projectile.position;
        let end = Vec2::new(
            projectile.position.x + projectile.velocity.x * deps.tick_dt,
            projectile.position.y + projectile.velocity.y * deps.tick_dt,
        );
        projectile.position = end;
        if projectile.remaining_ticks > 0 {
            projectile.remaining_ticks -= 1;
        }
        // Swept segment-vs-AABB so fast projectiles cannot tunnel through actors that
        // sit between two sampled positions; we pick the earliest entry along the segment
        // (BTreeMap iteration order breaks ties by ActorId for determinism).
        let mut hit_target: Option<(ActorId, Vec2, f32, f32)> = None;
        for actor in state.world.actors.values() {
            if actor.id == projectile.owner {
                continue;
            }
            if actor.status.is_dead() {
                continue;
            }
            if let Some(t) = segment_hits_aabb(start, end, actor.position, actor.half_extents) {
                let hit_pos = Vec2::new(start.x + (end.x - start.x) * t, start.y + (end.y - start.y) * t);
                match hit_target {
                    Some((_, _, _, best_t)) if t >= best_t => {}
                    _ => hit_target = Some((actor.id, hit_pos, projectile.damage, t)),
                }
            }
        }
        let hit_target = hit_target.map(|(id, pos, dmg, _)| (id, pos, dmg));
        if let Some((target_id, hit_pos, damage)) = hit_target {
            let target = state
                .world
                .actors
                .get_mut(&target_id)
                .expect("hit target must exist by construction");
            let previous_status = target.status;
            // **M5**: when the target has a chassis, route through layered armor;
            // otherwise fall back to the legacy flat-HP path so M1.5 and pre-M5
            // scenarios keep their hit semantics.
            let (chassis_outcome, zone_label) = if target.chassis.is_some() {
                let zone = zone_from_hit(target.position, target.half_extents, hit_pos);
                let (_, outcome) = target.apply_zone_damage(zone, damage, "projectile_hit");
                (Some(outcome), zone.as_str().to_string())
            } else {
                let _ = target.apply_damage(damage);
                (None, "torso".to_string())
            };
            let new_status = target.status;
            report.hits.push(HitOutcome {
                projectile_id: projectile.id,
                shooter: projectile.owner,
                target: target_id,
                hit_position: hit_pos,
                damage,
                previous_status,
                new_status,
                zone: zone_label,
                chassis_outcome,
            });
            continue;
        }
        let oob = projectile.position.x < deps.region_min_x - 64.0
            || projectile.position.x > deps.region_max_x + 64.0
            || projectile.position.y < state.world.floor_y - 64.0
            || projectile.position.y > deps.region_max_y + 64.0;
        if oob || projectile.remaining_ticks == 0 {
            report.expired_projectiles.push(ExpiredProjectile {
                id: projectile.id,
                owner: projectile.owner,
                last_position: projectile.position,
            });
            continue;
        }
        survivors.push(projectile);
    }
    state.projectiles = survivors;
}

/// Returns the entry parameter `t` in `[0, 1]` for the segment `start -> end` against the
/// AABB centred on `centre` with `half_extents`, or `None` if the segment misses. A point
/// already inside the AABB at `start` returns `Some(0.0)`.
///
/// **DR-033 forward-hook**: thin `Vec2` adapter that delegates to
/// `cf_physics::segment_hits_aabb` so M5.5's broadphase/narrowphase can build on
/// the shared swept primitive without depending on `cf-actor`.
fn segment_hits_aabb(start: Vec2, end: Vec2, centre: Vec2, half_extents: Vec2) -> Option<f32> {
    cf_physics::segment_hits_aabb(
        (start.x, start.y),
        (end.x, end.y),
        (centre.x, centre.y),
        (half_extents.x, half_extents.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ActorState, Inventory};
    use cf_equipment::{rifle_preset, RIFLE_M1_DEFAULT_ID};

    fn setup() -> (ActorSimState, BTreeMap<ActorId, ControlIntent>) {
        let mut world = ActorWorld::new(0.0, -980.0);
        let inv = Inventory::with_rifle(RIFLE_M1_DEFAULT_ID);
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::new(50.0, 16.0), 100.0, inv);
        actor.on_ground = true;
        world.insert(actor);
        // Target dummy at x=400. Damage of 12 * 9 hits = 108 > 100 so it can be killed.
        let dummy_inv = Inventory::default();
        let mut dummy = ActorState::player(ActorId(2), "red", Vec2::new(400.0, 16.0), 100.0, dummy_inv);
        dummy.controllable = false;
        dummy.on_ground = true;
        world.insert(dummy);

        let mut state = ActorSimState::new(world);
        state.ensure_rifle_for(
            ActorId(1),
            RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap(), 60),
        );
        let intents = BTreeMap::new();
        (state, intents)
    }

    fn deps() -> StepDeps {
        StepDeps {
            tick_dt: 1.0 / 60.0,
            region_min_x: 0.0,
            region_max_x: 1280.0,
            region_max_y: 720.0,
            auto_reload_when_empty: false,
        }
    }

    #[test]
    fn idle_actor_just_settles() {
        let (mut state, mut intents) = setup();
        let report = step_no_rng(&mut state, &mut intents, deps());
        assert_eq!(report.actor_outcomes.len(), 2);
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(!player.fired);
        assert!(!player.jump_accepted);
    }

    #[test]
    fn move_intent_advances_position() {
        let (mut state, mut intents) = setup();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                move_x: 1.0,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let _ = step_no_rng(&mut state, &mut intents, deps());
        let actor = state.world.actors.get(&ActorId(1)).unwrap();
        assert!(actor.position.x > 50.0);
    }

    #[test]
    fn jump_only_works_on_ground() {
        let (mut state, mut intents) = setup();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                jump: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(player.jump_accepted);

        // In the air, a second jump intent must be refused.
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                jump: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report2 = step_no_rng(&mut state, &mut intents, deps());
        let player2 = report2.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(!player2.jump_accepted, "no double-jump in M1");
    }

    #[test]
    fn fire_spawns_projectile_with_recoil() {
        let (mut state, mut intents) = setup();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Cfctl)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        assert_eq!(report.spawned_projectiles.len(), 1);
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(player.fired);
        assert!(player.recoil_applied > 0.0);
        let actor = state.world.actors.get(&ActorId(1)).unwrap();
        // Recoil pushed the firer slightly backwards; aim is +x so velocity_x went negative.
        assert!(actor.velocity.x < 0.0);
    }

    #[test]
    fn projectile_eventually_hits_dummy_and_can_kill_it() {
        let (mut state, mut intents) = setup();
        // Fire 9 shots (12 dmg * 9 = 108 > 100 hp on the dummy). Allow ~120 ticks for travel.
        let mut shots_fired = 0;
        let mut hits = 0;
        let fire_interval_ticks = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().fire_interval_ticks(60);
        for tick in 0..240u32 {
            let intent = if tick % fire_interval_ticks == 0 && shots_fired < 9 {
                shots_fired += 1;
                ControlIntent {
                    actor: ActorId(1),
                    fire: true,
                    ..ControlIntent::new(ActorId(1), IntentSource::Cfctl)
                }
            } else {
                ControlIntent::new(ActorId(1), IntentSource::Cfctl)
            };
            intents.insert(ActorId(1), intent);
            let report = step_no_rng(&mut state, &mut intents, deps());
            hits += report.hits.len();
        }
        assert!(hits >= 9, "all 9 shots must connect; got {hits}");
        let dummy = state.world.actors.get(&ActorId(2)).unwrap();
        // CCCP Actor.cpp:1229 — HP=0 enters DYING dwell first. After the
        // 60-tick dwell at 60Hz the status advances to DEAD; the loop above
        // runs 240 ticks, well past the dwell.
        assert!(
            matches!(dummy.status, Status::Dying | Status::Dead),
            "dummy must enter DYING (or DEAD after dwell); got {:?}",
            dummy.status
        );
    }

    #[test]
    fn fast_projectile_hits_actor_via_swept_segment() {
        // Regression: at the M1 rifle's 1200 units/s a projectile travels 20 units per
        // tick, which is wider than the default 16-unit actor AABB. Without a swept
        // segment-vs-AABB test the projectile point can step over an actor between two
        // sampled positions and miss entirely. Place a dummy whose AABB sits cleanly
        // between two consecutive projectile points so a non-swept check would tunnel.
        let mut world = ActorWorld::new(0.0, -980.0);
        let inv = Inventory::with_rifle(RIFLE_M1_DEFAULT_ID);
        let mut shooter = ActorState::player(ActorId(1), "blue", Vec2::new(50.0, 16.0), 100.0, inv);
        shooter.on_ground = true;
        world.insert(shooter);
        // Muzzle x = 50 + 12 = 62, +20/tick. After 16 ticks the projectile is at x=382;
        // after 17 ticks it is at x=402. An AABB centred at x=391 spans [383, 399], which
        // both sampled points miss but the segment crosses.
        let mut dummy = ActorState::player(ActorId(2), "red", Vec2::new(391.0, 16.0), 100.0, Inventory::default());
        dummy.controllable = false;
        dummy.on_ground = true;
        world.insert(dummy);
        let mut state = ActorSimState::new(world);
        state.ensure_rifle_for(
            ActorId(1),
            RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap(), 60),
        );
        let mut intents = BTreeMap::new();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Cfctl)
            },
        );
        let mut total_hits = 0;
        for _ in 0..40 {
            let report = step_no_rng(&mut state, &mut intents, deps());
            total_hits += report.hits.len();
            intents.insert(ActorId(1), ControlIntent::new(ActorId(1), IntentSource::Cfctl));
        }
        assert_eq!(
            total_hits, 1,
            "swept segment must register the otherwise-tunneling shot"
        );
    }

    #[test]
    fn dead_actor_does_not_accept_input() {
        let (mut state, mut intents) = setup();
        if let Some(a) = state.world.actors.get_mut(&ActorId(1)) {
            a.hp = 0.0;
            a.status = Status::Dead;
        }
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                move_x: 1.0,
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        // Dead actors don't fire.
        assert!(!player.fired);
        assert_eq!(player.move_x, 0.0);
    }

    #[test]
    fn reset_returns_actor_to_spawn_and_reloads_rifle() {
        let (mut state, mut intents) = setup();
        // Fire once to consume ammo.
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let _ = step_no_rng(&mut state, &mut intents, deps());
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                reset: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(player.reset);
        let actor = state.world.actors.get(&ActorId(1)).unwrap();
        assert_eq!(actor.position, actor.spawn);
        let rifle = state.rifles.get(&ActorId(1)).unwrap();
        let mag_capacity = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().mag_capacity;
        assert_eq!(rifle.ammo_in_mag, mag_capacity);
    }

    #[test]
    fn reset_clears_resetting_actors_inflight_projectiles() {
        // Regression: `intent.reset` must mirror `ControlCommand::ScenarioReset`
        // and clear the resetting actor's own pre-reset projectiles. Without this,
        // a player who fires and then immediately resets in the next tick can have
        // their pre-reset shot hit the dummy after respawn, which contradicts the
        // user-facing semantics of "reset to spawn with full HP / ammo".
        let (mut state, mut intents) = setup();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let _ = step_no_rng(&mut state, &mut intents, deps());
        assert_eq!(state.projectiles.len(), 1, "fire must spawn one projectile");
        let projectile_id = state.projectiles[0].id;
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                reset: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        assert!(
            state.projectiles.iter().all(|p| p.owner != ActorId(1)),
            "reset must drain the resetting actor's own projectiles"
        );
        assert!(
            report
                .expired_projectiles
                .iter()
                .any(|e| e.id == projectile_id && e.owner == ActorId(1)),
            "every cleared projectile must be reported as expired so the event log balances spawned and expired"
        );
    }

    #[test]
    fn jump_impulse_uses_actor_tuning_field() {
        // Regression: the jump apply path used to hardcode 420.0 instead of reading
        // ActorTuning::jump_impulse, so any tuning change was silently dead code.
        // We assert post-jump vy equals ActorTuning::jump_impulse minus the one-tick
        // gravity decay (apply_jump runs before step_kinematics inside step_one_actor,
        // so the post-tick vy reflects exactly one frame of gravity).
        let (mut state, mut intents) = setup();
        let dep = deps();
        let gravity_step = state.world.gravity * dep.tick_dt;
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                jump: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let _ = step_no_rng(&mut state, &mut intents, dep);
        let actor = state.world.actors.get(&ActorId(1)).unwrap();
        let expected = ActorTuning::default().jump_impulse + gravity_step;
        assert!(
            (actor.velocity.y - expected).abs() < 1e-3,
            "post-jump vy must equal ActorTuning::jump_impulse ({}) minus one tick of gravity (={expected}); got {}",
            ActorTuning::default().jump_impulse,
            actor.velocity.y
        );
    }

    #[test]
    fn fire_blocked_when_selected_slot_is_not_rifle() {
        // M1-FIX-11 regression: selecting an empty slot must gate fire/reload so
        // inventory selection drives gameplay, not just the HUD.
        let (mut state, mut intents) = setup();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                selected_item: Some(ItemSlot(1)),
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let _ = step_no_rng(&mut state, &mut intents, deps());
        // Now the selected slot is 1 (Empty). Pressing fire must NOT spawn a projectile
        // even though the actor still owns the rifle in slot 0.
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        assert!(
            report.spawned_projectiles.is_empty(),
            "fire must be gated on selected slot"
        );
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(!player.fired);
        // Ammo should still be untouched.
        let mag = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().mag_capacity;
        assert_eq!(state.rifles.get(&ActorId(1)).unwrap().ammo_in_mag, mag);
    }

    #[test]
    fn vertical_aim_produces_vertical_projectile_velocity() {
        // M1-FIX-12 regression: aim straight up must yield (0, +speed) projectile
        // velocity, not (0, 0). Diagonal aim must scale both components.
        use crate::Vec2;
        let (mut state, mut intents) = setup();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                aim: Vec2::new(0.0, 1.0),
                fire: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        assert_eq!(report.spawned_projectiles.len(), 1);
        let projectile = &report.spawned_projectiles[0];
        let speed = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().projectile_speed;
        assert!(
            projectile.velocity.x.abs() < 1e-3,
            "vertical aim must zero out vx, got {}",
            projectile.velocity.x
        );
        assert!(
            (projectile.velocity.y - speed).abs() < 1e-3,
            "vertical aim must produce vy = +speed, got {}",
            projectile.velocity.y
        );
    }

    #[test]
    fn reset_restores_selected_slot_to_default() {
        // M1-FIX-8 regression: act.player.reset must restore selected slot back to
        // slot 0 so the actor can fire again after being reset.
        let (mut state, mut intents) = setup();
        // Select slot 1 (Empty).
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                selected_item: Some(ItemSlot(1)),
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let _ = step_no_rng(&mut state, &mut intents, deps());
        assert_eq!(
            state.world.actors.get(&ActorId(1)).unwrap().inventory.selected,
            ItemSlot(1)
        );
        // Reset.
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                reset: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let _ = step_no_rng(&mut state, &mut intents, deps());
        assert_eq!(
            state.world.actors.get(&ActorId(1)).unwrap().inventory.selected,
            ItemSlot(0),
            "reset must clear selected back to slot 0 so the rifle is firable again"
        );
    }

    #[test]
    fn checksum_is_deterministic() {
        let (mut a, mut a_int) = setup();
        let (mut b, mut b_int) = setup();
        for tick in 0..30 {
            let intent = if tick % 6 == 0 {
                ControlIntent {
                    actor: ActorId(1),
                    move_x: 1.0,
                    fire: true,
                    ..ControlIntent::new(ActorId(1), IntentSource::Human)
                }
            } else {
                ControlIntent {
                    actor: ActorId(1),
                    move_x: 1.0,
                    ..ControlIntent::new(ActorId(1), IntentSource::Human)
                }
            };
            a_int.insert(ActorId(1), intent.clone());
            b_int.insert(ActorId(1), intent);
            let _ = step_no_rng(&mut a, &mut a_int, deps());
            let _ = step_no_rng(&mut b, &mut b_int, deps());
        }
        assert_eq!(a.checksum_bytes(), b.checksum_bytes());
    }

    #[test]
    fn stability_decreases_on_recoil_and_recovers_on_ground() {
        let (mut state, mut intents) = setup();
        // Start stable.
        let actor = state.world.actors.get(&ActorId(1)).unwrap();
        assert!((actor.stability - 1.0).abs() < 1e-6, "initial stability must be 1.0");

        // Fire: recoil should reduce stability.
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                aim: Vec2::new(1.0, 0.0),
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(player.recoil_applied > 0.0, "rifle must have fired");
        let post_fire = state.world.actors.get(&ActorId(1)).unwrap().stability;
        assert!(post_fire < 1.0, "stability must decrease after recoil, got {post_fire}");

        // Idle ticks on ground: stability should recover.
        for _ in 0..60 {
            let _ = step_no_rng(&mut state, &mut intents, deps());
        }
        let recovered = state.world.actors.get(&ActorId(1)).unwrap().stability;
        assert!(
            recovered > post_fire,
            "stability must recover after idle ground ticks, got {recovered} (was {post_fire})"
        );
    }

    #[test]
    fn edge_triggered_jump_is_not_dropped_within_one_tick() {
        // W1.3 item 862: verify that a jump edge-trigger set and consumed
        // within one step() call is honored (not lost to clear_edges ordering).
        let (mut state, mut intents) = setup();
        // Ensure actor is on ground.
        {
            let actor = state.world.actors.get_mut(&ActorId(1)).unwrap();
            actor.on_ground = true;
            actor.velocity = Vec2::ZERO;
        }
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                jump: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(
            player.jump_accepted,
            "jump edge must be consumed in the same tick it was set"
        );
    }

    #[test]
    fn move_x_inf_rejected_by_engine_guard() {
        // W1.3 item 863: act.player.move already has the is_finite guard in
        // cf-control's server dispatch; this test confirms the actor sim
        // itself never receives non-finite move_x (the guard is upstream).
        // Here we verify the sim doesn't crash on a zero-move intent.
        let (mut state, mut intents) = setup();
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                move_x: 0.0,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        assert!(!report.actor_outcomes.is_empty(), "step must produce outcomes");
    }

    #[test]
    fn climb_intent_produces_climbing_stance() {
        // Item 675: Stance::Climbing consumes climb intent.
        let (mut state, _) = setup();
        {
            let actor = state.world.actors.get_mut(&ActorId(1)).unwrap();
            actor.on_ground = true;
            actor.climb_active = true;
        }
        let stance = state.world.actors.get(&ActorId(1)).unwrap().stance();
        assert_eq!(stance, crate::Stance::Climbing);
    }

    #[test]
    fn crouch_sets_stance_on_ground() {
        // Item 676: crouching stance when on ground.
        let (mut state, _) = setup();
        {
            let actor = state.world.actors.get_mut(&ActorId(1)).unwrap();
            actor.on_ground = true;
            actor.crouch_active = true;
        }
        let stance = state.world.actors.get(&ActorId(1)).unwrap().stance();
        assert_eq!(stance, crate::Stance::Crouching);
    }

    #[test]
    fn projectile_does_not_hit_owner() {
        // Item 678: actor-projectile self-filter (shooter does not hit themselves).
        let (mut state, mut intents) = setup();
        // Fire a projectile.
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                fire: true,
                aim: Vec2::new(1.0, 0.0),
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step_no_rng(&mut state, &mut intents, deps());
        assert!(
            report.actor_outcomes.iter().any(|o| o.fired),
            "must fire a projectile"
        );
        // Run many ticks: the projectile should never hit the actor who fired it.
        for _ in 0..120 {
            let r = step_no_rng(&mut state, &mut intents, deps());
            for hit in &r.hits {
                assert_ne!(hit.target, ActorId(1), "projectile must not hit its owner");
            }
        }
    }
}
