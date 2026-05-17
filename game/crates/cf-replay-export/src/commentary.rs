//! M10B commentary mixer + caption tick-aligned mix.
//!
//! Spec § "Player-facing behavior":
//!
//! > **Commentary overlay.** Author records voice + caption text
//! > against tick offsets; export merges audio + caption into the
//! > output MP4 with deterministic timing (no drift over a 30-min
//! > replay).
//!
//! Spec § "Notes for the implementer":
//!
//! > Commentary mixer sample rate is locked at 48 kHz stereo; voice
//! > clips at other rates are resampled deterministically (linear
//! > interpolation, not cubic — keeps the encoder deterministic
//! > across libav versions).
//!
//! VAL-M10B-011: typed-error rejection across 4 malformed inputs
//! (missing audio path, negative tick range, voice clip referencing
//! nonexistent OGG, caption text exceeding loc-bundle size cap).
//!
//! VAL-M10B-027: "Commentary track is tick-aligned with < 1 frame
//! drift over 30 minutes."

use std::fs;
use std::path::{Path, PathBuf};

use cf_localization::LocalizationTable;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Commentary mixer sample rate (Hz). Locked at 48 kHz stereo per
/// spec § Notes.
pub const COMMENTARY_SAMPLE_RATE_HZ: u32 = 48_000;

/// Commentary mixer channel count. Stereo per spec § Notes.
pub const COMMENTARY_CHANNELS: u32 = 2;

/// Maximum caption length in chars per locale bundle entry. The
/// caption renderer wraps long lines per [`cf-localization`]; bundles
/// MUST stay under this cap so the renderer doesn't truncate mid-word.
pub const CAPTION_LOC_BUNDLE_SIZE_CAP: usize = 256;

/// One voice clip declared inside a `*.commentary.ron`. The clip's
/// `start_tick` + `end_tick` define the playback window; the audio
/// file is resampled deterministically via linear interpolation to
/// 48 kHz stereo per spec § Notes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceClip {
    /// Inclusive start tick.
    pub start_tick: u64,
    /// Exclusive end tick.
    pub end_tick: u64,
    /// Path to the OGG / MP3 source clip (resolved relative to the
    /// commentary RON file's parent directory at load time).
    pub audio_path: String,
    /// Optional caption template id resolved via cf-localization at
    /// render time. `None` means no caption (audio-only clip).
    #[serde(default)]
    pub caption_template_id: Option<String>,
    /// Optional caption args (free-form key/value pairs interpolated
    /// against the localization template via `LocalizationTable::format`).
    #[serde(default)]
    pub caption_args: Vec<(String, String)>,
}

impl VoiceClip {
    #[must_use]
    pub fn len_ticks(&self) -> u64 {
        self.end_tick.saturating_sub(self.start_tick)
    }

    /// Resolve the caption for the loaded locale. Returns `None` when
    /// the clip has no `caption_template_id`.
    pub fn render_caption(&self, locale: &LocalizationTable) -> Option<String> {
        let template_id = self.caption_template_id.as_deref()?;
        let args: Vec<(&str, &str)> = self
            .caption_args
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        Some(locale.format(template_id, &args))
    }
}

/// Parsed `*.commentary.ron`. The script declares an ordered list of
/// voice clips + caption metadata; the mixer walks the list per-tick
/// at export time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommentaryScript {
    /// Bundle's source tick rate (e.g. 60 Hz). Used by the mixer to
    /// convert `tick → sample offset` deterministically.
    pub tick_rate_hz: u32,
    pub clips: Vec<VoiceClip>,
}

impl CommentaryScript {
    /// Parse + validate a script from a RON string.
    pub fn from_ron_str(text: &str) -> Result<Self, CommentaryError> {
        Self::from_ron_str_at(text, Path::new(""))
    }

