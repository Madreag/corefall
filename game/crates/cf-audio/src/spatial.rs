//! **M12B** § HRTF spatial audio — table + per-source HRIR convolution
//! descriptor.
//!
//! Per spec § HRTF resolution (locked):
//!
//! ```text
//! SpatialEnvelope {
//!     azimuth_rad: atan2(source.y - listener.y, source.x - listener.x) - listener.facing_rad,
//!     elevation_rad: 0.0,  // 2D side-view; reserve for future M14 verticality
//!     distance_m: norm(source - listener),
//!     gain: source_gain(distance_m, source.base_gain),
//!     hrir_index: lookup(azimuth_rad, elevation_rad),  // 32 × 8 grid
//!     occlusion_db: sum(wall.transmission_loss_db for wall in walls_between(source, listener)),
//!     medium_filter: medium_at(midpoint(source, listener)),
//!     doppler_factor: (343.0 + dot(listener.vel, dir_to_source)) /
//!                     (343.0 + dot(source.vel, dir_to_source)),
//!     reverb_send_db: reverb_profile.wet_dry_mix * 0.6 if listener_in_same_room else 0.0,
//! }
//! ```
//!
//! Per spec § HRTF table format:
//!
//! > `game/content/audio/hrtf/mit_kemar_subset.bin` — fixed-layout binary,
//! > 32 azimuth × 8 elevation × 2 ears × 128 samples × 4 bytes (f32) ≈
//! > 256 KB on disk after compression. Loaded once at startup into an
//! > `Arc<HrirTable>`; lookup is `O(1)` index math, no allocation per
//! > audio frame.
//!
//! Pure math; the actual HRIR convolution lives in
//! `cf-app::audio_backend::hrtf_convolution`. Determinism surface stays
//! here.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::doppler::{resolve_doppler, DopplerShift};
use crate::medium::{Medium, MediumFilter, SPEED_OF_SOUND_AIR_M_PER_S};
use crate::occlusion::{resolve_occlusion, OcclusionEnvelope, WallAcoustics};
use crate::reverb::ReverbProfile;

pub const HRTF_AZIMUTH_BUCKETS: usize = 32;
/// row; the 8 elevation rows reserve capacity for M14 verticality).
pub const HRTF_ELEVATION_BUCKETS: usize = 8;
pub const HRTF_EARS: usize = 2;
pub const HRTF_SAMPLES: usize = 128;
pub const HRTF_TOTAL_F32: usize = HRTF_AZIMUTH_BUCKETS * HRTF_ELEVATION_BUCKETS * HRTF_EARS * HRTF_SAMPLES;
pub const HRTF_TOTAL_BYTES: usize = HRTF_TOTAL_F32 * 4;
/// "here" direction string per spec § Direction-string section.
pub const SPATIAL_HERE_RADIUS_M: f32 = 1.5;
/// "ahead"; `|azimuth_rad - π| < π/12` is "behind you".
pub const AHEAD_BEHIND_CONE_RAD: f32 = std::f32::consts::FRAC_PI_2 / 6.0; // π/12

/// index resolver. Cf-app loads this once via [`HrirTable::placeholder`]
/// (or, in production, [`HrirTable::from_bytes`]) and shares it as
/// `Arc<HrirTable>`.
///
/// The table is sparse-shaped for M12B (only row 0 — horizontal — is
/// consumed); M14 verticality fills the other 7 elevation rows.
#[derive(Debug, Clone)]
pub struct HrirTable {
    /// Flat sample buffer: `[az][el][ear][sample]` row-major.
    samples: Arc<[f32]>,
    /// Stable identifier for diagnostic surfacing.
    label: &'static str,
}

impl HrirTable {
    /// contains a single-tap impulse (sample 0 = 1.0, rest = 0.0).
    /// Replays use this when the production `mit_kemar_subset.bin` isn't
    /// on disk; behavior stays deterministic (the cf-app HRIR
    /// convolution adapter sees a pass-through).
    #[must_use]
    pub fn placeholder() -> Self {
        let mut buf = vec![0.0_f32; HRTF_TOTAL_F32];
        for az in 0..HRTF_AZIMUTH_BUCKETS {
            for el in 0..HRTF_ELEVATION_BUCKETS {
                for ear in 0..HRTF_EARS {
                    let idx =
                        ((az * HRTF_ELEVATION_BUCKETS + el) * HRTF_EARS + ear) * HRTF_SAMPLES;
                    buf[idx] = 1.0;
                }
            }
        }
        Self {
            samples: buf.into(),
            label: "m12b_placeholder",
        }
    }

