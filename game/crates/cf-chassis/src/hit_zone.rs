//! **M13** § "Hit zone determination — how a 2D side-view projectile picks a
//! limb". Per-stance AABB tables that map (`local_x`, `local_y`) in normalized
//! actor-local space to a `BodyZone`. The lookup is fully deterministic — no
//! RNG, just AABB containment tests in spec-locked iteration order.

use crate::BodyZone;

/// Resolver result for a single hit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitZoneResolution {
    pub zone: BodyZone,
    pub local_x: f32,
    pub local_y: f32,
}

/// Stance discriminator used for AABB-table lookup. Mirrors the M6
/// `cf_actor::Stance` taxonomy for the four stances the spec tabulates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitZoneStance {
    Standing,
    Crouching,
    Prone,
    Crawl,
}

/// One (zone, x_range, y_range) entry from the M13 spec.
#[derive(Debug, Clone, Copy)]
struct ZoneAabb {
    zone: BodyZone,
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
}

impl ZoneAabb {
    const fn new(zone: BodyZone, x_min: f32, x_max: f32, y_min: f32, y_max: f32) -> Self {
        Self {
            zone,
            x_min,
            y_min,
            x_max,
            y_max,
        }
    }

    fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }
}

// **STANDING** zone AABB table (per M13 spec § "STANDING:"). Order matters
// — smaller / higher-priority zones come first so the first containment
// hit wins. Local space convention: `local_x` ∈ [-0.5, +0.5] (negative =
// near side after facing flip); `local_y` ∈ [0.0, 1.0] (0 = feet, 1 = head crown).
const STANDING_TABLE: &[ZoneAabb] = &[
    ZoneAabb::new(BodyZone::Head, -0.15, 0.15, 0.85, 1.00),
    ZoneAabb::new(BodyZone::Backpack, 0.15, 0.30, 0.55, 0.85),
    ZoneAabb::new(BodyZone::HandLeft, -0.40, -0.25, 0.30, 0.50),
    ZoneAabb::new(BodyZone::HandRight, 0.25, 0.40, 0.30, 0.50),
    ZoneAabb::new(BodyZone::ForearmLeft, -0.40, -0.20, 0.35, 0.55),
    ZoneAabb::new(BodyZone::ForearmRight, 0.20, 0.40, 0.35, 0.55),
    ZoneAabb::new(BodyZone::ArmLeft, -0.40, -0.15, 0.45, 0.80),
    ZoneAabb::new(BodyZone::ArmRight, 0.15, 0.40, 0.45, 0.80),
    ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.45, 0.85),
    ZoneAabb::new(BodyZone::ShinLeft, -0.18, 0.0, 0.08, 0.15),
    ZoneAabb::new(BodyZone::ShinRight, 0.0, 0.18, 0.08, 0.15),
    ZoneAabb::new(BodyZone::FootLeft, -0.18, 0.0, 0.0, 0.08),
    ZoneAabb::new(BodyZone::FootRight, 0.0, 0.18, 0.0, 0.08),
    ZoneAabb::new(BodyZone::LegLeft, -0.20, 0.0, 0.10, 0.45),
    ZoneAabb::new(BodyZone::LegRight, 0.0, 0.20, 0.10, 0.45),
];

const CROUCHING_TABLE: &[ZoneAabb] = &[
    ZoneAabb::new(BodyZone::Head, -0.15, 0.15, 0.70, 0.85),
    ZoneAabb::new(BodyZone::ArmRight, 0.15, 0.40, 0.50, 0.70),
    ZoneAabb::new(BodyZone::ArmLeft, -0.40, -0.15, 0.50, 0.70),
    ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.40, 0.70),
    ZoneAabb::new(BodyZone::FootLeft, -0.18, 0.0, 0.0, 0.10),
    ZoneAabb::new(BodyZone::FootRight, 0.0, 0.18, 0.0, 0.10),
    ZoneAabb::new(BodyZone::LegLeft, -0.20, 0.0, 0.10, 0.40),
    ZoneAabb::new(BodyZone::LegRight, 0.0, 0.20, 0.10, 0.40),
];

