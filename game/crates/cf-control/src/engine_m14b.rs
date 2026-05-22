//! tick_m14b method.
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
    pub(crate) fn tick_m14b(&self, tick: Tick, sim_time_ms: f64) {
        let dt_secs = 1.0_f32 / self.config.tick_rate_hz.max(1) as f32;
        // -- Phase 1: advance per-tick world state under a write lock -------
        //    (DamagedGrav wave-front growth + decay transient apertures).
        if let Ok(mut state) = self.state.write() {
            cf_physics::advance_damaged_grav_wave_fronts(&mut state.m14b_gravity_overrides, dt_secs);
            // Tick down transient wind sources (e.g. pipe ruptures); remove
            // any whose ttl reached zero.
            let mut expired_ids: Vec<u32> = Vec::new();
            for (id, ttl) in state.m14b_transient_wind_ttl.iter_mut() {
                if *ttl <= 1 {
                    expired_ids.push(*id);
                } else {
                    *ttl -= 1;
                }
            }
            for expired_id in &expired_ids {
                state.m14b_transient_wind_ttl.remove(expired_id);
                state.m14b_wind_sources.retain(|w| w.id != *expired_id);
                // Clean up synthetic cells created for this transient.
                // Each rupture creates two cells with ids
                // (expired_id + 10_000) and (expired_id + 10_001).
                let high_cell = expired_id.saturating_add(10_000);
                let low_cell = expired_id.saturating_add(10_001);
                state.m14b_atmos_cells.retain(|c| c.id != high_cell && c.id != low_cell);
                state.m14b_transient_cells.retain(|c| *c != high_cell && *c != low_cell);
            }
        }

        // -- Phase 2: snapshot inputs under a read lock --------------------
        let (overrides, wind_sources, atmos_cells, base_field, actor_positions, projectile_positions, prev_active) = {
            let state = match self.state.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            let base_field = state
                .actor_state
                .as_ref()
                .map(|sim| cf_physics::GravityField::Uniform(sim.world.gravity))
                .unwrap_or_default();
            let actors: Vec<M14bActorSnapshot> = state
                .actor_state
                .as_ref()
                .map(|sim| {
                    sim.world
                        .actors
                        .values()
                        .map(|a| M14bActorSnapshot {
                            actor_id: a.id,
                            pos: [a.position.x, a.position.y],
                            mass: a.total_mass_cached.max(1.0),
                            half_extent_y: a.half_extents.y,
                            on_ground: a.on_ground,
                            velocity: [a.velocity.x, a.velocity.y],
                        })
                        .collect()
                })
                .unwrap_or_default();
            let projectiles: Vec<(u64, [f32; 2])> = state
                .actor_state
                .as_ref()
                .map(|sim| {
                    sim.projectiles
                        .iter()
                        .map(|p| (p.id, [p.position.x, p.position.y]))
                        .collect()
                })
                .unwrap_or_default();
            (
                state.m14b_gravity_overrides.clone(),
                state.m14b_wind_sources.clone(),
                state.m14b_atmos_cells.clone(),
                base_field,
                actors,
                projectiles,
                state.m14b_active_overrides.clone(),
            )
        };

        let mut events_to_emit: Vec<(&'static str, &'static str, serde_json::Value)> = Vec::new();
        let mut banners_to_push: Vec<crate::state::HudBannerView> = Vec::new();
        let mut new_active: BTreeMap<ActorId, std::collections::BTreeSet<u32>> = BTreeMap::new();
        let mut velocity_deltas: BTreeMap<ActorId, [f32; 2]> = BTreeMap::new();
        let mut angular_deltas: BTreeMap<ActorId, f32> = BTreeMap::new();
        let mut projectile_deltas: BTreeMap<u64, [f32; 2]> = BTreeMap::new();
        // Local-g in m/s² (scale-aware): scenarios may author gravity in
        // pixel units (~980) or SI (~9.81); buoyancy + stratification
        // need SI so we down-scale when magnitude looks pixel-scale.
        let base_g_mag = base_field.sample([0.0, 0.0]).magnitude;
        let local_g_m_s2 = if base_g_mag > 50.0 {
            base_g_mag / 100.0
        } else {
            base_g_mag
        };

        if !overrides.is_empty() || !wind_sources.is_empty() {
            for snap in &actor_positions {
                let actor_id = &snap.actor_id;
                let pos = &snap.pos;
                let mass = &snap.mass;
                let half_extent_y = &snap.half_extent_y;
                let on_ground = &snap.on_ground;
                let velocity = &snap.velocity;
                // -- Gravity override sampling -------------------------------
                let base_vec = base_field.sample(*pos);
                let result = cf_physics::apply_overrides(base_vec, *pos, Some(actor_id.0), &overrides);
                let active_set: std::collections::BTreeSet<u32> = result.active_ids.iter().copied().collect();
                let prev_set = prev_active.get(actor_id).cloned().unwrap_or_default();
                for id in active_set.difference(&prev_set) {
                    if let Some(ovr) = overrides.iter().find(|o| o.id() == *id) {
                        let payload = serde_json::json!({
                            "actor_id": actor_id.0,
                            "override_id": *id,
                            "kind": ovr.kind_str(),
                            "magnitude": result.gravity.magnitude,
                            "direction": result.gravity.direction,
                        });
                        events_to_emit.push(("gravity", "override_activated", payload));
                        // MAGNETIC ANCHOR HUD banner when a magnetic_boot
                        // override engages for this actor (spec acceptance
                        // criterion).
                        if matches!(ovr, cf_physics::GravityOverride::MagneticBoots { .. }) {
                            banners_to_push.push(crate::state::HudBannerView {
                                id: format!("magnetic_anchor.actor.{}", actor_id.0),
                                label: "MAGNETIC ANCHOR".to_string(),
                                severity: "info".to_string(),
                                raised_at_tick: tick.0,
                                expires_at_tick: Some(tick.0 + 120),
                                accessibility_id: format!("hud.banner.magnetic_anchor.{}", actor_id.0),
                            });
                        }
                    }
                }
                for id in prev_set.difference(&active_set) {
                    let kind = overrides
                        .iter()
                        .find(|o| o.id() == *id)
                        .map(|o| o.kind_str())
                        .unwrap_or("unknown");
                    let payload = serde_json::json!({
                        "actor_id": actor_id.0,
                        "override_id": *id,
                        "kind": kind,
                    });
                    events_to_emit.push(("gravity", "override_deactivated", payload));
                }
                new_active.insert(*actor_id, active_set);

                // -- Gravity Δv correction on actor velocity ----------------
                // Apply correction so the effective per-tick acceleration
                // on the actor is `override_g` instead of `base_g`. The
                // base gravity is already applied by cf_actor::sim::step;
                // we add (override - base) × dt.
                let base_g_x = base_vec.direction[0] * base_vec.magnitude;
                let base_g_y = base_vec.direction[1] * base_vec.magnitude;
                let over_g_x = result.gravity.direction[0] * result.gravity.magnitude;
                let over_g_y = result.gravity.direction[1] * result.gravity.magnitude;
                let correction_x = over_g_x - base_g_x;
                let correction_y = over_g_y - base_g_y;
                let base_g_y_abs = base_g_y.abs();
                // X correction always applies (well pull is tangent-safe
                // on ground per spec acceptance scenario).
                let apply_x = correction_x.abs() > 1e-3;
                // Y correction applies when actor is airborne OR the
                // correction is strong enough to lift the actor off
                // ground (reverse-g, gravity well above). Skipped when
                // grounded + correction is downward (floor handles it)
                // OR low-g (correction is small upward but actor stays
                // resting on floor — no sub-pixel bouncing artifact).
                let apply_y = !on_ground || correction_y.abs() > base_g_y_abs;
                let dvx = if apply_x { correction_x * dt_secs } else { 0.0 };
                let dvy = if apply_y { correction_y * dt_secs } else { 0.0 };
                if dvx.abs() > 1e-6 || dvy.abs() > 1e-6 {
                    let entry = velocity_deltas.entry(*actor_id).or_insert([0.0, 0.0]);
                    entry[0] += dvx;
                    entry[1] += dvy;
                }
                // velocity is captured for downstream use (mass impulse).
                let _ = velocity;

                // -- Wind force sampling (with chimney/buoyancy) ------------
                let wind_outcome =
                    cf_atmos::wind_force_with_buoyancy_at(*pos, &atmos_cells, &wind_sources, local_g_m_s2);
                let mag = wind_outcome.magnitude_sq.sqrt();
                if mag >= 1.0 {
                    let wdvx = wind_outcome.force_n[0] / mass.max(1.0) * dt_secs;
                    let wdvy = wind_outcome.force_n[1] / mass.max(1.0) * dt_secs;
                    let entry = velocity_deltas.entry(*actor_id).or_insert([0.0, 0.0]);
                    entry[0] += wdvx;
                    entry[1] += wdvy;
                    // -- Off-center hit → angular impulse ------------------
                    // Spec: "the actor receives lateral impulse via
                    // cf-physics::angular_impulse_from_offcenter_hit".
                    // The wind acts at the actor's centre-of-pressure
                    // (`half_extent_y / 2` above CG); 2D cross product
                    // gives the angular Δv.
                    let hit_offset = [0.0_f32, half_extent_y * 0.5];
                    let impulse_n = [wind_outcome.force_n[0] * dt_secs, wind_outcome.force_n[1] * dt_secs];
                    let moi = (mass * half_extent_y * half_extent_y).max(1.0);
                    let d_omega = cf_actor::angular_impulse_from_offcenter_hit(hit_offset, impulse_n, moi);
                    if d_omega.abs() > 1e-6 {
                        *angular_deltas.entry(*actor_id).or_insert(0.0) += d_omega;
                    }
                    let payload = serde_json::json!({
                        "actor_id": actor_id.0,
                        "force_n": wind_outcome.force_n,
                        "source_aperture_id": wind_outcome.source_aperture_id,
                        "magnitude_n": mag,
                    });
                    events_to_emit.push(("atmos", "wind_force_applied", payload));
                }
            }
            // -- Projectile gravity-override sampling -----------------------
            // Spec player-facing: "Gravity well anomalies bend walk paths
            // and projectile trajectories visibly per cell."
            for (pid, ppos) in &projectile_positions {
                let base_vec = base_field.sample(*ppos);
                let result = cf_physics::apply_overrides(base_vec, *ppos, None, &overrides);
                if result.active_ids.is_empty() {
                    continue;
                }
                let base_g_x = base_vec.direction[0] * base_vec.magnitude;
                let base_g_y = base_vec.direction[1] * base_vec.magnitude;
                let over_g_x = result.gravity.direction[0] * result.gravity.magnitude;
                let over_g_y = result.gravity.direction[1] * result.gravity.magnitude;
                let dvx = (over_g_x - base_g_x) * dt_secs;
                let dvy = (over_g_y - base_g_y) * dt_secs;
                if dvx.abs() > 1e-6 || dvy.abs() > 1e-6 {
                    projectile_deltas.insert(*pid, [dvx, dvy]);
                }
            }
        }

        // -- Stratification step (every 4th tick) ----------------------------
        let mut strat_deltas: Vec<cf_atmos::StratificationDelta> = Vec::new();
        if tick.0.is_multiple_of(4) {
            if let Ok(mut state) = self.state.write() {
                strat_deltas = cf_atmos::stratify(&mut state.m14b_strat_cells, local_g_m_s2);
            }
            for delta in &strat_deltas {
                let payload = serde_json::json!({
                    "cell_id": delta.cell_id,
                    "gas": delta.gas.label(),
                    "fraction_delta": delta.fraction_delta,
                });
                events_to_emit.push(("atmos", "gas_stratified", payload));
            }
        }

        // -- Commit state mutations ----------------------------------------
        if let Ok(mut state) = self.state.write() {
            state.m14b_active_overrides = new_active;
            for banner in banners_to_push {
                push_banner_dedup(&mut state.hud_banners, banner);
            }
            if let Some(sim) = state.actor_state.as_mut() {
                for (actor_id, dv) in &velocity_deltas {
                    if let Some(actor) = sim.world.actors.get_mut(actor_id) {
                        actor.velocity.x += dv[0];
                        actor.velocity.y += dv[1];
                    }
                }
                for (actor_id, d_omega) in &angular_deltas {
                    if let Some(actor) = sim.world.actors.get_mut(actor_id) {
                        actor.attitude.angular_vel += *d_omega;
                    }
                }
                for (pid, dv) in &projectile_deltas {
                    if let Some(p) = sim.projectiles.iter_mut().find(|p| p.id == *pid) {
                        p.velocity.x += dv[0];
                        p.velocity.y += dv[1];
                    }
                }
            }
        }

        // -- Emit events ----------------------------------------------------
        for (cat, ty, payload) in events_to_emit {
            self.recorder.record(tick, sim_time_ms, cat, ty, payload, None);
        }
    }

}
