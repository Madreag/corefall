# AGENTS.md — Corefall

The ONLY rules file. ~30 lines on purpose.

## What you read during implementation

ONLY these:

- `specs/active/<id>.md` — the milestone you're implementing.
- Source files under `game/crates/cf-*/`.

Do NOT read `docs/plan/`, `docs/MISSING_FEATURES.md`, `docs/FUTURE_FEATURES.md`, `docs/implementation-log/`, `docs/reviews/`, `CHANGELOG.md`, or any other doc unless the user explicitly says "consult `<file>`".

## Workspace

- Rust workspace at `game/Cargo.toml`. Crates under `game/crates/cf-*/`.
- Run `cargo` commands from `game/`.
- Crate name prefix: `cf-`.

## Code conventions

- No `println!` in production code. Use `tracing`.
- No `unwrap()` on user-controllable inputs.
- No `thread_rng()` in sim crates (`cf-sim-core`, `cf-physics`, `cf-material`, `cf-ai`, `cf-terrain`, `cf-atmos`).
- No `unsafe` blocks.
- Tick rate is configurable. Never hardcode `60` as a sim architectural assumption.
- **Comments stay short.** Default to none. One line max for non-obvious WHY. NO multi-line narratives. NO restating what the code does. NO milestone/spec references in code bodies. Doc-comments document API contracts only. If a comment needs more than one line, the code probably needs a better name.
- **Files stay under 2000 LOC.** If a file grows past 1000 LOC look for natural module boundaries. Past 2000 LOC: split unless there's a perf or safety reason to keep it monolithic. Document the reason in the file header if you must.
- **All tuning constants live in `content/` JSON/RON.** Code reads them via loaders with hardcoded fallback only for boot. Add a JSON file before adding a `const X: f32 = ...` in source.

## Workflow

1. Read `specs/active/<id>.md`.
2. **Audit first.** For each Gherkin scenario in Acceptance Criteria, check whether current code already satisfies it. Read `game/crates/cf-*/src/` to verify. Don't blind-implement what already exists.
3. **Fill the gaps.** Implement only the scenarios that fail the audit.
4. **Re-verify.** After each gap is filled, run the relevant scenario mentally (or via a focused test if quick) and confirm it passes.
5. Commit each meaningful gap-fill with subject `<id>: <imperative summary>`. Multiple commits per spec is fine.
6. **Report a per-scenario verdict table** at the end of the session in this format:

   ```
   | Scenario | Verdict | Notes |
   |---|---|---|
   | Solo play for 5 minutes without crash | PASS (already in) | cf-app already runs cleanly; no panic at 18000 ticks |
   | Stability decreases on recoil | PASS (already in) | cf-actor::sim::tick_stability since W1.3 commit ae639f0 |
   | Knockdown stuns the actor | IMPLEMENTED | Added Stance::KnockedDown gating in cf-actor; commit <sha> |
   | NaN/Inf input rejected | STILL FAILING | act.player.move guard exists; act.player.aim missing same guard |
   ```

   Verdicts: `PASS (already in)` / `IMPLEMENTED` / `STILL FAILING` / `BLOCKED`.

7. When every scenario verdict is `PASS (already in)` or `IMPLEMENTED`, move the spec from `specs/active/` to `specs/done/`.

## Don't

- Don't create new `AGENTS.md` files in subdirectories.
- Don't open `docs/plan/` files during implementation.
- Don't ask the user for content already in the active spec — implement what the spec says.
- Don't mark a milestone done until every Acceptance Criterion in the spec is satisfied with real code.
