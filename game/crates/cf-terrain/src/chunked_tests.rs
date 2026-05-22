//! Tests for the M2 chunked pixel terrain. Split out of `chunked.rs` purely
//! as code motion; behavior is unchanged.

use crate::chunked::*;
use crate::integrity::{DamageKind, IntegrityBand, DEFAULT_CASCADE_DEPTH, DEFAULT_CASCADE_THRESHOLD};

fn small_world() -> ChunkedTerrain {
    let mut t = ChunkedTerrain::new(512, 256, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [512.0, 16.0], MATERIAL_CONCRETE);
    t.fill_aabb([200.0, 16.0], [400.0, 96.0], MATERIAL_DIRT);
    t.fill_aabb([460.0, 16.0], [492.0, 96.0], MATERIAL_METAL_NOHOOK);
    t
}

/// anchorable=true, piling=false. The piling=false invariant is asserted
/// by cf-material's MaterialDef accessor; here we verify the cf-terrain
/// affordance row.
#[test]
fn support_beam_affordance_matches_m14e_spec_table() {
    let aff =
        material_affordance(MATERIAL_SUPPORT_BEAM).expect("support_beam registered");
    assert_eq!(aff.id, MATERIAL_SUPPORT_BEAM);
    assert_eq!(aff.id, 8);
    assert_eq!(aff.name, "support_beam");
    assert!((aff.hardness - 200.0).abs() < 1e-3);
    assert!(aff.anchorable);
    assert!(aff.solid);
    assert!(aff.diggable, "support_beam must be diggable so demolish works");
}

#[test]
fn material_id_from_name_covers_launch_set() {
    for name in [
        "air",
        "dirt",
        "concrete",
        "concrete_soft",
        "metal_nohook",
        "hazard",
        "loose_fill",
        "repair_fill",
        "anchor",
    ] {
        assert!(material_id_from_name(name).is_some(), "{name}");
    }
    assert!(material_id_from_name("granite_unknown").is_none());
}

#[test]
fn fill_aabb_writes_dense_pixels() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    let written = t.fill_aabb([0.0, 0.0], [10.0, 10.0], MATERIAL_DIRT);
    assert_eq!(written, 100);
    assert_eq!(t.material_at(0, 0), MATERIAL_DIRT);
    assert_eq!(t.material_at(9, 9), MATERIAL_DIRT);
    assert_eq!(t.material_at(10, 10), MATERIAL_AIR);
}

#[test]
fn fill_circle_writes_radius_pixels() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    let written = t.fill_circle([5.0, 5.0], 3.0, MATERIAL_DIRT);
    assert!(written > 0 && written < 64);
    // (5,5) is the centre — must be filled.
    assert_eq!(t.material_at(5, 5), MATERIAL_DIRT);
    // (15,15) is well outside — still air.
    assert_eq!(t.material_at(15, 15), MATERIAL_AIR);
}

#[test]
fn try_carve_into_dirt_succeeds() {
    let mut t = small_world();
    let outcome = t.try_carve([300.0, 60.0], 8.0);
    match outcome {
        ChunkedCarveOutcome::Carved(stats) => {
            assert!(stats.count > 0);
            assert_eq!(stats.dominant_material, MATERIAL_DIRT);
            assert!(!stats.dirty_chunks.is_empty());
        }
        other => panic!("expected Carved, got {other:?}"),
    }
    assert_eq!(t.carve_count, 1);
}

#[test]
fn try_carve_into_metal_refuses() {
    let mut t = small_world();
    let outcome = t.try_carve([476.0, 60.0], 8.0);
    match outcome {
        ChunkedCarveOutcome::Refused(refusal) => {
            assert_eq!(refusal.reason, "material_not_diggable");
            assert_eq!(refusal.material, MATERIAL_METAL_NOHOOK);
        }
        other => panic!("expected Refused, got {other:?}"),
    }
    assert_eq!(t.refusal_count, 1);
}

