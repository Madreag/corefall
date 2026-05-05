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
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[references/prototype-run-bundle-schema|run-bundle schema]]

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
| Headless bridge | Same commands work in `cx-headless`; rendering is optional. | M3/M9 |
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
| Objectives | active tasks, timers, extraction, fail states, command-core/base-power state. |
| UI tree | windows, panels, buttons, sliders, lists, focus target, stable ids, text, enabled/disabled state, bounds. |
| Audio/caption feed | caption id, source, priority, spatial hint, transcript, alert class. |
| Recent events | input, combat, body, terrain, AI, mission, UI, performance, warnings, errors, parent-event chain. |
| Performance | frame time, sim tick cost, event volume, control API latency, dropped observation frames. |

## Action Model

The automation layer should only expose actions that map to real gameplay or UI affordances unless an explicit debug capability is enabled.

| Action Family | Examples |
|---|---|
| Player controls | move axis, jump, crouch, aim vector, fire, reload, use tool, switch item, interact, stop. |
| Tactical controls | select unit, issue order, queue order, set rally point, follow/protect/breach/retreat, assume direct control, release to AI. |
| UI controls | focus by id, click by id, set slider, choose option, type text, submit/cancel, navigate tabs. |
| Scenario controls | load scenario, reset, set seed, pause, step ticks, run for N ticks, set speed, capture bundle. |
| Inspection | query entity, query UI tree, query terrain patch, query event chain, query last failure reason. |
| Debug-only | spawn fixture, teleport, force damage, reveal map, grant item. Disabled unless the run manifest declares debug capability. |

All action requests should return `accepted`, `rejected`, or `queued`, with a reason label and the tick where the command took effect.

## Minimum Commands

During development, invoke the CLI as `cargo run -p cxctl -- ...` until a local `cxctl` binary is installed or added to PATH. The examples below use the shorter installed-binary form.

```text
cxctl scenario load micro_breach --seed 42
cxctl observe --once --format json
cxctl observe --stream --hz 30
cxctl act move --x 1.0
cxctl act aim --world 320,140
cxctl act fire --pressed true
cxctl ui tree
cxctl ui click loadout.confirm
cxctl run --ticks 600 --write-run-bundle
cxctl assert objective.result == win
cxctl replay verify prototype_runs/native/<run_id>
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
| M6 | Reuse the same layer for AI-H harness bots; bot decisions cite observation fields and event ids. |
| M7 | Scenario director, command-core/base-power, debrief, and retry are controllable/queryable. |
| M8 | Editor and mod tooling expose semantic UI and package validation commands. |
| M9+ | Headless/server modes use the same command/observation contract for replay, LAN, online, and scale tests. |

## Definition Of Done

- An AI implementation agent (Codex, Factory droid, Claude Code, Cursor, or any future tool) can launch a scenario, observe state, move/aim/fire/use UI, run assertions, and write a run bundle without image-based control.
- Every player-visible control has a matching semantic action or UI command.
- Every critical state a human can understand from the screen has a structured observation, caption, or event equivalent.
- E2E tests for gameplay and UI prefer `cxctl`/control API over OS-level input where possible.
- The interface is documented enough for future bot authors to build against versioned schemas.

## Source Trail

- [[spec/prototype-roadmap]] — pinned transport, CLI reference, repository layout, kickoff smoke.
- [[spec/native-implementation-backlog]] — M0-006 control bootstrap task card, milestone integration tasks.
- [[references/prototype-run-bundle-schema]] — `control` event category and run-bundle gates.
- [[systems/replay-determinism-and-run-evidence]] — deterministic-island contract.
- [[spec/ai-trust-harness-slice-a]] — AI-H scenario runner reuses this layer.
- [[spec/ux-wireframes-slice-a]] — UI-tree primitives.
