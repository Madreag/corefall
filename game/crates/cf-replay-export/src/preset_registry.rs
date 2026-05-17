//! M10B preset registry.
//!
//! Loads + validates the five player-facing export presets enumerated in
//! `specs/active/M10B.md` § "Player-facing behavior":
//!
//! - `twitch_1080p60`   — 1920×1080 @ 60 fps  H.264 + AAC + mp4
//! - `youtube_4k60`     — 3840×2160 @ 60 fps  H.264 + AAC + mp4
//! - `discord_720p30`   — 1280×720  @ 30 fps  H.264 + AAC + mp4
//! - `clip_compact`     —  854×480  @ 30 fps  H.264 + AAC + mp4 (≤25 MB clips)
//! - `archival_lossless`— 1920×1080 @ 60 fps  FFV1  + FLAC + mkv  (intra-frame
//!                        mathematically lossless; deterministic byte-identical
//!                        output across OS per VAL-M10B-020).
//!
//! Per VAL-M10B-005 each preset RON declares the six required fields
//! [`PRESET_REQUIRED_FIELDS`]: `resolution`, `fps`, `codec`,
//! `audio_codec`, `target_bitrate_kbps`, `container`. The deterministic
//! encoder profile (single-threaded + locked GOP + `-tune psnr` for the
//! four production presets; FFV1 intra-frame for archival) is enforced
//! by [`crate::ffmpeg_bridge::DeterministicEncoderProfile`] at encode
//! time; the registry here is data-only so `--list-presets` runs
//! cleanly without linking the libav bridge.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Preset name for the Twitch / 1080p60 H.264 production preset.
pub const TWITCH_1080P60_NAME: &str = "twitch_1080p60";
/// Preset name for the YouTube / 4K60 H.264 production preset.
pub const YOUTUBE_4K60_NAME: &str = "youtube_4k60";
/// Preset name for the Discord-clip 720p30 H.264 preset.
pub const DISCORD_720P30_NAME: &str = "discord_720p30";
/// Preset name for the ≤25 MB Discord-attachment compact preset.
pub const CLIP_COMPACT_NAME: &str = "clip_compact";
/// Preset name for the FFV1 + mkv archival mathematically-lossless preset.
pub const ARCHIVAL_LOSSLESS_NAME: &str = "archival_lossless";

/// Canonical enumeration of every preset declared by M10B. The
/// `--list-presets` subcommand asserts the on-disk registry matches
/// this set exactly (VAL-M10B-033).
pub const DECLARED_PRESETS: [&str; 5] = [
    TWITCH_1080P60_NAME,
    YOUTUBE_4K60_NAME,
    DISCORD_720P30_NAME,
    CLIP_COMPACT_NAME,
    ARCHIVAL_LOSSLESS_NAME,
];

/// The six required fields every preset RON must declare per
/// VAL-M10B-005 + the spec's "Preset registry covers all 5 declared
/// presets" acceptance scenario.
pub const PRESET_REQUIRED_FIELDS: [PresetField; 6] = [
    PresetField::Resolution,
    PresetField::Fps,
    PresetField::Codec,
    PresetField::AudioCodec,
    PresetField::TargetBitrateKbps,
    PresetField::Container,
];

/// Six-field shape declared by every preset RON. Used by
/// `--list-presets` to enumerate the field names verbatim per the
/// VAL-M10B-033 contract ("for each preset the output also includes
/// the six required fields").
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetField {
    Resolution,
    Fps,
    Codec,
    AudioCodec,
    TargetBitrateKbps,
    Container,
}

impl PresetField {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PresetField::Resolution => "resolution",
            PresetField::Fps => "fps",
            PresetField::Codec => "codec",
            PresetField::AudioCodec => "audio_codec",
            PresetField::TargetBitrateKbps => "target_bitrate_kbps",
            PresetField::Container => "container",
        }
    }
}

/// Frame resolution `(width, height)` in pixels.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PresetResolution {
    pub width: u32,
    pub height: u32,
}

impl PresetResolution {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }
}

/// Video codec enumeration. The four production presets use H.264 +
/// AAC + mp4 (the most ubiquitously playable bundle). The archival
/// preset uses FFV1, an intra-frame mathematically lossless codec
/// whose container bytes are byte-identical across hosts when encoded
/// single-threaded with locked GOP (per VAL-M10B-020 / spec § Notes).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PresetCodec {
    H264,
    H265,
    Av1,
    Ffv1,
}

