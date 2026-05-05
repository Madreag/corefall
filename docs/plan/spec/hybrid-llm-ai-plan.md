---
type: spec
status: closed-direction
created: 2026-05-05
topic: hybrid local AI plus LLM cognition
authority: "Direction-grade plan for the async LLM 'mind' layer that augments local AI. Local AI is never blocked on an LLM. Provider/model/specific schema versions remain open; the architectural posture is closed."
ready_when: "M6.5 lands MIND-001..MIND-010 against the deterministic mock provider with M6 local AI continuing to act under provider failure/sleep/stale."
feeds:
  - DR-002
  - DR-006
  - DR-008
  - DR-009
  - DR-012
  - DR-013
  - DR-022
  - DR-024
  - DR-032
---

← [[spec/index|spec index]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/ai-control-observability-layer|control layer]] · [[decisions/dr-008-ai-architecture|DR-008]] · [[decisions/dr-022-ai-humanlike-bar|DR-022]]

# Hybrid LLM AI Plan

> [!summary] Recommendation
> Add an async "LLM mind layer" to the roadmap, but never place an LLM in the reflex loop. The shipped AI must remain strong with zero network access: local utility, behavior tree/GOAP, navigation, perception, squad jobs, and commander rules keep moving at frame speed. LLMs run in the background as optional higher-level cognition: doctrine changes, squad intent, personality, post-mission reflection, deception, chatter, memory, profile evolution, mission-director adaptation, and mod/workbench assistance.

> [!important] Roadmap integration
> Add a side track named **T-LLM - Async LLM Mind Layer** and a milestone named **M6.5 - LLM Mind Lab** after the local AI baseline in M6. M6 remains local-only. M6.5 proves provider adapters, strict schemas, validation, replay logging, cost/latency budgets, deterministic fallback, and one visible doctrine change in a controlled breach scenario.

## Why This Belongs In The Game

The user goal is "most humanlike AI in the genre", not just better bots. Traditional game AI can be fast, fair, inspectable, deterministic, and fun. LLMs can add long-horizon interpretation, personality, memory, explanation, squad voice, narrative adaptation, and creative doctrine changes. The right design is a hybrid:

| Layer | Responsible for | Cadence | LLM allowed? |
|---|---|---:|---|
| Reflex | Dodge, fire, aim assistance, jump, crawl, flee grenade, avoid fire, immediate pickup/drop, emergency retreat. | 8-16 ms | No |
| Tactic | Cover choice, reload timing, flank attempt, suppress, breach, rescue, tool choice, local formation. | 100-250 ms | No direct control |
| Job/Commander | Squad role assignment, objective priorities, base defense posture, request reinforcements, retreat threshold. | 0.5-2 s | Optional proposal input |
| LLM Mind | Doctrine patch, intent explanation, personality, memory extraction, enemy campaign adaptation, mission director flavor, post-mission debrief, modded profile generation. | 2-30 s, or between missions | Yes |
| Strategic Reflection | "Player always digs from below", "snipers beat us", "send shield mech next time", faction memory, named actor growth. | between missions / background | Yes |

The key rule: the game AI never waits for the LLM. Local AI keeps acting. When an LLM result arrives, it is treated as a proposal with a time-to-live, validation, confidence, and replay-visible reason.

## Design Decision

Use LLMs as **advisors, profile writers, memory narrators, and strategic commanders**, not as bodies.

Do:

- Let LLMs change utility weights, doctrine tags, squad priorities, dialogue lines, deception plans, route preferences, and training goals.
- Let LLMs summarize combat history into memories and update named actor personalities.
- Let LLMs generate mod/workbench suggestions, AI profile drafts, mission variants, and post-mission commander notes.
- Let LLMs run on cloud providers or local providers through a shared adapter.
- Make the LLM feature optional and fully degradable to local AI.

Do not:

- Do not use an LLM to aim, dodge, jump, fire, or path per frame.
- Do not stream raw game state every tick to a model.
- Do not let an LLM emit arbitrary executable code into an active campaign.
- Do not require an API key for the core game to work.
- Do not hide unfair perfect information inside LLM prompts.
- Do not make replay, E2E, or AI-H tests depend on a live paid model.

## Relationship To Existing Vault Decisions

| Existing source | Relevant current stance | Hybrid LLM impact |
|---|---|---|
| [[decisions/dr-008-ai-architecture]] | Hybrid jobs + utility + scripted hooks; local AI remains open until harness proof. | Keep the DR-008 local stack as the base. Add the LLM as an optional commander/personality layer above it. |
| [[decisions/dr-022-ai-humanlike-bar]] | AI must satisfy intent, perception, doctrine, mistakes, recovery, strategic adaptation, replay proof, and fairness. | LLMs directly help doctrine, intent, personality, adaptation, and debriefs. They are risky for replay proof and fairness unless sandboxed. |
| [[spec/ai-control-observability-layer]] | `cx-control`, `cxctl`, structured observations, semantic actions, UI tree, local bot API. | Reuse this as the observation/action boundary for LLM agents, headless eval, and future bot authors. |
| [[spec/replay-recorder-slice-a]] | Replay events and snapshots are required early. | Every prompt, compressed observation, proposal, validation result, accepted patch, rejection, and memory write needs replay/run-bundle evidence. |
| [[spec/native-implementation-backlog]] | M6 local AI currently has no LLM runtime dependency. | Preserve that anti-scope. Add M6.5 after M6 instead of polluting M6. |
| [[spec/modding-model]] | Modding data and script capability need validation/provenance. | LLM-generated profiles are mod data, not privileged code. They must pass the same schema/workbench validation. |
| [[spec/backend-networking]] | Local-first service spine; MMO-ready posture later. | LLM services should be local-first and opt-in. Multiplayer later should run authoritative LLM cognition server-side or as non-authoritative flavor. |

