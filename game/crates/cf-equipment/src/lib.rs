//! M1: minimal weapon presets and per-actor weapon state.
//!
//! Owns one canonical preset (id [`RIFLE_M1_DEFAULT_ID`], built via [`rifle_preset`]),
//! plus the [`RifleState`] machine that
//! the engine ticks each fixed step. The state machine emits structured outcomes
//! (`fired`, `reloaded`, `dry_fire`) that the caller turns into `weapon.*` events.
//!
//! Anti-scope (see crate AGENTS.md): no full role-record system here yet — that lands in
//! M5. M1 only needs a single rifle that fires, reloads, applies recoil, and reports
//! ammo / cooldown state to the HUD.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::derivable_impls,
    clippy::missing_const_for_fn
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Spec for one rifle preset. Loaded from a hard-coded registry in M1; M5 introduces
/// the full role-record schema (`cf-equipment::RoleRecord`) and a `content/equipment/`
/// data path.
///
/// Timings are stored in seconds, NOT ticks, so the same preset behaves identically
/// at 60 Hz and 120 Hz. Use [`RifleSpec::fire_interval_ticks`] etc. to derive tick
/// counts for the configured `tick_rate_hz`. This honours the AGENTS.md
/// "No-Compromise Performance Defaults" rule (no hardcoded 60 Hz constants).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RifleSpec {
    pub preset_id: String,
    /// Seconds between consecutive shots. `0.1` = 10 RPS.
    pub fire_interval_seconds: f32,
    pub mag_capacity: u32,
    /// Seconds the actor spends reloading. `1.5` = 1.5 s.
    pub reload_seconds: f32,
    /// Horizontal recoil impulse applied to the firer's velocity_x (units / s).
    pub recoil_impulse: f32,
    /// Distance forward of the actor centre to spawn the projectile (world units).
    pub muzzle_forward_offset: f32,
    /// Vertical offset above the actor centre (world units; positive = up).
    pub muzzle_vertical_offset: f32,
    /// Speed of the projectile (world units / s). Pure horizontal in M1; M5 wires aim.
    pub projectile_speed: f32,
    /// Damage applied to the first hit body (M1 keeps damage instantaneous; M5 routes
    /// through the chassis grammar).
    pub damage_per_hit: f32,
    /// Seconds of flight time before the projectile expires if it never hits.
    pub projectile_lifetime_seconds: f32,
}

impl RifleSpec {
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
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

    /// Ticks between consecutive shots at the given tick rate. Always ≥ 1.
    pub fn fire_interval_ticks(&self, tick_rate_hz: u32) -> u32 {
        Self::seconds_to_ticks(self.fire_interval_seconds, tick_rate_hz)
    }

    /// Ticks for one full reload at the given tick rate. Always ≥ 1.
    pub fn reload_ticks(&self, tick_rate_hz: u32) -> u32 {
        Self::seconds_to_ticks(self.reload_seconds, tick_rate_hz)
    }

    /// Maximum projectile flight ticks at the given tick rate. Always ≥ 1.
    pub fn projectile_max_flight_ticks(&self, tick_rate_hz: u32) -> u32 {
        Self::seconds_to_ticks(self.projectile_lifetime_seconds, tick_rate_hz)
    }
}

/// Stable id for the M1 default rifle preset. Use [`rifle_preset`] to materialize the
/// owned [`RifleSpec`].
pub const RIFLE_M1_DEFAULT_ID: &str = "rifle_m1_default";

fn rifle_m1_default() -> RifleSpec {
    RifleSpec {
        preset_id: RIFLE_M1_DEFAULT_ID.to_string(),
        fire_interval_seconds: 0.1,
        mag_capacity: 30,
        reload_seconds: 1.5,
        recoil_impulse: 25.0,
        muzzle_forward_offset: 12.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 1200.0,
        damage_per_hit: 12.0,
        projectile_lifetime_seconds: 1.5,
    }
}

