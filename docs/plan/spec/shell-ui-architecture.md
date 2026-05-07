---
type: spec
status: closed-direction
authority: "Shell UI: title, main menu, pause, settings, lobby, workbench, briefing, debrief, map, achievements, replay viewer, codex, photo mode, cosmetic locker, death cam, mod manager. Comic-noir presentation per DR-019. Flashy + punchy juice."
ready_when: "Every shell surface fully functional with cfctl parity per T-CONTROL; first-30-seconds friction <5% in playtest; accessibility ACC-A passes; localization Tier-A languages tested."
feeds:
  - DR-005
  - DR-009
  - DR-012
  - DR-019
  - DR-020
  - DR-024
  - DR-029
  - DR-031
  - DR-034
  - DR-039
  - DR-042
  - DR-046
  - DR-047
---

← [[spec/index|spec section]] · [[spec/visual-direction|visual direction]] · [[spec/audio-identity|audio]] · [[spec/equipment-loadout-workbench-slice-a|loadout workbench]] · [[spec/ai-control-observability-layer|cfctl/control]] · [[decisions/dr-046-player-facing-surfaces-direction|DR-046]]

# Shell UI Architecture

> [!summary] What this page is
> The full menu/UI shell architecture: every player-facing surface from launching the .exe to closing it. Every surface is comic-noir presentation per DR-019, flashy + punchy per DR-046, fully accessible per T-ACCESSIBILITY, fully scriptable via cfctl per T-CONTROL.

## Surface Tree

```
[App Launch]
  ↓
[Splash / Engine Boot] — 3s; AnimateDiff loop; legal disclaimers
  ↓
[Title Screen] — animated logo + parallax bg + "press start"
  ↓
[Main Menu] ←→ Settings / Credits / Quit
  ↓
[Profile Select] (multi-profile)
  ↓
[Hub] (campaign / skirmish / multiplayer / workshop / labs / tutorial)
  ↓
[Lobby] (pre-match config) → [Loadout Workbench] → [Briefing]
  ↓
[Match] ←→ [Pause Menu] ←→ Settings
  ↓
[Debrief] → [Replay Viewer] → [Highlight Reel] → [Hub]
```

## Surfaces

| Surface | Purpose | Done When |
|---|---|---|
| **Splash** | Studio logo + engine logo + legal | 3s; AnimateDiff cinematic loop. |
| **Title screen** | Game logo + "press start" + version + AI-cinematic bg | Hover to highlight; press → main menu transition. |
| **Main menu** | Hub of hubs | All 8 menu options work; comic-panel layout; transitions per DR-046 juice rules. |
| **Profile select** | Multi-profile per system | New / Load / Delete / Cloud-sync. |
| **Pause menu** | Resume / Save / Load / Settings / Restart / Quit | ESC opens; pauses sim deterministically per DR-002. |
| **Settings menu** | Graphics / Audio / Controls / Accessibility / Gameplay / Language / Online | All tabs functional; cfctl parity; settings persist; live-reload where possible. |
| **Server browser** | List + filter + favorites + history + direct-IP join | Per DR-005; Steam/EOS adapters optional. |
| **Lobby** | Pre-match config; team/faction/loadout/ready | Per DR-042 match grammar. |
| **Loadout workbench** | Drag/drop loadout builder | Per [[spec/equipment-loadout-workbench-slice-a]]; full Tier 3 polish. |
| **Mission briefing** | Comic-panel cards per [[spec/art-and-asset-pipeline]] | All 30+ launch missions have briefing. |
| **Mission debrief** | Comic-panel timeline + death recap + replay CTA + share button | Per DR-018 + DR-023. |
| **Strategic map** | Multi-world astrography per DR-039 | All 12 worlds visualized; faction state; comms light-lag. |
| **Hub UI** | Base / squad / campaign / mods / progression overview | All categories accessible. |
| **Replay viewer** | Scrub + speed + multi-cam + bookmark + clip export | Per DR-002. |
| **Codex / lore browser** | In-game encyclopedia | All factions/worlds/characters/weapons/materials unlockable + browsable. |
| **Photo mode** | Free camera + freeze + filters + screenshot export | Per DR-063 streaming/creator features. |
| **Cosmetic locker** | Unlocked skins/decals/paint/voice/emblems | Earned via play (never paid per DR-031). |
| **Achievements** | List + per-achievement unlock animation | 60-100 achievements at launch. |
| **Death cam** | Auto-replay last 5s on death | "Show me why" handoff per DR-023. |
| **Mod manager** | Browse Workshop / Local / Subscribe / Install / Update / Uninstall | Trust tiers per DR-034. |
| **Workshop submission** | One-button mod publish from in-game | Per DR-006 + DR-059. |
| **Difficulty / accessibility presets** | Standard/Easy/Hard/Custom + sliders | Per DR-023 + DR-012. |

## Settings Tree (Detail)

