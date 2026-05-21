//! M15 engine glue: heat field init, thermal source injection,
//! scenario-driven AmbientWorld selection, and byte-view helpers.
//! Lifted out of engine.rs as part of the >2000 LOC split.

use crate::engine::EngineMutable;

pub(crate) fn u16_slice_to_bytes(pixels: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() * 2);
    for p in pixels {
        out.extend_from_slice(&p.to_le_bytes());
    }
    out
}

pub(crate) fn infer_ambient_world_from_scenario_id(scenario_id: &str) -> cf_material::AmbientWorld {
    let id = scenario_id.to_ascii_lowercase();
    if id.contains("vulcan") {
        cf_material::AmbientWorld::Vulcan
    } else if id.contains("mimas") {
        cf_material::AmbientWorld::Mimas
    } else if id.contains("mars") {
        cf_material::AmbientWorld::Mars
    } else {
        cf_material::AmbientWorld::Earth
    }
}

pub(crate) fn inject_thermal_sources_and_diffuse(state: &mut EngineMutable) {
    use cf_terrain::chunked::CHUNK_SIZE;
    let sources = cf_material::ThermalSourceTable::load_default_or_baseline();
    let diffuse_mix = sources.diffuse_mix;
    let cool_mix = sources.cool_mix;

    let chunked = match state.chunked_terrain.as_ref() {
        Some(t) => t,
        None => return,
    };
    let width = chunked.width_px as i64;
    let height = chunked.height_px as i64;
    let chunk_size = CHUNK_SIZE as i64;
    let awake = chunked.awake_chunk_coords();
    let coords: Vec<(i32, i32)> = if awake.is_empty() {
        chunked.allocated_chunk_coords()
    } else {
        awake
    };

    for (cx, cy) in &coords {
        let chunk_origin_x = (*cx as i64) * chunk_size;
        let chunk_origin_y = (*cy as i64) * chunk_size;
        for ly in 0..chunk_size {
            let y = chunk_origin_y + ly;
            if y < 0 || y >= height {
                continue;
            }
            for lx in 0..chunk_size {
                let x = chunk_origin_x + lx;
                if x < 0 || x >= width {
                    continue;
                }
                let mat = chunked.material_at(x, y);
                let source_temp = match sources.lookup(mat) {
                    Some(t) => t,
                    None => continue,
                };
                if let Some((heat_cx, heat_cy)) = state.heat_field.world_to_cell(x as f32, y as f32) {
                    let current = state.heat_field.temperature_at_cell(heat_cx, heat_cy);
                    if source_temp > current {
                        state.heat_field.set_temperature(heat_cx, heat_cy, source_temp);
                    }
                }
            }
        }
    }
    state.heat_field.diffuse(diffuse_mix);
    state.heat_field.cool_toward_ambient(cool_mix);
}

/// Populate the HeatField from scenario atmosphere_cells. Without this
/// the heat field stays at ambient and no thermal transitions fire.
pub(crate) fn build_heat_field_from_atmosphere(cells: &[cf_atmos::AtmosCell]) -> cf_terrain::HeatField {
    let mut field = cf_terrain::HeatField::default();
    if cells.is_empty() {
        return field;
    }
    let grid_size = cf_terrain::HEAT_GRID_SIZE;
    for atmos in cells {
        let temp = atmos.temp_k;
        if !temp.is_finite() || temp < 2.7 {
            continue;
        }
        for cy in 0..grid_size {
            for cx in 0..grid_size {
                let world_x = field.anchor[0] + (cx as f32 + 0.5) * field.cell_size_px;
                let world_y = field.anchor[1] + (cy as f32 + 0.5) * field.cell_size_px;
                if world_x >= atmos.min[0]
                    && world_x < atmos.max[0]
                    && world_y >= atmos.min[1]
                    && world_y < atmos.max[1]
                {
                    field.set_temperature(cx, cy, temp);
                }
            }
        }
    }
    field
}
