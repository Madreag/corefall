//! CHASSIS-A acceptance test suite — M5 chassis grammar (DR-014 / DR-021).
//!
//! Pinned by `docs/plan/spec/feature-completion-checklist.md` row `M5-D05`
//! ("BODY-A and CHASSIS-A acceptance tests pass"). Every test starts with
//! `chassis_a_` so it can be selected with `cargo test -p cf-chassis chassis_a`.
//!
//! These tests exercise the same production `ChassisState` API that
//! `cf-control` / `cfctl` / `cf-app` route through; no `#[cfg(test)]` shortcuts
//! that bypass the live damage pipeline. The CHASSIS-A suite covers the chassis
//! grammar promises in DR-014 ("Mechs / Powered armor / Armor layers / Robots /
//! Damageable equipment / Staged machine damage / Pilot rescue / Repair
//! salvage") + DR-021 ("Mech scale ladder + module system").

use crate::*;

#[test]
fn chassis_a_powered_armor_eject_sequence_transitions_through_ejected() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);
    assert_eq!(state.pilot_state, PilotState::Bound);

    let accepted = state.attempt_eject(100).expect("eject must be accepted from Bound");
    assert!(!accepted.tutorial_extract, "non-tutorial chassis must run real eject");
    assert!(accepted.ticks_total > 0, "eject window must consume ticks");
    assert_eq!(state.pilot_state, PilotState::Ejecting);
    assert!(state.eject_window.ticks_remaining > 0);
    assert_eq!(state.eject_window.triggered_at_tick, 100);

    let ticks_total = state.eject_window.ticks_total;
    let mut last_progress = None;
    for _ in 0..ticks_total {
        last_progress = state.tick_eject();
    }
    assert_eq!(last_progress, Some(EjectProgress::Ejected));
    assert_eq!(state.pilot_state, PilotState::Ejected);
    assert_eq!(state.eject_window.ticks_remaining, 0);
    assert!(!state.pilot_state.is_in_chassis());
    assert!(!state.pilot_state.is_lost());
}

#[test]
fn chassis_a_light_mech_backpack_destruction_fails_jet_module() {
    let spec = light_mech_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);
    let jet_before = state.module_by_kind(ModuleKind::Jet).expect("light mech has jet");
    assert_eq!(jet_before.state, ModuleStateKind::Nominal);
    assert!(jet_before.hp > 0.0);

    let outcome = state.apply_zone_damage(BodyZone::Backpack, 2000.0, "autocannon_hit");
    assert!(outcome.zone_destroyed, "backpack zone must be destroyed by overkill");
    assert!(state.zone(BodyZone::Backpack).unwrap().destroyed);

    let jet_after = state.module_by_kind(ModuleKind::Jet).expect("jet still in roster");
    assert_eq!(jet_after.state, ModuleStateKind::Failed);
    assert_eq!(jet_after.last_reason, "bound_zone_destroyed");
    assert_eq!(jet_after.hp, 0.0);

    let transition = outcome
        .module_transitions
        .iter()
        .find(|t| t.id == jet_after.id)
        .expect("jet module transition recorded in outcome");
    assert_eq!(transition.state, ModuleStateKind::Failed);
    assert_eq!(transition.reason, "bound_zone_destroyed");
}

#[test]
fn chassis_a_salvage_returns_surviving_modules_with_non_empty_ids() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);

    let _ = state.apply_zone_damage(BodyZone::Torso, 2000.0, "blast");
    state.stage = ChassisStage::Wreck;

    let outcome = state
        .salvage("pilot_returns")
        .expect("wreck-stage chassis must accept salvage");

    assert!(
        !outcome.salvaged_module_ids.iter().any(|id| id.contains("shield")),
        "shield (bound to destroyed torso) must not be salvageable"
    );
    assert!(
        outcome.salvaged_module_ids.iter().any(|id| id.contains("sensor")),
        "sensor (bound to intact head) must be salvageable"
    );
    assert!(
        outcome.salvaged_module_ids.iter().any(|id| id.contains("jet")),
        "jet (bound to intact backpack) must be salvageable"
    );

    for id in &outcome.salvaged_module_ids {
        assert!(!id.is_empty(), "salvaged module id must not be empty");
    }

    assert_eq!(outcome.reason, "pilot_returns");
    assert!(!state.salvaged_modules.is_empty());
    for module in &state.salvaged_modules {
        assert!(module.last_reason.starts_with("salvaged:"));
    }
    assert_eq!(state.stage, ChassisStage::Wreck);
}

