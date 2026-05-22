//! **M4B § "Old saves still load"** — the migration registry.
//!
//! Every schema bump ships a paired [`SaveMigration`] handler in the
//! registry. When the loader sees `schema_version < CURRENT_SAVE_SCHEMA_VERSION`,
//! it walks the registry forward through every intermediate version on disk
//! and produces a [`crate::WorldSave`] at the current schema. The registry
//! never skips a step.
//!
//! ## Determinism contract
//!
//! Migration is pure: it consumes one `WorldSave` and produces another. No
//! clock reads, no `rand::*`, no network I/O. The migrated blob's
//! canonical-JSON BLAKE3 MUST match the golden fixture under
//! `game/content/save_corpus/v(N)_minimal.cfsave` for the matching target
//! version (M4B's `m4b_migration_matrix.sh` CI gate enforces this).
//!
//! ## "Mod-extending fields survive migration"
//!
//! Every handler MUST round-trip [`crate::SaveBlob::mod_payload`] and
//! [`crate::WorldSave::mod_payload`] verbatim. Handlers MAY add their own
//! defaults but MUST NOT mutate existing mod entries. The
//! `mod_extension_survives_migration` test in this module pins the
//! invariant.

use crate::{SaveError, SaveSchemaVersion, WorldSave, CURRENT_SAVE_SCHEMA_VERSION};

/// A single forward-direction migration handler. Walks one save from its
/// `from()` version to `to()`. The registry walks the chain by composing
/// handlers in order.
pub trait SaveMigration: Send + Sync + 'static {
    /// The source version this handler accepts.
    fn from(&self) -> SaveSchemaVersion;
    /// The target version this handler produces.
    fn to(&self) -> SaveSchemaVersion;
    /// Short identifier for the audit trail (`system.save_migrated.handler_chain`).
    fn handler_id(&self) -> &'static str;
    /// Apply the migration. Returns the upgraded save on success or a
    /// structured [`SaveError`] on failure.
    fn migrate(&self, blob: WorldSave) -> Result<WorldSave, SaveError>;
}

/// Outcome of a successful migration walk.
#[derive(Debug, Clone)]
pub struct MigrationOutcome {
    pub from: SaveSchemaVersion,
    pub to: SaveSchemaVersion,
    pub handler_chain: Vec<&'static str>,
    pub blob: WorldSave,
}

/// The ordered registry of every migration handler shipped by this build.
/// Ordered by `from()` ascending; every successor's `from()` MUST equal the
/// previous handler's `to()`. The registry is constructed once per call so
/// callers can inject a mock registry in tests.
///
/// Spec § Notes calls out this surface as `cf_save::migration::REGISTRY`.
/// Trait objects can't be `const` in Rust, so the registry is exposed as a
/// function; consumers MUST call [`registry`] (or the camel-case
/// [`REGISTRY`] alias) rather than instantiate handlers directly.
pub fn registry() -> Vec<Box<dyn SaveMigration>> {
    vec![Box::new(crate::migration_v1_to_v2::MigrationV1ToV2)]
}

/// Spec-literal alias of [`registry`]. Matches the canonical name from
/// `specs/done/M4B.md` (`cf_save::migration::REGISTRY`).
#[allow(non_snake_case)]
pub fn REGISTRY() -> Vec<Box<dyn SaveMigration>> {
    registry()
}

/// build-time panic". Rust traits can't enforce the contiguity at the type
/// system level (it depends on runtime construction order); this function
/// is the next best thing — a `#[test]` calls it to assert the chain is
/// gapless at compile-test time. Returns Err with a structured reason on
/// gap; cf-save's CI test suite asserts Ok.
pub fn assert_registry_chain_is_contiguous(reg: &[Box<dyn SaveMigration>]) -> Result<(), String> {
    if reg.is_empty() {
        return Ok(());
    }
    for (i, handler) in reg.iter().enumerate().skip(1) {
        let prev = &reg[i - 1];
        if handler.from() != prev.to() {
            return Err(format!(
                "registry chain gap: handler[{}].from = {} but handler[{}].to = {}",
                i,
                handler.from().as_string(),
                i - 1,
                prev.to().as_string()
            ));
        }
        if !handler.to().newer_than(handler.from()) {
            return Err(format!(
                "registry handler[{}] is non-forward: from = {}, to = {}",
                i,
                handler.from().as_string(),
                handler.to().as_string()
            ));
        }
    }
    // The first handler MUST start from a known shipped version (v1 at M4B).
    if reg[0].from() != crate::V1_0_0 {
        return Err(format!(
            "registry chain must start at v1.0.0; first handler.from = {}",
            reg[0].from().as_string()
        ));
    }
    // The last handler MUST end at the current build's schema (no gap to current).
    let last = reg.last().expect("non-empty checked above");
    if last.to() != CURRENT_SAVE_SCHEMA_VERSION {
        return Err(format!(
            "registry chain must terminate at CURRENT_SAVE_SCHEMA_VERSION = {}; last handler.to = {}",
            CURRENT_SAVE_SCHEMA_VERSION.as_string(),
            last.to().as_string()
        ));
    }
    Ok(())
}

