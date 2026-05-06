# Corefall Agent Guide

This file is for AI implementation agents working in `~/projects/corefall`.

## Source Of Truth

This is the implementation repo. Do not duplicate the planning vault here.

The canonical research and planning vault is:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault
```

Root planning files live here:

```text
/Users/erol/projects/cortex-command-repos-all/VAULT_PLAN.md
/Users/erol/projects/cortex-command-repos-all/DIRECTORY.md
/Users/erol/projects/cortex-command-repos-all/AGENTS.md
/Users/erol/projects/cortex-command-repos-all/GAME_DESCRIPTION_FOR_FRIEND.md
```

Before implementing a milestone, read the canonical vault directly. If any path below is missing, search the canonical vault with `rg --files` and ask the user before making architecture-changing assumptions.

## Milestone Authority Stack

For milestone scope and acceptance, documents are not peers. Use this authority order every time:

1. The user's current assignment or explicit correction.
2. The assigned milestone section in `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`.
3. The assigned milestone task cards in `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md`.
4. DRs/spec files that the roadmap or backlog explicitly links for that milestone.
5. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md` as tracking and evidence only.
6. Implementation logs, `CHANGELOG.md`, run bundles, notes, review reports, and handoff summaries as evidence only.

If a lower-authority file says a roadmap/backlog requirement is deferred, partial, unnecessary, or complete, that claim is invalid unless the roadmap/backlog was changed first with explicit user approval. Do not use evidence files to redefine milestone scope.

When files conflict:

- Roadmap/backlog wins for what must be built.
- Linked DR/spec wins only for the detailed shape of an item already in roadmap/backlog scope.
- Checklist/log/changelog/run-bundle claims must be corrected to match the roadmap/backlog, not the other way around.
- If the roadmap and backlog disagree on a material requirement, stop and ask the user before implementing or marking completion.

## Milestone Acceptance Gate

A milestone is complete only when every roadmap done-criterion and every backlog task card for that milestone is PASS with evidence. Every handoff, review, or completion report must include an ID-by-ID acceptance matrix:

```text
M<id>-001: PASS/FAIL - evidence
M<id>-002: PASS/FAIL - evidence
...
```

Any `FAIL`, `PARTIAL`, `DEFERRED`, `READY_FOR_HUMAN`, or "lands later" item in roadmap/backlog scope means the milestone is not complete. The only exception is when the roadmap/backlog itself marks the item as human-gated or future scope.

No verified review finding may be carried forward by default. Low, Medium, High, and Blocker findings must all be fixed before the milestone is called complete. The only exception is an explicit user-approved deferral for that exact finding; the deferral must record the issue ID, reason, owner, next milestone/checkpoint, and evidence path in the implementation log, `CHANGELOG.md`, checklist row, and roadmap/DR docs if scope or risk changed.

Do not summarize a milestone as complete from prose. Completion is the acceptance matrix plus validation evidence.

## Contract Integrity Gate

Passing commands are not enough. Every milestone must prove that implementation behavior matches the roadmap/backlog contract through the same code paths users, tools, CI, and future milestones will rely on.

Hard rules:

- No parallel production paths. `cf-app`, `cfctl`, `cf-control`, `cf-replay`, scenario loading, metadata generation, schema validation, and run-bundle writing must share the same core contract code whenever they claim the same behavior. If a helper bypasses production behavior, name it `*_test_*`, keep it test-only where possible, and never call it from binaries.
- No fake success. A command returning `accepted`, `ok`, or `PASS` must either perform the requested state change or reject the request with a specific error. Ignoring unsupported fields, silently defaulting malformed params, or accepting no-op command semantics is a milestone failure.
- Required fields are required. If a spec says a field is mandatory, missing or malformed input must fail in both tests and live `cfctl`/JSON-RPC validation. Do not treat absence as compatible unless the spec explicitly allows it.
- Evidence must be source-truthful. Run bundles, summaries, observations, and checklist rows must reflect the loaded scenario, active config, current binary/git state, and actual runtime path. Hardcoded metadata must not masquerade as loaded manifest/build data.
- Checklist rows cannot launder deferrals. If a checked row's notes contain `deferred`, `follow-up`, `not implemented`, `reserved`, `fake`, `stub`, `placeholder`, or equivalent wording for required roadmap/backlog scope, the row is not complete unless the user explicitly approved that exact deferral.
- Every reviewed bug needs a regression proof. For each verified finding, add a test or validation command that would have failed before the fix and now passes. If a test is impossible, document why and provide the strongest equivalent live validation.

