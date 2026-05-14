//! M2: minimal objective state machine.
//!
//! (Historical name: M1.5 — renamed to M2 in the canonical roadmap. Code
//! comments may still say "M1.5"; they refer to the same milestone whose
//! spec lives at `specs/done/M2.md`.)
//!
//! This crate owns the `MissionState` + `Objective` types used by the M2 micro
//! breach scenario. The full mission director (typed manifest, command-core, base
//! power, commander AI, comic-noir cards) lands at M7. M2 only needs:
//!
//! - A small ordered list of objectives the player must clear in turn (or fail).
//! - A win/loss decision per tick (player dead, timer expired, or all required
//!   objectives complete).
//! - A `MissionTickReport` the engine turns into `mission.*` recorder events with
//!   the same field shape M7's full director will emit.
//!
//! The crate is deterministic and pure: every public mutator is a `&mut self` step
//! that consumes structured inputs and returns a report, so replay parity is the
//! caller's wiring contract — not this crate's.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::redundant_closure,
    clippy::derivable_impls,
    clippy::wildcard_in_or_patterns,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::match_wildcard_for_single_variants,
    clippy::trivially_copy_pass_by_ref,
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    clippy::single_match_else,
    // M7 director v0.5 additions: phases/reinforcement/boss/graph modules
    // trip a handful of additional pedantic lints whose remedy doesn't add
    // clarity (inherent `from_str`, similar names across enums, etc.).
    clippy::should_implement_trait,
    clippy::explicit_iter_loop,
    clippy::needless_lifetimes,
    clippy::similar_names,
    clippy::map_unwrap_or
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

// M2 spec "## Files" wiring: re-export the canonical types via thin
// modules so consumers that import per the spec path (`cf_mission::objective::*`
// / `cf_mission::director::*`) compile cleanly.
pub mod director;
pub mod objective;

// **M7**: Mission director v0.5 — additive multi-objective DiGraph + 4-phase
// pacing + reinforcement waves + mini-boss patterns. The M2 single-vec
// objective list keeps working unchanged; M7 layers the v0.5 graph on top
// so scenarios can opt in.
pub mod boss_phases;
pub mod objective_graph;
pub mod phases;
pub mod reinforcement;

pub use boss_phases::{BossPhase, BossPhaseChangedEvent, BossSpecialAbilityEvent, BossState};
pub use objective_graph::{
    BranchingPoint, ExtendedObjectiveKind, ObjectiveBranchedEvent, ObjectiveGraph, ObjectiveNode, ObjectiveNodeStatus,
    OptionalOfferedEvent,
};
pub use phases::{MissionPhase, PhaseChangedEvent, PhaseState};
pub use reinforcement::{ReinforcementRegistry, ReinforcementWave, ReinforcementWaveSpawnedEvent};

use cf_actor::{ActorId, ActorState, Status};

/// One objective the player must clear (or, if `optional`, may skip).
///
/// M1.5 supports three kinds:
///
/// - `BreachBarrier { target }` — break the breach strip with the given id.
/// - `NeutralizeActor { target }` — drive the named actor to `Status::Dead`.
/// - `ReachZone { min, max }` — the player's position lies inside the AABB.
///
/// M7's full mission director extends this to typed mission manifests; the
/// objective ids and status names ship unchanged so M1.5 evidence stays valid.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Objective {
    pub id: String,
    pub kind: ObjectiveKind,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub status: ObjectiveStatus,
    /// **M1.5**: highest progress milestone emitted so far for this objective.
    /// 0 = none, 1 = 25%, 2 = 50%, 3 = 75%, 4 = 100% (the 100% milestone fires
    /// in lockstep with `objective_completed`). Tracked so `mission.objective_updated`
    /// fires once per crossed quartile.
    #[serde(default)]
    pub progress_milestone_index: u8,
    /// **M2 re-audit (2026-05-13)**: continuous progress fraction (0.0..1.0).
    /// PROGRESS_QUARTILES = [0.25, 0.5, 0.75, 1.0] drives the M2 quartile
    /// event emission. Mirrors the spec's `Objective.progress: f32`.
    #[serde(default)]
    pub progress: f32,
    /// **M2 re-audit (2026-05-13)**: optional fail-sensor descriptor per
    /// the spec literal `Objective { id, kind, status, progress, fail_sensor }`.
    /// `None` for objectives without an explicit fail-sensor (the kind's
    /// implicit fail-sensor still applies — e.g. DefendReactor fails on
    /// reactor destruction). M7+ uses this for declarative fail-sensors
    /// (`FailSensor::TimerWindow { from_tick, threshold_ticks }`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fail_sensor: Option<FailSensor>,
}

/// **M2 re-audit (2026-05-13)**: declarative fail-sensor descriptor. M7+
/// extends with richer sensors; M2 ships the type so scenario manifests can
/// reference it without a schema bump.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FailSensor {
    /// Fail if `current_tick - from_tick > threshold_ticks`.
    TimerWindow { from_tick: u64, threshold_ticks: u64 },
    /// Fail when the target actor's HP reaches zero.
    ActorHpZero { target: u64 },
}

/// Kind of objective. Discriminator names match the canonical roadmap glossary so
/// M7's typed manifest can read M2 scenario files without migrating ids.
///
/// **M2 re-audit pass 3 (2026-05-13)**: spec literal at M2.md line 109 calls
/// for `ReachZone, KillActor, SurviveTimer, DefendActor, EscortActor`. The
/// codebase predates the spec rename so `NeutralizeActor` / `DefendReactor`
/// are still the canonical Rust identifiers — but `kill_actor` and
/// `defend_actor` are accepted as JSON discriminator aliases via `serde(alias)`
/// so scenario manifests authored against the spec deserialize cleanly. The
/// underlying behaviour is identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectiveKind {
    BreachBarrier {
        target: String,
    },
    #[serde(alias = "kill_actor")]
    NeutralizeActor {
        target: u64,
    },
    ReachZone {
        min: [f32; 2],
        max: [f32; 2],
    },
    /// M2.5: defend a reactor (named static actor) until either the mission
    /// timer expires (success) or the reactor's hp reaches zero (failure).
    /// `target` is the reactor id. The `defend_actor` JSON discriminator
    /// (M2 spec literal) is accepted as an alias.
    #[serde(alias = "defend_actor")]
    DefendReactor {
        target: String,
    },
    /// **M2 re-audit (2026-05-13)**: spec literal — "ObjectiveKind enum:
    /// ReachZone, KillActor, SurviveTimer, DefendActor, EscortActor". The
    /// variant completes when `current_tick - started_at_tick >= survive_ticks`
    /// AND the actor is still alive.
    SurviveTimer {
        survive_ticks: u64,
    },
    /// **M2 re-audit (2026-05-13)**: escort `target` actor until they
    /// reach `destination` AABB. Fails if `target` dies during transit.
    EscortActor {
        target: u64,
        destination_min: [f32; 2],
        destination_max: [f32; 2],
    },
}

impl ObjectiveKind {
    pub fn category(&self) -> &'static str {
        match self {
            ObjectiveKind::BreachBarrier { .. } => "breach_barrier",
            ObjectiveKind::NeutralizeActor { .. } => "neutralize_actor",
            ObjectiveKind::ReachZone { .. } => "reach_zone",
            ObjectiveKind::DefendReactor { .. } => "defend_reactor",
            ObjectiveKind::SurviveTimer { .. } => "survive_timer",
            ObjectiveKind::EscortActor { .. } => "escort_actor",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectiveStatus {
    Pending,
    Active,
    Completed,
    Failed,
}

impl Default for ObjectiveStatus {
    fn default() -> Self {
        ObjectiveStatus::Pending
    }
}

impl ObjectiveStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ObjectiveStatus::Pending => "pending",
            ObjectiveStatus::Active => "active",
            ObjectiveStatus::Completed => "completed",
            ObjectiveStatus::Failed => "failed",
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, ObjectiveStatus::Completed | ObjectiveStatus::Failed)
    }
}

/// Mission outcome reason once `Lost`. M1.5 only needs two reasons; M7 adds more
/// (objective_failed, ally_lost, command_core_destroyed, etc.).
///
/// **M2 re-audit (2026-05-13)**: `ObjectiveFailed` now carries the failing
/// objective id + a reason label per the spec literal "ObjectiveFailed {
/// id, reason }". `Aborted` variant added so the abort path doesn't have
/// to route through a raw string literal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LossReason {
    PlayerDead,
    TimerExpired,
    /// M2.5: a `defend_reactor` objective failed because the reactor was
    /// destroyed before the mission timer expired.
    ReactorDestroyed,
    /// M2: a player-tracked objective failed.
    ObjectiveFailed {
        id: String,
        reason: String,
    },
    /// M2: player-initiated mission abandonment via `act.player.abort`.
    Aborted,
}