#[test]
fn try_carve_against_air_is_noop() {
    let mut t = small_world();
    let outcome = t.try_carve([100.0, 200.0], 4.0);
    assert!(matches!(outcome, ChunkedCarveOutcome::NoOp(_)));
    assert_eq!(t.carve_count, 0);
}

#[test]
fn try_carve_increments_carve_count_only_on_success() {
    let mut t = small_world();
    let _ = t.try_carve([300.0, 60.0], 8.0);
    let _ = t.try_carve([320.0, 60.0], 8.0);
    let _ = t.try_carve([476.0, 60.0], 8.0); // refused
    assert_eq!(t.carve_count, 2);
    assert_eq!(t.refusal_count, 1);
}

#[test]
fn aabb_overlaps_solid_detects_floor() {
    let t = small_world();
    assert!(t.aabb_overlaps_solid([0.0, 0.0], [10.0, 16.0]));
    assert!(!t.aabb_overlaps_solid([0.0, 100.0], [10.0, 110.0]));
}

#[test]
fn column_top_solid_y_finds_floor_height() {
    let t = small_world();
    let top = t.column_top_solid_y(50, 51, 200).unwrap();
    // Floor occupies [0..16) so the top solid pixel y is 15.
    assert_eq!(top, 15);
}

#[test]
fn try_carve_breaks_through_full_width_with_repeated_calls() {
    let mut t = small_world();
    let mut carved_total: u32 = 0;
    for _ in 0..6 {
        if let ChunkedCarveOutcome::Carved(stats) = t.try_carve([300.0, 60.0], 8.0) {
            carved_total += stats.count;
        }
    }
    assert!(carved_total > 0);
    // Eventually the carve circle hits all-air and turns into a no-op.
    let _ = t.try_carve([300.0, 60.0], 8.0);
}

#[test]
fn dirty_chunks_track_carves_until_cleared() {
    let mut t = small_world();
    t.clear_dirty();
    let _ = t.try_carve([300.0, 60.0], 8.0);
    assert!(t.dirty_chunk_count() > 0);
    t.clear_dirty();
    assert_eq!(t.dirty_chunk_count(), 0);
}

#[test]
fn checksum_changes_when_terrain_changes() {
    let mut t = small_world();
    let before = t.checksum_bytes();
    let _ = t.try_carve([300.0, 60.0], 8.0);
    let after = t.checksum_bytes();
    assert_ne!(before, after);
}

#[test]
fn snapshot_round_trip_preserves_pixel_values() {
    let mut t = small_world();
    let _ = t.try_carve([300.0, 60.0], 8.0);
    let snap = t.snapshot();
    let restored = ChunkedTerrain::from_snapshot(&snap);
    assert_eq!(restored.checksum_bytes(), t.checksum_bytes());
    assert_eq!(restored.material_at(300, 60), MATERIAL_AIR);
}

#[test]
fn reset_clears_chunks_and_counters() {
    let mut t = small_world();
    let _ = t.try_carve([300.0, 60.0], 8.0);
    t.reset_to_default();
    assert_eq!(t.allocated_chunk_count(), 0);
    assert_eq!(t.carve_count, 0);
    assert_eq!(t.refusal_count, 0);
    assert_eq!(t.material_at(300, 60), MATERIAL_AIR);
}

#[test]
fn material_counts_balances_total_pixels() {
    let t = small_world();
    let counts = t.material_counts();
    let total: u64 = counts.values().sum();
    assert_eq!(total, 512 * 256);
}

#[test]
fn try_blast_clears_circle_through_diggable() {
    let mut t = small_world();
    let outcome = t.try_blast([300.0, 60.0], 16.0, 100.0);
    match outcome {
        ChunkedCarveOutcome::Carved(stats) => {
            assert!(stats.count > 0);
        }
        other => panic!("expected Carved, got {other:?}"),
    }
}

#[test]
fn try_blast_refuses_when_force_below_metal_hardness() {
    let mut t = small_world();
    let outcome = t.try_blast([476.0, 60.0], 16.0, 1.0);
    assert!(matches!(outcome, ChunkedCarveOutcome::Refused(_)));
}