#[test]
fn chassis_a_module_state_ladder_progresses_nominal_to_failed() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);
    let jet_id = state
        .module_by_kind(ModuleKind::Jet)
        .expect("powered armor has jet")
        .id
        .clone();
    let jet_max_hp = state.module(&jet_id).unwrap().hp_max;
    assert!(jet_max_hp > 0.0);
    assert_eq!(state.module(&jet_id).unwrap().state, ModuleStateKind::Nominal);

    let t1 = state
        .apply_module_damage(&jet_id, jet_max_hp * 0.45, "stress")
        .expect("first transition out of Nominal");
    assert_eq!(t1.state, ModuleStateKind::Degraded);
    assert_eq!(state.module(&jet_id).unwrap().state, ModuleStateKind::Degraded);

    let t2 = state
        .apply_module_damage(&jet_id, jet_max_hp * 0.35, "stress")
        .expect("second transition into Warning");
    assert_eq!(t2.state, ModuleStateKind::Warning);
    assert_eq!(state.module(&jet_id).unwrap().state, ModuleStateKind::Warning);

    let t3 = state
        .apply_module_damage(&jet_id, jet_max_hp * 2.0, "stress")
        .expect("final transition into Failed");
    assert_eq!(t3.state, ModuleStateKind::Failed);
    assert_eq!(state.module(&jet_id).unwrap().state, ModuleStateKind::Failed);
    assert_eq!(state.module(&jet_id).unwrap().hp, 0.0);

    let extra = state.apply_module_damage(&jet_id, 1.0, "stress");
    assert!(extra.is_none(), "damage past Failed produces no further transitions");
}

#[test]
fn chassis_a_weapon_jam_and_clear_round_trip_updates_stage() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);
    assert!(!state.weapon_jammed);

    assert!(state.jam_weapon("debris_in_action"));
    assert!(state.weapon_jammed);
    assert!(state.last_stage_reason.contains("debris_in_action"));
    assert!(
        !state.jam_weapon("retry"),
        "jamming an already-jammed weapon must return false"
    );

    let _ = state.recompute_stage();
    let after_jam_stage = state.stage;
    assert!(
        after_jam_stage >= ChassisStage::WeaponJammed,
        "stage must advance to >= WeaponJammed; got {after_jam_stage:?}"
    );

    assert!(state.clear_jam());
    assert!(!state.weapon_jammed);
    assert!(!state.clear_jam(), "clearing an already-clear weapon must return false");
}

#[test]
fn chassis_a_repair_zone_restores_modules_and_steps_stage_back() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);

    let _ = state.apply_zone_damage(BodyZone::Backpack, 2000.0, "blast");
    let _ = state.recompute_stage();
    let stage_after_dmg = state.stage;
    assert!(
        stage_after_dmg >= ChassisStage::ModuleFailed,
        "damage must advance stage to >= ModuleFailed; got {stage_after_dmg:?}"
    );
    assert_eq!(
        state.module_by_kind(ModuleKind::Jet).unwrap().state,
        ModuleStateKind::Failed
    );
    assert!(state.zone(BodyZone::Backpack).unwrap().destroyed);

    let outcome = state
        .repair_zone(BodyZone::Backpack, "field_kit")
        .expect("backpack zone exists");
    assert!(outcome.was_destroyed);
    assert!(!outcome.modules_restored.is_empty(), "jet must be in restored list");
    assert_eq!(outcome.reason, "field_kit");

    let jet = state.module_by_kind(ModuleKind::Jet).unwrap();
    assert_eq!(jet.state, ModuleStateKind::Nominal);
    assert_eq!(jet.hp, jet.hp_max);
    assert!(jet.last_reason.starts_with("repaired_via:field_kit"));

    let backpack = state.zone(BodyZone::Backpack).unwrap();
    assert!(!backpack.destroyed);
    assert!(backpack.wound_hp > 0.0);
    assert!((backpack.external_integrity() - 1.0).abs() < 1e-3);

    let new_stage = state.stage;
    assert!(
        new_stage < stage_after_dmg,
        "repair must step stage back; before {stage_after_dmg:?} after {new_stage:?}"
    );

    let back_joint = state
        .body_graph
        .joints
        .iter()
        .find(|j| j.id == "back_mount")
        .expect("backpack joint exists");
    assert!(back_joint.intact, "repair_zone must reattach backpack joint");
}

