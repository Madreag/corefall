# A5 — Files & Content Audit

Audit of M4A milestone implementation against `specs/done/M4A.md`. Read-only;
no source files modified. Stat'd 13 May 2026.

## File Existence Matrix

| Spec Path | Exists? | Size (bytes) | Notes |
| --- | --- | --- | --- |
| `game/crates/cf-asset-ledger/src/lib.rs` | YES | 8627 | Public API exports (`add_entry`, `list_entries`, `regenerate_entry`, `verify_entry`); 4 unit tests inline |
| `game/crates/cf-asset-ledger/src/entry.rs` | YES | 16932 | Defines `AssetEntry` schema struct + `AssetEntryBuilder` + `ASSET_ENTRY_SCHEMA_VERSION = "1.0.0"` |
| `game/crates/cf-asset-ledger/src/storage.rs` | YES | 21994 | Append-only JSONL handle, `LedgerHandle`, `live_entries()`, `supersede_entry()`, `compact()`, `LedgerSummary` |
| `game/crates/cf-asset-ledger/src/regenerator.rs` | YES | 20758 | `regenerate_entry`, `regenerate_all`, `regenerate_with_cascade`, `mark_dependents_stale`, freeze-then-store path |
| `game/crates/cf-asset-ledger/src/integrity.rs` | YES | 11241 | blake3 streaming hash, `verify_entry`, `VerifyResult` w/ Fresh/Stale/Drifted/Missing/Failed |
| `game/crates/cf-asset-ledger/src/category.rs` | YES | 14880 | `AssetCategory` (17 variants), `ProductionTier` (7), `RegenStatus`, `License`, `PackageRef` enums |
| `game/crates/cf-asset-ledger/src/cli.rs` | YES | 26052 | `cmd_add` / `cmd_list` / `cmd_show` / `cmd_diff` / `cmd_verify` / `cmd_regenerate` / `cmd_summary` / `cmd_compact` |
| `game/crates/cf-mod/src/main.rs` | YES (MODIFY) | 47888 | `Cmd::Ledger { action: Box<LedgerAction> }` + `run_ledger()` dispatcher wired (lines 43-45, 213, 227-447) |
| `game/content/asset_ledger/ledger.jsonl` | YES | 1 | Empty placeholder (single `\n`); spec allows empty initial file |
| `game/content/asset_ledger/regen_manifest.ron` | YES | 5237 | 12 pipelines listed; parses cleanly via `ron 0.12.1` |
| `game/scripts/ledger_audit.sh` | YES | 1040 | Mode `0755` (`-rwxr-xr-x`); executable bit set |
| `game/crates/cf-asset-ledger/schemas/v1/asset_entry.schema.json` | YES | 5551 | Draft-2020-12; `additionalProperties: false`; 17-category + 7-tier enums match spec wire names |
| `game/crates/cf-asset-ledger/schemas/v1/regen_manifest.schema.json` | YES | 1870 | Draft-2020-12; required={schema_version, pipelines}; per-pipeline schema correct |

## Schema Files

### asset_entry.schema.json

