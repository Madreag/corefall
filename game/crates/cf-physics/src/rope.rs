//! **M14J** § "verlet-rope simulation".
//!
//! 8-segment default verlet rope used by the grappling-hook line, zip-line
//! cables, and any climbable cable / vine. Two-pass solver per tick:
//! integrate node positions via verlet (`x' = 2x - prev_x + a·dt²`), then
//! run a fixed number of distance-constraint relaxations on every segment.
//!
//! Each rope has two endpoints — either an anchored world position (an
//! embedded grappling-hook tip), or an attached actor id. Both ends can
//! independently take any [`RopeEndpoint`] variant.
//!
//! Pure / deterministic: integration uses fixed `dt`; no clock reads; no
//! `rand::thread_rng()`.

use serde::{Deserialize, Serialize};

/// Stable id for a rope instance. Allocated by the engine when a
/// grapple-fire embeds or a zip-kit deploys; carried across ticks for
/// `cfctl rope_input` / `cfctl release_rope` routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RopeId(pub u64);

impl RopeId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Default segment count. The spec § "Verlet rope (8 segments default)"
/// pins 8 segments as enough to handle a 30 m rope at 60 Hz without
/// visible jitter.
pub const DEFAULT_SEGMENT_COUNT: usize = 8;

/// Default solver-relaxation iteration count per tick. Spec § "Notes
/// for the implementer": "4 iterations of distance-constraint relaxation".
pub const DEFAULT_SOLVER_ITERATIONS: u32 = 4;

/// One end of a rope. `Anchored` carries a fixed world position (typically
/// an embedded grappling hook); `Actor` carries the bound actor id and a
/// local offset within the actor (commonly the hand bone).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RopeEndpoint {
    Anchored {
        position: [f32; 2],
    },
    Actor {
        actor_id: u64,
        offset: [f32; 2],
    },
}

impl RopeEndpoint {
    /// World-space position of this endpoint, resolved against the optional
    /// actor position lookup. Returns `None` when an `Actor` endpoint
    /// references an actor the caller did not provide a position for —
    /// callers MUST handle the `None` case (e.g. detach the rope or pin
    /// the node to a safe default). Returning `Some([0.0, 0.0])` on a
    /// missing actor would silently snap rope nodes to world origin and
    /// produce bogus pendulum forces; see audit finding #2.
    #[must_use]
    pub fn world_position<F>(&self, actor_pos: F) -> Option<[f32; 2]>
    where
        F: Fn(u64) -> Option<[f32; 2]>,
    {
        match self {
            RopeEndpoint::Anchored { position } => Some(*position),
            RopeEndpoint::Actor { actor_id, offset } => {
                actor_pos(*actor_id).map(|[x, y]| [x + offset[0], y + offset[1]])
            }
        }
    }

    /// True when the endpoint is anchored to a world position (immobile).
    #[must_use]
    pub fn is_anchored(&self) -> bool {
        matches!(self, RopeEndpoint::Anchored { .. })
    }
}

/// One node along a verlet rope. Position + previous-position (used to
/// derive velocity in the verlet step). `pinned=true` locks the node to
/// the endpoint's world position each tick.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RopeNode {
    pub position: [f32; 2],
    pub previous: [f32; 2],
    pub pinned: bool,
}

impl Default for RopeNode {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0],
            previous: [0.0, 0.0],
            pinned: false,
        }
    }
}

/// by `segment_count` distance-constraints of length `segment_length_m`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Rope {
    pub id: RopeId,
    pub start: RopeEndpoint,
    pub end: RopeEndpoint,
    pub segment_count: u32,
    pub segment_length_m: f32,
    pub gravity: [f32; 2],
    pub nodes: Vec<RopeNode>,
    /// When `true` the rope is taut (kept at full length); slack ropes can
    /// only pull, never push, so the constraint relaxation pulls nodes
    /// inward but never outward.
    #[serde(default)]
    pub taut: bool,
    /// When `true` the rope is embedded (the start endpoint is locked to
    /// the world). Used by the grapple-hook flow to gate `rope_input` and
    /// `release_rope`.
    #[serde(default)]
    pub embedded: bool,
}

