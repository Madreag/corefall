# M9B — Trench Networks + Defensive Position Authored Content

## Status

`active`

## Intent

**M9B is the authored-trench-network milestone** — promotes "dig a trench around the reactor" from M9's free-form per-pixel carve into a first-class authored content layer: 6 trench cross-section variants, a zigzag-pattern procgen generator, per-segment cover state, fire-step / breastwork / duckboard / drainage modules, per-zone trench templates (CC parity), and 8 launch scenarios that prove the gameplay loop. After M9B, a player digging a trench gets meaningful tactical state ("standing here gives me partial cover; crouch for full cover; the fire-step lets me shoot over the parapet"), procgen levels ship with WWI-style zigzag trench lines, and mappers can author trench templates declaratively instead of hand-painting pixels.

**Why this milestone exists:** M9 proved the trench-as-cover gameplay loop with per-pixel carve. But "any hole the player digs" is not the Cortex-Command-style authored defensive content players expect. CC ships explicit `Trench` / `Bunker` / `MGNest` static actors with deterministic cross-section, declared cover states, and per-zone templates. M9B fills that gap on the trench side; M9C fills it on the static-fortification side.

M9B promise: **"trenches feel like authored WWI defenses (zigzag pattern, fire-steps, breastworks, duckboards over mud) — not just holes — with per-segment cover state and 6 declared cross-section variants; procgen ships zigzag trench lines on PvP + Reactor Defense maps; mappers can drop a trench template anywhere on the map."**

## Player-facing behavior

### Trench cross-section variants (6 authored)

Each variant is a `TrenchSegment` asset with declared depth, width, slope, embedded modules, and per-stance cover state.

| Variant | Depth | Width | Embedded modules | Cover (Standing) | Cover (Crouched) | Cover (Prone) | Use case |
|---|---|---|---|---|---|---|---|
| `shallow_scrape` | 6 px | 12 px | — | None | Partial | Full | Emergency cover; dug in 5s with hands |
| `standard` | 16 px | 16 px | duckboard | Partial | Full | Full | Default infantry trench |
| `deep` | 24 px | 16 px | duckboard + drainage | Full | Full | Full (head below grade) | Sustained defense; safe from MG |
| `communication` | 16 px | 8 px | duckboard | Partial | Full | Full | Connects firing trenches; narrow → no fighting |
| `fire_step` | 16 px (with 8 px raised step) | 20 px | duckboard + step | Standing on step = exposed (firing posture) | Crouched off-step = full | Full | Letting the defender shoot over the parapet |
| `parapet_raised` | 16 px (with 8 px sandbag breastwork above grade) | 24 px | duckboard + breastwork (sandbag wall, M9C) | Full (breastwork covers head) | Full | Full | Pre-built / hardened forward position |

- Player sees a **cover indicator** on their HUD when standing inside a trench segment: 3-state chevron icon (Standing-Exposed / Partial / Full) tied to the segment's variant + the player's current stance.
- Cover state drives **incoming-fire damage routing** (M14): a `Full`-cover hit checks the parapet material first; only over-penetrating rounds reach the actor. `Partial` cover splits the body graph — head/shoulders exposed, torso/legs protected.

### Zigzag pattern procgen generator

WWI-style zigzag: a continuous trench line never runs straight for more than 12 tiles before kinking 45°, preventing **enfilade** (a single shooter sweeping the whole line).

- **Generator input**: a polyline path (start → end), desired segment density, branch points for communication trenches.
- **Per-kink offset**: ±45° every 8-12 tiles (deterministic; seeded off `world_seed`).
- **Branching rule**: every N segments, spawn a perpendicular communication trench connecting to the rear line (depth 16 px).
- **Endpoint capping**: every trench line ends in either (a) a dead-end with a fire_step facing outward, or (b) a connection to a static fortification (MG nest / bunker entry — owned by M9C).
- **Procgen consumers**: M2 PvP arenas (M9B-default trench presets per arena type), M9 Reactor Defense (player can request "place trench preset" instead of free-form carving), M43 PvE survival (procgen ruins ship with rotted/partial trench lines).

