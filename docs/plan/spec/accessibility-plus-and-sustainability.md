---
type: spec
status: closed-direction
authority: "Accessibility-plus extensions beyond DR-012 floor (cognitive + motor + hearing + reading + sensory + ALL-audio captions + cinematic accessibility) + sustainability + sunset planning + 5-year content plan + open-source path + endless content guarantee + server hosting handoff + content archival + revenue share for key contributors."
ready_when: "All accessibility-plus presets functional + community-tested with disability advocacy partners; sustainability plan documented; 5-year content roadmap published; sunset trigger criteria documented."
feeds:
  - DR-001
  - DR-005
  - DR-006
  - DR-012
  - DR-024
  - DR-031
  - DR-035
  - DR-047
  - DR-051
---

← [[spec/index|spec section]] · [[decisions/dr-051-accessibility-sustainability-platform-and-launch-polish|DR-051]] · [[spec/accessibility-comfort-slice-a|accessibility floor]]

# Accessibility-Plus & Sustainability

## Accessibility-Plus Extensions

Beyond DR-012 floor.

### Cognitive accessibility

| Component | Detail |
|---|---|
| Lower stimulation mode | Reduced VFX, slower pace, simpler UI, fewer simultaneous threats. |
| Simple HUD preset | Minimal HUD; only critical info. |
| One-thing-at-a-time tutorial pacing | Slower tutorial cadence; explicit "wait for ready" prompts. |
| Cognitive-load-reduction toggle | Master switch; cascades to all cognitive options. |

### Motor accessibility

| Component | Detail |
|---|---|
| Single-button play mode | Context-aware single button performs most-relevant action. |
| Gesture controls | Swipe gestures for action mapping. |
| Eye tracking integration | Tobii eye-tracker support. |
| Slow-mo / pause-during-input mode | Time slows on input; for cognitive disabilities + single-handed players. |
| One-handed mode | All actions accessible with 1 hand. |
| Configurable hold-vs-toggle | Per-action; for endurance-limited players. |
| Haptic feedback alternatives | For sensory + tactile feedback. |

### Hearing accessibility

| Component | Detail |
|---|---|
| Sign language overlay | For cinematics; community-authored ASL/BSL/etc. |
| Visual sub-bass cues | Screen pulse on bass thump. |
| Haptic feedback alternatives | Already in motor; extends here. |
| Full subtitle option | NOT just critical audio; ALL audio with optional speaker label + tone description. |
| Audio description for visual events | Text + voice descriptions. |

### Reading accessibility

| Component | Detail |
|---|---|
| Dyslexic font option | OpenDyslexic. |
| High-contrast text | Beyond DR-012; opt-in. |
| Reading speed control | Per-paragraph TTS readout. |
| Per-paragraph TTS readout | Audio narration toggle. |
| Large-print preset | Cascade text-scaling. |

### Sensory accessibility

| Component | Detail |
|---|---|
| Pause-on-window-loss | Auto-pause when game window not focused. |
| Reduce-screen-shake | Already in DR-012; cascade here. |
| Low-violence mode | Decals minimal; blood color black-white; reduced gore. |
| Sensory-overload prevention | Fewer simultaneous VFX; per-tick particle cap. |
| Anxiety-mode | Slower combat cadence; reduced enemy aggression baseline. |
| Confirmation prompts on irreversible actions | Auto-prompt before quit-to-menu, abandon-mission, etc. |

### Color blind

8 protanope/deuteranope/tritanope/atypical/protocols; tested with actual color-blind testers per DR-012.

### Cinematic accessibility

| Component | Detail |
|---|---|
| Audio description for cinematics | Text + voice descriptions of visual events. |
| Skip-cinematic for low-bandwidth | Skip + summary text. |

## Sustainability + Sunset Planning

### 5-year content plan

| Year | Plan |
|---|---|
| Year 1 | Balance + cosmetics; quarterly content updates; bug-fix patches. |
| Year 2 | 1-2 expansions (paid; never gates core); new factions; new biomes. |
| Year 3 | Ranked PvP infrastructure mature; tournament season cadence; ladder reset. |
| Year 4 | Console eval + post-launch DLC; possible 1-2 console ports. |
| Year 5 | Open-source path evaluation; community handoff readiness. |

### Sunset plan

If dev moves to next project:
- Workshop / community handoff.
- cf-server hosting infrastructure community-managed.
- Engine + tooling open-sourced (per DR-001 ethical stance).
- Content archive maintained on community mirror.
- Vault published as community wiki post-sunset.

### Open-source path

If commercial path fails OR after 5+ years: donate engine + content to community per Apache-2.0 / MIT; documentation handoff.

### Endless content guarantee

Workshop + procedural generator MUST outlive first-party content; cf-server runs forever as community-hosted.

### Server hosting handoff

Community can host MMO shards forever; cf-server free + open-source-able post-sunset.

### Content archival

Replays + saves work after game-development ends.

### Documentation as legacy

Every system documented; vault published as community wiki post-sunset.

### Revenue share for key contributors

Post-launch DLC modders; fair % per DLC sold (negotiated per partner).

## Done-Criteria

### Accessibility-plus

- [ ] All cognitive presets functional.
- [ ] All motor presets functional.
- [ ] All hearing presets functional.
- [ ] All reading presets functional.
- [ ] All sensory presets functional.
- [ ] 8 color-blind protocols.
- [ ] Cinematic audio description.
- [ ] Community-tested with disability advocacy partners.

### Sustainability

- [ ] 5-year content plan documented.
- [ ] Sunset trigger criteria documented.
- [ ] Sunset trigger flow tested (mock run).
- [ ] Open-source path documented.
- [ ] Server hosting handoff documented.
- [ ] Content archival format guaranteed forward-compatible.

## Source Trail

- [[decisions/dr-051-accessibility-sustainability-platform-and-launch-polish]]
- [[decisions/dr-012-accessibility-comfort-readability]]
- WCAG 2.1 AAA: aspirational; AA targeted by DR-012.
- OpenDyslexic font: https://opendyslexic.org/
- Tobii eye-tracking: https://gaming.tobii.com/
- Stardew Valley sustainability: 8+ years post-launch.
- Hades sustainability: ongoing community 5+ years.
