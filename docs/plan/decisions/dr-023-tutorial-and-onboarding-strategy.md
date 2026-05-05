---
type: decision
id: DR-023
status: closed-direction
priority: P0
closed_at: 2026-05-04
revisit_trigger: "Onboarding mission proves too rigid for new players, or labs are unused after first hour, or tooltip system fails to fade gracefully."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[spec/missions-and-objectives|missions]] · [[spec/mission-director-slice-a|mission director]] · [[spec/ux-wireframes-slice-a|UX wireframes]] · [[decisions/dr-017-mission-generation-strategy|DR-017]]

# DR-023: Tutorial And Onboarding Strategy

> [!success] Status: CLOSED-DIRECTION (project owner committed 2026-05-04)
> Hybrid+: one cinematic onboarding contract that delivers the fantasy emotionally + permanent in-fiction training labs accessible from the base/workbench.

## Decision

**One polished first mission** + **modular 2-minute labs** + **contextual fading tooltips** + **"show me why" handoff to replay/lab from any failure**.

### First mission (one of)

A polished onboarding mission that immediately delivers the fantasy: direct-control a body, shoot, dig/breach, command one AI teammate, call a loadout/delivery, rescue or stabilize a wounded unit, and see a replay/debrief explain what happened. Lethal events are demoted to KO until the onboarding mission ends (`tutorial_safety` policy from [[decisions/dr-018-death-meaning-and-consequence-ladder]]).

### Modular labs (always available from base/workbench)

| Lab | What It Teaches |
|---|---|
| Movement / Aim | Ground movement, aim, recoil, jetpack, stance. |
| Terrain / Materials | Digging, breaching, repair, collapse risk, material overlay. |
| Loadout / Delivery | Loadout building, delivery craft, LZ risk, equipment role cards. |
| Squad Orders / AI | Squad orders, AI intent, rescue, retreat, recovery. |
| Command Core / Base | Rooting the core, base power, shields, turrets, sensors, doors, repair pads. |
| Avatar Mode | Uprooting the core and embedding it into a body/mech as a risky avatar. |
| Chassis Damage | Armor/mech module damage, smoke/failure states, ejection, salvage. |
| Replay / Debrief | Why I died, what I could have done, retry same seed. |

Each lab is ~2 minutes. They remain useful forever for testing builds, mods, mechs, weapons, and AI behavior.

### Contextual tooltips

- Tooltips are contextual, optional, and **fade as mastery rises**.
- Fade trigger: per-tooltip use counter or per-system mastery flag.
- Player can re-enable any tooltip from settings.

### "Show me why" handoff

Every failure (death, mission loss, command-core lost, mech wrecked) opens a "show me why" path that hands off to the replay viewer or the relevant lab for retry.

## What's NOT The Strategy

- **Not** "discoverable / minimal" for core systems. The world, enemies, materials, and emergent tactics can stay mysterious; controls, UI, death causes, AI orders, loadout consequences, and command-core tradeoffs must be taught clearly.
- **Not** a single linear tutorial chain that locks the player out of the rest of the game.
- **Not** modal text walls — teaching is in-context.

## What This Locks In

| Spec Area | Implication |
|---|---|
| First playable | A1..A7 prototype path includes onboarding-mission scaffolding (even if minimal). |
| Mission manifest | `tutorial_safety` death policy is a first-class scenario manifest field. See [[decisions/dr-017-mission-generation-strategy]] and [[decisions/dr-018-death-meaning-and-consequence-ladder]]. |
| Workbench / base UI | Lab launcher is part of the base/workbench surface. See [[spec/ux-wireframes-slice-a]] and [[spec/equipment-loadout-workbench-slice-a]]. |
| Lab mode | Each lab is a tiny scenario manifest reusing the standard mission grammar. |
| Replay / debrief | Death recap → "show me why" → opens replay viewer or relevant lab. See [[spec/replay-recorder-slice-a]]. |
| Tooltip system | Per-tooltip fade with use counter or mastery flag. New `tooltip_state` data. |
| Accessibility | Tutorial UX falls under [[decisions/dr-012-accessibility-comfort-readability]]; large text, captions, controller route all required. |
| Modding | Modders can author labs the same way they author missions. See [[decisions/dr-006-modding-data-model]]. |

## What This Does NOT Lock

| Non-Commitment | Why |
|---|---|
| Onboarding mission length | Open. Suggested 10-20 min. |
| Number of launch labs | Open. Suggested 8 (per the table). |
| Whether labs are unlocked progressively or all-from-start | Open. Likely all-from-start; mastery flags decide which the game suggests. |
| Tooltip visual style | Tied to DR-019 visual direction. |
| Whether onboarding has narrative VO | Open. Tied to general audio plan. |

## Why Not The Alternatives

| Alternative | Why Rejected |
|---|---|
| Hand-authored tutorial mission only | Misses the long-tail value of repeatable labs for build testing. |
| Interactive systems tutorials only | Misses the emotional fantasy delivery; new players don't see "the game" in the first 30 minutes. |
| Discoverable / minimal | Core systems too complex (command core, chassis grammar, AI doctrine) to discover without help. Frustration risk. |
| Hybrid (onboarding + labs) without tooltip-fade or "show me why" handoff | Misses the death-recap-as-teaching loop; misses the gradual hand-off to mastery. |

## Evidence Trail

- Project owner verbatim (2026-05-04 spec round 3): "Hybrid+: One cinematic onboarding contract + permanent in-fiction training labs… First mission teaches the fantasy emotionally. Labs teach systems safely and repeatably. Tooltips are contextual, optional, and fade as mastery rises. Every failure can open 'show me why' into replay/debrief/lab retry."
- Captured in [[research-log/2026-05-04-spec-round-3-visuals-audio-tutorial-mechs-ai]].
- Aligns with [[decisions/dr-018-death-meaning-and-consequence-ladder]] tutorial_safety policy.

## Revisit Trigger

- Onboarding mission proves too rigid for new players.
- Labs are unused after the first hour.
- Tooltip system fails to fade gracefully.
- "Show me why" handoff confuses players instead of teaching.
