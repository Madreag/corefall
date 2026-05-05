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
4. Read the assigned milestone rows in the feature checklist.
5. Run the Open Decision Gates pre-check before locking any open decision.
6. Implement all agent-completable task cards for the milestone.
7. Run Standard Validation plus milestone-specific validation.
8. Produce run-bundle evidence under `prototype_runs/native/`.
9. Update the canonical vault roadmap/checklist and repo-local changelog.
10. Leave both repos commit-ready, and commit only when the user asks or when the active assignment explicitly includes committing.

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
- Don't add cloud-save dependencies during T-SAVE work; cloud-save backend decision is post-launch.
- Don't introduce a UI surface without a matching `cf-control` / `cfctl` path. Eyes/ears/hands rule.

## Starting Point

Unless the user assigns a different target, start with:

1. M0 - Engine Bootstrap
2. M1 - Actor Controller And Sim Core
3. M1.5 - Micro Breach Fun Slice

Do not skip M1.5. It exists because the actor-feel lab alone was too sterile; the project needs early fun evidence before deeper systems attach.
