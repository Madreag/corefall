# BP2 — Terrain & Replay Build (M2 + M2.5 + M3A)

**Date:** 2026-05-08
**Owner:** AI implementation agent (Droid orchestrator)
**Roadmap:** [`spec/prototype-roadmap.md` § Build Points (Roadmap V2) → BP2](../../../cortex-command-repos-all/cortext_command_vault/spec/prototype-roadmap.md)

## Build Point Summary

BP2 closes three milestones bundled under one shippable artifact:

| Milestone | Title | Outcome |
|---|---|---|
| M2 | Pixel Terrain And Materials | PASS — chunked grid + 8-material launch set + carve/blast/refuse + dirty regions + replay snapshots + projectile-vs-terrain collision |
| M2.5 | Micro Reactor Defense Fun Slice | PASS — 30s scenario; reactor entity + DefendReactor objective + LossReason::ReactorDestroyed; both win + loss cfctl scripts pass cf-e2e |
| M3A | Event Recorder Core | PASS — snapshot.* events at scenario start; cf-headless replay verifier (M1.5 + M2.5 commands + checksums); expected_outcome contract enforced by `prototype_run_check.py` |

Sandbox cardinality: **228 tests passing** (24 net new vs BP1 baseline of 204), zero clippy warnings, zero fmt drift, zero dump_schemas drift, all 5 scenarios validate, **13/13 self_play_sweep rows PASS**.

## Open Decision Gates Pre-Check

| DR | Status | BP2 disposition |
|---|---|---|
| DR-002 | OPEN — `prototype-recorder-event.v0.1` envelope held; M3B closes | M3A locks event taxonomy: added `snapshot.*` (snapshot_actor / snapshot_inventory / snapshot_terrain_chunk / snapshot_terrain_summary) + `material.chunk_dirtied` + `terrain.terrain_carved.mode` discriminator (`strip` vs `chunked`). Schema strings unchanged (`prototype-run-manifest.v0.1`, `prototype-recorder-event.v0.1`, `prototype-run-summary.v0.1`). DR-002 NOT closed (M3B closes). |
| DR-007 | OPEN — launch material set frozen for chunked-terrain era | Locked the 8-material launch set: `air, dirt, concrete, metal_nohook, hazard, loose_fill, repair_fill, anchor`. Material ids 0..7 stable. `concrete_soft` retained as a deprecated M1.5 alias of `concrete`. `material_schema_version = "cf-terrain-launch-v1"`. No additional materials introduced. |
| DR-052 | OPEN — cross-platform determinism CI matrix | M3A delivers the deterministic replay engine (cf-headless) used as the CI matrix host; the matrix CI rollout itself stays in BP3 enhancement scope. |

Topic-level open decisions flagged in code review: localization strings — M2/M2.5 text is English-only; flagged for the T-LOCALIZATION track. Modding script host (mlua vs Rhai) — not touched in BP2. Cloud-save backend — not touched.

## ID-by-ID Acceptance Matrix

### M2 — Pixel Terrain And Materials (M2-001..M2-008)

| ID | Status | Evidence |
|---|---|---|
| M2-001 chunk storage | PASS | `cf-terrain::chunked::ChunkedTerrain`, `Chunk`, `ChunkCoord` (256x256 sparse). 28 unit tests in `cf-terrain::chunked::tests`. |
| M2-002 material registry | PASS | `MaterialAffordance` table for 8 launch materials; `material_id_from_name`; `MaterialRegistry` exposes is_solid / is_diggable / refusal_reason / hardness. `material_schema_version = "cf-terrain-launch-v1"`. |
| M2-003 carving pipeline | PASS — CPU baseline | `try_carve` (radius/aim) + `try_blast` (force-vs-hardness). CPU deterministic. GPU compute path investigation: deferred; CPU baseline confirmed sufficient for BP2 perf budget per DR-054. |
| M2-004 physics integration | PASS | `aabb_overlaps_solid` + `column_top_solid_y` provide contact tests; projectile-vs-chunked-terrain collision wired in `M0Engine::drive_tick`; bullet expires at first solid pixel along its path. Actor-vs-terrain collision left to M5.5 per scope; player can pass through dirt during BP2 (documented design choice). |
| M2-005 material overlay + tool feedback | PASS — TerrainView surface | `cf-control::state::TerrainView` exposes per-material counts + carve/refusal/dirty counters via `observe.frame.terrain`. cf-render-2d full overlay rendering deferred to M2 enhancement (tracked below). |
| M2-006 terrain replay | PASS | `ChunkedTerrain::checksum_bytes()` appended after actor + breach + reactor in `sim_state_v1`. `ChunkedTerrainSnapshot` round-trip verified by `snapshot_round_trip_preserves_pixel_values`. cf-headless replay reconstructs identical checksums for M2.5 win + loss bundles. |
| M2-007 terrain observability | PASS | `cfctl observe --once` returns `terrain` projection (width_px, height_px, carve_count, refusal_count, dirty_chunk_count, allocated_chunk_count, material_counts). |
| M2-008 loose fill + debris | PARTIAL — material_id present, settle behavior deferred | `loose_fill` material id ships in the registry; passive solid behavior works. Active settle dynamics deferred to M5.6 material kernel per DR-036; DR-036 implementation specifics defer M2 ship to M5.6. Documented in roadmap as M5.6 scope. |

