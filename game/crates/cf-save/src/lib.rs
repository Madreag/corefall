//! M5 + M4B: save/load roundtrip for chassis + equipment + actor state, now
//! layered under a semver schema (`SaveSchemaVersion`), an explicit migration
//! registry, delta-encoded incremental snapshots, a BLAKE3 ledger chain for
//! tamper-evident replays, and a canonical-JSON BLAKE3 checksum.
//!
//! M5 declared the actor + chassis + equipment payload. M4B binds that
//! payload into a versioned + delta-compressed + tamper-evident persistence
//! layer:
//!
//! - [`SaveSchemaVersion`] is a `{major, minor, patch}` triple serialized as a
//!   3-element JSON array so it round-trips through canonical-JSON BLAKE3
//!   cleanly (no whitespace / key-ordering ambiguity).
//! - [`SaveBlob`] keeps the M5 per-actor contract; its `schema_version` field
//!   is now a [`SaveSchemaVersion`]. v1 saves on disk (numeric
//!   `schema_version`) still deserialize via the compat layer.
//! - [`WorldSave`] is the M4B whole-world container: actors + terrain chunks
//!   + projectiles + opaque `mod_payload`. It is what `.cfsave` files store.
//! - [`migration`] holds the registry of `SaveMigration` handlers. The
//!   registry walks forward through every version on disk, never skips a
//!   step, and never silently drops fields.
//! - [`delta`] holds the baseline + delta-chain encoder and the per-actor /
//!   per-chunk / per-projectile JSON-patch differs.
//! - [`ledger_chain`] holds the BLAKE3-keyed `prev_event_hash` chain encoder
//!   + verifier (tournament-mode tamper evidence).
//! - [`checksum`] holds the canonical-JSON BLAKE3 used as the `.cfsave`
//!   integrity check.
//! - [`quicksave`] wires F5 / F9 fast-path serialization (the cf-app
//!   integration adapter; the hotkey loop lives in cf-app).
//!
//! Determinism contract: every public function is pure; no clock reads; no
//! `rand::thread_rng()`. Checksums + chain hashes are computed over the
//! canonical JSON representation so platform float-representation
//! differences cannot leak.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use cf_chassis::ChassisState;

pub mod checksum;
pub mod delta;
pub mod delta_actor;
pub mod delta_chunk;
pub mod delta_projectile;
pub mod ledger_chain;
pub mod migration;
pub mod migration_v1_to_v2;
pub mod quicksave;

/// **M4B**: explicit semver `{major, minor, patch}` for the `.cfsave` schema.
///
/// Serialized as a 3-element JSON array `[major, minor, patch]` so canonical
/// JSON BLAKE3 is unambiguous (whitespace + key ordering of an object body
/// would otherwise leak into the checksum). A custom `Deserialize`
/// implementation accepts BOTH the array form AND a bare integer (for
/// backwards-compat with v1 saves that wrote `"schema_version": 1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SaveSchemaVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SaveSchemaVersion {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self { major, minor, patch }
    }

    pub fn as_tuple(self) -> (u16, u16, u16) {
        (self.major, self.minor, self.patch)
    }

    pub fn as_string(self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Returns true when `self` is strictly newer (lex order on tuple) than
    /// the supplied maximum.
    pub fn newer_than(self, other: Self) -> bool {
        self.as_tuple() > other.as_tuple()
    }
}

/// First shipped schema (M5 + M5W1). All v1 saves on disk parse into this.
pub const V1_0_0: SaveSchemaVersion = SaveSchemaVersion::new(1, 0, 0);

/// Current schema shipped by this build. Bumped by M4B from v1.0.0 to v2.0.0
/// when the M4B migration registry + delta + chain layers landed.
pub const V2_0_0: SaveSchemaVersion = SaveSchemaVersion::new(2, 0, 0);

/// The schema version this build writes by default.
pub const CURRENT_SAVE_SCHEMA_VERSION: SaveSchemaVersion = V2_0_0;

/// **Backwards-compat alias** for callers that imported the original M5
/// constant. v1 SaveBlobs on disk carry `schema_version: 1` and now parse
/// into [`V1_0_0`] via the deserializer's compat path. New callers should
/// prefer [`CURRENT_SAVE_SCHEMA_VERSION`].
pub const SAVE_BLOB_VERSION: SaveSchemaVersion = CURRENT_SAVE_SCHEMA_VERSION;

