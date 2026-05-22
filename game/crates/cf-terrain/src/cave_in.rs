//! **M14E** § Cave-in roll + falling-debris spawn + cascade-to-neighbor.
//!
//! Pure helpers: state in, state out. No clock, no `thread_rng`.
//! Callers supply an `f32` RNG draw from the engine's seeded RNG when
//! the cave-in roll fires (`engine.rng.next_f32()`). Determinism is
//! guaranteed by the seeded-draw contract — same seed → same outcome
//! (VAL-M14E-017).
//!
//! Two surfaces:
//!   - [`cave_in_chance_per_tick`] computes the spec-literal probability
//!     `(unsupported_span_px - 16) × 0.0001 × vibration_modifier`.
//!   - [`falling_debris_count`] returns `min(span_px × ceiling_thickness, 200)`.
//!   - [`CaveInOutcome::roll`] consumes a uniform `f32 ∈ [0, 1)` draw
//!     and returns Trigger or Hold.
//!
//! Cascade detection ([`CascadeNeighbor::for_chunk`]) reports which
//! neighbor chunk coordinates should re-run `compute_integrity_pass`
//! after a primary cave-in. Reuses the M18 `terrain.terrain_cascade`
//! event family per VAL-M14E-026.

use serde::{Deserialize, Serialize};

/// Hard ceiling on falling-debris pixel count per cave-in event (bounds
/// cosmetic load per spec Notes "to bound cosmetic load").
pub const FALLING_DEBRIS_CAP: u32 = 200;

/// Spec literal coefficient for the cave-in probability formula.
/// `cave_in_chance_per_tick = (unsupported_span_px - 16) × 0.0001 × vibration_modifier`
pub const CAVE_IN_BASE_COEFFICIENT: f32 = 0.000_1;

/// Spec literal: tunnel widths at or below this threshold do not cave
/// in regardless of vibration (the 16-pixel floor in the formula).
pub const UNSUPPORTED_SPAN_FLOOR_PX: u32 = 16;

/// Vibration multiplier baseline for plain digger / pickaxe ("normal"
/// dig). Plasma cutter / mining laser multipliers stack on top (per
/// VAL-M14E-015 the plasma cutter doubles this).
pub const VIBRATION_MODIFIER_BASELINE: f32 = 1.0;
pub const VIBRATION_MODIFIER_PLASMA_CUTTER: f32 = 2.0;

/// Per-tick cave-in probability per the spec literal formula:
/// `(unsupported_span_px - 16) × 0.0001 × vibration_modifier`.
/// Returns `0.0` for spans at or below the 16-pixel floor.
/// Per VAL-M14E-020 the constant `16`, multiplier `0.0001`, and the
/// `vibration_modifier` factor must appear bit-equal in this order.
#[must_use]
pub fn cave_in_chance_per_tick(unsupported_span_px: u32, vibration_modifier: f32) -> f32 {
    if unsupported_span_px <= UNSUPPORTED_SPAN_FLOOR_PX {
        return 0.0;
    }
    let above_floor = (unsupported_span_px - UNSUPPORTED_SPAN_FLOOR_PX) as f32;
    above_floor * CAVE_IN_BASE_COEFFICIENT * vibration_modifier.max(0.0)
}

/// Falling-debris pixel count per cave-in event: `min(span_px × ceiling_thickness, 200)`.
/// Per VAL-M14E-021 and the spec Notes literal.
#[must_use]
pub fn falling_debris_count(span_px: u32, ceiling_thickness_px: u32) -> u32 {
    span_px.saturating_mul(ceiling_thickness_px).min(FALLING_DEBRIS_CAP)
}

/// Per-tick cave-in roll outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaveInOutcome {
    /// Roll fired — engine emits `terrain.cave_in_triggered`.
    Trigger,
    /// Roll did not fire this tick.
    Hold,
}

impl CaveInOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            CaveInOutcome::Trigger => "trigger",
            CaveInOutcome::Hold => "hold",
        }
    }

    pub fn fired(self) -> bool {
        matches!(self, CaveInOutcome::Trigger)
    }
}

