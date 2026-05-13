# Planner Prompt

Use this when writing a new spec from the backlog / old roadmap. Run as a separate session from implementation.

---

## Prompt to paste

You are the **planner** for milestone `<ID>` (e.g., `M14`).

## Your job

Write ONE file at `specs/active/<ID>.md` following `specs/_template.md`. Then STOP. Do not implement anything.

## What to read

In this order:

1. `specs/_template.md` — the format you must follow.
2. `specs/done/` — read every milestone in here that is in your Dependencies list, so you know what already exists.
3. `specs/backlog/old-roadmap.md` (or whatever the legacy roadmap is) — find the section for `<ID>`. Extract scope, dependencies, done-criteria.
4. `specs/backlog/MISSING_FEATURES.md` and `specs/backlog/FUTURE_FEATURES.md` — search for items tagged `[<ID>]` and pull anything relevant into the spec.
5. `game/crates/cf-*/AGENTS.md` if any per-crate AGENTS.md files still exist — read the ones for crates listed in your `Crates / modules touched` table.
6. The actual source of those crates — skim `lib.rs` to know what already exists vs what you need to add.

## Rules

- **Word budget: ~800-1500 words.** Tight enough to fit in the high-recall context window, fat enough to fully describe a multi-crate milestone.
- **Be specific about files and types.** The implementer will not re-read the roadmap. If a file is in your `Files` list, the implementer will create/modify exactly that file. If a type is named in `Crates / modules touched`, the implementer will use that exact name.
- **Out-of-scope is critical.** List anything in the legacy roadmap that's tagged for `<ID>` but you're explicitly leaving for a later milestone. The implementer needs to know what NOT to do.
- **Acceptance Criteria use Gherkin.** Given/When/Then. One scenario per observable behavior. The implementer is done when every scenario passes.
- **No process gates.** Don't add "T-CAPTURE", "T-RELEASE", "BP closure gate", "AI Self-Test Report", or any of the legacy ceremony into the spec. If a behavior matters, it goes in Acceptance Criteria. If it doesn't, it doesn't go in the spec.
- **No tests in the spec.** Tests are decided by the implementer (or skipped per current project posture). Acceptance Criteria describe runtime behavior, not test surfaces.

## When you're done

1. Save the spec at `specs/active/<ID>.md`.
2. Print a one-line summary: "Spec written for `<ID>` — N words, M acceptance scenarios, K files touched."
3. STOP. Do not implement. Do not commit. Wait for human review.
