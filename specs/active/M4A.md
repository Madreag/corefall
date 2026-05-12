# M4A — Readability + ACC-A Floor

## Status

`active`

## Intent

Game state is readable from the HUD without text walls. Body silhouette + module strip + ammo + objective + timer + last-event ticker render with material overlay integration and tool-validity color cues. Accessibility floor (DR-012) is hit: 200% text scale + reflow, high-contrast mode, color-independent state labels, controller route through HUD, remap holds (hold-to-confirm), captions surface.

## Player-facing behavior

- HUD has dedicated zones: top-left status (HP, stance, stability), top-right ammo + selected item, bottom-center objective banner + countdown timer, bottom-right last-event ticker, left edge body silhouette + module strip.
- Status banners pop on chassis events: "ARMOR CRACKED LEFT", "JET FAILED", "EJECT NOW" (text-only at M4A; comic-noir styling lands at M4B).
- Material overlay UI is integrated; toggling it tints the world AND adds a legend to the HUD.
- Tool-validity color cue: when the player aims the digger at a non-diggable surface, the reticle/cursor turns red with a small refusal label.
- Accessibility: 200% text scale toggleable; high-contrast mode toggleable; every state has a text label (no color-only meaning); controller D-pad routes through HUD focus traversal; key remap is validated and holds across sessions; captions surface for all important audio/event cues.
- All the above are reachable through `cfctl act.input.focus`, `act.settings.set`, and `observe.accessibility.*`.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-ui` | MODIFY | HUD layout: silhouette zone, module strip, ammo, objective banner, timer, last-event ticker. Status banner stack with severity tag (info/warning/critical). 200% scale reflow logic. High-contrast palette. Color-independent state labels. Material overlay legend. Tool-validity reticle. |
| `cf-control` | MODIFY | `act.input.focus <direction>` (left/right/up/down/next/prev). `act.settings.set` with key remap validation (rejects unknown actions/keys with reason). `observe.accessibility.*` surface (ui_scale_applied, contrast_mode, captions_enabled, reduced_motion, key_bindings). `observe.actor.silhouette` (per-zone HP%). `observe.actor.module_strip` (per-module state). |
| `cf-actor` | MODIFY | `BodySilhouette` projection (head/torso/arms/legs HP%). Stance enum extended (Crouching, Climbing, Jetting, Ejecting, KnockedDown). `Stance::from_chassis` derivation. |
| `cf-replay` | MODIFY | Event categories: `ux.*` (banner_raised/dismissed, focus_moved, captions_shown), `accessibility.*` (settings_changed, ui_scale_applied, contrast_mode_toggled). |
| `cf-app` | MODIFY | Bind keyboard/controller to focus traversal; bind 200% scale + high-contrast to settings patches; render captions overlay. |

## Files

- `game/crates/cf-ui/src/lib.rs` (MODIFY)
- `game/crates/cf-ui/src/hud.rs` (NEW or MODIFY: zoned layout)
- `game/crates/cf-ui/src/silhouette.rs` (NEW)
- `game/crates/cf-ui/src/banners.rs` (NEW: status banner stack)
- `game/crates/cf-ui/src/captions.rs` (NEW)
- `game/crates/cf-ui/src/contrast.rs` (NEW: high-contrast palette)
- `game/crates/cf-control/src/server.rs` (MODIFY: act.input.focus, settings remap validation)
- `game/crates/cf-control/src/settings.rs` (MODIFY)
- `game/crates/cf-actor/src/lib.rs` (MODIFY: BodySilhouette + Stance extensions)
- `game/crates/cf-replay/src/lib.rs` (MODIFY: ux/accessibility categories)
- `game/crates/cf-app/src/main.rs` (MODIFY: focus + scale + contrast + captions)
- `game/scripts/cfctl/m4a_acc_a_floor.cfctl.json` (EXISTS)
- `game/scripts/cfctl/m4a_micro_breach_readability.cfctl.json` (EXISTS)

## Acceptance criteria

```gherkin
Scenario: HUD layout is zoned
  Given an active scenario
  Then the HUD has dedicated zones for: status (top-left), ammo+item (top-right), objective+timer (bottom-center), last-event (bottom-right), silhouette+module-strip (left edge)
  And no zone overlaps another at default scale

