//! **M12** § Dynamic color grading per-scene mood.
//!
//! M12 promises "vivid color-rich illustrated aesthetic" — NOT a static
//! noir/monochrome filter. The grading shader applied to the final
//! composite reads a per-scene `SceneMood` resource and tints toward
//! either bright + saturated (daylight) OR cool + shifted (nighttime) OR
//! warm + flickered (hazard), but ALWAYS preserves at least the
//! [`MONOCHROME_FLOOR`] saturation floor so the player never drops into
//! grayscale.
//!
//! Per the M12 acceptance criterion:
//!
//! > Given a scenario with daylight, nighttime, and hazard segments
//! >   Then daylight scenes are bright + saturated
//! >   And nighttime scenes shift cool but never go monochrome
//! >   And hazard scenes shift warm but never go monochrome
//!
//! Producers (cf-mission scenario authors) set [`SceneMood::current`] when
//! a phase change emits `mission.director_phase_change`; cf-app mirrors it
//! into the [`ColorGradingState`] resource, and the post-process pass
//! samples [`ColorGradingState::current_grade`] to compute the final tint.

use bevy::prelude::*;

/// **M12**: per-scene mood. cf-mission scenario authors set this when
/// declaring scenario phases; the default is `Daylight` (bright + neutral).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub enum SceneMood {
    /// Bright, saturated, neutral tint (default).
    #[default]
    Daylight,
    /// Cool blue shift — nighttime / industrial-interior / vacuum.
    Nighttime,
    /// Warm orange-red shift — hazard / reactor-meltdown / sandstorm.
    Hazard,
    /// Cool desaturated drift — vacuum exposure / oxygen-loss visualization.
    Vacuum,
    /// Sickly green tint — chemical/toxin/disease vignette.
    Toxin,
}

impl SceneMood {
    /// Canonical snake_case identifier.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SceneMood::Daylight => "daylight",
            SceneMood::Nighttime => "nighttime",
            SceneMood::Hazard => "hazard",
            SceneMood::Vacuum => "vacuum",
            SceneMood::Toxin => "toxin",
        }
    }

    /// Parse from the snake_case wire form.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(value: &str) -> Option<SceneMood> {
        Some(match value {
            "daylight" => SceneMood::Daylight,
            "nighttime" => SceneMood::Nighttime,
            "hazard" => SceneMood::Hazard,
            "vacuum" => SceneMood::Vacuum,
            "toxin" => SceneMood::Toxin,
            _ => return None,
        })
    }
}

/// **M12**: minimum saturation floor — every grading mode preserves AT
/// LEAST this much saturation so a scene never collapses to pure grayscale.
/// Per the acceptance criterion "nighttime scenes shift cool but never go
/// monochrome / hazard scenes shift warm but never go monochrome".
pub const MONOCHROME_FLOOR: f32 = 0.45;

/// **M12**: one fully-resolved color grade derived from a [`SceneMood`] +
/// intensity. Consumers feed this into the post-process shader.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorGrade {
    /// Per-channel tint multipliers (post-tonemap, multiplicative).
    pub tint_rgb: [f32; 3],
    /// Final saturation (`0.0` = grayscale, `1.0` = fully saturated).
    /// Guaranteed `>= MONOCHROME_FLOOR` for every grade.
    pub saturation: f32,
    /// Final brightness multiplier (`1.0` = baseline).
    pub brightness: f32,
    /// Cool ↔ warm temperature shift in arbitrary units (`-1.0` cool,
    /// `1.0` warm, `0.0` neutral).
    pub temperature: f32,
}

impl ColorGrade {
    /// Identity (neutral, untinted, fully saturated).
    #[must_use]
    pub fn identity() -> Self {
        Self {
            tint_rgb: [1.0, 1.0, 1.0],
            saturation: 1.0,
            brightness: 1.0,
            temperature: 0.0,
        }
    }

