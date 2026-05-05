---
type: spec
status: stub
ready_when: "HUD-01..HUD-03, ORDER-01, BUY-01 acceptance tests pass."
---

← [[spec/index|spec section]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]] · [[systems/ux-overlay-screen-brief|UX overlay brief]] · [[systems/ux-ui-and-retention|UX/retention]]

# UX/UI Model

> [!warning] Stub
> Build-facing UX wireframes, accessibility floors, and UX-W acceptance tests now live in [[spec/ux-wireframes-slice-a]].

## What goes here when ready

- Screen inventory (HUD, squad, command overlay, buy/loadout, material/path overlay, replay viewer, mission briefing/end, settings).
- HUD state machine mirroring `Actor::Status`.
- Slowdown/pause modes.
- Accessibility defaults.
- Acceptance tests (HUD-01..HUD-03, SQUAD-01, ORDER-01, BUY-01, MAT-01, REPLAY-01).

## Inputs

- [[systems/ux-overlay-screen-brief]]
- [[spec/ux-wireframes-slice-a]]
- [[systems/ux-ui-and-retention]]
- [[game/player-loop-and-ux]]
- [[decisions/dr-003-body-damage-readability]]
- [[decisions/dr-009-command-ux-style]]
