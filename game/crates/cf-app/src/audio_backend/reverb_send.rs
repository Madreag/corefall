//! **M12B** § Convolution reverb send bus.
//!
//! Per M12B spec § Files:
//!
//! > `game/crates/cf-app/src/audio_backend/reverb_send.rs` (NEW:
//! > convolution reverb send bus)
//!
//! Per spec § Notes:
//!
//! > Reverb send is one global send bus, not per-room: the IR loaded
//! > into the send bus is swapped when the listener crosses a room
//! > boundary (per M19G boundary detection). Cross-fade the IR over
//! > 250 ms to avoid clicks.
//!
//! The adapter is **pure shape** — the actual convolution against the
//! IR file happens in the bevy_audio backend. This module owns the
//! IR-id selection logic + the cross-fade alpha computation.

use cf_audio::{DecayBand, ReverbProfile};

/// **M12B** § IR cross-fade window in ms when the listener crosses a
/// room boundary. Spec literal § "Cross-fade the IR over 250 ms to
/// avoid clicks".
pub const IR_CROSS_FADE_MS: f32 = 250.0;

/// **M12B** § Canonical IR file ids — the 8 pre-baked impulse
/// responses shipped under `game/content/audio/reverb/impulse_responses/`.
/// Per M12B spec § Files:
///
/// > 8 IRs — bunker_small_steel / bunker_med_concrete / warehouse_large
/// > / cave_natural / fabric_lined / glass_lab / open_outdoor /
/// > vacuum_null.
const IR_IDS: &[&str] = &[
    "bunker_small_steel",
    "bunker_med_concrete",
    "warehouse_large",
    "cave_natural",
    "fabric_lined",
    "glass_lab",
    "open_outdoor",
    "vacuum_null",
];

/// **M12B** § Pick the canonical IR id for a given [`ReverbProfile`].
/// The selector reads `tail_seconds` + `decay_band` and picks the IR
/// whose spectral character + decay length matches best.
///
/// Per spec § Notes:
///
/// > Convolution-reverb modder pipeline (custom IRs) — modders extend
/// > `material_registry` + select from the 8 baked IRs.
#[must_use]
pub fn current_ir_id_for(profile: &ReverbProfile) -> &'static str {
    // Open outdoor: dry-only.
    if profile.tail_seconds <= 0.2 + 1e-3 && profile.wet_dry_mix <= 0.05 {
        return "open_outdoor";
    }
    match (profile.decay_band, profile.tail_seconds) {
        (DecayBand::BrightRinging, t) if t < 0.5 => "bunker_small_steel",
        (DecayBand::BrightShort, _) => "glass_lab",
        (DecayBand::Bright, t) if t < 1.0 => "bunker_med_concrete",
        (DecayBand::Bright, _) => "warehouse_large",
        (DecayBand::WarmMid, _) => "cave_natural",
        (DecayBand::WarmLow, _) => "cave_natural",
        (DecayBand::Dampened, _) | (DecayBand::Anechoic, _) => "fabric_lined",
        (DecayBand::BrightRinging, _) => "warehouse_large",
    }
}

/// **M12B** § All canonical IR ids — used by content-validation tests
/// to confirm every id has a backing IR file in
/// `game/content/audio/reverb/impulse_responses/`.
#[must_use]
pub fn all_ir_ids() -> &'static [&'static str] {
    IR_IDS
}

/// **M12B** § Compute the cross-fade alpha during an IR swap. `0.0` at
/// fade-start, `1.0` at fade-complete. Linear ramp; pre-clamped to
/// `[0.0, 1.0]`.
#[must_use]
pub fn cross_fade_alpha(elapsed_ms: f32) -> f32 {
    (elapsed_ms / IR_CROSS_FADE_MS).clamp(0.0, 1.0)
}

