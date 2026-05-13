# Coherence Tier 3 — Polish & Consolidation

**Status:** `active` — nice-to-have; can run in parallel with Tier 4
**Prerequisite:** Tier 1 + Tier 2 PRs merged
**Estimated effort:** AI-scale 45-60 minutes (single PR, 4 commits)
**Output:** 1 PR titled `specs: tier-3 coherence polish (M2.5 split + storyteller API + cross-refs + procgen 12-worlds)`

---

## Goals

Polish work that reduces spec-maintenance friction over time:

1. **Edit 3.1** — Split M2.5 into M2.5 (scenario) + M2.5-SCHEMA (event surface lock)
2. **Edit 3.2** — Add storyteller event registration API to M7
3. **Edit 3.3** — Add cross-reference headers to damage-model specs (M5 / M5.5 / M5.6 / M5.7 / M5.8)
4. **Edit 3.4** — Add procgen 12-worlds acceptance criterion to M11.5

After Tier 3 PR merges:
- M2.5 scenario implementer doesn't need to grok 1985 lines of event surface lock
- M7 storyteller has a registration API so downstream events register cleanly
- Damage-model specs have explicit "canonical owner" markers
- M11.5 procgen verifies all 12 worlds, not just 3 launch worlds
- 40 → 41 active specs

---

## Edit 3.1 — Split M2.5 into M2.5 (scenario) + M2.5-SCHEMA (event surface)

### Problem

`specs/active/M2.5.md` is **1985 lines** — the largest active spec. It covers:

1. **Reactor defense scenario** — playable 60-90s mission (the actual scenario)
2. **Massive event-surface lock** for the entire damage model: 18 afflictions, 5 hazards, 3-layer armor, War Thunder angle math, spalling, HE/HEAT/APFSDS, internal organs/circuits, concussion/internal_shock, fluid drain, origin reaction, atmospherics placeholders, shield placeholders, environment.signal_delta, 8 launch materials affordances

The implementer reading 1985 lines thinks they're shipping reactor defense; they're actually locking event schemas for M5/M5.5/M5.6/M5.7/M5.8/M5.9/M5.10.

### Fix

Split into 2 milestones:

| Milestone | Scope |
|---|---|
| **M2.5 — Reactor Defense Scenario** | The playable 60-90s scenario / reactor as static actor / trench gameplay / per-pixel integrity tiers / 8 launch materials affordances |
| **M2.5-SCHEMA — Deep Damage Event Surface Lock** | All `armor.*` / `internal.*` / `concussion.*` / `fluid.*` / `origin.*` / `atmos.*` / `shield.*` / `affliction.*` / `hazard.*` / `environment.*` / `thermal.*` event schemas at v0.1 (M3A locks; M5+ fills producers) |

Both ship in BP3. Implementers work in parallel: scenario implementer + schema-lock implementer.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M2.5.md` | **MODIFY** (strip to scenario only) |
| `specs/active/M2.5-SCHEMA.md` | **CREATE** (cut event surface lock from M2.5) |
| `README.md` | **MODIFY** (add M2.5-SCHEMA to BP3; update spec count) |

### Step 1: Create `specs/active/M2.5-SCHEMA.md`

Extract all event-schema-lock sections from M2.5 and put them here:

- Section "M2.5 firehose surface — what M3A MUST handle without renaming" (large event family table)
- "War Thunder-style armor simulation — angled armor / deflection / spalling / 7+ armor types"
- "Per-limb armor model + armor-piercing math (M5 forward-compat)"
- "Internal damage system — organs (humans/androids) + circuitry (robots)"
- "Concussion + Internal Shock dose accumulator"
- "Fluid drain system (robots / mechs / power suits)"
- "Sound clip variants per armor material + impact state"
- "Atmospheric coupling — reactor venting (M5.9 forward-compat)"
- "Shield surface forward-compat (M5+ chassis + M7+ base)"
- "Affliction taxonomy surface (M5.7 forward-compat)"
- "Hazard taxonomy — 5 launch hazards"
- "Damage attribution depth + cause chain extension"
- "Origin reaction + force feedback"
- All `combat.projectile_hit_mo` expanded payload definitions

Create the file with this structure:

```markdown
# M2.5-SCHEMA — Deep Damage Event Surface Lock

## Status

`active`

## Intent

**M2.5-SCHEMA is the event-surface lock milestone** — the canonical schema for every event family that the damage / hazard / affliction / armor / internal / fluid / origin / atmospherics / shield / environment / thermal kernels will emit from M5 onward.

Per M3A's locked v0.1 envelope: schemas declared here are **additive-only** for the rest of the project. New event types can be ADDED without bumping the envelope; existing event types cannot CHANGE shape.

