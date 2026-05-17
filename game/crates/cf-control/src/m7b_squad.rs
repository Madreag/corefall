//! M7B: engine-side integration for the deep squad command grammar.
//!
//! Spec § "Verb registry + doctrine-compatibility matrix MUST be data-driven
//! RON so M25 wheel / Tab overlay / M23B commander-doctrine re-enumerate
//! without a Rust rebuild." This module owns the per-squad `SquadState`
//! (formation, current verb, role assignments, breach-chain progress,
//! bounding-step state) and exposes JSON-payload builders the engine emits
//! through the recorder.
//!
//! Squad state lives here — NOT on the held actor — so brain-hop preserves
//! doctrine + formation + role assignments per spec.

use std::collections::BTreeMap;

use serde_json::{json, Value};

use cf_ai::{
    archetype_bt::ArchetypeBtKind,
    commander_hop::CommanderHopState,
    formation::{FormationKind, SquadRoleHint},
    squad_command_grammar::{builtin_registry, CommandIssue, DoctrineCompatMatrix, VerbArgValue, VerbRegistry},
    squad_state::{SlotBrokenReport, SquadState},
    DoctrineMode,
};

/// **M7B**: convenience id for the engine's player squad. At M7B the
/// player commands at most one squad; M25 widens to a small fleet.
pub const PLAYER_SQUAD_ID: u64 = 0;

/// **M7B**: engine-side container for per-squad state. Owned by the
/// `EngineMutable` so brain-hop / scenario reset can find it deterministically.
#[derive(Debug, Clone, Default)]
pub struct M7BSquadWorld {
    pub squads: BTreeMap<u64, SquadState>,
    pub registry: VerbRegistry,
    pub matrix: DoctrineCompatMatrix,
    /// Optional in-flight commander-hop transition (one at a time per
    /// session — the M25 fleet expansion may relax that).
    pub commander_hop: Option<CommanderHopState>,
}

impl M7BSquadWorld {
    pub fn new() -> Self {
        Self {
            squads: BTreeMap::new(),
            registry: builtin_registry(),
            matrix: DoctrineCompatMatrix::builtin(),
            commander_hop: None,
        }
    }

    /// Ensure a `SquadState` exists for the given id. Returns a mutable
    /// reference to the row.
    pub fn ensure_squad(&mut self, squad_id: u64, doctrine: DoctrineMode) -> &mut SquadState {
        self.squads
            .entry(squad_id)
            .or_insert_with(|| SquadState::new(squad_id, doctrine))
    }

    pub fn squad(&self, squad_id: u64) -> Option<&SquadState> {
        self.squads.get(&squad_id)
    }

    pub fn squad_mut(&mut self, squad_id: u64) -> Option<&mut SquadState> {
        self.squads.get_mut(&squad_id)
    }

    /// **M7B**: issue a verb against a squad. Returns the issue outcome +
    /// the JSON payload the engine should emit on the recorder
    /// (either `squad.command_issued` or `squad.command_vetoed`).
    pub fn issue_verb(
        &mut self,
        squad_id: u64,
        verb_id: &str,
        args: Vec<VerbArgValue>,
        issuer_actor_id: u64,
        issued_tick: u64,
    ) -> IssueOutcome {
        let registry = self.registry.clone();
        let matrix = self.matrix.clone();
        let squad = self.ensure_squad(squad_id, DoctrineMode::Defensive);
        let res = squad.try_issue(&registry, &matrix, verb_id, args, issuer_actor_id, issued_tick);
        let payload = match &res {
            CommandIssue::Accepted(cmd) => {
                let args_json: Vec<Value> = cmd.args.iter().map(arg_to_json).collect();
                json!({
                    "squad_id": squad_id,
                    "verb_id": cmd.verb_id,
                    "verb_family": cmd.family.as_str(),
                    "args": args_json,
                    "issuer_actor_id": cmd.issuer_actor_id,
                    "issued_tick": cmd.issued_tick,
                })
            }
            CommandIssue::Vetoed {
                reason_label,
                attempted_verb_id,
                doctrine,
            } => json!({
                "squad_id": squad_id,
                "verb_id": attempted_verb_id,
                "doctrine": doctrine.as_str(),
                "reason_label": reason_label,
                "issuer_actor_id": issuer_actor_id,
                "issued_tick": issued_tick,
            }),
            CommandIssue::Rejected { reason_label } => json!({
                "squad_id": squad_id,
                "verb_id": verb_id,
                "doctrine": squad.doctrine.as_str(),
                "reason_label": reason_label,
                "issuer_actor_id": issuer_actor_id,
                "issued_tick": issued_tick,
            }),
        };
        IssueOutcome { outcome: res, payload }
    }

