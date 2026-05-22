//! Per-tick loose-item gravity / floor / settle dwell integration.
//!
//! Extracted from [`crate::sim`] for file size.

use crate::sim::{ActorSimState, SettledLooseItem, StepDeps, StepReport};
use crate::Vec2;

/// Loose-item physics constants.
///
/// Tuned for the cfctl scripts in `game/scripts/cfctl/m1_*` (60Hz baseline).
/// All values are in actor sim units (pixels per second). At 60Hz a tick is
/// ~16.67ms, so a gravity of 600 u/s² imparts ~10 u/s of vertical velocity
/// per tick — items fall fast enough that a body dropping from spawn height
/// reaches the floor in roughly 10 ticks, then bounces and settles in
/// ~30 more. The settle dwell window is 10 ticks so observers see a stable
/// `actor.inventory_settled` event well within the 60-tick post-death
/// window the M1 scripts allocate.
const LOOSE_ITEM_GRAVITY_Y: f32 = 600.0;
/// Velocity magnitude below which a loose item is considered "at rest". This
/// avoids declaring settled on jitter from clamped floor collision.
const LOOSE_ITEM_SETTLE_SPEED_THRESHOLD: f32 = 1.0;
/// Number of consecutive ticks on-ground + below-threshold required for
/// settle to fire. 10 ticks @60Hz = ~167ms (and 83ms @120Hz).
const LOOSE_ITEM_SETTLE_DWELL_TICKS: u32 = 10;
/// Coefficient of restitution applied on floor impact. < 1 so items bleed
/// energy and eventually rest.
const LOOSE_ITEM_FLOOR_RESTITUTION: f32 = 0.35;
/// Per-tick horizontal-velocity damping while on the ground.
const LOOSE_ITEM_GROUND_FRICTION: f32 = 0.85;

/// item. Items that already settled this tick clear their one-shot latch;
/// items that newly settle push a `SettledLooseItem` into the step report
/// for the engine to emit `actor.inventory_settled`.
pub(crate) fn step_loose_items(state: &mut ActorSimState, deps: StepDeps, report: &mut StepReport) {
    if state.loose_items.is_empty() {
        return;
    }
    let dt = deps.tick_dt;
    let floor_y = state.world.floor_y;
    for item in &mut state.loose_items {
        // Clear the one-shot latch from the previous tick so post-settle
        // ticks don't double-emit.
        item.just_settled_this_tick = false;

        if !item.settled {
            // Integrate gravity → position.
            item.velocity.y += LOOSE_ITEM_GRAVITY_Y * dt;
            item.position.x += item.velocity.x * dt;
            item.position.y += item.velocity.y * dt;

            // Floor collision. world.floor_y is the floor surface; the
            // item's centre rests at floor_y - half_extents.y when grounded.
            // Below this terminal-bounce threshold we snap vel.y to zero
            // outright; otherwise an infinite series of micro-bounces would
            // converge to the steady-state v* = gravity*dt / (1 + restitution)
            // (~2.6 u/s for the M1 R2 tuning) and never fall below the speed
            // gate — the item would jiggle on the floor forever.
            let terminal_bounce_threshold = (LOOSE_ITEM_GRAVITY_Y * dt) * 1.5;
            let rest_y = floor_y - item.half_extents.y;
            let on_ground = if item.position.y >= rest_y {
                item.position.y = rest_y;
                if item.velocity.y > 0.0 {
                    let bounced = -item.velocity.y * LOOSE_ITEM_FLOOR_RESTITUTION;
                    item.velocity.y = if bounced.abs() < terminal_bounce_threshold {
                        0.0
                    } else {
                        bounced
                    };
                }
                // Apply ground friction to horizontal velocity.
                item.velocity.x *= LOOSE_ITEM_GROUND_FRICTION;
                if item.velocity.x.abs() < 0.5 {
                    item.velocity.x = 0.0;
                }
                true
            } else {
                false
            };

            let speed_squared = item.velocity.x * item.velocity.x + item.velocity.y * item.velocity.y;
            let below_threshold = speed_squared < LOOSE_ITEM_SETTLE_SPEED_THRESHOLD * LOOSE_ITEM_SETTLE_SPEED_THRESHOLD;
            if on_ground && below_threshold {
                item.on_ground_dwell_ticks = item.on_ground_dwell_ticks.saturating_add(1);
                if item.on_ground_dwell_ticks >= LOOSE_ITEM_SETTLE_DWELL_TICKS {
                    item.settled = true;
                    item.just_settled_this_tick = true;
                    // Force exact rest so checksums are byte-stable across
                    // 60Hz vs 120Hz runs (where the integration step
                    // produces different sub-quantization residue).
                    item.velocity = Vec2::ZERO;
                    item.position.y = rest_y;
                    report.settled_loose_items.push(SettledLooseItem {
                        id: item.id,
                        source_event_id: item.source_event_id.clone(),
                        item_label: item.item_label.clone(),
                        position: item.position,
                    });
                }
            } else {
                item.on_ground_dwell_ticks = 0;
            }
        }
    }
}
