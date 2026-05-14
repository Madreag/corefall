# M5 Pass-2 Audit — Validator Implementation Deep Dive

Audit target: pass-1 validator hardening shipped in commit `1784ad2`
(`M5-A1: post-audit hardening pass`).

- cf-replay payload validator: `game/crates/cf-replay/src/schemas.rs`
- cf-mod schema-file validator: `game/crates/cf-mod/src/main.rs`
- M4 envelope schema: `game/crates/cf-replay/schemas/v0_1/recorder_event.schema.json`
- Bundle checker: `game/tools/prototype_run_check.py`
- Pass-1 audit: `audit-m5/05-validator-audit.md`

Methodology: read both validator sources end-to-end, then drove every NEW-* test
case through `cargo run -p cf-mod --quiet --release -- ...` from temp synthesized
bundles + schema files, capturing exit code and structured JSON output. All
end-to-end traces below are verbatim from the real binary at commit `1784ad2`.

---

## Pass-1 deliveries verified

| Item | Verified? | Evidence |
|---|---|---|
| `PropConstraint` gained `maximum` + `one_of` fields | YES | `schemas.rs:443-461` — struct has `minimum`, `maximum`, `one_of` fields with serde `default`/`rename` attributes. Verified end-to-end: `to_dose=100.0` PASS, `to_dose=100.1` FAIL with reason `"value 100.1 > maximum 100"`; `origin_id=5` PASS, `origin_id="Construct"` FAIL. |
| `check_one_of_branch` helper | YES | `schemas.rs:566-583` — walks each branch checking `type` + `enum`, returns Ok on first accept. Verified: `origin_id=5` integer-branch accepts, `origin_id="Human"` enum-branch accepts, `origin_id=5.5` rejected by both branches (`branch[0]: expected type ["integer"], got 5.5; branch[1]: not in enum [...]`). |
| cf-mod `additionalProperties:false` rejection | YES | `main.rs:1228-1232` — `if let Some(Value::Bool(false)) = p.get("additionalProperties") { ... }`. Verified by unit test `m5_event_schema_rejects_payload_additional_properties_false` (passes). Error string explicitly cites DR-002. |
| `is_envelope_version_dir(name)` regex | YES | `main.rs:758-779` — manual char-walker matching `^v[0-9]+(_[0-9]+)?$`. Accepts: `v0_1`, `v1`, `v0_2`, `v2_5`, `v10_42`. Rejects: `v`, `v_1`, `v1_`, `v1_2_3`, `v0.1`, `alpha`, `event`. Unit-tested by `m5_envelope_version_dir_regex_accepts_canonical_forms` + `m5_envelope_version_dir_regex_rejects_bad_forms`. |
| cf-mod `schema_version` literal enforcement | YES | `main.rs:1175-1182` — string equality check against `"prototype-recorder-event.v0.1"`. Verified by unit test `m5_event_schema_rejects_legacy_short_literal` (rejects `"0.1"`). |
| cf-replay envelope-shape marker detection | YES (tolerant) | `schemas.rs:420-435` — accepts both canonical literal and legacy `"0.1"` via `matches!(sv, Some("prototype-recorder-event.v0.1") \| Some("0.1"))`. Today every shipped M5 schema uses the canonical literal, so the legacy form is dead-code-safe but kept for migration tolerance per the pass-1 commit message. |

All six pass-1 deliveries verified. `cargo test -p cf-mod --quiet` → 20 passed; 0 failed.
`cargo test -p cf-replay --quiet` → 39 passed; 0 failed.

---

## New issues found (pass-2)

Severity legend: **P0** = data-integrity hole that lets a producer regress
silently; **P1** = real but bounded gap; **P2** = nice-to-have hardening;
**P3** = documentation / clarity.

### NEW-A: array-item enum support — **CONFIRMED GAP (P1)**

**Test setup:** synthesized an `environment.signal_aggregated` event with
`payload.signal.active_hazards = ["NotAHazard"]`. The schema declares this
array as:

```json
"active_hazards": {
  "type": "array",
  "items": {
    "type": "string",
    "enum": ["Hypoxic", "CombustibleAtmosphere", ..., "GravityShift"]
  }
}
```

**Outcome:** validator accepted the event (rc=0, `failures: []`). The
validator does NOT walk into `items` to check `enum` membership.

