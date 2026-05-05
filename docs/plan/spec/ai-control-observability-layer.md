---
type: spec
status: planning-anchor-v0
authority: "Developer/AI control, observability, and future bot-authoring interface for native prototypes and the eventual game."
feeds:
  - DR-002
  - DR-004
  - DR-008
  - DR-009
  - DR-012
  - DR-013
  - DR-022
  - DR-024
  - DR-033
  - DR-034
  - DR-035
  - DR-036
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/full-collision-physics-plan|full collision plan]] · [[spec/server-app-architecture|server app]] · [[spec/persistent-mmo-architecture|persistent MMO]] · [[comparables/noita-grade-material-simulation-research|material simulation research]] · [[references/prototype-run-bundle-schema|run-bundle schema]]

# AI Control And Observability Layer

> [!summary] Goal
> Build an "eyes, ears, hands, and voice" layer into the game from M0 onward so AI agents, automated tests, accessibility tools, and future bot authors can observe structured state and issue real player/UI actions without relying on screenshots.

## Design Principle

The game should be controllable through the same semantic action model a human uses, and observable through structured state streams that explain what the game knows, not only what pixels show.

Screenshots and video captures remain useful evidence, but they are not the primary automation interface. The primary interface is a local, scriptable, tick-aware control and telemetry API.

## Interface Stack

| Layer | Purpose | First Milestone |
|---|---|---|
| `cx-control` crate | Shared command/observation/event schemas. | M0 |
| Local control server | JSON-RPC or compact JSON messages over localhost WebSocket plus optional Unix domain socket / named pipe. | M0/M1 |
| CLI client | `cxctl` for scripts: load scenario, step, observe, act, replay, assert, dump bundle. | M0/M1 |
| Headless/server bridge | Same commands work in `cx-headless` and capability-gated `cx-server`; rendering is optional. | M3/M9 |
| UI semantic tree | Query/click/type/focus every UI action by stable id, role, label, state, and bounds. | M4 |
| Bot SDK surface | Versioned API for player bots and modded agents, backed by the same command/observation schemas. | M6/M8 |

Recommended initial transport: localhost-only WebSocket JSON-RPC because it is easy for any AI agent (Codex, Factory droids, Claude Code, Cursor), Python, Node, browser tools, and future community bot authors. Keep the schema transport-neutral so it can later run over UDS, pipes, replay files, or server-authoritative net channels.

