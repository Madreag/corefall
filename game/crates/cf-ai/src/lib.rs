//! M1.5 reactive enemy controller.
//!
//! M1.5 ships ONE enemy archetype: the `ReactiveGuard`. It exists to give the
//! micro-breach scenario a reason to exist (pressure + counter-attack) without
//! pre-empting the M6 AI core. The DR-008 LEAN (hybrid jobs + utility scoring +
//! scripted hooks) is honoured by this implementation as follows:
//!
//! - **Job (intent layer)**: the guard runs a tiny scripted state machine —
//!   `Idle → Alerted → Engaged` — based on whether the player is inside its sight
//!   cone. M6 will replace the script with the full job board.
//! - **Tactic (utility scoring)**: per tick the guard scores three tactics
//!   (`Reload`, `Attack`, `Hold`) and picks the highest. Scores are deterministic
//!   functions of the tick, distance, ammo, and cooldowns. M6 will widen the
//!   tactic library; the score-then-pick contract stays the same.
//! - **Custom (scripted hooks)**: aim settle, miss roll, and burst pacing are
//!   scripted in this file. Mods will eventually slot in via the M5/M8 modding
//!   data path; M1.5 keeps everything in code.
//!
//! Every recorder-relevant decision is exposed via [`EnemyTickReport`]; the
//! engine turns it into the `ai.*` / `equipment.weapon_*` / `combat.projectile_*`
//! events the run-bundle schema requires for M1.5.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::struct_excessive_bools,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::if_not_else,
    clippy::field_reassign_with_default,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::too_many_lines,
    clippy::trivially_copy_pass_by_ref,
    clippy::large_enum_variant
)]

use serde::{Deserialize, Serialize};

use cf_actor::{ActorId, ActorState, Status, Vec2};
use cf_sim_core::Rng;

/// Tunable parameters for the M1.5 reactive guard.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ReactiveGuardParams {
    pub sight_radius: f32,
    /// Total cone angle in degrees (so half on each side of the facing direction).
    pub sight_cone_degrees: f32,
    /// Time after first sighting before the guard can fire. Clamped to ≥ 0.05 s.
    pub aim_settle_seconds: f32,
    /// Probability `[0, 1]` that an otherwise-valid shot misses (aim drift). The
    /// engine uses the seeded RNG so the same scenario+seed produces identical
    /// outcomes across runs.
    pub miss_chance: f32,
    /// Seconds the guard stays alerted after losing sight before reverting to idle.
    pub alert_dwell_seconds: f32,
    /// Number of shots in a burst before the guard pauses (and considers reloading).
    pub burst_shots: u32,
    /// Pause between bursts in seconds.
    pub burst_pause_seconds: f32,
    /// Damage applied to the player on a successful hit. Independent of the player's
    /// rifle preset — guard balance lives in this struct.
    pub damage_per_hit: f32,
    /// Speed of guard projectiles (world units / s).
    pub projectile_speed: f32,
    /// Lifetime of guard projectiles in seconds.
    pub projectile_lifetime_seconds: f32,
    /// Magazine capacity. After this many shots the guard reloads.
    pub mag_capacity: u32,
    /// Reload duration in seconds.
    pub reload_seconds: f32,
    /// Forward muzzle offset (world units, projected along aim).
    pub muzzle_forward_offset: f32,
    /// Vertical muzzle offset (world units, additive).
    pub muzzle_vertical_offset: f32,
}

impl Default for ReactiveGuardParams {
    fn default() -> Self {
        Self {
            sight_radius: 480.0,
            sight_cone_degrees: 120.0,
            aim_settle_seconds: 0.4,
            miss_chance: 0.35,
            alert_dwell_seconds: 1.5,
            burst_shots: 3,
            burst_pause_seconds: 0.45,
            damage_per_hit: 14.0,
            projectile_speed: 900.0,
            projectile_lifetime_seconds: 1.4,
            mag_capacity: 12,
            reload_seconds: 1.8,
            muzzle_forward_offset: 12.0,
            muzzle_vertical_offset: 4.0,
        }
    }
}

