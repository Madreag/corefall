//! M9B: cover-state-aware damage routing (M14 surface).
//!
//! Spec §"Player-facing behavior — Trench cross-section variants" /
//! "Cover state drives incoming-fire damage routing (M14)":
//!
//! > A `Full`-cover hit checks the parapet material first; only
//! > over-penetrating rounds reach the actor.
//! > `Partial` cover splits the body graph — head/shoulders exposed,
//! > torso/legs protected.
//!
//! VAL-M9B-DMGROUTE-001: an actor inside `standard` (Full cover when
//! crouched) takes a head-zone hit; the M14 damage pipeline first
//! applies the hit to the parapet pixel column; only residual energy
//! reaches the actor. When the same actor stands (Partial cover) and is
//! hit at the head zone, the hit bypasses the parapet (head/shoulders
//! exposed) while torso/legs hits remain absorbed by the parapet.
//!
//! This module is the pure decision function: given (cover_state,
//! body_zone), what is the damage route? The engine (cf-actor + M14
//! pipeline) consumes the result and feeds either:
//!   - the parapet pixel column first (subtract integrity, only deliver
//!     residual energy to the actor), or
//!   - the actor's body graph directly (no parapet interposition).

use crate::cover_state::CoverState;

/// One of the M14 body-graph zones the incoming-fire pipeline routes
/// damage through. Spec §"Cover state drives incoming-fire damage
/// routing" splits the body into "head/shoulders" (upper) and
/// "torso/legs" (lower) for Partial-cover routing.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DamageZone {
    /// Head — eyes / forehead / jaw.
    Head = 0,
    /// Shoulders — both shoulder joints + upper deltoid.
    Shoulders = 1,
    /// Torso — chest + abdomen.
    Torso = 2,
    /// Legs — thighs + knees + shins + feet.
    Legs = 3,
}

impl DamageZone {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DamageZone::Head => "head",
            DamageZone::Shoulders => "shoulders",
            DamageZone::Torso => "torso",
            DamageZone::Legs => "legs",
        }
    }

    /// Upper-body zones (head + shoulders) — exposed under Partial cover.
    #[must_use]
    pub const fn is_upper_body(self) -> bool {
        matches!(self, DamageZone::Head | DamageZone::Shoulders)
    }
}

/// The route the M14 pipeline takes for a single hit. Spec §"Cover
/// state drives incoming-fire damage routing":
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum DamageRoute {
    /// `Full` cover: route the hit through the parapet pixel column
    /// first. Only over-penetrating rounds reach the actor. The engine
    /// records `damage_route="parapet_first"`.
    ParapetFirst,
    /// `Partial` cover + upper-body zone OR `Exposed`: hit bypasses the
    /// parapet and goes directly to the actor's body graph. The engine
    /// records `damage_route="actor_direct"`.
    ActorDirect,
}

impl DamageRoute {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            DamageRoute::ParapetFirst => "parapet_first",
            DamageRoute::ActorDirect => "actor_direct",
        }
    }

    #[must_use]
    pub const fn parapet_first(self) -> bool {
        matches!(self, DamageRoute::ParapetFirst)
    }
}

/// Decide how to route an incoming hit given the actor's current cover
/// state and the body-graph zone the hit would land in absent cover.
///
/// Routing table per spec §"Cover state drives incoming-fire damage
/// routing":
///
/// | cover_state | head/shoulders         | torso/legs       |
/// |---|---|---|
/// | `Exposed`   | actor_direct           | actor_direct     |
/// | `Partial`   | actor_direct (exposed) | parapet_first    |
/// | `Full`      | parapet_first          | parapet_first    |
#[must_use]
pub fn damage_route_for(cover: CoverState, zone: DamageZone) -> DamageRoute {
    match (cover, zone.is_upper_body()) {
        (CoverState::Exposed, _) => DamageRoute::ActorDirect,
        (CoverState::Partial, true) => DamageRoute::ActorDirect,
        (CoverState::Partial, false) => DamageRoute::ParapetFirst,
        (CoverState::Full, _) => DamageRoute::ParapetFirst,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// VAL-M9B-DMGROUTE-001 (Full cover): a head-zone hit routes through
    /// the parapet first.
    #[test]
    fn full_cover_routes_head_hit_through_parapet() {
        assert_eq!(
            damage_route_for(CoverState::Full, DamageZone::Head),
            DamageRoute::ParapetFirst
        );
    }

    #[test]
    fn full_cover_routes_torso_hit_through_parapet() {
        assert_eq!(
            damage_route_for(CoverState::Full, DamageZone::Torso),
            DamageRoute::ParapetFirst
        );
    }

    /// VAL-M9B-DMGROUTE-001 (Partial cover): head/shoulders hit
    /// bypasses the parapet (exposed), torso/legs hit absorbed.
    #[test]
    fn partial_cover_routes_head_hit_direct_torso_through_parapet() {
        assert_eq!(
            damage_route_for(CoverState::Partial, DamageZone::Head),
            DamageRoute::ActorDirect
        );
        assert_eq!(
            damage_route_for(CoverState::Partial, DamageZone::Shoulders),
            DamageRoute::ActorDirect
        );
        assert_eq!(
            damage_route_for(CoverState::Partial, DamageZone::Torso),
            DamageRoute::ParapetFirst
        );
        assert_eq!(
            damage_route_for(CoverState::Partial, DamageZone::Legs),
            DamageRoute::ParapetFirst
        );
    }

    /// Exposed cover state: all hits route directly to actor.
    #[test]
    fn exposed_cover_routes_all_hits_direct() {
        for zone in [
            DamageZone::Head,
            DamageZone::Shoulders,
            DamageZone::Torso,
            DamageZone::Legs,
        ] {
            assert_eq!(
                damage_route_for(CoverState::Exposed, zone),
                DamageRoute::ActorDirect,
                "exposed must route {zone:?} directly"
            );
        }
    }

    /// VAL-M9B-DMGROUTE-001 alias name matching the project test name.
    #[test]
    fn cover_damage_routing_partial_full() {
        // Full + head -> parapet_first (must eat parapet)
        assert_eq!(
            damage_route_for(CoverState::Full, DamageZone::Head),
            DamageRoute::ParapetFirst
        );
        // Partial + head -> actor_direct (exposed)
        assert_eq!(
            damage_route_for(CoverState::Partial, DamageZone::Head),
            DamageRoute::ActorDirect
        );
        // Partial + legs -> parapet_first (lower body protected)
        assert_eq!(
            damage_route_for(CoverState::Partial, DamageZone::Legs),
            DamageRoute::ParapetFirst
        );
        // Exposed + head -> actor_direct
        assert_eq!(
            damage_route_for(CoverState::Exposed, DamageZone::Head),
            DamageRoute::ActorDirect
        );
    }

    #[test]
    fn route_as_str_round_trip() {
        assert_eq!(DamageRoute::ParapetFirst.as_str(), "parapet_first");
        assert_eq!(DamageRoute::ActorDirect.as_str(), "actor_direct");
        assert!(DamageRoute::ParapetFirst.parapet_first());
        assert!(!DamageRoute::ActorDirect.parapet_first());
    }

    #[test]
    fn damage_zone_upper_body_classification() {
        assert!(DamageZone::Head.is_upper_body());
        assert!(DamageZone::Shoulders.is_upper_body());
        assert!(!DamageZone::Torso.is_upper_body());
        assert!(!DamageZone::Legs.is_upper_body());
    }
}
