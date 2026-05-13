# M4A — Asset Ledger Infrastructure

## Status

`active`

## Intent

**M4A is the asset-ledger foundation** — the `cf-asset-ledger` crate that every AI-generated asset (SVG placeholder, ComfyUI sprite, Stable Audio Open SFX, voice clip, music track, localized string table, codex entry, animation frame, VFX particle config, mod-supplied asset) writes regeneration metadata into. Without this, BP12 polish cannot reproduce assets deterministically, modders cannot ship reproducible asset packs, and the project violates DR-053 (asset traceability).

M4A is the **production-track foundation milestone**. Every subsequent asset-pipeline milestone (M9A SVG, M12A audio, M18A animation, M24A VFX, M25A narrative, M32A ComfyUI, M37A voice/music, M38A localization, M45A cosmetic, M48A polish, M48B marketing) writes into the ledger this milestone establishes. The ledger is **the single source of truth** for "how did this asset come to exist and how do I regenerate it byte-identically?"

M4A promise: **"every byte of game art / sound / text is reproducible from prompt + seed + model_version recorded in the ledger; nothing is hand-crafted-and-lost; the entire content roster can be re-baked from scratch on a fresh checkout."**

## Player-facing behavior

(M4A is infrastructure, not directly player-facing — but it underpins every visible asset.)

- **Modders can read the ledger** to understand HOW any asset was made + reproduce it locally
- **Re-baking the full content roster** is a single command (`cf-asset-ledger regenerate --all`) which produces byte-identical output (assuming pinned model versions)
- **Asset attribution** is queryable per asset: who/what generated it + when + with what prompt + what model
- **Per-asset license** field (most are CC0-equivalent player-owned; mod assets carry their own license metadata)

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-asset-ledger` | NEW | Append-only JSONL ledger + per-entry schema + CLI + regenerator engine + integrity check |
| `cf-mod` | MODIFY | Add `cf-mod ledger {add,list,diff,regenerate,verify}` subcommands; mod-pack publisher writes ledger entries automatically |
| `cf-replay` | MODIFY | When a run bundle references an asset (capture grid screenshots, audio playback), the `asset_ref` field links to a ledger entry id |
| `cf-control` | MODIFY | `observe.assets.ledger_summary` returns total count + per-category counts + per-pipeline-tier counts + missing-entry warnings |

## Files

Source:
- `game/crates/cf-asset-ledger/src/lib.rs` (NEW)
- `game/crates/cf-asset-ledger/src/entry.rs` (NEW: AssetEntry schema)
- `game/crates/cf-asset-ledger/src/storage.rs` (NEW: append-only JSONL writer + reader)
- `game/crates/cf-asset-ledger/src/regenerator.rs` (NEW: re-bake from prompt + seed + model_version)
- `game/crates/cf-asset-ledger/src/integrity.rs` (NEW: per-entry blake3 verification + drift detection)
- `game/crates/cf-asset-ledger/src/category.rs` (NEW: asset category enum + extension registry)
- `game/crates/cf-asset-ledger/src/cli.rs` (NEW: CLI for `cf-mod ledger ...`)
- `game/crates/cf-mod/src/main.rs` (MODIFY: ledger subcommands + automatic mod-pack entry registration)

Content + scripts:
- `game/content/asset_ledger/ledger.jsonl` (NEW: append-only ledger; one entry per line)
- `game/content/asset_ledger/regen_manifest.ron` (NEW: per-pipeline regen scripts + model_version pin)
- `game/scripts/ledger_audit.sh` (NEW: nightly audit script — verifies all referenced assets exist + integrity matches)

Schemas:
- `game/crates/cf-asset-ledger/schemas/v1/asset_entry.schema.json` (NEW: locked v1 schema for ledger entries)
- `game/crates/cf-asset-ledger/schemas/v1/regen_manifest.schema.json` (NEW)

## Asset entry schema (locked v1)

Every entry in `content/asset_ledger/ledger.jsonl` conforms to this schema:

```rust
pub struct AssetEntry {
    // Identity
    pub id: AssetId,                                 // blake3(category + canonical_name + tier)
    pub category: AssetCategory,                     // 16 categories below
    pub kind: AssetKind,                             // sub-category (e.g. weapon-side, ui-icon)
    pub canonical_name: String,                      // human-readable id (e.g. "iron_rifle_m1_side_v2")
    pub tier: ProductionTier,                        // Tier0 / Tier1_SVG / Tier1_LLM_Audio / Tier2_ComfyUI / Tier2_Audio / Tier3_Polish
    
