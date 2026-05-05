---
type: decision
id: DR-032
status: closed-direction
priority: P0
closed_at: 2026-05-05
revisit_trigger: "Local AI proves materially insufficient for the DR-022 humanlike bar even after M6 closes; or LLM cost/latency/fairness/determinism gates make M6.5 unachievable; or a player-facing LLM dependency becomes unavoidable for competitive viability."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/hybrid-llm-ai-plan|hybrid LLM AI plan]] · [[spec/prototype-roadmap|native build roadmap]] · [[decisions/dr-008-ai-architecture|DR-008]] · [[decisions/dr-022-ai-humanlike-bar|DR-022]]

# DR-032: Hybrid LLM AI Direction

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-05)
> The shipped game uses **hybrid AI**: classic local game AI (utility / behavior trees / GOAP / jobs / navigation / commander rules) owns the body and runs at frame speed; **LLMs run async in the background** as an optional "mind" layer that proposes doctrine patches, squad goals, personality, post-mission reflection, debriefs, memory, dialogue, and commander adaptation. **Local AI never blocks on an LLM.** **No live LLM is required for the core game, CI, or AI-H tests.**

## Decision

**Add an async LLM mind layer to the roadmap as a side track (T-LLM) and a milestone (M6.5 — LLM Mind Lab) AFTER local M6 AI baseline.** Keep DR-008 local AI as the foundation. Keep DR-022 humanlike bar testable without any LLM.

## What This Locks In

| Aspect | Commitment |
|---|---|
| Frame-critical control | Reflex (8-16 ms) and tactical (100-250 ms) decisions stay local. **No LLM in the reflex/tactical loop.** |
| LLM scope | Doctrine patches, squad plans, dialogue/captions, memory writes, post-mission debrief, enemy commander adaptation, modded profile/doctrine pack drafts. |
| LLM cadence | 2-30 s for live tasks, or between missions for reflection. Always async. |
| Local AI independence | Local AI must keep acting through provider sleep, failure, malformed output, stale response, cost cap, and disabled provider. |
| API requirement | **No API key required for the core game to work.** Default mode is `mock` (deterministic). |
| Output safety | LLM outputs are data only (`AiMindProposal` schema). No arbitrary code, no direct low-level actions, no fog-of-war bypass. |
| Determinism | Live LLM calls are nondeterministic by default; CI uses the deterministic mock provider; replay can rerun recorded `AiMindProposal`/validation results. |
| Replay/audit | Every mind task, prompt hash, response hash, validation result, accepted patch, rejection, and memory write is replay-recorded with secret redaction. |
| Provider portfolio | OpenAI Responses API, Anthropic Messages API, local Ollama, local OpenAI-compatible (vLLM/llama.cpp), deterministic mock — all behind one provider trait, feature-gated for cloud adapters. |
| Player default | Disabled. Opt-in via settings; enables `mock` first; cloud/local providers each require explicit configuration. |
| Multiplayer/MMO posture | LLM cognition runs server-authoritative; clients see resulting orders/events, never privileged prompts. |
| Modding posture | LLM-generated profiles/doctrine packs are mod data, validated by the same workbench schema/provenance pipeline as any other mod content. |
| Captioning | Every generated dialogue/radio line emits a caption per [[decisions/dr-020-audio-identity]] / T-AUDIO / T-ACCESSIBILITY. |
| Localization | English-first at v1; localization plan open (see [[spec/prototype-roadmap]] anti-goals). |

## What This Does NOT Lock

- Specific provider/model IDs (data-driven; GPT-5.5/Claude class lineage will move).
- Specific local LLM stack (Ollama vs vLLM vs llama.cpp adapter selection deferred to M6.5 implementation).
- Whether mind cognition graduates from "optional" to "default" post-launch (revisit after M6.5 + M9 evidence).
- Exact prompt-pack structure for community-authored mind packs (deferred to M8 modding work).

## What This Explicitly REJECTS

| Rejected Pattern | Why |
|---|---|
| LLM as the reflex/tactical controller | Latency, cost, hallucination, and fairness break the play loop. |
| Streaming raw game state every tick to a model | Cost and bandwidth blow up; fairness/fog-of-war is hard to enforce. |
| LLM-emitted executable code into a live campaign | Sandbox/safety nightmare; bypasses modding validation. |
| Hard dependency on a paid API for the core game | Privacy, cost, availability, and offline-play violations. |
| Hidden omniscient prompts that give the AI unfair info | Fails DR-022 fairness criterion. |
| LLM in the deterministic CI/AI-H/replay path | Nondeterminism contaminates replay validation. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Pure classic AI | Underdelivers DR-022 "most humanlike AI in the genre" promise: weak intent, weak strategic adaptation, weak personality. |
| LLM as the primary AI controller | Fails latency, cost, fairness, determinism, and offline-play constraints. |
| LLM only in modding tools (offline) | Misses live commander adaptation, debrief richness, and personality-driven dialogue. |
| Single-provider lock-in (OpenAI-only or Anthropic-only) | Player privacy, cost, and offline play would be hostage to one vendor. |

