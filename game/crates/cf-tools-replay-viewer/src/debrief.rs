//! M3B-003: debrief summary.
//!
//! Composes a concise post-run report covering:
//!
//! - Outcome (`mission_resolved.result` + reason).
//! - Objectives (`objective_started` / `objective_failed` / `objective_completed`).
//! - Key events (counts by category, top 3 event types overall).
//! - Damage / death recap (`actor_died` count + reactor-damage trajectory).
//! - Terrain changes (`terrain_carved` count + total carved pixels +
//!   chunk_dirtied count + dominant materials touched).
//! - Checksum status (`final_sim_checksum` + cadence + checksum_event_count
//!   + first/last tick).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;

use crate::bundle::Bundle;

#[derive(Debug, Clone)]
pub struct Debrief<'a> {
    pub bundle: &'a Bundle,
    pub outcome: Outcome,
    pub objectives: Vec<Objective>,
    pub damage: DamageRecap,
    pub terrain: TerrainRecap,
    pub key_events: KeyEvents,
    pub checksum: ChecksumStatus,
    pub m17: M17Recap,
}

#[derive(Debug, Clone, Default)]
pub struct Outcome {
    pub result: Option<String>,
    pub reason: Option<String>,
    pub resolved_at_tick: Option<u64>,
    pub resolved_event_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Objective {
    pub objective: String,
    pub started_at_tick: Option<u64>,
    pub ended_at_tick: Option<u64>,
    pub state: ObjectiveState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObjectiveState {
    Active,
    Completed,
    Failed,
}

impl ObjectiveState {
    fn label(&self) -> &'static str {
        match self {
            ObjectiveState::Active => "active",
            ObjectiveState::Completed => "completed",
            ObjectiveState::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct DamageRecap {
    pub actor_deaths: u64,
    pub projectile_hits: u64,
    pub total_projectile_damage: f64,
    pub reactor_damage_events: u64,
    pub reactor_destroyed: bool,
    pub reactor_destroyed_at_tick: Option<u64>,
    /// + cumulative damage. Each entry maps shooter actor_id → (hits, damage).
    pub by_source_actor: BTreeMap<u64, (u64, f64)>,
    /// label (e.g. "rifle_m1_default", "knife_m6_default"). Sourced from
    /// `equipment.weapon_fired.weapon` events crossed with hits.
    pub by_weapon: BTreeMap<String, u64>,
    /// the `surface_kind` payload field on `combat.projectile_hit_mo`.
    /// Drops projectile_hits w/o explicit surface_kind into "unknown".
    pub by_surface_kind: BTreeMap<String, u64>,
    /// `damage_kind` (kinetic / piercing / slash / blunt / explosion / etc.).
    pub by_damage_kind: BTreeMap<String, u64>,
    /// hits sourced from `armor.layer_hp_changed.layer`.
    pub by_layer_struck: BTreeMap<String, u64>,
    pub pierced_count: u64,
}

#[derive(Debug, Clone, Default)]
pub struct TerrainRecap {
    pub terrain_carved_events: u64,
    pub total_carved_pixels: u64,
    pub chunk_dirtied_events: u64,
    /// material -> carve event count (e.g., dirt: 7, concrete: 2)
    pub by_material: BTreeMap<String, u64>,
}

#[derive(Debug, Clone, Default)]
pub struct KeyEvents {
    pub by_category: BTreeMap<String, u64>,
    pub by_type: BTreeMap<String, u64>,
    pub error_count: u64,
    pub warn_count: u64,
    pub dropped_total: u64,
}

#[derive(Debug, Clone, Default)]
pub struct ChecksumStatus {
    pub algorithm: String,
    pub scope: String,
    pub cadence_ticks: u64,
    pub final_checksum: Option<String>,
    pub checksum_event_count: u64,
    pub first_tick: Option<u64>,
    pub last_tick: Option<u64>,
}

/// Reaction class of an origin. Drives which vocabulary the death recap uses
/// so a synthetic chassis is never described as "bleeding out" and an organic
/// body is never described as "going offline".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginClass {
    Organic,
    Synthetic,
}

impl OriginClass {
    fn label(self) -> &'static str {
        match self {
            OriginClass::Organic => "organic",
            OriginClass::Synthetic => "synthetic",
        }
    }
}

/// Map a replay `origin_id` (PascalCase, locked at v0.1) to its reaction class.
/// Robot is the only fully-synthetic launch origin; the rest route through the
/// organic flesh/blood/concussion model.
fn origin_class_of(origin_id: &str) -> Option<OriginClass> {
    match origin_id {
        "Human" | "Android" | "PoweredOrganic" | "HeavyBiomech" => Some(OriginClass::Organic),
        "Robot" => Some(OriginClass::Synthetic),
        _ => None,
    }
}

/// Per-kind tally over the `resource.*` event stream.
#[derive(Debug, Clone, Default)]
pub struct ResourceKindStat {
    pub changed: u64,
    pub critical: u64,
    pub depleted: u64,
}

/// `origin.shot_force_feedback` + `origin.helmet_breach` aggregate.
#[derive(Debug, Clone, Default)]
pub struct ForceFeedbackSummary {
    pub total: u64,
    /// pain_jolt / servo_jolt / frame_ring counts.
    pub by_feedback_kind: BTreeMap<String, u64>,
    pub helmet_breaches: u64,
    pub helmet_breach_actors: BTreeSet<u64>,
    pub g_load_total: f64,
    pub g_load_by_actor: BTreeMap<u64, f64>,
}

/// `internal.*` organ-vs-circuit destroyed/damaged tallies.
#[derive(Debug, Clone, Default)]
pub struct InternalDamageBreakdown {
    pub organs_damaged: u64,
    pub organs_destroyed: u64,
    pub circuits_damaged: u64,
    pub circuits_destroyed: u64,
}

/// One dead (or offlined) actor's origin-appropriate cause chain. Organic
/// fields stay empty for a synthetic chassis and vice-versa.
#[derive(Debug, Clone)]
pub struct OriginDeath {
    pub actor_id: u64,
    pub origin_id: Option<String>,
    pub class: Option<OriginClass>,
    pub died_at_tick: Option<u64>,
    // organic cause channels
    pub organs_destroyed: Vec<String>,
    pub organ_failures: Vec<String>,
    pub concussion_band: Option<String>,
    pub concussion_ko: bool,
    pub bled_out: bool,
    pub bleeding: bool,
    // synthetic cause channels
    pub circuits_destroyed: Vec<String>,
    pub circuit_failures: Vec<String>,
    pub internal_shock_hits: u64,
    pub modules_damaged: Vec<String>,
    pub power_depleted: bool,
    pub went_offline: bool,
    pub offline_reason: Option<String>,
    pub thermal_cascade: bool,
}

impl OriginDeath {
    fn new(actor_id: u64) -> Self {
        OriginDeath {
            actor_id,
            origin_id: None,
            class: None,
            died_at_tick: None,
            organs_destroyed: Vec::new(),
            organ_failures: Vec::new(),
            concussion_band: None,
            concussion_ko: false,
            bled_out: false,
            bleeding: false,
            circuits_destroyed: Vec::new(),
            circuit_failures: Vec::new(),
            internal_shock_hits: 0,
            modules_damaged: Vec::new(),
            power_depleted: false,
            went_offline: false,
            offline_reason: None,
            thermal_cascade: false,
        }
    }
}

/// M17 per-origin death recap + resource / force-feedback / internal-damage
/// rollups, all derived from the recorded event stream (offline; no engine).
#[derive(Debug, Clone, Default)]
pub struct M17Recap {
    pub deaths: Vec<OriginDeath>,
    pub resource_timeline: BTreeMap<String, ResourceKindStat>,
    pub force_feedback: ForceFeedbackSummary,
    pub internal_damage: InternalDamageBreakdown,
}

/// Compose a debrief from the bundle.
pub fn compose<'a>(bundle: &'a Bundle) -> Debrief<'a> {
    let outcome = compose_outcome(bundle);
    let objectives = compose_objectives(bundle);
    let damage = compose_damage(bundle);
    let terrain = compose_terrain(bundle);
    let key_events = compose_key_events(bundle);
    let checksum = compose_checksum(bundle);
    let m17 = compose_m17(bundle);
    Debrief {
        bundle,
        outcome,
        objectives,
        damage,
        terrain,
        key_events,
        checksum,
        m17,
    }
}

fn compose_outcome(bundle: &Bundle) -> Outcome {
    let resolved = bundle.first_event_of_type("mission_resolved");
    if let Some(event) = resolved {
        let result = event.payload.get("result").and_then(|v| v.as_str()).map(str::to_string);
        let reason = event.payload.get("reason").and_then(|v| v.as_str()).map(str::to_string);
        Outcome {
            result,
            reason,
            resolved_at_tick: Some(event.tick),
            resolved_event_id: Some(event.event_id.clone()),
        }
    } else {
        Outcome::default()
    }
}

