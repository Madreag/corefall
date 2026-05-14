# A1 — Asset Entry Schema Audit

## Summary

The `AssetEntry` schema (Rust struct + JSON schema + spec) is field-for-field aligned with the M4A spec at `v1.0.0`. Every spec field is present with a matching type and wire format, all required serde renames (`Audio_SFX`, `Tier1_SVG`, `CC0`, etc.) are present, and 46 unit + integration tests pass. No BLOCKERs and no MAJOR drift. Three intentional deviations are documented in code: (1) `AssetId::compute` uses a `|` seam character not specified literally in the spec, (2) JSON schema enforces `additionalProperties: false` at the top level while the Rust struct silently drops unknown top-level keys (intentional forward-compat asymmetry), and (3) the spec's "16 categories" comment is followed by a 17-entry list including the `Mod_Custom` catch-all, which the impl honors as 17 variants.

---

## Field-by-Field Map (Spec → Rust struct → JSON schema)

Spec block: `/Users/erol/projects/corefall/specs/done/M4A.md` lines 56-118.
Rust struct: `/Users/erol/projects/corefall/game/crates/cf-asset-ledger/src/entry.rs` lines 99-161.
JSON schema: `/Users/erol/projects/corefall/game/crates/cf-asset-ledger/schemas/v1/asset_entry.schema.json`.

| Spec Field | Rust Field | JSON Schema Required | Type Match | Notes |
|---|---|---|---|---|
| `id: AssetId` | `id: AssetId` (newtype around `String`) | required | YES | Both: 64-char lowercase hex blake3. JSON pattern `^[0-9a-f]{64}$` enforces. |
| `category: AssetCategory` | `category: AssetCategory` | required | YES | 17 variants (16 engine + `ModCustom`); see Category table. |
| `kind: AssetKind` | `kind: String` | required | TYPE-NAME DRIFT | Spec names the type `AssetKind` but never defines variants. Rust uses raw `String` (`"weapon-side"`, `"ui-icon"` example values from spec). Documented as sub-category free-form string. Acceptable. |
| `canonical_name: String` | `canonical_name: String` | required (`minLength: 1`) | YES | |
| `tier: ProductionTier` | `tier: ProductionTier` | required | YES | 7 variants; see Tier table. |
| `pipeline: PipelineId` | `pipeline: PipelineId` (type alias `String`) | required (`minLength: 1`) | YES | Free-form namespaced string per spec (`M9A_svg_v1`, etc.). |
| `generator: GeneratorRef` | `generator: GeneratorRef` (struct: `tool`, `model`, `workflow?`, `model_version?`) | required (`tool`, `model`) | YES | JSON schema requires `tool` + `model`; Rust matches. `workflow` / `model_version` are `Option<String>` with `serde(default, skip_serializing_if = Option::is_none)`. |
| `prompt: String` | `prompt: String` | required | YES | |
| `negative_prompt: Option<String>` | `negative_prompt: Option<String>` (`skip_serializing_if = Option::is_none`) | optional | YES | JSON schema allows `["string", "null"]`. |
| `seed: u64` | `seed: u64` | required | YES | JSON schema `integer, minimum: 0`. |
| `palette_ref: Option<PaletteId>` | `palette_ref: Option<String>` (`PaletteId = String`) | optional | YES | |
| `style_lora: Option<LoraRef>` | `style_lora: Option<String>` (`LoraRef = String`) | optional | YES | |
| `upstream_assets: Vec<AssetId>` | `upstream_assets: Vec<AssetId>` (`serde(default)`) | optional | YES | JSON schema array of 64-hex strings. |
| `output_path: PathBuf` | `output_path: PathBuf` | required (`minLength: 1`) | YES | `PathBuf` serializes as string. |
| `output_format: String` | `output_format: String` | required (`minLength: 1`) | YES | |
| `output_size_bytes: u64` | `output_size_bytes: u64` | required | YES | JSON schema `integer, minimum: 0`. |
| `output_blake3: String` | `output_blake3: String` | required | YES (loose) | JSON schema accepts empty OR 64-hex (`^([0-9a-f]{64})?$`). Empty allowed because builder records empty hash when file is missing and forces `regen_status = Missing`. Acceptable. |
| `additional_outputs: Vec<AdditionalOutput>` | `additional_outputs: Vec<AdditionalOutput>` (`serde(default)`) | optional | YES | Sub-struct: `label`, `output_path`, `blake3`, `size_bytes`. JSON schema mirrors. |
| `generated_at_iso: String` | `generated_at_iso: String` | required (`minLength: 1`) | YES | RFC 3339; builder defaults to `chrono::Utc::now().to_rfc3339()`. |
| `generated_on_machine: String` | `generated_on_machine: String` | required | YES | Builder defaults to `HOSTNAME` / `COMPUTERNAME` / `"unknown"`. |
| `generated_by_human: bool` | `generated_by_human: bool` (`serde(default)`) | optional | YES | Defaults `false`. |
| `human_edit_notes: Option<String>` | `human_edit_notes: Option<String>` (`skip_serializing_if = Option::is_none`) | optional | YES | |
| `package_source: PackageRef` | `package_source: PackageRef` (`serde(default)` → `Vanilla`) | optional | YES | See PackageRef table. JSON `oneOf` covers unit + tagged-string variants. |
| `license: License` | `license: License` (`serde(default)` → `Cc0`) | optional | YES | See License table. |
| `regen_command: String` | `regen_command: String` | required (`minLength: 1`) | YES | Builder defaults to `format!("cf-mod ledger regenerate {id}")`. |
| `regen_inputs: Vec<RegenInputRef>` | `regen_inputs: Vec<RegenInputRef>` (`serde(default)`) | optional | YES | Enum `AssetId(AssetId)` or `Path { path, blake3 }`. |
| `regen_validated_at: Option<String>` | `regen_validated_at: Option<String>` (`skip_serializing_if = Option::is_none`) | optional | YES | |
| `regen_status: RegenStatus` | `regen_status: RegenStatus` (`serde(default = "default_regen_status")` → `Stale`) | optional | YES | Spec lists 5 variants; Rust matches. Default = `Stale` is consistent with the "entry exists but never validated" semantic. |
| `superseded_by: Option<AssetId>` | `superseded_by: Option<AssetId>` (`skip_serializing_if = Option::is_none`) | optional | YES | JSON pattern `^([0-9a-f]{64})?$`. |
| `deprecated_at: Option<String>` | `deprecated_at: Option<String>` (`skip_serializing_if = Option::is_none`) | optional | YES | |
| `schema_version: String "1.0.0"` | `schema_version: String` (`serde(default = "default_schema_version")` → `"1.0.0"`) | optional but enum-locked | YES | JSON schema `enum: ["1.0.0"]`. Constant `ASSET_ENTRY_SCHEMA_VERSION = "1.0.0"`. |
| *(spec text: "Mod-extension fields via `extension_fields: HashMap<String, Value>`")* | `extension_fields: BTreeMap<String, serde_json::Value>` (`serde(default, skip_serializing_if = BTreeMap::is_empty)`) | optional | YES (semantic) | Spec describes this in "Schema design > Mod-extension fields" prose, not in the inline struct definition. Rust uses `BTreeMap` (stable iteration order — useful for diffs) instead of `HashMap`. JSON schema permits `additionalProperties: true` inside this nested object. |

