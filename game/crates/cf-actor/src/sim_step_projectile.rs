//! Per-tick projectile integration + swept-collision priority queue.
//!
//! Extracted from [`crate::sim`] for file size.

use crate::sim::{ActorSimState, ExpiredProjectile, HitOutcome, Projectile, StepDeps, StepReport, zone_from_hit};
use crate::{ActorId, Vec2};

pub(crate) fn step_projectiles(state: &mut ActorSimState, deps: StepDeps, report: &mut StepReport) {
    let mut survivors: Vec<Projectile> = Vec::with_capacity(state.projectiles.len());
    let projectiles = std::mem::take(&mut state.projectiles);
    for mut projectile in projectiles {
        let start = projectile.position;
        let end = Vec2::new(
            projectile.position.x + projectile.velocity.x * deps.tick_dt,
            projectile.position.y + projectile.velocity.y * deps.tick_dt,
        );
        projectile.position = end;
        if projectile.remaining_ticks > 0 {
            projectile.remaining_ticks -= 1;
        }
        // queue. Collect EVERY actor whose AABB the segment crosses this
        // tick, sort by entry-t ascending (ties: ActorId), then resolve
        // hits in priority order. The projectile carries `damage` as
        // energy; each hit absorbs energy + the projectile continues
        // through actors as long as energy > 0 (per CCCP Atom::Travel +
        // MovableObject::CollideAtPoint). Without this, every projectile
        // could only ever hit one actor per tick — even though the M14
        // swept-collision priority queue helper expects multi-actor hits.
        //
        // The ray origin + direction + entry_t + distance_traveled are
        // latched on each HitOutcome so the engine emits the
        // `combat.swept_collision` event with accurate metadata even when
        // a multi-tick projectile resolves its hits on later ticks.
        let seg_dx = end.x - start.x;
        let seg_dy = end.y - start.y;
        let seg_len = (seg_dx * seg_dx + seg_dy * seg_dy).sqrt().max(f32::EPSILON);
        let ray_dir = Vec2::new(seg_dx / seg_len, seg_dy / seg_len);
        let mut candidates: Vec<(ActorId, f32, Vec2)> = Vec::new();
        for actor in state.world.actors.values() {
            if actor.id == projectile.owner {
                continue;
            }
            if actor.status.is_dead() {
                continue;
            }
            if let Some(t) = segment_hits_aabb(start, end, actor.position, actor.half_extents) {
                let hit_pos = Vec2::new(start.x + seg_dx * t, start.y + seg_dy * t);
                candidates.push((actor.id, t, hit_pos));
            }
        }
        candidates.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        let mut any_hit = false;
        let mut remaining_damage = projectile.damage;
        let candidate_count = candidates.len();
        for (idx, (target_id, entry_t, hit_pos)) in candidates.into_iter().enumerate() {
            if remaining_damage <= 0.0 {
                break;
            }
            let distance_traveled = seg_len * entry_t;
            let target = state
                .world
                .actors
                .get_mut(&target_id)
                .expect("hit target must exist by construction");
            let previous_status = target.status;
            // either STOPS in the target (delivers full remaining damage)
            // OR passes through (60% absorbed by the actor, 40% continues
            // to the next actor in priority order).
            //
            // passthroughs (was 1.66× over-applied), AND when there's only
            // ONE candidate (the common single-target case), the entire
            // remaining damage lands on that one actor — the 60/40 split
            // applies ONLY to actual passthroughs.
            let is_last_candidate = idx + 1 == candidate_count;
            let damage_this_hit = if is_last_candidate {
                remaining_damage
            } else {
                remaining_damage * 0.6
            };
            let (chassis_outcome, zone_label) = if target.chassis.is_some() {
                let zone = zone_from_hit(target.position, target.half_extents, hit_pos);
                let (_, outcome) = target.apply_zone_damage(zone, damage_this_hit, "projectile_hit");
                (Some(outcome), zone.as_str().to_string())
            } else {
                let _ = target.apply_damage(damage_this_hit);
                (None, "torso".to_string())
            };
            let new_status = target.status;
            report.hits.push(HitOutcome {
                projectile_id: projectile.id,
                shooter: projectile.owner,
                target: target_id,
                hit_position: hit_pos,
                damage: damage_this_hit,
                previous_status,
                new_status,
                zone: zone_label,
                chassis_outcome,
                entry_t,
                ray_origin: start,
                ray_direction: ray_dir,
                distance_traveled,
            });
            any_hit = true;
            if is_last_candidate {
                remaining_damage = 0.0;
            } else {
                remaining_damage = (remaining_damage - damage_this_hit).max(0.0);
            }
            if remaining_damage < 0.5 {
                break;
            }
        }
        if any_hit {
            continue;
        }
        let oob = projectile.position.x < deps.region_min_x - 64.0
            || projectile.position.x > deps.region_max_x + 64.0
            || projectile.position.y < state.world.floor_y - 64.0
            || projectile.position.y > deps.region_max_y + 64.0;
        if oob || projectile.remaining_ticks == 0 {
            report.expired_projectiles.push(ExpiredProjectile {
                id: projectile.id,
                owner: projectile.owner,
                last_position: projectile.position,
            });
            continue;
        }
        survivors.push(projectile);
    }
    state.projectiles = survivors;
}

/// Returns the entry parameter `t` in `[0, 1]` for the segment `start -> end` against the
/// AABB centred on `centre` with `half_extents`, or `None` if the segment misses. A point
/// already inside the AABB at `start` returns `Some(0.0)`.
///
/// **DR-033 forward-hook**: thin `Vec2` adapter that delegates to
/// `cf_physics::segment_hits_aabb` so M5.5's broadphase/narrowphase can build on
/// the shared swept primitive without depending on `cf-actor`.
fn segment_hits_aabb(start: Vec2, end: Vec2, centre: Vec2, half_extents: Vec2) -> Option<f32> {
    cf_physics::segment_hits_aabb(
        (start.x, start.y),
        (end.x, end.y),
        (centre.x, centre.y),
        (half_extents.x, half_extents.y),
    )
}
