---
type: decision
id: DR-004
status: open
priority: P0
revisit_trigger: "When DR-001 (engine), DR-002 (replay), and the actor-feel sandbox are scheduled."
---

← [[decisions/index|decision records]] · [[spec/actor-feel-sandbox-slice-a|actor-feel Slice A]] · [[strategy/research-to-spec-roadmap|research-to-spec roadmap]] · [[strategy/best-cortex-like-game-principles|best principles]] · [[comparables/openlierox-local-audit|OpenLieroX audit]] · `cortex-command-repos-all/VAULT_PLAN.md` (research vault root)

# DR-004: First Playable Slice

> [!info] Status: OPEN; LEAN: single-actor sandbox -> small squad mission -> bunker breach
> BP1 has completed the A-side proof (M1 + M1.5). DR-004 remains open because the full first-playable promise is the B/C ladder culminating in M7's Breach Contract.
 
## Context

Decide what the first publicly-demoable slice contains. This shapes prototype order, hiring focus, milestone narrative, and the test bench for AI/terrain/UX trust.

## Options

| Option | Summary |
|---|---|
| A. Single-actor sandbox | One controllable actor in a small destructible scene. |
| B. Small squad scenario | 3-5 actor squad + simple objective. |
| C. Mission slice (BunkerBreach-equivalent) | Full bunker breach with AI commander. |
| D. Open-world campaign demo | Faction map + travel between operations. |
| E. Combat arena (Liero-style) | 2-4 player free-for-all in a small destructible map. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Fastest "game feels good" answer; tests core controller; tests destruction. | Doesn't prove squad AI or strategy layer. | None significant. |
| B | Tests command UX, AI trust, body damage at squad scale. | Risk of feeling shallow if mission is bare. | AI trust readiness. |
| C | Most impressive demo; tests breach + reinforcement + replay. | Largest risk surface; many systems must work simultaneously. | Whether AI/terrain/replay are mature enough. |
| D | Sells campaign vision. | Premature; depends on contracts/factions/economy systems. | Scope blowout. |
| E | Easy to demo; high "fun" return. | Doesn't prove AI/strategy core; multiplayer pressure too early. | Networking maturity. |

## Evaluation

| Lens | A | B | C | D | E |
|---|---|---|---|---|---|
| Player value | Feel | Tactical | Strategic | Strategic | Combat |
| Readability | High | Medium | Medium | Low | High |
| AI burden | Low | Medium | Highest | Highest | Medium |
| UX burden | Low | Medium | Highest | Highest | Medium |
| Performance risk | Low | Medium | Highest | Highest | Medium |
| Modding impact | Low | Medium | High | High | High |
| Networking/replay impact | Low | Low | High | High | Highest |
| Content cost | Lowest | Low | Highest | Highest | Medium |
| Retention upside | Low (demo only) | Medium | High | Highest | Medium |
| Ethics/fairness | Low risk | Low risk | Low risk | Low risk | Multiplayer fairness risk |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| Cortex feel comes from controller + destruction + body. | [[design/design-decisions]], [[engine/projectile-to-impact-lifecycle]] | High |
| AI trust scenarios assume squad behavior. | [[systems/ai-trust-test-suite]] | High |
| BunkerBreach is the canonical hero mission pattern. | [[systems/destruction-objective-mission-patterns]], BunkerBreach.lua | High |
| Replay/event capture is foundational and not yet built. | [[systems/replay-event-architecture]] | High |
| OpenSoldat local audit adds control-state, reticle feedback, inherited projectile velocity, and weapon-feel schema lessons for slice A. | [[comparables/opensoldat-local-audit]] | High |
| OpenLieroX local audit adds rope/tether mastery, material anchor rules, and short destructive arena lessons for a slice-A mobility lane. | [[comparables/openlierox-local-audit]] | High |
| Slice A now has an implementation-facing prototype requirements page with scope, material set, event hooks, acceptance tests, first tickets, and kill criteria. | [[spec/actor-feel-sandbox-slice-a]] | High |
| Native BP1 completed the A-side proof: M1 actor control + M1.5 micro breach win/loss with run-bundle evidence, cf-e2e scripts, and T-CAPTURE. | [[prototypes/native-m1-5-micro-breach]], [[spec/prototype-roadmap#Build Points (Roadmap V2)]] | High |
| Solo-first promise dominates research direction. | [[strategy/best-cortex-like-game-principles]] | High |

## Current Recommendation

Recommendation: **Sequenced A -> B -> C**, where:

- A is now represented by the native BP1 proof: single actor + micro breach. The remaining A-side lessons feed BP2's terrain/replay proof rather than reopening the actor lab.
- B is the demo slice (~6-8 weeks after A): 3-actor squad, command overlay v0, body damage UI, replay recap.
- C is the public alpha slice (~16-24 weeks after B): bunker breach with reinforcements, AI commander, replay/event scrub.

Reasoning: each slice unlocks the next. Skipping to C without A makes scope unmanageable; skipping to D/E loses our solo-first promise.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| A: 5 minutes of solo play feels good with one actor + dig/shoot. | Core controller. | Pass = quotable "this is fun" reaction in playtest. |
| A recorder: last 30 seconds of a damage/death/terrain failure can be reconstructed. | DR-002-ready debugging infrastructure. | Pass = Slice A emits input, weapon, projectile, terrain, actor status, and snapshot events listed in [[spec/actor-feel-sandbox-slice-a]]. |
| A mobility lane: anchor/jet/tether feedback is readable. | Movement mastery can become a retention hook. | Pass = player can explain valid/invalid anchor or thrust state without reading a tooltip. |
| A->B handoff: 3-actor squad with one AI behavior produces no clumping deaths in 10 minutes. | Squad MVP. | Pass = no friend-trapped-in-tunnel within 10 min. |
| B->C handoff: BunkerBreach equivalent runs to completion with AI commander reinforcing. | Mission system trust. | Pass = mission completable solo in under 15 min. |
| Replay: every slice publishes a replay recap at end. | Replay foundation. | Pass = recap shows correct cause-of-death. |

## Risks

| Risk | Mitigation |
|---|---|
| A slips because "feel" is subjective. | Time-box; second prototype iteration only after a third user playtest. |
| B reveals AI is harder than estimated. | Trim scope to 3 AI behaviors; expand only after AI trust suite passes. |
| C demands networking/co-op pressure too early. | Co-op stays a stretch goal for slice C; prototype freely but ship C as solo-first. |
| Demo pressure pushes to D before B finishes. | Resist on the launch path; campaign demos before B is solid lie about systems quality. Side-prototypes are still fine. |

## Revisit Trigger

Reopen this decision when:

- DR-001 (engine) settles; affects prototype velocity.
- DR-002 (replay) settles; affects acceptance tests.
- M7's Breach Contract either validates or invalidates the A -> B -> C sequencing.

## Source Trail

- [[strategy/research-to-spec-roadmap]]
- [[strategy/best-cortex-like-game-principles]]
- [[systems/ai-trust-test-suite]]
- [[systems/destruction-objective-mission-patterns]]
- [[systems/replay-event-architecture]]
- [[engine/body-damage-wound-gib-lifecycle]]
- [[comparables/opensoldat-local-audit]]
- [[comparables/openlierox-local-audit]]
- [[spec/actor-feel-sandbox-slice-a]]
