use serde::{Deserialize, Serialize};

use cf_actor::ActorId;

use crate::{GuardState, ReactiveGuardParams, Tactic};

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
    #[serde(default = "default_max_hp")]
    pub max_hp: f32,
    #[serde(default)]
    pub dying_dwell_remaining_ticks: u32,
    #[serde(default)]
    pub heard_alarm_this_tick: Option<[f32; 2]>,
    #[serde(default)]
    pub memory_last_refresh_tick: Option<u64>,
    #[serde(default)]
    pub stuck_ticks: u32,
    #[serde(default)]
    pub stuck_recovery_latched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_state_change_cause: Option<String>,
}

pub(crate) fn default_max_hp() -> f32 {
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
            last_state_change_cause: None,
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
