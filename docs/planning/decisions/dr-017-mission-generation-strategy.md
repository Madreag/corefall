---
type: decision
id: DR-017
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Procedural generator fails to deliver replayable variety, or the manifest format proves too rigid for hand-authored set pieces."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/missions-and-objectives|missions spec]] · [[spec/mission-director-slice-a|mission director]] · [[decisions/dr-006-modding-data-model|DR-006 modding]] · [[decisions/dr-014-tone-player-promise|DR-014 tone]]

# DR-017: Mission Generation Strategy

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Manifest-first hybrid: hand-authored anchor missions + procedural/contract generation + first-class player-authored scenarios — all using the same typed mission manifest.

## Decision

**Manifest-first hybrid mission strategy.**

Every mission — official, generated, or player-made — uses the same typed mission manifest contract:

- Objectives.
- Teams.
- Terrain rules / material profile.
- Command-core / base state.
- Equipment capability requirements.
- Director pacing.
- Commander AI.
- Save fields.
- Replay events.
- Validation diagnostics.

### Three production tracks share the format

| Track | Purpose | v1 Scope |
|---|---|---|
| **Hand-authored anchor missions** | Quality, onboarding, story tone, set-piece moments, prove the intended fantasy. | A small number, e.g. 5-12. Cover breach, rescue, base defense, salvage, command-core uproot/avatar last stand, extraction. |
| **Procedural / contract generation** | Replayability between anchors. | Faction jobs, daily/seeded contracts, bunker breach variants, rescue, salvage raids, base defense, sabotage, survival, commander-adaptation. Deterministic seeds. Replay cards. Same-seed retry. |
| **Player-authored scenarios** | Long-tail content engine. | Editor/workbench uses the **same** manifest format and validators as the internal tools. Players build contracts, tune material profiles, set capability requirements, define objectives, test AI, export/share replay-compatible scenario packs. |

## What This Locks In

| Spec Area | Implication |
|---|---|
| Mission manifest schema | One typed schema serves engine, editor, AI, replay, mod tools, and players. See [[spec/mission-director-slice-a]]. |
| Modding | Player-authored scenarios are first-class, not a side mode. Editor parity with internal tools is required. See [[decisions/dr-006-modding-data-model]] and [[spec/modding-model]]. |
| AI | Commander AI and director must consume the same manifest fields whether the mission is authored or generated. See [[decisions/dr-008-ai-architecture]]. |
| Replay | Generated contracts must produce replay-compatible bundles, including the seed used. See [[decisions/dr-002-replay-event-architecture]]. |
| Workbench | Mission editor lives in the same workbench surface as the package builder. See [[spec/package-builder-workbench-slice-a]]. |
| First playable | A1..A7 prototype path can use the same manifest format from day one, even if only a single anchor mission exists. See [[spec/prototype-implementation-backlog-slice-a]]. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Number of anchor missions at launch | Open. Suggested 5-12. |
| Procedural generator algorithm specifics | Open. Wave-function-collapse, grammar-based, parameter-based, or hybrid all viable. |
| Daily-seed contract cadence | Open. Could be daily, weekly, or on-demand. |
| Whether the editor is in-game or a separate app | Open. Workbench parity matters more than where it lives. |
| Final mission length distribution | Open. Likely a mix of 10-min skirmishes and 30-60 min ops. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Hand-authored only | Highest narrative quality but lowest replayability per dev hour; doesn't fit a solo-dev team supporting MMO-scale ambitions. |
| Procedural only | Replayability without anchor moments to teach the fantasy; risks "everything feels samey". |
| Hybrid without player authoring | Wastes the editor parity that the manifest format makes nearly free; cuts off the long-tail content engine. |
| Player-authored as the only content plan | No anchor quality bar; would feel like an empty editor at launch. |

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 2): "Manifest-first hybrid: hand-authored anchor missions + procedural contracts + first-class player-authored scenarios through the same tools… For v1, missions should not be hand-authored only and not purely procedural. The best structure is a typed mission-manifest system where every mission, whether official, generated, or player-made, uses the same contract."
- Captured in [[research-log/2026-05-04-spec-round-2-setting-mission-death]].
- Aligned with the existing manifest direction in [[spec/mission-director-slice-a]].

## Revisit Trigger

- Procedural generator fails to deliver replayable variety after MISSION-A tests.
- Manifest format proves too rigid for hand-authored set pieces.
- Editor parity with internal tools turns out to be much more expensive than estimated.
