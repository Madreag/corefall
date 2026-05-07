---
type: spec
status: closed-direction
authority: "Localization: 11 Tier-A fully-localized languages + 8 Tier-B UI-only + community-localizable mod layer. AI translation + community review. Project Fluent. Multi-script font support. RTL audit."
ready_when: "All player-visible strings keyed; Tier-A languages tested in playtest; mod-localization layer accepts community packs; CI gate verifies zero hardcoded English."
feeds:
  - DR-012
  - DR-014
  - DR-019
  - DR-024
  - DR-046
  - DR-047
---

← [[spec/index|spec section]] · [[decisions/dr-046-player-facing-surfaces-direction|DR-046]] · [[spec/narrative-bible|narrative bible]] · [[spec/shell-ui-architecture|shell UI]]

# Localization Plan

## Strategy

**AI translation + community review + first-class moddable layer + multi-script font support.**

## Language Tiers

### Tier-A (UI fully localized + narrative copy translated)

| Language | Locale code | Notes |
|---|---|---|
| English | en | Source. |
| Spanish (LATAM) | es-419 | Large Steam audience. |
| Portuguese (Brazil) | pt-BR | Large Steam audience. |
| German | de | Major EU. |
| French | fr | Major EU. |
| Italian | it | EU. |
| Russian | ru | Large Steam audience. |
| Polish | pl | Strong Steam community. |
| Simplified Chinese | zh-Hans | Large global audience. |
| Japanese | ja | Strong indie + JRPG audience. |
| Korean | ko | Strong Steam audience. |

### Tier-B (UI strings localized; narrative remains English)

Turkish (tr), Czech (cs), Dutch (nl), Ukrainian (uk), Arabic (ar — RTL), Vietnamese (vi), Thai (th), Indonesian (id).

### Mod-localization

First-class moddable layer. Modders submit `.ftl` packs via Steam Workshop or community mirror.

## Technical

| Component | Detail |
|---|---|
| **Format** | Project Fluent (.ftl) — https://projectfluent.org/. Supports plurals, gendered, nested, message references. |
| **Rust crate** | `fluent-rs` + `fluent-bundle`. |
| **Macro** | `t!("key.id")` and `t_args!("key.id", arg=value)` convenience macros. |
| **Hot-reload** | Locale switcher in settings; live-reload without restart. |
| **Fallback** | en for missing keys. CI gate logs missing keys. |
| **Font selection** | Noto Sans + Noto Sans CJK + Noto Naskh Arabic (multi-script coverage, OFL license). |
| **RTL support** | Arabic; Right-to-Left text direction; UI mirroring for menus where appropriate. |
| **Multi-script verification** | CJK rendering tested; Cyrillic; Arabic shaping. |

## AI Translation Pipeline

1. Source string written in `content/i18n/en/<file>.ftl`.
2. AI agent (GPT-4o / Claude Sonnet 3.7) translates per Tier-A language.
3. AI agent reviews translation for context consistency + tactical-pulp tone.
4. Project-owner approves OR community reviews via per-language Discord channel.
5. Commit to `content/i18n/<lang>/<file>.ftl`.

## File Structure

```
content/i18n/
├── en/                          # source
│   ├── ui.ftl                   # menu/HUD/settings strings
│   ├── narrative_factions.ftl   # faction lore
│   ├── narrative_npcs.ftl       # NPC bios + dialogue
│   ├── narrative_missions.ftl   # mission briefings/debriefs
│   ├── codex.ftl                # codex entries
│   ├── achievements.ftl         # achievement names + flavor
│   ├── tutorial.ftl             # tutorial copy
│   ├── tooltips.ftl             # contextual tooltips
│   ├── captions.ftl             # SFX/audio captions per DR-020
│   └── errors.ftl               # error messages
├── es-419/
│   └── ... (mirrored)
├── ja/
│   └── ...
└── ar/
    └── ...
```

## Mod-Localization

Mod authors include `mods/<mod-id>/i18n/<lang>/<file>.ftl` files. Loaded after first-party strings; can override OR extend.

## CI Gates

- `cf-i18n-check` — verifies zero hardcoded English in UI/HUD/captions/error messages source.
- `cf-i18n-coverage` — verifies all keys present in each Tier-A language.
- `cf-i18n-rtl-test` — verifies Arabic UI mirrors correctly.
- `cf-i18n-script-test` — verifies CJK + Cyrillic + Arabic + Latin all render correctly.

## Community Review Program

- Per-language Discord channel (post-launch Discord).
- Volunteer reviewers credited in-game.
- Per-language coordinator role (community-elected).

## Done-Criteria

- [ ] All player-visible strings keyed via Fluent.
- [ ] All Tier-A languages translated + community-reviewed.
- [ ] All Tier-B languages have UI translation.
- [ ] CI gates passing.
- [ ] RTL + CJK + Cyrillic rendering verified.
- [ ] Mod-localization layer accepts packs.
- [ ] Locale switcher in settings live-reloads.

## Source Trail

- [[decisions/dr-046-player-facing-surfaces-direction]]
- Project Fluent: https://projectfluent.org/
- fluent-rs: https://crates.io/crates/fluent
- Noto Sans: https://fonts.google.com/noto
