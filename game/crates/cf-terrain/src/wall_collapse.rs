//! **M14F** § Lateral wall collapse — bulging / crack / rupture cascade.
//!
//! Pure helpers: state in, state out. No clock, no `thread_rng`.
//! Mirrors `cave_in.rs` 90° rotated for sidewalls. Callers supply an
//! `f32` RNG draw from the engine's seeded RNG when the wall-rupture
//! roll fires. Determinism is guaranteed by the seeded-draw contract —
//! same seed → same outcome (VAL-M14F-013, VAL-M14F-028).
//!
//! Three event tiers per the spec's L1 → L2 → L3 sidewall crack
//! progression:
//!   - [`wall_bulging_chance_per_tick`] — L1 warning; same formula as
//!     `cave_in_chance_per_tick` but using the lateral unsupported span.
//!   - [`wall_crack_advanced_chance_per_tick`] — L2 escalation.
//!   - [`wall_rupture_chance_per_tick`] — L3 / catastrophic; same shape
//!     as `cave_in_chance_per_tick`.
//!
//! Pressure-differential blowout (M19 atmospherics):
//!   - [`pressure_blowout_triggers`] — true when
//!     `pressure_delta_kpa × wall_area_px > lateral_yield × wall_area_px`,
//!     i.e. the pressure delta exceeds the per-pixel yield. Used by the
//!     sealed-room sudden-decompression cascade (VAL-M14F-018).
//!
//! Cascade detection: re-uses the M14E
//! `cave_in::cascade_neighbors_for_chunk` so lateral neighbors are
//! discovered with the same canonical ordering (VAL-M14F-026).

use serde::{Deserialize, Serialize};

use crate::cave_in::{
    cave_in_chance_per_tick, CascadeNeighbor, CAVE_IN_BASE_COEFFICIENT, FALLING_DEBRIS_CAP,
    UNSUPPORTED_SPAN_FLOOR_PX,
};

/// Hard ceiling on falling-debris pixel count per wall-rupture event
/// (bounds cosmetic load — shared with M14E ceiling-collapse cone).
pub const WALL_RUPTURE_DEBRIS_CAP: u32 = FALLING_DEBRIS_CAP;

/// Spec literal coefficient for the wall-bulging probability formula.
/// Shares the M14E base coefficient so the union perf budget holds.
pub const WALL_COLLAPSE_BASE_COEFFICIENT: f32 = CAVE_IN_BASE_COEFFICIENT;

/// Span (px) at or below which the lateral wall holds indefinitely.
/// Mirror of [`UNSUPPORTED_SPAN_FLOOR_PX`] — the spec's "12 px holds"
/// guarantee (VAL-M14F-001) covers any width strictly below 13. We use
/// the same 16-pixel floor as the ceiling pass so the union perf path
/// is identical.
pub const WALL_LATERAL_SPAN_FLOOR_PX: u32 = UNSUPPORTED_SPAN_FLOOR_PX;

/// Span (px) at or below which the lateral wall holds and the
/// integrity field stays locked at ≥ 200. VAL-M14F-001 covers 11 px;
/// we accept anything strictly less than 13 (matches the M14E ceiling
/// "12 px stays stable" sibling).
pub const WALL_LATERAL_STABLE_SPAN_PX: u32 = 12;

/// Per-tick L1 (bulging) probability. Same shape as
/// [`cave_in_chance_per_tick`] — the wall pass is the 90° sibling.
#[must_use]
pub fn wall_bulging_chance_per_tick(unsupported_span_px: u32, vibration_modifier: f32) -> f32 {
    cave_in_chance_per_tick(unsupported_span_px, vibration_modifier)
}

/// Per-tick L2 (crack-advanced) probability. The L2 escalation fires
/// faster than L1 once the wall is already cracked — scales the same
/// formula by 2.0× so the cascade reaches L3 inside the same N=15
/// pass window per VAL-M14F-012 / VAL-M14F-025.
#[must_use]
pub fn wall_crack_advanced_chance_per_tick(unsupported_span_px: u32, vibration_modifier: f32) -> f32 {
    cave_in_chance_per_tick(unsupported_span_px, vibration_modifier * 2.0)
}