fn compose_objectives(bundle: &Bundle) -> Vec<Objective> {
    let mut by_id: BTreeMap<String, Objective> = BTreeMap::new();
    for event in bundle.events.iter() {
        let key = event
            .payload
            .get("objective")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        match (event.event_type.as_str(), key) {
            ("objective_started", Some(name)) => {
                by_id
                    .entry(name.clone())
                    .or_insert(Objective {
                        objective: name,
                        started_at_tick: None,
                        ended_at_tick: None,
                        state: ObjectiveState::Active,
                    })
                    .started_at_tick
                    .get_or_insert(event.tick);
            }
            ("objective_failed", Some(name)) => {
                let entry = by_id.entry(name.clone()).or_insert(Objective {
                    objective: name,
                    started_at_tick: None,
                    ended_at_tick: None,
                    state: ObjectiveState::Active,
                });
                entry.ended_at_tick = Some(event.tick);
                entry.state = ObjectiveState::Failed;
            }
            ("objective_completed", Some(name)) => {
                let entry = by_id.entry(name.clone()).or_insert(Objective {
                    objective: name,
                    started_at_tick: None,
                    ended_at_tick: None,
                    state: ObjectiveState::Active,
                });
                entry.ended_at_tick = Some(event.tick);
                entry.state = ObjectiveState::Completed;
            }
            _ => {}
        }
    }
    by_id.into_values().collect()
}

fn compose_damage(bundle: &Bundle) -> DamageRecap {
    let mut recap = DamageRecap::default();
    // resolve weapon labels on subsequent projectile_hit events.
    let mut weapon_by_projectile: BTreeMap<u64, String> = BTreeMap::new();
    for event in bundle.events.iter() {
        match event.event_type.as_str() {
            "actor_died" => recap.actor_deaths += 1,
            "weapon_fired" => {
                if let Some(weapon) = event.payload.get("weapon").and_then(|v| v.as_str()) {
                    if let Some(projectile_id) = event.payload.get("projectile_id").and_then(|v| v.as_u64()) {
                        weapon_by_projectile.insert(projectile_id, weapon.to_string());
                    }
                }
            }
            "projectile_hit" | "projectile_hit_mo" => {
                recap.projectile_hits += 1;
                if let Some(dmg) = event.payload.get("damage").and_then(|v| v.as_f64()) {
                    recap.total_projectile_damage += dmg;
                    // per-source-actor breakdown
                    if let Some(shooter) = event
                        .payload
                        .get("shooter")
                        .and_then(|v| v.as_u64())
                        .or_else(|| event.payload.get("shooter_id").and_then(|v| v.as_u64()))
                    {
                        let entry = recap.by_source_actor.entry(shooter).or_insert((0, 0.0));
                        entry.0 += 1;
                        entry.1 += dmg;
                    }
                }
                // per-weapon breakdown
                if let Some(projectile_id) = event.payload.get("projectile_id").and_then(|v| v.as_u64()) {
                    if let Some(weapon) = weapon_by_projectile.get(&projectile_id) {
                        *recap.by_weapon.entry(weapon.clone()).or_insert(0) += 1;
                    }
                }
                // per-surface-kind + damage_kind breakdown (from
                // combat.projectile_hit_mo expanded payload).
                if let Some(kind) = event.payload.get("surface_kind").and_then(|v| v.as_str()) {
                    *recap.by_surface_kind.entry(kind.to_string()).or_insert(0) += 1;
                } else {
                    *recap.by_surface_kind.entry("unknown".to_string()).or_insert(0) += 1;
                }
                if let Some(kind) = event.payload.get("damage_kind").and_then(|v| v.as_str()) {
                    *recap.by_damage_kind.entry(kind.to_string()).or_insert(0) += 1;
                }
            }
            "layer_hp_changed" => {
                if let Some(layer) = event.payload.get("layer").and_then(|v| v.as_str()) {
                    *recap.by_layer_struck.entry(layer.to_string()).or_insert(0) += 1;
                }
            }
            "layer_destroyed" | "all_layers_destroyed" => recap.pierced_count += 1,
            "reactor_damaged" => {
                recap.reactor_damage_events += 1;
                let destroyed = event
                    .payload
                    .get("destroyed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                if destroyed && !recap.reactor_destroyed {
                    recap.reactor_destroyed = true;
                    recap.reactor_destroyed_at_tick = Some(event.tick);
                }
            }
            "reactor_destroyed" => {
                recap.reactor_destroyed = true;
                recap.reactor_destroyed_at_tick.get_or_insert(event.tick);
            }
            _ => {}
        }
    }
    recap
}

fn compose_terrain(bundle: &Bundle) -> TerrainRecap {
    let mut recap = TerrainRecap::default();
    for event in bundle.events.iter() {
        match event.event_type.as_str() {
            "terrain_carved" => {
                recap.terrain_carved_events += 1;
                if let Some(count) = event.payload.get("count").and_then(|v| v.as_u64()) {
                    recap.total_carved_pixels += count;
                }
                if let Some(material) = event.payload.get("material").and_then(|v| v.as_str()) {
                    *recap.by_material.entry(material.to_string()).or_insert(0) += 1;
                }
            }
            "chunk_dirtied" => recap.chunk_dirtied_events += 1,
            _ => {}
        }
    }
    recap
}

fn compose_key_events(bundle: &Bundle) -> KeyEvents {
    let mut k = KeyEvents {
        by_category: bundle.summary.event_counts.by_category.clone(),
        by_type: bundle.summary.event_counts.by_type.clone(),
        error_count: bundle
            .summary
            .event_counts
            .by_severity
            .get("error")
            .copied()
            .unwrap_or(0),
        warn_count: bundle
            .summary
            .event_counts
            .by_severity
            .get("warn")
            .copied()
            .unwrap_or(0),
        dropped_total: bundle.summary.event_counts.dropped_total,
    };
    if k.by_category.is_empty() && k.by_type.is_empty() {
        for event in bundle.events.iter() {
            *k.by_category.entry(event.category.clone()).or_insert(0) += 1;
            *k.by_type.entry(event.event_type.clone()).or_insert(0) += 1;
        }
    }
    k
}

fn compose_checksum(bundle: &Bundle) -> ChecksumStatus {
    ChecksumStatus {
        algorithm: bundle.manifest.checksum.algorithm.clone(),
        scope: bundle.manifest.checksum.scope.clone(),
        cadence_ticks: bundle.manifest.checksum.cadence_ticks,
        final_checksum: bundle.summary.final_sim_checksum.clone(),
        checksum_event_count: bundle.summary.checksum_event_count,
        first_tick: bundle.summary.first_tick,
        last_tick: bundle.summary.last_tick,
    }
}

