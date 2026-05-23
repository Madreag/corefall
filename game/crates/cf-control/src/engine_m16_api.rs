//! M16 § Engine API for spawning hazards / anomalies / artifacts +
//! observing the M16 worlds from cfctl + acceptance tests.

use cf_actor::ActorId;
use cf_affliction::{self as affl, ClearReason, M16AfflictionKind, M16TriggerThresholds};
use cf_anomaly::AnomalyKind;
use cf_artifact::ArtifactInstanceId;
use cf_hazard::{HazardId, HazardKind};
use cf_sim_core::Tick;

use crate::engine::M0Engine;
use crate::m16_tick;

impl M0Engine {
    /// Spawn a hazard tile. Returns the assigned id + emits hazard.spawned.
    pub fn m16_spawn_hazard(
        &self,
        kind: HazardKind,
        position: [f32; 2],
        intensity: f32,
        source_event_id: Option<String>,
    ) -> HazardId {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let (id, ev) = state
            .m16_hazard_world
            .spawn(kind, position, intensity, tick.0, source_event_id.clone());
        let _ = self.recorder.record(
            tick,
            sim_time_ms,
            "hazard",
            "spawned",
            serde_json::json!({
                "hazard_id": ev.hazard_id.to_string(),
                "kind": ev.kind.as_str(),
                "position": ev.position,
                "intensity": ev.intensity,
                "source_event_id": ev.source_event_id.clone().unwrap_or_default(),
            }),
            ev.source_event_id,
        );
        id
    }

    /// Spawn an anomaly zone. Returns the assigned id.
    pub fn m16_spawn_anomaly(&self, kind: AnomalyKind, position: [f32; 2]) -> u64 {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = state.clock.tick().0;
        state.m16_anomaly_world.spawn(kind, position, tick)
    }

    /// Spawn an artifact at `position` + emit artifact.spawned.
    pub fn m16_spawn_artifact(&self, spec_id: &str, position: [f32; 2]) -> Option<ArtifactInstanceId> {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let registry = state.m16_artifact_registry.clone();
        m16_tick::spawn_artifact_with_event(
            &mut state.m16_artifact_world,
            &registry,
            spec_id,
            position,
            tick,
            sim_time_ms,
            None,
            &self.recorder,
        )
    }

