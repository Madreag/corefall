//! **M12C** § "LUFS-aware narration/music/SFX duck profile during
//! cinematic playback".
//!
//! Per spec § "ElevenLabs narration sync" → Loudness contract:
//!
//! > narration mixed at -14 LUFS;
//! > ambient music ducked to -22 LUFS during narration windows;
//! > SFX held at -16 LUFS (per M12A audio mix).
//!
//! Per spec acceptance criterion "Cinematic mixer ducks music under
//! narration":
//!
//! > Given a cinematic is playing with narration starting at t=1500 ms
//! > And the music bed is at -16 LUFS at t=0
//! > When the narration word stream begins at t=1500 ms
//! >   Then the music bed ducks to -22 LUFS within 200 ms
//! >   And the narration plays at -14 LUFS
//! >   And SFX hold at -16 LUFS
//! > When narration ends at t=18000 ms
//! >   Then the music bed returns to -16 LUFS within 400 ms
//!
//! The mixer here is replay-deterministic: same script + same playhead
//! ms → same LUFS output. cf-app reads the resolved gain values per
//! frame to attenuate the live audio backend; the bake pipeline applies
//! the inverse so the mastered narration WAV matches the M12A
//! loudness contract.

use serde::{Deserialize, Serialize};

/// Per spec § Loudness contract — narration target LUFS.
pub const CINEMATIC_NARRATION_LUFS: f32 = -14.0;

/// Per spec § Loudness contract — music bed LUFS during narration.
pub const CINEMATIC_MUSIC_LUFS_DURING_NARRATION: f32 = -22.0;

/// Per spec § Loudness contract — music bed LUFS outside narration.
pub const CINEMATIC_MUSIC_LUFS_OUTSIDE_NARRATION: f32 = -16.0;

/// Per spec § Loudness contract — SFX bus LUFS during cinematics.
pub const CINEMATIC_SFX_LUFS: f32 = -16.0;

/// Per spec acceptance criterion: "music bed ducks to -22 LUFS WITHIN
/// 200 ms" (when narration begins).
pub const CINEMATIC_DUCK_ATTACK_MS: u32 = 200;

/// Per spec acceptance criterion: "music bed returns to -16 LUFS WITHIN
/// 400 ms" (when narration ends).
pub const CINEMATIC_DUCK_RELEASE_MS: u32 = 400;

/// Resolved per-frame LUFS values that cf-app feeds to the audio
/// backend during cinematic playback. The state machine is purely
/// driven by the cinematic playhead + the narration word stream.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CinematicMix {
    /// True while a cinematic is active.
    pub active: bool,
    /// Current music bed LUFS (eases between -16 and -22).
    pub music_lufs: f32,
    /// Current narration LUFS (-14 during narration; otherwise -inf
    /// effectively, encoded as 0.0 gain).
    pub narration_lufs: f32,
    /// Current SFX bus LUFS (held at -16 during cinematics).
    pub sfx_lufs: f32,
    /// True when a narration word is currently active.
    pub narration_active: bool,
    /// True when the duck attack is in progress.
    pub ducking: bool,
    /// True when the duck release is in progress.
    pub releasing: bool,
}

impl Default for CinematicMix {
    fn default() -> Self {
        Self {
            active: false,
            music_lufs: CINEMATIC_MUSIC_LUFS_OUTSIDE_NARRATION,
            narration_lufs: 0.0,
            sfx_lufs: CINEMATIC_SFX_LUFS,
            narration_active: false,
            ducking: false,
            releasing: false,
        }
    }
}

/// Cinematic LUFS state machine. Pure compute — caller drives via
/// `tick(dt_ms)` + `set_narration_active(bool)` once per frame.
#[derive(Debug, Clone, PartialEq)]
pub struct CinematicMixer {
    mix: CinematicMix,
    /// Time-elapsed counter (ms) toward the attack target (during
    /// ducking).
    duck_elapsed_ms: u32,
    /// Time-elapsed counter (ms) toward the release target (during
    /// releasing).
    release_elapsed_ms: u32,
    /// Profile-specific overrides for music LUFS during/outside
    /// narration (Cassandra cello bed, Randy percussion, etc.).
    override_music_lufs_during_narration: Option<f32>,
    override_music_lufs_outside_narration: Option<f32>,
}

impl Default for CinematicMixer {
    fn default() -> Self {
        Self::new()
    }
}

impl CinematicMixer {
    /// Construct a fresh mixer in the "outside narration" steady state
    /// (music at -16 LUFS, no ducking).
    #[must_use]
    pub fn new() -> Self {
        Self {
            mix: CinematicMix::default(),
            duck_elapsed_ms: 0,
            release_elapsed_ms: 0,
            override_music_lufs_during_narration: None,
            override_music_lufs_outside_narration: None,
        }
    }

    /// Engage the mixer for a fresh cinematic. Resets every counter.
    pub fn engage(&mut self) {
        self.mix = CinematicMix {
            active: true,
            ..CinematicMix::default()
        };
        self.duck_elapsed_ms = 0;
        self.release_elapsed_ms = 0;
    }

    /// Disengage the mixer at `cinematic.ended`.
    pub fn release(&mut self) {
        self.mix = CinematicMix::default();
        self.duck_elapsed_ms = 0;
        self.release_elapsed_ms = 0;
    }

    /// Override the per-cinematic music LUFS targets (set from the
    /// active storyteller profile per spec § "Phoebe — Warm strings…
    /// at -22 LUFS during narration").
    pub fn set_profile_music_lufs(
        &mut self,
        outside_narration: Option<f32>,
        during_narration: Option<f32>,
    ) {
        self.override_music_lufs_outside_narration = outside_narration;
        self.override_music_lufs_during_narration = during_narration;
    }