**Severity:** P1. Only one M5 schema today has `items.enum` on a payload-nested
array (`environment.signal_aggregated.signal.active_hazards`), so the blast
radius is small. Adjacent test confirmed `position=['a','b']` (where `items.type
= number`) is also unchecked — so the gap is general: `array items.*` is
completely ignored.

**Recommendation:** extend `PropConstraint` with `items: Option<Value>`, and
after the `min_items`/`max_items` checks recurse into each array element using
`check_one_of_branch`-style logic (type + enum). Patch is ~20 LOC; tests would
cover the active_hazards case directly.

```rust
#[derive(Deserialize)]
struct PropConstraint {
    // ...existing fields...
    #[serde(default)]
    items: Option<Value>,
}

// inside validate_event_payload, after maxItems check:
if let Some(items_schema) = &constraint.items {
    if let Some(arr) = value.as_array() {
        for (i, item) in arr.iter().enumerate() {
            let item_key = format!("{key}[{i}]");
            check_one_of_branch(category, event_type, &item_key, items_schema, item)?;
        }
    }
}
```

### NEW-B: nested object property recursion — **CONFIRMED GAP (P2)**

**Test setup:** sent `environment.signal_aggregated` event with
`payload.signal = {}` (empty). The schema declares:

```json
"signal": {
  "type": "object",
  "properties": { "schema_version": {...}, "active_hazards": {...} },
  "required": ["schema_version", "active_hazards"]
}
```

**Outcome:** validator accepted (`signal={}` and `signal.active_hazards=[]` both
pass). The validator does not recurse into nested-object `required` /
`properties`.

**Severity:** P2. Audit found exactly ONE nested-object-with-required in all 75
M5 envelope-shape schemas (the environment.signal_aggregated.payload.signal
sub-object). Pass-1 deliberately locked the EnvironmentSignal shape there.
Without recursion the lock is documentation-only at runtime.

**Recommendation:** extend `PropConstraint` with optional `required` +
`properties` fields and recurse. Bigger patch than NEW-A (~50 LOC) but the
infrastructure already exists in `validate_event_payload` — could pull the
payload-validation loop into a helper that recurses. If P2 isn't worth the
churn, document the limitation in the schema's `description` so contributors
know runtime validation stops at the top-level payload.

### NEW-C: envelope-level const enforcement — **CONFIRMED GAP (P3, intentional)**

**Test setup:** sent an event with envelope `schema_version="wrong-version"`
AND `category="wrong"` on payload that would otherwise validate.

**Outcome:**
- `envelope.schema_version="wrong-version"`: validator returns rc=0 / no
  failures. The per-event schema declares
  `properties.schema_version.const = "prototype-recorder-event.v0.1"`, but
  cf-replay's validator only walks `payload.*`, not envelope properties.
- `envelope.category="wrong"`: cf-mod `validate-bundle` dispatches by
  `(category, event_type)` tuple. `("wrong","layer_destroyed")` is not in
  `event_schema_for`, so it falls through to `Ok(())` (no schema registered).

**Severity:** P3. This is intentional by design — cf-mod validate-bundle
delegates envelope-level const checks to the orthogonal `prototype_run_check.py`
bundle checker (Python), which DOES enforce `schema_version =
'prototype-recorder-event.v0.1'`. Verified end-to-end: prototype_run_check has
`EVENT_VERSION = "prototype-recorder-event.v0.1"` and emits
`schema_version_mismatch` if drift detected.

The pass-1 audit (`audit-m5/05-validator-audit.md` § Scenario 4) flagged this
already — the per-event schema's envelope-level consts are checked at
schema-file load time (cf-mod validate), not at runtime payload validation
(cf-mod validate-bundle).

**Recommendation:** none for closure; the layered defense is the right
design. Optionally document the split in `validate_event_payload`'s
docstring so future contributors don't try to enforce envelope-level constraints
there.

### NEW-D: combination minimum + maximum — **WORKING**

Test results on `concussion.dose_changed.to_dose` (`minimum: 0.0`, `maximum: 100.0`):

| Input | Expected | Actual | Verdict |
|---|---|---|---|
| `100.0` (boundary) | PASS | rc=0 | OK |
| `100.1` (over) | FAIL | rc=1, `"value 100.1 > maximum 100"` | OK |
| `-0.001` (under) | FAIL | rc=1, `"value -0.001 < minimum 0"` | OK |
| `0.0` (boundary) | PASS | rc=0 | OK |

