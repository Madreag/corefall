//! M4 § Stable record_id layer.
//!
//! Per CCCP `MovableMan.cpp:126-143`: `GetMOFromID` can return stale pointers
//! because pooled memory re-allocates at old addresses. cf-replay's
//! `RecordId(u64)` registry is the canonical id layer for actors, items,
//! projectiles, chunks, and every M9 deep-damage entity kind. Lifecycle
//! events (`<kind>.entity_created`, `<kind>.entity_destroyed`) fire when
//! entities are minted or retired so the run bundle carries the full
//! birth/death story for offline review.
//!
//! `RecordId` values are stable for the lifetime of a run — they MUST NOT
//! be re-issued when an entity is retired. The registry tracks the next id
//! per-kind, so id `Actor::42` and `Chunk::42` are distinct.

use std::collections::BTreeMap;

use cf_sim_core::Tick;
use serde::{Deserialize, Serialize};

use crate::Recorder;

/// Stable u64 record id. Use this for every entity referenced by an event —
/// NEVER raw pointers or pooled MOIDs (per CCCP `MovableMan.cpp:126-143`
/// stale-pointer warning).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RecordId(pub u64);

impl RecordId {
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Entity kinds the M4 spec enumerates. M4 locks the taxonomy; producers
/// at M9 / M13 / M17 / M19 / M20 ladder up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EntityKind {
    Actor,
    Item,
    Projectile,
    Chunk,
    HazardCell,
    AfflictionInstance,
    Shield,
    ArmorLayer,
    ArmorItem,
    ArmorDebris,
    Organ,
    Circuit,
    FluidReservoir,
    FluidLeak,
    Atm,
    EnvironmentSignal,
    Module,
}

impl EntityKind {
    /// Lowercase snake_case category name used in lifecycle events.
    pub fn category_name(self) -> &'static str {
        match self {
            EntityKind::Actor => "actor",
            EntityKind::Item => "item",
            EntityKind::Projectile => "projectile",
            EntityKind::Chunk => "chunk",
            EntityKind::HazardCell => "hazard_cell",
            EntityKind::AfflictionInstance => "affliction_instance",
            EntityKind::Shield => "shield",
            EntityKind::ArmorLayer => "armor_layer",
            EntityKind::ArmorItem => "armor_item",
            EntityKind::ArmorDebris => "armor_debris",
            EntityKind::Organ => "organ",
            EntityKind::Circuit => "circuit",
            EntityKind::FluidReservoir => "fluid_reservoir",
            EntityKind::FluidLeak => "fluid_leak",
            EntityKind::Atm => "atm",
            EntityKind::EnvironmentSignal => "environment_signal",
            EntityKind::Module => "module",
        }
    }
}

/// Per-kind id allocator. Each kind has its own monotonically-increasing
/// counter, so `Actor::1` and `Chunk::1` are distinct ids.
#[derive(Debug, Default)]
pub struct RecordIdRegistry {
    next_id_per_kind: BTreeMap<EntityKind, u64>,
}

impl RecordIdRegistry {
    pub fn new() -> Self {
        Self {
            next_id_per_kind: BTreeMap::new(),
        }
    }

    /// Inspect the next id that would be minted for `kind` without
    /// allocating. Returns 1 when the kind has never been allocated.
    pub fn peek_next(&self, kind: EntityKind) -> u64 {
        self.next_id_per_kind.get(&kind).copied().unwrap_or(0) + 1
    }

    /// Mint a new id for this entity kind. Emits a `<kind>.entity_created`
    /// lifecycle event via the supplied recorder. Returns the minted id
    /// AND the event id of the emitted lifecycle event so callers can use
    /// it as a parent_event_id for downstream cause-chain links.
    pub fn allocate(
        &mut self,
        kind: EntityKind,
        recorder: &Recorder,
        tick: Tick,
        sim_time_ms: f64,
        parent_event_id: Option<String>,
    ) -> (RecordId, String) {
        let counter = self.next_id_per_kind.entry(kind).or_insert(0);
        *counter += 1;
        let id = RecordId(*counter);
        let category = kind.category_name();
        let payload = serde_json::json!({
            "record_id": id.raw(),
            "kind": category,
            "tick": tick.0,
        });
        let event_id = recorder.record(tick, sim_time_ms, category, "entity_created", payload, parent_event_id);
        (id, event_id)
    }

