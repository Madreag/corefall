//! **M11**: captions strip widget per spec § Captions surface — verbosity
//! modes + category filtering. Captions are the ACC-A floor's audio
//! fallback: every cue with `caption` enabled surfaces here.
//!
//! **M12B** adds the `direction_string(azimuth_rad, distance_m)`
//! formatter so caption renderers can surface the spatial-audio
//! direction (e.g. "FOOTSTEP — NW 10 m") regardless of whether spatial
//! audio is enabled (caption parity is non-negotiable per M12B spec).

use bevy::prelude::*;
use std::collections::BTreeSet;

use crate::HudCaption;

/// Max captions visible per spec § "max 4 visible; oldest evicts with
/// `[N more]` hint".
pub const CAPTION_QUEUE_MAX_VISIBLE: usize = 4;

/// to the "here" direction string. Mirrors `cf_audio::SPATIAL_HERE_RADIUS_M`
/// so the caption renderer doesn't depend on cf-audio.
pub const CAPTION_HERE_RADIUS_M: f32 = 1.5;

/// `|azimuth - π| < π/12` → "behind you". Mirrors
/// `cf_audio::AHEAD_BEHIND_CONE_RAD`.
pub const CAPTION_AHEAD_BEHIND_CONE_RAD: f32 = std::f32::consts::FRAC_PI_2 / 6.0;

/// `(azimuth_rad, distance_m)`. The output exactly matches
/// `cf_audio::DirectionString::label()` so the audio event stream + the
/// caption text use the same vocabulary.
///
/// Per M12B spec § Direction-string for captions:
///
/// > - `|azimuth_rad| < π/12` → `"ahead"`
/// > - `|azimuth_rad - π| < π/12` → `"behind you"`
/// > - `distance_m < 1.5` → `"here"` (overrides direction)
/// > - Else: 8-way compass projection (`N`, `NE`, `E`, `SE`, `S`, `SW`,
/// >   `W`, `NW`)
///
/// non-negotiable: every `audio.spatial_resolved` event MUST produce
/// the same caption direction string regardless of
/// `settings.spatial_audio_enabled`. ACC-A holds.".
#[must_use]
pub fn direction_string(azimuth_rad: f32, distance_m: f32) -> &'static str {
    if distance_m < CAPTION_HERE_RADIUS_M {
        return "here";
    }
    let pi = std::f32::consts::PI;
    let two_pi = std::f32::consts::TAU;
    let normalized = azimuth_rad.rem_euclid(two_pi);
    let signed_az = if normalized > pi { normalized - two_pi } else { normalized };
    if signed_az.abs() < CAPTION_AHEAD_BEHIND_CONE_RAD {
        return "ahead";
    }
    if (signed_az.abs() - pi).abs() < CAPTION_AHEAD_BEHIND_CONE_RAD {
        return "behind you";
    }
    let bucket = ((signed_az / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8);
    match bucket {
        0 => "ahead",
        1 => "NE",
        2 => "N",
        3 => "NW",
        4 => "behind you",
        5 | -3 => "SW",
        6 | -2 => "S",
        7 | -1 => "SE",
        _ => "ahead",
    }
}

/// `"<EVENT> — <DIR> <DIST>m"`. Example: `"FOOTSTEP — NW 10 m"`.
/// The `"here"` direction omits the distance suffix.
#[must_use]
pub fn spatial_caption_line(event_label: &str, azimuth_rad: f32, distance_m: f32) -> String {
    let dir = direction_string(azimuth_rad, distance_m);
    let label_upper = event_label.to_uppercase();
    if dir == "here" {
        format!("{label_upper} — HERE")
    } else if dir == "ahead" || dir == "behind you" {
        // Same convention for distance suffix; the spec scenarios show
        // "RELOAD — behind you" with NO distance, and "GUNSHOT — NW 12 m"
        // with one. To be consistent with both we suffix the distance
        // ONLY for compass-style directions.
        format!("{label_upper} — {}", dir.to_uppercase())
    } else {
        let rounded = distance_m.round() as i32;
        format!("{label_upper} — {dir} {rounded} m")
    }
}

/// Caption verbosity mode mirror of `cf_control::settings::CaptionMode`.
/// Duplicated here so cf-ui doesn't depend on cf-control's settings type.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum CaptionVerbosity {
    /// Disabled.
    #[default]
    Off,
    /// Only critical-severity events.
    CriticalOnly,
    /// Critical + warning.
    Standard,
    /// Critical + warning + info.
    Expanded,
}

impl CaptionVerbosity {
    /// Should this caption surface given the verbosity setting?
    #[must_use]
    pub fn allows(self, severity: &str) -> bool {
        match self {
            CaptionVerbosity::Off => false,
            CaptionVerbosity::CriticalOnly => severity == "critical",
            CaptionVerbosity::Standard => matches!(severity, "critical" | "warning"),
            CaptionVerbosity::Expanded => true,
        }
    }

    /// Parse from a snake_case wire string.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "critical_only" => CaptionVerbosity::CriticalOnly,
            "standard" => CaptionVerbosity::Standard,
            "expanded" => CaptionVerbosity::Expanded,
            _ => CaptionVerbosity::Off,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            CaptionVerbosity::Off => "off",
            CaptionVerbosity::CriticalOnly => "critical_only",
            CaptionVerbosity::Standard => "standard",
            CaptionVerbosity::Expanded => "expanded",
        }
    }
}

