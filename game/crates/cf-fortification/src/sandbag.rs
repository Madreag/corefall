//! M9C § "Sandbag wall (3 height tiers)": per-pixel-erodible sandbag
//! wall with three height tiers.
//!
//! Per the spec table:
//!
//! | Tier            | Height | HP  | Cover (Standing) | Cover (Crouched) |
//! |-----------------|--------|-----|------------------|------------------|
//! | `sandbag_low`   | 4 px   | 200 | None             | Partial          |
//! | `sandbag_mid`   | 8 px   | 400 | Partial          | Full             |
//! | `sandbag_high`  | 12 px  | 600 | Full             | Full             |
//!
//! Per the spec's Gherkin acceptance scenario:
//!
//! > Sandbag walls **degrade per-pixel** (M14): hit a high wall with
//! > sustained MG and the top row erodes first, downgrading the tier
//! > (high → mid → low → gone) over time. `sandbag_eroded` event
//! > fires per tier-drop.
//!
//! The kernel exposes:
//!
//! - [`SandbagTier`] — enum of the three tiers (`Low | Mid | High`).
//! - [`SandbagWall`] — placed wall: tier, current HP, per-pixel mask.
//! - [`apply_damage_to_wall`] — erode the top row first, emit
//!   `SandbagErodedEvent` on tier transitions (HP < 400 high→mid,
//!   HP < 200 mid→low, HP 0 removed).
//! - [`sandbag_eroded_events`] — pure helper that returns the
//!   transitions for an `(old_hp, new_hp)` pair (used by cf-control
//!   to emit events without re-rolling the per-pixel mask).
//!
//! VAL-M9C-016 / VAL-M9C-017 land here.

use serde::{Deserialize, Serialize};

use crate::common::FortificationId;

/// Maximum HP of a freshly built `sandbag_high` wall (spec table).
pub const SANDBAG_HIGH_MAX_HP: u32 = 600;
/// Maximum HP of a freshly built `sandbag_mid` wall (spec table).
pub const SANDBAG_MID_MAX_HP: u32 = 400;
/// Maximum HP of a freshly built `sandbag_low` wall (spec table).
pub const SANDBAG_LOW_MAX_HP: u32 = 200;

/// Per-tile pixel-height of each tier. The per-pixel erosion mask is
/// `height` rows tall; the top row is row 0 and is destroyed first.
/// Spec § "Sandbag wall (3 height tiers)" table.
pub const SANDBAG_HIGH_HEIGHT_PX: u32 = 12;
pub const SANDBAG_MID_HEIGHT_PX: u32 = 8;
pub const SANDBAG_LOW_HEIGHT_PX: u32 = 4;

/// Three sandbag-wall height tiers per the spec table.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandbagTier {
    Low = 0,
    Mid = 1,
    High = 2,
}

impl SandbagTier {
    pub const ALL: [SandbagTier; 3] = [
        SandbagTier::Low,
        SandbagTier::Mid,
        SandbagTier::High,
    ];

    /// Pixel height of the tier; the per-pixel mask is this many rows
    /// tall with the top row at row index 0.
    #[must_use]
    pub const fn height_px(self) -> u32 {
        match self {
            SandbagTier::Low => SANDBAG_LOW_HEIGHT_PX,
            SandbagTier::Mid => SANDBAG_MID_HEIGHT_PX,
            SandbagTier::High => SANDBAG_HIGH_HEIGHT_PX,
        }
    }

    /// HP cap for a freshly built wall at this tier.
    #[must_use]
    pub const fn max_hp(self) -> u32 {
        match self {
            SandbagTier::Low => SANDBAG_LOW_MAX_HP,
            SandbagTier::Mid => SANDBAG_MID_MAX_HP,
            SandbagTier::High => SANDBAG_HIGH_MAX_HP,
        }
    }

    /// Stable string id used on replay-event payloads (spec scenario
    /// declares `sandbag_eroded { from: "high", to: "mid" }`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SandbagTier::Low => "low",
            SandbagTier::Mid => "mid",
            SandbagTier::High => "high",
        }
    }
}

