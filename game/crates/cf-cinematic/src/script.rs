//! **M12C**: RON loader for `<id>.cinematic.ron` script files.
//!
//! Schema validates per-shot move-stack + chapter list + narration track
//! ref. Per spec § "Files":
//!
//! - `game/content/cinematics/opening/<mission_id>.cinematic.ron`
//! - `game/content/cinematics/between/<storyteller_id>_<variant>.cinematic.ron`
//! - `game/content/cinematics/ending/<storyteller_id>.cinematic.ron`
//!
//! Determinism: the loader hashes the file bytes (blake3) into the
//! `CinematicScript::source_sha256` so two engines loading the same RON
//! see byte-identical scripts.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::camera_moves::ShotMove;
use crate::storyteller_profile::StorytellerId;
use crate::CINEMATIC_SCHEMA_VERSION;

/// Stable identifier for a cinematic; used as map key in
/// `save.cinematic_seen_set` + codex replay surface + chapter-marker
/// payloads.
pub type CinematicId = String;

/// 0-based index into [`CinematicScript::shots`]. The renderer's camera-
/// move composer keys off this index to derive the shake noise seed
/// deterministically (spec § "the kernel passes the M4 replay seed +
/// per-shot index").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShotIndex(pub u32);

/// A chapter beat the kernel auto-emits via `cinematic.chapter_marker`
/// when the playhead crosses `at_ms`. Per spec § "Chapter markers are
/// author-defined in the script (`chapters: [{ id, at_ms }]`)".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChapterMarker {
    /// Author-defined chapter id (e.g. `"dropship_door_opens"`,
    /// `"boss_silhouette_reveal"`, `"rival_taunt"`).
    pub id: String,
    /// Tick offset (in ms from cinematic start) when the marker fires.
    pub at_ms: u32,
}

/// actor pose ID for a shot. Pose IDs resolve against the M9A animation
/// frame catalog (e.g. `"squad_low_ready"`, `"chassis_idle"`). The
/// cinematic kernel emits these as `cinematic.chapter_marker` payload
/// metadata so cf-app's animation bridge can swap the actor's frame.
///
/// (never — actors use M9A animation frames + scripted pose IDs)."
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorPose {
    /// Actor reference (string id resolved against the active scenario's
    /// actor table — e.g. `"player"`, `"squad_alpha"`, or a numeric id).
    pub actor_id: String,
    /// Pose id from the M9A animation catalog (e.g. `"low_ready"`,
    /// `"chassis_idle"`, `"crouched_aim"`).
    pub pose_id: String,
}

/// One scripted shot within a cinematic. The composer applies each move
/// in `moves` to the cinematic camera stack additively in declared order;
/// the renderer reads the final composed transform from
/// `cf-render-2d::camera_takeover`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shot {
    /// Author-defined shot name (e.g. `"dropship_landing"`). Used in
    /// editor tooling + diagnostics; not part of the chapter event
    /// surface.
    #[serde(default)]
    pub label: String,
    /// Total shot duration in ms. The composer clamps each move's
    /// `duration_ms` against this when computing the active range.
    pub duration_ms: u32,
    /// Move-stack — Pan / Dolly / Zoom / Orbit / Shake primitives. Per
    /// spec § "Camera primitives compose into a per-shot move-stack".
    pub moves: Vec<ShotMove>,
    /// enter their pre-mission stance (chassis idle + weapon at
    /// low-ready + storyteller-specific body language)". Authored
    /// actor pose declarations the cinematic bridge applies to the
    /// referenced actors at shot start.
    #[serde(default)]
    pub actor_poses: Vec<ActorPose>,
}

