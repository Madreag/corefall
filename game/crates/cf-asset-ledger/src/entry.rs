//! M4A `AssetEntry` schema (locked at v1.0.0).
//!
//! Mirrors the spec exactly. Every entry written to `content/asset_ledger/ledger.jsonl`
//! conforms to this struct. New optional fields must use `serde(default)` so
//! the schema can grow without bumping the version. Forward-compatible field
//! additions remain in v1; layout-breaking changes go to v2 with a migration
//! shim registered at M39.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::category::{AssetCategory, License, PackageRef, ProductionTier, RegenStatus};

/// AssetEntry schema version. Lock at 1.0.0 per the M4A spec.
pub const ASSET_ENTRY_SCHEMA_VERSION: &str = "1.0.0";

/// AssetEntry's `id` is a blake3 hex string derived from
/// `category + canonical_name + tier`. We keep the hex form so it's
/// JSONL-friendly (no base64 escaping) and round-trips through grep / awk.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AssetId(pub String);

impl std::fmt::Display for AssetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AssetId {
    /// Compute `blake3(category|canonical_name|tier)` and return the hex.
    /// The seam character is `|` because `:` appears in pipeline ids and
    /// `/` appears in canonical_names; using `|` keeps the hash inputs
    /// unambiguous.
    pub fn compute(category: AssetCategory, canonical_name: &str, tier: ProductionTier) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(category.as_str().as_bytes());
        hasher.update(b"|");
        hasher.update(canonical_name.as_bytes());
        hasher.update(b"|");
        hasher.update(tier.as_str().as_bytes());
        Self(hex::encode(hasher.finalize().as_bytes()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One additional output (mipmap / normal map / alt-resolution) attached
/// to the same logical asset. Stored alongside the primary so a single
/// regenerate call can verify the entire bundle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdditionalOutput {
    pub label: String,
    pub output_path: PathBuf,
    pub blake3: String,
    pub size_bytes: u64,
}

/// Reference to an upstream input the regen pipeline needs (either another
/// ledger entry or a content-tracked input file).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegenInputRef {
    /// Another ledger entry (e.g. Tier 1 SVG used as ControlNet input).
    AssetId(AssetId),
    /// A content-tracked input file (e.g. a prompt template, palette JSON).
    Path { path: PathBuf, blake3: String },
}

/// Concrete generator metadata. Captures the tool + model + workflow used to
/// produce the asset. Determinism contract: same `tool` + `model` +
/// `workflow` + same `seed` MUST produce same `output_blake3`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GeneratorRef {
    pub tool: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_version: Option<String>,
}

/// Palette/lora references are simple string ids; the M4A schema does NOT
/// store the binary contents (those are tracked as `upstream_assets` or
/// `regen_inputs`).
pub type PaletteId = String;
pub type LoraRef = String;
pub type PipelineId = String;

/// A single AssetEntry. Field order mirrors the spec.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetEntry {
    pub id: AssetId,
    pub category: AssetCategory,
    /// Sub-category (e.g. "weapon-side", "ui-icon").
    pub kind: String,
    pub canonical_name: String,
    pub tier: ProductionTier,

    pub pipeline: PipelineId,
    pub generator: GeneratorRef,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    pub seed: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette_ref: Option<PaletteId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub style_lora: Option<LoraRef>,
    #[serde(default)]
    pub upstream_assets: Vec<AssetId>,

    pub output_path: PathBuf,
    pub output_format: String,
    pub output_size_bytes: u64,
    /// Hex blake3 hash of `output_path` at write-time.
    pub output_blake3: String,
    #[serde(default)]
    pub additional_outputs: Vec<AdditionalOutput>,

    pub generated_at_iso: String,
    pub generated_on_machine: String,
    #[serde(default)]
    pub generated_by_human: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_edit_notes: Option<String>,
    #[serde(default)]
    pub package_source: PackageRef,
    #[serde(default)]
    pub license: License,

    pub regen_command: String,
    #[serde(default)]
    pub regen_inputs: Vec<RegenInputRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regen_validated_at: Option<String>,
    #[serde(default = "default_regen_status")]
    pub regen_status: RegenStatus,

    /// When this entry is replaced by a newer one (re-generation), point to
    /// the new entry. `superseded_by` is mutable on the entry's append-only
    /// rewrite pass — see `storage::supersede_entry`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<AssetId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deprecated_at: Option<String>,

    /// Locked at "1.0.0" — every field is additive going forward.
    #[serde(default = "default_schema_version")]
    pub schema_version: String,

    /// Mod-extension surface. Engine ignores unknown keys; mods may stash
    /// per-package metadata here without bumping the schema.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extension_fields: BTreeMap<String, serde_json::Value>,
}

