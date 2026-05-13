
## Status

`active`

## Intent

**M4B is the visual direction closure** — per DR-019 + DR-046 + DR-055. After M4B, the comic-noir aesthetic is fully layered on top of M4A's HUD: hand-drawn ink-line UI / panel-bordered banners / dramatic ink-style impact particles / dynamic color grading / animated UI panels / juice rules applied per-surface. M4A is readable; M4B is *cinematic*.

M4B promise: **"the game looks like a high-budget studio production — comic-noir aesthetic with deep tactile feedback."**

## Player-facing behavior

### Comic-noir aesthetic (per DR-019)

- Hand-drawn ink-line UI elements (HUD borders, panels, banners)
- Comic-book-style speech bubbles for AI chatter (M9.5 forward-compat)
- Dramatic ink-style impact particles (instead of bland geometric shapes)
- Per-actor outline shaders (selective; emphasized actors get heavier ink)
- Dynamic color grading per scene (mission-noir filter; cinematic peak)

### Juice rules (per DR-055 + DR-046)

- Button hover juice (scale 1.0 → 1.05 over 80ms ease-out + glow halo + tick SFX)
- Click punch juice (scale 1.0 → 0.95 → 1.0 over 120ms + click SFX)
- Banner slide-in juice (slide from edge over 200ms ease-in-out)
- Critical-hit punch (hit-stop + screen flash + chromatic aberration)
- Reload completion ding (subtle SFX cue)
- Weapon swap whoosh (audio + brief light streak)
- Pickup glow (item pulses on hover)

### Animation system

- UI panel transitions (slide + skew + ease per DR-046)
- Per-element animation hooks (entry, exit, hover, focus, click, drag)
- Animation system respects `reduce_motion` setting (instant transitions)

### Comic-noir applied to:

- Mission briefing panels (comic-book style with hand-drawn art)
- Death recap modal (graphic-novel style cause-chain panels)
- Win/loss screen (cinematic comic-noir transition)
- Settings menu (ink-line tabs + hand-drawn icons)
- Hub / lobby / mod manager (post-launch M8+ inherits styling)

### AI-Authored visual assets

Per T-CONTENT-ART + AI audio pipeline:
- 50+ launch UI ink-line drawings (HUD borders, panel frames, banner styles)
- 30+ impact particle variations (kinetic / thermal / electric / chemical)
- 12 mission briefing comic panels (1 per launch scenario)
- All assets AI-generated + ledgered + regenerable

### Content roster at M4B

| Content | Roster |
|---|---|
| **SFX** (toward 400+) | 200 cumulative (M4B adds button hover / click punch / banner slide / hit-stop / reload ding / weapon swap whoosh / pickup glow / 30+ impact variations) |
| **Music** (toward 30+ tracks) | 8 launch ambient tracks (per scenario) |

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-render-2d::comic_noir` | NEW | Comic-noir styling + ink-line shaders |
| `cf-render-2d::juice` | NEW | Per-element juice rules (hover, click, swap, hit-stop) |
| `cf-ui::animation` | NEW | UI panel transitions + reduce_motion respect |
| `cf-ui::comic_panels` | NEW | Comic-book-style mission briefing + death recap |
| `cf-replay` | MODIFY | ux.juice_applied events |

## Acceptance criteria

```gherkin
Scenario: Comic-noir UI renders
  Given M4B active + any scenario
  Then HUD borders use ink-line shader
  And impact particles use ink-style brushes
  And per-actor outlines (selective)

Scenario: Button hover juice
  Given a focusable button
  When cursor hovers:
    Then scale animates 1.0 → 1.05 over 80ms ease-out
    And glow halo appears
    And tick SFX plays
  When reduce_motion=true:
    Then animation skipped (instant 1.05)

Scenario: Click punch juice
  When button clicked:
    Then scale 1.0 → 0.95 → 1.0 over 120ms
    And brighter flash + click SFX
  When reduce_motion=true: skipped

Scenario: Banner slide-in
  Given new banner pushed
  Then banner slides from right edge over 200ms ease-in-out
  When reduce_motion=true: instant appear

Scenario: Mission briefing comic panel
  Given mission start
  Then comic_panel mission_briefing renders (hand-drawn art + text)
  And player can dismiss (Space) or auto-advances after 5s

Scenario: Death recap as graphic novel
  Given player death
  Then death recap renders as 4-panel comic with cause chain
  And player can navigate panels (Left/Right arrows)

Scenario: All juice respects accessibility
  Given Settings.reduce_motion=true + reduce_flash=true + reduce_shake=true
  Then every juice rule respects the flags
  And no juice violates the accessibility floor
```

## Dependencies

- M4A (must close) — base HUD readability layer
- M3B (must close) — death recap consumes comic panels
- M2.5 (must close) — reactor scenarios use comic briefings

## Closure procedure

Reference bundle + 8 sweep rows (visual variants + accessibility-suppressed) + DR-019 closure note. PASS.
