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
        /// **M6B**: per-tick band transition emit captured during the
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
            // **M6B**: each slot consults the canonical ItemSpec
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
            // **M6B**: the per-actor inventory grid is the canonical
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
        // **M6**: when a swap completes, set the actor's selected slot to
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

    /// **M6**: tick the grenade + knife projectiles + facing-from-aim
    /// derivation + per-tool bipod auto-stow. Emits
    /// `equipment.grenade_detonated`, `combat.knife_throw_landed`, and
    /// `actor.facing_changed` events. Called from `drive_tick` after
    /// `tick_m6_actor_state`.
    pub(crate) fn tick_m6_equipment(&self, tick: Tick, sim_time_ms: f64) {
        const GRAVITY_PER_S2: f32 = 360.0;
        let dt_seconds = 1.0_f32 / self.config.tick_rate_hz.max(1) as f32;

        struct GrenadeDetonation {
            id: u64,
            owner: u64,
            kind: cf_equipment::GrenadeKind,
            position: cf_actor::Vec2,
            radius: f32,
            damage_at_center: f32,
            adhesive: bool,
            spawns_hazard: bool,
            vision_disrupt: bool,
        }
        struct KnifeLanding {
            id: u64,
            owner: u64,
            target_id: Option<u64>,
            stuck_target: &'static str,
            position: cf_actor::Vec2,
            damage: f32,
            hp_before: f32,
            hp_after: f32,
        }
        struct FacingFlip {
            actor: u64,
            from: cf_actor::FacingDirection,
            to: cf_actor::FacingDirection,
        }

        let mut detonations: Vec<GrenadeDetonation> = Vec::new();
        let mut landings: Vec<KnifeLanding> = Vec::new();
        let mut facing_flips: Vec<FacingFlip> = Vec::new();

        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return,
        };

        // (a) Advance grenade projectiles under gravity + collision.
        // Adhesive grenades stick on first ground contact; non-adhesive
        // grenades bounce.
        let mut remaining: Vec<GrenadeProjectile> = Vec::new();
        let terrain_solid = |pos: cf_actor::Vec2, terrain: &Option<cf_terrain::ChunkedTerrain>| -> bool {
            match terrain.as_ref() {
                Some(t) => t.registry.is_solid(t.material_at_world(pos.x, pos.y)),
                None => false,
            }
        };
        let projectiles = std::mem::take(&mut state.grenade_projectiles);
        for mut g in projectiles {
            if !g.stuck {
                g.velocity.y -= GRAVITY_PER_S2 * dt_seconds;
                let new_pos = cf_actor::Vec2::new(
                    g.position.x + g.velocity.x * dt_seconds,
                    g.position.y + g.velocity.y * dt_seconds,
                );
                let hit = terrain_solid(new_pos, &state.chunked_terrain);
                if hit {
                    if g.adhesive {
                        g.stuck = true;
                        g.velocity = cf_actor::Vec2::ZERO;
                    } else {
                        g.velocity = cf_actor::Vec2::new(g.velocity.x * 0.4, -g.velocity.y * 0.4);
                    }
                } else {
                    g.position = new_pos;
                }
            }
            g.fuse_remaining = (g.fuse_remaining - dt_seconds).max(0.0);
            if g.fuse_remaining <= 0.0 {
                detonations.push(GrenadeDetonation {
                    id: g.id,
                    owner: g.owner.0,
                    kind: g.kind,
                    position: g.position,
                    radius: g.radius,
                    damage_at_center: g.damage_at_center,
                    adhesive: g.adhesive,
                    spawns_hazard: g.spawns_hazard,
                    vision_disrupt: g.vision_disrupt,
                });
            } else {
                remaining.push(g);
            }
        }
        state.grenade_projectiles = remaining;

        // (b) Advance knife projectiles under physics + detect collision.
        let mut remaining_knives: Vec<cf_equipment::KnifeProjectile> = Vec::new();
        let knives = std::mem::take(&mut state.knife_projectiles);
        for mut k in knives {
            if k.state == cf_equipment::KnifeThrowState::InFlight {
                let new_pos = cf_actor::Vec2::new(
                    k.origin_x + k.velocity_x * dt_seconds,
                    k.origin_y + k.velocity_y * dt_seconds,
                );
                k.origin_x = new_pos.x;
                k.origin_y = new_pos.y;
                k.remaining_seconds = (k.remaining_seconds - dt_seconds).max(0.0);
                // Hit actor scan (any actor other than the thrower within 6
                // units of the knife's current position).
                let actor_hit = state.actor_state.as_ref().and_then(|sim| {
                    sim.world
                        .actors
                        .iter()
                        .filter(|(id, _)| id.0 != k.owner_actor)
                        .find(|(_, a)| {
                            let dx = a.position.x - new_pos.x;
                            let dy = a.position.y - new_pos.y;
                            (dx * dx + dy * dy).sqrt() <= 6.0
                        })
                        .map(|(id, _)| *id)
                });
                if let Some(target_id) = actor_hit {
                    let mut hp_before = 0.0;
                    let mut hp_after = 0.0;
                    if let Some(target) = state
                        .actor_state
                        .as_mut()
                        .and_then(|sim| sim.world.actors.get_mut(&target_id))
                    {
                        hp_before = target.hp;
                        let _ = target.apply_damage(k.damage);
                        hp_after = target.hp;
                    }
                    k.stick_in_actor();
                    landings.push(KnifeLanding {
                        id: k.projectile_id,
                        owner: k.owner_actor,
                        target_id: Some(target_id.0),
                        stuck_target: "actor",
                        position: new_pos,
                        damage: k.damage,
                        hp_before,
                        hp_after,
                    });
                } else if terrain_solid(new_pos, &state.chunked_terrain) {
                    k.stick_in_wall();
                    landings.push(KnifeLanding {
                        id: k.projectile_id,
                        owner: k.owner_actor,
                        target_id: None,
                        stuck_target: "wall",
                        position: new_pos,
                        damage: 0.0,
                        hp_before: 0.0,
                        hp_after: 0.0,
                    });
                } else if k.remaining_seconds <= 0.0 {
                    // Flight expired without contact — drop the projectile.
                    continue;
                }
            }
            remaining_knives.push(k);
        }
        state.knife_projectiles = remaining_knives;

        // (c) Facing-from-aim derivation per spec § "Side-view facing
        // direction": each tick, derive `FacingDirection::from_aim(aim)`
        // and emit `actor.facing_changed` on flip.
        let facing_snapshot: Vec<(ActorId, cf_actor::Vec2, cf_actor::FacingDirection)> = state
            .actor_state
            .as_ref()
            .map(|sim| sim.world.actors.iter().map(|(id, a)| (*id, a.aim, a.facing)).collect())
            .unwrap_or_default();
        for (actor_id, aim, current_facing) in facing_snapshot {
            let derived = cf_actor::FacingDirection::from_aim(aim);
            if derived != current_facing {
                if let Some(actor) = state
                    .actor_state
                    .as_mut()
                    .and_then(|sim| sim.world.actors.get_mut(&actor_id))
                {
                    actor.facing = derived;
                }
                facing_flips.push(FacingFlip {
                    actor: actor_id.0,
                    from: current_facing,
                    to: derived,
                });
                state.m6_last_facing.insert(actor_id, derived);
            }
        }

        // (d) Bipod auto-stow: when the actor exits crouch/prone, stow the
        // bipod automatically per spec § "When player stands: bipod
        // auto-stows".
        let bipod_actors: Vec<ActorId> = state
            .actor_state
            .as_ref()
            .map(|sim| sim.world.actors.keys().copied().collect())
            .unwrap_or_default();
        let mut bipod_stow_emits: Vec<u64> = Vec::new();
        for actor_id in bipod_actors {
            let needs_stow = state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&actor_id))
                .map(|a| a.bipod.state == cf_equipment::BipodState::Deployed && !(a.crouch_active || a.prone_active))
                .unwrap_or(false);
            if needs_stow {
                if let Some(actor) = state
                    .actor_state
                    .as_mut()
                    .and_then(|sim| sim.world.actors.get_mut(&actor_id))
                {
                    if actor.bipod.stow() {
                        bipod_stow_emits.push(actor_id.0);
                    }
                }
            }
        }

        drop(state);

        // (e) Emit follow-on events with the lock released so the recorder
        // can re-borrow.
        for det in detonations {
            let grenade_event_id = self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "grenade_detonated",
                json!({
                    "actor": det.owner,
                    "kind": det.kind.as_str(),
                    "position": [det.position.x, det.position.y],
                    "radius": det.radius,
                    "damage_at_center": det.damage_at_center,
                    "adhesive": det.adhesive,
                    "spawns_hazard": det.spawns_hazard,
                    "vision_disrupt": det.vision_disrupt,
                    "projectile_id": det.id,
                }),
                None,
            );
            // Type-specific effect emissions.
            match det.kind {
                cf_equipment::GrenadeKind::Frag => {
                    // **M14** § "HE round overpressure model" — emit the
                    // overpressure wave + per-actor combat.explosive_hit_mo +
                    // explosion severance + 3-organ routing.
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "armor",
                        "he_overpressure_wave",
                        json!({
                            "center": [det.position.x, det.position.y],
                            "radius": det.radius,
                            "damage_at_zero_distance": det.damage_at_center,
                            "falloff_curve": "inverse_square",
                        }),
                        Some(grenade_event_id.clone()),
                    );
                    // Radius damage: apply damage_at_center * (1 - distance/radius)
                    // to actors inside the radius.
                    if let Ok(mut s) = self.state.write() {
                        let radius = det.radius.max(1.0);
                        let center = det.position;
                        let dmg_center = det.damage_at_center;
                        let actor_ids: Vec<ActorId> = s
                            .actor_state
                            .as_ref()
                            .map(|sim| sim.world.actors.keys().copied().collect())
                            .unwrap_or_default();
                        let wave = cf_physics::HeWave {
                            center: [center.x, center.y],
                            radius,
                            damage_at_zero_distance: dmg_center,
                        };
                        // Pre-roll RNG for the 3-organ routing + explosion-
                        // severance check so the engine state lock can be
                        // dropped before recorder calls.
                        let mut rng_rolls: Vec<f32> = Vec::with_capacity(actor_ids.len() * 4);
                        for _ in 0..(actor_ids.len() * 4) {
                            rng_rolls.push((s.rng.next_u64() as f64 / u64::MAX as f64) as f32);
                        }
                        // Snapshot per-actor state so we can compute damage
                        // BEFORE applying it.
                        struct ExplosionVictim {
                            actor: ActorId,
                            dist: f32,
                            damage: f32,
                            zone: String,
                            origin_id: String,
                            team_is_robot: bool,
                            facing_sign: f32,
                            position: [f32; 2],
                        }
                        let mut victims: Vec<ExplosionVictim> = Vec::new();
                        for aid in &actor_ids {
                            let (dx, dy, origin_id, team, facing_sign, pos) = {
                                let Some(actor) = s.actor_state.as_ref().and_then(|sim| sim.world.actors.get(aid))
                                else {
                                    continue;
                                };
                                (
                                    actor.position.x - center.x,
                                    actor.position.y - center.y,
                                    actor.origin_id.clone(),
                                    actor.team.clone(),
                                    actor.facing.sign(),
                                    [actor.position.x, actor.position.y],
                                )
                            };
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist >= radius {
                                continue;
                            }
                            let dmg = cf_physics::he_damage_at_distance(&wave, dist);
                            // Use bottom-half radius heuristic for zone:
                            // close hits land on torso, mid on abdomen.
                            let zone = if dist < radius * 0.25 {
                                "torso".to_string()
                            } else {
                                "abdomen".to_string()
                            };
                            let team_is_robot =
                                team.eq_ignore_ascii_case("red_robot") || team.eq_ignore_ascii_case("robot");
                            victims.push(ExplosionVictim {
                                actor: *aid,
                                dist,
                                damage: dmg,
                                zone,
                                origin_id,
                                team_is_robot,
                                facing_sign,
                                position: pos,
                            });
                        }
                        // Apply damage.
                        for v in &victims {
                            if let Some(actor) = s
                                .actor_state
                                .as_mut()
                                .and_then(|sim| sim.world.actors.get_mut(&v.actor))
                            {
                                let _ = actor.apply_damage(v.damage);
                            }
                        }
                        drop(s);
                        // Emit per-victim chain (cause-chain rooted on the
                        // grenade_event_id).
                        let mut roll_cursor = 0usize;
                        for v in victims {
                            // combat.explosive_hit_mo
                            let explo_id = self.recorder.record(
                                tick,
                                sim_time_ms,
                                "combat",
                                "explosive_hit_mo",
                                json!({
                                    "owner_id": det.owner,
                                    "target_id": v.actor.0,
                                    "kind": "frag",
                                    "distance": v.dist,
                                    "damage": v.damage,
                                    "position": v.position,
                                    "blast_center": [center.x, center.y],
                                    "blast_radius": radius,
                                    "source_event_id": grenade_event_id.clone(),
                                }),
                                Some(grenade_event_id.clone()),
                            );
                            // Explosion proximity severance — per-zone joint
                            // impulse check across forward exposed zones.
                            let base_impulse = (v.damage * 60.0).max(0.0);
                            let exposure = cf_physics::classify_hit_direction(
                                (v.position[0] - center.x, v.position[1] - center.y),
                                v.facing_sign,
                            );
                            for zone in cf_physics::exposed_zones(exposure) {
                                let joint = cf_physics::Joint::default_for_zone(zone);
                                let eval = cf_physics::evaluate_joint(joint, base_impulse);
                                if eval.gib {
                                    let _ = self.recorder.record(
                                        tick,
                                        sim_time_ms,
                                        "attachable",
                                        "gib_threshold_crossed",
                                        json!({
                                            "actor_id": v.actor.0,
                                            "attachable_id": *zone,
                                            "parent_zone": *zone,
                                            "joint_impulse": eval.impulse_in,
                                            "gib_impulse_limit": joint.gib_impulse_limit,
                                            "source_event_id": explo_id.clone(),
                                            "cause": "explosion",
                                        }),
                                        Some(explo_id.clone()),
                                    );
                                } else if eval.detach {
                                    let _ = self.recorder.record(
                                        tick,
                                        sim_time_ms,
                                        "attachable",
                                        "detached",
                                        json!({
                                            "actor_id": v.actor.0,
                                            "attachable_id": *zone,
                                            "parent_zone": *zone,
                                            "joint_impulse": eval.impulse_in,
                                            "joint_strength": joint.joint_strength,
                                            "gib_impulse_limit": joint.gib_impulse_limit,
                                            "detach_position": v.position,
                                            "detach_velocity": [
                                                (v.position[0] - center.x).signum() * (eval.impulse_out / 80.0).clamp(0.0, 500.0),
                                                (v.position[1] - center.y).signum() * (eval.impulse_out / 80.0).clamp(0.0, 500.0),
                                            ],
                                            "damage_multiplier": joint.damage_multiplier,
                                            "source_event_id": explo_id.clone(),
                                            "cause": "explosion",
                                        }),
                                        Some(explo_id.clone()),
                                    );
                                }
                                let _ = self.recorder.record(
                                    tick,
                                    sim_time_ms,
                                    "physics",
                                    "impulse_propagated",
                                    json!({
                                        "actor_id": v.actor.0,
                                        "from_zone": *zone,
                                        "to_zone": "parent",
                                        "impulse_in": eval.impulse_in,
                                        "impulse_absorbed": eval.impulse_absorbed,
                                        "impulse_out": eval.impulse_out,
                                        "joint_strength": joint.joint_strength,
                                        "damage_multiplier": joint.damage_multiplier,
                                        "kind": "explosion",
                                        "source_event_id": explo_id.clone(),
                                    }),
                                    Some(explo_id.clone()),
                                );
                            }
                            // Per-organ 3-organ explosion routing per
                            // `cf_internal::route_explosion_internal_damage`.
                            let graph_kind = if v.team_is_robot {
                                cf_internal::InternalGraphKind::Robot
                            } else {
                                cf_internal::InternalGraphKind::Humanoid
                            };
                            let rolls = if roll_cursor + 3 <= rng_rolls.len() {
                                &rng_rolls[roll_cursor..roll_cursor + 3]
                            } else {
                                &rng_rolls[..]
                            };
                            roll_cursor = roll_cursor.saturating_add(3);
                            let decisions = cf_internal::route_explosion_internal_damage(
                                graph_kind,
                                v.zone.as_str(),
                                v.damage,
                                rolls,
                            );
                            for d in &decisions {
                                if v.team_is_robot {
                                    let _ = self.recorder.record(
                                        tick,
                                        sim_time_ms,
                                        "internal",
                                        "circuit_damaged",
                                        json!({
                                            "actor_id": v.actor.0,
                                            "circuit_id": d.target_id,
                                            "circuit_kind": cf_internal::circuit_kind(d.target_id),
                                            "from_hp": 100.0_f32,
                                            "to_hp": (100.0_f32 - d.applied_damage).max(0.0),
                                            "cause": "explosion",
                                            "source_hit_event_id": explo_id.clone(),
                                            "route_via_m14": true,
                                        }),
                                        Some(explo_id.clone()),
                                    );
                                } else {
                                    let _ = self.recorder.record(
                                        tick,
                                        sim_time_ms,
                                        "internal",
                                        "organ_damaged",
                                        json!({
                                            "actor_id": v.actor.0,
                                            "organ_id": d.target_id,
                                            "organ_kind": cf_internal::organ_kind(d.target_id),
                                            "from_hp": 100.0_f32,
                                            "to_hp": (100.0_f32 - d.applied_damage).max(0.0),
                                            "cause": "explosion",
                                            "source_hit_event_id": explo_id.clone(),
                                            "route_via_m14": true,
                                        }),
                                        Some(explo_id.clone()),
                                    );
                                }
                            }
                            let _ = v.origin_id;
                        }
                    }
                }
                cf_equipment::GrenadeKind::Smoke => {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "hazard",
                        "spawned",
                        json!({
                            "kind": "smoke",
                            "position": [det.position.x, det.position.y],
                            "radius": det.radius,
                            "owner": det.owner,
                            "source": "grenade_smoke",
                        }),
                        None,
                    );
                }
                cf_equipment::GrenadeKind::Flash => {
                    if let Ok(mut s) = self.state.write() {
                        let radius = det.radius.max(1.0);
                        let center = det.position;
                        let actor_ids: Vec<ActorId> = s
                            .actor_state
                            .as_ref()
                            .map(|sim| sim.world.actors.keys().copied().collect())
                            .unwrap_or_default();
                        for aid in actor_ids {
                            let dx = s
                                .actor_state
                                .as_ref()
                                .and_then(|sim| sim.world.actors.get(&aid))
                                .map(|a| a.position.x - center.x)
                                .unwrap_or(f32::INFINITY);
                            let dy = s
                                .actor_state
                                .as_ref()
                                .and_then(|sim| sim.world.actors.get(&aid))
                                .map(|a| a.position.y - center.y)
                                .unwrap_or(f32::INFINITY);
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist < radius {
                                if let Some(actor) =
                                    s.actor_state.as_mut().and_then(|sim| sim.world.actors.get_mut(&aid))
                                {
                                    actor.afflictions.push(cf_actor::Affliction {
                                        kind: cf_actor::AfflictionKind::InternalShock,
                                        intensity: 1.0,
                                        expires_tick: Some(tick.0 + 120),
                                    });
                                }
                            }
                        }
                    }
                }
                cf_equipment::GrenadeKind::Stick
                | cf_equipment::GrenadeKind::HighExplosive
                | cf_equipment::GrenadeKind::Acid
                | cf_equipment::GrenadeKind::PipeBomb
                | cf_equipment::GrenadeKind::Molotov
                | cf_equipment::GrenadeKind::ProximityMine
                | cf_equipment::GrenadeKind::PressureMine
                | cf_equipment::GrenadeKind::TripwireMine
                | cf_equipment::GrenadeKind::C4Charge
                | cf_equipment::GrenadeKind::Incendiary
                | cf_equipment::GrenadeKind::BouncingBetty => {
                    // Stick + M6C throwables: apply inverse-square radius
                    // damage. Material / hazard side-effects for acid /
                    // molotov / incendiary land via M15 material spawn at
                    // their owning milestone wires.
                    if let Ok(mut s) = self.state.write() {
                        let radius = det.radius.max(1.0);
                        let center = det.position;
                        let dmg_center = det.damage_at_center;
                        let actor_ids: Vec<ActorId> = s
                            .actor_state
                            .as_ref()
                            .map(|sim| sim.world.actors.keys().copied().collect())
                            .unwrap_or_default();
                        for aid in actor_ids {
                            let (dx, dy) = {
                                let Some(actor) = s.actor_state.as_ref().and_then(|sim| sim.world.actors.get(&aid))
                                else {
                                    continue;
                                };
                                (actor.position.x - center.x, actor.position.y - center.y)
                            };
                            let dist = (dx * dx + dy * dy).sqrt();
                            if dist >= radius {
                                continue;
                            }
                            let frac = (1.0 - dist / radius).clamp(0.0, 1.0);
                            let dmg = dmg_center * frac;
                            if let Some(actor) = s.actor_state.as_mut().and_then(|sim| sim.world.actors.get_mut(&aid)) {
                                let _ = actor.apply_damage(dmg);
                            }
                        }
                    }
                }
            }
        }
        for landing in landings {
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "knife_throw_landed",
                json!({
                    "actor": landing.owner,
                    "projectile_id": landing.id,
                    "stuck_target": landing.stuck_target,
                    "target_id": landing.target_id,
                    "position": [landing.position.x, landing.position.y],
                    "damage": landing.damage,
                    "hp_before": landing.hp_before,
                    "hp_after": landing.hp_after,
                }),
                None,
            );
        }
        for flip in facing_flips {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "facing_changed",
                json!({
                    "actor": flip.actor,
                    "from": flip.from.as_str(),
                    "to": flip.to.as_str(),
                    "cause": "aim_derived",
                }),
                None,
            );
        }
        for actor_id in bipod_stow_emits {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "bipod_stowed",
                json!({"actor": actor_id, "state": "stowed", "cause": "auto_stow_on_stand"}),
                None,
            );
        }

        // **M6**: tick `AdvancedFireMode::Burst3` follow-up shots. The first
        // round in a burst already fired through the M1 path; this scheduler
        // emits the remaining 2 rounds at `BURST3_INTER_SHOT_SECONDS` cadence
        // so the full 3-round burst lands within 100 ms per spec § "SMG
        // burst-3 fire mode" Gherkin.
        struct Burst3Shot {
            owner: ActorId,
            projectile_id: u64,
            shell_id: u64,
            muzzle_origin: cf_actor::Vec2,
            velocity: cf_actor::Vec2,
            damage: f32,
            loudness_radius: f32,
            facing_sign: f32,
            remaining_in_mag: u32,
            is_tracer: bool,
            spec_loudness: f32,
            suppressor_attached: bool,
            preset_id: String,
        }
        let mut burst3_shots: Vec<Burst3Shot> = Vec::new();
        let actor_ids_burst: Vec<ActorId> = self
            .state
            .read()
            .ok()
            .and_then(|s| {
                s.actor_state
                    .as_ref()
                    .map(|sim| sim.world.actors.keys().copied().collect())
            })
            .unwrap_or_default();
        if let Ok(mut s) = self.state.write() {
            for actor_id in &actor_ids_burst {
                let (remaining, mut next_at, fire_mode) =
                    match s.actor_state.as_ref().and_then(|sim| sim.world.actors.get(actor_id)) {
                        Some(actor) => (
                            actor.burst3_remaining_shots,
                            actor.burst3_next_fire_at_seconds,
                            actor.weapon_fire_mode,
                        ),
                        None => continue,
                    };
                if remaining == 0 || fire_mode != cf_equipment::AdvancedFireMode::Burst3 {
                    if remaining > 0 {
                        if let Some(actor) = s
                            .actor_state
                            .as_mut()
                            .and_then(|sim| sim.world.actors.get_mut(actor_id))
                        {
                            actor.burst3_remaining_shots = 0;
                            actor.burst3_next_fire_at_seconds = 0.0;
                        }
                    }
                    continue;
                }
                next_at -= dt_seconds;
                if next_at > 0.0 {
                    if let Some(actor) = s
                        .actor_state
                        .as_mut()
                        .and_then(|sim| sim.world.actors.get_mut(actor_id))
                    {
                        actor.burst3_next_fire_at_seconds = next_at;
                    }
                    continue;
                }
                let rifle_snapshot = s.actor_state.as_ref().and_then(|sim| sim.rifles.get(actor_id)).cloned();
                let Some(rifle) = rifle_snapshot else {
                    continue;
                };
                let spec = rifle.spec.clone();
                let (position, aim, suppressor_factor, suppressor_attached) = {
                    let Some(actor) = s.actor_state.as_ref().and_then(|sim| sim.world.actors.get(actor_id)) else {
                        continue;
                    };
                    let aim = if actor.aim == cf_actor::Vec2::ZERO {
                        cf_actor::Vec2::new(1.0, 0.0)
                    } else {
                        actor.aim.normalize_or_x()
                    };
                    (
                        actor.position,
                        aim,
                        actor.suppressor.loudness_factor(),
                        actor.suppressor.attached && actor.suppressor.integrity > 0.0,
                    )
                };
                let muzzle = cf_actor::Vec2::new(
                    position.x + aim.x * spec.muzzle_forward_offset,
                    position.y + spec.muzzle_vertical_offset + aim.y * spec.muzzle_forward_offset,
                );
                let velocity = cf_actor::Vec2::new(aim.x * spec.projectile_speed, aim.y * spec.projectile_speed);
                let loudness_radius = 480.0_f32
                    * (spec.damage_per_hit / 10.0).clamp(1.0, 3.0)
                    * spec.loudness.max(0.1)
                    * suppressor_factor;
                let max_flight = rifle.projectile_max_flight_ticks();
                let projectile_id = s.next_guard_projectile_id;
                s.next_guard_projectile_id = s.next_guard_projectile_id.saturating_add(1);
                let shell_id = s.next_guard_projectile_id;
                s.next_guard_projectile_id = s.next_guard_projectile_id.saturating_add(1);
                let facing_sign = if aim.x >= 0.0 { 1.0_f32 } else { -1.0_f32 };
                if let Some(sim) = s.actor_state.as_mut() {
                    sim.projectiles.push(cf_actor::sim::Projectile {
                        id: projectile_id,
                        owner: *actor_id,
                        origin: muzzle,
                        position: muzzle,
                        velocity,
                        damage: spec.damage_per_hit,
                        remaining_ticks: max_flight,
                        mass_kg: spec.bullet_mass_kg,
                        sharpness: spec.bullet_sharpness,
                    });
                }
                let mag_remaining = s
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.rifles.get(actor_id))
                    .map_or(0_u32, |r| r.ammo_in_mag);
                let is_tracer = spec.tracer_round_to_total_ratio > 0
                    && (rifle.shot_index_in_mag % spec.tracer_round_to_total_ratio.max(1))
                        == spec.tracer_round_to_total_ratio.max(1) - 1;
                burst3_shots.push(Burst3Shot {
                    owner: *actor_id,
                    projectile_id,
                    shell_id,
                    muzzle_origin: muzzle,
                    velocity,
                    damage: spec.damage_per_hit,
                    loudness_radius,
                    facing_sign,
                    remaining_in_mag: mag_remaining,
                    is_tracer,
                    spec_loudness: spec.loudness,
                    suppressor_attached,
                    preset_id: spec.preset_id.clone(),
                });
                if let Some(actor) = s
                    .actor_state
                    .as_mut()
                    .and_then(|sim| sim.world.actors.get_mut(actor_id))
                {
                    actor.burst3_remaining_shots = actor.burst3_remaining_shots.saturating_sub(1);
                    actor.burst3_next_fire_at_seconds = if actor.burst3_remaining_shots > 0 {
                        cf_equipment::BURST3_INTER_SHOT_SECONDS
                    } else {
                        0.0
                    };
                }
                let _ = facing_sign;
            }
        }

        for shot in burst3_shots {
            let weapon_fired_id = self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "weapon_fired",
                json!({
                    "actor": shot.owner.0,
                    "preset_id": shot.preset_id,
                    "muzzle_origin": [shot.muzzle_origin.x, shot.muzzle_origin.y],
                    "bipod_deployed": false,
                    "suppressor_attached": shot.suppressor_attached,
                    "burst3_followup": true,
                }),
                None,
            );
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "projectile_spawned",
                json!({
                    "id": shot.projectile_id,
                    "owner": shot.owner.0,
                    "origin": [shot.muzzle_origin.x, shot.muzzle_origin.y],
                    "velocity": [shot.velocity.x, shot.velocity.y],
                    "damage": shot.damage,
                    "is_tracer": shot.is_tracer,
                    "particle_index": 0,
                    "particle_count": 1,
                    "burst3_followup": true,
                }),
                Some(weapon_fired_id.clone()),
            );
            let shell = cf_equipment::ShellEjection::default_for(
                cf_equipment::ShellKind::Rifle,
                shot.shell_id,
                shot.muzzle_origin.x,
                shot.muzzle_origin.y + 4.0,
                shot.facing_sign,
            );
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "shell_ejected",
                json!({
                    "actor": shot.owner.0,
                    "shell_id": shell.shell_id,
                    "kind": shell.kind.as_str(),
                    "origin": [shell.origin_x, shell.origin_y],
                    "velocity": [shell.velocity_x, shell.velocity_y],
                    "lifetime_seconds": shell.lifetime_seconds,
                    "cosmetic": true,
                }),
                Some(weapon_fired_id.clone()),
            );
            if shot.loudness_radius > 0.0 {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "alarm_registered",
                    json!({
                        "actor": shot.owner.0,
                        "source_id": shot.preset_id,
                        "pos": [shot.muzzle_origin.x, shot.muzzle_origin.y],
                        "muzzle_origin": [shot.muzzle_origin.x, shot.muzzle_origin.y],
                        "loudness_radius": shot.loudness_radius,
                        "loudness": shot.loudness_radius,
                        "suppressed": shot.suppressor_attached,
                        "cause": "weapon_fired",
                    }),
                    Some(weapon_fired_id),
                );
            }
            let _ = shot.remaining_in_mag;
            let _ = shot.spec_loudness;
        }
    }

    /// **M6**: per-tick perception emissions. Drives the new
    /// `perception.footstep_emitted` / `perception.occlusion_applied` event
    /// families from the unified cf-perception kernel. Co-exists with the
    /// legacy M2 `ai.perception_signal` event (emitted from
    /// `emit_guard_events`) so M2 replay consumers continue working.
    ///
    /// - **Footsteps**: emitted on a cadence (every `FOOTSTEP_PERIOD_TICKS`
    ///   ticks at 60 Hz) for any actor whose horizontal speed is above the
    ///   walk threshold. The surface kind is derived from the terrain
    ///   material at the actor's feet.
    /// - **Occlusion**: emitted once per (observer, target) pair where the
    ///   observer is an AI guard and the line from observer to target
    ///   crosses at least one solid terrain pixel. The factor is the
    ///   product of per-sample attenuations along the ray.
    pub(crate) fn tick_m6_perception(&self, tick: Tick, sim_time_ms: f64) {
        const FOOTSTEP_PERIOD_TICKS: u32 = 20;
        const OCCLUSION_RAY_STEPS: u32 = 16;

        struct FootstepEmit {
            actor: u64,
            surface: &'static str,
            loudness: f32,
            band: &'static str,
            /// **M12B** § Source world position. Used by the spatial
            /// resolve pass to emit `audio.spatial_resolved` etc. for
            /// each footstep cue.
            position: [f32; 2],
            /// **M12B** § Source velocity (m/s).
            velocity: [f32; 2],
        }
        struct OcclusionEmit {
            actor: u64,
            receiver: u64,
            factor: f32,
        }
        let mut footsteps: Vec<FootstepEmit> = Vec::new();
        let mut occlusions: Vec<OcclusionEmit> = Vec::new();

        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return,
        };

        // Footstep emission — actors moving horizontally on a surface.
        // **M12B**: snapshot full velocity (not just speed) so the
        // spatial-resolve pass can emit a moving-source doppler factor.
        let actor_movement: Vec<(ActorId, cf_actor::Vec2, cf_actor::Vec2, f32, bool, bool)> = state
            .actor_state
            .as_ref()
            .map(|sim| {
                sim.world
                    .actors
                    .iter()
                    .map(|(id, a)| {
                        (
                            *id,
                            cf_actor::Vec2::new(a.position.x, a.position.y + a.half_extents.y),
                            a.velocity,
                            a.velocity.x.abs(),
                            a.sprint_active,
                            a.on_ground,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        for (actor_id, feet_pos, velocity, speed, sprinting, on_ground) in actor_movement {
            if !on_ground || speed < cf_actor::Stance::WALK_THRESHOLD {
                state.m6_footstep_cooldown.insert(actor_id, 0);
                continue;
            }
            let cd = state.m6_footstep_cooldown.entry(actor_id).or_insert(0);
            *cd = cd.saturating_add(1);
            if *cd < FOOTSTEP_PERIOD_TICKS {
                continue;
            }
            *cd = 0;

            let surface_kind = match state.chunked_terrain.as_ref() {
                Some(terrain) => {
                    let mat = terrain.material_at_world(feet_pos.x, feet_pos.y + 1.0);
                    match cf_terrain::material_name_from_id(mat) {
                        "dirt" => cf_perception::SurfaceKind::Dirt,
                        "concrete" | "concrete_soft" => cf_perception::SurfaceKind::Concrete,
                        "metal_nohook" => cf_perception::SurfaceKind::Metal,
                        "loose_fill" => cf_perception::SurfaceKind::LooseFill,
                        _ => cf_perception::SurfaceKind::Dirt,
                    }
                }
                None => cf_perception::SurfaceKind::Dirt,
            };
            let stance_loudness = if sprinting { 0.9 } else { 0.5 };
            let emission = cf_perception::FootstepEmission {
                actor: actor_id.0,
                position: feet_pos,
                surface: surface_kind,
                stance_loudness,
            };
            let loudness = cf_perception::footstep_loudness(emission);
            let band = cf_perception::LoudnessBand::from_intensity(loudness).as_str();
            footsteps.push(FootstepEmit {
                actor: actor_id.0,
                surface: surface_kind.as_str(),
                loudness,
                band,
                position: [feet_pos.x, feet_pos.y],
                velocity: [velocity.x, velocity.y],
            });
        }

        // Occlusion emission — observer-target pairs (AI guard → player).
        let player_pos = state.player_actor.and_then(|pid| {
            state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&pid))
                .map(|a| (pid, a.position))
        });
        let guard_positions: Vec<(ActorId, cf_actor::Vec2)> = state
            .reactive_guards
            .keys()
            .filter_map(|gid| {
                state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(gid))
                    .map(|g| (*gid, g.position))
            })
            .collect();
        if let (Some((player_id, player_position)), Some(terrain)) = (player_pos, state.chunked_terrain.as_ref()) {
            for (_gid, observer_pos) in &guard_positions {
                let dx = player_position.x - observer_pos.x;
                let dy = player_position.y - observer_pos.y;
                let steps = OCCLUSION_RAY_STEPS as f32;
                let mut result = cf_perception::OcclusionResult::passthrough();
                for i in 1..OCCLUSION_RAY_STEPS {
                    let t = i as f32 / steps;
                    let sx = observer_pos.x + dx * t;
                    let sy = observer_pos.y + dy * t;
                    let mat = terrain.material_at_world(sx, sy);
                    let occluder = match cf_terrain::material_name_from_id(mat) {
                        "concrete" | "concrete_soft" => cf_perception::occlusion::OcclusionMaterial::Concrete,
                        "metal_nohook" => cf_perception::occlusion::OcclusionMaterial::Metal,
                        "loose_fill" => cf_perception::occlusion::OcclusionMaterial::LooseFill,
                        "dirt" => cf_perception::occlusion::OcclusionMaterial::Concrete,
                        _ => continue,
                    };
                    if terrain.registry.is_solid(mat) {
                        result = cf_perception::apply_occlusion(result, occluder);
                    }
                }
                if result.factor < 1.0 {
                    occlusions.push(OcclusionEmit {
                        actor: player_id.0,
                        receiver: player_id.0,
                        factor: result.factor,
                    });
                }
            }
        }

        drop(state);

        for emit in footsteps {
            self.recorder.record(
                tick,
                sim_time_ms,
                "perception",
                "footstep_emitted",
                json!({
                    "actor": emit.actor,
                    "surface": emit.surface,
                    "loudness": emit.loudness,
                    "band": emit.band,
                }),
                None,
            );
            // **M12B** § Per-footstep spatial-resolve emission. Spec
            // acceptance "Player locates an unseen footstep by ear within
            // 15 degrees": `audio.spatial_resolved` fires with azimuth +
            // distance + hrir_index when the footstep SFX fires.
            let cue_name = format!("footstep.{}", emit.actor);
            self.emit_m12b_spatial_resolve(
                tick,
                sim_time_ms,
                &cue_name,
                emit.position,
                emit.velocity,
                cf_audio::Medium::Air,
                &[],
                cf_audio::ReverbProfile::open_outdoor(),
                None,
            );
        }
        for emit in occlusions {
            self.recorder.record(
                tick,
                sim_time_ms,
                "perception",
                "occlusion_applied",
                json!({
                    "actor": emit.actor,
                    "receiver": emit.receiver,
                    "factor": emit.factor,
                    "occlusion_factor": emit.factor,
                }),
                None,
            );
        }
    }

    /// **M6**: tick the squad of followers — each follower consults its
    /// `current_command` and acts accordingly. M6 implements two of the
    /// four kinds end-to-end (FollowLeader + HoldPosition); DefendPoint and
    /// PushToWaypoint move toward a waypoint when one is set. Full AI
    /// archetypes (cover seeking, suppression, retreat, engage en-route)
    /// land in M7 — at M6 we only need the action surface to be reachable
    /// so `squad.command_issued` has a real consumer.
    pub(crate) fn tick_m6_squad(&self, tick: Tick, sim_time_ms: f64) {
        use cf_squad::SquadCommandKind;
        const FOLLOWER_SPEED_PX_S: f32 = 90.0;
        const FOLLOWER_STOP_RADIUS: f32 = 24.0;
        let dt = 1.0_f32 / self.config.tick_rate_hz.max(1) as f32;
        let mut state = match self.state.write() {
            Ok(s) => s,
            Err(_) => return,
        };
        if state.squad.followers.is_empty() {
            return;
        }
        let leader_id = state.squad.leader.as_ref().map(|m| m.actor);
        let leader_pos = leader_id.and_then(|lid| {
            state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&lid))
                .map(|a| a.position)
        });
        let follower_targets: Vec<(cf_actor::ActorId, SquadCommandKind, Option<cf_actor::Vec2>)> = state
            .squad
            .followers
            .iter()
            .map(|m| (m.actor, m.current_command.kind, m.current_command.waypoint))
            .collect();
        let mut command_events: Vec<(u64, &'static str)> = Vec::new();
        for (actor_id, kind, waypoint) in follower_targets {
            let target = match kind {
                SquadCommandKind::FollowLeader => leader_pos,
                SquadCommandKind::HoldPosition => None,
                SquadCommandKind::DefendPoint | SquadCommandKind::PushToWaypoint => waypoint,
            };
            let stop_radius = match kind {
                SquadCommandKind::FollowLeader => FOLLOWER_STOP_RADIUS,
                _ => 8.0,
            };
            let new_vel = if let Some(target_pos) = target {
                if let Some(actor) = state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(&actor_id))
                {
                    let dx = target_pos.x - actor.position.x;
                    let dy = target_pos.y - actor.position.y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > stop_radius {
                        cf_actor::Vec2::new((dx / dist) * FOLLOWER_SPEED_PX_S, actor.velocity.y)
                    } else {
                        cf_actor::Vec2::new(0.0, actor.velocity.y)
                    }
                } else {
                    continue;
                }
            } else {
                let vy = state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(&actor_id))
                    .map(|a| a.velocity.y)
                    .unwrap_or(0.0);
                cf_actor::Vec2::new(0.0, vy)
            };
            if let Some(actor) = state
                .actor_state
                .as_mut()
                .and_then(|sim| sim.world.actors.get_mut(&actor_id))
            {
                actor.velocity = new_vel;
                actor.position = cf_actor::Vec2::new(actor.position.x + new_vel.x * dt, actor.position.y);
                command_events.push((actor_id.0, kind.as_str()));
            }
        }
        drop(state);
        let _ = (sim_time_ms, tick, command_events);
    }

}
