use crate::{
    Aabb, ArmorMountAngles, BodyGraph, BodyZone, ChassisKind, ChassisModule, ChassisSpec, EquipmentSocket,
    FailureCascade, Joint, ModuleKind, MovementContribution, CRAB_QUADRUPED_ID, SOCKET_TORSO_HARDPOINT,
};

use super::common::make_zone;

/// body graph: 4 legs + 2 claws + torso + sensor cluster + carapace = 11 zones.
pub(crate) fn crab_body_graph() -> BodyGraph {
    let zones = vec![
        BodyZone::Torso,
        BodyZone::Carapace,
        BodyZone::SensorCluster,
        BodyZone::LegFrontLeft,
        BodyZone::LegFrontRight,
        BodyZone::LegRearLeft,
        BodyZone::LegRearRight,
        BodyZone::ClawLeft,
        BodyZone::ClawRight,
        BodyZone::Head,
        BodyZone::Backpack,
    ];
    let joints = vec![
        Joint {
            id: "carapace_to_torso".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::Carapace,
            intact: true,
        },
        Joint {
            id: "sensor_to_carapace".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::SensorCluster,
            intact: true,
        },
        Joint {
            id: "leg_front_left".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegFrontLeft,
            intact: true,
        },
        Joint {
            id: "leg_front_right".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegFrontRight,
            intact: true,
        },
        Joint {
            id: "leg_rear_left".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegRearLeft,
            intact: true,
        },
        Joint {
            id: "leg_rear_right".to_string(),
            parent: BodyZone::Carapace,
            child: BodyZone::LegRearRight,
            intact: true,
        },
        Joint {
            id: "claw_left".to_string(),
            parent: BodyZone::LegFrontLeft,
            child: BodyZone::ClawLeft,
            intact: true,
        },
        Joint {
            id: "claw_right".to_string(),
            parent: BodyZone::LegFrontRight,
            child: BodyZone::ClawRight,
            intact: true,
        },
    ];
    let sockets = vec![
        EquipmentSocket {
            id: "claw_left".to_string(),
            zone: BodyZone::ClawLeft,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: "claw_right".to_string(),
            zone: BodyZone::ClawRight,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: SOCKET_TORSO_HARDPOINT.to_string(),
            zone: BodyZone::Torso,
            occupied: false,
            mounted_role: None,
        },
    ];
    let movement_contributions = vec![
        MovementContribution {
            zone: BodyZone::Carapace,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: true,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::LegFrontLeft,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegFrontLeft)
        },
        MovementContribution {
            zone: BodyZone::LegFrontRight,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegFrontRight)
        },
        MovementContribution {
            zone: BodyZone::LegRearLeft,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegRearLeft)
        },
        MovementContribution {
            zone: BodyZone::LegRearRight,
            move_speed_factor_when_destroyed: 0.75,
            jump_impulse_factor_when_destroyed: 0.75,
            ..MovementContribution::neutral(BodyZone::LegRearRight)
        },
        MovementContribution {
            zone: BodyZone::ClawLeft,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ClawRight,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::SensorCluster,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
    ];
    BodyGraph {
        zones,
        joints,
        sockets,
        movement_contributions,
    }
}

pub fn crab_quadruped_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Torso, 120.0, 8.0, 80.0, 4.0, 100.0, 30.0),
        make_zone(BodyZone::Carapace, 200.0, 12.0, 100.0, 6.0, 140.0, 40.0),
        make_zone(BodyZone::SensorCluster, 30.0, 4.0, 18.0, 2.0, 24.0, 10.0),
        make_zone(BodyZone::LegFrontLeft, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::LegFrontRight, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::LegRearLeft, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::LegRearRight, 60.0, 6.0, 40.0, 3.0, 50.0, 16.0),
        make_zone(BodyZone::ClawLeft, 40.0, 5.0, 26.0, 2.0, 30.0, 12.0),
        make_zone(BodyZone::ClawRight, 40.0, 5.0, 26.0, 2.0, 30.0, 12.0),
        // Head zone retained for HUD silhouette parity; "head" zone on a crab
        // is the sensor-tower top.
        make_zone(BodyZone::Head, 25.0, 4.0, 15.0, 2.0, 20.0, 10.0),
        // No backpack on crab; emit empty zone so iteration order remains stable.
        make_zone(BodyZone::Backpack, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0),
    ];
    let modules = vec![
        ChassisModule::new("sensor_cluster.array", ModuleKind::Sensor, BodyZone::SensorCluster, 60.0),
        ChassisModule::new("carapace_core.reactor", ModuleKind::Reactor, BodyZone::Carapace, 140.0)
            .with_local_aabb(Aabb::new(-6.0, 4.0, 6.0, 14.0))
            .with_failure_cascade(FailureCascade::ReactorOverpressure),
        ChassisModule::new("motor_controller.fl", ModuleKind::MotorController, BodyZone::LegFrontLeft, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.fr", ModuleKind::MotorController, BodyZone::LegFrontRight, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.rl", ModuleKind::MotorController, BodyZone::LegRearLeft, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("motor_controller.rr", ModuleKind::MotorController, BodyZone::LegRearRight, 40.0)
            .with_failure_cascade(FailureCascade::MobilityLoss),
        ChassisModule::new("fuel_tank.back", ModuleKind::FuelTank, BodyZone::Carapace, 60.0)
            .with_local_aabb(Aabb::new(2.0, 6.0, 8.0, 12.0)),
        ChassisModule::new("weapon_mount.left", ModuleKind::WeaponMount, BodyZone::ClawLeft, 50.0),
        ChassisModule::new("weapon_mount.right", ModuleKind::WeaponMount, BodyZone::ClawRight, 50.0),
        // No jet on crab per spec.
        ChassisModule::not_present("jet.none", ModuleKind::Jet),
        ChassisModule::not_present("shield.none", ModuleKind::Shield),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: CRAB_QUADRUPED_ID.to_string(),
        kind: ChassisKind::CrabQuadruped,
        display_name: "Crab Quadruped CQ-1".to_string(),
        body_graph: crab_body_graph(),
        zones,
        modules,
        eject_window_seconds: 1.2,
        mass_kg: 1200.0,
        // Crab / quadruped: 30° front, 30° side, 30° back.
        armor_angles: ArmorMountAngles::new(30.0, 30.0, 30.0),
    }
}
