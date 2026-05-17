//! M6C-8: Mortar crew-served operation.
//!
//! Gherkin scenario M6C-8:
//! ```text
//! Scenario: M6C-8 Mortar crew-served
//!   Given mortar_60mm requires 2 actors (gunner + loader)
//!   When solo actor attempts to fire:
//!     Then "Crew required" warning
//!   When second actor assists:
//!     Then mortar.crewed fires
//! ```
//!
//! The minimum crew size for a heavy weapon comes from
//! [`crate::heavy::HeavyWeaponPreset::crew_required`]. This module
//! provides the warning + crewed-event derivation.

use serde::{Deserialize, Serialize};

/// Reason returned when a solo actor tries to fire a crew-served mortar.
pub const CREW_REQUIRED_REASON: &str = "crew_required";

/// Result of a crew-fire attempt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrewFireOutcome {
    /// Solo / undermanned crew — reason returned for HUD warning.
    Rejected { reason: String, missing: u8 },
    /// Crewed and ready — engine should emit `mortar.crewed` and chamber the shot.
    Crewed { crew_size: u8 },
}

impl CrewFireOutcome {
    pub fn is_crewed(&self) -> bool {
        matches!(self, CrewFireOutcome::Crewed { .. })
    }
}

/// Evaluate a crew-served fire attempt. `crew_required` is the minimum
/// crew (e.g. 2 for `mortar_60mm`); `crew_present` is the number of
/// actors currently assisting (the gunner counts as 1).
pub fn evaluate_crew_fire(crew_required: u8, crew_present: u8) -> CrewFireOutcome {
    if crew_present < crew_required {
        CrewFireOutcome::Rejected {
            reason: CREW_REQUIRED_REASON.to_string(),
            missing: crew_required - crew_present,
        }
    } else {
        CrewFireOutcome::Crewed { crew_size: crew_present }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solo_fire_is_rejected_with_crew_required_reason() {
        // M6C-8 Scenario:
        //   When solo actor attempts to fire:
        //     Then "Crew required" warning
        let out = evaluate_crew_fire(2, 1);
        match out {
            CrewFireOutcome::Rejected { reason, missing } => {
                assert_eq!(reason, "crew_required");
                assert_eq!(missing, 1);
            }
            CrewFireOutcome::Crewed { .. } => panic!("expected rejection"),
        }
    }

    #[test]
    fn second_actor_assists_lets_mortar_fire() {
        // M6C-8 Scenario continued:
        //   When second actor assists:
        //     Then mortar.crewed fires
        let out = evaluate_crew_fire(2, 2);
        assert!(out.is_crewed());
        match out {
            CrewFireOutcome::Crewed { crew_size } => assert_eq!(crew_size, 2),
            CrewFireOutcome::Rejected { .. } => unreachable!(),
        }
    }

    #[test]
    fn single_crew_weapon_is_always_crewed() {
        let out = evaluate_crew_fire(1, 1);
        assert!(out.is_crewed());
    }
}