#[test]
fn chassis_a_repair_module_restores_single_module_only() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);
    let jet_id = state.module_by_kind(ModuleKind::Jet).unwrap().id.clone();
    let shield_id = state.module_by_kind(ModuleKind::Shield).unwrap().id.clone();

    let _ = state.apply_module_damage(&jet_id, 1000.0, "overload");
    let _ = state.apply_module_damage(&shield_id, 1000.0, "overload");
    assert_eq!(state.module(&jet_id).unwrap().state, ModuleStateKind::Failed);
    assert_eq!(state.module(&shield_id).unwrap().state, ModuleStateKind::Failed);

    let transition = state.repair_module(&jet_id, "repair_drone").expect("module exists");
    assert_eq!(transition.id, jet_id);
    assert_eq!(transition.state, ModuleStateKind::Nominal);
    assert_eq!(transition.reason, "repaired:repair_drone");

    let jet_after = state.module(&jet_id).unwrap();
    assert_eq!(jet_after.state, ModuleStateKind::Nominal);
    assert_eq!(jet_after.hp, jet_after.hp_max);

    let shield_after = state.module(&shield_id).unwrap();
    assert_eq!(
        shield_after.state,
        ModuleStateKind::Failed,
        "shield must NOT be restored by jet repair"
    );

    let noop = state.repair_module(&jet_id, "repair_drone");
    assert!(
        noop.is_none(),
        "repairing an already-nominal module yields no transition"
    );
}

#[test]
fn chassis_a_tutorial_safety_caps_damage_at_pilot_injured() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, true);
    assert!(state.tutorial_safety);
    state.pilot_state = PilotState::Injured;

    let _ = state.apply_zone_damage(BodyZone::Torso, 5000.0, "tutorial_blast");
    let _ = state.recompute_stage();

    let capped_stage = state.stage;
    assert!(
        capped_stage <= ChassisStage::PilotInjured,
        "tutorial safety must cap stage at <= PilotInjured; got {capped_stage:?}"
    );
    assert!(
        !matches!(
            state.stage,
            ChassisStage::Wreck | ChassisStage::Gibbed | ChassisStage::Eject | ChassisStage::BailTooLate
        ),
        "tutorial chassis must never reach terminal stages from damage alone"
    );

    state.mark_gibbed("force_gibbed_during_tutorial");
    assert_ne!(
        state.stage,
        ChassisStage::Gibbed,
        "tutorial safety must reject mark_gibbed"
    );
}

#[test]
fn chassis_a_tutorial_safety_eject_yields_instant_extraction() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, true);
    assert!(state.tutorial_safety);
    assert_eq!(state.pilot_state, PilotState::Bound);

    let accepted = state.attempt_eject(10).expect("tutorial extract must accept");
    assert!(accepted.tutorial_extract);
    assert_eq!(accepted.ticks_total, 0);
    assert_eq!(state.pilot_state, PilotState::Extracted);
    assert_eq!(state.eject_window.ticks_remaining, 0);
    assert_eq!(state.eject_window.triggered_at_tick, 10);

    let again = state.attempt_eject(20);
    assert!(again.is_none(), "Extracted pilot can't re-eject");
}

