//! Mutable sim state owned by the engine across ticks: [`ActorSimState`],
//! [`Projectile`], [`LooseItem`], and the [`RifleStates`] alias.
//!
//! Extracted from [`crate::sim`] for file size; re-exported from `crate::sim`
//! so existing `cf_actor::sim::X` paths continue to work.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use cf_equipment::RifleState;

use crate::{quantize_f32, ActorId, ActorWorld, Vec2};

/// Per-actor rifle state tracked alongside the actor world. Keyed by [`ActorId`]; only
/// actors carrying a rifle in their inventory get an entry.
pub type RifleStates = BTreeMap<ActorId, RifleState>;

/// One projectile in flight.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Projectile {
    pub id: u64,
    pub owner: ActorId,
    pub origin: Vec2,
    pub position: Vec2,
    pub velocity: Vec2,
    pub damage: f32,
    pub remaining_ticks: u32,
    /// **Per-projectile mass** (kg). Sourced from `RifleSpec.bullet_mass_kg`
    /// at spawn; drives the `cf-physics::try_penetrate` impulse formula.
    /// Default 0.05 kg for byte-identical compat with pre-extension
    /// projectiles.
    #[serde(default = "default_projectile_mass_kg")]
    pub mass_kg: f32,
    /// **Per-projectile sharpness** in [0, 1]. Sourced from
    /// `RifleSpec.bullet_sharpness` at spawn; the penetration-formula
    /// multiplier. Default 0.8 for byte-identical compat with the
    /// pre-extension hardcoded value.
    #[serde(default = "default_projectile_sharpness")]
    pub sharpness: f32,
}

fn default_projectile_mass_kg() -> f32 {
    0.05
}

fn default_projectile_sharpness() -> f32 {
    0.8
}

/// **M1 R2 / Gap G1**: a dropped inventory item subject to gravity + ground
/// collision until it settles. Spawned on `actor.inventory_dropped` from the
/// DYING entry path; settles when its velocity magnitude is below the
/// settle threshold AND it has been on the ground for `settle_dwell_ticks`
/// consecutive ticks. The engine emits `actor.inventory_settled` once on
/// the settle tick, parent_event_id = the originating inventory_dropped.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LooseItem {
    pub id: u64,
    pub source_event_id: String,
    pub item_label: String,
    pub position: Vec2,
    pub velocity: Vec2,
    pub half_extents: Vec2,
    pub settled: bool,
    pub on_ground_dwell_ticks: u32,
    /// Set on the tick the engine fires `actor.inventory_settled`; cleared
    /// the next tick so the engine doesn't double-emit.
    pub just_settled_this_tick: bool,
}

/// Entire mutable sim state owned by the engine across ticks.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ActorSimState {
    pub world: ActorWorld,
    pub rifles: RifleStates,
    pub projectiles: Vec<Projectile>,
    next_projectile_id: u64,
    /// **M1 R2 / Gap G1**: loose items spawned by `actor.inventory_dropped`.
    /// The sim applies gravity + ground collision each tick until they settle.
    #[serde(default)]
    pub loose_items: Vec<LooseItem>,
    /// Monotonic id counter for `LooseItem`s; never reused across resets so
    /// the `actor.inventory_settled` event log carries a stable id.
    #[serde(default)]
    next_loose_item_id: u64,
}

impl ActorSimState {
    pub fn new(world: ActorWorld) -> Self {
        Self {
            world,
            rifles: BTreeMap::new(),
            projectiles: Vec::new(),
            next_projectile_id: 0,
            loose_items: Vec::new(),
            next_loose_item_id: 0,
        }
    }

    /// **M1 R2 / Gap G1**: spawn a `LooseItem` from an
    /// `actor.inventory_dropped` outcome. Returns the new item's id.
    pub fn spawn_loose_item(
        &mut self,
        item_label: impl Into<String>,
        position: Vec2,
        velocity: Vec2,
        source_event_id: impl Into<String>,
    ) -> u64 {
        let id = self.next_loose_item_id;
        self.next_loose_item_id = self.next_loose_item_id.wrapping_add(1);
        self.loose_items.push(LooseItem {
            id,
            source_event_id: source_event_id.into(),
            item_label: item_label.into(),
            position,
            velocity,
            half_extents: Vec2::new(4.0, 4.0),
            settled: false,
            on_ground_dwell_ticks: 0,
            just_settled_this_tick: false,
        });
        id
    }

    pub fn ensure_rifle_for(&mut self, actor_id: ActorId, state: RifleState) {
        self.rifles.entry(actor_id).or_insert(state);
    }

    /// Current value of the projectile-id counter. Exposed so callers that rebuild
    /// the sim state (e.g. `scenario.reset`) can carry the counter forward and keep
    /// `projectile_id` globally unique across resets in the (monotonic) event log.
    pub fn next_projectile_id(&self) -> u64 {
        self.next_projectile_id
    }

    /// Override the projectile-id counter. The next allocated projectile id will
    /// be `id`. Used by `scenario.reset` to preserve uniqueness of `projectile_id`
    /// across the reset boundary; the event log's `combat.projectile_*` cause-chain
    /// would otherwise alias pre-reset and post-reset projectiles.
    pub fn set_next_projectile_id(&mut self, id: u64) {
        self.next_projectile_id = id;
    }

    pub(crate) fn allocate_projectile_id(&mut self) -> u64 {
        let id = self.next_projectile_id;
        self.next_projectile_id += 1;
        id
    }

    /// Hash bytes for the deterministic checksum. Layout-stable; future milestones
    /// append projectile + RNG state without changing earlier slots.
    pub fn checksum_bytes(&self) -> Vec<u8> {
        let mut out = self.world.checksum_bytes();
        out.extend_from_slice(&(self.rifles.len() as u64).to_le_bytes());
        for (id, state) in &self.rifles {
            out.extend_from_slice(&id.0.to_le_bytes());
            out.extend_from_slice(&state.ammo_in_mag.to_le_bytes());
            out.extend_from_slice(&state.fire_cooldown_ticks.to_le_bytes());
            out.extend_from_slice(&state.reload_remaining_ticks.to_le_bytes());
        }
        out.extend_from_slice(&(self.projectiles.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.next_projectile_id.to_le_bytes());
        for p in &self.projectiles {
            out.extend_from_slice(&p.id.to_le_bytes());
            out.extend_from_slice(&p.owner.0.to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.position.x).to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.position.y).to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.velocity.x).to_le_bytes());
            out.extend_from_slice(&quantize_f32(p.velocity.y).to_le_bytes());
            out.extend_from_slice(&p.remaining_ticks.to_le_bytes());
        }
        // **M1 R2 / Gap G1**: loose items appended AFTER the projectile
        // section so runs that produced zero loose items remain byte-stable
        // (the length prefix is 0 → no per-item bytes follow).
        out.extend_from_slice(&(self.loose_items.len() as u64).to_le_bytes());
        out.extend_from_slice(&self.next_loose_item_id.to_le_bytes());
        for item in &self.loose_items {
            out.extend_from_slice(&item.id.to_le_bytes());
            out.extend_from_slice(&quantize_f32(item.position.x).to_le_bytes());
            out.extend_from_slice(&quantize_f32(item.position.y).to_le_bytes());
            out.extend_from_slice(&quantize_f32(item.velocity.x).to_le_bytes());
            out.extend_from_slice(&quantize_f32(item.velocity.y).to_le_bytes());
            out.push(u8::from(item.settled));
            out.extend_from_slice(&item.on_ground_dwell_ticks.to_le_bytes());
        }
        out
    }
}
