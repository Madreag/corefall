//! Critical-module + penetration-ray + armor events.
//!
//! Extracted from engine.rs.

#![allow(unused_imports, dead_code, clippy::too_many_arguments)]

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use cf_actor::sim::{step as actor_step, ActorSimState, ActorTickOutcome, StepDeps, StepReport};
use cf_actor::{ActorId, ActorWorld, ControlIntent, IntentSource, ItemSlot, Vec2};
use cf_replay::{
    diagnostics, ArtifactItem, BuildInfo, BundleInputs, CapabilitiesBlock, CaptureConfig,
    ChecksumConfig, PerfSample, Recorder, RunManifest, SceneInfo, SettingsBlock, TestRecord,
    CONTROL_SCHEMA_VERSION, EVENT_ENVELOPE_VERSION, MANIFEST_SCHEMA_VERSION, SCENARIO_SCHEMA_VERSION,
};
use cf_sim_core::{
    checksum::{sim_state_v1, CHECKSUM_ALGORITHM, CHECKSUM_SCOPE},
    ids::{iso_hyphen_safe, make_run_id},
    Rng, SimClock, SimConfig, Tick, WallClock,
};

use crate::engine::*;
use crate::scenario::Scenario;
use crate::server::{async_trait, CommandResult, ControlCommand, EngineHandle, SettingsPatch};
use crate::state::{ActorView, ObserveFrame, ObserveSettings, RunStatus};
use crate::{Settings, SCHEMA_VERSION};

impl M0Engine {
    pub(crate) fn emit_critical_module_outcome_events(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        actor: ActorId,
        outcome: &cf_chassis::CriticalModuleOutcome,
        parent: Option<String>,
    ) {
        if let Some(transition) = &outcome.transition {
            let _ = self.recorder.record(
                tick,
                sim_time_ms,
                "chassis",
                "module_state_changed",
                json!({
                    "actor": actor.0,
                    "module_id": transition.id,
                    "state": transition.state.as_str(),
                    "reason": transition.reason,
                }),
                parent.clone(),
            );
        }
        for cascade in &outcome.cascade_events {
            let payload = match cascade {
                cf_chassis::CriticalModuleEvent::AmmoCooking { rounds_cooked } => json!({
                    "actor": actor.0,
                    "module_id": outcome.module_id,
                    "rounds": rounds_cooked,
                }),
                cf_chassis::CriticalModuleEvent::AmmoDetonated { rounds_detonated } => json!({
                    "actor": actor.0,
                    "module_id": outcome.module_id,
                    "rounds": rounds_detonated,
                }),
                cf_chassis::CriticalModuleEvent::EngineOilLeak | cf_chassis::CriticalModuleEvent::EngineFire => {
                    json!({"actor": actor.0, "module_id": outcome.module_id})
                }
                cf_chassis::CriticalModuleEvent::ReactorPressureAdvanced { tier } => json!({
                    "actor": actor.0,
                    "module_id": outcome.module_id,
                    "tier": tier,
                }),
                cf_chassis::CriticalModuleEvent::PilotDirectHit { damage } => json!({
                    "actor": actor.0,
                    "module_id": outcome.module_id,
                    "damage": damage,
                }),
                cf_chassis::CriticalModuleEvent::OpticsImpaired { blind } => json!({
                    "actor": actor.0,
                    "module_id": outcome.module_id,
                    "blind": blind,
                }),
                cf_chassis::CriticalModuleEvent::MobilityReduced { immobile } => json!({
                    "actor": actor.0,
                    "module_id": outcome.module_id,
                    "immobile": immobile,
                }),
            };
            let _ = self
                .recorder
                .record(tick, sim_time_ms, "module", cascade.as_str(), payload, parent.clone());
        }
    }

