//! M6C-4: Flamethrower fuel canister coupling.
//!
//! Gherkin scenario M6C-4:
//! ```text
//! Scenario: M6C-4 Flamethrower fuel canister coupling
//!   Given flamethrower in primary + fuel_canister in tank_utility
//!   When player fires:
//!     Then fuel drains from canister
//!     And fire material spawns per M15
//! ```
//!
//! Fuel consumption: `FUEL_PER_BURST_LITERS` per trigger-held second; the
//! flame width and fire-material spawn count both scale with remaining
//! fuel pressure so an empty canister disables the weapon.

use serde::{Deserialize, Serialize};

/// Litres of fuel drained per second of continuous trigger-held fire.
pub const FUEL_PER_SECOND_LITERS: f32 = 0.6;

/// Fire material tiles spawned per second of continuous fire (M15
/// consumer). Each tile is a [`FireSpawnTick`] entry.
pub const FIRE_TILES_PER_SECOND: u32 = 6;

/// Per-tile fire intensity (M15 consumer; max 1.0).
pub const FIRE_TILE_INTENSITY: f32 = 0.85;

/// Maximum operating pressure at full canister (1.0 = full bar).
pub const FUEL_PRESSURE_FULL: f32 = 1.0;

/// Minimum operating pressure (below this the regulator cuts off).
pub const FUEL_PRESSURE_CUTOFF: f32 = 0.05;

/// One spawned fire tile (M15 fire material consumer surface).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireSpawnTick {
    pub x: f32,
    pub y: f32,
    pub intensity: f32,
}

/// Outcome of one tick of [`FlamethrowerState::tick`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FlamethrowerTickOutcome {
    /// True when the weapon emitted at least one fire-material tile this tick.
    pub fired_this_tick: bool,
    /// Liters consumed this tick.
    pub fuel_consumed_l: f32,
    /// Fire tiles spawned this tick. Engine forwards to M15 material spawn.
    pub fire_spawns: Vec<FireSpawnTick>,
    /// True when the canister ran dry this tick (engine emits
    /// `equipment.fuel_canister_empty`).
    pub canister_empty: bool,
}

/// Persistent flamethrower state. Drives [`tick`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FlamethrowerState {
    /// Remaining fuel in the coupled canister (litres).
    pub fuel_remaining_l: f32,
    /// Maximum canister capacity (litres).
    pub canister_capacity_l: f32,
}

impl FlamethrowerState {
    pub fn new(canister_capacity_l: f32) -> Self {
        let cap = canister_capacity_l.max(0.0);
        Self {
            fuel_remaining_l: cap,
            canister_capacity_l: cap,
        }
    }

    /// Current canister pressure ratio in [0, 1].
    pub fn pressure(&self) -> f32 {
        if self.canister_capacity_l <= 0.0 {
            return 0.0;
        }
        (self.fuel_remaining_l / self.canister_capacity_l).clamp(0.0, 1.0)
    }

    /// True when the regulator allows the weapon to fire.
    pub fn can_fire(&self) -> bool {
        self.pressure() > FUEL_PRESSURE_CUTOFF
    }

    /// Refill the canister (engine swaps the tank slot).
    pub fn refill(&mut self, liters: f32) {
        self.fuel_remaining_l = (self.fuel_remaining_l + liters.max(0.0)).min(self.canister_capacity_l);
    }

