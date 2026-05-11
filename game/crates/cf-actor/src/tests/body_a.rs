//! BODY-A acceptance test suite — M5 body damage model (DR-014).
//!
//! Pinned by `docs/plan/spec/feature-completion-checklist.md` row `M5-D05`
//! ("BODY-A and CHASSIS-A acceptance tests pass"). Every test starts with
//! `body_a_` so it can be selected with `cargo test -p cf-actor body_a`.
//!
//! These tests exercise the same production `ActorState` API that `cf-control`
//! / `cfctl` / `cf-app` route through. No `#[cfg(test)]` shortcuts that bypass
//! the chassis pipeline. BODY-A covers the body damage promises in DR-014
//! ("Body damage / armor layers / pilot rescue / damageable equipment / staged
//! body damage / parent-event cause chain") through the actor's chassis-routed
//! damage path and the public `body_silhouette()` / `chassis_view()` /
//! `ActorObservation` surfaces that `cfctl observe` exposes.

use crate::*;

fn make_actor_with_chassis(spec: cf_chassis::ChassisSpec) -> ActorState {
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
    let chassis = cf_chassis::ChassisState::from_spec(&spec, 60, false);
    actor.attach_chassis(chassis);
    actor
}

#[test]
fn body_a_status_state_machine_transitions_through_stable_unstable_downed_dead() {
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);

    assert_eq!(actor.status, Status::Stable);
    assert!(actor.status.accepts_input());
    assert_eq!(ActorObservation::from(&actor).status, "stable");

    // Stable → Unstable (HP <= 50% of 100).
    actor.apply_damage(45.0);
    assert_eq!(actor.status, Status::Stable);
    let change = actor.apply_damage(10.0);
    assert_eq!(change, Some(Status::Unstable));
    assert_eq!(actor.status, Status::Unstable);
    assert!(actor.status.accepts_input());
    assert_eq!(ActorObservation::from(&actor).status, "unstable");

    // Unstable → Downed (HP <= 10% of 100).
    let change = actor.apply_damage(40.0);
    assert_eq!(change, Some(Status::Downed));
    assert_eq!(actor.status, Status::Downed);
    assert!(!actor.status.accepts_input());
    assert_eq!(ActorObservation::from(&actor).status, "downed");

    // Downed → Dead.
    let change = actor.apply_damage(20.0);
    assert_eq!(change, Some(Status::Dead));
    assert_eq!(actor.status, Status::Dead);
    assert!(actor.status.is_dead());
    assert_eq!(ActorObservation::from(&actor).status, "dead");

    // Damage past Dead is a no-op AND status never re-derives.
    let no_change = actor.apply_damage(100.0);
    assert!(no_change.is_none());
    assert_eq!(actor.status, Status::Dead);
}

