//! **M12A** § Bus mixer — master / SFX / music / voice / ambient buses.
//!
//! Per spec § Files: `game/crates/cf-audio/src/mix.rs` (NEW: master / SFX
//! / music / voice / ambient buses). Bus volumes mirror the live
//! `cf-control::Settings.audio.{master,sfx,music,voice,ambient}_volume`
//! sliders so the player's Settings → Audio tab actually attenuates the
//! corresponding bus at the mixer level.
//!
//! Loudness normalization is enforced at GENERATION (the bake pipeline
//! normalizes every SFX to -16 LUFS); runtime mixing is purely linear
//! gain to preserve replay-determinism. Per the spec pitfall: "cf-audio
//! does NOT do runtime normalization (would hurt determinism)".

use serde::{Deserialize, Serialize};

/// **M12A** § Bus taxonomy — one fader per `Settings.audio.*_volume`
/// slider. cf-app mirrors each into the matching `MixBuses` field every
/// frame.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum AudioBus {
    Master,
    Sfx,
    Music,
    Voice,
    Ambient,
}

impl AudioBus {
    /// Snake_case identifier matching the `Settings.audio.*_volume` field.
    pub fn as_str(self) -> &'static str {
        match self {
            AudioBus::Master => "master",
            AudioBus::Sfx => "sfx",
            AudioBus::Music => "music",
            AudioBus::Voice => "voice",
            AudioBus::Ambient => "ambient",
        }
    }

    /// Parse from snake_case wire form.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<AudioBus> {
        Some(match s {
            "master" => AudioBus::Master,
            "sfx" => AudioBus::Sfx,
            "music" => AudioBus::Music,
            "voice" => AudioBus::Voice,
            "ambient" => AudioBus::Ambient,
            _ => return None,
        })
    }
}

/// Per-bus fader state. Volumes are linear `[0.0, 1.0]` — the bake
/// pipeline ensures the un-mixed LUFS is consistent across SFX, so
/// linear faders sound correct without runtime renormalization.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MixBuses {
    /// Master fader; multiplies every other bus.
    pub master: f32,
    /// All non-music, non-voice SFX (weapons, footsteps, impacts).
    pub sfx: f32,
    /// Music tracks (intro + ambient + boss + adaptive).
    pub music: f32,
    /// Voice lines (NPC dialog + storyteller narration + boss).
    pub voice: f32,
    /// Ambient loops (world ambient + weather + hazard).
    pub ambient: f32,
}

impl Default for MixBuses {
    fn default() -> Self {
        // Default sliders match `cf-shell::state::SettingsScaffold`
        // audio defaults: master=0.7, sfx=0.7, music=0.5, voice=0.8,
        // ambient=0.6.
        Self {
            master: 0.7,
            sfx: 0.7,
            music: 0.5,
            voice: 0.8,
            ambient: 0.6,
        }
    }
}

impl MixBuses {
    /// Effective playback gain for `bus`: `master × bus_volume`. Returned
    /// in `[0, 1]`.
    #[must_use]
    pub fn effective_gain(&self, bus: AudioBus) -> f32 {
        let master = self.master.clamp(0.0, 1.0);
        let bus_v = match bus {
            AudioBus::Master => 1.0,
            AudioBus::Sfx => self.sfx,
            AudioBus::Music => self.music,
            AudioBus::Voice => self.voice,
            AudioBus::Ambient => self.ambient,
        }
        .clamp(0.0, 1.0);
        master * bus_v
    }

    /// Set a single bus fader from a settings dispatch.
    pub fn set(&mut self, bus: AudioBus, value: f32) {
        let v = value.clamp(0.0, 1.0);
        match bus {
            AudioBus::Master => self.master = v,
            AudioBus::Sfx => self.sfx = v,
            AudioBus::Music => self.music = v,
            AudioBus::Voice => self.voice = v,
            AudioBus::Ambient => self.ambient = v,
        }
    }
}

/// Spec acceptance: "per-SFX target loudness within ±2 LU of declared
/// target". Bake-time normalization tolerance.
pub const LUFS_TOLERANCE: f32 = 2.0;

/// Spec acceptance: "No SFX clips above -1 dBFS peak". Bake-time peak
/// ceiling.
pub const PEAK_DBFS_CEILING: f32 = -1.0;

/// Spec § Architecture rules: "every SFX normalized to -16 LUFS
/// short-term per EBU R 128".
pub const DEFAULT_TARGET_LUFS: f32 = -16.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bus_round_trips_through_str() {
        for bus in [
            AudioBus::Master,
            AudioBus::Sfx,
            AudioBus::Music,
            AudioBus::Voice,
            AudioBus::Ambient,
        ] {
            assert_eq!(AudioBus::from_str(bus.as_str()), Some(bus));
        }
        assert!(AudioBus::from_str("garbage").is_none());
    }

    #[test]
    fn effective_gain_multiplies_master() {
        let b = MixBuses {
            master: 0.5,
            sfx: 0.8,
            music: 0.5,
            voice: 0.5,
            ambient: 0.5,
        };
        let sfx_gain = b.effective_gain(AudioBus::Sfx);
        assert!((sfx_gain - 0.4).abs() < 1e-4);
    }

    #[test]
    fn effective_gain_clamps_to_unit_range() {
        let b = MixBuses {
            master: 2.0,
            sfx: 2.0,
            music: 0.5,
            voice: 0.5,
            ambient: 0.5,
        };
        let gain = b.effective_gain(AudioBus::Sfx);
        assert!(gain <= 1.0 + 1e-4);
    }

    #[test]
    fn set_clamps_negative_and_oversized_values() {
        let mut b = MixBuses::default();
        b.set(AudioBus::Sfx, -1.0);
        assert!(b.sfx.abs() < 1e-4);
        b.set(AudioBus::Sfx, 5.0);
        assert!((b.sfx - 1.0).abs() < 1e-4);
    }

    #[test]
    fn default_buses_match_settings_defaults() {
        let b = MixBuses::default();
        assert!((b.master - 0.7).abs() < 1e-4);
        assert!((b.sfx - 0.7).abs() < 1e-4);
        assert!((b.music - 0.5).abs() < 1e-4);
        assert!((b.voice - 0.8).abs() < 1e-4);
        assert!((b.ambient - 0.6).abs() < 1e-4);
    }

    #[test]
    fn lufs_tolerance_and_peak_ceiling_constants_match_spec() {
        assert!((LUFS_TOLERANCE - 2.0).abs() < 1e-4);
        assert!((PEAK_DBFS_CEILING - -1.0).abs() < 1e-4);
        assert!((DEFAULT_TARGET_LUFS - -16.0).abs() < 1e-4);
    }
}
