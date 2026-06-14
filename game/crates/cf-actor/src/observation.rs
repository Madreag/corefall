use serde::{Deserialize, Serialize};

use cf_equipment::{AdvancedFireMode, BipodState};

use crate::defaults::{
    default_bloom_factor, default_mass_kg, default_origin_id, default_stability,
    default_stability_recovery_rate,
};
use crate::{ActorState, BodySilhouette, InventoryItem, LimbLossFlags};


/// M17 per-origin resource projection for the observe envelope + HUD bridge.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct M17ResourceView {
    pub origin: String,
    pub blood: f32,
    pub blood_max: f32,
    pub oil: f32,
    pub oil_max: f32,
    pub power: f32,
    pub power_max: f32,
    pub caloric: f32,
    pub oxygen_seconds: f32,
    pub heat: f32,
    pub concussion_dose: f32,
    pub concussion_band: String,
    pub internal_shock_dose: f32,
    pub power_fire_locked: bool,
    pub overclock_tier: u8,
    pub throttled: bool,
    // --- M20 EnvironmentSignal exposure slices (single source of truth the
    // HUD / AI / replay / accessibility consume) ---
    /// True when the actor is an oxygen-breathing organic (for AI O2 logic).
    pub is_organic: bool,
    /// True when ambient pressure is at / near vacuum.
    pub is_vacuum_exposed: bool,
    /// Sealed helmet (or dive suit) present.
    pub helmet_sealed: bool,
    /// Ambient O2 partial pressure (kPa).
    pub ambient_o2_kpa: f32,
    /// Robot chassis heat fraction is in the meltdown-imminent band.
    pub meltdown_imminent: bool,
    /// Thermal band label (nominal / throttle / critical / meltdown).
    pub thermal_band: String,
    /// Mobility multiplier from resource degradation (0 = seized).
    pub mobility_mult: f32,
    /// Action-speed multiplier (overclock boost / throttle).
    pub action_speed_factor: f32,
}

impl M17ResourceView {
    pub fn from_actor(actor: &ActorState) -> Self {
        let profile = crate::origin::OriginProfile::canonical(actor.origin());
        let r = &actor.resources;
        let band = crate::overclock::ThermalBand::from_heat(r.heat);
        Self {
            origin: actor.origin_id.clone(),
            blood: r.blood,
            blood_max: profile.blood_max_ml,
            oil: r.oil,
            oil_max: profile.oil_max_ml,
            power: r.power,
            power_max: profile.power_max_kwh,
            caloric: r.caloric_energy,
            oxygen_seconds: r.oxygen_supply,
            heat: r.heat,
            concussion_dose: r.concussion_dose,
            concussion_band: crate::concussion::ConcussionBand::from_dose(r.concussion_dose)
                .as_str()
                .to_string(),
            internal_shock_dose: r.internal_shock_dose,
            power_fire_locked: actor.power_fire_locked,
            overclock_tier: actor.overclock.tier,
            throttled: actor.overclock.throttled,
            is_organic: profile.oxygen_required || profile.has_blood() || profile.has_bio_fluid(),
            is_vacuum_exposed: actor.atmosphere_sample.pressure_kpa < 11.0,
            helmet_sealed: actor.body_armor.helmet_seal_active() || actor.body_armor.dive_suit_equipped(),
            ambient_o2_kpa: actor.atmosphere_sample.o2_partial_kpa,
            meltdown_imminent: matches!(
                band,
                crate::overclock::ThermalBand::Critical | crate::overclock::ThermalBand::Meltdown
            ),
            thermal_band: band.as_str().to_string(),
            mobility_mult: actor.m17_mobility_mult,
            action_speed_factor: actor.action_speed_factor,
        }
    }
}