    /// Linear blend between two grades (`0.0` = `from`, `1.0` = `to`).
    #[must_use]
    pub fn blend(from: ColorGrade, to: ColorGrade, t: f32) -> ColorGrade {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Self {
            tint_rgb: [
                from.tint_rgb[0] * inv + to.tint_rgb[0] * t,
                from.tint_rgb[1] * inv + to.tint_rgb[1] * t,
                from.tint_rgb[2] * inv + to.tint_rgb[2] * t,
            ],
            saturation: (from.saturation * inv + to.saturation * t).max(MONOCHROME_FLOOR),
            brightness: from.brightness * inv + to.brightness * t,
            temperature: from.temperature * inv + to.temperature * t,
        }
    }
}

impl Default for ColorGrade {
    fn default() -> Self {
        Self::identity()
    }
}

/// **M12**: resolve the canonical color grade for `mood` at full intensity.
/// Per the M12 spec § Visual direction "Color grading — Dynamic per-scene
/// + mission mood".
#[must_use]
pub fn grade_for_mood(mood: SceneMood) -> ColorGrade {
    match mood {
        SceneMood::Daylight => ColorGrade {
            tint_rgb: [1.05, 1.02, 0.98],
            saturation: 1.0,
            brightness: 1.05,
            temperature: 0.1,
        },
        SceneMood::Nighttime => ColorGrade {
            tint_rgb: [0.78, 0.82, 1.0],
            saturation: 0.7,
            brightness: 0.75,
            temperature: -0.5,
        },
        SceneMood::Hazard => ColorGrade {
            tint_rgb: [1.15, 0.85, 0.7],
            saturation: 0.85,
            brightness: 1.0,
            temperature: 0.6,
        },
        SceneMood::Vacuum => ColorGrade {
            tint_rgb: [0.85, 0.88, 0.95],
            saturation: MONOCHROME_FLOOR,
            brightness: 0.65,
            temperature: -0.3,
        },
        SceneMood::Toxin => ColorGrade {
            tint_rgb: [0.85, 1.1, 0.85],
            saturation: 0.75,
            brightness: 0.9,
            temperature: -0.1,
        },
    }
}

/// **M12**: scene-mood state resource. cf-app pushes `current` from the
/// mission/scenario phase change; the post-process pass reads
/// [`ColorGradingState::current_grade`].
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ColorGradingState {
    /// Active mood used for the current frame's grade.
    pub current: SceneMood,
    /// Optional in-flight transition (target + progress 0..1).
    pub transition: Option<(SceneMood, f32)>,
    /// `0.0..=1.0` intensity multiplier (1.0 = full grade; 0.0 = identity).
    /// Defaults to `1.0`. Drops to `0.0` only when the high-contrast palette
    /// mode is active (the palette swap owns the visuals; grading stands
    /// down).
    pub intensity: f32,
}

impl Default for ColorGradingState {
    fn default() -> Self {
        Self {
            current: SceneMood::Daylight,
            transition: None,
            intensity: 1.0,
        }
    }
}

impl ColorGradingState {
    /// Start a smooth transition to `target`. Sets `transition` to
    /// `(target, 0.0)`; subsequent calls to [`Self::tick`] interpolate the
    /// progress.
    pub fn cross_fade_to(&mut self, target: SceneMood) {
        if self.current == target && self.transition.is_none() {
            return;
        }
        self.transition = Some((target, 0.0));
    }

    /// Advance the cross-fade (`step` is a per-tick fraction; `0.01` ≈
    /// 100-frame fade at 60 Hz). When the fade reaches `1.0`, `current`
    /// snaps to the target.
    pub fn tick(&mut self, step: f32) {
        let Some((target, mut t)) = self.transition else {
            return;
        };
        t = (t + step.max(0.0)).clamp(0.0, 1.0);
        if t >= 1.0 {
            self.current = target;
            self.transition = None;
        } else {
            self.transition = Some((target, t));
        }
    }

    /// Resolve the final per-frame grade after applying any in-flight
    /// transition + intensity scaling.
    #[must_use]
    pub fn current_grade(&self) -> ColorGrade {
        let base = grade_for_mood(self.current);
        let resolved = match self.transition {
            Some((target, t)) => ColorGrade::blend(base, grade_for_mood(target), t),
            None => base,
        };
        ColorGrade::blend(ColorGrade::identity(), resolved, self.intensity.clamp(0.0, 1.0))
    }
}

