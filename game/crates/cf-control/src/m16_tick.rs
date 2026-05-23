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
    self as affl, ActorAfflictions, AfflictionRegistry, AutoTriageReason, ClearReason, M16AfflictionKind,
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
    pub swim_started: u32,
    pub swim_ended: u32,
    pub drowning_started: u32,
    pub drowning_lethal: u32,
    pub auto_triage_reasons: Vec<(ActorId, AutoTriageReason)>,
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
    } = inputs;

    let mut out = M16TickOutput::default();

    // ----- 1) Hazard grid tick (spread + dissipation + cosmetic tick) -----
    let HazardTickOutput {
        spawned,
        spread,
        actor_contact: _,
        tick: cosmetic_tick,
        dissipated,
    } = state.hazard_world.tick_grid(state.hazard_registry, tick.0, tick_rate_hz);
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

    // ----- 4) Affliction per-actor tick (decay + clear) -----
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

    // ----- 6) Auto-triage reasons (M7 utility scorer bonus surface) -----
    for (actor_id, afflictions) in state.affliction_by_actor.iter() {
        let hp_percent = actor_state
            .world
            .actors
            .get(actor_id)
            .map(|a| (a.hp / a.hp.max(1.0)).clamp(0.0, 1.0))
            .unwrap_or(1.0);
        let reasons = affl::auto_triage_reasons(
            afflictions,
            0,
            false,
            hp_percent,
            afflictions.severity_of(M16AfflictionKind::Radiation),
            afflictions.severity_of(M16AfflictionKind::Electrified) >= 0.8,
            false,
            f32::INFINITY,
        );
        for r in reasons {
            out.auto_triage_reasons.push((*actor_id, r));
        }
    }

    // Suppress unused-warning for `recorder` when no actors are present.
    let _ = sim_time_ms;

    out
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
