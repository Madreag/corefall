//! engine_handle::observe_actor_impl — extracted from engine_handle.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::{json, Value};

use cf_actor::{ActorId, ControlIntent, Vec2};
use cf_replay::{ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig, ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord};

use crate::engine::*;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::Settings;

impl M0Engine {
    pub(crate) fn observe_actor_impl(&self, actor_id: Option<u64>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id = actor_id.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&ActorId(target_id))?;
        let rifle = sim.rifles.get(&ActorId(target_id));
        let observation = cf_actor::ActorObservation::from_actor_and_rifle(actor, rifle);
        let mut payload = serde_json::to_value(observation).ok()?;
        // sim state + atmosphere snapshot.
        let mass_breakdown = cf_actor::mass_breakdown(actor);
        let mass_total = mass_breakdown.total();
        let mass_factor = if mass_total > 0.0 {
            (80.0_f32 / mass_total).clamp(0.25, 1.2)
        } else {
            1.0
        };
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("total_mass_kg".to_string(), json!(mass_total));
            obj.insert("mass_factor_walk".to_string(), json!(mass_factor));
            obj.insert("mass_factor_jump".to_string(), json!(mass_factor));
            obj.insert("chassis_mass_kg".to_string(), json!(mass_breakdown.chassis_kg));
            obj.insert("limb_mass_kg".to_string(), json!(mass_breakdown.limb_kg));
            obj.insert("held_devices_mass_kg".to_string(), json!(mass_breakdown.held_kg));
            obj.insert("inventory_weight_kg".to_string(), json!(mass_breakdown.inventory_kg));
            obj.insert(
                "jetpack_dry_mass_kg".to_string(),
                json!(actor.jetpack.as_ref().map_or(0.0, |j| j.dry_mass_kg)),
            );
            obj.insert(
                "jetpack_fuel_mass_kg".to_string(),
                json!(mass_breakdown.jetpack_fuel_kg),
            );
            obj.insert("wound_mass_kg".to_string(), json!(mass_breakdown.wound_kg));
            // Walking sim state.
            obj.insert("move_state".to_string(), json!(actor.move_state.as_str()));
            obj.insert("prone_state".to_string(), json!(actor.prone_state.as_str()));
            obj.insert("upper_body_state".to_string(), json!(actor.upper_body_state.as_str()));
            obj.insert(
                "attitude".to_string(),
                json!({
                    "rot": actor.attitude.rot,
                    "angular_vel": actor.attitude.angular_vel,
                    "rot_target": actor.attitude.rot_target,
                }),
            );
            obj.insert(
                "walk_angle".to_string(),
                json!({"fg": actor.walk_angle.fg, "bg": actor.walk_angle.bg}),
            );
            obj.insert(
                "walk_path_offset".to_string(),
                json!({"x": actor.walk_path_offset.x, "y": actor.walk_path_offset.y}),
            );
            obj.insert(
                "arm_sway".to_string(),
                json!({
                    "fg_arm_rot": actor.arm_sway.fg_arm_rot,
                    "bg_arm_rot": actor.arm_sway.bg_arm_rot,
                    "head_rot": actor.arm_sway.head_rot,
                    "bg_supporting_fg": actor.arm_sway.bg_supporting_fg,
                }),
            );
            obj.insert("stride_frame".to_string(), json!(actor.stride_frame));
            obj.insert("stride_timer_ms".to_string(), json!(actor.stride_timer_ms));
            obj.insert("last_stride_side_fg".to_string(), json!(actor.last_stride_side_fg));
            // Jetpack surface.
            if let Some(jet) = actor.jetpack.as_ref() {
                obj.insert(
                    "jetpack".to_string(),
                    json!({
                        "id": jet.id,
                        "type": jet.jetpack_type.as_str(),
                        "jet_time_left_ms": jet.jet_time_left_ms,
                        "jet_time_total_ms": jet.jet_time_total_ms,
                        "fuel_ratio": jet.fuel_ratio(),
                        "is_emitting": jet.is_emitting,
                        "throttle": jet.throttle,
                        "emit_angle": jet.emit_angle,
                    }),
                );
            }
            // Atmosphere overlay surface.
            obj.insert(
                "atmosphere".to_string(),
                json!({
                    "pressure_kpa": actor.atmosphere_sample.pressure_kpa,
                    "temp_k": actor.atmosphere_sample.temp_k,
                    "o2_partial_kpa": actor.atmosphere_sample.o2_partial_kpa,
                    "pollutant_partial_kpa": actor.atmosphere_sample.pollutant_partial_kpa,
                    "volatiles_partial_kpa": actor.atmosphere_sample.volatiles_partial_kpa,
                    "smoke_pct": actor.atmosphere_sample.smoke_pct,
                    "wind": actor.atmosphere_sample.wind,
                    "local_gravity_m_s2": actor.atmosphere_sample.local_gravity_m_s2,
                    "hypoxia_severity": actor.atmosphere_sample.hypoxia_severity(),
                }),
            );
        }
        Some(payload)
    }
}