impl ReactiveGuardParams {
    fn aim_settle_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.aim_settle_seconds, tick_rate_hz)
    }
    fn alert_dwell_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.alert_dwell_seconds, tick_rate_hz)
    }
    fn burst_pause_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.burst_pause_seconds, tick_rate_hz)
    }
    fn reload_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.reload_seconds, tick_rate_hz)
    }
    pub fn projectile_lifetime_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.projectile_lifetime_seconds, tick_rate_hz)
    }
}

fn seconds_to_ticks(seconds: f32, tick_rate_hz: u32) -> u32 {
    let rate = tick_rate_hz.max(1);
    let ticks = (f64::from(seconds.max(0.0)) * f64::from(rate)).round();
    if ticks < 1.0 {
        1
    } else if ticks > f64::from(u32::MAX) {
        u32::MAX
    } else {
        ticks as u32
    }
}

/// Discrete states the guard can be in. The engine emits an `ai.state_changed`
/// event whenever this changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardState {
    Idle,
    Alerted,
    Engaged,
    Dead,
}

impl GuardState {
    pub fn as_str(self) -> &'static str {
        match self {
            GuardState::Idle => "idle",
            GuardState::Alerted => "alerted",
            GuardState::Engaged => "engaged",
            GuardState::Dead => "dead",
        }
    }
}

/// Tactic the utility scorer chose this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tactic {
    /// Standing by; no target.
    Hold,
    /// Aim and (eventually) fire at the player.
    Attack,
    /// Reload the magazine.
    Reload,
    /// Lost sight; investigate / dwell.
    Search,
}

impl Tactic {
    pub fn as_str(self) -> &'static str {
        match self {
            Tactic::Hold => "hold",
            Tactic::Attack => "attack",
            Tactic::Reload => "reload",
            Tactic::Search => "search",
        }
    }
}

/// Per-actor controller state. Lives across ticks; the engine owns the storage.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactiveGuard {
    pub actor: ActorId,
    pub params: ReactiveGuardParams,
    pub state: GuardState,
    pub aim: [f32; 2],
    pub last_player_seen_tick: Option<u64>,
    pub last_player_position: Option<[f32; 2]>,
    pub aim_settle_remaining_ticks: u32,
    pub alert_dwell_remaining_ticks: u32,
    pub burst_pause_remaining_ticks: u32,
    pub reload_remaining_ticks: u32,
    pub fire_cooldown_ticks: u32,
    pub burst_shots_fired: u32,
    pub ammo_in_mag: u32,
    pub last_tactic: Tactic,
}

impl ReactiveGuard {
    pub fn new(actor: ActorId, params: ReactiveGuardParams) -> Self {
        Self {
            actor,
            params,
            state: GuardState::Idle,
            aim: [-1.0, 0.0],
            last_player_seen_tick: None,
            last_player_position: None,
            aim_settle_remaining_ticks: 0,
            alert_dwell_remaining_ticks: 0,
            burst_pause_remaining_ticks: 0,
            reload_remaining_ticks: 0,
            fire_cooldown_ticks: 0,
            burst_shots_fired: 0,
            ammo_in_mag: params.mag_capacity,
            last_tactic: Tactic::Hold,
        }
    }

