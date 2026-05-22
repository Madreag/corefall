use crate::{
    Aabb, ArmorMountAngles, BodyZone, ChassisKind, ChassisModule, ChassisSpec, FailureCascade, ModuleKind,
    POWERED_ARMOR_ID,
};

use super::common::make_zone;
use super::infantry::infantry_body_graph;

/// Build the canonical Powered Armor chassis spec — Spartan-ish, jet pack,
/// shield generator, full body armor.
pub fn powered_armor_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Head, 30.0, 6.0, 18.0, 3.0, 24.0, 12.0),
        make_zone(BodyZone::Torso, 80.0, 8.0, 50.0, 4.0, 60.0, 30.0),
        make_zone(BodyZone::ArmRight, 36.0, 5.0, 24.0, 2.0, 30.0, 12.0),
        make_zone(BodyZone::ArmLeft, 36.0, 5.0, 24.0, 2.0, 30.0, 12.0),
        make_zone(BodyZone::LegRight, 40.0, 5.0, 24.0, 2.0, 36.0, 16.0),
        make_zone(BodyZone::LegLeft, 40.0, 5.0, 24.0, 2.0, 36.0, 16.0),
        make_zone(BodyZone::Backpack, 30.0, 4.0, 20.0, 2.0, 18.0, 4.0),
        make_zone(BodyZone::ForearmRight, 24.0, 4.0, 16.0, 2.0, 20.0, 8.0),
        make_zone(BodyZone::ForearmLeft, 24.0, 4.0, 16.0, 2.0, 20.0, 8.0),
        make_zone(BodyZone::HandRight, 18.0, 3.0, 12.0, 1.0, 14.0, 6.0),
        make_zone(BodyZone::HandLeft, 18.0, 3.0, 12.0, 1.0, 14.0, 6.0),
        make_zone(BodyZone::ShinRight, 28.0, 4.0, 18.0, 2.0, 22.0, 10.0),
        make_zone(BodyZone::ShinLeft, 28.0, 4.0, 18.0, 2.0, 22.0, 10.0),
        make_zone(BodyZone::FootRight, 20.0, 3.0, 14.0, 1.0, 16.0, 8.0),
        make_zone(BodyZone::FootLeft, 20.0, 3.0, 14.0, 1.0, 16.0, 8.0),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.rifle", ModuleKind::WeaponMount, BodyZone::ArmRight, 60.0),
        ChassisModule::new("jet.pack", ModuleKind::Jet, BodyZone::Backpack, 40.0),
        ChassisModule::new("shield.bubble", ModuleKind::Shield, BodyZone::Torso, 50.0),
        ChassisModule::new("sensor.scope", ModuleKind::Sensor, BodyZone::Head, 25.0),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
        // power_core (torso center), targeting_computer (head),
        // gun_mount (arm), shield_emitter (chest).
        ChassisModule::new("power_core.cell", ModuleKind::PowerCore, BodyZone::Torso, 50.0)
            .with_local_aabb(Aabb::new(-3.0, 4.0, 3.0, 12.0)),
        ChassisModule::new("targeting_computer.optics", ModuleKind::Optics, BodyZone::Head, 25.0)
            .with_local_aabb(Aabb::new(-2.0, 16.0, 2.0, 20.0))
            .with_failure_cascade(FailureCascade::SightImpairment),
        ChassisModule::new("targeting_computer.cpu", ModuleKind::TargetingComputer, BodyZone::Head, 20.0)
            .with_local_aabb(Aabb::new(-1.5, 14.0, 1.5, 17.0)),
    ];
    ChassisSpec {
        id: POWERED_ARMOR_ID.to_string(),
        kind: ChassisKind::PoweredArmor,
        display_name: "Powered Armor MK-I".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.0,
        mass_kg: 350.0,
        // Powered Armor (Spartan-ish): 15° front slope, 0° side, 15° back slope.
        armor_angles: ArmorMountAngles::new(15.0, 0.0, 15.0),
    }
}