impl Default for SaveSchemaVersion {
    fn default() -> Self {
        CURRENT_SAVE_SCHEMA_VERSION
    }
}

impl Serialize for SaveSchemaVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // 3-element JSON array per the M4B contract: deterministic + minimal.
        use serde::ser::SerializeTuple;
        let mut t = serializer.serialize_tuple(3)?;
        t.serialize_element(&self.major)?;
        t.serialize_element(&self.minor)?;
        t.serialize_element(&self.patch)?;
        t.end()
    }
}

impl<'de> Deserialize<'de> for SaveSchemaVersion {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Accept three shapes:
        //   1) {"major": 2, "minor": 0, "patch": 0}        (canonical object form)
        //   2) [2, 0, 0]                                    (canonical array form)
        //   3) 1                                            (legacy v1 numeric form)
        // The numeric path is what makes pre-M4B `.cfsave` files load cleanly
        // under v2.x. See § "Save written under v1 loads under current build".
        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::Array(arr) => {
                if arr.len() != 3 {
                    return Err(serde::de::Error::custom(format!(
                        "SaveSchemaVersion array form must have 3 elements, got {}",
                        arr.len()
                    )));
                }
                let parse_u16 = |v: &serde_json::Value| -> Result<u16, D::Error> {
                    v.as_u64()
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| serde::de::Error::custom("SaveSchemaVersion element must be a u16 integer"))
                };
                Ok(Self {
                    major: parse_u16(&arr[0])?,
                    minor: parse_u16(&arr[1])?,
                    patch: parse_u16(&arr[2])?,
                })
            }
            serde_json::Value::Object(map) => {
                let parse_field = |k: &str| -> Result<u16, D::Error> {
                    map.get(k)
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|n| u16::try_from(n).ok())
                        .ok_or_else(|| {
                            serde::de::Error::custom(format!(
                                "SaveSchemaVersion object form missing required u16 field '{k}'"
                            ))
                        })
                };
                Ok(Self {
                    major: parse_field("major")?,
                    minor: parse_field("minor")?,
                    patch: parse_field("patch")?,
                })
            }
            serde_json::Value::Number(n) => {
                let major = n
                    .as_u64()
                    .and_then(|v| u16::try_from(v).ok())
                    .ok_or_else(|| serde::de::Error::custom("SaveSchemaVersion numeric form must fit in u16"))?;
                Ok(Self {
                    major,
                    minor: 0,
                    patch: 0,
                })
            }
            other => Err(serde::de::Error::custom(format!(
                "SaveSchemaVersion must be an array, object, or integer; got {other:?}"
            ))),
        }
    }
}

/// One save-blob payload. M5 ships chassis + equipment (rifle preset id +
/// remaining ammo) + actor identity. M4B adds an opaque `mod_payload` so a
/// build without a mod installed round-trips the mod's extension data
/// verbatim through migration (the M4B mod ecosystem promise).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SaveBlob {
    pub schema_version: SaveSchemaVersion,
    pub actor_id: u64,
    pub team: String,
    pub origin_id: String,
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub aim: [f32; 2],
    pub hp: f32,
    pub hp_max: f32,
    pub on_ground: bool,
    pub status: String,
    /// Currently selected inventory slot.
    pub selected_slot: u32,
    /// Rifle preset id mounted at the rifle socket (if any).
    pub rifle_preset: Option<String>,
    /// Remaining ammo in the mounted rifle (if any).
    pub rifle_ammo: Option<u32>,
    /// Active reload-ticks (if reloading).
    pub rifle_reload_remaining_ticks: Option<u32>,
    /// Chassis state when one is attached.
    pub chassis: Option<ChassisState>,
    /// M5 W1: flags for gear dropped by limb loss + chassis detached state.
    #[serde(default)]
    pub gear_dropped_by_limb_loss: bool,
    #[serde(default)]
    pub chassis_detached: bool,
    /// M5 W1: active afflictions on this actor (empty until BP4 affliction producers exist).
    #[serde(default)]
    pub afflictions: Vec<String>,
    /// M5 W1: crouch/climb/jet movement-intent flags.
    #[serde(default)]
    pub crouch_active: bool,
    #[serde(default)]
    pub climb_active: bool,
    #[serde(default)]
    pub jet_active: bool,
    /// **M4B § "Mod-extending fields survive migration"**: opaque passthrough
    /// for third-party mods to attach extension data under their own
    /// namespace. Builds without the mod installed MUST round-trip this map
    /// verbatim through migration.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mod_payload: BTreeMap<String, serde_json::Value>,
}