/// One cave-in roll. Consumes a uniform `f32 ∈ [0, 1)` draw from the
/// engine's seeded RNG and the per-tick chance from
/// [`cave_in_chance_per_tick`]; returns Trigger or Hold.
///
/// Per VAL-M14E-017 callers MUST source `rng_draw` from the engine's
/// seeded RNG (never from `thread_rng`).
#[must_use]
pub fn cave_in_roll(rng_draw: f32, unsupported_span_px: u32, vibration_modifier: f32) -> CaveInOutcome {
    let chance = cave_in_chance_per_tick(unsupported_span_px, vibration_modifier);
    if rng_draw.is_nan() || chance <= 0.0 {
        return CaveInOutcome::Hold;
    }
    if rng_draw < chance.clamp(0.0, 1.0) {
        CaveInOutcome::Trigger
    } else {
        CaveInOutcome::Hold
    }
}

/// One neighbor chunk that should re-run the integrity pass after a
/// primary cave-in (cascade detection). Used by
/// [`cascade_neighbors_for_chunk`] which returns the four side
/// neighbors per Stationeers parity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CascadeNeighbor {
    pub cx: i32,
    pub cy: i32,
}

/// Compute the four side-neighbor chunk coordinates of `(cx, cy)`. The
/// engine schedules `compute_integrity_pass` on these chunks within
/// `INTEGRITY_PASS_CADENCE_TICKS` of the primary cave-in (VAL-M14E-018).
/// Deterministic ordering: north, south, west, east.
#[must_use]
pub fn cascade_neighbors_for_chunk(cx: i32, cy: i32) -> [CascadeNeighbor; 4] {
    [
        CascadeNeighbor { cx, cy: cy - 1 },
        CascadeNeighbor { cx, cy: cy + 1 },
        CascadeNeighbor { cx: cx - 1, cy },
        CascadeNeighbor { cx: cx + 1, cy },
    ]
}

/// Snapshot payload for `terrain.cave_in_triggered`. The engine reads
/// this directly into the replay event payload. Per VAL-M14E-004 the
/// event must carry `chunk_id`, `bbox`, and `falling_debris_count`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaveInPayload {
    /// Chunk coord encoded as `(cx, cy)` per the project's chunk key.
    pub chunk_id: (i32, i32),
    /// Inclusive pixel-space AABB of the collapsing ceiling.
    pub bbox_min: [i64; 2],
    pub bbox_max: [i64; 2],
    /// Pixel count per `falling_debris_count(span_px, ceiling_thickness)`.
    pub falling_debris_count: u32,
    /// Unsupported span (px) the roll fired on. Helpful for debug.
    pub unsupported_span_px: u32,
    /// Vibration modifier the roll consumed.
    pub vibration_modifier: f32,
    /// Per-tick chance the roll consumed (`cave_in_chance_per_tick`).
    pub chance_per_tick: f32,
    /// True when this cave-in is the primary; false when secondary
    /// (cascaded from a neighbor). VAL-M14E-018 / VAL-M14E-026.
    pub cascade_primary: bool,
}

impl CaveInPayload {
    /// Construct a primary-cave-in payload.
    #[must_use]
    pub fn primary(
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        ceiling_thickness_px: u32,
        vibration_modifier: f32,
    ) -> Self {
        Self {
            chunk_id,
            bbox_min,
            bbox_max,
            falling_debris_count: falling_debris_count(unsupported_span_px, ceiling_thickness_px),
            unsupported_span_px,
            vibration_modifier,
            chance_per_tick: cave_in_chance_per_tick(unsupported_span_px, vibration_modifier),
            cascade_primary: true,
        }
    }