    // Production metadata
    pub pipeline: PipelineId,                        // e.g. "M9A_svg_v1", "M32A_comfyui_v1"
    pub generator: GeneratorRef,                     // e.g. {tool: "ComfyUI", model: "Flux.1-dev", workflow: "weapon_side_v3.json"}
    pub prompt: String,                              // canonical prompt (sanitized; deterministic)
    pub negative_prompt: Option<String>,             // for diffusion models
    pub seed: u64,                                   // RNG seed for deterministic regen
    pub palette_ref: Option<PaletteId>,              // reference to palette JSON used
    pub style_lora: Option<LoraRef>,                 // per-faction style LoRA reference (M32A+)
    pub upstream_assets: Vec<AssetId>,               // dependencies (e.g. Tier 2 uses Tier 1 as ControlNet input)
    
    // Output
    pub output_path: PathBuf,                        // relative to content/assets/
    pub output_format: String,                       // "svg", "png", "ogg", "webp", "json", "ron"
    pub output_size_bytes: u64,
    pub output_blake3: String,                       // hex blake3 hash for drift detection
    pub additional_outputs: Vec<AdditionalOutput>,   // normal maps, mipmaps, alternate resolutions
    
    // Provenance
    pub generated_at_iso: String,                    // RFC 3339 timestamp
    pub generated_on_machine: String,                // hostname (for tracing CI vs local)
    pub generated_by_human: bool,                    // true if a human polished by hand; false = pure pipeline output
    pub human_edit_notes: Option<String>,            // if human_edit=true, what they changed
    pub package_source: PackageRef,                  // vanilla / mod / faction-pack
    pub license: License,                            // CC0 / CC-BY / proprietary / mod-supplied
    
    // Regeneration
    pub regen_command: String,                       // exact CLI command to reproduce
    pub regen_inputs: Vec<RegenInputRef>,            // ordered list of dependencies for regen
    pub regen_validated_at: Option<String>,          // last successful regen check (RFC 3339)
    pub regen_status: RegenStatus,                   // Fresh / Stale / Drifted / Missing / Failed
    
    // Lifecycle
    pub superseded_by: Option<AssetId>,              // when Tier 2 replaces Tier 1, link forward
    pub deprecated_at: Option<String>,
    pub schema_version: String,                      // "1.0.0" — additive only
}

pub enum AssetCategory {
    UiIcon,                                          // HUD icons, menu icons, action prompt glyphs
    WeaponSprite,                                    // weapon side-view sprites (per M9A; refined M32A)
    ActorSprite,                                     // actor side-view sprites + walk frames
    VehicleSprite,                                   // vehicle side-view sprites + boarding states
    ChassisSprite,                                   // chassis silhouettes per weight class
    BaseModuleSprite,                                // turret, pump, valve, generator, etc.
    TerrainTile,                                     // material tiles + integrity-band variants
    MaterialSwatch,                                  // material registry swatches + overlay tints
    Particle,                                        // VFX particle textures (per M24A)
    Animation,                                       // animation frame strips (per M18A)
    Audio_SFX,                                       // sound effects (per M12A + M37A)
    Audio_Voice,                                     // voice samples (per M37A)
    Audio_Music,                                     // music tracks (per M37A)
    Narrative_Text,                                  // codex entries + dialog + lore (per M25A)
    Localization_Strings,                            // per-language string tables (per M38A)
    Cosmetic,                                        // cosmetic skins + scars + faction variants (per M45A)
    Mod_Custom,                                      // mod-supplied; modder declares category
}

pub enum ProductionTier {
    Tier0_Placeholder,                               // hand-coded colored rectangle / sine wave
    Tier1_SVG,                                       // M9A: SVG + LLM-prompted shape generation
    Tier1_LLM_Audio,                                 // M12A: LLM-generated SFX placeholder
    Tier2_ComfyUI,                                   // M32A: SDXL/Flux/AnimateDiff production-quality
    Tier2_Audio_Production,                          // M37A: Stable Audio Open production + voice synth
    Tier3_Polish,                                    // M48A: hand-tweaked / final mix / Aseprite-touched
    Mod_Supplied,                                    // mod-author-supplied; no tier; trust per package
}