M2.5-SCHEMA exists separately from M2.5 (Reactor Defense Scenario) because they have different implementer audiences:
- **M2.5** implementer ships a playable scenario
- **M2.5-SCHEMA** implementer locks event schemas (no scenario work)

Both close BP3 closure gate; they're sister milestones.

M2.5-SCHEMA promise: **"every damage event from M2.5 forward emits the structured event family — no schema bump cascades when producers ladder up at M5/M5.5/M5.6/M5.7/M5.8/M5.9/M5.10."**

## What M2.5-SCHEMA does

M2.5-SCHEMA does NOT ship producer code. It ships:

1. Event schema JSON files at `game/crates/cf-replay/schemas/event/<family>_<type>.json`
2. Locked v0.1 envelope conformance (all schemas validate against the M3A locked envelope)
3. `cf-mod validate` rules for each event family
4. Cross-references to which milestone fills the producer for each event family

## Event families locked at M2.5-SCHEMA

[PASTE the "M2.5 firehose surface — what M3A MUST handle without renaming" table from M2.5 here]

## Detailed event schemas (per family)

### armor.* family (M5+ chassis fills)

[PASTE the full armor.* event details from M2.5 here — armor.layer_hp_changed, armor.layer_critical, armor.layer_destroyed, armor.chunked_off, armor.spalling, armor.angle_deflection_calculated, armor.ricochet, armor.penetration_ray_traversed, armor.he_overpressure_wave, armor.heat_jet_penetrated, armor.heat_jet_pre_detonated_by_era, armor.apfsds_penetrated, armor.era_panel_detonated, armor.schurzen_pre_detonated, armor.multi_hit_degradation, armor.reactive_armor_consumed]

### internal.* family (M5.5+ ray traversal fills)

[PASTE the full internal.* event details from M2.5 here]

### concussion.* family (M5.8 origin model fills)

[PASTE the concussion.* and internal_shock.* event details from M2.5 here]

### fluid.* family (M5+ chassis fluid system fills)

[PASTE the fluid.* event details from M2.5 here]

### origin.* family (M5.8 origin model fills)

[PASTE the origin.* event details from M2.5 here]

### hazard.* family (M5.7 hazard package fills)

[PASTE the hazard.* event details from M2.5 here]

### affliction.* family (M5.7 affliction layer fills)

[PASTE the affliction.* event details from M2.5 here]

### atmos.* family (M5.9 atmospherics kernel fills)

[PASTE the atmos.* event details from M2.5 here]

### shield.* family (M5+ chassis + M7+ base fill)

[PASTE the shield.* event details from M2.5 here]

### environment.* family (M5.10 aggregator fills)

[PASTE the environment.* event details from M2.5 here]

### thermal.* family (M5.7+ material kernel fills)

[PASTE the thermal.* event details from M2.5 here]

### combat.projectile_hit_mo expanded payload

[PASTE the combat.projectile_hit_mo expanded payload spec from M2.5 here]

### War Thunder-style armor angle math + ricochet formulas

[PASTE the angle math + ricochet probability + per-ammo round tier table from M2.5 here]

### Per-limb armor + AP rounds — damage routing through layers

[PASTE the armor item schema + per-zone armor slot mapping + damage routing flow from M2.5 here]

### Internal damage system — organs (humans/androids) + circuits (robots)

[PASTE the human organ graph (15 organs) + robot circuit graph (12 internal modules) + damage routing flow from M2.5 here]

### Concussion + Internal Shock dose accumulator

[PASTE the 5-band accumulator (Clear/Mild/Moderate/Severe/KO_Imminent/KO) + decay rules + robot equivalent from M2.5 here]

### Fluid drain system

[PASTE the 4 fluid reservoirs + leak mechanics + ignition risk from M2.5 here]

### Sound clip variants per armor material + impact state

[PASTE the sound clip table per material + impact state from M2.5 here]

## Acceptance criteria

```gherkin
Scenario: All event family schemas exist at v0.1
  Given M2.5-SCHEMA closure
  Then game/crates/cf-replay/schemas/event/ contains JSON files for all locked families
  And cf-mod validate game/crates/cf-replay/schemas/ exits 0
  And each schema declares schema_version="0.1" matching the M3A locked envelope

Scenario: Schemas accept producer events from later milestones additively
  Given M5 ships chassis.armor_layer_destroyed
  When cf-replay envelope validates the event
  Then it conforms to armor.layer_destroyed.json (just with bound_zone added)
  And no envelope bump required (additive payload extension)

Scenario: cf-mod validate covers all M2.5-SCHEMA families
  Given content/* references events from any M2.5-SCHEMA family
  When cargo run -p cf-mod -- validate content/ runs
  Then exit 0 if events conform to schemas
  And exit non-zero with structured error if event shape drifts

Scenario: M3A locks the envelope at v0.1 (cross-reference)
  Given M3A's envelope schema
  Then M2.5-SCHEMA's per-event schemas all conform to it
  And bumping schema_version requires migration tooling (deferred to BP6+ per M3A)
```

