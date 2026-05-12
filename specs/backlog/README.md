# specs/backlog/

Planning inputs the **planner** reads when writing a new spec. The **implementer** never opens these.

## Contents

| File | What it is | When to consult |
|---|---|---|
| `old-roadmap.md` | Copy of the legacy `docs/plan/spec/prototype-roadmap.md`. The original BP0..BP12 planning. | Planner pass: extract milestone scope, dependencies, original done-criteria. |
| `MISSING_FEATURES.md` | ~4,300-item inventory of BP0..BP3 closure gaps tagged by milestone / DR / wave. | Planner pass: pull items tagged `[<ID>]` into the new spec's scope or Out-of-Scope. |
| `FUTURE_FEATURES.md` | ~440-item inventory of BP4..BP12 forward-looking gaps. | Planner pass: ditto for forward milestones. |

## Why these are gitignored from the implementer

The original failure mode was: agents loaded all four (roadmap + MISSING + FUTURE + per-crate AGENTS.md) at once, blew through the recall window, and silently dropped 90% of the items. Moving these to `backlog/` and forbidding the implementer from reading them is the structural fix.

Planner reads them once → distills into a focused `~1500-word` spec → implementer reads only that.
