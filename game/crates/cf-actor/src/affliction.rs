use serde::{Deserialize, Serialize};

/// values driven by origin reaction matrix at M17. Reserved layout slot at
/// M5 so save bundles + observe frames serialize the slot now and M17
/// fills values without a checksum byte-layout shift. Per-origin survival
/// resources: `0.0` means "origin does not use this resource" (a robot has
/// `blood = 0`, a human has `oil = 0`); the origin profile decides which
/// are live (`cf_actor::origin::OriginProfile`).
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
    /// Blood volume in mL (humans / androids). Survival resource: `0` = dead.
    pub blood: f32,
    /// Oil / coolant volume in mL (robots / androids synthetic side).
    pub oil: f32,
    /// Heavy-biomech bio-fluid volume in mL (slow-clot blood variant).
    pub bio_fluid: f32,
    /// Robot internal-shock dose (0-100; concussion equivalent for robots).
    pub internal_shock_dose: f32,
}

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
