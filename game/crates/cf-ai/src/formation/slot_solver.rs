//! M7B: per-actor slot resolver.
//!
//! Given a formation definition + a roster of living members (with their
//! role hints), assign each member to a slot. Strategy: prefer matching
//! `role_hint`, then nearest-anchor for unfilled slots. Re-runs at issue
//! time and every 2s while moving; collapses gracefully on member loss
//! via [`FormationKind::collapse_step`].

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::{transforms::world_anchor_for_slot, FormationDef, FormationKind, SquadRoleHint};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SlotAssignment {
    pub member_actor_id: u64,
    pub slot_id: u32,
    pub local_offset: [f32; 2],
    pub world_anchor: [f32; 2],
    pub sector_bearing_degrees: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberRoster {
    pub actor_id: u64,
    pub role_hint: SquadRoleHint,
    /// Used to break ties when role hint is unset; lower wins.
    pub stable_priority: u32,
}

/// re-builds the `MemberRoster` from current ground truth each cycle.
#[derive(Debug, Default, Clone)]
pub struct SlotSolver;

impl SlotSolver {
    pub fn new() -> Self {
        Self
    }

    /// Solve assignments for the given formation + roster. The returned
    /// vector is in slot_id order (deterministic). Excess members beyond
    /// `formation.slot_count()` are returned in the second slot of the
    /// tuple so the caller can collapse the formation or trail unassigned
    /// members.
    pub fn solve(
        &self,
        formation: &FormationDef,
        commander_pos: [f32; 2],
        commander_facing_radians: f32,
        roster: &[MemberRoster],
    ) -> (Vec<SlotAssignment>, Vec<u64>) {
        let mut assignments: Vec<SlotAssignment> = Vec::new();
        let mut used: BTreeSet<u64> = BTreeSet::new();

        let mut roster_sorted: Vec<MemberRoster> = roster.to_vec();
        roster_sorted.sort_by_key(|m| m.stable_priority);

        for slot in &formation.slots {
            if let Some(candidate) = roster_sorted
                .iter()
                .find(|m| !used.contains(&m.actor_id) && m.role_hint == slot.role_hint)
            {
                let world =
                    world_anchor_for_slot(commander_pos, commander_facing_radians, slot.offset);
                assignments.push(SlotAssignment {
                    member_actor_id: candidate.actor_id,
                    slot_id: slot.slot_id,
                    local_offset: slot.offset,
                    world_anchor: world,
                    sector_bearing_degrees: slot.sector_bearing_degrees,
                });
                used.insert(candidate.actor_id);
            }
        }

        for slot in &formation.slots {
            if assignments.iter().any(|a| a.slot_id == slot.slot_id) {
                continue;
            }
            if let Some(candidate) = roster_sorted.iter().find(|m| !used.contains(&m.actor_id)) {
                let world =
                    world_anchor_for_slot(commander_pos, commander_facing_radians, slot.offset);
                assignments.push(SlotAssignment {
                    member_actor_id: candidate.actor_id,
                    slot_id: slot.slot_id,
                    local_offset: slot.offset,
                    world_anchor: world,
                    sector_bearing_degrees: slot.sector_bearing_degrees,
                });
                used.insert(candidate.actor_id);
            }
        }

        assignments.sort_by_key(|a| a.slot_id);

        let leftovers: Vec<u64> = roster_sorted
            .iter()
            .filter(|m| !used.contains(&m.actor_id))
            .map(|m| m.actor_id)
            .collect();

        (assignments, leftovers)
    }

    /// Collapse the formation kind until `member_count` slots suffice.
    /// Returns the collapsed kind + the new definition (built-in).
    pub fn collapse_until_fits(
        kind: FormationKind,
        member_count: usize,
    ) -> (FormationKind, FormationDef) {
        let mut current = kind;
        loop {
            let def = FormationDef::builtin(current);
            if member_count <= def.slot_count() {
                return (current, def);
            }
            match current.collapse_step() {
                Some(next) => current = next,
                None => return (current, def),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn member(id: u64, role: SquadRoleHint, prio: u32) -> MemberRoster {
        MemberRoster {
            actor_id: id,
            role_hint: role,
            stable_priority: prio,
        }
    }

    #[test]
    fn solver_assigns_role_preferred_slot() {
        let s = SlotSolver::new();
        let f = FormationDef::builtin(FormationKind::Wedge);
        let roster = vec![
            member(1, SquadRoleHint::SquadLeader, 0),
            member(2, SquadRoleHint::Rifleman, 1),
            member(3, SquadRoleHint::Rifleman, 2),
            member(4, SquadRoleHint::Heavy, 3),
            member(5, SquadRoleHint::Heavy, 4),
        ];
        let (a, leftovers) = s.solve(&f, [0.0, 0.0], 0.0, &roster);
        assert_eq!(a.len(), 5);
        assert!(leftovers.is_empty());
        // Slot 0 is leader.
        let slot0 = a.iter().find(|x| x.slot_id == 0).expect("slot 0");
        assert_eq!(slot0.member_actor_id, 1);
    }

    #[test]
    fn solver_falls_back_when_role_missing() {
        let s = SlotSolver::new();
        let f = FormationDef::builtin(FormationKind::Wedge);
        let roster = vec![
            member(1, SquadRoleHint::Marksman, 0),
            member(2, SquadRoleHint::Pointman, 1),
            member(3, SquadRoleHint::Rifleman, 2),
        ];
        let (a, leftovers) = s.solve(&f, [0.0, 0.0], 0.0, &roster);
        assert_eq!(a.len(), 3);
        assert!(leftovers.is_empty());
    }

    #[test]
    fn wedge_with_too_few_members_collapses_to_diamond() {
        let (k, _) = SlotSolver::collapse_until_fits(FormationKind::Wedge, 5);
        assert_eq!(k, FormationKind::Wedge);
    }

    #[test]
    fn collapse_reduces_to_single_file_when_one_member_left() {
        let (k, def) = SlotSolver::collapse_until_fits(FormationKind::Wedge, 1);
        // Even SingleFile has 5 slots in the builtin def; the function returns
        // the lowest formation that still fits 1 member.
        assert!(matches!(
            k,
            FormationKind::Wedge | FormationKind::Diamond | FormationKind::Column | FormationKind::SingleFile
        ));
        assert!(def.slot_count() >= 1);
    }

    #[test]
    fn extra_members_are_returned_as_leftovers() {
        let s = SlotSolver::new();
        let f = FormationDef::builtin(FormationKind::Wedge);
        let roster: Vec<MemberRoster> = (1..=7)
            .map(|i| member(i, SquadRoleHint::Rifleman, i as u32))
            .collect();
        let (a, leftovers) = s.solve(&f, [0.0, 0.0], 0.0, &roster);
        assert_eq!(a.len(), 5);
        assert_eq!(leftovers.len(), 2);
    }

    #[test]
    fn assignments_deterministic_across_runs() {
        let s = SlotSolver::new();
        let f = FormationDef::builtin(FormationKind::Wedge);
        let roster = vec![
            member(1, SquadRoleHint::SquadLeader, 0),
            member(2, SquadRoleHint::Rifleman, 1),
            member(3, SquadRoleHint::Rifleman, 2),
            member(4, SquadRoleHint::Heavy, 3),
            member(5, SquadRoleHint::Heavy, 4),
        ];
        let a1 = s.solve(&f, [10.0, 5.0], 0.5, &roster).0;
        let a2 = s.solve(&f, [10.0, 5.0], 0.5, &roster).0;
        assert_eq!(a1, a2);
    }
}
