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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RifleSpec {
    pub preset_id: String,
    /// Ticks between consecutive shots (cool-down). 6 ticks at 60 Hz = 10 RPS.
    pub fire_interval_ticks: u32,
    pub mag_capacity: u32,
    /// Ticks the actor spends reloading. 90 ticks at 60 Hz = 1.5 s.
    pub reload_ticks: u32,
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
    /// Hard ceiling on flight time so projectiles can't outlive the run if they miss.
    pub projectile_max_flight_ticks: u32,
}

/// Stable id for the M1 default rifle preset. Use [`rifle_preset`] to materialize the
/// owned [`RifleSpec`].
pub const RIFLE_M1_DEFAULT_ID: &str = "rifle_m1_default";

fn rifle_m1_default() -> RifleSpec {
    RifleSpec {
        preset_id: RIFLE_M1_DEFAULT_ID.to_string(),
        fire_interval_ticks: 6,
        mag_capacity: 30,
        reload_ticks: 90,
        recoil_impulse: 25.0,
        muzzle_forward_offset: 12.0,
        muzzle_vertical_offset: 4.0,
        projectile_speed: 1200.0,
        damage_per_hit: 12.0,
        projectile_max_flight_ticks: 90,
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

/// Per-actor rifle state machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RifleState {
    pub spec: RifleSpec,
    pub ammo_in_mag: u32,
    /// Ticks until the rifle can fire again. 0 = ready.
    pub fire_cooldown_ticks: u32,
    /// Ticks remaining in an in-progress reload. 0 = idle.
    pub reload_remaining_ticks: u32,
}

impl RifleState {
    pub fn new(spec: RifleSpec) -> Self {
        Self {
            ammo_in_mag: spec.mag_capacity,
            fire_cooldown_ticks: 0,
            reload_remaining_ticks: 0,
            spec,
        }
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
        }
    } else if state.fire_cooldown_ticks > 0 {
        state.fire_cooldown_ticks -= 1;
    }

    // Reload requested (or auto-reload when the magazine just emptied).
    let want_reload =
        inputs.reload_pressed || (inputs.auto_reload_when_empty && state.ammo_in_mag == 0 && !state.is_reloading());
    if want_reload && !state.is_reloading() && state.ammo_in_mag < state.spec.mag_capacity {
        state.reload_remaining_ticks = state.spec.reload_ticks;
        // Cancel the pending fire cooldown; reloading takes over.
        state.fire_cooldown_ticks = 0;
        outcomes.reload_started = true;
    }

    if inputs.fire_pressed && !state.is_reloading() {
        if state.ammo_in_mag == 0 {
            outcomes.dry_fire = true;
        } else if state.fire_cooldown_ticks == 0 {
            state.ammo_in_mag -= 1;
            state.fire_cooldown_ticks = state.spec.fire_interval_ticks;
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
        RifleState::new(rifle_preset(RIFLE_M1_DEFAULT_ID).expect("default preset"))
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
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        let mut r = rifle();
        let outcomes = tick_rifle(
            &mut r,
            RifleTickInputs {
                fire_pressed: true,
                ..Default::default()
            },
        );
        assert!(outcomes.fired_this_tick);
        assert_eq!(r.ammo_in_mag, spec.mag_capacity - 1);
        assert_eq!(r.fire_cooldown_ticks, spec.fire_interval_ticks);
    }

    #[test]
    fn cannot_fire_during_cooldown() {
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        let mut r = rifle();
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
        assert_eq!(r.ammo_in_mag, spec.mag_capacity - 1);
    }

    #[test]
    fn dry_fire_when_empty() {
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        let mut r = rifle();
        for _ in 0..spec.mag_capacity {
            for _ in 0..spec.fire_interval_ticks {
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
        let spec = rifle_preset(RIFLE_M1_DEFAULT_ID).unwrap();
        let mut r = rifle();
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
        for _ in 0..(spec.reload_ticks - 1) {
            let _ = tick_rifle(&mut r, RifleTickInputs::default());
            assert!(r.is_reloading());
        }
        let completion = tick_rifle(&mut r, RifleTickInputs::default());
        assert!(completion.reload_completed);
        assert_eq!(r.ammo_in_mag, spec.mag_capacity);
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
}