/// Single pass over the event stream that builds the M17 origin recap:
/// actor→origin map, dead-actor cause chains, resource tallies, force-feedback
/// rollup, and internal-damage breakdown. Everything is derived from recorded
/// events — the debrief never touches the engine.
fn compose_m17(bundle: &Bundle) -> M17Recap {
    let mut origin_id_by_actor: BTreeMap<u64, String> = BTreeMap::new();
    let mut causes: BTreeMap<u64, OriginDeath> = BTreeMap::new();
    let mut dead: BTreeMap<u64, Option<u64>> = BTreeMap::new();
    let mut force_feedback = ForceFeedbackSummary::default();
    let mut resource_timeline: BTreeMap<String, ResourceKindStat> = BTreeMap::new();
    let mut internal_damage = InternalDamageBreakdown::default();

    for event in bundle.events.iter() {
        let cat = event.category.as_str();
        let ety = event.event_type.as_str();
        let p = &event.payload;
        let aid = event_actor_id(event);

        // actor → origin (last-seen wins) from the events that carry origin_id.
        if let Some(id) = aid {
            if (cat == "origin" && ety == "shot_force_feedback") || (cat == "concussion" && ety == "dose_changed") {
                if let Some(origin) = p.get("origin_id").and_then(|v| v.as_str()) {
                    origin_id_by_actor.insert(id, origin.to_string());
                }
            }
        }

        // Death / incapacitation detection (mirrors DamageRecap's actor_died
        // plus an `actor_status_changed → dead/dying/inert` fallback).
        let status_is_terminal = ety == "actor_status_changed"
            && p.get("new_status")
                .and_then(|v| v.as_str())
                .map(|s| {
                    matches!(
                        s.to_ascii_lowercase().as_str(),
                        "dead" | "dying" | "inert" | "incapacitated" | "destroyed"
                    )
                })
                .unwrap_or(false);
        if ety == "actor_died" || status_is_terminal {
            if let Some(id) = aid {
                dead.entry(id).or_insert(Some(event.tick));
            }
        }

        match (cat, ety) {
            ("internal", "organ_damaged") => internal_damage.organs_damaged += 1,
            ("internal", "organ_destroyed") => {
                internal_damage.organs_destroyed += 1;
                if let Some(id) = aid {
                    causes
                        .entry(id)
                        .or_insert_with(|| OriginDeath::new(id))
                        .organs_destroyed
                        .push(organ_or_circuit_label(p, "organ_kind", "organ_id", "organ"));
                }
            }
            ("internal", "organ_failure_cascade") => {
                if let Some(id) = aid {
                    causes
                        .entry(id)
                        .or_insert_with(|| OriginDeath::new(id))
                        .organ_failures
                        .push(organ_or_circuit_label(p, "organ_kind", "organ_id", "organ"));
                }
            }
            ("internal", "circuit_damaged") => internal_damage.circuits_damaged += 1,
            ("internal", "circuit_destroyed") => {
                internal_damage.circuits_destroyed += 1;
                if let Some(id) = aid {
                    causes
                        .entry(id)
                        .or_insert_with(|| OriginDeath::new(id))
                        .circuits_destroyed
                        .push(organ_or_circuit_label(p, "circuit_kind", "circuit_id", "circuit"));
                }
            }
            ("internal", "circuit_failure_cascade") => {
                if let Some(id) = aid {
                    causes
                        .entry(id)
                        .or_insert_with(|| OriginDeath::new(id))
                        .circuit_failures
                        .push(organ_or_circuit_label(p, "circuit_kind", "circuit_id", "circuit"));
                }
            }
            ("concussion", "band_changed") => {
                if let Some(id) = aid {
                    if let Some(band) = p.get("to_band").and_then(|v| v.as_str()) {
                        causes.entry(id).or_insert_with(|| OriginDeath::new(id)).concussion_band = Some(band.to_string());
                    }
                }
            }
            ("concussion", "ko_threshold_crossed") => {
                if let Some(id) = aid {
                    causes.entry(id).or_insert_with(|| OriginDeath::new(id)).concussion_ko = true;
                }
            }
            ("internal_shock", "dose_changed") => {
                if let Some(id) = aid {
                    causes.entry(id).or_insert_with(|| OriginDeath::new(id)).internal_shock_hits += 1;
                }
            }
            ("internal_shock", "module_damaged") => {
                if let Some(id) = aid {
                    let module = p.get("module_id").and_then(|v| v.as_str()).unwrap_or("module").to_string();
                    causes
                        .entry(id)
                        .or_insert_with(|| OriginDeath::new(id))
                        .modules_damaged
                        .push(module);
                }
            }
            ("chassis", "thermal_throttle_started") => {
                if let Some(id) = aid {
                    causes.entry(id).or_insert_with(|| OriginDeath::new(id)).thermal_cascade = true;
                }
            }
            ("affliction", "applied") => {
                if let Some(id) = aid {
                    if p.get("kind").and_then(|v| v.as_str()) == Some("bleeding") {
                        causes.entry(id).or_insert_with(|| OriginDeath::new(id)).bleeding = true;
                    }
                }
            }
            ("resource", "changed") => {
                if let Some(kind) = p.get("kind").and_then(|v| v.as_str()) {
                    resource_timeline.entry(kind.to_string()).or_default().changed += 1;
                }
            }
            ("resource", "critical") => {
                if let Some(kind) = p.get("kind").and_then(|v| v.as_str()) {
                    resource_timeline.entry(kind.to_string()).or_default().critical += 1;
                }
            }
            ("resource", "depleted") => {
                if let Some(kind) = p.get("kind").and_then(|v| v.as_str()) {
                    resource_timeline.entry(kind.to_string()).or_default().depleted += 1;
                    if let Some(id) = aid {
                        let c = causes.entry(id).or_insert_with(|| OriginDeath::new(id));
                        match kind {
                            "blood" | "bio_fluid" => c.bled_out = true,
                            "power" | "oil" => c.power_depleted = true,
                            _ => {}
                        }
                    }
                }
            }
            ("resource", "cascade_offline") => {
                if let Some(id) = aid {
                    let reason = p.get("reason").and_then(|v| v.as_str()).map(str::to_string);
                    let c = causes.entry(id).or_insert_with(|| OriginDeath::new(id));
                    c.went_offline = true;
                    c.power_depleted = true;
                    if c.offline_reason.is_none() {
                        c.offline_reason = reason;
                    }
                    dead.entry(id).or_insert(Some(event.tick));
                }
            }
            ("origin", "shot_force_feedback") => {
                force_feedback.total += 1;
                if let Some(fk) = p.get("feedback_kind").and_then(|v| v.as_str()) {
                    *force_feedback.by_feedback_kind.entry(fk.to_string()).or_insert(0) += 1;
                }
                if let Some(g) = p.get("g_load_delta").and_then(|v| v.as_f64()) {
                    if g.is_finite() && g != 0.0 {
                        force_feedback.g_load_total += g;
                        if let Some(id) = aid {
                            *force_feedback.g_load_by_actor.entry(id).or_insert(0.0) += g;
                        }
                    }
                }
            }
            ("origin", "helmet_breach") => {
                force_feedback.helmet_breaches += 1;
                if let Some(id) = aid {
                    force_feedback.helmet_breach_actors.insert(id);
                }
            }
            _ => {}
        }
    }

    let mut deaths: Vec<OriginDeath> = Vec::new();
    for (aid, tick) in dead.into_iter() {
        let mut d = causes.remove(&aid).unwrap_or_else(|| OriginDeath::new(aid));
        d.died_at_tick = tick;
        d.origin_id = origin_id_by_actor.get(&aid).cloned();
        d.class = d
            .origin_id
            .as_deref()
            .and_then(origin_class_of)
            .or_else(|| infer_class_from_causes(&d));
        deaths.push(d);
    }

    M17Recap {
        deaths,
        resource_timeline,
        force_feedback,
        internal_damage,
    }
}

/// Best-effort reaction class when no `origin_id` telemetry is attached to the
/// dead actor: organic if it shows flesh/blood/concussion damage, synthetic if
/// it shows circuit/shock/power/heat damage.
fn infer_class_from_causes(d: &OriginDeath) -> Option<OriginClass> {
    let organic = !d.organs_destroyed.is_empty()
        || !d.organ_failures.is_empty()
        || d.concussion_band.is_some()
        || d.concussion_ko
        || d.bled_out
        || d.bleeding;
    let synthetic = !d.circuits_destroyed.is_empty()
        || !d.circuit_failures.is_empty()
        || d.internal_shock_hits > 0
        || !d.modules_damaged.is_empty()
        || d.went_offline
        || d.thermal_cascade;
    match (organic, synthetic) {
        (true, false) => Some(OriginClass::Organic),
        (false, true) => Some(OriginClass::Synthetic),
        _ => None,
    }
}

fn organ_or_circuit_label(p: &serde_json::Value, kind_key: &str, id_key: &str, fallback: &str) -> String {
    p.get(kind_key)
        .and_then(|v| v.as_str())
        .or_else(|| p.get(id_key).and_then(|v| v.as_str()))
        .unwrap_or(fallback)
        .to_string()
}

/// Order-preserving de-duplicating join (so `heart, heart, liver` → `heart, liver`).
fn join_unique(items: &[String]) -> String {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut out: Vec<&str> = Vec::new();
    for item in items {
        if seen.insert(item.as_str()) {
            out.push(item.as_str());
        }
    }
    out.join(", ")
}

/// True when a verbatim payload string is safe to print on a synthetic line
/// (carries no organic vocabulary that would read as wrong-origin language).
fn synthetic_safe(s: &str) -> bool {
    let s = s.to_ascii_lowercase();
    !["organ", "blood", "bled", "concuss", "flesh"].iter().any(|w| s.contains(w))
}

