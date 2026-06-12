//! M16B — Medical Scanner device (M14H consumer).
//!
//! The scanner is a placed device that, after `scan_duration_seconds`,
//! diagnoses an actor's disease (reading stage + dose + age) and emits
//! `disease.diagnosed`. The diagnostic *report* surface lives in
//! `cf-treatment::scanner`; this module owns the device spec + the
//! diagnosis action.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use cf_disease::{
    lifecycle::{diagnose, ActorDiseases, DiseaseDiagnosedEvent},
    DiseaseKind, MEDICAL_SCANNER_T1_ID,
};
use serde::{Deserialize, Serialize};

/// Default scan duration in seconds.
pub const SCAN_DURATION_SECONDS_DEFAULT: f32 = 30.0;
/// Default diagnosis confidence for a T1 scanner.
pub const SCAN_CONFIDENCE_DEFAULT: f32 = 0.95;

/// Medical Scanner device spec, loaded from
/// `content/equipment/medical_scanner_t1.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MedicalScannerSpec {
    pub item_id: String,
    pub display_name: String,
    pub scan_duration_seconds: f32,
    pub diagnosis_confidence: f32,
    pub reads_dose: bool,
    pub reads_age: bool,
    pub tier: u8,
}

impl MedicalScannerSpec {
    pub fn t1_default() -> Self {
        Self {
            item_id: MEDICAL_SCANNER_T1_ID.to_string(),
            display_name: "Medical Scanner T1".to_string(),
            scan_duration_seconds: SCAN_DURATION_SECONDS_DEFAULT,
            diagnosis_confidence: SCAN_CONFIDENCE_DEFAULT,
            reads_dose: true,
            reads_age: true,
            tier: 1,
        }
    }

    /// Diagnose the active infection of `kind`, producing `disease.diagnosed`
    /// with the scanner's confidence. Returns `None` if the actor has no such
    /// infection.
    pub fn diagnose(
        &self,
        state: &ActorDiseases,
        actor_id: u64,
        kind: DiseaseKind,
        cumulative_dose: f32,
        actor_age_seconds: f32,
        tick: u64,
    ) -> Option<DiseaseDiagnosedEvent> {
        diagnose(
            state,
            actor_id,
            kind,
            self.diagnosis_confidence,
            if self.reads_dose { cumulative_dose } else { 0.0 },
            if self.reads_age { actor_age_seconds } else { 0.0 },
            tick,
        )
    }

    /// Diagnose every active infection on the actor (full scan report feed).
    pub fn diagnose_all(
        &self,
        state: &ActorDiseases,
        actor_id: u64,
        cumulative_dose: f32,
        actor_age_seconds: f32,
        tick: u64,
    ) -> Vec<DiseaseDiagnosedEvent> {
        state
            .active
            .iter()
            .filter_map(|d| self.diagnose(state, actor_id, d.kind, cumulative_dose, actor_age_seconds, tick))
            .collect()
    }
}

/// In-flight scan progress on one device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanInProgress {
    pub actor_id: u64,
    pub seconds_remaining: f32,
    pub completed: bool,
}

impl ScanInProgress {
    pub fn start(actor_id: u64, spec: &MedicalScannerSpec) -> Self {
        Self {
            actor_id,
            seconds_remaining: spec.scan_duration_seconds,
            completed: false,
        }
    }

    /// Advance the scan; returns true on the tick it completes.
    pub fn tick(&mut self, dt_seconds: f32) -> bool {
        if self.completed {
            return false;
        }
        self.seconds_remaining -= dt_seconds;
        if self.seconds_remaining <= 0.0 {
            self.seconds_remaining = 0.0;
            self.completed = true;
            return true;
        }
        false
    }
}

/// Load `medical_scanner_t1.ron` (or any scanner RON) from a path.
pub fn load_scanner_spec(path: &Path) -> Result<MedicalScannerSpec, ScannerLoadError> {
    let body = fs::read_to_string(path).map_err(|e| ScannerLoadError::Io(path.to_path_buf(), e.to_string()))?;
    match ron::from_str::<MedicalScannerSpec>(&body) {
        Ok(s) => Ok(s),
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "medical scanner spec parse failed");
            Err(ScannerLoadError::Parse(path.to_path_buf(), e.to_string()))
        }
    }
}

/// Load every scanner spec in a directory, keyed by item id.
pub fn load_scanner_dir(dir: &Path) -> Result<BTreeMap<String, MedicalScannerSpec>, ScannerLoadError> {
    let mut out = BTreeMap::new();
    if !dir.exists() {
        out.insert(MEDICAL_SCANNER_T1_ID.to_string(), MedicalScannerSpec::t1_default());
        return Ok(out);
    }
    for entry in fs::read_dir(dir).map_err(|e| ScannerLoadError::Io(dir.to_path_buf(), e.to_string()))?.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if !name.starts_with("medical_scanner") || path.extension().and_then(|s| s.to_str()) != Some("ron") {
            continue;
        }
        let spec = load_scanner_spec(&path)?;
        out.insert(spec.item_id.clone(), spec);
    }
    if out.is_empty() {
        out.insert(MEDICAL_SCANNER_T1_ID.to_string(), MedicalScannerSpec::t1_default());
    }
    Ok(out)
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ScannerLoadError {
    #[error("io error reading {0:?}: {1}")]
    Io(PathBuf, String),
    #[error("parse error in {0:?}: {1}")]
    Parse(PathBuf, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_disease::{lifecycle::expose, OriginId, TransmissionVector};

    #[test]
    fn scanner_completes_after_30s() {
        let spec = MedicalScannerSpec::t1_default();
        let mut scan = ScanInProgress::start(7, &spec);
        let mut done = false;
        for _ in 0..30 {
            if scan.tick(1.0) {
                done = true;
            }
        }
        assert!(done && scan.completed);
    }

    #[test]
    fn scanner_diagnoses_pneumonia_with_confidence() {
        let spec = MedicalScannerSpec::t1_default();
        let mut state = ActorDiseases::with_origin(OriginId::Human);
        expose(&mut state, 7, DiseaseKind::Pneumonia, TransmissionVector::Airborne, None, 0);
        state.find_mut(DiseaseKind::Pneumonia).unwrap().stage = cf_disease::lifecycle::DiseaseStage::Manifest;
        let ev = spec.diagnose(&state, 7, DiseaseKind::Pneumonia, 0.0, 1000.0, 50).expect("diagnosis");
        assert_eq!(ev.pathogen, DiseaseKind::Pneumonia);
        assert!((ev.confidence - 0.95).abs() < 1e-6);
        assert!((ev.age_seconds - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn scanner_diagnose_returns_none_for_healthy_actor() {
        let spec = MedicalScannerSpec::t1_default();
        let state = ActorDiseases::with_origin(OriginId::Human);
        assert!(spec.diagnose(&state, 7, DiseaseKind::Flu, 0.0, 0.0, 0).is_none());
    }
}
