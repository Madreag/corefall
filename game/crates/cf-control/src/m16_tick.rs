//! M16 § Hazard + Anomaly + Artifact + Swim + Affliction per-tick orchestrator.
//!
//! This module is called from `drive_tick` AFTER the actor sim has stepped
//! so positions are stable for this tick. It batches the M16 worlds'
//! `tick` calls and returns a [`M16TickOutput`] which the engine drains
//! into the recorder using the M16 schemas:
//!   - hazard.spawned / spread / actor_contact / tick / dissipated
//!   - affliction.applied / escalated / cleared / tick
//!   - anomaly.entered / damage_applied
//!   - artifact.spawned / picked_up / carried_bonus_applied
//!   - actor.swim_started / swim_ended / drowning_started / drowning_lethal

use serde_json::json;

use cf_actor::ActorId;
use cf_actor::sim::ActorSimState;
use cf_replay::Recorder;
use cf_sim_core::Tick;

use cf_affliction::{
    self as affl, ActorAfflictions, AfflictionRegistry, AtmosphericSusceptibility, AutoTriageReason,
    ClearReason, EnvAfflictionKind, EnvAfflictionRegistry, EnvAfflictionState, EnvClearReason,
    EnvSeverity, EnvSignal, M16AfflictionKind, M16TriggerThresholds, OriginId,
};
use cf_anomaly::{AnomalyRegistry, AnomalyTickOutput, AnomalyWorld};
use cf_artifact::{
    ArtifactCarriedBonusEvent, ArtifactPickedUpEvent, ArtifactRegistry, ArtifactSpawnedEvent, ArtifactWorld,
};
use cf_hazard::{
    HazardActorContactEvent, HazardRegistry, HazardSpawnedEvent, HazardTickOutput, HazardWorld,
};
use cf_swim::{SwimTickEvents, SwimTickInput, SwimWorld};

pub struct M16TickInputs<'a> {
    pub tick: Tick,
    pub sim_time_ms: f64,
    pub tick_rate_hz: u32,
    pub actor_state: &'a ActorSimState,
    pub survival_mode_active: bool,
    pub recorder: &'a Recorder,
    /// Optional terrain reference for material-affordance-aware hazard
    /// spread (fire → flammable, electric → conductive/water). When
    /// `None`, the legacy unconstrained spread is used.
    pub terrain: Option<&'a cf_terrain::chunked::ChunkedTerrain>,
}

pub struct M16TickStateMut<'a> {
    pub hazard_world: &'a mut HazardWorld,
    pub hazard_registry: &'a HazardRegistry,
    pub anomaly_world: &'a mut AnomalyWorld,
    pub anomaly_registry: &'a AnomalyRegistry,
    pub artifact_world: &'a mut ArtifactWorld,
    pub artifact_registry: &'a ArtifactRegistry,
    pub swim_world: &'a mut SwimWorld,
    pub affliction_by_actor: &'a mut std::collections::BTreeMap<ActorId, ActorAfflictions>,
    pub affliction_registry: &'a AfflictionRegistry,
    pub trigger_thresholds: &'a std::collections::BTreeMap<ActorId, M16TriggerThresholds>,
    pub last_auto_triage_reason: &'a mut std::collections::BTreeMap<ActorId, AutoTriageReason>,
    pub env_state_by_actor: &'a mut std::collections::BTreeMap<ActorId, EnvAfflictionState>,
    pub env_registry: &'a EnvAfflictionRegistry,
}

/// Aggregate output from one tick — currently used for assertions in
/// tests but the engine also returns the totals to surface in
/// `observe.frame` for the HUD bridge.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct M16TickOutput {
    pub hazards_spawned: u32,
    pub hazards_spread: u32,
    pub hazards_dissipated: u32,
    pub hazard_actor_contacts: u32,
    pub anomaly_entered_events: u32,
    pub anomaly_damage_events: u32,
    pub artifacts_picked_up: u32,
    pub afflictions_applied: u32,
    pub afflictions_escalated: u32,
    pub afflictions_cleared: u32,
    /// M16C — Pain stack recomputes that changed (emitted pain.stack_changed).
    pub pain_recomputed: u32,
    pub swim_started: u32,
    pub swim_ended: u32,
    pub drowning_started: u32,
    pub drowning_lethal: u32,
    pub auto_triage_reasons: Vec<(ActorId, AutoTriageReason)>,
    pub env_threshold_crossings: u32,
    pub env_severity_changes: u32,
    pub env_cleared: u32,
    pub env_origin_immune: u32,
}

