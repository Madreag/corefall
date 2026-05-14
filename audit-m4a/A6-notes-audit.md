# A6 — Notes, Architecture, Decisions, Closure Audit

Scope: `specs/done/M4A.md` § Notes for the implementer + Decision-record alignment + Closure procedure + Out of scope, cross-checked against `game/crates/cf-asset-ledger/`, `game/crates/cf-mod/`, `game/content/asset_ledger/`, `game/scripts/`, and `docs/plan/decisions/`.

## Architecture Rules

1. **Append-only ledger**: PARTIAL — append path is correct (`LedgerHandle::append` only ever uses `OpenOptions::append`), and the spec's documented mutation (re-generation supersede) is implemented in `storage::supersede_entry` (storage.rs:155-205) which IS the rewrite mutation the spec sanctioned. HOWEVER, a SECOND mutation surface exists: `regenerator::mark_dependents_stale` → `rewrite_with_status` (regenerator.rs:268-296) also truncates + rewrites the entire ledger to flip `regen_status` to `Stale` on dependents. Storage's docstring claims supersede is "the ONLY sanctioned post-write mutation" while regenerator's docstring claims `rewrite_with_status` is "the only sanctioned mutation surface" — direct contradiction. The behaviour (flipping a status field) is not strictly append-only.
2. **JSONL not JSON**: PASS — one entry per line, `\n` separator, each append re-opens with `O_APPEND` (storage.rs:81-99). `tail -f` works (covered by `append_creates_jsonl_one_entry_per_line`). NOTE: the spec text says "concurrent-write-safe via fcntl"; the implementation relies on POSIX `O_APPEND` atomicity for sub-PIPE_BUF writes (documented in storage.rs:11-13 and 81-87) rather than an `flock`/`fcntl` advisory lock. This is a generally accepted substitute for line-sized JSONL writes but is a literal deviation from the spec.
3. **blake3 not sha256**: PASS — `blake3 = { workspace = true }` in `cf-asset-ledger/Cargo.toml`; `blake3::Hasher` used in `entry::AssetId::compute` and `integrity::hash_path`. No `sha256`/`sha2` reference anywhere in the crate.
4. **AssetId formula**: PASS with documented deviation — spec literal is `blake3(category + canonical_name + tier)` with no seam; implementation uses `blake3(category|canonical_name|tier)` (`|` byte between fields). The deviation is explicitly documented in `entry.rs:43-46` ("The seam character is `|` because `:` appears in pipeline ids and `/` appears in canonical_names; using `|` keeps the hash inputs unambiguous"). The seam is SAFER (avoids collisions like `"ab"+"c"` vs `"a"+"bc"`). **Recommendation: keep `|` seam**; it is the more defensible choice and `validate_entry_json` recomputes with the same formula so there is no drift between writer and validator.
5. **Pipeline id namespaced**: PARTIAL — `regen_manifest.ron` lists `M9A_svg_v1`, `M12A_llm_audio_v1`, `M32A_comfyui_v1`, `M37A_voice_v1`, `M37A_music_v1`, `M38A_localization_v1`, `M18A_animation_v1`, `M24A_particle_v1`, `M25A_narrative_v1`, `M45A_cosmetic_v1`, `M48A_polish_v1`, `Mod_Supplied_v1`. **Missing: `M48B_*` (marketing)** — the spec lists M48B as a pipeline that writes to this ledger but it has no entry in the manifest.
6. **Determinism gate (freeze-then-store)**: PASS — `regenerator::rebake_from_freeze` (regenerator.rs:115-148) materializes the canonical output from `<output_path>.frozen`; `snapshot_freeze` writes a fresh freeze. `regenerate_entry` defaults to freeze-path when no pipeline runner is passed. Test `freeze_then_store_round_trip` proves the round-trip; integration test `regenerate_byte_identical` proves CLI-level byte identity after corruption.

## Schema Design

