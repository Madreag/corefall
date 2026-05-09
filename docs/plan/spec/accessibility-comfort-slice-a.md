---
type: spec
status: prototype-reqs
ready_when: "ACC-A-01..ACC-A-16 pass across HUD, command, buy/loadout, equipment workbench, replay/death recap, hub, package-builder, and settings/accessibility surfaces, with screenshots and run-bundle evidence."
feeds:
  - DR-003
  - DR-004
  - DR-006
  - DR-008
  - DR-009
  - DR-012
---

← [[spec/index|spec section]] · [[decisions/dr-012-accessibility-comfort-readability|DR-012]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]] · [[systems/ux-overlay-screen-brief|UX overlay brief]] · [[spec/prototype-implementation-backlog-slice-a|implementation backlog]] · [[references/prototype-run-bundle-schema|run-bundle schema]] · [[spec/equipment-loadout-workbench-slice-a|equipment workbench]] · [[spec/equipment-role-card-renderer-slice-a|role-card renderer]] · [[spec/replay-recorder-slice-a|replay recorder]] · [[spec/backend-service-hub-slice-a|backend/hub]] · [[spec/package-builder-workbench-slice-a|package-builder]]

# Accessibility And Comfort Slice A

> [!summary] Purpose
> Define the build-facing accessibility, comfort, and readability floor for the first playable prototypes. This page is not a final platform-certification checklist. It is the minimum contract that keeps dense Cortex-like simulation readable, navigable, and testable while actor feel, AI, equipment, replay, missions, modding, and backend work are still evolving.

> [!important] Product stance
> Accessibility is combat readability, input reliability, comfort, and trust. The goal is not to make the game shallow; the goal is to ensure wounds, terrain danger, AI failures, loadout warnings, package diagnostics, and replay causes are understandable without perfect eyesight, color perception, hearing, motor precision, or tolerance for shake/flash.

## Slice A Question

Can a player read, navigate, and understand the prototype across combat HUD, command overlay, loadout/workbench, replay/death recap, hub, and package diagnostics at normal and high-scale settings without relying on color alone, mouse-only controls, audio-only cues, or high-motion/high-flash feedback?

## Accessibility Surface Matrix

| Surface | Minimum Slice A Requirement | Evidence To Capture |
|---|---|---|
| Tactical HUD | Actor status, wound/state labels, current item, ammo/cooldown, danger, order, and last critical event remain readable at 100%, 150%, and 200% scale. | ACC-A-01/02/03 screenshots; UX-W-01..03 notes. |
| Command overlay | Order wheel/list, route preview, blocker reason, slowdown state, confirm/cancel prompts, and path danger use same-input navigation and non-color-only labels. | ACC-A-04/05 screenshot plus keyboard/controller route. |
| Material/path overlay | Material state uses color plus hatching/icon/label; hazards use shape and captions; reduced-motion mode preserves danger meaning. | ACC-A-03/06 plus MAT-T overlay capture. |
| Buy/loadout | Role filters, item rows, cost/mass/bot-skill/terrain fit/warnings/delivery risk remain readable and sortable at 200% scale. | ACC-A-08, UX-W-06/07, LOAD-A evidence. |
| Equipment workbench | Trace tab, source inspector, role-card detail drawer, package diagnostics, overlap compare, bot trust panel, and fixture tabs reflow without meaning loss. | ACC-A-08, LOAD-W-010, LOAD-FIELD, LOAD-FIELD-SOURCE evidence. |
| Replay/death recap | Cause chain, event categories, timeline, AI labels, equipment consequence, and unknown-cause fallback are text-visible and non-color-only. | ACC-A-09, REC-A-04, UX-W-09/10 evidence. |
| Hub/server browser | Server rows, join blockers, package compatibility, local supervisor state, and health indicators expose exact reason and next action. | ACC-A-01/03/04 plus BACK-A join-blocker evidence. |
| Package builder | Diagnostics table, source path, include stack, severity, first fix, mode verdict, provenance, and test-launch result are accessible by keyboard/controller. | ACC-A-08, PACK-A, PACK-014C/D evidence. |
| Settings/accessibility | Text scale, contrast mode, remap, hold/toggle, captions, motion/shake, flash reduction, audio mix, objective reminders, and reset/profile controls are reachable from first-run and pause/hub. | ACC-A-04/05/06/07 screenshots and event log. |

## Settings Contract

