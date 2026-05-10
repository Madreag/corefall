//! M1: actor sim primitives.
//!
//! This crate owns the cross-binary types used by `cf-control`'s engine, `cf-app`'s
//! Bevy bridge, and the future networking/AI crates:
//!
//! - [`ActorId`], [`Status`], [`Inventory`], [`ActorState`]: components in the M1
//!   data model. Bevy ECS components are NOT defined here; the renderer wraps these
//!   types with `#[derive(Component)]` newtypes in `cf-app`/`cf-render-2d`.
//! - [`ActorWorld`]: the authoritative simulation state. Owned by the `cf-control`
//!   engine, drained per fixed tick.
//! - [`ControlIntent`]: a single tick's worth of player input. Produced by `cf-control`
//!   from JSON-RPC commands or Bevy keyboard/mouse input, consumed by [`ActorWorld::tick`].
//! - [`ActorObservation`]: the snapshot shape exposed via `observe.once`/`observe.frame`.
//!
//! Determinism contract: every public mutator is pure (state in → state out via `&mut self`)
//! and never reads a wall clock or `rand::thread_rng`. The engine's seeded RNG is the only
//! source of nondeterminism allowed inside a tick, and it is wired in by callers.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::struct_excessive_bools,
    clippy::derivable_impls,
    clippy::missing_const_for_fn,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::items_after_statements,
    clippy::return_self_not_must_use,
    clippy::float_cmp,
    clippy::if_not_else,
    clippy::cast_lossless,
    clippy::for_kv_map,
    clippy::too_many_lines,
    clippy::needless_pass_by_value
)]

pub mod sim;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Stable per-actor id. Allocated by the scenario loader; future networking will
/// reuse the same id space across the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActorId(pub u64);

impl ActorId {
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Body status state machine (M1 minimum surface; M3/M4/M5 expand wounds + chassis layers).
///
/// `#[repr(u8)]` with explicit discriminants pins the layout used by
/// [`ActorState::checksum_bytes`] so future milestones can append new variants
/// (after `Dead`) without silently shifting determinism checksums. Inserting a
/// variant in the middle would require bumping the checksum schema.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Healthy and acting normally.
    Stable = 0,
    /// Below `hp_unstable_threshold`; movement still works but HUD warns.
    Unstable = 1,
    /// At/below `hp_downed_threshold`; player loses control but is not yet dead.
    Downed = 2,
    /// HP at zero; cannot recover this run.
    Dead = 3,
}

impl Status {
    pub fn is_dead(self) -> bool {
        matches!(self, Status::Dead)
    }

    /// True if the actor can accept new control input (move / aim / fire / reload).
    pub fn accepts_input(self) -> bool {
        matches!(self, Status::Stable | Status::Unstable)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Stable => "stable",
            Status::Unstable => "unstable",
            Status::Downed => "downed",
            Status::Dead => "dead",
        }
    }
}

/// M4A readable stance/locomotion state derived from per-tick actor state.
///
/// Until M5 introduces real chassis, body graph, and animation events, the M4A
/// stance is a derivation of `velocity`, `on_ground`, and `Status` that gives
/// the HUD, `cfctl observe`, and AI agents a non-color-only readable signal
/// of what the body is doing (per `spec/animation-system` and DR-003).
/// M5 replaces the derivation with explicit animation-state and chassis-stage
/// tags but the HUD/observe surface contract stays stable.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// On ground, stationary, accepting input.
    Idle = 0,
    /// On ground, horizontal velocity > walk threshold and < run threshold.
    Walking = 1,
    /// On ground, horizontal velocity >= run threshold.
    Running = 2,
    /// Off ground (jumping, falling, jetting).
    Airborne = 3,
    /// Status::Downed but still alive.
    Downed = 4,
    /// Status::Dead.
    Dead = 5,
}

