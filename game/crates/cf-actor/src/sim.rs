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

use crate::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Status, Vec2};

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
    pub spawned_event_id: Option<String>,
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
            out.extend_from_slice(&quantize(p.position.x).to_le_bytes());
            out.extend_from_slice(&quantize(p.position.y).to_le_bytes());
            out.extend_from_slice(&quantize(p.velocity.x).to_le_bytes());
            out.extend_from_slice(&quantize(p.velocity.y).to_le_bytes());
            out.extend_from_slice(&p.remaining_ticks.to_le_bytes());
        }
        out
    }
}

fn quantize(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1024.0).round() as i32
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
    pub spawned_event_id: Option<String>,
}

/// Spawned projectile metadata for the recorder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpawnedProjectile {
    pub id: u64,
    pub owner: ActorId,
    pub origin: Vec2,
    pub velocity: Vec2,
    pub damage: f32,
}

/// Projectile that flew off the map / outlasted its budget without hitting anything.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExpiredProjectile {
    pub id: u64,
    pub owner: ActorId,
    pub last_position: Vec2,
    pub spawned_event_id: Option<String>,
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
pub fn step(state: &mut ActorSimState, intents: &mut BTreeMap<ActorId, ControlIntent>, deps: StepDeps) -> StepReport {
    let mut report = StepReport::default();

    let actor_ids: Vec<ActorId> = state.world.actors.keys().copied().collect();
    for actor_id in actor_ids {
        let intent = intents
            .remove(&actor_id)
            .unwrap_or_else(|| ControlIntent::new(actor_id, IntentSource::Cfctl));
        let outcome = step_one_actor(state, actor_id, intent, deps, &mut report);
        report.actor_outcomes.push(outcome);
    }

    step_projectiles(state, deps, &mut report);

    report
}