/// On-disk schema for `content/cinematics/**/<id>.cinematic.ron`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CinematicScript {
    /// Schema version for the on-disk file. Bumped when the file
    /// surface changes incompatibly.
    pub schema_version: u32,
    /// Stable id (matches the filename stem).
    pub id: CinematicId,
    /// Source classification (opening / between / ending). Picks the
    /// `cinematic.started.source` enum + the cf-shell hook that fires it.
    pub source: ScriptSource,
    /// Active storyteller scope, or `None` when the script is
    /// storyteller-agnostic (e.g. mission openings). The kernel picks
    /// the active storyteller from M25 director state at playback time.
    #[serde(default)]
    pub storyteller: Option<StorytellerId>,
    /// Per-shot script in playback order.
    pub shots: Vec<Shot>,
    /// Chapter markers in playback order. The scheduler emits them
    /// when the playhead crosses each `at_ms`.
    #[serde(default)]
    pub chapters: Vec<ChapterMarker>,
    /// Optional reference to the bake narration WAV (relative to
    /// `game/content/audio/voice/cinematic/`). The cinematic plays
    /// silently without narration when this is empty.
    #[serde(default)]
    pub narration_track_id: Option<String>,
    /// Briefing card lines (spec § "6-line briefing fades in over the
    /// lower third"). Empty = no card. Only meaningful for openings.
    #[serde(default)]
    pub briefing_card_lines: Vec<String>,
    /// When (ms) the briefing card fades in. Default 15000 ms per spec
    /// § "at T+15s, a 6-line briefing fades in".
    #[serde(default = "default_briefing_at_ms")]
    pub briefing_at_ms: u32,
    /// BLAKE3 of the source RON bytes. Filled in by the loader; not
    /// part of the on-disk authored data.
    #[serde(skip)]
    pub source_sha256: Option<String>,
}

fn default_briefing_at_ms() -> u32 {
    15_000
}

/// Source classification for the `cinematic.started.source` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScriptSource {
    /// Plays automatically when a mission scenario boots (30-60s).
    Opening,
    /// Plays in the base scene between mission departures (15-30s).
    Between,
    /// 3-act campaign ending (2-5min) at M49 launch BP12.
    Ending,
}

impl ScriptSource {
    /// Canonical snake_case identifier matching the event schema enum.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ScriptSource::Opening => "opening",
            ScriptSource::Between => "between",
            ScriptSource::Ending => "ending",
        }
    }

    /// (opening) / "15-30s" (between) / "between 120000 and 300000 ms"
    /// (ending). Returns `(min_ms, max_ms)`.
    #[must_use]
    pub fn duration_range_ms(self) -> (u32, u32) {
        match self {
            ScriptSource::Opening => (30_000, 60_000),
            ScriptSource::Between => (15_000, 30_000),
            ScriptSource::Ending => (120_000, 300_000),
        }
    }
}

/// Errors raised by the script loader.
#[derive(Debug, Error)]
pub enum ScriptLoadError {
    /// `ron::de::SpannedError` wrapper.
    #[error("cinematic script parse failed: {0}")]
    Parse(String),
    /// Validation failure (duration out of range, duplicate chapter id,
    /// etc.).
    #[error("cinematic script validation failed: {0}")]
    Validation(String),
}

impl CinematicScript {
    /// Parse a script from RON source bytes, fill the BLAKE3 hash, and
    /// run schema-level validation.
    ///
    /// boot on a missing cinematic file": the caller decides what to do
    /// on filesystem-missing errors; this function only handles
    /// supplied bytes.
    pub fn from_ron(bytes: &[u8]) -> Result<Self, ScriptLoadError> {
        let s = std::str::from_utf8(bytes).map_err(|e| ScriptLoadError::Parse(e.to_string()))?;
        let mut script: CinematicScript =
            ron::from_str(s).map_err(|e| ScriptLoadError::Parse(e.to_string()))?;
        let sha = blake3::hash(bytes);
        script.source_sha256 = Some(hex::encode(sha.as_bytes()));
        script.validate()?;
        Ok(script)
    }

    /// Total duration in ms — sum of every shot's `duration_ms`.
    #[must_use]
    pub fn total_duration_ms(&self) -> u32 {
        self.shots.iter().map(|s| s.duration_ms).sum()
    }

