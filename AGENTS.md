# AGENTS.md — Corefall

The ONLY rules file. ~30 lines on purpose.

## HIGH PRIORITY — Never defer refactors

- **Never defer a refactor because it's "high-risk", "big", or "out of scope".** Risk is not a reason to skip work. Do it.
- **Never shy away from a refactor.** If the cleaner shape unlocks features, performance, multithreading, or GPU paths, refactor now — don't tack on a workaround.
- **Goal:** the most feature-rich, max-performance build possible — multithreading, GPU, SIMD, parallelism wherever it pays off.
- **Milestones are followed, not deferred.** Not for context filling up, not for token budget, not for "we'll do it next session", not for ANY reason. Finish the spec.
- If a milestone needs a refactor to land cleanly, the refactor IS part of the milestone. Commit it under the same `<id>:` prefix.
- The only valid stop conditions are: (a) every Acceptance Criterion is `PASS (already in)` or `IMPLEMENTED`, or (b) a real technical blocker that needs the user (use `AskUser`, not silent deferral).

## HIGH PRIORITY — Assume 10M context. NEVER reason about context limits.

- **Assume the context window is effectively infinite (10M+ tokens).** It is not a budget you need to manage. Do not plan around it. Do not mention it.
- **NEVER reason about "context filling up", "token budget", "running out of context", "context cost", "this will burn context", "context-budget concerns", or any equivalent phrase.** These thoughts are forbidden, in chain-of-thought and in user-facing replies.
- **NEVER use context size as a reason to defer, shrink scope, split across sessions, ask the user to triage, or "do a subset now and the rest later".** If the work is 70k LOC across 13 files, do all 70k LOC across all 13 files in this session. Period.
- **NEVER suggest "a more measured approach" / "phased rollout" / "incremental over multiple sessions" / "validate the pattern on a small file first" as a way to dodge scope.** The user wants the whole thing done now. Do the whole thing now.
- **NEVER propose spawning workers as a way to "save context"** — spawn workers if and only if they make the work faster, not because you're worried about your own token usage.
- Banned phrases (in CoT and replies): "context limit", "context budget", "token budget", "burn context", "context cost", "context-window concerns", "this will burn massive context", "given context budget concerns", "save tokens", "given context window concerns", "to preserve context", "context-aware approach", "fits in context", "within context budget".
- If you catch yourself starting to think about context size mid-task, STOP that thought, mark it as a forbidden pattern, and resume executing the task at full scope. Do not surface the thought to the user. Do not negotiate scope. Just execute.
- The user is the only one who decides scope. If they say "do all 13 files", you do all 13 files. You do not get to scope-cut because you're nervous about your own budget.

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