| Setting | Values For Slice A | Applies To | Why It Exists |
|---|---|---|---|
| `text_scale` | 100%, 150%, 200% | HUD, command, loadout, workbench, replay, hub, settings. | Microsoft XAG 101 expects readable text and scalable UI; our dense tables need proof early. |
| `ui_density` | Compact, Comfortable | Loadout/workbench, hub tables, replay browser. | Lets dense tools stay useful without forcing one layout on all players. |
| `contrast_mode` | Standard, High Contrast Dark, High Contrast Light | All UI, overlays, captions, focus rings. | XAG 102 contrast targets; terrain backgrounds fluctuate heavily. |
| `color_cue_mode` | Default, Colorblind-safe, Monochrome test | HUD, material overlays, item warnings, server/package status. | Critical states cannot rely on color alone. |
| `caption_mode` | Critical only, Expanded, Off | AI, delivery, objective, combat alerts, workbench critical sounds. | Audio cues need visible alternatives; chaos needs category filters. |
| `caption_background` | Off, 50%, 80%, 100% opacity | Captions and critical event labels. | Keeps text readable over bright terrain/explosions. |
| `input_profile` | Keyboard/mouse, controller, keyboard-only, custom | Gameplay and UI. | Same-input navigation and remapping must include direct control and menus. |
| `remap_actions` | Gameplay, command, UI, replay, workbench groups | All input surfaces. | XAG 107 and game guidelines expect remapping, not platform-only remap. |
| `hold_behavior` | Hold, Toggle, Press-to-cycle | Dig, aim mode, command overlay, drag-like workbench actions. | Avoids required long holds and repeated strain. |
| `game_speed_assist` | Off, Slowdown75, Slowdown25, Pause in menus | Command, planning, accessibility profile. | Game-speed control is a comfort and comprehension tool, not a cheat in solo/prototype modes. |
| `screen_shake_scale` | 0%, 25%, 50%, 100% | Explosions, gibs, dropship impacts, weapon recoil. | Preserves feedback while reducing discomfort. |
| `camera_motion` | Reduced, Standard | Follow camera, recoil camera, replay camera. | Reduces sickness/disorientation while keeping state labels. |
| `flash_reduction` | On, Off | Explosions, muzzle flashes, alarm flashes, package error pulses. | Keeps flashes under the chosen safety threshold; use labels instead. |
| `objective_help` | Minimal, Standard, Verbose | Mission strip, HUD reminders, replay/debrief, workbench. | Current objective reminders reduce cognitive load in complex missions. |
| `debug_explainer_level` | Player, Designer, Raw | AI labels, equipment trace, package diagnostics. | Separates player readability from creator/debug detail without hiding cause. |

## Source-Aligned Floors

| Area | Slice A Floor | Source |
|---|---|---|
| Text size | Important PC 1080p text targets at least 18 px by default; console/TV targets at least 26 px where applicable; UI supports 200% scaling without loss of meaning. | Microsoft XAG 101 |
| Text reflow | Scaled text may scroll in one direction when needed, but no critical single UI block should require both horizontal and vertical scrolling. | Microsoft XAG 101, XAG 112 |
| Contrast | Important standard text/visuals target at least 4.5:1, large elements 3:1, inactive elements 3:1, and high-contrast elements 7:1. | Microsoft XAG 102 |
| Color alternatives | Critical color-coded information also uses shape, pattern, iconography, label, position, sound/haptic/caption, or other channel. | Microsoft XAG 103 |
| UI navigation | Focus order follows visual/operational meaning, repeated controls are stable, scaled layout updates focus order, and every screen has an obvious back path. | Microsoft XAG 112 |
| Input | UI and gameplay support keyboard/controller digital paths, remapping, sensitivity alternatives, and non-simultaneous actions where practical. | Microsoft XAG 107; Game Accessibility Guidelines |
| Motion/flash | Auto-moving/blinking/updating info has pause/stop/hide or frequency control where applicable; flashing avoids more-than-three-per-second unsafe patterns unless below threshold. | WCAG 2.2, translated to game UI and effects |
| Game speed | Solo/prototype flows include game-speed adjustment and pause/slowdown affordances. | Game Accessibility Guidelines |
| Audio alternatives | Critical audio cues get visible equivalents or captions; captions are readable over gameplay backgrounds. | Game Accessibility Guidelines; Microsoft game accessibility sources |

## Equipment And Workbench Requirements

The user's equipment reminder matters here: loadout/workbench UI will be one of the densest surfaces in the game. Accessibility requirements must cover the CCCP-derived field atlas and generated artifacts, not just generic menus.

