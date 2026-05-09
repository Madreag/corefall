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

## Self-Play Validation Gate

Source-truth + unit tests are necessary but not sufficient. The reviewer must confirm the implementing agent self-played the milestone through the production cf-control / cfctl path AND visually confirmed the result by reading `summary_grid.png`. See `corefall/AGENTS.md` § **Self-Play Validation Rule** for the four axes (Hands / Eyes / Ears / Hear) and the Mandatory Self-Play Validation Matrix.

Every milestone closeout must include a row-per-action matrix proving:

1. **Hands (act)** — the action was driven via `cf-e2e --script <s> --capture-grid` through real `act.player.*` / `act.settings.*` / `scenario.*` / `runbundle.*` JSON-RPC methods.
2. **Eyes (see)** — the action's visible result is verifiable in `summary_grid.png` (the agent personally read the PNG with the `Read` tool, looked at the pixels, and wrote a one-sentence prose confirmation per action — NOT just the `non_blank_ratio` metric, NOT "looks fine", NOT "PASS"). When per-action keyframes are produced (cf-capture event keyframes), the agent reads those too and confirms the per-action visual change explicitly. "I checked the source" or "the test passed" is a Hands cell, not an Eyes cell.
3. **Ears (observe + events)** — the action emits a structured event in `events.jsonl` AND the resulting state is reachable via `observe.once` or `inspect.*`.
4. **Hear (audio events) [BP6+]** — once `cf-audio` ships, audible events emit `audio.event_fired` rows. Until then this axis is "no audio surface yet" (no-op, not deferral).

Plus mandatory mission-win + mission-loss + headless-smoke + 60 Hz + 120 Hz determinism rows.

If any row says FAIL, n/a-by-default-but-actually-needed, "deferred", or "I checked the source", the verdict is `Needs Fixes`. If the harness can't fill a row (e.g. cf-e2e can't pass a needed setting; observe.once doesn't expose a needed field), the harness gap is a milestone bug — flag it in findings and the implementer must fix the harness in the same pass per the "make it possible" clause.

## BP Goal Coverage Gate

The Self-Play Validation Gate proves *every cfctl action works*. The BP Goal Coverage Gate proves *the BP delivers what its roadmap entry says it delivers*. These are different contracts.

Every BP review must include a **BP Goal Coverage Report** that:

1. **Quotes the BP's stated goals verbatim** from `cortext_command_vault/spec/prototype-roadmap.md` (Build Points table + per-milestone done-criteria + the BP's fun-proof slice description). This is what the project owner signed up to ship.
2. **Maps each goal to evidence** in the closure run bundle:
   - Goal → cfctl action(s) that exercise it → `summary_grid.png` frame the agent personally read → `events.jsonl` event type + count → `observe.once` field that confirms post-state → unit/integration test that locks the contract.
3. **Articulates the look + feel + juice** the agent observed in the captures. The agent must write prose like "frame 14 shows the dirt shield mound between the guard sprite at x=900 and the reactor sprite at x=600; frame 41 shows the same shield with a horizontal tunnel carved at y=32; frame 58 shows two projectile sprites passing through the tunnel and one impact sprite at the reactor's AABB" — NOT "the captures look correct" or "the summary grid is non-blank".
4. **Calls out missed goals** with severity. If the BP's roadmap entry says "the player should feel the strategic choice between preserving and breaching the dirt shield" and the captures only show the breaching path, that's a `Needs Fixes` finding, NOT an "Accept with follow-ups".

If the BP Goal Coverage Report is missing, has placeholder text, or has goals where the "evidence" cell points at a unit test instead of a capture frame + event row + observe field, the verdict is `Needs Fixes`.

The agent fills this report itself as part of the review; it is NOT delegable to a human playtest. Human playtest is optional confirmation (see below).

## AI-Agent Self-Test Report Gate (replaces mandatory human playtest)