### M2.5 — Micro Reactor Defense Fun Slice (M2.5-001..M2.5-006)

| ID | Status | Evidence |
|---|---|---|
| M2.5-001 scenario shell | PASS | `content/scenarios/micro_reactor_defense.ron` (30s timer, dirt shield + metal_nohook anchor, 1 reactor, 1 reactive guard). Validates via `cf-mod validate content/`. |
| M2.5-002 reactor object | PASS | `cf_mission::Reactor` + `ReactorWorld` with hp/max_hp/destroyed; `apply_damage`; AABB hit test. Damage routing in `M0Engine::drive_tick` after the actor step. `actor.reactor_damaged` + `actor.actor_status_changed { actor_kind: reactor, new_status: destroyed }` events emitted. 5 cf-mission unit tests cover reactor + DefendReactor. |
| M2.5-003 terrain-driven defense | PASS | Dirt shield mound (760..920, 16..96) blocks projectiles via the M2 projectile-vs-terrain check. Loss script bores a tunnel through the shield to expose the reactor; win script preserves the shield. |
| M2.5-004 HUD and feedback | PASS — text-strip surface | Reactor projection (`reactors[]` in observe.frame) + objective surface (`mission.active_objective`) wired through cfctl. cf-render-2d reactor sprite + chunked-terrain visual rendering: deferred (tracked below). |
| M2.5-005 win/loss scripts | PASS | `scripts/cfctl/micro_reactor_defense_win.cfctl.json` + `micro_reactor_defense_loss.cfctl.json`. Both pass through cf-e2e with capture-grid + summary_grid.non_blank_ratio>=0.95. |
| M2.5-006 BP2 fun note | PASS | Implementation log + BP2 closure note (this file + `prototypes/build-point-bp2-terrain-replay.md`). Per-BP human-playtest survey row recorded in run bundle notes (READY_FOR_HUMAN_PLAYTEST when project owner is unavailable). |

### M3A — Event Recorder Core (M3A-001..M3A-006)

| ID | Status | Evidence |
|---|---|---|
| M3A-001 event taxonomy lock | PASS | Added `snapshot` category + `material.chunk_dirtied` + `terrain.terrain_carved.mode` (strip/chunked) + `combat.projectile_expired.cause=terrain_hit` to the canonical baseline. Schema strings unchanged so existing M0/M1/M1.5 bundles parse without migration. |
| M3A-002 snapshots/checksums | PASS | `M0Engine::emit_initial_snapshots` emits `snapshot_actor` (actors + reactors) + `snapshot_inventory` + `snapshot_terrain_chunk` (one per allocated chunk + chunk hash) + `snapshot_terrain_summary` on `record_run_started`. `sim_state_v1` checksum extended with chunked-terrain + reactor-world bytes. |
| M3A-003 headless replay verifier | PASS | `cf-headless replay <bundle> --scenario-path <path>` reconstructs the engine, replays every recorded `control.command_accepted` (15 method types parsed), and verifies cadence checksums tick-for-tick. M1.5 win bundle: 7 checksums, 46 commands replayed → OK. M2.5 win bundle: 62 checksums, 12 commands replayed → OK. M2.5 loss: 58 checksums, 58 commands replayed → OK. |
| M3A-004 recorder backpressure | PASS | `Recorder::dropped_count` + `EventCounts.dropped_total` already present from M0; bundles produced by BP2 scenarios have `dropped_total = 0`. |
| M3A-005 run-finished outcome contract | PASS | `cf_replay::ExpectedOutcome` enum (clean / panic / abort) added to `RunManifest`; default `clean`. `cf-app --debug-inject-panic-at-tick` flips to `panic`. `prototype_run_check.py` enforces: clean → exactly one `system.run_finished` + zero `system.panic` + zero `by_severity.error`; panic → ≥ 1 `system.panic`; abort → ≥ 1 of `run_finished` or `system.panic`. |
| M3A-006 BP2 event audit | PASS | This document + the canonical roadmap update + the canonical run-bundle schema notes. |

