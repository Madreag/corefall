# M3B — Replay Viewer + Debrief

## Status

`active`

## Intent

A simple offline tool reads a run bundle and presents the events in a human-scannable form: scrub through events, filter by category, see the parent cause chain for any death or mission-resolved event, and emit a debrief markdown summarizing the run. DR-002 (replay/event architecture) closes when the viewer + cause-chain + debrief work end-to-end.

## Player-facing behavior

- (M3B is offline tooling, not in the game loop.) After a run, the player or developer runs `cargo run -p cf-tools-replay-viewer -- view <bundle> [--at-tick N] [--filter <category>] [--tail-len N]` to read the events.
- `cause-chain <bundle> [--event-type T]` walks parent links from a `actor_died` or `mission_resolved` event back to the root cause.
- The viewer emits a markdown debrief at `<bundle>/debrief.md` that summarizes outcome, mission state, key events, checksums.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-tools-replay-viewer` | NEW (or promote stub) | Binary + library. CLI: `view`, `cause-chain`, `debrief`. egui-based or pure-CLI markdown emit. |
| `cf-replay` | MODIFY (small) | `parent_event_id` field on every event that has a logical cause (combat hit → status change → death; tool action → carve event; objective completed → mission resolved). Read-side helpers for cause-chain walking. |

## Files

- `game/crates/cf-tools-replay-viewer/src/lib.rs` (NEW or MODIFY)
- `game/crates/cf-tools-replay-viewer/src/main.rs` (NEW or MODIFY)
- `game/crates/cf-tools-replay-viewer/src/view.rs` (NEW: event tail + filter)
- `game/crates/cf-tools-replay-viewer/src/cause_chain.rs` (NEW: parent-chain walker)
- `game/crates/cf-tools-replay-viewer/src/debrief.rs` (NEW: markdown emitter)
- `game/crates/cf-replay/src/event.rs` (MODIFY: parent_event_id)

## Acceptance criteria

```gherkin
Scenario: View a bundle's events
  Given a valid run bundle
  When `cf-tools-replay-viewer view <bundle>` runs
  Then stdout shows a chronological event list with tick, category, type, payload summary
  And the output is human-scannable (no raw JSON dumps)

Scenario: Filter by tick
  Given a 5-minute bundle
  When `cf-tools-replay-viewer view <bundle> --at-tick 1800` runs
  Then the output is centered on tick 1800 with --tail-len events on either side

Scenario: Filter by event category
  Given a bundle with mixed events
  When `cf-tools-replay-viewer view <bundle> --filter mission` runs
  Then the output contains only mission.* events

Scenario: Cause chain for actor_died
  Given a bundle where an enemy guard died from player fire
  When `cf-tools-replay-viewer cause-chain <bundle> --event-type actor_died` runs
  Then the output walks parent_event_id links from actor_died → actor_status_changed (DEAD) → actor_status_changed (DOWNED) → combat.projectile_hit → equipment.weapon_fired → input.intent_received (fire press)
  And the chain terminates with one of: RootReached / ParentMissingFromBundle / MaxDepthReached / CycleDetected

Scenario: Cause chain for mission_resolved
  Given a bundle where the mission was won by reaching extraction
  When `cf-tools-replay-viewer cause-chain <bundle> --event-type mission_resolved` runs
  Then the output shows the parent chain OR a clear "no parent chain (event was emitted directly without a parent)" message
  And the message is honest, not a silent empty result

Scenario: Debrief markdown
  Given a completed run bundle
  When `cf-tools-replay-viewer debrief <bundle>` runs
  Then a debrief.md file is written next to the bundle
  And the markdown includes:
    - ## Outcome (won/lost/aborted/in_progress + reason)
    - ## Mission state (objectives + transitions)
    - ## Key events (count by category; first/last by tick)
    - ## Checksum status (algorithm, scope, cadence, final_hex, event_count)
    - ## Captures (if any)

Scenario: Cause chain handles cycle detection
  Given a (hypothetically corrupt) bundle with a parent cycle A→B→A
  When cause-chain runs
  Then the walker terminates with CycleDetected
  And does not infinite-loop

Scenario: Read-only — viewer never mutates the bundle
  Given any bundle
  When any viewer subcommand runs
  Then no file inside the bundle is modified
  And no side-effect file (other than debrief.md) is written
```

## Out of scope

- GUI scrubber (interactive timeline scrubbing) — future tooling polish
- Replay editing — future
- Live attach to a running engine — DR-052 / future net work
- Animated cause-chain visualization — future polish
- Comic-noir styling on the debrief — M4B (deferred)

## Dependencies

- M3A event recorder (must be done): events.jsonl exists, snapshot cadence is real, parent_event_id surface defined.

## Notes for the implementer

- Cause-chain works because every event carries `parent_event_id` (M3A surface). When the parent is missing from the bundle (because it pre-dates the bundle's first event, or the bundle was trimmed), report `ParentMissingFromBundle` honestly.
- Debrief must include the checksum block — this is the offline reviewer's confidence check that the bundle is genuine and complete.
- DR-002 closure: when M3B done-criteria pass, update `decisions/dr-002-replay-event-architecture.md` status to CLOSED-DIRECTION-WITH-EVIDENCE in the same pass. (If decisions/ files have been removed, skip.)
- Viewer is a CLI for now; egui visualization is optional polish. The CLI markdown emit is the floor.
