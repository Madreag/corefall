//! **M14J** § "actor advanced mobility" — engine-side dispatch.
//!
//! Owns the 9 cfctl methods + per-tick rope/zipline/mount integration:
//!  - `act.player.vault`
//!  - `act.player.wall_jump`
//!  - `act.player.fire_grapple { target_x, target_y }`
//!  - `act.player.rope_input { climb, swing }`
//!  - `act.player.release_rope`
//!  - `act.player.zipline_clip { line_id }`
//!  - `act.player.zipline_brake { engaged }`
//!  - `act.player.mount { critter_id }`
//!  - `act.player.dismount`
//!
//! Each dispatch records the M14J replay event + a `control.command_accepted`
//! or `control.command_rejected` envelope so the run bundle stays auditable.

use std::collections::BTreeMap;

use serde_json::json;

use cf_actor::{
    mount::{resolve_dismount, MountState, MOUNT_TOP_SPEED_RETAINED},
    parkour::{
        wall_jump_velocity_delta, MAX_CHAINED_WALL_JUMPS, VAULT_DURATION_MS, VAULT_MAX_OBSTACLE_HEIGHT_M,
        WALL_JUMP_DURATION_MS,
    },
    ActorId,
};
use cf_equipment::{
    fire_grapple, GrappleFireOutcome, GRAPPLE_LONG_DISTANCE_M, ROPE_CLIMB_SPEED_M_PER_S,
    ROPE_RAPPEL_SPEED_M_PER_S,
};
use cf_physics::{rope::pendulum_release_velocity, Rope, RopeEndpoint, RopeId, DEFAULT_SEGMENT_COUNT};
use cf_sim_core::Tick;

use crate::engine::M0Engine;
use crate::server::CommandResult;