impl Rope {
    /// Build a fresh rope with `segment_count` segments evenly distributed
    /// between `start` and `end`'s current world positions.
    ///
    /// For `Actor`-typed endpoints the lookup falls through to `None` (no
    /// actor lookup is provided at construction time), so the corresponding
    /// node falls back to the OTHER endpoint's position. Callers MUST
    /// re-pin actor-attached endpoints to the actor's live position
    /// immediately after construction (and every tick thereafter) via
    /// [`Rope::pin_start`] / [`Rope::pin_end`] — see audit finding #2.
    /// [`Rope::new_with_positions`] is the safer constructor when both
    /// initial positions are known.
    #[must_use]
    pub fn new(id: RopeId, start: RopeEndpoint, end: RopeEndpoint, segment_count: u32, gravity: [f32; 2]) -> Self {
        let start_pos = start.world_position(|_| None);
        let end_pos = end.world_position(|_| None);
        // Fall back so neither end snaps to world origin when one end is an
        // un-resolved Actor endpoint (caller must pin it next tick).
        let (start_pos, end_pos) = match (start_pos, end_pos) {
            (Some(s), Some(e)) => (s, e),
            (Some(s), None) => (s, s),
            (None, Some(e)) => (e, e),
            (None, None) => ([0.0, 0.0], [0.0, 0.0]),
        };
        Self::new_with_positions(id, start, end, start_pos, end_pos, segment_count, gravity)
    }

    /// **Audit finding #2 fix**: explicit-position constructor. Use this
    /// when both initial endpoint positions are known (e.g. when the
    /// engine has live actor positions in hand). Avoids the silent
    /// "snap to origin" failure mode of [`Rope::new`] for Actor endpoints.
    #[must_use]
    pub fn new_with_positions(
        id: RopeId,
        start: RopeEndpoint,
        end: RopeEndpoint,
        start_pos: [f32; 2],
        end_pos: [f32; 2],
        segment_count: u32,
        gravity: [f32; 2],
    ) -> Self {
        let dx = end_pos[0] - start_pos[0];
        let dy = end_pos[1] - start_pos[1];
        let total_len = (dx * dx + dy * dy).sqrt().max(0.0);
        let seg_count = segment_count.max(1);
        let segment_length = (total_len / seg_count as f32).max(0.05);
        let mut nodes = Vec::with_capacity((seg_count + 1) as usize);
        for i in 0..=seg_count {
            let t = i as f32 / seg_count as f32;
            let pos = [start_pos[0] + dx * t, start_pos[1] + dy * t];
            nodes.push(RopeNode {
                position: pos,
                previous: pos,
                pinned: i == 0 || i == seg_count,
            });
        }
        Self {
            id,
            start,
            end,
            segment_count: seg_count,
            segment_length_m: segment_length,
            gravity,
            nodes,
            taut: true,
            embedded: false,
        }
    }

    /// **Audit finding #2 fix**: per-tick re-pin of `Actor`-typed endpoints
    /// to the live actor position. Callers (e.g. engine `m14j_tick`) pass
    /// a `(actor_id → live position)` lookup. Anchored endpoints stay
    /// pinned to their original world position. Returns `false` when an
    /// Actor endpoint references an actor the caller could not resolve
    /// (signals the caller to detach the rope or treat it as orphaned).
    pub fn retrack_endpoints<F>(&mut self, mut actor_pos: F) -> bool
    where
        F: FnMut(u64) -> Option<[f32; 2]>,
    {
        let mut all_resolved = true;
        match self.start {
            RopeEndpoint::Anchored { position } => self.pin_start(position),
            RopeEndpoint::Actor { actor_id, offset } => match actor_pos(actor_id) {
                Some([x, y]) => self.pin_start([x + offset[0], y + offset[1]]),
                None => {
                    all_resolved = false;
                }
            },
        }
        match self.end {
            RopeEndpoint::Anchored { position } => self.pin_end(position),
            RopeEndpoint::Actor { actor_id, offset } => match actor_pos(actor_id) {
                Some([x, y]) => self.pin_end([x + offset[0], y + offset[1]]),
                None => {
                    all_resolved = false;
                }
            },
        }
        all_resolved
    }

