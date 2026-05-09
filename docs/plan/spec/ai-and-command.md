---
type: spec
status: stub
ready_when: "DR-008 and DR-009 close; AI-01..AI-12 pass with replays."
---

← [[spec/index|spec section]] · [[spec/ai-trust-harness-slice-a|AI harness Slice A]] · [[systems/ai-and-bots|AI and bots]] · [[systems/ai-trust-test-suite|AI trust suite]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# AI And Command Model

> [!warning] Stub

## What goes here when ready

- Layered AI (reflex / tactic / navigation / job / commander / personality).
- Order contract (intent persists, tactic adapts, path repairs).
- Reason labels emitted as events for debug/replay.
- Command UX: direct + slowdown overlay + optional tactical map.
- Authoring: jobs/tactics as data + Lua hooks.

## Current Build Checklist

Use [[spec/ai-trust-harness-slice-a]] to make the AI trust suite runnable. Promote only proven harness/AI behavior back into this curated spec page after replay-backed tests pass.

## Exploratory Requirements

| Requirement | Why |
|---|---|
| AI writes through the same serializable intent/control layer as player input. | OpenSoldat and OpenLieroX both make bots operate through gameplay controls; this supports replay, network prediction, and debug. |
| Terrain manipulation is a normal action type. | OpenLieroX bots carve/clear terrain as part of movement and combat; Cortex-like AI must dig/breach/rescue without special-case hacks. |
| Mobility tools have AI-visible affordance checks. | Rope/tether/jetpack use depends on valid anchor/material/path state; failures need player-visible labels. |
| Every stuck recovery is logged. | OpenLieroX has large stuck/rope recovery heuristics; our version needs reason fields and replay tests instead of hidden guesses. |
| Hazard avoidance is a reflex gate. | Comparable bot code shows projectile avoidance can rot into TODOs; solo-first AI cannot hide this. |

## Inputs

- [[systems/ai-and-bots]]
- [[systems/ai-trust-test-suite]]
- [[spec/ai-trust-harness-slice-a]]
- [[engine/ai-order-lifecycle]]
- [[engine/ai-pathfinding-activities]]
- [[systems/ux-overlay-screen-brief]]
- [[decisions/dr-008-ai-architecture]]
- [[decisions/dr-009-command-ux-style]]
- [[comparables/opensoldat-local-audit]]
- [[comparables/openlierox-local-audit]]