The corefall AGENTS.md previously required a per-BP human-playtest survey row in `prototype_runs/native/<bp>_*/notes.md` with the question "did the new systems make the game more fun than the previous BP?". That row is now **optional confirmation**, NOT a closure gate. The closure gate is the **AI-Agent Self-Test Report**, which the implementing agent OR the reviewer produces from the run bundle + capture grid + observe surface.

The AI-Agent Self-Test Report must answer, per BP, with concrete evidence:

| Question | What the agent must produce |
|---|---|
| **Q1.** What does this BP claim to deliver, in the project owner's words? | Verbatim quotes from `prototype-roadmap.md` BP table row + fun-proof slice description + per-milestone done-criteria. |
| **Q2.** Does the playable scenario(s) deliver Q1's claim end-to-end through cfctl-driven inputs? | Per cfctl action: tick range exercised → `summary_grid.png` frame indices → events emitted → observe field reached. |
| **Q3.** Does the visual presentation match the maturity level the BP promises? | Agent prose describing what frames N, N+10, N+30, ... show: actor sprite movement, terrain deformation, projectile flight, HUD updates, mission-state transitions, lighting/effects. |
| **Q4.** Does the simulation behavior match the project owner's stated feel? | Agent compares observed behavior against BP's "feel" claim (e.g. "actors move with weight", "terrain refuses metal_nohook", "reactor takes damage from projectile-vs-AABB hits", "guard utility-scores tactics"). One row per feel claim. |
| **Q5.** Are there obvious inside-scope affordances the BP's text implies but the implementation skipped? | The Minimum-Bar Design Coverage check, expressed as an action list. Empty list = clean. |
| **Q6.** Did the BP regress any prior-BP feel/feature? | Run prior-BP scenarios under the new build; confirm summary_grid.png + final_sim_checksum still match the prior-BP exemplar (or the new behavior is documented as intentional). |
| **Q7.** What would a human playtester see in the first 30 seconds that the AI agent missed? | Honest disclosure section. If the agent can't think of anything, write "no candidate gaps identified by AI agent — human playtest still recommended for novelty signal but not gating". |

The report is written into `prototype_runs/native/<bp>_*/notes.md` under the heading **`## AI-Agent Self-Test Report`**. The agent's identity (Droid + the model used, e.g. `Droid (claude-sonnet-4.5-20250522)`) and timestamp must be recorded so future reviewers can see which agent run produced the report.

Optional: a **`## Human Playtest Survey (optional confirmation)`** section can sit below it with the project owner's notes after they actually play the BP. The owner's row is **non-gating**; its absence does NOT block BP closure when the AI-Agent Self-Test Report is complete and Accept-verdicted.

The reviewer's verdict on the BP IS the verdict on the AI-Agent Self-Test Report. If the report is missing, hand-waved, or has placeholder text where the agent's prose observation should be, the verdict is `Needs Fixes`. The agent does not get to skip writing the prose by saying "the test passed" — the prose IS the test.

## Hands/Eyes/Ears Capability Floor (the agent's responsibility, enforced here)

The AI agent is the playtest mechanism for Corefall. That is only true if the agent has full hands/eyes/ears coverage of the game at the milestone's maturity level. The reviewer must confirm:

- **Hands floor:** Every player-facing action AND every settings/scenario/runbundle command the milestone introduces has a JSON-RPC method exposed via cf-control + a cfctl script that exercises it. Mouse/keyboard input device injection (`act.input.key_press` / `act.input.mouse_click` / `act.input.mouse_move`) is forward-looking; it is required as a stub from BP3 onward (when M4A introduces UI surfaces that need click testing) and required as a working surface from BP3-M4A close.
- **Eyes floor:** cf-capture must produce `summary_grid.png` + per-event keyframe PNGs for every fun-proof scenario from BP2 onward. The agent must be able to read each PNG with the `Read` tool. `--capture-each-action` (cf-e2e flag) forces a keyframe at each cfctl action's tick from BP3 onward so the agent can write per-action visual prose without manually correlating tick → frame.
- **Ears floor:** `observe.once` must surface every player-visible state the milestone introduces (HUD numbers, inventory, mission step, terrain stats, reactor hp, etc.). Every player-visible state change must emit a structured event in `events.jsonl` so an offline reviewer can grep for it without replaying the run.
- **Hear floor [BP6+]:** Once `cf-audio` ships, every audible event emits `audio.event_fired` so the agent can verify the same way it verifies visual events.

