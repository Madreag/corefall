use serde::{Deserialize, Serialize};

use crate::Vec2;

/// Body status state machine (M1 surface; M3/M4/M5 expand wounds + chassis layers).
///
/// Lifecycle per CCCP `Actor.h:33`:
/// `Stable → Unstable → Downed → Dying → Dead` with `Inactive` as the
/// tutorial/cutscene escape hatch that pauses the state machine entirely.
///
/// `#[repr(u8)]` with explicit discriminants pins the layout used by
/// [`crate::ActorState::checksum_bytes`]. New variants (`Inactive=4`, `Dying=5`) are
/// **appended after `Dead=3`** to preserve existing checksum byte layout —
/// inserting them between `Downed` and `Dead` would silently shift every
/// pre-M1 bundle's checksum. Order in the enum body is for readability; the
/// numeric tag is what `checksum_bytes` records.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Stable = 0,
    Unstable = 1,
    Downed = 2,
    Dead = 3,
    Inactive = 4,
    Dying = 5,
}

impl Status {
    pub fn is_dead(self) -> bool {
        matches!(self, Status::Dead)
    }

    pub fn accepts_input(self) -> bool {
        matches!(self, Status::Stable | Status::Unstable)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Stable => "stable",
            Status::Unstable => "unstable",
            Status::Downed => "downed",
            Status::Dying => "dying",
            Status::Dead => "dead",
            Status::Inactive => "inactive",
        }
    }
}

/// Readable stance/locomotion state derived from per-tick actor state.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    Idle = 0,
    Walking = 1,
    Running = 2,
    Airborne = 3,
    Downed = 4,
    Dead = 5,
    Crouching = 6,
    Climbing = 7,
    Jetting = 8,
    Ejecting = 9,
    KnockedDown = 10,
    Stand = 11,
    Sprint = 12,
    CrouchWalk = 13,
    Prone = 14,
    ProneWalk = 15,
    Slide = 16,
    Vault = 17,
    Dive = 18,
    Lean = 19,
    Dying = 20,
    RopeClimb = 21,
    LadderClimb = 22,
    PipeClimb = 23,
    StealthAttack = 24,
    KnifeThrow = 25,
    Swim = 26,
    Crewing = 27,
    WallJump = 28,
    RopeHanging = 29,
    RopeSwinging = 30,
    Ziplining = 31,
    Mounted = 32,
    SwimSurface = 33,
    SwimSubmerged = 34,
}

impl Stance {
    pub const WALK_THRESHOLD: f32 = 8.0;
    pub const RUN_THRESHOLD: f32 = 60.0;

    pub fn as_str(self) -> &'static str {
        match self {
            Stance::Idle => "idle",
            Stance::Walking => "walking",
            Stance::Running => "running",
            Stance::Airborne => "airborne",
            Stance::Downed => "downed",
            Stance::Dead => "dead",
            Stance::Crouching => "crouching",
            Stance::Climbing => "climbing",
            Stance::Jetting => "jetting",
            Stance::Ejecting => "ejecting",
            Stance::KnockedDown => "knocked_down",
            Stance::Stand => "stand",
            Stance::Sprint => "sprint",
            Stance::CrouchWalk => "crouch_walk",
            Stance::Prone => "prone",
            Stance::ProneWalk => "prone_walk",
            Stance::Slide => "slide",
            Stance::Vault => "vault",
            Stance::Dive => "dive",
            Stance::Lean => "lean",
            Stance::Dying => "dying",
            Stance::RopeClimb => "rope_climb",
            Stance::LadderClimb => "ladder_climb",
            Stance::PipeClimb => "pipe_climb",
            Stance::StealthAttack => "stealth_attack",
            Stance::KnifeThrow => "knife_throw",
            Stance::Swim => "swim",
            Stance::Crewing => "crewing",
            Stance::WallJump => "wall_jump",
            Stance::RopeHanging => "rope_hanging",
            Stance::RopeSwinging => "rope_swinging",
            Stance::Ziplining => "ziplining",
            Stance::Mounted => "mounted",
            Stance::SwimSurface => "swim_surface",
            Stance::SwimSubmerged => "swim_submerged",
        }
    }

    pub fn locks_fire(self) -> bool {
        matches!(
            self,
            Stance::Slide
                | Stance::Vault
                | Stance::Dive
                | Stance::StealthAttack
                | Stance::KnifeThrow
                | Stance::Dead
                | Stance::Downed
                | Stance::Dying
                | Stance::KnockedDown
                | Stance::Ejecting
        )
    }

    pub fn from_state(velocity: Vec2, on_ground: bool, status: Status) -> Stance {
        match status {
            Status::Dead => Stance::Dead,
            Status::Dying | Status::Downed => Stance::Downed,
            Status::Inactive => Stance::Idle,
            Status::Stable | Status::Unstable => {
                if !on_ground {
                    Stance::Airborne
                } else {
                    let speed = velocity.x.abs();
                    if speed >= Self::RUN_THRESHOLD {
                        Stance::Running
                    } else if speed >= Self::WALK_THRESHOLD {
                        Stance::Walking
                    } else {
                        Stance::Idle
                    }
                }
            }
        }
    }

    #[allow(clippy::fn_params_excessive_bools)]
    pub fn from_chassis(
        velocity: Vec2,
        on_ground: bool,
        status: Status,
        crouch_active: bool,
        climb_active: bool,
        jet_active: bool,
        ejecting: bool,
    ) -> Stance {
        if ejecting {
            return Stance::Ejecting;
        }
        match status {
            Status::Dead => Stance::Dead,
            Status::Dying | Status::Downed => Stance::Downed,
            Status::Inactive => Stance::Idle,
            Status::Stable | Status::Unstable => {
                if jet_active {
                    return Stance::Jetting;
                }
                if climb_active {
                    return Stance::Climbing;
                }
                if !on_ground {
                    return Stance::Airborne;
                }
                if crouch_active {
                    return Stance::Crouching;
                }
                let speed = velocity.x.abs();
                if speed >= Self::RUN_THRESHOLD {
                    Stance::Running
                } else if speed >= Self::WALK_THRESHOLD {
                    Stance::Walking
                } else {
                    Stance::Idle
                }
            }
        }
    }
}