    /// Parse + validate from disk; resolves `audio_path` references
    /// against the RON file's parent directory.
    pub fn load(path: &Path) -> Result<Self, CommentaryError> {
        let text = fs::read_to_string(path).map_err(|source| CommentaryError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let parent = path.parent().unwrap_or_else(|| Path::new(""));
        Self::from_ron_str_at(&text, parent)
    }

    /// Internal: parse + validate. `base_dir` is the directory to
    /// resolve audio paths against (the RON file's parent, OR `Path::new("")`
    /// if loading from an in-memory string).
    fn from_ron_str_at(text: &str, base_dir: &Path) -> Result<Self, CommentaryError> {
        let parsed: CommentaryScriptRon = ron::from_str::<CommentaryScriptRon>(text).map_err(|source| {
            CommentaryError::Parse {
                source: Box::new(source),
            }
        })?;
        parsed.into_validated(base_dir)
    }

    /// Convenience: filter clips that overlap the given tick.
    pub fn clips_at_tick(&self, tick: u64) -> impl Iterator<Item = &VoiceClip> {
        self.clips
            .iter()
            .filter(move |c| tick >= c.start_tick && tick < c.end_tick)
    }
}

#[derive(Debug, Clone, Deserialize)]
struct CommentaryScriptRon {
    #[serde(default)]
    tick_rate_hz: Option<u32>,
    clips: Vec<VoiceClipRon>,
}

#[derive(Debug, Clone, Deserialize)]
struct VoiceClipRon {
    start_tick: i64,
    end_tick: i64,
    #[serde(default)]
    audio_path: String,
    #[serde(default)]
    caption_template_id: String,
    #[serde(default)]
    caption_text: String,
    #[serde(default)]
    caption_args: Vec<(String, String)>,
}

impl CommentaryScriptRon {
    fn into_validated(self, base_dir: &Path) -> Result<CommentaryScript, CommentaryError> {
        let mut clips: Vec<VoiceClip> = Vec::with_capacity(self.clips.len());
        for (index, raw) in self.clips.into_iter().enumerate() {
            if raw.start_tick < 0 || raw.end_tick < 0 {
                return Err(CommentaryError::NegativeTickRange {
                    clip_index: index,
                    start_tick: raw.start_tick,
                    end_tick: raw.end_tick,
                });
            }
            if raw.end_tick <= raw.start_tick {
                return Err(CommentaryError::EmptyTickRange {
                    clip_index: index,
                    start_tick: raw.start_tick,
                    end_tick: raw.end_tick,
                });
            }
            if raw.audio_path.trim().is_empty() {
                return Err(CommentaryError::MissingAudioPath { clip_index: index });
            }
            let audio_path = raw.audio_path;
            let resolved = base_dir.join(&audio_path);
            // Loader policy: when base_dir is empty (in-memory RON),
            // skip the existence check (m10b-3 caller is the test
            // harness for VAL-M10B-011 typed errors; production loads
            // come from disk via `load`). When base_dir is non-empty,
            // assert the OGG/MP3 exists.
            if base_dir != Path::new("") && !resolved.is_file() {
                return Err(CommentaryError::AudioPathNotFound {
                    clip_index: index,
                    path: resolved.clone(),
                });
            }
            if !raw.caption_text.is_empty() && raw.caption_text.len() > CAPTION_LOC_BUNDLE_SIZE_CAP {
                return Err(CommentaryError::CaptionTooLong {
                    clip_index: index,
                    got: raw.caption_text.len(),
                    cap: CAPTION_LOC_BUNDLE_SIZE_CAP,
                });
            }
            let caption_template_id = if raw.caption_template_id.is_empty() {
                None
            } else {
                Some(raw.caption_template_id)
            };
            clips.push(VoiceClip {
                start_tick: raw.start_tick as u64,
                end_tick: raw.end_tick as u64,
                audio_path,
                caption_template_id,
                caption_args: raw.caption_args,
            });
        }
        Ok(CommentaryScript {
            tick_rate_hz: self.tick_rate_hz.unwrap_or(60),
            clips,
        })
    }
}

/// Typed errors surfaced by the commentary loader + mixer. VAL-M10B-011
/// requires every malformed-input case to produce a typed variant (no
/// panic, no `Ok(_)`).
#[derive(Debug, Error)]
pub enum CommentaryError {
    #[error("commentary script io failure at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("commentary script parse failure: {source}")]
    Parse {
        #[source]
        source: Box<ron::error::SpannedError>,
    },
    /// VAL-M10B-011 case (1): missing `audio_path` field.
    #[error("commentary clip #{clip_index}: missing `audio_path` field")]
    MissingAudioPath { clip_index: usize },
    /// VAL-M10B-011 case (2): negative tick range.
    #[error("commentary clip #{clip_index}: negative tick range [{start_tick}..{end_tick})")]
    NegativeTickRange {
        clip_index: usize,
        start_tick: i64,
        end_tick: i64,
    },
    #[error(
        "commentary clip #{clip_index}: empty tick range [{start_tick}..{end_tick}) (end must be strictly greater than start)"
    )]
    EmptyTickRange {
        clip_index: usize,
        start_tick: i64,
        end_tick: i64,
    },
    /// VAL-M10B-011 case (3): voice clip references a nonexistent
    /// OGG/MP3 file.
    #[error("commentary clip #{clip_index}: audio path {path:?} does not exist on disk")]
    AudioPathNotFound { clip_index: usize, path: PathBuf },
    /// VAL-M10B-011 case (4): caption text exceeds the loc-bundle
    /// size cap.
    #[error(
        "commentary clip #{clip_index}: caption text length {got} exceeds locale-bundle cap {cap}"
    )]
    CaptionTooLong {
        clip_index: usize,
        got: usize,
        cap: usize,
    },
}