/// Run one sim tick. Pure with respect to time — RNG-free.
pub fn run_m16_tick(inputs: M16TickInputs<'_>, state: M16TickStateMut<'_>) -> M16TickOutput {
    let M16TickInputs {
        tick,
        sim_time_ms,
        tick_rate_hz,
        actor_state,
        survival_mode_active,
        recorder,
        terrain,
    } = inputs;

    let mut out = M16TickOutput::default();

    // ----- 1) Hazard grid tick (spread + dissipation + cosmetic tick) -----
    let hazard_output = match terrain {
        Some(terrain_ref) => state.hazard_world.tick_grid_with_affordance(
            state.hazard_registry,
            tick.0,
            tick_rate_hz,
            |pos| {
                let mat_id = terrain_ref.material_at(pos[0] as i64, pos[1] as i64);
                let name = terrain_ref.registry.name(mat_id);
                tile_affordance_for_material_name(name)
            },
        ),
        None => state.hazard_world.tick_grid(state.hazard_registry, tick.0, tick_rate_hz),
    };
    let HazardTickOutput {
        spawned,
        spread,
        actor_contact: _,
        tick: cosmetic_tick,
        dissipated,
    } = hazard_output;
    out.hazards_spawned = spawned.len() as u32;
    out.hazards_spread = spread.len() as u32;
    out.hazards_dissipated = dissipated.len() as u32;
    for ev in &spawned {
        let _ = recorder.record(
            tick,
            sim_time_ms,
            "hazard",
            "spawned",
            json!({
                "hazard_id": ev.hazard_id.to_string(),
                "kind": ev.kind.as_str(),
                "position": ev.position,
                "intensity": ev.intensity,
                "source_event_id": ev.source_event_id.clone().unwrap_or_default(),
            }),
            ev.source_event_id.clone(),
        );
    }
    for ev in &spread {
        let _ = recorder.record(
            tick,
            sim_time_ms,
            "hazard",
            "spread",
            json!({
                "hazard_id": ev.hazard_id.to_string(),
                "kind": ev.kind.as_str(),
                "from_pos": ev.from_pos,
                "to_pos": ev.to_pos,
                "intensity": ev.intensity,
                "rate": ev.rate,
            }),
            None,
        );
    }
    for ev in cosmetic_tick.iter() {
        let _ = recorder.record_cosmetic(
            tick,
            sim_time_ms,
            "hazard",
            "tick",
            json!({
                "hazard_id": ev.hazard_id.to_string(),
                "tick": ev.tick,
            }),
            None,
        );
    }
    for ev in &dissipated {
        let _ = recorder.record(
            tick,
            sim_time_ms,
            "hazard",
            "dissipated",
            json!({
                "hazard_id": ev.hazard_id.to_string(),
                "reason": ev.reason.as_str(),
            }),
            None,
        );
    }

    // ----- 2) Hazard actor contact + affliction application -----
    let actors_list: Vec<(ActorId, [f32; 2])> = actor_state
        .world
        .actors
        .iter()
        .map(|(id, a)| (*id, [a.position.x, a.position.y]))
        .collect();
    let anomaly_actors_list: Vec<(u64, [f32; 2])> = actors_list
        .iter()
        .map(|(id, pos)| (id.0, *pos))
        .collect();
    for (actor_id, pos) in &actors_list {
        let contacts: Vec<HazardActorContactEvent> =
            state.hazard_world.resolve_actor_contacts(actor_id.0, *pos, 1.5);
        for c in &contacts {
            let contact_event_id = recorder.record(
                tick,
                sim_time_ms,
                "hazard",
                "actor_contact",
                json!({
                    "actor_id": c.actor_id,
                    "hazard_id": c.hazard_id.to_string(),
                    "kind": c.kind.as_str(),
                    "intensity": c.intensity,
                }),
                None,
            );
            out.hazard_actor_contacts += 1;
            let spec = state.hazard_registry.lookup(c.kind);
            if let Some(affliction_kind_str) = spec.on_contact_affliction.as_deref() {
                if let Some(kind) = M16AfflictionKind::from_str(affliction_kind_str) {
                    let actor_afflictions =
                        state.affliction_by_actor.entry(*actor_id).or_default();
                    let severity_to_add = c.intensity.clamp(0.05, 1.0);
                    let (applied, escalated) = affl::apply_affliction(
                        actor_afflictions,
                        actor_id.0,
                        kind,
                        severity_to_add,
                        state.affliction_registry,
                        tick.0,
                        tick_rate_hz,
                        contact_event_id.clone(),
                    );
                    if let Some(ev) = applied {
                        out.afflictions_applied += 1;
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "affliction",
                            "applied",
                            json!({
                                "actor_id": ev.actor_id,
                                "kind": kind.as_str(),
                                "source_event_id": ev.source_event_id,
                                "expected_duration_ticks": ev.expected_duration_ticks,
                                "severity_0_1": ev.severity,
                            }),
                            Some(contact_event_id.clone()),
                        );
                    }
                    if let Some(ev) = escalated {
                        out.afflictions_escalated += 1;
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "affliction",
                            "escalated",
                            json!({
                                "actor_id": ev.actor_id,
                                "kind": kind.as_str(),
                                "from_severity": ev.from_severity,
                                "to_severity": ev.to_severity,
                            }),
                            Some(contact_event_id.clone()),
                        );
                    }
                }
            }
        }
    }

    // ----- 3) Anomaly tick -----
    let AnomalyTickOutput { entered, damage } =
        state.anomaly_world.tick(state.anomaly_registry, tick.0, &anomaly_actors_list);
    out.anomaly_entered_events = entered.len() as u32;
    out.anomaly_damage_events = damage.len() as u32;
    for ev in &entered {
        let _ = recorder.record(
            tick,
            sim_time_ms,
            "anomaly",
            "entered",
            json!({
                "actor_id": ev.actor_id,
                "anomaly_id": ev.anomaly_id.to_string(),
                "kind": ev.kind.as_str(),
                "position": ev.position,
            }),
            None,
        );
    }
    for ev in &damage {
        let damage_event_id = recorder.record(
            tick,
            sim_time_ms,
            "anomaly",
            "damage_applied",
            json!({
                "actor_id": ev.actor_id,
                "anomaly_id": ev.anomaly_id.to_string(),
                "kind": ev.kind.as_str(),
                "damage": ev.damage,
                "applied_affliction": ev.applied_affliction,
            }),
            None,
        );
        if let Some(affliction_kind_str) = ev.applied_affliction.as_deref() {
            if let Some(kind) = M16AfflictionKind::from_str(affliction_kind_str) {
                let actor_afflictions = state
                    .affliction_by_actor
                    .entry(ActorId(ev.actor_id))
                    .or_default();
                let (applied, escalated) = affl::apply_affliction(
                    actor_afflictions,
                    ev.actor_id,
                    kind,
                    0.2,
                    state.affliction_registry,
                    tick.0,
                    tick_rate_hz,
                    damage_event_id.clone(),
                );
                if let Some(applied_ev) = applied {
                    out.afflictions_applied += 1;
                    let _ = recorder.record(
                        tick,
                        sim_time_ms,
                        "affliction",
                        "applied",
                        json!({
                            "actor_id": applied_ev.actor_id,
                            "kind": kind.as_str(),
                            "source_event_id": applied_ev.source_event_id,
                            "expected_duration_ticks": applied_ev.expected_duration_ticks,
                            "severity_0_1": applied_ev.severity,
                        }),
                        Some(damage_event_id.clone()),
                    );
                }
                if let Some(esc) = escalated {
                    out.afflictions_escalated += 1;
                    let _ = recorder.record(
                        tick,
                        sim_time_ms,
                        "affliction",
                        "escalated",
                        json!({
                            "actor_id": esc.actor_id,
                            "kind": kind.as_str(),
                            "from_severity": esc.from_severity,
                            "to_severity": esc.to_severity,
                        }),
                        Some(damage_event_id.clone()),
                    );
                }
            }
        }
    }

    // ----- 4) Affliction per-actor tick (decay + clear + critical banners) -----
    let actor_ids: Vec<ActorId> = state.affliction_by_actor.keys().copied().collect();
    for actor_id in actor_ids {
        let actor_state_ref = state.affliction_by_actor.get_mut(&actor_id);
        if actor_state_ref.is_none() {
            continue;
        }
        let afflictions = actor_state_ref.unwrap();
        let producer = affl::tick_actor(
            afflictions,
            actor_id.0,
            state.affliction_registry,
            tick.0,
            tick_rate_hz,
            survival_mode_active,
        );
        for ev in &producer.cleared {
            out.afflictions_cleared += 1;
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "cleared",
                json!({
                    "actor_id": ev.actor_id,
                    "kind": ev.kind.as_str(),
                    "reason": ev.reason.as_str(),
                }),
                None,
            );
        }
        for ev in &producer.tick {
            let _ = recorder.record_cosmetic(
                tick,
                sim_time_ms,
                "affliction",
                "tick",
                json!({
                    "actor_id": ev.actor_id,
                    "kind": ev.kind.as_str(),
                    "hp_delta": ev.hp_delta,
                    "tick": ev.tick,
                }),
                None,
            );
        }
        // M16 § "Banners fire on critical afflictions" — drain the pending
        // critical-banner queue and emit ux.banner_raised with severity
        // critical so the engine surfaces the affliction on the HUD.
        let pending: Vec<M16AfflictionKind> =
            afflictions.critical_banner_pending.drain(..).collect();
        for kind in pending {
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "ux",
                "banner_raised",
                json!({
                    "actor_id": actor_id.0,
                    "text": format!("CRITICAL: {}", kind.as_str()),
                    "severity": "critical",
                    "source": "m16_affliction_strip",
                    "kind": kind.as_str(),
                }),
                None,
            );
        }
    }

    // ----- 5) Swim per-actor tick -----
    for (actor_id, actor) in actor_state.world.actors.iter() {
        let in_water = actor.swim_kind != cf_actor::SwimKind::None;
        let submerged = actor.swim_kind.is_submerged();
        let helmet_sealed = actor.body_armor.helmet_seal_active() || actor.body_armor.dive_suit_equipped();
        let input = SwimTickInput {
            actor_id: actor_id.0,
            position: [actor.position.x, actor.position.y],
            in_water,
            submerged,
            helmet_sealed,
            depth_m: (-actor.position.y).max(0.0),
        };
        let SwimTickEvents {
            swim_started,
            swim_ended,
            drowning_started,
            drowning_lethal,
            hypoxic_actor_ids,
        } = state.swim_world.tick_one(input, tick.0, tick_rate_hz, cf_swim::DEFAULT_OXYGEN_RESERVOIR_SECONDS);
        for ev in &swim_started {
            out.swim_started += 1;
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "swim_started",
                json!({
                    "actor": ev.actor_id,
                    "actor_id": ev.actor_id,
                    "position": ev.position,
                    "tick": ev.tick,
                }),
                None,
            );
        }
        for ev in &swim_ended {
            out.swim_ended += 1;
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "swim_ended",
                json!({
                    "actor": ev.actor_id,
                    "actor_id": ev.actor_id,
                    "position": ev.position,
                    "tick": ev.tick,
                }),
                None,
            );
        }
        for ev in &drowning_started {
            out.drowning_started += 1;
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "drowning_started",
                json!({
                    "actor_id": ev.actor_id,
                    "position": ev.position,
                }),
                None,
            );
        }
        for ev in &drowning_lethal {
            out.drowning_lethal += 1;
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "actor",
                "drowning_lethal",
                json!({
                    "actor_id": ev.actor_id,
                    "position": ev.position,
                    "depth_m": ev.depth_m,
                }),
                None,
            );
        }
        for hypoxic_id in &hypoxic_actor_ids {
            let actor_afflictions = state.affliction_by_actor.entry(ActorId(*hypoxic_id)).or_default();
            let (applied, _) = affl::apply_affliction(
                actor_afflictions,
                *hypoxic_id,
                M16AfflictionKind::Hypoxic,
                0.1,
                state.affliction_registry,
                tick.0,
                tick_rate_hz,
                format!("swim:{}", tick.0),
            );
            if applied.is_some() {
                out.afflictions_applied += 1;
            }
        }
    }

    // ----- 5.5) M16 § per-spec affliction producers (chassis / atmos / survival) -----
    let actors_for_producers: Vec<(ActorId, cf_actor::ActorState)> = actor_state
        .world
        .actors
        .iter()
        .map(|(id, a)| (*id, a.clone()))
        .collect();
    for (actor_id, actor) in actors_for_producers.iter() {
        let actor_aff = state.affliction_by_actor.entry(*actor_id).or_default();
        let registry = state.affliction_registry;
        // M16C § Pain affliction — recomputed from the M14G wound list every
        // PAIN_RECOMPUTE_INTERVAL_TICKS ticks (perf-bound). Pain stacks per
        // active wound × severity × 12; the stack drives aim wobble, move
        // speed, morale drain, and the auto-triage trigger.
        if cf_affliction::pain_recompute_due(tick.0) {
            if let Some(pain_ev) =
                cf_affliction::recompute_pain(actor_aff, actor_id.0, &actor.m14g_wound_list, tick.0)
            {
                out.pain_recomputed += 1;
                let _ = recorder.record(
                    tick,
                    sim_time_ms,
                    "pain",
                    "stack_changed",
                    json!({
                        "actor_id": pain_ev.actor_id,
                        "tick": pain_ev.tick,
                        "old_stack": pain_ev.old_stack,
                        "new_stack": pain_ev.new_stack,
                        "severity": pain_ev.severity,
                        "aim_wobble_multiplier": pain_ev.aim_wobble_multiplier,
                    }),
                    None,
                );
            }
        }
        // hunger: caloric_energy < 20 (per M17)
        if actor.resources.caloric_energy < 20.0 && survival_mode_active {
            let sev = ((20.0 - actor.resources.caloric_energy) / 20.0).clamp(0.05, 1.0);
            if actor_aff.severity_of(M16AfflictionKind::Hunger) < sev {
                let (a, _) = affl::apply_affliction(
                    actor_aff,
                    actor_id.0,
                    M16AfflictionKind::Hunger,
                    (sev - actor_aff.severity_of(M16AfflictionKind::Hunger)).max(0.01),
                    registry,
                    tick.0,
                    tick_rate_hz,
                    format!("hunger:{}", tick.0),
                );
                if a.is_some() {
                    out.afflictions_applied += 1;
                }
            }
        }
        // thirst: water reservoir empty (M17 placeholder uses oxygen_supply as proxy).
        if actor.resources.oxygen_supply < 10.0 && survival_mode_active {
            let sev = 0.5_f32;
            if actor_aff.severity_of(M16AfflictionKind::Thirst) < sev {
                let (a, _) = affl::apply_affliction(
                    actor_aff,
                    actor_id.0,
                    M16AfflictionKind::Thirst,
                    0.05,
                    registry,
                    tick.0,
                    tick_rate_hz,
                    format!("thirst:{}", tick.0),
                );
                if a.is_some() {
                    out.afflictions_applied += 1;
                }
            }
        }
        // concussed: high-impulse impact (concussion_dose proxied by g_load_dose).
        if actor.resources.concussion_dose > 0.5 {
            let sev = (actor.resources.concussion_dose / 2.0).clamp(0.1, 1.0);
            if actor_aff.severity_of(M16AfflictionKind::Concussed) < sev {
                let (a, _) = affl::apply_affliction(
                    actor_aff,
                    actor_id.0,
                    M16AfflictionKind::Concussed,
                    0.1,
                    registry,
                    tick.0,
                    tick_rate_hz,
                    format!("concussion:{}", tick.0),
                );
                if a.is_some() {
                    out.afflictions_applied += 1;
                }
            }
        }
        // bleeding: wound contact (any active wounds → bleeding kind).
        let wound_count = wound_count_for_actor(actor);
        if wound_count > 0 {
            let sev = (wound_count as f32 * 0.25).clamp(0.1, 1.0);
            if actor_aff.severity_of(M16AfflictionKind::Bleeding) < sev {
                let delta = (sev - actor_aff.severity_of(M16AfflictionKind::Bleeding)).max(0.05);
                let (a, _) = affl::apply_affliction(
                    actor_aff,
                    actor_id.0,
                    M16AfflictionKind::Bleeding,
                    delta,
                    registry,
                    tick.0,
                    tick_rate_hz,
                    format!("wound:{}", tick.0),
                );
                if a.is_some() {
                    out.afflictions_applied += 1;
                }
            }
        }
        // overheating: chassis heat > 90% (proxy via resources.heat normalized to 1.0 cap).
        if actor.resources.heat > 0.9 {
            let sev = ((actor.resources.heat - 0.9) / 0.1).clamp(0.1, 1.0);
            if actor_aff.severity_of(M16AfflictionKind::Overheating) < sev {
                let delta = (sev - actor_aff.severity_of(M16AfflictionKind::Overheating)).max(0.05);
                let (a, _) = affl::apply_affliction(
                    actor_aff,
                    actor_id.0,
                    M16AfflictionKind::Overheating,
                    delta,
                    registry,
                    tick.0,
                    tick_rate_hz,
                    format!("overheat:{}", tick.0),
                );
                if a.is_some() {
                    out.afflictions_applied += 1;
                }
            }
        }
        // low_battery: chassis power < 30% threshold.
        if actor.resources.battery_charge > 0.0 && actor.resources.battery_charge < 30.0 {
            let sev = ((30.0 - actor.resources.battery_charge) / 30.0).clamp(0.1, 1.0);
            if actor_aff.severity_of(M16AfflictionKind::LowBattery) < sev {
                let (a, _) = affl::apply_affliction(
                    actor_aff,
                    actor_id.0,
                    M16AfflictionKind::LowBattery,
                    0.05,
                    registry,
                    tick.0,
                    tick_rate_hz,
                    format!("low_battery:{}", tick.0),
                );
                if a.is_some() {
                    out.afflictions_applied += 1;
                }
            }
        }
        // vacuum_exposure: helmet integrity < 100% + atmospheric pressure < 0.1 atm
        // (proxied via oxygen_supply == 0 + body_armor helmet_seal_active = false).
        let helmet_seal = actor.body_armor.helmet_seal_active();
        if !helmet_seal && actor.resources.oxygen_supply < 1.0 {
            let race = race_from_origin(&actor.origin_id);
            let ttd = affl::vacuum_exposure_ttd_seconds(race);
            if ttd.is_finite() {
                let sev = 0.6_f32;
                if actor_aff.severity_of(M16AfflictionKind::VacuumExposure) < sev {
                    let registry_clone = registry.clone();
                    let _ = registry_clone;
                    let (a, _) = affl::apply_affliction(
                        actor_aff,
                        actor_id.0,
                        M16AfflictionKind::VacuumExposure,
                        0.1,
                        registry,
                        tick.0,
                        tick_rate_hz,
                        format!("vacuum:{}", tick.0),
                    );
                    if a.is_some() {
                        out.afflictions_applied += 1;
                        let _ = recorder.record(
                            tick,
                            sim_time_ms,
                            "affliction",
                            "applied",
                            json!({
                                "actor_id": actor_id.0,
                                "kind": "vacuum_exposure",
                                "source_event_id": format!("vacuum:{}", tick.0),
                                "expected_duration_ticks": 1,
                                "severity_0_1": 0.1,
                                "per_origin_ttd_seconds": ttd,
                                "race": race_str(race),
                            }),
                            None,
                        );
                    }
                }
            }
        }
    }

    // ----- 6) Auto-triage reasons (M7 utility scorer bonus + ai.auto_triage_initiated) -----
    let default_thresholds = M16TriggerThresholds::default();
    for (actor_id, afflictions) in state.affliction_by_actor.iter() {
        let actor = match actor_state.world.actors.get(actor_id) {
            Some(a) => a,
            None => continue,
        };
        let hp_percent = (actor.hp / 100.0_f32.max(1.0)).clamp(0.0, 1.0);
        let thresholds = state
            .trigger_thresholds
            .get(actor_id)
            .copied()
            .unwrap_or(default_thresholds);
        let wound_count = wound_count_for_actor(actor);
        let arterial = arterial_wound_for_actor(actor);
        let dose_rate = afflictions.severity_of(M16AfflictionKind::Radiation);
        let continuous_shock = afflictions.severity_of(M16AfflictionKind::Electrified) >= 0.8
            || afflictions.severity_of(M16AfflictionKind::Shocked) >= 0.5;
        let helmet_breach_drown = afflictions.severity_of(M16AfflictionKind::Drowning) > 0.0
            && !actor.body_armor.helmet_seal_active();
        let compound_ttd = compound_ttd_for_actor(afflictions, actor);
        let reasons = affl::auto_triage_reasons(
            afflictions,
            &thresholds,
            wound_count,
            arterial,
            hp_percent,
            dose_rate,
            continuous_shock,
            helmet_breach_drown,
            compound_ttd,
        );
        let dominant = reasons.first().copied();
        if let Some(reason) = dominant {
            let prev = state.last_auto_triage_reason.get(actor_id).copied();
            if prev != Some(reason) {
                state.last_auto_triage_reason.insert(*actor_id, reason);
                let _ = recorder.record(
                    tick,
                    sim_time_ms,
                    "ai",
                    "auto_triage_initiated",
                    json!({
                        "medic_actor_id": 0,
                        "target_actor_id": actor_id.0,
                        "dying_tick": tick.0,
                        "reach_deadline_tick": tick.0 + (6 * tick_rate_hz as u64),
                        "apply_deadline_tick": tick.0 + (8 * tick_rate_hz as u64),
                        "reach_seconds": 6.0,
                        "apply_seconds": 8.0,
                        "trigger_reason": reason.as_str(),
                    }),
                    None,
                );
            }
        } else {
            state.last_auto_triage_reason.remove(actor_id);
        }
        for r in reasons {
            out.auto_triage_reasons.push((*actor_id, r));
        }
    }

    // ----- 7) M16A § Env affliction kernel (11 env-driven kinds) -----
    let env_actors: Vec<(ActorId, cf_actor::ActorState)> = actor_state
        .world
        .actors
        .iter()
        .map(|(id, a)| (*id, a.clone()))
        .collect();
    let dt_seconds = 1.0_f32 / (tick_rate_hz.max(1) as f32);
    for (actor_id, actor) in env_actors.iter() {
        let env_state = state.env_state_by_actor.entry(*actor_id).or_default();
        let origin = OriginId::from_str(&actor.origin_id);
        let susceptibility = AtmosphericSusceptibility::for_origin(origin);
        let signal = build_env_signal_for_actor(actor, state.affliction_by_actor.get(actor_id));
        let source_id = format!("m16a_env:{}", tick.0);
        let env_out = cf_affliction::env_tick_all(
            env_state,
            actor_id.0,
            susceptibility,
            &signal,
            state.env_registry,
            dt_seconds,
            Some(source_id),
        );
        for ev in env_out.threshold_crossed {
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "env_threshold_crossed",
                json!({
                    "actor_id": ev.actor_id,
                    "kind": ev.kind.as_str(),
                    "severity": ev.severity.as_str(),
                    "severity_0_1": ev.severity_0_1,
                    "accumulator_value": ev.accumulator_value,
                    "source_event_id": ev.source_event_id.clone().unwrap_or_default(),
                    "origin_id": ev.origin_id.as_str(),
                }),
                ev.source_event_id,
            );
            out.env_threshold_crossings += 1;
        }
        for ev in env_out.severity_changed {
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "env_severity_changed",
                json!({
                    "actor_id": ev.actor_id,
                    "kind": ev.kind.as_str(),
                    "from_severity": ev.from_severity,
                    "to_severity": ev.to_severity,
                    "accumulator_value": ev.accumulator_value,
                }),
                None,
            );
            out.env_severity_changes += 1;
        }
        for ev in env_out.cleared {
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "env_cleared",
                json!({
                    "actor_id": ev.actor_id,
                    "kind": ev.kind.as_str(),
                    "reason": ev.reason.as_str(),
                }),
                None,
            );
            out.env_cleared += 1;
        }
        for ev in env_out.origin_immune {
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "env_origin_immune",
                json!({
                    "actor_id": ev.actor_id,
                    "kind": ev.kind.as_str(),
                    "origin_id": ev.origin_id.as_str(),
                    "reason": ev.reason,
                    "alt_kind": ev.alt_kind.map(|k| k.as_str()),
                }),
                None,
            );
            out.env_origin_immune += 1;
        }
        if env_out.m16b_sepsis_feed {
            env_state.m16b_sepsis_feed = true;
        }
    }

    // Suppress unused-warning for `recorder` when no actors are present.
    let _ = sim_time_ms;

    out
}

