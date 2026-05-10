# Debrief — `m3b_fixture_actor_died_chain`

Scenario `actor_died_chain_fixture` (M3B Actor Death Cause Chain Fixture); milestone `m3b` (M3B); seed 1; tick rate 60 Hz; run mode `fixture`.

Wall duration 0.300 s; ticks run 17; total events 13; result `pass`; exit code 0.

## Outcome

- Result: `won` (reason: `all_red_actors_defeated`)
- Resolved at tick: 16
- Resolved event id: `m3b_fixture_actor_died_chain:16:10`

## Objectives

| objective | state | started_tick | ended_tick |
|-----------|-------|--------------|------------|
| `defeat_red_team` | `completed` | 1 | 16 |

## Damage & Death Recap

- Actor deaths: 1
- Projectile hits: 1
- Total projectile damage delivered: 100.0
- Reactor damage events: 0
- Reactor destroyed: no

## Terrain Changes

- `terrain_carved` events: 0
- Total carved pixels: 0
- `chunk_dirtied` events: 0

## Key Events

- Errors: 0
- Warnings: 0
- Dropped events: 0
- By category:
  - `actor`: 1
  - `combat`: 3
  - `control`: 1
  - `determinism`: 1
  - `mission`: 3
  - `snapshot`: 2
  - `system`: 2

Top event types:

- `snapshot_actor`: 2
- `actor_died`: 1
- `command_accepted`: 1
- `mission_resolved`: 1
- `objective_completed`: 1
- `objective_started`: 1
- `projectile_hit`: 1
- `projectile_spawned`: 1

## Checksum Status

- Algorithm: `blake3` · Scope: `sim_state_v1` · Cadence: every 60 ticks
- Final sim checksum: `abcdef0123456789fedcba9876543210abcdef0123456789fedcba9876543210`
- Checksum events emitted: 1
- First tick: 0 · Last tick: 17
