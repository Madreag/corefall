use std::collections::BTreeMap;

use crate::{ChassisSpec, CRAB_QUADRUPED_ID, DRONE_ID, HEAVY_TROOPER_ID, INFANTRY_ID, LIGHT_MECH_ID, POWERED_ARMOR_ID};

pub(crate) mod common;
pub(crate) mod crab;
pub(crate) mod drone;
pub(crate) mod heavy_trooper;
pub(crate) mod infantry;
pub(crate) mod light_mech;
pub(crate) mod powered_armor;

pub use crab::crab_quadruped_spec;
pub use drone::drone_spec;
pub use heavy_trooper::heavy_trooper_spec;
pub use infantry::infantry_spec;
pub use light_mech::light_mech_spec;
pub use powered_armor::powered_armor_spec;

/// Stable registry of every launch chassis spec. **M14A** ships 6 archetypes:
/// Infantry, Powered Armor, Light Mech, Crab Quadruped, Drone, Heavy Trooper.
pub fn chassis_specs() -> BTreeMap<&'static str, ChassisSpec> {
    let mut m = BTreeMap::new();
    m.insert(INFANTRY_ID, infantry_spec());
    m.insert(POWERED_ARMOR_ID, powered_armor_spec());
    m.insert(LIGHT_MECH_ID, light_mech_spec());
    m.insert(CRAB_QUADRUPED_ID, crab_quadruped_spec());
    m.insert(DRONE_ID, drone_spec());
    m.insert(HEAVY_TROOPER_ID, heavy_trooper_spec());
    m
}

pub fn chassis_spec(spec_id: &str) -> Option<ChassisSpec> {
    chassis_specs().get(spec_id).cloned()
}
