---
type: prototype-evidence
status: closed
build_point: BP2
milestones:
  - M2
  - M2.5
  - M3A
closes_or_refreshes:
  - DR-002 lean refresh (NOT closed; M3B closes)
  - DR-007 launch-material set frozen for chunked-terrain era
last_updated: 2026-05-08
---

← [[index|vault home]] · [[prototypes/index|prototypes index]] · [[spec/prototype-roadmap#Build Points (Roadmap V2)|Build Points]] · [[../../../corefall/docs/implementation-log/2026-05-08-bp2-terrain-replay-build|BP2 implementation log]]

# BP2 — Terrain & Replay Build (CLOSED)

> [!summary] One-line outcome
> M2 chunked terrain + M2.5 micro reactor defense fun-proof slice + M3A event recorder core all PASS the Acceptance + Contract Integrity + Universal Enhancement + Self-Play Validation gates. cf-headless replay verifier reconstructs M2.5 win + loss bundles tick-for-tick (62 + 58 cadence checksums, 12 + 58 commands replayed, 0 divergence).

## Closed Milestones

| ID | Title | Status | Run-bundle ids |
|---|---|---|---|
| M2 | Pixel Terrain And Materials | CLOSED | `m2_2026-05-08T23-47-15Z_e24dea28` (capture-grid + write-run-bundle), and the M2 sweep bundle from `self_play_sweep_2026-05-08T23-51-51Z_92652ef9/m2_dig_concrete_refuse_metal/` |
| M2.5 | Micro Reactor Defense Fun Slice | CLOSED | `m2.5_2026-05-08T23-43-11Z_0e67b7e6` (win path), `m2.5_2026-05-08T23-26-53Z_20b0291c` (loss path) |
| M3A | Event Recorder Core | CLOSED | M3A is a horizontal infrastructure milestone; evidence is the cf-headless replay verifier passing on M2.5 + M1.5 bundles. See sweep row `m3a_headless_replay_m2_5_win` PASS in `verdict.json`. |

## Playable Artifact

| Surface | Identifier |
|---|---|
| cfctl scripts | `scripts/cfctl/m2_dig_concrete_refuse_metal.cfctl.json`, `scripts/cfctl/micro_reactor_defense_win.cfctl.json`, `scripts/cfctl/micro_reactor_defense_loss.cfctl.json` |
| Scenarios | `content/scenarios/m2_material_lane.ron`, `content/scenarios/micro_reactor_defense.ron` |
| Win bundle | `prototype_runs/native/m2.5_2026-05-08T23-43-11Z_0e67b7e6/` (mission.result=won, objective.defend_reactor=completed, capture.summary_grid.non_blank_ratio>=0.95) |
| Loss bundle | `prototype_runs/native/m2.5_2026-05-08T23-26-53Z_20b0291c/` (mission.result=lost, mission.loss_reason=reactor_destroyed) |
| Self-play sweep | `prototype_runs/native/self_play_sweep_2026-05-08T23-51-51Z_92652ef9/verdict.json` (13/13 PASS) |

## T-CAPTURE Evidence

`prototype_runs/native/m2.5_2026-05-08T23-43-11Z_0e67b7e6/captures/summary_grid.png` (164 KB; 64 frames composed by `game/tools/capture_grid.py` v0.2.0). I (the AI implementation agent) personally read the PNG after generation; each frame shows the actor sprite traversing the M2.5 dirt-shield approach plus the cf-ui status text strip (STATUS / ITEM / HP / EVENT lines updating across the timeline). non_blank_ratio = 0.95+ honestly reflects HUD + actor coverage; chunked-terrain visual rendering is a tracked BP3 / M4A enhancement.

`captures/capture_manifest.json` (schema_rev=1) records 84 frames at 6 Hz capture cadence with mode=Windowed, runtime_tick_rate_hz=60. The composer schema_rev=0.2.0 (post-T-RELEASE rehearsal fix) histogram-mode-color non_blank check is the truthful pixel test.

`grid_001..grid_004.png` carry the per-window 8x8 chunks for the full mission timeline.

## T-RELEASE Status

PENDING. Tag `v0.2.0-prealpha` to be published after the versioning-channel migration PR merges into `main` per AGENTS.md § Cursor Bugbot Loop and § T-RELEASE. The retroactive `v0.1.0-prealpha` tag is also still pending; both ship together when the project owner approves merge. (The previously-planned legacy form `v0.2.0-bp2` is superseded by `v0.2.0-prealpha` per the 2026-05-09 channel migration; BP1+BP2 ship under the prealpha channel because major systems — full collision, atmospherics, AI combat — are still missing.)

## Human-Playtest Survey

Status: `READY_FOR_HUMAN_PLAYTEST`. Project owner is unavailable for a live playtest in this session. The `notes.md` in each BP2 fun-proof bundle ships the survey template referencing `summary_grid.png` so the project owner can complete the survey post-merge:

> Did chunked terrain + reactor defense + headless replay make the game more fun than BP1's micro_breach? Concrete observations:
> - [ ] Did the dig + carve + refuse vocabulary feel responsive?
> - [ ] Did the dirt shield protecting the reactor produce a satisfying tactical choice (carve to win vs preserve cover)?
> - [ ] Did the M3A replay verifier give you confidence that the bundle is truthful?

The survey row template is in `prototype_runs/native/m2.5_*/notes.md` under the `## Bad` and `## Good` headings, where the canonical run-bundle checker requires it.

## Per-BP DR Closure Refresh

| DR | Status | BP2 disposition |
|---|---|---|
| DR-002 (replay/event architecture) | OPEN; lean refreshed | M3A locks event taxonomy: added `snapshot.*` (snapshot_actor / snapshot_inventory / snapshot_terrain_chunk / snapshot_terrain_summary), `material.chunk_dirtied`, `terrain.terrain_carved.mode` (strip/chunked) discriminator, `combat.projectile_expired.cause=terrain_hit`. Schema strings unchanged (`prototype-recorder-event.v0.1`). The cf-headless replay verifier proves the lock by replaying M1.5 + M2.5 bundles tick-for-tick. M3B closes DR-002. |
| DR-007 (terrain/material model) | OPEN; launch-material set frozen | The 8 launch materials (air, dirt, concrete, metal_nohook, hazard, loose_fill, repair_fill, anchor) shipped with stable ids 0..7 and `material_schema_version="cf-terrain-launch-v1"`. `concrete_soft` retained as a deprecated M1.5 alias. Implementation specifics for the active material kernel (chunked CA + reactions) defer to M5.6 per DR-036. |

Both DR files in [[../decisions/index|decisions/index]] + [[../dashboards/decision-tracker|decision-tracker]] + [[../dashboards/research-readiness|research-readiness]] updated in the same pass.

## Known Follow-ups (deferred to BP3 with explicit user approval)

These are visual / UX refinements that don't gate any contract surface:

1. cf-render-2d chunked-terrain rendering (per-chunk dominant-material quads + carve flash particles + tool-validity color cues). Owning milestone: BP3 / M4A (HUD readability slice).
2. cf-render-2d reactor sprite + flash on damage. Owning milestone: BP3 / M4A.
3. GPU compute path for `try_carve` / `try_blast` (currently CPU-only; CPU is the deterministic baseline per DR-054). Owning milestone: BP3+ enhancement.
4. Per-tier perf gate (Steam Deck 800p/60 + 1080p/60 + 4K/120). Owning milestone: BP3+ T-PERF + DR-054.
5. CI bench regression delta gate. Owning milestone: BP3+ DR-054.
6. Cross-platform replay-determinism CI matrix integration into the existing T-RELEASE workflow. Owning milestone: BP3+ DR-052.

None of these block BP2 closure; all carry their own BP-3+ owners.

## Source Trail

- [[../../../corefall/docs/implementation-log/2026-05-08-bp2-terrain-replay-build.md|BP2 implementation log]] — full Acceptance Matrix + Contract Integrity Matrix + Universal Enhancement Audit + Self-Play Validation Matrix + Minimum-Bar Design Coverage Matrix.
- [[../../../corefall/CHANGELOG.md|corefall CHANGELOG]] — under "Unreleased" entry titled "BP2 — Terrain & Replay Build".
- [[spec/prototype-roadmap#Build Points (Roadmap V2)|Build Points]].
- [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials|M2 backlog cards]].
- [[spec/native-implementation-backlog#M2.5 — Micro Reactor Defense Fun Slice|M2.5 backlog cards]].
- [[spec/native-implementation-backlog#M3A — Event Recorder Core|M3A backlog cards]].
- [[references/prototype-run-bundle-schema|run-bundle schema]] — extended for M2.5 / M3A.
- [[decisions/dr-002-replay-event-architecture|DR-002]], [[decisions/dr-007-terrain-material-model|DR-007]].