const PRONE_TABLE: &[ZoneAabb] = &[
    ZoneAabb::new(BodyZone::Head, -0.40, -0.30, 0.05, 0.25),
    ZoneAabb::new(BodyZone::Backpack, -0.20, 0.20, 0.20, 0.30),
    ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.05, 0.30),
    ZoneAabb::new(BodyZone::ArmLeft, -0.30, 0.0, 0.10, 0.25),
    ZoneAabb::new(BodyZone::ArmRight, 0.0, 0.30, 0.10, 0.25),
    ZoneAabb::new(BodyZone::FootLeft, 0.30, 0.40, 0.0, 0.10),
    ZoneAabb::new(BodyZone::FootRight, 0.30, 0.40, 0.0, 0.10),
    ZoneAabb::new(BodyZone::LegLeft, 0.10, 0.40, 0.05, 0.25),
    ZoneAabb::new(BodyZone::LegRight, 0.10, 0.40, 0.05, 0.25),
];

const CRAWL_TABLE: &[ZoneAabb] = &[
    ZoneAabb::new(BodyZone::Head, -0.40, -0.30, 0.05, 0.20),
    ZoneAabb::new(BodyZone::Torso, -0.30, 0.30, 0.05, 0.30),
    ZoneAabb::new(BodyZone::LegLeft, 0.10, 0.40, 0.05, 0.20),
    ZoneAabb::new(BodyZone::LegRight, 0.10, 0.40, 0.05, 0.20),
];

fn table_for(stance: HitZoneStance) -> &'static [ZoneAabb] {
    match stance {
        HitZoneStance::Standing => STANDING_TABLE,
        HitZoneStance::Crouching => CROUCHING_TABLE,
        HitZoneStance::Prone => PRONE_TABLE,
        HitZoneStance::Crawl => CRAWL_TABLE,
    }
}

/// Resolves the body zone at the given local-space coordinate.
/// `local_x` is post-facing-flip (positive = near side); `local_y` is
/// normalized 0..1 from feet to crown.
pub fn resolve(stance: HitZoneStance, local_x: f32, local_y: f32) -> Option<HitZoneResolution> {
    let table = table_for(stance);
    for entry in table {
        if entry.contains(local_x, local_y) {
            return Some(HitZoneResolution {
                zone: entry.zone,
                local_x,
                local_y,
            });
        }
    }
    None
}

/// Spec § "Per-stance hit probability distributions (designer reference)".
/// Tabulated expected distribution percentages — used by tests + AI
/// hint surfaces; NOT used at runtime to bias the resolver (which is
/// purely AABB-driven).
pub fn expected_distribution_standing_horizontal() -> [(BodyZone, f32); 5] {
    [
        (BodyZone::Head, 0.12),
        (BodyZone::Torso, 0.50),
        (BodyZone::ArmRight, 0.15),
        (BodyZone::LegRight, 0.20),
        (BodyZone::FootRight, 0.03),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standing_head_zone_resolves() {
        let r = resolve(HitZoneStance::Standing, 0.0, 0.92).unwrap();
        assert_eq!(r.zone, BodyZone::Head);
    }

    #[test]
    fn standing_torso_zone_resolves_at_mid_height() {
        let r = resolve(HitZoneStance::Standing, 0.0, 0.65).unwrap();
        assert_eq!(r.zone, BodyZone::Torso);
    }

    #[test]
    fn standing_leg_resolves_at_low_height() {
        let r = resolve(HitZoneStance::Standing, -0.10, 0.25).unwrap();
        assert_eq!(r.zone, BodyZone::LegLeft);
    }

    #[test]
    fn crouching_head_is_lower_than_standing() {
        let r = resolve(HitZoneStance::Crouching, 0.0, 0.80).unwrap();
        assert_eq!(r.zone, BodyZone::Head);
        // A shot at standing-head height (0.92) would miss the crouching actor.
        assert!(resolve(HitZoneStance::Crouching, 0.0, 0.95).is_none());
    }

    #[test]
    fn prone_head_is_at_facing_front() {
        let r = resolve(HitZoneStance::Prone, -0.35, 0.10).unwrap();
        assert_eq!(r.zone, BodyZone::Head);
    }

    #[test]
    fn crawl_table_has_minimal_silhouette() {
        // Crawl skips arms — those points return None.
        let r = resolve(HitZoneStance::Crawl, 0.0, 0.50);
        assert!(r.is_none(), "crawl has flat profile; mid-height arms gap");
    }
}
