# specs/active/

Milestone(s) currently being implemented. ONE file per milestone. The implementer reads ONLY files in this folder + source code.

## Naming

`<milestone-id>.md` — e.g., `M5.5.md`, `M5.6.md`, `BP4.md`.

## Lifecycle

1. **Planner session:** writes a new spec here from `specs/backlog/`. Stops without implementing. (See `specs/_planner.md`.)
2. **Human review:** project owner reads the spec (~5 min) and approves or corrects.
3. **Implementer session:** reads ONLY `specs/active/<id>.md` + source files. Implements until every Acceptance Criterion is satisfied. Commits.
4. **On completion:** spec is moved to `specs/done/<id>.md`. New spec written for the next milestone.

## Rule

If `specs/active/` has more than 2-3 files, you've over-committed. Finish one before starting the next.
