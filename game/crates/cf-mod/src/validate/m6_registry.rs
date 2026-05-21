use std::{fs, path::Path};

use crate::report::ValidationReport;

/// **M6**: shared shape for one entry in a M6 equipment registry RON.
/// Every entry MUST declare a non-empty `id` and a non-empty `kind` (or
/// `class` for weapons). Display name is optional in this minimal contract.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6WeaponEntry {
    pub(crate) id: String,
    pub(crate) class: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6WeaponRegistry {
    pub(crate) schema_version: u32,
    pub(crate) weapons: Vec<M6WeaponEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6GrenadeEntry {
    pub(crate) id: String,
    pub(crate) kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) display_name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) fuse_seconds: Option<f32>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6GrenadeRegistry {
    pub(crate) schema_version: u32,
    pub(crate) grenades: Vec<M6GrenadeEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6MeleeEntry {
    pub(crate) id: String,
    pub(crate) kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6MeleeRegistry {
    pub(crate) schema_version: u32,
    pub(crate) melees: Vec<M6MeleeEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6ToolEntry {
    pub(crate) id: String,
    pub(crate) kind: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) display_name: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct M6ToolRegistry {
    pub(crate) schema_version: u32,
    pub(crate) tools: Vec<M6ToolEntry>,
}

pub(crate) fn validate_m6_registry_common(
    path: &Path,
    schema_version: u32,
    entry_ids: Vec<String>,
    entry_kinds: Vec<String>,
    expected_label: &str,
) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    if schema_version != 1 {
        messages.push(format!(
            "{expected_label}.schema_version must be 1 (got {schema_version})"
        ));
    }
    if entry_ids.is_empty() {
        messages.push(format!("{expected_label} registry must have at least 1 entry"));
    }
    for (i, id) in entry_ids.iter().enumerate() {
        if id.trim().is_empty() {
            messages.push(format!("{expected_label}[{i}].id must be non-empty"));
        }
    }
    for (i, k) in entry_kinds.iter().enumerate() {
        if k.trim().is_empty() {
            messages.push(format!("{expected_label}[{i}].kind/class must be non-empty"));
        }
    }
    let _ = path;
    messages
}

pub(crate) fn validate_weapon_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6WeaponRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.weapons.iter().map(|w| w.id.clone()).collect();
    let kinds: Vec<String> = parsed.weapons.iter().map(|w| w.class.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "weapon_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("weapon_registry ({} entries)", parsed.weapons.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

pub(crate) fn validate_grenade_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6GrenadeRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.grenades.iter().map(|g| g.id.clone()).collect();
    let kinds: Vec<String> = parsed.grenades.iter().map(|g| g.kind.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "grenade_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("grenade_registry ({} entries)", parsed.grenades.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

pub(crate) fn validate_melee_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6MeleeRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.melees.iter().map(|m| m.id.clone()).collect();
    let kinds: Vec<String> = parsed.melees.iter().map(|m| m.kind.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "melee_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("melee_registry ({} entries)", parsed.melees.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}

pub(crate) fn validate_tool_registry(path: &Path, report: &mut ValidationReport) {
    let raw = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("read failed: {err}"));
            return;
        }
    };
    let parsed: M6ToolRegistry = match ron::from_str(&raw) {
        Ok(v) => v,
        Err(err) => {
            report.add_error(path.to_path_buf(), format!("ron parse failed: {err}"));
            return;
        }
    };
    let ids: Vec<String> = parsed.tools.iter().map(|t| t.id.clone()).collect();
    let kinds: Vec<String> = parsed.tools.iter().map(|t| t.kind.clone()).collect();
    let messages = validate_m6_registry_common(path, parsed.schema_version, ids, kinds, "tool_registry");
    if messages.is_empty() {
        report.add_pass(
            path.to_path_buf(),
            format!("tool_registry ({} entries)", parsed.tools.len()),
        );
    } else {
        report.add_error(path.to_path_buf(), messages.join("; "));
    }
}