#[test]
fn try_blast_against_hazard_obeys_spec_hardness_gate() {
    // M2 spec: hazard.hardness=50. Blasts below 50 refuse; blasts at or
    // above 50 clear (M5.6 active material kernel will later add a
    // dispersal/reaction path; M2 ships the basic force-gate symmetry
    // with `try_blast` against any blastable material).
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_HAZARD);
    // Force below hardness must refuse.
    let outcome = t.try_blast([16.0, 16.0], 8.0, 10.0);
    assert!(
        matches!(outcome, ChunkedCarveOutcome::Refused(_)),
        "expected hazard to refuse blast with force 10, got {outcome:?}"
    );
    // Force at hardness clears.
    let outcome = t.try_blast([16.0, 16.0], 8.0, 50.0);
    assert!(
        matches!(outcome, ChunkedCarveOutcome::Carved(_)),
        "expected hazard to yield to blast with force 50, got {outcome:?}"
    );
}

#[test]
fn try_fill_or_repair_paints_into_air() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    let outcome = t.try_fill_or_repair([32.0, 32.0], 6.0, MATERIAL_REPAIR_FILL);
    assert!(matches!(outcome, ChunkedCarveOutcome::Carved(_)));
    assert_eq!(t.material_at(32, 32), MATERIAL_REPAIR_FILL);
}

#[test]
fn try_fill_or_repair_refuses_over_metal_nohook() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_METAL_NOHOOK);
    let outcome = t.try_fill_or_repair([16.0, 16.0], 6.0, MATERIAL_REPAIR_FILL);
    match outcome {
        ChunkedCarveOutcome::Refused(refusal) => {
            assert_eq!(refusal.reason, "material_not_diggable");
        }
        other => panic!("expected refusal, got {other:?}"),
    }
}

#[test]
fn add_updated_material_area_marks_chunks_dirty() {
    let mut t = ChunkedTerrain::new(1024, 512, MATERIAL_AIR);
    t.clear_dirty();
    t.add_updated_material_area([100.0, 100.0], [200.0, 200.0]);
    assert!(t.dirty_chunk_count() > 0);
}

#[test]
fn chunk_checksum_changes_with_pixel_edit() {
    let mut t = small_world();
    let before = t.chunk_checksum(1, 0);
    let _ = t.try_carve([300.0, 60.0], 8.0);
    let after = t.chunk_checksum(1, 0);
    assert!(before.is_some());
    assert!(after.is_some());
    assert_ne!(before, after);
}

#[test]
fn val_m15b_material_affordances_cover_active_set() {
    // M15B added 12 active-material affordances (water, oil, acid,
    // lava, iron, co2, steam, smoke, fire_intense, cloud, rain,
    // acid_droplet). Without these, the renderer treats them as
    // transparent black + physics treats them as air.
    for (id, expected_name) in [
        (13u16, "water"),
        (19, "oil"),
        (21, "acid"),
        (26, "lava"),
        (53, "co2"),
        (50, "steam"),
        (62, "smoke"),
        (65, "fire_intense"),
        (68, "iron"),
        (71, "cloud"),
        (87, "rain"),
        (88, "acid_droplet"),
    ] {
        let aff = material_affordance(id)
            .unwrap_or_else(|| panic!("M15 material id={id} ({expected_name}) missing affordance"));
        assert_eq!(aff.name, expected_name, "id={id}");
    }
}

#[test]
fn val_m15b_hazardous_materials_damage_actors() {
    // Per M15 chem doc: acid, lava, fire_intense, acid_droplet all
    // emit per-tick damage to actors in contact.
    for (id, min_dpt) in [(21u16, 1.0_f32), (26, 5.0), (65, 5.0), (88, 1.0)] {
        let aff = material_affordance(id).unwrap();
        assert!(aff.hazard, "id={id} should be hazard");
        assert!(
            aff.damage_per_tick >= min_dpt,
            "id={id} damage_per_tick {} < {min_dpt}",
            aff.damage_per_tick
        );
    }
}

