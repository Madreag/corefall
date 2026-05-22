//! Scenario → runtime structure builders. Lifted from engine.rs.

use cf_actor::ActorWorld;
use std::collections::BTreeMap;

pub(crate) fn build_rifles_for_world(
    world: &ActorWorld,
    tick_rate_hz: u32,
) -> BTreeMap<cf_actor::ActorId, cf_equipment::RifleState> {
    let mut rifles = BTreeMap::new();
    for actor in world.actors.values() {
        for item in &actor.inventory.items {
            if let cf_actor::InventoryItem::Rifle { preset } = item {
                if let Some(spec) = cf_equipment::rifle_preset(preset) {
                    rifles.insert(actor.id, cf_equipment::RifleState::new(spec, tick_rate_hz));
                    break;
                }
            }
        }
    }
    rifles
}

pub(crate) fn build_gravity_override(
    ovr: &cf_mission::ScenarioGravityOverride,
) -> cf_physics::GravityOverride {
    match ovr {
        cf_mission::ScenarioGravityOverride::UniformWell { id, center, radius, magnitude } => {
            cf_physics::GravityOverride::UniformWell {
                id: *id,
                center: [center.0, center.1],
                radius: *radius,
                magnitude: *magnitude,
            }
        }
        cf_mission::ScenarioGravityOverride::RegionLowG { id, min, max, local_g } => {
            cf_physics::GravityOverride::RegionLowG {
                id: *id,
                min: [min.0, min.1],
                max: [max.0, max.1],
                local_g: *local_g,
            }
        }
        cf_mission::ScenarioGravityOverride::MagneticBoots { id, actor_id } => {
            cf_physics::GravityOverride::MagneticBoots { id: *id, actor_id: *actor_id }
        }
        cf_mission::ScenarioGravityOverride::ReverseG { id, min, max } => {
            cf_physics::GravityOverride::ReverseG {
                id: *id,
                min: [min.0, min.1],
                max: [max.0, max.1],
            }
        }
        cf_mission::ScenarioGravityOverride::DamagedGrav {
            id,
            center,
            radius,
            magnitude_factor,
            wave_front_radius,
            wave_front_growth_per_s,
        } => cf_physics::GravityOverride::DamagedGrav {
            id: *id,
            center: [center.0, center.1],
            radius: *radius,
            magnitude_factor: *magnitude_factor,
            wave_front_radius: *wave_front_radius,
            wave_front_growth_per_s: *wave_front_growth_per_s,
        },
    }
}

pub(crate) fn build_wind_source(w: &cf_mission::ScenarioWindSource) -> cf_atmos::WindSource {
    cf_atmos::WindSource {
        id: w.id,
        origin: [w.origin.0, w.origin.1],
        axis: [w.axis.0, w.axis.1],
        aperture_area_m2: w.aperture_area_m2,
        cell_high_id: w.cell_high_id,
        cell_low_id: w.cell_low_id,
        jet_length: w.jet_length,
        jet_half_width: w.jet_half_width,
    }
}

pub(crate) fn next_unit_draw(state: &mut u64) -> f32 {
    let mut z = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    *state = z;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    let bits = (z >> 40) as u32;
    (bits as f32) / ((1u32 << 24) as f32)
}

pub(crate) fn build_m14d_projectile_snapshot(
    p: &crate::scenario::ScenarioM14dProjectile,
) -> cf_physics::ProjectileSnapshot {
    cf_physics::ProjectileSnapshot {
        id: p.id,
        kind: p.kind,
        position: [p.position.0, p.position.1],
        velocity: [p.velocity.0, p.velocity.1],
        radius: p.radius.max(0.0),
        mass_kg: p.mass_kg.max(0.0),
        owner_actor_id: p.owner_actor_id,
    }
}

pub(crate) fn build_atmos_cell(c: &cf_mission::ScenarioAtmosCell) -> cf_atmos::AtmosCell {
    cf_atmos::AtmosCell {
        id: c.id,
        min: [c.min.0, c.min.1],
        max: [c.max.0, c.max.1],
        pressure_kpa: c.pressure_kpa,
        temp_k: c.temp_k,
    }
}

pub(crate) fn build_strat_cell(c: &cf_mission::ScenarioAtmosCell) -> cf_atmos::StratCell {
    let column_id = c.column_id.unwrap_or(c.id);
    let center_y = (c.min.1 + c.max.1) * 0.5;
    let fractions: Vec<(cf_atmos::Gas, f32)> = c
        .gases
        .iter()
        .map(|(label, frac)| (gas_from_label(label), frac.clamp(0.0, 1.0)))
        .collect();
    cf_atmos::StratCell {
        cell_id: c.id,
        column_id,
        center_y,
        fractions,
    }
}

pub(crate) fn gas_from_label(label: &str) -> cf_atmos::Gas {
    match label.to_ascii_lowercase().as_str() {
        "h2" | "hydrogen" => cf_atmos::Gas::H2,
        "he" | "helium" => cf_atmos::Gas::He,
        "methane" | "ch4" => cf_atmos::Gas::Methane,
        "water_vapor" | "h2o" | "watervapor" => cf_atmos::Gas::WaterVapor,
        "n2" | "nitrogen" => cf_atmos::Gas::N2,
        "o2" | "oxygen" => cf_atmos::Gas::O2,
        "co2" | "carbon_dioxide" => cf_atmos::Gas::CO2,
        "n2o" | "nitrous_oxide" => cf_atmos::Gas::N2O,
        "volatiles" | "fuel" => cf_atmos::Gas::Volatiles,
        _ => cf_atmos::Gas::Pollutant,
    }
}

pub(crate) fn m9_concussion_band_for_dose(dose: f32) -> &'static str {
    if dose >= 100.0 {
        "KO"
    } else if dose >= 80.0 {
        "KO_Imminent"
    } else if dose >= 60.0 {
        "Severe"
    } else if dose >= 40.0 {
        "Moderate"
    } else if dose >= 20.0 {
        "Mild"
    } else {
        "Clear"
    }
}

pub(crate) fn registry_color_hex_for(material_id: cf_terrain::MaterialId) -> Option<String> {
    let path = cf_material::MaterialRegistry::locate_default()?;
    let (registry, _report) = cf_material::load_registry_from_file(&path).ok()?;
    let def = registry.find_by_id(material_id)?;
    Some(def.color_hex.clone())
}

pub(crate) fn registry_color_hex_from_cache(
    cache: Option<&cf_material::MaterialRegistry>,
    material_id: cf_terrain::MaterialId,
) -> Option<String> {
    match cache {
        Some(reg) => reg.find_by_id(material_id).map(|def| def.color_hex.clone()),
        None => registry_color_hex_for(material_id),
    }
}