impl SaveBlob {
    /// Compute a deterministic canonical-JSON BLAKE3 checksum. See
    /// [`checksum::canonical_blake3_hex`] for the canonical-JSON contract;
    /// this method is the SaveBlob-facing convenience wrapper.
    pub fn checksum_hex(&self) -> Result<String, SaveError> {
        let json = serde_json::to_string(self).map_err(SaveError::SerializeJson)?;
        Ok(checksum::blake3_hex_of(json.as_bytes()))
    }

    /// Serialize to a pretty JSON string + checksum. Returns the `(json, hex)`
    /// pair so the caller can write the JSON to disk and store the hex in a
    /// sidecar (`save_blob.checksum`).
    pub fn serialize(&self) -> Result<(String, String), SaveError> {
        let json = serde_json::to_string_pretty(self).map_err(SaveError::SerializeJson)?;
        let canonical = serde_json::to_string(self).map_err(SaveError::SerializeJson)?;
        Ok((json, checksum::blake3_hex_of(canonical.as_bytes())))
    }

    /// Deserialize from JSON, optionally verifying the supplied checksum and
    /// asserting `schema_version` is not from a future build.
    pub fn deserialize(json: &str, expected_hex: Option<&str>) -> Result<Self, SaveError> {
        let blob: SaveBlob = serde_json::from_str(json).map_err(SaveError::DeserializeJson)?;
        if blob.schema_version.newer_than(CURRENT_SAVE_SCHEMA_VERSION) {
            return Err(SaveError::UnsupportedFutureVersion {
                found: blob.schema_version,
                max_supported: CURRENT_SAVE_SCHEMA_VERSION,
            });
        }
        if let Some(expected) = expected_hex {
            let actual = blob.checksum_hex()?;
            if actual != expected {
                return Err(SaveError::ChecksumMismatch {
                    expected: expected.to_string(),
                    actual,
                });
            }
        }
        Ok(blob)
    }
}

/// **M4B**: per-projectile snapshot. M4B treats projectiles as opaque JSON
/// so mods can add fields. The delta encoder operates on these maps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProjectileSnapshot {
    pub id: u64,
    pub state: serde_json::Value,
}

/// **M4B**: per-terrain-chunk snapshot. M4B treats chunks as opaque JSON
/// (RLE encoding done at the chunk level; M4B only stores the canonical
/// `serde_json::Value` so delta encoding is uniform).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TerrainChunkSnapshot {
    pub chunk_id: String,
    pub state: serde_json::Value,
}

/// **M4B**: whole-world save container. The `.cfsave` file format. Owns
/// the SaveSchemaVersion at the top level + the mod-payload passthrough +
/// the list of per-actor [`SaveBlob`]s + terrain chunks + projectiles.
///
/// The per-actor SaveBlob continues to carry its own `schema_version` field
/// (M5 owns that payload). Both versions advance together via migration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorldSave {
    pub schema_version: SaveSchemaVersion,
    pub world_tick: u64,
    /// Per-actor M5 payload.
    pub actors: Vec<SaveBlob>,
    /// Terrain chunks at save time.
    #[serde(default)]
    pub terrain_chunks: Vec<TerrainChunkSnapshot>,
    /// In-flight projectiles at save time.
    #[serde(default)]
    pub projectiles: Vec<ProjectileSnapshot>,
    /// **M4B § "Mod-extending fields survive migration"**: opaque passthrough
    /// for third-party mod extensions at the whole-world level.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mod_payload: BTreeMap<String, serde_json::Value>,
}

impl WorldSave {
    /// Construct an empty save at the current schema version. Useful as a
    /// baseline for tests + the initial autosave slot.
    pub fn empty(world_tick: u64) -> Self {
        Self {
            schema_version: CURRENT_SAVE_SCHEMA_VERSION,
            world_tick,
            actors: Vec::new(),
            terrain_chunks: Vec::new(),
            projectiles: Vec::new(),
            mod_payload: BTreeMap::new(),
        }
    }

