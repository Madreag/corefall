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

    /// derivation + per-tool bipod auto-stow. Emits
    /// `equipment.grenade_detonated`, `combat.knife_throw_landed`, and
    /// `actor.facing_changed` events. Called from `drive_tick` after
    /// `tick_m6_actor_state`.

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
            /// resolve pass to emit `audio.spatial_resolved` etc. for
            /// each footstep cue.
            position: [f32; 2],
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