    /// Reset to spawn defaults. `scenario.reset` calls this so a re-played run
    /// starts the guard idle, fully loaded, and forgetful.
    pub fn reset(&mut self) {
        self.state = GuardState::Idle;
        self.aim = [-1.0, 0.0];
        self.last_player_seen_tick = None;
        self.last_player_position = None;
        self.aim_settle_remaining_ticks = 0;
        self.alert_dwell_remaining_ticks = 0;
        self.burst_pause_remaining_ticks = 0;
        self.reload_remaining_ticks = 0;
        self.fire_cooldown_ticks = 0;
        self.burst_shots_fired = 0;
        self.ammo_in_mag = self.params.mag_capacity;
        self.last_tactic = Tactic::Hold;
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(48);
        out.extend_from_slice(&self.actor.0.to_le_bytes());
        out.push(self.state as u8);
        out.extend_from_slice(&self.ammo_in_mag.to_le_bytes());
        out.extend_from_slice(&self.fire_cooldown_ticks.to_le_bytes());
        out.extend_from_slice(&self.reload_remaining_ticks.to_le_bytes());
        out.extend_from_slice(&self.aim_settle_remaining_ticks.to_le_bytes());
        out.extend_from_slice(&self.alert_dwell_remaining_ticks.to_le_bytes());
        out.extend_from_slice(&self.burst_pause_remaining_ticks.to_le_bytes());
        out.extend_from_slice(&self.burst_shots_fired.to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.aim[0]).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.aim[1]).to_le_bytes());
        out.extend_from_slice(&self.last_player_seen_tick.unwrap_or(0).to_le_bytes());
        out
    }
}

fn quantize_f32(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1024.0).round() as i32
}

/// View projection of the guard for `observe.frame`. Cosmetic-only fields the
/// HUD reads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactiveGuardView {
    pub actor: u64,
    pub state: String,
    pub last_tactic: String,
    pub ammo: u32,
    pub mag_capacity: u32,
    pub fire_cooldown_ticks: u32,
    pub reload_remaining_ticks: u32,
    pub aim_settle_remaining_ticks: u32,
    pub alert_dwell_remaining_ticks: u32,
    pub aim: [f32; 2],
}

impl From<&ReactiveGuard> for ReactiveGuardView {
    fn from(g: &ReactiveGuard) -> Self {
        Self {
            actor: g.actor.0,
            state: g.state.as_str().to_string(),
            last_tactic: g.last_tactic.as_str().to_string(),
            ammo: g.ammo_in_mag,
            mag_capacity: g.params.mag_capacity,
            fire_cooldown_ticks: g.fire_cooldown_ticks,
            reload_remaining_ticks: g.reload_remaining_ticks,
            aim_settle_remaining_ticks: g.aim_settle_remaining_ticks,
            alert_dwell_remaining_ticks: g.alert_dwell_remaining_ticks,
            aim: g.aim,
        }
    }
}

/// Inputs for one [`step`] call.
#[derive(Debug, Clone, Copy)]
pub struct GuardTickInputs<'a> {
    pub tick: u64,
    pub tick_rate_hz: u32,
    pub self_actor: &'a ActorState,
    pub player: Option<&'a ActorState>,
}

/// Outcomes of one [`step`] call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnemyTickReport {
    pub state_changed: Option<GuardStateTransition>,
    pub perception: Option<PerceptionRecord>,
    pub tactic_chosen: Option<TacticRecord>,
    pub fire: Option<FireRecord>,
    pub reload_started: bool,
    pub reload_completed: bool,
    pub dry_fire: bool,
}

/// Recorded `ai.state_changed` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct GuardStateTransition {
    pub previous: GuardState,
    pub next: GuardState,
    pub cause: &'static str,
}

/// Recorded `ai.perception` payload.
#[derive(Debug, Clone, PartialEq)]
pub struct PerceptionRecord {
    pub player_seen: bool,
    pub distance: Option<f32>,
    pub angle_degrees: Option<f32>,
    pub last_seen_position: Option<[f32; 2]>,
    pub state: GuardState,
}

/// Recorded `ai.tactic_chosen` payload. `score_*` fields are the utility scores
/// the scorer evaluated this tick — exposed so the run-bundle viewer can show
/// the AI's reasoning.
#[derive(Debug, Clone, PartialEq)]
pub struct TacticRecord {
    pub tactic: Tactic,
    pub reason: &'static str,
    pub score_attack: f32,
    pub score_reload: f32,
    pub score_hold: f32,
    pub score_search: f32,
}

