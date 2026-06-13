//! treatment.rs — the psych-medication registry (8 classes + item catalog),
//! the per-actor / per-condition treatment plan (therapy-session counter +
//! medication course), and the deterministic therapy-efficacy roller.
//!
//! Mirrors the `cf-equipment::cures` item-spec + `load_dir` pattern so the
//! medication catalog can be authored as `content/psych_meds/*.ron` with a
//! hardcoded boot catalog.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{conditions::ConditionKind, mh_roll, ticks_to_seconds, SALT_THERAPY};

/// The 8 launch psych-medication classes (spec § Pharmacological treatment
/// registry). Variant order is the stable serialization order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum PsychMedClass {
    /// SSRI-equivalent — depression / anxiety / PTSD adjunct.
    Ssri = 0,
    /// Benzo-equivalent — anxiety / panic / sleep.
    Benzo = 1,
    /// SNRI-equivalent — depression-resistant.
    Snri = 2,
    /// Opioid-equivalent — severe pain; addiction risk.
    Opioid = 3,
    /// Stimulant-equivalent — combat boost; addiction risk.
    Stimulant = 4,
    /// Anti-psychotic — severe PTSD with psychotic features.
    Antipsychotic = 5,
    /// Sleep aid — insomnia.
    SleepAid = 6,
    /// Withdrawal-assist — methadone-equivalent (opioid) / clonidine (stim).
    WithdrawalAssist = 7,
}

impl PsychMedClass {
    pub fn as_str(self) -> &'static str {
        match self {
            PsychMedClass::Ssri => "ssri",
            PsychMedClass::Benzo => "benzo",
            PsychMedClass::Snri => "snri",
            PsychMedClass::Opioid => "opioid",
            PsychMedClass::Stimulant => "stimulant",
            PsychMedClass::Antipsychotic => "antipsychotic",
            PsychMedClass::SleepAid => "sleep_aid",
            PsychMedClass::WithdrawalAssist => "withdrawal_assist",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "ssri" => PsychMedClass::Ssri,
            "benzo" => PsychMedClass::Benzo,
            "snri" => PsychMedClass::Snri,
            "opioid" => PsychMedClass::Opioid,
            "stimulant" => PsychMedClass::Stimulant,
            "antipsychotic" => PsychMedClass::Antipsychotic,
            "sleep_aid" => PsychMedClass::SleepAid,
            "withdrawal_assist" => PsychMedClass::WithdrawalAssist,
            _ => return None,
        })
    }

    pub fn all() -> &'static [PsychMedClass] {
        &[
            PsychMedClass::Ssri,
            PsychMedClass::Benzo,
            PsychMedClass::Snri,
            PsychMedClass::Opioid,
            PsychMedClass::Stimulant,
            PsychMedClass::Antipsychotic,
            PsychMedClass::SleepAid,
            PsychMedClass::WithdrawalAssist,
        ]
    }

    /// Per-dose addiction risk for habit-forming classes (benzo / opioid /
    /// stimulant). Non-addictive classes return 0.0.
    pub fn addiction_risk_per_dose(self) -> f32 {
        match self {
            PsychMedClass::Benzo => crate::BENZO_ADDICTION_RISK_PER_DOSE,
            PsychMedClass::Opioid => 0.06,
            PsychMedClass::Stimulant => crate::STIM_ADDICTION_RISK_PER_DOSE,
            _ => 0.0,
        }
    }

    /// True for the habit-forming classes that can drive an Addiction.
    pub fn is_habit_forming(self) -> bool {
        self.addiction_risk_per_dose() > 0.0
    }
}

/// One psych-medication item family (`content/psych_meds/*.ron`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PsychMedItemSpec {
    pub item_id: String,
    pub display_name: String,
    pub class: PsychMedClass,
    /// Conditions this medication is indicated for.
    pub treats: Vec<ConditionKind>,
    /// Therapeutic onset (seconds): time on the medication before it counts
    /// toward remission.
    pub onset_seconds: f32,
    pub dose_count: u8,
    pub dose_interval_hours: f32,
    pub addiction_risk_per_dose: f32,
    pub tier: u8,
}

