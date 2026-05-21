//! M5: chassis grammar, body graph, armor layers, modules, damage stages.
//!
//! This crate is the runtime contract that turns the M1 "single rifle + flat HP bar"
//! actor into the M5 contract from
//! `spec/chassis-armor-mechs-and-origins` + DR-014 + DR-021:
//!
//! - **Body graph** (`BodyGraph`): named limbs (head/torso/arms/legs/backpack),
//!   attachment joints between them, equipment sockets (`hand_right`, `back_mount`,
//!   etc.), wound containers per zone, armor-coverage map per zone, and per-limb
//!   movement-contribution fields (does losing this limb disable jet/jump/climb/aim/etc.).
//! - **Layered armor** (`ArmorLayer`): each zone has an external + internal + core
//!   layer with hp/max_hp/hardness/integrity. Damage strips layers in order; once
//!   the core is breached the zone becomes a wound container and routes damage to
//!   the actor's HP.
//! - **Modules** (`ChassisModule`): jet / shield / sensor / weapon_mount / repair_drone
//!   with `nominal → degraded → warning → failed` state machines bound to a body zone
//!   so module health follows the zone it sits on (failing a torso destroys the jet
//!   when bound there; surviving with one arm degrades weapon_mount).
//! - **Damage stages** (`ChassisStage`): the 11-stage pipeline from the roadmap M5
//!   done-criteria: nominal → degraded → module-warning → module-failed →
//!   weapon-jammed → armor-cracked → disabled → pilot-injured → eject → bail-too-late
//!   → wreck → gibbed. Transitions emit replay events with reason labels.
//! - **Pilot binding** (`PilotState`): pilot lives inside a chassis until it wrecks;
//!   `attempt_eject` moves to Ejecting then Ejected → Extracted; missing the eject
//!   window flips to BailedTooLate → Lost.
//! - **Tutorial-safety policy**: when `tutorial_safety = true` lethal damage caps
//!   at Disabled / PilotInjured (no Wreck / Gibbed), and `attempt_eject` cannot
//!   transition to BailedTooLate during the tutorial window.
//!
//! Reference chassis are provided as `powered_armor_default`, `light_mech_default`,
//! and `infantry_default`. Modders can clone them via `ChassisSpec::clone` and
//! mutate before insertion.
//!
//! Determinism contract: every public mutator is pure (state in → state out via
//! `&mut self`); no clock reads; no `rand::thread_rng()`. The engine seeds any RNG
//! it needs for jam-chance rolls and feeds it in explicitly.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::derivable_impls,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::too_many_lines,
    clippy::if_not_else,
    clippy::needless_pass_by_value,
    clippy::needless_continue
)]

mod ability;
mod armor;
mod body_graph;
mod body_zone;
mod drone;
pub mod hit_zone;
mod kind;
mod module;
mod outcomes;
mod spec;
mod specs;
mod stage;
mod state;
mod weapon_modifier;

pub use ability::*;
pub use armor::*;
pub use body_graph::*;
pub use body_zone::*;
pub use drone::*;
pub use kind::*;
pub use module::*;
pub use outcomes::*;
pub use spec::*;
pub use specs::*;
pub use stage::*;
pub use state::*;
pub use weapon_modifier::*;

#[cfg(test)]
mod tests {
    use super::specs::infantry::infantry_body_graph;
    use super::*;

    mod chassis_a;

