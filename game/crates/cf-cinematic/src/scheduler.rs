//! **M12C**: Cinematic kernel — shot scheduler + chapter-marker emitter
//! + skip-confirm window enforcement.
//!
//! Per spec § "Crates / modules touched" → cf-cinematic::scheduler is
//! the per-tick driver that:
//!
//! 1. Owns the per-cinematic playhead clock (`playhead_ms`).
//! 2. Fires `cinematic.started` on the first tick of playback.
//! 3. Fires `cinematic.chapter_marker` when the playhead crosses each
//!    author-defined chapter `at_ms`.
//! 4. Fires `cinematic.narration_word` per ElevenLabs word-timestamp
//!    crossing (only when captions on, per spec § "cinematic
//!    .narration_word events still emit (replay parity) / the caption
//!    ribbon is hidden").
//! 5. Fires `cinematic.paused` / `cinematic.resumed` on pause toggle.
//! 6. Fires `cinematic.skipped` on player skip + writes to seen_set.
//! 7. Fires `cinematic.ended` on last tick (or on skip).
//!
//! Determinism: the kernel is a pure tick-driven state machine; the
//! shake noise seed flows through `cf-cinematic::camera_moves` via the
//! M4 replay seed XOR shot index. No `thread_rng` is ever invoked.

use serde::{Deserialize, Serialize};

use crate::camera_moves::{apply_move_stack, ComposedOffset};
use crate::narration_sync::{word_at_ms, NarrationTrack, WordHighlightState};
use crate::script::{ChapterMarker, CinematicId, CinematicScript, ScriptSource};
use crate::skip_pause_replay::{SeenSet, SkipPauseReplayPolicy, SkipReason};
use crate::storyteller_profile::StorytellerProfile;

/// Source classification mirror (matches `ScriptSource` but kept
/// separate so consumers can opt into the kernel without pulling in
/// the script loader).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CinematicSource {
    /// Mission opening.
    Opening,
    /// Between-mission base monologue.
    Between,
    /// Campaign-ending cinematic.
    Ending,
}

impl From<ScriptSource> for CinematicSource {
    fn from(s: ScriptSource) -> Self {
        match s {
            ScriptSource::Opening => CinematicSource::Opening,
            ScriptSource::Between => CinematicSource::Between,
            ScriptSource::Ending => CinematicSource::Ending,
        }
    }
}

impl CinematicSource {
    /// Canonical snake_case identifier matching the event schema enum.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CinematicSource::Opening => "opening",
            CinematicSource::Between => "between",
            CinematicSource::Ending => "ending",
        }
    }
}

/// Playback-phase discriminator. The scheduler is a state machine
/// driven by per-tick advance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackPhase {
    /// Pre-launch: the kernel has the script loaded but
    /// `cinematic.started` has not yet fired.
    PendingStart,
    /// Actively advancing the playhead.
    Playing,
    /// Paused via `[P]`.
    Paused,
    /// Reached the last tick OR was skipped; `cinematic.ended` already
    /// fired.
    Ended,
}

/// One side-effect event emitted by the kernel during a per-tick
/// advance. The consumer fans these out to the replay recorder + audio
/// mixer + caption ribbon + briefing card.
#[derive(Debug, Clone, PartialEq)]
pub enum CinematicEvent {
    /// `cinematic.started { id, source, replay: bool }`.
    Started {
        /// Cinematic id.
        id: CinematicId,
        /// Source classification.
        source: CinematicSource,
        /// True when this is a codex replay (no save mutation).
        replay: bool,
    },
    /// `cinematic.chapter_marker { id, chapter_id, ms }`.
    Chapter {
        /// Cinematic id.
        id: CinematicId,
        /// Author-defined chapter id.
        chapter_id: String,
        /// ms from cinematic start.
        ms: u32,
    },
    /// `cinematic.narration_word { id, word_index, text }`. Always
    /// emitted for replay parity; the caption ribbon is hidden when
    /// captions are off.
    NarrationWord {
        /// Cinematic id.
        id: CinematicId,
        /// 0-based index into the narration track word list.
        word_index: u32,
        /// Word text.
        text: String,
        /// ms from cinematic start when the word began.
        ms: u32,
    },
    /// `cinematic.paused { id, ms }`.
    Paused {
        /// Cinematic id.
        id: CinematicId,
        /// Playhead at pause.
        ms: u32,
    },
    /// `cinematic.resumed { id, ms }`.
    Resumed {
        /// Cinematic id.
        id: CinematicId,
        /// Playhead at resume.
        ms: u32,
    },
    /// `cinematic.skipped { id, skipped_at_ms, reason }`.
    Skipped {
        /// Cinematic id.
        id: CinematicId,
        /// Playhead at skip.
        skipped_at_ms: u32,
        /// Reason (user input / sandbox suppressed / completed).
        reason: SkipReason,
    },
    /// `cinematic.ended { id, duration_ms, was_skipped }`.
    Ended {
        /// Cinematic id.
        id: CinematicId,
        /// Total duration up to ended marker.
        duration_ms: u32,
        /// True when the player skipped before completion.
        was_skipped: bool,
    },
}