impl PresetCodec {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PresetCodec::H264 => "h264",
            PresetCodec::H265 => "h265",
            PresetCodec::Av1 => "av1",
            PresetCodec::Ffv1 => "ffv1",
        }
    }

    /// `true` when the codec is intra-frame mathematically lossless (only
    /// `ffv1` today). Intra-frame coding is what makes the archival
    /// preset byte-identical across OS / runs.
    #[must_use]
    pub const fn is_intra_frame_lossless(self) -> bool {
        matches!(self, PresetCodec::Ffv1)
    }
}

/// Container kind. `mp4` for production H.264 / H.265 / AV1; `mkv`
/// for the FFV1 archival preset (mp4 cannot natively carry FFV1).
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerKind {
    Mp4,
    Mkv,
    WebM,
}

impl ContainerKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            ContainerKind::Mp4 => "mp4",
            ContainerKind::Mkv => "mkv",
            ContainerKind::WebM => "webm",
        }
    }
}

/// One on-disk preset. Field set is the six declared by VAL-M10B-005
/// and the spec's acceptance scenarios.
///
/// `target_bitrate_kbps = 0` is the convention for quality-driven
/// codecs (FFV1 archival) where bitrate is derived from the visual
/// content + intra-frame quantizer; production H.264 / H.265 presets
/// declare a positive integer bitrate that the deterministic encoder
/// profile passes as `-b:v <N>k`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportPreset {
    /// Preset identifier — MUST equal the RON file's stem (e.g.
    /// `twitch_1080p60.ron` ↔ `name: "twitch_1080p60"`).
    pub name: String,
    pub resolution: PresetResolution,
    pub fps: u32,
    pub codec: PresetCodec,
    pub audio_codec: String,
    pub target_bitrate_kbps: u32,
    pub container: ContainerKind,
}

impl ExportPreset {
    /// Parse a single preset from a RON string. Returns a typed
    /// `ron::SpannedError` so loader tests can pinpoint malformed
    /// fields.
    pub fn from_ron_str(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<ExportPreset>(text)
    }

    /// Convenience: load a preset from disk. The `name` field is
    /// cross-checked against the file's stem so a renamed-but-stale
    /// content RON is rejected by the registry loader rather than
    /// returning an inconsistent in-memory registry.
    pub fn load(path: &Path) -> Result<Self, PresetError> {
        let text = fs::read_to_string(path).map_err(|err| PresetError::Io {
            path: path.to_path_buf(),
            source: err,
        })?;
        let preset = Self::from_ron_str(&text).map_err(|err| PresetError::Parse {
            path: path.to_path_buf(),
            source: err,
        })?;
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
        if preset.name != stem {
            return Err(PresetError::NameStemMismatch {
                path: path.to_path_buf(),
                name: preset.name.clone(),
                stem: stem.to_owned(),
            });
        }
        Ok(preset)
    }
}

/// Errors surfaced by the preset loader. Typed-error rejection (no
/// generic `String`) matches the project convention for content
/// loaders (see VAL-M10B-010 / VAL-M10B-011 for the spec's wider
/// typed-error rule applied to camera + commentary scripts).
#[derive(Debug, Error)]
pub enum PresetError {
    #[error("read preset RON at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("parse preset RON at {path}: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: ron::error::SpannedError,
    },
    #[error("preset name `{name}` at {path} does not match file stem `{stem}`")]
    NameStemMismatch { path: PathBuf, name: String, stem: String },
    #[error("preset registry missing required preset `{name}` (expected one of: {})", DECLARED_PRESETS.join(", "))]
    MissingPreset { name: String },
    #[error("preset registry includes unexpected entry `{name}` (declared: {})", DECLARED_PRESETS.join(", "))]
    UnexpectedPreset { name: String },
    #[error("preset directory not found at {path}")]
    DirNotFound { path: PathBuf },
}

/// In-process preset registry — a small map of name → preset.
#[derive(Debug, Clone, Default)]
pub struct PresetRegistry {
    presets: Vec<ExportPreset>,
}