impl Stance {
    /// Threshold below which horizontal velocity counts as `Idle` (world units / s).
    /// Mirrors `cf-physics::WALK_SPEED_FLOOR` in spirit; we keep the literal here so
    /// `cf-actor` does not depend on physics constants.
    pub const WALK_THRESHOLD: f32 = 8.0;
    /// Threshold at/above which horizontal velocity counts as `Running`.
    pub const RUN_THRESHOLD: f32 = 60.0;

    pub fn as_str(self) -> &'static str {
        match self {
            Stance::Idle => "idle",
            Stance::Walking => "walking",
            Stance::Running => "running",
            Stance::Airborne => "airborne",
            Stance::Downed => "downed",
            Stance::Dead => "dead",
        }
    }

    /// Derive stance from kinematic + status state. Pure; no clock reads.
    pub fn from_state(velocity: Vec2, on_ground: bool, status: Status) -> Stance {
        match status {
            Status::Dead => Stance::Dead,
            Status::Downed => Stance::Downed,
            Status::Stable | Status::Unstable => {
                if !on_ground {
                    Stance::Airborne
                } else {
                    let speed = velocity.x.abs();
                    if speed >= Self::RUN_THRESHOLD {
                        Stance::Running
                    } else if speed >= Self::WALK_THRESHOLD {
                        Stance::Walking
                    } else {
                        Stance::Idle
                    }
                }
            }
        }
    }
}

/// Item id used by the M1 inventory. Maps 1:1 to a slot index in [`Inventory::items`].
/// Resolved against per-actor item presets (`cf-equipment::RIFLE_M1_DEFAULT_ID`, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ItemSlot(pub u32);

impl ItemSlot {
    pub const fn raw(self) -> u32 {
        self.0
    }
}

/// 2D vector used by sim systems. We do NOT depend on `glam` here so this crate stays
/// dependency-light. The Bevy bridge converts to `Vec2`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    /// Returns a unit vector. If the input is the zero vector OR contains a non-finite
    /// component (NaN / Inf), returns `Vec2::new(1.0, 0.0)` so consumers (e.g. weapon
    /// muzzle origin, projectile velocity, recoil) never produce NaNs. NaN comparisons
    /// always return false, so a plain `len < 1e-6` guard is NOT sufficient — we must
    /// explicitly check `is_finite()` on every component.
    pub fn normalize_or_x(self) -> Vec2 {
        if !self.x.is_finite() || !self.y.is_finite() {
            return Vec2::new(1.0, 0.0);
        }
        let len = self.length();
        // 1e-6 tolerance picked because: at f32 precision, vector lengths
        // below ~1.2e-38 underflow to zero outright (subnormal); below
        // ~1e-6 the per-component division `self.x / len` loses ~6
        // significant digits and the resulting "normalized" vector points
        // in essentially-arbitrary directions. 1e-6 is safely above that
        // floor at the canonical aim/muzzle-velocity scale (1 unit = 1 m,
        // typical aim magnitudes 0.1-1.0). When BP4-BP5 introduce
        // sub-millimeter precision physics (e.g., particle systems), this
        // should become scale-relative (issue #19 follow-up).
        if !len.is_finite() || len < 1e-6 {
            Vec2::new(1.0, 0.0)
        } else {
            Vec2::new(self.x / len, self.y / len)
        }
    }
}

impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, rhs: Vec2) -> Vec2 {
        Vec2::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Vec2 {
        Vec2::new(self.x * rhs, self.y * rhs)
    }
}

/// One inventory slot with a fixed item kind and (optional) ammo state.
///
/// Kept simple in M1: each actor has up to 4 slots. The "selected" slot drives
/// `weapon_fired` / `weapon_reloaded`. Slots beyond the rifle are placeholders for M5.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InventoryItem {
    /// Nothing in the slot.
    Empty,
    /// One M1 rifle preset. Concrete fire/reload state lives in `cf-equipment::RifleState`.
    /// We keep the slot shape here so the scenario manifest stays decoupled from rifle internals.
    Rifle { preset: String },
}

impl InventoryItem {
    pub fn label(&self) -> &str {
        match self {
            InventoryItem::Empty => "empty",
            InventoryItem::Rifle { .. } => "rifle",
        }
    }

