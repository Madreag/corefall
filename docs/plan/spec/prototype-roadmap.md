---
type: spec
status: stub
ready_when: "DR-001 and DR-004 close; first prototype is benchmarked."
---

← [[spec/index|spec section]] · [[spec/actor-feel-sandbox-slice-a|actor-feel Slice A]] · [[spec/terrain-material-sandbox-slice-a|terrain/material Slice A]] · [VAULT_PLAN.md](../../VAULT_PLAN.md) · [[strategy/research-to-spec-roadmap|roadmap]]

# Prototype Roadmap

> [!warning] Stub
> The full execution plan lives in [VAULT_PLAN.md](../../VAULT_PLAN.md). This spec page will hold the curated, time-boxed prototype order.

## What goes here when ready

- Slice A: actor-feel sandbox (single actor + small destructible scene). See [[spec/actor-feel-sandbox-slice-a]] and [[spec/terrain-material-sandbox-slice-a]] for current prototype requirements.
- Slice B: small squad scenario (3 actors + simple objective + replay).
- Slice C: bunker breach with AI commander.
- Kill criteria per slice.
- Risk budget per system (AI, terrain, replay, networking).

## Current Slice A Scope

| Slice | Status | Core Proof | Required Companion Infrastructure |
|---|---|---|---|
| [[spec/actor-feel-sandbox-slice-a|A: Actor-feel sandbox]] | <span class="cc-flag cc-blue">PROTOTYPE REQS</span> | One actor is fun for five minutes: move, aim, shoot, dig, explode, repair, recover, and optionally tether/grapple/jet. | Explicit control intent, material overlay, semantic terrain events, rolling recorder, actor/terrain snapshots. |
| [[spec/terrain-material-sandbox-slice-a|A.T: Terrain/material sandbox]] | <span class="cc-flag cc-blue">PROTOTYPE REQS</span> | Eight-material fixture proves dig, shoot, blast, repair/fill, hazard, anchor/nohook, path refresh, and replay/export behavior. | MAT-T event contract, dirty-region batching, path refresh counters, recorder chunk snapshots, AI material labels. |

## Slice A Exit Criteria

| Gate | Pass Criteria |
|---|---|
| Feel | A-FEEL-01..06 in [[spec/actor-feel-sandbox-slice-a]] pass. |
| Material readability | MAT-01A..D pass against the minimum material set. |
| Terrain/material lab | MAT-T-01..MAT-T-10 in [[spec/terrain-material-sandbox-slice-a]] pass or failures are logged with redesign decisions. |
| Replay | REC-01 and REC-02 reconstruct damage/death/terrain causes. |
| DR handoff | DR-004 can decide whether to move to Slice B, repeat A, or split mobility into A.1. |

## Inputs

- [VAULT_PLAN.md](../../VAULT_PLAN.md)
- [[spec/actor-feel-sandbox-slice-a]]
- [[spec/terrain-material-sandbox-slice-a]]
- [[strategy/research-to-spec-roadmap]]
- [[decisions/dr-004-first-playable-slice]]
- [[decisions/dr-001-engine-strategy]]