fn render_death_recap(out: &mut String, deaths: &[OriginDeath]) {
    if deaths.is_empty() {
        let _ = writeln!(out, "_no actor deaths recorded_");
        return;
    }
    for d in deaths.iter() {
        let at = d
            .died_at_tick
            .map(|t| format!(" (tick {t})"))
            .unwrap_or_default();
        let origin = d.origin_id.as_deref();
        match d.class {
            Some(OriginClass::Organic) => {
                let mut parts: Vec<String> = Vec::new();
                if !d.organs_destroyed.is_empty() {
                    parts.push(format!("organ destroyed: {}", join_unique(&d.organs_destroyed)));
                }
                if !d.organ_failures.is_empty() {
                    parts.push(format!("organ failure: {}", join_unique(&d.organ_failures)));
                }
                if let Some(band) = &d.concussion_band {
                    parts.push(format!("concussion band: {band}"));
                }
                if d.concussion_ko {
                    parts.push("concussion KO".to_string());
                }
                if d.bled_out {
                    parts.push("bled out".to_string());
                } else if d.bleeding {
                    parts.push("bleeding".to_string());
                }
                if parts.is_empty() {
                    parts.push("cause undetermined".to_string());
                }
                let _ = writeln!(
                    out,
                    "- actor #{} ({}, organic){}: {}",
                    d.actor_id,
                    origin.unwrap_or("organic"),
                    at,
                    parts.join("; ")
                );
            }
            Some(OriginClass::Synthetic) => {
                let mut parts: Vec<String> = Vec::new();
                if !d.circuits_destroyed.is_empty() {
                    parts.push(format!("circuit destroyed: {}", join_unique(&d.circuits_destroyed)));
                }
                if !d.circuit_failures.is_empty() {
                    parts.push(format!("circuit failure: {}", join_unique(&d.circuit_failures)));
                }
                if d.internal_shock_hits > 0 {
                    parts.push(format!("internal shock ×{}", d.internal_shock_hits));
                }
                if !d.modules_damaged.is_empty() {
                    parts.push(format!("module damage: {}", join_unique(&d.modules_damaged)));
                }
                if d.went_offline {
                    match d.offline_reason.as_deref() {
                        Some(r) if synthetic_safe(r) => parts.push(format!("went offline ({r})")),
                        _ => parts.push("went offline".to_string()),
                    }
                } else if d.power_depleted {
                    parts.push("power depleted".to_string());
                }
                if d.thermal_cascade {
                    parts.push("thermal cascade".to_string());
                }
                if parts.is_empty() {
                    parts.push("cause undetermined".to_string());
                }
                let _ = writeln!(
                    out,
                    "- actor #{} ({}, synthetic){}: {}",
                    d.actor_id,
                    origin.unwrap_or("synthetic"),
                    at,
                    parts.join("; ")
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "- actor #{}{}: cause undetermined (no origin telemetry)",
                    d.actor_id, at
                );
            }
        }
    }
}

fn render_resource_timeline(out: &mut String, m17: &M17Recap) {
    if m17.resource_timeline.is_empty() {
        let _ = writeln!(out, "_no `resource.*` events recorded_");
        return;
    }
    let _ = writeln!(out, "| kind | changed | critical | depleted |");
    let _ = writeln!(out, "|------|---------|----------|----------|");
    for (kind, stat) in m17.resource_timeline.iter() {
        let _ = writeln!(
            out,
            "| `{kind}` | {} | {} | {} |",
            stat.changed, stat.critical, stat.depleted
        );
    }
}

fn render_internal_damage(out: &mut String, m17: &M17Recap) {
    let i = &m17.internal_damage;
    let _ = writeln!(
        out,
        "- Organs — damaged: {} · destroyed: {}",
        i.organs_damaged, i.organs_destroyed
    );
    let _ = writeln!(
        out,
        "- Circuits — damaged: {} · destroyed: {}",
        i.circuits_damaged, i.circuits_destroyed
    );
}

fn render_force_feedback(out: &mut String, m17: &M17Recap) {
    let ff = &m17.force_feedback;
    let _ = writeln!(out, "- Total `origin.shot_force_feedback` events: {}", ff.total);
    if !ff.by_feedback_kind.is_empty() {
        let _ = writeln!(out, "- By feedback kind:");
        for (kind, count) in ff.by_feedback_kind.iter() {
            let _ = writeln!(out, "  - `{kind}`: {count}");
        }
    }
    let _ = writeln!(
        out,
        "- Helmet breaches: {} (actors affected: {})",
        ff.helmet_breaches,
        ff.helmet_breach_actors.len()
    );
    let _ = writeln!(out, "- Cumulative g-load dose: {:.1}", ff.g_load_total);
    if !ff.g_load_by_actor.is_empty() {
        let _ = writeln!(out, "- G-load by actor:");
        for (actor, dose) in ff.g_load_by_actor.iter() {
            let _ = writeln!(out, "  - actor #{actor}: {dose:.1}");
        }
    }
}

