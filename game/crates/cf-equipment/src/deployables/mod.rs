//! M9C § "Bomb-disposal robot (T2 deployable; M44C IFV-grade
//! chassis)": deployable items with chassis grammar attached.
//!
//! The deployables namespace owns equipment items that, on use, place
//! a persistent entity in the world (rather than firing a projectile
//! or modifying terrain). M9C ships one deployable —
//! `bomb_disposal_robot` — but the module scaffold leaves room for
//! future deployables (squad-portable mortar tripod, etc.) without
//! reshaping the directory.

pub mod bomb_disposal_robot;

pub use bomb_disposal_robot::{
    bomb_disposal_robot_spec, BombDisposalRobotSpec, BombDisposalRobotState,
    BombDisposalRobotStatus, BOMB_DISPOSAL_ROBOT_CHASSIS_ID,
    BOMB_DISPOSAL_ROBOT_DRIVE_PX_PER_SECOND, BOMB_DISPOSAL_ROBOT_HP,
    BOMB_DISPOSAL_ROBOT_MECHANICAL_ARM_DISARM_SECONDS,
    BOMB_DISPOSAL_ROBOT_REACTIVE_ARMOR_REDUCTION_PERCENT,
};
