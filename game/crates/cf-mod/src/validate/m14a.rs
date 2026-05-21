use std::{fs, path::Path};

use crate::report::ValidationReport;

/// M14A: validate `content/limb_paths/*.ron`.
pub(crate) fn validate_m14a_limb_path(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Spec {
        schema_version: u32,
        chassis_archetype: String,
        move_state: String,
        side: String,
        start: (f32, f32),
        segments: Vec<(f32, f32)>,
        travel_speed: Vec<f32>,
        travel_speed_multiplier: f32,
        push_force: f32,
        foot_collisions_disabled_segment: i32,
    }
    match ron::from_str::<Spec>(&raw) {
        Ok(s) => {
            if s.schema_version != 1 {
                report.add_error(
                    path.to_path_buf(),
                    format!("limb_path schema_version must be 1, got {}", s.schema_version),
                );
                return;
            }
            if s.segments.is_empty() {
                report.add_error(path.to_path_buf(), "limb_path must have ≥1 segment".to_string());
                return;
            }
            if s.push_force <= 0.0 {
                report.add_error(path.to_path_buf(), "limb_path push_force must be > 0".to_string());
                return;
            }
            if !matches!(
                s.move_state.as_str(),
                "no_move"
                    | "stand"
                    | "walk"
                    | "crouch"
                    | "crawl"
                    | "arm_crawl"
                    | "climb"
                    | "jump"
                    | "dislodge"
                    | "hover"
            ) {
                report.add_error(
                    path.to_path_buf(),
                    format!("limb_path unknown move_state: {}", s.move_state),
                );
                return;
            }
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "limb_path ({} {} {} segs={})",
                    s.chassis_archetype,
                    s.move_state,
                    s.side,
                    s.segments.len()
                ),
            );
        }
        Err(err) => report.add_error(path.to_path_buf(), format!("ron parse failed: {err}")),
    }
}

/// M14A: validate `content/jetpacks/*.ron`.
pub(crate) fn validate_m14a_jetpack(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Spec {
        schema_version: u32,
        id: String,
        jetpack_type: String,
        jet_time_total_ms: u32,
        jet_replenish_rate: f32,
        minimum_fuel_ratio: f32,
        jet_angle_range: f32,
        can_adjust_angle_while_firing: bool,
        adjusts_throttle_for_weight: bool,
        base_thrust_n: f32,
        burst_thrust_multiplier: f32,
        dry_mass_kg: f32,
        fuel_density_kg_per_ms: f32,
        bound_zone: String,
        emitter_offset: (f32, f32),
    }
    match ron::from_str::<Spec>(&raw) {
        Ok(s) => {
            if s.schema_version != 1 {
                report.add_error(path.to_path_buf(), "jetpack schema_version must be 1".to_string());
                return;
            }
            if !matches!(s.jetpack_type.as_str(), "standard" | "jump_pack") {
                report.add_error(
                    path.to_path_buf(),
                    format!("unknown jetpack_type: {}", s.jetpack_type),
                );
                return;
            }
            if s.minimum_fuel_ratio < 0.0 || s.minimum_fuel_ratio > 1.0 {
                report.add_error(path.to_path_buf(), "minimum_fuel_ratio out of [0,1]".to_string());
                return;
            }
            if s.base_thrust_n <= 0.0 {
                report.add_error(path.to_path_buf(), "base_thrust_n must be > 0".to_string());
                return;
            }
            report.add_pass(
                path.to_path_buf(),
                format!(
                    "jetpack ({} type={} thrust={}N)",
                    s.id, s.jetpack_type, s.base_thrust_n
                ),
            );
        }
        Err(err) => report.add_error(path.to_path_buf(), format!("ron parse failed: {err}")),
    }
}

/// M14A: validate `content/quick_action_layouts/*.ron`.
pub(crate) fn validate_m14a_quick_action_layout(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct SlotSpec {
        slot: u8,
        kind: String,
        item_id: String,
        ammo: u32,
        ammo_max: u32,
        cooldown_total_ticks: u32,
    }
    #[derive(serde::Deserialize)]
    #[allow(dead_code)]
    struct Spec {
        schema_version: u32,
        chassis_archetype: String,
        slots: Vec<SlotSpec>,
    }
    match ron::from_str::<Spec>(&raw) {
        Ok(s) => {
            if s.schema_version != 1 {
                report.add_error(
                    path.to_path_buf(),
                    "quick_action_layout schema_version must be 1".to_string(),
                );
                return;
            }
            if s.slots.len() != 8 {
                report.add_error(
                    path.to_path_buf(),
                    format!("quick_action_layout must define 8 slots, got {}", s.slots.len()),
                );
                return;
            }
            for slot in &s.slots {
                if !matches!(
                    slot.kind.as_str(),
                    "empty" | "weapon" | "melee" | "grenade" | "consumable" | "ability" | "tool"
                ) {
                    report.add_error(
                        path.to_path_buf(),
                        format!("quick_action_layout unknown slot kind: {}", slot.kind),
                    );
                    return;
                }
            }
            report.add_pass(
                path.to_path_buf(),
                format!("quick_action_layout ({}, 8 slots)", s.chassis_archetype),
            );
        }
        Err(err) => report.add_error(path.to_path_buf(), format!("ron parse failed: {err}")),
    }
}