#[test]
fn body_a_layered_armor_breach_order_external_internal_core() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    // Powered Armor Head: External 30hp@hardness 6, Internal 18hp@hardness 3,
    // Core 24hp@hardness 0, Wound 12. First hit breaches only External.
    let (_, outcome1) = actor.apply_zone_damage(cf_chassis::BodyZone::Head, 36.0, "rifle_round");
    {
        let head = actor
            .chassis
            .as_ref()
            .unwrap()
            .zone(cf_chassis::BodyZone::Head)
            .unwrap();
        assert!(
            head.layers
                .iter()
                .find(|l| l.kind == cf_chassis::ArmorLayerKind::External)
                .unwrap()
                .is_breached(),
            "first hit must breach external"
        );
        assert!(
            !head
                .layers
                .iter()
                .find(|l| l.kind == cf_chassis::ArmorLayerKind::Internal)
                .unwrap()
                .is_breached(),
            "first hit must NOT breach internal yet"
        );
        assert!(
            !head
                .layers
                .iter()
                .find(|l| l.kind == cf_chassis::ArmorLayerKind::Core)
                .unwrap()
                .is_breached(),
            "first hit must NOT breach core yet"
        );
    }
    assert!(
        outcome1
            .layers_breached
            .iter()
            .any(|(k, _)| *k == cf_chassis::ArmorLayerKind::External),
        "outcome must record external breach"
    );

    // Second hit breaches Internal (already-damaged Internal has hp 15 left after first hit;
    // 25 dmg minus hardness 3 = 22 effective, breaches the 15 remaining).
    let (_, outcome2) = actor.apply_zone_damage(cf_chassis::BodyZone::Head, 25.0, "rifle_round");
    {
        let head = actor
            .chassis
            .as_ref()
            .unwrap()
            .zone(cf_chassis::BodyZone::Head)
            .unwrap();
        assert!(
            head.layers
                .iter()
                .find(|l| l.kind == cf_chassis::ArmorLayerKind::Internal)
                .unwrap()
                .is_breached(),
            "second hit must breach internal"
        );
        assert!(
            !head
                .layers
                .iter()
                .find(|l| l.kind == cf_chassis::ArmorLayerKind::Core)
                .unwrap()
                .is_breached(),
            "second hit must NOT breach core yet"
        );
    }
    assert!(
        outcome2
            .layers_breached
            .iter()
            .any(|(k, _)| *k == cf_chassis::ArmorLayerKind::Internal),
        "outcome must record internal breach"
    );

    // Third hit (overkill) breaches Core + drains wound.
    let (_, outcome3) = actor.apply_zone_damage(cf_chassis::BodyZone::Head, 200.0, "rifle_round");
    {
        let head = actor
            .chassis
            .as_ref()
            .unwrap()
            .zone(cf_chassis::BodyZone::Head)
            .unwrap();
        for layer in &head.layers {
            assert!(layer.is_breached(), "every layer must be breached after overkill");
        }
        assert!(head.destroyed);
    }
    assert!(outcome3.zone_destroyed);

    let view = actor.chassis_view().unwrap();
    let head_view = view.zones.iter().find(|z| z.zone == "head").unwrap();
    assert!(head_view.external_integrity <= 0.0);
    assert!(head_view.internal_integrity <= 0.0);
    assert!(head_view.core_integrity <= 0.0);
    assert!(head_view.destroyed);
}

#[test]
fn body_a_destroyed_arm_right_disables_rifle_via_movement_factor() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    let _ = actor.apply_zone_damage(cf_chassis::BodyZone::ArmRight, 2000.0, "blast");

    let chassis = actor.chassis.as_ref().unwrap();
    assert!(chassis.destroyed_zones().contains(&cf_chassis::BodyZone::ArmRight));

    let (_move, _jump, disable_rifle, _crawl, drop_gear, _jet) =
        chassis.body_graph.movement_factor(&chassis.destroyed_zones());
    assert!(disable_rifle, "destroyed right arm must disable rifle");
    assert!(drop_gear, "destroyed right arm must drop carried gear");

    let silhouette = actor.body_silhouette();
    assert!(!silhouette.placeholder);
    assert!(
        silhouette.arm_right_hp_pct <= 0.0001,
        "arm_right silhouette must reflect destruction; got {}",
        silhouette.arm_right_hp_pct
    );
    assert!(silhouette.arm_left_hp_pct >= 0.99, "arm_left must remain intact");

    let view = actor.chassis_view().unwrap();
    assert!(view.destroyed_zones.contains(&"arm_right".to_string()));
}

#[test]
fn body_a_destroyed_backpack_disables_jet_via_movement_factor() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    let _ = actor.apply_zone_damage(cf_chassis::BodyZone::Backpack, 2000.0, "rocket_hit");

    let chassis = actor.chassis.as_ref().unwrap();
    let (_move, _jump, _rifle, _crawl, _drop, disable_jet) =
        chassis.body_graph.movement_factor(&chassis.destroyed_zones());
    assert!(disable_jet, "destroyed backpack must disable jet via movement_factor");

    let view = actor.chassis_view().unwrap();
    let jet = view
        .modules
        .iter()
        .find(|m| m.kind == "jet")
        .expect("powered armor has jet module");
    assert_eq!(jet.state, "failed");
    assert_eq!(jet.last_reason, "bound_zone_destroyed");
    assert!(view.destroyed_zones.contains(&"backpack".to_string()));
}