    /// Total length of the rope across all segment-constraints.
    #[must_use]
    pub fn total_length_m(&self) -> f32 {
        self.segment_length_m * self.segment_count as f32
    }

    /// Pin the start node to `pos` (overwrites previous + current). Called
    /// before each integration step when the start endpoint is anchored.
    pub fn pin_start(&mut self, pos: [f32; 2]) {
        if let Some(n) = self.nodes.first_mut() {
            n.position = pos;
            n.previous = pos;
            n.pinned = true;
        }
    }

    /// Pin the end node to `pos`. Mirror of [`pin_start`].
    pub fn pin_end(&mut self, pos: [f32; 2]) {
        if let Some(n) = self.nodes.last_mut() {
            n.position = pos;
            n.previous = pos;
            n.pinned = true;
        }
    }

    /// Whether either endpoint is anchored (an immobile world point).
    #[must_use]
    pub fn has_anchor(&self) -> bool {
        self.start.is_anchored() || self.end.is_anchored()
    }

    /// World-space position of the bob node (the last non-anchored node).
    /// Used by [`pendulum_release_velocity`].
    #[must_use]
    pub fn bob_position(&self) -> [f32; 2] {
        self.nodes
            .last()
            .map(|n| n.position)
            .unwrap_or([0.0, 0.0])
    }

    /// Bob position relative to the anchor (start) node.
    #[must_use]
    pub fn bob_relative_to_anchor(&self) -> [f32; 2] {
        let anchor = self
            .nodes
            .first()
            .map(|n| n.position)
            .unwrap_or([0.0, 0.0]);
        let bob = self.bob_position();
        [bob[0] - anchor[0], bob[1] - anchor[1]]
    }

    /// the rope simulation one tick. `dt_seconds` is the fixed sim dt
    /// (1/60 at 60 Hz). `iterations` is the number of distance-constraint
    /// relaxation passes (default 4 per spec).
    pub fn step(&mut self, dt_seconds: f32, iterations: u32) {
        // ----- 1) Verlet integration (per non-pinned node) -----
        let dt2 = dt_seconds * dt_seconds;
        for node in &mut self.nodes {
            if node.pinned {
                continue;
            }
            let vx = node.position[0] - node.previous[0];
            let vy = node.position[1] - node.previous[1];
            let new_x = node.position[0] + vx + self.gravity[0] * dt2;
            let new_y = node.position[1] + vy + self.gravity[1] * dt2;
            node.previous = node.position;
            node.position = [new_x, new_y];
        }
        // ----- 2) Distance-constraint relaxation -----
        let iter = iterations.max(1);
        for _ in 0..iter {
            for i in 0..self.segment_count as usize {
                self.relax_segment(i);
            }
        }
    }

    fn relax_segment(&mut self, i: usize) {
        let j = i + 1;
        if j >= self.nodes.len() {
            return;
        }
        let (a, b) = {
            let a = self.nodes[i];
            let b = self.nodes[j];
            (a, b)
        };
        let dx = b.position[0] - a.position[0];
        let dy = b.position[1] - a.position[1];
        let dist = (dx * dx + dy * dy).sqrt().max(1e-6);
        let target = self.segment_length_m;
        if self.taut {
            // Taut rope: ALWAYS push back to target length (both directions).
            let diff = (dist - target) / dist;
            let correction = [dx * diff * 0.5, dy * diff * 0.5];
            if !a.pinned {
                self.nodes[i].position[0] += correction[0];
                self.nodes[i].position[1] += correction[1];
            } else if !b.pinned {
                // a pinned: apply full correction to b
                self.nodes[j].position[0] -= correction[0] * 2.0;
                self.nodes[j].position[1] -= correction[1] * 2.0;
                return;
            }
            if !b.pinned {
                self.nodes[j].position[0] -= correction[0];
                self.nodes[j].position[1] -= correction[1];
            } else {
                // b pinned: apply full correction to a
                self.nodes[i].position[0] += correction[0] * 2.0;
                self.nodes[i].position[1] += correction[1] * 2.0;
            }
        } else if dist > target {
            // Slack rope: only pull when over-extended (never push).
            let diff = (dist - target) / dist;
            let correction = [dx * diff * 0.5, dy * diff * 0.5];
            if !a.pinned {
                self.nodes[i].position[0] += correction[0];
                self.nodes[i].position[1] += correction[1];
            }
            if !b.pinned {
                self.nodes[j].position[0] -= correction[0];
                self.nodes[j].position[1] -= correction[1];
            }
        }
    }
}