/// On-disk spec for one of the three sandbag-wall fortifications
/// (`sandbag_low` / `sandbag_mid` / `sandbag_high`) authored under
/// `content/fortifications/`. The RON file pins the per-tier HP cap
/// + footprint so modders can override.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SandbagWallSpec {
    pub tier: SandbagTier,
    pub max_hp: u32,
    pub footprint_tiles: (u32, u32),
    /// `(width_tiles, height_px)`; height_px MUST equal tier.height_px().
    pub height_px: u32,
}

impl SandbagWallSpec {
    pub fn from_ron_str(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str::<SandbagWallSpec>(text)
    }
}

/// Placed sandbag wall in the world. The pixel mask is owned by the
/// wall so erosion (M14 per-pixel damage routing) can drive HP
/// downward + tier transitions in lockstep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandbagWall {
    pub id: FortificationId,
    pub tier: SandbagTier,
    pub hp: u32,
    pub pixel_mask: SandbagPixelMask,
}

impl SandbagWall {
    /// Construct a freshly built wall at the supplied tier with the
    /// tier's full HP and a fully populated pixel mask. `width_px` is
    /// the wall's horizontal extent in pixels (the kernel does not
    /// assume a fixed tile-count; modders may author wider walls in
    /// content RON).
    #[must_use]
    pub fn new_full(id: FortificationId, tier: SandbagTier, width_px: u32) -> Self {
        Self {
            id,
            tier,
            hp: tier.max_hp(),
            pixel_mask: SandbagPixelMask::full(tier.height_px(), width_px),
        }
    }

    /// True when the wall has been entirely destroyed (HP 0 + every
    /// pixel eroded).
    #[must_use]
    pub fn is_destroyed(&self) -> bool {
        self.hp == 0 && self.pixel_mask.alive_count() == 0
    }
}

/// Per-pixel mask owned by a sandbag wall.
///
/// `rows[0]` is the **top** row (eroded first per spec scenario:
/// "the top row of sandbags is destroyed first"). Each row is a
/// `Vec<bool>` of length `width`; `true` means the pixel is intact.
///
/// The mask is stored owned (rather than as a bitmap) because the
/// kernel runs at sandbag-wall resolution (≤ 12 rows × ≤ 48 px wide);
/// the memory footprint is trivial and the simple shape keeps unit
/// tests readable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandbagPixelMask {
    pub rows: Vec<Vec<bool>>,
}

impl SandbagPixelMask {
    /// Build a fully intact mask `(height, width)`.
    #[must_use]
    pub fn full(height_px: u32, width_px: u32) -> Self {
        let h = height_px as usize;
        let w = width_px as usize;
        Self {
            rows: vec![vec![true; w]; h],
        }
    }

    /// Count of pixels still intact.
    #[must_use]
    pub fn alive_count(&self) -> u32 {
        self.rows
            .iter()
            .map(|row| row.iter().filter(|p| **p).count() as u32)
            .sum()
    }

    /// Count of pixels in the topmost intact row that are still alive.
    /// Returns 0 when every pixel is gone.
    #[must_use]
    pub fn top_row_alive_count(&self) -> u32 {
        self.first_intact_row()
            .map_or(0, |idx| self.rows[idx].iter().filter(|p| **p).count() as u32)
    }

    /// Index of the topmost row that still has at least one intact pixel.
    /// `None` when the mask is fully eroded.
    #[must_use]
    pub fn first_intact_row(&self) -> Option<usize> {
        self.rows
            .iter()
            .position(|row| row.iter().any(|p| *p))
    }

    /// Index of the bottommost row that still has at least one intact pixel.
    /// `None` when the mask is fully eroded.
    #[must_use]
    pub fn last_intact_row(&self) -> Option<usize> {
        self.rows
            .iter()
            .enumerate()
            .rev()
            .find(|(_, row)| row.iter().any(|p| *p))
            .map(|(i, _)| i)
    }

