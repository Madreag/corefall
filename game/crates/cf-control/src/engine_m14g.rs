//! M14G thermal + material contacts + wound-aging methods.
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
    pub(crate) fn tick_m14g_thermal_contacts(&self, tick: Tick, sim_time_ms: f64) {
        enum ThermalDelta {
            Created(cf_environment::ThermalWoundEmit),
            Escalated {
                old_kind: cf_wound::WoundKind,
                emit: cf_environment::ThermalWoundEmit,
            },
        }
        let mut to_emit: Vec<(u64, ThermalDelta)> = Vec::new();
        if let Ok(mut s) = self.state.write() {
            let zones = s.m14g_thermal_zones.clone();
            for z in &zones {
                if tick.0 < z.start_tick {
                    continue;
                }
                if let Some(end) = z.end_tick {
                    if tick.0 > end {
                        continue;
                    }
                }
                let key = (z.actor_id, z.zone.clone());
                let dwell = s
                    .m14g_thermal_dwell_ticks
                    .entry(key.clone())
                    .or_insert(0);
                *dwell = dwell.saturating_add(1);
                let dwell_now = *dwell;
                let zone_id = cf_wound::registry::ZoneId::from(z.zone.as_str());
                if let Some(emit) =
                    cf_environment::classify_tile_thermal(zone_id, z.temperature_k, dwell_now)
                {
                    let prev = s.m14g_thermal_emitted_kind.get(&key).copied();
                    if prev != Some(emit.kind) {
                        s.m14g_thermal_emitted_kind.insert(key.clone(), emit.kind);
                        match prev {
                            None => to_emit.push((z.actor_id, ThermalDelta::Created(emit))),
                            Some(old_kind) => to_emit.push((
                                z.actor_id,
                                ThermalDelta::Escalated { old_kind, emit },
                            )),
                        }
                    }
                }
            }
        }
        for (actor_id, delta) in to_emit {
            match delta {
                ThermalDelta::Created(emit) => {
                    let m14g_emit = cf_physics::M14gWoundEmit {
                        kind: emit.kind,
                        severity: emit.severity,
                        zone: emit.zone,
                        dirt_pct: 0.0,
                    };
                    let _ = self
                        .m14g_emit_wound_created(tick, sim_time_ms, actor_id, m14g_emit, None);
                }
                ThermalDelta::Escalated { old_kind, emit } => {
                    let _ = self.m14g_emit_wound_escalated(
                        tick,
                        sim_time_ms,
                        actor_id,
                        old_kind,
                        emit.kind,
                        emit.severity,
                        emit.zone,
                        None,
                    );
                }
            }
        }
    }

    /// each scenario-authored material contact once at its `fire_tick`
    /// via [`cf_material::classify_reaction`].
    pub(crate) fn tick_m14g_material_contacts(&self, tick: Tick, sim_time_ms: f64) {
        let mut to_emit: Vec<(u64, cf_material::ReactionWoundEmit)> = Vec::new();
        if let Ok(mut s) = self.state.write() {
            let contacts = s.m14g_material_contacts.clone();
            for (idx, c) in contacts.iter().enumerate() {
                if tick.0 != c.fire_tick {
                    continue;
                }
                if s.m14g_material_contacts_fired.contains(&idx) {
                    continue;
                }
                let zone_id = cf_wound::registry::ZoneId::from(c.zone.as_str());
                if let Some(emit) =
                    cf_material::classify_reaction(&c.material, zone_id, c.intensity)
                {
                    s.m14g_material_contacts_fired.insert(idx);
                    to_emit.push((c.actor_id, emit));
                }
            }
        }
        for (actor_id, emit) in to_emit {
            let m14g_emit = cf_physics::M14gWoundEmit {
                kind: emit.kind,
                severity: emit.severity,
                zone: emit.zone,
                dirt_pct: 0.0,
            };
            let _ = self.m14g_emit_wound_created(tick, sim_time_ms, actor_id, m14g_emit, None);
        }
    }

    /// wound aging pass. Increments `age_ticks` every tick + commits
    /// visible-state mutations every 5 ticks. Emits `wound.aged` /
    /// `wound.scabbed` / `wound.scarred` events. Does NOT roll
    /// infection chance (deferred to M14H — VAL-M14G-047).
    pub(crate) fn tick_m14g_wound_aging(&self, tick: Tick, sim_time_ms: f64) {
        use cf_wound::aging::{AgingEvent, AgingNewState};
        let mut emitted_events: Vec<serde_json::Value> = Vec::new();
        if let Ok(mut s) = self.state.write() {
            s.m14g_wound_aging_invocations =
                s.m14g_wound_aging_invocations.saturating_add(1);
            if s.m14g_wound_registry.is_none() {
                s.m14g_wound_registry = Some(cf_wound::WoundSpecRegistry::baked_default());
            }
            let tick_rate = s.clock.config().tick_rate_hz;
            // Pull the registry out of the option so the borrow on actors
            // can run without re-borrowing the mutable option.
            let registry = s.m14g_wound_registry.clone().unwrap_or_default();
            if let Some(sim) = s.actor_state.as_mut() {
                for (actor_id, actor) in sim.world.actors.iter_mut() {
                    let events = cf_wound::aging::aging_tick_pass(
                        &mut actor.m14g_wound_list,
                        &registry,
                        tick.0,
                        tick_rate,
                    );
                    for ev in events {
                        match ev {
                            AgingEvent::Aged { wound_id, zone, new_state } => {
                                emitted_events.push(serde_json::json!({
                                    "category": "wound",
                                    "event_type": "aged",
                                    "actor_id": actor_id.0,
                                    "tick": tick.0,
                                    "zone": zone.as_str(),
                                    "wound_id": wound_id.0,
                                    "new_state": new_state.as_str(),
                                }));
                                let _ = AgingNewState::BandageSoaked;
                            }
                            AgingEvent::Scabbed { wound_id, zone, kind } => {
                                emitted_events.push(serde_json::json!({
                                    "category": "wound",
                                    "event_type": "scabbed",
                                    "actor_id": actor_id.0,
                                    "tick": tick.0,
                                    "zone": zone.as_str(),
                                    "wound_id": wound_id.0,
                                    "kind": kind.as_str(),
                                }));
                            }
                            AgingEvent::Scarred { wound_id, zone, kind } => {
                                emitted_events.push(serde_json::json!({
                                    "category": "wound",
                                    "event_type": "scarred",
                                    "actor_id": actor_id.0,
                                    "tick": tick.0,
                                    "zone": zone.as_str(),
                                    "wound_id": wound_id.0,
                                    "kind": kind.as_str(),
                                }));
                            }
                        }
                    }
                }
            }
        }
        for ev in emitted_events {
            let cat = ev["category"].as_str().unwrap_or("wound").to_string();
            let ty = ev["event_type"].as_str().unwrap_or("aged").to_string();
            self.recorder.record(
                tick,
                sim_time_ms,
                &cat,
                &ty,
                ev,
                None,
            );
        }
    }

    ///
    /// Pushes a typed wound onto the actor's `m14g_wound_list` and emits a
    /// `wound.created` event with the canonical payload (actor_id, tick,
    /// wound_id, kind, zone, severity, dirt_pct). Honors per-origin
    /// substitution through the loaded `WoundSpecRegistry` so robot actors
    /// receive `CrushLimb` in place of `LacerationLight`, etc. (VAL-M14G-021).
    ///
    /// Returns the recorded event id on success so callers can stitch
    /// `parent_event_id` chains, or `None` when the actor has no entry in
    /// `actor_state.world.actors` (e.g., torus/static targets without a
    /// chassis component).
    pub(crate) fn m14g_emit_wound_created(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        actor_id: u64,
        emit: cf_physics::M14gWoundEmit,
        parent: Option<String>,
    ) -> Option<String> {
        let actor_key = cf_actor::ActorId(actor_id);
        let mut s = self.state.write().ok()?;
        if s.m14g_wound_registry.is_none() {
            s.m14g_wound_registry = Some(cf_wound::WoundSpecRegistry::baked_default());
        }
        let origin_id = s
            .actor_state
            .as_ref()
            .and_then(|sim| sim.world.actors.get(&actor_key).map(|a| a.origin_id.clone()))
            .unwrap_or_default();
        let registry = s.m14g_wound_registry.clone().unwrap_or_default();
        let origin = cf_wound::registry::OriginId::from(origin_id.as_str());
        let resolved =
            match cf_physics::substitute_for_origin(&registry, emit.clone(), &origin) {
                Some(e) => e,
                None => emit,
            };
        let sim = s.actor_state.as_mut()?;
        let actor = sim.world.actors.get_mut(&actor_key)?;
        let wound_id = actor.m14g_wound_list.alloc_id();
        let wound = {
            let mut w = cf_wound::Wound::new(
                wound_id,
                resolved.kind,
                resolved.severity,
                resolved.zone.clone(),
            );
            w.dirt_pct = resolved.dirt_pct.clamp(0.0, 1.0);
            w
        };
        let kind_name = wound.kind.as_str();
        let severity = wound.severity;
        let dirt_pct = wound.dirt_pct;
        let zone_name = wound.zone.as_str().to_string();
        let wound_kind = wound.kind;
        actor.m14g_wound_list.push(resolved.zone.clone(), wound);
        drop(s);
        let payload = serde_json::json!({
            "actor_id": actor_id,
            "tick": tick.0,
            "wound_id": wound_id.0,
            "kind": kind_name,
            "zone": zone_name,
            "severity": severity,
            "dirt_pct": dirt_pct,
        });
        let event_id = self
            .recorder
            .record(tick, sim_time_ms, "wound", "created", payload, parent);
        // increment the actor's concussion_count + may emit
        // memory_loss.minor/major.
        if matches!(wound_kind, cf_wound::WoundKind::Concussion) && severity >= 0.5 {
            self.m14i_record_concussion(actor_id, tick, sim_time_ms);
        }
        Some(event_id)
    }

    /// existing wound on `(actor_id, zone)` from `old_kind` to
    /// `new_kind` and emit a `wound.escalated` event. Used by the
    /// per-tick thermal pass when a sustained burn / frostbite contact
    /// crosses the next tier threshold. The existing wound in the
    /// `ActorWoundList` is mutated in place (kind + severity updated)
    /// so the actor's wound count stays the same — the spec mandates
    /// escalation upgrades the wound, not a fresh emission.
    pub(crate) fn m14g_emit_wound_escalated(
        &self,
        tick: Tick,
        sim_time_ms: f64,
        actor_id: u64,
        old_kind: cf_wound::WoundKind,
        new_kind: cf_wound::WoundKind,
        new_severity: f32,
        zone: cf_wound::registry::ZoneId,
        parent: Option<String>,
    ) -> Option<String> {
        let actor_key = cf_actor::ActorId(actor_id);
        let mut s = self.state.write().ok()?;
        let sim = s.actor_state.as_mut()?;
        let actor = sim.world.actors.get_mut(&actor_key)?;
        let mut upgraded_wound_id: Option<cf_wound::WoundId> = None;
        let mut old_severity: f32 = 0.0;
        if let Some(wounds) = actor.m14g_wound_list.zone_mut(&zone) {
            if let Some(w) = wounds.iter_mut().find(|w| w.kind == old_kind) {
                old_severity = w.severity;
                w.kind = new_kind;
                w.severity = new_severity.clamp(0.0, 1.0);
                upgraded_wound_id = Some(w.id);
            }
        }
        drop(s);
        let wound_id = upgraded_wound_id?;
        let payload = serde_json::json!({
            "actor_id": actor_id,
            "tick": tick.0,
            "wound_id": wound_id.0,
            "zone": zone.as_str(),
            "old_kind": old_kind.as_str(),
            "new_kind": new_kind.as_str(),
            "old_severity": old_severity,
            "new_severity": new_severity.clamp(0.0, 1.0),
        });
        Some(self
            .recorder
            .record(tick, sim_time_ms, "wound", "escalated", payload, parent))
    }

}
