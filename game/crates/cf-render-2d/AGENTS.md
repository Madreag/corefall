# cf-render-2d — AGENTS.md

## Owns
- M0 minimal Bevy plugin (`CfRenderPlugin`): inserts `ClearColor(M0_CLEAR_COLOR)` and spawns the main 2D camera so `cf-app` always presents a defined cleared frame.
- The M0 clear color constant (`M0_CLEAR_COLOR = #0d121a`).
- M2 will own the chunked terrain pipeline + GPU-assisted carving + sprite batching + particle systems built on wgpu.

## Public API Boundary
- Plugin: `CfRenderPlugin { clear_color: Color }` (defaults to `M0_CLEAR_COLOR`).
- Constant: `M0_CLEAR_COLOR`.

## Does NOT Own
- HUD / mission cards / accessibility UI → `cf-ui`.
- Window creation / event loop / input → `cf-app` (Bevy `WindowPlugin`).
- Sim/state → `cf-sim-core` + `cf-control::engine`.

## Test Surface
- `plugin_inserts_clear_color` unit test asserts the plugin wires `ClearColor` into the world.
- `cargo test -p cf-render-2d`.

## Cross-Crate Contracts
- Depends on: `bevy`.
- Depended on by: `cf-app`.

## Common Pitfalls
- Bevy 0.14 moved `ClearColorConfig` into `core_2d`; the M0 plugin uses the `Camera2dBundle::default()` + `ClearColor` resource to stay version-agnostic.

## Source Trail
- spec/prototype-roadmap §M0 — Engine Bootstrap (M0-S05).
- DR-019 / DR-024 / DR-028.