impl CinematicEvent {
    /// Replay event_type kind discriminator (snake_case identifier
    /// matching the JSON schema enum).
    #[must_use]
    pub fn kind(&self) -> CinematicEventKind {
        match self {
            CinematicEvent::Started { .. } => CinematicEventKind::Started,
            CinematicEvent::Chapter { .. } => CinematicEventKind::ChapterMarker,
            CinematicEvent::NarrationWord { .. } => CinematicEventKind::NarrationWord,
            CinematicEvent::Paused { .. } => CinematicEventKind::Paused,
            CinematicEvent::Resumed { .. } => CinematicEventKind::Resumed,
            CinematicEvent::Skipped { .. } => CinematicEventKind::Skipped,
            CinematicEvent::Ended { .. } => CinematicEventKind::Ended,
        }
    }
}

/// Event-type discriminator matching the replay schema enum names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CinematicEventKind {
    /// `cinematic.started`.
    Started,
    /// `cinematic.chapter_marker`.
    ChapterMarker,
    /// `cinematic.narration_word`.
    NarrationWord,
    /// `cinematic.paused`.
    Paused,
    /// `cinematic.resumed`.
    Resumed,
    /// `cinematic.skipped`.
    Skipped,
    /// `cinematic.ended`.
    Ended,
}

impl CinematicEventKind {
    /// Canonical snake_case identifier matching the replay JSON schema
    /// event_type field.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CinematicEventKind::Started => "started",
            CinematicEventKind::ChapterMarker => "chapter_marker",
            CinematicEventKind::NarrationWord => "narration_word",
            CinematicEventKind::Paused => "paused",
            CinematicEventKind::Resumed => "resumed",
            CinematicEventKind::Skipped => "skipped",
            CinematicEventKind::Ended => "ended",
        }
    }
}

/// Snapshot of the cinematic state visible to `srv.dump_cinematic_state`
/// + the renderer's camera-takeover bridge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CinematicState {
    /// Schema version for the snapshot blob.
    pub schema_version: u32,
    /// Id of the cinematic currently playing (or last played).
    pub cinematic_id: Option<CinematicId>,
    /// Source classification.
    pub source: Option<CinematicSource>,
    /// Active phase.
    pub phase: PlaybackPhase,
    /// Playhead position (ms).
    pub playhead_ms: u32,
    /// Total duration (ms).
    pub duration_ms: u32,
    /// True = replay playback (no save mutation).
    pub replay: bool,
    /// True = paused.
    pub paused: bool,
    /// True = the active storyteller is Sandbox (cinematic suppressed
    /// for replay parity).
    pub sandbox_suppressed: bool,
    /// Active word index in the narration track (if any).
    pub active_word_index: Option<u32>,
    /// Briefing card lines surfaced (after `briefing_at_ms`). Empty
    /// before that boundary or when no card is authored.
    pub briefing_card_lines: Vec<String>,
    /// Composed camera offset for the current tick.
    pub camera_translation: [f32; 2],
    /// Composed shake offset (px).
    pub camera_shake_px: [f32; 2],
    /// Composed orthographic half-height (0 = no zoom override).
    pub camera_ortho_half_height: f32,
}