    /// Erode `pixel_count` pixels strictly from the top row first. The
    /// erosion sweeps left-to-right within a row before moving down to
    /// the next row.
    ///
    /// Returns the number of pixels actually eroded (saturates at the
    /// remaining intact pixel count).
    pub fn erode_from_top(&mut self, pixel_count: u32) -> u32 {
        let mut to_erode = pixel_count;
        let mut eroded = 0u32;
        let row_count = self.rows.len();
        for row_idx in 0..row_count {
            if to_erode == 0 {
                break;
            }
            let row_len = self.rows[row_idx].len();
            for col in 0..row_len {
                if to_erode == 0 {
                    break;
                }
                if self.rows[row_idx][col] {
                    self.rows[row_idx][col] = false;
                    to_erode -= 1;
                    eroded += 1;
                }
            }
        }
        eroded
    }
}

/// Public helper used by cf-mod loader tests + cf-control snapshot
/// diagnostics. Returns the freshly-built pixel mask for a tier given
/// a wall width in pixels.
#[must_use]
pub fn sandbag_pixel_mask(tier: SandbagTier, width_px: u32) -> SandbagPixelMask {
    SandbagPixelMask::full(tier.height_px(), width_px)
}

/// Spec § Sandbag-wall acceptance: classify an HP into the active tier.
///
/// > And when HP drops below 400, sandbag_eroded event fires with
/// > from=high to=mid
/// > … When HP drops below 200, sandbag_eroded fires again with
/// > from=mid to=low
/// > … When HP reaches 0 the wall is destroyed entirely
///
/// Returns `Some(tier)` while the wall is intact; `None` once HP hits
/// 0 (the wall is removed from the world).
#[must_use]
pub fn sandbag_tier_for_hp(hp: u32) -> Option<SandbagTier> {
    if hp == 0 {
        None
    } else if hp < SANDBAG_MID_MAX_HP {
        // < 400
        if hp < SANDBAG_LOW_MAX_HP {
            // < 200 → low
            Some(SandbagTier::Low)
        } else {
            // 200..399 → mid
            Some(SandbagTier::Mid)
        }
    } else {
        // 400..600 → high
        Some(SandbagTier::High)
    }
}

/// Replay event emitted on a tier transition. Spec field names match
/// the acceptance scenario verbatim: `from: "high", to: "mid"` /
/// `from: "mid", to: "low"`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandbagErodedEvent {
    pub fortification_id: FortificationId,
    pub from: SandbagTier,
    pub to: SandbagTier,
}

/// Pure helper: given an HP delta on a sandbag wall (old → new),
/// return the ordered list of `SandbagErodedEvent` records the
/// emitter must publish. Both tier transitions (high→mid AND
/// mid→low) may fire in the same delta if a single hit punches HP
/// through both thresholds.
///
/// `id` is the wall's fortification handle (echoed on every event).
#[must_use]
pub fn sandbag_eroded_events(
    id: FortificationId,
    old_hp: u32,
    new_hp: u32,
) -> Vec<SandbagErodedEvent> {
    if new_hp >= old_hp {
        return vec![];
    }
    let from_tier = sandbag_tier_for_hp(old_hp);
    let to_tier = sandbag_tier_for_hp(new_hp);
    let mut out = vec![];
    if let (Some(from), Some(to)) = (from_tier, to_tier) {
        if from == to {
            return out;
        }
        // Spec table is ordered high → mid → low, with hp thresholds
        // 400 (high→mid) and 200 (mid→low). Emit one event per crossed
        // threshold; "from" is the tier above the crossed threshold.
        match (from, to) {
            (SandbagTier::High, SandbagTier::Mid) => out.push(SandbagErodedEvent {
                fortification_id: id,
                from: SandbagTier::High,
                to: SandbagTier::Mid,
            }),
            (SandbagTier::Mid, SandbagTier::Low) => out.push(SandbagErodedEvent {
                fortification_id: id,
                from: SandbagTier::Mid,
                to: SandbagTier::Low,
            }),
            (SandbagTier::High, SandbagTier::Low) => {
                out.push(SandbagErodedEvent {
                    fortification_id: id,
                    from: SandbagTier::High,
                    to: SandbagTier::Mid,
                });
                out.push(SandbagErodedEvent {
                    fortification_id: id,
                    from: SandbagTier::Mid,
                    to: SandbagTier::Low,
                });
            }
            _ => {}
        }
    } else if from_tier.is_some() && to_tier.is_none() {
        // Wall destroyed entirely. The spec doesn't define a "to: gone"
        // event shape for sandbag_eroded; cf-control emits a separate
        // fortification_destroyed event for HP=0. We still emit the
        // crossed mid→low / high→mid transitions on the way down so
        // the replay reflects every threshold crossed before death.
        match from_tier {
            Some(SandbagTier::High) => {
                out.push(SandbagErodedEvent {
                    fortification_id: id,
                    from: SandbagTier::High,
                    to: SandbagTier::Mid,
                });
                out.push(SandbagErodedEvent {
                    fortification_id: id,
                    from: SandbagTier::Mid,
                    to: SandbagTier::Low,
                });
            }
            Some(SandbagTier::Mid) => out.push(SandbagErodedEvent {
                fortification_id: id,
                from: SandbagTier::Mid,
                to: SandbagTier::Low,
            }),
            _ => {}
        }
    }
    out
}

