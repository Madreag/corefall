# Replay Viewer — `m2.5_2026-05-09T04-47-07Z_e66a7ad6`

- Scenario: `micro_reactor_defense` (micro_reactor_defense)
- Milestone: `m2.5` (M2.5)
- Tick rate: 60 Hz · Run mode: `bevy-control-driven` · Seed: 47
- Total events: 7777 · First tick: 0 · Last tick: 1989

## State

- Anchor tick: `end (1989)`
- Paused: `false`
- Filter: `combat`
- Tail length: 16

## Tail (16 of 112 matching, showing tick 1522..1678)

| tick | category | type | event_id | payload (one line) |
|------|----------|------|----------|--------------------|
| 1522 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1522:6020` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-885.45928955078…` |
| 1530 | combat | projectile_expired | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1530:6047` | `{"cause":"terrain_hit","id":1000048,"last_position":[1049.9384765625,14.516448974609377],"owner…` |
| 1534 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1534:6067` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-885.45928955078…` |
| 1546 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1546:6112` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-900.0,0.0],"wil…` |
| 1551 | combat | projectile_expired | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1551:6129` | `{"cause":"terrain_hit","id":1000049,"last_position":[917.1197509765624,81.65251922607422],"owne…` |
| 1580 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1580:6242` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-900.0,0.0],"wil…` |
| 1586 | combat | projectile_hit | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1586:6263` | `{"damage":8.0,"hit_position":[573.74951171875,36.0],"projectile_id":1000050,"shooter":2,"target…` |
| 1592 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1592:6289` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-900.0,0.0],"wil…` |
| 1604 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1604:6334` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-900.0,0.0],"wil…` |
| 1620 | combat | projectile_hit | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1620:6392` | `{"damage":8.0,"hit_position":[573.74951171875,36.0],"projectile_id":1000051,"shooter":2,"target…` |
| 1632 | combat | projectile_hit | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1632:6439` | `{"damage":8.0,"hit_position":[573.74951171875,36.0],"projectile_id":1000052,"shooter":2,"target…` |
| 1638 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1638:6465` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-900.0,0.0],"wil…` |
| 1644 | combat | projectile_hit | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1644:6486` | `{"damage":8.0,"hit_position":[573.74951171875,36.0],"projectile_id":1000053,"shooter":2,"target…` |
| 1650 | combat | projectile_spawned | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1650:6513` | `{"damage":8.0,"lifetime_ticks":84,"origin":[1168.0,36.0],"owner":2,"velocity":[-885.45928955078…` |
| 1667 | combat | projectile_expired | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1667:6574` | `{"cause":"terrain_hit","id":1000055,"last_position":[917.1197509765624,81.65251922607422],"owne…` |
| 1678 | combat | projectile_hit | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1678:6616` | `{"damage":8.0,"hit_position":[573.74951171875,36.0],"projectile_id":1000054,"shooter":2,"target…` |

## Step Controls

- Step forward: re-run with `--at-tick 18446744073709551615` (current anchor: end (1989))
- Step backward: re-run with `--at-tick 18446744073709551614` (clamps at 0)
- Resume / pause: re-run with `--paused` flag toggle (renderer surfaces the value above).
- Filter: re-run with `--filter <category[,category...]>` (current: combat)
