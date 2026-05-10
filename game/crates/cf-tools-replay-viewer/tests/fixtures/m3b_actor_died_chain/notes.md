# M3B Actor Death Cause Chain — Synthetic Fixture

This bundle is a hand-crafted fixture that exercises the full `actor_died`
cause chain through the M3B viewer. Real BP2 bundles (`m1.5_*`, `m2_*`,
`m2.5_*`) do not contain `actor_died` because the player survives every
canonical fun-proof scenario. This fixture fills that gap so M3B-D02
("Death recap renders the parent cause chain for `actor_died` and
`mission_resolved` events") has actual evidence.

## Cause chain shape

```text
mission_resolved (tick 16)
  ↑ actor_died (tick 15, target=2, cause=projectile)
    ↑ projectile_hit (tick 15, projectile_id=1000, damage=100)
      ↑ projectile_spawned (tick 10, projectile_id=1000, shooter=1)
        ↑ weapon_fired (tick 10, shooter=1, weapon=rifle)
          ↑ command_accepted (tick 10, method=act.player.fire)
            ↑ run_started (tick 0)
```

`mission_resolved` and `objective_completed` both link back to `actor_died`
as their parent (the death of the last red-team actor is what won the
mission). The viewer's cause-chain renders both the death recap and the
mission-resolution recap from this single fixture.

## Assumptions Tested

- Cause-chain walks through every link from `actor_died` to root (`run_started`).
- `mission_resolved` (with parent → `actor_died`) renders a 7-link chain.
- `projectile_hit` renders a 4-link chain back to `command_accepted`.

## Good

- Fixture is deterministic and committed to the repo. Every reviewer reproduces the same trace.
- Demonstrates BOTH `actor_died` and `mission_resolved` cause chains.

## Bad

- Counts in `summary.json.event_counts.by_category` and `by_type` were hand-computed; if events are added the maps must be updated in lockstep (the viewer's strict validator will reject stale counts).

## Meh

- Fixture is synthetic, not from a real engine run. A future BP that adds a death-path scenario can replace this fixture with a live bundle.

## Evidence Links

- `events.jsonl` — 13 events with full parent_event_id chain.
- `run_manifest.json` — schema-valid manifest pointing at `actor_died_chain_fixture` scene.
- `summary.json` — total + by_category + by_type + dropped_total all consistent with events.

## Next Actions

- Tests in `crates/cf-tools-replay-viewer/tests/fixtures_integration.rs` validate this bundle loads + cause chain matches the documented shape above.
- Replace with a live bundle once a real death-path scenario lands (BP3+).