/// Spec § "Repair: act.player.repair_fortification consumes sandbags
/// from inventory equal to the missing HP". The ratio is **50 HP
/// restored per consumed sandbag** for the sandbag-wall family
/// (closure-feature `m9c-6` description verbatim).
///
/// This constant is the engine-side anchor the cfctl
/// `act.player.repair_fortification` handler reads to compute the
/// per-call inventory deduction + HP restoration.
pub const SANDBAG_REPAIR_HP_PER_SANDBAG: u32 = 50;

/// Pure helper: given a wall's current HP + tier, compute the
/// `(sandbags_required, hp_after_repair)` pair for a full repair to
/// `tier.max_hp()`. Returns `(0, current_hp)` when the wall is
/// already at max HP.
///
/// Used by the engine-side dispatch for
/// `act.player.repair_fortification` to size the inventory deduction
/// before applying the restoration (VAL-M9C-REPAIR-FORTIFICATION-
/// BEHAVIOR).
#[must_use]
pub fn sandbag_repair_cost(current_hp: u32, tier: SandbagTier) -> (u32, u32) {
    let max_hp = tier.max_hp();
    if current_hp >= max_hp {
        return (0, current_hp);
    }
    let missing = max_hp - current_hp;
    let sandbags = missing.div_ceil(SANDBAG_REPAIR_HP_PER_SANDBAG);
    (sandbags, max_hp)
}

/// Apply a repair to a sandbag wall: consume up to `available_sandbags`
/// to restore HP toward `tier.max_hp()` at the spec's 50 HP / sandbag
/// ratio. Returns the `(sandbags_consumed, hp_after)` pair. If the
/// caller cannot supply enough sandbags for a full repair, the
/// helper applies a partial repair scaled to what's available.
///
/// Used by the engine-side `act.player.repair_fortification`
/// dispatch (VAL-M9C-REPAIR-FORTIFICATION-BEHAVIOR).
pub fn repair_sandbag_wall(wall: &mut SandbagWall, available_sandbags: u32) -> (u32, u32) {
    let (needed, _max) = sandbag_repair_cost(wall.hp, wall.tier);
    if needed == 0 || available_sandbags == 0 {
        return (0, wall.hp);
    }
    let consumed = needed.min(available_sandbags);
    let restored_hp = consumed.saturating_mul(SANDBAG_REPAIR_HP_PER_SANDBAG);
    let new_hp = wall.hp.saturating_add(restored_hp).min(wall.tier.max_hp());
    wall.hp = new_hp;
    if let Some(tier) = sandbag_tier_for_hp(new_hp) {
        wall.tier = tier;
    }
    (consumed, new_hp)
}