## Target Architecture

```text
Game sim at 60/120 Hz
  |
  | emits events, observations, blackboard deltas
  v
Observation Compressor
  - filters by actor/faction/mission scope
  - converts events into compact facts
  - enforces fog-of-war and fairness
  - writes replay-visible prompt input records
  |
  v
Mind Task Queue
  - priority, budget, TTL, cancellation
  - provider routing: mock/local/cloud
  - model routing: small/fast vs deep/slow
  |
  v
LLM Provider Adapter
  - OpenAI Responses API
  - Anthropic Messages API
  - local OpenAI-compatible server via vLLM/llama.cpp
  - Ollama local API
  - deterministic mock provider for tests
  |
  v
Strict Structured Output
  - AiMindProposal JSON schema
  - no arbitrary code
  - no direct low-level actions
  |
  v
Proposal Validator
  - schema validation
  - TTL/staleness check
  - capability and fog-of-war check
  - cost/latency/abuse gate
  - replay event: accepted/rejected plus reason
  |
  v
Policy Compiler
  - doctrine patch -> utility weights
  - squad goal -> commander blackboard
  - personality update -> actor profile
  - memory write -> campaign memory
  - dialogue -> captioned radio event
  |
  v
Local AI Executor
  - behavior tree / utility / GOAP / jobs / navigation
  - continues without waiting
```

## Data Contracts

The LLM layer is data-driven from the start. Schemas live inside the existing `cx-ai` crate under a `mind` submodule (`cx-ai::mind::schema`) plus generated JSON Schemas under `corefall-game/crates/cx-ai/schemas/mind/v<N>/` until the project stabilizes; they are NOT in a separate `cx-ai-mind-schema` crate. Provider adapters live inside `cx-ai::mind::provider`, behind cargo features (`mind-openai`, `mind-anthropic`, `mind-ollama`, `mind-openai-compatible`). The deterministic mock provider is always built. Test scenarios live in `tests/` and content packs live in `content/`, consistent with the workspace layout pinned in [[spec/prototype-roadmap]].

### `MindObservationFrame`

Compact, fairness-filtered state for one mind task.

| Field | Type | Notes |
|---|---|---|
| `schema_version` | string | Must be present in all network/logged envelopes. |
| `run_id` | string | Links to run bundle. |
| `sim_tick` | integer | The source tick for staleness. |
| `scope` | enum | `actor`, `squad`, `faction`, `mission_director`, `post_mission`. |
| `visible_facts` | array | Only facts this AI should know. No cheating unless test mode marks it. |
| `recent_events` | array | Summarized replay events, not raw unbounded logs. |
| `orders` | array | Current command structure. |
| `resources` | object | Ammo, health, energy, base power, cooldowns, deploy budget. |
| `threats` | array | Known/estimated threats with uncertainty labels. |
| `terrain_affordances` | array | Breachable, unstable, covered, climbable, flooded, burning. |
| `actor_profiles` | array | Roles, traits, injuries, equipment state, trust/fatigue. |
| `constraints` | array | Safety, objective, rules, scenario author constraints. |

### `MindTask`

The queued request sent to a provider.

| Field | Type | Notes |
|---|---|---|
| `task_id` | string | Stable ID for replay and cancellation. |
| `kind` | enum | `doctrine_patch`, `squad_plan`, `dialogue`, `memory_extract`, `enemy_adaptation`, `debrief`, `profile_generation`. |
| `priority` | integer | Queue priority; reflex AI never waits on it. |
| `deadline_ms` | integer | Worker should cancel or mark stale after this. |
| `max_cost_usd` | number | Budget gate per task. |
| `provider_policy` | object | Which model class is allowed. |
| `observation` | `MindObservationFrame` | Compressed input. |
| `output_schema` | string | Required schema ID. |

### `AiMindProposal`

The only kind of live gameplay output an LLM may produce.

```json
{
  "schema_version": "mind.proposal.v1",
  "task_id": "mind_00042",
  "sim_tick_observed": 18000,
  "valid_until_tick": 19500,
  "scope": "squad",
  "confidence": 0.78,
  "summary": "Enemy has overcommitted to the front door. Breach lower wall and send shield unit first.",
  "intent_label": "flank_via_breach",
  "orders": [
    {
      "actor_ref": "squad.role.shield",
      "goal": "advance_to_cover",
      "target_hint": "lower_breach_left",
      "reason": "absorbs initial fire while engineer opens a path"
    }
  ],
  "doctrine_patch": {
    "duration": "until_objective_or_45s",
    "utility_weight_changes": {
      "prefer_breachable_material": 0.25,
      "avoid_main_door": 0.35,
      "rescue_downed_actor": 0.10
    },
    "risk_posture": "aggressive_but_extract_if_two_down"
  },
  "dialogue": [
    {
      "speaker_ref": "commander",
      "caption": "Front door is a trap. Shield takes point, engineer opens the lower wall.",
      "tone": "calm_urgent"
    }
  ],
  "memory_writes": [
    {
      "subject": "player_tactic",
      "fact": "Player tends to funnel enemies through visible doors before breaching from below.",
      "confidence": 0.64
    }
  ],
  "risks": [
    "Engineer may be exposed during breach."
  ]
}
```

