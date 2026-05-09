---
type: spec
status: prototype-reqs
ready_when: "The buy/loadout UI, actor slot card, squad summary, workbench diagnostics, AI debug labels, replay labels, bot trust panel, and overlap review table consume generated role-card and AI summary artifacts and pass LOAD-R-01..LOAD-R-13."
feeds:
  - DR-003
  - DR-004
  - DR-006
  - DR-008
  - DR-009
---

<- [[spec/index|spec section]] · [[spec/equipment-loadout|equipment/loadout]] · [[spec/equipment-loadout-workbench-slice-a|loadout workbench Slice A]] · [[spec/ux-wireframes-slice-a|UX wireframes Slice A]] · [[spec/package-builder-workbench-slice-a|package-builder Slice A]] · [[spec/ai-trust-harness-slice-a|AI harness Slice A]] · [[references/equipment-ai-behavior-contract|AI behavior contract]] · [[references/equipment-ai-summary-seed-slice-a|AI summary seed]] · [[references/equipment-source-anchored-device-snapshots|source snapshots]] · [[references/equipment-comparable-design-patterns|comparable patterns]] · [[references/equipment-role-cards-slice-a|generated role cards]] · [[references/equipment-role-card-renderer-view-slice-a|renderer view]] · [[references/equipment-consumer-traceability-slice-a|generated trace report]] · [[references/equipment-overlap-audit-slice-a|overlap audit]] · [[references/equipment-overlap-resolution-worksheet-slice-a|overlap worksheet]] · [[references/equipment-provenance-workbench-view|provenance view]] · [[references/equipment-overlay-merged-preview|merged preview]] · [[references/equipment-loadout-fixtures-slice-a|LOAD-A fixtures]] · [[references/equipment-package-diagnostics-slice-a|package diagnostics]] · [[references/sources|sources]]

# Equipment Role-Card Renderer Slice A

> [!summary] Purpose
> Turn the generated LOAD-009/LOAD-010 equipment artifacts into concrete UI, workbench, AI-debug, replay, and balance-review requirements. This page is the missing layer between "we have role-card data" and "players, bots, modders, designers, and test agents can use it."

> [!important] Product stance
> The role-card renderer is not a documentation widget. It is a product surface. If two items feel identical, if a bot will misuse a tool, if a package author forgot AI metadata, or if a loadout cannot breach the mission material, that must be visible before the player pays, deploys, or blames the AI.

## Slice A Question

Can the game render every important equipment item as a readable, system-backed role card, hide internal/payload items from the player catalog while keeping them visible to tools, and turn overlap groups into actual design decisions instead of silent catalog bloat?

## Inputs

| Input | Path | Required Use |
|---|---|---|
| Role cards JSON | `cortext_command_vault/references/equipment-role-cards.slice-a.json` | Canonical Slice A item-card data for renderer tests. Contains 106 cards. |
| Role cards note | [[references/equipment-role-cards-slice-a]] | Human-readable summary, count tables, sample cards, and generation caveats. |
| Renderer view JSON | `cortext_command_vault/references/equipment-role-card-renderer-view.slice-a.json` | Canonical generated LOAD-R fixture: 63 player-catalog rows, 106 workbench rows, 5 detail drawers, 10 overlap rows, and 9 fixture summaries. |
| Renderer view note | [[references/equipment-role-card-renderer-view-slice-a]] | Human-readable catalog slice, special detail drawers, overlap renderer rows, fixture summaries, and LOAD-R coverage map. |
| Overlap audit JSON | `cortext_command_vault/references/equipment-overlap-audit.slice-a.json` | Canonical Slice A overlap groups. Contains 10 groups and 42 player-catalog items under role-signature pressure. |
| Overlap audit note | [[references/equipment-overlap-audit-slice-a]] | Human-readable high/medium risk groups and required differentiators. |
| Overlap worksheet | [[references/equipment-overlap-resolution-worksheet-slice-a]] | Candidate role splits, skin/legacy decisions, mission fixture needs, and consumer implications for each overlap group. |
| Merged overlay | [[references/equipment-overlay-merged-preview]] | Patch-applied item records that feed role cards, fixtures, and package diagnostics. |
| Provenance workbench view | [[references/equipment-provenance-workbench-view]] | Fixture-level source/provenance/warning rows used by detail drawers and workbench panels. |
| LOAD-A fixtures | [[references/equipment-loadout-fixtures-slice-a]] | First catalog/cart/actor-column fixtures for UI and AI tests. |
| AI scenario seeds | [[references/equipment-ai-scenarios-slice-a]] | Bot behavior tests that must consume the same role and warning vocabulary. |
| AI behavior contract | [[references/equipment-ai-behavior-contract]] | Canonical item-choice/refusal reason labels and event names for bot honesty rows. |
| AI summary seed | [[references/equipment-ai-summary-seed-slice-a]] | Generated bot-facing row contract for claim state, blackboard keys, required reason labels, event families, source confidence, and first fix actions. |
| Source-anchored snapshots | [[references/equipment-source-anchored-device-snapshots]] | Concrete field-value examples for source tabs and source-backed role explanations in detail drawers. |
| Comparable design patterns | [[references/equipment-comparable-design-patterns]] | OpenSoldat-style handling/feel fields, OpenRA projectile/effect separation, and authoring/runtime boundaries for LOAD-COMP rows. |
| Package diagnostics | [[references/equipment-package-diagnostics-slice-a]] | Expected package-builder warnings and bot-assignment gates. |
| Field map | [[references/equipment-cccp-field-map]] | Field ownership map for deciding which value belongs to item definition, loadout template, runtime state, replay, backend, or package diagnostics. |
| Interactive loadout workbench spec | [[spec/equipment-loadout-workbench-slice-a]] | Concrete UI prototype target for catalog rows, actor columns, detail drawers, diagnostics, overlap compare, bot trust, and export preview. |

