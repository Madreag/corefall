//! M10B default audio base mix.
//!
//! Spec § "Notes for the implementer":
//!
//! > Audio mix in export: by default mixes `cf-audio` runtime SFX +
//! > voice + music tracks captured in the run bundle's
//! > `audio.event_played` events (M12A schema). Commentary track adds
//! > on top of base mix; user can disable base mix with
//! > `--no-audio-base` for clean commentary-only export.
//!
//! VAL-M10B-DEFAULT-AUDIO-MIX: "By default (no `--no-audio-base`),
//! the exported MP4's audio track is the deterministic mix of the
//! bundle's `audio.event_played` events (M12A schema) through the
//! base audio mixer."
//!
//! VAL-CROSS-013: "The output MP4's audio track from an
//! `m9c_full_strongpoint` export MUST include audible waveforms for
//! M9C audio cues (`mg_nest_burst`, `mine_arming_beep`,
//! `electrified_shock_zap`) when those `audio.event_played` events
//! appear in the bundle."
//!
//! The base mixer is deterministic + headless: every
//! `audio.event_played` event becomes a short envelope-modulated sine
//! wave at a cue-specific frequency. The waveform is mixed into a
//! 48 kHz stereo PCM buffer; the offline ffmpeg bridge (m10b-4)
//! encodes the buffer with the deterministic encoder profile.

use cf_replay::Event;

use crate::commentary::{COMMENTARY_CHANNELS, COMMENTARY_SAMPLE_RATE_HZ};

/// One sound on the timeline.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioEvent {
    pub tick: u64,
    pub canonical_name: String,
    pub bus: String,
    pub gain: f32,
}

/// Burst envelope length in samples — `40 ms` at 48 kHz = 1920
/// samples. Matches the cue family's average release time + keeps
/// audible peaks centred on the source tick.
pub const ENVELOPE_LENGTH_SAMPLES: usize = 1_920;

/// Peak-detection threshold for VAL-M10B-DEFAULT-AUDIO-MIX +
/// VAL-CROSS-013 assertions. Mixer output at the source-event tick
/// MUST exceed this floor; silence between events MUST stay below.
pub const PEAK_THRESHOLD_DBFS: f32 = -60.0;

/// Convert a linear PCM peak amplitude to dBFS.
#[must_use]
pub fn linear_to_dbfs(peak_amplitude: f32) -> f32 {
    if peak_amplitude <= 0.0 {
        return -120.0;
    }
    20.0 * peak_amplitude.log10()
}

/// Look up the canonical synthesis frequency for a cue. Cue families
/// are spread across the spectrum so adjacent cues remain
/// distinguishable in the offline mix. The table is deterministic +
/// covers every cue the mission's spec inventories (cf-audio
/// registry's `mg_nest_burst`, `mine_arming_beep`,
/// `electrified_shock_zap`, etc.). Unknown cues fall back to a
/// fingerprint hash so even modder-defined cues produce a distinct
/// audible tone.
#[must_use]
pub fn synthesis_frequency_hz(canonical_name: &str) -> f32 {
    match canonical_name {
        // M9C cues — frequencies pulled from the M12A canonical
        // synthesis spec. Distinct so a tester can identify by ear.
        "mg_nest_burst" => 440.0,
        "mine_arming_beep" => 880.0,
        "electrified_shock_zap" => 1320.0,
        "wire_snag_pain" => 660.0,
        "spotlight_relay_click" => 1760.0,
        "tripwire_snap" => 1100.0,
        // M9B trench cues.
        "duckboard_step" => 200.0,
        "mud_squelch" => 110.0,
        "entrenching_dig" => 150.0,
        "drainage_drip" => 360.0,
        // Generic fall-through: hash the canonical name into the
        // 200-3000 Hz audible band. Pure ASCII byte arithmetic keeps
        // the mapping deterministic across hosts.
        _ => {
            let mut hash: u32 = 5_381;
            for b in canonical_name.bytes() {
                hash = hash.wrapping_mul(33).wrapping_add(u32::from(b));
            }
            let band = (hash % 2_800) as f32;
            200.0 + band
        }
    }
}

