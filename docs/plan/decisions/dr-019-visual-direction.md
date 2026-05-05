---
type: decision
id: DR-019
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Pixel-art readability fails at the chassis/mech scale; or modernized presentation layer overwhelms the pixel-sim look."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/visual-direction|visual direction spec]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[decisions/dr-014-tone-player-promise|DR-014]]

# DR-019: Visual Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Hybrid: **Cortex Command-style pixel-art battlefield** for the simulation layer + **Mark-of-the-Ninja-style silhouette discipline and comic-noir UI** for the presentation layer.

## Decision

**Pixel-sim battlefield + modern readable silhouettes + comic/noir UI and briefing language.**

The simulation layer (terrain, actors, gibs, particles, fire, smoke, sparks, dropped gear) stays small chunky pixel art. This keeps destructible terrain, tunnels, body damage, and modding-friendly sprites readable and natural — the way Cortex Command, Liero, and Soldat do. The presentation layer (HUD, briefings, debriefs, mission cards, replay panels, faction cards, chassis silhouettes) uses cleaner modern silhouettes, bold lighting, strong status colors/icons, and comic-panel debriefs in the spirit of Mark of the Ninja.

This is **not** a raw Cortex clone, **not** modern hand-painted 2D, and **not** photoreal-ish 2.5D.

## What This Locks In

| Spec Area | Implication |
|---|---|
| Engine | Renderer must support sub-pixel-clean pixel-art rendering AND clean vector-ish UI overlays. See [[decisions/dr-001-engine-strategy]]. |
| HUD / UX | Comic-noir briefings, mission cards, replay panels, chassis silhouettes. See [[spec/ux-overlay-screen-brief]] and [[systems/ux-overlay-screen-brief]]. |
| Modding | Sprite-friendly: pixel art is straightforward to author and mod. UI is still themable. See [[spec/modding-model]]. |
| Chassis | Pixel battlefield must read clearly at infantry, powered armor, light/medium/heavy mech scales. See [[spec/chassis-armor-mechs-and-origins]]. |
| Materials/destruction | Pixel-sim terrain naturally supports per-pixel mutation and material overlays. See [[decisions/dr-007-terrain-material-model]]. |
| Faction visual register | Each faction has a visible silhouette/colour/icon language; see [[spec/setting-and-world-frame]]. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Exact pixel resolution / palette | Open. Will be tuned during A1+ playtests. |
| Whether HUD uses literal comic panels or comic-inspired layout | Open. Aesthetic choice. |
| Color blind / accessibility palette specifics | Covered by DR-012; this DR doesn't override. |
| Whether briefings are static art or animated | Open. Static comic panels likely cheapest. |
| Number of factions visually authored at launch | Open. Tied to setting/launch faction set. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Modern stylized 2D (Hollow Knight / Dead Cells) | Beautiful, but expensive and harder to reconcile with destructible pixel terrain. |
| Photoreal-ish 2.5D | Hurts readability of small actors and material overlays; production cost too high for solo/small team. |
| Pure comic/noir | Stylish but loses the physical toy-box chaos that destructible-pixel sim creates. |
| Pure raw Cortex clone | Looks dated; the project deserves a modernized presentation layer. |

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 3): "Pixel-sim battlefield, modern readable silhouettes, comic/noir UI and briefing language."
- Captured in [[research-log/2026-05-04-spec-round-3-visuals-audio-tutorial-mechs-ai]].
- Spec page: [[spec/visual-direction]].

## Revisit Trigger

- Pixel-art readability fails at chassis/mech scale.
- Modernized presentation layer overwhelms the pixel-sim look.
- Modding community can't author sprites at the chosen resolution.