#[test]
fn body_a_destroyed_leg_chain_halves_movement_speed_factor() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    let _ = actor.apply_zone_damage(cf_chassis::BodyZone::LegRight, 2000.0, "blast");

    let chassis = actor.chassis.as_ref().unwrap();
    let (move_factor, jump_factor, _rifle, _crawl, _drop, _jet) =
        chassis.body_graph.movement_factor(&chassis.destroyed_zones());
    assert!(
        move_factor <= 0.5,
        "destroyed right leg must reduce move factor to <= 0.5; got {move_factor}"
    );
    assert!(
        jump_factor <= 0.4,
        "destroyed right leg must reduce jump factor to <= 0.4; got {jump_factor}"
    );

    let silhouette = actor.body_silhouette();
    assert!(
        silhouette.leg_right_hp_pct <= 0.0001,
        "leg_right silhouette must reflect destruction"
    );
    assert!(silhouette.leg_left_hp_pct >= 0.99, "leg_left must remain intact");
}

#[test]
fn body_a_joint_severance_propagates_through_zone_destruction() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    for joint in &actor.chassis.as_ref().unwrap().body_graph.joints {
        let joint_id = &joint.id;
        assert!(joint.intact, "joint {joint_id} should start intact");
    }

    let (_, outcome) = actor.apply_zone_damage(cf_chassis::BodyZone::ArmRight, 2000.0, "blast");

    let chassis = actor.chassis.as_ref().unwrap();
    let shoulder_right = chassis
        .body_graph
        .joints
        .iter()
        .find(|j| j.id == "shoulder_right")
        .expect("shoulder_right joint exists");
    assert!(
        !shoulder_right.intact,
        "shoulder_right must be severed when arm_right destroyed"
    );

    let elbow_right = chassis
        .body_graph
        .joints
        .iter()
        .find(|j| j.id == "elbow_right")
        .expect("elbow_right joint exists");
    assert!(
        !elbow_right.intact,
        "elbow_right must be severed when arm_right destroyed (downstream chain link)"
    );

    let shoulder_left = chassis
        .body_graph
        .joints
        .iter()
        .find(|j| j.id == "shoulder_left")
        .expect("shoulder_left joint exists");
    assert!(shoulder_left.intact, "shoulder_left must remain intact");

    assert!(
        outcome.joints_severed.contains(&"shoulder_right".to_string()),
        "outcome must record shoulder_right severance for replay"
    );
    assert!(
        outcome.joints_severed.contains(&"elbow_right".to_string()),
        "outcome must record elbow_right severance for replay"
    );
}

#[test]
fn body_a_wound_container_drains_after_all_layers_breach_destroys_zone() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    let (_status, outcome) = actor.apply_zone_damage(cf_chassis::BodyZone::Torso, 2000.0, "explosive_blast");

    assert!(outcome.zone_destroyed);
    assert!(
        outcome.wound_damage > 0.0,
        "wound damage must drain after layers breach"
    );
    assert!(
        outcome.actor_hp_damage > 0.0,
        "overkill damage must spill to actor HP beyond the wound buffer"
    );

    let torso = actor
        .chassis
        .as_ref()
        .unwrap()
        .zone(cf_chassis::BodyZone::Torso)
        .unwrap();
    for layer in &torso.layers {
        let kind = layer.kind;
        assert!(layer.is_breached(), "{kind:?} layer must be breached");
    }
    assert_eq!(torso.wound_hp, 0.0);
    assert!(torso.destroyed);

    let silhouette = actor.body_silhouette();
    assert!(
        silhouette.torso_hp_pct <= 0.0001,
        "torso silhouette must reflect destruction; got {}",
        silhouette.torso_hp_pct
    );
    assert!(actor.hp < actor.hp_max, "actor HP must reflect wound spill damage");
}