/// Synthesize a 48 kHz stereo base-mix PCM buffer from the bundle's
/// `audio.event_played` events. Buffer length is
/// `tick_count * COMMENTARY_SAMPLE_RATE_HZ / tick_rate_hz` samples per
/// channel (stereo interleaved).
///
/// Each event becomes a [`ENVELOPE_LENGTH_SAMPLES`]-sample sine burst
/// centred on the source tick. Gain is applied per-event from the
/// envelope payload (`gain` field on the audio_event_played schema).
#[must_use]
pub fn synthesize_base_mix(events: &[Event], tick_rate_hz: u32, end_tick: u64) -> Vec<f32> {
    let samples_per_tick = COMMENTARY_SAMPLE_RATE_HZ as f64 / tick_rate_hz as f64;
    let total_samples = (end_tick as f64 * samples_per_tick) as usize;
    let total_channels = COMMENTARY_CHANNELS as usize;
    let mut buffer = vec![0.0f32; total_samples * total_channels];

    for event in events.iter().filter(|e| e.event_type == "audio.event_played") {
        let canonical_name = event
            .payload
            .get("canonical_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let gain = event
            .payload
            .get("gain")
            .and_then(|v| v.as_f64())
            .map(|g| g as f32)
            .unwrap_or(1.0);
        let bus = event
            .payload
            .get("bus")
            .and_then(|v| v.as_str())
            .unwrap_or("sfx");
        let _ = bus; // placeholder for future bus-routed mixing

        let center_sample = (event.tick as f64 * samples_per_tick) as usize;
        let frequency_hz = synthesis_frequency_hz(canonical_name);
        for i in 0..ENVELOPE_LENGTH_SAMPLES {
            let abs_index = center_sample.saturating_add(i);
            if abs_index >= total_samples {
                break;
            }
            let phase = (i as f32 / COMMENTARY_SAMPLE_RATE_HZ as f32) * frequency_hz * std::f32::consts::TAU;
            // Linear decay envelope so the peak lives at the start of
            // the burst (= the source tick) and tapers away.
            let envelope = 1.0 - (i as f32 / ENVELOPE_LENGTH_SAMPLES as f32);
            let sample = phase.sin() * envelope * gain;
            buffer[abs_index * total_channels] += sample;
            buffer[abs_index * total_channels + 1] += sample;
        }
    }
    buffer
}

/// Find the peak amplitude inside a tick window.
///
/// `window_radius_ticks` is symmetric around `tick` (the live engine
/// fires `audio.event_played` at integer tick boundaries; we sample ±
/// the radius to catch the rising / falling edge of the burst).
#[must_use]
pub fn peak_dbfs_at_tick(buffer: &[f32], tick: u64, tick_rate_hz: u32, window_radius_ticks: u64) -> f32 {
    let samples_per_tick = COMMENTARY_SAMPLE_RATE_HZ as f64 / tick_rate_hz as f64;
    let center = (tick as f64 * samples_per_tick) as i64;
    let radius_samples = (window_radius_ticks as f64 * samples_per_tick) as i64;
    let total_channels = COMMENTARY_CHANNELS as usize;
    let start = (center - radius_samples).max(0) as usize;
    let end = ((center + radius_samples) as usize).min(buffer.len() / total_channels);
    let mut peak: f32 = 0.0;
    for sample_index in start..end {
        for c in 0..total_channels {
            let v = buffer[sample_index * total_channels + c].abs();
            if v > peak {
                peak = v;
            }
        }
    }
    linear_to_dbfs(peak)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// VAL-M10B-DEFAULT-AUDIO-MIX: peaks > -60 dBFS at audio.event_played
    /// tick offsets; ≤ -60 dBFS elsewhere.
    #[test]
    fn default_audio_mix_from_audio_event_played() {
        let events = vec![
            audio_event(120, "mg_nest_burst", 1.0),
            audio_event(360, "mine_arming_beep", 0.8),
            audio_event(600, "electrified_shock_zap", 0.9),
        ];
        let buffer = synthesize_base_mix(&events, 60, 1_000);
        assert!(!buffer.is_empty());
        for ev in &events {
            let peak = peak_dbfs_at_tick(&buffer, ev.tick, 60, 1);
            assert!(
                peak > PEAK_THRESHOLD_DBFS,
                "expected peak > {PEAK_THRESHOLD_DBFS} dBFS at tick {} (got {peak})",
                ev.tick
            );
        }
        // Sample a tick that has no event — silence floor.
        let silent_peak = peak_dbfs_at_tick(&buffer, 800, 60, 1);
        assert!(
            silent_peak <= PEAK_THRESHOLD_DBFS,
            "silence between events should be ≤ {PEAK_THRESHOLD_DBFS} dBFS (got {silent_peak})"
        );
    }

    /// VAL-CROSS-013: audio mix carries M9C cues at correct tick
    /// offsets — `mg_nest_burst`, `mine_arming_beep`,
    /// `electrified_shock_zap` all > -60 dBFS.
    #[test]
    fn audio_mix_carries_m9c_cues_at_correct_offsets() {
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
                "M9C cue {} @ tick {} must be audible (got {peak} dBFS)",
                ev.payload.get("canonical_name").unwrap(),
                ev.tick
            );
        }
    }

    /// Distinct cues produce distinct frequencies so the listener can
    /// audibly tell them apart.
    #[test]
    fn distinct_cues_map_to_distinct_frequencies() {
        let a = synthesis_frequency_hz("mg_nest_burst");
        let b = synthesis_frequency_hz("mine_arming_beep");
        let c = synthesis_frequency_hz("electrified_shock_zap");
        assert_ne!(a, b);
        assert_ne!(b, c);
        assert_ne!(a, c);
    }

    /// Mixer is byte-deterministic across two runs on the same host.
    #[test]
    fn mix_is_byte_identical_across_runs() {
        let events = vec![audio_event(120, "mg_nest_burst", 1.0)];
        let a = synthesize_base_mix(&events, 60, 600);
        let b = synthesize_base_mix(&events, 60, 600);
        assert_eq!(a, b);
    }

    /// linear_to_dbfs handles zero amplitude without panicking.
    #[test]
    fn linear_to_dbfs_handles_zero() {
        assert!(linear_to_dbfs(0.0) <= -100.0);
    }
}
