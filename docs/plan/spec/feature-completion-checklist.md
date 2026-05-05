---
type: spec
status: live-checklist
authority: "Completion and rating checklist generated from prototype-roadmap.md and native-implementation-backlog.md. Update this after each implementation pass."
last_updated: 2026-05-05
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
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[dashboards/research-readiness|readiness]] · [VAULT_PLAN.md](../../VAULT_PLAN.md)

# Feature Completion Checklist

> [!summary] Purpose
> This is the living completion checklist for the native roadmap. It turns roadmap features, milestone scope, milestone done-criteria, side-track obligations, and native backlog task cards into rating rows. When an AI agent finishes a feature, task card, or milestone, it must update the relevant rows instead of only saying "done" in chat.

> [!important] Use this with the roadmap and backlog
> Build scope still comes from [[spec/prototype-roadmap]] and [[spec/native-implementation-backlog]]. This checklist tracks completion, evidence, human ratings, and AI self-ratings. If the roadmap/backlog changes, update this checklist in the same pass.

> [!info] Current coverage
> 497 baseline checklist rows plus the server/MMO addendum below. The old M9-M12 rows remain for continuity, but agents implementing M9-M12 must use the addendum plus [[spec/prototype-roadmap]], [[spec/native-implementation-backlog]], [[spec/server-app-architecture]], and [[spec/persistent-mmo-architecture]] as the authoritative scope until the next full regeneration.

> [!important] Server/MMO addendum active
> The 2026-05-05 server direction added DR-034, DR-035, T-SERVER, server/anti-cheat/MMO run-bundle categories, and expanded M9-M12. This file now includes a focused addendum so implementing agents have checklist rows immediately. The next full regeneration should merge these rows into the normal sections and remove the historical M9-M12 labels.

## Rating System

| Column | Who Fills It | Scale | Meaning |
|---|---|---|---|
| `Done` | Agent, only after validation | `[ ]` or `[x]` | Check only when the item is implemented, validated, and linked to evidence. Leave unchecked for partial work. |
| `Evidence` | Agent | link or path | Link the run bundle, prototype note, test log, screenshot, replay report, commit, or blocker note. |
| `H-Full` | Human owner | 1-10 | Human rating for how fully implemented the feature feels. `10` means complete enough to keep building on. |
| `H-Quality` | Human owner | 1-10 | Human rating for polish, feel, UX, correctness, readability, and maintainability. |
| `H-Review` | Human owner | 1-10 | Human rating for how much review/rework is still needed. `1` means no concern; `10` means urgent review. |
| `AI-Full` | Implementing/reviewing agent | 1-10 | AI self-rating for implementation completeness after validation. |
| `AI-Quality` | Implementing/reviewing agent | 1-10 | AI self-rating for quality after tests, E2E, bug hunt, and documentation. |
| `AI-Review` | Implementing/reviewing agent | 1-10 | AI self-rating for review risk. `1` means low risk; `10` means needs immediate review. |

## Update Rules For Agents

| Rule | Requirement |
|---|---|
| Read first | Before implementation, read [[spec/prototype-roadmap]], [[spec/native-implementation-backlog]], and this checklist. |
| Update exact rows | When finishing work, update every affected row: roadmap feature, milestone scope, milestone done-criterion, side-track obligation, and native task card. |
| Evidence required | Do not check a row without evidence. Evidence can be a run bundle, test log, replay report, screenshot, prototype note, commit hash, or explicit blocker note. |
| Human ratings | Do not invent human ratings. Leave `H-*` blank unless the user gives ratings. |
| AI ratings | Fill `AI-*` when you claim a row is done or substantially progressed. Be conservative; low full/quality or high review risk is useful. |
| Partial work | Leave `Done` unchecked, fill evidence and AI ratings, and explain remaining work in `Notes`. |
| Roadmap drift | If you add, split, rename, or delete roadmap/backlog features, update this file in the same commit/pass. |
| Milestone handoff | Final handoff must list checklist IDs changed and any rows left `READY_FOR_HUMAN`. |

## Table Of Contents