### `MindValidationResult`

Every proposal needs an explicit accept/reject result.

| Field | Type | Notes |
|---|---|---|
| `task_id` | string | Matches proposal. |
| `accepted` | boolean | False if stale, invalid, impossible, unfair, over budget, or unsafe. |
| `reasons` | array | Replay-visible reasons. |
| `applied_patch_ids` | array | Links to local AI blackboard/doctrine changes. |
| `visible_to_player` | boolean | True if a reason label, radio line, UI order, or replay annotation should show. |

### `MindMemoryRecord`

Structured memory write produced by accepted proposals or post-mission reflection.

| Field | Type | Notes |
|---|---|---|
| `memory_id` | string | Stable id (`<run_id>:mem:<seq>`). |
| `subject` | enum | `actor`, `squad`, `faction`, `player_tactic`, `world_event`. |
| `subject_ref` | string | Stable ref to the subject (actor id, squad id, faction id, scenario id). |
| `claim` | string | The remembered claim, free-text but bounded length. |
| `evidence_event_ids` | array | Ids of events in the run bundle that support this memory. |
| `confidence` | number | `0.0..1.0`; below 0.5 is "rumor", above 0.8 is "established". |
| `expiry` | enum | `transient` (within mission), `campaign`, `permanent`. |
| `visibility` | enum | `internal`, `dev_debug`, `player_inspectable`. |
| `source_task_id` | string | The `MindTask` that produced this memory (for audit). |
| `created_tick` | integer | Sim tick when the memory was created. |

### `MindProviderConfig`

Workspace-level config of available providers, routing, and budgets. Loaded from `content/config/mind.ron` (or an override path) and validated at startup.

| Field | Type | Notes |
|---|---|---|
| `enabled` | boolean | Master switch. Default `false`. |
| `default_mode` | enum | `mock`, `local`, `cloud`. |
| `redact_prompts_in_run_bundles` | boolean | Default `true`; never write prompts containing player-identifiable data into bundles. |
| `max_run_cost_usd` | number | Hard cap per run for live cloud calls. |
| `max_parallel_tasks` | integer | Bound on concurrent in-flight tasks. |
| `providers` | array of `Provider` | See `## Example Provider Config`. |
| `routes` | array of `Route` | Maps task `kind` to preferred + fallback providers, deadline, cost cap. |
| `language` | string | BCP-47 language tag for generated dialogue/captions. **Default `en` for v1; localization is open per [[spec/prototype-roadmap]] anti-goals.** |
| `caption_required` | boolean | Default `true`; every dialogue line must produce a caption per T-AUDIO + T-ACCESSIBILITY. |
| `cost_budgets` | object | Per-environment defaults (see Cost Budgets below). |

#### Cost Budgets (Defaults, Tunable)

| Environment | `max_run_cost_usd` | `max_parallel_tasks` | Notes |
|---|---|---|---|
| CI / mock | 0.00 | 0 | Mock provider only; no live calls. |
| Dev iteration | 0.10 | 2 | Local + small cloud as needed. |
| M6.5 lab live runs | 0.25 | 2 | Manual research only; never required for CI. |
| Player default | 0.00 (off) | 0 | Disabled until cost/quality/privacy proven. |
| Player power-user opt-in | 0.50 | 4 | Requires explicit settings opt-in. |

## Provider Strategy

Keep model routing data-driven. Model names change, access tiers change, and local hardware varies.

| Provider class | Best use | Avoid |
|---|---|---|
| Deterministic mock provider | Tests, replay, CI, schema validation, cost-free development. | Judging final AI quality. |
| Local small LLM through Ollama / llama.cpp / vLLM | Offline mode, privacy, modder experiments, cheap dialogue/profile drafting. | High-stakes tactical decisions unless quality is proven. |
| Fast cloud model such as Claude Haiku class or OpenAI mini/nano class | Cheap classification, memory extraction, intent labels, short radio variants, profile tags. | Deep campaign strategy. |
| Mid/high cloud model such as Claude Sonnet class or GPT-5.x class | Squad plan proposals, doctrine patches, mission director decisions, "why" labels. | Frame-critical action. |
| Deep model such as Opus class or high-reasoning GPT-5.5 | Between-mission reflection, enemy commander campaign adaptation, generating new doctrine packs, debugging AI transcripts. | Any live moment where latency or cost breaks play. |
| Realtime speech/voice models | Natural radio chatter, accessibility captions, voice command later. | Tactical authority unless converted through the same validator. |

### Current API Notes From 2026-05-05 Research

