# specs/done/

Archive of completed milestones. Read-only reference for the planner when a new spec depends on a previously-shipped milestone.

The implementer does not read this folder during normal implementation. The planner may consult it when writing a new spec to know what already exists.

## Naming

Same as `specs/active/`: `<milestone-id>.md`.

## Move-in trigger

A spec lands here when:

1. Every Acceptance Criterion in the spec is satisfied with real code in `game/crates/`.
2. The implementer has committed and pushed.
3. The user has confirmed the milestone is done.
