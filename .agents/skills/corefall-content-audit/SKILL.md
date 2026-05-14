---
name: corefall-content-audit
description: Per-milestone content-completeness audit for Corefall. Reads a single milestone spec from specs/active/ or specs/done/, builds a contract checklist of every concrete claim in the spec (named entities, cumulative content counts, files, cfctl methods, event schemas, acceptance scenarios, crate modifications), then verifies each item exists in the code with non-stub state. Distinguishes legitimate forward-compat placeholders from gaps. Returns CLEAN or NEEDS-FIXES with a per-item gap list. Skips T-CAPTURE, grading.json, BP-level reports, perf gates, and test-script execution — only checks content completeness and compile health.
when_to_use: After implementing a single milestone (M6 through M49 plus suffix variants like M9A, M11A, M12A). Trigger on prompts like "audit content for M9", "is M11 fully implemented", "verify M6 against its spec", "check M14 for partial implementation", "content audit M25". Do NOT use for BP-level closure review (use corefall-review for that). Do NOT use for end-of-launch quality review.
argument-hint: "<milestone-id-or-spec-path>"
---

# Corefall Content Audit

Audit milestone `$ARGUMENTS` for content completeness against its spec. **You are the auditor.** No helper scripts required (write your own ad-hoc bash/python if you need it, but the analysis is yours).

The verdict is binary:

- **CLEAN** — every concrete claim in the spec is implemented with non-stub code. Cumulative content counts match the spec. Compile + lint + cf-mod validate all green. Forward-compat placeholders are legitimate per the spec's own declarations. No undeclared deferrals.
- **NEEDS-FIXES** — at least one gap exists. Report every gap by name with file:line evidence.

## Scope rules (READ THIS FIRST)

**The milestone spec file is the ONLY source of truth.** Locate it:
- Active spec: `/Users/erol/projects/corefall/specs/active/<MX>.md`
- Done spec: `/Users/erol/projects/corefall/specs/done/<MX>.md`