/// Recorded enemy weapon fire. The engine spawns a projectile the player can
/// actually be hit by.
#[derive(Debug, Clone, PartialEq)]
pub struct FireRecord {
    pub muzzle_origin: [f32; 2],
    pub velocity: [f32; 2],
    pub aim: [f32; 2],
    pub damage: f32,
    pub miss_roll: f32,
    pub miss_threshold: f32,
    pub will_miss: bool,
    pub lifetime_ticks: u32,
}

/// One reactive-guard tick. Returns a structured report the engine turns into
/// recorder events; the engine is responsible for spawning the projectile and
/// applying damage when `fire.is_some()` AND `!fire.will_miss`.
#[must_use]
pub fn step(guard: &mut ReactiveGuard, inputs: GuardTickInputs<'_>, rng: &mut Rng) -> EnemyTickReport {
    let mut report = EnemyTickReport::default();

    // 1) Death check. A dead guard does nothing.
    if inputs.self_actor.status == Status::Dead || inputs.self_actor.hp <= 0.0 {
        if guard.state != GuardState::Dead {
            let prev = guard.state;
            guard.state = GuardState::Dead;
            report.state_changed = Some(GuardStateTransition {
                previous: prev,
                next: GuardState::Dead,
                cause: "actor_died",
            });
        }
        return report;
    }

    // 2) Tick down cooldowns.
    decrement(&mut guard.fire_cooldown_ticks, 1);
    decrement(&mut guard.aim_settle_remaining_ticks, 1);
    decrement(&mut guard.burst_pause_remaining_ticks, 1);
    decrement(&mut guard.alert_dwell_remaining_ticks, 1);

    // 3) Reload progress.
    if guard.reload_remaining_ticks > 0 {
        guard.reload_remaining_ticks -= 1;
        if guard.reload_remaining_ticks == 0 {
            guard.ammo_in_mag = guard.params.mag_capacity;
            guard.burst_shots_fired = 0;
            report.reload_completed = true;
        }
    }

    // 4) Perception. The guard sees the player when:
    //    - Player exists and is alive.
    //    - Distance ≤ sight_radius.
    //    - Angle from the guard's facing direction ≤ sight_cone / 2.
    let perception = compute_perception(guard, &inputs);
    report.perception.clone_from(&perception);

    // 5) State machine. Transitions are reason-labelled so the recorder cause
    //    chain stays semantically valid.
    if let Some(p) = &perception {
        if p.player_seen {
            guard.last_player_seen_tick = Some(inputs.tick);
            guard.last_player_position = p.last_seen_position;
            guard.alert_dwell_remaining_ticks = guard.params.alert_dwell_ticks(inputs.tick_rate_hz);
            // First sighting starts the aim-settle timer.
            if guard.state != GuardState::Engaged {
                guard.aim_settle_remaining_ticks = guard.params.aim_settle_ticks(inputs.tick_rate_hz);
            }
            let prev = guard.state;
            guard.state = GuardState::Engaged;
            if prev != GuardState::Engaged {
                report.state_changed = Some(GuardStateTransition {
                    previous: prev,
                    next: GuardState::Engaged,
                    cause: "player_visible",
                });
            }
        } else if guard.alert_dwell_remaining_ticks > 0 {
            let prev = guard.state;
            if guard.state == GuardState::Engaged {
                guard.state = GuardState::Alerted;
                if prev != GuardState::Alerted {
                    report.state_changed = Some(GuardStateTransition {
                        previous: prev,
                        next: GuardState::Alerted,
                        cause: "player_lost",
                    });
                }
            }
        } else if guard.state != GuardState::Idle {
            let prev = guard.state;
            guard.state = GuardState::Idle;
            report.state_changed = Some(GuardStateTransition {
                previous: prev,
                next: GuardState::Idle,
                cause: "alert_expired",
            });
        }
    }

    // 6) Aim tracking. When a player is currently visible, aim straight at them.
    //    When alerted but not visible, aim at the last seen position.
    update_aim(guard, &perception, inputs.self_actor.position);

    // 7) Utility scoring → tactic choice.
    let player_visible = perception.as_ref().is_some_and(|p| p.player_seen);
    let player_distance = perception.as_ref().and_then(|p| p.distance);
    let scores = score_tactics(guard, player_visible, player_distance);
    let (tactic, reason) = pick_tactic(guard, &scores, player_visible);
    guard.last_tactic = tactic;
    report.tactic_chosen = Some(TacticRecord {
        tactic,
        reason,
        score_attack: scores.attack,
        score_reload: scores.reload,
        score_hold: scores.hold,
        score_search: scores.search,
    });

    // 8) Apply tactic.
    match tactic {
        Tactic::Reload => {
            if guard.reload_remaining_ticks == 0 && guard.ammo_in_mag < guard.params.mag_capacity {
                guard.reload_remaining_ticks = guard.params.reload_ticks(inputs.tick_rate_hz);
                guard.fire_cooldown_ticks = 0;
                guard.burst_pause_remaining_ticks = 0;
                guard.burst_shots_fired = 0;
                report.reload_started = true;
            }
        }
        Tactic::Attack => {
            if let Some(fire) = try_fire(guard, inputs.self_actor, &perception, rng, inputs.tick_rate_hz) {
                report.fire = Some(fire);
            } else if guard.ammo_in_mag == 0 && guard.reload_remaining_ticks == 0 {
                report.dry_fire = true;
                guard.reload_remaining_ticks = guard.params.reload_ticks(inputs.tick_rate_hz);
                report.reload_started = true;
            }
        }
        Tactic::Hold | Tactic::Search => {}
    }

    report
}