| Equipment Surface | Required Accessibility Behavior | Linked Test |
|---|---|---|
| Catalog table | Role, cost, mass, bot skill, terrain fit, warnings, package source, and overlap state remain readable at 200% scale. | ACC-A-08, LOAD-R-01/02 |
| Role-card drawer | `best_at`, `bad_at`, handling, terrain consequence, AI policy, source/provenance, and replay/backend fields use section headers and keyboard focus regions. | ACC-A-08, LOAD-R-03 |
| Trace tab | Consumer gaps, source confidence, diagnostics, and open targets use icon+label badges, not only red/yellow/green. | ACC-A-03/08, LOAD-W-010 |
| Source inspector | Source path, module/include context, field provenance, warning id, and first fix action are copyable/selectable and navigable without mouse. | ACC-A-04/08, LOAD-FIELD-SOURCE-01..06 |
| Bot trust panel | Bot-safe/manual/risky/blocked labels include reason text and scenario id. | ACC-A-03/07/08, AI-H-LOAD, AI-EQ |
| Overlap compare | Duplicate-role pressure is visible as role split/skin/legacy/manual/mission-fixture labels. | ACC-A-03/08, LOAD-010 |
| Package diagnostics | Severity, mode verdict, bot assignment verdict, package path, source link, and first fix stay visible at scale. | ACC-A-08, PACK-014C/D |
| Export preview | Item ids, package hashes, warning ids, accessibility setting snapshot, and event labels appear in the run-bundle preview. | ACC-A-10, REC-A-LOAD |

## Runtime Event Contract

| Event | Required Fields | Consumers |
|---|---|---|
| `ux_accessibility_setting_changed` | setting id, old value, new value, source screen, input method, timestamp. | UX telemetry, run bundle, settings regression. |
| `ux_text_scale_applied` | scale, affected screen, reflow mode, overflow count, clipped count. | ACC-A-01, screenshots, layout tests. |
| `ux_contrast_mode_changed` | mode, palette id, surface, failed contrast count if measured. | ACC-A-02. |
| `ux_color_cue_audit` | screen, critical state count, color-only count, missing label ids. | ACC-A-03. |
| `ux_input_remap_changed` | action id, old binding, new binding, device class, conflict state. | ACC-A-05. |
| `ux_focus_path_tested` | screen, device, route id, trap count, back-path result. | ACC-A-04. |
| `ux_caption_shown` | caption id, event id, category, verbosity, duration, occlusion flag. | ACC-A-07, replay/death recap. |
| `ux_screen_shake_scaled` | event id, original magnitude, applied scale, replacement cue id. | ACC-A-06. |
| `ux_flash_suppressed` | event id, source effect, suppression reason, replacement cue id. | ACC-A-06. |
| `ux_motion_reduced` | camera/effect id, mode, applied alternative. | ACC-A-06. |
| `ux_objective_reminder_shown` | objective id, verbosity, trigger, player action. | ACC-A-07, mission comprehension. |

## Run-Bundle Evidence Additions

Future prototype run folders should include these accessibility fields alongside the existing manifest/events/summary files from [[references/prototype-run-bundle-schema]].

| Artifact | Required Accessibility Fields |
|---|---|
| `run_manifest.json` | Accessibility profile name, text scale, contrast mode, color cue mode, input profile, caption mode, shake scale, flash reduction, game-speed assist, build hash. |
| `events.jsonl` | Event families above plus cross-links to HUD, command, loadout, workbench, replay, hub, package, and mission events. |
| `summary.json` | ACC-A pass/fail rows, screenshots captured, overflow/clipped text count, focus trap count, color-only critical state count, caption coverage count, flash suppression count. |
| `notes.md` | Screenshot contact sheet, layout failures, player confusion notes, setting deltas, and next fixes. |
| `/screenshots/` | Normal and 200% captures for HUD, command, buy/loadout, equipment workbench, replay/death recap, hub, package builder, and settings. |

## ACC-A Acceptance Tests

| ID | Test | Pass Criteria |
|---|---|---|
| ACC-A-01 | Text scale and reflow | HUD, command, buy/loadout, equipment workbench, replay, hub, package builder, and settings remain usable at 100%, 150%, and 200% scale with no critical overlap or unannounced truncation. |
| ACC-A-02 | Contrast floor | Important text, icons, focus rings, warnings, minimap/overlay marks, and diagnostics meet configured contrast floors in standard and high-contrast modes. |
| ACC-A-03 | No color-only critical state | Wounds, danger, AI refusal, package error, item warning, objective state, server blocker, and terrain hazard are understandable in monochrome screenshots. |
| ACC-A-04 | Same-input navigation | Keyboard and controller can reach and leave every major UI region in HUD overlays, command, loadout, workbench, replay, hub, package builder, and settings. |
| ACC-A-05 | Remap and hold alternatives | Core gameplay, command, UI, replay, and workbench actions can be remapped; hold actions offer toggle or press alternatives where practical. |
| ACC-A-06 | Motion/shake/flash comfort | Reduced-motion mode suppresses or scales screen shake/camera motion/flashes and replaces lost meaning with labels, arrows, captions, or state changes. |
| ACC-A-07 | Captions and critical audio alternatives | Objective prompts, AI warnings, delivery danger, death causes, package errors, and combat-critical audio cues have visible equivalents. |
| ACC-A-08 | Equipment/workbench readability | Role cards, trace tabs, source inspector, diagnostics, overlap compare, bot trust, and export preview pass ACC-A-01..04. |
| ACC-A-09 | Replay/death recap readability | Cause chain and event filters are readable, keyboard/controller navigable, captioned where needed, and non-color-only. |
| ACC-A-10 | Hub/backend blocker readability | Disabled join/start buttons always explain exact reason and next action, including package/accessibility/profile blockers. |
| ACC-A-11 | Package diagnostics accessibility | Diagnostics can be sorted, filtered, opened, and traced to source without mouse-only actions; severity is not color-only. |
| ACC-A-12 | Objective and help reminders | Player can show current objective, controls, and next recommended action during gameplay and workbench flows without leaving the screen. |
| ACC-A-13 | Settings availability | Accessibility settings are reachable from first-run, hub, pause, and prototype debug menu. |
| ACC-A-14 | Settings persistence | Accessibility settings persist across restart and are recorded in run-bundle manifests. |
| ACC-A-15 | Player/debug separation | Player mode shows curated labels; designer/raw mode can expose extra trace detail without changing player-mode meaning. |
| ACC-A-16 | Regression evidence | Every ACC-A run exports screenshots, event counts, setting values, failures, and next fixes into the prototype run bundle. |

