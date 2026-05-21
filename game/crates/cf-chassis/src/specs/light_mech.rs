use crate::{
    Aabb, ArmorMountAngles, BodyZone, ChassisKind, ChassisModule, ChassisSpec, FailureCascade, ModuleKind,
    LIGHT_MECH_ID,
};

use super::common::make_zone;
use super::infantry::infantry_body_graph;

/// Build the canonical Light Mech chassis spec — ~3x human, heavier armor,
/// repair drone, jet pack.
pub fn light_mech_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Head, 60.0, 10.0, 30.0, 5.0, 36.0, 12.0),
        make_zone(BodyZone::Torso, 180.0, 12.0, 100.0, 6.0, 120.0, 30.0),
        make_zone(BodyZone::ArmRight, 80.0, 8.0, 50.0, 4.0, 60.0, 12.0),
        make_zone(BodyZone::ArmLeft, 80.0, 8.0, 50.0, 4.0, 60.0, 12.0),
        make_zone(BodyZone::LegRight, 100.0, 8.0, 60.0, 4.0, 80.0, 16.0),
        make_zone(BodyZone::LegLeft, 100.0, 8.0, 60.0, 4.0, 80.0, 16.0),
        make_zone(BodyZone::Backpack, 60.0, 6.0, 40.0, 3.0, 30.0, 4.0),
        make_zone(BodyZone::ForearmRight, 50.0, 6.0, 30.0, 3.0, 36.0, 8.0),
        make_zone(BodyZone::ForearmLeft, 50.0, 6.0, 30.0, 3.0, 36.0, 8.0),
        make_zone(BodyZone::HandRight, 30.0, 4.0, 20.0, 2.0, 24.0, 6.0),
        make_zone(BodyZone::HandLeft, 30.0, 4.0, 20.0, 2.0, 24.0, 6.0),
        make_zone(BodyZone::ShinRight, 70.0, 6.0, 40.0, 3.0, 54.0, 10.0),
        make_zone(BodyZone::ShinLeft, 70.0, 6.0, 40.0, 3.0, 54.0, 10.0),
        make_zone(BodyZone::FootRight, 40.0, 4.0, 28.0, 2.0, 32.0, 8.0),
        make_zone(BodyZone::FootLeft, 40.0, 4.0, 28.0, 2.0, 32.0, 8.0),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.rifle", ModuleKind::WeaponMount, BodyZone::ArmRight, 100.0),
        ChassisModule::new("jet.heavy", ModuleKind::Jet, BodyZone::Backpack, 80.0),
        ChassisModule::new("shield.heavy", ModuleKind::Shield, BodyZone::Torso, 100.0),
        ChassisModule::new("sensor.array", ModuleKind::Sensor, BodyZone::Head, 50.0),
        ChassisModule::new("repair_drone.bay", ModuleKind::RepairDrone, BodyZone::Torso, 50.0),
        // **M13** § "Per-chassis module positions" — Light Mech (cockpit + frame):
        // cockpit (top-front), reactor (torso center), fuel_tank (torso back),
        // ammo_rack (torso side; explosive), engine (torso back; fire-risk),
        // transmission (between torso + leg), motor_controller_per_leg,
        // optics_pod (head), comm_relay (head), targeting_computer (chest).
        ChassisModule::new("cockpit.main", ModuleKind::Cockpit, BodyZone::Head, 120.0)
            .with_local_aabb(Aabb::new(-4.0, 28.0, 4.0, 36.0))
            .with_failure_cascade(FailureCascade::PilotDirectDamage),
        ChassisModule::new("reactor.core", ModuleKind::Reactor, BodyZone::Torso, 180.0)
            .with_local_aabb(Aabb::new(-6.0, 8.0, 6.0, 20.0))
            .with_failure_cascade(FailureCascade::ReactorOverpressure),
        ChassisModule::new("fuel_tank.rear", ModuleKind::FuelTank, BodyZone::Torso, 80.0)
            .with_local_aabb(Aabb::new(2.0, 4.0, 8.0, 16.0)),
        ChassisModule::new("ammo_rack.main", ModuleKind::AmmoRack, BodyZone::Torso, 60.0)
            .with_local_aabb(Aabb::new(-8.0, 6.0, -3.0, 14.0))
            .with_ammo(15),
        ChassisModule::new("engine.main", ModuleKind::Engine, BodyZone::Torso, 100.0)
            .with_local_aabb(Aabb::new(0.0, 2.0, 6.0, 10.0))
            .with_failure_cascade(FailureCascade::EngineFire),
        ChassisModule::new("transmission.main", ModuleKind::Transmission, BodyZone::Torso, 60.0)
            .with_local_aabb(Aabb::new(-3.0, -2.0, 3.0, 4.0))
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.left", ModuleKind::MotorController, BodyZone::LegLeft, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.right", ModuleKind::MotorController, BodyZone::LegRight, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("optics_pod.main", ModuleKind::Optics, BodyZone::Head, 35.0)
            .with_local_aabb(Aabb::new(-2.5, 30.0, 2.5, 34.0))
            .with_failure_cascade(FailureCascade::SightImpairment),
        ChassisModule::new("comm_relay.head", ModuleKind::CommRelay, BodyZone::Head, 25.0)
            .with_local_aabb(Aabb::new(-2.0, 33.0, 2.0, 36.0)),
        ChassisModule::new("targeting_computer.chest", ModuleKind::TargetingComputer, BodyZone::Torso, 35.0)
            .with_local_aabb(Aabb::new(-3.0, 18.0, 3.0, 22.0)),
    ];
    ChassisSpec {
        id: LIGHT_MECH_ID.to_string(),
        kind: ChassisKind::LightMech,
        display_name: "Light Mech LM-1".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.5,
        mass_kg: 1800.0,
        // Light Mech (cockpit): 30° front slope, 0° side, 15° back slope.
        armor_angles: ArmorMountAngles::new(30.0, 0.0, 15.0),
    }
}
