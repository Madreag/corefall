//! **M12** § CCCP-style intro slideshow renderer.
//!
//! Mirrors Cortex Command's title-screen intro
//! (`Source/Menus/TitleScreen.cpp:256-386`) — 8 painted full-screen
//! slides with subtitle text fades, one looping music track, one optional
//! voice-over narration, total runtime ~60-90 seconds, skippable on any
//! "any-start" press. Reusable for both opening (`SlideshowSlot::IntroCampaign`)
//! and campaign-end at M49 (`SlideshowSlot::EndingCampaign`).
//!
//! This module owns the renderer-agnostic state machine + per-frame
//! advance logic; cf-app + cf-shell drive the actual Bevy entity spawning
//! by reading the [`SlideshowState`] resource each frame.

use bevy::prelude::*;

/// **M12**: which slideshow slot is playing. Distinct slots route different
/// asset sets + different exit destinations.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SlideshowSlot {
    /// First-launch / New Campaign intro (8 slides).
    IntroCampaign,
    /// Main Menu → Story → "Replay Intro" re-watch (same 8 slides).
    ReplayIntro,
    /// Campaign-end bookend at M49 (3-5 slides).
    EndingCampaign,
}

impl SlideshowSlot {
    /// Canonical snake_case identifier (matches the `ux.slideshow_*`
    /// schema enum).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SlideshowSlot::IntroCampaign => "intro_campaign",
            SlideshowSlot::ReplayIntro => "replay_intro",
            SlideshowSlot::EndingCampaign => "ending_campaign",
        }
    }

    /// Parse from the snake_case wire form.
    #[allow(clippy::should_implement_trait)]
    #[must_use]
    pub fn from_str(value: &str) -> Option<SlideshowSlot> {
        Some(match value {
            "intro_campaign" => SlideshowSlot::IntroCampaign,
            "replay_intro" => SlideshowSlot::ReplayIntro,
            "ending_campaign" => SlideshowSlot::EndingCampaign,
            _ => return None,
        })
    }
}

/// **M12**: one slide. Each slide pairs a painted asset (LoadingBg ledger
/// entry) with subtitle text + a duration. Subtitle text fades in over
/// `SUBTITLE_FADE_IN_MS`, holds for the dwell, and fades out over
/// `SUBTITLE_FADE_OUT_MS`. Asset id is the canonical name in the asset
/// ledger so cf-app's `AssetIndex` can look up the PNG path.
#[derive(Debug, Clone, PartialEq)]
pub struct SlideshowSlide {
    /// Canonical asset id (e.g. `intro_slide_1_earth_collapse`). cf-app
    /// looks this up in `AssetIndex` to find the PNG path.
    pub asset_id: String,
    /// Subtitle text (ICU-MessageFormat-localized by cf-localization at
    /// runtime; here we carry the raw English string).
    pub subtitle: String,
    /// Per-slide dwell in ms (subtitle fade-in + hold + fade-out).
    pub duration_ms: u32,
}

impl SlideshowSlide {
    #[must_use]
    pub fn new(asset_id: impl Into<String>, subtitle: impl Into<String>, duration_ms: u32) -> Self {
        Self {
            asset_id: asset_id.into(),
            subtitle: subtitle.into(),
            duration_ms,
        }
    }
}

/// Subtitle fade-in duration (ms). Per CCCP TitleScreen.cpp § subtitle
/// fade.
pub const SUBTITLE_FADE_IN_MS: u32 = 400;

/// Subtitle fade-out duration (ms).
pub const SUBTITLE_FADE_OUT_MS: u32 = 400;

