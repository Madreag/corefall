//! comorbidity.rs — the mental-health comorbidity matrix: which conditions
//! travel together, the deterministic secondary-onset roller, and the
//! co-presence detector.
//!
//! Spec § Comorbidity matrix: "PTSD + Depression common; Addiction + Withdrawal
//! mandatory pair; Insomnia comorbid with most. M16C tracks comorbid pairs for
//! treatment-plan adjustment." Authored as
//! `content/psych_conditions/_comorbidity.ron`.
//!
//! Dependency direction is one-way: this module reads + mutates the
//! `conditions` types but `conditions` never reaches back here, so the two
//! stay acyclic. The engine consumer ties them together (trigger a primary,
//! then `apply_comorbidities`), exactly as it wires the rest of the kernel.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    conditions::{ActorMentalHealth, ComorbidityDetectedEvent, ConditionKind, ConditionTriggeredEvent, TriggerReason},
    mh_roll, SALT_COMORBID,
};

/// One directed comorbidity relationship: when `primary` is present, `comorbid`
/// tends to co-occur.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComorbidityPair {
    pub primary: ConditionKind,
    pub comorbid: ConditionKind,
    /// Probability `comorbid` co-onsets when `primary` triggers (the secondary
    /// onset roll). `0.0` for mandatory pairs whose onset is driven elsewhere
    /// (e.g. Withdrawal onsets on drug absence, not at Addiction onset).
    pub chance: f32,
    /// A guaranteed clinical relationship (the Addiction + Withdrawal pair).
    /// Mandatory pairs are always flagged when both are present and are never
    /// part of the probabilistic co-onset roll.
    pub mandatory: bool,
}

/// The comorbidity matrix (directed pairs).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComorbidityMatrix {
    pub pairs: Vec<ComorbidityPair>,
}

impl Default for ComorbidityMatrix {
    fn default() -> Self {
        Self::default_matrix()
    }
}

impl ComorbidityMatrix {
    /// The launch comorbidity matrix (spec § Comorbidity matrix).
    pub fn default_matrix() -> Self {
        use ConditionKind::{Addiction, AnxietyDisorder, Depression, Insomnia, PanicDisorder, Ptsd, Withdrawal};
        let pair = |primary, comorbid, chance, mandatory| ComorbidityPair {
            primary,
            comorbid,
            chance,
            mandatory,
        };
        Self {
            pairs: vec![
                // PTSD + Depression common; PTSD brings insomnia (spec PTSD row).
                pair(Ptsd, Depression, 0.30, false),
                pair(Ptsd, Insomnia, 0.40, false),
                // Insomnia comorbid with most mood/anxiety conditions.
                pair(Depression, Insomnia, 0.35, false),
                pair(AnxietyDisorder, Insomnia, 0.30, false),
                pair(PanicDisorder, Insomnia, 0.25, false),
                pair(Addiction, Insomnia, 0.20, false),
                // Anxiety chronic feeds panic disorder (spec Panic trigger).
                pair(AnxietyDisorder, PanicDisorder, 0.20, false),
                // Mandatory pair — Addiction + Withdrawal always travel
                // together; Withdrawal onset is the absence check, so no
                // co-onset roll (chance 0, mandatory true).
                pair(Addiction, Withdrawal, 0.0, true),
            ],
        }
    }

    /// Directed pairs whose `primary` is `kind` and that can probabilistically
    /// co-onset (non-mandatory, chance > 0).
    pub fn co_onset_candidates(&self, primary: ConditionKind) -> impl Iterator<Item = &ComorbidityPair> {
        self.pairs
            .iter()
            .filter(move |p| p.primary == primary && !p.mandatory && p.chance > 0.0)
    }

