use serde::{Deserialize, Serialize};

/// Named body zones. M5 ships the full 14-zone limb model the roadmap M5
/// section requires: head, torso, upper-arm / forearm / hand (left + right),
/// thigh / shin / foot (left + right), and a backpack/jetpack slot.
/// `ArmLeft` / `ArmRight` / `LegLeft` / `LegRight` remain as **composite**
/// aliases (the upper limb in their respective chain) so legacy zone-from-hit
/// resolution + AI heuristics + checksum byte layout stay stable; the new
/// granular variants are appended to the enum so cross-milestone determinism
/// is preserved per the `repr(u8)` discriminant order.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BodyZone {
    Head = 0,
    Torso = 1,
    /// Composite alias for the upper-arm chain (left). Treated as the upper
    /// arm during zone resolution; granular `ForearmLeft`/`HandLeft` are
    /// addressable separately.
    ArmLeft = 2,
    ArmRight = 3,
    LegLeft = 4,
    LegRight = 5,
    Backpack = 6,
    ForearmLeft = 7,
    ForearmRight = 8,
    HandLeft = 9,
    HandRight = 10,
    ShinLeft = 11,
    ShinRight = 12,
    FootLeft = 13,
    FootRight = 14,
    // **M13** § "Quadruped=11 zones (4 legs + 2 claws + torso + sensor cluster + carapace)".
    // Front-left/right + rear-left/right legs; left/right claws; carapace shell; sensor cluster.
    LegFrontLeft = 15,
    LegFrontRight = 16,
    LegRearLeft = 17,
    LegRearRight = 18,
    ClawLeft = 19,
    ClawRight = 20,
    Carapace = 21,
    SensorCluster = 22,
    // **M13** § "Drone=4 zones (chassis + 2 arms + sensor pod)".
    DroneCore = 23,
    DroneArmLeft = 24,
    DroneArmRight = 25,
    DroneSensorPod = 26,
}