### Graphics
- Resolution, fullscreen/windowed/borderless, V-Sync, FPS cap (30/60/120/144/unlimited), quality preset (Steam Deck/Low/Med/High/Ultra/Custom), shader cache regeneration, particle density, decal density, shadow quality (off/low/med/high), normal-map quality, HDR (if monitor supports), color blind filter (Deuteranope/Protanope/Tritanope/None), screen shake amount (0-200%), camera shake (0-200%), film grain (off/light/medium), chromatic aberration (off/light/medium), bloom (off/low/med/high), gamma slider, brightness slider.

### Audio
- Master volume, music volume, SFX volume, voice (NPC) volume, voice (radio) volume, ambient volume, UI volume, output device selector, audio quality (low/med/high), spatial audio toggle, 3D voice toggle (Steam Audio per DR-043), captions (on/off/forced), caption size (small/medium/large/X-large), caption color, caption background opacity.

### Controls
- Keyboard remapping per action (full remap; per-context bindings), mouse sensitivity, mouse smoothing, controller deadzone, controller sensitivity (X/Y separate), invert Y, vibration toggle, vibration intensity, controller-bindings preset (Xbox/PS/Steam Deck/Custom), keybind import/export, menu input (KB/M ↔ Controller hot-swap auto-detect).

### Accessibility (per DR-012)
- UI scale (100/125/150/175/200%), high contrast mode, reduce motion, reduce shake, reduce flash, screen reader mode, one-handed mode, slow-down on input (0-200%), pause on focus loss, large pointer, focus indicators (high contrast outlines), captions style (above), font size, font choice (default/dyslexic/monospace).

### Gameplay
- Difficulty preset, autosave frequency (per minute / per phase / off), autosave slot count, ironman toggle, hint frequency (high/med/low/off), tutorial tooltips toggle (re-enable per category), confirmations on destructive actions, friendly fire policy (per scenario manifest if not overridden), camera mode default (side/tactical/replay-scrub), HUD density (low/med/high), HUD positioning (left/center/right side), aim assist (off/low/med/high; multiplayer-aware policy).

### Language
- Locale switcher (Tier-A languages full; Tier-B UI only); subtitle language; caption language; speech-to-text toggle (for streamer hearing-impaired support).

### Online
- Connection mode (auto/IPv6/IPv4), region preference, server browser filters defaults, cross-play toggle (Steam ↔ EOS), telemetry opt-in/opt-out (default per DR-047 region), crash report opt-in, Steam Workshop auto-update, mod trust tier max.

## Juice Rules (Per DR-046)

| Element | Behavior |
|---|---|
| Button hover | Scale 1.0→1.05 over 80ms ease-out + glow halo + soft tick SFX. |
| Button click | Scale punch + flash + click SFX (mid-frequency punch + sub-bass thump). |
| Menu transition | Comic-panel slide-in + skew + 200ms ease-in-out + ambient mix duck. |
| Settings save | Soft confirmation tick + animated value snap. |
| Loadout drag | Cursor follow + slot-glow on valid drop targets. |
| Loadout drop | Snap-in + bass thump + slot-flash. |
| Mission start | Dropship cinematic 4s + LZ flash + objective banner. |
| Mission victory | Comic-page-flip + slow-mo + music swell + confetti VFX. |
| Mission defeat | Scroll-of-failure + dirge. |
| Death | Slow-mo 0.3s + camera dolly + "show me why" prompt. |
| Achievement | Comic-panel pop-in + cheer sting. |
| Cosmetic unlock | Reveal animation + lights + cheer. |

## cfctl Parity (T-CONTROL)

Every shell surface MUST be controllable from cfctl per [[spec/ai-control-observability-layer]]:

- `cfctl observe --hud` — current HUD state
- `cfctl observe --settings` — settings tree dump
- `cfctl act settings set <key> <value>` — change setting
- `cfctl act keybind <action> <key>` — remap
- `cfctl ui select <id>` — focus / activate
- `cfctl ui type <text>` — text input
- `cfctl ui assert <id> <prop> <op> <value>` — UI test
- `cfctl observe --captions` — caption queue
- `cfctl observe --cinematic` — current cinematic state

## Performance Budget

| Surface | Frame budget |
|---|---|
| In-match HUD | < 1ms |
| Pause menu overlay | < 1ms |
| Main menu (full screen) | < 4ms |
| Loadout workbench | < 4ms |
| Briefing/debrief comic panels | < 8ms |
| Map view | < 8ms |
| Replay viewer with sim playback | < 16ms |

## Done-Criteria

- [ ] All surfaces implemented + accessible.
- [ ] Settings persist + live-reload where possible.
- [ ] cfctl parity verified across every surface.
- [ ] First-30-seconds friction <5% in playtest cohort.
- [ ] ACC-A acceptance passes per surface.
- [ ] Tier-A localization tested + community-reviewed.
- [ ] Juice rules audited for consistency + responsiveness.
- [ ] Steam Deck performance budget met.

## Source Trail

- [[decisions/dr-046-player-facing-surfaces-direction]]
- [[decisions/dr-019-visual-direction]]
- [[decisions/dr-012-accessibility-comfort-readability]]
- [[spec/ai-control-observability-layer]]
