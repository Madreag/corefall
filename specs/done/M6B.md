# M6B — Item Schema Canonicalization + Per-Item Weight + Grid Dimensions + Inventory Encumbrance

## Status

`done`

## Intent

Lock the canonical `ItemSpec` schema with explicit per-item `mass_kg`, Tetris-grid `dimensions { w, h }`, `bulk_volume_l`, per-origin `carry_capacity_modifier`, and the per-actor `inventory_encumbrance` aggregator that feeds M14A mass aggregation, M27 Tetris UX, and M14A walk-speed modifier — the foundational schema every other equipment/loot/crafting milestone references.

## Canonical ownership

Canonical owner of the per-item physical schema (mass + dimensions + bulk + slot + container nesting + per-origin carry caps). Consumed by M6C (equipment SKUs), M27 (Tetris grid), M27B (loot tables), M32C (crafting outputs), M19N (food storage), M14A (mass aggregation).

## Player-facing behavior

- Every item declares mass in kg + grid dimensions (W×H tiles) + bulk volume (liters; Tarkov parity).
- Per-actor `max_carry_kg` baseline 50 kg; per-origin modifier (heavy_biomech 1.5×; drone 0.3×; android 1.0×; robot 1.2×).
- Encumbrance penalty: walk speed × `lerp(1.0, 0.5, total_carried_kg / max_carry_kg)`.
- Per-actor max carry volume baseline 60 L; same per-origin scaling.
- Backpack tiers expand grid: small (4×6), medium (6×8), large (8×10), industrial (10×12).
- Container nesting allowed up to 2 levels (chest → crate → item; not deeper).
- Stack items declare `stack_mass = item_mass × count`; loose items in containers stack visually but count toward grid + weight individually.
- Liquid containers track full vs empty mass separately (water bottle: 0.2 kg empty + 1.0 L water = 1.2 kg full).
- Hot-swap M14A QAB items declare `quick_slot_eligible = true`.

## `ItemSpec` schema (locked)

```rust
pub struct ItemSpec {
    pub id: ItemId,
    pub display_name: String,
    pub mass_kg: f32,
    pub dimensions: GridDim { w: u8, h: u8 },
    pub bulk_volume_l: f32,
    pub stackable: bool,
    pub max_stack: u16,
    pub category: ItemCategory,
    pub container_capacity: Option<ContainerCapacity>,
    pub liquid_capacity_l: Option<f32>,
    pub rotation_allowed: bool,
    pub quick_slot_eligible: bool,
    pub durability_max: Option<u32>,
    pub repair_recipe: Option<RecipeId>,
    pub material_weight_breakdown: BTreeMap<MaterialId, f32>,
    pub crafting_yield_count: u8,
    pub origin_compatibility: BTreeSet<OriginId>,
    pub forbid_for_origin: BTreeSet<OriginId>,
}

pub enum ItemCategory {
    Weapon, Armor, Tool, Medical, Survival, Sensor, Consumable, Material, Container, Ammo, Magazine, Liquid, Specialty
}
```

## Crates / modules touched

| Crate | Status | What |
|---|---|---|
| `cf-equipment::item_spec` | NEW | ItemSpec schema + registry + validation |
| `cf-actor::inventory` | MODIFY (extend M6) | Per-actor inventory grid; per-actor `max_carry_kg`; encumbrance compute |
| `cf-actor::mass_aggregator` | MODIFY | Feed inventory total mass into M14A total_mass |
| `cf-control` | MODIFY | observe.actor.inventory extended with mass + bulk |
| `cf-ui::inventory_grid` | NEW | Tetris-grid UX (M27 polishes) |
| `cf-mod` | MODIFY | Validate `content/equipment/items/*.ron` against ItemSpec schema |
| `cf-replay` | MODIFY | 4 new event schemas |

## Files

- `game/crates/cf-equipment/src/item_spec.rs` (NEW)
- `game/crates/cf-actor/src/inventory.rs` (MODIFY)
- `game/crates/cf-actor/src/mass_aggregator.rs` (MODIFY)
- `game/crates/cf-ui/src/inventory_grid.rs` (NEW)
- `game/crates/cf-replay/schemas/event/item_picked_up_with_mass.json` (NEW)
- `game/crates/cf-replay/schemas/event/item_dropped_with_mass.json` (NEW)
- `game/crates/cf-replay/schemas/event/encumbrance_threshold_crossed.json` (NEW)
- `game/crates/cf-replay/schemas/event/container_nested.json` (NEW)
- `game/content/equipment/items/manifest.ron` (NEW; lists all ItemSpec IDs)

## Tunable defaults

