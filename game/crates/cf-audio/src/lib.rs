//! cf-audio — audio cue catalogue + plugin trait used by the M0/M1 engine.
//!
//! The crate intentionally stays presentation-free: it defines the **catalogue**
//! of audio cues the sim/engine emits (`AudioCue`) and a trait `AudioPlugin`
//! that any future native backend (rodio, oboe, AVAudioEngine, etc.) can
//! implement. The default `NullAudioPlugin` is the M1 wiring — every cue is a
//! no-op so headless replays + `cargo test` runs are silent and byte-stable.
//!
//! ## Why this lives here, not in `cf-app`
//!
//! - `cf-control::engine` needs to surface "this cue should play" *via the same
//!   tick-deterministic event stream* the replay verifier consumes. Pulling in
//!   an audio backend would balloon the determinism surface; the trait split
//!   keeps real audio out of the sim path entirely.
//! - `cf-tools-replay-viewer` (M3A) can implement `AudioPlugin` to *re-play* a
//!   recorded bundle's audio cues for QA review without ever touching the live
//!   sim.
//! - `cf-e2e` tests (M1 R2) can use `RecordingAudioPlugin` to assert "this
//!   script triggered N gunshot cues + M reload-complete cues + K body-hit
//!   cues" without flaking on real-world audio device availability.
//!
//! ## Captions
//!
//! Every cue carries an optional human-readable caption string. The engine
//! mirrors the caption into `HudState.captions` so the accessibility surface
//! (DR-019: "captions are the audio for deaf-or-headphones-off players")
//! always shows the same text whether or not a real audio backend is wired.

#![forbid(unsafe_code)]
#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tracing::trace;

pub mod caption_bridge;
pub mod chatter;
pub mod cinematic_mixer;
pub mod deterministic_replay;
pub mod doppler;
pub mod echo;
pub mod footstep_lookup;
pub mod loader;
pub mod medium;
pub mod mix;
pub mod occlusion;
pub mod positional;
pub mod registry;
pub mod reverb;
pub mod spatial;

pub use footstep_lookup::{fallback_footstep_cue, lookup_footstep_cue};

pub use caption_bridge::{
    caption_visible, render_caption_for_sfx, resolve_template, CaptionRegistry, CaptionSeverity, CaptionTemplate,
};
pub use cinematic_mixer::{
    CinematicMix, CinematicMixer, CINEMATIC_DUCK_ATTACK_MS, CINEMATIC_DUCK_RELEASE_MS,
    CINEMATIC_MUSIC_LUFS_DURING_NARRATION, CINEMATIC_MUSIC_LUFS_OUTSIDE_NARRATION, CINEMATIC_NARRATION_LUFS,
    CINEMATIC_SFX_LUFS,
};
pub use chatter::{
    tts_stub, voice_id_for_archetype, ChatterCaption, ChatterCategory, ChatterCooldownTable, ChatterEmittedEvent,
    EmissionInfo, Phoneme, PLACEHOLDER_PHONEME_MS,
};
pub use deterministic_replay::{
    is_cosmetic_audio_event, is_cosmetic_audio_event_for, AudioPlaybackEvent, AudioReplayQueue, M12B_COSMETIC_EVENT_TYPES,
};
pub use doppler::{clamp_factor, resolve_doppler, DopplerShift, DOPPLER_FACTOR_MAX, DOPPLER_FACTOR_MIN};
pub use echo::{dominant_band, echo_response_for, weighted_mean_coefficient, DecayBand, EchoResponse};
pub use loader::{relative_to_repo_root, SfxEntry, SfxPool, MEMORY_BUDGET_BYTES};
pub use medium::{
    medium_at_midpoint, Medium, MediumFilter, SPEED_OF_SOUND_AIR_M_PER_S, SPEED_OF_SOUND_AMMONIA_M_PER_S,
    SPEED_OF_SOUND_VACUUM_M_PER_S, SPEED_OF_SOUND_WATER_M_PER_S,
};
pub use mix::{AudioBus, MixBuses, DEFAULT_TARGET_LUFS, LUFS_TOLERANCE, PEAK_DBFS_CEILING};
pub use occlusion::{resolve_occlusion, walls_between, OcclusionEnvelope, WallAcoustics, MAX_WALLS_PER_RAY};
pub use positional::{
    direction_of, distance_attenuation, occlusion_attenuation, resolve_direction_string, resolve_spatial, source_gain,
    top_n_by_gain, AudioDirection, DirectionString, HrirIndex, HrirTable, ListenerContext, SourceContext,
    SpatialEnvelope, AHEAD_BEHIND_CONE_RAD, DIRECTION_HERE_THRESHOLD_M, HRTF_AZIMUTH_BUCKETS, HRTF_EARS,
    HRTF_ELEVATION_BUCKETS, HRTF_SAMPLES, HRTF_TOTAL_BYTES, HRTF_TOTAL_F32, MAX_SIMULTANEOUS_VOICES,
    SPATIAL_HERE_RADIUS_M,
};
pub use registry::{family_members, AudioAsset, AudioRegistry, TRENCH_AUDIO_CUES};
pub use reverb::{derive_reverb_profile, fraction_of_walls_open, ReverbProfile, WallComposition};