**Verdict on field coverage:** every spec field is present in the Rust struct with a correct type. `extension_fields` is the only Rust field beyond the spec's inline `pub struct AssetEntry { ... }` block; it is explicitly licensed by the spec's "Schema design" paragraph.

---

## Category Enum Audit

Spec text: 17 variants listed under the "16 categories below" comment (the trailing `Mod_Custom` is the catch-all). Rust enum matches the list exactly.

| Spec Wire Name | Rust Variant | serde rename | `as_str()` Returns |
|---|---|---|---|
| `UiIcon` | `UiIcon` | (none) | `"UiIcon"` |
| `WeaponSprite` | `WeaponSprite` | (none) | `"WeaponSprite"` |
| `ActorSprite` | `ActorSprite` | (none) | `"ActorSprite"` |
| `VehicleSprite` | `VehicleSprite` | (none) | `"VehicleSprite"` |
| `ChassisSprite` | `ChassisSprite` | (none) | `"ChassisSprite"` |
| `BaseModuleSprite` | `BaseModuleSprite` | (none) | `"BaseModuleSprite"` |
| `TerrainTile` | `TerrainTile` | (none) | `"TerrainTile"` |
| `MaterialSwatch` | `MaterialSwatch` | (none) | `"MaterialSwatch"` |
| `Particle` | `Particle` | (none) | `"Particle"` |
| `Animation` | `Animation` | (none) | `"Animation"` |
| `Audio_SFX` | `AudioSfx` | `#[serde(rename = "Audio_SFX")]` | `"Audio_SFX"` |
| `Audio_Voice` | `AudioVoice` | `#[serde(rename = "Audio_Voice")]` | `"Audio_Voice"` |
| `Audio_Music` | `AudioMusic` | `#[serde(rename = "Audio_Music")]` | `"Audio_Music"` |
| `Narrative_Text` | `NarrativeText` | `#[serde(rename = "Narrative_Text")]` | `"Narrative_Text"` |
| `Localization_Strings` | `LocalizationStrings` | `#[serde(rename = "Localization_Strings")]` | `"Localization_Strings"` |
| `Cosmetic` | `Cosmetic` | (none) | `"Cosmetic"` |
| `Mod_Custom` | `ModCustom` | `#[serde(rename = "Mod_Custom")]` | `"Mod_Custom"` |

