//! M7-A: Autonomy mode + Squad doctrine enums.
//!
//! Autonomy mode (per actor) controls how aggressively the 5-layer thinking
//! stack acts without explicit player orders. Squad doctrine (per squad)
//! biases the Utility scorer's task weights for all members.
//!
//! M7-B owns the cfctl methods that set/observe these (see spec § Smart
//! commandable AI — Three-layer control model). M7-A ships the types so
//! the Utility scorer can already consume them.

use serde::{Deserialize, Serialize};

/// **M7-A**: per-actor autonomy mode. Set once per actor or globally; persists
/// across mission saves in M25 campaign.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AutonomyMode {
    /// Bot does everything based on role + priority table; minimal player
    /// input needed. Player commands override when issued.
    #[default]
    FullAuto,
    /// Bot auto-acts on top 3 highest-priority tasks; medium/low-priority
    /// tasks wait for player nudge.
    Standard,
    /// Bot waits for explicit orders only; default auto-behaviors disabled.
    Manual,
}

impl AutonomyMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AutonomyMode::FullAuto => "full_auto",
            AutonomyMode::Standard => "standard",
            AutonomyMode::Manual => "manual",
        }
    }

    pub fn from_str(value: &str) -> Option<AutonomyMode> {
        Some(match value {
            "full_auto" => AutonomyMode::FullAuto,
            "standard" => AutonomyMode::Standard,
            "manual" => AutonomyMode::Manual,
            _ => return None,
        })
    }

    /// Returns the maximum number of utility-scorer ranks the bot will
    /// auto-act on without an explicit order. FullAuto = unbounded;
    /// Standard = top-3; Manual = zero.
    pub fn auto_action_cap(self) -> usize {
        match self {
            AutonomyMode::FullAuto => usize::MAX,
            AutonomyMode::Standard => 3,
            AutonomyMode::Manual => 0,
        }
    }
}

/// **M7**: per-squad doctrine. Biases every member's Utility scorer.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DoctrineMode {
    /// Hold cover positions; suppression fire prioritized.
    #[default]
    Defensive,
    /// Push toward player; less suppression; flanking.
    Aggressive,
    /// One member explores ahead while others wait.
    Scout,
}

impl DoctrineMode {
    pub fn as_str(self) -> &'static str {
        match self {
            DoctrineMode::Defensive => "defensive",
            DoctrineMode::Aggressive => "aggressive",
            DoctrineMode::Scout => "scout",
        }
    }

    pub fn from_str(value: &str) -> Option<DoctrineMode> {
        Some(match value {
            "defensive" => DoctrineMode::Defensive,
            "aggressive" => DoctrineMode::Aggressive,
            "scout" => DoctrineMode::Scout,
            _ => return None,
        })
    }
}
