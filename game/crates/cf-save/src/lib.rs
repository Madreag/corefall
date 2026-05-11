//! M5: save/load roundtrip for chassis + equipment + actor state.
//!
//! The full T-SAVE system (versioned `.cfsave`, multi-slot, autosave, ironman,
//! migration) lands in a later milestone (DR-029). M5 ships **just enough** to
//! validate that the chassis state machine (zones, modules, pilot binding,
//! eject window) round-trips through serde and that a deterministic blake3
//! checksum proves the round-trip is byte-stable. The roadmap done-criterion
//! M5-004 asks for `Save/load checksum` — that's what this crate delivers.
//!
//! Determinism contract: every public function is pure; no clock reads; no
//! `rand::thread_rng()`. The checksum is computed over the canonical JSON
//! representation so platform float-representation differences cannot leak.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::must_use_candidate,
    clippy::doc_markdown
)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

use cf_chassis::ChassisState;

pub const SAVE_BLOB_VERSION: u32 = 1;

/// One save-blob payload. M5 ships chassis + equipment (rifle preset id +
/// remaining ammo) + actor identity. Later milestones (T-SAVE) extend with
/// world state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveBlob {
    pub schema_version: u32,
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
}

impl SaveBlob {
    /// Compute a deterministic checksum over the canonical JSON form. The
    /// checksum is what cf-control verifies on load to detect tampering or
    /// in-flight corruption.
    pub fn checksum_hex(&self) -> Result<String, SaveError> {
        let json = serde_json::to_string(self).map_err(SaveError::SerializeJson)?;
        let hash = blake3::hash(json.as_bytes());
        Ok(hex::encode(hash.as_bytes()))
    }

    /// Serialize to a pretty JSON string + checksum. Returns the `(json, hex)`
    /// pair so the caller can write the JSON to disk and store the hex in a
    /// sidecar (`save_blob.checksum`).
    pub fn serialize(&self) -> Result<(String, String), SaveError> {
        let json = serde_json::to_string_pretty(self).map_err(SaveError::SerializeJson)?;
        let canonical = serde_json::to_string(self).map_err(SaveError::SerializeJson)?;
        let hash = blake3::hash(canonical.as_bytes());
        Ok((json, hex::encode(hash.as_bytes())))
    }

    /// Deserialize from JSON, optionally verifying the supplied checksum.
    pub fn deserialize(json: &str, expected_hex: Option<&str>) -> Result<Self, SaveError> {
        let blob: SaveBlob = serde_json::from_str(json).map_err(SaveError::DeserializeJson)?;
        if blob.schema_version != SAVE_BLOB_VERSION {
            return Err(SaveError::SchemaVersionMismatch {
                expected: SAVE_BLOB_VERSION,
                actual: blob.schema_version,
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

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("serialize save blob to json failed: {0}")]
    SerializeJson(#[source] serde_json::Error),
    #[error("deserialize save blob from json failed: {0}")]
    DeserializeJson(#[source] serde_json::Error),
    #[error("save blob schema version mismatch: expected {expected}, got {actual}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },
    #[error("save blob checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blob(with_chassis: bool) -> SaveBlob {
        let chassis = if with_chassis {
            let spec = cf_chassis::powered_armor_spec();
            let mut state = ChassisState::from_spec(&spec, 60, false);
            // Apply a bit of damage so the round-trip exercises non-default state.
            let _ = state.apply_zone_damage(cf_chassis::BodyZone::Torso, 40.0, "test");
            let _ = state.recompute_stage();
            Some(state)
        } else {
            None
        };
        SaveBlob {
            schema_version: SAVE_BLOB_VERSION,
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
        }
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
        // Checksum is deterministic across serializations.
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
    fn schema_version_mismatch_is_detected() {
        let mut original = blob(false);
        original.schema_version = 99;
        let (json, _hex) = original.serialize().unwrap();
        let err = SaveBlob::deserialize(&json, None).err().unwrap();
        assert!(matches!(err, SaveError::SchemaVersionMismatch { .. }));
    }

    #[test]
    fn chassis_damage_persists_through_roundtrip() {
        let original = blob(true);
        let (json, hex) = original.serialize().unwrap();
        let recovered = SaveBlob::deserialize(&json, Some(&hex)).unwrap();
        let chassis = recovered.chassis.as_ref().unwrap();
        // External torso layer should be partially damaged.
        let torso = chassis.zone(cf_chassis::BodyZone::Torso).unwrap();
        let external = torso
            .layers
            .iter()
            .find(|l| l.kind == cf_chassis::ArmorLayerKind::External)
            .unwrap();
        assert!(external.hp < external.hp_max);
    }
}
