# M5 Pass-2 Audit — Cross-Cutting + Repo Hygiene

**Audit date:** 5/13/2026 10:42 PM MST
**Auditor:** worker subagent (pass-2 sweep)
**Scope:** repo hygiene, test coverage, documentation drift, CI integration, snapshot completeness, cross-schema consistency, amendment-id naming — meta-level concerns the per-family audits in pass-1 did not cover.

**Inputs:**

- M5 spec: `/Users/erol/projects/corefall/specs/done/M5.md`
- Project AGENTS.md: `/Users/erol/projects/corefall/AGENTS.md`
- CHANGELOG: `/Users/erol/projects/corefall/CHANGELOG.md`
- M5 schemas: `/Users/erol/projects/corefall/game/crates/cf-replay/schemas/event/` (75 M5-prefixed files after pass-1)
- CI workflows: `/Users/erol/projects/corefall/.github/workflows/{ci,release}.yml`
- cf-replay test source: `/Users/erol/projects/corefall/game/crates/cf-replay/src/schemas.rs::tests`
- cf-mod test source: `/Users/erol/projects/corefall/game/crates/cf-mod/src/main.rs::tests`
- cf-mod integration tests: `/Users/erol/projects/corefall/game/crates/cf-mod/tests/`
- Sister-milestone M9 spec: `/Users/erol/projects/corefall/specs/active/M9.md`
- M4 spec: `/Users/erol/projects/corefall/specs/done/M4.md`
- Pass-1 commit: `1784ad2` ("M5-A1: post-audit hardening pass — 17 audit findings closed; ready for M6")
- M5 close commit: `01c9a71` ("M5: close event-surface lock — move spec to done")
- M5 lock commit: `1fb5b3c` ("M5: lock 74 deep-damage event schemas at v0.1 envelope")

---

## Category 1: Repo hygiene

### A1. AGENTS.md M5 workflow compliance

AGENTS.md mandates (lines 18-50):

> 5. Commit each meaningful gap-fill with subject `<id>: <imperative summary>`. Multiple commits per spec is fine.
> 6. Report a per-scenario verdict table at the end of the session in this format: …
> 7. When every scenario verdict is `PASS (already in)` or `IMPLEMENTED`, move the spec from `specs/active/` to `specs/done/`.

**Check 1 — spec moved to done/:** PASS. `specs/done/M5.md` exists and `specs/active/M5.md` does not. The move was committed at `01c9a71` ("M5: close event-surface lock — move spec to done"). Verified via `LS specs/done/` returns `M5.md` (line 7) and `LS specs/active/` (70 files, none named `M5.md`).

**Check 2 — commit subject `<id>: <imperative summary>` format:**

```
1fb5b3c M5: lock 74 deep-damage event schemas at v0.1 envelope
01c9a71 M5: close event-surface lock — move spec to done
1784ad2 M5-A1: post-audit hardening pass — 17 audit findings closed; ready for M6
```

All three subjects conform to `<id>: <imperative-summary>`. The pass-1 commit uses `M5-A1` as the id; AGENTS.md is silent on whether amendment ids follow `M<N>A` (M4A's precedent) or `M<N>-A<n>` (M5-A1's invention). See § G1 for the amendment-naming verdict.

**Check 3 — per-scenario verdict table at session end:** **NOT DURABLE.**

AGENTS.md requires the verdict table at the end of the session. The closure session (commit `01c9a71`) and pass-1 session (commit `1784ad2`) emitted verdict tables only in chat — these are ephemeral. **No verdict table is persisted in any file under `specs/done/`, `audit-m5/`, `audit-m5-pass2/`, `CHANGELOG.md`, `docs/`, or commit messages** (the commit messages are one-line subjects with no body — no table).

The pass-1 `audit-m5/*.md` files DO contain "Per-event verdict table" sections but they are pre-fix audits (the audit work that motivated pass-1), not post-fix verdicts proving the M5 spec is closed.

**Verdict:** verdict tables existed in agent chat output, but the durable audit-trail surface (commits + repo files) carries only the audit-input tables under `audit-m5/`. Compliance is technical but not robust against later session reconstruction. Recommended fix in § "Recommended fixes".

**Check 4 — meaningful gap-fill commits (multiple per spec):** PASS. Three commits:

- `1fb5b3c` — initial 74-schema lock at v0.1 (close M5 acceptance contract).
- `01c9a71` — spec move (workflow step 7).
- `1784ad2` — pass-1 hardening (17 audit findings).

Each commit corresponds to a meaningful gap-fill phase. Multiple-commits-per-spec is allowed and exercised.

### A2. CHANGELOG.md entry

**Check:** `cat CHANGELOG.md | grep -nE 'M5(-A1)?'` returns 3 lines:

```
30:| M5 — Equipment, Chassis, Damage Grammar | LANDED | commit `29edc1b`; DR-014 + DR-021 closed | None |
33:**M5 — Equipment, Chassis, And Damage Grammar (LANDED):**
743:- **M5 — `cfctl observe --inline --stream` is now an explicit error**: …
```

