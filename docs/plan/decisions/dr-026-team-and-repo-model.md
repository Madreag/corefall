---
type: decision
id: DR-026
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Modular crate boundaries fail under integration; or AI-augmented solo throughput is materially below what a small team would deliver; or external collaborators join in a way that changes ownership."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-024-native-engine-stack|DR-024]]

# DR-026: Team And Repo Model

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> **AI-augmented solo / small-core development**. The repo is a **modular cargo workspace** with one crate per feature/agent boundary so AI agents can own crates without colliding. Inter-crate boundaries are defined by traits and event types, not by reaching into each other's structs.

## Decision

**One developer + AI agents. Crate boundaries are the team boundary.** Each crate has an explicit owner (human or AI agent task), an explicit public interface (traits, types, events), and an explicit test surface. New work happens in a single crate or at a documented boundary between crates.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Repo layout | Cargo workspace at `corefall-game/` with one crate per feature subsystem. See [[spec/prototype-roadmap]] Repository Layout. |
| Inter-crate boundaries | Public APIs are traits + types + event structs. Internal data structures stay private. |
| Agent ownership | Each AI agent task names the crate(s) it owns. Cross-crate work requires an explicit handoff or merge. |
| Build hygiene | `cargo check` + `cargo test` + `cargo clippy -- -D warnings` are CI-enforced on every PR. |
| Documentation | Every crate has a top-level rustdoc that explains its purpose, public API, and the boundary it owns. |
| AGENTS.md | Project-level + per-crate AGENTS.md files steer AI agents into the right context. |

## What This Does NOT Lock

- The exact list of crates (will evolve through milestones).
- Whether external collaborators eventually contribute (open).
- Specific AI orchestration tooling (Factory droids, Claude Code, Codex CLI, etc.).
- Whether mod scripts share the repo or live in a separate package repository.

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Monolithic single-crate codebase | Hard for AI agents to scope work; creates merge collisions; blurs ownership. |
| Many tiny crates with deep dependency chains | Compile-time cost; brittle interfaces; over-engineering for solo+AI scale. |
| Fork an existing engine (CCCP-derived) | Carries AGPL/license complexity; mixes upstream maintenance with our roadmap (per DR-001/DR-024). |
| Hire small team upfront | Cost/coordination overhead; AI agents already match small-team output for many feature classes. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): "AI-augmented solo. Modular repo so AI agents can own crates without breakage."
- Mission orchestrator/worker patterns from prior projects (Madreag/maxproxy) demonstrate the agent-owned-feature model produces clean PRs at AI scale.
- Cargo workspace pattern is idiomatic Rust and matches this team-shape.

## Risks

| Risk | Mitigation |
|---|---|
| Crate boundaries leak (impl details escape via re-exports) | Lint each crate's public API at PR review; periodic boundary audits. |
| Cross-crate refactors are slow | Define clear "type-level" boundaries early; large refactors use explicit cross-crate PRs with audit. |
| AI agents over-claim ownership and create merge collisions | Mission preflight: only one open PR per crate at a time (mirrors Madreag mission preflight pattern). |
| Solo+AI throughput estimate is wrong | Track actual milestone throughput; adjust scope or pull in collaborators if a milestone slips materially. |
| Documentation rot | Per-crate AGENTS.md and rustdoc are part of acceptance for every milestone. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M0 — Workspace builds with the crate split. | Cargo workspace pattern works. |
| M1..M3 — Each milestone is delivered as PRs that touch a small set of crates with clean boundaries. | Boundary discipline holds under real work. |
| M5 — One AI-agent-driven PR adds a new chassis archetype using only `cx-chassis` + a content directory. | Agent-owned-feature model works in practice. |
| M8 — Mod loader proves the boundary contract: a third-party mod adds a chassis without modifying core crates. | The same boundaries that serve agents serve modders. |

## Revisit Trigger

- Modular crate boundaries fail under integration (large refactors keep crossing crates).
- AI-augmented solo throughput is materially below what a small team would deliver.
- External collaborators join and the ownership model needs to change.
- The crate count balloons in a way that hurts compile time or developer ergonomics.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-024-native-engine-stack]]
- [[spec/prototype-roadmap]] — Repository Layout + Side Tracks.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
- AGENTS.md (root)
