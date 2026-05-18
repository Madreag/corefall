//! **M12C**: Per-storyteller cinematic profile.
//!
//! Per spec § "Per-storyteller cinematic style":
//!
//! | Storyteller | Camera bias | Audio bias | Color grade bias | Pacing |
//! |---|---|---|---|---|
//! | Cassandra Classic | Slow dolly + low-angle dread shots | Deep cello bed | -15% sat, -8% value | 15-25s per shot |
//! | Phoebe Chillax | Steady medium shots | Warm strings | +10% sat, +5% value | 8-15s per shot |
//! | Randy Random | Whip-pans + dutch tilt | Percussive synth | +20% contrast | 3-8s per shot |
//! | Ironman | Locked-off heroic | Marching drum | Stark high-key | 10-18s per shot |
//! | Sandbox | Plain establishing wide | Ambient bed | Default neutral | Cinematic SUPPRESSED |
//!
//! Per spec § Notes for the implementer: "The 'Sandbox suppresses
//! cinematics' rule is implemented by the storyteller profile, not by
//! the cinematic kernel; this keeps the suppress logic data-driven and
//! lets mods author their own suppress-cinematics storytellers."

use serde::{Deserialize, Serialize};

/// Storyteller identifier matching M25 director state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorytellerId {
    /// Cassandra Classic — dread + slow burn.
    CassandraClassic,
    /// Phoebe Chillax — quirky + warm.
    PhoebeChillax,
    /// Randy Random — chaotic + percussive.
    RandyRandom,
    /// Ironman — stoic + locked-off.
    Ironman,
    /// Sandbox — neutral; cinematics suppressed entirely.
    Sandbox,
}

impl StorytellerId {
    /// Canonical snake_case identifier (matches the `gameplay.storyteller`
    /// setting + on-disk RON id).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            StorytellerId::CassandraClassic => "cassandra_classic",
            StorytellerId::PhoebeChillax => "phoebe_chillax",
            StorytellerId::RandyRandom => "randy_random",
            StorytellerId::Ironman => "ironman",
            StorytellerId::Sandbox => "sandbox",
        }
    }

    /// Parse from canonical snake_case identifier. Unknown → `None`.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        Some(match s {
            "cassandra_classic" | "Cassandra Classic" => StorytellerId::CassandraClassic,
            "phoebe_chillax" | "Phoebe Chillax" => StorytellerId::PhoebeChillax,
            "randy_random" | "Randy Random" => StorytellerId::RandyRandom,
            "ironman" | "Ironman" => StorytellerId::Ironman,
            "sandbox" | "Sandbox" => StorytellerId::Sandbox,
            _ => return None,
        })
    }
}

/// Color-grade adjustments applied to the cinematic image. Values are
/// multiplicative deltas around 1.0 ("no change") for saturation +
/// value + contrast; the spec quotes them as percentages.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ColorGradeBias {
    /// Saturation multiplier. `1.0` = neutral; `0.85` = -15%; `1.10` = +10%.
    pub saturation: f32,
    /// Value multiplier (HSV V). `1.0` = neutral.
    pub value: f32,
    /// Contrast multiplier (post-tonemap). `1.0` = neutral; `1.20` = +20%.
    pub contrast: f32,
}

impl Default for ColorGradeBias {
    fn default() -> Self {
        COLOR_GRADE_NEUTRAL
    }
}

/// Neutral grade (Sandbox + default). Used by the renderer when no
/// storyteller profile is active.
pub const COLOR_GRADE_NEUTRAL: ColorGradeBias = ColorGradeBias {
    saturation: 1.0,
    value: 1.0,
    contrast: 1.0,
};

/// Storyteller profile — biases applied to the cinematic kernel at
/// playback time. Loaded from
/// `content/cinematics/storyteller_profiles/<id>.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorytellerProfile {
    /// Identifier matching the on-disk filename stem.
    pub id: StorytellerId,
    /// True = drop all cinematics; player goes straight to gameplay UI
    /// (Sandbox). Per spec § "Sandbox skips entirely".
    #[serde(default)]
    pub suppress_cinematics: bool,
    /// Inclusive shot-dwell window. Per spec Pacing column (e.g.
    /// `[15_000, 25_000]` for Cassandra). The kernel uses this to
    /// validate authored scripts + to bias auto-generated cinematics.
    pub shot_dwell_ms: [u32; 2],
    /// Audio bed asset id (matches `cf-asset-ledger` canonical_name).
    /// Empty for Sandbox (ambient-only).
    #[serde(default)]
    pub audio_bed_id: String,
    /// LUFS target for the music bed during narration windows. Per
    /// spec § "ambient music ducked to -22 LUFS during narration
    /// windows".
    #[serde(default = "default_music_lufs_during_narration")]
    pub music_lufs_during_narration: f32,
    /// LUFS target for the music bed outside narration windows.
    #[serde(default = "default_music_lufs_outside_narration")]
    pub music_lufs_outside_narration: f32,
    /// Color grade biases.
    pub color_grade: ColorGradeBias,
    /// Per-storyteller narrator voice id. Per spec § "per-storyteller
    /// voice from M37A".
    #[serde(default)]
    pub narrator_voice_id: String,
}

