---
type: spec
status: research-backed-review-protocol
authority: "Recommended review skill stack for AI agents reviewing Corefall implementation work."
last_updated: 2026-05-06
feeds:
  - DR-002
  - DR-003
  - DR-008
  - DR-012
  - DR-024
  - DR-026
  - DR-033
  - DR-034
  - DR-035
  - DR-036
---

← [[index|vault home]] · [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/feature-completion-checklist|feature checklist]] · [[spec/ai-coder-reading-list|AI coder reading list]] · [[spec/ai-control-observability-layer|AI control layer]]

# AI Code Review And Bug-Hunt Skills

> [!summary] Recommendation
> Corefall should use a staged review stack, not a single generic "review this code" prompt. Every milestone should get a full diff review, then a separate full affected-code review, then specialized passes for spec coverage, test quality, determinism/replay, security/supply chain, performance, `cfctl` observability, and vault/checklist coherence. The project-local Claude Code skill is installed at `/Users/erol/projects/corefall/.claude/skills/corefall-review/SKILL.md` and is invoked as `/corefall-review <milestone-or-range>`.

> [!important] Why this exists
> Agent-written code often looks polished while still being wrong: hallucinated APIs, decorative tests, stale patterns, overbuilt abstractions, missing edge cases, and confidently wrong product logic. Corefall adds harder failure modes: deterministic sim drift, replay mismatch, unchecked event schemas, hidden wall-clock state, platform-specific ordering, incomplete `cfctl` surfaces, and stale planning docs.

> [!danger] Zero known issues by default
> Low, Medium, High, and Critical findings all block milestone acceptance once verified. `Accept With Follow-Ups`, `known issue`, or `defer` is allowed only when the user explicitly approves that exact finding. The deferral must record issue ID, reason, owner, next milestone/checkpoint, and evidence path in the implementation log, changelog, checklist, and roadmap/DR docs if scope or risk changed.

> [!failure] Contract integrity is mandatory
> Reviewers must prove shared code paths, required-field rejection, source-truthful run bundles, real command semantics, and checklist truth. Green validation is insufficient if app/tool/server paths diverge, commands fake success, or checklist rows hide required missing work.

## Bottom Line

Use this stack after every meaningful implementation pass:

1. **Diff Impact Review** - review exactly what changed and what was deleted.
2. **Full Affected-Code Review** - read the full files, callers, tests, configs, and docs around the diff.
3. **Spec Contract Gap Review** - compare implementation against roadmap/backlog/checklist/DRs.
4. **Edge-Case Path Hunter** - mechanically enumerate branches, state transitions, boundaries, and unhandled paths.
5. **Test Quality Auditor** - verify tests would fail for real bugs; reject tautological tests.
6. **Rust Safety And Idiom Reviewer** - check Rust toolchain, `unsafe`, panics, error handling, dependency hygiene, and idioms.
7. **Determinism / Replay / Run-Bundle Reviewer** - check fixed tick, RNG, event order, checksums, replayability, and run-bundle evidence.
8. **Security / Secrets / Supply Chain Reviewer** - check loopback controls, secrets, auth boundaries, untrusted content, licenses, and dependency risks.
9. **Performance / 4K120 Reviewer** - check frame/tick budgets, allocation, event volume, data structures, and scalability.
10. **AI Control / Observability Reviewer** - confirm every new player-facing surface is reachable through `cfctl` and emits events.
11. **Vault / Roadmap / Checklist Coherence Reviewer** - check planning docs, changelog, implementation logs, and feature checklist updates.
12. **Contract Integrity Reviewer** - prove shared source-of-truth paths, mandatory field rejection, no fake success, and no checklist laundering.
13. **Review Synthesis Judge** - dedupe findings, verify evidence, rank severity, and decide whether the milestone is actually done.

This is now installed as a real Claude Code project skill and repo review rule set:

| Installed File | Purpose |
|---|---|
| `/Users/erol/projects/corefall/.claude/skills/corefall-review/SKILL.md` | Claude Code skill entrypoint with YAML frontmatter, dynamic git context, required sources, zero-known-issues gate, contract-integrity gate, workflow, severity, and output contract. |
| `/Users/erol/projects/corefall/.claude/skills/corefall-review/references/review-passes.md` | Supporting reference for the separate review passes. Load when doing a full review. |
| `/Users/erol/projects/corefall/.claude/skills/corefall-review/references/m0-review.md` | M0-specific overlay: what M0 must have, must not grow into, and what is not yet testable. |
| `/Users/erol/projects/corefall/.claude/skills/corefall-review/templates/review-report.md` | Review report template. |

## How To Invoke It

From a Claude Code session rooted in `/Users/erol/projects/corefall`, use:

```text
/corefall-review M0
```

or:

```text
/corefall-review HEAD~1..HEAD
```

If no argument is supplied, the skill reviews the current working tree. Claude Code should also auto-discover it for prompts like "review M0", "bug hunt this milestone", "find misses", or "is this implementation done?"

