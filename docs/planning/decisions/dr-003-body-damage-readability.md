---
type: decision
id: DR-003
status: open
priority: P0
revisit_trigger: "When a damage UX prototype runs HUD-01..HUD-03 acceptance tests."
---

← [[decisions/index|decision records]] · [[engine/body-damage-wound-gib-lifecycle|body damage lifecycle]] · [[systems/damage-equipment-and-items|damage/equipment]] · [[systems/ux-overlay-screen-brief|UX overlay brief]]

# DR-003: Body Damage Readability Model

> [!info] Status: OPEN; LEAN: keep wound/gib physical, add status + body silhouette UI

## Context

Cortex damage is a graph: wounds, attachables, joints, parent body, status enum, inventory. It is a major part of game identity. Players love memorable gibs but lose to "I had no idea I was hurt" deaths. We need to choose how much of that graph is visible, and through what UI affordances. See [[engine/body-damage-wound-gib-lifecycle]].

## Options

| Option | Summary |
|---|---|
| A. Classic gibs only (no body UI) | Show only sprite damage; rely on visuals + audio. |
| B. Single HP bar + classic gibs | Add a generic HP bar over the actor. |
| C. Body silhouette + wound count | Compact figure HUD showing per-limb wound state. |
| D. Layered damage (armor + body + wounds + stability + device) | Full layered model with separate UI for each. |
| E. Hybrid: mandatory C, optional D | Body silhouette by default; expose layered model in advanced HUD. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Pure simulation aesthetic. | High death-by-confusion rate. | Whether veterans actually want this. |
| B | Universal, easy. | Loses limb-specific damage information. | Generic HP makes Cortex feel like every other shooter. |
| C | Cheap to implement; preserves identity. | Won't capture armor/device damage. | Whether per-limb wound counts are visible enough. |
| D | Most legible. | UI complexity; production cost; tutorial burden. | Whether players will actually parse 5 layers in combat. |
| E | Default sensible; expert mode for fans. | Two designs to maintain. | Cognitive cost of advanced HUD. |

## Evaluation

| Lens | A | B | C | D | E |
|---|---|---|---|---|---|
| Player value | Aesthetic | Generic | Tactical | Most info | Tactical, expert option |
| Readability | Lowest | Medium | High | Highest | High by default |
| AI burden | None | None | Bots get same data via Lua | Bots use full layers | Bots use full layers |
| UX burden | Low | Low | Medium | High | Medium-High |
| Performance risk | Low | Low | Low | Medium | Medium |
| Modding impact | Modders add visual effects | Modders add UI elements | Modders can override silhouette | Modders can author layers | Same as D |
| Networking/replay impact | Events still emitted | Events still emitted | Events match UI | Events match UI | Events match UI |
| Content cost | Low | Low | Medium | High | High |
| Retention upside | Memorable gibs | Generic | Memorable + tactical | Strongest | Strongest |
| Ethics/fairness | OK | OK | OK | OK | OK |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| Damage chain is a graph (wound -> attachable -> root -> status -> inventory). | [[engine/body-damage-wound-gib-lifecycle]] | High |
| `Actor::Status` already exposes STABLE/UNSTABLE/DYING/DEAD as semantic states. | `Source/Entities/Actor.h:33-39` | High |
| AHuman has explicit no-head and limbless rules. | `Source/Entities/AHuman.cpp:2620-2640` | High |
| Players blame physics chaos for "random" deaths in current Cortex. | [[systems/ux-ui-and-retention]] (UX problems table) | Medium |
| Comparable Soldat readability pattern is small but explicit damage feedback. | [[comparables/soldat-and-opensoldat]] | Medium |

## Current Recommendation

Recommendation: **E. Hybrid (default C, optional D)**.

- Default HUD: body silhouette + wound count + status pill (STABLE/UNSTABLE/DYING).
- Advanced HUD (toggle): adds armor bar, device health, stability sub-state, recovery timer.
- Replays use full layered data; UI uses default unless player opted in.

Why: solves the readability gap for new players, keeps the simulation aesthetic for veterans, and aligns engine status enum with UI state.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| HUD-01: Identify wounded limb in 1s. | Silhouette is parseable. | Pass = > 80% players. |
| HUD-02: Status read in 1 glance. | STABLE/UNSTABLE/DYING are visible. | Pass = correct ID > 90%. |
| HUD-03: Loud weapon awareness. | Alarm/sound feedback. | Pass = "you were heard" badge appears within 500ms. |
| Advanced HUD opt-in usage rate after 10 hours. | Veteran satisfaction. | > 25% suggests E is right; < 5% suggests D was unnecessary. |

## Risks

| Risk | Mitigation |
|---|---|
| Silhouette becomes visual clutter at squad scale. | Compact "card" form for non-controlled units. |
| Advanced HUD overwhelms tutorials. | Lock behind tutorial completion or veteran flag. |
| Mods break silhouette by changing body parts. | Provide a fallback "generic figure" if attachables are non-standard. |
| AI uses different data than UI. | Single source of truth: body event stream. |

## Revisit Trigger

Reopen this decision when:

- Damage UX prototype runs HUD-01..HUD-03.
- A modded faction uses non-standard body parts (e.g. crab, robot, ghost).
- The replay viewer needs a richer body-state visualization.

## Source Trail

- [[engine/body-damage-wound-gib-lifecycle]]
- [[engine/combat-actors-gibbing]]
- [[engine/projectile-to-impact-lifecycle]]
- [[systems/damage-equipment-and-items]]
- [[systems/ux-overlay-screen-brief]]
- [[systems/ux-ui-and-retention]]
- [[systems/ai-trust-test-suite]]