    /// Validation: schema version compatibility + duration window +
    /// chapter ordering + chapter id uniqueness + chapter `at_ms`
    /// staying inside `total_duration_ms()`.
    pub fn validate(&self) -> Result<(), ScriptLoadError> {
        if self.schema_version != CINEMATIC_SCHEMA_VERSION {
            return Err(ScriptLoadError::Validation(format!(
                "schema_version {} != {}",
                self.schema_version, CINEMATIC_SCHEMA_VERSION
            )));
        }
        if self.id.is_empty() {
            return Err(ScriptLoadError::Validation("id must not be empty".to_string()));
        }
        if self.shots.is_empty() {
            return Err(ScriptLoadError::Validation(format!(
                "{}: shots must not be empty",
                self.id
            )));
        }
        let total = self.total_duration_ms();
        let (lo, hi) = self.source.duration_range_ms();
        if total < lo || total > hi {
            return Err(ScriptLoadError::Validation(format!(
                "{}: total duration {} ms outside [{}, {}] for source {}",
                self.id,
                total,
                lo,
                hi,
                self.source.as_str(),
            )));
        }
        let mut seen: BTreeSet<&str> = BTreeSet::new();
        let mut last_at = 0u32;
        for ch in &self.chapters {
            if !seen.insert(ch.id.as_str()) {
                return Err(ScriptLoadError::Validation(format!(
                    "{}: duplicate chapter id {}",
                    self.id, ch.id
                )));
            }
            if ch.at_ms < last_at {
                return Err(ScriptLoadError::Validation(format!(
                    "{}: chapter {} at_ms {} out of order (< {})",
                    self.id, ch.id, ch.at_ms, last_at
                )));
            }
            if ch.at_ms > total {
                return Err(ScriptLoadError::Validation(format!(
                    "{}: chapter {} at_ms {} exceeds total {}",
                    self.id, ch.id, ch.at_ms, total
                )));
            }
            last_at = ch.at_ms;
        }
        if self.briefing_card_lines.len() > 8 {
            return Err(ScriptLoadError::Validation(format!(
                "{}: briefing_card_lines length {} > 8",
                self.id,
                self.briefing_card_lines.len()
            )));
        }
        Ok(())
    }

    /// Locate the shot index + intra-shot t (ms) for a global playhead
    /// position. Returns `None` when `playhead_ms` is beyond the script
    /// total duration.
    #[must_use]
    pub fn shot_at_ms(&self, playhead_ms: u32) -> Option<(ShotIndex, u32)> {
        let mut acc = 0u32;
        for (i, shot) in self.shots.iter().enumerate() {
            let next = acc + shot.duration_ms;
            if playhead_ms < next {
                return Some((ShotIndex(i as u32), playhead_ms - acc));
            }
            acc = next;
        }
        None
    }