impl Default for CinematicState {
    fn default() -> Self {
        Self {
            schema_version: 1,
            cinematic_id: None,
            source: None,
            phase: PlaybackPhase::Ended,
            playhead_ms: 0,
            duration_ms: 0,
            replay: false,
            paused: false,
            sandbox_suppressed: false,
            active_word_index: None,
            briefing_card_lines: Vec::new(),
            camera_translation: [0.0, 0.0],
            camera_shake_px: [0.0, 0.0],
            camera_ortho_half_height: 0.0,
        }
    }
}

/// Cinematic playback kernel — one instance per playing cinematic.
/// The owner (typically cf-control's engine) calls `advance(dt_ms)`
/// each tick and fans the emitted `CinematicEvent`s into the
/// recorder + UI + audio mixer.
#[derive(Debug, Clone)]
pub struct CinematicKernel {
    script: CinematicScript,
    narration: NarrationTrack,
    profile: StorytellerProfile,
    state: CinematicState,
    /// Replay seed XOR'd with shot index when computing shake noise.
    seed: u64,
    /// True after the first `advance` call (used to fire `Started`).
    started_emitted: bool,
    /// True after `Ended` fired.
    ended_emitted: bool,
    /// Per-frame highlight cursor; tracks the last emitted
    /// `NarrationWord` to avoid double-firing.
    last_emitted_word_index: Option<u32>,
    /// Player skip/pause policy.
    policy: SkipPauseReplayPolicy,
    /// True = replay playback from codex (no save mutation).
    replay: bool,
}

impl CinematicKernel {
    /// Construct a fresh kernel for the supplied script + storyteller
    /// profile + narration track. The seen-set + paused state come from
    /// the player save (loaded by the caller).
    ///
    /// When the storyteller profile flags `suppress_cinematics=true`
    /// (Sandbox), the constructor still returns a valid kernel; the
    /// caller calls [`Self::suppress_for_sandbox`] to emit the
    /// `cinematic.skipped { reason: sandbox_suppressed }` + `Ended`
    /// events without advancing the playhead.
    #[must_use]
    pub fn new(
        script: CinematicScript,
        profile: StorytellerProfile,
        narration: NarrationTrack,
        seed: u64,
        seen_set: SeenSet,
        replay: bool,
    ) -> Self {
        let duration_ms = script.total_duration_ms();
        let state = CinematicState {
            schema_version: 1,
            cinematic_id: Some(script.id.clone()),
            source: Some(script.source.into()),
            phase: PlaybackPhase::PendingStart,
            playhead_ms: 0,
            duration_ms,
            replay,
            paused: false,
            sandbox_suppressed: profile.suppress_cinematics,
            active_word_index: None,
            briefing_card_lines: Vec::new(),
            camera_translation: [0.0, 0.0],
            camera_shake_px: [0.0, 0.0],
            camera_ortho_half_height: 0.0,
        };
        Self {
            script,
            narration,
            profile,
            state,
            seed,
            started_emitted: false,
            ended_emitted: false,
            last_emitted_word_index: None,
            policy: SkipPauseReplayPolicy {
                seen: seen_set,
                paused: false,
            },
            replay,
        }
    }

    /// Read-only view of the kernel state (for `srv.dump_cinematic_state`).
    #[must_use]
    pub fn state(&self) -> &CinematicState {
        &self.state
    }

    /// Read-only view of the active storyteller profile.
    #[must_use]
    pub fn profile(&self) -> &StorytellerProfile {
        &self.profile
    }

    /// Read-only view of the active script.
    #[must_use]
    pub fn script(&self) -> &CinematicScript {
        &self.script
    }

    /// Read-only seen-set (caller updates per `Ended` event).
    #[must_use]
    pub fn seen(&self) -> &SeenSet {
        &self.policy.seen
    }

    /// Per spec § "the cinematic is suppressed entirely / the player is
    /// dropped directly into gameplay UI / `cinematic.skipped` is
    /// emitted for replay parity".
    ///
    /// Caller invokes this immediately after construction when
    /// `profile.suppress_cinematics == true` (Sandbox). Emits a
    /// `Skipped { reason: SandboxSuppressed, skipped_at_ms: 0 }` +
    /// `Ended { was_skipped: true }` event pair. The Started event is
    /// NOT emitted because the cinematic never actually played.
    pub fn suppress_for_sandbox(&mut self) -> Vec<CinematicEvent> {
        if !self.profile.suppress_cinematics {
            return Vec::new();
        }
        let id = self.script.id.clone();
        self.state.phase = PlaybackPhase::Ended;
        self.ended_emitted = true;
        vec![
            CinematicEvent::Skipped {
                id: id.clone(),
                skipped_at_ms: 0,
                reason: SkipReason::SandboxSuppressed,
            },
            CinematicEvent::Ended {
                id,
                duration_ms: 0,
                was_skipped: true,
            },
        ]
    }