    #[test]
    fn powered_armor_spec_has_canonical_zones_and_modules() {
        let s = powered_armor_spec();
        assert_eq!(s.kind, ChassisKind::PoweredArmor);
        // 15-zone full body graph: 7 base zones (head/torso/arms/legs/backpack)
        // + 8 granular limbs (forearms/hands + shins/feet pairs) per M5 spec.
        assert_eq!(s.zones.len(), 15);
        // Granular limbs verified:
        for zone in [
            BodyZone::ForearmRight,
            BodyZone::ForearmLeft,
            BodyZone::HandRight,
            BodyZone::HandLeft,
            BodyZone::ShinRight,
            BodyZone::ShinLeft,
            BodyZone::FootRight,
            BodyZone::FootLeft,
        ] {
            assert!(s.zones.iter().any(|z| z.zone == zone), "missing granular zone {zone:?}");
        }
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::WeaponMount));
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::Jet));
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::Shield));
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::Sensor));
    }

    #[test]
    fn destroying_hand_right_disables_rifle_via_movement_contribution() {
        let graph = infantry_body_graph();
        let (_m, _j, disable_rifle, _, drop_gear, _) = graph.movement_factor(&[BodyZone::HandRight]);
        assert!(disable_rifle, "destroyed right hand must disable rifle");
        assert!(drop_gear, "destroyed right hand must drop gear");
    }

    #[test]
    fn destroying_shin_left_reduces_movement_speed() {
        let graph = infantry_body_graph();
        let (move_factor, jump_factor, _, _, _, _) = graph.movement_factor(&[BodyZone::ShinLeft]);
        assert!(move_factor <= 0.5, "destroyed left shin must reduce move speed");
        assert!(jump_factor <= 0.4, "destroyed left shin must reduce jump");
    }

    #[test]
    fn parent_chain_resolves_correctly() {
        assert_eq!(BodyZone::HandRight.parent(), Some(BodyZone::ForearmRight));
        assert_eq!(BodyZone::ForearmRight.parent(), Some(BodyZone::ArmRight));
        assert_eq!(BodyZone::ArmRight.parent(), Some(BodyZone::Torso));
        assert_eq!(BodyZone::Torso.parent(), None);
        assert_eq!(BodyZone::FootLeft.parent(), Some(BodyZone::ShinLeft));
        assert!(BodyZone::HandLeft.is_left_arm_chain());
        assert!(BodyZone::FootRight.is_right_leg_chain());
    }

    #[test]
    fn light_mech_spec_has_repair_drone() {
        let s = light_mech_spec();
        assert!(s.modules.iter().any(|m| m.kind == ModuleKind::RepairDrone));
    }

    #[test]
    fn infantry_has_no_jet_module_present() {
        let s = infantry_spec();
        let jet = s.modules.iter().find(|m| m.kind == ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::NotPresent);
    }

    #[test]
    fn chassis_state_initializes_to_nominal() {
        let spec = powered_armor_spec();
        let state = ChassisState::from_spec(&spec, 60, false);
        assert_eq!(state.stage, ChassisStage::Nominal);
        assert_eq!(state.pilot_state, PilotState::Bound);
        assert!((state.integrity() - 1.0).abs() < 1e-3);
    }

    #[test]
    fn external_layer_glances_low_damage() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::Torso, 3.0, "projectile_hit");
        assert!(!outcome.glances.is_empty(), "expected hardness glance");
        assert!(outcome.layer_damage.is_empty(), "no layer should take damage");
    }

    #[test]
    fn damage_breaches_external_then_internal_then_core() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let total = 80.0 + 50.0 + 60.0 + 30.0 + 10.0; // overkill torso
        let _ = state.apply_zone_damage(BodyZone::Torso, total, "projectile_hit");
        let torso = state.zone(BodyZone::Torso).unwrap();
        assert!(torso.destroyed, "torso must be destroyed");
        for layer in &torso.layers {
            assert!(layer.is_breached(), "layer {} should be breached", layer.kind.as_str());
        }
    }

    #[test]
    fn destroying_backpack_fails_jet_module() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let total = 100.0 + 60.0 + 30.0; // overkill backpack
        let outcome = state.apply_zone_damage(BodyZone::Backpack, total, "projectile_hit");
        assert!(outcome.zone_destroyed);
        let jet = state.module_by_kind(ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::Failed);
    }

    #[test]
    fn stage_advances_to_armor_cracked_at_50_percent() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let _ = state.apply_zone_damage(BodyZone::Torso, 200.0, "projectile_hit");
        let _ = state.apply_zone_damage(BodyZone::ArmRight, 150.0, "projectile_hit");
        let _ = state.recompute_stage();
        assert!(state.stage >= ChassisStage::ArmorCracked);
    }

    #[test]
    fn attempt_eject_starts_window() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let accepted = state.attempt_eject(120).unwrap();
        assert!(!accepted.tutorial_extract);
        assert_eq!(state.pilot_state, PilotState::Ejecting);
        assert!(state.eject_window.ticks_remaining > 0);
    }

    #[test]
    fn tick_eject_completes_to_ejected() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        state.attempt_eject(0).unwrap();
        let ticks = state.eject_window.ticks_total;
        for _ in 0..ticks - 1 {
            let _ = state.tick_eject();
        }
        let final_progress = state.tick_eject();
        assert_eq!(final_progress, Some(EjectProgress::Ejected));
        assert_eq!(state.pilot_state, PilotState::Ejected);
    }

    #[test]
    fn bail_too_late_when_wreck_first() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        state.attempt_eject(0).unwrap();
        // Wreck the chassis mid-eject.
        let _ = state.apply_zone_damage(BodyZone::Torso, 1000.0, "projectile_hit");
        let _ = state.recompute_stage();
        // Force-set stage to Wreck (recompute may not catch it without all zones gone).
        state.stage = ChassisStage::Wreck;
        let ticks = state.eject_window.ticks_total;
        for _ in 0..ticks - 1 {
            let _ = state.tick_eject();
        }
        let final_progress = state.tick_eject();
        assert_eq!(final_progress, Some(EjectProgress::BailedTooLate));
        assert_eq!(state.pilot_state, PilotState::BailedTooLate);
    }

    #[test]
    fn tutorial_safety_caps_at_pilot_injured() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, true);
        state.pilot_state = PilotState::Injured;
        let _ = state.apply_zone_damage(BodyZone::Torso, 1000.0, "projectile_hit");
        let _ = state.recompute_stage();
        assert!(
            state.stage <= ChassisStage::PilotInjured,
            "tutorial safety must cap stage; got {:?}",
            state.stage
        );
    }

    #[test]
    fn tutorial_safety_blocks_eject_to_lost() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, true);
        let outcome = state.attempt_eject(10).unwrap();
        assert!(outcome.tutorial_extract);
        assert_eq!(state.pilot_state, PilotState::Extracted);
    }

    #[test]
    fn repair_zone_restores_modules() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let _ = state.apply_zone_damage(BodyZone::Backpack, 500.0, "projectile_hit");
        let outcome = state.repair_zone(BodyZone::Backpack, "field_kit").unwrap();
        assert!(outcome.was_destroyed);
        assert!(!outcome.modules_restored.is_empty());
        let jet = state.module_by_kind(ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::Nominal);
    }

    #[test]
    fn salvage_pulls_surviving_modules() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        // Wreck the torso so the chassis is salvageable.
        let _ = state.apply_zone_damage(BodyZone::Torso, 1000.0, "projectile_hit");
        let _ = state.recompute_stage();
        state.stage = ChassisStage::Wreck;
        let outcome = state.salvage("ejected_pilot_returns").unwrap();
        // Shield was bound to torso — should NOT be salvaged.
        assert!(!outcome.salvaged_module_ids.iter().any(|id| id.starts_with("shield")));
        // Sensor (head) + jet (backpack) survive.
        assert!(!outcome.salvaged_module_ids.is_empty());
        assert!(!state.salvaged_modules.is_empty());
    }

    #[test]
    fn weapon_jam_and_clear_round_trip() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        assert!(state.jam_weapon("debris_in_action"));
        assert!(state.weapon_jammed);
        let _ = state.recompute_stage();
        assert!(state.stage >= ChassisStage::WeaponJammed);
        assert!(state.clear_jam());
        assert!(!state.weapon_jammed);
    }

    #[test]
    fn movement_factor_reflects_destroyed_zones() {
        let graph = infantry_body_graph();
        let (m, j, dr, fc, dg, dj) = graph.movement_factor(&[BodyZone::LegRight, BodyZone::Backpack]);
        assert!(m <= 0.5);
        assert!(j <= 0.4);
        assert!(!dr);
        assert!(!fc);
        assert!(!dg);
        assert!(dj, "destroyed backpack must disable jet");
    }

    #[test]
    fn checksum_layout_is_stable() {
        let spec = powered_armor_spec();
        let s1 = ChassisState::from_spec(&spec, 60, false);
        let s2 = ChassisState::from_spec(&spec, 60, false);
        assert_eq!(s1.checksum_bytes(), s2.checksum_bytes());
    }

    #[test]
    fn checksum_distinguishes_zone_damage() {
        let spec = powered_armor_spec();
        let mut a = ChassisState::from_spec(&spec, 60, false);
        let mut b = ChassisState::from_spec(&spec, 60, false);
        let _ = a.apply_zone_damage(BodyZone::Torso, 50.0, "hit");
        let _ = b.apply_zone_damage(BodyZone::LegLeft, 50.0, "hit");
        assert_ne!(a.checksum_bytes(), b.checksum_bytes());
    }

    #[test]
    fn registry_resolves_canonical_ids() {
        assert!(chassis_spec(POWERED_ARMOR_ID).is_some());
        assert!(chassis_spec(LIGHT_MECH_ID).is_some());
        assert!(chassis_spec(INFANTRY_ID).is_some());
        assert!(chassis_spec("nonexistent").is_none());
    }

    /// **M13** § "Chassis archetypes — M13 ships 5": crab + drone archetypes
    /// must be in the canonical registry alongside the 3 humanoid kinds.
    #[test]
    fn registry_ships_six_chassis_archetypes() {
        assert!(chassis_spec(CRAB_QUADRUPED_ID).is_some());
        assert!(chassis_spec(DRONE_ID).is_some());
        // **M14A** ships the 6th archetype — Heavy Trooper.
        assert!(chassis_spec(HEAVY_TROOPER_ID).is_some());
        assert_eq!(chassis_specs().len(), 6, "M14A ships 6 chassis archetypes");
    }

    /// **M14A** § "Heavy Armor" — heavy trooper spec contract.
    #[test]
    fn heavy_trooper_spec_has_tank_grade_zones() {
        let s = heavy_trooper_spec();
        assert_eq!(s.kind, ChassisKind::HeavyTrooper);
        assert!((s.mass_kg - 380.0).abs() < 1e-3);
        // Torso External ≥ 400 HP at hardness ≥ 22.
        let torso = s.zones.iter().find(|z| z.zone == BodyZone::Torso).unwrap();
        let torso_ext = torso.layers.iter().find(|l| l.kind == ArmorLayerKind::External).unwrap();
        assert!(torso_ext.hp >= 400.0);
        assert!(torso_ext.hardness >= 22.0);
        // Per-zone tunings: torso dmg_multiplier=0.6, stagger=0.2, gib≥3200.
        assert!((torso.damage_multiplier - 0.6).abs() < 1e-6);
        assert!((torso.stagger_factor - 0.2).abs() < 1e-6);
        assert!(torso.gib_impulse_limit >= 3200.0);
    }

    /// **M13** § "Quadruped=11 zones": crab body graph zone count contract.
    #[test]
    fn crab_quadruped_has_eleven_zones() {
        let s = crab_quadruped_spec();
        assert_eq!(s.kind, ChassisKind::CrabQuadruped);
        assert_eq!(s.zones.len(), 11);
        for zone in [
            BodyZone::Carapace,
            BodyZone::SensorCluster,
            BodyZone::LegFrontLeft,
            BodyZone::LegFrontRight,
            BodyZone::LegRearLeft,
            BodyZone::LegRearRight,
            BodyZone::ClawLeft,
            BodyZone::ClawRight,
        ] {
            assert!(s.zones.iter().any(|z| z.zone == zone), "missing crab zone {zone:?}");
        }
        // No jet on crab.
        let jet = s.modules.iter().find(|m| m.kind == ModuleKind::Jet).unwrap();
        assert_eq!(jet.state, ModuleStateKind::NotPresent);
    }

    /// **M13** § "Drone=4 zones": drone body graph zone count contract.
    #[test]
    fn drone_has_four_zones() {
        let s = drone_spec();
        assert_eq!(s.kind, ChassisKind::Drone);
        assert_eq!(s.zones.len(), 4);
        for zone in [
            BodyZone::DroneCore,
            BodyZone::DroneArmLeft,
            BodyZone::DroneArmRight,
            BodyZone::DroneSensorPod,
        ] {
            assert!(s.zones.iter().any(|z| z.zone == zone), "missing drone zone {zone:?}");
        }
    }

    /// **M13** § "Armor mounting angles per chassis archetype": per-chassis
    /// angles match the spec table.
    #[test]
    fn armor_mount_angles_match_spec() {
        assert_eq!(infantry_spec().armor_angles, ArmorMountAngles::new(0.0, 0.0, 0.0));
        assert_eq!(powered_armor_spec().armor_angles, ArmorMountAngles::new(15.0, 0.0, 15.0));
        assert_eq!(light_mech_spec().armor_angles, ArmorMountAngles::new(30.0, 0.0, 15.0));
        assert_eq!(
            crab_quadruped_spec().armor_angles,
            ArmorMountAngles::new(30.0, 30.0, 30.0)
        );
        assert_eq!(drone_spec().armor_angles, ArmorMountAngles::new(0.0, 0.0, 0.0));
    }

    /// **M13** § "Chassis ability slots": per-chassis slot count + activate.
    #[test]
    fn chassis_ability_slots_count_per_kind() {
        assert_eq!(ChassisKind::Infantry.ability_slot_count(), 1);
        assert_eq!(ChassisKind::PoweredArmor.ability_slot_count(), 2);
        assert_eq!(ChassisKind::LightMech.ability_slot_count(), 3);
        assert_eq!(ChassisKind::Drone.ability_slot_count(), 1);
    }

    #[test]
    fn chassis_ability_activate_advances_and_cools_down() {
        let mut state = ChassisState::from_spec(&powered_armor_spec(), 60, false);
        let result = state.activate_ability(ChassisAbility::Overdrive);
        assert!(result.is_ok());
        // Activating again while active fails.
        let err = state.activate_ability(ChassisAbility::Overdrive).unwrap_err();
        assert_eq!(err, AbilityRejectReason::AlreadyActive);
        // Tick out the effect — engine should drain effect ticks.
        for _ in 0..state.abilities.find(ChassisAbility::Overdrive).unwrap().effect_total_ticks {
            state.tick_abilities();
        }
        // Now cooling down.
        let err = state.activate_ability(ChassisAbility::Overdrive).unwrap_err();
        assert_eq!(err, AbilityRejectReason::OnCooldown);
    }

    /// **M13** § "Weapon modifier slots": attach/detach respects max slot count.
    #[test]
    fn weapon_modifier_slot_count_per_chassis_tier() {
        let mut set = WeaponModifierSet::new(ChassisKind::Infantry);
        assert_eq!(set.max_slots, 1);
        set.attach(WeaponModifier::Homing).unwrap();
        assert!(set.attach(WeaponModifier::Explosive).is_err());
        let mut mech = WeaponModifierSet::new(ChassisKind::LightMech);
        assert_eq!(mech.max_slots, 3);
        mech.attach(WeaponModifier::Homing).unwrap();
        mech.attach(WeaponModifier::Explosive).unwrap();
        mech.attach(WeaponModifier::Freezing).unwrap();
        assert!(mech.is_combined());
        assert!(mech.attach(WeaponModifier::ChainLightning).is_err());
    }

    /// **M13** § "30+ launch modifiers": registry has at least 30 modifiers.
    #[test]
    fn weapon_modifier_registry_has_thirty_plus() {
        assert!(WeaponModifier::all().len() >= 30);
        assert_eq!(WeaponModifier::parse("homing"), Some(WeaponModifier::Homing));
        assert_eq!(WeaponModifier::parse("nonsense"), None);
    }

    /// **M13** § "Drone allies — 4 modes": parse + fuel drain.
    #[test]
    fn drone_modes_round_trip_and_drain_fuel() {
        assert_eq!(DroneMode::parse("auto_mine"), Some(DroneMode::AutoMine));
        assert_eq!(DroneMode::parse("auto-carry"), Some(DroneMode::AutoCarry));
        let mut drone = DroneAllyState::default();
        assert!((drone.fuel - 1.0).abs() < 1e-6);
        for _ in 0..18000 {
            drone.tick_fuel(60);
        }
        assert!(drone.fuel < 1.0, "5 minutes of fuel drain should reduce charge");
    }

    /// **M13** § "Cockpit camera anchor — Medium + Heavy classes only".
    #[test]
    fn cockpit_anchor_rejects_unsupported_chassis() {
        let mut infantry = ChassisState::from_spec(&infantry_spec(), 60, false);
        assert!(infantry.set_camera_anchor(CameraAnchor::Cockpit).is_err());
        let mut mech = ChassisState::from_spec(&light_mech_spec(), 60, false);
        assert!(mech.set_camera_anchor(CameraAnchor::Cockpit).is_ok());
        assert_eq!(mech.camera_anchor, CameraAnchor::Cockpit);
    }

    /// **M13** § "Boarding / disembarking transitions": 1500ms transitions
    /// are tick-rate-stable.
    #[test]
    fn boarding_transitions_match_1500ms_at_any_tick_rate() {
        let mut at_60 = ChassisState::from_spec(&powered_armor_spec(), 60, false);
        let at_120 = ChassisState::from_spec(&powered_armor_spec(), 120, false);
        assert_eq!(at_60.transition_ticks_total, 90); // 1.5s * 60Hz
        assert_eq!(at_120.transition_ticks_total, 180); // 1.5s * 120Hz
        assert!(at_60.begin_boarding());
        assert!(at_60.is_in_transition());
        // Cannot start a second transition while one is in flight.
        assert!(!at_60.begin_disembarking());
        // Tick until completion.
        let mut completed = None;
        for _ in 0..at_60.transition_ticks_total {
            completed = at_60.tick_transitions();
        }
        assert_eq!(completed, Some(TransitionCompleted::Boarded));
        assert!(!at_60.is_in_transition());
        let _ = at_120;
    }

    /// **M13** § "Hit reactions per body part": tabulated reactions per zone.
    #[test]
    fn hit_reactions_match_spec_table() {
        let head = HitReaction::for_zone(BodyZone::Head);
        assert_eq!(head.kind, "stagger_stun");
        assert!((head.duration_seconds - 0.5).abs() < 1e-6);
        assert_eq!(head.concussion_dose, 15);
        let hand = HitReaction::for_zone(BodyZone::HandRight);
        assert_eq!(hand.kind, "drop_weapon");
        assert!((hand.drop_chance - 0.40).abs() < 1e-6);
        let leg = HitReaction::for_zone(BodyZone::LegLeft);
        assert!((leg.speed_factor - 0.7).abs() < 1e-6);
        let head_ticks = head.duration_ticks(60);
        assert_eq!(head_ticks, 30);
    }

    /// **M13** § "Critical chassis modules with full mechanics": ammo rack
    /// cooking + detonation cascade.
    #[test]
    fn ammo_rack_cascade_cooks_then_detonates() {
        let mut state = ChassisState::from_spec(&light_mech_spec(), 60, false);
        // Apply damage to drive AmmoRack toward Warning then Failed.
        let warn = state.apply_critical_module_damage("ammo_rack.main", 50.0, "shaped_charge").unwrap();
        assert!(matches!(
            warn.cascade_events.first(),
            Some(CriticalModuleEvent::AmmoCooking { .. })
        ));
        // Now finish off the rack.
        let finish = state.apply_critical_module_damage("ammo_rack.main", 50.0, "second_hit").unwrap();
        assert!(finish
            .cascade_events
            .iter()
            .any(|e| matches!(e, CriticalModuleEvent::AmmoDetonated { .. })));
        assert_eq!(state.stage, ChassisStage::Gibbed);
    }

    /// **M13** § "Spalling integration": deterministic fragment routing.
    #[test]
    fn spalling_fragments_are_deterministic_given_same_seed() {
        let mut a = ChassisState::from_spec(&light_mech_spec(), 60, false);
        let mut b = ChassisState::from_spec(&light_mech_spec(), 60, false);
        let frags_a = a.spawn_spalling_fragments((0.0, 0.0), 3, 30.0, 42);
        let frags_b = b.spawn_spalling_fragments((0.0, 0.0), 3, 30.0, 42);
        assert_eq!(frags_a.len(), 3);
        assert_eq!(frags_a.len(), frags_b.len());
        for (fa, fb) in frags_a.iter().zip(frags_b.iter()) {
            assert_eq!(fa.module_id, fb.module_id);
            assert!((fa.damage - fb.damage).abs() < 1e-3);
        }
    }

    /// **M13** § "Limb loss functional consequences" — head destruction
    /// flags `lethal=true` (instant death per CCCP decapitation rule).
    #[test]
    fn head_destruction_flags_lethal_when_not_tutorial_safe() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::Head, 1000.0, "headshot");
        assert!(outcome.zone_destroyed, "head zone must be destroyed by 1000 dmg");
        assert!(outcome.lethal, "head destruction must flag lethal=true");
    }

    /// **M13** § "Tutorial-safety scenario variant" — head destruction does
    /// NOT flag lethal when tutorial_safety=true.
    #[test]
    fn head_destruction_skips_lethal_in_tutorial_safety() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, true);
        let outcome = state.apply_zone_damage(BodyZone::Head, 1000.0, "headshot");
        assert!(outcome.zone_destroyed);
        assert!(!outcome.lethal, "tutorial_safety must suppress lethal");
    }

    /// **M13** § "Torso loss = INSTANT DEATH": torso destruction flags lethal.
    #[test]
    fn torso_destruction_flags_lethal() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::Torso, 2000.0, "shaped_charge");
        assert!(outcome.zone_destroyed);
        assert!(outcome.lethal);
    }

    /// **M13** § "Arm loss" — destroying an arm does NOT flag lethal (only
    /// head/torso do).
    #[test]
    fn arm_destruction_does_not_flag_lethal() {
        let spec = powered_armor_spec();
        let mut state = ChassisState::from_spec(&spec, 60, false);
        let outcome = state.apply_zone_damage(BodyZone::ArmLeft, 1000.0, "explosion");
        assert!(outcome.zone_destroyed);
        assert!(!outcome.lethal, "arm destruction must NOT flag lethal");
    }

    /// **M13** § "Engineer auto-repair contract" — per-module repair cost
    /// table matches the spec values.
    #[test]
    fn engineer_auto_repair_table_matches_spec() {
        let jet = ModuleRepairCost::for_module(ModuleKind::Jet);
        assert!((jet.seconds_per_hp - 0.6).abs() < 1e-6);
        assert_eq!(jet.engineer_priority, 8);
        let core = ModuleRepairCost::for_module(ModuleKind::PowerCore);
        assert_eq!(core.engineer_priority, 9);
        let ext = ModuleRepairCost::for_armor_layer(ArmorLayerKind::External);
        assert!((ext.seconds_per_hp - 0.3).abs() < 1e-6);
        assert_eq!(ext.engineer_priority, 9);
    }

    // ------------------------------------------------------------------
    // M14C — ERA (Explosive Reactive Armor) module
    // ------------------------------------------------------------------

    /// **VAL-M14C-002**: `ModuleKind::Era` variant exists in `cf-chassis`
    /// with one-shot consumable behavior.
    #[test]
    fn era_module_kind_constructs_and_serializes() {
        let m = ChassisModule::new("era.front", ModuleKind::Era, BodyZone::Torso, 30.0);
        assert_eq!(m.kind, ModuleKind::Era);
        assert_eq!(ModuleKind::Era.as_str(), "era");
        assert!(m.era_consumable);
        assert!(m.era_charge_kg > 0.0);
    }

    /// **VAL-M14C-002**: ERA panel is consumable — first HEAT impact
    /// returns Some(charge); a second impact returns None (already spent).
    #[test]
    fn era_module_one_shot_consumable() {
        let mut m = ChassisModule::new("era.front", ModuleKind::Era, BodyZone::Torso, 30.0)
            .with_era(1.0, true);
        // First detonation: panel still consumable, returns Some(1.0).
        let first = m.consume_era_panel();
        assert_eq!(first, Some(1.0));
        assert!(!m.era_consumable, "era_consumable must transition true -> false");
        // Second detonation: panel spent.
        let second = m.consume_era_panel();
        assert_eq!(second, None);
        assert!(!m.era_consumable, "spent panel stays spent");
    }

    /// **VAL-M14C-002 follow-on**: non-ERA modules never report a panel
    /// charge.
    #[test]
    fn non_era_module_does_not_consume_panel() {
        let mut m = ChassisModule::new("ammo.main", ModuleKind::AmmoRack, BodyZone::Torso, 60.0);
        assert!(!m.era_consumable);
        assert_eq!(m.consume_era_panel(), None);
    }

    /// **VAL-M14C-002 / VAL-M14C-025**: builder helper sets era_charge_kg
    /// on ERA modules and ignores it on non-ERA kinds.
    #[test]
    fn era_builder_helper_only_affects_era_modules() {
        let m = ChassisModule::new("era.heavy", ModuleKind::Era, BodyZone::Torso, 60.0).with_era(1.5, true);
        assert!((m.era_charge_kg - 1.5).abs() < 1e-6);
        let n = ChassisModule::new("ammo.main", ModuleKind::AmmoRack, BodyZone::Torso, 60.0).with_era(1.5, true);
        assert!((n.era_charge_kg - 0.0).abs() < 1e-6, "non-ERA module unchanged");
        assert!(!n.era_consumable);
    }
}
