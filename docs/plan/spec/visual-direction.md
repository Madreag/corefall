---
type: spec
status: planning-anchor-v0
authority: "Visual direction for the simulation and presentation layers. Specific palette/resolution/UI motifs remain open."
ready_when: "First playable Slice A renders chassis-bearing actors at infantry, powered armor, and at least one mech tier with readable silhouettes."
feeds:
  - DR-014
  - DR-019
---

← [[spec/index|spec section]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[decisions/dr-019-visual-direction|DR-019]] · [[spec/chassis-armor-mechs-and-origins|chassis spec]] · [[spec/setting-and-world-frame|setting]]

# Visual Direction

> [!summary] What this page is
> The two-layer visual identity: pixel-sim battlefield + modern silhouette-disciplined comic-noir presentation. Spec is direction-grade; specific palette, resolution, and UI motifs remain to be pinned by playtests.

## Two Layers

### Layer 1 — Pixel-sim battlefield

Small chunky pixel art for everything that lives in the simulation:

- Terrain (per-pixel material, mutable).
- Actors (infantry, powered armor, mechs at all tiers).
- Held devices (weapons, tools, mods).
- Particles (sparks, smoke, dust, blood, ejected casings).
- Gibs and debris.
- Fire / hazard cells.
- Dropped gear.
- Salvage objects.

Why pixel: destructible terrain reads naturally; modding/sprite authoring is simple; faction silhouettes can be designed for distinctness; Cortex/Liero/Soldat lineage is honored without being copied.

### Layer 2 — Comic-noir presentation

Modern, clean, silhouette-disciplined UI and out-of-sim surfaces:

- HUD (chassis silhouette, module strip, status banners).
- Squad panel.
- Command overlay.
- Mission briefings (comic-panel cards).
- Mission debriefs (comic timeline of what happened).
- Replay viewer.
- Loadout / workbench.
- Hub UI.
- Faction cards.
- Death recap.

Why comic-noir: matches the tactical pulp sci-fi tone (DR-014); supports strong silhouette discipline (Mark of the Ninja-style); makes briefings and replays feel like artifacts of the world; readable at high information density.

## Style Anchors

| Anchor | What To Borrow | What NOT To Borrow |
|---|---|---|
| Cortex Command | Pixel terrain mutation, actor/device authoring grammar, body damage readability, modding-friendly sprites. | Dated UI, weak briefing layer, low-contrast palette. |
| Mark of the Ninja | Silhouette discipline, status colors/icons, comic-panel briefings, readable lighting. | 3D rendered look, single-character framing. |
| Soldat | Punchy weapon SFX/visuals, faction recognition. | Simple terrain (no destructible material). |
| Liero / OpenLieroX | Chunky destructible terrain, weapon variety visual language. | Dated UI; bot/HUD design. |
| Noita | Material readability through color. | Per-pixel chemistry depth (DR-007 keeps this as moonshot). |

## Visual Rules

| Rule | Why |
|---|---|
| Silhouettes first. Faction, chassis tier, role, and damage stage must be readable from silhouette alone at battlefield zoom. | Players can't read text mid-combat; silhouettes carry tactical information. |
| Status colors are universal. Health/stage/alarm/status colors are one consistent language across HUD, replay, briefing. | Avoid "what does red mean here vs there?" friction. |
| Comic-panel briefings, not modal text walls. | Tone alignment + readability + asynchronous reading. |
| Material overlays are toggled, not always-on. | Combat shouldn't drown in overlay paint; debug/lab modes can show more. |
| Damage stages have distinct visual cues (smoke wisps → sparks → fire → smoke column → wreck silhouette). | Every chassis stage from [[spec/chassis-armor-mechs-and-origins]] needs a visual stage. |
| Faction visual register matches the faction grammar from [[spec/setting-and-world-frame]]. | Each faction's doctrine/tech-tier/origin-mix produces a distinct visual register. |

## Open Questions

| Question | Status |
|---|---|
| Exact base resolution (320×180? 480×270? 640×360?) | Open. Tied to mech scale + screen real estate; pin in A1+ playtests. |
| Palette size (16-color? 32? 256?) | Open. Suggested 32-64 for richer faction differentiation. |
| Pixel scale ratio (1× / 2× / 3×) | Open. Tied to TV-vs-monitor target. |
| Comic-panel UI: animated or static? | Open. Static is cheapest. |
| Lighting model (unlit pixel art or 2D dynamic lighting)? | Open. Dynamic lighting helps tone but hurts perf and modding clarity. |
| Mech scale visual relationship (do heavy mechs zoom out the camera?) | Open. Tied to DR-021 + camera commitment in v0 spec. |

## Source Trail

- [[decisions/dr-019-visual-direction]]
- [[decisions/dr-014-tone-player-promise]]
- [[decisions/dr-007-terrain-material-model]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/setting-and-world-frame]]
- [[spec/ux-overlay-screen-brief]]
- [[systems/ux-overlay-screen-brief]]
- [[spec/ux-wireframes-slice-a]]
- [[comparables/comparison-matrix]]