    pub fn is_rifle(&self) -> bool {
        matches!(self, InventoryItem::Rifle { .. })
    }
}

/// Up to four inventory slots. M1 ships one rifle; remaining slots are `Empty`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Inventory {
    pub items: Vec<InventoryItem>,
    pub selected: ItemSlot,
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            items: vec![InventoryItem::Empty; 4],
            selected: ItemSlot(0),
        }
    }
}

impl Inventory {
    pub fn with_rifle(preset: &str) -> Self {
        let mut inv = Self::default();
        inv.items[0] = InventoryItem::Rifle {
            preset: preset.to_string(),
        };
        inv
    }

    pub fn selected_item(&self) -> &InventoryItem {
        self.items
            .get(self.selected.0 as usize)
            .unwrap_or(&InventoryItem::Empty)
    }

    /// Set `selected` to the requested slot iff the slot exists. Returns true if the
    /// selection changed.
    pub fn try_select(&mut self, slot: ItemSlot) -> bool {
        if (slot.0 as usize) < self.items.len() && self.selected != slot {
            self.selected = slot;
            true
        } else {
            false
        }
    }

    pub fn rifle_slot(&self) -> Option<ItemSlot> {
        self.items
            .iter()
            .enumerate()
            .find_map(|(i, it)| if it.is_rifle() { Some(ItemSlot(i as u32)) } else { None })
    }
}

/// Source of a `ControlIntent` for replay/audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentSource {
    /// Keyboard / mouse / gamepad inside `cf-app`.
    Human,
    /// JSON-RPC `act.player.*` calls coming from `cfctl` / scripted E2E / future bots.
    Cfctl,
}

/// One tick's worth of player input. Produced by `cf-control` and applied by
/// [`ActorWorld::tick`]. Sticky vs. edge-triggered semantics matter:
///
/// - `move_x`, `aim`: continuous (latest value wins).
/// - `jump`, `fire`, `reload`, `selected_item`, `reset`: edge-triggered (true only on
///   the tick the button was pressed; cleared by the engine after consumption).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ControlIntent {
    pub actor: ActorId,
    pub source: IntentSource,
    pub move_x: f32,
    pub jump: bool,
    pub aim: Vec2,
    pub fire: bool,
    pub reload: bool,
    pub selected_item: Option<ItemSlot>,
    pub reset: bool,
}

impl Default for IntentSource {
    fn default() -> Self {
        IntentSource::Human
    }
}

impl ControlIntent {
    pub fn new(actor: ActorId, source: IntentSource) -> Self {
        Self {
            actor,
            source,
            ..Self::default()
        }
    }

    /// Reset edge-triggered fields. Continuous fields (move_x, aim) are preserved.
    pub fn clear_edges(&mut self) {
        self.jump = false;
        self.fire = false;
        self.reload = false;
        self.selected_item = None;
        self.reset = false;
    }

    /// Returns true when no actively-driven input is present. `aim` is a
    /// continuous field that persists across ticks (not cleared by
    /// [`clear_edges`](Self::clear_edges)), so it is intentionally excluded
    /// here — a sticky aim direction does not indicate the player is
    /// currently providing input.
    pub fn is_idle(&self) -> bool {
        self.move_x.abs() < f32::EPSILON
            && !self.jump
            && !self.fire
            && !self.reload
            && self.selected_item.is_none()
            && !self.reset
    }
}

/// Per-actor authoritative state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActorState {
    pub id: ActorId,
    pub team: String,
    pub spawn: Vec2,
    pub position: Vec2,
    pub velocity: Vec2,
    pub aim: Vec2,
    pub on_ground: bool,
    pub status: Status,
    pub hp: f32,
    pub hp_max: f32,
    pub hp_unstable_threshold: f32,
    pub hp_downed_threshold: f32,
    pub inventory: Inventory,
    /// True if this actor accepts player intent (only one in M1 scenarios).
    pub controllable: bool,
    /// Half-extents of the AABB used for ground collision + future limb proxies.
    /// M1 uses a chunky 8x16 actor footprint; M5 will replace this with chassis zones.
    pub half_extents: Vec2,
}