## Evidence Trail

- Project owner (2026-05-05) approved adding the async LLM mind layer to the roadmap as a P0 ROADMAP EXTENSION; assistant produced [[spec/hybrid-llm-ai-plan]] with a 38-source research table.
- DR-008 commits to hybrid jobs + utility + scripted hooks; this DR layers an async LLM advisor on top.
- DR-022 humanlike-bar criteria (intent, perception, doctrine, mistakes, recovery, strategic adaptation, replay proof, fairness) include surfaces (intent, doctrine, strategic adaptation, dialogue) where async LLM cognition adds direct value, and surfaces (perception fairness, replay proof, mistake/recovery) where strict validation is required.
- Source review (2026-05-05): The Mind and the Body (hybrid embodied/deliberative architecture); Generative Agents (memory + reflection loops); Voyager (skill libraries); modular Slay-the-Spire LLM agents (hybrid is the right call); LLM-NPC psychological-load study (concise, captioned, tactically useful); VR latency study (never block on model). See full source list in [[spec/hybrid-llm-ai-plan]].
- Provider portfolio: OpenAI Responses API + Structured Outputs (current production patterns); Anthropic Messages API + tool use + prompt caching; Ollama + vLLM + llama.cpp for local. All adapters slot behind a shared trait.

## Risks

| Risk | Mitigation |
|---|---|
| LLM cost overruns | `MindProviderConfig.max_run_cost_usd` hard cap; per-task budget gate in queue; mock-by-default in dev/CI. |
| Latency spikes | Async only; deadlines per task; local AI never waits; staleness check on response. |
| Hallucination / invalid actions | Strict `AiMindProposal` schema + validator; no live arbitrary code; bounded caption length; no direct low-level actions. |
| Fairness leak (omniscient prompt) | Observation compressor enforces fog-of-war; fairness audit in MIND-006 acceptance test. |
| Determinism contamination | Live LLM never in CI; replay reuses recorded proposals; mock provider is deterministic. |
| Privacy / secret leaks | Prompts/responses redact player-identifying data; secrets never written to run bundles; opt-in only. |
| Vendor / model deprecation | Provider adapters behind a shared trait; model IDs are data-driven; M6.5 ships against mock first. |
| Player perception of "AI is a chatbot" | Generated text surfaces only as captioned radio lines, debrief cards, replay annotations — never a free-form chat box. |
| Multiplayer cheating via prompt | Server-authoritative LLM cognition; clients never see privileged prompts/state. |

## Prototype / Validation Plan

| Test | What It Proves |
|---|---|
| MIND-001 — `ai_mind.enabled=false` | Game runs with no provider; AI-H tests pass; this is the floor. |
| MIND-002 — Provider sleeps 30 s | Local AI keeps acting; player can finish the scenario. |
| MIND-003 — Malformed response | Validator rejects; replay records rejection; game continues. |
| MIND-004 — Stale response | Late response is rejected or downgraded to post-hoc memory. |
| MIND-005 — Doctrine patch visible | Accepted proposal changes utility weights and produces visible reason labels. |
| MIND-006 — Fog-of-war fairness | Mind prompt excludes hidden enemy state unless explicit debug mode. |
| MIND-007 — Memory write | Post-encounter memory writes are visible in run bundle and feed later prompt context. |
| MIND-008 — Replay audit | Replay viewer shows mind task, prompt hash, provider class, proposal summary, validator result, applied patch ids. |
| MIND-009 — Cost cap | Provider tasks stop when run cost cap is reached; local AI continues. |
| MIND-010 — Humanlike score delta | AI-H report compares local-only vs mind-enabled runs across the 8 DR-022 criteria. |

## Revisit Trigger

- Local AI fails the DR-022 humanlike bar even after M6 closes.
- LLM cost/latency/fairness/determinism gates make M6.5 unachievable.
- A player-facing LLM dependency becomes unavoidable for competitive viability.
- A provider/regulatory shift breaks the current adapter portfolio.

## Source Trail

- [[spec/hybrid-llm-ai-plan]] — full plan with 38-source research table, schemas, provider config, M6.5 acceptance tests.
- [[decisions/dr-008-ai-architecture]] — local AI foundation.
- [[decisions/dr-022-ai-humanlike-bar]] — humanlike criteria.
- [[decisions/dr-002-replay-event-architecture]] — replay/event posture for mind events.
- [[decisions/dr-006-modding-data-model]] — mod data validation pipeline reused for LLM-authored content.
- [[decisions/dr-013-backend-service-scope]] — local-first backend posture for provider services.
- [[decisions/dr-024-native-engine-stack]] — Rust crate boundary.
- [[spec/prototype-roadmap]] — T-LLM side track and M6.5 milestone.
- [[spec/native-implementation-backlog]] — M6.5 task cards.
- [[spec/ai-control-observability-layer]] — observation/action layer reused as mind-frame source.
- [[research-log/2026-05-05-hybrid-llm-ai-direction]]
