# cf-save — AGENTS.md

## Owns
- M5 slice of T-SAVE (DR-029): `SaveBlob` v1 payload carrying actor identity (id / team / origin / position / velocity / aim / hp / status) + selected inventory slot + rifle preset id + remaining ammo + active reload-ticks + full `ChassisState` (zones with per-layer hp + wound + destroyed, modules with state + last_reason, pilot state, eject window, weapon_jammed flag).
- Canonical-JSON serialization through serde + deterministic blake3 checksum over the canonical (non-pretty) JSON byte stream, so platform float-representation drift cannot leak between save/load on different hosts.
- `SAVE_BLOB_VERSION = 1` schema-version constant + version-mismatch detection on load.
- Tampering detection: `SaveBlob::deserialize(json, Some(expected_hex))` recomputes the checksum and returns `SaveError::ChecksumMismatch` when it doesn't match.
- Determinism contract: every public function is pure; no clock reads; no `rand::thread_rng()`; serialization order is stable.

## Public API Boundary
- Types: `SaveBlob`, `SaveError` (variants: `SerializeJson`, `DeserializeJson`, `SchemaVersionMismatch { expected, actual }`, `ChecksumMismatch { expected, actual }`).
- Functions: `SaveBlob::checksum_hex(&self)`, `SaveBlob::serialize(&self) -> (json, hex)`, `SaveBlob::deserialize(json, Option<expected_hex>) -> SaveBlob`.
- Constant: `SAVE_BLOB_VERSION`.

## Does NOT Own
- Multi-actor / world-state save — M5 ships a single-actor blob; full world snapshot lands in the T-SAVE side track (multi-slot, autosave, ironman, scenario policies, migration handlers).
- Mission / objective / breach state serialization — owned by `cf-mission` + serialized through a future T-SAVE world envelope.
- Cloud-save backend / online sync — post-launch decision; do NOT add cloud-save dependencies during T-SAVE work (per Open Decision Gates Protocol).
- On-disk file layout / `.cfsave` envelope / autosave cadence — the M5 slice writes JSON + sidecar hex; the binary envelope + multi-slot directory layout is T-SAVE.

## Test Surface
- Unit tests: `cargo test -p cf-save` — 5 tests:
  - `save_blob_round_trips_without_chassis` (baseline actor-only blob).
  - `save_blob_round_trips_with_chassis` (full ChassisState attached + checksum stability across serializations).
  - `checksum_mismatch_is_detected` (`SaveError::ChecksumMismatch` returned for tampered hex).
  - `schema_version_mismatch_is_detected` (`SaveError::SchemaVersionMismatch` returned when `schema_version != SAVE_BLOB_VERSION`).
  - `chassis_damage_persists_through_roundtrip` (40 dmg to torso External layer survives serialize → deserialize).

## Cross-Crate Contracts
- Depends on: `cf-chassis` (for `ChassisState`, `BodyZone`, `ArmorLayerKind` used in tests), `cf-equipment` (for `RIFLE_M1_DEFAULT_ID` rifle preset id used in tests).
- Depended on by: `cf-control` — `runbundle.write` integration writes a `SaveBlob` snapshot at run-bundle close so a replay viewer can reproduce the actor + chassis state at any tick boundary.

## Common Pitfalls
- Always bump `SAVE_BLOB_VERSION` when the `SaveBlob` struct shape changes — old saves loaded against a new struct must deterministically fail with `SchemaVersionMismatch` rather than silently default-fill unknown fields.
- The checksum is computed over the canonical (non-pretty) JSON byte stream, NOT the raw struct bytes; recomputing the checksum requires going back through serde to get byte-identical input.
- `SaveBlob::serialize` returns the pretty JSON for human-readable disk writes AND the hex checksum derived from the canonical JSON. Don't checksum the pretty form — whitespace/indentation drift across `serde_json` versions would break it.
- `deserialize` with `expected_hex = None` skips checksum verification; production callers should always pass the on-disk sidecar hex to catch corruption.

## Source Trail
- spec/prototype-roadmap §M5 — Equipment, Chassis, And Damage Grammar (M5-004 Save/load checksum done-criterion).
- DR-029 (save-game model; CLOSED for the M5 slice; full T-SAVE side track owns multi-slot/autosave/ironman/migration).
- docs/implementation-log/2026-05-10-m5-chassis-grammar.md.