## Required Surfaces

| Surface | Primary User | Must Show | Must Not Show |
|---|---|---|---|
| Buy/loadout catalog row | Player | Display name, role icon, package, cost, mass, range, terrain/support tag, bot competence, warning count, overlap badge. | Internal payloads, turret components, non-catalog implementation pieces. |
| Item detail drawer | Player and designer | Short role text, best/bad use, handling commitment, terrain consequence, bot policy, risk labels, provenance badges, source package, related overlap group. | Raw JSON dumps as the only explanation. |
| Actor slot card | Player and AI tester | Slot fit, actor role fit, one/two-hand constraints, mass burden, bot competence, missing role warning, delivery risk contribution. | Order-sensitive hidden cargo assignment. |
| Squad capability summary | Player | Breach, dig, fill, heal, scout, anti-craft, suppression, mobility, delivery burden, bot-safe count. | A simple total DPS/cost score that hides role gaps. |
| Workbench diagnostic panel | Creator | Warning code, severity, first fix action, source path, provenance, package-mode verdict, impacted consumers. | Blocking private/dev experimentation unless package mode requires it. |
| AI debug/replay label | Tester and future player recap | Item id, primary verb, selected/refused reason, danger radius, target class, role signature, causality event ids. | "AI failed" without a reason label. |
| Balance overlap table | Designer | Role signature, items, risk, differentiator spans, recommended action, decision status. | Duplicate weapons presented as meaningful variety without evidence. |

## Field Contract By Consumer

| Consumer | Required Fields From Role Card | Renderer Obligation |
|---|---|---|
| Player catalog | `display_name`, `catalog_visibility`, `archetype`, `role_tags`, `primary_verb`, `range_band`, `terrain_consequence`, `ui_contract`, `balance_inputs`, `bot_competence`, `risk_profile`. | Render a dense row and detail drawer that explain why the item changes play. |
| AI runtime and harness | `bot_competence`, `bot_policy`, `target_classes`, `range_band`, `material_fit`, `terrain_consequence`, `risk_profile`, `next_tests`. | Convert item selection and refusal into stable reason labels. |
| Workbench/package builder | `modding_workbench`, `source_path`, `package_id`, `catalog_visibility`, `risk_profile`, `next_tests`. | Show warnings, first fix actions, source positions, and package-mode implications. |
| Balancing/design | `role_signature`, `balance_inputs`, `handling_commitment`, `primary_verb`, `terrain_consequence`, overlap group membership. | Make overlap visible and require a differentiator, catalog, skin, or legacy decision. |
| Replay/backend | `item_id`, `package_id`, `replay_contract`, `role_signature`, `source_kind`, `catalog_visibility`. | Emit stable item/role labels in replay events, support reports, and package compatibility summaries. |

## Renderer Layout Skeleton

The same data should scale from a row to a drawer. Slice A can be plain HTML/React/native UI; the contract matters more than final art.

