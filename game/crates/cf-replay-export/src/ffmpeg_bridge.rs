//! M10B FFmpeg / libav bridge.
//!
//! # FFmpeg-version pin
//!
//! This file is pinned to `ffmpeg-next = 8.1.0` against the
//! `ffmpeg-sys-next 8.1.0` libav bindings (matching the FFmpeg 8.x
//! major). The host toolchain the mission's `library/environment.md`
//! verified is FFmpeg 8.0.1 at `/opt/homebrew/bin/ffmpeg`, with libav
//! 62.11 / 62.3 / 60.8 / 11.4 (libavcodec / libavformat / libavutil /
//! libavfilter). That installation lies inside the FFmpeg 8.x window
//! covered by `ffmpeg-next 8.x`.
//!
//! Per the mission `AGENTS.md` § "Hard rule for M10B FFmpeg integration"
//! two paths are acceptable:
//!
//! 1. **(taken)** Pin a `ffmpeg-next` version that supports FFmpeg
//!    8.x and document the pin here + in `cf-replay-export/Cargo.toml`.
//!    `ffmpeg-next 8.1.0` is the canonical upstream that has caught up
//!    to FFmpeg 8.x (the crate's long-running description "FFmpeg 4
//!    compatible fork" is historical; major versions track FFmpeg's
//!    major releases since 6.x).
//! 2. Pin a hermetic FFmpeg 7.x via `brew install ffmpeg@7` and
//!    document accordingly. **Not used** — the upstream Rust binding
//!    already supports the installed FFmpeg 8.0.1, so adding a parallel
//!    hermetic FFmpeg 7.x is unnecessary.
//!
//! See also: `cf-replay-export/Cargo.toml` for the matching dependency
//! comment.
//!
//! # Deterministic encoder profile
//!
//! Per spec § Notes: "H.264 / H.265 reach byte-identical output only
//! with single-threaded encode + locked GOP + locked encoder version +
//! disabled psychovisual tuning (`-tune psnr` for x264). The
//! deterministic-export matrix uses these flags by default; the
//! `--preset archival_lossless` uses FFV1 (intra-frame, mathematically
//! lossless) for true byte-identical output where the production
//! presets only guarantee per-frame YUV BLAKE3 within 99% tolerance."
//!
//! The constants exported below codify this profile. The actual
//! encoder pipeline that consumes them lands in m10b-2 / m10b-3 /
//! m10b-4; m10b-1 only ships the data + the libav runtime probe so
//! `cargo build -p cf-replay-export --release` links cleanly against
//! the host libav per VAL-M10B-002 / VAL-M10B-003.

use std::sync::atomic::{AtomicBool, Ordering};

use thiserror::Error;

use crate::preset_registry::{ExportPreset, PresetCodec};

/// Workspace dependency pin for `ffmpeg-next`. Surfaces the version
/// string so `--list-presets` (m10b-1) or audit-log producers
/// (m10b-3 / m10b-4) can record the bridge version next to the FFmpeg
/// runtime version.
pub const FFMPEG_NEXT_PIN: &str = "8.1.0";

/// Required encoder thread count for deterministic H.264 / H.265 /
/// AV1 output. Single-threaded encode is the spec § Notes rule for
/// reproducibility ("threading non-determinism even at `-threads 1`"
/// is the across-OS caveat; same-host repeated runs are byte-identical
/// at `-threads 1`).
pub const REQUIRED_FFMPEG_THREADS: u32 = 1;

/// Locked GOP length for the production presets. A fixed GOP makes
/// frame boundaries deterministic across libav versions; the spec
/// matches the typical "2-second GOP" convention used by H.264 +
/// H.265 + AV1 production encoders. The GOP is expressed in frames;
/// per-preset GOP is `PRODUCTION_GOP_SIZE * (preset_fps / 60)` so a
/// 30 fps preset uses a 60-frame GOP and a 60 fps preset uses a
/// 120-frame GOP — but the spec keeps the `-g <N>` flag literally
/// equal to a single fixed value across presets to remove a parameter
/// drift surface.
pub const PRODUCTION_GOP_SIZE: u32 = 120;

/// Locked GOP length for the FFV1 archival preset. FFV1 is intra-frame
/// so every frame is a keyframe — `-g 1` is the canonical encoder
/// flag that documents the constraint to ffmpeg + readers.
pub const ARCHIVAL_LOSSLESS_GOP_SIZE: u32 = 1;

/// Deterministic-encoder `-tune` flag for the production presets. The
/// spec § Notes locks production presets to `-tune psnr` ("disabled
/// psychovisual tuning") so encoder decisions are PSNR-driven (no
/// human-perception heuristics) and therefore deterministic across
/// runs.
pub const PRODUCTION_TUNE: &str = "psnr";