#[test]
fn chassis_a_three_launch_archetypes_have_distinct_signatures() {
    let infantry = infantry_spec();
    let powered = powered_armor_spec();
    let mech = light_mech_spec();

    assert_eq!(infantry.kind, ChassisKind::Infantry);
    assert_eq!(powered.kind, ChassisKind::PoweredArmor);
    assert_eq!(mech.kind, ChassisKind::LightMech);

    assert!(
        infantry.mass_kg < powered.mass_kg,
        "infantry must mass less than powered armor"
    );
    assert!(
        powered.mass_kg < mech.mass_kg,
        "powered armor must mass less than light mech"
    );

    let infantry_present: Vec<_> = infantry.modules.iter().filter(|m| m.state.is_present()).collect();
    assert_eq!(infantry_present.len(), 1);
    assert_eq!(infantry_present[0].kind, ModuleKind::WeaponMount);

    let powered_kinds: Vec<_> = powered
        .modules
        .iter()
        .filter(|m| m.state.is_present())
        .map(|m| m.kind)
        .collect();
    assert!(powered_kinds.contains(&ModuleKind::WeaponMount));
    assert!(powered_kinds.contains(&ModuleKind::Jet));
    assert!(powered_kinds.contains(&ModuleKind::Shield));
    assert!(powered_kinds.contains(&ModuleKind::Sensor));
    assert!(
        !powered_kinds.contains(&ModuleKind::RepairDrone),
        "powered armor must not ship with repair drone (mech-only per DR-021)"
    );

    let mech_kinds: Vec<_> = mech
        .modules
        .iter()
        .filter(|m| m.state.is_present())
        .map(|m| m.kind)
        .collect();
    assert!(mech_kinds.contains(&ModuleKind::RepairDrone));

    assert_eq!(infantry.eject_window_seconds, 0.0);
    assert!(powered.eject_window_seconds > 0.0);
    assert!(mech.eject_window_seconds >= powered.eject_window_seconds);
}

#[test]
fn chassis_a_eject_after_wreck_yields_bailed_too_late() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);
    state.attempt_eject(0).expect("eject must accept while still Bound");
    assert_eq!(state.pilot_state, PilotState::Ejecting);

    state.stage = ChassisStage::Wreck;
    state.last_stage_reason = "torso_destroyed_before_eject_completed".to_string();

    let ticks_total = state.eject_window.ticks_total;
    let mut last = None;
    for _ in 0..ticks_total {
        last = state.tick_eject();
    }
    assert_eq!(last, Some(EjectProgress::BailedTooLate));
    assert_eq!(state.pilot_state, PilotState::BailedTooLate);
    assert!(state.pilot_state.is_lost());
}

#[test]
fn chassis_a_pilot_lifecycle_bound_through_extracted() {
    let spec = powered_armor_spec();
    let mut state = ChassisState::from_spec(&spec, 60, false);

    assert_eq!(state.pilot_state, PilotState::Bound);
    assert!(state.pilot_state.is_in_chassis());
    assert!(!state.pilot_state.is_lost());

    state.attempt_eject(50).expect("eject accepted from Bound");
    assert_eq!(state.pilot_state, PilotState::Ejecting);

    let ticks_total = state.eject_window.ticks_total;
    for _ in 0..ticks_total {
        let _ = state.tick_eject();
    }
    assert_eq!(state.pilot_state, PilotState::Ejected);
    assert!(!state.pilot_state.is_in_chassis());
    assert!(!state.pilot_state.is_lost());

    assert!(state.mark_pilot_extracted());
    assert_eq!(state.pilot_state, PilotState::Extracted);
    assert!(!state.pilot_state.is_in_chassis());
    assert!(!state.pilot_state.is_lost());

    assert!(
        !state.mark_pilot_extracted(),
        "mark_pilot_extracted is idempotent past Extracted"
    );
}