    /// Pick up an artifact. Emits artifact.picked_up + artifact.carried_bonus_applied.
    pub fn m16_pickup_artifact(&self, instance_id: ArtifactInstanceId, actor_id: u64) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let registry = state.m16_artifact_registry.clone();
        m16_tick::pickup_artifact_with_events(
            &mut state.m16_artifact_world,
            &registry,
            instance_id,
            actor_id,
            tick,
            sim_time_ms,
            &self.recorder,
        )
    }

    /// Apply a counter (water on fire, alkali on acid) at `position`.
    /// Returns the number of hazards doused.
    pub fn m16_apply_counter(&self, kind: HazardKind, position: [f32; 2], radius_tiles: f32) -> u32 {
        let mut state = self.state.write().expect("engine state poisoned");
        m16_tick::apply_hazard_counter(&mut state.m16_hazard_world, kind, position, radius_tiles)
    }

    /// Clear an affliction (medikit/environment/death).
    pub fn m16_clear_affliction(
        &self,
        actor_id: u64,
        kind: M16AfflictionKind,
        reason: ClearReason,
    ) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        m16_tick::clear_affliction_with_event(
            &mut state.m16_affliction_by_actor,
            ActorId(actor_id),
            kind,
            reason,
            tick,
            sim_time_ms,
            &self.recorder,
        )
    }

    /// Set the PvE Survival mode flag. Survival afflictions (hunger /
    /// thirst / sleep_dep / sanity_low) are only active when this is true.
    pub fn m16_set_survival_mode(&self, active: bool) {
        let mut state = self.state.write().expect("engine state poisoned");
        state.m16_survival_mode_active = active;
    }

    /// Snapshot the M16 hazard grid for `observe.frame` + `snapshot.snapshot_hazard_grid`.
    pub fn m16_hazard_snapshot(&self) -> cf_hazard::HazardGridSnapshot {
        let state = self.state.read().expect("engine state poisoned");
        state.m16_hazard_world.snapshot()
    }

    /// Return the aggregate artifact bonus carried by `actor_id`.
    pub fn m16_actor_artifact_bonus(&self, actor_id: u64) -> cf_artifact::ArtifactBonus {
        let state = self.state.read().expect("engine state poisoned");
        state
            .m16_artifact_world
            .aggregate_bonus_for_actor(&state.m16_artifact_registry, actor_id)
    }

    /// Return the active afflictions for `actor_id` (kind + severity).
    pub fn m16_actor_afflictions(&self, actor_id: u64) -> Vec<(String, f32)> {
        let state = self.state.read().expect("engine state poisoned");
        let entry = state
            .m16_affliction_by_actor
            .get(&ActorId(actor_id));
        entry
            .map(|a| {
                a.active
                    .iter()
                    .map(|x| (x.kind.as_str().to_string(), x.severity))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// M16A § Return the per-actor env affliction snapshot — kind +
    /// severity_0_1 + accumulator_value + severity_band for all 11 env
    /// kinds whose state is active. Powers `cfctl
    /// query.actor.affliction_state`.
    pub fn m16a_actor_env_state(
        &self,
        actor_id: u64,
    ) -> Vec<(String, f32, f32, &'static str)> {
        let state = self.state.read().expect("engine state poisoned");
        let entry = state.m16a_env_state_by_actor.get(&ActorId(actor_id));
        let mut out: Vec<(String, f32, f32, &'static str)> = Vec::new();
        let entry = match entry {
            Some(e) => e,
            None => return out,
        };
        for k in cf_affliction::EnvAfflictionKind::all() {
            let sev = entry.severity(*k);
            let acc = entry.accumulator(*k).kind_value;
            if sev > 0.0 || acc > 0.0 {
                let band = cf_affliction::EnvSeverity::from_severity_0_1(sev);
                out.push((k.as_str().to_string(), sev, acc, band.as_str()));
            }
        }
        out
    }

    /// Apply a hazard contact directly (used by acceptance tests + cfctl).
    pub fn m16_apply_affliction(
        &self,
        actor_id: u64,
        kind: M16AfflictionKind,
        severity: f32,
        source: &str,
    ) -> bool {
        let mut state = self.state.write().expect("engine state poisoned");
        let tick = Tick(state.clock.tick().0);
        let sim_time_ms = state.clock.sim_time_ms();
        let tick_rate = self.config.tick_rate_hz;
        let registry = state.m16_affliction_registry.clone();
        let actor_aff = state
            .m16_affliction_by_actor
            .entry(ActorId(actor_id))
            .or_default();
        let (applied, escalated) = affl::apply_affliction(
            actor_aff,
            actor_id,
            kind,
            severity,
            &registry,
            tick.0,
            tick_rate,
            source.to_string(),
        );
        if let Some(ev) = applied {
            let _ = self.recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "applied",
                serde_json::json!({
                    "actor_id": ev.actor_id,
                    "kind": kind.as_str(),
                    "source_event_id": ev.source_event_id,
                    "expected_duration_ticks": ev.expected_duration_ticks,
                    "severity_0_1": ev.severity,
                }),
                None,
            );
            return true;
        }
        if let Some(ev) = escalated {
            let _ = self.recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "escalated",
                serde_json::json!({
                    "actor_id": ev.actor_id,
                    "kind": kind.as_str(),
                    "from_severity": ev.from_severity,
                    "to_severity": ev.to_severity,
                }),
                None,
            );
            return true;
        }
        false
    }

    /// Return the swim state for an actor (oxygen / submersion).
    pub fn m16_actor_swim_state(&self, actor_id: u64) -> Option<cf_swim::SwimState> {
        let state = self.state.read().expect("engine state poisoned");
        state.m16_swim_world.by_actor.get(&actor_id).cloned()
    }

    /// Detector query — return nearby anomalies (id, kind string, world pos,
    /// detector_required).
    pub fn m16_detector_query(
        &self,
        position: [f32; 2],
        radius_m: f32,
    ) -> Vec<(u64, String, [f32; 2], bool)> {
        let state = self.state.read().expect("engine state poisoned");
        let zones = state.m16_anomaly_world.detector_query(position, radius_m);
        zones
            .into_iter()
            .map(|(id, kind, pos)| {
                let spec = state.m16_anomaly_registry.lookup(kind);
                (id, kind.as_str().to_string(), pos, spec.detector_required)
            })
            .collect()
    }

    /// Set the per-actor auto-triage trigger thresholds per spec § "Player
    /// can edit affliction trigger thresholds per actor".
    pub fn m16_set_trigger_thresholds(&self, actor_id: u64, thresholds: M16TriggerThresholds) {
        let mut state = self.state.write().expect("engine state poisoned");
        state.m16_trigger_thresholds.insert(ActorId(actor_id), thresholds);
    }

    /// Returns the active per-actor trigger thresholds (default when
    /// unset).
    pub fn m16_trigger_thresholds_for(&self, actor_id: u64) -> M16TriggerThresholds {
        let state = self.state.read().expect("engine state poisoned");
        state
            .m16_trigger_thresholds
            .get(&ActorId(actor_id))
            .copied()
            .unwrap_or_default()
    }

    /// Read the M16 storyteller registry. Surfaced for cfctl / replay
    /// debug + cf-mod validation.
    pub fn m16_storyteller_event_ids(&self) -> Vec<String> {
        let state = self.state.read().expect("engine state poisoned");
        state
            .m16_storyteller_registry
            .by_id
            .keys()
            .cloned()
            .collect()
    }

    /// True when `actor_id` is carrying the `anomaly_detector` item in
    /// inventory. Used by the HUD minimap to gate detector-required
    /// anomaly surfacing.
    pub fn m16_actor_has_anomaly_detector(&self, actor_id: u64) -> bool {
        let state = self.state.read().expect("engine state poisoned");
        let sim = match state.actor_state.as_ref() {
            Some(s) => s,
            None => return false,
        };
        let actor = match sim.world.actors.get(&ActorId(actor_id)) {
            Some(a) => a,
            None => return false,
        };
        if let Some(grid) = &actor.inventory_grid {
            if inventory_grid_contains_id(grid, cf_equipment::sensor::ANOMALY_DETECTOR_ID) {
                return true;
            }
        }
        false
    }
}

fn inventory_grid_contains_id(grid: &cf_actor::inventory::InventoryGrid, id: &str) -> bool {
    fn walk(items: &[cf_actor::inventory::PlacedItem], id: &str) -> bool {
        for item in items {
            if item.item_id == id {
                return true;
            }
            if let Some(c) = &item.container {
                if walk(&c.items, id) {
                    return true;
                }
            }
        }
        false
    }
    walk(&grid.items, id)
}