/// All known presets. Keyed by `preset_id` for scenario lookup.
#[must_use]
pub fn rifle_presets() -> BTreeMap<&'static str, RifleSpec> {
    let mut m = BTreeMap::new();
    m.insert(RIFLE_M1_DEFAULT_ID, rifle_m1_default());
    m
}

/// Look up a preset by id; returns `None` if unknown so the engine can reject the
/// scenario before tick 0.
#[must_use]
pub fn rifle_preset(preset_id: &str) -> Option<RifleSpec> {
    rifle_presets().get(preset_id).cloned()
}

/// Per-actor rifle state machine. Carries the configured `tick_rate_hz` so timings
/// derived from `RifleSpec` (in seconds) resolve to a stable tick budget at both
/// 60 Hz and 120 Hz simulations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RifleState {
    pub spec: RifleSpec,
    /// Tick rate the engine ticks this rifle at; used to convert `spec.*_seconds`
    /// to tick counts. Always ≥ 1 (clamped at construction).
    pub tick_rate_hz: u32,
    pub ammo_in_mag: u32,
    /// Ticks until the rifle can fire again. 0 = ready.
    pub fire_cooldown_ticks: u32,
    /// Ticks remaining in an in-progress reload. 0 = idle.
    pub reload_remaining_ticks: u32,
}

impl RifleState {
    pub fn new(spec: RifleSpec, tick_rate_hz: u32) -> Self {
        Self {
            ammo_in_mag: spec.mag_capacity,
            fire_cooldown_ticks: 0,
            reload_remaining_ticks: 0,
            tick_rate_hz: tick_rate_hz.max(1),
            spec,
        }
    }

    /// Cool-down after a shot, in ticks at this rifle's configured tick rate.
    pub fn fire_interval_ticks(&self) -> u32 {
        self.spec.fire_interval_ticks(self.tick_rate_hz)
    }

    /// Full reload duration in ticks at this rifle's configured tick rate.
    pub fn reload_ticks(&self) -> u32 {
        self.spec.reload_ticks(self.tick_rate_hz)
    }

    /// Maximum projectile flight in ticks at this rifle's configured tick rate.
    pub fn projectile_max_flight_ticks(&self) -> u32 {
        self.spec.projectile_max_flight_ticks(self.tick_rate_hz)
    }

    pub fn ready_to_fire(&self) -> bool {
        self.fire_cooldown_ticks == 0 && self.reload_remaining_ticks == 0 && self.ammo_in_mag > 0
    }

    pub fn is_reloading(&self) -> bool {
        self.reload_remaining_ticks > 0
    }

    /// Reset ammo + cooldowns. Used by `act.player.reset` and scenario reload.
    pub fn reset(&mut self) {
        self.ammo_in_mag = self.spec.mag_capacity;
        self.fire_cooldown_ticks = 0;
        self.reload_remaining_ticks = 0;
    }
}

/// Outcomes of one tick of the rifle state machine. Converted to recorder events by the
/// caller; the data needed for `weapon_fired`, `weapon_reloaded`, etc. is included here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TickOutcomes {
    pub fired_this_tick: bool,
    pub reload_started: bool,
    pub reload_completed: bool,
    pub dry_fire: bool,
    pub recoil_impulse_applied: f32,
}

impl TickOutcomes {
    pub const fn empty() -> Self {
        Self {
            fired_this_tick: false,
            reload_started: false,
            reload_completed: false,
            dry_fire: false,
            recoil_impulse_applied: 0.0,
        }
    }
}

/// Inputs for [`tick_rifle`]. `fire_pressed` and `reload_pressed` are edge-triggered
/// per `cf-actor::ControlIntent`; the caller clears them after the tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RifleTickInputs {
    pub fire_pressed: bool,
    pub reload_pressed: bool,
    pub auto_reload_when_empty: bool,
}

impl Default for RifleTickInputs {
    fn default() -> Self {
        Self {
            fire_pressed: false,
            reload_pressed: false,
            auto_reload_when_empty: false,
        }
    }
}