/// **M12B** § One playback frame produced by the reverb send bus —
/// per-bus send level + active IR id + cross-fade alpha when an IR
/// swap is in flight.
#[derive(Debug, Clone, PartialEq)]
pub struct ReverbSendFrame {
    /// Linear `[0.0, 1.0]` send level (wet fraction).
    pub send: f32,
    /// Active IR id.
    pub ir_id: &'static str,
    /// Previous IR id (during cross-fade); `None` when no fade is in
    /// flight.
    pub fading_from: Option<&'static str>,
    /// 0.0..=1.0 cross-fade progress (0 = fully on previous, 1 = fully
    /// on current).
    pub cross_fade_alpha: f32,
}

impl ReverbSendFrame {
    /// **M12B** § Resolve a [`ReverbSendFrame`] given a [`ReverbProfile`]
    /// + the previous frame (for cross-fade detection).
    #[must_use]
    pub fn from_profile(profile: &ReverbProfile, previous: Option<&ReverbSendFrame>) -> Self {
        let ir_id = current_ir_id_for(profile);
        let send = profile.wet_dry_mix.clamp(0.0, 1.0);
        let (fading_from, alpha) = match previous {
            Some(prev) if prev.ir_id != ir_id => (Some(prev.ir_id), 0.0),
            _ => (None, 1.0),
        };
        Self {
            send,
            ir_id,
            fading_from,
            cross_fade_alpha: alpha,
        }
    }

    /// **M12B** § `true` when an IR cross-fade is in flight.
    #[must_use]
    pub fn is_fading(&self) -> bool {
        self.fading_from.is_some() && self.cross_fade_alpha < 1.0
    }
}

/// **M12B** § Reverb send bus. The cf-app Bevy audio backend owns one
/// instance; the bus has a single global send level + a single global
/// IR. Per spec § "Reverb send is one global send bus, not per-room".
#[derive(Debug, Clone, Default)]
pub struct ReverbSendBus {
    /// Most-recent frame produced; updated per Bevy tick when the
    /// listener's current room changes.
    pub current_frame: Option<ReverbSendFrame>,
    /// Time elapsed (ms) within the current IR cross-fade.
    pub cross_fade_elapsed_ms: f32,
}

