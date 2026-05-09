---
type: decision
id: DR-015
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Prototype playtests show strategy-style play is not viable, or direct-control play becomes mandatory in a way that fights the solo AI/command promise."
---

<- [[decisions/index|decision records]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[game/player-loop-and-ux|player loop and UX]] · [[decisions/dr-008-ai-architecture|AI architecture]] · [[decisions/dr-009-command-ux-style|command UX]]

# DR-015: Player Identity And Control Posture

> [!success] Status: CLOSED-DIRECTION (project owner committed)
> The player is a persistent command identity with optional direct possession/piloting. Direct control is a power tool, not a mandatory always-on mode.

## Decision

The player is a **continuity commander / command-core operator**.

The player identity is anchored in a vulnerable command core, neural uplink, continuity core, operator node, or equivalent fiction. That core is also a physical strategic object: when rooted in the base it powers base systems, and when uprooted it can be embedded into a body/chassis as a dangerous mobile avatar. From that identity, the player can:

- Run the battle as a strategy game through orders, priorities, doctrines, loadouts, rescue plans, and tactical overlays.
- Let AI control soldiers, androids, robots, drones, powered armor, and mechs by default.
- Directly possess or pilot any eligible body/chassis when they want hands-on control.
- Release control back to AI without making that body inert or useless.
- Keep the command core rooted in the base to power shields, powered turrets, sensors, doors, repair platforms, energy pads, command relays, and logistics beacons.
- Uproot the command core and plant/embed it into a unit body or chassis to create a stronger core-bearer/avatar with higher armor, health, mobility, energy, equipment power, abilities, and command/control aura.
- Manage persistent campaign state between missions: squad roster, veterans, chassis records, salvage, base state, contract history, enemy commanders, and replay archive.

The game should be valid both as:

| Play Style | What It Means |
|---|---|
| Commander-first | Player rarely direct-controls; they plan, order, observe, intervene through command tools, and rely on capable AI. |
| Pilot-first | Player frequently rides into bodies/mechs and uses direct control for clutch moments, breaches, rescues, duels, and skilled movement. |
| Hybrid | Player switches fluidly between command and direct control as the battlefield changes. This is the default fantasy. |

## Command Core Power States

| State | What It Means | Main Tradeoff |
|---|---|---|
| `rooted_base` | Core is installed in the base and powers shields, turrets, sensors, doors, repair/charging platforms, command relays, and base-wide control boosts. | Strong defense and support, but core position is predictable. |
| `portable_core` | Core is uprooted, carried, transported, or being moved toward another socket/chassis. | Enables evacuation or avatar play, but base drops to reserve power and the core is exposed. |
| `embedded_avatar` | Core is planted into a body, android, robot, powered armor, or mech. The unit becomes a stronger core-bearing avatar. | Massive local power and direct intervention, but base systems weaken/offline and losing the avatar can lose the run. |

See [[spec/command-core-base-power]] for the implementation-facing system contract.

## Why This Matters

The vault already treats the brain/body split as foundational: the player identity is not the current body. This decision keeps that strength while raising the AI bar. If AI can only function when the player is actively puppeting a unit, the solo-first strategy promise fails.

The desired product is not a normal shooter with squad helpers, and not a pure RTS. It is a tactical physics sandbox where bodies and machines are actors in an inspectable plan. The player may step inside one at any moment, but the rest of the squad should keep thinking, reacting, rescuing, fighting, retreating, repairing, and explaining itself.

## Spec Implications

| Area | Required Implication |
|---|---|
| AI | Every controllable body/chassis needs a competent autonomous mode with visible intent, reason labels, and recovery behavior. |
| Direct control | Taking control overrides AI through the same serializable intent/control layer where practical. Releasing control hands the actor back to AI cleanly. |
| Command UX | Orders, doctrine, tactical map, squad panel, and alerts must be good enough to play without constant possession. |
| Campaign/save model | Save identity belongs to the command core/profile, not to the currently controlled body. Actors and chassis are persistent assets with their own records. |
| Base power | Rooted command core powers base modules; uprooting it changes shields, turrets, sensors, doors, repair platforms, energy pads, command relays, and logistics. |
| Avatar core | Embedding the core into a body/chassis creates a boosted avatar state with explicit armor/health/mobility/energy/equipment/ability/control effects and high loss risk. |
| Replay | Replay must distinguish player-piloted actions, AI-controlled actions, order-driven actions, and autonomous recovery decisions. |
| Multiplayer architecture | Future co-op can map each player to a command identity/operator rather than tying each player permanently to one body. |
| Accessibility | Strategy-first play can support players who cannot or do not want to perform every twitch-control action manually. |

## Non-Goals

| Non-Goal | Reason |
|---|---|
| Force direct control at all times. | This would undermine the strategy-game path and the AI trust promise. |
| Make AI play the game invisibly. | The player must see intent, blocked reasons, risks, and cause chains. |
| Split into four unrelated campaign identities at launch. | A unified command-core model is cleaner; origin flavor can vary narrator voice, faction reaction, and campaign traits later. |
| Remove direct control friction. | Physical puppeting remains important when the player chooses to intervene. |

## Prototype Requirements

| Test | Pass Signal |
|---|---|
| Commander-only breach | A player can complete or meaningfully fail a small mission without direct possession, using orders and loadout planning. |
| Pilot intervention | A player can possess one actor/chassis for a clutch action, then release it back to AI without breaking the plan. |
| Rooted-core base power | A rooted core powers at least shields, one turret, and one repair/charging platform with visible UI state. |
| Uproot/avatar tradeoff | Uprooting and embedding the core into a body/chassis creates a boosted avatar while weakening or offlining base modules. |
| AI handoff replay | Replay labels AI-owned, player-owned, and order-driven actions clearly. |
| Strategy readability | Squad panel and command overlay explain what each body is trying to do and why. |

## Source Trail

- [[game/player-loop-and-ux]]
- [[design/design-decisions]]
- [[spec/authoritative-game-spec-v0]]
- [[spec/progression-retention]]
- [[spec/chassis-armor-mechs-and-origins]]
- [[spec/command-core-base-power]]
- [[decisions/dr-008-ai-architecture]]
- [[decisions/dr-009-command-ux-style]]
