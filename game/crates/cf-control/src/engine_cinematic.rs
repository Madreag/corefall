//! Cinematic kernel methods on M0Engine.
//!
//! Extracted from engine.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use cf_actor::sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport};
use cf_actor::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig,
    ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::engine::*;
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {
    /// translate it into the canonical replay event surface.
    pub(crate) fn emit_cinematic_event(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        event: &cf_cinematic::CinematicEvent,
        parent: Option<&str>,
    ) {
        let (event_type, payload) = match event {
            cf_cinematic::CinematicEvent::Started { id, source, replay } => (
                "started",
                json!({
                    "id": id,
                    "source": source.as_str(),
                    "replay": *replay,
                }),
            ),
            cf_cinematic::CinematicEvent::Chapter { id, chapter_id, ms } => (
                "chapter_marker",
                json!({
                    "id": id,
                    "chapter_id": chapter_id,
                    "ms": *ms,
                }),
            ),
            cf_cinematic::CinematicEvent::NarrationWord {
                id,
                word_index,
                text,
                ms,
            } => (
                "narration_word",
                json!({
                    "id": id,
                    "word_index": *word_index,
                    "text": text,
                    "ms": *ms,
                }),
            ),
            cf_cinematic::CinematicEvent::Paused { id, ms } => ("paused", json!({"id": id, "ms": *ms})),
            cf_cinematic::CinematicEvent::Resumed { id, ms } => ("resumed", json!({"id": id, "ms": *ms})),
            cf_cinematic::CinematicEvent::Skipped {
                id,
                skipped_at_ms,
                reason,
            } => (
                "skipped",
                json!({
                    "id": id,
                    "skipped_at_ms": *skipped_at_ms,
                    "reason": reason.as_str(),
                }),
            ),
            cf_cinematic::CinematicEvent::Ended {
                id,
                duration_ms,
                was_skipped,
            } => (
                "ended",
                json!({
                    "id": id,
                    "duration_ms": *duration_ms,
                    "was_skipped": *was_skipped,
                }),
            ),
        };
        self.recorder.record(
            tick,
            sim_time_ms,
            "cinematic",
            event_type,
            payload,
            parent.map(|s| s.to_string()),
        );
    }

    /// from the per-storyteller stinger table. Pure function of
    /// `(cinematic_id, seed, table)` so replay parity holds.
    /// Returns `None` when the table parse fails or has zero variants.
    pub fn select_opening_stinger(
        &self,
        cinematic_id: &str,
        stinger_table_bytes: &[u8],
    ) -> Option<cf_cinematic::StingerVariant> {
        let table = cf_cinematic::StingerTable::from_ron(stinger_table_bytes).ok()?;
        table.pick(cinematic_id, self.config.seed).cloned()
    }

    /// variant pre-injected into the briefing card. The stinger's
    /// `line_a` / `line_b` are appended to `script.briefing_card_lines`
    /// before kernel construction so the stinger surfaces on the
    /// mission-briefing fade card alongside the authored briefing.
    pub fn engage_cinematic_kernel_with_stinger(
        &self,
        id: &str,
        source: cf_cinematic::ScriptSource,
        storyteller: cf_cinematic::StorytellerId,
        mut script: cf_cinematic::CinematicScript,
        narration: cf_cinematic::NarrationTrack,
        stinger: Option<cf_cinematic::StingerVariant>,
        replay: bool,
    ) {
        if let Some(s) = stinger {
            if !s.line_a.is_empty() {
                script.briefing_card_lines.push(s.line_a.clone());
            }
            if !s.line_b.is_empty() {
                script.briefing_card_lines.push(s.line_b.clone());
            }
            // Cap at 8 lines (BriefingCardState::BRIEFING_MAX_LINES).
            if script.briefing_card_lines.len() > 8 {
                script.briefing_card_lines.truncate(8);
            }
        }
        self.engage_cinematic_kernel(id, source, storyteller, script, narration, replay);
    }

    /// at codex replay. Fires the initial `cinematic.started` event +
    /// `cinematic.skipped { reason: sandbox_suppressed }` +
    /// `cinematic.ended` pair when the storyteller is Sandbox.
    pub fn engage_cinematic_kernel(
        &self,
        _id: &str,
        _source: cf_cinematic::ScriptSource,
        storyteller: cf_cinematic::StorytellerId,
        script: cf_cinematic::CinematicScript,
        narration: cf_cinematic::NarrationTrack,
        replay: bool,
    ) {
        let profile = cf_cinematic::builtin_profile(storyteller);
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let seed = self.config.seed;
        let seen = state.cinematic_seen_set.clone();
        let mut kernel = cf_cinematic::CinematicKernel::new(script, profile.clone(), narration, seed, seen, replay);
        let mut events: Vec<cf_cinematic::CinematicEvent> = Vec::new();
        if kernel.profile().suppress_cinematics {
            events.extend(kernel.suppress_for_sandbox());
        }
        let parent = state.run_started_event_id.clone();
        let sandbox_suppressed = kernel.profile().suppress_cinematics;
        state.cinematic_seen_set = kernel.seen().clone();
        // LUFS overrides (Cassandra cello @ -22 LUFS during narration,
        // Randy percussion +20% contrast, etc.) per spec acceptance
        // criterion "Per-storyteller cinematic profile biases camera +
        // audio + color". Sandbox path skips engage so the steady-state
        // mixer (music @ -16 LUFS) is preserved.
        if !sandbox_suppressed {
            state.cinematic_mixer.engage();
            state.cinematic_mixer.set_profile_music_lufs(
                Some(profile.music_lufs_outside_narration),
                Some(profile.music_lufs_during_narration),
            );
            state.cinematic_takeover = cf_cinematic::CinematicTakeoverSnapshot {
                active: true,
                translation: [0.0, 0.0],
                shake_px: [0.0, 0.0],
                ortho_half_height: 0.0,
                color_grade: cf_cinematic::ColorGradeSnapshot {
                    saturation: profile.color_grade.saturation,
                    value: profile.color_grade.value,
                    contrast: profile.color_grade.contrast,
                },
                paused: false,
            };
        } else {
            state.cinematic_mixer.release();
            state.cinematic_takeover = cf_cinematic::CinematicTakeoverSnapshot::default();
        }
        state.cinematic_kernel = Some(kernel);
        drop(state);
        for ev in events {
            self.emit_cinematic_event(tick, sim_time_ms, &ev, parent.as_deref());
        }
    }

    /// per-frame loop calls this between physics + render steps. Emits
    /// any `Started` / `Chapter` / `NarrationWord` / `Ended` events
    /// that fired during the advance. Returns the kernel state after
    /// the advance (or `None` when no cinematic is active).
    pub fn advance_cinematic_kernel(&self, dt_ms: u32) -> Option<cf_cinematic::CinematicState> {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick();
        let sim_time_ms = state.clock.sim_time_ms();
        let (events, snapshot, kernel_ended, profile, seen) = {
            let kernel = state.cinematic_kernel.as_mut()?;
            let events = kernel.advance(dt_ms);
            let snapshot = kernel.state().clone();
            let kernel_ended = matches!(snapshot.phase, cf_cinematic::PlaybackPhase::Ended);
            let profile = kernel.profile().clone();
            let seen = kernel.seen().clone();
            (events, snapshot, kernel_ended, profile, seen)
        };
        // Mirror camera takeover snapshot for the renderer bridge.
        let color_grade = cf_cinematic::ColorGradeSnapshot {
            saturation: profile.color_grade.saturation,
            value: profile.color_grade.value,
            contrast: profile.color_grade.contrast,
        };
        state.cinematic_takeover = cf_cinematic::CinematicTakeoverSnapshot {
            active: !kernel_ended,
            translation: snapshot.camera_translation,
            shake_px: snapshot.camera_shake_px,
            ortho_half_height: snapshot.camera_ortho_half_height,
            color_grade,
            paused: snapshot.paused,
        };
        // Drive the mixer narration-active state from the kernel's word
        // crossings (event-driven below would lose state across paused
        // ticks; the kernel snapshot's `active_word_index` is the ground
        // truth).
        let narration_active = snapshot.active_word_index.is_some() && !snapshot.paused;
        state.cinematic_mixer.set_narration_active(narration_active);
        state.cinematic_mixer.tick(dt_ms);
        state.cinematic_seen_set = seen;
        let parent = state.run_started_event_id.clone();
        if kernel_ended {
            // next tick" — release the takeover + mixer when the kernel
            // ends so the renderer falls back to gameplay camera.
            state.cinematic_kernel = None;
            state.cinematic_mixer.release();
            state.cinematic_takeover = cf_cinematic::CinematicTakeoverSnapshot::default();
        }
        drop(state);
        for ev in events {
            self.emit_cinematic_event(tick, sim_time_ms, &ev, parent.as_deref());
        }
        Some(snapshot)
    }

    /// snapshot. cf-app's bridge mirrors this into
    /// `cf-render-2d::camera_takeover::CinematicCameraTakeover` each
    /// frame. Per spec § Notes for the implementer: "The cinematic
    /// camera REPLACES the gameplay camera at the render layer
    /// (`cf-render-2d::camera_takeover`)".
    pub fn cinematic_takeover_snapshot(&self) -> cf_cinematic::CinematicTakeoverSnapshot {
        self.state.read().ok().map(|s| s.cinematic_takeover).unwrap_or_default()
    }

    /// bridge consumes this each frame to attenuate the live audio
    /// backend per spec § "Cinematic mixer ducks music under narration".
    pub fn cinematic_mixer_snapshot(&self) -> cf_audio::CinematicMix {
        self.state
            .read()
            .ok()
            .map(|s| *s.cinematic_mixer.mix())
            .unwrap_or_default()
    }

    /// cinematics the player has watched. Per spec § Notes: "Codex
    /// unlock state lives in `save.cinematic_seen_set: HashSet<CinematicId>`;
    /// persisted via M41 save format." M41 reads this; M12C ships the
    /// in-memory mirror.
    pub fn cinematic_seen_set(&self) -> cf_cinematic::SeenSet {
        self.state
            .read()
            .map(|s| s.cinematic_seen_set.clone())
            .unwrap_or_default()
    }

    /// any in-memory seen-set the engine had accumulated. Per spec §
    /// Notes: "Codex unlock state lives in `save.cinematic_seen_set
    /// : HashSet<CinematicId>`; persisted via M41 save format."
    pub fn restore_cinematic_seen_set(&self, seen: cf_cinematic::SeenSet) {
        if let Ok(mut state) = self.state.write() {
            state.cinematic_seen_set = seen;
        }
    }

    /// reads the active storyteller from M25 director state and applies
    /// its profile globally." M25 is still active; M12C reads from
    /// `Settings.storyteller`, falling back to Cassandra Classic.
    pub fn active_storyteller(&self) -> cf_cinematic::StorytellerId {
        let settings = self.current_settings();
        cf_cinematic::StorytellerId::from_str(&settings.storyteller)
            .unwrap_or(cf_cinematic::StorytellerId::CassandraClassic)
    }

    /// Between-mission cinematic ("40% chance the rival's faction-
    /// channel taunt plays over a static portrait card"). Drained by
    /// the between-mission engage path. M25 will fold real rival-alive
    /// state into this gate when it ships.
    pub fn cinematic_rival_taunt_should_play(&self) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        // Use the deterministic per-engine RNG so two runs at the same
        // seed observe the same rival-taunt sequence.
        let v = state.rng.next_u64();
        // 40% gate: roll < (u64::MAX * 0.4). Compare via shifted compare
        // to avoid overflow on multiplication.
        let threshold = (u64::MAX / 10) * 4;
        let result = v < threshold;
        state.cinematic_rival_taunt_roll = state.cinematic_rival_taunt_roll.wrapping_add(1);
        result
    }
}

