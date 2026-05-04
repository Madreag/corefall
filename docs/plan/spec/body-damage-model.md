---
type: spec
status: stub
ready_when: "DR-003 closes; HUD-01..HUD-03 acceptance pass."
---

← [[spec/index|spec section]] · [[engine/body-damage-wound-gib-lifecycle|body damage lifecycle]] · [[decisions/dr-003-body-damage-readability|DR-003]]

# Body / Damage Model

> [!warning] Stub

## What goes here when ready

- Damage channels (piercing, blunt, explosive, thermal, chemical, electric).
- Wound-emitter pattern; entry/exit wounds; attachable joint failure.
- Status enum: STABLE, UNSTABLE, DYING, DEAD, INACTIVE.
- Inventory/gold fallout on death.
- HUD silhouette default + advanced opt-in.
- Mission-critical override for brains/key assets.

## Inputs

- [[engine/body-damage-wound-gib-lifecycle]]
- [[engine/projectile-to-impact-lifecycle]]
- [[systems/damage-equipment-and-items]]
- [[systems/ux-overlay-screen-brief]]
- [[decisions/dr-003-body-damage-readability]]