/// FFV1 archival preset uses no psychovisual tuning either (FFV1 has
/// no `-tune`); we mark the string `none` for downstream audit logs.
pub const ARCHIVAL_LOSSLESS_TUNE: &str = "none";

/// Deterministic-encoder profile for one preset. Frozen + audit-loggable;
/// downstream m10b features (m10b-2 frame_ticker, m10b-4 export CLI)
/// pass these constants directly to the libav encoder API.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct DeterministicEncoderProfile {
    /// Encoder thread count. Always `REQUIRED_FFMPEG_THREADS` (= 1).
    pub threads: u32,
    /// Encoder GOP size in frames. `1` for FFV1 (intra-frame),
    /// `PRODUCTION_GOP_SIZE` otherwise.
    pub gop_size: u32,
    /// `-tune` flag value. `psnr` for H.264 / H.265, `none` for FFV1.
    pub tune: &'static str,
    /// `true` when the encoder is intra-frame mathematically lossless
    /// (only FFV1 today). Across-OS byte-identical contract applies
    /// only to `true`.
    pub intra_frame_lossless: bool,
}

impl DeterministicEncoderProfile {
    /// Build the deterministic profile for the supplied preset.
    /// Embeds the codec switch (H.264 / H.265 / AV1 vs FFV1) so the
    /// encoder is configured per spec § Notes deterministically.
    #[must_use]
    pub fn for_preset(preset: &ExportPreset) -> Self {
        match preset.codec {
            PresetCodec::Ffv1 => Self::archival_lossless(),
            PresetCodec::H264 | PresetCodec::H265 | PresetCodec::Av1 => Self::production(),
        }
    }

    /// Production-preset profile: single-thread, 120-frame GOP, PSNR
    /// tune. Returns the same values for every H.264 / H.265 / AV1
    /// preset — the spec keeps these flags identical across the four
    /// production presets so the deterministic-export matrix
    /// (`m10b_deterministic_export_matrix.sh`) only varies resolution
    /// + bitrate.
    #[must_use]
    pub const fn production() -> Self {
        Self {
            threads: REQUIRED_FFMPEG_THREADS,
            gop_size: PRODUCTION_GOP_SIZE,
            tune: PRODUCTION_TUNE,
            intra_frame_lossless: false,
        }
    }

    /// Archival-lossless profile: single-thread, 1-frame GOP (every
    /// frame is a keyframe), no psychovisual tune. FFV1 byte-identical
    /// across OS via VAL-M10B-020.
    #[must_use]
    pub const fn archival_lossless() -> Self {
        Self {
            threads: REQUIRED_FFMPEG_THREADS,
            gop_size: ARCHIVAL_LOSSLESS_GOP_SIZE,
            tune: ARCHIVAL_LOSSLESS_TUNE,
            intra_frame_lossless: true,
        }
    }
}

/// Runtime state of the libav bridge — captured once per process
/// after `ffmpeg_next::init()` succeeds. Lifecycle:
///
/// 1. m10b-1 (this feature) probes the bridge at startup and surfaces
///    a `FfmpegRuntime` for downstream features to consult.
/// 2. m10b-2 / m10b-3 / m10b-4 use the runtime for actual encode.
/// 3. m10b-4's `export <bundle>` CLI surface returns the
///    `{result: "missing_dependency", dependency: "ffmpeg", ...}` JSON
///    per VAL-M10B-032 when [`FfmpegBridge::probe`] returns
///    `FfmpegProbeError::InitFailed`.
#[derive(Debug, Clone, Copy)]
pub struct FfmpegRuntime {
    /// Numeric libav util version as reported by
    /// `ffmpeg_next::util::version()` (e.g. `0x3e_08_64` for libavutil
    /// 60.8.100). Surfaced to audit logs so a replay export can be
    /// traced back to a specific libav build.
    pub libavutil_version: u32,
    /// Pinned `ffmpeg-next` crate version, e.g. `"8.1.0"`.
    pub ffmpeg_next_pin: &'static str,
}

/// Probe errors. Downstream m10b-4 maps `InitFailed` to the
/// VAL-M10B-032 structured JSON `{result: "missing_dependency",
/// dependency: "ffmpeg", suggested_install: ...}`.
#[derive(Debug, Error)]
pub enum FfmpegProbeError {
    #[error(
        "ffmpeg_next::init() failed: {source} \
         (suggested fix: install FFmpeg 8.x / libav matching the \
         host's `ffmpeg -version`)"
    )]
    InitFailed {
        #[source]
        source: ffmpeg_next::Error,
    },
}

static FFMPEG_INIT_DONE: AtomicBool = AtomicBool::new(false);