> [!tip] M0 is valid review scope
> Use `/corefall-review M0` now. The installed skill has an M0 overlay that distinguishes **required M0 evidence** from **not-yet-testable future systems**. It should not fail M0 for lacking actors, terrain, AI, materials, multiplayer, or real gameplay; it should fail M0 for missing workspace structure, `cfctl` no-op control, fixed tick, run-bundle evidence, schema/versioning, accessibility flags, validation, checklist/changelog updates, or silent DR assumptions.

## Research Takeaways

| Finding | What It Means For Corefall |
|---|---|
| Diff-only review misses cross-file bugs and intent bugs. | Review changed lines first, then separately inspect full files, callers, specs, schemas, tests, and run bundles. |
| Multi-agent/specialist review works better than one broad reviewer. | Use domain reviewers: bug hunter, security, test quality, determinism, performance, docs/vault, and final judge. |
| False positives destroy trust. | Require file/line evidence, repro/test suggestion, and a verification pass before findings are reported as blocking. |
| Agent-written tests can be decorative. | Every test added by an implementor must be checked by asking: "What bug would make this fail?" |
| Requirements and behavioral contracts must be explicit. | The reviewer must compare code against roadmap task cards, DRs, run-bundle schema, and checklist rows. |
| Green commands can still be false evidence. | Add a Contract Integrity Matrix: shared source of truth, positive proof, negative/adversarial proof, and checklist truth for each contract path. |
| Game determinism needs permanent tooling. | Reviewers must look for checksums, deterministic RNG, ordered iteration, stable entity IDs, fixed tick, and replay divergence tools. |
| Security review must include business logic and trust boundaries. | `cf-control`, `cf-server`, mod loading, package validation, LLM providers, and save files need deeper review than ordinary app code. |
| Rust safety needs tool-backed review. | Use `cargo fmt`, `cargo check`, `clippy -D warnings`, tests, `cargo deny`, `cargo audit`, `miri` where applicable, fuzz/property tests for parsers and schemas, and `loom` for concurrency primitives. |
| Review instructions must be repo-specific and short enough to apply. | Put Corefall-specific review rules in a dedicated doc/skill; avoid generic "write good code" filler. |
| Review work must feed the vault. | Every review outcome should update `feature-completion-checklist`, implementation logs, user-approved deferrals, and relevant DR evidence. |

## Recommended Skill Stack

| Skill | Use When | Reads | Must Produce |
|---|---|---|---|
| `corefall-review-coordinator` | Any milestone/feature review. | All required docs below, `git diff`, test outputs, run bundles. | Unified review plan, spawned/pass-by-pass results, deduped final report. |
| `corefall-diff-reviewer` | First pass after implementation. | `git status`, `git diff --stat`, `git diff --name-only`, full diff. | Line-specific findings only from changed/deleted code plus direct integration context. |
| `corefall-full-code-reviewer` | Second pass after diff review. | Full touched files, callers, tests, configs, crate AGENTS, related modules. | Findings diff-only review could miss: wrong abstraction, hidden coupling, stale callers, dead code, bad module ownership. |
| `corefall-contract-gap-finder` | Any roadmap/backlog milestone. | Roadmap milestone, backlog task cards, checklist rows, DR gates, run-bundle schema. | Matrix of requirement -> evidence -> pass/fail/gap. |
| `corefall-edge-case-path-hunter` | Logic, parsers, state machines, event schemas, AI, sim systems. | Focused functions/modules or diff hunks. | JSON findings for unhandled path, trigger condition, guard, consequence. |
| `corefall-test-quality-auditor` | Any new or changed tests. | Tests plus implementation and spec. | "Would this fail?" assessment, missing edge cases, flaky/tautological/mock-only tests, needed property/fuzz/E2E tests. |
| `corefall-rust-safety-reviewer` | Any Rust code. | Workspace, dependencies, unsafe blocks, errors, panics, lints. | Rust-specific findings: `unsafe`, panic paths, overflow, indexing, Send/Sync, FFI, dependency audit, idiomatic API. |
| `corefall-determinism-replay-reviewer` | Sim, replay, physics, AI, networking, server, run bundles. | `cf-sim-core`, `cf-replay`, events, checksums, RNG, scenario manifests, run bundles. | Determinism risks, replay mismatch risks, missing checksums/events, first-divergence tooling gaps. |
| `corefall-physics-material-reviewer` | Collision/material/terrain/atmosphere/damage milestones. | DR-033, DR-036, physics/material specs, tests, run bundles. | Collision matrix gaps, CCD misses, impulse-to-damage gaps, material reaction order, atmosphere hazards, perf risk. |
| `corefall-network-server-reviewer` | M9-M12 or any server/control protocol change. | Server specs, protocol schemas, transport code, run bundles, security rules. | Authority, determinism island, auth, replay alignment, shard persistence, protocol compatibility findings. |
| `corefall-ai-behavior-reviewer` | AI logic, trust harness, LLM mind layer. | DR-008, DR-022, DR-032, AI harness specs, event logs. | Humanlike-bar coverage, explainability, stuck recovery, fairness, deterministic fallback, LLM schema/cost/privacy risks. |
| `corefall-ux-accessibility-reviewer` | UI, HUD, settings, controls, tutorials, editor. | DR-012, UI specs, `cfctl` surfaces, captures if present. | UI scale/contrast/caption/input/focus issues, `cfctl` accessibility gaps, overlap/readability risks. |
| `corefall-performance-reviewer` | Any sim/render/server/data structure change. | Bench output, hot loops, event volume, allocations, data structures. | 4K/120 and tick-budget risks, memory growth, benchmark gaps, data layout suggestions. |
| `corefall-contract-integrity-reviewer` | Every milestone, especially tool/app/control/replay paths. | App/CLI/server/control paths, config builders, schemas, metadata, run bundles, checklist notes. | Shared-path proof, negative/adversarial tests, fake-success findings, checklist-laundering findings. |
| `corefall-security-supply-chain-reviewer` | Any control/server/mod/LLM/dependency/secrets code. | OWASP-style review, dependency manifests, environment handling, logs. | Exploitable or concretely dangerous findings; no speculative noise. |
| `corefall-vault-coherence-reviewer` | End of every implementation/review pass. | Vault docs, checklist, implementation log, changelog, DRs. | Stale docs, missing checklist evidence, broken links, stale names, unclosed DR protocol gaps. |
| `corefall-review-synthesis-judge` | After specialist passes. | All findings and verification evidence. | Deduped final finding list, severity, confidence, blockers, user-approved deferrals, required next fixes. |