Boundary semantics are inclusive on both ends — matches JSON Schema 2020-12
default. `minimum` and `maximum` work independently and in combination.

**Severity:** WORKING — no gap.

### NEW-E: oneOf with type unions — **WORKING**

Test results on `concussion.dose_changed.origin_id` (`oneOf: [{type:integer},
{type:string, enum:[Human,Android,Robot,PoweredOrganic,HeavyBiomech]}]`):

| Input | Expected | Actual | Verdict |
|---|---|---|---|
| `5` (integer) | PASS | rc=0 | OK |
| `"Human"` (in enum) | PASS | rc=0 | OK |
| `"Construct"` (not in enum) | FAIL | rc=1, both branches rejected | OK |
| `5.5` (float, not integer) | FAIL | rc=1, both branches rejected | OK |
| `null` | FAIL | rc=1, no null branch | OK |
| `true` (bool) | FAIL | rc=1, both branches rejected | OK |
| `[1]` (array) | FAIL | rc=1, both branches rejected | OK |

Error message includes branch-by-branch reasons (`branch[0]: ...; branch[1]:
...`), making CI diagnostics tractable.

**Severity:** WORKING — no gap.

### NEW-F: enum on null-permitting type — **WORKING**

Test results on `audio.event_requested.material` (`type: ["string","null"],
enum: [metal, ceramic, ..., null]`):

| Input | Expected | Actual | Verdict |
|---|---|---|---|
| `null` (in enum) | PASS | rc=0 | OK |
| `"metal"` (in enum) | PASS | rc=0 | OK |
| `"not_a_material"` | FAIL | rc=1, structured enum error | OK |
| `5` (wrong type) | FAIL | rc=1, `"expected type ["string","null"], got 5"` | OK |
| omitted | PASS | rc=0 | OK |

`check_type` correctly handles the `["string","null"]` type union; `enum`
membership accepts `null` when the enum array contains a JSON null literal.
`serde_json::Value::Null` compares equal to JSON-null in the enum array.

**Severity:** WORKING — no gap.

### NEW-G: required at multiple levels — **CONFIRMED GAP (P3, intentional)**