/// Walk the registry forward from `blob.schema_version` to `target`,
/// applying every handler in sequence. Returns the [`MigrationOutcome`]
/// (including the ordered list of handler ids the loader walked through).
///
/// - If `blob.schema_version == target`, returns the blob unchanged with an
///   empty handler chain.
/// - If `blob.schema_version > target`, returns
///   [`SaveError::UnsupportedFutureVersion`].
/// - If the registry is missing an intermediate handler between
///   `blob.schema_version` and `target`, returns
///   [`SaveError::MigrationFailed`] with a structured reason (the gap).
pub fn migrate(blob: WorldSave, target: SaveSchemaVersion) -> Result<MigrationOutcome, SaveError> {
    migrate_with_registry(blob, target, registry())
}

/// Walk the supplied (typically `registry()`) migration chain forward.
#[allow(clippy::needless_pass_by_value)]
pub fn migrate_with_registry(
    mut blob: WorldSave,
    target: SaveSchemaVersion,
    registry: Vec<Box<dyn SaveMigration>>,
) -> Result<MigrationOutcome, SaveError> {
    let from = blob.schema_version;
    if from.newer_than(target) {
        return Err(SaveError::UnsupportedFutureVersion {
            found: from,
            max_supported: target,
        });
    }
    let mut handler_chain = Vec::new();
    while blob.schema_version != target {
        let current = blob.schema_version;
        let handler = registry
            .iter()
            .find(|h| h.from() == current)
            .ok_or_else(|| SaveError::MigrationFailed {
                from,
                to: target,
                reason: format!(
                    "no migration handler registered from {} (registry chain stopped before target {})",
                    current.as_string(),
                    target.as_string()
                ),
            })?;
        let next = handler.to();
        if !next.newer_than(current) {
            return Err(SaveError::MigrationFailed {
                from,
                to: target,
                reason: format!(
                    "registry has non-forward handler {}: from={} to={}",
                    handler.handler_id(),
                    current.as_string(),
                    next.as_string()
                ),
            });
        }
        handler_chain.push(handler.handler_id());
        blob = handler.migrate(blob)?;
        // Defensive: ensure the handler honored its declared target.
        if blob.schema_version != next {
            return Err(SaveError::MigrationFailed {
                from,
                to: target,
                reason: format!(
                    "handler {} promised target {} but produced {}",
                    handler.handler_id(),
                    next.as_string(),
                    blob.schema_version.as_string()
                ),
            });
        }
    }
    Ok(MigrationOutcome {
        from,
        to: target,
        handler_chain,
        blob,
    })
}