    /// **M7B**: switch the formation kind for a squad.
    pub fn set_formation(
        &mut self,
        squad_id: u64,
        kind: FormationKind,
        commander_pos: [f32; 2],
        commander_facing_radians: f32,
        commander_actor_id: Option<u64>,
        tick: u64,
    ) -> FormationSetOutcome {
        let squad = self.ensure_squad(squad_id, DoctrineMode::Defensive);
        let previous = squad.set_formation(kind);
        let assignments = squad.solve_slots(commander_pos, commander_facing_radians, tick);
        let assignment_payloads: Vec<Value> = assignments
            .iter()
            .map(|a| {
                json!({
                    "squad_id": squad_id,
                    "member_actor_id": a.member_actor_id,
                    "slot_id": a.slot_id,
                    "local_offset": a.local_offset,
                    "world_anchor": a.world_anchor,
                    "sector_bearing_degrees": a.sector_bearing_degrees,
                })
            })
            .collect();
        let formation_payload = json!({
            "squad_id": squad_id,
            "formation_kind": squad.formation_kind.as_str(),
            "previous_kind": previous.as_str(),
            "slot_count": assignments.len(),
            "slot_assignments": assignment_payloads.clone(),
            "commander_actor_id": commander_actor_id,
        });
        FormationSetOutcome {
            previous,
            new_kind: squad.formation_kind,
            formation_payload,
            assignment_payloads,
        }
    }

    /// **M7B**: assign a sticky role to a member.
    pub fn assign_role(
        &mut self,
        squad_id: u64,
        member_actor_id: u64,
        role: SquadRoleHint,
    ) -> RoleAssignmentOutcome {
        let squad = self.ensure_squad(squad_id, DoctrineMode::Defensive);
        let result = squad.assign_role(member_actor_id, role);
        let previous_role = match result {
            cf_ai::squad_state::RoleAssignmentResult::Changed { previous } => Some(previous.as_str().to_string()),
            _ => None,
        };
        let payload = json!({
            "squad_id": squad_id,
            "member_actor_id": member_actor_id,
            "role": role.as_str(),
            "previous_role": previous_role,
        });
        RoleAssignmentOutcome { result, payload }
    }

    /// **M7B**: open a breach chain (Stack → Breach → Frag → Advance).
    /// Returns the `squad.breach_chain_started` payload. Pre-assigns
    /// sectors-of-fire from the `StackDoor` formation slot definitions:
    /// Stack-1 ahead, Stack-2 left, Stack-3 right, Stack-4 rear per spec.
    pub fn start_breach_chain(
        &mut self,
        squad_id: u64,
        door_id: u64,
        side: &str,
        stack_actor_ids: Vec<u64>,
        tick: u64,
    ) -> Value {
        let squad = self.ensure_squad(squad_id, DoctrineMode::Defensive);
        squad.start_breach_chain(door_id, side, tick);
        // Pre-assign sectors-of-fire per spec § "per-actor sectors-of-fire
        // are pre-assigned per slot (Stack-1 ahead, Stack-2 left, Stack-3
        // right, Stack-4 rear)". Take the StackDoor formation builtin
        // slot order and pair with the supplied actor ids.
        let stack_def = cf_ai::formation::FormationDef::builtin(FormationKind::StackDoor);
        let sectors: Vec<Value> = stack_actor_ids
            .iter()
            .zip(stack_def.slots.iter())
            .map(|(actor_id, slot)| {
                json!({
                    "actor_id": *actor_id,
                    "slot_id": slot.slot_id,
                    "sector_bearing_degrees": slot.sector_bearing_degrees,
                    "role_hint": slot.role_hint.as_str(),
                })
            })
            .collect();
        json!({
            "squad_id": squad_id,
            "door_id": door_id,
            "side": side,
            "started_tick": tick,
            "stack_actor_ids": stack_actor_ids,
            "sectors_of_fire": sectors,
        })
    }

    /// **M7B**: advance the breach chain. Returns:
    /// - `step_payload`: `squad.breach_chain_step` payload (always present
    ///   until the chain completes).
    /// - `complete_payload`: `squad.breach_chain_complete` payload (Some
    ///   when the last `Advance` step has fired).
    pub fn advance_breach_chain(
        &mut self,
        squad_id: u64,
        tick: u64,
    ) -> BreachChainAdvance {
        self.advance_breach_chain_with_actors(squad_id, tick, &[])
    }