```text
+----------------------------------------------------------------------------+
| [Role Icon] Compact Assault Rifle                  Coalition.rte  Catalog   |
| Primary firearm | Assault | medium range | mostly actor damage              |
| Cost 45 oz   Mass 9 kg   Bot: Risky   AI: seeded, needs harness             |
|                                                                            |
| Best at: controlled mid-range infantry fights                               |
| Bad at: hard material breach, close rescue, bot-safe explosive work          |
| Handling: light aimed standoff, quick reload compared with heavier rifles    |
| Terrain: no reliable carve/fill behavior                                     |
| AI policy: bot use gated until explicit target/range/friendly-fire fields    |
| Provenance: generated + manual overlay available                             |
| Overlap: HIGH, medium assault group, needs visible differentiator             |
| [Compare] [Assign To Actor] [Open Workbench Source] [Add To Test Fixture]    |
+----------------------------------------------------------------------------+
```

### Visual Rules

| Rule | Why |
|---|---|
| Catalog rows are dense but not cryptic. | This is a repeated planning surface, not a marketing card. |
| Icons and short labels both exist. | No critical role, warning, or bot state can be color-only. |
| Detail drawers are available for every catalog item. | The player can learn why an item matters without leaving the buy flow. |
| Provenance badges are compact. | Source trust matters, but the buy screen should not become a legal lecture. |
| High-risk overlap badge is visible in designer/workbench mode. | Duplicate catalog roles must be resolved deliberately. |
| Hidden/internal cards are not catalog rows. | Internal payloads still matter to package/replay diagnostics, but not as player purchase options. |

## Visibility Policy

| `catalog_visibility` | Player Catalog | Workbench | Replay / Debug | Notes |
|---|---|---|---|---|
| `player_catalog` | Show by default. | Show. | Show. | Normal buy/loadout items. |
| `replacement_or_legacy_catalog` | Hide or group under legacy/replaced unless explicitly enabled. | Show with replacement policy. | Show. | Preserves history without cluttering the default catalog. |
| `internal_component` | Hide. | Show under owning item/effect graph. | Show as child event when needed. | Turret guns, embedded devices, subcomponents. |
| `internal_payload` | Hide. | Show under parent projectile/device. | Show as child event when needed. | Spawned ordnance and payloads. |
| `hidden_or_internal` | Hide. | Show in diagnostics and source browser. | Show only if it appears in event data. | Non-catalog examples like Concrete Sprayer still inform schema and tooling. |

## Role And Warning Vocabulary

| Vocabulary | Current Values / Examples | Renderer Rule |
|---|---|---|
| Primary verbs | `engage_actor`, `suppress_or_break`, `destroy_area_or_hard_target`, `excavate_or_breach`, `fill_or_build`, `heal_or_rescue`, `traverse_or_reposition`, `backup_or_finish`, `long_range_pick`. | Use as the main action label. It should be readable as "what this item does." |
| Terrain consequence | `mostly_actor_damage`, `removes_or_opens_material`, `adds_or_repairs_material`, `area_hazard_or_blast`, `mobility_state_change`, `hidden_internal_or_payload`. | Pair with material-fit chips and overlay previews. |
| Bot competence | `Good`, `Risky`, `Manual Recommended`, `No AI Support Yet`, `not_for_default_bot_loadout`; generated `claim_state` values from [[references/equipment-ai-summary-seed-slice-a]]. | Player-facing rows show `Good/Risky/Manual/No AI`; hidden cards use workbench language; bot trust panels show the generated claim state and harness promotion status. |
| Warning badges | `ai_summary_seed_needs_harness_or_manual_review`, `unclear_role_tags`, `package_builder_visibility`, `bot_use_needs_gate`. | Badge text must link to first fix action in workbench mode. |
| Provenance | direct, inherited, inferred, missing, manual. | Detail drawers can show inferred/manual; AI and balance tests need source-backed promotion before settled claims. |

## Special-Case Rendering Requirements

| Item | Why It Is A Test Case | Required Rendering |
|---|---|---|
| `Coalition.rte/Assault Rifle` | High-risk medium assault overlap, player catalog, bot `Risky`, missing AI summary. | Catalog row visible; high-overlap badge in designer mode; bot gate visible; compare against Compact Assault Rifle, AK-47, M16A2, Old Stock Battle Rifle. |
| `Base.rte/Medikit` | Scripted support tool with heal/rescue behavior. | Render as `heal_or_rescue`; show target rules, support context, and bot-risk status; include AI-H-LOAD link. |
| `Base.rte/Grapple Gun` | Mobility/scripted/manual item. | Render as mobility with anchor/failure-state placeholders; label `Manual Recommended` until bot navigation tests exist. |
| `Base.rte/Concrete Sprayer` | Build/fill support tool, important but hidden/non-catalog in current corpus slice. | Hide from default catalog; show in workbench/source/replay; mark `fill_or_build`, `adds_or_repairs_material`, and `not_for_default_bot_loadout`. |
| `Base.rte/Rocket Launcher` | Heavy explosive/breach/craft-threat item with bot `Good` only if safety is proven. | Render danger radius, target classes, long reload/heavy handling, terrain blast risk, friendly-fire policy, and LOAD-010 overlap membership if applicable. |