/// Public projection of an actor for the cf-control observe envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorObservation {
    pub id: u64,
    pub team: String,
    pub controllable: bool,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub aim: [f32; 2],
    pub on_ground: bool,
    pub status: String,
    pub hp: f32,
    pub hp_max: f32,
    pub selected_slot: u32,
    pub selected_item: String,
    /// item labels in slot order (length 4 for M1; matches `Inventory.items`).
    /// Closes the M1 spec drift item — spec said the observation includes
    /// `inventory[]` but code only surfaced `selected_slot + selected_item`.
    #[serde(default)]
    pub inventory: Vec<String>,
    /// M4A: derived stance label (idle/walking/running/airborne/downed/dead/crouching/...).
    pub stance: String,
    /// M4A: per-zone body silhouette projection. `placeholder=false` when sourced
    /// from a real M5 chassis body graph.
    pub body_silhouette: BodySilhouette,
    #[serde(default)]
    pub chassis: Option<ChassisView>,
    #[serde(default = "default_origin_id")]
    pub origin_id: String,
    /// M17 — per-origin survival resources + concussion / overclock state.
    #[serde(default)]
    pub m17: M17ResourceView,
    /// W1.3: stability scalar (0.0 = fully disrupted, 1.0 = stable).
    #[serde(default = "default_stability")]
    pub stability: f32,
    /// W1.3: stability recovery rate per tick when grounded.
    #[serde(default = "default_stability_recovery_rate")]
    pub stability_recovery_rate: f32,
    /// Physical mass in kg. Affects movement feel, stability resistance, knockdown.
    #[serde(default = "default_mass_kg")]
    pub mass_kg: f32,
    #[serde(default)]
    pub crouch_active: bool,
    #[serde(default)]
    pub climb_active: bool,
    #[serde(default)]
    pub jet_active: bool,
    #[serde(default)]
    pub sharp_aim_progress: f32,
    #[serde(default)]
    pub recoil_accumulator: f32,
    #[serde(default)]
    pub knockdown_ticks_remaining: u32,
    #[serde(default)]
    pub dying_dwell_ticks_remaining: u32,
    #[serde(default)]
    pub mission_critical: bool,
    /// 1.0 = standing/walking; >1 = movement / airborne / sharp-aim breakup.
    #[serde(default = "default_bloom_factor")]
    pub bloom_factor: f32,
    #[serde(default)]
    pub facing: String,
    #[serde(default)]
    pub stamina: f32,
    #[serde(default)]
    pub stamina_max: f32,
    #[serde(default)]
    pub sprint_active: bool,
    #[serde(default)]
    pub prone_active: bool,
    #[serde(default)]
    pub lean_angle_degrees: f32,
    #[serde(default)]
    pub lean_direction: String,
    #[serde(default)]
    pub stealth_meter: f32,
    #[serde(default)]
    pub spotted: bool,
    #[serde(default)]
    pub cover_side: String,
    #[serde(default)]
    pub cover_effectiveness: f32,
    #[serde(default)]
    pub inventory_weight_kg: f32,
    #[serde(default)]
    pub weight_forces_walk: bool,
    /// + action-rejection contract.
    #[serde(default)]
    pub limb_loss: LimbLossFlags,
    /// Each entry includes kind + state ("empty" / "occupied" / "locked")
    /// + the locked tooltip on the reserved slots.
    #[serde(default)]
    pub inventory_extended: Vec<ExtendedInventorySlotView>,
    /// suppressor_attached, reload_state, charge_fraction). See
    /// [`WeaponStateView`] for the shape contract.
    #[serde(default)]
    pub weapon_state: WeaponStateView,
    /// player's brain currently resides in this actor.
    #[serde(default)]
    pub is_brain: bool,
    /// reaction label + ticks remaining + speed factor. Empty label when no
    /// reaction is active.
    #[serde(default)]
    pub hit_reaction_kind: String,
    #[serde(default)]
    pub hit_reaction_ticks_remaining: u32,
    /// the actor is a drone).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drone_fuel: Option<f32>,
    #[serde(default)]
    pub max_carry_kg: f32,
    #[serde(default)]
    pub max_carry_volume_l: f32,
    /// Distinct from the legacy `inventory_weight_kg` (M6 slot sum) so
    /// the M14A mass aggregator can consume a per-item canonical surface.
    #[serde(default)]
    pub total_carried_kg: f32,
    #[serde(default)]
    pub total_carried_volume_l: f32,
    /// (`1.0` empty, `0.5` at 100% carry).
    #[serde(default = "default_bloom_factor")]
    pub encumbrance_walk_speed_multiplier: f32,
    #[serde(default)]
    pub encumbrance_band: String,
    /// "ENCUMBERED" warning per spec § "Encumbrance at 100% reduces
    /// walk speed").
    #[serde(default)]
    pub encumbered: bool,
    /// mass + bulk + nested container counts). `None` for pre-M6B
    /// legacy actors; `Some(...)` for any actor with an attached
    /// `inventory_grid`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory_grid: Option<InventoryGridView>,
}

