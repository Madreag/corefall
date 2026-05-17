//! M9C: engineering_tool — T2 engineer's tool capable of digging an
//! anti-tank ditch.
//!
//! Spec § "Anti-tank ditch + dragon's teeth + tank trap": "Anti-tank
//! ditch is a deeper authored carve than a trench — 8×4, mandates
//! `engineering_tool` (T2 from M30B) and 60s to dig."
//!
//! The engineering_tool also drives the AI engineer doctrine
//! (AI-ENG-A-03) when laying / repairing perimeter mines + wire.
//!
//! VAL-M9C-054 lands here.

use serde::{Deserialize, Serialize};

/// Canonical id under which the engineering tool is registered.
pub const ENGINEERING_TOOL_ID: &str = "engineering_tool";

/// Spec § AT ditch dig time: 60 in-game seconds.
pub const ENGINEERING_TOOL_AT_DITCH_SECONDS: u32 = 60;

/// On-disk + in-code spec for the engineering tool.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EngineeringToolSpec {
    pub id: String,
    pub display_name: String,
    /// Time in whole seconds required to carve an anti-tank ditch
    /// (spec: 60 s).
    pub at_ditch_dig_seconds: u32,
    pub tier: u8,
    pub mass_kg: f32,
    pub max_durability: f32,
}

#[must_use]
pub fn engineering_tool_m9c_default() -> EngineeringToolSpec {
    EngineeringToolSpec {
        id: ENGINEERING_TOOL_ID.to_string(),
        display_name: "Engineering Tool".to_string(),
        at_ditch_dig_seconds: ENGINEERING_TOOL_AT_DITCH_SECONDS,
        tier: 2,
        mass_kg: 4.5,
        max_durability: 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engineering_tool_default_matches_spec() {
        let e = engineering_tool_m9c_default();
        assert_eq!(e.id, ENGINEERING_TOOL_ID);
        assert_eq!(e.tier, 2);
        assert_eq!(e.at_ditch_dig_seconds, 60);
    }
}