| Vendor | Useful current facts | Roadmap impact |
|---|---|---|
| OpenAI | GPT-5.5 docs emphasize the Responses API, structured outputs, tool calling, prompt caching, and adjustable reasoning effort. Model catalog pages list very large context for GPT-5.5-class models. | Use Responses API for cloud OpenAI provider. Use structured outputs for `AiMindProposal`. Use low effort for live suggestions, high/xhigh only for between-mission thinking. |
| Anthropic | Claude model docs position Haiku as fastest, Sonnet as best speed/intelligence, and Opus as most capable. Messages API supports tools and structured output patterns. | Use Haiku-class for cheap labels/dialogue, Sonnet-class for live strategy, Opus-class for deep campaign reflection. |
| Local providers | Ollama exposes local generate/chat APIs. vLLM and llama.cpp can provide OpenAI-compatible local HTTP servers. | The adapter should support both direct Ollama and OpenAI-compatible local endpoints. Local mode protects the baseline from cloud cost and availability. |

## Roadmap Extension

Do not make this part of M0-M6 baseline AI. Add it as a side track and a bridge milestone.

| Roadmap point | Add / change | Acceptance bar |
|---|---|---|
| M0 - Native bootstrap | Add only config stubs and feature flags: `ai_mind.enabled=false`, provider config format, secret redaction policy. | Game starts with no API keys. Config validation rejects secrets in checked fixtures. |
| T-CONTROL | Ensure the control/observation stream can also feed LLM observation frames. | `cxctl observe --mind-frame squad_alpha` returns compact JSON without screenshots. |
| M3 - Replay/Event | Add mind event families: `mind.task_created`, `mind.prompt_recorded`, `mind.response_received`, `mind.proposal_validated`, `mind.patch_applied`, `mind.patch_rejected`, `mind.memory_written`. | Replay viewer can show why a mind proposal affected behavior. |
| M6 - Local AI | Keep local-only. Add hook points for doctrine patches and blackboard goals, but no live LLM dependency. | AI-H tests pass with mock/no provider. |
| M6.5 - LLM Mind Lab | New milestone. Build provider abstraction, schemas, mock provider, observation compressor, validator, policy compiler, cost/latency telemetry, and one visible doctrine patch in a controlled breach lab. | Local AI continues acting if provider sleeps, fails, times out, or returns invalid output. |
| M7 - Breach Contract | Optional LLM commander can generate a squad plan or enemy adaptation before/during mission. | Same mission remains playable and testable with `ai_mind.enabled=false`. |
| M8 - Editor/Mods | Add mind profile editor, prompt-pack/workbench validation, doctrine presets, and local provider test panel. | Modded mind packs are data files with schema validation and provenance. |
| M9 - Headless/Eval | Run batch AI-H/MIND tests across mock, local, and configured cloud providers. | Headless report compares latency, cost, acceptance rate, stale rate, win/loss impact, and humanlike score. |
| M10+ - Multiplayer/Backend | Keep authoritative AI server-side. Clients receive resulting orders/events, not privileged prompts. | LLM never gives a client hidden information or non-authoritative control. |
| Post-MVP | Campaign memory, rival commanders, named actor growth, AI training lab, player-visible "show me why" explanations. | Player can inspect AI reasoning without seeing unfair hidden facts. |

## New Side Track: T-LLM - Async LLM Mind Layer