Every milestone closeout must include a **Contract Integrity Matrix**:

```text
Contract path: <cf-app/cfctl/server/replay/etc>
Shared source of truth: <module/function/schema>
Positive proof: <test/command/bundle>
Negative/adversarial proof: <test/command/error>
Checklist truth: <rows updated, no hidden deferrals>
```

If a contract path has no negative/adversarial proof, do not mark it complete.

## No-Compromise Performance Defaults

Corefall is a no-compromise performance and feel project. Do not turn roadmap defaults into hardcoded ceilings.

Performance-sensitive values must be configuration-driven unless the roadmap/backlog explicitly marks them as fixed invariants. This includes:

- Sim tick rate.
- Render cadence / frame pacing.
- Input sampling cadence.
- Physics substeps / solver iteration counts.
- Network send, receive, rollback, snapshot, and interpolation rates.
- Replay checksum cadence and snapshot cadence.
- Asset streaming budgets, worker counts, memory budgets, and quality tiers.

If a milestone names a default or validation value, implement it as a default, not as an architectural constant. Example: `60 Hz default; 120 Hz option` means the engine must accept tick-rate configuration and must not contain gameplay/control/replay/render assumptions that only work at 60 Hz.

Tick-rate policy until the canonical roadmap says otherwise:

- M0 may keep the roadmap's 60 Hz compatibility/default validation path.
- M0 must preserve and validate a 120 Hz path wherever fixed-tick sim behavior is implemented.
- 128 Hz is a candidate for later evidence-based evaluation, especially for network/server cadence, but must not be blocked by M0 architecture.
- Run bundles and observations must record the configured tick rate.
- Tests for fixed-tick systems must cover more than one tick rate whenever the system is tick-rate-sensitive.

Hardcoded performance-sensitive constants are a milestone failure unless they are named constants backed by roadmap/backlog text and exposed through the relevant config surface. If an agent believes a value should be fixed for design reasons, it must be recorded as an explicit roadmap/backlog decision before being treated as fixed.

## Short Assignment Expansion

If the user says something short like:

```text
Implement M0 from /Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md
```

or:

```text
Implement M1
```

treat that as a complete milestone assignment. Do not ask the user for a larger prompt. Expand the short assignment using this `AGENTS.md`, the canonical roadmap, the native backlog, the feature checklist, the AI-coder reading list, and the milestone's linked DRs/specs.

For any milestone assignment, the worker must:

1. Read the mandatory docs below.
2. Read the assigned milestone section in the roadmap.
3. Read the assigned milestone task cards in the native backlog.
4. Read the assigned milestone rows in the feature checklist as evidence/tracking only, not scope authority.
5. Run the Open Decision Gates pre-check before locking any open decision.
6. Implement all agent-completable task cards for the milestone.
7. Run Standard Validation plus milestone-specific validation.
8. Produce run-bundle evidence under `prototype_runs/native/`.
9. Update the canonical vault roadmap/checklist and repo-local changelog.
10. Produce the ID-by-ID acceptance matrix from the Milestone Acceptance Gate.
11. Confirm no performance-sensitive roadmap default was hardcoded as an architectural ceiling.
12. Leave both repos commit-ready, and commit only when the user asks or when the active assignment explicitly includes committing.

## Mandatory Read Order Before Any Milestone

Read these in order before implementing a roadmap milestone:

1. `/Users/erol/projects/cortex-command-repos-all/AGENTS.md`
2. `/Users/erol/projects/cortex-command-repos-all/VAULT_PLAN.md`
3. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/index.md` (Planning Docs panel)
4. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-coder-reading-list.md`
5. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/authoritative-game-spec-v0.md`
6. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`
7. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md`
8. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md`
9. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-control-observability-layer.md`
10. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/references/prototype-run-bundle-schema.md`
11. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/decisions/index.md`
12. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/dashboards/decision-tracker.md`

For milestone-specific docs, use the tables in:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-coder-reading-list.md
```

If the canonical `spec/ai-coder-reading-list.md` disagrees with the list above, the canonical list wins. Propose a row update there in the same pass and commit the AGENTS.md edit alongside it.

## Review And Bug-Hunt Skill

Claude Code has a project-local review skill installed at:

```text
.claude/skills/corefall-review/SKILL.md
```

Use `/corefall-review <milestone-or-range>` for deep milestone reviews, bug hunts, gap finding, and pre-merge audits. The skill runs a separate diff review, full affected-code review, contract gap review, edge-case hunt, test audit, Rust/determinism/security/performance review, `cfctl` observability review, and vault coherence pass.

Repo-specific review behavior is pinned in the skill entrypoint at `.claude/skills/corefall-review/SKILL.md`. If the user asks "review M0", "bug hunt this", "find misses", or "is this done?", treat that as enough context to invoke the review skill and review the current working tree or supplied commit/range. If the review finds any verified issue at any severity, the next action is a fix/stabilization pass, not milestone acceptance, unless the user explicitly approves deferring that exact issue.

## Repository Layout

The native game workspace lives at the corefall repo's `game/` directory. This matches the canonical roadmap's `Repository Layout` section in `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`; no path mapping is needed.

| Canonical (in roadmap) | This repo |
|---|---|
| `game/` (workspace root) | `game/` |
| `game/Cargo.toml` | `game/Cargo.toml` |
| `game/crates/cf-app` ... `cf-server` | `game/crates/cf-app` ... `cf-server` |
| `game/content/` | `game/content/` |
| `game/mods/` | `game/mods/` |
| `game/scripts/cfctl/` | `game/scripts/cfctl/` |
| `game/assets/` | `game/assets/` |
| `game/tests/` | `game/tests/` |
| `game/tools/` | `game/tools/` |
| Run-bundle root | `prototype_runs/native/` (at corefall repo root) |
| Implementation logs | `docs/implementation-log/` (at corefall repo root) |
| Repo-only changelog | `CHANGELOG.md` (at corefall repo root) |

The crate name prefix is `cf-` throughout the implementation repo and canonical vault. Use `cargo run -p cf-<name>` for workspace binaries and keep new crates on the same prefix unless a future DR explicitly changes the naming convention.

Do not put source code in the planning vault. Do not copy the whole vault into this repo. Implementation notes and milestone evidence belong in this repo under `docs/implementation-log/` and `prototype_runs/native/`.

## Per-Crate AGENTS.md

Once `game/` is bootstrapped as a workspace with crates, every crate ships its own `AGENTS.md` per the `Per-Crate AGENTS.md Template` section in `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`. The crate's `AGENTS.md` is the boundary contract:

- Owns
- Public API Boundary
- Does NOT Own
- Test Surface
- Cross-Crate Contracts
- Common Pitfalls
- Source Trail

M0's task cards include creating the first set of per-crate AGENTS.md files alongside the workspace scaffold.

## Standard Validation

Run these from `game/` unless a task card narrows the set:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cfctl -- observe --once
python3 /Users/erol/projects/cortex-command-repos-all/research_tools/prototype_run_check.py /Users/erol/projects/corefall/prototype_runs/native/<run_id>
```

Milestones with gameplay/tool UI also require a scripted E2E command and a screenshot/capture artifact listed in `summary.json.artifacts`.

`cfctl` lives at `game/crates/cfctl/`. Invoke as `cargo run -p cfctl -- <subcommand>` during M0..M1; once installed or added to PATH, `cfctl <subcommand>` is shorthand. The full CLI surface is pinned in the canonical `CLI Reference` section of `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`.