    /// Retire an id. Emits a `<kind>.entity_destroyed` lifecycle event.
    /// `cause_event_id` becomes the lifecycle event's parent_event_id so the
    /// M10 cause-chain walker can trace the destruction back to its trigger
    /// (e.g. a `combat.projectile_hit_mo`). Returns the lifecycle event id.
    pub fn retire(
        &mut self,
        kind: EntityKind,
        id: RecordId,
        recorder: &Recorder,
        tick: Tick,
        sim_time_ms: f64,
        cause_event_id: Option<String>,
    ) -> String {
        let category = kind.category_name();
        let payload = serde_json::json!({
            "record_id": id.raw(),
            "kind": category,
            "tick": tick.0,
        });
        recorder.record(tick, sim_time_ms, category, "entity_destroyed", payload, cause_event_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_independent_per_kind() {
        let recorder = Recorder::new("m4_test_record_id".to_string());
        let mut registry = RecordIdRegistry::new();

        let (a1, _) = registry.allocate(EntityKind::Actor, &recorder, Tick(0), 0.0, None);
        let (a2, _) = registry.allocate(EntityKind::Actor, &recorder, Tick(0), 0.0, None);
        let (a3, _) = registry.allocate(EntityKind::Actor, &recorder, Tick(0), 0.0, None);

        let (c1, _) = registry.allocate(EntityKind::Chunk, &recorder, Tick(0), 0.0, None);
        let (c2, _) = registry.allocate(EntityKind::Chunk, &recorder, Tick(0), 0.0, None);

        assert_eq!(a1, RecordId(1));
        assert_eq!(a2, RecordId(2));
        assert_eq!(a3, RecordId(3));
        assert_eq!(c1, RecordId(1));
        assert_eq!(c2, RecordId(2));
        assert_eq!(registry.peek_next(EntityKind::Actor), 4);
        assert_eq!(registry.peek_next(EntityKind::Chunk), 3);
        assert_eq!(registry.peek_next(EntityKind::Item), 1);
    }

    #[test]
    fn allocate_emits_entity_created_event() {
        let recorder = Recorder::new("m4_test_record_id_emit".to_string());
        let mut registry = RecordIdRegistry::new();
        let (id, event_id) = registry.allocate(EntityKind::HazardCell, &recorder, Tick(7), 116.6, None);
        assert_eq!(id, RecordId(1));
        let events = recorder.snapshot_events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_id, event_id);
        assert_eq!(events[0].category, "hazard_cell");
        assert_eq!(events[0].event_type, "entity_created");
        assert_eq!(events[0].payload["record_id"], serde_json::json!(1));
        assert_eq!(events[0].payload["kind"], serde_json::json!("hazard_cell"));
        assert_eq!(events[0].payload["tick"], serde_json::json!(7));
    }

    #[test]
    fn retire_emits_entity_destroyed_event_with_cause_parent() {
        let recorder = Recorder::new("m4_test_record_id_retire".to_string());
        let mut registry = RecordIdRegistry::new();
        let (id, created_event_id) = registry.allocate(EntityKind::Organ, &recorder, Tick(1), 16.6, None);
        let destroyed_event_id = registry.retire(
            EntityKind::Organ,
            id,
            &recorder,
            Tick(10),
            166.6,
            Some(created_event_id.clone()),
        );
        let events = recorder.snapshot_events();
        assert_eq!(events.len(), 2);
        let destroyed = events.iter().find(|e| e.event_id == destroyed_event_id).unwrap();
        assert_eq!(destroyed.category, "organ");
        assert_eq!(destroyed.event_type, "entity_destroyed");
        assert_eq!(destroyed.parent_event_id, Some(created_event_id));
        assert_eq!(destroyed.payload["record_id"], serde_json::json!(1));
    }

    #[test]
    fn category_name_covers_every_entity_kind() {
        let kinds = [
            EntityKind::Actor,
            EntityKind::Item,
            EntityKind::Projectile,
            EntityKind::Chunk,
            EntityKind::HazardCell,
            EntityKind::AfflictionInstance,
            EntityKind::Shield,
            EntityKind::ArmorLayer,
            EntityKind::ArmorItem,
            EntityKind::ArmorDebris,
            EntityKind::Organ,
            EntityKind::Circuit,
            EntityKind::FluidReservoir,
            EntityKind::FluidLeak,
            EntityKind::Atm,
            EntityKind::EnvironmentSignal,
            EntityKind::Module,
        ];
        let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
        for k in kinds {
            let n = k.category_name();
            assert!(!n.is_empty(), "category_name() for {k:?} must be non-empty");
            assert!(seen.insert(n), "duplicate category_name {n} for {k:?}");
        }
    }
}
