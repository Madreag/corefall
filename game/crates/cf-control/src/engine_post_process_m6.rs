//! post_process_m6_fire_modes.
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
    pub(crate) fn post_process_m6_fire_modes(&self, report: &mut cf_actor::sim::StepReport) {
        let Ok(mut state) = self.state.write() else {
            return;
        };
        state.m6_charge_misfires.clear();

        struct ActorFire {
            actor: ActorId,
            fire_mode: cf_equipment::AdvancedFireMode,
            preset_id: String,
            charge_fraction: f32,
        }
        let mut fires: Vec<ActorFire> = Vec::new();
        for outcome in &report.actor_outcomes {
            if !outcome.fired {
                continue;
            }
            let Some(sim) = state.actor_state.as_ref() else {
                break;
            };
            let Some(actor) = sim.world.actors.get(&outcome.actor) else {
                continue;
            };
            let preset_id = match actor.inventory.selected_item() {
                cf_actor::InventoryItem::Rifle { preset } => preset.clone(),
                cf_actor::InventoryItem::Empty => String::new(),
            };
            fires.push(ActorFire {
                actor: outcome.actor,
                fire_mode: actor.weapon_fire_mode,
                preset_id,
                charge_fraction: actor.weapon_charge_fraction,
            });
        }

        for fire in &fires {
            match fire.fire_mode {
                cf_equipment::AdvancedFireMode::Burst3 => {
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&fire.actor) {
                            actor.burst3_remaining_shots = cf_equipment::BURST3_ROUND_COUNT.saturating_sub(1);
                            actor.burst3_next_fire_at_seconds = cf_equipment::BURST3_INTER_SHOT_SECONDS;
                        }
                    }
                }
                cf_equipment::AdvancedFireMode::Charge => {
                    let charge = fire.charge_fraction.clamp(0.0, 1.0);
                    let multiplier = cf_equipment::charge_damage_multiplier(charge);
                    let misfire = charge < cf_equipment::SNIPER_MISFIRE_BELOW;
                    let mut ids: Vec<u64> = Vec::new();
                    for spawn in report.spawned_projectiles.iter_mut() {
                        if spawn.owner == fire.actor {
                            spawn.damage *= multiplier;
                            ids.push(spawn.id);
                        }
                    }
                    if let Some(sim) = state.actor_state.as_mut() {
                        for proj in sim.projectiles.iter_mut() {
                            if ids.contains(&proj.id) {
                                proj.damage *= multiplier;
                            }
                        }
                        if let Some(actor) = sim.world.actors.get_mut(&fire.actor) {
                            actor.weapon_charge_fraction = 0.0;
                            actor.fire_held_prev = false;
                        }
                    }
                    state.m6_charge_misfires.insert(
                        fire.actor,
                        ChargeFireInfo {
                            charge_fraction: charge,
                            misfire,
                        },
                    );
                }
                cf_equipment::AdvancedFireMode::Arc => {
                    // Spawn a grenade_launcher arc projectile (gravity-affected
                    // GrenadeProjectile with blast_radius=60 per spec) in place
                    // of the straight-flight rifle round produced by the M1
                    // path. Only fires when the selected weapon is the M6
                    // grenade_launcher preset; other presets fall through to
                    // the default rifle projectile.
                    if fire.preset_id != cf_equipment::weapon::GRENADE_LAUNCHER_M6_DEFAULT_ID {
                        continue;
                    }
                    let preset = cf_equipment::weapon::grenade_launcher::grenade_launcher_m6_default();
                    let blast_radius = preset.firing.ai_blast_radius.max(60.0);
                    let damage_at_center = preset.firing.damage_per_hit;
                    let projectile_lifetime = preset.firing.projectile_lifetime_seconds.max(1.0);
                    let mut converted: Vec<(u64, cf_actor::Vec2, cf_actor::Vec2)> = Vec::new();
                    report.spawned_projectiles.retain(|spawn| {
                        if spawn.owner == fire.actor {
                            converted.push((spawn.id, spawn.origin, spawn.velocity));
                            false
                        } else {
                            true
                        }
                    });
                    if let Some(sim) = state.actor_state.as_mut() {
                        sim.projectiles
                            .retain(|p| !converted.iter().any(|(id, _, _)| *id == p.id));
                    }
                    for (proj_id, origin, velocity) in converted {
                        state.grenade_projectiles.push(GrenadeProjectile {
                            id: proj_id,
                            owner: fire.actor,
                            kind: cf_equipment::GrenadeKind::Frag,
                            position: origin,
                            velocity,
                            fuse_remaining: projectile_lifetime,
                            radius: blast_radius,
                            damage_at_center,
                            adhesive: false,
                            spawns_hazard: false,
                            vision_disrupt: false,
                            stuck: false,
                        });
                    }
                }
                cf_equipment::AdvancedFireMode::Single
                | cf_equipment::AdvancedFireMode::Auto
                | cf_equipment::AdvancedFireMode::Pump => {}
            }
        }
    }

}