### Per-segment cover state field

Every `TrenchSegment` instance exposes a `CoverState` field readable by:
- **AI doctrine** (M22 + M44D tank-crew) — AI in trench evaluates "I have Full cover; safe to reload" vs "I am on fire_step; expose to fire 1 burst then duck."
- **HUD readability** — chevron icon on player HUD; per-segment chevron icon when material overlay `tactical` mode is on (M9B adds this 6th overlay mode).
- **Replay** — `trench.cover_state_changed` event when player crosses segment boundary or changes stance.

### Trench modules (embedded sub-content)

| Module | Function | Material cost | Build time |
|---|---|---|---|
| `duckboard` | Floor planks over mud; converts wet-mud slipping (M3) to firm footing; drains water | 2 wood | 4s |
| `fire_step` | Raised 8-px platform in trench wall; player stands on step → exposed shoot posture | 4 dirt + 1 wood | 8s |
| `breastwork` | Sandbag wall above grade (M9C parts inventory); HP 400 vs small-arms | 6 sandbags | 12s (or pre-built via parapet_raised variant) |
| `drainage_sump` | Gravity-fed drain at trench low point; flushes water tiles (M19) accumulating in trench | 2 dirt + 1 pipe | 6s |
| `revetment` | Wood/iron-mesh side reinforcement; M14E integrity field 600 prevents wall slough | 4 wood + 2 iron | 10s |
| `corner_traverse` | Reinforced corner at zigzag kinks; M14 prevents grenade-fragmentation funneling | 2 dirt + 4 sandbags | 6s |

### Per-zone trench templates (CC parity)

Templates are declarative — `content/trench_templates/<id>.trench.ron` — describing a complete defensive layout that a mapper or M28F blueprint can drop wholesale. Cortex Command's `Scene` asset embeds similar `TrenchPreset` data; M9B mirrors that grammar.

Template fields: footprint, path polyline, per-segment variant overrides, embedded fortifications (MG nests + watchtowers from M9C placed at template-relative offsets), per-zone metadata (faction, doctrine hint, recommended garrison size).

### 8 launch trench scenarios