impl BodyZone {
    pub fn as_str(self) -> &'static str {
        match self {
            BodyZone::Head => "head",
            BodyZone::Torso => "torso",
            BodyZone::ArmLeft => "arm_left",
            BodyZone::ArmRight => "arm_right",
            BodyZone::LegLeft => "leg_left",
            BodyZone::LegRight => "leg_right",
            BodyZone::Backpack => "backpack",
            BodyZone::ForearmLeft => "forearm_left",
            BodyZone::ForearmRight => "forearm_right",
            BodyZone::HandLeft => "hand_left",
            BodyZone::HandRight => "hand_right",
            BodyZone::ShinLeft => "shin_left",
            BodyZone::ShinRight => "shin_right",
            BodyZone::FootLeft => "foot_left",
            BodyZone::FootRight => "foot_right",
            BodyZone::LegFrontLeft => "leg_front_left",
            BodyZone::LegFrontRight => "leg_front_right",
            BodyZone::LegRearLeft => "leg_rear_left",
            BodyZone::LegRearRight => "leg_rear_right",
            BodyZone::ClawLeft => "claw_left",
            BodyZone::ClawRight => "claw_right",
            BodyZone::Carapace => "carapace",
            BodyZone::SensorCluster => "sensor_cluster",
            BodyZone::DroneCore => "drone_core",
            BodyZone::DroneArmLeft => "drone_arm_left",
            BodyZone::DroneArmRight => "drone_arm_right",
            BodyZone::DroneSensorPod => "drone_sensor_pod",
        }
    }

    /// Canonical iteration order for events + checksums. Humanoid zones first
    /// (stable since M5); quadruped + drone zones appended at the end so
    /// existing checksums for chassis-less + humanoid actors stay byte-identical.
    pub fn all() -> &'static [BodyZone] {
        &[
            BodyZone::Head,
            BodyZone::Torso,
            BodyZone::ArmLeft,
            BodyZone::ArmRight,
            BodyZone::LegLeft,
            BodyZone::LegRight,
            BodyZone::Backpack,
            BodyZone::ForearmLeft,
            BodyZone::ForearmRight,
            BodyZone::HandLeft,
            BodyZone::HandRight,
            BodyZone::ShinLeft,
            BodyZone::ShinRight,
            BodyZone::FootLeft,
            BodyZone::FootRight,
            BodyZone::LegFrontLeft,
            BodyZone::LegFrontRight,
            BodyZone::LegRearLeft,
            BodyZone::LegRearRight,
            BodyZone::ClawLeft,
            BodyZone::ClawRight,
            BodyZone::Carapace,
            BodyZone::SensorCluster,
            BodyZone::DroneCore,
            BodyZone::DroneArmLeft,
            BodyZone::DroneArmRight,
            BodyZone::DroneSensorPod,
        ]
    }

    /// Parent zone in the kinematic chain. `Head` and `Torso` and `Backpack`
    /// have no parent; the chains are torso → arm/leg (upper) → forearm/shin →
    /// hand/foot. Quadruped legs root in carapace; claws root in their leg.
    /// Drone arms root in the drone core; sensor pod is a leaf on the core.
    pub fn parent(self) -> Option<BodyZone> {
        match self {
            BodyZone::Head
            | BodyZone::Torso
            | BodyZone::Backpack
            | BodyZone::Carapace
            | BodyZone::SensorCluster
            | BodyZone::DroneCore => None,
            BodyZone::ArmLeft | BodyZone::ArmRight | BodyZone::LegLeft | BodyZone::LegRight => Some(BodyZone::Torso),
            BodyZone::ForearmLeft => Some(BodyZone::ArmLeft),
            BodyZone::ForearmRight => Some(BodyZone::ArmRight),
            BodyZone::HandLeft => Some(BodyZone::ForearmLeft),
            BodyZone::HandRight => Some(BodyZone::ForearmRight),
            BodyZone::ShinLeft => Some(BodyZone::LegLeft),
            BodyZone::ShinRight => Some(BodyZone::LegRight),
            BodyZone::FootLeft => Some(BodyZone::ShinLeft),
            BodyZone::FootRight => Some(BodyZone::ShinRight),
            BodyZone::LegFrontLeft
            | BodyZone::LegFrontRight
            | BodyZone::LegRearLeft
            | BodyZone::LegRearRight => Some(BodyZone::Carapace),
            BodyZone::ClawLeft => Some(BodyZone::LegFrontLeft),
            BodyZone::ClawRight => Some(BodyZone::LegFrontRight),
            BodyZone::DroneArmLeft | BodyZone::DroneArmRight | BodyZone::DroneSensorPod => Some(BodyZone::DroneCore),
        }
    }

    /// True iff this zone is one of the M13 quadruped-only zones.
    pub fn is_quadruped_zone(self) -> bool {
        matches!(
            self,
            BodyZone::LegFrontLeft
                | BodyZone::LegFrontRight
                | BodyZone::LegRearLeft
                | BodyZone::LegRearRight
                | BodyZone::ClawLeft
                | BodyZone::ClawRight
                | BodyZone::Carapace
                | BodyZone::SensorCluster
        )
    }

    /// True iff this zone is one of the M13 drone-only zones.
    pub fn is_drone_zone(self) -> bool {
        matches!(
            self,
            BodyZone::DroneCore | BodyZone::DroneArmLeft | BodyZone::DroneArmRight | BodyZone::DroneSensorPod
        )
    }

    /// True for the right-side arm chain (upper / forearm / hand).
    pub fn is_right_arm_chain(self) -> bool {
        matches!(self, BodyZone::ArmRight | BodyZone::ForearmRight | BodyZone::HandRight)
    }

    /// True for the left-side arm chain.
    pub fn is_left_arm_chain(self) -> bool {
        matches!(self, BodyZone::ArmLeft | BodyZone::ForearmLeft | BodyZone::HandLeft)
    }

    /// True for the right-side leg chain.
    pub fn is_right_leg_chain(self) -> bool {
        matches!(self, BodyZone::LegRight | BodyZone::ShinRight | BodyZone::FootRight)
    }

    /// True for the left-side leg chain.
    pub fn is_left_leg_chain(self) -> bool {
        matches!(self, BodyZone::LegLeft | BodyZone::ShinLeft | BodyZone::FootLeft)
    }
}

/// Equipment socket id. Sockets are addressed by stable string so modders can
/// reference them in role records (`hand_right`, `back_mount`, etc.). M5 ships a
/// canonical set; mods can add more.
pub const SOCKET_HAND_RIGHT: &str = "hand_right";
pub const SOCKET_HAND_LEFT: &str = "hand_left";
pub const SOCKET_BACK_MOUNT: &str = "back_mount";
pub const SOCKET_HEAD_MOUNT: &str = "head_mount";
pub const SOCKET_TORSO_HARDPOINT: &str = "torso_hardpoint";
