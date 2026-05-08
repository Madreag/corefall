//! M1.5: minimal objective state machine.
//!
//! This crate owns the `MissionState` + `Objective` types used by the M1.5 micro
//! breach scenario. The full mission director (typed manifest, command-core, base
//! power, commander AI, comic-noir cards) lands at M7. M1.5 only needs:
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
    clippy::redundant_closure,
    clippy::derivable_impls,
    clippy::wildcard_in_or_patterns,
    clippy::needless_pass_by_value,
    clippy::ref_option,
    clippy::match_wildcard_for_single_variants,
    clippy::trivially_copy_pass_by_ref
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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
}

/// Kind of objective. Discriminator names match the canonical roadmap glossary so
/// M7's typed manifest can read M1.5 scenario files without migrating ids.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ObjectiveKind {
    BreachBarrier { target: String },
    NeutralizeActor { target: u64 },
    ReachZone { min: [f32; 2], max: [f32; 2] },
}

impl ObjectiveKind {
    pub fn category(&self) -> &'static str {
        match self {
            ObjectiveKind::BreachBarrier { .. } => "breach_barrier",
            ObjectiveKind::NeutralizeActor { .. } => "neutralize_actor",
            ObjectiveKind::ReachZone { .. } => "reach_zone",
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LossReason {
    PlayerDead,
    TimerExpired,
}

impl LossReason {
    pub fn as_str(self) -> &'static str {
        match self {
            LossReason::PlayerDead => "player_dead",
            LossReason::TimerExpired => "timer_expired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "result")]
pub enum MissionResult {
    Active,
    Won,
    Lost { reason: LossReason },
}

impl MissionResult {
    pub fn as_str(&self) -> &'static str {
        match self {
            MissionResult::Active => "active",
            MissionResult::Won => "won",
            MissionResult::Lost { .. } => "lost",
        }
    }

    pub fn is_terminal(&self) -> bool {
        !matches!(self, MissionResult::Active)
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

/// Per-tick mission inputs. The engine assembles this from its actor world plus the
/// breach state (broken? in-range? hp_remaining?) before calling [`step`].
#[derive(Debug, Clone, Copy)]
pub struct MissionTickInputs<'a> {
    pub tick: u64,
    pub player: Option<&'a ActorState>,
    pub actors: &'a BTreeMap<ActorId, ActorState>,
    /// Map of `breach_id -> broken?`. `true` once the breach is fully carved.
    pub breaches_broken: &'a BTreeMap<String, bool>,
}

/// Per-tick report. Every `Vec` carries objective ids; the engine turns each into a
/// `mission.objective_*` event in tick order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MissionTickReport {
    pub objective_started: Vec<String>,
    pub objective_completed: Vec<String>,
    pub objective_failed: Vec<String>,
    /// Set on the tick the mission resolves (`Won` or `Lost`).
    pub final_result: Option<MissionResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissionState {
    pub objectives: Vec<Objective>,
    pub started_at_tick: u64,
    pub time_limit_ticks: u64,
    pub loss: LossConditions,
    pub result: MissionResult,
    pub last_event_tick: u64,
    pub last_event_label: String,
}

impl MissionState {
    pub fn new(objectives: Vec<Objective>, started_at_tick: u64, loss: LossConditions) -> Self {
        let mut state = Self {
            objectives,
            started_at_tick,
            time_limit_ticks: loss.time_limit_ticks,
            loss,
            result: MissionResult::Active,
            last_event_tick: started_at_tick,
            last_event_label: "mission_started".to_string(),
        };
        // First objective starts Pending; the first tick will activate it.
        if state.objectives.iter().all(|o| o.status == ObjectiveStatus::Pending) {
            if let Some(first) = state.objectives.first_mut() {
                first.status = ObjectiveStatus::Active;
            }
        }
        state
    }

    /// Reset the mission to its starting state. Used by `scenario.reset` so the
    /// engine can rewind objectives + result + timer without rebuilding from the
    /// scenario manifest.
    pub fn reset(&mut self, started_at_tick: u64) {
        for o in &mut self.objectives {
            o.status = ObjectiveStatus::Pending;
        }
        if let Some(first) = self.objectives.first_mut() {
            first.status = ObjectiveStatus::Active;
        }
        self.started_at_tick = started_at_tick;
        self.result = MissionResult::Active;
        self.last_event_tick = started_at_tick;
        self.last_event_label = "mission_started".to_string();
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
    pub fn elapsed_ticks(&self, current_tick: u64) -> u64 {
        current_tick.saturating_sub(self.started_at_tick)
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

/// Drive the mission state machine for one tick. Idempotent once the result is
/// terminal (returns an empty report after Won/Lost).
#[must_use]
pub fn step(state: &mut MissionState, inputs: MissionTickInputs<'_>) -> MissionTickReport {
    let mut report = MissionTickReport::default();
    if state.result.is_terminal() {
        return report;
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
            report.final_result = Some(state.result);
            return report;
        }
    }
    if state.time_limit_ticks > 0 && state.elapsed_ticks(inputs.tick) >= state.time_limit_ticks {
        state.result = MissionResult::Lost {
            reason: LossReason::TimerExpired,
        };
        state.last_event_tick = inputs.tick;
        state.last_event_label = "mission_lost_timer".to_string();
        report.final_result = Some(state.result);
        return report;
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
        report.final_result = Some(state.result);
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
    pub result: String,
    pub loss_reason: Option<String>,
    pub elapsed_ticks: u64,
    pub time_limit_ticks: u64,
    pub ticks_remaining: Option<u64>,
    pub active_objective: Option<String>,
    pub objectives: Vec<ObjectiveView>,
    pub last_event_tick: u64,
    pub last_event_label: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObjectiveView {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub optional: bool,
    pub target_actor: Option<u64>,
    pub target_breach: Option<String>,
    pub zone_min: Option<[f32; 2]>,
    pub zone_max: Option<[f32; 2]>,
}

impl MissionView {
    pub fn from_state(state: &MissionState, current_tick: u64) -> Self {
        let active_objective = state.active_objective_index().map(|i| state.objectives[i].id.clone());
        let loss_reason = match state.result {
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
        Self {
            result: state.result.as_str().to_string(),
            loss_reason,
            elapsed_ticks: state.elapsed_ticks(current_tick),
            time_limit_ticks: state.time_limit_ticks,
            ticks_remaining: state.ticks_remaining(current_tick),
            active_objective,
            objectives,
            last_event_tick: state.last_event_tick,
            last_event_label: state.last_event_label.clone(),
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
            },
            Objective {
                id: "neutralize".to_string(),
                kind: ObjectiveKind::NeutralizeActor { target: 2 },
                optional: false,
                status: ObjectiveStatus::Pending,
            },
            Objective {
                id: "extract".to_string(),
                kind: ObjectiveKind::ReachZone {
                    min: [1180.0, 16.0],
                    max: [1280.0, 64.0],
                },
                optional: false,
                status: ObjectiveStatus::Pending,
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
    fn first_objective_active_on_construction() {
        let state = build_state();
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
    }

    #[test]
    fn breach_completion_advances_to_neutralize() {
        let mut state = build_state();
        let actors = mk_actors(player_at(120.0, 32.0), false);
        let mut breaches = BTreeMap::new();
        breaches.insert("outer_wall".to_string(), true);
        let report = step(
            &mut state,
            MissionTickInputs {
                tick: 60,
                player: actors.get(&ActorId(1)),
                actors: &actors,
                breaches_broken: &breaches,
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
            },
        );
        assert!(report.objective_completed.is_empty());
        assert!(report.final_result.is_none());
    }

    #[test]
    fn reset_returns_to_pending_with_first_active() {
        let mut state = build_state();
        state.objectives[0].status = ObjectiveStatus::Completed;
        state.objectives[1].status = ObjectiveStatus::Active;
        state.result = MissionResult::Won;
        state.reset(100);
        assert_eq!(state.objectives[0].status, ObjectiveStatus::Active);
        assert_eq!(state.objectives[1].status, ObjectiveStatus::Pending);
        assert_eq!(state.started_at_tick, 100);
        assert!(matches!(state.result, MissionResult::Active));
    }

    #[test]
    fn mission_view_round_trip() {
        let state = build_state();
        let view = MissionView::from_state(&state, 30);
        assert_eq!(view.result, "active");
        assert_eq!(view.active_objective.as_deref(), Some("breach"));
        assert_eq!(view.objectives.len(), 3);
        assert_eq!(view.objectives[0].kind, "breach_barrier");
        assert_eq!(view.objectives[1].kind, "neutralize_actor");
        assert_eq!(view.objectives[2].kind, "reach_zone");
    }
}
