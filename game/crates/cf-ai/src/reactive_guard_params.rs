use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_retreat_hp_pct")]
    pub retreat_hp_pct: f32,
    #[serde(default = "default_recover_hp_pct")]
    pub recover_hp_pct: f32,
    #[serde(default = "default_hearing_radius")]
    pub hearing_radius: f32,
    #[serde(default = "default_memory_decay_ticks")]
    pub memory_decay_ticks: u32,
    #[serde(default = "default_dying_dwell_seconds")]
    pub dying_dwell_seconds: f32,
}

pub(crate) fn default_retreat_hp_pct() -> f32 {
    0.30
}
pub(crate) fn default_recover_hp_pct() -> f32 {
    0.35
}
pub(crate) fn default_hearing_radius() -> f32 {
    480.0
}
pub(crate) fn default_memory_decay_ticks() -> u32 {
    300
}
pub(crate) fn default_dying_dwell_seconds() -> f32 {
    1.0
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
    pub(crate) fn aim_settle_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.aim_settle_seconds, tick_rate_hz)
    }
    pub(crate) fn alert_dwell_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.alert_dwell_seconds, tick_rate_hz)
    }
    pub(crate) fn burst_pause_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.burst_pause_seconds, tick_rate_hz)
    }
    pub(crate) fn reload_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.reload_seconds, tick_rate_hz)
    }
    pub fn projectile_lifetime_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.projectile_lifetime_seconds, tick_rate_hz)
    }
    /// Mirrors the body state machine's DYING dwell (cf-actor's
    /// `dying_dwell_seconds`); the AI surface uses its own copy because
    /// the AI tick and the actor tick are independently invoked.
    pub fn dying_dwell_ticks(&self, tick_rate_hz: u32) -> u32 {
        seconds_to_ticks(self.dying_dwell_seconds, tick_rate_hz)
    }
}

pub(crate) fn seconds_to_ticks(seconds: f32, tick_rate_hz: u32) -> u32 {
    let rate = tick_rate_hz.max(1);
    let clamped = seconds.max(0.0);
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