/// Catalogue of M1 audio cues. Tied 1:1 to the event taxonomy in
/// `cf-replay` so a replay-viewer plugin can pattern-match on the event
/// stream and call `play()` deterministically.
///
/// Extensible: new cues should append at the end so old replay bundles
/// remain readable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AudioCue {
    /// `equipment.weapon_fired`: a rifle shot. The cue carries the weapon
    /// preset id so a real backend can map id → wav.
    WeaponFired {
        /// Equipment preset id (e.g. `cf_equipment::RIFLE_M1_DEFAULT_ID`).
        equipment_id: String,
        /// Caption surfaced to `HudState.captions` for accessibility.
        caption: String,
    },
    /// `equipment.weapon_reload_started`: the reload animation/timer began.
    ReloadStarted {
        /// Equipment preset id.
        equipment_id: String,
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
    /// `equipment.weapon_reload_completed`: reload finished, mag refilled.
    ReloadCompleted {
        /// Equipment preset id.
        equipment_id: String,
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
    /// `combat.projectile_hit`: a projectile landed on an actor. Carries
    /// the impacted zone so a backend can play head/torso/arm/leg variants.
    BodyHit {
        /// Impact zone (e.g. "head", "torso", "leg").
        zone: String,
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
    /// `actor.inventory_dropped`: an item fell out of an actor's hand on
    /// DYING entry. The clang/drop sound.
    InventoryDropped {
        /// Label of the dropped item (e.g. "rifle").
        item_label: String,
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
    /// `actor.inventory_settled`: a loose item came to rest on the floor.
    /// Used for the final "ka-thunk" after bounces stop.
    InventorySettled {
        /// Label of the settled item.
        item_label: String,
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
    /// `actor.actor_status_changed`: STABLE → UNSTABLE → DOWNED → DYING → DEAD.
    /// One status-change pip per transition; lets the player hear when they
    /// got knocked down even if they were looking at the wrong part of the screen.
    StatusChanged {
        /// Actor id of the transitioning actor.
        actor: u64,
        /// New status name (e.g. "dying", "downed").
        new_status: String,
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
    /// seven canonical juice rules per `cf-render-2d::juice::JuiceKind`.
    /// The accessibility_suppressed flag mirrors the matching
    /// `JuicePulse.accessibility_suppressed` so a recording plugin can
    /// see which cues were silenced by reduce_motion/reduce_flash.
    Juice {
        /// snake_case rule name matching `JuiceKind::as_str` (e.g.
        /// "button_hover"). Named `rule` to avoid colliding with the
        /// outer `#[serde(tag = "kind")]` enum-variant discriminator.
        rule: String,
        /// Optional HUD node id the juice targeted (e.g. "menu.new_game").
        target_node: Option<String>,
        /// Mirror of the `JuicePulse.accessibility_suppressed` flag.
        accessibility_suppressed: bool,
    },
    /// One-shot when the per-chunk integrity field first crosses the
    /// L1 threshold. Drives the "tunnel_creak" SFX backend stub.
    TunnelCreak {
        /// Chunk coordinate that triggered the warning.
        chunk_id: (i32, i32),
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
    /// per cave-in event. Drives the "cave_in_thunder" SFX backend stub.
    CaveInThunder {
        /// Chunk coordinate of the collapsing ceiling.
        chunk_id: (i32, i32),
        /// X pixel for the spatializer anchor (centre of the bbox).
        world_pos_x_px: i64,
        /// Y pixel for the spatializer anchor (centre of the bbox).
        world_pos_y_px: i64,
        /// Caption surfaced to `HudState.captions`.
        caption: String,
    },
}

impl AudioCue {
    /// Returns the caption text the engine will mirror into
    /// `HudState.captions`.
    pub fn caption(&self) -> &str {
        match self {
            AudioCue::WeaponFired { caption, .. } => caption,
            AudioCue::ReloadStarted { caption, .. } => caption,
            AudioCue::ReloadCompleted { caption, .. } => caption,
            AudioCue::BodyHit { caption, .. } => caption,
            AudioCue::InventoryDropped { caption, .. } => caption,
            AudioCue::InventorySettled { caption, .. } => caption,
            AudioCue::StatusChanged { caption, .. } => caption,
            AudioCue::Juice { .. } => "",
            AudioCue::TunnelCreak { caption, .. } => caption,
            AudioCue::CaveInThunder { caption, .. } => caption,
        }
    }

    /// Stable cue tag used for the SFX-stub filename lookup. Backends map
    /// the tag → `content/sfx/<tag>.wav`.
    pub fn stub_tag(&self) -> &'static str {
        match self {
            AudioCue::WeaponFired { .. } => "weapon_fired",
            AudioCue::ReloadStarted { .. } => "reload_started",
            AudioCue::ReloadCompleted { .. } => "reload_completed",
            AudioCue::BodyHit { .. } => "body_hit",
            AudioCue::InventoryDropped { .. } => "inventory_dropped",
            AudioCue::InventorySettled { .. } => "inventory_settled",
            AudioCue::StatusChanged { .. } => "status_changed",
            AudioCue::Juice { .. } => "juice",
            AudioCue::TunnelCreak { .. } => "tunnel_creak",
            AudioCue::CaveInThunder { .. } => "cave_in_thunder",
        }
    }
}

/// Trait every audio backend implements. The engine calls `play(cue)`
/// from the recorder path so cues are deterministic relative to the tick
/// number that generated them.
pub trait AudioPlugin: Send + Sync {
    /// Play a cue. Backends are free to no-op if the cue is unrecognized
    /// or if the player has disabled audio.
    fn play(&self, cue: &AudioCue);
}

/// Default plugin used in headless / replay / test runs. Every cue is a
/// no-op; the caption mirroring side-effect still happens on the engine
/// side because it goes through `HudState`, not the plugin.
#[derive(Debug, Default, Clone, Copy)]
pub struct NullAudioPlugin;

impl AudioPlugin for NullAudioPlugin {
    fn play(&self, cue: &AudioCue) {
        // Tracing so a future debug session can see cues fire without
        // attaching a real backend.
        trace!(target: "cf::audio", tag = cue.stub_tag(), caption = cue.caption(), "cue (null plugin)");
    }
}

/// Test plugin that records every cue it sees. Useful for cf-e2e and
/// cargo-test assertions like "verify that running `m1_jump_only`
/// triggered exactly 6 `WeaponFired` cues with caption matching `rifle`."
#[derive(Default)]
pub struct RecordingAudioPlugin {
    /// All cues seen, in the order `play()` was called.
    pub cues: Mutex<Vec<AudioCue>>,
}

impl RecordingAudioPlugin {
    /// Returns a snapshot of all cues recorded so far.
    pub fn snapshot(&self) -> Vec<AudioCue> {
        self.cues.lock().map(|v| v.clone()).unwrap_or_default()
    }
}

impl AudioPlugin for RecordingAudioPlugin {
    fn play(&self, cue: &AudioCue) {
        if let Ok(mut v) = self.cues.lock() {
            v.push(cue.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_plugin_compiles_and_no_ops() {
        let p = NullAudioPlugin;
        p.play(&AudioCue::WeaponFired {
            equipment_id: "rifle_m1_default".to_string(),
            caption: "rifle fires".to_string(),
        });
        // No state to inspect — by definition no-op.
    }

    #[test]
    fn recording_plugin_captures_cues_in_order() {
        let p = RecordingAudioPlugin::default();
        p.play(&AudioCue::WeaponFired {
            equipment_id: "rifle_m1_default".to_string(),
            caption: "rifle fires".to_string(),
        });
        p.play(&AudioCue::ReloadStarted {
            equipment_id: "rifle_m1_default".to_string(),
            caption: "reloading rifle".to_string(),
        });
        p.play(&AudioCue::BodyHit {
            zone: "torso".to_string(),
            caption: "body hit (torso)".to_string(),
        });
        let snapshot = p.snapshot();
        assert_eq!(snapshot.len(), 3);
        assert_eq!(snapshot[0].stub_tag(), "weapon_fired");
        assert_eq!(snapshot[1].stub_tag(), "reload_started");
        assert_eq!(snapshot[2].stub_tag(), "body_hit");
    }

    #[test]
    fn audio_cue_round_trips_through_serde() {
        let cue = AudioCue::InventoryDropped {
            item_label: "rifle".to_string(),
            caption: "rifle dropped".to_string(),
        };
        let json = serde_json::to_string(&cue).expect("serialize");
        let back: AudioCue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cue, back);
    }

    #[test]
    fn juice_cue_serializes_with_rule_field() {
        let cue = AudioCue::Juice {
            rule: "button_hover".to_string(),
            target_node: Some("menu.new_game".to_string()),
            accessibility_suppressed: false,
        };
        assert_eq!(cue.stub_tag(), "juice");
        let json = serde_json::to_string(&cue).expect("serialize");
        let back: AudioCue = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(cue, back);
    }

    #[test]
    fn juice_cue_carries_accessibility_suppression() {
        let cue = AudioCue::Juice {
            rule: "critical_hit_punch".to_string(),
            target_node: None,
            accessibility_suppressed: true,
        };
        match cue {
            AudioCue::Juice {
                accessibility_suppressed,
                ..
            } => assert!(accessibility_suppressed),
            _ => panic!("expected Juice variant"),
        }
    }
}
