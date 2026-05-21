use serde::{Deserialize, Serialize};

/// **M5.8 forward-hook (DR-040 ResourceAccumulators)**: per-actor resource
/// values driven by origin reaction matrix at M5.8. Reserved layout slot at
/// M5 so save bundles + observe frames serialize the slot now and M5.8 can
/// fill values without a checksum byte-layout shift.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ResourceAccumulators {
    pub caloric_energy: f32,
    pub battery_charge: f32,
    pub power: f32,
    pub heat: f32,
    pub oxygen_supply: f32,
    pub g_load_dose: f32,
    pub concussion_dose: f32,
}

/// **M5.7/M5.8 forward-hook (DR-036 affliction layer)**: per-actor systemic
/// state. Spec-locked enum prevents typos across BP4 milestones.
#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AfflictionKind {
    Wetness,
    Burning,
    Corroded,
    Electrified,
    Poisoned,
    Asphyxiating,
    Suffocating,
    Drowning,
    Depressurizing,
    InternalShock,
    CoolantLeaking,
    OilLeaking,
    Overheating,
    LowBattery,
    PowerStarved,
    Weak,
    Exhausted,
    Hypoxia,
    Downclocked,
    HeatExhaustion,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Affliction {
    pub kind: AfflictionKind,
    #[serde(default)]
    pub intensity: f32,
    #[serde(default)]
    pub expires_tick: Option<u64>,
}
