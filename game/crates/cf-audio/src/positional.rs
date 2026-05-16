//! **M12A** § 3D positional audio per M19 atmospherics integration.
//!
//! Per spec acceptance criterion:
//!
//! ```text
//! Scenario: 3D positional audio per M19 integration
//!   Given a gunshot at position (X, Y)
//!   When the player listens from position (X', Y')
//!   Then cf-audio applies distance attenuation
//!   And per-M19 atmospheric occlusion (walls block partial; vacuum blocks fully)
//!   And captions show correct direction-indicator
//! ```
//!
//! This module ships:
//! - Distance attenuation: inverse-square with a per-SFX `propagation_range_m`
//!   floor below which the sound cuts off cleanly (no audible whisper-trail
//!   beyond the radius).
//! - Atmospheric occlusion: linear/exponential curve based on the M19
//!   atmospheric density between source + listener.
//! - Direction indicator (8-way compass) for caption rendering.
//!
//! Steam Audio HRTF integration is OUT OF SCOPE (M36A/M48A per spec).

use serde::{Deserialize, Serialize};

/// Compass bearing for the audio-direction caption indicator.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum AudioDirection {
    /// Due north of the listener (`+Y` world axis).
    North,
    /// Between north and east — upper-right quadrant.
    NorthEast,
    /// Due east of the listener (`+X` world axis).
    East,
    /// Between south and east — lower-right quadrant.
    SouthEast,
    /// Due south of the listener (`-Y` world axis).
    South,
    /// Between south and west — lower-left quadrant.
    SouthWest,
    /// Due west of the listener (`-X` world axis).
    West,
    /// Between north and west — upper-left quadrant.
    NorthWest,
    /// Source is within `DIRECTION_HERE_THRESHOLD_M` of the listener — no
    /// meaningful direction.
    Here,
}

impl AudioDirection {
    /// 8-way cardinal label for the caption template.
    pub fn label(self) -> &'static str {
        match self {
            AudioDirection::North => "north",
            AudioDirection::NorthEast => "northeast",
            AudioDirection::East => "east",
            AudioDirection::SouthEast => "southeast",
            AudioDirection::South => "south",
            AudioDirection::SouthWest => "southwest",
            AudioDirection::West => "west",
            AudioDirection::NorthWest => "northwest",
            AudioDirection::Here => "here",
        }
    }

    /// Parse from snake_case (case-insensitive).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<AudioDirection> {
        Some(match s.to_ascii_lowercase().as_str() {
            "north" | "n" => AudioDirection::North,
            "northeast" | "ne" => AudioDirection::NorthEast,
            "east" | "e" => AudioDirection::East,
            "southeast" | "se" => AudioDirection::SouthEast,
            "south" | "s" => AudioDirection::South,
            "southwest" | "sw" => AudioDirection::SouthWest,
            "west" | "w" => AudioDirection::West,
            "northwest" | "nw" => AudioDirection::NorthWest,
            "here" => AudioDirection::Here,
            _ => return None,
        })
    }
}

/// Minimum distance to assign a directional indicator (< this = `Here`).
pub const DIRECTION_HERE_THRESHOLD_M: f32 = 0.5;

/// Compute the 8-way compass bearing of `source` relative to `listener`.
/// World coordinates: `+X = east`, `+Y = north`. Returns `Here` when the
/// source is within [`DIRECTION_HERE_THRESHOLD_M`].
#[must_use]
pub fn direction_of(source: [f32; 2], listener: [f32; 2]) -> AudioDirection {
    let dx = source[0] - listener[0];
    let dy = source[1] - listener[1];
    let dist = (dx * dx + dy * dy).sqrt();
    if dist < DIRECTION_HERE_THRESHOLD_M {
        return AudioDirection::Here;
    }
    let angle_deg = dy.atan2(dx).to_degrees();
    // Convert mathematical angle (0° = east, CCW) to compass angle (0° = north, CW).
    let compass = ((90.0 - angle_deg).rem_euclid(360.0)) as i32;
    match compass {
        0..=22 | 338..=359 => AudioDirection::North,
        23..=67 => AudioDirection::NorthEast,
        68..=112 => AudioDirection::East,
        113..=157 => AudioDirection::SouthEast,
        158..=202 => AudioDirection::South,
        203..=247 => AudioDirection::SouthWest,
        248..=292 => AudioDirection::West,
        293..=337 => AudioDirection::NorthWest,
        _ => AudioDirection::North,
    }
}

