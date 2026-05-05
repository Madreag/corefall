---
type: decision
id: DR-022
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "AI-H tests can validate intent/perception/doctrine/recovery but commander adaptation across missions proves architecturally infeasible."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[decisions/dr-008-ai-architecture|DR-008]] · [[spec/ai-trust-harness-slice-a|AI harness Slice A]] · [[systems/ai-trust-test-suite|AI trust suite]]

# DR-022: AI Humanlike-ness Success Bar

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Persistent teammate-and-rival AI: friendly bots feel like teammates and enemy commanders feel like opponents with memory. The bar is: "I can predict a bot's style, trust its stated intent, be surprised by its choices without feeling cheated, and see it learn from one mission into the next."

## Decision

The AI ships when it satisfies **all** of the following success criteria:

| Criterion | What It Means | How To Test |
|---|---|---|
| **Intent** | Bots announce/display "covering door", "breaching left wall", "low ammo, falling back", "pilot trapped, ejecting", "no safe explosive shot". | Replay shows reason labels for every meaningful action. AI-H scenarios validate per [[spec/ai-trust-harness-slice-a]]. |
| **Perception** | Bots act from sight/hearing/memory, not omniscience. Wrong beliefs are visible and correctable. | Replay overlay shows perception state, alarm events, last-known-target, line-of-sight. False beliefs surface in debrief. |
| **Doctrine / personality** | Cautious medic, aggressive breacher, stubborn heavy, glory-hound sniper, careful engineer, panicking rookie, cold robot all behave differently in repeatable ways. | Two bots with same role + different doctrine produce different replay traces in same scenario. |
| **Plausible mistakes** | Bots miss, hesitate, overcommit, panic, take a bad route, drop gear, misread a threat, waste ammo — but the mistake is explainable. | Replay shows reason chain for every mistake. No "AI just stopped working" moments. |
| **Recovery** | Bots replan after terrain destruction, pick up dropped gear, call for help, retreat, dig another route, revive/rescue, eject, repair, or admit an order is impossible. | AI-H scenarios cover stuck-recovery, route-blocked, weapon-lost, low-ammo, friendly-down, hazard. |
| **Strategic adaptation** | Enemy commander remembers "player tunnels under bases", "player relies on shields", "player overuses heavy mechs", "player rushes the brain/core", then counters with sensors, patrol changes, traps, EMP, ambushes, hard materials, decoys, anti-mech tools. | Cross-mission test: same player tactic three missions in a row triggers visible commander counter on mission four. |
| **Replay proof** | Every impressive or bad AI moment can be replayed with reason labels and cause chains. | Every AI decision in the replay viewer shows: perception, options considered, score, chosen action, result. |
| **Fairness** | AI can be clever, but cannot feel secretly omniscient, silently cheating, or unbeatable because of hidden information. | "AI cheated" feedback events from playtests trigger root-cause investigation. No hidden vision/range bonuses without UI exposure. |

All eight must hold. Failing any one resets the AI claim.

## What This Locks In

| Spec Area | Implication |
|---|---|
| AI architecture (DR-008) | The hybrid jobs+utility+scripted layered model must support all eight criteria. Reason labels and perception events are mandatory output. |
| Replay / event | Per-decision events with `tactic_chosen`, `perception_updated`, `recovery_action`, `commander_adaptation`. See [[systems/replay-event-architecture]]. |
| Cross-mission state | Commander AI memory persists across missions in campaign mode. New campaign-state schema. |
| AI trust harness | AI-H tests cover intent, perception, doctrine, mistakes, recovery, strategic adaptation, replay, fairness — eight test families, not the original six. See [[spec/ai-trust-harness-slice-a]]. |
| Modding | Mod-authored doctrines/personalities must satisfy the same criteria; validator catches missing reason labels. See [[decisions/dr-006-modding-data-model]]. |
| UX | Player-facing intent labels are mandatory: HUD, squad panel, command overlay all show what the bot is trying. See [[spec/ux-overlay-screen-brief]]. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Specific perception model (raycast vs. sight cone vs. memory grid) | Open. Architectural specifics tied to engine choice. |
| Number of personality archetypes at launch | Open. Suggested 6-10. |
| Whether commander adaptation uses ML or hand-authored rules | Open. Hand-authored first; ML moonshot. |
| Granularity of cross-mission memory | Open. Per-faction-commander, per-encounter, per-tactic — TBD. |
| Whether fairness includes "AI handicap toggle" for difficulty | Open. Likely yes; transparency required. |

## Why Not The Alternatives

| Alternative Lower Bar | Why Rejected |
|---|---|
| "Communicates intent" alone | Necessary but insufficient; without perception/doctrine/adaptation it's just a chat overlay. |
| "Plausible mistakes" alone | Necessary but insufficient; mistakes without recovery feel broken. |
| "Has personality" alone | Necessary but insufficient; without strategic adaptation it's flavor only. |
| "Strategic adaptation" alone | Necessary but insufficient; without intent/perception/recovery it can feel like cheating. |

The DR-014 bar of "MOST HUMANLIKE AI IN THE GENRE" requires all four lower bars to hold simultaneously, plus replay-proof and fairness.

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 3): "Persistent teammate-and-rival AI… The highest bar is: I can predict a bot's style, trust its stated intent, be surprised by its choices without feeling cheated, and see it learn from one mission into the next."
- Eight criteria paraphrased from project owner statement.
- Captured in [[research-log/2026-05-04-spec-round-3-visuals-audio-tutorial-mechs-ai]].
- Implements the bar from [[decisions/dr-014-tone-player-promise]] and the layered approach from [[decisions/dr-008-ai-architecture]].

## Revisit Trigger

- AI-H tests can validate seven of eight criteria but commander adaptation across missions proves architecturally infeasible.
- Performance budget for perception/memory modeling explodes.
- Modders cannot author doctrines that satisfy the criteria without 10x the schema complexity.