## Overlap Resolution Rules

Overlap is a design prompt, not a ban. Personal/private prototypes can keep every item. The product-quality rule is that every retained default-catalog item needs a visible reason to exist.

| Risk | Required Action Before Default Catalog | Acceptable Outcomes |
|---|---|---|
| High | Add a visible gameplay differentiator or decide catalog/skin/legacy status. | New handling identity, AI policy, economy role, mission-role hook, faction identity with clear text, skin grouping, or hidden legacy row. |
| Medium | Add at least one material, fuse, ammo, handling, bot policy, economy, or mission differentiator before bot-safe/default claims. | Keep for prototype, mark manual/risky, add fixture, or merge later. |
| Low | Track as watch item. | Keep unless playtests show confusion. |

### Current High-Risk Groups

| Group | Items | Why Risky | Suggested Differentiators |
|---|---|---|---|
| Medium assault rifles | Old Stock Battle Rifle, Assault Rifle, Compact Assault Rifle, AK-47, M16A2. | Same player-catalog primary-firearm assault signature, medium range, mostly actor damage, bot `Risky`. | Distinguish recoil recovery, magazine economy, burst/full-auto identity, penetration, weight, one-arm fallback, faction ammo availability, price/mass, AI preferred range, and mission reward role. |
| Medium sidearms / compact weapons | .357 Magnum, Desert Eagle, MP5K, UZI. | All read as backup/finish or compact combat items without enough proven role separation. | Make revolver armor punch, Desert Eagle high-risk burst, MP5K stable bot-safe close suppressor, UZI panic spray or dual-wield identity. |
| Short sidearms | Old Stock Pistol, Auto Pistol, Beretta 93R, Luger P08. | Same short backup role pressure. | Separate draw speed, one-arm reliability, stealth/noise, ammo cost, burst mode, veteran sidearm perks, and salvage availability. |

### Medium-Risk Group Handling

| Group Type | Required Follow-Up |
|---|---|
| Explosive bandoliers and grenades | Show fuse/trigger/throw arc/terrain channel/friendly-fire policy; add bot refusal tests. |
| Digger vs shovel | Compare tunnel profile, dig rate, material fit, melee utility, bot stance, and delivery burden in the terrain sandbox. |
| Heavy automatic weapons | Compare suppression cone, setup time, recoil, ammo logistics, actor stability, and AI target class. |
| Sniper rifles | Compare aim time, sharp length, penetration, reload, visibility, and mission overwatch role. |
| Shields | Compare coverage arc, mass, one-hand compatibility, durability, AI stance, and actor-slot tradeoff. |

## Data Model Projection

The renderer can use a normalized view assembled from the generated artifacts.

```json
{
  "item_id": "Coalition.rte/Assault Rifle",
  "display_name": "Assault Rifle",
  "catalog_visibility": "player_catalog",
  "role": {
    "archetype": "Primary firearm",
    "tags": ["Assault"],
    "primary_verb": "engage_actor",
    "range_band": "medium",
    "terrain_consequence": "mostly_actor_damage"
  },
  "stats": {
    "gold_cost": 50,
    "mass": 11,
    "rate_of_fire": 800,
    "reload_time_ms": 1800,
    "round_count": 35,
    "projectile_velocity": 90
  },
  "bot": {
    "competence": "Risky",
    "reason_labels_required": ["bot_use_needs_gate"]
  },
  "warnings": ["missing_explicit_ai_item_fields"],
  "overlap": {
    "risk": "high",
    "group_id": "player_catalog | Primary firearm | Assault | engage_actor | medium | actor | - | mostly_actor_damage"
  }
}
```

## Acceptance Tests

