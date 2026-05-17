//! M10B commentary integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export commentary`
//! (expect: 4 typed errors + tick-aligned + drift_max_frames ≤ 1).
//!
//! VAL-M10B-011 — typed-error rejection across 4 malformed inputs.
//! VAL-M10B-027 — tick-aligned mix; ≤ 1 frame drift over 30 minutes.

use cf_replay_export::commentary::{
    max_drift_frames, sample_offsets_for_clip, CommentaryError, CommentaryScript, VoiceClip,
    CAPTION_LOC_BUNDLE_SIZE_CAP, COMMENTARY_CHANNELS, COMMENTARY_SAMPLE_RATE_HZ,
};

#[test]
fn commentary_typed_error_missing_audio_path() {
    let bad = r#"(clips: [(start_tick: 0, end_tick: 100)])"#;
    let err = CommentaryScript::from_ron_str(bad).expect_err("missing audio_path must error");
    assert!(matches!(err, CommentaryError::MissingAudioPath { clip_index: 0 }));
}

#[test]
fn commentary_typed_error_negative_tick_range() {
    let bad = r#"(clips: [(start_tick: -50, end_tick: 10, audio_path: "x.ogg")])"#;
    let err = CommentaryScript::from_ron_str(bad).expect_err("negative tick must error");
    assert!(matches!(err, CommentaryError::NegativeTickRange { .. }));
}

#[test]
fn commentary_typed_error_nonexistent_audio_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let path = tmp.path().join("s.commentary.ron");
    std::fs::write(
        &path,
        r#"(clips: [(start_tick: 0, end_tick: 100, audio_path: "ghost.ogg")])"#,
    )
    .unwrap();
    let err = CommentaryScript::load(&path).expect_err("missing OGG must error");
    assert!(matches!(err, CommentaryError::AudioPathNotFound { .. }));
}

#[test]
fn commentary_typed_error_caption_too_long() {
    let long = "x".repeat(CAPTION_LOC_BUNDLE_SIZE_CAP + 5);
    let body = format!(
        r#"(clips: [(start_tick: 0, end_tick: 100, audio_path: "v.ogg", caption_text: "{long}")])"#
    );
    let err = CommentaryScript::from_ron_str(&body).expect_err("oversized caption must error");
    assert!(matches!(err, CommentaryError::CaptionTooLong { .. }));
}

#[test]
fn commentary_tick_aligned_mix_at_48_khz_stereo() {
    let clip = VoiceClip {
        start_tick: 300,
        end_tick: 900,
        audio_path: "vo.ogg".into(),
        caption_template_id: None,
        caption_args: Vec::new(),
    };
    assert_eq!(COMMENTARY_SAMPLE_RATE_HZ, 48_000);
    assert_eq!(COMMENTARY_CHANNELS, 2);
    let (start, end) = sample_offsets_for_clip(&clip, 60);
    // 300 ticks * (48000 / 60) = 240_000; 900 * 800 = 720_000
    assert_eq!(start, 240_000);
    assert_eq!(end, 720_000);
}

#[test]
fn commentary_drift_max_frames_under_one_over_30_min() {
    // 30 min @ 60 Hz tick rate = 108_000 ticks. Ten 1-min clips spaced
    // back-to-back exercise the cumulative drift path.
    let clips: Vec<VoiceClip> = (0..10u64)
        .map(|i| VoiceClip {
            start_tick: i * 10_800 + 17,
            end_tick: (i + 1) * 10_800 + 17,
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
    assert!(drift <= 1.0, "drift_max_frames={drift} must be ≤ 1");
}
