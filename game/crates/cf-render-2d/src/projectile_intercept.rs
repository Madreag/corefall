//! **M14D** — Projectile intercept renderer (spark cluster + dual-trail
//! termination).
//!
//! Bevy-free, pure-data helper consumed by the live + offline renderers.
//! Honors the `cosmetic: true` flag on `collision.projectile_pair_contact`
//! events: under render backpressure the helper drops the spark primitive
//! (the sim event still passes through to the replay log + the renderer
//! still consumes the event, but the visual primitive is suppressed). The
//! killcam excludes these contacts by default; per-player
//! `replay_intercepts` setting opts the player back in.
//!
//! Surface:
//!   - [`IntercepRenderPrimitive`] enum — `SparkCluster` + per-projectile
//!     `TrailTerminator`.
//!   - [`IntercepRenderQueue::enqueue`] pushes the primitive trio for one
//!     intercept event when backpressure allows; drops cleanly otherwise.

use serde::{Deserialize, Serialize};

/// One render primitive emitted by the projectile-pair intercept helper.
/// All payloads carry the canonical (`projectile_a_id`,
/// `projectile_b_id`) so the renderer can dedupe + match to the source
/// event.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IntercepRenderPrimitive {
    /// Mid-air spark cluster anchored at the intercept point.
    SparkCluster {
        a_id: u64,
        b_id: u64,
        anchor: [f32; 2],
        /// Mirror of `outcome` discriminator — drives palette / particle
        /// count without recomputing it client-side.
        outcome: IntercepOutcomeDiscriminator,
    },
    /// Termination point for one projectile's trail (one primitive per
    /// projectile in the pair, total 2 per intercept).
    TrailTerminator {
        projectile_id: u64,
        anchor: [f32; 2],
        outcome: IntercepOutcomeDiscriminator,
    },
}

/// Stable discriminator copied verbatim from the M14D outcome enum so
/// the renderer doesn't depend on cf-physics.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntercepOutcomeDiscriminator {
    FuzeTriggered,
    MutualCancellation,
    ApsIntercept,
    KineticDeflect,
}

impl IntercepOutcomeDiscriminator {
    pub fn as_str(self) -> &'static str {
        match self {
            IntercepOutcomeDiscriminator::FuzeTriggered => "fuze_triggered",
            IntercepOutcomeDiscriminator::MutualCancellation => "mutual_cancellation",
            IntercepOutcomeDiscriminator::ApsIntercept => "aps_intercept",
            IntercepOutcomeDiscriminator::KineticDeflect => "kinetic_deflect",
        }
    }
}

/// Inputs to [`IntercepRenderQueue::enqueue`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct IntercepEventInput {
    pub a_id: u64,
    pub b_id: u64,
    pub anchor: [f32; 2],
    pub outcome: IntercepOutcomeDiscriminator,
    /// Mirror of the replay event's `cosmetic` flag. Always `true` for
    /// M14D pair contacts; carried explicitly so the helper can be
    /// reused by future non-cosmetic kernels without touching the
    /// backpressure semantics.
    pub cosmetic: bool,
}

/// Render queue for projectile-intercept primitives. Caller supplies
/// the current backpressure threshold + queue depth; the helper drops
/// any cosmetic primitive when depth ≥ threshold. **VAL-M14D-018**
/// pins the drop-when-cosmetic-under-backpressure contract.
#[derive(Debug, Default, Clone)]
pub struct IntercepRenderQueue {
    primitives: Vec<IntercepRenderPrimitive>,
    /// Counter of cosmetic primitives the queue suppressed under
    /// backpressure since the last `drain` — surfaced through
    /// [`IntercepRenderQueue::backpressure_drops`] for assertions.
    backpressure_drops: usize,
}