## Run-Bundle Naming

Run bundles live under `prototype_runs/native/` at the corefall repo root. Naming follows the canonical `Run-Bundle Naming Convention` section of `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`.

```text
prototype_runs/native/<milestone>_<UTC ISO-8601 with hyphens>_<short_hash>/
```

Example: `prototype_runs/native/m0_2026-05-04T22-30-00Z_a1b2c3d4/`.

Each bundle contains `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`, and optional `screenshots/` / `captures/`. Validate it with the run-bundle checker named in Standard Validation.

## Open Decision Gates

Do not silently assume an open decision is settled.

If a milestone touches an OPEN decision record or topic-level open decision:

- Confirm the current lean from the canonical vault per the `Open Decision Gates Protocol` section of `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`.
- Implement only what the milestone allows.
- If the lean is contested or would materially change architecture, stop and ask the user through the active agent's available user-input/chat mechanism.
- When prototype evidence closes a DR, update the canonical vault in the same pass (DR file + decisions/index + decision-tracker + research-readiness + a dated research-log note) or explicitly report that the vault update is still pending.

## Eyes, Ears, Hands Rule

Every player-facing surface must be controllable and observable through the planned `cf-control` / `cfctl` layer unless explicitly marked human-only with a reason.

The rule: any pixel a human can interact with on screen, the AI worker must be able to drive through `cfctl`. Screenshot-only testing is not enough. A milestone is incomplete if AI agents cannot inspect and drive the new gameplay/UI surface through structured commands.

See `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-control-observability-layer.md` for the full observe/inspect/act surface; every new player-facing surface must extend it.

## Completion Contract

After implementing any feature, task card, side-track item, or milestone, an agent must leave the project in a state where another agent can see exactly what changed and what remains.

Required completion actions:

1. Update code and tests in `game/`.
2. Run the Standard Validation commands (above) plus any milestone-specific validation from the assigned roadmap/backlog section.
3. Emit or update run-bundle evidence when the task includes runnable behavior.
4. Update `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md` rows that correspond to the completed work. Fill evidence, commands, run-bundle paths, and AI self-ratings; leave human rating fields blank unless the user provides them.
5. Update `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md` if status, scope, dependencies, evidence, commands, risks, or follow-up work changed.
6. Add or update the milestone implementation note under `docs/implementation-log/`.
7. Add a concise repo-local entry to `CHANGELOG.md`.
8. If the milestone closes a DR, update the DR file + `decisions/index.md` + `dashboards/decision-tracker.md` + `dashboards/research-readiness.md` + a dated `research-log/` note in the same pass.
9. Verify every new player-facing surface is reachable from `cfctl` with assert/inspect coverage.
10. Report any vault updates that could not be completed, with exact file paths and reasons.
11. Report the milestone acceptance matrix with every roadmap done-criterion and every backlog task card marked PASS or FAIL.
12. Report the performance/config audit for the milestone: which tick rates, frame rates, solver rates, network rates, replay cadences, and quality budgets are configurable; which values were validated; and why any fixed constant is allowed.
13. Run `/corefall-review <milestone>` from `/Users/erol/projects/corefall`, fix every verified finding at every severity, and rerun `/corefall-review <milestone>` until the verdict is `Accept`. If the user explicitly defers a finding, record the deferral ID, reason, owner, next checkpoint, and evidence path.
14. Report the Contract Integrity Matrix proving shared code paths, required-field rejection, fake-success absence, source-truthful evidence, and checklist truth.

Do not mark work complete if the checklist/roadmap updates are skipped. Do not mark work complete if any roadmap done-criterion or backlog task card is deferred, partial, or only documented as future work. Do not mark work complete until `/corefall-review <milestone>` has been run and rerun to `Accept`, unless every remaining verified finding has explicit user-approved deferral evidence. Do not mark work complete if the Contract Integrity Matrix is missing positive and negative/adversarial proof for each contract path. Do not mark work complete if a performance-sensitive value is hardcoded without roadmap/backlog authority and a config-path explanation. If a task genuinely does not affect the roadmap, record "roadmap update not needed" in the implementation log and explain why.

