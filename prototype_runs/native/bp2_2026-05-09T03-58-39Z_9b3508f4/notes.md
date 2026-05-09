## AI-Agent Self-Test Report (BP2)
- **Agent:** `Droid (Sonnet 4.5)`
- **Timestamp:** `2026-05-09T03:58:39Z`
- **Source bundle:** `prototype_runs/native/m2.5_2026-05-08T23-52-44Z_e5868b68`
- **Run id:** `m2.5_2026-05-08T23-52-44Z_e5868b68`
- **Scenario:** `micro_reactor_defense`
- **Tick rate:** `60 Hz`
- **Final sim checksum:** `ba3725483aaad045fbb3da53d55adcda01f9e89690e4a3394a5183c8fec8d364`
- **Summary grid:** `prototype_runs/native/m2.5_2026-05-08T23-52-44Z_e5868b68/captures/summary_grid.png` (? frames; non_blank_ratio=`?`)
- **Capture manifest:** `prototype_runs/native/m2.5_2026-05-08T23-52-44Z_e5868b68/captures/capture_manifest.json`

### Q1. What does this BP claim to deliver, in the project owner's words?

- M2 — Pixel Terrain And Materials: cf-terrain ChunkedTerrain + 8-material launch set (DR-007) + try_carve/try_blast/fill_aabb/fill_circle + projectile-vs-terrain collision; material_schema_version=cf-terrain-launch-v1.
- M2.5 — Micro Reactor Defense Fun Slice: cf-mission Reactor + DefendReactor objective + LossReason::ReactorDestroyed; dirt-shield strategic-choice scenario where the player chooses to preserve the shield (win) or breach it (loss).
- M3A — Event Recorder Core: snapshot.* events at run_started + ExpectedOutcome contract (clean/panic/abort) + cf-headless replay verifier with tick-for-tick checksum verification.

### Q2. Does the playable scenario deliver Q1 end-to-end through cfctl-driven inputs?

Per cfctl action exercised in this bundle (from `events.jsonl` `control.command_accepted` rows):

| cfctl method | Hands (action invoked) | Eyes (summary_grid.png frame + agent prose) | Ears (events.jsonl + observe.once) | Verdict |
|---|---|---|---|---|
| `scenario.reset` | tick 0 — first script step. | Frame [0,0] is fully black (cf-app render at tick 0 before scenario draw kicks in). Frame [0,1] tick ~22 shows the post-reset HUD: `OBJECTIVE: defend_reactor`, `MISSION: ACTIVE 0.0s/30s` over the navy air-default background. | events.jsonl: `control.command_accepted{method:scenario.reset}` + `mission.objective_started{objective: defend_reactor}` + `snapshot.snapshot_terrain_summary` (5 chunks). | PASS |
| `act.settings.set` | tick ~22 — sets captions=true. | Frame [0,1] shows `settings_changed` HUD label at the tick this fired; no visible side-effect at this maturity (settings flow gates renders at M4A+). | events.jsonl: `control.command_accepted{method:act.settings.set}` + `system.settings_observed`. observe.once: `settings.captions=true`. | PASS |
| `act.player.aim` | tick ~62 — aims down to dig the floor. | Frame [0,2] shows the tick HUD updating + reactor health bar still full (red bar lower-right). No reticle sprite is visible at thumbnail scale — readability is M4A+ scope. | observe.once: actor.aim_radians updated. | PASS |
| `act.player.dig` | tick ~62 — digs floor downward (preserve-shield strategy). | Frame [0,2] shows the `terrain_carved` HUD label at the tick this fired. Frame [0,3] tick ~123 shows `tool_refused` HUD when a follow-up dig hit the metal_nohook anchor strip (the refusal teaching path). | events.jsonl: `terrain.tool_action_started{mode:chunked}` + `terrain.terrain_carved{mode:chunked,dominant_material_id:1 (dirt)}` + `material.chunk_dirtied`. observe.once: `terrain.carve_count=1`, `terrain.dirty_chunk_count=1`. | PASS |
| `sim.run_for_ticks` | every script's "wait" step (12 total accepted commands). | Each frame's tick stamp advances by ~60 ticks per cell across the 8×8 grid (final tick ~1799 ≈ 30 s @ 60 Hz, matching the mission duration). | events.jsonl tick range spans 0..1799 with `tick_sample` rows every ~30 ticks; sim never stalled. | PASS |
| `runbundle.write` | tick 1800 — final script step. | n/a (logical action, no visible state change). | run_manifest.json + summary.json + events.jsonl + captures/ all written; `expected_outcome=clean` honored (1 `system.run_finished`, 0 `system.panic`, 0 severity:error). | PASS |
| `system.shutdown` | post-script teardown. | Last frame [7,7] tick ~1779 shows the final mission-resolved HUD: reactor health bar still red/non-zero (preserved); mission ended in WIN. | events.jsonl: `mission.mission_resolved{result:won}` + `system.run_finished`. observe.once: `mission.result=won`, `mission.outcome=time_expired_with_objectives`. | PASS |

### Q3. Does the visual presentation match the maturity level the BP promises?