fn step_one_actor(
    state: &mut ActorSimState,
    actor_id: ActorId,
    intent: ControlIntent,
    deps: StepDeps,
    report: &mut StepReport,
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
        let accepted_input = actor.status.accepts_input();
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
            let tuning = ActorTuning::default();
            if accepted_input && intent.jump {
                let (new_vy, accepted) = apply_jump(JumpInputs {
                    velocity_y: actor.velocity.y,
                    on_ground: actor.on_ground,
                    jump_impulse: tuning.jump_impulse,
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
                max_speed: tuning.max_speed,
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
        return outcome;
    }
    if !pass.accepted_input {
        return outcome;
    }

    // Tick the rifle (separate borrow from the actor world). Fire/reload intent only
    // applies when the actor's currently selected inventory slot is the rifle; otherwise
    // the rifle still ticks (so cooldowns advance) but ignores the pressed edges.
    let rifle_selected = state
        .world
        .actors
        .get(&actor_id)
        .is_some_and(|a| a.inventory.selected_item().is_rifle());
    let rifle_outcomes = if let Some(rifle) = state.rifles.get_mut(&actor_id) {
        let inputs = RifleTickInputs {
            fire_pressed: intent.fire && rifle_selected,
            reload_pressed: intent.reload && rifle_selected,
            auto_reload_when_empty: deps.auto_reload_when_empty && rifle_selected,
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
        let (muzzle, velocity, damage) = {
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
            let muzzle = Vec2::new(
                actor.position.x + aim.x.signum() * spec.muzzle_forward_offset,
                actor.position.y + spec.muzzle_vertical_offset,
            );
            let velocity = Vec2::new(aim.x * spec.projectile_speed, aim.y * spec.projectile_speed);
            (muzzle, velocity, spec.damage_per_hit)
        };
        outcome.muzzle_origin = Some(muzzle);
        let projectile_id = state.allocate_projectile_id();
        state.projectiles.push(Projectile {
            id: projectile_id,
            owner: actor_id,
            origin: muzzle,
            position: muzzle,
            velocity,
            damage,
            remaining_ticks: max_flight,
            spawned_event_id: None,
        });
        report.spawned_projectiles.push(SpawnedProjectile {
            id: projectile_id,
            owner: actor_id,
            origin: muzzle,
            velocity,
            damage,
        });
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
            let _ = target.apply_damage(damage);
            let new_status = target.status;
            report.hits.push(HitOutcome {
                projectile_id: projectile.id,
                shooter: projectile.owner,
                target: target_id,
                hit_position: hit_pos,
                damage,
                previous_status,
                new_status,
                spawned_event_id: projectile.spawned_event_id.clone(),
            });
            continue;
        }
        let oob = projectile.position.x < deps.region_min_x - 64.0
            || projectile.position.x > deps.region_max_x + 64.0
            || projectile.position.y < state.world.floor_y - 64.0
            || projectile.position.y > state.world.floor_y + 4096.0;
        if oob || projectile.remaining_ticks == 0 {
            report.expired_projectiles.push(ExpiredProjectile {
                id: projectile.id,
                owner: projectile.owner,
                last_position: projectile.position,
                spawned_event_id: projectile.spawned_event_id.clone(),
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
fn segment_hits_aabb(start: Vec2, end: Vec2, centre: Vec2, half_extents: Vec2) -> Option<f32> {
    let min_x = centre.x - half_extents.x;
    let max_x = centre.x + half_extents.x;
    let min_y = centre.y - half_extents.y;
    let max_y = centre.y + half_extents.y;
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let mut t_near = f32::NEG_INFINITY;
    let mut t_far = f32::INFINITY;
    if dx.abs() <= f32::EPSILON {
        if start.x < min_x || start.x > max_x {
            return None;
        }
    } else {
        let t1 = (min_x - start.x) / dx;
        let t2 = (max_x - start.x) / dx;
        let (lo, hi) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
        t_near = t_near.max(lo);
        t_far = t_far.min(hi);
    }
    if dy.abs() <= f32::EPSILON {
        if start.y < min_y || start.y > max_y {
            return None;
        }
    } else {
        let t1 = (min_y - start.y) / dy;
        let t2 = (max_y - start.y) / dy;
        let (lo, hi) = if t1 <= t2 { (t1, t2) } else { (t2, t1) };
        t_near = t_near.max(lo);
        t_far = t_far.min(hi);
    }
    if t_near > t_far || t_far < 0.0 || t_near > 1.0 {
        return None;
    }
    Some(t_near.clamp(0.0, 1.0))
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
            auto_reload_when_empty: false,
        }
    }

    #[test]
    fn idle_actor_just_settles() {
        let (mut state, mut intents) = setup();
        let report = step(&mut state, &mut intents, deps());
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
        let _ = step(&mut state, &mut intents, deps());
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
        let report = step(&mut state, &mut intents, deps());
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
        let report2 = step(&mut state, &mut intents, deps());
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
        let report = step(&mut state, &mut intents, deps());
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
            let report = step(&mut state, &mut intents, deps());
            hits += report.hits.len();
        }
        assert!(hits >= 9, "all 9 shots must connect; got {hits}");
        let dummy = state.world.actors.get(&ActorId(2)).unwrap();
        assert!(dummy.status.is_dead(), "dummy hp should drop to zero");
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
            let report = step(&mut state, &mut intents, deps());
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
        let report = step(&mut state, &mut intents, deps());
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
        let _ = step(&mut state, &mut intents, deps());
        intents.insert(
            ActorId(1),
            ControlIntent {
                actor: ActorId(1),
                reset: true,
                ..ControlIntent::new(ActorId(1), IntentSource::Human)
            },
        );
        let report = step(&mut state, &mut intents, deps());
        let player = report.actor_outcomes.iter().find(|o| o.actor == ActorId(1)).unwrap();
        assert!(player.reset);
        let actor = state.world.actors.get(&ActorId(1)).unwrap();
        assert_eq!(actor.position, actor.spawn);
        let rifle = state.rifles.get(&ActorId(1)).unwrap();
        let mag_capacity = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap().mag_capacity;
        assert_eq!(rifle.ammo_in_mag, mag_capacity);
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
        let _ = step(&mut state, &mut intents, dep);
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
        let _ = step(&mut state, &mut intents, deps());
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
        let report = step(&mut state, &mut intents, deps());
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
        let report = step(&mut state, &mut intents, deps());
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
        let _ = step(&mut state, &mut intents, deps());
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
        let _ = step(&mut state, &mut intents, deps());
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
            let _ = step(&mut a, &mut a_int, deps());
            let _ = step(&mut b, &mut b_int, deps());
        }
        assert_eq!(a.checksum_bytes(), b.checksum_bytes());
    }
}
