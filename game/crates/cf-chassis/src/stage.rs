use serde::{Deserialize, Serialize};

/// 11-stage chassis damage pipeline from the roadmap M5 done-criteria. Stages
/// monotonically advance (except via repair which can step back at most one level).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChassisStage {
    Nominal = 0,
    Degraded = 1,
    ModuleWarning = 2,
    ModuleFailed = 3,
    WeaponJammed = 4,
    ArmorCracked = 5,
    Disabled = 6,
    PilotInjured = 7,
    Eject = 8,
    BailTooLate = 9,
    Wreck = 10,
    Gibbed = 11,
}

impl ChassisStage {
    pub fn as_str(self) -> &'static str {
        match self {
            ChassisStage::Nominal => "nominal",
            ChassisStage::Degraded => "degraded",
            ChassisStage::ModuleWarning => "module_warning",
            ChassisStage::ModuleFailed => "module_failed",
            ChassisStage::WeaponJammed => "weapon_jammed",
            ChassisStage::ArmorCracked => "armor_cracked",
            ChassisStage::Disabled => "disabled",
            ChassisStage::PilotInjured => "pilot_injured",
            ChassisStage::Eject => "eject",
            ChassisStage::BailTooLate => "bail_too_late",
            ChassisStage::Wreck => "wreck",
            ChassisStage::Gibbed => "gibbed",
        }
    }

    /// True for the terminal stages where the chassis is no longer pilotable.
    pub fn is_terminal(self) -> bool {
        matches!(self, ChassisStage::Wreck | ChassisStage::Gibbed)
    }

    pub fn is_ejecting(self) -> bool {
        matches!(self, ChassisStage::Eject | ChassisStage::BailTooLate)
    }
}

/// Pilot state inside the chassis. Lifecycle:
/// `Bound → Injured? → Ejecting → Ejected → Extracted` (success path) or
/// `Bound → Ejecting? → BailedTooLate → Lost` (failure path) or
/// `Bound → ... → Lost` (gibbed without ejecting).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PilotState {
    /// Pilot is bound to the chassis (default while flying / walking the mech).
    Bound = 0,
    /// Pilot is injured but still bound (decel/jet capability reduced).
    Injured = 1,
    /// Pilot has triggered eject; eject sequence is mid-flight.
    Ejecting = 2,
    /// Pilot has ejected and is now controlled as foot infantry.
    Ejected = 3,
    /// Pilot has reached a safe extraction zone / objective.
    Extracted = 4,
    /// Pilot tried to eject too late; sequence failed (chassis already wrecked).
    BailedTooLate = 5,
    /// Pilot is lost (chassis wrecked + no eject OR bail-too-late).
    Lost = 6,
}

impl PilotState {
    pub fn as_str(self) -> &'static str {
        match self {
            PilotState::Bound => "bound",
            PilotState::Injured => "injured",
            PilotState::Ejecting => "ejecting",
            PilotState::Ejected => "ejected",
            PilotState::Extracted => "extracted",
            PilotState::BailedTooLate => "bailed_too_late",
            PilotState::Lost => "lost",
        }
    }

    pub fn is_in_chassis(self) -> bool {
        matches!(self, PilotState::Bound | PilotState::Injured)
    }

    pub fn is_lost(self) -> bool {
        matches!(self, PilotState::Lost | PilotState::BailedTooLate)
    }
}