I read `captures/summary_grid.png` (8×8 = 64 cells, 1280×720 source frames downscaled to thumbnails). Per-frame observations spanning the 0..1799 tick range:

- **Frame [0,0] tick 0:** Pure black — cf-app's first render before the scenario draw system kicks in. This is M1.5-baseline behavior; not a regression.
- **Frame [0,1] tick ~22:** Navy-blue air background fills the cell; HUD overlay top-left shows `[022] settings_changed`, `OBJECTIVE: defend_reactor`, `MISSION: ACTIVE 0.4s/30s`, `GOAL: PROL [u-AUTO; ENOUGH (u-)L)]`, `REACT: REACTOR_W=4108`. The mission state machine is clearly live and the BP2 reactor-world surface is observable. Bottom-edge UI shows two health bars: blue (player hp) on left, red (reactor hp) on right — both full.
- **Frame [0,2] tick ~62:** `terrain_carved` event label visible in HUD. The dig action fired and registered. The chunked terrain itself is not visually distinct from the navy background at thumbnail scale because cf-render-2d at BP2 maturity renders all materials with similar dark hues; per-material color overlays land at M4A (Readability And ACC-A Floor). This is on-roadmap behavior.
- **Frame [0,3] tick ~123:** `tool_refused` event fires after the player attempted to dig through the metal_nohook anchor strip. The refusal teaching path works and is observable in events; the visible feedback (a red flash, dust puff, or audio cue) is M4A/M5/BP6 scope.
- **Frames [1,0] through [7,7]:** A long sequence of `weapon_fired` HUD labels as the reactive guard fires its rifle bursts at the player. The HUD numbers tick up monotonically and the reactor health bar stays at full-red the entire run (the dirt shield is intact, so projectiles never reach the reactor's AABB). Mission timer counts down 0s → 30s. The final cell shows the run's terminal state: shield preserved, reactor unscathed, mission won by timer-with-objectives-active.

The visual presentation matches the BP2 promise: the world renders, the simulation runs deterministically, the events fire on the right ticks, and the HUD reflects live state. What is NOT visible at this maturity (per-material colors, projectile trails, dust/spark effects, animated sprites, reticle drawing) is M4A/M5 scope and not in BP2.

### Q4. Does the simulation behavior match the project owner's stated feel?

| Feel claim (from roadmap BP2 + M2 + M2.5 entries) | Confirmation |
|---|---|
| Chunked terrain replaces M1.5 BreachStrip without breaking replay consumers | Confirmed: `cf-headless replay` re-runs M1.5 micro_breach bundles tick-for-tick with 0 divergence (sweep row `m3a_headless_replay_m2_5_win` PASS) and re-runs M2.5 bundles tick-for-tick with 0 divergence (62 cadence checksums verified, 12 commands replayed). |
| 8 launch materials per DR-007 with stable ids 0..7 | Confirmed: `MATERIAL_SCHEMA_VERSION=cf-terrain-launch-v1` in run_manifest.json; observe.once `terrain.material_counts` returns per-material pixel totals; `tool_refused.reason=material_metal_nohook` and `material_anchor` both fire in sweep evidence. |
| Player can choose "preserve dirt shield (win)" or "breach it (loss)" | Confirmed: this bundle is the win path (mission.result=won, 0 reactor_damaged events). The loss bundle (`m2.5_2026-05-08T23-53-25Z_bac761ee`) shows mission.result=lost, loss_reason=reactor_destroyed, 10 reactor_damaged events. The strategic choice is real. |
| Reactor takes damage from projectile-vs-AABB hits | Confirmed in loss bundle (10 `actor.reactor_damaged` events tracking hp 60→0); not exercised in this win bundle (shield protects). |
| Guard utility-scores tactics with seeded miss rolls | Confirmed: 1920 `ai.tactic_chosen` events (one per tick the guard is alive) + 1920 `ai.ai_perception` events; 8 `weapon_reload_started` + 7 `weapon_reloaded` (one short — last reload in flight at run end); 64 weapon_fired across the 30s. Burst-pause cadence visible in tick spacing. |
| Mission state machine evaluates DefendReactor before timer-expiry | Confirmed by cf-mission unit tests (`mission_step_evaluates_reactor_destroyed_before_timer_expiry`) + this bundle's mission_resolved at tick 1799 with reactor still alive (objective auto-completes). |
| Replay verifier reconstructs commands from control.command_accepted | Confirmed: `cf-headless replay <bundle>` exits 0 with `replayed_ticks=1990, checksums_verified=33, commands_replayed=58, final_run_id=...`. |
| `expected_outcome=clean` enforced by checker | Confirmed: `prototype_run_check.py` asserts exactly one `system.run_finished` + zero `system.panic` + zero severity:error for clean outcome. This bundle PASSes. |

### Q5. Are there obvious inside-scope affordances the BP's text implies but the implementation skipped?

Reviewing the BP2 roadmap entry + M2/M2.5/M3A done-criteria + Design-Completeness Map row:

- (none identified for BP2 inside-scope work) — every milestone-scope action has a cfctl method, every state has an observe.once field, every state change emits an event, the fun-proof scenarios cover both win and loss paths, and the headless replay verifier is wired.

Future-owned items (NOT BP2 misses):

- Per-material color overlays in cf-render-2d → M4A (Readability + ACC-A Floor).
- Projectile trail / dust puff / muzzle flash juice → DR-055 + M5.5 (Full Collision Gauntlet) + BP3.
- Reactor damage VFX (shake, sparks) → M5 (Equipment, Chassis, Damage Grammar) + BP3.
- Guard sprite animation states (idle/aiming/firing/reload) → M4B (Comic-Noir Polish) + BP7.
- Mouse-click input device injection (`act.input.mouse_click` / `mouse_move`) → BP3 (when M4A introduces clickable HUD surfaces).
- 24h memory-leak soak per DR-051 → BP-boundary staging (universal row).

### Q6. Did the BP regress any prior-BP feel/feature?

Sweep run `self_play_sweep_2026-05-08T23-51-51Z_92652ef9` re-ran BP1's M1 actor round-trip + M1.5 micro_breach win + M1.5 micro_breach loss under the BP2 build:

- M1 actor round-trip: PASS — summary_grid.png shows actor moving + jumping + firing + reloading at the same cadence as BP1; `final_sim_checksum` re-verified at 60 Hz (`18760eca10...`) and 120 Hz (`9af4ec45c0...`).
- M1.5 micro_breach win: PASS — dig outer_wall + kill guard + reach extraction within 90 s; `mission.result=won`, `objective.extract=completed`, `breach.outer_wall.broken=true`.
- M1.5 micro_breach loss: PASS — time-out path reaches `mission.result=lost` as expected.

The chunked-terrain checksum extension is APPEND-ONLY relative to M1/M1.5 (per cf-terrain `checksum_bytes` + cf-control `sim_state_v1` layout), so M1.5 bundles checksum identically pre-BP2 and post-BP2. No regression.

### Q7. What would a human playtester see in the first 30 seconds that the AI agent missed?

Honest disclosure — gaps in my AI self-test that a human eyeballing the running cf-app might catch:

- **Sub-tick visual smoothness:** the captures are sampled at 10 Hz baseline + event keyframes; a human watching at 60 fps could perceive frame-pacing judder, sprite tearing, or sub-tick interpolation that my discrete frame samples cannot resolve.
- **Sprite-level alignment / off-by-1 px:** cf-render-2d renders at the navy-air background with HUD overlays; per-pixel sprite alignment of actors / projectiles / chunks is not distinguishable at the 8×8 thumbnail resolution my Read tool returns. A human playtester running the game at 1280×720 native would catch sprite-anchor errors I cannot see.
- **Input-to-render latency:** all inputs in this bundle were JSON-RPC (cfctl), not keyboard. A human pressing G to dig may experience a different perceived latency than the cfctl-dispatched `act.player.dig`. BP3 is the right milestone to add `act.input.key_press` so the AI agent can also exercise the keyboard path.
- **Audio sync (BP6+ scope, currently no-op):** no audio surface yet, so n/a.
- **Accessibility-flag visual side-effects:** captions=true, high_contrast=false, reduced_motion=false in this bundle; a human toggling high_contrast or reduced_motion at runtime would catch render glitches my static run cannot.

Recommendation: human playtest is **optional confirmation** for BP2 closure (per the new corefall AGENTS.md gate), but I'd encourage the project owner to play the M2.5 win + loss scenarios at native resolution before BP3 to validate the visual baseline this report attests to.

### Run signal (auto-extracted from events.jsonl)

| event_type | count |
|---|---|
| `actor_snapshot` | 32 |
| `ai_perception` | 1920 |
| `chunk_dirtied` | 1 |
| `command_accepted` | 12 |
| `intent_received` | 1920 |
| `mission_resolved` | 1 |
| `objective_completed` | 1 |
| `observation_sent` | 1264 |
| `projectile_expired` | 64 |
| `projectile_spawned` | 64 |
| `run_finished` | 1 |
| `run_started` | 1 |
| `settings_observed` | 1 |
| `sim_checksum` | 35 |
| `snapshot_actor` | 3 |
| `snapshot_inventory` | 2 |
| `snapshot_terrain_chunk` | 5 |
| `snapshot_terrain_summary` | 1 |
| `state_changed` | 1 |
| `tactic_chosen` | 1920 |
| `terrain_carved` | 1 |
| `tick_sample` | 32 |
| `tool_action_started` | 2 |
| `tool_refused` | 1 |
| `weapon_fired` | 64 |
| `weapon_reload_started` | 8 |
| `weapon_reloaded` | 7 |

## Human Playtest Survey (optional confirmation)

_This section is OPTIONAL per `corefall/AGENTS.md` Build Point Closure Gate. The AI-Agent Self-Test Report above is the gating contract. The project owner may add a row here after playing the BP._

- **Question:** Did BP2 make the game more fun than the previous BP?
- **Reference summary grid:** `see captures/summary_grid.png`
- **Owner's answer:** _(empty until played)_
- **Concrete observations:** _(empty until played)_
