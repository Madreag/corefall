//! M6C-6: EVA suit + helmet vacuum seal.
//!
//! Gherkin scenario M6C-6:
//! ```text
//! Scenario: M6C-6 EVA suit + helmet seal vacuum
//!   Given player wearing eva_suit + sealed helmet in vacuum
//!   Then O2 supply from tank slot maintains breathing
//!   And no decompression damage
//! ```
//!
//! A vacuum environment is survivable when EVERY exposed slot
//! (body + helmet) carries a sealed PPE preset AND there is a positive
//! O2 supply in the actor's tank slot.

use serde::{Deserialize, Serialize};

/// Damage per second applied to the actor when the suit is breached in
/// vacuum (M19 atmos consumer).
pub const DECOMPRESSION_DAMAGE_PER_SECOND: f32 = 8.0;

/// O2 litres consumed per second while sealed.
pub const SEALED_O2_DRAIN_PER_SECOND_L: f32 = 0.5;

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct VacuumTickResult {
    /// True when the actor is currently surviving vacuum.
    pub sealed_and_supplied: bool,
    /// O2 consumed this tick (litres).
    pub o2_consumed_l: f32,
    /// Decompression damage applied this tick (HP).
    pub decompression_damage: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VacuumTickInputs {
    /// True when the actor's body slot carries a sealed PPE.
    pub body_sealed: bool,
    /// True when the actor's helmet slot is sealed.
    pub helmet_sealed: bool,
    /// True when ambient pressure is vacuum-equivalent.
    pub vacuum_environment: bool,
    /// Remaining O2 in the actor's tank slot (litres).
    pub o2_remaining_l: f32,
    /// Tick duration in seconds.
    pub dt_seconds: f32,
}

/// Apply one tick of vacuum exposure. Returns the o2 drained + any
/// decompression damage that should be applied to the actor.
pub fn tick_vacuum(inputs: VacuumTickInputs) -> VacuumTickResult {
    let mut out = VacuumTickResult::default();
    let dt = inputs.dt_seconds.max(0.0);
    if !inputs.vacuum_environment || dt == 0.0 {
        out.sealed_and_supplied = true;
        return out;
    }
    let sealed = inputs.body_sealed && inputs.helmet_sealed;
    let want_o2 = SEALED_O2_DRAIN_PER_SECOND_L * dt;
    let have_o2 = inputs.o2_remaining_l.max(0.0);
    let supplied = sealed && have_o2 >= want_o2;
    if supplied {
        out.sealed_and_supplied = true;
        out.o2_consumed_l = want_o2;
        out.decompression_damage = 0.0;
    } else {
        out.sealed_and_supplied = false;
        out.o2_consumed_l = have_o2.min(want_o2);
        out.decompression_damage = DECOMPRESSION_DAMAGE_PER_SECOND * dt;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sealed_with_supply_survives_vacuum() {
        // M6C-6 Scenario:
        //   Given player wearing eva_suit + sealed helmet in vacuum
        //   Then O2 supply from tank slot maintains breathing
        //   And no decompression damage
        let r = tick_vacuum(VacuumTickInputs {
            body_sealed: true,
            helmet_sealed: true,
            vacuum_environment: true,
            o2_remaining_l: 10.0,
            dt_seconds: 1.0,
        });
        assert!(r.sealed_and_supplied);
        assert!((r.o2_consumed_l - SEALED_O2_DRAIN_PER_SECOND_L).abs() < 1e-3);
        assert_eq!(r.decompression_damage, 0.0);
    }

    #[test]
    fn unsealed_body_triggers_decompression_damage() {
        let r = tick_vacuum(VacuumTickInputs {
            body_sealed: false,
            helmet_sealed: true,
            vacuum_environment: true,
            o2_remaining_l: 10.0,
            dt_seconds: 1.0,
        });
        assert!(!r.sealed_and_supplied);
        assert!(r.decompression_damage > 0.0);
    }

    #[test]
    fn empty_tank_triggers_decompression() {
        let r = tick_vacuum(VacuumTickInputs {
            body_sealed: true,
            helmet_sealed: true,
            vacuum_environment: true,
            o2_remaining_l: 0.0,
            dt_seconds: 1.0,
        });
        assert!(!r.sealed_and_supplied);
        assert!((r.decompression_damage - DECOMPRESSION_DAMAGE_PER_SECOND).abs() < 1e-3);
    }

    #[test]
    fn non_vacuum_environment_is_safe() {
        let r = tick_vacuum(VacuumTickInputs {
            body_sealed: false,
            helmet_sealed: false,
            vacuum_environment: false,
            o2_remaining_l: 0.0,
            dt_seconds: 1.0,
        });
        assert!(r.sealed_and_supplied);
        assert_eq!(r.o2_consumed_l, 0.0);
        assert_eq!(r.decompression_damage, 0.0);
    }
}