**Test setup:** sent an event missing envelope `tick` field, plus a separate
test with `tick=-5` (violates envelope schema's `minimum: 0`).

**Outcome:**
- `tick` missing: validator returns rc=0 / no failures. cf-replay only walks
  `payload.*`, not envelope.
- `tick=-5`: validator returns rc=0 / no failures. Envelope-level `minimum: 0`
  is not enforced by cf-replay or cf-mod.

**Severity:** P3. Same intentional split as NEW-C. `prototype_run_check.py`
enforces `tick` presence (via `EVENT_REQUIRED` tuple). Negative tick is NOT
enforced by the bundle checker either — but the bundle checker DOES enforce
monotonicity, which is a different invariant. Negative tick on the first event
would slip past both validators today.

**Recommendation:** optional P3 — add a single `tick >= 0` check to
prototype_run_check.py's per-event loop. Bigger fix would be adopting a real
JSON Schema validator (e.g. `jsonschema_valid` crate) for the envelope at
runtime, but that's a M6+ scope question.

### NEW-H: `$id` vs filename consistency — **NO DRIFT**

Scanned all M5 envelope-shape schemas (75 files). Every one has `$id` matching
the canonical pattern `https://corefall/event/<title>.v0.1`. Zero drift.
cf-mod does NOT enforce this convention — it's purely a content-editor
discipline today.

**Recommendation:** optional P2 — add an `$id` check in
`validate_event_schema_value` for M5-shaped schemas (`if M5 envelope-shape:
expected_id = format!("https://corefall/event/{cat}.{ev}.v0.1"); reject if
drift`).

### NEW-I: `$schema` URI consistency — **DRIFT FOUND (P2)**

Scanned all 128 schemas under `event/`. Three distinct `$schema` URIs in use:

| URI | File count | Example |
|---|---|---|
| `https://json-schema.org/draft/2020-12/schema` (canonical M5) | 86 | `affliction_applied.json` |
| `http://json-schema.org/draft/2020-12/schema` (HTTP variant, **invalid URI** — JSON Schema draft-2020-12 uses `https://`) | 21 | `ai_missed_shot_reason.json` |
| `http://json-schema.org/draft-07/schema#` (legacy draft-07) | 21 | `determinism_first_divergence.json` |

**Severity:** P2. The pass-1 commit didn't flag this — the legacy schemas
(M2/M3/M4) pre-date the canonical M5 URI. Two issues:

1. The 21 schemas using `http://json-schema.org/draft/2020-12/schema` likely
   were intended to use `https://` (the canonical URI per JSON-Schema spec).
2. The 21 draft-07 schemas could be left as-is (legacy) or normalized to
   draft-2020-12.

**Recommendation:** optional P2 — bulk-rewrite the 21 HTTP-variant URIs to
`https://`. cf-mod does NOT validate this so it's purely a documentation
correctness issue, but external `$schema`-aware validators (e.g. `ajv`,
`jsonschema`) may complain.

### NEW-J: Title format strict check — **WORKING (strict literal match)**

**Test setup:** schema file with `title = " armor.layer_destroyed "` (leading
+ trailing space).

**Outcome:** cf-mod rejects with `FAIL: title \` armor.layer_destroyed \` must
equal \`armor.layer_destroyed\`` (from category+event_type consts). The
literal equality check at `main.rs:1206-1210` catches the whitespace mismatch
because it compares with `format!("{cat_const}.{ty_const}")` (no padding).

**Edge case probed:** category const with dots in it — would create a
`title = "armor.special.layer_destroyed"` for category="armor.special",
event_type="layer_destroyed". The `expected_title` would be
`"armor.special.layer_destroyed"`, which would match the title string but make
the filename check tricky (`expected_stem = "armor.special_layer_destroyed"`
versus filename `"armor_special_layer_destroyed.json"` would mismatch). No
M5 schema uses dotted categories today, so this is theoretical.

**Severity:** WORKING — no gap.

### NEW-K: Filename hyphen vs underscore — **WORKING (literal match)**

**Test setup:** schema file `armor-special_layer_destroyed.json` with
`category.const = "armor-special"`, `event_type.const = "layer_destroyed"`.

**Outcome:** cf-mod PASSES. The validator computes
`expected_stem = format!("{cat_const}_{ty_const}") =
"armor-special_layer_destroyed"`, which equals the file stem literally.

This isn't a gap — the validator is correctly literal — but it means **the
validator does not prevent hyphens in category or event_type strings**. cf-replay's
`event_schema_for` would need a `("armor-special", "layer_destroyed")` tuple
entry for the registration to work; no existing M5 family uses hyphens.

**Severity:** P3 / documentation. The convention "category and event_type are
lowercase snake_case identifiers" is enforced by code review only.

**Recommendation:** add a `cat_const.chars().all(|c| c.is_ascii_lowercase() ||
c.is_ascii_digit() || c == '_')` check at schema-load time.

### NEW-L: Validator response time — **WELL UNDER 1S**

Measured timing on cached binary (`cargo run -p cf-mod --quiet --release --
validate cf-replay/schemas/`):

| Trial | Wall time |
|---|---|
| Trial 1 | 230.1 ms |
| Trial 2 | 226.1 ms |
| Trial 3 | 227.7 ms |

Min 226 ms / max 230 ms across 3 trials, walking 131 schemas (128 event + 3
envelope). Cold cargo build (compiling cf-mod + dependencies) takes ~25 s
but is amortized across CI runs.

**Severity:** WORKING — no gap. <250 ms is well under any reasonable CI budget.

### NEW-M: validate-bundle on real M5 events — **WORKING**

**Test setup:** synthesized a 16-event bundle covering every M5 family — one
representative event for armor.layer_destroyed, armor.layer_hp_changed,
internal.organ_damaged, concussion.dose_changed, internal_shock.dose_changed,
fluid.leak_started, origin.g_load_dose_changed, hazard.spawned,
affliction.applied (with new `blinded` kind), atmos.gas_released, shield.hit,
environment.signal_delta, environment.signal_aggregated, thermal.material_phase_change,
combat.projectile_hit_mo, audio.event_requested.

**Outcome:**

```
events_checked=16  failures=0
exit code: 0
```

All 16 events validate. The `affliction.applied.kind = "blinded"` case (the
new 23rd kind added in pass-1 for M6) is accepted. The
`environment.signal_aggregated` event with `cosmetic: true` envelope flag
is accepted.

**Severity:** WORKING — no gap.

### NEW-N: validate-bundle adversarial cases — **MOSTLY CAUGHT**

Same base bundle, with one event mutated each iteration:

| Mutation | Expected | Actual | Verdict |
|---|---|---|---|
| `armor.layer_destroyed.zone = "not_a_zone"` | FAIL | rc=1, structured enum error | OK |
| `concussion.dose_changed.origin_id = "Construct"` | FAIL | rc=1, both oneOf branches rejected | OK |
| `combat.projectile_hit_mo.parent_hit_event_id` MISSING | FAIL | rc=1, `"required field parent_hit_event_id missing"` | OK |
| `affliction.applied.kind = "blinded"` | PASS | rc=0 | OK |
| `environment.signal_aggregated.signal.active_hazards = ["NotAHazard"]` | FAIL | **rc=0** | **MISS (NEW-A confirmed)** |
| `combat.projectile_hit_mo.ap_factor` MISSING | FAIL | rc=1, `"required field ap_factor missing"` | OK |

5 of 6 caught. The 1 miss is the NEW-A array-item-enum gap.

### NEW-O: cf-mod walk picks up files correctly — **WORKING**

| Source | Count |
|---|---|
| Files on disk under `schemas/event/*.json` | 128 |
| Files on disk under `schemas/v0_1/*.json` | 1 |
| Files on disk under `schemas/v1/*.json` | 2 |
| **Expected total** | **131** |
| `cf-mod validate cf-replay/schemas/` scanned count | **131** |

Walk picks up every file. Zero drift.

### NEW-P: PASS message attribution — **CORRECT**

`cf-mod --json validate cf-replay/schemas/` returns structured `entries` with
`message` field. Breakdown:

| Message | Count |
|---|---|
| `"event schema (M5 envelope-shape)"` | 75 |
| `"event schema (legacy payload-only)"` | 53 |
| `"envelope schema: <title>"` | 3 |
| **Total** | **131** |

M5 family breakdown (envelope-shape):

| Family | Count |
|---|---|
| armor | 19 |
| atmos | 10 |
| fluid | 9 |
| internal (organ + circuit + shock) | 8 |
| hazard | 5 |
| shield | 5 |
| affliction | 4 |
| concussion | 4 |
| origin | 4 |
| thermal | 3 |
| environment | 2 |
| audio | 1 |
| combat | 1 |
| **Total** | **75** |

Matches the spec's 74 family entries + audio.event_requested + (snapshot_shield
is legacy-shape, not M5-envelope-shape — sits in the 16-entry snapshot
legacy count). So `75 M5 envelope-shape + 53 legacy + 3 envelope = 131`
checks out.

