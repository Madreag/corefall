# cf-terrain — AGENTS.md

## Owns
- **M1.5 soft-breach barrier proxy** (root module): `BreachStrip`, `BreachWorld`, `try_dig`, `DigRequest`, `DigOutcome`, `BreachView`. Retained for backward compat with `micro_breach.ron`; the event names + payload shapes intentionally match what M2 emits so consumers (replay viewer, AI hooks, run-bundle checker) do not migrate.
- **M2 chunked pixel terrain** (`chunked` module): `ChunkedTerrain` (sparse 256×256 chunks), `Chunk`, `ChunkCoord`, `MaterialId` + `MaterialAffordance` + `MaterialRegistry`, `try_carve`, `try_blast`, `fill_aabb`, `fill_circle`, `aabb_overlaps_solid`, `column_top_solid_y`, `material_at_world`, `ChunkedTerrainSnapshot` round-trip, `TerrainStamp` (FillAabb / FillCircle), layout-stable `checksum_bytes`. CHUNK_SIZE = 256 (constant per the canonical roadmap M2 scope).
- **DR-007 launch material set** (8 ids 0..7): `air, dirt, concrete, metal_nohook, hazard, loose_fill, repair_fill, anchor` with stable affordances (solid / diggable / hardness / anchorable / hazard / path_cost / overlay_rgba / refusal_reason). `concrete_soft` retained as a deprecated M1.5 alias of `concrete`. `material_schema_version = "cf-terrain-launch-v1"`.
- `BreachWorld::reset` + `ChunkedTerrain::reset_to_default` for the engine to rewind on `scenario.reset`. Both `checksum_bytes` outputs feed `sim_state_v1` deterministically (append-only relative to the M0 prefix).
- Anti-scope: NO active CA / reaction table / phase change / chemistry — those land at M5.6 (material kernel). NO collision matrix — that lands at M5.5. NO atmospherics — that lands at M5.10 / M7.5.

## Public API Boundary
- Types from root: `BreachStrip`, `BreachWorld`, `DigRequest`, `DigOutcome`, `BreachView`.
- Functions from root: `try_dig(&mut BreachWorld, DigRequest) -> DigOutcome`, `BreachWorld::is_broken`, `BreachWorld::broken_map`, `BreachWorld::reset`, `BreachWorld::checksum_bytes`, `BreachWorld::iter`.
- Re-exported from `chunked` module at the crate root: `ChunkedTerrain`, `MaterialId`, `MaterialAffordance`, `MaterialRegistry`, `ChunkedTerrainSnapshot`, `ChunkedTerrainSnapshotChunk`, `TerrainStamp`, `Chunk`, `ChunkCoord`, `ChunkedCarveOutcome`, `ChunkedCarveStats`, `ChunkedCarveRefusal`, `ChunkedCarveNoOp`, `material_affordance`, `material_id_from_name`, plus the 8 `MATERIAL_*` id constants and `MATERIAL_SCHEMA_VERSION`.

## Does NOT Own
- Recorder events / run-bundle writing → `cf-control` engine emits `terrain.*` + `material.*` events from outcomes.
- Material kernel + reactions → `cf-material` (DR-036 / T-MAT, lands at M5.6).
- Atmosphere networks → `cf-atmos` (DR-036 / M7.5).
- Physics/collision against terrain proxies → `cf-physics` (DR-033 / T-PHYS, lands at M5.5).
- Projectile-vs-terrain wiring → `cf-control::M0Engine::drive_tick` consumes `material_at_world` + `is_solid`.