    /// Returns every chapter whose `at_ms` lies in the half-open interval
    /// `[prev_ms, curr_ms)`. Used by the scheduler to fire pending
    /// markers each tick boundary.
    #[must_use]
    pub fn chapters_in_range(&self, prev_ms: u32, curr_ms: u32) -> Vec<&ChapterMarker> {
        if curr_ms <= prev_ms {
            return Vec::new();
        }
        self.chapters
            .iter()
            .filter(|c| c.at_ms >= prev_ms && c.at_ms < curr_ms)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera_moves::{EaseKind, MoveKind};

    fn make_shot(label: &str, duration_ms: u32) -> Shot {
        Shot {
            label: label.to_string(),
            duration_ms,
            moves: vec![ShotMove {
                kind: MoveKind::Pan,
                start_ms: 0,
                duration_ms,
                easing: EaseKind::EaseInOutCubic,
                pan: [10.0, 0.0],
                ..ShotMove::default()
            }],
            actor_poses: Vec::new(),
        }
    }

    #[test]
    fn script_validates_opening_duration_window() {
        let mut s = CinematicScript {
            schema_version: CINEMATIC_SCHEMA_VERSION,
            id: "cin_test_open".to_string(),
            source: ScriptSource::Opening,
            storyteller: None,
            shots: vec![make_shot("a", 30_000)],
            chapters: vec![],
            narration_track_id: None,
            briefing_card_lines: vec![],
            briefing_at_ms: 15_000,
            source_sha256: None,
        };
        s.validate().expect("valid 30s opening");
        s.shots = vec![make_shot("a", 10_000)];
        assert!(s.validate().is_err(), "10s opening rejected (< 30s)");
    }

    #[test]
    fn script_validates_chapter_order_and_uniqueness() {
        let mut s = CinematicScript {
            schema_version: CINEMATIC_SCHEMA_VERSION,
            id: "cin_chapters".to_string(),
            source: ScriptSource::Opening,
            storyteller: None,
            shots: vec![make_shot("a", 30_000)],
            chapters: vec![
                ChapterMarker {
                    id: "a".to_string(),
                    at_ms: 1_000,
                },
                ChapterMarker {
                    id: "a".to_string(),
                    at_ms: 2_000,
                },
            ],
            narration_track_id: None,
            briefing_card_lines: vec![],
            briefing_at_ms: 15_000,
            source_sha256: None,
        };
        assert!(s.validate().is_err(), "duplicate chapter id rejected");
        s.chapters[1].id = "b".to_string();
        s.chapters[1].at_ms = 500;
        assert!(s.validate().is_err(), "out-of-order chapters rejected");
    }

    #[test]
    fn chapters_in_range_returns_inclusive_lower_bound() {
        let s = CinematicScript {
            schema_version: CINEMATIC_SCHEMA_VERSION,
            id: "cin_range".to_string(),
            source: ScriptSource::Opening,
            storyteller: None,
            shots: vec![make_shot("a", 60_000)],
            chapters: vec![
                ChapterMarker {
                    id: "a".to_string(),
                    at_ms: 8_000,
                },
                ChapterMarker {
                    id: "b".to_string(),
                    at_ms: 22_000,
                },
            ],
            narration_track_id: None,
            briefing_card_lines: vec![],
            briefing_at_ms: 15_000,
            source_sha256: None,
        };
        // First tick crossing 8000.
        let mid = s.chapters_in_range(0, 8_500);
        assert_eq!(mid.len(), 1);
        assert_eq!(mid[0].id, "a");
        // Subsequent tick — boundary already past.
        let later = s.chapters_in_range(8_500, 21_000);
        assert!(later.is_empty());
        let crossing_b = s.chapters_in_range(21_000, 22_500);
        assert_eq!(crossing_b.len(), 1);
        assert_eq!(crossing_b[0].id, "b");
    }

    #[test]
    fn shot_at_ms_resolves_per_shot_offset() {
        let s = CinematicScript {
            schema_version: CINEMATIC_SCHEMA_VERSION,
            id: "cin_lookup".to_string(),
            source: ScriptSource::Opening,
            storyteller: None,
            shots: vec![make_shot("a", 10_000), make_shot("b", 20_000), make_shot("c", 30_000)],
            chapters: vec![],
            narration_track_id: None,
            briefing_card_lines: vec![],
            briefing_at_ms: 15_000,
            source_sha256: None,
        };
        assert_eq!(s.shot_at_ms(0), Some((ShotIndex(0), 0)));
        assert_eq!(s.shot_at_ms(5_000), Some((ShotIndex(0), 5_000)));
        assert_eq!(s.shot_at_ms(10_000), Some((ShotIndex(1), 0)));
        assert_eq!(s.shot_at_ms(15_000), Some((ShotIndex(1), 5_000)));
        assert_eq!(s.shot_at_ms(30_000), Some((ShotIndex(2), 0)));
        assert_eq!(s.shot_at_ms(60_000), None);
    }

    #[test]
    fn from_ron_round_trips_minimal_script() {
        let ron_src = r#"(
            schema_version: 1,
            id: "cin_round_trip",
            source: opening,
            storyteller: None,
            shots: [
                (
                    label: "open",
                    duration_ms: 30000,
                    moves: [],
                ),
            ],
            chapters: [],
            narration_track_id: None,
            briefing_card_lines: [],
            briefing_at_ms: 15000,
        )"#;
        let parsed = CinematicScript::from_ron(ron_src.as_bytes()).expect("parse");
        assert_eq!(parsed.id, "cin_round_trip");
        assert_eq!(parsed.source, ScriptSource::Opening);
        assert_eq!(parsed.total_duration_ms(), 30_000);
        assert!(parsed.source_sha256.is_some());
    }
}
