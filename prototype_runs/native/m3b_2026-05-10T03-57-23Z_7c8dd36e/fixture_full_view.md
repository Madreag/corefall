# Replay Viewer — `m3b_fixture_actor_died_chain`

- Scenario: `actor_died_chain_fixture` (M3B Actor Death Cause Chain Fixture)
- Milestone: `m3b` (M3B)
- Tick rate: 60 Hz · Run mode: `fixture` · Seed: 1
- Total events: 13 · First tick: 0 · Last tick: 17

## State

- Anchor tick: `end (17)`
- Paused: `false`
- Filter: `(all categories)`
- Tail length: 32

## Tail (13 of 13 matching, showing tick 0..17)

| tick | category | type | event_id | payload (one line) |
|------|----------|------|----------|--------------------|
| 0 | system | run_started | `m3b_fixture_actor_died_chain:0:0` | `{"reason":"fixture","run_mode":"fixture","scenario":"actor_died_chain_fixture","seed":1,"tick_r…` |
| 0 | snapshot | snapshot_actor | `m3b_fixture_actor_died_chain:0:1` | `{"actor":1,"hp":100.0,"hp_max":100.0,"position":[100.0,32.0],"team":"blue"}` |
| 0 | snapshot | snapshot_actor | `m3b_fixture_actor_died_chain:0:2` | `{"actor":2,"hp":1.0,"hp_max":80.0,"position":[200.0,32.0],"team":"red"}` |
| 1 | mission | objective_started | `m3b_fixture_actor_died_chain:1:3` | `{"objective":"defeat_red_team"}` |
| 10 | control | command_accepted | `m3b_fixture_actor_died_chain:10:4` | `{"actor":1,"method":"act.player.fire"}` |
| 10 | combat | weapon_fired | `m3b_fixture_actor_died_chain:10:5` | `{"shooter":1,"weapon":"rifle"}` |
| 10 | combat | projectile_spawned | `m3b_fixture_actor_died_chain:10:6` | `{"projectile_id":1000,"shooter":1,"velocity":[800.0,0.0]}` |
| 15 | combat | projectile_hit | `m3b_fixture_actor_died_chain:15:7` | `{"damage":100.0,"hit_position":[200.0,34.0],"projectile_id":1000,"shooter":1,"target":2}` |
| 15 | actor | actor_died | `m3b_fixture_actor_died_chain:15:8` | `{"actor":2,"cause":"projectile","killed_by_actor":1,"position":[200.0,32.0]}` |
| 16 | mission | objective_completed | `m3b_fixture_actor_died_chain:16:9` | `{"objective":"defeat_red_team"}` |
| 16 | mission | mission_resolved | `m3b_fixture_actor_died_chain:16:10` | `{"reason":"all_red_actors_defeated","result":"won"}` |
| 17 | determinism | sim_checksum | `m3b_fixture_actor_died_chain:17:11` | `{"checksum_hex":"abcdef0123456789fedcba9876543210abcdef0123456789fedcba9876543210"}` |
| 17 | system | run_finished | `m3b_fixture_actor_died_chain:17:12` | `{"exit_code":0}` |

## Step Controls

- Step forward: re-run with `--at-tick 18446744073709551615` (current anchor: end (17))
- Step backward: re-run with `--at-tick 18446744073709551614` (clamps at 0)
- Resume / pause: re-run with `--paused` flag toggle (renderer surfaces the value above).
- Filter: re-run with `--filter <category[,category...]>` (current: (all categories))
