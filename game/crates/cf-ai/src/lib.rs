//! M1.5 reactive enemy controller.
//!
//! M1.5 ships ONE enemy archetype: the `ReactiveGuard`. It exists to give the
//! micro-breach scenario a reason to exist (pressure + counter-attack) without
//! pre-empting the M6 AI core. The DR-008 LEAN (hybrid jobs + utility scoring +
//! scripted hooks) is honoured by this implementation as follows:
//!
//! - **Job (intent layer)**: the guard runs a tiny scripted state machine —
//!   `Idle → Alert → Engaged → Retreating → Dying → Dead` — based on whether the
//!   player is inside its sight cone (and its own hp). M6 will replace the
//!   script with the full job board.
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
    /// Time after first sighting before the guard can fire. Setting `0.0`
    /// produces an instant settle (no delay); any positive sub-tick value is
    /// rounded up to one tick.
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
    /// **M1.5 G1**: hp fraction below which the guard transitions to
    /// `Retreating` (default 0.30 = 30%). Once Retreating, tactic gating
    /// prefers Reload + Search over Attack and the AI does NOT promote back
    /// to Engaged without a hysteresis margin. Set to 0.0 to disable
    /// retreat behaviour entirely.
    #[serde(default = "default_retreat_hp_pct")]
    pub retreat_hp_pct: f32,
    /// **M1.5 G1**: hp fraction above which a Retreating guard returns to
    /// Engaged once a player is back in LOS. Hysteresis margin against the
    /// retreat_hp_pct gate so a healing wobble doesn't flap states.
    /// Default = retreat_hp_pct + 0.05.
    #[serde(default = "default_recover_hp_pct")]
    pub recover_hp_pct: f32,
    /// **M1.5 G1**: hearing radius for `equipment.alarm_registered`
    /// consumption (CCCP `NativeHumanAI.lua:558-615`). 0.0 disables hearing.
    #[serde(default = "default_hearing_radius")]
    pub hearing_radius: f32,
    /// **M1.5 G1**: number of ticks the guard remembers the last-known
    /// player position after losing LOS. After this many ticks elapse with
    /// no fresh sighting / hearing, the memory entry is purged and an
    /// `ai.perception_signal { kind: memory_decayed }` event fires.
    /// 0 disables memory decay (memory persists until reset).
    #[serde(default = "default_memory_decay_ticks")]
    pub memory_decay_ticks: u32,
    /// **M1.5 G1**: dwell seconds in `Dying` state before the engine
    /// transitions to `Dead`. Mirrors `cf-actor`'s DYING dwell so the AI
    /// state surface stays synchronised with the body state machine.
    #[serde(default = "default_dying_dwell_seconds")]
    pub dying_dwell_seconds: f32,
}

fn default_retreat_hp_pct() -> f32 {
    0.30
}
fn default_recover_hp_pct() -> f32 {
    0.35
}
fn default_hearing_radius() -> f32 {
    480.0
}
fn default_memory_decay_ticks() -> u32 {
    300
}
fn default_dying_dwell_seconds() -> f32 {
    1.0
}

/// **M1.5 G6**: deserialised entry from `content/ai/difficulty.json`.
/// Engines load the registry once at boot and apply a preset to each
/// reactive guard via `apply_to(params)`. Fields are public so cf-mod
/// validation can introspect.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DifficultyPreset {
    pub id: String,
    pub display_name: String,
    pub hp: f32,
    pub aim_settle_ticks: u32,
    pub miss_chance: f32,
    pub sight_range: f32,
    pub sight_fov_degrees: f32,
    pub hearing_radius: f32,
    pub memory_decay_ticks: u32,
    pub reload_ms: u32,
    pub retreat_hp_pct: f32,
}

impl DifficultyPreset {
    /// Apply this preset to the params struct in place. Fields not
    /// represented in the preset stay at their current value (e.g.
    /// burst_pause_seconds is a tuning detail not surfaced to the player).
    pub fn apply_to(&self, params: &mut ReactiveGuardParams, tick_rate_hz: u32) {
        params.miss_chance = self.miss_chance;
        params.sight_radius = self.sight_range;
        params.sight_cone_degrees = self.sight_fov_degrees;
        params.hearing_radius = self.hearing_radius;
        params.memory_decay_ticks = self.memory_decay_ticks;
        params.retreat_hp_pct = self.retreat_hp_pct;
        params.recover_hp_pct = (self.retreat_hp_pct + 0.05).min(1.0);
        // aim_settle_ticks → aim_settle_seconds via the tick rate so the
        // existing seconds-based field stays the source-of-truth.
        params.aim_settle_seconds = if tick_rate_hz > 0 {
            self.aim_settle_ticks as f32 / tick_rate_hz as f32
        } else {
            self.aim_settle_ticks as f32 / 60.0
        };
        params.reload_seconds = self.reload_ms as f32 / 1000.0;
    }

