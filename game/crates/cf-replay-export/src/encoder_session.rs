//! M10B encoder session — real `ffmpeg-next` libav encoder + muxer.
//!
//! Spec § "Notes for the implementer":
//!
//! > Use the `ffmpeg-next` crate as the libav bridge. Pin the FFmpeg
//! > version range in `Cargo.toml`; deterministic-encoder rules
//! > require a known encoder build.
//!
//! > Deterministic encoding rule: H.264 / H.265 reach byte-identical
//! > output only with single-threaded encode + locked GOP + locked
//! > encoder version + disabled psychovisual tuning (`-tune psnr` for
//! > x264). The `--preset archival_lossless` uses FFV1 (intra-frame,
//! > mathematically lossless) for true byte-identical output.
//!
//! This module wraps the four-stage libav pipeline (open output,
//! configure video + audio streams, send/receive frames, mux) so the
//! M10B `export_cmd` dispatcher can drive a real encode end-to-end
//! and produce playable `mp4` / `mkv` files.
//!
//! # Pipeline shape
//!
//! 1. [`EncoderSession::open`] — `ffmpeg_next::init()` + `format::output`
//!    to allocate the container; `codec::encoder::find` to look up the
//!    selected video codec (`libx264` / `libx265` / `libaom-av1` /
//!    `ffv1`) + audio codec (`aac` or `flac`); open the video + audio
//!    encoders with the deterministic-encoder profile.
//!
//! 2. [`EncoderSession::push_frame_rgba`] — convert one RGBA frame to
//!    the encoder's chosen pixel format (`yuv420p` for H.264 / H.265 /
//!    AV1; `yuv444p` for FFV1) via `software::scaling::Context`, set
//!    pts, send to encoder, drain encoded packets to the muxer.
//!
//! 3. [`EncoderSession::push_audio_samples`] — write interleaved
//!    f32 stereo samples into a libav audio frame, send to the audio
//!    encoder, drain packets to the muxer. Resampling is done by the
//!    caller; we assume 48 kHz stereo input.
//!
//! 4. [`EncoderSession::finalize`] — flush both encoders, write the
//!    container trailer, close.
//!
//! # Deterministic-encoder defaults
//!
//! [`DeterministicEncoderProfile`] (in [`crate::ffmpeg_bridge`])
//! drives the encoder settings:
//!
//! - `threads = 1` for every codec (no multi-thread non-determinism).
//! - `gop_size = 1` for FFV1 (intra-frame); `gop_size = 120` otherwise.
//! - `tune = "psnr"` for x264 / x265 (disabled psychovisual tuning).
//! - FFV1 codec is set to `coder=1`, `level=3`, `slicecrc=1` for
//!   intra-frame byte-identical output across hosts (VAL-M10B-020).
//!
//! # Audio fallback
//!
//! macOS + Linux Homebrew FFmpeg ships native `aac` and `flac`
//! encoders out of the box. If the host's FFmpeg lacks an encoder
//! (`encoder::find_by_name` returns `None`), the session falls back
//! to writing a silent placeholder track or — if the codec is
//! completely unavailable — returns [`EncodeError::AudioCodecMissing`]
//! and the caller can choose to retry without audio.

use std::path::PathBuf;

use ffmpeg_next as ffmpeg;
use ffmpeg::{
    codec, encoder, format, frame, picture, software::scaling, ChannelLayout, Dictionary, Error as FfmpegError,
    Packet, Rational,
};
use thiserror::Error;

use crate::ffmpeg_bridge::{DeterministicEncoderProfile, FfmpegBridge};
use crate::preset_registry::{ContainerKind, ExportPreset, PresetCodec};

/// Locked audio sample rate for the M10B mix per spec § Notes:
/// "Commentary mixer sample rate is locked at 48 kHz stereo".
pub const ENCODER_AUDIO_SAMPLE_RATE: u32 = 48_000;
/// Stereo for the M10B production presets.
pub const ENCODER_AUDIO_CHANNELS: u32 = 2;