    /// **M5**: emit chassis-related events from a [`cf_chassis::ZoneDamageOutcome`].
    /// Also recomputes the chassis stage and emits `chassis.stage_changed` when it
    /// advances. **M13** appends `combat.hit_reaction_played` per zone hit, plus
    /// **M14** § "Full penetration ray flow". When a projectile pierces
    /// outer armor, this helper traces the ray through every chassis
    /// interior module in distance-from-impact order, applies module
    /// damage per `damage_multiplier × (1 - armor_absorption)`, and emits:
    ///
    ///   - `armor.penetration_ray_traversed` with the full module list
    ///   - per-module damage via `apply_critical_module_damage` so
    ///     module.hp + module.state mutate alongside the events
    ///   - `armor.spalling` + `armor.spalling_fragment_spawned` +
    ///     `armor.spalling_fragment_hit_module` when threshold crossed,
    ///     each spalling-fragment-hit ALSO mutates the target module's HP
    ///
    /// **M14 audit pass 3 (Finding 6)**: previous implementation was
    /// "pure event emission" — replay bundles claimed modules were hit
    /// for damage X while the chassis state never moved. Now the helper
    /// actually applies damage to each module via the chassis API.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn emit_m14_penetration_ray(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        actor: ActorId,
        hit: &cf_actor::sim::HitOutcome,
        outcome: &cf_chassis::ZoneDamageOutcome,
        ray_direction: [f32; 2],
        parent: Option<String>,
    ) {
        // Collect chassis interior modules.
        let modules_snapshot: Vec<(String, cf_chassis::ModuleKind, [f32; 2], bool, f32, f32)> = self
            .state
            .read()
            .ok()
            .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
            .and_then(|actors| {
                actors.get(&actor).and_then(|a| {
                    a.chassis.as_ref().map(|chassis| {
                        let position = a.position;
                        chassis
                            .modules
                            .iter()
                            .map(|m| {
                                let centre = if m.local_aabb.is_positioned() {
                                    let aabb = &m.local_aabb;
                                    [
                                        position.x + (aabb.min_x + aabb.max_x) * 0.5,
                                        position.y + (aabb.min_y + aabb.max_y) * 0.5,
                                    ]
                                } else {
                                    [position.x, position.y]
                                };
                                let dx = centre[0] - hit.hit_position.x;
                                let dy = centre[1] - hit.hit_position.y;
                                let dist = (dx * dx + dy * dy).sqrt();
                                let is_ammo_rack = matches!(m.kind, cf_chassis::ModuleKind::AmmoRack);
                                (
                                    m.id.clone(),
                                    m.kind,
                                    centre,
                                    is_ammo_rack,
                                    dist,
                                    1.0 - (m.hp / m.hp_max.max(0.001)).clamp(0.0, 1.0),
                                )
                            })
                            .collect::<Vec<_>>()
                    })
                })
            })
            .unwrap_or_default();
        if modules_snapshot.is_empty() {
            return;
        }
        let mut interior_modules: Vec<cf_physics::InteriorModule> = modules_snapshot
            .into_iter()
            .map(
                |(id, _kind, pos, is_ammo, dist, absorbed_frac)| cf_physics::InteriorModule {
                    id,
                    damage_multiplier: 0.6,
                    armor_absorption: absorbed_frac.clamp(0.0, 0.9),
                    position: pos,
                    distance_along_ray: dist,
                    is_ammo_rack: is_ammo,
                },
            )
            .collect();
        interior_modules.sort_by(|a, b| {
            a.distance_along_ray
                .partial_cmp(&b.distance_along_ray)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // Initial ray energy proxies projectile damage; backstop absorption
        // is M14 baseline (0.5) until M15 wires real ammo specs.
        let result = cf_physics::traverse_ray(
            [hit.hit_position.x, hit.hit_position.y],
            ray_direction,
            hit.damage * 4.0,
            &interior_modules,
            0.5,
        );
        let modules_hit_payload: Vec<serde_json::Value> = result
            .modules_hit
            .iter()
            .map(|m| serde_json::json!([m.module_id, m.damage]))
            .collect();
        let pen_ray_id = self.recorder.record(
            tick,
            sim_time_ms,
            "armor",
            "penetration_ray_traversed",
            json!({
                "ray_origin": result.ray_origin,
                "ray_direction": result.ray_direction,
                "modules_hit": modules_hit_payload,
                "final_resting_point": result.final_resting_point,
                "energy_remaining": result.energy_remaining,
                "exited_backstop": result.exited_backstop,
            }),
            parent.clone(),
        );
        // **M14 audit pass 3 (Finding 6)**: actually apply damage to each
        // interior module the ray traversed. This mutates module.hp +
        // module.state on the chassis so subsequent observe.frame +
        // snapshot_chassis events reflect the real state.
        // **M14 audit pass 4 (Finding 3)**: capture each outcome + emit
        // the chassis.module_state_changed transition + per-cascade
        // chassis.* events so replay viewers can observe state changes
        // caused by penetration rays the same way zone-damage paths do.
        for m in &result.modules_hit {
            let outcome_opt = if let Ok(mut s) = self.state.write() {
                s.actor_state.as_mut().and_then(|sim| {
                    sim.world.actors.get_mut(&actor).and_then(|target_actor| {
                        target_actor.chassis.as_mut().and_then(|chassis| {
                            chassis.apply_critical_module_damage(&m.module_id, m.damage, "penetration_ray")
                        })
                    })
                })
            } else {
                None
            };
            if let Some(outcome) = outcome_opt {
                self.emit_critical_module_outcome_events(tick, sim_time_ms, actor, &outcome, Some(pen_ray_id.clone()));
            }
        }
        // **M14** § "Spalling fragments spawn at impact point". When the
        // outer armor damage exceeds the spalling threshold, spawn 1-3
        // fragments + emit the per-fragment + per-module-hit events.
        let outer_armor_damage = outcome
            .layer_damage
            .iter()
            .map(|ld| ld.damage)
            .fold(0.0_f32, |a, b| a + b);
        let spalling_threshold = 5.0_f32;
        let rng_roll = if let Ok(mut s) = self.state.write() {
            (s.rng.next_u64() as f64 / u64::MAX as f64) as f32
        } else {
            0.5
        };
        let fragment_count = cf_physics::spalling_fragment_count(outer_armor_damage, spalling_threshold, rng_roll);
        if fragment_count > 0 {
            let zone_label = outcome
                .zone
                .map(|z| z.as_str().to_string())
                .unwrap_or_else(|| hit.zone.clone());
            let layer = outcome
                .layers_breached
                .first()
                .map(|(layer, _)| match layer {
                    cf_chassis::ArmorLayerKind::External => "External",
                    cf_chassis::ArmorLayerKind::Internal => "Internal",
                    cf_chassis::ArmorLayerKind::Core => "Core",
                })
                .unwrap_or("External");
            let damage_per_fragment = (outer_armor_damage
                * cf_physics::spalling_fragment_damage_fraction(0, fragment_count, rng_roll))
            .max(0.0);
            let spalling_id = self.recorder.record(
                tick,
                sim_time_ms,
                "armor",
                "spalling",
                json!({
                    "item_id": actor.0 as i64,
                    "zone": zone_label,
                    "layer": layer,
                    "fragment_count": fragment_count,
                    "damage_per_fragment": damage_per_fragment,
                    "cause_event_id": pen_ray_id.clone(),
                }),
                Some(pen_ray_id.clone()),
            );
            // Each fragment carries a fraction of the original damage and
            // hits 1-2 modules per spec.
            for frag_idx in 0..fragment_count {
                let frac = cf_physics::spalling_fragment_damage_fraction(frag_idx, fragment_count, rng_roll);
                let frag_damage = outer_armor_damage * frac;
                let fragment_id = format!("{actor:?}:spall:{frag_idx}", actor = actor.0);
                let frag_spawn_id = self.recorder.record(
                    tick,
                    sim_time_ms,
                    "armor",
                    "spalling_fragment_spawned",
                    json!({
                        "fragment_id": fragment_id,
                        "direction": ray_direction,
                        "damage": frag_damage,
                        "source_event_id": spalling_id.clone(),
                    }),
                    Some(spalling_id.clone()),
                );
                // Hit the next 1-2 modules along the ray + apply damage.
                // **M14 audit pass 3 (Finding 6)**: spalling fragments now
                // mutate module HP. Previous implementation emitted the
                // hit event but never reduced module.hp.
                // **M14 audit pass 4 (Finding 3)**: also emit
                // module_state_changed + cascade events when the spalling
                // hit pushes the module across a state threshold.
                for hit_m in interior_modules.iter().take(2) {
                    let frag_hit_damage = frag_damage * 0.5;
                    let outcome_opt = if let Ok(mut s) = self.state.write() {
                        s.actor_state.as_mut().and_then(|sim| {
                            sim.world.actors.get_mut(&actor).and_then(|target_actor| {
                                target_actor.chassis.as_mut().and_then(|chassis| {
                                    chassis.apply_critical_module_damage(
                                        &hit_m.id,
                                        frag_hit_damage,
                                        "spalling_fragment",
                                    )
                                })
                            })
                        })
                    } else {
                        None
                    };
                    let frag_hit_event_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "armor",
                        "spalling_fragment_hit_module",
                        json!({
                            "fragment_id": fragment_id,
                            "module_id": hit_m.id,
                            "damage": frag_hit_damage,
                            "source_event_id": frag_spawn_id.clone(),
                        }),
                        Some(frag_spawn_id.clone()),
                    );
                    if let Some(outcome) = outcome_opt {
                        self.emit_critical_module_outcome_events(
                            tick,
                            sim_time_ms,
                            actor,
                            &outcome,
                            Some(frag_hit_event_id),
                        );
                    }
                }
            }
        }
    }

    /// **M14C** § per-hit producer wiring for HEAT + APFSDS rounds.
    ///
    /// When the projectile's `RoundKind` is `Heat`, this helper invokes
    /// [`cf_physics::heat_impact_producer`] against the target's chassis
    /// modules + any ERA panel on the path, emitting in strict order:
    /// `armor.era_pre_detonated` (when an ERA panel is on the path, per
    /// VAL-M14C-009) THEN `armor.heat_jet_traversed` (the per-module
    /// HEAT path, per VAL-M14C-007/010/021).
    ///
    /// When the round is `Apfsds`, this helper invokes
    /// [`cf_physics::apfsds_impact_producer`] and emits
    /// `armor.apfsds_long_rod_through` with per-module energy decay
    /// entries (per VAL-M14C-008/012).
    ///
    /// For other round kinds this helper is a no-op (regular / tracer /
    /// pellet / high_explosive fall back to the M14 traversal path).
    ///
    /// **Side effect**: an ERA pre-detonation against HEAT consumes the
    /// matched ERA panel via [`cf_chassis::ChassisModule::consume_era_panel`]
    /// so a second HEAT impact on the same panel does not re-trigger
    /// pre-detonation (per VAL-M14C-002 one-shot rule).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn emit_m14c_armor_events(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        actor: ActorId,
        hit: &cf_actor::sim::HitOutcome,
        round_kind: cf_equipment::RoundKind,
        parent: Option<String>,
    ) {
        if !matches!(
            round_kind,
            cf_equipment::RoundKind::Heat | cf_equipment::RoundKind::Apfsds
        ) {
            return;
        }
        // Collect chassis interior modules in distance-from-impact order +
        // the first ERA panel id on the path (for HEAT consume).
        type ChassisSnapshot = (Vec<cf_physics::InteriorModule>, Option<(String, f32)>);
        let chassis_snapshot: Option<ChassisSnapshot> = self
            .state
            .read()
            .ok()
            .and_then(|s| s.actor_state.as_ref().map(|sim| sim.world.actors.clone()))
            .and_then(|actors| {
                actors.get(&actor).and_then(|a| {
                    a.chassis.as_ref().map(|chassis| {
                        let position = a.position;
                        let mut entries: Vec<cf_physics::InteriorModule> = Vec::new();
                        let mut era_on_path: Option<(String, f32)> = None;
                        for m in &chassis.modules {
                            let centre = if m.local_aabb.is_positioned() {
                                let aabb = &m.local_aabb;
                                [
                                    position.x + (aabb.min_x + aabb.max_x) * 0.5,
                                    position.y + (aabb.min_y + aabb.max_y) * 0.5,
                                ]
                            } else {
                                [position.x, position.y]
                            };
                            let dx = centre[0] - hit.hit_position.x;
                            let dy = centre[1] - hit.hit_position.y;
                            let dist = (dx * dx + dy * dy).sqrt();
                            let is_ammo_rack = matches!(m.kind, cf_chassis::ModuleKind::AmmoRack);
                            if matches!(m.kind, cf_chassis::ModuleKind::Era)
                                && m.era_consumable
                                && era_on_path.is_none()
                            {
                                era_on_path = Some((m.id.clone(), m.era_charge_kg.max(0.0)));
                            }
                            entries.push(cf_physics::InteriorModule {
                                id: m.id.clone(),
                                damage_multiplier: 0.6,
                                armor_absorption: (1.0_f32 - (m.hp / m.hp_max.max(0.001)).clamp(0.0, 1.0))
                                    .clamp(0.0, 0.9),
                                position: centre,
                                distance_along_ray: dist,
                                is_ammo_rack,
                            });
                        }
                        entries.sort_by(|a, b| {
                            a.distance_along_ray
                                .partial_cmp(&b.distance_along_ray)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        (entries, era_on_path)
                    })
                })
            });
        let (interior_modules, era_on_path) = match chassis_snapshot {
            Some(snap) => snap,
            None => return,
        };
        if interior_modules.is_empty() {
            return;
        }

        match round_kind {
            cf_equipment::RoundKind::Heat => {
                // Spec § "Notes for the implementer": 10 MJ jet @ ~3 km/s,
                // 5° cone half-angle, 0.6 m optimum standoff, ~0.2 m min
                // standoff for jet formation. Impact angle is 0 here —
                // the engine routes the swept ray through the producer
                // before the off-axis glance gate so on-target HEAT
                // impacts always traverse (off-axis glance is handled by
                // the M14 baseline path).
                let input = cf_physics::HeatImpactInput {
                    actor_id: actor.0,
                    charge_mass_kg: 1.0,
                    jet_velocity_mps: 3000.0,
                    cone_half_angle_deg: 5.0,
                    optimum_standoff_m: 0.6,
                    min_jet_formation_standoff_m: 0.2,
                    standoff_m: 0.6,
                    impact_angle_deg: 0.0,
                    modules: interior_modules,
                    era_charge_kg: era_on_path.as_ref().map(|(_, kg)| *kg),
                };
                let outcome = cf_physics::heat_impact_producer(&input);

                // Step 1: emit armor.era_pre_detonated FIRST (strict
                // ordering per VAL-M14C-009).
                let era_parent = parent.clone();
                let mut last_parent = parent;
                if let Some(era_event) = outcome.era_event.as_ref() {
                    let era_recorded_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "armor",
                        "era_pre_detonated",
                        json!({
                            "actor_id": era_event.actor_id,
                            "module_id": era_event.module_id,
                            "era_charge_kg": era_event.era_charge_kg,
                            "penetration_reduction": era_event.penetration_reduction,
                        }),
                        era_parent,
                    );
                    // Consume the ERA panel (one-shot) so a second HEAT
                    // hit on the same panel does NOT re-pre-detonate.
                    if let Some((era_id, _)) = era_on_path.as_ref() {
                        if let Ok(mut s) = self.state.write() {
                            if let Some(sim) = s.actor_state.as_mut() {
                                if let Some(target) = sim.world.actors.get_mut(&actor) {
                                    if let Some(chassis) = target.chassis.as_mut() {
                                        for m in &mut chassis.modules {
                                            if &m.id == era_id {
                                                let _ = m.consume_era_panel();
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    last_parent = Some(era_recorded_id);
                }

                // Step 2: emit armor.heat_jet_traversed.
                if let Some(traversed) = outcome.traversed.as_ref() {
                    let path_payload: Vec<serde_json::Value> = traversed
                        .path
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "module_id": p.module_id,
                                "depth_mm": p.depth_mm,
                                "damage": p.damage,
                            })
                        })
                        .collect();
                    let heat_event_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "armor",
                        "heat_jet_traversed",
                        json!({
                            "actor_id": traversed.actor_id,
                            "modules": traversed.modules,
                            "path": path_payload,
                            "effective_damage": traversed.effective_damage,
                            "standoff_m": traversed.standoff_m,
                            "impact_angle_deg": traversed.impact_angle_deg,
                        }),
                        last_parent.clone(),
                    );
                    // **M14G § VAL-CROSS-001 / VAL-M14G-022**: emit the
                    // typed wound cluster from the HEAT jet — one Burn3rd
                    // per traversed module + one GunshotThrough on the
                    // crew compartment when the jet reaches it. Per
                    // VAL-CROSS-021 the cluster size scales with the
                    // module path length (sub-optimal standoff shrinks
                    // the path → cluster shrinks accordingly).
                    let module_zones: Vec<cf_wound::registry::ZoneId> = traversed
                        .modules
                        .iter()
                        .map(|m| cf_wound::registry::ZoneId::from(m.as_str()))
                        .collect();
                    // The HEAT jet reaches the crew compartment whenever
                    // it penetrates the armor and produces a non-empty
                    // module path. Under sub-optimal standoff the
                    // producer trims the path; an empty path => no
                    // GunshotThrough emission (VAL-CROSS-021).
                    let crew_torso = if module_zones.is_empty() {
                        None
                    } else {
                        Some(cf_wound::registry::ZoneId::from("crew_torso"))
                    };
                    let emits = cf_physics::classify_heat_cluster(
                        &module_zones,
                        crew_torso,
                        traversed.effective_damage.clamp(0.05, 1.0),
                    );
                    let parent_id = Some(heat_event_id);
                    for emit in emits {
                        let _ = self.m14g_emit_wound_created(
                            tick,
                            sim_time_ms,
                            actor.0,
                            emit,
                            parent_id.clone(),
                        );
                    }
                }
            }
            cf_equipment::RoundKind::Apfsds => {
                // Spec § "Notes for the implementer": 7 kg DU rod @
                // 1600 m/s = 9.0 MJ KE; per-module energy decay =
                // KE_in × (1 - absorption_ratio). APFSDS ignores ERA
                // (VAL-M14C-024) so `era_charge_kg` is never read.
                let input = cf_physics::ApfsdsImpactInput {
                    actor_id: actor.0,
                    rod_mass_kg: 7.0,
                    velocity_mps: 1600.0,
                    modules: interior_modules,
                };
                let outcome = cf_physics::apfsds_impact_producer(&input);
                if let Some(ev) = outcome.event.as_ref() {
                    let path_payload: Vec<serde_json::Value> = ev
                        .path
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "module_id": p.module_id,
                                "energy_absorbed_j": p.energy_absorbed_j,
                                "energy_remaining_j": p.energy_remaining_j,
                                "depth_mm": p.depth_mm,
                            })
                        })
                        .collect();
                    let apfsds_event_id = self.recorder.record(
                        tick,
                        sim_time_ms,
                        "armor",
                        "apfsds_long_rod_through",
                        json!({
                            "actor_id": ev.actor_id,
                            "path": path_payload,
                            "initial_energy_j": ev.initial_energy_j,
                            "final_energy_j": ev.final_energy_j,
                        }),
                        parent.clone(),
                    );
                    // **M14G § VAL-CROSS-002**: APFSDS emits one
                    // `ShrapnelThrough` per traversed module + spalling
                    // fragments (one `ShrapnelEmbedded` per module). The
                    // shrapnel severity tracks the energy decay ratio.
                    let parent_id = Some(apfsds_event_id);
                    let initial = ev.initial_energy_j.max(1.0);
                    for p in &ev.path {
                        let zone = cf_wound::registry::ZoneId::from(p.module_id.as_str());
                        let severity_through =
                            (p.energy_remaining_j.max(0.0) / initial).clamp(0.05, 1.0);
                        let severity_embedded =
                            (p.energy_absorbed_j.max(0.0) / initial).clamp(0.05, 1.0);
                        let through_emit = cf_physics::classify_shrapnel(
                            zone.clone(),
                            severity_through,
                            true,
                        );
                        let _ = self.m14g_emit_wound_created(
                            tick,
                            sim_time_ms,
                            actor.0,
                            through_emit,
                            parent_id.clone(),
                        );
                        let embedded_emit =
                            cf_physics::classify_shrapnel(zone, severity_embedded, false);
                        let _ = self.m14g_emit_wound_created(
                            tick,
                            sim_time_ms,
                            actor.0,
                            embedded_emit,
                            parent_id.clone(),
                        );
                    }
                }
            }
            _ => {}
        }
    }

}