    /// Built-in preset by id (mirrors the three entries in
    /// `content/ai/difficulty.json`). Returns None for unknown ids.
    /// Used as a fallback when the registry file is missing / not loaded.
    pub fn builtin(id: &str) -> Option<DifficultyPreset> {
        Some(match id {
            "cakewalk" => DifficultyPreset {
                id: "cakewalk".into(),
                display_name: "Cakewalk".into(),
                hp: 60.0,
                aim_settle_ticks: 24,
                miss_chance: 0.3,
                sight_range: 240.0,
                sight_fov_degrees: 90.0,
                hearing_radius: 320.0,
                memory_decay_ticks: 180,
                reload_ms: 2400,
                retreat_hp_pct: 0.5,
            },
            "tough_crowd" => DifficultyPreset {
                id: "tough_crowd".into(),
                display_name: "Tough Crowd".into(),
                hp: 80.0,
                aim_settle_ticks: 12,
                miss_chance: 0.1,
                sight_range: 320.0,
                sight_fov_degrees: 120.0,
                hearing_radius: 480.0,
                memory_decay_ticks: 300,
                reload_ms: 1800,
                retreat_hp_pct: 0.3,
            },
            "veteran" => DifficultyPreset {
                id: "veteran".into(),
                display_name: "Veteran".into(),
                hp: 120.0,
                aim_settle_ticks: 6,
                miss_chance: 0.05,
                sight_range: 480.0,
                sight_fov_degrees: 140.0,
                hearing_radius: 600.0,
                memory_decay_ticks: 600,
                reload_ms: 1200,
                retreat_hp_pct: 0.2,
            },
            _ => return None,
        })
    }
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
            retreat_hp_pct: default_retreat_hp_pct(),
            recover_hp_pct: default_recover_hp_pct(),
            hearing_radius: default_hearing_radius(),
            memory_decay_ticks: default_memory_decay_ticks(),
            dying_dwell_seconds: default_dying_dwell_seconds(),
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
    /// **M1.5 G1**: dwell window in `Dying` before promoting to `Dead`.
    /// Mirrors the body state machine's DYING dwell (cf-actor's
    /// `dying_dwell_seconds`); the AI surface uses its own copy because
    /// the AI tick and the actor tick are independently invoked.
    pub fn dying_dwell_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.dying_dwell_seconds, tick_rate_hz)
    }
}

fn seconds_to_ticks(seconds: f32, tick_rate_hz: u32) -> u32 {
    let rate = tick_rate_hz.max(1);
    let clamped = seconds.max(0.0);
    // Preserve the explicit "no delay" intent: callers passing exactly 0.0
    // get 0 ticks. Any positive sub-tick duration still rounds up to 1 so
    // a configured timer can never silently disappear into the rounding.
    if clamped == 0.0 {
        return 0;
    }
    let ticks = (f64::from(clamped) * f64::from(rate)).round();
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
///
/// **M1.5**: spec mandates 6 states (`Idle → Alert → Engaged → Retreating →
/// Dying → Dead`). `Retreating` fires when the guard's hp drops below
/// `retreat_hp_pct * max_hp` (default 30%); `Dying` mirrors the actor body
/// state machine's 1000ms DYING dwell so the AI surface stays synchronised
/// with the body surface even while the actor is being torn down.
///
/// The serde name for the previously-called `Alerted` variant is now `alert`
/// to match the spec text (`ai.state_changed { to: "alert", ... }`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardState {
    Idle,
    Alert,
    Engaged,
    /// **M1.5**: hp dropped below `retreat_hp_pct * max_hp`; guard prefers
    /// reload + cover-seeking tactics over Attack.
    Retreating,
    /// **M1.5**: actor status entered DYING (HP=0). Guard cannot fire.
    /// Auto-transitions to Dead when the actor's body state machine
    /// completes its DYING dwell.
    Dying,
    Dead,
}