    /// Read-only view of the mixer state.
    #[must_use]
    pub fn mix(&self) -> &CinematicMix {
        &self.mix
    }

    /// LUFS the music bed should sit at during narration windows.
    #[must_use]
    pub fn music_lufs_during_narration(&self) -> f32 {
        self.override_music_lufs_during_narration
            .unwrap_or(CINEMATIC_MUSIC_LUFS_DURING_NARRATION)
    }

    /// LUFS the music bed should sit at outside narration windows.
    #[must_use]
    pub fn music_lufs_outside_narration(&self) -> f32 {
        self.override_music_lufs_outside_narration
            .unwrap_or(CINEMATIC_MUSIC_LUFS_OUTSIDE_NARRATION)
    }

    /// Notify the mixer that narration is currently active (`true`) or
    /// not (`false`). Drives the duck attack + release.
    pub fn set_narration_active(&mut self, active: bool) {
        if active == self.mix.narration_active {
            return;
        }
        self.mix.narration_active = active;
        if active {
            self.mix.ducking = true;
            self.mix.releasing = false;
            self.duck_elapsed_ms = 0;
            self.mix.narration_lufs = CINEMATIC_NARRATION_LUFS;
        } else {
            self.mix.releasing = true;
            self.mix.ducking = false;
            self.release_elapsed_ms = 0;
            self.mix.narration_lufs = 0.0;
        }
    }

    /// Advance the mixer by `dt_ms`. Eases the music bed between the
    /// two target LUFS values per the attack / release timings.
    pub fn tick(&mut self, dt_ms: u32) {
        if !self.mix.active {
            return;
        }
        let target_during = self.music_lufs_during_narration();
        let target_outside = self.music_lufs_outside_narration();
        if self.mix.ducking {
            self.duck_elapsed_ms = self
                .duck_elapsed_ms
                .saturating_add(dt_ms)
                .min(CINEMATIC_DUCK_ATTACK_MS);
            let t = self.duck_elapsed_ms as f32 / CINEMATIC_DUCK_ATTACK_MS as f32;
            self.mix.music_lufs = lerp(target_outside, target_during, t);
            if self.duck_elapsed_ms >= CINEMATIC_DUCK_ATTACK_MS {
                self.mix.ducking = false;
                self.mix.music_lufs = target_during;
            }
        } else if self.mix.releasing {
            self.release_elapsed_ms = self
                .release_elapsed_ms
                .saturating_add(dt_ms)
                .min(CINEMATIC_DUCK_RELEASE_MS);
            let t = self.release_elapsed_ms as f32 / CINEMATIC_DUCK_RELEASE_MS as f32;
            self.mix.music_lufs = lerp(target_during, target_outside, t);
            if self.release_elapsed_ms >= CINEMATIC_DUCK_RELEASE_MS {
                self.mix.releasing = false;
                self.mix.music_lufs = target_outside;
            }
        }
        // SFX is fixed at -16 LUFS during cinematics per spec.
        self.mix.sfx_lufs = CINEMATIC_SFX_LUFS;
    }
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_outside_narration_at_minus_16() {
        let m = CinematicMixer::new();
        assert!((m.mix().music_lufs - -16.0).abs() < 1e-3);
        assert!((m.mix().sfx_lufs - -16.0).abs() < 1e-3);
        assert!(!m.mix().narration_active);
    }

    #[test]
    fn engage_resets_state_to_steady_outside_narration() {
        let mut m = CinematicMixer::new();
        m.engage();
        assert!(m.mix().active);
        assert!((m.mix().music_lufs - -16.0).abs() < 1e-3);
    }

    #[test]
    fn ducks_music_to_minus_22_within_200ms() {
        let mut m = CinematicMixer::new();
        m.engage();
        m.set_narration_active(true);
        assert!((m.mix().narration_lufs - -14.0).abs() < 1e-3);
        m.tick(100); // halfway through attack.
        let half = m.mix().music_lufs;
        assert!(half < -16.0 && half > -22.0, "half ducked but not at target: {half}");
        m.tick(100); // complete attack.
        assert!((m.mix().music_lufs - -22.0).abs() < 1e-3);
    }

    #[test]
    fn releases_music_back_to_minus_16_within_400ms() {
        let mut m = CinematicMixer::new();
        m.engage();
        m.set_narration_active(true);
        m.tick(500); // full duck.
        assert!((m.mix().music_lufs - -22.0).abs() < 1e-3);
        m.set_narration_active(false);
        m.tick(200); // halfway through release.
        let half = m.mix().music_lufs;
        assert!(half > -22.0 && half < -16.0, "half released: {half}");
        m.tick(200); // complete release.
        assert!((m.mix().music_lufs - -16.0).abs() < 1e-3);
    }

    #[test]
    fn sfx_holds_at_minus_16_during_cinematic() {
        let mut m = CinematicMixer::new();
        m.engage();
        m.tick(1_000);
        assert!((m.mix().sfx_lufs - -16.0).abs() < 1e-3);
        m.set_narration_active(true);
        m.tick(500);
        assert!((m.mix().sfx_lufs - -16.0).abs() < 1e-3);
    }

    #[test]
    fn profile_overrides_music_lufs() {
        let mut m = CinematicMixer::new();
        m.engage();
        m.set_profile_music_lufs(Some(-18.0), Some(-24.0));
        m.set_narration_active(true);
        m.tick(500);
        assert!((m.mix().music_lufs - -24.0).abs() < 1e-3);
        m.set_narration_active(false);
        m.tick(500);
        assert!((m.mix().music_lufs - -18.0).abs() < 1e-3);
    }

    #[test]
    fn release_disengages_active_flag() {
        let mut m = CinematicMixer::new();
        m.engage();
        assert!(m.mix().active);
        m.release();
        assert!(!m.mix().active);
    }
}