/// `cf_equipment::inventory::ExtendedSlot` but lives here so observe.actor
/// stays a pure cf-actor projection.
///
/// surfaces both per-slot mass + bulk per spec § Crates / modules touched
/// (cf-control MODIFY — observe.actor.inventory extended with mass + bulk).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ExtendedInventorySlotView {
    pub kind: String,
    pub state: String,
    #[serde(default)]
    pub item_id: String,
    #[serde(default)]
    pub weight_kg: f32,
    /// `cf_equipment::ItemSpec.bulk_volume_l`). Zero for empty slots.
    #[serde(default)]
    pub bulk_volume_l: f32,
    #[serde(default)]
    pub locked_tooltip: Option<String>,
}

/// `observe.actor`. Surfaces the canonical mass + bulk per item so the
/// HUD + M27 Tetris UX + M14A mass aggregator all see one source of
/// truth without each having to recompute from the spec registry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InventoryGridPlacementView {
    pub instance_id: u64,
    pub item_id: String,
    pub category: String,
    pub origin: [u8; 2],
    pub dimensions: [u8; 2],
    pub rotated: bool,
    pub stack_count: u16,
    pub mass_kg: f32,
    pub bulk_volume_l: f32,
    pub is_container: bool,
    pub nested_count: u16,
    pub current_liquid_l: f32,
    pub liquid_capacity_l: f32,
    pub quick_slot_eligible: bool,
}

/// `observe.actor.inventory_grid`. Mirrors
/// `cf_actor::InventoryGrid` with derived totals so cfctl consumers see
/// the canonical M6B surface without consulting the engine binary.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InventoryGridView {
    pub tier: String,
    pub grid_w: u8,
    pub grid_h: u8,
    pub placements: Vec<InventoryGridPlacementView>,
    pub total_mass_kg: f32,
    pub total_bulk_l: f32,
}

///
/// `Idle` covers both "ready to fire" and "between shots / pump-action
/// chamber" — anything that is not actively in a multi-tick reload animation.
/// `Reloading` covers the multi-tick reload window driven by the M1
/// `cf_equipment::RifleState::reload_remaining_ticks` counter (plus any
/// future weapon-specific reload state machines).
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReloadState {
    #[default]
    Idle = 0,
    Reloading = 1,
}

impl ReloadState {
    pub fn as_str(self) -> &'static str {
        match self {
            ReloadState::Idle => "idle",
            ReloadState::Reloading => "reloading",
        }
    }
}

/// / modules touched / cf-actor" bullet "ActorObservation extensions
/// (cover_state, stamina, lean_angle, weapon_state)" weapon_state field).
///
/// Six fields are surfaced, all live:
///
/// - `mag_remaining` — current rounds in the chambered magazine, read from
///   the live [`cf_equipment::RifleState::ammo_in_mag`] when the engine
///   passes a rifle handle; falls back to the rifle preset's `mag_capacity`
///   when no rifle state is available (e.g. test paths).
/// - `fire_mode` — extended fire-mode discriminator
///   ([`cf_equipment::AdvancedFireMode`]) reflecting the live
///   `ActorState::weapon_fire_mode`. Rotated by
///   `act.player.cycle_fire_mode`.
/// - `bipod_state` — current [`cf_equipment::BipodState`] (`Stowed` /
///   `Deployed`) read from `ActorState::bipod.state`.
/// - `suppressor_attached` — true when `ActorState::suppressor.attached`
///   is set on the actor's currently-equipped weapon.
/// - `reload_state` — [`ReloadState`] reload-window discriminator (`Idle`
///   vs `Reloading`), derived from
///   [`cf_equipment::RifleState::reload_remaining_ticks`] (`Reloading` when
///   `> 0`, otherwise `Idle`). Defaults to `Idle` when no rifle handle is
///   threaded through.
/// - `charge_fraction` — charge-mode (e.g. sniper) accumulator scalar
///   `0..1` from `ActorState::weapon_charge_fraction`. 0.0 when the
///   weapon is not in `AdvancedFireMode::Charge` mode or the trigger
///   has not been held this trigger cycle.
///
/// The shape is deliberately additive — every field carries a default so the
/// observation surface remains consistent for the conversion paths that
/// only see [`ActorState`]. Engine code that has the per-actor
/// [`cf_equipment::RifleState`] in hand should use
/// [`ActorObservation::from_actor_and_rifle`] so the magazine + reload
/// fields reflect the live tick state.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct WeaponStateView {
    pub mag_remaining: u32,
    pub fire_mode: AdvancedFireMode,
    pub bipod_state: BipodState,
    pub suppressor_attached: bool,
    pub reload_state: ReloadState,
    pub charge_fraction: f32,
}