impl GuardState {
    pub fn as_str(self) -> &'static str {
        match self {
            GuardState::Idle => "idle",
            GuardState::Alert => "alert",
            GuardState::Engaged => "engaged",
            GuardState::Retreating => "retreating",
            GuardState::Dying => "dying",
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
    /// **M1.5 G1**: max_hp latched at construction so the retreat threshold
    /// stays stable when the actor's HP regenerates / is healed (M5+).
    /// Falls back to 1.0 when zero so divisions stay finite.
    #[serde(default = "default_max_hp")]
    pub max_hp: f32,
    /// **M1.5 G1**: countdown ticks while in `Dying`. When zero the engine
    /// fires the Dying → Dead transition with reason `"dying_dwell_elapsed"`.
    #[serde(default)]
    pub dying_dwell_remaining_ticks: u32,
    /// **M1.5 G2 (hearing)**: when the guard consumed an alarm this tick,
    /// the alarm source position so the perception_signal payload can echo
    /// the heard position back. None when no alarm consumed.
    #[serde(default)]
    pub heard_alarm_this_tick: Option<[f32; 2]>,
    /// **M1.5 G3 (memory grid)**: tick number when the most recent fresh
    /// player observation (sight OR hearing) landed. Used to decay the
    /// memory entry after `memory_decay_ticks`. None when no memory.
    #[serde(default)]
    pub memory_last_refresh_tick: Option<u64>,
    /// **M1.5 G5 (stuck recovery)**: number of ticks the guard has spent
    /// pursuing a player it can't reach. Engine increments per pursuit
    /// tick; resets to 0 when the player is visible or the guard fires
    /// successfully. > 60 triggers an `ai.stuck_state_changed` event +
    /// recovery action.
    #[serde(default)]
    pub stuck_ticks: u32,
    /// **M1.5 G5**: latched while the guard is in the "stuck recovery"
    /// substate. Cleared when stuck_ticks resets to 0.
    #[serde(default)]
    pub stuck_recovery_latched: bool,
}

fn default_max_hp() -> f32 {
    100.0
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
            max_hp: default_max_hp(),
            dying_dwell_remaining_ticks: 0,
            heard_alarm_this_tick: None,
            memory_last_refresh_tick: None,
            stuck_ticks: 0,
            stuck_recovery_latched: false,
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
        self.dying_dwell_remaining_ticks = 0;
        self.heard_alarm_this_tick = None;
        self.memory_last_refresh_tick = None;
        self.stuck_ticks = 0;
        self.stuck_recovery_latched = false;
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
    /// **M1.5 G2 (hearing)**: alarm events the engine collected this tick
    /// (typically the player's `equipment.alarm_registered` from rifle fire).
    /// The guard consumes alarms inside its `hearing_radius`. Multiple
    /// alarms within range collapse to one perception_signal per tick
    /// (closest-source wins).
    pub alarms: &'a [AlarmInput],
}

/// **M1.5 G2 (hearing)**: one alarm the guard can react to this tick.
#[derive(Debug, Clone, Copy)]
pub struct AlarmInput {
    pub source_actor: u64,
    pub source_position: [f32; 2],
    pub loudness_radius: f32,
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
    /// **M1.5 G2 (hearing) / G3 (memory)**: per-tick perception signals
    /// (sight, sight_lost, hearing, memory_decayed). One step may produce
    /// multiple signals; the engine emits one `ai.perception_signal` event
    /// per entry. The legacy `perception` field stays the dominant sight
    /// summary so existing replay consumers don't break.
    pub perception_signals: Vec<PerceptionSignal>,
    /// **M1.5 G4 (missed shot reason)**: populated when the guard fired
    /// AND the miss roll landed above the threshold.
    pub missed_shot_reason: Option<MissedShotReason>,
    /// **M1.5 G5 (stuck recovery)**: emitted on the tick the guard crosses
    /// the stuck-tick threshold.
    pub stuck_recovery: Option<StuckRecoveryRecord>,
    /// **M1.5 G1 (target acquired)**: populated on Engaged ← non-Engaged.
    pub target_acquired: Option<TargetAcquiredRecord>,
    /// **M1.5 G1 (target lost)**: populated on Engaged → Alert.
    pub target_lost: Option<TargetLostRecord>,
}

/// **M1.5 G2/G3**: one perception event the guard registered this tick.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerceptionSignal {
    /// One of `"sight"`, `"sight_lost"`, `"hearing"`, `"memory_decayed"`.
    pub kind: &'static str,
    /// Source actor id (player = the only signal source at M1.5).
    pub source_actor: Option<u64>,
    /// World position where the signal originated.
    pub source_position: Option<[f32; 2]>,
    /// Confidence in `[0.0, 1.0]`. Hearing decays linearly with distance.
    pub confidence: f32,
    /// Tick the signal fired. Useful for replay-viewer time-anchoring.
    pub tick: u64,
}

/// **M1.5 G4**: why an otherwise-valid shot missed. Stable vocabulary so
/// the replay viewer can render an icon set without string-typing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedShotReason {
    /// Roll exceeded the configured miss_chance.
    RecoilDeviation,
    /// The target moved between aim and trigger.
    TargetMoved,
    /// Occlusion entered the line between the guard and target.
    Occlusion,
    /// Player invoked something that made the shot lucky (sharp aim, dodge).
    LuckyDodge,
}

