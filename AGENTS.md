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
3. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-coder-reading-list.md`
4. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/authoritative-game-spec-v0.md`
5. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md`
6. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/native-implementation-backlog.md`
7. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/feature-completion-checklist.md`
8. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-control-observability-layer.md`
9. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/references/prototype-run-bundle-schema.md`
10. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/decisions/index.md`
11. `/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/dashboards/decision-tracker.md`

For milestone-specific docs, use the tables in:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault/spec/ai-coder-reading-list.md
```

## Implementation Workspace

The actual native game code belongs under:

```text
game/
```

Do not put source code in the planning vault. Do not copy the whole vault into this repo. Implementation notes and milestone evidence belong in this repo under:

```text
docs/implementation-log/
prototype_runs/native/
```

## Open Decision Gates

Do not silently assume an open decision is settled.

If a milestone touches an OPEN decision record or topic-level open decision:

- Confirm the current lean from the canonical vault.
- Implement only what the milestone allows.
- If the lean is contested or would materially change architecture, stop and ask the user.
- When prototype evidence closes a DR, update the canonical vault in the same pass or explicitly report that the vault update is still pending.

## Eyes, Ears, Hands Rule

Every player-facing surface must be controllable and observable through the planned `cx-control` / `cxctl` layer unless explicitly marked human-only with a reason.

Screenshot-only testing is not enough. A milestone is incomplete if AI agents cannot inspect and drive the new gameplay/UI surface through structured commands.

## Evidence Requirements

Every meaningful prototype run must emit a run bundle under:

```text
prototype_runs/native/
```

Every completed task must update or produce:

- The relevant checklist rows in the canonical `feature-completion-checklist.md`; check off completed rows and fill evidence, commands, run-bundle paths, and AI self-ratings.
- The canonical `prototype-roadmap.md`; update milestone/feature status, evidence links, changed scope, open follow-ups, and any newly discovered dependency or sequencing issue. If no roadmap edit is needed, say why in the implementation log.
- A milestone note under `docs/implementation-log/`
- A repo-local entry in `CHANGELOG.md`
- Run-bundle paths
- Commands run
- Bugs found and fixed
- Known limitations
- AI self-ratings for implementation completeness and quality

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

1. Update code and tests in `game/`.
2. Run the validation commands required by the assigned roadmap/backlog section.
3. Emit or update run-bundle evidence when the task includes runnable behavior.
4. Update the canonical vault checklist rows that correspond to the completed work.
5. Update the canonical roadmap if status, scope, dependencies, evidence, commands, risks, or follow-up work changed.
6. Add or update the milestone implementation note under `docs/implementation-log/`.
7. Add a concise repo-local entry to `CHANGELOG.md`.
8. Report any vault updates that could not be completed, with exact file paths and reasons.

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

## Starting Point

Unless the user assigns a different target, start with:

1. M0 - Engine Bootstrap
2. M1 - Actor Controller And Sim Core
3. M1.5 - Micro Breach Fun Slice

Do not skip M1.5. It exists because the actor-feel lab alone was too sterile; the project needs early fun evidence before deeper systems attach.
