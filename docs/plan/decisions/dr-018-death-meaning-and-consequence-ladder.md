---
type: decision
id: DR-018
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Playtests show rescue-first feels too soft, or confirmed-death permanence triggers save-scumming, or chassis-specific death meanings confuse players."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/body-damage-model|body damage model]] · [[spec/chassis-armor-mechs-and-origins|chassis spec]] · [[spec/progression-retention|progression/retention]] · [[spec/command-core-base-power|command core]] · [[decisions/dr-003-body-damage-readability|DR-003]] · [[decisions/dr-014-tone-player-promise|DR-014]] · [[decisions/dr-015-player-identity-control-posture|DR-015]]

# DR-018: Death Meaning And Consequence Ladder

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Tiered consequence ladder. Rescue-first by default in the campaign; confirmed deaths are permanent; scenario authors can override.

## Decision

**Tiered consequence ladder per actor type, scenario-configurable.**

When a named actor takes lethal damage:

1. The simulation determines what physically happened (downed, unstable, dying, trapped, ejected, maimed, repaired, revived, extracted, lost).
2. The scenario policy determines whether the actor is recoverable, permanently dead, or replaceable.
3. The replay/debrief surfaces the cause chain regardless of recovery.

### Default campaign policy: rescue-first with confirmed permanence

Named actors should not vanish from one bad physics hit if there was a believable rescue / repair / eject / stabilize / revive / tow / extraction window. But once the fiction and simulation say the actor is **truly gone**, that death is permanent and remembered. Survivors return with scars, prosthetics, trauma, traits, changed AI behavior, rescue history, reputation, and battle memories.

### Per-origin death meanings

| Origin | Lethal Event | Recovery Window | Permanence Trigger |
|---|---|---|---|
| Organic actor (human, biomech) | Bleeding/wounds → stabilization → revive window → death | Yes — medic, stabilizer, revive in time | Brain damage, head/torso destruction, no medic in time, prolonged DYING, no extraction. Veteran returns with scars/prosthetics if rescued. |
| Android / robot | Module failure → EMP shock → reboot → repair → shell replacement | Yes — data-core recovery, shell swap | Data-core destroyed; persistent identity lost. Shell can be salvaged for parts. |
| Mech / powered armor | Pilot eject → trapped pilot → wreck → destroyed chassis | Eject window depends on chassis stage | Pilot dies if eject fails; chassis can be wrecked-but-salvageable or fully destroyed. |
| Clone / shell-replacement body | Body cheap; clones can be replaced | Yes — easy | Veteran *identity* is not free; if the underlying continuity (memory, training, traits) is lost, that's the real loss. |
| Command core / operator (player anchor) | Highest-stakes loss | Limited; depends on uproot state | If destroyed or lost while embedded in an avatar, can be **campaign-ending** or a major strategic disaster. See [[spec/command-core-base-power]]. |

### Required event chain

Every lethal event emits the cause chain so the debrief can answer "why":

| Event | Payload Highlights |
|---|---|
| `actor_status_changed` | Actor, old/new status, cause event id, reason label. |
| `chassis_stage_changed` | Per [[spec/chassis-armor-mechs-and-origins]] — eject windows, wreck, gibbed/exploded. |
| `pilot_state_changed` | If chassis was occupied. |
| `rescue_attempted` / `rescue_succeeded` / `rescue_failed` | Who tried, with what tool, why it succeeded/failed. |
| `extraction_offered` / `extraction_taken` / `extraction_missed` | Surfaces "you could have saved them" moments. |
| `actor_lost_permanently` | Final, non-recoverable. Triggers veteran archive update. |
| `salvage_recovered` | Gear, modules, body parts, data cores. |

## Scenario Override Policies

The campaign default is rescue-first + confirmed permanence. Scenario authors can pick:

| Policy | Use Case |
|---|---|
| `default` | Campaign default. |
| `hardcore_permadeath` | One hit one death; no rescue window. For roguelite scenarios. |
| `arcade_sandbox` | Deaths reset on round end; for skirmish/training. |
| `clone_war` | Bodies cheap and replaceable; veteran identity still mortal. |
| `roguelite_run` | Permadeath but new run starts fresh; campaign meta resets. |
| `tutorial_safety` | Lethal events demoted to KO until tutorial mission ends. |
| `rescue_friendly` | Generous extraction windows for early-game players. |
| `iron_company` | All lethal events permanent + named-veteran focus. |
| `command_core_endgame` | Command-core destruction is mandatory mission failure. |

The default campaign mixes these intentionally: most missions use `default`; specific anchor missions can use `iron_company` for stakes or `clone_war` for set pieces.

## What This Locks In

| Spec Area | Implication |
|---|---|
| Body damage model | Must implement coarse states (STABLE/UNSTABLE/DYING/DEAD/INACTIVE) + extended ladder per origin. See [[spec/body-damage-model]]. |
| Chassis spec | Eject windows, wreck states, salvage events already align. See [[spec/chassis-armor-mechs-and-origins]]. |
| Replay/debrief | Cause-chain tracing is mandatory; "why did Lt. Hernandez die" must be answerable. See [[spec/replay-recorder-slice-a]]. |
| Mission director | Extraction offer / rescue affordance / salvage events must be first-class mission concepts. See [[spec/mission-director-slice-a]]. |
| Progression/retention | Veteran archive, scar/prosthetic system, reputation, memorial wall (if any) belong in [[spec/progression-retention]]. |
| AI | Bots must understand rescue, retreat, stabilize, eject, salvage, and explain the choice. See [[decisions/dr-008-ai-architecture]]. |
| UX | "EXTRACTION OFFERED" / "RESCUE WINDOW CLOSING" / "PILOT EJECTING" banners belong in HUD. Death recap shows the recovery windows that opened and closed. See [[spec/ux-wireframes-slice-a]]. |
| Modding | Scenario policy is exposed in the manifest; authors pick. See DR-017 and [[spec/modding-model]]. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Pure permadeath | Punishes physics chaos too hard; causes save-scumming; cuts narrative depth of veteran arcs. |
| Pure recoverable | Erases emotional stakes; turns deaths into mild inconvenience; weakens the chassis/eject grammar. |
| Tiered per chassis only (no scenario override) | Locks every campaign to one feel; players can't run iron-man, can't run arcade. |
| Player-configurable global only | Campaign loses curated emotional pacing; iron-company moments need the campaign's authority. |

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 2): "Tiered consequence ladder. Campaign default should be rescue-first with real confirmed deaths. A named actor should not vanish from one bad physics hit if there was a believable rescue, repair, eject, stabilize, revive, tow, or extraction opportunity. But once the fiction and simulation say the actor is truly gone, that death should be permanent and remembered."
- Per-origin meanings paraphrased from project owner statement (organic, android/robot, mech/powered armor, clone/shell, command core/operator).
- Captured in [[research-log/2026-05-04-spec-round-2-setting-mission-death]].
- Aligned with [[spec/body-damage-model]], [[spec/chassis-armor-mechs-and-origins]], [[spec/progression-retention]], [[spec/command-core-base-power]].

## Revisit Trigger

- Playtests show rescue-first feels too soft (players never lose anyone).
- Confirmed-death permanence triggers save-scumming behaviour we can't fix with UX.
- Chassis-specific death meanings confuse players or mod authors.
- Scenario-policy system is too complex to teach.