    /// Request a pause. Idempotent — no event emitted when already
    /// paused / ended / pending. Returns the `Paused` event when the
    /// transition actually fires.
    pub fn request_pause(&mut self) -> Option<CinematicEvent> {
        if self.state.phase != PlaybackPhase::Playing {
            return None;
        }
        self.state.phase = PlaybackPhase::Paused;
        self.state.paused = true;
        self.policy.paused = true;
        Some(CinematicEvent::Paused {
            id: self.script.id.clone(),
            ms: self.state.playhead_ms,
        })
    }

    /// Request a resume. Idempotent — no event emitted unless we were
    /// paused. Returns the `Resumed` event when the transition fires.
    pub fn request_resume(&mut self) -> Option<CinematicEvent> {
        if self.state.phase != PlaybackPhase::Paused {
            return None;
        }
        self.state.phase = PlaybackPhase::Playing;
        self.state.paused = false;
        self.policy.paused = false;
        Some(CinematicEvent::Resumed {
            id: self.script.id.clone(),
            ms: self.state.playhead_ms,
        })
    }

    /// Request a skip. Rejected when `policy.skip_allowed` returns
    /// false (spec § "Skip is disabled for the first 3 seconds on
    /// never-before-seen cinematics"). Returns `Some((skipped,
    /// ended))` when the skip fires.
    pub fn request_skip(&mut self) -> Option<(CinematicEvent, CinematicEvent)> {
        if self.ended_emitted {
            return None;
        }
        if !self.policy.skip_allowed(&self.script.id, self.state.playhead_ms) {
            return None;
        }
        let id = self.script.id.clone();
        let ms = self.state.playhead_ms;
        let duration = self.state.duration_ms;
        self.state.phase = PlaybackPhase::Ended;
        self.ended_emitted = true;
        // Skip past the confirm window marks the cinematic as seen
        // per spec § "save.cinematic_seen_set is updated to include
        // the id" (and the cinematic is unlockable in codex).
        if !self.replay {
            self.policy.seen.mark_seen(&id);
        }
        Some((
            CinematicEvent::Skipped {
                id: id.clone(),
                skipped_at_ms: ms,
                reason: SkipReason::UserInput,
            },
            CinematicEvent::Ended {
                id,
                duration_ms: duration,
                was_skipped: true,
            },
        ))
    }

