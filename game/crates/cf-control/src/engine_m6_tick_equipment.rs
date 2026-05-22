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
}