## Review Workflow For Every Milestone

### 0. Preflight

The reviewer must read:

- `/Users/erol/projects/corefall/AGENTS.md`
- `/Users/erol/projects/cortex-command-repos-all/AGENTS.md`
- [[spec/ai-coder-reading-list]]
- [[spec/prototype-roadmap]]
- [[spec/native-implementation-backlog]]
- [[spec/feature-completion-checklist]]
- [[spec/ai-control-observability-layer]]
- [[references/prototype-run-bundle-schema]]
- The assigned milestone detail, backlog task cards, relevant DRs, implementation log, and run bundle.

Then collect:

```bash
cd /Users/erol/projects/corefall
git status --short
git diff --stat
git diff --name-only
git diff --find-renames --find-copies
git diff --unified=80
```

If the work was already committed, replace `git diff` with the exact commit or range being reviewed, for example:

```bash
git show --stat --find-renames <commit>
git show --find-renames --find-copies --unified=80 <commit>
```

### 1. Full Diff Review

Purpose: catch bugs introduced directly by the change.

Required checks:

- Every changed line read.
- Every deletion explained.
- Every changed public API checked against callers.
- Every new dependency, feature flag, env var, config, schema, and CLI flag checked.
- Every new test checked for meaningful assertions.
- Any skipped validation command explained.

Output: findings only when there is a concrete issue, with file/line evidence.

### 2. Full Affected-Code Review

Purpose: catch bugs hidden outside the diff.

Required checks:

- Open full touched files.
- Search callers/usages with `rg`.
- Open direct tests, integration tests, fixtures, schemas, and run-bundle consumers.
- Check crate boundaries against roadmap ownership.
- Check whether a simpler existing helper already exists.
- Check for stale or duplicated patterns the agent imported from training data.

Output: cross-file issues and architecture/maintenance findings.

### 3. Spec Contract Gap Review

Purpose: catch "works but not what we asked for."

Build this matrix:

| Contract | Source | Evidence | Status | Gap |
|---|---|---|---|---|
| Roadmap milestone requirement | Roadmap heading/link | Code/test/run-bundle/log | Pass/Fail/Partial | Specific missing work |
| Backlog task card | Native backlog | Code/test/run-bundle/log | Pass/Fail/Partial | Specific missing work |
| Checklist row | Feature checklist | Evidence cell | Pass/Fail/Partial | Required update |
| DR gate | Decision record | Closure/evidence note | Pass/Fail/Partial | Confirm/AskUser/update DR |
| `cfctl` surface | AI control layer | command/result | Pass/Fail/Partial | Missing observe/inspect/act |

No milestone is done if a required contract has no evidence.

### 4. Edge-Case Path Hunter

Use this pass on state machines, parsers, schemas, CLI flags, sim systems, AI decisions, save/load, networking, material reactions, collision resolution, and replay logic.

For every branch/path:

- Empty input.
- Missing optional field.
- Extra unknown field.
- Invalid enum/value.
- Boundary value: 0, 1, max, negative if relevant.
- Tick 0, final tick, paused sim, shutdown during work.
- Duplicate IDs.
- Out-of-order events.
- Concurrent command arrival.
- Lost connection, timeout, canceled request.
- Bad file path, missing file, permission error.
- Cross-platform path/case differences.
- Floating-point or integer overflow.
- Non-deterministic iteration.
- Panic path.

Report only unhandled paths.

### 5. Test Quality Review

Reject tests that only prove the code runs.

For every test:

- What behavior does it protect?
- What bug would make it fail?
- Does the expected value come from independent reasoning or from the same implementation path?
- Does it cover an error path?
- Is it deterministic?
- Is it isolated?
- Is it too tightly coupled to internal implementation?
- Does it include meaningful scenario/run-bundle evidence where required?

Required test types by area:

| Area | Required Review Bias |
|---|---|
| Parsers/schemas/manifests | Property tests and malformed input tests. |
| Replay/events | Golden fixtures, JSONL validation, ordering, parent links, checksum fields. |
| Sim/physics/materials | Deterministic fixtures, edge positions, high speed, tiny bodies, contact order, perf counters. |
| Networking/server | Multi-client alignment, timeout, reconnect, malicious/invalid packets, authority checks. |
| AI | Scenario harness, reason labels, stuck recovery, reproducible decision traces. |
| UI/accessibility | `cfctl` assertions, focus traversal, scale/contrast/caption checks. |

### 6. Rust Safety And Dependency Review

Required checks:

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo deny check` once configured.
- `cargo audit` once configured.
- `cargo +nightly miri test` for crates that use unsafe code, pointer-heavy data, or critical sim containers.
- `RUSTFLAGS="--cfg loom" cargo test --test <loom_test> --release` for concurrency primitives once introduced.
- `cargo fuzz` for parsers/protocol decoders/package loaders once introduced.
- `cargo nextest run` for scalable CI once the workspace grows.

Manual Rust review:

- No `unsafe` unless isolated, justified, documented, tested, and preferably Miri-covered.
- No hidden panics in libraries; use `thiserror` in libs and `anyhow` in bins.
- No indexing without bounds reasoning in untrusted or sim-critical paths.
- No `HashMap`/`HashSet` iteration feeding deterministic sim outcomes unless order is explicitly sorted.
- No `SystemTime`, wall-clock, thread scheduling, OS randomness, locale, filesystem ordering, or nondeterministic IDs in sim state.
- Explicit initialization for all sim data.
- Stable serialization and schema versioning.
- `Cargo.lock` tracked and dependency additions justified.

### 7. Corefall-Specific Determinism / Replay Review

This pass is mandatory for M0+ even if the milestone is not "about replay."

Check:

- Fixed sim tick is the only gameplay time source.
- Seeded deterministic RNG is used and its state is included in checksums/events where relevant.
- Event IDs are stable and ordered.
- Run-bundle JSONL is valid one-event-per-line.
- `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md` exist when required.
- `schema_version` is present on protocol and run-bundle surfaces.
- Checksums are present where the milestone promises them.
- Replay acceptance gate exists or an explicit milestone reason says "not yet."
- State affected by gameplay is not stored only in renderer/UI/audio objects.
- Platform-specific ordering is neutralized.
- If two systems can run concurrently, review the handoff ordering.

Reviewers should assume future M9-M12 networking will magnify any determinism weakness.

### 8. Security / Control / Supply Chain Review

Apply this to `cf-control`, `cfctl`, `cf-server`, mod/package loading, LLM provider integrations, save/persistence, networking, and any dependency change.

Check:

- Control API binds to loopback by default.
- No remote bind unless explicit capability and documented risk.
- No secrets in repo, logs, run bundles, screenshots, or summaries.
- LLM/API keys are env-only and mockable in CI.
- Untrusted RON/JSON/package content validates before use.
- Path traversal blocked for package/content/save paths.
- No arbitrary script execution unless the modding script host DR has closed.
- Server authority is not bypassed by client commands.
- Logs avoid personal data and private tokens.
- Dependencies have acceptable license/provenance notes in [[references/usage-ledger]] where relevant.

### 9. Performance / 4K120 Review

Corefall's target is an ambitious sim-heavy game. Reviewers should look for performance issues early, before systems become expensive to rewrite.

Check:

- Fixed tick work has explicit budget or benchmark plan.
- Allocations in hot loops are avoided or justified.
- Events are batched or bounded where volume can explode.
- No unbounded per-frame scans of all actors/material pixels/contacts unless milestone explicitly accepts it.
- Data layout makes sense for the access pattern.
- `cf-bench` profile exists once the milestone introduces measurable work.
- Perf regressions are logged in run-bundle summary when validation commands run.

### 10. AI Control / Observability Review

Every new feature must be machine-observable and machine-drivable.

Check:

- Can `cfctl observe` see the state a human would see?
- Can `cfctl inspect` explain the selected actor/object/event/reason?
- Can `cfctl act` perform every player-facing action without OS-level screenshots/input?
- Are errors structured and visible through the control API?
- Is the UI tree exposed for player-facing UI?
- Are accessibility settings exposed in observations?
- Do run bundles include enough events to reconstruct what happened?

If a human can click it, trigger it, read it, or diagnose it, the AI control layer needs a path.

### 11. Vault / Checklist / Changelog Coherence Review

Every implementation pass must leave planning state usable for the next worker.

Check:

- [[spec/feature-completion-checklist]] rows updated with evidence and AI self-ratings.
- [[spec/prototype-roadmap]] updated only if scope/evidence/risk changed.
- [[spec/native-implementation-backlog]] updated if task cards changed.
- Corefall `CHANGELOG.md` updated.
- Corefall implementation log added.
- DR closure protocol followed when applicable.
- New player-facing features linked from relevant spec docs.
- No stale `cx`, `corefall-game`, `cortex-game`, or broken Obsidian anchors.
- No reference repo edits.

### 12. Contract Integrity Review

This pass catches "green but wrong" implementations.

Required checks:

- No parallel production paths: app, CLI, control API, scenario loader, metadata builder, run-bundle writer, and schema validator share the same core contract logic.
- No fake success: accepted commands mutate state or reject unsupported semantics with a specific error.
- Required fields reject missing/malformed values in live protocol paths.
- Run bundles and observations use loaded scenario data, active config, current binary/git state, and actual runtime path.
- Checklist rows do not hide required missing work behind "deferred", "follow-up", "reserved", "stub", "fake", "placeholder", or equivalent wording.
- Every verified bug fix includes a regression proof that would have failed before the fix.

Output a Contract Integrity Matrix:

| Contract Path | Shared Source Of Truth | Positive Proof | Negative/Adversarial Proof | Checklist Truth |
|---|---|---|---|---|

If a contract path has no negative/adversarial proof, verdict is REQUEST CHANGES.

### 13. Final Synthesis Judge

The final reviewer deduplicates and verifies all findings.

Output format:

| ID | Severity | Confidence | Pass | File/Line | Finding | Evidence | Repro/Test | Suggested Fix | Blocks? |
|---|---|---|---|---|---|---|---|---|---|

Severity rules:

| Severity | Meaning | Blocks? |
|---|---|---|
| Critical | Crash/data loss/security leak/invalid run-bundle/determinism break/server authority break. | Yes |
| High | Incorrect behavior, missing required task, missing meaningful test, broken `cfctl` surface, replay/event gap. | Yes |
| Medium | Maintainability, edge-case, performance, or documentation risk that should be fixed before milestone acceptance. | Yes |
| Low | Small verified cleanup, clarity, or correctness issue that should still be fixed before milestone acceptance. | Yes |

The judge must not pass a milestone with any unresolved verified finding at any severity unless the user explicitly approves deferring that exact finding. Low means small fix, not safe carry-forward. Medium means fix now, not later by default.

## M0-Specific Review Checklist

M0 is mostly infrastructure, so the review bar is contract accuracy and future-proofing.

| Area | What The Reviewer Must Verify |
|---|---|
| Workspace | `game/` is a Cargo workspace with the roadmap's 29 `cf-*` crates; no stale `cx-*`; no stale `corefall-game`. |
| Per-crate docs | Each crate has an `AGENTS.md` matching the canonical template or a justified equivalent. |
| Toolchain | Rust toolchain pinned; CI matrix covers Linux/macOS/Windows as planned. |
| CLI | `cf-app`, `cfctl`, and any scaffolded binaries expose the M0 flags documented in the roadmap. |
| Control API | JSON-RPC 2.0 over loopback; mandatory `schema_version`; no remote default bind. |
| Schemas | Schemas emitted under the planned `cf-control` path; schema versioning visible. |
| Fixed tick | `cf-sim-core` has deterministic 60 Hz tick stepping, pause/resume/step/run-for behavior. |
| RNG | Deterministic RNG is seeded; no gameplay use of system randomness. |
| Run bundle | Correct directory naming; required files exist; JSONL validates; summary counters make sense. |
| Panic/tracing | Panic hook emits a `system.panic` event; severity counters and tracing init exist. |
| Scenario | `content/scenarios/m0_blank.ron` parses and runs headless. |
| Accessibility surface | UI scale, high contrast, captions, reduced motion, reduced shake, reduced flash flags exist and can be observed. |
| Tests | Unit/integration tests protect behavior, not just compilation. |
| Validation | Standard validation commands were actually run and results recorded. |
| Anti-scope | No actor/equipment/AI/terrain systems snuck into M0. |
| Vault state | M0 checklist rows, implementation log, changelog, and user-approved deferrals are current. |

## Future Milestone Review Overlays

| Milestone Area | Extra Review Focus |
|---|---|
| M1 actor control | Input buffering, acceleration/friction/jump edge cases, deterministic state, `cfctl` action parity, feel tests. |
| M1.5 micro breach | Real objective pressure, enemy reactivity, run-bundle evidence, fun proof, no sterile lab-only success claim. |
| M2 terrain | Dirty regions, collision/pathfinding updates, terrain mutation events, material affordances, perf counters. |
| M3 replay | First-divergence tooling, checksum granularity, replay viewer, event schema compatibility, golden replay fixtures. |
| M4 UI/accessibility | Scale/contrast/captions/focus, non-overlap, semantic UI tree, no screenshot-only automation. |
| M5 equipment/chassis | Staged damage, armor/equipment degradation, slot rules, AI-readable metadata, item role clarity. |
| M5.5 collision | Collision matrix coverage, CCD tiers, projectile-projectile behavior, limb/body/object contacts, impulse-to-damage. |
| M5.6/M5.7 materials | Reaction order, gas/liquid/temperature edge cases, bounded active regions, affliction routing, material lab evidence. |
| M6 AI | Trust harness scenarios, reason labels, stuck recovery, doctrine/personality, fairness and replay proof. |
| M6.5 LLM mind | Async nonblocking design, schema validation, deterministic fallback, cost cap, privacy, replayed decisions. |
| M7 missions/base | Objective grammar, command core risk/reward, base power/shield/turret/sensor events, mission manifest coverage. |
| M8 editor/mods | Package validation, provenance, deterministic builds, bad package diagnostics, script-host gate. |
| M9-M12 server/MMO | Server authority, replay alignment, transport decision, persistence, anti-cheat, shard isolation, admin tools. |

## Future Split-Out Skill Files

The installed `/corefall-review` skill currently orchestrates all review passes. If reviews become too large, split the single skill into these project-local skills while keeping `/corefall-review` as the coordinator:

| Future Skill Directory | Purpose |
|---|---|
| `.claude/skills/corefall-diff-reviewer/SKILL.md` | Full diff review with changed-line focus. |
| `.claude/skills/corefall-full-code-reviewer/SKILL.md` | Full affected-code review with callers/tests/configs. |
| `.claude/skills/corefall-contract-gap-finder/SKILL.md` | Roadmap/backlog/checklist/DR requirement coverage matrix. |
| `.claude/skills/corefall-edge-case-path-hunter/SKILL.md` | Exhaustive path and boundary review. |
| `.claude/skills/corefall-test-quality-auditor/SKILL.md` | Test adequacy, tautology, edge-case, fuzz/property/E2E audit. |
| `.claude/skills/corefall-rust-safety-reviewer/SKILL.md` | Rust idiom, safety, dependency, Miri/Loom/fuzz/audit pass. |
| `.claude/skills/corefall-determinism-replay-reviewer/SKILL.md` | Fixed-tick, RNG, event order, checksum, replay/run-bundle review. |
| `.claude/skills/corefall-security-supply-chain-reviewer/SKILL.md` | OWASP-style, control/server/mod/LLM/dependency/security pass. |
| `.claude/skills/corefall-performance-reviewer/SKILL.md` | 4K120 and sim/server scalability review. |
| `.claude/skills/corefall-ai-control-observability-reviewer/SKILL.md` | `cfctl` eyes/ears/hands parity review. |
| `.claude/skills/corefall-vault-coherence-reviewer/SKILL.md` | Checklist, roadmap, changelog, implementation log, DR coherence. |
| `.claude/skills/corefall-review-synthesis-judge/SKILL.md` | Dedupes, verifies, ranks, and decides blockers. |

## Review Report Template

```md
# Corefall Review Report - <Milestone/Feature>