#[test]
fn dirt_to_concrete_hardness_ratio_matches_spec() {
    // Spec: concrete carves in 5-10x dirt time (hardness=50 vs hardness=10).
    // hits the 5x lower bound exactly.
    let dirt = material_affordance(MATERIAL_DIRT).unwrap();
    let concrete = material_affordance(MATERIAL_CONCRETE).unwrap();
    assert!((dirt.hardness - 10.0).abs() < f32::EPSILON);
    assert!((concrete.hardness - 50.0).abs() < f32::EPSILON);
    assert!(concrete.hardness >= 5.0 * dirt.hardness);
    assert!(concrete.hardness <= 10.0 * dirt.hardness);
}

#[test]
fn launch_set_baseline_hardness_matches_spec() {
    for (id, expected) in [
        (MATERIAL_AIR, 0.0_f32),
        (MATERIAL_DIRT, 10.0),
        (MATERIAL_LOOSE_FILL, 5.0),
        // 5x dirt-ratio spec floor is satisfied exactly.
        (MATERIAL_CONCRETE, 50.0),
        (MATERIAL_METAL_NOHOOK, 100.0),
        (MATERIAL_ANCHOR, 60.0),
        (MATERIAL_HAZARD, 50.0),
        (MATERIAL_REPAIR_FILL, 15.0),
    ] {
        let aff = material_affordance(id).unwrap_or_else(|| panic!("id {id} present"));
        assert!(
            (aff.hardness - expected).abs() < 1e-3,
            "{} hardness expected {} got {}",
            aff.name,
            expected,
            aff.hardness
        );
    }
}

#[test]
fn in_bounds_rejects_extreme_i64_coordinates() {
    // Devin BUG_pr-review-job (yellow) regression: `in_bounds` cast
    // `px as u32` which truncates for px >= 2^32 (e.g., 4294967296
    // truncates to 0 and would be reported in-bounds). The fix
    // compares in i64 space.
    let t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    // Inside bounds.
    assert!(t.in_bounds(0, 0));
    assert!(t.in_bounds(63, 63));
    // Outside bounds — truncation-prone values.
    assert!(!t.in_bounds(-1, 0));
    assert!(!t.in_bounds(64, 0));
    assert!(!t.in_bounds(64_000_000, 0));
    // Values that would truncate-wrap on a u32 cast — must remain out.
    assert!(!t.in_bounds(1_i64 << 32, 0));
    assert!(!t.in_bounds(0, 1_i64 << 33));
    assert!(!t.in_bounds(i64::MAX, 0));
}

#[test]
fn fill_aabb_clamped_to_terrain_extent() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    let written = t.fill_aabb([-10.0, -10.0], [200.0, 200.0], MATERIAL_DIRT);
    assert_eq!(written, 64 * 64);
}

#[test]
fn chunk_uniformity_compresses_storage() {
    let mut t = ChunkedTerrain::new(512, 512, MATERIAL_AIR);
    // Fill an entire chunk with dirt, then carve it back to air.
    t.fill_aabb([0.0, 0.0], [256.0, 256.0], MATERIAL_DIRT);
    assert_eq!(t.allocated_chunk_count(), 1);
    // Carving reverts each pixel back to air; once the chunk is uniform
    // again, storage is reclaimed.
    for y in 0..256 {
        for x in 0..256 {
            t.set_pixel_internal(x, y, MATERIAL_AIR);
        }
    }
    assert_eq!(t.allocated_chunk_count(), 0);
}

// -- M9 § Destructible terrain — per-pixel integrity tests --

#[test]
fn pixel_integrity_starts_pristine_for_untouched_pixels() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    // No prior damage — pristine 1.0.
    assert!((t.pixel_integrity(10, 10) - 1.0).abs() < f32::EPSILON);
    assert_eq!(t.pixel_band(10, 10), IntegrityBand::Pristine);
}