- Required fields list: `[id, category, kind, canonical_name, tier, pipeline, generator, prompt, seed, output_path, output_format, output_size_bytes, output_blake3, generated_at_iso, generated_on_machine, regen_command]` (16 fields).
- Spec's `AssetEntry` declares 28 fields total. The following are NOT in the schema's `required` list but ARE in the spec struct (so they're either pure optionals or rely on `serde(default)` deserialization for backward compat):
  - `negative_prompt` (spec: `Option<String>` → schema permits `["string","null"]`) — **MATCH**
  - `palette_ref`, `style_lora`, `human_edit_notes`, `regen_validated_at`, `superseded_by`, `deprecated_at` — all spec'd as `Option<…>` → schema treats as optional with `"null"` allowed — **MATCH**
  - `upstream_assets`, `additional_outputs`, `regen_inputs` (Vec) — spec'd as plain `Vec<…>` (non-optional), but code uses `#[serde(default)]` (default = empty). Schema omits them from `required`. **MINOR DRIFT**: schema does not enforce presence even though spec struct declares them required. Practically harmless because default is the empty vec; flagged below as MINOR.
  - `generated_by_human` (spec: `bool`, non-optional) — schema omits from `required`; code uses `#[serde(default)]` (default = false). **MINOR DRIFT**.
  - `package_source` (spec: `PackageRef`, non-optional) — schema omits from `required`; code defaults to `Vanilla`. **MINOR DRIFT**.
  - `license` (spec: `License`, non-optional) — schema omits from `required`; code defaults to `CC0`. **MINOR DRIFT**.
  - `regen_status` (spec: `RegenStatus`, non-optional) — schema omits from `required`; code defaults to `Stale`. **MINOR DRIFT**.
  - `schema_version` (spec: `String`, non-optional) — schema omits from `required`; code defaults to `"1.0.0"`. **MINOR DRIFT** (acceptable because the enum still restricts the value to `["1.0.0"]` when present).
  - `extension_fields` (spec: `HashMap<String, Value>`, mod-extension surface) — schema permits the object with `additionalProperties: true`; not in required. **MATCH**.
- No extra fields beyond the spec. `additionalProperties: false` is strict.
- Enum wire names (`category`): all 17 match spec literally — `UiIcon`, `WeaponSprite`, `ActorSprite`, `VehicleSprite`, `ChassisSprite`, `BaseModuleSprite`, `TerrainTile`, `MaterialSwatch`, `Particle`, `Animation`, `Audio_SFX`, `Audio_Voice`, `Audio_Music`, `Narrative_Text`, `Localization_Strings`, `Cosmetic`, `Mod_Custom`. **MATCH**.
- Enum wire names (`tier`): all 7 match — `Tier0_Placeholder`, `Tier1_SVG`, `Tier1_LLM_Audio`, `Tier2_ComfyUI`, `Tier2_Audio_Production`, `Tier3_Polish`, `Mod_Supplied`. **MATCH**.
- Enum wire names (`regen_status`): all 5 match — `Fresh`, `Stale`, `Drifted`, `Missing`, `Failed`. **MATCH**.
- License enum: schema permits `CC0`, `CC-BY`, `CC-BY-SA`, `Proprietary` (string form) + `mod-supplied`/`custom` (object form with inner string). Spec text lists "CC0 / CC-BY / proprietary / mod-supplied" as examples; `CC-BY-SA` is an additive permissive extra, **NOT** a drift from spec.
- `additionalProperties` policy: **strict** on the outer object and on `generator`, `additional_outputs[]`, `regen_inputs[].AssetId`, `regen_inputs[].Path`, `package_source.{Mod,FactionPack}`, `license.{mod-supplied,custom}`. `extension_fields` is the documented mod-surface and permits arbitrary keys.

### regen_manifest.schema.json

- Schema parses: **YES** (well-formed Draft-2020-12 JSON).
- `required`: `[schema_version, pipelines]` — **MATCH** to the file contract.
- Per-pipeline `required`: `[pipeline_id, regen_command, model_version, deterministic]`. `owner_milestone`, `freeze_path_suffix`, `notes` are optional. **MATCH** to the file shape.
- `additionalProperties: false` on the outer object and each pipeline — strict, good for catching typos.

## regen_manifest.ron

- Parses as RON: **YES** (verified by spawning a temporary `cargo run` with `ron = "0.12.1"` + `serde` against the manifest file; deserialized cleanly into a 12-pipeline structure with all fields required-non-Option).
- Pipelines listed (12 total):
  1. `M9A_svg_v1` (M9A) — deterministic
  2. `M12A_llm_audio_v1` (M12A) — deterministic
  3. `M32A_comfyui_v1` (M32A) — non-deterministic / freeze-then-store
  4. `M37A_voice_v1` (M37A) — non-deterministic
  5. `M37A_music_v1` (M37A) — non-deterministic
  6. `M38A_localization_v1` (M38A) — deterministic
  7. `M18A_animation_v1` (M18A) — non-deterministic
  8. `M24A_particle_v1` (M24A) — deterministic
  9. `M25A_narrative_v1` (M25A) — deterministic
  10. `M45A_cosmetic_v1` (M45A) — deterministic
  11. `M48A_polish_v1` (M48A) — deterministic
  12. `Mod_Supplied_v1` (M4A) — non-deterministic (extra; not in spec list but useful)
- Spec-required milestones (from spec § "Notes for the implementer" — "Each downstream pipeline milestone (M9A, M12A, M18A, M24A, M25A, M32A, M37A, M38A, M45A, M48A, M48B) writes a ledger entry per generated asset"):
  - M9A ✓
  - M12A ✓
  - M18A ✓
  - M24A ✓
  - M25A ✓
  - M32A ✓
  - M37A ✓ (covered by both `M37A_voice_v1` and `M37A_music_v1`)
  - M38A ✓
  - M45A ✓
  - M48A ✓
  - **M48B ✗ — MISSING**
- Missing pipelines from spec: **M48B** (marketing assets pipeline; named in spec's "Crates / modules touched" + § Files preamble + § Notes/Per-pipeline integration).

## Content directory

- `game/content/asset_ledger/` exists: **YES**
- `ledger.jsonl` exists: **YES**
- Is `ledger.jsonl` empty (initial)? Effectively yes — 1 byte (a single `\n`). The spec explicitly states "ledger.jsonl can be empty per spec, but file must exist", and `wc -l` reports 0 entries. This matches the M4A acceptance criterion *"Given a fresh ledger / When `cf-mod ledger add` is invoked 100 times / Then ledger.jsonl contains 100 lines"*.

## Scripts directory

- `ledger_audit.sh` exists: **YES**
- Executable bit set: **YES**

  ```
  -rwxr-xr-x@ 1 erol  staff  1040 May 13 19:45 /Users/erol/projects/corefall/game/scripts/ledger_audit.sh
  ```

- `file(1)` confirms `Bourne-Again shell script text executable, ASCII text`.
- Runs without error (help mode):

  ```
  $ bash game/scripts/ledger_audit.sh --help
  #!/usr/bin/env bash
  # M4A: nightly ledger audit. Verifies that every entry in the canonical
  # `content/asset_ledger/ledger.jsonl` references an existing output_path
  # whose blake3 matches the ledger record. Drift / missing / failed entries
  # cause a non-zero exit.
  #
  # Usage:
  #   game/scripts/ledger_audit.sh         # verify all entries (strict)
  #   game/scripts/ledger_audit.sh --json  # emit JSON report on stdout
  #
  ...
  [Process exited with code 0]
  ```

  Exit 0 on `--help`. Script delegates the real audit work to `cargo run -p cf-mod -- ledger verify --strict --all` (or `--json` JSON-mode variant).

## Workspace plumbing

- `game/Cargo.toml` workspace.members includes `crates/cf-asset-ledger`: **YES**

  ```toml
  members = [
    "crates/cf-app",
    ...
    "crates/cf-mod",
    "crates/cf-asset-ledger",        # <— line 25
    "crates/cf-tools-editor",
    ...
  ]
  ```

- `game/crates/cf-mod/Cargo.toml` depends on cf-asset-ledger: **YES**

  ```toml
  [dependencies]
  ...
  cf-asset-ledger = { path = "../cf-asset-ledger" }
  cf-control      = { path = "../cf-control" }
  ...
  ```

- `game/crates/cf-control/Cargo.toml` depends on cf-asset-ledger: **YES**

  ```toml
  [dependencies]
  ...
  chrono            = { workspace = true }
  cf-asset-ledger   = { path = "../cf-asset-ledger" }
  cf-sim-core       = { path = "../cf-sim-core" }
  ...
  ```

- `game/crates/cfctl/Cargo.toml` depends on cf-asset-ledger: **YES**

  ```toml
  [dependencies]
  ...
  hex              = { workspace = true }
  cf-asset-ledger  = { path = "../cf-asset-ledger" }
  cf-control       = { path = "../cf-control" }
  ...
  ```

## Gaps (BLOCKER / MAJOR / MINOR)

1. **MAJOR — `regen_manifest.ron` is missing the M48B (marketing) pipeline.** The spec § "Notes for the implementer / Per-pipeline integration" explicitly lists `M48B` in the canonical set of downstream pipeline milestones that must write into the ledger. The manifest currently has 12 pipelines (M9A, M12A, M18A, M24A, M25A, M32A, M37A×2, M38A, M45A, M48A, Mod_Supplied) — M48B has no entry. Even if M48B is "marketing" and not strictly gameplay, the manifest is the source-of-truth registry per-pipeline; missing it makes any future M48B asset that's appended to the ledger fail the spec contract ("pipeline_id matches AssetEntry.pipeline").

2. **MINOR — `asset_entry.schema.json` `required` list omits 7 fields the spec struct declares as non-Option.** Specifically `upstream_assets`, `additional_outputs`, `regen_inputs`, `generated_by_human`, `package_source`, `license`, `regen_status`, `schema_version` are not in the schema's `required` array even though the Rust struct declares them as plain (non-`Option<…>`) fields. The Rust code mitigates this with `#[serde(default)]` so deserialization works fine, but the JSON schema is more permissive than the spec's narrative contract. Practical risk: a malformed mod-supplied JSONL entry could omit `regen_status` (or `schema_version`!) and still validate against the schema, losing information.

3. **MINOR — `Mod_Supplied_v1` is an extra pipeline not enumerated by the spec.** Adding it is reasonable (gives mod-supplied assets a manifest entry) but the spec did not list it as one of the 11 mandatory pipelines and there's no in-spec rationale. Document or remove.

4. **MINOR — `License` enum permits `CC-BY-SA` which is additive over the spec's example list.** Spec text lists "CC0 / CC-BY / proprietary / mod-supplied" with `/` suggesting examples rather than an exhaustive set, and SPDX-style `Custom(String)` covers anything else. This is acceptable but worth flagging as a permitted extension.

5. **MINOR — `ledger.jsonl` contains a single trailing newline (1 byte).** Spec says "ledger.jsonl can be empty per spec, but file must exist" — a 0-byte file would be equally valid. The reader (`storage::read_all`) skips blank lines, so the trailing newline is harmless and `wc -l` reports 0 entries.

## Recommended Fixes

1. **(MAJOR)** Append a `M48B_marketing_v1` pipeline entry to `game/content/asset_ledger/regen_manifest.ron` with `owner_milestone: "M48B"`, an appropriate `regen_command` placeholder, a pinned `model_version`, and `deterministic: false` (marketing assets are likely freeze-then-store like other Tier 2 production paths). Example addition:

   ```ron
   (
       pipeline_id: "M48B_marketing_v1",
       owner_milestone: "M48B",
       regen_command: "cf-tools-marketing --asset-id $ASSET_ID --seed $SEED --out $OUTPUT_PATH",
       model_version: "marketing-pack-v1",
       deterministic: false,
       freeze_path_suffix: ".frozen",
       notes: "Marketing assets (key art, social cards, Steam capsules); freeze-then-store.",
   ),
   ```

2. **(MINOR)** Tighten `asset_entry.schema.json` `required` to include the 7 spec-mandatory non-Option fields (`upstream_assets`, `additional_outputs`, `regen_inputs`, `generated_by_human`, `package_source`, `license`, `regen_status`, `schema_version`). This forces ledger lines to carry the full envelope and prevents mods from quietly dropping fields. Alternatively, document in `lib.rs` that these are "soft-optional with engine defaults" per a deliberate forward-compat decision.

3. **(MINOR)** Either add a spec note documenting `Mod_Supplied_v1` as the canonical pipeline-id for mod-supplied assets (so `cf-mod ledger add` for mod content can declare `--pipeline Mod_Supplied_v1`), or remove the entry from the manifest. The current manifest line + comment ("Mod-supplied content uses freeze-then-store; mod author asserts determinism") suggests deliberate design — promote it to spec language in the next iteration.

4. **(OPTIONAL)** Consider truncating `ledger.jsonl` to 0 bytes (rather than 1 trailing newline) so `wc -c` reports `0` and the file matches the canonical "empty append-only journal" state. Cosmetic only; not blocking.
