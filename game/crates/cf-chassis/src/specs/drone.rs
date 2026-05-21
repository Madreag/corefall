use crate::{
    Aabb, ArmorMountAngles, BodyGraph, BodyZone, ChassisKind, ChassisModule, ChassisSpec, EquipmentSocket,
    FailureCascade, Joint, ModuleKind, MovementContribution, DRONE_ID,
};

use super::common::make_zone;

/// **M13** § "Drone" body graph: 4 zones (chassis core + 2 arms + sensor pod).
pub(crate) fn drone_body_graph() -> BodyGraph {
    let zones = vec![
        BodyZone::DroneCore,
        BodyZone::DroneArmLeft,
        BodyZone::DroneArmRight,
        BodyZone::DroneSensorPod,
    ];
    let joints = vec![
        Joint {
            id: "arm_left_to_core".to_string(),
            parent: BodyZone::DroneCore,
            child: BodyZone::DroneArmLeft,
            intact: true,
        },
        Joint {
            id: "arm_right_to_core".to_string(),
            parent: BodyZone::DroneCore,
            child: BodyZone::DroneArmRight,
            intact: true,
        },
        Joint {
            id: "sensor_pod_to_core".to_string(),
            parent: BodyZone::DroneCore,
            child: BodyZone::DroneSensorPod,
            intact: true,
        },
    ];
    let sockets = vec![
        EquipmentSocket {
            id: "drone_arm_left".to_string(),
            zone: BodyZone::DroneArmLeft,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: "drone_arm_right".to_string(),
            zone: BodyZone::DroneArmRight,
            occupied: false,
            mounted_role: None,
        },
    ];
    let movement_contributions = vec![
        MovementContribution {
            zone: BodyZone::DroneCore,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: true,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::DroneArmLeft,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::DroneArmRight,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::DroneSensorPod,
            ..MovementContribution::neutral(BodyZone::DroneSensorPod)
        },
    ];
    BodyGraph {
        zones,
        joints,
        sockets,
        movement_contributions,
    }
}

/// **M13** § "Drone" chassis spec. 4-zone autonomous miniature chassis; no pilot.
pub fn drone_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::DroneCore, 40.0, 4.0, 24.0, 2.0, 30.0, 16.0),
        make_zone(BodyZone::DroneArmLeft, 20.0, 2.0, 12.0, 1.0, 14.0, 8.0),
        make_zone(BodyZone::DroneArmRight, 20.0, 2.0, 12.0, 1.0, 14.0, 8.0),
        make_zone(BodyZone::DroneSensorPod, 15.0, 2.0, 10.0, 1.0, 12.0, 6.0),
    ];
    let modules = vec![
        ChassisModule::new("power_core.cell", ModuleKind::PowerCore, BodyZone::DroneCore, 40.0)
            .with_local_aabb(Aabb::new(-2.0, 0.0, 2.0, 4.0)),
        ChassisModule::new("sensor_pod.main", ModuleKind::Sensor, BodyZone::DroneSensorPod, 25.0),
        ChassisModule::new("motor_controller.l", ModuleKind::MotorController, BodyZone::DroneArmLeft, 18.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.r", ModuleKind::MotorController, BodyZone::DroneArmRight, 18.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("comm_relay.main", ModuleKind::CommRelay, BodyZone::DroneCore, 18.0),
        ChassisModule::not_present("jet.none", ModuleKind::Jet),
        ChassisModule::not_present("weapon_mount.none", ModuleKind::WeaponMount),
        ChassisModule::not_present("shield.none", ModuleKind::Shield),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: DRONE_ID.to_string(),
        kind: ChassisKind::Drone,
        display_name: "Recon Drone DR-1".to_string(),
        body_graph: drone_body_graph(),
        zones,
        modules,
        eject_window_seconds: 0.5,
        mass_kg: 60.0,
        // Drone: small target; flat armor mount on all faces.
        armor_angles: ArmorMountAngles::new(0.0, 0.0, 0.0),
    }
}
