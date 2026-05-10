# Replay Viewer — `m2.5_2026-05-09T04-47-07Z_e66a7ad6`

- Scenario: `micro_reactor_defense` (micro_reactor_defense)
- Milestone: `m2.5` (M2.5)
- Tick rate: 60 Hz · Run mode: `bevy-control-driven` · Seed: 47
- Total events: 7777 · First tick: 0 · Last tick: 1989

## State

- Anchor tick: `end (1989)`
- Paused: `false`
- Filter: `mission`
- Tail length: 16

## Tail (3 of 3 matching, showing tick 1..1095)

| tick | category | type | event_id | payload (one line) |
|------|----------|------|----------|--------------------|
| 1 | mission | objective_started | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1:24` | `{"objective":"defend_reactor"}` |
| 1095 | mission | objective_failed | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4391` | `{"objective":"defend_reactor"}` |
| 1095 | mission | mission_resolved | `m2.5_2026-05-09T04-47-07Z_e66a7ad6:1095:4392` | `{"reason":"reactor_destroyed","result":"lost"}` |

## Step Controls

- Step forward: re-run with `--at-tick 18446744073709551615` (current anchor: end (1989))
- Step backward: re-run with `--at-tick 18446744073709551614` (clamps at 0)
- Resume / pause: re-run with `--paused` flag toggle (renderer surfaces the value above).
- Filter: re-run with `--filter <category[,category...]>` (current: mission)
