//! **M11 / c4b4ea0**: triage window HUD widget per spec § Smart-AI HUD
//! widgets. Renders compound TTD as ONE integer + auto-rescue ETA. Reads
//! TTD floors via cf-actor's `TtdContract` trait so M17 can replace the
//! impl without changing this widget.

use bevy::prelude::*;

/// Box color per spec § "green box if auto-rescue arrives before TTD;
/// red box if TTD beats rescue; no box if no compound stack".
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TriageVerdict {
    /// No compound stack — widget hidden.
    Hidden,
    /// Auto-rescue arrives in time.
    GreenRescueInTime,
    /// Player must self-rescue (rescue too late).
    RedSelfRescue,
}

impl TriageVerdict {
    /// Derive verdict from compound TTD + auto-rescue ETA in seconds.
    /// Both should be in seconds.
    #[must_use]
    pub fn from_ttd_and_eta(compound_ttd_s: f32, rescue_eta_s: Option<f32>) -> Self {
        if !compound_ttd_s.is_finite() {
            return TriageVerdict::Hidden;
        }
        match rescue_eta_s {
            Some(eta) if eta.is_finite() && eta < compound_ttd_s => TriageVerdict::GreenRescueInTime,
            _ => TriageVerdict::RedSelfRescue,
        }
    }

    /// ASCII glyph for the verdict line (color-independent).
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            TriageVerdict::Hidden => "",
            TriageVerdict::GreenRescueInTime => "[OK]",
            TriageVerdict::RedSelfRescue => "[!!]",
        }
    }
}

/// One row in the per-affliction breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct TriageAffliction {
    /// Affliction name (snake_case).
    pub kind: String,
    /// Per-affliction TTD in seconds.
    pub ttd_seconds: f32,
}

/// Resource projection of the triage window. cf-app's bridge writes per
/// frame from the engine's affliction stack + cf-priority's Medic ETA.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct TriageWindowState {
    /// Compound TTD in whole seconds (one integer per spec).
    pub compound_ttd_s: f32,
    /// Auto-rescue ETA in seconds (`None` when no Medic in squad).
    pub rescue_eta_s: Option<f32>,
    /// Per-affliction breakdown (max ~5 rows; verbose mode shows more).
    pub afflictions: Vec<TriageAffliction>,
}

impl TriageWindowState {
    /// Derive the verdict from current state.
    #[must_use]
    pub fn verdict(&self) -> TriageVerdict {
        if self.afflictions.is_empty() {
            return TriageVerdict::Hidden;
        }
        TriageVerdict::from_ttd_and_eta(self.compound_ttd_s, self.rescue_eta_s)
    }

    /// Headline line per spec — `YOU DIE IN N s — RESCUE M s [GLYPH]`.
    #[must_use]
    pub fn headline(&self) -> String {
        let verdict = self.verdict();
        if verdict == TriageVerdict::Hidden {
            return String::new();
        }
        let glyph = verdict.glyph();
        let eta = self
            .rescue_eta_s
            .map(|e| format!("RESCUE {e:.0}s"))
            .unwrap_or_else(|| "NO RESCUE".to_string());
        format!("TRIAGE {} YOU DIE IN {:.0}s — {}", glyph, self.compound_ttd_s, eta)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_afflictions_renders_hidden() {
        let s = TriageWindowState::default();
        assert_eq!(s.verdict(), TriageVerdict::Hidden);
        assert_eq!(s.headline(), "");
    }

    #[test]
    fn rescue_in_time_renders_green() {
        let s = TriageWindowState {
            compound_ttd_s: 18.0,
            rescue_eta_s: Some(6.0),
            afflictions: vec![TriageAffliction {
                kind: "bleed_2w".into(),
                ttd_seconds: 18.0,
            }],
        };
        assert_eq!(s.verdict(), TriageVerdict::GreenRescueInTime);
        let head = s.headline();
        assert!(head.contains("[OK]"));
        assert!(head.contains("18s"));
        assert!(head.contains("RESCUE 6s"));
    }

    #[test]
    fn no_medic_renders_red() {
        let s = TriageWindowState {
            compound_ttd_s: 13.5,
            rescue_eta_s: None,
            afflictions: vec![
                TriageAffliction {
                    kind: "bleed_2w".into(),
                    ttd_seconds: 18.0,
                },
                TriageAffliction {
                    kind: "burning".into(),
                    ttd_seconds: 32.0,
                },
            ],
        };
        assert_eq!(s.verdict(), TriageVerdict::RedSelfRescue);
        assert!(s.headline().contains("NO RESCUE"));
    }
}
