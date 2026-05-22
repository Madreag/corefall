//! [`Loadout`] + [`LoadoutFile`] + the M13 JSON loader for
//! `content/equipment/loadouts/*.json`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// A loadout is a named set of role records (e.g., "infantry default" = rifle +
/// medkit). M5 ships LOAD-A (Loadout A) fixture stubs; **M13** ships the
/// data-driven loader (`load_loadout_from_json` / `load_loadouts_from_dir`)
/// so `content/equipment/loadouts/*.json` is the runtime source of truth.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loadout {
    pub id: String,
    pub display_name: String,
    /// Ordered list of role-record ids. The first entry is treated as the
    /// primary weapon by AI doctrine.
    pub role_ids: Vec<String>,
    pub provenance: String,
}

/// shape is intentionally permissive: `schema_version` is required so
/// downstream migrations can step the field set forward; `description` is
/// optional flavour text consumed by mod-tools and stripped by the runtime
/// loader. All canonical Loadout fields round-trip 1:1.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoadoutFile {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub role_ids: Vec<String>,
    pub provenance: String,
    /// Optional human-readable summary. Ignored by the runtime loader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl LoadoutFile {
    /// Convert to the runtime [`Loadout`] (drops `description` + `schema_version`).
    pub fn into_loadout(self) -> Loadout {
        Loadout {
            id: self.id,
            display_name: self.display_name,
            role_ids: self.role_ids,
            provenance: self.provenance,
        }
    }
}

/// Bumped only on breaking field changes; additive fields (like `description`)
/// keep the version stable.
pub const LOADOUT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug)]
pub enum LoadoutLoadError {
    /// `serde_json` deserialization failed.
    Parse(String),
    /// The on-disk `schema_version` does not match [`LOADOUT_SCHEMA_VERSION`].
    SchemaVersionMismatch { expected: u32, actual: u32 },
    /// The on-disk `id` does not match the registry slot the caller asked for.
    IdMismatch { expected: String, actual: String },
    /// `role_ids` was empty — every loadout must reference at least one role.
    EmptyRoleIds,
    /// A `role_ids` entry does not resolve via [`crate::role_record`].
    UnknownRoleId(String),
}

impl std::fmt::Display for LoadoutLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadoutLoadError::Parse(e) => write!(f, "loadout json parse failed: {e}"),
            LoadoutLoadError::SchemaVersionMismatch { expected, actual } => {
                write!(f, "loadout schema_version mismatch: expected {expected}, got {actual}")
            }
            LoadoutLoadError::IdMismatch { expected, actual } => {
                write!(f, "loadout id mismatch: expected {expected}, got {actual}")
            }
            LoadoutLoadError::EmptyRoleIds => write!(f, "loadout role_ids must not be empty"),
            LoadoutLoadError::UnknownRoleId(id) => write!(f, "loadout references unknown role id `{id}`"),
        }
    }
}

impl std::error::Error for LoadoutLoadError {}

/// version + role-id references against [`crate::role_record`]. The `expected_id`
/// argument lets the caller assert filename↔id parity at load time; pass
/// `None` for free-form parsing.
pub fn load_loadout_from_json(
    json: &str,
    expected_id: Option<&str>,
) -> Result<Loadout, LoadoutLoadError> {
    let file: LoadoutFile =
        serde_json::from_str(json).map_err(|e| LoadoutLoadError::Parse(e.to_string()))?;
    if file.schema_version != LOADOUT_SCHEMA_VERSION {
        return Err(LoadoutLoadError::SchemaVersionMismatch {
            expected: LOADOUT_SCHEMA_VERSION,
            actual: file.schema_version,
        });
    }
    if let Some(id) = expected_id {
        if file.id != id {
            return Err(LoadoutLoadError::IdMismatch {
                expected: id.to_string(),
                actual: file.id.clone(),
            });
        }
    }
    if file.role_ids.is_empty() {
        return Err(LoadoutLoadError::EmptyRoleIds);
    }
    for role_id in &file.role_ids {
        if crate::presets::role_record(role_id).is_none() {
            return Err(LoadoutLoadError::UnknownRoleId(role_id.clone()));
        }
    }
    Ok(file.into_loadout())
}

/// loadout id. Files whose `schema_version`, `role_ids`, or id don't validate
/// return an error so cf-mod / scenario.load can surface a typed reason.
pub fn load_loadouts_from_dir(
    dir: &std::path::Path,
) -> Result<BTreeMap<String, Loadout>, LoadoutLoadError> {
    let mut out = BTreeMap::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(it) => it,
        Err(e) => {
            return Err(LoadoutLoadError::Parse(format!(
                "read_dir {} failed: {e}",
                dir.display()
            )))
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| LoadoutLoadError::Parse(format!("read {} failed: {e}", path.display())))?;
        let loadout = load_loadout_from_json(&raw, Some(&stem))?;
        out.insert(loadout.id.clone(), loadout);
    }
    Ok(out)
}
