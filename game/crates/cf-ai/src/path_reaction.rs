//! M9 — Path reaction: react to terrain dirty regions during a guard's pursuit.
//!
//! Spec § Reactive guard targeting + path reaction — when
//! `terrain.terrain_dirty_region_batch` fires inside the guard's current
//! path, the AI must respond within 60 ticks with a recovery action.
//! Producer: this module computes the next action (Reroute / FireOverObstacle
//! / GiveUpAndFireFromHere) from inputs (path, dirty rect bbox, guard's
//! current position). The engine fires `ai.path_invalidated` +
//! `ai.recovery_action` events with the result.

/// Recovery action picked by the AI on path invalidation. Maps to the
/// `ai.recovery_action` event's `action` field.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum RecoveryAction {
    /// Compute a new path around the obstacle and continue pursuing.
    Reroute,
    /// Fire over the obstacle from the current position (e.g. arched
    /// projectile trajectory) without moving.
    FireOverObstacle,
    /// Give up moving; stand and fire from here.
    GiveUpAndFireFromHere,
}

impl RecoveryAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecoveryAction::Reroute => "reroute",
            RecoveryAction::FireOverObstacle => "fire_over_obstacle",
            RecoveryAction::GiveUpAndFireFromHere => "give_up_and_fire_from_here",
        }
    }
}

/// Pure-function decision: pick a recovery action from inputs. The engine
/// passes the dirty-rect intersection ratio (`fraction_of_path_dirty` in
/// `[0, 1]`), whether the guard has LOS to the target, and the remaining
/// path length. Mirrors the M9 spec's "reroute / fire_over / give_up"
/// outcomes.
#[must_use]
pub fn pick_recovery_action(
    fraction_of_path_dirty: f32,
    has_los_to_target: bool,
    remaining_path_length: f32,
) -> RecoveryAction {
    let dirty = fraction_of_path_dirty.clamp(0.0, 1.0);
    if has_los_to_target && dirty < 0.5 {
        RecoveryAction::FireOverObstacle
    } else if dirty >= 0.9 || remaining_path_length <= 0.0 {
        RecoveryAction::GiveUpAndFireFromHere
    } else {
        RecoveryAction::Reroute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_dirt_fires_over_when_has_los() {
        let a = pick_recovery_action(0.2, true, 30.0);
        assert_eq!(a, RecoveryAction::FireOverObstacle);
    }

    #[test]
    fn heavy_dirt_gives_up() {
        let a = pick_recovery_action(0.95, false, 30.0);
        assert_eq!(a, RecoveryAction::GiveUpAndFireFromHere);
    }

    #[test]
    fn moderate_dirt_reroutes() {
        let a = pick_recovery_action(0.5, false, 30.0);
        assert_eq!(a, RecoveryAction::Reroute);
    }

    #[test]
    fn empty_path_gives_up() {
        let a = pick_recovery_action(0.3, false, 0.0);
        assert_eq!(a, RecoveryAction::GiveUpAndFireFromHere);
    }
}