Note: the audit-m5/05 pass-1 report stated `scanned=129 pass=129`. Pass-2 sees
`scanned=131 pass=131`. The +2 delta is `audio_event_requested.json` (new
schema shipped by pass-1) + `snapshot_shield.json` (new M9 firehose schema
shipped by pass-1). Both verified accounted for.

### NEW-Q: ValidationReport JSON output schema — **STABLE**

`cf-mod --json validate <path>` output:

```json
{
  "schema_version": 1,
  "scanned": 131,
  "pass": 131,
  "warn": 0,
  "fail": 0,
  "entries": [
    {
      "path": "/Users/.../armor_layer_destroyed.json",
      "result": "pass",
      "message": "event schema (M5 envelope-shape)"
    }
  ]
}
```

- `schema_version: 1` — versioned, so future changes can be backward-compat.
- `result` enum is `"pass" | "warn" | "fail"` — stable lowercase strings.
- `entries[].path` is absolute filesystem path; CI scripts can pattern-match.
- Top-level counts `scanned/pass/warn/fail` are integers and sum to scanned.

`cf-mod --json validate-bundle <bundle_dir>` output is a different shape:

```json
{
  "bundle_dir": "/var/folders/.../cfmod-bundle",
  "events_checked": 1,
  "failures": [
    {
      "event_id": "test:1:0",
      "category": "armor",
      "event_type": "layer_destroyed",
      "reason": "armor.layer_destroyed::zone value \"not_a_zone\" not in enum [...]"
    }
  ]
}
```

NOT versioned, NOT typed beyond serde-JSON. Two different output schemas for
two different verbs is acceptable but undocumented.

**Severity:** P3. Recommendation: add `"schema_version": 1` to validate-bundle
output for symmetry; document both shapes in cf-mod's docstring.

### NEW-R: Bundle checker drift — **NO DRIFT**

