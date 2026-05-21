use serde::{Deserialize, Serialize};

use crate::Vec2;

/// **M6**: side-view facing direction. Updated when the player aims; the
/// sprite renderer flips horizontally on change. M13 chassis adds armor
/// zone visibility per facing direction (spec § "Side-view facing direction
/// + limb-loss action restrictions (M13 forward-compat)").
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FacingDirection {
    Left = 0,
    Right = 1,
}

impl Default for FacingDirection {
    fn default() -> Self {
        FacingDirection::Right
    }
}

impl FacingDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            FacingDirection::Left => "left",
            FacingDirection::Right => "right",
        }
    }

    pub fn flipped(self) -> Self {
        match self {
            FacingDirection::Left => FacingDirection::Right,
            FacingDirection::Right => FacingDirection::Left,
        }
    }

    pub fn from_aim(aim: Vec2) -> Self {
        if aim.x >= 0.0 {
            FacingDirection::Right
        } else {
            FacingDirection::Left
        }
    }

    pub fn sign(self) -> f32 {
        match self {
            FacingDirection::Left => -1.0,
            FacingDirection::Right => 1.0,
        }
    }
}

/// **M6 (M13 forward-compat)**: per-limb loss tracking. M6 only sets these
/// flags via scenario seed / debug commands so the action-rejection surface
/// can be tested before M13 chassis ships full limb damage routing. Each
/// flag rejects a specific action category per spec § "Limb-loss action
/// restrictions".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct LimbLossFlags {
    pub both_arms_lost: bool,
    pub single_arm_lost: bool,
    pub both_legs_lost: bool,
    pub single_leg_lost: bool,
    pub both_hands_lost: bool,
    pub backpack_lost: bool,
    pub head_destroyed: bool,
    pub torso_destroyed: bool,
}

impl LimbLossFlags {
    pub fn weapon_fire_disabled(self) -> bool {
        self.both_arms_lost || self.both_hands_lost
    }

    pub fn two_hand_weapon_disabled(self) -> bool {
        self.single_arm_lost
    }

    pub fn movement_disabled(self) -> bool {
        self.both_legs_lost
    }

    pub fn sprint_disabled(self) -> bool {
        self.both_legs_lost || self.single_leg_lost
    }

    pub fn instant_death(self) -> bool {
        self.head_destroyed || self.torso_destroyed
    }

    pub fn reject_reason_for(self, action: &str) -> Option<&'static str> {
        match action {
            "fire" | "throw_grenade" | "knife_throw" => {
                if self.both_hands_lost {
                    Some("no_hands_for_grip")
                } else if self.both_arms_lost {
                    Some("no_arms_for_weapon")
                } else {
                    None
                }
            }
            "two_hand_fire" => {
                if self.single_arm_lost {
                    Some("single_arm_two_hand_weapon_rejected")
                } else {
                    None
                }
            }
            "sprint" | "jump" | "vault" | "climb_up" | "climb_down" => {
                if self.both_legs_lost {
                    Some("no_legs_for_movement")
                } else if self.single_leg_lost {
                    Some("single_leg_reduced_mobility")
                } else {
                    None
                }
            }
            "deploy_jet" => {
                if self.backpack_lost {
                    Some("backpack_lost_no_jet")
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// Stable per-actor id. Allocated by the scenario loader; future networking will
/// reuse the same id space across the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(pub u64);

impl ActorId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}
