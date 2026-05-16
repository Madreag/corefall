# M6B — Item Schema Canonicalization + Per-Item Weight + Grid Dimensions + Inventory Encumbrance

## Status

`active`

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
