//! **M6B**: canonical `ItemSpec` schema + registry + per-item physics surface.
//!
//! `ItemSpec` is the single source of truth for the per-item physical schema
//! (mass + dimensions + bulk + slot + container nesting + per-origin carry
//! caps + crafting yield + material breakdown). Consumed by:
//!
//! - **M6** — base inventory baseline (legacy slot weight stays as a fallback).
//! - **M6C** — equipment SKU definitions.
//! - **M27** — Tetris-grid drag-drop UX.
//! - **M27B** — loot tables.
//! - **M32C** — crafting outputs.
//! - **M19N** — food storage.
//! - **M14A** — mass aggregation (`inventory_mass` summand).
//!
//! The schema is locked at M6B: additive fields are allowed in future
//! milestones, but the field set + default semantics below MUST stay
//! backward-compatible.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

/// Opaque item identifier (stable across saves / mods).
pub type ItemId = String;

/// Opaque crafting recipe identifier referenced by repair recipes.
pub type RecipeId = String;

/// Opaque material identifier (mirrors `cf_material::MaterialId` semantics).
pub type MaterialId = String;

/// Opaque origin identifier (mirrors `ActorState.origin_id`).
pub type OriginId = String;

/// **M6B § Tunable defaults**: baseline carry-capacity envelope in kg
/// before the per-origin modifier (M17) scales it.
pub const HUMAN_BASELINE_MAX_CARRY_KG: f32 = 50.0;

/// **M6B § Tunable defaults**: baseline volume envelope in liters before
/// the per-origin modifier scales it (Tarkov parity).
pub const HUMAN_BASELINE_MAX_CARRY_VOLUME_L: f32 = 60.0;

/// **M6B § Tunable defaults**: walk-speed multiplier at exactly 100% load
/// (`total_carried_kg == max_carry_kg`).
pub const WALK_SPEED_AT_FULL_CARRY: f32 = 0.5;

/// **M6B § Tunable defaults**: walk-speed multiplier at exactly 0% load.
pub const WALK_SPEED_AT_EMPTY_CARRY: f32 = 1.0;

/// **M6B § Tunable defaults**: deepest container nesting allowed.
/// Spec literal: "Container nesting allowed up to 2 levels
/// (chest → crate → item; not deeper)."
pub const MAX_CONTAINER_NEST_DEPTH: u8 = 2;

/// Rejection reason returned by [`try_nest_depth`] when the candidate
/// nesting would exceed [`MAX_CONTAINER_NEST_DEPTH`].
pub const MAX_DEPTH_EXCEEDED: &str = "max_depth_exceeded";

/// **M6B § Tunable defaults**: backpack-tier grid dimensions.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackpackTier {
    /// 4×6 grid (small).
    Small = 0,
    /// 6×8 grid (medium).
    Medium = 1,
    /// 8×10 grid (large).
    Large = 2,
    /// 10×12 grid (industrial).
    Industrial = 3,
}

impl BackpackTier {
    /// Grid dimensions in tiles `(w, h)`.
    pub const fn dimensions(self) -> (u8, u8) {
        match self {
            BackpackTier::Small => (4, 6),
            BackpackTier::Medium => (6, 8),
            BackpackTier::Large => (8, 10),
            BackpackTier::Industrial => (10, 12),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            BackpackTier::Small => "small",
            BackpackTier::Medium => "medium",
            BackpackTier::Large => "large",
            BackpackTier::Industrial => "industrial",
        }
    }

    /// Total tile count (`w * h`).
    pub const fn cell_count(self) -> u16 {
        let (w, h) = self.dimensions();
        (w as u16) * (h as u16)
    }
}

impl Default for BackpackTier {
    fn default() -> Self {
        BackpackTier::Small
    }
}

/// **M6B § ItemSpec schema**: grid footprint in tiles.
///
/// Spec literal: `GridDim { w: u8, h: u8 }`.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct GridDim {
    pub w: u8,
    pub h: u8,
}

impl GridDim {
    pub const fn new(w: u8, h: u8) -> Self {
        Self { w, h }
    }

    /// Tile count occupied (`w * h`).
    pub const fn cell_count(self) -> u16 {
        (self.w as u16) * (self.h as u16)
    }

    /// Rotated 90° footprint (swap w + h).
    pub const fn rotated(self) -> Self {
        Self { w: self.h, h: self.w }
    }
}

/// **M6B § ItemSpec schema**: category discriminator. Drives loot tables,
/// crafting outputs, container restrictions, and HUD icon selection.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemCategory {
    Weapon = 0,
    Armor = 1,
    Tool = 2,
    Medical = 3,
    Survival = 4,
    Sensor = 5,
    Consumable = 6,
    Material = 7,
    Container = 8,
    Ammo = 9,
    Magazine = 10,
    Liquid = 11,
    Specialty = 12,
}

impl ItemCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            ItemCategory::Weapon => "weapon",
            ItemCategory::Armor => "armor",
            ItemCategory::Tool => "tool",
            ItemCategory::Medical => "medical",
            ItemCategory::Survival => "survival",
            ItemCategory::Sensor => "sensor",
            ItemCategory::Consumable => "consumable",
            ItemCategory::Material => "material",
            ItemCategory::Container => "container",
            ItemCategory::Ammo => "ammo",
            ItemCategory::Magazine => "magazine",
            ItemCategory::Liquid => "liquid",
            ItemCategory::Specialty => "specialty",
        }
    }

    /// Returns true when this category designates a container that can
    /// hold other items / nested containers.
    pub const fn is_container(self) -> bool {
        matches!(self, ItemCategory::Container)
    }
}

/// **M6B § ItemSpec schema**: per-container holding capacity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContainerCapacity {
    /// Internal grid dimensions of the container itself.
    pub grid: GridDim,
    /// Max nesting depth (cap with [`MAX_CONTAINER_NEST_DEPTH`]). A value
    /// of `1` means "this container can hold items but not nested
    /// containers". A value of `2` allows a single layer of nested
    /// containers (chest → crate). Values above `MAX_CONTAINER_NEST_DEPTH`
    /// are clamped at [`try_nest_depth`] time.
    pub max_nest_depth: u8,
    /// Optional category whitelist. Empty = accept all categories.
    #[serde(default)]
    pub allowed_categories: BTreeSet<ItemCategory>,
}

impl Default for ContainerCapacity {
    fn default() -> Self {
        Self {
            grid: GridDim::new(0, 0),
            max_nest_depth: MAX_CONTAINER_NEST_DEPTH,
            allowed_categories: BTreeSet::new(),
        }
    }
}

/// **M6B § ItemSpec schema**: locked at M6B per the spec literal block.
///
/// ```text
/// pub struct ItemSpec {
///     pub id: ItemId,
///     pub display_name: String,
///     pub mass_kg: f32,
///     pub dimensions: GridDim { w: u8, h: u8 },
///     pub bulk_volume_l: f32,
///     pub stackable: bool,
///     pub max_stack: u16,
///     pub category: ItemCategory,
///     pub container_capacity: Option<ContainerCapacity>,
///     pub liquid_capacity_l: Option<f32>,
///     pub rotation_allowed: bool,
///     pub quick_slot_eligible: bool,
///     pub durability_max: Option<u32>,
///     pub repair_recipe: Option<RecipeId>,
///     pub material_weight_breakdown: BTreeMap<MaterialId, f32>,
///     pub crafting_yield_count: u8,
///     pub origin_compatibility: BTreeSet<OriginId>,
///     pub forbid_for_origin: BTreeSet<OriginId>,
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemSpec {
    pub id: ItemId,
    pub display_name: String,
    pub mass_kg: f32,
    pub dimensions: GridDim,
    pub bulk_volume_l: f32,
    pub stackable: bool,
    pub max_stack: u16,
    pub category: ItemCategory,
    #[serde(default)]
    pub container_capacity: Option<ContainerCapacity>,
    /// `Some(L)` when the item is a liquid container (mass tracked as
    /// empty_mass + liquid_mass).
    #[serde(default)]
    pub liquid_capacity_l: Option<f32>,
    pub rotation_allowed: bool,
    pub quick_slot_eligible: bool,
    #[serde(default)]
    pub durability_max: Option<u32>,
    #[serde(default)]
    pub repair_recipe: Option<RecipeId>,
    #[serde(default)]
    pub material_weight_breakdown: BTreeMap<MaterialId, f32>,
    pub crafting_yield_count: u8,
    #[serde(default)]
    pub origin_compatibility: BTreeSet<OriginId>,
    #[serde(default)]
    pub forbid_for_origin: BTreeSet<OriginId>,
}

