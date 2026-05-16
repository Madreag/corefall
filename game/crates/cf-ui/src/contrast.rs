//! **M11**: high-contrast palette helpers per spec § Contrast palette
//! swap. Three modes: Standard (default), HighContrastDark (pure white on
//! solid black), HighContrastLight (pure black on solid white).

use bevy::prelude::*;

/// Contrast mode mirror of `cf_control::settings::ContrastMode`.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum ContrastModeUi {
    #[default]
    Standard,
    HighContrastDark,
    HighContrastLight,
}

impl ContrastModeUi {
    /// `true` for either high-contrast variant.
    #[must_use]
    pub fn is_high_contrast(self) -> bool {
        !matches!(self, ContrastModeUi::Standard)
    }

    /// Parse from a snake_case wire string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "high_contrast_dark" => ContrastModeUi::HighContrastDark,
            "high_contrast_light" => ContrastModeUi::HighContrastLight,
            _ => ContrastModeUi::Standard,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ContrastModeUi::Standard => "standard",
            ContrastModeUi::HighContrastDark => "high_contrast_dark",
            ContrastModeUi::HighContrastLight => "high_contrast_light",
        }
    }
}

/// HUD text color for the given contrast mode.
#[must_use]
pub fn text_color(mode: ContrastModeUi) -> Color {
    match mode {
        ContrastModeUi::Standard => Color::srgb(0.96, 0.96, 0.92),
        ContrastModeUi::HighContrastDark => Color::srgb(1.0, 1.0, 1.0),
        ContrastModeUi::HighContrastLight => Color::srgb(0.0, 0.0, 0.0),
    }
}

/// HUD strip background color for the given contrast mode.
#[must_use]
pub fn strip_bg_color(mode: ContrastModeUi) -> Color {
    match mode {
        ContrastModeUi::Standard => Color::srgba(0.0, 0.0, 0.0, 0.45),
        ContrastModeUi::HighContrastDark => Color::srgba(0.0, 0.0, 0.0, 1.0),
        ContrastModeUi::HighContrastLight => Color::srgba(1.0, 1.0, 1.0, 1.0),
    }
}

/// HUD banner background color for the given contrast mode + severity.
#[must_use]
pub fn banner_bg_color(mode: ContrastModeUi, severity: &str) -> Color {
    if mode.is_high_contrast() {
        return strip_bg_color(mode);
    }
    match severity {
        "critical" => Color::srgba(0.7, 0.05, 0.05, 0.85),
        "warning" => Color::srgba(0.7, 0.5, 0.0, 0.85),
        _ => Color::srgba(0.0, 0.0, 0.0, 0.6),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_palette_uses_neutral_text() {
        let c = text_color(ContrastModeUi::Standard);
        let srgb = c.to_srgba();
        assert!((srgb.red - 0.96).abs() < 0.01);
    }

    #[test]
    fn high_contrast_dark_uses_white_text() {
        let c = text_color(ContrastModeUi::HighContrastDark);
        let srgb = c.to_srgba();
        assert!((srgb.red - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn high_contrast_light_uses_black_text() {
        let c = text_color(ContrastModeUi::HighContrastLight);
        let srgb = c.to_srgba();
        assert!(srgb.red.abs() < f32::EPSILON);
    }

    #[test]
    fn high_contrast_collapses_banner_colors() {
        // In high-contrast mode the per-severity colors collapse to the
        // strip background so the ASCII glyph carries the signal.
        let c1 = banner_bg_color(ContrastModeUi::HighContrastDark, "critical");
        let c2 = banner_bg_color(ContrastModeUi::HighContrastDark, "info");
        assert_eq!(c1.to_srgba(), c2.to_srgba());
    }
}