- **Locked at v1.0.0**: PASS — `ASSET_ENTRY_SCHEMA_VERSION = "1.0.0"` (entry.rs:18); test `schema_version_locked_at_v1`; `validate_entry_json` rejects schema drift (test `validate_entry_json_rejects_schema_drift`); JSON schema enum locks to `["1.0.0"]`.
- **Forward-compat (serde default)**: PASS — every optional field uses `#[serde(default ...)]`: `negative_prompt`, `palette_ref`, `style_lora`, `upstream_assets`, `additional_outputs`, `generated_by_human`, `human_edit_notes`, `package_source`, `license`, `regen_inputs`, `regen_validated_at`, `regen_status`, `superseded_by`, `deprecated_at`, `schema_version`, `extension_fields` (entry.rs:107-160).
- **Mod-extension fields**: PASS in code, INCONSISTENT in JSON schema — `extension_fields: BTreeMap<String, serde_json::Value>` is present on `AssetEntry` (entry.rs:159-160). The Rust struct does NOT use `#[serde(deny_unknown_fields)]`, so unknown top-level keys are silently dropped (engine "ignores unknown fields" — matches spec). However the JSON-Schema artifact at `schemas/v1/asset_entry.schema.json` has `additionalProperties: false` at the root, which would REJECT unknown top-level keys if anyone runs the schema as a strict gate. The schema is currently documentation-only (no runtime validator references it), so this is paper-only inconsistency, but a tool that adopts the schema later will diverge from the Rust runtime. Mods are expected to stash extra metadata in `extension_fields` per spec; the catch-all `extension_fields` is properly typed as `additionalProperties: true` in the schema.

## Per-Pipeline Integration

- **Pipelines list complete in regen_manifest.ron**: 11/12 from spec list; **MISSING: M48B (marketing)**. Mod-supplied catch-all also present (`Mod_Supplied_v1`).
- **`cf-mod ledger add` validates output_path exists pre-write**: PASS — `cli.rs:137-142` bails with "output file does not exist at <path>; produce the asset before calling `cf-mod ledger add`" before any append happens.
- **`cf-mod ledger add` fails fast on missing output_path**: PASS — `anyhow::bail!` returns non-zero before `handle.append(...)`; no partial entry is written. Pipeline tools that error out cannot leak a half-written ledger line.

## CI Integration

- **ledger_audit.sh nightly**: PASS — `game/scripts/ledger_audit.sh` runs `cargo run --release -p cf-mod -- ledger verify --strict --all` (with `--json` mode supported). Sets `set -euo pipefail` so any drift / missing / failed exits non-zero.
- **Pre-commit hook**: DEFERRED — no pre-commit infrastructure exists in the repo; the spec's hook is optional and not required for M4A closure. Implementing it later is a one-line addition (`cf-mod ledger verify --strict --all`).
- **Release CI**: DEFERRED — no GH Actions / GitLab CI configuration in the repo. `ledger_audit.sh` is the canonical script and can be wired into Release CI when CI infrastructure lands. The `cf-mod ledger regenerate --all` path works end-to-end (test `full_re_bake_from_scratch_is_idempotent`) and only awaits CI plumbing.
- **Mod-CI**: DEFERRED — Steam Workshop integration is M33+ scope; out of scope for M4A.

## Pitfalls