| Constant | Value |
|---|---:|
| Human baseline max_carry_kg | 50 |
| Heavy_biomech modifier | 1.5× |
| Robot modifier | 1.2× |
| Drone modifier | 0.3× |
| Walk-speed at 100% carry | 0.5× |
| Walk-speed at 50% carry | 0.75× |
| Bulk limit baseline | 60 L |
| Backpack tier 1 grid | 4×6 |
| Backpack tier 4 grid | 10×12 |
| Max container nesting depth | 2 |

## Acceptance criteria

```gherkin
Scenario: Item declares mass + dimensions
  Given rifle_m1 spec with mass=3.5 kg + dimensions 2×4
  When player picks up rifle
  Then item_picked_up_with_mass fires
  And inventory.total_mass += 3.5
  And actor.M14A.total_mass updated

Scenario: Encumbrance at 100% reduces walk speed
  Given a human at max_carry_kg=50 + carrying 50kg
  When walk-speed compute runs
  Then walk_speed_multiplier = 0.5
  And HUD shows "ENCUMBERED" warning

Scenario: Per-origin scaling applies
  Given a heavy_biomech with same load
  Then max_carry_kg = 75 (1.5× baseline)
  And encumbrance threshold differs accordingly

Scenario: Container nesting depth-limited
  Given a chest (level 1) containing a crate (level 2)
  When player tries to nest another container inside the crate
  Then act.player.nest_container rejects with "max_depth_exceeded"

Scenario: Liquid container full vs empty mass
  Given an empty water_bottle (0.2 kg empty + 1L capacity)
  When player fills with 1L water
  Then item mass = 1.2 kg
  When player drinks 500ml
  Then item mass = 0.7 kg

Scenario: Determinism — encumbrance reproduced
  Given identical seed + identical inventory load
  When 100 ticks of movement elapse
  Then identical actor positions per tick
```

## Out of scope

- Tetris drag-drop UX details — M27.
- Loot drop logic — M27B.
- Crafting integration — M32C.
- Per-item visual sprite production — M9A asset pipeline.

## Dependencies

- M6 (done) — base inventory baseline.
- M14A (active) — mass aggregator consumer.
- M27 (active) — Tetris grid UX consumer.

## Notes for the implementer

- ItemSpec is the **single source of truth** — every other equipment/loot/crafting milestone reads from this registry.
- Per-origin carry modifier is multiplicative; stacks with M17 origin matrix.
- Encumbrance penalty curve is deterministic + replay-stable.

## Per-scenario verdict

| Scenario | Verdict | Notes |
|---|---|---|
| Item declares mass + dimensions | IMPLEMENTED | `cf_equipment::item_spec` registry returns `rifle_m1 { mass_kg=3.5, dimensions=2×4 }`; engine `add_to_inventory_grid_mut` adds mass to actor's `inventory_grid`; legacy `inventory_weight_kg` recompute also reads canonical `mass_kg_for_id` (no more hardcoded 8 kg); `equipment.item_picked_up_with_mass` event fires with canonical mass; `mass_aggregator::total_mass` reflects new inventory contribution. Tests: `scenario_item_declares_mass_and_dimensions`, `m6b_pickup_emits_mass_aware_event_and_updates_grid`. |
| Encumbrance at 100% reduces walk speed | IMPLEMENTED | `walk_speed_multiplier(50, 50) = 0.5`; `encumbrance_band` returns Heavy at 100%; `cf-actor::sim` `effective_max_speed` multiplied by `encumbrance_walk_speed_multiplier()`; HUD widget `cf-ui::inventory_grid::InventoryGridState::warning_text() = "ENCUMBERED"` at Heavy band; per-tick recompute in `tick_m6_actor_state` rebaselines envelope + emits `inventory.encumbrance_threshold_crossed` on band transitions. Tests: `scenario_encumbrance_at_100_percent_reduces_walk_speed`, `scenario_encumbrance_table_lerp_curve`, `scenario_encumbrance_reduces_actor_displacement_vs_unloaded`, `m6b_encumbrance_band_transition_fires_event`. |
| Per-origin scaling applies | IMPLEMENTED | `carry_capacity_modifier("heavy_biomech")=1.5×`, `max_carry_kg_for_origin("heavy_biomech")=75`; `InventoryEncumbrance::for_origin` baseline scaled; `actor.max_carry_kg()` returns origin-scaled cap; per-tick `recompute_inventory_encumbrance` rebaselines from `origin_id` so M17 origin swap immediately reflects in the envelope. Tests: `scenario_per_origin_scaling_applies`, `scenario_per_origin_actor_envelope_seeded_from_origin`, `scenario_origin_change_rebaselines_max_carry_on_next_recompute`. |
| Container nesting depth-limited | IMPLEMENTED | `InventoryGrid::try_nest_container` rejects with `MAX_DEPTH_EXCEEDED` ("max_depth_exceeded") when candidate depth > `MAX_CONTAINER_NEST_DEPTH=2`; `ContainerCapacity::allowed_categories` whitelist enforced (empty=accept-all); `act.player.nest_container` cfctl method wired to `M6Action::NestContainer`; engine routes rejection through `CommandResult::rejected` + `actor.action_rejected` with the spec-locked reason; on success emits `inventory.container_nested` with 1-indexed depth. Tests: `scenario_container_nesting_depth_limited_rejects_max_depth_exceeded`, `scenario_container_nesting_depth_helper_matches_spec`, `allowed_categories_whitelist_rejects_off_category`, `allowed_categories_empty_set_accepts_all`, `m6b_nest_container_engine_rejects_max_depth`. |
| Liquid container full vs empty mass | IMPLEMENTED | `water_bottle` spec carries `mass_kg=0.2 + liquid_capacity_l=Some(1.0)`; `liquid_fill_mass(spec, 1.0)=1.2`, `0.5=0.7`; `InventoryGrid::adjust_liquid` updates the placed item's `current_liquid_l` and the grid's `total_mass_kg` reflects the new full/half/empty mass; per-tick recompute flushes the new mass through the encumbrance envelope. Tests: `scenario_liquid_container_full_vs_empty_mass`, `liquid_container_tracks_full_vs_empty`. |
| Determinism — encumbrance reproduced | IMPLEMENTED | Encumbrance compute is pure (no RNG, no wall clock, no system time); `walk_speed_multiplier` + `encumbrance_band` are byte-stable across repeated calls; serde round-trip preserves `InventoryGrid` mass deterministically; end-to-end engine sim test verifies two encumbered actors with identical seed produce bit-identical positions per tick across 100 ticks of movement; cross-check verifies encumbered actor travels ~0.5× empty actor distance (per spec curve). Tests: `scenario_determinism_encumbrance_reproduced`, `scenario_determinism_engine_100_ticks_two_runs_identical_positions`, `scenario_encumbrance_reduces_actor_displacement_vs_unloaded`. |