If both exist, prefer `specs/done/` (it's the closed version the implementer just moved). If only `specs/active/` exists, use that.

**Do NOT consult** any of these as scope sources:
- `docs/plan/spec/prototype-roadmap.md` (outdated milestone naming + scope drift)
- `docs/plan/spec/native-implementation-backlog.md`
- `docs/plan/spec/feature-completion-checklist.md`
- `docs/plan/content-roster-tracking.md` (use as informational only; the spec's own content roster table is authoritative)
- `README.md`
- Any other milestone's spec, unless this milestone's spec explicitly cross-references it (e.g. "Schema definitions live in specs/active/M5.md § armor.* family").

If the spec is ambiguous on a specific point, **flag the ambiguity in the report and ask the user**. Do not invent scope. Do not fall back to the roadmap.

**Read the spec end-to-end.** Many milestones are 500-2500 lines. Skim is not enough. The contract is in the prose, the tables, the Gherkin scenarios, the "Files" list, the "Crates / modules touched" table, the "Out of scope" section, and the "Notes for the implementer" section together — not just the headline behavior.

## What's on the contract

Build a single checklist from the spec. Every item on the checklist is a verifiable claim. Items come from these locations in the spec:

### Source 1: "Player-facing behavior" section

Every named entity, count, and concrete behavior. Examples:

- "6 launch weapons (Rifle, SMG, Shotgun, Sniper, Pistol, Grenade Launcher)" → 6 named items to find in code
- "4 grenade types (Frag, Smoke, Flash, Stick)" → 4 named items
- "7 tools (Digger, Repair tool, Foam gun, Concrete gun, Welder, Drill, Multi-tool, Beacon, Sensor pulse)" → count the items in the list, regardless of the heading's "7" claim. If the list shows 9, the contract is 9.
- "8 active slots + 3 reserved tank slots" → 8+3 inventory slots, with the 3 tank slots reserved (locked) at this milestone if the spec says so
- "5 enemy AI archetypes (Rifleman, Sniper, Assault, Engineer, Spotter)" → 5 named archetypes

For each named entity, note:
- Its name (exactly as the spec writes it)
- The crate or registry it should live in (often clear from context; check the "Crates / modules touched" table if uncertain)
- Any specific properties the spec attaches (e.g. "Sniper / Charge mode 800ms", "Spotter calls reinforcements")

### Source 2: "Content roster at MX" table

Every cumulative count. Examples:

- "Weapons (toward 70+): 30 weapons cumulative" → at this milestone's close, the weapon registry has ≥ 30 distinct weapons
- "Music tracks (toward 30+): 18 music tracks cumulative" → `content/assets/audio/music/` has ≥ 18 tracks (or equivalent registry)
- "Codex entries (toward 600): 300 codex entries" → content/narrative/codex/ has ≥ 300 entries
- "Languages: English baseline" → `content/localization/en.json` exists; others may be empty

The cumulative number is a floor. Over-count is fine (informational, not a gap). Under-count is a gap.

If the spec's claim is "N+" or "≥ N" or "300+", the floor is N.

If a count is for production-track content (M9A SVG, M12A audio, M32A Tier 2 art), the spec may say "0 cumulative at this code milestone" — meaning the production track owns this content. Trust the spec; do not invent.

### Source 3: "Crates / modules touched" table

Every row is a contract:

- **Status: NEW** → the crate or module must be newly created
- **Status: MODIFY** → the named modifications must be present in the existing crate
- **Status: MODIFY (deep)** → substantial new code in the existing crate

For each row, the "What changes" cell describes specifically what new types, functions, fields, or behaviors must exist. Read it carefully and add each to the checklist.

### Source 4: "Files" list

Every file path listed must exist on disk with non-trivial content.

- **NEW** files: must exist with substantive code (not just `// TODO` or empty struct)
- **MODIFY** files: must exist (they already did) AND contain the described changes

### Source 5: cfctl surface

Every `act.*`, `observe.*`, `inspect.*`, `scenario.*`, `runbundle.*`, `system.*`, `sim.*` method named in the spec.

- Must be registered in `cf-control/src/server.rs` (or equivalent)
- Must dispatch to engine code, not stub-reject with `Err("not implemented")`
- For observe/inspect: must return real data, not hardcoded defaults

### Source 6: Event families

Every `category.event_type` named in the spec.

- Must have a schema file at `game/crates/cf-replay/schemas/event/<category>_<type>.json`
- Must be defined as an emittable event variant in cf-replay
- Must be emitted by the engine at the spec-described trigger point (grep the engine for the emission site)

### Source 7: Acceptance criteria Gherkin scenarios

Each `Scenario:` block in the "Acceptance criteria" section describes a concrete behavior:

- `Given X, When Y, Then Z`
- For each scenario, identify the code path that implements the Z-clause behavior
- The behavior must exist in the engine code, not just in a test file
- A unit test asserting the behavior is NOT evidence the behavior is implemented — it's evidence the test exists. Find the production code path the test exercises.

If the only code paying attention to a Gherkin scenario is a test file, the behavior is unimplemented. Gap.

### Source 8: Notes for the implementer

This section often contains specific contract details that aren't in the body. Examples from real specs:

- "stability cost = recoil_impulse / 200"
- "knockdown threshold = stability < 0.1 + impulse > 100"
- "DYING dwell = 1000ms (60 ticks at 60Hz)"
- "loudness_radius = 480 × (damage/10).clamp(1,3)"

Verify the numeric constants are present in the code (grep for the literal value or the named constant).

### Source 9: Schemas list

Every schema file path listed must exist on disk and validate cleanly via `cargo run -p cf-mod -- validate <path>`.

## The audit passes

Run these passes in order. Each pass produces gap-candidates that may or may not be legitimate after Pass 7.

### Pass 1: Build the contract checklist

Read the spec end-to-end. Extract checklist items from all 9 sources above. Write them down (in your scratch space, the report draft, or a tmp file — your call).

A reasonable milestone has 50-500 checklist items. M6 might have 150. M11 might have 400. M21 might have 30. Match your detail level to the spec's depth.

### Pass 2: Locate the implementation

For each checklist item, grep the codebase to locate the implementing code. Track:
- file:line where the item is defined
- file:line where the item is wired/registered/dispatched
- file:line where the item is consumed (if applicable — e.g. event emission site)

Items with no located implementation → gap-candidate.

### Pass 3: Verify non-stub state

For each located implementation, read the code. Look for partial-implementation smells:

- `todo!()`, `unimplemented!()` macros
- `panic!("not implemented")`, `panic!("TODO")`, `panic!("stub")`
- `// TODO`, `// FIXME`, `// XXX`, `// STUB`, `// HACK`, `// PLACEHOLDER`
- Function bodies that return `Default::default()`, `Self::default()`, `Vec::new()`, `None`, or `Ok(())` when the spec requires real work
- Trait method impls with empty bodies or `unimplemented!()`
- `placeholder=true` fields
- Magic-number sentinels (0, -1, "") where the spec requires real values
- Hardcoded test fixtures returned instead of real data

Each smell is a gap-candidate. Resolve in Pass 7.

### Pass 4: Cumulative content counts

For each "Content roster at MX" row, count the actual implementation:

- **Weapons**: count weapon definitions in cf-equipment (count `WeaponDef` instances OR weapon .rs files OR registry entries — pick whichever the codebase uses, look for an existing pattern). Compare to spec's cumulative claim.
- **Actors**: count actor archetype definitions / faction-specific actor types
- **Vehicles**: count vehicle definitions
- **Base objects**: count base module definitions
- **Factions**: count faction definitions in faction registry
- **Missions**: count `.ron` files under `game/content/scenarios/`
- **Worlds**: count world configs under `game/content/worlds/`
- **Biomes**: count biome configs (per-world × per-biome)
- **Materials**: count entries in material registry
- **Ores**: count ore definitions
- **Music**: count `.ogg` files in music asset path
- **SFX**: count `.ogg` files in SFX asset path
- **Narrative words**: count words across `content/narrative/` (rough wc -w is fine)
- **Codex entries**: count `.codex.ron` files
- **Achievements**: count achievement registry entries
- **Languages**: count `.json` localization tables
- **Endgame modes**: count mode definitions
- **Cosmetics**: count cosmetic registry entries

For each, do the math and write it in the report:
```
Weapons: spec says 30 cumulative; registry has 24 (need: 6 more — exact missing names: <list>)
```

Be specific. "24 of 30, 6 missing" not "short". Where possible, list the *names* of the missing ones by cross-referencing the spec's named-entity list.

### Pass 5: Compile + lint + content validate

Run, in this order:

```bash
cd /Users/erol/projects/corefall
cargo build --workspace --all-targets 2>&1 | tail -80
cargo test --workspace --no-run 2>&1 | tail -80
cargo clippy --workspace --all-targets -- -D warnings 2>&1 | tail -80
cargo run -p cf-mod -- validate content/ 2>&1 | tail -80
```

Any failure = gap. Capture the exact error message + file:line in the report.

**Do NOT** run `cargo test` (test execution). The user has explicitly deferred test-suite execution to BP12. You're auditing CONTENT and COMPILE, not running test suites.

Tests must compile (`--no-run` proves they compile). They do not have to pass at this milestone's audit.

### Pass 6: Cross-spec contract check (light)

Some surfaces are locked by earlier milestones and consumed by this one. Verify:

- **Event envelope conformance**: any new events this milestone fires must conform to the M4-locked envelope (`schema_version`, `run_id`, `tick`, `category`, `event_type`, `payload`, etc.). Check a sample.
- **Event family conformance**: if this milestone fires `armor.*` or `internal.*` or other M5-locked event families, schemas should match the M5 spec definitions. Check a sample.

This pass is NOT a full cross-spec analysis. It's a light spot-check. The deep cross-spec review happens at BP closure, not per-milestone.

### Pass 7: Forward-compat vs gap discrimination

For each gap-candidate from Passes 2-3, determine whether it's a legitimate forward-compat placeholder or a real gap.

**Legitimate forward-compat** requires BOTH:
1. The spec body explicitly declares the placeholder by name with shape (e.g. "BodySilhouette.placeholder=true at M11; M13 fills") OR the spec's "Out of scope" section names the future-owner milestone for that exact feature.
2. The placeholder code matches the declaration (the field exists, the type is right, the default value matches "placeholder").

If both conditions are met → LEGITIMATE, drop from gap list.

If either fails → GAP.

Examples of legitimate forward-compat (from real M6 spec):

```
✓ Actor::tank_primary / tank_secondary / tank_utility slots with slot_state="locked"
  — spec says "M6 reserves; M17 + M19 fill"; declared shape; matches.
✓ BodySilhouette.placeholder=true
  — spec says "M1 emits placeholder=true; M13 fills with chassis-backed data".
✓ AmmoRack module forward-compat in chassis spec
  — declared with placeholder=true; documented future-owner M13/M14.
```

Examples of NOT legitimate (real gap):

```
✗ todo!() in cover_seeking.rs:42 for a behavior the M7 spec mandates
  — not in "Out of scope", not declared as placeholder; gap.
✗ WeaponDef for "shotgun" missing entirely
  — spec lists in 6 launch weapons; gap.
✗ act.player.dig method registered but dispatches to Err("not implemented")
  — spec mandates dig; gap.
✗ Stub implementation that returns Default::default() for a calculation
  — spec mandates specific formula (e.g. impulse / 200); gap.
```

### Pass 8: Out-of-scope verification

For each "Out of scope" item in the spec:

- The item must name a future owner milestone (e.g. "Squad commands — M13+ chassis owns")
- If no owner is named, the deferral is malformed; flag as a spec ambiguity (not a code gap, but report it)
- If the future owner exists in the spec ladder, the deferral is legitimate
- If the future owner does not exist (e.g. "M99"), spec ambiguity; flag

## Report format

Use this exact format. Be concrete. Specific file paths and line numbers.

```markdown
# Content Audit Report — <MX>

Auditor: <agent identity + model + timestamp>
Spec source: specs/active/<MX>.md OR specs/done/<MX>.md
Spec line count: NNNN

## Contract checklist summary

Total items: NNN
- Player-facing behavior: NN
- Content roster: NN cumulative items
- Crates/modules touched: NN rows
- Files: NN paths
- cfctl methods: NN
- Event families: NN event types
- Acceptance scenarios: NN Gherkin scenarios
- Schemas: NN files

Implementation status:
- Implemented (verified): NNN
- Legitimate forward-compat: NN
- GAPS: N

## Gaps

(Only present if NEEDS-FIXES. Group by category.)

### Missing files
- `game/crates/cf-foo/src/bar.rs` — declared in spec "Files" list, does not exist
- `game/content/scenarios/m6_action_sweep.ron` — declared, does not exist

### Missing named entities
- Weapon "Shotgun" — spec lists in 6 launch weapons, no definition in cf-equipment/src/weapon/
- Tool "Welder" — spec lists in 7 tools, no entry in cf-equipment tool registry
- AI archetype "Spotter" — spec lists in 5 archetypes, no archetype definition in cf-ai/src/archetypes/

### Cumulative content shortfalls
- Weapons: spec claims 30 cumulative, registry has 24. Missing 6: <name1>, <name2>, ..., <name6>
- SFX: spec claims 90 cumulative, content/assets/audio/sfx/ has 65. Missing 25 (per spec's SFX list). Specific names: <list>
- Codex entries: spec claims 300, content/narrative/codex/ has 280. 20 short.

### Missing cfctl surface
- `act.player.dig` — declared in spec, not registered in cf-control/src/server.rs
- `observe.actor.silhouette` — declared, returns hardcoded empty struct (cf-control/src/server.rs:842)

### Missing event types
- `terrain.terrain_carved` — declared, no schema at cf-replay/schemas/event/terrain_carved.json
- `ai.archetype_chosen` — declared, never emitted in cf-ai (no emission site found)

### Partial implementations (NEEDS-FIXES)
- `cf-ai/src/cover_seeking.rs:42` — function body is `todo!()`; spec mandates implementation
- `cf-equipment/src/weapon/sniper.rs:128` — `fn charge_fire(&self) -> { Default::default() }`; spec mandates charge mechanism
- `cf-actor/src/limb_loss.rs:67` — `panic!("not implemented")` in arm_severed handler; spec mandates limb-loss action restrictions
- `cf-perception/src/footstep.rs:23` — returns `0.0` for loudness regardless of surface; spec mandates per-surface loudness modifier

### Missing acceptance-scenario behaviors
- Gherkin "Stealth kill instant-kill from behind" — spec scenario, no implementation in cf-equipment::stealth_kill or cf-actor (only a test exists, no engine code path)

### Compile / lint / content failures
- `cargo build --workspace --all-targets`: FAIL at cf-foo/src/bar.rs:10 — type mismatch (Vec<u32> vs Vec<i32>)
- `cargo clippy --workspace --all-targets -- -D warnings`: FAIL at cf-baz/src/qux.rs:25 — unused_variable (treated as error under -D warnings)
- `cargo run -p cf-mod -- validate content/`: FAIL — material registry missing required field `path_cost` on entry "concrete"

### Cross-spec contract drift (Pass 6 spot check)
- Event `combat.weapon_fired` payload missing field `recoil_impulse` (M4 envelope requires it on combat.* events with recoil)

### Spec ambiguities (flag for user)
- Spec line 1232: "approximately 5 archetypes" — is the floor 5 or some other number? Need confirmation.
- "Out of scope" item "Time stop ability — future" has no future-owner milestone named. Spec needs cleanup.

## Forward-compat placeholders (legitimate, NOT gaps)

(List these so the user knows you verified they're legitimate, not silently waived.)

- `Actor::tank_primary/secondary/utility` with slot_state="locked" — spec declares M6 reserves; M17 fills. ✓
- `BodySilhouette.placeholder=true` for non-chassis actors — spec declares M11 surface; M13 fills with chassis-backed. ✓
- `AmmoRack module` reserved fields — spec declares M13 chassis fills. ✓

## Out-of-scope items (legitimate deferrals)

- "Squad commands beyond 4 verbs" — spec out-of-scope, M13+ chassis owns. ✓
- "Multiple enemies in scenario" — spec out-of-scope, M13+ scenarios own. ✓

## Build / lint status

| Command | Result | Notes |
|---|---|---|
| `cargo build --workspace --all-targets` | PASS / FAIL | (failure detail if FAIL) |
| `cargo test --workspace --no-run` | PASS / FAIL | (tests compile? — no execution) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS / FAIL | (lint detail if FAIL) |
| `cargo run -p cf-mod -- validate content/` | PASS / FAIL | (content validation detail if FAIL) |

## Verdict

CLEAN
OR
NEEDS-FIXES — N gaps, fix all before closing milestone.

(If CLEAN, recommend the implementer: `git mv specs/active/<MX>.md specs/done/<MX>.md && git commit -m "<MX>: ..."`.)
(If NEEDS-FIXES, implementer fixes each gap and re-runs the audit until CLEAN.)
```

## Loop semantics

If the verdict is NEEDS-FIXES:
1. Implementer reads the gap list.
2. Fixes each gap in order. For each fix, verify against the spec — don't "fix" a legitimate forward-compat placeholder by implementing it now (that's scope creep).
3. Re-runs this audit.
4. Iterates until verdict is CLEAN.

**Halt criteria** for the loop:
- (a) Verdict = CLEAN → done; implementer commits + moves spec.
- (b) Same gap set 2 iterations in a row → halt, ask user (implementer is stuck or oscillating).
- (c) Iteration count > 8 → halt, ask user (likely spec ambiguity or larger architectural issue).
- (d) Only spec-ambiguity gaps remain → halt, ask user to clarify the spec.

When the loop halts via (b), (c), or (d), the milestone is NOT marked complete unless the user explicitly approves.

## What this skill does NOT do

The following are valuable but explicitly out of scope for this skill:

- T-CAPTURE evidence (`summary_grid.png`, `capture_manifest.json`, non_blank_ratio)
- AI-Agent Self-Test Report (Q1-Q7 prose answers)
- LLM-Graded Test Verdicts (`grading.json` with 8-15 dimensions per scenario)
- BP Goal Coverage Report (verbatim BP goal quoting)
- Self-Play Validation Matrix (Hands/Eyes/Ears/Hear rows)
- Universal Enhancement Audit (DR-056 14 universal rows)
- Per-tier perf gates (Steam Deck 800p/60, 1080p/60, 4K/120)
- 24h memory-leak soak
- Network sync verification (cfctl test sync-drift)
- Cross-platform CI matrix verification
- Run bundle validation against prototype_run_check.py
- `cargo test --workspace` (test EXECUTION — only --no-run to confirm compile)
- BP-level cross-milestone analysis
- Roadmap drift detection
- Decision-record closure status tracking
- Human-playtest survey row
- Capture-grid review

These belong in the existing `corefall-review` skill for end-of-BP / pre-launch quality review. This skill is purpose-built for **per-milestone content completeness**, fast iteration, and the user's specific concern: "every weapon/actor/feature in the spec must be fully implemented, nothing partial, nothing deferred unless spec explicitly defers it with a named future owner."

## When NOT to use this skill

- BP closure review (use `corefall-review`)
- Pre-launch quality gate (use `corefall-review`)
- Multi-milestone or cross-cutting analysis
- Subjective fun/feel evaluation
- Performance / memory / network testing

## Identity + auditability

Record at the top of every audit report:
- Agent identity (model name + version if visible)
- Timestamp
- Spec source path
- Spec line count (gives the user a quick sanity check that you read the whole thing)

This makes the audit traceable. Future re-audits can confirm whether the same gap exists.

## Final reminder

**You do the analysis.** Don't outsource counting to a helper script unless YOU decide one would help (then write it yourself, ad hoc). The user prefers your reasoning over scripted automation.

**Be concrete.** "Weapon X missing" not "weapons are short". "cf-ai/src/cover_seeking.rs:42 is todo!()" not "AI behavior is incomplete".

**Be honest.** If you can't tell whether a `placeholder=true` is legitimate, say so. Flag it. Don't guess CLEAN just to move on.

**Don't grade fun or feel.** That's the existing skill's job. Yours is binary: every claim implemented, or not.