/// Build the M16A env signal slice for `actor` from current sim state +
/// parent affliction signals. The actual M19/M28/M9 producers will fill
/// in atmospheric / thermal / electric values as those crates ship live
/// state to the engine; this helper is the consumer-side aggregator.
fn build_env_signal_for_actor(
    actor: &cf_actor::ActorState,
    parent: Option<&cf_affliction::ActorAfflictions>,
) -> EnvSignal {
    let body_weight_kg = 70.0_f32;
    let extra_weight = (actor.mass_kg - body_weight_kg).max(0.0);
    let mut signal = EnvSignal {
        humidity_pct: 50.0,
        co2_partial_kpa: 0.04,
        o2_partial_kpa: 21.0,
        occupant_count: 1,
        room_temp_k: 293.15,
        refrigerant_partial_kpa: 0.0,
        electric_shock_event_j: 0.0,
        spotlight_lit: false,
        razor_wire_contact: false,
        bladed_hit_severity: 0.0,
        wet_duckboard_contact: false,
        feet_dry_and_warm: true,
        heavy_weapon_kg: extra_weight,
        baseline_carry_kg: 20.0,
        analyzer_alarm_unaddressed: false,
        extreme_breach_event: false,
        stabilize_assist: false,
    };
    if let Some(parent_state) = parent {
        if parent_state.severity_of(M16AfflictionKind::Hypoxic) > 0.5 {
            signal.o2_partial_kpa = 10.0;
        }
        if parent_state.severity_of(M16AfflictionKind::Hyperthermic) > 0.0 {
            signal.room_temp_k = 330.0;
        }
        if parent_state.severity_of(M16AfflictionKind::Hypothermic) > 0.0 {
            signal.room_temp_k = 260.0;
        }
        if parent_state.severity_of(M16AfflictionKind::Electrified) > 0.0 {
            signal.electric_shock_event_j = 80.0
                * parent_state.severity_of(M16AfflictionKind::Electrified);
        }
        if parent_state.severity_of(M16AfflictionKind::Wet) > 0.5
            && signal.room_temp_k < 285.0
        {
            signal.wet_duckboard_contact = true;
            signal.feet_dry_and_warm = false;
        }
    }
    signal
}

