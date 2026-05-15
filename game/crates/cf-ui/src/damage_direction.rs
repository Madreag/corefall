//! M8 — Damage direction indicator (screen-edge red arrow).
//!
//! Per spec § UX widgets: red arrow pointing toward damage source; fades
//! after 1s; `reduce_motion`: instant on/off (no fade).

use bevy::prelude::*;

/// Spec-mandated default fade-out duration in ms.
pub const DEFAULT_FADE_MS: u32 = 1000;

/// One damage-direction marker.
#[derive(Debug, Clone, PartialEq)]
pub struct DamageDirectionMarker {
    /// Bearing in degrees (0..360, 0 = north).
    pub bearing_degrees: f32,
    /// Remaining fade time in ms (counts down to 0).
    pub remaining_ms: u32,
}

/// Damage-direction widget Bevy resource.
#[derive(Resource, Debug, Clone, Default)]
pub struct DamageDirectionState {
    /// Active markers (newest first).
    pub markers: Vec<DamageDirectionMarker>,
    /// When true, markers appear instantly without fade animation.
    pub reduce_motion: bool,
    /// Whether the indicator is enabled (Settings.damage_direction_enabled).
    pub enabled: bool,
}

impl DamageDirectionState {
    /// Push a fresh marker at `bearing_degrees`. Honors reduce_motion by
    /// setting an instant on/off duration.
    pub fn push(&mut self, bearing_degrees: f32) {
        let remaining_ms = if self.reduce_motion { 200 } else { DEFAULT_FADE_MS };
        self.markers.push(DamageDirectionMarker {
            bearing_degrees,
            remaining_ms,
        });
    }

    /// Decay every marker by `dt_ms`, dropping any whose remaining_ms is
    /// zero. Returns the number of markers still active.
    pub fn tick(&mut self, dt_ms: u32) -> usize {
        for m in &mut self.markers {
            m.remaining_ms = m.remaining_ms.saturating_sub(dt_ms);
        }
        self.markers.retain(|m| m.remaining_ms > 0);
        self.markers.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_uses_default_fade() {
        let mut s = DamageDirectionState::default();
        s.push(45.0);
        assert_eq!(s.markers[0].remaining_ms, DEFAULT_FADE_MS);
    }

    #[test]
    fn push_with_reduce_motion_uses_instant_duration() {
        let mut s = DamageDirectionState {
            reduce_motion: true,
            ..DamageDirectionState::default()
        };
        s.push(90.0);
        assert_eq!(s.markers[0].remaining_ms, 200);
    }

    #[test]
    fn tick_drops_expired_markers() {
        let mut s = DamageDirectionState::default();
        s.push(0.0);
        s.tick(2000);
        assert!(s.markers.is_empty());
    }
}