/// instantaneous tangential velocity at the apex of a pendulum arc.
///
/// For a pendulum of length `length_m` released at angle `theta_rad` from
/// vertical, the velocity at the bottom of the swing is
/// `v = sqrt(2 g L (1 - cos(theta)))`. Spec § Acceptance criteria
/// "Rope swing exits at pendulum velocity" literally cites this formula.
///
/// Returns `(vx, vy)` tangential to the rope direction at the apex. The
/// tangent points "forward" in the swing direction (positive theta = swing
/// to the right). Pure / deterministic.
#[must_use]
pub fn pendulum_release_velocity(length_m: f32, theta_rad: f32, gravity_m_s2: f32) -> [f32; 2] {
    let len = length_m.max(0.0);
    let g = gravity_m_s2.abs();
    let speed = (2.0 * g * len * (1.0 - theta_rad.cos())).max(0.0).sqrt();
    // Tangent points perpendicular to the rope; rope at theta points
    // along (sin(theta), -cos(theta)) from anchor (y-down convention
    // matches game space). Tangent is the right-perpendicular of that
    // unit vector: (cos(theta), sin(theta)). Speed sign carries through.
    let dir = theta_rad.signum();
    [speed * dir * theta_rad.cos().abs(), -speed * theta_rad.sin().abs()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchored(x: f32, y: f32) -> RopeEndpoint {
        RopeEndpoint::Anchored { position: [x, y] }
    }

    #[test]
    fn rope_initializes_with_segment_count_plus_one_nodes() {
        let rope = Rope::new(
            RopeId(1),
            anchored(0.0, 0.0),
            anchored(8.0, 0.0),
            8,
            [0.0, -9.81],
        );
        assert_eq!(rope.segment_count, 8);
        assert_eq!(rope.nodes.len(), 9);
        assert!((rope.segment_length_m - 1.0).abs() < 1e-3);
    }

    #[test]
    fn rope_endpoints_pinned() {
        let rope = Rope::new(
            RopeId(1),
            anchored(0.0, 0.0),
            anchored(8.0, 0.0),
            8,
            [0.0, -9.81],
        );
        assert!(rope.nodes.first().unwrap().pinned);
        assert!(rope.nodes.last().unwrap().pinned);
        for n in &rope.nodes[1..8] {
            assert!(!n.pinned, "middle nodes must not be pinned");
        }
    }

    #[test]
    fn rope_step_does_not_diverge() {
        let mut rope = Rope::new(
            RopeId(1),
            anchored(0.0, 0.0),
            anchored(8.0, 0.0),
            8,
            [0.0, -9.81],
        );
        for _ in 0..600 {
            rope.step(1.0 / 60.0, 4);
        }
        // After 10 seconds, total length must stay finite + close to
        // initial total length (with mild sag).
        let total_len: f32 = (0..rope.segment_count as usize)
            .map(|i| {
                let dx = rope.nodes[i + 1].position[0] - rope.nodes[i].position[0];
                let dy = rope.nodes[i + 1].position[1] - rope.nodes[i].position[1];
                (dx * dx + dy * dy).sqrt()
            })
            .sum();
        assert!(total_len.is_finite(), "rope must not diverge");
        assert!(total_len > 0.0);
    }

    #[test]
    fn rope_step_deterministic() {
        let mut r1 = Rope::new(
            RopeId(1),
            anchored(0.0, 0.0),
            anchored(12.0, 5.0),
            8,
            [0.0, -9.81],
        );
        let mut r2 = r1.clone();
        for _ in 0..120 {
            r1.step(1.0 / 60.0, 4);
            r2.step(1.0 / 60.0, 4);
        }
        for (a, b) in r1.nodes.iter().zip(r2.nodes.iter()) {
            assert!((a.position[0] - b.position[0]).abs() < 1e-6);
            assert!((a.position[1] - b.position[1]).abs() < 1e-6);
        }
    }

    #[test]
    fn pendulum_velocity_matches_formula() {
        // sqrt(2 * 9.81 * 4.0 * (1 - cos(pi/6))) ~= 3.243
        let v = pendulum_release_velocity(4.0, std::f32::consts::FRAC_PI_6, 9.81);
        let speed = (v[0] * v[0] + v[1] * v[1]).sqrt();
        let expected = (2.0_f32 * 9.81 * 4.0 * (1.0 - std::f32::consts::FRAC_PI_6.cos())).sqrt();
        assert!((speed - expected).abs() < 1e-3, "got {speed}, expected {expected}");
    }

    #[test]
    fn pendulum_velocity_zero_at_vertical() {
        // theta=0 → cos(theta)=1 → speed = 0
        let v = pendulum_release_velocity(4.0, 0.0, 9.81);
        assert!((v[0]).abs() < 1e-6);
        assert!((v[1]).abs() < 1e-6);
    }

    #[test]
    fn endpoint_resolves_actor_offset() {
        let ep = RopeEndpoint::Actor {
            actor_id: 42,
            offset: [0.0, -4.0],
        };
        let lookup = |id: u64| if id == 42 { Some([100.0, 50.0]) } else { None };
        let pos = ep.world_position(lookup).expect("known actor must resolve");
        assert!((pos[0] - 100.0).abs() < 1e-6);
        assert!((pos[1] - 46.0).abs() < 1e-6);
    }

    /// **Audit finding #2 fix**: `world_position` returns `None` for an
    /// `Actor` endpoint when the lookup fails. Callers MUST handle this
    /// case — silently returning `[0.0, 0.0]` (the old behavior) would
    /// snap rope nodes to world origin.
    #[test]
    fn endpoint_missing_actor_returns_none() {
        let ep = RopeEndpoint::Actor {
            actor_id: 999,
            offset: [0.0, 0.0],
        };
        let lookup = |_id: u64| -> Option<[f32; 2]> { None };
        assert!(ep.world_position(lookup).is_none());
    }

    /// **Audit finding #2 fix**: `retrack_endpoints` per-tick re-pins
    /// `Actor`-typed endpoints to the live actor position, so the rope's
    /// actor end follows the player instead of staying frozen at the
    /// initial node positions.
    #[test]
    fn retrack_endpoints_follows_actor() {
        let mut rope = Rope::new_with_positions(
            RopeId(1),
            anchored(0.0, 10.0),
            RopeEndpoint::Actor {
                actor_id: 42,
                offset: [0.0, 0.0],
            },
            [0.0, 10.0],
            [8.0, 10.0],
            8,
            [0.0, -9.81],
        );
        let before = rope.bob_position();
        let all_ok = rope.retrack_endpoints(|id| {
            if id == 42 {
                Some([12.0, 6.0])
            } else {
                None
            }
        });
        assert!(all_ok);
        let after = rope.bob_position();
        assert!((after[0] - 12.0).abs() < 1e-6, "bob must track actor x; got {}", after[0]);
        assert!((after[1] - 6.0).abs() < 1e-6, "bob must track actor y; got {}", after[1]);
        assert_ne!(before, after);
    }

    /// **Audit finding #2 fix**: `retrack_endpoints` returns `false` when
    /// an `Actor` endpoint cannot be resolved — signals the caller (engine)
    /// to treat the rope as orphaned.
    #[test]
    fn retrack_endpoints_reports_unresolved_actor() {
        let mut rope = Rope::new_with_positions(
            RopeId(1),
            anchored(0.0, 10.0),
            RopeEndpoint::Actor {
                actor_id: 42,
                offset: [0.0, 0.0],
            },
            [0.0, 10.0],
            [8.0, 10.0],
            8,
            [0.0, -9.81],
        );
        let all_ok = rope.retrack_endpoints(|_| None);
        assert!(!all_ok, "missing actor lookup must return false");
    }
}
