//! **M14G** § Severity bands.
//!
//! 6-band severity ladder covering `[0, 1]`:
//!
//! | Band | Range | Label | Auto-triage threshold |
//! |------|-------|-------|-----------------------|
//! | Scratch  | `[0.00, 0.15)` | `[ . ] Scratch`  | `0.05` |
//! | Light    | `[0.15, 0.30)` | `[ * ] Light`    | `0.10` |
//! | Moderate | `[0.30, 0.50)` | `[**] Moderate`  | `0.20` |
//! | Severe   | `[0.50, 0.75)` | `[!] Severe`     | `0.30` |
//! | Critical | `[0.75, 0.90)` | `[!!] CRITICAL`  | `0.40` |
//! | Lethal   | `[0.90, 1.00]` | `[!!!] LETHAL`   | `0.50` |
//!
//! The `[!!] CRITICAL` label is locked by VAL-M14G-020 — do NOT change the
//! string without coordinating with `cf-ui::wound_strip` consumers.

use serde::{Deserialize, Serialize};

pub const BAND_LABEL_CRITICAL: &str = "[!!] CRITICAL";

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SeverityBand {
    Scratch = 0,
    Light = 1,
    Moderate = 2,
    Severe = 3,
    Critical = 4,
    Lethal = 5,
}

impl SeverityBand {
    pub const ALL: [SeverityBand; 6] = [
        SeverityBand::Scratch,
        SeverityBand::Light,
        SeverityBand::Moderate,
        SeverityBand::Severe,
        SeverityBand::Critical,
        SeverityBand::Lethal,
    ];

    /// Classify a normalized severity into its band. Inclusive on the
    /// lower bound, exclusive on the upper bound (except for Lethal which
    /// includes 1.0).
    pub fn from_severity(s: f32) -> Self {
        let s = s.clamp(0.0, 1.0);
        if s < 0.15 {
            SeverityBand::Scratch
        } else if s < 0.30 {
            SeverityBand::Light
        } else if s < 0.50 {
            SeverityBand::Moderate
        } else if s < 0.75 {
            SeverityBand::Severe
        } else if s < 0.90 {
            SeverityBand::Critical
        } else {
            SeverityBand::Lethal
        }
    }

    /// Human-readable band label surfaced on the silhouette badge.
    pub fn label(self) -> &'static str {
        match self {
            SeverityBand::Scratch => "[ . ] Scratch",
            SeverityBand::Light => "[ * ] Light",
            SeverityBand::Moderate => "[**] Moderate",
            SeverityBand::Severe => "[!] Severe",
            SeverityBand::Critical => BAND_LABEL_CRITICAL,
            SeverityBand::Lethal => "[!!!] LETHAL",
        }
    }

    /// 32-bit RGBA color for the band badge.
    pub fn color(self) -> [u8; 4] {
        match self {
            SeverityBand::Scratch => [200, 200, 200, 255],
            SeverityBand::Light => [255, 230, 120, 255],
            SeverityBand::Moderate => [255, 180, 60, 255],
            SeverityBand::Severe => [240, 80, 50, 255],
            SeverityBand::Critical => [180, 30, 30, 255],
            SeverityBand::Lethal => [80, 10, 10, 255],
        }
    }

    /// Auto-triage threshold delta added to the Medic utility scorer when
    /// this band of wound is present.
    pub fn auto_triage_threshold(self) -> f32 {
        match self {
            SeverityBand::Scratch => 0.05,
            SeverityBand::Light => 0.10,
            SeverityBand::Moderate => 0.20,
            SeverityBand::Severe => 0.30,
            SeverityBand::Critical => 0.40,
            SeverityBand::Lethal => 0.50,
        }
    }

    /// Treatment-difficulty curve sample per band.
    pub fn treatment_difficulty_curve(self) -> f32 {
        match self {
            SeverityBand::Scratch => 0.10,
            SeverityBand::Light => 0.20,
            SeverityBand::Moderate => 0.35,
            SeverityBand::Severe => 0.55,
            SeverityBand::Critical => 0.75,
            SeverityBand::Lethal => 1.00,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            SeverityBand::Scratch => "Scratch",
            SeverityBand::Light => "Light",
            SeverityBand::Moderate => "Moderate",
            SeverityBand::Severe => "Severe",
            SeverityBand::Critical => "Critical",
            SeverityBand::Lethal => "Lethal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M14G-004: SeverityBand boundary classification.
    #[test]
    fn band_boundary_table() {
        let cases: &[(f32, SeverityBand)] = &[
            (0.0, SeverityBand::Scratch),
            (0.149, SeverityBand::Scratch),
            (0.15, SeverityBand::Light),
            (0.299, SeverityBand::Light),
            (0.30, SeverityBand::Moderate),
            (0.499, SeverityBand::Moderate),
            (0.50, SeverityBand::Severe),
            (0.749, SeverityBand::Severe),
            (0.75, SeverityBand::Critical),
            (0.899, SeverityBand::Critical),
            (0.90, SeverityBand::Lethal),
            (1.0, SeverityBand::Lethal),
        ];
        for (s, expected) in cases {
            let got = SeverityBand::from_severity(*s);
            assert_eq!(got, *expected, "severity {s} → {got:?}, expected {expected:?}");
        }
    }

    /// VAL-M14G-005: each band exposes label, color, auto-triage threshold,
    /// and treatment-difficulty curve with pairwise distinctness and
    /// monotonic thresholds.
    #[test]
    fn band_metadata_per_band() {
        let labels: Vec<&str> = SeverityBand::ALL.iter().map(|b| b.label()).collect();
        let colors: Vec<[u8; 4]> = SeverityBand::ALL.iter().map(|b| b.color()).collect();
        let thresholds: Vec<f32> = SeverityBand::ALL.iter().map(|b| b.auto_triage_threshold()).collect();
        let curves: Vec<f32> = SeverityBand::ALL
            .iter()
            .map(|b| b.treatment_difficulty_curve())
            .collect();
        // Pairwise distinct labels.
        for i in 0..labels.len() {
            for j in (i + 1)..labels.len() {
                assert_ne!(labels[i], labels[j], "labels duplicated at {i}/{j}");
            }
        }
        // Pairwise distinct colors.
        for i in 0..colors.len() {
            for j in (i + 1)..colors.len() {
                assert_ne!(colors[i], colors[j], "colors duplicated at {i}/{j}");
            }
        }
        // Monotonic thresholds.
        for w in thresholds.windows(2) {
            assert!(w[0] < w[1], "auto-triage threshold not monotonic: {} → {}", w[0], w[1]);
        }
        // Monotonic difficulty curve.
        for w in curves.windows(2) {
            assert!(w[0] < w[1], "treatment-difficulty curve not monotonic: {} → {}", w[0], w[1]);
        }
        // Non-empty labels.
        for l in &labels {
            assert!(!l.is_empty());
        }
    }

    #[test]
    fn critical_label_locked() {
        assert_eq!(SeverityBand::Critical.label(), "[!!] CRITICAL");
        assert_eq!(SeverityBand::from_severity(0.85).label(), "[!!] CRITICAL");
    }
}