impl ActorState {
    /// Create a default M1 player actor at `spawn` with `inventory` and full HP.
    pub fn player(id: ActorId, team: impl Into<String>, spawn: Vec2, hp_max: f32, inventory: Inventory) -> Self {
        Self {
            id,
            team: team.into(),
            spawn,
            position: spawn,
            velocity: Vec2::ZERO,
            aim: Vec2::new(1.0, 0.0),
            on_ground: false,
            status: Status::Stable,
            hp: hp_max,
            hp_max,
            hp_unstable_threshold: hp_max * 0.5,
            hp_downed_threshold: hp_max * 0.1,
            inventory,
            controllable: true,
            half_extents: Vec2::new(8.0, 16.0),
        }
    }

    /// Reset the actor back to its spawn state. Position, velocity, aim, on-ground,
    /// status, and HP all return to defaults; the selected inventory slot is cleared
    /// to `0` so the actor can fire its rifle again after `act.player.reset`.
    /// Inventory items themselves are not rewound (slot contents are immutable in M1).
    pub fn reset(&mut self) {
        self.position = self.spawn;
        self.velocity = Vec2::ZERO;
        self.aim = Vec2::new(1.0, 0.0);
        self.on_ground = false;
        self.status = Status::Stable;
        self.hp = self.hp_max;
        self.inventory.selected = ItemSlot(0);
    }

    /// Apply damage with a cause string. Returns the new status if it changed.
    pub fn apply_damage(&mut self, amount: f32) -> Option<Status> {
        if amount <= 0.0 || self.status.is_dead() {
            return None;
        }
        self.hp = (self.hp - amount).max(0.0);
        let new_status = self.derived_status();
        if new_status != self.status {
            self.status = new_status;
            Some(new_status)
        } else {
            None
        }
    }

    fn derived_status(&self) -> Status {
        if self.hp <= 0.0 {
            Status::Dead
        } else if self.hp <= self.hp_downed_threshold {
            Status::Downed
        } else if self.hp <= self.hp_unstable_threshold {
            Status::Unstable
        } else {
            Status::Stable
        }
    }

    /// Derived M4A stance for HUD + `cfctl observe`. See [`Stance::from_state`].
    pub fn stance(&self) -> Stance {
        Stance::from_state(self.velocity, self.on_ground, self.status)
    }

    /// M4A body silhouette per-zone hp percentage. At M4A there is no real body
    /// graph (M5 owns that); we project the actor's total HP onto a four-zone
    /// silhouette so the HUD silhouette and `cfctl observe.body_silhouette`
    /// render a non-color-only damage map. M5 replaces this with the real
    /// per-zone wound model from `spec/body-damage-model` without changing
    /// the surface contract.
    pub fn body_silhouette(&self) -> BodySilhouette {
        let pct = if self.hp_max > 0.0 {
            (self.hp / self.hp_max).clamp(0.0, 1.0)
        } else {
            0.0
        };
        BodySilhouette {
            head_hp_pct: pct,
            torso_hp_pct: pct,
            arm_left_hp_pct: pct,
            arm_right_hp_pct: pct,
            leg_left_hp_pct: pct,
            leg_right_hp_pct: pct,
            placeholder: true,
        }
    }

    /// Hash bytes for the M1 deterministic checksum extension. Layout-stable; future
    /// milestones append fields without bumping the schema. Field encodings are picked
    /// to round-trip the full source domain — the inventory slot writes its full `u32`
    /// (`ItemSlot.0.to_le_bytes()`) so growing the inventory beyond 255 slots in a
    /// future milestone cannot silently collide divergent states into the same hash.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(96);
        out.extend_from_slice(&self.id.0.to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.position.x).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.position.y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.velocity.x).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.velocity.y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.aim.x).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.aim.y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.hp).to_le_bytes());
        out.push(self.status as u8);
        out.push(u8::from(self.on_ground));
        out.extend_from_slice(&self.inventory.selected.0.to_le_bytes());
        out
    }
}

