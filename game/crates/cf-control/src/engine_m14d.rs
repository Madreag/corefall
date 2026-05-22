//! M14D projectile-pair pass + schedule-trace methods.
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
    pub(crate) fn record_schedule_trace_marker(&self, marker: &'static str) {
        if let Ok(mut s) = self.state.write() {
            if s.m14d_schedule_trace.len() >= 120 {
                s.m14d_schedule_trace.pop_front();
            }
            s.m14d_schedule_trace.push_back(marker);
        }
    }

    /// markers. Surfaces the ordered ring buffer for engine integration
    /// tests asserting pass invocation ordering.
    pub fn m14d_schedule_trace_snapshot(&self) -> Vec<&'static str> {
        self.state
            .read()
            .map(|s| s.m14d_schedule_trace.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn m14d_pair_pass_invocations(&self) -> u64 {
        self.state.read().map(|s| s.m14d_pair_pass_invocations).unwrap_or(0)
    }

    /// projectile-pair pass.
    pub fn m14d_last_pair_pass_trace(&self) -> cf_physics::ProjectilePairPassTrace {
        self.state
            .read()
            .map(|s| s.m14d_last_pair_pass_trace)
            .unwrap_or_default()
    }

    /// surfaced to consumers (cf-killcam).
    pub fn m14d_replay_intercepts(&self) -> bool {
        self.state.read().map(|s| s.m14d_replay_intercepts).unwrap_or(false)
    }

    /// owner actor id (or 0 for base-mounted units). Returns the
    /// default idle [`cf_equipment::Cram`] if no intercept has ever
    /// engaged this owner. Callers should observe
    /// `cooldown_active`/`cooldown_ticks_remaining` to determine
    /// whether the unit may fire another APS pulse this tick.
    pub fn m14d_cram_cooldown(&self, owner_actor_id: u64) -> cf_equipment::Cram {
        self.state
            .read()
            .ok()
            .and_then(|s| s.m14d_cram_cooldowns.get(&owner_actor_id).copied())
            .unwrap_or_default()
    }

    /// used by integration tests to verify projectile consumption.
    pub fn m14d_projectile_pair_pool_len(&self) -> usize {
        self.state
            .read()
            .map(|s| s.m14d_projectile_pair_pool.len())
            .unwrap_or(0)
    }

    /// Returns a deep copy of every active projectile so tests can
    /// assert per-projectile state without holding the state lock.
    pub fn m14d_projectile_pair_pool_snapshot(&self) -> Vec<cf_physics::ProjectileSnapshot> {
        self.state
            .read()
            .map(|s| s.m14d_projectile_pair_pool.clone())
            .unwrap_or_default()
    }

    /// projectile CCD pass — STRICTLY between the actor-collision pass
    /// and the terrain pass. Drives the `cf_physics::projectile`
    /// broadphase + narrowphase against the projectile-pair pool
    /// authored by the scenario manifest, emits
    /// `collision.projectile_pair_contact` events for every resolved
    /// pair contact, prunes consumed projectiles from the pool, and
    /// (**VAL-M14D-006**) engages / decays C-RAM cooldown latches keyed
    /// by the firing APS laser's `owner_actor_id`.
    pub(crate) fn tick_m14d_projectile_pair(&self, tick: Tick, sim_time_ms: f64) {
        let tick_dt = 1.0 / (self.config.tick_rate_hz.max(1) as f32);
        let pool_snapshot = match self.state.read() {
            Ok(s) => s.m14d_projectile_pair_pool.clone(),
            Err(_) => return,
        };
        if let Ok(mut s) = self.state.write() {
            for cram in s.m14d_cram_cooldowns.values_mut() {
                cram.tick();
            }
        }
        let (contacts, trace) = if pool_snapshot.is_empty() {
            (Vec::new(), cf_physics::ProjectilePairPassTrace::default())
        } else {
            cf_physics::run_projectile_pair_pass(&pool_snapshot, tick_dt)
        };
        if let Ok(mut s) = self.state.write() {
            s.m14d_pair_pass_invocations = s.m14d_pair_pass_invocations.saturating_add(1);
            s.m14d_last_pair_pass_trace = trace;
        }
        for contact in &contacts {
            let payload = json!({
                "projectile_a_id": contact.a_id,
                "projectile_b_id": contact.b_id,
                "projectile_a_kind": contact.a_kind.as_str(),
                "projectile_b_kind": contact.b_kind.as_str(),
                "outcome": contact.outcome.as_str(),
                "intercept_point": contact.intercept_point,
                "toi": contact.toi,
                "convergence_deg": contact.convergence_deg,
                "a_energy_retained": contact.a_energy_retained,
                "b_energy_retained": contact.b_energy_retained,
                "cosmetic": contact.cosmetic,
            });
            let pair_event_id = self
                .recorder
                .record(tick, sim_time_ms, "collision", "projectile_pair_contact", payload, None);
            // detonation at the intercept point emits a 3× ShrapnelEmbedded
            // cluster on actors within blast radius (3 m / 96 px). Per
            // VAL-M14G-018 each fragment lands on `torso_front` and
            // `ActorWoundList[zone].shrapnel_count == 3`.
            if matches!(contact.outcome, cf_physics::ProjectilePairOutcome::FuzeTriggered) {
                const BLAST_RADIUS_PX: f32 = 96.0;
                const SHRAPNEL_COUNT: usize = 3;
                let center = contact.intercept_point;
                let nearby_actors: Vec<u64> = self
                    .state
                    .read()
                    .ok()
                    .map(|s| {
                        let mut hits = Vec::new();
                        if let Some(sim) = s.actor_state.as_ref() {
                            for (id, actor) in sim.world.actors.iter() {
                                let dx = actor.position.x - center[0];
                                let dy = actor.position.y - center[1];
                                if (dx * dx + dy * dy).sqrt() <= BLAST_RADIUS_PX {
                                    hits.push(id.0);
                                }
                            }
                        }
                        hits
                    })
                    .unwrap_or_default();
                let zone = cf_wound::registry::ZoneId::from("torso_front");
                let parent_id = Some(pair_event_id);
                for actor_id in nearby_actors {
                    for _ in 0..SHRAPNEL_COUNT {
                        let emit = cf_physics::classify_shrapnel(zone.clone(), 0.3, false);
                        let _ = self.m14g_emit_wound_created(
                            tick,
                            sim_time_ms,
                            actor_id,
                            emit,
                            parent_id.clone(),
                        );
                    }
                }
            }
        }
        let mut cram_engagements: Vec<u64> = Vec::new();
        for contact in &contacts {
            if contact.outcome != cf_physics::ProjectilePairOutcome::ApsIntercept {
                continue;
            }
            let aps_id = if contact.a_kind == cf_physics::ProjectileKind::ApsLaser {
                Some(contact.a_id)
            } else if contact.b_kind == cf_physics::ProjectileKind::ApsLaser {
                Some(contact.b_id)
            } else {
                None
            };
            if let Some(id) = aps_id {
                if let Some(snap) = pool_snapshot.iter().find(|p| p.id == id) {
                    cram_engagements.push(snap.owner_actor_id);
                }
            }
        }
        if let Ok(mut s) = self.state.write() {
            let mut consumed: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
            let mut velocity_overrides: BTreeMap<u64, [f32; 2]> = BTreeMap::new();
            for contact in &contacts {
                match contact.a_post_velocity {
                    Some(v) => {
                        velocity_overrides.insert(contact.a_id, v);
                    }
                    None => {
                        consumed.insert(contact.a_id);
                    }
                }
                match contact.b_post_velocity {
                    Some(v) => {
                        velocity_overrides.insert(contact.b_id, v);
                    }
                    None => {
                        consumed.insert(contact.b_id);
                    }
                }
            }
            s.m14d_projectile_pair_pool.retain(|p| !consumed.contains(&p.id));
            for p in s.m14d_projectile_pair_pool.iter_mut() {
                if let Some(v) = velocity_overrides.get(&p.id) {
                    p.velocity = *v;
                }
                p.position[0] += p.velocity[0] * tick_dt;
                p.position[1] += p.velocity[1] * tick_dt;
            }
            for owner in cram_engagements {
                let cram = s.m14d_cram_cooldowns.entry(owner).or_default();
                cram.engage_cooldown();
            }
        }
    }

}
