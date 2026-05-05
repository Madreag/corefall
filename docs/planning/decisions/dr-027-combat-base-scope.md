---
type: decision
id: DR-027
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Combat-base loop fails to deliver tactical depth; or colony-sim depth becomes critical to retention/identity; or the base feature scope ranges beyond what the AI/UX/replay budgets can support."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-015-player-identity-control-posture|DR-015]] · [[spec/command-core-base-power|command-core spec]]

# DR-027: Combat-Base Scope

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> **Deep combat-base, not full colony sim.** The player-built base is a tactical weapon: command core + power grid + shields + turrets + sensors + doors + repair pads + hangar + storage + traps + breachable structure. The base earns its complexity by being **destroyable, defendable, and uprootable** — not by being an economy/colony-management layer.

## Decision

**The base layer is a combat-and-command system, not a colony sim.** It must be deep enough that base design is a meaningful pre-mission and during-mission decision; it must NOT be deep enough to require farming, citizens, happiness, hunger, sleep, or supply chains.

## What This Locks In

| System | Commitment |
|---|---|
| Command core | Rooted core powers ≥ 2 base systems (per [[spec/command-core-base-power]]). Uprootable into a chassis avatar with stat boost (per [[decisions/dr-015-player-identity-control-posture]]). Losing the core can mean mission failure based on scenario policy. |
| Power grid | Power flows from the core (or auxiliary generators) to base systems. Cutting power disables systems. Repair restores. |
| Shields | Energy shields with health, recovery delay, modular placement. Bypassable by breach mechanics. |
| Turrets | Stationary defense weapons with role-card-style metadata, jam states, and ammo. |
| Sensors | Reveal range, cloaking detection, intrusion alerts. Disable-able. |
| Doors / breachable structure | Doors with HP and lock states; walls breachable per [[decisions/dr-007-terrain-material-model]]. |
| Repair pads | Heal nearby actors over time; finite charges per scenario. |
| Hangar / storage | Ready slots for chassis, equipment caches, salvage staging. |
| Traps | Mines, tripwires, decoys. Authorable in scenarios and base layouts. |
| Base authoring | Players design base layouts in the scenario editor (per DR-030); also a campaign-long base evolves with veterans/salvage/research. |

## What This Does NOT Lock

- The exact unlock/research tree for base systems (open; will be tuned during M7+).
- Whether bases persist across MMO shards (covered by DR-005 + DR-024 networking work).
- The economy layer (currency, salvage values) — that's covered separately.
- Specific number of base modules at launch.

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| Full colony sim (Rimworld / Dwarf Fortress) | Diffuses the combat focus; pulls AI/UI/replay budgets into job assignment, mood, food, etc. |
| Idle/AFK base economy | Undermines the tactical pulp tone (per DR-014); creates pay-to-skip incentives. |
| Base-as-economy-only (a passive resource generator) | Base must matter tactically, not just produce coins. |
| Base-only PvP (a base-vs-base mode without combat) | Base-vs-base is fine within a mission; "base war" mode is post-launch only. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| No base layer at all | Loses Cortex's command-core inheritance and the strategic identity per DR-015. |
| Light base (turrets + shields only) | Underdelivers on the "tactical pulp disaster sandbox" promise per DR-014. |
| Full colony sim | Wrong genre; balloons content and AI cost; competes with the combat focus. |
| Base-as-cosmetic | Loses gameplay reason to defend or breach. |

## Evidence Trail

- Project owner verbatim (2026-05-04 stack round): "Deep combat-base. Command core + power grid + shields + turrets + sensors + doors + repair pads + hangar + storage + traps + breachable structure. NOT full colony sim."
- DR-015 (player identity) already commits to the command-core operator role.
- [[spec/command-core-base-power]] establishes the rooted/uprooted contract.
- [[systems/destruction-objective-mission-patterns]] catalogues breach/defend grammar.

## Risks

| Risk | Mitigation |
|---|---|
| Base design becomes shallow / uninteresting | Acceptance test: 5 distinct base layouts produce meaningfully different mission outcomes by M7. |
| Base scope creeps into colony sim | This DR is the explicit guardrail; revisit only with a deliberate tone change. |
| Base authoring UX is too complex | Scenario editor co-evolves per DR-030; in-engine workbench from M8. |
| Replay/event coverage of base systems is thin | Each base module emits events per [[decisions/dr-002-replay-event-architecture]]; baked into M5/M7 scope. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| M7 — Breach Contract proof mission with command-core + 1 shield + 1 turret + 1 door + 1 repair pad. | Combat-base minimum is real. |
| M7 — Player wins by breach AND by uprooting the core into avatar play. | Both branches of DR-015 are exercised. |
| M8 — Player authors a 5-system base layout in the editor; mission director balances it. | Authoring UX matches scope. |
| M9 — Base systems replay headlessly with bit-identical state. | Determinism extends to base layer. |

## Revisit Trigger

- Combat-base loop fails to deliver tactical depth in M7 playtests.
- Colony-sim depth becomes critical to retention/identity.
- Base feature scope ranges beyond AI/UX/replay budgets.
- A future mode (PvP base-war, MMO siege) demands different base semantics.

## Source Trail

- Project owner stack-round answers (2026-05-04).
- [[decisions/dr-014-tone-player-promise]]
- [[decisions/dr-015-player-identity-control-posture]]
- [[spec/command-core-base-power]]
- [[systems/destruction-objective-mission-patterns]]
- [[spec/prototype-roadmap]] — M7 Mission Director and Breach Contract.
- [[research-log/2026-05-04-roadmap-rebuild-native-stack]]