- **Pipeline forgetting to call `add`**: PARTIAL — `cf-mod validate` (`run_validate` → `walk` → `validate_ledger_jsonl`) confirms every line in `ledger.jsonl` is schema-valid, but there is no check that walks `content/assets/` and warns about files that have no corresponding ledger entry. The spec describes a future "untracked content/assets/" CI warning; this is documentation-only at M4A.
- **Hand-edit drift**: COVERED — `ledger verify --strict --all` (and `--strict-status` on the `Verify` subcommand) exits non-zero on `Drifted`. Test `verify_detects_drift_non_zero_exit` proves the path.
- **GPU non-determinism**: COVERED — freeze-then-store is the default regen path; non-deterministic pipelines (`M32A_comfyui_v1`, `M37A_voice_v1`, `M37A_music_v1`, `M18A_animation_v1`, `Mod_Supplied_v1`) carry `deterministic: false` in `regen_manifest.ron` and rely on `<output_path>.frozen`.
- **Schema field churn**: LOCKED — `ASSET_ENTRY_SCHEMA_VERSION = "1.0.0"`, validator rejects mismatch, JSON schema enum constrains to `["1.0.0"]`. Future additive fields only need `#[serde(default)]`; layout-breaking changes require the M39 migration shim per spec.
- **Ledger size**: COVERED — `cf-mod ledger compact --keep-latest --before <ts>` exists (`cli::cmd_compact` → `storage::LedgerHandle::compact`), drops superseded history, writes a `.bak` backup. Git LFS recommendation is documentation; not enforced by tooling.
- **Concurrent writes**: COVERED-VIA-SUBSTITUTE — POSIX `O_APPEND` atomicity is used in place of fcntl advisory locking. Storage docstring (storage.rs:11-13, 81-87) explicitly documents the substitution. Per-mod sub-ledgers + build-end merge (the other half of the spec's recommendation) is NOT implemented; this is acceptable because per-mod ledger paths are supported via `LedgerPaths`/`--ledger-path`, but the merge tool isn't built (deferred to M33+ workshop scope per Mod-CI).

## Decision-Record Alignment

- **DR-053 (asset ledger + AI-generated traceability)**: PARTIAL — `docs/plan/decisions/dr-053-ai-audio-pipeline-realtime-and-generative.md` exists, marked `status: closed-direction` (closed 2026-05-06). The spec's closure procedure step 3 calls for status → `CLOSED-DIRECTION-WITH-EVIDENCE`; this is a procedural follow-up not reflected in the on-disk DR yet.
- **DR-044 (audiovisual production pipeline)**: PASS — `dr-044-audiovisual-production-pipeline.md` present; M4A as foundation acknowledged.
- **DR-006 (mod parity)**: PASS — `dr-006-modding-data-model.md` present; ledger has `AssetCategory::ModCustom`, `ProductionTier::ModSupplied`, `PackageRef::Mod(String)` / `FactionPack(String)`, `License::ModSupplied(String)`; CLI accepts `--package-source mod:<id>`; `cf-mod validate` runs the same ledger validator on mod-supplied ledgers as on vanilla.
- **DR-024 (native engine stack, pure Rust, no Python in core)**: PASS — `cf-asset-ledger/Cargo.toml` lists ONLY Rust deps (`anyhow`, `blake3`, `chrono`, `clap`, `hex`, `ron`, `serde`, `serde_json`, `thiserror`, `tracing`, `tracing-subscriber`). No `pyo3`, no shelling-out to Python in the core; Python is reserved for downstream pipeline tools (M9A/M32A/M37A) per spec.

## Closure Procedure

1. **Reference bundle `prototype_runs/native/m4a_<UTC>_<hash>/`**: DEFERRED — 61 `m4a_*` bundles exist under `prototype_runs/native/` but they were captured 5/10-5/11 from the PRIOR M4A scope (accessibility / focus traversal / hold-to-confirm); the asset-ledger crate landed 5/13. No reference bundle was captured for the asset-ledger M4A scope. Procedural follow-up; not a code gap.
2. **Self-play sweep rows**:
   - `m4a_ledger_add`: NO cfctl script; **COVERED by integration test** `append_only_one_line_per_entry` + every other test in `game/crates/cf-mod/tests/ledger_cli_integration.rs`.
   - `m4a_ledger_list`: NO cfctl script; COVERED by integration test `list_filters_category_tier`.
   - `m4a_ledger_regenerate`: NO cfctl script; COVERED by `regenerate_byte_identical` + `full_re_bake_from_scratch_is_idempotent`.
   - `m4a_ledger_verify`: NO cfctl script; COVERED by `verify_detects_drift_non_zero_exit` + `summary_groups_status`.
   - `m4a_ledger_drift_detection`: NO cfctl script; COVERED by `verify_detects_drift_non_zero_exit`.
   - `m4a_ledger_mod_pack_integration`: NO cfctl script and **NO integration test**. `cf-mod package` is stubbed ("lands at M5/M8"). The Gherkin "Mod pack integration" scenario (`cf-mod package` registers every mod asset as a ledger entry; mod manifest references ledger entry ids; copy + verify on install) is the only acceptance criterion not exercised by code. PackageRef::Mod + Mod_Supplied tier + ModCustom category exist as schema surface, but the publisher that USES them isn't built.
   - `m4a_universal_done_criteria`: NO cfctl script; the only existing M4A asset-ledger cfctl scenario is `m4a_ledger_summary.cfctl.json` which smoke-tests the `observe.assets.ledger_summary` JSON-RPC surface.
3. **DR-053 doc update to `CLOSED-DIRECTION-WITH-EVIDENCE`**: DEFERRED to procedural follow-up; DR file still says `closed-direction`.
4. **Register AssetEntry schema in M39 manifest**: DEFERRED to M39 close per spec.
5. **Move M4A → done/**: PASS — `specs/done/M4A.md` exists with `## Status` field set to `done`; `specs/active/M4A.md` deleted per `git status` (`D specs/active/M4A.md`).

## Out of scope (verified NOT implemented)

- **Actual asset generation pipelines (M9A/M12A/M32A/M37A/M38A/...)**: VERIFIED NOT IMPLEMENTED — no `cf-tools-svg-gen`, `cf-tools-comfyui`, `cf-tools-voice-gen`, `cf-tools-music-gen`, etc. exist in `game/crates/`. The regen-manifest references these commands as future tools.
- **Mod content moderation (M36 / M49)**: NOT IMPLEMENTED — correct.
- **Cosmetic-locker entitlement tracking (M45A + M49)**: NOT IMPLEMENTED — correct.
- **Steam Workshop publishing UI (M33 + M36A)**: NOT IMPLEMENTED — correct.
- **Asset versioning conflict resolution (M33)**: NOT IMPLEMENTED — correct.
- **CDN / asset delivery for online mods**: NOT IMPLEMENTED — correct.
- **Per-asset license verification**: VERIFIED — `License` enum stored on every entry (`Cc0` / `CcBy` / `CcBySa` / `Proprietary` / `ModSupplied(String)` / `Custom(String)`); no code path verifies, contacts a license server, or rejects entries by license. Author-declared, not engine-checked, per spec.
- **AI model version-pinning enforcement**: VERIFIED — `GeneratorRef::model_version: Option<String>` stored; `regen_manifest.ron` carries `model_version` per pipeline; the regenerator does not enforce model-version pinning beyond `output_blake3` mismatch detection. Pipelines own enforcement, per spec.

## Gaps (BLOCKER / MAJOR / MINOR)

- **BLOCKER**: none. Architecture is intact, append/JSONL/blake3/determinism are correct, schema is locked.
- **MAJOR**:
  - **Closure-procedure cfctl sweep rows missing**: 6 of 7 sweep rows from the closure procedure (`m4a_ledger_add`, `_list`, `_regenerate`, `_verify`, `_drift_detection`, `_mod_pack_integration`, `_universal_done_criteria`) have no `.cfctl.json` script. Five are functionally covered by `ledger_cli_integration.rs` integration tests, but the spec's per-row sweep evidence (via cfctl) doesn't exist on disk. The one cfctl that does exist (`m4a_ledger_summary.cfctl.json`) is for the `observe.assets.ledger_summary` JSON-RPC surface, not in the closure-procedure list.
  - **`m4a_ledger_mod_pack_integration` has NO test coverage at all**: the Gherkin "Mod pack integration" acceptance scenario requires `cf-mod package` to auto-register mod assets in the ledger; `cf-mod package` is stubbed in `cf-mod/src/main.rs` ("not implemented in M0; package builder lands at M5/M8"). Schema surface exists; publisher does not.
  - **Reference bundle for asset-ledger M4A not captured**: the 61 `m4a_*` bundles under `prototype_runs/native/` are from the prior accessibility scope (5/10-5/11); no fresh bundle was captured for the 5/13 asset-ledger landing.
- **MINOR**:
  - **`M48B_marketing_*` pipeline missing from `regen_manifest.ron`**: the spec lists M48B as a writer; the manifest has 11/12 of the named pipelines but skips M48B.
  - **Append-only contract has TWO mutation surfaces, not one**: `storage::supersede_entry` (sanctioned by spec) AND `regenerator::rewrite_with_status` (used by `mark_dependents_stale`). Both rewrite the whole file. Docstrings in both modules each claim to be "the only sanctioned mutation surface" — direct contradiction. The spec authorises supersede only; flipping `regen_status` to `Stale` on dependents is a useful behaviour but is not in the spec's append-only contract.
  - **fcntl advisory locking not used**: the spec wording "concurrent-write-safe via fcntl" is replaced by POSIX `O_APPEND` atomicity. Documented in `storage.rs` but is a literal deviation. Real concurrent CI workers running on the same NFS / SMB share could in theory race; in practice for local-disk JSONL append it's fine.
  - **JSON-Schema artifact strict (`additionalProperties: false`) but Rust runtime permissive**: paper-only inconsistency. No runtime tool consumes the JSON schema; `validate_entry_json` uses serde which silently ignores unknown top-level keys, which is what the spec asks for.
  - **DR-053 status not flipped to `CLOSED-DIRECTION-WITH-EVIDENCE`**: spec closure step 3, procedural follow-up.

## Recommended Fixes

1. **MAJOR — Add M48B marketing pipeline** to `game/content/asset_ledger/regen_manifest.ron`:
   ```ron
   (
       pipeline_id: "M48B_marketing_v1",
       owner_milestone: "M48B",
       regen_command: "cf-tools-marketing --asset-id $ASSET_ID --out $OUTPUT_PATH",
       model_version: "marketing-pack-v1",
       deterministic: false,
       freeze_path_suffix: ".frozen",
       notes: "Marketing assets (screenshots, hero shots, social images) use freeze-then-store.",
   ),
   ```
2. **MAJOR — Land the closure cfctl sweep**: ship `m4a_ledger_add.cfctl.json` / `_list` / `_regenerate` / `_verify` / `_drift_detection` / `_universal_done_criteria` that drive the cf-mod CLI through each verb against a temp workspace. Or, explicitly note in the closure file that integration tests in `cf-mod/tests/ledger_cli_integration.rs` are the canonical evidence.
3. **MAJOR — Decide mod-pack integration**: either (a) implement `cf-mod package` minimally so it auto-registers mod assets into the ledger and ship `m4a_ledger_mod_pack_integration` evidence, or (b) explicitly cite the "Mod pack integration" Gherkin scenario as deferred-to-M5/M8 with a documented rationale.
4. **MAJOR — Capture a reference bundle**: run cf-headless / cfctl over a small ledger fixture (add 50 sample entries, verify all-Fresh, regenerate produces byte-identical output) and write the bundle into `prototype_runs/native/m4a_<UTC>_<hash>/`.
5. **MINOR — Resolve append-only contradiction**: either (a) keep `mark_dependents_stale` as a NEW mutation surface and update `storage::supersede_entry` docstring to say "one of two sanctioned mutation surfaces", or (b) reimplement `mark_dependents_stale` as a pure-read function that returns IDs without touching the ledger, leaving status flipping to a future "stale-marker append" entry type. Option (b) preserves the spec literal.
6. **MINOR — Document POSIX-append-atomicity substitution** prominently (it's there in the source, but worth a SECURITY/CI note that NFS / SMB workers must NOT share a ledger across machines without explicit per-mod sub-ledgers + a merge step).
7. **MINOR — Align JSON schema with Rust runtime**: change `additionalProperties: false` → `additionalProperties: true` on the AssetEntry root, OR add `#[serde(deny_unknown_fields)]` on the Rust struct and force all mod-extension data through `extension_fields`. The current pair is inconsistent.
8. **MINOR — Flip DR-053 to `CLOSED-DIRECTION-WITH-EVIDENCE`** in `docs/plan/decisions/dr-053-ai-audio-pipeline-realtime-and-generative.md` once the reference bundle lands.
9. **KEEP the `|` seam in `AssetId::compute`**: it is documented, safer, and `validate_entry_json` agrees with the writer. Removing it to match the spec literal would (re-)introduce a collision class.
