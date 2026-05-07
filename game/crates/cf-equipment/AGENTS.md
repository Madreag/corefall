# cf-equipment — AGENTS.md

## Owns
- M1 weapon presets + per-actor weapon state machine.
- `RifleSpec` — preset id, fire interval, mag capacity, reload ticks, recoil impulse, muzzle origin, projectile speed/damage/lifetime.
- `RifleState` + `tick_rifle` — fire / reload / dry-fire / cooldown state machine consumed by `cf-actor::sim`.
- `RIFLE_M1_DEFAULT_ID = "rifle_m1_default"` and `rifle_preset(id)` lookup.
- M5 will introduce the full role-record schema (`cf-equipment::RoleRecord`) + `content/equipment/` data path; the rifle preset is a stand-in until then.

## Public API Boundary
- Types: `RifleSpec`, `RifleState`, `TickOutcomes`, `RifleTickInputs`.
- Functions: `tick_rifle(state, inputs)`, `rifle_preset(id)`, `rifle_presets()`.
- Constant: `RIFLE_M1_DEFAULT_ID`.

## Does NOT Own
- Projectile flight / hit detection → `cf-actor::sim` owns projectile bodies and AABB hits.
- Damage routing / chassis modules / armor layers → `cf-chassis` at M5.
- Mod-loadable equipment data → `cf-mod` at M5+.

## Test Surface
- Unit tests: `cargo test -p cf-equipment` covers ready-to-fire, fire decrements ammo + cooldown, cooldown blocks fire, dry-fire when empty, reload duration, auto-reload-when-empty, reset, preset lookup.

## Cross-Crate Contracts
- Depended on by: `cf-actor::sim`, `cf-control::scenario` (for `ScenarioActor::rifle_state`), `cf-control::engine` (initial actor world).
- `tick_rifle` is the single mutator; callers MUST feed edge-triggered `fire_pressed` / `reload_pressed` per tick.

## Common Pitfalls
- The reload counter advances regardless of `fire_pressed`; finishing a reload this tick takes priority over firing so the actor can shoot on the next tick.
- `auto_reload_when_empty=true` only starts a reload when ammo is 0 AND a reload is not already in progress.
- `RifleSpec.preset_id` is owned `String` (not `&'static str`) so RifleSpec can be deserialized; lookup callers should pass `&str` slices.

## Source Trail
- spec/prototype-roadmap §M1 — Actor Controller And Sim Core (M1-003).
- spec/equipment-loadout (M5 full role-record).
- references/equipment-role-records-slice-a (M5 vault data).
- docs/implementation-log/2026-05-06-m1-actor-controller.md.