    /// **M7B**: like `advance_breach_chain` but stamps the per-step actor
    /// ids + sectors-of-fire from the StackDoor formation. The engine
    /// calls this with the resolved stack actor ids so the replay event
    /// carries the full per-step assignment.
    pub fn advance_breach_chain_with_actors(
        &mut self,
        squad_id: u64,
        tick: u64,
        stack_actor_ids: &[u64],
    ) -> BreachChainAdvance {
        let squad = self.ensure_squad(squad_id, DoctrineMode::Defensive);
        let door_id = squad.breach_chain.as_ref().map(|c| c.door_id);
        let started_tick = squad.breach_chain.as_ref().map(|c| c.started_tick);
        let next = squad.advance_breach_chain(tick);
        let stack_def = cf_ai::formation::FormationDef::builtin(FormationKind::StackDoor);
        let sectors: Vec<Value> = stack_actor_ids
            .iter()
            .zip(stack_def.slots.iter())
            .map(|(actor_id, slot)| {
                json!({
                    "actor_id": *actor_id,
                    "slot_id": slot.slot_id,
                    "sector_bearing_degrees": slot.sector_bearing_degrees,
                    "role_hint": slot.role_hint.as_str(),
                })
            })
            .collect();
        match (next, door_id) {
            (Some(step), Some(d)) => BreachChainAdvance {
                step_payload: Some(json!({
                    "squad_id": squad_id,
                    "door_id": d,
                    "step": step.as_str(),
                    "step_ordinal": step.ordinal(),
                    "tick": tick,
                    "actor_ids": stack_actor_ids,
                    "sectors_of_fire": sectors,
                })),
                complete_payload: None,
            },
            (None, Some(d)) => BreachChainAdvance {
                step_payload: None,
                complete_payload: Some(json!({
                    "squad_id": squad_id,
                    "door_id": d,
                    "completed_tick": tick,
                    "duration_ticks": started_tick.map(|s| tick.saturating_sub(s)),
                    "actor_ids": stack_actor_ids,
                })),
            },
            _ => BreachChainAdvance {
                step_payload: None,
                complete_payload: None,
            },
        }
    }

    /// **M7B**: tick the bounding-retreat sequence one swap. Returns the
    /// `squad.bounding_step` payload if a swap occurred. Half the squad
    /// covers while the other half moves rearward; the cover/move actor
    /// split is sourced from the current `role_assignments` (Heavy +
    /// Marksman + SquadLeader cover; Pointman + Rifleman + Engineer move).
    pub fn tick_bounding(&mut self, squad_id: u64, rally: Option<[f32; 2]>) -> Option<Value> {
        let squad = self.squad_mut(squad_id)?;
        let event = squad.tick_bounding()?;
        let (cover_actors, moving_actors) = bounding_split(&squad.role_assignments, event.new_phase);
        Some(json!({
            "squad_id": squad_id,
            "step_index": event.step_index,
            "previous_phase": event.previous_phase.as_str(),
            "new_phase": event.new_phase.as_str(),
            "cover_actors": cover_actors,
            "moving_actors": moving_actors,
            "rally_position": rally.unwrap_or([0.0, 0.0]),
        }))
    }

    /// **M7B**: produce one `squad.formation_slot_broken` payload per
    /// member-actor that has wandered out of its assigned slot position.
    /// `positions` is the engine-supplied world position table.
    pub fn detect_and_report_broken_slots(
        &self,
        squad_id: u64,
        positions: &BTreeMap<u64, [f32; 2]>,
        next_solve_tick: u64,
    ) -> Vec<Value> {
        let Some(squad) = self.squads.get(&squad_id) else {
            return Vec::new();
        };
        squad
            .detect_broken_slots(positions)
            .into_iter()
            .map(|r| slot_broken_payload(squad_id, &r, next_solve_tick))
            .collect()
    }

    /// **M7B**: 2s periodic reslot driver. The engine calls this once per
    /// tick; when the squad is moving + the cadence has elapsed, the
    /// solver re-runs and the engine emits the resulting
    /// `squad.formation_set` + per-slot `squad.formation_slot_assigned`
    /// events.
    pub fn tick_periodic_reslot(
        &mut self,
        squad_id: u64,
        commander_pos: [f32; 2],
        commander_facing_radians: f32,
        commander_actor_id: Option<u64>,
        current_tick: u64,
        tick_rate_hz: u32,
    ) -> Option<FormationSetOutcome> {
        let due = self
            .squads
            .get(&squad_id)
            .is_some_and(|s| s.is_due_for_reslot(current_tick, tick_rate_hz));
        if !due {
            return None;
        }
        Some(self.set_formation(
            squad_id,
            self.squads.get(&squad_id)?.formation_kind,
            commander_pos,
            commander_facing_radians,
            commander_actor_id,
            current_tick,
        ))
    }

