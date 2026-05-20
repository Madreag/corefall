//! **M14H** § per-actor persistent state surfaces used by the field-medic
//! workflow. Sits in `cf-actor` alongside [`crate::cardiac`] so the
//! save/load round-trip + per-tick aging passes can mutate the data
//! without cross-crate dependencies.

use serde::{Deserialize, Serialize};

/// **M14H** § kind of active per-actor buff. Each variant corresponds to a
/// treatment producer whose effect persists past `treatment.completed`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuffKind {
    /// Painkiller (opioid t1) — reduces Pain by 30 points; 4h duration.
    PainkillerOpioidT1,
    /// Anti-anxiety benzo t1 — reduces panic / anxiety severity; 6h duration.
    AntiAnxietyBenzoT1,
    /// Combat stim t1 — +20% accuracy + 20% move speed; 90s duration.
    CombatStimT1,
    /// Combat stim crash — applied automatically after CombatStimT1 expires.
    CombatStimT1Crash,
    /// Hospital bed — +50% natural heal rate while bedded (per-tick).
    HospitalBedV1,
    /// IV fluids — restores hydration over duration; rehydrates unconscious.
    IvFluidsV1,
    /// Oxygen therapy — accelerates post-hypoxia recovery.
    OxygenTherapyV1,
    /// Anti-radiation chelation — reduces radiation dose over duration.
    AntiRadiationChelation,
}

impl BuffKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BuffKind::PainkillerOpioidT1 => "painkiller_opioid_t1",
            BuffKind::AntiAnxietyBenzoT1 => "anti_anxiety_benzo_t1",
            BuffKind::CombatStimT1 => "combat_stim_t1",
            BuffKind::CombatStimT1Crash => "combat_stim_t1_crash",
            BuffKind::HospitalBedV1 => "hospital_bed_v1",
            BuffKind::IvFluidsV1 => "iv_fluids_v1",
            BuffKind::OxygenTherapyV1 => "oxygen_therapy_v1",
            BuffKind::AntiRadiationChelation => "anti_radiation_chelation",
        }
    }

    /// Default duration in sim seconds for a buff kind.
    pub fn default_duration_seconds(self) -> f32 {
        match self {
            BuffKind::PainkillerOpioidT1 => 4.0 * 3600.0,
            BuffKind::AntiAnxietyBenzoT1 => 6.0 * 3600.0,
            BuffKind::CombatStimT1 => 90.0,
            BuffKind::CombatStimT1Crash => 60.0,
            BuffKind::HospitalBedV1 => f32::INFINITY,
            BuffKind::IvFluidsV1 => 120.0,
            BuffKind::OxygenTherapyV1 => 60.0,
            BuffKind::AntiRadiationChelation => 60.0,
        }
    }
}

/// **M14H** § single active buff on an actor. Stored in `ActorState.m14h_buffs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveBuff {
    pub kind: BuffKind,
    pub applied_tick: u64,
    pub expires_tick: u64,
}

impl ActiveBuff {
    pub fn new(kind: BuffKind, applied_tick: u64, tick_rate_hz: u32) -> Self {
        let duration = kind.default_duration_seconds();
        let expires_tick = if duration.is_finite() {
            applied_tick.saturating_add((duration * tick_rate_hz.max(1) as f32) as u64)
        } else {
            u64::MAX
        };
        Self {
            kind,
            applied_tick,
            expires_tick,
        }
    }

    pub fn is_expired(&self, current_tick: u64) -> bool {
        current_tick >= self.expires_tick
    }
}

/// **M14H** § antibiotic-course tracking state.
///
/// Doses are taken on an interval; `resistance_risk` flips true if a dose
/// is missed (compound score crosses threshold for the t1 / t2 risk surfaces).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AntibioticCourseState {
    /// Tier of the course (1 or 2).
    pub tier: u8,
    /// Doses taken so far.
    pub doses_taken: u32,
    /// Total doses required for completion (14 for t1, 21 for t2).
    pub doses_required: u32,
    /// Interval between doses in hours (8h for t1, 6h for t2).
    pub dose_interval_hours: f32,
    /// Tick of the next scheduled dose (0 = take immediately).
    pub next_dose_tick: u64,
    /// True if the actor missed a dose interval — feeds resistance risk.
    pub resistance_risk: bool,
}

impl AntibioticCourseState {
    pub fn t1(applied_tick: u64) -> Self {
        Self {
            tier: 1,
            doses_taken: 0,
            doses_required: 14,
            dose_interval_hours: 8.0,
            next_dose_tick: applied_tick,
            resistance_risk: false,
        }
    }

    pub fn t2(applied_tick: u64) -> Self {
        Self {
            tier: 2,
            doses_taken: 0,
            doses_required: 21,
            dose_interval_hours: 6.0,
            next_dose_tick: applied_tick,
            resistance_risk: false,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.doses_taken >= self.doses_required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buff_default_duration() {
        assert_eq!(
            BuffKind::CombatStimT1.default_duration_seconds(),
            90.0
        );
        assert_eq!(
            BuffKind::PainkillerOpioidT1.default_duration_seconds(),
            4.0 * 3600.0
        );
    }

    #[test]
    fn buff_expiry() {
        let buff = ActiveBuff::new(BuffKind::CombatStimT1, 1000, 60);
        // 90s × 60 Hz = 5400 ticks.
        assert_eq!(buff.expires_tick, 1000 + 5400);
        assert!(!buff.is_expired(1500));
        assert!(buff.is_expired(7000));
    }

    #[test]
    fn antibiotic_course_t1_setup() {
        let s = AntibioticCourseState::t1(100);
        assert_eq!(s.doses_required, 14);
        assert_eq!(s.dose_interval_hours, 8.0);
        assert!(!s.is_complete());
    }

    #[test]
    fn antibiotic_course_t2_setup() {
        let s = AntibioticCourseState::t2(100);
        assert_eq!(s.doses_required, 21);
        assert_eq!(s.dose_interval_hours, 6.0);
    }
}
