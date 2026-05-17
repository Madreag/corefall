//! M7B: squad-scoped state container.
//!
//! Spec § "Squad state (verb + formation + priority table + roles) lives on
//! the squad, not the held actor, so doctrine survives the hop." Brain-hop
//! transfers M5 input control between actors WITHOUT mutating the squad
//! state row owned here.
//!
//! `squad_state.rs` does NOT own the squad's MEMBER LIST (that's
//! `cf-squad::Squad`). It owns the squad-level command surface: current
//! command (verb + args), current formation kind, per-actor role
//! assignments, per-actor slot assignments, breach-chain progress, and
//! bounding-step state.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::autonomy::DoctrineMode;
use crate::formation::{slot_solver::SlotAssignment, FormationKind, SlotSolver, SquadRoleHint};
use crate::squad_command_grammar::{
    builtin_registry, try_issue, CommandIssue, DoctrineCompatMatrix, SquadCommand, VerbArgValue,
    VerbRegistry,
};

/// **M7B**: spec § "Slot solver runs at issue + every 2s while moving" —
/// the cadence in seconds. The engine converts this to ticks via the
/// configured tick rate.
pub const SLOT_RESLOT_CADENCE_SECONDS: f32 = 2.0;

/// **M7B**: distance threshold (world units) past which an actor is
/// considered to have "lost the slot position" and the engine emits a
/// `squad.formation_slot_broken`. Re-solve happens next 2s tick.
pub const SLOT_BROKEN_THRESHOLD_UNITS: f32 = 12.0;

/// **M7B**: identifies one squad inside the world. Owned by cf-control
/// (mission roster), exposed here as an opaque u64 so cf-ai stays free of
/// the cf-squad type graph.
pub type SquadId = u64;

/// **M7B**: per-squad state owned by cf-ai. Mutated through `apply_*`
/// helpers that emit replay events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadState {
    pub squad_id: SquadId,
    pub doctrine: DoctrineMode,
    /// Current accepted command, if any.
    pub current_command: Option<SquadCommand>,
    /// Last veto reason label (for Tab overlay flash).
    pub last_veto_label: Option<String>,
    /// Current formation kind.
    pub formation_kind: FormationKind,
    /// Latest slot assignments (after the most recent solve).
    pub slot_assignments: Vec<SlotAssignment>,
    /// Per-actor role assignment (sticky across reslot until KIA / brain-hop).
    pub role_assignments: BTreeMap<u64, SquadRoleHint>,
    /// Active breach chain (Some while progressing).
    pub breach_chain: Option<BreachChainState>,
    /// Active bounding retreat (Some while in `retreat_in_order`).
    pub bounding: Option<BoundingState>,
    /// Tick of the last slot solve. Engine reslots when
    /// `tick - last_solve_tick >= reslot_interval_ticks` AND the squad is
    /// moving.
    pub last_solve_tick: u64,
}

impl SquadState {
    pub fn new(squad_id: SquadId, doctrine: DoctrineMode) -> Self {
        Self {
            squad_id,
            doctrine,
            current_command: None,
            last_veto_label: None,
            formation_kind: FormationKind::Wedge,
            slot_assignments: Vec::new(),
            role_assignments: BTreeMap::new(),
            breach_chain: None,
            bounding: None,
            last_solve_tick: 0,
        }
    }

    /// **M7B**: assign a sticky role to a member.
    pub fn assign_role(&mut self, actor_id: u64, role: SquadRoleHint) -> RoleAssignmentResult {
        let prev = self.role_assignments.insert(actor_id, role);
        match prev {
            Some(p) if p == role => RoleAssignmentResult::Unchanged,
            Some(p) => RoleAssignmentResult::Changed { previous: p },
            None => RoleAssignmentResult::Assigned,
        }
    }

    /// **M7B**: try to issue a verb. On `Accepted`, commit; on `Vetoed`,
    /// stash the reason label without mutating `current_command`; on
    /// `Rejected`, return the parse error label.
    pub fn try_issue(
        &mut self,
        registry: &VerbRegistry,
        matrix: &DoctrineCompatMatrix,
        verb_id: &str,
        args: Vec<VerbArgValue>,
        issuer_actor_id: u64,
        issued_tick: u64,
    ) -> CommandIssue {
        let outcome = try_issue(
            registry,
            matrix,
            self.doctrine,
            verb_id,
            args,
            issuer_actor_id,
            issued_tick,
        );
        match &outcome {
            CommandIssue::Accepted(cmd) => {
                self.current_command = Some(cmd.clone());
                self.last_veto_label = None;
            }
            CommandIssue::Vetoed { reason_label, .. } => {
                self.last_veto_label = Some(reason_label.clone());
            }
            CommandIssue::Rejected { reason_label } => {
                self.last_veto_label = Some(reason_label.clone());
            }
        }
        outcome
    }