    /// **M7B**: notify the world that a squad member has been killed in
    /// action. Removes the role assignment, triggers an immediate reslot
    /// (per spec § "collapses gracefully on member loss"), and returns
    /// the resulting collapse + reslot payloads for the engine to emit.
    pub fn on_member_kia(
        &mut self,
        squad_id: u64,
        actor_id: u64,
        commander_pos: [f32; 2],
        commander_facing_radians: f32,
        commander_actor_id: Option<u64>,
        current_tick: u64,
    ) -> KiaOutcome {
        let squad = match self.squad_mut(squad_id) {
            Some(s) => s,
            None => {
                return KiaOutcome {
                    removed: false,
                    reslot: None,
                }
            }
        };
        let removed = squad.on_member_kia(actor_id);
        if !removed {
            return KiaOutcome {
                removed: false,
                reslot: None,
            };
        }
        let kind = squad.formation_kind;
        let reslot = self.set_formation(
            squad_id,
            kind,
            commander_pos,
            commander_facing_radians,
            commander_actor_id,
            current_tick,
        );
        KiaOutcome {
            removed: true,
            reslot: Some(reslot),
        }
    }

    /// **M7B**: open + immediately commit a brain-hop. Returns the
    /// `squad.brain_hop` payload.
    pub fn brain_hop_payload(
        &mut self,
        squad_id: u64,
        from_actor_id: u64,
        to_actor_id: u64,
        tick: u64,
        same_squad: bool,
    ) -> Value {
        let state = CommanderHopState::open(from_actor_id, tick);
        let result = cf_ai::commander_hop::finalize_hop(&state, to_actor_id, squad_id, same_squad, tick).ok();
        json!({
            "squad_id": squad_id,
            "from_actor_id": from_actor_id,
            "to_actor_id": to_actor_id,
            "initiated_tick": tick,
            "completed_tick": tick,
            "same_squad": same_squad,
            "valid": result.is_some(),
        })
    }

    /// **M7B**: produce the full `srv.dump_squad_state` JSON view for a
    /// squad. Includes the squad-state row + verb registry + formation
    /// catalog + archetype-BT node counts so the M25 wheel / Tab overlay
    /// renders without a second round-trip.
    pub fn dump_state_view(&self, squad_id: u64) -> Value {
        let squad = self.squads.get(&squad_id);
        let verbs_json: Vec<Value> = self
            .registry
            .iter()
            .map(|def| {
                let args: Vec<Value> = def
                    .args
                    .iter()
                    .map(|a| {
                        json!({
                            "name": a.name,
                            "kind": a.kind.as_str(),
                            "required": a.required,
                        })
                    })
                    .collect();
                // Per-verb doctrine-compatibility row per spec § "each
                // with name + arg schema + valid-target predicate +
                // doctrine-compat row". Lists every doctrine and whether
                // that doctrine accepts or vetoes this verb.
                let mut doctrine_row = serde_json::Map::new();
                for doctrine in [
                    DoctrineMode::Defensive,
                    DoctrineMode::Aggressive,
                    DoctrineMode::Scout,
                ] {
                    let vetoed = self.matrix.veto_reason(doctrine, &def.verb_id).is_some();
                    doctrine_row.insert(
                        doctrine.as_str().to_string(),
                        Value::String(if vetoed { "vetoed".to_string() } else { "allowed".to_string() }),
                    );
                }
                json!({
                    "verb_id": def.verb_id,
                    "display_name": def.display_name,
                    "family": def.family.as_str(),
                    "args": args,
                    "valid_target": def.valid_target,
                    "doctrine_compat": Value::Object(doctrine_row),
                })
            })
            .collect();

        let formations_json: Vec<Value> = FormationKind::ALL
            .iter()
            .map(|kind| {
                let def = cf_ai::formation::FormationDef::builtin(*kind);
                let slots: Vec<Value> = def
                    .slots
                    .iter()
                    .map(|s| {
                        json!({
                            "slot_id": s.slot_id,
                            "offset": s.offset,
                            "role_hint": s.role_hint.as_str(),
                            "sector_bearing_degrees": s.sector_bearing_degrees,
                        })
                    })
                    .collect();
                json!({
                    "kind": kind.as_str(),
                    "slot_count": def.slots.len(),
                    "slots": slots,
                })
            })
            .collect();

        let bt_kinds_json: Vec<Value> = ArchetypeBtKind::ALL
            .iter()
            .map(|k| {
                json!({
                    "kind": k.as_str(),
                    "node_count": cf_ai::archetype_bt::node_ids_for(*k).len(),
                    "node_ids": cf_ai::archetype_bt::node_ids_for(*k),
                })
            })
            .collect();

        let squad_view = squad.map(|s| {
            let role_assignments: Vec<Value> = s
                .role_assignments
                .iter()
                .map(|(actor_id, role)| {
                    json!({
                        "actor_id": *actor_id,
                        "role": role.as_str(),
                    })
                })
                .collect();
            json!({
                "squad_id": s.squad_id,
                "doctrine": s.doctrine.as_str(),
                "current_verb": s.current_command.as_ref().map(|c| c.verb_id.clone()),
                "current_command_family": s.current_command.as_ref().map(|c| c.family.as_str().to_string()),
                "last_veto_label": s.last_veto_label,
                "formation_kind": s.formation_kind.as_str(),
                "role_assignments": role_assignments,
                "slot_assignments": s.slot_assignments,
                "breach_chain_active": s.breach_chain.is_some(),
                "bounding_active": s.bounding.is_some(),
                "last_solve_tick": s.last_solve_tick,
            })
        });

        json!({
            "squad": squad_view,
            "verb_registry_count": self.registry.len(),
            "verb_registry": verbs_json,
            "formations": formations_json,
            "archetype_bt": bt_kinds_json,
            "doctrine_vetoes": serde_json::to_value(&self.matrix.vetoes).unwrap_or(Value::Null),
        })
    }
}