pub enum RegenStatus {
    Fresh,                                           // entry matches output_path's current blake3
    Stale,                                           // entry exists but never validated (CI hasn't re-baked recently)
    Drifted,                                         // entry's blake3 doesn't match output_path's current hash (assets edited outside pipeline)
    Missing,                                         // output_path doesn't exist on disk
    Failed,                                          // most recent regen attempt errored
}
```

## CLI surface

```bash
# Add a new entry (called by every pipeline tool — M9A, M12A, etc.)
cf-mod ledger add \
    --category WeaponSprite \
    --kind weapon-side \
    --canonical-name "iron_rifle_m1_side_v1" \
    --tier Tier1_SVG \
    --pipeline M9A_svg_v1 \
    --prompt "industrial rifle, side-profile, dark steel, 32x16 px" \
    --seed 1234 \
    --output-path content/assets/placeholders/weapons/iron_rifle_m1_side.svg

# List entries with filters
cf-mod ledger list --category WeaponSprite --tier Tier1_SVG
cf-mod ledger list --pipeline M32A_comfyui_v1 --status Fresh

# Show entry details
cf-mod ledger show <asset_id>

# Diff: compare ledger metadata vs actual disk state
cf-mod ledger diff --all
cf-mod ledger diff <asset_id>

# Regenerate from ledger entry (re-runs the pipeline)
cf-mod ledger regenerate <asset_id>
cf-mod ledger regenerate --category WeaponSprite --tier Tier1_SVG
cf-mod ledger regenerate --all                                # full re-bake

# Verify integrity (re-hash and compare; flags Drifted entries)
cf-mod ledger verify --all
cf-mod ledger verify --strict                                 # CI mode: exit non-zero on any drift

# Audit summary (counts + status)
cf-mod ledger summary
# Prints:
#   Total entries: 4827
#   By category: UiIcon=84, WeaponSprite=210, ActorSprite=176, ...
#   By tier: Tier0=12, Tier1_SVG=2104, Tier1_LLM_Audio=412, Tier2_ComfyUI=1843, Tier2_Audio=298, Tier3_Polish=158
#   Status: Fresh=4801, Stale=18, Drifted=4, Missing=3, Failed=1
#   Missing: [list of asset_ids]
#   Drifted: [list of asset_ids]
```

## Acceptance criteria

```gherkin
Scenario: cf-asset-ledger crate ships
  Given M4A closure
  Then `cf-asset-ledger` crate exists in game/crates/
  And exports public API: `add_entry`, `list_entries`, `regenerate_entry`, `verify_entry`
  And the AssetEntry schema is locked at v1.0.0

Scenario: Append-only JSONL ledger
  Given a fresh ledger
  When `cf-mod ledger add` is invoked 100 times
  Then ledger.jsonl contains 100 lines (one entry per line)
  And no line is ever modified post-write (append-only contract)
  When the same asset is re-generated:
    Then a NEW entry is appended (NOT overwrite)
    And the old entry is marked `superseded_by = <new_entry_id>`
  And the file can be tailed to see new entries (works with `tail -f`)

Scenario: Integrity check detects drift
  Given an asset whose output_path content is modified outside the pipeline (e.g. hand-edited)
  When `cf-mod ledger verify <asset_id>` runs
  Then status = Drifted
  And the difference between ledger blake3 and current file blake3 is reported
  Exit code is non-zero

Scenario: Regenerate produces byte-identical output
  Given a Tier 1 SVG asset with pinned pipeline + seed + model_version
  When `cf-mod ledger regenerate <asset_id>` runs
  Then the regenerated output's blake3 matches the original ledger blake3
  And the file is byte-identical
  (Determinism contract: same prompt + same seed + same model_version = same bytes)

Scenario: Full re-bake from scratch
  Given a fresh checkout (no content/assets/ directory)
  When `cf-mod ledger regenerate --all` runs
  Then every entry in ledger.jsonl is regenerated
  And every output_path file exists with correct blake3
  And exit code is 0 if no failures
  And the operation is idempotent (running twice yields no change on second pass)

Scenario: Per-category + per-tier filtering
  Given a ledger with 5 categories and 4 tiers
  When `cf-mod ledger list --category WeaponSprite --tier Tier2_ComfyUI` runs
  Then output is only entries matching both filters
  When `--status Drifted` is passed:
    Then output is filtered to drifted entries