    /// Construct a secondary (cascaded) cave-in payload.
    #[must_use]
    pub fn cascade(
        chunk_id: (i32, i32),
        bbox_min: [i64; 2],
        bbox_max: [i64; 2],
        unsupported_span_px: u32,
        ceiling_thickness_px: u32,
        vibration_modifier: f32,
    ) -> Self {
        Self {
            cascade_primary: false,
            ..Self::primary(
                chunk_id,
                bbox_min,
                bbox_max,
                unsupported_span_px,
                ceiling_thickness_px,
                vibration_modifier,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// across 6 input cases bit-equal to the formula.
    #[test]
    fn chance_formula_matches_spec_literal() {
        let cases = [
            (17u32, 1.0_f32, (17 - 16) as f32 * 0.0001 * 1.0),
            (24, 1.0, (24 - 16) as f32 * 0.0001 * 1.0),
            (32, 1.0, (32 - 16) as f32 * 0.0001 * 1.0),
            (48, 1.0, (48 - 16) as f32 * 0.0001 * 1.0),
            (24, 2.0, (24 - 16) as f32 * 0.0001 * 2.0),
            (32, 2.0, (32 - 16) as f32 * 0.0001 * 2.0),
        ];
        for (span, vib, expected) in cases {
            let got = cave_in_chance_per_tick(span, vib);
            assert!(
                (got - expected).abs() < 1e-9,
                "span={span} vib={vib}: expected {expected}, got {got}"
            );
        }
    }

    #[test]
    fn chance_zero_at_or_below_16_px_floor() {
        assert_eq!(cave_in_chance_per_tick(0, 1.0), 0.0);
        assert_eq!(cave_in_chance_per_tick(10, 1.0), 0.0);
        assert_eq!(cave_in_chance_per_tick(16, 1.0), 0.0);
        assert_eq!(cave_in_chance_per_tick(17, 0.0), 0.0);
    }

    /// across the 3 spec-cited input cases.
    #[test]
    fn falling_debris_count_matches_spec_table() {
        assert_eq!(falling_debris_count(24, 4), 96);
        assert_eq!(falling_debris_count(48, 8), 200);
        assert_eq!(falling_debris_count(100, 10), 200);
    }

    #[test]
    fn falling_debris_count_caps_at_200() {
        assert_eq!(falling_debris_count(1000, 100), FALLING_DEBRIS_CAP);
        assert_eq!(falling_debris_count(u32::MAX, 1), FALLING_DEBRIS_CAP);
    }

    #[test]
    fn roll_triggers_when_draw_below_chance() {
        // span=32 vib=1.0 → chance=0.0016; draw=0.0001 → trigger.
        assert_eq!(cave_in_roll(0.0001, 32, 1.0), CaveInOutcome::Trigger);
    }

    #[test]
    fn roll_holds_when_draw_above_chance() {
        assert_eq!(cave_in_roll(0.999, 32, 1.0), CaveInOutcome::Hold);
        assert_eq!(cave_in_roll(0.5, 32, 1.0), CaveInOutcome::Hold);
    }

    #[test]
    fn roll_holds_at_or_below_16_px_floor() {
        // chance always 0 → never trigger.
        for span in [0u32, 1, 10, 15, 16] {
            for vib in [0.0_f32, 0.5, 1.0, 4.0] {
                assert_eq!(cave_in_roll(0.0, span, vib), CaveInOutcome::Hold);
            }
        }
    }

    #[test]
    fn roll_holds_on_nan_draw() {
        // Defensive: NaN draws never trigger.
        assert_eq!(cave_in_roll(f32::NAN, 32, 1.0), CaveInOutcome::Hold);
    }

    /// order.
    #[test]
    fn cascade_returns_four_side_neighbors_in_canonical_order() {
        let nbrs = cascade_neighbors_for_chunk(3, 5);
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

    #[test]
    fn cave_in_payload_primary_carries_required_fields() {
        let p = CaveInPayload::primary((1, 2), [16, 32], [48, 64], 32, 4, 1.0);
        assert_eq!(p.chunk_id, (1, 2));
        assert_eq!(p.bbox_min, [16, 32]);
        assert_eq!(p.bbox_max, [48, 64]);
        assert_eq!(p.falling_debris_count, 128);
        assert!(p.cascade_primary);
    }

    #[test]
    fn cave_in_payload_cascade_carries_secondary_flag() {
        let p = CaveInPayload::cascade((1, 2), [16, 32], [48, 64], 32, 4, 1.0);
        assert!(!p.cascade_primary);
    }

    /// at tick 200 instead of tick 600.
    #[test]
    fn plasma_cutter_doubles_chance_per_tick() {
        let baseline = cave_in_chance_per_tick(32, VIBRATION_MODIFIER_BASELINE);
        let plasma = cave_in_chance_per_tick(32, VIBRATION_MODIFIER_PLASMA_CUTTER);
        assert!(
            (plasma - 2.0 * baseline).abs() < 1e-9,
            "plasma vibration must exactly double the baseline chance"
        );
    }

    /// produces the same outcome on every call.
    #[test]
    fn roll_is_deterministic_for_fixed_inputs() {
        for draw in [0.0_f32, 0.0001, 0.001, 0.5, 0.999] {
            for span in [17u32, 24, 32, 48] {
                let a = cave_in_roll(draw, span, 1.0);
                let b = cave_in_roll(draw, span, 1.0);
                let c = cave_in_roll(draw, span, 1.0);
                assert_eq!(a, b);
                assert_eq!(b, c);
            }
        }
    }
}
