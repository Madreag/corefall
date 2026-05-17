# M4B — Save Format Versioning + Schema Migration + Delta-Encoded Snapshots + Run-Bundle Ledger Deep

## Status

`active`

## Intent

M4B locks the **deep persistence layer** below M4 + M5 + DR-029: an explicit semver schema for `.cfsave` blobs, a forward-compat migration table that translates older saves into the current schema without data loss, delta-encoded incremental snapshots between full checkpoints (Powder-Toy-style baseline + per-tick delta chain) so that long replays and large saves stay an order of magnitude smaller without sacrificing the byte-identical determinism contract, and a per-mission run-bundle ledger with a cryptographic chain-of-custody (BLAKE3 hash chain) that proves no bundle has been tampered with between record and replay. M4 declared the event envelope; M5 shipped chassis + equipment round-trip; M10 made bundles human-readable. M4B is the binding glue that makes saves and replays survive across game versions, mod updates, and competitive scrutiny.

## Canonical ownership

Owns the `.cfsave` semver registry, the migration table (`SaveMigration::v(N) -> v(N+1)`), the snapshot baseline + delta-chain encoder, the run-bundle ledger chain-of-custody, the `cf-save::migration` and `cf-save::delta` modules, and the canonical migration test corpus under `game/content/save_corpus/`. M5 keeps owning the actor + chassis + equipment payload; M4 keeps owning the run bundle envelope; M4B binds them into a versioned + delta-compressed + tamper-evident persistence layer. DR-029 (T-SAVE deep) closes on M4B + future M4C content extensions.

## Player-facing behavior

