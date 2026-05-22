//! M7B: parser surface for the squad-command grammar.
//!
//! Inputs arrive from two paths:
//! - **cfctl**: structured `(verb_id, args[])` from the JSON-RPC surface.
//! - **wheel**: enumerated `VerbDef` row + bound args from the UI.
//!
//! Both call into [`parse_verb_invocation`], which validates the verb_id is
//! registered, checks argument arity + kind, and produces a [`ParsedVerb`]
//! that the engine then passes to `try_issue` for the doctrine check.

use serde::{Deserialize, Serialize};

use super::{
    verb_registry::{VerbArgKind, VerbRegistry},
    VerbArgValue,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedVerb {
    pub verb_id: String,
    pub args: Vec<VerbArgValue>,
}

pub fn parse_verb_invocation(
    registry: &VerbRegistry,
    verb_id: &str,
    args: Vec<VerbArgValue>,
) -> Result<ParsedVerb, ParseError> {
    let def = registry.find(verb_id).ok_or_else(|| ParseError::UnknownVerb {
        verb_id: verb_id.to_string(),
    })?;
    let required = def.required_args();
    if args.len() < required {
        return Err(ParseError::MissingArgs {
            verb_id: verb_id.to_string(),
            got: args.len(),
            required,
        });
    }
    if args.len() > def.args.len() {
        return Err(ParseError::TooManyArgs {
            verb_id: verb_id.to_string(),
            got: args.len(),
            max: def.args.len(),
        });
    }
    for (i, arg) in args.iter().enumerate() {
        let expected = def.args[i].kind;
        if arg.kind() != expected {
            return Err(ParseError::ArgKindMismatch {
                verb_id: verb_id.to_string(),
                index: i,
                expected,
                got: arg.kind(),
            });
        }
        if !arg.is_well_formed() {
            return Err(ParseError::ArgNotFinite {
                verb_id: verb_id.to_string(),
                index: i,
            });
        }
    }
    Ok(ParsedVerb {
        verb_id: verb_id.to_string(),
        args,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnknownVerb {
        verb_id: String,
    },
    MissingArgs {
        verb_id: String,
        got: usize,
        required: usize,
    },
    TooManyArgs {
        verb_id: String,
        got: usize,
        max: usize,
    },
    ArgKindMismatch {
        verb_id: String,
        index: usize,
        expected: VerbArgKind,
        got: VerbArgKind,
    },
    ArgNotFinite {
        verb_id: String,
        index: usize,
    },
}

impl ParseError {
    pub fn as_label(&self) -> String {
        match self {
            ParseError::UnknownVerb { verb_id } => format!("unknown_verb_{verb_id}"),
            ParseError::MissingArgs { verb_id, .. } => format!("missing_args_{verb_id}"),
            ParseError::TooManyArgs { verb_id, .. } => format!("too_many_args_{verb_id}"),
            ParseError::ArgKindMismatch { verb_id, index, .. } => format!("arg_kind_mismatch_{verb_id}_{index}"),
            ParseError::ArgNotFinite { verb_id, index } => format!("arg_not_finite_{verb_id}_{index}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::verb_registry::builtin_registry;
    use super::*;

    #[test]
    fn unknown_verb_rejected() {
        let reg = builtin_registry();
        let err = parse_verb_invocation(&reg, "no_such_verb", vec![]).unwrap_err();
        assert!(matches!(err, ParseError::UnknownVerb { .. }));
    }

    #[test]
    fn move_to_requires_waypoint() {
        let reg = builtin_registry();
        let err = parse_verb_invocation(&reg, "move_to", vec![]).unwrap_err();
        assert!(matches!(err, ParseError::MissingArgs { .. }));
    }

    #[test]
    fn move_to_accepts_waypoint() {
        let reg = builtin_registry();
        let out = parse_verb_invocation(
            &reg,
            "move_to",
            vec![VerbArgValue::Waypoint([1.0, 2.0])],
        )
        .expect("accepted");
        assert_eq!(out.verb_id, "move_to");
    }

    #[test]
    fn move_to_rejects_actor_arg() {
        let reg = builtin_registry();
        let err = parse_verb_invocation(&reg, "move_to", vec![VerbArgValue::Actor(7)]).unwrap_err();
        assert!(matches!(err, ParseError::ArgKindMismatch { .. }));
    }

    #[test]
    fn stack_door_requires_door_and_side() {
        let reg = builtin_registry();
        let err = parse_verb_invocation(
            &reg,
            "stack_door",
            vec![VerbArgValue::Door(11)],
        )
        .unwrap_err();
        assert!(matches!(err, ParseError::MissingArgs { .. }));
    }

    #[test]
    fn stack_door_accepts_door_and_side() {
        let reg = builtin_registry();
        let out = parse_verb_invocation(
            &reg,
            "stack_door",
            vec![VerbArgValue::Door(11), VerbArgValue::Side("left".to_string())],
        )
        .expect("accepted");
        assert_eq!(out.args.len(), 2);
    }
}