**Confirmed:** the Rust enum has 17 variants (16 engine + `ModCustom` catch-all). The unit test `all_categories_have_unique_str` asserts exactly 17 with `assert_eq!(seen.len(), 17)`. JSON schema `enum` lists exactly the same 17 strings. The spec text's "16 categories below" comment is a minor textual inconsistency; the listed-variant set is 17 and `Mod_Custom` is documented in spec line 92 (`Mod_Custom` — `mod-supplied; modder declares category`). Treating this as **acceptable**: `Mod_Custom` is the catch-all the spec text calls out separately ("mod-supplied; modder declares category"); the engine ships 16 first-party categories + the mod escape hatch.

---

## Tier Enum Audit

| Spec Wire Name | Rust Variant | serde rename | `as_str()` Returns |
|---|---|---|---|
| `Tier0_Placeholder` | `Tier0Placeholder` | `#[serde(rename = "Tier0_Placeholder")]` | `"Tier0_Placeholder"` |
| `Tier1_SVG` | `Tier1Svg` | `#[serde(rename = "Tier1_SVG")]` | `"Tier1_SVG"` |
| `Tier1_LLM_Audio` | `Tier1LlmAudio` | `#[serde(rename = "Tier1_LLM_Audio")]` | `"Tier1_LLM_Audio"` |
| `Tier2_ComfyUI` | `Tier2ComfyUi` | `#[serde(rename = "Tier2_ComfyUI")]` | `"Tier2_ComfyUI"` |
| `Tier2_Audio_Production` | `Tier2AudioProduction` | `#[serde(rename = "Tier2_Audio_Production")]` | `"Tier2_Audio_Production"` |
| `Tier3_Polish` | `Tier3Polish` | `#[serde(rename = "Tier3_Polish")]` | `"Tier3_Polish"` |
| `Mod_Supplied` | `ModSupplied` | `#[serde(rename = "Mod_Supplied")]` | `"Mod_Supplied"` |

**Confirmed:** 7 variants. Unit test `all_tiers_have_unique_str` asserts `assert_eq!(seen.len(), 7)`. JSON schema `enum` lists exactly the same 7 strings. The spec's tier list (lines 121-128 of M4A.md) is matched exactly.

Note: spec's PascalCase `ProductionTier` field example list in the struct (line 64) says `Tier0 / Tier1_SVG / Tier1_LLM_Audio / Tier2_ComfyUI / Tier2_Audio / Tier3_Polish` — shorter for brevity. The full enum in spec lines 121-128 uses `Tier0_Placeholder`, `Tier2_Audio_Production`, etc., which is what the Rust code matches.

---

## RegenStatus Enum Audit

| Spec Wire Name | Rust Variant | serde rename | `as_str()` Returns |
|---|---|---|---|
| `Fresh` | `Fresh` | (none) | `"Fresh"` |
| `Stale` | `Stale` | (none) | `"Stale"` |
| `Drifted` | `Drifted` | (none) | `"Drifted"` |
| `Missing` | `Missing` | (none) | `"Missing"` |
| `Failed` | `Failed` | (none) | `"Failed"` |

**Confirmed:** 5 variants matching spec exactly. JSON schema `enum` lists the same 5. Default value via `default_regen_status() -> RegenStatus::Stale` (correct — "entry exists but never validated"). Builder forces `Missing` if the output file cannot be hashed at build time.

---

## License Enum Audit