## Dependencies

- **M3A (event recorder; must be done OR concurrent)** — M2.5-SCHEMA's schemas conform to M3A's locked envelope

## Closure procedure

Reference bundle (any M2.5-scenario bundle that produces M2.5-SCHEMA-conformant events). All schemas exist; all cf-mod validation passes. Move M2.5-SCHEMA.md → done/.

## Cross-DR

DR-002 (locked envelope), DR-024.

## Implementer notes

M2.5-SCHEMA is purely a **declarative milestone**. The implementer:

1. Reads M2.5-SCHEMA.md (this file)
2. Creates one JSON schema file per event family member at `game/crates/cf-replay/schemas/event/<family>_<type>.json`
3. Runs `cargo run -p cf-mod -- validate game/crates/cf-replay/schemas/` to verify conformance
4. Does NOT touch any producer code (M5/M5.5/M5.6/M5.7/M5.8/M5.9/M5.10 implementers do that)

Each schema follows this skeleton (per M3A's envelope):

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "armor.layer_destroyed",
  "type": "object",
  "properties": {
    "schema_version": { "const": "0.1" },
    "category": { "const": "armor" },
    "event_type": { "const": "layer_destroyed" },
    "actor_id": { "type": "integer" },
    "tick": { "type": "integer" },
    "payload": {
      "type": "object",
      "properties": {
        "item_id": { "type": "integer" },
        "zone": { "type": "string", "enum": ["head", "torso", "arm_left", ...] },
        "layer": { "type": "string", "enum": ["External", "Internal", "Core"] },
        "breach_kind": { "type": "string", "enum": ["punctured", "shattered", "melted", "chemically_corroded"] }
      },
      "required": ["item_id", "zone", "layer", "breach_kind"]
    }
  },
  "required": ["schema_version", "category", "event_type", "tick", "payload"]
}
```

Counting: M2.5-SCHEMA registers ~60-80 event schemas across 11 families (armor, internal, concussion, fluid, origin, hazard, affliction, atmos, shield, environment, thermal + combat.projectile_hit_mo expanded payload). One JSON file per event type.
```

### Step 2: Modify `specs/active/M2.5.md`

Remove the sections that moved to M2.5-SCHEMA. Keep ONLY:

- Title + Status + Intent (rewrite to focus on scenario)
- Information priority — what the player sees vs feels
- Player narrative flow (60-90 seconds)
- Destructible terrain — 5-tier HP color states
- 8 launch materials + color signatures + affordance flags
- Trench gameplay depth — partial cover + erosion
- Tutorial-safety scenario variant
- Cause chain (just the high-level narrative cause; full schema is M2.5-SCHEMA)
- Crates / modules touched (scenario-relevant only)
- Acceptance criteria (scenario-relevant subset)
- Out of scope
- Dependencies
- Notes for the implementer (scenario-specific)

Update **Intent** to focus on scenario:

```markdown
## Intent

**M2.5 is the reactor defense scenario milestone** — a playable 60-90 second `micro_reactor_defense` scenario proving M2 chunked terrain is fun in player's hands. After M2.5, players can dig a trench around a reactor, hold position while one reactive enemy attacks, win if the reactor survives 60s timer, lose if it explodes.

**M2.5-SCHEMA is its sister milestone** — it locks the canonical event surfaces for the deep damage / hazard / affliction / armor / internal / fluid / origin / atmospherics / shield / environment / thermal kernels that ladder up at M5/M5.5/M5.6/M5.7/M5.8/M5.9/M5.10. M2.5 ships the scenario; M2.5-SCHEMA ships the schemas.

Both close in BP3 alongside M2.2A + M2.2B + M2.2C + M3A + M3B + M4A. BP3 closure gate runs across all 7 milestones.

M2.5 promise: **"defend the reactor for 60 seconds; the trench you dig matters."**
```

Add a cross-reference note in the M2.5 Crates section:

```markdown
**Cross-reference:** All event schema definitions (full armor.* / internal.* / concussion.* / fluid.* / origin.* / hazard.* / affliction.* / atmos.* / shield.* / environment.* / thermal.* families + combat.projectile_hit_mo expanded payload) live in `specs/active/M2.5-SCHEMA.md`. M2.5 (this file) fires placeholder events that conform to those schemas; M5/M5.5/M5.6/M5.7/M5.8/M5.9/M5.10 fill the producers.
```

### Step 3: Modify `README.md`

Find the active spec count badge (was 40 after Tier 2):

**BEFORE:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-40%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

**AFTER:**
```markdown
[![Specs](https://img.shields.io/badge/active%20specs-41%20%28M2.2A..M12%29-blueviolet?style=flat-square)](specs/active/)
```