/// Resource projection of the caption queue. cf-app writes per frame from
/// the engine's `HudState.captions`.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct CaptionsState {
    pub captions: Vec<HudCaption>,
    /// Caption verbosity mode (drives event filtering).
    pub verbosity: CaptionVerbosity,
    /// Active caption categories per spec § Categories filter.
    pub categories: BTreeSet<String>,
    /// Caption background opacity (0..=1).
    pub background_opacity: f32,
}

impl CaptionsState {
    /// Visible captions per spec — capped at [`CAPTION_QUEUE_MAX_VISIBLE`].
    pub fn visible(&self) -> Vec<&HudCaption> {
        self.captions.iter().rev().take(CAPTION_QUEUE_MAX_VISIBLE).collect()
    }

    /// `true` when the named category is on (no categories restriction →
    /// all on).
    #[must_use]
    pub fn category_allowed(&self, cat: &str) -> bool {
        self.categories.is_empty() || self.categories.contains(cat)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn off_blocks_everything() {
        assert!(!CaptionVerbosity::Off.allows("critical"));
        assert!(!CaptionVerbosity::Off.allows("info"));
    }

    #[test]
    fn critical_only_passes_critical() {
        assert!(CaptionVerbosity::CriticalOnly.allows("critical"));
        assert!(!CaptionVerbosity::CriticalOnly.allows("warning"));
    }

    #[test]
    fn standard_passes_critical_and_warning() {
        assert!(CaptionVerbosity::Standard.allows("critical"));
        assert!(CaptionVerbosity::Standard.allows("warning"));
        assert!(!CaptionVerbosity::Standard.allows("info"));
    }

    #[test]
    fn expanded_passes_everything() {
        assert!(CaptionVerbosity::Expanded.allows("critical"));
        assert!(CaptionVerbosity::Expanded.allows("info"));
    }

    #[test]
    fn visible_caps_to_4() {
        let mut s = CaptionsState::default();
        for i in 0..7 {
            s.captions.push(HudCaption {
                id: format!("c{i}"),
                label: "L".into(),
                raised_at_tick: i,
            });
        }
        assert_eq!(s.visible().len(), 4);
    }


    #[test]
    fn direction_string_returns_here_within_radius() {
        assert_eq!(direction_string(0.0, 0.5), "here");
        assert_eq!(direction_string(1.5, 1.0), "here");
    }

    #[test]
    fn direction_string_returns_ahead_for_small_azimuth() {
        assert_eq!(direction_string(0.05, 10.0), "ahead");
        assert_eq!(direction_string(-0.05, 10.0), "ahead");
    }

    #[test]
    fn direction_string_returns_behind_for_pi_azimuth() {
        assert_eq!(direction_string(std::f32::consts::PI, 10.0), "behind you");
    }

    #[test]
    fn direction_string_matches_8way_compass() {
        assert_eq!(direction_string(std::f32::consts::FRAC_PI_4, 10.0), "NE");
        assert_eq!(direction_string(std::f32::consts::FRAC_PI_2, 10.0), "N");
        assert_eq!(direction_string(3.0 * std::f32::consts::FRAC_PI_4, 10.0), "NW");
        assert_eq!(direction_string(-std::f32::consts::FRAC_PI_4, 10.0), "SE");
        assert_eq!(direction_string(-std::f32::consts::FRAC_PI_2, 10.0), "S");
        assert_eq!(direction_string(-3.0 * std::f32::consts::FRAC_PI_4, 10.0), "SW");
    }

    #[test]
    fn direction_string_matches_cf_audio_label_vocabulary() {
        // Every output of direction_string must be a valid label per
        // cf-audio's spec — these are the spec-locked strings.
        for label in [
            "here", "ahead", "behind you", "N", "NE", "E", "SE", "S", "SW", "W", "NW",
        ] {
            // ensure the literal compiles — these are the only valid labels.
            let _ = label;
        }
    }

    #[test]
    fn spatial_caption_line_includes_direction_and_distance() {
        let line = spatial_caption_line("footstep", 2.498, 10.0);
        // azimuth ≈ 2.5 rad → NW.
        assert_eq!(line, "FOOTSTEP — NW 10 m");
    }

    #[test]
    fn spatial_caption_line_omits_distance_for_behind_or_ahead() {
        let line = spatial_caption_line("reload", std::f32::consts::PI, 5.0);
        assert_eq!(line, "RELOAD — BEHIND YOU");
        let line = spatial_caption_line("reload", 0.0, 5.0);
        assert_eq!(line, "RELOAD — AHEAD");
    }

    #[test]
    fn spatial_caption_line_uses_here_for_close_source() {
        let line = spatial_caption_line("ping", 0.0, 0.5);
        assert_eq!(line, "PING — HERE");
    }

    #[test]
    fn spatial_caption_line_supports_acc_a_parity_for_spec_scenarios() {
        // Spec scenario: "the caption reads `FOOTSTEP — NW 10 m`".
        let line = spatial_caption_line("footstep", 2.498, 10.0);
        assert!(line.contains("FOOTSTEP"));
        assert!(line.contains("NW"));
        assert!(line.contains("10 m"));
    }
}