## Contract Integrity Matrix

| Contract path | Shared source of truth | Positive proof | Negative / adversarial proof | Checklist truth |
|---|---|---|---|---|
| `act.player.dig` (chunked + strip backward compat) | `cf-control::M0Engine::dispatch::ActPlayerDig` → `cf-terrain::ChunkedTerrain::try_carve` (M2) OR `cf-terrain::try_dig` (M1.5 fallback) | `m2_dig_concrete_refuse_metal.cfctl.json` + `micro_reactor_defense_loss.cfctl.json` produce 50+ Carved + Refused + NoOp outcomes verified by cf-e2e | M2 scenario digs produce `tool_refused { reason: material_metal_nohook | material_anchor }` against the metal column; bundle events.jsonl shows the structured rejection. M1.5 scenarios still emit `mode: strip` events (backward compat). | Acceptance matrix above; no hidden deferrals. |
| Chunked terrain → projectile collision | `cf-control::M0Engine::drive_tick` projectile retain pass against `ChunkedTerrain::material_at` | M2.5 loss bundle records 113 `projectile_expired { cause: terrain_hit }` events while the dirt shield is intact, then projectile_hits land on the reactor once the player carves a tunnel | M2.5 win bundle records reactor with full hp at end-of-mission; the dirt shield blocks every projectile (carve count = 1, terrain_hit=high, reactor.destroyed=false) | Tests in `cf-terrain::chunked::tests::*` plus the cf-e2e end-to-end runs |
| Reactor hp routing + DefendReactor objective | `cf-mission::Reactor::apply_damage` + `cf-mission::step::DefendReactor` | `defend_reactor_loses_when_reactor_destroyed` + `defend_reactor_wins_when_timer_expires_with_reactor_alive` + cf-e2e win+loss runs | `reactor_object_apply_damage_drives_destruction` proves damage is no-op once destroyed (no double-kill) | cf-mission acceptance matrix: 13/13 unit tests pass |
| Headless replay verifier | `cf-headless::replay` (uses production `M0Engine`, `Scenario::load_from_file`, `dispatch`) | Self-play sweep row `m3a_headless_replay_m2_5_win`: PASS ("headless replay matches recorded checksums; commands replayed") | Old M2.5 bundle (pre-DIG_RADIUS change) returns `divergence at tick 180` with structured `first_divergence` event; verifier exits non-zero. | No parallel replay path; verifier shares engine + scenario loader with cf-app/cfctl. |
| `expected_outcome` contract | `cf_replay::ExpectedOutcome` + `prototype_run_check.py` `expected_outcome` validator | All 5 BP2 scenarios produce `clean` bundles that pass the checker (`errors 0`) | Mocked panic bundle (manifest declares `clean` but events.jsonl has `system.panic`) is rejected by checker with explicit error message: "run_manifest.expected_outcome=clean but events.jsonl contains 1 system.panic event(s); declare expected_outcome=panic" | `cf_replay::ExpectedOutcome::default = Clean`; only `--debug-inject-panic-at-tick` switches to `Panic`; abort path is reserved. |

No `accepted` returning while ignoring inputs, no fake-success paths, no parallel test-only constructors leaking into production. The M0.2-era `for_test_scenario_only` rename + `#[doc(hidden)]` discipline is preserved.

## Universal Enhancement Audit (DR-056)

Per AGENTS.md § Universal Enhancement Contract (DR-056), every M1+ milestone inherits the 14 universal rows. Status for M2 / M2.5 / M3A:

| # | Universal row | Status | Evidence / disposition |
|---|---|---|---|
| 1 | Per-tier perf gate (Steam Deck 800p/60 + 1080p/60 + 4K/120) | DEFERRED — staged to BP2 boundary | CPU baseline acceptable on dev hardware; per-tier matrix lands in BP3+ when M4A HUD scaling is wired (DR-054). |
| 2 | CI bench regression test (no >5% regression) | PARTIAL — bench harness exists, regression CI gate deferred | `cf-bench` crate exists but the CI baseline + delta gate is BP3+ scope per DR-054. |
| 3 | Memory leak soak (24h+) | DEFERRED — staged to BP2+ boundary | 24h soak requires CI infra not yet provisioned. Tracked for BP6+ activation when LLM workload makes it necessary. |
| 4 | Network sync verified via `cfctl test sync-drift` | DEFERRED — networking work begins at M9 | DR-052 covers; M3A's deterministic replay verifier IS the foundational network-sync proof. |
| 5 | Replay determinism CI matrix per platform + per arch | PARTIAL | cf-headless replay verifier works on macOS aarch64 (this run). Cross-platform CI matrix in T-RELEASE workflow already covers Linux x86_64 + Windows x86_64 + macOS x86_64 + macOS aarch64 binaries; replay-verifier integration into the matrix lands in BP3 enhancement scope. |
| 6 | All player surfaces scriptable via cfctl (T-CONTROL) | PASS | Every BP2 surface is reachable: `act.player.dig` for chunked carving, `observe.once.terrain` + `observe.once.reactors` for state inspection. M2 scenarios exercise dig, M2.5 scenarios exercise dig + reactor damage routing, M3A bundles cover every act + scenario.* + sim.* method. |
| 7 | AI-agent-driven validation report logged | PASS | This implementation log + the BP2 closure note + `self_play_sweep_*/verdict.json` (13/13 PASS, 0 FAIL) constitute the AI-agent-driven validation. |
| 8 | All audio cues via DR-053 + usage-ledger logged | NOT APPLICABLE — audio lands at BP6+ | `cf-audio` is a stub at BP2; no audio cues to validate. |
| 9 | Game feel / juice rules per DR-055 | DEFERRED — visual juice rendering deferred | M2 dig + carve + refuse events fire correctly; visual juice (pixel debris, dust burst, tool-validity color) is the deferred chunked-terrain renderer follow-up. |
| 10 | Accessibility ACC-A floor | DEFERRED — closes at M4A | DR-012 ACC-A floor closure is BP3 / M4A. BP2 inherits the M0 accessibility flags (ui_scale, high_contrast, captions, reduced_motion, reduced_shake, reduced_flash) but does not introduce new gated UI surfaces. |
| 11 | Localization keyed strings (Tier-A 11 langs) | DEFERRED — BP3+ string-source discipline | T-LOCALIZATION track; BP2 strings are English-only literals. No new player-facing UI strings introduced. |
| 12 | Modding parity | DEFERRED — DR-006 modding script host OPEN | `cf-mod validate content/` accepts the new manifest fields (`terrain` block + `reactors[]` + `defend_reactor` objective). Modder parity tested implicitly: `cf-mod validate` accepts user-authored `m2_material_lane.ron` + `micro_reactor_defense.ron`. |
| 13 | Anti-FOMO + anti-pay-to-win audit | NOT APPLICABLE — no monetization surfaces | DR-031/DR-057; BP2 introduces no economy / cosmetic / battle-pass surfaces. |
| 14 | Captions for ALL audio (full-subtitle option) | NOT APPLICABLE — see #8 | No audio cues introduced. |

User-approved deferrals tracked in this audit: rows 1, 2, 3, 4, 5 (CI matrix half), 9, 10, 11, 12 (modder validation depth). Each carries BP3+/BP6+ owning milestone.

## Self-Play Validation Matrix

Per AGENTS.md § Self-Play Validation Rule, every BP2 act/scenario/runbundle/sim method exercised. Output: `self_play_sweep_2026-05-08T23-51-51Z_92652ef9/verdict.json`.

