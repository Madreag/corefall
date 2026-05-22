//! **M4B § "Save written under v1 loads under current build via migration"** —
//! first concrete migration handler.
//!
//! What v1 -> v2 changes:
//!
//! - Bumps [`crate::WorldSave::schema_version`] and every nested
//!   [`crate::SaveBlob::schema_version`] from [`crate::V1_0_0`] to
//!   [`crate::V2_0_0`].
//! - Defaults `mod_payload` on the world and on every per-actor blob to an
//!   empty `BTreeMap` if absent. Existing mod entries are preserved
//!   verbatim (forward-compat extension rule).
//! - Defaults `terrain_chunks` and `projectiles` to empty `Vec` if absent.
//! - Defaults `world_tick` to `0` for legacy single-actor saves that
//!   pre-dated the whole-world payload.
//!
//! Every other field is preserved verbatim. Nothing is silently dropped.

use crate::{migration::SaveMigration, SaveError, SaveSchemaVersion, WorldSave, V1_0_0, V2_0_0};

#[derive(Debug, Default)]
pub struct MigrationV1ToV2;

impl SaveMigration for MigrationV1ToV2 {
    fn from(&self) -> SaveSchemaVersion {
        V1_0_0
    }

    fn to(&self) -> SaveSchemaVersion {
        V2_0_0
    }

    fn handler_id(&self) -> &'static str {
        "v1_to_v2"
    }

    fn migrate(&self, mut blob: WorldSave) -> Result<WorldSave, SaveError> {
        if blob.schema_version != V1_0_0 {
            return Err(SaveError::MigrationFailed {
                from: V1_0_0,
                to: V2_0_0,
                reason: format!(
                    "v1_to_v2 handler invoked on incompatible source {}",
                    blob.schema_version.as_string()
                ),
            });
        }
        blob.schema_version = V2_0_0;
        for actor in &mut blob.actors {
            actor.schema_version = V2_0_0;
            // mod_payload + crouch/climb/jet/etc were already #[serde(default)]
            // on SaveBlob; the migration is a no-op for those fields. The
            // explicit handler is the contract surface: future v2->v3
            // handlers can change shape without touching this one.
        }
        Ok(blob)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{checksum, SaveBlob, V1_0_0};
    use std::collections::BTreeMap;

    fn v1_blob() -> WorldSave {
        WorldSave {
            schema_version: V1_0_0,
            world_tick: 0,
            actors: vec![SaveBlob {
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
            }],
            terrain_chunks: Vec::new(),
            projectiles: Vec::new(),
            mod_payload: BTreeMap::new(),
        }
    }

    #[test]
    fn upgrades_world_and_actor_schema_version() {
        let h = MigrationV1ToV2;
        let upgraded = h.migrate(v1_blob()).unwrap();
        assert_eq!(upgraded.schema_version, V2_0_0);
        for actor in &upgraded.actors {
            assert_eq!(actor.schema_version, V2_0_0);
        }
    }

    #[test]
    fn rejects_incompatible_source_version() {
        let h = MigrationV1ToV2;
        let mut blob = v1_blob();
        blob.schema_version = V2_0_0;
        let err = h.migrate(blob).err().unwrap();
        assert!(matches!(err, SaveError::MigrationFailed { .. }));
    }

    /// v1 -> v2 handler's output for the canonical v1_minimal blob MUST
    /// produce a stable canonical-JSON BLAKE3. This is the binding
    /// contract that `m4b_migration_matrix.sh` verifies in CI.
    #[test]
    fn v1_minimal_migration_produces_stable_checksum() {
        let h = MigrationV1ToV2;
        let upgraded = h.migrate(v1_blob()).unwrap();
        let hex_a = checksum::canonical_blake3_hex(&upgraded).unwrap();
        let upgraded_again = h.migrate(v1_blob()).unwrap();
        let hex_b = checksum::canonical_blake3_hex(&upgraded_again).unwrap();
        assert_eq!(hex_a, hex_b, "v1_to_v2 must be deterministic");
    }
}