impl ReverbSendBus {
    /// **M12B** § Update the bus given a new [`ReverbProfile`] (from the
    /// listener's current room). Returns the resolved
    /// [`ReverbSendFrame`] for the Bevy backend.
    pub fn update(&mut self, profile: &ReverbProfile, delta_ms: f32) -> ReverbSendFrame {
        let prev = self.current_frame.clone();
        let mut frame = ReverbSendFrame::from_profile(profile, prev.as_ref());
        // If we entered an IR-swap on this update, reset the fade
        // elapsed; otherwise advance it.
        if frame.is_fading() {
            self.cross_fade_elapsed_ms = 0.0;
        } else if let Some(prev_frame) = prev.as_ref() {
            if prev_frame.is_fading() {
                self.cross_fade_elapsed_ms += delta_ms.max(0.0);
                frame.fading_from = prev_frame.fading_from;
                frame.cross_fade_alpha = cross_fade_alpha(self.cross_fade_elapsed_ms);
                if frame.cross_fade_alpha >= 1.0 {
                    frame.fading_from = None;
                    self.cross_fade_elapsed_ms = 0.0;
                }
            }
        }
        self.current_frame = Some(frame.clone());
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_audio::DecayBand;

    fn p(band: DecayBand, tail: f32, mix: f32) -> ReverbProfile {
        ReverbProfile {
            tail_seconds: tail,
            decay_coefficient: 0.5,
            decay_band: band,
            wet_dry_mix: mix,
            early_reflection_delay_ms: 10.0,
            aperture_attenuation_db: 0.0,
        }
    }

    #[test]
    fn current_ir_picks_bunker_small_steel_for_steel_bunker() {
        let prof = p(DecayBand::BrightRinging, 0.22, 0.6);
        assert_eq!(current_ir_id_for(&prof), "bunker_small_steel");
    }

    #[test]
    fn current_ir_picks_warehouse_for_concrete_warehouse() {
        let prof = p(DecayBand::Bright, 2.1, 0.57);
        assert_eq!(current_ir_id_for(&prof), "warehouse_large");
    }

    #[test]
    fn current_ir_picks_bunker_med_concrete_for_small_concrete_room() {
        let prof = p(DecayBand::Bright, 0.5, 0.4);
        assert_eq!(current_ir_id_for(&prof), "bunker_med_concrete");
    }

    #[test]
    fn current_ir_picks_glass_lab_for_glass() {
        let prof = p(DecayBand::BrightShort, 0.4, 0.4);
        assert_eq!(current_ir_id_for(&prof), "glass_lab");
    }

    #[test]
    fn current_ir_picks_fabric_lined_for_dampened() {
        let prof = p(DecayBand::Dampened, 0.4, 0.2);
        assert_eq!(current_ir_id_for(&prof), "fabric_lined");
        let prof = p(DecayBand::Anechoic, 0.3, 0.1);
        assert_eq!(current_ir_id_for(&prof), "fabric_lined");
    }

    #[test]
    fn current_ir_picks_open_outdoor_for_open() {
        let prof = ReverbProfile::open_outdoor();
        assert_eq!(current_ir_id_for(&prof), "open_outdoor");
    }

    #[test]
    fn cross_fade_alpha_clamps_unit_range() {
        assert!((cross_fade_alpha(-100.0)).abs() < 1e-6);
        assert!((cross_fade_alpha(500.0) - 1.0).abs() < 1e-6);
        assert!((cross_fade_alpha(125.0) - 0.5).abs() < 1e-4);
    }

    #[test]
    fn reverb_send_frame_starts_at_full_when_no_previous() {
        let prof = p(DecayBand::Bright, 2.1, 0.57);
        let frame = ReverbSendFrame::from_profile(&prof, None);
        assert_eq!(frame.ir_id, "warehouse_large");
        assert!((frame.send - 0.57).abs() < 1e-4);
        assert!(frame.fading_from.is_none());
        assert!((frame.cross_fade_alpha - 1.0).abs() < 1e-4);
    }

    #[test]
    fn reverb_send_frame_starts_cross_fade_when_ir_changes() {
        let prev = ReverbSendFrame {
            send: 0.6,
            ir_id: "bunker_small_steel",
            fading_from: None,
            cross_fade_alpha: 1.0,
        };
        let next_prof = p(DecayBand::Bright, 2.1, 0.57);
        let frame = ReverbSendFrame::from_profile(&next_prof, Some(&prev));
        assert_eq!(frame.ir_id, "warehouse_large");
        assert_eq!(frame.fading_from, Some("bunker_small_steel"));
        assert!(frame.cross_fade_alpha < 1.0);
        assert!(frame.is_fading());
    }

    #[test]
    fn bus_update_advances_cross_fade_alpha_per_tick() {
        let prof_a = p(DecayBand::BrightRinging, 0.22, 0.6);
        let prof_b = p(DecayBand::Bright, 2.1, 0.57);
        let mut bus = ReverbSendBus::default();
        let _ = bus.update(&prof_a, 0.0); // initial frame.
        let _frame_first_swap = bus.update(&prof_b, 0.0); // start cross-fade.
        let frame_advance = bus.update(&prof_b, 125.0);
        assert!(frame_advance.is_fading());
        assert!((frame_advance.cross_fade_alpha - 0.5).abs() < 0.05);
        let frame_done = bus.update(&prof_b, 250.0);
        assert!(!frame_done.is_fading());
        assert!((frame_done.cross_fade_alpha - 1.0).abs() < 1e-4);
    }

    #[test]
    fn all_ir_ids_includes_the_eight_canonical_ids() {
        let ids = all_ir_ids();
        assert_eq!(ids.len(), 8);
        for canonical in [
            "bunker_small_steel",
            "bunker_med_concrete",
            "warehouse_large",
            "cave_natural",
            "fabric_lined",
            "glass_lab",
            "open_outdoor",
            "vacuum_null",
        ] {
            assert!(ids.contains(&canonical), "missing canonical IR id {canonical}");
        }
    }

    #[test]
    fn ir_cross_fade_constant_matches_spec_literal() {
        // Spec § "Cross-fade the IR over 250 ms".
        assert!((IR_CROSS_FADE_MS - 250.0).abs() < 1e-3);
    }
}