Find the BP3 row for M2.5 in the Build Points table. Add a new row immediately AFTER M2.5:

```markdown
| BP3 | **M2.5-SCHEMA — Deep Damage Event Surface Lock** | Planned | Locks v0.1 event schemas for armor / internal / concussion / fluid / origin / hazard / affliction / atmos / shield / environment / thermal families + combat.projectile_hit_mo expanded payload (~60-80 event schemas across 11 families). Pure schema-lock milestone; M2.5 ships scenario, M2.5-SCHEMA ships schemas. Both close BP3 together. M5+ fills producers. |
```

### Acceptance criteria for Edit 3.1

```bash
# File exists
test -f specs/active/M2.5-SCHEMA.md && echo "PASS: M2.5-SCHEMA.md exists" || echo "FAIL"

# M2.5 has been trimmed (was 1985 lines; should be much less)
[ "$(wc -l < specs/active/M2.5.md | tr -d ' ')" -lt 1200 ] && echo "PASS: M2.5 < 1200 lines" || echo "FAIL: $(wc -l < specs/active/M2.5.md) lines"

# M2.5-SCHEMA has the moved content
grep -q "armor.layer_destroyed" specs/active/M2.5-SCHEMA.md && echo "PASS: armor family in schema" || echo "FAIL"
grep -q "concussion.dose_changed" specs/active/M2.5-SCHEMA.md && echo "PASS: concussion family in schema" || echo "FAIL"
grep -q "origin.shot_force_feedback" specs/active/M2.5-SCHEMA.md && echo "PASS: origin family in schema" || echo "FAIL"

# M2.5 has cross-reference
grep -q "live in.*M2.5-SCHEMA" specs/active/M2.5.md && echo "PASS: M2.5 cross-ref" || echo "FAIL"

# README updated
grep -q "active%20specs-41" README.md && echo "PASS: README badge 41" || echo "FAIL"
grep -q "M2.5-SCHEMA" README.md && echo "PASS: README BP3 lists M2.5-SCHEMA" || echo "FAIL"

# Total spec count
test "$(ls specs/active/M*.md | wc -l | tr -d ' ')" = "41" && echo "PASS: 41 active specs" || echo "FAIL: $(ls specs/active/M*.md | wc -l) specs"
```

### Commit message for Edit 3.1

```
specs: Edit 3.1 — split M2.5 into scenario + event surface lock

M2.5 was 1985 lines: the largest active spec. It bundled a playable
reactor defense scenario with massive event surface locks for the
damage / hazard / affliction / armor / internal / fluid / origin /
atmos / shield / environment / thermal kernels.

The implementer reading 1985 lines thinks they're building reactor
defense; they're actually locking event schemas for 7+ later milestones.

Split into 2 milestones:

- M2.5 — Reactor Defense Scenario (playable scenario, ~500-1000 lines)
- M2.5-SCHEMA — Deep Damage Event Surface Lock (~60-80 event schemas
  across 11 families; pure declarative milestone)

Both close BP3 together. Implementers work in parallel.
```

---

## Edit 3.2 — Add storyteller event registration API to M7

### Problem

`specs/active/M7.md` § Storyteller / incident director defines 5 storytellers but doesn't specify how downstream milestones (M5.7 hazards, M7.7 weather, M11.7 PvE events, M12 MMO events) register their events.

Without a registration API, each downstream milestone has to either:
1. Hardcode events in M7's cf-storyteller crate (M7 must know about M11.7's bosses — backward dep)
2. Define events in their own crates with parallel storyteller logic (drift risk)

### Fix

Add an explicit event registration API to M7's `cf-storyteller` crate. Downstream milestones register events at scenario load.

### Files to modify

| File | Action |
|---|---|
| `specs/active/M7.md` | **MODIFY** (add storyteller registration API to existing storyteller section) |
| `specs/active/M11.7.md` | **MODIFY** (update to use registration API; add explicit reference) |
| `specs/active/M5.7.md` | **MODIFY** (add note that hazard storyteller events register via M7 API) |

### Step 1: Modify `specs/active/M7.md`

Find the **Storyteller / incident director (Rimworld-inspired)** section. Add a new subsection at the end of it:

```markdown
#### Storyteller event registration API (canonical owner)

Per the layered hybrid AI architecture (DR-008), storyteller events come from MANY owning milestones (M5.7 hazards, M7.7 weather, M11.7 PvE events, M12 MMO events). M7's `cf-storyteller` crate exposes a **registration API** so downstream milestones plug in events without M7 needing to know about them.

```rust
// cf-storyteller/src/registry.rs
pub trait StorytellerEvent: Send + Sync {
    fn id(&self) -> EventId;
    fn name(&self) -> &str;
    fn severity(&self) -> Severity;             // Info | Warning | Critical
    fn cooldown_ticks(&self) -> u32;
    fn intensity_contribution(&self) -> f32;    // 0.0..1.0 — how much it raises tension
    fn trigger_check(&self, ctx: &StorytellerContext) -> bool;
    fn execute(&self, ctx: &mut StorytellerContext) -> Vec<ReplayEvent>;
}

