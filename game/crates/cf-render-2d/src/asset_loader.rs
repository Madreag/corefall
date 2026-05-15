//! **M9A § "cf-render-2d loads SVG at startup"**: Tier-1 placeholder asset
//! loader.
//!
//! The M9A pipeline emits canonical SVG + PNG pairs under
//! `content/assets/placeholders/<category>/<canonical_name>.{svg,png}` and a
//! one-line ledger row per asset under `content/asset_ledger/ledger.jsonl`.
//! This module gives cf-render-2d a small in-memory index of those assets so
//! sprite-spawning code can ask:
//!
//!   "give me the PNG path for `corp_rifleman_human_idle_right`"
//!
//! without re-reading the ledger or filesystem each call. The index is a
//! cosmetic-only convenience — engine sim state never depends on it.
//!
//! cf-app is responsible for actually pushing the PNG bytes into a Bevy
//! texture atlas via `AssetServer::load` or `Image::from_dynamic` per the
//! M9A spec's "loads SVG → texture atlases at startup" clause. That hand-off
//! lives in cf-app; this loader just gives cf-app the path index.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// One Tier-1 placeholder entry. Mirrors the cf-asset-ledger AssetEntry
/// shape but trimmed to cosmetic-only render-side fields.
#[derive(Debug, Clone)]
pub struct AssetIndexEntry {
    pub canonical_name: String,
    pub category: String,
    pub kind: String,
    pub svg_path: PathBuf,
    pub png_path: Option<PathBuf>,
    pub palette_ref: Option<String>,
    pub width: u32,
    pub height: u32,
}

impl AssetIndexEntry {
    pub fn svg_path(&self) -> &Path {
        &self.svg_path
    }

    pub fn png_path(&self) -> Option<&Path> {
        self.png_path.as_deref()
    }
}

/// In-memory registry: lookup an asset by its canonical name.
#[derive(Resource, Debug, Clone, Default)]
pub struct AssetIndex {
    entries: HashMap<String, AssetIndexEntry>,
    by_category: HashMap<String, Vec<String>>,
}

impl AssetIndex {
    pub fn insert(&mut self, entry: AssetIndexEntry) {
        self.by_category
            .entry(entry.category.clone())
            .or_default()
            .push(entry.canonical_name.clone());
        self.entries.insert(entry.canonical_name.clone(), entry);
    }

    pub fn get(&self, canonical_name: &str) -> Option<&AssetIndexEntry> {
        self.entries.get(canonical_name)
    }

    pub fn names_in_category(&self, category: &str) -> impl Iterator<Item = &String> {
        self.by_category.get(category).into_iter().flat_map(|v| v.iter())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Resolve an asset file living under the M9A placeholder tree. Falls back
/// to the workspace-root `content/` if the leaf file isn't present under the
/// crate's local `content/`. Path resolution mirrors what
/// `tools/asset_gen/build_placeholders.py` writes: absolute paths in the
/// ledger; cosmetic-only crates can stash relative paths during dev.
pub fn resolve_placeholder_path(base: &Path, category: &str, canonical_name: &str, ext: &str) -> PathBuf {
    let sub = category_subdir(category);
    base.join("content")
        .join("assets")
        .join("placeholders")
        .join(sub)
        .join(format!("{canonical_name}.{ext}"))
}

/// Map an asset category to the on-disk subdirectory used by the M9A
/// pipeline. Keep in sync with `tools/asset_gen/build_placeholders.py
/// _output_dir_for`.
pub const fn category_subdir(category: &str) -> &'static str {
    match category.as_bytes() {
        b"WeaponSprite" => "weapons",
        b"ActorSprite" => "actors",
        b"VehicleSprite" => "vehicles",
        b"ChassisSprite" => "chassis",
        b"BaseModuleSprite" => "base_modules",
        b"UiIcon" => "ui_icons",
        b"MaterialSwatch" => "materials",
        b"Particle" => "particles",
        b"TerrainTile" => "terrain_tiles",
        b"Cosmetic" => "cosmetics",
        b"FactionEmblem" => "faction_emblems",
        b"CaptureGridOverlay" => "capture_overlays",
        _ => "misc",
    }
}

/// **M9A § "Modder authoring"** + "loads SVG → texture atlases at startup":
/// load the ledger row indices into `AssetIndex` so render code can look up
/// canonical-name → path without re-reading the ledger.jsonl every frame.
///
/// The ledger.jsonl format is one AssetEntry per line per
/// cf-asset-ledger/schemas/v1/asset_entry.schema.json. We extract the
/// minimum render-relevant fields: canonical_name, category, kind,
/// output_path, palette_ref, plus the first PNG additional_output for
/// quick atlas-load.
///
/// Returns the number of entries indexed.
pub fn load_ledger_index(ledger_path: &Path, into: &mut AssetIndex) -> std::io::Result<usize> {
    use std::io::{BufRead, BufReader};

    let file = std::fs::File::open(ledger_path)?;
    let reader = BufReader::new(file);
    let mut n = 0usize;
    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        let value: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let canonical_name = match value.get("canonical_name").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        let category = value
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("misc")
            .to_string();
        let kind = value.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let svg_path = match value.get("output_path").and_then(|v| v.as_str()) {
            Some(p) => PathBuf::from(p),
            None => continue,
        };
        let png_path = value
            .get("additional_outputs")
            .and_then(|v| v.as_array())
            .and_then(|arr| {
                arr.iter().find_map(|o| {
                    let label = o.get("label")?.as_str()?;
                    if label.starts_with("png_") {
                        o.get("output_path").and_then(|p| p.as_str()).map(PathBuf::from)
                    } else {
                        None
                    }
                })
            });
        let palette_ref = value.get("palette_ref").and_then(|v| v.as_str()).map(|s| s.to_string());
        let width = 256;
        let height = 256;
        into.insert(AssetIndexEntry {
            canonical_name,
            category,
            kind,
            svg_path,
            png_path,
            palette_ref,
            width,
            height,
        });
        n += 1;
    }
    Ok(n)
}