/// Convenience wrapper that walks straight to [`CURRENT_SAVE_SCHEMA_VERSION`].
pub fn migrate_to_current(blob: WorldSave) -> Result<MigrationOutcome, SaveError> {
    migrate(blob, CURRENT_SAVE_SCHEMA_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SaveBlob, V1_0_0, V2_0_0};
    use std::collections::BTreeMap;

    fn v1_minimal_blob() -> SaveBlob {
        SaveBlob {
            schema_version: V1_0_0,
            actor_id: 1,
            team: "blue".to_string(),
            origin_id: "human".to_string(),
            position: [0.0, 0.0],
            velocity: [0.0, 0.0],
            aim: [1.0, 0.0],
            hp: 100.0,
            hp_max: 100.0,
            on_ground: true,
            status: "stable".to_string(),
            selected_slot: 0,
            rifle_preset: None,
            rifle_ammo: None,
            rifle_reload_remaining_ticks: None,
            chassis: None,
            gear_dropped_by_limb_loss: false,
            chassis_detached: false,
            afflictions: vec![],
            crouch_active: false,
            climb_active: false,
            jet_active: false,
            mod_payload: BTreeMap::new(),
        }
    }

    /// CURRENT_SAVE_SCHEMA_VERSION. This is the build-time equivalent of
    /// the "missing intermediate handler is a panic" guarantee.
    #[test]
    fn registry_chain_is_contiguous_from_v1_to_current() {
        let reg = registry();
        match assert_registry_chain_is_contiguous(&reg) {
            Ok(()) => {}
            Err(reason) => panic!("registry chain contiguity violated: {reason}"),
        }
        // Bonus: every handler in the registry must have a unique
        // handler_id (so the audit trail in save_migrated.handler_chain is
        // unambiguous when multiple handlers fire).
        let mut ids: Vec<&'static str> = reg.iter().map(|h| h.handler_id()).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(n, ids.len(), "every handler_id in the registry must be unique");
    }

    /// Spec calls out `cf_save::migration::REGISTRY` as the canonical name.
    /// The PascalCase alias must produce an equivalent chain to `registry()`.
    #[test]
    #[allow(non_snake_case)]
    fn pascal_case_REGISTRY_alias_matches_registry() {
        let a = registry();
        #[allow(non_snake_case)]
        let b = REGISTRY();
        assert_eq!(a.len(), b.len());
        for (h1, h2) in a.iter().zip(b.iter()) {
            assert_eq!(h1.handler_id(), h2.handler_id());
            assert_eq!(h1.from(), h2.from());
            assert_eq!(h1.to(), h2.to());
        }
    }

    #[test]
    fn migrate_v1_to_v2_walks_one_handler() {
        let world = WorldSave {
            schema_version: V1_0_0,
            world_tick: 0,
            actors: vec![v1_minimal_blob()],
            terrain_chunks: Vec::new(),
            projectiles: Vec::new(),
            mod_payload: BTreeMap::new(),
        };
        let outcome = migrate(world, V2_0_0).expect("v1->v2 migrates");
        assert_eq!(outcome.from, V1_0_0);
        assert_eq!(outcome.to, V2_0_0);
        assert_eq!(outcome.handler_chain, vec!["v1_to_v2"]);
        assert_eq!(outcome.blob.schema_version, V2_0_0);
        assert_eq!(outcome.blob.actors.len(), 1);
        assert_eq!(outcome.blob.actors[0].schema_version, V2_0_0);
    }

    #[test]
    fn migrate_noop_returns_empty_handler_chain_when_already_at_target() {
        let world = WorldSave::empty(0);
        let outcome = migrate(world.clone(), CURRENT_SAVE_SCHEMA_VERSION).unwrap();
        assert!(outcome.handler_chain.is_empty());
        assert_eq!(outcome.blob, world);
    }

    #[test]
    fn migrate_rejects_future_version_with_unsupported_error() {
        let mut world = WorldSave::empty(0);
        world.schema_version = SaveSchemaVersion::new(99, 0, 0);
        let err = migrate(world, V2_0_0).err().unwrap();
        assert!(matches!(err, SaveError::UnsupportedFutureVersion { .. }));
    }

    /// variant a handler MUST return when its input has no explicit
    /// `defaults_for_missing` rule for a required field. Future schema
    /// bumps may exercise this; the current v1→v2 handler has no
    /// required-without-default fields. This test exercises the variant
    /// shape so a mistaken removal would break the contract.
    #[test]
    fn missing_required_field_variant_can_be_returned_by_handler() {
        let err = SaveError::MissingRequiredField {
            version: V1_0_0,
            field: "future_required_field".to_string(),
        };
        let msg = format!("{err}");
        assert!(msg.contains("future_required_field"));
        assert!(msg.contains("v1.0.0"));
    }

    /// and per-actor `mod_payload` MUST round-trip verbatim through any
    /// migration handler in the registry.
    #[test]
    fn mod_extension_survives_migration() {
        let mut world = WorldSave {
            schema_version: V1_0_0,
            world_tick: 60,
            actors: vec![{
                let mut b = v1_minimal_blob();
                b.mod_payload
                    .insert("acme_corp.actor".to_string(), serde_json::json!({"buffs": ["hardy"]}));
                b
            }],
            terrain_chunks: Vec::new(),
            projectiles: Vec::new(),
            mod_payload: BTreeMap::new(),
        };
        world
            .mod_payload
            .insert("acme_corp.world".to_string(), serde_json::json!({"weather": "snow"}));

        let outcome = migrate(world, V2_0_0).expect("v1->v2 migrates");
        assert_eq!(
            outcome.blob.mod_payload.get("acme_corp.world").cloned(),
            Some(serde_json::json!({"weather": "snow"}))
        );
        assert_eq!(
            outcome.blob.actors[0].mod_payload.get("acme_corp.actor").cloned(),
            Some(serde_json::json!({"buffs": ["hardy"]}))
        );
    }
}