- **Old saves still load.** A `.cfsave` written under any prior shipped schema version loads cleanly under the current build — the migration table runs forward through every intermediate version on disk, never asks the player to "convert", and never silently drops fields.
- **Save files are smaller.** A 30-minute mission's run bundle ships ~5× smaller on disk because every snapshot after the first baseline is a delta against the previous, not a full state dump.
- **Save corruption is detectable.** When a save fails its embedded BLAKE3 checksum, the loader surfaces a structured `SaveError::ChecksumMismatch` modal in plain language instead of a panic.
- **Replays survive a game update.** A replay recorded on `v0.7.0-alpha` plays back identically on `v0.8.0-alpha` after the upgrade migration runs; the player sees a single-line "Replay migrated from v0.7.0 → v0.8.0" banner in the viewer header, no UX disruption.
- **Tamper-evident competitive replays.** Tournament-mode bundles ship with a chain-of-custody manifest (per-event BLAKE3 chained to the previous event's hash); the viewer + server both verify the chain on load and reject bundles whose chain is broken.
- **Autosave is cheap.** Mission autosave fires every 60 seconds without a perceptible frame hitch because each autosave is a delta against the previous checkpoint, not a full state dump.
- **Save / load roundtrip is sub-second.** Quicksave (F5) + quickload (F9) on a 200-actor scenario completes in under 800 ms wall clock on the reference Workstation tier.
- **Cross-save composition works.** A player can load a `.cfsave` from one scenario and the run bundle from a different scenario back-to-back; the loader validates each artifact against its own schema version independently and surfaces any incompatibility before the world spawns.

## Crates / modules touched

| Crate | Status | What changes |
|---|---|---|
| `cf-save` | MODIFY | Promote `SAVE_BLOB_VERSION` from `u32 = 1` to `SaveSchemaVersion { major, minor, patch }`; add `SaveBlob::schema_version: SaveSchemaVersion`; expose `cf_save::migration::migrate(blob, target) -> Result<SaveBlob, MigrationError>` + `cf_save::migration::REGISTRY` (ordered chain of `Box<dyn SaveMigration>`); add `SaveError::{ChecksumMismatch, UnsupportedFutureVersion, MigrationFailed { from, to, reason }, MissingRequiredField { version, field }}` |
| `cf-save::migration` | NEW | Per-version migration handler trait + concrete `Migration_v1_to_v2` etc.; canonical migration test corpus loader |
| `cf-save::delta` | NEW | Baseline snapshot + delta encoder (per-actor + per-chunk + per-projectile delta); replay-side reconstructor; blake3 anchor every N deltas |
| `cf-save::ledger_chain` | NEW | Per-event BLAKE3 chain (`prev_hash` field on every recorded event); chain verifier; tamper-detection rejector |
| `cf-replay` | MODIFY | `RunManifest::save_schema_version: SaveSchemaVersion`; `RunManifest::delta_baseline_cadence_ticks: u64` (default 600 = 10 s @ 60 Hz); `RunManifest::ledger_chain_anchor: Option<String>` (blake3 of last event for tournament mode); event envelope `prev_event_hash: Option<String>` for chained mode; producer of `snapshot.baseline_emitted` + `snapshot.delta_emitted` event types |
| `cf-control` | MODIFY | `cfctl save quicksave / quickload / autosave-now / list / inspect <path> / migrate <path> --to <version>`; `cfctl observe.save.last` returns last save metadata (path, schema_version, size_bytes, blake3); `system.save_completed` + `system.save_loaded` + `system.save_migrated` events |
| `cf-control` schemas | MODIFY | `RunManifest` JSON Schema updated for new fields; round-trip migration validation |
| `cf-headless` | MODIFY | `cf-headless replay <bundle>` runs ledger chain verification before sim playback; `cf-headless save migrate <path> --to <version>` standalone migrator; `cf-headless save inspect <path>` prints schema_version + delta chain depth + ledger anchor |
| `cf-app` | MODIFY | F5 / F9 quicksave + quickload hotkeys wired to `cf-save`; "save corrupted" modal renders `SaveError` variants in plain language; "replay migrated" banner in viewer header when load triggers a migration step |
| `cf-shell` | MODIFY | `save_load` module reads + displays schema version next to each slot; rejects slots from future versions with a clear message; offers "Migrate now" CTA for old slots |
| `cf-mod` | MODIFY | `cf-mod ledger verify --bundle <path>` verifies cryptographic chain-of-custody for a single run bundle; `cf-mod save validate <path>` runs full schema + migration + checksum validation |
| `cf-tools-replay-viewer` | MODIFY (small) | Reads + reconstructs delta chain into per-tick snapshots transparently; surface "delta depth: N" + "last baseline at tick: T" in viewer header; viewer `validate` subcommand runs ledger chain check |

## Files

Source:
- `game/crates/cf-save/src/lib.rs` (MODIFY: SaveSchemaVersion + error variants + migration entry point)
- `game/crates/cf-save/src/migration.rs` (NEW: trait `SaveMigration` + ordered `REGISTRY` + per-version handlers)
- `game/crates/cf-save/src/migration_v1_to_v2.rs` (NEW: first concrete migration handler)
- `game/crates/cf-save/src/delta.rs` (NEW: baseline + delta encoder/decoder)
- `game/crates/cf-save/src/delta_actor.rs` (NEW: per-actor delta)
- `game/crates/cf-save/src/delta_chunk.rs` (NEW: per-terrain-chunk delta)
- `game/crates/cf-save/src/delta_projectile.rs` (NEW: per-projectile delta)
- `game/crates/cf-save/src/ledger_chain.rs` (NEW: blake3 chain encoder/verifier)
- `game/crates/cf-save/src/checksum.rs` (NEW: canonical-JSON blake3 over save blob)
- `game/crates/cf-save/src/quicksave.rs` (NEW: F5/F9 fast path with delta tier)
- `game/crates/cf-replay/src/lib.rs` (MODIFY: RunManifest fields + envelope `prev_event_hash`)
- `game/crates/cf-replay/src/snapshot_baseline.rs` (NEW: baseline snapshot writer)
- `game/crates/cf-replay/src/snapshot_delta.rs` (NEW: delta snapshot writer paired with cf-save::delta)
- `game/crates/cf-control/src/m4b_save.rs` (NEW: cfctl save subcommands)
- `game/crates/cf-headless/src/save_migrate.rs` (NEW: standalone migrator binary path)
- `game/crates/cf-app/src/quicksave.rs` (NEW: F5/F9 hotkey wiring)
- `game/crates/cf-shell/src/save_load.rs` (MODIFY: schema version display + migrate CTA)
- `game/crates/cf-mod/src/bundle_chain_verify.rs` (NEW: ledger chain verifier)
- `game/crates/cf-tools-replay-viewer/src/delta_reconstructor.rs` (NEW: transparent delta-to-snapshot reconstructor)

Schemas:
- `game/crates/cf-control/schemas/v1/save_blob.schema.json` (MODIFY: SaveSchemaVersion + delta cadence)
- `game/crates/cf-control/schemas/v1/run_manifest.schema.json` (MODIFY: delta_baseline_cadence_ticks + ledger_chain_anchor)
- `game/crates/cf-replay/schemas/event/snapshot_baseline_emitted.json` (NEW)
- `game/crates/cf-replay/schemas/event/snapshot_delta_emitted.json` (NEW)
- `game/crates/cf-replay/schemas/event/save_completed.json` (NEW)
- `game/crates/cf-replay/schemas/event/save_loaded.json` (NEW)
- `game/crates/cf-replay/schemas/event/save_migrated.json` (NEW)
- `game/crates/cf-replay/schemas/event/ledger_chain_verified.json` (NEW)

Test corpus + scripts:
- `game/content/save_corpus/v1_minimal.cfsave` (NEW: canonical v1 fixture)
- `game/content/save_corpus/v1_full_squad.cfsave` (NEW: full-squad v1 fixture for migration test)
- `game/content/save_corpus/v2_minimal.cfsave` (NEW: canonical v2 fixture; binding contract for v1->v2 migration output)
- `game/content/save_corpus/tampered_chain.cfsave` (NEW: deliberately corrupted chain for rejection test)
- `game/scripts/m4b_migration_matrix.sh` (NEW: loads every fixture, asserts migrate(...) succeeds + checksum matches expected)
- `game/scripts/m4b_delta_compression_bench.sh` (NEW: emits compression ratio for 1-min / 5-min / 30-min mission)

## Acceptance criteria

```gherkin
Scenario: Save written under v1 loads under current build via migration
  Given a `.cfsave` written under SaveSchemaVersion { major: 1, minor: 0, patch: 0 }
  When the current build (v2.x) loads it
  Then cf_save::migration::REGISTRY runs the v1->v2 handler
  And the resulting SaveBlob.schema_version equals the current build's version
  And no payload field is silently dropped (every v1 field is either preserved verbatim or has an explicit `defaults_for_missing` rule in the handler)
  And system.save_migrated fires with from=v1.0.0 + to=v2.0.0 + handler_chain=["v1_to_v2"]

Scenario: Save from a future version is rejected clearly
  Given a `.cfsave` with schema_version { major: 99, minor: 0, patch: 0 }
  When the current build attempts to load it
  Then SaveError::UnsupportedFutureVersion { found, max_supported } returns
  And cf-app surfaces "This save was created in a newer game version (v99.0.0). Update Corefall to load it." with no panic
  And no partial world is spawned

Scenario: Corrupted save surfaces a clean error, never panics
  Given a `.cfsave` whose payload bytes have been flipped after the embedded checksum was computed
  When the loader reads it
  Then SaveError::ChecksumMismatch { expected, actual } returns
  And cf-app renders the plain-language modal "Save file appears corrupted (checksum mismatch). Try another slot."
  And no actor is spawned, no audio is started, no shell state mutates

Scenario: Delta snapshot reconstructs to byte-identical state
  Given a mission recorded with delta_baseline_cadence_ticks=600 over 3600 ticks
  When cf-headless replay reconstructs every tick from the baseline + delta chain
  Then for every tick T in [0, 3600), reconstructed_state(T) == live_recorded_state(T) byte-for-byte
  And the determinism.sim_checksum at every checksum boundary matches the recorded value

Scenario: Delta compression hits its target ratio
  Given a 30-minute mission with 200 actors + 500 projectiles + 1000 hazard pixels
  When the run bundle is written with delta_baseline_cadence_ticks=600
  Then events.jsonl plus snapshot.* events together occupy less than 1/4 the size of the equivalent full-snapshot bundle
  And m4b_delta_compression_bench.sh records ratio >= 4.0x for the canonical benchmark scenario

Scenario: Ledger chain rejects tampered bundle
  Given a tournament-mode run bundle whose event N has had its payload modified after recording
  When cf-mod ledger verify --bundle <path> runs
  Then the verifier walks the prev_event_hash chain and finds the break at event N
  And the verifier exits non-zero with structured JSON: { result: "tampered", first_break: { event_id, expected_hash, actual_hash } }
  And cf-tools-replay-viewer validate refuses to render the bundle

Scenario: Ledger chain passes for a clean tournament bundle
  Given a tournament-mode run bundle whose chain is intact end-to-end
  When cf-mod ledger verify --bundle <path> runs
  Then the verifier reports { result: "clean", events_verified: N, anchor: <hex> }
  And the anchor equals run_manifest.json.ledger_chain_anchor
  And ledger_chain_verified event fires in the viewer's audit log with anchor + total_events

Scenario: Quicksave + quickload roundtrip beats 800 ms on Workstation tier
  Given a 200-actor scenario running steady state
  When the player presses F5 (quicksave) then F9 (quickload)
  Then the F5 path completes in under 400 ms wall clock and emits system.save_completed
  And the F9 path completes in under 400 ms wall clock and emits system.save_loaded
  And the post-load state matches the pre-save state byte-for-byte under canonical blake3

Scenario: Migration corpus matrix passes for every fixture
  Given every fixture under game/content/save_corpus/v1_*.cfsave
  When m4b_migration_matrix.sh runs in CI
  Then each fixture migrates to the current schema without error
  And the migrated blob's canonical-JSON blake3 matches the v(N)_minimal.cfsave or v(N)_full_squad.cfsave golden file for the target version

Scenario: Cross-version replay viewer surfaces migration banner
  Given a run bundle whose RunManifest.save_schema_version is v1.0.0 and the current build is v2.0.0
  When cf-tools-replay-viewer view <bundle> opens it
  Then the viewer header reads "Replay migrated from v1.0.0 -> v2.0.0 (handler: v1_to_v2)"
  And the per-tick event list renders correctly under the migrated schema
  And no event_id collides or is reordered relative to the recorded order

Scenario: Mod-extending fields survive migration
  Given a `.cfsave` recorded with a third-party mod adding extra fields under SaveBlob.mod_payload["acme_corp"]
  When the loader runs migration under a build that has the mod uninstalled
  Then mod_payload["acme_corp"] is preserved verbatim in the migrated blob (forward-compat extension rule)
  And reinstalling the mod and reloading the migrated save restores the mod's state without loss

Scenario: Delta baseline cadence is enforced
  Given a run bundle with delta_baseline_cadence_ticks=600 and total_ticks=3601
  When the bundle is inspected by cf-headless save inspect
  Then exactly 7 snapshot.baseline_emitted events fire (at ticks 0, 600, 1200, 1800, 2400, 3000, 3600)
  And every other snapshot-bearing tick fires snapshot.delta_emitted referencing the most recent baseline_event_id
```

## Out of scope

- Cloud save sync (Steam Cloud, EOS Cloud) — M36A platform integration ships those.
- Multi-slot save UI polish + cosmetic thumbnails — `cf-shell::save_load` keeps its baseline; cosmetic Tier 2 portraits / capture grids land in M32A.
- Cross-game-engine import (Cortex Command / Noita / Stationeers saves) — never.
- Save-blob encryption — tampering is detected via the chain, not prevented via encryption; encryption is a separate auditable concern owned by future M48 covert-ops content if ever needed.
- Per-actor partial save (just-this-character export) — M4B keeps the whole-world contract; partial-actor export is M27A inventory loadout export only.
- Live "edit save in flight" tools — out of scope; saves are immutable once written.

## Dependencies

- M4 (done; commit `prototype-recorder-event.v0.1`) — event envelope + run bundle layout.
- M5 (done) — SaveBlob baseline + chassis/equipment payload.
- M3A (done) — cross-OS determinism (so canonical-JSON blake3 of a save is identical on Linux / macOS / Windows).
- M10 (done) — viewer + cause-chain (delta reconstructor plugs into the existing viewer API).
- M8A (done) — parallel determinism + shard merge (delta encoder is per-tick, sits inside the serial commit phase).
- DR-002 — replay + event architecture.
- DR-029 — T-SAVE (canonical save contract).
- DR-052 — float determinism rules.

## Notes for the implementer

- `SaveSchemaVersion` is a `{u16 major, u16 minor, u16 patch}` tuple serialized as a 3-element JSON array so it round-trips through canonical-JSON blake3 cleanly. Don't use a string like `"v1.2.3"` — the JSON canonicalizer's whitespace + key ordering matters and an array of u16 is unambiguous.
- The migration registry is `Vec<Box<dyn SaveMigration>>` ordered by source version; the `migrate(blob, target)` entry point walks the chain forward, never skips. A missing intermediate handler is a build-time panic (registered via `inventory` or equivalent compile-time check).
- Per-actor delta uses `serde_json::Value` diff for forward-compat; ergonomic enough that mods can add fields without breaking older deltas. Binary delta is a future optimization but not required for M4B close.
- Baseline cadence default = 600 ticks (10 s @ 60 Hz) is tuned to keep delta chain depth ≤ 600 (reconstruction cost stays sub-millisecond). Configurable via `run_manifest.json` if a future scenario wants tighter cadence.
- The ledger chain uses BLAKE3 keyed mode with a per-run key derived from `manifest.run_id` + scenario seed; this binds the chain to its run and makes splicing events from two different runs detectable.
- `cf-headless replay <bundle> --skip-chain-verify` is allowed for dev workflows but ALWAYS-ON in tournament mode (CI gate enforces).
- The migration corpus fixtures (`game/content/save_corpus/v1_minimal.cfsave` etc.) are checked into the repo. Each future schema version MUST land a paired golden fixture in the same PR that introduces the schema bump; M4B's CI gate `m4b_migration_matrix.sh` enforces this as a hard pre-merge check.
- The `mod_payload` opaque-passthrough rule is non-negotiable: a build without a mod installed MUST round-trip the mod's payload verbatim through migration. This is what makes mod ecosystems possible. The mod manifest declares its payload schema_version separately; collision rules live in M33 modding workbench, not M4B.
- F5 / F9 hotkeys are reserved by `cf-shell::keybinds`; M4B wires them; cf-app key remap respects the binding. The 800 ms budget assumes Workstation tier with NVMe; lower tiers may exceed this and that's acceptable — the budget is a Workstation-class promise, not a portable hard cap.
- The autosave timer (60 s default) runs on the engine clock, not wall clock; this preserves determinism for replay-against-autosave testing in M4B's `m4b_migration_matrix.sh`.
- Tournament-mode chain anchor (`run_manifest.json.ledger_chain_anchor`) is the single piece of evidence a tournament organizer needs to publish; given the anchor + the bundle, any third party can verify the bundle was not tampered. This is the M4B promise to competitive Corefall.