    /// if the byte buffer doesn't match [`HRTF_TOTAL_BYTES`] exactly.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HrirParseError> {
        if bytes.len() != HRTF_TOTAL_BYTES {
            return Err(HrirParseError::SizeMismatch {
                expected: HRTF_TOTAL_BYTES,
                actual: bytes.len(),
            });
        }
        let mut buf = vec![0.0_f32; HRTF_TOTAL_F32];
        for (i, chunk) in bytes.chunks_exact(4).enumerate() {
            let arr: [u8; 4] = chunk.try_into().map_err(|_| HrirParseError::Truncated)?;
            buf[i] = f32::from_le_bytes(arr);
            if !buf[i].is_finite() {
                return Err(HrirParseError::NonFiniteSample { index: i });
            }
        }
        Ok(Self {
            samples: buf.into(),
            label: "mit_kemar_subset",
        })
    }

    #[must_use]
    pub fn label(&self) -> &'static str {
        self.label
    }

    /// el_bucket, ear) → f32 slice of [`HRTF_SAMPLES`] taps`. Used by
    /// the cf-app HRIR convolution adapter.
    #[must_use]
    pub fn lookup(&self, az_bucket: usize, el_bucket: usize, ear: usize) -> &[f32] {
        let az = az_bucket.min(HRTF_AZIMUTH_BUCKETS - 1);
        let el = el_bucket.min(HRTF_ELEVATION_BUCKETS - 1);
        let ear = ear.min(HRTF_EARS - 1);
        let start = ((az * HRTF_ELEVATION_BUCKETS + el) * HRTF_EARS + ear) * HRTF_SAMPLES;
        &self.samples[start..start + HRTF_SAMPLES]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HrirParseError {
    /// Byte length doesn't match the canonical layout.
    SizeMismatch {
        /// Expected byte length.
        expected: usize,
        /// Observed byte length.
        actual: usize,
    },
    /// A non-finite sample (NaN / Inf) was encountered.
    NonFiniteSample {
        /// f32 sample index.
        index: usize,
    },
    /// Final chunk wasn't 4 bytes (truncated file).
    Truncated,
}

impl std::fmt::Display for HrirParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HrirParseError::SizeMismatch { expected, actual } => write!(
                f,
                "HRIR table size mismatch: expected {expected} bytes, got {actual}"
            ),
            HrirParseError::NonFiniteSample { index } => {
                write!(f, "HRIR sample {index} is not finite (NaN or Inf)")
            }
            HrirParseError::Truncated => write!(f, "HRIR table truncated mid-sample"),
        }
    }
}

impl std::error::Error for HrirParseError {}

/// 32×8 grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HrirIndex {
    /// 0..[`HRTF_AZIMUTH_BUCKETS`].
    pub azimuth_bucket: u8,
    /// 0..[`HRTF_ELEVATION_BUCKETS`].
    pub elevation_bucket: u8,
}