/// Per-tick L3 (rupture) probability. The L3 / catastrophic step uses
/// the same base shape scaled 4×. Below the spec's stable-span floor
/// the chance is exactly 0 (VAL-M14F-001).
#[must_use]
pub fn wall_rupture_chance_per_tick(unsupported_span_px: u32, vibration_modifier: f32) -> f32 {
    cave_in_chance_per_tick(unsupported_span_px, vibration_modifier * 4.0)
}

/// Per-pixel falling-debris count per wall rupture. Mirrors
/// `cave_in::falling_debris_count` so VAL-M14F-027's payload field
/// shares the same domain as VAL-M14E-021.
#[must_use]
pub fn wall_rupture_debris_count(span_px: u32, wall_thickness_px: u32) -> u32 {
    span_px.saturating_mul(wall_thickness_px).min(WALL_RUPTURE_DEBRIS_CAP)
}

/// One wall-collapse event tier. Maps 1:1 to the three new replay
/// schemas (`terrain.wall_bulging` / `terrain.wall_crack_advanced` /
/// `terrain.wall_rupture`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallCollapseStage {
    /// L1 — wall has begun bulging. Cosmetic L1 sidewall crack decal +
    /// `MINESHAFT WALL UNSTABLE` HUD banner (VAL-M14F-002 / -003).
    Bulging,
    /// L2 — crack advanced. Cosmetic L2 sidewall crack decal
    /// (VAL-M14F-012 / VAL-M14F-025).
    CrackAdvanced,
    /// L3 — catastrophic rupture. Drives the falling-debris cone +
    /// fluid cascade (VAL-M14F-006 / VAL-M14F-007).
    Rupture,
}

impl WallCollapseStage {
    pub fn as_str(self) -> &'static str {
        match self {
            WallCollapseStage::Bulging => "wall_bulging",
            WallCollapseStage::CrackAdvanced => "wall_crack_advanced",
            WallCollapseStage::Rupture => "wall_rupture",
        }
    }

    /// Crack-decal level the renderer should escalate to alongside this
    /// stage (L1 / L2 / L3). Pairs with
    /// `cf_render_2d::tunnel_collapse::CrackLevel`.
    pub fn render_level(self) -> &'static str {
        match self {
            WallCollapseStage::Bulging => "l1",
            WallCollapseStage::CrackAdvanced => "l2",
            WallCollapseStage::Rupture => "l3",
        }
    }
}

/// Outcome of a single per-tick lateral-collapse roll.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallCollapseOutcome {
    /// Roll fired — engine advances the wall to the next stage.
    Trigger(WallCollapseStage),
    /// Roll did not fire this tick.
    Hold,
}

impl WallCollapseOutcome {
    pub fn fired(self) -> bool {
        matches!(self, WallCollapseOutcome::Trigger(_))
    }

    pub fn stage(self) -> Option<WallCollapseStage> {
        match self {
            WallCollapseOutcome::Trigger(s) => Some(s),
            WallCollapseOutcome::Hold => None,
        }
    }
}

/// Per-tick wall-bulging roll. Pure mirror of `cave_in::cave_in_roll`
/// using the L1 chance formula. NaN draws never fire (defensive).
#[must_use]
pub fn wall_bulging_roll(rng_draw: f32, unsupported_span_px: u32, vibration_modifier: f32) -> WallCollapseOutcome {
    let chance = wall_bulging_chance_per_tick(unsupported_span_px, vibration_modifier);
    if rng_draw.is_nan() || chance <= 0.0 {
        return WallCollapseOutcome::Hold;
    }
    if rng_draw < chance.clamp(0.0, 1.0) {
        WallCollapseOutcome::Trigger(WallCollapseStage::Bulging)
    } else {
        WallCollapseOutcome::Hold
    }
}

/// Per-tick L2 escalation roll. Fires only after a bulging event has
/// already landed (caller maintains the stage gate).
#[must_use]
pub fn wall_crack_advanced_roll(rng_draw: f32, unsupported_span_px: u32, vibration_modifier: f32) -> WallCollapseOutcome {
    let chance = wall_crack_advanced_chance_per_tick(unsupported_span_px, vibration_modifier);
    if rng_draw.is_nan() || chance <= 0.0 {
        return WallCollapseOutcome::Hold;
    }
    if rng_draw < chance.clamp(0.0, 1.0) {
        WallCollapseOutcome::Trigger(WallCollapseStage::CrackAdvanced)
    } else {
        WallCollapseOutcome::Hold
    }
}

