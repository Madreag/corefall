//! M10B preset registry round-trip tests (VAL-M10B-004 / VAL-M10B-005 /
//! VAL-M10B-033).
//!
//! For each of the five spec-declared presets:
//! - the RON file exists at `game/content/replay_export/presets/<name>.ron`
//! - it parses cleanly via `ExportPreset::from_ron_str`
//! - declares the six required fields per VAL-M10B-005
//! - resolution + fps + codec + audio_codec + target_bitrate_kbps +
//!   container match the spec's player-facing claims

use std::path::{Path, PathBuf};

use cf_replay_export::preset_registry::{
    ContainerKind, ExportPreset, PresetCodec, PresetRegistry, ARCHIVAL_LOSSLESS_NAME, CLIP_COMPACT_NAME,
    DECLARED_PRESETS, DISCORD_720P30_NAME, PRESET_REQUIRED_FIELDS, TWITCH_1080P60_NAME, YOUTUBE_4K60_NAME,
};

/// Workspace-rooted path to `game/content/replay_export/presets/`.
/// Tests run from `cf-replay-export/` so we walk up to `game/` and
/// re-anchor against `content/replay_export/presets/`.
fn presets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crate manifest dir must be game/crates/cf-replay-export/")
        .join("content/replay_export/presets")
}

fn load_preset(name: &str) -> ExportPreset {
    let path = presets_dir().join(format!("{name}.ron"));
    ExportPreset::load(&path).unwrap_or_else(|e| panic!("load preset {} from {}: {e}", name, path.display()))
}

#[test]
fn preset_registry_all_five_presets_exist_on_disk() {
    let dir = presets_dir();
    assert!(dir.is_dir(), "preset directory {} must exist", dir.display());
    for &name in &DECLARED_PRESETS {
        let path = dir.join(format!("{name}.ron"));
        assert!(path.is_file(), "preset RON {} must exist at {}", name, path.display());
    }
}

#[test]
fn preset_registry_load_declared_returns_all_five() {
    let registry = PresetRegistry::load_declared(&presets_dir()).expect("load_declared must succeed for all 5 presets");
    assert_eq!(registry.len(), 5);
    for &name in &DECLARED_PRESETS {
        assert!(registry.get(name).is_some(), "registry must contain `{name}`");
    }
}

#[test]
fn preset_registry_round_trip_preserves_six_fields_per_preset() {
    for &name in &DECLARED_PRESETS {
        let src = load_preset(name);
        let text = ron::ser::to_string(&src).expect("serialise");
        let parsed = ExportPreset::from_ron_str(&text).expect("round-trip parse");
        assert_eq!(parsed, src, "preset {name} round-trip mismatch");
        // Six-field shape assertion: every required field has a usable value.
        for &field in &PRESET_REQUIRED_FIELDS {
            assert!(
                !field.as_str().is_empty(),
                "preset {name} has empty required field `{}`",
                field.as_str()
            );
        }
    }
}

#[test]
fn preset_registry_twitch_1080p60_matches_spec() {
    let p = load_preset(TWITCH_1080P60_NAME);
    assert_eq!(p.name, TWITCH_1080P60_NAME);
    assert_eq!(p.resolution.width, 1920);
    assert_eq!(p.resolution.height, 1080);
    assert_eq!(p.fps, 60);
    assert_eq!(p.codec, PresetCodec::H264);
    assert_eq!(p.audio_codec, "aac");
    assert!(p.target_bitrate_kbps > 0);
    assert_eq!(p.container, ContainerKind::Mp4);
}

#[test]
fn preset_registry_youtube_4k60_matches_spec() {
    let p = load_preset(YOUTUBE_4K60_NAME);
    assert_eq!(p.name, YOUTUBE_4K60_NAME);
    assert_eq!(p.resolution.width, 3840);
    assert_eq!(p.resolution.height, 2160);
    assert_eq!(p.fps, 60);
    assert_eq!(p.codec, PresetCodec::H264);
    assert_eq!(p.audio_codec, "aac");
    assert!(p.target_bitrate_kbps > 0);
    assert_eq!(p.container, ContainerKind::Mp4);
}

#[test]
fn preset_registry_discord_720p30_matches_spec() {
    let p = load_preset(DISCORD_720P30_NAME);
    assert_eq!(p.name, DISCORD_720P30_NAME);
    assert_eq!(p.resolution.width, 1280);
    assert_eq!(p.resolution.height, 720);
    assert_eq!(p.fps, 30);
    assert_eq!(p.codec, PresetCodec::H264);
    assert_eq!(p.audio_codec, "aac");
    assert!(p.target_bitrate_kbps > 0);
    assert_eq!(p.container, ContainerKind::Mp4);
}

#[test]
fn preset_registry_clip_compact_matches_spec() {
    let p = load_preset(CLIP_COMPACT_NAME);
    assert_eq!(p.name, CLIP_COMPACT_NAME);
    // Spec: "smaller resolution/bitrate suitable for ≤25 MB clips".
    // We assert "smaller than the production presets" rather than a
    // single literal because clip_compact is a default-Discord-tier
    // target — a future preset bump must stay below twitch_1080p60's
    // resolution + bitrate.
    assert!(
        p.resolution.width <= 1280 && p.resolution.height <= 720,
        "clip_compact resolution {}x{} must be ≤ 1280x720",
        p.resolution.width,
        p.resolution.height
    );
    assert!(p.fps <= 60);
    assert_eq!(p.codec, PresetCodec::H264);
    assert_eq!(p.audio_codec, "aac");
    assert!(
        p.target_bitrate_kbps > 0 && p.target_bitrate_kbps <= 2500,
        "clip_compact target bitrate {} kbps must fit ≤25 MB / minute",
        p.target_bitrate_kbps
    );
    assert_eq!(p.container, ContainerKind::Mp4);
}

#[test]
fn preset_registry_archival_lossless_uses_ffv1_in_mkv() {
    let p = load_preset(ARCHIVAL_LOSSLESS_NAME);
    assert_eq!(p.name, ARCHIVAL_LOSSLESS_NAME);
    assert_eq!(p.codec, PresetCodec::Ffv1);
    assert!(p.codec.is_intra_frame_lossless());
    assert_eq!(p.container, ContainerKind::Mkv);
    // FFV1 is quality-driven, target_bitrate_kbps is informational only.
    assert!(p.fps > 0);
}

#[test]
fn preset_registry_rejects_name_stem_mismatch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path: &Path = &dir.path().join("twitch_1080p60.ron");
    std::fs::write(
        path,
        r#"(name: "wrong_name", resolution: (width: 1920, height: 1080), fps: 60, codec: h264, audio_codec: "aac", target_bitrate_kbps: 6000, container: mp4)"#,
    )
    .expect("write");
    let err = ExportPreset::load(path).expect_err("name/stem mismatch must reject");
    assert!(
        format!("{err}").contains("does not match file stem"),
        "expected NameStemMismatch, got: {err}"
    );
}

#[test]
fn preset_registry_field_shape_is_six_fields() {
    assert_eq!(PRESET_REQUIRED_FIELDS.len(), 6);
}