/// Quantize an `f32` to a deterministic `i32` representation for cross-platform checksum
/// stability. Per-pixel resolution is plenty for the M1 actor; finer scales can append.
pub(crate) fn quantize_f32(value: f32) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    (value * 1024.0).round() as i32
}

/// The set of [`ActorState`]s in a scenario, plus the player actor id (if any).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorWorld {
    pub actors: BTreeMap<ActorId, ActorState>,
    pub player: Option<ActorId>,
    /// Y coordinate of the world floor (sand-table simplification for M1 — the M2 chunked
    /// terrain replaces this).
    pub floor_y: f32,
    /// Gravity in world units / s² (negative = pulls toward floor). Defaults to scenario's
    /// `gravity`. Sim systems apply this each tick.
    pub gravity: f32,
}

impl ActorWorld {
    pub fn new(floor_y: f32, gravity: f32) -> Self {
        Self {
            actors: BTreeMap::new(),
            player: None,
            floor_y,
            gravity,
        }
    }

    pub fn insert(&mut self, actor: ActorState) {
        if actor.controllable && self.player.is_none() {
            self.player = Some(actor.id);
        }
        self.actors.insert(actor.id, actor);
    }

    pub fn player_actor(&self) -> Option<&ActorState> {
        self.player.and_then(|id| self.actors.get(&id))
    }

    pub fn player_actor_mut(&mut self) -> Option<&mut ActorState> {
        let id = self.player?;
        self.actors.get_mut(&id)
    }

    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.actors.len() * 96 + 16);
        out.extend_from_slice(&quantize_f32(self.floor_y).to_le_bytes());
        out.extend_from_slice(&quantize_f32(self.gravity).to_le_bytes());
        for (_, actor) in &self.actors {
            out.extend_from_slice(&actor.checksum_bytes());
        }
        out
    }
}

/// M4A body silhouette projection. Per-zone hp percentages clamped to `[0, 1]`.
/// `placeholder = true` until M5 lands the real body graph; HUD + AI consumers
/// must treat the layout as stable but the per-zone values as derived (not
/// individually targetable yet).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BodySilhouette {
    pub head_hp_pct: f32,
    pub torso_hp_pct: f32,
    pub arm_left_hp_pct: f32,
    pub arm_right_hp_pct: f32,
    pub leg_left_hp_pct: f32,
    pub leg_right_hp_pct: f32,
    pub placeholder: bool,
}

impl Default for BodySilhouette {
    fn default() -> Self {
        Self {
            head_hp_pct: 1.0,
            torso_hp_pct: 1.0,
            arm_left_hp_pct: 1.0,
            arm_right_hp_pct: 1.0,
            leg_left_hp_pct: 1.0,
            leg_right_hp_pct: 1.0,
            placeholder: true,
        }
    }
}

/// M4A module strip placeholder. M5's chassis grammar replaces this with real
/// per-module state (see [[spec/chassis-armor-mechs-and-origins]]); M4A ships
/// the surface so HUD + `cfctl observe` consumers + accessibility tooling can
/// rely on the contract early.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleStrip {
    /// Module slots, each with a stable id + textual state. Empty when no
    /// chassis is bound. M4A populates `weapon_mount` from the selected
    /// rifle's status (READY / RELOADING / EMPTY / NO RIFLE) and stubs
    /// `jet`, `shield`, and `sensor` as `not_present` so consumers can
    /// distinguish "no module" from "module destroyed".
    pub modules: Vec<ModuleState>,
    pub placeholder: bool,
}