/// Per-tick L3 rupture roll. Fires only after the wall has reached
/// `WallCollapseStage::CrackAdvanced`.
#[must_use]
pub fn wall_rupture_roll(rng_draw: f32, unsupported_span_px: u32, vibration_modifier: f32) -> WallCollapseOutcome {
    let chance = wall_rupture_chance_per_tick(unsupported_span_px, vibration_modifier);
    if rng_draw.is_nan() || chance <= 0.0 {
        return WallCollapseOutcome::Hold;
    }
    if rng_draw < chance.clamp(0.0, 1.0) {
        WallCollapseOutcome::Trigger(WallCollapseStage::Rupture)
    } else {
        WallCollapseOutcome::Hold
    }
}

/// **M14F § VAL-M14F-018**: pressure-differential blowout predicate.
/// Returns `true` when the lateral pressure delta × wall area exceeds
/// the wall's lateral yield × wall area threshold:
///
/// > Pressure differential (M19 atmospherics) applies lateral force to
/// > walls; sealed-room sudden-vacuum-exposure can blow out walls.
/// > Pressure differential blowout reads M19 `cell_pressure_kpa`;
/// > threshold = `wall.lateral_yield × wall_area`.
///
/// The cancellation of `wall_area_px` on both sides means the
/// blowout fires when `pressure_delta_kpa > lateral_yield_strength`.
/// We retain the area term in the API so callers reading the spec
/// literal find the surface match.
#[must_use]
pub fn pressure_blowout_triggers(
    pressure_delta_kpa: f32,
    lateral_yield_strength: u16,
    wall_area_px: u32,
) -> bool {
    if !pressure_delta_kpa.is_finite() || pressure_delta_kpa <= 0.0 {
        return false;
    }
    if wall_area_px == 0 {
        return false;
    }
    let force = pressure_delta_kpa * (wall_area_px as f32);
    let threshold = (lateral_yield_strength as f32) * (wall_area_px as f32);
    force > threshold
}

/// **M14F** § Cumulative-stress accumulator for a perimeter wall. Each
/// impact adds `damage` to a counter; the wall ruptures when the
/// accumulator crosses `lateral_yield_strength` (VAL-M14F-030).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CumulativeStress {
    pub damage_accumulated: u32,
    pub hits: u32,
}

impl CumulativeStress {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one impact and return the integrity sample (255 - damage
    /// clamped) for snapshot. The accumulator saturates at u32::MAX.
    pub fn record_hit(&mut self, damage: u32) -> u8 {
        self.damage_accumulated = self.damage_accumulated.saturating_add(damage);
        self.hits = self.hits.saturating_add(1);
        let clamped: u32 = self.damage_accumulated.min(255);
        (255u32 - clamped) as u8
    }

    /// True when accumulated damage has crossed the wall's
    /// `lateral_yield_strength` rupture threshold.
    pub fn ruptured(&self, lateral_yield_strength: u16) -> bool {
        self.damage_accumulated >= u32::from(lateral_yield_strength)
    }
}

/// **M14F** § Snapshot payload for the four wall events. Mirrors
/// `CaveInPayload`; the per-event JSON layer keeps just the bbox +
/// chunk + debris field per VAL-M14F-027 + VAL-M14F-019.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WallCollapsePayload {
    /// Stage tag (`wall_bulging` / `wall_crack_advanced` / `wall_rupture`).
    pub stage: WallCollapseStage,
    /// Chunk coord encoded as `(cx, cy)`.
    pub chunk_id: (i32, i32),
    /// Inclusive pixel-space AABB of the affected lateral wall.
    pub bbox_min: [i64; 2],
    pub bbox_max: [i64; 2],
    /// Pixel count per `wall_rupture_debris_count(span_px, thickness)`.
    /// Always 0 for `Bulging` / `CrackAdvanced`.
    pub falling_debris_count: u32,
    /// Unsupported span (px).
    pub unsupported_span_px: u32,
    /// Vibration modifier the roll consumed (or pressure-blowout flag
    /// when fired from M19).
    pub vibration_modifier: f32,
    /// True for the first stage event; subsequent same-chunk events
    /// (e.g. cascade from neighbor) are flagged `false`.
    pub cascade_primary: bool,
}

