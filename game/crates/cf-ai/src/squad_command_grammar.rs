//! M7B: Squad-command grammar — verb registry + parser + doctrine compat.
//!
//! The grammar is the player's interface to the squad. A `SquadVerb` names a
//! command (`Advance`, `Stack (door, left)`, `Frag-Out`, ...); the registry
//! enumerates 50+ verbs each with a stable `verb_id`, an argument schema, a
//! valid-target predicate, and a doctrine-compatibility row. The parser is
//! data-driven so the M25 wheel + Tab overlay + M23B commander-doctrine all
//! enumerate from the same source.
//!
//! Spec § "50+ named squad verbs in a data-driven registry; wheel + Tab
//! overlay enumerate from the same registry the parser accepts."

use serde::{Deserialize, Serialize};

pub mod doctrine_compat;
pub mod parser;
pub mod verb_registry;

pub use doctrine_compat::{DoctrineCompatMatrix, VetoReason};
pub use parser::{parse_verb_invocation, ParsedVerb};
pub use verb_registry::{
    builtin_registry, verb_family_label, VerbArgKind, VerbArgSpec, VerbDef, VerbFamily, VerbRegistry,
};

use crate::autonomy::DoctrineMode;

/// validation + doctrine compat). The squad state then drives the BT
/// expansion + per-actor goal assignment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SquadCommand {
    pub verb_id: String,
    pub family: VerbFamily,
    pub args: Vec<VerbArgValue>,
    /// Originating issuer (player actor id at human-issued time; AI commander
    /// actor id when the M23B planner issues).
    pub issuer_actor_id: u64,
    /// Tick the command was committed to squad state.
    pub issued_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind", content = "value")]
pub enum VerbArgValue {
    /// World position [x, y].
    Waypoint([f32; 2]),
    /// Target actor id.
    Actor(u64),
    /// Door entity id.
    Door(u64),
    /// Stack side: `left` | `right` | `top` | `bottom`.
    Side(String),
    /// Sector vector (origin, direction) for Overwatch.
    Sector { origin: [f32; 2], direction: [f32; 2] },
    /// Window entity id (for Suppress (window) / Stack (window)).
    Window(u64),
    /// Pure string (e.g. role identifier for Engineer-Up).
    Label(String),
    /// Free-floating index (slot id, magazine count, etc.).
    Index(u32),
}

impl VerbArgValue {
    pub fn kind(&self) -> VerbArgKind {
        match self {
            VerbArgValue::Waypoint(_) => VerbArgKind::Waypoint,
            VerbArgValue::Actor(_) => VerbArgKind::Actor,
            VerbArgValue::Door(_) => VerbArgKind::Door,
            VerbArgValue::Side(_) => VerbArgKind::Side,
            VerbArgValue::Sector { .. } => VerbArgKind::Sector,
            VerbArgValue::Window(_) => VerbArgKind::Window,
            VerbArgValue::Label(_) => VerbArgKind::Label,
            VerbArgValue::Index(_) => VerbArgKind::Index,
        }
    }