## Scope
- Repo:
- Commit/range:
- Milestone:
- Docs read:
- Validation commands inspected:
- Run bundles inspected:

## Decision
- Verdict: PASS / REQUEST CHANGES / NOT REVIEWABLE
- Blocking findings: <count>
- High findings: <count>
- Medium findings: <count>
- Low findings: <count>

## Contract Coverage
| Requirement | Source | Evidence | Status | Gap |
|---|---|---|---|---|

## Findings
| ID | Severity | Confidence | Pass | File/Line | Finding | Evidence | Repro/Test | Suggested Fix | Blocks? |
|---|---|---|---|---|---|---|---|---|---|

## Contract Integrity Matrix
| Contract Path | Shared Source Of Truth | Positive Proof | Negative/Adversarial Proof | Checklist Truth |
|---|---|---|---|---|

## User-Approved Deferrals
| Finding ID | User approval | Reason | Owner | Next checkpoint | Evidence path |
|---|---|---|---|---|---|

If this table is empty and any verified finding remains unresolved, verdict must be REQUEST CHANGES.

## Test Quality
| Test | Protects Behavior? | Would Fail For Bug? | Gap |
|---|---|---|---|

## Determinism / Replay / Run-Bundle
| Check | Evidence | Status | Gap |
|---|---|---|---|

