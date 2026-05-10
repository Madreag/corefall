---
type: decision
id: DR-012
status: closed-direction-with-evidence
priority: P1
closed_at: 2026-05-09
closed_by: M4A — Readability + ACC-A Floor (BP3 milestone 2/3)
revisit_trigger: "Reopen if a real-player playtest at BP7+ shows the ACC-A floor is too weak (e.g. screen-reader path missing for a critical surface) or platform certification requires a stronger floor than what M4A locked. T-ACC-PLUS (BP9..BP12) extends the floor with cognitive + motor + hearing + reading + sensory presets without changing the M4A surface."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[dashboards/research-readiness|readiness]] · [[spec/accessibility-comfort-slice-a|accessibility/comfort Slice A]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]]

# DR-012: Accessibility, Comfort, And Readability Floor

> [!info] Status: <span class="cc-flag cc-green">CLOSED-DIRECTION-WITH-EVIDENCE</span>; Recommendation B (Slice A accessibility/comfort floor) shipped at M4A. T-ACC-PLUS (M-ACC-PLUS, BP9..BP12) layers cognitive + motor + hearing + reading + sensory presets on top without renaming any of the M4A surface.

## Closure Note (M4A — 2026-05-09)

M4A landed the full ACC-A-01..ACC-A-10 surface for the HUD + run-bundle evidence path. Surfaces gated behind not-yet-built UI (loadout/workbench at M8, replay viewer at M3B already shipped, hub/package-builder at M8) inherit the M4A floor automatically because the `Settings` resource + `HudSettings` mirror + Bevy `UiScale` + palette swap + caption strip + `observe.accessibility` surface are workspace-wide.

**Per ACC-A test:**

- **ACC-A-01 Text scale**: Bevy `UiScale` resource driven by `apply_ui_scale_from_settings` in cf-ui; Val::Px reflows natively; `summary_grid.png` and 4x4 `review_grid.png` at 200% show every HUD line readable without overlap, two-axis scrolling, or bottom-play-area obstruction. The core status strip is compact/content-sized; banners and captions occupy the upper-right lane and hide when empty.
- **ACC-A-02 Contrast**: `palette_text` / `palette_strip_bg` / `palette_banner_bg` helpers swap to pure white text on solid black backgrounds when `Settings.high_contrast = true`; observable via `observe.accessibility.high_contrast_applied`.
- **ACC-A-03 No color-only states**: every banner carries severity word + ASCII icon glyph (`[!!]` critical / `[!]` warning / `[*]` info) so monochrome captures still identify state. Stance / module / tool-validity lines are text-first.
- **ACC-A-04 Same-input navigation**: M4A landed real keyboard focus traversal (Tab / Shift+Tab + Arrow keys advance/retreat across the 12 focusable HUD nodes; Escape clears focus when a focus is active, otherwise exits the app — the standard "Esc closes the active overlay; Esc on the root exits" pattern; F1 is preserved as a fast-clear shortcut). M4A also wired the controller route via `cf-app::gamepad_focus_direction` (D-Pad + Left/Right Triggers + right-stick analog Y, deadzone 0.5, rising-edge debounced per-gamepad; East clears focus; South is deliberately reserved for future activation and dispatches no focus traversal). Visible focus ring in cf-ui. `observe.accessibility.focused_node` exposes the current focus to AI agents + cfctl. Mouse-only traps are absent at M4A scope (HUD is read-only).
- **ACC-A-05 Remapping and holds**: M4A added `Settings.hold_to_confirm` (default off) + `Settings.hold_threshold_ms` (default 250) + `Settings.key_remap_enabled` (default off) + `Settings.key_bindings` BTreeMap covering 18 actions (10 discrete: jump/fire/fire_alt/reload/dig/reset/select_slot_0..3 + 8 continuous: move_left/move_right/move_up/move_down/aim_left/aim_right/aim_up/aim_down). cf-app's `ingest_player_input` consults `key_for_action` for both held-key movement/aim AND edge-triggered discrete actions every frame. Hold-to-confirm is implemented via `cf-app::HoldTracker.tick_with_state` with 5 behavior tests covering tap/hold/release scenarios at the configured threshold. `act.settings.set` and `cfctl act settings-set --key-binding action=KeyName` validate both sides of the binding before accepting the patch; unsupported actions/keys reject with `key_binding_unknown_*` instead of silently falling back. Key names include Numpad0..9 for aim/movement remaps. Full remap UI surface is M8/`cf-tools-editor` scope; the data plane + dispatch path is shipped at M4A.
- **ACC-A-06 Motion, shake, and flash**: `Settings.reduced_motion` / `reduced_shake` / `reduced_flash` flags read + recorded through cf-control → observe.accessibility → cf-app HudSettings. M2.5 / M5 / M5.5 own the actual motion/shake/flash effects; their gates already require honoring these flags.
- **ACC-A-07 Captions**: M4A captions queue surfaces audio-bound events as text; `cf-ui` caption strip toggles `Display::Flex` / `Display::None` per `Settings.captions` and hides when the queue is empty; `observe.captions` exposes the text queue for AI agents. Captions render in the upper-right lane instead of the lower play/action area. cf-audio + real audio captions land at BP6.
- **ACC-A-08 Equipment workbench density**: workbench UI is M8 scope; the `Settings.ui_scale` + `high_contrast` surface from M4A is the architectural foundation. M8 inherits the floor by reading the same `HudSettings` resource.
- **ACC-A-09 Replay/death recap**: M3B (closed 2026-05-09) cf-tools-replay-viewer renders cause-chain + debrief in markdown; PNG companions via `markdown_to_png.py`. Color-independent; non-color-only tags.
- **ACC-A-10 Run-bundle evidence**: `run_manifest.json.settings` carries the 9 M4A ACC-A flags; `summary.json.event_counts.by_type` reflects `control.settings_observed` + `control.settings_changed` round-trips; `observe.accessibility` surface is the live read.