fn decrement(value: &mut u32, by: u32) {
    if *value >= by {
        *value -= by;
    } else {
        *value = 0;
    }
}

fn compute_perception(guard: &ReactiveGuard, inputs: &GuardTickInputs<'_>) -> Option<PerceptionRecord> {
    let player = inputs.player?;
    if player.status.is_dead() {
        return Some(PerceptionRecord {
            player_seen: false,
            distance: None,
            angle_degrees: None,
            last_seen_position: guard.last_player_position,
            state: guard.state,
        });
    }
    let dx = player.position.x - inputs.self_actor.position.x;
    let dy = player.position.y - inputs.self_actor.position.y;
    let distance = ((dx * dx) + (dy * dy)).sqrt();
    if distance > guard.params.sight_radius {
        return Some(PerceptionRecord {
            player_seen: false,
            distance: Some(distance),
            angle_degrees: None,
            last_seen_position: guard.last_player_position,
            state: guard.state,
        });
    }
    let facing = if inputs.self_actor.aim != Vec2::ZERO {
        inputs.self_actor.aim.normalize_or_x()
    } else {
        Vec2::new(-1.0, 0.0)
    };
    let to_player = if distance > 1e-3 {
        Vec2::new(dx / distance, dy / distance)
    } else {
        return Some(PerceptionRecord {
            player_seen: true,
            distance: Some(distance),
            angle_degrees: Some(0.0),
            last_seen_position: Some([player.position.x, player.position.y]),
            state: guard.state,
        });
    };
    let dot = (facing.x * to_player.x + facing.y * to_player.y).clamp(-1.0, 1.0);
    let angle_rad = dot.acos();
    let angle_deg = angle_rad * 180.0 / std::f32::consts::PI;
    let half_cone = (guard.params.sight_cone_degrees / 2.0).max(0.0);
    let visible = angle_deg <= half_cone;
    Some(PerceptionRecord {
        player_seen: visible,
        distance: Some(distance),
        angle_degrees: Some(angle_deg),
        last_seen_position: if visible {
            Some([player.position.x, player.position.y])
        } else {
            guard.last_player_position
        },
        state: guard.state,
    })
}