impl ActorObservation {
    /// threaded through so [`WeaponStateView::mag_remaining`] and
    /// [`WeaponStateView::reload_state`] reflect the live tick values from
    /// the engine's [`crate::sim::RifleStates`] map. Pass `None` from
    /// contexts that don't track rifle state (tests, benches, and any
    /// pre-rifle-allocation path); the magazine field falls back to the
    /// rifle preset's `mag_capacity` and reload reports `Idle`.
    pub fn from_actor_and_rifle(actor: &ActorState, rifle: Option<&cf_equipment::RifleState>) -> Self {
        Self {
            id: actor.id.0,
            team: actor.team.clone(),
            controllable: actor.controllable,
            position: [actor.position.x, actor.position.y],
            velocity: [actor.velocity.x, actor.velocity.y],
            aim: [actor.aim.x, actor.aim.y],
            on_ground: actor.on_ground,
            status: actor.status.as_str().to_string(),
            hp: actor.hp,
            hp_max: actor.hp_max,
            selected_slot: actor.inventory.selected.0,
            selected_item: actor.inventory.selected_item().label().to_string(),
            inventory: actor.inventory.items.iter().map(|i| i.label().to_string()).collect(),
            stance: actor.stance().as_str().to_string(),
            body_silhouette: actor.body_silhouette(),
            chassis: actor.chassis_view(),
            origin_id: actor.origin_id.clone(),
            m17: M17ResourceView::from_actor(actor),
            stability: actor.stability,
            stability_recovery_rate: actor.stability_recovery_rate,
            mass_kg: actor.mass_kg,
            crouch_active: actor.crouch_active,
            climb_active: actor.climb_active,
            jet_active: actor.jet_active,
            sharp_aim_progress: actor.sharp_aim_progress,
            recoil_accumulator: actor.recoil_accumulator,
            knockdown_ticks_remaining: actor.knockdown_ticks_remaining,
            dying_dwell_ticks_remaining: actor.dying_dwell_ticks_remaining,
            mission_critical: actor.mission_critical,
            bloom_factor: actor.bloom_factor,
            facing: actor.facing.as_str().to_string(),
            stamina: actor.stamina.current,
            stamina_max: actor.stamina.max,
            sprint_active: actor.sprint_active,
            prone_active: actor.prone_active,
            lean_angle_degrees: actor.lean_state.angle_degrees,
            lean_direction: actor.lean_state.direction.as_str().to_string(),
            stealth_meter: actor.stealth_meter,
            spotted: actor.stealth_meter >= 0.5,
            cover_side: actor.cover_state.side.as_str().to_string(),
            cover_effectiveness: actor.cover_state.effectiveness,
            inventory_weight_kg: actor.inventory_weight_kg,
            weight_forces_walk: actor.inventory_weight_kg > 30.0,
            limb_loss: actor.limb_loss,
            inventory_extended: actor.extended_inventory_view(),
            weapon_state: actor.weapon_state_view(rifle),
            is_brain: actor.is_brain,
            hit_reaction_kind: actor.hit_reaction_kind.clone(),
            hit_reaction_ticks_remaining: actor.hit_reaction_ticks_remaining,
            drone_mode: actor.drone_ally.as_ref().map(|d| d.mode.as_str().to_string()),
            drone_fuel: actor.drone_ally.as_ref().map(|d| d.fuel),
            max_carry_kg: actor.max_carry_kg(),
            max_carry_volume_l: actor.max_carry_volume_l(),
            total_carried_kg: actor.inventory_grid_total_mass_kg(),
            total_carried_volume_l: actor.inventory_grid_total_bulk_l(),
            encumbrance_walk_speed_multiplier: actor.encumbrance_walk_speed_multiplier(),
            encumbrance_band: actor.encumbrance_band().as_str().to_string(),
            encumbered: actor.is_encumbered(),
            inventory_grid: actor.inventory_grid_view(),
        }
    }
}