impl HrirIndex {
    /// is clamped to 0 (M12B reserves the elevation rows; only the
    /// horizontal row is consumed).
    #[must_use]
    pub fn from_azimuth_elevation(azimuth_rad: f32, _elevation_rad: f32) -> Self {
        let two_pi = std::f32::consts::TAU;
        let wrapped = azimuth_rad.rem_euclid(two_pi);
        let bucket_width = two_pi / (HRTF_AZIMUTH_BUCKETS as f32);
        let mut bucket = (wrapped / bucket_width).floor() as i32;
        if bucket < 0 {
            bucket += HRTF_AZIMUTH_BUCKETS as i32;
        }
        let bucket = (bucket as usize).min(HRTF_AZIMUTH_BUCKETS - 1);
        Self {
            azimuth_bucket: bucket as u8,
            elevation_bucket: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum DirectionString {
    /// Source within [`SPATIAL_HERE_RADIUS_M`] of listener.
    Here,
    /// Source directly ahead of listener facing.
    Ahead,
    /// Source directly behind listener facing.
    BehindYou,
    /// Source compass-N relative to listener facing.
    N,
    /// Compass-NE relative to listener facing.
    Ne,
    /// Compass-E relative to listener facing.
    E,
    /// Compass-SE relative to listener facing.
    Se,
    /// Compass-S relative to listener facing.
    S,
    /// Compass-SW relative to listener facing.
    Sw,
    /// Compass-W relative to listener facing.
    W,
    /// Compass-NW relative to listener facing.
    Nw,
}

impl DirectionString {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            DirectionString::Here => "here",
            DirectionString::Ahead => "ahead",
            DirectionString::BehindYou => "behind you",
            DirectionString::N => "N",
            DirectionString::Ne => "NE",
            DirectionString::E => "E",
            DirectionString::Se => "SE",
            DirectionString::S => "S",
            DirectionString::Sw => "SW",
            DirectionString::W => "W",
            DirectionString::Nw => "NW",
        }
    }
}

/// Direction-string for captions:
///
/// > - `|azimuth_rad| < π/12` → `"ahead"`
/// > - `|azimuth_rad - π| < π/12` → `"behind you"`
/// > - `distance_m < 1.5` → `"here"` (overrides direction)
/// > - Else: 8-way compass projection (`N`, `NE`, `E`, `SE`, `S`, `SW`,
/// >   `W`, `NW`)
///
/// Compass labels are in the listener's facing frame: with the listener
/// facing East (`facing_rad=0`), world-N (`azimuth=π/2`) → compass `N`
/// relative to facing (i.e. left of facing). Buckets are 45° wide
/// centred on the cardinal direction, with `ahead` / `behind you` cones
/// of ±π/12 carving into the front/back buckets.
#[must_use]
pub fn direction_string(azimuth_rad: f32, distance_m: f32) -> DirectionString {
    if distance_m < SPATIAL_HERE_RADIUS_M {
        return DirectionString::Here;
    }
    let pi = std::f32::consts::PI;
    let two_pi = std::f32::consts::TAU;
    let normalized = azimuth_rad.rem_euclid(two_pi);
    // ahead = |az| < π/12 (signed; wraps at 2π).
    let signed_az = if normalized > pi { normalized - two_pi } else { normalized };
    if signed_az.abs() < AHEAD_BEHIND_CONE_RAD {
        return DirectionString::Ahead;
    }
    if (signed_az.abs() - pi).abs() < AHEAD_BEHIND_CONE_RAD {
        return DirectionString::BehindYou;
    }
    // Compass labels at 45° spacing (CCW relative to listener facing).
    // The cardinal directions sit at:
    //   az=0   → ahead  (carved out)
    //   az=π/4 → NE
    //   az=π/2 → N
    //   az=3π/4 → NW
    //   az=π   → behind you  (carved out)
    //   az=-3π/4 / 5π/4 → SW
    //   az=-π/2 / 3π/2 → S
    //   az=-π/4 / 7π/4 → SE
    //
    // Round `signed_az` to the nearest π/4 multiple to find the compass
    // bucket; the special-case ahead/behind cones above already
    // guarantee that buckets carved by them never reach this point.
    let bucket = ((signed_az / std::f32::consts::FRAC_PI_4).round() as i32).rem_euclid(8);
    match bucket {
        0 => DirectionString::Ahead, // unreachable due to ahead cone, but safe.
        1 => DirectionString::Ne,
        2 => DirectionString::N,
        3 => DirectionString::Nw,
        4 => DirectionString::BehindYou,
        5 | -3 => DirectionString::Sw,
        6 | -2 => DirectionString::S,
        7 | -1 => DirectionString::Se,
        _ => DirectionString::Ahead,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ListenerContext {
    /// Listener world position.
    pub position: [f32; 2],
    /// Listener world velocity (m/s).
    pub velocity: [f32; 2],
    /// Listener facing direction in radians (0 = +X, CCW positive).
    pub facing_rad: f32,
    /// Listener's current room id. `None` for outdoor / not in any room.
    pub room_id: Option<u64>,
}

#[derive(Debug, Clone, Copy)]
pub struct SourceContext {
    /// Source world position.
    pub position: [f32; 2],
    /// Source world velocity (m/s).
    pub velocity: [f32; 2],
    /// Source base gain `[0.0, 1.0]` before spatial attenuation.
    pub base_gain: f32,
    /// Source's propagation range in meters (per-SFX falloff radius).
    pub propagation_range_m: f32,
    /// Source's current room id (for `listener_in_same_room` reverb-send
    /// gate).
    pub room_id: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpatialEnvelope {
    /// Listener-relative azimuth in radians.
    pub azimuth_rad: f32,
    /// Listener-relative elevation in radians (always 0 at M12B).
    pub elevation_rad: f32,
    /// Source ↔ listener distance in meters.
    pub distance_m: f32,
    /// Resolved gain `[0.0, 1.0]` (distance × occlusion × medium).
    pub gain: f32,
    /// 32×8 HRIR table index.
    pub hrir_index: HrirIndex,
    /// Occlusion descriptor (sum of wall losses + min cutoff).
    pub occlusion: OcclusionEnvelope,
    /// Medium filter (low-pass + gain + speed-of-sound).
    pub medium_filter: MediumFilter,
    /// Resolved Doppler descriptor (factor + clamped flag + medium-corrected c).
    pub doppler: DopplerShift,
    /// Reverb-send level in dB (`0.0` = full wet; negative attenuates).
    /// `-inf` here is represented by `gain=0` via the reverb-send adapter.
    pub reverb_send_db: f32,
    /// Caption-friendly direction string.
    pub direction: DirectionString,
}

/// pair given the room reverb profile + the per-wall acoustic list.
///
/// All inputs are deterministic; identical inputs → identical envelope
///
/// `walls_between_source_and_listener` carries the [`WallAcoustics`]
/// list for every wall traversed by the ray (deduplicated). When the
/// listener and source share a room id, the reverb send rides at
/// `wet_dry_mix * 0.6` per spec § "reverb_send_db: reverb_profile.wet_dry_mix
/// * 0.6 if listener_in_same_room else 0.0".
#[must_use]
pub fn resolve_spatial(
    source: SourceContext,
    listener: ListenerContext,
    medium: Medium,
    walls_between_source_and_listener: &[WallAcoustics],
    reverb_profile: ReverbProfile,
) -> SpatialEnvelope {
    let medium_filter = MediumFilter::for_medium(medium);

    // Distance.
    let dx = source.position[0] - listener.position[0];
    let dy = source.position[1] - listener.position[1];
    let distance_m = (dx * dx + dy * dy).sqrt();

    // Azimuth (listener-relative).
    let world_azimuth = dy.atan2(dx);
    let azimuth_rad = world_azimuth - listener.facing_rad;

    // HRIR bucket.
    let hrir_index = HrirIndex::from_azimuth_elevation(azimuth_rad, 0.0);

    // Distance attenuation.
    let dist_atten = crate::positional::distance_attenuation(distance_m, source.propagation_range_m);

    // Occlusion envelope.
    let occlusion = resolve_occlusion(walls_between_source_and_listener);
    let occlusion_gain = occlusion.gain_factor();

    // Gain composition: base × distance × medium × occlusion.
    let base_gain = source.base_gain.clamp(0.0, 1.0);
    let gain_raw = base_gain * dist_atten * medium_filter.gain * occlusion_gain;
    let gain = gain_raw.clamp(0.0, 1.0);

    // Doppler — vacuum short-circuits to factor=1 inside resolve_doppler.
    let doppler = if medium_filter.is_silent() {
        DopplerShift::unity()
    } else {
        let c = if medium_filter.speed_of_sound_m_per_s > 0.0 {
            medium_filter.speed_of_sound_m_per_s
        } else {
            SPEED_OF_SOUND_AIR_M_PER_S
        };
        resolve_doppler(source.position, source.velocity, listener.position, listener.velocity, c)
    };

    // Reverb send — listener_in_same_room gates the send.
    let same_room = match (listener.room_id, source.room_id) {
        (Some(l), Some(s)) => l == s,
        _ => false,
    };
    let reverb_send_db = if same_room {
        // Spec literal: wet_dry_mix * 0.6 — interpreted as a linear
        // multiplier on the wet send. The `audio.reverb_applied` event
        // surfaces the raw `wet_dry_mix` separately for replay
        // forensics.
        let send = (reverb_profile.wet_dry_mix * 0.6).clamp(0.0, 1.0);
        // Convert linear → dB for the event payload: 20*log10(send).
        // Send <= 1e-4 surfaces as -80 dB (effective silence).
        if send <= 1e-4 {
            -80.0
        } else {
            20.0 * send.log10()
        }
    } else {
        -80.0
    };

    let direction = direction_string(azimuth_rad, distance_m);

    SpatialEnvelope {
        azimuth_rad,
        elevation_rad: 0.0,
        distance_m,
        gain,
        hrir_index,
        occlusion,
        medium_filter,
        doppler,
        reverb_send_db,
        direction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::echo::DecayBand;

    fn mk_source(pos: [f32; 2]) -> SourceContext {
        SourceContext {
            position: pos,
            velocity: [0.0, 0.0],
            base_gain: 1.0,
            propagation_range_m: 100.0,
            room_id: None,
        }
    }

    fn mk_listener_facing_east() -> ListenerContext {
        ListenerContext {
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            facing_rad: 0.0, // +X = east.
            room_id: None,
        }
    }

    #[test]
    fn hrir_index_buckets_north_correctly() {
        // Listener at origin, source at +Y. World atan2(Y, 0) = π/2 → bucket
        // ~ N/4 of the way around the 32 wedges → bucket 8.
        let idx = HrirIndex::from_azimuth_elevation(std::f32::consts::FRAC_PI_2, 0.0);
        assert_eq!(idx.azimuth_bucket, 8);
        assert_eq!(idx.elevation_bucket, 0);
    }

    #[test]
    fn hrir_index_wraps_negative_azimuth() {
        let idx_neg = HrirIndex::from_azimuth_elevation(-std::f32::consts::FRAC_PI_2, 0.0);
        let idx_pos = HrirIndex::from_azimuth_elevation(
            std::f32::consts::TAU - std::f32::consts::FRAC_PI_2,
            0.0,
        );
        assert_eq!(idx_neg, idx_pos);
    }

    #[test]
    fn placeholder_hrir_table_has_canonical_size() {
        let t = HrirTable::placeholder();
        let slice = t.lookup(0, 0, 0);
        assert_eq!(slice.len(), HRTF_SAMPLES);
        assert!((slice[0] - 1.0).abs() < 1e-6);
        for s in &slice[1..] {
            assert!(s.abs() < 1e-6);
        }
    }

    #[test]
    fn hrir_table_from_bytes_rejects_size_mismatch() {
        let bad = vec![0_u8; HRTF_TOTAL_BYTES - 1];
        let err = HrirTable::from_bytes(&bad).unwrap_err();
        assert!(matches!(err, HrirParseError::SizeMismatch { .. }));
    }

    #[test]
    fn hrir_table_from_bytes_accepts_canonical_size() {
        let buf = vec![0_u8; HRTF_TOTAL_BYTES];
        let t = HrirTable::from_bytes(&buf).expect("parse");
        assert_eq!(t.label(), "mit_kemar_subset");
    }

    #[test]
    fn direction_string_returns_here_within_threshold() {
        let d = direction_string(0.0, 0.5);
        assert_eq!(d, DirectionString::Here);
    }

    #[test]
    fn direction_string_returns_ahead_for_small_azimuth() {
        let d = direction_string(0.05, 10.0);
        assert_eq!(d, DirectionString::Ahead);
    }

    #[test]
    fn direction_string_returns_behind_for_pi_azimuth() {
        let d = direction_string(std::f32::consts::PI, 10.0);
        assert_eq!(d, DirectionString::BehindYou);
    }

    #[test]
    fn direction_string_returns_left_compass_for_negative_azimuth() {
        // Az = π/2 → 90° CCW → "N" relative to listener facing.
        let d = direction_string(std::f32::consts::FRAC_PI_2, 10.0);
        assert_eq!(d, DirectionString::N);
        // Az = -π/2 → 90° CW → "S" relative to listener facing.
        let d = direction_string(-std::f32::consts::FRAC_PI_2, 10.0);
        assert_eq!(d, DirectionString::S);
        // Az = 3π/4 → "NW".
        let d = direction_string(3.0 * std::f32::consts::FRAC_PI_4, 10.0);
        assert_eq!(d, DirectionString::Nw);
        // Az = π/4 → "NE".
        let d = direction_string(std::f32::consts::FRAC_PI_4, 10.0);
        assert_eq!(d, DirectionString::Ne);
        // Az = -π/4 → "SE".
        let d = direction_string(-std::f32::consts::FRAC_PI_4, 10.0);
        assert_eq!(d, DirectionString::Se);
        // Az = -3π/4 → "SW".
        let d = direction_string(-3.0 * std::f32::consts::FRAC_PI_4, 10.0);
        assert_eq!(d, DirectionString::Sw);
    }

    #[test]
    fn resolve_spatial_locates_north_source_when_listener_faces_east() {
        // Listener faces east, source at +Y (10 m north).
        let env = resolve_spatial(
            mk_source([0.0, 10.0]),
            mk_listener_facing_east(),
            Medium::Air,
            &[],
            ReverbProfile::open_outdoor(),
        );
        assert!((env.distance_m - 10.0).abs() < 1e-4);
        assert!((env.azimuth_rad - std::f32::consts::FRAC_PI_2).abs() < 1e-3);
        assert_eq!(env.direction, DirectionString::N);
    }

    #[test]
    fn footstep_at_nw_10m_resolves_to_nw_caption() {
        // Spec acceptance scenario: actor at world (-8, 6) (NW, 10 m).
        let listener = mk_listener_facing_east();
        let env = resolve_spatial(
            mk_source([-8.0, 6.0]),
            listener,
            Medium::Air,
            &[],
            ReverbProfile::open_outdoor(),
        );
        // World atan2(6, -8) ≈ 2.498 rad ≈ NW relative to east-facing.
        assert!((env.azimuth_rad - 2.498).abs() < 0.05);
        assert_eq!(env.direction, DirectionString::Nw);
        assert!((env.distance_m - 10.0).abs() < 0.05);
    }

    #[test]
    fn vacuum_medium_silences_gain() {
        let listener = mk_listener_facing_east();
        let env = resolve_spatial(
            mk_source([8.0, 0.0]),
            listener,
            Medium::Vacuum,
            &[],
            ReverbProfile::open_outdoor(),
        );
        assert!(env.gain.abs() < 1e-6);
        // Doppler short-circuits to unity in vacuum.
        assert!((env.doppler.factor - 1.0).abs() < 1e-6);
    }

    #[test]
    fn underwater_medium_applies_low_pass_and_attenuation() {
        let listener = mk_listener_facing_east();
        let env = resolve_spatial(
            mk_source([0.0, 2.0]),
            listener,
            Medium::Water,
            &[],
            ReverbProfile::open_outdoor(),
        );
        assert_eq!(env.medium_filter.medium, Medium::Water);
        assert!((env.medium_filter.cutoff_hz - 800.0).abs() < 1e-3);
        assert!((env.medium_filter.gain - 0.6).abs() < 1e-3);
        // Audible (non-zero) gain.
        assert!(env.gain > 0.0);
    }

    #[test]
    fn concrete_wall_drops_gain_and_low_passes() {
        let listener = mk_listener_facing_east();
        let wall = WallAcoustics {
            transmission_loss_db: 28.0,
            low_pass_cutoff_hz: 800.0,
        };
        let env = resolve_spatial(
            mk_source([5.0, 0.0]),
            listener,
            Medium::Air,
            &[wall],
            ReverbProfile::open_outdoor(),
        );
        assert!((env.occlusion.occlusion_db - -28.0).abs() < 1e-4);
        assert!((env.occlusion.low_pass_cutoff_hz - 800.0).abs() < 1e-4);
        assert!(env.gain < 0.1); // ~10^(-28/20) = 0.04
    }

    #[test]
    fn reverb_send_only_when_listener_in_same_room() {
        let mut listener = mk_listener_facing_east();
        listener.room_id = Some(7);
        let mut source = mk_source([5.0, 0.0]);
        source.room_id = Some(7);
        let env_same = resolve_spatial(
            source,
            listener,
            Medium::Air,
            &[],
            ReverbProfile {
                tail_seconds: 2.0,
                decay_coefficient: 0.85,
                decay_band: DecayBand::Bright,
                wet_dry_mix: 0.5,
                early_reflection_delay_ms: 15.0,
                aperture_attenuation_db: 0.0,
            },
        );
        source.room_id = Some(99);
        let env_diff = resolve_spatial(
            source,
            listener,
            Medium::Air,
            &[],
            ReverbProfile {
                tail_seconds: 2.0,
                decay_coefficient: 0.85,
                decay_band: DecayBand::Bright,
                wet_dry_mix: 0.5,
                early_reflection_delay_ms: 15.0,
                aperture_attenuation_db: 0.0,
            },
        );
        assert!(env_same.reverb_send_db > env_diff.reverb_send_db);
        assert!(env_diff.reverb_send_db <= -80.0 + 0.1);
    }

    #[test]
    fn spatial_envelope_round_trips_through_serde() {
        let env = resolve_spatial(
            mk_source([10.0, 5.0]),
            mk_listener_facing_east(),
            Medium::Air,
            &[],
            ReverbProfile::open_outdoor(),
        );
        let s = serde_json::to_string(&env).unwrap();
        let back: SpatialEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(env, back);
    }

    #[test]
    fn spatial_resolve_is_deterministic() {
        let l = mk_listener_facing_east();
        let s = mk_source([12.34, 56.78]);
        let env_a = resolve_spatial(s, l, Medium::Air, &[], ReverbProfile::open_outdoor());
        let env_b = resolve_spatial(s, l, Medium::Air, &[], ReverbProfile::open_outdoor());
        assert_eq!(env_a, env_b);
    }

    #[test]
    fn loads_canonical_placeholder_binary_from_disk() {
        // Locate game/content/audio/hrtf/mit_kemar_subset.bin relative to
        // the CARGO_MANIFEST_DIR. The cargo test harness sets it to
        // `<repo>/game/crates/cf-audio` so we walk up to `game/` and
        // descend into content.
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let path = manifest
            .parent()
            .and_then(|p| p.parent())
            .map(|game| game.join("content/audio/hrtf/mit_kemar_subset.bin"));
        let Some(path) = path else { return };
        if !path.exists() {
            // CI environment may not have generated the binary yet.
            return;
        }
        let bytes = std::fs::read(&path).expect("read hrtf binary");
        let t = HrirTable::from_bytes(&bytes).expect("load placeholder hrtf");
        let slice = t.lookup(0, 0, 0);
        assert_eq!(slice.len(), HRTF_SAMPLES);
        // Placeholder always puts a 1.0 impulse at sample 0.
        assert!((slice[0] - 1.0).abs() < 1e-4);
    }

    #[test]
    fn ahead_left_facing_east_at_ne_world() {
        // Source at world (10, 10) — listener facing east at origin. World
        // azimuth = π/4; listener-relative = π/4. The spec acceptance
        // scenario says "azimuth ≈ π/4 (45° left of facing — NE in world,
        // but N relative to facing)". Verify the resolved azimuth.
        let env = resolve_spatial(
            mk_source([10.0, 10.0]),
            mk_listener_facing_east(),
            Medium::Air,
            &[],
            ReverbProfile::open_outdoor(),
        );
        assert!((env.azimuth_rad - std::f32::consts::FRAC_PI_4).abs() < 1e-3);
    }

    #[test]
    fn vacuum_caption_parity_acc_a_floor() {
        // Spec § "Vacuum blocks audio entirely (DR-014 vacuum_no_voice)":
        //   listener inside a sealed suit in vacuum
        //   And a metal pipe shears off 8 m away in the same vacuum field
        //   When the metal-bend SFX fires
        //   Then medium_at returns medium="vacuum"
        //   And SpatialEnvelope.gain == 0
        //   And no waveform reaches the listener's headphones
        //   And the caption "PIPE SHEAR — NE 8 m" still surfaces (ACC-A parity)
        let listener = ListenerContext {
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            facing_rad: 0.0,
            room_id: None,
        };
        let source = SourceContext {
            position: [5.66, 5.66], // NE, ~8 m away.
            velocity: [0.0, 0.0],
            base_gain: 1.0,
            propagation_range_m: 100.0,
            room_id: None,
        };
        let env = resolve_spatial(source, listener, Medium::Vacuum, &[], ReverbProfile::open_outdoor());
        assert!(env.gain.abs() < 1e-6, "vacuum must produce gain == 0");
        assert_eq!(env.medium_filter.medium, Medium::Vacuum);
        // ACC-A parity: the direction string is independent of gain — captions still surface.
        assert_eq!(env.direction, DirectionString::Ne);
        assert!((env.distance_m - 8.005).abs() < 0.05);
    }

    #[test]
    fn determinism_across_30_sfx_emit_positions_byte_identical() {
        // Spec § "Determinism across HRTF + reverb + occlusion resolution":
        //   two engines with the same seed + identical scenario (player + 30 SFX emit positions)
        //   When 600 ticks of audio resolution elapse
        //   Then per-tick audio.spatial_resolved + audio.reverb_applied + audio.occluded +
        //   audio.doppler_shifted event streams are byte-identical.
        //
        // We exercise the determinism contract at the pure-math layer: 30
        // deterministic source positions resolved against the same listener
        // must produce byte-identical SpatialEnvelope outputs across two
        // runs.
        let make_envelopes = || {
            let listener = ListenerContext {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                facing_rad: 0.0,
                room_id: Some(7),
            };
            (0..30)
                .map(|i| {
                    let theta = i as f32 * 0.21;
                    let r = 5.0 + (i as f32) * 0.7;
                    let pos = [r * theta.cos(), r * theta.sin()];
                    let vel = [(i as f32) * 1.3, -(i as f32) * 0.7];
                    let source = SourceContext {
                        position: pos,
                        velocity: vel,
                        base_gain: 1.0,
                        propagation_range_m: 100.0,
                        room_id: Some(if i % 2 == 0 { 7 } else { 8 }),
                    };
                    resolve_spatial(source, listener, Medium::Air, &[], ReverbProfile::open_outdoor())
                })
                .collect::<Vec<_>>()
        };
        let a = make_envelopes();
        let b = make_envelopes();
        assert_eq!(a.len(), 30);
        for (i, (envelope_a, envelope_b)) in a.iter().zip(b.iter()).enumerate() {
            assert_eq!(envelope_a, envelope_b, "envelope {i} diverged across runs");
        }
    }

    #[test]
    fn determinism_30_source_serialization_is_byte_identical() {
        // Stronger determinism check: serialized JSON of all 30 envelopes must
        // be byte-for-byte identical across runs (covers any hidden non-canonical
        // ordering or floating-point representation drift in serde).
        let resolve_all = || {
            let listener = ListenerContext {
                position: [0.0, 0.0],
                velocity: [0.0, 0.0],
                facing_rad: 0.0,
                room_id: Some(7),
            };
            let mut out = Vec::new();
            for i in 0..30 {
                let theta = i as f32 * 0.21;
                let r = 5.0 + (i as f32) * 0.7;
                let pos = [r * theta.cos(), r * theta.sin()];
                let vel = [(i as f32) * 1.3, -(i as f32) * 0.7];
                let source = SourceContext {
                    position: pos,
                    velocity: vel,
                    base_gain: 1.0,
                    propagation_range_m: 100.0,
                    room_id: Some(if i % 2 == 0 { 7 } else { 8 }),
                };
                let env = resolve_spatial(source, listener, Medium::Air, &[], ReverbProfile::open_outdoor());
                out.push(serde_json::to_string(&env).expect("serialize"));
            }
            out
        };
        let a = resolve_all();
        let b = resolve_all();
        assert_eq!(a, b, "byte-identical determinism contract failed");
    }
}
