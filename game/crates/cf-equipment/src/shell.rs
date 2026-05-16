//! M6: shell casing ejection (cosmetic `MovableObject` per CCCP `Round::Shell`).

use serde::{Deserialize, Serialize};

/// One ejected shell casing.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShellEjection {
    /// Stable per-shell id (deterministic; engine increments).
    pub shell_id: u64,
    /// World position of muzzle/ejection port.
    pub origin_x: f32,
    pub origin_y: f32,
    /// Initial velocity (units / s).
    pub velocity_x: f32,
    pub velocity_y: f32,
    /// Time-to-live in seconds (auto-despawn).
    pub lifetime_seconds: f32,
    /// Discriminator for renderer (rifle / pistol / shotgun shell visuals).
    pub kind: ShellKind,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShellKind {
    Rifle = 0,
    Pistol = 1,
    Shotgun = 2,
    Sniper = 3,
    Smg = 4,
}

impl ShellKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ShellKind::Rifle => "rifle",
            ShellKind::Pistol => "pistol",
            ShellKind::Shotgun => "shotgun",
            ShellKind::Sniper => "sniper",
            ShellKind::Smg => "smg",
        }
    }
}

impl ShellEjection {
    /// Default shell parameters for a given weapon kind.
    pub fn default_for(kind: ShellKind, shell_id: u64, origin_x: f32, origin_y: f32, facing_sign: f32) -> Self {
        let (vx_mag, vy_mag, lifetime) = match kind {
            ShellKind::Rifle => (35.0, 25.0, 1.5),
            ShellKind::Pistol => (28.0, 20.0, 1.2),
            ShellKind::Shotgun => (50.0, 35.0, 2.0),
            ShellKind::Sniper => (40.0, 30.0, 1.8),
            ShellKind::Smg => (32.0, 22.0, 1.3),
        };
        Self {
            shell_id,
            origin_x,
            origin_y,
            velocity_x: -vx_mag * facing_sign,
            velocity_y: vy_mag,
            lifetime_seconds: lifetime,
            kind,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_ejects_opposite_facing() {
        let s = ShellEjection::default_for(ShellKind::Rifle, 1, 0.0, 0.0, 1.0);
        assert!(s.velocity_x < 0.0);
        let s_left = ShellEjection::default_for(ShellKind::Rifle, 2, 0.0, 0.0, -1.0);
        assert!(s_left.velocity_x > 0.0);
    }

    #[test]
    fn shotgun_shell_louder_arc() {
        let r = ShellEjection::default_for(ShellKind::Rifle, 1, 0.0, 0.0, 1.0);
        let s = ShellEjection::default_for(ShellKind::Shotgun, 2, 0.0, 0.0, 1.0);
        assert!(s.velocity_y > r.velocity_y);
    }
}
