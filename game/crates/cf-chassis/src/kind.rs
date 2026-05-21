use serde::{Deserialize, Serialize};

/// Stable id for a chassis spec preset. Scenarios reference these by id and the
/// runtime resolves them through [`crate::chassis_spec`].
pub const POWERED_ARMOR_ID: &str = "powered_armor_v1";
pub const LIGHT_MECH_ID: &str = "light_mech_v1";
pub const INFANTRY_ID: &str = "infantry_v1";
/// **M13** § "Chassis archetypes — M13 ships 5" — non-humanoid quadruped
/// archetype: 4 legs + 2 claws + carapace + sensor cluster; no jet.
pub const CRAB_QUADRUPED_ID: &str = "crab_quadruped_v1";
/// **M13** § "Chassis archetypes — M13 ships 5" — autonomous miniature
/// chassis: 4 zones (chassis core + 2 arms + sensor pod); no pilot.
pub const DRONE_ID: &str = "drone_v1";
/// **M14A** § "Heavy Armor — `heavy_trooper_v1`" — tank-grade infantry
/// chassis. 380 kg base loaded; rifles glance, only AP/HE breach.
pub const HEAVY_TROOPER_ID: &str = "heavy_trooper_v1";

/// **M13** § "Chassis archetypes — M13 ships 5". Discriminants `Infantry=0`
/// through `LightMech=2` are pinned for cross-milestone determinism; the new
/// `CrabQuadruped=3` + `Drone=4` are appended so the repr(u8) byte layout
/// stays stable.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChassisKind {
    Infantry = 0,
    PoweredArmor = 1,
    LightMech = 2,
    /// **M13** non-humanoid quadruped (crab-like) chassis.
    CrabQuadruped = 3,
    /// **M13** autonomous drone chassis (no pilot binding).
    Drone = 4,
    /// **M14A** § "Heavy Armor" — tank-grade infantry; small arms barely
    /// scratch; 380 kg loaded; throttle-for-weight jet visibly struggles.
    HeavyTrooper = 5,
}

impl ChassisKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ChassisKind::Infantry => "infantry",
            ChassisKind::PoweredArmor => "powered_armor",
            ChassisKind::LightMech => "light_mech",
            ChassisKind::CrabQuadruped => "crab_quadruped",
            ChassisKind::Drone => "drone",
            ChassisKind::HeavyTrooper => "heavy_trooper",
        }
    }

    /// Reference ids (`POWERED_ARMOR_ID`, `LIGHT_MECH_ID`, `INFANTRY_ID`,
    /// `CRAB_QUADRUPED_ID`, `DRONE_ID`, `HEAVY_TROOPER_ID`).
    pub fn default_spec_id(self) -> &'static str {
        match self {
            ChassisKind::Infantry => INFANTRY_ID,
            ChassisKind::PoweredArmor => POWERED_ARMOR_ID,
            ChassisKind::LightMech => LIGHT_MECH_ID,
            ChassisKind::CrabQuadruped => CRAB_QUADRUPED_ID,
            ChassisKind::Drone => DRONE_ID,
            ChassisKind::HeavyTrooper => HEAVY_TROOPER_ID,
        }
    }

    /// **M13** § "Per-chassis ability slot count (Light=1, Medium=2, Heavy=3,
    /// Drone=1)". Used by [`crate::ChassisAbilitySlots`] to bound the active
    /// ability roster.
    pub fn ability_slot_count(self) -> u8 {
        match self {
            ChassisKind::Infantry | ChassisKind::Drone => 1,
            ChassisKind::PoweredArmor | ChassisKind::CrabQuadruped => 2,
            ChassisKind::LightMech | ChassisKind::HeavyTrooper => 3,
        }
    }

    /// **M13** § "Weapon modifier slots (Noita-style combinatorial)". Per
    /// the spec's per-chassis-tier table (Infantry 0-1, Powered armor 1-2,
    /// Light mech 2-3, Heavy mech future 3-4). We surface the max for each.
    pub fn weapon_modifier_slot_count(self) -> u8 {
        match self {
            ChassisKind::Infantry | ChassisKind::Drone => 1,
            ChassisKind::PoweredArmor | ChassisKind::CrabQuadruped => 2,
            ChassisKind::LightMech | ChassisKind::HeavyTrooper => 3,
        }
    }

    /// **M13** § "Pilot-inside-chassis dual silhouette" — weight class drives
    /// the pilot silhouette size scaling (Light 60% / Medium 40% / Heavy 25%).
    /// Returns the scale factor (0..1) for the pilot inset overlay.
    pub fn pilot_silhouette_scale(self) -> f32 {
        match self {
            ChassisKind::Infantry | ChassisKind::Drone => 1.0,
            ChassisKind::PoweredArmor => 0.6,
            ChassisKind::CrabQuadruped | ChassisKind::HeavyTrooper => 0.4,
            ChassisKind::LightMech => 0.25,
        }
    }

    /// **M13** § "Cockpit camera anchor" — only Medium + Heavy classes
    /// support the cockpit anchor (Light has third-person only). Returns
    /// `true` when [`CameraAnchor::Cockpit`] is a valid request.
    pub fn supports_cockpit_anchor(self) -> bool {
        matches!(
            self,
            ChassisKind::LightMech | ChassisKind::CrabQuadruped | ChassisKind::HeavyTrooper
        )
    }
}

/// **M13** § "Cockpit camera anchor (first-person mech view)". Tracks the
/// current camera anchor request driven by `act.input.camera_anchor`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CameraAnchor {
    /// Default third-person follow camera.
    #[default]
    Default = 0,
    /// First-person inside chassis cockpit (Medium + Heavy classes only).
    Cockpit = 1,
}

impl CameraAnchor {
    pub fn as_str(self) -> &'static str {
        match self {
            CameraAnchor::Default => "default",
            CameraAnchor::Cockpit => "cockpit",
        }
    }

    pub fn parse(s: &str) -> Option<CameraAnchor> {
        match s.to_ascii_lowercase().as_str() {
            "default" | "follow" | "third_person" => Some(CameraAnchor::Default),
            "cockpit" | "first_person" => Some(CameraAnchor::Cockpit),
            _ => None,
        }
    }
}