fn default_regen_status() -> RegenStatus {
    RegenStatus::Stale
}

fn default_schema_version() -> String {
    ASSET_ENTRY_SCHEMA_VERSION.to_string()
}

/// Builder-style constructor used by pipeline tools.
///
/// All required identity fields up-front; optional fields default to "not set".
/// The `id` is computed from `(category, canonical_name, tier)` at build time.
#[derive(Debug, Clone)]
pub struct AssetEntryBuilder {
    category: AssetCategory,
    kind: String,
    canonical_name: String,
    tier: ProductionTier,
    pipeline: PipelineId,
    prompt: String,
    seed: u64,
    output_path: PathBuf,
    generator: GeneratorRef,
    output_format: Option<String>,
    output_size_bytes: Option<u64>,
    output_blake3: Option<String>,
    negative_prompt: Option<String>,
    palette_ref: Option<PaletteId>,
    style_lora: Option<LoraRef>,
    upstream_assets: Vec<AssetId>,
    additional_outputs: Vec<AdditionalOutput>,
    generated_at_iso: Option<String>,
    generated_on_machine: Option<String>,
    generated_by_human: bool,
    human_edit_notes: Option<String>,
    package_source: PackageRef,
    license: License,
    regen_command: Option<String>,
    regen_inputs: Vec<RegenInputRef>,
    regen_status: RegenStatus,
    extension_fields: BTreeMap<String, serde_json::Value>,
}

impl AssetEntryBuilder {
    pub fn new(
        category: AssetCategory,
        kind: impl Into<String>,
        canonical_name: impl Into<String>,
        tier: ProductionTier,
        pipeline: impl Into<String>,
        prompt: impl Into<String>,
        seed: u64,
        output_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            category,
            kind: kind.into(),
            canonical_name: canonical_name.into(),
            tier,
            pipeline: pipeline.into(),
            prompt: prompt.into(),
            seed,
            output_path: output_path.into(),
            generator: GeneratorRef::default(),
            output_format: None,
            output_size_bytes: None,
            output_blake3: None,
            negative_prompt: None,
            palette_ref: None,
            style_lora: None,
            upstream_assets: Vec::new(),
            additional_outputs: Vec::new(),
            generated_at_iso: None,
            generated_on_machine: None,
            generated_by_human: false,
            human_edit_notes: None,
            package_source: PackageRef::Vanilla,
            license: License::Cc0,
            regen_command: None,
            regen_inputs: Vec::new(),
            regen_status: RegenStatus::Stale,
            extension_fields: BTreeMap::new(),
        }
    }

    pub fn with_generator(mut self, generator: GeneratorRef) -> Self {
        self.generator = generator;
        self
    }
    pub fn with_negative_prompt(mut self, negative_prompt: impl Into<String>) -> Self {
        self.negative_prompt = Some(negative_prompt.into());
        self
    }
    pub fn with_palette(mut self, palette: impl Into<String>) -> Self {
        self.palette_ref = Some(palette.into());
        self
    }
    pub fn with_style_lora(mut self, lora: impl Into<String>) -> Self {
        self.style_lora = Some(lora.into());
        self
    }
    pub fn with_upstream(mut self, ids: impl IntoIterator<Item = AssetId>) -> Self {
        self.upstream_assets = ids.into_iter().collect();
        self
    }
    pub fn with_additional_output(mut self, extra: AdditionalOutput) -> Self {
        self.additional_outputs.push(extra);
        self
    }
    pub fn with_generated_at_iso(mut self, ts: impl Into<String>) -> Self {
        self.generated_at_iso = Some(ts.into());
        self
    }
    pub fn with_generated_on_machine(mut self, host: impl Into<String>) -> Self {
        self.generated_on_machine = Some(host.into());
        self
    }
    pub fn with_human_edit(mut self, edited: bool, notes: Option<String>) -> Self {
        self.generated_by_human = edited;
        self.human_edit_notes = notes;
        self
    }
    pub fn with_package_source(mut self, source: PackageRef) -> Self {
        self.package_source = source;
        self
    }
    pub fn with_license(mut self, license: License) -> Self {
        self.license = license;
        self
    }
    pub fn with_regen_command(mut self, cmd: impl Into<String>) -> Self {
        self.regen_command = Some(cmd.into());
        self
    }
    pub fn with_regen_inputs(mut self, inputs: impl IntoIterator<Item = RegenInputRef>) -> Self {
        self.regen_inputs = inputs.into_iter().collect();
        self
    }
    pub fn with_regen_status(mut self, status: RegenStatus) -> Self {
        self.regen_status = status;
        self
    }
    pub fn with_extension_field(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extension_fields.insert(key.into(), value);
        self
    }
    pub fn with_output_blake3(mut self, hex: impl Into<String>) -> Self {
        self.output_blake3 = Some(hex.into());
        self
    }
    pub fn with_output_size(mut self, size: u64) -> Self {
        self.output_size_bytes = Some(size);
        self
    }
    pub fn with_output_format(mut self, fmt: impl Into<String>) -> Self {
        self.output_format = Some(fmt.into());
        self
    }

    /// Build the entry. If `output_blake3` was not explicitly set, the
    /// builder will attempt to hash the file at `output_path`. If that
    /// fails (file missing) the entry is built with an empty blake3 and
    /// status forced to `Missing`.
    pub fn build(self) -> AssetEntry {
        let id = AssetId::compute(self.category, &self.canonical_name, self.tier);
        let (size_bytes, blake3_hex, status_override) = match (self.output_size_bytes, self.output_blake3.clone()) {
            (Some(s), Some(h)) => (s, h, None),
            _ => match crate::integrity::hash_path(&self.output_path) {
                Ok((size, hash)) => (
                    self.output_size_bytes.unwrap_or(size),
                    self.output_blake3.clone().unwrap_or(hash),
                    None,
                ),
                Err(_) => (
                    self.output_size_bytes.unwrap_or(0),
                    self.output_blake3.unwrap_or_default(),
                    Some(RegenStatus::Missing),
                ),
            },
        };
        let output_format = self
            .output_format
            .or_else(|| {
                self.output_path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|s| s.to_string())
            })
            .unwrap_or_else(|| self.category.default_extension().to_string());
        let regen_command = self
            .regen_command
            .unwrap_or_else(|| format!("cf-mod ledger regenerate {}", id.as_str()));
        let generated_at_iso = self.generated_at_iso.unwrap_or_else(|| {
            if deterministic_ledger_enabled() {
                deterministic_generated_at_iso(&blake3_hex)
            } else {
                chrono::Utc::now().to_rfc3339()
            }
        });
        let generated_on_machine = self.generated_on_machine.unwrap_or_else(|| {
            if deterministic_ledger_enabled() {
                "deterministic".to_string()
            } else {
                default_hostname()
            }
        });
        AssetEntry {
            id,
            category: self.category,
            kind: self.kind,
            canonical_name: self.canonical_name,
            tier: self.tier,
            pipeline: self.pipeline,
            generator: self.generator,
            prompt: self.prompt,
            negative_prompt: self.negative_prompt,
            seed: self.seed,
            palette_ref: self.palette_ref,
            style_lora: self.style_lora,
            upstream_assets: self.upstream_assets,
            output_path: self.output_path,
            output_format,
            output_size_bytes: size_bytes,
            output_blake3: blake3_hex,
            additional_outputs: self.additional_outputs,
            generated_at_iso,
            generated_on_machine,
            generated_by_human: self.generated_by_human,
            human_edit_notes: self.human_edit_notes,
            package_source: self.package_source,
            license: self.license,
            regen_command,
            regen_inputs: self.regen_inputs,
            regen_validated_at: None,
            regen_status: status_override.unwrap_or(self.regen_status),
            superseded_by: None,
            deprecated_at: None,
            schema_version: ASSET_ENTRY_SCHEMA_VERSION.to_string(),
            extension_fields: self.extension_fields,
        }
    }
}

