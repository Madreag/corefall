//! drive_tick — per-tick orchestrator.
//! drive_tick orchestrator. >2000 LOC by design: per-tick hot path; keeping
//! every phase inline lets the compiler keep `state: RwLockWriteGuard` in one
//! borrow scope, avoids re-locking, and inlines the sequential phase code.
//! Refactor only if benchmarked p99 stays ≤4 ms.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use cf_actor::sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport};
use cf_actor::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig,
    ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::engine::*;
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {
    pub fn drive_tick(&self) -> Option<Tick> {
        let start = Instant::now();
        let mut state = self.state.write().expect("engine state poisoned");
        let effective_pct = effective_sim_speed_pct(&state.settings, &state.pie_menu, state.multiplayer_session);
        if effective_pct == 0 {
            return None;
        }
        if effective_pct < 100 {
            state.game_speed_accumulator = state.game_speed_accumulator.saturating_add(u16::from(effective_pct));
            if state.game_speed_accumulator < 100 {
                return None;
            }
            state.game_speed_accumulator -= 100;
        }
        let advanced = state.clock.advance();
        let mut checksum_payload: Option<(Tick, f64, String)> = None;
        let mut tick_sample_payload: Option<(Tick, f64, TickSampleStats)> = None;
        let mut step_report: Option<(Tick, f64, ControlIntent, StepReport)> = None;
        let mut snapshot_payload: Option<(Tick, f64, ActorWorldSnapshot)> = None;
        // M1.5: bundle returned from `cf-terrain::try_dig` plus the dig source.
        // Stored locally so events can be emitted after the state guard is dropped.
        let mut dig_payload: Option<(Tick, f64, DigEvent)> = None;
        let mut ai_payloads: Vec<(Tick, f64, ActorId, cf_ai::EnemyTickReport)> = Vec::new();
        let mut guard_fire_records: Vec<GuardFireRecord> = Vec::new();
        let mut mission_payload: Option<(Tick, f64, cf_mission::MissionTickReport)> = None;
        if let Some(tick) = advanced {
            state.rng.next_u64();

            // M3 re-audit pass 4 (2026-05-13): stamp the current tick onto
            // the terrain so subsequent pixel writes set the right
            // `last_modified_tick` on the affected chunk(s). Engine drives
            // this BEFORE any carve / blast / fill in this tick's body.
            if let Some(t) = state.chunked_terrain.as_mut() {
                t.set_current_tick(tick.0);
            }

            // BP2 dig path. Chunked terrain takes priority when loaded; legacy
            // breach strips drive M1.5 backward compatibility. The dig first
            // probes chunked terrain in front of the player; if that produces a
            // result (Carved / Refused / NoOp) we consume the dig there. If
            // chunked terrain is NOT loaded but a breach world is, we fall back
            // to the M1.5 strip path.
            if state.pending_dig.is_some() && (state.chunked_terrain.is_some() || state.breach_world.is_some()) {
                let pending = state.pending_dig.take().expect("pending dig is_some");
                let player_pos_aim = state.player_actor.and_then(|pid| {
                    state
                        .actor_state
                        .as_ref()
                        .and_then(|sim| sim.world.actors.get(&pid))
                        .map(|a| ((a.position.x, a.position.y), (a.aim.x, a.aim.y)))
                });
                if let Some(((px, py), (ax, ay))) = player_pos_aim {
                    if let Some(terrain) = state.chunked_terrain.as_mut() {
                        // Tool reach + radius: 22-pixel reach along aim, 12-px
                        // carve radius. The radius is tuned so consecutive
                        // digs while the player walks (~3-4 px/tick) overlap
                        // and form a continuous tunnel without leaving micro-
                        // gaps that would block projectile-vs-terrain checks.
                        // M2 design intent (tight bites that require many
                        // digs) is preserved because each dig still only
                        // clears ~450 pixels out of a typical ~12,800-pixel
                        // shield mound.
                        const DIG_REACH: f32 = 22.0;
                        const DIG_RADIUS: f32 = 12.0;
                        let aim_len = (ax * ax + ay * ay).sqrt().max(0.001);
                        let nx = ax / aim_len;
                        let ny = ay / aim_len;
                        let target_x = px + nx * DIG_REACH;
                        let target_y = py + ny * DIG_REACH;
                        let outcome = terrain.try_carve([target_x, target_y], DIG_RADIUS);
                        dig_payload = Some((
                            tick,
                            state.clock.sim_time_ms(),
                            DigEvent::Chunked {
                                outcome,
                                source: pending.source,
                                origin: [px, py],
                                aim: [nx, ny],
                                target: [target_x, target_y],
                            },
                        ));
                    } else if let Some(world) = state.breach_world.as_mut() {
                        let outcome = cf_terrain::try_dig(
                            world,
                            cf_terrain::DigRequest {
                                origin: [px, py],
                                aim: [ax, ay],
                                explicit_target: pending.target.clone(),
                            },
                        );
                        dig_payload = Some((
                            tick,
                            state.clock.sim_time_ms(),
                            DigEvent::Strip {
                                outcome,
                                source: pending.source,
                                origin: [px, py],
                            },
                        ));
                    }
                }
            }

            // BEFORE the actor sim consumes it. The scripted steps emulate
            // what cfctl `act.player.{aim,fire,reload}` would do at the
            // matching tick, so headless cfctl drives of
            // `m14c_heat_vs_era.ron` / `m14c_apfsds_vs_heavy.ron` fire the
            // HEAT / APFSDS round at a deterministic tick without an
            // external driver. Multiple steps for the same tick stack
            // (last write wins on overlapping fields).
            let scripted_for_tick: Vec<crate::scenario::ScenarioScriptStep> = state
                .m14c_scripted_steps
                .iter()
                .filter(|step| step.tick == tick.0)
                .cloned()
                .collect();
            if !scripted_for_tick.is_empty() {
                if let Some(player_id) = state.player_actor {
                    state.pending_intent.actor = player_id;
                    state.pending_intent.source = cf_actor::IntentSource::Cfctl;
                    for step in &scripted_for_tick {
                        if let Some((ax, ay)) = step.aim {
                            if ax.is_finite() && ay.is_finite() {
                                state.pending_intent.aim = cf_actor::Vec2::new(ax, ay);
                            }
                        }
                        if step.fire {
                            state.pending_intent.fire = true;
                            state.pending_intent.fire_held = true;
                            state.pending_intent.ammo_kind = step.resolved_ammo_kind();
                        }
                        if step.reload {
                            state.pending_intent.reload = true;
                        }
                    }
                }
            }

            // M1: step the actor world if present. The pending intent is consumed and
            // its edge-triggered fields cleared so the next tick starts fresh. M1.5
            // augments this by running each reactive guard's controller and feeding its
            // generated intent into the same actor-step pipeline.
            if state.actor_state.is_some() {
                let mut intent = state.pending_intent.clone();
                state.pending_intent.clear_edges();
                let region_min_x = self.config.region_anchor_x;
                let region_max_x = self.config.region_anchor_x + self.config.region_width.max(0.0);
                let region_max_y = self.config.region_anchor_y + self.config.region_height.max(0.0);
                let tick_dt = SimConfig {
                    tick_rate_hz: self.config.tick_rate_hz,
                }
                .tick_dt()
                .as_secs_f32();
                let auto_reload = false;
                let player = state.player_actor;
                // - While `intent.fire_held`: accumulate `weapon_charge_fraction`
                //   at (tick_dt / SNIPER_CHARGE_MAX_SECONDS) per tick (so a full
                //   charge lands at 0.8 s of hold per spec § "Sniper charge mode")
                //   and suppress the rifle fire so the M1 path doesn't pop a round
                //   prematurely.
                // - On the release edge: synthesize a one-tick `intent.fire=true`
                //   so the M1 rifle path produces exactly one shot (whose damage
                //   the post-step pass scales by `charge_damage_multiplier`).
                // `fire_held_prev` latches the previous tick's hold state so the
                // release edge survives even when the player release-pulse is a
                // single tick.
                if let Some(player_id) = player {
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                            let now_held = intent.fire_held;
                            let was_held = actor.fire_held_prev;
                            match actor.weapon_fire_mode {
                                cf_equipment::AdvancedFireMode::Charge => {
                                    if now_held {
                                        let inc = tick_dt / cf_equipment::SNIPER_CHARGE_MAX_SECONDS;
                                        actor.weapon_charge_fraction =
                                            (actor.weapon_charge_fraction + inc).clamp(0.0, 1.0);
                                        intent.fire = false;
                                        intent.fire_held = false;
                                    } else if was_held && actor.weapon_charge_fraction > 0.0 {
                                        intent.fire = true;
                                        intent.fire_held = false;
                                    } else {
                                        actor.weapon_charge_fraction = 0.0;
                                    }
                                    actor.fire_held_prev = now_held;
                                }
                                cf_equipment::AdvancedFireMode::Burst3 => {
                                    // Burst-3 is "fire 3 rounds per trigger
                                    // press". Suppress the M1 trigger while a
                                    // burst is still in flight so the
                                    // FullAuto cadence on SMG doesn't pile
                                    // overlapping bursts together. The first
                                    // round still fires through the M1 path
                                    // on the edge press; the M6 tick
                                    // scheduler emits rounds 2 and 3.
                                    if actor.burst3_remaining_shots > 0 {
                                        intent.fire = false;
                                        intent.fire_held = false;
                                    }
                                    actor.weapon_charge_fraction = 0.0;
                                    actor.fire_held_prev = false;
                                }
                                _ => {
                                    actor.weapon_charge_fraction = 0.0;
                                    actor.fire_held_prev = false;
                                }
                            }
                        }
                    }
                }
                let mut intents = BTreeMap::new();
                if let Some(player_id) = player {
                    intents.insert(player_id, intent.clone());
                }

                // M1.5: tick reactive guards. We collect their fire records and apply
                // them to the actor world AFTER the player step so we don't aliasing
                // borrow the actor world mutably twice. The temporary `take()` of
                // each guard releases the BTreeMap borrow so we can mutate state.rng.
                let sim_time_ms = state.clock.sim_time_ms();
                // active, gate guard AI ticks on phase >= Launch so the
                // player can pre-dig during Setup + Prep without taking
                // fire. Scenarios without an M9 reactor director (M7
                // 4-phase, M2 mission, M1 lab) keep the legacy behaviour
                // where guards tick from tick 0.
                let guard_ai_unlocked = state
                    .m7_ai_world
                    .phase
                    .as_ref()
                    .map(|p| {
                        if p.phase_sequence == cf_mission::MissionPhase::M9_PACING {
                            p.is_launch_or_later()
                        } else {
                            true
                        }
                    })
                    .unwrap_or(true);
                let guard_ids: Vec<ActorId> = state.reactive_guards.keys().copied().collect();
                for guard_id in guard_ids {
                    let (self_actor, player_actor) = {
                        let sim = match state.actor_state.as_ref() {
                            Some(s) => s,
                            None => break,
                        };
                        (
                            sim.world.actors.get(&guard_id).cloned(),
                            player.and_then(|pid| sim.world.actors.get(&pid).cloned()),
                        )
                    };
                    let self_actor = match self_actor {
                        Some(a) => a,
                        None => continue,
                    };
                    let player_ref = player_actor.as_ref();
                    let mut guard = state
                        .reactive_guards
                        .remove(&guard_id)
                        .expect("guard exists by construction");
                    if !guard_ai_unlocked {
                        // Guard is gated behind the M9 Prep window. Hold
                        // its state without driving the FSM so it does
                        // not fire, acquire targets, or react to alarms
                        // before tick ~300.
                        state.reactive_guards.insert(guard_id, guard);
                        continue;
                    }
                    let alarms_snapshot: Vec<cf_ai::AlarmInput> = state.pending_alarms.clone();
                    let report = cf_ai::step(
                        &mut guard,
                        cf_ai::GuardTickInputs {
                            tick: tick.0,
                            tick_rate_hz: self.config.tick_rate_hz,
                            self_actor: &self_actor,
                            player: player_ref,
                            alarms: &alarms_snapshot,
                            // M2 re-audit (2026-05-13): M2 has exactly one damage source
                            // (the player). M7+ extends to multi-actor damage sources by
                            // wiring a per-actor `last_damage_source_actor_id` tracker.
                            last_damage_source: player_ref.map(|p| p.id.0),
                        },
                        &mut state.rng,
                    );
                    state.reactive_guards.insert(guard_id, guard);
                    if let Some(fire) = &report.fire {
                        guard_fire_records.push(GuardFireRecord {
                            shooter: guard_id,
                            origin: fire.muzzle_origin,
                            velocity: fire.velocity,
                            damage: fire.damage,
                            lifetime_ticks: fire.lifetime_ticks,
                            will_miss: fire.will_miss,
                        });
                    }
                    ai_payloads.push((tick, sim_time_ms, guard_id, report));
                }

                // M1: actor_step now takes a mutable RNG closure for the
                // multi-particle spread cone. Engine's seeded RNG flows in;
                // determinism is preserved across replays. We split the
                // `state` borrow into disjoint fields via destructuring so
                // the closure can capture `&mut state.rng` while the sim
                // takes `&mut state.actor_state`. The local destructure
                // refers to `EngineMutable` (the inner struct held by
                // `RwLock`); fields named here must match its definition.
                let settings_for_tuning = state.settings.clone();
                let EngineMutable {
                    actor_state: actor_state_slot,
                    rng: rng_slot,
                    ..
                } = &mut *state;
                let actor_state_mut = actor_state_slot.as_mut().expect("actor state present");
                // Gap F3: build tuning from live settings so cvar patches
                // applied via `act.settings.set` take effect on the next tick.
                let tuning = cf_actor::sim::ActorTuning {
                    max_speed: 220.0,
                    ground_acceleration: settings_for_tuning.accel,
                    air_acceleration: 600.0,
                    ground_friction: settings_for_tuning.friction,
                    jump_impulse: settings_for_tuning.jump_force,
                    terminal_velocity: -1800.0,
                    recoil_decay_per_tick: settings_for_tuning.recoil_decay_per_tick,
                    sharp_aim_build_ticks: settings_for_tuning.sharp_aim_build_ticks,
                    walk_threshold: settings_for_tuning.walk_threshold,
                };
                let report = actor_step(
                    actor_state_mut,
                    &mut intents,
                    StepDeps {
                        tick_dt,
                        region_min_x,
                        region_max_x,
                        region_max_y,
                        auto_reload_when_empty: auto_reload,
                        tuning: Some(tuning),
                        tutorial_safety: self.config.tutorial_safety,
                    },
                    &mut || rng_slot.next_u64(),
                );

                // M1.5: spawn guard projectiles into the same projectile pool the
                // actor step uses so cf-actor's swept hit detection runs against
                // them on subsequent ticks. We allocate ids from the dedicated
                // guard range to avoid colliding with player projectile ids.
                if !guard_fire_records.is_empty() {
                    for fire in &guard_fire_records {
                        let id = state.next_guard_projectile_id;
                        state.next_guard_projectile_id = state.next_guard_projectile_id.wrapping_add(1);
                        let actor_state_mut = state.actor_state.as_mut().expect("actor state present");
                        actor_state_mut.projectiles.push(cf_actor::sim::Projectile {
                            id,
                            owner: fire.shooter,
                            origin: cf_actor::Vec2::new(fire.origin[0], fire.origin[1]),
                            position: cf_actor::Vec2::new(fire.origin[0], fire.origin[1]),
                            velocity: cf_actor::Vec2::new(fire.velocity[0], fire.velocity[1]),
                            damage: fire.damage,
                            remaining_ticks: fire.lifetime_ticks,
                            // Guard-fired rounds inherit the same baseline
                            // as a default rifle until the guard spec adds
                            // per-tower mass + sharpness fields.
                            mass_kg: 0.05,
                            sharpness: 0.8,
                        });
                    }
                }

                step_report = Some((tick, state.clock.sim_time_ms(), intent, report));
            }

            // M2: projectile-vs-chunked-terrain collision. Solid terrain
            // pixels stop projectiles cold; the projectile expires with cause
            // `terrain_hit`. This is what makes M2.5 micro_reactor_defense
            // strategic: dirt mounds between the guard and the reactor block
            // bullets, and the player's dig action exposes the reactor.
            //
            // M2 extension: route hits through `cf_physics::try_penetrate`
            // so impulse² > integrity² determines pass/fail per CCCP
            // `SceneMan.cpp:571`. Passing projectiles carve the pixel and
            // emit a `terrain.terrain_penetration_threshold` + the
            // `terrain.terrain_pixel_dislodged` debris event. Failing
            // projectiles roll for stickiness and may be drawn in.
            //
            // every projectile-vs-terrain hit (pass OR fail) also applies
            // per-pixel integrity damage via
            // `ChunkedTerrain::try_penetrate_pixel`. The pixel may survive
            // (hard material) or be destroyed (soft material), and band
            // crossings + cascade decay are emitted as `terrain.*` events
            // alongside the existing M2 events. Captured here as
            // `damage_outcome` so the emit loop has both the binary
            // pass/fail and the per-pixel ledger.
            struct TerrainHit {
                projectile_id: u64,
                owner: ActorId,
                pos: [f32; 2],
                material_id: cf_terrain::MaterialId,
                material_name: &'static str,
                impulse_squared: f32,
                integrity_squared: f32,
                impulse: f32,
                integrity: f32,
                passed: bool,
                stuck: bool,
                damage: f32,
                spawn_material: Option<cf_terrain::MaterialId>,
                damage_outcome: Option<cf_terrain::PenetrationOutcome>,
                stickiness: f32,
                /// check, captured so `combat.embedded_in_terrain` can carry
                /// the exact roll for replay verification.
                rng_roll: f32,
            }
            let mut terrain_hits: Vec<TerrainHit> = Vec::new();
            if state.chunked_terrain.is_some() && state.actor_state.is_some() {
                let EngineMutable {
                    actor_state,
                    chunked_terrain,
                    rng,
                    ..
                } = &mut *state;
                let terrain = chunked_terrain.as_mut().expect("chunked terrain present");
                if let Some(actor_state_mut) = actor_state.as_mut() {
                    let mut survivors: Vec<cf_actor::sim::Projectile> = Vec::new();
                    for proj in actor_state_mut.projectiles.drain(..) {
                        let mat = terrain.material_at_world(proj.position.x, proj.position.y);
                        if !terrain.registry.is_solid(mat) {
                            survivors.push(proj);
                            continue;
                        }
                        // Material-aware penetration formula.
                        let aff = terrain.registry.affordance(mat).expect("solid material has affordance");
                        // Per-projectile mass + sharpness sourced from
                        // `RifleSpec.bullet_mass_kg` / `bullet_sharpness`
                        // at spawn time and carried through the `Projectile`
                        // struct. Tank-grade rounds (APFSDS long-rod /
                        // HEAT shaped-charge) override the defaults via
                        // their spec rows so heavy weapons punch through
                        // walls per the M14C contract.
                        let velocity = (proj.velocity.x * proj.velocity.x + proj.velocity.y * proj.velocity.y).sqrt();
                        // Seeded RNG roll for stickiness — preserves determinism.
                        let rng_roll = (rng.next_u64() as f64 / u64::MAX as f64) as f32;
                        let outcome = cf_physics::try_penetrate(cf_physics::PenetrationInputs {
                            mass: proj.mass_kg,
                            velocity,
                            sharpness: proj.sharpness,
                            integrity: aff.hardness,
                            stickiness: aff.stickiness,
                            restitution: aff.restitution,
                            friction: aff.friction,
                            rng_roll,
                        });
                        let pos = [proj.position.x, proj.position.y];
                        // normalize impact energy from projectile velocity
                        // (reference rifle round at ~150 u/s = 0.5 impact,
                        // matching the spec's "impact_energy=0.5" scenario).
                        // Clamp to [0, 1] so debug-spawn bullets cannot
                        // overshoot the formula.
                        let impact_energy = (velocity / 300.0).clamp(0.0, 1.0);
                        let px_i = (proj.position.x - terrain.anchor[0]).floor() as i64;
                        let py_i = (proj.position.y - terrain.anchor[1]).floor() as i64;
                        // Apply per-pixel integrity damage. The pixel may
                        // survive (hardness > impact) or be destroyed
                        // (integrity reached 0). When destroyed, cascade
                        // decay propagates to 4-neighbors below the
                        // cascade-threshold gate (default 0.6).
                        let damage_outcome = terrain.try_penetrate_pixel(
                            px_i,
                            py_i,
                            impact_energy,
                            cf_terrain::DamageKind::ProjectileHit,
                            None,
                        );
                        // decision for projectile lifecycle (lives/dies/stuck)
                        // but stops the redundant carve — destruction is now
                        // driven by integrity reaching 0. When the per-pixel
                        // outcome reports `destroyed=true`, the world has
                        // already cleared the pixel. When it reports
                        // `destroyed=false`, the pixel survives the hit even
                        // if cf_physics says the projectile pierced (the
                        // pixel's residual hardness wears down per spec).
                        if outcome.passes {
                            terrain_hits.push(TerrainHit {
                                projectile_id: proj.id,
                                owner: proj.owner,
                                pos,
                                material_id: mat,
                                material_name: aff.name,
                                impulse_squared: outcome.impulse_squared,
                                integrity_squared: outcome.integrity_squared,
                                impulse: outcome.impulse,
                                integrity: outcome.integrity,
                                passed: true,
                                stuck: false,
                                damage: proj.damage,
                                spawn_material: aff.spawn_material,
                                damage_outcome,
                                stickiness: aff.stickiness,
                                rng_roll,
                            });
                        } else {
                            terrain_hits.push(TerrainHit {
                                projectile_id: proj.id,
                                owner: proj.owner,
                                pos,
                                material_id: mat,
                                material_name: aff.name,
                                impulse_squared: outcome.impulse_squared,
                                integrity_squared: outcome.integrity_squared,
                                impulse: outcome.impulse,
                                integrity: outcome.integrity,
                                passed: false,
                                stuck: outcome.stuck,
                                damage: proj.damage,
                                spawn_material: aff.spawn_material,
                                damage_outcome,
                                stickiness: aff.stickiness,
                                rng_roll,
                            });
                        }
                    }
                    actor_state_mut.projectiles = survivors;
                }
            }
            if !terrain_hits.is_empty() {
                let sim_time_ms = state.clock.sim_time_ms();
                for hit in terrain_hits {
                    // M2: penetration threshold event carries the formula
                    // inputs so replays + AI agents can verify the contact.
                    //
                    // M3 re-audit pass 4 (2026-05-13): spec requires
                    // `parent_event_id` linking to the
                    // `combat.projectile_spawned` event. Use the persisted
                    // spawn id map.
                    let projectile_spawn_parent = state.projectile_spawn_event_ids.get(&hit.projectile_id).cloned();
                    let pen_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "terrain",
                        "terrain_penetration_threshold",
                        json!({
                            "projectile_id": hit.projectile_id,
                            "owner": hit.owner.0,
                            "material_id": hit.material_id,
                            "material": hit.material_name,
                            "impulse": hit.impulse,
                            "integrity": hit.integrity,
                            "impulse_squared": hit.impulse_squared,
                            "integrity_squared": hit.integrity_squared,
                            "passed": hit.passed,
                            "stuck": hit.stuck,
                            "spawned_material": hit.spawn_material
                                .map(cf_terrain::material_name_from_id),
                            "spawned_material_id": hit.spawn_material,
                            "debris_count": if hit.passed { 1 } else { 0 },
                            "position": hit.pos,
                        }),
                        projectile_spawn_parent,
                    );
                    if hit.passed {
                        // Carve emitted by try_carve; the dislodged-pixel
                        // event closes the cause chain on the same tick.
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "terrain_pixel_dislodged",
                            json!({
                                "pos": hit.pos,
                                "source_material": hit.material_name,
                                "source_material_id": hit.material_id,
                                "spawn_material": hit.spawn_material
                                    .map(cf_terrain::material_name_from_id),
                                "spawn_material_id": hit.spawn_material,
                                "count": 1u32,
                                "child_pixel_id": format!("proj{}:{}",
                                    hit.projectile_id, tick.0),
                            }),
                            Some(pen_id.clone()),
                        );
                        if let Ok(mut s) = self.state.write() {
                            s.total_debris_spawned = s.total_debris_spawned.saturating_add(1);
                        }
                    }
                    // cascade rule**: emit per-pixel band crossings and
                    // cascade decay events. Each event carries
                    // parent_event_id = pen_id so the M10 cause-chain
                    // walker resolves projectile → pixel → cascade.
                    if let Some(damage) = &hit.damage_outcome {
                        if damage.band_crossed {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "terrain",
                                "material_state_changed",
                                json!({
                                    "pos": damage.pos,
                                    "material_id": damage.material_id,
                                    "material_name": damage.material_name,
                                    "from_band": damage.band_before.as_str(),
                                    "to_band": damage.band_after.as_str(),
                                    "integrity_before": damage.integrity_before,
                                    "integrity_after": damage.integrity_after,
                                    "cause": "projectile_hit",
                                    "parent_event_id": pen_id.clone(),
                                }),
                                Some(pen_id.clone()),
                            );
                        }
                        if damage.destroyed {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "terrain",
                                "pixel_removed",
                                json!({
                                    "pos": damage.pos,
                                    "was_material": damage.material_id,
                                    "was_material_name": damage.material_name,
                                    "cascade_cause": "direct_damage",
                                    "parent_event_id": pen_id.clone(),
                                }),
                                Some(pen_id.clone()),
                            );
                        }
                        // neighbor. affected_count surfaces total cascade
                        // reach so the M10 viewer can render a "domino"
                        // visualization.
                        let affected_count = damage.cascades.len() as u32;
                        for cascade in &damage.cascades {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "terrain",
                                "cascade_triggered",
                                json!({
                                    "from_pos": cascade.from_pos,
                                    "to_pos": cascade.to_pos,
                                    "cascade_depth": cascade.depth,
                                    "cascade_threshold": cascade.threshold,
                                    "cascade_decay_pct": cf_terrain::DEFAULT_CASCADE_DECAY_PCT,
                                    "affected_count": affected_count,
                                    "material_id": cascade.material_id,
                                    "material_name": cascade.material_name,
                                    "integrity_before": cascade.integrity_before,
                                    "integrity_after": cascade.integrity_after,
                                    "from_band": cascade.from_band.as_str(),
                                    "to_band": cascade.to_band.as_str(),
                                    "destroyed_neighbor": cascade.destroyed_neighbor,
                                    "reason": "neighbor_destroyed",
                                    "parent_event_id": pen_id.clone(),
                                }),
                                Some(pen_id.clone()),
                            );
                            if cascade.destroyed_neighbor {
                                self.recorder.record(
                                    tick,
                                    sim_time_ms,
                                    "terrain",
                                    "pixel_removed",
                                    json!({
                                        "pos": cascade.to_pos,
                                        "was_material": cascade.material_id,
                                        "was_material_name": cascade.material_name,
                                        "cascade_cause": "neighbor_destroyed",
                                        "parent_event_id": pen_id.clone(),
                                    }),
                                    Some(pen_id.clone()),
                                );
                            }
                        }
                    }
                    // when the stickiness roll pulled the projectile in
                    // (cf_physics::try_penetrate.outcome.stuck) emit
                    // `combat.embedded_in_terrain` carrying the seeded RNG
                    // roll + material stickiness coefficient so replays +
                    // M14 tests can verify the embedding contract.
                    if hit.stuck {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "combat",
                            "embedded_in_terrain",
                            json!({
                                "projectile_id": hit.projectile_id,
                                "owner_id": hit.owner.0,
                                "position": hit.pos,
                                "material_id": hit.material_id,
                                "material": hit.material_name,
                                "stickiness": hit.stickiness,
                                "rng_roll": hit.rng_roll,
                                "source_event_id": pen_id.clone(),
                            }),
                            Some(pen_id.clone()),
                        );
                    }
                    // Legacy combat.projectile_expired so existing tooling
                    // (M2.5 reactor scenarios, M3A determinism viewer) still
                    // observes the projectile lifecycle.
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "combat",
                        "projectile_expired",
                        json!({
                            "id": hit.projectile_id,
                            "owner": hit.owner.0,
                            "last_position": hit.pos,
                            "cause": "terrain_hit",
                            "material": hit.material_name,
                            "passed": hit.passed,
                            "stuck": hit.stuck,
                        }),
                        Some(pen_id),
                    );
                    let _ = hit.damage; // reserved for future M5.5 splash damage routing
                }
            }

            // CCCP)". For every actor with at least one destroyed chassis
            // zone, apply per-tick bleed damage scaled by the number of
            // lost zones. Drops the actor's HP directly; the existing
            // status-change pipeline emits actor_status_changed when HP
            // crosses zero. Bleed events are surfaced under affliction.tick
            // category so the M16 affliction system can consume them.
            struct BleedHit {
                actor: ActorId,
                lost_zones: u32,
                damage: f32,
            }
            let mut bleed_hits: Vec<BleedHit> = Vec::new();
            if let Some(actor_state) = state.actor_state.as_mut() {
                let tick_rate = self.config.tick_rate_hz.max(1);
                for (aid, actor) in actor_state.world.actors.iter_mut() {
                    if matches!(actor.status, cf_actor::Status::Dead) {
                        continue;
                    }
                    let lost_zones: u32 = actor
                        .chassis
                        .as_ref()
                        .map(|c| c.destroyed_zones().len() as u32)
                        .unwrap_or(0);
                    if lost_zones == 0 {
                        continue;
                    }
                    let dmg = cf_physics::bleed_per_tick(lost_zones, tick_rate);
                    if dmg > 0.0 {
                        actor.hp = (actor.hp - dmg).max(0.0);
                        if actor.hp <= 0.0 && !matches!(actor.status, cf_actor::Status::Dying | cf_actor::Status::Dead)
                        {
                            actor.status = cf_actor::Status::Dying;
                        }
                        bleed_hits.push(BleedHit {
                            actor: *aid,
                            lost_zones,
                            damage: dmg,
                        });
                    }
                }
            }
            // Emit bleed-tick events (deferred to avoid recorder re-entrancy
            // while we hold the actor_state lock).
            if !bleed_hits.is_empty() {
                let sim_time_ms = state.clock.sim_time_ms();
                drop(state);
                for bleed in &bleed_hits {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "affliction",
                        "tick",
                        json!({
                            "actor_id": bleed.actor.0,
                            "kind": "bleeding",
                            "lost_zones": bleed.lost_zones,
                            "damage_per_tick": bleed.damage,
                            "source": "m14_limb_loss",
                            "cause": "limb_loss_bleed_out",
                        }),
                        None,
                    );
                }
                state = self.state.write().expect("re-acquire engine state");
            }

            // M2: hazard tile contact damage routing. For every actor whose
            // AABB overlaps any hazard pixel this tick, apply
            // damage_per_tick × overlap_scale via `cf_physics::hazard_contact_damage`
            // and emit `terrain.hazard_contact_or_avoidance`. The damage flows
            // into the actor sim state for `actor.actor_status_changed` to
            // surface as a HUD banner.
            struct HazardHit {
                actor: ActorId,
                pixel_count: u32,
                damage: f32,
            }
            let mut hazard_hits: Vec<HazardHit> = Vec::new();
            if state.chunked_terrain.is_some() && state.actor_state.is_some() {
                let EngineMutable {
                    actor_state,
                    chunked_terrain,
                    ..
                } = &mut *state;
                let terrain = chunked_terrain.as_ref().expect("chunked terrain present");
                if let Some(actor_state_ref) = actor_state.as_mut() {
                    for (aid, actor) in actor_state_ref.world.actors.iter_mut() {
                        if actor.status == cf_actor::Status::Dead {
                            continue;
                        }
                        // Sample actor's AABB against the terrain hazard pixels.
                        // Half-extents from the actor itself; M1.5 chassis-less
                        // actors use 8x16.
                        let hx = 8.0_f32;
                        let hy = 16.0_f32;
                        let min = [actor.position.x - hx, actor.position.y - hy];
                        let max = [actor.position.x + hx, actor.position.y + hy];
                        let mut hazard_pixels = 0u32;
                        let mut total_damage_per_tick = 0.0f32;
                        // Scan a sparse subset to keep this O(small) — every
                        // 4th pixel in the actor AABB is sampled (256 samples
                        // for a 16x32 actor). Sufficient for hazard detection
                        // at M2 resolution.
                        let mut py = min[1].floor() as i64;
                        while py <= max[1].ceil() as i64 {
                            let mut px = min[0].floor() as i64;
                            while px <= max[0].ceil() as i64 {
                                let mat = terrain.material_at(px, py);
                                if terrain.registry.is_hazard(mat) {
                                    hazard_pixels += 1;
                                    total_damage_per_tick =
                                        total_damage_per_tick.max(terrain.registry.damage_per_tick(mat));
                                }
                                px += 4;
                            }
                            py += 4;
                        }
                        if hazard_pixels > 0 && total_damage_per_tick > 0.0 {
                            let dmg = cf_physics::hazard_contact_damage(hazard_pixels, total_damage_per_tick);
                            if dmg > 0.0 {
                                actor.hp = (actor.hp - dmg).max(0.0);
                                if actor.hp <= 0.0 && actor.status != cf_actor::Status::Dead {
                                    actor.status = cf_actor::Status::Dead;
                                }
                                hazard_hits.push(HazardHit {
                                    actor: *aid,
                                    pixel_count: hazard_pixels,
                                    damage: dmg,
                                });
                            }
                        }
                    }
                }
            }
            if !hazard_hits.is_empty() {
                let sim_time_ms = state.clock.sim_time_ms();
                let current_tick = tick.0;
                // Build the per-hit emit decision FIRST, while we still hold
                // the write guard from drive_tick. Re-entrant locking on the
                // same RwLock from inside drive_tick deadlocks — std::sync
                // RwLock has no re-entrant read support. Resolve all reads
                // against the in-scope `state` guard.
                let mut emits: Vec<HazardHit> = Vec::new();
                for hit in &hazard_hits {
                    let recent = state
                        .hazard_last_contact_tick
                        .get(&hit.actor)
                        .map(|prev| current_tick.saturating_sub(*prev) < 6)
                        .unwrap_or(false);
                    if !recent {
                        emits.push(HazardHit {
                            actor: hit.actor,
                            pixel_count: hit.pixel_count,
                            damage: hit.damage,
                        });
                        state.hazard_last_contact_tick.insert(hit.actor, current_tick);
                    }
                }
                drop(state);
                for hit in emits {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "terrain",
                        "hazard_contact_or_avoidance",
                        json!({
                            "actor_id": hit.actor.0,
                            "hazard_material": "hazard",
                            "hazard_material_id": cf_terrain::MATERIAL_HAZARD,
                            "contact": true,
                            "damage_applied": hit.damage,
                            "pixel_overlap": hit.pixel_count,
                            "cause_label": "actor_in_hazard_tile",
                        }),
                        None,
                    );
                }
                state = self.state.write().expect("engine state poisoned");
            }

            // M2.5: route projectile hits onto reactor AABBs. We walk every
            // live projectile after the actor step and damage the first
            // reactor whose AABB contains the projectile position. Hits emit
            // `combat.projectile_hit` (target=reactor) + `actor.actor_status_changed`
            // (target=reactor) when the reactor reaches zero hp.
            //
            // Per-hit state (`hp_after`, `hp_max`, `destroyed_after`) is
            // captured AT THE MOMENT THE HIT IS PROCESSED, not later. The
            // earlier "read final reactor state in the emit loop" approach
            // was Bugbot 2ce56d7e: when two projectiles hit the same reactor
            // in one tick, the first hit's event would falsely report the
            // post-second-hit hp + destroyed flag, producing duplicate
            // destruction events.
            struct ReactorHit {
                rid: String,
                damage_applied: f32,
                position: [f32; 2],
                projectile_id: u64,
                hp_before: f32,
                hp_after: f32,
                hp_max: f32,
                destroyed_after: bool,
                /// True only on the hit that flipped the reactor to
                /// destroyed (so we emit `actor_status_changed` exactly
                /// once per reactor).
                triggered_destruction: bool,
                // from `Reactor::apply_damage_cascade`.
                layer_events: Vec<cf_mission::ArmorLayerHpEvent>,
                pressure_state_change: Option<(cf_mission::PressureState, cf_mission::PressureState)>,
                pressure_state_after: cf_mission::PressureState,
                hp_percent_after: f32,
            }
            let mut reactor_hits: Vec<ReactorHit> = Vec::new();
            if state.reactor_world.is_some() && state.actor_state.is_some() {
                let EngineMutable {
                    actor_state,
                    reactor_world,
                    ..
                } = &mut *state;
                let reactors = reactor_world.as_mut().expect("reactor world present");
                if let Some(actor_state_mut) = actor_state.as_mut() {
                    actor_state_mut.projectiles.retain(|proj| {
                        let mut consumed = false;
                        for r in reactors.iter_mut() {
                            if r.is_destroyed() {
                                continue;
                            }
                            if r.aabb_contains(proj.position.x, proj.position.y) {
                                let prev_destroyed = r.is_destroyed();
                                // `engine.config.tutorial_safety` so the
                                // reactor's lethal-damage path caps at 1
                                // HP + Critical instead of destroying
                                // when tutorial_safety is enabled.
                                let report =
                                    r.apply_damage_cascade_with_safety(proj.damage, self.config.tutorial_safety);
                                reactor_hits.push(ReactorHit {
                                    rid: r.id.clone(),
                                    damage_applied: report.damage_applied,
                                    position: [proj.position.x, proj.position.y],
                                    projectile_id: proj.id,
                                    hp_before: report.hp_before,
                                    hp_after: report.hp_after,
                                    hp_max: r.max_hp,
                                    destroyed_after: report.now_destroyed,
                                    triggered_destruction: report.triggered_destruction && !prev_destroyed,
                                    layer_events: report.layer_events,
                                    pressure_state_change: report.pressure_state_change,
                                    pressure_state_after: r.pressure_state,
                                    hp_percent_after: report.hp_percent_after,
                                });
                                consumed = true;
                                break;
                            }
                        }
                        !consumed
                    });
                }
            }
            if !reactor_hits.is_empty() {
                let sim_time_ms = state.clock.sim_time_ms();
                for hit in reactor_hits {
                    // projectile's spawn event id through so M10's
                    // "show me why" walker can hop `projectile_hit →
                    // projectile_spawned → weapon_fired → ai.tactic_chosen
                    // → ai.target_acquired → ai.target_scored`. Falls back
                    // to None when the spawn id is no longer in the map
                    // (e.g. a hit fired the same tick as a reset).
                    let projectile_spawn_parent = state.projectile_spawn_event_ids.get(&hit.projectile_id).cloned();
                    state.projectile_spawn_event_ids.remove(&hit.projectile_id);
                    let hit_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "combat",
                        "projectile_hit",
                        json!({
                            "target_kind": "reactor",
                            "target": hit.rid.clone(),
                            "position": hit.position,
                            "damage": hit.damage_applied,
                            "projectile_id": hit.projectile_id,
                            "parent_event_id": projectile_spawn_parent.clone(),
                        }),
                        projectile_spawn_parent,
                    );
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "actor",
                        "reactor_damaged",
                        json!({
                            "reactor": hit.rid.clone(),
                            "hp": hit.hp_after,
                            "hp_max": hit.hp_max,
                            "destroyed": hit.destroyed_after,
                            "damage_applied": hit.damage_applied,
                        }),
                        Some(hit_id.clone()),
                    );
                    // `mission.reactor_hp_changed` with parent_event_id
                    // chain back to combat.projectile_hit (which itself
                    // parents to the projectile_spawned + weapon_fired).
                    // M10 replay viewer walks parent_event_id to render the
                    // cause-chain death recap.
                    let hp_changed_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "mission",
                        "reactor_hp_changed",
                        json!({
                            "reactor_id": hit.rid.clone(),
                            "hp_before": hit.hp_before,
                            "hp_after": hit.hp_after,
                            "hp_max": hit.hp_max,
                            "hp_percent": hit.hp_percent_after,
                            "damage_applied": hit.damage_applied,
                            "source_actor_id": serde_json::Value::Null,
                            "cause": "projectile_hit",
                            "parent_event_id": hit_id.clone(),
                        }),
                        Some(hit_id.clone()),
                    );
                    // per cascade entry. Layer destroyed events carry the
                    // breach_kind so the M10 viewer can render
                    // "Reactor External armor breached: punctured".
                    for layer_event in &hit.layer_events {
                        let layer_str = layer_event.layer.as_str();
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "armor",
                            "layer_hp_changed",
                            json!({
                                "actor_id": serde_json::Value::Null,
                                "item_id": 0,
                                "zone": "reactor_core",
                                "layer": layer_str,
                                "from": layer_event.from,
                                "to": layer_event.to,
                                "cause": "projectile_hit",
                                "ap_factor": 1.0,
                            }),
                            Some(hp_changed_id.clone()),
                        );
                        if layer_event.critical {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "armor",
                                "layer_critical",
                                json!({
                                    "item_id": 0,
                                    "zone": "reactor_core",
                                    "layer": layer_str,
                                    "hp_percent": layer_event.to / hit.hp_max.max(1.0),
                                }),
                                Some(hp_changed_id.clone()),
                            );
                        }
                        if layer_event.destroyed {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "armor",
                                "layer_destroyed",
                                json!({
                                    "item_id": 0,
                                    "zone": "reactor_core",
                                    "layer": layer_str,
                                    "breach_kind": "punctured",
                                }),
                                Some(hp_changed_id.clone()),
                            );
                        }
                    }
                    // on crossings (the `pressure_state_change` is `Some`
                    // exactly when the band advanced).
                    if let Some((from, to)) = hit.pressure_state_change {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "mission",
                            "reactor_pressure_state_changed",
                            json!({
                                "reactor_id": hit.rid.clone(),
                                "from": from.as_str(),
                                "to": to.as_str(),
                                "hp_percent": hit.hp_percent_after,
                                "reason": "damage_accumulation",
                                "parent_event_id": hp_changed_id.clone(),
                            }),
                            Some(hp_changed_id.clone()),
                        );
                        // state — Venting/Critical reactors radiate more.
                        // Future M16+ thermal kernel consumes this.
                        let thermal_k = match to {
                            cf_mission::PressureState::Venting => 1200.0,
                            cf_mission::PressureState::Critical => 800.0,
                            cf_mission::PressureState::Stressed => 500.0,
                            cf_mission::PressureState::Destroyed => 0.0,
                            cf_mission::PressureState::Nominal => 300.0,
                        };
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "thermal",
                            "signature_changed",
                            json!({
                                "actor_id": serde_json::Value::Null,
                                "from_k": 0.0,
                                "to_k": thermal_k,
                                "source": "reactor_pressure_change",
                            }),
                            Some(hp_changed_id.clone()),
                        );
                    }
                    // Emit `actor_status_changed` ONLY on the hit that
                    // flipped the reactor to destroyed. Subsequent same-
                    // tick hits on the same reactor have
                    // `destroyed_after == true` but `triggered_destruction
                    // == false`, so they don't duplicate the transition
                    // event.
                    if hit.triggered_destruction {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "actor",
                            "actor_status_changed",
                            json!({
                                "actor_kind": "reactor",
                                "actor": hit.rid.clone(),
                                "previous_status": "active",
                                "new_status": "destroyed",
                                "cause": "projectile_hit",
                            }),
                            Some(hit_id.clone()),
                        );
                        // with parent_event_id chain so M10's "Show me why"
                        // resolver finds: mission_resolved → reactor_destroyed
                        // → reactor_hp_changed → projectile_hit.
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "mission",
                            "reactor_destroyed",
                            json!({
                                "reactor_id": hit.rid.clone(),
                                "position": hit.position,
                                "final_pressure_state": hit.pressure_state_after.as_str(),
                                "source_actor_id": serde_json::Value::Null,
                                "cause": "projectile_hit",
                                "parent_event_id": hp_changed_id.clone(),
                            }),
                            Some(hp_changed_id.clone()),
                        );
                    }
                }
            }

            // M1.5: tick the mission state machine after the actor world settles.
            // This runs even when the scenario has no actor world so a breach-only
            // or timer-only scenario still ticks its loss timer and objectives.
            if state.mission.is_some() {
                let sim_time_ms = state.clock.sim_time_ms();
                // Snapshot inputs so we can drop the actor borrow before we mutate
                // the mission slot. The actor world clones cheaply (BTreeMap is
                // O(n)); 16-actor scenarios are well within budget. When no actor
                // world is loaded we feed the mission an empty actor map.
                let breaches_broken = state.breach_world.as_ref().map(|w| w.broken_map()).unwrap_or_default();
                let breaches_progress = state
                    .breach_world
                    .as_ref()
                    .map(|w| w.progress_map())
                    .unwrap_or_default();
                let player_id = state.player_actor;
                let (actors_clone, player_clone) = match state.actor_state.as_ref() {
                    Some(actor_state_ref) => {
                        let actors = actor_state_ref.world.actors.clone();
                        let player_clone = player_id.and_then(|pid| actors.get(&pid).cloned());
                        (actors, player_clone)
                    }
                    None => (BTreeMap::new(), None),
                };
                let reactors_destroyed = state
                    .reactor_world
                    .as_ref()
                    .map(|w| w.destroyed_map())
                    .unwrap_or_default();
                let mission = state.mission.as_mut().expect("mission present");
                let inputs = cf_mission::MissionTickInputs {
                    tick: tick.0,
                    player: player_clone.as_ref(),
                    actors: &actors_clone,
                    breaches_broken: &breaches_broken,
                    reactors_destroyed: &reactors_destroyed,
                    breaches_progress: &breaches_progress,
                };
                let report = cf_mission::step(mission, inputs);
                if !report.objective_completed.is_empty()
                    || !report.objective_started.is_empty()
                    || !report.objective_failed.is_empty()
                    || !report.objective_updated.is_empty()
                    || report.final_result.is_some()
                {
                    mission_payload = Some((tick, sim_time_ms, report));
                }
            }
            // at 30s / 15s / 5s remaining (single-shot per threshold per run).
            // Computed against the active mission's `time_limit_ticks` so 120Hz
            // runs scale automatically (3600 @60Hz = 7200 @120Hz; same wall time).
            if state.mission.is_some() {
                let sim_time_ms = state.clock.sim_time_ms();
                let tick_rate_hz = self.config.tick_rate_hz.max(1);
                let mission_ref = state.mission.as_ref().expect("mission present");
                let total_ticks = mission_ref.loss.time_limit_ticks;
                if total_ticks > 0 && !mission_ref.result.is_terminal() {
                    let remaining_ticks = total_ticks.saturating_sub(tick.0);
                    let remaining_s = remaining_ticks / u64::from(tick_rate_hz);
                    for (threshold_s, severity, caption) in cf_mission::TIMER_WARNING_THRESHOLDS_S {
                        let threshold = *threshold_s;
                        let already_emitted = state
                            .m9_timer_warnings_emitted
                            .get(&threshold)
                            .copied()
                            .unwrap_or(false);
                        if !already_emitted && remaining_s <= u64::from(threshold) {
                            state.m9_timer_warnings_emitted.insert(threshold, true);
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "mission",
                                "timer_warning_threshold",
                                json!({
                                    "threshold_s": threshold,
                                    "remaining_ticks": remaining_ticks,
                                    "total_ticks": total_ticks,
                                    "severity": *severity,
                                    "caption_key": *caption,
                                }),
                                state.last_mission_event_id.clone(),
                            );
                        }
                    }
                }
            }
            // Runs after the dig + projectile + mission passes so the
            // chemistry sees the current pixel state. Fires when
            // chunked_terrain is loaded (M1.5/M2+ scenarios). Per
            // resulting state participates in the next checksum.
            if state.chunked_terrain.is_some() {
                let sim_time_ms = state.clock.sim_time_ms();
                // kernel reads the heat field for phase transitions,
                // inject heat from hot materials (fire_intense, lava,
                // lightning) AND apply one diffusion pass to smooth
                // gradients. Without this, the heat field stays at
                // initial scenario state forever; with it, phase
                // transitions fire DYNAMICALLY in response to fire
                // spreading + cooling, exactly per spec.
                inject_thermal_sources_and_diffuse(&mut *state);
                let prev_heat_snapshot = state.prev_heat_field.clone();
                let report = {
                    let EngineMutable {
                        chunked_terrain,
                        material_kernel,
                        reaction_registry,
                        phase_registry,
                        heat_field,
                        ..
                    } = &mut *state;
                    let terrain = chunked_terrain.as_mut().expect("chunked_terrain present");
                    cf_material::kernel_step(
                        terrain,
                        material_kernel,
                        reaction_registry,
                        phase_registry,
                        heat_field,
                        prev_heat_snapshot.as_ref(),
                    )
                };
                // Snapshot heat field for next-tick threshold detection.
                state.prev_heat_field = Some(state.heat_field.clone());
                for ev in &report.reactions {
                    let mut emission_positions_json: Vec<serde_json::Value> = Vec::new();
                    for p in &ev.emission_positions {
                        emission_positions_json.push(json!([p[0], p[1]]));
                    }
                    let mut payload = json!({
                        "reaction_id": ev.reaction_id,
                        "material_a": ev.material_a,
                        "material_b": ev.material_b,
                        "output": ev.output,
                        "byproduct": ev.byproduct,
                        "emissions": ev.emissions,
                        "emission_positions": emission_positions_json,
                        "pos": ev.pos,
                        "energy_release_j": ev.energy_release_j,
                        "auto_ignite": ev.auto_ignite,
                    });
                    if ev.violent {
                        payload["violent"] = serde_json::Value::Bool(true);
                        if let Some(c) = &ev.flash_color_hex {
                            payload["flash_color_hex"] = serde_json::Value::String(c.clone());
                        }
                    }
                    self.recorder.record(tick, sim_time_ms, "material", "reaction_triggered", payload.clone(), None);
                    // M15D § Also emit the unified reaction.triggered
                    // event (new schema) so M15D consumers see one
                    // consistent payload regardless of which category
                    // namespace they listen on.
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "reaction",
                        "triggered",
                        json!({
                            "reaction_id": ev.reaction_id,
                            "material_a": ev.material_a,
                            "material_b": ev.material_b,
                            "output": ev.output,
                            "byproduct": ev.byproduct,
                            "pos": ev.pos,
                            "delta_h_kj_per_mol": ev.energy_release_j / 1000.0,
                            "rate_per_s": 0.0_f32,
                            "variant": "PerPixel",
                            "auto_ignite": ev.auto_ignite,
                        }),
                        None,
                    );
                    if ev.violent {
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "material",
                            "violent_burst",
                            json!({
                                "reaction_id": ev.reaction_id,
                                "pos": ev.pos,
                                "energy_release_j": ev.energy_release_j,
                                "flash_color_hex": ev.flash_color_hex,
                            }),
                            None,
                        );
                    }
                }
                // M15D § Derive + emit the spec's 5 new reaction events
                // (autoignited, chain_propagated, completed) from the
                // per-tick triggered stream. quenched + mass_balance
                // are producer-specific and emitted elsewhere.
                let derived = cf_material::derive_m15d_events(&report.reactions, &state.reaction_registry);
                for evt in &derived.autoignited {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "reaction",
                        "autoignited",
                        json!({
                            "reaction_id": evt.reaction_id,
                            "pos": evt.pos,
                            "temperature_k": evt.temperature_k,
                            "pressure_kpa": evt.pressure_kpa,
                            "delta_h_kj": evt.delta_h_kj,
                            "moles_reacted": evt.moles_reacted,
                        }),
                        None,
                    );
                }
                for evt in &derived.chain_propagated {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "reaction",
                        "chain_propagated",
                        json!({
                            "reaction_id": evt.reaction_id,
                            "from_pos": evt.from_pos,
                            "to_pos": evt.to_pos,
                            "chain_depth": evt.chain_depth,
                        }),
                        None,
                    );
                }
                for evt in &derived.completed {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "reaction",
                        "completed",
                        json!({
                            "reaction_id": evt.reaction_id,
                            "pos": evt.pos,
                            "total_moles_reacted": evt.total_moles_reacted,
                            "cumulative_delta_h_j": evt.cumulative_delta_h_j,
                            "duration_ticks": evt.duration_ticks,
                        }),
                        None,
                    );
                }
                // Route phase-transition events to the recorder.
                for ev in &report.phase_transitions {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "material",
                        "phase_transition",
                        json!({
                            "material": ev.material,
                            "product_material": ev.product_material,
                            "from_state": ev.from_state.as_str(),
                            "to_state": ev.to_state.as_str(),
                            "pos": ev.pos,
                            "temperature_k": ev.temperature_k,
                            "latent_heat_j_per_kg": ev.latent_heat_j_per_kg,
                            "direction": ev.direction,
                        }),
                        None,
                    );
                }
                // Route cellular_step summary event when CA actually
                // moved pixels (per M15 spec event vocabulary).
                if report.ca.pixels_moved > 0 || !report.ca.dirty_chunks.is_empty() {
                    let dirty_chunks_json: Vec<serde_json::Value> = report
                        .ca
                        .dirty_chunks
                        .iter()
                        .map(|(cx, cy)| json!([cx, cy]))
                        .collect();
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "material",
                        "cellular_step",
                        json!({
                            "tick": report.ca.tick,
                            "parity": report.ca.parity,
                            "pixels_moved": report.ca.pixels_moved,
                            "dirty_chunks": dirty_chunks_json,
                        }),
                        None,
                    );
                }

                // feed them to the cycle. Per-tick PrecipitationCycle
                // tracks cloud-saturation; fires nucleation +
                // precipitation events when gates cross.
                //
                // **Perf** § iterate only the awake chunk set, not the
                // whole world. For a typical scene with sleeping
                // geometry this drops the scan from
                // O(width × height) ≈ 1M ops to O(awake_chunks × 4096)
                // which is typically <16K ops.
                let precip_evts = {
                    let EngineMutable {
                        chunked_terrain,
                        heat_field,
                        precipitation_cycle,
                        precipitation_config,
                        ..
                    } = &mut *state;
                    let terrain = chunked_terrain.as_mut().expect("chunked_terrain present");
                    let width = terrain.width_px as i64;
                    let height = terrain.height_px as i64;
                    let chunk_size = cf_terrain::chunked::CHUNK_SIZE as i64;
                    // Scan only awake chunks (with active_region==true)
                    // OR dirty chunks (recent edits). Falls back to all
                    // allocated chunks for the first tick when nothing
                    // is awake yet.
                    let awake = terrain.awake_chunk_coords();
                    let chunk_coords: Vec<(i32, i32)> = if awake.is_empty() {
                        terrain.allocated_chunk_coords()
                    } else {
                        awake
                    };
                    let mut observed = 0u32;
                    // **Perf gate**: cap per-tick steam observations at
                    // 4096 so a steam-cloud-heavy scenario doesn't
                    // blow the per-tick budget.
                    const STEAM_OBS_CAP: u32 = 4096;
                    'scan: for (cx, cy) in &chunk_coords {
                        let chunk_origin_x = (*cx as i64) * chunk_size;
                        let chunk_origin_y = (*cy as i64) * chunk_size;
                        for ly in 0..chunk_size {
                            let y = chunk_origin_y + ly;
                            if y < 0 || y >= height {
                                continue;
                            }
                            for lx in 0..chunk_size {
                                if observed >= STEAM_OBS_CAP {
                                    break 'scan;
                                }
                                let x = chunk_origin_x + lx;
                                if x < 0 || x >= width {
                                    continue;
                                }
                                if terrain.material_at(x, y) != 50 {
                                    continue;
                                }
                                let altitude_px = (height - y) as f32;
                                let temp_k = heat_field.temperature_at_world(x as f32, y as f32);
                                precipitation_cycle.observe_steam_pixel(
                                    cf_material::PrecipitationInputs {
                                        material: 50,
                                        world_x: x as i32,
                                        world_y: y as i32,
                                        altitude_px,
                                        ambient_temp_k: temp_k,
                                        ambient_pressure_kpa: precipitation_config.reference_pressure_kpa,
                                        ambient_world: precipitation_cycle.world,
                                        pollutant_fraction_local: 0.0,
                                        tick: tick.0,
                                    },
                                );
                                observed = observed.saturating_add(1);
                            }
                        }
                    }
                    // Apply the cycle's pixel side-effects to terrain.
                    let _ = precipitation_cycle.apply_to_terrain(terrain, tick.0);
                    // Drain events for recorder routing below.
                    precipitation_cycle.drain_events()
                };
                let (nucleated, precipitations) = precip_evts;
                for ev in &nucleated {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "material",
                        "phase_nucleated",
                        json!({
                            "from_material": ev.from_material,
                            "to_material": ev.to_material,
                            "from": ev.from,
                            "to": ev.to,
                            "pos": ev.pos,
                            "altitude_px": ev.altitude_px,
                            "temperature_k": ev.temperature_k,
                        }),
                        None,
                    );
                }
                for ev in &precipitations {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "material",
                        "precipitation_started",
                        json!({
                            "material": ev.material,
                            "pos": ev.pos,
                            "saturation": ev.saturation,
                            "pollutant_fraction": ev.pollutant_fraction,
                            "ambient": ev.ambient,
                        }),
                        None,
                    );
                }
            }

            let cadence = self.config.checksum_cadence_ticks;
            if cadence > 0 && tick.0 % cadence == 0 {
                let actor_bytes = build_checksum_bytes(&state);
                let cs = sim_state_v1(tick, &state.rng, &actor_bytes);
                let sim_time_ms = state.clock.sim_time_ms();
                checksum_payload = Some((tick, sim_time_ms, cs.to_hex()));
                // M0.2-F4: emit a tick_sample summarizing the last `cadence` ticks.
                let stats = TickSampleStats::from_recent(&state.tick_durations_us, cadence as usize);
                tick_sample_payload = Some((tick, sim_time_ms, stats));
                if let Some(actor_state) = state.actor_state.as_ref() {
                    snapshot_payload = Some((tick, sim_time_ms, ActorWorldSnapshot::from(actor_state)));
                }
            }
        }
        let elapsed_us = start.elapsed().as_micros().min(u128::from(u64::MAX)) as u64;
        state.tick_durations_us.push(elapsed_us);
        // `TickSampleStats::from_recent` only ever reads the last `cadence_ticks` entries
        // (default 60). Cap the buffer well above that so long-running sessions without a
        // `scenario.reset` don't accumulate millions of dead entries (~1.7 MB/hr at 60 Hz).
        // Drain in batches so the trim cost amortises to O(1) per tick.
        if state.tick_durations_us.len() > TICK_DURATIONS_HISTORY_CAP * 2 {
            let drop = state.tick_durations_us.len() - TICK_DURATIONS_HISTORY_CAP;
            state.tick_durations_us.drain(..drop);
        }
        let new_tick = state.clock.tick().0;
        drop(state);
        // Publish the latest tick so the panic reporter records `system.panic` at the
        // current tick (preserves events.jsonl monotonic ordering).
        self.current_tick.store(new_tick, std::sync::atomic::Ordering::Relaxed);

        // Emit M1 events from the actor step.
        if let Some((tick, sim_time_ms, intent, mut report)) = step_report {
            // `AdvancedFireMode::Charge`, and the Grenade Launcher's
            // `AdvancedFireMode::Arc` mode. Mutates the in-flight projectile
            // list + the `report.spawned_projectiles` view so emit_actor_events
            // sees the correctly-typed projectile (or no projectile, for Arc)
            // and the right damage (for Charge).
            self.post_process_m6_fire_modes(&mut report);
            self.record_schedule_trace_marker("actor_collision_start");
            self.emit_actor_events(tick, sim_time_ms, &intent, &report);
            self.record_schedule_trace_marker("actor_collision_end");
            // STRICTLY between the actor-collision pass and the terrain
            // pass. Runs every tick (the pool is empty for pre-M14D
            // scenarios so the cost is negligible).
            self.record_schedule_trace_marker("projectile_pair_start");
            self.tick_m14d_projectile_pair(tick, sim_time_ms);
            self.record_schedule_trace_marker("projectile_pair_end");
            // Wired AFTER the projectile-pair pass + BEFORE the terrain
            // dirty-region flush so cascading cave-ins ride on the same
            // dirty batch as their primary. The pool is empty for
            // pre-M14E scenarios so the cost is zero.
            self.tick_m14e_structural_integrity(tick, sim_time_ms);
            // integrity pass — runs at the same N=15 cadence as the
            // M14E ceiling pass + drives the bulging → crack_advanced
            // → rupture cascade per lateral chunk. Runs immediately
            // after the ceiling pass so the union per-chunk budget is
            // bounded by VAL-CROSS-006.
            self.tick_m14f_lateral_collapse(tick, sim_time_ms);
            // thermal pass — drives the
            // [`cf_environment::classify_tile_thermal`] producer for
            // every actor zone the scenario flagged as resting against
            // a hot/cold tile. Runs BEFORE the aging pass so the
            // emitted `wound.created` events feed the aging cadence on
            // the same tick they fire.
            self.tick_m14g_thermal_contacts(tick, sim_time_ms);
            // fires one [`cf_material::classify_reaction`] call per
            // scenario-authored material contact at its `fire_tick`.
            self.tick_m14g_material_contacts(tick, sim_time_ms);
            // aging pass. Increments `age_ticks` every tick on every
            // wound; commits visible-state mutations (bandage soak,
            // scab, scar, dirt escalation, Frostbite → Necrosis) every
            // 5 ticks. Does NOT roll infection chance (deferred to
            // M14H per VAL-M14G-047).
            self.tick_m14g_wound_aging(tick, sim_time_ms);
            // biological aging clock (per-year stat degradation +
            // retirement + per-week terminal-roll), prosthetic wear,
            // phantom-limb panic cadence, and radiation→cancer
            // hand-off.
            self.m14i_tick(tick, sim_time_ms);
            // drain + drowning emission. Steps every verlet rope, advances
            // zip-line riders along the cable, and fires `actor.drowned`
            // when a submerged actor's breath reaches 0.
            self.m14j_tick(tick, sim_time_ms);
            // projectiles. Spec § "Doppler shift on supersonic projectile
            // flyby": "audio.doppler_shifted fires per tick with the
            // resolved doppler_factor". Iterates `sim.projectiles` and
            // emits the 4 cosmetic spatial-resolve events for each live
            // projectile.
            self.emit_m12b_per_tick_projectile_audio(tick, sim_time_ms);
            // auto-triage / auto-repair missions on fresh DYING +
            // chassis-module-degraded transitions, and emit
            // `ai.auto_triage_applied` + `ai.auto_repair_progressed`
            // for missions whose deadlines elapsed this tick.
            self.emit_m7_auto_triage_repair_events(tick, sim_time_ms, &report);
            // mission director — phase pacing, reinforcement waves,
            // mini-boss damage + phase ability, objective-graph
            // branching + optional offers. Opt-in via the scenario
            // manifest's `phase_state` / `reinforcement_waves` /
            // `boss_state` / `objective_graph` fields.
            self.emit_m7_mission_director_events(tick, sim_time_ms, &report);
            // per-event mood / stress / faction deltas the spec
            // mandates beyond the scenario-start baseline emission. Ally
            // killed (-15), kill scored (+5), and wounded (-10) on every
            // hit; sustained-combat stress pump on each weapon fired;
            // friendly-fire relationship shift (-30) on intra-faction
            // hits. The helpers live on `M7AiWorld`; this method walks
            // the per-tick `StepReport` and records the resulting
            // `ai.mood_changed`, `ai.stress_threshold_crossed`, and
            // `ai.faction_allegiance_changed` payloads.
            self.emit_m7_mood_stress_faction_events(tick, sim_time_ms, &report);
        }
        // this tick to the next-tick AI pending queue. Clear the staging
        // buffer so each tick produces a fresh batch.
        if let Ok(mut s) = self.state.write() {
            let staged = std::mem::take(&mut s.pending_alarms_staging);
            s.pending_alarms = staged;
        }

        if let Some((tick, sim_time_ms, hex)) = checksum_payload {
            // the `chunk_summary` field per M3.md spec literal "And it
            // appears in the determinism.sim_checksum payload's
            // chunk-summary field". Format: ordered array of
            // {cx, cy, hex} so the JSON serialises deterministically and
            // M4 cross-OS verifiers can diff per-chunk.
            let chunk_summary: Vec<serde_json::Value> = self
                .state
                .read()
                .ok()
                .and_then(|s| s.chunked_terrain.as_ref().map(|t| t.chunk_summary_entries()))
                .unwrap_or_default()
                .into_iter()
                .map(|(cx, cy, hex)| json!({"cx": cx, "cy": cy, "hex": hex}))
                .collect();
            self.recorder.record(
                tick,
                sim_time_ms,
                "determinism",
                "sim_checksum",
                json!({
                    "checksum_hex": hex,
                    "algorithm": CHECKSUM_ALGORITHM,
                    "scope": CHECKSUM_SCOPE,
                    "cadence_ticks": self.config.checksum_cadence_ticks,
                    "tick_rate_hz": self.config.tick_rate_hz,
                    "seed": self.config.seed,
                    "chunk_summary": chunk_summary,
                }),
                None,
            );
        }
        if let Some((tick, sim_time_ms, stats)) = tick_sample_payload {
            self.recorder.record(
                tick,
                sim_time_ms,
                "system",
                "tick_sample",
                json!({
                    "tick_rate_hz": self.config.tick_rate_hz,
                    "window_ticks": stats.window_ticks,
                    "avg_tick_ms": stats.avg_tick_ms,
                    "max_tick_ms": stats.max_tick_ms,
                    "p99_tick_ms": stats.p99_tick_ms,
                    "samples_observed": stats.samples_observed,
                }),
                None,
            );
        }
        if let Some((tick, sim_time_ms, snapshot)) = snapshot_payload {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "actor_snapshot",
                json!({
                    "actors": snapshot.actors,
                    "player_actor_id": snapshot.player_actor_id,
                }),
                None,
            );
        }

        // M1.5 / M2: emit terrain dig events (always emit `tool_action_started`,
        // then `terrain_carved` or `tool_refused` based on outcome). The
        // `source: chunked|strip` field lets replay viewers tell M1.5 strip
        // digs from M2 chunked-terrain digs.
        let mut dig_validity_update: Option<(u64, ToolValidityUpdate)> = None;
        if let Some((tick, sim_time_ms, evt)) = dig_payload {
            let dig_source = match evt.source() {
                IntentSource::Human => "human",
                IntentSource::Cfctl => "cfctl",
                IntentSource::Ai => "ai",
                IntentSource::Replay => "replay",
            };
            let mode = match &evt {
                DigEvent::Strip { .. } => "strip",
                DigEvent::Chunked { .. } => "chunked",
            };
            let action_id = self.recorder.record(
                tick,
                sim_time_ms,
                "terrain",
                "tool_action_started",
                json!({
                    "tool": "digger",
                    "mode": mode,
                    "source": dig_source,
                    "origin": evt.origin(),
                    "explicit_target": evt.outcome_target_string(),
                }),
                None,
            );
            // M3 re-open (2026-05-13) fix #5: emit the spec-aligned
            // `equipment.tool_action_started` mirror so consumers that read
            // the M3 spec text literally see the event under the
            // `equipment.*` category. The terrain.* event is retained for
            // back-compat with existing replays + the BP3 test manifest.
            // Both share the same parent_event_id (None at start; the
            // terminal `equipment.tool_action_completed` chains back to
            // `action_id`).
            let equipment_action_id = self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "tool_action_started",
                json!({
                    "tool": "digger",
                    "mode": mode,
                    "source": dig_source,
                    "origin": evt.origin(),
                    "explicit_target": evt.outcome_target_string(),
                    "terrain_action_id": action_id.clone(),
                }),
                Some(action_id.clone()),
            );
            match evt {
                DigEvent::Strip { outcome, .. } => match outcome {
                    cf_terrain::DigOutcome::Carved {
                        strip_id,
                        material,
                        bbox_min,
                        bbox_max,
                        damage_applied,
                        hp_remaining,
                        broken,
                    } => {
                        dig_validity_update = Some((tick.0, ToolValidityUpdate::Carve));
                        // must be schema-compatible with the chunked path
                        // (BreachStrip replaceability). Compute mask_id via
                        // the same recipe (tool_id, dig_radius, mask_shape),
                        // emit material_ids[], pixel_count + dirty_chunks[]
                        // alongside the strip-specific extras.
                        let strip_material_id = cf_terrain::material_id_from_name(&material).unwrap_or(0);
                        let mut hasher = blake3::Hasher::new();
                        hasher.update(b"digger");
                        hasher.update(&12.0_f32.to_le_bytes());
                        hasher.update(b"circle");
                        let strip_mask_id = hex::encode(&hasher.finalize().as_bytes()[..16]);
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "terrain_carved",
                            json!({
                                "tick": tick.0,
                                "mode": "strip",
                                "tool_id": "digger",
                                "mask_id": strip_mask_id,
                                // requires scalar `material_id` and `bbox`
                                // in (x, y, w, h) tuple form.
                                "material_id": strip_material_id,
                                "bbox": { "min": bbox_min, "max": bbox_max },
                                "bbox_xywh": [
                                    bbox_min[0],
                                    bbox_min[1],
                                    (bbox_max[0] - bbox_min[0]).max(0.0),
                                    (bbox_max[1] - bbox_min[1]).max(0.0),
                                ],
                                "material": material.clone(),
                                "material_before": material.clone(),
                                "material_after": if broken { "air" } else { &material },
                                "material_ids": [strip_material_id],
                                "dominant_material_id": strip_material_id,
                                "pixel_count": 1u32,
                                "removed_count": 1u32,
                                "debris_count": 0u32,
                                "dirty_chunks": serde_json::Value::Array(Vec::new()),
                                "count": 1u32,
                                "strip_id": strip_id,
                                "damage_applied": damage_applied,
                                "hp_remaining": hp_remaining,
                                "broken": broken,
                            }),
                            Some(action_id.clone()),
                        );
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "terrain_breach_stub",
                            json!({
                                "strip_id": "stub",
                                "tick": tick.0,
                                "broken": broken,
                            }),
                            Some(action_id),
                        );
                    }
                    cf_terrain::DigOutcome::Refused {
                        reason,
                        strip_id,
                        material,
                        bbox_min,
                        bbox_max,
                    } => {
                        dig_validity_update = Some((
                            tick.0,
                            ToolValidityUpdate::Refuse {
                                reason: reason.clone(),
                                target: strip_id.clone(),
                            },
                        ));
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "tool_refused",
                            json!({
                                // requires `tool_id` + `target_material_id`.
                                "tool_id": "digger",
                                "target_material_id": material.as_ref().and_then(|m| cf_terrain::material_id_from_name(m)),
                                "reason": reason,
                                "mode": "strip",
                                "strip_id": strip_id,
                                "material": material,
                                "bbox_min": bbox_min,
                                "bbox_max": bbox_max,
                            }),
                            Some(action_id),
                        );
                    }
                },
                DigEvent::Chunked {
                    outcome, aim, target, ..
                } => match outcome {
                    cf_terrain::ChunkedCarveOutcome::Carved(stats) => {
                        dig_validity_update = Some((tick.0, ToolValidityUpdate::Carve));
                        let mat_name = cf_terrain::material_affordance(stats.dominant_material)
                            .map(|m| m.name)
                            .unwrap_or("unknown");
                        let dirty: Vec<serde_json::Value> = stats
                            .dirty_chunks
                            .iter()
                            .map(|c| json!({"cx": c.cx, "cy": c.cy}))
                            .collect();
                        // (tool_id, dig_radius, mask_shape) so replay
                        // determinism holds — same carve at same spot
                        // produces the same mask_id. Mask shape is a
                        // circle with 12-px radius for the digger; the
                        // hash inputs are pure-data so wall-clock time
                        // doesn't leak in.
                        // position-independent per spec implementer-notes
                        // ("blake3 hash over (mask_shape, tool_id,
                        // dig_radius)"). Position lives on the event's
                        // `pos`/`bbox` fields; identical carve shapes at
                        // different positions now share a mask_id.
                        let mut hasher = blake3::Hasher::new();
                        hasher.update(b"digger");
                        hasher.update(&12.0_f32.to_le_bytes());
                        hasher.update(b"circle");
                        let mask_id = hex::encode(&hasher.finalize().as_bytes()[..16]);
                        // Spawn debris (capped at 100 per event per spec
                        // "Debris cap per event"). We cap the debris count
                        // to keep render + replay readable.
                        const DEBRIS_CAP: u32 = 100;
                        let debris_count = stats.count.min(DEBRIS_CAP);
                        let debris_capped = stats.count > DEBRIS_CAP;
                        let chunk_carved_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "terrain_carved",
                            json!({
                                "tick": tick.0,
                                "mode": "chunked",
                                "tool_id": "digger",
                                "mask_id": mask_id,
                                // requires `material_id` (scalar dominant id)
                                // alongside `material_ids[]` AND `pixel_count`
                                // for parity with the strip emit (BreachStrip
                                // replaceability contract).
                                "material_id": stats.dominant_material,
                                "bbox": { "min": stats.bbox_min, "max": stats.bbox_max },
                                "bbox_xywh": [
                                    stats.bbox_min[0],
                                    stats.bbox_min[1],
                                    stats.bbox_max[0].saturating_sub(stats.bbox_min[0]).saturating_add(1),
                                    stats.bbox_max[1].saturating_sub(stats.bbox_min[1]).saturating_add(1),
                                ],
                                "pos": stats.bbox_min,
                                "material": mat_name,
                                "material_ids": [stats.dominant_material],
                                "dominant_material_id": stats.dominant_material,
                                "pixel_count": stats.count,
                                "count": stats.count,
                                "removed_count": stats.count,
                                "debris_count": debris_count,
                                "aim": aim,
                                "target": target,
                                "dirty_chunks": dirty,
                            }),
                            Some(action_id.clone()),
                        );
                        // states. Emit `terrain.material_state_changed`
                        // (Pristine → Destroyed band crossing) +
                        // `terrain.pixel_removed` (integrity reached 0) +
                        // `terrain.debris_spawned` (debris particles
                        // spawned) per chunk-carve event. At M9 this is
                        // emitted PER CARVE EVENT (not per pixel) to keep
                        // the event volume bounded; M14+ refines to a
                        // per-pixel integrity ladder with full cascade
                        // depth.
                        let pos_min = stats.bbox_min;
                        let _band_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "material_state_changed",
                            json!({
                                "pos": pos_min,
                                "material_id": stats.dominant_material,
                                "material_name": mat_name,
                                "from_band": "Pristine",
                                "to_band": "Destroyed",
                                "integrity_before": 1.0,
                                "integrity_after": 0.0,
                                "cause": "dig",
                                "parent_event_id": chunk_carved_id.clone(),
                            }),
                            Some(chunk_carved_id.clone()),
                        );
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "pixel_removed",
                            json!({
                                "pos": pos_min,
                                "was_material": stats.dominant_material,
                                "was_material_name": mat_name,
                                "cascade_cause": "direct_damage",
                                "parent_event_id": chunk_carved_id.clone(),
                            }),
                            Some(chunk_carved_id.clone()),
                        );
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "debris_spawned",
                            json!({
                                "pos": pos_min,
                                "material_id": stats.dominant_material,
                                "material_name": mat_name,
                                "debris_count": debris_count,
                                "kind": "dig_debris",
                                "parent_event_id": chunk_carved_id.clone(),
                            }),
                            Some(chunk_carved_id.clone()),
                        );
                        // pixels decay on neighbor destruction. After the
                        // carve clears its bbox, walk the perimeter and
                        // apply REAL integrity decay to neighbors below
                        // the cascade-threshold gate (default 0.6). Each
                        // affected neighbor produces one
                        // `terrain.cascade_triggered` event with the
                        // before/after integrity + band. cascade_depth=1
                        // at M9 — no recursion past direct 4-neighbors.
                        if stats.count > 0 {
                            // Re-acquire the engine state write lock so we
                            // can mutate the per-pixel integrity grid. The
                            // outer `state` was dropped before the dig
                            // event emit loop — this matches the existing
                            // `self.state.write()` pattern used by the
                            // dirty-rect accumulator below.
                            let cascades = match self.state.write() {
                                Ok(mut s) => match s.chunked_terrain.as_mut() {
                                    Some(t) => t.apply_cascade_to_carve_perimeter(
                                        stats.bbox_min,
                                        stats.bbox_max,
                                        Some(&chunk_carved_id),
                                    ),
                                    None => Vec::new(),
                                },
                                Err(_) => Vec::new(),
                            };
                            let affected_count = cascades.len() as u32;
                            for cascade in &cascades {
                                self.recorder.record(
                                    tick,
                                    sim_time_ms,
                                    "terrain",
                                    "cascade_triggered",
                                    json!({
                                        "from_pos": cascade.from_pos,
                                        "to_pos": cascade.to_pos,
                                        "cascade_depth": cascade.depth,
                                        "cascade_threshold": cascade.threshold,
                                        "cascade_decay_pct": cf_terrain::DEFAULT_CASCADE_DECAY_PCT,
                                        "affected_count": affected_count,
                                        "material_id": cascade.material_id,
                                        "material_name": cascade.material_name,
                                        "integrity_before": cascade.integrity_before,
                                        "integrity_after": cascade.integrity_after,
                                        "from_band": cascade.from_band.as_str(),
                                        "to_band": cascade.to_band.as_str(),
                                        "destroyed_neighbor": cascade.destroyed_neighbor,
                                        "reason": "neighbor_destroyed",
                                        "parent_event_id": chunk_carved_id.clone(),
                                    }),
                                    Some(chunk_carved_id.clone()),
                                );
                                if cascade.destroyed_neighbor {
                                    self.recorder.record(
                                        tick,
                                        sim_time_ms,
                                        "terrain",
                                        "pixel_removed",
                                        json!({
                                            "pos": cascade.to_pos,
                                            "was_material": cascade.material_id,
                                            "was_material_name": cascade.material_name,
                                            "cascade_cause": "neighbor_destroyed",
                                            "parent_event_id": chunk_carved_id.clone(),
                                        }),
                                        Some(chunk_carved_id.clone()),
                                    );
                                }
                            }
                        }
                        // M2: emit a per-pixel dislodged event for the
                        // first N pixels (capped) so the cause chain
                        // covers the spawn_material debris. Per-pixel
                        // events are rate-limited to one summary event
                        // when debris_count > 8 (keeps event log volume
                        // bounded for large carves).
                        let spawn_mat =
                            cf_terrain::material_affordance(stats.dominant_material).and_then(|a| a.spawn_material);
                        if debris_count > 0 {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "terrain",
                                "terrain_pixel_dislodged",
                                json!({
                                    "pos": stats.bbox_min,
                                    "source_material": mat_name,
                                    "source_material_id": stats.dominant_material,
                                    "spawn_material": spawn_mat
                                        .map(cf_terrain::material_name_from_id),
                                    "spawn_material_id": spawn_mat,
                                    "count": debris_count,
                                    "child_pixel_id": format!("{}:{}:{}",
                                        stats.bbox_min[0],
                                        stats.bbox_min[1],
                                        tick.0),
                                }),
                                Some(chunk_carved_id.clone()),
                            );
                            if debris_capped {
                                self.recorder.record(
                                    tick,
                                    sim_time_ms,
                                    "terrain",
                                    "debris_capped",
                                    json!({
                                        "capped": true,
                                        "requested_count": stats.count,
                                        "granted_count": debris_count,
                                    }),
                                    Some(chunk_carved_id.clone()),
                                );
                            }
                        }
                        // M2 also emits a `material.chunk_dirtied` event per
                        // dirty chunk so the M5.6 active material kernel can
                        // pick up the same vocabulary later.
                        for chunk in &stats.dirty_chunks {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "material",
                                "chunk_dirtied",
                                json!({
                                    "cx": chunk.cx,
                                    "cy": chunk.cy,
                                    "cause": "dig",
                                }),
                                Some(chunk_carved_id.clone()),
                            );
                        }
                        // M3 re-open (2026-05-13): instead of emitting a
                        // per-carve `terrain.terrain_dirty_region_batch`
                        // (which made the "ONE per tick coalesced" spec
                        // contract a lie when two carves landed in the same
                        // tick), push every dirty chunk into the engine's
                        // per-tick accumulator. The end-of-tick flush in
                        // `drive_tick` emits exactly one batch with all
                        // `source_event_ids[]` and a coalesced rect list
                        // bounded by the ≤25-rect budget. See `specs/active/M3.md`
                        // § Re-opened gaps, scenarios 2-4.
                        if let Ok(mut s) = self.state.write() {
                            for c in &stats.dirty_chunks {
                                let origin = c.pixel_origin();
                                s.pending_dirty_rects.push(PendingDirtyRect {
                                    source_event_id: chunk_carved_id.clone(),
                                    cx: c.cx,
                                    cy: c.cy,
                                    min: [origin[0], origin[1]],
                                    max: [
                                        origin[0] + cf_terrain::CHUNK_SIZE as i64,
                                        origin[1] + cf_terrain::CHUNK_SIZE as i64,
                                    ],
                                });
                            }
                            // Update cumulative counters.
                            s.total_carve_events = s.total_carve_events.saturating_add(1);
                            s.total_debris_spawned = s.total_debris_spawned.saturating_add(debris_count as u64);
                        }
                    }
                    cf_terrain::ChunkedCarveOutcome::Refused(refusal) => {
                        let mat_name = cf_terrain::material_affordance(refusal.material)
                            .map(|m| m.name)
                            .unwrap_or("unknown");
                        dig_validity_update = Some((
                            tick.0,
                            ToolValidityUpdate::Refuse {
                                reason: refusal.reason.to_string(),
                                target: Some(format!("chunked:{mat_name}")),
                            },
                        ));
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "tool_refused",
                            json!({
                                // requires `tool_id` + `target_material_id`.
                                "tool_id": "digger",
                                "target_material_id": refusal.material,
                                "reason": refusal.reason,
                                "mode": "chunked",
                                "material": mat_name,
                                "material_id": refusal.material,
                                "probe_at": refusal.probe_at,
                            }),
                            Some(action_id),
                        );
                    }
                    cf_terrain::ChunkedCarveOutcome::NoOp(noop) => {
                        dig_validity_update = Some((
                            tick.0,
                            ToolValidityUpdate::Refuse {
                                reason: "out_of_range".to_string(),
                                target: None,
                            },
                        ));
                        self.recorder.record(
                            tick,
                            sim_time_ms,
                            "terrain",
                            "tool_refused",
                            json!({
                                // refusal has no target material; emit
                                // tool_id only.
                                "tool_id": "digger",
                                "reason": "out_of_range",
                                "mode": "chunked",
                                "probe_at": Some(noop.probe_at),
                            }),
                            Some(action_id.clone()),
                        );
                    }
                },
            }
            // M3 re-open (2026-05-13) fix #5: emit the spec-aligned
            // `equipment.tool_action_completed` terminus. Result derives from
            // the dig_validity_update set above (Carve → "carved";
            // Refuse → "refused" with reason). Parent chains back to the
            // `equipment.tool_action_started` mirror so consumers walk the
            // equipment.* chain end-to-end.
            let (outcome_label, refusal_reason) = match &dig_validity_update {
                Some((_, ToolValidityUpdate::Carve)) => ("carved", None),
                Some((_, ToolValidityUpdate::Refuse { reason, .. })) => ("refused", Some(reason.clone())),
                None => ("noop", None),
            };
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "tool_action_completed",
                json!({
                    "tool": "digger",
                    "result": outcome_label,
                    "reason": refusal_reason,
                    "tool_action_started_id": equipment_action_id.clone(),
                }),
                Some(equipment_action_id),
            );
        }
        // M4A: persist tool-validity update for the HUD + observe consumers.
        if let Some((update_tick, update)) = dig_validity_update {
            // M11 § DR-012: snapshot pre-state so we can emit
            // ux.tool_validity_changed for the actual transition.
            let (from_state, prev_reason) = {
                let s = self.state.read().expect("engine state poisoned");
                let was_valid = s.hud_tool_validity.valid;
                let had_carve = s.hud_tool_validity.last_carve_tick.is_some();
                let had_refusal = s.hud_tool_validity.last_refusal_tick.is_some();
                let from = if !had_carve && !had_refusal {
                    "ready"
                } else if was_valid {
                    "valid"
                } else {
                    "invalid"
                };
                (from.to_string(), s.hud_tool_validity.last_refusal_reason.clone())
            };
            let mut state = self.state.write().expect("engine state poisoned");
            let to_state: &'static str;
            let emit_reason: Option<String>;
            match update {
                ToolValidityUpdate::Carve => {
                    state.hud_tool_validity.last_carve_tick = Some(update_tick);
                    state.hud_tool_validity.valid = true;
                    to_state = "valid";
                    emit_reason = None;
                }
                ToolValidityUpdate::Refuse { reason, target } => {
                    state.hud_tool_validity.last_refusal_tick = Some(update_tick);
                    state.hud_tool_validity.last_refusal_reason = Some(reason.clone());
                    state.hud_tool_validity.last_refusal_target = target;
                    state.hud_tool_validity.valid = false;
                    to_state = "invalid";
                    emit_reason = Some(reason);
                }
            }
            let sim_time_ms = state.clock.sim_time_ms();
            drop(state);
            if from_state.as_str() != to_state {
                let mut payload = json!({
                    "from": from_state,
                    "to": to_state,
                });
                if let Some(r) = emit_reason.or(prev_reason) {
                    payload["reason"] = json!(r);
                }
                self.recorder.record_cosmetic(
                    cf_sim_core::Tick(update_tick),
                    sim_time_ms,
                    "ux",
                    "tool_validity_changed",
                    payload,
                    None,
                );
            }
        }

        // M1.5: emit AI events for each guard.
        for (tick, sim_time_ms, guard_id, report) in &ai_payloads {
            self.emit_guard_events(*tick, *sim_time_ms, *guard_id, report);
        }

        // managed in `m7_ai_world`. The M2 reactive guards still drive the
        // M2 FSM + projectile spawn; M7-A's stack overlays the reason-label
        // + thinking_layer_invoked + auto-triage/auto-repair surfaces.
        for (tick, sim_time_ms, guard_id, _) in &ai_payloads {
            self.emit_m7_ai_events(*tick, *sim_time_ms, *guard_id);
        }

        // M1.5: emit mission events.
        // M2 re-audit (2026-05-13): all mission.objective_* events chain to
        // their parent. objective_started chains to mission_started; the
        // remaining lifecycle events chain to the corresponding
        // objective_started.
        if let Some((tick, sim_time_ms, report)) = mission_payload {
            for id in &report.objective_started {
                let parent = self.state.read().ok().and_then(|s| s.mission_started_event_id.clone());
                // contain `objective_id` AND `kind`. We retain `objective`
                // as a backwards-compat alias. `kind` is the typed
                // `ObjectiveKind::category()` string (ReachZone →
                // "reach_zone", SurviveTimer → "survive_timer", etc.).
                let kind = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| {
                        s.mission
                            .as_ref()
                            .and_then(|m| m.objectives.iter().find(|o| &o.id == id).map(|o| o.kind.category()))
                    })
                    .unwrap_or("unknown");
                let event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_started",
                    json!({
                        "objective": id,
                        "objective_id": id,
                        "kind": kind,
                    }),
                    parent,
                );
                if let Ok(mut s) = self.state.write() {
                    s.mission_objective_started_event_ids
                        .insert(id.clone(), event_id.clone());
                    s.last_mission_event_id = Some(event_id);
                }
            }
            // milestones. The 100% milestone fires on the same tick as
            // `objective_completed` so the cause chain reads
            // `objective_updated{1.0} → objective_completed → mission_resolved`.
            for update in &report.objective_updated {
                let parent = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.mission_objective_started_event_ids.get(&update.objective_id).cloned());
                // `objective_id` per schema; `objective` retained as alias.
                let event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_updated",
                    json!({
                        "objective_id": update.objective_id,
                        "objective": update.objective_id,
                        "progress": update.progress,
                    }),
                    parent,
                );
                if let Ok(mut s) = self.state.write() {
                    s.last_mission_event_id = Some(event_id);
                }
            }
            // event id so `mission.mission_resolved` on the Won path can
            // chain back to it (spec literal cause chain).
            let mut last_completed_event_id: Option<String> = None;
            for id in &report.objective_completed {
                let parent = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.mission_objective_started_event_ids.get(id).cloned());
                // `objective_id` per schema; `objective` retained as alias.
                let event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "objective_completed",
                    json!({
                        "objective_id": id,
                        "objective": id,
                    }),
                    parent,
                );
                last_completed_event_id = Some(event_id.clone());
                if let Ok(mut s) = self.state.write() {
                    s.last_mission_event_id = Some(event_id);
                }
                // ejected (Ejected pilot reached the extraction zone),
                // promote the chassis pilot_state to Extracted so further
                // damage is fully suppressed.
                if let Ok(mut s) = self.state.write() {
                    let player_id = s.player_actor;
                    if let Some(pid) = player_id {
                        if let Some(sim) = s.actor_state.as_mut() {
                            if let Some(actor) = sim.world.actors.get_mut(&pid) {
                                if let Some(chassis) = actor.chassis.as_mut() {
                                    if chassis.mark_pilot_extracted() {
                                        let actor_id = pid.0;
                                        drop(s);
                                        self.recorder.record(
                                            tick,
                                            sim_time_ms,
                                            "chassis",
                                            "pilot_extracted",
                                            json!({"actor": actor_id, "via": "reach_zone"}),
                                            None,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // M3B can walk the cause chain from mission_resolved back to
            // the trigger objective_failed → ... → player_dead chain.
            //
            // `objective_failed` payload must include a `reason` field
            // (e.g. "timer_expired", "player_dead", "reactor_destroyed").
            // We derive it from the mission's final_result so each
            // objective_failed event carries the same reason vocabulary
            // as `mission.mission_resolved.loss_reason`.
            let derived_reason = report.final_result.as_ref().and_then(|r| match r {
                cf_mission::MissionResult::Lost { reason } => Some(reason.as_str().to_string()),
                _ => None,
            });
            let mut last_failed_event_id: Option<String> = None;
            for id in &report.objective_failed {
                let parent = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.mission_objective_started_event_ids.get(id).cloned());
                // `objective_id` per schema; `objective` retained as alias.
                let mut payload = json!({
                    "objective_id": id,
                    "objective": id,
                });
                if let Some(reason) = &derived_reason {
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("reason".into(), json!(reason));
                    }
                }
                let event_id = self
                    .recorder
                    .record(tick, sim_time_ms, "mission", "objective_failed", payload, parent);
                last_failed_event_id = Some(event_id.clone());
                if let Ok(mut s) = self.state.write() {
                    s.last_mission_event_id = Some(event_id);
                }
            }
            if let Some(result) = report.final_result {
                let payload = match &result {
                    cf_mission::MissionResult::Won => json!({"result": "won"}),
                    cf_mission::MissionResult::Lost { reason } => {
                        // attach show_me_why_event_id pointing at the player's
                        // last input.intent_received event (the divergence
                        // anchor M3B's replay viewer rewinds to). cf-ui surfaces
                        // a CTA button when this id is present. Also latched
                        // into MissionState so observe.once.mission carries
                        // the CTA flag without re-walking events.jsonl.
                        let show_me_why = self
                            .state
                            .read()
                            .ok()
                            .and_then(|s| s.last_player_input_event_id.clone());
                        if let Ok(mut s) = self.state.write() {
                            if let Some(mission) = s.mission.as_mut() {
                                mission.show_me_why_event_id = show_me_why.clone();
                                mission.show_replay_cta = show_me_why.is_some();
                            }
                        }
                        let mut p = json!({"result": "lost", "loss_reason": reason.as_str()});
                        if let Some(id) = show_me_why {
                            if let Some(obj) = p.as_object_mut() {
                                obj.insert("show_me_why_event_id".into(), json!(id));
                                obj.insert("show_replay_cta".into(), json!(true));
                            }
                        }
                        p
                    }
                    cf_mission::MissionResult::InProgress => json!({"result": "in_progress"}),
                    cf_mission::MissionResult::Aborted => json!({"result": "aborted"}),
                };
                // Chain into the last objective_failed (if any) on the same
                // tick — that's the most specific cause of the resolution.
                // For wins the parent is None (the chain walks back through
                // the most recent objective_completed via its own
                // parent_event_id link, but at M1.5 we don't have that link
                // wired into the objective_completed loop yet — additive
                // schema upgrade for M5+).
                //
                // M2 re-audit pass 4 (2026-05-13): when the loss reason is
                // PlayerDead, no objective_failed fires (the player-dead
                // check short-circuits in `cf_mission::step`), so
                // `last_failed_event_id` is None and the cause chain
                // breaks at the very first hop. Fall back to the player's
                // last status_changed event id so M10 walkers can hop
                // `mission_resolved → actor_status_changed(player DYING)
                // → wound_added → projectile_hit → ...` cleanly.
                let resolved_parent = if last_failed_event_id.is_some() {
                    last_failed_event_id.clone()
                } else if matches!(
                    &result,
                    cf_mission::MissionResult::Lost { reason }
                        if matches!(reason, cf_mission::LossReason::PlayerDead)
                ) {
                    self.state
                        .read()
                        .ok()
                        .and_then(|s| s.last_player_status_event_id.clone())
                } else if matches!(&result, cf_mission::MissionResult::Won) {
                    // chains mission_resolved → objective_completed (the
                    // last one).
                    last_completed_event_id.clone()
                } else {
                    None
                };
                let resolved_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "mission",
                    "mission_resolved",
                    payload,
                    resolved_parent,
                );
                // M2 re-audit (2026-05-13): lifecycle InProgress → Resolved.
                if let Ok(mut s) = self.state.write() {
                    if let Some(mission) = s.mission.as_mut() {
                        mission.lifecycle = cf_mission::MissionLifecycle::Resolved;
                    }
                    s.last_mission_event_id = Some(resolved_event_id);
                }
            }
            // any objective state change with a real `parent_event_id`.
            // Pick the most-specific mission event id this tick (in
            // priority order): mission_resolved > last objective_failed >
            // last objective_completed > any objective_updated/started.
            // Falls back to the engine's last mission_event_id stored in
            // state (covers `started` and `updated` events).
            let snapshot_parent: Option<String> = if last_failed_event_id.is_some() {
                last_failed_event_id.clone()
            } else if last_completed_event_id.is_some() {
                last_completed_event_id.clone()
            } else {
                self.state.read().ok().and_then(|s| s.last_mission_event_id.clone())
            };
            self.emit_initial_snapshots(tick, sim_time_ms, snapshot_parent.as_deref());
        }

        //   snapshot_actor every 15 ticks (250ms @ 60Hz)
        //   snapshot_terrain_summary every 1 second (60 ticks @ 60Hz, 120 @ 120Hz)
        // Implemented inline so the cadence rides the engine's
        // post-tick path. We use the engine's tick_rate_hz to scale the
        // periods so 120Hz runs honour the same wall-clock cadence.
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            let actor_period = (self.config.tick_rate_hz.max(1) as u64) / 4; // ~250ms
            let summary_period = self.config.tick_rate_hz.max(1) as u64; // 1 second
            let run_started_parent = self.state.read().ok().and_then(|s| s.run_started_event_id.clone());
            if actor_period > 0 && t.0 > 0 && t.0 % actor_period == 0 {
                self.emit_periodic_snapshot_actor(t, sim_time_ms, run_started_parent.clone());
            }
            if summary_period > 0 && t.0 > 0 && t.0 % summary_period == 0 {
                self.emit_periodic_snapshot_terrain_summary(t, sim_time_ms, run_started_parent.clone());
                // tied to the 1-second summary so M10 timeline + M11
                // reactor strip see fresh `pressure_state` +
                // `armor_layers` every wall-clock second irrespective
                // of tick rate. The reactor is invariant under most
                // ticks (HP only changes on hits), so 1Hz cadence is
                // ample for the HUD.
                self.emit_periodic_snapshot_reactor(t, sim_time_ms, run_started_parent.clone());
            }
            // since the last tick, announce it so the canonical checker can
            // verify the priority discipline (critical events never silently
            // disappear).
            let dropped_gameplay_now = self.recorder.dropped_gameplay_count();
            let last_reported = self
                .state
                .read()
                .ok()
                .map(|s| s.last_reported_dropped_gameplay)
                .unwrap_or(0);
            if dropped_gameplay_now > last_reported {
                self.recorder.record(
                    t,
                    sim_time_ms,
                    "system",
                    "critical_drop",
                    json!({
                        "dropped_gameplay_count_delta": dropped_gameplay_now - last_reported,
                        "dropped_gameplay_count_total": dropped_gameplay_now,
                        "reason": "recorder_capacity_exceeded",
                    }),
                    None,
                );
                if let Ok(mut s) = self.state.write() {
                    s.last_reported_dropped_gameplay = dropped_gameplay_now;
                }
            }
            // sample per `summary_period` ticks. Mirrors `system.tick_sample`
            // (existing) but exposes the spec-required category + event
            // type name so the M10 viewer and grading harness can filter.
            if summary_period > 0 && t.0 > 0 && t.0 % summary_period == 0 {
                let p99_tick_us = self
                    .state
                    .read()
                    .ok()
                    .map(|s| {
                        let mut samples = s.tick_durations_us.clone();
                        if samples.is_empty() {
                            0u64
                        } else {
                            samples.sort_unstable();
                            let idx = ((samples.len() as f64 * 0.99) as usize).min(samples.len() - 1);
                            samples[idx]
                        }
                    })
                    .unwrap_or(0);
                self.recorder.record(
                    t,
                    sim_time_ms,
                    "performance",
                    "tick_cost_sample",
                    json!({
                        "tick": t.0,
                        "tick_rate_hz": self.config.tick_rate_hz,
                        "p99_tick_us": p99_tick_us,
                        "p99_tick_ms": p99_tick_us as f64 / 1000.0,
                        "cadence_ticks": summary_period,
                    }),
                    run_started_parent,
                );
            }
        }

        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.tick_chassis_eject_for_all(t, sim_time_ms);
        }

        // stamina drain, cinematic countdown + transition, lean integration,
        // cover sampling, stealth-meter integration, inventory weight recompute,
        // and the WeaponSwap state machine. See `specs/active/M6.md` §
        // "Actor controller depth" and § "Inventory: ... weight + drop/pickup".
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.tick_m6_actor_state(t, sim_time_ms);
            self.tick_m6_equipment(t, sim_time_ms);
            self.tick_m6_perception(t, sim_time_ms);
            self.tick_m6_squad(t, sim_time_ms);
        }

        // M8 cfctl surfaces seed — `cf_camera::tick_hit_stop` decays the
        // 50-200ms camera freeze pulse, `cf_killcam::tick` walks the
        // Idle → Recording → Playing → Done playback timeline (and resets
        // back to Idle on Done), and `cf_squad_ui::TagState::expire_old`
        // GCs MMB tags whose TTL elapsed. See `specs/active/M8.md` §
        // Camera + game feel / § Photo mode + Replay scrubber + Killcam /
        // § Pie menu (MMB tag).
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.tick_m8(t, sim_time_ms);
        }

        // M3 re-open (2026-05-13): flush the per-tick coalesced
        // `terrain.terrain_dirty_region_batch`. All carves during this tick
        // pushed their dirty chunks into `state.pending_dirty_rects`; here we
        // drain the accumulator, merge adjacent/overlapping rects via greedy
        // AABB union until count ≤ 25, and emit ONE batch with all
        // `source_event_ids[]`. Tracks `unupdated_areas` (count merged below
        // budget) + emits `terrain.forced_refresh_requested` when sustained
        // pressure exceeds the threshold for N consecutive ticks. See
        // `specs/active/M3.md` § Re-opened gaps.
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.record_schedule_trace_marker("terrain_start");
            self.flush_pending_dirty_batch(t, sim_time_ms);
            self.record_schedule_trace_marker("terrain_end");
        }

        // M4A: refresh HUD banners + captions + tool_validity caches AFTER all events
        // have been emitted for this tick. The cache reads world state directly so it
        // does not have to scan the event log on every observe().
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            let mut state = self.state.write().expect("engine state poisoned");
            self.refresh_hud_caches(&mut state, t, sim_time_ms);
            self.refresh_hud_chassis_banners(&mut state, t);
        }

        // and "an actor that loses the slot position emits
        // `squad.formation_slot_broken` and the solver reassigns next 2s
        // tick." We tick the player squad's reslot cadence + slot-broken
        // detection per frame; the reslot helper short-circuits when the
        // squad is idle, and the slot-broken helper short-circuits when
        // no member has wandered out of range.
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.tick_m7b_squad(t, sim_time_ms);
        }

        // - GAP-1 emits `trench.cover_state_changed` on actor segment
        //   boundary cross / stance change.
        // - GAP-2 ticks the AI-TRENCH-A-01 doctrine for opted-in actors
        //   and emits `ai.cover_decision`.
        // - GAP-3 ticks drainage on every sump-bearing segment under
        //   active rainfall.
        // - GAP-4 routes guard MG fire records into the per-segment
        //   breastwork HP gate and emits `trench.breastwork_breached`.
        // - GAP-5 ticks per-segment collapse on the audit cadence.
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.tick_m9b_cover_state_changes(t, sim_time_ms);
            self.tick_m9b_ai_cover_decisions(t, sim_time_ms);
            self.tick_m9b_drainage(t, sim_time_ms);
            self.tick_m9b_breastwork_hits(guard_fire_records.iter(), t, sim_time_ms);
            self.tick_m9b_collapse(t, sim_time_ms);
        }

        // Per-tick step that:
        //   1. Samples per-actor gravity overrides → applies acceleration +
        //      emits gravity.override_activated / gravity.override_deactivated.
        //   2. Samples per-actor wind force from the cell + aperture index →
        //      applies impulse + emits atmos.wind_force_applied.
        //   3. Runs gas stratification at 1/4 the tick rate → emits
        //      atmos.gas_stratified per cell that changed composition.
        // See `specs/active/M14B.md` § "Crates / modules touched" /
        // "Acceptance criteria".
        if let Some(t) = advanced {
            let sim_time_ms = self.state.read().map(|s| s.clock.sim_time_ms()).unwrap_or(0.0);
            self.tick_m14b(t, sim_time_ms);
        }

        // event per advanced tick. At ticks where `tick % cadence == 0` the
        // emitter fires `snapshot.baseline_emitted` (full state); otherwise it
        // fires `snapshot.delta_emitted` (JSON-Patch diff vs the previous
        // tick's state). Disabled when cadence is 0.
        if let Some(t) = advanced {
            self.emit_m4b_snapshot_for_tick(t);
        }

        advanced
    }

}