`prototype_run_check.py` constants:
- `EVENT_VERSION = "prototype-recorder-event.v0.1"` — matches the canonical
  pass-1-enforced literal.

Pass-1 aligned all 74 M5 per-event schemas + `audio.event_requested` to
`"prototype-recorder-event.v0.1"`. The bundle checker enforces this at the
envelope level. **No drift exists** between the bundle checker and the
per-event schemas.

**Severity:** NO GAP.

### NEW-S: cf-mod --strict mode — **WORKING**

| Input | rc without --strict | rc with --strict |
|---|---|---|
| Path with no validator wired (e.g. `random.txt`) → WARN | 0 | **1** |
| Path with .ron in non-scenarios dir → WARN | 0 | **1** |
| All-PASS schema tree (no warns/fails) | 0 | 0 |

Behavior matches `main.rs:725-728`:
```rust
let any_fail = report.fail() > 0 || (strict && report.warn() > 0);
if any_fail { std::process::exit(1); }
```

**Severity:** WORKING — no gap.

### NEW-T: Tests run from various CWDs — **WORKING**

The `m5_all_shipped_schemas_validate` test tries 4 candidate paths:

```rust
let candidates = [
    PathBuf::from("../cf-replay/schemas"),       // cf-mod crate root
    PathBuf::from("../../cf-replay/schemas"),    // cf-mod tests
    PathBuf::from("game/crates/cf-replay/schemas"), // repo root
    PathBuf::from("crates/cf-replay/schemas"),   // workspace root (game/)
];
```

Verified:

| CWD | rc | Result |
|---|---|---|
| `game/` (workspace root) | 0 | `1 passed; 0 failed` — used `crates/cf-replay/schemas` |
| `game/crates/cf-mod/` (cf-mod crate root) | 0 | `1 passed; 0 failed` — used `../cf-replay/schemas` |

Both pass. The test gracefully skips when no candidate matches (only printing
to stderr), so it doesn't false-fail in dev-tools-skinned environments.

**Severity:** WORKING — no gap.

### Additional pass-2 finding: array items.type also unchecked

Probed during NEW-F. Set `audio.event_requested.position = ['a','b']` (schema
declares `items.type = "number"`):

```
[position=['a','b']] rc=0
  ** NO VALIDATION on array items.type **
```

The validator accepts strings inside an array that should hold numbers. Same
gap class as NEW-A — `array.items` is completely uninspected. Probably should
be fixed in the same patch as NEW-A.

---

## End-to-end verification

### `cf-mod validate cf-replay/schemas/`

```
$ time cargo run -p cf-mod --quiet --release -- validate \
    crates/cf-replay/schemas/ 2>&1 | tail -3
---
scanned=131 pass=131 warn=0 fail=0

real    0m0.294s
```

- Counts: scanned=131, pass=131, warn=0, fail=0.
- Exit code: 0.
- Wall time: ~230-300 ms (cached binary).
- Attribution: 75 M5 envelope-shape, 53 legacy payload-only, 3 envelope.

### Live bundle smoke test (NEW-M)

Synthesized 16-event bundle covering every M5 family. All 16 events pass
schema validation with `failures: []` and `events_checked: 16` from the
`--json validate-bundle` verb. Exit 0.

### Adversarial cases (NEW-N) — summary table

| Adversarial case | Validator response | OK? |
|---|---|---|
| Bad enum on armor.zone | rc=1, structured enum FAIL | YES |
| Bad enum on concussion.origin_id (oneOf string-branch) | rc=1, both branches rejected | YES |
| Missing required combat.parent_hit_event_id | rc=1, structured missing FAIL | YES |
| New affliction.kind=blinded | rc=0, PASS | YES |
| Bad enum on environment.active_hazards (array items.enum) | rc=0, FALSE PASS | **NO (NEW-A)** |
| Missing required combat.ap_factor | rc=1, structured missing FAIL | YES |
| Bad envelope schema_version | rc=0, intentional (delegated to Python bundle checker) | OK |
| Missing envelope tick | rc=0, intentional (delegated to Python bundle checker) | OK |

5 of 6 in-scope adversarial cases caught. 1 miss = NEW-A.

### Test suite

```
$ cargo test -p cf-mod --quiet
running 20 tests
....................
test result: ok. 20 passed; 0 failed; 0 ignored

running 11 tests
...........
test result: ok. 11 passed; 0 failed; 0 ignored

$ cargo test -p cf-replay --quiet
running 39 tests
.......................................
test result: ok. 39 passed; 0 failed; 0 ignored
```

