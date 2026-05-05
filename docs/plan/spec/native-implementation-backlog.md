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
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[spec/ai-control-observability-layer|AI control/observability]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [VAULT_PLAN.md](../../VAULT_PLAN.md)

# Native Implementation Backlog

> [!summary] Purpose
> Concrete task-card handoff for the native roadmap. Assign agents work from this page, not from the historical browser/Slice-A backlog. Each milestone card names write scope, implementation work, tests, E2E proof, run-bundle evidence, bug-hunt obligations, and anti-scope.

> [!important] Assignment rule
> Assign one milestone or one contiguous subset of task cards at a time. The agent must not stop at "implemented"; it must run validation, fix failures, bug hunt, emit a run bundle, update vault evidence, and write a final audit.

## Global Rules For Every Task

| Rule | Requirement |
|---|---|
| Read first | `AGENTS.md`, [[spec/prototype-roadmap]] (especially the Glossary, Toolchain And Workspace Bootstrap, CLI Reference, Control Transport And Envelope, Scenario Manifest Schema, Run-Bundle Naming Convention, Per-Milestone Kickoff Smoke), this backlog, and linked DR/spec pages. |
| Work location | Native production prototype lives in `cortex-game/` unless the implementation agent proposes and documents a different path. |
| Reference repos | Never edit CCCP/C4/comparable repos. They are read-only reference material. |
| Crate ownership | Touch only owned crates/files plus explicit boundary crates. Each crate ships an `AGENTS.md` per the [Per-Crate AGENTS.md Template](../spec/prototype-roadmap.md#per-crate-agentsmd-template). |
| Events first | Any behavior that affects player understanding, replay, AI, networking, save, or debugging emits an event. |
| Control first | Every new player-facing control or UI action gets a semantic `cx-control` / `cxctl` path unless explicitly marked human-only. The transport pin is in [Control Transport](../spec/prototype-roadmap.md#control-transport-and-envelope). |
| Tests required | Every new behavior gets unit or integration coverage per [Testing Layers](../spec/prototype-roadmap.md#testing-layers); every player-facing milestone gets E2E proof. |
| Evidence required | Every meaningful run emits a checked run bundle named per [Run-Bundle Naming Convention](../spec/prototype-roadmap.md#run-bundle-naming-convention) under `prototype_runs/native/`. |
| Human gates | If a criterion requires project-owner play, platform hardware, or human accessibility feedback, prepare the build, run the kickoff smoke, fill in the [Human Playtest Checklist Template](../spec/prototype-roadmap.md#human-playtest-checklist-template), and mark `READY_FOR_HUMAN`. |
| Logging/errors | Use the policy in [Logging, Tracing, And Error Policy](../spec/prototype-roadmap.md#logging-tracing-and-error-policy). No `println!`, no `unwrap()` on user-controllable inputs, no `rand::thread_rng` inside sim. |
| Bug log | Found bugs go in the milestone vault note under `## Bugs Found And Fixed` per the [Bug Log Format](../spec/prototype-roadmap.md#bug-log-format). |

## Standard Validation

Run these unless a task card narrows the set:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cxctl -- observe --once
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
> Before feature work, run [M0 Kickoff Smoke](../spec/prototype-roadmap.md#per-milestone-kickoff-smoke). Apply the [Toolchain And Workspace Bootstrap](../spec/prototype-roadmap.md#toolchain-and-workspace-bootstrap) recipe verbatim: `rust-toolchain.toml`, workspace `Cargo.toml`, `rustfmt.toml`, `clippy.toml`, `.cargo/config.toml`, `.gitignore`, `.github/workflows/ci.yml`. M0 is not done until those files exist, the smoke commands pass, and the bundle validates.

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M0-001 workspace scaffold | `cortex-game/Cargo.toml`, `cortex-game/crates/*`, `cortex-game/.cargo/config.toml`, `cortex-game/rust-toolchain.toml`, `cortex-game/rustfmt.toml`, `cortex-game/clippy.toml`, `cortex-game/.gitignore`, root docs | Apply the bootstrap recipe verbatim: workspace + 22 crates (`cx-app`, `cx-sim-core`, `cx-terrain`, `cx-physics`, `cx-actor`, `cx-chassis`, `cx-equipment`, `cx-ai`, `cx-mission`, `cx-replay`, `cx-control`, `cxctl`, `cx-e2e`, `cx-save`, `cx-net`, `cx-render-2d`, `cx-ui`, `cx-audio`, `cx-mod`, `cx-tools-editor`, `cx-headless`, `cx-bench`); per-crate `AGENTS.md` per the template; pinned Bevy/glam/serde/clap/tokio/jsonrpsee/blake3/tracing/rand_xoshiro/schemars deps from the workspace dependencies table. | `cargo metadata`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. | Commit-ready diff + vault note `prototypes/native-m0-bootstrap.md`; commit only when the user explicitly asks. | No gameplay systems beyond no-op scene; no extra crates not in the recipe. |
| M0-002 Bevy app shell | `cx-app`, `cx-render-2d` | Open a window, clear screen, fixed title/version, ESC exits; support all M0 flags from the [CLI Reference](../spec/prototype-roadmap.md#cli-reference) (`--scenario`, `--seed`, `--run-seconds`, `--ticks`, `--write-run-bundle`, `--run-bundle-dir`, `--control-api`, `--control-port`, `--control-uds`, `--headless-smoke`, `--debug-capabilities`, `--ui-scale`, `--high-contrast`). | Native app smoke for 5 seconds; CLI flag parser tests; `--headless-smoke` exits cleanly. | Screenshot/capture of blank window if available; CLI help output captured to `notes.md`. | No menu system or final UI. |
| M0-003 fixed tick island | `cx-sim-core` | Implement fixed 60 Hz tick with optional 120 Hz; deterministic seed/RNG wrapper using `rand_xoshiro::Xoshiro256StarStar`; `Tick(u64)`, `WallClock`, `SimClock` types; tick counters; `pause`, `resume`, `step(n)`, `run_for(n)` API consumed by `cx-control`. | Unit tests for tick accumulation, seed repeatability (same seed → same checksum after 1000 ticks), pause/resume/step semantics; lints disallow `rand::thread_rng` and `SystemTime::now` per `clippy.toml`. | `system.run_started`, `system.tick_sample`, `system.run_finished` events emitted via `cx-replay`. | No full scheduler rewrite; no Bevy scheduling-stage redesign. |
| M0-004 run bundle writer | `cx-replay`, `tools/` | Write `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`; include build hash, config hash, scene id, schema version, capabilities, expected tests; directory naming per [Run-Bundle Naming Convention](../spec/prototype-roadmap.md#run-bundle-naming-convention). | Checker passes on M0 bundle; round-trip test for the envelope; non-blocking write under stress (events queued; dropped counter visible in `summary.json.event_counts.dropped_total`). | `prototype_runs/native/m0_<UTC-iso>_<short-hash>/`. | Do not design final replay UI. |
| M0-005 CI matrix | `.github/workflows/ci.yml`, `cortex-game/` | Apply the CI YAML from the bootstrap recipe; Win/Linux/macOS matrix; `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`, `cxctl observe smoke`, `cxctl run --write-run-bundle` smoke. | CI green when runners available; local commands pass regardless. | Link CI/log or local validation output in vault note. | No release packaging; no Steam Deck CI yet. |
| M0-006 control/observe bootstrap | `cx-control`, `cxctl`, `cx-app`, `cx-replay` | Define command/observation envelope per [Control Transport And Envelope](../spec/prototype-roadmap.md#control-transport-and-envelope) (JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890`, optional UDS, `schema_version` mandatory, blake3 short-hash run ids); generate JSON Schemas via `schemars` under `crates/cx-control/schemas/v1/`; implement `scenario.load`, `sim.pause/resume/step/run_for_ticks`, `observe.once`, `observe.subscribe/unsubscribe`, `observe.frame` notification, `act.player.*`, `runbundle.write`, `system.shutdown`; `cxctl` subcommands `observe --once|--stream`, `run --ticks --write-run-bundle`, `scenario load`, `pause`, `step`, `script run` per [CLI Reference](../spec/prototype-roadmap.md#cli-reference). | Unit tests for envelope (request/response/notification roundtrip); schema_version mismatch returns `-32602` with fix-hint; `cargo run -p cxctl -- observe --once`; no-op `run --ticks 300 --write-run-bundle` writes a valid bundle; loopback-only by default; heartbeat ping/pong. | Observation JSON sample and control-run bundle linked in M0 note. | No remote bot API; no gameplay debug cheats; no unauthenticated remote bind. |
| M0-007 m0_blank scenario fixture | `content/scenarios/m0_blank.ron` | Author the M0 scenario manifest per [Scenario Manifest Schema](../spec/prototype-roadmap.md#scenario-manifest-schema) minimal skeleton; loadable by `cx-app --scenario m0_blank`; validates with `cargo run -p cx-mod -- validate content/`. | Schema validation test; `--scenario m0_blank` smoke. | The RON file in `content/scenarios/`. | No teams, actors, terrain, or objectives. |
| M0-008 panic hook + tracing init | `cx-app`, `cxctl`, `cx-headless`, `cx-bench`, `cx-e2e`, `cx-mod` | Each binary's `main()` initializes `tracing-subscriber` with `EnvFilter` per [Logging, Tracing, And Error Policy](../spec/prototype-roadmap.md#logging-tracing-and-error-policy); installs a panic hook that emits `system.panic` event with backtrace before exit; severity counters are incremented in `summary.json.event_counts.by_severity`. | Binary boot test asserts the subscriber is registered; panic test triggers a controlled panic in a sub-thread and verifies the event is emitted; counter assertion. | One example structured log entry from each binary in the run notes. | No log-aggregation product. |

M0 reference E2E:

```bash
cargo run -p cx-app -- --scenario m0_blank --run-seconds 5 --write-run-bundle
cargo run -p cxctl -- observe --once
cargo run -p cxctl -- run --ticks 300 --write-run-bundle
python3 research_tools/prototype_run_check.py prototype_runs/native/<m0_run>
```

---

## M1 — Actor Controller And Sim Core

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M1-001 control intent | `cx-actor`, `cx-sim-core`, `cx-replay` | Input maps to `ControlIntent` before sim consequences: move, jump, aim, fire, reload, selected item. | Input serialization test; intent precedes action event. | Run bundle with `input_intent`. | No rollback/netcode. |
| M1-002 actor movement | `cx-actor`, `cx-physics` | Position/velocity, gravity, ground collision, jump/fall/recovery, reset. | Unit tests for gravity/ground/contact; E2E movement route. | 5-minute run with actor snapshots. | No complex ragdoll. |
| M1-003 rifle loop | `cx-equipment`, `cx-actor` | One rifle: fire interval, ammo, reload, recoil, muzzle origin, hit/miss event. | Ammo/reload/recoil tests; scripted fire/reload E2E. | `weapon_fired`, `weapon_reloaded`, `projectile_*` events. | No large arsenal. |
| M1-004 status strip | `cx-ui`, `cx-actor` | Minimal HUD: status, ammo, selected item, reticle state. | UI state source tests; screenshot artifact. | HUD screenshot in run bundle. | No final comic-noir UI. |
| M1-005 HTML lab supersession note | vault only | Record whether native M1 supersedes HTML lab for actor iteration; list gaps if not. | N/A. | `prototypes/native-m1-actor-controller.md`. | Do not delete HTML evidence. |
| M1-006 semantic actor control | `cx-control`, `cx-actor`, `cx-equipment`, `cx-e2e` | Drive movement, aim, fire, reload, selected item, and reset through the same `ControlIntent` path as human input; stream actor/equipment observations. | Scripted movement/fire/reload through `cxctl`; assert events and observations agree. | Control-script run bundle with no screenshot dependency. | No network prediction/rollback. |

M1 E2E:

```bash
cargo run -p cx-e2e -- --scenario m1_actor_range --script move_jump_fire_reload --write-run-bundle
cargo run -p cxctl -- script run m1_move_jump_fire_reload --write-run-bundle
```

Human gate: project-owner five-minute feel reaction. If unavailable, mark `READY_FOR_HUMAN_PLAYTEST`.

---

## M1.5 — Micro Breach Fun Slice

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M1.5-001 scenario shell | `cx-mission`, `cx-app`, `content/scenarios/` | 60-90s `micro_breach` scenario: spawn, objective, timer, extraction, win/loss. | Objective state tests; scripted win/loss. | `objective_*` events in two bundles. | No full mission director. |
| M1.5-002 reactive enemy | `cx-ai`, `cx-actor`, `cx-equipment` | One enemy: sight cone, aim delay, imperfect fire, reload, death; no omniscience. | Perception/aim/fire tests; E2E enemy kill + player death. | `ai_perception`, `tactic_chosen`, `weapon_fired`, `actor_status_changed`. | No full AI doctrine system. |
| M1.5-003 temporary soft breach | `cx-terrain` or `cx-mission` fixture | Minimal soft barrier/diggable tiles with success/refusal events; compatibility with future M2 event names. | Dig success/refusal tests. | `terrain_carved` or `terrain_breach_stub` with bbox/material. | No full chunked terrain. |
| M1.5-004 readable loop HUD | `cx-ui` | Objective/timer, player status, enemy status, selected item, last event. | Screenshot at 100% and 200% scale if UI scaling exists. | Capture artifact. | No full HUD art pass. |
| M1.5-005 fun/evidence note | vault only | Compare reaction against "ok I guess"; list whether pressure/goal changed feel. | N/A. | `prototypes/native-m1-5-micro-breach.md`. | Do not claim final fun. |
| M1.5-006 control-driven E2E | `cx-control`, `cx-e2e`, `cx-mission` | Write `cxctl` scripts for win path and loss path; assertions read objective/enemy/player state from observations and events. | `cargo run -p cxctl -- script run micro_breach_win` and `micro_breach_loss`; observation stream freshness check. | Two checked run bundles, action transcript, observation sample. | No brittle OS-level mouse/keyboard automation. |

M1.5 E2E:

```bash
cargo run -p cxctl -- script run micro_breach_win --expect objective.result=win --write-run-bundle
cargo run -p cxctl -- script run micro_breach_loss --expect objective.result=loss --write-run-bundle
cargo run -p cx-e2e -- --scenario micro_breach --script win_path --expect win --write-run-bundle
cargo run -p cx-e2e -- --scenario micro_breach --script lose_path --expect loss --write-run-bundle
```

Human gate: project-owner plays at least three runs and records verbatim reaction.

---

## M2 — Pixel Terrain And Materials

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M2-001 chunk storage | `cx-terrain` | 256x256 chunk grid, material id per pixel, sparse storage, CPU read/write. | Chunk bounds, material set/get, serialization tests. | Terrain snapshot in run bundle. | No Noita chemistry. |
| M2-002 material registry | `cx-terrain`, `content/materials/` | Air, dirt, concrete, metal-nohook, hazard, loose fill, repair-fill, anchor; hardness/affordance fields. | Material schema validation. | Material schema version in manifest. | No full research tree. |
| M2-003 carving pipeline | `cx-terrain`, `cx-render-2d`, `cx-equipment` | Digger and blast carve; CPU fallback; optional wgpu path behind feature flag. | Carve bbox/count tests; GPU/CPU parity if GPU path exists. | Dirty-region and perf counters. | No production destruction VFX. |
| M2-004 physics integration | `cx-physics`, `cx-terrain` | Actor collision respects terrain after edits; chunk boundary tests. | Collision after carve/fill tests. | E2E dig-through-wall. | No full pathfinding. |
| M2-005 material overlay | `cx-ui`, `cx-render-2d` | Toggle overlay shows material ids and tool validity. | Screenshot at 100/200% if applicable. | Overlay capture. | No tactical map. |
| M2-006 terrain replay | `cx-replay`, `cx-terrain` | Terrain snapshots/checksums and event replay reconstruct terrain. | Live vs replay checksum test. | Replay report. | No final cinematic replay. |

M2 E2E:

```bash
cargo run -p cx-e2e -- --scenario m2_material_lane --script dig_concrete_refuse_metal --expect win --write-run-bundle
```

---

## M3 — Replay And Event Recorder

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M3-001 event taxonomy | `cx-replay` | Stable event envelope, categories, parent ids, schema versions. | Schema/event ordering tests. | Updated run-bundle schema note if fields change. | No analytics service. |
| M3-002 snapshots/checksums | `cx-replay`, `cx-terrain`, `cx-actor`, `cx-equipment` | Actor/inventory/terrain snapshots and checksums. | Checksum repeatability tests. | `determinism.sim_checksum` events. | No full deterministic promise for cosmetics. |
| M3-003 headless replay | `cx-headless`, `cx-replay` | Replay M2/M1.5 bundles without rendering and verify checksums. | Replay compare test. | First-divergence report on failure. | No network server yet. |
| M3-004 viewer | `cx-ui`, `cx-replay` | Event tail, filters, parent-chain view, death/failure recap. | Viewer smoke test; screenshot. | Viewer capture in bundle. | No polished replay browser. |
| M3-005 recorder backpressure | `cx-replay` | Dropped-event counters and non-blocking recorder path. | Stress event-volume test. | Summary volume/perf rows. | No cloud telemetry. |

M3 E2E:

```bash
cargo run -p cx-headless -- replay prototype_runs/native/<m2_run> --verify-checksums
```

---

## M4 — HUD And Comic-Noir UI

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M4-001 HUD state model | `cx-ui`, `cx-actor`, `cx-equipment`, `cx-chassis` | Actor status, body silhouette placeholder, ammo, item, objective, last event. | UI state tests. | HUD screenshots. | No final art polish. |
| M4-002 comic-noir cards | `cx-ui`, `content/ui/` | Pre-mission and debrief card templates. | Layout snapshot tests if available. | 100/200% captures. | No full campaign UI. |
| M4-003 accessibility floor | `cx-ui`, `cx-app` | 200% scale, high contrast, keyboard/controller focus, captions hook, reduced shake/flash flags. | E2E accessibility smoke. | ACC-A status in notes. | No certification claim. |
| M4-004 material/tool feedback | `cx-ui`, `cx-terrain` | Tool validity labels and non-color-only material feedback. | Overlay screenshot tests. | Capture artifact. | No full tactical map. |

M4 E2E:

```bash
cargo run -p cx-e2e -- --scenario micro_breach --ui-scale 2.0 --high-contrast --verify-focus --write-run-bundle
```

Human gate: multi-person readability/accessibility playtests.

---

## M5 — Equipment, Chassis, And Damage Grammar

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M5-001 role records | `cx-equipment`, `content/equipment/` | Runtime role-record model from vault fixtures: role tags, bot policy, source/provenance fields. | Schema/fixture tests. | LOAD-A fixture import report. | No full economy/store. |
| M5-002 chassis model | `cx-chassis` | Armor zones, modules, pilot binding, powered armor and light mech. | State transition tests. | `chassis_stage_changed` events. | No full mech roster. |
| M5-003 damage/eject/repair/salvage | `cx-chassis`, `cx-equipment`, `cx-replay` | Module damage, jam, eject, repair, salvage events and HUD labels. | E2E wreck/eject/salvage. | Chassis run bundle. | No final gore/body system. |
| M5-004 save hooks | `cx-save`, `cx-chassis` | Serialize chassis/equipment state enough for roundtrip. | Save/load checksum. | Save artifact linked from run. | No full campaign save UI. |

M5 E2E:

```bash
cargo run -p cx-e2e -- --scenario m5_chassis_wreck_eject --expect pilot_extracted --write-run-bundle
```

---

## M6 — AI Core And Trust Harness

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M6-001 perception/memory | `cx-ai`, `cx-actor`, `cx-replay` | Sight, hearing, last-known memory, forgetting. | Perception unit tests; occlusion tests. | `ai_perception_signal` events. | No LLM runtime dependency. |
| M6-002 utility/doctrine | `cx-ai` | Utility scoring and 4-6 doctrine profiles. | Scoring tests with deterministic fixtures. | `ai_tactic_scored`, `tactic_chosen`. | No full strategic commander yet. |
| M6-003 mistakes/recovery | `cx-ai`, `cx-replay` | Panic/hesitate/miss/stuck/recover behavior with reason labels. | Recovery scenario tests. | `ai_recovery_action`. | No fake randomness without causes. |
| M6-004 AI-H harness | `cx-ai`, `cx-headless`, `tools/` | Runnable AI-H-01..06 suite with report output. | Harness pass/fail tests. | AI-H report bundle. | No broad campaign AI. |
| M6-005 bot overlay | `cx-ui`, `cx-ai` | Visible intent labels for friendly/enemy bots. | Screenshot capture. | Overlay screenshot. | No dialogue system. |

M6 E2E:

```bash
cargo run -p cx-ai --bin ai_harness -- --suite AI-H-01..AI-H-06 --write-run-bundle
```

---

## M7 — Mission Director And Breach Contract

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M7-001 manifest schema | `cx-mission`, `content/scenarios/` | Typed manifest: teams, objectives, materials, command core, base systems, loadout requirements, director. | Schema validation tests. | Manifest fixture in bundle. | No full campaign generator. |
| M7-002 director/commander | `cx-mission`, `cx-ai` | Pacing, reinforcement, LZ risk, commander reason labels. | Director phase tests. | `commander_decision.*`. | No MMO war layer. |
| M7-003 command core/base slice | `cx-mission`, `cx-chassis`, `cx-ui` | Rooted core powers shield/turret/door/repair; uproot/embed avatar tradeoff. | CORE-A subset tests. | `command_core_state_changed`, `base_power_changed`. | No full base builder. |
| M7-004 Breach Contract | `content/scenarios/`, `cx-app` | Playable mission: breach, fight, extract, win/loss/debrief. | E2E win/loss; replay. | MISSION-A run bundles. | No campaign map. |
| M7-005 debrief/retry | `cx-ui`, `cx-replay`, `cx-save` | Comic-noir debrief with cause chain and retry same seed. | UI/replay tests. | Debrief screenshot. | No full progression system. |

M7 E2E:

```bash
cargo run -p cx-e2e -- --scenario breach_contract --script win_path --expect win --write-run-bundle
cargo run -p cx-e2e -- --scenario breach_contract --script core_loss --expect loss --write-run-bundle
```

Human gate: project-owner plays five runs and records verbatim reaction.

---

## M8 — Scenario Editor And Mod Tools

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M8-001 editor workbench | `cx-tools-editor`, `cx-ui` | In-engine editor for spawns, materials, objectives, core/base state, loadout requirements. | Editor state tests; focus/accessibility smoke. | Editor screenshots. | No marketplace. |
| M8-002 package builder | `cx-mod`, `tools/` | Deterministic `.cxpkg`, manifest/provenance validation, dependency graph. | Package determinism tests. | PACK-A report. | No public hosting. |
| M8-003 script host | `cx-mod` | Implement chosen Lua/Rhai sandbox with capability declarations. | Sandbox denies FS/network by default. | Script-host test report. | No unbounded script API. |
| M8-004 sample mod | `mods/sample_*`, `content/` | New chassis + scenario + AI doctrine sample mod. | Validate/load/run sample mod. | Modded run bundle. | No full mod catalog. |

M8 E2E:

```bash
cargo run -p cx-mod -- validate content/ mods/ --strict
cargo run -p cx-e2e -- --scenario sample_mod_breach --expect win --write-run-bundle
```

---

## M9 — Headless Server And Determinism Islands

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M9-001 headless binary | `cx-headless`, `cx-app` | Run sim without renderer/audio; load scenario; accept scripted inputs. | Linux headless smoke. | Headless logs in bundle. | No public server browser. |
| M9-002 determinism contracts | `cx-sim-core`, `cx-replay`, docs | Document deterministic/stochastic/cosmetic subsystems. | Contract tests. | Determinism report. | No whole-engine determinism claim. |
| M9-003 replay-from-events | `cx-headless`, `cx-replay` | 10-minute M7 replay verifies actor/terrain/inventory checksums. | Replay compare. | First-divergence report if fail. | No network sync yet. |
| M9-004 headless perf | `cx-bench`, `cx-headless` | 10x real-time replay validation target. | Bench test. | Perf report. | No optimization-only rabbit hole. |

---

## M10 — LAN Co-op

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M10-001 authority model | `cx-net`, `cx-sim-core` | Server-authoritative input/snapshot/event model. | Unit tests for input validation. | Authority memo. | No anti-cheat product. |
| M10-002 LAN discovery/lobby | `cx-net`, `cx-ui` | Host/list/join on LAN; ready-up. | Local two-client smoke. | Lobby screenshot. | No NAT/relay. |
| M10-003 replication | `cx-net`, `cx-replay` | Actors, terrain, inventory, objective state replicate; per-client bundles align. | Replay compare across clients. | Two-client run bundles. | No public matchmaking. |

---

## M11 — Online Co-op Private

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M11-001 transport adapter | `cx-net` | NAT/relay candidate behind trait boundary. | Simulated latency tests. | Transport decision note. | No platform lock-in. |
| M11-002 package hash sync | `cx-net`, `cx-mod`, `cx-ui` | Join preflight checks content hashes and produces clean mismatch actions. | Mismatch tests. | Join-failure screenshots. | No public mod CDN. |
| M11-003 online session smoke | `cx-net`, `cx-app` | Two remote clients complete private co-op Breach Contract. | Remote run compare. | Per-client bundles. | No public launch promise. |

---

## M12 — PvP And MMO Experiments

| ID | Owns | Build | Tests | Evidence | Anti-scope |
|---|---|---|---|---|---|
| M12-001 PvP arena | `cx-net`, `cx-mission` | 2-4 players, small destructible map, server-authoritative validation. | Stress run. | Bandwidth/cheat notes. | No launch PvP promise. |
| M12-002 scale shard | `cx-headless`, `cx-net`, `cx-bench` | N=20 small-shard simulation for 10 minutes. | Load test. | Perf/desync report. | No MMO product commitment. |
| M12-003 DR-005 review | vault only | Revisit multiplayer posture with evidence. | N/A. | Updated DR or research log. | No silent scope expansion. |

---

## Side Track Injection Rules

These side tracks are not separate "later" workstreams. Every milestone final audit must say which side-track obligations were touched, skipped, or blocked.

| Track | Applies Starting | Agent Obligation | Evidence |
|---|---|---|---|
| T-PLATFORM | M0 | Keep current-platform commands passing; preserve Win/Linux/macOS portability in paths, file watching, case sensitivity, GPU backend assumptions, and input/audio setup. | Validation log; CI log when available. |
| T-CONTROL | M0 | Add semantic control/observation coverage for every new gameplay/UI action; prefer `cxctl` for E2E; record debug capabilities in the manifest. | `cxctl` command log, observation sample, and run-bundle events. |
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
## Final Audit
## Next Actions
```

The final audit must state every task card as `pass`, `blocked`, `human-gated`, or `deferred-with-reason`.