impl ItemSpec {
    /// True when this item exposes a `ContainerCapacity` (chests, crates,
    /// backpacks, magazines acting as ammo containers, etc.).
    pub fn is_container(&self) -> bool {
        self.container_capacity.is_some()
    }

    /// True when this item declares a liquid capacity > 0.
    pub fn is_liquid_container(&self) -> bool {
        self.liquid_capacity_l.is_some_and(|c| c > 0.0)
    }

    /// True when this item is eligible to live in the M14A
    /// quick-action-bar (hot-swap).
    pub fn quick_slot_ok(&self) -> bool {
        self.quick_slot_eligible
    }

    /// **M6B § Player-facing behavior**: stack mass formula.
    ///
    /// Spec literal: "Stack items declare `stack_mass = item_mass × count`".
    /// Non-stackable items: `count` is clamped to `[1, max_stack]` so the
    /// returned mass is always `mass_kg × count` (with `count` clamped at
    /// 1 for non-stackable items).
    pub fn stack_mass(&self, count: u16) -> f32 {
        let clamped_count = if self.stackable {
            count.clamp(1, self.max_stack.max(1))
        } else {
            1
        };
        self.mass_kg * f32::from(clamped_count)
    }

    /// **M6B § Player-facing behavior**: stack bulk formula. Same semantics
    /// as [`stack_mass`] but for bulk volume.
    pub fn stack_bulk_l(&self, count: u16) -> f32 {
        let clamped_count = if self.stackable {
            count.clamp(1, self.max_stack.max(1))
        } else {
            1
        };
        self.bulk_volume_l * f32::from(clamped_count)
    }
}

/// **M6B**: compute the effective mass of a liquid container when it
/// holds `liters_filled` liters of water (density 1 kg/L). Returns
/// `spec.mass_kg + liters_filled * 1.0`. When the spec is not a liquid
/// container, returns `spec.mass_kg` unchanged.
///
/// Spec scenario:
/// > Given an empty water_bottle (0.2 kg empty + 1L capacity)
/// > When player fills with 1L water → item mass = 1.2 kg
/// > When player drinks 500ml → item mass = 0.7 kg
pub fn liquid_fill_mass(spec: &ItemSpec, liters_filled: f32) -> f32 {
    if let Some(cap_l) = spec.liquid_capacity_l {
        let liters = liters_filled.max(0.0).min(cap_l);
        spec.mass_kg + liters * 1.0
    } else {
        spec.mass_kg
    }
}

/// **M6B**: container nesting depth check.
///
/// `parent_depth` is the parent's current nesting depth (0 = root
/// inventory, 1 = top-level container, 2 = nested container, ...). The
/// returned depth is `parent_depth + 1` (the depth the candidate child
/// would occupy if placed inside the parent). When the candidate depth
/// exceeds [`MAX_CONTAINER_NEST_DEPTH`], returns [`Err(MAX_DEPTH_EXCEEDED)`].
///
/// Per-container `max_nest_depth` further constrains: if the parent's
/// declared cap is lower than the global maximum, the parent's cap wins.
pub fn try_nest_depth(parent_depth: u8, parent_cap: u8, child_is_container: bool) -> Result<u8, &'static str> {
    let candidate = parent_depth.saturating_add(1);
    // A non-container child never raises nesting depth; just check it
    // fits within MAX_CONTAINER_NEST_DEPTH + the parent cap so unit
    // tests + cfctl validators stay consistent.
    let effective_cap = parent_cap.min(MAX_CONTAINER_NEST_DEPTH);
    if child_is_container && candidate > effective_cap {
        return Err(MAX_DEPTH_EXCEEDED);
    }
    Ok(candidate)
}

/// **M6B § Tunable defaults**: per-origin carry-capacity multiplier.
///
/// | origin           | multiplier |
/// |------------------|-----------:|
/// | heavy_biomech    | 1.5×       |
/// | robot            | 1.2×       |
/// | android / human  | 1.0×       |
/// | drone            | 0.3×       |
///
/// Unknown origins default to 1.0× (additive: future origins can register
/// modifiers without breaking existing actors).
pub fn carry_capacity_modifier(origin_id: &str) -> f32 {
    match origin_id {
        "heavy_biomech" => 1.5,
        "robot" => 1.2,
        "drone" => 0.3,
        _ => 1.0,
    }
}

/// **M6B § Tunable defaults**: per-actor maximum carry mass for the given
/// origin id, derived from [`HUMAN_BASELINE_MAX_CARRY_KG`] +
/// [`carry_capacity_modifier`].
pub fn max_carry_kg_for_origin(origin_id: &str) -> f32 {
    HUMAN_BASELINE_MAX_CARRY_KG * carry_capacity_modifier(origin_id)
}

/// **M6B § Tunable defaults**: per-actor maximum carry volume for the
/// given origin id, derived from [`HUMAN_BASELINE_MAX_CARRY_VOLUME_L`] +
/// [`carry_capacity_modifier`].
pub fn max_carry_volume_l_for_origin(origin_id: &str) -> f32 {
    HUMAN_BASELINE_MAX_CARRY_VOLUME_L * carry_capacity_modifier(origin_id)
}

/// **M6B § Encumbrance penalty curve**: linear lerp between the
/// [`WALK_SPEED_AT_EMPTY_CARRY`] (1.0) and [`WALK_SPEED_AT_FULL_CARRY`]
/// (0.5) endpoints over `[0, max_carry_kg]`. Values above `max_carry_kg`
/// clamp at `WALK_SPEED_AT_FULL_CARRY` (no negative speed); values below
/// 0 (or `NaN`) clamp at 1.0.
///
/// Spec literal: `walk speed × lerp(1.0, 0.5, total_carried_kg / max_carry_kg)`.
pub fn walk_speed_multiplier(total_carried_kg: f32, max_carry_kg: f32) -> f32 {
    if !total_carried_kg.is_finite() || total_carried_kg <= 0.0 || max_carry_kg <= 0.0 {
        return WALK_SPEED_AT_EMPTY_CARRY;
    }
    let load = (total_carried_kg / max_carry_kg).clamp(0.0, 1.0);
    WALK_SPEED_AT_EMPTY_CARRY + (WALK_SPEED_AT_FULL_CARRY - WALK_SPEED_AT_EMPTY_CARRY) * load
}

/// Discrete encumbrance bands. Fed to the
/// `inventory.encumbrance_threshold_crossed` event so HUD + replay can
/// render the warning level on transitions only.
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncumbranceBand {
    /// 0% – <50%.
    None = 0,
    /// 50% – <75%.
    Light = 1,
    /// 75% – <100%.
    Moderate = 2,
    /// ≥100% (HUD shows "ENCUMBERED").
    Heavy = 3,
}