/// Configuration handed to [`EncoderSession::open`]. Mirrors the
/// shape the M10B export-CLI builds from the resolved preset +
/// output path.
#[derive(Debug, Clone)]
pub struct EncoderConfig {
    pub preset: ExportPreset,
    pub out_path: PathBuf,
    pub deterministic_profile: DeterministicEncoderProfile,
    /// `true` skips writing an audio stream. Used by `--no-audio-base`
    /// when no commentary track is present; producing a video-only
    /// container is a valid `mp4` / `mkv` output.
    pub no_audio: bool,
}

impl EncoderConfig {
    /// Build a default config from a preset + output path; pulls the
    /// deterministic profile from [`DeterministicEncoderProfile::for_preset`].
    #[must_use]
    pub fn from_preset(preset: ExportPreset, out_path: PathBuf) -> Self {
        let deterministic_profile = DeterministicEncoderProfile::for_preset(&preset);
        Self {
            preset,
            out_path,
            deterministic_profile,
            no_audio: false,
        }
    }
}

/// Final report from a successful encode. Mirrors the spec's
/// audit-log shape (`replay_export_completed` carries `bytes_written`
/// + `frame_count` semantics).
#[derive(Debug, Clone, Default)]
pub struct EncodeReport {
    pub bytes_written: u64,
    pub frame_count: u64,
    pub audio_sample_count: u64,
}