/// **M12A** § Distance attenuation. Returns the per-source volume gain
/// `[0, 1]` based on the Euclidean distance + the SFX's declared
/// `propagation_range_m` falloff. Inverse-square curve clamped to the
/// `[0, 1]` band; values past `propagation_range_m` are zero (the
/// sound cleanly cuts off — no whisper-trail beyond the audible
/// radius).
#[must_use]
pub fn distance_attenuation(distance_m: f32, propagation_range_m: f32) -> f32 {
    if distance_m < 0.0 {
        return 1.0;
    }
    if propagation_range_m <= 0.0 {
        return if distance_m == 0.0 { 1.0 } else { 0.0 };
    }
    if distance_m >= propagation_range_m {
        return 0.0;
    }
    let near = (propagation_range_m * 0.1).max(0.1);
    if distance_m <= near {
        return 1.0;
    }
    let normalized = (distance_m - near) / (propagation_range_m - near);
    // Inverse-square fall-off mapped to [1.0..=0.0] over the active band.
    let falloff = 1.0 - normalized.powi(2);
    falloff.clamp(0.0, 1.0)
}

/// **M12A** § Atmospheric occlusion. Walls + dense atmosphere attenuate
/// audio; vacuum completely silences it (sound doesn't propagate in
/// space). `atmosphere_density` is the M19 per-tile density value (0.0
/// = vacuum, 1.0 = sea-level); `wall_thickness_m` is the cumulative
/// solid material between source + listener.
///
/// Per spec: "walls block partial; vacuum blocks fully".
#[must_use]
pub fn occlusion_attenuation(atmosphere_density: f32, wall_thickness_m: f32) -> f32 {
    let atmo = atmosphere_density.clamp(0.0, 1.0);
    if atmo < 0.01 {
        // Vacuum — no sound propagates.
        return 0.0;
    }
    let wall_atten = (1.0 - (wall_thickness_m * 0.5).clamp(0.0, 1.0)).max(0.0);
    atmo.sqrt() * wall_atten
}

/// **M12A** § Combined per-source gain — distance × occlusion + a
/// hard-clamp to `[0, 1]`. Returns `0.0` for sources outside the
/// propagation range OR in vacuum.
#[must_use]
pub fn source_gain(
    source: [f32; 2],
    listener: [f32; 2],
    propagation_range_m: f32,
    atmosphere_density: f32,
    wall_thickness_m: f32,
) -> f32 {
    let dx = source[0] - listener[0];
    let dy = source[1] - listener[1];
    let dist = (dx * dx + dy * dy).sqrt();
    let attn = distance_attenuation(dist, propagation_range_m);
    let occ = occlusion_attenuation(atmosphere_density, wall_thickness_m);
    (attn * occ).clamp(0.0, 1.0)
}

/// **M12A** § Voice prioritization — spec § Pitfalls: "Spam-event audio
/// pile-up: 50 actors firing simultaneously = 50 gunshots playing at
/// once. Audio engine must limit voice count + prioritize (closest
/// first)".
///
/// Returns the indices of the top-N sources sorted by descending gain
/// (closest + loudest first). cf-app's playback dispatcher uses this
/// to cap the simultaneous-voice count.
#[must_use]
pub fn top_n_by_gain(gains: &[f32], n: usize) -> Vec<usize> {
    let mut indexed: Vec<(usize, f32)> = gains.iter().copied().enumerate().collect();
    indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    indexed.into_iter().take(n).map(|(i, _)| i).collect()
}