/// **M7B**: split the squad's role assignments into the "cover this half"
/// and "move this half" actor lists for the bounding-retreat sequence.
/// Heavy + Marksman + SquadLeader bias toward covering; Pointman +
/// Rifleman + Engineer bias toward moving. Phase swaps swap the lists.
pub fn bounding_split(
    role_assignments: &BTreeMap<u64, SquadRoleHint>,
    new_phase: cf_ai::squad_state::BoundingPhase,
) -> (Vec<u64>, Vec<u64>) {
    let mut covering: Vec<u64> = Vec::new();
    let mut moving: Vec<u64> = Vec::new();
    for (actor_id, role) in role_assignments {
        let prefers_cover = matches!(
            role,
            SquadRoleHint::Heavy | SquadRoleHint::Marksman | SquadRoleHint::SquadLeader
        );
        if prefers_cover {
            covering.push(*actor_id);
        } else {
            moving.push(*actor_id);
        }
    }
    // Phase swap: on MoveHalf, the originally-covering crew moves; on
    // CoverHalf, the originally-moving crew covers. Mirrors spec § "half
    // the squad covers while half moves 30u rearward; squad.bounding_step
    // fires per swap" — alternation is what keeps continuous cover.
    match new_phase {
        cf_ai::squad_state::BoundingPhase::CoverHalf => (covering, moving),
        cf_ai::squad_state::BoundingPhase::MoveHalf => (moving, covering),
    }
}

/// **M7B**: render a `squad.formation_slot_broken` JSON payload.
pub fn slot_broken_payload(squad_id: u64, report: &SlotBrokenReport, next_solve_tick: u64) -> Value {
    json!({
        "squad_id": squad_id,
        "member_actor_id": report.member_actor_id,
        "slot_id": report.slot_id,
        "reason": "out_of_range",
        "next_solve_tick": next_solve_tick,
        "distance": report.distance,
        "threshold": report.threshold,
    })
}

fn arg_to_json(v: &VerbArgValue) -> Value {
    match v {
        VerbArgValue::Waypoint(p) => json!({"kind": "waypoint", "value": p}),
        VerbArgValue::Actor(a) => json!({"kind": "actor", "value": *a}),
        VerbArgValue::Door(d) => json!({"kind": "door", "value": *d}),
        VerbArgValue::Side(s) => json!({"kind": "side", "value": s}),
        VerbArgValue::Sector { origin, direction } => json!({
            "kind": "sector",
            "value": {"origin": origin, "direction": direction},
        }),
        VerbArgValue::Window(w) => json!({"kind": "window", "value": *w}),
        VerbArgValue::Label(l) => json!({"kind": "label", "value": l}),
        VerbArgValue::Index(i) => json!({"kind": "index", "value": *i}),
    }
}

/// **M7B**: typed wrapper for `issue_verb` results.
pub struct IssueOutcome {
    pub outcome: CommandIssue,
    pub payload: Value,
}

impl IssueOutcome {
    pub fn is_accepted(&self) -> bool {
        self.outcome.is_accepted()
    }
}

/// **M7B**: typed wrapper for formation-set results.
pub struct FormationSetOutcome {
    pub previous: FormationKind,
    pub new_kind: FormationKind,
    pub formation_payload: Value,
    pub assignment_payloads: Vec<Value>,
}

/// **M7B**: typed wrapper for role-assignment results.
pub struct RoleAssignmentOutcome {
    pub result: cf_ai::squad_state::RoleAssignmentResult,
    pub payload: Value,
}