impl IntercepRenderQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// True when the renderer's current queue depth is at or above the
    /// backpressure threshold (cosmetic primitives must be dropped).
    pub fn under_backpressure(queue_depth: usize, backpressure_threshold: usize) -> bool {
        queue_depth >= backpressure_threshold
    }

    /// Enqueue the 3 primitives (1 spark cluster + 2 trail terminators)
    /// for one intercept event. Returns the number of primitives
    /// pushed — `0` when the helper dropped under backpressure.
    pub fn enqueue(
        &mut self,
        event: IntercepEventInput,
        queue_depth: usize,
        backpressure_threshold: usize,
    ) -> usize {
        if event.cosmetic && Self::under_backpressure(queue_depth, backpressure_threshold) {
            self.backpressure_drops += 1;
            return 0;
        }
        self.primitives.push(IntercepRenderPrimitive::SparkCluster {
            a_id: event.a_id,
            b_id: event.b_id,
            anchor: event.anchor,
            outcome: event.outcome,
        });
        self.primitives.push(IntercepRenderPrimitive::TrailTerminator {
            projectile_id: event.a_id,
            anchor: event.anchor,
            outcome: event.outcome,
        });
        self.primitives.push(IntercepRenderPrimitive::TrailTerminator {
            projectile_id: event.b_id,
            anchor: event.anchor,
            outcome: event.outcome,
        });
        3
    }

    /// All primitives currently queued.
    pub fn primitives(&self) -> &[IntercepRenderPrimitive] {
        &self.primitives
    }

    /// Counter of cosmetic primitives the queue suppressed under
    /// backpressure since the last `drain`.
    pub fn backpressure_drops(&self) -> usize {
        self.backpressure_drops
    }

    /// Drain the queue + the backpressure-drop counter.
    pub fn drain(&mut self) -> Vec<IntercepRenderPrimitive> {
        self.backpressure_drops = 0;
        std::mem::take(&mut self.primitives)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(a: u64, b: u64, anchor: [f32; 2]) -> IntercepEventInput {
        IntercepEventInput {
            a_id: a,
            b_id: b,
            anchor,
            outcome: IntercepOutcomeDiscriminator::FuzeTriggered,
            cosmetic: true,
        }
    }

    /// **VAL-M14D-017**: under no backpressure, enqueue produces 1
    /// spark cluster + 2 trail terminators (one per projectile id) at
    /// the TOI intercept coords.
    #[test]
    fn enqueue_produces_three_primitives_per_event() {
        let mut q = IntercepRenderQueue::new();
        let pushed = q.enqueue(ev(7, 9, [120.0, 80.0]), 0, 64);
        assert_eq!(pushed, 3);
        let primitives = q.primitives();
        assert_eq!(primitives.len(), 3);
        let spark_count = primitives
            .iter()
            .filter(|p| matches!(p, IntercepRenderPrimitive::SparkCluster { .. }))
            .count();
        assert_eq!(spark_count, 1);
        let trail_count = primitives
            .iter()
            .filter(|p| matches!(p, IntercepRenderPrimitive::TrailTerminator { .. }))
            .count();
        assert_eq!(trail_count, 2);
        // Anchor coords match the TOI intercept.
        for p in primitives {
            let anchor = match p {
                IntercepRenderPrimitive::SparkCluster { anchor, .. } => *anchor,
                IntercepRenderPrimitive::TrailTerminator { anchor, .. } => *anchor,
            };
            assert!((anchor[0] - 120.0).abs() < f32::EPSILON);
            assert!((anchor[1] - 80.0).abs() < f32::EPSILON);
        }
    }

    /// **VAL-M14D-018**: under backpressure (queue depth ≥ threshold)
    /// cosmetic primitives are dropped — but the backpressure-drop
    /// counter records the suppression so callers can verify.
    #[test]
    fn enqueue_drops_cosmetic_under_backpressure() {
        let mut q = IntercepRenderQueue::new();
        let pushed = q.enqueue(ev(7, 9, [120.0, 80.0]), 128, 64);
        assert_eq!(pushed, 0);
        assert!(q.primitives().is_empty());
        assert_eq!(q.backpressure_drops(), 1);
    }

    /// **VAL-M14D-018**: non-cosmetic events are NOT dropped even at
    /// the backpressure threshold (renderer still honours sim-critical
    /// surfaces).
    #[test]
    fn enqueue_honors_non_cosmetic_under_backpressure() {
        let mut q = IntercepRenderQueue::new();
        let mut e = ev(7, 9, [120.0, 80.0]);
        e.cosmetic = false;
        let pushed = q.enqueue(e, 128, 64);
        assert_eq!(pushed, 3);
        assert_eq!(q.primitives().len(), 3);
        assert_eq!(q.backpressure_drops(), 0);
    }

    /// **VAL-M14D-017**: each TrailTerminator is keyed to a distinct
    /// projectile id (one per projectile in the pair).
    #[test]
    fn trail_terminators_carry_distinct_projectile_ids() {
        let mut q = IntercepRenderQueue::new();
        q.enqueue(ev(7, 9, [120.0, 80.0]), 0, 64);
        let ids: Vec<u64> = q
            .primitives()
            .iter()
            .filter_map(|p| {
                if let IntercepRenderPrimitive::TrailTerminator { projectile_id, .. } = p {
                    Some(*projectile_id)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&7));
        assert!(ids.contains(&9));
    }

    /// Drain resets the backpressure counter + primitives list.
    #[test]
    fn drain_resets_queue_and_counter() {
        let mut q = IntercepRenderQueue::new();
        q.enqueue(ev(1, 2, [0.0, 0.0]), 0, 64);
        q.enqueue(ev(3, 4, [10.0, 0.0]), 128, 64);
        assert_eq!(q.primitives().len(), 3);
        assert_eq!(q.backpressure_drops(), 1);
        let drained = q.drain();
        assert_eq!(drained.len(), 3);
        assert!(q.primitives().is_empty());
        assert_eq!(q.backpressure_drops(), 0);
    }
}