/// Render a debrief as deterministic markdown.
pub fn render_markdown(debrief: &Debrief<'_>) -> String {
    let mut out = String::new();
    let m = &debrief.bundle.manifest;
    let s = &debrief.bundle.summary;

    let _ = writeln!(out, "# Debrief — `{}`", m.run_id);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Scenario `{}` ({}); milestone `{}` ({}); seed {}; tick rate {} Hz; run mode `{}`.",
        m.scene.id, m.scene.display_name, m.milestone, m.prototype_slice, m.seed, m.tick_rate_hz, m.run_mode,
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Wall duration {:.3} s; ticks run {}; total events {}; result `{}`; exit code {}.",
        s.duration_sec, s.performance.ticks_run, s.event_counts.total, s.result, s.exit_code,
    );
    let _ = writeln!(out);

    // ---- Outcome ----
    let _ = writeln!(out, "## Outcome");
    let _ = writeln!(out);
    match (&debrief.outcome.result, &debrief.outcome.reason) {
        (Some(result), Some(reason)) => {
            let _ = writeln!(out, "- Result: `{result}` (reason: `{reason}`)");
        }
        (Some(result), None) => {
            let _ = writeln!(out, "- Result: `{result}` (no explicit reason)");
        }
        (None, _) => {
            let _ = writeln!(out, "- Result: not resolved (`mission_resolved` event not emitted)");
        }
    }
    if let Some(tick) = debrief.outcome.resolved_at_tick {
        let _ = writeln!(out, "- Resolved at tick: {tick}");
    }
    if let Some(eid) = &debrief.outcome.resolved_event_id {
        let _ = writeln!(out, "- Resolved event id: `{eid}`");
    }
    let _ = writeln!(out);

    // ---- Objectives ----
    let _ = writeln!(out, "## Objectives");
    let _ = writeln!(out);
    if debrief.objectives.is_empty() {
        let _ = writeln!(out, "_no `objective_*` events emitted_");
    } else {
        let _ = writeln!(out, "| objective | state | started_tick | ended_tick |");
        let _ = writeln!(out, "|-----------|-------|--------------|------------|");
        for obj in debrief.objectives.iter() {
            let _ = writeln!(
                out,
                "| `{name}` | `{state}` | {started} | {ended} |",
                name = obj.objective,
                state = obj.state.label(),
                started = obj.started_at_tick.map(|t| t.to_string()).unwrap_or_else(|| "—".into()),
                ended = obj.ended_at_tick.map(|t| t.to_string()).unwrap_or_else(|| "—".into()),
            );
        }
    }
    let _ = writeln!(out);

    // ---- Damage / death recap ----
    let _ = writeln!(out, "## Damage & Death Recap");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Actor deaths: {}", debrief.damage.actor_deaths);
    let _ = writeln!(out, "- Projectile hits: {}", debrief.damage.projectile_hits);
    let _ = writeln!(
        out,
        "- Total projectile damage delivered: {:.1}",
        debrief.damage.total_projectile_damage
    );
    let _ = writeln!(out, "- Reactor damage events: {}", debrief.damage.reactor_damage_events);
    if debrief.damage.reactor_destroyed {
        let tick = debrief
            .damage
            .reactor_destroyed_at_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "?".into());
        let _ = writeln!(out, "- Reactor destroyed: yes (at tick {tick})");
    } else {
        let _ = writeln!(out, "- Reactor destroyed: no");
    }
    let _ = writeln!(out);

    // ---- Terrain changes ----
    let _ = writeln!(out, "## Terrain Changes");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- `terrain_carved` events: {}",
        debrief.terrain.terrain_carved_events
    );
    let _ = writeln!(out, "- Total carved pixels: {}", debrief.terrain.total_carved_pixels);
    let _ = writeln!(
        out,
        "- `chunk_dirtied` events: {}",
        debrief.terrain.chunk_dirtied_events
    );
    if !debrief.terrain.by_material.is_empty() {
        let _ = writeln!(out, "- By material:");
        for (mat, count) in debrief.terrain.by_material.iter() {
            let _ = writeln!(out, "  - `{mat}`: {count}");
        }
    }
    let _ = writeln!(out);

    // ---- Key events ----
    let _ = writeln!(out, "## Key Events");
    let _ = writeln!(out);
    let _ = writeln!(out, "- Errors: {}", debrief.key_events.error_count);
    let _ = writeln!(out, "- Warnings: {}", debrief.key_events.warn_count);
    let _ = writeln!(out, "- Dropped events: {}", debrief.key_events.dropped_total);
    if !debrief.key_events.by_category.is_empty() {
        let _ = writeln!(out, "- By category:");
        for (cat, count) in debrief.key_events.by_category.iter() {
            let _ = writeln!(out, "  - `{cat}`: {count}");
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(out, "Top event types:");
    let _ = writeln!(out);
    let mut top: Vec<(&String, &u64)> = debrief.key_events.by_type.iter().collect();
    top.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (ty, count) in top.iter().take(8) {
        let _ = writeln!(out, "- `{ty}`: {count}");
    }
    let _ = writeln!(out);

    // ---- Checksum status ----
    let _ = writeln!(out, "## Checksum Status");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Algorithm: `{}` · Scope: `{}` · Cadence: every {} ticks",
        debrief.checksum.algorithm, debrief.checksum.scope, debrief.checksum.cadence_ticks,
    );
    match &debrief.checksum.final_checksum {
        Some(hex) => {
            let _ = writeln!(out, "- Final sim checksum: `{hex}`");
        }
        None => {
            let _ = writeln!(out, "- Final sim checksum: _none recorded_");
        }
    }
    let _ = writeln!(
        out,
        "- Checksum events emitted: {}",
        debrief.checksum.checksum_event_count
    );
    let _ = writeln!(
        out,
        "- First tick: {} · Last tick: {}",
        debrief
            .checksum
            .first_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into()),
        debrief
            .checksum
            .last_tick
            .map(|t| t.to_string())
            .unwrap_or_else(|| "—".into()),
    );
    let _ = writeln!(out);

    // ---- Cause chain (for losses only) — M10 § 18-section debrief ----
    let _ = writeln!(out, "## Cause Chain");
    let _ = writeln!(out);
    let lost = debrief.outcome.result.as_deref().map(|r| r == "lost").unwrap_or(false);
    if !lost {
        let _ = writeln!(
            out,
            "_N/A — mission did not end in a loss; no failure chain to explain._"
        );
    } else {
        let trigger = debrief.bundle.first_event_of_type("mission_resolved");
        match trigger {
            Some(trigger) => {
                let chain = crate::cause_chain::trace(debrief.bundle, trigger, crate::cause_chain::DEFAULT_MAX_DEPTH);
                let _ = writeln!(
                    out,
                    "Walk from `mission_resolved` back to the root cause (plain language):"
                );
                let _ = writeln!(out);
                for link in &chain.links {
                    let body = crate::renderer::render_event_body(link.event);
                    let _ = writeln!(out, "- tick {} — {}", link.event.tick, body);
                }
                let term_label = match chain.terminated_reason {
                    crate::cause_chain::ChainTermination::RootReached => "root reached",
                    crate::cause_chain::ChainTermination::ParentMissingFromBundle => {
                        "parent missing from bundle (partial bundle?)"
                    }
                    crate::cause_chain::ChainTermination::MaxDepthReached => "depth limit reached",
                    crate::cause_chain::ChainTermination::CycleDetected => "cycle detected (corrupt bundle)",
                };
                let _ = writeln!(out, "\nChain depth: {} · termination: {term_label}.", chain.links.len());
            }
            None => {
                let _ = writeln!(
                    out,
                    "_mission was lost per `result=lost` but no `mission_resolved` event was located_"
                );
            }
        }
    }
    let _ = writeln!(out);

    // ---- Accessibility surface — M10 § DR-012 audit trail ----
    let _ = writeln!(out, "## Accessibility Surface");
    let _ = writeln!(out);
    let s = &debrief.bundle.manifest.settings;
    let _ = writeln!(out, "- UI scale: `{}`", s.ui_scale);
    let _ = writeln!(out, "- High contrast: `{}`", s.high_contrast);
    let _ = writeln!(out, "- Captions: `{}`", s.captions);
    let _ = writeln!(out, "- Reduced motion: `{}`", s.reduced_motion);
    let _ = writeln!(out, "- Reduced shake: `{}`", s.reduced_shake);
    let _ = writeln!(out, "- Reduced flash: `{}`", s.reduced_flash);
    let _ = writeln!(out, "- Hold-to-confirm: `{}`", s.hold_to_confirm);
    let _ = writeln!(out, "- Hold threshold ms: `{}`", s.hold_threshold_ms);
    let _ = writeln!(out, "- Key remap enabled: `{}`", s.key_remap_enabled);
    let _ = writeln!(out, "- Key bindings: {}", s.key_bindings.len());
    let _ = writeln!(out);

    // ---- Recorder health — M10 § 18-section debrief ----
    let _ = writeln!(out, "## Recorder Health");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- Total events: {}",
        debrief
            .key_events
            .by_category
            .values()
            .sum::<u64>()
            .max(debrief.bundle.summary.event_counts.total)
    );
    let _ = writeln!(out, "- Dropped events: {}", debrief.key_events.dropped_total);
    let _ = writeln!(out, "- Error severity events: {}", debrief.key_events.error_count);
    let _ = writeln!(out, "- Warning severity events: {}", debrief.key_events.warn_count);
    let categories_active = debrief.key_events.by_category.len();
    let _ = writeln!(out, "- Categories with events: {}", categories_active);
    if debrief.key_events.dropped_total > 0 {
        let _ = writeln!(
            out,
            "- ⚠ Recorder dropped events under backpressure (cosmetic drops are expected)."
        );
    } else {
        let _ = writeln!(out, "- Recorder under capacity (0 drops)");
    }
    let _ = writeln!(out);

    // ---- Per-origin death recap — M17 origin reaction model ----
    let _ = writeln!(out, "## Per-Origin Death Recap");
    let _ = writeln!(out);
    render_death_recap(&mut out, &debrief.m17.deaths);
    let _ = writeln!(out);

    // markdown lists 18 mandated `##` sections. The M17 producers now fill
    // Resource Timeline / Internal Damage Breakdown / Origin Force Feedback
    // Summary with real rollups; the remaining headers ladder up at their own
    // milestones (armor M13, fluid/concussion M14, hazard/affliction M16,
    // atmos M19) and stay as stubs until then. Headers MUST appear so AI
    // Self-Test grading + spec literal assertions find them in any bundle.
    for section_stub in [
        ("Mission State", "_synthesizes objective progression + reactor state at run end_"),
        ("Resource Timeline", "_M17 resource cost trend (ladder up at M17)_"),
        ("Armor Durability", "_M13 per-zone armor layer hp summary (ladder up at M13)_"),
        ("Internal Damage Breakdown", "_M14 per-organ + per-circuit hp deltas (ladder up at M14)_"),
        ("Concussion Timeline", "_M17 concussion dose + band crossings_"),
        ("Fluid Drain Timeline", "_M14 fluid reservoir + leak history_"),
        ("Origin Force Feedback Summary", "_M17 per-origin G-load + helmet breach summary_"),
        ("Hazard Summary", "_M16 hazard spawns + actor contacts_"),
        ("Affliction Summary", "_M16 affliction applies + clears_"),
        ("Atmospheric Events", "_M19 atmos pressure / temperature / phase transitions_"),
    ] {
        let _ = writeln!(out, "## {}", section_stub.0);
        match section_stub.0 {
            "Resource Timeline" => render_resource_timeline(&mut out, &debrief.m17),
            "Internal Damage Breakdown" => render_internal_damage(&mut out, &debrief.m17),
            "Origin Force Feedback Summary" => render_force_feedback(&mut out, &debrief.m17),
            _ => {
                let _ = writeln!(out, "{}", section_stub.1);
            }
        }
        let _ = writeln!(out);
    }

    // ---- Captures section — M14 audit pass (GAP-M10-02 MEDIUM fix) ----
    // Spec § Captures section: "Given a bundle with captures/ — Then
    // `## Captures` lists each PNG by filename with type tag
    // (capture-frame / capture-grid / capture-summary-grid) + tick range".
    let _ = writeln!(out, "## Captures");
    let captures_dir = debrief.bundle.bundle_dir.join("captures");
    if captures_dir.is_dir() {
        let mut entries: Vec<std::path::PathBuf> = std::fs::read_dir(&captures_dir)
            .ok()
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.path())
                    .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("png"))
                    .collect()
            })
            .unwrap_or_default();
        entries.sort();
        if entries.is_empty() {
            let _ = writeln!(out, "- _no PNG captures in this run_");
        } else {
            for path in &entries {
                let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("(unnamed)");
                let kind = if name.contains("summary_grid") || name.contains("summary-grid") {
                    "capture-summary-grid"
                } else if name.contains("grid") {
                    "capture-grid"
                } else {
                    "capture-frame"
                };
                let _ = writeln!(out, "- `{name}` — {kind}");
            }
        }
    } else {
        let _ = writeln!(out, "- _no captures/ directory in this run bundle_");
    }
    let _ = writeln!(out);

    // ---- Thinking timeline (per-bot AI panel) — M10 § smart-AI surface ----
    let _ = writeln!(out, "## Thinking Timeline");
    let _ = writeln!(out);
    let died_actor = thinking_timeline_actor_id(debrief.bundle);
    match died_actor {
        Some(actor_id) => {
            let entries = crate::thinking_timeline::build_timeline(debrief.bundle, actor_id);
            // Spec: "render the killed actor's full thinking timeline for the
            // last 10 ticks before death". Use the death tick as the upper
            // bound and slice to last 10.
            let died_at_tick = debrief
                .bundle
                .events
                .iter()
                .find(|e| e.event_type == "actor_died" && event_actor_id(e) == Some(actor_id))
                .map(|e| e.tick);
            let entries = crate::thinking_timeline::slice_window(&entries, died_at_tick, Some(10));
            let panel = crate::thinking_timeline::render_markdown(actor_id, &entries);
            out.push_str(&panel);
        }
        None => {
            let _ = writeln!(out, "_no `actor_died` events in this bundle_");
        }
    }

    out
}