#[test]
fn body_a_body_silhouette_reflects_per_zone_destruction() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    let pre = actor.body_silhouette();
    assert!(
        !pre.placeholder,
        "chassis-attached silhouette must not be a placeholder"
    );
    assert!((pre.head_hp_pct - 1.0).abs() < 0.05);
    assert!((pre.torso_hp_pct - 1.0).abs() < 0.05);
    assert!((pre.arm_right_hp_pct - 1.0).abs() < 0.05);

    let _ = actor.apply_zone_damage(cf_chassis::BodyZone::ArmRight, 2000.0, "rifle_round");

    let post = actor.body_silhouette();
    assert!(!post.placeholder);
    assert!(post.arm_right_hp_pct <= 0.0001);
    assert!(post.head_hp_pct >= 0.99, "head must stay intact");
    assert!(post.torso_hp_pct >= 0.99, "torso must stay intact");
    assert!(post.arm_left_hp_pct >= 0.99, "arm_left must stay intact");
    assert!(post.leg_right_hp_pct >= 0.99, "leg_right must stay intact");
    assert!(post.leg_left_hp_pct >= 0.99, "leg_left must stay intact");

    let obs = ActorObservation::from(&actor);
    assert!(!obs.body_silhouette.placeholder);
    assert!(obs.body_silhouette.arm_right_hp_pct <= 0.0001);
}

#[test]
fn body_a_chassis_view_exposes_destroyed_zones_list() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    // Use just-enough damage to destroy each zone without overkill spill that
    // would kill the actor (apply_zone_damage early-returns on Status::Dead).
    // PA LegLeft: External 40/h5 + Internal 24/h2 + Core 36 + Wound 16 → 125
    // raw dmg breaks the zone with ~9 spill HP.
    let _ = actor.apply_zone_damage(cf_chassis::BodyZone::LegLeft, 125.0, "blast");
    // PA Backpack: External 30/h4 + Internal 20/h2 + Core 18 + Wound 4 → 80
    // raw dmg breaks the zone with ~2 spill HP.
    let _ = actor.apply_zone_damage(cf_chassis::BodyZone::Backpack, 80.0, "blast");

    assert!(
        !actor.status.is_dead(),
        "actor must remain alive to expose chassis_view; hp={}",
        actor.hp
    );

    let view = actor.chassis_view().expect("chassis attached");
    assert!(view.destroyed_zones.contains(&"leg_left".to_string()));
    assert!(view.destroyed_zones.contains(&"backpack".to_string()));
    assert!(!view.destroyed_zones.contains(&"head".to_string()));
    assert!(!view.destroyed_zones.contains(&"torso".to_string()));
    assert_eq!(view.destroyed_zones.len(), 2);

    let obs = ActorObservation::from(&actor);
    let obs_chassis = obs.chassis.as_ref().expect("obs chassis present");
    assert!(obs_chassis.destroyed_zones.contains(&"leg_left".to_string()));
    assert!(obs_chassis.destroyed_zones.contains(&"backpack".to_string()));
}

#[test]
fn body_a_zone_damage_outcome_records_typed_cause_string() {
    let mut actor = make_actor_with_chassis(cf_chassis::powered_armor_spec());

    let cause = "autocannon_round_30mm";
    let (_, outcome1) = actor.apply_zone_damage(cf_chassis::BodyZone::Torso, 50.0, cause);
    assert_eq!(outcome1.cause, cause);
    assert_eq!(outcome1.zone, Some(cf_chassis::BodyZone::Torso));

    let (_, outcome2) = actor.apply_zone_damage(cf_chassis::BodyZone::Head, 10.0, "rifle_round");
    assert_eq!(outcome2.cause, "rifle_round");
    assert_eq!(outcome2.zone, Some(cf_chassis::BodyZone::Head));

    // Zero / NaN damage produces no outcome (no fake-success accept).
    let (_, empty) = actor.apply_zone_damage(cf_chassis::BodyZone::Torso, 0.0, "ignored");
    assert_eq!(empty.zone, None);
    assert_eq!(empty.cause, "");
    let (_, nan) = actor.apply_zone_damage(cf_chassis::BodyZone::Torso, f32::NAN, "ignored");
    assert_eq!(nan.zone, None);
}

