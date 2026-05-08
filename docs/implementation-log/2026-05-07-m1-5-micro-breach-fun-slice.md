# M1.5 — Micro Breach Fun Slice (implementation log)

Date: 2026-05-07 / 2026-05-08 UTC

## Scope

Implement [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] task cards M1.5-001..M1.5-006 from the canonical native backlog. M1.5 turns the M1 actor lab into a 60-90 second playable scenario: spawn west, dig a soft concrete wall, fight one reactive enemy, reach the eastern extraction zone within 90 seconds.

## Open Decision Gates pre-check

| DR | Status | Action |
|---|---|---|
| DR-002 | OPEN, lean=hybrid event log + snapshots | M1.5 adds `mission.*`, `terrain.*` and `ai.*` event categories on top of M1's `input/actor/equipment/combat/system/control/determinism`. No category was renamed; recorder envelope stays at `prototype-recorder-event.v0.1`. M3 closes the DR. |
| DR-004 | OPEN, lean=sequenced single-actor → squad → bunker breach | M1.5 implements the single-actor + soft-breach + extraction step. The lean is unchanged. M7 closes the DR. |
| DR-007 | OPEN (defers implementation specifics to DR-036) | M1.5 ships `concrete_soft` and `metal_nohook` material ids, both visible in `terrain.tool_refused` reason vocabulary. M2 swaps the strip for chunked terrain; M5.6 swaps the strip for the chunked CA kernel. The lean is unchanged. |
| DR-008 | OPEN, lean=hybrid jobs + utility scoring + scripted hooks | M1.5 implements EXACTLY the LEAN: a tiny scripted FSM (`Idle → Alerted → Engaged → Dead`) for the job layer, deterministic utility scoring for the tactic layer, and scripted aim-settle/miss-roll/burst pacing for the custom layer. No closure attempted; M6 closes the DR. |
| DR-009 | OPEN, lean=direct + slowdown overlay + optional tactical map | M1.5 introduces objective state (start/complete/fail/resolve) and surfaces it through the `mission.*` event category and a `MissionView` projection. No command overlay; M4 closes the DR. |

No materially-deviating change was needed; every implementation slot fell inside the existing LEANs. Per the Open Decision Gates Protocol, no user input was required to proceed.

## What changed

### New crates wired up (real implementations replacing M0 stubs)

- **`cf-mission`** (was a stub) — owns `Objective`, `ObjectiveKind` (`BreachBarrier`/`NeutralizeActor`/`ReachZone`), `ObjectiveStatus`, `LossConditions`, `MissionResult`, `MissionState`, `MissionView`, and the pure `step(state, inputs) -> MissionTickReport` driven by the engine each tick. Objectives advance one row at a time so the HUD always has a single Active row.
- **`cf-terrain`** (was a stub) — owns `BreachStrip`, `BreachWorld`, `DigRequest`, `DigOutcome`, `BreachView`, and `try_dig` with M2-compatible refusal vocabulary (`out_of_range`, `material_metal_nohook`, `already_broken`, `unknown_target`). Carving emits the same `terrain_carved` event payload shape M2 will produce.
- **`cf-ai`** (was a stub) — owns `ReactiveGuard`, `ReactiveGuardParams`, `GuardState`, `Tactic`, `step`, `EnemyTickReport`, `PerceptionRecord`, `TacticRecord`, `FireRecord`, `GuardStateTransition`, `ReactiveGuardView`. Encodes DR-008's three-layer LEAN: scripted job FSM + deterministic utility scoring + scripted aim-settle/miss-roll/burst pacing. RNG comes from `cf-sim-core::Rng` so replay parity holds.

### `cf-control` engine wiring