/// Bevy plugin that creates an empty `AssetIndex` resource and lets cf-app
/// hydrate it via a startup system. cf-app is responsible for choosing the
/// ledger path (vanilla vs mod-pack ledger merge).
pub struct AssetIndexPlugin;

impl Plugin for AssetIndexPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AssetIndex>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn resolve_placeholder_path_for_weapon() {
        let base = Path::new("/Users/erol/projects/corefall");
        let p = resolve_placeholder_path(base, "WeaponSprite", "rifle_iron_m1_side", "svg");
        assert_eq!(
            p,
            Path::new("/Users/erol/projects/corefall/content/assets/placeholders/weapons/rifle_iron_m1_side.svg")
        );
    }

    #[test]
    fn category_subdir_known_categories() {
        assert_eq!(category_subdir("WeaponSprite"), "weapons");
        assert_eq!(category_subdir("ActorSprite"), "actors");
        assert_eq!(category_subdir("UiIcon"), "ui_icons");
        assert_eq!(category_subdir("FactionEmblem"), "faction_emblems");
        assert_eq!(category_subdir("UnknownCategory"), "misc");
    }

    #[test]
    fn asset_index_inserts_and_retrieves() {
        let mut idx = AssetIndex::default();
        idx.insert(AssetIndexEntry {
            canonical_name: "rifle_iron_m1_side".to_string(),
            category: "WeaponSprite".to_string(),
            kind: "weapon-side".to_string(),
            svg_path: PathBuf::from("/tmp/rifle.svg"),
            png_path: Some(PathBuf::from("/tmp/rifle.png")),
            palette_ref: Some("hostile_corp".to_string()),
            width: 256,
            height: 256,
        });
        assert_eq!(idx.len(), 1);
        let e = idx.get("rifle_iron_m1_side").unwrap();
        assert_eq!(e.category, "WeaponSprite");
        assert_eq!(e.png_path(), Some(Path::new("/tmp/rifle.png")));
        let weapons: Vec<&String> = idx.names_in_category("WeaponSprite").collect();
        assert_eq!(weapons.len(), 1);
        assert!(idx.get("unknown").is_none());
    }

    #[test]
    fn load_ledger_index_parses_jsonl() {
        let dir = std::env::temp_dir().join(format!("cf-render-2d-asset-loader-{}-{}", std::process::id(), 42));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("ledger.jsonl");
        let mut f = std::fs::File::create(&path).unwrap();
        let entry = serde_json::json!({
            "id": "0".repeat(64),
            "canonical_name": "rifle_test",
            "category": "WeaponSprite",
            "kind": "weapon-side",
            "tier": "Tier1_SVG",
            "pipeline": "M9A_svg_v1",
            "generator": { "tool": "tools/asset_gen/build_placeholders.py", "model": "procedural-svg-composer-v1" },
            "prompt": "p",
            "seed": 1,
            "output_path": "/tmp/rifle_test.svg",
            "output_format": "svg",
            "output_size_bytes": 100,
            "output_blake3": "a".repeat(64),
            "additional_outputs": [
                {"label": "png_256", "output_path": "/tmp/rifle_test.png", "blake3": "b".repeat(64), "size_bytes": 200}
            ],
            "generated_at_iso": "ledger-deterministic:0",
            "generated_on_machine": "deterministic",
            "generated_by_human": false,
            "license": "CC0",
            "regen_command": "cf-mod ledger regenerate",
            "schema_version": "1.0.0",
            "palette_ref": "hostile_corp"
        });
        writeln!(f, "{}", serde_json::to_string(&entry).unwrap()).unwrap();
        drop(f);

        let mut idx = AssetIndex::default();
        let n = load_ledger_index(&path, &mut idx).unwrap();
        assert_eq!(n, 1);
        let e = idx.get("rifle_test").unwrap();
        assert_eq!(e.category, "WeaponSprite");
        assert_eq!(e.png_path(), Some(Path::new("/tmp/rifle_test.png")));
        assert_eq!(e.palette_ref.as_deref(), Some("hostile_corp"));
    }
}
