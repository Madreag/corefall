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
    clippy::single_match_else
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
    BreachBarrier {
        target: String,
    },
    NeutralizeActor {
        target: u64,
    },
    ReachZone {
        min: [f32; 2],
        max: [f32; 2],
    },
    /// M2.5: defend a reactor (named static actor) until either the mission
    /// timer expires (success) or the reactor's hp reaches zero (failure).
    /// `target` is the reactor id.
    DefendReactor {
        target: String,
    },
}

impl ObjectiveKind {
    pub fn category(&self) -> &'static str {
        match self {
            ObjectiveKind::BreachBarrier { .. } => "breach_barrier",
            ObjectiveKind::NeutralizeActor { .. } => "neutralize_actor",
            ObjectiveKind::ReachZone { .. } => "reach_zone",
            ObjectiveKind::DefendReactor { .. } => "defend_reactor",
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
    /// M2.5: a `defend_reactor` objective failed because the reactor was
    /// destroyed before the mission timer expired.
    ReactorDestroyed,
    /// M2.5+: a defend_target objective failed for a generic reason. Not
    /// emitted in BP2 by default; reserved.
    ObjectiveFailed,
}

impl LossReason {
    pub fn as_str(self) -> &'static str {
        match self {
            LossReason::PlayerDead => "player_dead",
            LossReason::TimerExpired => "timer_expired",
            LossReason::ReactorDestroyed => "reactor_destroyed",
            LossReason::ObjectiveFailed => "objective_failed",
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
    let reactor_destroyed_match: Option<(usize, String, String)> =
        state.objectives.iter().enumerate().find_map(|(idx, obj)| {
            if obj.status != ObjectiveStatus::Active {
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
        report.final_result = Some(state.result);
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
                report.final_result = Some(state.result);
            } else {
                state.result = MissionResult::Lost {
                    reason: LossReason::TimerExpired,
                };
                state.last_event_tick = inputs.tick;
                state.last_event_label = "mission_lost_timer".to_string();
                report.final_result = Some(state.result);
            }
            return report;
        }
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
            ObjectiveKind::DefendReactor { .. } => {
                // DefendReactor only completes via the timer-expired branch
                // above; passive ticks never auto-complete it.
                false
            }
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
    pub target_reactor: Option<String>,
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
                reactors_destroyed: &BTreeMap::new(),
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

    fn build_reactor_defense_state(time_limit_ticks: u64) -> MissionState {
        let objectives = vec![Objective {
            id: "defend_reactor".to_string(),
            kind: ObjectiveKind::DefendReactor {
                target: "core_reactor".to_string(),
            },
            optional: false,
            status: ObjectiveStatus::Pending,
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
            },
            Objective {
                id: "reach".to_string(),
                kind: ObjectiveKind::ReachZone {
                    min: [1180.0, 16.0],
                    max: [1280.0, 64.0],
                },
                optional: false,
                status: ObjectiveStatus::Pending,
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
            },
        );
        assert!(matches!(state.result, MissionResult::Won));
        assert_eq!(report.objective_completed, vec!["defend_reactor".to_string()]);
        assert!(report.final_result.is_some());
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
