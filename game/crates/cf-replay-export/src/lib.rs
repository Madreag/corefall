//! M10B: replay-as-MP4 export pipeline.
//!
//! `cf-replay-export` owns the offline video-output surface enumerated in
//! `specs/active/M10B.md`:
//!
//! - [`ffmpeg_bridge`] — libav wrapper (`ffmpeg-next` 8.1.0 against
//!   FFmpeg 8.0.1). Deterministic-encoder profile constants: single-thread
//!   + locked GOP + `-tune psnr` for the four production presets; FFV1
//!   intra-frame for the `archival_lossless` archival preset.
//! - [`preset_registry`] — typed loader + in-process registry for the
//!   five player-facing presets (`twitch_1080p60`, `youtube_4k60`,
//!   `discord_720p30`, `clip_compact`, `archival_lossless`). RON files
//!   live under `game/content/replay_export/presets/`.
//! - [`chapter_markers`] — data-driven chapter rule loader. Maps M4
//!   event types (`mission.objective_*`, `actor_status_changed=killed`,
//!   `reactor.armor_layer_destroyed`, `atmos.breach_detected`,
//!   `mission.commander_*`) AND the M9B trench events + M9C
//!   fortification events into MP4 chapter markers. RON lives at
//!   `game/content/replay_export/chapter_rules/default.ron`.
//!
//! Subsequent m10b features (m10b-2 frame_ticker + offline_render, m10b-3
//! overlay_graph + commentary, m10b-4 editor + cfctl shim, m10b-5
//! schemas + CI scripts) consume these three modules.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::missing_const_for_fn,
    clippy::doc_markdown,
    clippy::doc_lazy_continuation,
    clippy::doc_overindented_list_items,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::return_self_not_must_use,
    clippy::items_after_statements,
    clippy::needless_pass_by_value,
    clippy::too_many_lines,
    clippy::match_same_arms,
    clippy::default_trait_access,
    clippy::single_match_else,
    clippy::if_not_else,
    clippy::similar_names,
    // PresetError + ChapterRulesError embed `ron::error::SpannedError`
    // (≥128 bytes) so loaders can pinpoint the offending field. We
    // accept the larger Result size to keep the typed-error contract
    // per VAL-M10B-010 / VAL-M10B-011 (no generic-string errors).
    clippy::result_large_err,
    clippy::stable_sort_primitive
)]

pub mod chapter_markers;
pub mod ffmpeg_bridge;
pub mod preset_registry;

pub use chapter_markers::{
    ChapterRule, ChapterRuleSet, ChapterRulesError, COMMANDER_EVENT_PREFIX, DEFAULT_CHAPTER_RULES_RON,
    REQUIRED_M4_EVENT_KINDS, REQUIRED_M9B_EVENT_KINDS, REQUIRED_M9C_EVENT_KINDS,
};
pub use ffmpeg_bridge::{
    DeterministicEncoderProfile, FfmpegBridge, FfmpegProbeError, FfmpegRuntime, ARCHIVAL_LOSSLESS_GOP_SIZE,
    ARCHIVAL_LOSSLESS_TUNE, FFMPEG_NEXT_PIN, PRODUCTION_GOP_SIZE, PRODUCTION_TUNE, REQUIRED_FFMPEG_THREADS,
};
pub use preset_registry::{
    ContainerKind, ExportPreset, PresetCodec, PresetError, PresetField, PresetRegistry, PresetResolution,
    ARCHIVAL_LOSSLESS_NAME, CLIP_COMPACT_NAME, DECLARED_PRESETS, DISCORD_720P30_NAME, PRESET_REQUIRED_FIELDS,
    TWITCH_1080P60_NAME, YOUTUBE_4K60_NAME,
};