fn race_from_origin(origin_id: &str) -> affl::Race {
    match origin_id {
        "methane" | "methane_breather" => affl::Race::Methane,
        "crystalline" => affl::Race::Crystalline,
        "aqueous" => affl::Race::Aqueous,
        "robot" | "synth" => affl::Race::Robotic,
        _ => affl::Race::Human,
    }
}

fn race_str(race: affl::Race) -> &'static str {
    match race {
        affl::Race::Human => "human",
        affl::Race::Methane => "methane",
        affl::Race::Crystalline => "crystalline",
        affl::Race::Aqueous => "aqueous",
        affl::Race::Robotic => "robotic",
    }
}

/// Translate a cf-terrain material name into a TileAffordance for the
/// cf-hazard affordance-aware spread. Maps spec § "spreads to adjacent
/// flammable (wood, oil, volatiles)" + "arcs to adjacent metal/conductive
/// within 3 tiles" + "spreads via water" onto the hazard predicate.
pub fn tile_affordance_for_material_name(name: &str) -> cf_hazard::TileAffordance {
    match name {
        "air" => cf_hazard::TileAffordance::Empty,
        "wood" | "oil" | "kerosene" | "ethanol" | "fuel" | "fabric" | "paper" | "coal" | "diesel"
        | "hydrazine" | "tnt" | "gunpowder" | "anfo" | "nitroglycerin" => cf_hazard::TileAffordance::Flammable,
        "iron" | "metal" | "steel" | "copper" | "aluminum" | "zinc" | "gold" | "metal_nohook" => {
            cf_hazard::TileAffordance::Conductive
        }
        "water" | "seawater" => cf_hazard::TileAffordance::Water,
        "concrete" | "concrete_soft" | "anchor" | "dirt" | "support_beam" | "loose_fill" | "repair_fill"
        | "hazard" | "stone" => cf_hazard::TileAffordance::Solid,
        _ => cf_hazard::TileAffordance::Other,
    }
}

