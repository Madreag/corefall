---
type: spec
status: implementation-backlog
authority: "Native Rust + Bevy/wgpu implementation task cards for prototype-roadmap M0..M12."
ready_when: "Each milestone has enough task cards, validation commands, artifact requirements, and anti-scope notes that an AI implementation agent can own the milestone without reinterpreting the roadmap."
feeds:
  - DR-001
  - DR-002
  - DR-003
  - DR-004
  - DR-005
  - DR-006
  - DR-007
  - DR-008
  - DR-009
  - DR-010
  - DR-011
  - DR-012
  - DR-013
  - DR-014
  - DR-015
  - DR-016
  - DR-017
  - DR-018
  - DR-019
  - DR-020
  - DR-021
  - DR-022
  - DR-023
  - DR-024
  - DR-025
  - DR-026
  - DR-027
  - DR-028
  - DR-029
  - DR-030
  - DR-031
  - DR-032
  - DR-033
  - DR-034
  - DR-035
  - DR-036
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/feature-completion-checklist|feature checklist]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[spec/server-app-architecture|server app architecture]] · [[spec/persistent-mmo-architecture|persistent MMO architecture]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/hybrid-llm-ai-plan|hybrid LLM AI plan]] · [[spec/ai-control-observability-layer|AI control/observability]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [VAULT_PLAN.md](../../VAULT_PLAN.md)

# Native Implementation Backlog

> [!summary] Purpose
> Concrete task-card handoff for the native roadmap. Assign agents work from this page, not from the historical browser/Slice-A backlog. Each milestone card names write scope, implementation work, tests, E2E proof, run-bundle evidence, bug-hunt obligations, and anti-scope.

> [!important] Assignment rule
> Assign one milestone or one contiguous subset of task cards at a time. The agent must not stop at "implemented"; it must run validation, fix failures, bug hunt, emit a run bundle, update vault evidence, and write a final audit.

## Global Rules For Every Task

| Rule | Requirement |
|---|---|
| Read first | `AGENTS.md`, [[spec/prototype-roadmap]] (especially the Glossary, Toolchain And Workspace Bootstrap, CLI Reference, Control Transport And Envelope, Scenario Manifest Schema, Run-Bundle Naming Convention, Per-Milestone Kickoff Smoke), this backlog, [[spec/feature-completion-checklist]], and linked DR/spec pages. |
| Work location | Native production prototype lives in `game/` unless the implementation agent proposes and documents a different path. |
| Reference repos | Never edit CCCP/C4/comparable repos. They are read-only reference material. |
| Crate ownership | Touch only owned crates/files plus explicit boundary crates. Each crate ships an `AGENTS.md` per the [[spec/prototype-roadmap#Per-Crate AGENTS.md Template|Per-Crate AGENTS.md Template]]. |
| Events first | Any behavior that affects player understanding, replay, AI, networking, save, or debugging emits an event. |
| Control first | Every new player-facing control or UI action gets a semantic `cf-control` / `cfctl` path unless explicitly marked human-only. The transport pin is in [[spec/prototype-roadmap#Control Transport And Envelope|Control Transport]]. |
| Tests required | Every new behavior gets unit or integration coverage per [[spec/prototype-roadmap#Testing Layers|Testing Layers]]; every player-facing milestone gets E2E proof. |
| Evidence required | Every meaningful run emits a checked run bundle named per [[spec/prototype-roadmap#Run-Bundle Naming Convention|Run-Bundle Naming Convention]] under `prototype_runs/native/`. |
| Checklist required | Every completed or partially completed task updates [[spec/feature-completion-checklist]] with affected row ids, evidence links, and AI self-ratings. Human rating columns stay blank unless the user provides ratings. |
| Human gates | If a criterion requires project-owner play, platform hardware, or human accessibility feedback, prepare the build, run the kickoff smoke, fill in the [[spec/prototype-roadmap#Human Playtest Checklist Template|Human Playtest Checklist Template]], and mark `READY_FOR_HUMAN`. |
| Logging/errors | Use the policy in [[spec/prototype-roadmap#Logging, Tracing, And Error Policy|Logging, Tracing, And Error Policy]]. No `println!`, no `unwrap()` on user-controllable inputs, no `rand::thread_rng` inside sim. |
| Bug log | Found bugs go in the milestone vault note under `## Bugs Found And Fixed` per the [[spec/prototype-roadmap#Bug Log Format|Bug Log Format]]. |

## Standard Validation

Run these unless a task card narrows the set:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cfctl -- observe --once
python3 research_tools/prototype_run_check.py prototype_runs/native/<run_id>
```

Milestones with gameplay/tool UI also require a scripted E2E command and a screenshot/capture artifact listed in `summary.json.artifacts`.

## Task Card Format

| Field | Meaning |
|---|---|
| ID | Stable task id, e.g. `M2-003`. |
| Owns | Crates/files this task may edit. |
| Build | Concrete implementation scope. |
| Tests | Unit/integration/E2E tests expected. |
| Evidence | Run bundle, screenshot, perf report, vault note. |
| Anti-scope | What the task must not grow into. |

---

## M0 — Engine Bootstrap

> [!important] Kickoff prerequisites
> Before feature work, run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M0 Kickoff Smoke]]. Apply the [[spec/prototype-roadmap#Toolchain And Workspace Bootstrap|Toolchain And Workspace Bootstrap]] recipe verbatim: `rust-toolchain.toml`, workspace `Cargo.toml`, `rustfmt.toml`, `clippy.toml`, `.cargo/config.toml`, `.gitignore`, `.github/workflows/ci.yml`. M0 is not done until those files exist, the smoke commands pass, and the bundle validates.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M0-001 workspace scaffold | `game/Cargo.toml`, `game/crates/*`, `game/.cargo/config.toml`, `game/rust-toolchain.toml`, `game/rustfmt.toml`, `game/clippy.toml`, `game/.gitignore`, root docs | Apply the bootstrap recipe verbatim: workspace + 29 crates (`cf-app`, `cf-sim-core`, `cf-terrain`, `cf-physics`, `cf-material`, `cf-atmos`, `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-ai`, `cf-mission`, `cf-replay`, `cf-control`, `cfctl`, `cf-e2e`, `cf-save`, `cf-net`, `cf-render-2d`, `cf-ui`, `cf-audio`, `cf-mod`, `cf-tools-editor`, `cf-headless`, `cf-server`, `cf-server-ops`, `cf-server-persistence`, `cf-server-anti-cheat`, `cf-server-admin`, `cf-bench`); per-crate `AGENTS.md` per the template; pinned Bevy/glam/serde/clap/tokio/jsonrpsee/blake3/tracing/rand_xoshiro/schemars deps from the workspace dependencies table. | `cargo metadata`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. | Commit-ready diff + vault note `prototypes/native-m0-bootstrap.md`; commit only when the user explicitly asks. | No gameplay systems beyond no-op scene; no extra crates not in the recipe. |
| M0-002 Bevy app shell | `cf-app`, `cf-render-2d` | Open a window, clear screen, fixed title/version, ESC exits; support all M0 flags from the [[spec/prototype-roadmap#CLI Reference|CLI Reference]] (`--scenario`, `--seed`, `--run-seconds`, `--ticks`, `--write-run-bundle`, `--run-bundle-dir`, `--control-api`, `--control-port`, `--control-uds`, `--headless-smoke`, `--debug-capabilities`, `--ui-scale`, `--high-contrast`). | Native app smoke for 5 seconds; CLI flag parser tests; `--headless-smoke` exits cleanly. | Screenshot/capture of blank window if available; CLI help output captured to `notes.md`. | No menu system or final UI. |
| M0-003 fixed tick island | `cf-sim-core` | Implement fixed 60 Hz tick with optional 120 Hz; deterministic seed/RNG wrapper using `rand_xoshiro::Xoshiro256StarStar`; `Tick(u64)`, `WallClock`, `SimClock` types; tick counters; `pause`, `resume`, `step(n)`, `run_for(n)` API consumed by `cf-control`. | Unit tests for tick accumulation, seed repeatability (same seed → same checksum after 1000 ticks), pause/resume/step semantics; lints disallow `rand::thread_rng` and `SystemTime::now` per `clippy.toml`. | `system.run_started`, `system.tick_sample`, `system.run_finished` events emitted via `cf-replay`. | No full scheduler rewrite; no Bevy scheduling-stage redesign. |
| M0-004 run bundle writer | `cf-replay`, `tools/` | Write `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`; include build hash, config hash, scene id, schema version, capabilities, expected tests; directory naming per [[spec/prototype-roadmap#Run-Bundle Naming Convention|Run-Bundle Naming Convention]]. | Checker passes on M0 bundle; round-trip test for the envelope; non-blocking write under stress (events queued; dropped counter visible in `summary.json.event_counts.dropped_total`). | `prototype_runs/native/m0_<UTC-iso>_<short-hash>/`. | Do not design final replay UI. |
| M0-005 CI matrix | `.github/workflows/ci.yml`, `game/` | Apply the CI YAML from the bootstrap recipe; Win/Linux/macOS matrix; `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`, `cfctl observe smoke`, `cfctl run --write-run-bundle` smoke. | CI green when runners available; local commands pass regardless. | Link CI/log or local validation output in vault note. | No release packaging; no Steam Deck CI yet. |
| M0-006 control/observe bootstrap | `cf-control`, `cfctl`, `cf-app`, `cf-replay` | Define command/observation envelope per [[spec/prototype-roadmap#Control Transport And Envelope|Control Transport And Envelope]] (JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890`, optional UDS, `schema_version` mandatory, blake3 short-hash run ids); generate JSON Schemas via `schemars` under `crates/cf-control/schemas/v1/`; implement `scenario.load`, `sim.pause/resume/step/run_for_ticks`, `observe.once`, `observe.subscribe/unsubscribe`, `observe.frame` notification, `act.player.*`, `runbundle.write`, `system.shutdown`; `cfctl` subcommands `observe --once|--stream`, `run --ticks --write-run-bundle`, `scenario load`, `pause`, `step`, `script run` per [[spec/prototype-roadmap#CLI Reference|CLI Reference]]. | Unit tests for envelope (request/response/notification roundtrip); schema_version mismatch returns `-32602` with fix-hint; `cargo run -p cfctl -- observe --once`; no-op `run --ticks 300 --write-run-bundle` writes a valid bundle; loopback-only by default; heartbeat ping/pong. | Observation JSON sample and control-run bundle linked in M0 note. | No remote bot API; no gameplay debug cheats; no unauthenticated remote bind. |
| M0-007 m0_blank scenario fixture | `content/scenarios/m0_blank.ron` | Author the M0 scenario manifest per [[spec/prototype-roadmap#Scenario Manifest Schema|Scenario Manifest Schema]] minimal skeleton; loadable by `cf-app --scenario m0_blank`; validates with `cargo run -p cf-mod -- validate content/`. | Schema validation test; `--scenario m0_blank` smoke. | The RON file in `content/scenarios/`. | No teams, actors, terrain, or objectives. |
| M0-008 panic hook + tracing init | `cf-app`, `cfctl`, `cf-headless`, `cf-bench`, `cf-e2e`, `cf-mod` | Each binary's `main()` initializes `tracing-subscriber` with `EnvFilter` per [[spec/prototype-roadmap#Logging, Tracing, And Error Policy|Logging, Tracing, And Error Policy]]; installs a panic hook that emits `system.panic` event with backtrace before exit; severity counters are incremented in `summary.json.event_counts.by_severity`. | Binary boot test asserts the subscriber is registered; panic test triggers a controlled panic in a sub-thread and verifies the event is emitted; counter assertion. | One example structured log entry from each binary in the run notes. | No log-aggregation product. |

M0 reference E2E:

```bash
cargo run -p cf-app -- --scenario m0_blank --run-seconds 5 --write-run-bundle
cargo run -p cfctl -- observe --once
cargo run -p cfctl -- run --ticks 300 --write-run-bundle
python3 research_tools/prototype_run_check.py prototype_runs/native/<m0_run>
```

---

## M1 — Actor Controller And Sim Core

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M1-001 control intent | `cf-actor`, `cf-sim-core`, `cf-replay` | Input maps to `ControlIntent` before sim consequences: move, jump, aim, fire, reload, selected item. | Input serialization test; intent precedes action event. | Run bundle with `input_intent`. | No rollback/netcode. |
| M1-002 actor movement | `cf-actor`, `cf-physics` | Position/velocity, gravity, ground collision, jump/fall/recovery, reset. | Unit tests for gravity/ground/contact; E2E movement route. | 5-minute run with actor snapshots. | No complex ragdoll. |
| M1-003 rifle loop | `cf-equipment`, `cf-actor` | One rifle: fire interval, ammo, reload, recoil, muzzle origin, hit/miss event. | Ammo/reload/recoil tests; scripted fire/reload E2E. | `weapon_fired`, `weapon_reloaded`, `projectile_*` events. | No large arsenal. |
| M1-004 status strip | `cf-ui`, `cf-actor` | Minimal HUD: status, ammo, selected item, reticle state. | UI state source tests; screenshot artifact. | HUD screenshot in run bundle. | No final comic-noir UI. |
| M1-005 HTML lab supersession note | vault only | Record whether native M1 supersedes HTML lab for actor iteration; list gaps if not. | N/A. | `prototypes/native-m1-actor-controller.md`. | Do not delete HTML evidence. |
| M1-006 semantic actor control | `cf-control`, `cf-actor`, `cf-equipment`, `cf-e2e` | Drive movement, aim, fire, reload, selected item, and reset through the same `ControlIntent` path as human input; stream actor/equipment observations. | Scripted movement/fire/reload through `cfctl`; assert events and observations agree. | Control-script run bundle with no screenshot dependency. | No network prediction/rollback. |

M1 E2E:

```bash
cargo run -p cf-e2e -- --scenario m1_actor_range --script move_jump_fire_reload --write-run-bundle
cargo run -p cfctl -- script run m1_move_jump_fire_reload --write-run-bundle
```

Human gate: project-owner five-minute feel reaction. If unavailable, mark `READY_FOR_HUMAN_PLAYTEST`.

---

## M1.5 — Micro Breach Fun Slice

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M1.5-001 scenario shell | `cf-mission`, `cf-app`, `content/scenarios/` | 60-90s `micro_breach` scenario: spawn, objective, timer, extraction, win/loss. | Objective state tests; scripted win/loss. | `objective_*` events in two bundles. | No full mission director. |
| M1.5-002 reactive enemy | `cf-ai`, `cf-actor`, `cf-equipment` | One enemy: sight cone, aim delay, imperfect fire, reload, death; no omniscience. | Perception/aim/fire tests; E2E enemy kill + player death. | `ai_perception`, `tactic_chosen`, `weapon_fired`, `actor_status_changed`. | No full AI doctrine system. |
| M1.5-003 temporary soft breach | `cf-terrain` or `cf-mission` fixture | Minimal soft barrier/diggable tiles with success/refusal events; compatibility with future M2 event names. | Dig success/refusal tests. | `terrain_carved` or `terrain_breach_stub` with bbox/material. | No full chunked terrain. |
| M1.5-004 readable loop HUD | `cf-ui` | Objective/timer, player status, enemy status, selected item, last event. | Screenshot at 100% and 200% scale if UI scaling exists. | Capture artifact. | No full HUD art pass. |
| M1.5-005 fun/evidence note | vault only | Compare reaction against "ok I guess"; list whether pressure/goal changed feel. | N/A. | `prototypes/native-m1-5-micro-breach.md`. | Do not claim final fun. |
| M1.5-006 control-driven E2E | `cf-control`, `cf-e2e`, `cf-mission` | Write `cfctl` scripts for win path and loss path; assertions read objective/enemy/player state from observations and events. | `cargo run -p cfctl -- script run micro_breach_win` and `micro_breach_loss`; observation stream freshness check. | Two checked run bundles, action transcript, observation sample. | No brittle OS-level mouse/keyboard automation. |

M1.5 E2E:

```bash
cargo run -p cfctl -- script run micro_breach_win --expect objective.result=win --write-run-bundle
cargo run -p cfctl -- script run micro_breach_loss --expect objective.result=loss --write-run-bundle
cargo run -p cf-e2e -- --scenario micro_breach --script win_path --expect win --write-run-bundle
cargo run -p cf-e2e -- --scenario micro_breach --script lose_path --expect loss --write-run-bundle
```

Human gate: project-owner plays at least three runs and records verbatim reaction.

---

## M2 — Pixel Terrain And Materials

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M2-001 chunk storage | `cf-terrain` | 256x256 chunk grid, material id per pixel, sparse storage, CPU read/write. | Chunk bounds, material set/get, serialization tests. | Terrain snapshot in run bundle. | No Noita chemistry. |
| M2-002 material registry | `cf-terrain`, `content/materials/` | Air, dirt, concrete, metal-nohook, hazard, loose fill, repair-fill, anchor; hardness/affordance fields. | Material schema validation. | Material schema version in manifest. | No full research tree. |
| M2-003 carving pipeline | `cf-terrain`, `cf-render-2d`, `cf-equipment` | Digger and blast carve; CPU fallback; optional wgpu path behind feature flag. | Carve bbox/count tests; GPU/CPU parity if GPU path exists. | Dirty-region and perf counters. | No production destruction VFX. |
| M2-004 physics integration | `cf-physics`, `cf-terrain` | Actor collision respects terrain after edits; chunk boundary tests. | Collision after carve/fill tests. | E2E dig-through-wall. | No full pathfinding. |
| M2-005 material overlay | `cf-ui`, `cf-render-2d` | Toggle overlay shows material ids and tool validity. | Screenshot at 100/200% if applicable. | Overlay capture. | No tactical map. |
| M2-006 terrain replay | `cf-replay`, `cf-terrain` | Terrain snapshots/checksums and event replay reconstruct terrain. | Live vs replay checksum test. | Replay report. | No final cinematic replay. |

M2 E2E:

```bash
cargo run -p cf-e2e -- --scenario m2_material_lane --script dig_concrete_refuse_metal --expect win --write-run-bundle
```

---

## M3 — Replay And Event Recorder

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M3-001 event taxonomy | `cf-replay` | Stable event envelope, categories, parent ids, schema versions. | Schema/event ordering tests. | Updated run-bundle schema note if fields change. | No analytics service. |
| M3-002 snapshots/checksums | `cf-replay`, `cf-terrain`, `cf-actor`, `cf-equipment` | Actor/inventory/terrain snapshots and checksums. | Checksum repeatability tests. | `determinism.sim_checksum` events. | No full deterministic promise for cosmetics. |
| M3-003 headless replay | `cf-headless`, `cf-replay` | Replay M2/M1.5 bundles without rendering and verify checksums. | Replay compare test. | First-divergence report on failure. | No network server yet. |
| M3-004 viewer | `cf-ui`, `cf-replay` | Event tail, filters, parent-chain view, death/failure recap. | Viewer smoke test; screenshot. | Viewer capture in bundle. | No polished replay browser. |
| M3-005 recorder backpressure | `cf-replay` | Dropped-event counters and non-blocking recorder path. | Stress event-volume test. | Summary volume/perf rows. | No cloud telemetry. |

M3 E2E:

```bash
cargo run -p cf-headless -- replay prototype_runs/native/<m2_run> --verify-checksums
```

---

## M4 — HUD And Comic-Noir UI

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M4-001 HUD state model | `cf-ui`, `cf-actor`, `cf-equipment`, `cf-chassis` | Actor status, body silhouette placeholder, ammo, item, objective, last event. | UI state tests. | HUD screenshots. | No final art polish. |
| M4-002 comic-noir cards | `cf-ui`, `content/ui/` | Pre-mission and debrief card templates. | Layout snapshot tests if available. | 100/200% captures. | No full campaign UI. |
| M4-003 accessibility floor | `cf-ui`, `cf-app` | 200% scale, high contrast, keyboard/controller focus, captions hook, reduced shake/flash flags. | E2E accessibility smoke. | ACC-A status in notes. | No certification claim. |
| M4-004 material/tool feedback | `cf-ui`, `cf-terrain` | Tool validity labels and non-color-only material feedback. | Overlay screenshot tests. | Capture artifact. | No full tactical map. |

M4 E2E:

```bash
cargo run -p cf-e2e -- --scenario micro_breach --ui-scale 2.0 --high-contrast --verify-focus --write-run-bundle
```

Human gate: multi-person readability/accessibility playtests.

---

## M5 — Equipment, Chassis, And Damage Grammar

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M5-001 role records | `cf-equipment`, `content/equipment/` | Runtime role-record model from vault fixtures: role tags, bot policy, source/provenance fields. | Schema/fixture tests. | LOAD-A fixture import report. | No full economy/store. |
| M5-002 chassis model | `cf-chassis` | Armor zones, modules, pilot binding, powered armor and light mech. | State transition tests. | `chassis_stage_changed` events. | No full mech roster. |
| M5-003 damage/eject/repair/salvage | `cf-chassis`, `cf-equipment`, `cf-replay` | Module damage, jam, eject, repair, salvage events and HUD labels. | E2E wreck/eject/salvage. | Chassis run bundle. | No final gore/body system. |
| M5-004 save hooks | `cf-save`, `cf-chassis` | Serialize chassis/equipment state enough for roundtrip. | Save/load checksum. | Save artifact linked from run. | No full campaign save UI. |

M5 E2E:

```bash
cargo run -p cf-e2e -- --scenario m5_chassis_wreck_eject --expect pilot_extracted --write-run-bundle
```

---

## M5.5 — Full Collision Gauntlet

> [!important] Kickoff prerequisites
> M2, M3, and M5 must be complete enough to provide terrain chunk proxies, replay/run-bundle events, and body/chassis/equipment proxy identities. Read [[spec/full-collision-physics-plan]] and [[decisions/dr-033-full-collision-physics-direction]] in full BEFORE feature work. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M5.5 Kickoff Smoke]]. M5.5 is not done until COLL-001..COLL-012 pass and the run replays headlessly with collision checksums.

> [!warning] Hard rules
> Everything physical collides by default unless a tested `collision_filter_reason` says otherwise. Do not brute-force all-pairs. Use broadphase, collision classes, proxies, CCD tiers, and deterministic pair ordering. Projectile-projectile collision is required for physical projectile classes; cosmetic tracers can be non-physical only if the actual projectile record still carries gameplay collision.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M5.5-001 collision class registry | `cf-physics`, `cf-mod`, `content/collision/` | Define physical classes from [[spec/full-collision-physics-plan]]: actor core, limb, armor zone, held weapon, loose item, kinetic projectile, explosive projectile, terrain proxy, debris chunk, mech part, base object, force field, sensor trigger, cosmetic particle. | Registry roundtrip; missing-class validation; class-id stability test. | `collision_class_registered` or schema audit in run note. | No gameplay behavior yet. |
| M5.5-002 collision matrix + filters | `cf-physics`, `cf-mod` | Data-driven matrix with collide/sensor/filter/damage response; every filter requires `collision_filter_reason`. | COLL-001; bad matrix fixtures fail with useful diagnostics. | Matrix file and validator report. | No silent ignore pairs. |
| M5.5-003 broadphase and pair cache | `cf-physics`, `cf-bench` | Dynamic tree/spatial hash hybrid; stable pair ids; deterministic pair ordering; projectile lane cache. | Pair-count tests; deterministic ordering; stress bench. | Perf counters: candidate pairs, narrowphase pairs, culled low-value pairs. | No O(n^2) production path. |
| M5.5-004 narrowphase/contact manifolds | `cf-physics` | Contact manifolds for circle/capsule/convex/AABB/segment/terrain-proxy pairs; material pair lookup. | Shape-pair unit tests; edge/tiny-hole fixtures. | `collision_contact_started/persisted/ended`. | No exact per-pixel rigid body solver. |
| M5.5-005 CCD tiers | `cf-physics`, `cf-bench` | Discrete, speculative, sweep ray, sweep capsule, sweep shape, TOI substep; per-class `ccd_class`. | COLL-007; tunneling fixtures for thin terrain, limb, shield, bullet, mech foot. | `toi_fraction` and CCD-tier fields in events. | No universal TOI for all debris. |
| M5.5-006 projectile-projectile contacts | `cf-physics`, `cf-equipment`, `content/projectiles/` | Swept projectile lane test; kinetic deflect/fragment/tumble/energy loss; explosive detonate/fuze-fail/deflect by profile. | COLL-006; deterministic bullet-cross fixtures. | `projectile_projectile_contact`, `projectile_deflected`, optional `projectile_fragmented`. | No fake random explosions for kinetic rounds. |
| M5.5-007 limb/equipment/body contacts | `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-physics` | Limb-to-limb/body/weapon/terrain/door contacts; held weapon physical contacts; scoped owner self-filter; dropped item contacts. | COLL-002..COLL-004; crowd corridor fixture. | Contact events plus body/equipment/chassis follow-up events. | No animation-only collision. |
| M5.5-008 impulse-to-damage routing | `cf-physics`, `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-terrain` | Convert contact impulse/material/area/sharpness into limb wounds, armor crack/spall, equipment jam/damage, chassis module failure, terrain/base damage. | COLL-005, COLL-008; threshold tests by material/origin. | `contact_impulse_applied`, `collision_damage_applied`, parent-linked body/equipment/terrain events. | No hidden HP-only damage. |
| M5.5-009 terrain/base/shield proxies | `cf-terrain`, `cf-mission`, `cf-physics` | Dirty chunk collision proxy rebuilds; doors/turrets/sensors/shields/repair pads register physical or sensor proxies. | Chunk seam tests; shield/body/projectile/base fixtures. | Terrain dirty-to-proxy events; base object contact events. | No full base builder here. |
| M5.5-010 `cfctl` collision observation | `cf-control`, `cfctl`, `cf-physics` | `cfctl observe --collisions` and `cfctl inspect collision <event-id>` show live pairs, filters, last contacts, TOI, impulses, and budget status. | CLI snapshot tests; stream freshness tests. | Observation samples in run notes. | No screenshot-only physics debugging. |
| M5.5-011 full gauntlet scenario | `content/scenarios/m5_5_full_collision_gauntlet.ron`, `cf-e2e` | Scenario scripts for COLL-001..COLL-012: crowd corridor, bullet cross, limb/weapon/door, debris crush, mech foot, shield, terrain seams. | Full E2E suite. | Checked run bundle with event counts by collision type. | No hand-tested-only acceptance. |
| M5.5-012 replay/perf/bug hunt | `cf-headless`, `cf-bench`, `tools/`, vault | Headless replay checksum; perf report; first-divergence event; bug-hunt log. | Replay verify; 1080p/60 pass; 4K/120 + Deck status recorded. | Prototype note under `prototypes/` with final audit and known issues. | No "works once" completion. |

M5.5 E2E:

```bash
cargo run -p cf-e2e -- --scenario m5_5_full_collision_gauntlet --suite COLL-001..COLL-012 --write-run-bundle
cargo run -p cfctl -- observe --collisions --stream --hz 30 --scenario m5_5_full_collision_gauntlet
cargo run -p cf-headless -- replay prototype_runs/native/<m5_5_run> --verify-checksums
cargo run -p cf-bench -- --scenario m5_5_full_collision_gauntlet --profile collision
```

Human gate: project-owner may play the gauntlet for feel, but all COLL-* tests are agent-completable.

---

## M5.6 — Material Kernel

> [!important] Kickoff prerequisites
> M2 (terrain + materials) and M3 (replay/event recorder) must be complete; M5.5 (full collision physics) must be complete enough to expose contact + impulse data the kernel can subscribe to. Read [[decisions/dr-036-systemic-material-simulation-direction]], [[decisions/dr-007-terrain-material-model]], and [[comparables/noita-grade-material-simulation-research]] in full BEFORE feature work. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M5.6 Kickoff Smoke]]. M5.6 is not done until MAT-01..MAT-03 + MAT-06 + MAT-13 pass and the run replays headlessly with material/reaction checksums.

> [!warning] Hard rules
> Active-region only (no everywhere-always sim). 64×64 chunks, dirty rects, sleeping chunks, per-chunk material checksum. CPU-deterministic kernel; chunk update order pinned; no platform-specific atomics in the inner loop. No GPU-only material updates that bypass replay. Curated launch material set (17) per DR-036; expansion materials require material lab + balance review (M8.5+).

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| MAT-01 chunked material grid | `cf-material`, `cf-terrain` | 64×64 active material chunks with `MaterialId`, `Temperature`, `State`, `Charge` fields per pixel. Sparse storage; dirty-rect tracking; sleeping chunks; per-chunk material checksum; integration with `cf-terrain` chunk grid. | Chunk roundtrip tests; sleep/wake transitions; checksum stability under no-op ticks. | `material_chunk_dirtied/slept/woken` events; per-chunk checksum field in snapshots. | No global per-pixel update pass; no GPU-only state. |
| MAT-02 launch material registry | `cf-material`, `cf-mod`, `content/materials/` | Curated 17-material registry per DR-036: air, dirt/sand, rock/concrete, metal, wood/organic, water, steam/mist, smoke, fire/heat, oil/fuel, acid, toxic sludge/liquid, toxic gas, lava, blood/vomit, electricity charge, pebble/debris. Each material defines density, viscosity, conductivity, ignition point, melting/boiling thresholds, AI affordance tags, hazard overlay id, sound id. | Registry roundtrip; missing-field validation; affordance-tag enum coverage. | Schema audit in run bundle; `cf-mod validate content/materials/ --strict` passes. | No expansion materials shipped here; no per-shader-only material data. |
| MAT-03 reaction table + engine | `cf-material`, `content/reactions/` | Data-driven pair/triple reaction table with priority, temperature thresholds, catalysts, byproducts, probability (deterministic via per-chunk RNG). Reaction engine consumes table; emits `reaction.*` events with cause chain. | Per-pair fixture tests; priority ordering tests; catalyst tests; byproduct mass-conservation tests where applicable. | `reaction.*` events visible in run bundle; reaction inspect via `cfctl observe --reactions`. | No hidden reactions without replay events; no implicit chemistry baked into rendering. |
| MAT-04 active-region scheduler | `cf-material`, `cf-physics` | Active-region selector based on actor proximity, recent edits, recent reactions, fire/electricity propagation; inactive chunks sleep with checksum verification on wake. Per-frame budget + LOD tier (full/coarse/sleeping). | Active-region selection tests; budget enforcement tests; perf bench. | `material_active_region_changed`, `material_budget_exceeded` events; perf counters. | No always-on global update path. |
| MAT-05 density layering + flow | `cf-material` | Stable layering rule for immiscible liquids (oil floats on water, sludge sinks). Gravity-driven flow with viscosity factor. Gas rises with diffusion. | Layering fixture tests (oil-on-water, sludge sink, steam rise); flow rate tests by viscosity. | Layering scenario in MAT-01..03 run bundle. | No real fluid-dynamics simulation; no Navier-Stokes. |
| MAT-06 phase change | `cf-material` | Temperature-driven phase transitions: water ↔ steam, lava ↔ rock, ice ↔ water (if shipped), oil ↔ fire. Each transition emits `material.*` event with parent cause. | Per-transition fixture tests; threshold tests. | Phase change events visible in M5.6 run bundle. | No alchemy here; no phase change for solids beyond launch list. |
| MAT-13 replay/perf/bug hunt | `cf-headless`, `cf-bench`, `tools/`, vault | Headless replay checksum for material kernel (per-chunk material checksums + reaction event order). Perf report. First-divergence report on mismatch. Bug-hunt log. | Replay verify; 1080p/60 active-region budget pass; perf bench captured. | Prototype note under `prototypes/` with final audit. | No "works once" completion. |
| MAT-control `cfctl observe --materials/--reactions` | `cf-control`, `cfctl`, `cf-material` | `cfctl observe --materials --stream --hz 30 --scope chunk:<x>,<y>` and `cfctl observe --reactions --stream --hz 30` and `cfctl inspect material/reaction <event-id>` per [[spec/prototype-roadmap#CLI Reference|Roadmap CLI Reference]]. | CLI snapshot tests; stream freshness tests. | Observation samples in run notes. | No screenshot-only material debugging. |

M5.6 E2E:

```bash
cargo run -p cf-e2e -- --scenario m5_6_material_kernel --suite MAT-01,MAT-02,MAT-03,MAT-06,MAT-13 --write-run-bundle
cargo run -p cfctl -- observe --materials --stream --hz 30 --scope chunk:0,0
cargo run -p cfctl -- observe --reactions --stream --hz 30
cargo run -p cf-headless -- replay prototype_runs/native/<m5_6_run> --verify-checksums
cargo run -p cf-bench -- --scenario m5_6_material_kernel --profile material --runs 100 --check-checksum-stability
cargo run -p cf-mod -- validate content/materials/ --strict
```

Human gate: project-owner may play the kernel scenarios for feel, but all MAT-* tests are agent-completable.

---

## M5.7 — Hazard Package

> [!important] Kickoff prerequisites
> M5.6 must be complete (kernel + reaction table + density layering + phase change). M5.5 collision must route impulse-to-damage so material hazards can route through that same path. Read [[decisions/dr-036-systemic-material-simulation-direction]] §Hazard Coverage and the linked Barotrauma posture in [[comparables/noita-grade-material-simulation-research]]. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M5.7 Kickoff Smoke]]. M5.7 is not done until MAT-04, MAT-05, MAT-07 pass and MAT-08 stub lands with the affliction model wired to HUD.

> [!warning] Hard rules
> Every hazard death must have a replay-visible cause chain (`material.*` → `reaction.*` → `damage.*` → `actor.*`). No invisible/instant lava death; pre-warning audio + hazard overlay required. AI must have an affordance tag for every hazard before that hazard ships in a mission (M6.6 gate). Afflictions are a state layer on the actor, not free-floating effect blobs.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| MAT-04 acid hazard | `cf-material`, `cf-actor`, `cf-equipment`, `cf-chassis` | Acid material with corrosion-over-time damage, armor degradation, equipment jam chance, neutralization by water/base reactions. Affliction `corroded`. | Acid pool damage fixture; armor-zone degrade test; neutralization test. | `material.acid_contact`, `damage.acid_applied`, `affliction.corroded_set`; HUD shows affliction. | No instant-kill acid; no acid that bypasses armor without replay event. |
| MAT-05 electricity hazard | `cf-material`, `cf-actor`, `cf-equipment` | Electricity charge propagates through conductive materials (water, metal), arcs to grounded actors, stuns or damages, can ignite oil/fuel. Affliction `electrified`. | Conductivity chain test (water-metal-actor); arc-jump fixture; ignition cross-test with oil. | `material.electricity_arc`, `damage.electric_applied`, `affliction.electrified_set`. | No magic chain-lightning; conductivity follows material registry. |
| MAT-07 debris/blunt impact | `cf-material`, `cf-physics`, `cf-chassis` | Debris/pebble materials behave as small physical bodies with impulse-to-damage routing through M5.5; collapse cascades for stacked debris. Affliction `concussed` for repeated head impacts. | Debris pile collapse test; impact damage threshold test by material/sharpness. | `damage.debris_impact_applied`; collapse events. | No granular sand fluid here (deferred to expansion). |
| MAT-08 ingestion stub | `cf-actor`, `cf-material` | Stub: actor ingestion model for toxic gas / smoke / acid mist with afflictions `asphyxiating`, `poisoned`, `coughing`. Stub: route through breathing model placeholder; full network behavior lands in M7.5. | Toxic-gas exposure fixture; affliction stack test. | `affliction.poisoned_set`, `affliction.asphyxiating_set`. | No full pulmonary sim; full atmospheric routing waits for M7.5. |
| MAT-affliction layer | `cf-actor`, `cf-ui` | Per-actor affliction state (`wetness`, `burning`, `corroded`, `electrified`, `poisoned`, `asphyxiating`, `concussed`). Visible on HUD. Decay/clear rules per material registry. | Affliction stack tests; HUD render test; decay tests. | `affliction.*` events; HUD screenshot. | No invisible afflictions; no permanent uncleanable afflictions outside design intent. |
| MAT-hazard overlay UI | `cf-ui`, `cf-render-2d`, `cf-material` | Hazard overlay (color-blind safe) for acid/electricity/fire/toxic gas/lava + caption hooks. Toggle key + accessibility default-on for low-vision profiles. | Overlay screenshot tests at 100% and 200% UI scale; high-contrast mode test. | Hazard overlay screenshots in M5.7 run bundle. | No color-only hazard signaling. |

M5.7 E2E:

```bash
cargo run -p cf-e2e -- --scenario m5_7_hazard_package --suite MAT-04,MAT-05,MAT-07,MAT-08-stub --write-run-bundle
cargo run -p cf-headless -- replay prototype_runs/native/<m5_7_run> --verify-checksums
```

Human gate: human playtester confirms hazard signal-to-death readability and HUD legibility before promotion to M6.

---

## M6 — AI Core And Trust Harness

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M6-001 perception/memory | `cf-ai`, `cf-actor`, `cf-replay` | Sight, hearing, last-known memory, forgetting. | Perception unit tests; occlusion tests. | `ai_perception_signal` events. | No LLM runtime dependency. |
| M6-002 utility/doctrine | `cf-ai` | Utility scoring and 4-6 doctrine profiles. | Scoring tests with deterministic fixtures. | `ai_tactic_scored`, `tactic_chosen`. | No full strategic commander yet. |
| M6-003 mistakes/recovery | `cf-ai`, `cf-replay` | Panic/hesitate/miss/stuck/recover behavior with reason labels. | Recovery scenario tests. | `ai_recovery_action`. | No fake randomness without causes. |
| M6-004 AI-H harness | `cf-ai`, `cf-headless`, `tools/` | Runnable AI-H-01..06 suite with report output. | Harness pass/fail tests. | AI-H report bundle. | No broad campaign AI. |
| M6-005 bot overlay | `cf-ui`, `cf-ai` | Visible intent labels for friendly/enemy bots. | Screenshot capture. | Overlay screenshot. | No dialogue system. |
| M6-006 mind hooks (T-LLM bridge) | `cf-ai` | Expose hook points that the future M6.5 mind layer will call: utility-weight patch API, commander-blackboard goal API, doctrine-tag set API, dialogue-queue API, memory-write API. M6 itself MUST NOT call any LLM. | Hook tests with synthetic patches; AI-H stays green when no hooks are called. | Hook trait docs in `cf-ai::doctrine`; example synthetic patch in tests. | No LLM runtime dependency in M6. |

M6 E2E:

```bash
cargo run -p cf-ai --bin ai_harness -- --suite AI-H-01..AI-H-06 --write-run-bundle
```

---

## M6.5 — LLM Mind Lab

> [!important] Kickoff prerequisites
> M6 must be complete (including M6-006 hook points). Read [[spec/hybrid-llm-ai-plan]] and [[decisions/dr-032-hybrid-llm-ai-direction]] in full BEFORE feature work. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M6.5 Kickoff Smoke]]. M6.5 is not done until MIND-001..MIND-010 pass against the deterministic mock provider, replay shows `mind` events, and local AI keeps acting through provider sleep/fail/stale/cost-cap.

> [!warning] Hard rules
> No live cloud LLM is required for any test. CI uses the mock provider only. Cloud/local provider adapters are cargo-feature-gated. Local AI MUST keep acting when the provider is disabled, sleeping, failing, returning malformed/stale output, or exhausted of budget. **Anti-goal: No LLM in the reflex/tactical loop.**

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M6.5-001 mind schemas | `cf-ai::mind::schema`, `game/crates/cf-ai/schemas/mind/v1/` | Define `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, `MindProviderConfig` per [[spec/hybrid-llm-ai-plan]]; emit JSON Schemas via `schemars`. | Roundtrip tests; bad-example rejection tests; schema-version mismatch test. | Schemas committed; example proposal validates. | No public schema export yet. |
| M6.5-002 mock provider | `cf-ai::mind::provider::mock` | Deterministic provider that consumes a canned-script directory; supports inject-canned, inject-malformed, inject-timeout, inject-stale, inject-cost-overflow modes. | Per-mode tests; CI uses mock only. | Mock provider used by all MIND-* tests. | No live cloud calls in mock. |
| M6.5-003 provider trait + adapters | `cf-ai::mind::provider` (cargo features `mind-openai`, `mind-anthropic`, `mind-ollama`, `mind-openai-compatible`) | Shared async trait; OpenAI Responses API adapter; Anthropic Messages API adapter; Ollama adapter; OpenAI-compatible adapter (vLLM/llama.cpp); each behind a cargo feature; secrets read from env per `MindProviderConfig.api_key_env`. | Adapter contract tests with mocked HTTP; feature-gate tests verify default build excludes cloud. | Adapter docs; example `MindProviderConfig`. | No vendor SDK lock-in; no API keys in repo. |
| M6.5-004 observation compressor | `cf-ai::mind::compressor`, `cf-control`, `cf-replay` | Derive `MindObservationFrame` from the `cf-control` observation stream + recent replay events; enforce fog-of-war BEFORE any provider sees a prompt. | Fog-of-war audit tests (synthetic hidden enemy never appears in frame); compactness tests; `cfctl observe --mind-frame <scope>` smoke. | Sample frames in run notes. | No raw-state passthrough. |
| M6.5-005 proposal validator | `cf-ai::mind::validator` | Reject stale, invalid, impossible, unfair, over-budget, hidden-info, and capability-violating proposals; replay-visible reasons. | Per-rejection-class unit tests; MIND-003/004/006/009 acceptance pass. | Validator decision log. | No silent acceptance. |
| M6.5-006 policy compiler | `cf-ai::mind::policy` | Convert accepted proposals into utility-weight patches, commander goals, doctrine tags, dialogue-queue entries, and `MindMemoryRecord` writes via M6 hook points. | Patch-application tests; doctrine-patch visibility test (MIND-005). | One visible doctrine patch in micro_breach_mind_lab. | No direct low-level action emission. |
| M6.5-007 mind events + run-bundle integration | `cf-replay`, `cf-ai::mind::events`, `tools/run_bundle_check.py` | Emit `mind.task_created`, `mind.prompt_recorded` (hashes by default; raw text only behind `debug_capabilities`), `mind.response_received`, `mind.proposal_validated`, `mind.patch_applied`, `mind.patch_rejected`, `mind.memory_written`. Update run-bundle checker to recognize the `mind` category. | Bundle-validation tests; secret-redaction tests. | Run bundles include `mind` events; redaction verified. | No raw secrets in run bundles. |
| M6.5-008 mind dashboard (dev) | `cf-tools-editor`, `cf-ui` | Dev-only workbench panel showing task count, stale rate, provider failures, estimated cost, model routing, and accept/reject reasons. | Dashboard render tests; screenshot. | Dashboard capture in M6.5 note. | No player-facing UI yet. |
| M6.5-009 micro_breach_mind_lab scenario | `content/scenarios/micro_breach_mind_lab.ron`, `content/mind/profiles/` | The M6.5 lab scenario in three modes (`mind_off`, `mind_mock`, `mind_live_optional`) with a sample commander mind profile and one designed doctrine-patch opportunity. | Scenario validates with `cf-mod validate`; all three modes load. | Scenario file + sample profile + canned-script. | No content tied to a specific cloud model id. |
| M6.5-010 MIND-* acceptance suite | `cf-ai`, `cf-headless`, `cf-bench`, `tests/` | Implement `cf-ai --bin mind_lab` with `--suite MIND-001..MIND-010 --provider <mock|...> --write-run-bundle`. Cover: baseline (off), nonblocking timeout, malformed response, stale response, doctrine-patch visibility, fog-of-war fairness, memory write, replay audit, cost cap, humanlike-score delta. | All MIND-* pass against mock; AI-H regression remains green; failure modes produce useful first-divergence reports. | MIND-001..MIND-010 run bundles archived; AI-H humanlike-score delta report. | No reliance on live cloud during CI. |

M6.5 E2E:

```bash
cargo run -p cfctl -- observe --mind-frame squad_alpha --once
cargo run -p cf-ai --bin mind_lab -- --suite MIND-001..MIND-010 --provider mock --write-run-bundle
cargo run -p cf-headless -- replay prototype_runs/native/<m6_5_run> --verify-checksums
```

Human gate: **none**. M6.5 is fully agent-completable; humans review the audit report.

---

## M6.6 — AI Material Competence

> [!important] Kickoff prerequisites
> M6 (AI core + AI-H harness) and M5.6 + M5.7 must be complete (kernel + reaction table + hazards + afflictions). Read [[decisions/dr-036-systemic-material-simulation-direction]] §AI Material Competence and [[decisions/dr-022-ai-humanlike-bar]]. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M6.6 Kickoff Smoke]]. M6.6 is not done until AI-MAT-01..AI-MAT-08 pass and AI-H-01..06 regression remains green.

> [!warning] Hard rules
> AI must respect fog-of-war on hazard perception (AI cannot perceive a toxic gas plume on the other side of a wall the player can't see either). Affordance tags drive utility scoring; no hand-coded "always run from acid" overrides. AI mistakes around hazards are allowed (per DR-022 humanlike imperfection) but they MUST emit reason labels (`hazard_unknown`, `hazard_underestimated`, `hazard_traded_for_objective`).

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| AI-MAT-01 hazard perception map | `cf-ai`, `cf-material` | Per-actor sampled hazard map of nearby material/temperature/charge/gas fields, fog-of-war respected. Updates on tick budget. | Fog-of-war audit (hidden hazard never appears in map); update-budget test. | `ai_hazard_map_updated` events. | No raw-state passthrough; no global omniscience. |
| AI-MAT-02 affordance tag wiring | `cf-ai`, `cf-material` | Material registry affordance tags (`avoid`, `seek`, `use-as-weapon`, `extinguish-with`, `neutralize-with`, `vent`, `pump`) consumed by utility scoring. | Per-tag scoring fixture; doctrine override fixture. | `tactic_scored` reason labels include affordance tag id. | No hard-coded material strings outside registry. |
| AI-MAT-03 hazard avoidance | `cf-ai`, `cf-actor` | Pathfinding prefers safe routes; replans on hazard detection; emits reason labels on detour. | Acid-pool detour fixture; electrified-water detour fixture; toxic-gas detour. | `ai_path_avoided_hazard` events. | No infallible avoidance; humanlike misses allowed with reason labels. |
| AI-MAT-04 hazard exploitation | `cf-ai`, `cf-equipment` | AI uses water against burning enemies, acid against unarmored, electricity against grouped enemies in water; doctrine-tag-gated. | Per-doctrine fixture (`improviser` exploits; `defensive` does not). | `tactic_chosen` events with `exploit_hazard:<id>` reason. | No griefing-only doctrines; no autoplay puzzle solver. |
| AI-MAT-05 friendly-fire safety | `cf-ai` | AI checks team membership before exploiting hazards on friendlies; emits reason labels on near-miss; escalates by doctrine. | Friendly-in-blast fixture; `cautious` vs `aggressive` doctrine difference. | `ai_friendly_fire_check` events. | No magic "I just don't shoot teammates" override; check is real and visible. |
| AI-MAT-06 self-preservation | `cf-ai`, `cf-actor` | AI extricates from hazards: ignites → drop weapon + dive into water; drowning → climb; corroded → equip swap; electrified → break contact. | Per-affliction recovery fixture. | `ai_recovery_action` events with `affliction:<id>` reason. | No instantaneous teleport-out; recoveries are realistic actions. |
| AI-MAT-07 base atmospherics awareness (stub) | `cf-ai`, `cf-atmos` (stub binding) | Stub: AI consumes future M7.5 atmosphere telemetry; for M6.6 the stub is a fake provider that returns synthetic hull state; verify scoring path works end-to-end. | Synthetic hull state fixture; scoring path test. | Stub trait + tests; wired-up gate for M7.5. | No real M7.5 work shipped here. |
| AI-MAT-08 reason label coverage | `cf-ai`, `cf-replay` | Every hazard-related decision emits a parent-linked replay reason label drawn from a closed enum (`hazard_unknown`, `hazard_underestimated`, `hazard_traded_for_objective`, `hazard_avoided`, `hazard_exploited`, `hazard_recovered`, `friendly_fire_avoided`). | Reason-label coverage test (no decisions emit `unknown` outside enum). | Reason-label histogram in run note. | No free-text reasons; closed enum enforced. |

M6.6 E2E:

```bash
cargo run -p cf-ai --bin ai_harness -- --suite AI-MAT-01..AI-MAT-08 --write-run-bundle
cargo run -p cf-ai --bin ai_harness -- --suite AI-H-01..AI-H-06 --write-run-bundle  # regression
cargo run -p cf-headless -- replay prototype_runs/native/<m6_6_run> --verify-checksums
```

Human gate: human playtester rates AI competence around hazards on the DR-022 humanlike scale; result archived in run note.

---

## M7 — Mission Director And Breach Contract

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M7-001 manifest schema | `cf-mission`, `content/scenarios/` | Typed manifest: teams, objectives, materials, command core, base systems, loadout requirements, director. | Schema validation tests. | Manifest fixture in bundle. | No full campaign generator. |
| M7-002 director/commander | `cf-mission`, `cf-ai` | Pacing, reinforcement, LZ risk, commander reason labels. | Director phase tests. | `commander_decision.*`. | No MMO war layer. |
| M7-003 command core/base slice | `cf-mission`, `cf-chassis`, `cf-ui` | Rooted core powers shield/turret/door/repair; uproot/embed avatar tradeoff. | CORE-A subset tests. | `command_core_state_changed`, `base_power_changed`. | No full base builder. |
| M7-004 Breach Contract | `content/scenarios/`, `cf-app` | Playable mission: breach, fight, extract, win/loss/debrief. | E2E win/loss; replay. | MISSION-A run bundles. | No campaign map. |
| M7-005 debrief/retry | `cf-ui`, `cf-replay`, `cf-save` | Comic-noir debrief with cause chain and retry same seed. | UI/replay tests. | Debrief screenshot. | No full progression system. |

M7 E2E:

```bash
cargo run -p cf-e2e -- --scenario breach_contract --script win_path --expect win --write-run-bundle
cargo run -p cf-e2e -- --scenario breach_contract --script core_loss --expect loss --write-run-bundle
```

Human gate: project-owner plays five runs and records verbatim reaction.

---

## M7.5 — Base Atmospherics

> [!important] Kickoff prerequisites
> M5.6 (material kernel) and M7 (mission director + Breach Contract) must be complete. Read [[decisions/dr-036-systemic-material-simulation-direction]] §Base Atmospherics and the Barotrauma posture in [[comparables/noita-grade-material-simulation-research]]. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M7.5 Kickoff Smoke]]. M7.5 is not done until MAT-09 + MAT-10 pass and the mission director can author room-state objectives.

> [!warning] Hard rules
> Approximate consistent rules; not a real-unit physics simulation. Hull/gap/pump/vent/oxygen/pressure/fire networks must be replay-deterministic and inspectable via `cfctl observe --atmospheres`. Server-authoritative atmosphere state (DR-005 / DR-034 / DR-035). Mission director authors objectives in terms of hull state (`pressurize hull H3`, `vent toxic gas from hull H7`); `cf-mod` validates hull/gap topology at scenario load.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| MAT-09 hull/gap topology | `cf-atmos`, `cf-mission`, `content/scenarios/` | Per-scenario hull volumes + gap connections. Each hull tracks water level, oxygen level, pressure, fire state, toxic gas mass. Gaps carry flow state + force. Topology validation at scenario load. | Topology roundtrip; isolated/connected hull tests; gap-state transition tests. | `atmosphere.hull_state_changed`, `atmosphere.gap_opened/closed` events. | No real Navier-Stokes; no per-pixel atmosphere. |
| MAT-10 atmosphere flow + breach | `cf-atmos`, `cf-mission` | Hull flooding (water flows down through gaps), oxygen depletion, pressure equalization, breach handling (sudden gap opening to outside). Pump + vent + door entities with admin actions; flow rates from material registry. | Flooding rate test; pressure equalization test; breach decompression test; pump/vent action tests. | `atmosphere.*` events visible in M7.5 run bundle. | No real-unit pressure (use approximate scalar 0..1 normalized to design intent). |
| MAT-11 fire + smoke network (atmospheric) | `cf-atmos`, `cf-material` | Fire propagates room-to-room through gaps using oxygen/fuel; smoke fills connected hulls; toxic gas migrates by gap pressure. Fires emit heat that triggers M5.6 phase changes (water → steam). | Fire spread fixture; smoke fill fixture; oxygen depletion via fire. | Fire propagation events; smoke event chain. | No purely cosmetic smoke; smoke must affect breathing model. |
| MAT-mission director hull objectives | `cf-mission`, `cf-atmos` | Mission director can author objectives like `pressurize <hull>`, `oxygenate <hull>`, `extinguish <hull>`, `vent toxic <hull>`, `flood <hull>`. Reason labels per commander decision. | Per-objective fixture in M7.5 scenario. | `mission.objective_progress` events with hull id. | No campaign generator. |
| MAT-atmosphere observation | `cf-control`, `cfctl`, `cf-atmos` | `cfctl observe --atmospheres --stream --hz 5 --scope <room-id\|all>` exposes hull state stream + per-hull inspect. | CLI snapshot tests; stream freshness tests. | Atmosphere observation samples in run notes. | No screenshot-only debugging. |
| MAT-server authority (atmos) | `cf-server`, `cf-atmos` | Server-authoritative atmosphere state per DR-005 / DR-034 / DR-035; client receives atmosphere snapshots, never simulates locally. | Server-vs-client divergence test (synthetic). | Replay determinism test passes for atmosphere. | No client-side authoritative hull state. |

M7.5 E2E:

```bash
cargo run -p cf-e2e -- --scenario m7_5_base_atmospherics --suite MAT-09,MAT-10 --write-run-bundle
cargo run -p cfctl -- observe --atmospheres --stream --hz 5 --scope all
cargo run -p cf-headless -- replay prototype_runs/native/<m7_5_run> --verify-checksums
cargo run -p cf-bench -- --scenario m7_5_base_atmospherics --profile material --runs 50 --check-checksum-stability
```

Human gate: project-owner plays a flooding-and-repair scenario and confirms readability of room-state objectives.

---

## M8 — Scenario Editor And Mod Tools

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M8-001 editor workbench | `cf-tools-editor`, `cf-ui` | In-engine editor for spawns, materials, objectives, core/base state, loadout requirements. | Editor state tests; focus/accessibility smoke. | Editor screenshots. | No marketplace. |
| M8-002 package builder | `cf-mod`, `tools/` | Deterministic `.cfpkg`, manifest/provenance validation, dependency graph. | Package determinism tests. | PACK-A report. | No public hosting. |
| M8-003 script host | `cf-mod` | Implement chosen Lua/Rhai sandbox with capability declarations. | Sandbox denies FS/network by default. | Script-host test report. | No unbounded script API. |
| M8-004 sample mod | `mods/sample_*`, `content/` | New chassis + scenario + AI doctrine sample mod. | Validate/load/run sample mod. | Modded run bundle. | No full mod catalog. |

M8 E2E:

```bash
cargo run -p cf-mod -- validate content/ mods/ --strict
cargo run -p cf-e2e -- --scenario sample_mod_breach --expect win --write-run-bundle
```

---

## M8.5 — Material Lab

> [!important] Kickoff prerequisites
> M5.6, M5.7, M6.6, M7.5, M8 must be complete (kernel + reactions + hazards + AI competence + atmospherics + scenario editor). Read [[decisions/dr-036-systemic-material-simulation-direction]] §Material Lab and §Authoring Discipline. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M8.5 Kickoff Smoke]]. M8.5 is not done until MAT-11 + MAT-14 pass and a designer authors + exports + reloads a working material puzzle in <10 minutes.

> [!warning] Hard rules
> Material lab is the gate for adding materials beyond the launch 17. New materials require: inspect overlay sample, AI affordance tag declared, replay event payload, recipe journal entry, accessibility caption. `cf-mod validate --strict` rejects packs missing any of these. No "dump 50 materials and see what works" pattern.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| MAT-11 material lab workbench | `cf-tools-editor`, `cf-ui`, `cf-material` | New `--mode material_lab` for `cf-tools-editor`: brush palette (paint material, paint temperature, paint charge), inspect tool (pixel readout), recipe journal (which reactions designer triggered), stamp library (water-on-fire, acid-on-flesh, electricity-in-water), AI puppet test (drop a bot in lab and check affordance scoring). | Editor state tests; brush + stamp tests; AI puppet test. | Material lab screenshots in M8.5 run bundle. | No marketplace. |
| MAT-12 expansion materials gate | `cf-mod`, `content/materials/expansion/` | Schema requirement for any non-launch material: inspect overlay color, AI affordance tags, replay event payload, recipe journal entry, hazard caption. `cf-mod validate --strict` rejects packs missing any. | Per-required-field validation test; rejection diagnostics test. | Schema audit report. | No relaxation of launch-set rules. |
| MAT-14 material pack mod | `mods/sample_material_pack/`, `content/` | A sample mod pack adding 1-2 expansion materials (e.g., slime, foam) with full schema compliance, AI affordance, replay events, hazard captions. Loads cleanly via M8 mod tools. | Validate/load/run sample material pack; AI puppet test passes for new affordance. | Modded run bundle with new material. | No half-spec'd mod packs that bypass launch-rules. |
| MAT-recipe journal | `cf-tools-editor`, `cf-ui` | In-engine recipe journal: shows "you just triggered: water + electricity → arc; oil + heat → fire". Persists across editor sessions. Exportable as content fragment for scenario hints. | Journal write/read tests; export roundtrip. | Journal screenshots. | No autoplay puzzle solver. |
| MAT-acid puzzle scenario | `content/scenarios/m8_5_acid_trap_puzzle.ron` | Designer authors a tiny puzzle scenario: acid pool blocks path; player must find oil + neutralize. Authored entirely in material lab + scenario editor; tested against M6 AI puppet. | Scenario validates; E2E win/loss; designer authors in <10 minutes (timed). | Authoring transcript + checked run bundle. | No campaign generator. |

M8.5 E2E:

```bash
cargo run -p cf-tools-editor -- --mode material_lab --scenario m8_5_acid_trap_puzzle --suite MAT-11,MAT-14 --write-run-bundle
cargo run -p cf-mod -- validate mods/sample_material_pack/ --strict
cargo run -p cf-headless -- replay prototype_runs/native/<m8_5_run> --verify-checksums
```

Human gate: a non-AI human designer authors the m8_5_acid_trap_puzzle scenario in <10 minutes; transcript archived in run note.

---

## M9 — Dedicated Server App + Determinism Islands

> [!important] Kickoff prerequisites
> M3 (replay/event recorder) and M7 (mission director + Breach Contract) must be complete. Read [[spec/server-app-architecture]], [[decisions/dr-005-multiplayer-posture]], [[decisions/dr-013-backend-service-scope]], [[decisions/dr-034-dedicated-server-application]] in full BEFORE feature work. Run [[spec/prototype-roadmap#Per-Milestone Kickoff Smoke|M9 Kickoff Smoke]]. M9 is not done until the M9 server-core subset passes and the reference Docker image runs unchanged. PvP/MMO scale gates stay in M12.

> [!warning] Hard rules
> Same sim path as the client. No "server-only" branch of game logic. Server-authoritative for sim, terrain mutation, AI decisions, mission director, persistence. No proprietary cloud database dependency. Networking transport library decision committed at M9 close.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M9-001 cf-headless sim runner | `cf-headless`, `cf-app` | Headless sim runner used by replay verification + CI; same sim, no renderer/audio; loads scenario; accepts scripted inputs. | Linux headless smoke; replay verification. | Headless logs in bundle; replay verification report. | No public server browser. |
| M9-002 determinism contracts | `cf-sim-core`, `cf-replay`, docs | Document deterministic/stochastic/cosmetic subsystems; contract tests for each. | Contract tests. | Determinism report. | No whole-engine determinism claim. |
| M9-003 replay-from-events | `cf-headless`, `cf-replay` | 10-minute M7 replay verifies actor/terrain/inventory checksums. | Replay compare. | First-divergence report on fail. | No network sync yet. |
| M9-004 headless perf | `cf-bench`, `cf-headless` | 10x real-time replay validation target. | Bench test. | Perf report. | No optimization-only rabbit hole. |
| M9-005 cf-server binary scaffold | `cf-server` (bin), `cf-server-ops` | New binary that pulls all sim crates (no render/audio/UI); RON config loader; `--mode <coop_room\|pvp_arena\|lan_room\|mmo_shard\|lobby_directory>` flag; `--validate-config-only` exit. | Config validator tests; mode-flag dispatch tests. | `cf-server --validate-config-only` smoke; CLI help output captured. | No production hosting yet. |
| M9-006 server lifecycle (cf-server-ops) | `cf-server-ops` | Health (`/health`), readiness (`/ready`), Prometheus-compatible metrics endpoint, structured JSON logs, drain shutdown (SIGTERM = graceful client disconnect within 10s + replay flush + persistence save), restart hooks. | Lifecycle integration test; SIGTERM + clean-exit test; metrics endpoint smoke. | Logs + metrics capture in M9 run bundle. | No log-aggregation product. |
| M9-007 cf-server-anti-cheat foundation | `cf-server-anti-cheat` | Profile registry (`casual`, `competitive`, `tournament_strict`), input rate limit hooks, replay drift detection skeleton, ban list persisted, audit log appended (`system.anti_cheat_*` events). | Profile-load tests; rate-limit unit tests; ban-list roundtrip. | Anti-cheat audit log sample in M9 run bundle. | No tournament-grade anti-cheat. |
| M9-008 cf-server-persistence foundation | `cf-server-persistence` | Snapshot writer (atomic temp + rename) + append-only event journal + restore loop; rolling backup; schema-versioned with migration handler hooks. | Snapshot/restore roundtrip; corruption-on-mid-write recovery. | Persistence sample in M9 run bundle. | Full MMO persistence is M12. |
| M9-009 cf-server-admin API | `cf-server-admin`, `cfctl` | Capability-gated admin endpoints over the same JSON-RPC envelope as `cfctl` (kick, save, restart, hot-load scenario). Default `admin` capability OFF. | Admin API auth/capability tests. | Admin command transcripts in M9 note. | No silent admin mutations. |
| M9-010 cf-net authority + transport | `cf-net`, `cf-sim-core` | Server-authoritative input/snapshot/event model; trait-bound transport adapter with `lightyear`/`renet`/`quinn` candidates; networking transport library decision committed at M9 close. | Unit tests for input validation; transport-trait contract tests. | Networking decision document committed to vault; authority memo. | No platform lock-in. |
| M9-011 coop_room mode | `cf-server`, `cf-net`, `cf-mission` | `cf-server --mode coop_room --scenario breach_contract` boots, accepts 2-4 clients, runs the mission, archives a per-session run bundle. | SERVER-001 acceptance test. | Co-op room run bundle. | No NAT/relay yet (M11). |
| M9-012 lan_room mode | `cf-server`, `cf-net` | LAN auto-discovery (mDNS/UDP broadcast); ready-up. | SERVER-003 acceptance test. | LAN room run bundle. | No public list. |
| M9-013 pvp_arena mode skeleton | `cf-server`, `cf-mission` | Mode boots with `pvp_arena` config; server-authoritative match scaffolding (full PvP gameplay lands in M12). | SERVER-002 boot test (gameplay tests in M12). | PvP arena boot run bundle. | No PvP scenario design. |
| M9-014 mmo_shard mode skeleton | `cf-server`, `cf-server-persistence` | Mode boots with empty world manifest; persistence snapshot every 10 min; restart restore <30 s. (Full MMO acceptance is M12.) | MMO-001 + MMO-002 boot/restore tests. | MMO shard boot run bundle. | No 50-100 client load yet. |
| M9-015 lobby_directory mode skeleton | `cf-server` | Mode lists registered shards via REST + WebSocket schema; multi-instance protocol. | SERVER-005 boot test. | Lobby directory boot capture. | No moderation. |
| M9-016 server mod loading | `cf-server`, `cf-mod` | Server loads same `cf-mod` package format as client; mod hash recorded; `server_only: true` packages allowed. | Server-side mod load test. | Mod load report. | No auto-download. |
| M9-017 reference Docker image | `tools/`, `docs/server-hosting.md` | Minimal Docker image runs `cf-server` unchanged; documented hosting guide for Linux + Windows. | Docker image smoke. | Image manifest + hosting guide. | No production registry. |
| M9-018 server-core acceptance suite | `cf-e2e`, `tests/`, `cf-server` | Implement and pass the M9 server-core subset from [[spec/server-app-architecture]]: SERVER-001, SERVER-006, SERVER-009, SERVER-010, SERVER-011, SERVER-014, SERVER-015, SERVER-016. Track SERVER-002/004/012 as M12 gates. | M9 server-core subset passes. | M9 run bundle. | No premature PvP/MMO scale acceptance. |

M9 E2E:

```bash
cargo run -p cf-server -- --mode coop_room --scenario breach_contract --ticks 36000 --write-run-bundle
cargo run -p cf-server -- --mode lan_room --auto-discover --validate-config-only
cargo run -p cf-server -- --mode mmo_shard --bootstrap-empty-shard
cargo run -p cf-server -- --mode lobby_directory --validate-config-only
cargo run -p cf-headless -- replay prototype_runs/native/<m9_run> --verify-checksums
docker run --rm cf-server:latest --validate-config-only
```

Human gate: optional. Project owner can manually host a `coop_room` and play a Breach Contract; SERVER-001..016 are agent-completable.

---

## M10 — LAN Co-op

> [!important] Kickoff prerequisites
> M9 dedicated server scaffold must be complete (M9-005 through M9-018). Read [[decisions/dr-005-multiplayer-posture]] + [[decisions/dr-034-dedicated-server-application]]. Run M10 kickoff smoke.

> [!warning] Hard rules
> All clients use `cf-control` envelope; no OS-level input automation in tests. Per-client run bundles MUST align tick-for-tick under `cf-headless replay-compare`. Mod hash sync mandatory.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M10-001 LAN host flow | `cf-server`, `cf-net`, `cf-ui` | Client UI launches `cf-server --mode lan_room` as a child process; ready-up; mission start. | Local two-client smoke (host + 1 join). | Lobby screenshot; ready-up event log. | No NAT/relay. |
| M10-002 replication | `cf-net`, `cf-replay` | Actors, terrain, inventory, objective state, base modules replicate via snapshot/event hybrid; interest filter at LAN scope (everything visible). | Replay compare across clients. | Two-client run bundles aligned. | No public matchmaking. |
| M10-003 anti-cheat profile `casual` | `cf-server-anti-cheat`, `cf-server` | LAN default profile logs anomalies but does not kick; verify foundation hooks fire. | Anti-cheat foundation tests. | Audit log sample. | No tournament-grade anti-cheat. |
| M10-004 mod hash sync UI | `cf-mod`, `cf-ui` | Join preflight checks package set + manifest hashes; mismatch produces clean diff UI. | Mismatch fixture tests. | Join-failure screenshot. | No auto-download (M11). |
| M10-005 friendly fire policy | `cf-mission`, `cf-server` | Configurable friendly-fire flag per scenario manifest; defaults per DR-018 consequence ladder. | Scenario policy tests. | Friendly-fire configuration capture. | No global PvP at LAN scope. |
| M10-006 LAN co-op proof mission | `content/scenarios/`, `cf-app`, `cf-server` | Two clients survive one 5-minute Breach Contract via `cf-server --mode lan_room`; per-client bundles align tick-for-tick. | Full E2E. | Per-client run bundles + alignment report. | No public WAN. |

M10 E2E:

```bash
cargo run -p cf-server -- --mode lan_room --scenario breach_contract --auto-discover &
cargo run -p cf-app -- --connect-lan auto --client-id alpha --write-run-bundle
cargo run -p cf-app -- --connect-lan auto --client-id bravo --write-run-bundle
cargo run -p cf-headless -- replay-compare prototype_runs/native/<alpha> prototype_runs/native/<bravo>
```

Human gate: project-owner LAN co-op session of one Breach Contract.

---

## M11 — Online Co-op (Self-Hosted Dedicated Servers)

> [!important] Kickoff prerequisites
> M9 + M10 complete. Networking transport library committed (M9-010). Read [[spec/server-app-architecture]] hosting posture + DR-005 + DR-013 + DR-034.

> [!warning] Hard rules
> A community member must be able to host an internet-reachable server with documented hosting steps. Steam/EOS adapters are optional cargo features. `lobby_directory` registration must work end-to-end. Mod hash sync is mandatory.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M11-001 NAT/relay transport | `cf-net` | NAT punch-through or relay using committed transport (lightyear/renet/quinn); fallback to TCP for restricted networks. | Simulated latency + packet-loss tests. | Transport decision document; latency tests. | No platform lock-in. |
| M11-002 lobby_directory integration | `cf-server`, `cf-net`, `cf-ui` | Server registers + heartbeats + deregisters; client browses + filters + joins. Multi-instance protocol. | Registration roundtrip; heartbeat expiry; deregister cleanup. | Registry capture; browse UI screenshot. | No first-party hosted directory. |
| M11-003 anti-cheat profile `competitive` | `cf-server-anti-cheat` | Default profile for online co-op; rejects input-rate-spike clients; ban list persisted across restart; audit log. | Anti-cheat acceptance fixture; ban-list roundtrip. | `system.anti_cheat_kicked` event in run bundle. | No tournament-grade. |
| M11-004 latency compensation | `cf-net`, `cf-actor` | Client-side prediction + server reconciliation for player actor; pure replication for AI bots. Tunable interpolation factor. | Latency-masked input tests at 50-150 ms RTT. | Latency masking screenshots/captures. | No predictive AI. |
| M11-005 package hash sync (production) | `cf-mod`, `cf-net` | Server checks client packages match; soft-fail with auto-download for dev workflow; hard-fail with mismatch report for shipping. | Mismatch fixture tests; auto-download dev path test. | Join-failure with downloadable diff screenshots. | No public mod CDN. |
| M11-006 account adapter foundation | `cf-server`, `cf-net` | Local account file (private), `lobby_directory` token (community), Steam/EOS/PlayFab adapters stubbed behind cargo features (`net-steam`, `net-eos`, `net-playfab`). | Adapter contract tests; redaction tests for tokens. | Adapter shape doc; redaction-test report. | No first-party identity service. |
| M11-007 Steam Datagram Relay adapter (optional) | `cf-net` (cargo feature `net-steam`) | Behind cargo feature; off by default; documented usage. | Adapter shape contract tests. | Adapter doc. | No Steam-only design. |
| M11-008 reference systemd / launchd / Docker | `tools/`, `docs/server-hosting.md` | Reference deployment templates for self-hosted operators. | Smoke deployment of each. | Templates committed. | No production registry. |
| M11-009 online co-op proof | `cf-server`, `content/scenarios/` | Two friends in different cities co-op a Breach Contract via self-hosted `cf-server`; per-client bundles align. | Cross-host smoke. | Per-client run bundles + alignment report. | Public PvP belongs to M12, not this M11 co-op proof. |

M11 E2E:

```bash
# operator side
cargo run -p cf-server -- --mode coop_room --bind 0.0.0.0:0 --public-address <addr> --lobby-register <directory_url>

# client side (different machine)
cargo run -p cf-app -- --connect <host:port> --client-id alpha --write-run-bundle
cargo run -p cf-app -- --connect <host:port> --client-id bravo --write-run-bundle

cargo run -p cf-headless -- replay-compare <alpha-bundle> <bravo-bundle>
```

Human gate: project-owner self-hosts a `coop_room` and runs an online co-op session.

---

## M12 — Public PvP Arenas + Persistent MMO Shards

> [!important] Kickoff prerequisites
> M9 + M10 + M11 complete. Read [[spec/persistent-mmo-architecture]] + [[decisions/dr-005-multiplayer-posture]] + [[decisions/dr-013-backend-service-scope]] + [[decisions/dr-034-dedicated-server-application]] + [[decisions/dr-035-persistent-mmo-architecture]] in full. Run M12 kickoff smoke.

> [!warning] Hard rules
> Public PvP and persistent MMO shards are full-product target modes, not post-launch-only architecture. M12 proves readiness. MMO shards are community-hostable. No proprietary cloud database. Subscription forbidden. Cross-shard live combat forbidden at v1. Auto-population (server bots dressed as players) forbidden.

PvP arena task cards:

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M12-001 pvp_arena gameplay | `cf-server`, `cf-mission`, `content/scenarios/pvp/` | 2-8 player server-authoritative match server; PvP scenarios; latency-masked client prediction. | 4-8 player stress run. | Match run bundle; bandwidth + cheat notes. | No ranked ladder yet (post-launch). |
| M12-002 PvP anti-cheat | `cf-server-anti-cheat` | `competitive` default; `tournament_strict` opt-in; replay drift detection; rejection events. | Anti-cheat fixture tests; spike-rate kick test. | Audit log sample. | No client-side anti-cheat lock-down. |
| M12-003 PvP perf / bandwidth | `cf-net`, `cf-bench`, `cf-server` | Bandwidth/authority/cheat models tested at 4-8 player density; perf gates per T-PERF. | Bench run. | Perf report. | No infinite-scale PvP. |

MMO shard task cards (per [[spec/persistent-mmo-architecture]]):

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M12-101 mmo_shard world manifest | `content/worlds/`, `cf-mission` | Persistent world manifest schema (region map, materials, hazards, faction territories); validates with `cf-mod validate`. | Schema validation tests. | Sample world manifest. | No seamless world. |
| M12-102 mmo_shard persistence | `cf-server-persistence` | Snapshot every 10 min + append-only journal; restart restore <30 s; crash + restart resumes within 1 min. | MMO-002, MMO-003 tests. | Persistence run-bundle. | No proprietary cloud DB. |
| M12-103 mmo_shard interest management | `cf-net`, `cf-sim-core` | Clients only receive events/snapshots for entities in their interest set; per-client interest range computation server-side. | MMO-009 test; event-volume audit. | Interest-set sample. | No client-side cheating with hidden info. |
| M12-104 mmo_shard account model | `cf-server`, `cf-net` | Token-based bearer; expiry; rotation; never logged. Local account + lobby_directory token + Steam/EOS/PlayFab adapter shapes. | Token redaction tests. | Account adapter doc. | No mandatory account for solo/private. |
| M12-105 mmo_shard mission director | `cf-mission`, `cf-ai` | Per-faction contract pool; mission director generates contracts; players can resume across sessions within timeout. | Director persistence test. | Contract pool sample. | No live cross-shard contracts. |
| M12-106 mmo_shard 50-client soak | `cf-bench`, `cf-server`, `cf-headless` | 50 simulated clients (`cfctl` puppets) connect for 1 hour at ≥30 Hz target. | MMO-004 test. | Soak run bundle + perf report. | No 1000-client moonshot. |
| M12-107 mmo_shard 100-client stretch | `cf-bench`, `cf-server` | 100 simulated clients sustained for 30 minutes; degraded mode acceptable. | MMO-005 test. | Stretch run bundle + degraded-mode report. | No flagship-tier (200+) at v1. |
| M12-108 cross-shard lobby/portal | `cf-server` (lobby_directory mode) | Two shards on different ports; lobby/portal lists both; player log-out from Shard A and log-in on Shard B works. | MMO-006 test. | Cross-shard transcript. | No cross-shard live combat. |
| M12-109 mmo_shard mod compatibility | `cf-mod`, `cf-server` | Per-shard pinned package set + trust tier ceiling; clients see manifest before join; mismatch produces actionable diff; server-only mods allowed. | MMO-007 test. | Mod compatibility doc. | No global mod CDN. |
| M12-110 mmo_shard anti-cheat | `cf-server-anti-cheat` | `competitive` default; operator-tunable; ban list persists; appeals out-of-game per operator policy. | MMO-008 test. | Audit log sample. | No tournament-grade. |
| M12-111 mmo_shard LLM mind | `cf-ai::mind`, `cf-server` | Mind workers run server-side; clients never see prompts; mind events redacted in client-visible event stream per DR-032. | MMO-010 test. | Redaction-test report. | No client-side LLM. |
| M12-112 mmo_shard schema migration | `cf-server-persistence`, `cf-save` | v0.1 shard state loads on v0.2 with declared migration handlers. | MMO-011 test. | Migration registry. | No silent data loss. |
| M12-113 mmo_shard no-cloud reference | `tools/`, `docs/mmo-hosting.md` | Operator runs shard with no proprietary cloud dependency (local FS, local lobby_directory). | MMO-012 test. | Hosting guide. | No publisher-only mode. |
| M12-114 DR-005/013/034/035 review | vault only | Revisit multiplayer + backend + server + MMO postures with M9-M12 evidence. | N/A. | Updated DRs or research log. | No silent scope expansion. |

M12 E2E:

```bash
# PvP
cargo run -p cf-server -- --mode pvp_arena --scenario pvp/breach_arena --max-clients 4 --anti-cheat-profile competitive --write-run-bundle
cargo run -p cf-bench -- --scenario pvp/breach_arena --profile pvp --runs 5 --write-bench-report

# MMO shard
cargo run -p cf-server -- --mode mmo_shard --bootstrap-empty-shard
cargo run -p cf-server -- --mode mmo_shard --scenario mmo/frontier_v1 --simulate-clients 50 --duration-min 60 --write-run-bundle
cargo run -p cf-server -- --mode mmo_shard --scenario mmo/frontier_v1 --simulate-clients 100 --duration-min 30 --write-run-bundle

# Cross-shard
cargo run -p cf-server -- --mode lobby_directory --bind 0.0.0.0:7878 &
cargo run -p cf-server -- --mode mmo_shard --bind 0.0.0.0:9001 --lobby-register http://localhost:7878 &
cargo run -p cf-server -- --mode mmo_shard --bind 0.0.0.0:9002 --lobby-register http://localhost:7878 &

# MMO acceptance suite
cargo run -p cf-e2e -- --suite MMO-001..MMO-012 --write-run-bundle

# MMO replay verification
cargo run -p cf-headless -- replay prototype_runs/native/<m12_mmo_run> --verify-checksums
```

Human gate: project-owner runs a small public shard for at least one session; community-tester feedback captured.

---

## Side Track Injection Rules

These side tracks are not separate "later" workstreams. Every milestone final audit must say which side-track obligations were touched, skipped, or blocked.

| Track | Applies Starting | Agent Obligation | Evidence |
|---|---|---|---|
| T-PLATFORM | M0 | Keep current-platform commands passing; preserve Win/Linux/macOS portability in paths, file watching, case sensitivity, GPU backend assumptions, and input/audio setup. | Validation log; CI log when available. |
| T-CONTROL | M0 | Add semantic control/observation coverage for every new gameplay/UI action; prefer `cfctl` for E2E; record debug capabilities in the manifest. | `cfctl` command log, observation sample, and run-bundle events. |
| T-PHYS | M0 | Any new gameplay object that is physical gets a collision class/proxy/matrix entry/event policy or a tested cosmetic/sensor/filter reason. | Collision matrix diff, `collision.*` event sample, or explicit `collision_filter_reason`. |
| T-SERVER | M0 (config stubs); M9 (full) | Any change that affects multiplayer/server modes/persistence/anti-cheat/admin must extend the `cf-server` config schema or anti-cheat profile registry, register migration handlers if persisted state changes, and audit which `cf-server` modes were touched. | Server config diff, anti-cheat profile change, persistence schema bump, or `lobby_directory` schema change documented per milestone. |
| T-MOD | M5 | Any new gameplay data should be data-driven unless hardcoded as a deliberate prototype shortcut; shortcuts must be named. | Schema/fixture test or explicit shortcut note. |
| T-AUDIO | M4 | Any player-facing combat, mech, command-core, alarm, or UI feedback that needs sound later should emit an event/caption hook now. | Event/caption hook in run bundle. |
| T-SAVE | M5 | New persistent identity/state must define save ownership even if no final save UI exists. | Save/load roundtrip or "not persistent yet" note. |
| T-ACCESSIBILITY | M1.5 visual, M4 required | Do not introduce color-only critical state; check 200% text where UI exists; captions/reduced-motion hooks when audio/VFX are added. | Screenshot/capture and ACC-A status. |
| T-PERF | M0 | Add cheap counters before optimizing: frame time, sim tick cost, event volume; add terrain/AI/network counters when those systems appear. | Perf rows in `summary.json` or bench report. |

---

## Result Writing

Every completed milestone gets a note named like:

```text
cortext_command_vault/prototypes/native-m<id>-<short-name>-<date>.md
```

Required headings:

```markdown
## Summary
## Build Scope
## Validation Commands
## E2E Runs
## Run Bundles
## Screenshots / Captures
## Bugs Found And Fixed
## Known Issues
## Checklist Updates
## Final Audit
## Next Actions
```

The checklist update must list every changed row id from [[spec/feature-completion-checklist]], the evidence link used, and the AI self-ratings assigned. The final audit must state every task card as `pass`, `blocked`, `human-gated`, or `deferred-with-reason`.
