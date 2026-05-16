//! **M14**: full swept-collision priority queue per CCCP `Atom::Travel` +
//! `MovableObject::CollideAtPoint`.
//!
//! When a projectile traces a swept path across multiple actors in one
//! tick, every intersected AABB is collected as a [`SweptHitCandidate`]
//! and resolved in priority order via [`prioritize_swept_collisions`]:
//! - Closer hits first (lower `entry_t`).
//! - Tie-break on `target_id` (deterministic across all platforms).
//!
//! The resolved list carries the priority index + total so each emit site
//! can report `priority_index / priority_total` on the
//! `combat.swept_collision` event.
//!
//! All functions are pure / deterministic. No clock; no `thread_rng`.

use serde::{Deserialize, Serialize};

/// One actor-vs-projectile candidate the swept-collision broadphase
/// produced. Multiple candidates per tick get fed through
/// [`prioritize_swept_collisions`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SweptHitCandidate {
    /// Target actor id (u64).
    pub target_id: u64,
    /// Entry parameter in [0, 1] along the projectile's swept segment.
    /// 0.0 = projectile already inside the AABB at start; 1.0 = enters
    /// only at end of segment.
    pub entry_t: f32,
    /// Distance from the projectile's origin to the entry point (world
    /// units, e.g. pixels). Used for sharpness-decay accounting + the
    /// `combat.swept_collision.distance_traveled` payload field.
    pub distance_traveled: f32,
    /// World-space entry point [x, y].
    pub entry_point: [f32; 2],
    /// World-space ray origin [x, y] (the projectile's start position this
    /// tick).
    pub ray_origin: [f32; 2],
    /// Normalized ray direction [x, y].
    pub ray_direction: [f32; 2],
}

/// Resolved swept-collision hit. `priority_index` is 0-based; smaller =
/// resolved first.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SweptHitResolved {
    pub target_id: u64,
    pub entry_t: f32,
    pub distance_traveled: f32,
    pub entry_point: [f32; 2],
    pub ray_origin: [f32; 2],
    pub ray_direction: [f32; 2],
    pub priority_index: u32,
    pub priority_total: u32,
}

/// **M14**: deterministically resolve a slate of candidate swept hits into
/// a priority-ordered slice. Closer (lower `entry_t`) hits resolve first;
/// ties break on `target_id` ascending.
///
/// `priority_index` / `priority_total` are filled in per spec § Acceptance
/// criteria.
#[must_use]
pub fn prioritize_swept_collisions(mut candidates: Vec<SweptHitCandidate>) -> Vec<SweptHitResolved> {
    candidates.sort_by(|a, b| {
        a.entry_t
            .partial_cmp(&b.entry_t)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.target_id.cmp(&b.target_id))
    });
    let total = candidates.len() as u32;
    candidates
        .into_iter()
        .enumerate()
        .map(|(i, c)| SweptHitResolved {
            target_id: c.target_id,
            entry_t: c.entry_t,
            distance_traveled: c.distance_traveled,
            entry_point: c.entry_point,
            ray_origin: c.ray_origin,
            ray_direction: c.ray_direction,
            priority_index: i as u32,
            priority_total: total,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(target_id: u64, entry_t: f32) -> SweptHitCandidate {
        SweptHitCandidate {
            target_id,
            entry_t,
            distance_traveled: entry_t * 100.0,
            entry_point: [entry_t * 100.0, 0.0],
            ray_origin: [0.0, 0.0],
            ray_direction: [1.0, 0.0],
        }
    }

    #[test]
    fn closer_hits_resolve_first() {
        let candidates = vec![cand(2, 0.8), cand(1, 0.2), cand(3, 0.5)];
        let resolved = prioritize_swept_collisions(candidates);
        assert_eq!(resolved[0].target_id, 1);
        assert_eq!(resolved[1].target_id, 3);
        assert_eq!(resolved[2].target_id, 2);
    }

    #[test]
    fn ties_break_on_target_id() {
        let candidates = vec![cand(5, 0.5), cand(2, 0.5), cand(7, 0.5)];
        let resolved = prioritize_swept_collisions(candidates);
        assert_eq!(resolved[0].target_id, 2);
        assert_eq!(resolved[1].target_id, 5);
        assert_eq!(resolved[2].target_id, 7);
    }

    #[test]
    fn priority_index_and_total_populated() {
        let candidates = vec![cand(2, 0.8), cand(1, 0.2)];
        let resolved = prioritize_swept_collisions(candidates);
        assert_eq!(resolved[0].priority_index, 0);
        assert_eq!(resolved[0].priority_total, 2);
        assert_eq!(resolved[1].priority_index, 1);
        assert_eq!(resolved[1].priority_total, 2);
    }

    #[test]
    fn empty_input_returns_empty() {
        let resolved = prioritize_swept_collisions(Vec::new());
        assert!(resolved.is_empty());
    }

    #[test]
    fn deterministic_across_runs() {
        let candidates = vec![cand(5, 0.5), cand(2, 0.5), cand(7, 0.5), cand(1, 0.3)];
        let a = prioritize_swept_collisions(candidates.clone());
        let b = prioritize_swept_collisions(candidates);
        assert_eq!(a, b);
    }
}