/// Canonical M12 intro narrative arc — 8 slides per spec § Narrative arc.
/// Each entry is `(asset_id, subtitle, duration_ms)`. Total runtime
/// 8 × ~8000 = ~64 seconds, within the 60-90 s spec band.
pub const INTRO_NARRATIVE: &[(&str, &str, u32)] = &[
    (
        "intro_slide_1_earth_collapse",
        "At the end of the 22nd century, Earth's old empires collapsed...",
        8000,
    ),
    (
        "intro_slide_2_brain_transfer",
        "...but the survivors learned to leave their bodies behind.",
        8000,
    ),
    (
        "intro_slide_3_chassis_socket",
        "With brains preserved in steel and silicon...",
        7000,
    ),
    (
        "intro_slide_4_solar_system",
        "...humanity scattered to twelve worlds across the Sol system.",
        9000,
    ),
    (
        "intro_slide_5_factions_orbit",
        "Each world is a frontier. Each frontier breeds factions.",
        8000,
    ),
    (
        "intro_slide_6_named_figures",
        "Coalition, Frontier, Ronin, Synth, Collective, Husks, Collegium, Starlight.",
        9000,
    ),
    (
        "intro_slide_7_simulation_real",
        "The bunkers run deep. The atmospheres are real. The bodies bleed, leak, and burn.",
        9000,
    ),
    (
        "intro_slide_8_dropship_descent",
        "You will now join the frontier. Your command core is waiting.",
        8000,
    ),
];

/// Build the canonical M12 8-slide intro from [`INTRO_NARRATIVE`].
#[must_use]
pub fn intro_slides() -> Vec<SlideshowSlide> {
    INTRO_NARRATIVE
        .iter()
        .map(|(a, s, d)| SlideshowSlide::new(*a, *s, *d))
        .collect()
}

/// Sum of every slide's duration — the total slideshow runtime (ms).
#[must_use]
pub fn slideshow_duration_ms(slides: &[SlideshowSlide]) -> u32 {
    slides.iter().map(|s| s.duration_ms).sum()
}

/// **M12**: phase of the slideshow lifecycle.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum SlideshowPhase {
    /// Idle — no slideshow playing.
    Idle,
    /// A slideshow is actively rendering.
    Playing,
    /// Slideshow ended naturally (reached the final slide).
    Completed,
    /// Player skipped the slideshow (any-start key).
    Skipped,
}

impl Default for SlideshowPhase {
    fn default() -> Self {
        SlideshowPhase::Idle
    }
}

impl SlideshowPhase {
    /// Snake_case identifier (used for the `ux.slideshow_ended.reason`
    /// schema field, for example).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            SlideshowPhase::Idle => "idle",
            SlideshowPhase::Playing => "playing",
            SlideshowPhase::Completed => "completed",
            SlideshowPhase::Skipped => "skipped",
        }
    }
}

/// **M12**: subtitle alpha (`[0.0, 1.0]`) at `elapsed_ms` within a slide
/// whose total dwell is `slide_ms`. Fades in over [`SUBTITLE_FADE_IN_MS`],
/// holds, fades out over [`SUBTITLE_FADE_OUT_MS`]. When `slide_ms` is
/// smaller than the combined fades, the fade durations clamp evenly.
#[must_use]
pub fn subtitle_alpha(elapsed_ms: u32, slide_ms: u32) -> f32 {
    if slide_ms == 0 {
        return 0.0;
    }
    let total = slide_ms as f32;
    let in_ms = (SUBTITLE_FADE_IN_MS as f32).min(total * 0.4);
    let out_ms = (SUBTITLE_FADE_OUT_MS as f32).min(total * 0.4);
    let t = elapsed_ms as f32;
    if t < in_ms && in_ms > 0.0 {
        return (t / in_ms).clamp(0.0, 1.0);
    }
    let out_start = total - out_ms;
    if t > out_start && out_ms > 0.0 {
        return (1.0 - (t - out_start) / out_ms).clamp(0.0, 1.0);
    }
    1.0
}

/// **M12**: live slideshow state. Owned by [`cf_shell`] / cf-app via the
/// `SlideshowState` resource; this struct stores the playing slot, the
/// queued slides, and the current playback cursor. cf-app's bridge ticks
/// the cursor each frame via [`SlideshowState::tick`].
#[derive(Resource, Debug, Clone)]
pub struct SlideshowState {
    /// Which slot is playing. `None` when idle.
    pub slot: Option<SlideshowSlot>,
    /// Phase of the slideshow lifecycle.
    pub phase: SlideshowPhase,
    /// All slides queued for the current playthrough.
    pub slides: Vec<SlideshowSlide>,
    /// Index of the currently-rendered slide (0-based). Invalid when idle.
    pub current_index: usize,
    /// Elapsed milliseconds within the current slide.
    pub current_slide_elapsed_ms: u32,
    /// Total elapsed milliseconds since the slideshow started.
    pub total_elapsed_ms: u32,
    /// Optional canonical music track id (e.g. `music_intro_campaign`).
    pub music_track_id: Option<String>,
    /// Optional canonical voice-over narration id.
    pub voice_track_id: Option<String>,
}

