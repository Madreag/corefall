//! M10B default audio base-mix integration tests.
//!
//! Verification command: `cargo test -p cf-replay-export default_audio_mix_from_audio_event_played`
//! (expect: PASS).
//!
//! VAL-M10B-DEFAULT-AUDIO-MIX: default-export audio shows non-silent
//! waveform at the tick-frame offsets corresponding to known
//! `audio.event_played` event ticks; silent elsewhere.
//!
//! VAL-CROSS-013: M10B export audio mix carries M9C audio cues at
//! correct tick offsets (`mg_nest_burst`, `mine_arming_beep`,
//! `electrified_shock_zap`).

use cf_replay::Event;
use cf_replay_export::audio_base_mix::{
    peak_dbfs_at_tick, synthesize_base_mix, synthesize_base_mix_or_silence, synthesize_silent_base_mix,
    NO_AUDIO_BASE_FLOOR_DBFS, PEAK_THRESHOLD_DBFS,
};

fn audio_event(tick: u64, canonical_name: &str, gain: f32) -> Event {
    Event {
        schema_version: cf_replay::EVENT_SCHEMA_VERSION.to_string(),
        run_id: "audio_base_mix_test".into(),
        tick,
        sim_time_ms: tick as f64 / 60.0 * 1000.0,
        event_id: format!("a{tick}_{canonical_name}"),
        category: "audio".into(),
        event_type: "audio.event_played".into(),
        payload: serde_json::json!({
            "canonical_name": canonical_name,
            "bus": "sfx",
            "direction": "here",
            "sequence": 0,
            "gain": gain,
        }),
        parent_event_id: None,
        actor_id: None,
        source_id: None,
        team: None,
        pos: None,
        bbox: None,
        dropped_count: None,
        cosmetic: Some(true),
        asset_ref: None,
        prev_event_hash: None,
        chained_hash_hex: None,
    }
}

#[test]
fn default_audio_mix_from_audio_event_played() {
    // Two audio.event_played events; silence either side.
    let events = vec![
        audio_event(120, "weapon_fired", 1.0),
        audio_event(480, "actor_landed", 0.7),
    ];
    let buffer = synthesize_base_mix(&events, 60, 1_200);
    assert!(!buffer.is_empty());
    for ev in &events {
        let peak = peak_dbfs_at_tick(&buffer, ev.tick, 60, 1);
        assert!(
            peak > PEAK_THRESHOLD_DBFS,
            "audio.event_played @ tick {} must produce peak > {} dBFS (got {} dBFS)",
            ev.tick,
            PEAK_THRESHOLD_DBFS,
            peak
        );
    }
    // Sample a silent tick away from all events.
    let silent = peak_dbfs_at_tick(&buffer, 1_000, 60, 1);
    assert!(
        silent <= PEAK_THRESHOLD_DBFS,
        "silence between events must be ≤ {} dBFS (got {} dBFS)",
        PEAK_THRESHOLD_DBFS,
        silent
    );
}

#[test]
fn default_audio_mix_carries_m9c_cues_at_correct_offsets() {
    let events = vec![
        audio_event(60, "mg_nest_burst", 1.0),
        audio_event(180, "mine_arming_beep", 1.0),
        audio_event(300, "electrified_shock_zap", 1.0),
    ];
    let buffer = synthesize_base_mix(&events, 60, 600);
    for ev in &events {
        let peak = peak_dbfs_at_tick(&buffer, ev.tick, 60, 1);
        assert!(
            peak > PEAK_THRESHOLD_DBFS,
            "M9C cue @ tick {} must be audible (peak={} dBFS)",
            ev.tick,
            peak
        );
    }
}

#[test]
fn default_audio_mix_is_byte_identical_across_runs() {
    let events = vec![
        audio_event(120, "mg_nest_burst", 1.0),
        audio_event(360, "mine_arming_beep", 0.8),
    ];
    let a = synthesize_base_mix(&events, 60, 600);
    let b = synthesize_base_mix(&events, 60, 600);
    assert_eq!(a, b, "mix must be deterministic");
}

/// VAL-M10B-NO-AUDIO-BASE: `--no-audio-base` mutes base SFX/music
/// (peak ≤ -90 dBFS at every cue tick); commentary remains audible
/// (synthesised separately above the muted base).
#[test]
fn no_audio_base_mutes_sfx_music() {
    let events = vec![
        audio_event(60, "mg_nest_burst", 1.0),
        audio_event(180, "mine_arming_beep", 1.0),
        audio_event(300, "electrified_shock_zap", 1.0),
    ];
    // Live mix produces audible peaks.
    let live = synthesize_base_mix(&events, 60, 600);
    for ev in &events {
        let peak = peak_dbfs_at_tick(&live, ev.tick, 60, 1);
        assert!(peak > PEAK_THRESHOLD_DBFS);
    }
    // `synthesize_base_mix_or_silence(no_audio_base=true)` mutes
    // every cue.
    let muted = synthesize_base_mix_or_silence(&events, 60, 600, true);
    for ev in &events {
        let peak = peak_dbfs_at_tick(&muted, ev.tick, 60, 1);
        assert!(
            peak <= NO_AUDIO_BASE_FLOOR_DBFS,
            "--no-audio-base must produce peak ≤ {NO_AUDIO_BASE_FLOOR_DBFS} dBFS at cue tick {} (got {peak} dBFS)",
            ev.tick
        );
    }
    // Buffer length parity so commentary can mix over either output
    // sample-for-sample.
    assert_eq!(live.len(), muted.len());
    // Also exercise the silent helper directly.
    let silent = synthesize_silent_base_mix(60, 600);
    assert!(
        silent.iter().all(|s| *s == 0.0),
        "synthesize_silent_base_mix must produce zeros"
    );
}