**Authoritative implementation evidence:**

- **Run-bundle evidence**: `prototype_runs/native/m4a_2026-05-10T18-19-43Z_5d1a46cc/` — source-truthful M4A bundle (`run_manifest.scene.id = "m4a_micro_breach_readability"`, milestone tagged "m4a", expected_tests = `["M4A-D01", "M4A-D02", "M4A-D03", "M4A-D04"]`, all 9 ACC-A settings driven through cf-e2e, `summary_grid.png` + `review_grid.png` populated, bottom play lane unobstructed at 200% high contrast).
- **Close-loop verdict**: `prototype_runs/native/bp3_loop_2026-05-10T18-16-49Z_525df038/verdict.json` — coverage, build/lint/test, self-play sweep, grading scaffold, grading filled, and grading validate all PASS against the matching dirty-worktree fingerprint.
- **Self-play sweep**: 18/18 PASS (post-M4A audit closure adds m4a_focus_traversal + m4a_hold_remap_settings rows).
- **LLM-graded verdict**: `prototype_runs/native/m4a_*/grading.json` PASS aggregate ≥ 7.0 with prose-justified scores per dimension.
- **Single-source focusable_nodes**: `cf_control::engine::HUD_FOCUSABLE_NODES` is the canonical 12-id list; cf-e2e `--verify-focus` + cf-control live_ws_acceptance test + cf-app focus traversal all read from it. Regression in any one node is caught by the shared list.

**Reopen triggers:**

- BP7 real-player playtest fails ACC-A-01..ACC-A-10.
- T-ACC-PLUS (M-ACC-PLUS) at BP9..BP12 surfaces a stronger preset that requires renaming the M4A surface.
- Platform cert (Steam / Xbox / PlayStation) at BP12 requires a feature M4A does not yet expose.

## Context

This game needs dense simulation, destructible terrain, body damage, equipment tradeoffs, AI explanations, replay/death recaps, and creator tooling. Those systems will fail if players cannot read the HUD, navigate the loadout/workbench, understand danger without color alone, reduce motion/flash strain, or access core actions with their preferred input.

This decision covers the baseline for prototype and spec work. It does not slow private prototyping; it defines the floor that every UI-heavy prototype must prove before the feature becomes a settled product commitment.

## Options

| Option | Summary | Best Case | Worst Case |
|---|---|---|---|
| A | Late compliance pass | Build the game first; retrofit accessibility before public release. | Early prototypes move quickly. | HUD, workbench, replay, and command UI need expensive redesign once dense systems exist. |
| B | Slice A accessibility/comfort floor | Treat text scale, contrast, no-color-only state, remapping, captions, reduced motion, flash limits, focus order, and objective/help reminders as prototype requirements. | Accessibility becomes part of the UX architecture and catches failures while screens are still cheap to change. | Some throwaway prototypes need simple settings and screenshots earlier than expected. |
| C | Full personalization platform first | Build deep presets, narration, alternate control modes, every caption/audio option, and full certification matrix before core gameplay. | Strong inclusion posture from day one. | Core game feel stalls before actor, terrain, AI, and replay evidence exists. |