impl Default for SlideshowState {
    fn default() -> Self {
        Self {
            slot: None,
            phase: SlideshowPhase::default(),
            slides: Vec::new(),
            current_index: 0,
            current_slide_elapsed_ms: 0,
            total_elapsed_ms: 0,
            music_track_id: None,
            voice_track_id: None,
        }
    }
}

impl SlideshowState {
    /// True iff the slideshow is currently animating slides.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.phase == SlideshowPhase::Playing
    }

    /// Start playback with the supplied slides + slot + (optional) audio
    /// track ids. Idempotent if a slideshow is already playing — replaces
    /// the previous queue.
    pub fn start(
        &mut self,
        slot: SlideshowSlot,
        slides: Vec<SlideshowSlide>,
        music_track_id: Option<String>,
        voice_track_id: Option<String>,
    ) {
        self.slot = Some(slot);
        self.slides = slides;
        self.current_index = 0;
        self.current_slide_elapsed_ms = 0;
        self.total_elapsed_ms = 0;
        self.music_track_id = music_track_id;
        self.voice_track_id = voice_track_id;
        self.phase = if self.slides.is_empty() {
            SlideshowPhase::Completed
        } else {
            SlideshowPhase::Playing
        };
    }

    /// Skip — sets phase to `Skipped` and jumps the cursor to the end.
    pub fn skip(&mut self) {
        if self.phase != SlideshowPhase::Playing {
            return;
        }
        self.phase = SlideshowPhase::Skipped;
        if !self.slides.is_empty() {
            self.current_index = self.slides.len() - 1;
            self.current_slide_elapsed_ms = self
                .slides
                .last()
                .map(|s| s.duration_ms)
                .unwrap_or(0);
        }
    }

    /// Reset to idle. Called after the renderer consumes the
    /// `Completed`/`Skipped` terminal phase.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advance the playback cursor by `dt_ms`. When the elapsed time
    /// exceeds the current slide's duration, the cursor advances to the
    /// next slide. When the cursor exhausts the queue, phase moves to
    /// `Completed`.
    pub fn tick(&mut self, dt_ms: u32) {
        if !self.is_playing() {
            return;
        }
        self.total_elapsed_ms = self.total_elapsed_ms.saturating_add(dt_ms);
        let mut remaining = dt_ms;
        while remaining > 0 && self.is_playing() {
            let slide_ms = self
                .slides
                .get(self.current_index)
                .map(|s| s.duration_ms)
                .unwrap_or(0);
            let new_elapsed = self.current_slide_elapsed_ms.saturating_add(remaining);
            if new_elapsed < slide_ms {
                self.current_slide_elapsed_ms = new_elapsed;
                return;
            }
            // Slide finished — advance.
            let used = slide_ms.saturating_sub(self.current_slide_elapsed_ms);
            remaining = remaining.saturating_sub(used);
            self.current_index = self.current_index.saturating_add(1);
            self.current_slide_elapsed_ms = 0;
            if self.current_index >= self.slides.len() {
                self.phase = SlideshowPhase::Completed;
                // Pin the cursor to the final slide for the renderer's
                // last-frame readout.
                if !self.slides.is_empty() {
                    self.current_index = self.slides.len() - 1;
                    self.current_slide_elapsed_ms = self
                        .slides
                        .last()
                        .map(|s| s.duration_ms)
                        .unwrap_or(0);
                }
                return;
            }
        }
    }

    /// The slide currently being rendered (`None` when idle).
    #[must_use]
    pub fn current_slide(&self) -> Option<&SlideshowSlide> {
        self.slides.get(self.current_index)
    }

    /// Subtitle alpha for the currently-displayed slide (0..1).
    #[must_use]
    pub fn current_subtitle_alpha(&self) -> f32 {
        match self.current_slide() {
            Some(slide) => subtitle_alpha(self.current_slide_elapsed_ms, slide.duration_ms),
            None => 0.0,
        }
    }
}