/// Apply `damage_hp` to the wall: clamp HP, erode pixels strictly
/// from the top row first (per VAL-M9C-017), and return the ordered
/// list of `sandbag_eroded` events the caller must publish (per
/// VAL-M9C-016).
///
/// The erosion ratio is "1 pixel per (max_hp / total_pixels) HP" so
/// a freshly-built `sandbag_high` (HP 600, height 12) erodes 1 pixel
/// per 1 HP if it's 50 px wide.
pub fn apply_damage_to_wall(wall: &mut SandbagWall, damage_hp: u32) -> Vec<SandbagErodedEvent> {
    let old_hp = wall.hp;
    let new_hp = old_hp.saturating_sub(damage_hp);
    wall.hp = new_hp;
    // Erode pixels proportional to HP loss. The "1 pixel per HP point"
    // shape is what the spec scenario describes — "hit a high wall
    // with sustained MG and the top row erodes first, downgrading the
    // tier over time". We compute the number of pixels-to-erode from
    // the HP delta so a single big hit can collapse the wall.
    let hp_delta = old_hp - new_hp;
    let total_pixels = wall.pixel_mask.alive_count();
    let pixels_to_erode = if old_hp == 0 {
        0
    } else if new_hp == 0 {
        total_pixels
    } else {
        // Erode pixels proportional to HP loss, rounded up so a 1-HP
        // hit at least removes 1 pixel on a full wall.
        let max_hp = wall.tier.max_hp();
        let frac = hp_delta as f64 / max_hp as f64;
        let approx = (frac * total_pixels as f64).ceil() as u32;
        approx.min(total_pixels)
    };
    wall.pixel_mask.erode_from_top(pixels_to_erode);
    let events = sandbag_eroded_events(wall.id, old_hp, new_hp);
    // Tier transitions follow HP per the spec; once HP < 400 the wall
    // is no longer "high" but the in-memory tier label may need to
    // catch up. The next applied damage re-checks via
    // `sandbag_tier_for_hp` so we stamp the live tier eagerly here.
    if let Some(tier) = sandbag_tier_for_hp(new_hp) {
        wall.tier = tier;
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_high() -> SandbagWall {
        SandbagWall::new_full(FortificationId(1), SandbagTier::High, 50)
    }

    /// intact while the top row still has pixels.
    #[test]
    fn sandbag_erodes_top_row_first() {
        let mut wall = fresh_high();
        let width = wall.pixel_mask.rows[0].len();
        let pre_top = wall.pixel_mask.rows[0].iter().filter(|p| **p).count();
        let pre_bottom_pixel = wall.pixel_mask.rows.last().unwrap()[0];
        assert_eq!(pre_top, width, "top row fully intact pre-damage");
        assert!(pre_bottom_pixel, "bottom pixel fully intact pre-damage");

        // Erode a handful of pixels — much less than a full row.
        wall.pixel_mask.erode_from_top(3);
        // Top row lost 3 pixels; nothing else changed.
        let post_top = wall.pixel_mask.rows[0].iter().filter(|p| **p).count();
        assert_eq!(post_top, width - 3);
        for row in wall.pixel_mask.rows.iter().skip(1) {
            assert!(
                row.iter().all(|p| *p),
                "non-top row must remain fully intact"
            );
        }
        // Hammer the entire top row + then some into the second row.
        wall.pixel_mask.erode_from_top((width - 3) as u32 + 2);
        // Top row must reach 0 alive pixels strictly before row 1
        // loses anything beyond the 2 we explicitly removed.
        let r0_after = wall.pixel_mask.rows[0].iter().filter(|p| **p).count();
        let r1_after = wall.pixel_mask.rows[1].iter().filter(|p| **p).count();
        assert_eq!(r0_after, 0, "top row fully eroded before row 1 begins");
        assert_eq!(r1_after, width - 2, "row 1 erodes only after row 0 is dry");
    }

    /// HP 0 destroys the wall.
    #[test]
    fn sandbag_tier_transitions() {
        let mut wall = fresh_high();
        assert_eq!(wall.tier, SandbagTier::High);

        // Big hit driving HP from 600 → 350: tier moves to mid; one
        // sandbag_eroded event high→mid fires.
        let events = apply_damage_to_wall(&mut wall, 250);
        assert_eq!(wall.hp, 350);
        assert_eq!(wall.tier, SandbagTier::Mid);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].from, SandbagTier::High);
        assert_eq!(events[0].to, SandbagTier::Mid);

        // Next hit driving HP from 350 → 150: tier moves to low.
        let events = apply_damage_to_wall(&mut wall, 200);
        assert_eq!(wall.hp, 150);
        assert_eq!(wall.tier, SandbagTier::Low);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].from, SandbagTier::Mid);
        assert_eq!(events[0].to, SandbagTier::Low);

        // Final hit driving HP from 150 → 0: wall destroyed; no
        // further tier transitions fire (the wall is removed).
        let events = apply_damage_to_wall(&mut wall, 200);
        assert_eq!(wall.hp, 0);
        assert!(wall.is_destroyed());
        // Final HP 0 from low tier → no eroded events (tier doesn't drop further).
        assert!(events.is_empty());
    }

    /// A single hit through HP=600 → 50 should fire BOTH transitions
    /// in order: high→mid first, then mid→low.
    #[test]
    fn sandbag_double_transition_in_one_hit() {
        let mut wall = fresh_high();
        let events = apply_damage_to_wall(&mut wall, 550);
        assert_eq!(wall.hp, 50);
        assert_eq!(wall.tier, SandbagTier::Low);
        assert_eq!(events.len(), 2);
        assert_eq!(
            (events[0].from, events[0].to),
            (SandbagTier::High, SandbagTier::Mid)
        );
        assert_eq!(
            (events[1].from, events[1].to),
            (SandbagTier::Mid, SandbagTier::Low)
        );
    }

    /// A hit from full HP straight to 0 should fire both transitions
    /// AND mark the wall destroyed.
    #[test]
    fn sandbag_high_to_zero_emits_both_then_destroys() {
        let mut wall = fresh_high();
        let events = apply_damage_to_wall(&mut wall, 600);
        assert_eq!(wall.hp, 0);
        assert!(wall.is_destroyed());
        assert_eq!(events.len(), 2);
        assert_eq!(
            (events[0].from, events[0].to),
            (SandbagTier::High, SandbagTier::Mid)
        );
        assert_eq!(
            (events[1].from, events[1].to),
            (SandbagTier::Mid, SandbagTier::Low)
        );
    }

    /// ratio computes the right `(sandbags, hp_after)` for the common
    /// repair scenarios.
    #[test]
    fn sandbag_repair_cost_uses_50_hp_per_sandbag() {
        // A `sandbag_high` wall at 300 HP needs (600 - 300) / 50 = 6 sandbags
        // to return to 600 HP.
        assert_eq!(
            sandbag_repair_cost(300, SandbagTier::High),
            (6, 600),
            "high wall at 300 HP costs 6 sandbags + restores to 600 HP"
        );
        // A `sandbag_mid` wall at 200 HP needs (400 - 200) / 50 = 4 sandbags.
        assert_eq!(
            sandbag_repair_cost(200, SandbagTier::Mid),
            (4, 400),
            "mid wall at 200 HP costs 4 sandbags + restores to 400 HP"
        );
        // Already at max → 0 sandbags, no change.
        assert_eq!(
            sandbag_repair_cost(600, SandbagTier::High),
            (0, 600),
            "full HP costs 0 sandbags"
        );
        // Partial-repair ceiling division: 1 HP missing costs 1 sandbag
        // (ceil(1 / 50) = 1).
        assert_eq!(
            sandbag_repair_cost(599, SandbagTier::High),
            (1, 600),
            "ceil-div: 1 HP missing costs 1 sandbag"
        );
    }

    /// updates tier; partial repair scales to available sandbags.
    /// max — `tier` reflects the wall's structural state (eroded
    /// walls degrade tier; repair stabilises at the current tier's
    /// cap). Rebuilding to a higher tier is a separate action.
    #[test]
    fn repair_sandbag_wall_restores_hp_within_current_tier() {
        let mut wall = SandbagWall::new_full(FortificationId(7), SandbagTier::High, 50);
        // Damage wall down to HP=200, which sits at the mid-tier
        // boundary (200..399 → Mid).
        apply_damage_to_wall(&mut wall, 400);
        assert_eq!(wall.hp, 200);
        assert_eq!(wall.tier, SandbagTier::Mid);

        // Repair Mid wall: (400 - 200) / 50 = 4 sandbags → HP back to 400.
        let (consumed, new_hp) = repair_sandbag_wall(&mut wall, 10);
        assert_eq!(consumed, 4);
        assert_eq!(new_hp, 400);
        assert_eq!(wall.tier, SandbagTier::High);
    }

    /// Partial repair: not enough sandbags for a full restore →
    /// scales linearly at 50 HP / sandbag.
    #[test]
    fn repair_sandbag_wall_partial_when_insufficient_inventory() {
        let mut wall = SandbagWall::new_full(FortificationId(7), SandbagTier::High, 50);
        apply_damage_to_wall(&mut wall, 300);
        assert_eq!(wall.hp, 300);
        // Mid tier (HP 300 < 400). Full repair would cost 2 sandbags
        // (400-300)/50 = 2 → HP back to 400. With only 1 sandbag,
        // partial repair restores 50 HP → HP=350 (still Mid).
        let (consumed, new_hp) = repair_sandbag_wall(&mut wall, 1);
        assert_eq!(consumed, 1);
        assert_eq!(new_hp, 350);
        assert_eq!(wall.tier, SandbagTier::Mid);
    }

    /// Repair on an already-full wall is a no-op (no inventory deduction).
    #[test]
    fn repair_sandbag_wall_at_full_hp_is_noop() {
        let mut wall = SandbagWall::new_full(FortificationId(1), SandbagTier::Mid, 25);
        assert_eq!(wall.hp, 400);
        let (consumed, new_hp) = repair_sandbag_wall(&mut wall, 99);
        assert_eq!(consumed, 0);
        assert_eq!(new_hp, 400);
    }

    /// Spec scenario explicit threshold values: HP=399 must be `mid`,
    /// HP=400 still `high`, HP=199 `low`, HP=200 still `mid`.
    #[test]
    fn sandbag_threshold_classification() {
        assert_eq!(sandbag_tier_for_hp(600), Some(SandbagTier::High));
        assert_eq!(sandbag_tier_for_hp(400), Some(SandbagTier::High));
        assert_eq!(sandbag_tier_for_hp(399), Some(SandbagTier::Mid));
        assert_eq!(sandbag_tier_for_hp(200), Some(SandbagTier::Mid));
        assert_eq!(sandbag_tier_for_hp(199), Some(SandbagTier::Low));
        assert_eq!(sandbag_tier_for_hp(1), Some(SandbagTier::Low));
        assert_eq!(sandbag_tier_for_hp(0), None);
    }

    /// Erosion saturates: feeding more pixel_count than alive pixels
    /// drains the mask without panicking.
    #[test]
    fn sandbag_erosion_saturates_at_zero() {
        let mut mask = SandbagPixelMask::full(SANDBAG_LOW_HEIGHT_PX, 10);
        let total = mask.alive_count();
        let actually_eroded = mask.erode_from_top(total + 100);
        assert_eq!(actually_eroded, total);
        assert_eq!(mask.alive_count(), 0);
        assert!(mask.first_intact_row().is_none());
    }

    #[test]
    fn sandbag_tier_metadata() {
        assert_eq!(SandbagTier::Low.height_px(), 4);
        assert_eq!(SandbagTier::Mid.height_px(), 8);
        assert_eq!(SandbagTier::High.height_px(), 12);
        assert_eq!(SandbagTier::Low.max_hp(), 200);
        assert_eq!(SandbagTier::Mid.max_hp(), 400);
        assert_eq!(SandbagTier::High.max_hp(), 600);
    }

    #[test]
    fn sandbag_spec_ron_round_trip() {
        let spec = SandbagWallSpec {
            tier: SandbagTier::High,
            max_hp: SANDBAG_HIGH_MAX_HP,
            footprint_tiles: (3, 1),
            height_px: SANDBAG_HIGH_HEIGHT_PX,
        };
        let s = ron::ser::to_string_pretty(&spec, ron::ser::PrettyConfig::default()).unwrap();
        let parsed = SandbagWallSpec::from_ron_str(&s).unwrap();
        assert_eq!(parsed, spec);
    }
}
