---
type: spec
status: closed-direction
authority: "Tutorial implementation: 1 polished onboarding mission + 8 modular labs + contextual fading tooltips + 'show me why' handoff per DR-023. Adaptive hints. AI-authored copy. Closes the implementation gap left by DR-023."
ready_when: "Onboarding completion >80% in playtest; lab completion >60% per lab; tutorial-safety policy honored; show-me-why handoff triggered correctly across all failure modes; AI-authored hints accurate >95%."
feeds:
  - DR-008
  - DR-012
  - DR-014
  - DR-018
  - DR-022
  - DR-023
  - DR-046
---

← [[spec/index|spec section]] · [[decisions/dr-023-tutorial-and-onboarding-strategy|DR-023]] · [[decisions/dr-046-player-facing-surfaces-direction|DR-046]] · [[spec/missions-and-objectives|missions]] · [[spec/replay-recorder-slice-a|replay]]

# Tutorial Implementation

## Overview

Per DR-023 hybrid+: **One cinematic onboarding mission + 8 modular labs + contextual fading tooltips + "show me why" handoff to replay/lab from any failure**. Implementation closes the gap.

## Onboarding Mission: "First Contract"

**Length:** 12-15 min.
**Setting:** Earth urban industrial outpost.
**Mission:** Recover a downed scientist from a destroyed research lab; clear husk infestation; extract via dropship.
**Tutorial-safety:** lethal demoted to KO until end of mission; per [[decisions/dr-018-death-meaning-and-consequence-ladder]].

### Beat structure
1. **Drop-in (2 min):** Direct-control body; learn movement (W/A/S/D/Space) + camera + aim. Simple terrain. Captioned dialog from commander.
2. **First contact (2 min):** Engage 1-2 light husks; teach fire (LMB) + reload (R) + switch weapon. Husks demoted to KO; teach revive.
3. **Squad partner (2 min):** AI teammate appears; teach squad order (TAB to issue / Q to call / wave gestures). Cover fire + push.
4. **Breach (2 min):** Door blocks path; teach digger tool / breach charge. Material physics.
5. **Recovery (2 min):** Find scientist (downed); teach revive (E) + carry (G).
6. **Dropship call (2 min):** LZ reveal; teach dropship command (call to LZ / abandon LZ); teach extract.
7. **Replay/debrief (2 min):** Auto-loaded debrief comic-panel timeline; explain "show me why" + lab launcher.

### Voice acting
- **Hero option:** ElevenLabs AI-voice for commander + scientist (license review pre-launch); 30-50 lines.
- **Fallback:** Text-only with subtitle + comic-panel speech bubbles.

### Done-criteria
- [ ] Player completes "First Contract" without external help in 80%+ playtest sessions.
- [ ] All 7 beats fire correctly.
- [ ] Tutorial-safety policy honored (no permanent deaths during onboarding).
- [ ] Replay viewer auto-loads debrief.

## Modular Labs (8)

Per DR-023, each ~2 min. Permanent fixtures accessible from base/workbench. Each lab is a self-contained scenario with cinematic intro + interactive teaching + outro confirmation.

| Lab | Teaches | Done-criteria |
|---|---|---|
| `lab_movement_aim` | Ground move, aim, recoil, jetpack, stance, cover. | Player completes 5 timed move/aim challenges. |
| `lab_terrain_materials` | Digging, breaching, repair, collapse risk, material overlay. | Player breaches 3 walls + identifies 5 materials. |
| `lab_loadout_delivery` | Loadout building, dropship craft, LZ risk, equipment role cards. | Player builds 3 loadouts + delivers 3 squads. |
| `lab_squad_orders_ai` | Squad orders, AI intent, rescue, retreat, recovery. | Player issues 6 distinct order types. |
| `lab_command_core_base` | Rooting the core, base power, shields, turrets, sensors, doors, repair pads. | Player roots core + powers 2 systems + uproots. |
| `lab_avatar_mode` | Uprooting the core and embedding it into a body/mech as a risky avatar. | Player uproots + embeds + survives 1 minute. |
| `lab_chassis_damage` | Armor/mech module damage, smoke/failure states, ejection, salvage. | Player ejects + recovers + repairs 1 chassis. |
| `lab_replay_debrief` | Why I died, what I could have done, retry same seed. | Player scrubs replay + identifies cause-of-death + retries. |

### Lab manifest format
```ron
// content/labs/lab_movement_aim.ron
lab: (
    id: "lab_movement_aim",
    title_key: "lab.movement_aim.title",  // localizable
    description_key: "lab.movement_aim.description",
    duration_estimate_s: 120,
    scenario: "tutorial/movement_aim",
    objectives: [
        ("complete_5_timed_challenges", { tutorial_safety: true }),
    ],
    teaches: ["movement_basics", "aim_recoil", "jetpack", "stance", "cover_use"],
    failure_routes: [
        ("died_to_husk", { auto_handoff: "lab_chassis_damage" }),
        ("stuck_in_terrain", { auto_handoff: "lab_terrain_materials" }),
    ],
)
```

