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

## Repository Layout

The native game workspace lives at the corefall repo's `corefall-game/` directory. This matches the canonical roadmap's [Repository Layout](https://.../prototype-roadmap.md#repository-layout) name; no path mapping is needed.

| Canonical (in roadmap) | This repo |
|---|---|
| `corefall-game/` (workspace root) | `corefall-game/` |
| `corefall-game/Cargo.toml` | `corefall-game/Cargo.toml` |
| `corefall-game/crates/cx-app` ... `cx-server` | `corefall-game/crates/cx-app` ... `cx-server` |
| `corefall-game/content/` | `corefall-game/content/` |
| `corefall-game/mods/` | `corefall-game/mods/` |
| `corefall-game/scripts/cxctl/` | `corefall-game/scripts/cxctl/` |
| `corefall-game/assets/` | `corefall-game/assets/` |
| `corefall-game/tests/` | `corefall-game/tests/` |
| `corefall-game/tools/` | `corefall-game/tools/` |
| Run-bundle root | `prototype_runs/native/` (at corefall repo root) |
| Implementation logs | `docs/implementation-log/` (at corefall repo root) |
| Repo-only changelog | `CHANGELOG.md` (at corefall repo root) |

The crate name prefix `cx-` is shorthand for the workspace; it is preserved across the rename for stability of `cargo run -p cx-<name>` invocations and to keep the existing AGENTS.md / decision records / task cards valid. If the user later asks to rename the prefix to `cf-`, that is a separate workspace-wide migration with its own DR.

Do not put source code in the planning vault. Do not copy the whole vault into this repo. Implementation notes and milestone evidence belong in this repo under `docs/implementation-log/` and `prototype_runs/native/`.

## Per-Crate AGENTS.md

Once `corefall-game/` is bootstrapped as a workspace with crates, every crate ships its own `AGENTS.md` per the [Per-Crate AGENTS.md Template](https://.../prototype-roadmap.md#per-crate-agentsmd-template) in the canonical roadmap. The crate's `AGENTS.md` is the boundary contract:

- Owns
- Public API Boundary
- Does NOT Own
- Test Surface
- Cross-Crate Contracts
- Common Pitfalls
- Source Trail

M0's task cards include creating the first set of per-crate AGENTS.md files alongside the workspace scaffold.

## Standard Validation

Run these from `corefall-game/` unless a task card narrows the set:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cxctl -- observe --once
python3 /Users/erol/projects/cortex-command-repos-all/research_tools/prototype_run_check.py /Users/erol/projects/corefall/prototype_runs/native/<run_id>
```

Milestones with gameplay/tool UI also require a scripted E2E command and a screenshot/capture artifact listed in `summary.json.artifacts`.

`cxctl` lives at `corefall-game/crates/cxctl/`. Invoke as `cargo run -p cxctl -- <subcommand>` during M0..M1; once installed or added to PATH, `cxctl <subcommand>` is shorthand. The full CLI surface is pinned in the canonical [CLI Reference](https://.../prototype-roadmap.md#cli-reference).

## Run-Bundle Naming

Run bundles live under `prototype_runs/native/` at the corefall repo root. Naming follows the canonical [Run-Bundle Naming Convention](https://.../prototype-roadmap.md#run-bundle-naming-convention):

```text
prototype_runs/native/<milestone>_<UTC ISO-8601 with hyphens>_<short_hash>/
```

Example: `prototype_runs/native/m0_2026-05-04T22-30-00Z_a1b2c3d4/`.

Each bundle contains `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`, and optional `screenshots/` / `captures/`. Validate with the run-bundle checker named in Standard Validation.

## Open Decision Gates

Do not silently assume an open decision is settled.

If a milestone touches an OPEN decision record or topic-level open decision:

- Confirm the current lean from the canonical vault per the [Open Decision Gates Protocol](https://.../prototype-roadmap.md#open-decision-gates-protocol).
- Implement only what the milestone allows.
- If the lean is contested or would materially change architecture, stop and ask the user through the active agent's available user-input/chat mechanism.
- When prototype evidence closes a DR, update the canonical vault in the same pass (DR file + decisions/index + decision-tracker + research-readiness + a dated research-log note) or explicitly report that the vault update is still pending.

## Eyes, Ears, Hands Rule

Every player-facing surface must be controllable and observable through the planned `cx-control` / `cxctl` layer unless explicitly marked human-only with a reason.

The rule: any pixel a human can interact with on screen, the AI worker must be able to drive through `cxctl`. Screenshot-only testing is not enough. A milestone is incomplete if AI agents cannot inspect and drive the new gameplay/UI surface through structured commands.

See the canonical [[spec/ai-control-observability-layer]] for the full observe/inspect/act surface; every new player-facing surface must extend it.

## Evidence Requirements

Every meaningful prototype run must emit a run bundle under:

```text
prototype_runs/native/
```

Every completed task must update or produce:

- The relevant checklist rows in the canonical `feature-completion-checklist.md`; check off completed rows and fill evidence, commands, run-bundle paths, and AI self-ratings.
- The canonical `prototype-roadmap.md`; update milestone/feature status, evidence links, changed scope, open follow-ups, and any newly discovered dependency or sequencing issue. If no roadmap edit is needed, say why in the implementation log.
- A milestone note under `docs/implementation-log/`.
- A repo-local entry in `CHANGELOG.md`.
- Run-bundle paths.
- Commands run.
- Bugs found and fixed.
- Known limitations.
- AI self-ratings for implementation completeness and quality.

Use human rating fields only for user/human review. AI agents fill only AI self-rating fields and evidence notes.

Canonical checklist:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md
```

Canonical roadmap:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md
```

Repo-only changelog:

```text
CHANGELOG.md
```

## Completion Contract

After implementing any feature, task card, side-track item, or milestone, an agent must leave the project in a state where another agent can see exactly what changed and what remains.

Required completion actions:

1. Update code and tests in `corefall-game/`.
2. Run the Standard Validation commands (above) plus any milestone-specific validation from the assigned roadmap/backlog section.
3. Emit or update run-bundle evidence when the task includes runnable behavior.
4. Update the canonical vault checklist rows that correspond to the completed work.
5. Update the canonical roadmap if status, scope, dependencies, evidence, commands, risks, or follow-up work changed.
6. Add or update the milestone implementation note under `docs/implementation-log/`.
7. Add a concise repo-local entry to `CHANGELOG.md`.
8. If the milestone closes a DR, update the DR file + `decisions/index.md` + `dashboards/decision-tracker.md` + `dashboards/research-readiness.md` + a dated `research-log/` note in the same pass.
9. Verify every new player-facing surface is reachable from `cxctl` with assert/inspect coverage.
10. Report any vault updates that could not be completed, with exact file paths and reasons.

Do not mark work complete if the checklist/roadmap updates are skipped. If a task genuinely does not affect the roadmap, record "roadmap update not needed" in the implementation log and explain why.

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

## Secrets Posture

- Never commit API keys, `.env` files, signing keys, or LLM provider tokens.
- Use environment variables for any secret per [[spec/hybrid-llm-ai-plan]] `MindProviderConfig.api_key_env`.
- The `.gitignore` already excludes `.env`, `.env.*` (with `!.env.example` exception). Do not weaken this without an explicit user request.
- LLM live providers are cargo-feature-gated and never required for any test. CI uses the deterministic mock provider only.

## Do Not

- Don't write source code under `cortext_command_vault/`. The vault is planning, not implementation.
- Don't edit canonical reference repos under `/Users/erol/projects/cortex-command-repos-all/{Cortex-Command-*,comparables_repos/*}` unless the user explicitly says so.
- Don't use `rand::thread_rng()` inside sim crates (`cx-sim-core`, `cx-physics`, `cx-material`, `cx-ai`, ...). Sim RNG must be seeded and recorded per the manifest.
- Don't use `println!` in production code. Use `tracing` per the canonical [Logging, Tracing, And Error Policy](https://.../prototype-roadmap.md#logging-tracing-and-error-policy).
- Don't `unwrap()` on user-controllable inputs.
- Don't skip the Open Decision Gates pre-check before assigning a milestone.
- Don't commit API keys, `.env` files, signing keys, or LLM provider tokens.
- Don't push directly to `main` without local Standard Validation.
- Don't mark work complete if the canonical checklist/roadmap updates are skipped.
- Don't add cloud-save dependencies during T-SAVE work; cloud-save backend decision is post-launch.
- Don't introduce a UI surface without a matching `cx-control` / `cxctl` path. Eyes/ears/hands rule.

## Starting Point

Unless the user assigns a different target, start with:

1. M0 - Engine Bootstrap
2. M1 - Actor Controller And Sim Core
3. M1.5 - Micro Breach Fun Slice

Do not skip M1.5. It exists because the actor-feel lab alone was too sterile; the project needs early fun evidence before deeper systems attach.
