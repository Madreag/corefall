//! M9 — Timer warning HUD captions (defended-actor mission).
//!
//! Spec § Player narrative flow — at 30s / 15s / 5s remaining the HUD
//! shows a banner-style caption: "30 SECONDS — HOLD" / "15 SECONDS —
//! REACTOR STRESSED" / "5 SECONDS — HOLD THE LINE". The HUD also tints
//! the timer text: green > 30s, yellow 10-30s, red < 10s. Captions
//! mirror the `mission.timer_warning_threshold` events emitted by the
//! engine (single-shot per threshold per run).

use bevy::prelude::*;

/// Severity band for the caption + timer color.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TimerSeverity {
    Info,
    Warning,
    Critical,
}

impl TimerSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimerSeverity::Info => "info",
            TimerSeverity::Warning => "warning",
            TimerSeverity::Critical => "critical",
        }
    }
}

/// One captured warning: threshold + caption text + severity.
#[derive(Debug, Clone, PartialEq)]
pub struct TimerWarning {
    pub threshold_s: u32,
    pub remaining_s: u32,
    pub caption: String,
    pub severity: TimerSeverity,
}

/// Mirrors the engine's `TIMER_WARNING_THRESHOLDS_S` from cf-mission.
pub const WARNING_THRESHOLDS: &[(u32, TimerSeverity, &str)] = &[
    (30, TimerSeverity::Warning, "30 SECONDS — HOLD"),
    (15, TimerSeverity::Warning, "15 SECONDS — REACTOR STRESSED"),
    (5, TimerSeverity::Critical, "5 SECONDS — HOLD THE LINE"),
];

/// Timer-color band per spec § Outcomes — green > 30s, yellow 10-30s,
/// red < 10s.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum TimerColor {
    Green,
    Yellow,
    Red,
}

impl TimerColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            TimerColor::Green => "green",
            TimerColor::Yellow => "yellow",
            TimerColor::Red => "red",
        }
    }

    #[must_use]
    pub fn from_remaining_s(remaining_s: u32) -> Self {
        if remaining_s > 30 {
            TimerColor::Green
        } else if remaining_s >= 10 {
            TimerColor::Yellow
        } else {
            TimerColor::Red
        }
    }
}

/// Bevy resource holding the active warning chain.
#[derive(Resource, Debug, Clone, Default)]
pub struct TimerWarningsState {
    pub warnings: Vec<TimerWarning>,
    pub last_color: Option<TimerColor>,
}

impl TimerWarningsState {
    /// Push a new warning matching the threshold; returns `true` if a new
    /// warning was added (i.e. this threshold hadn't fired yet). Mirrors
    /// the single-shot-per-threshold contract from the engine.
    pub fn push_threshold(&mut self, threshold_s: u32, remaining_s: u32) -> bool {
        if self.warnings.iter().any(|w| w.threshold_s == threshold_s) {
            return false;
        }
        let template = WARNING_THRESHOLDS.iter().find(|(t, _, _)| *t == threshold_s);
        let (severity, caption) = match template {
            Some((_, sev, cap)) => (*sev, (*cap).to_string()),
            None => (TimerSeverity::Info, format!("{threshold_s} SECONDS")),
        };
        self.warnings.push(TimerWarning {
            threshold_s,
            remaining_s,
            caption,
            severity,
        });
        true
    }

    pub fn update_color(&mut self, remaining_s: u32) -> TimerColor {
        let c = TimerColor::from_remaining_s(remaining_s);
        self.last_color = Some(c);
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threshold_emits_once_per_run() {
        let mut s = TimerWarningsState::default();
        assert!(s.push_threshold(30, 30));
        assert!(!s.push_threshold(30, 28));
        assert!(s.push_threshold(15, 15));
        assert!(s.push_threshold(5, 5));
        assert_eq!(s.warnings.len(), 3);
    }

    #[test]
    fn timer_color_bands() {
        assert_eq!(TimerColor::from_remaining_s(60), TimerColor::Green);
        assert_eq!(TimerColor::from_remaining_s(31), TimerColor::Green);
        assert_eq!(TimerColor::from_remaining_s(30), TimerColor::Yellow);
        assert_eq!(TimerColor::from_remaining_s(15), TimerColor::Yellow);
        assert_eq!(TimerColor::from_remaining_s(10), TimerColor::Yellow);
        assert_eq!(TimerColor::from_remaining_s(9), TimerColor::Red);
        assert_eq!(TimerColor::from_remaining_s(0), TimerColor::Red);
    }
}