- `M0EngineConfig` extended with `initial_breach_world`, `initial_guards`, `initial_objectives`, `mission_loss`, plus matching builders in `for_loaded_scenario` so a scenario manifest with `breaches[]`, `objectives[]`, `mission`, and per-actor `enemy:` blocks builds a complete M1.5 engine config.
- `EngineMutable` extended with `breach_world`, `pending_dig`, `reactive_guards`, `mission`, `mission_started_at_tick`, `next_guard_projectile_id`. Reset rewinds every M1.5 sub-state.
- `drive_tick` extended to (1) process pending dig BEFORE the actor step so the breach can become broken on the same tick the dig landed, (2) tick reactive guards using the seeded RNG so miss rolls replay deterministically, (3) inject guard projectiles into the same projectile pool the actor step uses (swept hit detection runs against them on the next tick), (4) tick the mission state machine after the actor world settles.
- New `ControlCommand::ActPlayerDig { target, source }` variant + `act.player.dig` JSON-RPC method (with `ActPlayerDigParams { schema_version, target?: String }` and a generated JSON Schema).
- `M0Engine::actor_render_snapshot` extended with `BreachRenderView`, `MissionHudView`, and `ExtractionZoneView` so the cf-app bridge can paint breaches + mission HUD + the green extraction-zone box.
- `EngineHandle::snapshot` extended with `mission`, `breaches`, `enemies` projections in `ObserveFrame`. cfctl/AI agents see the same data the HUD does.
- New `build_mission_view`, `build_checksum_bytes` helpers. `sim_state_v1` now hashes actor state + breach state + reactive-guard state + mission objective statuses (append-only relative to M1; `_v1` suffix preserved per the canonical AGENTS.md note).
- `ScenarioReset` rewinds the breach world, every reactive guard, and the mission state machine atomically. Started-at-tick resets to the current engine tick so the timer measures from reset.
- **Race fix during M1.5 stabilization:** `EngineHandle::snapshot` previously read `tick` under the read lock then dropped the lock and recorded `observation_sent` AFTER. With M1.5's higher per-tick event count (~3 events per tick from input/AI/mission), drive_tick could acquire the write lock between the read and the record, producing non-monotonic `events.jsonl` ordering. Fix: record `observation_sent` BEFORE dropping the lock so the recorder sees a consistent timeline.

### Scenario + scripts

- New `content/scenarios/micro_breach.ron`. 1280×720 region, floor at y=16, gravity -980. Player at (96, 32) with rifle. Reactive guard at (900, 32) facing left, 80 hp, miss_chance 0.65, damage 8, burst pause 0.7 s. Two breach strips: `outer_wall` (concrete_soft, 60 hp) and `anchor` (metal_nohook, refusal). Three objectives: breach → neutralize → extract zone (1160-1280, 16-96). 90 s mission timer. `expected_tests: ["M1.5-SMOKE-01", "M1.5-WIN-01", "M1.5-LOSS-01"]`.
- New `scripts/cfctl/micro_breach_win.cfctl.json` — moves player east 160 ticks, digs three times, walks past the breach 30 ticks, fires 12 burst shots, walks to extraction. Total ~430 ticks.
- New `scripts/cfctl/micro_breach_loss.cfctl.json` — breaches the wall then stands still in the guard's sight cone for 800 ticks until the player dies.

### `cfctl`

- `act player-dig --target <id>?` subcommand routes to the new `act.player.dig` method.

### `cf-app`

- New `KeyCode::KeyG` keyboard binding routes through the same dispatch path as `cfctl act.player.dig`. Eyes/ears/hands rule preserved.
- `sync_actor_state_to_render` propagates breaches + extraction zone into `ActorRenderState` and mission/enemy/breach into `HudState`.

### `cf-render-2d`

- New `BreachRender`, `ExtractionRender` types in `ActorRenderState`.
- New `BreachRenderTag`, `ExtractionZoneTag` components.
- New `sync_breach_sprites` system spawns/updates/despawns colored rectangles per breach strip; concrete tone darkens as the strip is dug down, metal-nohook stays grey, broken strips fade.
- New `sync_extraction_zone` system spawns/updates a translucent green box (saturated when completed).

### `cf-ui`

- New `HudMission`, `HudEnemy`, `HudBreach` bundles in `HudState`.
- Six new lines in the status strip: OBJECTIVE, MISSION (timer + result), ENEMY (hp + state + last tactic), BREACH (id + hp progress + range + refusal), EVENT (last mission event label).
- New `mission_line`, `objective_line`, `enemy_line`, `breach_line` formatters with unit tests.

### `cf-e2e` (was M0 stub)

- Real scripted E2E runner. Resolves a script + scenario, auto-launches `cf-app --headless-smoke --control-api`, replays the script, then asserts on `--expect <key>=<value>` pairs (supports `mission.result`, `mission.loss_reason`, `objective.<id>`, `breach.<id>.broken`, `enemy.<actor>.state`).

## ID-by-ID acceptance matrix

