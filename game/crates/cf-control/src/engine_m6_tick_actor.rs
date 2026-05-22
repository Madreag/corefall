//! M6 per-tick: actor state, equipment, perception, squad.
//!
//! Extracted from engine.rs.

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
    pub(crate) fn tick_m6_actor_state(&self, tick: Tick, sim_time_ms: f64) {
        const COVER_PROBE_OFFSET: f32 = 6.0;
        const STAMINA_EMIT_DELTA: f32 = 0.05;
        const SLOT_WEIGHT_RIFLE_KG: f32 = 8.0;
        const SLOT_WEIGHT_EMPTY_KG: f32 = 0.0;

        struct StanceTransition {
            actor: u64,
            from_stance: &'static str,
            to_stance: &'static str,
        }
        struct StaminaEmit {
            actor: u64,
            stamina: f32,
            sprinting: bool,
        }
        struct StealthEmit {
            actor: u64,
            stealth_meter: f32,
            spotted: bool,
        }
        struct WeightEmit {
            actor: u64,
            total_weight_kg: f32,
            forces_walk: bool,
        }
        struct SwapEmit {
            actor: u64,
            active_slot: u8,
        }
        struct ActionReject {
            actor: u64,
            reason: &'static str,
        }
        /// inventory-weight pass so the recorder can fire the
        /// `inventory.encumbrance_threshold_crossed` event after the
        /// write-guard is released.
        struct EncumbranceEmit {
            actor: u64,
            from_band: &'static str,
            to_band: &'static str,
            total_carried_kg: f32,
            max_carry_kg: f32,
            carry_ratio: f32,
            walk_speed_multiplier: f32,
            origin_id: String,
        }

        let mut stance_transitions: Vec<StanceTransition> = Vec::new();
        let mut stamina_emits: Vec<StaminaEmit> = Vec::new();
        let mut stealth_emits: Vec<StealthEmit> = Vec::new();
        let mut weight_emits: Vec<WeightEmit> = Vec::new();
        let mut swap_emits: Vec<SwapEmit> = Vec::new();
        let mut action_rejects: Vec<ActionReject> = Vec::new();
        let mut encumbrance_emits: Vec<EncumbranceEmit> = Vec::new();

        let tick_rate_hz = self.config.tick_rate_hz;
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return,
        };

        let observer_positions: Vec<(ActorId, cf_actor::Vec2, cf_actor::Vec2, f32)> = state
            .reactive_guards
            .keys()
            .filter_map(|gid| {
                state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(gid))
                    .map(|guard| {
                        let facing_sign = if guard.aim.x >= 0.0 { 1.0 } else { -1.0 };
                        let aim_vec = cf_actor::Vec2::new(facing_sign, 0.0);
                        (*gid, guard.position, aim_vec, 240.0_f32)
                    })
            })
            .collect();

        let actor_ids: Vec<ActorId> = state
            .actor_state
            .as_ref()
            .map(|sim| sim.world.actors.keys().copied().collect())
            .unwrap_or_default();

        for actor_id in actor_ids {
            // Snapshot terrain probes (left + right of actor) without holding
            // a mutable borrow on the actor itself.
            let probe = state.actor_state.as_ref().and_then(|sim| {
                sim.world.actors.get(&actor_id).map(|a| {
                    let half_x = a.half_extents.x.max(1.0);
                    let probe_x = half_x + COVER_PROBE_OFFSET;
                    let left = cf_actor::Vec2::new(a.position.x - probe_x, a.position.y);
                    let right = cf_actor::Vec2::new(a.position.x + probe_x, a.position.y);
                    let feet = cf_actor::Vec2::new(a.position.x, a.position.y + a.half_extents.y);
                    (a.position, a.velocity, left, right, feet, a.aim)
                })
            });
            let Some((actor_pos, _actor_vel, left_probe, right_probe, _feet_probe, _aim)) = probe else {
                continue;
            };
            let (left_solid, right_solid) = match state.chunked_terrain.as_ref() {
                Some(terrain) => {
                    let lm = terrain.material_at_world(left_probe.x, left_probe.y);
                    let rm = terrain.material_at_world(right_probe.x, right_probe.y);
                    (terrain.registry.is_solid(lm), terrain.registry.is_solid(rm))
                }
                None => (false, false),
            };
            let cover_side = match (left_solid, right_solid) {
                (true, true) => cf_actor::CoverSide::Both,
                (true, false) => cf_actor::CoverSide::Left,
                (false, true) => cf_actor::CoverSide::Right,
                (false, false) => cf_actor::CoverSide::None,
            };
            let cover_effectiveness = match (left_solid, right_solid) {
                (true, true) => 1.0,
                (true, false) | (false, true) => 0.7,
                (false, false) => 0.0,
            };

            // Stealth-meter target: take the worst (most visible) sightline
            // across all observer guards. We use the pure sight kernel from
            // cf-perception so the same numbers feed AI and HUD.
            let mut worst_instantaneous: f32 = 0.0;
            for (_gid, observer_pos, _observer_aim, max_range) in &observer_positions {
                let check = cf_perception::SightCheck {
                    observer: *observer_pos,
                    observer_facing_x: 1.0,
                    target: actor_pos,
                    view_cone_half_angle: 1.0,
                    max_range: *max_range,
                    occlusion_factor: 1.0,
                };
                let result = cf_perception::compute_sightline(check);
                if result.is_visible() && result.visibility > worst_instantaneous {
                    worst_instantaneous = result.visibility;
                }
            }

            let Some(actor) = state
                .actor_state
                .as_mut()
                .and_then(|sim| sim.world.actors.get_mut(&actor_id))
            else {
                continue;
            };

            // (a) Stamina step + auto-cancel + change emission.
            let stamina_before = actor.stamina.current;
            let sprinting_before = actor.stamina.sprinting;
            actor.stamina.step(tick_rate_hz);
            if actor.stamina.should_auto_cancel_sprint() {
                actor.sprint_active = false;
                actor.stamina.sprinting = false;
            }
            let stamina_changed = (actor.stamina.current - stamina_before).abs() >= STAMINA_EMIT_DELTA
                || actor.stamina.sprinting != sprinting_before;
            let stamina_now = actor.stamina.current;
            let sprinting_now = actor.stamina.sprinting;
            if stamina_changed {
                let last = state.m6_last_stamina_emit.get(&actor_id).copied().unwrap_or(-1.0);
                if (stamina_now - last).abs() >= STAMINA_EMIT_DELTA || last < 0.0 {
                    state.m6_last_stamina_emit.insert(actor_id, stamina_now);
                    stamina_emits.push(StaminaEmit {
                        actor: actor_id.0,
                        stamina: stamina_now,
                        sprinting: sprinting_now,
                    });
                }
            }

            let actor = state
                .actor_state
                .as_mut()
                .and_then(|sim| sim.world.actors.get_mut(&actor_id))
                .expect("actor still present");

            // (b) Cinematic countdown + transition.
            if actor.cinematic_ticks_remaining > 0 {
                actor.cinematic_ticks_remaining -= 1;
                if actor.cinematic_ticks_remaining == 0 {
                    let from_stance = actor.cinematic_kind.map(|s| s.as_str()).unwrap_or("idle");
                    let to_stance = match actor.cinematic_kind {
                        Some(cf_actor::Stance::Slide) => "crouching",
                        Some(cf_actor::Stance::Vault) => "stand",
                        Some(cf_actor::Stance::LadderClimb)
                        | Some(cf_actor::Stance::RopeClimb)
                        | Some(cf_actor::Stance::PipeClimb)
                        | Some(cf_actor::Stance::Climbing) => "stand",
                        Some(cf_actor::Stance::Dive) => "stand",
                        Some(cf_actor::Stance::StealthAttack) => "stand",
                        Some(cf_actor::Stance::KnifeThrow) => "stand",
                        _ => "stand",
                    };
                    if matches!(actor.cinematic_kind, Some(cf_actor::Stance::Slide)) {
                        actor.crouch_active = true;
                    }
                    actor.cinematic_kind = None;
                    stance_transitions.push(StanceTransition {
                        actor: actor_id.0,
                        from_stance,
                        to_stance,
                    });
                }
            }

            // (c) Lean integration.
            actor.lean_state.step(tick_rate_hz);

            // (d) Cover state recompute.
            actor.cover_state = cf_actor::CoverState {
                side: cover_side,
                effectiveness: cover_effectiveness,
                peeking: actor.lean_state.is_leaning() && cover_side != cf_actor::CoverSide::None,
            };

            // (e) Stealth-meter step.
            let visibility = cf_perception::StealthVisibility {
                instantaneous: worst_instantaneous,
                noise: if sprinting_now { 0.5 } else { 0.0 },
                crouched: actor.crouch_active,
                prone: actor.prone_active,
                stationary: actor.velocity.x.abs() < cf_actor::Stance::WALK_THRESHOLD,
            };
            let target = visibility.effective();
            let prev_meter = actor.stealth_meter;
            let mut meter = cf_perception::StealthMeter {
                value: prev_meter,
                ..cf_perception::StealthMeter::default()
            };
            let new_meter = meter.step_toward(target);
            actor.stealth_meter = new_meter;
            let band = if new_meter >= cf_perception::stealth_meter::SPOTTED_CAPTION_THRESHOLD {
                2_u8
            } else if new_meter >= cf_perception::stealth_meter::STEALTH_KILL_THRESHOLD {
                1_u8
            } else {
                0_u8
            };
            let prev_band = state.m6_last_stealth_band.get(&actor_id).copied().unwrap_or(255);
            if band != prev_band {
                state.m6_last_stealth_band.insert(actor_id, band);
                stealth_emits.push(StealthEmit {
                    actor: actor_id.0,
                    stealth_meter: new_meter,
                    spotted: band == 2,
                });
            }

            let actor = state
                .actor_state
                .as_mut()
                .and_then(|sim| sim.world.actors.get_mut(&actor_id))
                .expect("actor still present");

            // (f) Inventory-weight recompute.
            //
            // registry (`cf_equipment::mass_kg_for_id`) for the
            // per-item mass — the M6 hardcoded SLOT_WEIGHT_RIFLE_KG=8
            // placeholder is now stale data drift. Unknown ids fall
            // back to SLOT_WEIGHT_RIFLE_KG so any not-yet-registered
            // preset still contributes a sensible non-zero mass and
            // the legacy "weight > 30 kg forces walk" pipeline keeps
            // working.
            let total_weight: f32 = actor
                .inventory
                .items
                .iter()
                .map(|item| match item {
                    cf_actor::InventoryItem::Empty => SLOT_WEIGHT_EMPTY_KG,
                    cf_actor::InventoryItem::Rifle { preset } => {
                        cf_equipment::mass_kg_for_id(preset).unwrap_or(SLOT_WEIGHT_RIFLE_KG)
                    }
                })
                .sum();
            actor.inventory_weight_kg = total_weight;
            // M6B surface; ensure it's attached + the envelope is
            // refreshed every tick so liquid-drain, M6C SKU swap,
            // M27B loot pickup, etc. always see a current
            // walk-speed-multiplier without each caller needing to
            // remember to recompute.
            actor.inventory_grid_attach();
            actor.recompute_inventory_encumbrance();
            // Capture all M6B encumbrance + sprint-cancel side-effects
            // BEFORE we drop the actor borrow, so we can update
            // state.m6b_last_encumbrance_band + push the emit without
            // racing the outer state borrow.
            let m6b_env = actor.inventory_encumbrance;
            let m6b_origin_id = actor.origin_id.clone();
            // Apply sprint cancellation on Heavy band before
            // releasing the actor reference.
            let mut m6b_sprint_cancelled = false;
            if let Some(env) = m6b_env {
                if env.encumbered() && actor.sprint_active {
                    actor.sprint_active = false;
                    actor.stamina.sprinting = false;
                    m6b_sprint_cancelled = true;
                }
            }
            let _ = m6b_sprint_cancelled;
            let forces_walk = total_weight > cf_equipment::WEIGHT_FORCE_WALK_KG;
            if forces_walk && actor.sprint_active {
                actor.sprint_active = false;
                actor.stamina.sprinting = false;
                action_rejects.push(ActionReject {
                    actor: actor_id.0,
                    reason: "weight_forces_walk",
                });
            }
            // Now safe to re-borrow state for the maps + emit pushes.
            if let Some(env) = m6b_env {
                let prev_enc_band = state.m6b_last_encumbrance_band.get(&actor_id).copied();
                if Some(env.band) != prev_enc_band {
                    state.m6b_last_encumbrance_band.insert(actor_id, env.band);
                    encumbrance_emits.push(EncumbranceEmit {
                        actor: actor_id.0,
                        from_band: prev_enc_band
                            .map(cf_equipment::EncumbranceBand::as_str)
                            .unwrap_or("none"),
                        to_band: env.band.as_str(),
                        total_carried_kg: env.total_carried_kg,
                        max_carry_kg: env.max_carry_kg,
                        carry_ratio: env.carry_ratio(),
                        walk_speed_multiplier: env.walk_speed_multiplier,
                        origin_id: m6b_origin_id,
                    });
                }
            }
            let prev_bucket = state.m6_last_weight_bucket.get(&actor_id).copied();
            if prev_bucket != Some(forces_walk) {
                state.m6_last_weight_bucket.insert(actor_id, forces_walk);
                weight_emits.push(WeightEmit {
                    actor: actor_id.0,
                    total_weight_kg: total_weight,
                    forces_walk,
                });
            }
        }

        // (g) WeaponSwap tick — drain completed swaps + collect emissions.
        // the target (deferred from dispatch time), clear the
        // `weapon_swap_in_progress` flag so firing unlocks, and emit
        // `equipment.weapon_swap_completed`.
        let swap_ids: Vec<ActorId> = state.weapon_swap_state.keys().copied().collect();
        for actor_id in swap_ids {
            let completed = {
                let swap = state
                    .weapon_swap_state
                    .get_mut(&actor_id)
                    .expect("swap present by construction");
                swap.tick(tick_rate_hz)
            };
            if completed {
                let target = state
                    .weapon_swap_state
                    .get(&actor_id)
                    .map(|s| s.target_slot)
                    .unwrap_or(0);
                state.weapon_swap_state.remove(&actor_id);
                if let Some(actor) = state
                    .actor_state
                    .as_mut()
                    .and_then(|sim| sim.world.actors.get_mut(&actor_id))
                {
                    let _ = actor.inventory.try_select(cf_actor::ItemSlot(u32::from(target)));
                    actor.weapon_swap_in_progress = false;
                }
                swap_emits.push(SwapEmit {
                    actor: actor_id.0,
                    active_slot: target,
                });
            }
        }

        drop(state);

        for emit in stance_transitions {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "stance_changed",
                json!({
                    "actor": emit.actor,
                    "from_stance": emit.from_stance,
                    "to_stance": emit.to_stance,
                    "cause": "cinematic_complete",
                }),
                None,
            );
        }
        for emit in stamina_emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "stamina_changed",
                json!({
                    "actor": emit.actor,
                    "stamina": emit.stamina,
                    "sprinting": emit.sprinting,
                }),
                None,
            );
        }
        for emit in stealth_emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "perception",
                "stealth_meter_changed",
                json!({
                    "actor": emit.actor,
                    "stealth_meter": emit.stealth_meter,
                    "spotted": emit.spotted,
                }),
                None,
            );
        }
        for emit in weight_emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "inventory",
                "weight_changed",
                json!({
                    "actor": emit.actor,
                    "total_weight_kg": emit.total_weight_kg,
                    "forces_walk": emit.forces_walk,
                }),
                None,
            );
        }
        for emit in swap_emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_swap_completed",
                json!({
                    "actor": emit.actor,
                    "active_slot": emit.active_slot,
                }),
                None,
            );
        }
        for emit in encumbrance_emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "inventory",
                "encumbrance_threshold_crossed",
                json!({
                    "actor": emit.actor,
                    "from_band": emit.from_band,
                    "to_band": emit.to_band,
                    "total_carried_kg": emit.total_carried_kg,
                    "max_carry_kg": emit.max_carry_kg,
                    "carry_ratio": emit.carry_ratio,
                    "walk_speed_multiplier": emit.walk_speed_multiplier,
                    "origin_id": emit.origin_id,
                }),
                None,
            );
        }
        for emit in action_rejects {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "action_rejected",
                json!({
                    "actor": emit.actor,
                    "action": "act.player.sprint",
                    "reason": emit.reason,
                }),
                None,
            );
        }
    }
}