/// The 8 launch psych-medication families (spec § Files).
pub fn default_psych_med_catalog() -> Vec<PsychMedItemSpec> {
    use ConditionKind::{AnxietyDisorder, Addiction, Depression, Insomnia, PanicDisorder, Ptsd, Withdrawal};
    vec![
        PsychMedItemSpec {
            item_id: "ssri_sertraline".to_string(),
            display_name: "SSRI (Sertraline-class)".to_string(),
            class: PsychMedClass::Ssri,
            treats: vec![Depression, AnxietyDisorder, Ptsd],
            onset_seconds: crate::SSRI_ONSET_SECONDS,
            dose_count: 30,
            dose_interval_hours: 24.0,
            addiction_risk_per_dose: 0.0,
            tier: 1,
        },
        PsychMedItemSpec {
            item_id: "benzo_diazepam".to_string(),
            display_name: "Benzodiazepine (Diazepam-class)".to_string(),
            class: PsychMedClass::Benzo,
            treats: vec![AnxietyDisorder, PanicDisorder, Insomnia],
            onset_seconds: 7.0 * crate::DAY_SECONDS,
            dose_count: 14,
            dose_interval_hours: 12.0,
            addiction_risk_per_dose: crate::BENZO_ADDICTION_RISK_PER_DOSE,
            tier: 1,
        },
        PsychMedItemSpec {
            item_id: "snri_venlafaxine".to_string(),
            display_name: "SNRI (Venlafaxine-class)".to_string(),
            class: PsychMedClass::Snri,
            treats: vec![Depression],
            onset_seconds: crate::SSRI_ONSET_SECONDS,
            dose_count: 30,
            dose_interval_hours: 24.0,
            addiction_risk_per_dose: 0.0,
            tier: 2,
        },
        PsychMedItemSpec {
            item_id: "opioid_morphine".to_string(),
            display_name: "Opioid Analgesic (Morphine-class)".to_string(),
            class: PsychMedClass::Opioid,
            treats: vec![],
            onset_seconds: crate::HOUR_SECONDS,
            dose_count: 6,
            dose_interval_hours: 6.0,
            addiction_risk_per_dose: 0.06,
            tier: 2,
        },
        PsychMedItemSpec {
            item_id: "stimulant_modafinil".to_string(),
            display_name: "Stimulant (Modafinil-class)".to_string(),
            class: PsychMedClass::Stimulant,
            treats: vec![],
            onset_seconds: 0.5 * crate::DAY_SECONDS,
            dose_count: 10,
            dose_interval_hours: 24.0,
            addiction_risk_per_dose: crate::STIM_ADDICTION_RISK_PER_DOSE,
            tier: 2,
        },
        PsychMedItemSpec {
            item_id: "antipsychotic_quetiapine".to_string(),
            display_name: "Anti-psychotic (Quetiapine-class)".to_string(),
            class: PsychMedClass::Antipsychotic,
            treats: vec![Ptsd],
            onset_seconds: 7.0 * crate::DAY_SECONDS,
            dose_count: 30,
            dose_interval_hours: 24.0,
            addiction_risk_per_dose: 0.0,
            tier: 3,
        },
        PsychMedItemSpec {
            item_id: "sleep_aid_zolpidem".to_string(),
            display_name: "Sleep Aid (Zolpidem-class)".to_string(),
            class: PsychMedClass::SleepAid,
            treats: vec![Insomnia],
            onset_seconds: 3.0 * crate::DAY_SECONDS,
            dose_count: 14,
            dose_interval_hours: 24.0,
            addiction_risk_per_dose: 0.0,
            tier: 1,
        },
        PsychMedItemSpec {
            item_id: "withdrawal_assist_methadone".to_string(),
            display_name: "Withdrawal-assist (Methadone/Clonidine-class)".to_string(),
            class: PsychMedClass::WithdrawalAssist,
            treats: vec![Addiction, Withdrawal],
            onset_seconds: 3.0 * crate::DAY_SECONDS,
            dose_count: 21,
            dose_interval_hours: 12.0,
            addiction_risk_per_dose: 0.0,
            tier: 2,
        },
    ]
}

/// Find the first medication item indicated for `condition`.
pub fn psych_med_for(catalog: &[PsychMedItemSpec], condition: ConditionKind) -> Option<&PsychMedItemSpec> {
    catalog.iter().find(|m| m.treats.contains(&condition))
}

/// Load `content/psych_meds/*.ron`, keyed by item id. Missing dir → defaults.
pub fn load_psych_med_dir(dir: &Path) -> Result<BTreeMap<String, PsychMedItemSpec>, PsychMedLoadError> {
    let mut out: BTreeMap<String, PsychMedItemSpec> = default_psych_med_catalog()
        .into_iter()
        .map(|m| (m.item_id.clone(), m))
        .collect();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)
        .map_err(|e| PsychMedLoadError::Io(dir.to_path_buf(), e.to_string()))?
        .flatten()
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let body = fs::read_to_string(&path).map_err(|e| PsychMedLoadError::Io(path.clone(), e.to_string()))?;
        match ron::from_str::<PsychMedItemSpec>(&body) {
            Ok(m) => {
                out.insert(m.item_id.clone(), m);
            }
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "psych med spec parse failed");
                return Err(PsychMedLoadError::Parse(path.clone(), e.to_string()));
            }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum PsychMedLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

