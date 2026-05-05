# Corefall Agent Guide

This file is for AI implementation agents working in `~/projects/corefall`.

## Source Of Truth

This is the implementation repo. The canonical research vault is still:

```text
/Users/erol/projects/cortex-command-repos-all/cortext_command_vault
```

The copied planning snapshot is in:

```text
docs/planning/
```

Use the local snapshot for fast implementation context, but check the canonical vault if a detail seems stale, contradictory, missing, or important to a decision.

## Mandatory Read Order Before Any Milestone

Read these in order before implementing a roadmap milestone:

1. [AGENTS.md](AGENTS.md)
2. [docs/planning/spec/ai-coder-reading-list.md](docs/planning/spec/ai-coder-reading-list.md)
3. [docs/planning/spec/authoritative-game-spec-v0.md](docs/planning/spec/authoritative-game-spec-v0.md)
4. [docs/planning/spec/prototype-roadmap.md](docs/planning/spec/prototype-roadmap.md)
5. [docs/planning/spec/native-implementation-backlog.md](docs/planning/spec/native-implementation-backlog.md)
6. [docs/planning/spec/feature-completion-checklist.md](docs/planning/spec/feature-completion-checklist.md)
7. [docs/planning/spec/ai-control-observability-layer.md](docs/planning/spec/ai-control-observability-layer.md)
8. [docs/planning/references/prototype-run-bundle-schema.md](docs/planning/references/prototype-run-bundle-schema.md)
9. [docs/planning/decisions/index.md](docs/planning/decisions/index.md)
10. [docs/planning/dashboards/decision-tracker.md](docs/planning/dashboards/decision-tracker.md)

For conditional milestone-specific docs, use the tables in [docs/planning/spec/ai-coder-reading-list.md](docs/planning/spec/ai-coder-reading-list.md).

## Required Roadmap Sections

Every implementation agent must read these roadmap sections before coding:

- Read Order
- Glossary
- Agent Implementation Contract
- Open Decision Gates Protocol
- Milestone Handoff Template
- Strategic Frame
- Stack At A Glance
- Repository Layout
- Toolchain And Workspace Bootstrap
- Per-Crate AGENTS.md Template
- Logging, Tracing, And Error Policy
- Asset And Placeholder Strategy
- Testing Layers
- CLI Reference
- Control Transport And Envelope
- Scenario Manifest Schema
- Run-Bundle Naming Convention
- Bug Log Format
- Per-Milestone Kickoff Smoke
- The assigned milestone section
- Validation Command Matrix
- Bug Hunt Checklist
- Definition Of Done
- Anti-Goals

## Open Decision Gates

Do not silently assume an open decision is settled.

If the assigned milestone touches an OPEN DR or topic-level open decision:

- Confirm the current lean from `docs/planning/decisions/` and the canonical vault.
- Implement only what the milestone allows.
- If the lean is contested or would materially change architecture, stop and ask the user.
- When prototype evidence closes a DR, update the DR file, decision index, tracker, readiness page, and implementation log in the same pass.

## Eyes, Ears, Hands Rule

Every player-facing surface must be controllable and observable through the planned `cx-control` / `cxctl` layer unless explicitly marked human-only with a reason.

Screenshot-only testing is not enough. A milestone is incomplete if AI agents cannot inspect and drive the new gameplay/UI surface through structured commands.

## Evidence Requirements

Every meaningful prototype run must emit a run bundle under:

```text
prototype_runs/native/
```

Each completed task must update:

- The relevant rows in [docs/planning/spec/feature-completion-checklist.md](docs/planning/spec/feature-completion-checklist.md)
- The milestone note under [docs/implementation-log/](docs/implementation-log/)
- Any run-bundle evidence links
- Any open bugs or known limitations

Use human rating fields only for user/human review. AI agents fill only AI self-rating fields and evidence notes.

## Reference Repos And Research Vault

Reference repos under `/Users/erol/projects/cortex-command-repos-all` are read-only unless the user explicitly says otherwise.

Do not copy code/assets from external projects into Corefall without logging the source and license posture in:

```text
docs/planning/references/usage-ledger.md
```

For now, reuse/licensing guidance is not a blocker for private research or prototypes, but provenance must be tracked so release decisions are clean later.

## Implementation Posture

Build the best game and best UX first. The planning docs contain safety, reuse, scope, and launch-boundary guidance, but they should not be misread as bans on research, prototyping, or learning from other games.

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

1. M0 — Engine Bootstrap
2. M1 — Actor Controller And Sim Core
3. M1.5 — Micro Breach Fun Slice

Do not skip M1.5. It exists because the actor-feel lab alone was too sterile; the project needs early fun evidence before deeper systems attach.
