# Replay Viewer — `m2.5_2026-05-09T04-47-07Z_e66a7ad6`

- Scenario: `micro_reactor_defense` (micro_reactor_defense)
- Milestone: `m2.5` (M2.5)
- Tick rate: 60 Hz · Run mode: `bevy-control-driven` · Seed: 47
- Total events: 7777 · First tick: 0 · Last tick: 1989

## State

- Anchor tick: `1095`
- Paused: `false`
- Filter: `(all categories)`
- Tail length: 12

## Tail (12 of 4393 matching, showing tick 1094..1095)

| tick | category | type | event_id | payload (one line) |
|------|----------|------|----------|--------------------|
| 1094 | input | intent_received | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1094:4381` | `{"actor":1,"aim_x":1.0,"aim_y":0.0,"applied_move_x":0.0,"fire":false,"jump":false,"jump_accepte…` |
| 1094 | ai | ai_perception | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1094:4382` | `{"actor":2,"angle_degrees":0.0,"distance":614.25048828125,"last_seen_position":[565.74951171875…` |
| 1094 | ai | tactic_chosen | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1094:4383` | `{"actor":2,"reason":"attack_target","score_attack":0.19287478923797607,"score_hold":0.100000001…` |
| 1094 | control | observation_sent | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1094:4384` | `{"frame_run_id":"m2.5_2026-05-09T04-47-07Z_e66a7ad6","tick":1094}` |
| 1095 | combat | projectile_hit | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4385` | `{"damage":8.0,"position":[643.0,36.0],"projectile_id":1000033,"target":"core_reactor","target_k…` |
| 1095 | actor | reactor_damaged | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4386` | `{"damage_applied":8.0,"destroyed":true,"hp":0.0,"hp_max":80.0,"reactor":"core_reactor"}` |
| 1095 | actor | actor_status_changed | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4387` | `{"actor":"core_reactor","actor_kind":"reactor","cause":"projectile_hit","new_status":"destroyed…` |
| 1095 | input | intent_received | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4388` | `{"actor":1,"aim_x":1.0,"aim_y":0.0,"applied_move_x":0.0,"fire":false,"jump":false,"jump_accepte…` |
| 1095 | ai | ai_perception | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4389` | `{"actor":2,"angle_degrees":0.0,"distance":614.25048828125,"last_seen_position":[565.74951171875…` |
| 1095 | ai | tactic_chosen | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4390` | `{"actor":2,"reason":"attack_target","score_attack":0.19287478923797607,"score_hold":0.100000001…` |
| 1095 | mission | objective_failed | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4391` | `{"objective":"defend_reactor"}` |
| 1095 | mission | mission_resolved | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4392` | `{"reason":"reactor_destroyed","result":"lost"}` |

## Step Controls

- Step forward: re-run with `--at-tick 1096` (current anchor: 1095)
- Step backward: re-run with `--at-tick 1094` (clamps at 0)
- Resume / pause: re-run with `--paused` flag toggle (renderer surfaces the value above).
- Filter: re-run with `--filter <category[,category...]>` (current: (all categories))
