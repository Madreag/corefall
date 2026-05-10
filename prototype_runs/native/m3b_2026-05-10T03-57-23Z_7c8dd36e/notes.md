# M3B Replay Viewer And Debrief — Audit-Closure Evidence

This directory replaces the prior `m3b_2026-05-10T01-37-50Z_c078e31d/` evidence
dir after the M3B audit (verdict: Needs Fixes) flagged 8 blockers, highs, and
mediums on 2026-05-09. Every fix is exercised here:

| Audit Finding | Severity | Fix | Evidence |
|---|---|---|---|
| `self_play_sweep.sh` exits 0 even with FAIL rows (counter in piped subshell) | BLOCKER | Tally counters in parent shell; build report into a string, then tee | `game/tools/self_play_sweep.sh:480-512` |
| Roadmap requires viewer/death-recap **screenshot** evidence; CLI emitted markdown only | BLOCKER | New `--png` flag on view / cause-chain / debrief; `game/tools/markdown_to_png.py` Pillow renderer | Every `*.png` file in this directory |
| `Bundle::load` PASSed corrupt bundles (bad schema, dup ids, non-object payload, stale count maps) | BLOCKER | 8 new BundleError variants + strict validation matching `prototype_run_check.py` | `bundle::tests::rejects_bad_*` (9 new tests) |
| `external:` parent prefix rejected by viewer (allowed by checker) | HIGH | Skip parent-resolve check for `external:` prefix | `bundle::tests::accepts_external_parent_prefix` |
| M3B-D02 overclaimed: no real run bundle has `actor_died` | HIGH | Synthetic fixture `crates/cf-tools-replay-viewer/tests/fixtures/m3b_actor_died_chain/` + 4 integration tests | `fixture_actor_died_*.{md,png}` here |
| Authoritative E2E command `cargo run -p cf-tools-replay-viewer -- <bundle>` exited 2 | HIGH | Added bare-bundle shorthand → equivalent to `debrief <bundle>` | `--help` long_about + main.rs dispatch |
| `--since-event-id` highlighting used lexicographic compare (`10` < `9`) | MEDIUM | Parse `(tick, seq)` from event_id, compare numerically | `viewer::tests::since_event_id_uses_numeric_tick_seq_not_lexicographic` |
| `cfctl replay scrub` documented but absent | MEDIUM | New `cfctl replay {view,scrub,cause-chain,debrief,validate}` subcommand proxies to viewer | `cfctl/src/main.rs:cmd_replay` |

## Evidence Layout

The evidence has two halves: **real BP2 bundle** (M2.5 micro_reactor_defense
loss path; the same bundle the M3A headless replay verifier replays) and
**synthetic actor_died fixture** (committed under
`game/crates/cf-tools-replay-viewer/tests/fixtures/m3b_actor_died_chain/`,
since real BP2 bundles have no actor_died — the player survives every
canonical fun-proof scenario).

### M2.5 bundle evidence (real run-bundle path)

| File | Subcommand | What it proves |
|---|---|---|
| `m2_5_validate.txt` | `validate` | Real BP2 bundle passes the strict schema-version + count-map + parent-chain checks. |
| `m2_5_debrief.{md,png}` | `debrief` | M3B-003 outcome (`lost` / `reactor_destroyed` at tick 1095), defend_reactor objective failed, 23 projectile_hits, reactor destroyed at tick 1095, 8 terrain_carved on dirt, 36 sim checksums, final_sim_checksum 9ed6b7f6… The PNG version is the closure-evidence "debrief artifact in BP3 note" requirement. |
| `m2_5_cause_chain_default.{md,png}` | `cause-chain` (default triggers) | M3B-002 walks 27 chains: terrain_carved breach + projectile_hits + reactor_damaged + objective_failed + mission_resolved. |
| `m2_5_cause_chain_reactor_damaged.{md,png}` | `cause-chain --event-type reactor_damaged` | Reactor damage at tick 488 traces back to `combat.projectile_hit` (chain depth 2, root reached). |
| `m2_5_cause_chain_mission_resolved.{md,png}` | `cause-chain --event-type mission_resolved` | M2.5 bundle's `mission_resolved` is correctly reported as "no parent chain" — tick-driven check, not event-driven. |
| `m2_5_view_mission_tail.{md,png}` | `view --filter mission --tail-len 16` | Mission category filter shows objective_started / objective_failed / mission_resolved. |
| `m2_5_view_combat_tail.{md,png}` | `view --filter combat --tail-len 16` | Last 16 combat events (projectile_hits + reactor_damaged sequence). |
| `m2_5_view_at_loss_tick.{md,png}` | `view --at-tick 1095 --tail-len 12` | Tick-scrubber clamps visibility to tick<=1095. |