pub struct StorytellerRegistry {
    events: Vec<Box<dyn StorytellerEvent>>,
}

impl StorytellerRegistry {
    pub fn register<E: StorytellerEvent + 'static>(&mut self, event: E) -> EventId { ... }
    pub fn unregister(&mut self, id: EventId) { ... }
    pub fn list(&self) -> &[Box<dyn StorytellerEvent>] { ... }
}
```

**Per-storyteller event filtering:**

Each storyteller (Cassandra / Phoebe / Randy / Ironman / Sandbox) configures which event categories + severity levels it allows:

| Storyteller | Allowed event categories | Severity cap |
|---|---|---|
| Cassandra Classic | All | Critical (balanced pacing) |
| Phoebe Chillax | All except `pirate_raid` + `plague_outbreak` | Warning (player-friendly) |
| Randy Random | All | Critical (chaotic; no cooldown discipline) |
| Ironman | All | Critical (no respawn; high stakes) |
| Sandbox | None (events disabled) | Info (player drives all events manually) |

**Per-scenario event whitelist:** scenarios can override storyteller defaults via `scenario.ron`:

```ron
ScenarioStorytellerConfig (
    storyteller: "phoebe_chillax",
    allowed_events: ["solar_flare", "trader_arrival"],  // override default whitelist
    blocked_events: ["pirate_raid"],
    intensity_curve_override: Some(IntensityCurve::Custom(... )),
)
```

**Events registered by other milestones:**

| Registering milestone | Events |
|---|---|
| **M5.7 (hazard package)** | `fire_spread_event` / `electric_arc_cascade` / `acid_pool_growth` / `radiation_storm` |
| **M5.9.5 (micro pressure hold)** | `pressure_breach_event` |
| **M7.7 (weather + day/night)** | `storm_arriving` / `lightning_strike` / `dust_storm` / `eclipse` |
| **M7.1 (factions)** | `faction_war_declared` / `trader_arrival` / `pirate_raid` / `mass_attack` |
| **M11.7 (PvE endgame)** | `solar_flare` / `volcanic_eruption` / `asteroid_impact` / `mysterious_signal` / `famine` / `plague_outbreak` / `mineral_rush` |
| **M12 (MMO)** | `cross_shard_event` / `world_boss_spawn` |

All registrations happen at scenario load via `cf-storyteller::registry.register(event)`. The cf-storyteller crate doesn't know what events exist; it just runs the registered set against the current storyteller's filter.

**Events emit `storyteller.event_triggered`:**

```rust
pub struct StorytellerEventTriggered {
    pub event_id: EventId,
    pub name: String,
    pub severity: Severity,
    pub intensity_at_trigger: f32,
    pub cause: TriggerCause,                    // Threshold | Cooldown | Manual
}
```

This is the M3A-locked event shape; downstream events must conform.
```

### Step 2: Modify `specs/active/M11.7.md`

In the "12 dynamic world events" section, add at the top:

```markdown
**Registration API:** All 12 events register via M7's `cf-storyteller::registry.register()` API at scenario load. M11.7 ships the event DATA + trigger logic + effects; M7 owns the registry + scheduling. See `specs/active/M7.md` § Storyteller event registration API for the canonical contract.
```

### Step 3: Modify `specs/active/M5.7.md`

In the "Hazard tile full mechanics" section (or wherever storyteller-triggered hazard events are discussed), add:

```markdown
**Storyteller integration:** Hazard-spawned events (fire_spread_event / electric_arc_cascade / acid_pool_growth / radiation_storm) register via M7's `cf-storyteller::registry.register()` API. M5.7 ships hazard mechanics + storyteller event hooks; M7 owns the storyteller runtime. See `specs/active/M7.md` § Storyteller event registration API.
```

### Acceptance criteria for Edit 3.2

```bash
# M7 has the storyteller API spec
grep -q "Storyteller event registration API" specs/active/M7.md && echo "PASS: M7 has API section" || echo "FAIL"
grep -q "pub trait StorytellerEvent" specs/active/M7.md && echo "PASS: M7 defines trait" || echo "FAIL"
grep -q "pub struct StorytellerRegistry" specs/active/M7.md && echo "PASS: M7 defines registry" || echo "FAIL"

# Downstream specs reference the API
grep -q "cf-storyteller::registry.register" specs/active/M11.7.md && echo "PASS: M11.7 references API" || echo "FAIL"
grep -q "Storyteller integration" specs/active/M5.7.md && echo "PASS: M5.7 references API" || echo "FAIL"
```

