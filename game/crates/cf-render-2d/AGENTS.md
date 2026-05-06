# cf-render-2d — AGENTS.md

## Owns
- `CfRenderPlugin` (M0): inserts `ClearColor(M0_CLEAR_COLOR)` and spawns the main 2D camera.
- `ActorSpritePlugin` (M1): publishes the engine's actor world as Bevy sprites.
  - `ActorRenderState` resource (cf-app bridge writes it once per frame).
  - `ActorRenderTag`, `FloorRenderTag`, `ReticleRenderTag` components.
  - `sync_actor_sprites` system: spawns / updates / despawns colored rectangles per actor; positions the floor under the play region; positions the reticle 32 px ahead of the player along the normalized aim vector.
- M2 will own the chunked terrain pipeline + GPU-assisted carving + sprite batching + particle systems built on wgpu.

## Public API Boundary
- Plugins: `CfRenderPlugin { clear_color: Color }`, `ActorSpritePlugin`.
- Resources: `ActorRenderState { actors, player_actor_id, region_width, region_height, floor_y }`.
- Components: `ActorRenderTag { id }`, `FloorRenderTag`, `ReticleRenderTag`.
- Constant: `M0_CLEAR_COLOR`.

## Does NOT Own
- HUD / mission cards / accessibility UI → `cf-ui`.
- Window creation / event loop / input → `cf-app` (Bevy `WindowPlugin` + cf-app bridge systems).
- Sim/state → `cf-sim-core` + `cf-control::engine`.
- Authoritative actor data → `cf-actor`. The render layer NEVER mutates engine state.

## Test Surface
- `plugin_inserts_clear_color`, `actor_sprite_plugin_initialises_state`. Run via `cargo test -p cf-render-2d`.

## Cross-Crate Contracts
- Depends on: `bevy`, `cf-actor` (for `ActorObservation`).
- Depended on by: `cf-app` (the bridge fills `ActorRenderState`).

## Common Pitfalls
- Bevy 0.14 moved `ClearColorConfig` into `core_2d`; the M0 plugin uses the `Camera2dBundle::default()` + `ClearColor` resource to stay version-agnostic.
- The render layer must NEVER read or mutate `cf-control::M0Engine` directly; the cf-app bridge owns that copy step.
- Actor color is keyed by team string (`blue` / `red` / fallback) and dimmed when status is `downed` or `dead`.

## Source Trail
- spec/prototype-roadmap §M0 — Engine Bootstrap (M0-S05).
- spec/prototype-roadmap §M1 — Actor Controller And Sim Core (M1-S05).
- DR-019 / DR-024 / DR-028.
- docs/implementation-log/2026-05-06-m1-actor-controller.md.