## Contextual Fading Tooltips

Per DR-023:

- Tooltips appear contextually (e.g., near a button, near a chassis, near a UI control).
- Per-tooltip use counter; fade after 3 uses.
- Per-mastery flag; if player has done X 10+ times, suppress related tooltip.
- Re-enable any tooltip via Settings → Gameplay → "Reset Tutorial Tooltips."

### Tooltip catalog (50+)
Each player-facing system has tooltips: movement, weapons, equipment, UI elements, command core, base modules, AI orders, replay/debrief, mods, settings.

### Tooltip definition format
```ron
// content/tooltips/movement_jetpack.ron
tooltip: (
    id: "movement_jetpack",
    trigger: { kind: "actor_has_jetpack_equipped" },
    title_key: "tooltip.movement_jetpack.title",
    body_key: "tooltip.movement_jetpack.body",
    icon: "ui/icons/jetpack.svg",
    fade_after_uses: 3,
    suppress_if: ["mastery.movement_jetpack >= 5"],
    relate_to: "lab_movement_aim",
)
```

## "Show Me Why" Handoff

Per DR-023. Failure mode handoff:

| Failure | Handoff |
|---|---|
| Player death | Replay scrubs to last 5s; "lab_chassis_damage" suggested; "lab_movement_aim" if movement failure. |
| Mission lost | Replay shows debrief timeline; "lab_squad_orders_ai" suggested; "lab_command_core_base" if core failure. |
| Command core lost | Replay shows core moment; "lab_command_core_base" suggested. |
| Mech wrecked | Replay shows wreck; "lab_chassis_damage" suggested. |
| Stuck in terrain | "lab_terrain_materials" suggested. |
| Squad refused order | "lab_squad_orders_ai" suggested. |
| LZ failed | "lab_loadout_delivery" suggested. |
| Equipment didn't work | "lab_loadout_delivery" suggested. |
| Bunker breached | "lab_command_core_base" suggested. |
| Material kill (acid/toxic gas) | Material education tooltip + "lab_terrain_materials" suggested. |
| Damage afflictions misunderstood | "lab_chassis_damage" suggested. |
| Replay/debrief misunderstood | "lab_replay_debrief" suggested. |

## Adaptive Hints

Hint engine reads `EnvironmentSignal` + AI bot scoring + player input patterns + session telemetry to surface contextual hints.

| Hint trigger | Example |
|---|---|
| Player pressed reload while ammo full | "You don't need to reload yet." |
| Player ignored ally call for rescue 30+s | "Press F to acknowledge ally call." |
| Player aimed but never fired | "LMB fires; check if weapon is jammed." |
| Player on Mars without sealed helmet | "Atmosphere is thin; helmet protects." |
| Enemy in cover; player didn't use grenade | "Press G to throw grenade." |
| Mission timer < 30s | "Extract before timer ends." |
| Husks approaching from rear | "Enemy detected behind you." |
| Player low HP, near medikit | "Press E to use medikit." |

Hint accuracy target: >95% (no false-positives).

Hints are suppressible per category in Settings.

## AI-Authored Mission Narrative

Per DR-046. Tutorial mission briefing/debrief copy generated by Claude Sonnet / GPT-4o per faction tone profile + reviewed by AI agent. Comic-panel art generated per [[spec/art-and-asset-pipeline]].

## Difficulty / Accessibility Presets

Per DR-012:

| Preset | Damage taken | Damage dealt | AI aggression | Time scale | Hint frequency |
|---|---|---|---|---|---|
| **Standard** | 100% | 100% | Standard | 100% | Medium |
| **Easy** | 60% | 130% | Reduced | 90% | High |
| **Hard** | 130% | 90% | Increased | 110% | Low |
| **Custom** | sliders | sliders | sliders | sliders | sliders |
| **Accessibility-relaxed** | 50% | 200% | Minimal | 60% | Forced |

## Done-Criteria

- [ ] Onboarding completion >80% in playtest cohort.
- [ ] Lab completion >60% per lab.
- [ ] Tutorial-safety policy honored across onboarding + labs.
- [ ] Show-me-why handoff triggers correctly for all 12 failure modes.
- [ ] AI-authored hints accuracy >95% in playtest.
- [ ] All tutorial copy localized to Tier-A languages.
- [ ] Tooltips fade gracefully + are reset-able.
- [ ] Difficulty/accessibility presets functional.
- [ ] CI gate: every UI element + system has tooltip data.

## Source Trail

- [[decisions/dr-023-tutorial-and-onboarding-strategy]]
- [[decisions/dr-046-player-facing-surfaces-direction]]
- [[spec/missions-and-objectives]]
- [[decisions/dr-018-death-meaning-and-consequence-ladder]]