### Commit message for Edit 3.2

```
specs: Edit 3.2 — add storyteller event registration API to M7

M7's storyteller didn't specify how downstream events (M5.7 hazards,
M7.7 weather, M11.7 PvE events, M12 MMO) register. Each milestone was
either hardcoding events in M7 (backward dep) or running parallel
storyteller logic (drift risk).

Added explicit StorytellerEvent trait + StorytellerRegistry in M7
with cf-storyteller::registry.register() API. M11.7 + M5.7 updated
to reference this API.

Per-storyteller event filtering (Cassandra / Phoebe / Randy / Ironman /
Sandbox) + per-scenario whitelist documented.
```

---

## Edit 3.3 — Add cross-reference headers to damage-model specs

### Problem

The damage model is duplicated across:
- M2.5 (event surface lock — now M2.5-SCHEMA after Edit 3.1)
- M5 (chassis grammar + 3-layer armor)
- M5.5 (full collision + impulse routing + spalling)
- M5.6 (material kernel + reactions)
- M5.7 (hazards + 22 afflictions; was 18 before Tier 2 Edit 2.4)
- M5.8 (origin reaction + per-origin resource model)

The intentional pattern is "each spec is self-contained for the implementer reading it" — so duplication is by design. But this creates spec-maintenance friction.

### Fix

Add explicit **"Canonical owner"** cross-reference headers at the top of each damage-related section in M5 / M5.5 / M5.6 / M5.7 / M5.8 so implementers know which spec is the source of truth for each sub-model.

This is a documentation-only fix (no content moves; just headers).

### Files to modify

| File | Action |
|---|---|
| `specs/active/M5.md` | **MODIFY** (add canonical-owner headers) |
| `specs/active/M5.5.md` | **MODIFY** (add canonical-owner headers) |
| `specs/active/M5.6.md` | **MODIFY** (add canonical-owner headers) |
| `specs/active/M5.7.md` | **MODIFY** (add canonical-owner headers) |
| `specs/active/M5.8.md` | **MODIFY** (add canonical-owner headers) |

### Step 1: Modify `specs/active/M5.md`

At the very top, immediately after `## Status` and `## Intent`, add:

```markdown
## Canonical ownership (damage model)

This spec is the **canonical owner of chassis damage grammar**: 3-layer armor (External / Internal / Core), 15-zone humanoid body graph, ChassisStage progression (Nominal → Degraded → Wreck), pilot eject mechanics, jam + clear weapon state.

**Related specs (do NOT redefine; reference here):**

| Topic | Canonical owner |
|---|---|
| Per-zone armor item schema (15-slot mapping) | **M5 (this spec)** |
| 3-layer armor cascade (External → Internal → Core) | **M5 (this spec)** |
| Full collision + impulse routing + spalling | **M5.5** |
| Material penetration + chemistry | **M5.6** |
| 22 afflictions + hazard tiles + anomalies | **M5.7** |
| Per-origin reaction matrix + resource model | **M5.8** |
| Event schemas (armor.* / internal.* / etc.) | **M2.5-SCHEMA** |

If you find duplication, the canonical owner is the source of truth.
```

### Step 2: Modify `specs/active/M5.5.md`

Add at the top after Status + Intent:

```markdown
## Canonical ownership (damage model)

This spec is the **canonical owner of collision + impulse routing**: swept-volume collision per CCCP `Atom::Travel`, sharpness decay, multi-actor hit priority queue, limb detachment via joint impulse, gib spawning with authored data, ragdoll-on-death, per-organ internal damage routing, War Thunder-style penetration ray + spalling fragments.

**Related specs:**

| Topic | Canonical owner |
|---|---|
| 3-layer armor + chassis grammar | M5 |
| Collision + impulse routing (this spec) | **M5.5 (this spec)** |
| Material reactions + penetration formulas | M5.6 |
| Hazards + afflictions | M5.7 |
| Per-origin reaction | M5.8 |
| Event schemas | M2.5-SCHEMA |
```

### Step 3: Modify `specs/active/M5.6.md`

Add at the top after Status + Intent:

```markdown
## Canonical ownership (damage model)

This spec is the **canonical owner of active-material kernel**: 50+ materials (M2 ships 8), 30+ reactions, phase transitions with latent heat, alchemy table, flask system, per-pixel cellular automata, GPU compute path, air/heat/gravity fields.

**Related specs:**

| Topic | Canonical owner |
|---|---|
| Chassis grammar + 3-layer armor | M5 |
| Collision + impulse + spalling | M5.5 |
| Material reactions + chemistry (this spec) | **M5.6 (this spec)** |
| Hazards + afflictions | M5.7 |
| Per-origin reaction | M5.8 |
| Event schemas | M2.5-SCHEMA |

Material schema preservation rules from M2 (registry, AddUpdatedMaterialArea, 9 affordance flags) carried forward without exception.
```

