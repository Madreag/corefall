//! **M11**: body silhouette widget per spec § "Body damage readability
//! (DR-003 closure detail)". Renders the 6-zone HUD silhouette with
//! per-zone HP% tinting. M11 ships the front-facing placeholder; side-view
//! sprite-flip is M13/M14 chassis.

use bevy::prelude::*;

use crate::HudBodySilhouette;

/// 5-tier color band per spec § Per-limb armor strip + body silhouette:
/// Pristine / Scratched / Cracked / Critical / Destroyed.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SilhouetteBand {
    Pristine,
    Scratched,
    Cracked,
    Critical,
    Destroyed,
}

impl SilhouetteBand {
    /// Map a normalized HP fraction to a band per spec thresholds.
    #[must_use]
    pub fn from_hp_fraction(hp: f32) -> Self {
        if hp >= 0.85 {
            SilhouetteBand::Pristine
        } else if hp >= 0.6 {
            SilhouetteBand::Scratched
        } else if hp >= 0.35 {
            SilhouetteBand::Cracked
        } else if hp > 0.0 {
            SilhouetteBand::Critical
        } else {
            SilhouetteBand::Destroyed
        }
    }

    /// Short ASCII glyph for the band (ACC-A color-independent label).
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            SilhouetteBand::Pristine => "[OK]",
            SilhouetteBand::Scratched => "[~]",
            SilhouetteBand::Cracked => "[!]",
            SilhouetteBand::Critical => "[!!]",
            SilhouetteBand::Destroyed => "[X]",
        }
    }

    /// snake_case identifier for the cfctl wire form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SilhouetteBand::Pristine => "pristine",
            SilhouetteBand::Scratched => "scratched",
            SilhouetteBand::Cracked => "cracked",
            SilhouetteBand::Critical => "critical",
            SilhouetteBand::Destroyed => "destroyed",
        }
    }
}

/// Resource projection of the body silhouette for the HUD. cf-app's bridge
/// writes this from the engine's `ActorObservation::body_silhouette` each
/// frame; cf-ui reads it to render the BODY line + per-zone glyphs.
#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct BodySilhouetteState {
    pub silhouette: HudBodySilhouette,
    /// `true` when the chassis is not attached and the silhouette is a
    /// scalar-HP placeholder per spec § "M11 placeholder true; M13 fills".
    pub placeholder: bool,
}

impl BodySilhouetteState {
    /// Compose the BODY line per spec § "BODY: H[OK]  T[~]  AL[!]  AR[!!]
    /// LL[OK] LR[X]" — color-independent, ASCII-only.
    #[must_use]
    pub fn body_line(&self) -> String {
        let s = &self.silhouette;
        format!(
            "BODY: H{} T{} AL{} AR{} LL{} LR{}",
            SilhouetteBand::from_hp_fraction(s.head_hp_pct).glyph(),
            SilhouetteBand::from_hp_fraction(s.torso_hp_pct).glyph(),
            SilhouetteBand::from_hp_fraction(s.arm_left_hp_pct).glyph(),
            SilhouetteBand::from_hp_fraction(s.arm_right_hp_pct).glyph(),
            SilhouetteBand::from_hp_fraction(s.leg_left_hp_pct).glyph(),
            SilhouetteBand::from_hp_fraction(s.leg_right_hp_pct).glyph(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_hp_renders_all_ok() {
        let state = BodySilhouetteState {
            silhouette: HudBodySilhouette::default(),
            placeholder: true,
        };
        let line = state.body_line();
        assert!(line.contains("H[OK]"));
        assert!(line.contains("LR[OK]"));
    }

    #[test]
    fn destroyed_arm_renders_x() {
        let sil = HudBodySilhouette {
            arm_left_hp_pct: 0.0,
            ..HudBodySilhouette::default()
        };
        let state = BodySilhouetteState {
            silhouette: sil,
            placeholder: true,
        };
        assert!(state.body_line().contains("AL[X]"));
    }

    #[test]
    fn band_thresholds_match_spec() {
        assert_eq!(SilhouetteBand::from_hp_fraction(1.0), SilhouetteBand::Pristine);
        assert_eq!(SilhouetteBand::from_hp_fraction(0.7), SilhouetteBand::Scratched);
        assert_eq!(SilhouetteBand::from_hp_fraction(0.4), SilhouetteBand::Cracked);
        assert_eq!(SilhouetteBand::from_hp_fraction(0.1), SilhouetteBand::Critical);
        assert_eq!(SilhouetteBand::from_hp_fraction(0.0), SilhouetteBand::Destroyed);
    }
}
