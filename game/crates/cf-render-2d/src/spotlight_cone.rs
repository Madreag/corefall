//! M9C: spotlight cone-of-light rendering.
//!
//! Spec §"Watchtower (3 height tiers) + spotlight + observation post"
//! table row 4 + §"Spotlight reveals concealed actor in cone" Gherkin
//! scenario:
//!
//! > Spotlight: nighttime gameplay (M31 day/night cycle when shipped);
//! > cone-of-light reveals concealed/prone enemies. Actor in cone
//! > receives "illuminated" status.
//!
//! Spec range: 24 tiles per [`SPOTLIGHT_RANGE_TILES`]; 45° cone half-
//! angle authored on the spotlight RON when modders extend.
//!
//! The kernel exposes pure geometry helpers so cf-app's renderer can
//! decide which screen-space tiles fall inside the cone without
//! linking the M9C kernel into the Bevy render graph.
//!
//! VAL-M9C-051 lands here.

use serde::{Deserialize, Serialize};

/// Spec §"Watchtower ... spotlight" table row 4: spotlight cone is
/// 24 tiles long.
pub const SPOTLIGHT_RANGE_TILES: u32 = 24;
/// Default half-angle of the cone in degrees (45° per spec § Notes
/// for the implementer).
pub const SPOTLIGHT_HALF_ANGLE_DEGREES: f32 = 22.5;
/// Spec §"Flashbang dazzles spotlight for 12 seconds" Gherkin scenario:
/// the spotlight is offline for 12 seconds after `spotlight_dazzled`.
pub const SPOTLIGHT_DAZZLE_DURATION_SECONDS: f32 = 12.0;

/// Live spotlight state mirrored from the engine into the renderer.
/// The cone is described by (origin, aim_radians, range_tiles,
/// half_angle_degrees). cf-app's render bridge writes this every
/// frame from the engine snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpotlightCone {
    /// World-space tile origin of the spotlight emitter.
    pub origin: (i32, i32),
    /// Aim direction in radians (0 = +X axis, π/2 = +Y axis).
    pub aim_radians: f32,
    /// Cone reach in tiles. Spec default = [`SPOTLIGHT_RANGE_TILES`].
    pub range_tiles: u32,
    /// Cone half-angle in degrees (full cone is 2× this value).
    /// Spec default = [`SPOTLIGHT_HALF_ANGLE_DEGREES`].
    pub half_angle_degrees: f32,
    /// Is the spotlight currently emitting light? `false` while
    /// `spotlight_dazzled` is in effect (12s post-flashbang).
    pub online: bool,
}

impl SpotlightCone {
    /// Default-cone factory matching the spec table.
    #[must_use]
    pub fn new_default(origin: (i32, i32), aim_radians: f32) -> Self {
        Self {
            origin,
            aim_radians,
            range_tiles: SPOTLIGHT_RANGE_TILES,
            half_angle_degrees: SPOTLIGHT_HALF_ANGLE_DEGREES,
            online: true,
        }
    }

    /// VAL-M9C-024 / VAL-M9C-023 helper: is the supplied target tile
    /// inside the cone? Returns `false` whenever the cone is offline,
    /// outside the range, or outside the half-angle envelope.
    #[must_use]
    pub fn contains_tile(&self, target: (i32, i32)) -> bool {
        if !self.online {
            return false;
        }
        let dx = (target.0 - self.origin.0) as f32;
        let dy = (target.1 - self.origin.1) as f32;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance > self.range_tiles as f32 {
            return false;
        }
        if distance < 0.5 {
            // Sitting on the origin: always lit (avoid atan2(0,0)).
            return true;
        }
        let angle = dy.atan2(dx);
        let mut diff = angle - self.aim_radians;
        // Normalise to [-π, π].
        while diff > std::f32::consts::PI {
            diff -= 2.0 * std::f32::consts::PI;
        }
        while diff < -std::f32::consts::PI {
            diff += 2.0 * std::f32::consts::PI;
        }
        let half_radians = self.half_angle_degrees.to_radians();
        diff.abs() <= half_radians
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spotlight_default_uses_spec_table_values() {
        let cone = SpotlightCone::new_default((0, 0), 0.0);
        assert_eq!(cone.range_tiles, SPOTLIGHT_RANGE_TILES);
        assert!((cone.half_angle_degrees - SPOTLIGHT_HALF_ANGLE_DEGREES).abs() < f32::EPSILON);
        assert!(cone.online);
    }

    /// Tile directly along the aim axis at half-range is lit.
    #[test]
    fn spotlight_cone_lights_tile_along_axis() {
        let cone = SpotlightCone::new_default((0, 0), 0.0);
        assert!(cone.contains_tile((10, 0)));
    }

    /// Tile beyond range is NOT lit.
    #[test]
    fn spotlight_cone_misses_tile_beyond_range() {
        let cone = SpotlightCone::new_default((0, 0), 0.0);
        assert!(!cone.contains_tile((25, 0)));
    }

    /// Tile outside the half-angle envelope is NOT lit.
    #[test]
    fn spotlight_cone_misses_tile_off_axis() {
        let cone = SpotlightCone::new_default((0, 0), 0.0);
        // Tile at 90° from the aim direction (orthogonal) — way
        // outside the 22.5° half-angle.
        assert!(!cone.contains_tile((0, 10)));
    }

    #[test]
    fn spotlight_cone_dazzled_lights_nothing() {
        let mut cone = SpotlightCone::new_default((0, 0), 0.0);
        cone.online = false;
        assert!(!cone.contains_tile((10, 0)));
    }

    /// Origin tile is always lit (avoids divide-by-zero / atan2(0,0)
    /// undefined behavior).
    #[test]
    fn spotlight_cone_lights_origin() {
        let cone = SpotlightCone::new_default((5, 5), 0.0);
        assert!(cone.contains_tile((5, 5)));
    }
}
