//! engine_handle::inspect_chassis_impl — extracted from engine_handle.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use serde_json::{json, Value};

use cf_actor::{ActorId, ControlIntent, Vec2};
use cf_replay::{ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig, ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord};

use crate::engine::*;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {
    pub(crate) fn inspect_chassis_impl(&self, target: Option<&str>) -> Option<serde_json::Value> {
        let state = self.state.read().ok()?;
        let sim = state.actor_state.as_ref()?;
        let target_id_opt: Option<u64> = match target {
            None | Some("player") | Some("") => None,
            Some(t) => t.parse::<u64>().ok(),
        };
        let target_id = target_id_opt.unwrap_or_else(|| sim.world.player.map(|id| id.0).unwrap_or(0));
        let actor = sim.world.actors.get(&cf_actor::ActorId(target_id))?;
        let chassis = actor.chassis.as_ref()?;

        // Build per-zone integrity payload.
        let zones: Vec<serde_json::Value> = chassis
            .zones
            .iter()
            .map(|z| {
                let layers: Vec<serde_json::Value> = z
                    .layers
                    .iter()
                    .map(|l| {
                        json!({
                            "kind": l.kind.as_str(),
                            "hp": l.hp,
                            "hp_max": l.hp_max,
                            "hardness": l.hardness,
                            "integrity": l.integrity(),
                            "breached": l.is_breached(),
                        })
                    })
                    .collect();
                json!({
                    "zone": z.zone.as_str(),
                    "layers": layers,
                    "external_integrity": z.external_integrity(),
                    "internal_integrity": z.internal_integrity(),
                    "core_integrity": z.core_integrity(),
                    "wound_hp": z.wound_hp,
                    "wound_hp_max": z.wound_hp_max,
                    "wound_integrity": z.wound_integrity(),
                    "zone_integrity": z.zone_integrity(),
                    "destroyed": z.destroyed,
                })
            })
            .collect();
        // Joints — exactly 14 for the humanoid body graph.
        let joints: Vec<serde_json::Value> = chassis
            .body_graph
            .joints
            .iter()
            .map(|j| {
                json!({
                    "id": j.id,
                    "parent": j.parent.as_str(),
                    "child": j.child.as_str(),
                    "intact": j.intact,
                })
            })
            .collect();
        // Equipment sockets — 5 for the humanoid body graph.
        let sockets: Vec<serde_json::Value> = chassis
            .body_graph
            .sockets
            .iter()
            .map(|s| {
                json!({
                    "id": s.id,
                    "zone": s.zone.as_str(),
                    "occupied": s.occupied,
                    "mounted_role": s.mounted_role,
                })
            })
            .collect();
        // Modules — id, kind, state, bound zone, hp.
        let modules: Vec<serde_json::Value> = chassis
            .modules
            .iter()
            .map(|m| {
                json!({
                    "id": m.id,
                    "kind": m.kind.as_str(),
                    "state": m.state.as_str(),
                    "bound_zone": m.bound_zone.as_str(),
                    "hp": m.hp,
                    "hp_max": m.hp_max,
                    "integrity": m.integrity(),
                    "last_reason": m.last_reason,
                })
            })
            .collect();
        // Movement-contribution table from the body graph (which capabilities are
        // lost when each zone is destroyed). Useful for AI doctrine + HUD.
        let movement_contributions: Vec<serde_json::Value> = chassis
            .body_graph
            .movement_contributions
            .iter()
            .map(|c| {
                json!({
                    "zone": c.zone.as_str(),
                    "move_speed_factor_when_destroyed": c.move_speed_factor_when_destroyed,
                    "jump_impulse_factor_when_destroyed": c.jump_impulse_factor_when_destroyed,
                    "disables_rifle_when_destroyed": c.disables_rifle_when_destroyed,
                    "forces_crawl_when_destroyed": c.forces_crawl_when_destroyed,
                    "drops_gear_when_destroyed": c.drops_gear_when_destroyed,
                    "disables_jet_when_destroyed": c.disables_jet_when_destroyed,
                })
            })
            .collect();
        let salvaged_module_ids: Vec<String> = chassis.salvaged_modules.iter().map(|m| m.id.clone()).collect();
        let destroyed_zones: Vec<String> = chassis
            .destroyed_zones()
            .iter()
            .map(|z| z.as_str().to_string())
            .collect();
        Some(json!({
            "schema_version": SCHEMA_VERSION,
            "actor_id": target_id,
            "spec_id": chassis.spec_id,
            "kind": chassis.kind.as_str(),
            "stage": chassis.stage.as_str(),
            "pilot_state": chassis.pilot_state.as_str(),
            "tutorial_safety": chassis.tutorial_safety,
            "mass_kg": chassis.mass_kg,
            "weapon_jammed": chassis.weapon_jammed,
            "tick_rate_hz": chassis.tick_rate_hz,
            "integrity": chassis.integrity(),
            "eject_ticks_remaining": chassis.eject_window.ticks_remaining,
            "eject_ticks_total": chassis.eject_window.ticks_total,
            "eject_triggered_at_tick": chassis.eject_window.triggered_at_tick,
            "body_graph": {
                "zone_count": chassis.body_graph.zones.len(),
                "joint_count": chassis.body_graph.joints.len(),
                "socket_count": chassis.body_graph.sockets.len(),
                "zones": chassis.body_graph.zones.iter().map(|z| z.as_str()).collect::<Vec<_>>(),
                "joints": joints,
                "sockets": sockets,
                "movement_contributions": movement_contributions,
            },
            "zones": zones,
            "destroyed_zones": destroyed_zones,
            "modules": modules,
            "salvaged_module_ids": salvaged_module_ids,
            "last_stage_reason": chassis.last_stage_reason,
        }))
    }
}