    /// Construct a single-actor save (M5 backwards-compat convenience).
    pub fn single_actor(blob: SaveBlob, world_tick: u64) -> Self {
        Self {
            schema_version: CURRENT_SAVE_SCHEMA_VERSION,
            world_tick,
            actors: vec![blob],
            terrain_chunks: Vec::new(),
            projectiles: Vec::new(),
            mod_payload: BTreeMap::new(),
        }
    }

    /// Canonical-JSON BLAKE3 hex of this save.
    pub fn checksum_hex(&self) -> Result<String, SaveError> {
        let json = serde_json::to_string(self).map_err(SaveError::SerializeJson)?;
        Ok(checksum::blake3_hex_of(json.as_bytes()))
    }

    /// Serialize the save plus its checksum. The save is written pretty (for
    /// human-readable diffs in mod tooling); the checksum is computed over
    /// the canonical (compact) form so platform float-representation cannot
    /// leak.
    pub fn serialize(&self) -> Result<(String, String), SaveError> {
        let pretty = serde_json::to_string_pretty(self).map_err(SaveError::SerializeJson)?;
        let canonical = serde_json::to_string(self).map_err(SaveError::SerializeJson)?;
        Ok((pretty, checksum::blake3_hex_of(canonical.as_bytes())))
    }

    /// Deserialize + verify the integrity contract:
    ///
    /// 1. Returns [`SaveError::ChecksumMismatch`] when `expected_hex` is set
    ///    and the computed checksum disagrees.
    /// 2. Returns [`SaveError::UnsupportedFutureVersion`] when the blob's
    ///    `schema_version` is newer than [`CURRENT_SAVE_SCHEMA_VERSION`].
    ///
    /// Migration is NOT performed here; callers wanting the v1 -> current
    /// upgrade path call [`migration::migrate`].
    pub fn deserialize(json: &str, expected_hex: Option<&str>) -> Result<Self, SaveError> {
        // Parse the raw bytes first; on parse failure we already know the
        // blob is malformed.
        let parsed: WorldSave = serde_json::from_str(json).map_err(SaveError::DeserializeJson)?;
        // Future-version check BEFORE checksum so we return the more
        // specific error to the player.
        if parsed.schema_version.newer_than(CURRENT_SAVE_SCHEMA_VERSION) {
            return Err(SaveError::UnsupportedFutureVersion {
                found: parsed.schema_version,
                max_supported: CURRENT_SAVE_SCHEMA_VERSION,
            });
        }
        if let Some(expected) = expected_hex {
            let actual = parsed.checksum_hex()?;
            if actual != expected {
                return Err(SaveError::ChecksumMismatch {
                    expected: expected.to_string(),
                    actual,
                });
            }
        }
        Ok(parsed)
    }
}