fn default_music_lufs_during_narration() -> f32 {
    -22.0
}

fn default_music_lufs_outside_narration() -> f32 {
    -16.0
}

impl StorytellerProfile {
    /// Average of `shot_dwell_ms` (ms). Used by the scheduler to bias
    /// shot pacing when authoring auto-generated cinematics.
    #[must_use]
    pub fn average_shot_dwell_ms(&self) -> u32 {
        (self.shot_dwell_ms[0] + self.shot_dwell_ms[1]) / 2
    }
}

/// Built-in profiles for the 5 launch storytellers. Falls back to these
/// when the on-disk RON profile is missing. Per spec § "5 storytellers ×
/// 3 variants" + § "5 storyteller-specific finales".
///
/// Defined as a function (not `const`) because `StorytellerProfile`
/// carries `String` fields that aren't `const`-constructible. The
/// runtime cost is trivial — five `String::new()` allocations the
/// first time each profile is requested, and the underlying string
/// data is never written to.
#[must_use]
pub fn default_profiles() -> [StorytellerProfile; 5] {
    [
        StorytellerProfile {
            id: StorytellerId::CassandraClassic,
            suppress_cinematics: false,
            shot_dwell_ms: [15_000, 25_000],
            audio_bed_id: String::new(),
            music_lufs_during_narration: -22.0,
            music_lufs_outside_narration: -16.0,
            color_grade: ColorGradeBias {
                saturation: 0.85,
                value: 0.92,
                contrast: 1.0,
            },
            narrator_voice_id: String::new(),
        },
        StorytellerProfile {
            id: StorytellerId::PhoebeChillax,
            suppress_cinematics: false,
            shot_dwell_ms: [8_000, 15_000],
            audio_bed_id: String::new(),
            music_lufs_during_narration: -22.0,
            music_lufs_outside_narration: -16.0,
            color_grade: ColorGradeBias {
                saturation: 1.10,
                value: 1.05,
                contrast: 1.0,
            },
            narrator_voice_id: String::new(),
        },
        StorytellerProfile {
            id: StorytellerId::RandyRandom,
            suppress_cinematics: false,
            shot_dwell_ms: [3_000, 8_000],
            audio_bed_id: String::new(),
            music_lufs_during_narration: -22.0,
            music_lufs_outside_narration: -16.0,
            color_grade: ColorGradeBias {
                saturation: 1.0,
                value: 1.0,
                contrast: 1.20,
            },
            narrator_voice_id: String::new(),
        },
        StorytellerProfile {
            id: StorytellerId::Ironman,
            suppress_cinematics: false,
            shot_dwell_ms: [10_000, 18_000],
            audio_bed_id: String::new(),
            music_lufs_during_narration: -22.0,
            music_lufs_outside_narration: -16.0,
            color_grade: ColorGradeBias {
                saturation: 1.0,
                value: 1.05,
                contrast: 1.10,
            },
            narrator_voice_id: String::new(),
        },
        StorytellerProfile {
            id: StorytellerId::Sandbox,
            suppress_cinematics: true,
            shot_dwell_ms: [0, 0],
            audio_bed_id: String::new(),
            music_lufs_during_narration: -22.0,
            music_lufs_outside_narration: -16.0,
            color_grade: COLOR_GRADE_NEUTRAL,
            narrator_voice_id: String::new(),
        },
    ]
}

/// Look up the built-in profile for `id`.
#[must_use]
pub fn builtin_profile(id: StorytellerId) -> StorytellerProfile {
    default_profiles()
        .into_iter()
        .find(|p| p.id == id)
        .expect("default_profiles covers every storyteller")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cassandra_dwell_window_matches_spec() {
        let p = builtin_profile(StorytellerId::CassandraClassic);
        assert_eq!(p.shot_dwell_ms, [15_000, 25_000]);
        assert!((p.color_grade.saturation - 0.85).abs() < 1e-3);
        assert!((p.color_grade.value - 0.92).abs() < 1e-3);
    }

    #[test]
    fn randy_dwell_window_matches_spec() {
        let p = builtin_profile(StorytellerId::RandyRandom);
        assert_eq!(p.shot_dwell_ms, [3_000, 8_000]);
        assert!((p.color_grade.contrast - 1.20).abs() < 1e-3);
    }

    #[test]
    fn sandbox_suppresses_cinematics() {
        let p = builtin_profile(StorytellerId::Sandbox);
        assert!(p.suppress_cinematics);
        assert_eq!(p.color_grade, COLOR_GRADE_NEUTRAL);
    }

    #[test]
    fn storyteller_id_round_trips_through_string() {
        for id in [
            StorytellerId::CassandraClassic,
            StorytellerId::PhoebeChillax,
            StorytellerId::RandyRandom,
            StorytellerId::Ironman,
            StorytellerId::Sandbox,
        ] {
            assert_eq!(StorytellerId::from_str(id.as_str()), Some(id));
        }
    }

    #[test]
    fn storyteller_id_parses_display_names() {
        assert_eq!(
            StorytellerId::from_str("Cassandra Classic"),
            Some(StorytellerId::CassandraClassic)
        );
        assert_eq!(
            StorytellerId::from_str("Phoebe Chillax"),
            Some(StorytellerId::PhoebeChillax)
        );
    }
}