impl M0Engine {
    /// Reject + record helper for M14J commands.
    fn m14j_reject(&self, method: &str, reason: &str, tick: Tick, sim_time_ms: f64) -> CommandResult {
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_rejected",
            json!({"method": method, "reason": reason}),
            None,
        );
        CommandResult::rejected(reason, tick.0)
    }

    /// `act.player.vault` — manual vault override. Triggers the 200ms
    /// Vault cinematic if the actor is on the ground and the parkour
    /// signal does not currently report a vault-too-tall situation.
    pub(crate) fn dispatch_m14j_vault(&self, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.vault", "no_player_actor", tick, sim_time_ms),
        };
        let mut obstacle_height = 0.0f32;
        let mut updated = false;
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                    // Use the cached parkour signal height when present;
                    // otherwise fall back to the spec max (1.2m) for the
                    // manual override path.
                    let h = actor
                        .parkour_signal
                        .vault_candidate
                        .map(|c| c.height_m)
                        .unwrap_or(VAULT_MAX_OBSTACLE_HEIGHT_M);
                    actor.parkour_signal.vault_ticks_remaining_ms = VAULT_DURATION_MS;
                    obstacle_height = h;
                    updated = true;
                }
            }
        }
        if !updated {
            return self.m14j_reject("act.player.vault", "vault_unavailable", tick, sim_time_ms);
        }
        let accepted_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.vault", "actor": player_id.0}),
            None,
        );
        self.recorder.record(
            tick,
            sim_time_ms,
            "actor",
            "vaulted",
            json!({
                "actor": player_id.0,
                "tick": tick.0,
                "obstacle_height": obstacle_height,
                "duration_ms": VAULT_DURATION_MS,
            }),
            Some(accepted_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// `act.player.wall_jump` — perpendicular impulse off a vertical surface.
    pub(crate) fn dispatch_m14j_wall_jump(&self, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.wall_jump", "no_player_actor", tick, sim_time_ms),
        };
        let mut chain_index = 0u32;
        let mut wall_normal_sign = 0.0f32;
        let jump_impulse = self.settings_jump_impulse_signed().abs();
        let mut accepted = false;
        let mut reject_reason: Option<&'static str> = None;
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                    // Chain limit per spec: 3 chained wall-jumps max
                    // before touching ground.
                    if actor.parkour_signal.chained_wall_jumps_since_ground >= MAX_CHAINED_WALL_JUMPS {
                        reject_reason = Some("wall_jump_chain_exhausted");
                    } else {
                        // Use the cached wall candidate if available,
                        // otherwise fall back to the actor's facing
                        // direction for the perpendicular kick.
                        let (sign_opt, wall_present) = match actor.parkour_signal.wall_candidate {
                            Some(w) => (w.normal_sign, true),
                            None => (-actor.facing.sign(), false),
                        };
                        if !wall_present && actor.on_ground {
                            // Need either a wall or airborne with no ground
                            // for wall jump.
                            reject_reason = Some("no_wall_in_contact");
                        } else {
                            wall_normal_sign = sign_opt;
                            chain_index = actor.parkour_signal.chained_wall_jumps_since_ground;
                            let dv = wall_jump_velocity_delta(
                                [actor.velocity.x, actor.velocity.y],
                                sign_opt,
                                jump_impulse,
                            );
                            actor.velocity.x += dv[0];
                            actor.velocity.y += dv[1];
                            actor.parkour_signal.chained_wall_jumps_since_ground += 1;
                            actor.parkour_signal.wall_jump_ticks_remaining_ms = WALL_JUMP_DURATION_MS;
                            actor.wall_jump_ticks_remaining_ms = WALL_JUMP_DURATION_MS;
                            // Spec: stance returns to airborne immediately
                            // after the cinematic; flag actor off ground.
                            actor.on_ground = false;
                            accepted = true;
                        }
                    }
                } else {
                    reject_reason = Some("actor_missing");
                }
            } else {
                reject_reason = Some("no_actor_world");
            }
        }
        if !accepted {
            return self.m14j_reject(
                "act.player.wall_jump",
                reject_reason.unwrap_or("wall_jump_unavailable"),
                tick,
                sim_time_ms,
            );
        }
        let accepted_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.wall_jump", "actor": player_id.0}),
            None,
        );
        self.recorder.record(
            tick,
            sim_time_ms,
            "actor",
            "wall_jumped",
            json!({
                "actor": player_id.0,
                "tick": tick.0,
                "chain_index": chain_index,
                "wall_normal_sign": wall_normal_sign,
                "impulse_perpendicular": jump_impulse * cf_actor::parkour::WALL_JUMP_PERPENDICULAR_FRACTION,
            }),
            Some(accepted_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// `act.player.fire_grapple { target_x, target_y }` — fire grapple gun.
    /// Returns Embedded outcome through the `grapple.fired` + `grapple.embedded`
    /// replay events. Spawns a verlet rope in `m14j_ropes`.
    pub(crate) fn dispatch_m14j_fire_grapple(
        &self,
        target_x: f32,
        target_y: f32,
        tick: Tick,
        sim_time_ms: f64,
    ) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.fire_grapple", "no_player_actor", tick, sim_time_ms),
        };
        let mut origin = [0.0f32, 0.0f32];
        if let Ok(state) = self.state.read() {
            if let Some(sim) = state.actor_state.as_ref() {
                if let Some(actor) = sim.world.actors.get(&player_id) {
                    origin = [actor.position.x, actor.position.y];
                }
            }
        }
        // For grapple landing predicate we treat any finite world target
        // as anchorable at M14J baseline (the chunked terrain anchor
        // predicate plugs in later via cf-terrain).
        let outcome = fire_grapple(origin, [target_x, target_y], |_x, _y| true);
        let fired_id = self.recorder.record(
            tick,
            sim_time_ms,
            "grapple",
            "fired",
            json!({
                "actor": player_id.0,
                "tick": tick.0,
                "target_x": target_x,
                "target_y": target_y,
                "range_m": ((target_x - origin[0]).powi(2) + (target_y - origin[1]).powi(2)).sqrt(),
            }),
            None,
        );
        match outcome {
            GrappleFireOutcome::Missed { reason } => {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "act.player.fire_grapple", "reason": reason}),
                    Some(fired_id),
                );
                return CommandResult::rejected(reason, tick.0);
            }
            GrappleFireOutcome::Embedded {
                anchor,
                rope_length_m,
                long_distance,
            } => {
                let rope_id = if let Ok(mut state) = self.state.write() {
                    let id = RopeId(state.m14j_next_rope_id);
                    state.m14j_next_rope_id += 1;
                    let segments = DEFAULT_SEGMENT_COUNT as u32;
                    // **Audit finding #2 fix**: use `new_with_positions` so
                    // the actor-end node initializes to the player's live
                    // position rather than world origin (the legacy
                    // `Rope::new` falls back to `[0,0]` for Actor endpoints
                    // because no actor lookup is available at construction).
                    let mut rope = Rope::new_with_positions(
                        id,
                        RopeEndpoint::Anchored { position: anchor },
                        RopeEndpoint::Actor {
                            actor_id: player_id.0,
                            offset: [0.0, 0.0],
                        },
                        anchor,
                        origin,
                        segments,
                        [0.0, -9.81],
                    );
                    rope.embedded = true;
                    rope.taut = true;
                    rope.segment_length_m = (rope_length_m / segments as f32).max(0.05);
                    state.m14j_ropes.insert(id, rope);
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                            actor.holding_rope = Some(id);
                        }
                    }
                    id
                } else {
                    RopeId(0)
                };
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "grapple",
                    "embedded",
                    json!({
                        "actor": player_id.0,
                        "tick": tick.0,
                        "anchor_x": anchor[0],
                        "anchor_y": anchor[1],
                        "rope_id": rope_id.raw(),
                        "rope_length_m": rope_length_m,
                        "long_distance": long_distance,
                    }),
                    Some(fired_id.clone()),
                );
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_accepted",
                    json!({"method": "act.player.fire_grapple", "actor": player_id.0, "long_distance": long_distance}),
                    Some(fired_id.clone()),
                );
                // Storyteller: long-distance grapple shots ( >= 25 m )
                // register as M25 story-grade events per spec.
                if long_distance {
                    self.recorder.record(
                        tick,
                        sim_time_ms,
                        "storyteller",
                        "grapple_long_distance_shot",
                        json!({
                            "actor": player_id.0,
                            "tick": tick.0,
                            "range_m": rope_length_m,
                            "threshold_m": GRAPPLE_LONG_DISTANCE_M,
                        }),
                        Some(fired_id),
                    );
                }
                CommandResult::accepted(tick.0)
            }
        }
    }

    /// `act.player.rope_input { climb, swing }`. Drives rope-climb /
    /// rappel + tangential swing. Spec defines climb at 0.8 m/s and
    /// rappel at 1.5 m/s.
    pub(crate) fn dispatch_m14j_rope_input(&self, climb: f32, swing: f32, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.rope_input", "no_player_actor", tick, sim_time_ms),
        };
        let mut applied = false;
        let mut rope_id: u64 = 0;
        if let Ok(mut state) = self.state.write() {
            let opt_rope_id = state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&player_id))
                .and_then(|a| a.holding_rope);
            if let Some(rid) = opt_rope_id {
                rope_id = rid.raw();
                // Snapshot the anchor position first (immutable borrow), then
                // apply velocity (mutable borrow) — avoid mixing.
                let anchor_opt = state
                    .m14j_ropes
                    .get(&rid)
                    .and_then(|r| r.nodes.first().map(|n| n.position));
                if let Some(sim) = state.actor_state.as_mut() {
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        // Climb axis: positive = pull toward anchor, negative = rappel.
                        let speed = if climb > 0.0 {
                            climb * ROPE_CLIMB_SPEED_M_PER_S
                        } else {
                            climb * ROPE_RAPPEL_SPEED_M_PER_S
                        };
                        let anchor = anchor_opt.unwrap_or([actor.position.x, actor.position.y]);
                        let dx = anchor[0] - actor.position.x;
                        let dy = anchor[1] - actor.position.y;
                        let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                        let unit = [dx / len, dy / len];
                        actor.velocity.x = unit[0] * speed;
                        actor.velocity.y = unit[1] * speed;
                        // Swing input adds tangential velocity at the bob.
                        let tangent = [-unit[1], unit[0]];
                        actor.velocity.x += tangent[0] * swing * 1.5;
                        actor.velocity.y += tangent[1] * swing * 1.5;
                        applied = true;
                    }
                }
            }
        }
        if !applied {
            return self.m14j_reject("act.player.rope_input", "not_on_rope", tick, sim_time_ms);
        }
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.rope_input", "actor": player_id.0, "rope_id": rope_id, "climb": climb, "swing": swing}),
            None,
        );
        CommandResult::accepted(tick.0)
    }

    /// `act.player.release_rope` — release the embedded grapple rope.
    /// Compute pendulum exit velocity at apex and apply to the actor;
    /// fire `rope.released`.
    pub(crate) fn dispatch_m14j_release_rope(&self, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.release_rope", "no_player_actor", tick, sim_time_ms),
        };
        let mut released = false;
        let mut rope_id_out: u64 = 0;
        let mut exit = [0.0f32, 0.0f32];
        let mut theta_at_release = 0.0f32;
        if let Ok(mut state) = self.state.write() {
            let rid_opt = state
                .actor_state
                .as_ref()
                .and_then(|sim| sim.world.actors.get(&player_id))
                .and_then(|a| a.holding_rope);
            if let Some(rid) = rid_opt {
                if let Some(rope) = state.m14j_ropes.remove(&rid) {
                    // Compute pendulum exit velocity per spec
                    // v = sqrt(2 g L (1 - cos(theta)))
                    let bob_rel = rope.bob_relative_to_anchor();
                    let length = rope.total_length_m().max(0.1);
                    // theta = angle from vertical (the rope's anchor-down
                    // axis is +y down in our world; clamp small lengths).
                    let theta = (bob_rel[0] / length).asin();
                    let v = pendulum_release_velocity(length, theta, 9.81);
                    exit = v;
                    theta_at_release = theta;
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                            actor.velocity.x = v[0];
                            actor.velocity.y = v[1];
                            actor.holding_rope = None;
                            actor.on_ground = false;
                        }
                    }
                    rope_id_out = rid.raw();
                    released = true;
                }
            }
        }
        if !released {
            return self.m14j_reject("act.player.release_rope", "not_on_rope", tick, sim_time_ms);
        }
        let accepted_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.release_rope", "actor": player_id.0, "rope_id": rope_id_out}),
            None,
        );
        self.recorder.record(
            tick,
            sim_time_ms,
            "rope",
            "released",
            json!({
                "actor": player_id.0,
                "tick": tick.0,
                "rope_id": rope_id_out,
                "exit_velocity_x": exit[0],
                "exit_velocity_y": exit[1],
                "theta_rad_at_release": theta_at_release,
            }),
            Some(accepted_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// `act.player.zipline_clip { line_id }` — clip onto a deployed zip line.
    pub(crate) fn dispatch_m14j_zipline_clip(&self, line_id: u64, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.zipline_clip", "no_player_actor", tick, sim_time_ms),
        };
        let mut accepted = false;
        let mut reject_reason: Option<&'static str> = None;
        if let Ok(mut state) = self.state.write() {
            let rid = RopeId(line_id);
            if !state.m14j_zipline_ropes.contains(&rid) || !state.m14j_ropes.contains_key(&rid) {
                reject_reason = Some("unknown_zipline_id");
            } else if let Some(sim) = state.actor_state.as_mut() {
                if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                    actor.zipline_attached = Some(rid);
                    actor.zipline_brake_engaged = false;
                    state.m14j_zipline_speed_by_rider.insert(player_id.0, 0.0);
                    accepted = true;
                }
            } else {
                reject_reason = Some("no_actor_world");
            }
        }
        if !accepted {
            return self.m14j_reject(
                "act.player.zipline_clip",
                reject_reason.unwrap_or("zipline_clip_failed"),
                tick,
                sim_time_ms,
            );
        }
        let accepted_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.zipline_clip", "actor": player_id.0, "line_id": line_id}),
            None,
        );
        self.recorder.record(
            tick,
            sim_time_ms,
            "zipline",
            "clipped",
            json!({
                "actor": player_id.0,
                "tick": tick.0,
                "rope_id": line_id,
                "from_end": "high",
            }),
            Some(accepted_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// `act.player.zipline_brake { engaged }`.
    pub(crate) fn dispatch_m14j_zipline_brake(&self, engaged: bool, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.zipline_brake", "no_player_actor", tick, sim_time_ms),
        };
        let mut applied = false;
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                    if actor.zipline_attached.is_some() {
                        actor.zipline_brake_engaged = engaged;
                        applied = true;
                    }
                }
            }
        }
        if !applied {
            return self.m14j_reject("act.player.zipline_brake", "not_on_zipline", tick, sim_time_ms);
        }
        self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.zipline_brake", "actor": player_id.0, "engaged": engaged}),
            None,
        );
        CommandResult::accepted(tick.0)
    }

    /// `act.player.mount { critter_id }` — mount a saddled critter.
    pub(crate) fn dispatch_m14j_mount(&self, critter_id: u64, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.mount", "no_player_actor", tick, sim_time_ms),
        };
        let mut combined_mass = 0.0f32;
        let mut applied = false;
        let mut reason: Option<&'static str> = None;
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                let critter_present = sim.world.actors.contains_key(&ActorId(critter_id));
                if !critter_present {
                    reason = Some("unknown_critter");
                } else {
                    let critter_mass = sim
                        .world
                        .actors
                        .get(&ActorId(critter_id))
                        .map(|a| a.mass_kg)
                        .unwrap_or(0.0);
                    if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                        if actor.mount.is_some() {
                            reason = Some("already_mounted");
                        } else {
                            let rider_mass = actor.mass_kg;
                            let m = MountState::new(ActorId(critter_id), rider_mass, critter_mass);
                            combined_mass = m.combined_mass_kg;
                            actor.mount = Some(m);
                            applied = true;
                        }
                    }
                    if applied {
                        if let Some(critter) = sim.world.actors.get_mut(&ActorId(critter_id)) {
                            critter.is_being_ridden = true;
                        }
                    }
                }
            }
        }
        if !applied {
            return self.m14j_reject(
                "act.player.mount",
                reason.unwrap_or("mount_failed"),
                tick,
                sim_time_ms,
            );
        }
        let accepted_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.mount", "actor": player_id.0, "critter_id": critter_id}),
            None,
        );
        self.recorder.record(
            tick,
            sim_time_ms,
            "actor",
            "mounted",
            json!({
                "actor": player_id.0,
                "tick": tick.0,
                "critter_id": critter_id,
                "combined_mass_kg": combined_mass,
                "mount_speed_retained": MOUNT_TOP_SPEED_RETAINED,
            }),
            Some(accepted_id),
        );
        CommandResult::accepted(tick.0)
    }

    /// `act.player.dismount` — dismount from a critter. Mid-motion staggers.
    pub(crate) fn dispatch_m14j_dismount(&self, tick: Tick, sim_time_ms: f64) -> CommandResult {
        let player_id = match self.player_actor_id() {
            Some(id) => id,
            None => return self.m14j_reject("act.player.dismount", "no_player_actor", tick, sim_time_ms),
        };
        let mut critter_id: u64 = 0;
        let mut critter_vel = [0.0f32, 0.0f32];
        let mut mounted = false;
        if let Ok(state) = self.state.read() {
            if let Some(sim) = state.actor_state.as_ref() {
                if let Some(actor) = sim.world.actors.get(&player_id) {
                    if let Some(m) = actor.mount {
                        critter_id = m.critter_id.0;
                        if let Some(crit) = sim.world.actors.get(&m.critter_id) {
                            critter_vel = [crit.velocity.x, crit.velocity.y];
                        }
                        mounted = true;
                    }
                }
            }
        }
        if !mounted {
            return self.m14j_reject("act.player.dismount", "not_mounted", tick, sim_time_ms);
        }
        let outcome = resolve_dismount(critter_vel);
        if let Ok(mut state) = self.state.write() {
            if let Some(sim) = state.actor_state.as_mut() {
                if let Some(actor) = sim.world.actors.get_mut(&player_id) {
                    actor.mount = None;
                    actor.velocity.x = outcome.inherited_velocity[0];
                    actor.velocity.y = outcome.inherited_velocity[1];
                    if outcome.mid_motion {
                        actor.knockdown_ticks_remaining =
                            (outcome.stagger_ms / (1000 / self.config.tick_rate_hz.max(1))).max(1);
                    }
                }
                if let Some(critter) = sim.world.actors.get_mut(&ActorId(critter_id)) {
                    critter.is_being_ridden = false;
                }
            }
        }
        let accepted_id = self.recorder.record(
            tick,
            sim_time_ms,
            "control",
            "command_accepted",
            json!({"method": "act.player.dismount", "actor": player_id.0, "critter_id": critter_id}),
            None,
        );
        let dismount_event_id = self.recorder.record(
            tick,
            sim_time_ms,
            "actor",
            "dismounted",
            json!({
                "actor": player_id.0,
                "tick": tick.0,
                "critter_id": critter_id,
                "mid_motion": outcome.mid_motion,
                "inherited_velocity_x": outcome.inherited_velocity[0],
                "inherited_velocity_y": outcome.inherited_velocity[1],
                "stagger_ms": outcome.stagger_ms,
            }),
            Some(accepted_id),
        );
        // story-grade narrative bullet per spec § "Storyteller hooks".
        if outcome.mid_motion {
            self.recorder.record(
                tick,
                sim_time_ms,
                "storyteller",
                "actor_dismounted_mid_gallop",
                json!({
                    "actor": player_id.0,
                    "tick": tick.0,
                    "critter_id": critter_id,
                    "inherited_velocity_x": outcome.inherited_velocity[0],
                    "inherited_velocity_y": outcome.inherited_velocity[1],
                }),
                Some(dismount_event_id),
            );
        }
        CommandResult::accepted(tick.0)
    }

    /// Player-actor id helper; returns `None` when no actor world is loaded.
    fn player_actor_id(&self) -> Option<ActorId> {
        self.state.read().ok().and_then(|s| s.player_actor)
    }

    /// engine's sim world. Clones the actor so tests can inspect any
    /// field without holding the engine lock.
    pub fn m14j_actor_clone(&self, actor_id: u64) -> Option<cf_actor::ActorState> {
        let s = self.state.read().ok()?;
        let sim = s.actor_state.as_ref()?;
        sim.world.actors.get(&ActorId(actor_id)).cloned()
    }

    pub fn m14j_player_id(&self) -> Option<u64> {
        let s = self.state.read().ok()?;
        s.player_actor.map(|p| p.0)
    }

    pub fn m14j_with_player_actor_mut<R>(&self, f: impl FnOnce(&mut cf_actor::ActorState) -> R) -> Option<R> {
        let mut s = self.state.write().ok()?;
        let player_id = s.player_actor?;
        let sim = s.actor_state.as_mut()?;
        let actor = sim.world.actors.get_mut(&player_id)?;
        Some(f(actor))
    }

    pub fn m14j_with_actor_mut<R>(&self, actor_id: u64, f: impl FnOnce(&mut cf_actor::ActorState) -> R) -> Option<R> {
        let mut s = self.state.write().ok()?;
        let sim = s.actor_state.as_mut()?;
        let actor = sim.world.actors.get_mut(&ActorId(actor_id))?;
        Some(f(actor))
    }

    pub fn m14j_rope_count(&self) -> usize {
        self.state.read().map(|s| s.m14j_ropes.len()).unwrap_or(0)
    }

    pub fn m14j_zipline_count(&self) -> usize {
        self.state.read().map(|s| s.m14j_zipline_ropes.len()).unwrap_or(0)
    }

    /// M14A's `JUMP_IMPULSE` constant" — read jump_impulse from settings.
    fn settings_jump_impulse_signed(&self) -> f32 {
        self.state
            .read()
            .ok()
            .map(|s| s.settings.jump_force)
            .unwrap_or(420.0)
    }

    /// Called by content / cfctl / scenario setup. Returns the new rope id.
    pub fn m14j_deploy_zipline(
        &self,
        actor_id: u64,
        anchor_a: [f32; 2],
        anchor_b: [f32; 2],
        tick: Tick,
        sim_time_ms: f64,
    ) -> Option<RopeId> {
        let outcome = cf_equipment::deploy_zip_kit(anchor_a, anchor_b);
        match outcome {
            cf_equipment::ZipKitDeployOutcome::Rejected { reason } => {
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "control",
                    "command_rejected",
                    json!({"method": "zip_kit.deploy", "reason": reason}),
                    None,
                );
                None
            }
            cf_equipment::ZipKitDeployOutcome::Deployed {
                high_end,
                low_end,
                span_m,
                height_delta_m,
            } => {
                let rid = if let Ok(mut state) = self.state.write() {
                    let id = RopeId(state.m14j_next_rope_id);
                    state.m14j_next_rope_id += 1;
                    let segments = DEFAULT_SEGMENT_COUNT as u32;
                    let mut rope = Rope::new(
                        id,
                        RopeEndpoint::Anchored { position: high_end },
                        RopeEndpoint::Anchored { position: low_end },
                        segments,
                        [0.0, -9.81],
                    );
                    rope.taut = true;
                    state.m14j_ropes.insert(id, rope);
                    state.m14j_zipline_ropes.insert(id);
                    id
                } else {
                    return None;
                };
                self.recorder.record(
                    tick,
                    sim_time_ms,
                    "zipline",
                    "deployed",
                    json!({
                        "actor": actor_id,
                        "tick": tick.0,
                        "rope_id": rid.raw(),
                        "high_end_x": high_end[0],
                        "high_end_y": high_end[1],
                        "low_end_x": low_end[0],
                        "low_end_y": low_end[1],
                        "span_m": span_m,
                        "height_delta_m": height_delta_m,
                    }),
                    None,
                );
                Some(rid)
            }
        }
    }

    /// per-rope nodes follow gravity + constraint relaxation; ziplines also
    /// advance their riders along the cable per
    /// [`cf_equipment::zipline_step_speed`].
    pub fn m14j_tick(&self, _tick: Tick, _sim_time_ms: f64) {
        let tick_rate_hz = self.config.tick_rate_hz.max(1) as f32;
        let dt = 1.0 / tick_rate_hz;
        // chest-high / vertical surfaces"** — populate parkour candidates
        // BEFORE the per-actor M14J pass so auto-vault has a candidate to
        // commit on. Uses chunked terrain when present; otherwise candidates
        // come from external setters (manual test paths).
        if let Ok(mut state) = self.state.write() {
            if state.chunked_terrain.is_some() {
                let actor_ids: Vec<u64> = state
                    .actor_state
                    .as_ref()
                    .map(|sim| sim.world.actors.keys().map(|id| id.0).collect())
                    .unwrap_or_default();
                for actor_id_u in &actor_ids {
                    // Take a snapshot of the chunked terrain reference for predicate.
                    // We need to read terrain AND mutate actor — release the
                    // borrow each iteration.
                    let actor_id = ActorId(*actor_id_u);
                    let (pos, half, facing, vx, on_ground) = {
                        let Some(sim) = state.actor_state.as_ref() else {
                            continue;
                        };
                        let Some(actor) = sim.world.actors.get(&actor_id) else {
                            continue;
                        };
                        (
                            [actor.position.x, actor.position.y],
                            [actor.half_extents.x, actor.half_extents.y],
                            actor.facing,
                            actor.velocity.x,
                            actor.on_ground,
                        )
                    };
                    // Run the swept-volume predicate against chunked terrain.
                    let (vault_cand, wall_cand) = {
                        let Some(terrain) = state.chunked_terrain.as_ref() else {
                            continue;
                        };
                        let is_solid = |x: f32, y: f32| -> bool {
                            // Treat (x, y) as a 1×1 pixel test for solidity.
                            terrain.aabb_overlaps_solid([x, y], [x + 1.0, y + 1.0])
                        };
                        let vc = cf_actor::detect_vault(pos, half, facing, vx, &is_solid);
                        let wc = if !on_ground {
                            cf_actor::detect_wall(pos, half, &is_solid)
                        } else {
                            None
                        };
                        (vc, wc)
                    };
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&actor_id) {
                            actor.parkour_signal.vault_candidate = vault_cand;
                            if wall_cand.is_some() {
                                actor.parkour_signal.wall_candidate = wall_cand;
                                actor.parkour_signal.wall_contact_grace_remaining_ms =
                                    cf_actor::WALL_CONTACT_GRACE_MS;
                            } else if actor.on_ground {
                                actor.parkour_signal.wall_candidate = None;
                            }
                        }
                    }
                }
            }
        }
        if let Ok(mut state) = self.state.write() {
            // **Audit finding #2 fix**: per-tick re-pin `Actor`-typed rope
            // endpoints to the live actor position so the bob actually
            // follows the player (previously the bob was stuck at the
            // initial node position from construction). Detach orphaned
            // ropes whose actor disappeared.
            let live_actor_positions: BTreeMap<u64, [f32; 2]> = state
                .actor_state
                .as_ref()
                .map(|sim| {
                    sim.world
                        .actors
                        .iter()
                        .map(|(id, a)| (id.0, [a.position.x, a.position.y]))
                        .collect()
                })
                .unwrap_or_default();
            let mut orphaned_ropes: Vec<RopeId> = Vec::new();
            for (rid, rope) in state.m14j_ropes.iter_mut() {
                let all_resolved = rope.retrack_endpoints(|id| live_actor_positions.get(&id).copied());
                if !all_resolved {
                    orphaned_ropes.push(*rid);
                }
            }
            for rid in orphaned_ropes {
                state.m14j_ropes.remove(&rid);
                state.m14j_zipline_ropes.remove(&rid);
                if let Some(sim) = state.actor_state.as_mut() {
                    for actor in sim.world.actors.values_mut() {
                        if actor.holding_rope == Some(rid) {
                            actor.holding_rope = None;
                        }
                        if actor.zipline_attached == Some(rid) {
                            actor.zipline_attached = None;
                        }
                    }
                }
            }
            // 1) Step every rope.
            for rope in state.m14j_ropes.values_mut() {
                rope.step(dt, cf_physics::DEFAULT_SOLVER_ITERATIONS);
            }
            // 2) Advance zipline riders.
            let zipline_ids: Vec<RopeId> = state.m14j_zipline_ropes.iter().copied().collect();
            for rid in zipline_ids {
                let rope_geo = state.m14j_ropes.get(&rid).map(|r| {
                    let start = r.nodes.first().map(|n| n.position).unwrap_or([0.0, 0.0]);
                    let end = r.nodes.last().map(|n| n.position).unwrap_or([0.0, 0.0]);
                    let dx = end[0] - start[0];
                    let dy = end[1] - start[1];
                    let span = (dx * dx + dy * dy).sqrt().max(1e-3);
                    let slope_pct = (dy.abs() / span).min(1.0);
                    (start, end, span, slope_pct, dy)
                });
                let Some((start, end, span, slope_pct, _dy)) = rope_geo else {
                    continue;
                };
                let (high_end, low_end) = if start[1] > end[1] { (start, end) } else { (end, start) };
                // Find riders clipped to this line.
                let rider_ids: Vec<u64> = state
                    .actor_state
                    .as_ref()
                    .map(|sim| {
                        sim.world
                            .actors
                            .iter()
                            .filter_map(|(id, a)| if a.zipline_attached == Some(rid) { Some(id.0) } else { None })
                            .collect()
                    })
                    .unwrap_or_default();
                for rider_id in rider_ids {
                    let brake = state
                        .actor_state
                        .as_ref()
                        .and_then(|sim| sim.world.actors.get(&ActorId(rider_id)))
                        .map(|a| a.zipline_brake_engaged)
                        .unwrap_or(false);
                    let speed = state.m14j_zipline_speed_by_rider.get(&rider_id).copied().unwrap_or(0.0);
                    let new_speed = cf_equipment::zipline_step_speed(speed, slope_pct, dt, brake, 9.81);
                    state.m14j_zipline_speed_by_rider.insert(rider_id, new_speed);
                    if let Some(sim) = state.actor_state.as_mut() {
                        if let Some(actor) = sim.world.actors.get_mut(&ActorId(rider_id)) {
                            // Direction = from high to low end, normalized.
                            let dx = low_end[0] - high_end[0];
                            let dy = low_end[1] - high_end[1];
                            let len = (dx * dx + dy * dy).sqrt().max(1e-3);
                            let unit = [dx / len, dy / len];
                            actor.velocity.x = unit[0] * new_speed;
                            actor.velocity.y = unit[1] * new_speed;
                            // When the rider reaches the low end, release.
                            let from_low = [actor.position.x - low_end[0], actor.position.y - low_end[1]];
                            let dist_to_low = (from_low[0] * from_low[0] + from_low[1] * from_low[1]).sqrt();
                            if dist_to_low < 0.6 && brake {
                                actor.zipline_attached = None;
                            }
                            let _ = span; // (suppress unused-var lint if compiler complains)
                        }
                    }
                }
            }
            // 3) Per-actor M14J integration pass: parkour timers, auto-vault,
            //    wall-jump grace, swim limb-path advance, swim stroke event,
            //    breath drain, drowning trigger.
            let dt_ms = ((1000.0 / tick_rate_hz) as u32).max(1);
            let actor_ids: Vec<u64> = state
                .actor_state
                .as_ref()
                .map(|sim| sim.world.actors.keys().map(|id| id.0).collect())
                .unwrap_or_default();
            // Snapshot per-actor M14J event outputs we then drain into the
            // recorder once the per-actor borrow is dropped.
            let mut per_actor_events: Vec<(u64, cf_actor::M14jTickEvents, f32)> = Vec::new();
            // critter.mass → critter chassis aggregates both for M14A speed
            // curves"**. Compute per-critter rider mass before mutating sim.
            let mut critter_extra_mass: BTreeMap<u64, f32> = BTreeMap::new();
            let mut critter_ride_direction: BTreeMap<u64, [f32; 2]> = BTreeMap::new();
            if let Some(sim) = state.actor_state.as_ref() {
                for (_rider_id, actor) in &sim.world.actors {
                    if let Some(m) = actor.mount {
                        critter_extra_mass.insert(m.critter_id.0, actor.mass_kg);
                        critter_ride_direction.insert(m.critter_id.0, m.ride_direction);
                    }
                }
            }
            if let Some(sim) = state.actor_state.as_mut() {
                for id in &actor_ids {
                    let actor_id = ActorId(*id);
                    let Some(actor) = sim.world.actors.get_mut(&actor_id) else {
                        continue;
                    };
                    let ev = cf_actor::tick_m14j_actor(actor, dt_ms, _tick.0);
                    let vault_av = actor.parkour_signal.vault_candidate.is_some();
                    let grapple_av = actor.holding_rope.is_none() && actor.zipline_attached.is_none();
                    let mount_av = actor.mount.is_some() || actor.is_being_ridden;
                    let zip_brake_av = actor.zipline_attached.is_some();
                    actor.quick_action_bar.update_m14j_context(vault_av, grapple_av, mount_av, zip_brake_av);
                    // when this actor is a critter being ridden, apply the rider
                    // mass to its mass cache (forces M14A walk-speed curves to
                    // recompute against combined mass).
                    if let Some(&extra_mass) = critter_extra_mass.get(&actor_id.0) {
                        actor.total_mass_cached = actor.mass_kg + extra_mass;
                        actor.total_mass_dirty = false;
                    }
                    // locomotion goal"**: when the critter has a rider, use
                    // ride_direction (scaled by MOUNT_TOP_SPEED_RETAINED + critter's
                    // top speed) as the goal velocity. For BP-level baseline, use
                    // a critter gait selector + apply velocity directly.
                    if let Some(&dir) = critter_ride_direction.get(&actor_id.0) {
                        let goal = cf_ai::select_gait_for_ride_input(
                            dir[0],
                            dir[1],
                            cf_actor::MOUNT_TOP_SPEED_RETAINED,
                        );
                        let top = goal.effective_top_speed;
                        actor.velocity.x = goal.goal_direction[0] * top;
                        // Vertical from gait is ignored on ground; air motion handled by gravity.
                    }
                    per_actor_events.push((*id, ev, actor.swim_breath_seconds));
                }
            }
            drop(state);
            // 4) Emit replay events for the per-actor M14J pass.
            for (actor_id, ev, breath_seconds) in per_actor_events {
                if let Some(h) = ev.vault_triggered_height_m {
                    self.recorder.record(
                        _tick,
                        _sim_time_ms,
                        "actor",
                        "vaulted",
                        json!({
                            "actor": actor_id,
                            "tick": _tick.0,
                            "obstacle_height": h,
                            "duration_ms": cf_actor::VAULT_DURATION_MS,
                            "trigger": "auto",
                        }),
                        None,
                    );
                }
                if let Some(kind) = ev.swim_stroke_emitted {
                    self.recorder.record(
                        _tick,
                        _sim_time_ms,
                        "swim",
                        "stroke",
                        json!({
                            "actor": actor_id,
                            "tick": _tick.0,
                            "stroke_kind": kind.as_str(),
                            "stamina_remaining": ev.stamina_remaining_snapshot,
                            "drain_multiplier": ev.swim_drain_multiplier_snapshot,
                        }),
                        None,
                    );
                }
                if let Some(depth_m) = ev.actor_drowned_depth_m {
                    // M14J § "actor_drowned (M14J supersedes M16) fires with
                    // breath_held_s + depth_m".
                    let race = self
                        .m14j_actor_clone(actor_id)
                        .map(|a| a.origin_id)
                        .unwrap_or_else(|| "human".to_string());
                    let drowned_id = self.recorder.record(
                        _tick,
                        _sim_time_ms,
                        "actor",
                        "drowned",
                        json!({
                            "actor": actor_id,
                            "tick": _tick.0,
                            "breath_held_s": breath_seconds.max(0.0),
                            "depth_m": depth_m,
                            "race": race,
                        }),
                        None,
                    );
                    // M14J § storyteller hook for actor_drowned.
                    self.recorder.record(
                        _tick,
                        _sim_time_ms,
                        "storyteller",
                        "actor_drowned",
                        json!({
                            "actor": actor_id,
                            "tick": _tick.0,
                            "depth_m": depth_m,
                            "race": race,
                        }),
                        Some(drowned_id),
                    );
                    // Append Drowning affliction.
                    if let Ok(mut state) = self.state.write() {
                        if let Some(sim) = state.actor_state.as_mut() {
                            if let Some(actor) = sim.world.actors.get_mut(&ActorId(actor_id)) {
                                actor.afflictions.push(cf_actor::Affliction {
                                    kind: cf_actor::AfflictionKind::Drowning,
                                    intensity: 1.0,
                                    expires_tick: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
}