    /// Helper combining the builtin registry + builtin doctrine matrix.
    pub fn try_issue_builtin(
        &mut self,
        verb_id: &str,
        args: Vec<VerbArgValue>,
        issuer_actor_id: u64,
        issued_tick: u64,
    ) -> CommandIssue {
        let reg = builtin_registry();
        let mat = DoctrineCompatMatrix::builtin();
        self.try_issue(&reg, &mat, verb_id, args, issuer_actor_id, issued_tick)
    }

    /// **M7B**: re-set the formation kind. Returns the previous kind.
    pub fn set_formation(&mut self, kind: FormationKind) -> FormationKind {
        let prev = self.formation_kind;
        self.formation_kind = kind;
        prev
    }

    /// **M7B**: produce a fresh set of slot assignments via the solver +
    /// commit them back into the squad state. The roster is built from
    /// `role_assignments`; the caller passes the commander pose + the
    /// current tick. Returns the new assignments.
    pub fn solve_slots(
        &mut self,
        commander_pos: [f32; 2],
        commander_facing_radians: f32,
        tick: u64,
    ) -> Vec<SlotAssignment> {
        let (kind, def) =
            SlotSolver::collapse_until_fits(self.formation_kind, self.role_assignments.len());
        if kind != self.formation_kind {
            self.formation_kind = kind;
        }
        let roster: Vec<crate::formation::slot_solver::MemberRoster> = self
            .role_assignments
            .iter()
            .enumerate()
            .map(|(idx, (actor_id, role))| crate::formation::slot_solver::MemberRoster {
                actor_id: *actor_id,
                role_hint: *role,
                stable_priority: idx as u32,
            })
            .collect();
        let solver = SlotSolver::new();
        let (a, _leftovers) = solver.solve(&def, commander_pos, commander_facing_radians, &roster);
        self.slot_assignments = a.clone();
        self.last_solve_tick = tick;
        a
    }

    /// **M7B**: register member death. Returns true when the role was
    /// removed (the engine then triggers a reslot).
    pub fn on_member_kia(&mut self, actor_id: u64) -> bool {
        self.role_assignments.remove(&actor_id).is_some()
    }

    /// **M7B**: true when the squad is due for its periodic 2s reslot.
    /// `now_tick` is the engine tick; `tick_rate_hz` is the configured
    /// tick rate (never hardcoded to 60 per AGENTS.md). The engine
    /// short-circuits when the squad is idle (no current command).
    pub fn is_due_for_reslot(&self, now_tick: u64, tick_rate_hz: u32) -> bool {
        if self.current_command.is_none() {
            return false;
        }
        let rate = tick_rate_hz.max(1);
        let cadence_ticks =
            (SLOT_RESLOT_CADENCE_SECONDS * rate as f32).round().max(1.0) as u64;
        now_tick.saturating_sub(self.last_solve_tick) >= cadence_ticks
    }

    /// **M7B**: per-member slot-broken detection. Returns the set of
    /// `(member_actor_id, slot_id)` pairs that have wandered past
    /// `SLOT_BROKEN_THRESHOLD_UNITS` from their assigned world anchor.
    /// The engine emits one `squad.formation_slot_broken` event per pair
    /// and schedules the reslot on the next 2s tick.
    pub fn detect_broken_slots(
        &self,
        positions: &BTreeMap<u64, [f32; 2]>,
    ) -> Vec<SlotBrokenReport> {
        let mut out = Vec::new();
        for assignment in &self.slot_assignments {
            let Some(pos) = positions.get(&assignment.member_actor_id) else {
                continue;
            };
            let dx = pos[0] - assignment.world_anchor[0];
            let dy = pos[1] - assignment.world_anchor[1];
            let d2 = dx * dx + dy * dy;
            if d2 > SLOT_BROKEN_THRESHOLD_UNITS * SLOT_BROKEN_THRESHOLD_UNITS {
                out.push(SlotBrokenReport {
                    member_actor_id: assignment.member_actor_id,
                    slot_id: assignment.slot_id,
                    distance: d2.sqrt(),
                    threshold: SLOT_BROKEN_THRESHOLD_UNITS,
                });
            }
        }
        out
    }

    /// **M7B**: open a breach chain (Stack → Breach → Frag → Advance).
    pub fn start_breach_chain(&mut self, door_id: u64, side: &str, tick: u64) {
        self.breach_chain = Some(BreachChainState {
            door_id,
            side: side.to_string(),
            started_tick: tick,
            current_step: BreachChainStep::Stack,
        });
    }

    /// **M7B**: advance the breach chain one step. Returns the step the
    /// chain entered, or `None` when the chain has completed.
    pub fn advance_breach_chain(&mut self, tick: u64) -> Option<BreachChainStep> {
        let _ = tick;
        let chain = self.breach_chain.as_mut()?;
        if let Some(step) = chain.current_step.next() {
            chain.current_step = step;
            Some(step)
        } else {
            self.breach_chain = None;
            None
        }
    }

