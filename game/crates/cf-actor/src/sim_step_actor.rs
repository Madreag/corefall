//! Per-actor per-tick sim step.
//!
//! Extracted from [`crate::sim`] for file size.

use cf_equipment::{tick_rifle, RifleTickInputs, TickOutcomes as RifleOutcomes};
use cf_physics::{
    apply_horizontal_motion, apply_jump, apply_recoil_with_mass, step_kinematics, HorizontalInputs, JumpInputs,
    StepInputs,
};

use crate::sim::{
    ActorSimState, ActorTickOutcome, ExpiredProjectile, Projectile, SpawnedProjectile, StepDeps, StepReport,
};
use crate::{ActorId, ControlIntent, InventoryItem, Stance, Status, Vec2};

pub(crate) fn step_one_actor<R: FnMut() -> u64>(
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
            reload_ticks_total: 0,
            magazine_index_after_reload: 0,
            fire_denied_reloading: false,
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
            travel_impulse_damage: false,
            fire_denied_by_swap: false,
            bipod_deployed_at_fire: false,
            suppressor_attached_at_fire: false,
            popped_round: None,
            shell_ejection: None,
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
            // movement-contribution multipliers scale max_speed + jump impulse,
            // and the `disables_rifle_when_destroyed` flag gates fire/reload.
            // The `forces_crawl_when_destroyed` and `disables_jet_when_destroyed`
            // flags route through stance derivation + jet command rejection.
            //
            // engine-supplied values (settings-driven cvars) instead of the
            // defaults. None preserves byte-identical behaviour on bundles
            // recorded before the cvar surface existed.
            let mut tuning = deps.tuning.unwrap_or_default();
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
            // grid's load curve (1.0 empty, 0.5 at 100% carry). Pure
            // multiplicative with chassis movement factor + crawl floor
            // so the spec's PARITY-34 (mass_factor) and M6B encumbrance
            // stack deterministically. Falls back to 1.0 when no
            // encumbrance envelope is attached (pre-M6B actors).
            let encumbrance_factor = actor.encumbrance_walk_speed_multiplier();
            let effective_max_speed = if force_crawl {
                tuning.max_speed * 0.25 * encumbrance_factor
            } else {
                tuning.max_speed * move_factor * encumbrance_factor
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

    // Previously the dwell decrement lived inside the post-`accepted_input`
    // block which short-circuits for non-accepting-input actors (Dying /
    // Downed / Dead / Inactive). That meant a Dying actor's dwell never
    // counted down → the DEAD transition relied on the M1 R1 fluke where
    // accepted_input was captured BEFORE apply_damage applied lethal damage
    // (1-tick decrement); a fresh Dying actor (e.g. tutorial-safety
    // synthesised) stays Dying forever. Tick the dwell here so it always
    // counts down, then honour the dying-cap policy (mission_critical /
    // tutorial_safety + controllable).
    if matches!(outcome.previous_status, Status::Dying) || matches!(outcome.new_status, Status::Dying) {
        let dying_cap_in_effect = {
            let actor = state
                .world
                .actors
                .get(&actor_id)
                .expect("actor id exists by construction");
            actor.mission_critical || (deps.tutorial_safety && actor.controllable)
        };
        if let Some(actor) = state.world.actors.get_mut(&actor_id) {
            if matches!(actor.status, Status::Dying) && actor.dying_dwell_ticks_remaining > 0 {
                actor.dying_dwell_ticks_remaining -= 1;
                if actor.dying_dwell_ticks_remaining == 0 && !dying_cap_in_effect {
                    actor.status = Status::Dead;
                    outcome.dying_dwell_elapsed = true;
                    outcome.new_status = Status::Dead;
                    outcome
                        .lethal_cause_event_id
                        .clone_from(&actor.last_lethal_cause_event_id);
                }
            }
        }
    }

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
    // `disables_rifle_when_destroyed=true` in the BodyGraph movement contribution
    // gates the fire path so a player with a blown-off rifle arm cannot keep shooting.
    //
    // Charge / Arc / Auto. The cf-control engine pre-processes Charge holds
    // (accumulating `weapon_charge_fraction` up to
    // [`cf_equipment::SNIPER_CHARGE_MAX_SECONDS`]) and post-processes the
    // fired projectile with [`cf_equipment::charge_damage_multiplier`] +
    // burst-3 follow-up + GL Arc conversion. The sim itself remains M1-shape;
    // the fire-mode hooks live in the engine because they cross multiple
    // sim ticks.
    let (rifle_selected, rifle_disabled_by_limb_loss, weapon_jammed, swap_in_progress) = {
        let actor = state.world.actors.get(&actor_id);
        let selected = actor.is_some_and(|a| a.inventory.selected_item().is_rifle());
        let swap = actor.is_some_and(|a| a.weapon_swap_in_progress);
        let (rifle_off, jammed) = actor.and_then(|a| a.chassis.as_ref()).map_or((false, false), |c| {
            let (_, _, disable_rifle, _, _, _) = c.body_graph.movement_factor(&c.destroyed_zones());
            (disable_rifle, c.weapon_jammed)
        });
        (selected, rifle_off, jammed, swap)
    };
    let can_fire = rifle_selected && !rifle_disabled_by_limb_loss && !weapon_jammed && !swap_in_progress;
    outcome.fire_denied_by_swap = swap_in_progress && (intent.fire || intent.fire_held);
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
    outcome.reload_ticks_total = rifle_outcomes.reload_ticks_total;
    outcome.magazine_index_after_reload = rifle_outcomes.magazine_index_after;
    outcome.fire_denied_reloading = rifle_outcomes.fire_denied_reloading;
    outcome.dry_fire = rifle_outcomes.dry_fire;

    if rifle_outcomes.fired_this_tick {
        outcome.fired = true;
        // recoil scaling so we can attribute the multiplied impulse and
        // loudness to the correct attachment. The bipod bloom multiplier
        // is applied later (after the per-stance multiplier) in the bloom
        // assignment below so it composes with stance + sharp aim + reload
        // contributions instead of being overwritten by the final
        // `outcome.bloom_factor = bloom` assignment.
        let (bipod_recoil_factor, bipod_deployed, suppressor_factor, suppressor_attached) =
            state.world.actors.get(&actor_id).map_or((1.0, false, 1.0, false), |a| {
                (
                    a.bipod.recoil_factor(),
                    a.bipod.equipped && a.bipod.state == cf_equipment::BipodState::Deployed,
                    a.suppressor.loudness_factor(),
                    a.suppressor.attached && a.suppressor.integrity > 0.0,
                )
            });
        let effective_recoil = rifle_outcomes.recoil_impulse_applied * bipod_recoil_factor;
        outcome.recoil_applied = effective_recoil;
        outcome.bipod_deployed_at_fire = bipod_deployed;
        outcome.suppressor_attached_at_fire = suppressor_attached;
        let (spec, max_flight) = state
            .rifles
            .get(&actor_id)
            .map(|r| (r.spec.clone(), r.projectile_max_flight_ticks()))
            .expect("fired rifle must have a state");

        // ejection so the engine can emit equipment.magazine_changed +
        // equipment.shell_ejected. The M1 RifleState.ammo_in_mag decrement
        // remains the source of truth for the tracer cadence (preserved in
        // outcomes.fired_is_tracer); the Magazine struct here exposes the
        // same CCCP `Magazine::PopNextRound` semantics for the new event
        // family while remaining byte-compatible with M1's tracer pattern.
        let (popped_round, shell_ejection) = {
            let _mag_capacity = spec.mag_capacity.max(1);
            let mag_remaining = state.rifles.get(&actor_id).map_or(0, |r| r.ammo_in_mag);
            //   1. `intent.ammo_kind` — per-shot override from cfctl
            //      `act.player.fire { ammo_kind: ... }` (e.g. HEAT / APFSDS
            //      for tank-grade rounds).
            //   2. `spec.primary_round` when it is NOT Regular — every M14C
            //      tank-grade RifleSpec (`rpg_launcher_v1`, `tank_autocannon_t3`)
            //      bakes the canonical primary kind into the preset so the
            //      magazine pops HEAT / APFSDS even when no cfctl override
            //      is provided.
            //   3. Tracer cadence — preserves M1 byte-identical Tracer vs
            //      Regular interleave per `tracer_round_to_total_ratio`.
            let round_kind = if let Some(kind) = intent.ammo_kind {
                kind
            } else if spec.primary_round != cf_equipment::RoundKind::Regular {
                if rifle_outcomes.fired_is_tracer {
                    cf_equipment::RoundKind::Tracer
                } else {
                    spec.primary_round
                }
            } else if rifle_outcomes.fired_is_tracer {
                cf_equipment::RoundKind::Tracer
            } else {
                cf_equipment::RoundKind::Regular
            };
            let popped = cf_equipment::PoppedRound {
                round_kind,
                remaining_in_mag: mag_remaining,
            };
            let shell_id = state.allocate_projectile_id();
            let facing_sign =
                state
                    .world
                    .actors
                    .get(&actor_id)
                    .map_or(1.0_f32, |a| if a.aim.x >= 0.0 { 1.0_f32 } else { -1.0_f32 });
            let position = state.world.actors.get(&actor_id).map_or(Vec2::ZERO, |a| a.position);
            let shell = cf_equipment::ShellEjection::default_for(
                cf_equipment::ShellKind::Rifle,
                shell_id,
                position.x,
                position.y + 4.0,
                facing_sign,
            );
            (popped, shell)
        };
        outcome.popped_round = Some(popped_round);
        outcome.shell_ejection = Some(shell_ejection);

        // Reborrow actor briefly to apply recoil + read aim/position.
        let (muzzle, aim, base_velocity, damage) = {
            let actor = state
                .world
                .actors
                .get_mut(&actor_id)
                .expect("actor id exists by construction");
            // M1 re-audit (2026-05-13): use mass-aware F=ma form.
            // mass_kg=80 (baseline) → same Δv as legacy apply_recoil.
            // mass_kg=160 (heavy) → half the Δv; mass_kg=40 (light) → 2× Δv.
            actor.velocity.x = apply_recoil_with_mass(actor.velocity.x, actor.aim.x, effective_recoil, actor.mass_kg);
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
            // `inherits_firer_velocity=true`, the projectile's initial
            // velocity is the muzzle vector PLUS the FULL actor velocity
            // (per CCCP HDFirearm.cpp:752). Pre-fix used `0.5` (half),
            // which the spec does not authorize.
            let inherit_fraction = if spec.inherits_firer_velocity { 1.0_f32 } else { 0.0_f32 };
            let base_velocity = Vec2::new(
                aim.x * spec.projectile_speed + actor.velocity.x * inherit_fraction,
                aim.y * spec.projectile_speed + actor.velocity.y * inherit_fraction,
            );
            (muzzle, aim, base_velocity, spec.damage_per_hit)
        };
        outcome.muzzle_origin = Some(muzzle);
        // Loudness radius (CCCP HDFirearm.cpp:948): scaled by spec.loudness too.
        // [`cf_equipment::SUPPRESSOR_LOUDNESS_FACTOR`].
        let loudness_radius = 480.0_f32 * (damage / 10.0).clamp(1.0, 3.0) * spec.loudness.max(0.1) * suppressor_factor;
        outcome.loudness_radius = loudness_radius;

        // Per-particle spawn loop. Particle count >= 1; >1 produces a spread
        // cone of `spread_radians` around the aim direction. Each particle
        // gets a unique projectile_id. The tracer flag from `tick_rifle`
        // applies to ALL particles of this shot (CCCP `Round.RTTRatio` is
        // per-shot, not per-particle).
        let particle_count = spec.particle_count.max(1);
        // adds 0.1 rad mount_motion penalty when riding a moving critter.
        let mount_spread_bonus = {
            let actor_for_mount = state.world.actors.get(&actor_id);
            match actor_for_mount.and_then(|a| a.mount) {
                Some(m) => {
                    let critter_speed = state
                        .world
                        .actors
                        .get(&m.critter_id)
                        .map(|c| (c.velocity.x.powi(2) + c.velocity.y.powi(2)).sqrt())
                        .unwrap_or(0.0);
                    if critter_speed > crate::mount::DISMOUNT_STATIONARY_SPEED_THRESHOLD {
                        crate::mount::MOUNT_MOTION_AIM_SPREAD_RAD
                    } else {
                        0.0
                    }
                }
                None => 0.0,
            }
        };
        let half_spread = (spec.spread_radians + mount_spread_bonus) * 0.5;
        // Angle of the base aim (radians).
        let base_angle = aim.y.atan2(aim.x);
        let base_speed = (base_velocity.x * base_velocity.x + base_velocity.y * base_velocity.y).sqrt();
        // (CCCP HDFirearm.cpp:752). Same fix as the single-particle path
        // above; kept duplicated because the multi-particle path computes
        // base_velocity independently.
        let inherit_fraction = if spec.inherits_firer_velocity { 1.0_f32 } else { 0.0_f32 };
        // Reborrow actor.velocity for inheritance addition per particle.
        let actor_velocity = state.world.actors.get(&actor_id).map_or(Vec2::ZERO, |a| a.velocity);
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
            let speed = if base_speed > 0.0 {
                base_speed
            } else {
                spec.projectile_speed
            };
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
                mass_kg: spec.bullet_mass_kg,
                sharpness: spec.bullet_sharpness,
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
                round_kind: popped_round.round_kind,
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
            let sign = if actor.recoil_alternation_sign >= 0 { 1.0 } else { -1.0 };
            let contribution = effective_recoil / 100.0;
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
        // Gap F3: prefer engine-supplied `deps.tuning.recoil_decay_per_tick`
        // when present so cfctl `act.settings.set { recoil_decay_per_tick: ... }`
        // takes effect immediately without per-actor patching.
        let decay_rate = deps.tuning.map_or(actor.recoil_decay_rate, |t| t.recoil_decay_per_tick);
        if outcome.recoil_applied == 0.0 && actor.recoil_accumulator.abs() > 1e-4 {
            let decay = decay_rate * actor.recoil_accumulator.signum();
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
        // Gap F3: prefer engine-supplied tuning when present.
        let rifle_equipped_for_sharp = actor.inventory.selected_item().is_rifle();
        let horizontal_speed = actor.velocity.x.abs();
        let walk_threshold = deps.tuning.map_or(actor.walk_threshold, |t| t.walk_threshold);
        let sharp_aim_build_ticks_eff = deps
            .tuning
            .map_or(actor.sharp_aim_build_ticks, |t| t.sharp_aim_build_ticks);
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
            let build_step = if sharp_aim_build_ticks_eff > 0 {
                1.0 / (sharp_aim_build_ticks_eff as f32)
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
        // airborne (falling) = 3.0×. Sharp aim multiplies the bloom by
        // (1 - 0.6 * sharp_aim_progress) so a full sharp aim cuts the reticle
        // down to 40% of its baseline.
        //
        // ground) gets the 7× multiplier — the spec literally distinguishes
        // "jumping = 7×" from "airborne = 3×". Implementation distinguishes
        // via `velocity.y > 0`: actor is rising = jumping; otherwise =
        // descending/falling. Previously the airborne branch returned 3.0
        // uniformly which collapsed both states.
        let speed = actor.velocity.x.abs();
        let is_jumping = !actor.on_ground && actor.velocity.y > 0.0;
        let mut bloom: f32 = if is_jumping {
            7.0
        } else if !actor.on_ground {
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
        // M6: per-stance bloom multiplier (crouch=0.6×, prone=0.4×, etc.).
        bloom *= crate::stance::stance_bloom_factor(actor.stance());
        // M6: deployed bipod attenuates bloom by BIPOD_BLOOM_FACTOR (0.5).
        // Applied here (before the final assignments below) so the
        // attenuation composes with the stance multiplier and survives
        // through to `actor.bloom_factor` + `outcome.bloom_factor`.
        if actor.bipod.equipped && actor.bipod.state == cf_equipment::BipodState::Deployed {
            bloom *= cf_equipment::BIPOD_BLOOM_FACTOR;
        }
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
        //
        // captured BEFORE this block so the engine missed the transition
        // and never emitted `actor.actor_status_changed cause="travel_impulse"`.
        // Re-sync new_status AND latch `travel_impulse_damage=true` so the
        // engine knows to use the travel_impulse cause + emit BodyHit audio.
        if matches!(actor.status, Status::Unstable) {
            const TRAVEL_IMPULSE_THRESHOLD: f32 = 100.0;
            const GIB_IMPULSE_LIMIT: f32 = 1000.0;
            let impulse = outcome.landed_impulse.max(outcome.recoil_applied);
            if impulse > TRAVEL_IMPULSE_THRESHOLD {
                let raw = (impulse - TRAVEL_IMPULSE_THRESHOLD) / (GIB_IMPULSE_LIMIT - TRAVEL_IMPULSE_THRESHOLD);
                let damage = (raw * actor.hp_max).max(0.0).min(actor.hp_max);
                if actor.apply_damage(damage).is_some() {
                    outcome.travel_impulse_damage = true;
                    outcome.new_status = actor.status;
                }
            }
        }

        // Tick down knockdown recovery.
        if actor.knockdown_ticks_remaining > 0 {
            actor.knockdown_ticks_remaining -= 1;
            if actor.knockdown_ticks_remaining == 0 && was_in_knockdown {
                outcome.knockdown_recovered = true;
            }
        }

        // unconditional pre-return block at the top of step_one_actor's
        // post-pass phase so dying actors (which don't accept input) still
        // tick their dwell. The dwell update therefore lands BEFORE this
        // block ran historically.

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
                outcome
                    .lethal_cause_event_id
                    .clone_from(&actor.last_lethal_cause_event_id);
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