fn update_aim(guard: &mut ReactiveGuard, perception: &Option<PerceptionRecord>, self_pos: Vec2) {
    let target = match perception {
        Some(p) if p.player_seen => p.last_seen_position,
        _ => guard.last_player_position,
    };
    if let Some([tx, ty]) = target {
        let dx = tx - self_pos.x;
        let dy = ty - self_pos.y;
        let len = ((dx * dx) + (dy * dy)).sqrt();
        if len > 1e-3 {
            guard.aim = [dx / len, dy / len];
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TacticScores {
    attack: f32,
    reload: f32,
    hold: f32,
    search: f32,
}

fn score_tactics(guard: &ReactiveGuard, player_visible: bool, player_distance: Option<f32>) -> TacticScores {
    let mut scores = TacticScores::default();
    let ammo_ratio = if guard.params.mag_capacity == 0 {
        0.0
    } else {
        guard.ammo_in_mag as f32 / guard.params.mag_capacity as f32
    };
    let reloading = guard.reload_remaining_ticks > 0;

    // Reload: high when low on ammo and not reloading; impossible while reloading.
    if reloading {
        scores.reload = -1.0;
    } else if ammo_ratio <= 0.0 {
        scores.reload = 1.0;
    } else if ammo_ratio < 0.34 {
        scores.reload = 0.6;
    } else {
        scores.reload = 0.05;
    }

    // Attack: requires visibility + ammo + cooldown clear; weighted by distance.
    if player_visible && guard.ammo_in_mag > 0 && guard.fire_cooldown_ticks == 0 && !reloading {
        let distance_pull = match player_distance {
            Some(d) => {
                let normalized = (1.0 - (d / guard.params.sight_radius)).clamp(0.0, 1.0);
                0.4 + 0.6 * normalized
            }
            None => 0.6,
        };
        let burst_penalty = if guard.burst_pause_remaining_ticks > 0 {
            -0.5
        } else {
            0.0
        };
        let aim_penalty = if guard.aim_settle_remaining_ticks > 0 {
            -0.25
        } else {
            0.0
        };
        scores.attack = (distance_pull + burst_penalty + aim_penalty).clamp(-1.0, 1.0);
    }

    // Hold: baseline non-zero so a guard with no tactic doesn't sit at score 0.0.
    scores.hold = 0.1;

    // Search: small positive when alerted-without-sight.
    if guard.state == GuardState::Alerted && !player_visible {
        scores.search = 0.3;
    }

    scores
}

fn pick_tactic(guard: &ReactiveGuard, scores: &TacticScores, player_visible: bool) -> (Tactic, &'static str) {
    if guard.reload_remaining_ticks > 0 {
        return (Tactic::Reload, "reload_in_progress");
    }
    if guard.ammo_in_mag == 0 {
        return (Tactic::Reload, "magazine_empty");
    }
    let mut best = (Tactic::Hold, scores.hold, "hold_default");
    if scores.attack > best.1 {
        best = (Tactic::Attack, scores.attack, "attack_target");
    }
    if scores.reload > best.1 {
        best = (Tactic::Reload, scores.reload, "low_ammo");
    }
    if scores.search > best.1 {
        best = (Tactic::Search, scores.search, "search_alerted");
    }
    let _ = player_visible; // Reserved for future heuristics; kept for ergonomics.
    (best.0, best.2)
}

fn try_fire(
    guard: &mut ReactiveGuard,
    self_actor: &ActorState,
    perception: &Option<PerceptionRecord>,
    rng: &mut Rng,
    tick_rate_hz: u32,
) -> Option<FireRecord> {
    if guard.aim_settle_remaining_ticks > 0 {
        return None;
    }
    if guard.fire_cooldown_ticks > 0 {
        return None;
    }
    if guard.burst_pause_remaining_ticks > 0 {
        return None;
    }
    if guard.ammo_in_mag == 0 {
        return None;
    }
    let player_visible = perception.as_ref().is_some_and(|p| p.player_seen);
    if !player_visible {
        return None;
    }
    let aim_unit = Vec2::new(guard.aim[0], guard.aim[1]).normalize_or_x();
    let muzzle = [
        self_actor.position.x + aim_unit.x * guard.params.muzzle_forward_offset,
        self_actor.position.y + guard.params.muzzle_vertical_offset + aim_unit.y * guard.params.muzzle_forward_offset,
    ];
    // Miss roll: deterministic from the engine RNG so replays match. We pull one
    // u64 and project its high 53 bits onto [0, 1). `u64::MAX as f64` would round
    // up to 2^64 (f64 has only 52 mantissa bits), so the largest u64 values would
    // produce exactly 1.0 and let `miss_chance == 1.0` ("always miss") still hit.
    let raw = rng.next_u64();
    let unit_roll = ((raw >> 11) as f64 / ((1u64 << 53) as f64)) as f32;
    let miss_threshold = guard.params.miss_chance.clamp(0.0, 1.0);
    // f32's ~24-bit mantissa cannot represent values strictly between (1 - 2^-24)
    // and 1.0, so `unit_roll` can still round up to 1.0 even from the 53-bit
    // source. Treat `miss_chance >= 1.0` as a guaranteed miss to honor the
    // documented `[0, 1]` contract.
    let will_miss = miss_threshold >= 1.0 || unit_roll < miss_threshold;
    let velocity = if will_miss {
        // Drift the projectile a fixed angular amount — enough to miss a 16-wide
        // actor at the maximum sight radius. The drift sign alternates by burst
        // shot index so misses are visually varied.
        let drift: f32 = 0.18
            * if guard.burst_shots_fired.is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
        let cos = drift.cos();
        let sin = drift.sin();
        let dx = aim_unit.x * cos - aim_unit.y * sin;
        let dy = aim_unit.x * sin + aim_unit.y * cos;
        [dx * guard.params.projectile_speed, dy * guard.params.projectile_speed]
    } else {
        [
            aim_unit.x * guard.params.projectile_speed,
            aim_unit.y * guard.params.projectile_speed,
        ]
    };
    guard.ammo_in_mag = guard.ammo_in_mag.saturating_sub(1);
    guard.burst_shots_fired += 1;
    guard.fire_cooldown_ticks = seconds_to_ticks(0.20, tick_rate_hz);
    if guard.burst_shots_fired >= guard.params.burst_shots {
        guard.burst_pause_remaining_ticks = guard.params.burst_pause_ticks(tick_rate_hz);
        guard.burst_shots_fired = 0;
    }
    let lifetime_ticks = guard.params.projectile_lifetime_ticks(tick_rate_hz);
    Some(FireRecord {
        muzzle_origin: muzzle,
        velocity,
        aim: [aim_unit.x, aim_unit.y],
        damage: guard.params.damage_per_hit,
        miss_roll: unit_roll,
        miss_threshold,
        will_miss,
        lifetime_ticks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_actor::{Inventory, InventoryItem, ItemSlot};

    fn guard_actor() -> ActorState {
        let inv = Inventory {
            items: vec![InventoryItem::Empty; 4],
            selected: ItemSlot(0),
        };
        let mut a = ActorState::player(ActorId(2), "red", Vec2::new(900.0, 32.0), 80.0, inv);
        a.controllable = false;
        a.aim = Vec2::new(-1.0, 0.0);
        a
    }

    fn player_actor(x: f32, y: f32) -> ActorState {
        ActorState::player(ActorId(1), "blue", Vec2::new(x, y), 100.0, Inventory::default())
    }

    fn rng() -> Rng {
        Rng::from_seed(13)
    }

    fn tick_inputs<'a>(tick: u64, guard_a: &'a ActorState, player: Option<&'a ActorState>) -> GuardTickInputs<'a> {
        GuardTickInputs {
            tick,
            tick_rate_hz: 60,
            self_actor: guard_a,
            player,
        }
    }

    #[test]
    fn idle_when_player_not_present() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, None), &mut rng);
        assert_eq!(guard.state, GuardState::Idle);
        assert!(report.fire.is_none());
        assert!(report.tactic_chosen.is_some());
    }

    #[test]
    fn engages_when_player_in_cone() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert_eq!(guard.state, GuardState::Engaged);
        assert!(report.state_changed.is_some());
        let perception = report.perception.unwrap();
        assert!(perception.player_seen);
        assert!(perception.distance.unwrap() > 0.0);
    }

    #[test]
    fn does_not_fire_during_aim_settle() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        // Tick 1 starts aim settle.
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert!(report.fire.is_none());
        assert!(guard.aim_settle_remaining_ticks > 0);
    }

    #[test]
    fn fires_after_aim_settles() {
        let mut params = ReactiveGuardParams::default();
        params.miss_chance = 0.0;
        params.aim_settle_seconds = 0.05;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = Rng::from_seed(7);
        let mut shots = 0;
        for tick in 1..=120 {
            let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            if report.fire.is_some() {
                shots += 1;
            }
        }
        assert!(shots > 0, "guard must fire at least once after aim settle");
    }

    #[test]
    fn out_of_cone_does_not_engage() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let mut actor = guard_actor();
        actor.aim = Vec2::new(1.0, 0.0); // Face right.
        let player = player_actor(0.0, 32.0); // Player far to the left.
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        let perception = report.perception.unwrap();
        assert!(!perception.player_seen);
        assert_ne!(guard.state, GuardState::Engaged);
    }

    #[test]
    fn dead_actor_locks_state_to_dead() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        let mut actor = guard_actor();
        actor.hp = 0.0;
        actor.status = Status::Dead;
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, None), &mut rng);
        assert_eq!(guard.state, GuardState::Dead);
        assert!(report.state_changed.is_some());
    }

    #[test]
    fn deterministic_under_same_seed() {
        fn play_500_ticks(seed: u64) -> Vec<bool> {
            let mut params = ReactiveGuardParams::default();
            params.aim_settle_seconds = 0.05;
            let mut guard = ReactiveGuard::new(ActorId(2), params);
            let actor = guard_actor();
            let player = player_actor(700.0, 32.0);
            let mut rng = Rng::from_seed(seed);
            let mut fires = Vec::new();
            for tick in 1..=500 {
                let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
                fires.push(report.fire.is_some());
            }
            fires
        }
        let a = play_500_ticks(13);
        let b = play_500_ticks(13);
        assert_eq!(a, b, "same seed must produce identical fire pattern");
    }

    #[test]
    fn out_of_ammo_triggers_reload() {
        let mut params = ReactiveGuardParams::default();
        params.aim_settle_seconds = 0.05;
        params.miss_chance = 0.0;
        params.mag_capacity = 2;
        params.burst_shots = 2;
        params.burst_pause_seconds = 0.05;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = rng();
        let mut reload_started = false;
        for tick in 1..=300 {
            let report = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            if report.reload_started {
                reload_started = true;
                break;
            }
        }
        assert!(reload_started);
    }

    #[test]
    fn reset_returns_full_mag_and_idle() {
        let mut guard = ReactiveGuard::new(ActorId(2), ReactiveGuardParams::default());
        guard.ammo_in_mag = 0;
        guard.state = GuardState::Engaged;
        guard.reload_remaining_ticks = 30;
        guard.reset();
        assert_eq!(guard.state, GuardState::Idle);
        assert_eq!(guard.ammo_in_mag, ReactiveGuardParams::default().mag_capacity);
        assert_eq!(guard.reload_remaining_ticks, 0);
    }
}
