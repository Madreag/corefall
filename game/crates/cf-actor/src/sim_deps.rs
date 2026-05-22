//! Per-tick step inputs: [`StepDeps`] and [`ActorTuning`].
//!
//! Extracted from [`crate::sim`] for file size; re-exported from `crate::sim`
//! so existing `cf_actor::sim::X` paths continue to work.

use serde::{Deserialize, Serialize};

/// Movement tuning. Hard-coded for the M1 actor; M5 will move these into the chassis
/// grammar so different chassis have different feel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActorTuning {
    pub max_speed: f32,
    pub ground_acceleration: f32,
    pub air_acceleration: f32,
    pub ground_friction: f32,
    pub jump_impulse: f32,
    pub terminal_velocity: f32,
    /// per-actor `recoil_decay_rate` field at construction. Defaulted on
    /// stale serialized bundles via `#[serde(default = ...)]`.
    #[serde(default = "default_recoil_decay_per_tick")]
    pub recoil_decay_per_tick: f32,
    #[serde(default = "default_sharp_aim_build_ticks")]
    pub sharp_aim_build_ticks: u32,
    /// "slow enough" gate.
    #[serde(default = "default_walk_threshold_tuning")]
    pub walk_threshold: f32,
}

fn default_recoil_decay_per_tick() -> f32 {
    0.05
}

fn default_sharp_aim_build_ticks() -> u32 {
    30
}

fn default_walk_threshold_tuning() -> f32 {
    1.5
}

impl Default for ActorTuning {
    fn default() -> Self {
        Self {
            max_speed: 220.0,
            ground_acceleration: 1500.0,
            air_acceleration: 600.0,
            ground_friction: 1200.0,
            jump_impulse: 420.0,
            terminal_velocity: -1800.0,
            recoil_decay_per_tick: default_recoil_decay_per_tick(),
            sharp_aim_build_ticks: default_sharp_aim_build_ticks(),
            walk_threshold: default_walk_threshold_tuning(),
        }
    }
}

/// Inputs for one [`crate::sim::step`] call.
///
/// When `tuning` is `None`, `ActorTuning::default()` is used (matches
/// historical M1 behaviour byte-for-byte).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct StepDeps {
    pub tick_dt: f32,
    pub region_min_x: f32,
    pub region_max_x: f32,
    /// Upper Y bound (in world units) for projectile out-of-bounds expiry. Derived
    /// from the scenario region height by the engine; the X-axis already uses
    /// `region_min_x` / `region_max_x`, and this mirrors the same data-driven
    /// pattern on the Y-axis instead of a hardcoded constant.
    pub region_max_y: f32,
    pub auto_reload_when_empty: bool,
    /// `None` for tests / callers that want the historical defaults).
    #[serde(default)]
    pub tuning: Option<ActorTuning>,
    /// for controllable actors so a tutorial player can never be punted to
    /// a restart screen by a single lethal hit (DR-023). The flag is
    /// sourced from the scenario manifest's `tutorial_safety` field via
    /// the engine; callers that want the M1 vanilla behaviour pass `false`.
    #[serde(default)]
    pub tutorial_safety: bool,
}