## First Tickets

| Order | Ticket | Done When |
|---|---|---|
| 1 | Add accessibility profile object to prototype config. | Text scale, contrast, color cue, input, caption, shake, flash, motion, and objective-help settings serialize into run manifests. |
| 2 | Add debug settings panel. | Settings are reachable by keyboard/controller and emit `ux_accessibility_setting_changed`. |
| 3 | Add screenshot capture matrix. | Normal and 200% captures are generated for HUD, command, buy/loadout, workbench, replay, hub, package builder, and settings. |
| 4 | Add no-color-only audit helper. | Critical states can be listed by screen with color-only count and missing label ids. |
| 5 | Add focus-path smoke tests. | Keyboard/controller route ids and focus trap count are exported for each major surface. |
| 6 | Wire equipment workbench ACC-A checks. | LOAD-W/LOAD-R/LOAD-FIELD evidence includes text scale, focus route, contrast, and no-color-only rows. |
| 7 | Wire captions to replay/death recap. | Critical audio and event labels show in live HUD and replay timeline with category filters. |
| 8 | Add motion/flash replacement cues. | Reduced mode turns shake/flash into labels/arrows/state changes and logs suppression events. |

## Non-Goals For Slice A

| Non-Goal | Why |
|---|---|
| Final public certification checklist | Platform scope and public-release targets are not settled. |
| Full screen narration implementation | Valuable later, but Slice A focuses on text, contrast, input, captions, motion, flash, and dense UI readability first. |
| Final visual identity | Accessibility colors and contrast modes can coexist with later art direction. |
| Full localization implementation | This page requires stable text ids and scalable layouts; localization gets its own plan/DR later. |
| Competitive accessibility policy | PvP/leaderboard fairness around assists belongs in future multiplayer/economy decisions. |

## Open Questions

| Question | Cheapest Test |
|---|---|
| Is 200% text scale enough for dense workbench tables, or do we need a dedicated comfortable layout? | Run ACC-A-08 on role-card catalog, trace tab, and diagnostics fixtures. |
| Should slowdown be framed as an accessibility setting, a tactical command feature, or both? | Run ORDER-01 and ACC-A-06 with slowdown labeled both ways. |
| How verbose should combat captions be by default? | Compare critical-only vs expanded captions in a chaotic actor-feel run. |
| Do terrain overlays need texture/hatching from day one? | Run monochrome captures of material/path overlay and see whether players still identify hazards/tool fit. |
| Should first-run accessibility setup appear before the main menu in private prototypes? | Try a simple first-run panel once the hub shell exists. |

## Source Trail

- [[decisions/dr-012-accessibility-comfort-readability]]
- [[spec/ux-wireframes-slice-a]]
- [[systems/ux-overlay-screen-brief]]
- [[spec/prototype-implementation-backlog-slice-a]]
- [[spec/equipment-loadout-workbench-slice-a]]
- [[references/equipment-trace-tab-view-slice-a]]
- [[references/equipment-role-card-renderer-view-slice-a]]
- [[references/equipment-package-diagnostics-slice-a]]
- [[references/prototype-run-bundle-schema]]
- Microsoft XAG 101: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101`
- Microsoft XAG 102: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/102`
- Microsoft XAG 103: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/103`
- Microsoft XAG 107: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107`
- Microsoft XAG 112: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112`
- Game Accessibility Guidelines full list: `https://gameaccessibilityguidelines.com/full-list/`
- WCAG 2.2: `https://www.w3.org/TR/WCAG22/`
