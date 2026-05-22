//! Per-actor [`RifleState`] machine, tick inputs, and [`tick_rifle`] step.

use serde::{Deserialize, Serialize};

use crate::fire_mode::FireMode;
use crate::rifle_spec::RifleSpec;

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
    /// released. Prevents the next held-tick from re-firing. Cleared when
    /// `RifleTickInputs::fire_pressed=false`.
    #[serde(default)]
    pub semi_latched: bool,
    /// tracer cadence: shot index N produces a tracer when
    /// `(N % tracer_round_to_total_ratio) == (tracer_round_to_total_ratio - 1)`
    /// for non-zero ratios (so the LAST shot in each cycle is the tracer, per
    /// CCCP Magazine semantics). Reset to 0 by `reset()` and on reload completion.
    #[serde(default)]
    pub shot_index_in_mag: u32,
    /// counter; incremented on each successful reload. Used by the engine
    /// to synthesize a stable `magazine_id` (e.g. `<preset_id>:<index>`)
    /// for `equipment.weapon_reload_started` / `equipment.weapon_reload_completed`.
    /// Starts at 0 — the magazine the rifle ships with.
    #[serde(default)]
    pub magazine_index: u32,
}

impl RifleState {
    pub fn new(spec: RifleSpec, tick_rate_hz: u32) -> Self {
        Self {
            ammo_in_mag: spec.mag_capacity,
            fire_cooldown_ticks: 0,
            reload_remaining_ticks: 0,
            tick_rate_hz: tick_rate_hz.max(1),
            spec,
            semi_latched: false,
            shot_index_in_mag: 0,
            magazine_index: 0,
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
        self.semi_latched = false;
        self.shot_index_in_mag = 0;
    }

    /// emit a tracer projectile per `tracer_round_to_total_ratio`. Deterministic
    /// — same mag index always produces the same answer for the same ratio.
    pub fn next_shot_is_tracer(&self) -> bool {
        let ratio = self.spec.tracer_round_to_total_ratio;
        if ratio == 0 {
            return false;
        }
        // Tracer falls on every Nth shot starting at index `ratio - 1` so a
        // ratio of 4 produces tracers at shots 3, 7, 11, ... (one per group
        // of 4). Matches CCCP `Magazine::RTTRatio` cycling.
        (self.shot_index_in_mag + 1).is_multiple_of(ratio)
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
    /// Always false when no shot fired.
    #[serde(default)]
    pub fired_is_tracer: bool,
    /// at the moment a reload was initiated this tick. Engine emits
    /// `equipment.weapon_reload_started.reload_duration_ticks` from this.
    /// Zero when `reload_started=false`.
    #[serde(default)]
    pub reload_ticks_total: u32,
    /// (incremented on each successful reload). Drives the per-mag
    /// `magazine_id` exposed on `equipment.weapon_reload_started` /
    /// `equipment.weapon_reload_completed` so M10 replay viewers can group
    /// per-magazine shots.
    #[serde(default)]
    pub magazine_index_after: u32,
    /// the rifle silently gated it (reload in progress). Engine emits
    /// `control.command_rejected reason="reloading"` from this.
    #[serde(default)]
    pub fire_denied_reloading: bool,
}

impl TickOutcomes {
    pub const fn empty() -> Self {
        Self {
            fired_this_tick: false,
            reload_started: false,
            reload_completed: false,
            dry_fire: false,
            recoil_impulse_applied: 0.0,
            fired_is_tracer: false,
            reload_ticks_total: 0,
            magazine_index_after: 0,
            fire_denied_reloading: false,
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
///
/// the trigger is released so a held button fires exactly once. `FullAuto` fires
/// at `fire_interval_seconds` cadence while held.
#[must_use]
pub fn tick_rifle(state: &mut RifleState, inputs: RifleTickInputs) -> TickOutcomes {
    let mut outcomes = TickOutcomes::empty();

    // Release the semi-mode latch as soon as the trigger lifts; subsequent
    // presses are honored. Must run BEFORE the fire check so the very tick
    // the player releases doesn't get a free shot.
    if !inputs.fire_pressed {
        state.semi_latched = false;
    }

    // Advance reload counter first; finishing a reload this tick must take priority over
    // firing so the actor can shoot again on the very next tick.
    if state.reload_remaining_ticks > 0 {
        state.reload_remaining_ticks -= 1;
        if state.reload_remaining_ticks == 0 {
            state.ammo_in_mag = state.spec.mag_capacity;
            state.shot_index_in_mag = 0;
            state.magazine_index = state.magazine_index.saturating_add(1);
            outcomes.reload_completed = true;
            outcomes.magazine_index_after = state.magazine_index;
            // Reload finished this tick; the fire check below would otherwise see a
            // zero cooldown and fire on the same tick. Defer firing to the next tick
            // to match the documented "shoot again on the very next tick" semantics.
            // M1 re-audit pass 4: if the player also pressed fire this tick while
            // the reload was completing, surface the rejection so the engine can
            // emit `control.command_rejected reason="reloading"`.
            if inputs.fire_pressed {
                outcomes.fire_denied_reloading = true;
            }
            return outcomes;
        }
        // Reload still in progress; if the player pressed fire this tick the
        // rifle silently gates it. Surface the rejection.
        if inputs.fire_pressed {
            outcomes.fire_denied_reloading = true;
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
        outcomes.reload_ticks_total = state.reload_remaining_ticks;
    }

    if inputs.fire_pressed && !state.is_reloading() {
        if state.ammo_in_mag == 0 {
            outcomes.dry_fire = true;
        } else if state.fire_cooldown_ticks == 0 {
            // Gate the actual fire on fire_mode + latch.
            let allow_fire = match state.spec.fire_mode {
                FireMode::FullAuto => true,
                FireMode::Semi => !state.semi_latched,
            };
            if allow_fire {
                outcomes.fired_is_tracer = state.next_shot_is_tracer();
                state.ammo_in_mag -= 1;
                state.shot_index_in_mag = state.shot_index_in_mag.saturating_add(1);
                state.fire_cooldown_ticks = state.fire_interval_ticks();
                outcomes.fired_this_tick = true;
                outcomes.recoil_impulse_applied = state.spec.recoil_impulse;
                if state.spec.fire_mode == FireMode::Semi {
                    state.semi_latched = true;
                }
            }
        }
    }

    outcomes
}