/// In-progress treatment course on one condition: a therapy-session counter
/// plus an optional medication course. Both arms (therapy + medication) feed
/// the lifecycle remission check.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct TreatmentPlan {
    pub therapy_sessions_completed: u32,
    pub medication: Option<PsychMedClass>,
    pub medication_started_tick: Option<u64>,
    /// Latched true once this plan has driven a lifecycle resolution
    /// (remission / chronic / refractory) so it cannot re-resolve every tick.
    pub resolved: bool,
}

impl TreatmentPlan {
    /// Record one completed 30-in-game-minute therapy session.
    pub fn record_therapy_session(&mut self) {
        self.therapy_sessions_completed = self.therapy_sessions_completed.saturating_add(1);
    }

    /// Begin a medication course of `class` at `tick`.
    pub fn start_medication(&mut self, class: PsychMedClass, tick: u64) {
        self.medication = Some(class);
        self.medication_started_tick = Some(tick);
    }

    /// Seconds the current medication course has been running (0 if none).
    pub fn medication_active_for(&self, tick: u64, tick_rate_hz: u32) -> f32 {
        match self.medication_started_tick {
            Some(start) if tick >= start => ticks_to_seconds(tick - start, tick_rate_hz),
            _ => 0.0,
        }
    }

    /// True once a medication course is running.
    pub fn medication_started(&self) -> bool {
        self.medication_started_tick.is_some()
    }
}

/// Deterministic [0,1) therapy-session efficacy roll. Higher = more effective.
/// Keyed so the same (seed, actor, condition, session) reproduces the outcome.
pub fn therapy_efficacy_roll(seed: u64, actor_id: u64, condition: ConditionKind, session_index: u32) -> f32 {
    mh_roll(seed, actor_id, condition, SALT_THERAPY ^ u64::from(session_index))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eight_med_classes_round_trip() {
        assert_eq!(PsychMedClass::all().len(), 8);
        for &c in PsychMedClass::all() {
            assert_eq!(PsychMedClass::from_str(c.as_str()), Some(c));
        }
    }

    #[test]
    fn eight_med_items_and_each_condition_indicated() {
        let cat = default_psych_med_catalog();
        assert_eq!(cat.len(), 8);
        // Every condition that names a medication in its spec has an item.
        for c in [
            ConditionKind::Ptsd,
            ConditionKind::AnxietyDisorder,
            ConditionKind::Depression,
            ConditionKind::Addiction,
            ConditionKind::Withdrawal,
            ConditionKind::Insomnia,
            ConditionKind::PanicDisorder,
        ] {
            assert!(psych_med_for(&cat, c).is_some(), "no med for {}", c.as_str());
        }
    }

    #[test]
    fn habit_forming_classes_carry_risk() {
        assert!((PsychMedClass::Benzo.addiction_risk_per_dose() - 0.04).abs() < 1e-6);
        assert!((PsychMedClass::Stimulant.addiction_risk_per_dose() - 0.07).abs() < 1e-6);
        assert_eq!(PsychMedClass::Ssri.addiction_risk_per_dose(), 0.0);
        assert!(PsychMedClass::Opioid.is_habit_forming());
        assert!(!PsychMedClass::Antipsychotic.is_habit_forming());
    }

    #[test]
    fn medication_active_for_tracks_elapsed() {
        let mut plan = TreatmentPlan::default();
        assert_eq!(plan.medication_active_for(100, 60), 0.0);
        plan.start_medication(PsychMedClass::Ssri, 0);
        // 14 days at 60 Hz.
        let ticks = (crate::SSRI_ONSET_SECONDS * 60.0) as u64;
        assert!((plan.medication_active_for(ticks, 60) - crate::SSRI_ONSET_SECONDS).abs() < 1.0);
    }

    #[test]
    fn therapy_roll_is_deterministic_and_bounded() {
        let a = therapy_efficacy_roll(42, 7, ConditionKind::Ptsd, 3);
        let b = therapy_efficacy_roll(42, 7, ConditionKind::Ptsd, 3);
        assert_eq!(a, b);
        assert!((0.0..1.0).contains(&a));
        assert_ne!(a, therapy_efficacy_roll(42, 7, ConditionKind::Ptsd, 4));
    }

    #[test]
    fn med_items_round_trip_through_ron() {
        for m in default_psych_med_catalog() {
            let s = ron::to_string(&m).unwrap();
            let back: PsychMedItemSpec = ron::from_str(&s).unwrap();
            assert_eq!(m, back);
        }
    }
}