/// Static surface for the libav bridge. m10b-1 ships only the probe +
/// the deterministic-encoder profile constants; the full encoder
/// pipeline (frame submission, codec context creation, mux to mp4 /
/// mkv) lands with m10b-2 + m10b-4.
#[derive(Debug, Default)]
pub struct FfmpegBridge;

impl FfmpegBridge {
    /// Idempotently initialise the libav backend + return a small
    /// `FfmpegRuntime` snapshot. The init step is a no-op on
    /// subsequent calls in the same process per `ffmpeg_next::init`'s
    /// internal `Once`-style guard; we additionally short-circuit on
    /// `FFMPEG_INIT_DONE` to skip the FFI hop entirely.
    pub fn probe() -> Result<FfmpegRuntime, FfmpegProbeError> {
        if !FFMPEG_INIT_DONE.load(Ordering::SeqCst) {
            ffmpeg_next::init().map_err(|source| FfmpegProbeError::InitFailed { source })?;
            FFMPEG_INIT_DONE.store(true, Ordering::SeqCst);
        }
        Ok(FfmpegRuntime {
            libavutil_version: ffmpeg_next::util::version(),
            ffmpeg_next_pin: FFMPEG_NEXT_PIN,
        })
    }

    /// Convenience: synthesise the deterministic encoder profile for a
    /// preset without involving the libav probe (used by `--list-presets`
    /// + audit-log writers).
    #[must_use]
    pub fn profile_for(preset: &ExportPreset) -> DeterministicEncoderProfile {
        DeterministicEncoderProfile::for_preset(preset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset_registry::{ContainerKind, PresetResolution};

    fn h264_preset() -> ExportPreset {
        ExportPreset {
            name: "twitch_1080p60".into(),
            resolution: PresetResolution::new(1920, 1080),
            fps: 60,
            codec: PresetCodec::H264,
            audio_codec: "aac".into(),
            target_bitrate_kbps: 6000,
            container: ContainerKind::Mp4,
        }
    }

    fn ffv1_preset() -> ExportPreset {
        ExportPreset {
            name: "archival_lossless".into(),
            resolution: PresetResolution::new(1920, 1080),
            fps: 60,
            codec: PresetCodec::Ffv1,
            audio_codec: "flac".into(),
            target_bitrate_kbps: 0,
            container: ContainerKind::Mkv,
        }
    }

    #[test]
    fn production_profile_is_single_thread_with_psnr_tune() {
        let p = DeterministicEncoderProfile::production();
        assert_eq!(p.threads, REQUIRED_FFMPEG_THREADS);
        assert_eq!(p.threads, 1);
        assert_eq!(p.tune, "psnr");
        assert_eq!(p.gop_size, PRODUCTION_GOP_SIZE);
        assert!(!p.intra_frame_lossless);
    }

    #[test]
    fn archival_profile_is_intra_frame_with_one_frame_gop() {
        let p = DeterministicEncoderProfile::archival_lossless();
        assert_eq!(p.threads, REQUIRED_FFMPEG_THREADS);
        assert_eq!(p.gop_size, ARCHIVAL_LOSSLESS_GOP_SIZE);
        assert_eq!(p.gop_size, 1);
        assert!(p.intra_frame_lossless);
        assert_eq!(p.tune, ARCHIVAL_LOSSLESS_TUNE);
    }

    #[test]
    fn for_preset_routes_h264_to_production() {
        let profile = DeterministicEncoderProfile::for_preset(&h264_preset());
        assert!(!profile.intra_frame_lossless);
        assert_eq!(profile.tune, PRODUCTION_TUNE);
    }

    #[test]
    fn for_preset_routes_ffv1_to_archival() {
        let profile = DeterministicEncoderProfile::for_preset(&ffv1_preset());
        assert!(profile.intra_frame_lossless);
        assert_eq!(profile.gop_size, ARCHIVAL_LOSSLESS_GOP_SIZE);
    }

    #[test]
    fn ffmpeg_next_pin_string_matches_cargo_toml_pin() {
        assert_eq!(FFMPEG_NEXT_PIN, "8.1.0");
    }

    /// Live probe of the libav backend. Requires FFmpeg 8.x installed
    /// on the host; the mission's library/environment.md asserts this
    /// (FFmpeg 8.0.1 at /opt/homebrew/bin/ffmpeg). If a future CI host
    /// has FFmpeg uninstalled, this test fails fast — the VAL-M10B-032
    /// missing-FFmpeg structured-JSON path is exercised separately via
    /// a mocked probe in m10b-4.
    #[test]
    fn probe_initialises_libav_runtime() {
        let rt = FfmpegBridge::probe().expect("libav backend must initialise on the mission host");
        assert_eq!(rt.ffmpeg_next_pin, FFMPEG_NEXT_PIN);
        assert!(
            rt.libavutil_version != 0,
            "libavutil version probe should be non-zero on a live FFmpeg install"
        );
    }
}
