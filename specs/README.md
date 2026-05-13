# specs/

Spec-driven workspace. One file per milestone. Self-contained.

## Layout

| Folder | Purpose | Loaded by agent? |
|---|---|---|
| `active/` | The milestone(s) currently being implemented. ONE file per milestone. | YES (implementer) |
| `done/` | Completed milestones, archived. | NO (reference only) |
| `backlog/` | Future milestones + planning notes + the old roadmap. | NO (planner only, on demand) |
| `_template.md` | Format every active spec must follow. | (template) |
| `_planner.md` | Prompt to give the agent when writing a new spec from backlog/old roadmap. | (template) |

## Day-to-day prompts

Replace all multi-paragraph prompts with one of these one-liners:

| You want... | Tell the agent |
|---|---|
| Write a new spec | `Write the spec for M14 at specs/active/M14.md using specs/_template.md and specs/_planner.md. Pull source from specs/backlog/. Stop when the spec is written — do NOT implement.` |
| Implement a spec | `Implement specs/active/M14.md.` |
| Audit completed work | `Audit specs/done/M14.md against current code in game/crates/. Report drift.` |
| Mark complete + move on | `Move specs/active/M14.md to specs/done/. Write spec for M15.` |

## Hand-off rule

The planner pass (writes the spec) and the implementer pass (writes the code) are separate sessions. The implementer never reads the old roadmap, the decisions, or the vision — only `specs/active/<id>.md` + source files. This keeps the implementer's working set small enough that nothing gets silently dropped.