/// **M12**: grading plugin. cf-app wires this alongside [`crate::JuicePlugin`].
pub struct ColorGradingPlugin;

impl Plugin for ColorGradingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ColorGradingState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mood_round_trips_through_str() {
        for m in [
            SceneMood::Daylight,
            SceneMood::Nighttime,
            SceneMood::Hazard,
            SceneMood::Vacuum,
            SceneMood::Toxin,
        ] {
            assert_eq!(SceneMood::from_str(m.as_str()), Some(m));
        }
        assert_eq!(SceneMood::from_str("garbage"), None);
    }

    #[test]
    fn daylight_is_bright_and_saturated() {
        let g = grade_for_mood(SceneMood::Daylight);
        assert!(g.saturation >= 0.9, "saturation = {}", g.saturation);
        assert!(g.brightness > 1.0);
        assert!(g.temperature >= 0.0);
    }

    #[test]
    fn nighttime_shifts_cool_but_never_monochrome() {
        let g = grade_for_mood(SceneMood::Nighttime);
        assert!(g.temperature < 0.0, "should be cool");
        assert!(g.saturation >= MONOCHROME_FLOOR, "must preserve saturation floor");
        assert!(g.brightness < 1.0, "nighttime is dimmer");
        // Cool tint = blue channel >= green >= red.
        assert!(g.tint_rgb[2] >= g.tint_rgb[1]);
        assert!(g.tint_rgb[1] >= g.tint_rgb[0]);
    }

    #[test]
    fn hazard_shifts_warm_but_never_monochrome() {
        let g = grade_for_mood(SceneMood::Hazard);
        assert!(g.temperature > 0.0, "should be warm");
        assert!(g.saturation >= MONOCHROME_FLOOR);
        // Warm tint = red channel highest.
        assert!(g.tint_rgb[0] >= g.tint_rgb[1]);
        assert!(g.tint_rgb[1] >= g.tint_rgb[2]);
    }

    #[test]
    fn vacuum_holds_at_monochrome_floor() {
        let g = grade_for_mood(SceneMood::Vacuum);
        assert!(g.saturation >= MONOCHROME_FLOOR);
        assert!(g.saturation <= MONOCHROME_FLOOR + 0.05);
    }

    #[test]
    fn toxin_tint_is_green_dominant() {
        let g = grade_for_mood(SceneMood::Toxin);
        assert!(g.tint_rgb[1] > g.tint_rgb[0]);
        assert!(g.tint_rgb[1] > g.tint_rgb[2]);
    }

    #[test]
    fn blend_respects_monochrome_floor() {
        let a = grade_for_mood(SceneMood::Daylight);
        let b = grade_for_mood(SceneMood::Vacuum);
        let mid = ColorGrade::blend(a, b, 0.5);
        assert!(mid.saturation >= MONOCHROME_FLOOR);
    }

    #[test]
    fn cross_fade_advances_to_target() {
        let mut s = ColorGradingState::default();
        s.cross_fade_to(SceneMood::Nighttime);
        for _ in 0..120 {
            s.tick(0.01);
        }
        assert_eq!(s.current, SceneMood::Nighttime);
        assert!(s.transition.is_none());
    }

    #[test]
    fn current_grade_at_zero_intensity_is_identity() {
        let mut s = ColorGradingState {
            current: SceneMood::Hazard,
            transition: None,
            intensity: 0.0,
        };
        s.tick(0.0);
        let g = s.current_grade();
        assert!((g.saturation - 1.0).abs() < 1e-3);
        assert!((g.brightness - 1.0).abs() < 1e-3);
    }

    #[test]
    fn cross_fade_redundant_call_is_idempotent() {
        let mut s = ColorGradingState::default();
        s.cross_fade_to(SceneMood::Daylight);
        assert!(s.transition.is_none());
    }
}
