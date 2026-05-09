---
type: decision
id: DR-012
status: open
priority: P1
revisit_trigger: "Slice A HUD, loadout/workbench, replay/death recap, hub, or package-builder screens fail accessibility/comfort tests at 200 percent text scale, keyboard/controller navigation, caption coverage, contrast, no-color-only state, reduced motion, or flash-safety checks."
---

← [[decisions/index|decision records]] · [[dashboards/decision-tracker|decision tracker]] · [[dashboards/research-readiness|readiness]] · [[spec/accessibility-comfort-slice-a|accessibility/comfort Slice A]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]]

# DR-012: Accessibility, Comfort, And Readability Floor

> [!info] Status: OPEN; LEAN: accessibility is a Slice A design floor, not a late compliance pass.

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