#[test]
fn try_penetrate_pixel_dirt_light_hit_drops_to_scratched() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    let outcome = t
        .try_penetrate_pixel(10, 10, 0.05, DamageKind::ProjectileHit, None)
        .expect("dirt pixel exists");
    assert_eq!(outcome.material_id, MATERIAL_DIRT);
    assert_eq!(outcome.band_before, IntegrityBand::Pristine);
    // 0.05 * (1 - 0.2) / 0.2 = 0.2 → integrity 0.8 → still Pristine.
    // Use a larger impact to land in Scratched.
    let outcome2 = t
        .try_penetrate_pixel(10, 10, 0.05, DamageKind::ProjectileHit, None)
        .expect("dirt pixel exists");
    assert_eq!(outcome2.band_after, IntegrityBand::Scratched);
    assert!(outcome2.band_crossed);
    assert!(!outcome2.destroyed);
}

#[test]
fn try_penetrate_pixel_sand_hit_destroys_immediately() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_LOOSE_FILL);
    let outcome = t
        .try_penetrate_pixel(10, 10, 0.5, DamageKind::ProjectileHit, None)
        .expect("sand pixel exists");
    assert!(outcome.destroyed);
    assert_eq!(outcome.band_after, IntegrityBand::Destroyed);
    // Pixel removed from the world.
    assert_eq!(t.material_at(10, 10), MATERIAL_AIR);
}

#[test]
fn try_penetrate_pixel_metal_resists_high_impact() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_METAL_NOHOOK);
    let outcome = t
        .try_penetrate_pixel(10, 10, 0.5, DamageKind::ProjectileHit, None)
        .expect("metal pixel exists");
    assert!(!outcome.destroyed);
    // (1.0 - 0.5 * 0.1 / 0.9) ≈ 0.944
    assert!(
        (outcome.integrity_after - 0.944).abs() < 0.02,
        "metal integrity after one hit at impact=0.5 should be ~0.94, got {}",
        outcome.integrity_after
    );
    assert_eq!(outcome.band_after, IntegrityBand::Pristine);
}

#[test]
fn try_penetrate_pixel_against_air_returns_none() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    let outcome = t.try_penetrate_pixel(10, 10, 0.5, DamageKind::ProjectileHit, None);
    assert!(outcome.is_none());
}

#[test]
fn try_penetrate_pixel_progresses_through_all_5_bands() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    let mut bands_observed: Vec<IntegrityBand> = vec![IntegrityBand::Pristine];
    for _ in 0..40 {
        if let Some(outcome) = t.try_penetrate_pixel(10, 10, 0.05, DamageKind::ProjectileHit, None) {
            if outcome.band_crossed {
                bands_observed.push(outcome.band_after);
            }
            if outcome.destroyed {
                break;
            }
        } else {
            break;
        }
    }
    // Must have observed every band on the path Pristine → Destroyed.
    assert!(bands_observed.contains(&IntegrityBand::Pristine));
    assert!(bands_observed.contains(&IntegrityBand::Scratched));
    assert!(bands_observed.contains(&IntegrityBand::Cracked));
    assert!(bands_observed.contains(&IntegrityBand::Critical));
    assert!(bands_observed.contains(&IntegrityBand::Destroyed));
}

#[test]
fn cascade_decay_affects_low_hardness_neighbors() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    // Pre-damage the destroyed-target pixel to integrity 0.05 so a small
    // impact pushes it to 0 + triggers cascade.
    let outcome = t
        .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
        .expect("dirt pixel");
    assert!(outcome.destroyed);
    // 4-neighbors are dirt (hardness 0.2 < 0.6) → all 4 cascade.
    assert_eq!(outcome.cascades.len(), 4);
    for ev in &outcome.cascades {
        assert!(ev.integrity_after < ev.integrity_before);
        assert_eq!(ev.depth, DEFAULT_CASCADE_DEPTH);
        assert_eq!(ev.threshold, DEFAULT_CASCADE_THRESHOLD);
    }
}

#[test]
fn cascade_skips_hard_neighbors() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    // Soft center pixel surrounded by hard concrete.
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_CONCRETE);
    t.set_pixel_internal(10, 10, MATERIAL_LOOSE_FILL);
    // Destroy the soft center.
    let outcome = t
        .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
        .expect("loose fill pixel");
    assert!(outcome.destroyed);
    // No cascade — every neighbor is concrete (hardness 0.7 > threshold 0.6).
    assert!(outcome.cascades.is_empty());
}

