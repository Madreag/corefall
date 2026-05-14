//! M6: grenade registry (4 grenades).
//!
//! Per spec § "4 grenade types with throw-arc preview":
//! - Frag (5 s fuse, radius damage)
//! - Smoke (5 s fuse, smoke hazard tile spawn)
//! - Flash (1.5 s fuse, deafen + blind)
//! - Stick (adheres to actor/surface, 4 s fuse)

pub mod flash;
pub mod frag;
pub mod smoke;
pub mod stick;

use serde::{Deserialize, Serialize};

pub const FRAG_M6_DEFAULT_ID: &str = "grenade_frag_m6";
pub const SMOKE_M6_DEFAULT_ID: &str = "grenade_smoke_m6";
pub const FLASH_M6_DEFAULT_ID: &str = "grenade_flash_m6";
pub const STICK_M6_DEFAULT_ID: &str = "grenade_stick_m6";

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GrenadeKind {
    Frag = 0,
    Smoke = 1,
    Flash = 2,
    Stick = 3,
}

impl GrenadeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            GrenadeKind::Frag => "frag",
            GrenadeKind::Smoke => "smoke",
            GrenadeKind::Flash => "flash",
            GrenadeKind::Stick => "stick",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrenadePreset {
    pub id: String,
    pub display_name: String,
    pub kind: GrenadeKind,
    pub fuse_seconds: f32,
    pub radius: f32,
    pub damage_at_center: f32,
    /// Adheres on impact (Stick semantics).
    pub adhesive: bool,
    /// True for hazard-spawning grenades (Smoke).
    pub spawns_hazard: bool,
    /// True for vision-disabling grenades (Flash).
    pub vision_disrupt: bool,
    /// Mass in kg.
    pub mass_kg: f32,
}

#[must_use]
pub fn m6_grenade_presets() -> Vec<GrenadePreset> {
    vec![
        frag::frag_m6_default(),
        smoke::smoke_m6_default(),
        flash::flash_m6_default(),
        stick::stick_m6_default(),
    ]
}

/// Cook a grenade — reduce the remaining fuse by `cook_seconds`. Returns the
/// new remaining fuse (clamped to >= 0).
#[must_use]
pub fn cook_grenade(initial_fuse: f32, cook_seconds: f32) -> f32 {
    if !initial_fuse.is_finite() || !cook_seconds.is_finite() {
        return 0.0;
    }
    (initial_fuse - cook_seconds.max(0.0)).max(0.0)
}

/// Compute the throw arc apex / impact (deterministic parabola) for HUD preview.
/// Returns a series of (x, y) sample points along the arc.
#[must_use]
pub fn arc_preview_samples(
    origin: (f32, f32),
    throw_velocity: (f32, f32),
    gravity_per_s2: f32,
    duration_seconds: f32,
    sample_count: usize,
) -> Vec<(f32, f32)> {
    if !origin.0.is_finite()
        || !origin.1.is_finite()
        || !throw_velocity.0.is_finite()
        || !throw_velocity.1.is_finite()
        || !gravity_per_s2.is_finite()
        || !duration_seconds.is_finite()
        || duration_seconds <= 0.0
        || sample_count == 0
    {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(sample_count);
    for i in 0..sample_count {
        let t = duration_seconds * (i as f32 / (sample_count - 1).max(1) as f32);
        let x = origin.0 + throw_velocity.0 * t;
        let y = origin.1 + throw_velocity.1 * t - 0.5 * gravity_per_s2 * t * t;
        out.push((x, y));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_four_kinds() {
        let v = m6_grenade_presets();
        assert_eq!(v.len(), 4);
        assert!(v.iter().any(|g| g.kind == GrenadeKind::Frag));
        assert!(v.iter().any(|g| g.kind == GrenadeKind::Smoke));
        assert!(v.iter().any(|g| g.kind == GrenadeKind::Flash));
        assert!(v.iter().any(|g| g.kind == GrenadeKind::Stick));
    }

    #[test]
    fn cook_reduces_fuse() {
        assert!((cook_grenade(5.0, 2.0) - 3.0).abs() < 1e-3);
    }

    #[test]
    fn cook_floors_at_zero() {
        assert_eq!(cook_grenade(2.0, 5.0), 0.0);
    }

    #[test]
    fn arc_preview_returns_samples() {
        let pts = arc_preview_samples((0.0, 0.0), (10.0, 20.0), 9.8, 2.0, 5);
        assert_eq!(pts.len(), 5);
    }

    #[test]
    fn nan_throw_returns_empty() {
        let pts = arc_preview_samples((0.0, 0.0), (f32::NAN, 0.0), 9.8, 2.0, 5);
        assert!(pts.is_empty());
    }
}