If any floor row is missing for a milestone-scope surface, the milestone is incomplete (harness gap = milestone bug per the "make it possible" clause). The reviewer must explicitly call this out and require the harness extension in the same pass.

## Universal Enhancement Audit Gate (DR-056)

Per `cortext_command_vault/spec/milestone-enhancement-pass-m1-plus.md`, every M1+ milestone inherits the Universal Enhancement Done-Criteria on top of its own scope. Audit:

- **Per-tier perf gate** (Steam Deck 800p/60 + 1080p/60 + 4K/120) recorded in `summary.json.performance` or `cf-bench` report.
- **CI bench regression** (no >5% regression vs baseline) per DR-054.
- **Memory leak soak** (24h+) clean per DR-051 + DR-054 — may stage at BP boundary; document in implementation log.
- **Network sync verified** via `cfctl test sync-drift` (or equivalent) per DR-052.
- **Replay determinism CI matrix** (per platform + per architecture) per DR-002 + DR-052.
- **All player surfaces scriptable via cfctl** — Eyes/ears/hands rule (T-CONTROL).
- **AI-agent-driven validation report** logged per DR-026 + DR-056.
- **All audio cues generated via DR-053 pipeline** + usage-ledger logged.
- **Game feel / juice rules** per DR-055 — every gameplay event has authored juice response.
- **Accessibility ACC-A floor verified** (UI 200% + high contrast + captions + reduced motion) per DR-012.
- **Localization keyed strings** (Tier-A 11 languages) verified per DR-046 — may stage at BP boundary if scope is English-only prototype; document staging.
- **Modding parity verified** (mod-author can extend; mod-test-run AI agent validates) per DR-006 + DR-050.
- **Anti-FOMO + anti-pay-to-win audit** passes per DR-031.
- **Captions for ALL audio** (full-subtitle option) per DR-051.

Plus per-milestone specifics from `milestone-enhancement-pass-m1-plus.md`. If a Universal row FAILS without explicit user-approved deferral evidence, verdict is `Needs Fixes`.

## Minimum-Bar Design Coverage Gate

Roadmap V2, task cards, and DRs are a **minimum bar**, not a ceiling. During review, verify that the implementer performed a design-coverage pass before acceptance.

For every player-facing feature, physical entity, AI behavior, UI surface, scenario, tool command, or release artifact touched by the scope, ask:

- Does it satisfy the literal roadmap/backlog/DR text?
- Does it also include the obvious game-facing behavior a player would expect from that feature at this maturity level?
- Is the behavior visible/readable in the game, observable through `cfctl`, recorded in replay/run bundles, testable through E2E, and represented in capture evidence when visual?
- Does it avoid static/no-op/fake versions of core promises such as sliding actors, fake objective flags, fake success responses, cosmetic-only physics, hidden simulation with no feedback, or untracked release evidence?
- If an expected behavior is intentionally not in scope, is the exclusion documented in the milestone note/checklist/roadmap with the owning future milestone?

The review should not force later-milestone scope into an early milestone. It should block acceptance when the implemented feature is materially weaker than the milestone's own player promise or when the agent skipped an obvious inside-scope affordance.

## Important Findings

Treat these as Blocker-level issues unless evidence proves a lower severity is more accurate:

- Code does not satisfy the assigned roadmap milestone, Build Point bundle, native backlog task card, feature checklist row, or open DR gate.
- Work is marked complete without run-bundle evidence when the milestone requires it.
- New player-facing UI/gameplay cannot be observed or controlled through `cfctl`.
- Sim, replay, physics, AI, material, or networking code introduces nondeterminism without an explicit approved reason.
- Run-bundle files are missing, invalid, unordered, unversioned, or not checked by the canonical checker.
- T-CAPTURE evidence (`summary_grid.png` + `capture_manifest.json`) is missing for a BP2+ fun-proof scenario, OR is not recorded in `summary.json.artifacts.items[]`, OR the cf-e2e script lacks the `capture.summary_grid.non_blank_ratio>=0.95` expectation.
- **BP Goal Coverage Gate failure:** the BP's stated goals from `prototype-roadmap.md` are not all mapped to evidence (cfctl action + frame + event + observe field), OR the agent's prose articulation of look/feel/juice is missing, OR the agent claims a goal is met without reading the relevant `summary_grid.png` frame.
- **AI-Agent Self-Test Report missing or hand-waved:** the report at `prototype_runs/native/<bp>_*/notes.md` lacks any of Q1..Q7, has placeholder text instead of agent prose, or lacks the agent identity + timestamp.
- **Source-truthful evidence violation:** run bundles, summaries, observations, or checklist rows contain hardcoded metadata that does not reflect the loaded scenario, active config, current binary/git state, or actual runtime path. Examples: every bundle's `summary.json.tests[0].id` hardcoded to "M0-SMOKE-01" regardless of milestone; `summary.json.next_actions` hardcoded to "Proceed to M1" for an M2.5 bundle; `summary.json.notes_extra` containing M0-only DR-002 staging prose ("M2/M3 will append terrain bytes") in an M2.5 bundle that already shipped that capability; `summary.json.artifacts.items[]` empty when `captures/summary_grid.png` exists on disk.
- The implementation meets a narrow task-card wording but misses an obvious inside-scope player-facing affordance, feedback state, `cfctl` observation/action, replay event, failure path, physical profile, or UX expectation implied by the milestone and product promise.
- User-controllable input can panic, corrupt state, access unintended paths, or bind control/server surfaces outside approved local boundaries.
- Tests are decorative, tautological, nondeterministic, or do not cover the behavior they claim to protect.
- Implementation changes require vault/checklist/changelog updates and those updates are missing.

> **Note:** the per-BP **human-playtest** survey is **optional confirmation**, not a Blocker. The closure gate is the AI-Agent Self-Test Report (above). Missing human-playtest row is acceptable when the AI report is complete and Accept-verdicted.

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