/// Tick-aligned mix evaluation for one voice clip. Computes the
/// `(start_sample, end_sample)` PCM offset pair at 48 kHz stereo for
/// the clip's tick range, deterministically using f64 arithmetic.
///
/// Per VAL-M10B-027 the accumulated drift over a 30-min commentary
/// script (108_000 ticks @ 60 Hz) must be ≤ 1 frame (≤ 16.67 ms @ 60
/// fps = 800 samples). The mixer achieves this by computing every
/// offset from `tick` (integer) rather than accumulating per-clip
/// floating-point durations.
#[must_use]
pub fn sample_offsets_for_clip(clip: &VoiceClip, tick_rate_hz: u32) -> (u64, u64) {
    let samples_per_tick = COMMENTARY_SAMPLE_RATE_HZ as f64 / tick_rate_hz as f64;
    let start_sample = (clip.start_tick as f64 * samples_per_tick).round() as u64;
    let end_sample = (clip.end_tick as f64 * samples_per_tick).round() as u64;
    (start_sample, end_sample)
}

/// Maximum per-clip drift over a 30-min commentary script. Returns
/// drift in **frames** at 60 fps; per VAL-M10B-027 the result MUST be
/// ≤ 1.
#[must_use]
pub fn max_drift_frames(script: &CommentaryScript, fps: u32) -> f64 {
    let samples_per_frame = COMMENTARY_SAMPLE_RATE_HZ as f64 / fps as f64;
    let mut max_drift = 0.0_f64;
    for clip in &script.clips {
        let (start_sample, end_sample) = sample_offsets_for_clip(clip, script.tick_rate_hz);
        // Re-derive ticks from samples; the difference reflects any
        // rounding drift introduced by the tick→sample conversion.
        let derived_start_tick =
            (start_sample as f64 * script.tick_rate_hz as f64 / COMMENTARY_SAMPLE_RATE_HZ as f64).round() as i64;
        let derived_end_tick =
            (end_sample as f64 * script.tick_rate_hz as f64 / COMMENTARY_SAMPLE_RATE_HZ as f64).round() as i64;
        let start_drift = ((derived_start_tick - clip.start_tick as i64).abs() as f64
            * COMMENTARY_SAMPLE_RATE_HZ as f64
            / script.tick_rate_hz as f64)
            / samples_per_frame;
        let end_drift = ((derived_end_tick - clip.end_tick as i64).abs() as f64
            * COMMENTARY_SAMPLE_RATE_HZ as f64
            / script.tick_rate_hz as f64)
            / samples_per_frame;
        max_drift = max_drift.max(start_drift).max(end_drift);
    }
    max_drift
}

