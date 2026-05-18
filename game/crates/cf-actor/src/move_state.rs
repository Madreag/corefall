//! **M14A**: MoveState + ProneState + UpperBodyState enums.
//!
//! Mirrors CCCP `Actor.h` MovementState + AHuman ProneState + ArmsState. The
//! discriminants follow CCCP's ordering so RON-authored content + replay
//! events can round-trip stably.

use serde::{Deserialize, Serialize};

/// **M14A** § "CCCP MovementState enum → Corefall MoveState enum".
///
/// CCCP source: `Entities/Actor.h:33` (NOMOVE..MOVEMENTSTATECOUNT) + AHuman
/// extends with CROUCH/CRAWL/ARMCRAWL/CLIMB. Hover is Corefall-only for
/// drones.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveState {
    /// Idle non-moving baseline (CCCP NOMOVE).
    NoMove = 0,
    /// Upright standing tick (CCCP STAND).
    Stand = 1,
    /// Walking with foot alternation (CCCP WALK).
    Walk = 2,
    /// Crouching gait (CCCP CROUCH).
    Crouch = 3,
    /// Belly crawl (CCCP CRAWL).
    Crawl = 4,
    /// Both-legs-lost arm-dragging crawl (CCCP ARMCRAWL).
    ArmCrawl = 5,
    /// Climb path (CCCP CLIMB).
    Climb = 6,
    /// Jet/jump path (CCCP JUMP).
    Jump = 7,
    /// Dislodge path — used when stuck (CCCP DISLODGE).
    Dislodge = 8,
    /// Drone hover — Corefall-only, no CCCP equivalent.
    Hover = 9,
}

impl Default for MoveState {
    fn default() -> Self {
        MoveState::Stand
    }
}

impl MoveState {
    pub fn as_str(self) -> &'static str {
        match self {
            MoveState::NoMove => "no_move",
            MoveState::Stand => "stand",
            MoveState::Walk => "walk",
            MoveState::Crouch => "crouch",
            MoveState::Crawl => "crawl",
            MoveState::ArmCrawl => "arm_crawl",
            MoveState::Climb => "climb",
            MoveState::Jump => "jump",
            MoveState::Dislodge => "dislodge",
            MoveState::Hover => "hover",
        }
    }

    pub fn parse(s: &str) -> Option<MoveState> {
        match s {
            "no_move" => Some(MoveState::NoMove),
            "stand" => Some(MoveState::Stand),
            "walk" => Some(MoveState::Walk),
            "crouch" => Some(MoveState::Crouch),
            "crawl" => Some(MoveState::Crawl),
            "arm_crawl" => Some(MoveState::ArmCrawl),
            "climb" => Some(MoveState::Climb),
            "jump" => Some(MoveState::Jump),
            "dislodge" => Some(MoveState::Dislodge),
            "hover" => Some(MoveState::Hover),
            _ => None,
        }
    }

    /// Index into per-stance arrays such as
    /// [`crate::attitude::RotAngleTargets`].
    pub fn target_index(self) -> usize {
        self as usize
    }

    pub const COUNT: usize = 10;
}

/// **M14A** § "Prone state machine" — CCCP `AHuman.cpp:2674-2691`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProneState {
    /// Upright. No spring biasing toward flat.
    NotProne = 0,
    /// 333 ms transition window — strong spring pulls rotation toward flat.
    GoProne = 1,
    /// Flat on ground — 0.65 spring holds rotation near +π/2.
    Prone = 2,
}

impl Default for ProneState {
    fn default() -> Self {
        ProneState::NotProne
    }
}

impl ProneState {
    pub fn as_str(self) -> &'static str {
        match self {
            ProneState::NotProne => "not_prone",
            ProneState::GoProne => "go_prone",
            ProneState::Prone => "prone",
        }
    }
}

/// **M14A** § "ArmsState" — what the upper body is doing this tick. Used by
/// the held-device sway + 2-handed weapon support detection.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UpperBodyState {
    /// Empty hands / casual swing.
    Idle = 0,
    /// FG arm aimed weapon, BG arm idle.
    Aiming = 1,
    /// FG arm aimed, BG arm supporting a 2-handed device.
    TwoHandSupporting = 2,
    /// Mid-throw windup.
    Throwing = 3,
    /// Both arms acting as legs (arm crawl).
    ArmCrawling = 4,
    /// Climbing — both arms reach.
    Climbing = 5,
}

impl Default for UpperBodyState {
    fn default() -> Self {
        UpperBodyState::Idle
    }
}

impl UpperBodyState {
    pub fn as_str(self) -> &'static str {
        match self {
            UpperBodyState::Idle => "idle",
            UpperBodyState::Aiming => "aiming",
            UpperBodyState::TwoHandSupporting => "two_hand_supporting",
            UpperBodyState::Throwing => "throwing",
            UpperBodyState::ArmCrawling => "arm_crawling",
            UpperBodyState::Climbing => "climbing",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_state_round_trip() {
        for s in [
            MoveState::NoMove,
            MoveState::Stand,
            MoveState::Walk,
            MoveState::Crouch,
            MoveState::Crawl,
            MoveState::ArmCrawl,
            MoveState::Climb,
            MoveState::Jump,
            MoveState::Dislodge,
            MoveState::Hover,
        ] {
            assert_eq!(MoveState::parse(s.as_str()), Some(s));
            assert!(s.target_index() < MoveState::COUNT);
        }
    }

    #[test]
    fn prone_default_is_not_prone() {
        assert_eq!(ProneState::default(), ProneState::NotProne);
    }

    #[test]
    fn upper_body_default_is_idle() {
        assert_eq!(UpperBodyState::default(), UpperBodyState::Idle);
    }
}