fn default_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// **M4A determinism contract**: when `CF_DETERMINISTIC_LEDGER=1`, both
/// `generated_at_iso` and `generated_on_machine` default to deterministic
/// placeholder strings rather than wall-clock / hostname. Makes the
/// ledger.jsonl file byte-reproducible across CI runs.
pub fn deterministic_ledger_enabled() -> bool {
    std::env::var("CF_DETERMINISTIC_LEDGER").is_ok_and(|v| matches!(v.as_str(), "1" | "true" | "TRUE"))
}

/// Pin a deterministic placeholder for `generated_at_iso` derived from
/// `seed` (typically the entry's output_blake3) so each asset gets a stable
/// per-entry pseudo-timestamp without colliding across entries.
pub fn deterministic_generated_at_iso(seed: &str) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"cf-asset-ledger/v1/generated_at_iso/");
    hasher.update(seed.as_bytes());
    let hash = hasher.finalize();
    let hex = hex::encode(hash.as_bytes());
    format!("ledger-deterministic:{}", &hex[..16])
}

impl AssetEntry {
    pub fn id(&self) -> &AssetId {
        &self.id
    }
    pub fn output_path(&self) -> &Path {
        &self.output_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn asset_id_is_deterministic() {
        let a = AssetId::compute(
            AssetCategory::WeaponSprite,
            "iron_rifle_m1_side_v1",
            ProductionTier::Tier1Svg,
        );
        let b = AssetId::compute(
            AssetCategory::WeaponSprite,
            "iron_rifle_m1_side_v1",
            ProductionTier::Tier1Svg,
        );
        assert_eq!(a, b);
    }

    #[test]
    fn asset_id_differs_by_tier() {
        let a = AssetId::compute(AssetCategory::WeaponSprite, "x", ProductionTier::Tier1Svg);
        let b = AssetId::compute(AssetCategory::WeaponSprite, "x", ProductionTier::Tier2ComfyUi);
        assert_ne!(a, b);
    }

    #[test]
    fn asset_id_differs_by_canonical_name() {
        let a = AssetId::compute(AssetCategory::WeaponSprite, "a", ProductionTier::Tier1Svg);
        let b = AssetId::compute(AssetCategory::WeaponSprite, "b", ProductionTier::Tier1Svg);
        assert_ne!(a, b);
    }

    #[test]
    fn asset_id_hex_is_64_chars() {
        let id = AssetId::compute(AssetCategory::WeaponSprite, "x", ProductionTier::Tier1Svg);
        assert_eq!(id.as_str().len(), 64);
        assert!(id.as_str().chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn entry_roundtrip_through_jsonl() {
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "test_rifle",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "side-profile rifle",
            1234,
            "content/assets/test_rifle.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_output_size(42)
        .with_generated_at_iso("2026-05-13T00:00:00Z")
        .with_generated_on_machine("ci")
        .build();
        let line = serde_json::to_string(&entry).expect("serialize");
        let parsed: AssetEntry = serde_json::from_str(&line).expect("deserialize");
        assert_eq!(parsed, entry);
    }

    #[test]
    fn schema_version_default_locked_at_v1() {
        assert_eq!(ASSET_ENTRY_SCHEMA_VERSION, "1.0.0");
    }

    /// **M4A § Schema design / Mod-extension fields**: mods stash custom
    /// metadata in `extension_fields: HashMap<String, Value>` and the
    /// engine round-trips it opaquely. This test pins the contract:
    /// build with extension fields → serialize → deserialize → fields
    /// are preserved byte-identically.
    #[test]
    fn extension_fields_round_trip_through_jsonl() {
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "weapon-side",
            "rifle_ext",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            1,
            "/tmp/rifle_ext.svg",
        )
        .with_output_blake3("a".repeat(64))
        .with_extension_field("mod_team", serde_json::json!("necropolis_dlc_team"))
        .with_extension_field(
            "compatibility",
            serde_json::json!({"min_engine": "0.0.1", "max_engine": "1.0.0"}),
        )
        .with_extension_field("tags", serde_json::json!(["spooky", "halloween", "limited"]))
        .build();
        let line = serde_json::to_string(&entry).unwrap();
        let parsed: AssetEntry = serde_json::from_str(&line).unwrap();
        assert_eq!(parsed.extension_fields.len(), 3);
        assert_eq!(
            parsed.extension_fields.get("mod_team").and_then(|v| v.as_str()),
            Some("necropolis_dlc_team")
        );
        assert_eq!(
            parsed
                .extension_fields
                .get("compatibility")
                .and_then(|v| v.get("min_engine"))
                .and_then(|v| v.as_str()),
            Some("0.0.1")
        );
        let tags = parsed.extension_fields.get("tags").and_then(|v| v.as_array()).unwrap();
        assert_eq!(tags.len(), 3);
    }

    /// **M4A determinism**: `CF_DETERMINISTIC_LEDGER=1` makes the builder
    /// pin `generated_at_iso` + `generated_on_machine` to deterministic
    /// values derived from the entry's blake3 instead of wall-clock /
    /// hostname. Tests that the env flag is honored.
    #[test]
    fn deterministic_ledger_env_flag_pins_metadata() {
        // SAFETY: tests that mutate env vars must run isolated; we
        // restore after to avoid leaking state into other tests.
        let prev = std::env::var("CF_DETERMINISTIC_LEDGER").ok();
        std::env::set_var("CF_DETERMINISTIC_LEDGER", "1");
        let entry = AssetEntryBuilder::new(
            AssetCategory::WeaponSprite,
            "kind",
            "deterministic",
            ProductionTier::Tier1Svg,
            "M9A_svg_v1",
            "p",
            1,
            "/tmp/d.svg",
        )
        .with_output_blake3("b".repeat(64))
        .build();
        match prev {
            Some(v) => std::env::set_var("CF_DETERMINISTIC_LEDGER", v),
            None => std::env::remove_var("CF_DETERMINISTIC_LEDGER"),
        }
        assert!(entry.generated_at_iso.starts_with("ledger-deterministic:"));
        assert_eq!(entry.generated_on_machine, "deterministic");
    }
}