| ID | Task | Owner crates/modules | Done when |
|---|---|---|---|
| T-LLM-001 | Define schemas for `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, and `MindProviderConfig`. | `cx-ai::mind::schema` (with generated JSON Schemas under `corefall-game/crates/cx-ai/schemas/mind/v1/`) | JSON/RON schemas exist, examples validate, bad examples fail. |
| T-LLM-002 | Add deterministic mock provider. | `cx-ai::mind::provider::mock` | Tests can inject canned responses, malformed responses, timeout, cost overflow, and stale response. |
| T-LLM-003 | Add provider adapter trait and routing config. | `cx-ai::mind::provider` (cargo features `mind-openai`, `mind-anthropic`, `mind-ollama`, `mind-openai-compatible`) | OpenAI-compatible, Anthropic, Ollama, and mock adapters share one interface. Live cloud adapters are feature-gated. |
| T-LLM-004 | Build observation compressor. | `cx-ai::mind::compressor`, `cx-control`, `cx-replay` | Produces compact, fog-of-war-filtered mind frames from event stream and blackboard. |
| T-LLM-005 | Build proposal validator. | `cx-ai::mind::validator` | Rejects stale, invalid, impossible, unfair, over-budget, hidden-info, and capability-violating proposals. |
| T-LLM-006 | Build policy compiler. | `cx-ai::mind::policy` | Accepted proposal can update utility weights, commander blackboard, actor doctrine tags, dialogue queue, and memory. |
| T-LLM-007 | Add replay/run-bundle events. | `cx-replay`, `cx-ai::mind::events`, `tools/run_bundle_check.py` (event-category extension) | Every mind task and outcome can be audited without exposing secrets. |
| T-LLM-008 | Add latency/cost/rate budget dashboard. | `cx-ui`, `cx-tools-editor` (workbench panel) | Dev can see task count, stale rate, provider failures, estimated cost, model routing, and accept/reject reasons. |
| T-LLM-009 | Add profile/workbench authoring. | `cx-tools-editor`, `content/mind/profiles/` | Designer can create a cautious medic, aggressive breacher, shield commander, rival raider, or base defense mind pack. |
| T-LLM-010 | Add AI-H/MIND eval scenarios. | `tests/`, `cx-headless`, `cx-bench` | Headless test suite proves local AI independence, proposal usefulness, fairness, and visible "why" labels. |

## New Milestone: M6.5 - LLM Mind Lab

### Purpose

Prove the hybrid idea in the smallest playable environment without making the base AI dependent on an external model.

### Required Scenario

"Micro Breach Mind Lab": one squad, one destructible breach path, one defended objective, one reactive enemy, one rescue/stakes condition, one commander mind.

The same scenario runs in three modes:

| Mode | Provider | Expected use |
|---|---|---|
| `mind_off` | None | Baseline local AI. |
| `mind_mock` | Deterministic mock | CI and replay proof. |
| `mind_live_optional` | Configured cloud/local provider | Manual research only; never required for CI. |

### Acceptance Tests

| Test ID | Test | Pass condition |
|---|---|---|
| MIND-001 | No-provider baseline | Scenario starts, plays, and AI-H tests run with `ai_mind.enabled=false`. |
| MIND-002 | Nonblocking timeout | Provider sleeps for 30 seconds; actors still fight, retreat, reload, rescue, and complete/fail locally. |
| MIND-003 | Malformed response | Invalid JSON is rejected; replay records rejection; game continues. |
| MIND-004 | Stale response | Response arriving after `valid_until_tick` is rejected or downgraded to post-hoc memory. |
| MIND-005 | Doctrine patch visible | Accepted proposal changes utility weights and produces visible reason labels. |
| MIND-006 | Fog-of-war fairness | Mind prompt excludes hidden enemy state unless scenario explicitly marks omniscient debug mode. |
| MIND-007 | Memory write | Post-encounter memory writes are visible in run bundle and later prompt context. |
| MIND-008 | Replay audit | Replay viewer shows mind task, compressed input hash, provider class, proposal summary, validator result, and applied patch IDs. |
| MIND-009 | Cost cap | Provider tasks stop when per-run cost or request cap is reached; local AI continues. |
| MIND-010 | Humanlike score delta | AI-H report compares local-only versus mind-enabled runs for intent, doctrine, recovery, strategic adaptation, and fairness. |

## LLM Memory Model

Use memory carefully. Humanlike AI needs memory, but bad memory creates nonsense, cheating, and expensive prompts.

| Memory type | Stored where | Used for | Retention |
|---|---|---|---|
| Tactical short-term | Blackboard / event ring buffer | Current fight context. | Seconds to minutes. |
| Actor episodic | Actor profile | Injuries, heroics, panic, saved ally, favorite tool, grudges. | Campaign. |
| Squad doctrine | Squad profile | Preferred breach style, rescue risk, stealth/aggression, fallback rules. | Campaign, editable. |
| Faction/rival memory | Campaign state | Enemy adapts to player tendencies and base defenses. | Campaign. |
| Designer prompt pack | Content package | Voice, doctrine, constraints, forbidden behavior, author notes. | Content version. |
| Replay transcript | Run bundle | Debugging and evidence. | Permanent artifact, with secret redaction. |

Memory writes must be structured. Avoid free-form "everything that happened" blobs. A good memory record has subject, claim, evidence event IDs, confidence, expiry, visibility, and whether the player can inspect it.

## Prompting And Output Rules

LLM prompts should be short, structured, and scenario-specific.

Prompt content should include:

- The current role: "You are a squad commander mind", "You are enemy faction strategist", "You are named actor personality narrator".
- Hard limits: no direct aim/fire/jump commands; propose doctrine/goals only; respect fog-of-war; output schema only.
- Current compact facts: visible threats, objectives, resources, body/equipment damage, terrain affordances, active orders.
- Player promise: tactical pulp sci-fi disaster sandbox, readable under pressure, fair but surprising.
- Desired output schema and examples.

Prompt content should not include:

- Raw frame dumps.
- Hidden enemy facts unless the AI is allowed to know them.
- API keys, file paths containing secrets, or user-identifying data.
- Full replay logs when a compact summary will do.
- Instructions to write executable code into live gameplay.

## Pros And Cons

| Benefit | Why it matters | Mitigation if risky |
|---|---|---|
| Better humanlike intent | The AI can explain "why" and act like a teammate/rival instead of a state machine. | Convert to reason labels, not raw model text. |
| Strategic adaptation | Enemy commanders can remember what beat them and change future loadouts/routes. | Validate against fairness and scenario constraints. |
| Personality | Named actors and factions can feel distinct across missions. | Store traits as data; do not depend on a live model each frame. |
| Mod authoring leverage | LLMs can draft profiles, dialogue, and doctrine packs. | Workbench validation and provenance required. |
| Replay/debrief richness | Runs become stories with useful AI audit trails. | Separate flavor text from authoritative events. |
| Cost and latency | Cloud calls are slow and potentially expensive. | Async only, budget caps, local/mock modes, prompt caching where supported. |
| Nondeterminism | Same prompt can produce different plans. | Never use live LLM in deterministic CI; record proposal outputs in run bundles. |
| Hallucination | Model may invent facts or invalid actions. | Strict schema plus validator plus hidden-info gate. |
| Fairness | Model might receive omniscient state. | Observation compressor enforces visibility. |
| Safety/privacy | Prompts may leak secrets or player data. | Redaction, opt-in, local mode, no secrets in replays. |

## Implementation Notes

### Runtime Concurrency

- Run LLM tasks on worker threads or async tasks outside the simulation tick.
- Every task has a deadline, cancellation token, max prompt size, max output size, and cost cap.
- Local AI reads only accepted policy patches from a thread-safe queue.
- Policies have TTLs. Stale policies decay or revert.
- If a provider fails, mark the provider degraded and continue.

### Determinism And Replay

Live LLM calls are nondeterministic by default. The game can still be replayable if it records the LLM output as an event:

- Deterministic replay mode reuses recorded `AiMindProposal` and validation results.
- Fresh simulation mode may call providers again and can diverge.
- CI uses mock provider only.
- Run bundles include provider class, model ID, config hash, prompt hash, response hash, latency, token/cost estimates, and acceptance result.
- Secret values are never written to run bundles.

### Security And Modding

LLM-generated content must be treated like untrusted mod content:

- Data schemas only.
- No arbitrary file IO.
- No arbitrary network access.
- No runtime code generation into live campaigns.
- Any future script generation goes through offline workbench review, static validation, provenance logging, and capability gates.

### UX

The player should feel the AI is smarter, not see a chatbot bolted onto combat.

Visible surfaces:

- Radio intent labels: "Shield taking point", "Engineer opening lower wall", "Medic refusing push, low supplies".
- Commander plan card before launch.
- Post-mission debrief: "Enemy adapted to repeated roof breaches".
- Replay annotations for AI decisions.
- Bot personality panel: doctrine, trust, fears, recent memories, preferred equipment.
- Accessibility captions for all generated or selected lines.

Hidden surfaces:

- Prompt text and model settings stay in dev/debug UI unless the player opts into advanced inspection.
- Provider cost and errors are dev/debug surfaces.

## Example Provider Config

```ron
AiMindConfig(
  enabled: false,
  default_mode: "mock",
  redact_prompts_in_run_bundles: true,
  max_run_cost_usd: 0.25,
  max_parallel_tasks: 2,
  providers: [
    Provider(
      id: "mock",
      kind: "deterministic_mock",
      enabled: true,
    ),
    Provider(
      id: "local_ollama",
      kind: "ollama",
      base_url: "http://127.0.0.1:11434",
      model: "data-driven-model-id",
      enabled: false,
    ),
    Provider(
      id: "local_openai_compatible",
      kind: "openai_compatible",
      base_url: "http://127.0.0.1:8000/v1",
      model: "data-driven-model-id",
      enabled: false,
    ),
    Provider(
      id: "openai_cloud",
      kind: "openai_responses",
      model: "gpt-5.5",
      api_key_env: "OPENAI_API_KEY",
      enabled: false,
    ),
    Provider(
      id: "anthropic_cloud",
      kind: "anthropic_messages",
      model: "claude-sonnet-class-data-driven-id",
      api_key_env: "ANTHROPIC_API_KEY",
      enabled: false,
    ),
  ],
  routes: [
    Route(kind: "memory_extract", preferred_provider: "local_ollama", fallback_provider: "mock", deadline_ms: 2500),
    Route(kind: "squad_plan", preferred_provider: "anthropic_cloud", fallback_provider: "mock", deadline_ms: 8000),
    Route(kind: "debrief", preferred_provider: "openai_cloud", fallback_provider: "local_openai_compatible", deadline_ms: 30000),
  ],
)
```

## Source Analysis

> [!note] Source posture
> Research date: 2026-05-05. Model catalogs and API features change. Keep model IDs in config, not hard-coded gameplay logic.

| ID | Source | What matters | Impact on this plan |
|---:|---|---|---|
| 1 | [OpenAI GPT-5.5 migration/latest model guide](https://developers.openai.com/api/docs/guides/latest-model) | Current OpenAI guidance emphasizes Responses API, reasoning effort, structured outputs, prompt caching, tools, and agent patterns. | Use Responses API for OpenAI provider and route expensive reasoning to background tasks. |
| 2 | [OpenAI model catalog](https://developers.openai.com/api/docs/models) | Model capabilities and context windows are provider/catalog facts, not stable assumptions. | Use data-driven provider configs and model classes. |
| 3 | [OpenAI Responses API overview](https://developers.openai.com/api/reference/responses/overview/) | Responses API supports stateful interactions, tool/function use, and structured output workflows. | Good fit for `AiMindProposal` and tool-like mind tasks. |
| 4 | [OpenAI Structured Outputs](https://developers.openai.com/api/docs/guides/structured-outputs) | Structured outputs help enforce schema-valid responses. | Required for provider adapters where available. |
| 5 | [OpenAI Realtime guide](https://developers.openai.com/api/docs/guides/realtime) | Realtime APIs are useful for low-latency speech/audio experiences. | Consider later for radio voice/captions, not tactical authority. |
| 6 | [Anthropic model overview](https://docs.anthropic.com/en/docs/about-claude/models/overview) | Anthropic positions Haiku/Sonnet/Opus by speed/capability tradeoff. | Use route classes: fast labels, live planning, deep reflection. |
| 7 | [Anthropic Messages API](https://docs.anthropic.com/en/api/messages) | Anthropic's primary text API supports model choice, messages, tools, and output configuration. | Build Anthropic adapter behind same provider trait. |
| 8 | [Anthropic tool use docs](https://docs.anthropic.com/en/docs/build-with-claude/tool-use) | Tool use allows a model to request external operations through explicit schemas. | Treat proposal validation and memory tools as explicit, gated surfaces. |
| 9 | [Anthropic structured outputs](https://docs.anthropic.com/en/docs/build-with-claude/structured-outputs) | Structured generation patterns reduce parse failures. | Use for `AiMindProposal` where available. |
| 10 | [Anthropic prompt caching](https://docs.anthropic.com/en/docs/build-with-claude/prompt-caching) | Reusing static context can reduce cost/latency. | Cache stable world rules, profile packs, and schema instructions. |
| 11 | [Ollama API introduction](https://docs.ollama.com/api/introduction) | Ollama provides local generation/chat APIs. | Add direct local provider mode. |
| 12 | [Ollama API docs](https://github.com/ollama/ollama/blob/main/docs/api.md) | API shapes are simple enough for local dev and modder experimentation. | Useful for offline mind lab testing. |
| 13 | [vLLM OpenAI-compatible server](https://docs.vllm.ai/en/latest/serving/openai_compatible_server.html) | vLLM can expose local/hosted models behind an OpenAI-compatible API. | One adapter can support local high-throughput inference. |
| 14 | [vLLM offline inference docs](https://docs.vllm.ai/en/latest/serving/offline_inference/) | Offline/batch inference supports eval and bulk analysis. | Good for headless AI-H/MIND batch runs. |
| 15 | [llama.cpp repository](https://github.com/ggml-org/llama.cpp) | llama.cpp provides local inference and OpenAI-compatible server modes. | Another local provider option for players/modders. |
| 16 | [llama.cpp OpenAI compatibility discussion](https://github.com/ggml-org/llama.cpp/discussions/3683) | OpenAI-compatible endpoints are common in local LLM stacks. | Design provider trait around a shared chat/completions-ish abstraction plus vendor-specific features. |
| 17 | [Generative Agents paper](https://arxiv.org/abs/2304.03442) | LLM agents can use memory, reflection, planning, and social behavior loops. | Adopt structured memory/reflection, but keep it off the reflex path. |
| 18 | [Generative Agents ACM publication](https://dl.acm.org/doi/10.1145/3586183.3606763) | Academic version grounds the architecture in observable agent behavior. | Supports actor/squad memory and reflection surfaces. |
| 19 | [Voyager paper](https://arxiv.org/abs/2305.16291) | LLM agent improves through skill acquisition and code-like behavior libraries in Minecraft. | Good inspiration for offline profile/skill generation, but live arbitrary code is not acceptable. |
| 20 | [Voyager project page](https://voyager.minedojo.org/) | Shows curriculum, skill library, and iterative prompting for embodied agents. | Use "skill/profile library" idea for workbench-generated doctrine packs. |
| 21 | [Voyager GitHub](https://github.com/MineDojo/Voyager) | Demonstrates practical LLM agent loop artifacts. | Useful reference for eval traces and skill-library structure. |
| 22 | [A Survey on LLM-Based Game Agents](https://arxiv.org/html/2404.02039v4) | Broad survey of LLM game agents, limitations, planning, memory, and evaluation. | Confirms need for hybrid architecture and eval harness. |
| 23 | [ACM review of LLM agents in games](https://dl.acm.org/doi/10.1145/3783862.3783876) | Game-agent LLM work is active but not a solved production reflex-control problem. | Avoid betting the core AI on LLMs. |
| 24 | [The Mind and the Body: Hybrid Architecture for Believable Game AI](https://aaltodoc.aalto.fi/server/api/core/bitstreams/59c4055a-625f-4ff0-bb9f-09897fcc5851/content) | Separates deliberative mind behavior from embodied reactive action. | Direct architectural fit for local body plus async mind. |
| 25 | [Game AI Planning: GOAP, Utility, and Behavior Trees](https://tonogameconsultants.com/game-ai-planning/) | Summarizes proven non-LLM planning techniques used in games. | Keep local AI as BT/utility/GOAP/jobs. |
| 26 | [Game AI Pro: Building Utility Decisions Into Your Existing Behavior Tree](http://www.gameaipro.com/GameAIPro/GameAIPro_Chapter10_Building_Utility_Decisions_into_Your_Existing_Behavior_Tree.pdf) | Utility and behavior trees compose well. | LLM proposals should patch utility weights and goals, not replace the tree. |
| 27 | [GOBT paper](https://www.jmis.org/archive/view_article_pubreader?pid=jmis-10-4-321) | Goal-oriented behavior trees combine planning intent with BT control. | Good local executor target for LLM-generated goals. |
| 28 | [PORTAL: Agents Play Thousands of 3D Video Games](https://arxiv.org/html/2503.13356v1) | Large-scale game-agent evaluation emphasizes generalization and benchmarked behavior. | Build batch MIND eval, not anecdotal "seems smart" claims. |
| 29 | [PORTAL PDF](https://arxiv.org/pdf/2503.13356) | PDF source for benchmark details. | Use for future benchmark design and scoring ideas. |
| 30 | [Optimus-2 CVPR 2025 paper](https://openaccess.thecvf.com/content/CVPR2025/papers/Li_Optimus-2_Multimodal_Minecraft_Agent_with_Goal-Observation-Action_Conditioned_Policy_CVPR_2025_paper.pdf) | Embodied agents need goal-observation-action structure. | Use explicit observation frames and action/proposal schemas. |
| 31 | [STORY2GAME paper](https://arxiv.org/pdf/2505.03547) | LLMs can assist game content generation from story constraints. | Use in editor/modding, not live action. |
| 32 | [LLM-NPC psychological/cognitive load study](https://www.arxiv.org/pdf/2604.10107) | LLM NPCs affect player perception and cognitive load. | Keep generated chatter concise, captioned, and tactically useful. |
| 33 | [LLM reasoner plus automated planner](https://arxiv.org/html/2501.10106v1) | Pairing LLM reasoning with symbolic/planner execution can reduce raw LLM action errors. | Proposal validator plus local policy compiler follows this pattern. |
| 34 | [LLM behavior agent with natural language personality control](https://etasr.com/index.php/ETASR/article/view/12631) | Personality conditioning can change agent behavior. | Use mind profiles/doctrine packs for named actors and factions. |
| 35 | [KTH thesis on LLMs for game NPCs](https://kth.diva-portal.org/smash/get/diva2:1938971/FULLTEXT01.pdf) | Practical NPC LLM work surfaces integration, latency, and design tradeoffs. | Supports async/background use and player-facing constraints. |
| 36 | [Modular hybrid Slay the Spire LLM agents](https://openreview.net/pdf/521037328af4df7687b86343bc0ed0bddcc441fb.pdf) | Modular/hybrid agents can combine domain code with LLM reasoning. | Supports keeping tactical mechanics local and using LLM for higher-order choices. |
| 37 | [AI-powered NPCs in VR latency paper](https://arxiv.org/html/2507.10469v1) | Latency matters for embodied player-facing AI. | Never block combat on model output. |
| 38 | [Serious game AI dialogue architecture](https://www.mdpi.com/2227-9709/13/1/16) | LLM dialogue systems need architecture, constraints, and evaluation. | Use schema, captioning, and strict role boundaries for radio/debrief text. |

## Best Next Prompt For An Implementing Agent

Use this after M6 local AI hooks exist:

```text
Implement M6.5 - LLM Mind Lab from cortext_command_vault/spec/hybrid-llm-ai-plan.md.