    /// **M7B**: open a bounding-retreat sequence (alternating cover + 30u
    /// rearward steps).
    pub fn start_bounding(&mut self, rally: [f32; 2], tick: u64) {
        self.bounding = Some(BoundingState {
            rally,
            started_tick: tick,
            steps_taken: 0,
            current_phase: BoundingPhase::CoverHalf,
        });
    }

    /// **M7B**: tick the bounding sequence. Returns the resulting event
    /// payload when a swap occurs.
    pub fn tick_bounding(&mut self) -> Option<BoundingEvent> {
        let state = self.bounding.as_mut()?;
        state.steps_taken += 1;
        let prev_phase = state.current_phase;
        state.current_phase = state.current_phase.swap();
        Some(BoundingEvent {
            step_index: state.steps_taken,
            previous_phase: prev_phase,
            new_phase: state.current_phase,
        })
    }
}

/// **M7B**: outcome of `assign_role`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoleAssignmentResult {
    Assigned,
    Unchanged,
    Changed { previous: SquadRoleHint },
}

/// **M7B**: breach-chain step ladder.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BreachChainStep {
    Stack,
    Breach,
    Frag,
    Advance,
}

impl BreachChainStep {
    pub fn as_str(self) -> &'static str {
        match self {
            BreachChainStep::Stack => "stack",
            BreachChainStep::Breach => "breach",
            BreachChainStep::Frag => "frag",
            BreachChainStep::Advance => "advance",
        }
    }

    pub fn next(self) -> Option<BreachChainStep> {
        Some(match self {
            BreachChainStep::Stack => BreachChainStep::Breach,
            BreachChainStep::Breach => BreachChainStep::Frag,
            BreachChainStep::Frag => BreachChainStep::Advance,
            BreachChainStep::Advance => return None,
        })
    }

    pub fn ordinal(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BreachChainState {
    pub door_id: u64,
    pub side: String,
    pub started_tick: u64,
    pub current_step: BreachChainStep,
}

/// **M7B**: bounding-retreat alternating phase.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundingPhase {
    CoverHalf,
    MoveHalf,
}