/// **M4B**: structured save errors. These map cleanly to the cf-app modal
/// strings + the cfctl JSON-RPC error responses.
#[derive(Debug, Error)]
pub enum SaveError {
    #[error("serialize save blob to json failed: {0}")]
    SerializeJson(#[source] serde_json::Error),
    #[error("deserialize save blob from json failed: {0}")]
    DeserializeJson(#[source] serde_json::Error),
    /// **M4B § "Corrupted save surfaces a clean error, never panics"**.
    #[error("save blob checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    /// **M4B § "Save from a future version is rejected clearly"**.
    #[error(
        "save written by a newer game version ({}); this build supports up to {}",
        found.as_string(),
        max_supported.as_string()
    )]
    UnsupportedFutureVersion {
        found: SaveSchemaVersion,
        max_supported: SaveSchemaVersion,
    },
    /// **M4B § "Save written under v1 loads under current build via migration"** —
    /// surfaces when the registry walks `from -> to` and a handler
    /// short-circuits with `reason`.
    #[error("migration failed: from={} to={} reason={reason}", from.as_string(), to.as_string())]
    MigrationFailed {
        from: SaveSchemaVersion,
        to: SaveSchemaVersion,
        reason: String,
    },
    /// **M4B § "no payload field is silently dropped"** — surfaces when a
    /// handler discovers an unknown required field on the input that has
    /// no `defaults_for_missing` rule in the migration registry.
    #[error(
        "save blob missing required field '{field}' under schema {} (no migration default)",
        version.as_string()
    )]
    MissingRequiredField {
        version: SaveSchemaVersion,
        field: String,
    },
    /// **M5 backcompat** — kept so existing match arms continue to compile.
    /// Returned only when the per-actor SaveBlob deserializer sees a numeric
    /// `schema_version` that maps to a version this build cannot migrate.
    #[error("save blob schema version mismatch: expected {}, got {}", expected.as_string(), actual.as_string())]
    SchemaVersionMismatch {
        expected: SaveSchemaVersion,
        actual: SaveSchemaVersion,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob_at(version: SaveSchemaVersion, with_chassis: bool) -> SaveBlob {
        let chassis = if with_chassis {
            let spec = cf_chassis::powered_armor_spec();
            let mut state = ChassisState::from_spec(&spec, 60, false);
            let _ = state.apply_zone_damage(cf_chassis::BodyZone::Torso, 40.0, "test");
            let _ = state.recompute_stage();
            Some(state)
        } else {
            None
        };
        SaveBlob {
            schema_version: version,
            actor_id: 1,
            team: "blue".to_string(),
            origin_id: "human".to_string(),
            position: [10.0, 20.0],
            velocity: [0.0, 0.0],
            aim: [1.0, 0.0],
            hp: 75.0,
            hp_max: 100.0,
            on_ground: true,
            status: "stable".to_string(),
            selected_slot: 0,
            rifle_preset: Some(cf_equipment::RIFLE_M1_DEFAULT_ID.to_string()),
            rifle_ammo: Some(20),
            rifle_reload_remaining_ticks: None,
            chassis,
            gear_dropped_by_limb_loss: false,
            chassis_detached: false,
            afflictions: vec![],
            crouch_active: false,
            climb_active: false,
            jet_active: false,
            mod_payload: BTreeMap::new(),
        }
    }

    fn blob(with_chassis: bool) -> SaveBlob {
        blob_at(CURRENT_SAVE_SCHEMA_VERSION, with_chassis)
    }

    #[test]
    fn save_blob_round_trips_without_chassis() {
        let original = blob(false);
        let (json, hex) = original.serialize().unwrap();
        let recovered = SaveBlob::deserialize(&json, Some(&hex)).unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn save_blob_round_trips_with_chassis() {
        let original = blob(true);
        let (json, hex) = original.serialize().unwrap();
        let recovered = SaveBlob::deserialize(&json, Some(&hex)).unwrap();
        assert_eq!(recovered, original);
        assert_eq!(recovered.checksum_hex().unwrap(), hex);
    }

    #[test]
    fn checksum_mismatch_is_detected() {
        let original = blob(true);
        let (json, _hex) = original.serialize().unwrap();
        let err = SaveBlob::deserialize(&json, Some("deadbeef")).err().unwrap();
        assert!(matches!(err, SaveError::ChecksumMismatch { .. }));
    }

    #[test]
    fn future_version_is_rejected_with_unsupported_error() {
        let mut original = blob(false);
        original.schema_version = SaveSchemaVersion::new(99, 0, 0);
        let (json, _hex) = original.serialize().unwrap();
        let err = SaveBlob::deserialize(&json, None).err().unwrap();
        assert!(matches!(err, SaveError::UnsupportedFutureVersion { .. }));
    }

    #[test]
    fn chassis_damage_persists_through_roundtrip() {
        let original = blob(true);
        let (json, hex) = original.serialize().unwrap();
        let recovered = SaveBlob::deserialize(&json, Some(&hex)).unwrap();
        let chassis = recovered.chassis.as_ref().unwrap();
        let torso = chassis.zone(cf_chassis::BodyZone::Torso).unwrap();
        let external = torso
            .layers
            .iter()
            .find(|l| l.kind == cf_chassis::ArmorLayerKind::External)
            .unwrap();
        assert!(external.hp < external.hp_max);
    }

    #[test]
    fn save_blob_round_trips_m13_actor_extension_fields() {
        let mut original = blob(true);
        original.crouch_active = true;
        original.climb_active = false;
        original.jet_active = true;
        original.gear_dropped_by_limb_loss = true;
        original.chassis_detached = true;
        original.afflictions = vec!["bleeding".to_string(), "concussion".to_string()];
        let (json, hex) = original.serialize().unwrap();
        let recovered = SaveBlob::deserialize(&json, Some(&hex)).unwrap();
        assert_eq!(recovered, original);
        assert!(recovered.crouch_active);
        assert!(!recovered.climb_active);
        assert!(recovered.jet_active);
        assert!(recovered.gear_dropped_by_limb_loss);
        assert!(recovered.chassis_detached);
        assert_eq!(recovered.afflictions, vec!["bleeding", "concussion"]);
    }

    /// **M4B** § "Save written under v1 loads under current build via
    /// migration" — the numeric compat path. Old saves wrote
    /// `"schema_version": 1`, and the new SaveSchemaVersion deserializer
    /// MUST parse that as v1.0.0 cleanly (then the migration registry walks
    /// forward).
    #[test]
    fn legacy_numeric_schema_version_parses_as_v1_0_0() {
        let raw = serde_json::json!({
            "schema_version": 1,
            "actor_id": 1u64,
            "team": "blue",
            "origin_id": "human",
            "position": [0.0, 0.0],
            "velocity": [0.0, 0.0],
            "aim": [1.0, 0.0],
            "hp": 100.0,
            "hp_max": 100.0,
            "on_ground": true,
            "status": "stable",
            "selected_slot": 0u32,
            "rifle_preset": null,
            "rifle_ammo": null,
            "rifle_reload_remaining_ticks": null,
            "chassis": null,
        });
        let json = serde_json::to_string(&raw).unwrap();
        let recovered: SaveBlob = serde_json::from_str(&json).expect("legacy numeric form must parse");
        assert_eq!(recovered.schema_version, V1_0_0);
    }

    /// **M4B** § "SaveSchemaVersion is a 3-element JSON array" — round-trip
    /// the canonical array form so the migration registry never has to
    /// guess what shape `schema_version` is on disk.
    #[test]
    fn schema_version_serializes_as_three_element_array() {
        let v = SaveSchemaVersion::new(2, 1, 5);
        let s = serde_json::to_string(&v).unwrap();
        assert_eq!(s, "[2,1,5]");
        let back: SaveSchemaVersion = serde_json::from_str(&s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn schema_version_accepts_object_form_for_human_authoring() {
        let raw = "{\"major\":1,\"minor\":2,\"patch\":3}";
        let v: SaveSchemaVersion = serde_json::from_str(raw).unwrap();
        assert_eq!(v, SaveSchemaVersion::new(1, 2, 3));
    }

    #[test]
    fn worldsave_round_trips_through_serialize_and_deserialize() {
        let original = WorldSave::single_actor(blob(true), 600);
        let (json, hex) = original.serialize().unwrap();
        let recovered = WorldSave::deserialize(&json, Some(&hex)).unwrap();
        assert_eq!(recovered, original);
    }

    #[test]
    fn worldsave_future_version_rejected() {
        let mut original = WorldSave::empty(0);
        original.schema_version = SaveSchemaVersion::new(99, 0, 0);
        let (json, _hex) = original.serialize().unwrap();
        let err = WorldSave::deserialize(&json, None).err().unwrap();
        assert!(matches!(err, SaveError::UnsupportedFutureVersion { .. }));
    }

    #[test]
    fn worldsave_checksum_mismatch_returns_clean_error() {
        let original = WorldSave::single_actor(blob(false), 0);
        let (json, _hex) = original.serialize().unwrap();
        // Flip one byte of the canonical JSON before recomputing checksum.
        let mut tampered = json.clone();
        if let Some(idx) = tampered.find("blue") {
            tampered.replace_range(idx..idx + 4, "RED!");
        }
        // The original valid checksum is what we'll claim — the tampered
        // bytes will not hash to it, so we expect a clean ChecksumMismatch.
        let valid_hex = original.checksum_hex().unwrap();
        let err = WorldSave::deserialize(&tampered, Some(&valid_hex)).err().unwrap();
        assert!(matches!(err, SaveError::ChecksumMismatch { .. }));
    }

    #[test]
    fn worldsave_mod_payload_preserved_verbatim() {
        let mut original = WorldSave::empty(0);
        original
            .mod_payload
            .insert("acme_corp".to_string(), serde_json::json!({"reactor_kw": 850}));
        let (json, hex) = original.serialize().unwrap();
        let recovered = WorldSave::deserialize(&json, Some(&hex)).unwrap();
        assert_eq!(recovered.mod_payload, original.mod_payload);
    }
}