/// Count bleed wounds on an actor for the auto-triage bleed-stack trigger.
/// Uses the M14G wound list when present; falls back to chassis destroyed
/// zones (each destroyed zone counts as a bleed source per cf-control's
/// existing bleed-tick wiring).
fn wound_count_for_actor(actor: &cf_actor::ActorState) -> u32 {
    let from_wound_list: u32 = actor
        .m14g_wound_list
        .iter()
        .map(|(_, v)| v.len() as u32)
        .sum();
    let from_chassis = actor
        .chassis
        .as_ref()
        .map(|c| c.destroyed_zones().len() as u32)
        .unwrap_or(0);
    from_wound_list.max(from_chassis)
}

/// True when the actor carries any wound flagged "arterial" or with a
/// known arterial severity tier. Falls back to false when the wound list
/// has no arterial signal exposed.
fn arterial_wound_for_actor(_actor: &cf_actor::ActorState) -> bool {
    false
}

/// Compute a compound TTD for the actor using cf_actor's
/// `InterimTtdContract` over the active M16 afflictions. Returns
/// `f32::INFINITY` when no lethal kinds are stacked.
fn compound_ttd_for_actor(afflictions: &ActorAfflictions, actor: &cf_actor::ActorState) -> f32 {
    use cf_actor::ttd::{AiDifficulty, InterimTtdContract, TtdAfflictionKind, TtdContract, TtdOrigin};
    let contract = InterimTtdContract::new();
    let origin = match actor.origin_id.as_str() {
        "robot" | "synth" => TtdOrigin::Robot,
        "android" | "hybrid" => TtdOrigin::Android,
        _ => TtdOrigin::Human,
    };
    let mut stack: Vec<TtdAfflictionKind> = Vec::new();
    if afflictions.severity_of(M16AfflictionKind::Bleeding) > 0.5 {
        stack.push(TtdAfflictionKind::Bleed2W);
    }
    if afflictions.severity_of(M16AfflictionKind::Burning) > 0.0 {
        stack.push(TtdAfflictionKind::Burning);
    }
    if afflictions.severity_of(M16AfflictionKind::Hypoxic) > 0.0
        || afflictions.severity_of(M16AfflictionKind::Drowning) > 0.0
    {
        stack.push(TtdAfflictionKind::OxygenEmpty);
    }
    if afflictions.severity_of(M16AfflictionKind::Concussed) > 0.0 {
        stack.push(TtdAfflictionKind::ConcussionGrace);
    }
    if stack.is_empty() {
        return f32::INFINITY;
    }
    contract.compound_ttd_seconds(&stack, origin, AiDifficulty::ToughCrowd)
}