    /// Advance the playhead by `dt_ms`. Returns every event that fired
    /// during the advance — `Started` (first tick), `Chapter` (per
    /// crossing), `NarrationWord` (per crossing), `Ended` (last tick).
    pub fn advance(&mut self, dt_ms: u32) -> Vec<CinematicEvent> {
        let mut out = Vec::new();
        if self.ended_emitted {
            return out;
        }
        if self.state.phase == PlaybackPhase::Paused {
            return out;
        }
        let prev_ms = self.state.playhead_ms;
        // First-tick `Started`.
        if !self.started_emitted {
            self.started_emitted = true;
            self.state.phase = PlaybackPhase::Playing;
            out.push(CinematicEvent::Started {
                id: self.script.id.clone(),
                source: self.script.source.into(),
                replay: self.replay,
            });
        }
        // Compute the new playhead.
        let mut new_ms = prev_ms.saturating_add(dt_ms);
        if new_ms > self.state.duration_ms {
            new_ms = self.state.duration_ms;
        }
        self.state.playhead_ms = new_ms;
        // Chapter markers in `[prev_ms, new_ms)` — half-open lower-
        // inclusive bound; the spec acceptance criterion requires
        // "when the playhead crosses t=8000 ms" → we emit when the
        // tick range crosses 8000.
        let lower = if self.started_emitted && prev_ms == 0 && new_ms > 0 {
            0
        } else {
            prev_ms.saturating_add(1)
        };
        for ch in self
            .script
            .chapters
            .iter()
            .filter(|c| c.at_ms >= lower && c.at_ms <= new_ms)
        {
            out.push(CinematicEvent::Chapter {
                id: self.script.id.clone(),
                chapter_id: ch.id.clone(),
                ms: ch.at_ms,
            });
        }
        // Narration word crossings. Walk the full word list; emit each
        // word whose start_ms falls in `(prev_ms, new_ms]` AND that
        // wasn't already emitted.
        for (i, w) in self.narration.words.iter().enumerate() {
            if w.start_ms > prev_ms && w.start_ms <= new_ms {
                let idx_u32 = i as u32;
                if self.last_emitted_word_index != Some(idx_u32) {
                    out.push(CinematicEvent::NarrationWord {
                        id: self.script.id.clone(),
                        word_index: idx_u32,
                        text: w.word.clone(),
                        ms: w.start_ms,
                    });
                    self.last_emitted_word_index = Some(idx_u32);
                }
            }
        }
        // Update active word for state snapshot.
        self.state.active_word_index = match word_at_ms(&self.narration, new_ms) {
            WordHighlightState::Highlighted(i) => Some(i as u32),
            WordHighlightState::Idle => None,
        };
        // Briefing card.
        self.state.briefing_card_lines = if new_ms >= self.script.briefing_at_ms {
            self.script.briefing_card_lines.clone()
        } else {
            Vec::new()
        };
        // Apply camera move composition.
        if let Some((shot_index, t_in_shot)) = self.script.shot_at_ms(new_ms) {
            let shot = &self.script.shots[shot_index.0 as usize];
            let composed = apply_move_stack(
                &shot.moves,
                t_in_shot,
                self.seed ^ (shot_index.0 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            );
            self.state.camera_translation = composed.translation;
            self.state.camera_shake_px = composed.shake;
            self.state.camera_ortho_half_height = composed.ortho_half_height;
        }
        // End-of-script.
        if new_ms >= self.state.duration_ms && !self.ended_emitted {
            self.ended_emitted = true;
            self.state.phase = PlaybackPhase::Ended;
            if !self.replay {
                self.policy.seen.mark_seen(&self.script.id);
            }
            out.push(CinematicEvent::Ended {
                id: self.script.id.clone(),
                duration_ms: self.state.duration_ms,
                was_skipped: false,
            });
        }
        out
    }

    /// Composed camera offset for the current frame (read by the
    /// renderer-side camera-takeover stack).
    #[must_use]
    pub fn camera_offset(&self) -> ComposedOffset {
        ComposedOffset {
            translation: self.state.camera_translation,
            ortho_half_height: self.state.camera_ortho_half_height,
            shake: self.state.camera_shake_px,
            shake_clamped: false,
        }
    }

    /// Per spec § Notes: "The cinematic camera REPLACES the gameplay
    /// camera at the render layer". `true` when the renderer should
    /// pick up the cinematic camera transform.
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self.state.phase,
            PlaybackPhase::PendingStart | PlaybackPhase::Playing | PlaybackPhase::Paused,
        )
    }

    /// True when player gameplay input should be blocked (except skip /
    /// pause). Per spec § "Player gameplay input is blocked except
    /// `act.player.skip_cinematic` and `act.player.pause_cinematic`".
    #[must_use]
    pub fn blocks_gameplay_input(&self) -> bool {
        self.is_active()
    }

    /// Forward the chapter list (for codex preview).
    #[must_use]
    pub fn chapters(&self) -> &[ChapterMarker] {
        &self.script.chapters
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_moves::{EaseKind, MoveKind, ShakeParams, ShotMove};
    use crate::narration_sync::NarrationWord;
    use crate::script::{ChapterMarker, Shot};
    use crate::storyteller_profile::{builtin_profile, StorytellerId, COLOR_GRADE_NEUTRAL};

    fn opening_script(id: &str, duration_ms: u32, chapters: Vec<ChapterMarker>) -> CinematicScript {
        CinematicScript {
            schema_version: crate::CINEMATIC_SCHEMA_VERSION,
            id: id.to_string(),
            source: ScriptSource::Opening,
            storyteller: None,
            shots: vec![Shot {
                label: "main".to_string(),
                duration_ms,
                moves: vec![ShotMove {
                    kind: MoveKind::Pan,
                    start_ms: 0,
                    duration_ms,
                    easing: EaseKind::EaseInOutCubic,
                    pan: [5.0, 0.0],
                    ..ShotMove::default()
                }],
            }],
            chapters,
            narration_track_id: None,
            briefing_card_lines: vec!["briefing".to_string()],
            briefing_at_ms: 15_000,
            source_sha256: None,
        }
    }

    #[test]
    fn first_advance_emits_started_only() {
        let script = opening_script("cin_test", 30_000, vec![]);
        let profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        let track = NarrationTrack::default();
        let mut k = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        let events = k.advance(16);
        assert!(matches!(events.first(), Some(CinematicEvent::Started { .. })));
        assert_eq!(k.state().phase, PlaybackPhase::Playing);
        assert_eq!(k.state().playhead_ms, 16);
    }

    #[test]
    fn chapter_marker_fires_on_crossing() {
        let chapters = vec![
            ChapterMarker {
                id: "dropship_door_opens".to_string(),
                at_ms: 8_000,
            },
            ChapterMarker {
                id: "boss_reveal".to_string(),
                at_ms: 22_000,
            },
        ];
        let script = opening_script("cin_chapters", 30_000, chapters);
        let profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        let track = NarrationTrack::default();
        let mut k = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        // Step to 7900 — no chapter yet.
        let _ = k.advance(7_900);
        let mid = k.advance(200); // up to 8100 → crosses 8000.
        let chapter_evts: Vec<_> = mid
            .iter()
            .filter(|e| matches!(e, CinematicEvent::Chapter { .. }))
            .collect();
        assert_eq!(chapter_evts.len(), 1);
        if let CinematicEvent::Chapter { chapter_id, ms, .. } = &chapter_evts[0] {
            assert_eq!(chapter_id, "dropship_door_opens");
            assert_eq!(*ms, 8_000);
        }
        // Step to 21999 → no chapter.
        let _ = k.advance(13_899);
        let cross_boss = k.advance(200); // up to 22099 → crosses 22000.
        let boss: Vec<_> = cross_boss
            .iter()
            .filter(|e| matches!(e, CinematicEvent::Chapter { .. }))
            .collect();
        assert_eq!(boss.len(), 1);
    }

    #[test]
    fn skip_blocked_in_confirm_window_for_unseen() {
        let script = opening_script("cin_skip", 30_000, vec![]);
        let profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        let track = NarrationTrack::default();
        let mut k = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        let _ = k.advance(1_500);
        assert!(k.request_skip().is_none(), "skip rejected before 3s");
        let _ = k.advance(2_000); // playhead = 3500.
        let r = k.request_skip();
        assert!(r.is_some(), "skip accepted at 3500");
        let (sk, end) = r.unwrap();
        if let CinematicEvent::Skipped { skipped_at_ms, .. } = sk {
            assert_eq!(skipped_at_ms, 3_500);
        }
        if let CinematicEvent::Ended { was_skipped, .. } = end {
            assert!(was_skipped);
        }
        // After skip, seen set is updated.
        assert!(k.seen().contains("cin_skip"));
    }

    #[test]
    fn pause_resume_round_trip_emits_events() {
        let script = opening_script("cin_pause", 30_000, vec![]);
        let profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        let track = NarrationTrack::default();
        let mut k = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        let _ = k.advance(4_200);
        let p = k.request_pause();
        assert!(matches!(p, Some(CinematicEvent::Paused { ms: 4_200, .. })));
        // Advance while paused — playhead does not move.
        let no_advance = k.advance(1_000);
        assert!(no_advance.is_empty());
        assert_eq!(k.state().playhead_ms, 4_200);
        let r = k.request_resume();
        assert!(matches!(r, Some(CinematicEvent::Resumed { ms: 4_200, .. })));
        // Subsequent advance proceeds.
        let _ = k.advance(100);
        assert_eq!(k.state().playhead_ms, 4_300);
    }

    #[test]
    fn ended_fires_at_total_duration() {
        let script = opening_script("cin_done", 30_000, vec![]);
        let profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        let track = NarrationTrack::default();
        let mut k = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        // Advance to just under total.
        let _ = k.advance(29_999);
        let last = k.advance(2); // overshoot.
        let ended: Vec<_> = last
            .iter()
            .filter(|e| matches!(e, CinematicEvent::Ended { .. }))
            .collect();
        assert_eq!(ended.len(), 1);
        if let CinematicEvent::Ended {
            duration_ms,
            was_skipped,
            ..
        } = ended[0]
        {
            assert_eq!(*duration_ms, 30_000);
            assert!(!*was_skipped);
        }
        assert!(k.seen().contains("cin_done"));
    }

    #[test]
    fn sandbox_suppression_emits_skip_and_end_without_playback() {
        let script = opening_script("cin_sandbox", 30_000, vec![]);
        let profile = builtin_profile(StorytellerId::Sandbox).clone();
        let track = NarrationTrack::default();
        let mut k = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        let evs = k.suppress_for_sandbox();
        assert_eq!(evs.len(), 2);
        assert!(matches!(
            evs[0],
            CinematicEvent::Skipped {
                reason: SkipReason::SandboxSuppressed,
                skipped_at_ms: 0,
                ..
            }
        ));
        assert!(matches!(evs[1], CinematicEvent::Ended { was_skipped: true, .. }));
        // Subsequent advance produces nothing.
        let nop = k.advance(1_000);
        assert!(nop.is_empty());
    }

    #[test]
    fn narration_word_events_fire_in_order() {
        let script = opening_script("cin_narrate", 30_000, vec![]);
        let mut profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        profile.color_grade = COLOR_GRADE_NEUTRAL;
        let track = NarrationTrack {
            words: vec![
                NarrationWord {
                    word: "the".to_string(),
                    start_ms: 100,
                    end_ms: 400,
                },
                NarrationWord {
                    word: "dropship".to_string(),
                    start_ms: 2_100,
                    end_ms: 2_700,
                },
            ],
        };
        let mut k = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        let one = k.advance(500); // covers word 0.
        let words: Vec<_> = one
            .iter()
            .filter_map(|e| {
                if let CinematicEvent::NarrationWord { text, .. } = e {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(words, vec!["the"]);
        let _ = k.advance(1_500);
        let two = k.advance(200); // crosses word 1 at 2_100.
        let words: Vec<_> = two
            .iter()
            .filter_map(|e| {
                if let CinematicEvent::NarrationWord { text, .. } = e {
                    Some(text.as_str())
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(words, vec!["dropship"]);
    }

    #[test]
    fn replay_does_not_mutate_seen_set() {
        let script = opening_script("cin_replay", 30_000, vec![]);
        let profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        let track = NarrationTrack::default();
        let seen = SeenSet::default();
        let mut k = CinematicKernel::new(script, profile, track, 42, seen, true);
        // Advance to end.
        let _ = k.advance(30_000);
        assert!(!k.seen().contains("cin_replay"));
    }

    #[test]
    fn deterministic_camera_offset_across_runs() {
        let script = CinematicScript {
            schema_version: crate::CINEMATIC_SCHEMA_VERSION,
            id: "cin_det".to_string(),
            source: ScriptSource::Opening,
            storyteller: None,
            shots: vec![Shot {
                label: "shake".to_string(),
                duration_ms: 30_000,
                moves: vec![ShotMove {
                    kind: MoveKind::Shake,
                    start_ms: 0,
                    duration_ms: 1_000,
                    shake: ShakeParams {
                        amplitude_px: 8.0,
                        frequency_hz: 30.0,
                        decay_s: 0.5,
                    },
                    ..ShotMove::default()
                }],
            }],
            chapters: vec![],
            narration_track_id: None,
            briefing_card_lines: vec![],
            briefing_at_ms: 15_000,
            source_sha256: None,
        };
        let profile = builtin_profile(StorytellerId::CassandraClassic).clone();
        let track = NarrationTrack::default();
        let mut a = CinematicKernel::new(script.clone(), profile.clone(), track.clone(), 42, SeenSet::default(), false);
        let mut b = CinematicKernel::new(script, profile, track, 42, SeenSet::default(), false);
        for _ in 0..30 {
            let _ = a.advance(33);
            let _ = b.advance(33);
        }
        assert_eq!(a.state().camera_shake_px, b.state().camera_shake_px);
        assert_eq!(a.state().camera_translation, b.state().camera_translation);
    }
}
