# cf-ui — AGENTS.md

## Owns
- **M1 status strip** (`StatusStripPlugin`): four-line text overlay pinned to the top-left.
  - STATUS line (stable / unstable / downed / dead).
  - ITEM line (slot number + label).
  - HP line (`X / 100`).
  - Reticle / fire-state line (`READY 30/30`, `RELOADING NN%`, `EMPTY`, `COOLDOWN Nt`, `NO RIFLE`).
- **M1.5 mission strip** (M1.5-004): OBJECTIVE / MISSION / ENEMY / BREACH / EVENT lines.
- **M4A readability + ACC-A floor** (M4A; DR-003 + DR-012 closure):
  - STANCE line (idle / walking / running / airborne / downed / dead) with optional `(airborne)` marker.
  - BODY silhouette line (per-zone hp%: head / torso / arms / legs) with `~` placeholder marker until M5.
  - MODS module-strip line (weapon_mount + jet/shield/sensor placeholder slots) with `~` placeholder marker.
  - TOOL line (VALID / REFUSED + reason + target).
  - Banner overlay strip (top-center, 4 slots): priority-ordered banner stack (critical > warning > info) with severity word + ASCII icon glyph (`[!!]` / `[!]` / `[*]`).
  - CAPTION strip (bottom-center, 3 slots): drained from `HudState.captions`; toggles `Display::None` when `Settings.captions == false`.
- `HudState { player, rifle, tick, tick_rate_hz, mission, enemy, breach, last_event, stance, body_silhouette, modules, banners, captions, tool_validity }` resource.
- `HudSettings { ui_scale, high_contrast, captions, reduced_motion, reduced_shake, reduced_flash }` resource (mirror of cf-control `Settings`).
- Formatter functions (unit-tested): `rifle_status_line`, `mission_line`, `objective_line`, `enemy_line`, `breach_line`, `stance_line`, `silhouette_line`, `module_line`, `tool_line`, `banner_line`.
- Bevy `UiScale` integration via `apply_ui_scale_from_settings` (200% scale reflows every Val::Px line natively).
- High-contrast palette swap via `palette_text` / `palette_strip_bg` / `palette_banner_bg` helpers + `update_palette_for_high_contrast` system.
- M4B (BP7) layers comic-noir mission cards + slide/skew styling on top of this same surface without changing `HudState` shape.

## Public API Boundary
- Plugin: `StatusStripPlugin`.
- Resources: `HudState`, `HudSettings`.
- Helper structs: `HudRifle`, `HudMission`, `HudEnemy`, `HudBreach`, `HudBodySilhouette`, `HudModuleStrip`, `HudModule`, `HudBanner`, `HudCaption`, `HudToolValidity`.
- Formatter functions: `rifle_status_line`, `mission_line`, `objective_line`, `enemy_line`, `breach_line`, `stance_line`, `silhouette_line`, `module_line`, `tool_line`, `banner_line`.
- Components: `StatusStripRoot`, `StatusStripText`, `ItemStripText`, `AmmoStripText`, `ReticleStripText`, `MissionStripText`, `ObjectiveStripText`, `EnemyStripText`, `BreachStripText`, `LastEventStripText`, `StanceStripText`, `SilhouetteStripText`, `ModuleStripText`, `ToolStripText`, `CaptionStripText`, `CaptionStripRoot`, `BannerStripRoot`, `BannerStripText`.

## Common Pitfalls
- Localization plan is OPEN — every label uses English-only literals (STATUS, ITEM, HP, READY, MODS, BODY, TOOL, MISSION, OBJECTIVE, ENEMY, BREACH, EVENT, STANCE, EJECT NOW, ARMOR CRACKED, AMMO OUT, RELOAD, etc.). Flag any string-source code path that bakes English-only strings; route through a future `cf-localization` boundary instead of hardcoding text. Tier-A localization closes at BP12.
- The cf-app bridge is the only writer of `HudState` AND `HudSettings`. The HUD reads via `Res<HudState>` / `Res<HudSettings>` and never mutates engine state.
- `apply_ui_scale_from_settings` clamps scale to `[0.5, 4.0]` to prevent wedge layouts. Bevy's `UiScale` is global; it reflows every Val::Px line in the workspace, including any future `cf-tools-editor` UI. Val::Percent layouts stay unaffected.
- Banner severity is text + icon (color-independent) — high-contrast palette intentionally drops the per-severity background color and falls back to solid black so the severity word + icon glyph carries the signal alone.
- `update_caption_strip` toggles `Display::None` on the strip root rather than removing the entities — this avoids re-spawn churn when captions toggle on/off mid-run.
- M5+ extensions: `body_silhouette.placeholder` flips to `false` when M5 lands real per-zone wound model; `module_strip.placeholder` flips when M5 lands real chassis modules. cf-ui formatter functions tolerate either state without re-tests.

## Source Trail
- spec/prototype-roadmap §M1 — Actor Controller And Sim Core (M1-004).
- spec/prototype-roadmap §M4 — HUD And Comic-Noir UI (M4A subsection).
- spec/native-implementation-backlog §M4 (M4-001..004).
- spec/ux-wireframes-slice-a.
- spec/accessibility-comfort-slice-a.
- spec/animation-system §Core Actor Presentation Rule.
- DR-003 (silhouette + advanced HUD opt-in lean — closed at M4A).
- DR-009 (command UX — closes at M4B).
- DR-012 (accessibility floor — CLOSED at M4A).
- DR-019 (visual direction — closes at M4B).
- docs/implementation-log/2026-05-06-m1-actor-controller.md.
- docs/implementation-log/2026-05-07-m1-5-micro-breach-fun-slice.md.
- docs/implementation-log/2026-05-09-m4a-readability-acc-a-floor.md.