impl LossReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            LossReason::PlayerDead => "player_dead",
            LossReason::TimerExpired => "timer_expired",
            LossReason::ReactorDestroyed => "reactor_destroyed",
            LossReason::ObjectiveFailed { .. } => "objective_failed",
            LossReason::Aborted => "aborted",
        }
    }

    /// **M2 re-audit (2026-05-13)**: when `ObjectiveFailed`, returns the
    /// failing objective id; otherwise `None`. Used by replay viewers and
    /// debrief markdown.
    pub fn objective_id(&self) -> Option<&str> {
        match self {
            LossReason::ObjectiveFailed { id, .. } => Some(id),
            _ => None,
        }
    }

    /// **M2 re-audit (2026-05-13)**: when `ObjectiveFailed`, returns the
    /// failure reason label; otherwise `None`.
    pub fn objective_reason(&self) -> Option<&str> {
        match self {
            LossReason::ObjectiveFailed { reason, .. } => Some(reason),
            _ => None,
        }
    }
}

/// **M2 re-audit (2026-05-13)**: mission lifecycle state machine per spec
/// literal "Mission state machine: Init → Loaded → InProgress → Resolved".
/// Independent from `MissionResult` — `MissionResult` is the OUTCOME shape
/// when `lifecycle == Resolved`. Transitions:
/// - `Init` → `Loaded` on scenario load (MissionState constructed)
/// - `Loaded` → `InProgress` on first tick / mission_started event
/// - `InProgress` → `Resolved` on mission_resolved event (Won/Lost/Aborted)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MissionLifecycle {
    /// Pre-scenario-load (no MissionState exists in this state — kept for
    /// symmetry with the spec wording).
    Init,
    /// Scenario loaded; objectives present; tick 0 not yet fired.
    #[default]
    Loaded,
    /// `mission.mission_started` event has fired.
    InProgress,
    /// `mission.mission_resolved` event has fired. The resolution shape
    /// (Won / Lost / Aborted) lives on `MissionState.result`.
    Resolved,
}

impl MissionLifecycle {
    pub fn as_str(self) -> &'static str {
        match self {
            MissionLifecycle::Init => "init",
            MissionLifecycle::Loaded => "loaded",
            MissionLifecycle::InProgress => "in_progress",
            MissionLifecycle::Resolved => "resolved",
        }
    }
}

/// **M2 re-audit (2026-05-13)**: renamed `Active → InProgress` per the
/// spec literal "MissionResult::{InProgress, Won, Lost, Aborted}".
/// `serde(rename_all = "snake_case")` makes the wire value `"in_progress"`
/// — the prior `"active"` was renamed in lockstep.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "result")]
pub enum MissionResult {
    InProgress,
    Won,
    Lost {
        reason: LossReason,
    },
    /// Player-initiated mission abandonment via `act.player.abort`.
    Aborted,
}

impl MissionResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissionResult::InProgress => "in_progress",
            MissionResult::Won => "won",
            MissionResult::Lost { .. } => "lost",
            MissionResult::Aborted => "aborted",
        }
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self, MissionResult::InProgress)
    }
}

/// Loss conditions for the M1.5 micro breach scenario. M7 will replace this with
/// the typed mission director's failure graph.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LossConditions {
    /// True if the player dying ends the mission as Lost(`PlayerDead`).
    #[serde(default = "default_true")]
    pub player_dead: bool,
    /// Optional time limit in ticks. `0` = no limit.
    #[serde(default)]
    pub time_limit_ticks: u64,
}

fn default_true() -> bool {
    true
}

impl Default for LossConditions {
    fn default() -> Self {
        Self {
            player_dead: true,
            time_limit_ticks: 0,
        }
    }
}

/// One reactor entry the engine tracks as a damageable static actor. The engine
/// projects current hp + destroyed flag into [`MissionTickInputs::reactors`] so
/// `defend_reactor` objectives can detect destruction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reactor {
    pub id: String,
    pub position: [f32; 2],
    pub half_extents: [f32; 2],
    pub hp: f32,
    pub max_hp: f32,
    /// True once `hp <= 0.0`. Latched: a reactor cannot un-destroy itself.
    #[serde(default)]
    pub destroyed: bool,
}

impl Reactor {
    pub fn is_destroyed(&self) -> bool {
        self.destroyed || self.hp <= 0.0
    }

    /// True if `(x, y)` is inside the reactor's AABB.
    pub fn aabb_contains(&self, x: f32, y: f32) -> bool {
        let min_x = self.position[0] - self.half_extents[0];
        let max_x = self.position[0] + self.half_extents[0];
        let min_y = self.position[1] - self.half_extents[1];
        let max_y = self.position[1] + self.half_extents[1];
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }

    /// Apply `damage` to this reactor's hp; returns the post-damage view.
    /// Damage is clamped at zero; `destroyed` flips true when hp hits zero.
    pub fn apply_damage(&mut self, damage: f32) {
        if self.is_destroyed() {
            return;
        }
        self.hp = (self.hp - damage.max(0.0)).max(0.0);
        if self.hp <= 0.0 {
            self.destroyed = true;
        }
    }

    pub fn reset(&mut self) {
        self.hp = self.max_hp;
        self.destroyed = false;
    }

    /// Layout-stable bytes for the determinism checksum.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(48 + self.id.len());
        v.extend_from_slice(&(self.id.len() as u32).to_le_bytes());
        v.extend_from_slice(self.id.as_bytes());
        v.extend_from_slice(&quantize(self.position[0]).to_le_bytes());
        v.extend_from_slice(&quantize(self.position[1]).to_le_bytes());
        v.extend_from_slice(&quantize(self.half_extents[0]).to_le_bytes());
        v.extend_from_slice(&quantize(self.half_extents[1]).to_le_bytes());
        v.extend_from_slice(&quantize(self.hp).to_le_bytes());
        v.extend_from_slice(&quantize(self.max_hp).to_le_bytes());
        v.push(u8::from(self.destroyed));
        v
    }
}

fn quantize(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1024.0).round() as i32
}

/// World container of every reactor the engine knows about.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ReactorWorld {
    pub reactors: BTreeMap<String, Reactor>,
}

impl ReactorWorld {
    pub fn new(reactors: Vec<Reactor>) -> Self {
        let mut map = BTreeMap::new();
        for r in reactors {
            map.insert(r.id.clone(), r);
        }
        Self { reactors: map }
    }

