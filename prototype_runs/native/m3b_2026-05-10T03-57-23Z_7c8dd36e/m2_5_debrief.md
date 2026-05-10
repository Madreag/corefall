# Debrief — `m2.5_2026-05-09T04-47-07Z_e66a7ad6`

Scenario `micro_reactor_defense` (micro_reactor_defense); milestone `m2.5` (M2.5); seed 47; tick rate 60 Hz; run mode `bevy-control-driven`.

Wall duration 34.004 s; ticks run 1989; total events 7777; result `pass`; exit code 0.

## Outcome

- Result: `lost` (reason: `reactor_destroyed`)
- Resolved at tick: 1095
- Resolved event id: `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4392`

## Objectives

| objective | state | started_tick | ended_tick |
|-----------|-------|--------------|------------|
| `defend_reactor` | `failed` | 1 | 1095 |

## Damage & Death Recap

- Actor deaths: 0
- Projectile hits: 23
- Total projectile damage delivered: 184.0
- Reactor damage events: 10
- Reactor destroyed: yes (at tick 1095)

## Terrain Changes

- `terrain_carved` events: 8
- Total carved pixels: 3222
- `chunk_dirtied` events: 9
- By material:
  - `dirt`: 8

## Key Events

- Errors: 0
- Warnings: 0
- Dropped events: 0
- By category:
  - `actor`: 47
  - `ai`: 3983
  - `combat`: 112
  - `control`: 1434
  - `determinism`: 36
  - `equipment`: 70
  - `input`: 1989
  - `material`: 9
  - `mission`: 3
  - `snapshot`: 11
  - `system`: 35
  - `terrain`: 48

Top event types:

- `ai_perception`: 1989
- `intent_received`: 1989
- `tactic_chosen`: 1989
- `observation_sent`: 1375
- `command_accepted`: 58
- `projectile_spawned`: 56
- `weapon_fired`: 56
- `sim_checksum`: 36

## Checksum Status

- Algorithm: `blake3` · Scope: `sim_state_v1` · Cadence: every 60 ticks
- Final sim checksum: `9ed6b7f611f79414c1461d49b656f504127f87bd4eef68fe84752f15e5c357b0`
- Checksum events emitted: 36
- First tick: 0 · Last tick: 1989
