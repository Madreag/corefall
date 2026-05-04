---
type: decision
id: DR-005
status: open
priority: P0
revisit_trigger: "When a terrain-event-rate prototype + authority feasibility memo are complete."
---

← [[decisions/index|decision records]] · [[systems/networking-backend-frontend|networking]] · [[engine/network-terrain-replication-lifecycle|terrain replication]] · [[comparables/soldat-and-opensoldat|OpenSoldat lessons]] · [[comparables/openlierox-local-audit|OpenLieroX audit]]

# DR-005: Multiplayer Posture

> [!info] Status: OPEN; LEAN: solo-first + co-op-ready architecture; prototype networking (including PvP experiments) freely; no launch PvP promise yet

## Context

Cortex/CCCP/C4 networking is RakNet-era and bitmap-delta-based. GameNetworkingSockets is a transport, not a game-state model. Terrain destruction + per-pixel material + Lua AI + heavy gibs are difficult to sync. The decision here is about **launch promise**, not research. PvP and online experiments are encouraged at any time as research/prototype work; this DR only governs what we *commit to ship* at launch. See [[engine/network-terrain-replication-lifecycle]] and [[systems/networking-backend-frontend]].

## Options

| Option | Summary |
|---|---|
| A. Solo-only at launch | No multiplayer; design simulation freely. |
| B. Solo + local co-op (split-screen) | Local 2-4 players; no online. |
| C. Solo + online co-op (server-authoritative) | Online with one host per session. |
| D. Solo + small online co-op + dedicated servers | Adds dedicated server option. |
| E. Live PvP (competitive) | Full competitive online. |
| F. Async strategic layer + local combat | Online for campaign/contracts/factions only; combat stays local. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Maximum sim freedom. | Loses social retention. | None. |
| B | Local couch co-op nostalgia. | Modest reach; doesn't help online community. | Input remap UX cost. |
| C | Best balance for solo-first promise. | Bandwidth + authority scale; replay/event must be solid. | Bandwidth at heavy combat density. |
| D | Community servers + modding power. | Server hosting cost; abuse moderation. | Operator demand. |
| E | Highest retention if it works. | Determinism/authority/cheating all hard with destructible sim. | Probably not viable in scope. |
| F | Adds online without sim sync risk. | Doesn't satisfy "play together" expectation. | Whether players accept async-only. |

## Evaluation

| Lens | A | B | C | D | E | F |
|---|---|---|---|---|---|---|
| Player value | Strong sim | Local social | Online social | Online + mod | Competitive | Persistence |
| Readability | High | Medium | Medium | Medium | Low | High |
| AI burden | Lowest | Low | Medium | Medium | Highest | Low |
| UX burden | Low | Medium | High | High | Highest | Medium |
| Performance risk | Lowest | Low | High | High | Highest | Lowest |
| Modding impact | Highest | High | High (mods + servers) | Highest | Hard (anti-cheat) | High |
| Networking/replay impact | None | None | Strong | Strong | Strongest | Backend only |
| Content cost | Lowest | Low | High | High | Highest | Medium |
| Retention upside | Medium | Medium | High | High | Highest | Medium |
| Ethics/fairness | Low | Low | Medium | Medium | Hardest | Low |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| CCCP networking is RakNet bitmap-delta + multiplayer activities; not modern. | [[engine/network-terrain-replication-lifecycle]] | High |
| C4 has more visible multiplayer infrastructure including NAT punch. | [[repos/c4-continuation-engine]] | High |
| OpenSoldat refactored to GameNetworkingSockets but still implements custom snapshots/deltas and game-state packets. | [[comparables/opensoldat-local-audit]] | High |
| OpenSoldat local code applies client-sent position, velocity, aim, keys, weapon, and ammo state with limited validation, making it a cautionary authority model for public online play. | [[comparables/opensoldat-local-audit]] | High |
| OpenLieroX legacy packets include movement/control/rope/weapon/velocity plus carve/damage messages; server-side read accepts client movement fields, making it another cautionary authority model. | [[comparables/openlierox-local-audit]] | High |
| OpenLieroX NewNet has save/restore/checksum/rollback-style intent but current source does not prove a finished reliable model. | [[comparables/openlierox-local-audit]] | High |
| OpenLieroX public changelogs repeatedly fixed rope, explosion, shot desync, invisible laser, dedicated server, upload-limit, and file-transfer/network issues. | [[comparables/openlierox-local-audit]] | Medium |
| Determinism with Lua + physics + RNG is widely known to be hard. | Industry standard; Noita talks. | High |
| Solo-first product promise dominates research. | [[strategy/best-cortex-like-game-principles]] | High |
| Replay/event architecture (DR-002) is the prerequisite for online. | [[systems/replay-event-architecture]] | High |

## Current Recommendation

Recommendation for **launch commitments**: **Posture A+B at launch with C-ready architecture; D, E, F as post-launch milestones if/when proven**.

- Ship solo-first.
- Add local co-op (B) early.
- Build all simulation/networking-relevant interfaces (event log, server authority hooks, snapshot writer, client prediction friendliness) so that **C is implementation work, not a redesign**.
- Live PvP (E) is not a launch promise yet. Prototype it freely whenever it could improve the game; promote to a launch promise only after a bandwidth/authority/cheating prototype passes.
- F (async strategic layer) can come post-launch.

Why: protects sim freedom; keeps the solo-first promise truthful at launch; preserves online optionality; supports ambitious networking experiments without forcing premature commitments.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| Terrain-event rate at peak combat (1 host + 20 actors). | Bandwidth budget for online co-op (C). | Pass = under target bandwidth (e.g. 64 KB/s/client). |
| Rope/tether reconciliation test. | Whether mobility mastery can be predicted/authoritatively corrected without feeling bad. | Pass = attach/release/force corrections are readable and rare. |
| Projectile/terrain semantic event test. | Whether explosions, beams, drills, and child projectiles can sync as events instead of bitmap spam. | Pass = no visible divergence after 5-minute combat replay. |
| Authority feasibility memo (server vs host vs deterministic). | Authority model viability. | Memo identifies one viable model with risk profile. |
| Co-op prototype with two clients on local network. | C path is real. | Pass = same world state for 5 minutes; Fail = drift. |
| Local split-screen (B). | Input remap + camera scope. | Pass = playable for 10 minutes without confusion. |

## Risks

| Risk | Mitigation |
|---|---|
| Online expectations from comparable shooters pull PvP into a premature launch promise. | Be honest in communication: "solo-first launch; online co-op planned; PvP is being prototyped but not promised". |
| Server costs balloon. | Co-op-first; rely on host or community-run dedicated server until usage data justifies more. |
| Cheating ecosystem pressures anti-cheat. | Server-authoritative design; replay/event log enables review. |
| Mods incompatible with co-op. | Per-mod compatibility tier; mod hash sync. |
| PvP prototype reveals real fun before sim is ready. | Capture findings in a new sub-DR; do not let excitement skip the bandwidth/authority work. |

## Revisit Trigger

Reopen this decision when:

- Terrain-event prototype measures actual bandwidth.
- Replay/event architecture (DR-002) is operational.
- Community demand data exists post-launch.
- A change in product goal (e.g. competitive ladder) is requested.

## Source Trail

- [[engine/network-terrain-replication-lifecycle]]
- [[systems/networking-backend-frontend]]
- [[comparables/soldat-and-opensoldat]]
- [[comparables/opensoldat-local-audit]]
- [[comparables/openlierox-local-audit]]
- [[systems/replay-event-architecture]]
- [[repos/c4-continuation-engine]]
- [[strategy/best-cortex-like-game-principles]]
