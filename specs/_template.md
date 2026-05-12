# <ID> — <Name>

<!-- Example: # M5.5 — Projectiles And Ballistics -->

## Status

`active` | `next` | `blocked: <reason>`

## Intent

One sentence. What this milestone delivers to the player.

<!-- Good: "Players can fire projectiles that obey gravity, drag, and material penetration, with visible trails and impact effects." -->
<!-- Bad: "Implement the projectile system." (no player-facing framing) -->

## Player-facing behavior

3-7 bullets. What changes from the player's POV.

- Bullet 1
- Bullet 2

## Crates / modules touched

List every crate the implementer will modify or create. Mark NEW vs MODIFY.

| Crate | Status | What changes |
|---|---|---|
| `cf-physics` | MODIFY | Add ballistic integrator; expose `Projectile` type. |
| `cf-projectile` | NEW | New crate for projectile lifecycle. |

## Files

Explicit list. Every file the implementer will create or modify.

- `game/crates/cf-projectile/src/lib.rs` (NEW)
- `game/crates/cf-physics/src/ballistic.rs` (NEW)
- `game/crates/cf-physics/src/lib.rs` (MODIFY: re-export ballistic)
- `game/crates/cf-control/src/server.rs` (MODIFY: add `act.player.fire_projectile`)
- `game/Cargo.toml` (MODIFY: register cf-projectile)

## Acceptance criteria

Gherkin scenarios. One per observable behavior. Be specific.

```gherkin
Scenario: Bullet drops under gravity over distance
  Given a rifle pointed horizontally at 60 m/s muzzle velocity
  When the player fires
  Then the bullet's vertical position drops by ~0.5 m at 30 m horizontal range
  And impact happens at the predicted parabolic intersection

Scenario: Projectile penetrates dirt, stops in concrete
  Given a bullet with 800 J kinetic energy
  When it hits a dirt block (hardness 10)
  Then it carves a tunnel and exits with reduced energy
  When the bullet then hits a concrete block (hardness 40)
  Then it stops and embeds at the surface

Scenario: Visible projectile trail
  Given a fired projectile in flight
  When the player observes the screen
  Then a visible trail line connects muzzle to current position
  And the trail fades over 0.5s
```

## Out of scope

Bulleted. What is explicitly NOT in this milestone (will be picked up by a later spec).

- Ricochets (M5.5.5)
- Tracer rounds (M5.6 visual juice pass)
- Penetration through actor body zones (M5.7 body damage extension)
- Projectile-vs-projectile collision (never)

## Dependencies

What must exist before this spec can start.

- M5 chassis system (closed; commit `29edc1b`)
- `cf-equipment::FiringProfile` (exists)

## Notes for the implementer

Optional. Anything the planner wants to flag — known traps, design rationale that would surprise a reader, etc. Keep terse.

- Use fixed-tick integration (Verlet); do NOT use frame-time integration.
- Bullet RNG (jam, deviation) MUST seed off the engine's deterministic RNG, not `thread_rng`.