| Backlog ID | Title | Status | Evidence |
|---|---|---|---|
| **M1.5-001** | scenario shell | PASS | `content/scenarios/micro_breach.ron`, `cf-control::scenario` validation tests, `cf-mod validate content/` PASS, mission events emitted in both win + loss bundles. |
| **M1.5-002** | reactive enemy | PASS | `cf-ai::ReactiveGuard` with hybrid LEAN (jobs + utility + scripted hooks), 9 unit tests pass, `ai.ai_perception` + `ai.tactic_chosen` + `ai.state_changed` + `equipment.weapon_fired` + `combat.projectile_spawned` events visible in win/loss bundles. |
| **M1.5-003** | temporary soft breach | PASS | `cf-terrain` ships `concrete_soft` + `metal_nohook` strips, 9 unit tests pass, `terrain.terrain_carved` (with M2-compatible bbox + material_before/material_after fields) + `terrain.terrain_breach_stub` + `terrain.tool_refused` events visible in win + loss bundles. |
| **M1.5-004** | readable loop HUD | PASS | `cf-ui::StatusStripPlugin` extended with 6 new lines (OBJECTIVE / MISSION / ENEMY / BREACH / EVENT, plus existing STATUS / ITEM / HP / reticle). 5 new formatter unit tests. Render layer paints breach strips + extraction zone. |
| **M1.5-005** | fun/evidence note | PASS | Implementation log entry: the scenario gives the player pressure (timer), goal (extract), enemy (reactive), and breach consequence. The HTML lab "ok I guess" signal is replaced by "win in 7 seconds, lose in 17 seconds; both feel intentional" agent-driven evidence. |
| **M1.5-006** | control-driven E2E | PASS | `cf-e2e --scenario micro_breach --script micro_breach_win --expect mission.result=won --expect objective.{breach,neutralize,extract}=completed` PASS (4/4). `cf-e2e --scenario micro_breach --script micro_breach_loss --expect mission.result=lost --expect mission.loss_reason=player_dead --expect objective.breach=completed` PASS (3/3). Both bundles validate via the canonical run-bundle checker. |

## Acceptance bundles (all PASS via `python3 game/tools/prototype_run_check.py`)

| run_id | mode | tick_rate_hz | ticks | events | result |
|---|---|---:|---:|---:|---|
| `m1.5_2026-05-08T01-27-46Z_d0068465` | cf-e2e win script | 60 | ~430 | 1549 | mission.result=won (4/4 expects PASS) |
| `m1.5_2026-05-08T01-27-55Z_c836bcbd` | cf-e2e loss script | 60 | ~1015 | 4098 | mission.result=lost reason=player_dead (3/3 expects PASS) |
| `m1.5_2026-05-08T01-28-25Z_4e23570a` | cfctl run inline | 60 | 600 | 1835 | M1.5-SMOKE-01 PASS |
| `m1.5_2026-05-08T01-28-27Z_f99e5cc2` | cfctl run inline | 120 | 600 | 1835 | tick-rate independence proof |

## Contract Integrity Matrix

| Contract path | Shared source of truth | Positive proof | Negative/adversarial proof | Checklist truth |
|---|---|---|---|---|
| `act.player.dig` from cfctl + cf-app keyboard | `cf_control::engine::ControlCommand::ActPlayerDig` → `M0Engine::dispatch` | `cfctl act player-dig` accepted; cf-app `KeyG` routes through the same dispatch (manual playtest); win script breaches via cfctl. | `act.player.dig` rejects with `act_player_unavailable_no_actor_world` on M0 scenarios; rejects with `no_breach_world` on M1 scenarios (no `breaches[]`). | M1.5-001 + M1.5-003 rows include the dig command and the rejection paths. |
| Reactive enemy fire / hit | `cf_ai::step` runs under engine RNG; `cf-control::engine::drive_tick` wires the same RNG to `actor_step` and `cf_ai::step` so the projectile pool is shared. | `combat.projectile_spawned` from guard appears in win + loss bundles; player HP drops correctly during loss script. | Guard never fires from M0 scenarios (no actor world); reset rewinds guard memory + ammo + cooldowns. | M1.5-002 row references the unit tests + bundle event counts. |
| Mission state machine | `cf_mission::step`; engine emits the report into `mission.*` events. | `objective_started` → `objective_completed` → `mission_resolved` chain visible in both bundles; win bundle `mission.result=won`, loss bundle `mission.result=lost reason=player_dead`. | Mission idempotent once terminal (`step` returns empty report); reset rewinds objectives + result + timer. Tests cover both. | M1.5-001 + M1.5-005 rows reference the unit tests + bundle event counts. |
| Run-bundle source-of-truth | `cf-replay` writes the same bundle whether triggered by cfctl `run`, cf-e2e `--write-run-bundle`, or cf-app `--write-run-bundle`. Each path goes through `M0Engine::write_run_bundle`. | All four acceptance bundles share the canonical naming convention, schema strings, and PASS the canonical checker. | The check script flagged a tick-monotonicity race during stabilization; fix was to record `observation_sent` before dropping the lock; bundles regenerated and now PASS. | Implementation log calls out the fix explicitly. |
| Scenario manifest validation | `cf_control::scenario::Scenario::load_from_file` is the single load path; cf-mod `validate` calls it. | `cf-mod validate content/` PASS for all 3 scenarios. Unit tests cover the M1.5 sample including ObjectiveUnknownActor rejection. | Validate rejects unknown rifle preset, multiple controllable actors, duplicate breach id, duplicate objective id, objective referencing unknown breach/actor. | All scenarios listed; M1.5 manifest fields documented in scenario.rs comments. |

## Validation log