- `/Users/erol/projects/corefall/AGENTS.md` (especially **Build Point Closure Gate** + **Contract Integrity Gate** + **Universal Enhancement Contract (DR-056)** + **Minimum Bar And Enhancement Rule** + **Self-Play Validation Rule** + **Cursor Bugbot Loop** sections).
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md` (especially the **Build Points** table BP0..BP12, the Universal Enhancement Done-Criteria callout above the Milestone Details header, the **Design-Completeness Map**, and the **§T-CAPTURE** + **§T-RELEASE** side-track sections).
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md`
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md` (per-milestone rows AND the **Build Points Checklist** addendum).
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/milestone-enhancement-pass-m1-plus.md` (Universal Enhancement Done-Criteria + per-milestone enhancement specifics for the assigned milestone).
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-control-observability-layer.md`
- `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/references/prototype-run-bundle-schema.md` (especially the `captures/` rows and `summary.json.artifacts[].type` values).
- Relevant DRs and specs linked from the assigned milestone or Build Point (including DR-052..057 for the universal contract).

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
10. Run a Minimum-Bar Design Coverage Review: confirm the milestone includes expected player-facing behavior, readability, events, `cfctl`, capture, perf, save/mod/accessibility hooks that are inside its scope; log future-scope gaps with the owning milestone.
11. Run a Contract Integrity Review: shared code paths, no fake success, mandatory-field rejection, source-truthful evidence, no checklist laundering, and regression proof for every prior finding.
12. Run a **Self-Play Validation Review**: confirm the implementing agent produced the Self-Play Validation Matrix (Hands / Eyes / Ears / Hear rows + mission win/loss + headless smoke + 60+120 Hz determinism rows), confirm `summary_grid.png` was personally read for every `act.*` action exercised, and confirm the harness was extended in-pass when a row could not be filled (the "make it possible" clause).
13. Run the **BP Goal Coverage Gate**: enumerate the BP's stated goals verbatim from `prototype-roadmap.md` and map each goal to evidence (cfctl action → frame → event → observe field → test). Articulate the look/feel/juice in agent prose. Block on missing prose or hand-waved goals.
14. Run the **AI-Agent Self-Test Report Gate**: confirm `prototype_runs/native/<bp>_*/notes.md` contains the `## AI-Agent Self-Test Report` section answering Q1..Q7 with concrete evidence, plus the agent identity + timestamp. Human-playtest survey row is optional and does not block when this report is complete.
15. Run the **Hands/Eyes/Ears Capability Floor check**: every milestone-scope action has a JSON-RPC method + cfctl script; every fun-proof scenario has summary_grid.png + per-event keyframes recorded in summary.json.artifacts.items[]; every player-visible state is reachable via observe.once and emits a structured event. Missing floor row = milestone bug ("make it possible" clause).
16. Run a **Universal Enhancement Audit (DR-056)**: confirm every M1+ milestone's universal rows PASS (per-tier perf, CI bench regression, memory-leak soak, network sync, replay determinism CI, cfctl scriptability, AI-agent validation report, AI audio pipeline, juice rules, ACC-A floor, Tier-A localization keyed strings, modding parity, anti-FOMO + anti-pay-to-win audit, captions for ALL audio). Allow staging at BP boundary only when documented.
17. Run a **Design-Completeness Map cross-check**: locate the milestone in the Design-Completeness Map; if the milestone delivers a row in that map, confirm the row's claim against the implementation. If the map and the implementation diverge, flag as a roadmap drift finding.
18. Verify with commands when feasible. If M0 or early work cannot test a later system, mark that item `Not yet testable` with the exact future milestone that owns it.
19. Produce findings first, ordered by severity, with file/line evidence. Include blockers, non-blocking gaps, missing tests, validation run/not run, and next fixes.
20. If any verified finding remains, set verdict to `Needs Fixes` unless the user explicitly approved deferring that exact finding.

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
5. Minimum-Bar Design Coverage Matrix: feature/player promise, expected affordance, implemented evidence, omitted/future-owned items.
6. **Self-Play Validation Matrix verification**: confirm every Hands/Eyes/Ears/Hear row passes; flag any "I checked the source" entries; confirm the agent personally read `summary_grid.png` and wrote per-action visual prose.
7. **BP Goal Coverage Report**: verbatim BP goals from `prototype-roadmap.md` → cfctl action(s) → `summary_grid.png` frame indices the agent read → events emitted → observe field reached → unit/integration test → agent prose articulating look/feel/juice. One row per stated goal; no hand-waving allowed.
8. **AI-Agent Self-Test Report verification**: confirm `prototype_runs/native/<bp>_*/notes.md` contains a complete `## AI-Agent Self-Test Report` section with Q1..Q7 answered concretely, plus agent identity + timestamp. Note whether the optional human-playtest survey row is also present (acknowledgment only; not a gate).
9. **Hands/Eyes/Ears Capability Floor check**: confirm every milestone-scope action has a cfctl method + script; every fun-proof has summary_grid.png + per-event keyframes recorded in summary.json.artifacts.items[]; every player-visible state is in observe.once + events.jsonl. Missing floor row = harness bug.
10. **Universal Enhancement (DR-056) Audit**: per-tier perf gate + CI bench + memory-leak soak + network sync + replay determinism CI + cfctl scriptability + AI-agent validation + AI audio + juice + ACC-A floor + Tier-A localization + modding parity + anti-FOMO + captions; flag staged rows.
11. **Design-Completeness Map cross-check**: confirm the milestone's row in the map matches the implementation.
12. Test gaps and missing evidence.
13. Roadmap/checklist/changelog/vault updates required.
14. Verdict: Accept, Needs Fixes, or Not Reviewable. `Accept` requires zero unresolved verified findings unless every remaining finding has explicit user-approved deferral evidence.

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