## `cfctl` Observability
| Surface | Observe | Inspect | Act | Gap |
|---|---|---|---|---|

## Vault / Checklist / Changelog
| Item | Status | Evidence |
|---|---|---|

## Accepted Known Issues
| Issue | Reason Accepted | Follow-up |
|---|---|---|

## Next Fix Prompt
Pasteable prompt for the implementing agent:
```
Fix the blocking findings in <report path>. Preserve passing validation. Add tests proving each fix. Update checklist/changelog/run-bundle evidence.
```
```

## Source Synthesis

| # | Source | What It Contributes | Corefall Use |
|---|---|---|---|
| 1 | [Claude Code Review docs](https://docs.anthropic.com/en/docs/claude-code/code-review) | Multi-agent PR review, severity tags, full-codebase context, and verification patterns. | Model the Corefall review pipeline as specialized agents plus final verification. |
| 2 | [Cloudflare - Orchestrating AI Code Review at scale](https://blog.cloudflare.com/ai-code-review/) | CI-native multi-reviewer orchestration, coordinator, risk tiers, structured findings, guardrails, AGENTS.md reviewer. | Use specialist reviewers and a synthesis judge; add an AGENTS/vault-coherence reviewer. |
| 3 | [Augment - How we built a high-quality AI code review agent](https://www.augmentcode.com/blog/how-we-built-high-quality-ai-code-review-agent) | Context retrieval, prompt philosophy, guardrails, precision/recall evals. | Tune review passes for high recall, then verify to reduce false positives. |
| 4 | [O'Reilly - AI Code Review Only Catches Half of Your Bugs](https://www.oreilly.com/radar/ai-code-review-only-catches-half-of-your-bugs/) | Intent/requirements bugs require behavioral contracts, not just code reading. | Add spec contract gap review against roadmap/backlog/DRs. |
| 5 | [Tenki - Reviewing AI Generated Code Checklist](https://www.tenki.cloud/blog/reviewing-ai-generated-code-checklist) | Seven AI-code failure modes: hallucinated APIs, tautological tests, cargo-cult patterns, overengineering, edge cases, wrong business logic, stale patterns. | Make those failure modes explicit review categories. |
| 6 | [Google - What to look for in a code review](https://google.github.io/eng-practices/review/reviewer/looking-for.html) | Design, functionality, complexity, tests, naming, comments, style, consistency, docs, every line, context. | Base review order and "read every human-written line" rule. |
| 7 | [Google - The Standard of Code Review](https://google.github.io/eng-practices/review/reviewer/standard.html) | Code health should improve or at least not degrade. | Use "does this improve Corefall's long-term maintainability?" as a blocker lens. |
| 8 | [OWASP Secure Code Review Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Secure_Code_Review_Cheat_Sheet.html) | Manual security review catches business-logic, auth, data-flow, crypto, and race-condition bugs. | Security reviewer checklist for control/server/mod/LLM surfaces. |
| 9 | [OWASP Code Review Guide](https://owasp.org/www-project-code-review-guide/) | Broader secure review methodology. | Backstop for security-sensitive milestones. |
| 10 | [OWASP Secure Coding Practices Checklist](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/stable-en/02-checklist/05-checklist) | Secure coding checklist categories. | Source for future security skill sub-checklists. |
| 11 | [Terminal Skills - code-reviewer](https://terminalskills.io/skills/code-reviewer) | Practical SKILL.md structure with severity-ranked correctness/security/performance/reliability/testing review. | Template for a general Corefall diff reviewer. |
| 12 | [BMAD Edge Case Hunter skill](https://github.com/bmad-code-org/BMAD-METHOD/blob/819d373e/src/core-skills/bmad-review-edge-case-hunter/SKILL.md) | Exhaustive path enumeration with strict JSON output and no editorializing. | Template for `corefall-edge-case-path-hunter`. |
| 13 | [Tessl - Best Agent Skills for AI Code Review](https://tessl.io/blog/best-agent-skills-for-ai-code-review-8-evaluated-skills-for-dev-workflows/) | Reviews skill categories and checklist-based PR reviewers. | Use skills as composable review protocols rather than one prompt. |
| 14 | [VoltAgent - awesome-agent-skills](https://github.com/VoltAgent/awesome-agent-skills) | Curated agent skills including multi-agent code review and Playwright/testing skills. | Source of future skill examples to adapt. |
| 15 | [SkillsSafe - Best AI Code Review Skills 2026](https://skillsafe.ai/blog/best-ai-code-review-skills-2026/) | Evaluates skills by depth, actionability, structure; highlights parallel-agent and code-review-expert patterns. | Scoring criteria for our own Corefall review skills. |
| 16 | [LLMBase code-review skill](https://llmbase.ai/skills/ahgraber/code-review/) | Triage, diff review, graph/caller/test coverage ideas, parallel subagent review. | Use preflight diff stats, caller search, and blast-radius summary. |
| 17 | [AI Agentivo code-review skill](https://aiagentivo.com/skills/yeachan-heo-oh-my-codex-code-review) | Delegated thorough review, external model consultation, severity framework. | Use external second-opinion validation for high-risk reviews. |
| 18 | [ExplainX code-review-expert](https://explainx.ai/skills/sanyuan0704/code-review-expert/code-review-expert) | SOLID, architecture smells, removal candidates, security/reliability scan. | Add dead-code/removal and simplification pass. |
| 19 | [ExplainX code-review-quality](https://explainx.ai/skills/proffesor-for-testing/agentic-qe/code-review-quality) | Blocker/major/minor/suggestion priorities; focus on bugs/security/testability/maintainability. | Severity calibration and no-style-nit rule. |
| 20 | [Agent Skills code-review checklist](https://agent-skills.md/skills/skillcreatorai/Ai-Agent-Skills/code-review) | Security/performance/quality/testing categories and output format. | Useful simple baseline for junior agents. |
| 21 | [Skill4Agent code-review](https://skill4agent.com/en/skill/jwynia-agent-skills/code-review) | Review size limits, quality pyramid, code smell thresholds, review readiness. | Add large-diff batching and anti-nitpicking guardrails. |
| 22 | [Hamy - 9 parallel AI agents that review my code](https://hamy.xyz/blog/2026-02_code-reviews-claude-subagents) | Parallel specialists: style, architecture, tests, dependency/deployment, simplification. | Supports Corefall specialist-reviewer stack. |
| 23 | [Sean Goedecke - AI agents and code review](https://www.seangoedecke.com/ai-agents-and-code-review/) | Best review asks whether this is the right approach, not just whether the code compiles. | Add full affected-code/architecture review after diff review. |
| 24 | [Git AutoReview Deep Review](https://gitautoreview.com/deep-review) | Deep review explores full codebase, data flow, tests, linter; catches diff-invisible issues. | Separate full-code review from diff review. |
| 25 | [diffray multi-agent code review](https://diffray.ai/multi-agent-code-review/) | Specialist agents, dependency tracing, verification, dedupe. | Synthesis judge and domain-specific agents. |
| 26 | [Greptile Agent](https://www.greptile.com/agent) | Graph-based codebase context, custom rules, test generation. | Future graph/call-chain review ideas. |
| 27 | [Developer Toolkit - Cursor AI code review](https://developertoolkit.ai/en/cursor-ide/lessons/code-review/) | Local pre-push review, BugBot rules, CLI workflows, project rules. | Create local preflight review prompt and future CI hook. |
| 28 | [DigitalOcean - AI Code Review Tools](https://www.digitalocean.com/resources/articles/ai-code-review-tools) | AI review limitations and human oversight for architecture/business logic. | Keep human/user override and final decision outside the AI reviewer. |
| 29 | [Rust Book - Useful Development Tools](https://doc.rust-lang.org/book/appendix-04-useful-development-tools.html) | `rustfmt`, `cargo fix`, Clippy, rust-analyzer. | Baseline Rust tool validation. |
| 30 | [ANSSI Secure Rust Guidelines checklist](https://anssi-fr.github.io/rust-guide/checklist.html) | Toolchain, Cargo.lock, dependency vetting, error handling, panic limits, unsafe/FFI/memory rules. | Rust safety reviewer checklist. |
| 31 | [Miri](https://rust.googlesource.com/miri/) | Undefined behavior detection, data races, uninitialized data, leaks, deterministic isolation, cross-target interpretation. | Use for unsafe/pointer/concurrency-critical crates. |
| 32 | [Proptest book](https://proptest-rs.github.io/proptest/proptest/) | Property-based testing framework. | Use for schemas, parsers, material reactions, event invariants. |
| 33 | [Rust Fuzz Book - cargo-fuzz](https://rust-fuzz.github.io/book/cargo-fuzz.html) | Recommended Rust fuzzing tool using libFuzzer. | Use for package loaders, protocol decoders, save/replay parsers. |
| 34 | [cargo-nextest](https://nexte.st/) | Faster test runner, per-test isolation, CI, record/replay/rerun, stress testing. | Use once workspace grows and CI needs speed/isolation. |
| 35 | [cargo-deny](https://embarkstudios.github.io/cargo-deny/) | Dependency graph linting for licenses/advisories/bans/sources. | Supply-chain and license/reuse checks. |
| 36 | [Loom docs](https://docs.rs/loom/latest/loom/) | Deterministic exploration of concurrent executions. | Use for server/control/concurrency primitives. |
| 37 | [Gaffer - Fix Your Timestep](https://gafferongames.com/post/fix_your_timestep/) | Fixed timestep, accumulator, interpolation, spiral-of-death avoidance. | Review sim loop and perf headroom. |
| 38 | [Gaffer - Deterministic Lockstep](https://gafferongames.com/post/deterministic_lockstep/) | Bit-level determinism, input networking, playout buffers, UDP input redundancy, checksum framing. | Review replay/networking architecture and determinism assumptions. |
| 39 | [SnapNet - Lockstep](https://www.snapnet.dev/blog/netcode-architectures-part-1-lockstep/) | Lockstep prerequisites: strict determinism, fixed tick, checksums, limitations. | M9-M12 networking review overlay. |
| 40 | [YellowAfterlife - Preparing your game for deterministic netcode](https://yal.cc/preparing-your-game-for-deterministic-netcode/) | Replay preparation, variable frame decoupling, save/load for rollback, desync caveats. | Review replay readiness before netcode. |
| 41 | [Bugnet - Debug Desync in Deterministic Lockstep Games](https://bugnet.io/blog/how-to-debug-desync-in-deterministic-lockstep-games) | Per-tick checksums, subsystem hash breakdowns, input hashes, replay divergence tools. | Required first-divergence review criteria. |
| 42 | [Stray Pixels - Wizard with a Gun State Divergences](https://straypixels.net/wwag-divergence/) | Checksum tracing, log drilling, replay-based divergence reproduction, common divergence causes. | Add checksum trace and divergence writeups to review expectations. |

## Open Follow-Ups

| Follow-Up | Why |
|---|---|
| Split `/corefall-review` into specialist skills if review outputs get too large. | The installed skill is intentionally one command for now; split later only when the single-skill workflow becomes unwieldy. |
| Add review report storage path in Corefall, probably `docs/reviews/<date>-<milestone>.md`. | Review findings should compound and be searchable. |
| Add checklist rows for "review complete" per milestone. | The existing checklist tracks implementation; review completion should be equally visible. |
| Add `cargo deny`, `cargo audit`, `miri`, `loom`, `proptest`, `cargo-fuzz`, and `nextest` progressively to the roadmap/backlog when their target milestones arrive. | These should not all be forced into M0, but they should be planned as gates when relevant code exists. |