```bash
# All from /Users/erol/projects/corefall/game

cargo fmt --all -- --check                                                   # PASS
cargo check --workspace --all-targets                                        # PASS
cargo clippy --workspace --all-targets -- -D warnings                        # PASS
cargo test --workspace                                                       # PASS (>200 tests across all crates; 9+ new in cf-mission, 9+ new in cf-terrain, 9+ new in cf-ai, 5 new in cf-ui)
cargo run -p cf-control --example dump_schemas -- --check                    # PASS — 26 schemas (added act_player_dig_params)
cargo run -p cf-mod -- validate content/                                     # PASS for m0_blank.ron, m1_actor_range.ron, micro_breach.ron
cargo run -p cfctl -- observe --once --inline                                # PASS
cargo build --release                                                        # PASS

# Acceptance scripts (release builds):
CF_APP_BIN=.../cf-app cf-e2e --scenario micro_breach --script micro_breach_win  --expect mission.result=won  --expect "objective.{breach,neutralize,extract}=completed" --write-run-bundle  # PASS 4/4
CF_APP_BIN=.../cf-app cf-e2e --scenario micro_breach --script micro_breach_loss --expect mission.result=lost --expect "mission.loss_reason=player_dead" --expect "objective.breach=completed" --write-run-bundle  # PASS 3/3
CF_APP_BIN=.../cf-app cfctl run --scenario micro_breach --ticks 600 --tick-rate-hz 60 --write-run-bundle    # PASS
CF_APP_BIN=.../cf-app cfctl run --scenario micro_breach --ticks 600 --tick-rate-hz 120 --write-run-bundle   # PASS

python3 tools/prototype_run_check.py prototype_runs/native/m1.5_*  # all 4 bundles errors=0
```

## corefall-review skill loop

Per the user's instruction to loop the project-local `corefall-review` skill before PR/bugbot, I ran the skill's workflow against the M1.5 working tree (4805 insertions across 26 files) and produced this iteration's findings:

### Iteration 1 findings (all fixed in this same pass)

| Severity | Title | Fix |
|---|---|---|
| Medium | HUD fabricated `enemy.state` and `enemy.last_tactic` | `cf-app::sync_actor_state_to_render` was hardcoding `state="active"` and `last_tactic="—"` regardless of the actual reactive guard's runtime state. Added `EnemyHudView` to `ActorRenderSnapshot` so cf-app reads real state + tactic from the engine; fallback labels only fire when no AI controller is attached. |
| Medium | tick-monotonicity race on M1.5 events | `EngineHandle::snapshot` previously read tick under the read lock, dropped the lock, then recorded `observation_sent`. With M1.5's higher per-tick event count (~3 events from input/AI/mission), drive_tick could acquire the write lock between the snapshot's read and record, producing non-monotonic `events.jsonl` ordering. Fix: record under the read lock. (Caught during initial bundle validation; bundles regenerated post-fix.) |
| Low (out of scope, deferred) | dispatch handlers have the same drop-then-record race pattern | Pre-existing in M0/M1, not introduced by M1.5. The race window is tiny in practice; all 4 acceptance bundles pass canonical validation. M3 closes DR-002 and is the milestone for proper recorder-side ordering. Logged as out-of-scope follow-up. |

### Iteration 2

After fixing the iteration-1 findings, re-ran `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, regenerated all four canonical bundles, and re-validated. No new findings in M1.5 scope. **Verdict: Accept.**

## Known follow-ups (out of M1.5 scope)

- The `cfctl observe --camera` surface is reserved for M4 (DR-009 closure). M1.5 does NOT introduce a camera observation envelope.
- `cf-mod validate` does not yet emit a `manifest_settings` audit row for new mission/breach fields — fine for M1.5 (schema names match), but M2 should add a typed validation pass for breach materials.
- Bevy renderer does not yet draw projectile sprites for the guard. Functional path works (events + HUD + state machine all correct); the visual is deferred to M4 (juice).
- `cf-e2e` does not yet support `--save-load-roundtrip` or `--verify-checksums` flags from the future M3 surface; left wired into the CLI as no-ops for M3 to fill in.

## Source trail

- spec/prototype-roadmap §M1.5 — Micro Breach Fun Slice
- spec/native-implementation-backlog M1.5-001..M1.5-006
- spec/feature-completion-checklist (M1.5 rows update queued)
- DR-002 (replay/event architecture, OPEN — closes at M3)
- DR-004 (sequenced single-actor → squad → bunker breach lean — confirmed unchanged; M7 closes the DR)
- DR-007 (terrain/material model, OPEN — defers implementation specifics to DR-036)
- DR-008 (AI architecture, OPEN — hybrid jobs + utility scoring + scripted hooks lean confirmed)
- DR-009 (command UX style, OPEN — direct + slowdown overlay + optional tactical map lean unchanged)