impl Default for ModuleStrip {
    fn default() -> Self {
        Self {
            modules: Vec::new(),
            placeholder: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModuleState {
    pub id: String,
    pub label: String,
    /// One of: `nominal`, `degraded`, `warning`, `failed`, `not_present`.
    pub state: String,
    /// One of: `weapon_mount`, `jet`, `shield`, `sensor`, `repair_drone`.
    pub kind: String,
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
    /// M4A: derived stance label (idle/walking/running/airborne/downed/dead).
    pub stance: String,
    /// M4A: per-zone body silhouette projection (placeholder until M5).
    pub body_silhouette: BodySilhouette,
}

impl From<&ActorState> for ActorObservation {
    fn from(actor: &ActorState) -> Self {
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
            stance: actor.stance().as_str().to_string(),
            body_silhouette: actor.body_silhouette(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_thresholds() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        assert_eq!(actor.status, Status::Stable);
        actor.apply_damage(60.0);
        assert_eq!(actor.status, Status::Unstable);
        actor.apply_damage(35.0);
        assert_eq!(actor.status, Status::Downed);
        actor.apply_damage(10.0);
        assert_eq!(actor.status, Status::Dead);
        // Damage after death is a no-op.
        let no_change = actor.apply_damage(10.0);
        assert!(no_change.is_none());
    }

    #[test]
    fn reset_returns_full_health() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::new(10.0, 20.0), 100.0, inv);
        actor.apply_damage(70.0);
        actor.position = Vec2::new(50.0, 50.0);
        actor.reset();
        assert_eq!(actor.position, Vec2::new(10.0, 20.0));
        assert_eq!(actor.status, Status::Stable);
        assert!((actor.hp - actor.hp_max).abs() < f32::EPSILON);
    }

    #[test]
    fn inventory_select_only_advances_when_slot_exists() {
        let mut inv = Inventory::with_rifle("rifle_m1_default");
        assert!(!inv.try_select(ItemSlot(0)));
        assert!(inv.try_select(ItemSlot(1)));
        assert!(!inv.try_select(ItemSlot(99)));
        assert_eq!(inv.selected, ItemSlot(1));
    }

    #[test]
    fn intent_clear_edges_drops_buttons_keeps_axes() {
        let mut intent = ControlIntent::new(ActorId(1), IntentSource::Human);
        intent.move_x = 1.0;
        intent.aim = Vec2::new(0.0, 1.0);
        intent.jump = true;
        intent.fire = true;
        intent.reload = true;
        intent.selected_item = Some(ItemSlot(2));
        intent.reset = true;
        intent.clear_edges();
        assert!((intent.move_x - 1.0).abs() < f32::EPSILON);
        assert_eq!(intent.aim, Vec2::new(0.0, 1.0));
        assert!(!intent.jump);
        assert!(!intent.fire);
        assert!(!intent.reload);
        assert!(intent.selected_item.is_none());
        assert!(!intent.reset);
    }

    #[test]
    fn quantize_handles_nonfinite() {
        assert_eq!(quantize_f32(f32::NAN), 0);
        assert_eq!(quantize_f32(f32::INFINITY), 0);
        assert_eq!(quantize_f32(0.5), 512);
    }

    #[test]
    fn normalize_or_x_rejects_nonfinite_components() {
        // NaN/Inf must NOT pass through `len < 1e-6` (NaN comparisons return false),
        // otherwise the division produces poison values that propagate to muzzle origin,
        // projectile velocity, and recoil. Defense-in-depth fallback to (1, 0).
        for (x, y) in [
            (f32::NAN, 0.0),
            (0.0, f32::NAN),
            (f32::NAN, f32::NAN),
            (f32::INFINITY, 0.0),
            (0.0, f32::NEG_INFINITY),
            (f32::INFINITY, f32::INFINITY),
        ] {
            let n = Vec2::new(x, y).normalize_or_x();
            assert_eq!(n, Vec2::new(1.0, 0.0), "non-finite ({x}, {y}) must normalize to (1, 0)");
        }
        // Finite zero stays at (1, 0).
        assert_eq!(Vec2::new(0.0, 0.0).normalize_or_x(), Vec2::new(1.0, 0.0));
        // Finite unit vectors normalize correctly.
        let n = Vec2::new(3.0, 4.0).normalize_or_x();
        assert!((n.x - 0.6).abs() < 1e-6);
        assert!((n.y - 0.8).abs() < 1e-6);
    }

    #[test]
    fn actor_world_inserts_player_id_once() {
        let mut world = ActorWorld::new(0.0, -980.0);
        let inv = Inventory::with_rifle("rifle_m1_default");
        world.insert(ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv.clone()));
        let mut second = ActorState::player(ActorId(2), "blue", Vec2::new(5.0, 0.0), 100.0, inv);
        second.controllable = true;
        world.insert(second);
        assert_eq!(world.player, Some(ActorId(1)), "first controllable actor wins");
    }

    #[test]
    fn checksum_bytes_are_layout_stable() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let actor = ActorState::player(ActorId(7), "blue", Vec2::new(1.0, 2.0), 100.0, inv);
        let bytes = actor.checksum_bytes();
        // 8 (id u64) + 4*7 (position.x/y, velocity.x/y, aim.x/y, hp as i32) + 1 (status u8)
        // + 1 (on_ground u8) + 4 (selected slot u32) = 42 bytes.
        assert_eq!(bytes.len(), 42);
    }

