//! **M14H** § Medical Scanner diagnostic surface.
//!
//! The Medical Scanner bridges M14G wounds + M16B diseases + M16C mental
//! health into a single "what does this actor need?" panel.
//!
//! Scan duration: 30s.

use serde::{Deserialize, Serialize};

use cf_wound::{SeverityBand, WoundKind};

pub const SCAN_DURATION_SECONDS_DEFAULT: f32 = 30.0;

/// **M14H** § single line on the scanner report (per-wound).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanWoundLine {
    pub wound_id: u64,
    pub kind: WoundKind,
    pub zone: String,
    pub severity: f32,
    pub band: SeverityBand,
    pub bandaged: bool,
    pub sutured: bool,
    pub dirt_pct: f32,
}

/// **M14H** § single line on the scanner report (per-disease).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanDiseaseLine {
    pub disease_id: String,
    pub stage: String,
    pub severity: f32,
}

/// **M14H** § single line on the scanner report (per-psych signal).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanPsychLine {
    pub signal_id: String,
    pub severity: f32,
}

/// **M14H** § scanner output snapshot consumed by the Patient Detail panel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScanReport {
    pub actor_id: u64,
    pub tick: u64,
    pub wounds: Vec<ScanWoundLine>,
    pub diseases: Vec<ScanDiseaseLine>,
    pub psych: Vec<ScanPsychLine>,
    pub pain_total: f32,
    pub compound_ttd_seconds: f32,
}

impl ScanReport {
    pub fn empty(actor_id: u64, tick: u64) -> Self {
        Self {
            actor_id,
            tick,
            wounds: Vec::new(),
            diseases: Vec::new(),
            psych: Vec::new(),
            pain_total: 0.0,
            compound_ttd_seconds: f32::INFINITY,
        }
    }
}

/// **M14H** § Medical Scanner state machine.
#[derive(Debug, Clone, PartialEq)]
pub struct MedicalScanner {
    pub actor_id: u64,
    pub duration_seconds: f32,
    pub seconds_remaining: f32,
    pub completed: bool,
    pub report: Option<ScanReport>,
}

impl MedicalScanner {
    pub fn new(actor_id: u64, duration_seconds: f32) -> Self {
        Self {
            actor_id,
            duration_seconds,
            seconds_remaining: duration_seconds,
            completed: false,
            report: None,
        }
    }

    pub fn tick(&mut self, dt_seconds: f32) {
        if self.completed {
            return;
        }
        self.seconds_remaining -= dt_seconds;
        if self.seconds_remaining <= 0.0 {
            self.completed = true;
        }
    }

    pub fn complete_with(&mut self, report: ScanReport) {
        self.completed = true;
        self.seconds_remaining = 0.0;
        self.report = Some(report);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scanner_completes_after_30s() {
        let mut s = MedicalScanner::new(7, SCAN_DURATION_SECONDS_DEFAULT);
        for _ in 0..30 {
            s.tick(1.0);
            if s.completed {
                break;
            }
        }
        assert!(s.completed);
    }

    #[test]
    fn scanner_report_completes_with_snapshot() {
        let mut s = MedicalScanner::new(7, SCAN_DURATION_SECONDS_DEFAULT);
        s.complete_with(ScanReport::empty(7, 100));
        assert!(s.report.is_some());
        assert!(s.completed);
    }
}