    /// The mandatory comorbid partners of `kind` (e.g. Withdrawal for
    /// Addiction).
    pub fn mandatory_partners(&self, primary: ConditionKind) -> impl Iterator<Item = ConditionKind> + '_ {
        self.pairs
            .iter()
            .filter(move |p| p.primary == primary && p.mandatory)
            .map(|p| p.comorbid)
    }

    /// True when `a` and `b` are a known comorbid pair (in either direction).
    pub fn is_comorbid_pair(&self, a: ConditionKind, b: ConditionKind) -> bool {
        self.pairs
            .iter()
            .any(|p| (p.primary == a && p.comorbid == b) || (p.primary == b && p.comorbid == a))
    }

    /// Load the matrix from `content/psych_conditions/_comorbidity.ron`. Missing
    /// dir / file → the default matrix.
    pub fn load_dir(dir: &Path) -> Result<Self, ComorbidityLoadError> {
        Self::load_file(&dir.join("_comorbidity.ron"))
    }

    /// Load the matrix from an explicit file path. Missing file → default.
    pub fn load_file(path: &Path) -> Result<Self, ComorbidityLoadError> {
        if !path.exists() {
            return Ok(Self::default_matrix());
        }
        let body = fs::read_to_string(path).map_err(|e| ComorbidityLoadError::Io(path.to_path_buf(), e.to_string()))?;
        match ron::from_str::<ComorbidityMatrix>(&body) {
            Ok(m) => Ok(m),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "comorbidity matrix parse failed");
                Err(ComorbidityLoadError::Parse(path.to_path_buf(), e.to_string()))
            }
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ComorbidityLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

/// Roll the secondary comorbid onsets for a freshly-triggered `primary` and
/// apply them to `state`. Deterministic (keyed by actor + comorbid kind). For
/// each non-mandatory candidate not already present, an independent
/// `mh_roll < chance` co-onsets the comorbid condition (recording
/// `comorbid_with = primary`) and emits both the trigger and the detection
/// event. Synthetic origins co-onset nothing.
pub fn apply_comorbidities(
    state: &mut ActorMentalHealth,
    matrix: &ComorbidityMatrix,
    actor_id: u64,
    primary: ConditionKind,
    seed: u64,
    tick: u64,
) -> (Vec<ConditionTriggeredEvent>, Vec<ComorbidityDetectedEvent>) {
    let mut triggered = Vec::new();
    let mut detected = Vec::new();
    if state.origin.is_synthetic() {
        return (triggered, detected);
    }
    // Snapshot the candidate kinds first (the roll is independent per kind) so
    // we don't borrow `matrix` across the mutable `state` calls.
    let candidates: Vec<(ConditionKind, f32)> = matrix
        .co_onset_candidates(primary)
        .map(|p| (p.comorbid, p.chance))
        .collect();
    for (comorbid, chance) in candidates {
        if state.has(comorbid) {
            continue;
        }
        if mh_roll(seed, actor_id, comorbid, SALT_COMORBID) >= chance {
            continue;
        }
        if let Some(ev) = state.trigger(actor_id, comorbid, TriggerReason::Comorbidity, tick) {
            if let Some(c) = state.find_mut(comorbid) {
                c.comorbid_with = Some(primary);
            }
            triggered.push(ev);
            detected.push(ComorbidityDetectedEvent {
                actor_id,
                tick,
                primary,
                comorbid,
            });
        }
    }
    (triggered, detected)
}