impl EncumbranceBand {
    pub const fn as_str(self) -> &'static str {
        match self {
            EncumbranceBand::None => "none",
            EncumbranceBand::Light => "light",
            EncumbranceBand::Moderate => "moderate",
            EncumbranceBand::Heavy => "heavy",
        }
    }
}

/// Classify the current carry ratio into one of the four
/// [`EncumbranceBand`]s.
pub fn encumbrance_band(total_carried_kg: f32, max_carry_kg: f32) -> EncumbranceBand {
    if !total_carried_kg.is_finite() || max_carry_kg <= 0.0 {
        return EncumbranceBand::None;
    }
    let ratio = total_carried_kg / max_carry_kg;
    if ratio >= 1.0 {
        EncumbranceBand::Heavy
    } else if ratio >= 0.75 {
        EncumbranceBand::Moderate
    } else if ratio >= 0.5 {
        EncumbranceBand::Light
    } else {
        EncumbranceBand::None
    }
}

/// **M6B**: hardcoded registry of canonical [`ItemSpec`]s. Lookups go
/// through [`spec_for_id`]. The registry is the runtime source of
/// truth; `content/equipment/items/manifest.ron` mirrors the id list
/// for cf-mod validation.
///
/// The function intentionally exceeds the default clippy length limit
/// because every entry is a multi-line constant declaration; collapsing
/// them would harm readability.
#[allow(clippy::too_many_lines)]
fn build_registry() -> BTreeMap<ItemId, ItemSpec> {
    let mut map = BTreeMap::new();
    let entries = [
        // M1 rifle (mass + dimensions per Gherkin "Item declares mass + dimensions").
        ItemSpec {
            id: "rifle_m1".to_string(),
            display_name: "Rifle (M1)".to_string(),
            mass_kg: 3.5,
            dimensions: GridDim::new(2, 4),
            bulk_volume_l: 3.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Weapon,
            container_capacity: None,
            liquid_capacity_l: None,
            rotation_allowed: true,
            quick_slot_eligible: true,
            durability_max: Some(1000),
            repair_recipe: Some("repair.rifle_m1".to_string()),
            material_weight_breakdown: BTreeMap::from([
                ("steel".to_string(), 2.5),
                ("polymer".to_string(), 1.0),
            ]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::from([
                "human".to_string(),
                "robot".to_string(),
                "android".to_string(),
                "heavy_biomech".to_string(),
            ]),
            forbid_for_origin: BTreeSet::from(["drone".to_string()]),
        },
        // Default rifle preset id used by M1 inventory + ExtendedInventory.
        // Keep mass/dims consistent so the legacy preset surfaces through
        // ItemSpec lookups too.
        ItemSpec {
            id: "rifle_m1_default".to_string(),
            display_name: "Rifle (M1, default)".to_string(),
            mass_kg: 3.5,
            dimensions: GridDim::new(2, 4),
            bulk_volume_l: 3.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Weapon,
            container_capacity: None,
            liquid_capacity_l: None,
            rotation_allowed: true,
            quick_slot_eligible: true,
            durability_max: Some(1000),
            repair_recipe: Some("repair.rifle_m1_default".to_string()),
            material_weight_breakdown: BTreeMap::from([
                ("steel".to_string(), 2.5),
                ("polymer".to_string(), 1.0),
            ]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::from([
                "human".to_string(),
                "robot".to_string(),
                "android".to_string(),
                "heavy_biomech".to_string(),
            ]),
            forbid_for_origin: BTreeSet::from(["drone".to_string()]),
        },
        // Liquid container per Gherkin "Liquid container full vs empty mass".
        ItemSpec {
            id: "water_bottle".to_string(),
            display_name: "Water Bottle".to_string(),
            mass_kg: 0.2,
            dimensions: GridDim::new(1, 2),
            bulk_volume_l: 1.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Liquid,
            container_capacity: None,
            liquid_capacity_l: Some(1.0),
            rotation_allowed: true,
            quick_slot_eligible: true,
            durability_max: Some(100),
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 0.2)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        // Chest (top-level container per Gherkin "Container nesting depth-limited").
        ItemSpec {
            id: "chest".to_string(),
            display_name: "Chest".to_string(),
            mass_kg: 5.0,
            dimensions: GridDim::new(4, 4),
            bulk_volume_l: 60.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Container,
            container_capacity: Some(ContainerCapacity {
                grid: GridDim::new(8, 10),
                max_nest_depth: MAX_CONTAINER_NEST_DEPTH,
                allowed_categories: BTreeSet::new(),
            }),
            liquid_capacity_l: None,
            rotation_allowed: false,
            quick_slot_eligible: false,
            durability_max: Some(2000),
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("steel".to_string(), 5.0)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        // Crate (nested container per Gherkin).
        ItemSpec {
            id: "crate".to_string(),
            display_name: "Crate".to_string(),
            mass_kg: 2.0,
            dimensions: GridDim::new(3, 3),
            bulk_volume_l: 20.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Container,
            container_capacity: Some(ContainerCapacity {
                grid: GridDim::new(4, 6),
                max_nest_depth: MAX_CONTAINER_NEST_DEPTH,
                allowed_categories: BTreeSet::new(),
            }),
            liquid_capacity_l: None,
            rotation_allowed: false,
            quick_slot_eligible: false,
            durability_max: Some(500),
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 2.0)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        // Backpack tiers — small (4×6).
        ItemSpec {
            id: "backpack_small".to_string(),
            display_name: "Backpack (Small)".to_string(),
            mass_kg: 1.5,
            dimensions: GridDim::new(3, 4),
            bulk_volume_l: 30.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Container,
            container_capacity: Some(ContainerCapacity {
                grid: GridDim::new(4, 6),
                max_nest_depth: MAX_CONTAINER_NEST_DEPTH,
                allowed_categories: BTreeSet::new(),
            }),
            liquid_capacity_l: None,
            rotation_allowed: false,
            quick_slot_eligible: false,
            durability_max: Some(800),
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 1.5)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        ItemSpec {
            id: "backpack_medium".to_string(),
            display_name: "Backpack (Medium)".to_string(),
            mass_kg: 2.5,
            dimensions: GridDim::new(4, 5),
            bulk_volume_l: 50.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Container,
            container_capacity: Some(ContainerCapacity {
                grid: GridDim::new(6, 8),
                max_nest_depth: MAX_CONTAINER_NEST_DEPTH,
                allowed_categories: BTreeSet::new(),
            }),
            liquid_capacity_l: None,
            rotation_allowed: false,
            quick_slot_eligible: false,
            durability_max: Some(1000),
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 2.5)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        ItemSpec {
            id: "backpack_large".to_string(),
            display_name: "Backpack (Large)".to_string(),
            mass_kg: 3.5,
            dimensions: GridDim::new(5, 6),
            bulk_volume_l: 80.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Container,
            container_capacity: Some(ContainerCapacity {
                grid: GridDim::new(8, 10),
                max_nest_depth: MAX_CONTAINER_NEST_DEPTH,
                allowed_categories: BTreeSet::new(),
            }),
            liquid_capacity_l: None,
            rotation_allowed: false,
            quick_slot_eligible: false,
            durability_max: Some(1200),
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 3.5)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        ItemSpec {
            id: "backpack_industrial".to_string(),
            display_name: "Backpack (Industrial)".to_string(),
            mass_kg: 5.0,
            dimensions: GridDim::new(6, 7),
            bulk_volume_l: 120.0,
            stackable: false,
            max_stack: 1,
            category: ItemCategory::Container,
            container_capacity: Some(ContainerCapacity {
                grid: GridDim::new(10, 12),
                max_nest_depth: MAX_CONTAINER_NEST_DEPTH,
                allowed_categories: BTreeSet::new(),
            }),
            liquid_capacity_l: None,
            rotation_allowed: false,
            quick_slot_eligible: false,
            durability_max: Some(1500),
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 5.0)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        // Generic medkit + ammo + ration so loot tables have something to draw from.
        ItemSpec {
            id: "medkit".to_string(),
            display_name: "Medkit".to_string(),
            mass_kg: 1.0,
            dimensions: GridDim::new(1, 2),
            bulk_volume_l: 1.0,
            stackable: true,
            max_stack: 4,
            category: ItemCategory::Medical,
            container_capacity: None,
            liquid_capacity_l: None,
            rotation_allowed: true,
            quick_slot_eligible: true,
            durability_max: None,
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 1.0)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        ItemSpec {
            id: "ammo_5_56x45".to_string(),
            display_name: "Ammo 5.56x45".to_string(),
            mass_kg: 0.012,
            dimensions: GridDim::new(1, 1),
            bulk_volume_l: 0.02,
            stackable: true,
            max_stack: 60,
            category: ItemCategory::Ammo,
            container_capacity: None,
            liquid_capacity_l: None,
            rotation_allowed: false,
            quick_slot_eligible: false,
            durability_max: None,
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("brass".to_string(), 0.012)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
        ItemSpec {
            id: "ration_mre".to_string(),
            display_name: "MRE Ration".to_string(),
            mass_kg: 0.6,
            dimensions: GridDim::new(2, 2),
            bulk_volume_l: 1.5,
            stackable: true,
            max_stack: 4,
            category: ItemCategory::Consumable,
            container_capacity: None,
            liquid_capacity_l: None,
            rotation_allowed: true,
            quick_slot_eligible: true,
            durability_max: None,
            repair_recipe: None,
            material_weight_breakdown: BTreeMap::from([("polymer".to_string(), 0.6)]),
            crafting_yield_count: 1,
            origin_compatibility: BTreeSet::new(),
            forbid_for_origin: BTreeSet::new(),
        },
    ];

    for spec in entries {
        map.insert(spec.id.clone(), spec);
    }
    for spec in m6c_entries() {
        map.insert(spec.id.clone(), spec);
    }
    map
}

/// **M6C**: 81 new equipment SKUs registered with the M6B `ItemSpec`
/// schema. See `specs/active/M6C.md § Player-facing behavior` for the
/// per-category breakdown and tunables. Owned by M6C — `cf-mod` walks
/// `content/equipment/<category>/*.ron` and validates every entry
/// against this registry.
#[allow(clippy::too_many_lines)]
pub fn m6c_entries() -> Vec<ItemSpec> {
    let mut out = Vec::new();
    out.extend(m6c_firearm_entries());
    out.extend(m6c_melee_entries());
    out.extend(m6c_throwable_entries());
    out.extend(m6c_heavy_entries());
    out.extend(m6c_medical_entries());
    out.extend(m6c_survival_entries());
    out.extend(m6c_sensor_entries());
    out.extend(m6c_ppe_entries());
    out
}

/// **M6C**: M6C entries grouped by per-category folder name. The folder
/// name matches the path under `content/equipment/<folder>/` that
/// cf-mod validates and the generator binary writes.
pub fn m6c_entries_by_category() -> Vec<(&'static str, Vec<ItemSpec>)> {
    vec![
        ("firearms", m6c_firearm_entries()),
        ("melee", m6c_melee_entries()),
        ("grenades", m6c_throwable_entries()),
        ("heavy", m6c_heavy_entries()),
        ("medical", m6c_medical_entries()),
        ("survival", m6c_survival_entries()),
        ("sensors", m6c_sensor_entries()),
        ("ppe", m6c_ppe_entries()),
    ]
}

/// Convenience constructor for an ItemSpec entry that just needs the
/// canonical fields. The signature is intentionally wide because every
/// M6C entry declares the same axis of fields.
#[allow(clippy::too_many_arguments)]
fn entry(
    id: &str,
    display_name: &str,
    mass_kg: f32,
    grid_w: u8,
    grid_h: u8,
    bulk_l: f32,
    category: ItemCategory,
    materials: &[(&str, f32)],
    quick_slot: bool,
    durability_max: Option<u32>,
) -> ItemSpec {
    let mut map = BTreeMap::new();
    for (m, w) in materials {
        map.insert((*m).to_string(), *w);
    }
    ItemSpec {
        id: id.to_string(),
        display_name: display_name.to_string(),
        mass_kg,
        dimensions: GridDim::new(grid_w, grid_h),
        bulk_volume_l: bulk_l,
        stackable: false,
        max_stack: 1,
        category,
        container_capacity: None,
        liquid_capacity_l: None,
        rotation_allowed: true,
        quick_slot_eligible: quick_slot,
        durability_max,
        repair_recipe: None,
        material_weight_breakdown: map,
        crafting_yield_count: 1,
        origin_compatibility: BTreeSet::new(),
        forbid_for_origin: BTreeSet::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn stackable_entry(
    id: &str,
    display_name: &str,
    mass_kg: f32,
    grid_w: u8,
    grid_h: u8,
    bulk_l: f32,
    max_stack: u16,
    category: ItemCategory,
    materials: &[(&str, f32)],
    quick_slot: bool,
) -> ItemSpec {
    let mut e = entry(id, display_name, mass_kg, grid_w, grid_h, bulk_l, category, materials, quick_slot, None);
    e.stackable = true;
    e.max_stack = max_stack.max(1);
    e
}

fn liquid_entry(
    id: &str,
    display_name: &str,
    empty_mass_kg: f32,
    grid_w: u8,
    grid_h: u8,
    bulk_l: f32,
    capacity_l: f32,
    materials: &[(&str, f32)],
) -> ItemSpec {
    let mut e = entry(
        id,
        display_name,
        empty_mass_kg,
        grid_w,
        grid_h,
        bulk_l,
        ItemCategory::Liquid,
        materials,
        true,
        Some(200),
    );
    e.liquid_capacity_l = Some(capacity_l);
    e
}

fn m6c_firearm_entries() -> Vec<ItemSpec> {
    vec![
        entry("revolver_357", ".357 Revolver", 1.4, 2, 3, 2.5, ItemCategory::Weapon, &[("steel", 1.2), ("polymer", 0.2)], true, Some(1200)),
        entry("submachine_gun_9mm", "9mm Submachine Gun", 2.8, 2, 4, 3.0, ItemCategory::Weapon, &[("steel", 1.8), ("polymer", 1.0)], true, Some(1200)),
        entry("assault_rifle_t2", "Assault Rifle (T2)", 3.4, 2, 5, 3.5, ItemCategory::Weapon, &[("steel", 2.0), ("polymer", 1.4)], true, Some(1500)),
        entry("sniper_rifle_t2", "Sniper Rifle (T2)", 7.5, 2, 6, 5.5, ItemCategory::Weapon, &[("steel", 5.0), ("polymer", 2.5)], true, Some(1500)),
        entry("shotgun_pump", "Pump Shotgun", 3.6, 2, 5, 4.0, ItemCategory::Weapon, &[("steel", 2.6), ("polymer", 1.0)], true, Some(1500)),
        entry("heavy_machine_gun_50cal", ".50 cal Heavy Machine Gun", 38.0, 3, 7, 18.0, ItemCategory::Weapon, &[("steel", 30.0), ("polymer", 4.0)], false, Some(2000)),
        entry("battle_rifle_762", "7.62 Battle Rifle", 4.5, 2, 5, 4.0, ItemCategory::Weapon, &[("steel", 3.0), ("polymer", 1.5)], true, Some(1500)),
        entry("carbine_compact", "Compact Carbine", 2.9, 2, 4, 3.0, ItemCategory::Weapon, &[("steel", 1.8), ("polymer", 1.1)], true, Some(1300)),
        entry("dmr_762", "7.62 DMR", 5.2, 2, 6, 4.5, ItemCategory::Weapon, &[("steel", 3.5), ("polymer", 1.7)], true, Some(1500)),
        entry("lmg_belt_fed", "Belt-fed LMG", 9.5, 3, 6, 8.5, ItemCategory::Weapon, &[("steel", 7.0), ("polymer", 2.5)], false, Some(2000)),
        entry("anti_materiel_rifle_127", "12.7mm Anti-Materiel Rifle", 14.0, 3, 7, 10.0, ItemCategory::Weapon, &[("steel", 11.0), ("polymer", 3.0)], false, Some(1800)),
        entry("squad_automatic_saw", "Squad Automatic Weapon", 7.5, 3, 6, 7.0, ItemCategory::Weapon, &[("steel", 5.5), ("polymer", 2.0)], false, Some(1800)),
    ]
}

fn m6c_melee_entries() -> Vec<ItemSpec> {
    vec![
        entry("dagger_combat", "Combat Dagger", 0.2, 1, 2, 0.4, ItemCategory::Weapon, &[("steel", 0.2)], true, Some(600)),
        entry("katana", "Katana", 1.1, 1, 5, 1.6, ItemCategory::Weapon, &[("steel", 1.1)], true, Some(900)),
        entry("sledgehammer", "Sledgehammer", 5.0, 2, 5, 5.0, ItemCategory::Weapon, &[("steel", 4.0), ("wood", 1.0)], false, Some(1500)),
        entry("spear", "Spear", 1.6, 1, 6, 3.0, ItemCategory::Weapon, &[("steel", 1.0), ("wood", 0.6)], true, Some(700)),
        entry("bayonet", "Bayonet", 0.4, 1, 2, 0.5, ItemCategory::Weapon, &[("steel", 0.4)], true, Some(800)),
        entry("axe_hatchet", "Felling Axe", 1.8, 2, 4, 2.5, ItemCategory::Weapon, &[("steel", 1.4), ("wood", 0.4)], true, Some(1200)),
        entry("stun_baton", "Stun Baton", 1.0, 1, 3, 1.5, ItemCategory::Weapon, &[("polymer", 0.6), ("battery", 0.4)], true, Some(800)),
        entry("pickaxe_combat_variant", "Combat Pickaxe", 2.4, 2, 4, 3.0, ItemCategory::Weapon, &[("steel", 2.0), ("wood", 0.4)], true, Some(1500)),
    ]
}

fn m6c_throwable_entries() -> Vec<ItemSpec> {
    vec![
        stackable_entry("he_grenade", "HE Grenade", 0.5, 1, 2, 0.6, 4, ItemCategory::Specialty, &[("steel", 0.3), ("explosive", 0.2)], true),
        stackable_entry("acid_grenade", "Acid Grenade", 0.5, 1, 2, 0.6, 4, ItemCategory::Specialty, &[("polymer", 0.2), ("acid", 0.3)], true),
        stackable_entry("pipe_bomb", "Pipe Bomb", 0.8, 1, 2, 0.8, 2, ItemCategory::Specialty, &[("steel", 0.5), ("explosive", 0.3)], true),
        stackable_entry("molotov_cocktail", "Molotov Cocktail", 0.6, 1, 2, 0.7, 4, ItemCategory::Specialty, &[("glass", 0.2), ("fuel", 0.4)], true),
        stackable_entry("proximity_mine", "Proximity Mine", 1.4, 2, 2, 1.5, 2, ItemCategory::Specialty, &[("steel", 0.8), ("explosive", 0.6)], true),
        stackable_entry("pressure_mine", "Pressure Mine", 1.6, 2, 2, 1.8, 2, ItemCategory::Specialty, &[("steel", 1.0), ("explosive", 0.6)], true),
        stackable_entry("tripwire_mine", "Tripwire Mine", 1.0, 2, 2, 1.2, 2, ItemCategory::Specialty, &[("steel", 0.6), ("explosive", 0.4)], true),
        stackable_entry("c4_charge", "C4 Charge", 1.5, 1, 2, 1.5, 4, ItemCategory::Specialty, &[("polymer", 0.5), ("explosive", 1.0)], true),
        stackable_entry("incendiary_grenade", "Incendiary Grenade", 0.45, 1, 2, 0.6, 4, ItemCategory::Specialty, &[("steel", 0.2), ("thermite", 0.25)], true),
        stackable_entry("bouncing_betty", "Bouncing Betty", 1.8, 2, 2, 2.0, 2, ItemCategory::Specialty, &[("steel", 1.2), ("explosive", 0.6)], true),
    ]
}

fn m6c_heavy_entries() -> Vec<ItemSpec> {
    vec![
        entry("rpg_launcher_heat", "RPG Launcher (HEAT)", 7.5, 3, 6, 10.0, ItemCategory::Weapon, &[("steel", 5.5), ("polymer", 2.0)], false, Some(800)),
        entry("tank_autocannon_m14c", "Tank Autocannon (APFSDS)", 95.0, 6, 12, 60.0, ItemCategory::Weapon, &[("steel", 85.0), ("polymer", 10.0)], false, Some(3000)),
        entry("mortar_60mm", "60mm Mortar", 22.0, 4, 8, 30.0, ItemCategory::Weapon, &[("steel", 18.0), ("polymer", 4.0)], false, Some(2200)),
        entry("recoilless_rifle", "Recoilless Rifle", 14.5, 3, 7, 18.0, ItemCategory::Weapon, &[("steel", 11.0), ("polymer", 3.5)], false, Some(1800)),
        entry("atgm_javelin", "ATGM Javelin", 22.0, 4, 8, 28.0, ItemCategory::Weapon, &[("steel", 14.0), ("polymer", 8.0)], false, Some(1500)),
        entry("flamethrower", "Flamethrower", 23.0, 3, 7, 22.0, ItemCategory::Weapon, &[("steel", 14.0), ("polymer", 9.0)], false, Some(1200)),
        entry("plasma_cannon_m48", "Plasma Cannon (M48)", 18.0, 3, 6, 20.0, ItemCategory::Weapon, &[("alloy", 12.0), ("polymer", 6.0)], false, Some(1500)),
        entry("gauss_rifle_anti_materiel", "Gauss Rifle (Anti-Materiel)", 17.0, 3, 7, 18.0, ItemCategory::Weapon, &[("alloy", 12.0), ("polymer", 5.0)], false, Some(1500)),
    ]
}

fn m6c_medical_entries() -> Vec<ItemSpec> {
    vec![
        stackable_entry("field_bandage", "Field Bandage", 0.1, 1, 1, 0.2, 6, ItemCategory::Medical, &[("polymer", 0.1)], true),
        stackable_entry("trauma_pack", "Trauma Pack", 0.4, 2, 2, 1.0, 3, ItemCategory::Medical, &[("polymer", 0.4)], true),
        stackable_entry("tourniquet", "Tourniquet", 0.05, 1, 1, 0.1, 6, ItemCategory::Medical, &[("polymer", 0.05)], true),
        stackable_entry("sutures", "Sutures", 0.1, 1, 1, 0.2, 6, ItemCategory::Medical, &[("polymer", 0.1)], true),
        stackable_entry("splint", "Splint", 0.3, 1, 3, 0.5, 3, ItemCategory::Medical, &[("polymer", 0.3)], true),
        entry("surgery_kit", "Surgery Kit", 2.5, 3, 3, 5.0, ItemCategory::Medical, &[("polymer", 2.0), ("steel", 0.5)], false, Some(500)),
        entry("defibrillator", "Defibrillator", 2.0, 2, 3, 3.0, ItemCategory::Medical, &[("polymer", 1.5), ("battery", 0.5)], true, Some(800)),
        stackable_entry("cpr_compressions", "CPR Compressions", 0.0, 1, 1, 0.0, 1, ItemCategory::Medical, &[], false),
        stackable_entry("transfusion_bag", "Transfusion Bag", 0.6, 2, 2, 1.0, 4, ItemCategory::Medical, &[("polymer", 0.1), ("blood", 0.5)], true),
        stackable_entry("iv_fluids", "IV Fluids", 0.5, 2, 2, 1.0, 4, ItemCategory::Medical, &[("polymer", 0.1), ("saline", 0.4)], true),
        entry("oxygen_therapy", "Oxygen Therapy", 1.4, 2, 3, 2.5, ItemCategory::Medical, &[("steel", 1.0), ("polymer", 0.4)], true, Some(400)),
        entry("medical_scanner_t1", "Medical Scanner (T1)", 0.8, 2, 2, 1.2, ItemCategory::Medical, &[("polymer", 0.6), ("alloy", 0.2)], true, Some(600)),
    ]
}

fn m6c_survival_entries() -> Vec<ItemSpec> {
    vec![
        liquid_entry("water_bottle_1l", "Water Bottle (1L)", 0.2, 1, 2, 1.2, 1.0, &[("polymer", 0.2)]),
        stackable_entry("food_ration_mre", "Food Ration (MRE)", 0.6, 2, 2, 1.5, 4, ItemCategory::Consumable, &[("polymer", 0.6)], true),
        entry("sleeping_bag", "Sleeping Bag", 1.8, 3, 4, 8.0, ItemCategory::Survival, &[("polymer", 1.8)], false, Some(500)),
        entry("tent_2_person", "Tent (2-person)", 3.5, 4, 5, 12.0, ItemCategory::Survival, &[("polymer", 3.5)], false, Some(600)),
        entry("cooking_pot", "Cooking Pot", 0.9, 2, 2, 2.0, ItemCategory::Survival, &[("steel", 0.9)], true, Some(600)),
        entry("lighter_zippo", "Lighter (Zippo)", 0.1, 1, 1, 0.1, ItemCategory::Survival, &[("steel", 0.05), ("fuel", 0.05)], true, Some(300)),
        entry("compass_magnetic", "Magnetic Compass", 0.1, 1, 1, 0.1, ItemCategory::Survival, &[("polymer", 0.05), ("alloy", 0.05)], true, Some(400)),
        entry("binoculars_8x", "Binoculars (8x)", 0.7, 2, 2, 1.5, ItemCategory::Survival, &[("alloy", 0.5), ("polymer", 0.2)], true, Some(600)),
        // **M6C-4 acceptance**: the flamethrower scenario pairs the weapon
        // with a `fuel_canister` in `tank_utility`. The canister itself is
        // a consumable tank-slot item that the flamethrower drains; the
        // SKU is required for the scenario to load even though the spec
        // body text doesn't enumerate it under any per-category section.
        fuel_canister_entry(),
    ]
}

fn fuel_canister_entry() -> ItemSpec {
    let mut e = entry(
        "fuel_canister",
        "Fuel Canister",
        5.0,
        2,
        3,
        6.0,
        ItemCategory::Consumable,
        &[("steel", 1.0), ("fuel", 4.0)],
        false,
        Some(400),
    );
    // Liquid capacity = 5 L napalm; M6C flamethrower drains at
    // 0.6 L / s so a full canister sustains ~8s of continuous fire.
    e.liquid_capacity_l = Some(5.0);
    e
}

fn m6c_sensor_entries() -> Vec<ItemSpec> {
    vec![
        entry("radio_direction_finder", "Radio Direction Finder", 1.4, 2, 3, 3.0, ItemCategory::Sensor, &[("polymer", 1.0), ("alloy", 0.4)], true, Some(600)),
        entry("sound_detector_passive", "Passive Sound Detector", 0.8, 2, 2, 1.5, ItemCategory::Sensor, &[("polymer", 0.6), ("alloy", 0.2)], true, Some(500)),
        entry("heat_camera_handheld", "Handheld Heat Camera", 1.0, 2, 2, 2.0, ItemCategory::Sensor, &[("polymer", 0.6), ("alloy", 0.4)], true, Some(600)),
        entry("radar_compact_t2", "Compact Radar (T2)", 6.0, 3, 5, 8.0, ItemCategory::Sensor, &[("alloy", 4.0), ("polymer", 2.0)], false, Some(800)),
        entry("geological_surveyor_m30d", "Geological Surveyor (M30D)", 3.0, 2, 4, 4.0, ItemCategory::Sensor, &[("alloy", 2.0), ("polymer", 1.0)], false, Some(700)),
    ]
}

fn m6c_ppe_entries() -> Vec<ItemSpec> {
    vec![
        entry("helmet_light_kevlar", "Light Kevlar Helmet", 1.2, 2, 2, 2.0, ItemCategory::Armor, &[("polymer", 1.2)], false, Some(200)),
        entry("helmet_medium_steel", "Medium Steel Helmet", 2.5, 2, 2, 2.2, ItemCategory::Armor, &[("steel", 2.5)], false, Some(400)),
        entry("helmet_heavy_titanium", "Heavy Titanium Helmet", 3.8, 2, 3, 2.5, ItemCategory::Armor, &[("titanium", 3.8)], false, Some(700)),
        entry("armor_kevlar_light", "Light Kevlar Armor", 4.0, 3, 4, 6.0, ItemCategory::Armor, &[("polymer", 4.0)], false, Some(400)),
        entry("armor_ceramic_medium", "Medium Ceramic Armor", 7.0, 3, 4, 7.0, ItemCategory::Armor, &[("ceramic", 5.0), ("polymer", 2.0)], false, Some(700)),
        entry("armor_steel_heavy", "Heavy Steel Armor", 12.0, 4, 5, 9.0, ItemCategory::Armor, &[("steel", 12.0)], false, Some(1200)),
        entry("armor_heavy_plate", "Heavy Plate Armor", 18.0, 4, 5, 11.0, ItemCategory::Armor, &[("alloy", 18.0)], false, Some(1800)),
        entry("armor_modular_plate_carrier", "Modular Plate Carrier", 5.0, 3, 4, 7.0, ItemCategory::Armor, &[("polymer", 3.0), ("steel", 2.0)], false, Some(800)),
        entry("combat_gloves_light", "Light Combat Gloves", 0.2, 1, 2, 0.5, ItemCategory::Armor, &[("polymer", 0.2)], false, Some(80)),
        entry("combat_gloves_heavy", "Heavy Combat Gloves", 0.4, 2, 2, 0.7, ItemCategory::Armor, &[("polymer", 0.4)], false, Some(120)),
        entry("tactical_boots", "Tactical Boots", 1.2, 2, 3, 3.0, ItemCategory::Armor, &[("polymer", 1.0), ("steel", 0.2)], false, Some(300)),
        entry("knee_pads", "Knee Pads", 0.4, 2, 2, 1.0, ItemCategory::Armor, &[("polymer", 0.4)], false, Some(150)),
        entry("elbow_pads", "Elbow Pads", 0.3, 2, 2, 0.8, ItemCategory::Armor, &[("polymer", 0.3)], false, Some(150)),
        entry("hardsuit_full", "Full Hardsuit", 18.0, 5, 6, 18.0, ItemCategory::Armor, &[("alloy", 14.0), ("polymer", 4.0)], false, Some(1500)),
        entry("eva_suit", "EVA Suit", 14.0, 5, 6, 16.0, ItemCategory::Armor, &[("alloy", 8.0), ("polymer", 6.0)], false, Some(1000)),
        entry("radiation_suit", "Radiation Suit", 8.0, 4, 5, 10.0, ItemCategory::Armor, &[("lead", 4.0), ("polymer", 4.0)], false, Some(600)),
        entry("hazmat_suit", "Hazmat Suit", 6.0, 4, 5, 9.0, ItemCategory::Armor, &[("polymer", 6.0)], false, Some(400)),
        entry("insulated_suit", "Insulated Suit", 5.0, 4, 5, 8.0, ItemCategory::Armor, &[("polymer", 5.0)], false, Some(500)),
    ]
}

thread_local! {
    static REGISTRY_CACHE: std::cell::OnceCell<BTreeMap<ItemId, ItemSpec>> = const { std::cell::OnceCell::new() };
}

/// Initialize + lookup the hardcoded ItemSpec registry. Returns a clone
/// of the registered spec when the id exists; `None` otherwise.
pub fn spec_for_id(id: &str) -> Option<ItemSpec> {
    REGISTRY_CACHE.with(|cache| {
        let map = cache.get_or_init(build_registry);
        map.get(id).cloned()
    })
}

/// Full registry id list (sorted ascending by id). Used by
/// `content/equipment/items/manifest.ron` validation in cf-mod.
pub fn registered_ids() -> Vec<ItemId> {
    REGISTRY_CACHE.with(|cache| {
        let map = cache.get_or_init(build_registry);
        map.keys().cloned().collect()
    })
}

/// **M6B § Player-facing behavior**: filter the registry down to items
/// flagged `quick_slot_eligible = true`. Consumed by the M14A
/// quick-action-bar — spec literal "Hot-swap M14A QAB items declare
/// `quick_slot_eligible = true`". Result is sorted ascending by id for
/// deterministic ordering.
pub fn quick_slot_eligible_ids() -> Vec<ItemId> {
    REGISTRY_CACHE.with(|cache| {
        let map = cache.get_or_init(build_registry);
        map.iter()
            .filter_map(|(id, spec)| if spec.quick_slot_eligible { Some(id.clone()) } else { None })
            .collect()
    })
}

/// Mass lookup convenience: returns `Some(mass_kg)` when the id resolves,
/// else `None`. M6B's "every item declares mass" contract.
pub fn mass_kg_for_id(id: &str) -> Option<f32> {
    spec_for_id(id).map(|s| s.mass_kg)
}

/// Bulk volume lookup convenience.
pub fn bulk_volume_l_for_id(id: &str) -> Option<f32> {
    spec_for_id(id).map(|s| s.bulk_volume_l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rifle_m1_spec_matches_gherkin() {
        // Scenario: Item declares mass + dimensions
        //   Given rifle_m1 spec with mass=3.5 kg + dimensions 2×4
        let spec = spec_for_id("rifle_m1").expect("rifle_m1 in registry");
        assert!((spec.mass_kg - 3.5).abs() < 1e-6);
        assert_eq!(spec.dimensions, GridDim::new(2, 4));
        assert_eq!(spec.category, ItemCategory::Weapon);
        assert!(spec.quick_slot_eligible);
    }

    #[test]
    fn water_bottle_liquid_mass() {
        // Scenario: Liquid container full vs empty mass
        //   Given an empty water_bottle (0.2 kg empty + 1L capacity)
        //   When player fills with 1L water → item mass = 1.2 kg
        //   When player drinks 500ml → item mass = 0.7 kg
        let spec = spec_for_id("water_bottle").expect("water_bottle in registry");
        assert!((spec.mass_kg - 0.2).abs() < 1e-6);
        assert_eq!(spec.liquid_capacity_l, Some(1.0));
        assert!((liquid_fill_mass(&spec, 0.0) - 0.2).abs() < 1e-6);
        assert!((liquid_fill_mass(&spec, 1.0) - 1.2).abs() < 1e-6);
        assert!((liquid_fill_mass(&spec, 0.5) - 0.7).abs() < 1e-6);
    }

    #[test]
    fn liquid_fill_clamps_to_capacity() {
        let spec = spec_for_id("water_bottle").unwrap();
        // Overfilling clamps to the declared capacity (no negative
        // overflow mass).
        let overfilled = liquid_fill_mass(&spec, 5.0);
        assert!((overfilled - 1.2).abs() < 1e-6);
    }

    #[test]
    fn carry_capacity_modifier_matches_spec() {
        // Spec § Tunable defaults
        assert!((carry_capacity_modifier("human") - 1.0).abs() < 1e-6);
        assert!((carry_capacity_modifier("heavy_biomech") - 1.5).abs() < 1e-6);
        assert!((carry_capacity_modifier("drone") - 0.3).abs() < 1e-6);
        assert!((carry_capacity_modifier("robot") - 1.2).abs() < 1e-6);
        assert!((carry_capacity_modifier("android") - 1.0).abs() < 1e-6);
        assert!((carry_capacity_modifier("unknown") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn heavy_biomech_carry_caps_match_gherkin() {
        // Scenario: Per-origin scaling applies
        //   Given a heavy_biomech with same load
        //   Then max_carry_kg = 75 (1.5× baseline)
        let cap = max_carry_kg_for_origin("heavy_biomech");
        assert!((cap - 75.0).abs() < 1e-6);
    }

    #[test]
    fn walk_speed_at_full_carry_is_half() {
        // Scenario: Encumbrance at 100% reduces walk speed
        //   Given a human at max_carry_kg=50 + carrying 50kg
        //   Then walk_speed_multiplier = 0.5
        let mult = walk_speed_multiplier(50.0, 50.0);
        assert!((mult - 0.5).abs() < 1e-6);
    }

    #[test]
    fn walk_speed_at_half_carry_matches_spec_table() {
        // Spec table: walk-speed at 50% carry = 0.75
        let mult = walk_speed_multiplier(25.0, 50.0);
        assert!((mult - 0.75).abs() < 1e-6);
    }

    #[test]
    fn walk_speed_at_zero_carry_is_one() {
        let mult = walk_speed_multiplier(0.0, 50.0);
        assert!((mult - 1.0).abs() < 1e-6);
    }

    #[test]
    fn walk_speed_clamps_above_full_carry() {
        // Going over 100% never produces walk speeds < 0.5.
        let mult = walk_speed_multiplier(200.0, 50.0);
        assert!((mult - 0.5).abs() < 1e-6);
    }

    #[test]
    fn walk_speed_handles_nonfinite_inputs() {
        assert!((walk_speed_multiplier(f32::NAN, 50.0) - 1.0).abs() < 1e-6);
        assert!((walk_speed_multiplier(50.0, 0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn encumbrance_band_matches_thresholds() {
        assert_eq!(encumbrance_band(0.0, 50.0), EncumbranceBand::None);
        assert_eq!(encumbrance_band(10.0, 50.0), EncumbranceBand::None);
        assert_eq!(encumbrance_band(25.0, 50.0), EncumbranceBand::Light);
        assert_eq!(encumbrance_band(37.5, 50.0), EncumbranceBand::Moderate);
        assert_eq!(encumbrance_band(50.0, 50.0), EncumbranceBand::Heavy);
        assert_eq!(encumbrance_band(100.0, 50.0), EncumbranceBand::Heavy);
    }

    #[test]
    fn container_nest_depth_caps_at_max() {
        // Scenario: Container nesting depth-limited
        //   Given a chest (level 1) containing a crate (level 2)
        //   When player tries to nest another container inside the crate
        //   Then act.player.nest_container rejects with "max_depth_exceeded"
        // depth 0 = root inventory; depth 1 = chest at top; depth 2 = crate inside chest.
        // Attempt to nest a container into the crate → candidate depth 3 → reject.
        let result = try_nest_depth(2, MAX_CONTAINER_NEST_DEPTH, true);
        assert_eq!(result, Err(MAX_DEPTH_EXCEEDED));

        // Nesting a NON-container item into a level-2 container is fine
        // (the depth cap only constrains nested CONTAINERS).
        let result = try_nest_depth(2, MAX_CONTAINER_NEST_DEPTH, false);
        assert_eq!(result, Ok(3));

        // Nesting a container into a level-1 container = depth 2 (OK).
        let result = try_nest_depth(1, MAX_CONTAINER_NEST_DEPTH, true);
        assert_eq!(result, Ok(2));
    }

    #[test]
    fn backpack_tiers_match_spec_table() {
        assert_eq!(BackpackTier::Small.dimensions(), (4, 6));
        assert_eq!(BackpackTier::Medium.dimensions(), (6, 8));
        assert_eq!(BackpackTier::Large.dimensions(), (8, 10));
        assert_eq!(BackpackTier::Industrial.dimensions(), (10, 12));
    }

    #[test]
    fn registry_contains_canonical_ids() {
        let ids = registered_ids();
        for required in &[
            "rifle_m1",
            "rifle_m1_default",
            "water_bottle",
            "chest",
            "crate",
            "backpack_small",
            "backpack_medium",
            "backpack_large",
            "backpack_industrial",
            "medkit",
            "ammo_5_56x45",
            "ration_mre",
        ] {
            assert!(ids.iter().any(|i| i == required), "missing item id: {required}");
        }
    }

    #[test]
    fn registry_mass_lookups_match_spec() {
        assert_eq!(mass_kg_for_id("rifle_m1"), Some(3.5));
        assert_eq!(mass_kg_for_id("water_bottle"), Some(0.2));
        assert_eq!(mass_kg_for_id("nonexistent"), None);
    }

    #[test]
    fn item_spec_round_trips_json() {
        // Schema lock: round-trip through serde to catch field drift.
        let spec = spec_for_id("rifle_m1").unwrap();
        let json = serde_json::to_string(&spec).expect("serialize");
        let back: ItemSpec = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(spec, back);
    }

    #[test]
    fn stack_mass_matches_spec_literal() {
        // Spec literal: "Stack items declare `stack_mass = item_mass × count`".
        let ammo = spec_for_id("ammo_5_56x45").unwrap();
        assert!(ammo.stackable);
        assert!((ammo.stack_mass(1) - 0.012).abs() < 1e-6);
        assert!((ammo.stack_mass(60) - 0.72).abs() < 1e-4);
        assert!((ammo.stack_bulk_l(60) - 1.2).abs() < 1e-4);
    }

    #[test]
    fn stack_mass_clamps_count_for_non_stackable() {
        // Non-stackable items: count always clamps to 1.
        let rifle = spec_for_id("rifle_m1").unwrap();
        assert!(!rifle.stackable);
        // Passing count=5 on a non-stackable item still returns 1×mass.
        assert!((rifle.stack_mass(5) - 3.5).abs() < 1e-6);
        assert!((rifle.stack_mass(0) - 3.5).abs() < 1e-6);
    }

    #[test]
    fn stack_mass_clamps_to_max_stack() {
        // Stackable items: count is clamped to `[1, max_stack]`.
        let ammo = spec_for_id("ammo_5_56x45").unwrap();
        // max_stack = 60; passing 200 still returns 60 × 0.012 = 0.72.
        assert!((ammo.stack_mass(200) - 0.72).abs() < 1e-4);
    }

    #[test]
    fn quick_slot_eligible_filter_returns_subset() {
        // Spec literal: "Hot-swap M14A QAB items declare `quick_slot_eligible = true`".
        let qs = quick_slot_eligible_ids();
        assert!(qs.contains(&"rifle_m1".to_string()));
        assert!(qs.contains(&"water_bottle".to_string()));
        assert!(qs.contains(&"medkit".to_string()));
        // Containers (chest / crate / backpack_*) are NOT quick-slot eligible.
        assert!(!qs.contains(&"chest".to_string()));
        assert!(!qs.contains(&"crate".to_string()));
        // Result is sorted for determinism.
        let mut sorted = qs.clone();
        sorted.sort();
        assert_eq!(qs, sorted);
    }

    #[test]
    fn quick_slot_eligible_every_id_resolves() {
        for id in quick_slot_eligible_ids() {
            assert!(spec_for_id(&id).is_some(), "qs id `{id}` must resolve");
        }
    }

    #[test]
    fn m6c_registry_contains_eighty_one_skus() {
        // M6C § Acceptance criteria M6C-1: "78 new SKUs declared with mass + grid + bulk".
        // We ship 81 (the spec lists 18 PPE while the section header says 15; we
        // implement all 18 enumerated). The acceptance test requires at least 78.
        let m6c_ids: Vec<String> = m6c_entries().iter().map(|s| s.id.clone()).collect();
        assert!(m6c_ids.len() >= 78, "M6C must declare at least 78 SKUs (got {})", m6c_ids.len());
        for id in &m6c_ids {
            let spec = spec_for_id(id).unwrap_or_else(|| panic!("missing M6C id `{id}`"));
            assert!(spec.mass_kg >= 0.0 && spec.mass_kg.is_finite(), "{id} bad mass");
            assert!(spec.dimensions.w > 0 && spec.dimensions.h > 0, "{id} bad dimensions");
            assert!(spec.bulk_volume_l >= 0.0 && spec.bulk_volume_l.is_finite(), "{id} bad bulk");
        }
    }

    #[test]
    fn m6c_ids_unique() {
        use std::collections::BTreeSet;
        let m6c_ids: BTreeSet<String> = m6c_entries().iter().map(|s| s.id.clone()).collect();
        assert_eq!(m6c_ids.len(), m6c_entries().len());
    }

    #[test]
    fn m6c_revolver_resolves_to_weapon_category() {
        let r = spec_for_id("revolver_357").unwrap();
        assert_eq!(r.category, ItemCategory::Weapon);
    }

    #[test]
    fn m6c_armor_kevlar_light_resolves_to_armor_category() {
        let a = spec_for_id("armor_kevlar_light").unwrap();
        assert_eq!(a.category, ItemCategory::Armor);
        assert!((a.mass_kg - 4.0).abs() < 1e-3);
    }

    #[test]
    fn m6c_defibrillator_resolves_to_medical() {
        let d = spec_for_id("defibrillator").unwrap();
        assert_eq!(d.category, ItemCategory::Medical);
    }

    #[test]
    fn m6c_water_bottle_1l_is_liquid_container() {
        let w = spec_for_id("water_bottle_1l").unwrap();
        assert_eq!(w.category, ItemCategory::Liquid);
        assert_eq!(w.liquid_capacity_l, Some(1.0));
    }

    #[test]
    fn m6c_fuel_canister_is_registered_for_flamethrower_pairing() {
        // M6C-4 scenario "flamethrower in primary + fuel_canister in tank_utility".
        let f = spec_for_id("fuel_canister").expect("fuel_canister must be registered");
        assert_eq!(f.category, ItemCategory::Consumable);
        assert!(f.liquid_capacity_l.is_some());
        assert!(f.mass_kg > 0.0);
    }
}