Goal:
Build the async LLM mind layer without making local AI depend on a live model.

Context:
- Read cortext_command_vault/spec/prototype-roadmap.md
- Read cortext_command_vault/spec/native-implementation-backlog.md
- Read cortext_command_vault/spec/ai-control-observability-layer.md
- Read cortext_command_vault/decisions/dr-008-ai-architecture.md
- Read cortext_command_vault/decisions/dr-022-ai-humanlike-bar.md
- Read cortext_command_vault/spec/hybrid-llm-ai-plan.md

Scope:
- Add MindObservationFrame, MindTask, AiMindProposal, MindValidationResult, MindMemoryRecord, and MindProviderConfig schemas.
- Add deterministic mock provider first.
- Add provider trait with feature-gated cloud/local adapters stubbed behind config.
- Add observation compressor using replay/control events, with fog-of-war filtering.
- Add proposal validator and policy compiler for doctrine patches.
- Add MIND-001..MIND-010 tests and run-bundle events.
- Do not require API keys or live cloud calls for tests.

Done when:
- MIND-001..MIND-010 pass with mock provider.
- Local AI keeps acting during timeout/failure.
- Replay/run bundle shows prompt/proposal/validation/apply/reject/memory events.
- One Micro Breach scenario visibly changes local utility behavior after an accepted doctrine patch.
- Final report lists changed files, tests run, remaining gaps, and any provider adapters left as stubs.
```

## Open Questions

| Question | Default answer for now |
|---|---|
| Should this be enabled for players by default? | No. Default off until cost, privacy, quality, and offline fallback are proven. |
| Should live cloud LLMs run during combat? | Optional, but async only and never required for local reflexes. |
| Should local LLMs be supported? | Yes. Support Ollama and OpenAI-compatible local endpoints when practical. |
| Can LLMs change scripts? | Not live. They can draft validated profile/script data in the workbench. |
| Can multiplayer use LLM minds? | Later. Server-authoritative only, with prompts/results hidden from clients except visible outcomes. |
| Can scenario authors use LLM profiles? | Yes, through data schemas and workbench validation. |

## Final Stance

This should be in the roadmap because it supports the north star: AI teammates and rivals that feel human, remember, explain themselves, adapt, and create stories players want to replay. It should not be used as the core action controller. The best version is a layered AI where classic game AI owns the body and the LLM owns slow cognition.

The practical sequence is:

1. Build local AI well enough to pass AI-H without LLMs.
2. Add replay/control observation hooks.
3. Add M6.5 with mock provider, schemas, validation, and one doctrine-patch proof.
4. Add optional local/cloud providers behind feature flags.
5. Use LLMs first for debriefs, profiles, "show me why", and enemy commander adaptation.
6. Only after those are stable, let LLM mind proposals influence live squad doctrine.