impl From<&ActorState> for ActorObservation {
    fn from(actor: &ActorState) -> Self {
        Self::from_actor_and_rifle(actor, None)
    }
}

impl ActorState {
    /// tank slots, with the tank slots reporting `state="locked"`).
    ///
    /// Slots 0..=7 mirror the actor's `Inventory.items` (8-slot vec on
    /// M6+; legacy 4-slot vecs naturally project as empty for the upper
    /// 4 slots). Slots 8..=10 are the M17 forward-compat tank slots and
    /// always report `state="locked"`.
    pub fn extended_inventory_view(&self) -> Vec<ExtendedInventorySlotView> {
        let slot_kinds = [
            "primary",
            "secondary",
            "sidearm",
            "tool1",
            "tool2",
            "grenade",
            "medical",
            "special",
        ];
        let tank_kinds = [
            ("tank_primary", "Reserved — see M17 for tank ladder"),
            ("tank_secondary", "Reserved — see M17 for tank ladder"),
            ("tank_utility", "Reserved — see M17 for tank ladder"),
        ];
        let mut out = Vec::with_capacity(slot_kinds.len() + tank_kinds.len());
        for (i, name) in slot_kinds.iter().enumerate() {
            let item = self.inventory.items.get(i).cloned().unwrap_or(InventoryItem::Empty);
            // `cf_equipment::ItemSpec` registry. Unknown ids fall back
            // to the M6 hardcoded weight (3.5) for legacy compat.
            let (state, item_id, weight, bulk) = match &item {
                InventoryItem::Empty => ("empty", String::new(), 0.0, 0.0),
                InventoryItem::Rifle { preset } => {
                    let spec = cf_equipment::spec_for_id(preset);
                    let mass = spec.as_ref().map_or(3.5, |s| s.mass_kg);
                    let bulk = spec.as_ref().map_or(0.0, |s| s.bulk_volume_l);
                    ("occupied", preset.clone(), mass, bulk)
                }
            };
            out.push(ExtendedInventorySlotView {
                kind: (*name).to_string(),
                state: state.to_string(),
                item_id,
                weight_kg: weight,
                bulk_volume_l: bulk,
                locked_tooltip: None,
            });
        }
        for (name, tooltip) in &tank_kinds {
            out.push(ExtendedInventorySlotView {
                kind: (*name).to_string(),
                state: "locked".to_string(),
                item_id: String::new(),
                weight_kg: 0.0,
                bulk_volume_l: 0.0,
                locked_tooltip: Some((*tooltip).to_string()),
            });
        }
        out
    }

    /// `observe.actor.inventory_grid`. Walks every top-level placement
    /// in the grid and emits a [`InventoryGridPlacementView`] with the
    /// canonical mass + bulk + category + nested-count derived from
    /// the [`cf_equipment::ItemSpec`] registry. Returns `None` when no
    /// grid is attached (pre-M6B legacy actors).
    pub fn inventory_grid_view(&self) -> Option<InventoryGridView> {
        let grid = self.inventory_grid.as_ref()?;
        let (w, h) = grid.dimensions();
        let placements = grid
            .items
            .iter()
            .map(|p| {
                let spec = cf_equipment::spec_for_id(&p.item_id);
                let (mass, bulk, dims, category, is_container, liquid_cap, quick_slot) = match spec {
                    Some(s) => (
                        p.mass_kg(&s),
                        p.bulk_volume_l(&s),
                        if p.rotated {
                            s.dimensions.rotated()
                        } else {
                            s.dimensions
                        },
                        s.category.as_str().to_string(),
                        s.is_container(),
                        s.liquid_capacity_l.unwrap_or(0.0),
                        s.quick_slot_eligible,
                    ),
                    None => (
                        0.0,
                        0.0,
                        cf_equipment::GridDim::new(1, 1),
                        String::new(),
                        false,
                        0.0,
                        false,
                    ),
                };
                let nested_count = p.container.as_ref().map(|c| c.items.len() as u16).unwrap_or(0);
                InventoryGridPlacementView {
                    instance_id: p.instance_id,
                    item_id: p.item_id.clone(),
                    category,
                    origin: [p.origin.0, p.origin.1],
                    dimensions: [dims.w, dims.h],
                    rotated: p.rotated,
                    stack_count: p.count,
                    mass_kg: mass,
                    bulk_volume_l: bulk,
                    is_container,
                    nested_count,
                    current_liquid_l: p.current_liquid_l,
                    liquid_capacity_l: liquid_cap,
                    quick_slot_eligible: quick_slot,
                }
            })
            .collect();
        Some(InventoryGridView {
            tier: grid.tier.as_str().to_string(),
            grid_w: w,
            grid_h: h,
            placements,
            total_mass_kg: grid.total_mass_kg(),
            total_bulk_l: grid.total_bulk_l(),
        })
    }