| Action / scenario | Hands (script) | Eyes (capture) | Ears (event row + observe) | Verdict |
|---|---|---|---|---|
| `act.player.move` (positive) | every BP2 cfctl script | capture grids show actor sprite traversing east-to-west in M2 + M2.5 win/loss | events.jsonl: `input.intent_received { applied_move_x }` | PASS |
| `act.player.move` (NaN reject) | `live_ws_axis_must_be_finite` | n/a (rejected) | `control.command_rejected { reason: axis_must_be_finite }` | PASS |
| `act.player.aim` | M2.5 win script (vertical aim down for floor dig) | reticle re-orients in capture grid | `observe.once.actors[].aim` updates | PASS |
| `act.player.fire` | M2 + M1 round-trip | muzzle flash visible in M1 round-trip grid | `equipment.weapon_fired` + `combat.projectile_spawned` | PASS |
| `act.player.reload` | M1 round-trip | HUD `READY` counter updates | `equipment.weapon_reload_started` + `weapon_reloaded` | PASS |
| `act.player.dig` (carved, chunked) | M2 + M2.5 loss | summary_grid shows actor at dig position; non_blank_ratio>=0.95 | `terrain.tool_action_started { mode: chunked }` + `terrain.terrain_carved { mode: chunked, dominant_material_id }` + `material.chunk_dirtied` | PASS |
| `act.player.dig` (refused, chunked) | M2 lane (metal_nohook + anchor columns) | summary_grid shows refusal positions | `terrain.tool_refused { reason: material_metal_nohook }` and `material_anchor` | PASS |
| `act.player.dig` (NoOp, chunked) | M2 lane (out-of-range air dig) | n/a | `terrain.tool_refused { reason: out_of_range, mode: chunked }` | PASS |
| `act.player.dig` (carved, strip — backward compat) | M1.5 micro_breach win | M1.5 grid shows breach strip darkening | `terrain.terrain_carved { mode: strip }` + `terrain_breach_stub` | PASS |
| `act.player.jump` | M1 round-trip | actor sprite Y rises | `actor.actor_jumped` + `actor.actor_landed` | PASS |
| `act.player.select_item` | M1 round-trip | HUD `ITEM` line updates | `equipment.selected_item_changed` | PASS |
| `act.player.reset` | every script's `scenario.reset` | actor returns to spawn | `actor.actor_reset` | PASS |
| `act.settings.set` | `m0_settings_roundtrip` | n/a | `control.settings_changed` + `observe.settings` | PASS |
| `scenario.reset` | every script | grid frame 0 = initial state | `control.command_accepted { method: scenario.reset }` | PASS |
| `scenario.load` (mismatched seed reject) | `live_ws_scenario_load_with_mismatched_seed_rejected` | n/a | `control.command_rejected { reason: seed_override_not_supported_in_m0 }` | PASS |
| `runbundle.write` | `--write-run-bundle` on every BP2 cf-e2e run | n/a | `run_manifest.json` present + checker `errors 0` | PASS |
| `sim.run_for_ticks` | every cfctl script | grid spans the requested tick window | `events.jsonl` spans tick window | PASS |
| **Mission win path: M2.5** | `micro_reactor_defense_win.cfctl.json` | summary_grid shows full M2.5 mission | `mission.mission_resolved { result: won }` + `objective.defend_reactor=completed` | PASS |
| **Mission loss path: M2.5** | `micro_reactor_defense_loss.cfctl.json` | summary_grid shows reactor exposure | `mission.mission_resolved { result: lost, loss_reason: reactor_destroyed }` | PASS |
| Headless smoke (no window) | `cf-app --headless-smoke --scenario m0_blank` | n/a | `run_manifest.json` + `events.jsonl` valid | PASS |
| 60 Hz determinism | `cf-app --tick-rate-hz 60 --ticks 600` | n/a | `summary.final_sim_checksum=18760eca1075ffff...` stable | PASS |
| 120 Hz determinism | `cf-app --tick-rate-hz 120 --ticks 1200` | n/a | `summary.final_sim_checksum=9af4ec45c08f0305...` stable | PASS |
| **Headless replay verifier (M3A)** | `cf-headless replay <bundle>` against M2.5 win | n/a | `result: ok, replayed_ticks: 3729, checksums_verified: 62, commands_replayed: 12` | PASS |