## Reference Repos And Reuse

Reference repos under `/Users/erol/projects/cortex-command-repos-all` are read-only unless the user explicitly says otherwise.

Do not copy code/assets from external projects into Corefall without logging the source and license posture in the canonical usage ledger:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/references/usage-ledger.md
```

For now, reuse/licensing guidance is not a blocker for private research or prototypes, but provenance must be tracked so release decisions are clean later.

## Implementation Posture

Build the best game and best UX first. Planning docs contain safety, reuse, scope, and launch-boundary guidance, but they should not be misread as bans on research, prototyping, or learning from other games.

Current direction:

- Strict 2D side-view.
- Rust + Bevy/wgpu hybrid + custom core crates.
- Desktop-first: Windows, Linux, macOS; Steam Deck floor.
- Solo-first, but architecture supports LAN, online co-op, PvP arenas, and persistent shards.
- Full collision as a core feel pillar.
- Systemic material simulation as a core feel pillar.
- Deep combat-base, not full colony sim.
- Command core rooted/uprooted/avatar tradeoff.
- AI trust and observability are product features.

## Git Hygiene

- Trunk: `main`. Direct commits to `main` are allowed for solo prototyping; cut a feature branch (`m<id>/<short-name>`, e.g., `m0/workspace-scaffold`) when the change is large or risky.
- Commit subject format: `<milestone-id>: <imperative summary>`. Examples: `M0: scaffold cargo workspace`, `M5.6: add reaction table priority resolution`. Body explains why-not-what.
- Run Standard Validation before any commit that touches code.
- Never include vault file paths in commit subjects unless the commit is vault-only.
- Do not push directly to `main` without a local Standard Validation pass.

## Cursor Bugbot Loop

The corefall GitHub repo has [Cursor Bugbot](https://cursor.com/docs/bugbot) installed as a GitHub App. Bugbot reviews every PR push and is **not** the same as the project-local `corefall-review` skill. Treat its findings/autofixes as advisory, not authoritative.

### How the loop runs

Every time a commit lands on a PR branch:

1. Bugbot review fires automatically.
2. If Bugbot finds an issue, it produces an **autofix commit** authored as `Cursor Agent <cursoragent@cursor.com>` and pushes it directly to the PR branch.
3. The autofix triggers another Bugbot review.
4. Bugbot autofix loops up to **3 times** per PR push.
5. After the 3 autofix iterations, Bugbot produces findings and waits for the human/agent to react.
6. **Any human/agent push to the PR branch re-triggers the full 3-iteration autofix loop** from step 1.

The loop runs in parallel with the GitHub Actions CI matrix. Bugbot can produce autofix commits **even while CI is still running** on an earlier commit. The matrix may already be a few commits behind by the time you look at it.

### Required behavior when reviewing a PR with Bugbot

1. **Do not push to the PR branch while Bugbot loops are still running.** Every push restarts the 3-iteration cycle and adds more Cursor Agent commits to evaluate. Wait for the user to confirm Bugbot + CI are settled before pushing fixes. The user will signal when the loops are done.
2. **Pull the branch and inspect every Cursor Agent commit since the last human commit, one by one.** For each:
   - Read the diff against the actual codebase, not just the commit message.
   - Cross-check Bugbot's stated root cause against the real failing CI step (read CI logs, not just the Bugbot summary).
   - Decide: is this a real bug? Is the fix actually addressing the right cause? Does the fix introduce a regression?
3. **Revert wrong autofixes with `git revert <sha>`**, not by force-pushing over them. Use a revert commit message that explains *why* the autofix was wrong (false positive, wrong RCA, masks a deeper issue, breaks something else). This preserves the audit trail of what Bugbot proposed and why it was rejected.
4. **For Bugbot findings that are false positives**, leave an inline PR comment on the file/line that Bugbot flagged, explaining why it's not a real bug. This prevents Bugbot from re-flagging the same finding on subsequent runs.
5. **Only push real fixes.** Every push triggers another 3-iteration Bugbot cycle. Batch real fixes into a single commit when possible.
6. **Real CI failures take precedence over Bugbot diagnoses.** Read the actual GitHub Actions log for the failing step. Bugbot tends to surface plausible but secondary issues that are masked by an earlier step failing first.

### Failure mode the rule prevents

On PR #1 (`m0-engine-bootstrap`, Madreag/corefall, 5/5/2026):

- The Windows CI job failed at `cargo fmt --all -- --check` with "Incorrect newline style" on every `.rs` file. Root cause: no `.gitattributes` in the repo, so `actions/checkout@v4` honored git's default `core.autocrlf=true` on Windows runners and rewrote LF → CRLF on checkout, which violated `rustfmt.toml`'s `newline_style = "Unix"`.
- Bugbot diagnosed the Windows failure as **"Windows bundle validation fails"** and autofixed `python3` → `python` in the run-bundle validation step (because `actions/setup-python@v5` only guarantees `python` on Windows).
- The autofix was a **valid forward-looking fix** but **not the cause of the current failure** — `cargo fmt` fired before the validation step, so the validation step never ran on Windows. Bugbot surfaced a real-but-secondary issue and advertised it as the fix.
- The agent (correctly) read the actual CI log, identified the line-ending root cause, kept Bugbot's autofix commit (it was right about something), and added `.gitattributes` on top to fix the actual blocker.

If the agent had blindly trusted Bugbot's diagnosis without reading the CI log, it would have merged the autofix expecting CI to pass — and the next push would have failed Windows again at the same `cargo fmt` step.

### Cursor Agent commit signature

Autofix commits are authored as:

```text
Author: Cursor Agent <cursoragent@cursor.com>
```

Search for this signature when auditing recent PR history. These are NOT human commits and NOT `corefall-review` skill commits. They come from the GitHub App and need explicit human/agent review before they're trusted.

## Secrets Posture

- Never commit API keys, `.env` files, signing keys, or LLM provider tokens.
- Use environment variables for any secret per `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/hybrid-llm-ai-plan.md` `MindProviderConfig.api_key_env`.
- The `.gitignore` already excludes `.env`, `.env.*` (with `!.env.example` exception). Do not weaken this without an explicit user request.
- LLM live providers are cargo-feature-gated and never required for any test. CI uses the deterministic mock provider only.

## Do Not

- Don't write source code under `cortext_command_vault/`. The vault is planning, not implementation.
- Don't edit canonical reference repos under `/Users/erol/projects/cortex-command-repos-all/{Cortex-Command-*,comparables_repos/*}` unless the user explicitly says so.
- Don't use `rand::thread_rng()` inside sim crates (`cf-sim-core`, `cf-physics`, `cf-material`, `cf-ai`, ...). Sim RNG must be seeded and recorded per the manifest.
- Don't use `println!` in production code. Use `tracing` per the canonical `Logging, Tracing, And Error Policy` section of `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`.
- Don't `unwrap()` on user-controllable inputs.
- Don't skip the Open Decision Gates pre-check before assigning a milestone.
- Don't commit API keys, `.env` files, signing keys, or LLM provider tokens.
- Don't push directly to `main` without local Standard Validation.
- Don't mark work complete if the canonical checklist/roadmap updates are skipped.
- Don't create root review instruction/report files. Standing review rules live in `.claude/skills/corefall-review/SKILL.md`; review reports belong under `docs/reviews/`.
- Don't add cloud-save dependencies during T-SAVE work; cloud-save backend decision is post-launch.
- Don't introduce a UI surface without a matching `cf-control` / `cfctl` path. Eyes/ears/hands rule.

## Starting Point

Unless the user assigns a different target, start with:

1. M0 - Engine Bootstrap
2. M1 - Actor Controller And Sim Core
3. M1.5 - Micro Breach Fun Slice

Do not skip M1.5. It exists because the actor-feel lab alone was too sterile; the project needs early fun evidence before deeper systems attach.
