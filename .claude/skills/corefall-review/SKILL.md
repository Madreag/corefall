---
name: corefall-review
description: Deep Corefall code review and bug hunt. Use when asked to review a Corefall milestone, Build Point (BP0..BP12), feature, diff, branch, PR, or implementation for bugs, misses, logic gaps, test gaps, roadmap/checklist drift, determinism/replay risk, cfctl coverage, T-CAPTURE evidence, performance, security, or Rust safety.
when_to_use: Trigger for prompts like review M0, review BP1, bug hunt this milestone, find gaps, audit the implementation, review my diff, check if this is done, or run a pre-merge review. Always use for Corefall review work before declaring a milestone or Build Point complete.
argument-hint: "[milestone-or-bp-or-git-range]"
---

# Corefall Review

Run a deep review of Corefall implementation work. Treat `$ARGUMENTS` as the milestone, Build Point (e.g. `BP1`, `BP2`), feature, branch, commit, or diff range to review. If no argument is provided, review the current working tree.

For Build Point arguments (`BP<N>`), the review covers EVERY milestone the BP bundles per the Build Points table in `cortext_command_vault/spec/prototype-roadmap.md` plus the per-BP human-playtest survey row and T-CAPTURE summary-grid evidence.

Use ultrathink. Optimize for true bugs, missed requirements, logic problems, determinism gaps, weak tests, stale docs, and milestone incompleteness. Do not spend review budget on style unless it hides a real defect or maintainability risk.

## Review Posture

Reviews must find true bugs, missed contracts, false evidence, logic gaps, weak tests, stale docs, and milestone incompleteness. Do not spend time on style unless it hides a real defect.

## Zero Known Issues Gate

If you verify any issue at any severity, the verdict is `Needs Fixes` until it is fixed. Low, Medium, High, and Blocker findings all block milestone acceptance.

Do not use `Accept With Follow-Ups` by default. A finding may remain only if the user explicitly approves deferring that exact issue and the deferral records issue ID, reason, owner, next checkpoint, and evidence path in the implementation log, changelog, checklist, and roadmap/DR docs if scope or risk changed.

## Contract Integrity Gate

Green commands are not enough. Prove the contract, not just that the process exits 0.

Always check:

- **Shared production paths:** `cf-app`, `cfctl`, `cf-control`, scenario loading, metadata generation, schema validation, and run-bundle writing must share the same core logic where they claim the same behavior.
- **No fake success:** `accepted`, `ok`, or `PASS` must mean the requested state change happened. Unsupported fields must reject or be explicitly documented as ignored. Silently consuming a flag without producing the side effect (e.g. `--capture-grid` with `--headless-smoke` producing zero PNGs) is a fake-success failure.
- **Mandatory field enforcement:** missing or malformed required fields must fail in the live protocol and tests.
- **Source-truthful evidence:** bundles and observations must reflect loaded scenario data, active config, current binary/git state, and actual runtime path.
- **Checklist truth:** checked rows cannot hide missing required work in notes with "deferred", "follow-up", "reserved", "stub", "fake", "placeholder", "not implemented", or equivalent wording.
- **Regression proof:** every verified bug fixed by an implementer must include a test or validation command that would have failed before the fix.
- **T-CAPTURE evidence (BP2 onward):** every fun-proof scenario must emit a `summary_grid.png` + `capture_manifest.json` recorded in `summary.json.artifacts`; the cf-e2e script must include `--expect capture.summary_grid.non_blank_ratio>=0.95` to catch black-frame regressions. See `cortext_command_vault/spec/prototype-roadmap.md` §T-CAPTURE.

If any item above is missing, verdict is `Needs Fixes`.

## Important Findings

Treat these as Blocker-level issues unless evidence proves a lower severity is more accurate:

- Code does not satisfy the assigned roadmap milestone, Build Point bundle, native backlog task card, feature checklist row, or open DR gate.
- Work is marked complete without run-bundle evidence when the milestone requires it.
- New player-facing UI/gameplay cannot be observed or controlled through `cfctl`.
- Sim, replay, physics, AI, material, or networking code introduces nondeterminism without an explicit approved reason.
- Run-bundle files are missing, invalid, unordered, unversioned, or not checked by the canonical checker.
- T-CAPTURE evidence (`summary_grid.png` + `capture_manifest.json`) is missing for a BP2+ fun-proof scenario, OR the cf-e2e script lacks the `capture.summary_grid.non_blank_ratio>=0.95` expectation, OR the per-BP human-playtest survey row in `prototype_runs/native/<bp>_*` notes does not reference the summary grid path it was answered against.
- User-controllable input can panic, corrupt state, access unintended paths, or bind control/server surfaces outside approved local boundaries.
- Tests are decorative, tautological, nondeterministic, or do not cover the behavior they claim to protect.
- Implementation changes require vault/checklist/changelog updates and those updates are missing.

## Nit Policy

Report at most five low-value nits. Suppress style-only comments that `cargo fmt`, `clippy`, or existing CI already catches unless the issue hides a real bug.

## Live Context

```!
cd /Users/erol/projects/corefall && {
  echo "## git status";
  git status --short;
  echo;
  echo "## diff stat";
  git diff --stat;
  echo;
  echo "## changed files";
  git diff --name-only;
  echo;
  echo "## recent commits";
  git log --oneline -8;
}
```

## Required Sources

Read these before judging the work:

- `/Users/erol/projects/corefall/AGENTS.md` (especially **Build Point Closure Gate** + **Contract Integrity Gate** + **Cursor Bugbot Loop** sections).
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md` (especially the **Build Points** table BP0..BP12 and the **§T-CAPTURE** side-track section).
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md`
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md` (per-milestone rows AND the **Build Points Checklist** addendum).
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-control-observability-layer.md`
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/references/prototype-run-bundle-schema.md` (especially the `captures/` rows and `summary.json.artifacts[].type` values).
- Relevant DRs and specs linked from the assigned milestone or Build Point.

If the review is for M0, also read [references/m0-review.md](references/m0-review.md).
For the full pass definitions, read [references/review-passes.md](references/review-passes.md).
For semantic contract failures, read [references/contract-integrity.md](references/contract-integrity.md).
Use [templates/review-report.md](templates/review-report.md) for the final report shape.

## Workflow

1. Determine review scope: working tree, commit, branch, PR, milestone, or feature. If ambiguous, review the working tree and say so.
2. Collect diff context with `git diff --stat`, `git diff --name-only`, and `git diff --find-renames --find-copies --unified=80`. If reviewing committed work, use `git show` or `git diff <range>`.
3. Run a full diff review: every changed line, every deletion, every public API change, every dependency/config/schema/CLI/test change.
4. Run a separate full affected-code review: read complete touched files, callers, tests, fixtures, schemas, crate `AGENTS.md`, and related modules.
5. Build a spec contract matrix: roadmap requirement, backlog task card, checklist row, DR gate, `cfctl` surface, evidence, status, gap.
6. Hunt edge cases: invalid input, empty input, duplicate IDs, out-of-order events, paused/shutdown state, bad paths, platform ordering, nondeterminism, panic paths, and boundary values.
7. Audit tests: identify what each test proves, what bug would make it fail, missing negative/property/fuzz/E2E tests, and any tautological or fixture-only tests.
8. Audit Rust and dependencies: panics, `unwrap`, error types, `unsafe`, deterministic data structures, dependency additions, feature flags, CI, and lints.
9. Audit Corefall-specific risks: fixed tick, seeded RNG, run bundles, event ordering, `schema_version`, replayability, `cfctl` eyes/ears/hands coverage, accessibility flags, performance budget, and vault/checklist/changelog coherence.
10. Run a Contract Integrity Review: shared code paths, no fake success, mandatory-field rejection, source-truthful evidence, no checklist laundering, and regression proof for every prior finding.
11. Verify with commands when feasible. If M0 or early work cannot test a later system, mark that item `Not yet testable` with the exact future milestone that owns it.
12. Produce findings first, ordered by severity, with file/line evidence. Include blockers, non-blocking gaps, missing tests, validation run/not run, and next fixes.
13. If any verified finding remains, set verdict to `Needs Fixes` unless the user explicitly approved deferring that exact finding.

## Severity

- **Blocker**: likely bug, broken milestone contract, missing required validation/evidence, nondeterminism in sim-critical code, security/control risk, or work marked done without checklist/run-bundle evidence.
- **High**: real defect or incomplete requirement that should be fixed before the milestone is accepted.
- **Medium**: meaningful gap, weak test, incomplete docs/checklist update, or maintainability risk likely to cause bugs soon.
- **Low**: useful cleanup or clarity issue. Keep these sparse.

All verified severities block milestone acceptance by default. Do not report speculative issues as blockers. If evidence is incomplete, say what you checked and what remains unknown.

## Final Output

Use this order:

1. Findings, ordered by severity, with `file:line` references.
2. Spec contract status: pass/fail/partial/not-yet-testable.
3. Validation status: commands run, pass/fail, commands skipped with reason.
4. Contract Integrity Matrix: each contract path, shared source of truth, positive proof, negative/adversarial proof, and checklist truth.
5. Test gaps and missing evidence.
6. Roadmap/checklist/changelog/vault updates required.
7. Verdict: Accept, Needs Fixes, or Not Reviewable. `Accept` requires zero unresolved verified findings unless every remaining finding has explicit user-approved deferral evidence.

## Loop Semantics

The skill is designed to be invoked in a loop after implementation: implement → review → fix → re-review → exit. Without explicit halt criteria, the loop is dangerous in three ways:

| Risk | Real-world failure |
|---|---|
| **Oscillation** | Worker fixes A → breaks B; fixes B → re-breaks A. Loop never ends. |
| **Trivial-nit ratchet** | Skill keeps producing 1 microscopic finding per iteration forever. |
| **Scope-bleed** | Skill auditing M0 finds drift in M5 docs. Worker "fixes" M5 → expands assignment unboundedly. |

### Halt Criteria

Run the skill in a loop until ANY of the following:

- **(a) Accept verdict.** Skill returns `Accept` with zero verified findings on the assigned milestone scope. Exit clean.
- **(b) Cycle.** Findings set in iteration N is identical to iteration N-1 (worker is stuck or oscillating). Halt and ask the user.
- **(c) Iteration cap.** Iteration N > 5. Halt and ask the user.
- **(d) Only out-of-scope findings remain.** All remaining findings touch milestones/areas other than the assigned scope. Batch them as `docs/implementation-log/<date>-<milestone>-out-of-scope-findings.md` and exit clean. Do NOT fix them in this loop.

When the loop halts via (b), (c), or (d), the milestone is NOT marked complete unless the user explicitly approves it.

### Out-Of-Scope Findings

A finding is out-of-scope when it touches a milestone, side track, decision record, or document that the assigned milestone does not own. Examples while reviewing M0:

- Drift in M5 task cards.
- Stale wording in DR-022's revisit trigger.
- A typo in the persistent MMO architecture spec.

Out-of-scope findings get logged, not fixed in this pass:

1. Append to `docs/implementation-log/<date>-<milestone>-out-of-scope-findings.md` with: finding text, file/line, severity, suggested fix, and which milestone/doc owns it.
2. Reference the file from the milestone's implementation log so the user can see them at handoff.
3. Do NOT modify the out-of-scope target files in this loop. The next milestone or a dedicated cleanup pass owns the fix.

This prevents scope-bleed without dropping the signal.

### Per-Iteration Discipline

Each iteration must:

1. Re-read live context (`git status`, `git diff`, recent commits).
2. Re-run the full Workflow steps for the assigned scope.
3. Compare findings set to the prior iteration. If identical → halt criterion (b) fires.
4. Fix every Blocker/High/Medium/Low finding on assigned-milestone scope before re-running the skill. Out-of-scope findings are logged, not fixed.
5. Append the iteration's verdict + finding count + fix summary to the milestone implementation log so the loop trail is auditable.