### Step 4: Modify `specs/active/M5.7.md`

Add at the top after Status + Intent:

```markdown
## Canonical ownership (damage model)

This spec is the **canonical owner of hazards + afflictions**: 5 hazard tile kinds with spread/dissipation rules, 22 afflictions (18 baseline + 4 survival), 6 STALKER-inspired anomaly hazards, 20+ artifacts, swimming + underwater combat, affliction strip HUD widget.

**Related specs:**

| Topic | Canonical owner |
|---|---|
| Chassis grammar + 3-layer armor | M5 |
| Collision + impulse + spalling | M5.5 |
| Material reactions | M5.6 |
| Hazards + 22 afflictions (this spec) | **M5.7 (this spec)** |
| Per-origin reaction | M5.8 |
| Event schemas | M2.5-SCHEMA |
```

### Step 5: Modify `specs/active/M5.8.md`

Add at the top after Status + Intent:

```markdown
## Canonical ownership (damage model)

This spec is the **canonical owner of origin reaction + per-origin resource model**: 10 launch races / origins, per-origin reaction matrix, no-HP-bar survival resource model (blood / oil / power / caloric / bio_fluid / oxygen_supply), G-Force vignette per origin, helmet breach mechanics, robot internal_shock vs human concussion, robot overclock + downclock.

**Related specs:**

| Topic | Canonical owner |
|---|---|
| Chassis grammar + 3-layer armor | M5 |
| Collision + impulse + spalling | M5.5 |
| Material reactions | M5.6 |
| Hazards + 22 afflictions | M5.7 |
| Origin reaction + resource model (this spec) | **M5.8 (this spec)** |
| 4-tier battery pack ladder | **M7.6** (moved here per Tier 1 Edit 1.4) |
| 5-tier gas tank ladder | **M5.9** (moved here per Tier 1 Edit 1.4) |
| 10-race × env-factor resistance matrix (120 cells) | **M5.10** (moved here per Tier 1 Edit 1.4) |
| Event schemas | M2.5-SCHEMA |
```

### Acceptance criteria for Edit 3.3

```bash
# All 5 specs have canonical ownership headers
for spec in M5 M5.5 M5.6 M5.7 M5.8; do
  grep -q "## Canonical ownership (damage model)" specs/active/${spec}.md && echo "PASS: ${spec} has header" || echo "FAIL: ${spec} missing header"
done

# Each spec correctly identifies itself as the canonical owner of its topic
grep -q "M5 (this spec)" specs/active/M5.md && echo "PASS: M5 self-ref" || echo "FAIL"
grep -q "M5.5 (this spec)" specs/active/M5.5.md && echo "PASS: M5.5 self-ref" || echo "FAIL"
grep -q "M5.6 (this spec)" specs/active/M5.6.md && echo "PASS: M5.6 self-ref" || echo "FAIL"
grep -q "M5.7 (this spec)" specs/active/M5.7.md && echo "PASS: M5.7 self-ref" || echo "FAIL"
grep -q "M5.8 (this spec)" specs/active/M5.8.md && echo "PASS: M5.8 self-ref" || echo "FAIL"
```

### Commit message for Edit 3.3

```
specs: Edit 3.3 — add canonical-owner cross-references to damage model

The damage model spans M5 + M5.5 + M5.6 + M5.7 + M5.8 with intentional
duplication ("each spec is self-contained for the implementer reading
it"). But spec-maintenance friction was growing.

Added "## Canonical ownership (damage model)" headers at the top of
each spec mapping every topic to its canonical owner. Implementers
can now see at a glance which spec is the source of truth.

Documentation-only fix; no content moves.
```

---

## Edit 3.4 — Add procgen 12-worlds acceptance to M11.5

### Problem

`specs/active/M11.5.md` ships 3 launch survival worlds (Earth + Mars + Mimas). M7.7 ships all 12 worlds. M11.5's "9 more post-launch unlock" implies procgen must support all 12 worlds, but the acceptance criteria only verify 3.

### Fix

Add explicit acceptance criterion that M11.5's procgen pipeline runs against ALL 12 worlds (not just the 3 launch survival ones).

### Files to modify

| File | Action |
|---|---|
| `specs/active/M11.5.md` | **MODIFY** (add 12-world acceptance scenario) |

### Step 1: Modify `specs/active/M11.5.md`

Find the **Acceptance criteria** section. Add this scenario at the end:

```gherkin
Scenario: Procgen runs against all 12 worlds in DR-039 catalog
  Given M11.5 closure + DR-039 ships 12 launch worlds (Earth / Mars / Moon / Phobos / Deimos / Mimas / Europa / Vulcan / Venus / Sol-zone / Belt asteroids / Orbital station)
  When `cargo test -p cf-world-generator -- --test procgen_all_12_worlds` runs
  Then procgen completes deterministically for ALL 12 worlds
  And each world's generated topology + biomes + ore distribution + hazards + AI raiders validates against the schema
  And the 3 launch survival worlds (Earth / Mars / Mimas) have full content; the other 9 have valid procgen but "post-launch unlock" flag

Scenario: Procgen deterministic across replay
  Given the same world_seed + world_id (e.g. mars + seed=1234)
  When procgen runs twice
  Then output is byte-identical
  And cf-headless replay reproduces the exact same world

Scenario: Per-world procgen validates against DR-039 catalog
  Given each of 12 worlds
  Then content/worlds/<world_id>.world.ron exists
  And cf-mod validate content/worlds/ exits 0
  And ore_distribution + atmospheric_composition + gravity + temperature_range match DR-039
```

### Acceptance criteria for Edit 3.4

```bash
grep -q "Scenario: Procgen runs against all 12 worlds" specs/active/M11.5.md && echo "PASS: 12-world acceptance scenario" || echo "FAIL"
grep -q "Scenario: Procgen deterministic across replay" specs/active/M11.5.md && echo "PASS: determinism scenario" || echo "FAIL"
grep -q "Scenario: Per-world procgen validates against DR-039" specs/active/M11.5.md && echo "PASS: DR-039 validation scenario" || echo "FAIL"
```

### Commit message for Edit 3.4

```
specs: Edit 3.4 — verify M11.5 procgen runs against all 12 worlds

M11.5 ships 3 launch survival worlds but procgen must support all 12
per DR-039 catalog. Added 3 acceptance scenarios:

- Procgen runs against all 12 worlds
- Procgen deterministic across replay
- Per-world procgen validates against DR-039 catalog
```

---

## Tier 3 — Full acceptance criteria

```bash
cd /Users/erol/projects/corefall

# Edit 3.1 checks
test -f specs/active/M2.5-SCHEMA.md
[ "$(wc -l < specs/active/M2.5.md | tr -d ' ')" -lt 1200 ]
grep -q "armor.layer_destroyed" specs/active/M2.5-SCHEMA.md
grep -q "live in.*M2.5-SCHEMA" specs/active/M2.5.md
grep -q "active%20specs-41" README.md

# Edit 3.2 checks
grep -q "Storyteller event registration API" specs/active/M7.md
grep -q "pub trait StorytellerEvent" specs/active/M7.md
grep -q "cf-storyteller::registry.register" specs/active/M11.7.md

# Edit 3.3 checks
grep -q "## Canonical ownership" specs/active/M5.md
grep -q "## Canonical ownership" specs/active/M5.5.md
grep -q "## Canonical ownership" specs/active/M5.6.md
grep -q "## Canonical ownership" specs/active/M5.7.md
grep -q "## Canonical ownership" specs/active/M5.8.md

# Edit 3.4 checks
grep -q "Scenario: Procgen runs against all 12 worlds" specs/active/M11.5.md

# File count
test "$(ls specs/active/M*.md | wc -l | tr -d ' ')" = "41"

# Workspace still builds
cd game && cargo build && cargo clippy --all-targets -- -D warnings
cd ..

echo "TIER 3 — ALL CHECKS PASS"
```

### Tier 3 PR template

**Title:** `specs: tier-3 coherence polish (M2.5 split + storyteller API + cross-refs + procgen 12-worlds)`

**Body:**

```markdown
## Summary

Tier 3 of the spec coherence pass per `specs/COHERENCE-PLAN.md`. Polish work that reduces spec-maintenance friction:

1. **Edit 3.1** — Split M2.5 into M2.5 (scenario) + M2.5-SCHEMA (event surface lock)
2. **Edit 3.2** — Add storyteller event registration API to M7 (StorytellerEvent trait + StorytellerRegistry)
3. **Edit 3.3** — Add canonical-owner cross-reference headers to M5 / M5.5 / M5.6 / M5.7 / M5.8
4. **Edit 3.4** — Add procgen 12-worlds acceptance criteria to M11.5

## Active spec count

- Before: 40
- After: 41 (added M2.5-SCHEMA)

## Verification

All acceptance checks from `COHERENCE-TIER-3.md` § Tier 3 — Full acceptance criteria. All PASS.

## Next

Can run in parallel with `COHERENCE-TIER-4.md`. Both can be combined into a single PR if desired.
```

---

## Done with Tier 3

Once the PR merges:
- ✅ M2.5 has 2 sister specs (scenario + schema)
- ✅ M7 storyteller has a registration API
- ✅ Damage model specs have canonical-owner cross-references
- ✅ M11.5 procgen verifies all 12 worlds
- ✅ 41 active specs

**Proceed to `COHERENCE-TIER-4.md`** for gap-filling additions (can run in parallel).
