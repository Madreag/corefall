use crate::{
    ArmorMountAngles, BodyGraph, BodyZone, ChassisKind, ChassisModule, ChassisSpec, EquipmentSocket, Joint,
    ModuleKind, MovementContribution, INFANTRY_ID, SOCKET_BACK_MOUNT, SOCKET_HAND_LEFT, SOCKET_HAND_RIGHT,
    SOCKET_HEAD_MOUNT, SOCKET_TORSO_HARDPOINT,
};

use super::common::make_zone;

/// Build the canonical Infantry body graph (no chassis, just the body).
pub(crate) fn infantry_body_graph() -> BodyGraph {
    // **M13** preserves the M5 15-zone humanoid contract — quadruped + drone
    // zones do NOT appear in the humanoid body graph (they belong to the
    // crab_body_graph / drone_body_graph functions instead).
    let zones: Vec<BodyZone> = BodyZone::all()
        .iter()
        .filter(|z| !z.is_quadruped_zone() && !z.is_drone_zone())
        .copied()
        .collect();
    let joints = vec![
        Joint {
            id: "neck".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::Head,
            intact: true,
        },
        Joint {
            id: "shoulder_left".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::ArmLeft,
            intact: true,
        },
        Joint {
            id: "shoulder_right".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::ArmRight,
            intact: true,
        },
        Joint {
            id: "elbow_left".to_string(),
            parent: BodyZone::ArmLeft,
            child: BodyZone::ForearmLeft,
            intact: true,
        },
        Joint {
            id: "elbow_right".to_string(),
            parent: BodyZone::ArmRight,
            child: BodyZone::ForearmRight,
            intact: true,
        },
        Joint {
            id: "wrist_left".to_string(),
            parent: BodyZone::ForearmLeft,
            child: BodyZone::HandLeft,
            intact: true,
        },
        Joint {
            id: "wrist_right".to_string(),
            parent: BodyZone::ForearmRight,
            child: BodyZone::HandRight,
            intact: true,
        },
        Joint {
            id: "hip_left".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::LegLeft,
            intact: true,
        },
        Joint {
            id: "hip_right".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::LegRight,
            intact: true,
        },
        Joint {
            id: "knee_left".to_string(),
            parent: BodyZone::LegLeft,
            child: BodyZone::ShinLeft,
            intact: true,
        },
        Joint {
            id: "knee_right".to_string(),
            parent: BodyZone::LegRight,
            child: BodyZone::ShinRight,
            intact: true,
        },
        Joint {
            id: "ankle_left".to_string(),
            parent: BodyZone::ShinLeft,
            child: BodyZone::FootLeft,
            intact: true,
        },
        Joint {
            id: "ankle_right".to_string(),
            parent: BodyZone::ShinRight,
            child: BodyZone::FootRight,
            intact: true,
        },
        Joint {
            id: "back_mount".to_string(),
            parent: BodyZone::Torso,
            child: BodyZone::Backpack,
            intact: true,
        },
    ];
    let sockets = vec![
        EquipmentSocket {
            id: SOCKET_HAND_RIGHT.to_string(),
            // Hand socket is on the granular Hand zone (M5 spec) so dropping
            // the hand drops the rifle.
            zone: BodyZone::HandRight,
            occupied: true,
            mounted_role: Some(cf_equipment::RIFLE_M1_DEFAULT_ID.to_string()),
        },
        EquipmentSocket {
            id: SOCKET_HAND_LEFT.to_string(),
            zone: BodyZone::HandLeft,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: SOCKET_BACK_MOUNT.to_string(),
            zone: BodyZone::Backpack,
            occupied: false,
            mounted_role: None,
        },
        EquipmentSocket {
            id: SOCKET_HEAD_MOUNT.to_string(),
            zone: BodyZone::Head,
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
            zone: BodyZone::Head,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::Torso,
            move_speed_factor_when_destroyed: 0.0,
            jump_impulse_factor_when_destroyed: 0.0,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: true,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: true,
        },
        MovementContribution {
            zone: BodyZone::ArmRight,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ArmLeft,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::LegRight,
            move_speed_factor_when_destroyed: 0.5,
            jump_impulse_factor_when_destroyed: 0.4,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::LegLeft,
            move_speed_factor_when_destroyed: 0.5,
            jump_impulse_factor_when_destroyed: 0.4,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::Backpack,
            move_speed_factor_when_destroyed: 1.0,
            jump_impulse_factor_when_destroyed: 1.0,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: true,
        },
        // Granular forearm/hand consequences: destroying the right hand drops
        // the rifle entirely; destroying the right forearm reduces aim
        // stability + disables fine-control rifle handling.
        MovementContribution {
            zone: BodyZone::ForearmRight,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::HandRight,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: true,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: true,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ForearmLeft,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::HandLeft,
            move_speed_factor_when_destroyed: 0.95,
            jump_impulse_factor_when_destroyed: 0.95,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        // Granular leg consequences: destroying a shin or foot cripples
        // movement on that side (limp); losing both feet forces a crawl.
        MovementContribution {
            zone: BodyZone::ShinRight,
            move_speed_factor_when_destroyed: 0.4,
            jump_impulse_factor_when_destroyed: 0.3,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::ShinLeft,
            move_speed_factor_when_destroyed: 0.4,
            jump_impulse_factor_when_destroyed: 0.3,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::FootRight,
            move_speed_factor_when_destroyed: 0.6,
            jump_impulse_factor_when_destroyed: 0.5,
            disables_rifle_when_destroyed: false,
            forces_crawl_when_destroyed: false,
            drops_gear_when_destroyed: false,
            disables_jet_when_destroyed: false,
        },
        MovementContribution {
            zone: BodyZone::FootLeft,
            move_speed_factor_when_destroyed: 0.6,
            jump_impulse_factor_when_destroyed: 0.5,
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

/// Build the canonical Infantry chassis spec — minimal armor, just a body.
pub fn infantry_spec() -> ChassisSpec {
    let zones = vec![
        make_zone(BodyZone::Head, 4.0, 2.0, 4.0, 0.0, 6.0, 12.0),
        make_zone(BodyZone::Torso, 8.0, 2.0, 6.0, 0.0, 12.0, 30.0),
        make_zone(BodyZone::ArmRight, 4.0, 1.0, 4.0, 0.0, 6.0, 12.0),
        make_zone(BodyZone::ArmLeft, 4.0, 1.0, 4.0, 0.0, 6.0, 12.0),
        make_zone(BodyZone::LegRight, 5.0, 1.0, 5.0, 0.0, 8.0, 16.0),
        make_zone(BodyZone::LegLeft, 5.0, 1.0, 5.0, 0.0, 8.0, 16.0),
        make_zone(BodyZone::Backpack, 0.0, 0.0, 0.0, 0.0, 0.0, 4.0),
        make_zone(BodyZone::ForearmRight, 3.0, 1.0, 3.0, 0.0, 4.0, 8.0),
        make_zone(BodyZone::ForearmLeft, 3.0, 1.0, 3.0, 0.0, 4.0, 8.0),
        make_zone(BodyZone::HandRight, 2.0, 0.0, 2.0, 0.0, 3.0, 6.0),
        make_zone(BodyZone::HandLeft, 2.0, 0.0, 2.0, 0.0, 3.0, 6.0),
        make_zone(BodyZone::ShinRight, 4.0, 1.0, 4.0, 0.0, 5.0, 10.0),
        make_zone(BodyZone::ShinLeft, 4.0, 1.0, 4.0, 0.0, 5.0, 10.0),
        make_zone(BodyZone::FootRight, 3.0, 0.0, 3.0, 0.0, 4.0, 8.0),
        make_zone(BodyZone::FootLeft, 3.0, 0.0, 3.0, 0.0, 4.0, 8.0),
    ];
    let modules = vec![
        ChassisModule::new("weapon_mount.rifle", ModuleKind::WeaponMount, BodyZone::ArmRight, 30.0),
        ChassisModule::not_present("jet.none", ModuleKind::Jet),
        ChassisModule::not_present("shield.none", ModuleKind::Shield),
        ChassisModule::not_present("sensor.none", ModuleKind::Sensor),
        ChassisModule::not_present("repair_drone.none", ModuleKind::RepairDrone),
    ];
    ChassisSpec {
        id: INFANTRY_ID.to_string(),
        kind: ChassisKind::Infantry,
        display_name: "Infantry (Foot)".to_string(),
        body_graph: infantry_body_graph(),
        zones,
        modules,
        eject_window_seconds: 0.0,
        mass_kg: 90.0,
        // Infantry has no armor mount slope (per spec table: 0° / 0° / 0°).
        armor_angles: ArmorMountAngles::new(0.0, 0.0, 0.0),
    }
}