/// Maximum simultaneous voices played by the cf-audio mixer. Lower
/// values reduce CPU overhead; Steam Deck T-PERF caps at 32.
pub const MAX_SIMULTANEOUS_VOICES: usize = 32;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_of_returns_north_for_due_north() {
        let dir = direction_of([0.0, 10.0], [0.0, 0.0]);
        assert_eq!(dir, AudioDirection::North);
    }

    #[test]
    fn direction_of_returns_east_for_due_east() {
        let dir = direction_of([10.0, 0.0], [0.0, 0.0]);
        assert_eq!(dir, AudioDirection::East);
    }

    #[test]
    fn direction_of_returns_south_for_due_south() {
        let dir = direction_of([0.0, -10.0], [0.0, 0.0]);
        assert_eq!(dir, AudioDirection::South);
    }

    #[test]
    fn direction_of_returns_west_for_due_west() {
        let dir = direction_of([-10.0, 0.0], [0.0, 0.0]);
        assert_eq!(dir, AudioDirection::West);
    }

    #[test]
    fn direction_of_returns_here_for_close_source() {
        let dir = direction_of([0.1, 0.1], [0.0, 0.0]);
        assert_eq!(dir, AudioDirection::Here);
    }

    #[test]
    fn direction_of_handles_intermediate_compass_points() {
        let ne = direction_of([10.0, 10.0], [0.0, 0.0]);
        assert_eq!(ne, AudioDirection::NorthEast);
        let nw = direction_of([-10.0, 10.0], [0.0, 0.0]);
        assert_eq!(nw, AudioDirection::NorthWest);
        let sw = direction_of([-10.0, -10.0], [0.0, 0.0]);
        assert_eq!(sw, AudioDirection::SouthWest);
        let se = direction_of([10.0, -10.0], [0.0, 0.0]);
        assert_eq!(se, AudioDirection::SouthEast);
    }

    #[test]
    fn audio_direction_round_trips_through_str() {
        for d in [
            AudioDirection::North,
            AudioDirection::NorthEast,
            AudioDirection::East,
            AudioDirection::SouthEast,
            AudioDirection::South,
            AudioDirection::SouthWest,
            AudioDirection::West,
            AudioDirection::NorthWest,
            AudioDirection::Here,
        ] {
            assert_eq!(AudioDirection::from_str(d.label()), Some(d));
        }
        assert!(AudioDirection::from_str("garbage").is_none());
    }

    #[test]
    fn distance_attenuation_peaks_at_near_field() {
        assert!((distance_attenuation(0.0, 50.0) - 1.0).abs() < 1e-4);
        assert!((distance_attenuation(1.0, 50.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn distance_attenuation_zero_beyond_range() {
        assert!(distance_attenuation(60.0, 50.0).abs() < 1e-4);
        assert!(distance_attenuation(50.0, 50.0).abs() < 1e-4);
    }

    #[test]
    fn distance_attenuation_decreases_with_distance() {
        let a = distance_attenuation(10.0, 50.0);
        let b = distance_attenuation(30.0, 50.0);
        let c = distance_attenuation(48.0, 50.0);
        assert!(a > b);
        assert!(b > c);
        assert!(c > 0.0);
    }

    #[test]
    fn occlusion_zero_in_vacuum() {
        assert!(occlusion_attenuation(0.0, 0.0).abs() < 1e-4);
        assert!(occlusion_attenuation(0.005, 0.0).abs() < 1e-4);
    }

    #[test]
    fn occlusion_full_in_atmosphere_without_walls() {
        assert!((occlusion_attenuation(1.0, 0.0) - 1.0).abs() < 1e-4);
    }

    #[test]
    fn occlusion_partial_through_walls() {
        let no_wall = occlusion_attenuation(1.0, 0.0);
        let half_wall = occlusion_attenuation(1.0, 1.0);
        let full_wall = occlusion_attenuation(1.0, 2.0);
        assert!(no_wall > half_wall);
        assert!(half_wall > full_wall);
        assert!(full_wall.abs() < 1e-4);
    }

    #[test]
    fn source_gain_combines_distance_and_occlusion() {
        let listener = [0.0, 0.0];
        let close = [1.0, 0.0];
        let far = [40.0, 0.0];
        let gain_close = source_gain(close, listener, 50.0, 1.0, 0.0);
        let gain_far = source_gain(far, listener, 50.0, 1.0, 0.0);
        assert!(gain_close > gain_far);
        // Vacuum kills the signal regardless of distance.
        let gain_vacuum = source_gain(close, listener, 50.0, 0.0, 0.0);
        assert!(gain_vacuum.abs() < 1e-4);
    }

    #[test]
    fn top_n_by_gain_sorts_descending() {
        let gains = vec![0.1, 0.9, 0.3, 0.7, 0.2];
        let top = top_n_by_gain(&gains, 3);
        assert_eq!(top, vec![1, 3, 2]);
    }

    #[test]
    fn top_n_by_gain_handles_empty_input() {
        let top = top_n_by_gain(&[], 5);
        assert!(top.is_empty());
    }
}