/// Detect every known comorbid pair currently co-present on `state` (both
/// conditions active). Surfaces mandatory pairs (e.g. Addiction + Withdrawal)
/// once the second condition onsets through its own trigger path. Each pair is
/// reported once, primary-before-comorbid by the matrix's direction.
pub fn detect_present(
    state: &ActorMentalHealth,
    matrix: &ComorbidityMatrix,
    actor_id: u64,
    tick: u64,
) -> Vec<ComorbidityDetectedEvent> {
    let mut out = Vec::new();
    for pair in &matrix.pairs {
        if state.has(pair.primary) && state.has(pair.comorbid) {
            out.push(ComorbidityDetectedEvent {
                actor_id,
                tick,
                primary: pair.primary,
                comorbid: pair.comorbid,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conditions::ConditionStage;
    use crate::OriginId;

    #[test]
    fn default_matrix_encodes_spec_pairs() {
        let m = ComorbidityMatrix::default_matrix();
        // PTSD + Depression common.
        assert!(m.is_comorbid_pair(ConditionKind::Ptsd, ConditionKind::Depression));
        // Addiction + Withdrawal mandatory.
        assert!(m
            .mandatory_partners(ConditionKind::Addiction)
            .any(|c| c == ConditionKind::Withdrawal));
        // Insomnia comorbid with most.
        for primary in [
            ConditionKind::Ptsd,
            ConditionKind::Depression,
            ConditionKind::AnxietyDisorder,
            ConditionKind::PanicDisorder,
        ] {
            assert!(
                m.is_comorbid_pair(primary, ConditionKind::Insomnia),
                "{} not linked to insomnia",
                primary.as_str()
            );
        }
    }

    #[test]
    fn mandatory_pair_is_not_a_co_onset_candidate() {
        let m = ComorbidityMatrix::default_matrix();
        // Addiction's only co-onset candidate is insomnia, never withdrawal.
        let cands: Vec<ConditionKind> = m.co_onset_candidates(ConditionKind::Addiction).map(|p| p.comorbid).collect();
        assert!(cands.contains(&ConditionKind::Insomnia));
        assert!(!cands.contains(&ConditionKind::Withdrawal));
    }

    #[test]
    fn apply_comorbidities_co_onsets_and_is_deterministic() {
        let m = ComorbidityMatrix::default_matrix();
        // Find an actor whose PTSD→Depression roll lands below 0.30 (co-onset).
        let mut actor = None;
        for a in 0..500u64 {
            if mh_roll(0x1234, a, ConditionKind::Depression, SALT_COMORBID) < 0.30 {
                actor = Some(a);
                break;
            }
        }
        let actor_id = actor.expect("some actor co-onsets depression");

        let run = || {
            let mut s = ActorMentalHealth::with_origin(OriginId::Human);
            s.trigger(actor_id, ConditionKind::Ptsd, TriggerReason::WitnessDeaths, 0);
            let (trig, det) = apply_comorbidities(&mut s, &m, actor_id, ConditionKind::Ptsd, 0x1234, 0);
            (s, trig, det)
        };
        let (s, trig, det) = run();
        assert!(s.has(ConditionKind::Depression), "depression co-onset");
        assert_eq!(
            s.find(ConditionKind::Depression).unwrap().comorbid_with,
            Some(ConditionKind::Ptsd)
        );
        assert_eq!(s.find(ConditionKind::Depression).unwrap().stage, ConditionStage::Acute);
        assert!(trig.iter().any(|e| e.condition == ConditionKind::Depression));
        assert!(det.iter().any(|e| e.primary == ConditionKind::Ptsd && e.comorbid == ConditionKind::Depression));

        // Determinism: identical inputs reproduce the identical co-onset set.
        let (s2, _, det2) = run();
        assert_eq!(s.active.len(), s2.active.len());
        assert_eq!(det.len(), det2.len());
    }

    #[test]
    fn synthetic_origin_co_onsets_nothing() {
        let m = ComorbidityMatrix::default_matrix();
        let mut s = ActorMentalHealth::with_origin(OriginId::Robot);
        // Robots can't even hold the primary, but guard the path anyway.
        let (trig, det) = apply_comorbidities(&mut s, &m, 7, ConditionKind::Ptsd, 0x1234, 0);
        assert!(trig.is_empty() && det.is_empty());
    }

    #[test]
    fn detect_present_surfaces_addiction_withdrawal() {
        let m = ComorbidityMatrix::default_matrix();
        let mut s = ActorMentalHealth::with_origin(OriginId::Human);
        s.trigger(7, ConditionKind::Addiction, TriggerReason::DrugDoses, 0);
        s.trigger(7, ConditionKind::Withdrawal, TriggerReason::DrugAbsence, 10);
        let det = detect_present(&s, &m, 7, 10);
        assert!(det
            .iter()
            .any(|e| e.primary == ConditionKind::Addiction && e.comorbid == ConditionKind::Withdrawal));
    }

    #[test]
    fn matrix_round_trips_through_ron() {
        let m = ComorbidityMatrix::default_matrix();
        let s = ron::to_string(&m).unwrap();
        let back: ComorbidityMatrix = ron::from_str(&s).unwrap();
        assert_eq!(m, back);
    }
}
