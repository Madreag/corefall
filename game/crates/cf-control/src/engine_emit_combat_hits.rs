//! emit_combat_hits — projectile-vs-actor hit event emission.
//!
//! Extracted from engine_emit_actor.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::BTreeMap;

use serde_json::{json, Value};

use cf_actor::sim::StepReport;
use cf_actor::{ActorId, ControlIntent, Vec2};
use cf_sim_core::Tick;

use crate::engine::*;

impl M0Engine {
    pub(crate) fn emit_combat_hits(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        intent_event_id: &str,
        intent: &ControlIntent,
        report: &StepReport,
        swept_priority: &BTreeMap<(u64, u64), (u32, u32)>,
        swept_distance: &BTreeMap<(u64, u64), f32>,
        swept_entry_t: &BTreeMap<(u64, u64), f32>,
        swept_origin: &BTreeMap<u64, [f32; 2]>,
        swept_direction: &BTreeMap<u64, [f32; 2]>,
    ) {
        for hit in &report.hits {
            // M1 Gap C2: parent the hit to its originating projectile_spawned
            // event rather than the input.intent_received root, so a M3B
            // viewer can walk hit -> spawn -> weapon_fired -> intent in one chain.
            // Spawns persist on `EngineMutable::projectile_spawn_event_ids`
            // because hits commonly fire ticks after the spawn.
            //
            // a swept-collision shot can produce multiple hits for the same
            // projectile this tick, and every hit's `parent_event_id` must
            // resolve to the same spawn. Pruning happens after the hits loop
            // via `projectiles_resolved_this_tick`.
            let hit_parent = self
                .state
                .read()
                .ok()
                .and_then(|s| s.projectile_spawn_event_ids.get(&hit.projectile_id).cloned())
                .unwrap_or_else(|| intent_event_id.to_string());
            let projectile_hit_event_id = self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "projectile_hit",
                json!({
                    "projectile_id": hit.projectile_id,
                    "shooter": hit.shooter.0,
                    "target": hit.target.0,
                    "hit_position": [hit.hit_position.x, hit.hit_position.y],
                    "damage": hit.damage,
                    "zone": hit.zone,
                }),
                Some(hit_parent),
            );
            // M14 boundary: cf_trench::damage_route_for(target_cover_state,
            // body_zone) → DamageRoute decides whether this hit checks
            // the parapet material first (ParapetFirst) or routes directly
            // to the actor zone (ActorDirect). M9B owns the pure deriver
            // + the per-segment breastwork HP gate; M14 owns the energy
            // attenuation pass that consumes DamageRoute. Wire the call
            // here when M14's damage pipeline lands.
            // `combat.swept_collision` for every projectile-vs-actor hit
            // with the priority index/total computed from the
            // closer-first ordering built above. The event chains to
            // `combat.projectile_hit` so the cause walker can hop
            // swept_collision → projectile_hit → projectile_spawned →
            // weapon_fired → input.intent_received.
            let (priority_index, priority_total) = swept_priority
                .get(&(hit.projectile_id, hit.target.0))
                .copied()
                .unwrap_or((0u32, 1u32));
            let swept_dist = swept_distance
                .get(&(hit.projectile_id, hit.target.0))
                .copied()
                .unwrap_or(0.0);
            let swept_t = swept_entry_t
                .get(&(hit.projectile_id, hit.target.0))
                .copied()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            let ray_origin = swept_origin
                .get(&hit.projectile_id)
                .copied()
                .unwrap_or([hit.hit_position.x, hit.hit_position.y]);
            let ray_direction = swept_direction.get(&hit.projectile_id).copied().unwrap_or([1.0, 0.0]);
            let (stance_label, facing_label) = self
                .state
                .read()
                .ok()
                .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
                .and_then(|actors| {
                    actors.get(&hit.target).map(|a| {
                        let inputs = cf_actor::StanceInputs {
                            velocity: a.velocity,
                            on_ground: a.on_ground,
                            status: a.status,
                            crouch_active: a.crouch_active,
                            climb_active: a.climb_active,
                            jet_active: a.jet_active,
                            knockdown_ticks_remaining: a.knockdown_ticks_remaining,
                            dying_ticks_remaining: a.dying_dwell_ticks_remaining,
                            panic_freeze_ticks_remaining: a.panic_freeze_ticks_remaining,
                            ..cf_actor::StanceInputs::default()
                        };
                        let stance = cf_actor::derive_stance(inputs);
                        let facing_label = a.facing.as_str();
                        (stance.as_str().to_string(), facing_label.to_string())
                    })
                })
                .unwrap_or_else(|| ("idle".to_string(), "right".to_string()));
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "swept_collision",
                json!({
                    "projectile_id": hit.projectile_id,
                    "shooter_id": hit.shooter.0,
                    "target_id": hit.target.0,
                    "priority_index": priority_index,
                    "priority_total": priority_total,
                    "entry_point": [hit.hit_position.x, hit.hit_position.y],
                    "ray_origin": ray_origin,
                    "ray_direction": ray_direction,
                    "entry_t": swept_t,
                    "distance_traveled": swept_dist,
                    "zone": hit.zone,
                    "stance": stance_label,
                    "facing": facing_label,
                    "damage": hit.damage,
                    "energy_remaining": (hit.damage - 1.0).max(0.0),
                    "source_event_id": projectile_hit_event_id.clone(),
                }),
                Some(projectile_hit_event_id.clone()),
            );
            // M12B: route body-hit through the spatial-resolve helper so
            // the 4 cosmetic events fire for the victim's position.
            self.emit_audio_cue_for_actor(
                cf_audio::AudioCue::BodyHit {
                    zone: hit.zone.clone(),
                    caption: format!("body hit ({})", hit.zone),
                },
                tick,
                sim_time_ms,
                hit.target,
            );
            if hit.previous_status != hit.new_status {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "actor",
                    "actor_status_changed",
                    json!({
                        "actor": hit.target.0,
                        "previous_status": hit.previous_status.as_str(),
                        "new_status": hit.new_status.as_str(),
                        "cause": "projectile_hit",
                        "projectile_event": projectile_hit_event_id,
                    }),
                    Some(projectile_hit_event_id.clone()),
                );
            }
            // Gap C3: when this hit lands the killing blow (target transitions
            // through DYING / DEAD), latch the projectile_hit event id onto
            // the victim's actor state so the dwell-elapsed DEAD event AND
            // the next-tick inventory_dropped event AND the DYING latch all
            // resolve back to this projectile_hit (and from there to
            // weapon_fired -> input.intent_received).
            if matches!(hit.new_status, cf_actor::Status::Dying | cf_actor::Status::Dead) {
                if let Ok(mut s) = self.state.write() {
                    if let Some(sim) = s.actor_state.as_mut() {
                        if let Some(target) = sim.world.actors.get_mut(&hit.target) {
                            target.last_lethal_cause_event_id = Some(projectile_hit_event_id.clone());
                        }
                    }
                }
            }
            // production wiring point for `killcam.played` / `killcam.skipped`.
            // When the projectile transitions the player from a live status
            // into DYING / DEAD, fire `M0Engine::trigger_killcam_on_death`
            // with the killer's actor id so `Settings.killcam_enabled` gates
            // the 3 s killcam playback (or emits `killcam.skipped` when
            // disabled). See `specs/active/M8.md` § "Killcam on player death".
            let is_lethal_transition = matches!(hit.new_status, cf_actor::Status::Dying | cf_actor::Status::Dead)
                && !matches!(hit.previous_status, cf_actor::Status::Dying | cf_actor::Status::Dead);
            if is_lethal_transition {
                let victim_is_player = self.state.read().ok().and_then(|s| s.player_actor) == Some(hit.target);
                if victim_is_player {
                    if let Ok(mut s) = self.state.write() {
                        self.trigger_killcam_on_death(Some(hit.shooter.0), hit.target.0, tick, sim_time_ms, &mut s);
                    }
                }
            }
            // emit a typed `wound.created` for the projectile hit using
            // the cf-physics `classify_gunshot` producer. The legacy
            // `combat.wound_added` placeholder is gone — per VAL-M14G-027
            // the producer must dispatch a `WoundKind`-typed event rather
            // than the legacy generic emit. We still publish an internal
            // parent id so the M9 organ-cascade chain can link back to
            // the typed wound.
            let entry_zone = cf_wound::registry::ZoneId::from(hit.zone.as_str());
            let exited_backstop = hit
                .chassis_outcome
                .as_ref()
                .map(|o| !o.layers_breached.is_empty())
                .unwrap_or(false);
            let exit_zone = match hit.zone.as_str() {
                "torso" | "torso_front" => cf_wound::registry::ZoneId::from("torso_back"),
                "torso_back" => cf_wound::registry::ZoneId::from("torso_front"),
                "chest" | "chest_front" => cf_wound::registry::ZoneId::from("chest_back"),
                other => cf_wound::registry::ZoneId::from(other),
            };
            let severity_in = (hit.damage / 100.0).clamp(0.05, 1.0);
            let severity_out = (severity_in * 0.85).clamp(0.05, 1.0);
            let gunshot = cf_physics::classify_gunshot(
                entry_zone,
                exit_zone,
                severity_in,
                severity_out,
                exited_backstop,
            );
            let mut wound_event_id: Option<String> = None;
            for emit in gunshot {
                let id = self.m14g_emit_wound_created(
                    tick,
                    sim_time_ms,
                    hit.target.0,
                    emit,
                    Some(projectile_hit_event_id.clone()),
                );
                if wound_event_id.is_none() {
                    wound_event_id = id;
                }
            }
            let wound_event_id = wound_event_id.unwrap_or_else(|| projectile_hit_event_id.clone());
            // events from the production hit path. Spec § "Internal organ
            // damage / Internal circuit damage / Concussion bands" requires
            // schemas WITH emission sites. Schemas live in cf-replay/schemas/
            // (M5-locked); producers ladder up here at M9.
            //
            // Routing rule at M9:
            //   - human/organic actor → internal.organ_damaged (per-organ HP
            //     delta keyed off hit.zone)
            //   - robot/mechanical actor → internal.circuit_damaged (per-
            //     circuit HP delta)
            //
            // M14+ refines the per-zone organ graph + per-circuit topology;
            // M9 emits the M5-shaped payload with scalar from/to using the
            // hit damage as proxy for the organ/circuit HP delta.
            //
            // The actor's "is_robot" detection currently has no explicit
            // flag, so M9 uses the convention "robot teams" (team starts
            // with 'r') to surface the producer for both pathways without
            // touching the M1 actor type. The audit verifies M9 SHIPS both
            // emission sites; M14 will fix the routing.
            let target_kind_is_robot = self
                .state
                .read()
                .ok()
                .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
                .and_then(|actors| actors.get(&hit.target).map(|a| a.team.clone()))
                .map(|team| team.eq_ignore_ascii_case("red_robot") || team.eq_ignore_ascii_case("robot"))
                .unwrap_or(false);
            // M9 organ_id/circuit_id stubs with cf_internal's weighted
            // selection. Hit zone proximity drives the candidate pool;
            // the engine's seeded RNG picks deterministically.
            let graph_kind = if target_kind_is_robot {
                cf_internal::InternalGraphKind::Robot
            } else {
                cf_internal::InternalGraphKind::Humanoid
            };
            let rng_roll = if let Ok(mut s) = self.state.write() {
                (s.rng.next_u64() as f64 / u64::MAX as f64) as f32
            } else {
                0.5
            };
            let decision = cf_internal::route_internal_damage(graph_kind, hit.zone.as_str(), hit.damage, rng_roll);
            // M14 fallback when decision is None (below heavy threshold) —
            // keep emitting the legacy M9 organ/circuit shape so existing
            // consumers (M10 cause-chain walker) still observe per-hit
            // routing. The legacy zone→organ table is M14-aware (organ
            // names match the schema enum) so output is valid.
            let (organ_id, circuit_id, applied_internal_dmg, route_via_m14) = match decision {
                Some(d) => match d.graph_kind {
                    cf_internal::InternalGraphKind::Humanoid => (d.target_id, "cpu", d.applied_damage, true),
                    cf_internal::InternalGraphKind::Robot => ("heart", d.target_id, d.applied_damage, true),
                },
                None => {
                    let o = match hit.zone.as_str() {
                        "head" => "brain",
                        "torso" => "heart",
                        "arm_left" | "left_arm" | "forearm_left" | "hand_left" => "lungs_left",
                        "arm_right" | "right_arm" | "forearm_right" | "hand_right" => "lungs_right",
                        "leg_left" | "left_leg" | "shin_left" | "foot_left" => "kidneys_left",
                        "leg_right" | "right_leg" | "shin_right" | "foot_right" => "kidneys_right",
                        _ => "heart",
                    };
                    let c = match hit.zone.as_str() {
                        "head" => "sensor_array",
                        "torso" => "cpu",
                        _ => "power_core",
                    };
                    (o, c, hit.damage, false)
                }
            };
            let from_hp = 100.0_f32;
            let to_hp = (from_hp - applied_internal_dmg).max(0.0);
            if target_kind_is_robot {
                let circuit_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "internal",
                    "circuit_damaged",
                    json!({
                        "actor_id": hit.target.0,
                        "circuit_id": circuit_id,
                        "circuit_kind": cf_internal::circuit_kind(circuit_id),
                        "from_hp": from_hp,
                        "to_hp": to_hp,
                        "cause": "kinetic_pierce",
                        "source_hit_event_id": projectile_hit_event_id.clone(),
                        "route_via_m14": route_via_m14,
                    }),
                    Some(wound_event_id.clone()),
                );
                if to_hp <= 0.0 {
                    let circuit_destroyed_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "circuit_destroyed",
                        json!({
                            "actor_id": hit.target.0,
                            "circuit_id": circuit_id,
                            "circuit_kind": cf_internal::circuit_kind(circuit_id),
                            "cause": "kinetic_pierce",
                            "source_hit_event_id": projectile_hit_event_id.clone(),
                        }),
                        Some(circuit_event_id),
                    );
                    let (affliction_kind, severity) = match circuit_id {
                        "power_core" => ("power_failure", 1.0),
                        "cpu" => ("control_lost", 1.0),
                        "sensor_array" => ("blindness", 0.8),
                        "motor_controller_left_arm" => ("arm_motor_failure_left", 0.8),
                        "motor_controller_right_arm" => ("arm_motor_failure_right", 0.8),
                        "motor_controller_left_leg" => ("leg_motor_failure_left", 0.8),
                        "motor_controller_right_leg" => ("leg_motor_failure_right", 0.8),
                        "hydraulic_pump" => ("hydraulic_failure", 0.7),
                        "coolant_pump" => ("overheating", 0.6),
                        "oil_reservoir" | "fuel_tank" => ("fluid_leak", 0.6),
                        "comm_relay" => ("comm_lost", 0.4),
                        _ => ("circuit_shock", 0.5),
                    };
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "circuit_failure_cascade",
                        json!({
                            "actor_id": hit.target.0,
                            "circuit_id": circuit_id,
                            "circuit_kind": cf_internal::circuit_kind(circuit_id),
                            "affliction_kind": affliction_kind,
                            "severity": severity,
                            "source_event_id": circuit_destroyed_id.clone(),
                        }),
                        Some(circuit_destroyed_id.clone()),
                    );
                }
            } else {
                let organ_event_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "internal",
                    "organ_damaged",
                    json!({
                        "actor_id": hit.target.0,
                        "organ_id": organ_id,
                        "organ_kind": cf_internal::organ_kind(organ_id),
                        "from_hp": from_hp,
                        "to_hp": to_hp,
                        "cause": "kinetic_pierce",
                        "source_hit_event_id": projectile_hit_event_id.clone(),
                        "route_via_m14": route_via_m14,
                    }),
                    Some(wound_event_id.clone()),
                );
                if to_hp <= 0.0 {
                    let organ_destroyed_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "organ_destroyed",
                        json!({
                            "actor_id": hit.target.0,
                            "organ_id": organ_id,
                            "organ_kind": cf_internal::organ_kind(organ_id),
                            "cause": "kinetic_pierce",
                            "source_hit_event_id": projectile_hit_event_id.clone(),
                        }),
                        Some(organ_event_id),
                    );
                    // Each destroyed organ triggers a specific affliction
                    // per CCCP organ-failure-cascade table.
                    let (affliction_kind, severity) = match organ_id {
                        "brain" => ("brain_failure", 1.0),
                        "heart" => ("cardiac_arrest", 1.0),
                        "lungs_left" | "lungs_right" => ("respiratory_failure", 0.8),
                        "liver" => ("toxicosis", 0.6),
                        "kidneys_left" | "kidneys_right" => ("renal_failure", 0.7),
                        "spine" => ("paralysis", 0.9),
                        "stomach" | "intestines" | "pancreas" => ("internal_bleed", 0.6),
                        "eyes_left" | "eyes_right" => ("blindness", 0.5),
                        "ears_left" | "ears_right" => ("deafness", 0.4),
                        _ => ("organ_shock", 0.5),
                    };
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "internal",
                        "organ_failure_cascade",
                        json!({
                            "actor_id": hit.target.0,
                            "organ_id": organ_id,
                            "organ_kind": cf_internal::organ_kind(organ_id),
                            "affliction_kind": affliction_kind,
                            "severity": severity,
                            "source_event_id": organ_destroyed_id.clone(),
                        }),
                        Some(organ_destroyed_id.clone()),
                    );
                }
            }
            // M17 § origin reaction matrix — EVERY chassis-bearing hit emits
            // origin.shot_force_feedback + the per-origin concussion /
            // internal-shock / helmet-breach reaction (shared with the
            // m17_inject_hit acceptance injector).
            self.emit_m17_shot_reaction(
                hit.target,
                hit.damage,
                hit.zone.as_str(),
                &projectile_hit_event_id,
                &wound_event_id,
                tick,
                sim_time_ms,
            );
            // M1: hit-stop request (DR-055 placeholder). Triggers when damage
            // exceeds a critical threshold so the renderer can briefly freeze
            // the frame. Full hit-stop renderer effect lands at M5+ when the
            // damage grammar carries crit info.
            const CRITICAL_DAMAGE_THRESHOLD: f32 = 20.0;
            if hit.damage > CRITICAL_DAMAGE_THRESHOLD {
                // per determinism-island-contract.md.
                self.recorder.record_cosmetic(
                    tick,
                    sim_time_ms,
                    "ux",
                    "hit_stop_requested",
                    json!({
                        "actor": hit.target.0,
                        "shooter": hit.shooter.0,
                        "damage": hit.damage,
                        "duration_ms": 80,
                    }),
                    Some(projectile_hit_event_id.clone()),
                );
            }
            if let Some(outcome) = &hit.chassis_outcome {
                self.emit_chassis_events(
                    tick,
                    sim_time_ms,
                    hit.target,
                    outcome,
                    Some(projectile_hit_event_id.clone()),
                );
                // zone is destroyed by this hit, route the impulse through
                // a `cf_physics::Joint`. If `joint_impulse > joint_strength`
                // emit `attachable.detached`; if `>= gib_impulse_limit`
                // emit `attachable.gib_threshold_crossed` + the spawn data
                // `body.gib_created`. Each child attachable cascades per
                // `cf_actor::default_cascade_chain`.
                if outcome.zone_destroyed {
                    let zone_label = outcome
                        .zone
                        .map(|z| z.as_str().to_string())
                        .unwrap_or_else(|| hit.zone.clone());
                    // Hit damage proxies impulse magnitude for M14
                    // (real impulse routing arrives at M15 when ammo specs
                    // carry kinetic energy). Scale 50× so heavy hits cross
                    // typical joint thresholds.
                    let joint_impulse_magnitude = (hit.damage * 50.0).max(0.0);
                    let joint = cf_physics::Joint::default_for_zone(zone_label.as_str());
                    let eval = cf_physics::evaluate_joint(joint, joint_impulse_magnitude);
                    let origin_kind_label = self
                        .state
                        .read()
                        .ok()
                        .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
                        .and_then(|actors| {
                            actors.get(&hit.target).map(|a| {
                                if target_kind_is_robot {
                                    "robot".to_string()
                                } else {
                                    a.origin_id.clone()
                                }
                            })
                        })
                        .unwrap_or_else(|| "human".to_string());
                    let origin_kind = cf_actor::GibOriginKind::from_str_lossy(origin_kind_label.as_str());
                    if eval.gib {
                        // Emit gib_threshold_crossed + body.gib_created
                        // + per-child cascade events.
                        let gib_threshold_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "attachable",
                            "gib_threshold_crossed",
                            json!({
                                "actor_id": hit.target.0,
                                "attachable_id": zone_label,
                                "parent_zone": zone_label,
                                "joint_impulse": eval.impulse_in,
                                "gib_impulse_limit": joint.gib_impulse_limit,
                                "source_event_id": projectile_hit_event_id.clone(),
                                "cause": "kinetic_pierce",
                            }),
                            Some(projectile_hit_event_id.clone()),
                        );
                        let gib_spawn = cf_actor::GibSpawn::default_for_origin(origin_kind);
                        let gib_created_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "body",
                            "gib_created",
                            json!({
                                "actor_id": hit.target.0,
                                "parent_zone": zone_label,
                                "gib_particle": gib_spawn.particle,
                                "count": gib_spawn.count,
                                "spread_radians": gib_spawn.spread_radians,
                                "min_velocity": gib_spawn.min_velocity,
                                "max_velocity": gib_spawn.max_velocity,
                                "life_variation": gib_spawn.life_variation,
                                "spread_mode": match gib_spawn.spread_mode {
                                    cf_actor::SpreadMode::SpreadRandom => "SpreadRandom",
                                    cf_actor::SpreadMode::SpreadEven => "SpreadEven",
                                    cf_actor::SpreadMode::SpreadSpiral => "SpreadSpiral",
                                },
                                "inherits_velocity": gib_spawn.inherits_velocity,
                                "inherits_angular_velocity": gib_spawn.inherits_angular_velocity,
                                "ignores_team_hits": gib_spawn.ignores_team_hits,
                                "origin_kind": origin_kind.as_str(),
                                "spawn_position": [hit.hit_position.x, hit.hit_position.y],
                                "source_event_id": gib_threshold_id.clone(),
                            }),
                            Some(gib_threshold_id.clone()),
                        );
                        // Cascade — every child attachable gibs in turn.
                        for child in cf_actor::default_cascade_chain(zone_label.as_str()) {
                            self.recorder.record(
                                tick,
                                sim_time_ms,
                                "body",
                                "gib_cascade_triggered",
                                json!({
                                    "actor_id": hit.target.0,
                                    "parent_zone": zone_label,
                                    "child_zone": *child,
                                    "cascade_depth": 1u32,
                                    "source_event_id": gib_created_id.clone(),
                                    "reason": "parent_gibbed",
                                }),
                                Some(gib_created_id.clone()),
                            );
                        }
                    } else if eval.detach {
                        // Cleanly detached — toss the limb forward at the
                        // residual impulse. detach_velocity ∝ impulse_out
                        // / mass; M14 uses a scaled forward toss.
                        let detach_position = [hit.hit_position.x, hit.hit_position.y];
                        let detach_velocity = [
                            ray_direction[0] * (eval.impulse_out / 80.0).clamp(0.0, 500.0),
                            ray_direction[1] * (eval.impulse_out / 80.0).clamp(0.0, 500.0),
                        ];
                        let detached_event_id = self.recorder.record(
                            tick,
                            sim_time_ms,
                            "attachable",
                            "detached",
                            json!({
                                "actor_id": hit.target.0,
                                "attachable_id": zone_label,
                                "parent_zone": zone_label,
                                "joint_impulse": eval.impulse_in,
                                "joint_strength": joint.joint_strength,
                                "gib_impulse_limit": joint.gib_impulse_limit,
                                "detach_position": detach_position,
                                "detach_velocity": detach_velocity,
                                "damage_multiplier": joint.damage_multiplier,
                                "source_event_id": projectile_hit_event_id.clone(),
                                "cause": "kinetic_pierce",
                            }),
                            Some(projectile_hit_event_id.clone()),
                        );
                        // post-survival pass can promote it to phantom_limb.
                        // Chains phantom_limb.acquired to attachable.detached
                        // so the cause-chain walker traces back to the
                        // projectile hit.
                        self.m14i_record_phantom_limb_with_parent(
                            hit.target.0,
                            zone_label.as_str(),
                            tick,
                            sim_time_ms,
                            Some(detached_event_id),
                        );
                    }
                    // Emit `physics.impulse_propagated` for the joint that
                    // routed this hit — closes the impulse chain back to
                    // the projectile_hit event so the M10 cause walker
                    // resolves swept_collision → impulse_propagated.
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "physics",
                        "impulse_propagated",
                        json!({
                            "actor_id": hit.target.0,
                            "from_zone": zone_label,
                            "to_zone": "parent",
                            "impulse_in": eval.impulse_in,
                            "impulse_absorbed": eval.impulse_absorbed,
                            "impulse_out": eval.impulse_out,
                            "joint_strength": joint.joint_strength,
                            "damage_multiplier": joint.damage_multiplier,
                            "kind": "kinetic",
                            "source_event_id": projectile_hit_event_id.clone(),
                        }),
                        Some(projectile_hit_event_id.clone()),
                    );
                }
                // destruction is INSTANT DEATH per the CCCP decapitation rule.
                // Forcing `actor.hp = 0` here lets the existing status-change
                // pipeline emit `actor_status_changed → Dead` + brain death
                // events on the next tick.
                if outcome.lethal {
                    if let Ok(mut s) = self.state.write() {
                        if let Some(sim) = s.actor_state.as_mut() {
                            if let Some(target) = sim.world.actors.get_mut(&hit.target) {
                                target.hp = 0.0;
                            }
                        }
                    }
                }
                // breached, traverse the chassis interior modules in ray-
                // order via `cf_physics::traverse_ray`. Emits
                // `armor.penetration_ray_traversed` + per-module damage
                // events + spalling fragments.
                if !outcome.layers_breached.is_empty() {
                    self.emit_m14_penetration_ray(
                        tick,
                        sim_time_ms,
                        hit.target,
                        hit,
                        outcome,
                        ray_direction,
                        Some(projectile_hit_event_id.clone()),
                    );
                }
                // projectile is a tank-grade round, route the impact
                // through `heat_impact_producer` / `apfsds_impact_producer`
                // and emit `armor.era_pre_detonated` (strict ordering)
                // BEFORE `armor.heat_jet_traversed` / `armor.apfsds_long_rod_through`.
                // For HEAT/APFSDS we do NOT require armor breach (a HEAT
                // jet bypasses spaced armor per VAL-M14C-021, and APFSDS
                // can over-penetrate unarmored infantry per VAL-M14C-016);
                // the producer's own gates (5° cone, ERA reduction,
                // standoff curve) decide whether to emit per impact.
                let round_kind_for_hit = self
                    .state
                    .read()
                    .ok()
                    .and_then(|s| s.projectile_round_kinds.get(&hit.projectile_id).copied())
                    .unwrap_or(cf_equipment::RoundKind::Regular);
                if matches!(
                    round_kind_for_hit,
                    cf_equipment::RoundKind::Heat | cf_equipment::RoundKind::Apfsds
                ) {
                    self.emit_m14c_armor_events(
                        tick,
                        sim_time_ms,
                        hit.target,
                        hit,
                        round_kind_for_hit,
                        Some(projectile_hit_event_id.clone()),
                    );
                }
            }
            // an AP-round hit per spec § "Hit-stop on impact — Given melee
            // hit OR AP round hit". At M8 baseline, an AP-round hit is
            // detected via the closest available signal in the M6-M8
            // codebase: either the M5 chassis armor was actually breached
            // (`chassis_outcome.layers_breached` non-empty — literal AP
            // behavior) OR the projectile's damage cleared the
            // CRITICAL_DAMAGE_THRESHOLD (high-energy round indistinguishable
            // from AP at M8; M13/M14 fills explicit AP-round tiers per
            // `combat_projectile_hit_mo.json::ap_round_tier`). Honors
            // `Settings.hit_stop_enabled` and emits `camera.hit_stop` with
            // `trigger="ap_round_hit"`.
            let pierced_armor = hit
                .chassis_outcome
                .as_ref()
                .map(|c| !c.layers_breached.is_empty())
                .unwrap_or(false);
            let is_ap_round_hit = pierced_armor || hit.damage > CRITICAL_DAMAGE_THRESHOLD;
            let mut ap_hit_stop_payload: Option<serde_json::Value> = None;
            if is_ap_round_hit {
                if let Ok(mut s) = self.state.write() {
                    if s.settings.hit_stop_enabled {
                        cf_camera::trigger_hit_stop(&mut s.camera_state, 100);
                        let applied = s.camera_state.hit_stop_remaining_ms;
                        ap_hit_stop_payload = Some(
                            json!({"duration_ms": applied, "trigger": "ap_round_hit", "actor_id": Some(hit.target.0)}),
                        );
                    }
                }
            }
            if let Some(payload) = ap_hit_stop_payload {
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "camera", "hit_stop", payload, None);
            }
        }
        // multi-actor swept-collision hits all parent to the same spawn.
        // The pre-M14 code pruned per-hit which dropped the parent ref for
        // every subsequent hit of the same projectile (the priority queue
        // depended on this fix).
        {
            let mut resolved_ids: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
            for hit in &report.hits {
                resolved_ids.insert(hit.projectile_id);
            }
            if !resolved_ids.is_empty() {
                if let Ok(mut s) = self.state.write() {
                    for id in &resolved_ids {
                        s.projectile_spawn_event_ids.remove(id);
                        // alongside the spawn-id entry; the projectile is
                        // resolved this tick so future ticks can't read
                        // it again.
                        s.projectile_round_kinds.remove(id);
                    }
                }
            }
        }
    }
}