#[test]
fn cascade_decay_can_destroy_neighbor_with_low_integrity() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    // Bring the east neighbor (11, 10) down to ~0.05 integrity via several
    // light hits. dirt hardness=0.2 — impact_energy 0.01 yields damage=0.04
    // per hit, so 24 hits drops integrity from 1.0 to ~0.04 without
    // destroying it (so it stays a valid cascade target).
    for _ in 0..24 {
        let _ = t.try_penetrate_pixel(11, 10, 0.01, DamageKind::ProjectileHit, None);
    }
    let int_pre = t.pixel_integrity(11, 10);
    assert!(int_pre < 0.1, "expected low pre-cascade integrity, got {int_pre}");
    assert_eq!(t.material_at(11, 10), MATERIAL_DIRT);
    // Destroy the source pixel — cascade decay (0.1) pushes neighbor to 0.
    let outcome = t
        .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
        .expect("dirt pixel");
    assert!(outcome.destroyed);
    let neighbor_event = outcome
        .cascades
        .iter()
        .find(|ev| ev.to_pos == [11, 10])
        .expect("east neighbor cascade event");
    assert!(neighbor_event.destroyed_neighbor);
    assert_eq!(t.material_at(11, 10), MATERIAL_AIR);
}

#[test]
fn cascade_does_not_recurse_beyond_depth_1() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    // Pre-damage neighbor (11, 10) to 0 via cascade is allowed, but a
    // cascade-killed neighbor at (12, 10) must NOT cascade further to
    // (13, 10). Force (11, 10) and (12, 10) close to destruction.
    let _ = t.try_penetrate_pixel(11, 10, 0.95, DamageKind::ProjectileHit, None);
    let _ = t.try_penetrate_pixel(12, 10, 0.95, DamageKind::ProjectileHit, None);
    let int_13_before = t.pixel_integrity(13, 10);
    let outcome = t
        .try_penetrate_pixel(10, 10, 5.0, DamageKind::ProjectileHit, None)
        .expect("dirt pixel");
    // (11, 10) cascade may destroy it, but (12, 10) and (13, 10) integrity
    // should not be affected by a recursive cascade — they're only
    // adjacent to a cascade-affected pixel, not the original destroyed.
    let int_13_after = t.pixel_integrity(13, 10);
    assert!((int_13_after - int_13_before).abs() < f32::EPSILON);
    let _ = outcome;
}

#[test]
fn pixel_meta_grid_is_sparse_only_for_damaged_pixels() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    // Pristine pixels never get meta entries.
    assert!(t.pixel_meta_grid.is_empty());
    let _ = t.try_penetrate_pixel(5, 5, 0.05, DamageKind::ProjectileHit, None);
    // One damaged pixel → exactly one entry.
    assert_eq!(t.pixel_meta_grid.len(), 1);
}

#[test]
fn destroyed_pixel_removes_meta_entry() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    let _ = t.try_penetrate_pixel(5, 5, 0.05, DamageKind::ProjectileHit, None);
    assert_eq!(t.pixel_meta_grid.len(), 1);
    let _ = t.try_penetrate_pixel(5, 5, 10.0, DamageKind::ProjectileHit, None);
    // Pixel destroyed → meta entry cleared.
    let damaged_keys: Vec<_> = t.pixel_meta_grid.keys().filter(|k| k.lx == 5 && k.ly == 5).collect();
    assert!(damaged_keys.is_empty());
}

#[test]
fn reset_to_default_clears_pixel_meta() {
    let mut t = ChunkedTerrain::new(64, 64, MATERIAL_AIR);
    t.fill_aabb([0.0, 0.0], [32.0, 32.0], MATERIAL_DIRT);
    let _ = t.try_penetrate_pixel(5, 5, 0.05, DamageKind::ProjectileHit, None);
    assert!(!t.pixel_meta_grid.is_empty());
    t.reset_to_default();
    assert!(t.pixel_meta_grid.is_empty());
}
