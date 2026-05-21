use crate::{
    ArmorMountAngles, BodyZone, ChassisKind, ChassisModule, ChassisSpec, ModuleKind, HEAVY_TROOPER_ID,
};

use super::common::make_zone;
use super::infantry::infantry_body_graph;

/// **M14A** § "Heavy Armor — `heavy_trooper_v1`" — tank-grade infantry chassis.
///
/// Per-zone External HP, hardness, and `damage_multiplier` / `gib_impulse_limit`
/// / `stagger_factor` are spec-locked so rifles glance + heavy never knocks down
/// on small-arms hits.
pub fn heavy_trooper_spec() -> ChassisSpec {
    let zones = vec![
        // Head: 240 HP / hardness 18 / dmg×0.6 / gib 1600 / stagger 0.2
        make_zone(BodyZone::Head, 240.0, 18.0, 80.0, 8.0, 120.0, 30.0)
            .with_damage_multiplier(0.6)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.2),
        // Torso: 400 HP / hardness 22 / dmg×0.6 / gib 3200 / stagger 0.2
        make_zone(BodyZone::Torso, 400.0, 22.0, 200.0, 12.0, 240.0, 60.0)
            .with_damage_multiplier(0.6)
            .with_gib_impulse_limit(3200.0)
            .with_stagger_factor(0.2),
        // Arms: 180 HP / hardness 16 / dmg×0.75 / gib 2400 / stagger 0.3
        make_zone(BodyZone::ArmRight, 180.0, 16.0, 80.0, 7.0, 100.0, 24.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        make_zone(BodyZone::ArmLeft, 180.0, 16.0, 80.0, 7.0, 100.0, 24.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        // Legs: 220 HP / hardness 16 / dmg×0.75 / gib 2400 / stagger 0.3
        make_zone(BodyZone::LegRight, 220.0, 16.0, 100.0, 8.0, 140.0, 32.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        make_zone(BodyZone::LegLeft, 220.0, 16.0, 100.0, 8.0, 140.0, 32.0)
            .with_damage_multiplier(0.75)
            .with_gib_impulse_limit(2400.0)
            .with_stagger_factor(0.3),
        // Backpack: 140 HP / hardness 12 / dmg×0.8 / gib 1600 / stagger 0.5
        make_zone(BodyZone::Backpack, 140.0, 12.0, 80.0, 6.0, 60.0, 16.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ForearmRight, 100.0, 14.0, 50.0, 5.0, 60.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ForearmLeft, 100.0, 14.0, 50.0, 5.0, 60.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::HandRight, 80.0, 12.0, 40.0, 4.0, 50.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::HandLeft, 80.0, 12.0, 40.0, 4.0, 50.0, 12.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ShinRight, 120.0, 14.0, 60.0, 5.0, 80.0, 16.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::ShinLeft, 120.0, 14.0, 60.0, 5.0, 80.0, 16.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::FootRight, 100.0, 12.0, 50.0, 4.0, 60.0, 14.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
        make_zone(BodyZone::FootLeft, 100.0, 12.0, 50.0, 4.0, 60.0, 14.0)
            .with_damage_multiplier(0.8)
            .with_gib_impulse_limit(1600.0)
            .with_stagger_factor(0.5),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.heavy", ModuleKind::WeaponMount, BodyZone::ArmRight, 200.0),
        ChassisModule::new("jet.heavy_trooper", ModuleKind::Jet, BodyZone::Backpack, 100.0),
        ChassisModule::new("shield.heavy_plate", ModuleKind::Shield, BodyZone::Torso, 150.0),
        ChassisModule::not_present("sensor.none", ModuleKind::Sensor),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: HEAVY_TROOPER_ID.to_string(),
        kind: ChassisKind::HeavyTrooper,
        display_name: "Heavy Trooper HT-1".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.2,
        mass_kg: 380.0,
        // Heavy Trooper armor mount angles: 40° front, 15° side, 30° back.
        armor_angles: ArmorMountAngles::new(40.0, 15.0, 30.0),
    }
}