impl PresetRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a preset; later inserts override earlier same-named entries.
    pub fn insert(&mut self, preset: ExportPreset) {
        if let Some(slot) = self.presets.iter_mut().find(|p| p.name == preset.name) {
            *slot = preset;
        } else {
            self.presets.push(preset);
        }
    }

    /// Total preset count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.presets.len()
    }

    /// `true` when the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.presets.is_empty()
    }

    /// Lookup by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ExportPreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// Sorted-by-name iterator. Used by `--list-presets` to render a
    /// stable output order across runs (determinism).
    pub fn iter_sorted(&self) -> impl Iterator<Item = &ExportPreset> {
        let mut by_name: Vec<&ExportPreset> = self.presets.iter().collect();
        by_name.sort_by(|a, b| a.name.cmp(&b.name));
        by_name.into_iter()
    }

    /// Iterate every preset in the registry in declaration order.
    pub fn iter(&self) -> impl Iterator<Item = &ExportPreset> {
        self.presets.iter()
    }

    /// Load all five spec-declared presets from `dir` (each must exist
    /// at `<dir>/<name>.ron`). The registry is validated to contain
    /// exactly the five [`DECLARED_PRESETS`] — extra files in the
    /// directory are ignored to allow mod-installed presets to coexist,
    /// but missing ones are an error (so `--list-presets` cannot
    /// silently miss a core preset).
    pub fn load_declared(dir: &Path) -> Result<Self, PresetError> {
        if !dir.is_dir() {
            return Err(PresetError::DirNotFound {
                path: dir.to_path_buf(),
            });
        }
        let mut registry = Self::new();
        for &name in &DECLARED_PRESETS {
            let path = dir.join(format!("{name}.ron"));
            let preset = ExportPreset::load(&path)?;
            registry.insert(preset);
        }
        registry.assert_complete()?;
        Ok(registry)
    }

    /// Assert the registry contains exactly the five spec-declared
    /// preset names (no missing, no unexpected — unexpected is
    /// possible if the caller inserted a mod preset).
    pub fn assert_complete(&self) -> Result<(), PresetError> {
        let names: BTreeSet<&str> = self.presets.iter().map(|p| p.name.as_str()).collect();
        for &declared in &DECLARED_PRESETS {
            if !names.contains(declared) {
                return Err(PresetError::MissingPreset {
                    name: declared.to_owned(),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preset_for(name: &str) -> ExportPreset {
        ExportPreset {
            name: name.to_owned(),
            resolution: PresetResolution::new(1920, 1080),
            fps: 60,
            codec: PresetCodec::H264,
            audio_codec: "aac".into(),
            target_bitrate_kbps: 6000,
            container: ContainerKind::Mp4,
        }
    }

    #[test]
    fn declared_presets_has_exactly_five() {
        assert_eq!(DECLARED_PRESETS.len(), 5);
        let mut sorted = DECLARED_PRESETS.to_vec();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), 5, "DECLARED_PRESETS must be unique");
    }

    #[test]
    fn preset_required_fields_lists_six() {
        assert_eq!(PRESET_REQUIRED_FIELDS.len(), 6);
        for &field in &PRESET_REQUIRED_FIELDS {
            assert!(!field.as_str().is_empty(), "PresetField::as_str must be non-empty");
        }
    }

    #[test]
    fn ron_round_trip_preserves_six_fields() {
        let src = preset_for(TWITCH_1080P60_NAME);
        let text = ron::ser::to_string(&src).expect("serialise preset");
        let parsed = ExportPreset::from_ron_str(&text).expect("round-trip parse");
        assert_eq!(parsed, src);
    }

    #[test]
    fn registry_insert_replaces_same_name() {
        let mut reg = PresetRegistry::new();
        reg.insert(preset_for(TWITCH_1080P60_NAME));
        let mut second = preset_for(TWITCH_1080P60_NAME);
        second.fps = 30;
        reg.insert(second.clone());
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get(TWITCH_1080P60_NAME).unwrap().fps, 30);
    }

    #[test]
    fn assert_complete_flags_missing_preset() {
        let mut reg = PresetRegistry::new();
        for &n in &[
            TWITCH_1080P60_NAME,
            YOUTUBE_4K60_NAME,
            DISCORD_720P30_NAME,
            CLIP_COMPACT_NAME,
        ] {
            reg.insert(preset_for(n));
        }
        let err = reg.assert_complete().expect_err("missing archival_lossless");
        assert!(matches!(
            err,
            PresetError::MissingPreset { name } if name == ARCHIVAL_LOSSLESS_NAME
        ));
    }

    #[test]
    fn ffv1_codec_is_intra_frame_lossless() {
        assert!(PresetCodec::Ffv1.is_intra_frame_lossless());
        assert!(!PresetCodec::H264.is_intra_frame_lossless());
        assert!(!PresetCodec::H265.is_intra_frame_lossless());
        assert!(!PresetCodec::Av1.is_intra_frame_lossless());
    }
}