    pub fn get(&self, id: &str) -> Option<&Reactor> {
        self.reactors.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Reactor> {
        self.reactors.get_mut(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Reactor> {
        self.reactors.values()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Reactor> {
        self.reactors.values_mut()
    }

    pub fn is_destroyed(&self, id: &str) -> bool {
        self.get(id).is_some_and(Reactor::is_destroyed)
    }

    pub fn destroyed_map(&self) -> BTreeMap<String, bool> {
        self.reactors
            .iter()
            .map(|(k, v)| (k.clone(), v.is_destroyed()))
            .collect()
    }

    pub fn reset(&mut self) {
        for r in self.reactors.values_mut() {
            r.reset();
        }
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.reactors.len() * 32 + 8);
        out.extend_from_slice(&(self.reactors.len() as u32).to_le_bytes());
        for r in self.reactors.values() {
            out.extend_from_slice(&r.checksum_bytes());
        }
        out
    }
}

/// Per-tick mission inputs. The engine assembles this from its actor world plus the
/// breach state (broken? in-range? hp_remaining?) and reactor state before calling
/// [`step`].
#[derive(Debug, Clone, Copy)]
pub struct MissionTickInputs<'a> {
    pub tick: u64,
    pub player: Option<&'a ActorState>,
    pub actors: &'a BTreeMap<ActorId, ActorState>,
    /// Map of `breach_id -> broken?`. `true` once the breach is fully carved.
    pub breaches_broken: &'a BTreeMap<String, bool>,
    /// Map of `reactor_id -> destroyed?`. `true` once hp <= 0. Defaults empty.
    pub reactors_destroyed: &'a BTreeMap<String, bool>,
    /// **M1.5**: map of `breach_id -> carve_progress` in `[0.0, 1.0]`. Drives
    /// `mission.objective_updated` events at 25/50/75/100% milestones for
    /// `BreachBarrier` objectives. May be left empty; missing ids default to
    /// `0.0` (no progress yet).
    pub breaches_progress: &'a BTreeMap<String, f32>,
}

/// Per-tick report. Every `Vec` carries objective ids; the engine turns each into a
/// `mission.objective_*` event in tick order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MissionTickReport {
    pub objective_started: Vec<String>,
    pub objective_completed: Vec<String>,
    pub objective_failed: Vec<String>,
    /// **M1.5**: progress milestone crossings. One entry per (objective_id,
    /// quartile) crossed on this tick. `progress` is the milestone value
    /// (0.25, 0.5, 0.75, or 1.0). The engine emits one `mission.objective_updated`
    /// event per entry.
    pub objective_updated: Vec<ObjectiveProgressUpdate>,
    /// Set on the tick the mission resolves (`Won` or `Lost`).
    pub final_result: Option<MissionResult>,
}

/// **M1.5**: one milestone-crossing entry surfaced on `MissionTickReport`.
/// The engine turns each into a `mission.objective_updated` event with a
/// payload of `{ objective_id, progress }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectiveProgressUpdate {
    pub objective_id: String,
    pub progress: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionState {
    /// **M2 re-audit (2026-05-13)**: scenario id (mirrors `scenario.id`).
    /// Empty string for legacy callers that didn't populate it; the engine
    /// sets this from the loaded scenario.
    #[serde(default)]
    pub id: String,
    /// **M2 re-audit (2026-05-13)**: lifecycle state machine per spec
    /// (`Init → Loaded → InProgress → Resolved`). Distinct from `result`
    /// (which is the outcome SHAPE when `lifecycle == Resolved`).
    #[serde(default)]
    pub lifecycle: MissionLifecycle,
    pub objectives: Vec<Objective>,
    pub started_at_tick: u64,
    pub time_limit_ticks: u64,
    pub loss: LossConditions,
    pub result: MissionResult,
    pub last_event_tick: u64,
    pub last_event_label: String,
    /// Tick of the most recent objective or result state transition.
    #[serde(default)]
    pub last_transition_tick: u64,
    /// Typed loss reason vocabulary for stable replay/analytics. Populated from
    /// `LossReason::as_str()` when the mission resolves as Lost.
    #[serde(default)]
    pub loss_reason_label: Option<String>,
    /// **M2 re-audit (2026-05-13)**: explicit typed loss reason (not just
    /// the as_str() label). Populated alongside `loss_reason_label` so
    /// consumers can access the structured payload (e.g.
    /// `LossReason::ObjectiveFailed { id, reason }`).
    #[serde(default)]
    pub loss_reason: Option<LossReason>,
    /// **M1.5**: tutorial-modal pause flag. While `true`, `step()` is a
    /// no-op AND elapsed-tick accounting skips the paused duration so the
    /// mission timer does NOT advance. Toggled via `MissionState::pause()`
    /// / `resume()` so the engine can wire `act.mission.{pause,resume}`
    /// cfctl methods + emit `mission.objective_paused` / `objective_resumed`
    /// events.
    #[serde(default)]
    pub paused: bool,
    /// **M1.5**: tick on which the most recent pause began. `None` when
    /// not paused; populated by `pause()`. `resume()` uses this to
    /// accumulate `total_paused_ticks` and then clears the field.
    #[serde(default)]
    pub pause_started_at_tick: Option<u64>,
    /// **M1.5**: cumulative paused duration in ticks. `elapsed_ticks` and
    /// `ticks_remaining` subtract this so the timer truly freezes while
    /// the modal is up.
    #[serde(default)]
    pub total_paused_ticks: u64,
    /// **M1.5**: DR-023 "Show me why" replay-handoff anchor. Populated by
    /// the engine when the mission resolves as Lost; points at the player's
    /// last `input.intent_received` event so M3B's replay viewer can rewind
    /// to the divergence tick. Stays `None` for Won / Aborted / Active.
    #[serde(default)]
    pub show_me_why_event_id: Option<String>,
    /// **M1.5**: cf-ui renders the "Show me why" CTA button on the
    /// mission-resolved modal when `true`. Latched from the
    /// mission_resolved event payload's `show_replay_cta` flag.
    #[serde(default)]
    pub show_replay_cta: bool,
}

impl MissionState {
    pub fn new(objectives: Vec<Objective>, started_at_tick: u64, loss: LossConditions) -> Self {
        // BP2 fix: leave all objectives Pending. The first call to `step()`
        // activates the first pending objective AND emits `mission.objective_started`
        // through the same code path that activates subsequent objectives. Without
        // this, the FIRST objective transitioned Pending → Active inside `new()`
        // (with no MissionTickReport in scope), so `mission.objective_started`
        // never fired for it — the engine only saw the second + later objectives'
        // started events. The bp2 test-coverage analyzer caught this gap by
        // cross-referencing the manifest's `required_events_emitted` list against
        // the M2.5 win bundle's events.jsonl.
        Self {
            id: String::new(),
            lifecycle: MissionLifecycle::Loaded,
            objectives,
            started_at_tick,
            time_limit_ticks: loss.time_limit_ticks,
            loss,
            result: MissionResult::InProgress,
            last_event_tick: started_at_tick,
            last_event_label: "mission_started".to_string(),
            last_transition_tick: started_at_tick,
            loss_reason_label: None,
            loss_reason: None,
            paused: false,
            pause_started_at_tick: None,
            total_paused_ticks: 0,
            show_me_why_event_id: None,
            show_replay_cta: false,
        }
    }

    /// **M2 re-audit (2026-05-13)**: returns the id of the currently-active
    /// objective (if any), per the spec's `current_objective_id` field. Walks
    /// objectives in order, returning the first `Active`.
    pub fn current_objective_id(&self) -> Option<&str> {
        self.objectives
            .iter()
            .find(|o| o.status == ObjectiveStatus::Active)
            .map(|o| o.id.as_str())
    }

    /// **M2 re-audit (2026-05-13)**: list of completed objective ids in
    /// declaration order, per the spec's `completed_objectives[]` field.
    pub fn completed_objective_ids(&self) -> Vec<String> {
        self.objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Completed)
            .map(|o| o.id.clone())
            .collect()
    }