impl MissedShotReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissedShotReason::RecoilDeviation => "recoil_deviation",
            MissedShotReason::TargetMoved => "target_moved",
            MissedShotReason::Occlusion => "occlusion",
            MissedShotReason::LuckyDodge => "lucky_dodge",
        }
    }
}

/// **M1.5 G5**: stuck-recovery payload — emitted once on the tick the guard
/// crosses the stuck-tick threshold. `action` is the chosen recovery
/// strategy from the M1.5 set (M2+ adds `dig_through`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StuckRecoveryRecord {
    pub stuck_ticks: u32,
    pub blocker: &'static str,
    pub action: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetAcquiredRecord {
    pub target_actor: u64,
    pub via: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TargetLostRecord {
    pub target_actor: u64,
    pub reason: &'static str,
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

    // 1) Death check. A dead guard does nothing. A DYING guard ticks down
    //    its dwell and then transitions to Dead; while in DYING the guard
    //    cannot fire / move / re-acquire.
    //
    // **M1.5 G1**: mirror the actor body state machine so the AI surface
    // exposes the full death ladder (Engaged → Dying → Dead) for the
    // replay viewer to walk.
    if inputs.self_actor.status == Status::Dead || guard.state == GuardState::Dead {
        if guard.state != GuardState::Dead {
            let prev = guard.state;
            guard.state = GuardState::Dead;
            report.state_changed = Some(GuardStateTransition {
                previous: prev,
                next: GuardState::Dead,
                cause: "dying_dwell_elapsed",
            });
        }
        return report;
    }
    if inputs.self_actor.status == Status::Dying || guard.state == GuardState::Dying {
        if guard.state != GuardState::Dying {
            let prev = guard.state;
            guard.state = GuardState::Dying;
            guard.dying_dwell_remaining_ticks = guard.params.dying_dwell_ticks(inputs.tick_rate_hz);
            report.state_changed = Some(GuardStateTransition {
                previous: prev,
                next: GuardState::Dying,
                cause: "killed_by_player",
            });
            return report;
        }
        if guard.dying_dwell_remaining_ticks > 0 {
            guard.dying_dwell_remaining_ticks -= 1;
            if guard.dying_dwell_remaining_ticks == 0 {
                let prev = guard.state;
                guard.state = GuardState::Dead;
                report.state_changed = Some(GuardStateTransition {
                    previous: prev,
                    next: GuardState::Dead,
                    cause: "dying_dwell_elapsed",
                });
            }
        }
        return report;
    }
    // HP=0 with status not yet DYING (e.g. tutorial_safety policy demoted
    // the kill into the body machine but we observed it pre-promotion):
    // synthesise the transition AI-side so AI surface stays ahead of the
    // body's DYING gate.
    if inputs.self_actor.hp <= 0.0 && guard.state != GuardState::Dying {
        let prev = guard.state;
        guard.state = GuardState::Dying;
        guard.dying_dwell_remaining_ticks = guard.params.dying_dwell_ticks(inputs.tick_rate_hz);
        report.state_changed = Some(GuardStateTransition {
            previous: prev,
            next: GuardState::Dying,
            cause: "killed_by_player",
        });
        return report;
    }

    // Clear per-tick latches.
    guard.heard_alarm_this_tick = None;

    // **M1.5 G2 (hearing)**: consume alarms within hearing_radius. Pick the
    // closest source so guards with multiple simultaneous alarms produce a
    // deterministic single perception_signal.
    if guard.params.hearing_radius > 0.0 && !inputs.alarms.is_empty() {
        let self_pos = inputs.self_actor.position;
        let mut closest: Option<(f32, &AlarmInput)> = None;
        for alarm in inputs.alarms {
            let dx = alarm.source_position[0] - self_pos.x;
            let dy = alarm.source_position[1] - self_pos.y;
            let dist = (dx * dx + dy * dy).sqrt();
            // The alarm's loudness_radius is the source's outer envelope.
            // The guard's hearing_radius is the listener's inner envelope.
            // Hearing fires when dist ≤ MIN(alarm.loudness_radius, guard.hearing_radius).
            let effective_radius = alarm.loudness_radius.min(guard.params.hearing_radius);
            if dist <= effective_radius && closest.as_ref().is_none_or(|(d, _)| dist < *d) {
                closest = Some((dist, alarm));
            }
        }
        if let Some((dist, alarm)) = closest {
            // Hearing confidence decays linearly with distance: full at the
            // source, zero at the guard's hearing_radius.
            let confidence = if guard.params.hearing_radius > 0.0 {
                (1.0 - dist / guard.params.hearing_radius).clamp(0.0, 1.0)
            } else {
                0.0
            };
            guard.heard_alarm_this_tick = Some(alarm.source_position);
            guard.last_player_position = Some(alarm.source_position);
            guard.memory_last_refresh_tick = Some(inputs.tick);
            guard.alert_dwell_remaining_ticks = guard.params.alert_dwell_ticks(inputs.tick_rate_hz);
            report.perception_signals.push(PerceptionSignal {
                kind: "hearing",
                source_actor: Some(alarm.source_actor),
                source_position: Some(alarm.source_position),
                confidence,
                tick: inputs.tick,
            });
            // Hearing-without-LOS transitions Idle → Alert with reason
            // `"heard_shot"` (AI-H-01 contract). Guards already in Alert
            // / Engaged stay in their current state; the alarm refreshes
            // the alert_dwell timer above.
            if guard.state == GuardState::Idle {
                guard.state = GuardState::Alert;
                report.state_changed = Some(GuardStateTransition {
                    previous: GuardState::Idle,
                    next: GuardState::Alert,
                    cause: "heard_shot",
                });
            }
        }
    }

    // 2) Tick down cooldowns. Capture pre-decrement values for `alert_dwell_remaining_ticks`
    //    and `burst_pause_remaining_ticks` so that the state-machine + tactic checks below
    //    compare against the value the previous tick LEFT (not the value AFTER decrementing
    //    on this tick). Without this, `alert_dwell_seconds * tick_rate_hz = D` produces a
    //    D-1 effective dwell because the SET-tick's value is decremented before any check
    //    on the following tick. Same fix for burst_pause so the firing/scoring gates honor
    //    the configured pause duration end-to-end.
    let prev_alert_dwell_remaining_ticks = guard.alert_dwell_remaining_ticks;
    let prev_burst_pause_remaining_ticks = guard.burst_pause_remaining_ticks;
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

    // **M1.5 G1 / G3**: emit a sight perception_signal so cf-e2e can assert
    // `ai.perception_signal.count` / `last.payload.kind=sight`. Sight signals
    // fire every tick the guard sees the player; sight_lost fires once on
    // the transition tick.
    let player_visible_now = perception.as_ref().is_some_and(|p| p.player_seen);
    let player_was_visible = guard
        .last_player_seen_tick
        .is_some_and(|t| t == inputs.tick.saturating_sub(1));
    if let Some(p) = &perception {
        if p.player_seen {
            report.perception_signals.push(PerceptionSignal {
                kind: "sight",
                source_actor: inputs.player.map(|pl| pl.id.0),
                source_position: p.last_seen_position,
                confidence: 1.0,
                tick: inputs.tick,
            });
        } else if player_was_visible {
            report.perception_signals.push(PerceptionSignal {
                kind: "sight_lost",
                source_actor: inputs.player.map(|pl| pl.id.0),
                source_position: p.last_seen_position,
                confidence: 0.0,
                tick: inputs.tick,
            });
        }
    }

    // **M1.5 G3 (memory grid decay)**: if the guard's memory has been stale
    // for `memory_decay_ticks` AND there's no fresh perception this tick,
    // purge the memory and emit a `memory_decayed` signal.
    if guard.params.memory_decay_ticks > 0 && !player_visible_now && guard.heard_alarm_this_tick.is_none() {
        if let Some(last_refresh) = guard.memory_last_refresh_tick {
            let age = inputs.tick.saturating_sub(last_refresh);
            if age >= u64::from(guard.params.memory_decay_ticks) && guard.last_player_position.is_some() {
                let pos = guard.last_player_position.take();
                guard.memory_last_refresh_tick = None;
                report.perception_signals.push(PerceptionSignal {
                    kind: "memory_decayed",
                    source_actor: inputs.player.map(|pl| pl.id.0),
                    source_position: pos,
                    confidence: 0.0,
                    tick: inputs.tick,
                });
            }
        }
    }

    // 5) State machine. Transitions are reason-labelled so the recorder cause
    //    chain stays semantically valid.
    //
    // **M1.5 G1**: hp-driven Retreating gate fires before the perception
    // gate so a sighting at low hp keeps the guard in Retreating (it can
    // still engage from Retreating, but the state surface reflects the
    // wound). Recover at recover_hp_pct (hysteresis vs retreat_hp_pct).
    let hp_pct = if guard.max_hp > 0.0 {
        inputs.self_actor.hp / guard.max_hp
    } else {
        1.0
    };
    let should_retreat = hp_pct < guard.params.retreat_hp_pct;
    if should_retreat && guard.state != GuardState::Retreating {
        if matches!(guard.state, GuardState::Engaged | GuardState::Alert | GuardState::Idle) {
            let prev = guard.state;
            guard.state = GuardState::Retreating;
            report.state_changed = Some(GuardStateTransition {
                previous: prev,
                next: GuardState::Retreating,
                cause: "low_hp",
            });
        }
    } else if !should_retreat && hp_pct >= guard.params.recover_hp_pct && guard.state == GuardState::Retreating {
        let prev = guard.state;
        guard.state = if player_visible_now {
            GuardState::Engaged
        } else {
            GuardState::Alert
        };
        report.state_changed = Some(GuardStateTransition {
            previous: prev,
            next: guard.state,
            cause: "hp_recovered",
        });
    }
    if let Some(p) = &perception {
        if p.player_seen {
            guard.last_player_seen_tick = Some(inputs.tick);
            guard.last_player_position = p.last_seen_position;
            guard.memory_last_refresh_tick = Some(inputs.tick);
            guard.alert_dwell_remaining_ticks = guard.params.alert_dwell_ticks(inputs.tick_rate_hz);
            // First sighting starts the aim-settle timer.
            if guard.state != GuardState::Engaged {
                guard.aim_settle_remaining_ticks = guard.params.aim_settle_ticks(inputs.tick_rate_hz);
            }
            let prev = guard.state;
            // **M1.5 G1**: while Retreating with a visible player, stay in
            // Retreating (do NOT auto-promote to Engaged). The hp gate above
            // already promoted back to Engaged when hp recovered.
            if guard.state != GuardState::Retreating {
                guard.state = GuardState::Engaged;
                if prev != GuardState::Engaged {
                    report.state_changed = Some(GuardStateTransition {
                        previous: prev,
                        next: GuardState::Engaged,
                        cause: "player_visible",
                    });
                    if let Some(player) = inputs.player {
                        report.target_acquired = Some(TargetAcquiredRecord {
                            target_actor: player.id.0,
                            via: "sight",
                        });
                    }
                }
            }
        } else if prev_alert_dwell_remaining_ticks > 0 {
            let prev = guard.state;
            if guard.state == GuardState::Engaged {
                guard.state = GuardState::Alert;
                if prev != GuardState::Alert {
                    report.state_changed = Some(GuardStateTransition {
                        previous: prev,
                        next: GuardState::Alert,
                        cause: "player_lost",
                    });
                    if let Some(player) = inputs.player {
                        report.target_lost = Some(TargetLostRecord {
                            target_actor: player.id.0,
                            reason: "los_blocked",
                        });
                    }
                }
            }
        } else if guard.state != GuardState::Idle && guard.state != GuardState::Retreating {
            let prev = guard.state;
            guard.state = GuardState::Idle;
            report.state_changed = Some(GuardStateTransition {
                previous: prev,
                next: GuardState::Idle,
                cause: "alert_expired",
            });
        }
    }

    // **M1.5 G5**: stuck-recovery detector. While the guard is Alert /
    // Engaged AND can't see the player, increment stuck_ticks. Reset when
    // the player is visible OR when the guard fires successfully OR when
    // memory decays. When stuck_ticks crosses 60 (1 second @60Hz) the
    // engine emits ai.stuck_state_changed + ai.recovery_action and the
    // counter resets. Recovery action is `wait_then_search` at M1.5;
    // M2+ adds `dig_through` when chunked-terrain pathing lands.
    if matches!(
        guard.state,
        GuardState::Alert | GuardState::Engaged | GuardState::Retreating
    ) && !player_visible_now
    {
        guard.stuck_ticks = guard.stuck_ticks.saturating_add(1);
        if guard.stuck_ticks >= 60 && !guard.stuck_recovery_latched {
            guard.stuck_recovery_latched = true;
            report.stuck_recovery = Some(StuckRecoveryRecord {
                stuck_ticks: guard.stuck_ticks,
                blocker: "no_path",
                action: "wait_then_search",
                reason: "los_blocked_too_long",
            });
            // Reset so a second stuck window can fire later in the run.
            guard.stuck_ticks = 0;
        }
    } else {
        guard.stuck_ticks = 0;
        guard.stuck_recovery_latched = false;
    }

    // 6) Aim tracking. When a player is currently visible, aim straight at them.
    //    When alerted but not visible, aim at the last seen position.
    update_aim(guard, &perception, inputs.self_actor.position);

    // 7) Utility scoring → tactic choice.
    let player_visible = perception.as_ref().is_some_and(|p| p.player_seen);
    let player_distance = perception.as_ref().and_then(|p| p.distance);
    let scores = score_tactics(guard, player_visible, player_distance, prev_burst_pause_remaining_ticks);
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
            if let Some(fire) = try_fire(
                guard,
                inputs.self_actor,
                &perception,
                rng,
                inputs.tick_rate_hz,
                prev_burst_pause_remaining_ticks,
            ) {
                // **M1.5 G4**: when the shot will miss, attach a deterministic
                // reason label so the cause-chain viewer can render an icon
                // set rather than string-typing. The reason is bucketed
                // from the same miss_roll the rng produced, so identical
                // seeds produce identical reasons across runs.
                if fire.will_miss {
                    report.missed_shot_reason = Some(classify_miss_reason(fire.miss_roll));
                }
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

/// **M1.5 G4**: bucket a `[0, 1]` miss roll into one of four reason labels.
/// Same seed → same reason. Order picked so low rolls (close to threshold)
/// favour recoil_deviation (the "your finger slipped" miss); higher rolls
/// shift toward target_moved / occlusion / lucky_dodge (the "they did
/// something" misses).
fn classify_miss_reason(miss_roll: f32) -> MissedShotReason {
    let r = miss_roll.clamp(0.0, 0.9999);
    if r < 0.25 {
        MissedShotReason::RecoilDeviation
    } else if r < 0.50 {
        MissedShotReason::TargetMoved
    } else if r < 0.75 {
        MissedShotReason::Occlusion
    } else {
        MissedShotReason::LuckyDodge
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

fn score_tactics(
    guard: &ReactiveGuard,
    player_visible: bool,
    player_distance: Option<f32>,
    prev_burst_pause_remaining_ticks: u32,
) -> TacticScores {
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
        let burst_penalty = if prev_burst_pause_remaining_ticks > 0 {
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
    if guard.state == GuardState::Alert && !player_visible {
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
    prev_burst_pause_remaining_ticks: u32,
) -> Option<FireRecord> {
    if guard.aim_settle_remaining_ticks > 0 {
        return None;
    }
    if guard.fire_cooldown_ticks > 0 {
        return None;
    }
    if prev_burst_pause_remaining_ticks > 0 {
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
            alarms: &[],
        }
    }

    fn tick_inputs_with_alarms<'a>(
        tick: u64,
        guard_a: &'a ActorState,
        player: Option<&'a ActorState>,
        alarms: &'a [AlarmInput],
    ) -> GuardTickInputs<'a> {
        GuardTickInputs {
            tick,
            tick_rate_hz: 60,
            self_actor: guard_a,
            player,
            alarms,
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

    /// Regression: prior to this fix, `alert_dwell_remaining_ticks` was decremented
    /// at the top of `step()` BEFORE the state-machine check, so configuring
    /// `alert_dwell_seconds * tick_rate_hz = D` produced D-1 ticks of Alert
    /// dwell instead of D. Bugbot ID cf33d096-95e2-4104-bfe8-c9127c660223.
    #[test]
    fn alert_dwell_lasts_full_configured_duration_after_player_lost() {
        let mut params = ReactiveGuardParams::default();
        params.alert_dwell_seconds = 0.05; // 0.05 * 60 = 3 ticks of Alert.
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player_visible = player_actor(700.0, 32.0);
        // Out-of-cone player so perception still runs (player_seen=false).
        // Sight radius default is 700 in cf-ai params; place far behind the guard.
        let player_lost = player_actor(2000.0, 32.0);
        let mut rng = rng();

        // Tick 1: player visible -> state becomes Engaged, dwell SET to 3.
        let _ = step(&mut guard, tick_inputs(1, &actor, Some(&player_visible)), &mut rng);
        assert_eq!(guard.state, GuardState::Engaged);
        assert_eq!(guard.alert_dwell_remaining_ticks, 3);

        // Tick 2: player out-of-sight -> dwell decrements to 2, prev=3 > 0 keeps Alert.
        let _ = step(&mut guard, tick_inputs(2, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        // Tick 3: dwell decrements to 1, prev=2 > 0 keeps Alert.
        let _ = step(&mut guard, tick_inputs(3, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        // Tick 4: dwell decrements to 0, prev=1 > 0 keeps Alert (third tick of dwell).
        let _ = step(&mut guard, tick_inputs(4, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);

        // Tick 5: dwell stays at 0, prev=0 fails the > 0 check -> transitions to Idle.
        let _ = step(&mut guard, tick_inputs(5, &actor, Some(&player_lost)), &mut rng);
        assert_eq!(guard.state, GuardState::Idle);
    }

    /// Regression: same off-by-one decrement-before-check pattern affected
    /// `burst_pause_remaining_ticks` so a configured pause of D ticks gated
    /// firing for only D-1 ticks. Bugbot ID cf33d096-95e2-4104-bfe8-c9127c660223.
    ///
    /// `try_fire` always sets `fire_cooldown_ticks` to `seconds_to_ticks(0.20, 60) = 12`
    /// after a successful shot. We use `burst_pause_seconds = 0.30` (18 ticks)
    /// so the pause duration is strictly longer than the fire cooldown — the
    /// last 6 blocked ticks are isolated to burst_pause alone, which is what
    /// this test exercises.
    #[test]
    fn burst_pause_blocks_fire_for_full_configured_duration() {
        let mut params = ReactiveGuardParams::default();
        params.aim_settle_seconds = 0.0;
        params.miss_chance = 0.0;
        params.mag_capacity = 10;
        params.burst_shots = 1;
        params.burst_pause_seconds = 0.30; // 18 ticks of pause; > 12-tick fire cooldown.
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let player = player_actor(700.0, 32.0);
        let mut rng = Rng::from_seed(7);

        // Tick 1: aim_settle = 0 means instant settle, guard fires immediately and
        // burst_pause SETS to 18.
        let r1 = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert!(r1.fire.is_some(), "tick 1: zero aim_settle, must fire");
        assert_eq!(guard.burst_pause_remaining_ticks, 18);

        // Ticks 2-19: burst_pause must block for the full 18-tick duration.
        // (Ticks 2-13 are also blocked by fire_cooldown=12; ticks 14-19 are
        // blocked by burst_pause alone since fire_cooldown has cleared by then.)
        for tick in 2..=19 {
            let r = step(&mut guard, tick_inputs(tick, &actor, Some(&player)), &mut rng);
            assert!(
                r.fire.is_none(),
                "tick {tick}: burst_pause should block fire for the full 18-tick configured duration"
            );
        }

        // Tick 20: pause expired (prev=0); fire_cooldown also clear; guard fires again.
        let r20 = step(&mut guard, tick_inputs(20, &actor, Some(&player)), &mut rng);
        assert!(
            r20.fire.is_some(),
            "tick 20: pause + cooldown expired, fire should resume"
        );
    }

    /// **M1.5 G2 / AI-H-01**: Sentry hears a threat (offscreen rifle fire)
    /// and transitions Idle → Alert with reason="heard_shot" + perception
    /// signal kind="hearing".
    #[test]
    fn ai_h_01_sentry_hears_threat_without_los() {
        let mut params = ReactiveGuardParams::default();
        params.hearing_radius = 480.0;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        let actor = guard_actor();
        let mut rng = rng();
        // Player NOT in sight cone (this fixture has guard facing left and
        // the player isn't passed) — pure hearing path.
        let alarms = [AlarmInput {
            source_actor: 1,
            source_position: [actor.position.x + 200.0, actor.position.y],
            loudness_radius: 480.0,
        }];
        let report = step(&mut guard, tick_inputs_with_alarms(1, &actor, None, &alarms), &mut rng);
        assert_eq!(guard.state, GuardState::Alert);
        let transitioned = report.state_changed.expect("state must change on heard_shot");
        assert_eq!(transitioned.previous, GuardState::Idle);
        assert_eq!(transitioned.next, GuardState::Alert);
        assert_eq!(transitioned.cause, "heard_shot");
        let hearing = report
            .perception_signals
            .iter()
            .find(|s| s.kind == "hearing")
            .expect("hearing perception_signal must fire");
        assert_eq!(hearing.source_actor, Some(1));
        assert!(hearing.confidence > 0.0 && hearing.confidence <= 1.0);
    }

    /// **M1.5 G4**: `classify_miss_reason` is a pure function of the roll;
    /// identical seeds produce identical reasons.
    #[test]
    fn classify_miss_reason_buckets_are_stable() {
        assert_eq!(classify_miss_reason(0.0), MissedShotReason::RecoilDeviation);
        assert_eq!(classify_miss_reason(0.24), MissedShotReason::RecoilDeviation);
        assert_eq!(classify_miss_reason(0.26), MissedShotReason::TargetMoved);
        assert_eq!(classify_miss_reason(0.49), MissedShotReason::TargetMoved);
        assert_eq!(classify_miss_reason(0.51), MissedShotReason::Occlusion);
        assert_eq!(classify_miss_reason(0.74), MissedShotReason::Occlusion);
        assert_eq!(classify_miss_reason(0.76), MissedShotReason::LuckyDodge);
        assert_eq!(classify_miss_reason(0.99), MissedShotReason::LuckyDodge);
    }

    /// **M1.5 G1**: low-HP guard transitions to Retreating.
    #[test]
    fn low_hp_transitions_to_retreating() {
        let mut params = ReactiveGuardParams::default();
        params.retreat_hp_pct = 0.5;
        let mut guard = ReactiveGuard::new(ActorId(2), params);
        guard.max_hp = 100.0;
        let mut actor = guard_actor();
        actor.hp = 40.0; // 40% < 50% retreat gate
        let player = player_actor(80.0, 32.0); // visible to start
        let mut rng = rng();
        let report = step(&mut guard, tick_inputs(1, &actor, Some(&player)), &mut rng);
        assert_eq!(guard.state, GuardState::Retreating);
        let transitioned = report.state_changed.expect("hp gate must transition");
        assert_eq!(transitioned.cause, "low_hp");
        assert_eq!(transitioned.next, GuardState::Retreating);
    }
}