    /// selected weapon. All six fields are now live:
    ///
    /// - `mag_remaining` reads from
    ///   [`cf_equipment::RifleState::ammo_in_mag`] when the caller threads
    ///   the per-actor rifle state; otherwise falls back to the rifle
    ///   preset's `mag_capacity` (and `0` when the active slot is not a
    ///   rifle).
    /// - `fire_mode` reads [`ActorState::weapon_fire_mode`] (rotated by
    ///   `act.player.cycle_fire_mode`).
    /// - `bipod_state` reads [`ActorState::bipod.state`].
    /// - `suppressor_attached` reads [`ActorState::suppressor.attached`].
    /// - `reload_state` is derived from
    ///   [`cf_equipment::RifleState::reload_remaining_ticks`] (`Reloading`
    ///   when `> 0`, `Idle` otherwise). When the caller passes `None`,
    ///   `Idle` is reported.
    /// - `charge_fraction` reads
    ///   [`ActorState::weapon_charge_fraction`] (filled by the
    ///   Charge-mode firing path).
    ///
    /// See the [`WeaponStateView`] doc comment for the full shape
    /// contract.
    pub fn weapon_state_view(&self, rifle: Option<&cf_equipment::RifleState>) -> WeaponStateView {
        let mag_remaining = match (self.inventory.selected_item(), rifle) {
            (InventoryItem::Rifle { .. }, Some(r)) => r.ammo_in_mag,
            (InventoryItem::Rifle { preset }, None) => {
                cf_equipment::rifle_preset(preset).map_or(0, |spec| spec.mag_capacity)
            }
            (InventoryItem::Empty, _) => 0,
        };
        let reload_state = match rifle {
            Some(r) if r.reload_remaining_ticks > 0 => ReloadState::Reloading,
            _ => ReloadState::Idle,
        };
        WeaponStateView {
            mag_remaining,
            fire_mode: self.weapon_fire_mode,
            bipod_state: self.bipod.state,
            suppressor_attached: self.suppressor.attached,
            reload_state,
            charge_fraction: self.weapon_charge_fraction.clamp(0.0, 1.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisView {
    pub spec_id: String,
    pub kind: String,
    pub stage: String,
    pub pilot_state: String,
    pub weapon_jammed: bool,
    pub tutorial_safety: bool,
    pub mass_kg: f32,
    pub zones: Vec<ChassisZoneView>,
    pub modules: Vec<ChassisModuleView>,
    pub integrity: f32,
    pub eject_ticks_remaining: u32,
    pub eject_ticks_total: u32,
    pub destroyed_zones: Vec<String>,
    pub salvaged_module_ids: Vec<String>,
}

/// Per-zone chassis view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisZoneView {
    pub zone: String,
    pub external_integrity: f32,
    pub internal_integrity: f32,
    pub core_integrity: f32,
    pub wound_integrity: f32,
    pub destroyed: bool,
    pub zone_integrity: f32,
}

/// Per-module chassis view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChassisModuleView {
    pub id: String,
    pub kind: String,
    pub state: String,
    pub bound_zone: String,
    pub integrity: f32,
    pub last_reason: String,
}
