//! **M12C**: In-engine cinematic playback kernel.
//!
//! Spec § "Closes the gap M12 left open between **static painted slideshow**
//! (8 painted CCCP-style intro slides) and **dramatic in-engine moments**":
//! ships a scripted in-engine cinematic playback system that takes control
//! of the live camera + actors + lighting + audio mixer to play mission-
//! opening cinematics (30-60s), between-mission cinematics (15-30s), and a
//! campaign-ending cinematic (2-5min) using the actual game world.
//!
//! ## Determinism contract
//!
//! Per spec § Notes for the implementer:
//!
//! - "Shake noise MUST seed off the engine's deterministic RNG, not
//!   `thread_rng()` (per AGENTS.md sim-crate rule). The shake primitive
//!   accepts a `seed: u64` parameter; the kernel passes the M4 replay
//!   seed + per-shot index."
//! - "Skip-confirm window is 3000 ms; the value is a constant in
//!   `cf-cinematic::skip_pause_replay` and NOT a hardcoded `60` tick
//!   assumption — it converts via the active tick rate."
//! - "Replay-deterministic: pure functions of script + tick, no
//!   `thread_rng()`."
//!
//! ## Source-of-truth boundaries
//!
//! - **Script loader / shot scheduler / camera-move composer**:
//!   `cf-cinematic` (THIS crate; pure compute, no bevy).
//! - **Camera transform application**: `cf-render-2d::camera_takeover`
//!   (renderer-side stack that reads from the `CinematicState` resource).
//! - **Subtitle ribbon**: `cf-ui::caption_ribbon` (separate from
//!   gameplay caption strip).
//! - **Audio mix / LUFS duck**: `cf-audio::cinematic_mixer`.
//! - **cfctl surface**: `cf-control` adds `act.player.skip_cinematic`,
//!   `act.player.pause_cinematic`, `act.player.replay_cinematic`,
//!   `srv.dump_cinematic_state`.
//! - **Replay events**: `cf-replay` adds the 7 `cinematic.*` event schemas.
//! - **Mission-open / between / end hooks**: `cf-shell::cinematic_hooks`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![allow(clippy::module_name_repetitions)]

pub mod camera_moves;
pub mod narration_sync;
pub mod scheduler;
pub mod script;
pub mod skip_pause_replay;
pub mod storyteller_profile;

pub use camera_moves::{
    apply_move_stack, compose_offset, easing_sample, EaseKind, MoveKind, ShakeParams, ShotMove,
};
pub use narration_sync::{
    word_at_ms, NarrationTrack, NarrationWord, WordHighlightState,
};
pub use scheduler::{
    CinematicEvent, CinematicEventKind, CinematicKernel, CinematicSource, CinematicState,
    PlaybackPhase,
};
pub use script::{
    ChapterMarker, CinematicId, CinematicScript, ScriptLoadError, ScriptSource, Shot, ShotIndex,
};
pub use skip_pause_replay::{
    skip_allowed_at, SeenSet, SkipPauseReplayPolicy, SkipReason, SKIP_CONFIRM_WINDOW_MS,
};
pub use storyteller_profile::{
    builtin_profile, default_profiles, StorytellerId, StorytellerProfile, COLOR_GRADE_NEUTRAL,
};

/// Schema version for cinematic on-disk RON files. Bumped when the script
/// schema (shots / moves / chapters / narration ref) gains or loses fields.
pub const CINEMATIC_SCHEMA_VERSION: u32 = 1;