| ID | Test | Pass Condition |
|---|---|---|
| LOAD-R-01 | Parse role cards. | Renderer loader parses `equipment-role-cards.slice-a.json`, counts 106 cards, and rejects missing required fields with source path. |
| LOAD-R-02 | Parse overlap audit. | Loader parses 10 overlap groups and links each player-catalog overlap item to its group; first generated proof is [[references/equipment-role-card-renderer-view-slice-a]]. |
| LOAD-R-03 | Catalog visibility. | Player catalog shows 63 unique `player_catalog` rows, hides internal/payload/hidden cards by default, and collapses duplicate catalog ids into workbench-visible diagnostics. |
| LOAD-R-04 | Detail drawer completeness. | A selected player-catalog item shows role, handling, terrain, bot, warning, provenance, package, replay fields, and a source-snapshot tab when a reviewed snapshot exists; first generated proof has 5 detail drawer examples. |
| LOAD-R-05 | Bot honesty. | Risky/manual/no-AI items show reason labels before assignment to bot-controlled slots. |
| LOAD-R-06 | Special items. | Medikit, Grapple Gun, Concrete Sprayer, Rocket Launcher, and Assault Rifle render with the special-case expectations above. |
| LOAD-R-07 | Overlap warning. | High-risk overlap groups are visible in designer/workbench mode and offer compare/resolve actions. |
| LOAD-R-08 | Squad summary. | A fixture loadout summarizes breach, dig, fill, heal, scout, anti-craft, mobility, bot-safe count, and missing capabilities. |
| LOAD-R-09 | Workbench drill-down. | A warning badge opens a diagnostic row with source path, provenance, severity, first fix action, and package-mode verdict. |
| LOAD-R-10 | Replay/debug labels. | Item events include stable item id, role signature, primary verb, package id, and selected/refused reason label. |
| LOAD-R-11 | Accessibility. | Role, warning, overlap, and bot state remain readable at 200% text scale and are not color-only. |
| LOAD-R-12 | Keyboard/controller navigation. | Catalog rows, detail drawer, overlap table, compare actions, and workbench source links are reachable without mouse-only traps. |
| LOAD-R-13 | AI summary seed parity. | Bot honesty rows join to [[references/equipment-ai-summary-seed-slice-a]] and show claim state, required reason labels, required events, source confidence, and first fix actions without contradicting the role card. |

## First Implementation Tickets

| Ticket | Scope | Depends On |
|---|---|---|
| LOAD-009A | Build role-card JSON loader and normalized renderer view model. | `equipment-role-cards.slice-a.json`, [[references/equipment-cccp-field-map]] |
| LOAD-009B | Render buy/loadout catalog rows with visibility policy, icons, warnings, bot state, and overlap badges. | [[spec/ux-wireframes-slice-a]] |
| LOAD-009C | Render item detail drawer with source/provenance/risk/role/debug fields. | [[references/equipment-provenance-workbench-view]] |
| LOAD-009D | Render actor slot cards and squad capability summary from LOAD-A fixtures. | [[references/equipment-loadout-fixtures-slice-a]] |
| LOAD-W | Build the interactive loadout/workbench prototype from [[spec/equipment-loadout-workbench-slice-a]]. | Generated LOAD-R view plus LOAD-A fixtures. |
| LOAD-W-010 | Add consumer traceability from [[references/equipment-consumer-traceability-matrix]], [[references/equipment-consumer-traceability-slice-a]], and [[references/equipment-trace-tab-view-slice-a]] to warning drill-downs, detail drawers, AI/replay labels, fixture tabs, and workbench rows. | Generated LOAD-R view plus traceability matrix/report/view. |
| LOAD-009E | Emit AI debug and replay labels for selected/refused item use. | [[spec/ai-trust-harness-slice-a]], [[spec/replay-recorder-slice-a]] |
| LOAD-009G | Use [[references/equipment-ai-behavior-contract]] labels for all bot honesty rows and selected/refused item events. | AI-EQ reason taxonomy. |
| LOAD-009H | Join [[references/equipment-ai-summary-seed-slice-a]] into bot honesty rows and replay labels. | Generated AI summary seed. |
| LOAD-009F | Add workbench diagnostic drill-down from warning badge to source/provenance/package verdict. | [[spec/package-builder-workbench-slice-a]], [[references/equipment-package-diagnostics-slice-a]] |
| LOAD-010A | Build overlap table with risk, items, differentiator spans, recommended action, and status. | `equipment-overlap-audit.slice-a.json` |
| LOAD-010B | Create high-risk overlap decision worksheet for assault rifles and sidearms. | [[references/equipment-overlap-audit-slice-a]] |
| LOAD-010C | Feed overlap groups into balance tests and catalog/skin/legacy decisions. | [[spec/equipment-loadout]] |

## UI Copy Rules

