# cf-ui — AGENTS.md

## Owns
- M1 status strip (`StatusStripPlugin`): four-line text overlay pinned to the top-left.
  - STATUS line (stable / unstable / downed / dead).
  - ITEM line (slot number + label).
  - HP line (`X / 100`).
  - Reticle / fire-state line (`READY 30/30`, `RELOADING NN%`, `EMPTY`, `COOLDOWN Nt`, `NO RIFLE`).
- `HudState { player, rifle, tick, tick_rate_hz }` resource + `HudRifle` bundle.
- `rifle_status_line(Option<&HudRifle>) -> String` formatter (unit-tested).
- M4 lands the full comic-noir HUD, mission cards, and accessibility floor on top of the same `HudState`.

## Public API Boundary
- Plugin: `StatusStripPlugin`.
- Resource: `HudState`.
- Helper struct: `HudRifle`.
- Function: `rifle_status_line`.
- Components: `StatusStripRoot`, `StatusStripText`, `ItemStripText`, `AmmoStripText`, `ReticleStripText`.

## Common Pitfalls
- Localization plan is OPEN — the M1 strip uses English-only literals (STATUS, ITEM, HP, READY, etc.). Flag any string-source code path that bakes English-only player-facing strings; route through a future `cf-localization` boundary instead of hardcoding text.
- The cf-app bridge is the only writer of `HudState`. The HUD reads it via `Res<HudState>` and never mutates engine state.

## Source Trail
- spec/prototype-roadmap §M1 — Actor Controller And Sim Core (M1-004).
- spec/ux-wireframes-slice-a (M4 full HUD).
- spec/accessibility-comfort-slice-a.
- DR-003 / DR-009 / DR-012 / DR-019.
- docs/implementation-log/2026-05-06-m1-actor-controller.md.