| Spec Wire Name | Rust Variant | serde rename | `as_label()` / serialized form |
|---|---|---|---|
| `CC0` | `Cc0` | `#[serde(rename = "CC0")]` | `"CC0"` |
| `CC-BY` | `CcBy` | `#[serde(rename = "CC-BY")]` | `"CC-BY"` |
| `CC-BY-SA` | `CcBySa` | `#[serde(rename = "CC-BY-SA")]` | `"CC-BY-SA"` |
| `proprietary` | `Proprietary` | `#[serde(rename = "Proprietary")]` | `"Proprietary"` |
| `mod-supplied` | `ModSupplied(String)` | `#[serde(rename = "mod-supplied")]` | `{"mod-supplied": "<inner>"}` |
| *(extension)* | `Custom(String)` | `#[serde(rename = "custom")]` | `{"custom": "<SPDX>"}` |

**Notes:**
- Spec text says `License // CC0 / CC-BY / proprietary / mod-supplied`. Rust uses **capitalized** `"Proprietary"` while the spec text uses **lowercase** `"proprietary"`. JSON schema also uses `"Proprietary"` (matches Rust). This is a MINOR capitalization deviation from the spec's prose comment but consistent between Rust and JSON schema.
- Rust adds a `Custom(String)` variant beyond the spec for SPDX expressions / free-form text. This is an additive extension (not in the locked spec) but it ships as part of v1.0.0; mods can use it.
- `Default::default()` returns `License::Cc0`. Builder uses `Cc0` as the default. Unit test `license_default_is_cc0` confirms.

---

## PackageRef Enum Audit

The spec uses `PackageRef` as the type for `package_source` and gives the inline example `// vanilla / mod / faction-pack`. It does not enumerate variant names in PascalCase.

| Spec Example | Rust Variant | Wire Form |
|---|---|---|
| `vanilla` | `Vanilla` (unit) | `"Vanilla"` |
| `mod` (with id) | `Mod(String)` | `{"Mod": "<mod_id>"}` |
| `faction-pack` (with id) | `FactionPack(String)` | `{"FactionPack": "<pack_id>"}` |

**Confirmed:** 3 variants matching the spec's three forms (unit + 2 tagged). JSON schema `oneOf` permits the unit-string `"Vanilla"` and the tagged-object forms. `Default::default()` returns `PackageRef::Vanilla`. Unit test `package_ref_default_is_vanilla` confirms.

---

## Drift Findings (BLOCKER / MAJOR / MINOR)

### BLOCKER

None.

### MAJOR

None. (The Rust + JSON-schema impl is field-for-field and rename-for-rename aligned with the spec.)

### MINOR

- **[MINOR] Spec text "16 categories below" vs. 17-variant list.** Spec line 60 says `// 16 categories below`, but the spec then enumerates 17 entries (the final being `Mod_Custom // mod-supplied; modder declares category`). Rust impl ships 17 variants (16 engine + `ModCustom`). The unit test `all_categories_have_unique_str` asserts `seen.len() == 17` with a documentation comment "16 engine categories + ModCustom catch-all = 17 enum variants." **Treat as docs-text inconsistency in the spec, not an impl drift.** `Mod_Custom` is the spec's documented catch-all for mod-supplied content, so the 17th variant is intentional and the impl maps it correctly to spec wire `"Mod_Custom"`.
- **[MINOR] `AssetId::compute` uses pipe-`|` seam.** Spec line 62 says `blake3(category + canonical_name + tier)` with `+` (concatenation), no seam character. The Rust impl interleaves a `|` byte between each component for disambiguation, with the in-code comment: *"The seam character is `|` because `:` appears in pipeline ids and `/` appears in canonical_names; using `|` keeps the hash inputs unambiguous."* This produces hash bytes that **differ** from a literal `+`-concatenation. **INTENTIONAL deviation, explicitly documented in code.** No external system depends on these hash bytes matching a non-Rust producer's literal-`+` interpretation, so functional impact is nil. Flagging for the audit record.
- **[MINOR] Spec types `kind: AssetKind`; Rust uses `kind: String`.** Spec line 63 names the type `AssetKind` but never enumerates its variants (only inline examples like `"weapon-side"`, `"ui-icon"`). Rust impl uses raw `String` with a doc comment "Sub-category (e.g. 'weapon-side', 'ui-icon')." Defensible given the spec's lack of a defined variant set, but the type name `AssetKind` is gone. Acceptable.
- **[MINOR] License wire name `"Proprietary"` vs. spec text `"proprietary"`.** Spec line 78 says `// CC0 / CC-BY / proprietary / mod-supplied`. Rust renames to `"Proprietary"` (capitalized); JSON schema also uses `"Proprietary"`. The two impl artifacts are consistent, but they differ from the spec's lowercase prose. Acceptable since the JSON schema enum is the canonical wire-format gate.
- **[MINOR] `License::Custom(String)` is an additive extension.** Not in the spec's 4-form enumeration. Ships in v1.0.0 for SPDX/free-form license declarations. Strictly additive; no drift risk.
- **[MINOR] No direct unit test for `extension_fields` round-trip.** The builder has `with_extension_field()` and the field exists on the struct, but no test inserts an extension key and asserts it survives a serialize/deserialize cycle. The general `entry_roundtrip_through_jsonl` covers default empty case only.
- **[MINOR] JSON schema is not enforced at runtime.** `validate_entry_json()` in `lib.rs` performs Rust-side serde-deserialize + id-drift + schema-version checks, but does not run a JSON-schema validator against `schemas/v1/asset_entry.schema.json`. External tools (mod CI gates) would need a separate `jsonschema` runner. The schema file exists as a documentation/external-validation artifact only.

