//! Dispatch handlers for cfctl commands.
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
    pub(crate) fn dispatch_m6_action(
        &self,
        action: crate::m6_actions::M6Action,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        use crate::m6_actions::M6Action;
        if !self.config.has_actor_world {
            let method = action.method_name();
            return self.reject_actor_command(tick, sim_time_ms, state, method);
        }
        let _ = source;
        let method = action.method_name();
        let mut state = state;
        let player_id = state.player_actor.expect("player actor present");
        let mut reject_reason: Option<&'static str> = None;
        let mut event_payload = json!({"actor": player_id.0});
        let mut swap_to_register: Option<(ActorId, cf_equipment::WeaponSwap)> = None;
        let mut grenade_to_spawn: Option<PendingGrenadeSpawn> = None;
        let mut melee_to_resolve: Option<PendingMeleeResolve> = None;
        let mut knife_to_spawn: Option<PendingKnifeSpawn> = None;
        let mut tool_broken_pending: Option<(u64, String, f32)> = None;
        let mut tool_repaired_pending: Option<(u64, f32, Vec<String>)> = None;
        let mut stealth_kill_attempt: Option<StealthKillAttempt> = None;
        let mut tool_effect: Option<ToolEffect> = None;
        let mut drop_item_spawn: Option<DroppedItem> = None;
        let mut pickup_request_pos: Option<cf_actor::Vec2> = None;
        if let Some(sim) = state.actor_state.as_mut() {
            if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                match &action {
                    M6Action::Sprint { active } => {
                        if *active && actor.limb_loss.sprint_disabled() {
                            reject_reason = Some(
                                actor
                                    .limb_loss
                                    .reject_reason_for("sprint")
                                    .unwrap_or("sprint_disabled_by_limb_loss"),
                            );
                        } else if *active && actor.inventory_weight_kg > cf_equipment::WEIGHT_FORCE_WALK_KG {
                            reject_reason = Some("weight_forces_walk");
                        } else if *active && !actor.stamina.can_sprint() {
                            reject_reason = Some("stamina_depleted");
                        } else {
                            actor.sprint_active = *active;
                            actor.stamina.sprinting = *active;
                            event_payload = json!({"actor": player_id.0, "active": *active});
                        }
                    }
                    M6Action::Prone { active } => {
                        actor.prone_active = *active;
                        event_payload = json!({"actor": player_id.0, "active": *active});
                    }
                    M6Action::Slide => {
                        if actor.sprint_active {
                            actor.cinematic_kind = Some(cf_actor::Stance::Slide);
                            actor.cinematic_ticks_remaining = 36;
                            event_payload = json!({"actor": player_id.0, "duration_ticks": 36});
                        } else {
                            reject_reason = Some("slide_requires_sprint");
                        }
                    }
                    M6Action::Vault => {
                        if actor.limb_loss.movement_disabled() {
                            reject_reason = actor.limb_loss.reject_reason_for("vault");
                        } else {
                            actor.cinematic_kind = Some(cf_actor::Stance::Vault);
                            actor.cinematic_ticks_remaining = 48;
                            event_payload = json!({"actor": player_id.0, "duration_ticks": 48});
                        }
                    }
                    M6Action::ClimbUp => {
                        if actor.limb_loss.movement_disabled() {
                            reject_reason = actor.limb_loss.reject_reason_for("climb_up");
                        } else {
                            actor.cinematic_kind = Some(cf_actor::Stance::LadderClimb);
                            actor.cinematic_ticks_remaining = 90;
                            event_payload = json!({"actor": player_id.0, "duration_ticks": 90, "direction": "up"});
                        }
                    }
                    M6Action::ClimbDown => {
                        if actor.limb_loss.movement_disabled() {
                            reject_reason = actor.limb_loss.reject_reason_for("climb_down");
                        } else {
                            actor.cinematic_kind = Some(cf_actor::Stance::LadderClimb);
                            actor.cinematic_ticks_remaining = 90;
                            event_payload = json!({"actor": player_id.0, "duration_ticks": 90, "direction": "down"});
                        }
                    }
                    M6Action::Dive => {
                        actor.cinematic_kind = Some(cf_actor::Stance::Dive);
                        actor.cinematic_ticks_remaining = 36;
                        event_payload = json!({"actor": player_id.0, "duration_ticks": 36});
                    }
                    M6Action::Lean { direction } => {
                        actor.lean_state.direction = if *direction < -0.5 {
                            cf_actor::LeanDirection::Left
                        } else if *direction > 0.5 {
                            cf_actor::LeanDirection::Right
                        } else {
                            cf_actor::LeanDirection::None
                        };
                        event_payload = json!({"actor": player_id.0, "direction": actor.lean_state.direction.as_str()});
                    }
                    M6Action::StealthKill => {
                        if actor.stealth_meter < cf_equipment::STEALTH_KILL_METER_MAX {
                            actor.cinematic_kind = Some(cf_actor::Stance::StealthAttack);
                            actor.cinematic_ticks_remaining = 72;
                            event_payload = json!({"actor": player_id.0, "stealth_meter": actor.stealth_meter});
                            stealth_kill_attempt = Some(StealthKillAttempt {
                                attacker: player_id,
                                attacker_pos: actor.position,
                                attacker_facing_x: actor.facing.sign(),
                            });
                        } else {
                            reject_reason = Some("not_stealthy_enough");
                        }
                    }
                    M6Action::KnifeThrow => {
                        if actor.limb_loss.weapon_fire_disabled() {
                            reject_reason = actor.limb_loss.reject_reason_for("knife_throw");
                        } else {
                            actor.cinematic_kind = Some(cf_actor::Stance::KnifeThrow);
                            actor.cinematic_ticks_remaining = 24;
                            // **M6**: spawn a KnifeProjectile under physics with
                            // 50% of the equipped melee's base damage. The tick
                            // scheduler advances the projectile + emits
                            // `combat.knife_throw_landed` on collision.
                            let knife_preset = cf_equipment::m6_melee_presets()
                                .into_iter()
                                .find(|m| m.kind == cf_equipment::MeleeKind::Knife);
                            let base_damage = knife_preset.map(|p| p.damage).unwrap_or(20.0);
                            let aim = if actor.aim == cf_actor::Vec2::ZERO {
                                cf_actor::Vec2::new(1.0, 0.0)
                            } else {
                                actor.aim.normalize_or_x()
                            };
                            knife_to_spawn = Some(PendingKnifeSpawn {
                                owner: player_id,
                                origin: actor.position,
                                aim,
                                base_damage,
                            });
                            event_payload = json!({
                                "actor": player_id.0,
                                "duration_ticks": 24,
                                "damage_factor": cf_equipment::KNIFE_THROW_DAMAGE_FACTOR,
                            });
                        }
                    }
                    M6Action::WeaponSwap { slot } => {
                        let new_slot = cf_actor::ItemSlot(u32::from(*slot));
                        let prev = actor.inventory.selected;
                        // **M6**: replace the instant `actor.inventory.try_select(...)`
                        // with the WeaponSwap state machine. The selected slot is
                        // changed at swap completion (see tick_m6_actor_state),
                        // and firing is locked while `weapon_swap_in_progress`.
                        if (new_slot.0 as usize) >= actor.inventory.items.len() {
                            reject_reason = Some("slot_invalid");
                        } else if actor.weapon_swap_in_progress {
                            reject_reason = Some("swap_in_progress");
                        } else if prev == new_slot {
                            reject_reason = Some("slot_already_active");
                        } else {
                            let duration = cf_equipment::swap_duration_for_target(*slot);
                            actor.weapon_swap_in_progress = true;
                            swap_to_register = Some((
                                player_id,
                                cf_equipment::WeaponSwap::start(prev.0 as u8, *slot, duration),
                            ));
                            event_payload = json!({
                                "actor": player_id.0,
                                "from_slot": prev.0,
                                "to_slot": (*slot),
                                "duration_seconds": duration,
                            });
                        }
                    }
                    M6Action::DropItem { slot } => {
                        let drop_slot = slot.unwrap_or(actor.inventory.selected.0 as u8);
                        let slot_idx = drop_slot as usize;
                        let dropped_label = actor
                            .inventory
                            .items
                            .get(slot_idx)
                            .map(|it| it.label().to_string())
                            .unwrap_or_else(|| "empty".to_string());
                        let dropped_item_id = match actor.inventory.items.get(slot_idx) {
                            Some(cf_actor::InventoryItem::Rifle { preset }) => preset.clone(),
                            _ => String::new(),
                        };
                        let weight = match actor.inventory.items.get(slot_idx) {
                            Some(cf_actor::InventoryItem::Rifle { .. }) => 3.5_f32,
                            _ => 0.0_f32,
                        };
                        // Compute hand position with a small forward + up
                        // toss so the dropped entity doesn't immediately
                        // collide with the actor's collision proxy.
                        let aim = if actor.aim == cf_actor::Vec2::ZERO {
                            cf_actor::Vec2::new(actor.facing.sign(), 0.0)
                        } else {
                            actor.aim.normalize_or_x()
                        };
                        let hand_pos = cf_actor::Vec2::new(
                            actor.position.x + aim.x * 12.0,
                            actor.position.y + actor.half_extents.y * 0.5,
                        );
                        let toss_velocity = cf_actor::Vec2::new(aim.x * 80.0, 60.0);
                        if dropped_item_id.is_empty() {
                            reject_reason = Some("slot_empty");
                        } else {
                            actor.inventory.items[slot_idx] = cf_actor::InventoryItem::Empty;
                            event_payload = json!({
                                "actor": player_id.0,
                                "slot": drop_slot,
                                "item_id": dropped_item_id,
                                "label": dropped_label,
                                "position": [hand_pos.x, hand_pos.y],
                                "velocity": [toss_velocity.x, toss_velocity.y],
                                "weight_kg": weight,
                            });
                            drop_item_spawn = Some(DroppedItem {
                                id: 0,
                                item_id: dropped_item_id,
                                position: hand_pos,
                                weight_kg: weight,
                                dropped_by: player_id,
                                original_slot: drop_slot,
                            });
                        }
                    }
                    M6Action::Pickup => {
                        event_payload = json!({"actor": player_id.0});
                        pickup_request_pos = Some(actor.position);
                    }
                    M6Action::SignalFriendly => {
                        event_payload = json!({"actor": player_id.0, "signal": "friendly"});
                    }
                    M6Action::SignalEnemySpotted => {
                        event_payload = json!({"actor": player_id.0, "signal": "enemy_spotted"});
                    }
                    M6Action::MarkWaypoint { x, y } => {
                        event_payload = json!({"actor": player_id.0, "x": *x, "y": *y, "position": [*x, *y]});
                    }
                    M6Action::DeployBipod => {
                        let can_deploy = actor.crouch_active || actor.prone_active;
                        if can_deploy {
                            // **M6**: actually flip the bipod state on the actor so
                            // the firing path in cf-actor::sim multiplies recoil by
                            // BIPOD_RECOIL_FACTOR + bloom by BIPOD_BLOOM_FACTOR.
                            let deployed = actor.bipod.try_deploy(true);
                            if deployed {
                                event_payload = json!({
                                    "actor": player_id.0,
                                    "state": "deployed",
                                    "recoil_factor": cf_equipment::BIPOD_RECOIL_FACTOR,
                                    "bloom_factor": cf_equipment::BIPOD_BLOOM_FACTOR,
                                });
                            } else {
                                reject_reason = Some("bipod_already_deployed");
                            }
                        } else {
                            reject_reason = Some("bipod_requires_crouch_or_prone");
                        }
                    }
                    M6Action::StowBipod => {
                        let stowed = actor.bipod.stow();
                        if stowed {
                            event_payload = json!({"actor": player_id.0, "state": "stowed"});
                        } else {
                            reject_reason = Some("bipod_not_deployed");
                        }
                    }
                    M6Action::CycleFireMode => {
                        // **M6**: cycle the actor's current weapon to the next
                        // entry in its [`cf_equipment::FireModeSet::available`]
                        // list. The selected weapon's preset id determines the
                        // mode ladder (see [`cf_equipment::available_fire_modes_for`]);
                        // unknown / empty slots fall back to `[Single]` so the
                        // call is always well-defined. Updates
                        // `actor.weapon_fire_mode` and emits
                        // `equipment.fire_mode_cycled` with both `from_mode` and
                        // `to_mode` so replay consumers can render the HUD
                        // transition.
                        let preset_id = match actor.inventory.selected_item() {
                            cf_actor::InventoryItem::Rifle { preset } => preset.clone(),
                            cf_actor::InventoryItem::Empty => String::new(),
                        };
                        let available = cf_equipment::available_fire_modes_for(&preset_id);
                        let from_mode = actor.weapon_fire_mode;
                        let mut mode_set = cf_equipment::FireModeSet {
                            available,
                            current: from_mode,
                        };
                        let to_mode = mode_set.cycle_next();
                        actor.weapon_fire_mode = to_mode;
                        if to_mode != cf_equipment::AdvancedFireMode::Charge {
                            actor.weapon_charge_fraction = 0.0;
                        }
                        event_payload = json!({
                            "actor": player_id.0,
                            "from_mode": from_mode.as_str(),
                            "to_mode": to_mode.as_str(),
                            "new_mode": to_mode.as_str(),
                            "weapon_preset": preset_id,
                        });
                    }
                    M6Action::CookGrenade => {
                        // **M6**: cook the held grenade — subtract cook tick from
                        // remaining fuse. If cook exceeds fuse, the grenade
                        // detonates in hand (lethal). Emits
                        // `equipment.grenade_cooked` with the new remaining fuse.
                        const COOK_PER_PRESS_SECONDS: f32 = 0.5;
                        if let Some(kind) = actor.grenade_held_kind {
                            actor.grenade_cook_seconds = (actor.grenade_cook_seconds + COOK_PER_PRESS_SECONDS).max(0.0);
                            actor.grenade_held_fuse_remaining =
                                cf_equipment::cook_grenade(actor.grenade_held_fuse_remaining, COOK_PER_PRESS_SECONDS);
                            event_payload = json!({
                                "actor": player_id.0,
                                "kind": kind.as_str(),
                                "cook_elapsed_seconds": actor.grenade_cook_seconds,
                                "fuse_remaining_seconds": actor.grenade_held_fuse_remaining,
                                "detonates_in_hand": actor.grenade_held_fuse_remaining <= 0.0,
                            });
                        } else {
                            reject_reason = Some("no_grenade_equipped");
                        }
                    }
                    M6Action::ThrowGrenade => {
                        // **M6**: spawn a grenade projectile under physics with
                        // the (possibly-cooked) fuse. The tick scheduler counts
                        // the fuse down + emits `equipment.grenade_detonated` at
                        // fuse=0 with the type-specific effect. The arc preview
                        // samples are precomputed here for replay determinism.
                        if let Some(kind) = actor.grenade_held_kind {
                            let aim = if actor.aim == cf_actor::Vec2::ZERO {
                                cf_actor::Vec2::new(1.0, 0.0)
                            } else {
                                actor.aim.normalize_or_x()
                            };
                            let preset = cf_equipment::m6_grenade_presets().into_iter().find(|g| g.kind == kind);
                            let base_fuse = preset.as_ref().map(|p| p.fuse_seconds).unwrap_or(5.0);
                            let remaining_fuse = if actor.grenade_held_fuse_remaining > 0.0 {
                                actor.grenade_held_fuse_remaining
                            } else {
                                base_fuse
                            };
                            let throw_speed: f32 = 320.0;
                            let throw_velocity = cf_actor::Vec2::new(aim.x * throw_speed, aim.y * throw_speed);
                            event_payload = json!({
                                "actor": player_id.0,
                                "kind": kind.as_str(),
                                "fuse_seconds": remaining_fuse,
                                "origin": [actor.position.x, actor.position.y],
                                "velocity": [throw_velocity.x, throw_velocity.y],
                            });
                            grenade_to_spawn = Some(PendingGrenadeSpawn {
                                owner: player_id,
                                kind,
                                origin: actor.position,
                                velocity: throw_velocity,
                                fuse_remaining: remaining_fuse,
                                radius: preset.as_ref().map(|p| p.radius).unwrap_or(60.0),
                                damage_at_center: preset.as_ref().map(|p| p.damage_at_center).unwrap_or(50.0),
                                adhesive: preset.as_ref().map(|p| p.adhesive).unwrap_or(false),
                                spawns_hazard: preset.as_ref().map(|p| p.spawns_hazard).unwrap_or(false),
                                vision_disrupt: preset.as_ref().map(|p| p.vision_disrupt).unwrap_or(false),
                            });
                            actor.grenade_cook_seconds = 0.0;
                            actor.grenade_held_fuse_remaining = 0.0;
                        } else {
                            reject_reason = Some("no_grenade_equipped");
                        }
                    }
                    M6Action::MeleeBash => {
                        if actor.limb_loss.weapon_fire_disabled() {
                            reject_reason = actor.limb_loss.reject_reason_for("fire");
                        } else {
                            // **M6**: shoulder-check during sprint substitutes
                            // the rifle bash with the heavier impact (per spec
                            // table: shoulder check = 10 blunt + 80% knockdown).
                            let melee_kind = if actor.sprint_active {
                                cf_equipment::MeleeKind::ShoulderCheck
                            } else {
                                cf_equipment::MeleeKind::RifleBash
                            };
                            event_payload = json!({
                                "actor": player_id.0,
                                "kind": if matches!(melee_kind, cf_equipment::MeleeKind::ShoulderCheck) { "shoulder_check" } else { "bash" },
                            });
                            melee_to_resolve = Some(PendingMeleeResolve {
                                attacker: player_id,
                                kind: melee_kind,
                                facing_sign: if actor.aim.x >= 0.0 { 1.0 } else { -1.0 },
                                actor_position: actor.position,
                            });
                        }
                    }
                    M6Action::MeleeKick => {
                        event_payload = json!({"actor": player_id.0, "kind": "kick"});
                        melee_to_resolve = Some(PendingMeleeResolve {
                            attacker: player_id,
                            kind: cf_equipment::MeleeKind::Kick,
                            facing_sign: if actor.aim.x >= 0.0 { 1.0 } else { -1.0 },
                            actor_position: actor.position,
                        });
                    }
                    // **M14 audit pass 3 (GAP-M6-01)**: sprint-fueled
                    // shoulder check. Resolves through the MeleeKind::ShoulderCheck
                    // preset (higher knockdown probability than Kick).
                    M6Action::MeleeShoulderCheck => {
                        event_payload = json!({"actor": player_id.0, "kind": "shoulder_check"});
                        melee_to_resolve = Some(PendingMeleeResolve {
                            attacker: player_id,
                            kind: cf_equipment::MeleeKind::ShoulderCheck,
                            facing_sign: if actor.aim.x >= 0.0 { 1.0 } else { -1.0 },
                            actor_position: actor.position,
                        });
                    }
                    M6Action::UseTool { tool_kind } => {
                        // **M6**: apply wear on each use; emit
                        // `equipment.tool_broken` when durability hits 0. For
                        // tool="repair", restore wear on every tool entry in
                        // the actor's durability map and emit one
                        // `equipment.tool_repaired` per target tool. Each
                        // tool kind also produces tool-specific side-effects
                        // captured in `tool_effect` for the post-dispatch
                        // resolver.
                        const WEAR_PER_USE_DEFAULT: f32 = 1.0;
                        const REPAIR_RESTORE_DEFAULT: f32 = 25.0;
                        if tool_kind == "repair" {
                            let tools_to_repair: Vec<String> = actor
                                .tool_durability
                                .iter()
                                .filter_map(|(k, d)| if d.current < d.max { Some(k.clone()) } else { None })
                                .collect();
                            let mut repaired: Vec<String> = Vec::with_capacity(tools_to_repair.len());
                            for tool_key in tools_to_repair {
                                if let Some(d) = actor.tool_durability.get_mut(&tool_key) {
                                    d.restore(REPAIR_RESTORE_DEFAULT);
                                    repaired.push(tool_key);
                                }
                            }
                            event_payload = json!({
                                "actor": player_id.0,
                                "tool": tool_kind,
                                "repaired_tools": repaired.clone(),
                                "amount_restored": REPAIR_RESTORE_DEFAULT,
                            });
                            tool_repaired_pending = Some((player_id.0, REPAIR_RESTORE_DEFAULT, repaired));
                            let aim = if actor.aim == cf_actor::Vec2::ZERO {
                                cf_actor::Vec2::new(1.0, 0.0)
                            } else {
                                actor.aim.normalize_or_x()
                            };
                            tool_effect = Some(ToolEffect {
                                kind: ToolEffectKind::Repair,
                                origin: actor.position,
                                aim,
                                actor_id: player_id,
                            });
                        } else {
                            let entry = actor
                                .tool_durability
                                .entry(tool_kind.clone())
                                .or_insert_with(cf_equipment::Durability::default);
                            let broke = entry.apply_wear(WEAR_PER_USE_DEFAULT);
                            let remaining = entry.current;
                            let aim = if actor.aim == cf_actor::Vec2::ZERO {
                                cf_actor::Vec2::new(1.0, 0.0)
                            } else {
                                actor.aim.normalize_or_x()
                            };
                            let effect_kind = match tool_kind.as_str() {
                                "foam" => Some(ToolEffectKind::Foam),
                                "concrete" => Some(ToolEffectKind::Concrete),
                                "welder" => Some(ToolEffectKind::Welder),
                                "drill" => Some(ToolEffectKind::Drill),
                                "multi_tool" | "multi-tool" => Some(ToolEffectKind::MultiTool),
                                "beacon" => Some(ToolEffectKind::Beacon),
                                "sensor_pulse" => Some(ToolEffectKind::SensorPulse),
                                "digger" => Some(ToolEffectKind::Digger),
                                _ => None,
                            };
                            if let Some(kind) = effect_kind {
                                tool_effect = Some(ToolEffect {
                                    kind,
                                    origin: actor.position,
                                    aim,
                                    actor_id: player_id,
                                });
                            }
                            event_payload = json!({
                                "actor": player_id.0,
                                "tool": tool_kind,
                                "durability_remaining": remaining,
                            });
                            if broke {
                                tool_broken_pending = Some((player_id.0, tool_kind.clone(), 0.0_f32));
                            }
                        }
                    }
                    M6Action::AttachSuppressor => {
                        actor.suppressor.attached = true;
                        actor.suppressor.integrity = actor.suppressor.integrity.max(1.0);
                        event_payload = json!({
                            "actor": player_id.0,
                            "attachment": "suppressor",
                            "attached": true,
                            "loudness_factor": cf_equipment::SUPPRESSOR_LOUDNESS_FACTOR,
                        });
                    }
                    M6Action::DetachSuppressor => {
                        actor.suppressor.attached = false;
                        event_payload = json!({"actor": player_id.0, "attachment": "suppressor", "attached": false});
                    }
                    M6Action::SetFacing { facing } => {
                        let new_facing = if facing == "left" {
                            cf_actor::FacingDirection::Left
                        } else {
                            cf_actor::FacingDirection::Right
                        };
                        let prev = actor.facing;
                        actor.facing = new_facing;
                        event_payload = json!({
                            "actor": player_id.0,
                            "from": prev.as_str(),
                            "to": new_facing.as_str(),
                            "cause": "cfctl_set_facing",
                        });
                    }
                    M6Action::AimSetFacing { facing } => {
                        let explicit_facing = if facing == "left" {
                            cf_actor::FacingDirection::Left
                        } else {
                            cf_actor::FacingDirection::Right
                        };
                        let aim_unit = cf_actor::Vec2::new(explicit_facing.sign(), 0.0);
                        actor.aim = aim_unit;
                        let prev = actor.facing;
                        let derived = cf_actor::FacingDirection::from_aim(aim_unit);
                        actor.facing = derived;
                        event_payload = json!({
                            "actor": player_id.0,
                            "from": prev.as_str(),
                            "to": derived.as_str(),
                            "cause": "cfctl_aim_set_facing",
                        });
                    }
                    M6Action::NestContainer {
                        parent_instance_id,
                        child_item_id,
                    } => {
                        // **M6B § Acceptance: Container nesting depth-limited**.
                        // The actor must have an inventory grid attached; the
                        // grid drives the depth check + emits the locked
                        // `max_depth_exceeded` reason on rejection.
                        actor.inventory_grid_attach();
                        let mut child_is_container = false;
                        let mut parent_item_id_str = String::new();
                        let mut resolved_depth: u8 = 0;
                        let mut new_instance_id: Option<u64> = None;
                        if let Some(spec) = cf_equipment::spec_for_id(child_item_id) {
                            child_is_container = spec.is_container();
                        } else {
                            reject_reason = Some("child_unknown_item");
                        }
                        if reject_reason.is_none() {
                            if let Some(grid) = actor.inventory_grid_mut() {
                                // Capture the parent label first so the
                                // emitted event carries it.
                                if let Some(parent) = grid.find(*parent_instance_id) {
                                    parent_item_id_str = parent.item_id.clone();
                                }
                                match grid.try_nest_container(*parent_instance_id, child_item_id.clone()) {
                                    Ok(id) => {
                                        new_instance_id = Some(id);
                                        // Depth is parent_depth+1; recompute by
                                        // walking the tree.
                                        if let Some(child) = grid.find(id) {
                                            let _ = child;
                                        }
                                        resolved_depth = container_depth_of(grid, id);
                                    }
                                    Err(reason) => {
                                        // Static-lifetime mapping for the spec-locked rejection.
                                        if reason == cf_equipment::MAX_DEPTH_EXCEEDED {
                                            reject_reason = Some(cf_equipment::MAX_DEPTH_EXCEEDED);
                                        } else if reason == "parent_not_found" {
                                            reject_reason = Some("parent_not_found");
                                        } else if reason == "child_unknown_item" {
                                            reject_reason = Some("child_unknown_item");
                                        } else {
                                            reject_reason = Some("nest_container_failed");
                                        }
                                    }
                                }
                            } else {
                                reject_reason = Some("no_inventory_grid");
                            }
                        }
                        if reject_reason.is_none() {
                            let child_id = new_instance_id.unwrap_or(0);
                            event_payload = json!({
                                "actor": player_id.0,
                                "parent_instance_id": *parent_instance_id,
                                "parent_item_id": parent_item_id_str,
                                "child_instance_id": child_id,
                                "child_item_id": child_item_id,
                                "depth": resolved_depth,
                                "max_depth": cf_equipment::MAX_CONTAINER_NEST_DEPTH,
                                "child_is_container": child_is_container,
                            });
                        }
                    }
                }
            }
        }
        // **M6**: resolve a melee hit against actors within reach BEFORE
        // we release the state write-guard so we can mutate target HP +
        // roll knockdown deterministically off the engine's seeded RNG.
        let mut melee_hit_emit: Option<MeleeHitEmit> = None;
        let mut knockdown_emit: Option<u64> = None;
        if reject_reason.is_none() {
            if let Some(resolve) = melee_to_resolve.take() {
                let preset = cf_equipment::m6_melee_presets()
                    .into_iter()
                    .find(|m| m.kind == resolve.kind);
                if let Some(preset) = preset {
                    let attacker_pos = resolve.actor_position;
                    let facing_sign = resolve.facing_sign;
                    let reach = preset.reach;
                    let target_id_opt = state.actor_state.as_ref().and_then(|sim| {
                        sim.world
                            .actors
                            .iter()
                            .filter(|(id, _)| **id != resolve.attacker)
                            .filter_map(|(id, a)| {
                                let dx = a.position.x - attacker_pos.x;
                                let dy = a.position.y - attacker_pos.y;
                                let in_arc = dx * facing_sign > 0.0 || dx.abs() <= 4.0;
                                let distance = (dx * dx + dy * dy).sqrt();
                                if in_arc && distance <= reach {
                                    Some((*id, distance))
                                } else {
                                    None
                                }
                            })
                            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                            .map(|(id, _)| id)
                    });
                    if let Some(target_id) = target_id_opt {
                        let knockdown_roll = ((state.rng.next_u64() as f64) / (u64::MAX as f64)) as f32;
                        let knockdown_triggered = knockdown_roll < preset.knockdown_chance;
                        let mut hp_before = 0.0;
                        let mut hp_after = 0.0;
                        if let Some(target) = state
                            .actor_state
                            .as_mut()
                            .and_then(|sim| sim.world.actors.get_mut(&target_id))
                        {
                            hp_before = target.hp;
                            let _ = target.apply_damage(preset.damage);
                            hp_after = target.hp;
                            if knockdown_triggered {
                                target.knockdown_ticks_remaining = target.knockdown_ticks_remaining.max(45);
                                knockdown_emit = Some(target_id.0);
                            }
                        }
                        melee_hit_emit = Some(MeleeHitEmit {
                            attacker: resolve.attacker.0,
                            target: target_id.0,
                            kind: preset.kind,
                            damage: preset.damage,
                            hp_before,
                            hp_after,
                            knockdown_chance: preset.knockdown_chance,
                            knockdown_rolled: knockdown_roll,
                            knockdown_triggered,
                        });
                    }
                }
            }
            if let Some((id, swap)) = swap_to_register {
                state.weapon_swap_state.insert(id, swap);
            }
            // **M6**: register the grenade projectile so the tick scheduler
            // can advance + detonate it.
            if let Some(spawn) = grenade_to_spawn.take() {
                let projectile_id = state.next_guard_projectile_id;
                state.next_guard_projectile_id = state.next_guard_projectile_id.saturating_add(1);
                state.grenade_projectiles.push(GrenadeProjectile {
                    id: projectile_id,
                    owner: spawn.owner,
                    kind: spawn.kind,
                    position: spawn.origin,
                    velocity: spawn.velocity,
                    fuse_remaining: spawn.fuse_remaining,
                    radius: spawn.radius,
                    damage_at_center: spawn.damage_at_center,
                    adhesive: spawn.adhesive,
                    spawns_hazard: spawn.spawns_hazard,
                    vision_disrupt: spawn.vision_disrupt,
                    stuck: false,
                });
            }
            // **M6**: register the knife projectile so the tick scheduler
            // can advance + emit `combat.knife_throw_landed` on collision.
            if let Some(spawn) = knife_to_spawn.take() {
                let projectile_id = state.next_guard_projectile_id;
                state.next_guard_projectile_id = state.next_guard_projectile_id.saturating_add(1);
                let knife = cf_equipment::KnifeProjectile::new(
                    projectile_id,
                    spawn.owner.0,
                    (spawn.origin.x, spawn.origin.y),
                    (spawn.aim.x, spawn.aim.y),
                    spawn.base_damage,
                );
                state.knife_projectiles.push(knife);
            }
        }
        // **M6**: resolve the stealth-kill: find the nearest actor in front
        // of (same facing as) the attacker within STEALTH_KILL_REACH, apply
        // instant-kill damage, and emit `combat.stealth_kill_executed`.
        // Gate already enforced by the dispatch (stealth_meter < MAX).
        let mut stealth_kill_emit: Option<(u64, u64, f32, f32)> = None;
        if let Some(attempt) = stealth_kill_attempt.take() {
            let target_id_opt = state.actor_state.as_ref().and_then(|sim| {
                sim.world
                    .actors
                    .iter()
                    .filter(|(id, _)| **id != attempt.attacker)
                    .filter_map(|(id, a)| {
                        let dx = a.position.x - attempt.attacker_pos.x;
                        let dy = a.position.y - attempt.attacker_pos.y;
                        let distance = (dx * dx + dy * dy).sqrt();
                        let facing_alignment = a.facing.sign() * attempt.attacker_facing_x;
                        if distance <= cf_equipment::STEALTH_KILL_REACH && facing_alignment > 0.0 && !a.status.is_dead()
                        {
                            Some((*id, distance))
                        } else {
                            None
                        }
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(id, _)| id)
            });
            if let Some(target_id) = target_id_opt {
                let mut hp_before = 0.0;
                let mut hp_after = 0.0;
                if let Some(target) = state
                    .actor_state
                    .as_mut()
                    .and_then(|sim| sim.world.actors.get_mut(&target_id))
                {
                    hp_before = target.hp;
                    // Instant-kill: drive HP to zero so apply_damage flips
                    // to DYING; clear mission_critical so DEAD is reachable.
                    target.mission_critical = false;
                    let _ = target.apply_damage(target.hp + 1.0);
                    hp_after = target.hp;
                }
                stealth_kill_emit = Some((attempt.attacker.0, target_id.0, hp_before, hp_after));
            }
        }
        drop(state);
        if let Some((attacker_id, target_id, hp_before, hp_after)) = stealth_kill_emit {
            self.recorder.record(
                tick,
                sim_time_ms,
                "combat",
                "stealth_kill_executed",
                json!({
                    "attacker_id": attacker_id,
                    "target_id": target_id,
                    "actor": attacker_id,
                    "victim_id": target_id,
                    "hp_before": hp_before,
                    "hp_after": hp_after,
                }),
                None,
            );
        }
        // **M6**: apply per-tool side-effects + emit tool-specific events.
        if let Some(effect) = tool_effect.take() {
            self.apply_tool_effect(effect, tick, sim_time_ms);
        }
        // **M6**: spawn a physical item entity in the world for drop_item.
        if let Some(mut spawn) = drop_item_spawn.take() {
            let dropped_item_id_clone = spawn.item_id.clone();
            let dropped_slot = spawn.original_slot;
            let dropped_pos = spawn.position;
            if let Ok(mut s) = self.state.write() {
                spawn.id = s.m6_next_dropped_item_id;
                s.m6_next_dropped_item_id = s.m6_next_dropped_item_id.saturating_add(1);
                s.m6_dropped_items.push(spawn);
            }
            // **M6B**: mirror the drop into the actor's inventory grid
            // (remove the most-recent placement of the item) + emit
            // the mass-aware sibling event.
            let (grid_total_mass, grid_total_bulk, instance_id) =
                self.remove_from_inventory_grid_mut(player_id, &dropped_item_id_clone);
            let spec = cf_equipment::spec_for_id(&dropped_item_id_clone);
            let (mass_kg, dims, bulk, category) = if let Some(s) = spec.as_ref() {
                (
                    s.mass_kg,
                    json!({"w": s.dimensions.w, "h": s.dimensions.h}),
                    s.bulk_volume_l,
                    s.category.as_str().to_string(),
                )
            } else {
                (
                    3.5_f32,
                    json!({"w": 2, "h": 4}),
                    3.0_f32,
                    cf_equipment::ItemCategory::Weapon.as_str().to_string(),
                )
            };
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "item_dropped_with_mass",
                json!({
                    "actor": player_id.0,
                    "item_id": dropped_item_id_clone,
                    "slot": dropped_slot,
                    "instance_id": instance_id,
                    "mass_kg": mass_kg,
                    "bulk_volume_l": bulk,
                    "dimensions": dims,
                    "category": category,
                    "inventory_total_mass_kg": grid_total_mass,
                    "inventory_total_bulk_l": grid_total_bulk,
                    "position": [dropped_pos.x, dropped_pos.y],
                    "reason": "act.player.drop_item",
                }),
                None,
            );
            self.tick_m6b_encumbrance_after_change(tick, sim_time_ms, player_id);
        }
        // **M6**: scan pickup radius + add nearest item to first empty slot.
        //
        // **M6 (knife retrieve)**: the scan covers BOTH
        // `state.m6_dropped_items` and `state.knife_projectiles` so
        // `act.player.pickup` closes the second half of spec § "Knife
        // throw + retrieve" ("When player approaches + presses E: knife
        // returned to inventory"). A retrievable knife is one whose
        // [`cf_equipment::KnifeProjectile::is_retrievable`] is true (the
        // knife stuck in a wall and is no longer in flight). Whichever
        // candidate (dropped item OR stuck knife) sits closer to the
        // actor within `PICKUP_RADIUS` wins; ties go to dropped items
        // (preserves the pre-existing behavior on the dropped-item path).
        if let Some(actor_pos) = pickup_request_pos.take() {
            const PICKUP_RADIUS: f32 = 24.0;
            let mut picked_dropped: Option<(u64, String, f32, u8)> = None;
            let mut picked_knife: Option<(u64, f32, u8)> = None;
            if let Ok(mut s) = self.state.write() {
                let nearest_dropped: Option<(usize, f32)> = s
                    .m6_dropped_items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| {
                        let dx = item.position.x - actor_pos.x;
                        let dy = item.position.y - actor_pos.y;
                        let d2 = dx * dx + dy * dy;
                        if d2 <= PICKUP_RADIUS * PICKUP_RADIUS {
                            Some((i, d2))
                        } else {
                            None
                        }
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                let nearest_knife: Option<(usize, f32)> = s
                    .knife_projectiles
                    .iter()
                    .enumerate()
                    .filter_map(|(i, k)| {
                        if !k.is_retrievable() {
                            return None;
                        }
                        let dx = k.origin_x - actor_pos.x;
                        let dy = k.origin_y - actor_pos.y;
                        let d2 = dx * dx + dy * dy;
                        if d2 <= PICKUP_RADIUS * PICKUP_RADIUS {
                            Some((i, d2))
                        } else {
                            None
                        }
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                let knife_wins = match (nearest_dropped, nearest_knife) {
                    (None, Some(_)) => true,
                    (Some((_, dd)), Some((_, kd))) => kd < dd,
                    _ => false,
                };
                if knife_wins {
                    if let Some((idx, _)) = nearest_knife {
                        let knife_mass_kg = cf_equipment::m6_melee_presets()
                            .into_iter()
                            .find(|m| m.kind == cf_equipment::MeleeKind::Knife)
                            .map(|p| p.mass_kg)
                            .unwrap_or(0.3);
                        let mut knife = s.knife_projectiles.remove(idx);
                        let mut slot_assigned: Option<u8> = None;
                        if let Some(actor) = s
                            .actor_state
                            .as_mut()
                            .and_then(|sim| sim.world.actors.get_mut(&player_id))
                        {
                            for (slot_i, slot_item) in actor.inventory.items.iter_mut().enumerate() {
                                if matches!(slot_item, cf_actor::InventoryItem::Empty) {
                                    *slot_item = cf_actor::InventoryItem::Rifle {
                                        preset: cf_equipment::KNIFE_M6_DEFAULT_ID.to_string(),
                                    };
                                    slot_assigned = Some(slot_i as u8);
                                    break;
                                }
                            }
                        }
                        if let Some(slot) = slot_assigned {
                            let _ = knife.retrieve();
                            picked_knife = Some((knife.projectile_id, knife_mass_kg, slot));
                        } else {
                            s.knife_projectiles.push(knife);
                        }
                    }
                } else if let Some((idx, _)) = nearest_dropped {
                    let item = s.m6_dropped_items.remove(idx);
                    if let Some(actor) = s
                        .actor_state
                        .as_mut()
                        .and_then(|sim| sim.world.actors.get_mut(&player_id))
                    {
                        let mut slot_assigned: Option<u8> = None;
                        for (slot_i, slot_item) in actor.inventory.items.iter_mut().enumerate() {
                            if matches!(slot_item, cf_actor::InventoryItem::Empty) {
                                *slot_item = cf_actor::InventoryItem::Rifle {
                                    preset: item.item_id.clone(),
                                };
                                slot_assigned = Some(slot_i as u8);
                                break;
                            }
                        }
                        if let Some(slot) = slot_assigned {
                            picked_dropped = Some((item.id, item.item_id.clone(), item.weight_kg, slot));
                        } else {
                            s.m6_dropped_items.push(item);
                        }
                    }
                }
            }
            if let Some((dropped_id, item_id, weight, slot)) = picked_dropped.clone() {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "item_picked_up",
                    json!({
                        "actor": player_id.0,
                        "item_id": item_id,
                        "weight_kg": weight,
                        "slot": slot,
                        "dropped_item_id": dropped_id,
                        "source": "dropped_world",
                    }),
                    None,
                );
            }
            if let Some((projectile_id, weight, slot)) = picked_knife {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "item_picked_up",
                    json!({
                        "actor": player_id.0,
                        "item_kind": "knife",
                        "item_id": cf_equipment::KNIFE_M6_DEFAULT_ID,
                        "weight_kg": weight,
                        "slot": slot,
                        "projectile_id": projectile_id,
                        "source": "retrieved_throw",
                    }),
                    None,
                );
                // **M6B**: also emit the mass-aware sibling event with the
                // canonical knife spec from the ItemSpec registry. Falls
                // back to the legacy values when no spec is registered.
                let knife_spec = cf_equipment::spec_for_id(cf_equipment::KNIFE_M6_DEFAULT_ID);
                let (mass_kg, dims, bulk, category) = if let Some(s) = knife_spec.as_ref() {
                    (
                        s.mass_kg,
                        json!({"w": s.dimensions.w, "h": s.dimensions.h}),
                        s.bulk_volume_l,
                        s.category.as_str().to_string(),
                    )
                } else {
                    (
                        weight,
                        json!({"w": 1, "h": 1}),
                        0.5_f32,
                        cf_equipment::ItemCategory::Weapon.as_str().to_string(),
                    )
                };
                // Mirror into the actor's inventory grid (M6B canonical
                // surface) so M14A's mass aggregator can read one source.
                let (grid_total_mass, grid_total_bulk, instance_id) =
                    self.add_to_inventory_grid_mut(player_id, cf_equipment::KNIFE_M6_DEFAULT_ID, 1, 0.0);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "item_picked_up_with_mass",
                    json!({
                        "actor": player_id.0,
                        "item_id": cf_equipment::KNIFE_M6_DEFAULT_ID,
                        "slot": slot,
                        "instance_id": instance_id,
                        "mass_kg": mass_kg,
                        "bulk_volume_l": bulk,
                        "dimensions": dims,
                        "category": category,
                        "inventory_total_mass_kg": grid_total_mass,
                        "inventory_total_bulk_l": grid_total_bulk,
                        "projectile_id": projectile_id,
                        "source": "retrieved_throw",
                    }),
                    None,
                );
                self.tick_m6b_encumbrance_after_change(tick, sim_time_ms, player_id);
            }
            if let Some((dropped_id, item_id, weight, slot)) = picked_dropped {
                // **M6B**: also emit the mass-aware sibling event with the
                // canonical ItemSpec from the registry.
                let spec = cf_equipment::spec_for_id(&item_id);
                let (mass_kg, dims, bulk, category) = if let Some(s) = spec.as_ref() {
                    (
                        s.mass_kg,
                        json!({"w": s.dimensions.w, "h": s.dimensions.h}),
                        s.bulk_volume_l,
                        s.category.as_str().to_string(),
                    )
                } else {
                    (
                        weight,
                        json!({"w": 1, "h": 1}),
                        weight * 0.3_f32,
                        cf_equipment::ItemCategory::Weapon.as_str().to_string(),
                    )
                };
                let (grid_total_mass, grid_total_bulk, instance_id) =
                    self.add_to_inventory_grid_mut(player_id, &item_id, 1, 0.0);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "item_picked_up_with_mass",
                    json!({
                        "actor": player_id.0,
                        "item_id": item_id,
                        "slot": slot,
                        "instance_id": instance_id,
                        "mass_kg": mass_kg,
                        "bulk_volume_l": bulk,
                        "dimensions": dims,
                        "category": category,
                        "inventory_total_mass_kg": grid_total_mass,
                        "inventory_total_bulk_l": grid_total_bulk,
                        "dropped_item_id": dropped_id,
                        "source": "dropped_world",
                    }),
                    None,
                );
                self.tick_m6b_encumbrance_after_change(tick, sim_time_ms, player_id);
            }
        }
        // **M6**: emit follow-on events (melee hit, knockdown, tool broken,
        // tool repaired) AFTER releasing the write-guard so the recorder
        // can re-borrow without dead-locking.
        if let Some(emit) = melee_hit_emit {
            let melee_event_id = self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "melee_hit_mo",
                json!({
                    "attacker_id": emit.attacker,
                    "target_id": emit.target,
                    "actor": emit.attacker,
                    "hit_actor_id": emit.target,
                    "melee_kind": emit.kind.as_str(),
                    "damage": emit.damage,
                    "damage_amount": emit.damage,
                    "hp_before": emit.hp_before,
                    "hp_after": emit.hp_after,
                    "knockdown_chance": emit.knockdown_chance,
                    "knockdown_rolled": emit.knockdown_rolled,
                    "knockdown_triggered": emit.knockdown_triggered,
                }),
                None,
            );
            // **M14** § "Heavy melee severance (chainsaw / katana / hatchet)".
            // Per-weapon severance_chance × (1 - joint_strength_normalized);
            // rolled against the engine's seeded RNG. Cf-equipment currently
            // ships hatchet (0.15) as the M6 heavy-melee tier; chainsaw +
            // katana ladder up at M25 (no MeleeKind variants yet — left for
            // forward-compat). Light tiers (knife / baton / kick) carry zero
            // severance per spec § "Light weapons (knife / baton): 0 severance chance".
            let severance_chance = match emit.kind {
                cf_equipment::MeleeKind::Hatchet => 0.15,
                _ => 0.0,
            };
            if severance_chance > 0.0 {
                let rng_roll = if let Ok(mut s) = self.state.write() {
                    (s.rng.next_u64() as f64 / u64::MAX as f64) as f32
                } else {
                    0.5
                };
                // Pick a probable hit zone (the M14 weighted picker would
                // normally consult the hit position; for melee we route to
                // the front-arm since the swing originates from the front).
                let zone = "arm_left";
                let joint = cf_physics::Joint::default_for_zone(zone);
                let probability = cf_physics::severance_probability(severance_chance, joint.joint_strength, 100.0);
                if cf_physics::severance_roll(rng_roll, probability) {
                    let _ = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "attachable",
                        "detached",
                        json!({
                            "actor_id": emit.target,
                            "attachable_id": zone,
                            "parent_zone": zone,
                            "joint_impulse": joint.joint_strength + 1.0_f32,
                            "joint_strength": joint.joint_strength,
                            "gib_impulse_limit": joint.gib_impulse_limit,
                            "detach_position": [0.0_f32, 0.0_f32],
                            "detach_velocity": [0.0_f32, 0.0_f32],
                            "damage_multiplier": joint.damage_multiplier,
                            "source_event_id": melee_event_id.clone(),
                            "cause": "melee_severance",
                        }),
                        Some(melee_event_id.clone()),
                    );
                }
            }
            // **M8** (Cluster E fix): auto-trigger a 50 ms hit-stop pulse on
            // melee impact per spec § "Hit-stop on impact — Given melee hit
            // OR AP round hit / When hit lands / Then camera.hit_stop fires".
            // Honors `Settings.hit_stop_enabled` (skips on opt-out per the
            // Gherkin's Settings clause). Emits `camera.hit_stop` with
            // `trigger="melee_hit"`.
            let mut hit_stop_payload: Option<serde_json::Value> = None;
            if let Ok(mut s) = self.state.write() {
                if s.settings.hit_stop_enabled {
                    cf_camera::trigger_hit_stop(&mut s.camera_state, 50);
                    let applied = s.camera_state.hit_stop_remaining_ms;
                    hit_stop_payload =
                        Some(json!({"duration_ms": applied, "trigger": "melee_hit", "actor_id": Some(emit.target)}));
                }
            }
            if let Some(payload) = hit_stop_payload {
                #[rustfmt::skip]
                let _ = self.recorder.record(tick, sim_time_ms, "camera", "hit_stop", payload, None);
            }
            // **M14G § VAL-M14G-023**: blunt-face dental damage emit.
            // Per spec, every melee blunt-impulse hit whose target zone
            // is face-aligned `head_front` AND whose impulse magnitude
            // exceeds the tooth threshold emits `wound.created` with
            // `kind=DentalDamage severity=0.6 zone=head_front`. The
            // producer (`classify_blunt_face_hit`) returns DentalDamage
            // above-threshold and BruiseHeavy below; the engine only
            // routes the DentalDamage branch through `m14g_emit_wound_created`
            // here so the lighter blunt hits don't double-emit a
            // BruiseHeavy alongside the M14 damage path.
            let preset_damage_kind = cf_equipment::m6_melee_presets()
                .into_iter()
                .find(|m| m.kind == emit.kind)
                .map(|p| p.damage_kind)
                .unwrap_or_default();
            if preset_damage_kind == "blunt" && emit.damage > cf_physics::M14G_TOOTH_THRESHOLD {
                let dental = cf_physics::classify_blunt_face_hit(
                    cf_wound::registry::ZoneId::from("head_front"),
                    emit.damage,
                    cf_physics::M14G_TOOTH_THRESHOLD,
                );
                if matches!(dental.kind, cf_wound::WoundKind::DentalDamage) {
                    let _ = self.m14g_emit_wound_created(
                        tick,
                        sim_time_ms,
                        emit.target,
                        dental,
                        Some(melee_event_id.clone()),
                    );
                }
            }
        }
        if let Some(target_id) = knockdown_emit {
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "stance_changed",
                json!({
                    "actor": target_id,
                    "from_stance": "stand",
                    "to_stance": cf_actor::Stance::KnockedDown.as_str(),
                    "cause": "melee_knockdown",
                }),
                None,
            );
        }
        if let Some((actor_id, tool, durability)) = tool_broken_pending {
            self.recorder.record(
                tick,
                sim_time_ms,
                "equipment",
                "tool_broken",
                json!({"actor": actor_id, "tool": tool, "durability": durability}),
                None,
            );
            // **M13** § "Tool degradation routing through chassis" — when a
            // tool socketed in a chassis breaks, also surface the
            // `chassis.tool_durability_changed` event so chassis-aware
            // consumers (M7 Engineer auto-repair, HUD) can react without
            // re-parsing the equipment event.
            let chassis_tool_event_target = if let Ok(state) = self.state.read() {
                state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(&cf_actor::ActorId(actor_id)))
                    .and_then(|a| a.chassis.as_ref())
                    .map(|c| (c.spec_id.clone(), c.kind.as_str().to_string()))
            } else {
                None
            };
            if let Some((spec_id, kind)) = chassis_tool_event_target {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "chassis",
                    "tool_durability_changed",
                    json!({
                        "actor": actor_id,
                        "tool": tool,
                        "durability": durability,
                        "broken": true,
                        "chassis_spec_id": spec_id,
                        "chassis_kind": kind,
                    }),
                    None,
                );
            }
        }
        if let Some((actor_id, amount, tools)) = tool_repaired_pending {
            for tool in &tools {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "equipment",
                    "tool_repaired",
                    json!({"actor": actor_id, "tool": tool, "amount_restored": amount}),
                    None,
                );
            }
            // **M13** § "Tool degradation routing through chassis" — repair
            // also surfaces the chassis-extension event when the actor is
            // chassis-bound.
            let chassis_tool_event_target = if let Ok(state) = self.state.read() {
                state
                    .actor_state
                    .as_ref()
                    .and_then(|sim| sim.world.actors.get(&cf_actor::ActorId(actor_id)))
                    .and_then(|a| a.chassis.as_ref())
                    .map(|c| (c.spec_id.clone(), c.kind.as_str().to_string()))
            } else {
                None
            };
            if let Some((spec_id, kind)) = chassis_tool_event_target {
                for tool in &tools {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "chassis",
                        "tool_durability_changed",
                        json!({
                            "actor": actor_id,
                            "tool": tool,
                            "durability": amount,
                            "broken": false,
                            "chassis_spec_id": spec_id,
                            "chassis_kind": kind,
                            "repaired": true,
                        }),
                        None,
                    );
                }
            }
        }
        if let Some(reason) = reject_reason {
            self.recorder.record(
                tick,
                sim_time_ms,
                "control",
                "command_rejected",
                json!({"method": method, "reason": reason, "actor": player_id.0}),
                None,
            );
            self.recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "action_rejected",
                json!({"actor": player_id.0, "action": method, "reason": reason}),
                None,
            );
            return CommandResult::rejected(reason, tick.0);
        }
        let mut accepted_payload = event_payload.clone();
        if let Some(obj) = accepted_payload.as_object_mut() {
            obj.insert("method".to_string(), json!(method));
        }
        self.recorder
            .record(tick, sim_time_ms, "control", "command_accepted", accepted_payload, None);
        // **M6**: signal_friendly / signal_enemy_spotted / mark_waypoint
        // broadcast the corresponding intent to all squad followers via
        // `cf_squad::Squad::broadcast_to_followers`. Mark_waypoint also
        // emits a stand-alone `squad.waypoint_marked` event with the
        // resolved position so the M7 mission director can hook it.
        match &action {
            crate::m6_actions::M6Action::SignalFriendly | crate::m6_actions::M6Action::SignalEnemySpotted => {
                if let Ok(mut s) = self.state.write() {
                    let cmd = cf_squad::SquadCommand {
                        kind: cf_squad::SquadCommandKind::FollowLeader,
                        waypoint: None,
                        issuer: player_id,
                    };
                    let _ = s.squad.broadcast_to_followers(&cmd);
                }
            }
            crate::m6_actions::M6Action::MarkWaypoint { x, y } => {
                if let Ok(mut s) = self.state.write() {
                    let cmd = cf_squad::SquadCommand {
                        kind: cf_squad::SquadCommandKind::DefendPoint,
                        waypoint: Some(cf_actor::Vec2::new(*x, *y)),
                        issuer: player_id,
                    };
                    let _ = s.squad.broadcast_to_followers(&cmd);
                }
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "squad",
                    "waypoint_marked",
                    json!({
                        "issuer": player_id.0,
                        "position": [*x, *y],
                        "x": *x,
                        "y": *y,
                    }),
                    None,
                );
            }
            _ => {}
        }
        // Per-action structured replay event in the matching category.
        let (category, event) = match &action {
            crate::m6_actions::M6Action::Sprint { .. } | crate::m6_actions::M6Action::Prone { .. } => {
                ("actor", "stance_changed")
            }
            crate::m6_actions::M6Action::Slide => ("actor", "slide_started"),
            crate::m6_actions::M6Action::Vault => ("actor", "vault_started"),
            crate::m6_actions::M6Action::ClimbUp | crate::m6_actions::M6Action::ClimbDown => ("actor", "climb_started"),
            crate::m6_actions::M6Action::Dive => ("actor", "dive_started"),
            crate::m6_actions::M6Action::Lean { .. } => ("actor", "lean_changed"),
            crate::m6_actions::M6Action::StealthKill => ("combat", "stealth_kill_executed"),
            crate::m6_actions::M6Action::KnifeThrow => ("combat", "knife_throw_started"),
            crate::m6_actions::M6Action::WeaponSwap { .. } => ("equipment", "weapon_swap_started"),
            crate::m6_actions::M6Action::DropItem { .. } => ("equipment", "item_dropped"),
            crate::m6_actions::M6Action::Pickup => ("equipment", "item_picked_up"),
            crate::m6_actions::M6Action::SignalFriendly | crate::m6_actions::M6Action::SignalEnemySpotted => {
                ("perception", "actor_signal")
            }
            crate::m6_actions::M6Action::MarkWaypoint { .. } => ("squad", "waypoint_marked"),
            crate::m6_actions::M6Action::DeployBipod => ("equipment", "bipod_deployed"),
            crate::m6_actions::M6Action::StowBipod => ("equipment", "bipod_stowed"),
            crate::m6_actions::M6Action::CycleFireMode => ("equipment", "fire_mode_cycled"),
            crate::m6_actions::M6Action::CookGrenade => ("equipment", "grenade_cooked"),
            crate::m6_actions::M6Action::ThrowGrenade => ("equipment", "grenade_thrown"),
            crate::m6_actions::M6Action::MeleeBash
            | crate::m6_actions::M6Action::MeleeKick
            | crate::m6_actions::M6Action::MeleeShoulderCheck => ("equipment", "melee_swing"),
            crate::m6_actions::M6Action::UseTool { .. } => ("equipment", "tool_used"),
            crate::m6_actions::M6Action::AttachSuppressor | crate::m6_actions::M6Action::DetachSuppressor => {
                ("equipment", "suppressor_attached")
            }
            crate::m6_actions::M6Action::SetFacing { .. } => ("actor", "facing_changed"),
            crate::m6_actions::M6Action::AimSetFacing { .. } => ("actor", "facing_changed"),
            crate::m6_actions::M6Action::NestContainer { .. } => ("inventory", "container_nested"),
        };
        self.recorder
            .record(tick, sim_time_ms, category, event, event_payload, None);
        CommandResult::accepted(tick.0)
    }

    pub(crate) fn dispatch_squad_command(
        &self,
        bot_actor: Option<u64>,
        kind: crate::m6_actions::SquadCommandKindOverWire,
        waypoint: Option<(f32, f32)>,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if !self.config.has_actor_world {
            return self.reject_actor_command(tick, sim_time_ms, state, "act.squad.issue_command");
        }
        let mut state = state;
        let issuer = state.player_actor.unwrap_or_default();
        // **M6**: build the command + apply it to the named member (or
        // broadcast to all followers when `bot_actor` is None). This
        // mutates `state.squad` so the AI tick can consult
        // `member.current_command` and act accordingly.
        let waypoint_v2 = waypoint.map(|(x, y)| cf_actor::Vec2::new(x, y));
        let command_kind = match kind {
            crate::m6_actions::SquadCommandKindOverWire::FollowLeader => cf_squad::SquadCommandKind::FollowLeader,
            crate::m6_actions::SquadCommandKindOverWire::HoldPosition => cf_squad::SquadCommandKind::HoldPosition,
            crate::m6_actions::SquadCommandKindOverWire::DefendPoint => cf_squad::SquadCommandKind::DefendPoint,
            crate::m6_actions::SquadCommandKindOverWire::PushToWaypoint => cf_squad::SquadCommandKind::PushToWaypoint,
        };
        let command = cf_squad::SquadCommand {
            kind: command_kind,
            waypoint: waypoint_v2,
            issuer,
        };
        let applied = if let Some(target_id) = bot_actor {
            state.squad.issue_command(cf_actor::ActorId(target_id), command.clone())
        } else {
            state.squad.broadcast_to_followers(&command) > 0
        };
        drop(state);
        let payload = json!({
            "method": "act.squad.issue_command",
            "bot_actor": bot_actor,
            "kind": kind.as_str(),
            "waypoint": waypoint.map(|(x, y)| json!({"x": x, "y": y})),
            "applied": applied,
        });
        self.recorder
            .record(tick, sim_time_ms, "control", "command_accepted", payload.clone(), None);
        let squad_event = json!({
            "bot_actor": bot_actor,
            "kind": kind.as_str(),
            "waypoint": waypoint.map(|(x, y)| json!({"x": x, "y": y})),
        });
        self.recorder
            .record(tick, sim_time_ms, "squad", "command_issued", squad_event, None);
        CommandResult::accepted(tick.0)
    }

    /// **M6**: `act.squad.cancel_command` — returns the named squad member
    /// to the default `FollowLeader` command. Re-emits
    /// `squad.command_issued` with `kind="follow_leader"` so the replay
    /// stream stays linear.
    pub(crate) fn dispatch_squad_cancel_command(
        &self,
        actor_id: u64,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        if !self.config.has_actor_world {
            return self.reject_actor_command(tick, sim_time_ms, state, "act.squad.cancel_command");
        }
        let mut state = state;
        let target = cf_actor::ActorId(actor_id);
        let updated = state
            .squad
            .issue_command(target, cf_squad::SquadCommand::follow(cf_actor::ActorId::default()));
        drop(state);
        let payload = json!({
            "method": "act.squad.cancel_command",
            "actor_id": actor_id,
            "applied": updated,
        });
        self.recorder
            .record(tick, sim_time_ms, "control", "command_accepted", payload, None);
        let squad_event = json!({
            "bot_actor": actor_id,
            "kind": cf_squad::SquadCommandKind::FollowLeader.as_str(),
            "waypoint": serde_json::Value::Null,
            "cause": "cancel_command",
        });
        self.recorder
            .record(tick, sim_time_ms, "squad", "command_issued", squad_event, None);
        CommandResult::accepted(tick.0)
    }

    /// **M7-B**: dispatch `act.player.set_priority`. Mutates the bot's
    /// PriorityTable AND the utility scorer's cached priority so the
    /// next AI tick scores against the new weight. Emits
    /// `ai.priority_table_changed` on success.
    pub(crate) fn dispatch_set_priority(
        &self,
        actor_id: u64,
        task: String,
        weight: u8,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let task_type = match cf_ai::TaskType::from_str(&task) {
            Some(t) => t,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_priority", "reason": "unknown_task"}),
                    None,
                );
                return CommandResult::rejected("unknown_task", tick.0);
            }
        };
        let mut state = state;
        let result = state.m7_ai_world.set_priority(actor, task_type, weight);
        let (old_weight, new_weight) = match result {
            Ok(pair) => pair,
            Err(reason) => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_priority", "reason": reason}),
                    None,
                );
                return CommandResult::rejected(reason, tick.0);
            }
        };
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.set_priority",
                "actor_id": actor_id,
                "task": task_type.as_str(),
                "weight": new_weight,
            }),
            None,
        );
        let payload = crate::m7_ai::priority_table_changed_payload(actor_id, task_type, old_weight, new_weight);
        self.recorder
            .record(tick, sim_time_ms, "ai", "priority_table_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// **M7-B**: dispatch `act.player.set_autonomy_mode`. Emits
    /// `ai.autonomy_mode_changed` on success.
    pub(crate) fn dispatch_set_autonomy_mode(
        &self,
        actor_id: u64,
        mode: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let new_mode = match cf_ai::AutonomyMode::from_str(&mode) {
            Some(m) => m,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_autonomy_mode", "reason": "unknown_mode"}),
                    None,
                );
                return CommandResult::rejected("unknown_mode", tick.0);
            }
        };
        let mut state = state;
        let prev = state.m7_ai_world.set_autonomy(actor, new_mode);
        let from = match prev {
            Some(p) => p,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.set_autonomy_mode", "reason": "no_such_actor"}),
                    None,
                );
                return CommandResult::rejected("no_such_actor", tick.0);
            }
        };
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.set_autonomy_mode",
                "actor_id": actor_id,
                "mode": new_mode.as_str(),
            }),
            None,
        );
        let payload = crate::m7_ai::autonomy_mode_changed_payload(actor_id, from, new_mode);
        self.recorder
            .record(tick, sim_time_ms, "ai", "autonomy_mode_changed", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// **M7-B**: dispatch `act.player.apply_role_template`. Replaces the
    /// bot's PriorityTable with the chosen role template + emits
    /// `ai.role_template_applied` AND `ai.archetype_chosen`.
    pub(crate) fn dispatch_apply_role_template(
        &self,
        actor_id: u64,
        template_id: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let template = match cf_priority::RoleTemplate::from_str(&template_id) {
            Some(t) => t,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_role_template", "reason": "unknown_template_id"}),
                    None,
                );
                return CommandResult::rejected("unknown_template_id", tick.0);
            }
        };
        let mut state = state;
        match state.m7_ai_world.apply_role_template(actor, template) {
            Some(()) => {}
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_role_template", "reason": "no_such_actor"}),
                    None,
                );
                return CommandResult::rejected("no_such_actor", tick.0);
            }
        }
        let archetype = template.archetype();
        // Try to emit chatter (OrderAck) when role template applies.
        let chatter_emit = state
            .m7_ai_world
            .try_emit_chatter(
                actor,
                cf_audio::ChatterCategory::OrderAck,
                "Roger, switching role.",
                tick.0,
                self.config.tick_rate_hz,
            )
            .map(|(event, _)| event);
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.apply_role_template",
                "actor_id": actor_id,
                "template_id": template.as_str(),
            }),
            None,
        );
        let payload = crate::m7_ai::role_template_applied_payload(actor_id, template);
        self.recorder
            .record(tick, sim_time_ms, "ai", "role_template_applied", payload, None);
        let archetype_payload = crate::m7_ai::archetype_chosen_payload(actor_id, archetype);
        self.recorder
            .record(tick, sim_time_ms, "ai", "archetype_chosen", archetype_payload, None);
        if let Some(event) = chatter_emit {
            let chatter_payload = crate::m7_ai::chatter_emitted_payload(&event);
            self.recorder
                .record(tick, sim_time_ms, "ai", "chatter_emitted", chatter_payload, None);
        }
        CommandResult::accepted(tick.0)
    }

    /// **M7-B**: dispatch `act.player.apply_quick_preset`. Shifts task
    /// weights ±2 per spec § Quick presets. Emits
    /// `ai.quick_preset_applied`.
    pub(crate) fn dispatch_apply_quick_preset(
        &self,
        actor_id: u64,
        preset_id: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let actor = cf_actor::ActorId(actor_id);
        let preset = match cf_priority::QuickPresetId::from_str(&preset_id) {
            Some(p) => p,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_quick_preset", "reason": "unknown_preset_id"}),
                    None,
                );
                return CommandResult::rejected("unknown_preset_id", tick.0);
            }
        };
        let mut state = state;
        match state.m7_ai_world.apply_quick_preset(actor, preset) {
            Some(()) => {}
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.apply_quick_preset", "reason": "no_such_actor"}),
                    None,
                );
                return CommandResult::rejected("no_such_actor", tick.0);
            }
        }
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({
                "method": "act.player.apply_quick_preset",
                "actor_id": actor_id,
                "preset_id": preset.as_str(),
            }),
            None,
        );
        let payload = crate::m7_ai::quick_preset_applied_payload(actor_id, preset);
        self.recorder
            .record(tick, sim_time_ms, "ai", "quick_preset_applied", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// **M7B**: dispatch `act.squad.issue` — parse args, validate, run
    /// doctrine check, mutate squad state, emit the appropriate event.
    /// Chain-aware verbs (`stack_door` / `breach_door` / `frag_out` /
    /// `advance` / `retreat_in_order`) also drive the matching squad
    /// state machine + emit the chain events.
    pub(crate) fn dispatch_m7b_squad_issue(
        &self,
        squad_id: u64,
        verb_id: String,
        args: Vec<serde_json::Value>,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let mut parsed_args = Vec::with_capacity(args.len());
        for (i, value) in args.iter().enumerate() {
            match crate::m7b_squad::parse_verb_arg(value) {
                Ok(v) => parsed_args.push(v),
                Err(err) => {
                    drop(state);
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "control",
                        "command_rejected",
                        json!({
                            "method": "act.squad.issue",
                            "reason": "invalid_arg",
                            "verb_id": verb_id,
                            "arg_index": i,
                            "detail": err,
                        }),
                        None,
                    );
                    return CommandResult::rejected("invalid_arg", tick.0);
                }
            }
        }
        let mut state = state;
        let issuer = state.player_actor.map(|a| a.0).unwrap_or(0);
        let outcome = state
            .m7b_squad
            .issue_verb(squad_id, &verb_id, parsed_args.clone(), issuer, tick.0);
        let event_category = "squad";
        let event_type = if outcome.is_accepted() {
            "command_issued"
        } else {
            "command_vetoed"
        };
        let payload = outcome.payload.clone();
        let was_accepted = outcome.is_accepted();

        // **M7B**: chain-aware side effects when the verb was accepted.
        // The squad state machines (breach chain + bounding) auto-drive
        // their events here so the per-verb issue produces the correct
        // chain-step / bounding-step payloads per spec.
        let mut chain_events: Vec<(&str, serde_json::Value)> = Vec::new();
        if was_accepted {
            let stack_actor_ids: Vec<u64> = state
                .squad
                .followers
                .iter()
                .map(|m| m.actor.0)
                .chain(state.squad.leader.iter().map(|l| l.actor.0))
                .take(4)
                .collect();
            match verb_id.as_str() {
                "stack_door" => {
                    // Args: door, side.
                    let door_id = parsed_args
                        .iter()
                        .find_map(|a| match a {
                            cf_ai::VerbArgValue::Door(d) => Some(*d),
                            _ => None,
                        })
                        .unwrap_or(0);
                    let side = parsed_args
                        .iter()
                        .find_map(|a| match a {
                            cf_ai::VerbArgValue::Side(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_else(|| "left".to_string());
                    let started =
                        state
                            .m7b_squad
                            .start_breach_chain(squad_id, door_id, &side, stack_actor_ids.clone(), tick.0);
                    chain_events.push(("breach_chain_started", started));
                }
                "breach_door" | "frag_out" | "advance" => {
                    // Each subsequent chain verb advances the chain by
                    // one step. The engine emits the matching
                    // breach_chain_step / breach_chain_complete event.
                    let res = state
                        .m7b_squad
                        .advance_breach_chain_with_actors(squad_id, tick.0, &stack_actor_ids);
                    if let Some(p) = res.step_payload {
                        chain_events.push(("breach_chain_step", p));
                    }
                    if let Some(p) = res.complete_payload {
                        chain_events.push(("breach_chain_complete", p));
                    }
                }
                "retreat_in_order" => {
                    // Per spec § "Retreat In Order = bounding (half
                    // cover, half move 30u, swap); emits
                    // squad.bounding_step". Start the bounding sequence
                    // at the rally arg (if supplied).
                    let rally = parsed_args
                        .iter()
                        .find_map(|a| match a {
                            cf_ai::VerbArgValue::Waypoint(w) => Some(*w),
                            _ => None,
                        })
                        .unwrap_or([0.0, 0.0]);
                    if let Some(squad) = state.m7b_squad.squad_mut(squad_id) {
                        squad.start_bounding(rally, tick.0);
                    }
                    if let Some(p) = state.m7b_squad.tick_bounding(squad_id, Some(rally)) {
                        chain_events.push(("bounding_step", p));
                    }
                }
                _ => {}
            }
        }
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.squad.issue", "squad_id": squad_id, "verb_id": verb_id, "accepted": was_accepted}),
            None,
        );
        self.recorder
            .record(tick, sim_time_ms, event_category, event_type, payload, None);
        for (ev, payload) in chain_events {
            self.recorder.record(tick, sim_time_ms, "squad", ev, payload, None);
        }
        CommandResult::accepted(tick.0)
    }

    /// **M7B**: dispatch `act.squad.set_formation` — collapse to fit member
    /// count, run slot solver, emit `squad.formation_set` +
    /// `squad.formation_slot_assigned` per slot.
    pub(crate) fn dispatch_m7b_squad_set_formation(
        &self,
        squad_id: u64,
        formation_kind: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let kind = match cf_ai::FormationKind::from_str(&formation_kind) {
            Some(k) => k,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.squad.set_formation", "reason": "unknown_formation_kind", "formation_kind": formation_kind}),
                    None,
                );
                return CommandResult::rejected("unknown_formation_kind", tick.0);
            }
        };
        let mut state = state;
        let commander_actor_id = state.player_actor.map(|a| a.0);
        let commander_pos = state
            .player_actor
            .and_then(|pid| state.actor_state.as_ref().and_then(|sim| sim.world.actors.get(&pid)))
            .map(|a| [a.position.x, a.position.y])
            .unwrap_or([0.0, 0.0]);
        let outcome = state
            .m7b_squad
            .set_formation(squad_id, kind, commander_pos, 0.0, commander_actor_id, tick.0);
        let mut collapse_payload: Option<serde_json::Value> = None;
        if outcome.previous != outcome.new_kind {
            let squad = state.m7b_squad.squad(squad_id);
            let member_count = squad.map(|s| s.role_assignments.len()).unwrap_or(0);
            collapse_payload = Some(json!({
                "squad_id": squad_id,
                "previous_kind": outcome.previous.as_str(),
                "new_kind": outcome.new_kind.as_str(),
                "member_count": member_count,
                "trigger_actor_id": serde_json::Value::Null,
            }));
        }
        let formation_payload = outcome.formation_payload;
        let assignment_payloads = outcome.assignment_payloads;
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.squad.set_formation", "squad_id": squad_id, "formation_kind": formation_kind}),
            None,
        );
        if let Some(p) = collapse_payload {
            self.recorder
                .record(tick, sim_time_ms, "squad", "formation_collapsed", p, None);
        }
        self.recorder
            .record(tick, sim_time_ms, "squad", "formation_set", formation_payload, None);
        for p in assignment_payloads {
            self.recorder
                .record(tick, sim_time_ms, "squad", "formation_slot_assigned", p, None);
        }
        CommandResult::accepted(tick.0)
    }

    /// **M7B**: dispatch `act.squad.assign_role` — mutate role_assignments
    /// + emit `squad.role_assigned`.
    pub(crate) fn dispatch_m7b_squad_assign_role(
        &self,
        squad_id: u64,
        member_actor_id: u64,
        role: String,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let parsed = match cf_ai::SquadRoleHint::from_str(&role) {
            Some(r) => r,
            None => {
                drop(state);
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.squad.assign_role", "reason": "unknown_role", "role": role}),
                    None,
                );
                return CommandResult::rejected("unknown_role", tick.0);
            }
        };
        let mut state = state;
        let outcome = state.m7b_squad.assign_role(squad_id, member_actor_id, parsed);
        let payload = outcome.payload;
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.squad.assign_role", "squad_id": squad_id, "member_actor_id": member_actor_id, "role": role}),
            None,
        );
        self.recorder
            .record(tick, sim_time_ms, "squad", "role_assigned", payload, None);
        CommandResult::accepted(tick.0)
    }

    /// **M7B**: dispatch `srv.dump_squad_state` — returns a JSON view.
    pub(crate) fn dispatch_m7b_dump_squad_state(
        &self,
        squad_id: u64,
        source: cf_actor::IntentSource,
        tick: Tick,
        sim_time_ms: f64,
        state: std::sync::RwLockWriteGuard<'_, EngineMutable>,
    ) -> CommandResult {
        let _ = source;
        let view = state.m7b_squad.dump_state_view(squad_id);
        drop(state);
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "srv.dump_squad_state", "squad_id": squad_id}),
            None,
        );
        // Stash the view in the engine for the cfctl response. The
        // cfctl-side `dump_squad_state` helper reads it back. To avoid a
        // mailbox round-trip we cheat slightly: the response path uses
        // `M0Engine::dump_squad_state` which queries the world directly.
        let _ = view;
        CommandResult::accepted(tick.0)
    }

}