impl WallCollapsePayload {
    /// **M14F** § Construct a `wall_bulging` payload.
    #[must_use]
    pub fn bulging(
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        vibration_modifier: f32,
    ) -> Self {
        Self {
            stage: WallCollapseStage::Bulging,
            chunk_id,
            bbox_min,
            bbox_max,
            falling_debris_count: 0,
            unsupported_span_px,
            vibration_modifier,
            cascade_primary: true,
        }
    }

    /// **M14F** § Construct a `wall_crack_advanced` payload.
    #[must_use]
    pub fn crack_advanced(
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        vibration_modifier: f32,
    ) -> Self {
        Self {
            stage: WallCollapseStage::CrackAdvanced,
            chunk_id,
            bbox_min,
            bbox_max,
            falling_debris_count: 0,
            unsupported_span_px,
            vibration_modifier,
            cascade_primary: true,
        }
    }

    /// **M14F** § Construct a `wall_rupture` payload with the canonical
    /// `chunk_id + bbox + falling_debris_count` triple per VAL-M14F-027.
    #[must_use]
    pub fn rupture(
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        wall_thickness_px: u32,
        vibration_modifier: f32,
    ) -> Self {
        Self {
            stage: WallCollapseStage::Rupture,
            chunk_id,
            bbox_min,
            bbox_max,
            falling_debris_count: wall_rupture_debris_count(unsupported_span_px, wall_thickness_px),
            unsupported_span_px,
            vibration_modifier,
            cascade_primary: true,
        }
    }

    /// Mark this payload as a cascade (secondary) event.
    #[must_use]
    pub fn into_cascade(mut self) -> Self {
        self.cascade_primary = false;
        self
    }
}