---

## Forward-Compat Audit

### Does adding a new optional field break existing entries? [N — evidence below]

**No.** All optional/additive fields use `#[serde(default)]` and (where appropriate) `#[serde(skip_serializing_if = "Option::is_none")]`. Concretely, every optional field in the struct (`negative_prompt`, `palette_ref`, `style_lora`, `upstream_assets`, `additional_outputs`, `generated_by_human`, `human_edit_notes`, `package_source`, `license`, `regen_inputs`, `regen_validated_at`, `regen_status`, `superseded_by`, `deprecated_at`, `schema_version`, `extension_fields`) is annotated with `serde(default)` or has an explicit `default = "<fn>"` callback. The schema-locking-test `schema_version_default_locked_at_v1` asserts the default is `"1.0.0"`.

The discipline contract from M4A spec line 281-283 — *"every field additive; new optional fields via serde-default"* — is structurally enforced by the existing field annotations. Any future additive field must follow the same pattern; this is a discipline-via-pattern not a compile-time constraint.

### Does removing a field break the lock? [Y — evidence below]

**Yes.** Required fields (`id`, `category`, `kind`, `canonical_name`, `tier`, `pipeline`, `generator`, `prompt`, `seed`, `output_path`, `output_format`, `output_size_bytes`, `output_blake3`, `generated_at_iso`, `generated_on_machine`, `regen_command`) have no `serde(default)` and would fail deserialize-of-old-entries if removed. Per the spec's "every field additive; layout-breaking changes go to v2 with a migration shim registered at M39" (spec line 281-282, plus `entry.rs` doc comment line 6-9), removing a required field is a v2 bump requiring M39 migration policy. Same conclusion for shrinking an enum (e.g. dropping `License::Custom`) — old entries serialized with that variant would fail to deserialize against the smaller enum.

### Does `extension_fields` silently accept unknown keys? [Partial — evidence + caveat below]

**Yes, but only when explicitly nested under `extension_fields`.** Top-level unknown keys are handled asymmetrically:

1. **Rust struct (`entry.rs` line 99)** does NOT use `#[serde(deny_unknown_fields)]`. Confirmed via `Grep` over the crate: 0 matches for `deny_unknown_fields` and 0 matches for `serde(flatten)` on `extension_fields`. So unknown TOP-LEVEL keys in a JSONL line will be **silently dropped** by `serde_json` during deserialization. They are NOT forwarded into `extension_fields`.
2. **`extension_fields` itself** is `BTreeMap<String, serde_json::Value>` — mods that want to attach metadata MUST nest it explicitly inside `extension_fields: { ... }`. The builder exposes `with_extension_field(key, value)` for this. Once nested, arbitrary keys/values flow through opaquely.
3. **JSON schema** sets `additionalProperties: false` at the top level (line 7) and `additionalProperties: true` inside `extension_fields` (line 189). So an external JSON-schema validator will **reject** top-level unknown keys, in contrast to Rust's silent-drop behavior.

This asymmetry is **intentional**: Rust lenient = forward-compat (old code can read newer entries with additive fields and silently ignore them, which is exactly the M4A-locked-v1 evolution model). JSON schema strict = wire-format-author discipline (mod authors must nest their custom metadata under `extension_fields`, not at top level). The lib.rs validate_entry_json() does an extra blake3-id-drift recompute check on top, providing a Rust-side validation path that's stricter than serde alone.

