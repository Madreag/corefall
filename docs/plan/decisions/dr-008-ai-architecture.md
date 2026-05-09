---
type: decision
id: DR-008
status: open
priority: P0
revisit_trigger: "When AI trust suite scenarios AI-01..AI-12 run end-to-end with replays."
---

← [[decisions/index|decision records]] · [[systems/ai-and-bots|AI and bots]] · [[systems/ai-trust-test-suite|AI trust suite]] · [[spec/ai-trust-harness-slice-a|AI harness Slice A]] · [[engine/ai-order-lifecycle|AI order lifecycle]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# DR-008: AI Architecture

> [!info] Status: OPEN; LEAN: hybrid jobs + utility scoring + scriptable hooks; debug overlays mandatory
>
> **Bar raised by project owner (DR-014, 2026-05-04):** "MOST HUMANLIKE AI IN THE GAME." Not just functional, not just trustworthy — the genre's high-water mark for human-feeling friendly and enemy AI. Implication: invest beyond utility scoring once basics work — perception/memory modeling, personality/doctrine layering, communicated intent, mistake patterns, learning-from-defeat. See [[decisions/dr-014-tone-player-promise]] and [[spec/chassis-armor-mechs-and-origins]] AI Contract.

## Context

Solo-first promise depends on AI trust. Cortex AI today is a hybrid: C++ controller plumbing + Lua tactical behavior + scene pathfinder. We need to choose how to evolve this stack so bots are commandable, recoverable, and explainable. See [[engine/ai-order-lifecycle]] and [[systems/ai-trust-test-suite]].

## Options

| Option | Summary |
|---|---|
| A. Pure Lua scripts (Cortex status quo) | Behavior in Lua only; engine offers state. |
| B. Behavior trees in engine + Lua hooks | Static behavior trees with mod-friendly Lua nodes. |
| C. Utility AI in engine + scripted considerations | Score-driven; reasons exposed. |
| D. Hybrid: jobs (intent) + utility (tactic) + scripted hooks (custom) | Layered model with clear separation. |
| E. ML-based bots | Trained NPCs. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Maximum mod flexibility; minimum engine work. | Hardest to make trustworthy; debug nightmare. | Whether community can author trust. |
| B | Visual debugger possible; clear node ownership. | Trees can be brittle for Cortex destructibility. | Tree complexity ceiling. |
| C | Scoreable; each decision has a reason. | Authoring utility scores is unfamiliar to many modders. | Score tuning bandwidth. |
| D | Best of all; clear "intent" layer for orders. | Most complex; multiple authoring surfaces. | Whether modders can navigate three layers. |
| E | Theoretically best gameplay. | Currently impractical for moddable solo game; cheating-prone. | Out of scope. |

## Evaluation

| Lens | A | B | C | D | E |
|---|---|---|---|---|---|
| Player value | Variable | Strong | Strong | Strongest | Speculative |
| Readability | Lowest | Medium | High | Highest | Medium |
| AI burden | Highest | Medium | Medium | Medium | Lowest at runtime |
| UX burden | Hardest debug | Medium | Easier debug | Easiest debug | Hardest |
| Performance risk | Variable | Low | Low | Low | Highest |
| Modding impact | Highest | High | High | Highest | Lowest |
| Networking/replay impact | Hard to capture | Medium | Easy (events with reasons) | Easy | Hardest |
| Content cost | Lowest | Medium | Medium-high | Highest | Highest |
| Retention upside | Variable | Strong | Strong | Strongest | Speculative |
| Ethics/fairness | Variable | Strong | Strong | Strongest | Risk of opaque AI |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| Cortex `NativeHumanAI.lua` selects modes via Lua coroutines. | [[engine/ai-order-lifecycle]] | High |
| `Controller::ShouldUpdateAIThisFrame` throttles AI per-frame; high-priority tick is missing. | [[engine/ai-order-lifecycle]] | High |
| Path-cost recalc can stall, causing stale paths. | [[engine/terrain-mutation-and-pathfinding-lifecycle]] | High |
| Tactics handler already exposes high-level jobs (attack/defend/patrol/brainhunt/sentry). | `TacticsHandler.lua` | High |
| AI trust suite is concrete and 12 scenarios deep. | [[systems/ai-trust-test-suite]] | High |
| OpenSoldat bots show the value of direct input/control-state AI, visible target/stuck counters, and the weakness of static waypoint assumptions for mutable terrain. | [[comparables/opensoldat-local-audit]] | High |
| OpenLieroX bots try to use terrain clearing, rope movement, weapon selection, pathing, mode goals, and stuck recovery, but hazard avoidance remains weak/TODO-heavy. | [[comparables/openlierox-local-audit]] | High |
| Rain World's autonomy + readability lessons. | [[comparables/noita-powder-toy-teardown-rain-world]] | Medium |
| Slice A AI harness requirements now map Cortex AI dispatch, Lua behavior selection, alarm/path/tool/stuck hooks, and external AI debug practices into a runnable scenario/report checklist. | [[spec/ai-trust-harness-slice-a]] | High |

## Current Recommendation

Recommendation: **D. Hybrid: jobs + utility + scripted hooks**.

Layers:

1. **Reflex (engine)**: dodge explosives, brace, avoid muzzle obstruction.
2. **Tactic (utility)**: pick cover, fire, reload, retreat, throw, suppress, flank. Scored per option with reason labels.
3. **Navigation (engine + scripted)**: pathfinding, local steering, dig/build plans.
4. **Job (data-driven)**: miner, engineer, medic, breacher, scout, sniper, commander.
5. **Commander (per-team)**: squad/side allocation, reinforcement, route planning.
6. **Personality (data + script)**: courage, discipline, panic, loyalty.

Comparable-derived requirements:

- AI, player, replay, and network should write through a shared intent/control interface.
- Terrain manipulation is a normal action type, not a special-case exception.
- Mobility tools such as rope/tether/jetpack need AI-safe affordance checks and visible refusal reasons.
- Every stuck recovery needs a logged reason, chosen recovery action, and next retry time.
- Hazard avoidance must be a first-class reflex with tests, not a later behavior-tree leaf.

Authoring:

- Engine ships defaults for each layer.
- Modders add new tactics, jobs, or personalities via typed manifest + optional Lua.
- Every decision emits an event with reason (`tactic_chosen` per [[systems/replay-event-architecture]]).

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| AI-01 to AI-12 from [[systems/ai-trust-test-suite]] run end-to-end with replays. | Core trust. | Pass = all 12 pass; Fail = root-cause + iterate. |
| AI-H-01..AI-H-06 from [[spec/ai-trust-harness-slice-a]] run first. | Harness is real before full suite scale. | Pass = AI-H bootstrap set reports pass/fail with replay links; Fail = missing event/harness plumbing. |
| Replay shows "tactic_chosen" with reason for every fight decision. | Debuggability. | Pass = readable; Fail = add reason fields. |
| Modder authors a "Suppression Specialist" job in 1 day. | Authoring time. | Pass = under 8h; Fail = simplify schema. |
| Path-failure recovery: 90% of bots recover within 5 seconds of route invalidation. | Stuck recovery. | Pass = > 90%. |
| Rope/tether/mobility test. | Bots can use or refuse mobility tools safely. | Pass = bot reaches target or explains why the anchor/path/tool is invalid. |
| Terrain-clearing test. | Bots can dig/breach without self-trapping or friendly fire. | Pass = route opens and replay labels tool choice, target material, and risk score. |

## Risks

| Risk | Mitigation |
|---|---|
| Layered model is too complex for modders. | Provide editor preview; minimal templates; sample mods. |
| Utility score tuning takes too long. | Start small; expand only after AI-01..AI-12 are green. |
| Replay-event channel becomes too chatty (every reason emitted). | Coalesce and cap per-tick; debug overlay vs published replay. |
| Modders break engine invariants via Lua hooks. | Sandbox + typed contract; enforce reflex/tactic/job ordering. |

## Revisit Trigger

Reopen this decision when:

- AI trust suite passes; we know which layers carry weight.
- Replay event budget is benchmarked.
- Modders submit prototype jobs/tactics; ergonomic feedback.
- Networking (DR-005) commits; AI sync constraints clarified.

## Source Trail

- [[engine/ai-order-lifecycle]]
- [[engine/ai-pathfinding-activities]]
- [[systems/ai-and-bots]]
- [[systems/ai-trust-test-suite]]
- [[spec/ai-trust-harness-slice-a]]
- [[systems/replay-event-architecture]]
- [[engine/terrain-mutation-and-pathfinding-lifecycle]]
- [[engine/loadout-delivery-economy-lifecycle]]
- [[comparables/noita-powder-toy-teardown-rain-world]]
- [[comparables/opensoldat-local-audit]]
- [[comparables/openlierox-local-audit]]