/// Bevy plugin wiring [`SlideshowState`].
pub struct SlideshowPlugin;

impl Plugin for SlideshowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SlideshowState>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_round_trips() {
        for slot in [
            SlideshowSlot::IntroCampaign,
            SlideshowSlot::ReplayIntro,
            SlideshowSlot::EndingCampaign,
        ] {
            assert_eq!(SlideshowSlot::from_str(slot.as_str()), Some(slot));
        }
    }

    #[test]
    fn canonical_intro_has_eight_slides() {
        assert_eq!(INTRO_NARRATIVE.len(), 8);
        assert_eq!(intro_slides().len(), 8);
    }

    #[test]
    fn intro_duration_lands_in_60_to_90_second_band() {
        let dur = slideshow_duration_ms(&intro_slides());
        assert!(dur >= 60_000, "got {dur} ms");
        assert!(dur <= 90_000, "got {dur} ms");
    }

    #[test]
    fn subtitle_fades_in_then_out() {
        let dur = 5000;
        let a_start = subtitle_alpha(0, dur);
        let a_mid = subtitle_alpha(2500, dur);
        let a_end = subtitle_alpha(5000, dur);
        assert!(a_start < 0.01, "alpha at start = {a_start}");
        assert!(a_mid > 0.95, "alpha at mid = {a_mid}");
        assert!(a_end < 0.01, "alpha at end = {a_end}");
    }

    #[test]
    fn subtitle_alpha_handles_zero_duration() {
        assert!(subtitle_alpha(0, 0) < 0.001);
    }

    #[test]
    fn start_seeds_state() {
        let mut s = SlideshowState::default();
        s.start(
            SlideshowSlot::IntroCampaign,
            intro_slides(),
            Some("music_intro_campaign".into()),
            None,
        );
        assert!(s.is_playing());
        assert_eq!(s.current_index, 0);
        assert_eq!(s.music_track_id.as_deref(), Some("music_intro_campaign"));
    }

    #[test]
    fn tick_advances_through_slides() {
        let mut s = SlideshowState::default();
        let slides = vec![
            SlideshowSlide::new("a", "first", 1000),
            SlideshowSlide::new("b", "second", 1000),
        ];
        s.start(SlideshowSlot::IntroCampaign, slides, None, None);
        s.tick(500);
        assert_eq!(s.current_index, 0);
        s.tick(700);
        assert_eq!(s.current_index, 1);
    }

    #[test]
    fn tick_completes_after_final_slide() {
        let mut s = SlideshowState::default();
        let slides = vec![SlideshowSlide::new("a", "only", 1000)];
        s.start(SlideshowSlot::IntroCampaign, slides, None, None);
        s.tick(2000);
        assert_eq!(s.phase, SlideshowPhase::Completed);
    }

    #[test]
    fn skip_jumps_to_skipped_phase() {
        let mut s = SlideshowState::default();
        s.start(SlideshowSlot::IntroCampaign, intro_slides(), None, None);
        s.skip();
        assert_eq!(s.phase, SlideshowPhase::Skipped);
        // Cursor pinned to final slide.
        assert_eq!(s.current_index, intro_slides().len() - 1);
    }

    #[test]
    fn skip_idle_is_a_noop() {
        let mut s = SlideshowState::default();
        s.skip();
        assert_eq!(s.phase, SlideshowPhase::Idle);
    }

    #[test]
    fn reset_returns_to_idle() {
        let mut s = SlideshowState::default();
        s.start(SlideshowSlot::IntroCampaign, intro_slides(), None, None);
        s.reset();
        assert_eq!(s.phase, SlideshowPhase::Idle);
        assert!(s.slot.is_none());
        assert!(s.slides.is_empty());
    }

    #[test]
    fn empty_slide_queue_completes_immediately() {
        let mut s = SlideshowState::default();
        s.start(SlideshowSlot::IntroCampaign, vec![], None, None);
        assert_eq!(s.phase, SlideshowPhase::Completed);
    }

    #[test]
    fn current_subtitle_alpha_is_zero_when_idle() {
        let s = SlideshowState::default();
        assert!(s.current_subtitle_alpha() < 0.001);
    }
}