    #[test]
    fn stance_derives_idle_when_grounded_and_still() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.on_ground = true;
        actor.velocity = Vec2::new(0.0, 0.0);
        assert_eq!(actor.stance(), Stance::Idle);
    }

    #[test]
    fn stance_derives_walking_running_airborne() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.on_ground = true;
        actor.velocity = Vec2::new(20.0, 0.0);
        assert_eq!(actor.stance(), Stance::Walking);
        actor.velocity = Vec2::new(80.0, 0.0);
        assert_eq!(actor.stance(), Stance::Running);
        actor.on_ground = false;
        actor.velocity = Vec2::new(20.0, 100.0);
        assert_eq!(actor.stance(), Stance::Airborne);
    }

    #[test]
    fn stance_derives_downed_and_dead_from_status() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.apply_damage(95.0);
        assert!(matches!(actor.stance(), Stance::Downed | Stance::Dead));
        actor.apply_damage(100.0);
        assert_eq!(actor.stance(), Stance::Dead);
    }

    #[test]
    fn body_silhouette_clamps_hp_to_unit_range() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.hp = 60.0;
        let s = actor.body_silhouette();
        assert!((s.head_hp_pct - 0.6).abs() < 1e-6);
        assert!(s.placeholder);
        actor.hp = -50.0;
        let s = actor.body_silhouette();
        assert!(s.head_hp_pct >= 0.0);
        actor.hp = 200.0;
        let s = actor.body_silhouette();
        assert!(s.head_hp_pct <= 1.0);
    }

    #[test]
    fn actor_observation_carries_stance_and_silhouette() {
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor.on_ground = true;
        actor.velocity = Vec2::new(80.0, 0.0);
        let obs = ActorObservation::from(&actor);
        assert_eq!(obs.stance, "running");
        assert!(obs.body_silhouette.placeholder);
        assert!((obs.body_silhouette.torso_hp_pct - 1.0).abs() < 1e-6);
    }

    #[test]
    fn checksum_distinguishes_high_inventory_slots() {
        // Regression: inventory.selected used to be cast `as u8`, silently truncating
        // the u32 ItemSlot. Slots 256 and 0 collided into the same checksum byte. Now
        // the full u32 is serialized so growing the inventory beyond 255 slots can't
        // hide divergent state behind identical bytes.
        let inv = Inventory::with_rifle("rifle_m1_default");
        let mut actor_a = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv.clone());
        let mut actor_b = ActorState::player(ActorId(1), "blue", Vec2::ZERO, 100.0, inv);
        actor_a.inventory.selected = ItemSlot(0);
        actor_b.inventory.selected = ItemSlot(256);
        assert_ne!(
            actor_a.checksum_bytes(),
            actor_b.checksum_bytes(),
            "slot 0 and slot 256 must produce different checksum bytes"
        );
    }
}