Scenario: Mod pack integration
  Given a mod author's `.cfmod` package
  When the mod is packaged via `cf-mod package`:
    Then every asset in the mod is registered as a new ledger entry
    And category = Mod_Custom; package_source = mod_id
    And the mod's manifest references ledger entry ids (NOT raw file paths)
  When the mod is installed by another player:
    Then ledger entries are copied to local ledger
    And blake3 integrity verified on install

Scenario: Upstream asset dependency graph
  Given a Tier 2 ComfyUI sprite that uses a Tier 1 SVG as ControlNet input
  Then the Tier 2 entry's `upstream_assets` field includes the Tier 1 entry's id
  When the Tier 1 entry is regenerated:
    Then dependents (Tier 2, Tier 3) are marked Stale
    And `cf-mod ledger regenerate --cascade <tier1_id>` regenerates the entire downstream graph

Scenario: Schema version locked at v1
  Given M4A closes with AssetEntry schema v1.0.0
  Then the schema is registered in M39's manifest of locked schemas
  Future schema bumps require a migration handler per M39 policy
  Additive field extensions (serde-default new fields) do NOT require a bump

Scenario: Run bundle references ledger entries
  Given a run bundle with capture grid screenshots
  Then each screenshot in the bundle has an `asset_ref` field linking to a ledger entry
  And `cf-headless replay` validates that referenced ledger entries exist + are Fresh

Scenario: Determinism contract — same seed reproduces same output
  Given two fresh checkouts on different machines
  When both run `cf-mod ledger regenerate <asset_id>`
  Then both produce byte-identical output (assuming pinned model + deterministic pipeline)
  (Cross-platform determinism: requires Tier 1 SVG to be fully deterministic; Tier 2 ComfyUI uses pinned seeds per workflow)

Scenario: Audit reports missing + drifted + failed
  Given some ledger entries with broken state
  When `cf-mod ledger summary` runs
  Then output groups entries by status (Fresh / Stale / Drifted / Missing / Failed)
  And lists the asset_ids in each non-Fresh bucket
  And CI gate `cf-mod ledger verify --strict --all` exits 0 only if all are Fresh

Scenario: Ledger size bounded under regen churn
  Given a developer regenerates the same asset 10000 times during iteration
  When inspecting the ledger
  Then it has 10000 append-only entries (no compaction yet)
  And `cf-mod ledger compact --keep-latest --before <date>` reduces it to current-state-only
  (Compaction is OPTIONAL; CI keeps append-only ledger for traceability)