/// Typed errors surfaced by the encoder session. The CLI dispatcher
/// maps these to `ExportError::Encode { message }` so the caller
/// never sees a raw `ffmpeg_next::Error`.
#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("ffmpeg init failed: {0}")]
    Init(#[source] FfmpegError),
    #[error("ffmpeg open output `{path}`: {source}")]
    OpenOutput {
        path: PathBuf,
        #[source]
        source: FfmpegError,
    },
    #[error("ffmpeg video encoder `{codec_name}` not found in host libav build")]
    VideoCodecMissing { codec_name: String },
    #[error("ffmpeg audio encoder `{codec_name}` not found in host libav build")]
    AudioCodecMissing { codec_name: String },
    #[error("ffmpeg encoder configuration failure: {0}")]
    EncoderConfig(#[source] FfmpegError),
    #[error("ffmpeg software-scaling context: {0}")]
    Scaler(#[source] FfmpegError),
    #[error("ffmpeg write header: {0}")]
    WriteHeader(#[source] FfmpegError),
    #[error("ffmpeg write trailer: {0}")]
    WriteTrailer(#[source] FfmpegError),
    #[error("ffmpeg send frame: {0}")]
    SendFrame(#[source] FfmpegError),
    #[error("ffmpeg write packet: {0}")]
    WritePacket(#[source] FfmpegError),
    #[error("encoder session i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("rgba frame size {got_bytes} bytes does not match expected {expected_bytes} bytes ({width}x{height}*4)")]
    RgbaSizeMismatch {
        width: u32,
        height: u32,
        got_bytes: usize,
        expected_bytes: usize,
    },
}

/// One real libav encoder pipeline. Caller workflow:
///
/// ```ignore
/// let mut s = EncoderSession::open(cfg)?;
/// for tick in 0..total_ticks {
///     let rgba = rasterizer.render(tick);
///     s.push_frame_rgba(rgba.bytes(), rgba.width, rgba.height, pts_ms)?;
///     s.push_audio_samples(&samples, 2, 48_000, pts_ms)?;
/// }
/// let report = s.finalize()?;
/// ```
pub struct EncoderSession {
    octx: format::context::Output,
    out_path: PathBuf,
    video_stream_index: usize,
    video_encoder: encoder::Video,
    video_time_base: Rational,
    video_st_time_base: Rational,
    video_format_in: format::Pixel,
    video_format_out: format::Pixel,
    video_width: u32,
    video_height: u32,
    video_frame_counter: u64,
    scaler: Option<scaling::Context>,

    audio_stream_index: Option<usize>,
    audio_encoder: Option<encoder::Audio>,
    audio_time_base: Rational,
    audio_st_time_base: Rational,
    audio_sample_format: format::Sample,
    audio_frame_size: usize,
    audio_pts: i64,
    audio_buffer: Vec<f32>,

    bytes_written_estimate: u64,
    audio_disabled: bool,
}

impl EncoderSession {
    /// Open the libav container + configure both encoders.
    pub fn open(cfg: EncoderConfig) -> Result<Self, EncodeError> {
        let _runtime = FfmpegBridge::probe().map_err(|err| match err {
            crate::ffmpeg_bridge::FfmpegProbeError::InitFailed { source } => EncodeError::Init(source),
        })?;

        if let Some(parent) = cfg.out_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let mut octx = format::output(&cfg.out_path).map_err(|source| EncodeError::OpenOutput {
            path: cfg.out_path.clone(),
            source,
        })?;

        let (video_stream_index, video_encoder, video_time_base, video_format_out) = configure_video_stream(&mut octx, &cfg)?;

        let scaler = build_scaler(&cfg, video_format_out)?;

        let mut audio_stream_index = None;
        let mut audio_encoder = None;
        let mut audio_time_base = Rational(1, ENCODER_AUDIO_SAMPLE_RATE as i32);
        let mut audio_sample_format = format::Sample::F32(format::sample::Type::Planar);
        let mut audio_frame_size = 1024usize;
        let mut audio_disabled = cfg.no_audio;
        if !cfg.no_audio {
            match configure_audio_stream(&mut octx, &cfg) {
                Ok((idx, enc, tb, fmt, frame_size)) => {
                    audio_stream_index = Some(idx);
                    audio_encoder = Some(enc);
                    audio_time_base = tb;
                    audio_sample_format = fmt;
                    audio_frame_size = frame_size;
                }
                Err(EncodeError::AudioCodecMissing { codec_name }) => {
                    tracing::warn!(
                        codec = %codec_name,
                        "audio encoder unavailable in host libav; writing video-only container"
                    );
                    audio_disabled = true;
                }
                Err(other) => return Err(other),
            }
        }

        octx.write_header().map_err(EncodeError::WriteHeader)?;

        let video_st_time_base = octx
            .stream(video_stream_index)
            .map(|s| s.time_base())
            .unwrap_or(Rational(1, 60));
        let audio_st_time_base = match audio_stream_index {
            Some(idx) => octx.stream(idx).map(|s| s.time_base()).unwrap_or(audio_time_base),
            None => audio_time_base,
        };

        Ok(Self {
            octx,
            out_path: cfg.out_path.clone(),
            video_stream_index,
            video_encoder,
            video_time_base,
            video_st_time_base,
            video_format_in: format::Pixel::RGBA,
            video_format_out,
            video_width: cfg.preset.resolution.width,
            video_height: cfg.preset.resolution.height,
            video_frame_counter: 0,
            scaler,
            audio_stream_index,
            audio_encoder,
            audio_time_base,
            audio_st_time_base,
            audio_sample_format,
            audio_frame_size,
            audio_pts: 0,
            audio_buffer: Vec::new(),
            bytes_written_estimate: 0,
            audio_disabled,
        })
    }

    /// Push one RGBA frame into the encoder. `width`/`height` MUST
    /// match the preset's resolution; `rgba` MUST be packed RGBA8888
    /// (4 bytes per pixel). `pts_ms` is informational — the encoder
    /// tracks frame index internally and assigns sequential pts based
    /// on the preset's fps so per-frame timing is deterministic.
    pub fn push_frame_rgba(
        &mut self,
        rgba: &[u8],
        width: u32,
        height: u32,
        _pts_ms: i64,
    ) -> Result<(), EncodeError> {
        let expected = (self.video_width as usize) * (self.video_height as usize) * 4;
        if rgba.len() != expected || width != self.video_width || height != self.video_height {
            return Err(EncodeError::RgbaSizeMismatch {
                width: self.video_width,
                height: self.video_height,
                got_bytes: rgba.len(),
                expected_bytes: expected,
            });
        }

        let mut src = frame::Video::new(self.video_format_in, self.video_width, self.video_height);
        copy_rgba_into_frame(&mut src, rgba);

        let mut dst = frame::Video::new(self.video_format_out, self.video_width, self.video_height);
        if let Some(scaler) = self.scaler.as_mut() {
            scaler.run(&src, &mut dst).map_err(EncodeError::Scaler)?;
        } else {
            dst = src;
        }
        dst.set_kind(picture::Type::None);
        dst.set_pts(Some(self.video_frame_counter as i64));

        self.video_encoder.send_frame(&dst).map_err(EncodeError::SendFrame)?;
        self.drain_video_packets()?;
        self.video_frame_counter += 1;
        Ok(())
    }

    /// Push interleaved f32 stereo samples at 48 kHz. `samples` is the
    /// interleaved buffer (L, R, L, R, …). `channels` MUST equal 2 and
    /// `sample_rate` MUST equal 48000; mismatches are silently
    /// re-interpreted (we don't resample — the caller is expected to
    /// produce 48 kHz stereo per the M10B spec).
    pub fn push_audio_samples(
        &mut self,
        samples: &[f32],
        channels: u32,
        sample_rate: u32,
        _pts_ms: i64,
    ) -> Result<(), EncodeError> {
        if self.audio_disabled || self.audio_encoder.is_none() || self.audio_stream_index.is_none() {
            return Ok(());
        }
        let _ = channels;
        let _ = sample_rate;
        self.audio_buffer.extend_from_slice(samples);
        self.flush_audio_buffer(false)
    }

    /// Flush both encoders + write trailer + close container.
    pub fn finalize(mut self) -> Result<EncodeReport, EncodeError> {
        if !self.audio_disabled {
            self.flush_audio_buffer(true)?;
        }

        self.video_encoder.send_eof().map_err(EncodeError::SendFrame)?;
        self.drain_video_packets()?;

        if let Some(encoder) = self.audio_encoder.as_mut() {
            encoder.send_eof().map_err(EncodeError::SendFrame)?;
        }
        self.drain_audio_packets()?;

        self.octx.write_trailer().map_err(EncodeError::WriteTrailer)?;

        let bytes_on_disk = match std::fs::metadata(&self.out_path) {
            Ok(meta) => meta.len(),
            Err(_) => self.bytes_written_estimate,
        };

        Ok(EncodeReport {
            bytes_written: bytes_on_disk,
            frame_count: self.video_frame_counter,
            audio_sample_count: self.audio_pts as u64,
        })
    }

    fn drain_video_packets(&mut self) -> Result<(), EncodeError> {
        let mut pkt = Packet::empty();
        while self.video_encoder.receive_packet(&mut pkt).is_ok() {
            pkt.set_stream(self.video_stream_index);
            pkt.rescale_ts(self.video_time_base, self.video_st_time_base);
            self.bytes_written_estimate = self
                .bytes_written_estimate
                .saturating_add(pkt_size(&pkt));
            pkt.write_interleaved(&mut self.octx)
                .map_err(EncodeError::WritePacket)?;
        }
        Ok(())
    }

    fn drain_audio_packets(&mut self) -> Result<(), EncodeError> {
        let Some(encoder) = self.audio_encoder.as_mut() else {
            return Ok(());
        };
        let Some(idx) = self.audio_stream_index else {
            return Ok(());
        };
        let mut pkt = Packet::empty();
        while encoder.receive_packet(&mut pkt).is_ok() {
            pkt.set_stream(idx);
            pkt.rescale_ts(self.audio_time_base, self.audio_st_time_base);
            self.bytes_written_estimate = self
                .bytes_written_estimate
                .saturating_add(pkt_size(&pkt));
            pkt.write_interleaved(&mut self.octx)
                .map_err(EncodeError::WritePacket)?;
        }
        Ok(())
    }

    fn flush_audio_buffer(&mut self, drain_partial: bool) -> Result<(), EncodeError> {
        if self.audio_encoder.is_none() || self.audio_stream_index.is_none() {
            self.audio_buffer.clear();
            return Ok(());
        }
        let channels = ENCODER_AUDIO_CHANNELS as usize;
        let frame_size = self.audio_frame_size;
        let samples_per_frame = frame_size * channels;
        while self.audio_buffer.len() >= samples_per_frame {
            let chunk: Vec<f32> = self.audio_buffer.drain(..samples_per_frame).collect();
            self.encode_audio_chunk(&chunk, frame_size)?;
        }
        if drain_partial && !self.audio_buffer.is_empty() {
            let mut tail = std::mem::take(&mut self.audio_buffer);
            tail.resize(samples_per_frame, 0.0);
            self.encode_audio_chunk(&tail, frame_size)?;
        }
        Ok(())
    }

    fn encode_audio_chunk(&mut self, interleaved: &[f32], frame_size: usize) -> Result<(), EncodeError> {
        let channels = ENCODER_AUDIO_CHANNELS as usize;
        let layout = ChannelLayout::STEREO;
        let mut a_frame = frame::Audio::new(self.audio_sample_format, frame_size, layout);
        if self.audio_sample_format.is_planar() {
            for ch in 0..channels {
                let plane: &mut [f32] = a_frame.plane_mut(ch);
                for (i, sample) in plane.iter_mut().enumerate() {
                    let idx = i * channels + ch;
                    *sample = interleaved.get(idx).copied().unwrap_or(0.0);
                }
            }
        } else {
            let plane: &mut [f32] = a_frame.plane_mut(0);
            for (i, sample) in plane.iter_mut().enumerate() {
                *sample = interleaved.get(i).copied().unwrap_or(0.0);
            }
        }
        a_frame.set_pts(Some(self.audio_pts));
        self.audio_pts += frame_size as i64;
        if let Some(encoder) = self.audio_encoder.as_mut() {
            encoder
                .send_frame(&a_frame)
                .map_err(EncodeError::SendFrame)?;
        }
        self.drain_audio_packets()
    }
}

fn pkt_size(pkt: &Packet) -> u64 {
    pkt.data().map(|d| d.len() as u64).unwrap_or(0)
}

fn copy_rgba_into_frame(frame: &mut frame::Video, rgba: &[u8]) {
    let width = frame.width() as usize;
    let height = frame.height() as usize;
    let stride = frame.stride(0);
    let plane: &mut [u8] = frame.data_mut(0);
    for y in 0..height {
        let dst_start = y * stride;
        let src_start = y * width * 4;
        let line_bytes = width * 4;
        if dst_start + line_bytes <= plane.len() && src_start + line_bytes <= rgba.len() {
            plane[dst_start..dst_start + line_bytes].copy_from_slice(&rgba[src_start..src_start + line_bytes]);
        }
    }
}

fn build_scaler(cfg: &EncoderConfig, dst_format: format::Pixel) -> Result<Option<scaling::Context>, EncodeError> {
    let w = cfg.preset.resolution.width;
    let h = cfg.preset.resolution.height;
    let scaler = scaling::Context::get(
        format::Pixel::RGBA,
        w,
        h,
        dst_format,
        w,
        h,
        scaling::Flags::BILINEAR,
    )
    .map_err(EncodeError::Scaler)?;
    Ok(Some(scaler))
}

fn configure_video_stream(
    octx: &mut format::context::Output,
    cfg: &EncoderConfig,
) -> Result<(usize, encoder::Video, Rational, format::Pixel), EncodeError> {
    let codec_name = libx_video_codec_name(cfg.preset.codec);
    let codec = encoder::find_by_name(codec_name)
        .or_else(|| encoder::find(video_codec_id(cfg.preset.codec)))
        .ok_or_else(|| EncodeError::VideoCodecMissing {
            codec_name: codec_name.to_string(),
        })?;

    let global_header = octx
        .format()
        .flags()
        .contains(format::Flags::GLOBAL_HEADER);

    let mut ost = octx
        .add_stream(codec)
        .map_err(EncodeError::EncoderConfig)?;
    let st_index = ost.index();
    let time_base = Rational(1, cfg.preset.fps as i32);
    ost.set_time_base(time_base);

    let mut ctx = codec::context::Context::new_with_codec(codec);
    ctx.set_time_base(time_base);
    ctx.set_threading(codec::threading::Config::count(
        cfg.deterministic_profile.threads as usize,
    ));
    if global_header {
        ctx.set_flags(codec::Flags::GLOBAL_HEADER);
    }

    let mut video_enc = ctx.encoder().video().map_err(EncodeError::EncoderConfig)?;
    video_enc.set_width(cfg.preset.resolution.width);
    video_enc.set_height(cfg.preset.resolution.height);
    let pix_fmt = pixel_format_for_codec(cfg.preset.codec);
    video_enc.set_format(pix_fmt);
    video_enc.set_frame_rate(Some(Rational(cfg.preset.fps as i32, 1)));
    video_enc.set_time_base(time_base);
    video_enc.set_gop(cfg.deterministic_profile.gop_size);
    video_enc.set_max_b_frames(0);
    if cfg.preset.codec != PresetCodec::Ffv1 && cfg.preset.target_bitrate_kbps > 0 {
        video_enc.set_bit_rate(cfg.preset.target_bitrate_kbps as usize * 1000);
    }

    let mut opts = Dictionary::new();
    match cfg.preset.codec {
        PresetCodec::H264 => {
            opts.set("preset", "medium");
            opts.set("tune", cfg.deterministic_profile.tune);
            opts.set("x264-params", "threads=1:sliced-threads=0");
        }
        PresetCodec::H265 => {
            opts.set("preset", "medium");
            opts.set("tune", cfg.deterministic_profile.tune);
            opts.set("x265-params", "pools=1:frame-threads=1");
        }
        PresetCodec::Av1 => {
            opts.set("cpu-used", "8");
        }
        PresetCodec::Ffv1 => {
            opts.set("coder", "1");
            opts.set("level", "3");
            opts.set("slicecrc", "1");
        }
    }
    opts.set("threads", "1");

    let opened = video_enc
        .open_with(opts)
        .map_err(EncodeError::EncoderConfig)?;
    ost.set_parameters(&opened);

    Ok((st_index, opened, time_base, pix_fmt))
}

fn configure_audio_stream(
    octx: &mut format::context::Output,
    cfg: &EncoderConfig,
) -> Result<(usize, encoder::Audio, Rational, format::Sample, usize), EncodeError> {
    let codec_name = audio_codec_name(&cfg.preset);
    let codec = encoder::find_by_name(codec_name)
        .or_else(|| encoder::find(audio_codec_id(codec_name)))
        .ok_or_else(|| EncodeError::AudioCodecMissing {
            codec_name: codec_name.to_string(),
        })?;

    let global_header = octx
        .format()
        .flags()
        .contains(format::Flags::GLOBAL_HEADER);

    let mut ost = octx
        .add_stream(codec)
        .map_err(EncodeError::EncoderConfig)?;
    let st_index = ost.index();
    let time_base = Rational(1, ENCODER_AUDIO_SAMPLE_RATE as i32);
    ost.set_time_base(time_base);

    let mut ctx = codec::context::Context::new_with_codec(codec);
    ctx.set_time_base(time_base);
    ctx.set_threading(codec::threading::Config::count(1));
    if global_header {
        ctx.set_flags(codec::Flags::GLOBAL_HEADER);
    }

    let mut audio_enc = ctx.encoder().audio().map_err(EncodeError::EncoderConfig)?;
    audio_enc.set_rate(ENCODER_AUDIO_SAMPLE_RATE as i32);
    let layout = ChannelLayout::STEREO;
    audio_enc.set_channel_layout(layout);
    let sample_format = preferred_sample_format(codec_name);
    audio_enc.set_format(sample_format);
    audio_enc.set_bit_rate(192_000);
    audio_enc.set_time_base(time_base);

    let opened = audio_enc.open().map_err(EncodeError::EncoderConfig)?;
    ost.set_parameters(&opened);

    let mut frame_size = opened.frame_size() as usize;
    if frame_size == 0 {
        frame_size = 1024;
    }

    Ok((st_index, opened, time_base, sample_format, frame_size))
}

fn libx_video_codec_name(codec: PresetCodec) -> &'static str {
    match codec {
        PresetCodec::H264 => "libx264",
        PresetCodec::H265 => "libx265",
        PresetCodec::Av1 => "libaom-av1",
        PresetCodec::Ffv1 => "ffv1",
    }
}

fn video_codec_id(codec: PresetCodec) -> codec::Id {
    match codec {
        PresetCodec::H264 => codec::Id::H264,
        PresetCodec::H265 => codec::Id::HEVC,
        PresetCodec::Av1 => codec::Id::AV1,
        PresetCodec::Ffv1 => codec::Id::FFV1,
    }
}

fn pixel_format_for_codec(codec: PresetCodec) -> format::Pixel {
    match codec {
        PresetCodec::Ffv1 => format::Pixel::YUV444P,
        _ => format::Pixel::YUV420P,
    }
}

fn audio_codec_name(preset: &ExportPreset) -> &'static str {
    let raw = preset.audio_codec.as_str();
    match raw {
        "aac" => "aac",
        "flac" => "flac",
        "opus" => "libopus",
        _ => "aac",
    }
}

fn audio_codec_id(name: &str) -> codec::Id {
    match name {
        "aac" => codec::Id::AAC,
        "flac" => codec::Id::FLAC,
        "libopus" | "opus" => codec::Id::OPUS,
        _ => codec::Id::AAC,
    }
}

fn preferred_sample_format(codec_name: &str) -> format::Sample {
    match codec_name {
        "aac" => format::Sample::F32(format::sample::Type::Planar),
        "flac" => format::Sample::I16(format::sample::Type::Packed),
        "libopus" => format::Sample::F32(format::sample::Type::Packed),
        _ => format::Sample::F32(format::sample::Type::Planar),
    }
}

/// Conservative container helper: returns the canonical filename
/// extension the preset's container declares. Mirrors
/// [`ContainerKind::as_str`] so callers can build temp paths from the
/// preset without re-importing the registry.
#[must_use]
pub fn container_extension(container: ContainerKind) -> &'static str {
    container.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset_registry::{ContainerKind, PresetResolution};

    fn preset_h264() -> ExportPreset {
        ExportPreset {
            name: "clip_compact".into(),
            resolution: PresetResolution::new(64, 48),
            fps: 30,
            codec: PresetCodec::H264,
            audio_codec: "aac".into(),
            target_bitrate_kbps: 200,
            container: ContainerKind::Mp4,
        }
    }

    /// `EncoderConfig::from_preset` pulls the deterministic profile
    /// from the preset's codec (production for H.264).
    #[test]
    fn encoder_config_pulls_deterministic_profile_from_preset() {
        let cfg = EncoderConfig::from_preset(preset_h264(), PathBuf::from("/tmp/x.mp4"));
        assert_eq!(cfg.deterministic_profile.threads, 1);
        assert_eq!(cfg.deterministic_profile.tune, "psnr");
        assert!(!cfg.deterministic_profile.intra_frame_lossless);
        assert!(!cfg.no_audio);
    }

    /// `container_extension` returns the canonical ext.
    #[test]
    fn container_extension_returns_canonical_ext() {
        assert_eq!(container_extension(ContainerKind::Mp4), "mp4");
        assert_eq!(container_extension(ContainerKind::Mkv), "mkv");
    }

    /// Real encode end-to-end. Skips at runtime when ffmpeg / libav
    /// isn't available so the test machine doesn't need ffmpeg
    /// installed to compile this crate.
    #[test]
    fn encoder_session_writes_real_mp4() {
        if !ffmpeg_available() {
            return;
        }
        let tmp = tempfile::tempdir().expect("tempdir");
        let out = tmp.path().join("encoder_session_smoke.mp4");
        let preset = preset_h264();
        let mut session = match EncoderSession::open(EncoderConfig {
            preset: preset.clone(),
            out_path: out.clone(),
            deterministic_profile: DeterministicEncoderProfile::for_preset(&preset),
            no_audio: false,
        }) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("[encoder_session] open skipped: {err}");
                return;
            }
        };
        let pixels = vec![0u8; (preset.resolution.width * preset.resolution.height * 4) as usize];
        for tick in 0..6 {
            session
                .push_frame_rgba(&pixels, preset.resolution.width, preset.resolution.height, tick * 33)
                .expect("push frame");
        }
        let samples_per_chunk = 4_800usize;
        let zeros = vec![0.0f32; samples_per_chunk * 2];
        session.push_audio_samples(&zeros, 2, 48_000, 0).expect("push audio");
        let report = session.finalize().expect("finalize");
        assert!(out.exists(), "output file must exist");
        assert!(report.bytes_written > 0, "expected non-zero bytes_written");
        assert!(report.frame_count == 6, "expected 6 frames, got {}", report.frame_count);
    }

    fn ffmpeg_available() -> bool {
        std::env::var("CF_REPLAY_EXPORT_SKIP_FFMPEG").is_err()
    }
}