**Test evidence (closest available, NOT a direct extension-keys test):**
- `entry::tests::entry_roundtrip_through_jsonl` round-trips a default (empty `extension_fields`) entry.
- `tests::validate_entry_json_accepts_canonical_entry` validates a canonical entry via the Rust-side validator.
- **Gap noted under MINOR: no test exercises the with_extension_field round-trip directly.** Production code can rely on the builder's correct insertion + serde's default Map handling, but a dedicated test would be appropriate as audit-follow-up.

### Does `AssetId::compute()` match spec literal `blake3(category + canonical_name + tier)`? [N — INTENTIONAL deviation; evidence below]

**No, by design.** Implementation interleaves a `|` byte between components:

```rust
// game/crates/cf-asset-ledger/src/entry.rs lines 41-48
pub fn compute(category: AssetCategory, canonical_name: &str, tier: ProductionTier) -> Self {
    let mut hasher = blake3::Hasher::new();
    hasher.update(category.as_str().as_bytes());
    hasher.update(b"|");
    hasher.update(canonical_name.as_bytes());
    hasher.update(b"|");
    hasher.update(tier.as_str().as_bytes());
    Self(hex::encode(hasher.finalize().as_bytes()))
}
```

vs. the spec line 62: `id: AssetId, // blake3(category + canonical_name + tier)`.

A literal interpretation (no seam) would have `blake3("WeaponSprite" ++ "iron_rifle_m1_side_v1" ++ "Tier1_SVG")`. The Rust impl inserts `|` seam bytes. **Resulting hash bytes differ from a literal-concatenation interpretation.**

The code comment justifies this: *"The seam character is `|` because `:` appears in pipeline ids and `/` appears in canonical_names; using `|` keeps the hash inputs unambiguous."* This is a defensive choice to prevent input-collision (e.g. `("Audio", "_SFX_xxx", "Tier1")` colliding with `("Audio_SFX", "xxx", "Tier1")` if no seam were used — though the current category set wouldn't have this collision, the seam future-proofs).

**Functional impact:** none. The id is computed deterministically + the hash is internal to the engine (never compared against a hash produced by a non-Rust producer). The four ID-related unit tests (`asset_id_is_deterministic`, `asset_id_differs_by_tier`, `asset_id_differs_by_canonical_name`, `asset_id_hex_is_64_chars`) all pass and confirm determinism + variation.

**Audit posture:** flag as INTENTIONAL DEVIATION from spec literal, justified in-code, no functional risk. Future spec update should rewrite line 62 to say `blake3(category + "|" + canonical_name + "|" + tier)` to remove the textual mismatch. The current rendering of the id is correctly anchored to the (category, canonical_name, tier) tuple per the spec's intent of "same name in same tier = same id; collision detection at-write" (Notes-for-implementer section).

---

## Closing Verdict

**PASS.**

Every spec field is present in the Rust struct with the correct type and the correct wire-format rename. All 7 critical wire renames the audit constraints called out (`Tier1_SVG`, `Audio_SFX`, `Mod_Custom`, `CC0`, `CC-BY`, `CC-BY-SA`, plus `Tier0_Placeholder`, `Tier2_ComfyUI`, `Tier2_Audio_Production`, `Tier3_Polish`, `Mod_Supplied`, `Narrative_Text`, `Localization_Strings`, `Tier1_LLM_Audio`) are present and verified. The schema is locked at `v1.0.0` via the `ASSET_ENTRY_SCHEMA_VERSION` constant + serde-default + JSON-schema enum + a dedicated unit test. Additive optional fields all use `serde(default)`; forward-compat for old readers of new entries is structurally sound. `extension_fields` is present, exposed via the builder, and the asymmetry between strict JSON-schema validation and lenient serde deserialization is intentional and consistent with the M4A "additive evolution without v-bump" contract.

The three documented deviations (`|` seam in `AssetId::compute`; spec text "16 categories" vs. 17-variant impl; spec `AssetKind` typed as Rust `String`) are intentional and either harmless or already justified in-code. The few MINOR findings (license capitalization, missing extension_fields round-trip test, no runtime JSON-schema enforcement) are documentation/test-coverage gaps, not behavioral bugs.

**Workspace test verification:** `cargo test -p cf-asset-ledger` → **46 passed; 0 failed**. Schema-version lock, id-determinism, wire-rename round-trip, JSONL append-only contract, and the integration-test schema-version-locked-v1 contract are all green.

No further A1-scope changes required for M4A schema closure.