```

## Out of scope

- **Actual asset generation pipelines** — M9A (SVG) + M12A (audio) + M32A (ComfyUI) + M37A (voice/music) + M38A (localization) own their own pipeline tools; M4A only owns the ledger schema + CLI
- **Mod content moderation** — server-side anti-spam / abuse moderation for mod-supplied assets is M36 / M49 server scope
- **Cosmetic-locker entitlement tracking** — M45A + M49 own anti-pay-to-win audit + Steam DLC integration
- **Steam Workshop publishing UI** — M33 + M36A
- **Asset versioning conflict resolution** — when two mods install assets with the same canonical_name, conflict resolution is M33 mod-conflict UI
- **CDN / asset delivery for online mods** — Steam Workshop handles; M49 lobby directory handles community
- **Per-asset license verification** — author declares; no automated license-checking at M4A
- **AI model version-pinning enforcement** — pipelines (M9A, M32A, etc.) own their own model_version pins; M4A only records them

## Dependencies

- **M0 engine bootstrap (closed)** — provides cargo workspace
- **M4 event recorder (must be concurrent OR closed)** — M4A registers its `asset_ref` field for run-bundle integration with M4's envelope; M4A can ship before M4 close (the envelope reservation is additive)
- **No gameplay dependencies** — M4A is pure infrastructure; ships in BP3 alongside M4

## Notes for the implementer

### Architecture rules

- **Append-only ledger**: never edit a line in `ledger.jsonl` after write. Re-generation produces a NEW entry; old entry gets `superseded_by` field. This is git-friendly + CI-friendly + replay-determinism-friendly.
- **JSONL not JSON**: one entry per line; readable with `tail -f`; concurrent-write-safe via fcntl; no JSON-array overhead. Per `prototype-recorder-event` precedent.
- **blake3 not sha256**: matches M4's checksum algorithm choice; same crate already in workspace.
- **Asset id = blake3(category + canonical_name + tier)**: deterministic; same name in same tier = same id; collision detection at-write.
- **Pipeline id is namespaced**: `M9A_svg_v1` / `M32A_comfyui_v1` / `M37A_voice_v1` — when a pipeline ships v2, old entries reference v1 (forward-compat).
- **Determinism gate**: same seed + same model_version + same prompt MUST produce same blake3. Pipelines that fail this contract (non-deterministic LLM output, GPU non-determinism) MUST use a freeze-then-store approach: pipeline runs once, output stored, ledger records output blake3; regeneration verifies against stored output, NOT re-runs the pipeline.

### Schema design

- **Locked at v1.0.0**: every field additive; new optional fields via serde-default
- **Forward-compat**: M39 catalogs this schema; any field changes require M39 migration policy
- **Mod-extension fields**: mods declare additional metadata via `extension_fields: HashMap<String, Value>`; engine ignores unknown fields

### Per-pipeline integration

Each downstream pipeline milestone (M9A, M12A, M18A, M24A, M25A, M32A, M37A, M38A, M45A, M48A, M48B) writes a ledger entry per generated asset. The pipeline tool calls `cf-mod ledger add` or uses the `cf-asset-ledger` Rust API directly.

Pipelines are responsible for:
1. Computing the canonical_name (deterministic from input parameters)
2. Setting tier + pipeline correctly
3. Recording prompt + seed + model_version
4. Writing output to declared `output_path`
5. Calling `cf-mod ledger add` AFTER writing the output

Pipelines that fail (network down, model unavailable) must NOT write a partial entry; either complete-success or fully-aborted.

### CI integration

- Nightly CI runs `cf-mod ledger verify --strict --all` → must pass; drifted assets break the build
- Pre-commit hook runs `cf-mod ledger verify --strict <changed files>` → catches local drift before push
- Release CI runs `cf-mod ledger regenerate --all` from clean checkout → validates full reproducibility
- Mod-CI runs `cf-mod ledger verify --strict <mod's entries>` before workshop upload

### Pitfalls

- **Forgetting to call `cf-mod ledger add` after pipeline output**: asset exists on disk but isn't in ledger; CI catches via "untracked content/assets/" warning
- **Editing assets by hand**: drifts the ledger; either re-generate via pipeline OR commit a Tier 3 polish entry that supersedes
- **Non-deterministic GPU/CUDA output across hardware**: pipeline tools MUST either pin to deterministic mode OR use freeze-then-store
- **Schema field churn**: lock at v1.0.0; only additive changes via serde-default; bumps require migration per M39
- **Ledger size in git**: ledger.jsonl grows; for the launch roster (~5000 assets × multiple regen iterations) it could reach 50-100 MB. Git LFS for the file. Alternatively, periodic compaction via `cf-mod ledger compact`.
- **Concurrent writes from CI workers**: parallel mod builds writing to the same ledger.jsonl — use fcntl advisory lock OR per-mod sub-ledgers merged at build-end.

### Decision-record alignment

- **DR-053 (asset ledger + AI-generated traceability) — CLOSES at M4A**
- **DR-044 (audiovisual production pipeline)**: M4A is the foundation; tier-by-tier pipelines ship as M9A/M32A/M37A/M48A
- **DR-006 (mod parity)**: mods write to same ledger; full parity
- **DR-024 (native engine stack)**: cf-asset-ledger is pure Rust; no Python in the core (Python is in pipelines)

### Closure procedure

1. Reference bundle: `prototype_runs/native/m4a_<UTC>_<hash>/` (proves cf-asset-ledger crate works + 50 sample entries added + verify all-Fresh + regenerate produces byte-identical output)
2. Self-play sweep rows: `m4a_ledger_add`, `m4a_ledger_list`, `m4a_ledger_regenerate`, `m4a_ledger_verify`, `m4a_ledger_drift_detection`, `m4a_ledger_mod_pack_integration`, `m4a_universal_done_criteria`. All PASS.
3. Update `docs/plan/decisions/dr-053-...` status → CLOSED-DIRECTION-WITH-EVIDENCE.
4. Register AssetEntry schema in M39's manifest at M39 close.
5. Move M4A → done/.