Scenario: 200% text scale doesn't break layout
  Given the player toggles ui_scale=2.0
  When the HUD re-renders
  Then no text overflows its zone
  And no overlapping happens between status / ammo / objective / silhouette zones
  And the playable area remains visible

Scenario: High-contrast mode swaps palette
  Given the player toggles contrast_mode=high
  Then HUD text/borders use a high-contrast palette
  And event banners remain readable
  And accessibility.contrast_mode_toggled fires

Scenario: Color-independent state labels
  Given an actor in DOWNED status
  Then the HUD shows the literal text "DOWNED" (not just a red icon)
  And every state surface has a text label

Scenario: Body silhouette per-zone HP
  Given a chassis actor with head=80%, torso=100%, arm_left=40% HP
  Then the HUD silhouette tints each zone proportionally
  And cfctl observe.actor.silhouette returns {head:0.8, torso:1.0, arm_left:0.4, ...}

Scenario: Module strip shows per-module state
  Given a chassis actor with jet=Failed, shield=Warning, weapon_mount=Nominal
  Then the HUD module strip shows JET (FAIL), SHIELD (WARN), WEAPON (OK)
  And cfctl observe.actor.module_strip returns the same per-module state

Scenario: Status banner raises on chassis stage change
  Given a chassis actor going from Nominal to Degraded
  Then a status banner "ARMOR CRACKED" appears in the banner stack
  And ux.banner_raised fires with severity=warning, text="ARMOR CRACKED"
  When the actor stage advances to Disabled
  Then a critical banner "EJECT NOW" appears

Scenario: Focus traversal via cfctl
  Given the HUD has focusable elements
  When cfctl act.input.focus direction=next runs 5 times
  Then the focused element advances each time
  And ux.focus_moved fires with the previous + next focus target

Scenario: Captions surface for important cues
  Given an audible event (gunshot, reload click, breach hit)
  Then a caption appears briefly in the captions overlay
  And ux.captions_shown fires with the caption text

Scenario: Key remap validates unknown bindings
  Given cfctl sends act.settings.set with key_bindings={"unknown_action": "KeyZ"}
  Then the engine rejects with reason="unknown_action"
  When cfctl sends key_bindings={"fire": "InvalidKey"}
  Then the engine rejects with reason="invalid_key_code"

Scenario: Hold-to-confirm threshold for accessibility
  Given hold_threshold_ms=500
  When the player presses a destructive action (eject, scenario.reset)
  Then the action only fires after 500ms of hold
  And captions show "Hold to confirm" during the wait

Scenario: Settings round-trip via observe
  Given the player sets ui_scale=2.0, contrast_mode=high, captions_enabled=true
  When cfctl observe.accessibility runs
  Then the response includes ui_scale_applied=2.0, contrast_mode=high, captions_enabled=true
  And run_manifest.json.settings reflects the same patch
  And the patch persists across scenario.reset
```

## Out of scope

- Comic-noir mission card styling — M4B (deferred to BP7)
- DR-019 visual-direction closure — M4B
- DR-009 command UX (slowdown overlay, tactical map) — M4B
- True SDF/MSDF text rendering — BP6+ engineering (TTF scaling at M4A is the floor)
- Real-player accessibility playtest — owner-gated (AI Self-Test is primary gate)

## Dependencies

- M1 + M1.5 + M2 + M2.5 (must be done): the data the HUD reads.
- M5 chassis (must be done OR concurrent): the silhouette + module strip read from chassis state.

## Notes for the implementer

- 200% scale uses Bevy 0.18.1 ab_glyph TTF — works for M4A. SDF/MSDF is BP6+ engineering.
- Banner stack has severity tags (info/warning/critical); render order = critical bottom (most visible).
- captions_enabled default = false; captions are an opt-in surface, not always-on. The accessibility audit is for users who turn it on.
- Key remap validation uses an explicit allowlist of action names + key codes; reject anything not in the list with a structured reason.
- Hold-to-confirm threshold is per-action; defaults are documented in cf-control settings.