- [Milestone Scope Checklist](#milestone-scope-checklist)
- [Milestone Done-Criteria Checklist](#milestone-done-criteria-checklist)
- [Roadmap Feature Index Checklist](#roadmap-feature-index-checklist)
- [Side Track Checklist](#side-track-checklist)
- [Server/MMO Addendum Checklist](#servermmo-addendum-checklist)
- [Native Task Card Checklist](#native-task-card-checklist)
- [Global Validation And Bug Hunt Checklist](#global-validation-and-bug-hunt-checklist)

---

## Server/MMO Addendum Checklist

Use these rows for all M9-M12/T-SERVER work until the checklist is fully regenerated. Human ratings stay blank until the owner gives them.

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `TSERVER-P00` | T-SERVER side track: `cx-server` is the shared dedicated server artifact for LAN, co-op, PvP arena, MMO shard, and lobby directory modes. | [[spec/prototype-roadmap#T-SERVER — Dedicated Server App Lifecycle And Community Hosting]] | - | - | - | - | - | - | - | Use same sim path as client; no server-only game logic. |
| [ ] | `M9-SERVER-CORE` | M9 server-core subset passes: SERVER-001, SERVER-006, SERVER-009, SERVER-010, SERVER-011, SERVER-014, SERVER-015, SERVER-016. | [[spec/server-app-architecture#Acceptance Suite]] | - | - | - | - | - | - | - | M9 does not require SERVER-002/004/012 PvP/MMO scale tests. |
| [ ] | `M9-CXSERVER` | `cx-server` binary scaffold: RON config, `--mode`, `--validate-config-only`, no render/UI/audio crates. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Owns `cx-server`, `cx-server-ops`. |
| [ ] | `M9-OPS` | Health, readiness, metrics, JSON logs, drain shutdown, restart hooks. | [[spec/server-app-architecture]] | - | - | - | - | - | - | - | Emits `server.*` events. |
| [ ] | `M9-ANTI-CHEAT-FOUNDATION` | Anti-cheat profile registry, rate-limit hooks, replay drift skeleton, persisted ban list, audit log. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Foundation only; tournament-grade remains later. |
| [ ] | `M9-PERSISTENCE-FOUNDATION` | Snapshot writer, append-only event journal, restore loop, backups, schema migration hooks. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Full MMO persistence remains M12. |
| [ ] | `M9-DOCKER` | Reference Docker image runs `cx-server` unchanged and is documented. | [[spec/server-app-architecture#Acceptance Suite]] | - | - | - | - | - | - | - | Linux required; Windows hosting guide required separately. |
| [ ] | `M10-LAN-CXSERVER` | LAN co-op runs through `cx-server --mode lan_room`; ready-up, replicated state, per-client replay alignment. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Includes `anti_cheat.profile_applied` with `casual`. |
| [ ] | `M11-ONLINE-SELF-HOSTED` | A community member can host `cx-server --mode coop_room`; remote friends join through NAT/relay and complete a Breach Contract. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers)]] | - | - | - | - | - | - | - | Package hash mismatch must fail cleanly. |
| [ ] | `M11-LOBBY-DIRECTORY` | `lobby_directory` registration, heartbeat, browse/filter, deregister, and expiry work end-to-end. | [[decisions/dr-013-backend-service-scope]] | - | - | - | - | - | - | - | Required for public discovery; optional for private deployments. |
| [ ] | `M12-PVP-ARENA` | `cx-server --mode pvp_arena` runs a 4-8 player public arena with server-authoritative state and replay-aligned clients. | [[spec/prototype-roadmap#M12 — Public PvP Arenas + Persistent MMO Shards]] | - | - | - | - | - | - | - | Uses `competitive` default; `tournament_strict` opt-in only. |
| [ ] | `M12-MMO-SUITE` | MMO-001..MMO-012 all pass, including 50-client 1-hour soak, persistence restart, interest management, no-cloud reference. | [[spec/persistent-mmo-architecture#Acceptance Suite]] | - | - | - | - | - | - | - | M12 evidence gate; failure reopens DR-035. |
| [ ] | `M12-PVP-MMO-DR-REVIEW` | DR-005/013/034/035 reviewed with M9-M12 evidence; scope promoted, adjusted, or reopened explicitly. | [[spec/native-implementation-backlog#M12 — Public PvP Arenas + Persistent MMO Shards]] | - | - | - | - | - | - | - | No silent demotion or silent scope expansion. |

---

## Milestone Scope Checklist

These rows come from the `Scope` lists under each roadmap milestone. They are broad implementation surfaces; the native task-card rows later in this file break them down further.

### M0 - Engine Bootstrap

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M0-P00` | Milestone proof: The native repo exists, builds on three platforms, runs a Bevy app with a fixed-tick sim plugin, ticks at 60 Hz, exits cleanly, produces a deterministic run bundle from a scripted no-op scene. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M0-S01` | Cargo workspace with the crate layout above. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S02` | `cx-app` binary that launches a Bevy app with empty schedule. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S03` | `cx-sim-core` fixed-tick scheduler (60 Hz default; 120 Hz option). | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S04` | `cx-replay` minimal event envelope + run-bundle writer (no events yet beyond `system_*`). | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S05` | `cx-render-2d` minimal wgpu pipeline that clears the screen. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S06` | `cx-control` minimal command/observation schema plus `cargo run -p cxctl -- observe --once`, `cargo run -p cxctl -- run --ticks`, `pause`, and `step`. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S07` | GitHub Actions CI: build matrix Win/Linux/macOS; cargo check + cargo test + cargo clippy. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S08` | Native run bundles compatible with `research_tools/prototype_run_check.py`; add a thin native helper or wrapper only if the milestone needs one. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-S09` | Hello-world scene: blank window, press ESC to exit, run-bundle written to `prototype_runs/native/`. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |

### M1 - Actor Controller And Sim Core

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M1-P00` | Milestone proof: One actor is playable on the native engine. Movement, aim, simple weapon, and the body-status state machine all run through the fixed-tick sim and emit replay events. This is the moment the **HTML lab is officially superseded as the iteration harness**. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M1-S01` | `cx-actor` actor components: `Position`, `Velocity`, `Aim`, `Status` (STABLE/UNSTABLE/DOWNED/DEAD), `Inventory`. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S02` | `cx-sim-core` control intent layer: input → `ControlIntent` resource → consumed by sim systems. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S03` | `cx-physics` minimal 2D physics: gravity, ground collision, recoil impulse. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S04` | `cx-equipment` minimal: one rifle preset; magazine/ammo state; fire/reload events. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S05` | `cx-render-2d`: pixel-art sprite rendering (sub-pixel-clean); chunky pixel actor sprite. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S06` | `cx-replay`: event taxonomy expanded to `input_intent`, `actor_status_changed`, `weapon_fired`, `weapon_reloaded`, `actor_snapshot`. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S07` | `cx-control`: movement, aim, fire, reload, selected-item, actor snapshot, and equipment observations/actions. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S08` | HUD stub via egui: ammo + status text overlay. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-S09` | Manual playtest: WASD movement, mouse aim, click-to-fire, R to reload. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |

### M1.5 - Micro Breach Fun Slice

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M1.5-P00` | Milestone proof: The native actor lab has something to do. This milestone directly answers the HTML playtest signal: "ok I guess... hard to tell." It adds the cheapest possible pressure, goal, enemy, and terrain consequence before the full terrain/material milestone. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M1.5-S01` | One 60-90 second playable micro scenario: start → breach a soft barrier → fight or bypass one reactive enemy → reach extraction. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-S02` | One reactive enemy dummy: limited sight cone, slow aim, imperfect fire, health/status, death event, and no omniscience. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-S03` | One soft breach surface: a tiny temporary destructible strip or tile field. It may be replaced by M2's real chunked terrain; it must still emit terrain-like events. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-S04` | One digger/tool action with visible refusal/success labels. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-S05` | One objective state machine: `objective_started`, `objective_updated`, `objective_completed`, `objective_failed`. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-S06` | HUD additions: objective text, timer, player status, enemy status, selected item, last important event. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-S07` | Run bundle captures input, enemy perception, enemy fire, hit/miss, player damage/death, tool use, terrain breach, objective result, and screenshot. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-S08` | `cargo run -p cxctl -- script ...` scripts drive both win and loss paths without requiring manual input. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |

### M2 - Pixel Terrain And Materials

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M2-P00` | Milestone proof: Mutable chunked pixel terrain. The player can dig a soft-material wall and the change is visible, replay-recorded, and respected by the simple physics. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M2-S01` | `cx-terrain` chunked pixel terrain: 256×256 chunks; per-pixel material id; sparse storage. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S02` | GPU-assisted carving compute shader (wgpu): blast/dig writes apply on the GPU when bounds are large; CPU fallback for small writes. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S03` | Material registry with launch material set: air, dirt, concrete, metal-nohook, hazard, loose fill, repair-fill, anchor. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S04` | Material affordances: hardness, anchorability, hazard flags, path-cost contribution. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S05` | Dirty-region tracker for downstream consumers (path, replay, render). | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S06` | Digger tool wired into `cx-equipment`; `tool_action_started` / `terrain_carved` / `tool_refused` events. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S07` | Material overlay (toggle key): renders material id as colored overlay. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S08` | Visual feedback: pixel debris particles when carving. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |

### M3 - Replay And Event Recorder

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M3-P00` | Milestone proof: Event taxonomy is complete enough that any prior milestone's run can be replayed headlessly and produce identical state checksums. Determinism islands are real. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M3-S01` | `cx-replay` event taxonomy expanded to cover all event categories from [[systems/replay-event-architecture]]: combat, body, terrain, AI, logistics, mission, modifier, network. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S02` | Snapshot writer: full actor/inventory/terrain snapshot at scene start + every objective change. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S03` | Checksum producer: per-tick or per-snapshot. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S04` | Headless replay binary: replays a run bundle without rendering and produces matching checksums. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S05` | Run-bundle viewer: simple egui-based event tail + filter + parent-chain view. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S06` | Determinism island contract: documents which subsystems are deterministic (sim core, terrain mutation, AI decisions) and which are not (audio, particles cosmetic, render). | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |

### M4 - HUD And Comic-Noir UI

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M4-P00` | Milestone proof: Game state is readable from the HUD without text walls. Comic-noir mission card style is established. Accessibility floor (DR-012) is hit. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M4-S01` | `cx-ui` HUD: body silhouette (DR-003 style); module strip stub; ammo + reload; objective banner; timer; last-important-event ticker. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S02` | Comic-noir mission card: pre-mission briefing card; post-mission debrief card; both static. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S03` | Status banners ("ARMOR CRACKED LEFT", "JET FAILED", "EJECT NOW") triggered by chassis events. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S04` | Material overlay UI integrated; tool-validity color cues. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S05` | Accessibility floor: 200% text scale + reflow; high-contrast mode; color-independent state labels; controller route through HUD; remap holds; captions. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S06` | SDF/vector text rendering for clean scaling. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |

### M5 - Equipment, Chassis, And Damage Grammar

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5-P00` | Milestone proof: The chassis grammar from DR-014/021 works on the native engine. One powered-armor actor and one light mech actor exercise the full ladder of layers + modules + damage stages + jam + eject + repair + salvage. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M5-S01` | `cx-chassis` chassis components: layered armor zones, modules with state, pilot/operator binding. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S02` | Damage stages: `nominal` → `degraded` → `module-warning` → `module-failed` → `weapon-jammed` → `armor-cracked` → `disabled` → `pilot-injured` → `eject` → `bail-too-late` → `wreck` → `gibbed/exploded`. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S03` | Module system: jet, shield, sensor, repair-drone, weapon-mount; each with damage states. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S04` | `cx-equipment` role records implementation; LOAD-A fixture support; AI policy hints. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S05` | Events: `chassis_stage_changed`, `module_state_changed`, `armor_layer_damaged`, `weapon_jammed`, `weapon_cleared`, `pilot_state_changed`, `pilot_ejected`, `pilot_extracted`, `pilot_lost`, `chassis_repaired`, `chassis_salvaged`. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S06` | Two reference chassis: powered armor (Spartan-ish proportions); light mech (~3× human). | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S07` | Tutorial-safety scenario policy honored: lethal demoted to KO during onboarding-shaped scenarios. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |

### M5.5 - Full Collision Gauntlet

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5.5-P00` | Milestone proof: The game has the physical consequence contract required by DR-033. Bodies, limbs, weapons, armor, mechs, projectiles, objects, terrain, shields, and base parts collide through explicit data and replay-visible events, without brute-force all-pairs. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M5.5-S01` | `cx-physics` collision pipeline: broadphase, narrowphase, contact manifold, stable pair ids, collision matrix loader, deterministic pair ordering, and contact-event emission. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S02` | Collision classes and proxies for actor core, limbs, armor zones, held weapons, loose items, kinetic projectiles, explosive projectiles, terrain proxies, debris chunks, mech parts, base objects, force fields, and sensor triggers. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S03` | Explicit collision matrix: player/player, unit/unit, AI/AI, enemy/enemy, ally/ally, limb/limb, limb/body, limb/weapon, weapon/weapon, projectile/body, projectile/terrain, projectile/equipment, projectile/shield, projectile/projectile, debris/body, mech/infantry, base/object interactions. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S04` | CCD tiers: discrete, speculative, sweep ray, sweep capsule, sweep shape, and TOI substep. Fast projectiles, important limbs, command-core bodies, and mech crush contacts cannot tunnel through thin terrain or units. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S05` | Projectile-projectile contact: kinetic bullet-bullet deflects/fragments/tumbles/loses energy; explosive projectile contacts can detonate, fuze-fail, or deflect by authored profile. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S06` | Impulse-to-damage routing: collision impulse, contact area, sharpness, material pair, armor layer, and origin/chassis rules produce body, armor, equipment, terrain, module, and base-object damage. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S07` | Terrain chunk collision proxies update from M2 dirty regions; chunk seams/tiny holes/edge cases are test fixtures. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S08` | `cx-replay`: `collision` event category with contact start/persist/end, impulse, projectile deflection, projectile-projectile contact, filter reason, collision damage, budget degradation, and first divergence events. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S09` | `cxctl observe --collisions` and `cxctl inspect collision <event-id>` for implementation agents and future bot authors. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S10` | Perf budget governor for low-value debris; never silently drops actor, limb, armor, weapon, key projectile, terrain, shield, command-core, or mission-critical contacts. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |

### M6 - AI Core And Trust Harness

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6-P00` | Milestone proof: The 8-criteria humanlike AI bar from DR-022 has a runnable harness. Perception, memory, doctrine, reason labels, recovery, and replay are all in place. Strategic adaptation across missions is staged but not yet required to fire. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M6-S01` | `cx-ai` perception model: sight cone + hearing range + memory grid for last-known positions. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S02` | Utility scoring + doctrine slots: cautious, aggressive, support, scout, sniper, etc. (start with 4-6). | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S03` | Reason-label events: `tactic_chosen` with reason string for every decision. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S04` | Mistake/recovery model: bots can panic, miss, get stuck; recovery actions emit events. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S05` | AI-H scenario runner: AI-H-01..AI-H-06 from [[spec/ai-trust-harness-slice-a]]. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S06` | Reason-label HUD overlay: shows what each visible bot is currently trying to do. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S07` | Cross-mission state stub: faction commander persists across the same campaign session (file-based). | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |

### M6.5 - LLM Mind Lab

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6.5-P00` | Milestone proof: An async LLM "mind" layer can run alongside local AI without blocking it. Strict-schema proposals (doctrine patches, squad orders, dialogue, memory writes) flow through a validator and policy compiler. A deterministic mock provider drives CI; cloud/local providers (OpenAI, Anthropic, Ollama, OpenAI-compatible) sit behind feature gates. Local AI keeps acting through provider sleep, failure, malformed/stale responses, and cost-cap exhaustion. **No API key is required to ship, test, or play.** | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M6.5-S01` | `cx-ai::mind::schema`: `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, `MindProviderConfig`. JSON Schemas under `cortex-game/crates/cx-ai/schemas/mind/v1/`. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S02` | `cx-ai::mind::provider`: shared trait + adapters (`mock` always built; `openai`/`anthropic`/`ollama`/`openai-compatible` behind cargo features `mind-openai`, `mind-anthropic`, `mind-ollama`, `mind-openai-compatible`). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S03` | `cx-ai::mind::compressor`: derives `MindObservationFrame` from the `cx-control` observation stream + replay events with fog-of-war filtering. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S04` | `cx-ai::mind::validator`: rejects stale, invalid, impossible, unfair, over-budget, hidden-info, capability-violating proposals. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S05` | `cx-ai::mind::policy`: applies accepted proposals as utility-weight patches, commander-blackboard goals, doctrine tags, dialogue queue entries, and `MindMemoryRecord` writes. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S06` | `cx-replay`: new `mind` event category (see [[references/prototype-run-bundle-schema]]) with `mind.task_created`, `mind.prompt_recorded`, `mind.response_received`, `mind.proposal_validated`, `mind.patch_applied`, `mind.patch_rejected`, `mind.memory_written`. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S07` | `cxctl observe --mind-frame <scope>`: emit a compact mind frame for `actor`/`squad`/`faction`/`mission_director`/`post_mission` scopes (no screenshots). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S08` | `content/scenarios/micro_breach_mind_lab.ron`: the M6.5 scenario in three modes (`mind_off`, `mind_mock`, `mind_live_optional`). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S09` | `cx-tools-editor`: dev-only mind dashboard (task count, stale rate, provider failures, estimated cost, model routing, accept/reject reasons). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |

### M7 - Mission Director And Breach Contract Proof Mission

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M7-P00` | Milestone proof: Everything above composes into one playable Breach Contract mission. Manifest format works. Command core works minimally. Base systems work minimally. Mission director paces the encounter. The first proof mission can be played, won, lost, replayed, debriefed. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M7-S01` | `cx-mission` typed scenario manifest schema (data-only): objectives, teams, terrain rules, command-core/base state, capability requirements, director phases, save fields, replay events, validation. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S02` | Mission director: manages pacing, reinforcement, LZ risk, objective escalation, with reason labels. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S03` | Command-core mechanic minimum: rooted core powers ≥ 2 base systems (shield + 1 turret). Uprooted core embeds into player avatar with stat boost. Losing core = mission failure if `command_core_endgame` policy. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S04` | Base system slice: command core + power grid + 1 shield + 1 turret + 1 door + 1 repair pad. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S05` | Breach Contract scenario: enter compound → breach wall → neutralize 2-3 enemies → reach extract → before timer. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S06` | Comic-noir pre-/post-mission cards. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S07` | Death recap from replay. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |

### M8 - Scenario Editor And Mod Tools

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M8-P00` | Milestone proof: Players can author scenarios using the same manifest format the engine ships with. Mod loader works. Package builder produces deterministic packages. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M8-S01` | `cx-tools-editor` in-engine workbench mode: scenario editor (place spawns, materials, objectives, command-core, base systems, capability requirements, director config); test-run; export. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S02` | `cx-mod` mod loader: discovers packages in `mods/`; validates schemas; loads at engine startup. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S03` | Package builder: produces deterministic `.cxpkg` archives; provenance tracking; loader graph; preset/effect graphs; migration preview. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S04` | Lua or Rhai scripting host for mod scripts (decision in M5; implement in M8). | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S05` | Scenario validator: catches missing fields, broken refs, AI policy violations, accessibility issues. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S06` | One sample mod: adds a new chassis archetype using the same grammar. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |

### M9 - Headless Server And Determinism Islands

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M9-P00` | Milestone proof: The sim runs without rendering on a Linux headless target. Deterministic islands are real and testable. Replays from events alone reconstruct identical state. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M9-S01` | `cx-headless` headless binary: same sim, no renderer, no audio, network-driven inputs. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-S02` | Determinism island contracts documented and validated: which subsystems are bit-deterministic; which are stochastic-but-replayable; which are cosmetic only. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-S03` | Headless replay-from-events: given a run bundle, the headless server replays and produces identical checksums. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-S04` | Performance pass: headless can run 10× real-time on baseline hardware for replay validation. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - |  |

### M10 - LAN Co-op

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M10-P00` | Milestone proof: Two clients on a local network can play one Breach Contract together with replicated state, authority resolution, and replay parity. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M10-S01` | `cx-net` authority model: server-authoritative for sim; clients send inputs, receive snapshots + events. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S02` | LAN discovery (no NAT yet). | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S03` | Lobby + ready-up. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S04` | Replicated state: actors, terrain, inventory, mission state. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S05` | Co-op friendly fire policy (configurable per scenario). | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S06` | Per-client replay bundles that align. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |

### M11 - Online Co-op (Private)

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M11-P00` | Milestone proof: Online co-op works through NAT/relay between two friends. Package hash sync prevents version mismatch crashes. | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M11-S01` | NAT punch-through or relay (transport library decision). | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-S02` | Lobby with code-based join. | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-S03` | Package hash sync: server checks client packages match; soft-fail with auto-download for the dev workflow; hard-fail with mismatch report for shipping. | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-S04` | Latency compensation: client-side prediction + server reconciliation for player actor; pure replication for AI bots. | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - |  |

### M12 - PvP And MMO Experiments

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M12-P00` | Milestone proof: The architecture can support PvP and large-scale online without re-architecting. Or it tells us where the wall is. | [[spec/prototype-roadmap#M12 — PvP And MMO Experiments]] | - | - | - | - | - | - | - | Milestone-level proof row. |

---

## Milestone Done-Criteria Checklist

These rows come from the roadmap milestone `Done-criteria` lists. A milestone is not complete until every agent-completable criterion is checked or explicitly marked `READY_FOR_HUMAN` with evidence.

### M0 - Engine Bootstrap

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M0-D01` | `cargo build --release` succeeds on Win/Linux/macOS. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-D02` | CI is green for all three platforms when runners are available; local current-platform validation passes before handoff. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-D03` | `cargo run` opens a window, ticks the sim at 60 Hz for 5 seconds, exits cleanly. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-D04` | A run bundle is written under `prototype_runs/native/m0_*/` with manifest+events+summary+notes. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-D05` | `python3 research_tools/prototype_run_check.py prototype_runs/native/<m0_run>` passes on the bundle. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-D06` | `cargo run -p cxctl -- observe --once` reads current run/tick/scenario state without screenshot capture. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-D07` | `cargo run -p cxctl -- run --ticks 300 --write-run-bundle` drives the no-op scene without OS input. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |
| [ ] | `M0-D08` | Repository is commit-ready, with a semantic commit only if the user explicitly asked the agent to commit. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - |  |

### M1 - Actor Controller And Sim Core

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M1-D01` | One actor is playable for 5 minutes without crash. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-D02` | All control inputs produce `input_intent` events. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-D03` | The actor can be moved, aimed, fired, and reloaded through `cxctl` or the control API with the same sim path as human input. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-D04` | Status transitions emit `actor_status_changed` with cause. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-D05` | A 5-minute run bundle validates with the run-bundle checker. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-D06` | Project owner does a manual playtest and writes a verbatim reaction in a vault note. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |
| [ ] | `M1-D07` | HTML lab is marked superseded; new prototype work goes into native. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - |  |

### M1.5 - Micro Breach Fun Slice

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M1.5-D01` | The micro scenario can be won and lost in 60-90 seconds. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-D02` | Enemy behavior is reactive but simple; it emits perception/fire/reload/death events with reason labels. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-D03` | The soft breach emits terrain-compatible events that M2 can replace without changing replay consumers. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-D04` | A scripted E2E run wins the scenario; another scripted or deterministic run loses it. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-D05` | Both E2E runs use the semantic control layer and assert objective outcome from structured observations/events. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-D06` | Run bundle validates and includes screenshot/capture plus objective outcome. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |
| [ ] | `M1.5-D07` | Project owner can play the scenario and record a verbatim reaction. If unavailable, mark `READY_FOR_HUMAN_PLAYTEST`. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - |  |

### M2 - Pixel Terrain And Materials

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M2-D01` | Player can dig through dirt fast, concrete slowly, metal-nohook is refused with reason label. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D02` | Carving emits `terrain_carved` events with bbox + material id + count. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D03` | Dirty regions update; render reflects mutation within one frame. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D04` | Material overlay reads correctly across all 8 launch materials. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D05` | Run bundle validates; replay can reconstruct the terrain state at any tick. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D06` | Perf budget: 1280×720 scene + carving session sustains 120 FPS on baseline hardware (per T-PERF). | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |

### M3 - Replay And Event Recorder

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M3-D01` | A 5-minute M2 run can be replayed headlessly and produces identical actor/terrain/inventory checksums. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D02` | Drift between replay and live run is reported per-tick with diff. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D03` | Replay viewer can scrub through events and show context. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D04` | Death recap: given an `actor_died` event, the viewer shows the parent cause chain. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D05` | Run bundle includes manifest, events, summary, snapshots, checksums, captures. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |

### M4 - HUD And Comic-Noir UI

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M4-D01` | HUD-01..HUD-03 acceptance tests from [[systems/ux-overlay-screen-brief]] pass with 5 playtesters. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-D02` | ACC-A floor passes for HUD + mission card + material overlay. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-D03` | Mission card renders pre/post mission with comic-noir style. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-D04` | 200% text scale doesn't break HUD layout. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |

### M5 - Equipment, Chassis, And Damage Grammar

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5-D01` | Player can take damage and progress through stages with HUD + replay parity. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D02` | Module damage produces module-warning → failure with reason labels. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D03` | Pilot eject works: player ejects from a wrecked mech and continues as foot infantry. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D04` | Chassis salvage emits `chassis_salvaged` with recoverable modules. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D05` | BODY-A and CHASSIS-A acceptance tests pass. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |

### M5.5 - Full Collision Gauntlet

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5.5-D01` | COLL-001 collision matrix generator fails on any physical pair with no rule. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D02` | COLL-002 player/ally/enemy/AI unit-unit body collisions block, shove, knock down, and recover with events. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D03` | COLL-003 limb-to-limb, limb-to-body, limb-to-terrain, and limb-to-door contacts work; detached limbs collide normally. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D04` | COLL-004 held weapons collide with limbs, terrain, doors, and other held weapons; owner self-filter is reason-labeled. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D05` | COLL-005 bullets hit bodies, armor, weapons, dropped items, terrain, shields, and mech modules with distinct events. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D06` | COLL-006 bullet-bullet/projectile-projectile contacts produce deflection/fragment/fuze/detonation outcomes per projectile profile. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D07` | COLL-007 high-speed projectiles and falling bodies do not tunnel through tiny holes, chunk boundaries, shields, or thin limbs. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D08` | COLL-008 physics impacts damage limbs, armor, equipment, chassis modules, debris, terrain, base objects, and mechs where thresholds are met. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D09` | COLL-009 Full Collision Gauntlet replays headlessly with identical contact ids/checksums. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D10` | COLL-010 `cxctl observe --collisions` exposes live contacts, filters, and last 30 collision events without screenshots. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D11` | COLL-011 perf report records 1080p/60 pass plus 4K/120 and Steam Deck status. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D12` | COLL-012 AI pathing/behavior reacts to body blocking, debris, doors, shields, and contact damage with reason labels. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |

### M6 - AI Core And Trust Harness

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6-D01` | AI-H-01..AI-H-06 pass with replay evidence. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-D02` | All 8 DR-022 criteria are testable; at least 6 are demonstrably met (intent, perception, doctrine, mistakes, recovery, replay proof; strategic adaptation + fairness staged). | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-D03` | A friendly bot in a 60-90s scene actively communicates intent through reason labels. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |

### M6.5 - LLM Mind Lab

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6.5-D01` | MIND-001 — `ai_mind.enabled=false` baseline plays the scenario; AI-H tests pass. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D02` | MIND-002 — Provider sleeps 30 s; actors keep fighting/retreating/reloading/rescuing; scenario completes locally. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D03` | MIND-003 — Malformed JSON is rejected; replay records rejection; game continues. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D04` | MIND-004 — Response arriving after `valid_until_tick` is rejected or downgraded to post-hoc memory. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D05` | MIND-005 — Accepted proposal patches utility weights and produces visible reason labels. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D06` | MIND-006 — Mind prompt excludes hidden enemy state unless explicit debug capability. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D07` | MIND-007 — Post-encounter memory writes are visible in run bundle and feed later prompt context. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D08` | MIND-008 — Replay viewer shows mind task, prompt hash, provider class, proposal summary, validator result, applied patch ids. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D09` | MIND-009 — Provider tasks halt at `max_run_cost_usd`; local AI continues. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D10` | MIND-010 — AI-H report compares local-only vs mind-enabled runs across all 8 DR-022 criteria. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D11` | CI uses mock provider only; live cloud calls are never required for any test. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |

### M7 - Mission Director And Breach Contract Proof Mission

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M7-D01` | Mission can be won and lost via the listed paths. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D02` | Replay reconstructs the mission tick-perfect. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D03` | Command-core uproot works: player embeds the core into a chassis and gains the avatar boost; rooted base systems shed. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D04` | MISSION-A acceptance tests pass. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D05` | Project owner plays the mission at least 5 times and writes a verbatim reaction. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D06` | At this point, the **A-FEEL gate from the prior HTML playtest is met** — the lab has something to do, not just operate. | [[spec/prototype-roadmap#M7 — Mission Director And Breach Contract Proof Mission]] | - | - | - | - | - | - | - |  |

### M8 - Scenario Editor And Mod Tools

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M8-D01` | A player can author a Breach Contract variant in the in-engine editor. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-D02` | The variant exports as a `.cxpkg`, loads back into the engine, runs. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-D03` | Sample mod's new chassis works in M7 mission. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-D04` | PACK-A and MOD-A acceptance tests pass. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |

### M9 - Headless Server And Determinism Islands

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M9-D01` | A 10-minute M7 mission run replays headlessly with bit-identical actor/terrain/inventory checksums. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-D02` | Headless server runs on a Linux VPS without graphics drivers. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-D03` | DET-A acceptance tests pass. | [[spec/prototype-roadmap#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - |  |

### M10 - LAN Co-op

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M10-D01` | Two clients survive one 5-minute Breach Contract together with no desync. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-D02` | Both clients' replay bundles align tick-for-tick. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-D03` | Bandwidth budget within target (TBD per T-PERF). | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |

### M11 - Online Co-op (Private)

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M11-D01` | Two friends in different cities co-op a Breach Contract. | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-D02` | Latency masking works at 50-150ms RTT without obvious jitter. | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-D03` | Package mismatch produces a clean error, not a crash. | [[spec/prototype-roadmap#M11 — Online Co-op (Private)]] | - | - | - | - | - | - | - |  |

### M12 - PvP And MMO Experiments

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M12-D01` | PvP is stable enough to run public stress tests. | [[spec/prototype-roadmap#M12 — PvP And MMO Experiments]] | - | - | - | - | - | - | - |  |
| [ ] | `M12-D02` | MMO prototype runs N=20 players for 10 minutes without desync. | [[spec/prototype-roadmap#M12 — PvP And MMO Experiments]] | - | - | - | - | - | - | - |  |
| [ ] | `M12-D03` | DR-005 launch posture is reconsidered with prototype evidence. | [[spec/prototype-roadmap#M12 — PvP And MMO Experiments]] | - | - | - | - | - | - | - |  |

---

## Roadmap Feature Index Checklist

These rows come from the roadmap `Feature Index`. They are the fastest way to see whether a named game/system feature has been built at least once.

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `F001` | Cargo workspace + crate layout | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 |
| [ ] | `F002` | Bevy app shell | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 |
| [ ] | `F003` | Custom wgpu render pipelines | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 (clear), M1 (sprite), M2 (terrain), M5 (chassis), M7 (full) |
| [ ] | `F004` | Fixed-tick sim scheduler | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 |
| [ ] | `F005` | Run-bundle writer / checker | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0, M3 |
| [ ] | `F006` | AI/dev control API schemas | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M0 |
| [ ] | `F007` | `cxctl` CLI observe/run/step/act/assert | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M0..M1.5 |
| [ ] | `F008` | Semantic UI tree and UI action control | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M4, M8 |
| [ ] | `F009` | Future bot authoring API | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M6, M8 |
| [ ] | `F010` | Actor controller + control intent | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1 |
| [ ] | `F011` | 2D physics baseline | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1 |
| [ ] | `F012` | T-PHYS full collision contract | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PHYS, M1..M12 |
| [ ] | `F013` | Micro Breach fun loop | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1.5 |
| [ ] | `F014` | Reactive enemy dummy | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1.5 |
| [ ] | `F015` | Temporary soft breach surface | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1.5, replaced by M2 terrain |
| [ ] | `F016` | Objective timer/win/loss state | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1.5, M7 |
| [ ] | `F017` | Pixel terrain (chunked) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2 |
| [ ] | `F018` | Material system + affordances | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2 |
| [ ] | `F019` | GPU-assisted terrain carving | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2 |
| [ ] | `F020` | Material overlay UI | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2, M4 |
| [ ] | `F021` | Event taxonomy (full) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3 |
| [ ] | `F022` | Snapshots + checksums | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3 |
| [ ] | `F023` | Headless replay | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3, M9 |
| [ ] | `F024` | Replay viewer | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3 |
| [ ] | `F025` | HUD body silhouette | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4 |
| [ ] | `F026` | Comic-noir mission cards | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4 |
| [ ] | `F027` | SDF/vector text | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4 |
| [ ] | `F028` | Accessibility floor | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4, T-ACCESSIBILITY |
| [ ] | `F029` | Equipment role records | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F030` | Chassis layers + modules | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F031` | Damage stages | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F032` | Pilot eject / repair / salvage | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F033` | Collision class/proxy registry | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5, M5.5 |
| [ ] | `F034` | Full collision matrix | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F035` | Limb/body/equipment/mech/base collision | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F036` | Projectile-projectile collision | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F037` | CCD tiers / TOI contact proof | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F038` | Collision impulse-to-damage routing | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F039` | `collision` event category in run bundles | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3, M5.5 |
| [ ] | `F040` | `cxctl observe --collisions` | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F041` | COLL-001..COLL-012 acceptance suite | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F042` | Tutorial-safety policy | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5, M7 |
| [ ] | `F043` | AI perception + memory | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F044` | AI utility + doctrine | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F045` | AI reason labels | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F046` | AI-H scenario runner | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F047` | Cross-mission commander state | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6, M7 |
| [ ] | `F048` | Async LLM mind layer (T-LLM) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5, T-LLM |
| [ ] | `F049` | `MindObservationFrame` + `MindTask` + `AiMindProposal` schemas | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F050` | Provider adapters (mock + OpenAI + Anthropic + Ollama + OpenAI-compatible) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F051` | Mock LLM provider for CI | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F052` | Observation compressor (fog-of-war filter) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F053` | Proposal validator + policy compiler | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F054` | `mind` event category in run bundles | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3, M6.5 |
| [ ] | `F055` | `cxctl observe --mind-frame` | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F056` | LLM mind dashboard (dev/debug) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5, M8 |
| [ ] | `F057` | MIND-001..MIND-010 acceptance suite | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F058` | LLM-driven debrief / commander adaptation | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 (optional augmentation), M9 |
| [ ] | `F059` | LLM-authored mod profiles (workbench) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F060` | Mission manifest schema | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F061` | Mission director | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F062` | Command-core mechanic | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F063` | Base systems (shield + turret + door + repair pad) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F064` | Breach Contract proof mission | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F065` | Comic-noir debrief | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4, M7 |
| [ ] | `F066` | In-engine scenario editor | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F067` | Mod loader + package builder | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F068` | Lua/Rhai script host | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F069` | Headless dedicated server | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M9 |
| [ ] | `F070` | Determinism island contracts | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M9 |
| [ ] | `F071` | LAN co-op | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M10 |
| [ ] | `F072` | Online co-op (NAT) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M11 |
| [ ] | `F073` | Package hash sync | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M11 |
| [ ] | `F074` | PvP prototype | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M12 |
| [ ] | `F075` | MMO experiment | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M12 |
| [ ] | `F076` | Diegetic audio + captions | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-AUDIO, M4..M7 |
| [ ] | `F077` | Save game system | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-SAVE, M5..M9 |
| [ ] | `F078` | CI matrix Win/Linux/macOS | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PLATFORM, M0..M12 |
| [ ] | `F079` | Steam Deck compatibility | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PLATFORM |
| [ ] | `F080` | 4K/120 + 1080p/60 + Deck/800p/60 perf | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PERF |

---

## Side Track Checklist

These rows come from roadmap side-track details. Side tracks are cross-cutting obligations, so agents should update these rows whenever a milestone touches the track.

### T-LLM - Async LLM Mind Layer

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-LLM-A01` | Default mode: `mock` (deterministic). No API key required. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A02` | Schemas: `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, `MindProviderConfig` per [[spec/hybrid-llm-ai-plan]]. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A03` | Provider portfolio: OpenAI Responses API + Structured Outputs; Anthropic Messages API; Ollama; OpenAI-compatible (vLLM, llama.cpp); deterministic mock. All behind one trait; cloud adapters cargo-feature-gated. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A04` | Latency contract: Local AI never waits. Every task has a deadline; stale responses are rejected or downgraded to memory. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A05` | Determinism: CI uses mock only. Replay reuses recorded proposals. Live cloud calls never required for any test. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A06` | Fairness: Observation compressor enforces fog-of-war. MIND-006 audits that prompts exclude hidden enemy state unless explicit debug capability. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A07` | Captioning: Every generated dialogue line emits a caption per T-AUDIO + T-ACCESSIBILITY. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A08` | Localization: English-first at v1 (matches Anti-Goals); language is a `MindProviderConfig.language` field for future packs. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A09` | Replay/audit: New `mind` event category in run bundles per [[references/prototype-run-bundle-schema]]; secrets redacted. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A10` | Player default: Disabled. Opt-in via settings; mock-first; cloud/local providers each require explicit configuration. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A11` | Multiplayer: Server-authoritative LLM cognition; clients see resulting orders/events, never privileged prompts. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A12` | Modding: LLM-authored profile/doctrine packs are mod data, validated by the standard package builder. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A13` | Cost budget: `max_run_cost_usd` hard cap per `MindProviderConfig`. CI: $0; dev iteration: $0.10; M6.5 lab: $0.25; player default: off; opt-in power-user: $0.50. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-D14` | Done criteria: every milestone that touches AI/UI/captions extends the mind layer with the relevant observation/proposal/event shape; CI never depends on live providers; the run-bundle audit shows every mind task with its provider class, prompt hash, response hash, validator result, and accepted patch ids. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |

### T-CONTROL - AI Control And Observability

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-CONTROL-R01` | `cx-control` owns versioned command, observation, UI-tree, and assertion schemas. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R02` | `cxctl` is the CLI interface for scripts: load scenario, pause, step, observe, act, click UI by id, assert objective state, and write run bundles. During development, run it as `cargo run -p cxctl -- ...`; `cxctl ...` is shorthand after the binary is installed or added to PATH. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R03` | A local-only control server, launched with `--control-api`, streams observations and accepts semantic action commands. Initial target is JSON-RPC/WebSocket or an equally scriptable transport. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R04` | Observation packets include tick, scenario, actors, equipment, terrain/material affordances, objectives, UI semantic tree, captions/audio cues, recent events, and performance counters. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R05` | Action packets map to real human/gameplay/UI affordances: move, aim, fire, reload, use, select unit, issue order, query/click/type UI, run/step/reset scenario, inspect entity/event chain. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R06` | Debug-only actions are capability-gated, disabled by default, and recorded in the run manifest. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-D07` | Done criteria: every new player-facing control or UI action is either controllable through `cxctl`/the control API or explicitly marked human-only with a reason; every new critical screen state has a structured observation/event/caption equivalent. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |

### T-PHYS - Full Collision And Physical Consequence

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-PHYS-A01` | Default rule: Physical objects collide by default. Missing matrix entries are build/test failures. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A02` | Performance rule: No naive all-pairs. Use broadphase, spatial hash/dynamic tree, chunk proxies, CCD tiers, stable pair ordering, and low-value debris budgets. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A03` | Projectile rule: Projectiles collide with units, limbs, armor, equipment, terrain, shields, base objects, and selected projectile classes. Kinetic bullet-bullet contacts deflect/fragment/lose energy unless authored otherwise. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A04` | Damage rule: Contact impulse can damage limbs, armor, weapons, equipment, mech modules, terrain, shields, and base objects. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A05` | Terrain rule: Pixels/materials stay authoritative; collision uses chunk proxies rebuilt from dirty regions plus exact material samples at contact. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A06` | Event rule: Meaningful contacts emit `collision.*` events and parent-link to combat/body/terrain/equipment damage. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A07` | Control rule: `cxctl observe --collisions` exposes live pair state, filters, recent contacts, and collision budget status. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A08` | AI rule: From M6 onward, AI perceives collision-affordance changes and emits reason labels when blocked, shoved, pinned, avoiding debris, or reacting to projectile danger. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-D09` | Done criteria: each milestone final audit says which new physical classes, pairs, filters, events, and perf counters were added. A gameplay object cannot become physical in art/combat without being registered in the T-PHYS matrix or explicitly declared cosmetic/sensor-only. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |

### T-PLATFORM - Cross-Platform CI And Steam Deck

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-PLATFORM-R01` | GitHub Actions matrix: Win (windows-latest), Linux (ubuntu-latest), macOS (macos-latest). | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-R02` | `cargo build --release`, `cargo test`, `cargo clippy -- -D warnings` on each. | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-R03` | Steam Deck testing pass at every milestone end (manual; document in vault). | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-R04` | Platform-specific issues (input mapping, audio backend, file paths) tracked per milestone. | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-D05` | Done criteria: CI green; Steam Deck plays the milestone's reference scene at 800p/60. | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |

### T-MOD - Modding And Scripting

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-MOD-R01` | Schema-first data: every mod-extensible system has a documented schema. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R02` | Scripting host: mlua or Rhai (decided during M5; implemented in M6 or M7). | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R03` | Sandbox: scripts cannot do filesystem/network without capability declaration. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R04` | Documentation: auto-generated API reference from Rust trait impls. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R05` | Sample mods: 3-5 sample mods covering chassis, weapons, scenarios, AI doctrines, materials. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-D06` | Done criteria: A modder authors a chassis + scenario + AI doctrine in under one weekend; package validates and runs. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |

### T-AUDIO - Diegetic SFX And Captions

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-AUDIO-R01` | Diegetic-first mix per DR-020. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R02` | Audio cue → caption event pipeline: every critical SFX has a caption. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R03` | Origin-specific failure sound families per [[spec/audio-identity]]. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R04` | Mix policy: synth music ducks under critical alarms. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R05` | Captioned playback in replay viewer. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-D06` | Done criteria: All M4..M7 SFX have captions; mix passes 5 deaf-accessibility playtest sessions. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |

### T-SAVE - Save Game System

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-SAVE-R01` | `cx-save` versioned save format (`.cxsave`). | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R02` | Saves include: command core state, base modules, actors/veterans, mechs, salvage, faction state, enemy commander memory, mission manifests, scenario policy. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R03` | Multiple save slots per profile. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R04` | Autosave before/after contracts. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R05` | Mission suspend/resume. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R06` | Same-seed retry. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R07` | Ironman / scenario policies persisted. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R08` | Replay archive linked to saves. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R09` | Migration-safe schema with version handlers. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-D10` | Done criteria: Save → load → continue mission produces identical state. Migration test: a v0.1 save loads on v0.2 with declared migration handlers. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |

### T-ACCESSIBILITY - Accessibility Floor

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-ACCESSIBILITY-R01` | Per DR-012 and [[spec/accessibility-comfort-slice-a]]: | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R02` | 200% text scale + reflow. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R03` | High contrast mode. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R04` | Color-independent state labels. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R05` | Controller / keyboard / mouse parity. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R06` | Remap holds. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R07` | Captions for all critical audio. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R08` | Reduced motion / shake / flash. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R09` | ACC-A acceptance tests at every milestone end. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-D10` | Done criteria: Every milestone's user-facing surface passes ACC-A floor. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |

### T-PERF - Performance Targets And Budgets

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-PERF-D01` | Done criteria: Reference scene meets the three targets. | [[spec/prototype-roadmap#T-PERF — Performance Targets And Budgets]] | - | - | - | - | - | - | - |  |

---

## Native Task Card Checklist

These rows come from [[spec/native-implementation-backlog]]. They are the concrete implementation units agents should be assigned. A task card row should not be checked until its tests, evidence, and anti-scope obligations are satisfied.

### M0 - Engine Bootstrap

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M0-001` | workspace scaffold. Build: Apply the bootstrap recipe verbatim: workspace + 22 crates (`cx-app`, `cx-sim-core`, `cx-terrain`, `cx-physics`, `cx-actor`, `cx-chassis`, `cx-equipment`, `cx-ai`, `cx-mission`, `cx-replay`, `cx-control`, `cxctl`, `cx-e2e`, `cx-save`, `cx-net`, `cx-render-2d`, `cx-ui`, `cx-audio`, `cx-mod`, `cx-tools-editor`, `cx-headless`, `cx-bench`); per-crate `AGENTS.md` per the template; pinned Bevy/glam/serde/clap/tokio/jsonrpsee/blake3/tracing/rand_xoshiro/schemars deps from the workspace dependencies table. Tests: `cargo metadata`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. Anti-scope: No gameplay systems beyond no-op scene; no extra crates not in the recipe. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `cortex-game/Cargo.toml`, `cortex-game/crates/*`, `cortex-game/.cargo/config.toml`, `cortex-game/rust-toolchain.toml`, `cortex-game/rustfmt.toml`, `cortex-game/clippy.toml`, `cortex-game/.gitignore`, root docs. Evidence target: Commit-ready diff + vault note `prototypes/native-m0-bootstrap.md`; commit only when the user explicitly asks. |
| [ ] | `M0-002` | Bevy app shell. Build: Open a window, clear screen, fixed title/version, ESC exits; support all M0 flags from the [CLI Reference](../spec/prototype-roadmap.md#cli-reference) (`--scenario`, `--seed`, `--run-seconds`, `--ticks`, `--write-run-bundle`, `--run-bundle-dir`, `--control-api`, `--control-port`, `--control-uds`, `--headless-smoke`, `--debug-capabilities`, `--ui-scale`, `--high-contrast`). Tests: Native app smoke for 5 seconds; CLI flag parser tests; `--headless-smoke` exits cleanly. Anti-scope: No menu system or final UI. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `cx-app`, `cx-render-2d`. Evidence target: Screenshot/capture of blank window if available; CLI help output captured to `notes.md`. |
| [ ] | `M0-003` | fixed tick island. Build: Implement fixed 60 Hz tick with optional 120 Hz; deterministic seed/RNG wrapper using `rand_xoshiro::Xoshiro256StarStar`; `Tick(u64)`, `WallClock`, `SimClock` types; tick counters; `pause`, `resume`, `step(n)`, `run_for(n)` API consumed by `cx-control`. Tests: Unit tests for tick accumulation, seed repeatability (same seed → same checksum after 1000 ticks), pause/resume/step semantics; lints disallow `rand::thread_rng` and `SystemTime::now` per `clippy.toml`. Anti-scope: No full scheduler rewrite; no Bevy scheduling-stage redesign. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `cx-sim-core`. Evidence target: `system.run_started`, `system.tick_sample`, `system.run_finished` events emitted via `cx-replay`. |
| [ ] | `M0-004` | run bundle writer. Build: Write `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`; include build hash, config hash, scene id, schema version, capabilities, expected tests; directory naming per [Run-Bundle Naming Convention](../spec/prototype-roadmap.md#run-bundle-naming-convention). Tests: Checker passes on M0 bundle; round-trip test for the envelope; non-blocking write under stress (events queued; dropped counter visible in `summary.json.event_counts.dropped_total`). Anti-scope: Do not design final replay UI. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `cx-replay`, `tools/`. Evidence target: `prototype_runs/native/m0_<UTC-iso>_<short-hash>/`. |
| [ ] | `M0-005` | CI matrix. Build: Apply the CI YAML from the bootstrap recipe; Win/Linux/macOS matrix; `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`, `cxctl observe smoke`, `cxctl run --write-run-bundle` smoke. Tests: CI green when runners available; local commands pass regardless. Anti-scope: No release packaging; no Steam Deck CI yet. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `.github/workflows/ci.yml`, `cortex-game/`. Evidence target: Link CI/log or local validation output in vault note. |
| [ ] | `M0-006` | control/observe bootstrap. Build: Define command/observation envelope per [Control Transport And Envelope](../spec/prototype-roadmap.md#control-transport-and-envelope) (JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890`, optional UDS, `schema_version` mandatory, blake3 short-hash run ids); generate JSON Schemas via `schemars` under `crates/cx-control/schemas/v1/`; implement `scenario.load`, `sim.pause/resume/step/run_for_ticks`, `observe.once`, `observe.subscribe/unsubscribe`, `observe.frame` notification, `act.player.*`, `runbundle.write`, `system.shutdown`; `cxctl` subcommands `observe --once\|--stream`, `run --ticks --write-run-bundle`, `scenario load`, `pause`, `step`, `script run` per [CLI Reference](../spec/prototype-roadmap.md#cli-reference). Tests: Unit tests for envelope (request/response/notification roundtrip); schema_version mismatch returns `-32602` with fix-hint; `cargo run -p cxctl -- observe --once`; no-op `run --ticks 300 --write-run-bundle` writes a valid bundle; loopback-only by default; heartbeat ping/pong. Anti-scope: No remote bot API; no gameplay debug cheats; no unauthenticated remote bind. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `cx-control`, `cxctl`, `cx-app`, `cx-replay`. Evidence target: Observation JSON sample and control-run bundle linked in M0 note. |
| [ ] | `M0-007` | m0_blank scenario fixture. Build: Author the M0 scenario manifest per [Scenario Manifest Schema](../spec/prototype-roadmap.md#scenario-manifest-schema) minimal skeleton; loadable by `cx-app --scenario m0_blank`; validates with `cargo run -p cx-mod -- validate content/`. Tests: Schema validation test; `--scenario m0_blank` smoke. Anti-scope: No teams, actors, terrain, or objectives. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `content/scenarios/m0_blank.ron`. Evidence target: The RON file in `content/scenarios/`. |
| [ ] | `M0-008` | panic hook + tracing init. Build: Each binary's `main()` initializes `tracing-subscriber` with `EnvFilter` per [Logging, Tracing, And Error Policy](../spec/prototype-roadmap.md#logging-tracing-and-error-policy); installs a panic hook that emits `system.panic` event with backtrace before exit; severity counters are incremented in `summary.json.event_counts.by_severity`. Tests: Binary boot test asserts the subscriber is registered; panic test triggers a controlled panic in a sub-thread and verifies the event is emitted; counter assertion. Anti-scope: No log-aggregation product. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | - | - | - | - | - | - | - | Owns: `cx-app`, `cxctl`, `cx-headless`, `cx-bench`, `cx-e2e`, `cx-mod`. Evidence target: One example structured log entry from each binary in the run notes. |
### M1 - Actor Controller And Sim Core

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M1-001` | control intent. Build: Input maps to `ControlIntent` before sim consequences: move, jump, aim, fire, reload, selected item. Tests: Input serialization test; intent precedes action event. Anti-scope: No rollback/netcode. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - | Owns: `cx-actor`, `cx-sim-core`, `cx-replay`. Evidence target: Run bundle with `input_intent`. |
| [ ] | `M1-002` | actor movement. Build: Position/velocity, gravity, ground collision, jump/fall/recovery, reset. Tests: Unit tests for gravity/ground/contact; E2E movement route. Anti-scope: No complex ragdoll. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - | Owns: `cx-actor`, `cx-physics`. Evidence target: 5-minute run with actor snapshots. |
| [ ] | `M1-003` | rifle loop. Build: One rifle: fire interval, ammo, reload, recoil, muzzle origin, hit/miss event. Tests: Ammo/reload/recoil tests; scripted fire/reload E2E. Anti-scope: No large arsenal. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - | Owns: `cx-equipment`, `cx-actor`. Evidence target: `weapon_fired`, `weapon_reloaded`, `projectile_*` events. |
| [ ] | `M1-004` | status strip. Build: Minimal HUD: status, ammo, selected item, reticle state. Tests: UI state source tests; screenshot artifact. Anti-scope: No final comic-noir UI. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-actor`. Evidence target: HUD screenshot in run bundle. |
| [ ] | `M1-005` | HTML lab supersession note. Build: Record whether native M1 supersedes HTML lab for actor iteration; list gaps if not. Tests: N/A. Anti-scope: Do not delete HTML evidence. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - | Owns: vault only. Evidence target: `prototypes/native-m1-actor-controller.md`. |
| [ ] | `M1-006` | semantic actor control. Build: Drive movement, aim, fire, reload, selected item, and reset through the same `ControlIntent` path as human input; stream actor/equipment observations. Tests: Scripted movement/fire/reload through `cxctl`; assert events and observations agree. Anti-scope: No network prediction/rollback. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | - | - | - | - | - | - | - | Owns: `cx-control`, `cx-actor`, `cx-equipment`, `cx-e2e`. Evidence target: Control-script run bundle with no screenshot dependency. |
### M1.5 - Micro Breach Fun Slice

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M1.5-001` | scenario shell. Build: 60-90s `micro_breach` scenario: spawn, objective, timer, extraction, win/loss. Tests: Objective state tests; scripted win/loss. Anti-scope: No full mission director. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - | Owns: `cx-mission`, `cx-app`, `content/scenarios/`. Evidence target: `objective_*` events in two bundles. |
| [ ] | `M1.5-002` | reactive enemy. Build: One enemy: sight cone, aim delay, imperfect fire, reload, death; no omniscience. Tests: Perception/aim/fire tests; E2E enemy kill + player death. Anti-scope: No full AI doctrine system. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - | Owns: `cx-ai`, `cx-actor`, `cx-equipment`. Evidence target: `ai_perception`, `tactic_chosen`, `weapon_fired`, `actor_status_changed`. |
| [ ] | `M1.5-003` | temporary soft breach. Build: Minimal soft barrier/diggable tiles with success/refusal events; compatibility with future M2 event names. Tests: Dig success/refusal tests. Anti-scope: No full chunked terrain. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - | Owns: `cx-terrain` or `cx-mission` fixture. Evidence target: `terrain_carved` or `terrain_breach_stub` with bbox/material. |
| [ ] | `M1.5-004` | readable loop HUD. Build: Objective/timer, player status, enemy status, selected item, last event. Tests: Screenshot at 100% and 200% scale if UI scaling exists. Anti-scope: No full HUD art pass. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - | Owns: `cx-ui`. Evidence target: Capture artifact. |
| [ ] | `M1.5-005` | fun/evidence note. Build: Compare reaction against "ok I guess"; list whether pressure/goal changed feel. Tests: N/A. Anti-scope: Do not claim final fun. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - | Owns: vault only. Evidence target: `prototypes/native-m1-5-micro-breach.md`. |
| [ ] | `M1.5-006` | control-driven E2E. Build: Write `cxctl` scripts for win path and loss path; assertions read objective/enemy/player state from observations and events. Tests: `cargo run -p cxctl -- script run micro_breach_win` and `micro_breach_loss`; observation stream freshness check. Anti-scope: No brittle OS-level mouse/keyboard automation. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | - | - | - | - | - | - | - | Owns: `cx-control`, `cx-e2e`, `cx-mission`. Evidence target: Two checked run bundles, action transcript, observation sample. |
### M2 - Pixel Terrain And Materials

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M2-001` | chunk storage. Build: 256x256 chunk grid, material id per pixel, sparse storage, CPU read/write. Tests: Chunk bounds, material set/get, serialization tests. Anti-scope: No Noita chemistry. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cx-terrain`. Evidence target: Terrain snapshot in run bundle. |
| [ ] | `M2-002` | material registry. Build: Air, dirt, concrete, metal-nohook, hazard, loose fill, repair-fill, anchor; hardness/affordance fields. Tests: Material schema validation. Anti-scope: No full research tree. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cx-terrain`, `content/materials/`. Evidence target: Material schema version in manifest. |
| [ ] | `M2-003` | carving pipeline. Build: Digger and blast carve; CPU fallback; optional wgpu path behind feature flag. Tests: Carve bbox/count tests; GPU/CPU parity if GPU path exists. Anti-scope: No production destruction VFX. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cx-terrain`, `cx-render-2d`, `cx-equipment`. Evidence target: Dirty-region and perf counters. |
| [ ] | `M2-004` | physics integration. Build: Actor collision respects terrain after edits; chunk boundary tests. Tests: Collision after carve/fill tests. Anti-scope: No full pathfinding. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cx-physics`, `cx-terrain`. Evidence target: E2E dig-through-wall. |
| [ ] | `M2-005` | material overlay. Build: Toggle overlay shows material ids and tool validity. Tests: Screenshot at 100/200% if applicable. Anti-scope: No tactical map. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-render-2d`. Evidence target: Overlay capture. |
| [ ] | `M2-006` | terrain replay. Build: Terrain snapshots/checksums and event replay reconstruct terrain. Tests: Live vs replay checksum test. Anti-scope: No final cinematic replay. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cx-replay`, `cx-terrain`. Evidence target: Replay report. |
### M3 - Replay And Event Recorder

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M3-001` | event taxonomy. Build: Stable event envelope, categories, parent ids, schema versions. Tests: Schema/event ordering tests. Anti-scope: No analytics service. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cx-replay`. Evidence target: Updated run-bundle schema note if fields change. |
| [ ] | `M3-002` | snapshots/checksums. Build: Actor/inventory/terrain snapshots and checksums. Tests: Checksum repeatability tests. Anti-scope: No full deterministic promise for cosmetics. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cx-replay`, `cx-terrain`, `cx-actor`, `cx-equipment`. Evidence target: `determinism.sim_checksum` events. |
| [ ] | `M3-003` | headless replay. Build: Replay M2/M1.5 bundles without rendering and verify checksums. Tests: Replay compare test. Anti-scope: No network server yet. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cx-headless`, `cx-replay`. Evidence target: First-divergence report on failure. |
| [ ] | `M3-004` | viewer. Build: Event tail, filters, parent-chain view, death/failure recap. Tests: Viewer smoke test; screenshot. Anti-scope: No polished replay browser. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-replay`. Evidence target: Viewer capture in bundle. |
| [ ] | `M3-005` | recorder backpressure. Build: Dropped-event counters and non-blocking recorder path. Tests: Stress event-volume test. Anti-scope: No cloud telemetry. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cx-replay`. Evidence target: Summary volume/perf rows. |
### M4 - HUD And Comic-Noir UI

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M4-001` | HUD state model. Build: Actor status, body silhouette placeholder, ammo, item, objective, last event. Tests: UI state tests. Anti-scope: No final art polish. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-actor`, `cx-equipment`, `cx-chassis`. Evidence target: HUD screenshots. |
| [ ] | `M4-002` | comic-noir cards. Build: Pre-mission and debrief card templates. Tests: Layout snapshot tests if available. Anti-scope: No full campaign UI. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `content/ui/`. Evidence target: 100/200% captures. |
| [ ] | `M4-003` | accessibility floor. Build: 200% scale, high contrast, keyboard/controller focus, captions hook, reduced shake/flash flags. Tests: E2E accessibility smoke. Anti-scope: No certification claim. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-app`. Evidence target: ACC-A status in notes. |
| [ ] | `M4-004` | material/tool feedback. Build: Tool validity labels and non-color-only material feedback. Tests: Overlay screenshot tests. Anti-scope: No full tactical map. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-terrain`. Evidence target: Capture artifact. |
### M5 - Equipment, Chassis, And Damage Grammar

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5-001` | role records. Build: Runtime role-record model from vault fixtures: role tags, bot policy, source/provenance fields. Tests: Schema/fixture tests. Anti-scope: No full economy/store. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cx-equipment`, `content/equipment/`. Evidence target: LOAD-A fixture import report. |
| [ ] | `M5-002` | chassis model. Build: Armor zones, modules, pilot binding, powered armor and light mech. Tests: State transition tests. Anti-scope: No full mech roster. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cx-chassis`. Evidence target: `chassis_stage_changed` events. |
| [ ] | `M5-003` | damage/eject/repair/salvage. Build: Module damage, jam, eject, repair, salvage events and HUD labels. Tests: E2E wreck/eject/salvage. Anti-scope: No final gore/body system. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cx-chassis`, `cx-equipment`, `cx-replay`. Evidence target: Chassis run bundle. |
| [ ] | `M5-004` | save hooks. Build: Serialize chassis/equipment state enough for roundtrip. Tests: Save/load checksum. Anti-scope: No full campaign save UI. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cx-save`, `cx-chassis`. Evidence target: Save artifact linked from run. |
### M5.5 - Full Collision Gauntlet

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5.5-001` | collision class registry. Build: Define physical classes from [[spec/full-collision-physics-plan]]: actor core, limb, armor zone, held weapon, loose item, kinetic projectile, explosive projectile, terrain proxy, debris chunk, mech part, base object, force field, sensor trigger, cosmetic particle. Tests: Registry roundtrip; missing-class validation; class-id stability test. Anti-scope: No gameplay behavior yet. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-physics`, `cx-mod`, `content/collision/`. Evidence target: `collision_class_registered` or schema audit in run note. |
| [ ] | `M5.5-002` | collision matrix + filters. Build: Data-driven matrix with collide/sensor/filter/damage response; every filter requires `collision_filter_reason`. Tests: COLL-001; bad matrix fixtures fail with useful diagnostics. Anti-scope: No silent ignore pairs. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-physics`, `cx-mod`. Evidence target: Matrix file and validator report. |
| [ ] | `M5.5-003` | broadphase and pair cache. Build: Dynamic tree/spatial hash hybrid; stable pair ids; deterministic pair ordering; projectile lane cache. Tests: Pair-count tests; deterministic ordering; stress bench. Anti-scope: No O(n^2) production path. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-physics`, `cx-bench`. Evidence target: Perf counters: candidate pairs, narrowphase pairs, culled low-value pairs. |
| [ ] | `M5.5-004` | narrowphase/contact manifolds. Build: Contact manifolds for circle/capsule/convex/AABB/segment/terrain-proxy pairs; material pair lookup. Tests: Shape-pair unit tests; edge/tiny-hole fixtures. Anti-scope: No exact per-pixel rigid body solver. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-physics`. Evidence target: `collision_contact_started/persisted/ended`. |
| [ ] | `M5.5-005` | CCD tiers. Build: Discrete, speculative, sweep ray, sweep capsule, sweep shape, TOI substep; per-class `ccd_class`. Tests: COLL-007; tunneling fixtures for thin terrain, limb, shield, bullet, mech foot. Anti-scope: No universal TOI for all debris. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-physics`, `cx-bench`. Evidence target: `toi_fraction` and CCD-tier fields in events. |
| [ ] | `M5.5-006` | projectile-projectile contacts. Build: Swept projectile lane test; kinetic deflect/fragment/tumble/energy loss; explosive detonate/fuze-fail/deflect by profile. Tests: COLL-006; deterministic bullet-cross fixtures. Anti-scope: No fake random explosions for kinetic rounds. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-physics`, `cx-equipment`, `content/projectiles/`. Evidence target: `projectile_projectile_contact`, `projectile_deflected`, optional `projectile_fragmented`. |
| [ ] | `M5.5-007` | limb/equipment/body contacts. Build: Limb-to-limb/body/weapon/terrain/door contacts; held weapon physical contacts; scoped owner self-filter; dropped item contacts. Tests: COLL-002..COLL-004; crowd corridor fixture. Anti-scope: No animation-only collision. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-actor`, `cx-chassis`, `cx-equipment`, `cx-physics`. Evidence target: Contact events plus body/equipment/chassis follow-up events. |
| [ ] | `M5.5-008` | impulse-to-damage routing. Build: Convert contact impulse/material/area/sharpness into limb wounds, armor crack/spall, equipment jam/damage, chassis module failure, terrain/base damage. Tests: COLL-005, COLL-008; threshold tests by material/origin. Anti-scope: No hidden HP-only damage. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-physics`, `cx-actor`, `cx-chassis`, `cx-equipment`, `cx-terrain`. Evidence target: `contact_impulse_applied`, `collision_damage_applied`, parent-linked body/equipment/terrain events. |
| [ ] | `M5.5-009` | terrain/base/shield proxies. Build: Dirty chunk collision proxy rebuilds; doors/turrets/sensors/shields/repair pads register physical or sensor proxies. Tests: Chunk seam tests; shield/body/projectile/base fixtures. Anti-scope: No full base builder here. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-terrain`, `cx-mission`, `cx-physics`. Evidence target: Terrain dirty-to-proxy events; base object contact events. |
| [ ] | `M5.5-010` | `cxctl` collision observation. Build: `cxctl observe --collisions` and `cxctl inspect collision <event-id>` show live pairs, filters, last contacts, TOI, impulses, and budget status. Tests: CLI snapshot tests; stream freshness tests. Anti-scope: No screenshot-only physics debugging. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-control`, `cxctl`, `cx-physics`. Evidence target: Observation samples in run notes. |
| [ ] | `M5.5-011` | full gauntlet scenario. Build: Scenario scripts for COLL-001..COLL-012: crowd corridor, bullet cross, limb/weapon/door, debris crush, mech foot, shield, terrain seams. Tests: Full E2E suite. Anti-scope: No hand-tested-only acceptance. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `content/scenarios/m5_5_full_collision_gauntlet.ron`, `cx-e2e`. Evidence target: Checked run bundle with event counts by collision type. |
| [ ] | `M5.5-012` | replay/perf/bug hunt. Build: Headless replay checksum; perf report; first-divergence event; bug-hunt log. Tests: Replay verify; 1080p/60 pass; 4K/120 + Deck status recorded. Anti-scope: No "works once" completion. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cx-headless`, `cx-bench`, `tools/`, vault. Evidence target: Prototype note under `prototypes/` with final audit and known issues. |
### M6 - AI Core And Trust Harness

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6-001` | perception/memory. Build: Sight, hearing, last-known memory, forgetting. Tests: Perception unit tests; occlusion tests. Anti-scope: No LLM runtime dependency. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cx-ai`, `cx-actor`, `cx-replay`. Evidence target: `ai_perception_signal` events. |
| [ ] | `M6-002` | utility/doctrine. Build: Utility scoring and 4-6 doctrine profiles. Tests: Scoring tests with deterministic fixtures. Anti-scope: No full strategic commander yet. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cx-ai`. Evidence target: `ai_tactic_scored`, `tactic_chosen`. |
| [ ] | `M6-003` | mistakes/recovery. Build: Panic/hesitate/miss/stuck/recover behavior with reason labels. Tests: Recovery scenario tests. Anti-scope: No fake randomness without causes. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cx-ai`, `cx-replay`. Evidence target: `ai_recovery_action`. |
| [ ] | `M6-004` | AI-H harness. Build: Runnable AI-H-01..06 suite with report output. Tests: Harness pass/fail tests. Anti-scope: No broad campaign AI. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cx-ai`, `cx-headless`, `tools/`. Evidence target: AI-H report bundle. |
| [ ] | `M6-005` | bot overlay. Build: Visible intent labels for friendly/enemy bots. Tests: Screenshot capture. Anti-scope: No dialogue system. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-ai`. Evidence target: Overlay screenshot. |
| [ ] | `M6-006` | mind hooks (T-LLM bridge). Build: Expose hook points that the future M6.5 mind layer will call: utility-weight patch API, commander-blackboard goal API, doctrine-tag set API, dialogue-queue API, memory-write API. M6 itself MUST NOT call any LLM. Tests: Hook tests with synthetic patches; AI-H stays green when no hooks are called. Anti-scope: No LLM runtime dependency in M6. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cx-ai`. Evidence target: Hook trait docs in `cx-ai::doctrine`; example synthetic patch in tests. |
### M6.5 - LLM Mind Lab

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6.5-001` | mind schemas. Build: Define `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, `MindProviderConfig` per [[spec/hybrid-llm-ai-plan]]; emit JSON Schemas via `schemars`. Tests: Roundtrip tests; bad-example rejection tests; schema-version mismatch test. Anti-scope: No public schema export yet. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-ai::mind::schema`, `cortex-game/crates/cx-ai/schemas/mind/v1/`. Evidence target: Schemas committed; example proposal validates. |
| [ ] | `M6.5-002` | mock provider. Build: Deterministic provider that consumes a canned-script directory; supports inject-canned, inject-malformed, inject-timeout, inject-stale, inject-cost-overflow modes. Tests: Per-mode tests; CI uses mock only. Anti-scope: No live cloud calls in mock. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-ai::mind::provider::mock`. Evidence target: Mock provider used by all MIND-* tests. |
| [ ] | `M6.5-003` | provider trait + adapters. Build: Shared async trait; OpenAI Responses API adapter; Anthropic Messages API adapter; Ollama adapter; OpenAI-compatible adapter (vLLM/llama.cpp); each behind a cargo feature; secrets read from env per `MindProviderConfig.api_key_env`. Tests: Adapter contract tests with mocked HTTP; feature-gate tests verify default build excludes cloud. Anti-scope: No vendor SDK lock-in; no API keys in repo. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-ai::mind::provider` (cargo features `mind-openai`, `mind-anthropic`, `mind-ollama`, `mind-openai-compatible`). Evidence target: Adapter docs; example `MindProviderConfig`. |
| [ ] | `M6.5-004` | observation compressor. Build: Derive `MindObservationFrame` from the `cx-control` observation stream + recent replay events; enforce fog-of-war BEFORE any provider sees a prompt. Tests: Fog-of-war audit tests (synthetic hidden enemy never appears in frame); compactness tests; `cxctl observe --mind-frame <scope>` smoke. Anti-scope: No raw-state passthrough. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-ai::mind::compressor`, `cx-control`, `cx-replay`. Evidence target: Sample frames in run notes. |
| [ ] | `M6.5-005` | proposal validator. Build: Reject stale, invalid, impossible, unfair, over-budget, hidden-info, and capability-violating proposals; replay-visible reasons. Tests: Per-rejection-class unit tests; MIND-003/004/006/009 acceptance pass. Anti-scope: No silent acceptance. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-ai::mind::validator`. Evidence target: Validator decision log. |
| [ ] | `M6.5-006` | policy compiler. Build: Convert accepted proposals into utility-weight patches, commander goals, doctrine tags, dialogue-queue entries, and `MindMemoryRecord` writes via M6 hook points. Tests: Patch-application tests; doctrine-patch visibility test (MIND-005). Anti-scope: No direct low-level action emission. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-ai::mind::policy`. Evidence target: One visible doctrine patch in micro_breach_mind_lab. |
| [ ] | `M6.5-007` | mind events + run-bundle integration. Build: Emit `mind.task_created`, `mind.prompt_recorded` (hashes by default; raw text only behind `debug_capabilities`), `mind.response_received`, `mind.proposal_validated`, `mind.patch_applied`, `mind.patch_rejected`, `mind.memory_written`. Update run-bundle checker to recognize the `mind` category. Tests: Bundle-validation tests; secret-redaction tests. Anti-scope: No raw secrets in run bundles. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-replay`, `cx-ai::mind::events`, `tools/run_bundle_check.py`. Evidence target: Run bundles include `mind` events; redaction verified. |
| [ ] | `M6.5-008` | mind dashboard (dev). Build: Dev-only workbench panel showing task count, stale rate, provider failures, estimated cost, model routing, and accept/reject reasons. Tests: Dashboard render tests; screenshot. Anti-scope: No player-facing UI yet. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-tools-editor`, `cx-ui`. Evidence target: Dashboard capture in M6.5 note. |
| [ ] | `M6.5-009` | micro_breach_mind_lab scenario. Build: The M6.5 lab scenario in three modes (`mind_off`, `mind_mock`, `mind_live_optional`) with a sample commander mind profile and one designed doctrine-patch opportunity. Tests: Scenario validates with `cx-mod validate`; all three modes load. Anti-scope: No content tied to a specific cloud model id. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `content/scenarios/micro_breach_mind_lab.ron`, `content/mind/profiles/`. Evidence target: Scenario file + sample profile + canned-script. |
| [ ] | `M6.5-010` | MIND-* acceptance suite. Build: Implement `cx-ai --bin mind_lab` with `--suite MIND-001..MIND-010 --provider <mock\|...> --write-run-bundle`. Cover: baseline (off), nonblocking timeout, malformed response, stale response, doctrine-patch visibility, fog-of-war fairness, memory write, replay audit, cost cap, humanlike-score delta. Tests: All MIND-* pass against mock; AI-H regression remains green; failure modes produce useful first-divergence reports. Anti-scope: No reliance on live cloud during CI. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cx-ai`, `cx-headless`, `cx-bench`, `tests/`. Evidence target: MIND-001..MIND-010 run bundles archived; AI-H humanlike-score delta report. |
### M7 - Mission Director And Breach Contract

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M7-001` | manifest schema. Build: Typed manifest: teams, objectives, materials, command core, base systems, loadout requirements, director. Tests: Schema validation tests. Anti-scope: No full campaign generator. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cx-mission`, `content/scenarios/`. Evidence target: Manifest fixture in bundle. |
| [ ] | `M7-002` | director/commander. Build: Pacing, reinforcement, LZ risk, commander reason labels. Tests: Director phase tests. Anti-scope: No MMO war layer. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cx-mission`, `cx-ai`. Evidence target: `commander_decision.*`. |
| [ ] | `M7-003` | command core/base slice. Build: Rooted core powers shield/turret/door/repair; uproot/embed avatar tradeoff. Tests: CORE-A subset tests. Anti-scope: No full base builder. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cx-mission`, `cx-chassis`, `cx-ui`. Evidence target: `command_core_state_changed`, `base_power_changed`. |
| [ ] | `M7-004` | Breach Contract. Build: Playable mission: breach, fight, extract, win/loss/debrief. Tests: E2E win/loss; replay. Anti-scope: No campaign map. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `content/scenarios/`, `cx-app`. Evidence target: MISSION-A run bundles. |
| [ ] | `M7-005` | debrief/retry. Build: Comic-noir debrief with cause chain and retry same seed. Tests: UI/replay tests. Anti-scope: No full progression system. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cx-ui`, `cx-replay`, `cx-save`. Evidence target: Debrief screenshot. |
### M8 - Scenario Editor And Mod Tools

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M8-001` | editor workbench. Build: In-engine editor for spawns, materials, objectives, core/base state, loadout requirements. Tests: Editor state tests; focus/accessibility smoke. Anti-scope: No marketplace. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `cx-tools-editor`, `cx-ui`. Evidence target: Editor screenshots. |
| [ ] | `M8-002` | package builder. Build: Deterministic `.cxpkg`, manifest/provenance validation, dependency graph. Tests: Package determinism tests. Anti-scope: No public hosting. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `cx-mod`, `tools/`. Evidence target: PACK-A report. |
| [ ] | `M8-003` | script host. Build: Implement chosen Lua/Rhai sandbox with capability declarations. Tests: Sandbox denies FS/network by default. Anti-scope: No unbounded script API. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `cx-mod`. Evidence target: Script-host test report. |
| [ ] | `M8-004` | sample mod. Build: New chassis + scenario + AI doctrine sample mod. Tests: Validate/load/run sample mod. Anti-scope: No full mod catalog. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `mods/sample_*`, `content/`. Evidence target: Modded run bundle. |
### M9 - Headless Server And Determinism Islands

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M9-001` | headless binary. Build: Run sim without renderer/audio; load scenario; accept scripted inputs. Tests: Linux headless smoke. Anti-scope: No public server browser. | [[spec/native-implementation-backlog#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cx-headless`, `cx-app`. Evidence target: Headless logs in bundle. |
| [ ] | `M9-002` | determinism contracts. Build: Document deterministic/stochastic/cosmetic subsystems. Tests: Contract tests. Anti-scope: No whole-engine determinism claim. | [[spec/native-implementation-backlog#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cx-sim-core`, `cx-replay`, docs. Evidence target: Determinism report. |
| [ ] | `M9-003` | replay-from-events. Build: 10-minute M7 replay verifies actor/terrain/inventory checksums. Tests: Replay compare. Anti-scope: No network sync yet. | [[spec/native-implementation-backlog#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cx-headless`, `cx-replay`. Evidence target: First-divergence report if fail. |
| [ ] | `M9-004` | headless perf. Build: 10x real-time replay validation target. Tests: Bench test. Anti-scope: No optimization-only rabbit hole. | [[spec/native-implementation-backlog#M9 — Headless Server And Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cx-bench`, `cx-headless`. Evidence target: Perf report. |
### M10 - LAN Co-op

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M10-001` | authority model. Build: Server-authoritative input/snapshot/event model. Tests: Unit tests for input validation. Anti-scope: No anti-cheat product. | [[spec/native-implementation-backlog#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Owns: `cx-net`, `cx-sim-core`. Evidence target: Authority memo. |
| [ ] | `M10-002` | LAN discovery/lobby. Build: Host/list/join on LAN; ready-up. Tests: Local two-client smoke. Anti-scope: No NAT/relay. | [[spec/native-implementation-backlog#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Owns: `cx-net`, `cx-ui`. Evidence target: Lobby screenshot. |
| [ ] | `M10-003` | replication. Build: Actors, terrain, inventory, objective state replicate; per-client bundles align. Tests: Replay compare across clients. Anti-scope: No public matchmaking. | [[spec/native-implementation-backlog#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Owns: `cx-net`, `cx-replay`. Evidence target: Two-client run bundles. |
### M11 - Online Co-op Private

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M11-001` | transport adapter. Build: NAT/relay candidate behind trait boundary. Tests: Simulated latency tests. Anti-scope: No platform lock-in. | [[spec/native-implementation-backlog#M11 — Online Co-op Private]] | - | - | - | - | - | - | - | Owns: `cx-net`. Evidence target: Transport decision note. |
| [ ] | `M11-002` | package hash sync. Build: Join preflight checks content hashes and produces clean mismatch actions. Tests: Mismatch tests. Anti-scope: No public mod CDN. | [[spec/native-implementation-backlog#M11 — Online Co-op Private]] | - | - | - | - | - | - | - | Owns: `cx-net`, `cx-mod`, `cx-ui`. Evidence target: Join-failure screenshots. |
| [ ] | `M11-003` | online session smoke. Build: Two remote clients complete private co-op Breach Contract. Tests: Remote run compare. Anti-scope: No public launch promise. | [[spec/native-implementation-backlog#M11 — Online Co-op Private]] | - | - | - | - | - | - | - | Owns: `cx-net`, `cx-app`. Evidence target: Per-client bundles. |
### M12 - PvP And MMO Experiments

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M12-001` | PvP arena. Build: 2-4 players, small destructible map, server-authoritative validation. Tests: Stress run. Anti-scope: No launch PvP promise. | [[spec/native-implementation-backlog#M12 — PvP And MMO Experiments]] | - | - | - | - | - | - | - | Owns: `cx-net`, `cx-mission`. Evidence target: Bandwidth/cheat notes. |
| [ ] | `M12-002` | scale shard. Build: N=20 small-shard simulation for 10 minutes. Tests: Load test. Anti-scope: No MMO product commitment. | [[spec/native-implementation-backlog#M12 — PvP And MMO Experiments]] | - | - | - | - | - | - | - | Owns: `cx-headless`, `cx-net`, `cx-bench`. Evidence target: Perf/desync report. |
| [ ] | `M12-003` | DR-005 review. Build: Revisit multiplayer posture with evidence. Tests: N/A. Anti-scope: No silent scope expansion. | [[spec/native-implementation-backlog#M12 — PvP And MMO Experiments]] | - | - | - | - | - | - | - | Owns: vault only. Evidence target: Updated DR or research log. |

---

## Global Validation And Bug Hunt Checklist

These rows come from the roadmap validation matrix, bug-hunt checklist, and definition of done. They should be updated at milestone closeout, not during every tiny edit.

### Validation Command Matrix

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `VAL-01` | Formatting: `cargo fmt --all --check` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M0 |
| [ ] | `VAL-02` | Compile: `cargo check --workspace --all-targets` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M0 |
| [ ] | `VAL-03` | Lints: `cargo clippy --workspace --all-targets -- -D warnings` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M0 |
| [ ] | `VAL-04` | Unit/integration tests: `cargo test --workspace` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M0 |
| [ ] | `VAL-05` | Native app smoke: `cargo run -p cx-app -- --scenario <milestone-smoke> --run-seconds 5 --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M0 |
| [ ] | `VAL-06` | Control API smoke: `cargo run -p cxctl -- observe --once` and `cargo run -p cxctl -- run --ticks 300 --write-run-bundle` against the current milestone scene. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M0 |
| [ ] | `VAL-07` | Run-bundle validation: `python3 research_tools/prototype_run_check.py prototype_runs/native/<run_id>` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M0 |
| [ ] | `VAL-08` | Scripted E2E: `cargo run -p cx-e2e -- --scenario <scenario-id> --expect <result> --write-run-bundle`; prefer `cxctl`/control API actions over OS-level input. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M1.5 |
| [ ] | `VAL-09` | Observation stream check: Stream `cargo run -p cxctl -- observe --stream --hz 30` during a scripted run and verify tick/order/event freshness. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M1.5 |
| [ ] | `VAL-10` | Replay check: `cargo run -p cx-headless -- replay prototype_runs/native/<run_id> --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M3 |
| [ ] | `VAL-11` | Screenshot/capture check: Capture listed in `summary.json.artifacts`; verify no blank/overlap failure. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M1.5 visual runs; M4 required |
| [ ] | `VAL-12` | Perf sample: `cargo run -p cx-bench -- --scenario <scenario-id> --profile milestone` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M2 |
| [ ] | `VAL-13` | Accessibility smoke: `cargo run -p cx-e2e -- --scenario <scenario-id> --ui-scale 2.0 --high-contrast --verify-focus` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M4 |
| [ ] | `VAL-14` | Save/load roundtrip: `cargo run -p cx-e2e -- --scenario <scenario-id> --save-load-roundtrip --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M5/T-SAVE |
| [ ] | `VAL-15` | Full collision gauntlet: `cargo run -p cx-e2e -- --scenario m5_5_full_collision_gauntlet --suite COLL-001..COLL-012 --write-run-bundle` then `cargo run -p cx-headless -- replay prototype_runs/native/<m5_5_run> --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M5.5/T-PHYS |
| [ ] | `VAL-16` | Collision observation stream: `cargo run -p cxctl -- observe --collisions --stream --hz 30 --scenario m5_5_full_collision_gauntlet` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M5.5/T-PHYS |
| [ ] | `VAL-17` | AI harness: `cargo run -p cx-ai --bin ai_harness -- --suite AI-H-01..AI-H-06 --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6 |
| [ ] | `VAL-18` | Mind frame observation: `cargo run -p cxctl -- observe --mind-frame squad_alpha --once` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-19` | Mind lab suite (mock): `cargo run -p cx-ai --bin mind_lab -- --suite MIND-001..MIND-010 --provider mock --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-20` | Mind cost-cap smoke: `cargo run -p cx-ai --bin mind_lab -- --suite MIND-009 --provider mock --max-run-cost-usd 0.0 --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-21` | Mind fairness audit: `cargo run -p cx-ai --bin mind_lab -- --suite MIND-006 --provider mock --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-22` | Package/mod validation: `cargo run -p cx-mod -- validate content/ mods/ --strict` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M8 |
| [ ] | `VAL-23` | Headless server smoke: `cargo run -p cx-headless -- --scenario breach_contract --ticks 3600 --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M9 |
| [ ] | `VAL-24` | LAN/online replay alignment: Compare per-client run bundles with `cx-headless replay-compare`. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M10+ |

### Bug Hunt Checklist

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `BUG-01` | Crashes/hangs: Can reset, exit, alt-tab, reload scenario, and replay complete without panic/deadlock? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-02` | Input: Are repeated inputs, held inputs, lost focus, mouse capture, controller fallback, and remap paths sane? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-03` | Replay/events: Are required events present, ordered, parent-linked, counted, and linked to visible behavior? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-04` | Determinism: If a deterministic claim is made, where is the checksum proof and first-divergence report? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-05` | UI/readability: Does UI fit at 100%, 150%, and 200%; are critical states not color-only; are labels non-overlapping? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-06` | Terrain/physics/collision: Do high-speed impacts, edge collisions, tiny holes, chunk borders, repeated edits, limb contacts, projectile-projectile contacts, weapon collisions, friendly body blocking, debris impacts, and mech crush contacts behave predictably? Are all collision filters reason-labeled? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-07` | AI: Can the AI explain perception, chosen tactic, refused action, stuck state, and recovery? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-08` | Save/load: Does save/load preserve identities, events, objective state, terrain, equipment, and checksums where promised? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-09` | Performance: Are frame spikes, sim tick cost, event volume, dirty-region cost, and memory growth reported? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-10` | Platform: Are path separators, case sensitivity, file watching, audio, input, and GPU backend assumptions portable? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-11` | Mod/package: Do bad packages fail with actionable diagnostics instead of panic/crash? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-12` | Documentation: Are roadmap/backlog/source links current; are ghost DRs or stale Slice-A references avoided? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |

### Definition Of Done

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `DOD-01` | Code: Implemented in the owned crates/files named by [[spec/native-implementation-backlog]]. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-02` | Tests: Unit/integration tests added for new core behavior and failure paths. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-03` | E2E: Milestone reference scenario runs from command line and produces expected outcome. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-04` | Run bundle: Bundle exists under `prototype_runs/native/` and passes the checker. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-05` | Replay: Required replay/checksum claims are backed by headless verification or explicitly not claimed. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-06` | Collision/physics: Any new physical object has a collision class/proxy/matrix entry/event policy or a tested cosmetic/sensor/filter reason. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-07` | Perf: Perf counters exist; T-PERF target status is recorded as pass/fail/blocked. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-08` | UI/accessibility: Any user-facing surface has screenshot evidence and ACC-A status when applicable. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-09` | Bug hunt: Bug checklist is completed; found bugs are fixed or logged as accepted known issues. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-10` | Vault: Prototype/research note is updated with run links, test commands, screenshots, final audit, and next actions. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-11` | Feature checklist: [[spec/feature-completion-checklist]] rows are updated for affected roadmap features, milestone scope, done-criteria, side tracks, and native task cards. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - | Added 2026-05-05 to match the roadmap's 12-row Definition Of Done. |
| [ ] | `DOD-12` | Human gates: Human-only checks are marked `READY_FOR_HUMAN`, with a short playtest checklist. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |

---

## Maintenance Notes

- This file intentionally duplicates roadmap/backlog items so completion and rating state can live in one place. Keep the roadmap/backlog as the source for build instructions.
- If a future pass renames milestone ids or task card ids, preserve old ids in notes until any evidence links have been migrated.
- Human `H-*` ratings should be left blank until the user provides them. Agents may suggest ratings but must label them as AI suggestions, not human ratings.
- For subjective items like feel, fun, readability, AI believability, or UX polish, mark agent-completable evidence first and leave the human gate ready for playtest.
