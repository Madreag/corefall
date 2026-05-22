# content/settings/

Player + admin + modder + AI-agent editable game settings.

## Topology

Each `*.json` here overrides one slice of `cf_control::Settings`. Engine loads
hardcoded defaults at boot, then layers JSON overrides on top in order:

1. `defaults.json` — full default snapshot for reference + new-install seed.
2. `graphics.json` — render quality, resolution, vsync, frame cap, comic-style.
3. `audio.json` — master/sfx/music/voice/captions volumes, accessibility audio.
4. `controls.json` — key bindings (per-action keycode list), mouse, gamepad.
5. `gameplay.json` — difficulty, autonomy, slowdown assists, pie-menu speed,
   sharp-aim duration, etc.
6. `accessibility.json` — colorblind palette, ui_scale, focus-trap behavior,
   reduced-motion, larger-font, screen-shake intensity.
7. `debug.json` — overlays, tracing levels, dev cheats (gated by build).
8. `network.json` — server address, port, lobby discovery, anti-cheat opts.

## Editing

Players: edit any field via the in-game options menu. Changes round-trip
through these files so they survive restarts + can be diff'd in git.

Admins running dedicated servers: edit `gameplay.json` + `network.json`
directly; restart server to apply.

Modders: ship a `<modname>/` subfolder with overrides; loader merges last.

AI coding agents: read JSON to inspect runtime config; write JSON to apply
balance changes without recompiling.

## Schema versioning

Every settings file MUST carry `"schema_version": N` at the top. Loaders
fail-closed + `tracing::warn!` on version mismatch; the user keeps the old
values.

## Validation

`cf-mod validate settings <path>` checks all `content/settings/*.json` for:

- Schema version ≥ current loader minimum.
- Key bindings reference known action ids.
- Numeric fields within accepted ranges (`master_volume ∈ [0.0, 1.0]`,
  `ui_scale ∈ [0.5, 4.0]`, etc.).
- No unknown top-level keys (typo catch).

Loaders MUST `tracing::warn!` on parse failure and fall back to hardcoded
defaults — never silent failure.