#[test]
fn body_a_detach_chassis_resets_half_extents_to_infantry_baseline() {
    let mut actor = make_actor_with_chassis(cf_chassis::light_mech_spec());
    assert!(actor.half_extents.x > 10.0, "light mech is wider than infantry");
    assert!(actor.half_extents.y > 20.0, "light mech is taller than infantry");
    assert!(actor.chassis.is_some());
    assert!(!actor.chassis_detached);

    let detached = actor.detach_chassis().expect("chassis present");
    assert_eq!(detached.kind, cf_chassis::ChassisKind::LightMech);

    assert!((actor.half_extents.x - 8.0).abs() < f32::EPSILON);
    assert!((actor.half_extents.y - 16.0).abs() < f32::EPSILON);
    assert!(actor.chassis.is_none());
    assert!(actor.chassis_detached);
    assert!(!actor.jet_active, "detach must clear jet flag");
    assert!(!actor.climb_active, "detach must clear climb flag");

    assert!(actor.chassis_view().is_none(), "chassis_view returns None after detach");
    let silhouette = actor.body_silhouette();
    assert!(
        silhouette.placeholder,
        "post-detach silhouette falls back to flat-HP placeholder"
    );

    assert!(actor.detach_chassis().is_none(), "second detach is a no-op");
}

#[test]
fn body_a_zone_damage_routes_through_chassis_when_attached_otherwise_actor_hp() {
    // Case 1: chassis attached. Small hit must be absorbed by armor layers
    // (powered armor torso has External 80hp + 8 hardness — 20 dmg never reaches HP).
    let mut with_chassis = make_actor_with_chassis(cf_chassis::powered_armor_spec());
    let prev_hp = with_chassis.hp;
    let (status_change, outcome_with) = with_chassis.apply_zone_damage(cf_chassis::BodyZone::Torso, 20.0, "rifle");
    assert!(
        !outcome_with.layer_damage.is_empty(),
        "damage must route through chassis layers when chassis is attached"
    );
    assert_eq!(
        outcome_with.actor_hp_damage, 0.0,
        "small hit must NOT spill past wound container"
    );
    assert_eq!(with_chassis.hp, prev_hp, "actor HP unchanged when armor absorbs hit");
    assert!(status_change.is_none(), "status doesn't change without HP loss");

    // Case 2: no chassis. Damage routes directly to actor.hp at full magnitude.
    let inv = Inventory::with_rifle("rifle_m1_default");
    let mut no_chassis = ActorState::player(ActorId(2), "blue", Vec2::ZERO, 100.0, inv);
    assert!(no_chassis.chassis.is_none());
    let prev_hp_no_chassis = no_chassis.hp;
    let (_status, outcome_without) = no_chassis.apply_zone_damage(cf_chassis::BodyZone::Torso, 20.0, "rifle");

    assert!(
        outcome_without.layer_damage.is_empty(),
        "chassis-less actor must NOT produce layer damage"
    );
    assert!(
        outcome_without.glances.is_empty(),
        "chassis-less actor must NOT produce armor glances"
    );
    assert_eq!(
        outcome_without.actor_hp_damage, 20.0,
        "chassis-less actor must take the full damage on HP"
    );
    assert_eq!(outcome_without.zone, Some(cf_chassis::BodyZone::Torso));
    assert_eq!(outcome_without.cause, "rifle");
    assert!(no_chassis.hp < prev_hp_no_chassis);
    assert!((no_chassis.hp - (prev_hp_no_chassis - 20.0)).abs() < f32::EPSILON);
}