> [!info] Transport pin
> The concrete transport (JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890`, optional UDS, mandatory `schema_version`, heartbeat) is pinned in [[spec/prototype-roadmap#control-transport-and-envelope]]. Schemas live under `cortex-game/crates/cx-control/schemas/v<N>/`.

### Concrete Envelope Examples

Request:

```json
{ "jsonrpc": "2.0", "id": 7, "method": "act.player.move", "params": { "schema_version": 1, "x": 1.0, "y": 0.0 } }
```

Successful response:

```json
{ "jsonrpc": "2.0", "id": 7, "result": { "schema_version": 1, "status": "accepted", "effective_tick": 1234, "reason": null } }
```

Rejected response:

```json
{ "jsonrpc": "2.0", "id": 7, "error": { "code": -32099, "message": "command_rejected", "data": { "reason": "actor_downed", "tick": 1234 } } }
```

Streaming observation notification:

```json
{
  "jsonrpc": "2.0",
  "method": "observe.frame",
  "params": {
    "schema_version": 1,
    "tick": 1234,
    "scenario": "m1_actor_range",
    "run_id": "m1_2026-05-04T22-30-00Z_a1b2c3d4",
    "actors": [{ "id": "alpha:0", "team": "alpha", "pos": [120.0, 64.0], "status": "STABLE" }],
    "ui_tree": { "focused": "hud.ammo", "interactable_count": 7 },
    "events_since": 1212,
    "events": [{ "id": "m1_...:1234:0", "type": "weapon_fired", "payload": { "weapon": "rifle.default" } }]
  }
}
```

Schema-version mismatch:

```json
{ "jsonrpc": "2.0", "id": 7, "error": { "code": -32602, "message": "InvalidParams", "data": { "reason": "schema_version_mismatch", "server_version": 1, "client_version": 2, "fix_hint": "Upgrade cxctl to v0.2 or pin client schema_version: 1" } } }
```

### Versioning

| Change | Bump |
|---|---|
| Add an optional field. | No bump (clients keep working). |
| Add a required field or remove a field. | Major bump (`schema_version` → next integer). Migration handlers MUST be registered server-side. |
| Rename a method. | Major bump. Old method kept under a `deprecated` alias for one schema version, then removed. |
| Add a new method. | No bump if optional; major if it replaces an existing flow. |

## Observation Model

Every observation packet should include enough data for an AI agent to decide and explain its next action.

| Field | Examples |
|---|---|
| Clock | run id, tick, dt, paused/stepping mode, scenario id, seed. |
| Player context | controlled actor, selected unit, command-core state, camera mode, active input mode. |
| Actors | stable ids, team, position, velocity, aim, stance, health/status, body zones, armor, chassis, inventory, visible damage, AI intent label. |
| Equipment | selected item, ammo, heat/energy, reload state, jam/damage stage, valid actions, refusal reasons. |
| Terrain | sampled local material grid, breachable surfaces, hazards, path blockers, recent edits, tool affordances. |
| Materials and atmospheres | active material cells, liquids/gases, local reactions, fire/electric/toxic hazards, pressure/oxygen, afflictions, containment state, and material-lab fixtures. |
| Objectives | active tasks, timers, extraction, fail states, command-core/base-power state. |
| UI tree | windows, panels, buttons, sliders, lists, focus target, stable ids, text, enabled/disabled state, bounds. |
| Audio/caption feed | caption id, source, priority, spatial hint, transcript, alert class. |
| Recent events | input, combat, body, terrain, AI, mission, UI, performance, warnings, errors, parent-event chain. |
| Collision state | Current contact pairs, collision filters, contact normals, TOI, impulse summaries, projectile deflections, recent collision damage, and collision budget status. |
| Performance | frame time, sim tick cost, event volume, control API latency, dropped observation frames. |

## Action Model

The automation layer should only expose actions that map to real gameplay or UI affordances unless an explicit debug capability is enabled. **The rule is: any pixel a human can interact with on screen, the AI must be able to drive through `cxctl`.**

| Action Family | Examples |
|---|---|
| Player controls | move axis, jump, crouch, aim vector, fire, reload, use tool, switch item, interact, drop, throw, stop. |
| Tactical controls | select unit/squad/faction, issue order (`move-to`, `attack`, `defend`, `retreat`, `breach`, `repair`, `support`, `follow`, `hold`, `extract`, `rescue`, `salvage`), queue order, set rally point, set doctrine, assume direct control, release to AI. |
| Camera controls | pan, zoom, follow target, switch mode (side / tactical-map / replay-scrub), set slowdown ratio. |
| UI controls | focus by id, click/double-click by id, hover (triggers tooltips/preview), set slider/select/checkbox/radio/text, type text, submit/cancel, navigate tabs, press individual keys (Tab/Enter/Esc/Arrow/F-keys/Ctrl+key combos), assert UI properties. |
| Scenario controls | load scenario, reset, set seed, pause, step ticks, run for N ticks, set speed, capture bundle, force `runbundle.write`. |
| Save controls | save slot, load slot, autosave, ironman flag, scenario policy override. |
| Settings controls | set UI scale, contrast mode, captions on/off, reduced motion/shake/flash, keybinds, language pack. Persist or transient. |
| Mod controls | enable/disable/validate/reload mod packs; check trust tier; check capability declarations. |
| Director controls (debug-gated) | force director phase, force reinforcement, force objective state, escalate, hint scenario hooks. Logged in manifest. |
| Inspection | query entity (actor/equipment/chassis/mission/base/objective/order/affliction/event), query UI tree with bounds, query terrain patch, query material patch, query atmosphere cell, query reaction chain, query event chain (parent/children), query last failure reason, query collision pair + filter reason + damage chain, query AI intent + reason chain, query mission director phase + commander reasons, query save slot list. |
| Debug-only | spawn fixture, teleport, force damage, reveal map, grant item. Disabled unless the run manifest declares `debug_capabilities`. Every debug action emits a `system.debug_action_used` event. |

All action requests should return `accepted`, `rejected`, or `queued`, with a reason label and the tick where the command took effect.

> [!important] Coverage rule
> If a human can do it on screen — click a button, drag a slider, type into a textbox, press a key, hover for a tooltip, switch tabs, scrub a replay, save/load, change a setting, queue an order, switch doctrine, change camera — the AI worker MUST be able to do the same thing through `cxctl` or the JSON-RPC envelope. Screenshot-only debugging is not a substitute. If a UI surface lacks a `cx-control` path, the milestone is incomplete.

## Minimum Commands

During development, invoke the CLI as `cargo run -p cxctl -- ...` until a local `cxctl` binary is installed or added to PATH. The examples below use the shorter installed-binary form.

```text
# Scenario / sim flow
cxctl scenario load micro_breach --seed 42
cxctl pause
cxctl step --ticks 60
cxctl resume
cxctl run --ticks 600 --write-run-bundle

# Observation (eyes/ears)
cxctl observe --once --format json
cxctl observe --stream --hz 30
cxctl observe --hud --stream --hz 10
cxctl observe --captions --stream --hz 10
cxctl observe --mission --once
cxctl observe --debrief --once
cxctl observe --ai --stream --hz 5
cxctl observe --base --once
cxctl observe --camera --once
cxctl observe --collisions --stream --hz 30
cxctl observe --materials --stream --hz 30 --scope chunk:0,0
cxctl observe --atmosphere --stream --hz 10
cxctl observe --reactions --stream --hz 30
cxctl observe --replay --once
cxctl observe --perf --stream --hz 1
cxctl observe --settings --once

# Inspection (deep dives)
cxctl inspect actor alpha:0
cxctl inspect equipment alpha:0:weapon
cxctl inspect chassis alpha:0
cxctl inspect mission --with-events
cxctl inspect base core:0 --with-events
cxctl inspect objective breach.win
cxctl inspect order alpha:1:move-to-7
cxctl inspect affliction alpha:0:burning
cxctl inspect collision <event-id> --with-parents --with-children
cxctl inspect material <event-id>
cxctl inspect reaction <event-id>
cxctl inspect event <event-id> --depth 5

# Player actions (hands)
cxctl act move --x 1.0
cxctl act aim --world 320,140
cxctl act fire --pressed true
cxctl act reload
cxctl act use-tool digger
cxctl act switch-item --slot primary

# Tactical actions
cxctl act tactical select alpha:1
cxctl act tactical order move-to --target 320,140 --reason "flank_left"
cxctl act tactical order breach --target door:0 --reason "objective_ingress"
cxctl act tactical doctrine cautious --unit alpha:1

# Camera + UI
cxctl act camera mode tactical-map
cxctl act camera follow alpha:0
cxctl ui tree --with-bounds
cxctl ui click loadout.confirm
cxctl ui hover hud.module.jet
cxctl ui set settings.ui_scale 200
cxctl ui type chat.input "covering fire on left"
cxctl ui press Tab
cxctl ui press Ctrl+S
cxctl ui assert hud.objective contains "Breach"
cxctl ui focus settings.captions

# Save / settings / mods
cxctl act save save 1 --description "before final breach"
cxctl act save load 1
cxctl act settings set captions on --persist
cxctl act keybind primary_fire MouseLeft
cxctl act mod validate --pack sample_breach --strict

# Director (debug-gated)
cxctl act director phase escalation --reason "scripted_test"

# Assertion + replay
cxctl assert objective.result == win
cxctl replay verify prototype_runs/native/<run_id>
cxctl replay scrub prototype_runs/native/<run_id> --tick 1850
cxctl runbundle write
cxctl health --format json
```

## Latency And Throughput Targets

| Target | Requirement |
|---|---|
| Control latency | Action accepted on the next fixed sim tick in local/headless mode. |
| Observation cadence | 20 Hz minimum for normal AI control; 60 Hz option for movement/aim tests. |
| Event stream | Lossless for normal milestone runs; dropped counts visible under stress. |
| Snapshot size | Configurable detail levels: `minimal`, `agent`, `debug`, `full`. |
| Non-blocking | Observation publishing must not stall the sim/render loop. |

## Safety And Capability Boundaries

| Capability | Default |
|---|---|
| Local control server | Disabled for normal player builds unless launched with `--control-api`. |
| Network exposure | Loopback only by default. Remote access requires explicit host/port/capability token. |
| Debug commands | Off by default; manifest must record `debug_capabilities: true`. |
| Bot access | Scenario/mod manifest declares which observation/action capabilities are allowed. |
| Replays | Control commands are recorded as events so failures are reproducible. |

## Milestone Integration

| Milestone | Required Integration |
|---|---|
| M0 | Define `cx-control` schemas; expose `observe --once`, `run --ticks`, `pause`, `step`, run-bundle hooks. |
| M1 | Control movement/aim/fire/reload through the command API; stream actor/equipment observations. |
| M1.5 | Script both win and loss Micro Breach runs entirely through `cxctl`; no manual input required for E2E. |
| M2 | Add terrain patch/material/affordance observations and dig/refusal actions. |
| M3 | Replay control commands and verify action/event timing. |
| M4 | Expose semantic UI tree and UI actions; screenshots become audit evidence, not control dependency. |
| M5 | Expose equipment, chassis, armor, damage-stage, eject, repair, and salvage observations/actions. |
| M5.5 | Expose `cxctl observe --collisions` and `cxctl inspect collision <event-id>` for collision matrix, live contacts, filters, projectile-projectile outcomes, impulse damage, CCD/TOI, and collision budget state (see [[spec/full-collision-physics-plan]] and [[decisions/dr-033-full-collision-physics-direction]]). |
| M5.6 | Expose `cxctl observe --materials`, `cxctl inspect material <cell-or-region>`, and deterministic material/reaction event chains for active-region CA, material transitions, containment, fire/liquid/gas/electric interactions, and material budget state (see [[comparables/noita-grade-material-simulation-research]] and [[decisions/dr-036-systemic-material-simulation-direction]]). |
| M6 | Reuse the same layer for AI-H harness bots; bot decisions cite observation fields and event ids. |
| M6.5 | Derive `MindObservationFrame` from this layer with fog-of-war filtering; expose `cxctl observe --mind-frame <scope>` for LLM mind workers (see [[spec/hybrid-llm-ai-plan]] and [[decisions/dr-032-hybrid-llm-ai-direction]]). |
| M6.6 | Add AI-facing material competence observations: hazard labels, safe/unsafe material affordances, containment opportunities, and explainable avoidance/rescue decisions. |
| M7 | Scenario director, command-core/base-power, debrief, and retry are controllable/queryable. |
| M7.5 | Expose `cxctl observe --atmosphere`, room pressure/oxygen/toxin state, leak paths, powered doors/vents/shields, and base-life-support events for command-core/base atmospherics. |
| M8 | Editor and mod tooling expose semantic UI and package validation commands. |
| M8.5 | Material lab scenarios expose fixture setup, reaction assertions, material sample export/import, and player-authored material test bundles through the same control contract. |
| M9 | `cx-server` exposes the same `cx-control` envelope for admin (capability-gated) + observation. `cxctl --target server://host:port` connects to a running server for ops/audit (see [[spec/server-app-architecture]] and [[decisions/dr-034-dedicated-server-application]]). |
| M10..M12 | Per-client and per-server run bundles use the same envelope; replay/replay-compare verifies multi-client and shard observation streams. |
| M9+ | Headless/server modes use the same command/observation contract for replay, LAN, online, public PvP, and persistent MMO shards. |

### Derived: Mind Observation Frames

The full observation stream is the source of truth. Mind workers (LLM advisors, DR-032 / [[spec/hybrid-llm-ai-plan]]) consume a **derived, compact, fog-of-war-filtered subset** called `MindObservationFrame`. The compressor lives in `cx-ai::mind::compressor` and reads from this layer. Fog-of-war is enforced **before** any provider sees a prompt.

`cxctl observe --mind-frame <scope>` returns a single frame for `actor`, `squad`, `faction`, `mission_director`, or `post_mission` scope (with optional `--ref <id>` to pin the subject). This is the same semantic surface that mind workers consume; CI uses it for fairness audits.

### Derived: Collision Observation Frames

The full observation stream also exposes a collision-focused view for T-PHYS and M5.5. `cxctl observe --collisions` returns:

- active pair ids, entity ids, collision classes, and filter reasons;
- contact point, normal, TOI fraction, relative velocity, and impulse summary;
- recent `collision.*` events with parent cause chains;
- projectile-projectile outcomes such as deflect, fragment, fuze-fail, or detonate;
- low-value contact budget/degradation counters;
- first-divergence data during replay verification.

This view is mandatory for implementation agents. They should be able to debug collision without repeatedly screenshotting the app.

### Derived: Material Observation Frames

The material-focused view is the mandatory debug surface for T-MAT and DR-036. `cxctl observe --materials` and `cxctl observe --atmosphere` return:

- active-region bounds, dirty cells, material ids, temperature/electric/toxic/flammable state, and material budget counters;
- liquid/gas/solid layering, containment, leak paths, pressure, oxygen, and toxin summaries;
- recent `material.*`, `reaction.*`, `atmosphere.*`, and `affliction.*` events with parent cause chains;
- reaction explanations such as `water_neutralized_acid`, `oil_ignited_by_spark`, `toxic_gas_asphyxiated_actor`, or `electricity_conducted_through_liquid`;
- AI-readable hazard affordances and refusal reasons;
- first-divergence data during replay verification.

This view is required before any material hazard can graduate from lab fixture to combat scenario. If the player can die from it, an AI worker must be able to inspect and replay why.

## Definition Of Done

- An AI implementation agent (Codex, Factory droid, Claude Code, Cursor, or any future tool) can launch a scenario, observe state, move/aim/fire/use UI, run assertions, and write a run bundle without image-based control.
- Every player-visible control has a matching semantic action or UI command.
- Every critical state a human can understand from the screen has a structured observation, caption, or event equivalent.
- Every critical material, atmosphere, collision, and server/shard state has a structured observation or event equivalent before it is treated as milestone-complete.
- E2E tests for gameplay and UI prefer `cxctl`/control API over OS-level input where possible.
- The interface is documented enough for future bot authors to build against versioned schemas.

## Source Trail

- [[spec/prototype-roadmap]] — pinned transport, CLI reference, repository layout, kickoff smoke.
- [[spec/native-implementation-backlog]] — M0-006 control bootstrap task card, milestone integration tasks.
- [[spec/full-collision-physics-plan]] — collision observation and M5.5 T-PHYS contract.
- [[comparables/noita-grade-material-simulation-research]] — material/atmosphere/reaction research feeding T-MAT.
- [[decisions/dr-036-systemic-material-simulation-direction]] — material simulation direction and M5.6/M5.7/M6.6/M7.5/M8.5 hooks.
- [[spec/server-app-architecture]] — dedicated server control/admin surface.
- [[spec/persistent-mmo-architecture]] — shard observation and replay/ops needs.
- [[references/prototype-run-bundle-schema]] — `control` event category and run-bundle gates.
- [[systems/replay-determinism-and-run-evidence]] — deterministic-island contract.
- [[spec/ai-trust-harness-slice-a]] — AI-H scenario runner reuses this layer.
- [[spec/ux-wireframes-slice-a]] — UI-tree primitives.
