//! M9B / VAL-M9B-DMGROUTE-001: cover-state damage routing on cf-actor.
//!
//! These tests exercise the cf-trench `damage_routing` surface through
//! a cf-actor-owned helper so the spec evidence
//! `cargo test -p cf-actor cover_damage_routing_partial_full` resolves
//! to a passing test in this crate. Behaviorally these mirror the
//! cf-trench unit tests; the cf-actor smoke surface confirms the
//! cross-crate import path that the M14 damage pipeline (also living
//! in cf-actor) will consume.

use cf_trench::{damage_route_for, CoverState, DamageRoute, DamageZone};

/// VAL-M9B-DMGROUTE-001: Full cover routes head/torso hits through the
/// parapet first; Partial cover exposes head/shoulders but absorbs
/// torso/legs through the parapet.
#[test]
fn cover_damage_routing_partial_full() {
    // Full + head -> parapet_first
    assert_eq!(
        damage_route_for(CoverState::Full, DamageZone::Head),
        DamageRoute::ParapetFirst,
    );
    // Full + torso -> parapet_first
    assert_eq!(
        damage_route_for(CoverState::Full, DamageZone::Torso),
        DamageRoute::ParapetFirst,
    );
    // Partial + head -> actor_direct (exposed)
    assert_eq!(
        damage_route_for(CoverState::Partial, DamageZone::Head),
        DamageRoute::ActorDirect,
    );
    // Partial + shoulders -> actor_direct (exposed)
    assert_eq!(
        damage_route_for(CoverState::Partial, DamageZone::Shoulders),
        DamageRoute::ActorDirect,
    );
    // Partial + torso -> parapet_first (lower body absorbed)
    assert_eq!(
        damage_route_for(CoverState::Partial, DamageZone::Torso),
        DamageRoute::ParapetFirst,
    );
    // Partial + legs -> parapet_first
    assert_eq!(
        damage_route_for(CoverState::Partial, DamageZone::Legs),
        DamageRoute::ParapetFirst,
    );
    // Exposed + head -> actor_direct (no parapet at all)
    assert_eq!(
        damage_route_for(CoverState::Exposed, DamageZone::Head),
        DamageRoute::ActorDirect,
    );
    // Exposed + legs -> actor_direct
    assert_eq!(
        damage_route_for(CoverState::Exposed, DamageZone::Legs),
        DamageRoute::ActorDirect,
    );
}

#[test]
fn cover_damage_routing_full_cover_eats_parapet_first_for_all_zones() {
    for zone in [
        DamageZone::Head,
        DamageZone::Shoulders,
        DamageZone::Torso,
        DamageZone::Legs,
    ] {
        assert_eq!(
            damage_route_for(CoverState::Full, zone),
            DamageRoute::ParapetFirst,
            "full cover must absorb {zone:?} through the parapet"
        );
    }
}

#[test]
fn cover_damage_routing_partial_splits_head_exposed_legs_absorbed() {
    assert!(matches!(
        damage_route_for(CoverState::Partial, DamageZone::Head),
        DamageRoute::ActorDirect
    ));
    assert!(matches!(
        damage_route_for(CoverState::Partial, DamageZone::Legs),
        DamageRoute::ParapetFirst
    ));
}

#[test]
fn cover_damage_routing_exposed_routes_everything_direct() {
    for zone in [
        DamageZone::Head,
        DamageZone::Shoulders,
        DamageZone::Torso,
        DamageZone::Legs,
    ] {
        assert_eq!(
            damage_route_for(CoverState::Exposed, zone),
            DamageRoute::ActorDirect,
            "exposed cover must route {zone:?} directly"
        );
    }
}