    /// True if the value is finite and structurally valid for its kind.
    pub fn is_well_formed(&self) -> bool {
        match self {
            VerbArgValue::Waypoint(p) => p[0].is_finite() && p[1].is_finite(),
            VerbArgValue::Sector { origin, direction } => {
                origin[0].is_finite()
                    && origin[1].is_finite()
                    && direction[0].is_finite()
                    && direction[1].is_finite()
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommandIssue {
    /// The verb passed argument validation + doctrine compat; commit it.
    Accepted(SquadCommand),
    /// The verb is structurally invalid (missing/wrong args).
    Rejected { reason_label: String },
    /// Doctrine forbids this verb in the current doctrine mode.
    Vetoed {
        reason_label: String,
        attempted_verb_id: String,
        doctrine: DoctrineMode,
    },
}

impl CommandIssue {
    pub fn is_accepted(&self) -> bool {
        matches!(self, CommandIssue::Accepted(_))
    }

    pub fn reason_label(&self) -> Option<&str> {
        match self {
            CommandIssue::Rejected { reason_label } => Some(reason_label.as_str()),
            CommandIssue::Vetoed { reason_label, .. } => Some(reason_label.as_str()),
            CommandIssue::Accepted(_) => None,
        }
    }
}

/// the typed outcome. Pure function — no side effects on squad state.
pub fn try_issue(
    registry: &VerbRegistry,
    matrix: &DoctrineCompatMatrix,
    doctrine: DoctrineMode,
    verb_id: &str,
    args: Vec<VerbArgValue>,
    issuer_actor_id: u64,
    issued_tick: u64,
) -> CommandIssue {
    let Some(def) = registry.find(verb_id) else {
        return CommandIssue::Rejected {
            reason_label: format!("unknown_verb_{verb_id}"),
        };
    };

    // Argument-shape validation.
    if args.len() < def.required_args() {
        return CommandIssue::Rejected {
            reason_label: format!("missing_args_{verb_id}"),
        };
    }
    if args.len() > def.args.len() {
        return CommandIssue::Rejected {
            reason_label: format!("too_many_args_{verb_id}"),
        };
    }
    for (i, arg) in args.iter().enumerate() {
        let expected = def.args[i].kind;
        if arg.kind() != expected {
            return CommandIssue::Rejected {
                reason_label: format!("arg_kind_mismatch_{verb_id}_{i}"),
            };
        }
        if !arg.is_well_formed() {
            return CommandIssue::Rejected {
                reason_label: format!("arg_not_finite_{verb_id}_{i}"),
            };
        }
    }

    // Doctrine veto.
    if let Some(veto) = matrix.veto_reason(doctrine, &def.verb_id) {
        return CommandIssue::Vetoed {
            reason_label: veto.into_label(doctrine, verb_id),
            attempted_verb_id: verb_id.to_string(),
            doctrine,
        };
    }

    CommandIssue::Accepted(SquadCommand {
        verb_id: verb_id.to_string(),
        family: def.family,
        args,
        issuer_actor_id,
        issued_tick,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_at_least_50_verbs() {
        let reg = builtin_registry();
        assert!(reg.len() >= 50, "verb registry has only {} entries", reg.len());
    }

    #[test]
    fn registry_verb_ids_are_unique() {
        let reg = builtin_registry();
        let mut ids: Vec<&str> = reg.iter().map(|d| d.verb_id.as_str()).collect();
        ids.sort_unstable();
        for window in ids.windows(2) {
            assert_ne!(window[0], window[1], "duplicate verb_id {:?}", window[0]);
        }
    }

    #[test]
    fn try_issue_rejects_missing_args() {
        let reg = builtin_registry();
        let matrix = DoctrineCompatMatrix::builtin();
        let out = try_issue(&reg, &matrix, DoctrineMode::Defensive, "move_to", vec![], 1, 100);
        assert!(matches!(out, CommandIssue::Rejected { .. }));
    }

    #[test]
    fn try_issue_accepts_well_formed_advance() {
        let reg = builtin_registry();
        let matrix = DoctrineCompatMatrix::builtin();
        let out = try_issue(
            &reg,
            &matrix,
            DoctrineMode::Aggressive,
            "advance",
            vec![],
            1,
            100,
        );
        assert!(out.is_accepted(), "advance should be accepted under aggressive doctrine: {out:?}");
    }

    #[test]
    fn try_issue_vetoes_press_attack_under_defensive() {
        let reg = builtin_registry();
        let matrix = DoctrineCompatMatrix::builtin();
        let out = try_issue(
            &reg,
            &matrix,
            DoctrineMode::Defensive,
            "press_attack",
            vec![],
            1,
            100,
        );
        match out {
            CommandIssue::Vetoed { reason_label, .. } => {
                assert!(
                    reason_label.contains("doctrine_defensive_blocks_press_attack"),
                    "unexpected reason: {reason_label}"
                );
            }
            other => panic!("expected veto, got {other:?}"),
        }
    }
}
