# M7B — Deep Squad Command Grammar + Formation Orders + Combat Doctrine

## Status

`active`

## Intent

Close the depth gap between M7's 5 AI archetypes and M25's 22-task Priority Table by shipping the **player-issuable squad-command grammar** — 50+ explicit orders, 9 formation kinds with per-actor slot resolution, and a Cortex-Command-style commander-hopping loop so the player can hand off direct control to any squad member while the rest of the squad keeps executing the doctrine. The squad's verb + formation + priority table is state on the *squad*, not the held actor, so doctrine survives the hop.

M7B promise: **"the squad obeys real grammar — formation orders, combat verbs, breach-stack discipline — and the player can take the wheel of any one of them without the rest forgetting the plan."**

## Canonical ownership

- `cf-ai::squad_command_grammar` (parser + verb registry + arg validator) — **NEW**.
- `cf-ai::formation` (9 formation kinds + slot solver + local→world transform) — **NEW**.
- `cf-ai::squad_state` (squad-scoped state; survives commander-hop) — **NEW**.
- `cf-ai::commander_hop` (input-routing + control transfer; integrates with M5) — **NEW**.
- `cf-ai::archetype_bt` (per-archetype BT, 30+ nodes each; deep-fills M7's Layer 3 stub) — **NEW**.
- M7's `Archetype`, `SquadDoctrine`, `BotMemory.doctrine_cache` are read-only inputs.
- M25 Priority Table + Tab overlay + Q-hold wheel are *consumers* of the registry shipped here.

## Player-facing behavior

- **50+ named squad verbs** in a data-driven registry; wheel + Tab overlay enumerate from the same registry the parser accepts. Movement: `Move To`, `Stop`, `Halt`, `Hold Position`, `Bound (alt)`, `Bound (succ)`, `Fall Back`, `Withdraw`, `Retreat In Order`, `Rally On Me`, `Regroup`. Engagement: `Advance`, `Press Attack`, `Hold Fire`, `Fire At Will`, `Engage Priority Target`, `Disengage`, `Overwatch (sector)`, `Suppress (target)`, `Suppress (window)`, `Cover Me`, `Cover That Wall`, `Frag-Out`, `Smoke`, `Flash`. Movement-to-contact: `Breach (door)`, `Stack (door, side)`, `Single-File`, `Wedge`, `Echelon-Left`, `Echelon-Right`, `Line Abreast`, `Column`, `Diamond`, `Form Defensive Perimeter`. Role-specific: `Sniper-Cover`, `Heavy-Forward`, `Engineer-Up`, `Medic-Up`, `Reinforce`, `Drag To Cover`. Logistics: `Pick Up`, `Drop`, `Hand Off`, `Reload Up`, `Top Off Mags`. Each verb has `verb_id`, required + optional args, valid-target predicate, and a doctrine-compatibility row.
- **9 formation kinds** with per-actor slot resolution: `Wedge`, `Diamond`, `Column`, `Line Abreast`, `Echelon-Left`, `Echelon-Right`, `Single-File`, `Stack (door)`, `Defensive Perimeter`. Each formation has slot count, per-slot relative-position vector in commander-facing local space, and per-slot role hint. Slot solver runs at issue + every 2s while moving; collapses gracefully on member loss (Wedge → Diamond → Column → Single-File).
- **Per-actor relative position** in formation routes through M22B's pathfinder as a moving goal; an actor that loses the slot position emits `squad.formation_slot_broken` and the solver reassigns next 2s tick.
- **Cortex-Command-style commander hopping** — Y (gamepad) / Backspace (KBM) enters Brain-Hop, freezes time, surfaces a LOS-radial list of squad members; selecting one transfers M5 input control. The previously-held actor reverts to AI under the *same* squad doctrine. Squad state (verb + formation + priority table + roles) lives on the squad, not the held actor, so doctrine survives the hop.
- **Breach-Stack-Frag-Clear is a four-verb chain**: `Stack (door, left)` positions 1-4 against the jamb, `Breach (door)` charges / kicks, `Frag-Out (interior)` arcs a grenade, `Advance` auto-pushes the stack with pre-assigned sectors-of-fire (Stack-1 ahead, Stack-2 left, Stack-3 right, Stack-4 rear). Emits `squad.breach_chain_started` → `squad.breach_chain_step (4×)` → `squad.breach_chain_complete`.
- **Suppress vs Overwatch vs Cover Me are distinct BT subtrees.** Suppress = sustained low-accuracy high-RPM on a point. Overwatch = hold fire until sector entry, snap-aim once. Cover Me = suppress any LOS-visible threat to the named mover until they reach goal.
- **Retreat verbs are layered.** `Fall Back` = 50u rearward facing the threat. `Withdraw` = move to extraction, weapons holstered. `Retreat In Order` = bounding (half cover, half move 30u, swap); emits `squad.bounding_step`. `Disengage` = break LOS via smoke + cover.
- **Per-member role assignment** is sticky + loadout-aware: `Pointman`, `Rifleman`, `Marksman`, `Heavy`, `Engineer`, `Medic`, `Squad Leader`. Determines slot affinity in the solver; reassigned on KIA / brain-hop.
- **Doctrine veto** — incompatible verb emits `squad.command_vetoed { reason_label }` (e.g. `doctrine_defensive_blocks_press_attack`); Tab overlay flashes the reason; override = re-issue doctrine first.
- **Replay-deterministic** — every command, reslot, chain step, veto is a stable-schema event; bit-exact across machines.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-ai` | MODIFY | Add 5 new modules listed under "Canonical ownership"; wire archetype_bt into the existing 5-layer thinking stack at Layer 3. |
| `cf-ai::squad_command_grammar` | NEW | Verb registry (50+ verbs) + argument typing + valid-target predicates + doctrine-compatibility matrix + parser. |
| `cf-ai::formation` | NEW | 9 formation kinds + slot solver + graceful-collapse FSM + relative-position transform (commander-facing local → world). |
| `cf-ai::squad_state` | NEW | Squad-scoped state (current verb + formation + priority-table + role assignments) decoupled from actor BT state. |
| `cf-ai::commander_hop` | NEW | Brain-hop entry / exit + freeze-time + LOS-radial selector + control-transfer to M5 input router. |
| `cf-ai::archetype_bt` | NEW | Per-archetype BT (30+ nodes for Rifleman, Sniper, Assault, Engineer, Spotter, Heavy) authored as RON; M7 was a stub. |
| `cf-control` | MODIFY | cfctl methods: `act.squad.issue (verb, args)`, `act.squad.set_formation`, `act.squad.assign_role`, `act.player.brain_hop (target_actor_id)`, `srv.dump_squad_state`. |
| `cf-replay` | MODIFY | 12 new event schemas (see Files). |
| `cf-ui` | MODIFY | Tab tactical overlay reads verb registry + formation registry; Q-hold context wheel reads same. |
| `cf-replay` | MODIFY | `squad.*` event family registered. |

## Files

- `game/crates/cf-ai/src/squad_command_grammar.rs` (NEW)
- `game/crates/cf-ai/src/squad_command_grammar/verb_registry.rs` (NEW; 50+ verbs)
- `game/crates/cf-ai/src/squad_command_grammar/parser.rs` (NEW)
- `game/crates/cf-ai/src/squad_command_grammar/doctrine_compat.rs` (NEW)
- `game/crates/cf-ai/src/formation.rs` (NEW)
- `game/crates/cf-ai/src/formation/slot_solver.rs` (NEW)
- `game/crates/cf-ai/src/formation/transforms.rs` (NEW)
- `game/crates/cf-ai/src/squad_state.rs` (NEW)
- `game/crates/cf-ai/src/commander_hop.rs` (NEW)
- `game/crates/cf-ai/src/archetype_bt.rs` (NEW)
- `game/crates/cf-ai/src/archetype_bt/rifleman.rs` (NEW; 30+ BT nodes)
- `game/crates/cf-ai/src/archetype_bt/sniper.rs` (NEW; 30+ BT nodes)
- `game/crates/cf-ai/src/archetype_bt/assault.rs` (NEW; 30+ BT nodes)
- `game/crates/cf-ai/src/archetype_bt/engineer.rs` (NEW; 30+ BT nodes)
- `game/crates/cf-ai/src/archetype_bt/spotter.rs` (NEW; 30+ BT nodes)
- `game/crates/cf-ai/src/archetype_bt/heavy.rs` (NEW; 30+ BT nodes)
- `game/content/ai/formations/wedge.ron` (NEW)
- `game/content/ai/formations/diamond.ron` (NEW)
- `game/content/ai/formations/column.ron` (NEW)
- `game/content/ai/formations/line_abreast.ron` (NEW)
- `game/content/ai/formations/echelon_left.ron` (NEW)
- `game/content/ai/formations/echelon_right.ron` (NEW)
- `game/content/ai/formations/single_file.ron` (NEW)
- `game/content/ai/formations/stack_door.ron` (NEW)
- `game/content/ai/formations/defensive_perimeter.ron` (NEW)
- `game/content/ai/verbs/registry.ron` (NEW; 50+ verb entries)
- `game/crates/cf-control/src/server.rs` (MODIFY: 5 new cfctl methods)
- `game/crates/cf-control/schemas/v1/squad_state.schema.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_command_issued.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_command_vetoed.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_formation_set.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_formation_slot_assigned.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_formation_slot_broken.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_formation_collapsed.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_role_assigned.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_breach_chain_started.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_breach_chain_step.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_breach_chain_complete.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_bounding_step.json` (NEW)
- `game/crates/cf-replay/schemas/event/squad_brain_hop.json` (NEW)
- `game/crates/cf-ui/src/tactical_overlay.rs` (MODIFY: enumerate verb registry)
- `game/crates/cf-ui/src/context_wheel.rs` (MODIFY: enumerate verb registry)

## Acceptance criteria

```gherkin
Scenario: Verb registry enumerates 50+ named squad commands
  Given a fresh squad
  When `srv.dump_squad_state` queries the verb registry
  Then ≥50 verb_ids are returned, each with name + arg schema + valid-target predicate + doctrine-compat row

Scenario: Wedge formation resolves slots and survives a member drop
  Given a 5-member squad
  When `act.squad.set_formation Wedge` issues
  Then squad.formation_set fires with 5 slot assignments in commander-facing local space
  When one member is killed
  Then squad.formation_collapsed fires (Wedge → Diamond) and survivors reslot within 2s

Scenario: Stack-Breach-Frag-Advance chain composes end-to-end
  Given a 4-member squad outside a door
  When the player issues `Stack (door, left)` then `Breach (door)` then `Frag-Out (interior)`
  Then squad.breach_chain_step fires 4× in order (stack → breach → frag → advance)
  And per-actor sectors-of-fire are pre-assigned per slot
  And squad.breach_chain_complete fires after the last actor clears the threshold

Scenario: Suppress vs Overwatch issue distinct BT subtrees
  Given a Rifleman in archetype_bt
  When the player issues `Suppress (window)`
  Then sustained low-accuracy high-RPM fire begins
  When the player instead issues `Overwatch (sector)`
  Then the Rifleman holds fire until a non-friendly enters the sector
  And the two cases emit distinct event traces

Scenario: Doctrine veto rejects incompatible verb
  Given a squad with doctrine = Defensive
  When the player issues `Press Attack`
  Then squad.command_vetoed fires with reason_label="doctrine_defensive_blocks_press_attack"
  And the previous command persists
  When doctrine is reassigned Aggressive and the verb re-issued, it is accepted

Scenario: Commander brain-hop preserves squad doctrine
  Given the player holds Squad Leader and squad command is `Advance`
  When brain_hop targets Rifleman-2
  Then squad.brain_hop fires; M5 input routes to Rifleman-2; former leader reverts to AI under `Advance`
  And squad state (verb + formation + priority table + roles) is unchanged
  When brain-hopping back, control returns with no squad-state mutation

Scenario: Bounding retreat alternates cover and movement
  Given a 4-member squad under fire
  When the player issues `Retreat In Order`
  Then half the squad covers while half moves 30u rearward; squad.bounding_step fires per swap
  And continuous cover is maintained to the rally point

Scenario: Per-archetype BT exposes 30+ nodes
  Given the rifleman archetype_bt
  When enumerated, ≥30 distinct node ids are present
  And the same floor holds for sniper, assault, engineer, spotter, heavy

Scenario: Replay determinism across hop + breach
  Given a recorded mission with one brain-hop and one breach chain
  When replayed on a different machine, every squad.* event is bit-exact

Scenario: Verb + formation registries surface to UI
  Given the Tab tactical overlay is open
  When the player scrolls the verb list
  Then entries come from cf-ai's verb registry (no UI-side duplicate)
  And the Q-hold context wheel reads from the same registry
```

## Out of scope

- Voice-commanded squad orders (mic input → grammar parser) — backlog.
- Player-authored custom verb chains stored to disk (macro recording) — backlog.
- AI-vs-AI squad-command grammar use (enemy AI commander issuing verbs) — already covered by M23B `tactical_planner` proposals consuming the same registry; no new work here.
- Cooperative-multiplayer co-commander (two humans share one squad) — M48 net layer.
- Per-squad-member personality affecting doctrine veto thresholds — M23B `BotMemory` consumer; not in M7B.
- Wheel / overlay visual polish (icons, animation) — M8 UX polish milestone.

## Dependencies

- M5 input router (done): brain-hop routes M5 events to the held actor.
- M7 archetypes + 5-layer thinking stack (done): M7B fills the Layer 3 BT slot left as a stub.
- M22 pathfinding (active): formation slot solver issues per-actor goal moves through M22's pather; M22B extends this with hierarchical grain.
- M25 Tab overlay + Q-hold wheel + 22-task Priority Table (active): consumers of the verb registry shipped here.
- M23B `commander_doctrine` (active): AI-side commander uses the same verb registry.
- `cf-replay` event-schema registration (done).

## Notes for the implementer

- Verb registry + doctrine-compatibility matrix MUST be data-driven RON so M25 wheel / Tab overlay / M23B commander-doctrine re-enumerate without a Rust rebuild.
- `squad_state` lives on the squad entity, NOT the held actor — putting it on the held actor breaks brain-hop and fails the spec.
- Formation slot vectors are commander-facing local; world-space transform uses the *current* commander facing, so the wedge rotates with them.
- Slot solver runs at issue + every 2s while moving. Per-tick is unbounded under squad-size churn.
- Brain-hop is replay-recorded with the human input timeline for the held actor; replays are bit-exact.
- No `thread_rng()` in any BT leaf — engine RNG seeded per actor per tick.
- BTs authored as RON + loaded at startup; do NOT hand-roll the 30-node-per-archetype floor in Rust.
- Commit one verb-family per commit (Movement, Engagement, Movement-to-Contact, Role-specific, Logistics).
- Veto reason labels use `doctrine_<X>_blocks_<Y>` convention so M23B can adapt.