/// One fixed-tick step of the rifle. Returns the outcomes the caller should turn into
/// recorder events, plus any recoil impulse the caller should apply to the firer.
#[must_use]
pub fn tick_rifle(state: &mut RifleState, inputs: RifleTickInputs) -> TickOutcomes {
    let mut outcomes = TickOutcomes::empty();

    // Advance reload counter first; finishing a reload this tick must take priority over
    // firing so the actor can shoot again on the very next tick.
    if state.reload_remaining_ticks > 0 {
        state.reload_remaining_ticks -= 1;
        if state.reload_remaining_ticks == 0 {
            state.ammo_in_mag = state.spec.mag_capacity;
            outcomes.reload_completed = true;
            // Reload finished this tick; the fire check below would otherwise see a
            // zero cooldown and fire on the same tick. Defer firing to the next tick
            // to match the documented "shoot again on the very next tick" semantics.
            return outcomes;
        }
    } else if state.fire_cooldown_ticks > 0 {
        state.fire_cooldown_ticks -= 1;
    }

    // Reload requested (or auto-reload when the magazine just emptied).
    let want_reload =
        inputs.reload_pressed || (inputs.auto_reload_when_empty && state.ammo_in_mag == 0 && !state.is_reloading());
    if want_reload && !state.is_reloading() && state.ammo_in_mag < state.spec.mag_capacity {
        state.reload_remaining_ticks = state.reload_ticks();
        // Cancel the pending fire cooldown; reloading takes over.
        state.fire_cooldown_ticks = 0;
        outcomes.reload_started = true;
    }

    if inputs.fire_pressed && !state.is_reloading() {
        if state.ammo_in_mag == 0 {
            outcomes.dry_fire = true;
        } else if state.fire_cooldown_ticks == 0 {
            state.ammo_in_mag -= 1;
            state.fire_cooldown_ticks = state.fire_interval_ticks();
            outcomes.fired_this_tick = true;
            outcomes.recoil_impulse_applied = state.spec.recoil_impulse;
        }
    }

    outcomes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rifle() -> RifleState {
        rifle_at(60)
    }

    fn rifle_at(tick_rate_hz: u32) -> RifleState {
        RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).expect("default preset"), tick_rate_hz)
    }

    #[test]
    fn rifle_starts_loaded_and_ready() {
        let r = rifle();
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        assert!(r.ready_to_fire());
        assert_eq!(r.ammo_in_mag, spec.mag_capacity);
    }

    #[test]
    fn fire_decrements_ammo_and_starts_cooldown() {
        let mut r = rifle();
        let cooldown = r.fire_interval_ticks();
        let mag = r.spec.mag_capacity;
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(outcomes.fired_this_tick);
        assert_eq!(r.ammo_in_mag, mag - 1);
        assert_eq!(r.fire_cooldown_ticks, cooldown);
    }

    #[test]
    fn cannot_fire_during_cooldown() {
        let mut r = rifle();
        let mag = r.spec.mag_capacity;
        let _ = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        let blocked = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(!blocked.fired_this_tick);
        assert_eq!(r.ammo_in_mag, mag - 1);
    }

    #[test]
    fn dry_fire_when_empty() {
        let mut r = rifle();
        let mag = r.spec.mag_capacity;
        let cooldown = r.fire_interval_ticks();
        for _ in 0..mag {
            for _ in 0..cooldown {
                let _ = tick_rifle(
                    &mut r,
                    RifleTickInputs {
                        fire_pressed: false,
                        ..Default::default()
                    },
                );
            }
            let _ = tick_rifle(
                &mut r,
                RifleTickInputs {
                    fire_pressed: true,
                    ..Default::default()
                },
            );
        }
        assert_eq!(r.ammo_in_mag, 0);
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(outcomes.dry_fire);
        assert!(!outcomes.fired_this_tick);
    }

    #[test]
    fn reload_takes_full_duration() {
        let mut r = rifle();
        let mag = r.spec.mag_capacity;
        let reload = r.reload_ticks();
        let _ = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        let started = tick_rifle(
            &mut r,
            RifleTickInputs {
                reload_pressed: true,
                ..Default::default()
            },
        );
        assert!(started.reload_started);
        for _ in 0..(reload - 1) {
            let _ = tick_rifle(&mut r, RifleTickInputs::default());
            assert!(r.is_reloading());
        }
        let completion = tick_rifle(&mut r, RifleTickInputs::default());
        assert!(completion.reload_completed);
        assert_eq!(r.ammo_in_mag, mag);
        assert!(!r.is_reloading());
    }

    #[test]
    fn auto_reload_when_empty_starts_after_dry_fire() {
        let mut r = rifle();
        r.ammo_in_mag = 0;
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: false,
                reload_pressed: false,
                auto_reload_when_empty: true,
            },
        );
        assert!(outcomes.reload_started);
        assert!(r.is_reloading());
    }

    #[test]
    fn reset_returns_full_mag() {
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        let mut r = rifle();
        r.ammo_in_mag = 5;
        r.fire_cooldown_ticks = 3;
        r.reload_remaining_ticks = 30;
        r.reset();
        assert_eq!(r.ammo_in_mag, spec.mag_capacity);
        assert_eq!(r.fire_cooldown_ticks, 0);
        assert_eq!(r.reload_remaining_ticks, 0);
    }

    #[test]
    fn rifle_preset_lookup() {
        assert!(rifle_preset(RIFLE_M1_DEFAULT_ID).is_some());
        assert!(rifle_preset("nonexistent").is_none());
    }

    #[test]
    fn timings_scale_with_tick_rate() {
        // 10 RPS / 1.5 s reload / 1.5 s flight at the canonical M1 preset.
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        // 60 Hz: 6 / 90 / 90.
        assert_eq!(spec.fire_interval_ticks(60), 6);
        assert_eq!(spec.reload_ticks(60), 90);
        assert_eq!(spec.projectile_max_flight_ticks(60), 90);
        // 120 Hz: 12 / 180 / 180.
        assert_eq!(spec.fire_interval_ticks(120), 12);
        assert_eq!(spec.reload_ticks(120), 180);
        assert_eq!(spec.projectile_max_flight_ticks(120), 180);
        // RifleState resolves the same values via its configured tick_rate_hz.
        let r60 = rifle_at(60);
        let r120 = rifle_at(120);
        assert_eq!(r60.fire_interval_ticks(), 6);
        assert_eq!(r120.fire_interval_ticks(), 12);
        assert_eq!(r60.reload_ticks(), 90);
        assert_eq!(r120.reload_ticks(), 180);
    }

    #[test]
    fn fire_rate_real_time_equivalent_at_60hz_and_120hz() {
        // Drive both 60 Hz and 120 Hz rifles for the same wall-clock window and
        // assert the same number of shots fired. Window: 1.0 s -> exactly 10 shots
        // at 10 RPS. At 60 Hz that's 60 ticks; at 120 Hz that's 120 ticks.
        fn shots_in_window(tick_rate_hz: u32, ticks: u32) -> u32 {
            let mut r = rifle_at(tick_rate_hz);
            let mut shots = 0;
            for _ in 0..ticks {
                let outcomes = tick_rifle(
                    &mut r,
                    RifleTickInputs {
                        fire_pressed: true,
                        ..Default::default()
                    },
                );
                if outcomes.fired_this_tick {
                    shots += 1;
                }
            }
            shots
        }
        let shots_60 = shots_in_window(60, 60);
        let shots_120 = shots_in_window(120, 120);
        assert_eq!(shots_60, shots_120, "10 RPS must hold across tick rates");
        // Should be 10 shots in 1 s for 10 RPS (one shot per fire_interval, ~10 shots).
        assert!(
            (9..=11).contains(&shots_60),
            "expected ~10 RPS, got {shots_60} at 60 Hz"
        );
    }
}