    /// **M2 re-audit (2026-05-13)**: list of failed objective ids in
    /// declaration order, per the spec's `failed_objectives[]` field.
    pub fn failed_objective_ids(&self) -> Vec<String> {
        self.objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Failed)
            .map(|o| o.id.clone())
            .collect()
    }

    /// **M1.5**: pause the mission's tick-driven progress + timer.
    /// Returns the id of the currently active objective (if any) so the
    /// caller can emit `mission.objective_paused { objective: <id> }`.
    /// No-op (returns None) if the mission is terminal or already paused.
    pub fn pause(&mut self, current_tick: u64) -> Option<String> {
        if self.result.is_terminal() || self.paused {
            return None;
        }
        self.paused = true;
        self.pause_started_at_tick = Some(current_tick);
        self.last_event_tick = current_tick;
        self.last_event_label = "objective_paused".to_string();
        self.last_transition_tick = current_tick;
        self.active_objective_id()
    }

    /// **M1.5**: resume after pause. Adds the paused duration to
    /// `total_paused_ticks` so timer reads correctly. Returns the id of
    /// the active objective so the engine can emit
    /// `mission.objective_resumed { objective: <id> }`. No-op (returns
    /// None) if the mission is not paused.
    pub fn resume(&mut self, current_tick: u64) -> Option<String> {
        if !self.paused {
            return None;
        }
        if let Some(started) = self.pause_started_at_tick.take() {
            self.total_paused_ticks = self
                .total_paused_ticks
                .saturating_add(current_tick.saturating_sub(started));
        }
        self.paused = false;
        self.last_event_tick = current_tick;
        self.last_event_label = "objective_resumed".to_string();
        self.last_transition_tick = current_tick;
        self.active_objective_id()
    }

    fn active_objective_id(&self) -> Option<String> {
        self.active_objective_index().map(|i| self.objectives[i].id.clone())
    }

    /// Reset the mission to its starting state. Used by `scenario.reset` so the
    /// engine can rewind objectives + result + timer without rebuilding from the
    /// scenario manifest.
    pub fn reset(&mut self, started_at_tick: u64) {
        for o in &mut self.objectives {
            o.status = ObjectiveStatus::Pending;
            o.progress_milestone_index = 0;
        }
        // Same BP2 fix as `new()`: do NOT activate the first objective here.
        // step() handles the activation on its next call so the `objective_started`
        // event for the first objective fires through the same path as later ones.
        self.started_at_tick = started_at_tick;
        self.result = MissionResult::InProgress;
        self.last_event_tick = started_at_tick;
        self.last_event_label = "mission_started".to_string();
        self.last_transition_tick = started_at_tick;
        self.loss_reason_label = None;
        self.paused = false;
        self.pause_started_at_tick = None;
        self.total_paused_ticks = 0;
        self.show_me_why_event_id = None;
        self.show_replay_cta = false;
    }

    /// Number of required objectives still in `Pending` or `Active` status.
    pub fn outstanding_required(&self) -> usize {
        self.objectives
            .iter()
            .filter(|o| !o.optional && !o.status.is_terminal())
            .count()
    }

    /// Number of required objectives in `Completed` status.
    pub fn completed_required(&self) -> usize {
        self.objectives
            .iter()
            .filter(|o| !o.optional && o.status == ObjectiveStatus::Completed)
            .count()
    }

    /// Number of required objectives in `Failed` status.
    pub fn failed_required(&self) -> usize {
        self.objectives
            .iter()
            .filter(|o| !o.optional && o.status == ObjectiveStatus::Failed)
            .count()
    }

    /// Index of the currently-active required objective (i.e. the next `Active`
    /// row), if any.
    pub fn active_objective_index(&self) -> Option<usize> {
        self.objectives.iter().position(|o| o.status == ObjectiveStatus::Active)
    }

    /// Ticks elapsed since `started_at_tick`. Saturates at 0.
    /// **M1.5**: subtracts `total_paused_ticks` AND the current pause
    /// in-flight (if `paused`) so the timer freezes while a tutorial
    /// modal is up.
    pub fn elapsed_ticks(&self, current_tick: u64) -> u64 {
        let raw = current_tick.saturating_sub(self.started_at_tick);
        let mut pause_credit = self.total_paused_ticks;
        if self.paused {
            if let Some(started) = self.pause_started_at_tick {
                pause_credit = pause_credit.saturating_add(current_tick.saturating_sub(started));
            }
        }
        raw.saturating_sub(pause_credit)
    }

    /// Ticks remaining before the timer expires. `None` when no timer is set.
    pub fn ticks_remaining(&self, current_tick: u64) -> Option<u64> {
        if self.time_limit_ticks == 0 {
            None
        } else {
            Some(self.time_limit_ticks.saturating_sub(self.elapsed_ticks(current_tick)))
        }
    }
}

/// **M1.5**: progress quartiles for `mission.objective_updated`. Stable
/// vocabulary so the run-bundle viewer can render a progress bar.
const PROGRESS_QUARTILES: [f32; 4] = [0.25, 0.5, 0.75, 1.0];