## Per-bullet audit (Player-facing behavior)

| Bullet | Status | Evidence |
|---|---|---|
| Mass + dimensions + bulk per item | IMPLEMENTED | `cf_equipment::ItemSpec { mass_kg, dimensions, bulk_volume_l }` + 12 registered items |
| Per-origin `max_carry_kg` scaling | IMPLEMENTED | `carry_capacity_modifier`, `max_carry_kg_for_origin` |
| Walk-speed lerp curve | IMPLEMENTED | `walk_speed_multiplier` applied in `cf-actor::sim::step_one_actor` |
| Per-origin `max_carry_volume_l` scaling | IMPLEMENTED | `max_carry_volume_l_for_origin` |
| Backpack tier grids | IMPLEMENTED | `BackpackTier::{Small,Medium,Large,Industrial}::dimensions()` |
| Container nesting depth=2 cap | IMPLEMENTED | `MAX_CONTAINER_NEST_DEPTH=2` enforced via `try_nest_depth` |
| `stack_mass = item_mass × count` | IMPLEMENTED | `ItemSpec::stack_mass(count)` + `stack_bulk_l(count)` helpers |
| Loose items count individually in containers | IMPLEMENTED | `loose_items_in_container_count_individually` test verifies 3 non-stackable rifles inside a chest produce 3 placements each contributing 3.5 kg |
| Liquid full vs empty mass | IMPLEMENTED | `liquid_fill_mass` + `InventoryGrid::adjust_liquid` |
| `quick_slot_eligible` for M14A QAB | IMPLEMENTED | `ItemSpec.quick_slot_eligible` flag + `cf_equipment::quick_slot_eligible_ids()` filter API |

## Cross-cutting surfaces (audit-pass deltas)

- **observe.actor.inventory** — `ExtendedInventorySlotView` extended with `bulk_volume_l`; `ActorObservation.inventory_grid: Option<InventoryGridView>` surfaces per-placement `mass_kg`, `bulk_volume_l`, `category`, `quick_slot_eligible`, `nested_count`, `current_liquid_l`, `liquid_capacity_l`. Both lookups query the canonical `cf_equipment::ItemSpec` registry (no hardcoded fallbacks).
- **cf-mod validator** — validates `content/equipment/items/manifest.ron` against the registry AND any other `*.ron` file in the items dir as a standalone `cf_equipment::ItemSpec` (per spec § "Validate `content/equipment/items/*.ron` against ItemSpec schema").
- **Engine per-tick encumbrance recompute** — `tick_m6_actor_state` rebaselines the envelope from `origin_id` AND refreshes total carried mass/bulk every tick, so any state change (origin swap, liquid drain, future M6C SKU swap, M27B loot pickup) immediately reflects in the walk-speed multiplier + band without caller plumbing.
- **Engine band-transition event surface** — `tick_m6_actor_state` emits `inventory.encumbrance_threshold_crossed` on `m6b_last_encumbrance_band` change; `dispatch_m6_action` ALSO emits the same event immediately after pickup/drop for HUD-instant feedback.