**ALL THREE references are to the OLD M5** ("Equipment, Chassis, Damage Grammar" — see line 33's full bullet). That M5 has been **renamed and renumbered** in the recent specs-reorganisation commits (`655cf39 specs: rename milestones to sequential M1..M49`, `1c7a73c specs: reorder M4..M49 by execution dependency, …`). The current M5 ("Deep Damage Event Surface Lock") is a different milestone that took over the M5 slot.

**No CHANGELOG entry exists for:**

- The current M5 ("Deep Damage Event Surface Lock") closure at commit `01c9a71`.
- The pass-1 (M5-A1) hardening at commit `1784ad2`.

**AGENTS.md policy:** the global agent rules say "Never create or update documentations and readme files unless specifically requested by the user." The project AGENTS.md does not override this. Under that rule, **CHANGELOG should NOT be updated without explicit user instruction** — and there is no record of such instruction in the task brief for either commit. Pass-1's lack of CHANGELOG update is therefore **policy-compliant**.

However, the resulting state is **misleading**: a future reader scanning CHANGELOG for "M5" lands on stale prose about a renumbered milestone. That is a hygiene gap regardless of whether updating CHANGELOG was sanctioned at the time.

**Verdict:** CHANGELOG is stale-but-compliant. Either (a) the user grants explicit permission to retroactively add M5 + M5-A1 entries, or (b) the user accepts the staleness and the CHANGELOG header note ("Use this file to summarize what changed in the implementation repo") becomes the disclaimer. Either is a user-level decision, not an agent-level fix.

### A3. specs/done/M5.md updates

**Check:** is the M5 spec mutable after move to done/?

AGENTS.md is silent on post-done/ spec mutability. The commit history shows the spec was moved (in `01c9a71`) but the file content was NOT edited in pass-1 (`1784ad2` touched only `game/crates/cf-replay/schemas/event/*.json`, `game/crates/cf-replay/src/schemas.rs`, `game/crates/cf-mod/src/main.rs`, and a few support files — but NOT `specs/done/M5.md` itself).

Pass-1 introduced these spec-vs-code drifts:

| Spec text (specs/done/M5.md) | Code reality after pass-1 |
|---|---|
| "22 affliction kinds (locked names; mechanics in M16): burning, wet, …, sanity_low." (1 paragraph in spec) | All 4 affliction.* schemas declare a 23-kind enum including the M5-A1 addition `blinded`. |
| `affliction.applied { …, expected_duration_ticks, severity_0_1 }` — kind list of 22 enumerated in prose | Schema enum size 23; pass-1 added `blinded` for M6 flash grenade. |
| Acceptance criteria scenario: "each schema declares schema_version=\"0.1\" matching the M4 locked envelope" | Schemas declare `schema_version: "prototype-recorder-event.v0.1"` (the canonical M4 envelope literal). The spec text `"0.1"` is technically incorrect after pass-1. |
| `combat.projectile_hit_mo` brace block lists `parent_event_id: EventId` | Schema renames the payload field to `parent_hit_event_id` (matching `origin.shot_force_feedback`'s `parent_hit_event_id` convention; avoids the envelope-level `parent_event_id` field name collision). |
| Spec § "Sound clip variants per armor material + impact state" — "M5 just locks the request shape" | Pass-1 SHIPPED the `audio.event_requested` schema fulfilling this spec promise. Spec doesn't list the schema filename as a deliverable. |

**Verdict:** the spec is **silently out-of-sync** with the schemas it nominally describes. AGENTS.md doesn't require an addendum, but downstream M6 / M13 implementers reading `specs/done/M5.md` literally would emit `schema_version: "0.1"` (rejected by cf-mod validator) and produce 22-kind affliction enums (rejected by cf-replay tests). Spec drift is **HIGH severity** for downstream implementers.

**Spec-amendment options:**

1. **Inline addendum at end of M5.md**: a `## M5-A1 Amendment (5/13/2026)` heading appended to the file, listing the 4 changes above as patches to the spec body. Leaves the original spec text intact but corrects the contract.
2. **Edit spec inline**: rewrite the affected paragraphs (22 → 23 affliction kinds, `"0.1"` → `"prototype-recorder-event.v0.1"`, `parent_event_id` → `parent_hit_event_id`, add audio_event_requested to the "Implementer notes" file list). More invasive but more readable.
3. **No spec edit, document only in CHANGELOG**: relies on CHANGELOG as the single source of post-closure truth.

Either option requires user instruction per AGENTS.md doc-update rule.

### A4. Sister-milestone (M9) alignment

**Check:** does M9.md reference M5 schemas that may have drifted post-pass-1?

`grep "M5\|schemas/event/" specs/active/M9.md`:

- Line 11: "**M5 is its sister milestone** — it locks the canonical event surfaces for the deep damage / hazard / affliction / armor / internal / fluid / origin / atmospherics / shield / environment / thermal kernels that ladder up at M13/M14/M15/M16/M17/M19/M20." — generic reference, no drift.
- Line 19: "Every damage event from M9 forward emits the structured `combat.*` / `terrain.*` / `hazard.*` / `affliction.*` / `atmos.*` / `shield.*` event families … . Event schema details live in `specs/active/M5.md`." — pointer is stale (M5 is now in `specs/done/`, not `specs/active/`). MINOR DRIFT.
- Line 120: "Per-layer event schemas (`armor.layer_hp_changed`, `armor.layer_critical`, `armor.layer_destroyed`, etc.) live canonically in `specs/active/M5.md` § armor.* family." — same stale active/ pointer.
- Line 141: "**Deep damage event family schemas (armor.* / internal.* / concussion.* / fluid.* / origin.* / hazard.* / affliction.* / atmos.* / shield.* / environment.* / thermal.*) defined canonically in `specs/active/M5.md`**" — same.
- Line 150: same.
- Lines 188, 191-196, 564, 653, 684, 692, 762: more references all using `specs/active/M5.md`.

**Drift:** every M9 reference still points to `specs/active/M5.md` even though M5 was moved to `specs/done/M5.md` in commit `01c9a71`. **MEDIUM SEVERITY** — readers following the pointer get a "file not found" symptom. The only fix is a bulk `s|specs/active/M5\.md|specs/done/M5.md|g` in M9.md (and any other spec referencing M5).

**Other M9-spec ↔ M5-schema drifts:** none observed. M9.md doesn't enumerate `combat.projectile_hit_mo` payload fields, doesn't enumerate the 22/23 affliction kinds, doesn't pin a `schema_version` literal. So pass-1's textual changes don't propagate into M9.

---

## Category 2: Test coverage

### B1. Per-family happy-path tests

Pass-1 added `m5_per_family_happy_path` to `game/crates/cf-replay/src/schemas.rs::tests` (line ~1100 in the current file). The test invokes `validate_event_payload(...)` with a representative payload for **14 distinct (category, event) pairs**:

| Family | Test coverage |
|---|---|
| armor | `armor.layer_hp_changed` |
| internal | `internal.organ_damaged` |
| concussion | `concussion.band_changed` |
| internal_shock | `internal_shock.dose_changed` |
| fluid | `fluid.leak_started` |
| origin | `origin.g_load_dose_changed` |
| hazard | `hazard.spawned` |
| affliction | `affliction.applied` (with new `blinded` kind to lock pass-1 addition) |
| atmos | `atmos.gas_released` |
| shield | `shield.hit` |
| environment | `environment.signal_delta` |
| thermal | `thermal.material_phase_change` |
| combat | `combat.projectile_hit_mo` |
| audio | `audio.event_requested` |

14 families covered — that is **all 13 M5 families plus the M5-A1 audio addition**. PASS.

**Per-family edge cases:** only ONE happy-path per family. Pass-1 did NOT add negative tests per family. Negative tests exist for:

- `armor.layer_destroyed` — missing-`breach_kind` (legacy M5 test); bad-`zone`-enum (M5-A1).
- `combat.projectile_hit_mo` — `parent_event_id` envelope-named-parent collision rejection (M5-A1).
- `concussion.dose_changed` — bad-`origin_id` rejection (M5-A1).

The remaining 10 families (internal, internal_shock, fluid, origin, hazard, affliction, atmos, shield, environment, thermal, audio) have **only happy-path coverage**. Edge-case coverage is implicit through the validator's family-agnostic implementation, but explicit per-family negative tests would catch any future schema-shape regression. LOW-MEDIUM GAP.

### B2. Round-trip envelope tests

**Check:** is there any test that does `Recorder::record(...)` → read back the resulting `Event` → `validate_event_payload(payload)`?

`grep -n "Recorder::record\|Recorder::new\|Recorder::with_capacity" game/crates/cf-replay/src/schemas.rs`: 0 matches. The schemas.rs test module is pure-validator and never instantiates the recorder.

`grep -rn "validate_event_payload" game/crates/cf-replay/src/lib.rs game/crates/cf-mod/`: hits in `lib.rs` (the API export) and `cf-mod/src/main.rs::validate_bundle_command`. The bundle-validate path IS a round-trip in production (recorder → JSONL → parse → validate), but **no in-process test asserts that a Recorder-emitted Event satisfies its registered schema**.

**Gap:** **MEDIUM SEVERITY.** A round-trip test would catch the class of regression where a producer's serialized payload diverges from the schema (e.g., if the recorder ever started emitting `parent_event_id` at envelope level but the schema also requires it at payload level — exactly the kind of bug pass-1 caught by spec inspection). The test would be a single-file integration test under `cf-replay/tests/`. See § B3 for the related-file gap.

### B3. cf-replay/tests/ integration tests

**Check:** does `game/crates/cf-replay/tests/` exist?

```
$ ls game/crates/cf-replay/
Cargo.toml  schemas  src
$ ls game/crates/cf-replay/tests
ls: tests: No such file or directory
```

**The `cf-replay/tests/` directory does NOT exist.** Pass-1 did not create it.

`game/crates/cf-replay/src/schemas.rs` module docstring (lines 22-25) explicitly references the missing directory:

```
//! `cf-mod validate-bundle` calls `validate_event_payload` on every event
//! in a run bundle; the workspace test under `cf-replay/tests` walks a
//! freshly-recorded smoke bundle to prove the schemas accept real events.
```

The docstring promises a workspace integration test "under `cf-replay/tests`" — and that test does not exist. **GAP — MEDIUM SEVERITY** because:

1. The docstring is itself stale (out-of-sync with the actual file tree).
2. No integration test proves that an M5-shaped recorder Event survives the round-trip through `validate_event_payload` against the on-disk schema.

**Recommended:** add `game/crates/cf-replay/tests/m5_round_trip.rs` with a test that:

1. Constructs a tiny `Recorder`,
2. Records one representative event per M5 family,
3. Parses the recorder's serialized output back through `serde_json::from_str::<Event>`,
4. Calls `validate_event_payload(event.category, event.event_type, &event.payload)`,
5. Asserts `Ok(())` on every family.

This is the single most leverage-rich integration test the M5 surface lacks today.

### B4. cf-mod integration tests

**Check:** `ls game/crates/cf-mod/tests/`:

```
ledger_cli_integration.rs   (M4A — 13355 bytes)
```

`ledger_cli_integration.rs` is the asset-ledger smoke test for M4A. There is **NO** integration test under `cf-mod/tests/` that exercises `cf-mod validate game/crates/cf-replay/schemas/` end-to-end.

**Gap:** **MEDIUM SEVERITY.** The hands-on tests in `audit-m5/05-validator-audit.md` confirm `cf-mod validate cf-replay/schemas/` exits 0 on the current tree, but there is no in-repo automation that re-runs it on every test invocation. The closest existing surface is `m5_all_shipped_schemas_validate` (a unit test in `cf-mod/src/main.rs`) which walks `cf-replay/schemas/` from inside cf-mod's test binary — that DOES exist (verified). So the gap is "no CLI integration test (subprocess invocation of `cargo run -p cf-mod -- validate …`)", but the in-process schema-walk path IS covered.

**Recommended:** add `cf-mod/tests/validate_cli_integration.rs` that subprocesses `cargo run -p cf-mod -- validate game/crates/cf-replay/schemas/` and asserts exit 0 + `scanned=N pass=N`. Catches regressions in the CLI surface (output format, exit codes) that the in-process unit test does not cover.

### B5. Adversarial test count (positive vs negative)

Counting tests in `game/crates/cf-replay/src/schemas.rs::tests` after pass-1:

**Positive (happy-path / acceptance):**

1. `schemas_load_for_every_registered_event_type` — confirms each shipped schema is parseable JSON.
2. `terrain_carved_event_validates_minimum_payload` — pre-M5 legacy.
3. `terrain_penetration_threshold_event_validates` — pre-M5 legacy.
4. `unknown_event_type_is_ok_by_default` — fallback path.
5. `validates_input_intent_received_required_fields` (also negative — has both branches).
6. `m5_armor_layer_destroyed_payload_validates` — M5 spec happy path.
7. `m5_armor_layer_destroyed_accepts_additive_payload_extension` — M5 additive contract.
8. `m5_per_family_happy_path` — pass-1 addition; 14 families × 1 happy event each.
9. `m5_schemas_declare_schema_version_v0_1` — global meta-test (75 schemas).

**Negative (rejection):**

1. `validates_input_intent_received_required_fields` (the missing-`actor` branch).
2. `validates_projectile_spawned_array_arity` — array `minItems` rejection.
3. `m5_armor_layer_destroyed_rejects_missing_breach_kind` — missing-required field.
4. `m5_armor_layer_destroyed_rejects_bad_zone_enum` — pass-1 addition; bad-enum rejection.
5. `m5_combat_projectile_hit_mo_rejects_envelope_named_parent` — pass-1 addition; payload field rename guard.
6. `m5_concussion_dose_changed_rejects_bad_origin` — pass-1 addition; oneOf rejection of non-canonical Origin.

**Ratio:** ~9 positive : 6 negative (≈ 60 : 40). **Adequate.** Negative-test coverage is concentrated on the highest-risk surfaces (M5-A1's three new constraints: bad enum, envelope-name collision, oneOf Origin enum). The remaining 10 families have positive-only coverage; see § B1 recommendation.

---

## Category 3: Documentation drift

### C1. Description field accuracy

**Affliction-count drift across the 4 affliction schemas:**

```
$ grep -l "23 affliction" game/crates/cf-replay/schemas/event/*.json
affliction_applied.json
$ grep -l "22 affliction" game/crates/cf-replay/schemas/event/*.json
(no output)
$ grep -l "blinded" game/crates/cf-replay/schemas/event/*.json
affliction_applied.json
affliction_cleared.json
affliction_escalated.json
affliction_tick.json
```

- `affliction_applied.json` description: "23 affliction kinds (locked names; M16 fills mechanics; **M5-A1 adds blinded for M6 flash grenade**)" + full 23-element comma-list. ✓ updated.
- `affliction_cleared.json` description: NO mention of count. Enum has 23 kinds including `blinded`. Description is purely about the `reason` enum.
- `affliction_tick.json` description: NO mention of count. Enum has 23 kinds including `blinded`. Description is about cosmetic semantics.
- `affliction_escalated.json` description: NO mention of count. Enum has 23 kinds including `blinded`. Description is about severity escalation.

**Verdict:** **NO ACTUAL DRIFT** — the enum (the authoritative shape) is consistent across all 4 affliction schemas. The cleared/tick/escalated descriptions never claimed a specific count, so they cannot be "self-contradictory". Pass-1's targeted update to affliction_applied is appropriate (that schema describes the canonical "applied" event where the count matters most).

**Hazard-spawned spec-vs-schema drift (resolved pre-M5-A1):**

`hazard_spawned.json` description acknowledges the spec drift: "**9 hazard kinds (locked): fire, smoke, electric, wet, hot, cold, acid, radiation, toxic. The 5-launch subset for M16-launch is {fire, smoke, electric, wet, hot|cold}; M16 extends to 9 by adding acid, radiation, toxic. The spec bullet's 'hot_cold' single-token name is split into 'hot' + 'cold' here; the spec bullet's 'radiation_zone' / 'toxic_atmosphere' bullet names are short-form 'radiation' / 'toxic' in this enum.**" — this is honest disclosure of the schema-vs-spec divergence and is already resolved.

**No other description drifts found.** Verified across:

- `combat_projectile_hit_mo.json` — description carries ricochet-threshold table + effective-thickness formula verbatim.
- `armor_spalling.json` — formula verbatim.
- `armor_angle_deflection_calculated.json` — formula present.
- `concussion_dose_changed.json` — per-origin decay rates correct.
- `atmos_*` — gas enums consistent across 3 gas-bearing schemas.
- `audio_event_requested.json` — 7-material × 5-impact-state taxonomy + 6-internal-hit-kind enum locked.

### C2. Producer-ladder cross-references

Walked every M5 schema's description for the "Producer fills at MX" claim and cross-checked against the spec ladder table:

| Family | Spec ladder | Schemas cite | Drift |
|---|---|---|---|
| armor.* | M13 + M14 | "M13 (chassis 3-layer armor) + M14 (full collision)" or "M13 + M14" | none |
| internal.* | M14 + M17 | "M14 (ray traversal) + M17 (per-origin organ/circuit graphs)" or "M14 + M17" | none |
| concussion.* | M17 | "M17 (origin reaction matrix)" or "M17" | none |
| internal_shock.* | M17 | "M17" | none |
| fluid.* | M13 + M14 | "M13 (chassis fluid system) + M14 (collision puncture)" or "M13 + M14" (`fluid_ignition` cites M16 hazard kernel as the ignition-source side, OK) | none |
| origin.* | M17 | "M17 (origin reaction matrix)" | `origin_oxygen_supply_changed.json` cites "M17 + M19" (oxygen supply has atmospherics dependency); slightly broader than spec but reasonable |
| hazard.* | M16 | "M16 (hazard package)" or "M16" | none |
| affliction.* | M16 | "M16" or "M16 (22-affliction taxonomy)" | none |
| atmos.* | M19 | "M19 (Stationeers-grade atmospherics)" or "M19" | none |
| shield.* | M13+ + M25+ | "M13+" or "M13+ chassis + M25+ base" | none |
| environment.* | M20 | "M20 (EnvironmentSignal aggregator)" or "M20" | none |
| thermal.* | M16 + M19 | "M16 + M19" or "M19 (material kernel)" | `thermal_material_phase_change.json` cites "M19" only (spec says M16 + M19); minor under-cite |
| combat.projectile_hit_mo | M13 + M14 | "M13 + M14" | none |
| audio.event_requested (M5-A1 NEW) | "M13.x cf-audio consumes" per spec § Sound clip variants | "M13.x+ cf-audio consumes" | none |

**Two minor under-/over-cites** (`origin.oxygen_supply_changed` says M17+M19; `thermal.material_phase_change` says M19 only). Both are defensible (the producer ladder is genuinely cross-cutting at those points). LOW severity.

### C3. Sound clip variants table

**Spec § "Sound clip variants per armor material + impact state":**

> | Material | Pristine hit | Cracked hit | Destroyed hit | Chunked-off | Pierce |
> |---|---|---|---|---|---|
> | `metal` | … |
> | `ceramic` | … |
> | `composite` | … |
> | `cloth` | … |
> | `leather` | … |
> | `hardened_plate` | … |
> | `reactive_armor` | … |

**Schema `audio_event_requested.json`** mirror:

- `payload.material` enum = `["metal", "ceramic", "composite", "cloth", "leather", "hardened_plate", "reactive_armor", null]` — 7 materials + null. ✓
- `payload.impact_state` enum = `["pristine_hit", "cracked_hit", "destroyed_hit", "chunked_off", "pierce", null]` — 5 states + null. ✓
- `payload.kind` enum = `["material_state", "internal_hit"]` — discriminator. ✓
- `payload.internal_hit_kind` enum = `["flesh_punctured", "bone_cracked", "organ_ruptured", "circuit_sparked", "circuit_destroyed", "fluid_pierce", null]` — 6 internal-hit names + null. ✓ (matches spec § "Internal hit sounds").

**Verdict:** ✓ — full 7×5 + 6-internal-hit taxonomy correctly encoded in the schema. Pass-1's `audio.event_requested` ship is faithful to spec.

---

## Category 4: CI integration

### D1. GitHub Actions workflows

```
$ ls /Users/erol/projects/corefall/.github/workflows/
ci.yml      release.yml
```

**ci.yml steps (relevant to schema validation):**

```yaml
- name: cargo fmt                                              # line 41
- name: cargo check (workspace --all-targets)                  # line 44
- name: cargo clippy (workspace --all-targets -D warnings)     # line 46
- name: cargo test --workspace                                 # line 48
- name: cargo build --release                                  # line 50
- name: cf-mod validate content                                # line 52 — runs `cargo run --release -p cf-mod -- validate content/`
- name: schemas in sync (dump_schemas --check)                 # line 54 — cf-control schema drift gate
- name: schema-version drift gate                              # line 91 — dump_schemas --check (duplicate)
```

**Critical finding:** `cf-mod validate content/` is wired into CI (line 52), but `cf-mod validate game/crates/cf-replay/schemas/` is NOT. This means:

- A regression that BREAKS an M5 schema (e.g. `additionalProperties: false` slipped onto payload, missing required envelope fields, broken filename↔category-const cross-check) would be caught only by:
  - `cargo test --workspace` (line 48), which exercises `m5_all_shipped_schemas_validate` (the in-process schema walk inside `cf-mod`'s test binary), AND
  - `schemas_load_for_every_registered_event_type` (in `cf-replay`'s test binary), which just confirms the JSON parses.
- The CLI surface (`cargo run -p cf-mod -- validate cf-replay/schemas/`) is NOT exercised by CI today.

**Gap:** **MEDIUM SEVERITY.** The in-process tests cover the validation logic, but the CLI surface (which is what humans + downstream tools invoke) has no CI verification. Recommend adding:

```yaml
- name: cf-mod validate cf-replay/schemas
  run: cargo run --release -p cf-mod -- validate crates/cf-replay/schemas/
```

at line ~53 of ci.yml (right after the existing `cf-mod validate content` step). This is 2 lines of YAML; would catch CLI-surface regressions for free.

### D2. cargo test in CI

`cargo test --workspace` runs at line 48 of ci.yml. Verified locally: `cd game && cargo test -p cf-replay --quiet` returns "39 passed; 0 failed". The M5-A1 tests are exercised automatically:

- `m5_per_family_happy_path` (14 families) ✓
- `m5_combat_projectile_hit_mo_rejects_envelope_named_parent` ✓
- `m5_concussion_dose_changed_rejects_bad_origin` ✓
- `m5_armor_layer_destroyed_rejects_bad_zone_enum` ✓
- `m5_schemas_declare_schema_version_v0_1` (75 schemas after audio addition) ✓

`cargo clippy --workspace --all-targets -- -D warnings` runs at line 46 → catches lint drift on M5-A1 source changes. ✓

**Verdict:** CI's cargo-test path **does** exercise pass-1's hardening. No gap.

### D3. Schema validation in CI

Recommend (as in D1) wiring:

```yaml
- name: cf-mod validate cf-replay/schemas
  shell: bash
  run: cargo run --release -p cf-mod -- validate crates/cf-replay/schemas/
```

This is the explicit positive-control proof that on every PR, the M5 schema set passes the CLI validator. Currently the proof only exists in-process (via the cf-mod unit test `m5_all_shipped_schemas_validate`). Adding the CLI step takes ~5 seconds of CI time and is the single most leverage-rich CI gap closure available.

---

## Category 5: Snapshot family completeness

### E1. Snapshot completeness vs M5 families

**M5 snapshot inventory (after pass-1):**

```
snapshot_actor.json
snapshot_inventory.json
snapshot_terrain_chunk.json
snapshot_terrain_summary.json
snapshot_chassis.json
snapshot_hazard_grid.json
snapshot_affliction.json
snapshot_armor_layer.json
snapshot_atmospherics.json
snapshot_environment_signal.json
snapshot_armor.json
snapshot_internal.json
snapshot_concussion.json
snapshot_fluid.json
snapshot_origin.json
snapshot_shield.json          (M5-A1 NEW)
```

**Cross-check vs M5 family list:**

| Family | Snapshot present? | Notes |
|---|---|---|
| armor.* | ✓ `snapshot_armor.json` + `snapshot_armor_layer.json` | dual-level snapshot |
| internal.* | ✓ `snapshot_internal.json` | covers both organs + circuits |
| concussion.* | ✓ `snapshot_concussion.json` | per-actor dose + band; **also carries `internal_shock_dose` + `g_load_dose`** per spec (M4 § snapshot_concussion payload contract — see § B in M4.md) |
| internal_shock.* | (covered) | NOT a separate snapshot. The spec's M4 § snapshot_concussion payload contract explicitly carries `internal_shock_dose (robots only)`. The internal_shock module-damage state is captured in `snapshot_internal.json` (`circuits[].condition + applied_afflictions`). **No separate snapshot_internal_shock needed.** |
| fluid.* | ✓ `snapshot_fluid.json` | |
| origin.* | ✓ `snapshot_origin.json` | |
| hazard.* | ✓ `snapshot_hazard_grid.json` | |
| affliction.* | ✓ `snapshot_affliction.json` | |
| atmos.* | ✓ `snapshot_atmospherics.json` | |
| shield.* | ✓ `snapshot_shield.json` | **M5-A1 addition** — closed the gap pass-1 found |
| environment.* | ✓ `snapshot_environment_signal.json` | |
| thermal.* | **MISSING** | No `snapshot_thermal.json`. |
| combat.* (projectile_hit_mo) | (transient; not a snapshot family) | hit events are transient; no snapshot needed |
| audio.* (event_requested) | (transient; not a snapshot family) | audio requests are transient |

**Findings:**

1. **`snapshot_internal_shock` — NOT needed.** The internal-shock dose is in `snapshot_concussion.json` per the M4 § snapshot_concussion contract; the module-damage state is in `snapshot_internal.json` (robot circuits). Adding a separate `snapshot_internal_shock.json` would duplicate state. **Verdict: COMPLETE — no gap.**

2. **`snapshot_thermal` — MISSING.** `thermal.*` is a locked M5 family (3 event types) with NO corresponding snapshot. The state thermal.* describes:
   - `thermal.signature_changed` — per-actor thermal signature (K).
   - `thermal.heat_exchanged` — per-tile heat flow (could be very high-frequency).
   - `thermal.material_phase_change` — per-material phase transitions.

   M9 firehose contract (M4.md § "M9 firehose surface — what M4 MUST handle without renaming") doesn't explicitly list a `snapshot_thermal` row. But the other 9 deep-damage families ALL got snapshot placeholders, so this is asymmetric.

   - `thermal.signature_changed` state could piggyback on `snapshot_actor` (add a `thermal_signature_k` field). That requires editing `snapshot_actor`'s contract.
   - `thermal.heat_exchanged` state is per-tile; the natural carrier is `snapshot_terrain_chunk` or a dedicated `snapshot_thermal_grid`.
   - `thermal.material_phase_change` state is per-tile material; the natural carrier is `snapshot_terrain_chunk` or material registry.

   **Verdict:** **MINOR GAP — LOW SEVERITY.** A `snapshot_thermal.json` placeholder (mirror of the snapshot_atmospherics + snapshot_environment_signal pattern) would close the symmetry. Pass-1 added `snapshot_shield` for exactly this reason; `snapshot_thermal` is the symmetric companion.

   Recommend adding:

   ```json
   {
     "$schema": "http://json-schema.org/draft-07/schema#",
     "$id": "snapshot_thermal.v0.1",
     "title": "snapshot.snapshot_thermal payload",
     "description": "M5 § thermal.* family + M4 § M9 firehose surface. Placeholder payload at M5 (placeholder=true); M16 + M19 fill (material kernel + atmos heat-exchange). Per-actor thermal signature + per-tile heat-exchange budget + per-material phase state.",
     "type": "object",
     "required": ["tick"],
     "properties": {
       "tick": { "type": "integer", "minimum": 0 },
       "placeholder": { "type": "boolean" },
       "by_actor": { "type": "array", "description": "Per-actor thermal signature: { actor_id, signature_k }" },
       "by_tile": { "type": "array", "description": "Per-tile heat flux summary (M9 batched): { chunk_id, local_pos, temperature_k, heat_flux_j_per_s }" },
       "by_material": { "type": "array", "description": "Active material phase transitions: { material_id, position, from_phase, to_phase, progress_0_1 }" }
     }
   }
   ```

   plus registration in `cf-replay/src/schemas.rs::event_schema_for` under `("snapshot", "snapshot_thermal")`. ~30 minutes of work.

3. **Other M5 families covered.** No other snapshot is missing.

**Summary verdict:** **1 of 13 M5 families lacks a snapshot placeholder** (`thermal.*`). All other families have a snapshot placeholder. Pass-1 closed the shield gap; this is the one remaining symmetry break.

---

## Category 6: Cross-schema consistency

### F1. Field naming consistency

`grep -lE '"[a-z]+[A-Z][a-zA-Z]*":' game/crates/cf-replay/schemas/event/{armor,internal,concussion,internal_shock,fluid,origin,hazard,affliction,atmos,shield,environment,thermal,audio,combat_projectile_hit_mo}*.json`: 0 matches.

Every M5 schema uses **snake_case for all field names**. No camelCase keys found. Examples confirmed:

- `ap_factor` (not `apFactor`)
- `hit_zone` (not `hitZone`)
- `from_hp` / `to_hp` (not `fromHp`/`toHp`)
- `leak_rate` (not `leakRate`)
- `parent_hit_event_id` (not `parentHitEventId`)
- `latent_heat_consumed_j` (not `latentHeatConsumedJ`)

**Verdict: CLEAN.**

### F2. Type consistency on ID fields

Audited every M5 schema for ID-field type pattern:

**Pattern 1: `integer` only** — used for actor/world entities tracked by stable RecordId(u64):

- `actor_id` (payload): `{ "type": "integer" }` in all schemas where present
- `item_id`: `integer`
- `shooter_id`, `weapon_id`, `projectile_id`, `target_id`: `integer`
- `organ_id` (payload key): typed as `string` enum (15 organ names) in `internal.*` — NOT integer because organs are name-keyed; same for circuits
- `helmet_item_id`: `integer`
- `defeated_round_id`: `integer`
- `era_panel_id`, `schurzen_plate_id`, `panel_id`, `debris_record_id`, `record_id`: `integer`
- `repaired_by_actor_id`: `integer`
- `cause_event_id`, `source_event_id`, `source_hit_event_id`, `parent_hit_event_id`, `source_event_id`, `reaction_id`: `string` (event-id string format `<run>:<tick>:<seq>`)

**Pattern 2: `["integer", "string"]` (polymorphic)** — used for world-aggregate IDs whose flavor is producer-determined at M16/M19 ladder-up:

- `hazard_id`, `atm_id`, `pipe_id`, `electrolyzer_id`, `from_pipe_id`, `to_pipe_id`
- `material_id`, `material_at_impact`, `material`
- `source_module_id`, `module_id` (in `modules_hit`, `modules_affected`)
- `internal_shock_module_id` (in origin.shot_force_feedback)

**Pattern 3: `["integer", "string", "null"]`** — used for env-level optional IDs:

- `actor_id` (envelope): `["integer", "null"]` in most schemas; some legacy schemas accept `["integer", "string", "null"]`

**Verdict: CONSISTENT pattern.** The split is intentional:

- Actor-domain RecordIds are integer-only (cf-replay::RecordId(u64) discipline).
- World-aggregate IDs accept either flavor so M16/M19/M20 producers can pick at ladder-up time without bumping schemas.

The only thing worth pinning would be a brief comment in the spec or a `docs/plan/spec/id-type-policy.md` documenting the rationale. LOW priority.

### F3. Time field consistency

| Field | Type | Unit | Convention |
|---|---|---|---|
| `tick` (envelope + every schema) | `integer >= 0` | ticks | sim-deterministic |
| `expected_duration_ticks` (affliction.applied) | `integer >= 0` | ticks | sim-deterministic |
| `ko_duration_s` (concussion.ko_threshold_crossed) | `number >= 0` | seconds | human-facing wall-time |
| `duration_s` (shield.disrupted) | `number >= 0` | seconds | human-facing |
| `from_s` / `to_s` (origin.oxygen_supply_changed) | `number >= 0` | seconds | human-facing |
| `oxygen_loss_rate` (origin.helmet_breach) | `number` | rate per second (implicit) | rate |
| `latent_heat_consumed_j` | `number` | joules | scalar |
| `decompression_rate_pa_per_s` | `number` | Pa/s | rate |
| `moles_per_s` (atmos.pipe_flow) | `number` | mol/s | rate |
| `input_water_kg_per_s` / `output_*_kg_per_s` (atmos.electrolysis_started) | `number` | kg/s | rate |

**Pattern:** sim-deterministic durations use **ticks (integer)**; human-facing durations use **seconds (number)**. Rates are always `<unit>_per_s` and typed `number`.

**Verdict: CONSISTENT** — no drift. The mix is intentional and well-documented by field name (the `_ticks` / `_s` / `_per_s` suffixes are the in-schema documentation).

**Minor opportunity:** the spec might benefit from a short § Time-and-rate field naming convention in M5.md (or in a shared docs/ref). Currently the convention is implicit-by-discipline. LOW priority.

---

## Category 7: M5-A1 amendment naming

### G1. Amendment ID convention

**Prior precedent in git log (`git log --oneline --all --grep="A1\|A: ship\|M4A\|M11A\|M5-A"`):**

```
3ac7f4b M4A: ship asset-ledger foundation + close 7-axis audit gaps
1784ad2 M5-A1: post-audit hardening pass — 17 audit findings closed; ready for M6
```

`specs/active/` carries 14 `M<N>A`-style files: `M11A.md, M12A.md, M18A.md, M24A.md, M25A.md, M27A.md, M28A.md, M29A.md, M32A.md, M32B.md, M33A.md, M36A.md, M36B.md, M37A.md, M38A.md, M40A.md, M40B.md, M43A.md, M45A.md, M48A.md, M48B.md, M48C.md, M9A.md`.

**Two distinct conventions coexist in the repo:**

| Convention | Meaning | Repo evidence |
|---|---|---|
| **`M<N>A` / `M<N>B` / `M<N>C` (suffix letters)** | A **sister milestone** to M<N>, with its own spec file under specs/active/ or specs/done/. New scope; landed independently. | M4A (asset-ledger), M11A, M12A, M18A, …, M48C — 24+ files. |
| **`M<N>-A<n>` (hyphen-letter-number)** | An **amendment** to an already-closed milestone — post-audit hardening, fixup work, drift correction. NO new spec file; the original spec stays in specs/done/. | M5-A1 (the pass-1 hardening) — sole instance. |

**Is the M5-A1 naming self-consistent with prior amendments?** There is NO prior amendment naming precedent (M4A was a sister milestone with its own M4A.md spec, NOT an amendment to closed M4). The M5-A1 commit subject creates a new precedent. **AGENTS.md is silent** on amendment naming (it covers commit-subject-format but not the id surface for post-closure work).

**Verdict on the M5-A1 naming choice:**

- **Pros:** distinguishes amendment-class work from sister-milestone work. The `A1` suffix invites `A2` / `A3` for further audit passes (avoiding the "M5A → M5B → M5C" suffix-letter exhaustion that the sister-milestone convention faces).
- **Cons:** parses ambiguously to a reader who knows the M4A precedent. A skimmer might assume `M5-A1` is shorthand for a sister-milestone `M5A` and look for a spec file (which doesn't exist).
- **Net:** the naming is **self-consistent within the M5-A1 commit's scope** (it's the only use), but introduces a new convention without documenting it. Adding 2 lines to AGENTS.md (or a `docs/plan/conventions.md`) clarifying the distinction would close the ambiguity.

**Recommendation:** keep `M5-A1` (it's the more expressive convention for amendments), and optionally amend AGENTS.md or add a brief `docs/plan/amendment-id-conventions.md`. LOW priority.

---

## Recommended fixes

Listed in priority order. None block M5 closure (which is already done); all are forward-looking hygiene for the post-pass-2 state.

| Priority | Fix | Effort | Impact |
|---|---|---|---|
| **P0** | Update M9.md's stale `specs/active/M5.md` pointers to `specs/done/M5.md` (8 occurrences). | 1 minute | Closes "file not found" symptom for any reader following the M9 spec. |
| **P0** | Add explicit CI step `cf-mod validate cf-replay/schemas/` to `.github/workflows/ci.yml` (~2 lines of YAML). | 5 minutes | Catches CLI-surface regressions every PR. Currently the in-process test covers the validator logic but not the binary. |
| **P0** | Add `game/crates/cf-replay/tests/m5_round_trip.rs` integration test that records 1 event per family through the Recorder and re-validates each via `validate_event_payload`. | 30-60 minutes | Closes the docstring-promised "workspace test under `cf-replay/tests` walks a freshly-recorded smoke bundle". Currently the docstring is stale. |
| **P1** | Ship a `snapshot_thermal.json` placeholder for M9 firehose symmetry (mirror of `snapshot_shield` pattern). | 30 minutes | Closes the only M5-family-without-snapshot gap. Defers actual thermal kernel work to M16+M19 (no producer impact at M5). |
| **P1** | Decide on spec-amendment approach for M5.md (4 silent drifts: 23 affliction kinds, `schema_version` literal, `parent_hit_event_id` rename, `audio.event_requested` deliverable). Either (a) inline `## M5-A1 Amendment` section appended to spec, (b) edit the affected paragraphs in place, (c) document only in CHANGELOG. **Requires user instruction** per AGENTS.md doc-update rule. | 5 minutes (option a/b); 0 minutes (option c — user decision only) | Closes the spec-vs-code drift that downstream M6/M13 implementers will hit when reading M5.md literally. |
| **P1** | Decide on CHANGELOG entry for the current M5 + M5-A1. AGENTS.md global rule requires explicit user instruction. | 15 minutes (with prose) | Removes the misleading "M5" CHANGELOG entries that all reference the now-renumbered old M5. |
| **P2** | Add per-family negative tests to `cf-replay/src/schemas.rs::tests` (one bad-enum + one missing-required per family) — 10 families × 2 tests = 20 tests. | 60 minutes | Catches any future per-schema regression in family-specific enums. Currently only armor + combat + concussion have negative tests. |
| **P2** | Add `cf-mod/tests/validate_cli_integration.rs` that subprocesses `cargo run -p cf-mod -- validate cf-replay/schemas/` and asserts exit 0 + scanned=N pass=N. | 30 minutes | Catches CLI-surface output-format / exit-code regressions that the in-process unit tests do not cover. |
| **P2** | Persist the M5 + M5-A1 per-scenario verdict tables somewhere durable (e.g., `audit-m5-pass2/00-verdict-table.md`) so future sessions can reconstruct closure-evidence without re-running the audit. | 15 minutes | Closes the AGENTS.md step-6 "report a per-scenario verdict table" gap-of-durability. |
| **P3** | Add brief `docs/plan/amendment-id-conventions.md` clarifying `M<N>A` (sister-milestone) vs `M<N>-A<n>` (post-closure amendment). 1 paragraph each. | 10 minutes | Closes the convention-ambiguity § G1 raised. |
| **P3** | Add `docs/plan/spec/id-type-policy.md` (1 paragraph) documenting why some IDs are `integer`-only and others are `["integer", "string"]`. | 10 minutes | Closes the implicit-convention drift § F2 noted. |

---

## Summary

- **Hygiene verdict:** **FIX-RECOMMENDED.** AGENTS.md workflow is followed at the commit-history level. The two material gaps are (1) M9 stale pointers and (2) M5.md silent drift (4 spec-vs-code mismatches). Neither blocks anything; both pollute future-reader experience.
- **Test coverage verdict:** **ADEQUATE WITH GAPS.** The per-family happy-path test (14 families) closed the pass-1 gap; the residual gaps are (1) absent `cf-replay/tests/` round-trip test, (2) absent `cf-mod` CLI integration test, (3) light per-family negative coverage. Cargo test runs in CI; pass-1's new tests are exercised.
- **CI integration verdict:** **WIRED FOR LOGIC, NOT FOR CLI.** Schema-validation logic is exercised by `cargo test --workspace` (M5-A1 tests run automatically). Schema-validation CLI (`cf-mod validate cf-replay/schemas/`) is NOT wired in CI. Recommend adding 2 lines of YAML.
- **Snapshot completeness verdict:** **12 of 13 M5 families have a snapshot placeholder.** `snapshot_thermal` is the only missing one (pass-1 added `snapshot_shield`). `snapshot_internal_shock` is correctly NOT separate — that state is folded into `snapshot_concussion` (per M4 § snapshot_concussion contract).
- **Schema cross-consistency verdict:** **CLEAN.** Pure snake_case, ID type pattern is intentional and consistent, time/rate field naming convention is uniform. No silent drift across schemas.
- **Amendment-id naming verdict:** **DEFENSIBLE, UNDOCUMENTED.** `M5-A1` is the right precedent for amendment work (distinct from `M4A` sister-milestone work), but the convention isn't documented anywhere. 2 lines of doc would close that.

### Top 3 must-fix items

1. **Fix M9.md's 8 stale `specs/active/M5.md` pointers → `specs/done/M5.md`.** Mechanical bulk-replace. 1 minute.
2. **Add `cf-mod validate cf-replay/schemas/` step to ci.yml.** ~2 lines of YAML; closes the CLI-surface regression gap.
3. **Add `game/crates/cf-replay/tests/m5_round_trip.rs`** — closes the docstring-promised round-trip integration test that doesn't exist on disk today.

### Top 3 nice-to-have items

1. **Ship `snapshot_thermal.json` placeholder.** Closes the one symmetry break in M5-family snapshot coverage.
2. **Persist verdict tables to a durable file under `audit-m5-pass2/`.** Closes the AGENTS.md step-6 durability gap (currently the verdict tables exist only in chat output).
3. **Decide spec-amendment approach for M5.md silent drifts.** User-level decision per AGENTS.md doc-update rule.

### Cross-references

- M5 spec (closed): `specs/done/M5.md`
- M9 spec (sister, active): `specs/active/M9.md`
- M4 spec (parent envelope, done): `specs/done/M4.md`
- AGENTS.md: `/Users/erol/projects/corefall/AGENTS.md`
- CHANGELOG.md: `/Users/erol/projects/corefall/CHANGELOG.md`
- CI workflow: `/Users/erol/projects/corefall/.github/workflows/ci.yml`
- Pass-1 hardening commit: `1784ad2`
- Pass-1 audit findings: `audit-m5/{01..06}-*.md`
- M5 schemas (75 files after pass-1): `game/crates/cf-replay/schemas/event/`