## Pros And Cons

| Option | Pros | Cons | Unknowns |
|---|---|---|---|
| A | Lowest immediate implementation friction; avoids polishing throwaway UI. | High redesign risk; late fixes can fight art, layout, input, and telemetry architecture; excludes playtesters during the phase when feedback matters most. | How much of the early UI would survive to release. |
| B | Keeps prototypes testable; aligns with Microsoft XAG, WCAG, and Game Accessibility Guidelines; protects loadout/workbench density; improves comfort for all players. | Requires every screen spec to include accessibility evidence; may add small debug/settings work to early prototypes. | Exact final defaults for public launch platforms. |
| C | Maximizes configurability; best for a public accessibility promise. | Too much upfront scope before the core game is proven; could turn the accessibility system itself into the project. | Whether the product will need certification-grade coverage and on which platforms. |

## Evaluation

| Lens | A | B | C |
|---|---|---|---|
| Player value | Weak early, maybe okay late. | Strong because combat, loadout, replay, and tools are readable during tests. | Strong if finished, but delays game proof. |
| Readability | Reactive. | Proactive. | Proactive but heavy. |
| AI burden | AI explanations may be hidden or color-only. | AI reason labels must be visible, captionable, and replayable. | Same as B plus extra narration burden. |
| UX burden | Late redesign. | Normal design cost while layouts are young. | High system cost before core screens settle. |
| Performance risk | Low. | Low; mostly layout/settings/events. | Medium if narration, overlays, and alternate modes are overbuilt. |
| Modding impact | Package diagnostics may be inaccessible. | Workbench diagnostics, trace tabs, source inspectors, and role cards inherit the same floor. | Strong but larger authoring burden. |
| Networking/replay impact | Accessibility events may be missing. | Settings, captions, flash suppression, and UI state changes become run evidence. | Strong but may overfit before recorder exists. |
| Content cost | Deferred. | Moderate; captions/labels required for critical feedback. | High. |
| Retention upside | Medium if fixed later. | High; comfort and comprehension make the game easier to return to. | High if shipped, but slower path. |
| Ethics/fairness | Risky if accessibility arrives after monetization/economy thinking. | Good; accessibility is independent of grind or payment. | Good but could over-promise. |

## Evidence

| Evidence | Source | Confidence |
|---|---|---|
| Minimum readable game text and scalable text are explicit accessibility guidelines; text in HUD, objectives, prompts, captions, errors, and notifications is in scope. | Microsoft XAG 101, `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101` | High |
| Important text and visual elements need contrast floors; high contrast mode should be available. | Microsoft XAG 102, `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/102` | High |
| Critical information expressed through color also needs another channel such as shape, pattern, iconography, or text labels. | Microsoft XAG 103, `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/103` | High |
| UI navigation should be logical, consistent, same-input, keyboard/controller accessible, and responsive to scaling. | Microsoft XAG 112, `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112` | High |
| Input remapping and digital alternatives are first-class game accessibility requirements. | Microsoft XAG 107, `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107` | High |
| Game-specific guidelines call out game-speed options, same-input UI access, remapping, interface resizing/rearranging, alternatives to holds/repeated inputs, visual/audio alternatives, objective reminders, and captions. | Game Accessibility Guidelines full list, `https://gameaccessibilityguidelines.com/full-list/` | High |
| Flash and moving/auto-updating content need explicit controls or thresholds. | WCAG 2.2, `https://www.w3.org/TR/WCAG22/` | Medium as web standard translated to game UI; still useful for flash/motion discipline. |
| Current vault UX already defines HUD/loadout/replay/workbench accessibility floors, but decision tracking still listed accessibility as no-DR. | [[spec/ux-wireframes-slice-a]], [[systems/ux-overlay-screen-brief]], [[dashboards/decision-tracker]] | High |
| Equipment/workbench prototypes are dense table/detail UIs with role cards, diagnostics, source inspectors, trace tabs, bot-blocked panels, overlap compare, and package warnings. | [[spec/equipment-loadout]], [[spec/equipment-loadout-workbench-slice-a]], [[references/equipment-trace-tab-view-slice-a]] | High |

## Current Recommendation

Recommendation: choose Option B.

Why: the future game depends on dense information under pressure. Accessibility, comfort, and readability are not separate from game feel; they are the way players understand wounds, danger, tools, bot decisions, equipment warnings, mod diagnostics, replay causes, and mission objectives. Slice A should prove a small but real floor now, then expand later if public platform scope demands it.