| Bad Copy | Better Copy | Reason |
|---|---|---|
| `Risky` alone | `Risky: missing AI range/friendly-fire rules` | The player and creator need the reason. |
| `Overlaps` alone | `High overlap: same assault medium-range role as 4 rifles` | Makes duplicate pressure concrete. |
| `Hidden` alone | `Workbench-only: internal payload or non-catalog item` | Avoids making internal content feel broken. |
| `Missing metadata` alone | `Missing AI summary. First fix: add target/range/friendly-fire fields.` | Turns warning into action. |
| `Manual` alone | `Manual recommended until grapple anchor/path tests pass` | Honest without making the item feel bad. |

## Open Questions

| Question | Cheapest Test |
|---|---|
| Should role cards live in buy/loadout only, or also appear in post-mission salvage? | Use same renderer in a salvage mock and compare scan speed. |
| How much overlap information should normal players see? | Hide high-risk labels in normal mode, then test whether comparison text alone prevents confusion. |
| Should bot competence be authored or generated from AI-H results? | Start authored/generated, then replace with harness-derived confidence once tests run. |
| Should hidden/internal cards appear in replay exports by default? | Emit them in debug exports; only surface to players when they explain a visible event. |
| Should default-catalog duplicates be grouped as skins? | Prototype assault rifle grouping and see whether it improves or harms faction flavor. |

## Source Trail

### Local

- `cortext_command_vault/references/equipment-role-cards.slice-a.json`
- `cortext_command_vault/references/equipment-role-card-renderer-view.slice-a.json`
- `cortext_command_vault/references/equipment-overlap-audit.slice-a.json`
- `cortext_command_vault/references/equipment-overlay-merged.preview.json`
- `cortext_command_vault/references/equipment-provenance-workbench-view.slice-a.json`
- `cortext_command_vault/references/equipment-loadout-fixtures.slice-a.json`
- `cortext_command_vault/references/equipment-package-diagnostics.slice-a.json`
- `../research_tools/equipment_role_cards.py`
- `../research_tools/equipment_role_card_renderer_view.py`
- `../research_tools/equipment_provenance_workbench_view.py`
- `../research_tools/equipment_overlay_check.py`
- [[references/equipment-role-cards-slice-a]]
- [[references/equipment-role-card-renderer-view-slice-a]]
- [[references/equipment-ai-behavior-contract]]
- [[references/equipment-ai-summary-seed-slice-a]]
- [[references/equipment-overlap-audit-slice-a]]
- [[references/equipment-overlap-resolution-worksheet-slice-a]]
- [[references/equipment-consumer-traceability-matrix]]
- [[references/equipment-consumer-traceability-slice-a]]
- [[references/equipment-role-design-deep-dive]]
- [[references/equipment-cccp-field-map]]
- [[references/equipment-provenance-workbench-view]]
- [[spec/equipment-loadout]]
- [[spec/equipment-loadout-workbench-slice-a]]
- [[spec/ux-wireframes-slice-a]]
- [[spec/package-builder-workbench-slice-a]]
- [[spec/ai-trust-harness-slice-a]]

## Change Log

- 2026-05-04: Created as the build-facing renderer and overlap-resolution layer for generated equipment role cards and overlap audit artifacts.
- 2026-05-04: Added [[references/equipment-role-card-renderer-view-slice-a]] as the generated LOAD-R fixture and validation target for catalog rows, detail drawers, overlap rows, fixture summaries, and renderer acceptance coverage.
- 2026-05-04: Linked [[spec/equipment-loadout-workbench-slice-a]] as the first interactive prototype target for consuming LOAD-R/LOAD-A data.
- 2026-05-04: Linked [[references/equipment-consumer-traceability-matrix]] so renderer rows can expose downstream AI/UI/package/balance/replay/backend consumer impact.
- 2026-05-04: Linked [[references/equipment-consumer-traceability-slice-a]] as the generated LOAD-011 source for row-level consumer status and trace-tab gaps.
- 2026-05-04: Linked [[references/equipment-ai-behavior-contract]] so bot honesty rows use the same item choice/refusal vocabulary as AI harness, package diagnostics, and replay events.
- 2026-05-04: Linked [[references/equipment-ai-summary-seed-slice-a]] so renderer bot honesty rows show generated claim state, reason labels, events, source confidence, and first fix actions instead of stale "AI summary missing" placeholders.
- 2026-05-04: Linked [[references/equipment-source-anchored-device-snapshots]] so detail drawers can show literal source values beside normalized role-card fields.