/// Compose a one-line aggregate-bonus summary suitable for the artifact
/// HUD panel. Returns the empty string when the carrier has no bonuses.
pub fn aggregate_bonus_summary(bonus: &cf_artifact::ArtifactBonus) -> String {
    let mut parts: Vec<String> = Vec::new();
    if bonus.max_hp_bonus != 0.0 {
        parts.push(format!("+{:.0} HP", bonus.max_hp_bonus));
    }
    if bonus.aim_accuracy_bonus_pct != 0.0 {
        parts.push(format!("+{:.0}% aim", bonus.aim_accuracy_bonus_pct * 100.0));
    }
    if bonus.drop_rate_bonus_pct != 0.0 {
        parts.push(format!("+{:.0}% drops", bonus.drop_rate_bonus_pct * 100.0));
    }
    if bonus.radiation_resistance != 0.0 {
        parts.push(format!("{:+.0}% rad", bonus.radiation_resistance * 100.0));
    }
    if bonus.cold_resistance != 0.0 {
        parts.push(format!("{:+.0}% cold", bonus.cold_resistance * 100.0));
    }
    if bonus.fire_resistance != 0.0 {
        parts.push(format!("{:+.0}% fire", bonus.fire_resistance * 100.0));
    }
    if bonus.electric_resistance != 0.0 {
        parts.push(format!("{:+.0}% elec", bonus.electric_resistance * 100.0));
    }
    if bonus.toxic_resistance != 0.0 {
        parts.push(format!("{:+.0}% tox", bonus.toxic_resistance * 100.0));
    }
    if bonus.tool_durability_multiplier != 0.0 && bonus.tool_durability_multiplier != 1.0 {
        parts.push(format!("x{:.1} tool dura", bonus.tool_durability_multiplier));
    }
    if bonus.battery_capacity_multiplier != 0.0 && bonus.battery_capacity_multiplier != 1.0 {
        parts.push(format!("x{:.1} batt", bonus.battery_capacity_multiplier));
    }
    if bonus.stamina_regen_multiplier != 0.0 && bonus.stamina_regen_multiplier != 1.0 {
        parts.push(format!("x{:.1} sta regen", bonus.stamina_regen_multiplier));
    }
    if bonus.sprint_speed_multiplier != 0.0 && bonus.sprint_speed_multiplier != 1.0 {
        parts.push(format!("x{:.1} sprint", bonus.sprint_speed_multiplier));
    }
    if bonus.carry_weight_multiplier != 0.0 && bonus.carry_weight_multiplier != 1.0 {
        parts.push(format!("x{:.1} carry", bonus.carry_weight_multiplier));
    }
    if bonus.reveals_anomalies {
        parts.push("anomaly reveal".to_string());
    }
    if bonus.damage_absorption_pct != 0.0 {
        parts.push(format!("{:.0}% absorb", bonus.damage_absorption_pct * 100.0));
    }
    parts.join(", ")
}