impl BoundingPhase {
    pub fn as_str(self) -> &'static str {
        match self {
            BoundingPhase::CoverHalf => "cover_half",
            BoundingPhase::MoveHalf => "move_half",
        }
    }

    #[must_use]
    pub fn swap(self) -> BoundingPhase {
        match self {
            BoundingPhase::CoverHalf => BoundingPhase::MoveHalf,
            BoundingPhase::MoveHalf => BoundingPhase::CoverHalf,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundingState {
    pub rally: [f32; 2],
    pub started_tick: u64,
    pub steps_taken: u32,
    pub current_phase: BoundingPhase,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoundingEvent {
    pub step_index: u32,
    pub previous_phase: BoundingPhase,
    pub new_phase: BoundingPhase,
}

/// **M7B**: one slot-broken detection produced by [`SquadState::detect_broken_slots`].
#[derive(Debug, Clone, PartialEq)]
pub struct SlotBrokenReport {
    pub member_actor_id: u64,
    pub slot_id: u32,
    pub distance: f32,
    pub threshold: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_squad_state_defaults_to_wedge() {
        let s = SquadState::new(1, DoctrineMode::Defensive);
        assert_eq!(s.formation_kind, FormationKind::Wedge);
        assert!(s.current_command.is_none());
    }

    #[test]
    fn issue_press_attack_vetoed_under_defensive() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        let out = s.try_issue_builtin("press_attack", vec![], 99, 100);
        assert!(matches!(out, CommandIssue::Vetoed { .. }));
        assert!(s.current_command.is_none());
        assert!(s.last_veto_label.as_deref().unwrap().contains("defensive"));
    }

    #[test]
    fn re_issue_after_doctrine_switch_accepted() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        let _ = s.try_issue_builtin("press_attack", vec![], 99, 100);
        s.doctrine = DoctrineMode::Aggressive;
        let out = s.try_issue_builtin("press_attack", vec![], 99, 101);
        assert!(out.is_accepted());
        assert!(s.current_command.is_some());
    }

    #[test]
    fn role_assignment_tracks_changes() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        assert_eq!(
            s.assign_role(1, SquadRoleHint::SquadLeader),
            RoleAssignmentResult::Assigned
        );
        assert_eq!(
            s.assign_role(1, SquadRoleHint::SquadLeader),
            RoleAssignmentResult::Unchanged
        );
        assert!(matches!(
            s.assign_role(1, SquadRoleHint::Heavy),
            RoleAssignmentResult::Changed { previous: SquadRoleHint::SquadLeader }
        ));
    }

    #[test]
    fn breach_chain_emits_four_steps() {
        let mut s = SquadState::new(1, DoctrineMode::Aggressive);
        s.start_breach_chain(42, "left", 100);
        assert!(s.breach_chain.is_some());
        // Already at Stack; advance gives Breach, Frag, Advance, then None.
        assert_eq!(s.advance_breach_chain(101), Some(BreachChainStep::Breach));
        assert_eq!(s.advance_breach_chain(102), Some(BreachChainStep::Frag));
        assert_eq!(s.advance_breach_chain(103), Some(BreachChainStep::Advance));
        assert_eq!(s.advance_breach_chain(104), None);
        assert!(s.breach_chain.is_none());
    }

    #[test]
    fn member_kia_triggers_reslot_signal() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        s.assign_role(1, SquadRoleHint::SquadLeader);
        assert!(s.on_member_kia(1));
        assert!(!s.on_member_kia(1));
    }

    #[test]
    fn solve_slots_records_assignments() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        s.assign_role(1, SquadRoleHint::SquadLeader);
        s.assign_role(2, SquadRoleHint::Rifleman);
        s.assign_role(3, SquadRoleHint::Rifleman);
        s.assign_role(4, SquadRoleHint::Heavy);
        s.assign_role(5, SquadRoleHint::Heavy);
        let a = s.solve_slots([0.0, 0.0], 0.0, 100);
        assert_eq!(a.len(), 5);
        assert_eq!(s.last_solve_tick, 100);
    }

    #[test]
    fn bounding_phase_alternates() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        s.start_bounding([10.0, 0.0], 100);
        let step1 = s.tick_bounding().unwrap();
        let step2 = s.tick_bounding().unwrap();
        assert_eq!(step1.previous_phase, BoundingPhase::CoverHalf);
        assert_eq!(step1.new_phase, BoundingPhase::MoveHalf);
        assert_eq!(step2.previous_phase, BoundingPhase::MoveHalf);
        assert_eq!(step2.new_phase, BoundingPhase::CoverHalf);
    }

    #[test]
    fn is_due_for_reslot_returns_false_when_idle() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        s.assign_role(1, SquadRoleHint::SquadLeader);
        s.solve_slots([0.0, 0.0], 0.0, 100);
        assert!(
            !s.is_due_for_reslot(1000, 60),
            "idle squad (no current_command) must not trigger reslot"
        );
    }

    #[test]
    fn is_due_for_reslot_fires_after_2_seconds() {
        let mut s = SquadState::new(1, DoctrineMode::Aggressive);
        s.assign_role(1, SquadRoleHint::SquadLeader);
        let out = s.try_issue_builtin("advance", vec![], 99, 100);
        assert!(out.is_accepted(), "advance must be accepted under aggressive");
        s.solve_slots([0.0, 0.0], 0.0, 100);
        let tick_rate = 60;
        let two_seconds = (SLOT_RESLOT_CADENCE_SECONDS * tick_rate as f32) as u64;
        assert!(!s.is_due_for_reslot(100 + two_seconds / 2, tick_rate));
        assert!(s.is_due_for_reslot(100 + two_seconds, tick_rate));
    }

    #[test]
    fn detect_broken_slots_returns_empty_when_all_in_position() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        s.assign_role(1, SquadRoleHint::SquadLeader);
        s.assign_role(2, SquadRoleHint::Rifleman);
        s.assign_role(3, SquadRoleHint::Rifleman);
        s.assign_role(4, SquadRoleHint::Heavy);
        s.assign_role(5, SquadRoleHint::Heavy);
        let assignments = s.solve_slots([0.0, 0.0], 0.0, 100);
        let mut positions = BTreeMap::new();
        for a in &assignments {
            positions.insert(a.member_actor_id, a.world_anchor);
        }
        assert!(s.detect_broken_slots(&positions).is_empty());
    }

    #[test]
    fn detect_broken_slots_reports_actor_far_from_anchor() {
        let mut s = SquadState::new(1, DoctrineMode::Defensive);
        s.assign_role(1, SquadRoleHint::SquadLeader);
        s.assign_role(2, SquadRoleHint::Rifleman);
        let assignments = s.solve_slots([0.0, 0.0], 0.0, 100);
        let leader_id = assignments[0].member_actor_id;
        let mut positions = BTreeMap::new();
        positions.insert(leader_id, [999.0, 999.0]);
        for a in &assignments[1..] {
            positions.insert(a.member_actor_id, a.world_anchor);
        }
        let broken = s.detect_broken_slots(&positions);
        assert_eq!(broken.len(), 1, "exactly the wandering actor should flag");
        assert_eq!(broken[0].member_actor_id, leader_id);
        assert!(broken[0].distance > broken[0].threshold);
    }
}