fn thinking_timeline_actor_id(bundle: &crate::bundle::Bundle) -> Option<u64> {
    bundle
        .events
        .iter()
        .filter(|e| e.event_type == "actor_died")
        .find_map(event_actor_id)
}

fn event_actor_id(event: &cf_replay::Event) -> Option<u64> {
    if let Some(id) = event.actor_id {
        return Some(id);
    }
    // Engine emits both "actor_id" and the shorter "actor" key on
    // various lifecycle events — see cf-control/src/engine.rs grep.
    if let Some(id) = event.payload.get("actor_id").and_then(|v| v.as_u64()) {
        return Some(id);
    }
    event.payload.get("actor").and_then(|v| v.as_u64())
}

/// Render a debrief as JSON for tooling. Used by `--json` flag.
pub fn render_json(debrief: &Debrief<'_>) -> serde_json::Value {
    let m = &debrief.bundle.manifest;
    let s = &debrief.bundle.summary;
    serde_json::json!({
        "run_id": m.run_id,
        "scenario": {
            "id": m.scene.id,
            "display_name": m.scene.display_name,
        },
        "milestone": m.milestone,
        "prototype_slice": m.prototype_slice,
        "tick_rate_hz": m.tick_rate_hz,
        "seed": m.seed,
        "duration_sec": s.duration_sec,
        "ticks_run": s.performance.ticks_run,
        "result": s.result,
        "exit_code": s.exit_code,
        "outcome": {
            "result": debrief.outcome.result,
            "reason": debrief.outcome.reason,
            "resolved_at_tick": debrief.outcome.resolved_at_tick,
            "resolved_event_id": debrief.outcome.resolved_event_id,
        },
        "objectives": debrief.objectives.iter().map(|o| {
            serde_json::json!({
                "objective": o.objective,
                "state": o.state.label(),
                "started_at_tick": o.started_at_tick,
                "ended_at_tick": o.ended_at_tick,
            })
        }).collect::<Vec<_>>(),
        "damage": {
            "actor_deaths": debrief.damage.actor_deaths,
            "projectile_hits": debrief.damage.projectile_hits,
            "total_projectile_damage": debrief.damage.total_projectile_damage,
            "reactor_damage_events": debrief.damage.reactor_damage_events,
            "reactor_destroyed": debrief.damage.reactor_destroyed,
            "reactor_destroyed_at_tick": debrief.damage.reactor_destroyed_at_tick,
        },
        "terrain": {
            "terrain_carved_events": debrief.terrain.terrain_carved_events,
            "total_carved_pixels": debrief.terrain.total_carved_pixels,
            "chunk_dirtied_events": debrief.terrain.chunk_dirtied_events,
            "by_material": debrief.terrain.by_material,
        },
        "key_events": {
            "error_count": debrief.key_events.error_count,
            "warn_count": debrief.key_events.warn_count,
            "dropped_total": debrief.key_events.dropped_total,
            "by_category": debrief.key_events.by_category,
            "by_type": debrief.key_events.by_type,
        },
        "checksum": {
            "algorithm": debrief.checksum.algorithm,
            "scope": debrief.checksum.scope,
            "cadence_ticks": debrief.checksum.cadence_ticks,
            "final_checksum": debrief.checksum.final_checksum,
            "checksum_event_count": debrief.checksum.checksum_event_count,
            "first_tick": debrief.checksum.first_tick,
            "last_tick": debrief.checksum.last_tick,
        },
        "m17": {
            "deaths": debrief.m17.deaths.iter().map(|d| {
                serde_json::json!({
                    "actor_id": d.actor_id,
                    "origin_id": d.origin_id,
                    "class": d.class.map(|c| c.label()),
                    "died_at_tick": d.died_at_tick,
                    "organs_destroyed": d.organs_destroyed,
                    "organ_failures": d.organ_failures,
                    "concussion_band": d.concussion_band,
                    "concussion_ko": d.concussion_ko,
                    "bled_out": d.bled_out,
                    "bleeding": d.bleeding,
                    "circuits_destroyed": d.circuits_destroyed,
                    "circuit_failures": d.circuit_failures,
                    "internal_shock_hits": d.internal_shock_hits,
                    "modules_damaged": d.modules_damaged,
                    "power_depleted": d.power_depleted,
                    "went_offline": d.went_offline,
                    "offline_reason": d.offline_reason,
                    "thermal_cascade": d.thermal_cascade,
                })
            }).collect::<Vec<_>>(),
            "resource_timeline": debrief.m17.resource_timeline.iter().map(|(kind, stat)| {
                (kind.clone(), serde_json::json!({
                    "changed": stat.changed,
                    "critical": stat.critical,
                    "depleted": stat.depleted,
                }))
            }).collect::<serde_json::Map<String, serde_json::Value>>(),
            "force_feedback": {
                "total": debrief.m17.force_feedback.total,
                "by_feedback_kind": debrief.m17.force_feedback.by_feedback_kind,
                "helmet_breaches": debrief.m17.force_feedback.helmet_breaches,
                "helmet_breach_actors": debrief.m17.force_feedback.helmet_breach_actors.iter().copied().collect::<Vec<u64>>(),
                "g_load_total": debrief.m17.force_feedback.g_load_total,
                "g_load_by_actor": debrief.m17.force_feedback.g_load_by_actor.iter().map(|(k, v)| (k.to_string(), serde_json::json!(*v))).collect::<serde_json::Map<String, serde_json::Value>>(),
            },
            "internal_damage": {
                "organs_damaged": debrief.m17.internal_damage.organs_damaged,
                "organs_destroyed": debrief.m17.internal_damage.organs_destroyed,
                "circuits_damaged": debrief.m17.internal_damage.circuits_damaged,
                "circuits_destroyed": debrief.m17.internal_damage.circuits_destroyed,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn build_bundle(run_id: &str, events: &[serde_json::Value], summary_overrides: serde_json::Value) -> Bundle {
        let mut p = std::env::temp_dir();
        p.push(format!("cf_replay_viewer_debrief_{}_{}", run_id, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let manifest = serde_json::json!({
            "schema_version": "prototype-run-manifest.v0.1",
            "run_id": run_id,
            "prototype_slice": "M3B",
            "run_mode": "test",
            "milestone": "m3b",
            "build": {"commit_sha": "deadbeef", "rust_version": "x", "bevy_version": "y", "platform": "test"},
            "scene": {"id": "synth", "display_name": "synth", "source_path": "x"},
            "seed": 42,
            "started_at_utc": "2026-05-09T00:00:00Z",
            "duration_target_sec": 1.0,
            "material_schema_version": "n/a",
            "config_hash": "abc",
            "assumptions_tested": [],
            "linked_specs": [],
            "expected_tests": [],
            "capture_config": {"events": true, "screenshots": false, "captures": false},
            "schemas": {"control": 1, "scenario": 1, "events": 1},
            "capabilities": {"debug": false, "control_api": true, "save_load": false, "debug_capabilities": []},
            "settings": {"ui_scale": 1.0, "high_contrast": false, "captions": true, "reduced_motion": false, "reduced_shake": false, "reduced_flash": false},
            "checksum": {"algorithm": "blake3", "scope": "sim_state_v1", "cadence_ticks": 60},
            "tick_rate_hz": 60,
            "expected_outcome": "clean"
        });
        std::fs::write(
            p.join("run_manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        let mut by_category: BTreeMap<String, u64> = BTreeMap::new();
        let mut by_type: BTreeMap<String, u64> = BTreeMap::new();
        for ev in events.iter() {
            let cat = ev.get("category").and_then(|v| v.as_str()).unwrap().to_string();
            let ty = ev.get("event_type").and_then(|v| v.as_str()).unwrap().to_string();
            *by_category.entry(cat).or_insert(0) += 1;
            *by_type.entry(ty).or_insert(0) += 1;
        }
        let mut summary = serde_json::json!({
            "schema_version": "prototype-run-summary.v0.1",
            "run_id": run_id,
            "manifest_run_id": run_id,
            "duration_sec": 1.0,
            "result": "pass",
            "ended_at_utc": "2026-05-09T00:00:01Z",
            "exit_code": 0,
            "event_counts": {
                "total": events.len(),
                "by_category": by_category,
                "by_type": by_type,
                "by_severity": {"error": 0, "warn": 0},
                "dropped_total": 0
            },
            "volume": {"events_jsonl_bytes": 0, "event_lines": events.len()},
            "performance": {"avg_frame_ms": 0.0, "p99_frame_ms": 0.0, "avg_tick_ms": 0.0, "p99_tick_ms": 0.0, "ticks_run": 0, "wall_seconds": 0.0, "tick_rate_hz": 60},
            "artifacts": {"items": []},
            "blockers": [],
            "next_actions": [],
            "tests": [],
            "final_sim_checksum": null,
            "checksum_event_count": 0,
            "first_tick": 0,
            "last_tick": 0
        });
        if let serde_json::Value::Object(ref mut map) = summary {
            if let serde_json::Value::Object(over) = summary_overrides {
                for (k, v) in over {
                    map.insert(k, v);
                }
            }
        }
        std::fs::write(p.join("summary.json"), serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
        let mut events_file = std::fs::File::create(p.join("events.jsonl")).unwrap();
        for ev in events.iter() {
            writeln!(events_file, "{}", serde_json::to_string(ev).unwrap()).unwrap();
        }
        drop(events_file);
        Bundle::load(&p).expect("debrief test bundle loads")
    }

    #[test]
    fn debrief_extracts_outcome_for_won_mission() {
        let run_id = "debrief_won";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 100, "sim_time_ms": 1666.0, "event_id": format!("{run_id}:100:1"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "won"}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        assert_eq!(d.outcome.result.as_deref(), Some("won"));
        assert_eq!(d.outcome.resolved_at_tick, Some(100));
        let md = render_markdown(&d);
        assert!(md.contains("Result: `won`"));
    }

    #[test]
    fn debrief_extracts_outcome_for_lost_mission_with_reason() {
        let run_id = "debrief_lost";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 50, "sim_time_ms": 833.0, "event_id": format!("{run_id}:50:1"), "category": "mission", "event_type": "objective_started", "payload": {"objective": "defend_reactor"}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 1095, "sim_time_ms": 18250.0, "event_id": format!("{run_id}:1095:2"), "category": "mission", "event_type": "objective_failed", "payload": {"objective": "defend_reactor"}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 1095, "sim_time_ms": 18250.0, "event_id": format!("{run_id}:1095:3"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "lost", "reason": "reactor_destroyed"}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        assert_eq!(d.outcome.result.as_deref(), Some("lost"));
        assert_eq!(d.outcome.reason.as_deref(), Some("reactor_destroyed"));
        assert_eq!(d.objectives.len(), 1);
        assert_eq!(d.objectives[0].state, ObjectiveState::Failed);
        let md = render_markdown(&d);
        assert!(md.contains("Result: `lost`"));
        assert!(md.contains("reason: `reactor_destroyed`"));
        assert!(md.contains("`defend_reactor`"));
        assert!(md.contains("failed"));
    }

    #[test]
    fn debrief_aggregates_damage_terrain_and_checksum() {
        let run_id = "debrief_full";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 10, "sim_time_ms": 166.0, "event_id": format!("{run_id}:10:1"), "category": "terrain", "event_type": "terrain_carved", "payload": {"material": "dirt", "count": 100}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 20, "sim_time_ms": 333.0, "event_id": format!("{run_id}:20:2"), "category": "terrain", "event_type": "chunk_dirtied", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 30, "sim_time_ms": 500.0, "event_id": format!("{run_id}:30:3"), "category": "combat", "event_type": "projectile_hit", "payload": {"damage": 8.0, "target": 1}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 40, "sim_time_ms": 666.0, "event_id": format!("{run_id}:40:4"), "category": "combat", "event_type": "reactor_damaged", "payload": {"hp_after": 0, "destroyed": true}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 41, "sim_time_ms": 683.0, "event_id": format!("{run_id}:41:5"), "category": "actor", "event_type": "actor_died", "payload": {"actor": 1, "cause": "projectile"}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 50, "sim_time_ms": 833.0, "event_id": format!("{run_id}:50:6"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "lost", "reason": "reactor_destroyed"}}),
        ];
        let summary_overrides = serde_json::json!({
            "final_sim_checksum": "abcd1234deadbeef",
            "checksum_event_count": 5,
            "first_tick": 0,
            "last_tick": 50
        });
        let bundle = build_bundle(run_id, &events, summary_overrides);
        let d = compose(&bundle);
        assert_eq!(d.damage.actor_deaths, 1);
        assert_eq!(d.damage.projectile_hits, 1);
        assert_eq!(d.damage.total_projectile_damage as i64, 8);
        assert!(d.damage.reactor_destroyed);
        assert_eq!(d.damage.reactor_destroyed_at_tick, Some(40));
        assert_eq!(d.terrain.terrain_carved_events, 1);
        assert_eq!(d.terrain.total_carved_pixels, 100);
        assert_eq!(d.terrain.by_material.get("dirt"), Some(&1));
        assert_eq!(d.checksum.final_checksum.as_deref(), Some("abcd1234deadbeef"));
        let md = render_markdown(&d);
        for h in [
            "## Outcome",
            "## Objectives",
            "## Damage & Death Recap",
            "## Terrain Changes",
            "## Key Events",
            "## Checksum Status",
            "## Cause Chain",
            "## Accessibility Surface",
            "## Recorder Health",
            "## Mission State",
            "## Resource Timeline",
            "## Armor Durability",
            "## Internal Damage Breakdown",
            "## Concussion Timeline",
            "## Fluid Drain Timeline",
            "## Origin Force Feedback Summary",
            "## Hazard Summary",
            "## Affliction Summary",
            "## Atmospheric Events",
            "## Captures",
            "## Thinking Timeline",
        ] {
            assert!(md.contains(h), "debrief markdown missing heading {h}");
        }
        assert!(md.contains("abcd1234deadbeef"));
        assert!(md.contains("Reactor destroyed: yes"));
    }

    #[test]
    fn debrief_renders_unresolved_mission_gracefully() {
        let run_id = "debrief_unresolved";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 1, "sim_time_ms": 16.0, "event_id": format!("{run_id}:1:1"), "category": "system", "event_type": "run_finished", "payload": {}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        assert!(d.outcome.result.is_none());
        let md = render_markdown(&d);
        assert!(md.contains("not resolved"));
    }

    #[test]
    fn debrief_json_round_trip_has_expected_keys() {
        let run_id = "debrief_json";
        let events = vec![
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 0, "sim_time_ms": 0.0, "event_id": format!("{run_id}:0:0"), "category": "system", "event_type": "run_started", "payload": {}}),
            serde_json::json!({"schema_version": "prototype-recorder-event.v0.1", "run_id": run_id, "tick": 100, "sim_time_ms": 1666.0, "event_id": format!("{run_id}:100:1"), "category": "mission", "event_type": "mission_resolved", "payload": {"result": "won"}}),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);
        let json = render_json(&d);
        for k in [
            "run_id",
            "scenario",
            "outcome",
            "objectives",
            "damage",
            "terrain",
            "key_events",
            "checksum",
            "m17",
        ] {
            assert!(json.get(k).is_some(), "json missing key {k}");
        }
    }

    fn ev(run_id: &str, tick: u64, seq: u64, category: &str, event_type: &str, payload: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "schema_version": "prototype-recorder-event.v0.1",
            "run_id": run_id,
            "tick": tick,
            "sim_time_ms": tick as f64 * 16.0,
            "event_id": format!("{run_id}:{tick}:{seq}"),
            "category": category,
            "event_type": event_type,
            "payload": payload,
        })
    }

    fn death_recap_line(md: &str, actor_id: u64) -> String {
        let needle = format!("- actor #{actor_id} (");
        md.lines()
            .find(|l| l.starts_with(&needle))
            .unwrap_or_else(|| panic!("death recap line for actor {actor_id} missing in:\n{md}"))
            .to_string()
    }

    #[test]
    fn debrief_m17_human_death_uses_organic_vocab() {
        let run_id = "debrief_m17_human";
        let events = vec![
            ev(run_id, 0, 0, "system", "run_started", serde_json::json!({})),
            ev(run_id, 5, 1, "origin", "shot_force_feedback", serde_json::json!({"actor_id": 1, "origin_id": "Human", "feedback_kind": "pain_jolt", "frame_ring": false, "g_load_delta": 4.0})),
            ev(run_id, 6, 2, "concussion", "dose_changed", serde_json::json!({"actor_id": 1, "from_dose": 0.0, "to_dose": 40.0, "origin_id": "Human"})),
            ev(run_id, 7, 3, "concussion", "band_changed", serde_json::json!({"actor_id": 1, "from_band": "Clear", "to_band": "Severe", "dose": 40.0})),
            ev(run_id, 8, 4, "concussion", "ko_threshold_crossed", serde_json::json!({"actor_id": 1})),
            ev(run_id, 9, 5, "internal", "organ_damaged", serde_json::json!({"actor_id": 1, "organ_id": "heart", "organ_kind": "heart", "from_hp": 100.0, "to_hp": 0.0})),
            ev(run_id, 10, 6, "internal", "organ_destroyed", serde_json::json!({"actor_id": 1, "organ_id": "heart", "organ_kind": "heart"})),
            ev(run_id, 11, 7, "internal", "organ_failure_cascade", serde_json::json!({"actor_id": 1, "organ_id": "heart", "organ_kind": "heart", "affliction_kind": "cardiac_arrest", "severity": 1.0})),
            ev(run_id, 12, 8, "affliction", "applied", serde_json::json!({"actor_id": 1, "kind": "bleeding", "severity_0_1": 0.7})),
            ev(run_id, 13, 9, "resource", "changed", serde_json::json!({"actor_id": 1, "kind": "blood", "from": 5000.0, "to": 1000.0})),
            ev(run_id, 14, 10, "resource", "critical", serde_json::json!({"actor_id": 1, "kind": "blood", "threshold_pct": 10.0, "current": 500.0})),
            ev(run_id, 15, 11, "resource", "depleted", serde_json::json!({"actor_id": 1, "kind": "blood"})),
            ev(run_id, 16, 12, "actor", "actor_died", serde_json::json!({"actor": 1, "cause": "bled_out"})),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);

        assert_eq!(d.m17.deaths.len(), 1);
        let death = &d.m17.deaths[0];
        assert_eq!(death.actor_id, 1);
        assert_eq!(death.class, Some(OriginClass::Organic));
        assert_eq!(death.origin_id.as_deref(), Some("Human"));
        assert!(death.bled_out);
        assert!(death.concussion_ko);
        assert_eq!(d.m17.internal_damage.organs_destroyed, 1);
        assert_eq!(d.m17.resource_timeline.get("blood").map(|s| s.depleted), Some(1));
        assert_eq!(d.m17.force_feedback.by_feedback_kind.get("pain_jolt"), Some(&1));

        let md = render_markdown(&d);
        assert!(md.contains("## Per-Origin Death Recap"));
        let line = death_recap_line(&md, 1);
        for present in ["organic", "organ", "concussion", "bled out"] {
            assert!(line.contains(present), "organic line missing `{present}`: {line}");
        }
        for forbidden in ["circuit", "offline", "thermal", "servo"] {
            assert!(!line.contains(forbidden), "organic line leaked synthetic word `{forbidden}`: {line}");
        }
    }

    #[test]
    fn debrief_m17_robot_death_uses_synthetic_vocab() {
        let run_id = "debrief_m17_robot";
        let events = vec![
            ev(run_id, 0, 0, "system", "run_started", serde_json::json!({})),
            ev(run_id, 5, 1, "origin", "shot_force_feedback", serde_json::json!({"actor_id": 2, "origin_id": "Robot", "feedback_kind": "servo_jolt", "frame_ring": true, "g_load_delta": 0.0, "internal_shock_module_id": "power_core"})),
            ev(run_id, 6, 2, "internal_shock", "dose_changed", serde_json::json!({"actor_id": 2, "from_dose": 0.0, "to_dose": 30.0})),
            ev(run_id, 7, 3, "internal", "circuit_damaged", serde_json::json!({"actor_id": 2, "circuit_id": "power_core", "circuit_kind": "power", "from_hp": 100.0, "to_hp": 0.0})),
            ev(run_id, 8, 4, "internal", "circuit_destroyed", serde_json::json!({"actor_id": 2, "circuit_id": "power_core", "circuit_kind": "power"})),
            ev(run_id, 9, 5, "internal", "circuit_failure_cascade", serde_json::json!({"actor_id": 2, "circuit_id": "power_core", "circuit_kind": "power", "affliction_kind": "power_failure", "severity": 1.0})),
            ev(run_id, 10, 6, "chassis", "thermal_throttle_started", serde_json::json!({"actor_id": 2, "heat": 0.9, "action_speed_factor": 0.6})),
            ev(run_id, 11, 7, "resource", "changed", serde_json::json!({"actor_id": 2, "kind": "power", "from": 100.0, "to": 10.0})),
            ev(run_id, 12, 8, "resource", "depleted", serde_json::json!({"actor_id": 2, "kind": "power"})),
            ev(run_id, 13, 9, "resource", "cascade_offline", serde_json::json!({"actor_id": 2, "kind": "power", "organ_id": "power_core", "reason": "circuit_destroyed"})),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);

        assert_eq!(d.m17.deaths.len(), 1);
        let death = &d.m17.deaths[0];
        assert_eq!(death.actor_id, 2);
        assert_eq!(death.class, Some(OriginClass::Synthetic));
        assert_eq!(death.origin_id.as_deref(), Some("Robot"));
        assert!(death.went_offline);
        assert!(death.thermal_cascade);
        assert_eq!(death.internal_shock_hits, 1);
        assert_eq!(d.m17.internal_damage.circuits_destroyed, 1);
        assert_eq!(d.m17.resource_timeline.get("power").map(|s| s.depleted), Some(1));
        assert_eq!(d.m17.force_feedback.by_feedback_kind.get("servo_jolt"), Some(&1));

        let md = render_markdown(&d);
        let line = death_recap_line(&md, 2);
        for present in ["synthetic", "circuit", "went offline", "thermal cascade"] {
            assert!(line.contains(present), "synthetic line missing `{present}`: {line}");
        }
        for forbidden in ["bled", "organ", "concussion", "blood"] {
            assert!(!line.contains(forbidden), "synthetic line leaked organic word `{forbidden}`: {line}");
        }
    }

    #[test]
    fn debrief_m17_force_feedback_and_resource_rollups() {
        let run_id = "debrief_m17_rollup";
        let events = vec![
            ev(run_id, 0, 0, "system", "run_started", serde_json::json!({})),
            ev(run_id, 1, 1, "origin", "shot_force_feedback", serde_json::json!({"actor_id": 1, "origin_id": "Human", "feedback_kind": "pain_jolt", "g_load_delta": 2.5})),
            ev(run_id, 2, 2, "origin", "shot_force_feedback", serde_json::json!({"actor_id": 1, "origin_id": "Human", "feedback_kind": "pain_jolt", "g_load_delta": 1.5})),
            ev(run_id, 3, 3, "origin", "helmet_breach", serde_json::json!({"actor_id": 1, "helmet_item_id": 0, "breach_pos": [0.0, 0.0], "oxygen_loss_rate": 3.0})),
            ev(run_id, 4, 4, "resource", "changed", serde_json::json!({"actor_id": 1, "kind": "oil", "from": 100.0, "to": 50.0})),
            ev(run_id, 5, 5, "resource", "critical", serde_json::json!({"actor_id": 1, "kind": "oil", "threshold_pct": 10.0, "current": 5.0})),
        ];
        let bundle = build_bundle(run_id, &events, serde_json::json!({}));
        let d = compose(&bundle);

        assert_eq!(d.m17.force_feedback.total, 2);
        assert_eq!(d.m17.force_feedback.by_feedback_kind.get("pain_jolt"), Some(&2));
        assert_eq!(d.m17.force_feedback.helmet_breaches, 1);
        assert_eq!(d.m17.force_feedback.helmet_breach_actors.len(), 1);
        assert!((d.m17.force_feedback.g_load_total - 4.0).abs() < 1e-6);
        let oil = d.m17.resource_timeline.get("oil").expect("oil tally");
        assert_eq!(oil.changed, 1);
        assert_eq!(oil.critical, 1);
        assert!(d.m17.deaths.is_empty());

        let md = render_markdown(&d);
        assert!(md.contains("## Per-Origin Death Recap"));
        assert!(md.contains("_no actor deaths recorded_"));
        assert!(md.contains("Cumulative g-load dose: 4.0"));
        assert!(md.contains("Helmet breaches: 1"));
    }
}
