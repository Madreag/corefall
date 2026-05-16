//! **M11**: banner stack widget per spec § Status banner taxonomy.
//! Re-exports the M4A banner stack + adds severity-sorted rendering with
//! ASCII glyphs (color-independent state labels).

use bevy::prelude::*;

use crate::HudBanner;

/// Severity bands per spec § "max 4 visible at once; oldest non-critical
/// evicts first; critical sticky; critical render BELOW warning + info".
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum BannerSeverity {
    Critical,
    Warning,
    Info,
}

impl BannerSeverity {
    /// snake_case identifier for cfctl wire form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            BannerSeverity::Critical => "critical",
            BannerSeverity::Warning => "warning",
            BannerSeverity::Info => "info",
        }
    }

    /// ASCII glyph (color-independent label per DR-012 ACC-A floor).
    #[must_use]
    pub fn glyph(self) -> &'static str {
        match self {
            BannerSeverity::Critical => "[!!]",
            BannerSeverity::Warning => "[!]",
            BannerSeverity::Info => "[*]",
        }
    }

    /// Parse from a (case-insensitive) wire string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "critical" => BannerSeverity::Critical,
            "warning" => BannerSeverity::Warning,
            _ => BannerSeverity::Info,
        }
    }
}

/// Max banners visible per spec § "max 4 visible at once".
pub const BANNER_STACK_MAX_VISIBLE: usize = 4;

/// Resource projection of the banner stack. Mirrors `HudState.banners` so
/// new code can depend on a dedicated state struct rather than reaching
/// into the HUD root.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct BannerStackState {
    pub banners: Vec<HudBanner>,
}

impl BannerStackState {
    /// Sort banners critical → warning → info with raised-at-tick as
    /// tiebreaker. Critical render below warning/info per spec
    /// peripheral-vision guidance.
    pub fn sorted(&self) -> Vec<&HudBanner> {
        let mut v: Vec<&HudBanner> = self.banners.iter().collect();
        v.sort_by_key(|b| match b.severity.as_str() {
            "critical" => (0_u8, std::cmp::Reverse(b.raised_at_tick)),
            "warning" => (1_u8, std::cmp::Reverse(b.raised_at_tick)),
            _ => (2_u8, std::cmp::Reverse(b.raised_at_tick)),
        });
        v.truncate(BANNER_STACK_MAX_VISIBLE);
        v
    }
}

/// Format a banner per the M4A banner_line() helper but using the M11
/// taxonomy enum.
#[must_use]
pub fn banner_text_line(banner: &HudBanner) -> String {
    let sev = BannerSeverity::from_str(&banner.severity);
    format!(
        "{glyph} {severity} {label}",
        glyph = sev.glyph(),
        severity = sev.as_str().to_uppercase(),
        label = banner.label,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(id: &str, sev: &str, label: &str, tick: u64) -> HudBanner {
        HudBanner {
            id: id.into(),
            severity: sev.into(),
            label: label.into(),
            raised_at_tick: tick,
        }
    }

    #[test]
    fn sort_critical_first_then_warning_then_info() {
        let s = BannerStackState {
            banners: vec![
                b("info1", "info", "INFO_FOO", 10),
                b("warn1", "warning", "HP_LOW", 20),
                b("crit1", "critical", "DYING", 30),
            ],
        };
        let sorted = s.sorted();
        assert_eq!(sorted[0].severity, "critical");
        assert_eq!(sorted[1].severity, "warning");
        assert_eq!(sorted[2].severity, "info");
    }

    #[test]
    fn ascii_glyphs_are_color_independent() {
        let banner = b("dying", "critical", "DYING", 1);
        let line = banner_text_line(&banner);
        assert!(line.starts_with("[!!]"));
        assert!(line.contains("CRITICAL"));
        assert!(line.contains("DYING"));
    }

    #[test]
    fn truncates_to_max_visible() {
        let banners: Vec<HudBanner> = (0..6).map(|i| b(&format!("b{i}"), "info", "X", i)).collect();
        let s = BannerStackState { banners };
        assert_eq!(s.sorted().len(), BANNER_STACK_MAX_VISIBLE);
    }
}