/// Apply a counter (water flask on fire, alkali on acid, etc.) to nearby
/// hazards of `kind` within `radius` tiles. Returns the count.
pub fn apply_hazard_counter(
    hazard_world: &mut HazardWorld,
    kind: cf_hazard::HazardKind,
    position: [f32; 2],
    radius_tiles: f32,
) -> u32 {
    hazard_world.apply_counter_radius(kind, position, radius_tiles)
}

/// Spawn an artifact in the world + emit `artifact.spawned`.
pub fn spawn_artifact_with_event(
    artifact_world: &mut ArtifactWorld,
    artifact_registry: &ArtifactRegistry,
    spec_id: &str,
    position: [f32; 2],
    tick: Tick,
    sim_time_ms: f64,
    source_anomaly_id: Option<u64>,
    recorder: &Recorder,
) -> Option<u64> {
    let ev = artifact_world.spawn(artifact_registry, spec_id, position, tick.0, source_anomaly_id)?;
    let _ = recorder.record(
        tick,
        sim_time_ms,
        "artifact",
        "spawned",
        json!({
            "instance_id": ev.instance_id,
            "spec_id": ev.spec_id,
            "rarity": ev.rarity.as_str(),
            "position": ev.position,
            "source_anomaly_id": ev.source_anomaly_id,
        }),
        None,
    );
    Some(ev.instance_id)
}

