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
//! **M14D** extends the prioritization surface to accept mixed actor /
//! projectile-pair candidates via [`SweptCandidateKind`] +
//! [`prioritize_mixed_swept_candidates`]. The M14D
//! `cf-physics::projectile` pair kernel emits its candidates with the
//! [`SweptCandidateKind::ProjectilePair`] discriminator so the engine's
//! TOI-ordered priority queue can interleave them with actor-vs-
//! projectile hits without losing the deterministic ordering contract.
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

    /// **VAL-M14D-013**: the mixed prioritizer accepts a vector
    /// containing actor + projectile-pair candidates and orders them by
    /// TOI alongside each other. ProjectilePair entries survive the
    /// prioritization (none dropped).
    #[test]
    fn mixed_prioritizer_orders_actor_and_projectile_pair_by_toi() {
        let mut candidates = vec![
            SweptCandidateKind::Actor(cand(10, 0.8)),
            SweptCandidateKind::ProjectilePair(ProjectilePairKey {
                a_id: 1,
                b_id: 2,
                toi: 0.25,
            }),
            SweptCandidateKind::Actor(cand(11, 0.5)),
            SweptCandidateKind::ProjectilePair(ProjectilePairKey {
                a_id: 3,
                b_id: 4,
                toi: 0.10,
            }),
        ];
        let preserved_pair_count = candidates
            .iter()
            .filter(|c| matches!(c, SweptCandidateKind::ProjectilePair(_)))
            .count();
        let resolved = prioritize_mixed_swept_candidates(std::mem::take(&mut candidates));
        assert_eq!(resolved.len(), 4);
        // Pair entries must be preserved.
        let resolved_pairs = resolved
            .iter()
            .filter(|c| matches!(c, ResolvedSweptCandidate::ProjectilePair { .. }))
            .count();
        assert_eq!(resolved_pairs, preserved_pair_count);
        // TOI-ordered output.
        let tois: Vec<f32> = resolved
            .iter()
            .map(|c| match c {
                ResolvedSweptCandidate::Actor(a) => a.entry_t,
                ResolvedSweptCandidate::ProjectilePair { toi, .. } => *toi,
            })
            .collect();
        for w in tois.windows(2) {
            assert!(w[0] <= w[1], "TOI order broken: {tois:?}");
        }
    }

    /// **VAL-M14D-013**: the mixed prioritizer is deterministic across
    /// identical input twice.
    #[test]
    fn mixed_prioritizer_is_deterministic() {
        let input = || {
            vec![
                SweptCandidateKind::Actor(cand(5, 0.5)),
                SweptCandidateKind::ProjectilePair(ProjectilePairKey {
                    a_id: 1,
                    b_id: 7,
                    toi: 0.5,
                }),
                SweptCandidateKind::ProjectilePair(ProjectilePairKey {
                    a_id: 3,
                    b_id: 4,
                    toi: 0.5,
                }),
                SweptCandidateKind::Actor(cand(2, 0.5)),
            ]
        };
        let a = prioritize_mixed_swept_candidates(input());
        let b = prioritize_mixed_swept_candidates(input());
        assert_eq!(a, b);
    }
}

/// Carried as a payload on [`SweptCandidateKind::ProjectilePair`]. The
/// engine matches the pair back to the M14D `cf-physics::projectile`
/// kernel's contact list by `(a_id, b_id)`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProjectilePairKey {
    pub a_id: u64,
    pub b_id: u64,
    /// Time-of-impact in `[0, 1]` along this tick's swept window.
    pub toi: f32,
}

/// M14 default) or projectile-vs-projectile pair (the M14D extension).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SweptCandidateKind {
    Actor(SweptHitCandidate),
    ProjectilePair(ProjectilePairKey),
}

/// Mirrors the [`SweptHitResolved`] shape on the actor branch and
/// carries the pair key + the priority index/total on the pair branch.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ResolvedSweptCandidate {
    Actor(SweptHitResolved),
    ProjectilePair {
        a_id: u64,
        b_id: u64,
        toi: f32,
        priority_index: u32,
        priority_total: u32,
    },
}

/// actor + projectile-pair swept candidates into a priority-ordered
/// slice. Closer-first (lower TOI) wins; ties break on the candidate
/// kind's stable key (`target_id` for actor; `(a_id, b_id)` for pair).
///
/// ProjectilePair candidates are preserved (never dropped) — the pair
/// kernel's output is treated as a first-class slate alongside the
/// actor candidates so the engine's per-tick schedule can interleave
/// the two passes without losing TOI order.
#[must_use]
pub fn prioritize_mixed_swept_candidates(mut candidates: Vec<SweptCandidateKind>) -> Vec<ResolvedSweptCandidate> {
    candidates.sort_by(|a, b| {
        let toi_a = match a {
            SweptCandidateKind::Actor(c) => c.entry_t,
            SweptCandidateKind::ProjectilePair(k) => k.toi,
        };
        let toi_b = match b {
            SweptCandidateKind::Actor(c) => c.entry_t,
            SweptCandidateKind::ProjectilePair(k) => k.toi,
        };
        toi_a
            .partial_cmp(&toi_b)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                // Deterministic tie-break: actor kind wins over pair on
                // ties (so the engine's existing M14 actor handlers run
                // first); within a kind, sort by stable key.
                match (a, b) {
                    (SweptCandidateKind::Actor(_), SweptCandidateKind::ProjectilePair(_)) => std::cmp::Ordering::Less,
                    (SweptCandidateKind::ProjectilePair(_), SweptCandidateKind::Actor(_)) => {
                        std::cmp::Ordering::Greater
                    }
                    (SweptCandidateKind::Actor(x), SweptCandidateKind::Actor(y)) => x.target_id.cmp(&y.target_id),
                    (SweptCandidateKind::ProjectilePair(x), SweptCandidateKind::ProjectilePair(y)) => {
                        (x.a_id, x.b_id).cmp(&(y.a_id, y.b_id))
                    }
                }
            })
    });
    let total = candidates.len() as u32;
    candidates
        .into_iter()
        .enumerate()
        .map(|(i, c)| match c {
            SweptCandidateKind::Actor(actor) => ResolvedSweptCandidate::Actor(SweptHitResolved {
                target_id: actor.target_id,
                entry_t: actor.entry_t,
                distance_traveled: actor.distance_traveled,
                entry_point: actor.entry_point,
                ray_origin: actor.ray_origin,
                ray_direction: actor.ray_direction,
                priority_index: i as u32,
                priority_total: total,
            }),
            SweptCandidateKind::ProjectilePair(k) => ResolvedSweptCandidate::ProjectilePair {
                a_id: k.a_id,
                b_id: k.b_id,
                toi: k.toi,
                priority_index: i as u32,
                priority_total: total,
            },
        })
        .collect()
}