## Test Surface
- Unit tests: `cargo test -p cf-terrain` covers:
  - **M1.5 BreachStrip**: out-of-range refusal, metal-nohook material refusal, three-attempt breach, nearest-strip picker, explicit-target routing, unknown-target refusal, reset, broken-map consistency, checksum-byte change after carve.
  - **M2 ChunkedTerrain**: launch-set material id resolution, FillAabb / FillCircle dense-pixel writes, try_carve into dirt + into metal (refused with material_metal_nohook + material_anchor) + into air (NoOp out_of_range), carve_count + refusal_count semantics, AABB-overlaps-solid + column-top-solid-y physics integration, dirty chunk tracking + clear_dirty, checksum changes when terrain changes, snapshot round-trip preserves pixel values, reset clears storage + counters, material counts balance terrain extent, try_blast clears diggables but refuses hazard at any finite force (regression for the hardness=0 → INFINITY fix), in_bounds rejects extreme i64 coords (regression for the u32 truncation fix), AABB clamp to terrain extent, chunk uniformity reclaims storage.

## Cross-Crate Contracts
- Depended on by: `cf-control` (engine + scenario + observe envelope + run-bundle event emission), `cf-render-2d` (M1.5 breach strip projection; chunked-terrain visual rendering tracked for BP3 / M4A), `cf-app` (HUD bridge).
- Events the engine emits from `cf-terrain` outcomes:
  - **M1.5 strip path**: `terrain.tool_action_started { mode: strip }`, `terrain.terrain_carved { mode: strip, strip_id }`, `terrain.terrain_breach_stub`, `terrain.tool_refused { mode: strip, reason }`.
  - **M2 chunked path**: `terrain.tool_action_started { mode: chunked }`, `terrain.terrain_carved { mode: chunked, dominant_material_id, dirty_chunks[] }`, `material.chunk_dirtied`, `terrain.tool_refused { mode: chunked, reason: out_of_range | material_<name> }`, `combat.projectile_expired { cause: terrain_hit }` (from projectile-vs-terrain collision).

## Common Pitfalls
- Refusal reason names ship with a stable vocabulary across both modes: `out_of_range`, `already_broken`, `unknown_target`, `material_metal_nohook`, `material_anchor`, `material_hazard`. Replay tooling parses these; do not change spelling without also bumping a schema fixture.
- `try_carve`'s first pass scans for any refusal-reason pixel inside the carve circle and short-circuits the entire dig if found — even when the majority of the circle is diggable. This is the documented teaching path: the player learns to position digs away from anchors / metal_nohook / hazard. M5.6 may relax to partial-carve semantics.
- `try_blast`'s `force >= aff.hardness` check uses `f32::INFINITY` for refusal-only materials (metal_nohook, anchor, hazard). hazard previously had `hardness = 0.0` which made any non-negative force clear it; that asymmetry was Devin BUG_pr-review-job (flag) and is now `f32::INFINITY` to match the other refusal-only materials.
- `material_at_world` subtracts the terrain anchor before the lookup; production code MUST use it (NOT `material_at(floor(world_x), floor(world_y))`) to honor non-(0, 0) anchor scenarios. Bugbot 864084a2 caught this in the projectile-vs-terrain check.
- Chunked storage is sparse: a chunk that fully matches `default_material` is reclaimed (removed from the BTreeMap) automatically by `set_pixel_internal` after each write. `material_at` returns the default for unallocated chunks.
- `BreachStrip` HP is a `f32` because hardness is a `f32`; the engine quantizes through `quantize` (×1024 → i32) for the checksum so per-pixel resolution is plenty.

## Source Trail
- spec/prototype-roadmap §M1.5 — Micro Breach Fun Slice (M1.5-003 temporary soft breach).
- spec/prototype-roadmap §M2 — Pixel Terrain And Materials (M2-001..M2-008 chunked grid + launch materials + carve/blast/refuse).
- spec/native-implementation-backlog M1.5-003 + M2-001..M2-008.
- DR-007 (terrain/material model, OPEN — launch-material set frozen at BP2 with `material_schema_version="cf-terrain-launch-v1"`).
- DR-036 (systemic material simulation direction — implementation specifics defer to M5.6).
- corefall/docs/implementation-log/2026-05-08-bp2-terrain-replay-build.md.