| Scenario | Layout | Players | Goal |
|---|---|---|---|
| `m9b_zigzag_baseline` | Single zigzag line, 60 tiles long, fire-steps every 12 tiles | 1v1 | Prove zigzag prevents single-shooter enfilade |
| `m9b_two_line_defense` | Forward firing trench + 16-tile communication trench + rear reserve line | 2v2 | Prove communication trenches enable reinforcement under fire |
| `m9b_fire_step_duel` | Two parallel trench lines 40 tiles apart, fire-step facing each other | 2v2 | Prove fire-step exposure trade-off matters |
| `m9b_drainage_flood` | Trench line during rainfall (M31); requires drainage_sump or floods | 1-2 | Prove drainage_sump gameplay |
| `m9b_reactor_defense_zigzag` | M9 reactor + pre-placed zigzag trench around it (template-based) | Solo | M9-grade scenario with M9B authored trench |
| `m9b_template_drop_test` | Mapper drops `wwi_frontline_a.trench.ron` template via cfctl | Sandbox | Validate template authoring + import pipeline |
| `m9b_ai_in_trench_doctrine` | AI defenders garrison a trench line; player attacks | 1v3 | Prove AI uses cover state correctly |
| `m9b_breastwork_breach` | Parapet-raised trench under sustained MG fire; breastwork degrades + breaches | 2v2 | Prove breastwork is a real M14 surface (HP, breach, repair) |

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-trench` | NEW | Trench segment kernel: cross-section variants, cover state, embedded modules, segment-boundary detection. |
| `cf-procgen::trench_generator` | NEW | Zigzag pattern generator: polyline → seeded kink sequence → segment placement → branching. |
| `cf-content::trench_templates` | NEW | `.trench.ron` template loader + validator + paste-into-world dispatcher. |
| `cf-actor::stance` | MODIFY | New stance-segment interaction: cover state derived from (stance × current trench segment variant). |
| `cf-equipment::tools` | MODIFY | Add `entrenching_tool` (digs `shallow_scrape` in 5s; pre-existing pickaxes dig deeper variants slower). |
| `cf-control` | MODIFY | New cfctl: `act.player.dig_trench_segment { variant }`, `act.player.place_trench_module`, `act.player.drop_trench_template`, `act.player.repair_trench_module`. |
| `cf-control::observe` | MODIFY | Add `observe.actor.cover_state` (3-state) + `observe.trench_segment_at_pos`. |
| `cf-ai` | MODIFY | New doctrine AI-TRENCH-A-01: garrison + cover-state-aware fire decisions. |
| `cf-ui::cover_indicator` | NEW | Player HUD chevron + material-overlay tactical mode (6th overlay). |
| `cf-replay` | MODIFY | 8 new event schemas. |
| `cf-mission` | MODIFY | 8 new scenarios registered. |
| `cf-mod` | MODIFY | Validate `content/trench_templates/*.trench.ron` + `content/trench_segments/*.ron`. |
| `cf-render-2d` | MODIFY | Per-variant trench sprite layers (duckboard, fire-step, drainage, revetment) + cover-state debug overlay. |
| `cf-audio` | MODIFY | Trench cues: `duckboard_step`, `mud_squelch`, `entrenching_dig`, `drainage_drip`. |

## Files

- `game/crates/cf-trench/Cargo.toml` (NEW)
- `game/crates/cf-trench/src/lib.rs` (NEW)
- `game/crates/cf-trench/src/segment.rs` (NEW)
- `game/crates/cf-trench/src/cover_state.rs` (NEW)
- `game/crates/cf-trench/src/modules.rs` (NEW)
- `game/crates/cf-procgen/src/trench_generator.rs` (NEW)
- `game/crates/cf-content/src/trench_templates.rs` (NEW)
- `game/crates/cf-actor/src/stance.rs` (MODIFY: cover-state derivation)
- `game/crates/cf-equipment/src/tools.rs` (MODIFY: entrenching_tool)
- `game/crates/cf-control/src/server.rs` (MODIFY: 4 new cfctl methods)
- `game/crates/cf-control/src/schemas.rs` (MODIFY: param + observe structs)
- `game/crates/cf-control/schemas/v1/actor_view.schema.json` (MODIFY: cover_state)
- `game/crates/cf-control/schemas/v1/observe_frame.schema.json` (MODIFY: trench_segment_at_pos)
- `game/crates/cf-ai/src/trench_doctrine.rs` (NEW)
- `game/crates/cf-ui/src/cover_indicator.rs` (NEW)
- `game/crates/cf-render-2d/src/trench_layers.rs` (NEW)
- `game/crates/cf-audio/src/registry.rs` (MODIFY: trench audio family)
- `game/crates/cf-mission/src/m9b_scenarios.rs` (NEW)
- `game/crates/cf-replay/schemas/event/trench_segment_dug.json` (NEW)
- `game/crates/cf-replay/schemas/event/trench_module_placed.json` (NEW)
- `game/crates/cf-replay/schemas/event/trench_module_repaired.json` (NEW)
- `game/crates/cf-replay/schemas/event/trench_template_dropped.json` (NEW)
- `game/crates/cf-replay/schemas/event/trench_cover_state_changed.json` (NEW)
- `game/crates/cf-replay/schemas/event/trench_breastwork_breached.json` (NEW)
- `game/crates/cf-replay/schemas/event/trench_drainage_flushed.json` (NEW)
- `game/crates/cf-replay/schemas/event/trench_segment_collapsed.json` (NEW)
- `game/content/trench_segments/shallow_scrape.ron` (NEW)
- `game/content/trench_segments/standard.ron` (NEW)
- `game/content/trench_segments/deep.ron` (NEW)
- `game/content/trench_segments/communication.ron` (NEW)
- `game/content/trench_segments/fire_step.ron` (NEW)
- `game/content/trench_segments/parapet_raised.ron` (NEW)
- `game/content/trench_modules/duckboard.ron` (NEW)
- `game/content/trench_modules/fire_step_module.ron` (NEW)
- `game/content/trench_modules/breastwork.ron` (NEW)
- `game/content/trench_modules/drainage_sump.ron` (NEW)
- `game/content/trench_modules/revetment.ron` (NEW)
- `game/content/trench_modules/corner_traverse.ron` (NEW)
- `game/content/trench_templates/wwi_frontline_a.trench.ron` (NEW)
- `game/content/trench_templates/wwi_frontline_b_two_line.trench.ron` (NEW)
- `game/content/trench_templates/reactor_defense_zigzag.trench.ron` (NEW)
- `game/content/trench_templates/forward_outpost_with_mgnest.trench.ron` (NEW)
- `game/content/scenarios/m9b_zigzag_baseline.ron` (NEW)
- `game/content/scenarios/m9b_two_line_defense.ron` (NEW)
- `game/content/scenarios/m9b_fire_step_duel.ron` (NEW)
- `game/content/scenarios/m9b_drainage_flood.ron` (NEW)
- `game/content/scenarios/m9b_reactor_defense_zigzag.ron` (NEW)
- `game/content/scenarios/m9b_template_drop_test.ron` (NEW)
- `game/content/scenarios/m9b_ai_in_trench_doctrine.ron` (NEW)
- `game/content/scenarios/m9b_breastwork_breach.ron` (NEW)
- `game/Cargo.toml` (MODIFY: register cf-trench)

## Acceptance criteria

```gherkin
Scenario: Player digs a standard trench segment with entrenching tool
  Given a player on flat dirt terrain holding entrenching_tool
  When the player issues act.player.dig_trench_segment { variant: "standard" }
  Then carving completes over 12 seconds (4× a shallow_scrape)
  And trench_segment_dug event fires with variant=standard + depth=16 + width=16
  And M3 per-pixel material is removed; M14E integrity field locks segment boundary
  And a duckboard module is auto-placed at the floor
  And cf-render-2d renders duckboard sprite + dirt-wall revetment placeholder

Scenario: Cover state derives from stance × segment variant
  Given a player standing inside a deep trench segment
  When observe.actor.cover_state is read
  Then it returns "Full" (deep variant: even standing the head is below grade)
  When the player switches to fire_step variant by moving to that segment
  And remains standing on the step
  Then cover_state returns "Exposed" (fire-step deliberately exposes torso)
  When the player crouches off the step
  Then cover_state returns "Full"
  And trench_cover_state_changed event fires on each transition

Scenario: Zigzag pattern generator produces no straight runs > 12 tiles
  Given a procgen request for a 60-tile trench line with world_seed=42
  When cf-procgen::trench_generator builds the polyline
  Then every straight segment length is ≤ 12 tiles
  And every kink is ±45° (no obtuse 22.5° intermediate angles)
  And the same world_seed produces an identical kink sequence across two engines
  And a single enfilade ray cast along the line cannot intersect more than 12 contiguous tiles of trench floor

Scenario: Communication trench branches connect firing line to rear
  Given the zigzag generator with branch_every=20 segments
  When generation completes
  Then perpendicular `communication` variant segments connect the front line to a rear-line polyline
  And M22 pathfinding reports a valid actor path from rear-spawn to forward fire-step entirely inside trench floor
  And no path step exits the trench (full-cover route)

Scenario: Trench template `wwi_frontline_a` drops into world deterministically
  Given an empty 200×60 sandbox map
  When the player issues act.player.drop_trench_template { id: "wwi_frontline_a", origin: (50, 30) }
  Then trench_template_dropped event fires with template_sha256 + segment_count
  And every declared segment + embedded module is placed
  And two engines with same seed produce identical SaveBlob.checksum at tick 60

Scenario: Drainage sump flushes accumulated water
  Given a `deep` trench segment with drainage_sump module + heavy rain (M31)
  When 600 ticks elapse
  Then water tiles in the trench floor stay ≤ 2 px deep (sump flushes at ≥ 2 px threshold)
  And trench_drainage_flushed event fires per flush cycle
  When the player demolishes the drainage_sump module
  Then water accumulates beyond 2 px within the next 600 ticks
  And the player's footing converts to wet-mud (M3 per-pixel slippery flag)

Scenario: Breastwork degrades + breaches under sustained MG fire
  Given a parapet_raised trench segment with full breastwork (HP 400)
  When an MG nest (M9C) fires 80 rounds × 6 J across the breastwork wall
  Then breastwork HP decreases per per-pixel material erosion (M14)
  And when HP reaches 0, trench_breastwork_breached event fires
  And cover_state for the segment downgrades from Full to Partial
  And the actor inside loses head-zone cover; M14 routes future hits through the gap

Scenario: AI doctrine garrisons trench and uses cover state correctly
  Given m9b_ai_in_trench_doctrine scenario: 3 AI defenders in a fire_step segment line
  When an enemy advances within engagement range
  Then AI-TRENCH-A-01 doctrine has each AI: step up (Exposed) → fire 1-3 round burst → step down (Full)
  And ai.cover_decision event fires with reason_label="step_up_for_shot" or "step_down_to_reload"
  And no AI remains Exposed continuously > 1.5 seconds (correct burst-and-duck behavior)

Scenario: Revetment prevents wall slough on hardness-0.2 dirt
  Given a `standard` trench in soft dirt (hardness 0.2) without revetment
  When 1800 ticks elapse (3 in-game minutes)
  Then trench_segment_collapsed event fires for ≥ 1 segment (walls slough naturally)
  Given the same trench with revetment module on both walls
  When 1800 ticks elapse
  Then no trench_segment_collapsed event fires (revetment locks M14E integrity ≥ 600)

Scenario: Cover indicator HUD chevron updates per-tick
  Given a player moving from open ground → shallow_scrape → standard → deep trench
  When the player crosses each segment boundary
  Then the HUD chevron updates to: Exposed → Partial → Partial → Full (matching variant × current stance=standing)
  And the chevron has 3 distinct visual states (icon + tint) per accessibility-friendly palette

Scenario: Determinism across full M9B pipeline
  Given two engines running m9b_reactor_defense_zigzag with same world_seed
  When 3600 ticks elapse (60s scenario)
  Then identical event sequence (trench_segment_dug, trench_cover_state_changed, trench_drainage_flushed)
  And identical SaveBlob.checksum at tick 3600
  And both engines render identical zigzag layout (visual diff = 0 px)

Scenario: Trench template validation rejects malformed RON
  Given a modder-authored `bad_template.trench.ron` with unknown segment variant "ultra_deep"
  When cf-mod validates the file at load
  Then validation fails with error="unknown_segment_variant: ultra_deep"
  And the template does not appear in the available template list
  And no panic occurs in cf-content::trench_templates loader

Scenario: Procgen rotted trenches in PvE ruins (M43)
  Given an M43 PvE survival procgen pass on a `ruined_frontline` biome
  When the world generates
  Then 2-4 trench template instances appear with `decay_factor: 0.4` (collapsed segments, missing duckboards)
  And the player can repair via act.player.repair_trench_module (consumes wood/iron)
  And gameplay rewards exploration of "what was here before" alongside fortification reuse
```

## Out of scope

- Underground tunnel networks (player digs straight down + connects with horizontal tunnel) — M30C cave-in + M14E owns the structural side; M9B is surface trench only.
- Trench-vs-vehicle anti-tank ditch — M9C ships dragon's teeth + anti-tank ditch as static-fortification content.
- Per-segment electrified-fence overlay — M9C owns electrified-fence kernel; M9B trenches consume that kernel only when a M9C fence is placed at the trench rim.
- Naval/amphibious assault trench (beach landing breach) — M44 amphibious content; future.
- Gas-warfare integration with trench atmosphere pockets (heavier-than-air gas pools in trenches) — M19/M28 atmospheric kernel forward-compatible; M9B emits the trench segment volume so a future spec can hook it.
- Trench foot / immersion-foot affliction tied to wet duckboards — M16A affliction kernel forward-compatible; M9B exposes `wet_duckboard_seconds` per actor so a future affliction spec can consume.
- Per-segment camouflage tarpaulin overlay (visual concealment from observers) — M9C camo netting covers this as a separate module.

## Dependencies

- M3 chunked terrain + per-pixel carve (done): segment carving consumer
- M9 Reactor Defense + 5-tier terrain HP + per-pixel integrity (done): cover-state pre-existing pixel data
- M14 collision + impulse routing (active): cover-routing hit logic
- M14E per-pixel structural integrity (active): revetment integrity-field anchor
- M14F lateral wall collapse (active): trench wall slough consumer
- M22 AI pathfinding (active): communication trench full-cover route reachability
- M28A base build mode UX (active): dig-trench as a build palette entry; demolish trench module as build-mode demolish
- M28F blueprints + zones (active): trench templates are M28F-style assets
- M30B mining tool tier ladder (active): entrenching_tool + pickaxe-as-trench-tool
- M30C cave-in physics (active): trench wall integrity consumer
- M31 weather (active): rainfall fills trenches; drainage_sump consumer
- M44D combat ground vehicles (active): tank-vs-trench interaction (tank cannot enter narrow trench; can lay suppressing fire over parapet)

## Notes for the implementer

- Cover state is **derived, not stored**: at every frame, `cover_state(actor) = lookup(segment_at(actor.pos)) × actor.stance`. Do NOT cache per-tick; the lookup is O(1) on a chunk-keyed segment index.
- The zigzag generator is deterministic but visually varied — seed off `(world_seed, polyline_hash)`. Two players on the same map see the same layout; two different polylines on the same map see independent kinks.
- Trench templates support `placeholder: { segment_id, fortification_id }` so M9C MG nests can be referenced before M9C ships — load gracefully with a "missing fortification" warning event.
- The `fire_step` variant is the **interesting** one: it deliberately exposes the actor. Make sure the HUD chevron flashes red when stepping up (player must consciously trade cover for shot). AI doctrine AI-TRENCH-A-01 burst-and-duck behavior is the primary CC-parity demonstration.
- `entrenching_tool` is a new T0 tool (cheap, slow): 5 dirt + 1 wood; digs shallow_scrape in 5s, standard in 12s. Higher-tier pickaxes from M30B dig faster but use stamina.
- Per-variant placement validation: `deep` requires `parent material.hardness < 0.5` (cannot dig through concrete/basalt); fall back to shallow_scrape with warning event.
- Sandbag breastwork consumes M9C-owned sandbag inventory; if M9C not yet shipped, parapet_raised variant fails validation with `requires_m9c=true` event (forward-compat).
- Reuse M14E `terrain.material_state_changed` and M18 `terrain.terrain_cascade` for collapse cascades — no new collapse event family in M9B.
- Procgen rotted trenches use `decay_factor: f32 (0.0..1.0)`; decay drives per-segment integrity penalty + per-module missing-rate.
- All authored content (segments / modules / templates) lives under `content/trench_*/` and is fully modder-overridable via `cf-mod`. Validation rejects unknown enums up front to avoid runtime panics.