### Synthetic actor_died fixture evidence (M3B-D02 ground truth)

| File | Subcommand | What it proves |
|---|---|---|
| `fixture_validate.txt` | `validate` | Hand-crafted fixture passes the strict validator. |
| `fixture_full_view.{md,png}` | `view --tail-len 32` | All 13 events of the fixture visible at end-of-run. |
| `fixture_actor_died_debrief.{md,png}` | `debrief` | Outcome=won (reason=all_red_actors_defeated), 1 actor_death, 1 projectile_hit / 100 dmg, fixture's final_sim_checksum. |
| `fixture_actor_died_cause_chain.{md,png}` | `cause-chain --event-type actor_died` | **The M3B-D02 evidence.** 6-link chain: actor_died → projectile_hit → projectile_spawned → weapon_fired → command_accepted → run_started. Chain depth 6, termination root_reached. |
| `fixture_mission_resolved_cause_chain.{md,png}` | `cause-chain --event-type mission_resolved` | 7-link chain in the canonical "death → mission outcome" shape: mission_resolved → actor_died → projectile_hit → … → run_started. This is what future engine code SHOULD emit (the M2.5 bundle's tick-driven mission_resolved is correct for that scenario, but the canonical death-driven chain shape only exists in this fixture today). |

The fixture is committed to the repo at
`game/crates/cf-tools-replay-viewer/tests/fixtures/m3b_actor_died_chain/`
with its own `notes.md` documenting the chain shape. 4 integration tests in
`game/crates/cf-tools-replay-viewer/tests/fixtures_integration.rs` validate
that the fixture loads + the cause chains match the documented shapes.

## Done-Criteria Coverage (M3B Roadmap)

- [x] **Replay viewer can scrub through events and show context.** Evidenced
  by `m2_5_view_*.{md,png}` files. Anchor tick clamps visibility, filter
  scopes the tail, pause/step state surfaced in the header. PNG rendering is
  the M3B-001 "Viewer capture in bundle" evidence target.
- [x] **Death recap renders the parent cause chain for `actor_died` and
  `mission_resolved` events.** The synthetic fixture renders BOTH chains
  end-to-end (`fixture_actor_died_cause_chain.png` 6-link chain;
  `fixture_mission_resolved_cause_chain.png` 7-link chain). The real M2.5
  bundle covers the alternative shape where mission_resolved is
  tick-driven (no parent — correctly reported as "root reached").
- [x] **DR-002 closure.** Status flipped OPEN → CLOSED-DIRECTION-WITH-EVIDENCE
  on 2026-05-09 at M3B; index + decision-tracker + research-readiness +
  research-log all updated in same pass.

## Self-Play Validation Matrix

| Action / scenario | Hands (script + step) | Eyes (file + visual confirm) | Ears (event row + observe field) | Verdict |
|---|---|---|---|---|
| `validate` (real bundle) | `m2_5_validate.txt` produced | `PASS bundle_dir=... events=7777 ticks=0..1989` | n/a (viewer reads, doesn't emit) | PASS |
| `validate` (fixture) | `fixture_validate.txt` produced | `PASS bundle_dir=... events=13 ticks=0..17` | n/a | PASS |
| `debrief` (real bundle, md+png) | `m2_5_debrief.{md,png}` | PNG renders all 6 sections; checksum 9ed6b7f6... visible | reads summary.final_sim_checksum + event_counts + mission_resolved payload | PASS |
| `debrief` (fixture, md+png) | `fixture_actor_died_debrief.{md,png}` | PNG renders won outcome + 1 actor death + checksum abcdef… | reads same fields against fixture | PASS |
| `cause-chain` (default, real) | `m2_5_cause_chain_default.{md,png}` | 27 chains rendered (terrain + projectile + reactor + objective + mission) | reads parent_event_id chains | PASS |
| `cause-chain --event-type actor_died` (fixture) | `fixture_actor_died_cause_chain.{md,png}` | PNG shows 6-link chain back to run_started (the canonical death-recap shape) | reads parent_event_id chain | PASS |
| `cause-chain --event-type mission_resolved` (fixture) | `fixture_mission_resolved_cause_chain.{md,png}` | PNG shows 7-link chain with actor_died as parent of mission_resolved | reads parent_event_id chain | PASS |
| `cause-chain --event-type reactor_damaged` (real bundle) | `m2_5_cause_chain_reactor_damaged.{md,png}` | Chain depth 2: reactor_damaged → projectile_hit | reads parent chain | PASS |
| `view --filter <cat>` | `m2_5_view_*.{md,png}` | Mission/combat category filters scope tail correctly | reads `events[*].category` | PASS |
| `view --at-tick 1095` | `m2_5_view_at_loss_tick.{md,png}` | Events with tick<=1095 visible only | reads `events[*].tick` | PASS |
| `cfctl replay scrub <bundle>` | (live demo) | Same output as direct viewer call | proxies to cf-tools-replay-viewer | PASS |

## Assumptions Tested

- A run bundle written by `cf-replay::write_run_bundle` can be loaded by
  `cf-tools-replay-viewer::Bundle::load` after the strict-validation
  tightening (no real BP2 bundle regressed).
- Synthetic fixture matches the strict validator (proves the contract is
  buildable by hand, not just by the recorder).
- `parent_event_id` chain in BP2 bundles + the synthetic fixture is
  connected end-to-end (every parent reference resolves, except the
  documented `external:` shape).
- PNG rendering is deterministic given the markdown content + font path
  + width + size — golden tests can compare PNGs offline.

## Good

- Every M3B subcommand (view / cause-chain / debrief / validate) emits both
  markdown and PNG, satisfying the roadmap's "Viewer capture in bundle" /
  "death/failure recap screenshot" / "debrief artifact in BP3 note"
  evidence targets without dragging in a heavy GUI dep.
- Strict validator now matches `prototype_run_check.py` byte-for-byte on
  the rules that were previously divergent (schema versions, dup event_ids,
  non-object payload, stale by_category / by_type maps, dropped_total
  underflow, `external:` prefix). Probe tests exercise each rule.
- `actor_died` cause chain has ground-truth evidence in committed fixture
  + 4 integration tests, not just a vibe assertion in notes.

## Bad

- The fixture's `mission_resolved → actor_died` chain shape is what
  future engine code SHOULD produce, but the M2.5 bundle's
  `mission_resolved` is tick-driven (no parent). Both shapes are
  legitimate — the viewer correctly distinguishes "root reached" from
  "parent missing from bundle" — but a reviewer should know the real-bundle
  evidence is a "no parent chain" rendering, not a deep walk.
- The PNG renderer uses Pillow; if Pillow isn't installed in CI, the
  `--png` flag fails. Self-play sweep checks for `markdown_to_png.py`
  before requiring PNG output, so the sweep degrades gracefully.

## Meh

- The viewer is markdown + PNG output only at M3B (anti-scope says "no
  polished replay browser"). A future BP can layer an egui/TUI on top of
  the same library API without refactoring.

## Evidence Links

- Source bundle (real): `prototype_runs/native/m2.5_2026-05-09T04-47-07Z_e66a7ad6/`
- Synthetic fixture: `game/crates/cf-tools-replay-viewer/tests/fixtures/m3b_actor_died_chain/`
- Viewer source: `game/crates/cf-tools-replay-viewer/`
- Markdown→PNG renderer: `game/tools/markdown_to_png.py`
- cfctl proxy: `game/crates/cfctl/src/main.rs:cmd_replay`
- Integration tests: `game/crates/cf-tools-replay-viewer/tests/fixtures_integration.rs`
- Implementation log: `docs/implementation-log/2026-05-09-m3b-replay-viewer-debrief.md`
- Audit-fix CHANGELOG entry: `CHANGELOG.md` § "M3B audit closure".

## Next Actions

- Land M4A (HUD readability + ACC-A floor) per the BP3 scope.
- Land M5 (equipment / chassis / damage grammar) per the BP3 scope.
- BP3 closing PR ships T-RELEASE engineering for double-click playability +
  retroactively re-tags v0.1.0-prealpha + v0.2.0-prealpha alongside
  v0.3.0-prealpha (the BP3 release).
- The M2.5 bundle's `mission_resolved` parent shape is correct for the
  current engine — no fix needed there. The fixture exists to prove the
  cause-chain machinery handles the death-driven shape too, for any
  future scenario that emits parent_event_id on mission_resolved.