/// Picks up an artifact + emits `artifact.picked_up` + `artifact.carried_bonus_applied`.
pub fn pickup_artifact_with_events(
    artifact_world: &mut ArtifactWorld,
    artifact_registry: &ArtifactRegistry,
    instance_id: u64,
    actor_id: u64,
    tick: Tick,
    sim_time_ms: f64,
    recorder: &Recorder,
) -> bool {
    let (pickup_ev, carry_ev) = match artifact_world.pickup(artifact_registry, instance_id, actor_id) {
        Some(v) => v,
        None => return false,
    };
    let pickup_event_id = recorder.record(
        tick,
        sim_time_ms,
        "artifact",
        "picked_up",
        json!({
            "instance_id": pickup_ev.instance_id,
            "spec_id": pickup_ev.spec_id,
            "actor_id": pickup_ev.actor_id,
            "rarity": pickup_ev.rarity.as_str(),
        }),
        None,
    );
    let _ = recorder.record(
        tick,
        sim_time_ms,
        "artifact",
        "carried_bonus_applied",
        json!({
            "instance_id": carry_ev.instance_id,
            "spec_id": carry_ev.spec_id,
            "actor_id": carry_ev.actor_id,
            "bonus": serde_json::to_value(carry_ev.bonus_snapshot).unwrap_or_default(),
        }),
        Some(pickup_event_id),
    );
    true
}

/// Clear an affliction (medikit / environment / death).
pub fn clear_affliction_with_event(
    affliction_by_actor: &mut std::collections::BTreeMap<ActorId, ActorAfflictions>,
    actor_id: ActorId,
    kind: M16AfflictionKind,
    reason: ClearReason,
    tick: Tick,
    sim_time_ms: f64,
    recorder: &Recorder,
) -> bool {
    let actor_afflictions = match affliction_by_actor.get_mut(&actor_id) {
        Some(a) => a,
        None => return false,
    };
    match affl::clear_affliction(actor_afflictions, actor_id.0, kind, reason) {
        Some(ev) => {
            let _ = recorder.record(
                tick,
                sim_time_ms,
                "affliction",
                "cleared",
                json!({
                    "actor_id": ev.actor_id,
                    "kind": kind.as_str(),
                    "reason": ev.reason.as_str(),
                }),
                None,
            );
            true
        }
        None => false,
    }
}

#[allow(unused_imports)]
use cf_actor as _;

// Force-bring `HazardSpawnedEvent` import path for clarity (compiler
// otherwise warns "unused import" because we only use the type indirectly
// through ev field access).
#[allow(dead_code)]
fn _import_helpers(
    _: &HazardSpawnedEvent,
    _: &ArtifactSpawnedEvent,
    _: &ArtifactPickedUpEvent,
    _: &ArtifactCarriedBonusEvent,
) {
}