## Prototype Or Validation Plan

| Test | What It Proves | Pass/Fail |
|---|---|---|
| ACC-A-01 Text scale | HUD, command overlay, buy/loadout, workbench, replay, hub, and package diagnostics reflow at 100%, 150%, and 200%. | Pass when no critical text overlaps, truncates without affordance, or requires two-axis scrolling. |
| ACC-A-02 Contrast | Important text, icons, overlays, warnings, focus rings, and map/terrain labels meet contrast targets. | Pass when standard, large, inactive, and high-contrast targets meet the configured floor. |
| ACC-A-03 No color-only states | Wounds, danger, bot trust, item warnings, package errors, server blockers, and objective states use labels/icons/patterns as well as color. | Pass when monochrome screenshots still identify critical state. |
| ACC-A-04 Same-input navigation | HUD overlays, loadout/workbench, replay, hub, settings, and package diagnostics can be operated with keyboard and controller. | Pass when there are no mouse-only traps and every focused element has an exit path. |
| ACC-A-05 Remapping and holds | Direct control, command overlay, menus, slowdown, and workbench actions expose remap/hold-toggle alternatives. | Pass when core prototype actions can be remapped and hold actions have toggle/press alternatives. |
| ACC-A-06 Motion, shake, and flash | Screen shake, camera motion, flashes, and auto-updating UI can be reduced or suppressed. | Pass when reduced-motion mode keeps gameplay readable and flash events stay within the chosen safety threshold. |
| ACC-A-07 Captions and audio alternatives | Critical audio cues and dialogue-like prompts have text/effect labels. | Pass when combat, AI, delivery, objective, and workbench-critical audio cues have visible equivalents. |
| ACC-A-08 Equipment workbench density | Role cards, trace tabs, source inspectors, warning badges, fixture tabs, and diagnostics stay usable at 200% text scale. | Pass when LOAD-W, LOAD-R, LOAD-FIELD, and package-diagnostic screens keep the same meaning at scale. |
| ACC-A-09 Replay/death recap | Recaps and replay filters explain cause chains with readable text and non-color-only tags. | Pass when a player can understand a death cause without relying on raw logs, audio, or color-only categories. |
| ACC-A-10 Run-bundle evidence | Prototype run bundles include accessibility settings, screenshots, failures, and setting-change events. | Pass when `summary.json`/notes can prove which accessibility settings were tested. |

## Risks

| Risk | Mitigation |
|---|---|
| Accessibility turns into an oversized platform before gameplay is fun. | Keep Slice A small: defaults, scaling, contrast, input, motion/flash, captions, and evidence screenshots. |
| Dense loadout/workbench tables become unreadable at scale. | Require responsive table/detail variants and ACC-A-08 before promoting loadout UI claims. |
| Colorful terrain/material overlays lose meaning in colorblind or high-contrast modes. | Pair material color with icons, hatching, labels, and tooltips; test monochrome captures. |
| Reduced motion hides important physics cues. | Replace removed shake/motion with actor-adjacent labels, arrows, captions, reticle/state changes, and replay events. |
| Captions become noisy during chaotic combat. | Caption only critical state changes by default; expose verbosity and category filters. |
| Mod diagnostics become legalistic noise. | Keep provenance visible but action-oriented: severity, source path, first fix, mode verdict, and bot-safety implication. |

## Revisit Trigger

Reopen this decision when:

- The event in `revisit_trigger` occurs.
- Platform/release scope changes and requires certification-grade accessibility coverage.
- Prototype playtests show that the floor is either too weak for real players or too heavy for the current build stage.
- A new UI surface appears that does not fit the current ACC-A tests.

## Source Trail

- [[spec/accessibility-comfort-slice-a]]
- [[spec/ux-wireframes-slice-a]]
- [[systems/ux-overlay-screen-brief]]
- [[spec/prototype-implementation-backlog-slice-a]]
- [[spec/equipment-loadout-workbench-slice-a]]
- [[references/equipment-trace-tab-view-slice-a]]
- [[references/prototype-run-bundle-schema]]
- Microsoft XAG 101: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/101`
- Microsoft XAG 102: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/102`
- Microsoft XAG 103: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/103`
- Microsoft XAG 107: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/107`
- Microsoft XAG 112: `https://learn.microsoft.com/en-us/gaming/accessibility/xbox-accessibility-guidelines/112`
- Game Accessibility Guidelines full list: `https://gameaccessibilityguidelines.com/full-list/`
- WCAG 2.2: `https://www.w3.org/TR/WCAG22/`