    /// Per-tick advance. `trigger_held` true while the player holds fire.
    /// `dt_seconds` is the tick duration. `nozzle` is the world-space muzzle
    /// position used to seed fire tile positions; `aim_unit` is the unit aim
    /// direction vector.
    pub fn tick(
        &mut self,
        trigger_held: bool,
        dt_seconds: f32,
        nozzle: (f32, f32),
        aim_unit: (f32, f32),
    ) -> FlamethrowerTickOutcome {
        let mut out = FlamethrowerTickOutcome::default();
        let dt = dt_seconds.max(0.0);
        if !trigger_held || !self.can_fire() || dt == 0.0 {
            return out;
        }
        let want_fuel = FUEL_PER_SECOND_LITERS * dt;
        let consumed = want_fuel.min(self.fuel_remaining_l);
        if consumed <= 0.0 {
            return out;
        }
        self.fuel_remaining_l -= consumed;
        out.fired_this_tick = true;
        out.fuel_consumed_l = consumed;

        let pressure_ratio = (consumed / want_fuel.max(1e-6)).clamp(0.0, 1.0);
        let tile_count = ((FIRE_TILES_PER_SECOND as f32) * dt * pressure_ratio).round().max(0.0) as u32;
        for i in 0..tile_count {
            let stride = i as f32 + 1.0;
            out.fire_spawns.push(FireSpawnTick {
                x: nozzle.0 + aim_unit.0 * stride,
                y: nozzle.1 + aim_unit.1 * stride,
                intensity: FIRE_TILE_INTENSITY * self.pressure().max(0.1),
            });
        }
        // Snap to zero when fuel drops below the regulator cutoff so the
        // weapon can't keep "trickling" at sub-cutoff pressure. The
        // `canister_empty` flag fires on the transition tick.
        let cutoff = FUEL_PRESSURE_CUTOFF * self.canister_capacity_l;
        if self.fuel_remaining_l <= cutoff {
            self.fuel_remaining_l = 0.0;
            out.canister_empty = true;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_canister_cannot_fire() {
        let s = FlamethrowerState::new(0.0);
        assert!(!s.can_fire());
    }

    #[test]
    fn fire_drains_fuel_and_spawns_tiles() {
        // M6C-4 Scenario:
        //   When player fires:
        //     Then fuel drains from canister
        //     And fire material spawns per M15
        let mut s = FlamethrowerState::new(5.0);
        let out = s.tick(true, 1.0, (10.0, 0.0), (1.0, 0.0));
        assert!(out.fired_this_tick);
        assert!(out.fuel_consumed_l > 0.0);
        assert!(!out.fire_spawns.is_empty());
        assert!(s.fuel_remaining_l < 5.0);
    }

    #[test]
    fn release_stops_consumption() {
        let mut s = FlamethrowerState::new(5.0);
        let _ = s.tick(true, 0.5, (0.0, 0.0), (1.0, 0.0));
        let before = s.fuel_remaining_l;
        let out = s.tick(false, 1.0, (0.0, 0.0), (1.0, 0.0));
        assert!(!out.fired_this_tick);
        assert_eq!(s.fuel_remaining_l, before);
    }

    #[test]
    fn canister_empty_signal_fires_when_drained() {
        let mut s = FlamethrowerState::new(0.6);
        // 1 second at 0.6 L/s exhausts the canister.
        let mut empty = false;
        for _ in 0..120 {
            let out = s.tick(true, 1.0 / 60.0, (0.0, 0.0), (1.0, 0.0));
            if out.canister_empty {
                empty = true;
                break;
            }
        }
        assert!(empty);
        assert_eq!(s.fuel_remaining_l, 0.0);
        assert!(!s.can_fire());
    }

    #[test]
    fn refill_brings_pressure_back() {
        let mut s = FlamethrowerState::new(5.0);
        s.fuel_remaining_l = 0.0;
        assert!(!s.can_fire());
        s.refill(3.0);
        assert!(s.can_fire());
        assert!((s.pressure() - 0.6).abs() < 1e-3);
    }

    #[test]
    fn fire_spawns_align_with_aim() {
        let mut s = FlamethrowerState::new(2.0);
        let out = s.tick(true, 1.0, (5.0, 10.0), (0.0, 1.0));
        assert!(out.fire_spawns.iter().all(|t| (t.x - 5.0).abs() < 1e-3));
        assert!(out.fire_spawns.iter().all(|t| t.y > 10.0));
    }
}