/// Drive the mission state machine for one tick. Idempotent once the result is
/// terminal (returns an empty report after Won/Lost).
#[must_use]
pub fn step(state: &mut MissionState, inputs: MissionTickInputs<'_>) -> MissionTickReport {
    let mut report = MissionTickReport::default();
    if state.result.is_terminal() {
        return report;
    }
    // **M1.5**: while paused (tutorial modal), suspend objective progress
    // AND timer accounting. The caller is responsible for calling
    // `MissionState::resume` to lift the gate.
    if state.paused {
        return report;
    }

    // 0) BP2 fix: if no objective is currently Active (e.g. on the FIRST tick
    //    after `MissionState::new()` or `reset()`), activate the first pending
    //    objective AND push it to `report.objective_started` so the engine
    //    emits a `mission.objective_started` event. Without this guard the
    //    first objective transitioned Pending → Active silently inside new()
    //    and the started event was lost.
    if !state.objectives.iter().any(|o| o.status == ObjectiveStatus::Active) {
        if let Some(first) = state
            .objectives
            .iter_mut()
            .find(|o| o.status == ObjectiveStatus::Pending)
        {
            first.status = ObjectiveStatus::Active;
            report.objective_started.push(first.id.clone());
            state.last_event_tick = inputs.tick;
            state.last_event_label = format!("objective_started:{}", first.id);
        }
    }

    // 1) Loss conditions take precedence over objective progress so a fail-state
    //    that lands on the same tick as an objective completion is still recorded
    //    as a loss. This matches the M7 mission-director failure ordering.
    if state.loss.player_dead {
        let player_dead = match inputs.player {
            Some(p) => p.status.is_dead() || p.hp <= 0.0,
            None => false,
        };
        if player_dead {
            state.result = MissionResult::Lost {
                reason: LossReason::PlayerDead,
            };
            state.last_event_tick = inputs.tick;
            state.last_event_label = "mission_lost_player_dead".to_string();
            report.final_result = Some(state.result.clone());
            return report;
        }
    }
    // Reactor destruction loses immediately for any active `defend_reactor`
    // objective. M2.5's micro reactor defense needs `mission.loss_reason =
    // reactor_destroyed` to be the visible failure label. The check runs BEFORE
    // the timer expiry check so a reactor destroyed exactly at the timer
    // boundary still records as `reactor_destroyed`.
    //
    // The first pass scans for the failing index using an immutable borrow so
    // we can write `state.last_event_label = format!(...)` afterwards without
    // aliasing `state.objectives`. The second pass mutates the matched
    // objective's `status` to `Failed` so `MissionView::from_state` reports
    // `failed` in the observe envelope (Devin review BUG_pr-review-job
    // -8dddb0ae78c7456997c4d2dc7aade217_0001).
    // Bugbot 3212230553 (Low): scan Active OR Pending defend_reactor
    // objectives. If a defend_reactor is queued behind another objective
    // (Pending status) and its reactor is destroyed in the meantime, the
    // mission MUST resolve as Lost { ReactorDestroyed } immediately rather
    // than wait for the objective to become Active. Completed/Failed rows
    // are skipped because they're terminal states.
    let reactor_destroyed_match: Option<(usize, String, String)> =
        state.objectives.iter().enumerate().find_map(|(idx, obj)| {
            if matches!(obj.status, ObjectiveStatus::Completed | ObjectiveStatus::Failed) {
                return None;
            }
            if let ObjectiveKind::DefendReactor { target } = &obj.kind {
                if inputs.reactors_destroyed.get(target).copied().unwrap_or(false) {
                    return Some((idx, obj.id.clone(), target.clone()));
                }
            }
            None
        });
    if let Some((idx, obj_id, target)) = reactor_destroyed_match {
        state.objectives[idx].status = ObjectiveStatus::Failed;
        state.result = MissionResult::Lost {
            reason: LossReason::ReactorDestroyed,
        };
        state.last_event_tick = inputs.tick;
        state.last_event_label = format!("mission_lost_reactor_destroyed:{target}");
        report.objective_failed.push(obj_id);
        report.final_result = Some(state.result.clone());
        return report;
    }

    let timer_expired = state.time_limit_ticks > 0 && state.elapsed_ticks(inputs.tick) >= state.time_limit_ticks;
    if timer_expired {
        // Special case: an active `defend_reactor` objective WINS when the
        // timer expires (the player held the reactor through the wave). We
        // detect this by looking for an active defend_reactor objective whose
        // reactor is still alive.
        let defend_active_alive = state.objectives.iter().any(|obj| {
            matches!(obj.status, ObjectiveStatus::Active)
                && match &obj.kind {
                    ObjectiveKind::DefendReactor { target } => {
                        !inputs.reactors_destroyed.get(target).copied().unwrap_or(false)
                    }
                    _ => false,
                }
        });
        if defend_active_alive {
            // Mark every active defend_reactor objective complete and check win
            // condition below.
            for obj in &mut state.objectives {
                if obj.status != ObjectiveStatus::Active {
                    continue;
                }
                if let ObjectiveKind::DefendReactor { target } = &obj.kind {
                    if !inputs.reactors_destroyed.get(target).copied().unwrap_or(false) {
                        obj.status = ObjectiveStatus::Completed;
                        report.objective_completed.push(obj.id.clone());
                        state.last_event_tick = inputs.tick;
                        state.last_event_label = format!("objective_completed:{}", obj.id);
                    }
                }
            }
            // Devin BUG_pr-review-job 0001 (flag): if the timer is expired AND
            // a defend_reactor was just completed by surviving the timer, the
            // mission MUST resolve on this tick rather than fall through. On
            // the NEXT tick the timer would still be expired, the
            // defend_reactor would no longer be active (we just completed
            // it), and `defend_active_alive` would be false, sending the
            // mission to TimerExpired loss instead of Won. The latent bug
            // only matters for hypothetical mixed-objective scenarios
            // (DefendReactor + ReachZone, etc.), but resolving here is the
            // robust fix.
            //
            // If every required objective is now complete, win immediately.
            // Otherwise, the player completed defend_reactor at the timer
            // boundary but still owes other required objectives — that's a
            // TimerExpired loss because the rest of the mission is not done.
            if state.outstanding_required() == 0 && state.failed_required() == 0 {
                state.result = MissionResult::Won;
                state.last_event_tick = inputs.tick;
                state.last_event_label = "mission_won".to_string();
                report.final_result = Some(state.result.clone());
            } else {
                state.result = MissionResult::Lost {
                    reason: LossReason::TimerExpired,
                };
                state.last_event_tick = inputs.tick;
                state.last_event_label = "mission_lost_timer".to_string();
                report.final_result = Some(state.result.clone());
            }
            return report;
        }
        // M2 audit pass 5 (2026-05-13): spec literal — "mission.objective_failed
        // fires with objective_id='reach_extraction', reason='timer_expired'"
        // when the timer expires while any Active objective is incomplete.
        // Iterate every Active objective and flip it to Failed so the engine
        // emits per-objective `mission.objective_failed` before
        // `mission.mission_resolved`.
        for obj in &mut state.objectives {
            if obj.status == ObjectiveStatus::Active {
                obj.status = ObjectiveStatus::Failed;
                report.objective_failed.push(obj.id.clone());
            }
        }
        state.result = MissionResult::Lost {
            reason: LossReason::TimerExpired,
        };
        state.last_event_tick = inputs.tick;
        state.last_event_label = "mission_lost_timer".to_string();
        report.final_result = Some(state.result.clone());
        return report;
    }

    // 1b) **M1.5**: emit `mission.objective_updated` events when the active
    // `BreachBarrier` objective crosses the 25/50/75/100% carve milestones.
    // The 100% milestone fires on the same tick as `objective_completed`
    // so the cause chain shows: dig_request -> objective_updated{progress:1.0}
    // -> objective_completed.
    for obj in &mut state.objectives {
        if obj.status != ObjectiveStatus::Active {
            continue;
        }
        let progress = match &obj.kind {
            ObjectiveKind::BreachBarrier { target } => inputs.breaches_progress.get(target).copied().unwrap_or(0.0),
            _ => continue,
        };
        while (obj.progress_milestone_index as usize) < PROGRESS_QUARTILES.len() {
            let next = PROGRESS_QUARTILES[obj.progress_milestone_index as usize];
            if progress + 1e-6 >= next {
                obj.progress_milestone_index += 1;
                report.objective_updated.push(ObjectiveProgressUpdate {
                    objective_id: obj.id.clone(),
                    progress: next,
                });
                state.last_event_tick = inputs.tick;
                state.last_event_label = format!("objective_updated:{}:{:.2}", obj.id, next);
            } else {
                break;
            }
        }
    }

    // 2) Progress objectives in declaration order. We only advance one row at a
    //    time so the player always has a single Active objective for the HUD.
    let mut started_index: Option<usize> = None;
    for (i, obj) in state.objectives.iter_mut().enumerate() {
        if obj.status != ObjectiveStatus::Active {
            continue;
        }
        let completed = match &obj.kind {
            ObjectiveKind::BreachBarrier { target } => inputs.breaches_broken.get(target).copied().unwrap_or(false),
            ObjectiveKind::NeutralizeActor { target } => inputs
                .actors
                .get(&ActorId(*target))
                .is_some_and(|a| a.status == Status::Dead),
            ObjectiveKind::ReachZone { min, max } => match inputs.player {
                Some(p) => point_in_aabb(p.position.x, p.position.y, *min, *max),
                None => false,
            },
            ObjectiveKind::DefendReactor { .. } => {
                // DefendReactor only completes via the timer-expired branch
                // above; passive ticks never auto-complete it.
                false
            }
            // M2 re-audit (2026-05-13): SurviveTimer completes when the
            // window has elapsed AND the player is still alive. The fail
            // branch (player dies) is handled by the player-dead loss
            // check earlier in step(); a SurviveTimer that hasn't elapsed
            // simply stays Active until then.
            ObjectiveKind::SurviveTimer { survive_ticks } => {
                let elapsed = inputs.tick.saturating_sub(state.started_at_tick);
                elapsed >= *survive_ticks
                    && inputs
                        .player
                        .is_some_and(|p| p.status != Status::Dead && p.status != Status::Dying)
            }
            // M2 re-audit (2026-05-13): EscortActor completes when the
            // escortee enters the destination AABB AND is still alive.
            ObjectiveKind::EscortActor {
                target,
                destination_min,
                destination_max,
            } => inputs.actors.get(&ActorId(*target)).is_some_and(|a| {
                a.status != Status::Dead
                    && point_in_aabb(a.position.x, a.position.y, *destination_min, *destination_max)
            }),
        };
        if completed {
            obj.status = ObjectiveStatus::Completed;
            report.objective_completed.push(obj.id.clone());
            state.last_event_tick = inputs.tick;
            state.last_event_label = format!("objective_completed:{}", obj.id);
            started_index = Some(i + 1);
            break;
        }
    }
    // Activate the next pending required objective, if any.
    if let Some(start_from) = started_index {
        for (j, obj) in state.objectives.iter_mut().enumerate().skip(start_from) {
            if obj.status == ObjectiveStatus::Pending {
                obj.status = ObjectiveStatus::Active;
                report.objective_started.push(obj.id.clone());
                state.last_event_tick = inputs.tick;
                state.last_event_label = format!("objective_started:{}", obj.id);
                break;
            }
            // Skip any already-completed/failed rows (e.g. optional rows resolved earlier).
            if !obj.status.is_terminal() {
                break;
            }
            let _ = j;
        }
    }

    // 3) Win condition: every required objective reached `Completed` and zero
    //    required failures.
    if state.outstanding_required() == 0 && state.failed_required() == 0 {
        state.result = MissionResult::Won;
        state.last_event_tick = inputs.tick;
        state.last_event_label = "mission_won".to_string();
        report.final_result = Some(state.result.clone());
    }

    // Track transition timing for analytics (W1 item 866).
    if report.final_result.is_some()
        || !report.objective_started.is_empty()
        || !report.objective_completed.is_empty()
        || !report.objective_failed.is_empty()
        || !report.objective_updated.is_empty()
    {
        state.last_transition_tick = inputs.tick;
    }
    if let Some(MissionResult::Lost { ref reason }) = report.final_result {
        state.loss_reason_label = Some(reason.as_str().to_string());
    }

    report
}

fn point_in_aabb(x: f32, y: f32, min: [f32; 2], max: [f32; 2]) -> bool {
    x >= min[0] && x <= max[0] && y >= min[1] && y <= max[1]
}

/// Convenience used by the engine to build a per-tick view for the observe
/// envelope. M1.5 keeps it tiny; M4 will wire the comic-noir HUD on top.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionView {
    /// **M2 audit pass 7 (2026-05-13)**: spec literal field name is
    /// `status`. `result` retained as alias because the wire was stable
    /// across the M1-M3 era.
    #[serde(rename = "status")]
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    /// **M2 audit pass 7 (2026-05-13)**: spec literal field name is
    /// `timer_total_ticks`. `time_limit_ticks` retained as alias.
    #[serde(rename = "timer_total_ticks")]
    pub time_limit_ticks: u64,
    /// **M2 audit pass 7 (2026-05-13)**: spec literal field name is
    /// `timer_ticks_remaining`.
    #[serde(rename = "timer_ticks_remaining")]
    pub ticks_remaining: Option<u64>,
    /// **M2 audit pass 7 (2026-05-13)**: spec literal field name is
    /// `current_objective_id`.
    #[serde(rename = "current_objective_id")]
    pub active_objective: Option<String>,
    /// **M2 audit pass 7 (2026-05-13)**: spec-literal `completed_objectives[]`
    /// — list of objective ids in completion order.
    #[serde(default)]
    pub completed_objectives: Vec<String>,
    /// **M2 audit pass 7 (2026-05-13)**: spec-literal `failed_objectives[]`.
    #[serde(default)]
    pub failed_objectives: Vec<String>,
    pub objectives: Vec<ObjectiveView>,
    pub last_event_tick: u64,
    pub last_event_label: String,
    /// **M1.5**: DR-023 "Show me why" replay-handoff anchor. Populated
    /// when the mission resolves as Lost; cf-ui renders the CTA button
    /// when this field is `Some`.
    #[serde(default)]
    pub show_me_why_event_id: Option<String>,
    /// **M1.5**: cf-ui modal flag — render the "Show me why" CTA when
    /// `true`.
    #[serde(default)]
    pub show_replay_cta: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveView {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub optional: bool,
    pub target_actor: Option<u64>,
    pub target_breach: Option<String>,
    pub target_reactor: Option<String>,
    pub zone_min: Option<[f32; 2]>,
    pub zone_max: Option<[f32; 2]>,
}