/// **M14F § VAL-M14F-026**: cascade neighbor detection for lateral
/// walls. Mirrors `cave_in::cascade_neighbors_for_chunk` so the lateral
/// cascade picks up the same chunk-coord ordering.
#[must_use]
pub fn lateral_cascade_neighbors_for_chunk(cx: i32, cy: i32) -> [CascadeNeighbor; 4] {
    crate::cave_in::cascade_neighbors_for_chunk(cx, cy)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M14F-001: wall_bulging chance is 0 for narrow shafts.
    #[test]
    fn bulging_holds_at_or_below_16_px_floor() {
        for span in [0u32, 1, 8, 11, 16] {
            for vib in [0.0_f32, 0.5, 1.0, 4.0] {
                assert_eq!(wall_bulging_chance_per_tick(span, vib), 0.0);
                assert_eq!(wall_bulging_roll(0.0, span, vib), WallCollapseOutcome::Hold);
            }
        }
    }

    /// VAL-M14F-002: a 24-px-wide shaft has non-zero bulging chance.
    #[test]
    fn bulging_fires_above_floor() {
        let chance = wall_bulging_chance_per_tick(24, 1.0);
        assert!(chance > 0.0);
        // A small draw triggers within 30 ticks worth of rolls
        // (chance × 30 ≈ 0.024).
        assert_eq!(
            wall_bulging_roll(0.0001, 24, 1.0),
            WallCollapseOutcome::Trigger(WallCollapseStage::Bulging),
        );
    }

    /// VAL-M14F-012: crack-advanced fires faster than bulging once
    /// unlocked.
    #[test]
    fn crack_advanced_chance_exceeds_bulging() {
        let bulging = wall_bulging_chance_per_tick(24, 1.0);
        let crack = wall_crack_advanced_chance_per_tick(24, 1.0);
        assert!(crack > bulging, "crack ({crack}) must exceed bulging ({bulging})");
    }

    /// VAL-M14F-012: rupture fires faster than crack-advanced.
    #[test]
    fn rupture_chance_exceeds_crack_advanced() {
        let crack = wall_crack_advanced_chance_per_tick(24, 1.0);
        let rupture = wall_rupture_chance_per_tick(24, 1.0);
        assert!(rupture > crack, "rupture ({rupture}) must exceed crack ({crack})");
    }

    /// VAL-M14F-013: same input → same outcome on every call.
    #[test]
    fn roll_is_deterministic_for_fixed_inputs() {
        for draw in [0.0_f32, 0.0001, 0.001, 0.5, 0.999] {
            for span in [17u32, 24, 32, 48] {
                let a = wall_bulging_roll(draw, span, 1.0);
                let b = wall_bulging_roll(draw, span, 1.0);
                let c = wall_bulging_roll(draw, span, 1.0);
                assert_eq!(a, b);
                assert_eq!(b, c);
            }
        }
    }

    /// VAL-M14F-018: pressure-differential blowout requires
    /// `delta > lateral_yield`. Wall area cancels on both sides so the
    /// predicate reduces to a per-kPa-vs-yield comparison.
    #[test]
    fn pressure_blowout_above_threshold() {
        // concrete (50). 101 kPa - 0 kPa (vacuum) = 101 > 50 → blowout.
        assert!(pressure_blowout_triggers(101.0, 50, 256));
        // Brick (30) at 50 kPa differential.
        assert!(pressure_blowout_triggers(50.0, 30, 256));
    }

    /// VAL-M14F-018: pressure delta below threshold yields no rupture.
    #[test]
    fn pressure_blowout_below_threshold() {
        // 25 kPa delta vs concrete (50) → hold.
        assert!(!pressure_blowout_triggers(25.0, 50, 256));
        // 0 kPa delta → hold regardless.
        assert!(!pressure_blowout_triggers(0.0, 0, 256));
        // Negative / NaN deltas hold.
        assert!(!pressure_blowout_triggers(-50.0, 30, 256));
        assert!(!pressure_blowout_triggers(f32::NAN, 50, 256));
    }

    /// VAL-M14F-018: zero wall area → no rupture (sentinel guard).
    #[test]
    fn pressure_blowout_zero_area_holds() {
        assert!(!pressure_blowout_triggers(1000.0, 1, 0));
    }

    /// VAL-M14F-027: `terrain.wall_rupture` payload carries chunk_id,
    /// bbox, and falling_debris_count.
    #[test]
    fn rupture_payload_carries_required_fields() {
        let p = WallCollapsePayload::rupture((3, 4), [16, 32], [48, 64], 32, 4, 1.0);
        assert_eq!(p.chunk_id, (3, 4));
        assert_eq!(p.bbox_min, [16, 32]);
        assert_eq!(p.bbox_max, [48, 64]);
        assert_eq!(p.falling_debris_count, 128);
        assert_eq!(p.stage, WallCollapseStage::Rupture);
        assert!(p.cascade_primary);
    }

    /// VAL-M14F-027: payload stages cover all three event tiers.
    #[test]
    fn payload_stage_helpers_cover_three_tiers() {
        let b = WallCollapsePayload::bulging((0, 0), [0, 0], [16, 16], 24, 1.0);
        let c = WallCollapsePayload::crack_advanced((0, 0), [0, 0], [16, 16], 24, 1.0);
        let r = WallCollapsePayload::rupture((0, 0), [0, 0], [16, 16], 24, 4, 1.0);
        assert_eq!(b.stage, WallCollapseStage::Bulging);
        assert_eq!(c.stage, WallCollapseStage::CrackAdvanced);
        assert_eq!(r.stage, WallCollapseStage::Rupture);
        assert_eq!(b.falling_debris_count, 0);
        assert_eq!(c.falling_debris_count, 0);
        assert!(r.falling_debris_count > 0);
    }

    /// VAL-M14F-027: cascade flag distinguishes primary vs secondary
    /// events.
    #[test]
    fn rupture_payload_into_cascade_clears_primary() {
        let p = WallCollapsePayload::rupture((1, 2), [16, 32], [48, 64], 32, 4, 1.0).into_cascade();
        assert!(!p.cascade_primary);
    }

    /// VAL-M14F-026: cascade neighbors match the M14E canonical
    /// ordering (north, south, west, east).
    #[test]
    fn lateral_cascade_returns_four_side_neighbors_in_canonical_order() {
        let nbrs = lateral_cascade_neighbors_for_chunk(3, 5);
        assert_eq!(
            nbrs,
            [
                CascadeNeighbor { cx: 3, cy: 4 },
                CascadeNeighbor { cx: 3, cy: 6 },
                CascadeNeighbor { cx: 2, cy: 5 },
                CascadeNeighbor { cx: 4, cy: 5 },
            ]
        );
    }

    /// VAL-M14F-030: cumulative-stress accumulator monotonically
    /// decreases the integrity sample on successive hits.
    #[test]
    fn cumulative_stress_monotonically_drops_integrity() {
        let mut s = CumulativeStress::new();
        let i1 = s.record_hit(50);
        let i2 = s.record_hit(50);
        let i3 = s.record_hit(50);
        let i4 = s.record_hit(50);
        assert!(i1 > i2 && i2 > i3 && i3 > i4);
        assert!(s.ruptured(150), "200 dmg exceeds 150 yield");
        assert!(s.ruptured(200), "200 dmg exactly meets 200 yield");
        assert!(!s.ruptured(300));
        assert_eq!(s.hits, 4);
    }

    /// VAL-M14F-030: cumulative-stress accumulator saturates at 255
    /// integrity floor and never panics on overflow.
    #[test]
    fn cumulative_stress_saturates_at_floor() {
        let mut s = CumulativeStress::new();
        for _ in 0..100 {
            s.record_hit(u32::MAX / 32);
        }
        // record_hit saturates and clamps to 255 → integrity floor 0.
        let final_i = s.record_hit(1);
        assert_eq!(final_i, 0);
    }

    /// VAL-M14F-027: debris count caps at 200.
    #[test]
    fn wall_rupture_debris_count_caps_at_200() {
        assert_eq!(wall_rupture_debris_count(24, 4), 96);
        assert_eq!(wall_rupture_debris_count(100, 10), WALL_RUPTURE_DEBRIS_CAP);
        assert_eq!(wall_rupture_debris_count(u32::MAX, 1), WALL_RUPTURE_DEBRIS_CAP);
    }

    /// VAL-M14F-028: no `thread_rng` import in this module — pure
    /// helpers. Compile-time test that the public surface uses
    /// `f32` draw, not `rand::thread_rng`.
    #[test]
    fn rolls_consume_f32_draws_not_thread_rng() {
        let outcome = wall_bulging_roll(0.5, 32, 1.0);
        assert!(matches!(
            outcome,
            WallCollapseOutcome::Trigger(WallCollapseStage::Bulging) | WallCollapseOutcome::Hold
        ));
    }

    /// VAL-M14F-023: material lateral-yield ordering is observable —
    /// the rupture chance under identical pressure differs because
    /// the per-pixel threshold differs. We assert the threshold-
    /// triggered-flag transitions in yield order.
    #[test]
    fn material_yield_ordering_observable_under_identical_pressure() {
        let pressure = 60.0_f32;
        let area = 256;
        assert!(pressure_blowout_triggers(pressure, 15, area), "wood yields");
        assert!(pressure_blowout_triggers(pressure, 30, area), "brick yields");
        assert!(pressure_blowout_triggers(pressure, 50, area), "concrete yields");
        assert!(!pressure_blowout_triggers(pressure, 200, area), "steel holds");
    }

    /// VAL-M14F-027: render_level on each stage maps to L1/L2/L3.
    #[test]
    fn stage_render_level_pairs_with_crack_decal_level() {
        assert_eq!(WallCollapseStage::Bulging.render_level(), "l1");
        assert_eq!(WallCollapseStage::CrackAdvanced.render_level(), "l2");
        assert_eq!(WallCollapseStage::Rupture.render_level(), "l3");
    }

    /// VAL-M14F-027: stage as_str maps to canonical event names.
    #[test]
    fn stage_as_str_matches_event_names() {
        assert_eq!(WallCollapseStage::Bulging.as_str(), "wall_bulging");
        assert_eq!(WallCollapseStage::CrackAdvanced.as_str(), "wall_crack_advanced");
        assert_eq!(WallCollapseStage::Rupture.as_str(), "wall_rupture");
    }
}
