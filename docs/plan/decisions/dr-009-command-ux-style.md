---
type: decision
id: DR-009
status: open
priority: P1
revisit_trigger: "When command-overlay prototype runs ORDER-01 acceptance with three players."
---

← [[decisions/index|decision records]] · [[systems/ux-overlay-screen-brief|UX overlay brief]] · [[systems/ai-and-bots|AI and bots]] · [[game/player-loop-and-ux|player loop]]

# DR-009: Command UX Style

> [!info] Status: OPEN; LEAN: direct control + slowdown command overlay; pause used sparingly

## Context

Cortex's identity comes from blending direct actor control with squad/strategy command. The future UX must let players act both as soldier and commander without losing simulation fidelity. See [[systems/ux-overlay-screen-brief]] and [[systems/ai-and-bots]].

## Options

| Option | Summary |
|---|---|
| A. Real-time only | Player commands while still controlling; no pause/slowdown. |
| B. True pause for commands | Press to pause; full command UI; resume. |
| C. Slowdown for commands | World slows; commands issued live but with breathing room. |
| D. Tactical map mode | Switch to top-down map; issue orders; switch back. |
| E. Hybrid: direct control + slowdown overlay + optional tactical map | Combines C and D. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Maximum tension; "real" feel. | Squad command is cumbersome under fire. | Whether players accept pure real-time. |
| B | Familiar from RTS. | Breaks Cortex tradition; loses simulation immersion. | Pause-thrashing risk. |
| C | Balances tension and control. | New players learn slowdown habits. | Slowdown ratio (75% vs 25%). |
| D | Strong squad UX; clear intent. | Camera dance; loses local feel. | Whether players want a different camera mode. |
| E | Best across modes. | UI complexity. | Whether expert players use both. |

## Evaluation

| Lens | A | B | C | D | E |
|---|---|---|---|---|---|
| Player value | Tense | RTS-like | Cortex-feel | Strategic | Strongest |
| Readability | Low | High | Medium | High | High |
| AI burden | Highest (no breathing room) | Medium | Medium | Medium | Medium |
| UX burden | Low | Medium | Medium | High | Highest |
| Performance risk | Low | Low | Medium | Medium | Medium |
| Modding impact | Low | Low | Medium | High | High |
| Networking/replay | Same | Multiplayer pause issues | Slow factor sync | Map view sync | Both sync |
| Content cost | Lowest | Low | Medium | High | Highest |
| Retention upside | Variable | Medium | Strong | Strong | Strongest |
| Ethics/fairness | OK | OK | OK | OK | OK |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| Cortex tradition: buy menu pauses, world keeps simulating slowly. | [[game/player-loop-and-ux]] | High |
| AI throttle (`Controller::ShouldUpdateAIThisFrame`) is incompatible with mass real-time orders. | [[engine/ai-order-lifecycle]] | High |
| OpenSoldat/Soldat is pure real-time; works because squads are small/disposable. | [[comparables/soldat-and-opensoldat]] | Medium |
| Teardown's plan-then-execute proves slowdown/planning windows. | [[comparables/noita-powder-toy-teardown-rain-world]] | Medium |
| AI trust requires command intent to be visible. | [[systems/ai-trust-test-suite]] | High |

## Current Recommendation

Recommendation: **E. Hybrid (direct control + slowdown overlay; optional tactical map)**.

- Default: direct control of one actor at full speed.
- Hold or toggle: slowdown command overlay (e.g. 25% speed) for issuing orders without leaving direct control.
- Optional: tactical map mode (full pause-equivalent) accessible via key, mostly for set-piece planning and replay scrub.
- Multiplayer: slowdown is host-side; co-op uses time dilation for both players; competitive disabled.
- Replay scrub uses tactical map mode.

Why: matches Cortex tradition, gives breathing room without breaking simulation feel, and supports both tactical map for big decisions and slowdown for fast ones.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| ORDER-01 from [[systems/ux-overlay-screen-brief]]: order preview with blocked segment. | Command overlay works. | Pass = player understands blocked reason in 2s. |
| Slowdown ratio user test (75% vs 25%). | Best feel ratio. | Choose ratio with > 70% preference. |
| Tactical map view: place a 4-unit order in 30 seconds. | Map mode utility. | Pass = under 30s. |
| Replay scrub uses map mode without confusion. | Replay UX. | Pass = users find it natural. |

## Risks

| Risk | Mitigation |
|---|---|
| Slowdown becomes a crutch; combat loses tension. | Tie to a resource (e.g. limited commander focus charge). |
| Tactical map view causes camera disorientation. | Smooth transition; visible cursor anchor. |
| Co-op slowdown is awkward in two-player. | Vote-to-slow or "leader" pattern. |
| Mods change time scales unpredictably. | Mod hooks limited to scripted activities; cannot change time during direct control. |

## Revisit Trigger

Reopen this decision when:

- Command overlay prototype runs in playtests.
- AI architecture (DR-008) commits; command UX must mirror AI's intent layer.
- Co-op (DR-005) commits; multiplayer slowdown rules clarified.
- Tactical-map utility is benchmarked.

## Source Trail

- [[systems/ux-overlay-screen-brief]]
- [[systems/ux-ui-and-retention]]
- [[systems/ai-and-bots]]
- [[systems/ai-trust-test-suite]]
- [[engine/ai-order-lifecycle]]
- [[game/player-loop-and-ux]]
- [[comparables/noita-powder-toy-teardown-rain-world]]
- [[comparables/soldat-and-opensoldat]]