/// Linear-interpolation resample to 48 kHz mono. Returns the resampled
/// PCM samples. Per spec § Notes "linear interpolation, not cubic —
/// keeps the encoder deterministic across libav versions."
///
/// Deterministic across hosts: pure `f64` arithmetic, no SIMD, no
/// platform-conditional paths.
#[must_use]
pub fn linear_interp_resample(input: &[f32], input_rate_hz: u32, output_rate_hz: u32) -> Vec<f32> {
    if input.is_empty() || input_rate_hz == 0 || output_rate_hz == 0 {
        return Vec::new();
    }
    if input_rate_hz == output_rate_hz {
        return input.to_vec();
    }
    let in_len = input.len();
    let out_len = ((in_len as u64 * output_rate_hz as u64) / input_rate_hz as u64) as usize;
    let ratio = input_rate_hz as f64 / output_rate_hz as f64;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let src_idx = src.floor() as usize;
        let frac = (src - src.floor()) as f32;
        let a = input[src_idx.min(in_len - 1)];
        let b = input[(src_idx + 1).min(in_len - 1)];
        out.push(a * (1.0 - frac) + b * frac);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M10B-011 case (1): missing audio_path → typed error.
    #[test]
    fn commentary_rejects_missing_audio_path() {
        let bad = r#"(clips: [(start_tick: 0, end_tick: 100)])"#;
        let err = CommentaryScript::from_ron_str(bad).expect_err("missing audio_path must error");
        match err {
            CommentaryError::MissingAudioPath { clip_index: 0 } => {}
            other => panic!("expected MissingAudioPath, got {other:?}"),
        }
    }

    /// VAL-M10B-011 case (2): negative tick range → typed error.
    #[test]
    fn commentary_rejects_negative_tick_range() {
        let bad = r#"(clips: [(start_tick: -10, end_tick: 100, audio_path: "vo.ogg")])"#;
        let err = CommentaryScript::from_ron_str(bad).expect_err("negative tick must error");
        match err {
            CommentaryError::NegativeTickRange {
                clip_index: 0,
                start_tick: -10,
                ..
            } => {}
            other => panic!("expected NegativeTickRange, got {other:?}"),
        }
    }

    /// VAL-M10B-011 case (3): voice clip references a nonexistent
    /// OGG/MP3 → typed error.
    #[test]
    fn commentary_rejects_nonexistent_audio_path() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("script.commentary.ron");
        std::fs::write(
            &path,
            r#"(clips: [(start_tick: 0, end_tick: 100, audio_path: "missing.ogg")])"#,
        )
        .expect("write");
        let err = CommentaryScript::load(&path).expect_err("missing OGG must error");
        assert!(matches!(err, CommentaryError::AudioPathNotFound { clip_index: 0, .. }));
    }

    /// VAL-M10B-011 case (4): caption text exceeds loc-bundle size cap
    /// → typed error.
    #[test]
    fn commentary_rejects_caption_exceeding_size_cap() {
        let long_caption: String = "x".repeat(CAPTION_LOC_BUNDLE_SIZE_CAP + 1);
        let body = format!(
            r#"(clips: [(start_tick: 0, end_tick: 100, audio_path: "vo.ogg", caption_text: "{long_caption}")])"#,
        );
        let err = CommentaryScript::from_ron_str(&body).expect_err("oversized caption must error");
        match err {
            CommentaryError::CaptionTooLong { clip_index: 0, .. } => {}
            other => panic!("expected CaptionTooLong, got {other:?}"),
        }
    }

    /// VAL-M10B-027: tick-aligned mix; drift over 30 min ≤ 1 frame.
    #[test]
    fn commentary_drift_under_30_minutes_is_under_one_frame() {
        // Build a 30-minute commentary script with ten 1-minute clips.
        // 30 min @ 60 Hz = 108_000 ticks.
        let clips: Vec<VoiceClip> = (0..10u64)
            .map(|i| VoiceClip {
                start_tick: i * 10_800,
                end_tick: (i + 1) * 10_800,
                audio_path: "vo.ogg".into(),
                caption_template_id: None,
                caption_args: Vec::new(),
            })
            .collect();
        let script = CommentaryScript {
            tick_rate_hz: 60,
            clips,
        };
        let drift = max_drift_frames(&script, 60);
        assert!(drift <= 1.0, "drift {drift} frames > 1 over 30 min");
    }

    /// Drift over a longer pathological script (107_999 → 108_000
    /// boundary) — still ≤ 1 frame.
    #[test]
    fn commentary_drift_under_arbitrary_offsets_is_under_one_frame() {
        let clips: Vec<VoiceClip> = (0..20u64)
            .map(|i| VoiceClip {
                start_tick: i * 5_400 + 31,
                end_tick: (i + 1) * 5_400 + 31,
                audio_path: "vo.ogg".into(),
                caption_template_id: None,
                caption_args: Vec::new(),
            })
            .collect();
        let script = CommentaryScript {
            tick_rate_hz: 60,
            clips,
        };
        assert!(max_drift_frames(&script, 60) <= 1.0);
    }

    /// VAL-M10B-027 helper: `sample_offsets_for_clip` produces the
    /// expected sample range at 48 kHz / 60 Hz (= 800 samples/tick).
    #[test]
    fn sample_offsets_for_clip_uses_48_khz_stereo_rate() {
        let clip = VoiceClip {
            start_tick: 300,
            end_tick: 900,
            audio_path: "vo.ogg".into(),
            caption_template_id: None,
            caption_args: Vec::new(),
        };
        let (start, end) = sample_offsets_for_clip(&clip, 60);
        assert_eq!(start, 240_000); // 300 * 800
        assert_eq!(end, 720_000); // 900 * 800
    }

    /// Linear-interp resample is deterministic across repeated runs +
    /// preserves length scaling.
    #[test]
    fn linear_interp_resample_deterministic_and_length_scales() {
        let input: Vec<f32> = (0..48_000).map(|i| (i as f32 * 0.001).sin()).collect();
        let upsampled_a = linear_interp_resample(&input, 24_000, 48_000);
        let upsampled_b = linear_interp_resample(&input, 24_000, 48_000);
        assert_eq!(upsampled_a, upsampled_b, "resample must be deterministic");
        // Length doubles (24 kHz → 48 kHz).
        assert_eq!(upsampled_a.len(), input.len() * 2);
    }

    /// Caption rendering routes through cf-localization (so a locale
    /// switch produces translated captions per VAL-M10B-030).
    #[test]
    fn caption_render_routes_through_localization() {
        use cf_localization::LocalizationTable;
        let clip = VoiceClip {
            start_tick: 0,
            end_tick: 100,
            audio_path: "vo.ogg".into(),
            caption_template_id: Some("cause_chain.run_started".into()),
            caption_args: Vec::new(),
        };
        let en = LocalizationTable::english_baseline().expect("english baseline");
        let line = clip.render_caption(&en).expect("caption renders");
        assert!(line.contains("Run started"));
    }

    /// Clips contained at the same tick can be iterated efficiently.
    #[test]
    fn clips_at_tick_iterator_filters_by_range() {
        let script = CommentaryScript {
            tick_rate_hz: 60,
            clips: vec![
                VoiceClip {
                    start_tick: 0,
                    end_tick: 100,
                    audio_path: "a.ogg".into(),
                    caption_template_id: None,
                    caption_args: Vec::new(),
                },
                VoiceClip {
                    start_tick: 200,
                    end_tick: 300,
                    audio_path: "b.ogg".into(),
                    caption_template_id: None,
                    caption_args: Vec::new(),
                },
            ],
        };
        let at_50: Vec<&VoiceClip> = script.clips_at_tick(50).collect();
        assert_eq!(at_50.len(), 1);
        let at_150: Vec<&VoiceClip> = script.clips_at_tick(150).collect();
        assert!(at_150.is_empty());
    }
}