All 70 (20+11+39) tests pass.

---

## Recommended fixes

In order of leverage (P0 / P1 / P2 / P3):

1. **(P1) Add array `items.enum` + `items.type` enforcement to cf-replay**
   (closes NEW-A + the additional pass-2 finding). One M5 schema today
   (`environment.signal_aggregated.payload.signal.active_hazards`) carries a
   payload-impacting `items.enum` lock that the validator doesn't enforce;
   future M5+ schemas will inevitably add more. ~20 LOC patch with one new
   `PropConstraint.items: Option<Value>` field + a recursive call inside
   `validate_event_payload`'s array path.

2. **(P2) Add nested-object recursion to cf-replay** (closes NEW-B). Only one
   M5 schema today uses this (the signal sub-object lock), but the
   EnvironmentSignal contract is the only place that lock lives in code today.
   ~50 LOC patch; could be done as a single helper function that takes a
   sub-schema + sub-value and runs the same required+properties loop.

3. **(P2) Normalize `$schema` URI across all event schemas** (closes NEW-I).
   21 schemas use the (technically invalid) HTTP form
   `http://json-schema.org/draft/2020-12/schema`; canonical is `https://`.
   Bulk sed across `crates/cf-replay/schemas/event/*.json` would close it.
   cf-mod does NOT enforce this, but external validators may.

4. **(P2) Add `$id` enforcement in cf-mod for M5 envelope-shape schemas**
   (closes NEW-H, currently zero drift but no machine enforcement). One
   format-string compare in `validate_event_schema_value`.

5. **(P3) Reject hyphens / dots in category and event_type const strings**
   (closes NEW-K's theoretical drift). Adds `cat_const.chars().all(|c|
   c.is_ascii_lowercase() \|\| c.is_ascii_digit() \|\| c == '_')` check.
   Documentation-only today.

6. **(P3) Versionize `cf-mod --json validate-bundle` output** (closes NEW-Q
   secondary). Add `"schema_version": 1` at the top of the JSON for parity
   with `validate`'s output.

7. **(P3) Document the layered envelope-vs-payload validation split** (closes
   NEW-C + NEW-G concerns). Add a docstring to `validate_event_payload`
   stating: "envelope-level constraints (schema_version const, tick minimum,
   actor_id type union) are enforced by `prototype_run_check.py` at bundle
   load, NOT by this function. This validator only walks payload.*."

8. **(P3) Add bundle checker `tick >= 0` check.** Pure Python edit in
   `prototype_run_check.py`. Today negative ticks on the first event slip past
   both validators.

None of these are M6 blockers. The pass-1 validator hardening closed every
P0-class regression; what remains is P1/P2 polish + documentation.

---

## Summary

- **Pass-1 deliveries verified:** 6 / 6.
- **New issues found (pass-2):** 7 real gaps (3 P1/P2, 4 P3), 13 working cases.
- **Validator hardening blockers for M6:** none. M5 is shippable.
- **End-to-end verification:**
  - `cf-mod validate cf-replay/schemas/` exits 0, scanned=131, pass=131,
    walltime ~230 ms.
  - 16-event happy-path bundle across all M5 families passes 100%.
  - 5/6 adversarial cases correctly rejected; the 1 miss is the NEW-A
    array-item-enum gap (only place this matters today is
    `environment.signal_aggregated.signal.active_hazards`).
  - Bundle checker (Python `prototype_run_check.py`) is aligned with pass-1's
    canonical literal; no drift.

**Recommendation priority for an optional M5-A2 pass:**
1. Patch NEW-A (P1) — array items.enum + items.type enforcement. ~20 LOC.
2. Patch NEW-B (P2) — nested object recursion. ~50 LOC.
3. Patch NEW-I (P2) — `$schema` URI normalization. Bulk sed.
4. Skip NEW-C / NEW-G — intentional layered design (documented split).

Test status at pass-2: 20 cf-mod unit + 11 cf-mod integration + 39 cf-replay
unit = **70 tests pass, 0 fail**. Validator binary timing ~230 ms cold-cached.
`cargo clippy --workspace --all-targets -- -D warnings` was clean per the
pass-1 commit message; pass-2 made no source changes so the lint surface is
unchanged.