/// **M7B**: combined return for `advance_breach_chain`.
pub struct BreachChainAdvance {
    pub step_payload: Option<Value>,
    pub complete_payload: Option<Value>,
}

/// **M7B**: combined return for `on_member_kia`. When `removed` is true,
/// the engine emits the collapse + reslot payloads in order.
pub struct KiaOutcome {
    pub removed: bool,
    pub reslot: Option<FormationSetOutcome>,
}

/// **M7B**: parse a `VerbArgValue` from a JSON `{kind, value}` shape.
pub fn parse_verb_arg(value: &Value) -> Result<VerbArgValue, String> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing kind".to_string())?;
    let v = value.get("value").ok_or_else(|| "missing value".to_string())?;
    Ok(match kind {
        "waypoint" => {
            let arr = v.as_array().ok_or("waypoint not array")?;
            if arr.len() != 2 {
                return Err("waypoint length != 2".to_string());
            }
            VerbArgValue::Waypoint([
                arr[0].as_f64().unwrap_or(f64::NAN) as f32,
                arr[1].as_f64().unwrap_or(f64::NAN) as f32,
            ])
        }
        "actor" => VerbArgValue::Actor(v.as_u64().ok_or("actor not u64")?),
        "door" => VerbArgValue::Door(v.as_u64().ok_or("door not u64")?),
        "side" => VerbArgValue::Side(v.as_str().ok_or("side not str")?.to_string()),
        "sector" => {
            let origin_arr = v
                .get("origin")
                .and_then(Value::as_array)
                .ok_or("sector.origin missing")?;
            let dir_arr = v
                .get("direction")
                .and_then(Value::as_array)
                .ok_or("sector.direction missing")?;
            VerbArgValue::Sector {
                origin: [
                    origin_arr[0].as_f64().unwrap_or(f64::NAN) as f32,
                    origin_arr[1].as_f64().unwrap_or(f64::NAN) as f32,
                ],
                direction: [
                    dir_arr[0].as_f64().unwrap_or(f64::NAN) as f32,
                    dir_arr[1].as_f64().unwrap_or(f64::NAN) as f32,
                ],
            }
        }
        "window" => VerbArgValue::Window(v.as_u64().ok_or("window not u64")?),
        "label" => VerbArgValue::Label(v.as_str().ok_or("label not str")?.to_string()),
        "index" => VerbArgValue::Index(v.as_u64().ok_or("index not u64")? as u32),
        other => return Err(format!("unknown arg kind {other}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cf_ai::squad_state::{BoundingPhase, BreachChainStep};

    #[test]
    fn issue_press_attack_vetoed_under_defensive() {
        let mut world = M7BSquadWorld::new();
        world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
        let out = world.issue_verb(PLAYER_SQUAD_ID, "press_attack", vec![], 1, 100);
        assert!(matches!(out.outcome, CommandIssue::Vetoed { .. }));
        assert_eq!(
            out.payload.get("reason_label").and_then(Value::as_str),
            Some("doctrine_defensive_blocks_press_attack")
        );
    }

    #[test]
    fn issue_press_attack_accepted_after_doctrine_switch() {
        let mut world = M7BSquadWorld::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
        squad.doctrine = DoctrineMode::Aggressive;
        let out = world.issue_verb(PLAYER_SQUAD_ID, "press_attack", vec![], 1, 100);
        assert!(out.is_accepted());
    }

    #[test]
    fn set_formation_emits_slot_assignments() {
        let mut world = M7BSquadWorld::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
        squad.assign_role(1, SquadRoleHint::SquadLeader);
        squad.assign_role(2, SquadRoleHint::Rifleman);
        squad.assign_role(3, SquadRoleHint::Rifleman);
        squad.assign_role(4, SquadRoleHint::Heavy);
        squad.assign_role(5, SquadRoleHint::Heavy);
        let out = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
        assert_eq!(out.assignment_payloads.len(), 5);
    }

    #[test]
    fn dump_state_view_lists_verb_registry_and_archetype_bts() {
        let world = M7BSquadWorld::new();
        let view = world.dump_state_view(PLAYER_SQUAD_ID);
        assert!(view.get("verb_registry_count").and_then(Value::as_u64).unwrap_or(0) >= 50);
        let bts = view.get("archetype_bt").and_then(Value::as_array).unwrap();
        assert_eq!(bts.len(), 6);
        for bt in bts {
            assert!(bt.get("node_count").and_then(Value::as_u64).unwrap_or(0) >= 30);
        }
    }

    #[test]
    fn breach_chain_advances_four_steps_then_completes() {
        let mut world = M7BSquadWorld::new();
        let _ = world.start_breach_chain(PLAYER_SQUAD_ID, 42, "left", vec![1, 2, 3, 4], 100);
        let steps: Vec<BreachChainStep> = vec![
            BreachChainStep::Breach,
            BreachChainStep::Frag,
            BreachChainStep::Advance,
        ];
        for expected in steps {
            let res = world.advance_breach_chain(PLAYER_SQUAD_ID, 101);
            let step = res.step_payload.expect("step payload").get("step").unwrap().as_str().unwrap().to_string();
            assert_eq!(step, expected.as_str());
            assert!(res.complete_payload.is_none());
        }
        let final_res = world.advance_breach_chain(PLAYER_SQUAD_ID, 105);
        assert!(final_res.step_payload.is_none());
        assert!(final_res.complete_payload.is_some());
    }

    #[test]
    fn breach_chain_includes_pre_assigned_sectors_of_fire() {
        let mut world = M7BSquadWorld::new();
        let start = world.start_breach_chain(PLAYER_SQUAD_ID, 42, "left", vec![1, 2, 3, 4], 100);
        // Start payload includes sectors per slot.
        let sectors = start
            .get("sectors_of_fire")
            .and_then(Value::as_array)
            .expect("sectors_of_fire");
        assert_eq!(sectors.len(), 4);
        // Stack-1 (slot 0) is forward (0 deg).
        let s0 = sectors[0].as_object().expect("entry");
        assert_eq!(s0.get("actor_id").and_then(Value::as_u64), Some(1));
        assert_eq!(s0.get("sector_bearing_degrees").and_then(Value::as_f64), Some(0.0));
        // Stack-2 (slot 1) sectors left (+90 deg).
        let s1 = sectors[1].as_object().expect("entry");
        assert_eq!(s1.get("actor_id").and_then(Value::as_u64), Some(2));
        assert_eq!(s1.get("sector_bearing_degrees").and_then(Value::as_f64), Some(90.0));
        // Stack-3 (slot 2) sectors right (-90 deg).
        let s2 = sectors[2].as_object().expect("entry");
        assert_eq!(s2.get("sector_bearing_degrees").and_then(Value::as_f64), Some(-90.0));
        // Stack-4 (slot 3) sectors rear (180 deg).
        let s3 = sectors[3].as_object().expect("entry");
        assert_eq!(s3.get("sector_bearing_degrees").and_then(Value::as_f64), Some(180.0));

        // Per-step events also carry the actor_ids + sectors.
        let step1 = world.advance_breach_chain_with_actors(PLAYER_SQUAD_ID, 101, &[1, 2, 3, 4]);
        let p = step1.step_payload.expect("step1");
        let actors = p.get("actor_ids").and_then(Value::as_array).expect("actor_ids");
        assert_eq!(actors.len(), 4);
        let step_sectors = p
            .get("sectors_of_fire")
            .and_then(Value::as_array)
            .expect("sectors_of_fire");
        assert_eq!(step_sectors.len(), 4);
    }

    #[test]
    fn bounding_alternates_phase() {
        let mut world = M7BSquadWorld::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
        squad.start_bounding([20.0, 0.0], 100);
        let p1 = world.tick_bounding(PLAYER_SQUAD_ID, Some([20.0, 0.0])).unwrap();
        let p2 = world.tick_bounding(PLAYER_SQUAD_ID, Some([20.0, 0.0])).unwrap();
        assert_eq!(p1.get("new_phase").and_then(Value::as_str), Some(BoundingPhase::MoveHalf.as_str()));
        assert_eq!(p2.get("new_phase").and_then(Value::as_str), Some(BoundingPhase::CoverHalf.as_str()));
    }

    #[test]
    fn bounding_step_splits_cover_and_moving_actors_by_role() {
        let mut world = M7BSquadWorld::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
        squad.assign_role(1, SquadRoleHint::SquadLeader);
        squad.assign_role(2, SquadRoleHint::Rifleman);
        squad.assign_role(3, SquadRoleHint::Heavy);
        squad.assign_role(4, SquadRoleHint::Pointman);
        squad.start_bounding([30.0, 0.0], 100);
        let p = world.tick_bounding(PLAYER_SQUAD_ID, Some([30.0, 0.0])).unwrap();
        let cover_actors = p.get("cover_actors").and_then(Value::as_array).unwrap();
        let moving_actors = p.get("moving_actors").and_then(Value::as_array).unwrap();
        assert_eq!(cover_actors.len() + moving_actors.len(), 4);
    }

    #[test]
    fn slot_broken_payload_emitted_when_actor_wanders() {
        let mut world = M7BSquadWorld::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
        squad.assign_role(1, SquadRoleHint::SquadLeader);
        squad.assign_role(2, SquadRoleHint::Rifleman);
        let _ = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
        let mut positions = std::collections::BTreeMap::new();
        positions.insert(1, [999.0, 999.0]);
        positions.insert(2, [0.0, 0.0]);
        let payloads = world.detect_and_report_broken_slots(PLAYER_SQUAD_ID, &positions, 200);
        assert!(!payloads.is_empty(), "wandering actor must produce a slot-broken event");
        let p0 = &payloads[0];
        assert_eq!(p0.get("squad_id").and_then(Value::as_u64), Some(PLAYER_SQUAD_ID));
        assert_eq!(p0.get("reason").and_then(Value::as_str), Some("out_of_range"));
    }

    #[test]
    fn tick_periodic_reslot_fires_at_2s_cadence() {
        let mut world = M7BSquadWorld::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Aggressive);
        squad.assign_role(1, SquadRoleHint::SquadLeader);
        squad.assign_role(2, SquadRoleHint::Rifleman);
        let _ = squad.try_issue_builtin("advance", vec![], 99, 100);
        let _ = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
        // Below cadence — no reslot.
        let early = world.tick_periodic_reslot(PLAYER_SQUAD_ID, [1.0, 0.0], 0.0, Some(1), 130, 60);
        assert!(early.is_none(), "below 2s should NOT trigger reslot");
        // Past cadence — reslot fires.
        let late = world.tick_periodic_reslot(PLAYER_SQUAD_ID, [1.0, 0.0], 0.0, Some(1), 100 + 120, 60);
        assert!(late.is_some(), "past 2s WITH command active should trigger reslot");
    }

    #[test]
    fn on_member_kia_collapses_and_reslots() {
        let mut world = M7BSquadWorld::new();
        let squad = world.ensure_squad(PLAYER_SQUAD_ID, DoctrineMode::Defensive);
        squad.assign_role(1, SquadRoleHint::SquadLeader);
        squad.assign_role(2, SquadRoleHint::Rifleman);
        squad.assign_role(3, SquadRoleHint::Rifleman);
        squad.assign_role(4, SquadRoleHint::Heavy);
        squad.assign_role(5, SquadRoleHint::Heavy);
        let _ = world.set_formation(PLAYER_SQUAD_ID, FormationKind::Wedge, [0.0, 0.0], 0.0, Some(1), 100);
        let pre = world.squad(PLAYER_SQUAD_ID).unwrap().role_assignments.len();
        let outcome = world.on_member_kia(PLAYER_SQUAD_ID, 5, [0.0, 0.0], 0.0, Some(1), 220);
        assert!(outcome.removed);
        let reslot = outcome.reslot.expect("reslot fires after KIA");
        assert_eq!(reslot.assignment_payloads.len(), pre - 1);
    }

    #[test]
    fn dump_state_view_includes_per_verb_doctrine_compat_row() {
        let world = M7BSquadWorld::new();
        let view = world.dump_state_view(PLAYER_SQUAD_ID);
        let verbs = view.get("verb_registry").and_then(Value::as_array).expect("verbs");
        // Find press_attack; defensive should say "vetoed", aggressive "allowed".
        let press = verbs
            .iter()
            .find(|v| v.get("verb_id").and_then(Value::as_str) == Some("press_attack"))
            .expect("press_attack present");
        let row = press
            .get("doctrine_compat")
            .and_then(Value::as_object)
            .expect("doctrine_compat row");
        assert_eq!(row.get("defensive").and_then(Value::as_str), Some("vetoed"));
        assert_eq!(row.get("aggressive").and_then(Value::as_str), Some("allowed"));
    }

    #[test]
    fn brain_hop_payload_records_same_squad_flag() {
        let mut world = M7BSquadWorld::new();
        let p = world.brain_hop_payload(PLAYER_SQUAD_ID, 7, 8, 100, true);
        assert_eq!(p.get("from_actor_id").and_then(Value::as_u64), Some(7));
        assert_eq!(p.get("to_actor_id").and_then(Value::as_u64), Some(8));
        assert_eq!(p.get("same_squad").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn parse_verb_arg_waypoint() {
        let v = json!({"kind": "waypoint", "value": [1.0, 2.0]});
        let out = parse_verb_arg(&v).unwrap();
        assert!(matches!(out, VerbArgValue::Waypoint([1.0, 2.0])));
    }

    #[test]
    fn parse_verb_arg_actor() {
        let v = json!({"kind": "actor", "value": 42});
        let out = parse_verb_arg(&v).unwrap();
        assert!(matches!(out, VerbArgValue::Actor(42)));
    }
}