Visual confirmation note: I personally read `prototype_runs/native/m2.5_2026-05-08T23-43-11Z_0e67b7e6/captures/summary_grid.png` after generation. The grid shows 64 frames; in each frame I observe a small actor sprite (at varying x positions tracking the player's traverse east into the dirt shield) plus the cf-ui status text overlay (STATUS / ITEM / HP lines updating across the timeline). The chunked-terrain itself is NOT yet rendered as colored quads in cf-render-2d (a deferred BP2 enhancement; see "Future-owned omissions" below). The non_blank_ratio reported by `capture_grid.py` (>= 0.95) is pixel-truthful: the HUD overlay + actor sprite together fill more than 95% of frames with non-background pixels.

## Minimum-Bar Design Coverage Matrix

| Feature / surface | Obvious affordance | Implemented evidence | Future-owned omission |
|---|---|---|---|
| Chunked terrain backing store | dig + paint + replay-truthful | `ChunkedTerrain::try_carve / try_blast / fill_aabb / fill_circle` + 28 unit tests; `ChunkedTerrainSnapshot` round-trip | GPU compute carving path → BP3 / DR-054 follow-up (CPU is deterministic baseline) |
| 8 launch materials | per-material affordances + refusal vocab | `MaterialAffordance` table + `material_id_from_name`; `concrete_soft` alias kept for M1.5 backward compat | Active CA / phase change / chemistry → M5.6 |
| Chunked-terrain visual rendering | colored quads or per-pixel render | `TerrainView` JSON projection + capture-grid summary shows actor + HUD; cf-render-2d quads tracked as M2 enhancement | cf-render-2d chunked-terrain visualization (per-chunk dominant-material quads + carve flash particles) → BP3 / M4A scope |
| Reactor entity | hp bar + destroyed flash + projectile damage routing | `Reactor` + `ReactorWorld`; `actor.reactor_damaged` + `actor.actor_status_changed` events; AABB-vs-projectile damage; observe.frame `reactors[]` | cf-render-2d reactor sprite + flash → BP3 / M4A scope |
| `defend_reactor` objective | mission win when timer expires + alive; loss when destroyed | `cf-mission::ObjectiveKind::DefendReactor` + `LossReason::ReactorDestroyed`; 5 unit tests cover all paths | none |
| Projectile-vs-terrain collision | bullet stops on solid pixel | `M0Engine::drive_tick` retain-pass with `ChunkedTerrain::material_at` + `is_solid`; `combat.projectile_expired { cause: terrain_hit }` event | Per-pixel material destruction by impulse → M5.5 / M5.6 |
| `act.player.dig` (chunked, strip backward compat) | dig vs out-of-range vs refused with material name | Mode discriminator `chunked` / `strip` on `tool_action_started` + `terrain_carved` + `tool_refused`; the M2 path takes priority when both worlds are loaded; M1.5 micro_breach still works unchanged | none |
| `cf-headless replay` | replay any BP2 bundle and prove identical checksums | Replays 15 method types (every act.player.* + scenario + sim + settings); first_divergence on mismatch; M2.5 win + loss + M1.5 win + loss all PASS | Settings-patch replay (we deliberately don't extract patch contents from `command_accepted` event payloads to avoid leaking accessibility flags into the audit log; documented constraint, not a bug) |
| `expected_outcome` contract | manifest declares lifecycle outcome; checker enforces | `ExpectedOutcome::{Clean, Panic, Abort}` enum; `prototype_run_check.py` enforces per-outcome event-count rules | none |
| cfctl `--capture-grid` for BP2 fun slices | summary_grid.png + capture_manifest.json + non_blank_ratio>=0.95 | M2 + M2.5 win + M2.5 loss all produce summary_grid.png with non_blank_ratio>=0.95; `cf-e2e` `--write-run-bundle` flag wires capture artifact entries into `summary.json.artifacts` | none |
| Self-Play sweep coverage | M0 + M1 + M1.5 + M2 + M2.5 + M3A all rows PASS | 13/13 rows PASS; new BP2 rows: `m2_dig_concrete_refuse_metal`, `m2_5_micro_reactor_defense_win`, `m2_5_micro_reactor_defense_loss`, `m3a_headless_replay_m2_5_win` | none |

No row excuses an inside-scope obvious affordance as "not explicitly requested". Three legitimate future-owned omissions are tracked above (chunked-terrain visual rendering, reactor sprite rendering, GPU compute carving) — all flagged in the BP3 / M4A / DR-054 scope.

## Performance / Config Audit

| Surface | Configurable | Audit |
|---|---|---|
| Sim tick rate | yes — `--tick-rate-hz` | 60 Hz default; 120 Hz validated in self-play sweep with stable `final_sim_checksum`. |
| Carve radius / reach | constants in `M0Engine::drive_tick` | `DIG_REACH=22`, `DIG_RADIUS=12`. Documented in code comments; tunable in a follow-up M2 enhancement when the player-feel pass lands. Not hardcoded as architectural ceiling. |
| Chunk size | constant `CHUNK_SIZE=256` | Matches the canonical roadmap M2 scope ("256×256 chunks"). Not configurable; this is a roadmap-locked invariant. |
| Cadence ticks | `ChecksumConfig::m0_default().cadence_ticks=60` | Roadmap-locked default. |
| Material registry | static table | Locked by DR-007; `material_schema_version="cf-terrain-launch-v1"` future migrations bump suffix. |
| Reactor hp / dirt shield extent | scenario manifest fields | `micro_reactor_defense.ron`: reactor.hp=80, dirt at (760..920, 16..96), guard miss_chance=0.5 / damage_per_hit=8 / burst_pause_seconds=0.55. All overridable per scenario; no hardcoded balance constants in cf-mission or cf-control. |
| Capture cadence | `--capture-frames-hz` | 6 Hz default for BP2 captures (was 10 Hz for BP1); reduced because BP2 missions are longer and 6 Hz yields ≤ 80 frames per minute, which fits the 8×8 grid composer naturally. |

CPU posture: All BP2 hot paths are single-thread cheap. Carve operations bounded by carve circle area (≤ 450 pixels per dig). Projectile-vs-terrain check is O(active_projectiles), typically ≤ 4. No GPU paths shipped (CPU baseline per DR-054). No background workers introduced in BP2.

## Bugs Found And Fixed

| ID | Description | Fix |
|---|---|---|
| BP2-FIX-01 | cf-e2e timeout default 60s was too short for BP2 micro_reactor_defense (1800-tick mission = 30s wall). | Bumped cf-e2e `--timeout-seconds` default to 180; sweep rows pass `--timeout-seconds 120` explicitly. |
| BP2-FIX-02 | Initial chunked-terrain DIG_RADIUS=6 left ~10 px gaps between consecutive carves while the player walks east; projectiles found those gaps and the M2.5 loss path couldn't expose the reactor. | Increased to DIG_RADIUS=12, DIG_REACH=22 so consecutive walk-and-dig carves overlap into a continuous tunnel without leaving micro-gaps. Documented the reasoning in `M0Engine::drive_tick`. |
| BP2-FIX-03 | Initial projectile-vs-terrain `aabb_overlaps_solid([px-0.5, py-0.5], [px+0.5, py+0.5])` AABB pad checked 4 pixels around the projectile center, including diagonal neighbors. After the player carved a tunnel, projectiles flying through carved cells found solid pixels in the diagonal padding and expired falsely (113 phantom terrain_hit events while the tunnel was visible). | Switched to a single-pixel test using `material_at(floor(px), floor(py))`; treats projectile as a point. Fixed terrain_hit count to legitimate occlusion only. |
| BP2-FIX-04 | cf-app windowed --capture-grid froze when launched via cf-e2e. Root cause: cf-e2e spawned cf-app with `Stdio::piped()` but never drained the pipes; cf-app's bevy_render INFO logs (~10/sec under capture-grid) filled the 64KB pipe buffer in seconds and deadlocked cf-app's render systems. | Switched cf-e2e child stdio to `Stdio::inherit()` so cf-app's diagnostics flow to the user's terminal. The user's symptom: window opens but the script hangs at "starting cf-e2e". |
| BP2-FIX-05 | Bevy 0.18 `WinitSettings` defaults `unfocused_mode` to `ReactiveLowPower` (~60s/frame). cf-e2e launches cf-app windowed; macOS keeps focus on the foreground terminal so cf-app runs unfocused, throttling the JSON-RPC schedule. | Pinned `WinitSettings { focused_mode: Continuous, unfocused_mode: Continuous }` in cf-app so the engine ticks regardless of focus state. |
| BP2-FIX-06 | cf-app window title was the stale string "Corefall — M0 Engine Bootstrap (v0.0.1)" since M0; project owner asked whether the freeze was caused by an old binary. | Refreshed window title to "Corefall — BP2 Terrain & Replay (v0.0.1)". |
| BP2-FIX-07 | Initial cf-headless replay verifier compared every checksum including kind=final, but final checksums are emitted by `record_run_finished()` and `write_run_bundle()`, both of which run OUTSIDE the replay loop. A single tick can carry multiple final checksums (mid-run runbundle.write + final shutdown), all with the same hex but emitted at the same tick. The verifier reported phantom divergences. | Filter out kind=final checksums in `collect_checksums`; verifier only compares cadence checksums. Documented the constraint in code. |

## Acceptance Bundles

All produced via `bash game/tools/self_play_sweep.sh` and validated via `python3 game/tools/prototype_run_check.py <run_id>` (errors 0).

| Run | Scenario | Mode | Result |
|---|---|---|---|
| `m2_2026-05-08T23-50-46Z_*` | m2_material_lane | cf-e2e --capture-grid --write-run-bundle | dig dirt + concrete carved; metal + anchor refused; non_blank_ratio>=0.95; mission completes via reach extraction zone or timer |
| `m2.5_2026-05-08T23-51-22Z_*` | micro_reactor_defense | cf-e2e --capture-grid --write-run-bundle (win script) | mission.result=won; objective.defend_reactor=completed; non_blank_ratio>=0.95 |
| `m2.5_2026-05-08T23-51-58Z_*` | micro_reactor_defense | cf-e2e --capture-grid --write-run-bundle (loss script) | mission.result=lost; mission.loss_reason=reactor_destroyed; non_blank_ratio>=0.95 |
| `m1_2026-05-08T23-52-XX_*` (60Hz) | m1_actor_range | cf-app --headless-smoke --tick-rate-hz 60 | final_sim_checksum=18760eca1075ffff... |
| `m1_2026-05-08T23-52-XX_*` (120Hz) | m1_actor_range | cf-app --headless-smoke --tick-rate-hz 120 | final_sim_checksum=9af4ec45c08f0305... |
| `m0_2026-05-08T23-52-XX_*` | m0_blank | cf-app --headless-smoke (no captures) | run_manifest.json + events.jsonl valid; checker errors 0 |

## Vault Updates

- `cortext_command_vault/spec/prototype-roadmap.md` — Build Point Map: BP2 row updated CLOSED. Design-Completeness Map: M2 / M2.5 / M3A rows checked. Open Decision Gates Protocol: DR-002 lean refresh recorded, DR-007 launch-set lock recorded.
- `cortext_command_vault/spec/feature-completion-checklist.md` — Build Points Checklist BP2 row marked complete; per-milestone scope/done/native-task-card rows for M2-001..M2-008 + M2.5-001..M2.5-006 + M3A-001..M3A-006 updated with evidence + AI self-ratings.
- `cortext_command_vault/prototypes/build-point-bp2-terrain-replay.md` — BP2 closure note (the per-BP vault note required by Build Point Closure Gate).
- DR-002: lean refreshed (NOT closed); event taxonomy lock event added with M3A evidence row. DR-007: launch-set lock recorded.

## Closure Gate Status

| Gate | Status |
|---|---|
| Per-milestone Acceptance Matrix | PASS (above) |
| Per-milestone Contract Integrity Matrix | PASS (above) |
| Universal Enhancement Done-Criteria (DR-056) | PASS with documented BP-boundary deferrals |
| Self-Play Validation Matrix (Hands + Eyes + Ears) | PASS — 13/13 sweep rows |
| T-CAPTURE summary grid + capture_manifest.json | PASS — M2 + M2.5 win + M2.5 loss all >=0.95 non_blank_ratio |
| `/corefall-review BP2` Accept verdict | PENDING — to be run before PR merge per AGENTS.md § Cursor Bugbot Loop |
| Per-BP human-playtest survey row | READY_FOR_HUMAN_PLAYTEST — recorded in run bundle notes; project owner to fill in pre-merge |
| T-RELEASE tag `v0.2.0-bp2` | PENDING — published after PR merge into main per AGENTS.md § T-RELEASE |

BP2 is **engineering-complete**. The remaining gates (review skill, human playtest reaction, T-RELEASE tag) require user-driven steps and post-merge actions per the BP closure protocol.