impl MissionView {
    pub fn from_state(state: &MissionState, current_tick: u64) -> Self {
        let active_objective = state.active_objective_index().map(|i| state.objectives[i].id.clone());
        let loss_reason = match &state.result {
            MissionResult::Lost { reason } => Some(reason.as_str().to_string()),
            _ => None,
        };
        let objectives = state
            .objectives
            .iter()
            .map(|o| ObjectiveView {
                id: o.id.clone(),
                kind: o.kind.category().to_string(),
                status: o.status.as_str().to_string(),
                optional: o.optional,
                target_actor: match &o.kind {
                    ObjectiveKind::NeutralizeActor { target } => Some(*target),
                    _ => None,
                },
                target_breach: match &o.kind {
                    ObjectiveKind::BreachBarrier { target } => Some(target.clone()),
                    _ => None,
                },
                target_reactor: match &o.kind {
                    ObjectiveKind::DefendReactor { target } => Some(target.clone()),
                    _ => None,
                },
                zone_min: match &o.kind {
                    ObjectiveKind::ReachZone { min, .. } => Some(*min),
                    _ => None,
                },
                zone_max: match &o.kind {
                    ObjectiveKind::ReachZone { max, .. } => Some(*max),
                    _ => None,
                },
            })
            .collect();
        // M2 audit pass 7 (2026-05-13): populate completed_objectives[] +
        // failed_objectives[] arrays so observe.mission carries them in
        // the JSON, per spec MissionState surface.
        let completed_objectives = state
            .objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Completed)
            .map(|o| o.id.clone())
            .collect();
        let failed_objectives = state
            .objectives
            .iter()
            .filter(|o| o.status == ObjectiveStatus::Failed)
            .map(|o| o.id.clone())
            .collect();
        Self {
            result: state.result.as_str().to_string(),
            loss_reason,
            elapsed_ticks: state.elapsed_ticks(current_tick),
            time_limit_ticks: state.time_limit_ticks,
            ticks_remaining: state.ticks_remaining(current_tick),
            active_objective,
            completed_objectives,
            failed_objectives,
            objectives,
            last_event_tick: state.last_event_tick,
            last_event_label: state.last_event_label.clone(),
            show_me_why_event_id: state.show_me_why_event_id.clone(),
            show_replay_cta: state.show_replay_cta,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_actor::{Inventory, Vec2};

    fn build_state() -> MissionState {
        let objectives = vec![
            Objective {
                id: "breach".to_string(),
                kind: ObjectiveKind::BreachBarrier {
                    target: "outer_wall".to_string(),
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
            Objective {
                id: "neutralize".to_string(),
                kind: ObjectiveKind::NeutralizeActor { target: 2 },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
            Objective {
                id: "extract".to_string(),
                kind: ObjectiveKind::ReachZone {
                    min: [1180.0, 16.0],
                    max: [1280.0, 64.0],
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
        ];
        MissionState::new(
            objectives,
            0,
            LossConditions {
                player_dead: true,
                time_limit_ticks: 60 * 90,
            },
        )
    }

    fn player_at(x: f32, y: f32) -> ActorState {
        ActorState::player(ActorId(1), "blue", Vec2::new(x, y), 100.0, Inventory::default())
    }

    fn mk_actors(player: ActorState, enemy_dead: bool) -> BTreeMap<ActorId, ActorState> {
        let mut m = BTreeMap::new();
        m.insert(player.id, player);
        let mut enemy = ActorState::player(ActorId(2), "red", Vec2::new(900.0, 32.0), 80.0, Inventory::default());
        if enemy_dead {
            enemy.hp = 0.0;
            enemy.status = Status::Dead;
        }
        m.insert(enemy.id, enemy);
        m
    }

    #[test]
    fn first_objective_starts_pending_then_activates_on_first_step() {
        // BP2 fix: first objective is Pending after construction; the first
        // step() activates it AND emits objective_started so the engine emits
        // a `mission.objective_started` event for objective 0.
        let mut state = build_state();
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Pending);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert_eq!(report.objective_started, vec!["breach".to_string()]);
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
    }

    #[test]
    fn pause_suspends_step_and_timer_resume_restores() {
        // **M1.5**: while paused, step() is a no-op AND the timer freezes.
        let mut state = build_state();
        let actors = mk_actors(player_at(120.0, 32.0), false);
        // Tick 1: activate breach.
        let _ = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        // Tick 5: pause. elapsed_ticks at tick 5 = 5.
        let active = state.pause(5).expect("pause returns active id");
        assert_eq!(active, "breach");
        assert!(state.paused);
        assert_eq!(state.elapsed_ticks(5), 5);
        // Tick 50: still paused; elapsed should not advance past tick 5's value.
        let mut breaches = BTreeMap::new();
        breaches.insert("outer_wall".to_string(), true);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 50,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &breaches,
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(report.objective_completed.is_empty(), "paused step is a no-op");
        assert!(state.objectives[0].status == ObjectiveStatus::Active);
        // ticks_remaining at tick 50 while paused = original time_limit - 5.
        let remaining_paused = state.ticks_remaining(50).unwrap();
        assert_eq!(remaining_paused, state.time_limit_ticks - 5);
        // Resume at tick 50: paused for 45 ticks; subsequent timer reads reflect that credit.
        let resumed = state.resume(50).expect("resume returns active id");
        assert_eq!(resumed, "breach");
        assert!(!state.paused);
        assert_eq!(state.total_paused_ticks, 45);
        // Tick 100: elapsed = 100 - 0 - 45 = 55.
        assert_eq!(state.elapsed_ticks(100), 55);
    }

    #[test]
    fn pause_resume_skip_when_terminal_or_double_called() {
        let mut state = build_state();
        let actors = mk_actors(player_at(120.0, 32.0), false);
        // Pause once.
        let _ = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(state.pause(2).is_some());
        // Second pause: None (already paused).
        assert!(state.pause(3).is_none());
        // Resume.
        assert!(state.resume(4).is_some());
        // Second resume: None (not paused).
        assert!(state.resume(5).is_none());
        // Terminal: pause refuses.
        state.result = MissionResult::Won;
        assert!(state.pause(6).is_none());
    }

    #[test]
    fn breach_progress_milestones_emit_objective_updated() {
        // **M1.5**: `mission.objective_updated` fires at 25/50/75/100%
        // carve milestones for the active `BreachBarrier` objective.
        let mut state = build_state();
        let actors = mk_actors(player_at(120.0, 32.0), false);
        // Tick 1: activate the breach objective.
        let _ = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        // Tick 2: 30% progress -> 25% milestone fires once.
        let mut progress = BTreeMap::new();
        progress.insert("outer_wall".to_string(), 0.30_f32);
        let r2 = step(
            &mut state,
            MissionTickInputs {
                tick: 2,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &progress,
            },
        );
        assert_eq!(r2.objective_updated.len(), 1);
        assert_eq!(r2.objective_updated[0].objective_id, "breach");
        assert!((r2.objective_updated[0].progress - 0.25).abs() < 1e-3);
        // Tick 3: 60% progress -> 50% milestone fires (75% not yet).
        progress.insert("outer_wall".to_string(), 0.60_f32);
        let r3 = step(
            &mut state,
            MissionTickInputs {
                tick: 3,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &progress,
            },
        );
        assert_eq!(r3.objective_updated.len(), 1);
        assert!((r3.objective_updated[0].progress - 0.5).abs() < 1e-3);
        // Tick 4: 99% progress crosses 75% only.
        progress.insert("outer_wall".to_string(), 0.99_f32);
        let r4 = step(
            &mut state,
            MissionTickInputs {
                tick: 4,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &progress,
            },
        );
        assert_eq!(r4.objective_updated.len(), 1);
        assert!((r4.objective_updated[0].progress - 0.75).abs() < 1e-3);
        // Tick 5: 100% progress + broken=true -> 100% milestone fires AND
        // the objective completes on the same tick. The objective_updated
        // entry precedes the objective_completed one so the cause chain
        // reads dig -> objective_updated{1.0} -> objective_completed.
        let mut broken = BTreeMap::new();
        broken.insert("outer_wall".to_string(), true);
        progress.insert("outer_wall".to_string(), 1.0_f32);
        let r5 = step(
            &mut state,
            MissionTickInputs {
                tick: 5,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &broken,
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &progress,
            },
        );
        assert_eq!(r5.objective_updated.len(), 1);
        assert!((r5.objective_updated[0].progress - 1.0).abs() < 1e-3);
        assert_eq!(r5.objective_completed, vec!["breach".to_string()]);
    }

    #[test]
    fn breach_completion_advances_to_neutralize() {
        let mut state = build_state();
        let actors = mk_actors(player_at(120.0, 32.0), false);
        // BP2 fix: now that the first objective starts Pending, drive one
        // empty step() first so it activates + emits objective_started.
        // Then the breach-broken tick completes it + activates "neutralize".
        let _activation = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
        let mut breaches = BTreeMap::new();
        breaches.insert("outer_wall".to_string(), true);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 60,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &breaches,
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert_eq!(report.objective_completed, vec!["breach".to_string()]);
        assert_eq!(report.objective_started, vec!["neutralize".to_string()]);
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Completed);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Active);
    }

    #[test]
    fn full_clear_wins_mission() {
        let mut state = build_state();
        let mut breaches = BTreeMap::new();
        breaches.insert("outer_wall".to_string(), true);
        let player = player_at(1200.0, 32.0);
        let actors = mk_actors(player.clone(), true);
        // Tick 1: breach completes.
        let _ = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &breaches,
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        // Tick 2: neutralize completes.
        let _ = step(
            &mut state,
            MissionTickInputs {
                tick: 2,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &breaches,
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        // Tick 3: extract completes.
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 3,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &breaches,
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert_eq!(report.objective_completed, vec!["extract".to_string()]);
        assert_eq!(report.final_result, Some(MissionResult::Won));
        assert!(matches!(state.result, MissionResult::Won));
    }

    #[test]
    fn player_dead_loses_immediately() {
        let mut state = build_state();
        let mut player = player_at(120.0, 32.0);
        player.hp = 0.0;
        player.status = Status::Dead;
        let actors = mk_actors(player, false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 30,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(matches!(
            state.result,
            MissionResult::Lost {
                reason: LossReason::PlayerDead
            }
        ));
        assert!(report.final_result.is_some());
    }

    #[test]
    fn timer_expiry_loses_mission() {
        let mut state = build_state();
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 60 * 90,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(matches!(
            state.result,
            MissionResult::Lost {
                reason: LossReason::TimerExpired
            }
        ));
        assert!(report.final_result.is_some());
    }

    #[test]
    fn terminal_state_is_idempotent() {
        let mut state = build_state();
        state.result = MissionResult::Won;
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 30,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(report.objective_completed.is_empty());
        assert!(report.final_result.is_none());
    }

    #[test]
    fn reset_returns_to_pending_then_activates_on_first_step() {
        // BP2 fix: reset() leaves all objectives Pending; the next step()
        // activates the first one + emits objective_started.
        let mut state = build_state();
        state.objectives[0].status = ObjectiveStatus::Completed;
        state.objectives[1].status = ObjectiveStatus::Active;
        state.result = MissionResult::Won;
        state.reset(100);
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Pending);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
        assert_eq!(state.started_at_tick, 100);
        assert!(matches!(state.result, MissionResult::InProgress));
        // Drive one step; first objective activates + objective_started fires.
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 101,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert_eq!(report.objective_started, vec!["breach".to_string()]);
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
    }

    fn build_reactor_defense_state(time_limit_ticks: u64) -> MissionState {
        let objectives = vec![Objective {
            id: "defend_reactor".to_string(),
            kind: ObjectiveKind::DefendReactor {
                target: "core_reactor".to_string(),
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        }];
        MissionState::new(
            objectives,
            0,
            LossConditions {
                player_dead: true,
                time_limit_ticks,
            },
        )
    }

    #[test]
    fn defend_reactor_loses_when_reactor_destroyed() {
        let mut state = build_reactor_defense_state(60 * 90);
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let mut reactors = BTreeMap::new();
        reactors.insert("core_reactor".to_string(), true);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 100,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &reactors,
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert_eq!(
            state.result,
            MissionResult::Lost {
                reason: LossReason::ReactorDestroyed
            }
        );
        assert_eq!(report.objective_failed, vec!["defend_reactor".to_string()]);
        assert!(report.final_result.is_some());
        // Regression for Devin BUG_pr-review-job-8dddb0ae78c7456997c4d2dc7aade217_0001:
        // the failing objective's `status` field MUST flip to `Failed` so the
        // observe envelope reports it correctly. Pre-fix, the loop borrowed
        // `&state.objectives` so `obj.status` could not be mutated.
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Failed);
        let view = MissionView::from_state(&state, 100);
        assert_eq!(view.objectives[0].status, "failed");
        assert_eq!(view.loss_reason.as_deref(), Some("reactor_destroyed"));
    }

    #[test]
    fn pending_defend_reactor_loses_when_reactor_destroyed_before_objective_activates() {
        // Bugbot 3212230553 (Low) regression: a `DefendReactor` queued
        // behind an earlier objective (Pending status) MUST detect its
        // reactor being destroyed and resolve the mission as
        // `Lost { ReactorDestroyed }`. Pre-fix the destruction was
        // ignored until the objective became Active.
        let objectives = vec![
            Objective {
                id: "reach".to_string(),
                kind: ObjectiveKind::ReachZone {
                    min: [1180.0, 16.0],
                    max: [1280.0, 64.0],
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
            Objective {
                id: "defend".to_string(),
                kind: ObjectiveKind::DefendReactor {
                    target: "core_reactor".to_string(),
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
        ];
        let mut state = MissionState::new(
            objectives,
            0,
            LossConditions {
                player_dead: true,
                time_limit_ticks: 60 * 60,
            },
        );
        // BP2 fix: after construction NO objective is Active yet — step()
        // activates the first one. `defend` is queued at index 1 = Pending.
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Pending);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let mut reactors = BTreeMap::new();
        reactors.insert("core_reactor".to_string(), true);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 100,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &reactors,
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(matches!(
            state.result,
            MissionResult::Lost {
                reason: LossReason::ReactorDestroyed
            }
        ));
        assert_eq!(report.objective_failed, vec!["defend".to_string()]);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Failed);
    }

    #[test]
    fn defend_reactor_with_outstanding_objective_loses_at_timer_even_with_reactor_alive() {
        // Devin BUG_pr-review-job 0001 (flag) regression: a mixed-objective
        // scenario (DefendReactor + ReachZone where the player is NOT in
        // the zone at timer expiry) must resolve on the timer-expired tick
        // as TimerExpired loss, not silently stay Active.
        let objectives = vec![
            Objective {
                id: "defend".to_string(),
                kind: ObjectiveKind::DefendReactor {
                    target: "core_reactor".to_string(),
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
            Objective {
                id: "reach".to_string(),
                kind: ObjectiveKind::ReachZone {
                    min: [1180.0, 16.0],
                    max: [1280.0, 64.0],
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
        ];
        let mut state = MissionState::new(
            objectives,
            0,
            LossConditions {
                player_dead: true,
                time_limit_ticks: 60 * 60,
            },
        );
        let actors = mk_actors(player_at(120.0, 32.0), false); // not in extract zone
        let mut reactors = BTreeMap::new();
        reactors.insert("core_reactor".to_string(), false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 60 * 60,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &reactors,
                breaches_progress: &BTreeMap::new(),
            },
        );
        // Defend was completed by surviving the timer.
        assert!(report.objective_completed.contains(&"defend".to_string()));
        // But the reach zone is still pending → mission must lose on timer.
        assert!(matches!(
            state.result,
            MissionResult::Lost {
                reason: LossReason::TimerExpired
            }
        ));
        assert!(report.final_result.is_some());
    }

    #[test]
    fn timer_expires_on_first_step_with_reachzone_first_defendreactor_pending_yields_timer_loss() {
        // Bugbot 3212591651 regression: when a multi-objective scenario has
        // ReachZone listed first (so Phase 0 activates ReachZone, not the
        // DefendReactor) and the timer happens to be already expired on the
        // first step() call, the mission must resolve as `Lost { TimerExpired }`
        // — NOT silently win because the Pending DefendReactor's reactor is
        // still alive. The win path requires the DefendReactor to actually be
        // Active, and the reach-zone is still outstanding, so the only correct
        // resolution is TimerExpired loss.
        let objectives = vec![
            Objective {
                id: "reach".to_string(),
                kind: ObjectiveKind::ReachZone {
                    min: [1180.0, 16.0],
                    max: [1280.0, 64.0],
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
            Objective {
                id: "defend".to_string(),
                kind: ObjectiveKind::DefendReactor {
                    target: "core_reactor".to_string(),
                },
                optional: false,
                status: ObjectiveStatus::Pending,
                progress_milestone_index: 0,
                progress: 0.0,
                fail_sensor: None,
            },
        ];
        let mut state = MissionState::new(
            objectives,
            0,
            LossConditions {
                player_dead: true,
                time_limit_ticks: 1, // timer expires on the first step()
            },
        );
        let actors = mk_actors(player_at(120.0, 32.0), false); // not in zone
        let mut reactors = BTreeMap::new();
        reactors.insert("core_reactor".to_string(), false); // reactor still alive
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &reactors,
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(matches!(
            state.result,
            MissionResult::Lost {
                reason: LossReason::TimerExpired
            }
        ));
        // Phase 0 still ran first — ReachZone activated.
        assert_eq!(report.objective_started, vec!["reach".to_string()]);
        // M2 audit pass 5 (2026-05-13): spec literal — active objectives
        // are flipped to Failed on timer-expired loss so the engine emits
        // `mission.objective_failed { reason: "timer_expired" }` before
        // `mission.mission_resolved`. The ReachZone just got Activated this
        // tick, then fails immediately.
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Failed);
        // DefendReactor never got activated — it stays Pending (the player
        // didn't even get a tick to start defending).
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
        assert_eq!(report.objective_failed, vec!["reach".to_string()]);
    }

    #[test]
    fn defend_reactor_wins_when_timer_expires_with_reactor_alive() {
        let mut state = build_reactor_defense_state(60 * 60);
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let mut reactors = BTreeMap::new();
        reactors.insert("core_reactor".to_string(), false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 60 * 60,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &reactors,
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(matches!(state.result, MissionResult::Won));
        assert_eq!(report.objective_completed, vec!["defend_reactor".to_string()]);
        assert!(report.final_result.is_some());
    }

    #[test]
    fn reactor_apply_damage_two_partial_hits_then_kill_in_separate_calls() {
        // Bugbot 2ce56d7e regression cover: simulate two partial hits + one
        // kill hit with the per-hit hp captured at each step. The cf-control
        // engine emits per-hit state captured at apply_damage time, not the
        // post-loop final state, so each event reflects the truthful hp.
        let mut r = Reactor {
            id: "r".to_string(),
            position: [0.0, 0.0],
            half_extents: [16.0, 16.0],
            hp: 100.0,
            max_hp: 100.0,
            destroyed: false,
        };
        let prev_hp_1 = r.hp;
        let prev_destroyed_1 = r.is_destroyed();
        r.apply_damage(30.0);
        assert_eq!(r.hp, 70.0);
        assert!(!r.is_destroyed());
        assert!(
            !prev_destroyed_1 && !r.is_destroyed(),
            "first hit should not have flipped destroyed"
        );
        let _ = prev_hp_1;

        let prev_hp_2 = r.hp;
        let prev_destroyed_2 = r.is_destroyed();
        r.apply_damage(40.0);
        assert_eq!(r.hp, 30.0);
        assert!(!r.is_destroyed());
        assert!(
            !prev_destroyed_2 && !r.is_destroyed(),
            "second hit should not have flipped destroyed"
        );
        let _ = prev_hp_2;

        let prev_destroyed_3 = r.is_destroyed();
        r.apply_damage(40.0);
        assert_eq!(r.hp, 0.0);
        assert!(r.is_destroyed());
        assert!(
            !prev_destroyed_3 && r.is_destroyed(),
            "third hit should have flipped destroyed"
        );

        // Subsequent damage is a no-op (latched destroyed).
        let before = r.hp;
        r.apply_damage(50.0);
        assert_eq!(r.hp, before);
    }

    #[test]
    fn reactor_object_apply_damage_drives_destruction() {
        let mut r = Reactor {
            id: "r".to_string(),
            position: [100.0, 32.0],
            half_extents: [16.0, 16.0],
            hp: 30.0,
            max_hp: 30.0,
            destroyed: false,
        };
        r.apply_damage(10.0);
        assert!(!r.is_destroyed());
        r.apply_damage(20.0);
        assert!(r.is_destroyed());
        let before = r.hp;
        r.apply_damage(50.0);
        assert_eq!(r.hp, before);
    }

    #[test]
    fn reactor_world_destroyed_map_round_trip() {
        let world = ReactorWorld::new(vec![Reactor {
            id: "alpha".to_string(),
            position: [0.0, 0.0],
            half_extents: [8.0, 8.0],
            hp: 50.0,
            max_hp: 50.0,
            destroyed: false,
        }]);
        let map = world.destroyed_map();
        assert_eq!(map.get("alpha"), Some(&false));
    }

    #[test]
    fn reactor_aabb_contains_inside_and_outside() {
        let r = Reactor {
            id: "r".to_string(),
            position: [100.0, 100.0],
            half_extents: [16.0, 16.0],
            hp: 50.0,
            max_hp: 50.0,
            destroyed: false,
        };
        assert!(r.aabb_contains(100.0, 100.0));
        assert!(r.aabb_contains(116.0, 116.0));
        assert!(!r.aabb_contains(200.0, 100.0));
    }

    #[test]
    fn mission_view_round_trip() {
        // BP2 fix: build_state() returns all objectives Pending; drive one
        // step() to activate the first one before asserting active_objective.
        let mut state = build_state();
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let _ = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &BTreeMap::new(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        let view = MissionView::from_state(&state, 30);
        assert_eq!(view.result, "in_progress");
        assert_eq!(view.active_objective.as_deref(), Some("breach"));
        assert_eq!(view.objectives.len(), 3);
        assert_eq!(view.objectives[0].kind, "breach_barrier");
        assert_eq!(view.objectives[1].kind, "neutralize_actor");
        assert_eq!(view.objectives[2].kind, "reach_zone");
    }

    #[test]
    fn objective_failed_emitted_on_reactor_destroyed() {
        // Item 679: test objective_failed event path.
        //
        // **Hardening regression fix (M1 R2)**: the production code at
        // `reactor_destroyed_match` was hardened per Bugbot 3212230553 (Low) to
        // scan Active OR Pending defend_reactor objectives so a destroyed
        // reactor on a queued objective still produces an immediate loss.
        // That means the destruction now resolves on tick 1 (when the first-
        // pending guard activates the objective and the reactor-destroyed
        // check runs in the same tick). The original test discarded tick 1
        // and asserted on tick 2 (which returns an empty report because the
        // mission is already terminal). Updated to assert on tick 1's report
        // — same intent (objective_failed fires when reactor destroyed),
        // correct lifecycle.
        let objectives = vec![Objective {
            id: "defend".to_string(),
            kind: ObjectiveKind::DefendReactor {
                target: "core".to_string(),
            },
            optional: false,
            status: ObjectiveStatus::Pending,
            progress_milestone_index: 0,
            progress: 0.0,
            fail_sensor: None,
        }];
        let loss = LossConditions {
            player_dead: false,
            time_limit_ticks: 3600,
        };
        let mut state = MissionState::new(objectives, 0, loss);
        let reactors = ReactorWorld::new(vec![Reactor {
            id: "core".to_string(),
            position: [50.0, 50.0],
            half_extents: [10.0, 10.0],
            hp: 0.0,
            max_hp: 100.0,
            destroyed: true,
        }]);
        let actors = mk_actors(player_at(100.0, 32.0), false);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 1,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &reactors.destroyed_map(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(
            !report.objective_failed.is_empty(),
            "objective_failed must be emitted when reactor is destroyed (tick 1 report = {report:?})"
        );
        assert_eq!(report.objective_failed[0], "defend");
        assert!(
            matches!(
                state.result,
                MissionResult::Lost {
                    reason: LossReason::ReactorDestroyed
                }
            ),
            "state.result must be Lost(ReactorDestroyed), got {:?}",
            state.result
        );
        // Subsequent ticks are no-ops because the mission is terminal.
        let report_after = step(
            &mut state,
            MissionTickInputs {
                tick: 2,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &BTreeMap::new(),
                reactors_destroyed: &reactors.destroyed_map(),
                breaches_progress: &BTreeMap::new(),
            },
        );
        assert!(report_after.objective_failed.is_empty(), "terminal step must be empty");
    }
}
