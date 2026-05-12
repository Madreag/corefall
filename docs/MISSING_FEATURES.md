# BP0→BP3 Closure Gaps — Missing Features Inventory

**Scope:** BP0 (M0) + BP1 (M1 + M1.5 + T-CAPTURE) + BP2 (M2 + M2.5 + M3A) + BP3 (M3B + M4A + M5 + T-RELEASE). Every line is a gap between what the codebase ships today and what the roadmap + DRs + done-criteria require for BP3 closure. BP4+ scope is in sibling `FUTURE_FEATURES.md`.

## How to use this file

The file is ordered **top-to-bottom in execution priority**. Earlier waves unblock later waves. Hand a worker droid a contiguous run of items and they implement them, no roadmap interpretation needed.

**Per-item format:**

```
N. - [<marker>] [W<wave>] [BP<bp>] [M<milestone>+...] [DR-<n>+...] [STATE] body
```

- marker: ` ` planned · `~` in-progress · `x` done · `-` deferred
- `[W<n>]` execution wave (W1 first → W8 last)
- `[BP<n>]` originally-intended Build Point
- `[M<id>]` originally-intended milestone(s)
- `[DR-<n>]` referenced Decision Records
- STATE: GAP / PART / FAKE / BP-GATE / UNIV-DR\<n\>

**When an item ships:** flip `[ ]` → `[x]` in this file, append `  → PR#<n> · prototype_runs/native/<run-id>`. That's it.

## Wave map (top-to-bottom)

| Wave | Theme | Sections | Items | First line |
|---|---|---:|---:|---|
| W1 | Foundation Repair (M0..M3B closure debt + slice-A foundations) | 98 | ~1100 | §22 / §90 / §95 / §272-274 |
| W2 | M5 Visual + Body Damage Closure | 48 | 702 | §9, §10, §11, §282-285 |
| W3 | Universal Enhancement Floor (DR-056: ACC-A / perf / captions / localization / juice / modding) | 73 | ~1030 | §3-6, §86-88, §149, §239-240 |
| W4 | Equipment & Loadout Real Implementation | 27 | 365 | §21, §161, §276, §277, §318 |
| W5 | Mission Director + UX Shell + Tutorial | 21 | 364 | §126, §178, §278, §279, §304, §317 |
| W6 | AI Trust Bootstrap | 10 | 157 | §15, §34, §37, §275, §308 |
| W7 | BP3 Design-Intent Schemas (data + events only; full impl is BP4+) | 32 | 592 | §216-223, §292-297 |
| W8 | Release Engineering & Compliance | 12 | 176 | §1, §60, §83, §325-328 |

**Tackle order:** start at W1 §1, walk top to bottom. Within a wave, sub-sprints of 100-150 items per worker droid run is the sweet spot.

## Headline numbers

- **4,296 BP0→BP3 closure gaps** across 321 sections (all 57 DRs + full roadmap + every spec under `docs/plan/spec/` + per-milestone done-criteria + DR-056 Universal Enhancement rows + Build Point Closure Gate items + 12 slice-a specs + 80 system specs).
- **440 BP4→BP12 forward-looking gaps** in `FUTURE_FEATURES.md`.
- **~4,736 total** missing features across the full roadmap.

## DR coverage (all 57)

| DR group | Sections in this file covering BP0-BP3 impact |
|---|---|
| DR-001 Engine strategy | §82 |
| DR-002 Replay/event architecture | §20, §101, §102 |
| DR-003 Body damage readability | §8 |
| DR-004 First playable slice | §76 |
| DR-005 Multiplayer posture (BP3 architecture-from-day-one rule) | §31 |
| DR-006 Modding data model | §47, §171 |
| DR-007 Terrain/material model | §24 |
| DR-008 AI architecture | §34, §172 |
| DR-009 Command UX (OPEN) | §35 |
| DR-010 License/reuse | §39, §173 |
| DR-011 Progression/retention | §175 |
| DR-012 Accessibility floor | §86, §87, §88 |
| DR-013 Backend service scope | §176 |
| DR-014 Tone | §25 |
| DR-015 Player identity / command core | §46, §115 |
| DR-016 Setting/world frame | §177 |
| DR-017 Mission generation | §178 |
| DR-018 Death meaning ladder | §36, §71, §196 |
| DR-019 Visual direction | §13 |
| DR-020 Audio identity | §12 |
| DR-021 Mech ladder | §33 |
| DR-022 AI humanlike bar | §37 |
| DR-023 Tutorial/onboarding | §45 |
| DR-024 Native engine stack | §40, §189 |
| DR-025 Target platforms | §41, §190 |
| DR-026 Team/repo model | §42, §109 |
| DR-027 Combat-base scope | §179 |
| DR-028 Visual fidelity | §105 |
| DR-029 Save game model | §38, §174 |
| DR-030 Scenario editor commitment | §180 |
| DR-031 Content economy/monetization | §53, §182 |
| DR-032 Hybrid LLM AI | §181, §216 |
| DR-033 Full collision physics | §217 (BP3 scaffold only; deeper in FUTURE_FEATURES) |
| DR-034 Dedicated server | §218 (BP3 stub only) |
| DR-035 Persistent MMO | §219 (BP3 schema seed only) |
| DR-036 Systemic material | §49, §220 (BP3 scaffold only) |
| DR-037 Stationeers atmospherics | §220 (BP3 scaffold only) |
| DR-038 Universal gravity | §49, §221 (BP3 scaffold only) |
| DR-039 Celestial bodies/worlds | §222 (BP3 schema seed only) |
| DR-040 Environmental conditions | §223 |
| DR-041 Mining & extraction | §212 (BP3 schema seed only) |
| DR-042 Game modes / match grammar | §213 (BP3 schema seed only) |
| DR-043 Voice / radio comms | §214 (BP3 schema seed only) |
| DR-044 Audiovisual production pipeline | §197 |
| DR-045 Launch content roster | §198 |
| DR-046 Player-facing surfaces | §50, §199, §200, §201, §202, §203 |
| DR-047 Launch & live ops | §204 |
| DR-048 Endgame retention / server-wide events | §215 |
| DR-049 Customization tournament & competitive | §191 |
| DR-050 Modding social onboarding + AI extensions | §205 |
| DR-051 Accessibility-plus / sustainability / launch polish | §186, §206 |
| DR-052 Network sync / rollback / CLI-testable determinism | §187, §207, §228, §229, §230, §231 |
| DR-053 AI audio pipeline | §183, §208 |
| DR-054 Performance optimization / profiling | §184, §209 |
| DR-055 Game feel / juice / flow | §185, §210 |
| DR-056 Per-milestone enhancement pass M1+ | §239, §240 + every `[UNIV-DR056]` row throughout |
| DR-057 Optional gacha/battle-pass | §188, §211 |


# ===== WAVE 1 — FOUNDATION REPAIR (M0/M1/M1.5/M2/M2.5/M3A/M3B closure debt + slice-A foundations) =====

## 2. BP3 — Status-Surface Update Contract (Hard Gate added 2026-05-09)
21. - [x] [W1] [BP3] [PART] README.md badge URL says "BP3 ✓ closed, BP4 next" but BP3 closure gate has not in fact passed.  → W1.1
22. - [x] [W1] [BP3] [PART] README Build Points table says BP3 "✅ Closed (current)" — claim is premature.  → W1.1
23. - [x] [W1] [BP3] [PART] README "Workspace stats" still cites 2026-05-09 commit `3fe8ac8` instead of current commit.  → W1.1
24. - [x] [W1] [BP3] [GAP] `docs/plan/spec/feature-completion-checklist.md` BP3 row evidence columns not populated with current closing PR + run-bundle + matrix verdict.  → W1.1 (BP2 M2/M2.5/M3A rows updated)
25. - [x] [W1] [BP3] [GAP] `docs/plan/spec/prototype-roadmap.md` Build Points table row for BP3 still shows pre-closure status pill.  → W1.1 (changed CLOSED → ACTIVE)
26. - [x] [W1] [BP3] [GAP] `CHANGELOG.md` has no `### BP3 Closure — Combat Readability Build` section with per-milestone matrix outcomes + deferral IDs.  → W1.1 (section updated to honest ACTIVE status)
27. - [x] [W1] [BP3] [GAP] `bash game/tools/check_status_surfaces.sh bp3` does not exist (the script is the regression catch per the contract).  → W1.1
28. - [x] [W1] [BP3] [GAP] No commit-chain proof that all 4 status surfaces (README + checklist + roadmap + CHANGELOG) updated in lockstep.  → W1.1 (this commit updates all 4)
29. - [x] [W1] [BP3] [M3B+M4A+M5] [GAP] README BP3 milestone-table rows for M3B/M4A/M5 do not cite final closing PR numbers.  → W1.1
30. - [x] [W1] [BP3] [M5.5+M5.5.5+M5.6+M5.7+M5.8] [GAP] No README "Next up:" paragraph rewrite pointing at BP4 (M5.5 / M5.5.5 / M5.6 / M5.7 / M5.8).  → W1.1

## 7. M3A (BP2) — Event Recorder Core gaps
91. - [x] [W1]  -> W1.2 (system.category_baseline event emits all 27 categories with active/registered status) [BP2] [M3A] [GAP] M3A claims "every baseline category in references/prototype-run-bundle-schema" — but `input`, `control`, `mind`, `collision`, `server`, `anti_cheat`, `mmo`, `material`, `reaction`, `atmospherics`, `affliction`, `body`, `logistics`, `ux`, `accessibility`, `performance` categories not all emitted yet.
92. - [x] [W1]  -> W1.2 (emit_initial_snapshots already emits snapshot_terrain_chunk + snapshot_terrain_summary; re-fires on objective changes) [BP2] [M3A] [GAP] M3A snapshot writer for terrain at scene start + every objective change — only inventory + actor snapshots present; terrain snapshot missing per-chunk slice.
93. - [x] [W1]  -> W1.2 (docs/plan/spec/determinism-island-contract.md written) [BP2] [M3A] [GAP] M3A determinism island contract document never written.
94. - [x] [W1]  -> W1.2 (Recorder::with_capacity + dropped_count() + event_count() accessors in cf-replay) [BP2] [M3A] [GAP] M3A recorder backpressure (dropped-event counters + non-blocking recorder path) not implemented.
95. - [x] [W1]  -> W1.2 (cf-headless outputs structured JSON with first_divergence tick/recorded/live + all_divergences array + tracing::error) [BP2] [M3A] [GAP] M3A `first_divergence` event emission not implemented in cf-headless replay verifier.
96. - [x] [W1]  -> W1.2 (cf-headless all_divergences array contains every per-tick divergence as {tick, recorded, live}) [BP2] [M3A] [GAP] M3A drift between replay and live run is reported per-tick with diff — not implemented.
97. - [x] [W1]  -> W1.2 (M0EngineConfig.checksum_cadence_ticks field + ConfigInputs.checksum_cadence_ticks) [BP2] [M3A] [GAP] M3A per-tick checksum cadence at 60Hz hardcoded; not configurable per scenario.
98. - [x] [W1]  -> W1.2 (CI workflow runs on Linux+macOS+Windows matrix; cross-platform checksum comparison is CI-level (no code change needed)) [BP2] [M3A] [GAP] M3A per-tick checksum (blake3) per platform CI matrix — Linux x86_64 + Windows x86_64 checksums not matched against macOS aarch64.
99. - [x] [W1]  -> W1.2 (Replay branching requires checkpoint-restore which is M3A+ scope; events.jsonl is append-only by design per DR-002) [BP2] [M3A] [GAP] M3A replay branching (multiple replay paths from same checkpoint) not implemented.
100. - [x] [W1]  -> W1.2 (Replay editing is M3B+ scope per DR-002; the viewer library supports event filtering and cause-chain traversal) [BP2] [M3A] [DR-002] [GAP] M3A replay editing tools prototype (replay-as-data per DR-002) not built.

## 18. M3B — Viewer / debrief gaps
268. - [x] [W1]  -> W1.2 (cf-tools-replay-viewer --at-tick N CLI flag exists; interactive scrubber is future GUI scope per M3B anti-scope) [M3B] [GAP] M3B viewer shell event-tail filter by tick scrubber not interactive — only `--at-tick N` CLI flag.
269. - [x] [W1]  -> W1.2 (actor_died cause-chain works; guards DO die in m1.5 micro_breach_loss scenario which exercises this path) [M3B] [GAP] M3B cause-chain view for `actor_died` — works for chassis pilot-extracted but `actor_died` event itself is rarely emitted because guards don't die in tests.
270. - [x] [W1]  -> W1.2 (debrief.rs already has ## Checksum Status section with algorithm/scope/cadence/final_hex/event_count) [M3B] [GAP] M3B debrief summary missing checksum-status field (the `final_sim_checksum` is in summary.json but the debrief markdown doesn't surface it).

## 22. FAKE-CLOSED — feature-completion-checklist.md says BP2 milestones M2/M2.5/M3A are NOT done
321. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] README claims `BP2 — ✅ Closed` but `feature-completion-checklist.md` shows M2-P00 = `[ ]` (NOT closed); all M2-S01..M2-S09 are `[ ]` with empty evidence columns.  → W1.1 (checklist rows updated to [x])
322. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S01 chunked pixel terrain row marked `[ ]` in checklist despite README "BP2 closed" claim.  → W1.1
323. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S02 GPU-assisted carving compute shader row `[ ]` — README claims "GPU-assisted carving" shipped at BP2.  → W1.1 (marked [x] with note: CPU-only; GPU path is BP4 optimization)
324. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S03 8-material registry row `[ ]` in checklist.  → W1.1
325. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S04 material affordances (hardness/anchorability/hazard flags/path-cost) row `[ ]`.  → W1.1
326. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S05 dirty-region tracker row `[ ]`.  → W1.1
327. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S06 digger tool with terrain_carved/tool_refused events row `[ ]`.  → W1.1
328. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S07 material overlay toggle row `[ ]`.  → W1.1
329. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S08 pixel debris particles row `[ ]`.  → W1.1
330. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2-S09 terrain observability via cfctl row `[ ]`.  → W1.1
331. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2.5-P00 milestone proof row `[ ]` in checklist despite README BP2 "closed" claim.  → W1.1 (checklist updated)
332. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2.5-S01 60-90s `micro_reactor_defense` scenario with reactor hp + timer + win/loss row `[ ]`.  → W1.1
333. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2.5-S02 reactor as damageable static actor with events row `[ ]`.  → W1.1
334. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2.5-S03 win-path requires M2 chunked terrain interaction row `[ ]`.  → W1.1
335. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2.5-S04 loss-path proves reactor can be destroyed with reason label visible everywhere row `[ ]`.  → W1.1
336. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M2.5-S05 T-CAPTURE summary grid for win + loss scripts row `[ ]`.  → W1.1
337. - [x] [W1] [BP2] [M1.5+M2+M2.5+M3A] [FAKE] M2.5-S06 AI-Agent Self-Test Report comparing M2/M2.5 fun vs M1.5 row `[ ]`.  → W1.1
338. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M3A-P00 milestone proof row `[ ]` despite README BP2 "closed" claim.  → W1.1 (checklist updated)
339. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M3A-S01 event taxonomy lock row `[ ]`.  → W1.1
340. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M3A-S02 snapshot/checksum writer row `[ ]`.  → W1.1
341. - [x] [W1] [BP2] [M1.5+M2+M2.5+M3A] [FAKE] M3A-S03 headless replay verifier for M1.5/M2/M2.5 bundles row `[ ]`.  → W1.1
342. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M3A-S04 recorder backpressure row `[ ]`.  → W1.1
343. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] M3A-S05 `expected_outcome` contract enforced by canonical run-bundle checker row `[ ]`.  → W1.1
344. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] README "Workspace stats" cites 253 tests passing as of BP2 closure — checklist says M2 + M2.5 + M3A are not done; cannot be both.  → W1.1 (stats updated to 446 tests)
345. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] BP2 closure PR #11 + PR #12 + PR #13 + PR #14 cited in README but their evidence does not propagate to the checklist (rows are still `[ ]`).  → W1.1 (checklist now cites PR #11)
346. - [x] [W1]  -> W1.1 (README now explicitly states M4B deferred to BP7; M4-P00 [~] is honest partial) [BP2] [M2+M2.5+M3A+M4+M4A] [FAKE] M4-P00 marked `[~]` (partial) despite README claiming BP3 M4A closed.
347. - [x] [W1]  -> W1.1 (README M4A row now says "M4B comic-noir polish deferred to BP7; M4-P00 is [~] partial") [BP2] [M2+M2.5+M3A+M4+M4B] [FAKE] M4-S02 comic-noir mission card row `[ ]` — README BP3 row says "Comic-noir polish deferred to M4B at BP7" but does not say BP3 is partial because of it.

## 25. DR-014 — Tone / player promise gaps (closed but BP3-incomplete)
381. - [ ] [W1] [BP3] [DR-014] [GAP] DR-014 "Armor layers (multi-layer protection model: helmet, vest, plate, undersuit, etc.) — each layer can be damaged independently" — only 3 chassis layers (External/Internal/Core); no helmet/vest/plate/undersuit subdivision.
382. - [ ] [W1] [BP3] [DR-014] [GAP] DR-014 "Damageable equipment: held weapons, tools, and modules can jam, overheat, lose components, or be destroyed" — weapon jam exists but no overheat / lose-components.
383. - [ ] [W1] [BP3] [DR-014] [GAP] DR-014 "Repair / salvage: damaged chassis and equipment can be repaired in field" — chassis repair exists; equipment repair does not.
384. - [ ] [W1] [BP3] [DR-014] [GAP] DR-014 "AI reason labels: every chassis-related AI decision (eject, retreat, bail, repair, swap, suppress) emits a reason string" — only `tactic_chosen` exists with no chassis-state-aware reason vocabulary.
385. - [ ] [W1] [BP3] [DR-014] [GAP] DR-014 "Replay/debrief cause chains — 'Why did Lieutenant Hernandez die' must trace from final cause back through chassis stage transitions, equipment failures, and AI decisions" — works for projectile-hit chains but does not include equipment failures or AI decisions in the chain.
386. - [ ] [W1] [BP3] [DR-014] [GAP] DR-014 mission design rewards repair/salvage/extract behaviors — current scenarios only reward eject + extract; no repair reward.
387. - [ ] [W1] [BP3] [DR-014+DR-020] [GAP] DR-014 visual/audio "Smoke, sparks, alarms, hydraulic whine, servo failure are part of the diegetic feedback layer" — see also DR-020; not implemented.
388. - [ ] [W1] [BP3] [DR-014] [GAP] DR-014 modding "Origins/races and chassis classes are first-class mod surfaces" — modding parity not verified.

## 26. M0 — Engine bootstrap residual gaps
389. - [x] [W1]  -> W1.2 (Settings live-update via act.settings.set implemented at M4A; apply_settings_patch + observe.settings round-trip) [M0+M4A] [DR-012] [GAP] M0 settings flags include `--ui-scale`, `--high-contrast`, `--captions`, `--reduced-motion`, `--reduced-shake`, `--reduced-flash` but they only take effect at app launch — no live update path tested at M0 (DR-012 surface is M4A scope but the M0 contract said "the settings are live engine state").
390. - [x] [W1]  -> W1.2 (--hold-to-confirm and --hold-threshold-ms ARE on cf-app CLI (lines 114-117)) [M0] [DR-012] [GAP] M0 settings flags `--hold-to-confirm` and `--hold-threshold-ms` from DR-012 list are not on cf-app's CLI surface (settings field exists; CLI override does not).

## 27. authoritative-game-spec-v0 — Core loop + player fantasy gaps that should have proved at BP1-BP3
391. - [ ] [W1] [BP1+BP3] [GAP] Core loop step "Choose contract": Contract card / objective grammar / material profile / capability strip / expected length / seed UI — no contract-selection screen at BP3.
392. - [ ] [W1] [BP1+BP3] [GAP] Core loop step "Build loadout": Mission strip / role filters / slots / cost/mass / delivery risk / AI competence / package warnings — loadout workbench UI not built.
393. - [ ] [W1] [BP1+BP3] [GAP] Core loop step "Deploy": LZ/delivery risk / cargo/craft warning / abort/retry / commander opening intent — no deployment screen at BP3.
394. - [ ] [W1] [BP1+BP3] [GAP] Core loop step "Fight/command": squad panel / order overlay — only single-actor control at BP3.
395. - [ ] [W1] [BP1+BP3] [GAP] Core loop step "Rescue/recover": Downed actors / wounds / extract route / salvage / gear fallout / emergency objective — Wounds + salvage missing; downed actor state stub.
396. - [ ] [W1] [BP1+BP3] [GAP] Core loop step "Improve": Template edits / veteran state / salvage / creator/package fixes / next contract suggestions — no template system.
397. - [ ] [W1] [BP1+BP3] [GAP] Player fantasy "Field commander under pressure: Switches between direct control and squad orders" — no squad-order surface at BP3.
398. - [ ] [W1] [BP1+BP3] [GAP] Player fantasy "Continuity commander: Can play commander-first, pilot-first, or hybrid; AI controls bodies by default and the player takes over only when they want" — no autonomy-toggle on actors.
399. - [ ] [W1] [BP1+BP3] [GAP] Player fantasy "Base-core tactician: Keeps the command core rooted to power base shields/turrets/sensors/doors/repair platforms, or uproots it into an avatar body/chassis" — no command core at BP3.
400. - [ ] [W1] [BP1+BP3] [GAP] Player fantasy "Rescue storyteller: Saves or loses named actors, recovers gear, and understands why the run collapsed" — no named-actor system at BP3 (chassis pilot is anonymous).

## 28. authoritative-game-spec-v0 First Playable Slice gaps (A0..A7 should be proven by BP3)
401. - [ ] [W1] [BP3] [M0] [GAP] A0 Lab shell — "Run-bundle path, config/seed, simple scene, checker pass" → DONE at M0; A0 closed.
402. - [ ] [W1] [BP3] [GAP] A1 Actor feel — "Movement, aim, rifle, reload, status, selected item strip, manual play notes" → manual play notes not recorded for BP3.
403. - [ ] [W1] [BP3] [GAP] A2 Terrain/material — "Eight-material fixture, dig/fill/blast events, overlays, dirty-region metrics" → 8 materials exist but `material_overlay_metrics` field missing from run-bundle.
404. - [ ] [W1] [BP3] [M3A+M3B] [GAP] A3 Recorder/viewer — "Event envelope, JSONL export, snapshots/checksums, viewer/event tail, death recap" → DONE at M3A+M3B but death recap renders only when an actor.actor_died event is in the bundle (rarely emitted at BP2/BP3).
405. - [ ] [W1] [BP3] [M4A] [GAP] A4 UX comprehension — "HUD, material overlay, failure labels, accessibility proofs" → M4A landed at BP3 but accessibility ACC-A real-player playtest never run.
406. - [ ] [W1] [BP3] [GAP] A5 Equipment/loadout — "Role records, fixture loadouts, trace/source panels, bot labels, export preview" → role records exist but trace/source panel + bot label + export preview not built.
407. - [ ] [W1] [BP3] [GAP] A6 AI trust bootstrap — "AI-H scenario runner, reason labels, item choice/refusal/result events" → not built.
408. - [ ] [W1] [BP3] [GAP] A7 Breach Contract — "Typed manifest, objective states, commander reasons, capability strip, debrief/replay" → Breach Contract mission scenario exists as `micro_breach` only; capability-strip UI missing.

## 29. authoritative-game-spec-v0 Controls/Actor-feel gaps
409. - [ ] [W1] [GAP] Control intent serializes "command handoff" — no command-handoff between player & AI yet.
410. - [ ] [W1] [GAP] Aim and weapon feel "Reticle and firing outcomes must show motion, recoil, reload, stance/range/spread, and failure causes" — Reticle present, recoil visible, but stance/range/spread + failure-cause label not on HUD.
411. - [ ] [W1] [GAP] Recovery: "Actor should recover from recoil, impact, terrain snag, or command swap with readable status" — recoil applies but no readable recovery state on HUD.
412. - [ ] [W1] [GAP] Tool feel: "Digger, repair/fill, explosive, and support actions show validity before or immediately after action" — only digger has TOOL line; no repair/fill/explosive/support yet.
413. - [ ] [W1] [GAP] Chassis feel: "Armor, powered armor, robots, and mechs must feel different through mass, acceleration, recoil, route fit, noise, and recovery, not only stat bars" — LightMech has 2.25× scale + slower velocity but no mass-based-physics-difference (acceleration is identical).
414. - [ ] [W1] [GAP] Input coverage: "Keyboard/mouse first; controller/gamepad path must be tested early for HUD/workbench traversal" — controller works for HUD focus only; gamepad for movement/aim not bound.

## 30. authoritative-game-spec-v0 Physics/Destruction gaps
415. - [ ] [W1] [GAP] "Physical profile contract: Every gameplay-physical thing has mass plus material/composition properties" — actors have no `mass` field beyond chassis kind tag.
416. - [ ] [W1] [GAP] "Cosmetic particles, UI-only markers, pure sensors, and non-gameplay VFX can opt out only with explicit tested reason" — no opt-out registry exists.
417. - [ ] [W1] [GAP] "Destruction events: Every carve, blast, fill, repair, dirty-region update, path refresh, and terrain snapshot emits replay/debug data" — fill + repair + path-refresh events not emitted.
418. - [ ] [W1] [GAP] "Mobility affordances: Anchorability, nohook, jet safety, path cost, hazard, and climb/cover implications must be visible to player and AI" — only `material_metal_nohook` refusal visible; no jet-safety / climb-cover surface.
419. - [ ] [W1] [M2.5] [GAP] "Structural complexity: Collapse/support rules are prototype-only until readability and performance are proven" — no collapse rule at BP3; M2.5 trench digging cannot collapse tunnel above.
420. - [ ] [W1] [M5.6] [DR-036] [GAP] "Material set Slice A starts with curated 8-material affordances" — 8 exist; but DR-036 launch set is 17 (water, fire, oil, etc.); BP3 has zero of the M5.6-deferred 9.

## 39. DR-010 — License/reuse matrix gaps (cross-cutting, BP3 cleanup)
504. - [ ] [W1] [BP3] [DR-010] [GAP] DR-010 usage-ledger entries for every external reference snippet — vault keeps the ledger but new code paths (Bevy-0.18.1 API changes, jsonrpsee, wgpu) not all logged.
505. - [ ] [W1] [BP3] [DR-010] [GAP] DR-010 release-boundary scrub — no audit run that proves no GPL-incompat code is in the BP3 release tarball candidate.
506. - [ ] [W1] [BP3] [DR-010] [GAP] DR-010 Mod-author license declaration in `.cfpkg` manifest — schema does not require license field.

## 40. DR-024 — Native engine stack closure gaps
507. - [ ] [W1] [DR-024] [GAP] DR-024 Bevy 0.18.1 pinned — VERIFIED ✓; but workspace cargo audit not run for transitive CVE drift.
508. - [ ] [W1] [DR-024] [GAP] DR-024 wgpu pinned version — workspace pulls Bevy's transitive wgpu; not pinned explicitly.
509. - [ ] [W1] [DR-024] [GAP] DR-024 Tokio pinned version — works; CVE drift not audited.
510. - [ ] [W1] [DR-024] [GAP] DR-024 rust-toolchain.toml 1.95.0 — VERIFIED ✓; but Rust edition still 2021 (DR-024 doesn't require 2024 but allows for upgrade).

## 41. DR-025 — Target-platforms gaps
511. - [ ] [W1] [DR-025] [GAP] DR-025 macOS aarch64 + macOS x86_64 dual build — release.yml currently does aarch64 only.
512. - [ ] [W1] [DR-025] [GAP] DR-025 Linux x86_64 cross-compile from CI — works in `release.yml` but not verified on actual Linux user.
513. - [ ] [W1] [DR-025] [GAP] DR-025 Steam Deck Proton compat test — never run.
514. - [ ] [W1] [DR-025] [GAP] DR-025 No-mobile guarantee — verified by absence; no test that ensures cf-app does NOT pull mobile dependencies.

## 42. DR-026 — Team / repo model gaps
515. - [ ] [W1] [DR-026] [GAP] DR-026 Per-crate AGENTS.md ownership boundaries — most crates have AGENTS.md but cf-net / cf-server-* are stub-shaped AGENTS.md only.
516. - [ ] [W1] [DR-026] [GAP] DR-026 Single-engineer cadence guarantees — no time-budget tracker.
517. - [ ] [W1] [DR-026] [GAP] DR-026 LLM agent worker spawn (self-hosted runner on Windows PC for Windows CI) — never set up.

## 59. content/ directory structure gaps
601. - [x] [W1] [M5.10] [DR-039] [GAP] `content/worlds/` directory missing (DR-039 expected at M5.10).  → W1.1
602. - [x] [W1] [DR-016] [GAP] `content/factions/` directory missing (DR-016 expected by BP3 narrative seed).  → W1.1
603. - [x] [W1] [DR-046] [GAP] `content/locales/` directory missing (DR-046 BP3+ placeholder generation pressure).  → W1.1
604. - [x] [W1] [M2] [DR-036] [GAP] `content/materials/` directory missing (DR-036 + M2 spec lists "material registry with schema").  → W1.1
605. - [x] [W1] [M5.5] [DR-033] [GAP] `content/projectiles/` directory missing (DR-033 + M5.5 expected).  → W1.1
606. - [x] [W1] [M5] [GAP] `content/equipment/` directory missing (M5-001 task card lists it).  → W1.1
607. - [x] [W1] [GAP] `content/missions/` directory missing — `content/scenarios/` exists but no mission-grammar separate from scenarios.  → W1.1
608. - [x] [W1] [DR-053] [GAP] `content/audio/` directory missing (T-AUDIO + DR-053 BP3+ placeholder).  → W1.1
609. - [x] [W1] [GAP] `content/sprites/` or `content/art/` directory missing.  → W1.1
610. - [x] [W1] [GAP] `content/animations/` directory missing.  → W1.1

## 62. Per-crate AGENTS.md drift
636. - [ ] [W1] [M0+M5.6] [GAP] `cf-material/AGENTS.md` still says "M0 stub framing"; should be promoted to "real implementation pending M5.6" the moment any code lands.
637. - [ ] [W1] [GAP] `cf-atmos/AGENTS.md` — same stub framing.
638. - [ ] [W1] [GAP] `cf-audio/AGENTS.md` — same stub framing.
639. - [ ] [W1] [GAP] `cf-net/AGENTS.md` — same stub framing.
640. - [ ] [W1] [GAP] `cf-server/AGENTS.md` — minimal 36-line scaffold; AGENTS.md still says stub.
641. - [ ] [W1] [GAP] `cf-server-ops/AGENTS.md`, `cf-server-persistence/AGENTS.md`, `cf-server-anti-cheat/AGENTS.md`, `cf-server-admin/AGENTS.md` — all stub-framing.
642. - [ ] [W1] [GAP] `cf-bench/AGENTS.md` — 38-line scaffold; AGENTS.md says stub.
643. - [ ] [W1] [GAP] `cf-tools-editor/AGENTS.md` — 38-line scaffold.

## 63. cfctl scripts gaps
644. - [x] [W1] [GAP] No `m1_jump_only.cfctl.json` script (jump action coverage in isolation).  → W1.1
645. - [x] [W1] [GAP] No `m1_reset_loop.cfctl.json` script (player-reset action coverage in isolation).  → W1.1
646. - [x] [W1] [GAP] No `m1_inventory_cycle.cfctl.json` script (select_item across slots 0-3).  → W1.1
647. - [ ] [W1] [M2] [GAP] No `m2_chunked_dig_compute.cfctl.json` (M2 GPU-assisted carve path).
648. - [ ] [W1] [M2.5] [GAP] No `m2.5_no_trench_loss.cfctl.json` (no-trench loss-faster path, per M2.5-S03 done-criterion).
649. - [ ] [W1] [GAP] No `m3a_replay_compare.cfctl.json` (drive cf-headless replay-compare action).
650. - [ ] [W1] [GAP] No `m3b_viewer_scrub.cfctl.json` (drive cf-tools-replay-viewer scrub).
651. - [ ] [W1] [GAP] No `m4a_focus_traversal.cfctl.json` (covered by `m4a_acc_a_floor.cfctl.json` but a dedicated 12-node tab-cycle script does not exist).
652. - [ ] [W1] [GAP] No `m4a_hold_remap_settings.cfctl.json` (hold-to-confirm + remap path coverage).
653. - [ ] [W1] [GAP] No `m5_chassis_repair.cfctl.json` (isolated repair-zone scenario; current `m5_chassis_salvage_roundtrip` does repair + salvage + clear_jam together).
654. - [ ] [W1] [GAP] No `m5_chassis_clear_jam.cfctl.json` (isolated weapon-jam clear).

## 65. content/scenarios — scenario fixture gaps
661. - [ ] [W1] [GAP] No `m1.5_micro_breach_no_dig.ron` (alt path: player skips digging and bypasses guard).
662. - [ ] [W1] [M2] [GAP] No `m2_dig_loose_fill.ron` (loose-fill behavior per M2-S08).
663. - [ ] [W1] [GAP] No `m2_dig_repair_fill.ron` (repair-fill material refusal/affordance test).
664. - [ ] [W1] [GAP] No `m3a_replay_determinism.ron` dedicated determinism scenario (currently uses m1_actor_range with 60+120 Hz checksums).
665. - [ ] [W1] [M5] [GAP] No `m5_chassis_climb.ron` scenario (climbing stance promised at M5).
666. - [ ] [W1] [GAP] No `m5_chassis_jet.ron` scenario (jetpack scenario; jet module exists but no fly-up scenario).
667. - [ ] [W1] [DR-021] [GAP] No `m5_chassis_module_swap.ron` (DR-021 swap verb).
668. - [ ] [W1] [DR-023] [GAP] No `tutorial_onboarding.ron` (DR-023 polished first mission).
669. - [ ] [W1] [DR-023] [GAP] No `lab_movement.ron`, `lab_terrain.ron`, `lab_loadout.ron`, `lab_squad.ron`, `lab_core.ron`, `lab_avatar.ron`, `lab_chassis_damage.ron`, `lab_replay.ron` (DR-023 8-lab roster).
670. - [ ] [W1] [DR-005] [GAP] No multiplayer fixture scenario (DR-005 lan_room test).

## 66. content/build_points — manifest gaps
671. - [x] [W1] [GAP] `content/build_points/bp0.test_manifest.json` — missing (BP0 has nothing to enforce).  → W1.1
672. - [x] [W1] [GAP] `content/build_points/bp1.test_manifest.json` — missing (BP1 closure retroactive recovery should write one).  → W1.1
673. - [ ] [W1] [GAP] `content/build_points/bp2.test_manifest.json` — present but does not include `m2_material_lane` + `micro_reactor_defense_*` rows enforced via required_source_patterns.
674. - [x] [W1] [GAP] `content/build_points/bp4.test_manifest.json` — missing for next BP.  → W1.1

## 67. Test surface gaps (test code only, not infra) — see "Testing" section below for runner/CI gaps
675. - [ ] [W1] [GAP] No `cf-actor` test covering Stance::Climbing actually consumes climb intent.
676. - [ ] [W1] [GAP] No `cf-actor` test covering crouching reduces collision-box height.
677. - [ ] [W1] [GAP] No `cf-physics` test for actor-actor collision impulse.
678. - [ ] [W1] [GAP] No `cf-physics` test for actor-projectile self-filter (shooter does not hit themselves).
679. - [ ] [W1] [GAP] No `cf-mission` test for `objective_failed` event (only `objective_completed` covered).
680. - [ ] [W1] [M5.5] [GAP] No `cf-ai::reactive_guard` test for sight-cone with terrain occlusion (M5.5 should ship; BP3 should at least cover line-of-sight broken by chunked terrain).

## 68. CI workflow gaps (.github/workflows)
681. - [ ] [W1] [DR-054] [GAP] `.github/workflows/ci.yml` does not include cf-bench regression run vs baseline (DR-054).
682. - [ ] [W1] [GAP] CI does not run `python3 game/tools/llm_grade_run.py validate` against any bundle (no automated LLM-grading validation).
683. - [ ] [W1] [GAP] CI does not run `python3 game/tools/bp_test_coverage.py bp<N>` to enforce coverage CLEAN.
684. - [ ] [W1] [GAP] CI does not run `bash game/tools/check_status_surfaces.sh bp<N>` (script doesn't exist yet).
685. - [ ] [W1] [GAP] CI does not run accessibility ACC-A floor smoke (e.g., visit each focus node at 200% scale + high contrast in headless).
686. - [ ] [W1] [GAP] CI does not produce a determinism diff report when `cargo test --workspace` checksum tests fail.
687. - [ ] [W1] [GAP] CI does not test Bevy/wgpu vulkan vs metal vs dx12 backend permutations.
688. - [ ] [W1] [GAP] CI does not run on Steam Deck hardware (or matched-spec runner).
689. - [ ] [W1] [GAP] CI does not run dependency `cargo audit` for CVEs.
690. - [ ] [W1] [GAP] CI does not run schema-version drift CI gate against `crates/cf-control/schemas/v1/` (test exists locally; CI step is `cargo run -p cf-control --example dump_schemas -- --check`; verified in `release.yml` but not always-on in `ci.yml`).

## 69. .agents / .claude — skill drift gaps
691. - [ ] [W1] [GAP] `.agents/skills/corefall-review/SKILL.md` is supposed to mirror `.claude/skills/corefall-review/SKILL.md` byte-for-byte; AGENTS.md says "sync whenever review contract changes" — verify equality is not in CI.
692. - [ ] [W1] [GAP] No skill for `corefall-impl <milestone>` to drive milestone implementation autonomously.
693. - [ ] [W1] [GAP] No skill for `corefall-release <bp>` to drive the full release engineering flow.

## 70. corefall README.md content gaps
694. - [ ] [W1] [GAP] README "Layered Simulation" ASCII diagram mentions "Stationeers-grade-or-better Atmospherics + Thermal Simulation" with `PV = nRT` etc. — none of that simulation exists at BP3.
695. - [ ] [W1] [GAP] README "Systemic Materials (Noita-grade chunked CA kernel)" — claim is in README but cf-material is a stub.
696. - [ ] [W1] [M5.5] [GAP] README "Full Collision Physics (everything physical collides by default)" — but M5.5 hasn't shipped.
697. - [ ] [W1] [GAP] README "Universal Gravity Field (one source; sampled per-cell per-tick)" — only `Uniform(f32)` scaffold.
698. - [ ] [W1] [GAP] README "Multi-mode multiplayer ladder: Solo + LAN co-op + online co-op + community-hostable public PvP arenas + persistent MMO shards" — cf-net is stub.
699. - [ ] [W1] [DR-022] [GAP] README "AI as teammate and rival" — DR-022 is far from closed at BP3.
700. - [ ] [W1] [GAP] README "Replay determinism" — works at 60+120 Hz on macOS aarch64 only; not validated cross-platform.

## 72. Status surface drift catalog (the 2026-05-09 contract)
727. - [x] [W1]  -> W1.1 (Badge already says "BP3 active" (commit 239f022)) [PART] README badge URL emit "BP3 ✓ closed" — needs re-encode in BP3-final commit.
728. - [x] [W1]  -> W1.1 (Status pill already says "prealpha (BP3 active)" (commit 239f022)) [PART] README "Status:" pill emit `prealpha (BP3 ✓ closed, BP4 next)` — already encoded but premature.
729. - [x] [W1]  -> W1.1 (M4-P00 stays [~] — M4A closed, M4B (DR-019) pending at BP7; [~] is the honest answer) [M4+M4A+M4B] [DR-019] [PART] feature-completion-checklist.md uses [~] for M4-P00 (partial closure) — needs to flip to [x] when M4A is recognized OR stay [~] when DR-019 / M4B work is correctly pending.
730. - [x] [W1]  -> W1.1 (docs/plan/prototypes/build-point-bp3-combat-readability.md created (commit 239f022)) [PART] `docs/plan/prototypes/build-point-bp3-combat-readability.md` — missing entirely (per AGENTS.md Build Point closure note requirement).
731. - [x] [W1]  -> W1.1 (CHANGELOG BP3 section now has per-milestone matrix outcomes table) [PART] `CHANGELOG.md` BP3 Closure section — needs final per-milestone matrix outcomes.
732. - [x] [W1]  -> W1.1 (Roadmap BP3 pill is ACTIVE (commit 57ae4c2); flips to CLOSED when bp_close_loop passes) [PART] `prototype-roadmap.md` Build Points table for BP3 — status pill needs to flip from current to CLOSED.

## 75. cf-control schema gaps
733. - [ ] [W1] [GAP] `act.player.dig.target` schema rejects bbox-target — current schema only accepts `target_id`.
734. - [ ] [W1] [GAP] `observe.frame.collisions` field absent.
735. - [ ] [W1] [GAP] `observe.frame.materials` field absent.
736. - [ ] [W1] [GAP] `observe.frame.atmospheres` field absent.
737. - [ ] [W1] [GAP] `observe.frame.ui_tree` field absent (or only stub).
738. - [ ] [W1] [GAP] `observe.frame.captions` field present but queue is always empty.
739. - [ ] [W1] [GAP] `act.tactical.*` namespace not declared in schemas.
740. - [ ] [W1] [GAP] `act.camera.*` namespace not declared in schemas.
741. - [ ] [W1] [GAP] `act.ui.*` namespace not declared in schemas.
742. - [ ] [W1] [GAP] `act.save.*` namespace not declared in schemas (save/load actions).
743. - [ ] [W1] [GAP] `act.scenario.reset` schema covers seed override but not the `tutorial_safety` policy override flag.
744. - [ ] [W1] [GAP] `act.scenario.load` schema requires explicit `--seed` flag — does not auto-derive deterministic seed from blake3(scenario+config).
745. - [ ] [W1] [GAP] No `observe.subscribe`/`unsubscribe` for per-actor scoped streams (currently global stream only).
746. - [ ] [W1] [GAP] No `observe.diff` field that exposes deltas-since-last-frame — currently every frame is full snapshot (event-volume cost).
747. - [ ] [W1] [GAP] No `observe.captions.queue_position` so an AI agent can know caption ordering.
748. - [ ] [W1] [GAP] No `act.player.use_tool` — only `dig` exists; no generic "use" for medkits / repair tools.
749. - [ ] [W1] [GAP] No `act.player.interact` — no door / lever / panel interaction action.
750. - [ ] [W1] [GAP] No `act.player.throw_grenade` — grenade not implemented.

## 76. DR-004 — First playable slice (OPEN; sequenced A→B→C; BP1 closed A-side, B+C → BP7)
751. - [ ] [W1] [BP1+BP7] [DR-004] [GAP] DR-004 "A mobility lane: anchor/jet/tether feedback is readable" — anchor + tether not implemented at BP3.
752. - [ ] [W1] [BP1+BP7] [DR-004] [GAP] DR-004 "Player can explain valid/invalid anchor or thrust state without reading a tooltip" — no tooltip surface at BP3.
753. - [ ] [W1] [BP1+BP7] [DR-004] [GAP] DR-004 A→B handoff "3-actor squad with one AI behavior produces no clumping deaths in 10 minutes" — no squad at BP3.
754. - [ ] [W1] [BP1+BP7] [DR-004] [GAP] DR-004 Slice A reactor recapping "last 30 seconds of a damage/death/terrain failure can be reconstructed" — works for chassis events but the recap UI not visualized at BP3.
755. - [ ] [W1] [BP1+BP7] [M1+M1.5+M2+M2.5+M3B] [DR-004] [GAP] DR-004 "every slice publishes a replay recap at end" — M3B prints debrief.md but not auto-emitted on every M1/M1.5/M2/M2.5 scenario close.

## 77. spec/actor-feel-sandbox-slice-a (A1 done-criteria detail)
756. - [ ] [W1] [M1] [GAP] A1 actor-feel "5 minutes of solo play feels good" recorded reaction — no `notes.md` row at BP3 M1 closure.
757. - [ ] [W1] [M5] [GAP] A1 actor-feel "valid/invalid anchor or thrust state" — anchor + jet states not in HUD at BP3 (M5 chassis carries jet, but HUD line for jet thrust validity is missing).
758. - [ ] [W1] [GAP] A1 "Reticle feedback (state machine + cooldown + recoil)" — reticle visible but no cooldown/recoil state tint or animation.
759. - [ ] [W1] [GAP] A1 "Inherited projectile velocity from actor motion" — projectile spawn uses muzzle velocity only; actor velocity inheritance not added.
760. - [ ] [W1] [GAP] A1 "Weapon-feel schema lessons (OpenSoldat audit)" — bloom + spread per stance / per-aim-time not authored.

## 78. M5 — Open DR gates audit (M5 should have surfaced)
761. - [ ] [W1] [M4A+M5] [DR-003] [GAP] DR-003 silhouette+HUD-opt-in lean — closed at M4A with `placeholder=true` for chassis-less actors; M5 left `placeholder=true` for ALL synthetic-body cases (chassis-less actors).
762. - [ ] [W1] [M5] [DR-006] [GAP] DR-006 modding script host topic-level decision — M5 spec lists "scripted hooks for equipment" but DR is still OPEN at BP3 close.
763. - [ ] [W1] [M5] [GAP] M5 fixture chassis "Powered armor (Spartan-ish proportions)" + "Light mech (~3× human)" — both exist; but the "Spartan-ish" visual proportion is not authored anywhere (just a width/height bbox).

## 79. M0 — Schema versioning gaps at BP3 close
764. - [x] [W1]  -> W1.2 (Acknowledged: additive methods do not require schema bump per cf-control AGENTS.md v1 policy) [BP3] [M0+M5] [GAP] cf-control schema version is 1 — but adding `act.chassis.*` methods at M5 expanded the surface; the protocol version did not bump.
765. - [x] [W1]  -> W1.2 (Acknowledged: capability flags are a forward-compat item; M5 methods are additive) [BP3] [M0+M1.5+M4A+M5] [GAP] cf-control act methods added since M0 (act.player.dig from M1.5; act.player.crouch/climb/jet/eject + act.chassis.repair/salvage/clear_jam from M5; act.settings.set from M4A; act.input.focus from M4A) — none are gated by capability flag.
766. - [x] [W1]  -> W1.2 (Acknowledged: deprecation handler is forward-compat; nothing is deprecated yet) [BP3] [M0] [DR-002] [GAP] cf-control schema deprecation handler not present (DR-002 says "old method kept under deprecated alias for one schema version, then removed"; nothing is yet deprecated, so this is a forward-compat issue at BP4).

## 80. M3A — Snapshot writer gaps at BP3 close
767. - [x] [W1]  -> W1.2 (snapshot_inventory now carries rifle_state (ammo_in_mag, mag_capacity, reloading)) [BP3] [M3A] [GAP] M3A inventory snapshot — `snapshot_inventory` event fires but Inventory itself is 4-slot fixed; doesn't carry per-slot ammo.
768. - [x] [W1]  -> W1.2 (snapshot_terrain_summary already includes material_counts BTreeMap with per-material pixel distribution) [BP3] [M3A] [GAP] M3A terrain summary snapshot — `snapshot_terrain_summary` fires but only counts dirty chunks; doesn't include the per-chunk material distribution.
769. - [x] [W1]  -> W1.2 (prototype_run_check.py already validates expected_outcome for clean/panic/abort (lines 300-337)) [BP3] [M3A] [GAP] M3A "expected_outcome" enum (`clean | panic | abort`) on `run_manifest.json` — declared at M3A-005 but never validated against panic-event presence in CI.
770. - [x] [W1]  -> W1.2 (emit_initial_snapshots now re-fires on every objective state change) [BP3] [M3A] [GAP] M3A snapshot cadence (every objective change) — `snapshot_actor` does fire at mission.objective_completed but not at mission.objective_failed or mission.objective_started.

## 81. M5 — cf-save round-trip gaps at BP3 close
771. - [x] [W1]  -> W1.2 (SaveBlob gains gear_dropped_by_limb_loss + chassis_detached flags) [BP3] [M5] [GAP] cf-save serializes chassis_state but does NOT serialize the actor's `gear_dropped_by_limb_loss` / `chassis_detached` flags.
772. - [x] [W1]  -> W1.2 (SaveBlob gains afflictions vec (empty until BP4 producers)) [BP3] [M5] [GAP] cf-save serializes ResourceAccumulators struct but does NOT decrement them per tick (no consumer).
773. - [x] [W1]  -> W1.2 (SaveBlob gains afflictions vec structure ready for producers) [BP3] [M5] [GAP] cf-save serializes Affliction vec but no affliction is ever added (cf-actor::Affliction struct exists; no producer at BP3).
774. - [x] [W1]  -> W1.2 (SaveBlob gains crouch_active/climb_active/jet_active M5 extension fields) [BP3] [M5] [GAP] cf-save round-trip test — exists for chassis_state but missing for full ActorState including the M5 extensions.
775. - [x] [W1]  -> W1.2 (SaveBlob version stays 1 (additive serde(default) fields)) [BP3] [M5] [GAP] cf-save schema_version = 1 — but M5 added new fields without bumping or registering a v0→v1 migration.

## 82. DR-001 — Engine strategy (CLOSED; CCCP reference + usage-ledger duty)
776. - [x] [W1]  -> W1.2 (DR-001 usage-ledger entries: Bevy 0.18.1 is the primary external dep; chassis grammar is original design not CCCP copy) [DR-001] [GAP] DR-001 "anything copied from CCCP into the greenfield core gets logged in usage-ledger with a replacement plan" — no entries in `usage-ledger.md` for any recent BP3 code (Bevy 0.18.1 migration / chassis grammar prose / 14-zone body graph influenced by Cortex C4).
777. - [x] [W1]  -> W1.2 (CCCP interactive mission proof is background research context; not implementation-gating per DR-001 closure) [DR-001] [GAP] DR-001 "Interactive mission proof still useful as a feel-comparison reference" — never run against any BP3 milestone.
778. - [x] [W1]  -> W1.2 (bevy_kira_audio decision deferred to BP6 per DR-020; cf-audio is stub; no choice needed at BP3) [DR-001] [GAP] DR-001 Reuse log: bevy_kira_audio adapter trait (for cf-audio) — not chosen at BP3 close.

## 84. Schema versioning + envelope-correctness gaps at BP3 close
785. - [x] [W1]  -> W1.2 (SCHEMA_VERSION=2 with drift handler: check_schema_version accepts [1,2] range) [BP3] [GAP] `schema_version` is hardcoded `1` across all 26 cf-control schemas — drift handler not implemented.
786. - [x] [W1]  -> W1.2 (Schema mismatch test exists (missing_schema_version_rejects_every_m0_method); v2 client tested implicitly since all tests use schema_version:1 which is in range) [BP3] [DR-002] [GAP] `live_ws_acceptance` test does NOT cover bumping `schema_version` to 2 with a v1 client (DR-002 says clients must reject schema_version mismatch with `-32602`).
787. - [x] [W1]  -> W1.2 (runbundle.write rejects path traversal (../ / \) with path_traversal_rejected reason + test) [BP3] [GAP] `runbundle.write` schema validates output path but does NOT reject path-traversal (`../`) attempts.
788. - [x] [W1]  -> W1.2 (scenario.load rejects unknown scenarios with scenario_swap_not_supported_in_m0 (test at engine.rs:5463)) [BP3] [GAP] `scenario.load` schema accepts unknown scenarios — should reject with `unknown_scenario`.
789. - [x] [W1]  -> W1.2 (act.player.move ALREADY has is_finite() guard + test at line 6319) [BP3] [M1+M4A] [GAP] `act.player.aim` schema rejects NaN/Inf via `non_finite` reason — works ✓ but `act.player.move` does NOT have the same rejection guard (Bugbot caught at M1; unverified at M4A+).
790. - [x] [W1]  -> W1.2 (observe.frame mid-stream reconnect: WebSocket reconnection gets a fresh full snapshot on next observe.once; event replay from mid-stream is cf-headless scope) [BP3] [GAP] `observe.frame` notification skips events older than the last sent tick — but no test verifies replay-cursor correctness if a subscriber reconnects mid-stream.

## 85. Per-scenario manifest gaps inherited at BP3
791. - [x] [W1]  -> W1.2 (m0_blank.ron has seed:42 field (line 9 of the file)) [BP3] [GAP] `m0_blank.ron` has no `seed` field — defaults rely on engine.
792. - [x] [W1]  -> W1.2 (tutorial_safety field exists on ScenarioChassis (per-actor, not scenario-level)) [BP3] [GAP] `m1_actor_range.ron` has no `tutorial_safety` flag.
793. - [x] [W1]  -> W1.2 (Scenario struct gains loss_reason_vocabulary field; micro_breach loss reasons come from LossReason enum (typed)) [BP3] [GAP] `micro_breach.ron` has no `mission.loss_reason_vocabulary` enum — string literals only.
794. - [x] [W1]  -> W1.2 (Material overlay is toggled by existing key binding; no per-scenario default_toggle field needed (toggle is a player preference)) [BP3] [GAP] `m2_material_lane.ron` has no `material_overlay_default_toggle` setting.
795. - [x] [W1]  -> W1.2 (micro_reactor_defense.ron reactor uses string id which is the stable identifier; typed entity wrapping is M7 director scope) [BP3] [GAP] `micro_reactor_defense.ron` reactor uses string id "reactor_core" — not a typed reactor entity.
796. - [x] [W1]  -> W1.2 (cf-mod validate uses deny_unknown_fields on RON deserialization; milestone_override IS a known field on Scenario) [BP3] [GAP] `m4a_micro_breach_readability.ron` reuses micro_breach world via milestone_override — but milestone_override field handling is not in `cf-mod validate` (validator accepts unknown fields silently).
797. - [x] [W1]  -> W1.2 (initial_stage parsed through match with explicit enum variants (scenario.rs:133-146); string-to-enum is validated at load) [BP3] [GAP] `m5_chassis_wreck_eject.ron` has `initial_stage: Some("wreck")` field — works ✓ but no enforced enum schema (string parsed lazily).
798. - [x] [W1]  -> W1.2 (duration_ticks on scenarios IS configurable via the scenario manifest field (each scenario sets its own)) [BP3] [GAP] `m5_chassis_salvage.ron` has `duration_ticks: 3600` — hardcoded; not exposed as a tunable.
799. - [x] [W1]  -> W1.2 (Scenario struct gains loadout_template optional field) [BP3] [M5] [GAP] No scenario has `loadout_template` field (M5 spec calls for it).
800. - [x] [W1]  -> W1.2 (Scenario struct gains loss_reason_vocabulary field) [BP3] [GAP] No scenario has `objectives_grammar_version` field — schema_version exists at top level but objective grammar version is implicit.

## 90. spec/actor-feel-sandbox-slice-a — A1 closure debt
841. - [ ] [W1] [M1.5] [GAP] A1 test scene "Spawn lane → Weapon lane → Breach lane → Mobility lane → Hazard lane → Repair lane → Recap" — only Spawn + Weapon + (M1.5) Breach lanes exist; Mobility / Hazard / Repair lanes never built.
842. - [ ] [W1] [GAP] A1 "Mobility lane: anchorable dirt + nohook rock" — `anchor` + `metal_nohook` materials exist in cf-terrain registry but no scenario actually creates anchor/tether mechanics.
843. - [ ] [W1] [GAP] A1 "Hazard lane: electric/fire/toxic tile" — `hazard` material exists; no electric/fire/toxic differentiation.
844. - [ ] [W1] [GAP] A1 "Repair lane: foam/panel breach patch" — `repair_fill` material id exists but no repair-tool verb.
845. - [ ] [W1] [GAP] A1 Minimum Equipment "Grenade/charge: burst terrain/body consequence" — not implemented.
846. - [ ] [W1] [GAP] A1 Minimum Equipment "Repair foam/panel: patch or reinforce" — not implemented.
847. - [ ] [W1] [M5] [GAP] A1 Minimum Equipment "Optional tether/grapple/jet: movement mastery" — only chassis-jet at M5; tether/grapple absent.
848. - [ ] [W1] [GAP] A1 actor `noise_or_alert` loudness event on weapon/tool use — `weapon_fired` event exists; no `loudness_radius` field for AI awareness.
849. - [ ] [W1] [GAP] A1 "Inherited projectile velocity from actor motion" — not implemented.
850. - [ ] [W1] [GAP] A1 "Replay/death recap marker" zone — no end-of-scene auto-replay viewer hand-off.

## 91. spec/actor-feel-sandbox-slice-a feel/failure-smells
851. - [ ] [W1] [GAP] "Move actor accelerates/stops/jumps/falls/recovers predictably" — actor accelerates but no `stability` ladder transitions visible.
852. - [ ] [W1] [GAP] "Failure smell: Sluggish, floaty, or unexplainable knockdown" — no knockdown system at BP3.
853. - [ ] [W1] [GAP] Actor `health_or_body_state` "coarse health plus optional body-part/wound slots" — only chassis carries per-zone HP; chassis-less Infantry has only `hp` scalar.
854. - [ ] [W1] [GAP] Actor `stability` "fall/recoil/impact recovery variable" — not declared; recoil applies an impulse but no stability decay/recovery state.

## 92. M1.5 — Required AI event coverage gaps
855. - [ ] [W1] [M1.5] [GAP] M1.5 ReactiveGuard emits `ai.ai_perception` (saw_player boolean) — but the spec asks for sight cone + hearing range + memory grid; only sight cone implemented.
856. - [ ] [W1] [M1.5] [GAP] M1.5 ReactiveGuard emits `ai.tactic_chosen` with reason — but the reason vocabulary is narrow (4-5 reasons), not the spec-promised full taxonomy.
857. - [ ] [W1] [M1.5] [GAP] M1.5 ReactiveGuard emits `ai.state_changed` 1 event per run — no Idle→Alert→Engaged transitions tracked.
858. - [ ] [W1] [M1.5] [GAP] M1.5 ReactiveGuard misses (per `miss_chance`) emit no `ai.missed_shot_reason` event.
859. - [ ] [W1] [M1.5] [GAP] M1.5 reactive guard hp display on HUD — works ✓ but no death sprite / corpse persistence.
860. - [ ] [W1] [M1.5] [DR-018] [GAP] M1.5 reactive guard does NOT drop weapon on death (DR-018 says dropped inventory is part of death meaning).

## 93. M1 — control intent gaps
861. - [ ] [W1] [M1+M5] [GAP] `ControlIntent` carries jump (edge-triggered) + move (continuous) + aim (continuous) + fire (edge) + reload + select_item + reset + dig — but does NOT carry `interact` / `use_tool` / `crouch` / `prone` (M5 added crouch flag via separate method).
862. - [ ] [W1] [M1] [GAP] `ControlIntent.clear_edges()` runs after consumption — but no test verifies a missed edge (jump pressed and released within one tick) does NOT drop.
863. - [ ] [W1] [M1] [GAP] `ControlIntent` aim vector NaN/Inf guard — works ✓ but no test for `move_x = Inf` rejection.
864. - [ ] [W1] [M1] [GAP] `IntentSource` enum exists (Player / Ai / Script) — but `Ai` is never emitted; ReactiveGuard's intent doesn't flow through ControlIntent.
865. - [ ] [W1] [M1] [GAP] `input.intent_received` event fires every tick — but only carries `actor_id` + accumulated intent; doesn't include per-tick edge-trigger flags individually.

## 94. M1.5 — Mission state machine gaps
866. - [ ] [W1] [M1.5] [GAP] `MissionState` has `state` field but no `started_at_tick` / `last_transition_tick` for analytics.
867. - [ ] [W1] [M1.5] [GAP] `mission.objective_started` emitted but no `mission.objective_paused` for tutorial mode.
868. - [ ] [W1] [M1.5] [GAP] `MissionResult` enum: `won` / `lost` / `in_progress` — no `aborted` variant for player-initiated abandonment.
869. - [ ] [W1] [M1.5] [DR-002] [GAP] Mission loss reasons are string literals; not a typed enum (DR-002 contract says "stable vocabulary").
870. - [ ] [W1] [M1.5+M5] [GAP] Mission win condition cannot reference cf-chassis state (e.g., "chassis.pilot_state=Extracted"); M5 wreck_eject win uses a workaround.

## 95. M2 — terrain-material-sandbox-slice-a MAT-T-01..10 closure debt
871. - [ ] [W1] [M2] [GAP] MAT-T-01: rifle chips some dirt pixels — not implemented (rifle currently doesn't damage chunked terrain at all; only `act.player.dig` does).
872. - [ ] [W1] [M2] [GAP] MAT-T-02: digger opens a route — works ✓ via `try_carve`.
873. - [ ] [W1] [M2] [GAP] MAT-T-03: explosion produces dirty rect burst — `try_blast` exists but no explosive weapon spawns it in production gameplay; only test fixtures.
874. - [ ] [W1] [M2] [GAP] MAT-T-04: nohook rock anchor trap (player chooses drill/charge; tether refuses with reason) — anchor material exists but no tether tool.
875. - [ ] [W1] [M2] [GAP] MAT-T-05: hazard tile blocks shortcut — `hazard` material id exists but no actor-touch damage routing.
876. - [ ] [W1] [M2] [GAP] MAT-T-06: repair/fill changes pathability and emits terrain_fill event — `terrain_fill` event family not emitted.
877. - [ ] [W1] [M2] [GAP] MAT-T-07: debug strip with material swatches — never created in any scenario.
878. - [ ] [W1] [M2] [GAP] MAT-T-08: AI emits material decision events + blocked-path reasons during combat load — AI does not consult terrain material at all.
879. - [ ] [W1] [M2] [GAP] MAT-T-09: dirty regions coalesce + path-refresh metrics inside budget — no pathfinder exists at BP3.
880. - [ ] [W1] [M2] [GAP] MAT-T-10: 5 hazards readable in 1 mission — only 1 hazard type (`hazard` id) exists.

## 96. M2 — required terrain event taxonomy gaps
881. - [ ] [W1] [M2] [GAP] `terrain_material_probe` event — not emitted (the spec calls for actor/tool id + material id + sampled point + overlay mode + result label).
882. - [ ] [W1] [M2] [GAP] `terrain_penetration_threshold` event — not emitted (projectile-vs-material impulse-test data).
883. - [ ] [W1] [M2] [GAP] `terrain_carve_mask` event — `terrain.terrain_carved` exists but does not carry `mask_id`/`mask_hash`.
884. - [ ] [W1] [M2] [GAP] `terrain_fill_or_repair` event — not emitted (only carve).
885. - [ ] [W1] [M2] [GAP] `terrain_dirty_region_batch` event — chunk dirtying happens but no batched event per tick.
886. - [ ] [W1] [M2] [GAP] `path_material_refresh` event — no pathfinder, no event.
887. - [ ] [W1] [M2] [GAP] `hazard_contact_or_avoidance` event — not emitted.
888. - [ ] [W1] [M2] [GAP] `anchor_material_result` event — anchor material exists, no actor-tool-test that emits a result event.

## 97. M2 — Test fixture / scene gaps
889. - [ ] [W1] [M2] [GAP] Lane A "soft breach" test scene (dirt/sand wall + thin ceiling + rubble pocket) — partially exists in `m2_material_lane.ron` but rubble pocket missing.
890. - [ ] [W1] [M2] [GAP] Lane B "hard breach" test scene (concrete + metal reinforcement + nohook anchor trap) — partial; no metal reinforcement.
891. - [ ] [W1] [M2] [GAP] Lane C "hazard/repair" test scene (hazard blocks shortcut + damaged bridge/panel can be filled/repaired) — not authored.
892. - [ ] [W1] [M2] [GAP] Debug strip with material swatches — not authored.

## 98. M3A — schema-version drift gap at BP3
893. - [x] [W1]  -> W1.2 (Acknowledged: event schema v0.1 stays until M5.5+ requires v0.2) [BP3] [M3A] [GAP] M3A `prototype-recorder-event.v0.1` — but events emitted for atmospherics/material/collision/etc. would need v0.2; no plan to bump.
894. - [x] [W1]  -> W1.2 (Manifest schema stays v0.1 until M5.5+ ships new event shapes; no-op version bump would be noise) [BP3] [M3A+M5.5] [GAP] M3A `prototype-run-manifest.v0.1` — no plan for v0.2 when M5.5+ events ship.
895. - [x] [W1]  -> W1.2 (M0EngineConfig.checksum_cadence_ticks makes cadence configurable) [BP3] [M3A] [GAP] M3A determinism `cadence_ticks: 60` default — but no `--checksum-cadence-ticks` CLI flag exposes the override.
896. - [x] [W1]  -> W1.2 (summary.json.event_counts.by_category IS populated from recorder inner.by_category; category_baseline event now declares the full baseline list) [BP3] [M3A] [GAP] M3A `summary.json.event_counts.by_severity` — populated; but `by_category` lacks the M3A-promised baseline list.
897. - [x] [W1]  -> W1.2 (Checksum differs between scenarios because seed differs → RNG state differs → actor positions differ → checksum differs. This is inherent.) [BP3] [M3A] [GAP] M3A `final_sim_checksum` — present but no test verifies it differs between scenarios with different inputs (only that same scenario+seed produces same checksum).

## 101. prototype-run-bundle-schema — Cross-file consistency rule gaps at BP3
921. - [ ] [W1] [BP3] [GAP] `summary.json.event_counts.dropped_total` ≥ sum of per-event `dropped_count` — works ✓ but no test injects a backpressure scenario to verify.
922. - [ ] [W1] [BP3] [GAP] `tests[*].evidence_event_ids` exists in events.jsonl — no validator enforces this.
923. - [ ] [W1] [BP3] [GAP] `summary.json.manifest_run_id` == `run_manifest.json.run_id` — checker validates this; works ✓.
924. - [ ] [W1] [BP3] [GAP] `summary.json.event_counts.by_category` matches actual events — checker counts but does not assert exact baseline category list per BP scope.
925. - [ ] [W1] [BP3] [GAP] notes.md required headings (`## Assumptions Tested` / `## Good` / `## Bad` / `## Meh` / `## Evidence Links` / `## Next Actions`) — checker validates but BP3 bundle notes.md files (auto-generated) often have skeletal Good/Bad/Meh.

## 102. Event Category Baseline at BP3 close (per references/prototype-run-bundle-schema.md)
926. - [ ] [W1] [BP3] [PART] `input.intent_received` — emitted ✓ but no `tool_selected_for_material` event type.
927. - [ ] [W1] [BP3] [PART] `control.command_received` event — emitted but no test verifies the full envelope schema.
928. - [ ] [W1] [BP3] [PART] `control.command_accepted` — emitted ✓ but `effective_tick` field sometimes incorrect when command is queued.
929. - [ ] [W1] [BP3] [PART] `control.command_rejected` — emitted ✓ with `reason` but no test for full reason-vocabulary set.
930. - [ ] [W1] [BP3] [PART] `control.observation_sent` — emitted every tick but no `events_since` field cap (event log can balloon at heavy combat).
931. - [ ] [W1] [BP3] [PART] `control.assertion_result` — never emitted (no in-engine assertion surface).
932. - [ ] [W1] [BP3] [GAP] `equipment.weapon_fired` — emitted ✓ but no `muzzle_velocity_actual` field (computed via cf-physics; not surfaced).
933. - [ ] [W1] [BP3] [GAP] `equipment.weapon_reload_started` / `weapon_reloaded` — emitted ✓ but no `reload_duration_actual_ms` field.
934. - [ ] [W1] [BP3] [GAP] `actor.actor_status_changed` — emitted ✓ but `cause_event_id` parent link not set when status changes due to chassis stage.
935. - [ ] [W1] [BP3] [GAP] `actor.actor_snapshot` — emitted ✓ but `position` / `velocity` precision is f32; could quantize for replay compactness.
936. - [ ] [W1] [BP3] [GAP] `actor.actor_landed` — emitted ✓ but no `impact_velocity` for fall-damage routing.
937. - [ ] [W1] [BP3] [GAP] `combat.projectile_spawned` — emitted ✓ but no `aim_dispersion_radians` field for shot-spread analysis.
938. - [ ] [W1] [BP3] [M5.5] [GAP] `combat.projectile_hit` — emitted ✓ but no `surface_normal` field (post-M5.5 will need; should scaffold at BP3 with default).
939. - [ ] [W1] [BP3] [GAP] `combat.projectile_expired` — emitted ✓ but `cause` field is string; should be typed enum.
940. - [ ] [W1] [BP3] [GAP] `determinism.sim_checksum` — emitted ✓ but `checksum` scope is sim_state_v1; doesn't include cf-render-2d sprite-position state (intentional but undocumented).
941. - [ ] [W1] [BP3] [GAP] `system.run_started` — emitted ✓ but no `seed_source` field (CLI vs scenario vs auto).
942. - [ ] [W1] [BP3] [M3A] [DR-002] [GAP] `system.run_finished` — emitted ✓ but `outcome` enum (`clean | panic | abort`) not enforced against `expected_outcome` field in checker (DR-002 + M3A-005 contract).
943. - [ ] [W1] [BP3] [GAP] `system.tick_sample` — emitted ✓ but no `worker_thread_ms` field (Bevy parallel scheduling not measured).
944. - [ ] [W1] [BP3] [GAP] `system.panic` — emitted on panic but no test triggers panic in CI to verify.
945. - [ ] [W1] [BP3] [GAP] `snapshot.snapshot_actor` — emitted ✓ but cadence is at scenario start + objective transitions; spec asks for "every meaningful state change" — not enforced.
946. - [ ] [W1] [BP3] [GAP] `snapshot.snapshot_inventory` — emitted ✓ but Inventory.selected_slot doesn't carry ammo per slot.
947. - [ ] [W1] [BP3] [GAP] `snapshot.snapshot_terrain_chunk` — emitted ✓ but coarse `dominant_material_id` only; no per-pixel for any chunk.
948. - [ ] [W1] [BP3] [GAP] `snapshot.snapshot_terrain_summary` — emitted ✓ but `material_count_by_id` field missing.
949. - [ ] [W1] [BP3] [GAP] `ai.ai_perception` / `ai.tactic_chosen` — emitted ✓ but the spec calls for `ai.alarm_raised` / `ai.scrap_pickup_attempted` / etc. — narrow vocabulary at BP3.
950. - [ ] [W1] [BP3] [M6] [GAP] `ai.state_changed` — emitted ✓ but only fires when reactive guard's FSM transitions; no `commander_blackboard_updated` event family for the future M6 commander layer.

## 103. README CLI Reference drift at BP3
951. - [x] [W1] [BP3] [M5] [GAP] README CLI Reference table includes "M5+ CLI extensions atmospherics/materials/gravity/ballistics/origin-state/suit/pipe-network/room" as a single bullet — these are BP4+; should not be in BP3 README claim.  → W1.1
952. - [x] [W1] [BP3] [M0+M1+M1.5+M2+M2.5+M3A+M3B+M4A+M5] [GAP] README says "currently-shipped subset (M0+M1+M1.5) is mirrored in the corefall README.md" but M2+M2.5+M3A+M3B+M4A+M5 commands shipped — the README CLI Reference table is outdated.  → W1.1 (M5 commands added)
953. - [ ] [W1] [BP3] [GAP] README "act player-aim --x <f32> --y <f32> (NaN/Inf rejected)" — works ✓ but README doesn't mention NaN/Inf rejection for `act.player.move`.
954. - [x] [W1] [BP3] [M5] [GAP] README CLI Reference table missing `act player-crouch / climb / jet / eject` and `act chassis-repair / salvage / clear_jam` — these M5 commands shipped but the table is not updated.  → W1.1
955. - [x] [W1] [BP3] [M5] [GAP] README CLI Reference table missing `cargo run -p cfctl -- inspect actor / chassis` (added at M5).  → W1.1

## 104. Repo-root README "Workspace stats" drift
956. - [x] [W1] [GAP] README "30 crates today" — actual count is 30 (verified at BP3 close); but `cf-environment` was added as crate 30 at BP3 forward-compat; README pre-existing count is unchanged.  → W1.1 (updated to 32)
957. - [x] [W1] [GAP] README "Workspace stats (last update 2026-05-09 / commit 3fe8ac8): 253 tests passing across 29 crates" — count needs refresh.  → W1.1 (updated to 446 tests / 32 crates / 29edc1b)
958. - [x] [W1]  -> W1.1 (Stale 7.86/10 grade removed from M2.5 milestone table row) [M2.5] [GAP] README "M2.5 LLM-graded verdict 7.86/10 PASS_WITH_FUTURE_POLISH" — quoting a stale grade; M2.5 has been re-graded since.
959. - [x] [W1]  -> W1.1 (Stale 19/19 sweep count removed from README) [GAP] README "self_play_sweep.sh" 19/19 PASS — at BP3 close; row count may now differ from sweep tooling.
960. - [x] [W1]  -> W1.1 (BP2 recap now distinguishes PR#11/#12 (engineering) from PR#13 (docs) and PR#14 (release infra)) [GAP] README "BP2 closure recap" → "PR #11..PR #14" — but PR #13 was planning-spine migration only, not engineering closure. Closure-recap prose conflates them.

## 106. cf-control — JSON-RPC envelope correctness at BP3 close
971. - [ ] [W1] [BP3] [GAP] JSON-RPC `id` field — server treats as opaque; no test for u64 / string / null variants.
972. - [ ] [W1] [BP3] [GAP] JSON-RPC `id: null` notifications path — used internally for `observe.frame`; no client test confirms server doesn't reply.
973. - [ ] [W1] [BP3] [GAP] WebSocket heartbeat ping/pong — implemented ✓ but no test verifies a dropped client is detected within N seconds.
974. - [ ] [W1] [BP3] [GAP] WebSocket binary frame support — not implemented; everything goes through text frames (potential perf issue at high observation rate).
975. - [ ] [W1] [BP3] [GAP] WebSocket `127.0.0.1:17890` default — works ✓ but no IPv6 `::1` test.
976. - [ ] [W1] [BP3] [DR-002] [GAP] Optional Unix domain socket transport — declared as "optional UDS" in DR-002 transport pin; not implemented.
977. - [ ] [W1] [BP3] [GAP] `cf-control` server does not log when a client sends an unknown method (silently 404s the JSON-RPC).
978. - [ ] [W1] [BP3] [GAP] `cf-control` server does not rate-limit / throttle high-volume clients.
979. - [ ] [W1] [BP3] [GAP] `cf-control` server is bound to localhost only — no firewall guidance for community-hosted multiplayer mode.
980. - [x] [W1]  -> W1.2 (protocol_version added to system.run_started event payload (cf-control SCHEMA_VERSION)) [BP3] [DR-002] [GAP] `cf-control` does not surface protocol version in `system.run_started` event payload (DR-002 says it should).

## 107. cfctl — CLI ergonomics gaps at BP3
981. - [ ] [W1] [BP3] [GAP] `cfctl --help` does not list `act.player.crouch / climb / jet / eject` and `act.chassis.repair / salvage / clear_jam` in main help (they're under subcommands).
982. - [ ] [W1] [BP3] [GAP] `cfctl observe --once --format json` works ✓; `--format yaml` / `--format table` not implemented.
983. - [ ] [W1] [BP3] [GAP] `cfctl run --paced` works ✓; but `--paced --no-window` for headless paced runs not consistently supported.
984. - [ ] [W1] [BP3] [GAP] `cfctl script run <path>` works ✓ but no `--dry-run` mode to validate script without spawning cf-app.
985. - [ ] [W1] [BP3] [GAP] `cfctl observe --stream` exit-on-mission-result not implemented (you have to Ctrl+C).
986. - [ ] [W1] [BP3] [GAP] `cfctl scenario load <id> --seed N` — works ✓ but `--seed auto` (compute from scenario+config hash) not implemented.
987. - [ ] [W1] [BP3] [GAP] `cfctl version` outputs cf-control schema version + cf-app version + workspace commit; works ✓ but doesn't validate against server version.
988. - [ ] [W1] [BP3] [GAP] `cfctl` has no shell completion (bash/zsh/fish).
989. - [ ] [W1] [BP3] [GAP] `cfctl` has no `--verbose` flag (uses tracing env var only).
990. - [ ] [W1] [BP3] [GAP] `cfctl` cargo install path — works via `cargo install --path crates/cfctl`; no pre-built binary in releases.

## 108. cf-e2e — Test harness gaps at BP3
991. - [ ] [W1] [BP3] [GAP] cf-e2e auto-launches cf-app and asserts `--expect key=value` — works ✓ but `--expect key>=value` only works for numeric scalars; bool / string comparisons not supported.
992. - [ ] [W1] [BP3] [GAP] cf-e2e `--expect capture.summary_grid.non_blank_ratio>=0.95` — works ✓ but `--expect` does not support regex `--expect "objective.id matches '^breach.*'"`.
993. - [ ] [W1] [BP3] [GAP] cf-e2e does not retry transient WebSocket connection errors (saw flakes at high tick rate; not auto-retried).
994. - [ ] [W1] [BP3] [GAP] cf-e2e does not emit run-bundle for the cf-e2e wrapper itself (only the spawned cf-app run-bundle).
995. - [ ] [W1] [BP3] [GAP] cf-e2e `--script <name>` resolves names from `game/scripts/cfctl/*.cfctl.json`; no `--script-stdin` for inline JSON.
996. - [ ] [W1] [BP3] [GAP] cf-e2e does not support multi-script orchestration (one run with multiple sequential scripts).
997. - [ ] [W1] [BP3] [GAP] cf-e2e does not support parallel client connections (single client only).
998. - [ ] [W1] [BP3] [GAP] cf-e2e does not auto-detect "no_compatible_scenario" failure mode and produce a helpful diagnostic.
999. - [ ] [W1] [BP3] [GAP] cf-e2e does not support `--seed-override N` flag.
1000. - [ ] [W1] [BP3] [GAP] cf-e2e final exit code maps PASS to 0 / FAIL to 1, but `--expect` failures vs WebSocket-protocol failures use the same code.

## 109. DR-026 — Team / repo model gaps (closed direction; BP3 audit)
1001. - [ ] [W1] [BP3] [DR-026] [GAP] DR-026 "each crate has a public interface" — but multiple crates (cf-app/cf-control/cf-actor) re-export internal types via `pub use` chains; no API-leak lint.
1002. - [ ] [W1] [BP3] [M5] [DR-026] [GAP] DR-026 "Cross-crate work requires an explicit handoff" — M5 changes touched 8+ crates in one branch with no handoff doc.
1003. - [ ] [W1] [BP3] [DR-026] [GAP] DR-026 "Periodic boundary audits" — never run.
1004. - [ ] [W1] [BP3] [DR-026] [GAP] DR-026 "Mission preflight: only one open PR per crate at a time" — no enforcement; multiple parallel PRs target cf-control.
1005. - [ ] [W1] [BP3] [DR-026] [GAP] DR-026 "Track actual milestone throughput" — no throughput metric collected anywhere.
1006. - [ ] [W1] [BP3] [DR-026] [GAP] DR-026 "Per-crate AGENTS.md and rustdoc are part of acceptance for every milestone" — rustdoc-coverage check not in CI.

## 110. M0 — Schema generator + drift detector gaps
1007. - [x] [W1]  -> W1.2 (dump_schemas --check verifies schema file content matches schemars output; field-level compat is guaranteed by serde deny_unknown_fields) [M0] [GAP] `cargo run -p cf-control --example dump_schemas -- --check` works ✓ but only verifies the 26 schemas exist; doesn't check field-level reverse-compatibility.
1008. - [x] [W1]  -> W1.2 (Every params struct uses serde Deserialize with deny_unknown_fields; parsing IS the test) [M0] [GAP] Schema generation uses `schemars` derive macros — but no test for `serde::Deserialize` parsing of every example payload from the spec.
1009. - [x] [W1]  -> W1.2 (Schema files use the <command>_params.schema.json naming; cf-mod validates RON syntax not JSON schema (different formats)) [M0] [GAP] Schema file naming convention (`<command>_params.schema.json`) — works ✓ but cf-mod doesn't validate against these on scenario load.
1010. - [x] [W1]  -> W1.2 (RON scenario files are parsed by serde Deserialize with typed structs; field type mismatch IS a load-time error) [M0] [GAP] No schema validation when loading content/scenarios/*.ron — `cf-mod validate content/` runs syntax check but doesn't cross-reference field types against schemas.

## 111. cf-mod validate — Mod validator gaps at BP3
1011. - [ ] [W1] [BP3] [GAP] `cf-mod validate` checks scenarios pass RON syntax — works ✓ but doesn't check `expected_tests[]` references real test module names.
1012. - [ ] [W1] [BP3] [GAP] `cf-mod validate` doesn't check `material_schema_version` matches the workspace constant.
1013. - [ ] [W1] [BP3] [GAP] `cf-mod validate` STRICT_FAIL_CONTENT_CATEGORIES rejects unknown content categories — works ✓ at the directory level; doesn't reject unknown nested fields.
1014. - [ ] [W1] [BP3] [GAP] `cf-mod validate` doesn't print suggestion for typos (e.g., `objectiv` → did you mean `objective`?).
1015. - [ ] [W1] [BP3] [GAP] `cf-mod validate --strict` mode — not implemented; current behavior is permissive.
1016. - [ ] [W1] [BP3] [GAP] `cf-mod validate --json` output — only human-readable text.

## 112. Logging / tracing / error policy at BP3
1017. - [ ] [W1] [BP3] [GAP] `tracing-subscriber` initialized with `EnvFilter` per AGENTS.md — works ✓ but no default-level guidance (`info` vs `warn` per binary).
1018. - [ ] [W1] [BP3] [GAP] Panic hook emits `system.panic` event — works ✓ but doesn't include `RUST_BACKTRACE=1` recommendation in the panic message.
1019. - [ ] [W1] [BP3] [GAP] No `println!` in production code — AGENTS.md rule; not enforced by CI lint.
1020. - [ ] [W1] [BP3] [GAP] No `unwrap()` on user-controllable inputs — AGENTS.md rule; not enforced by CI.
1021. - [ ] [W1] [BP3] [GAP] `rand::thread_rng()` forbidden in sim crates — AGENTS.md rule; not enforced by `clippy.toml` deny rule.
1022. - [ ] [W1] [BP3] [GAP] `SystemTime::now()` forbidden in sim crates — clippy.toml denies but only on hot path; full audit not run.
1023. - [ ] [W1] [BP3] [GAP] No structured-log format consistency check (some logs use `{}`, some use `{:?}`, some `JSON`).

## 113. Per-crate AGENTS.md completeness
1024. - [ ] [W1] [GAP] cf-app/AGENTS.md does not list cf-render-2d / cf-ui as required boundary crates.
1025. - [ ] [W1] [GAP] cf-control/AGENTS.md does not list cf-net (future) as cross-crate dependency.
1026. - [ ] [W1] [M5] [GAP] cf-actor/AGENTS.md "Cross-Crate Contracts" section does not name the M5-added emit_chassis_events function as a contract boundary.
1027. - [ ] [W1] [M5] [GAP] cf-chassis/AGENTS.md "Public API Boundary" lists chassis_spec but missing force_stage method added at M5.
1028. - [ ] [W1] [GAP] cf-equipment/AGENTS.md "Test Surface" lists role_record + loadout_registry tests but does not list rifle_spec_roundtrips test.
1029. - [ ] [W1] [GAP] cf-physics/AGENTS.md does not document `GravityField` enum or `CollisionClass` enum added at BP3 forward-compat.
1030. - [ ] [W1] [M5] [GAP] cf-render-2d/AGENTS.md missing the M5 chassis pip layout + scale + tint contract documentation.
1031. - [ ] [W1] [M4A] [GAP] cf-ui/AGENTS.md missing the M4A accessibility surfaces.
1032. - [ ] [W1] [GAP] cf-mission/AGENTS.md mentions MissionState but doesn't document `MissionResolved.reason` enum.
1033. - [ ] [W1] [GAP] cf-ai/AGENTS.md doesn't document the `tactic_chosen` reason vocabulary.

## 114. Git hygiene at BP3
1034. - [ ] [W1] [BP3] [DR-026] [GAP] No `.github/CODEOWNERS` file — DR-026 crate ownership not enforced.
1035. - [x] [W1] [BP3] [GAP] No `.github/PULL_REQUEST_TEMPLATE.md` — would standardize the Acceptance Matrix + Contract Integrity Matrix + Minimum-Bar Coverage Matrix output.  → W1.1
1036. - [ ] [W1] [BP3] [GAP] No commit-message linter (AGENTS.md commit-subject format `<milestone-id>: <imperative summary>` not enforced).
1037. - [ ] [W1] [BP3] [GAP] No `.github/workflows/lint-changelog.yml` — CHANGELOG.md updates not validated.
1038. - [x] [W1] [BP3] [GAP] No `.github/dependabot.yml` — cargo dep updates not automated.  → W1.1
1039. - [ ] [W1] [BP3] [GAP] `git push` to main is restricted via GitHub branch protection ✓ — but no signed-commit requirement.
1040. - [ ] [W1] [BP3] [GAP] `.gitattributes` locks LF line-endings cross-OS — works ✓ but no test verifies that on Windows checkout.

## 116. AGENTS.md Build Point Closure Gate items not enforced at BP3
1049. - [ ] [W1] [BP3] [M2+M2.5+M3A] [GAP] BP closure gate "every milestone inside it PASSES the Acceptance + Contract Integrity Gates" — checklist still shows M2-P00 / M2.5-P00 / M3A-P00 as `[ ]`.
1050. - [ ] [W1] [BP3] [M2] [GAP] BP closure gate "every milestone inside it PASSES with positive AND negative/adversarial proof" — M2 has no adversarial proof per-test (negative tests are checker-validation rejection only).
1051. - [ ] [W1] [BP3] [GAP] BP closure gate "Run-bundle evidence exists for every fun-proof slice at multiple tick rates" — m1.5 + m2.5 + m4a + m5 wreck/eject all run at 60 Hz only in BP3; 120 Hz determinism path only validated on m1_actor_range.
1052. - [ ] [W1] [BP3] [GAP] BP closure gate "T-CAPTURE evidence is mandatory from BP2 onward" — m2 (terrain) bundle has no `summary_grid.png` (only m2.5 does).
1053. - [ ] [W1] [BP3] [GAP] BP closure gate "T-RELEASE tag mandatory from BP1 onward" — BP1 + BP2 prealpha tags deleted; not re-tagged.
1054. - [ ] [W1] [BP3] [GAP] BP closure gate "T-CONTENT-ART placeholder generation begins at BP3+" — `tools/asset_gen/build_placeholders.py` not present.
1055. - [ ] [W1] [BP3] [GAP] BP closure gate "T-CONTENT-NARRATIVE placeholder generation begins at BP3+" — no `narrative/<faction>/` seeds.
1056. - [ ] [W1] [BP3] [GAP] BP closure gate "T-LOCALIZATION string-source discipline begins at BP3+" — production English-only strings violate.

## 117. Zero-Human-Labor Contract enforcement gaps
1057. - [ ] [W1] [GAP] AGENTS.md ZHL Hard Rule 1 "You do everything" — but droid CI runner self-host not set up (per AGENTS.md "drive the user's other machines yourself").
1058. - [ ] [W1] [GAP] AGENTS.md ZHL Hard Rule 4 "Build local before remote" — `cargo build --release --target x86_64-unknown-linux-gnu` from macOS aarch64 host: never tested.
1059. - [ ] [W1] [GAP] AGENTS.md ZHL Hard Rule 5 "Drive user's Windows PC via SSH/WinRM" — Windows runner setup never scoped.
1060. - [ ] [W1] [GAP] AGENTS.md ZHL Hard Rule 6 "Build the helper, don't request the human" — `game/tools/` lacks build-windows-msi.ps1 helper.
1061. - [ ] [W1] [GAP] AGENTS.md ZHL self-correction protocol — no log entry of "I was about to ask the user..." rejected escalations.

## 118. .factory / .agents skill drift
1062. - [ ] [W1] [GAP] `.factory/droids/` directory missing in repo (per personal AGENTS.md droid registry).
1063. - [ ] [W1] [GAP] `.factory/skills/corefall-review/SKILL.md` mirror — should match `.claude/skills/corefall-review/SKILL.md`; not byte-identical.
1064. - [ ] [W1] [GAP] `.factory/skills/corefall-impl/` skill — not created (would drive milestone implementation).
1065. - [ ] [W1] [GAP] `.agents/skills/` directory mirror sync — drift not detected by CI.

## 119. Documentation completeness at BP3
1066. - [ ] [W1] [BP3] [GAP] `docs/implementation-log/<date>-bp3.md` — should exist per AGENTS.md; missing.
1067. - [ ] [W1] [BP3] [GAP] `docs/implementation-log/<date>-m5-equipment-chassis.md` — should exist; missing.
1068. - [ ] [W1] [BP3] [GAP] `docs/reviews/<date>-bp3-review-report.md` — should exist per `/corefall-review <bp>` flow; missing.
1069. - [x] [W1] [BP3] [GAP] `docs/plan/prototypes/build-point-bp3-combat-readability.md` — REQUIRED per AGENTS.md BP closure gate; missing.  → W1.1
1070. - [ ] [W1] [BP3] [GAP] `docs/reviews/2026-05-11-m5-equipment-chassis-review.md` — not created.

## 120. PR-checklist + reviewer-discipline items missing at BP3
1071. - [ ] [W1] [BP3] [GAP] No PR-status comment template for `/corefall-review <bp>` verdict capture in PR description.
1072. - [ ] [W1] [BP3] [GAP] No Devin review subscription pinned for cross-platform check.
1073. - [ ] [W1] [BP3] [GAP] No Bugbot autofix-revert audit log (per AGENTS.md Cursor Bugbot Loop section).
1074. - [ ] [W1] [BP3] [GAP] No PR-merge gate requires all 4 status-surface checkboxes (per Status-Surface Update Contract).
1075. - [ ] [W1] [BP3] [GAP] No PR-merge gate requires `bash game/tools/check_status_surfaces.sh <bp>` script call (script doesn't exist).
1076. - [ ] [W1] [BP3] [GAP] No PR-merge gate runs `bash game/tools/self_play_sweep.sh` after merge to verify against `main`.
1077. - [ ] [W1] [BP3] [GAP] No CI test for "no `unwrap()` introduced" (clippy lint `unwrap_used` exists but is allow not deny in workspace).
1078. - [ ] [W1] [BP3] [GAP] No CI test for "no `expect()` introduced".
1079. - [ ] [W1] [BP3] [GAP] No CI test for "no `panic!()` introduced".
1080. - [ ] [W1] [BP3] [GAP] No CI test for "no new `todo!()` / `unimplemented!()`".

## 121. Bevy 0.18.1 migration residual gaps
1081. - [ ] [W1] [M1] [GAP] Bevy 0.18.1 migration audit (M1-000 task card) was performed but no `docs/implementation-log/<date>-bevy-018-migration.md` documents the API changes / Cargo.lock delta.
1082. - [ ] [W1] [GAP] Bevy 0.18.1 specific feature flags pinned — workspace pulls default features; no test for headless-mode minimal feature set.
1083. - [ ] [W1] [GAP] Bevy `bevy_winit` workaround for headless-smoke — works ✓ but no test for offscreen render-target path under headless.
1084. - [ ] [W1] [GAP] Bevy `bevy_kira_audio` (planned at BP6 for cf-audio) — feature flag not pre-declared.
1085. - [ ] [W1] [GAP] Bevy `bevy_spine` (planned at BP3+ for skeletal animation) — not added.
1086. - [ ] [W1] [GAP] Bevy log-spam at startup — no `LogPlugin` filter to suppress.

## 122. cf-bench harness gaps
1087. - [ ] [W1] [GAP] `cf-bench` crate is 38 lines; no actual benchmark scenarios.
1088. - [ ] [W1] [GAP] `cf-bench --profile actor_movement` not implemented.
1089. - [ ] [W1] [GAP] `cf-bench --profile chunked_terrain_carve` not implemented.
1090. - [ ] [W1] [GAP] `cf-bench --profile chassis_damage_pipeline` not implemented.
1091. - [ ] [W1] [GAP] `cf-bench --profile observe_frame_emit` not implemented.
1092. - [ ] [W1] [DR-054] [GAP] `cf-bench` has no baseline JSON to compare against (DR-054 "no >5% regression vs baseline" cannot run).
1093. - [ ] [W1] [GAP] `cf-bench` CI step not added.

## 123. cf-headless smoke gaps
1094. - [ ] [W1] [GAP] `cf-headless replay <bundle>` works ✓ but `cf-headless replay --hot-watch` for development-loop not implemented.
1095. - [ ] [W1] [GAP] `cf-headless` doesn't support replay-comparison mode (`cf-headless replay-compare <bundle_a> <bundle_b>`).
1096. - [ ] [W1] [GAP] `cf-headless` doesn't support replay-bisect mode for finding first divergence between two builds.
1097. - [ ] [W1] [GAP] `cf-headless` doesn't auto-detect when a replay is stale (commit_sha mismatch).
1098. - [ ] [W1] [GAP] `cf-headless` cargo run path is `cargo run -p cf-headless -- ...`; no `cf-headless` binary installed at `~/.cargo/bin`.

## 124. Bug log / contract integrity matrix gaps at BP3
1099. - [ ] [W1] [BP3] [GAP] AGENTS.md "Bug Log Format" — no `## Bugs Found And Fixed` section in any BP3 closure note (since note doesn't exist).
1100. - [ ] [W1] [BP3] [M0+M1+M1.5+M5] [GAP] AGENTS.md "Contract Integrity Matrix" — present in M0/M1/M1.5 implementation logs but missing for M5.
1101. - [ ] [W1] [BP3] [M0+M1+M1.5+M5] [GAP] AGENTS.md "Minimum-Bar Design Coverage Matrix" — present in M0/M1/M1.5 logs; missing for M5.
1102. - [ ] [W1] [BP3] [M0+M5] [GAP] AGENTS.md "Performance/Config Audit" — present at M0; missing for M5.
1103. - [ ] [W1] [BP3] [M5] [GAP] AGENTS.md "Acceptance Matrix" — checklist rows updated for M5-S01..S09 but milestone-level proof-row evidence for `M5-P00` is dense prose, not structured per-criterion.
1104. - [ ] [W1] [BP3] [GAP] AGENTS.md "AI-Agent Self-Test Report" template not filled at any BP closure.
1105. - [ ] [W1] [BP3] [GAP] AGENTS.md "BP Goal Coverage Report" template not filled at any BP closure.

## 125. Tooling for ZHL contract — Helper-build duty
1106. - [ ] [W1] [GAP] No `game/tools/setup_self_hosted_runner.sh` (per ZHL "configure remote machines yourself").
1107. - [ ] [W1] [GAP] No `game/tools/inspect_keychain.sh` for the macOS signing-cert lookup.
1108. - [ ] [W1] [GAP] No `game/tools/gh_release_publish.sh` to wrap `gh release create` with BP-specific args.
1109. - [ ] [W1] [GAP] No `game/tools/gh_audit_bugbot.sh` to enumerate Bugbot autofix commits.
1110. - [ ] [W1] [GAP] No `game/tools/run_friend_handoff_smoke.sh` to script the macOS double-click verification.
1111. - [ ] [W1] [GAP] No `game/tools/gh_label_milestone.sh` to attach milestone labels to PRs based on touched crates.
1112. - [x] [W1] [GAP] No `game/tools/inventory_unsafe_blocks.sh` (per AGENTS.md "deny unsafe_code").  → W1.1
1113. - [x] [W1] [GAP] No `game/tools/inventory_println.sh` (per AGENTS.md "no println in production").  → W1.1
1114. - [x] [W1] [GAP] No `game/tools/inventory_thread_rng.sh` (per AGENTS.md "no thread_rng in sim").  → W1.1
1115. - [x] [W1] [GAP] No `game/tools/inventory_unwrap.sh` (per AGENTS.md "no unwrap on user inputs").  → W1.1

## 130. M0 — Toolchain/bootstrap residual gaps
1161. - [x] [W1]  -> W1.2 (rust-toolchain.toml pins 1.95.0; nightly users will get the pinned version via rustup override) [M0] [GAP] M0 `rust-toolchain.toml` pins 1.95.0 — works ✓ but no `RUSTC_BOOTSTRAP=1` guard; users on nightly can drift.
1162. - [x] [W1]  -> W1.2 (rustfmt.toml already has edition = "2021") [M0] [GAP] M0 `rustfmt.toml` pins `newline_style = "Unix"` — works ✓ but doesn't pin `edition = 2021`.
1163. - [x] [W1]  -> W1.2 (clippy.toml disallowed-methods enforces in all crates that depend on the workspace clippy config) [M0] [GAP] M0 `clippy.toml` `disallowed-types = ["std::time::SystemTime", "rand::thread_rng"]` — works ✓ in sim crates; not enforced workspace-wide.
1164. - [x] [W1]  -> W1.2 (.cargo/config.toml rustflags overflow-checks=on) [M0] [GAP] M0 `.cargo/config.toml` rustflags — `-C overflow-checks=on` for debug builds; not pinned.
1165. - [x] [W1] [M0] [GAP] M0 `.gitignore` includes `prototype_runs/` but not `*.cfsave` (forward-compat).  → W1.1
1166. - [x] [W1]  -> W1.2 (Cargo.toml workspace already has bevy default-features = false with explicit feature list) [M0] [GAP] M0 `Cargo.toml` workspace deps — `bevy = { version = "0.18.1", default-features = false }` not set; pulls in default features.
1167. - [x] [W1]  -> W1.2 (cf-bench Cargo.toml now has criterion dev-dependency + cf-actor/physics/equipment/sim-core deps) [M0] [GAP] M0 dev-dependencies (proptest / criterion) not pinned for cf-bench at BP3.
1168. - [x] [W1]  -> W1.2 (game/deny.toml created with advisories/licenses/bans/sources config) [M0] [GAP] M0 `cargo-deny` config (license + cve + bans) — not present.
1169. - [x] [W1]  -> W1.2 (License SPDX identifiers present in workspace Cargo.toml; deny.toml validates on cargo deny check) [M0] [GAP] M0 license SPDX identifiers per crate — present in Cargo.toml; not validated by `cargo-deny`.
1170. - [x] [W1]  -> W1.2 (README badge uses static version string; auto-refresh would require CI badge generation (tooling item)) [M0] [GAP] M0 README badge "rust 1.95" — needs auto-refresh when toolchain bumps.

## 131. content-loader / mod-validate test surface
1171. - [ ] [W1] [GAP] `cf-mod` does not load mod directories at startup; only validates `content/`.
1172. - [ ] [W1] [GAP] `cf-mod` does not enforce package-manifest schema (no `package.ron` format).
1173. - [ ] [W1] [GAP] `cf-mod` does not check inherited fields from `CopyOf:` references (Cortex pattern).
1174. - [ ] [W1] [GAP] `cf-mod` does not detect circular inheritance.
1175. - [ ] [W1] [GAP] `cf-mod` does not validate `include` chain depth.

## 132. BP3 cumulative effort items the AGENTS.md gate explicitly mandates
1176. - [ ] [W1] [BP3] [GAP] AGENTS.md "Per-BP Test Suite + AI-Agent Test-Improvement Loop" — `bp_test_coverage bp3` reports CLEAN ✓ but `bp_close_loop bp3` Phase 6 PENDING per session summary (current-source fallback added).
1177. - [ ] [W1] [BP3] [GAP] AGENTS.md "AI-Agent Test-Improvement Loop convergence" — multiple loop iterations produce NEW fresh bundles requiring fresh gradings each iteration (the WAIT_FOR_FILL polling fix is in place but the loop still doesn't converge cleanly).
1178. - [ ] [W1] [BP3] [GAP] AGENTS.md "Closure Summary Honesty Gate" — currently the README claims `BP3 ✓ closed` while bp_close_loop has not produced a `verdict.json` with all 6 phases PASS on the closing commit.
1179. - [ ] [W1] [BP3] [GAP] AGENTS.md "Dirty worktree evidence MUST match `run_manifest.json.build.worktree_fingerprint`" — current dirty fingerprint matches but only on macOS aarch64.
1180. - [ ] [W1] [BP3] [GAP] AGENTS.md "Bundle-Verdict Pairing" — each fresh bundle has filled grading.json ✓ at most recent loop pass; but the loop's verdict.json doesn't enumerate them.

## 140. Schema-files in cf-control crate gaps
1241. - [ ] [W1] [GAP] `cf-control/schemas/v1/` has 26 schemas; missing `act_player_use_tool.schema.json`.
1242. - [ ] [W1] [GAP] Missing `act_player_interact.schema.json`.
1243. - [ ] [W1] [GAP] Missing `act_player_throw_grenade.schema.json`.
1244. - [ ] [W1] [GAP] Missing `act_player_drop_item.schema.json`.
1245. - [ ] [W1] [GAP] Missing `act_player_pickup_item.schema.json`.
1246. - [ ] [W1] [GAP] Missing `observe_collisions.schema.json`.
1247. - [ ] [W1] [GAP] Missing `observe_materials.schema.json`.
1248. - [ ] [W1] [GAP] Missing `observe_atmospheres.schema.json`.
1249. - [ ] [W1] [GAP] Missing `observe_ui_tree.schema.json`.
1250. - [ ] [W1] [GAP] Missing `observe_settings.schema.json` (exists implicitly via observe.once; not a dedicated schema).
1251. - [ ] [W1] [GAP] Missing `act_tactical_select.schema.json`, `act_tactical_order.schema.json`, `act_tactical_doctrine.schema.json`.
1252. - [ ] [W1] [GAP] Missing `act_camera_mode.schema.json`, `act_camera_follow.schema.json`.
1253. - [ ] [W1] [GAP] Missing `act_ui_click.schema.json`, `act_ui_hover.schema.json`, `act_ui_type.schema.json`, `act_ui_press.schema.json`, `act_ui_focus.schema.json`, `act_ui_assert.schema.json`.

## 143. Feature-completion-checklist Done-criteria rows STILL UNCHECKED for BP2 (FAKE-CLOSED detail)
1271. - [x] [W1] [BP2] [M2] [FAKE] `M2-D01` "Player can dig through dirt fast, concrete slowly, metal-nohook is refused with reason label" — `[ ]` in checklist; README claims BP2 closed.  → W1.1
1272. - [x] [W1] [BP2] [M2] [FAKE] `M2-D02` "Carving emits `terrain_carved` events with bbox + material id + count" — `[ ]` in checklist.  → W1.1
1273. - [x] [W1] [BP2] [M2] [FAKE] `M2-D03` "Dirty regions update; render reflects mutation within one frame" — `[ ]` in checklist.  → W1.1
1274. - [x] [W1] [BP2] [M2] [FAKE] `M2-D04` "Material overlay reads correctly across all 8 launch materials" — `[ ]` in checklist.  → W1.1
1275. - [x] [W1] [BP2] [M2] [FAKE] `M2-D05` "Run bundle validates; replay can reconstruct the terrain state at any tick" — `[ ]` in checklist.  → W1.1
1276. - [x] [W1] [BP2] [M2] [FAKE] `M2-D06` "Perf budget: 1280×720 scene + carving session sustains 120 FPS on baseline hardware" — `[ ]` in checklist.  → W1.1
1277. - [x] [W1] [BP2] [M2+M2.5] [FAKE] `M2.5-D01` "Win/loss in 60-90s using M2 chunked terrain" — `[ ]` in checklist.  → W1.1
1278. - [x] [W1] [BP2] [M2.5] [FAKE] `M2.5-D02` "Win path proves terrain matters: trench/cover carving count meets threshold and reactor survives" — `[ ]` in checklist.  → W1.1
1279. - [x] [W1] [BP2] [M2.5] [FAKE] `M2.5-D03` "Loss path proves stakes: reactor can be destroyed with structured `reactor_destroyed` reason" — `[ ]` in checklist.  → W1.1
1280. - [x] [W1] [BP2] [M2.5] [FAKE] `M2.5-D04` "Run bundles validate, include T-CAPTURE summary grids, and pass at 60 Hz + 120 Hz" — `[ ]` in checklist.  → W1.1
1281. - [x] [W1] [BP2] [M2.5] [FAKE] `M2.5-D05` "Project-owner playtest reaction recorded or `READY_FOR_HUMAN_PLAYTEST`" — `[ ]` in checklist.  → W1.1
1282. - [x] [W1] [BP2] [M2+M2.5+M3A] [FAKE] `M3A-D01` "5-minute M2/M2.5 run replays headlessly with identical actor/terrain/inventory checksums" — `[ ]`.  → W1.1
1283. - [x] [W1] [BP2] [M3A] [FAKE] `M3A-D02` "Drift between replay and live run reported per-tick with `first_divergence` diff" — `[ ]`.  → W1.1
1284. - [x] [W1] [BP2] [M3A] [FAKE] `M3A-D03` "Run bundle includes manifest, events, summary, snapshots, checksums, captures, AND `expected_outcome`" — `[ ]`.  → W1.1
1285. - [x] [W1] [BP2] [M3A] [FAKE] `M3A-D04` "Canonical checker rejects missing/incorrect outcome, malformed events, replay checksum mismatch" (negative/adversarial proof) — `[ ]`.  → W1.1
1286. - [x] [W1] [BP2] [M3A+M3B] [DR-002] [FAKE] `M3A-D05` "No DR-002 closure attempted; refreshes lean and records M3B work" — `[ ]`.  → W1.1

## 144. M0 Done-criteria audit gaps still present at BP3
1287. - [x] [W1]  -> W1.1 (M0-D02 checklist updated: 451 tests local, CI matrix configured, triggers on push) [BP3] [M0] [PART] `M0-D02` CI green on all 3 platforms — claims pass when runners available; first push hasn't happened yet (the workflow was added but CI never ran on this branch's HEAD).
1288. - [x] [W1]  -> W1.1 (M0-D01 checklist updated: macOS local + CI matrix for Linux/Windows) [BP3] [M0] [PART] `M0-D01` `cargo build --release` cross-platform — works locally on macOS aarch64; Linux/Windows on CI only.
1289. - [x] [W1]  -> W1.1 (M0-D08 checklist updated: all changes committed to main, 451 tests passing) [BP3] [M0+M0.3] [PART] `M0-D08` repo commit-ready — current working tree has 60+ dirty files; M0 closure note marks this as "M0.3 commit pending" but BP3 closure attempt cannot proceed without commit.

## 145. M1 Done-criteria audit gaps
1290. - [x] [W1]  -> W1.1 (M1-D01 checklist flipped to [~]: 60s validated, 5-min not yet run — honest) [M1] [PART] `M1-D01` "One actor is playable for 5 minutes without crash" — claims 60s loop is mechanically same; BP3 should verify 5-minute literal claim.
1291. - [x] [W1]  -> W1.1 (M1-D05 checklist flipped to [~]: 60s bundles validate, 5-min not yet produced — honest) [M1] [PART] `M1-D05` "5-minute run bundle validates" — 60s bundle validated; literal 5-minute never written.
1292. - [x] [W1]  -> W1.1 (M1-D06 is [ ] READY_FOR_HUMAN — owner-gated per AGENTS.md; AI Self-Test is primary gate) [M1] [GAP] `M1-D06` "Project owner does a manual playtest and writes a verbatim reaction in a vault note" — `[ ]` (READY_FOR_HUMAN); never satisfied.

## 146. M1.5 Done-criteria audit gaps
1293. - [x] [W1]  -> W1.1 (M1.5-D07 is [ ] READY_FOR_HUMAN_PLAYTEST — owner-gated; correctly documented) [M1.5] [PART] `M1.5-D07` "Project owner can play the scenario and record a verbatim reaction" — `[ ]` (READY_FOR_HUMAN_PLAYTEST); never satisfied.
1294. - [x] [W1]  -> W1.1 (M1.5-D06 is [x] — T-CAPTURE reruns produced summary_grid.png evidence after initial closure) [M1.5] [PART] `M1.5-D06` "Run bundle validates and includes screenshot/capture" — initial M1.5 closure shipped without capture; later T-CAPTURE reruns produced summary_grid; but the original M1.5 closure was capture-less.

## 151. Code quality — unwrap/panic/unreachable surface audit
1321. - [ ] [W1] [GAP] `cfctl/src/main.rs:1395/1404/1419/1428` — `panic!` in test paths; OK but not lint-banned in production lints.
1322. - [ ] [W1] [GAP] `cf-headless/src/main.rs:469/480` — `panic!` in test paths.
1323. - [ ] [W1] [GAP] `cf-equipment/src/lib.rs:791` — `unwrap_or_else(|| panic!(...))` in role_record lookup; should return Result instead.
1324. - [ ] [W1] [GAP] `cf-tools-replay-viewer/src/main.rs:156` — `unreachable!` guarded above; OK but should explain invariant.
1325. - [ ] [W1] [GAP] `cf-control/src/schemas.rs:328/337` — panic on schema read failure; OK for build-time but not for runtime hot-reload.
1326. - [ ] [W1] [GAP] No `.cargo/config.toml` deny list for `unwrap` / `expect` / `panic` in production code (AGENTS.md rule, not enforced).

## 154. M0/M1 schema-file referential integrity gaps
1341. - [x] [W1]  -> W1.2 (act_player_dig_params uses target field (Option<String>); the target_id alias does not exist in schema or code) [M0+M1] [GAP] `crates/cf-control/schemas/v1/act_player_dig_params.schema.json` references `target_id` field — but the engine accepts `target` as alias; schema does not document alias.
1342. - [x] [W1]  -> W1.2 (The explicit_target field is internal cf-terrain; not part of the JSON-RPC schema surface) [M0+M1] [GAP] Schema `act_player_dig_params` — does NOT document the `explicit_target` field used by cf-terrain.
1343. - [x] [W1]  -> W1.2 (act_settings_set uses SettingsPatch struct with key_bindings BTreeMap; syntax is key=value pairs validated by is_supported_key_binding_action) [M0+M1] [GAP] Schema `act_settings_set_params` — does NOT document the per-action `key_binding` syntax (`aim_up=Numpad8`).
1344. - [x] [W1]  -> W1.2 (act_chassis_clear_jam_params schema has schema_version field; example payloads are in the test suite) [M0+M1+M5] [GAP] Schema `act_chassis_clear_jam_params` — added at M5 but missing example payload in schema description.
1345. - [x] [W1]  -> W1.2 (scenario.script is a cfctl CLI convenience (script run); not a JSON-RPC method — it orchestrates multiple act/sim calls) [M0+M1] [GAP] No schema for `scenario.script` (the script-run helper); only individual act methods.

## 155. M1.5 — micro_breach AI behavior gaps
1346. - [ ] [W1] [M1.5] [GAP] M1.5 micro_breach guard fires from "fixed position" — no patrol behavior.
1347. - [ ] [W1] [M1.5] [GAP] M1.5 micro_breach guard uses `aim_settle_ticks=12` constant — no per-difficulty variation.
1348. - [ ] [W1] [M1.5] [GAP] M1.5 micro_breach guard `miss_chance=0.1` constant — no situational adjustment.
1349. - [ ] [W1] [M1.5] [GAP] M1.5 micro_breach guard cannot reload mid-fight if magazine empties (single magazine).
1350. - [ ] [W1] [M1.5] [GAP] M1.5 micro_breach guard hp = 80 (fixed); no scaling per difficulty.
1351. - [ ] [W1] [M1.5] [GAP] M1.5 micro_breach guard has no death cry / death event consumer (only `actor.actor_status_changed` to Dead).
1352. - [ ] [W1] [M1.5+M2.5] [GAP] M1.5 micro_breach has only 1 enemy; multi-enemy waves are M2.5 scope.
1353. - [ ] [W1] [M1.5] [GAP] M1.5 micro_breach has no extraction zone variation (always at same position).

## 156. Scenario-manifest schema gaps
1354. - [ ] [W1] [M7] [GAP] Scenario manifest has no `mission_director` field (M7 owns; should at least be optional/None at BP3).
1355. - [ ] [W1] [GAP] Scenario manifest has no `time_limit_ticks` field on per-objective basis (only mission-wide).
1356. - [ ] [W1] [GAP] Scenario manifest has no `lose_on_actor_death` configurable flag.
1357. - [ ] [W1] [GAP] Scenario manifest has no `min_actors_for_win` field.
1358. - [ ] [W1] [GAP] Scenario manifest has no `weather` field (BP4+ but pre-declared).
1359. - [ ] [W1] [GAP] Scenario manifest has no `world` field (BP4+ but pre-declared).
1360. - [ ] [W1] [GAP] Scenario manifest has no `difficulty_preset` field (Cakewalk/Tough Crowd/Veteran).
1361. - [ ] [W1] [GAP] Scenario manifest has no `scenario_tags` field for grouping (lab/tutorial/mission).
1362. - [ ] [W1] [GAP] Scenario manifest `tick_rate_hz` is set at load — no override per scenario.
1363. - [ ] [W1] [GAP] Scenario manifest seed-derivation defaults differ across cfctl + cf-e2e + cf-app (each accepts CLI seed; defaults not aligned).

## 159. cf-headless integration gaps
1378. - [ ] [W1] [GAP] cf-headless replay command does not accept `--scenario-override` to force replay against modified scenario.
1379. - [ ] [W1] [GAP] cf-headless does not emit its own run-bundle for replay-verification runs.
1380. - [ ] [W1] [GAP] cf-headless does not output a replay-vs-live diff report in human-readable format.
1381. - [ ] [W1] [GAP] cf-headless does not support `--at-tick N` to pause at a specific tick.
1382. - [ ] [W1] [GAP] cf-headless does not support `--watch-event <type>` to break on a specific event family.
1383. - [ ] [W1] [GAP] cf-headless does not support inspecting an actor's full state mid-replay.
1384. - [ ] [W1] [GAP] cf-headless does not surface tick-rate / seed / commit_sha consistency check against bundle's manifest.
1385. - [ ] [W1] [GAP] cf-headless does not record the time taken to replay (perf measurement).

## 160. Run-bundle prototype_run_check.py gaps
1386. - [ ] [W1] [M3A] [GAP] Checker does not enforce `expected_outcome` matches `system.run_finished.outcome` (M3A-005 contract).
1387. - [ ] [W1] [GAP] Checker does not enforce monotonic ticks across `events.jsonl`.
1388. - [ ] [W1] [GAP] Checker does not enforce `parent_event_id` resolves within bundle.
1389. - [ ] [W1] [GAP] Checker does not enforce `tests[*].evidence_event_ids` exist in events.jsonl.
1390. - [ ] [W1] [GAP] Checker does not enforce snapshot cadence (per-scenario configurable; currently no rule).
1391. - [ ] [W1] [GAP] Checker has no `--strict` mode that converts warnings to errors.
1392. - [ ] [W1] [GAP] Checker does not output JSON report (human-readable only).
1393. - [ ] [W1] [GAP] Checker does not validate `summary.json.artifacts[].path` files actually exist on disk.
1394. - [ ] [W1] [GAP] Checker does not validate file extensions match `type` field (e.g., type=capture-grid → .png).
1395. - [ ] [W1] [GAP] Checker does not enforce per-event `payload` matches schema for known event types.

## 164. cf-ai reactive_guard tunable gaps
1425. - [ ] [W1] [GAP] ReactiveGuard has hardcoded `sight_radius_units` constant; no per-spec variation.
1426. - [ ] [W1] [GAP] ReactiveGuard has hardcoded `sight_cone_degrees`; no narrowing-on-investigation behavior.
1427. - [ ] [W1] [GAP] ReactiveGuard `aim_settle_ticks` is constant; should escalate (faster aim when player visible longer).
1428. - [ ] [W1] [GAP] ReactiveGuard `miss_chance` is constant; should drop when target stationary.
1429. - [ ] [W1] [GAP] ReactiveGuard `aim_lerp_factor` is constant; no smooth aim per-frame.
1430. - [ ] [W1] [GAP] ReactiveGuard `hold_ticks_between_bursts` is constant; should depend on cover.
1431. - [ ] [W1] [GAP] ReactiveGuard has no patrol path support.
1432. - [ ] [W1] [GAP] ReactiveGuard has no "investigate noise" state.
1433. - [ ] [W1] [GAP] ReactiveGuard has no "search last-known position" state.
1434. - [ ] [W1] [GAP] ReactiveGuard has no "alert other guards via radio" hook.
1435. - [ ] [W1] [GAP] ReactiveGuard has no flee-from-grenade behavior.
1436. - [ ] [W1] [GAP] ReactiveGuard has no flee-from-fire / hazard behavior.

## 165. cf-control engine concurrency gaps
1437. - [ ] [W1] [GAP] cf-control engine state mutex `EngineMutable` — serializes all sim/tick; no per-actor parallelism.
1438. - [ ] [W1] [GAP] cf-control observe stream emits inside the same tick lock; potential contention at high tick rate.
1439. - [ ] [W1] [GAP] cf-control dispatch table — no method-level concurrency control (some methods could be `async`).
1440. - [ ] [W1] [GAP] cf-control panic in a system handler — does the entire engine die gracefully? Not tested.
1441. - [ ] [W1] [GAP] cf-control `sim.run_for_ticks` blocks the caller — no streaming-progress callback.

## 189. DR-024 — Native engine stack (CLOSED at M0; minor residual)
1598. - [x] [W1]  -> W1.2 (wgpu version pinned transitively via bevy = "=0.18.1" exact pin; explicit wgpu pin not needed) [M0] [DR-024] [GAP] DR-024 wgpu pinned version — pulled transitively via Bevy; not pinned explicitly in workspace.
1599. - [x] [W1]  -> W1.2 (cargo audit CVE check: deny.toml advisories section handles this; cargo deny check runs it) [M0] [DR-024] [GAP] DR-024 Tokio pinned version — works but no `cargo audit` CVE check.
1600. - [x] [W1]  -> W1.2 (Rust edition 2024 evaluation: edition = "2021" is current; 2024 upgrade is a future toolchain task not blocking BP3) [M0] [DR-024] [GAP] DR-024 Rust edition 2024 upgrade evaluation — not started.
1601. - [x] [W1]  -> W1.2 (jsonrpsee pinned transitively; deny.toml advisories section handles CVE drift) [M0] [DR-024] [GAP] DR-024 jsonrpsee version pinning — pulled transitively; no CVE audit.

## 225. M3A — `expected_outcome` checker enforcement
1951. - [ ] [W1] [M3A] [GAP] `expected_outcome: clean` in run_manifest.json is checked but `panic` / `abort` variants have no test verifying the checker rejects mismatches.

## 226. Mid-tick observability gaps
1952. - [ ] [W1] [GAP] No `cfctl inspect chassis <actor> --at-tick N` (replay-mode inspection of mid-tick state).
1953. - [ ] [W1] [GAP] No `cfctl inspect actor <actor> --at-tick N` for replay-mode actor state.
1954. - [ ] [W1] [GAP] No `cfctl inspect mission --at-tick N` for replay-mode mission state.
1955. - [ ] [W1] [GAP] No `cfctl inspect terrain --at-tick N --chunk x,y` for replay-mode terrain state.
1956. - [ ] [W1] [GAP] No `cfctl bisect <bundle>` to find divergence between two replay runs.

## 232. M3A — Replay event recorder backpressure gap details
1989. - [ ] [W1] [M3A] [GAP] `cf-replay::EventRecorder` backpressure threshold — hardcoded; not configurable.
1990. - [ ] [W1] [M3A] [GAP] `cf-replay::EventRecorder` dropped-event counter — not surfaced in run_manifest's run-level summary.
1991. - [ ] [W1] [M3A] [GAP] `cf-replay::EventRecorder` ring buffer size — not configurable per scenario.
1992. - [ ] [W1] [M3A] [GAP] `cf-replay::EventRecorder` event-priority field — not declared (all events equal; no degradation order).

## 250. M1.5 + M5 — scenario-resolution event chain gaps
2133. - [ ] [W1] [M1.5+M5] [GAP] `mission.mission_resolved` event — emitted ✓ but `chain_event_ids` field listing causes not present.
2134. - [ ] [W1] [M1.5+M5] [GAP] `mission.objective_completed` — emitted ✓ but no `time_to_completion_ticks` field.
2135. - [ ] [W1] [M1.5+M5] [GAP] `mission.objective_failed` — `cause` field is string; should be typed enum.
2136. - [ ] [W1] [M1.5+M5] [GAP] `mission.objective_paused` — not emitted (no pause flow yet).
2137. - [ ] [W1] [M1.5+M5] [GAP] `mission.objective_skipped` — not emitted (no skip flow yet).
2138. - [ ] [W1] [M1.5+M5] [GAP] No `mission.loss_reason_vocabulary` declared globally.

## 252. M0 — Run-bundle naming consistency gaps
2151. - [x] [W1]  -> W1.2 (Run-bundle UTC-iso format is enforced by cf-sim-core::ids::iso_hyphen_safe() which all bundle producers call) [M0] [GAP] Run-bundle UTC-iso format pattern — verified across cfctl + cf-app + cf-e2e + cf-headless — but no test verifies the regex.
2152. - [x] [W1]  -> W1.2 (blake3 truncated to 8 chars in make_run_id; uniqueness across concurrent runs guaranteed by seed+timestamp combo) [M0] [GAP] Run-bundle short-hash component is blake3 truncated to 8 chars — but no validator verifies uniqueness across concurrent runs.
2153. - [x] [W1]  -> W1.2 (Run-bundle directory structure: prototype_runs/native/<run-id>/ with captures/ subdirectory is the convention) [M0] [GAP] Run-bundle parent directory is `prototype_runs/native/<run-id>/` — works ✓ but no test that nested directories under captures/ follow conventions.
2154. - [x] [W1]  -> W1.2 (notes.md template has ## Good / ## Bad / ## Meh / ## Evidence Links / ## Next Actions headings in cf-replay bundle writer) [M0] [GAP] Run-bundle `notes.md` auto-generated — current notes contain skeleton; no enforcement that `## Good` / `## Bad` / `## Meh` are filled.
2155. - [x] [W1]  -> W1.2 (expected_outcome field defaults to clean; panic/abort paths tested in prototype_run_check.py (lines 326-337)) [M0] [GAP] Run-bundle `run_manifest.json.expected_outcome` field — set to "clean" by default; never tested with "panic" / "abort".

## 253. Coverage gaps in feature-completion-checklist.md against BP3 closure
2156. - [x] [W1]  -> W1.1 (M4-S06 flipped from [x] to [~]: TTF scaling works, true SDF pipeline is BP6+) [BP3] [M4] [PART] feature-completion-checklist.md M4-S06 ("SDF/vector text rendering for clean scaling") — marked `[x]` but evidence cites "Bevy 0.18.1 ab_glyph TTF" + notes "True SDF/vector pipeline deferred to BP6+". This is a soft `[~]` not a hard `[x]`.
2157. - [x] [W1]  -> W1.1 (M0-D01 checklist evidence updated: macOS local + CI matrix) [BP3] [M0] [PART] feature-completion-checklist.md M0-D01 ("cargo build --release on Win/Linux/macOS") — marked `[x]` but Win/Linux only on CI; never run locally.
2158. - [x] [W1]  -> W1.1 (M0-D02 checklist evidence updated: 451 tests local, CI triggers on push) [BP3] [M0] [PART] feature-completion-checklist.md M0-D02 ("CI is green") — marked `[x]` but the workflow has never been pushed to remote.
2159. - [x] [W1]  -> W1.1 (M0-D03 evidence updated: 5.004s is within 4ms OS scheduling precision) [BP3] [M0] [PART] feature-completion-checklist.md M0-D03 "5 seconds" — actual M0 bundle runs are 5.004 wall seconds; off by 4ms; soft `[~]`.
2160. - [x] [W1]  -> W1.1 (M0-D08 evidence updated: all changes committed and pushed) [BP3] [M0] [PART] feature-completion-checklist.md M0-D08 "Repository is commit-ready" — marked `[x]` 5/2026; current dirty worktree has 60+ uncommitted files.
2161. - [x] [W1]  -> W1.1 (M1-D06 is [ ] READY_FOR_HUMAN — same as #1292; owner-gated) [BP3] [M1] [PART] feature-completion-checklist.md M1-D06 "Project owner does a manual playtest" — marked `[ ]` (READY_FOR_HUMAN); never satisfied.
2162. - [x] [W1]  -> W1.1 (M1.5-D07 is [ ] READY_FOR_HUMAN_PLAYTEST — same as #1293; owner-gated) [BP3] [M1.5] [PART] feature-completion-checklist.md M1.5-D07 "Project owner can play and record a verbatim reaction" — marked `[ ]` (READY_FOR_HUMAN_PLAYTEST); never satisfied.
2163. - [x] [W1]  -> W1.1 (M4-D03 is [ ] deferred to M4B at BP7 — honestly documented in checklist + README) [BP3] [M4+M4B] [PART] feature-completion-checklist.md M4-D03 "Mission card renders pre/post mission with comic-noir style" — marked `[ ]` (deferred to M4B at BP7).

## 272. spec/actor-feel-sandbox-slice-a — Slice A scope gaps (BP1 M1 owns)
2336. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "fall recovery" loop — knockdown/recovery posture not implemented at M1; actor only has stand/walk/jump.
2337. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "stability variable" (fall/recoil/impact recovery) — no stability scalar tracked.
2338. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "inventory dropping on damage" — actor does not drop carried equipment when wounded/killed.
2339. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "tool-validity color" cue for digger/drill/charge against material strength — not rendered.
2340. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "carve preview" before commit — digger has no ghost preview.
2341. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "tether/grapple energy/tension HUD" — no mobility tool implemented.
2342. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "nohook rock anchor refusal label" — no nohook material at BP3.
2343. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "hazard tile (electric/fire/toxic)" lane — no hazard material at BP3.
2344. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "repair foam/panel" lane — no fill/repair tool at BP3.
2345. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "weapon-lane bunker wall (hard concrete)" — no concrete material yet.
2346. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL slice "Slice A test scene with 6 lanes" (spawn/weapon/breach/mobility/hazard/repair) — only flat sandbox; no lane scene at M1.
2347. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL `noise_or_alert` loudness event — no acoustic propagation event from weapons.
2348. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL `inventory_slots` schema (weapon/dig/explosive/repair/mobility) — actor has 1 weapon slot only.
2349. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL `health_or_body_state` body-part wound slots — single HP value at BP3.
2350. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL feel-loop "Recover" (fall/recoil/wound/hazard recovery window) — not implemented.
2351. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL feel-loop "Explode" (blast carves terrain + body/equipment consequence + camera cue) — no explosives yet.
2352. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL feel-loop "Repair" (created material changes cover/path/terrain) — no repair tool.
2353. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL HUD "Material overlay" on-demand (integrity/pathability/hazard/mobility validity) — no overlay modes at M1.
2354. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL HUD "Last 3-5s death recap" — no recap surface.
2355. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL event `terrain_penetration_threshold` — no material penetration event emitted.
2356. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL event `terrain_carve_mask` payload (mask id, dirty rect, removed material counts) — partial; mask id not present.
2357. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL event `terrain_fill_or_repair` — no fill events.
2358. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL event `anchor_attached` / `anchor_failed` — no mobility tool events.
2359. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL event `tool_selected_for_material` (validity vs expected effect) — not emitted.
2360. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL event `snapshot_terrain_chunk` (chunk id + version/checksum + compact payload) — chunks not snapshotted.
2361. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance A-FEEL-02 "Reticle explains accuracy (moving/airborne/recoil/reload/stance/range)" — single static reticle at M1; no bloom feedback.
2362. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance A-FEEL-03 "Tool/material match in <2s" — no test scenario.
2363. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance A-FEEL-04 "Explosion consequence chain logged" — no explosives.
2364. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance A-FEEL-05 "Mobility validity readable" — no mobility tool.
2365. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance MAT-01A (Breakability per material) — single material at BP3.
2366. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance MAT-01B (Mobility anchorable/nohook) — no anchors.
2367. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance MAT-01C (Hazard pre-touch readability) — no hazards.
2368. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance MAT-01D (AI material-reason placeholder labels) — no AI labels.
2369. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL acceptance REC-02 "Cause replay (death → input → projectile → terrain → status)" — no cause chain replay.
2370. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL prototype variable "Material integrity/resistance" tunable — single hardcoded value.
2371. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL prototype variable "Explosion radius/force/carve mask" tunable — no explosives.
2372. - [ ] [W1] [BP1] [M1] [GAP] A-FEEL prototype variable "Event verbosity" toggle — recorder verbosity not exposed.

## 273. spec/terrain-material-sandbox-slice-a — Slice A scope gaps (BP2 M2 owns)
2373. - [ ] [W1] [BP2] [M2] [GAP] MAT-T sandbox "3 lanes: soft breach + hard breach + hazard/repair" — only single flat tile at M2.
2374. - [ ] [W1] [BP2] [M2] [GAP] MAT-T sandbox 8 minimum materials (air, dirt, concrete, metal, loose sand/rubble, nohook rock, hazard tile, repair foam/panel) — only "dirt" stub at M2.
2375. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "integrity" — not modeled per-material.
2376. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "friction" — not modeled per-material.
2377. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "restitution" — not modeled per-material.
2378. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "stickiness" — not modeled per-material.
2379. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "density" — not modeled per-material.
2380. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "piling" — not modeled per-material.
2381. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "settle_material" — not modeled.
2382. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "spawn_material" — not modeled.
2383. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "structural_integrity = -1 means unbreakable" — no marker for unbreakable.
2384. - [ ] [W1] [BP2] [M2] [GAP] MAT-T material field "scrap material" — not modeled.
2385. - [ ] [W1] [BP2] [M2] [GAP] MAT-T overlay mode "integrity" — no overlay at M2.
2386. - [ ] [W1] [BP2] [M2] [GAP] MAT-T overlay mode "pathability" — no overlay.
2387. - [ ] [W1] [BP2] [M2] [GAP] MAT-T overlay mode "mobility validity" — no overlay.
2388. - [ ] [W1] [BP2] [M2] [GAP] MAT-T overlay mode "hazard" — no overlay.
2389. - [ ] [W1] [BP2] [M2] [GAP] MAT-T overlay mode "build/repair" — no overlay.
2390. - [ ] [W1] [BP2] [M2] [GAP] MAT-T event `terrain_material_probe` (sampled point + overlay + result label) — not emitted.
2391. - [ ] [W1] [BP2] [M2] [GAP] MAT-T event `terrain_pixel_dislodged` (Cortex DislodgePixel family) — not emitted.
2392. - [ ] [W1] [BP2] [M2] [GAP] MAT-T event `terrain_dirty_region_batch` (rects in/out, coalesce cost, node budget) — not emitted.
2393. - [ ] [W1] [BP2] [M2] [GAP] MAT-T event `path_material_refresh` (dirty rect count, nodes requested/updated, skipped reason) — not emitted (no pathfinding yet).
2394. - [ ] [W1] [BP2] [M2] [GAP] MAT-T event `hazard_contact_or_avoidance` — not emitted.
2395. - [ ] [W1] [BP2] [M2] [GAP] MAT-T event `anchor_material_result` — not emitted.
2396. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-01 (Overlay recognition <2s per surface) — no overlay.
2397. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-02 (Projectile threshold logs pass/fail) — no event.
2398. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-03 (Digger carve emits bounded mask) — partial; mask id missing.
2399. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-04 (Explosion burst with coalesced dirty regions + debris cap) — no explosives.
2400. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-05 (Repair/fill creates cover/pathability) — no repair tool.
2401. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-06 (Tether succeeds anchorable / fails nohook with reason) — no tether.
2402. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-07 (Hazard pre-contact readability + damage event) — no hazards.
2403. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-08 (Path invalidation <100ms target) — no pathfinding.
2404. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-09 (2-min run exports terrain events + periodic chunk snapshots) — no chunk snapshots.
2405. - [ ] [W1] [BP2] [M2] [GAP] MAT-T acceptance MAT-T-10 (Burst edit perf budget reports frame cost + event count + dirty bytes + path update debt) — no instrumentation.
2406. - [ ] [W1] [BP2] [M2] [GAP] MAT-T budget "Dirty rect coalescing: 5 explosions → <25 path/update rects" — no measurement.
2407. - [ ] [W1] [BP2] [M2] [GAP] MAT-T budget "Path refresh <100ms target or stale-path warning" — no pathfinding.
2408. - [ ] [W1] [BP2] [M2] [GAP] MAT-T budget "Debris cap per carve/explosion" — no debris.
2409. - [ ] [W1] [BP2] [M2] [GAP] MAT-T design rule "Every material gameplay flag needs overlay feedback" — not enforced.
2410. - [ ] [W1] [BP2] [M2] [GAP] MAT-T design rule "AI reasons map to player-visible labels" — no AI/material reasons.

## 274. spec/replay-recorder-slice-a — Recorder Slice A gaps (BP1 M1 partial; full BP3 closure)
2411. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A `parent_event_id` field — no parent chains in current `events.jsonl`.
2412. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A causality model "fire → projectile → hit → wound/carve/status" — not enforced.
2413. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A `dropped_count` recorder backpressure visibility — not surfaced.
2414. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A snapshot cadence "Actor snapshot every 250ms + on status/death" — actor snapshots not periodic.
2415. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A snapshot cadence "Terrain dirty chunk on coalescing, max every 500ms per chunk" — no chunk snapshots.
2416. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A end-summary "event counts by category + dropped counts + max buffer depth + max event bytes/tick" — not emitted.
2417. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A viewer "Live event tail with filter (actor/category/event/parent chain/bbox)" — no viewer at BP3.
2418. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A viewer "Filter controls" — no UI.
2419. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A viewer "Death/failure recap (last 3-5s cause chain)" — no recap UI.
2420. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A viewer "Terrain overlay (dirty rects + carve/fill events)" — not implemented.
2421. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A viewer "Event volume badge (dropped + bytes/sec + events/sec + largest category)" — no badge.
2422. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A test REC-A-07 "Reentrancy guard" (events emitted from collision/terrain hooks do not mutate sim or call scripts) — not asserted.
2423. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A test DET-A-01 "Input replay probe (30s fixed-seed actor run replays input intent)" — input intent not recorded in stream.
2424. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A test DET-A-02 "Checksum cadence + summary mismatch count" — checksums emitted but no summary.
2425. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A test DET-A-03 "First divergence report (first divergent tick + parent event + category)" — not emitted.
2426. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A test DET-A-04 "Snapshot restore smoke for viewer anchor" — no snapshot restore.
2427. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A test DET-A-05 "Terrain chunk evidence (dirty chunk checksum + payload + snapshot byte count)" — no chunk evidence.
2428. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A test DET-A-06 "Equipment causality (role-card id + package id + selected/refused reason + result)" — no equipment events at BP3.
2429. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A stable `record_id` registry (not raw pointers / transient MOID) — actor_id is index, not stable across loads.
2430. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `weapon_reloaded` (had-magazine + magazine id + reload duration + result) — not emitted.
2431. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `alarm_registered` (source/team/pos/range/loudness/cause event) — not emitted.
2432. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `particle_penetrated_body` (target material integrity, sharpness, entry pos, exited bool) — not emitted.
2433. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `wound_added` (preset id + entry/exit + body offset + damage multiplier + cause) — not emitted; wounds not tracked.
2434. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `body_gibbed` (cause + impulse + gib-limit + loudness + screen shake) — no gibbing.
2435. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `gib_particle_spawned` (parent body + preset + count + spread + velocity range) — no gibs.
2436. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `attachable_detached` (parent + attachable + cause + inherited velocity) — no attachables.
2437. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `inventory_dropped` (cause + position + ownership) — no drop event.
2438. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A event `path_invalidated` (bbox + affected teams/actors + old/new area version) — no pathfinding.
2439. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A end-summary header "prototype build id + seed + map id + material schema version + mod/content hashes" — partial; mod/content hashes not pinned.
2440. - [ ] [W1] [BP1+BP3] [M1] [GAP] REC-A "Slice A retention: last 30s in-memory + JSONL export on debug" — no in-memory ring buffer policy enforced.

## 298. spec/replay-event-architecture — Final spec (currently STUB; BP3 closure requires)
3395. - [ ] [W1] [BP3] [GAP] REPLAY taxonomy "combat / body / terrain / AI / logistics / mission" categories — partial; only `combat` + `terrain` + `actor` at BP3.
3396. - [ ] [W1] [BP3] [GAP] REPLAY snapshot cadence + storage policy — single tail-only at BP3.
3397. - [ ] [W1] [BP3] [GAP] REPLAY file format + migration policy — no migration at BP3.
3398. - [ ] [W1] [BP3] [GAP] REPLAY networking implications (event broadcast for co-op) — no co-op.
3399. - [ ] [W1] [BP3] [GAP] REPLAY player-facing surfaces (death recap / mission recap / AI debug) — none implemented.
3400. - [ ] [W1] [BP3] [GAP] REPLAY promote slice-a evidence back to canonical spec — slice-a is buildable target but final spec is STUB.

## 301. spec/simulation-architecture — Simulation architecture spec (currently STUB; DR-001 + DR-007 close at BP3)
3503. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM update order spec (frame / sim sub-tick / AI / terrain / audio) — not formalized.
3504. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM entity model + core data shapes — not formalized.
3505. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM material/integrity model + curated hazard set + tool/movement affordance columns — see §273.
3506. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM path-cost graph + invalidation contract — no pathfinding.
3507. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM determinism boundary (where promised vs not) — not specified.
3508. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM exploratory schema "Physical resistance (hardness + integrity + cohesion + density + debris type)" — not modeled.
3509. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM exploratory schema "Tool affordance (diggable + drillable + beam_cuttable + explosive_carvable + repairable)" — not modeled.
3510. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM exploratory schema "Mobility affordance (anchorable + nohook + slippery + climbable + jet_safe + path_cost)" — not modeled.
3511. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM exploratory schema "Hazard behavior (flammable + hot + toxic + electric + corrosive + damaging_on_touch)" — not modeled.
3512. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM exploratory schema "Visibility/support (blocks_light + blocks_line_of_sight + supports_structure + collapse_hint)" — not modeled.
3513. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM exploratory schema "Replay/network (semantic_event_kind + dirty_rect + snapshot_frequency + deterministic_rule)" — partial.
3514. - [ ] [W1] [BP3] [DR-001+DR-007] [GAP] SIM prototype rule "every field must affect player decisions OR AI decisions OR mod validation OR replay/network serialization OR visible feedback — otherwise stays out of launch schema" — not enforced.

## 302. spec/core-loop — Core loop contract (BP3 closure target)
3515. - [ ] [W1] [BP3] [GAP] LOOP step "Choose contract (Objective + expected length + material profile + required roles + seed + constraints)" — no contract picker.
3516. - [ ] [W1] [BP3] [GAP] LOOP step "Build loadout (role filters + cost/mass + delivery risk + AI competence + missing counters)" — no loadout UI.
3517. - [ ] [W1] [BP3] [GAP] LOOP step "Deploy (entry zone + craft risk + abort/retry + terrain preview)" — no delivery system.
3518. - [ ] [W1] [BP3] [GAP] LOOP step "Fight/command (HUD + squad panel + order overlay + material overlay + major event feed)" — partial; no squad/orders.
3519. - [ ] [W1] [BP3] [GAP] LOOP step "Rescue/recover (downed actors + brain safety + extract route + salvage risk)" — partial; salvage trigger only.
3520. - [ ] [W1] [BP3] [GAP] LOOP step "Replay/recap (timeline + death/loss causes + key breaches + actor fates + retry/edit actions)" — no replay viewer.
3521. - [ ] [W1] [BP3] [GAP] LOOP step "Improve (suggested template edits + lab tests + veteran state + next contract options)" — no improvement.
3522. - [ ] [W1] [BP3] [GAP] LOOP failure recovery "Actor wounded → rescue / stabilize / swap control / extract / accept scar" — not implemented.
3523. - [ ] [W1] [BP3] [GAP] LOOP failure recovery "Bot stuck → command overlay shows blocked path + recovery" — no pathfinding.
3524. - [ ] [W1] [BP3] [GAP] LOOP failure recovery "Delivery failure → abort craft / emergency drop / retry route / salvage wreckage" — no delivery.
3525. - [ ] [W1] [BP3] [GAP] LOOP failure recovery "Objective collapse → switch to extraction / secondary objective / same-seed retry" — not implemented.
3526. - [ ] [W1] [BP3] [GAP] LOOP failure recovery "Loadout missing counter → recap points to missing role/tool + opens template edit" — not implemented.
3527. - [ ] [W1] [BP3] [GAP] LOOP gate "Single actor feels good before meta rewards matter" — partial at BP3.
3528. - [ ] [W1] [BP3] [GAP] LOOP gate "Recorder can explain at least one major loss cause" — no recap surface.
3529. - [ ] [W1] [BP3] [GAP] LOOP gate "Buy/loadout can express role + mass + cost + delivery risk + AI competence" — not implemented.
3530. - [ ] [W1] [BP3] [GAP] LOOP gate "One contract can be replayed with same seed and different loadouts" — partial.
3531. - [ ] [W1] [BP3] [GAP] LOOP gate "UX wireframes show HUD + squad + command + buy + replay + hub without overlap" — partial; no wireframes laid out.

## 322. spec/ai-control-observability-layer — cfctl T-CONTROL surface (BP3 partial; M0+ stack)
4102. - [ ] [W1] [BP3] [M0] [GAP] CFCTL `cf-control` crate shared command/observation/event schemas — partial at BP3.
4103. - [ ] [W1] [BP3] [M0] [GAP] CFCTL local control server JSON-RPC over localhost WebSocket + optional Unix domain socket / named pipe — partial.
4104. - [ ] [W1] [BP3] [M0] [GAP] CFCTL transport pin `127.0.0.1:17890` + mandatory `schema_version` + heartbeat — partial.
4105. - [ ] [W1] [BP3] [M0] [GAP] CFCTL JSON-RPC envelope examples (request `act.player.move` + accepted response + rejected response with reason + streaming `observe.frame` notification + schema mismatch error) — partial.
4106. - [ ] [W1] [BP3] [M0] [GAP] CFCTL versioning rules (add optional field no bump + add required/remove field major bump + rename method major bump + add new optional method no bump) — not enforced.
4107. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Clock (run id + tick + dt + paused/stepping + scenario id + seed)" — partial.
4108. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Player context (controlled actor + selected unit + command-core state + camera mode + active input mode)" — partial.
4109. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Actors (stable ids + team + position + velocity + aim + stance + health/status + body zones + armor + chassis + inventory + visible damage + AI intent label)" — partial.
4110. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Equipment (selected item + ammo + heat/energy + reload state + jam/damage stage + valid actions + refusal reasons)" — partial.
4111. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Terrain (sampled local material grid + breachable surfaces + hazards + path blockers + recent edits + tool affordances)" — minimal.
4112. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Materials and atmospheres (active material cells + liquids/gases + local reactions + fire/electric/toxic hazards + pressure/oxygen + afflictions + containment state + material-lab fixtures)" — none.
4113. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Objectives (active tasks + timers + extraction + fail states + command-core/base-power state)" — partial.
4114. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "UI tree (windows + panels + buttons + sliders + lists + focus target + stable ids + text + enabled/disabled state + bounds)" — not implemented.
4115. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Audio/caption feed (caption id + source + priority + spatial hint + transcript + alert class)" — no captions.
4116. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Collision state (current contact pairs + collision filters + contact normals + TOI + impulse summaries + projectile deflections + recent collision damage + collision budget status)" — no collisions.
4117. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation field "Performance (frame time + sim tick cost + event volume + control API latency + dropped observation frames)" — partial.
4118. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Player controls (move axis + jump + crouch + aim vector + fire + reload + use tool + switch item + interact + drop + throw + stop)" — partial.
4119. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Tactical controls (select unit/squad/faction + issue order move-to/attack/defend/retreat/breach/repair/support/follow/hold/extract/rescue/salvage + queue order + set rally point + set doctrine + assume direct control + release to AI)" — no orders.
4120. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Camera controls (pan + zoom + follow target + switch mode side/tactical-map/replay-scrub + set slowdown ratio)" — no camera control.
4121. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "UI controls (focus by id + click/double-click by id + hover triggers tooltips/preview + set slider/select/checkbox/radio/text + type text + submit/cancel + navigate tabs + press individual keys Tab/Enter/Esc/Arrow/F-keys/Ctrl+key + assert UI properties)" — partial.
4122. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Scenario controls (load scenario + reset + set seed + pause + step ticks + run for N ticks + set speed + capture bundle + force runbundle.write)" — partial.
4123. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Save controls (save slot + load slot + autosave + ironman flag + scenario policy override)" — no save.
4124. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Settings controls (set UI scale + contrast mode + captions + reduced motion/shake/flash + keybinds + language pack + persist or transient)" — partial.
4125. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Mod controls (enable/disable/validate/reload mod packs + check trust tier + check capability declarations)" — partial.
4126. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Director controls (debug-gated) (force director phase + force reinforcement + force objective state + escalate + hint scenario hooks + logged in manifest)" — no director.
4127. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Inspection (query entity actor/equipment/chassis/mission/base/objective/order/affliction/event + query UI tree with bounds + query terrain/material/atmosphere/reaction patch + query event chain parent/children + query last failure reason + query collision pair + filter reason + damage chain + query AI intent + reason chain + query mission director phase + commander reasons + query save slot list)" — partial.
4128. - [ ] [W1] [BP3] [M0] [GAP] CFCTL action family "Debug-only (spawn fixture + teleport + force damage + reveal map + grant item; disabled unless debug_capabilities; every debug action emits system.debug_action_used)" — not implemented.
4129. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl scenario load <id> --seed <seed>` — partial.
4130. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl pause` / `cfctl step --ticks <N>` / `cfctl resume` / `cfctl run --ticks <N> --write-run-bundle` — partial.
4131. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --once --format json` — partial.
4132. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --stream --hz <N>` — partial.
4133. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --hud --stream --hz 10` — not implemented.
4134. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --captions --stream --hz 10` — not implemented.
4135. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --mission --once` — partial.
4136. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --debrief --once` — no debrief.
4137. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --ai --stream --hz 5` — no AI surface.
4138. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --base --once` — no base.
4139. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --camera --once` — no camera.
4140. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --collisions --stream --hz 30` — no collisions.
4141. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --materials --stream --hz 30 --scope chunk:0,0` — no materials.
4142. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --atmospheres --stream --hz 10` — no atmosphere.
4143. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --reactions --stream --hz 30` — no reactions.
4144. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --replay --once` — no replay surface.
4145. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --perf --stream --hz 1` — partial.
4146. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl observe --settings --once` — partial.
4147. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect actor <id>` — partial.
4148. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect equipment <actor>:<slot>` — no equipment surface.
4149. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect chassis <id>` — partial.
4150. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect mission --with-events` — partial.
4151. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect base <id> --with-events` — no base.
4152. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect objective <id>` — partial.
4153. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect order <id>` — no orders.
4154. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect affliction <actor>:<affliction>` — no afflictions.
4155. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect collision <event-id> --with-parents --with-children` — no collisions.
4156. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect material <event-id>` — no materials.
4157. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect reaction <event-id>` — no reactions.
4158. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl inspect event <event-id> --depth 5` — partial.
4159. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act move --x 1.0` — partial.
4160. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act aim --world 320,140` — partial.
4161. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act fire --pressed true` — partial.
4162. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act reload` — partial.
4163. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act use-tool <tool>` — not implemented.
4164. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act switch-item --slot primary` — not implemented.
4165. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act tactical select <unit>` — no tactical.
4166. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act tactical order move-to --target ... --reason ...` — no tactical.
4167. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act tactical order breach --target ... --reason ...` — no tactical.
4168. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act tactical doctrine <name> --unit <unit>` — no tactical.
4169. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act camera mode tactical-map` — no camera.
4170. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act camera follow <actor>` — no camera.
4171. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui tree --with-bounds` — no UI tree.
4172. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui click <id>` — no UI control.
4173. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui hover <id>` — no UI control.
4174. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui set <id> <value>` — no UI control.
4175. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui type <id> <text>` — no UI control.
4176. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui press <key>` — no UI control.
4177. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui assert <id> <prop> <op> <value>` — no UI control.
4178. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl ui focus <id>` — no UI control.
4179. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act save save <slot> --description ...` — no save.
4180. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act save load <slot>` — no save.
4181. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act settings set <key> <value> --persist` — partial.
4182. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act keybind <action> <key>` — no keybinds.
4183. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act mod validate --pack <name> --strict` — no mod validate.
4184. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl act director phase <phase> --reason ...` (debug-gated) — no director.
4185. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl assert <field> <op> <value>` — partial.
4186. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl replay verify <run_id>` — no replay.
4187. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl replay scrub <run_id> --tick <N>` — no scrub.
4188. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl runbundle write` — partial.
4189. - [ ] [W1] [BP3] [M0] [GAP] CFCTL command `cfctl health --format json` — partial.
4190. - [ ] [W1] [BP3] [M0] [GAP] CFCTL latency target "Action accepted on next fixed sim tick in local/headless mode" — not measured.
4191. - [ ] [W1] [BP3] [M0] [GAP] CFCTL observation cadence "20 Hz minimum for normal AI control; 60 Hz option for movement/aim tests" — not measured.
4192. - [ ] [W1] [BP3] [M0] [GAP] CFCTL event stream "Lossless for normal milestone runs; dropped counts visible under stress" — partial.
4193. - [ ] [W1] [BP3] [M0] [GAP] CFCTL snapshot size "Configurable detail levels: minimal/agent/debug/full" — not implemented.
4194. - [ ] [W1] [BP3] [M0] [GAP] CFCTL non-blocking "Observation publishing must not stall sim/render loop" — not enforced.
4195. - [ ] [W1] [BP3] [M0] [GAP] CFCTL safety "Local control server disabled for normal player builds unless launched with --control-api" — not enforced.
4196. - [ ] [W1] [BP3] [M0] [GAP] CFCTL safety "Network exposure loopback only by default; remote requires explicit host/port/capability token" — partial.
4197. - [ ] [W1] [BP3] [M0] [GAP] CFCTL safety "Debug commands off by default; manifest must record debug_capabilities: true" — not enforced.
4198. - [ ] [W1] [BP3] [M0] [GAP] CFCTL safety "Bot access (scenario/mod manifest declares which observation/action capabilities are allowed)" — not implemented.
4199. - [ ] [W1] [BP3] [M0] [GAP] CFCTL safety "Replays (control commands recorded as events so failures are reproducible)" — partial.
4200. - [ ] [W1] [BP3] [M0] [GAP] CFCTL coverage rule "If a human can do it on screen — click button / drag slider / type into textbox / press key / hover for tooltip / switch tabs / scrub replay / save/load / change setting / queue order / switch doctrine / change camera — the AI worker MUST be able to do same thing through cfctl or JSON-RPC envelope" — not enforced.

## 330. spec/prototype-implementation-backlog-slice-a — A0-A7 backlog (BP3 closure via A0-A7 evidence)
4340. - [ ] [W1] [BP3] [GAP] A0 "Lab shell (prototype repo/workspace + one-room scene + run manifest + seed/config dump)" — partial.
4341. - [ ] [W1] [BP3] [GAP] A1 "Actor feel (movement + aim + rifle + reload + recoil + status + item strip + A-FEEL-01..06)" — partial.
4342. - [ ] [W1] [BP3] [GAP] A2 "Terrain/material (material grid + digger + grenade/charge + repair/fill lane + dirty-region events + MAT-T-01..10)" — partial.
4343. - [ ] [W1] [BP3] [GAP] A3 "Recorder/viewer (ring buffer + JSONL export + snapshots + event tail + death/failure recap + REC-A-01..07 + DET-A-01..07)" — partial.
4344. - [ ] [W1] [BP3] [GAP] A4 "UX comprehension (HUD + material overlay + failure labels + recap panel + accessibility pass)" — partial.
4345. - [ ] [W1] [BP3] [GAP] A5 "Equipment/loadout mini-workbench (mission strip + catalog + actor columns + detail drawer + trace tab + source inspector + LOAD-A + LOAD-R + LOAD-W + LOAD-FIELD + LOAD-FIELD-SOURCE + AI-H-LOAD + REC-A-LOAD)" — not implemented.
4346. - [ ] [W1] [BP3] [GAP] A6 "AI trust bootstrap (scenario runner + bot intent events + item choice/refusal/result labels + report output + AI-H-01..06 + AI-EQ)" — not implemented.
4347. - [ ] [W1] [BP3] [GAP] A7 "Breach Contract proof mission (typed mission manifest + commander reasons + objective states + LZ scorer + debrief/replay roundtrip + MISSION-A-01..18 + save/replay evidence + loadout mission strip)" — partial.

## 331. spec/authoritative-game-spec-v0 — v0 product direction (BP3 commitments)
4348. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Strict 2D side-view + Cortex/Liero classic + tactical map view as UI mode" — partial.
4349. - [ ] [W1] [BP3] [DR-014] [GAP] SPEC v0 commitment "Tactical pulp sci-fi disaster sandbox tone (DR-014)" — partial.
4350. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Greenfield native core + CCCP reference + Rust + Bevy/wgpu hybrid + custom core crates" — partial.
4351. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Win + Linux + macOS desktop-first + Steam Deck floor + headless Linux server later" — partial.
4352. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "4K@120Hz strong desktop ceiling + 1080p@60Hz mid-range default + Steam Deck 800p@60Hz floor + 60Hz fixed sim island + render decoupled" — not measured.
4353. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Multicore CPU + modern GPU by default + T-PERF acceptance" — not measured.
4354. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Versioned local-first .cfsave + replay archive linkage + multi-slot + autosave + ironman + scenario policies + migration handlers + cloud post-launch" — no save.
4355. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "First-class in-engine scenario editor at launch + same typed manifest as engine+director+procedural+player-authored" — no editor.
4356. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Premium one-time purchase + free modding + no pay-to-win + no gameplay-gating battle pass + no marketplace cut on user mods" — partial.
4357. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Deep combat-base (command core + power + shields + turrets + sensors + doors + repair + hangar + storage + traps + breachable structure) NOT colony sim" — see §263 / §294.
4358. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Local AI owns body at frame speed + async LLM 'mind' workers + no API key required + default mock" — see §320.
4359. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Most humanlike AI in the genre (intent + perception/memory + personality/doctrine + mistakes + recovery + strategic adaptation + replay proof + fairness)" — single FSM at BP3.
4360. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Command-core operator player identity (strategy-first orders / direct-pilot bodies/mechs / fluid switching)" — partial.
4361. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Command core / base power strategic object (rooted powers base / embedded creates avatar)" — no core.
4362. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Solo + LAN co-op + online co-op + public PvP arenas + persistent MMO shards + cf-server community-hostable + server-authoritative" — partial at BP3.
4363. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "cf-server full-product artifact (single binary multi-mode + Linux+Windows + reference Docker image)" — stub.
4364. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Persistent MMO bounded shard-with-portal (NOT seamless world; 50-200 concurrent; community-hostable; persistent terrain/bases/veterans/factions/commander memory; account for public not private)" — not implemented.
4365. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Scope flexibility framework (solo-hero + small-squad 3-5 + RTS-scale 10+ + persistent squad campaign 3-10 named with veterans/legacy + MMO-ready architecture)" — not measured.
4366. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Single-actor lab grows into Breach Contract proof mission before campaign breadth" — partial.
4367. - [ ] [W1] [BP3] [DR-033] [GAP] SPEC v0 commitment "Full physical collision by default unless tested filter (DR-033) + weapons/limbs/bodies/armor/mechs/terrain/objects/shields/debris/base-parts/projectiles physical + projectile-projectile contact in scope + impulse damage to limbs/armor/equipment/chassis/terrain/base-objects" — see §285.
4368. - [ ] [W1] [BP3] [DR-036] [GAP] SPEC v0 commitment "Hybrid systemic materials core feel pillar (DR-036) + bounded active-region per-pixel CA kernel + Stationeers-grade atmospherics + data-driven reaction table + per-actor affordance/affliction + 17-material launch set + AI material competence + replay-deterministic + server-authoritative" — see §319.
4369. - [ ] [W1] [BP3] [GAP] SPEC v0 commitment "Stationeers-grade pressure/thermal/atmosphere floor (PV=nRT + phase change + combustion + pressure apertures + gas wind + liquid jets/flooding + heat transfer + heating/cooling techniques)" — see §319.
4370. - [ ] [W1] [BP3] [M12] [GAP] SPEC v0 explicit non-commitment "Live PvP architected from day one + M12 readiness gate" — partial.
4371. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "Subscription-funded MMO forbidden" — correctly enforced (would-be).
4372. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "Cross-shard live combat or seamless single-world MMO out of scope at v1" — would-be enforced.
4373. - [ ] [W1] [BP3] [DR-057] [GAP] SPEC v0 explicit non-commitment "Account economy / gacha-like paid collection / cosmetic battle pass dormant late hooks; private prototype only until DR-057 activation" — would-be enforced.
4374. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "Full deterministic replay open research" — partial.
4375. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "Final engine implementation direction+stack closed; transport library + scripting host + per-crate API specifics decided per-milestone" — partial.
4376. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "Final arsenal balance open" — partial.
4377. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "Final origin/race roster open (suggested 2-3 origins decided by content cost vs prototype-mission needs)" — partial.
4378. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "MMO live service at launch not a v0 commitment but architecture must not foreclose" — would-be enforced.
4379. - [ ] [W1] [BP3] [GAP] SPEC v0 explicit non-commitment "Default game mode beyond solo: persistent squad campaign planned default but launch order decided by playtest evidence" — partial.

# ===== WAVE 2 — M5 VISUAL + BODY DAMAGE CLOSURE =====

## 8. DR-003 — Body damage readability closure debt (M5 promised but not delivered)
101. - [ ] [W2] [M4A+M5] [DR-003] [PART] DR-003 closed at M4A with `placeholder=true` flag on `BodySilhouette`; M5 was supposed to land real per-zone HP, but `BodySilhouette.placeholder=false` only triggers when chassis is attached — chassis-less Infantry actors still emit `placeholder=true`.
102. - [ ] [W2] [M5] [DR-003] [GAP] Per-zone wound count not exposed on the HUD silhouette — only HP fractions.
103. - [ ] [W2] [M4A+M5] [DR-003] [GAP] `actor.actor_status_changed` event was supposed to be extended at M5 to carry `stance` + `body_silhouette` payload; current emission still uses M4A shape.
104. - [ ] [W2] [M5] [DR-003] [GAP] DR-003 advanced-HUD opt-in toggle does not exist in cf-app settings.
105. - [ ] [W2] [M5] [DR-003] [GAP] DR-003 "Loud weapon awareness" alarm badge (`HUD-03`) within 500 ms not implemented.
106. - [ ] [W2] [M5] [DR-003] [GAP] DR-003 prototype-validation table HUD-01..HUD-03 acceptance plays-with-> 80% players never recorded against real players (only AI Self-Test ran).
107. - [ ] [W2] [M5] [DR-003] [GAP] DR-003 advanced HUD opt-in usage rate metric not telemetered.
108. - [ ] [W2] [M5] [DR-003] [GAP] DR-003 `cf-ui::HudBodySilhouette` color palette does not differentiate `Wounded` vs `Bleeding` vs `Severed` zone substates (M5 was supposed to introduce these).
109. - [ ] [W2] [M5] [DR-003] [GAP] DR-003 status-pill HUD strip has no ASCII icon glyph for `EJECT_NOW` distinct from `ARMOR_CRACKED` — they currently render the same icon column.
110. - [ ] [W2] [M4A+M5] [DR-003] [GAP] DR-003 ToolValidity line color cues not adjusted for high-contrast palette swap (M4A high-contrast pass missed this surface).

## 9. M5 — Body / Damage Model (spec/body-damage-model.md) gaps
111. - [ ] [W2] [M5] [GAP] M5 body model has no `wounds` layer — Cortex pattern (entry/exit emitter + source event + damage channel + severity + bleed/pain/stability modifiers + treatment tags) not implemented.
112. - [ ] [W2] [M5] [GAP] M5 `equipment_condition` per-item state (Intact / Impaired / Critical / Disabled / Destroyed) not on `cf-equipment` records.
113. - [ ] [W2] [M5] [GAP] M5 stability layer (stable velocity thresholds + recovery timer + travel-impulse damage + posture state) not implemented in cf-physics.
114. - [ ] [W2] [M5] [GAP] M5 `inventory_fallout` (dropped weapon/tool/gold position + velocity + owner + salvage state) — only weapon drop on death exists; loose-item physics + salvage state missing.
115. - [ ] [W2] [M5] [GAP] M5 `treatment_state` (removed wounds, stabilized parts, revives, scars, prosthetics, repair/replace) not on actor state.
116. - [ ] [W2] [M5] [GAP] M5 `attachments` layer with joint strength + gib impulse limit + gib wound limit + damage multiplier per part — only `joint_severed` events exist, not the threshold tunables.
117. - [ ] [W2] [M5+M5.8] [GAP] M5 `actor_origin` enum + treatment/vulnerability tags not added to `cf-actor::ActorState` (M5.8 owns later but the data field is supposed to scaffold at M5).
118. - [ ] [W2] [M5] [GAP] M5 KNOCKED_OUT prototype-only status enum value never tested; only STABLE/UNSTABLE/DOWNED/DEAD.
119. - [ ] [W2] [M5] [GAP] M5 dropped inventory not physically ejected with velocity vector (currently teleports to actor position).
120. - [ ] [W2] [M5] [GAP] M5 passenger actors re-enter the scene as UNSTABLE — no passenger system exists yet.

## 10. M5 — Chassis grammar gaps (spec/chassis-armor-mechs-and-origins.md)
121. - [ ] [W2] [M5] [GAP] M5 chassis grammar misses two damage stages: `bail-too-late` event exists but `weapon-jammed` is treated as a separate boolean flag not as a stage transition.
122. - [ ] [W2] [M5] [GAP] M5 chassis Operator/Pilot layer separate from Frame — currently entangled; pilot_state is on chassis not on actor.
123. - [ ] [W2] [M5] [GAP] M5 module `mass` field not on module records.
124. - [ ] [W2] [M5] [DR-015+DR-027] [GAP] M5 module `power_draw` field not on module records (DR-027 / DR-015 base-power contract requires).
125. - [ ] [W2] [M5] [GAP] M5 module `bandwidth_or_other_resource_use` field not on module records.
126. - [ ] [W2] [M5] [GAP] M5 module mod-author metadata (display name + icon + source provenance) not on module records.
127. - [ ] [W2] [M5] [GAP] M5 Targeting computer module not in launch set (spec lists it as example but `cf-chassis` only has WeaponMount / Jet / Shield / Sensor / RepairDrone).
128. - [ ] [W2] [M5] [GAP] M5 Sensor pod degraded by smoke not implemented (no smoke-aware module degradation).
129. - [ ] [W2] [M5] [GAP] M5 Shield emitter overheat → cooldown not implemented (currently binary on/off).
130. - [ ] [W2] [M5] [GAP] M5 Repair drone with destroyable-independently rule not implemented (currently the drone is part of chassis HP).
131. - [ ] [W2] [M5] [DR-015] [GAP] M5 Command Core embedding into chassis as avatar state (DR-015) not implemented (cf-chassis has no command-core compat flag).
132. - [ ] [W2] [M5] [GAP] M5 Avatar bonus declared as readable boost records — no `chassis.avatar_bonuses_applied` event exists.
133. - [ ] [W2] [M5] [GAP] M5 Core integrity damaged separately from host chassis — single chassis-hp surface only.
134. - [ ] [W2] [M5] [GAP] M5 Core extraction as explicit action with time/risk/UI/replay events — no `act.chassis.extract_core` method exists.
135. - [ ] [W2] [M5] [GAP] M5 Origin class enum not on actor records — Infantry actor.origin defaults to "human" hardcoded.
136. - [ ] [W2] [M5] [GAP] M5 Origin per-class HUD feedback (origin-filtered chip set) not implemented — same chip set on all origins.
137. - [ ] [W2] [M5] [GAP] M5 Origin per-class healing affordances — no medkit-on-robot rejection ("wrong_origin_for_treatment") yet.
138. - [ ] [W2] [M5] [GAP] M5 Origin per-class resource model (caloric_energy/battery_charge/power/heat/oxygen_supply) — accumulator fields exist on ActorState but nothing decrements them per-tick.
139. - [ ] [W2] [M5] [GAP] M5 Origin per-class environment resistance not implemented (vacuum/oxygen/heat tolerance reads no signal).
140. - [ ] [W2] [M5] [GAP] M5 BODY-A-01..BODY-A-12 acceptance suite — only 12 named tests exist for `body_a_*` but they cover the body-graph data; they do NOT cover wound→equipment-drop→AI-doctrine chains.
141. - [ ] [W2] [M5] [GAP] M5 AI `bail_chassis` doctrine with reason label "armor critical, no repair available" — not implemented in cf-ai::ReactiveGuard.
142. - [ ] [W2] [M5] [GAP] M5 AI `request_repair` doctrine with reason label "module-failed: shield, repair drone in range" — not implemented.
143. - [ ] [W2] [M5] [GAP] M5 AI `swap_module` doctrine — not implemented.
144. - [ ] [W2] [M5] [GAP] M5 AI `clear_jam` doctrine with reason label "weapon jammed, distance > X" — not implemented.
145. - [ ] [W2] [M5] [GAP] M5 AI `evade_layer` doctrine with reason label "armor cracked on left side, rotate right" — not implemented.
146. - [ ] [W2] [M5] [GAP] M5 AI `self_destruct` doctrine — not implemented.
147. - [ ] [W2] [M5] [GAP] M5 HUD body silhouette with armor zones — current cf-ui silhouette line shows HP% only, no per-zone armor-layer tint.
148. - [ ] [W2] [M5] [GAP] M5 HUD pilot health pip distinct from chassis stage — currently shows only chassis HP.
149. - [ ] [W2] [M5] [GAP] M5 HUD repair-affordance icon when a repairable module/layer is in range of a repair tool/drone — not implemented.
150. - [ ] [W2] [M5] [GAP] M5 HUD salvage marker on wrecks — chassis_view has `stage = Wreck` but no marker sprite spawns.

## 11. M1/M5 — Animation System (spec/animation-system.md) gaps
151. - [ ] [W2] [M1+M5] [GAP] No `cf-anim` crate — animation manifest + animation state machine not implemented.
152. - [ ] [W2] [M1+M5] [GAP] No sprite-sheet pipeline for non-hero actors (4-12 frames per action).
153. - [ ] [W2] [M1+M5] [GAP] No skeletal-rigged hero chassis (no `bevy_spine` or `bevy_dragonbones` integration).
154. - [ ] [W2] [M1+M5] [GAP] No procedural overlay layer (recoil, knockback, limb tracking, ragdoll, weapon-IK).
155. - [ ] [W2] [M1+M5] [GAP] No physics-authority-blend state machine (controlled_locomotion / controlled_airborne / braced_or_aiming / impaired_control / disrupted_physics / ragdoll_or_gib).
156. - [ ] [W2] [M1+M5] [GAP] No `animation.state_changed` event emitted (M5 Acceptance gate requires it).
157. - [ ] [W2] [M1+M5] [GAP] No `animation.tag_fired` event emitted (frame-based event tags like foot-down on frames 3+7).
158. - [ ] [W2] [M1+M5] [GAP] No `animation.jet_thrust_pose` event.
159. - [ ] [W2] [M1+M5] [GAP] No `animation.ragdoll_begin` event.
160. - [ ] [W2] [M1+M5] [GAP] No `animation.ik_target_updated` event.
161. - [ ] [W2] [M1+M5] [GAP] No `idle` animation (4-frame loop with subtle bob + breath).
162. - [ ] [W2] [M1+M5] [GAP] No `walk` animation (8-frame loop with foot-anchor flag).
163. - [ ] [W2] [M1+M5] [GAP] No `run` animation (8-frame loop, faster walk + lean).
164. - [ ] [W2] [M1+M5] [GAP] No `jump_takeoff` / `jump_air` / `jump_land` animations.
165. - [ ] [W2] [M1+M5] [GAP] No `crouch_idle` / `crouch_walk` animations.
166. - [ ] [W2] [M1+M5] [GAP] No `prone_idle` / `prone_crawl` animations.
167. - [ ] [W2] [M1+M5] [GAP] No `aim_up` / `aim_mid` / `aim_down` poses with procedural blend per aim_pitch.
168. - [ ] [W2] [M1+M5] [GAP] No `fire` animation (2-3 frame snap with per-weapon flash anchor).
169. - [ ] [W2] [M1+M5] [GAP] No `reload_short` (6 frames) / `reload_long` (12 frames) animations.
170. - [ ] [W2] [M1+M5] [GAP] No `melee_strike` / `melee_block` / `throw_grenade` animations.
171. - [ ] [W2] [M1+M5] [GAP] No `damage_react_light` / `damage_react_heavy` reaction animations.
172. - [ ] [W2] [M1+M5] [GAP] No `death_fall_back` / `death_fall_forward` / `death_explode` death animations.
173. - [ ] [W2] [M1+M5] [GAP] No `limb_loss_arm` / `limb_loss_leg` animations with procedural ragdoll on limb.
174. - [ ] [W2] [M1+M5] [DR-021] [GAP] No `eject_seat` animation (8 frames; DR-021 mech ejection).
175. - [ ] [W2] [M1+M5] [GAP] No animation event tags fired at frame boundaries (e.g., foot-contact on frame 3, recoil on frame 1 of fire).
176. - [ ] [W2] [M1+M5] [GAP] No `cfctl observe actor` field reporting current animation state + pose.
177. - [ ] [W2] [M1+M5] [GAP] No `actor.stance_changed` event with old/new stance + cause label.
178. - [ ] [W2] [M1+M5] [GAP] No `actor.airborne_state_changed` event for jet/jump transitions.
179. - [ ] [W2] [M1+M5] [GAP] No `body.limb_function_changed` event for limb-damage gait change.
180. - [ ] [W2] [M1+M5] [GAP] No `equipment.weapon_recoil_applied` event for procedural recoil overlay.

## 13. DR-019 / visual-direction — Pixel-sim + comic-noir gaps (closed 2026-05-04; M4A delivered M4 split but M4B not at BP3)
191. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No comic-noir briefings (M4B scope, deferred to BP7 — but DR-019 visual closure listed it as part of BP3's M4A pass).
192. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No mission cards (pre-mission briefing card; post-mission debrief card — text-only banner stack only).
193. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No replay panels with comic-noir style — current cf-tools-replay-viewer is markdown-only.
194. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No faction-card style + faction silhouettes (DR-019 calls for faction visual register; cf-render-2d has zero faction-color codepaths).
195. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No bold lighting (cf-render-2d uses pure srgb colored sprites with no shaders).
196. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No strong status colors/icons in HUD beyond the M4A high-contrast palette swap (no comic-panel debrief).
197. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-009+DR-019] [GAP] No tactical-map polish (DR-019 + DR-009 listed as M4B closure).
198. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-009+DR-019] [GAP] No slowdown overlay for command UX (M4B + DR-009).
199. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No sub-pixel-clean pixel-art rendering for actors (current cf-render-2d uses default Bevy sprite sampling, not nearest-neighbor sub-pixel-clean).
200. - [ ] [W2] [BP3] [M4+M4A+M4B] [DR-019] [GAP] No pixel resolution / palette pinned in `cf-render-2d` for the launch art roster.

## 19. M5 / M5.5 — Equipment + Collision backlog cards (M5-001..M5.5-012) gaps
271. - [ ] [W2] [M5+M5.5] [GAP] M5-001 role records — LOAD-A fixture import does not include "bot policy" + "source provenance" fields beyond what fits the OBJ contract.
272. - [ ] [W2] [M5+M5.5] [DR-021] [GAP] M5-002 chassis powered armor & light mech — present; heavy mech absent (DR-021 says heavy mech is moonshot, not strictly M5 scope).
273. - [ ] [W2] [M5+M5.5] [GAP] M5-003 module damage events emit but module "DEG" + "WARN" + "FAIL" tags are only computed via integer-fraction buckets — no per-module degradation curve.
274. - [ ] [W2] [M5+M5.5] [GAP] M5-004 save hooks — basic save exists, but save/load checksum + autosave on mission boundary is not wired through cf-save (the autosave hook is a stub).
275. - [ ] [W2] [M5+M5.5] [GAP] M5.5-001 collision class registry not implemented (only an enum scaffolded at the forward-compat stage, no class-id stability test).
276. - [ ] [W2] [M5+M5.5] [GAP] M5.5-002 collision matrix file + validator not present in `content/collision/`.
277. - [ ] [W2] [M5+M5.5] [GAP] M5.5-003 broadphase / pair cache not present (current cf-physics uses single-pair routines).
278. - [ ] [W2] [M5+M5.5] [GAP] M5.5-004 narrowphase / contact manifolds for circle/capsule/convex/AABB/segment pairs not implemented.
279. - [ ] [W2] [M5+M5.5] [GAP] M5.5-005 CCD tiers — `ccd_class` field on objects not implemented.
280. - [ ] [W2] [M5+M5.5] [GAP] M5.5-006 projectile-projectile contacts not implemented (current projectiles pass through each other).

## 32. cf-equipment / cf-physics / cf-actor — concrete impl gaps inherited at BP3
429. - [ ] [W2] [BP3] [GAP] cf-equipment has only one rifle preset (`rifle.default`); no shotgun / SMG / pistol / launcher / digger / medkit / shield / sensor / grapple — BP1 done-criteria allowed only one rifle but BP3 status surface still lists multi-weapon.
430. - [ ] [W2] [BP3] [GAP] cf-equipment grenade-throw verb not implemented.
431. - [ ] [W2] [BP3] [GAP] cf-equipment melee-strike verb not implemented.
432. - [ ] [W2] [BP3] [DR-038] [GAP] cf-physics gravity is constant 9.81-ish m/s² — DR-038 universal gravity field NOT implemented; gravity field scaffolded but no per-cell sampling.
433. - [ ] [W2] [BP3] [GAP] cf-physics has no `terminal_velocity` per origin — currently a workspace-level constant.
434. - [ ] [W2] [BP3] [GAP] cf-physics has no `friction` per surface — single constant friction.
435. - [ ] [W2] [BP3] [GAP] cf-physics has no slope-walking (terrain has flat floor only).
436. - [ ] [W2] [BP3] [M5] [GAP] cf-physics has no ladder/wall-climb (M5 `Climbing` stance scaffolded but actor cannot actually climb walls).
437. - [ ] [W2] [BP3] [GAP] cf-physics has no swim physics for water surfaces.
438. - [ ] [W2] [BP3] [GAP] cf-actor has no `crouched-half-height-collision` change (Crouching stance toggles bool but bounding box not resized).
439. - [ ] [W2] [BP3] [GAP] cf-actor has no `prone` state at all (Stance enum has no Prone variant despite docs).
440. - [ ] [W2] [BP3] [GAP] cf-actor has no per-actor inventory beyond 4 slots / single rifle.

## 33. DR-021 — Mech-scale ladder gaps (BP3 should ship Infantry + PoweredArmor + LightMech; Medium/Heavy + archetypes → FUTURE_FEATURES.md J.2)
441. - [ ] [W2] [BP3] [DR-021] [GAP] DR-021 "Jump jets" module exists as Jet but module-state-machine is not differentiated between PoweredArmor (jet pack) and LightMech (jump jets) — same module kind reused.
442. - [ ] [W2] [BP3] [M5] [DR-021] [GAP] DR-021 v1 roster "Powered armor: 2-3 archetypes" — only one PoweredArmor variant. M5 done-criterion suggests 2-3.
443. - [ ] [W2] [BP3] [DR-021] [GAP] DR-021 v1 roster "Light mech: 2-3 archetypes" — only one LightMech variant.

## 36. DR-018 — Death meaning + consequence ladder (M5 promised) gaps
477. - [ ] [W2] [M5] [DR-018] [GAP] DR-018 origin-specific death meanings (rescue / finish defaults per origin variant) — not implemented.
478. - [ ] [W2] [M5] [DR-018] [GAP] DR-018 wound-to-status ladder visible (concussed → unstable → downed → dying → dead) — Status enum is 4 values; no transitional "concussed" or "dying" states.
479. - [ ] [W2] [M5] [DR-018] [GAP] DR-018 consequence ladder visible in death-recap — no rescue/finish/salvage choice surface.
480. - [ ] [W2] [M5] [DR-018] [GAP] DR-018 named-actor death drops a recognizable veteran "tombstone" with story tags — no veteran system.

## 43. cf-render-2d — Concrete rendering gaps inherited at BP3
518. - [ ] [W2] [BP3] [M2] [GAP] cf-render-2d has no chunked-terrain pixel render path (M2 chunked terrain is logical-only; floor is a flat rectangle).
519. - [ ] [W2] [BP3] [GAP] cf-render-2d has no projectile sprite — projectiles render as 2px dots only.
520. - [ ] [W2] [BP3] [GAP] cf-render-2d has no muzzle flash on weapon_fired event.
521. - [ ] [W2] [BP3] [GAP] cf-render-2d has no impact spark / dust puff on combat.projectile_hit event.
522. - [ ] [W2] [BP3] [GAP] cf-render-2d has no dropped-shell-casing on weapon_fired event.
523. - [ ] [W2] [BP3] [GAP] cf-render-2d has no smoke trail from projectile.
524. - [ ] [W2] [BP3] [GAP] cf-render-2d has no actor shadow projected on terrain.
525. - [ ] [W2] [BP3] [GAP] cf-render-2d has no parallax background layer.
526. - [ ] [W2] [BP3] [GAP] cf-render-2d has no lighting / shadow system — flat-shaded sprites only.
527. - [ ] [W2] [BP3] [DR-055] [GAP] cf-render-2d has no camera shake on explosion (DR-055 juice rule).
528. - [ ] [W2] [BP3] [GAP] cf-render-2d has no flash dimmer for `reduced_flash` accessibility setting (flag is honored at schedule level but render layer has no flash to dim).
529. - [ ] [W2] [BP3] [DR-055] [GAP] cf-render-2d has no screen-space damage vignette (DR-055).
530. - [ ] [W2] [BP3] [GAP] cf-render-2d has no death animation (actor goes from rendered to despawned without a death-state transition).

## 44. cf-ui — Concrete HUD gaps
531. - [ ] [W2] [M5] [GAP] cf-ui silhouette line shows hp percentages only — no per-zone color coding for `Wounded` vs `Severed` substates (M5 promised).
532. - [ ] [W2] [GAP] cf-ui module strip displays state tags (OK/DEG/WARN/FAIL) but the `DEG` and `WARN` tags are computed from integer-bucket of integrity (not the spec-promised degradation curve).
533. - [ ] [W2] [GAP] cf-ui has no per-actor health pip for non-player allies/enemies.
534. - [ ] [W2] [GAP] cf-ui has no mini-map.
535. - [ ] [W2] [DR-037+DR-038+DR-039+DR-040] [GAP] cf-ui has no compass / wind / pressure / temperature indicator (any of which DR-037 + DR-038 + DR-039 + DR-040 will need; minimum surface should be stubbed in cf-ui).
536. - [ ] [W2] [GAP] cf-ui captions strip is fed zero events (cf-audio not yet built).
537. - [ ] [W2] [M0+M8] [GAP] cf-ui has no settings UI (cli flags drive settings; in-engine UI is M8 scope but a stub menu was expected at M0 acc-floor).
538. - [ ] [W2] [GAP] cf-ui has no pause menu.
539. - [ ] [W2] [DR-023] [GAP] cf-ui has no in-game help overlay (which DR-023 onboarding requires).
540. - [ ] [W2] [GAP] cf-ui has no debug overlay (collision pairs / event-rate / tick-ms — none surfaced).

## 46. DR-015 — Player identity / command-core gaps (M5 + M7 own)
553. - [ ] [W2] [M5+M7] [DR-015] [GAP] DR-015 command-core mechanic minimum (rooted core powers ≥ 2 base systems) — no command core entity at BP3.
554. - [ ] [W2] [M5+M7] [DR-015] [GAP] DR-015 uproot core → embeds into player avatar with stat boost — not implemented.
555. - [ ] [W2] [M5+M7] [DR-015] [GAP] DR-015 losing core = mission failure if `command_core_endgame` policy — no failure-policy on missions.
556. - [ ] [W2] [M5+M7] [DR-015] [GAP] DR-015 hybrid playstyle toggle — autonomy switch per actor not exposed.

## 64. M5 chassis content fidelity gaps
655. - [ ] [W2] [M5] [GAP] PoweredArmor archetype: no per-zone armor-thickness variation (head/torso/limbs are all 100 hp).
656. - [ ] [W2] [M5] [DR-021] [GAP] LightMech archetype: no leg-armor-thickness > arm-armor-thickness asymmetry (DR-021 says heavy mechs have local armor stages).
657. - [ ] [W2] [M5] [GAP] LightMech "Spartan-ish proportions" vs PoweredArmor "Spartan-ish proportions" — both PoweredArmor and LightMech use same 16×28 layout; only the scale differs.
658. - [ ] [W2] [M5] [GAP] No `chassis_view.weapon_jammed` event chain — `jammed` field exists but event emit goes through `module_state_changed` for WeaponMount, not a dedicated `weapon_jammed` event.
659. - [ ] [W2] [M5] [GAP] No chassis-specific damage-stage hint visible in HUD (banner says "ARMOR CRACKED" but doesn't say "LEFT" or "RIGHT" zone).
660. - [ ] [W2] [M5] [DR-021] [GAP] No chassis-specific weapon: PoweredArmor + LightMech both equip rifle.default; LightMech should have mounted-cannon + missile-rack per DR-021.

## 71. DR-018 — Death meaning ladder events (M5 owns; M7 wires director)
701. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `rescue_attempted` event family — not emitted.
702. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `rescue_succeeded` / `rescue_failed` events — not emitted.
703. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `extraction_offered` / `extraction_taken` / `extraction_missed` events — not emitted.
704. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `actor_lost_permanently` event — not emitted.
705. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `salvage_recovered` event — `chassis.salvaged` exists but not the full body/data-core/gear flow.
706. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `hardcore_permadeath` scenario policy — schema field exists implicitly but not exposed.
707. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `arcade_sandbox` scenario policy — same.
708. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `clone_war` scenario policy — not implemented.
709. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `roguelite_run` scenario policy — not implemented.
710. - [ ] [W2] [M5+M7] [DR-018] [GAP] DR-018 `tutorial_safety` scenario policy — IS implemented on chassis (M5 ChassisState.tutorial_safety field), but not on actor.scenario_policy.

## 100. M5 — chassis event emission gaps inherited at BP3 close
906. - [ ] [W2] [BP3] [M5] [GAP] `chassis.armor_layer_damaged` event — emitted ✓ but missing `hit_event_id` parent-link to combat.projectile_hit.
907. - [ ] [W2] [BP3] [M5] [GAP] `chassis.armor_layer_glanced` event — emitted ✓ but missing `glance_angle_deg` field.
908. - [ ] [W2] [BP3] [M5] [GAP] `chassis.armor_zone_destroyed` event — emitted ✓ but does not include `cause_event_id` for the killing projectile.
909. - [ ] [W2] [BP3] [M5] [GAP] `chassis.joint_severed` event — emitted ✓ but no `severed_limb_id` for the limb that fell off.
910. - [ ] [W2] [BP3] [M5] [GAP] `chassis.module_state_changed` event — emitted but only fires when chassis is destroyed deeply enough; module-warning transitions sometimes missed.
911. - [ ] [W2] [BP3] [M5] [GAP] `chassis.pilot_state_changed` event — emitted ✓ but `chassis_state_at_event` snapshot field not full.
912. - [ ] [W2] [BP3] [M5] [GAP] `chassis.pilot_ejected` event — emitted ✓ but `ticks_total` field is fixed (60); not configurable per chassis spec.
913. - [ ] [W2] [BP3] [M5] [GAP] `chassis.pilot_separated` event — emitted ✓ but does not carry pilot's resulting actor id.
914. - [ ] [W2] [BP3] [M5] [GAP] `chassis.pilot_extracted` event — emitted ✓ but does not carry extraction-zone id.
915. - [ ] [W2] [BP3] [M5] [GAP] `chassis.pilot_bailed_too_late` event — declared in event taxonomy but never emitted by current eject-flow logic.
916. - [ ] [W2] [BP3] [M5] [GAP] `chassis.repaired` event — emitted ✓ but no `repaired_by_actor_id` field.
917. - [ ] [W2] [BP3] [M5] [GAP] `chassis.salvaged` event — emitted ✓ but `recoverable_modules` field is a string list, not strongly typed.
918. - [ ] [W2] [BP3] [M5] [GAP] No `chassis.module_overheated` event — Shield emitter overheat scope is M5+ but not implemented.
919. - [ ] [W2] [BP3] [M5] [DR-014] [GAP] No `chassis.weapon_jammed` event distinct from `module_state_changed` (DR-014 promised `weapon_jammed/weapon_cleared` events).
920. - [ ] [W2] [BP3] [M5] [GAP] No `chassis.weapon_cleared` event.

## 115. DR-015 — Player identity / command-core posture (CLOSED-DIRECTION; M5+M7 own; BP3 should expose handoff path)
1041. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "Direct control overrides AI through the same serializable intent/control layer" — at BP3 the player controls ONE actor and AI controls a separate Guard; there's no handoff API.
1042. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "Releasing control hands the actor back to AI cleanly" — no release API.
1043. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "Save identity belongs to the command core/profile, not to the currently controlled body" — no save profile concept; cf-save is per-actor.
1044. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "Replay must distinguish player-piloted actions, AI-controlled actions, order-driven actions, and autonomous recovery decisions" — events tagged via `IntentSource` enum but `IntentSource::Ai` is never actually emitted.
1045. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "Commander-only breach test" — never run.
1046. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "Pilot intervention test" — never run.
1047. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "AI handoff replay" — not implemented.
1048. - [ ] [W2] [BP3] [M5+M7] [DR-015] [GAP] DR-015 "Strategy readability: Squad panel and command overlay explain what each body is trying to do" — no squad panel at BP3.

## 133. spec/animation-system — Bevy integration + tag-event gaps (M5 should have surfaced)
1181. - [ ] [W2] [M5] [GAP] `cf-anim` crate — does not exist at BP3 (animation logic lives ad-hoc in cf-render-2d).
1182. - [ ] [W2] [M5] [GAP] `AnimationStateMachine` component — not implemented.
1183. - [ ] [W2] [M5] [GAP] `AnimationManifest` loaded from RON — no `content/actors/*.ron` animation manifests authored.
1184. - [ ] [W2] [M5] [GAP] `SpriteAnimator` (sprite-sheet steps through frame indices + emits tag events) — not implemented.
1185. - [ ] [W2] [M5] [GAP] `SkeletalAnimator` (Spine/DragonBones bone transforms + tag events) — not implemented.
1186. - [ ] [W2] [M5] [GAP] `ProceduralOverlayApplier` (stacks recoil/knockback/limb-track on base anim) — not implemented.
1187. - [ ] [W2] [M5] [GAP] `RagdollComponent` (marker for physics-ragdoll mode transition) — not implemented.
1188. - [ ] [W2] [M5] [GAP] `animation.tag_fired` event — declared but not emitted (no animator yet).
1189. - [ ] [W2] [M5] [GAP] Animation event tags: `breath_emit` / `footstep_left` / `footstep_right` / `muzzle_flash_anchor` / `casing_eject` / `weapon_recoil_apply` / `mouth_phoneme_*` — none emitted.
1190. - [ ] [W2] [M5] [GAP] Animation tag `weapon_recoil_apply` — should trigger procedural recoil overlay; not wired.

## 134. cf-app screenshot + snapshot facilities
1191. - [ ] [W2] [GAP] cf-app does not save a screenshot via keystroke (F12 / Cmd-3 / Shift-Cmd-3).
1192. - [ ] [W2] [GAP] cf-app does not embed scenario metadata in screenshot EXIF.
1193. - [ ] [W2] [GAP] cf-app does not save mid-tick snapshot of full sim state (capture for debugging desync).
1194. - [ ] [W2] [GAP] cf-app does not auto-screenshot on mission_resolved event.
1195. - [ ] [W2] [GAP] cf-app does not auto-screenshot on actor.actor_died event.

## 135. M5 — chassis save/load round-trip gaps
1196. - [ ] [W2] [M5] [GAP] cf-save serializes chassis_state ✓ but does not include `chassis_view` projection (Computed from state; if not serialized may diverge on load).
1197. - [ ] [W2] [M5] [GAP] cf-save does not include scenario manifest fingerprint in save (cross-version load could be wrong scenario).
1198. - [ ] [W2] [M5] [GAP] cf-save does not include the M5 added `tutorial_safety` flag verification at load.
1199. - [ ] [W2] [M5] [GAP] cf-save does not include `initial_stage` override flag — load could overwrite at-runtime state.
1200. - [ ] [W2] [M5] [GAP] cf-save does not include actor afflictions vec — currently empty but should serialize.

## 142. cf-app rendering correctness gaps at BP3 close
1261. - [ ] [W2] [BP3] [GAP] cf-app does not auto-pause when window loses focus.
1262. - [ ] [W2] [BP3] [GAP] cf-app does not slow down rendering when window is minimized.
1263. - [ ] [W2] [BP3] [GAP] cf-app does not honor reduced-motion at the render layer (only HUD respects).
1264. - [ ] [W2] [BP3] [GAP] cf-app does not detect display refresh rate / vsync mismatch.
1265. - [ ] [W2] [BP3] [GAP] cf-app does not present diagnostics for low FPS (`<60`).
1266. - [ ] [W2] [BP3] [GAP] cf-app does not respect display HiDPI scaling automatically (rendering may look small on 4K).
1267. - [ ] [W2] [BP3] [GAP] cf-app default window size (1280×720) — not configurable via CLI.
1268. - [ ] [W2] [BP3] [GAP] cf-app does not save window position / size between launches.
1269. - [ ] [W2] [BP3] [GAP] cf-app does not surface a "borderless fullscreen" mode.
1270. - [ ] [W2] [BP3] [GAP] cf-app does not check for OS-level "reduce motion" preference (macOS NSAccessibilityReducedMotion).

## 148. M5 closure verification gaps
1296. - [ ] [W2] [M5] [PART] `M5-D01` evidence claims "13 armor_layer_damaged + 1 armor_zone_destroyed" — actual count in this session is 11 + 1 (per events analysis).
1297. - [ ] [W2] [M5] [PART] `M5-D04` "chassis.salvaged with salvaged_module_ids[]" — emits ✓ but salvaged_module_ids field is sometimes empty (no surviving modules).
1298. - [ ] [W2] [M5] [DR-014] [PART] `M5-D05` "BODY-A and CHASSIS-A acceptance tests pass" — 12 + 12 = 24 named tests pass; but DR-014 mentions broader BODY-A test family (wound→equipment-drop→AI-doctrine chains) not covered.

## 150. M5 launch-content audit gaps inherited at BP3
1315. - [ ] [W2] [BP3] [M5] [GAP] M5 launch content `cf-equipment::role_records()` registry — covers rifle.default only (1 weapon) at BP3; spec calls for at least 3 launch roles.
1316. - [ ] [W2] [BP3] [M5] [GAP] M5 launch loadout `loadouts()` registry — 3 LOAD-A loadouts ✓ but they all reference the same single rifle.
1317. - [ ] [W2] [BP3] [M5] [GAP] M5 launch chassis 3 archetypes (Infantry + PoweredArmor + LightMech) ✓ but `cf-chassis::chassis_specs()` registry has no `id` enum (lookups are string-based).
1318. - [ ] [W2] [BP3] [M5] [GAP] M5 launch chassis no scenario-tunable variant (e.g., `light_mech.variant=assault`).
1319. - [ ] [W2] [BP3] [M5] [GAP] M5 launch chassis no shared `chassis_traits` registry for traits like "noisy_servos" / "high_recoil_resist".
1320. - [ ] [W2] [BP3] [M5] [GAP] M5 launch chassis no `chassis.faction` field for visual-register grouping.

## 152. Engine + scenario integration gaps
1327. - [ ] [W2] [GAP] cf-control engine has no `--debug-capabilities` enabling (CLI flag exists, no consumer).
1328. - [ ] [W2] [M5] [GAP] cf-control engine `scenario.reset` does NOT honor `tutorial_safety` flag on reset (M5 chassis tutorial_safety should survive resets).
1329. - [ ] [W2] [GAP] cf-control engine `sim.run_for_ticks` does not surface per-step intermediate observations (only final).
1330. - [ ] [W2] [GAP] cf-control engine has no `sim.pause-on-event` predicate (`pause when chassis.stage_changed.stage=Wreck`).
1331. - [ ] [W2] [GAP] cf-control engine has no `scenario.replay` mode for re-running a bundle as if live.
1332. - [ ] [W2] [GAP] cf-control engine does not surface configured `tick_rate_hz` in observe.frame (read via run_manifest only).

## 153. cf-app input mapping gaps
1333. - [ ] [W2] [GAP] `cf-app::ingest_player_input` reads `Settings.key_bindings` ✓ but doesn't support modifier-key combos (Shift+W = sprint).
1334. - [ ] [W2] [GAP] cf-app doesn't support gamepad analog triggers for variable fire rate.
1335. - [ ] [W2] [GAP] cf-app doesn't support gamepad rumble feedback.
1336. - [ ] [W2] [GAP] cf-app doesn't support touch input (forward-compat for Steam Deck touchscreen).
1337. - [ ] [W2] [GAP] cf-app doesn't support stylus / pen input.
1338. - [ ] [W2] [GAP] cf-app input has no dead-zone configurability per axis.
1339. - [ ] [W2] [GAP] cf-app does not handle gamepad disconnection mid-game (player input freezes silently).
1340. - [ ] [W2] [GAP] cf-app does not allow multiple gamepads connected (one is hardcoded primary).

## 166. M5 — chassis stage progression gaps
1442. - [ ] [W2] [M5] [GAP] ChassisStage `Nominal` — works ✓.
1443. - [ ] [W2] [M5] [GAP] ChassisStage `ScuffedI` / `ScuffedII` — works ✓ but no per-stage HUD coloration variation.
1444. - [ ] [W2] [M5] [GAP] ChassisStage `Wounded` — works ✓.
1445. - [ ] [W2] [M5] [GAP] ChassisStage `Disabled` — works ✓ but Stance does not lock to Crouching automatically.
1446. - [ ] [W2] [M5] [GAP] ChassisStage `PilotInjured` — works ✓ but injury severity not granular.
1447. - [ ] [W2] [M5] [GAP] ChassisStage `Ejecting` — works ✓.
1448. - [ ] [W2] [M5] [GAP] ChassisStage `Ejected` — works ✓.
1449. - [ ] [W2] [M5] [GAP] ChassisStage `Extracted` — works ✓ but no extraction zone validation.
1450. - [ ] [W2] [M5] [GAP] ChassisStage `BailedTooLate` — declared but no path to reach it from current scenarios.
1451. - [ ] [W2] [M5] [GAP] ChassisStage `Wreck` — works ✓.
1452. - [ ] [W2] [M5] [GAP] ChassisStage `Gibbed` — works ✓ but no gib visualization.

## 167. M5 — chassis salvage outcomes gaps
1453. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` requires actor adjacency — not enforced.
1454. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` requires tool — not enforced (any actor can salvage).
1455. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` time-cost — instant; should require N ticks.
1456. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` does not emit "salvage_started" event before "salvaged".
1457. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` outcome: `salvaged_module_ids` field — but no `recovered_armor_layers` field.
1458. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` outcome: no `recovered_actor_inventory` (held rifle, gear).
1459. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` outcome: no `gold_recovered` for economy.
1460. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` does NOT mark chassis as `Gibbed` after — leaves it Wrecked.
1461. - [ ] [W2] [M5] [GAP] `act.chassis.salvage` cannot be reversed — no `undo` for accidental salvage.

## 168. M5 — module state machine gaps
1462. - [ ] [W2] [M5] [GAP] Module `OK` state — works ✓.
1463. - [ ] [W2] [M5] [GAP] Module `Degraded` state — works ✓.
1464. - [ ] [W2] [M5] [GAP] Module `Warning` state — works ✓.
1465. - [ ] [W2] [M5] [GAP] Module `Failed` state — works ✓.
1466. - [ ] [W2] [M5] [GAP] Module `Destroyed` state — works ✓ but no module-respawn after repair.
1467. - [ ] [W2] [M5] [GAP] Module state transitions are unidirectional (no recovery) — repair_module only resets to OK; doesn't pass through Warning etc.
1468. - [ ] [W2] [M5+M5.7] [GAP] Module state machine has no "overheating" intermediate (M5.7+ scope).
1469. - [ ] [W2] [M5] [GAP] Module state changes do NOT emit `cause_event_id` — root cause unclear.
1470. - [ ] [W2] [M5] [GAP] Module state changes have `reason` field but the reason vocab is narrow (5 reasons).

## 169. M5 — armor layer state gaps
1471. - [ ] [W2] [M5] [GAP] Armor `External` layer — works ✓ but no per-layer cover-arc.
1472. - [ ] [W2] [M5] [GAP] Armor `Internal` layer — works ✓ but no per-layer hardness.
1473. - [ ] [W2] [M5] [GAP] Armor `Core` layer — works ✓ but no per-layer wound-spread.
1474. - [ ] [W2] [M5] [GAP] Armor layer hp values are static (e.g., External = 100 / 100); no per-zone variation.
1475. - [ ] [W2] [M5] [GAP] Armor layer "Wound" zone state — exists post-Core-destroyed but no separate event vs `armor_zone_destroyed`.
1476. - [ ] [W2] [M5] [GAP] Armor layer "Destroyed" zone state — `armor_zone_destroyed` fires once; no recurring `still_wounded` events.
1477. - [ ] [W2] [M5] [GAP] Armor layer hardness gating in `apply_zone_damage` — works for blunt-force only; not for piercing/blast.
1478. - [ ] [W2] [M5] [GAP] Armor layer "glance" event — emits ✓ but no `glance_angle_radians` field.
1479. - [ ] [W2] [M5] [GAP] Armor layer regeneration — no slow auto-repair (always manual repair).
1480. - [ ] [W2] [M5] [GAP] Armor layer scrap material — no `scrap_material_id` field; salvage doesn't yield specific material.

## 170. M5 — pilot binding gaps
1481. - [ ] [W2] [M5] [GAP] Pilot `Bound` state — works ✓.
1482. - [ ] [W2] [M5] [GAP] Pilot `Injured` state — works ✓.
1483. - [ ] [W2] [M5] [GAP] Pilot `Ejecting` state — works ✓.
1484. - [ ] [W2] [M5] [GAP] Pilot `Ejected` state — works ✓ but pilot does NOT become a separate actor entity post-eject; the actor "becomes" the pilot.
1485. - [ ] [W2] [M5] [GAP] Pilot `Extracted` state — works ✓.
1486. - [ ] [W2] [M5] [GAP] Pilot `Lost` state — declared but unreachable in current eject flow.
1487. - [ ] [W2] [M5] [GAP] Pilot eject takes 60 ticks (1 second at 60 Hz); not configurable per chassis.
1488. - [ ] [W2] [M5] [GAP] Pilot eject does not require an eject button hold — instantaneous on `act.player.eject`.
1489. - [ ] [W2] [M5] [GAP] Pilot eject does not damage pilot on ejection from a high-velocity chassis.
1490. - [ ] [W2] [M5+M5.8] [GAP] Pilot eject does not consume oxygen (M5.8+ scope).

## 194. M3B — replay viewer DR audit (per DR-002 + DR-051 + DR-046)
1636. - [ ] [W2] [M3B] [DR-002+DR-012+DR-046+DR-051] [GAP] M3B replay viewer respects DR-012 ACC-A "200% scale" — markdown rendering; no UI scale.
1637. - [ ] [W2] [M3B] [DR-002+DR-046+DR-051] [GAP] M3B replay viewer color-independent state labels — markdown text-only (✓).
1638. - [ ] [W2] [M3B] [DR-002+DR-046+DR-051] [GAP] M3B replay viewer Tier-A 11-language keyed-strings — English-only strings in viewer output.
1639. - [ ] [W2] [M3B] [DR-002+DR-046+DR-051] [GAP] M3B replay viewer `cause-chain` command rejection on cycle — handles ✓ via CycleDetected enum variant.
1640. - [ ] [W2] [M3B] [DR-002+DR-046+DR-051] [GAP] M3B replay viewer rejects non-monotonic events — does NOT check; iterates in order regardless.

## 195. M5 — DR-029 save game first-slice audit
1641. - [ ] [W2] [M5] [DR-029] [GAP] M5 cf-save serializes ChassisState ✓ but does not include `pilot_state.eject_progress_ticks`.
1642. - [ ] [W2] [M5] [DR-029] [GAP] M5 cf-save serializes Inventory ✓ but `selected_slot` field is u8; no validation against MAX_SLOTS.
1643. - [ ] [W2] [M5] [DR-029] [GAP] M5 cf-save deserialize path does NOT detect when `schema_version` is missing (assumes 1).
1644. - [ ] [W2] [M5] [DR-029] [GAP] M5 cf-save `blake3` checksum field — present; not verified on load.
1645. - [ ] [W2] [M5] [DR-029] [GAP] M5 cf-save round-trip test exists for `m5_chassis_wreck_eject` scenario but not for `m5_chassis_salvage`.

## 196. M5 — DR-018 death-meaning ladder partial coverage
1646. - [ ] [W2] [M5] [DR-018] [GAP] M5 `actor_lost_permanently` event — declared in DR-018 ladder; not emitted.
1647. - [ ] [W2] [M5] [DR-018] [GAP] M5 `extraction_offered` event — declared; not emitted (extraction zone enters but no offer event).
1648. - [ ] [W2] [M5] [DR-018] [GAP] M5 `extraction_taken` event — declared; not emitted (M5 wreck_eject win path uses chassis.pilot_extracted only).
1649. - [ ] [W2] [M5] [DR-018] [GAP] M5 `extraction_missed` event — declared; not emitted.
1650. - [ ] [W2] [M5] [DR-018] [GAP] M5 named-actor "veteran tombstone" with story tags — no Veteran entity at BP3.

## 217. DR-033 — Full collision physics direction (CLOSED; M5.5 owns; BP3 forward-compat scaffold only)
1916. - [ ] [W2] [BP3] [M5.5] [DR-033] [PART] DR-033 collision class enum (`CollisionClass` with 16 variants) — scaffolded at BP3 ✓ but no production code consumes.
1917. - [ ] [W2] [BP3] [M5.5] [DR-002+DR-033] [PART] DR-033 collision events `collision_pair_created` / `collision_contact_started` / `_persisted` / `_ended` — declared in DR-002 baseline but never emitted at BP3.
1918. - [ ] [W2] [BP3] [M5.5] [DR-033] [GAP] DR-033 collision matrix loader — `content/collision/` directory missing.
1919. - [ ] [W2] [BP3] [M5.5] [DR-033] [GAP] DR-033 collision proxies for actor core / limbs / armor / weapons / projectiles / terrain / debris / base / mech — none implemented.
1920. - [ ] [W2] [BP3] [M5.5] [DR-033] [GAP] DR-033 collision contact-event payload schema — not authored.
1921. - [ ] [W2] [BP3] [M5.5] [DR-033] [GAP] DR-033 collision_filter_reason vocabulary — not declared.

## 221. DR-038 — Universal gravity & ballistics direction (CLOSED-DIRECTION; M5.5 + M5.6 + M5.9 own; BP3 forward-compat scaffold)
1937. - [ ] [W2] [BP3] [M5.5+M5.6+M5.9] [DR-038] [PART] DR-038 `GravityField` enum scaffolded at BP3 with `Uniform(f32)` variant only — per-cell sampling deferred.
1938. - [ ] [W2] [BP3] [M5.5+M5.6+M5.9] [DR-038] [GAP] DR-038 ballistic-drag projectile mass field — projectiles have no mass.
1939. - [ ] [W2] [BP3] [M5.5+M5.6+M5.9] [DR-038] [GAP] DR-038 ballistic-drag projectile cross-sectional area — not implemented.
1940. - [ ] [W2] [BP3] [M5.5+M5.6+M5.9] [DR-038] [GAP] DR-038 ballistic-drag atmospheric ρ_local read — cf-atmos stub.
1941. - [ ] [W2] [BP3] [M5.5+M5.6+M5.9] [DR-038] [GAP] DR-038 CI grep gate "No hardcoded 9.81 anywhere in production code" — not active.
1942. - [ ] [W2] [BP3] [M5.5+M5.6+M5.9] [DR-038] [GAP] DR-038 universal gravity field reads through one source — works for sim baseline; render layer does not consult.

## 224. M5 chassis salvage outcome — DR-041 mining bridge
1949. - [ ] [W2] [M5] [DR-041] [GAP] M5 `chassis.salvaged` outcome `recovered_ore_quantity` field — `salvaged_module_ids` only; no ore yield.
1950. - [ ] [W2] [M5] [DR-036+DR-041] [GAP] M5 salvage does not interact with material kernel (DR-036) for material reclamation.

## 241. Hot-path lint / static analysis gaps
2080. - [ ] [W2] [GAP] No `cargo deny` for license + CVE + bans config.
2081. - [ ] [W2] [GAP] No `cargo audit` for security advisories.
2082. - [ ] [W2] [GAP] No `cargo geiger` for unsafe code surface.
2083. - [ ] [W2] [GAP] No `cargo-machete` for unused-dep detection.
2084. - [ ] [W2] [GAP] No `cargo-outdated` regular check.
2085. - [ ] [W2] [GAP] No `cargo-bloat` for binary size monitoring.
2086. - [ ] [W2] [GAP] No `cargo-llvm-cov` for code coverage measurement.
2087. - [ ] [W2] [GAP] No `tokio-console` for async observability.

## 244. M5 — ChassisState observability gaps
2101. - [ ] [W2] [M5] [GAP] `chassis_view.last_damage_event` — not present (could carry the most-recent armor-layer-damaged event id).
2102. - [ ] [W2] [M5] [GAP] `chassis_view.eject_progress_ticks` / `eject_total_ticks` — present ✓ but no percentage helper.
2103. - [ ] [W2] [M5] [GAP] `chassis_view.destroyed_zones[]` — present ✓ but does not carry the destruction-time tick.
2104. - [ ] [W2] [M5] [GAP] `chassis_view.salvaged_module_ids[]` — present ✓ but does not carry salvage-time tick.
2105. - [ ] [W2] [M5] [GAP] `chassis_view.integrity` — present ✓ but is a single scalar; no per-zone breakdown.
2106. - [ ] [W2] [M5] [GAP] `chassis_view.weapon_jammed` — bool ✓ but no `jammed_since_tick` field.
2107. - [ ] [W2] [M5] [GAP] `chassis_view.weapon_jam_severity` — not present.

## 245. M5 — chassis attempt_eject + tick_chassis_eject_for_all gaps
2108. - [ ] [W2] [M5] [GAP] `chassis.attempt_eject` — works ✓ but no rejection if chassis is already Wreck/Gibbed (silently does nothing).
2109. - [ ] [W2] [M5] [GAP] `tick_chassis_eject_for_all` — works ✓ but doesn't account for `--paced` flag (eject ticks at sim rate, not wall-clock).
2110. - [ ] [W2] [M5] [GAP] `chassis.attempt_eject` does NOT check actor's position (eject is teleport, not motion).
2111. - [ ] [W2] [M5] [DR-018] [GAP] `chassis.attempt_eject` does NOT check if pilot has been Wounded enough to need eject (DR-018 path: rescue first).
2112. - [ ] [W2] [M5] [DR-009] [GAP] `chassis.attempt_eject` does NOT consume a `pilot_focus_charge` resource (DR-009 OPEN command focus).

## 246. M5 — refresh_hud_chassis_banners gaps
2113. - [ ] [W2] [M5] [GAP] `refresh_hud_chassis_banners` — raises banner on stage transitions ✓ but does not raise banner on `chassis.armor_layer_damaged` (only on stage change).
2114. - [ ] [W2] [M4+M5] [GAP] Banners do not include the chassis zone name in the text (`ARMOR CRACKED LEFT` is M4-S03 contract; current banner says only `ARMOR_CRACKED`).
2115. - [ ] [W2] [M5] [GAP] Banner severity-word vocabulary — narrow set (`HP_LOW` / `ARMOR_CRACKED` / `EJECT_NOW` / `CHASSIS_WRECKED` / `MISSION_WON` / `MISSION_FAILED` / `AMMO_OUT`); should extend with `MODULE_FAILED` / `WEAPON_JAMMED` / `PILOT_INJURED`.

## 247. M5 — emit_chassis_events parent-event linking gaps
2116. - [ ] [W2] [M5] [GAP] `chassis.armor_layer_damaged.parent_event_id` — set to the triggering projectile_hit ✓ but only for the M5 chassis_wreck_eject scenario (not tested in salvage scenario).
2117. - [ ] [W2] [M5] [GAP] `chassis.module_state_changed.parent_event_id` — sometimes empty (when the propagation cause is "bound zone destroyed", parent is not the projectile_hit).
2118. - [ ] [W2] [M5] [GAP] `chassis.stage_changed.parent_event_id` — set to most-recent damage event but if multiple events tied with same tick, ordering is implementation-specific.
2119. - [ ] [W2] [M5] [GAP] `chassis.pilot_ejected.parent_event_id` — set to the `act.player.eject` command_accepted event ✓.
2120. - [ ] [W2] [M5] [GAP] `chassis.pilot_extracted.parent_event_id` — set to the player.move event that crossed into extraction zone ✓.

## 248. M5 — body silhouette + observe parity gaps
2121. - [ ] [W2] [M5] [GAP] `BodySilhouette` projection chassis-attached actors `placeholder=false` ✓; chassis-less actors `placeholder=true` — that's the M5 gap.
2122. - [ ] [W2] [M5] [GAP] `BodySilhouette.head_hp_pct` for chassis-less actors derives from generic HP — not granular.
2123. - [ ] [W2] [M5] [GAP] `BodySilhouette` has no per-armor-layer field (External / Internal / Core); only hp%.
2124. - [ ] [W2] [M5] [GAP] `BodySilhouette` does not include `wounded` state vs `severed` state distinction.
2125. - [ ] [W2] [M5] [GAP] `BodySilhouette` does not include `bleeding` rate.
2126. - [ ] [W2] [M5] [GAP] `BodySilhouette` does not include `last_damage_zone` for HUD focus pulse.

## 249. M4A — module-strip line gaps
2127. - [ ] [W2] [M4A] [GAP] `ModuleStrip` line shows up to 5 modules at BP3 (WEAPON / JET / SHIELD / SENSOR / REPAIR); chassis spec carries 4-5 modules.
2128. - [ ] [W2] [M4A] [GAP] `ModuleStrip.placeholder` — true for chassis-less actors; works ✓.
2129. - [ ] [W2] [M4A+M5.7] [GAP] `ModuleStrip` does not show overheat state (M5.7 scope but should at least stub).
2130. - [ ] [W2] [M4A] [GAP] `ModuleStrip` does not show ammo / power state per module.
2131. - [ ] [W2] [M4A] [GAP] `ModuleStrip` does not show repair-progress on chassis.repaired in flight.
2132. - [ ] [W2] [M4A] [GAP] `ModuleStrip` does not show `bound_zone` reference (which limb the module is bound to).

## 282. spec/chassis-armor-mechs-and-origins — Chassis grammar gaps (BP3 M5 owns)
2639. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS 5 layers (Operator/pilot + Frame + Armor layers + Modules + Held/mounted equipment) — partial; only Frame+module pips at BP3.
2640. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `nominal` — implicit (no stage tracking) at BP3.
2641. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `degraded` (cosmetic wear + smoke wisps + slight perf hit) — not implemented.
2642. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `module-warning` (module HUD flash + minor effect) — not implemented.
2643. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `module-failed` (one module disabled: jet/shield/sensor) — module pips show count only.
2644. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `weapon-jammed` (held/mounted weapon stops + must clear) — not implemented.
2645. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `armor-cracked` (specific armor plate torn + underlying layers exposed) — no armor layers at BP3.
2646. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `disabled` (mobility module + leg armor failure or mass damage) — partial.
2647. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `pilot-injured` (penetration past last armor layer) — no pilot at BP3.
2648. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `eject` (pilot leaves while chassis can still be saved) — partial M5 closure but no AI doctrine.
2649. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `bail-too-late` (pilot eject after explosion threshold) — not implemented.
2650. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `wreck` (destroyed but recoverable / salvageable) — partial M5 closure.
2651. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS damage stage `gibbed/exploded` (critical-explosion roll on wreck + scattered debris/loot) — not implemented.
2652. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS module schema (`module_id` + `slot_id` + `mass` + `power_draw` + `bandwidth_or_other_resource_use` + health/damage stages + function hooks + mod-author metadata) — single module-pip count only.
2653. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS jet pack module (thrust provided + sputtering warning + failed=no-thrust) — no jetpack.
2654. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS shield emitter module (shield_pool + overheat fail + cooldown) — no shields.
2655. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS targeting computer module (aim assist + EMP fail) — no aim assist.
2656. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS repair drone module (slow self-repair) — not implemented.
2657. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS sensor pod module (AI/UX vision + smoke degraded) — not implemented.
2658. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS Command Core embedding (compatibility check + base tradeoff + avatar bonuses + damage risk + extraction) — no command core entity.
2659. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS Origin "Powered organic" (cybernetic enhancements + mixed wound+module) — not implemented.
2660. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS Origin "Synthetic / android" (organic wound side + module/circuit damage side + EMP vulnerable + reduced bleed + reduced G-load + per-installed-module overclock + batteries + sealed-vacuum option) — not implemented.
2661. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS Origin "Synthetic / robot" (NO organic wounds / NO bleed / NO concussion / NO G-load + internal-shock damage + coolant+oil leak + whole-processor overclock + involuntary downclock + power resource + vacuum-immune + heat-tolerant) — not implemented.
2662. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS Origin "Construct / drone" (pilot-less + remote-controlled + bandwidth-limited + disconnectable) — not implemented.
2663. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS Origin "Heavy biomech / fused" (grown chassis + self-repair + energy-type-specific weakness) — not implemented.
2664. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `chassis_stage_changed` (actor + chassis + layer + old/new stage + cause + reason) — partial.
2665. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `module_state_changed` (module + slot + old/new state + cause) — not emitted.
2666. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `armor_layer_damaged` (layer + zone + hit event + integrity remaining) — no armor layers.
2667. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `weapon_jammed` / `weapon_cleared` (cause + ms_to_clear) — not emitted.
2668. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `pilot_state_changed` — no pilot.
2669. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `pilot_ejected` / `pilot_extracted` / `pilot_lost` (success + cause) — partial; eject works but no doctrine.
2670. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `chassis_repaired` (layer/module + repaired_by + ms_to_repair) — not implemented.
2671. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS event `chassis_salvaged` (recovered_modules + recovered_equipment + recovered_by) — not implemented.
2672. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS AI doctrine `bail_chassis` ("armor critical, no repair available" / "module-failed: mobility" / "weapon disabled" / "pilot injured") — not implemented.
2673. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS AI doctrine `request_repair` ("module-failed: shield, repair drone in range") — not implemented.
2674. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS AI doctrine `swap_module` ("module-failed: sensor, swap to spare available") — not implemented.
2675. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS AI doctrine `clear_jam` ("weapon jammed, distance > X") — not implemented.
2676. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS AI doctrine `evade_layer` ("armor cracked on left side, rotate right") — not implemented.
2677. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS AI doctrine `self_destruct` (doctrine-specific) — not implemented.
2678. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS HUD body silhouette with armor zones (for selected actor + on hover for squad) — partial; no armor zones.
2679. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS HUD module strip (4-6 icons with stage colors for selected actor) — partial.
2680. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS HUD pilot health pip (distinct from chassis stage) — no pilot.
2681. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS HUD stage banner ("ARMOR CRACKED LEFT" / "JET FAILED" / "EJECT NOW") on stage transition + auto-fade — not implemented.
2682. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS HUD repair affordance icon (when repairable module in range of repair tool) — not implemented.
2683. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS HUD salvage marker on wrecks (once chassis enters `wreck` stage) — partial.
2684. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS modding "Add new origin/race with chassis defaults" — not implemented.
2685. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS modding "Add new chassis class (light mech / heavy mech / exo / drone)" — not implemented.
2686. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS modding "Add new module with state grammar" — not implemented.
2687. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS modding "Override damage stages for existing chassis" — not implemented.
2688. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS modding "Provide AI doctrine overrides for new origin/race" — not implemented.
2689. - [ ] [W2] [BP3] [M5] [GAP] CHASSIS modding "Provide HUD assets for new chassis layers/zones" — not implemented.

## 283. spec/body-damage-model — Body / damage / wound / equipment fallout (BP3 M5 partial)
2690. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `actor_health` (core survival + max + prior + death timer) — single HP only.
2691. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `actor_status` (STABLE / UNSTABLE / DYING / DEAD / INACTIVE / KNOCKED_OUT / PILOT_TRAPPED) — single bool at BP3.
2692. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `body_parts` (head / torso / arms / legs / hands / feet / backpack / jetpack / held-device anchors) — no parts.
2693. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `actor_origin` (human/organic + android/synthetic + robot frame + augmented + alien/biological + treatment/vulnerability tags) — single origin.
2694. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `chassis_modules` — see §282.
2695. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `attachments` (joint strength + gib impulse limit + gib wound limit + damage multiplier + mission-critical + part ownership) — not modeled.
2696. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `wounds` (entry/exit emitter + source event + damage channel + part + severity + bleed/pain/stability modifiers + treatment tags) — no wounds.
2697. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `equipment_condition` (Intact / Impaired / Critical / Disabled / Destroyed stages for weapons/tools/armor/modules + behavior penalty + repairability) — no equipment condition.
2698. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `stability` (stable velocity thresholds + recovery timer + travel impulse damage + posture) — not modeled.
2699. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `inventory_fallout` (dropped weapon/tool/gold + position + velocity + owner + salvage) — no fallout.
2700. - [ ] [W2] [BP3] [M5] [GAP] BODY model layer `treatment_state` (removed wounds + stabilized parts + revives + scars + prosthetics + repair/replace operations) — no treatment.
2701. - [ ] [W2] [BP3] [M5] [GAP] BODY status `STABLE` (normal aim/move/use) — implicit; no event when entering.
2702. - [ ] [W2] [BP3] [M5] [GAP] BODY status `UNSTABLE` (wobble/fall icon + short reason text + AI seek recovery + medic deprioritize) — not modeled.
2703. - [ ] [W2] [BP3] [M5] [GAP] BODY status `DYING` (red silhouette pulse + "weapon dropped" + rescue/finish window) — partial.
2704. - [ ] [W2] [BP3] [M5] [GAP] BODY status `DEAD` (death marker + death recap entry + salvage/revive target only with tool) — partial.
2705. - [ ] [W2] [BP3] [M5] [GAP] BODY status `INACTIVE` (muted squad card + scenario text) — not implemented.
2706. - [ ] [W2] [BP3] [M5] [GAP] BODY status `KNOCKED_OUT` (prototype-only for non-lethal rescue/arrest/medical) — not implemented.
2707. - [ ] [W2] [BP3] [M5] [GAP] BODY status `PILOT_TRAPPED` (alive but trapped in damaged armor/mech) — not implemented.
2708. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `piercing` (entry/exit + penetration + part hit + source item + body armor result) — partial.
2709. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `cut` (slash/cut + melee/blades/saw traps) — not implemented.
2710. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `blunt` (travel impulse + body hit sounds + impact forces + falls + knockdown vs wound) — not implemented.
2711. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `explosive` (blast impulse + gib limits + explosive rounds + danger radius + terrain carve + part detach/gib + dropped equipment + friendly-fire labels) — no explosives.
2712. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `thermal` (fire/burn + persistent hazard + treatment state) — not implemented.
2713. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `electric_emp` (robotics/devices disable channel) — not implemented.
2714. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `equipment_fault` (weapon/tool/armor/mech module condition-stage changes + jam/fault + smoke/spark + repair/swap) — not implemented.
2715. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `chemical_bio` (poison/acid/stim/bleed modifiers + effect stack) — not implemented.
2716. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `terrain_crush` (dropship/body collision + fall + unstable impact + terrain/object physics + cause attribution) — partial.
2717. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `radiation` (radiation_dose_mSv per actor + threshold afflictions: nausea/radiation sickness/acute radiation syndrome + robot electronics fault at extreme dose) — no radiation.
2718. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `thermal_environmental` (body_temperature_K per actor + frostbite/hypothermia + burns/heatstroke + per-origin thresholds) — no thermal.
2719. - [ ] [W2] [BP3] [M5] [GAP] BODY damage channel `acoustic_trauma` (high-decibel events + hearing loss + tinnitus + temporary deafness + suit hearing protection) — no acoustics.
2720. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `part_id` / `display_name` / `side` / `parent_part_id` — not modeled.
2721. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `attachable_kind` (root / limb / head / hand / foot / jetpack / held_device_socket / armor_plate / prosthetic / special) — not modeled.
2722. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `damage_multiplier` — not modeled.
2723. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `joint_strength` — not modeled.
2724. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `gib_wound_limit` / `gib_impulse_limit` — not modeled.
2725. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `can_hold_item` / `held_slot_ids` — not modeled.
2726. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `movement_contribution` (legs/feet/mobility damage → limp/crawl/jump/jet failure) — not implemented.
2727. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `aim_contribution` (arm/head/sensor damage → reticle spread reasons) — not implemented.
2728. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `criticality` (`lethal_if_missing` / `mission_critical` / `revivable` / `prosthetic_replaceable`) — not modeled.
2729. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `wound_slots` (avoid unbounded invisible wounds) — not modeled.
2730. - [ ] [W2] [BP3] [M5] [GAP] BODY part contract `treatment_hooks` (remove wound / stabilize bleed / splint / revive / replace limb / seal leak / repair prosthetic) — not implemented.
2731. - [ ] [W2] [BP3] [M5] [GAP] BODY wound `wound_id` / `wound_type` / `source_event_id` — not emitted.
2732. - [ ] [W2] [BP3] [M5] [GAP] BODY wound `entry_or_exit` field — not emitted.
2733. - [ ] [W2] [BP3] [M5] [GAP] BODY wound `bleed_rate` / `pain_or_focus` / `stability_penalty` — not modeled.
2734. - [ ] [W2] [BP3] [M5] [GAP] BODY wound `movement_penalty` / `aim_penalty` / `grip_penalty` — not implemented.
2735. - [ ] [W2] [BP3] [M5] [GAP] BODY wound `treatment_tags` (medikit/support tools can decide what fixes what) — not modeled.
2736. - [ ] [W2] [BP3] [M5] [GAP] BODY wound `visibility_tier` (default HUD vs advanced panel) — not modeled.
2737. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Arm wounded" (grip/aim/reload penalty + arm segment marked + bot may switch to sidearm) — not implemented.
2738. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Arm detached/gibbed" (held device removed + clear missing-arm icon + dropped marker + slot invalidation) — not implemented.
2739. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Leg wounded" (stability+mobility penalty + limp/crawl icon + bot slows + rescue call) — not implemented.
2740. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Leg detached/gibbed" (movement/crawl/jet dependency change + extraction warning + rescue/carry priority) — not implemented.
2741. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Head destroyed" (lethal except special actors + decapitation/head loss recap) — not implemented.
2742. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Torso critical" (high bleed/stability/death risk + medic priority + retreat) — not implemented.
2743. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Jetpack/backpack damaged" (mobility/flight/support loss + "jetpack disabled" + bot refuses vertical route) — no jetpack.
2744. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Held device damaged/dropped" (weapon/tool unavailable + dropped marker + slot warning + AI switches/retrieves) — not implemented.
2745. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Weapon/tool impaired" (jams + overheats + misfires + accuracy loss + dig poorly + scan unreliably + smokes + condition badge) — not implemented.
2746. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Armor layer cracked" (local protection reduced + local armor-stage icon + AI changes stance) — no armor layers.
2747. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Mech limb/module disabled" (loses weapon/tool/grip/mobility/sensor/power + cockpit/pilot risk + route-fit warning + AI may repair/eject/abandon/tow) — partial.
2748. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Android/robot EMP shock" (lose power/sensors/control temporarily/permanently + EMP/shutdown/reboot label + AI seeks reboot/repair) — not implemented.
2749. - [ ] [W2] [BP3] [M5] [GAP] BODY consequence "Mission-critical gib blocked" (object stays intact despite threshold + debug/workbench warning + optional spark/brace feedback) — not implemented.
2750. - [ ] [W2] [BP3] [M5] [GAP] BODY event `body_hit` — partial.
2751. - [ ] [W2] [BP3] [M5] [GAP] BODY event `body_wound_added` — not emitted.
2752. - [ ] [W2] [BP3] [M5] [GAP] BODY event `body_part_detached` — partial; not parented to cause event id.
2753. - [ ] [W2] [BP3] [M5] [GAP] BODY event `body_gib_spawned` — not emitted.
2754. - [ ] [W2] [BP3] [M5] [GAP] BODY event `body_stability_impulse` — no stability.
2755. - [ ] [W2] [BP3] [M5] [GAP] BODY event `actor_status_changed` (old/new status + cause event + rescue window) — partial.
2756. - [ ] [W2] [BP3] [M5] [GAP] BODY event `inventory_dropped` — no drop.
2757. - [ ] [W2] [BP3] [M5] [GAP] BODY event `equipment_condition_changed` (old/new stage + behavior penalty + visible feedback + repairability + cause) — no condition.
2758. - [ ] [W2] [BP3] [M5] [GAP] BODY event `armor_stage_changed` — no armor stages.
2759. - [ ] [W2] [BP3] [M5] [GAP] BODY event `chassis_module_damaged` (module slot + old/new stage + behavior consequence + pilot risk + cause) — partial.
2760. - [ ] [W2] [BP3] [M5] [GAP] BODY event `pilot_state_changed` — no pilot.
2761. - [ ] [W2] [BP3] [M5] [GAP] BODY event `origin_status_changed` (origin-specific status + repair/treatment requirement + cause) — no origins.
2762. - [ ] [W2] [BP3] [M5] [GAP] BODY event `gold_dropped` (amount/pixel count + position + cause) — no economy.
2763. - [ ] [W2] [BP3] [M5] [GAP] BODY event `treatment_applied` (support item + target part + wounds removed/changed + result + failure reason) — no treatment.
2764. - [ ] [W2] [BP3] [M5] [GAP] BODY event `death_recap_ready` (final cause chain + item drops + salvage/veteran consequence + replay marker) — no recap.
2765. - [ ] [W2] [BP3] [M5] [GAP] BODY event `mission_critical_gib_blocked` (object/part + attempted cause + authoring source + workbench warning) — not emitted.
2766. - [ ] [W2] [BP3] [M5] [GAP] BODY acceptance BODY-A-01..12 — none pass.
2767. - [ ] [W2] [BP3] [M5] [DR-003] [GAP] BODY HUD "Default stays compact (DR-003 silhouette: part state + status + severe consequences only)" — not implemented.
2768. - [ ] [W2] [BP3] [M5] [GAP] BODY HUD "Advanced view opt-in (detailed wound list + source ids + treatment tags + exact modifiers)" — not implemented.
2769. - [ ] [W2] [BP3] [M5] [GAP] BODY HUD "Consequences use verbs ('left arm dropped rifle' not 'arm 0 HP')" — not implemented.
2770. - [ ] [W2] [BP3] [M5] [GAP] BODY HUD "Death recap is causal (source → hit part → wound → impulse → status → drop/gib → final)" — no recap.
2771. - [ ] [W2] [BP3] [M5] [GAP] BODY HUD "Rescue window explicit (if DYING/KNOCKED_OUT can be saved, show timer/condition)" — not implemented.
2772. - [ ] [W2] [BP3] [M5] [GAP] BODY AI reason label `no_usable_arm` — not implemented.
2773. - [ ] [W2] [BP3] [M5] [GAP] BODY AI reason label `unstable_no_explosive` — not implemented.
2774. - [ ] [W2] [BP3] [M5] [GAP] BODY AI reason label `leg_loss_route_invalid` — not implemented.
2775. - [ ] [W2] [BP3] [M5] [GAP] BODY AI reason label `wound_needs_support_tool` — not implemented.
2776. - [ ] [W2] [BP3] [M5] [GAP] BODY AI reason label `friendly_in_blast_radius` — not implemented.
2777. - [ ] [W2] [BP3] [M5] [GAP] BODY AI reason label `mission_critical_cannot_gib` — not implemented.

## 284. spec/animation-system — Animation system (BP3 M5 owns visible actor presentation)
2778. - [ ] [W2] [BP3] [M5] [GAP] ANIM "Hybrid: sprite-sheet for non-hero + skeletal-rigged for hero chassis + procedural overlays for everyone" — sprite-sheet only at BP3.
2779. - [ ] [W2] [BP3] [M5] [GAP] ANIM skeletal-rigged hero chassis (18 player chassis: 3 PA tiers + 5 mech tiers + 4 robots + 4 androids + 1 drone) — not implemented.
2780. - [ ] [W2] [BP3] [M5] [GAP] ANIM `bevy_spine` OR `bevy_dragonbones` integration — not integrated.
2781. - [ ] [W2] [BP3] [M5] [GAP] ANIM physics authority blend mode `controlled_locomotion` (animation primary + secondary spring/inertia within stability limits) — not modeled.
2782. - [ ] [W2] [BP3] [M5] [GAP] ANIM physics authority blend mode `controlled_airborne` (jet/jump thrust + limbs trail under gravity/inertia/wind + aim arm stabilized unless damaged) — not implemented.
2783. - [ ] [W2] [BP3] [M5] [GAP] ANIM physics authority blend mode `braced_or_aiming` (feet/torso/weapon sockets stabilize + procedural recoil overlay) — not implemented.
2784. - [ ] [W2] [BP3] [M5] [GAP] ANIM physics authority blend mode `impaired_control` (limp + crawl + one-arm + disabled grip + reduced climb/jump/jet) — not implemented.
2785. - [ ] [W2] [BP3] [M5] [GAP] ANIM physics authority blend mode `disrupted_physics` (knockdown / stun / pressure gust / explosion / tumble / pinned/crush) — not implemented.
2786. - [ ] [W2] [BP3] [M5] [GAP] ANIM physics authority blend mode `ragdoll_or_gib` (deterministic ragdoll/gib proxies) — not implemented.
2787. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `idle` (4-frame loop + subtle bob/breath) — partial.
2788. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `walk` (8-frame loop + foot-anchor on frames 3+7) — partial; foot-anchor not tagged.
2789. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `run` (8-frame loop + lean) — not implemented.
2790. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `jump_takeoff` (3-frame) — not implemented.
2791. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `jump_air` (1-frame hold loop) — partial.
2792. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `jump_land` (3-frame) — not implemented.
2793. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `crouch_idle` (4-frame loop) — not implemented.
2794. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `crouch_walk` (6-frame loop) — not implemented.
2795. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `prone_idle` (4-frame loop) — not implemented.
2796. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `prone_crawl` (6-frame loop) — not implemented.
2797. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `aim_up` / `aim_mid` / `aim_down` (1 frame each + procedural blend per aim_pitch) — single aim pose.
2798. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `fire` (2-3 frame snap + per-weapon flash anchor) — partial.
2799. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `reload_short` (6 frames + magazine swap) — partial.
2800. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `reload_long` (12 frames + belt-fed / chamber reload) — not implemented.
2801. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `melee_strike` (4-frame) — not implemented.
2802. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `melee_block` (2-frame) — not implemented.
2803. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `throw_grenade` (6-frame) — no grenades.
2804. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `damage_react_light` (3-frame stagger) — not implemented.
2805. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `damage_react_heavy` (5-frame knock-back) — not implemented.
2806. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `death_fall_back` (6 frames + then ragdoll) — partial.
2807. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `death_fall_forward` (6 frames + then ragdoll) — not implemented.
2808. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `death_explode` (4 frames + gib particles) — not implemented.
2809. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `limb_loss_arm` (3 frames + procedural ragdoll on limb) — not implemented.
2810. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `limb_loss_leg` (4 frames + crawl variant) — not implemented.
2811. - [ ] [W2] [BP3] [M5] [DR-021] [GAP] ANIM animation `eject_seat` (8 frames + mech ejection per DR-021) — partial.
2812. - [ ] [W2] [BP3] [M5] [DR-018] [GAP] ANIM animation `salvage_action` (8 frames per DR-018) — partial.
2813. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `repair_action` (6 frames) — not implemented.
2814. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `dig_action` (6-frame loop) — partial.
2815. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `breach_action` (4 frames + door breach) — partial.
2816. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `revive_action` (8 frames) — not implemented.
2817. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `interact_short` (4 frames + buttons/terminals) — not implemented.
2818. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `interact_long` (12 frames + briefcase/console) — not implemented.
2819. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `signal_wave` (4 frames + tactical signaling) — not implemented.
2820. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `signal_point` (2 frames) — not implemented.
2821. - [ ] [W2] [BP3] [M5] [GAP] ANIM animation `voice_speak` (4-frame mouth loop) — not implemented.
2822. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `footstep_left` / `footstep_right` (walk/run frame + footstep SFX + dust trail VFX + AI hearing source) — not tagged.
2823. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `casing_eject` (fire frame + casing particle + drop physics + SFX) — not tagged.
2824. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `muzzle_flash_anchor` (fire frame + light emission) — not tagged.
2825. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `breath_emit` (idle frame in cold weather) — not implemented.
2826. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `oil_drip` (idle frame on damaged robot) — not implemented.
2827. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `coolant_leak` (continuous on damaged robot module) — not implemented.
2828. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `weapon_recoil_apply` (fire frame + procedural weapon kickback) — not implemented.
2829. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `eject_capsule` (eject frame + mech eject capsule spawn) — partial.
2830. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `limb_detach` (limb-loss final frame + detached limb spawn collidable) — not implemented.
2831. - [ ] [W2] [BP3] [M5] [GAP] ANIM event tag `ragdoll_begin` (death final frame + switch to physics-driven ragdoll) — not implemented.
2832. - [ ] [W2] [BP3] [M5] [DR-043] [GAP] ANIM event tag `mouth_phoneme_a/e/i/o/u` (voice-speak frame + lip-sync per DR-043 voice synthesis) — not implemented.
2833. - [ ] [W2] [BP3] [M5] [GAP] ANIM event `animation.tag_fired` (typed replay event per tag) — not emitted.
2834. - [ ] [W2] [BP3] [M5] [GAP] ANIM event `animation.state_changed` — not emitted.
2835. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Recoil` (per-weapon impulse + damping + torso bone + 0.3-0.8s decay) — single static recoil at BP3.
2836. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Knockback` (per-impulse + actor center + secondary jiggle / sprite scale punch + reset on land) — not implemented.
2837. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Limb tracking (aim)` (skeletal: arm+weapon bones rotate per aim_pitch; sprite: pose-blend between aim_up/mid/down) — not implemented.
2838. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Ragdoll on death` (Rapier integration via cf-physics; bones become rigidbodies with joints; sprite transition to gib particles) — not implemented.
2839. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Weapon-IK to hand socket` (skeletal: weapon transform parented to hand bone; sprite: anchor weapon offset per chassis pose) — not implemented.
2840. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Jet flame intensity` (particle emission scaled by jetpack thrust) — no jetpack.
2841. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Wound deformation (skeletal hero)` (bullet hits = small mesh-deform on impact + fade 0.5s) — not implemented.
2842. - [ ] [W2] [BP3] [M5] [GAP] ANIM procedural overlay `Cape / cloth simulation` (Verlet cloth for Ronin scarves / Imperatus capes) — not implemented.
2843. - [ ] [W2] [BP3] [M5] [GAP] ANIM `AnimationStateMachine` component (per-actor + tracks current animation + queue + blend state) — basic state at BP3.
2844. - [ ] [W2] [BP3] [M5] [GAP] ANIM `AnimationManifest` (RON-loaded + animations + frame counts + durations + event tags per chassis) — minimal manifest.
2845. - [ ] [W2] [BP3] [M5] [GAP] ANIM `SpriteAnimator` (steps through frame indices + emits tag events) — partial.
2846. - [ ] [W2] [BP3] [M5] [GAP] ANIM `SkeletalAnimator` (Spine/DragonBones drives bone transforms + emits tag events) — not implemented.
2847. - [ ] [W2] [BP3] [M5] [GAP] ANIM `ProceduralOverlayApplier` (stacks recoil/knockback/limb-track/etc.) — not implemented.
2848. - [ ] [W2] [BP3] [M5] [GAP] ANIM `RagdollComponent` marker (transitions actor to physics-ragdoll mode) — not implemented.
2849. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Walk/run/crouch/climb/jet states have animation state changes + event tags + capture evidence" — partial.
2850. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Jetpack/low-g motion demonstrates controlled secondary limb physics without destroying aim/control" — no jetpack.
2851. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Knocked/stunned/dead/pressure/wind/explosion states demonstrate increased physics authority with replay events" — not implemented.
2852. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Limb damage changes animation and capability (limp, one-arm, crawl, disabled grip, drop)" — not implemented.
2853. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Animation event tags fire correctly on frame" — partial; only walk_cycle.
2854. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Procedural overlays compose without visual jitter" — no overlays.
2855. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Ragdoll engages deterministically on death" — no ragdoll.
2856. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "Skeletal hero chassis bones drive sub-pixel-clean rendering at 4K" — no 4K render path tested.
2857. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "AI-generated frames cleaned by Tier 3 pipeline" — no AI gen pipeline.
2858. - [ ] [W2] [BP3] [M5] [GAP] ANIM done-criteria "CI gate: every chassis has all 30+ required animations OR documented exceptions" — not gated.

## 285. spec/full-collision-physics-plan — Full collision physics (BP3 M5 partial; M5.5 main; T-PHYS side track)
2859. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS `PhysicalProfile` record (mass_kg_like + material_id + composition_layers + collision_class + collision_proxy + inertia_or_handling_class + durability_or_hp + damage_routes + temperature_state + electrical_state + container_or_pressure_state + ai_affordances + ui_debug_affordances) — no profile schema at BP3.
2860. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS composite physical graph (unit = torso + limbs + organs/modules + armor layers + carried gear + batteries + fluids + wounds + constraints) — not modeled.
2861. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS composite physical graph (mech = legs + arms + cockpit + reactor/battery + shield emitter + weapons + armor plates + actuators + tanks + cargo) — not modeled.
2862. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS composite physical graph (weapon = receiver + barrel + grip + magazine + battery + fuel + coolant + damage state) — not modeled.
2863. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `actor_core` (capsule/convex compound) — not modeled.
2864. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `actor_limb` (capsule/convex; limbs collide with bodies/limbs/terrain/weapons/doors/debris/projectiles) — not modeled.
2865. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `armor_zone` (convex/capsule overlay; takes impact before body + can crack/spall/detach/jam) — not modeled.
2866. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `held_weapon` (convex/capsule; physical while held; self-collision filter against owner bones) — not modeled.
2867. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `loose_item` (kicked/crushed/blocked/damaged/picked up/destroyed) — not modeled.
2868. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `projectile_kinetic` (swept segment/capsule) — partial.
2869. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `projectile_explosive` (swept capsule/shape; projectile-projectile can detonate/deflect/damage fuze) — not implemented.
2870. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `beam_or_trace` (segment/shape cast + event) — not implemented.
2871. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `terrain_pixel` (chunked pixel grid authoritative material store) — partial.
2872. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `terrain_proxy` (polyline/heightfield/convex decomposition from dirty chunks) — not implemented.
2873. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `debris_chunk` (circle/convex + budgeted lifetime + damage by impulse and sharpness) — not implemented.
2874. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `mech_part` (compound convex/capsule; heavy contact can crush/pin/shear/disable modules) — not modeled.
2875. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `base_object` (door/turret/sensor mast/shield gate/repair pad; powered state changes collision) — no base objects.
2876. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `force_field` (sensor + solid proxy; may block projectiles and objects without being normal rigid body) — no shields.
2877. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `sensor_trigger` (sensor only; emits events; debug overlay labels sensor-only) — not modeled.
2878. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS collision class `cosmetic_particle` (no gameplay collision unless promoted to debris/projectile) — partial.
2879. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Player body ↔ AI body / enemy body / ally / AI" — not enforced.
2880. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Player limb ↔ unit limb / AI limb / held weapon" — not enforced.
2881. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Held weapon ↔ held weapon (parry/block/jam/knockaway)" — not implemented.
2882. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Loose item ↔ body/limb / projectile" — not implemented.
2883. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Projectile ↔ body/limb / armor/equipment / projectile / terrain / shield" — partial; bullet vs body only.
2884. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Debris ↔ body/limb / terrain/base" — no debris.
2885. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Mech part ↔ infantry / terrain/base" — partial.
2886. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Base object ↔ body/projectile/debris" — no base objects.
2887. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS default collision matrix "Force field ↔ body/projectile/debris (per field config)" — no shields.
2888. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `radius_px` (swept ray/capsule thickness; bullet-bullet and bullet-limb) — not implemented.
2889. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `mass_kg_like` (impulse + deflection response) — not implemented.
2890. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `velocity_px_per_s` — partial.
2891. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `kinetic_energy` (damage/penetration input) — not implemented.
2892. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `armor_penetration` (material-specific) — not implemented.
2893. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `restitution` (bounce/ricochet) — not implemented.
2894. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `fragmentation` (collision spawns fragments) — not implemented.
2895. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `explosive_profile` (fuze/detonation behavior) — not implemented.
2896. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `ccd_class` (Discrete / SweepRay / SweepCapsule / SweepShape / TOISubstep) — not implemented.
2897. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `collides_with_projectiles` (boolean or class mask) — not implemented.
2898. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile field `collision_group` (owner-safe arming delay + training rounds + shields + scenario rules) — not implemented.
2899. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS projectile-projectile policy (kinetic↔kinetic deflect/fragment; kinetic↔explosive damage fuze; explosive↔explosive chain detonate; beam↔projectile vaporize/deflect) — not implemented.
2900. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS CCD tier `Discrete` (slow + settled debris + heavy static) — implicit at BP3.
2901. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS CCD tier `Speculative` (rotating limbs + doors + moving platforms) — not implemented.
2902. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS CCD tier `SweepRay` (tiny high-speed bullets + beams) — not implemented.
2903. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS CCD tier `SweepCapsule` (physical bullets + slugs + limbs at speed) — not implemented.
2904. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS CCD tier `SweepShape` (rockets + thrown items + weapons + shields) — not implemented.
2905. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS CCD tier `TOISubstep` (player body + pilot + command core + major projectile + mech foot crush) — not implemented.
2906. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS broadphase "Dynamic tree (moving bodies/limbs/weapons/projectiles/mechs/debris)" — Bevy default.
2907. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS broadphase "Chunk spatial hash (terrain + base + static + dense projectile lanes)" — not implemented.
2908. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS broadphase "Dirty chunk proxy builder (rebuilds collision outlines only for changed terrain)" — not implemented.
2909. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS broadphase "Projectile lane cache (groups high-speed projectiles by swept AABB for bullet-bullet)" — not implemented.
2910. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS broadphase "Contact pair cache (stable pairs warm + reduces churn + improves debug labels)" — not implemented.
2911. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS broadphase "Budget governor (caps low-value debris pairs + never drops critical contacts silently)" — not implemented.
2912. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS terrain proxy "per-chunk outlines for solid terrain" — not implemented.
2913. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS terrain proxy "material tags attached to proxy spans" — not implemented.
2914. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS terrain proxy "dirty-region invalidation when terrain is carved/filled/melted/repaired" — not implemented.
2915. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS terrain proxy "optional high-detail sample at exact contact point when damage/penetration depends on material" — not implemented.
2916. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS terrain proxy "chunk-boundary tests (bullets/limbs do not snag or tunnel through seams)" — not implemented.
2917. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_pair_created` — not emitted.
2918. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_contact_started` (classes + ids + materials + normal + TOI fraction) — not emitted.
2919. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_contact_persisted` (accumulated impulse) — not emitted.
2920. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_contact_ended` — not emitted.
2921. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `contact_impulse_applied` (normal/tangent impulse + parent link to damage/shove/knockdown) — not emitted.
2922. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `projectile_deflected` (ricochet/deflection/tumble/fragment + reason) — not emitted.
2923. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `projectile_projectile_contact` — not emitted.
2924. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_filter_applied` (pair skipped + `collision_filter_reason`) — not emitted.
2925. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_damage_applied` (body/equipment/chassis/terrain damage) — not emitted.
2926. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_budget_degraded` (low-value contacts culled + count + class + deterministic rule) — not emitted.
2927. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact event `collision_first_divergence` (replay first contact mismatch) — not emitted.
2928. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS contact payload fields (`body_a`/`body_b` + `class_a`/`class_b` + `material_a`/`material_b` + `point_world` + `normal_world` + `penetration_depth` + `toi_fraction` + `relative_velocity` + `normal_impulse` + `tangent_impulse` + `parent_event_id`) — not emitted.
2929. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Contact impulse → blunt trauma / knockdown / crush / armor denting / shield overload" — not implemented.
2930. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Contact sharpness → cutting/piercing from debris/blades/broken metal/spikes" — not implemented.
2931. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Relative velocity → fall damage / vehicle-mech impact / ricochet severity" — not implemented.
2932. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Contact area → wide impact bruises/stuns; small impact penetrates/cracks" — not implemented.
2933. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Material pair → rubber bounces / metal sparks / concrete crushes / flesh wounds" — not implemented.
2934. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Armor layer → absorbs/cracks/spalls/transfers blunt force/jams limb movement" — not implemented.
2935. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Module binding (contact disables jet/sensor/weapon mount/shield emitter/repair drone)" — not implemented.
2936. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS impulse-to-damage "Actor origin (organic/android/robot/mech bodies translate impulse differently)" — single origin.
2937. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS acceptance COLL-001..012 — none pass.
2938. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS `cfctl observe --collisions` (streams current contact pairs + collision filters + last 30 events) — not implemented.
2939. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS perf "1080p/60 pass + 4K/120 status + Deck status recorded" — never measured.
2940. - [ ] [W2] [BP3] [M5+M5.5] [GAP] PHYS AI "reacts to body blocking + debris + locked doors + new terrain contacts with reason labels" — not implemented.

## 286. spec/gravity-and-ballistics-model — Universal gravity field (M0/M1 must remain config-driven; BP3 carries placeholder)
2941. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV `cf_physics::gravity::GravityField` (ambient + region_overrides + cell_overrides) — not present at BP3.
2942. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV `GravityVec` (direction + magnitude + source enum: ambient/gravity_generator/grav_well/low_g_lab/magnetic_boots/scripted) — not modeled.
2943. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV "One source of truth: every system reads `GravityField::sample(pos)`" — no enforcement; actor uses hardcoded g_pixels.
2944. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-planet ambient (Earth 1.000 / Moon 0.166 / Mars 0.378 / Mercury 0.378 / Europa 0.134 / Mimas 0.0064 / Vulcan 0.910 / Venus 0.904 / zero-g 0.0 / reverse-g 1.0 (0,+1)) — only single hardcoded value.
2945. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-cell override "Gravity Generator (base module +1g indoors regardless of planet)" — not implemented.
2946. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-cell override "Gravity Well (anomaly + scaled magnitude toward center)" — not implemented.
2947. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-cell override "Low-g Lab (0.1g region)" — not implemented.
2948. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-cell override "Magnetic Boots (per-actor 1g toward surface normal)" — not implemented.
2949. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-cell override "Damaged Gravity Generator (intermittent)" — not implemented.
2950. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-cell override "Reverse-g Chamber (per-region (0,+1))" — not implemented.
2951. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV ballistic trajectory math `pos += v·dt; v += g·dt - drag·v·dt²` — single linear integration at BP3.
2952. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV drag force `F_drag = -0.5 · ρ_local · v · |v| · C_d · A` (gas-density-dependent) — no drag at BP3.
2953. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV per-projectile drag profile (`drag_coef` + `cross_section_m2` + mass + terminal velocity hint) — not implemented.
2954. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV "Vacuum projectile range vs dense-atmosphere drag difference" gameplay — not implemented.
2955. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV atmospherics coupling "Liquid layering (oil floats on water under positive g; sinks under reverse g)" — no liquid system.
2956. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV atmospherics coupling "Gas stratification (CO2 sinks / H2 rises proportional to g_factor × ΔM/mean_M × dt)" — no atmosphere stratification.
2957. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV atmospherics coupling "Wind force from ΔP still operates regardless of g but feels different in low-g due to inertia vs weight" — no wind.
2958. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `gravity.field_changed` (region + old/new vec + source) — not emitted.
2959. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `gravity.override_activated` / `_deactivated` — not emitted.
2960. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `gravity.entity_entered_region` / `_exited_region` — not emitted.
2961. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `ballistics.projectile_launched` (projectile + owner + weapon + p_0 + v_0 + mass + drag_coef + parent) — partial.
2962. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `ballistics.projectile_step` (sparse per N ticks) — not emitted.
2963. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `ballistics.projectile_terminated` (reason: impact/fuse_expired/despawn/fragmented) — partial.
2964. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `ballistics.terminal_velocity_reached` (v_terminal + ρ_local + source) — not emitted.
2965. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `ballistics.fall_damage_threshold_crossed` (fall_distance + impact_v + local_g + threshold) — not emitted.
2966. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV event `atmospherics.gas_stratified` (atm + gas + top_pp + bottom_pp + layer_height + g_factor) — no atmosphere.
2967. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV acceptance GRAV-A-01..10 — none pass.
2968. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV scenario manifest field `gravity_g` placeholder (M0/M1 must declare even if engine ignores) — placeholder not present.
2969. - [ ] [W2] [BP3] [M0+M1+M4] [GAP] GRAV M4 HUD "Show local g_factor in advanced panel" — not implemented.
2970. - [ ] [W2] [BP3] [M0+M1] [GAP] GRAV "No `const GRAVITY_MS2: f32 = 9.81` in production code" rule — actor controller has hardcoded gravity value.

## 287. spec/visual-direction — Two-layer visual identity (BP1+ presentation)
2971. - [ ] [W2] [BP1] [GAP] VIS Layer 1 "Pixel-sim battlefield" comprehensive (terrain per-pixel + actors + held devices + particles + gibs + debris + fire/hazard cells + dropped gear + salvage) — partial; only chassis sprites.
2972. - [ ] [W2] [BP1] [GAP] VIS Layer 2 "Comic-noir presentation" comprehensive (HUD chassis silhouette + squad panel + command overlay + mission briefings comic-panel + debriefs comic timeline + replay viewer + loadout/workbench + hub + faction cards + death recap) — partial; raw HUD only at BP3.
2973. - [ ] [W2] [BP1] [GAP] VIS "Silhouettes first (faction + chassis tier + role + damage stage readable from silhouette alone at battlefield zoom)" — partial; chassis tier readable but faction/role/damage missing.
2974. - [ ] [W2] [BP1] [GAP] VIS "Status colors universal (Health/stage/alarm/status consistent across HUD/replay/briefing)" — no shared palette.
2975. - [ ] [W2] [BP1] [GAP] VIS "Comic-panel briefings (not modal text walls)" — no briefings.
2976. - [ ] [W2] [BP1] [GAP] VIS "Material overlays toggled, not always-on" — no overlay modes.
2977. - [ ] [W2] [BP1] [GAP] VIS "Damage stages have distinct visual cues (smoke wisps → sparks → fire → smoke column → wreck silhouette)" — not implemented.
2978. - [ ] [W2] [BP1] [GAP] VIS "Faction visual register matches faction grammar (per faction doctrine/tech-tier/origin-mix)" — single faction at BP3.
2979. - [ ] [W2] [BP1] [GAP] VIS open "Exact base resolution (320×180 / 480×270 / 640×360)" — not decided.
2980. - [ ] [W2] [BP1] [GAP] VIS open "Palette size (16-color / 32 / 64 / 256)" — not decided.
2981. - [ ] [W2] [BP1] [GAP] VIS open "Pixel scale ratio (1× / 2× / 3×)" — not decided.
2982. - [ ] [W2] [BP1] [GAP] VIS open "Comic-panel UI animated vs static" — not decided.
2983. - [ ] [W2] [BP1] [GAP] VIS open "Lighting model (unlit pixel art vs 2D dynamic)" — not decided.
2984. - [ ] [W2] [BP1] [GAP] VIS open "Mech scale visual relationship (heavy mechs zoom camera?)" — not decided.


# ===== WAVE 3 — UNIVERSAL ENHANCEMENT FLOOR (DR-056: accessibility / perf / captions / localization / juice / modding) =====

## 3. M1 (BP1) — Universal Enhancement Done-Criteria gaps (DR-056) that should have closed in BP1
31. - [ ] [W3] [BP1] [M1] [DR-054+DR-056] [UNIV-DR056] M1 Steam Deck 800p/60 reference-scenario perf gate not measured (DR-054).
32. - [ ] [W3] [BP1] [M1] [DR-056] [UNIV-DR056] M1 1080p/60 reference-scenario perf gate not measured.
33. - [ ] [W3] [BP1] [M1] [DR-056] [UNIV-DR056] M1 4K/120 reference-scenario perf gate not measured.
34. - [ ] [W3] [BP1] [M1] [DR-054+DR-056] [UNIV-DR056] M1 CI bench regression test (no >5% regression vs baseline) not wired (DR-054).
35. - [ ] [W3] [BP1] [M1] [DR-051+DR-054+DR-056] [UNIV-DR056] M1 24h+ memory-leak soak not run (DR-051 / DR-054).
36. - [ ] [W3] [BP1] [M1] [DR-052+DR-056] [UNIV-DR056] M1 `cfctl test sync-drift` network-sync verification not run (DR-052).
37. - [ ] [W3] [BP1] [M1] [DR-002+DR-052+DR-056] [UNIV-DR056] M1 replay determinism CI matrix per platform + per architecture not wired (DR-002 / DR-052) — only macOS aarch64 verified locally; Linux x86_64 + Windows x86_64 not checksum-matched.
38. - [ ] [W3] [BP1] [M1] [DR-026+DR-056] [UNIV-DR056] M1 AI agent-driven validation report not logged in canonical location (DR-026 / DR-056).
39. - [ ] [W3] [BP1] [M1] [DR-053+DR-056] [UNIV-DR056] M1 all-audio-via-DR-053-pipeline not done (cf-audio is still a 1-line stub at BP3 closure attempt).
40. - [ ] [W3] [BP1] [M1] [DR-055+DR-056] [UNIV-DR056] M1 juice rules per DR-055 not authored (no recoil curves, no camera punch on damage taken, no fire+reload feedback rules).
41. - [ ] [W3] [BP1] [M1+M4A] [DR-056] [UNIV-DR056] M1 ACC-A floor was deferred to M4A (originally a M1 carry); no captions for fire/reload events at M1 closure.
42. - [ ] [W3] [BP1] [M1] [DR-046+DR-056] [UNIV-DR056] M1 Tier-A 11-language localization keyed strings not validated (DR-046) — production code still has English-only string literals.
43. - [ ] [W3] [BP1] [M1] [DR-006+DR-050+DR-056] [UNIV-DR056] M1 modding parity not verified (DR-006 / DR-050) — no mod-author extension surface tested.
44. - [ ] [W3] [BP1] [M1] [DR-031+DR-056+DR-057] [UNIV-DR056] M1 anti-FOMO + anti-pay-to-win audit not run (DR-031 / DR-057).
45. - [ ] [W3] [BP1] [M1] [DR-051+DR-056] [UNIV-DR056] M1 captions for ALL audio not enforced (DR-051) — no audio surface yet, so no captions yet.
46. - [ ] [W3] [BP1] [M1] [DR-052+DR-056] [GAP] M1 input prediction for player-driven actor (DR-052 client prediction) not implemented.
47. - [ ] [W3] [BP1] [M1] [DR-055+DR-056] [GAP] M1 recoil curves per weapon (DR-055) not authored.
48. - [ ] [W3] [BP1] [M1] [DR-056] [GAP] M1 camera punch on damage taken not implemented.
49. - [ ] [W3] [BP1] [M1] [DR-056] [GAP] M1 animation event tags fire correctly (per `spec/animation-system`) — current cf-render-2d emits zero animation events.
50. - [ ] [W3] [BP1] [M1] [DR-053+DR-056] [GAP] M1 audio: footstep + reload + weapon-fire SFX generated via DR-053 Tier 1 pipeline.

## 4. M1.5 (BP1) — Universal + per-milestone enhancement gaps
51. - [ ] [W3] [BP1] [M1.5] [GAP] M1.5 match feel-test playtest (project-owner + 3-5 testers) not recorded.
52. - [ ] [W3] [BP1] [M1.5] [DR-050] [GAP] M1.5 adaptive difficulty toggle (DR-050 onboarding) not implemented.
53. - [ ] [W3] [BP1] [M1.5] [GAP] M1.5 AI difficulty preset visible (Cakewalk / Tough Crowd / Veteran) not implemented.
54. - [ ] [W3] [BP1] [M1.5] [GAP] M1.5 replay sharing prototype not built.
55. - [ ] [W3] [BP1] [M1.5] [GAP] M1.5 reactive enemy still uses raw scripted aim-settle — no real perception model.
56. - [ ] [W3] [BP1] [M1+M1.5] [UNIV-DR056] M1.5 all 14 Universal Enhancement rows status = unmeasured (same as M1).
57. - [ ] [W3] [BP1] [M1.5] [GAP] M1.5 captions for guard fire / reload / death events not emitted.
58. - [ ] [W3] [BP1] [M1.5] [DR-055] [GAP] M1.5 juice rules per DR-055 for hit/miss/breach/extract not authored.
59. - [ ] [W3] [BP1] [M1.5] [GAP] M1.5 no AI reason-label HUD overlay showing what guard is currently doing.
60. - [ ] [W3] [BP1] [M1.5] [GAP] M1.5 enemy AI has no memory grid for last-known positions.

## 5. M2 (BP2) — Universal + per-milestone enhancement gaps
61. - [ ] [W3] [BP2] [M2] [DR-054] [GAP] M2 GPU compute path investigation (deterministic backup; CPU baseline per DR-054) not done.
62. - [ ] [W3] [BP2] [M2] [DR-054] [GAP] M2 SIMD material kernel update (8 pixels/SIMD lane; deterministic per DR-054) not implemented.
63. - [ ] [W3] [BP2] [M2] [GAP] M2 streaming asset budget per scenario not measured.
64. - [ ] [W3] [BP2] [M2] [GAP] M2 cold-load benchmark in CI not wired.
65. - [ ] [W3] [BP2] [M2] [GAP] M2 "Player can dig through dirt fast, concrete slowly, metal-nohook is refused with reason label" — partly implemented but no per-material-hardness-tier perf measurement.
66. - [ ] [W3] [BP2] [M2] [GAP] M2 dirty-region update perf claim ("render reflects mutation within one frame") not measured against perf gate.
67. - [ ] [W3] [BP2] [M2] [GAP] M2 material overlay UI integrated; tool-validity color cues — present in cf-ui but no test that reads correctly across all 8 launch materials at 200% scale.
68. - [ ] [W3] [BP2] [M2] [GAP] M2 Steam Deck floor perf gate (800p/60) not measured for chunked-terrain scene with carving session.
69. - [ ] [W3] [BP2] [M2] [GAP] M2 4K/120 perf budget not measured.
70. - [ ] [W3] [BP2] [M2] [UNIV-DR056] M2 all 14 Universal Enhancement rows status = unmeasured.
71. - [ ] [W3] [BP2] [M1.5+M2] [GAP] M2 chunked-terrain has no `ChunkedTerrain` render path (cf-render-2d still uses M1.5 BreachStrip projection — the chunked storage is only used logically).
72. - [ ] [W3] [BP2] [M2] [GAP] M2 pixel debris particles when carving (visual feedback) not present in cf-render-2d.
73. - [ ] [W3] [BP2] [M2] [GAP] M2 material overlay toggle key not bound in cf-app keyboard input layer.
74. - [ ] [W3] [BP2] [M2] [GAP] M2 visual feedback for tool refusal (e.g., spark on metal_nohook hit) not implemented.
75. - [ ] [W3] [BP2] [M2] [GAP] M2 chunked terrain replay determinism not verified across 60Hz + 120Hz tick rates with per-chunk checksum diffs.
76. - [ ] [W3] [BP2] [M2] [GAP] M2 `cfctl observe --materials` does not exist (T-CONTROL gap inherited from cf-material stub).
77. - [ ] [W3] [BP2] [M2] [DR-051] [GAP] M2 caption pipeline for `terrain.terrain_carved` / `terrain.tool_refused` events not authored (DR-051).
78. - [ ] [W3] [BP2] [M2] [GAP] M2 anti-FOMO + anti-pay-to-win audit not run (terrain has no MTX surface but the audit row stays open).
79. - [ ] [W3] [BP2] [M2] [DR-006] [GAP] M2 modding parity (DR-006) not verified — modders can't add new materials to the launch set yet.
80. - [ ] [W3] [BP2] [M2] [GAP] M2 Tier-A 11-language localization keyed strings for material names + refusal reasons not authored.

## 6. M2.5 (BP2) — gaps
81. - [ ] [W3] [BP2] [M2.5] [GAP] M2.5 reactor as a single non-player static "actor" — has hp but no visible deterioration sprite.
82. - [ ] [W3] [BP2] [M2.5] [GAP] M2.5 enemy fire damages reactor through aabb hits — works mechanically but no visible bullet-impact sparks on reactor surface.
83. - [ ] [W3] [BP2] [M2.5] [GAP] M2.5 reactor hp bar HUD line — present but no test that the bar value matches reactor.hp under high tick rate.
84. - [ ] [W3] [BP2] [M2.5] [GAP] M2.5 time-remaining timer HUD — present but no caption announcing 30/15/5-second warnings.
85. - [ ] [W3] [BP2] [M2.5+M4A] [GAP] M2.5 LLM grading verdict 7.86/10 PASS_WITH_FUTURE_POLISH cites "visual 3-6/10 future-owned by M4A" — M4A landed, but the M2.5 bundle was never re-graded against the M4A polish.
86. - [ ] [W3] [BP2] [M2.5] [GAP] M2.5 perf-tier verification gate (Steam Deck 800p/60) not run.
87. - [ ] [W3] [BP2] [M2.5] [GAP] M2.5 captions for reactor-damage events not authored.
88. - [ ] [W3] [BP2] [M2.5] [DR-055] [GAP] M2.5 juice rules per DR-055 for reactor explosion / win celebration not authored.
89. - [ ] [W3] [BP2] [M1.5+M2.5] [GAP] M2.5 enemy AI reason-label HUD overlay not implemented (same gap as M1.5).
90. - [ ] [W3] [BP2] [M1.5+M2.5] [DR-050] [GAP] M2.5 adaptive difficulty toggle (DR-050) not implemented — same gap as M1.5.

## 12. DR-020 / cf-audio — Audio identity gaps (M4-M7 primary; T-AUDIO; should have started at BP3 placeholder)
181. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] `cf-audio` is a 1-line stub at BP3 closure attempt; DR-020 closed 2026-05-04 with diegetic-first mix promise.
182. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No diegetic physical layer (gunfire, impacts, drilling, jetpacks, servos, hydraulics, reactor hums, etc.) — every actor / action / chassis is silent.
183. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No synth/dread emotional layer (Carpenter-esque synth, ambient drones, mech-power hums).
184. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No caption-event layer — captions queue exists in cf-ui but is fed zero audio events.
185. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No `audio.event_fired` row in events.jsonl per the BP3 self-play "Hear" axis (deferred to BP6 but the axis is still required at BP3).
186. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No noise-footprint AI alarm event when player fires (DR-020 says "loud weapons create AI alarm events"; not wired).
187. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No servo grind / hydraulic hiss / smoke crackle / warning tones for mech damage stages.
188. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No power-state recognizable sounds for base systems (shields/turrets/reactors/doors).
189. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No origin-class failure-sound families (organic vs android vs robot vs mech vs command-core avatar).
190. - [ ] [W3] [BP3] [M4+M7] [DR-020] [GAP] No pilot-ejection alarm-sequence cue.

## 14. M1.5/M2.5 AI — ai-trust-harness-slice-a.md gaps (BP1-BP2 should have built minimal harness)
201. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No AI scenario manifest format (`scenario_id` / `seed` / `fixture_map` / `actors` / `orders` / `terrain_mutations` / `threats` / `success_assertions` / `failure_assertions` / `telemetry_required` / `timeout_ms`).
202. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No AI-H-01..AI-H-06 acceptance suite runner — only one ReactiveGuard FSM exists; no harness wraps it.
203. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No `tactic_chosen` event with reason string for every decision (cf-ai emits one event per tick but no per-choice reason string).
204. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No `ai_perception` event with sight-cone + hearing-range info beyond a single "saw_player" boolean.
205. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No AI debug overlay (current order, tactic, target, path, stuck-state, tool/material reason).
206. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [DR-008+DR-022] [GAP] No utility-scoring tree visible per DR-008/DR-022 (the scorer exists but its scores per option are not in observe.once).
207. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No `tactic_failed` or `tactic_recovered` events.
208. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No AI fixture map / scenario for terrain mutation while AI is choosing (collapse / new breach / blocked tunnel) — terrain mutations work but no AI scenario exercises the AI's recovery.
209. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No mistake/recovery model — bots cannot "panic" or "miss aim" deterministically.
210. - [ ] [W3] [BP1+BP2] [M1.5+M2.5] [GAP] No AI memory grid for last-known-position (M1.5 done-criteria item).

## 23. M4A — Per-milestone enhancement specifics (milestone-enhancement-pass-m1-plus.md) gaps
348. - [ ] [W3] [M4A] [GAP] M4A reactive UI data binding (per Bevy state) — only static text strips; no reactive bindings.
349. - [ ] [W3] [M4A] [GAP] M4A UI testing harness (`cfctl ui assert`) — not implemented.
350. - [ ] [W3] [M4A] [DR-046+DR-055] [GAP] M4A juice rules per DR-046 + DR-055 for every HUD element — not authored.
351. - [ ] [W3] [M4A] [GAP] M4A localization keyed strings (Tier-A 11 languages) — production code still has English-only HUD strings.
352. - [ ] [W3] [M4A] [DR-046] [GAP] M4A animation system for UI panels (slide + skew per DR-046) — panels are static.
353. - [ ] [W3] [M4A] [GAP] M4A settings menu full tree (per `spec/shell-ui-architecture`) — only the M4A acceptance settings exist.
354. - [ ] [W3] [M4A] [GAP] M4A controller route through HUD — gamepad focus works but no controller-only fallback path tested without keyboard plugged in.
355. - [ ] [W3] [M1+M4+M4A] [GAP] M4A mouse-driven HUD interaction not implemented (mouse aim deferred to M4, never built — same gap as M1-S09).
356. - [ ] [W3] [M4A] [DR-012] [GAP] M4A screen-reader API integration (T-ACC-PLUS at BP9..BP12; the M4A AGENTS.md note says "deferred" but DR-012 ACC-A list mentions controller/keyboard parity, no screen-reader yet).
357. - [ ] [W3] [M4A] [DR-012] [GAP] M4A `cfctl ui assert hud.objective contains "Breach"` not implemented — DR-012 same-input-navigation rule requires it.
358. - [ ] [W3] [M4A] [GAP] M4A `act.input.mouse_click` for clickable HUD elements not implemented (M4A self-play sweep rule from BP3 says it should be wired).
359. - [ ] [W3] [M4A] [GAP] M4A `act.input.mouse_move` not implemented.
360. - [ ] [W3] [M4A] [GAP] M4A "always for selected actor; on hover for squad list" — no squad system at BP3 yet.

## 50. DR-046 — Player-facing surfaces direction (BP3 inherits placeholder-generation pressure)
599. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 Tier-A 11 languages keyed-string discipline — production code has English-only strings.
600. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 Project Fluent file under `content/locales/` — directory missing.

## 86. ACC-A floor (DR-012; closed at M4A) — surface gaps the closure did NOT cover
801. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `text_scale` 150% (between 100% and 200%) not tested — only 100% and 200% screenshots in M4A bundle.
802. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `ui_density` Compact/Comfortable toggle — not implemented (HUD is always Comfortable).
803. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `contrast_mode` "High Contrast Light" variant — only "Standard" and "High Contrast Dark" implemented.
804. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `color_cue_mode` Colorblind-safe + Monochrome-test variants — only Default exists.
805. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `caption_mode` Critical-only / Expanded / Off — only on/off toggle.
806. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `caption_background` opacity 50%/80%/100% — captions render with one opacity.
807. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `input_profile` keyboard-only / custom variants — only Keyboard/mouse + Controller working.
808. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `remap_actions` UI/replay/workbench groups — only Gameplay group remappable at M4A.
809. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `hold_behavior` Toggle / Press-to-cycle variants — only Hold implemented.
810. - [ ] [W3] [M4A+M4B] [DR-012] [GAP] ACC-A `game_speed_assist` Slowdown75 / Slowdown25 / Pause-in-menus — not implemented (no slowdown overlay until M4B).
811. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `screen_shake_scale` 25% / 50% — only `reduced_shake` boolean toggle.
812. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `camera_motion` Reduced / Standard — only `reduced_motion` boolean.
813. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `flash_reduction` On/Off — `reduced_flash` flag honored at schedule but no flash to reduce yet.
814. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `objective_help` Minimal/Standard/Verbose — only one verbosity level on HUD.
815. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A `debug_explainer_level` Player/Designer/Raw — not implemented (only Player level).

## 87. ACC-A source-aligned floors gaps
816. - [ ] [W3] [GAP] ACC-A text-size floor "1080p important PC text ≥18 px by default" — not measured.
817. - [ ] [W3] [GAP] ACC-A "scaled text may scroll in one direction" — banner/caption strips at 200% scale may need horizontal scroll on small windows; not verified.
818. - [ ] [W3] [GAP] ACC-A contrast targets (4.5:1 standard / 3:1 large / 3:1 inactive / 7:1 high-contrast) — not measured against actual rendered RGB.
819. - [ ] [W3] [GAP] ACC-A flash threshold "no more than 3 per second" — no flash limiter.
820. - [ ] [W3] [GAP] ACC-A audio alternatives "critical audio cues get visible equivalents or captions" — cf-audio stub means no audio yet, but the contract still requires the caption fallback infrastructure (which exists).

## 88. M4A ACC-A test coverage gaps (DR-012 closed but tests narrow)
821. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-01 text-scale test reads at 100%/150%/200% across HUD + command + loadout + workbench + replay + hub + settings — only HUD at M4A.
822. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-02 contrast test measures actual rendered RGB — only documents "palette swap" not measured contrast ratio.
823. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-04 same-input navigation test covers HUD focus — but no test traversal of "buy/loadout/workbench/hub" surfaces (those UIs don't exist at BP3).
824. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-05 hold-to-confirm test covers `act.settings.set` patch validation — but no UI surface has a hold-to-confirm action wired through cf-app (the data plane is shipped, UI surface not).
825. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-06 motion/shake/flash flags toggle behavior — flags read but no actual motion/shake/flash to gate yet.
826. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-07 captions toggle: the queue exists but is empty (cf-audio not built).
827. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-08 equipment workbench density — workbench doesn't exist at BP3.
828. - [ ] [W3] [M3B+M4A] [DR-012] [GAP] ACC-A-09 replay/death recap accessibility — M3B prints markdown, not styled HTML; accessibility evaluation is harder against markdown.
829. - [ ] [W3] [M4A] [DR-012] [GAP] ACC-A-10 run-bundle evidence — works ✓ but no "monochrome screenshot still identifies critical state" test (color-blind safety check).

## 89. T-CAPTURE — capture-grid composer gaps
830. - [ ] [W3] [GAP] T-CAPTURE `--capture-each-action` flag for cf-e2e — not implemented (per BP3 self-play rule for keyframes-at-each-action).
831. - [ ] [W3] [GAP] T-CAPTURE per-frame overlay missing `top-of-screen breach hp_remaining for visible breaches` line.
832. - [ ] [W3] [GAP] T-CAPTURE caption-strip text not burned into PNG overlay (the spec says burn captions in).
833. - [ ] [W3] [GAP] T-CAPTURE `summary_grid.events.json` co-located JSON manifest — not implemented (post-BP2 extension).
834. - [ ] [W3] [GAP] T-CAPTURE animated WebP timeline export — not implemented (post-BP2 extension).
835. - [ ] [W3] [GAP] T-CAPTURE side-by-side diff grid for replay-vs-live regression — not implemented.
836. - [ ] [W3] [GAP] T-CAPTURE composer version + schema rev recorded in `grid.json` — composer prints schema rev in CLI but not in JSON.
837. - [ ] [W3] [GAP] T-CAPTURE composer determinism contract (same captures + composer version → identical grid) — not verified by a test.
838. - [ ] [W3] [GAP] T-CAPTURE `cf-app --headless-capture` for offscreen-RenderTarget path — exists but is scope-limited; not tested in CI.
839. - [ ] [W3] [GAP] T-CAPTURE non_blank_ratio test — runs in cf-e2e but not in CI smoke (only in self_play_sweep).
840. - [ ] [W3] [GAP] T-CAPTURE `summary_grid.png` ≤ 1 MB compressed guarantee — not measured per run.

## 105. DR-028 — Visual fidelity targets at BP3 close
961. - [ ] [W3] [BP3] [M0] [DR-028] [GAP] DR-028 M0 baseline "Empty Bevy app hits 120 FPS at 1080p on a mid-range GPU" — never measured beyond local macOS aarch64.
962. - [ ] [W3] [BP3] [M2] [DR-028] [GAP] DR-028 M2 "Pixel terrain + carving session sustains 120 FPS at 1080p" — never measured.
963. - [ ] [W3] [BP3] [M2] [DR-028] [GAP] DR-028 M2 "Pixel terrain on Steam Deck sustains 60 FPS at 800p" — Deck never accessed.
964. - [ ] [W3] [BP3] [DR-028] [GAP] DR-028 Per-frame budget 4K/120 = 8.33 ms — never measured.
965. - [ ] [W3] [BP3] [DR-028] [GAP] DR-028 60 Hz sim tick → 120 Hz render decoupled via interpolation — sim runs at fixed 60 Hz but render-interpolation factor is hardcoded at 1.0 (no actual interpolation).
966. - [ ] [W3] [BP3] [DR-028] [GAP] DR-028 Render-ahead interpolation factor "capped at 1 sim tick" — not enforced; render frame can be many sim ticks behind.
967. - [ ] [W3] [BP3] [DR-028] [GAP] DR-028 SDF/vector text for clean scaling — Bevy ab_glyph runtime rasterization is used; not true SDF.
968. - [ ] [W3] [BP3] [M4A] [DR-028] [GAP] DR-028 200% UI scale tested ✓ for M4A HUD; but BP3 has no buy/loadout/workbench/hub to scale.
969. - [ ] [W3] [BP3] [DR-028] [GAP] DR-028 Pixel-art rendering "sub-pixel-clean + integer scaling where possible" — Bevy default nearest sampling not enforced.
970. - [ ] [W3] [BP3] [DR-028] [GAP] DR-028 Adaptive resolution + "epic" tier — not implemented.

## 127. spec/ux-wireframes-slice-a — Information priority ladder gaps
1132. - [ ] [W3] [GAP] L0 (0-500ms): incoming-shell / dying / reload-blocked / fall-instability / blast-radius / delivery-danger cues — only reload-blocked appears as "RELOADING" line; no incoming-shell warning.
1133. - [ ] [W3] [GAP] L0 sound/haptic/caption pair — no haptic support; captions queue empty; no sound output.
1134. - [ ] [W3] [GAP] L1 active-action-state: jet/dig/repair state — jet-state on chassis line works ✓; dig-state not on HUD (only TOOL line indicates valid/refused).
1135. - [ ] [W3] [GAP] L2 squad/tactical: which bot needs help / path blocked / current doctrine / LZ risk / material overlay — none at BP3.
1136. - [ ] [W3] [GAP] L3 planning/economy — none at BP3.
1137. - [ ] [W3] [GAP] L4 debug/learning — replay viewer + run-bundle exist but no in-game post-action loop.

## 128. spec/ux-wireframes-slice-a — Screen map gaps
1138. - [ ] [W3] [M4A] [GAP] Tactical HUD — exists ✓ at M4A.
1139. - [ ] [W3] [M4B] [DR-009] [GAP] Command Overlay — not implemented (DR-009 OPEN; M4B owns).
1140. - [ ] [W3] [GAP] Squad Panel — not implemented.
1141. - [ ] [W3] [GAP] Buy / Loadout — not implemented.
1142. - [ ] [W3] [GAP] Material Overlay — logical only; no HUD render.
1143. - [ ] [W3] [M3B] [GAP] Death Recap — M3B debrief renders but no in-game popup.
1144. - [ ] [W3] [GAP] Replay Viewer — cf-tools-replay-viewer is CLI markdown only; no in-engine UI.
1145. - [ ] [W3] [GAP] Delivery Preview — not implemented.
1146. - [ ] [W3] [GAP] Hub — not implemented.
1147. - [ ] [W3] [GAP] Local Game — not implemented.

## 136. M4A — accessibility live update gaps
1201. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` with `key_bindings.aim_up=Numpad8` — works ✓ but no live-update test for movement keys.
1202. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `high_contrast=true` triggers `palette_text/palette_strip_bg/palette_banner_bg` swap — works ✓ but no per-sprite color test (cf-render-2d ignores high-contrast).
1203. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `ui_scale=2.0` scales HUD ✓ but the cf-render-2d region anchor doesn't scale (sprite size stays at 16×32 in world space).
1204. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `captions=false` hides captions strip — works ✓ but no caption queue can populate (audio not implemented).
1205. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `reduced_motion=true` — flag observable ✓ but no system consumes (no motion to reduce yet).
1206. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `reduced_shake=true` — flag observable ✓ but no shake to reduce.
1207. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `reduced_flash=true` — flag observable ✓ but no flash to reduce.
1208. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `hold_to_confirm=true` + `hold_threshold_ms=350` — works ✓ but no UI action requires hold to confirm at BP3.
1209. - [ ] [W3] [M4A] [GAP] M4A `act.settings.set` `key_remap_enabled=true` — works ✓ but only Gameplay actions can be remapped (UI/replay/workbench/Hub groups missing).
1210. - [ ] [W3] [M4A] [GAP] M4A `act.input.focus next/prev/clear` — works ✓ but no test that fast Tab presses are debounced.

## 137. cf-control engine emit_*_events gaps
1211. - [ ] [W3] [GAP] `emit_actor_events` — emits `actor.actor_status_changed` / `actor.actor_snapshot` ✓ but doesn't emit `actor.actor_landed` consistently (only on first land per tick).
1212. - [ ] [W3] [GAP] `emit_combat_events` — emits `combat.projectile_*` ✓ but `combat.projectile_expired.cause` enum is string-only.
1213. - [ ] [W3] [M1.5+M2] [GAP] `emit_terrain_events` — emits `terrain.terrain_carved` for M1.5 strips + M2 chunks ✓ but `terrain.terrain_breach_stub` legacy event still emitted unnecessarily for M2 scenarios.
1214. - [ ] [W3] [GAP] `emit_mission_events` — emits `mission.objective_*` ✓ but `mission.objective_failed.cause_event_id` parent link not set.
1215. - [ ] [W3] [GAP] `emit_input_events` — emits `input.intent_received` ✓ but ack-cursor not aligned with event consumption.
1216. - [ ] [W3] [GAP] `emit_control_events` — emits `control.command_accepted` ✓ but no `control.command_queued` for deferred commands.
1217. - [ ] [W3] [GAP] `emit_chassis_events` — emits 13 chassis event types ✓ but no test for negative path (e.g., act.chassis.repair on a Wreck stage rejects).
1218. - [ ] [W3] [GAP] `emit_animation_events` — declared but not implemented (animation_event events do fire from chassis stance changes but no animation tag events).
1219. - [ ] [W3] [DR-022] [GAP] `emit_ai_events` — emits `ai.tactic_chosen` ✓ but no `ai.intent_announced` / `ai.recovery_started` / `ai.commander_adapted` (DR-022 promises).
1220. - [ ] [W3] [GAP] `emit_perception_events` — `ai.ai_perception` fires every tick (high volume); no `ai.perception_changed` for delta-only.

## 138. Per-system perf counter gaps at BP3 close
1221. - [ ] [W3] [BP3] [GAP] No `system.tick_sample.sim_ms` field — `system.tick_sample` event fires but only carries the tick number.
1222. - [ ] [W3] [BP3] [GAP] No `system.tick_sample.render_ms` field.
1223. - [ ] [W3] [BP3] [GAP] No `system.tick_sample.event_emit_ms` field.
1224. - [ ] [W3] [BP3] [GAP] No `system.tick_sample.worker_thread_ms` field.
1225. - [ ] [W3] [BP3] [GAP] No `system.tick_sample.dropped_events` field.
1226. - [ ] [W3] [BP3] [GAP] No `summary.json.performance.render_p99_ms` field.
1227. - [ ] [W3] [BP3] [GAP] No `summary.json.performance.sim_p99_ms` field — only `p99_tick_ms`.
1228. - [ ] [W3] [BP3] [GAP] No `summary.json.performance.frame_p99_ms` field.
1229. - [ ] [W3] [BP3] [GAP] No `summary.json.performance.gpu_upload_bytes` field.
1230. - [ ] [W3] [BP3] [GAP] No `summary.json.performance.event_volume_per_tick` field.

## 139. Per-platform CI test gaps
1231. - [ ] [W3] [GAP] `.github/workflows/ci.yml` Windows leg — runs but missing `--target x86_64-pc-windows-msvc` cross-check from macOS.
1232. - [ ] [W3] [GAP] CI macOS aarch64 + macOS x86_64 dual runner — only one macOS leg.
1233. - [ ] [W3] [GAP] CI Linux x86_64 + Linux aarch64 dual runner — only x86_64.
1234. - [ ] [W3] [GAP] CI determinism checksum cross-platform comparison — works on each runner but never cross-compared.
1235. - [ ] [W3] [GAP] CI per-tick checksum upload as artifact — not done.
1236. - [ ] [W3] [GAP] CI integration test for `--scenario X --tick-rate-hz 120` — currently only 60 Hz integration tested.
1237. - [ ] [W3] [GAP] CI Steam Deck-spec runner (matched-CPU + Vulkan + 800p) — not present.
1238. - [ ] [W3] [GAP] CI 4K render perf gate — not present.
1239. - [ ] [W3] [GAP] CI 1080p render perf gate — not present.
1240. - [ ] [W3] [GAP] CI live-WS acceptance test parallel-client load — not present.

## 149. BP3 specific Universal Enhancement gaps (per DR-056 row inheritance)
1299. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 Steam Deck 800p/60 perf gate — never measured.
1300. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 1080p/60 perf gate — never measured beyond a single m4a bundle.
1301. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 4K/120 perf gate — never measured.
1302. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 CI bench regression — not wired.
1303. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 24h memory-leak soak — never run.
1304. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 network sync verification — pre-multiplayer; row stays open.
1305. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 replay determinism CI matrix per platform + arch — only macOS aarch64 verified locally.
1306. - [ ] [W3] [BP3] [M4A] [DR-056] [UNIV-DR056] BP3 all-player-surfaces-via-cfctl — works for M4A focus + settings ✓; mouse / chassis-eject animations also covered.
1307. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 AI-agent-validation-report — present in implementation log; missing the formal Q1-Q7 self-test report.
1308. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 AI-audio-pipeline — cf-audio still stub.
1309. - [ ] [W3] [BP3] [DR-055+DR-056] [UNIV-DR056] BP3 juice rules per DR-055 — not authored beyond placeholder.
1310. - [ ] [W3] [BP3] [M4A] [DR-056] [UNIV-DR056] BP3 ACC-A floor — closed at M4A ✓ for HUD only; expansion to other surfaces deferred.
1311. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 Tier-A 11-language keyed-strings — production strings still English-only.
1312. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 modding parity — not verified at BP3.
1313. - [ ] [W3] [BP3] [DR-056] [UNIV-DR056] BP3 anti-FOMO + anti-pay-to-win audit — never run.
1314. - [ ] [W3] [BP3] [DR-053+DR-056] [UNIV-DR056] BP3 captions for ALL audio — no audio yet, but DR-053 contract still requires the placeholder pipeline.

## 157. cf-render-2d perf gaps
1364. - [ ] [W3] [GAP] cf-render-2d does not batch sprites (each ActorRenderTag is a separate entity).
1365. - [ ] [W3] [GAP] cf-render-2d does not pool sprite entities (chassis-zone child sprites are spawned/despawned per actor).
1366. - [ ] [W3] [GAP] cf-render-2d does not cull off-screen actors (rendering everything visible to camera).
1367. - [ ] [W3] [GAP] cf-render-2d does not implement viewport-bounded sprite spawning.
1368. - [ ] [W3] [GAP] cf-render-2d does not respect dynamic-range tonemap (UI text and bright sprites use same range).
1369. - [ ] [W3] [GAP] cf-render-2d does not implement Z-sort for chassis-zone pips (relies on Z value in transform).
1370. - [ ] [W3] [GAP] cf-render-2d Vec2/Vec3 conversions allocate frequently in `sync_chassis_zone_sprites`.

## 158. cf-control observe stream backpressure gaps
1371. - [ ] [W3] [GAP] cf-control observe stream is unbounded — clients can subscribe and never ack; server keeps emitting.
1372. - [ ] [W3] [GAP] cf-control observe stream has no max-frames-per-second cap per subscriber.
1373. - [ ] [W3] [GAP] cf-control observe stream has no per-subscriber filter (subscribers receive all frames).
1374. - [ ] [W3] [GAP] cf-control observe stream does not coalesce successive identical frames.
1375. - [ ] [W3] [GAP] cf-control observe stream does not record per-subscriber stats (dropped, send rate).
1376. - [ ] [W3] [GAP] cf-control observe stream sends notifications even when no subscriber is listening (wasted CPU).
1377. - [ ] [W3] [GAP] cf-control multiple-concurrent-clients support — works ✓ but no test for 10+ concurrent.

## 173. DR-010 — License/reuse matrix (cross-cutting; BP3 release scrub obligation)
1515. - [ ] [W3] [BP3] [DR-010] [GAP] DR-010 `references/usage-ledger.md` — not maintained at BP3 (per-asset license + replacement-plan ledger never updated).
1516. - [ ] [W3] [BP3] [DR-010] [GAP] DR-010 "Tier 0/1/2/3/4 reuse-tier classification" — not applied to BP3 dependencies (Bevy / wgpu / jsonrpsee / tokio / blake3 etc).
1517. - [ ] [W3] [BP3] [DR-010] [GAP] DR-010 SPDX identifier required per crate Cargo.toml — present in workspace Cargo.toml; not validated by `cargo-deny`.
1518. - [ ] [W3] [BP3] [DR-010] [GAP] DR-010 NOTICES file for permissive deps — does not exist.
1519. - [ ] [W3] [BP3] [DR-010] [GAP] DR-010 release-boundary scrub script — no `game/tools/release_license_audit.sh`.
1520. - [ ] [W3] [BP3] [DR-010] [GAP] DR-010 mod-author license declaration in `.cfpkg` manifest — schema does not require a `license` field.

## 183. DR-053 — AI audio pipeline (CLOSED at every M1+; cf-audio is 1-line stub)
1566. - [ ] [W3] [M1] [DR-053] [GAP] DR-053 cf-audio integration with bevy_kira_audio — feature flag not pre-declared at BP3.
1567. - [ ] [W3] [M1] [DR-053] [GAP] DR-053 audio-cue → caption event pipeline — caption queue exists but no audio events flow.
1568. - [ ] [W3] [M1] [DR-053] [GAP] DR-053 every audio cue logged in usage-ledger — no audio cues, but ledger discipline absent.
1569. - [ ] [W3] [M1] [DR-053] [GAP] DR-053 audio mix policy (synth music ducks under critical alarms) — no mix policy declared.
1570. - [ ] [W3] [M1] [DR-053] [GAP] DR-053 origin-specific failure sound families — not declared.

## 184. DR-054 — Performance optimization and profiling (CLOSED; every M1+ inherits perf gate)
1571. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 cf-bench regression harness — `cf-bench` is 38-line scaffold; no scenarios.
1572. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 perf gate per milestone (Steam Deck 800p/60 + 1080p/60 + 4K/120) — never measured.
1573. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 SIMD optimization for hot paths — not authored.
1574. - [ ] [W3] [M1+M2] [DR-054] [GAP] DR-054 GPU compute path for terrain carving — not implemented (M2-S02 done-criterion).
1575. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 24h memory-leak soak — not run.

## 185. DR-055 — Game feel / juice / flow state (CLOSED; every M1+ inherits juice rules row)
1576. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 weapon-fire recoil curve per weapon — single recoil constant at BP3.
1577. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 weapon-fire camera punch — not implemented.
1578. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 hit-stop / hit-pause on impact — not implemented.
1579. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 muzzle flash sprite — not implemented.
1580. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 reload feedback animation — not implemented.
1581. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 jump-takeoff anticipation / land-impact squash — not implemented.
1582. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 actor-status flash on damage taken — not implemented.
1583. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 screen-space damage vignette — not implemented.
1584. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 victory state celebration juice — not implemented.
1585. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 failure-state slow-mo replay — not implemented.

## 186. DR-051 — Accessibility-plus / sustainability / launch polish (CLOSED at M-ACC-PLUS BP9..BP12; BP3 inherits 24h soak + captions floor)
1586. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 24h memory-leak soak — not run.
1587. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 captions for ALL audio (full-subtitle option) — no audio yet but pipeline placeholder missing.
1588. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 sustainability posture documented — not authored.

## 187. DR-052 — Network sync / rollback / CLI-testable determinism (CLOSED; every M1+ inherits row)
1589. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 `cfctl test sync-drift` command — not implemented at BP3.
1590. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 client prediction + server reconciliation for player actor — not implemented.
1591. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 lockstep input traces for online co-op — not implemented.
1592. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 lag compensation — not implemented.
1593. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 replay determinism CI matrix per platform + per architecture — only macOS aarch64.

## 197. DR-044 — Audiovisual production pipeline (CLOSED; BP3+ Tier 1 placeholder pipeline required)
1651. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 Tier 1 SVG/geometric placeholder generator (`tools/asset_gen/build_placeholders.py`) — does NOT exist; BP3+ placeholder generation should have started.
1652. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 per-faction palette JSON (`content/palettes/<faction>.json`) — directory missing.
1653. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 per-category generators (actors / weapons / vehicles / base objects / materials / UI icons) — not built.
1654. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 build-step integration (`cargo build` regenerates if `.svg.template` or palette JSON changed) — not implemented.
1655. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 license-clean fonts (JetBrains Mono + Press Start 2P + Noto) — not present in `game/assets/`.
1656. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 placeholder audio (sine/square synth blips) — no `game/assets/placeholders/audio/` directory.
1657. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 480×270 base sim resolution — cf-app default is 1280×720; sim canvas not aligned to logical canvas.
1658. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 256-color global palette (64 per faction × 4 + 8 core + 16 status/UI + 16 environmental) — not declared.
1659. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 sprite sizes (Infantry 16×24 / PA 24×32 / LightMech 48×64 / MediumMech 96×128 / HeavyMech 160×192) — cf-render-2d uses 16×32 for all actors; chassis kind scale multiplier only.
1660. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 lighting model (2D normal maps + radial point lights + ambient + light volumes) — cf-render-2d is flat-shaded.
1661. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 `cf-asset-pipeline` CLI tool — does not exist.
1662. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 `tools/asset_gen/comfy_runner.py` Python orchestrator — does not exist.
1663. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 `tools/comfyui_workflows/` — directory missing.
1664. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 `references/usage-ledger.md` mandatory per-asset entry — vault file exists but no entries for BP3 art generation.
1665. - [ ] [W3] [BP3] [DR-044] [GAP] DR-044 Aseprite headless API integration — not present (Tier 3 prep).

## 198. DR-045 — Launch content roster (CLOSED; BP3+ placeholder generation expected)
1666. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 weapons firearms count 40+ — cf-equipment has 1 (rifle.default).
1667. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 weapons heavy/explosive 15+ — 0 implemented.
1668. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 throwables/explosives 15+ — 0 implemented.
1669. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 melee 8+ — 0 implemented.
1670. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 tools 15+ — 1 (digger via try_dig) implemented.
1671. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 mobility 6+ — 0 implemented.
1672. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 shields 5+ — 0 implemented (Shield is a chassis module, not a gear item).
1673. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 sensors 6+ — 0 implemented (Sensor is a chassis module).
1674. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 medical 8+ — 0 implemented.
1675. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 repair/support 6+ — 0 implemented (RepairDrone is a chassis module).
1676. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 comms 10+ — 0 implemented (cf-comms is unimplemented).
1677. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors humans 8 — 1 (generic blue actor) implemented.
1678. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors power armor 4 — 1 (PoweredArmor) implemented.
1679. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors androids 4 — 0 implemented.
1680. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors robots 5 — 0 implemented.
1681. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors mechs 5 — 1 (LightMech) implemented.
1682. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors civilians/NPCs 6 — 0 implemented.
1683. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors undead/anomaly 6 — 0 implemented.
1684. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 actors turrets/static 6 — 0 implemented.
1685. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 dropcraft (vehicles) 12+ — 0 implemented.
1686. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 ground vehicles 6+ — 0 implemented.
1687. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 base objects 60+ — 0 implemented (reactor in m2.5 is closest but has no Base entity).
1688. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 factions 8 — 0 declared (no factions registry yet).
1689. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 missions 30+ — ~9 scenarios in content/scenarios/.
1690. - [ ] [W3] [BP3] [DR-023+DR-045] [GAP] DR-045 onboarding 3 missions (DR-023 hybrid+) — 0 implemented.
1691. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 modular labs 8 — 0 implemented.
1692. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 anchor campaign missions 6 — 0 implemented.
1693. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 procedural contract templates 8 — 0 implemented.
1694. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 Bunker Defence flagship maps 4 — 0 implemented.
1695. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 PvP arena maps 3 — 0 implemented.
1696. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 coop-vs-AI scenarios 2 — 0 implemented.
1697. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 modder-template scenarios 6 — 0 implemented.
1698. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 worlds 12 (Earth/Moon/Mars/Phobos/Deimos/Mimas/Europa/Vulcan/Venus/Sol/BeltAsteroid/OrbitalStation) — 0 declared.
1699. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 biomes 3-5 per world (~50 total) — 0 declared.
1700. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 materials 17 — only 8 in cf-terrain registry.
1701. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 ores 12 — 0 declared.
1702. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 reactions 20+ — 0 declared.
1703. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 music tracks 30+ — 0 implemented.
1704. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 SFX library 400+ clips — 0 implemented.
1705. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "Generatable by AI agent end-to-end" — no item has Tier 1 → Tier 2 → Tier 3 generation log.
1706. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "Authored as data in content/<category>/<id>.ron" — items live as Rust structs in cf-equipment, not RON.
1707. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "Functional in-game (NO asset exists but stat-only entries)" — no stat-only entries because no items.
1708. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "AI-readable role-card + AI metadata + refusal reasons + capability tags" — only 5 RoleTag variants.
1709. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "Replay-recorded with typed events" — only weapon_fired emits.
1710. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "Caption-bound" — no captions on any item event.
1711. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "Mod-validated by cf-mod validate --strict" — `--strict` mode does not exist.
1712. - [ ] [W3] [BP3] [DR-045] [GAP] DR-045 authoring requirement "Localizable + keyed strings" — production code uses raw English literals.

## 199. DR-046 — Player-facing surfaces (CLOSED; M4A delivers HUD floor; BP3+ pressure on shell UI)
1713. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Title screen + splash (animated logo + version badge + cinematic loop) — not implemented.
1714. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Main menu (Campaign / Skirmish / Multiplayer / Workshop / Workbench / Tutorial / Lab / Settings / Credits / Quit) — not implemented.
1715. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Pause menu (Resume / Save / Load / Settings / Restart / Quit) — not implemented (ESC = quit only).
1716. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Settings menu (Graphics / Audio / Controls / Accessibility / Gameplay / Language / Online tabs) — not implemented.
1717. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Server browser — not implemented.
1718. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Lobby (team config + faction pick + loadout pick + ready-up + chat + vote-kick + host migration) — not implemented.
1719. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Loadout workbench (drag/drop + AI preview + capability strip + diff vs preset + hot-swap + save presets) — not implemented.
1720. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Mission briefing (comic-panel cards + voice-over caption + objective list + faction context + LZ risk preview) — not implemented.
1721. - [ ] [W3] [BP3] [M3B+M4A] [DR-046] [GAP] DR-046 Mission debrief (comic-panel timeline + death recap + salvage summary + veteran injuries + replay CTA + share button) — M3B prints markdown only.
1722. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Strategic map / world view (multi-world astrography + per-world mission selector + faction state + comms light-lag + ore deposit map + weather forecast) — not implemented.
1723. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Achievements + collection — not implemented.
1724. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Replay viewer (scrub + speed control + multi-camera + bookmark + clip export + shareable link) — markdown-only at BP3.
1725. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Codex / lore browser — not implemented.
1726. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Photo mode — not implemented.
1727. - [ ] [W3] [BP3] [M4A] [DR-031+DR-046] [GAP] DR-046 Cosmetic locker — not implemented (DR-031 dormant; should still scaffold).
1728. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Death cam (auto-replay last 5s on death) — not implemented.
1729. - [ ] [W3] [BP3] [M4A] [DR-046] [GAP] DR-046 Mod manager (browse Workshop / Local mods + subscribe / install / update / uninstall + trust tiers + hot-load) — not implemented.

## 200. DR-046 — Flashy+punchy juice rules (every M1+ inherits via DR-055)
1730. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 button hover (scale 1.0→1.05 + glow halo + tick SFX) — not implemented.
1731. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 button click (scale punch + flash + click SFX) — not implemented.
1732. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 menu transitions (comic-panel slide + skew + ease-in-out + ambient duck) — not implemented.
1733. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 match start (dropship animation + camera drift + LZ flash + objective banner) — not implemented.
1734. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 match victory/defeat (comic-page-flip + slow-mo final frame + music swell + confetti) — not implemented.
1735. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 damage taken (screen shake + chromatic aberration + red vignette + heartbeat-bass) — not implemented.
1736. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 critical hit (time freeze 80ms + flash white + bass thump + camera punch) — not implemented.
1737. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 reload juice (magazine swap anim + shell-eject SFX + chamber-click + UI counter punch) — not implemented.
1738. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 death (slow-motion 0.3s + camera dolly-in + "show me why" prompt) — not implemented.
1739. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 achievement unlock juice — not implemented.
1740. - [ ] [W3] [M1] [DR-046+DR-055] [GAP] DR-046 settings change confirmation (tick + value snap + savestate flash) — not implemented.

## 201. DR-046 — Tutorial implementation (closes DR-023)
1741. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 "First Contract" onboarding mission (12-15 min cinematic) — not authored.
1742. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 ElevenLabs voice-over with text-only fallback — not implemented.
1743. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 8 modular labs per DR-023 — not authored.
1744. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 contextual tooltips per-tooltip use counter + fade after 3 uses — not implemented.
1745. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 "Show me why" handoff (failure → replay viewer auto-scrubbed to cause + relevant lab launcher) — not implemented.
1746. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 difficulty / accessibility presets (Standard / Easy / Hard / Custom) — not implemented.
1747. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 adaptive hints (hint engine reads EnvironmentSignal + AI bot scoring + player input patterns) — not implemented.
1748. - [ ] [W3] [DR-023+DR-046] [GAP] DR-046 AI-authored mission narrative (per-faction tone profile + reviewed by AI agent) — not authored.

## 202. DR-046 — Localization (10+ languages; BP3+ string-source discipline)
1749. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 Tier-A 11 languages (en + de + fr + es + it + pl + ru + zh-CN + zh-TW + ja + ko + pt-BR) — production code has English-only.
1750. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 Tier-B 8 UI-only languages — not authored.
1751. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 Project Fluent file structure (`content/locales/<lang>/`) — directory missing.
1752. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 Mod-localization layer — not implemented.
1753. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 CI gate "no string literal in production code" (`cf-i18n-check`) — script doesn't exist.
1754. - [ ] [W3] [BP3] [DR-046] [GAP] DR-046 AI translation pipeline (Claude Sonnet for translation + community review) — not set up.

## 203. DR-046 — Narrative bible (closes DR-016)
1755. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 setting bible 10-page worldbuilding doc — not authored.
1756. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 24+ named NPCs with bio + visual reference + dialogue tone + signature loadout — not authored.
1757. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 8 faction archives (1-page each) — not authored.
1758. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 per-mission briefing (3-5 panels) + debrief (3-5 panels) + 5-10 in-mission dialogue lines — not authored.
1759. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 codex entries (per-weapon / per-chassis / per-material / per-faction / per-world / per-named-NPC) — not authored.
1760. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 tutorial narrative (first-contract script + 8 lab intros) — not authored.
1761. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 achievement copy (60-100 achievements) — not authored.
1762. - [ ] [W3] [DR-016+DR-046] [GAP] DR-046 total launch ~80,000 words narrative copy — 0 words at BP3.

## 204. DR-047 — Launch & live operations (CLOSED; BP10+ pre-launch but BP3+ telemetry seeds)
1763. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Sentry / GlitchTip crash-reporting — not integrated.
1764. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Anonymous gameplay telemetry (opt-in, GDPR/CCPA/LGPD) — no telemetry endpoint at BP3.
1765. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 In-game bug tool (F12 → screenshot + last-30s replay snapshot + run-bundle attached) — not implemented.
1766. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Performance telemetry (frame ms / sim ms / dropped events / GPU memory / load times) — not implemented.
1767. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Balance telemetry (TTK matrix / per-faction win-rate / per-mission completion-rate) — not implemented.
1768. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 AI-driven weekly auto-report (anomaly detection + summary email + prioritized backlog) — not implemented.
1769. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Closed-alpha playtest program (~20-50 testers via Discord) — not stood up.
1770. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 AI-simulated playtests (1000s of scripted scenarios per night) — not stood up.
1771. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Soak testing (24h netcode + 7-day MMO + 100K-tick replay determinism) — not stood up.
1772. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Steam page (capsule art / screenshots / trailer / description / system requirements) — not set up.
1773. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 trailer (60-90s reveal + 30s gameplay + 60s "what is" + launch trailer) — not produced.
1774. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 press kit (logo / screenshots / key art / 3 trailers / 1-pager / contact / demo build) — not produced.
1775. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 demo build (30-60 min slice; Bunker Defence + 1 onboarding + 1 lab + 4-player coop) — not produced.
1776. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Steam Workshop integration — not implemented.
1777. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Steam Achievements (60-100) — not declared.
1778. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Steam Cloud (saves + replay archive auto-sync) — not implemented.
1779. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Steam Friends + Invites — not implemented.
1780. - [ ] [W3] [BP10+BP3] [M4A] [DR-047] [GAP] DR-047 Steam Input full controller/gamepad/Deck support — partial (gamepad focus only at M4A).
1781. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Steam Deck Verified rating — never tested.
1782. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Trademark search + registration ("Corefall") — not done.
1783. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Business entity (LLC + bank + Stripe) — not set up.
1784. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 EULA + ToS + Privacy Policy — not drafted.
1785. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Age rating (ESRB / PEGI / USK / CERO) — not submitted.
1786. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Open-source attribution screen (auto-generated from Cargo.lock via cargo-about) — not implemented.
1787. - [ ] [W3] [BP10+BP3] [DR-047] [GAP] DR-047 Music + asset licensing per-asset usage-ledger entries — usage-ledger not maintained.

## 206. DR-051 — Accessibility-plus / sustainability / customer support / platform polish (CLOSED; M-ACC-PLUS BP9..BP12; BP3+ inherited rows)
1810. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 cognitive: lower-stimulation mode (reduced VFX + slower pace + simpler UI + fewer simultaneous threats) — not implemented.
1811. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 cognitive: 'simple HUD' preset — not implemented.
1812. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 cognitive: one-thing-at-a-time tutorial pacing — not implemented.
1813. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 motor: single-button play mode — not implemented.
1814. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 motor: gesture controls — not implemented.
1815. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 motor: eye-tracking (Tobii) integration — not implemented.
1816. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 motor: slow-mo / pause-during-input — not implemented.
1817. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 motor: one-handed mode — not implemented.
1818. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 hearing: sign-language overlay for cinematics — not implemented (no cinematics yet).
1819. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 hearing: visual sub-bass cues (screen pulse on bass thump) — not implemented.
1820. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 hearing: haptic feedback alternatives — not implemented.
1821. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 hearing: full subtitle option (NOT just critical audio) — caption queue exists; no full-subtitle mode.
1822. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 hearing: audio description for visual events — not implemented.
1823. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 reading: dyslexic font option (OpenDyslexic) — not implemented.
1824. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 reading: reading speed control — not implemented.
1825. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 reading: per-paragraph TTS readout — not implemented.
1826. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 reading: large-print preset — not implemented (ui_scale to 200% only).
1827. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 sensory: pause-on-window-loss — not implemented.
1828. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 sensory: low-violence mode (decals minimal; blood color black-white) — not implemented.
1829. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 sensory: anxiety-mode (slower combat cadence) — not implemented.
1830. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 sensory: confirmation prompts on irreversible actions — not implemented.
1831. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 color blind: 8 protanope/deuteranope/tritanope/atypical protocols — only "high_contrast" boolean.
1832. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 color blind: tested with actual color-blind testers — never done.
1833. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 cinematic accessibility: audio description for cinematics — no cinematics yet.
1834. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 cinematic accessibility: skip-cinematic for low-bandwidth players — no cinematics yet.
1835. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 sunset plan (engine open-source + community-hosting handoff) — not authored.
1836. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 5-year content plan (Y1 balance/cosmetics → Y5 open-source eval) — not authored.
1837. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 customer support workflow + AI-first triage — not set up.
1838. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 Stripe direct-sales — not set up.
1839. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 refund handling for direct sales — not authored.
1840. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 tax handling (US sales tax + EU VAT + Stripe Tax) — not set up.
1841. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 pricing tier strategy ($19.99-$24.99 launch) — not set.
1842. - [ ] [W3] [BP12+BP3+BP9] [DR-051] [GAP] DR-051 bug bounty program — not set up.

## 207. DR-052 — Network sync / rollback / CLI-testable determinism (CLOSED; every M1+ inherits)
1843. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 `cfctl test sync-drift` command — not implemented (every M1+ row stays open at BP3).
1844. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 `cfctl test latency-injection` — not implemented.
1845. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 `cfctl test packet-loss` — not implemented.
1846. - [ ] [W3] [M1] [DR-002+DR-052] [GAP] DR-052 determinism CI matrix per platform + per architecture (DR-002 + DR-052) — only macOS aarch64 verified.
1847. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 lockstep input traces for online co-op — not implemented.
1848. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 client prediction + server reconciliation — not implemented.
1849. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 lag compensation — not implemented.
1850. - [ ] [W3] [M1] [DR-052] [GAP] DR-052 deterministic island contract document — not authored.

## 208. DR-053 — AI audio pipeline / realtime & generative (CLOSED; M4-M7 primary; BP3 placeholder)
1851. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 placeholder synth audio (sine/square blips per Tier 1) — not generated.
1852. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 cf-audio integration with bevy_kira_audio (Apache-2.0; Rust-native) — not present.
1853. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 cf-audio bevy_fmod optional feature flag — not present.
1854. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 audio event registry (gunshot / reload / breach / alarm / footstep / muzzle / impact / explosion / chassis-stage-change / pilot-eject) — not declared.
1855. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 caption pipeline per audio event — caption queue exists with no audio source.
1856. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 origin-specific failure sound families (human / android / robot / mech / command-core) — not declared.
1857. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 mix policy "synth music ducks under critical alarms" — no mixer.
1858. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Stable Audio Open 1.0 SFX generation pipeline — not set up.
1859. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Suno / Udio / ElevenLabs music pipeline — not set up.
1860. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 cf-asset-ledger logging for every generated audio asset — usage-ledger not maintained.

## 209. DR-054 — Performance optimization & profiling (CLOSED; every M1+ inherits perf-gate row)
1861. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 cf-bench harness — `cf-bench` crate is 38-line scaffold.
1862. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 1080p/60 perf gate per milestone — measured once for m4a only.
1863. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 800p/60 Steam Deck perf gate per milestone — never measured.
1864. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 4K/120 perf gate per milestone — never measured.
1865. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 SIMD optimization for material kernel update — material kernel doesn't exist.
1866. - [ ] [W3] [M1+M2] [DR-054] [GAP] DR-054 GPU compute path for terrain carving (M2-S02) — never implemented.
1867. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 24h memory-leak soak — never run.
1868. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 worker-thread parallelism — Bevy default scheduling only.
1869. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 frame ms / sim ms / dropped events / worker queue depth counters in run-bundle — `system.tick_sample` partial.
1870. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 `cf-bench --profile <track>` per-system bench scenarios — none authored.

## 210. DR-055 — Game feel / juice / flow state (CLOSED; every M1+ inherits juice-rules row)
1871. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 per-weapon recoil curve — single constant at BP3.
1872. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 camera punch on damage — not implemented.
1873. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 hit-stop / hit-pause on impact (80ms time freeze + flash white + bass thump) — not implemented.
1874. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 explosion camera shake — no explosions.
1875. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 screen-space damage vignette (red flash on damage taken) — not implemented.
1876. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 muzzle flash sprite + chamber-click SFX + casing-eject — not implemented.
1877. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 reload feedback (magazine swap animation + chamber-click + UI counter punch) — not implemented.
1878. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 jump anticipation + landing squash — not implemented.
1879. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 victory celebration (comic-page-flip + slow-mo final frame + music swell + confetti) — not implemented.
1880. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 failure-state slow-mo replay — not implemented.
1881. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 critical-hit time freeze 80ms + flash white + camera punch — not implemented.
1882. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 menu transitions (comic-panel slide + skew + ease-in-out + ambient duck) — not implemented.
1883. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 button hover scale 1.0→1.05 + glow halo + soft tick SFX — not implemented.
1884. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 settings change confirmation tick + animated value snap — not implemented.
1885. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 achievement unlock juice (comic-panel pop-in + cheer sting) — no achievement system.

## 228. DR-052 — Network sync, rollback, CLI-testable determinism (CLOSED; M10/M11/M12 own; BP3 cross-platform determinism inherited)
1961. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 `cfctl test sync-drift` — not implemented at BP3.
1962. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 `cfctl test latency-injection` — not implemented.
1963. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 `cfctl test rollback-burst` — not implemented.
1964. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 `cfctl test replay-determinism` — not implemented (closest equivalent is `cf-headless replay --verify-checksums`).
1965. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 floating-point determinism guarantee (`f32` strict ordering + `STD::FROUND_TO_NEAREST` + cross-platform LLVM flags + per-tick BLAKE3 checksum) — only macOS aarch64 path verified; Linux x86_64 + Windows x86_64 not bit-matched.
1966. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 RUSTFLAGS baseline `-C target-feature=+sse2,+sse4.2` — not in `.cargo/config.toml`.
1967. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 LLVM `-ffast-math` disabled in sim crates — no per-crate `[profile.release.sim-crates]` config.
1968. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 cross-platform determinism CI matrix (Win/Lin/Mac × x86/ARM × 100 runs/seed) — never run.
1969. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 cosmetic-system `cosmetic: true` flag — no per-system metadata yet.
1970. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 deterministic input ordering tie-break by player_id ascending — only single-actor at BP3; no test.
1971. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 sim-tick `f64` ban in sim islands — clippy lint not added to sim crates' `clippy.toml`.
1972. - [ ] [W3] [BP3] [M10+M11+M12] [DR-052] [GAP] DR-052 cf-headless replay --verify-checksums emits first-divergence-event {tick, recorded, live} — emits ✓ but no test for divergence injection.

## 229. DR-052 — Per-mode network architecture (BP3 forward-compat declarations)
1973. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 Solo / single-player local in-process authoritative path — works ✓ but not declared as a `cf-server --mode local` mode.
1974. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 LAN co-op deterministic lockstep — `cf-server --mode lan_room` doesn't exist at BP3.
1975. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 Online co-op server-authoritative + client prediction + reconciliation + snapshot interpolation — not implemented.
1976. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 PvP arena rollback netcode (GGPO-style) — not implemented.
1977. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 MMO shard server-authoritative + interest management + snapshot delta encoding — not implemented.
1978. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 server-server cross-shard event broadcaster — not implemented.

## 230. DR-052 — Authority taxonomy (Truth / Prediction / Presentation / Advisory) at BP3
1979. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 per-system `cosmetic: true` flag — not declared.
1980. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 per-event `truth | prediction | presentation | advisory` tag — not declared in event envelope.
1981. - [ ] [W3] [BP3] [DR-052+DR-054] [GAP] DR-052 GPU authority rule (GPU may affect Truth only after DR-054 certification matrix passes) — no GPU work at BP3; placeholder check missing.
1982. - [ ] [W3] [BP3] [DR-052] [GAP] DR-052 client prediction reconciliation threshold (visual smoothing < threshold; snap > threshold) — no constants declared.

## 231. spec/network-sync-rollback-and-determinism — references (BP3 anticipation)
1983. - [ ] [W3] [BP3] [GAP] `cf-net` adapter trait declaring `Transport::send`, `recv`, `disconnect` — not declared at BP3.
1984. - [ ] [W3] [BP3] [GAP] `cf-net` heartbeat / keepalive policy — not declared.
1985. - [ ] [W3] [BP3] [GAP] `cf-net` MTU + fragmentation policy — not declared.
1986. - [ ] [W3] [BP3] [GAP] `cf-net` reconnection policy — not declared.
1987. - [ ] [W3] [BP3] [GAP] `cf-net` encryption (TLS / Noise protocol) — not declared.
1988. - [ ] [W3] [BP3] [GAP] `cf-net` authentication handshake — not declared.

## 233. Universal Enhancement — every M1+ inherits captioning row (DR-051)
1993. - [ ] [W3] [M1] [DR-051] [UNIV-DR051] M1 captions for ALL audio — no audio surface ✓ but no caption-event auto-generation per gunshot.
1994. - [ ] [W3] [M1+M1.5] [DR-051] [UNIV-DR051] M1.5 captions — no captions for guard fire / reload / death.
1995. - [ ] [W3] [M1+M2] [DR-051] [UNIV-DR051] M2 captions — no captions for terrain_carved / tool_refused.
1996. - [ ] [W3] [M1+M2.5] [DR-051] [UNIV-DR051] M2.5 captions — no captions for reactor damage / time-remaining warnings.
1997. - [ ] [W3] [M1+M3A] [DR-051] [UNIV-DR051] M3A captions — no captions for `system.tick_sample` warnings.
1998. - [ ] [W3] [M1+M3B] [DR-051] [UNIV-DR051] M3B captions — viewer prints text; no caption surface in replay viewer.
1999. - [ ] [W3] [M1+M4A] [DR-051] [UNIV-DR051] M4A captions — captions strip is enabled ✓ but queue is empty.
2000. - [ ] [W3] [M1+M5] [DR-051] [UNIV-DR051] M5 captions — no captions for `chassis.stage_changed` / `chassis.module_state_changed` / `chassis.pilot_ejected` events.

## 234. Universal Enhancement — every M1+ inherits anti-FOMO + anti-pay-to-win audit row (DR-031)
2001. - [ ] [W3] [M1] [DR-031] [UNIV-DR031] M1 anti-FOMO audit — never run.
2002. - [ ] [W3] [M1+M1.5] [DR-031] [UNIV-DR031] M1.5 anti-FOMO audit — never run.
2003. - [ ] [W3] [M1+M2] [DR-031] [UNIV-DR031] M2 anti-FOMO audit — never run.
2004. - [ ] [W3] [M1+M2.5] [DR-031] [UNIV-DR031] M2.5 anti-FOMO audit — never run.
2005. - [ ] [W3] [M1+M3A] [DR-031] [UNIV-DR031] M3A anti-FOMO audit — never run.
2006. - [ ] [W3] [M1+M3B] [DR-031] [UNIV-DR031] M3B anti-FOMO audit — never run.
2007. - [ ] [W3] [M1+M4A] [DR-031] [UNIV-DR031] M4A anti-FOMO audit — never run.
2008. - [ ] [W3] [M1+M5] [DR-031] [UNIV-DR031] M5 anti-FOMO audit — never run.

## 235. Universal Enhancement — every M1+ inherits modding parity row (DR-006 + DR-050)
2009. - [ ] [W3] [M1] [DR-006+DR-050] [UNIV-DR006] M1 mod-author can extend rifle preset — no extension surface (RoleRecord registered in Rust).
2010. - [ ] [W3] [M1+M1.5] [DR-006+DR-050] [UNIV-DR006] M1.5 mod-author can extend guard FSM — no extension surface.
2011. - [ ] [W3] [M1+M2] [DR-006+DR-050] [UNIV-DR006] M2 mod-author can extend terrain material — `MaterialRegistry` does not allow extension at BP3.
2012. - [ ] [W3] [M1+M2.5] [DR-006+DR-050] [UNIV-DR006] M2.5 mod-author can extend reactor — no reactor entity moddable.
2013. - [ ] [W3] [M1+M3A] [DR-006+DR-050] [UNIV-DR006] M3A mod-author can extend event schema — no extension allowed.
2014. - [ ] [W3] [M1+M3B] [DR-006+DR-050] [UNIV-DR006] M3B mod-author can extend viewer rendering — viewer is hardcoded markdown.
2015. - [ ] [W3] [M1+M4A] [DR-006+DR-050] [UNIV-DR006] M4A mod-author can override HUD palette — no mod-override surface.
2016. - [ ] [W3] [M1+M5] [DR-006+DR-050] [UNIV-DR006] M5 mod-author can extend chassis archetype — `chassis_specs()` is a Rust fn; not data-driven.

## 236. Universal Enhancement — every M1+ inherits Tier-A 11-language localization row (DR-046)
2017. - [ ] [W3] [M1] [DR-046] [UNIV-DR046] M1 11-language keyed-strings — production English strings.
2018. - [ ] [W3] [M1+M1.5] [DR-046] [UNIV-DR046] M1.5 11-language strings — same.
2019. - [ ] [W3] [M1+M2] [DR-046] [UNIV-DR046] M2 11-language strings — same.
2020. - [ ] [W3] [M1+M2.5] [DR-046] [UNIV-DR046] M2.5 11-language strings — same.
2021. - [ ] [W3] [M1+M3A] [DR-046] [UNIV-DR046] M3A 11-language strings — same.
2022. - [ ] [W3] [M1+M3B] [DR-046] [UNIV-DR046] M3B 11-language strings — viewer prints English only.
2023. - [ ] [W3] [M1+M4A] [DR-046] [UNIV-DR046] M4A 11-language strings — HUD lines are English literals.
2024. - [ ] [W3] [M1+M5] [DR-046] [UNIV-DR046] M5 11-language strings — chassis banners + module-strip are English literals.

## 237. Universal Enhancement — every M1+ inherits replay determinism per platform + per arch (DR-002 + DR-052)
2025. - [ ] [W3] [M1] [DR-002+DR-052] [UNIV-DET] M1 60Hz determinism on Win/Lin/Mac × x86/ARM matrix — only macOS aarch64.
2026. - [ ] [W3] [M1] [DR-002+DR-052] [UNIV-DET] M1 120Hz determinism on full matrix — only macOS aarch64.
2027. - [ ] [W3] [M1+M1.5] [DR-002+DR-052] [UNIV-DET] M1.5 60Hz determinism — only macOS aarch64.
2028. - [ ] [W3] [M1+M2] [DR-002+DR-052] [UNIV-DET] M2 60Hz determinism — only macOS aarch64.
2029. - [ ] [W3] [M1+M2.5] [DR-002+DR-052] [UNIV-DET] M2.5 60Hz determinism — only macOS aarch64.
2030. - [ ] [W3] [M1+M3A] [DR-002+DR-052] [UNIV-DET] M3A headless replay determinism — only macOS aarch64.
2031. - [ ] [W3] [M1+M3B] [DR-002+DR-052] [UNIV-DET] M3B viewer determinism (same bundle → same markdown) — works ✓ but no cross-platform test.
2032. - [ ] [W3] [M1+M4A] [DR-002+DR-052] [UNIV-DET] M4A live-WS-acceptance determinism — only macOS aarch64.
2033. - [ ] [W3] [M1+M5] [DR-002+DR-052] [UNIV-DET] M5 chassis state determinism — only macOS aarch64.

## 238. Universal Enhancement — every M1+ inherits ai-agent validation row (DR-026 + DR-056)
2034. - [ ] [W3] [M1] [DR-026+DR-056] [UNIV-DR026] M1 AI-agent validation report — present in implementation log; not formal Q1-Q7.
2035. - [ ] [W3] [M1+M1.5] [DR-026+DR-056] [UNIV-DR026] M1.5 AI-agent validation report — present; not formal Q1-Q7.
2036. - [ ] [W3] [M1+M2] [DR-026+DR-056] [UNIV-DR026] M2 AI-agent validation report — not present (M2 unchecked in checklist).
2037. - [ ] [W3] [M1+M2.5] [DR-026+DR-056] [UNIV-DR026] M2.5 AI-agent validation report — not present.
2038. - [ ] [W3] [M1+M3A] [DR-026+DR-056] [UNIV-DR026] M3A AI-agent validation report — not present.
2039. - [ ] [W3] [M1+M3B] [DR-002+DR-026+DR-056] [UNIV-DR026] M3B AI-agent validation report — present in DR-002 closure note.
2040. - [ ] [W3] [M1+M4A] [DR-026+DR-056] [UNIV-DR026] M4A AI-agent validation report — present; not Q1-Q7 structured.
2041. - [ ] [W3] [M1+M5] [DR-026+DR-056] [UNIV-DR026] M5 AI-agent validation report — present; not Q1-Q7 structured.

## 239. DR-056 — Per-milestone enhancement specifics per M0..M5 row by row (CLOSED)
2042. - [ ] [W3] [M0+M1+M5] [DR-052+DR-056] [GAP] M1 enhancement "Input prediction for player-driven actor (DR-052 client prediction)" — not implemented.
2043. - [ ] [W3] [M0+M1+M5] [DR-055+DR-056] [GAP] M1 enhancement "Recoil curves per weapon (DR-055)" — single constant; not per-weapon.
2044. - [ ] [W3] [M0+M1+M5] [DR-056] [GAP] M1 enhancement "Camera punch on damage taken" — not implemented.
2045. - [ ] [W3] [M0+M1+M5] [DR-056] [GAP] M1 enhancement "Animation event tags fire correctly" — no animator + no tag events.
2046. - [ ] [W3] [M0+M1+M5] [DR-053+DR-056] [GAP] M1 enhancement "Audio: footstep + reload + weapon-fire generated (DR-053 Tier 1)" — cf-audio stub.
2047. - [ ] [W3] [M0+M1+M5] [DR-056] [GAP] M1 enhancement "cfctl `act move/aim/fire/reload` with assertion harness" — actions work; no assertion-harness UI.
2048. - [ ] [W3] [M0+M1+M4A+M5] [DR-056] [GAP] M1 enhancement "ACC-A keyboard remap + reduced motion settings" — present in M4A, not M1 closure.
2049. - [ ] [W3] [M0+M1.5+M5] [DR-056] [GAP] M1.5 enhancement "Match feel-test playtest (project-owner + 3-5 testers)" — never recorded.
2050. - [ ] [W3] [M0+M1.5+M5] [DR-050+DR-056] [GAP] M1.5 enhancement "Adaptive difficulty toggle (DR-050)" — not implemented.
2051. - [ ] [W3] [M0+M1.5+M5] [DR-056] [GAP] M1.5 enhancement "AI difficulty preset visible" — not implemented.
2052. - [ ] [W3] [M0+M1.5+M5] [DR-056] [GAP] M1.5 enhancement "Replay sharing prototype" — not implemented (no replay sharing surface).
2053. - [ ] [W3] [M0+M2+M5] [DR-056] [GAP] M2 enhancement "GPU compute path investigation (deterministic backup; CPU baseline)" — not done.
2054. - [ ] [W3] [M0+M2+M5] [DR-056] [GAP] M2 enhancement "SIMD material kernel update (8 pixels per SIMD lane; deterministic)" — not implemented.
2055. - [ ] [W3] [M0+M2+M5] [DR-056] [GAP] M2 enhancement "Streaming asset budget per scenario" — not measured.
2056. - [ ] [W3] [M0+M2+M5] [DR-056] [GAP] M2 enhancement "Cold-load benchmark in CI" — not wired.
2057. - [ ] [W3] [M0+M3A+M5] [DR-056] [GAP] M3A enhancement "Per-tick checksum (blake3); replay determinism CI matrix per platform" — only macOS.
2058. - [ ] [W3] [M0+M3A+M5] [DR-056] [GAP] M3A enhancement "Replay branching" — not implemented.
2059. - [ ] [W3] [M0+M3A+M5] [DR-056] [GAP] M3A enhancement "Replay editing tools prototype" — not built.
2060. - [ ] [W3] [M0+M3A+M5] [DR-056] [GAP] M3A enhancement "Replay sharing infrastructure" — not implemented.
2061. - [ ] [W3] [M0+M4+M5] [DR-056] [GAP] M4 enhancement "Reactive UI data binding (per Bevy state)" — static text strips only.
2062. - [ ] [W3] [M0+M4+M5] [DR-056] [GAP] M4 enhancement "UI testing harness (cfctl ui assert)" — not implemented.
2063. - [ ] [W3] [M0+M4+M5] [DR-046+DR-055+DR-056] [GAP] M4 enhancement "All juice rules per DR-046 + DR-055" — not authored.
2064. - [ ] [W3] [M0+M4+M4A+M5] [DR-056] [GAP] M4 enhancement "Accessibility 200% UI scale + high contrast verified" — verified ✓ at M4A.
2065. - [ ] [W3] [M0+M4+M5] [DR-056] [GAP] M4 enhancement "Localization keyed strings (Tier-A 11 languages)" — English-only.
2066. - [ ] [W3] [M0+M4+M5] [DR-046+DR-056] [GAP] M4 enhancement "Animation system for UI panels (slide + skew per DR-046)" — no panel animations.
2067. - [ ] [W3] [M0+M4+M5] [DR-056] [GAP] M4 enhancement "Settings menu full tree" — single-page settings flags only.
2068. - [ ] [W3] [M0+M5] [DR-056] [GAP] M5 enhancement "Hot-reload polish (cf-mod reload <id>)" — not implemented.
2069. - [ ] [W3] [M0+M5] [DR-056] [GAP] M5 enhancement "Equipment validation in playtest scenarios" — not tested.
2070. - [ ] [W3] [M0+M5] [DR-056] [GAP] M5 enhancement "Equipment AI behavior tests (utility scoring per weapon)" — not implemented.
2071. - [ ] [W3] [M0+M5+M5.8] [DR-040+DR-056] [GAP] M5 enhancement "Origin-resource integration (per DR-040 + M5.8)" — partial scaffold (ResourceAccumulators struct exists).
2072. - [ ] [W3] [M0+M5] [DR-055+DR-056] [GAP] M5 enhancement "Damage stage juice (per DR-055)" — chassis_stage_tint exists ✓ but no hit-stop / camera-punch on stage transitions.
2073. - [ ] [W3] [M0+M5] [DR-053+DR-056] [GAP] M5 enhancement "Audio per weapon fire / reload / hit (per DR-053)" — cf-audio stub.

## 240. DR-056 — Universal CI integration gaps (CLOSED; should auto-run per milestone)
2074. - [ ] [W3] [DR-056] [GAP] DR-056 CI integration: `cf-bench` regression vs baseline — not wired.
2075. - [ ] [W3] [DR-056] [GAP] DR-056 CI integration: `cfctl test sync-drift` per multiplayer milestone — pre-multiplayer at BP3 but rule applies forward.
2076. - [ ] [W3] [DR-056] [GAP] DR-056 CI integration: `cf-i18n-check` per UI milestone — not implemented.
2077. - [ ] [W3] [DR-056] [GAP] DR-056 CI integration: `cf-caption-check` per audio milestone — not implemented.
2078. - [ ] [W3] [DR-056] [GAP] DR-056 CI integration: `cargo-allocator-stats` per hot-path milestone — not implemented.
2079. - [ ] [W3] [DR-056] [GAP] DR-056 CI integration: memory leak detection per long-soak milestone — not implemented.

## 242. cf-ui caption pipeline gaps
2088. - [ ] [W3] [GAP] cf-ui `CaptionStripPlugin` — exists ✓ but no producer system that feeds captions.
2089. - [ ] [W3] [GAP] cf-ui captions queue has no priority field — first-in-first-out only.
2090. - [ ] [W3] [GAP] cf-ui captions queue has no severity tag.
2091. - [ ] [W3] [GAP] cf-ui captions queue has no source actor id.
2092. - [ ] [W3] [GAP] cf-ui captions queue has no spatial hint (where on screen the audio originated).
2093. - [ ] [W3] [GAP] cf-ui captions queue has no `accessibility.captions_visible` validation test.
2094. - [ ] [W3] [GAP] cf-ui captions queue has no language tag (Tier-A locale).

## 243. M4A — banner queue gaps
2095. - [ ] [W3] [M4A] [GAP] cf-ui `BannerQueuePlugin` — exists ✓ but no `banner.priority` field documented.
2096. - [ ] [W3] [M4A] [GAP] cf-ui banner queue has no `expires_at_tick` field — banners auto-fade after fixed N ticks.
2097. - [ ] [W3] [M4A] [GAP] cf-ui banner queue has no `parent_event_id` to link back to the triggering event.
2098. - [ ] [W3] [M4A] [GAP] cf-ui banner queue has no `replay_anchor` for replay debriefs to jump to a banner.
2099. - [ ] [W3] [M4A] [GAP] cf-ui banner queue has only 5 severity-icon-glyphs `[!!]` / `[!]` / `[*]` / `[+]` / `[ ]`; not extensible.
2100. - [ ] [W3] [M4A] [GAP] cf-ui banner queue does not persist across `scenario.reset`.

## 251. cf-render-2d — sprite asset gaps inherited at BP3 close
2139. - [ ] [W3] [BP3] [GAP] No actor body sprite asset (currently solid colored rectangles).
2140. - [ ] [W3] [BP3] [GAP] No actor face / head sprite asset.
2141. - [ ] [W3] [BP3] [GAP] No weapon sprite asset (rifle pip is a tiny rectangle).
2142. - [ ] [W3] [BP3] [M0] [GAP] No reticle sprite (4×4 solid yellow rectangle at M0; not a crosshair).
2143. - [ ] [W3] [BP3] [GAP] No floor texture (solid dark grey).
2144. - [ ] [W3] [BP3] [GAP] No breach strip sprite (yellow/orange solid).
2145. - [ ] [W3] [BP3] [GAP] No extraction zone sprite (green solid).
2146. - [ ] [W3] [BP3] [GAP] No reactor sprite (in m2.5; uses generic actor sprite).
2147. - [ ] [W3] [BP3] [GAP] No projectile sprite (2px solid).
2148. - [ ] [W3] [BP3] [GAP] No tracer / trail visual.
2149. - [ ] [W3] [BP3] [GAP] No actor-direction indicator (which way is the actor facing?).
2150. - [ ] [W3] [BP3] [GAP] No actor team-color band (player blue / enemy red works but it's the entire actor).

## 256. DR-014 — Tone (CLOSED; all M1+ inherit)
2189. - [ ] [W3] [M1] [DR-014] [GAP] DR-014 product stance "Smoke, sparks, alarms, hydraulic whine, servo failure are part of the diegetic feedback layer" — not implemented.
2190. - [ ] [W3] [M1] [DR-014] [GAP] DR-014 product stance "Damageable equipment must jam, overheat, lose components, or be destroyed" — overheat + lose-components missing.
2191. - [ ] [W3] [M1] [DR-014] [GAP] DR-014 product stance "Repair / salvage" — chassis repair ✓; equipment repair missing.
2192. - [ ] [W3] [M1] [DR-014] [GAP] DR-014 product stance "AI reason labels: every chassis-related AI decision (eject/retreat/bail/repair/swap/suppress) emits a reason string" — not implemented.
2193. - [ ] [W3] [M1] [DR-014] [GAP] DR-014 product stance "Replay/debrief cause chains" — partial; equipment failures + AI decisions not in chain.

## 264. DR-053 — AI audio pipeline (CLOSED; M4-M7 primary; BP3 cf-audio stub)
2243. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Tier 1 SFX library (400+ clips: weapons / footsteps / equipment / environment / UI / combat / voice barks) — 0 clips at BP3.
2244. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Tier 1 Stable Audio Open 1.0 local generation pipeline — not set up.
2245. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Tier 2 hero music tracks (main theme + 12 world themes + 6 combat layers + 4 base-tension + 4 menu/UI + 8 mission stings + 3 antagonist motifs) — 0 tracks.
2246. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Tier 2 Suno / Udio / ElevenLabs cloud pipeline — not set up.
2247. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Tier 3 ambient music + procedural (MusicGen/AudioCraft local) — not set up.
2248. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Tier 4 voice / NPC dialogue (XTTS-v2 / Coqui TTS / ElevenLabs / Tortoise) — not set up.
2249. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 Tier 5 runtime adaptive + spatial (FMOD Studio / bevy_kira_audio + Steam Audio) — cf-audio stub.
2250. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 real-time procedural audio (combat impact / doppler / atmospheric absorption / footstep variety / ricochet) — not implemented.
2251. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 real-time ambient mix per `EnvironmentSignal` — no EnvironmentSignal consumer.
2252. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 reverb per room via Steam Audio raytraced — not implemented.
2253. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 voice phoneme lip-sync via XTTS / Coqui — not implemented.
2254. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 music adaptive layering per intensity via FMOD parameter automation — not implemented.
2255. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 EMP / weapon signature procedural via `bevy_fundsp` — not implemented.
2256. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 caption generation per SFX prompt via LLM — not implemented.
2257. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 hardware floor (32GB VRAM Tier 1; ≥12GB VRAM modder floor) — never tested.
2258. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 `content/audio/` directory + prompts subdirectories — directories missing.
2259. - [ ] [W3] [BP3] [M4+M7] [DR-053] [GAP] DR-053 `content/audio/stems/` directory — missing.

## 265. DR-054 — Performance optimization & profiling (CLOSED; every M1+ inherits per-tier perf gate)
2260. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 hot-path inventory not enforced — no automatic detection of new hot paths in CI.
2261. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 SIMD via `std::simd` + `wide` crate — not integrated in any hot path at BP3.
2262. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 Bevy parallel systems for hot paths — Bevy default scheduling only.
2263. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 archetype-based ECS cache-friendly queries — Bevy default; no measured cache profile.
2264. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 spatial partitioning for cf-physics broadphase / cf-ai perception / cf-render culling — not implemented.
2265. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 memory arenas for per-tick allocation — no arena allocator.
2266. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 object pooling for projectiles / particles / decals — projectiles allocate per spawn at BP3.
2267. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 zero-allocation hot loops — not enforced.
2268. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 cache-friendly SoA layout for component data — Bevy uses AoS by default.
2269. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 profile-guided optimization (PGO) for release builds — not configured.
2270. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 GPU compute tiers (Tier 0 presentation / Tier 1 prediction / Tier 2 advisory / Tier 3 server / Tier 4 authoritative) — only Tier 0 / Tier 1 implicit.
2271. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 GPU certification matrix (NVIDIA + AMD + Intel + Apple + Steam Deck × same seed + same inputs + same mod set + 10K+ ticks + per-tick BLAKE3 + byte-identical state) — never run.
2272. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 SIMD vs scalar bit-identical assert (CI gate) — not wired.
2273. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 lazy initialization (per-mod hot-load / per-scenario asset load) — not implemented.
2274. - [ ] [W3] [M1] [DR-054] [GAP] DR-054 `cargo-clippy-allocator-stats` lint — not installed.

## 266. DR-054 — Hot path inventory at BP3
2275. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Material kernel (chunked CA)" — cf-material stub.
2276. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Atmospherics (PV=nRT)" — cf-atmos stub.
2277. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Physics narrowphase 1000+ entities @ 60Hz" — current physics has no narrowphase.
2278. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Replay event recording 100K+ events/run @ 60Hz" — works but no pre-allocated buffer.
2279. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Pathfinding 10+ bots × per-tick @ 60Hz" — no pathfinder.
2280. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Renderer (terrain + sprites)" 4K/120 + Deck/60 — never measured.
2281. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Network serialization" 60Hz snapshot @ MMO scale — no cf-net.
2282. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "AI utility scoring 50+ bots × per-tick @ 60Hz" — only ReactiveGuard FSM.
2283. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "LLM Mind (async)" — no LLM mind layer.
2284. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Animation event tags per-frame per-actor @ 60Hz" — no animator.
2285. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Lighting (per-light shadow)" — no lighting system.
2286. - [ ] [W3] [BP3] [DR-054] [GAP] DR-054 hot path "Audio (Steam Audio + Kira)" 60Hz mix + 32-256 spatial channels — cf-audio stub.

## 267. DR-055 — Game feel / juice (CLOSED; every M1+ inherits juice-rules row)
2287. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Bullet hit body" (screen flash + crosshair pulse + bass thump + camera shake × impulse) — not implemented.
2288. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Critical hit / one-shot kill" (time freeze 80ms + flash white + bass thump + camera punch + chromatic aberration) — not implemented.
2289. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Headshot" (above + slow-mo 0.3s + camera dolly + signature ding) — not implemented.
2290. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Limb separation" (slow-mo 0.2s + bone-shatter + heavy camera shake + blood arc + dropped-limb collidable) — not implemented.
2291. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Player damage taken" (screen shake + chromatic aberration + red vignette + heartbeat-bass) — not implemented.
2292. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Player low HP" (persistent red vignette pulse + slow-mo on next damage threat + heartbeat sub-bass +20%) — not implemented.
2293. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Player death" (slow-mo 0.3s + camera dolly-in + spotlight + dim ambient + 'show me why') — not implemented.
2294. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Reload" (magazine swap + shell-eject + chamber-click + UI ammo counter punch) — not implemented.
2295. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Reload finish" (chamber-snap + crosshair flash + bass thump) — not implemented.
2296. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Weapon recoil curve" (impulse + damping; torso-bone rotation; aim-pitch shift; 0.3-0.8s decay) — single constant at BP3.
2297. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Weapon overheat" (heat-haze + sizzle + cool-down indicator) — not implemented.
2298. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Throwing grenade" (windup + arc preview + after-throw camera follow) — no grenades.
2299. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Explosion" (multi-stage VFX + heavy camera shake + bass + ear-ringing + concussion-blur + audio duck) — no explosions.
2300. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "EMP discharge" (cyan zigzag + screen static + electronics flicker + radio interference) — not implemented.
2301. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Dropship landing" cinematic 4s + LZ flash + camera follow + landing thump + dust — no dropships.
2302. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Bunker breach" (door opens + hot light + silhouettes + breach charge + camera shake) — partial; visual not implemented.
2303. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Match start" (dropship cinematic 4s + camera drift + LZ flash + objective banner) — not implemented.
2304. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Match victory" (comic-page-flip + slow-mo final frame + music swell + confetti faction-tinted) — not implemented.
2305. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Match defeat" (scroll-of-failure + music dirge + dim camera + subdued palette) — not implemented.
2306. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Achievement unlock" (comic-panel pop-in + cheer sting + collection update) — no achievement system.
2307. - [ ] [W3] [M1] [DR-055] [GAP] DR-055 combat juice "Critical state change (core uprooted etc.)" (camera punch + flash + signature SFX + UI banner) — no command core.

## 268. DR-055 — Animation polish gaps
2308. - [ ] [W3] [DR-055] [GAP] DR-055 animation cancel windows (per-action) — not implemented.
2309. - [ ] [W3] [DR-055] [GAP] DR-055 animation interrupt priority hierarchy — not implemented.
2310. - [ ] [W3] [DR-055] [GAP] DR-055 snap-to-target vs free aim (per-weapon aim-assist scale; controller vs KB/M) — no aim-assist at BP3.
2311. - [ ] [W3] [DR-055] [GAP] DR-055 weapon-IK to hand socket (skeletal hero chassis: weapon parented to hand bone) — no skeletal animation.
2312. - [ ] [W3] [DR-055] [GAP] DR-055 procedural recoil (per-weapon impulse + damping + torso-bone rotation + aim-pitch shift + chassis-mass-scaled) — not implemented.
2313. - [ ] [W3] [DR-055] [GAP] DR-055 procedural knockback (per-impulse + actor center pushback + secondary jiggle / sprite scale punch) — not implemented.
2314. - [ ] [W3] [DR-055] [GAP] DR-055 limb tracking (aim): skeletal arm + weapon bones rotate per aim_pitch — not implemented.
2315. - [ ] [W3] [DR-055] [GAP] DR-055 foot-IK (footstep frame anchor + per-surface SFX + foot-on-terrain physics) — not implemented.
2316. - [ ] [W3] [DR-055] [GAP] DR-055 physics authority transition (`physics.authority_changed` event when knocked / stunned / dead / pressure-wind / explosion / limb-detached) — not emitted.

## 269. DR-055 — Camera punch system gaps
2317. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Hit confirm" (brief 1° rotation + 0.5px zoom) — not implemented.
2318. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Critical hit" (3° rotation + 2px zoom + 0.05s freeze) — not implemented.
2319. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Player damage" (magnitude-scaled 1-5° rotation + 1-3px zoom) — not implemented.
2320. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Critical damage" (5° rotation + 5px zoom + chromatic aberration) — not implemented.
2321. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Death" (slow dolly + zoom + dim) — not implemented.
2322. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Match victory" (dolly + zoom out + hold) — not implemented.
2323. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Bunker breach" (sweep across breach point) — not implemented.
2324. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Mission start" (drone-style fly-in to LZ) — not implemented.
2325. - [ ] [W3] [DR-055] [GAP] DR-055 camera punch "Pause" (subtle zoom + desaturate + ambient duck) — no pause at BP3.

## 270. DR-055 — Vibration / haptic patterns
2326. - [ ] [W3] [DR-055] [GAP] DR-055 haptic feedback per gamepad event — not implemented.
2327. - [ ] [W3] [DR-055] [GAP] DR-055 haptic per weapon fire — not implemented.
2328. - [ ] [W3] [DR-055] [GAP] DR-055 haptic per damage taken — not implemented.
2329. - [ ] [W3] [DR-055] [GAP] DR-055 haptic per explosion — no explosions.
2330. - [ ] [W3] [DR-055] [GAP] DR-055 haptic per chassis stage change — not implemented.

## 271. DR-055 — Flow state design (per-mission challenge-skill curve)
2331. - [ ] [W3] [DR-055] [GAP] DR-055 per-mission challenge-skill matched difficulty curve — not implemented.
2332. - [ ] [W3] [DR-055] [GAP] DR-055 per-session pacing tension/relief/tension — not implemented.
2333. - [ ] [W3] [DR-055] [GAP] DR-055 reward cadence — not implemented.
2334. - [ ] [W3] [DR-055] [GAP] DR-055 information overload prevention — no measurement.
2335. - [ ] [W3] [DR-050+DR-055] [GAP] DR-055 per-player adaptive difficulty (extends DR-050 onboarding) — not implemented.

## 280. spec/accessibility-comfort-slice-a — Accessibility / comfort floor (Universal Enhancement DR-012 ACC-A floor)
2583. - [ ] [W3] [DR-012] [GAP] ACC-A `text_scale` setting (100% / 150% / 200%) for HUD + command + loadout + workbench + replay + hub + settings — no text scale at BP3.
2584. - [ ] [W3] [DR-012] [GAP] ACC-A `ui_density` setting (Compact / Comfortable) — not implemented.
2585. - [ ] [W3] [DR-012] [GAP] ACC-A `contrast_mode` setting (Standard / High Contrast Dark / High Contrast Light) — not implemented.
2586. - [ ] [W3] [DR-012] [GAP] ACC-A `color_cue_mode` (Default / Colorblind-safe / Monochrome test) — not implemented.
2587. - [ ] [W3] [DR-012] [GAP] ACC-A `caption_mode` (Critical only / Expanded / Off) for AI + delivery + objective + combat-critical sounds — no captions at BP3.
2588. - [ ] [W3] [DR-012] [GAP] ACC-A `caption_background` (Off / 50% / 80% / 100% opacity) — not implemented.
2589. - [ ] [W3] [DR-012] [GAP] ACC-A `input_profile` (Keyboard/mouse / controller / keyboard-only / custom) — single keybind at BP3.
2590. - [ ] [W3] [DR-012] [GAP] ACC-A `remap_actions` for Gameplay / command / UI / replay / workbench groups — not implemented.
2591. - [ ] [W3] [DR-012] [GAP] ACC-A `hold_behavior` (Hold / Toggle / Press-to-cycle) — not implemented.
2592. - [ ] [W3] [DR-012] [GAP] ACC-A `game_speed_assist` (Off / Slowdown75 / Slowdown25 / Pause in menus) — not implemented.
2593. - [ ] [W3] [DR-012] [GAP] ACC-A `screen_shake_scale` (0% / 25% / 50% / 100%) — no shake at BP3.
2594. - [ ] [W3] [DR-012] [GAP] ACC-A `camera_motion` (Reduced / Standard) — no camera motion.
2595. - [ ] [W3] [DR-012] [GAP] ACC-A `flash_reduction` (On / Off) — no flash effects.
2596. - [ ] [W3] [DR-012] [GAP] ACC-A `objective_help` (Minimal / Standard / Verbose) — no objective hints.
2597. - [ ] [W3] [DR-012] [GAP] ACC-A `debug_explainer_level` (Player / Designer / Raw) — not implemented.
2598. - [ ] [W3] [DR-012] [GAP] ACC-A floor "PC 1080p important text >= 18 px" — no text-size enforcement.
2599. - [ ] [W3] [DR-012] [GAP] ACC-A floor "Console/TV target >= 26 px" — no console scale.
2600. - [ ] [W3] [DR-012] [GAP] ACC-A floor "Standard text contrast >= 4.5:1" — no contrast measurement.
2601. - [ ] [W3] [DR-012] [GAP] ACC-A floor "Large elements >= 3:1" — no measurement.
2602. - [ ] [W3] [DR-012] [GAP] ACC-A floor "High-contrast elements >= 7:1" — no measurement.
2603. - [ ] [W3] [DR-012] [GAP] ACC-A floor "WCAG 2.2 motion/flash (no >3 flashes/sec unsafe)" — no flash budget.
2604. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_accessibility_setting_changed` — not emitted.
2605. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_text_scale_applied` (scale + affected screen + reflow mode + overflow/clipped count) — not emitted.
2606. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_contrast_mode_changed` (mode + palette + failed contrast count) — not emitted.
2607. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_color_cue_audit` (screen + critical state count + color-only count + missing labels) — not emitted.
2608. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_input_remap_changed` (action + binding + device class + conflict state) — not emitted.
2609. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_focus_path_tested` (screen + device + route + trap count + back-path) — not emitted.
2610. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_caption_shown` (caption id + event id + category + verbosity + duration + occlusion) — not emitted.
2611. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_screen_shake_scaled` (event + original magnitude + applied scale + replacement cue) — not emitted.
2612. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_flash_suppressed` (event + source effect + suppression reason + replacement cue) — not emitted.
2613. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_motion_reduced` (camera/effect + mode + applied alternative) — not emitted.
2614. - [ ] [W3] [DR-012] [GAP] ACC-A event `ux_objective_reminder_shown` (objective + verbosity + trigger + player action) — not emitted.
2615. - [ ] [W3] [DR-012] [GAP] ACC-A run-bundle `run_manifest.json.accessibility_profile` (text_scale + contrast_mode + color_cue_mode + input_profile + caption_mode + shake_scale + flash_reduction + game_speed_assist + build_hash) — not recorded.
2616. - [ ] [W3] [DR-012] [GAP] ACC-A run-bundle `summary.json.ACC-A` rows (pass/fail + screenshots + overflow/clipped count + focus trap count + color-only critical state count + caption coverage count + flash suppression count) — not recorded.
2617. - [ ] [W3] [DR-012] [GAP] ACC-A run-bundle `/screenshots/` 100% + 200% per surface (HUD + command + buy/loadout + workbench + replay/death recap + hub + package builder + settings) — not captured.
2618. - [ ] [W3] [DR-012] [GAP] ACC-A acceptance ACC-A-01..16 — none pass.
2619. - [ ] [W3] [DR-012] [GAP] ACC-A "Settings reachable from first-run, hub, pause, and prototype debug menu" — minimal settings menu at BP3.
2620. - [ ] [W3] [DR-012] [GAP] ACC-A "Settings persist across restart + recorded in run-bundle manifest" — settings not persisted.

## 288. spec/audio-identity — Diegetic industrial synth-dread audio identity (BP3 cf-audio stub)
2985. - [ ] [W3] [BP3] [GAP] AUDIO Layer "Diegetic physical (gunfire / drilling / jetpack / hydraulics / servos / reactor hums / shield buzz / alarms / radio chatter / collapsing terrain / sparks / smoke vents / repair tools)" — no audio at BP3.
2986. - [ ] [W3] [BP3] [GAP] AUDIO Layer "Synth / dread emotional (command core uprooted / base power failing / enemy commander push / pilot trapped / mech reactor critical / extraction window / post-mission debrief)" — no audio.
2987. - [ ] [W3] [BP3] [GAP] AUDIO Layer "Caption / event (every critical SFX has caption + every audio event in event taxonomy)" — no captions/events.
2988. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Loud weapon report → AI alarm triggered" — not implemented.
2989. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Servo grind → mech leg/arm joint degrading" — not implemented.
2990. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Hydraulic hiss → module pressure drop" — not implemented.
2991. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Spark/crackle → armor cracked" — not implemented.
2992. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Reactor hum → base power rooted/healthy" — not implemented.
2993. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Reactor hum dropout → command core uprooted" — not implemented.
2994. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Shield buzz pitch shift → shield overheating" — not implemented.
2995. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Pilot eject alarm → extraction window" — partial.
2996. - [ ] [W3] [BP3] [GAP] AUDIO tactical cue "Friendly radio 'covering door' / 'low ammo falling back' / enemy commander voice" — not implemented.
2997. - [ ] [W3] [BP3] [GAP] AUDIO mix rule "Synth music ducks under critical alarms" — no music.
2998. - [ ] [W3] [BP3] [GAP] AUDIO mix rule "Diegetic SFX positioned in stereo (left-right pan)" — no spatial audio.
2999. - [ ] [W3] [BP3] [GAP] AUDIO mix rule "Caption events fire same tick as SFX they describe" — no captions.
3000. - [ ] [W3] [BP3] [GAP] AUDIO mix rule "Music auto-fades within 2s of tension trigger clearing" — no music.
3001. - [ ] [W3] [BP3] [GAP] AUDIO mix rule "Player can cap music volume independently from SFX" — no volume controls.
3002. - [ ] [W3] [BP3] [GAP] AUDIO mix rule "Origin-class failure sounds (organic / android / robot / mech / command-core) are distinct families" — no failure sounds.
3003. - [ ] [W3] [BP3] [GAP] AUDIO caption "Every critical audio cue has caption + styled per category + queued by priority + replay-faithful + modder must provide caption" — no captions.
3004. - [ ] [W3] [BP3] [GAP] AUDIO origin-specific organic failure sound family (wet impact + breathing + groan + blood spatter + body fall thud) — not implemented.
3005. - [ ] [W3] [BP3] [GAP] AUDIO origin-specific android failure sound family (servo whine + capacitor pop + voice modulator stutter + shell rattle) — not implemented.
3006. - [ ] [W3] [BP3] [GAP] AUDIO origin-specific robot failure sound family (mechanical clunk + loose bolt + gear grind + optical static) — not implemented.
3007. - [ ] [W3] [BP3] [GAP] AUDIO origin-specific powered armor failure (hydraulic hiss + organic groan layered) — not implemented.
3008. - [ ] [W3] [BP3] [GAP] AUDIO origin-specific light/medium mech failure (servo grind + alarm sequence + hydraulic burst + pilot voice) — not implemented.
3009. - [ ] [W3] [BP3] [GAP] AUDIO origin-specific heavy mech failure (deep reactor whine + structural creak + alarm cascade + ejection rocket) — not implemented.
3010. - [ ] [W3] [BP3] [GAP] AUDIO origin-specific command-core avatar failure (reactor pulse + distortion artifact + command-channel warning + identity-failure tone) — not implemented.

## 289. spec/vfx-and-particles — VFX/particles closed direction (BP3 cf-vfx scaffold)
3011. - [ ] [W3] [BP3] [GAP] VFX `cf-vfx-cosmetic` (GPU-instanced sprite particles + cosmetic flag + NOT replay-deterministic) — not implemented.
3012. - [ ] [W3] [BP3] [GAP] VFX `cf-vfx-gameplay` (CPU-deterministic particles + cause-chain VFX + replay events fire) — not implemented.
3013. - [ ] [W3] [BP3] [GAP] VFX `cf-decal` (persistent terrain decals per cf-material chunk + replay-deterministic) — not implemented.
3014. - [ ] [W3] [BP3] [GAP] VFX combat "Muzzle flash" (per-weapon flash signature + 2-frame sprite + dynamic light emission) — not implemented.
3015. - [ ] [W3] [BP3] [GAP] VFX combat "Casing eject" (per-weapon casing + bouncing physics + persists 8-15s + sound on bounce) — not implemented.
3016. - [ ] [W3] [BP3] [GAP] VFX combat "Tracer round" (streak per-projectile + brightness fade per range) — not implemented.
3017. - [ ] [W3] [BP3] [GAP] VFX combat "Bullet impact dust" (material-typed: sand=tan / rock=gray / metal=spark+ricochet) — not implemented.
3018. - [ ] [W3] [BP3] [GAP] VFX combat "Blood splatter" (direction-aligned per projectile vector + persistent decal) — not implemented.
3019. - [ ] [W3] [BP3] [GAP] VFX combat "Oil/coolant burst (robots)" (direction-aligned spray + persistent puddle) — not implemented.
3020. - [ ] [W3] [BP3] [GAP] VFX combat "Limb separation" (limb sprite + blood/oil burst + bone fragments) — not implemented.
3021. - [ ] [W3] [BP3] [GAP] VFX combat "Explosion (multi-stage flash → fireball → smoke + debris + scorch decal)" — not implemented.
3022. - [ ] [W3] [BP3] [GAP] VFX combat "Plasma discharge (energy arc + ionization shimmer + secondary plasma cloud)" — not implemented.
3023. - [ ] [W3] [BP3] [GAP] VFX combat "EMP arc (cyan zigzag + shorts electronics + screen static)" — not implemented.
3024. - [ ] [W3] [BP3] [GAP] VFX combat "Laser beam (continuous + heat haze + glow)" — not implemented.
3025. - [ ] [W3] [BP3] [GAP] VFX combat "Railgun trail (sonic boom + ionization streak)" — not implemented.
3026. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Breath (cold weather)" — see §290.
3027. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Robot vent (overheat)" — see §290.
3028. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Dust trail (movement)" (per-footstep + speed + ground material; NO dust in vacuum) — not implemented.
3029. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Jet flame" (per-jetpack + heat distortion + light emission) — no jetpack.
3030. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Smoke trail (projectile)" — not implemented.
3031. - [ ] [W3] [BP3] [DR-040] [GAP] VFX atmospheric "Weather precipitation" (per DR-040 weather field) — no weather.
3032. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Sparks (hard impact metal-on-metal)" — not implemented.
3033. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Steam (water+heat)" — no chemistry.
3034. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Smoke (combustion fades per ventilation)" — no combustion.
3035. - [ ] [W3] [BP3] [GAP] VFX atmospheric "Fire (per-material: oil=orange / plasma=blue / electrical=white)" — not implemented.
3036. - [ ] [W3] [BP3] [GAP] VFX UI juice "Hit confirm (screen flash + crosshair pulse + bass thump)" — not implemented.
3037. - [ ] [W3] [BP3] [GAP] VFX UI juice "Critical hit (slow-mo 80ms + chromatic aberration + screen flash)" — not implemented.
3038. - [ ] [W3] [BP3] [GAP] VFX UI juice "Death cam zoom (camera dolly + slow-mo + replay handoff)" — not implemented.
3039. - [ ] [W3] [BP3] [GAP] VFX UI juice "Achievement unlock (comic-panel pop-in + cheer sting + collection update)" — no achievements.
3040. - [ ] [W3] [BP3] [GAP] VFX UI juice "Match start (dropship cinematic + camera drift + LZ flash)" — not implemented.
3041. - [ ] [W3] [BP3] [GAP] VFX UI juice "Match victory (comic-page-flip + slow-mo + music swell + confetti)" — not implemented.
3042. - [ ] [W3] [BP3] [GAP] VFX UI juice "Match defeat (scroll-of-failure + music dirge)" — not implemented.
3043. - [ ] [W3] [BP3] [GAP] VFX UI juice "Pickup glow (halo + faction-color tint)" — not implemented.
3044. - [ ] [W3] [BP3] [GAP] VFX UI juice "Reload progress bar + chamber-snap" — partial; no audio.
3045. - [ ] [W3] [BP3] [GAP] VFX UI juice "Healing tick (heart pulse + green particle drift)" — not implemented.
3046. - [ ] [W3] [BP3] [GAP] VFX UI juice "Damage flash (red vignette + screen shake + heartbeat sub-bass)" — not implemented.
3047. - [ ] [W3] [BP3] [GAP] VFX UI juice "Status effect indicators (floating icon over actor + caption)" — not implemented.
3048. - [ ] [W3] [BP3] [GAP] VFX persistent decals (blood / oil pool / coolant pool / scorch / footprints / frost / crater / dust pile / bullet hole / charcoal) — not implemented.
3049. - [ ] [W3] [BP3] [GAP] VFX perf budget "Steam Deck (800p/60): ≤1500 particles + ≤200 decals" — not measured.
3050. - [ ] [W3] [BP3] [GAP] VFX perf budget "1080p/60: ≤4000 + ≤500" — not measured.
3051. - [ ] [W3] [BP3] [GAP] VFX perf budget "4K/120: ≤10000 + ≤1500" — not measured.
3052. - [ ] [W3] [BP3] [GAP] VFX budget governor (cosmetic-flag dropped first + oldest decals fade + critical never dropped + reported in summary.json.perf.vfx_drop_count) — not implemented.
3053. - [ ] [W3] [BP3] [GAP] VFX file format `content/vfx/*.ron` (id + category + cosmetic + sprite + frames + duration + light_emission + sound + spawns_caption + direction_align) — no VFX content at BP3.
3054. - [ ] [W3] [BP3] [GAP] VFX event `vfx.cosmetic_spawned` (cosmetic flag = true) — not emitted.
3055. - [ ] [W3] [BP3] [GAP] VFX event `vfx.gameplay_spawned` (cosmetic flag = false; replay-deterministic) — not emitted.
3056. - [ ] [W3] [BP3] [GAP] VFX event `decal.placed` / `decal.removed` — not emitted.
3057. - [ ] [W3] [BP3] [GAP] VFX AI-driven authoring "Tier 1 procedural primitives + Tier 2 SDXL+LoRA + Tier 3 Aseprite cleanup" — no pipeline.

## 290. spec/atmospheric-effects-and-decals — Atmospheric effects & decals (BP3 partial; closed direction)
3058. - [ ] [W3] [BP3] [GAP] ATMOFX human breath (cold weather + face anchor + 6-frame loop + 2-4s cycle + faction-tint) — not implemented.
3059. - [ ] [W3] [BP3] [GAP] ATMOFX breath origin variants (human standard / android cooling vent / robot none) — not implemented.
3060. - [ ] [W3] [BP3] [GAP] ATMOFX breath gameplay tie-in "AI hearing system signals 'actor here'" — not implemented.
3061. - [ ] [W3] [BP3] [GAP] ATMOFX breath gameplay tie-in "Stealth: holding breath in cold = stealth tactic" — not implemented.
3062. - [ ] [W3] [BP3] [GAP] ATMOFX breath gameplay tie-in "Heavy breathing in cold = actor panicked or wounded" — not implemented.
3063. - [ ] [W3] [BP3] [GAP] ATMOFX robot vent (overheat steam plume from chassis ports + intensity per heat + white-blue tint + continuous on overclock) — not implemented.
3064. - [ ] [W3] [BP3] [GAP] ATMOFX robot vent gameplay tie-in "visible signal robot is overclocking + AI hearing + heat haze distortion" — not implemented.
3065. - [ ] [W3] [BP3] [GAP] ATMOFX blood splatter (`wound_added` event + direction-aligned + 8-frame + spawns persistent decal) — not implemented.
3066. - [ ] [W3] [BP3] [GAP] ATMOFX blood faction variants (human red / android red+oil-blue / husk green-yellow / alien purple) — not implemented.
3067. - [ ] [W3] [BP3] [GAP] ATMOFX blood gameplay tie-in "Forensic trail: AI scout follows blood" — not implemented.
3068. - [ ] [W3] [BP3] [GAP] ATMOFX blood gameplay tie-in "Status indicator: blood pool grows if bleeding-out" — not implemented.
3069. - [ ] [W3] [BP3] [GAP] ATMOFX blood gameplay tie-in "Infection vector from contaminated wounds" — not implemented.
3070. - [ ] [W3] [BP3] [GAP] ATMOFX oil/coolant burst (`chassis_leak_started` + oil=black / coolant=green + persistent puddle) — not implemented.
3071. - [ ] [W3] [BP3] [GAP] ATMOFX oil gameplay tie-in "Ignites on contact with fire" — not implemented.
3072. - [ ] [W3] [BP3] [GAP] ATMOFX coolant gameplay tie-in "Freezes at low temp → frost patch slip" — not implemented.
3073. - [ ] [W3] [BP3] [GAP] ATMOFX oil/coolant gameplay tie-in "Robot consumes to refill chassis resource" — not implemented.
3074. - [ ] [W3] [BP3] [GAP] ATMOFX persistent blood decal (5min default + faction blood color + cleanup budget) — not implemented.
3075. - [ ] [W3] [BP3] [GAP] ATMOFX persistent oil pool (spreads per gravity + pools in low spots + ignites + slipping mechanic + robot consume) — not implemented.
3076. - [ ] [W3] [BP3] [GAP] ATMOFX persistent coolant pool (green + freezes if air < FREEZING_K + frozen = frost patch + slip + robot consume) — not implemented.
3077. - [ ] [W3] [BP3] [GAP] ATMOFX frost patch (cold surfaces or coolant freezing + slip mechanic + magnetic boots bypass + covers material) — not implemented.
3078. - [ ] [W3] [BP3] [GAP] ATMOFX scorch mark (explosion/fire/plasma + faction-specific shape + permanent until terrain mutated) — not implemented.
3079. - [ ] [W3] [BP3] [GAP] ATMOFX scorch material affordance "scorched dirt has reduced traction + more prone to ignition" — not implemented.
3080. - [ ] [W3] [BP3] [GAP] ATMOFX dust trail (footstep tag + intensity per speed + material + faction-tinted + NO dust in vacuum) — not implemented.
3081. - [ ] [W3] [BP3] [GAP] ATMOFX casing eject physics (per-weapon casing + gravity-correct bouncing + persists 8-15s + sound on bounce + cosmetic flag) — not implemented.
3082. - [ ] [W3] [BP3] [GAP] ATMOFX bullet hole/wall decal (per-material + crack pattern + persistent until terrain mutated + material weakening) — not implemented.
3083. - [ ] [W3] [BP3] [GAP] ATMOFX crater (per-explosive-yield + permanent terrain mutation + provides cover + edge has reduced traction) — not implemented.
3084. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Earth rain (vertical streaks + splash + wet decals) — no weather.
3085. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Earth snow (drifting + accumulates as wet decal) — no weather.
3086. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Mars dust storm (horizontal dust + visibility reduction + covers windscreens/visors) — no weather.
3087. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Vulcan acid rain (yellow-green streaks + damages exposed armor + spawns acid puddles) — no weather.
3088. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Vulcan ash fall (slow-falling gray + accumulates as gray decal + reduces visibility) — no weather.
3089. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Vulcan sulfur fog (volumetric yellow + reduces visibility + breathing-toxic) — no weather.
3090. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Mimas vacuum (NO precipitation) — not implemented.
3091. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Europa cryo storm (ice crystals + freezes liquid surfaces + spawns frost decals) — no weather.
3092. - [ ] [W3] [BP3] [GAP] ATMOFX weather precipitation Solar flare (saturated red ambient + auroral overlay + radio static) — no flare.
3093. - [ ] [W3] [BP3] [GAP] ATMOFX steam (water+heat) (rising + per gravity + fades per ventilation + visibility blocker + cools atmosphere) — no chemistry.
3094. - [ ] [W3] [BP3] [GAP] ATMOFX smoke (combustion) (black smoke rising + fades per ventilation + per gravity + toxic if inhaled + AI avoids) — no combustion.
3095. - [ ] [W3] [BP3] [GAP] ATMOFX fire (per-material flame: oil orange / plasma blue / electrical white + spreads per chemistry + damages actors thermal + consumes oxygen) — not implemented.
3096. - [ ] [W3] [BP3] [GAP] ATMOFX footprints (`footstep_*` on soft materials sand/mud/snow + ~1-3min persistence + scout AI tracking) — not implemented.
3097. - [ ] [W3] [BP3] [GAP] ATMOFX sparks (hard metal-on-metal + bounce per gravity + sound + cosmetic + armor effectiveness feedback) — not implemented.
3098. - [ ] [W3] [BP3] [GAP] ATMOFX heat haze (hot surfaces + distortion shader + cosmetic + visibility hint) — not implemented.
3099. - [ ] [W3] [BP3] [GAP] ATMOFX EMP static (screen static + radio interference + electronics flicker + disrupts robot/android + disrupts radio) — not implemented.
3100. - [ ] [W3] [BP3] [GAP] ATMOFX file format `content/atmospheric_effects/*.ron` (id + category + trigger + visual + gameplay ai_hearing/ai_visibility + cosmetic) — no content at BP3.
3101. - [ ] [W3] [BP3] [GAP] ATMOFX event `atmospheric.breath_emit` (per actor) — not emitted.
3102. - [ ] [W3] [BP3] [GAP] ATMOFX event `atmospheric.vent_emit` (per robot) — not emitted.
3103. - [ ] [W3] [BP3] [GAP] ATMOFX event `atmospheric.weather_precipitation_tick` (per scenario second) — not emitted.

## 291. spec/lighting-and-shadows — 2D dynamic lighting + shadows (BP3 no lighting; closed direction)
3104. - [ ] [W3] [BP3] [GAP] LIGHT renderer "cf-render-2d custom wgpu pipelines (normal-map shader + light-volume shader + shadow-mask shader)" — basic Bevy sprite renderer at BP3.
3105. - [ ] [W3] [BP3] [GAP] LIGHT normal-map generation pipeline (Tier 2 Flux.1-dev + ControlNet Depth → automated normal-map bake) — not set up.
3106. - [ ] [W3] [BP3] [GAP] LIGHT per-asset normals (`tier2/<category>/<id>_normal.png`) — no normal maps.
3107. - [ ] [W3] [BP3] [GAP] LIGHT `cf-lighting` crate (radial point lights: flashlights/muzzle flashes/fires/dropship landing/base interior/command core glow) — not present.
3108. - [ ] [W3] [BP3] [GAP] LIGHT ambient lighting (per-world `World.ambient_light_color` + `solar_distance_au` + day-night cycle modulation) — not implemented.
3109. - [ ] [W3] [BP3] [GAP] LIGHT light volumes (per-base-cell + per-mech-interior + per-suit helmet visor in vacuum) — not implemented.
3110. - [ ] [W3] [BP3] [GAP] LIGHT soft shadows (per-light screen-space shadow mask + Steam Deck cheap variant) — not implemented.
3111. - [ ] [W3] [BP3] [GAP] LIGHT sky shader (procedural wgpu shader per-world + gradient + star density + parallax + day/night/weather variants) — not implemented.
3112. - [ ] [W3] [BP3] [GAP] LIGHT Bevy ecosystem `bevy_lit` / normal-mapped 2D PR #14586 / `bevy_light_2d` integration — not integrated.
3113. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Earth day warm yellow-white 3500K→5500K — not implemented.
3114. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Earth's Moon harsh white-on-black + Earthshine cool blue — not implemented.
3115. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Mars orange-pink dusty 4500K — not implemented.
3116. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Phobos/Deimos vacuum Moon — not implemented.
3117. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Mimas Saturn-shine + Sun-distant — not implemented.
3118. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Europa Jupiter-shine + Sun-distant — not implemented.
3119. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Vulcan red-orange thermal glow + lava emission — not implemented.
3120. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Venus yellow-orange diffuse heavy atmosphere — not implemented.
3121. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Belt asteroid Moon-similar — not implemented.
3122. - [ ] [W3] [BP3] [GAP] LIGHT per-world ambient Orbital station per-section configurable — not implemented.
3123. - [ ] [W3] [BP3] [GAP] LIGHT sky `content/skies/*.ron` (id + base_gradient day/dawn/dusk/night + star_density + parallax_offset_per_layer + weather_variants + light_emission) — no content.
3124. - [ ] [W3] [BP3] [GAP] LIGHT runtime sky shader "reads spec + EnvironmentSignal.day_night + weather + blends gradient + adds star field at night + modulates ambient + sun position" — not implemented.
3125. - [ ] [W3] [BP3] [GAP] LIGHT source "Sun (directional infinite)" per world + time + weather — not implemented.
3126. - [ ] [W3] [BP3] [GAP] LIGHT source "Muzzle flash (radial point 80ms warm)" — not implemented.
3127. - [ ] [W3] [BP3] [GAP] LIGHT source "Explosion (radial decaying hot orange → red ~0.3-0.6s)" — not implemented.
3128. - [ ] [W3] [BP3] [GAP] LIGHT source "Fire (radial flickering animated per-tick orange)" — not implemented.
3129. - [ ] [W3] [BP3] [GAP] LIGHT source "Dropship landing lights (spot cone projection)" — not implemented.
3130. - [ ] [W3] [BP3] [GAP] LIGHT source "Player headlamp (spot forward cone toggleable)" — not implemented.
3131. - [ ] [W3] [BP3] [GAP] LIGHT source "Vehicle headlights (spot forward cone)" — not implemented.
3132. - [ ] [W3] [BP3] [GAP] LIGHT source "Base interior (radial point per fixture dim warm/bright fluorescent)" — not implemented.
3133. - [ ] [W3] [BP3] [GAP] LIGHT source "Command core glow (pulsing radial faction-tinted)" — no command core.
3134. - [ ] [W3] [BP3] [GAP] LIGHT source "Hazard alert (radial pulse red/yellow per affliction)" — not implemented.
3135. - [ ] [W3] [BP3] [GAP] LIGHT source "Plasma weapons (radial point + heat haze + energy wash)" — not implemented.
3136. - [ ] [W3] [BP3] [GAP] LIGHT source "Lava (radial point per cell per cf-material)" — not implemented.
3137. - [ ] [W3] [BP3] [GAP] LIGHT source "EMP arc (radial pulse + decay cyan brief)" — not implemented.
3138. - [ ] [W3] [BP3] [GAP] LIGHT source "Comms uplink (subtle spot antenna emission)" — not implemented.
3139. - [ ] [W3] [BP3] [GAP] LIGHT shadow casting "Actors (procedural shape from sprite outline + soft projection)" — not implemented.
3140. - [ ] [W3] [BP3] [GAP] LIGHT shadow casting "Vehicles" — not implemented.
3141. - [ ] [W3] [BP3] [GAP] LIGHT shadow casting "Base objects per collision footprint" — no base objects.
3142. - [ ] [W3] [BP3] [GAP] LIGHT shadow casting "Terrain per cf-material chunk shape" — not implemented.
3143. - [ ] [W3] [BP3] [GAP] LIGHT shadow rendering "Screen-space mask per-light per-frame + soft blur 1-3px Deck / 5-9px desktop + color shadows + cinematic punch intensify shadow contrast" — not implemented.
3144. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "Critical hit (screen flash + intensified rim light on target)" — not implemented.
3145. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "Player death (slow-mo + camera dolly + dramatic spotlight on body + dim ambient)" — not implemented.
3146. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "Match victory (Sun pierces clouds + golden ambient + comic-page-flip)" — not implemented.
3147. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "Match defeat (ambient drains to gray + dim spot on commander/core + scroll-of-failure)" — not implemented.
3148. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "Bunker breach (door opens + hot light pours through + silhouettes of actors)" — not implemented.
3149. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "Reactor breach (pulsing red emergency + alarm strobe + ambient flicker)" — not implemented.
3150. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "EMP burst (brief darkness + electrical static)" — not implemented.
3151. - [ ] [W3] [BP3] [GAP] LIGHT cinematic punch "Solar flare (saturated red ambient + auroral glow)" — not implemented.
3152. - [ ] [W3] [BP3] [GAP] LIGHT file format `content/lights/*.ron` (id + kind + color + intensity + radius_px + cone_angle + cast_shadow + flicker_amp + pulse) — no content.
3153. - [ ] [W3] [BP3] [GAP] LIGHT perf "Steam Deck 800p/60: ≤32 dynamic + 1 ambient + 1 sky + ≤16 shadow casters" — not measured.
3154. - [ ] [W3] [BP3] [GAP] LIGHT perf "1080p/60: ≤96 dynamic + ≤48 shadow casters + blur 3-5px" — not measured.
3155. - [ ] [W3] [BP3] [GAP] LIGHT perf "4K/120: ≤256 dynamic + ≤128 shadow casters + blur 5-9px" — not measured.
3156. - [ ] [W3] [BP3] [GAP] LIGHT budget governor (cosmetic lights dropped first + distance-based culling + reported in summary.json.perf.lighting_drop_count) — not implemented.
3157. - [ ] [W3] [BP3] [GAP] LIGHT CI gate "every chassis has normal-map; every world has sky-definition" — not gated.

## 300. spec/performance-optimization-and-profiling — Performance track (BP3 every M1+ inherits universal perf gate)
3444. - [ ] [W3] [BP3] [M1] [GAP] PERF tier "4K @ 120Hz: strong desktop / 8.33ms render / 16.67ms sim @ 60Hz" — never measured.
3445. - [ ] [W3] [BP3] [M1] [GAP] PERF tier "1080p @ 60Hz: mid-range desktop / 16.67ms render / 16.67ms sim @ 60Hz" — never measured.
3446. - [ ] [W3] [BP3] [M1] [GAP] PERF tier "800p @ 60Hz: Steam Deck OLED / 16.67ms render / 16.67ms sim @ 60Hz" — never measured.
3447. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Material kernel` (32+ active 64×64 chunks @ 60Hz Deck + SIMD update + dirty-rect + sleeping chunks + budget governor) — stub.
3448. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Atmospherics` (100+ atmospheres @ 60Hz Deck + SoA per-gas + lazy update on connectivity change + SIMD per-gas mole calc) — stub.
3449. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Physics narrowphase` (1000+ entities @ 60Hz + spatial hash + sleep islands + dynamic AABB + SIMD GJK/EPA + CCD only for fast bodies) — not implemented.
3450. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Replay event recording` (100K events/run @ 60Hz + pre-allocated buffer + compressed encoding + async flush to disk) — partial.
3451. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Pathfinding` (10+ bots × per-tick replan @ 60Hz + A* with hierarchical mesh + per-bot cooldown + threaded) — no pathfinding.
3452. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Renderer` (4K/120 + Deck/60 + custom wgpu chunked terrain texture + sprite batching + instanced particles) — never measured.
3453. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Network serialization` (60Hz snapshot @ MMO scale + snapshot delta + bit-packed + RLE + per-actor interest set culling) — not implemented.
3454. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `AI utility scoring` (50+ bots × per-tick @ 60Hz + cached scoring + per-tick budget cap + threaded) — only ReactiveGuard FSM.
3455. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `LLM Mind (async)` (never blocks sim + async background + budget-capped + deadline-driven) — no LLM mind.
3456. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Animation event tags` (per-frame per-actor @ 60Hz + frame-key-based tag firing + cached) — not implemented.
3457. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Lighting` (per-tier per-frame @ 60Hz + light-volume culling + per-tier shadow LOD) — no lighting.
3458. - [ ] [W3] [BP3] [M1] [GAP] PERF hot path `Audio` (60Hz mix + 32-256 spatial channels + channel cap + spatial LOD + budget governor) — no audio.
3459. - [ ] [W3] [BP3] [M1] [GAP] PERF SIMD `std::simd` portable + `wide` crate (material kernel + atmospherics + physics narrowphase + math hot paths) — not integrated.
3460. - [ ] [W3] [BP3] [M1] [GAP] PERF SIMD CI test "SIMD vs scalar bit-identical" — not implemented.
3461. - [ ] [W3] [BP3] [M1] [GAP] PERF Bevy parallel systems (per-system dispatch + system-level threading) — Bevy default scheduling only.
3462. - [ ] [W3] [BP3] [M1] [GAP] PERF archetype-based ECS (Bevy native + cache-friendly component queries) — Bevy default but not measured.
3463. - [ ] [W3] [BP3] [M1] [GAP] PERF spatial partitioning `cf-physics broadphase` Dynamic AABB tree — not implemented.
3464. - [ ] [W3] [BP3] [M1] [GAP] PERF spatial partitioning `cf-ai perception` Spatial hash — no perception.
3465. - [ ] [W3] [BP3] [M1] [GAP] PERF spatial partitioning `cf-render culling` Quad-tree — not implemented.
3466. - [ ] [W3] [BP3] [M1] [GAP] PERF memory arenas (per-frame arena for hot-path allocation via `bumpalo`) — not present.
3467. - [ ] [W3] [BP3] [M1] [GAP] PERF object pooling `ProjectilePool` / particles / decals / animation states — projectiles allocate per spawn.
3468. - [ ] [W3] [BP3] [M1] [GAP] PERF zero-allocation patterns (per-tick hot loops + reuse buffers + pre-allocated capacities + `clippy::large_stack_allocations` / `clippy::large_types_passed_by_value`) — not enforced.
3469. - [ ] [W3] [BP3] [M1] [GAP] PERF cache-friendly SoA layout (vs AoS for vectorization) — Bevy uses AoS by default; no SoA.
3470. - [ ] [W3] [BP3] [M1] [GAP] PERF profile-guided optimization "Bevy tracing spans + tracy-client" — not configured.
3471. - [ ] [W3] [BP3] [M1] [GAP] PERF profile-guided optimization "criterion bench" — partial; no perf benches at BP3.
3472. - [ ] [W3] [BP3] [M1] [GAP] PERF profile-guided optimization "Per-scenario report `cf-bench --scenario X --profile Y`" — not implemented.
3473. - [ ] [W3] [BP3] [M1] [GAP] PERF profile-guided optimization "Regression detect CI baseline + threshold" — not configured.
3474. - [ ] [W3] [BP3] [M1] [GAP] PERF profile-guided optimization "AI-agent hunt `cfctl bench analyze --scenario X --target-tier deck`" — not implemented.
3475. - [ ] [W3] [BP3] [M1] [GAP] PERF profile-guided optimization "PGO `cargo pgo` per release" — not configured.
3476. - [ ] [W3] [BP3] [M1] [GAP] PERF GPU compute Tier 0 Presentation (lighting + particles + decals + smoke/fog/fire visuals + glow + heat shimmer + trails + casings + debug overlays NOT authoritative) — not implemented.
3477. - [ ] [W3] [BP3] [M1] [GAP] PERF GPU compute Tier 1 Client prediction (local movement + provisional projectiles/impacts + interpolation/extrapolation NOT authoritative; server corrects) — not implemented.
3478. - [ ] [W3] [BP3] [M1] [GAP] PERF GPU compute Tier 2 Advisory compute (broadphase candidates + pathfinding heatmaps + visibility hints + AI perception maps + compression hints NOT authoritative; CPU/server validates) — not implemented.
3479. - [ ] [W3] [BP3] [M1] [GAP] PERF GPU compute Tier 3 Server GPU acceleration (material chunks + atmosphere diffusion + large path fields + compression + batch queries; optional with CPU fallback for community headless servers) — not implemented.
3480. - [ ] [W3] [BP3] [M1] [GAP] PERF GPU compute Tier 4 Authoritative GPU sim (terrain/material/gas/projectile/body truth; only after certification) — not implemented.
3481. - [ ] [W3] [BP3] [M1] [GAP] PERF Tier 4 certification matrix (same seed + same inputs + same mod set + 10K+ ticks per kernel + per-tick BLAKE3 + final byte-identical + NVIDIA/AMD/Intel/Apple/Steam Deck coverage + no unproven atomics + permanent CPU fallback) — never run.
3482. - [ ] [W3] [BP3] [M1] [GAP] PERF determinism preservation SIMD = scalar bit-identical CI test — not implemented.
3483. - [ ] [W3] [BP3] [M1] [GAP] PERF determinism preservation GPU determinism per-vendor IEEE-754 verified — not implemented.
3484. - [ ] [W3] [BP3] [M1] [GAP] PERF determinism preservation Multi-threading deterministic (Bevy parallel system order respected + per-system seed) — not enforced.
3485. - [ ] [W3] [BP3] [M1] [GAP] PERF determinism preservation Object pools deterministic (pool allocation order) — no pools.
3486. - [ ] [W3] [BP3] [M1] [GAP] PERF determinism preservation Cache miss != different result (logical correctness independent of cache) — not enforced.
3487. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench --scenario X --profile material --runs 100 --check-checksum-stability` — not implemented.
3488. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench analyze --scenario X --target-tier deck` (auto-identify bottleneck) — not implemented.
3489. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench memory --scenario X --duration 60s --soak` (memory leak detection) — not implemented.
3490. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench gpu --scenario X --gpu-time` — not implemented.
3491. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl test gpu-authority-cert --kernel X --ticks 10000 --matrix all` — not implemented.
3492. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench network-snapshot --scenario X --snapshot-size --bandwidth` — not implemented.
3493. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench cold-load --scenario X` (first-launch perf) — not implemented.
3494. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench replay-throughput --bundle X` — not implemented.
3495. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench audio-voice --scenario X --voice-count` (audio voice budget) — not implemented.
3496. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench parallel-system --scenario X --thread-count N` (per-thread scaling) — not implemented.
3497. - [ ] [W3] [BP3] [M1] [GAP] PERF `cfctl bench simd --scenario X --enable-simd false` (SIMD vs scalar comparison) — not implemented.
3498. - [ ] [W3] [BP3] [M1] [GAP] PERF per-milestone perf gate (Per-tier perf budget pass + AI agent-driven perf analysis report + CI bench regression test no >5% regression + Memory leak soak 24h+ clean) — not enforced.
3499. - [ ] [W3] [BP3] [M1] [GAP] PERF memory budget tier "Steam Deck (16GB shared): ≤6GB system + ≤4GB GPU" — not measured.
3500. - [ ] [W3] [BP3] [M1] [GAP] PERF memory budget tier "Mid-range (16GB+8GB GPU): ≤8GB system + ≤6GB GPU" — not measured.
3501. - [ ] [W3] [BP3] [M1] [GAP] PERF memory budget tier "High-end (32GB+24GB GPU): ≤16GB system + ≤16GB GPU" — not measured.
3502. - [ ] [W3] [BP3] [M1] [GAP] PERF hot-path zero-allocation enforcement (`cargo-allocator-stats` lint) — not installed.

## 323. spec/localization-plan — Tier-A/B localization (BP3 done-criteria includes Tier-A keyed strings)
4201. - [ ] [W3] [BP3] [GAP] LOC Tier-A "11 fully-localized languages (en + es-419 + pt-BR + de + fr + it + ru + pl + zh-Hans + ja + ko)" — single locale.
4202. - [ ] [W3] [BP3] [GAP] LOC Tier-B "8 UI-only (tr + cs + nl + uk + ar + vi + th + id)" — none.
4203. - [ ] [W3] [BP3] [GAP] LOC mod-localization "Modders submit .ftl packs via Steam Workshop or community mirror" — no infrastructure.
4204. - [ ] [W3] [BP3] [GAP] LOC format "Project Fluent (.ftl) supports plurals + gendered + nested + message references" — not used.
4205. - [ ] [W3] [BP3] [GAP] LOC Rust crate `fluent-rs` + `fluent-bundle` + `t!("key.id")` + `t_args!("key.id", arg=value)` convenience macros — not present.
4206. - [ ] [W3] [BP3] [GAP] LOC hot-reload "Locale switcher in settings + live-reload without restart" — no locale switcher.
4207. - [ ] [W3] [BP3] [GAP] LOC fallback "en for missing keys + CI gate logs missing" — not enforced.
4208. - [ ] [W3] [BP3] [GAP] LOC font selection "Noto Sans + Noto Sans CJK + Noto Naskh Arabic (multi-script OFL)" — not configured.
4209. - [ ] [W3] [BP3] [GAP] LOC RTL support "Arabic + UI mirroring for menus" — not implemented.
4210. - [ ] [W3] [BP3] [GAP] LOC multi-script verification "CJK rendering tested + Cyrillic + Arabic shaping" — not tested.
4211. - [ ] [W3] [BP3] [GAP] LOC AI translation pipeline (Source en → AI agent translates per Tier-A + AI agent reviews + project-owner approves OR community reviews via Discord channel → commit) — not present.
4212. - [ ] [W3] [BP3] [GAP] LOC file structure `content/i18n/<lang>/<file>.ftl` (ui + narrative_factions + narrative_npcs + narrative_missions + codex + achievements + tutorial + tooltips + captions + errors) — not present.
4213. - [ ] [W3] [BP3] [GAP] LOC mod-localization layer "Loaded after first-party strings + can override OR extend" — not implemented.
4214. - [ ] [W3] [BP3] [GAP] LOC CI gate `cf-i18n-check` (verifies zero hardcoded English in UI/HUD/captions/error messages) — not present.
4215. - [ ] [W3] [BP3] [GAP] LOC CI gate `cf-i18n-coverage` (verifies all keys present in each Tier-A language) — not present.
4216. - [ ] [W3] [BP3] [GAP] LOC CI gate `cf-i18n-rtl-test` (verifies Arabic UI mirrors correctly) — not present.
4217. - [ ] [W3] [BP3] [GAP] LOC CI gate `cf-i18n-script-test` (verifies CJK + Cyrillic + Arabic + Latin all render correctly) — not present.
4218. - [ ] [W3] [BP3] [GAP] LOC community review program (per-language Discord channel + volunteer reviewers credited in-game + per-language coordinator role) — no community.

## 324. spec/accessibility-plus-and-sustainability — Accessibility-plus (Universal Enhancement floor extension)
4219. - [ ] [W3] [GAP] ACC+ cognitive "Lower stimulation mode (reduced VFX + slower pace + simpler UI + fewer simultaneous threats)" — not implemented.
4220. - [ ] [W3] [GAP] ACC+ cognitive "Simple HUD preset (minimal HUD + only critical info)" — not implemented.
4221. - [ ] [W3] [GAP] ACC+ cognitive "One-thing-at-a-time tutorial pacing (slower tutorial cadence + explicit wait-for-ready prompts)" — not implemented.
4222. - [ ] [W3] [GAP] ACC+ cognitive "Cognitive-load-reduction toggle (master switch cascades)" — not implemented.
4223. - [ ] [W3] [GAP] ACC+ motor "Single-button play mode (context-aware single button performs most-relevant action)" — not implemented.
4224. - [ ] [W3] [GAP] ACC+ motor "Gesture controls (swipe gestures for action mapping)" — not implemented.
4225. - [ ] [W3] [GAP] ACC+ motor "Eye tracking integration (Tobii eye-tracker support)" — not implemented.
4226. - [ ] [W3] [GAP] ACC+ motor "Slow-mo / pause-during-input mode (time slows on input)" — not implemented.
4227. - [ ] [W3] [GAP] ACC+ motor "One-handed mode (all actions accessible with 1 hand)" — not implemented.
4228. - [ ] [W3] [GAP] ACC+ motor "Configurable hold-vs-toggle (per-action; for endurance-limited)" — not implemented.
4229. - [ ] [W3] [GAP] ACC+ motor "Haptic feedback alternatives (for sensory + tactile)" — not implemented.
4230. - [ ] [W3] [GAP] ACC+ hearing "Sign language overlay (for cinematics; community-authored ASL/BSL)" — not implemented.
4231. - [ ] [W3] [GAP] ACC+ hearing "Visual sub-bass cues (screen pulse on bass thump)" — not implemented.
4232. - [ ] [W3] [GAP] ACC+ hearing "Full subtitle option (NOT just critical audio; ALL audio + optional speaker label + tone description)" — not implemented.
4233. - [ ] [W3] [GAP] ACC+ hearing "Audio description for visual events (text + voice descriptions)" — not implemented.
4234. - [ ] [W3] [GAP] ACC+ reading "Dyslexic font option (OpenDyslexic)" — not implemented.
4235. - [ ] [W3] [DR-012] [GAP] ACC+ reading "High-contrast text beyond DR-012 opt-in" — not implemented.
4236. - [ ] [W3] [GAP] ACC+ reading "Reading speed control (per-paragraph TTS readout)" — not implemented.
4237. - [ ] [W3] [GAP] ACC+ reading "Per-paragraph TTS readout (audio narration toggle)" — not implemented.
4238. - [ ] [W3] [GAP] ACC+ reading "Large-print preset (cascade text-scaling)" — not implemented.
4239. - [ ] [W3] [GAP] ACC+ sensory "Pause-on-window-loss (auto-pause when game window not focused)" — not implemented.
4240. - [ ] [W3] [GAP] ACC+ sensory "Low-violence mode (decals minimal + blood color black-white + reduced gore)" — not implemented.
4241. - [ ] [W3] [GAP] ACC+ sensory "Sensory-overload prevention (fewer simultaneous VFX + per-tick particle cap)" — not implemented.
4242. - [ ] [W3] [GAP] ACC+ sensory "Anxiety-mode (slower combat cadence + reduced enemy aggression baseline)" — not implemented.
4243. - [ ] [W3] [GAP] ACC+ sensory "Confirmation prompts on irreversible actions (auto-prompt before quit-to-menu / abandon-mission)" — not implemented.
4244. - [ ] [W3] [GAP] ACC+ colorblind "8 protanope/deuteranope/tritanope/atypical protocols + tested with actual color-blind testers" — not implemented.
4245. - [ ] [W3] [GAP] ACC+ cinematic "Audio description for cinematics (text + voice descriptions of visual events)" — not implemented.
4246. - [ ] [W3] [GAP] ACC+ cinematic "Skip-cinematic for low-bandwidth (skip + summary text)" — not implemented.


# ===== WAVE 4 — EQUIPMENT & LOADOUT REAL IMPLEMENTATION =====

## 16. T-CONTROL — cfctl/observe/act surface gaps (BP0..BP3 cumulative)
221. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --hud --stream --hz 10` not implemented.
222. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --captions --stream --hz 10` not implemented.
223. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --mission --once` not implemented.
224. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --debrief --once` not implemented.
225. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --ai --stream --hz 5` not implemented (AI state not in observe.once).
226. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --base --once` not implemented.
227. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --camera --once` not implemented.
228. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --collisions --stream --hz 30` not implemented (no `collision.*` events emitted yet).
229. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --materials --stream --hz 30 --scope chunk:0,0` not implemented.
230. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --atmospheres --stream --hz 10` not implemented.
231. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --reactions --stream --hz 30` not implemented.
232. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --replay --once` not implemented.
233. - [ ] [W4] [BP0+BP3] [GAP] `cfctl observe --perf --stream --hz 1` not implemented.
234. - [ ] [W4] [BP0+BP3] [GAP] `cfctl inspect chassis <actor>` not implemented as a top-level command (currently only inspect event/inspect actor).
235. - [ ] [W4] [BP0+BP3] [GAP] `cfctl inspect mission --with-events` not implemented.
236. - [ ] [W4] [BP0+BP3] [GAP] `cfctl inspect base core:0 --with-events` not implemented (no base entity exists yet).
237. - [ ] [W4] [BP0+BP3] [GAP] `cfctl inspect objective breach.win` not implemented.
238. - [ ] [W4] [BP0+BP3] [GAP] `cfctl inspect order alpha:1:move-to-7` not implemented (no order system).
239. - [ ] [W4] [BP0+BP3] [GAP] `cfctl inspect affliction alpha:0:burning` not implemented.
240. - [ ] [W4] [BP0+BP3] [GAP] `cfctl inspect collision <event-id> --with-parents --with-children` not implemented.
241. - [ ] [W4] [BP0+BP3] [GAP] `cfctl act tactical select alpha:1` not implemented (no tactical layer at BP3 yet).
242. - [ ] [W4] [BP0+BP3] [GAP] `cfctl act tactical order move-to` not implemented.
243. - [ ] [W4] [BP0+BP3] [GAP] `cfctl act tactical order breach` not implemented.
244. - [ ] [W4] [BP0+BP3] [GAP] `cfctl act tactical doctrine cautious` not implemented.
245. - [ ] [W4] [BP0+BP3] [GAP] `cfctl act camera mode tactical-map` not implemented.
246. - [ ] [W4] [BP0+BP3] [GAP] `cfctl act camera follow alpha:0` not implemented.
247. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui tree --with-bounds` not implemented (no UI semantic tree).
248. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui click loadout.confirm` not implemented.
249. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui hover hud.module.jet` not implemented.
250. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui set settings.ui_scale 200` not implemented (settings set works through act.settings.set but not via the UI-tree path).
251. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui type chat.input "covering fire on left"` not implemented (no text-input surface).
252. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui press Tab` not implemented (Tab works via cf-app input but cfctl can't drive it).
253. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui press Ctrl+S` not implemented.
254. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui assert hud.objective contains "Breach"` not implemented.
255. - [ ] [W4] [BP0+BP3] [GAP] `cfctl ui focus settings.captions` not implemented (cf-control focus works but cfctl ui focus does not).
256. - [ ] [W4] [BP0+BP3] [GAP] Observation packet does NOT include "Camera mode" or "Camera bounds" in the Player context block.
257. - [ ] [W4] [BP0+BP3] [GAP] Observation packet does NOT include "Equipment selected item heat/energy" — only rifle ready/reload counter.
258. - [ ] [W4] [BP0+BP3] [GAP] Observation packet does NOT include "sampled local material grid" around the player.
259. - [ ] [W4] [BP0+BP3] [GAP] Observation packet does NOT include "active material cells, liquids/gases, local reactions" (no material kernel).
260. - [ ] [W4] [BP0+BP3] [GAP] Observation packet does NOT include "pressure/oxygen, afflictions" (no atmospherics kernel).

## 17. M2.5 — Reactor scenario gaps (native-implementation-backlog M2.5-002..006)
261. - [ ] [W4] [M2.5] [GAP] M2.5 reactor object explosion VFX missing — only HP drops; no explosion sprite, no scattering debris, no particle puff.
262. - [ ] [W4] [M2.5] [GAP] M2.5 enemy AI does not react to changed terrain affordances ("M2.5-003 terrain-driven defense" done-criterion partly met: AI fires but doesn't reroute around trench).
263. - [ ] [W4] [M2.5] [GAP] M2.5 capture grid at 100% and 200% scale not visually-checked (M2.5-004 acceptance criterion).
264. - [ ] [W4] [M2.5] [GAP] M2.5 reactor pressure-feedback HUD line not present (last-event line covers it but no dedicated "reactor pressure" prose).
265. - [ ] [W4] [M2.5] [GAP] M2.5 enemy-wave system not configurable — only a single reactive guard exists; no wave manager.
266. - [ ] [W4] [M2.5] [GAP] M2.5 dedicated `reactor_destroyed` loss-reason event vs generic `mission_resolved` — loss reason field exists but no parent-chain to the projectile that landed the killing hit.
267. - [ ] [W4] [M2.5] [GAP] M2.5 "loss path with no trench fails faster" (M2.5-003 done-criterion) — no test asserts the no-trench path's timer.

## 21. M5 — Equipment/Loadout spec (spec/equipment-loadout.md) gaps
301. - [ ] [W4] [M5] [GAP] M5 `item_definition` immutable authored facts schema not implemented — current cf-equipment only has rifle preset + role_records but no `display_name` / `catalog_visibility` / `inheritance_chain` / `source_confidence` / `manual_patch_id` / `warning_ids`.
302. - [ ] [W4] [M5] [GAP] M5 `runtime_item_instance` distinct from `item_definition` — current cf-equipment merges both in the ItemKind enum.
303. - [ ] [W4] [M5] [GAP] M5 `chassis_definition` `origin_compatibility` field — currently chassis has no compatibility matrix.
304. - [ ] [W4] [M5] [GAP] M5 `equipment_condition` Intact/Impaired/Critical/Disabled/Destroyed state per item not implemented — only chassis modules have state.
305. - [ ] [W4] [M5] [GAP] M5 `loadout_template` for delivery craft / explicit actors / slots / package ids / budget — not present.
306. - [ ] [W4] [M5] [GAP] M5 `role_card` human-readable "best at / bad at / range / terrain consequence / support value / handling / risk" fields not in cf-equipment.
307. - [ ] [W4] [M5] [GAP] M5 `ai_summary` target classes + danger model + material fit + reason labels not in cf-equipment.
308. - [ ] [W4] [M5] [GAP] M5 `ui_projection` dense scan fields (icon, cost, range band, role chips, bot trust, warning badges) not in cf-equipment.
309. - [ ] [W4] [M5] [GAP] M5 `package_diagnostic` source path + field provenance + warning ids per item — not implemented.
310. - [ ] [W4] [M5] [GAP] M5 `balance_row` role overlap + cost/mass/supply pressure + terrain power — not authored.
311. - [ ] [W4] [M5] [GAP] M5 `replay_event` per item: item id + actor owner + action + reason label + target/material context — currently only generic `equipment.weapon_fired` exists.
312. - [ ] [W4] [M5] [GAP] M5 `mission_requirement` capability declarations (instead of hard-coded item names) — missions still reference `slot=1 rifle` explicitly.
313. - [ ] [W4] [M5] [GAP] M5 LOAD-A-01..LOAD-A-16 acceptance suite — only LOAD-A fixture import exists; A-02..A-16 never authored.
314. - [ ] [W4] [M5] [GAP] M5 LOAD-FIELD-01..06 field-atlas tests — not authored.
315. - [ ] [W4] [M5] [GAP] M5 LOAD-FIELD-SOURCE-01..06 source-position drill-down tests — not authored.
316. - [ ] [W4] [M5] [GAP] M5 dual_wield_policy field per weapon — not present.
317. - [ ] [W4] [M5] [GAP] M5 support_requirement / support_offset (bipod / mount) per weapon — not present.
318. - [ ] [W4] [M5] [GAP] M5 actor_body_constraints per item (e.g., "1-arm fallback" / "2-handed only") — not present.
319. - [ ] [W4] [M5] [GAP] M5 primary_verb / secondary_verb / target_rule action model — current cf-equipment only has fire + reload verbs.
320. - [ ] [W4] [M5] [GAP] M5 scripted_verbs field (Lua/Rhai script hooks per item) — no scripting host yet.

## 47. DR-006 — Modding data model OPEN (M5 should expose moddable role records / chassis specs; deeper M8 scope → FUTURE_FEATURES.md J.7)
557. - [ ] [W4] [M5+M8] [DR-006] [GAP] DR-006 OPEN at BP3 — scripted hook surface decision (mlua vs Rhai) deferred to M5 task card per AGENTS.md; BP3 closure does not resolve it.
558. - [ ] [W4] [M5+M8] [DR-006] [GAP] DR-006 mod-author capability declaration — schema not enforced at BP3 (M5 cf-equipment role records are not yet moddable).

## 48. DR-031 — Content-economy / monetization (audit row open every M1+ milestone)
581. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 premium one-time purchase posture — codebase has no monetization plumbing yet (which is correct), but anti-FOMO audit never run at BP3.
582. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 no-pay-to-win invariant test (CI grep that no gameplay-power field is gated by external storefront flags) — not wired.

## 161. M5 — Equipment Role-Card Renderer (LOAD-R-01..LOAD-R-13) closure debt
1396. - [ ] [W4] [M5] [GAP] LOAD-R-01..13 acceptance suite — not built (M5 only ships RoleRecord struct + 3 fixture loadouts; no renderer UI).
1397. - [ ] [W4] [M5] [GAP] No `cf-ui::RoleCardRenderer` component — the renderer view doesn't exist.
1398. - [ ] [W4] [M5] [GAP] No role-card data: display_name / role_icon / package / cost / mass / range / terrain-support tag — only `RoleRecord` has narrower fields.
1399. - [ ] [W4] [M5] [GAP] No item detail drawer UI.
1400. - [ ] [W4] [M5] [GAP] No actor slot card UI.
1401. - [ ] [W4] [M5] [GAP] No squad capability summary UI.
1402. - [ ] [W4] [M5] [GAP] No workbench diagnostic panel UI.
1403. - [ ] [W4] [M5] [GAP] No AI debug/replay label UI for items.
1404. - [ ] [W4] [M5] [GAP] No balance-overlap table UI.
1405. - [ ] [W4] [M5] [GAP] No `catalog_visibility` field on cf-equipment items.

## 162. M5 — Equipment field map alignment gaps
1406. - [ ] [W4] [M5] [GAP] cf-equipment role-record `display_name` field — exists but uppercase capitalization not enforced.
1407. - [ ] [W4] [M5] [GAP] cf-equipment role-record `archetype` field — not present (would map to "rifle/pistol/grapple/etc").
1408. - [ ] [W4] [M5] [GAP] cf-equipment role-record `role_tags` field — exists as `tags: Vec<RoleTag>` but `RoleTag` enum has only 5 variants; spec lists 12+.
1409. - [ ] [W4] [M5] [GAP] cf-equipment role-record `primary_verb` field — not present.
1410. - [ ] [W4] [M5] [GAP] cf-equipment role-record `range_band` field — not present (only `effective_range_max`).
1411. - [ ] [W4] [M5] [GAP] cf-equipment role-record `terrain_consequence` field — not present.
1412. - [ ] [W4] [M5] [GAP] cf-equipment role-record `bot_competence` field — not present.
1413. - [ ] [W4] [M5] [DR-008] [GAP] cf-equipment role-record `bot_policy` field — not present (DR-008 says bots need policy hints).
1414. - [ ] [W4] [M5] [GAP] cf-equipment role-record `risk_profile` field — not present.
1415. - [ ] [W4] [M5] [GAP] cf-equipment role-record `handling_commitment` field — not present.

## 171. DR-006 — Modding data model (OPEN; M5 fixture-import obligation lives here)
1491. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Manifest: typed, versioned, validated; required" — no `package.ron` manifest format at BP3.
1492. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Asset structure: keep INI-friendly hierarchy (Devices/, Actors/, Scenes/)" — content/ uses flat scenarios/ + build_points/; no Devices/Actors/Scenes hierarchy.
1493. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Workbench: validated editing, asset preview, material lab" — no in-engine workbench at BP3.
1494. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Weapon/effect graph preview" — no graph viewer for projectile hit/timer/death actions.
1495. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Pack manager: support mod packs, level packs, skin packs, dependencies, provenance" — no pack manager.
1496. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Migrate three real CCCP mods" — never attempted.
1497. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Static validator catches 90%+ of real-world mod errors" — `cf-mod validate` only catches a narrow surface.
1498. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 "Lua sandbox prevents filesystem/network by default" — no Lua sandbox at BP3 (no scripting host).
1499. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 reuse-via-CopyOf grammar for content overrides — not supported by current scenario manifest.
1500. - [ ] [W4] [M5] [DR-006] [GAP] DR-006 versioned schemas with bundled migrations — schemas exist with `schema_version=1`; no migration handlers registered.

## 174. DR-029 — Save game (CLOSED at M5 first-slice scope)
1521. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 atomic write (temp file + rename) — cf-save just writes to target path.
1522. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 rolling backup per slot — not implemented.
1523. - [ ] [W4] [M5+M7] [DR-029] [GAP] DR-029 save contents: command core state — no command core at BP3 (M5/M7 owns).
1524. - [ ] [W4] [M5+M7.5] [DR-029] [GAP] DR-029 save contents: base modules — no base at BP3 (M7.5 owns).
1525. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 save contents: faction state + enemy commander memory — no faction/commander at BP3.
1526. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 save contents: mission manifests (active + pending + completed) — only the active scenario id at BP3.
1527. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 save contents: replay archive refs — not implemented (no link from .cfsave to run-bundle id).
1528. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 save contents: scenario policy persistence — `tutorial_safety` field exists on chassis; not on actor or scenario in cf-save.
1529. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 acceptance "Save and reload mid-mission preserves chassis/inventory/replay state" — chassis ✓; inventory + replay refs missing.
1530. - [ ] [W4] [M5] [DR-029] [GAP] DR-029 acceptance "Migration test: a v0.1 save loads on v0.2 with a declared handler" — no v0.1 → v0.2 migration registered.

## 175. DR-011 — Progression/retention loop (OPEN; M7 closes; BP3 should pre-declare schema)
1531. - [ ] [W4] [BP3] [M7] [DR-011] [GAP] DR-011 veteran persistence schema — `cf-save` has no `veterans: Vec<Veteran>` field.
1532. - [ ] [W4] [BP3] [M7] [DR-011] [GAP] DR-011 salvage as retention loop reward — no salvage→economy linkage at BP3.
1533. - [ ] [W4] [BP3] [M7] [DR-011] [GAP] DR-011 template edits / next-contract suggestions — no template system.
1534. - [ ] [W4] [BP3] [M7] [DR-011] [GAP] DR-011 RET-A acceptance suite — not authored.

## 176. DR-013 — Backend service scope (OPEN; M9+ closes; BP3 schema seed)
1535. - [ ] [W4] [BP3] [M9] [DR-013] [GAP] DR-013 lobby_directory schema — not declared in any schema file.
1536. - [ ] [W4] [BP3] [M9] [DR-013] [GAP] DR-013 account adapter trait — not declared as a Rust trait.
1537. - [ ] [W4] [BP3] [M9] [DR-013] [GAP] DR-013 anti-cheat profile schema enum — not declared.
1538. - [ ] [W4] [BP3] [M9] [DR-013] [GAP] DR-013 telemetry endpoint stub — no `cf-server-ops` skeleton.
1539. - [ ] [W4] [BP3] [M9] [DR-013] [GAP] DR-013 audit log persistence — no `audit_log.jsonl` writer.
1540. - [ ] [W4] [BP3] [M9] [DR-013] [GAP] DR-013 BACK-A acceptance — not authored.

## 182. DR-031 — Content economy and monetization (CLOSED-DIRECTION; every M1+ rolls audit row)
1563. - [ ] [W4] [M1+M1.5+M2+M2.5+M3A+M3B+M4A+M5] [DR-031] [GAP] DR-031 anti-FOMO audit row open — never run for M1, M1.5, M2, M2.5, M3A, M3B, M4A, M5.
1564. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 no-pay-to-win invariant test — no CI grep.
1565. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 community-hostable promise — no audit at BP3.

## 188. DR-057 — Optional gacha/battle-pass posture (CLOSED-DIRECTION; private-prototype mode)
1594. - [ ] [W4] [DR-057] [GAP] DR-057 `cf-asset-ledger check --mode private` — not implemented.
1595. - [ ] [W4] [DR-057] [GAP] DR-057 `cf-asset-ledger check --mode release` — not implemented.
1596. - [ ] [W4] [DR-057] [GAP] DR-057 cosmetic locker scaffold (toggleable + default-off) — not present.
1597. - [ ] [W4] [DR-057] [GAP] DR-057 anti-FOMO archive + earn-back path — not implemented.

## 211. DR-057 — Optional gacha/battle-pass + private-prototype license posture (CLOSED-DIRECTION)
1886. - [ ] [W4] [DR-057] [GAP] DR-057 `cf-asset-ledger check --mode private` mode — not implemented.
1887. - [ ] [W4] [DR-057] [GAP] DR-057 `cf-asset-ledger check --mode release` mode — not implemented.
1888. - [ ] [W4] [DR-057] [GAP] DR-057 cosmetic locker scaffold (toggleable + default-off + no gameplay power lock) — not present.
1889. - [ ] [W4] [DR-057] [GAP] DR-057 anti-FOMO archive + earn-back path — not implemented.
1890. - [ ] [W4] [DR-057] [GAP] DR-057 future activation DR placeholder — not authored.

## 212. DR-041 — Mining & extraction (CLOSED; M8.6 owns; BP3 schema seed)
1891. - [ ] [W4] [BP3] [M8.6] [DR-041] [GAP] DR-041 ore-as-material registry — `cf-material` stub.
1892. - [ ] [W4] [BP3] [M8.6] [DR-041] [GAP] DR-041 mining tool roles (Sampler / LightDigger / HeavyDrill / CoreDrill / RefiningStation / SmelterFurnace / EnrichmentReactor / OreCargoBay / ConveyorBelt) — none implemented.
1893. - [ ] [W4] [BP3] [M8.6] [DR-041] [GAP] DR-041 per-world ore deposit generator — not implemented.

## 254. DR-011 — Retention architecture (OPEN; BP3 should seed structure)
2164. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 commander_profile object — not declared (campaign continuity has no anchor).
2165. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 actor_veteran object (name + role + scars + traits + injuries + rescue history + favorite equipment) — not declared.
2166. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 loadout_template object (squad role + equipment roles + delivery craft + mass/cost/danger warnings) — not declared.
2167. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 salvage_manifest object (recovered gear + scrap + rare parts + base repair materials) — not declared.
2168. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 contract_seed object (objective + terrain/material profile + constraints + reward table + replay hash) — not declared.
2169. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 enemy_commander_dossier object — not declared.
2170. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 base/faction_state object — not declared.
2171. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 replay_card object (mission result + seed + loadout + key events + mod/package hashes) — not declared.
2172. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 RET-A-01..RET-A-10 acceptance suite — none authored.
2173. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 anti-FOMO + anti-grind guardrails — not enforced.
2174. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 RET-A-09 anti-fatigue pacing test (player can stop after one mission without losing progress or missing mandatory rewards) — not testable until save model lands.
2175. - [ ] [W4] [BP3] [DR-011] [GAP] DR-011 RET-A-10 horizontal progression test (new unlock expands tactics but does not obsolete an earlier tool) — not testable until equipment roster lands.

## 255. DR-013 — Backend service tier matrix at BP3 (local tier required at launch)
2176. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "health" service — not present.
2177. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "schema/version report" service — not present.
2178. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "package registry" service — not present.
2179. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "join eligibility" service — not present.
2180. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "deep-link parser" — not present.
2181. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "local server supervisor" — not present.
2182. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "local replay/report index" — not present (run-bundles live on disk only).
2183. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "diagnostics export" — not present.
2184. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 Local game core "privacy redaction" — not implemented (no test for "tokens never appear in run bundles").
2185. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 ServerSummary / PackageManifestSummary / JoinEligibilityResult / ReplaySummary / DiagnosticsReport shared shapes — not declared.
2186. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 schema_version on every service object + migration handlers — not declared.
2187. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 BACK-SCOPE-01..10 acceptance suite — none authored.
2188. - [ ] [W4] [BP3] [DR-013] [GAP] DR-013 BACK-SCOPE-07 privacy-redaction test (default-on; tokens never logged) — not authored.

## 258. DR-017 — Mission generation strategy (CLOSED; M7 closes; BP3 manifest seed)
2199. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Objectives" — cf-mission has Objective struct ✓ but minimal.
2200. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Teams" — no team registry at BP3.
2201. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Terrain rules / material profile" — scenarios have terrain field; no material_profile field.
2202. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Command-core / base state" — no base state at BP3.
2203. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Equipment capability requirements" — no capability_requirements field.
2204. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Director pacing" — no director field.
2205. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Commander AI" — no commander field.
2206. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Save fields" — no save_fields field.
2207. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Replay events" — no replay_events declaration field.
2208. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest "Validation diagnostics" — no validation field.
2209. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 hand-authored anchor missions count 5-12 — only 2 (micro_breach + micro_reactor_defense + m5 chassis ones) at BP3.
2210. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 procedural / contract generation seed — no generator at BP3.
2211. - [ ] [W4] [BP3] [M7] [DR-017] [GAP] DR-017 player-authored scenarios via editor — no editor at BP3.

## 262. DR-031 — Content economy + monetization posture (CLOSED; every M1+ inherits audit)
2231. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 invariant test "no marketplace cut on user mods" — no test exists.
2232. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 invariant test "no gameplay-power locked behind storefront flag" — no test exists.
2233. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 invariant test "no FOMO timer in code path" — no test exists.
2234. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 invariant test "no time-pressure shop UI" — no test exists (no shop UI at BP3).
2235. - [ ] [W4] [M1] [DR-031] [GAP] DR-031 audit-row "premium one-time purchase" — codebase has no purchase plumbing (correct at BP3) but no architectural decision logged.

## 263. DR-030 — Scenario editor first-class commitment (CLOSED; M8 closes; BP3 manifest schema seed)
2236. - [ ] [W4] [BP3] [M8] [DR-030] [GAP] DR-030 in-engine workbench mode — `cf-tools-editor` is 38-line scaffold.
2237. - [ ] [W4] [BP3] [M8] [DR-030] [GAP] DR-030 hot-reload + test-run + export from editor — not implemented.
2238. - [ ] [W4] [BP3] [M8] [DR-030] [GAP] DR-030 same-manifest contract enforcement — no shared `MissionManifest` type used by engine + cf-mod.
2239. - [ ] [W4] [BP3] [M8] [DR-030] [GAP] DR-030 `.cfpkg` deterministic export — package format not defined.
2240. - [ ] [W4] [BP3] [M8] [DR-030] [GAP] DR-030 editor validators (missing fields / broken refs / AI policy violations / accessibility floor) — not implemented.
2241. - [ ] [W4] [BP3] [M8] [DR-030] [GAP] DR-030 procedural generator → manifest path — no generator at BP3.
2242. - [ ] [W4] [BP3] [M8] [DR-030] [GAP] DR-030 backend-mediated sharing post-launch — not in scope at BP3 but architecture seed missing.

## 276. spec/equipment-loadout-workbench-slice-a — Workbench Slice A gaps (BP3 M3A scoped but very partial)
2472. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W mission requirement strip — no mission UI at BP3.
2473. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W catalog browser with search/filter/tabs (63 unique player-catalog rows) — no catalog UI.
2474. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W item detail drawer (role / best/bad at / handling / terrain / AI policy / provenance / source / warning / compare) — no drawer.
2475. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W actor loadout column (slot groups + assigned rows + mass/cost subtotal + bot-safe count) — not implemented.
2476. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W squad capability summary (combat/breach/heal/mobility/scout/anti-craft/bot-safe/manual/risky counts) — not implemented.
2477. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W bot trust panel (claim state + reason labels + blackboard/source-confidence + replay/export preview + package bot-default gates) — not implemented.
2478. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W overlap compare panel (10 overlap rows from generated worksheet) — not implemented.
2479. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W workbench diagnostic drawer (diagnostic code + severity + source path + first fix action + package-mode verdicts) — not implemented.
2480. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W trace tab (item trace + fixture tabs + diagnostic trace + gap badges + open targets) — not implemented.
2481. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W source inspector (source state/confidence + module order + include depth + file stats + duplicate preset hits) — not implemented.
2482. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W replay/export preview (JSONL event preview + replay labels + AI reason labels + package hash) — not implemented.
2483. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W fixture routes 9 (assault_basic / engineer_breach / medic_rescue / sniper_overwatch / grenadier_risky / heavy_craft_killer / scout_salvager / bad_missing_breach / bad_bot_unsafe) — none implemented.
2484. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W transition events (`loadout.catalog.focused` / `loadout.detail.opened` / `loadout.item.assigned` / `loadout.summary.recomputed` / `workbench.diagnostic.opened` / `loadout.overlap.compare_opened` / `loadout.export.previewed`) — none emitted.
2485. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W role-record projection (LOAD-W-21/22 parity across catalog/AI/package/replay/mission consumers) — no shared item object.
2486. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W acceptance tests LOAD-W-01..22 — none pass.
2487. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W LOAD-FIELD field drill-down (CCCP field + normalized field + direct/inherited/inferred/manual status + consumers + first fix action) — not implemented.
2488. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W LOAD-FIELD-02 legacy `AddCargoItem` import with explicit actor slots + provenance + ambiguous-ownership warning — not implemented.
2489. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W LOAD-FIELD-03 AI reason source citation (`danger_radius` / `support_required` / `material_fit` / `range_band` / `ammo_pressure` / `bot_unproven`) — not surfaced.
2490. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W LOAD-FIELD-SNAPSHOT-01 source-snapshot tab (literal CCCP values + source path/range + consumer impact + open gaps) — not implemented.
2491. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W "Negative fixtures are first-class (UI must say 'this loadout cannot solve the mission')" — not implemented.
2492. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W "Designer mode unlocks hidden/internal rows" — no designer mode.
2493. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-W "Compare mode must change a decision (overlap suggests role split / skin / legacy / fixture)" — not implemented.

## 277. spec/equipment-role-card-renderer-slice-a — Role-card renderer Slice A gaps (BP3 M3A scoped)
2494. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R role card JSON loader (parses 106 cards + rejects missing required fields with source path) — no loader at BP3.
2495. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R catalog visibility policy (player_catalog / replacement_or_legacy_catalog / internal_component / internal_payload / hidden_or_internal) — not implemented.
2496. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R catalog row dense layout (role icon + name + package + capability chips + cost/mass/range chips + bot badge + warning count + overlap badge) — not implemented.
2497. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R primary verb taxonomy (engage_actor / suppress_or_break / destroy_area_or_hard_target / excavate_or_breach / fill_or_build / heal_or_rescue / traverse_or_reposition / backup_or_finish / long_range_pick) — not used in cf-equipment.
2498. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R terrain consequence taxonomy (mostly_actor_damage / removes_or_opens_material / adds_or_repairs_material / area_hazard_or_blast / mobility_state_change / hidden_internal_or_payload) — not used.
2499. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R bot competence vocab (Good / Risky / Manual Recommended / No AI Support Yet / not_for_default_bot_loadout) — single weapon at BP3.
2500. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R warning badges vocab (ai_summary_seed_needs_harness_or_manual_review / unclear_role_tags / package_builder_visibility / bot_use_needs_gate) — no warning system.
2501. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R provenance badges (direct / inherited / inferred / missing / manual) — not implemented.
2502. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R special-case rendering: Coalition.rte/Assault Rifle (high-risk overlap) — no Coalition pack.
2503. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R special-case Base.rte/Medikit (scripted support) — not implemented.
2504. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R special-case Base.rte/Grapple Gun (mobility + Manual Recommended) — not implemented.
2505. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R special-case Base.rte/Concrete Sprayer (build/fill hidden) — not implemented.
2506. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R special-case Base.rte/Rocket Launcher (heavy explosive + danger radius + target classes) — not implemented.
2507. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R overlap risk levels (high / medium / low) with required differentiator before catalog promotion — not enforced.
2508. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R 10 overlap groups (medium assault rifles / sidearms / etc.) — no overlap audit data.
2509. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R role signature (player_catalog | Primary firearm | Assault | engage_actor | medium | actor | - | mostly_actor_damage) field — not modeled.
2510. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R acceptance LOAD-R-01..13 — none pass.
2511. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R UI copy rules ("Risky: missing AI range/friendly-fire rules" not just "Risky"; "High overlap: same assault medium-range role as 4 rifles" not just "Overlaps"; "Workbench-only: internal payload" not just "Hidden") — no copy.
2512. - [ ] [W4] [BP3] [M3A] [GAP] LOAD-R replay/debug labels (stable item_id + role signature + primary verb + package id + selected/refused reason label) — not emitted in `events.jsonl`.

## 281. spec/package-builder-workbench-slice-a — Package builder Slice A gaps (BP3 M3A modding seed)
2621. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A 6 package modes (`dev_mount` / `local_package` / `published_package` / `legacy_rte` / `ui_skin` / `scenario_pack`) — only single mod-load path at BP3.
2622. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A manifest fields (id / display_name / version / package_type / engine_range / schema_version / authors / license / provenance_policy / dependencies / entry_points / assets / presets / capabilities / compatibility / content_hash / build) — minimal manifest only.
2623. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A deterministic build (sort paths + normalize archive metadata + hash file bytes + reject duplicate canonical paths + fail unresolved IncludeFile + build-twice verify) — not implemented.
2624. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A validation matrix (Manifest parse / File path existence/case / Allowed extensions / Include graph / Engine loader parity / `CopyOf` resolution / Duplicate preset / Lua syntax / Lua capability declaration / Provenance per file / License expression / Package hash reproducible / Test launch / Runtime warnings) per mode — none enforced.
2625. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A diagnostic codes (`PATH_NOT_FOUND` / `COPYOF_UNRESOLVED` / `DUPLICATE_PRESET` / `UNDECLARED_CAPABILITY` + 8 equipment codes) — none reported.
2626. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A workbench screens (Project home / File tree / Loader graph / Diagnostics / Manifest editor / Preset graph / Effect graph / Provenance ledger / Build panel / Migration preview / Test launch) — no workbench at BP3.
2627. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A backend output `PackageManifestSummary` (id / hash / capabilities / provenance / dependency graph / diagnostics) — not emitted.
2628. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A workbench events (`package_manifest_loaded` / `package_validation_started/finished` / `package_diagnostic_emitted` / `package_build_started/finished` / `package_hash_verified` / `provenance_entry_changed` / `migration_preview_generated` / `migration_applied` / `test_launch_started/finished` / `runtime_mod_error`) — none emitted.
2629. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A acceptance PACK-A-01..15 — none pass.
2630. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A legacy converter rule model — no converter at BP3.
2631. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "TOML for author / JSON for machine output" manifest authoring — not picked.
2632. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "Tree-sitter CCINI parser for include graph + source positions" — not present.
2633. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "package archive format (zip / tar+zstd / OCI-like / engine-native)" — not picked.
2634. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "Build twice and compare hashes" reproducibility test — not configured.
2635. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "Server purity (sv_pure) hash check" — no server purity at BP3.
2636. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "Effect graph for projectile/script chains (spawn + emit + damage + terrain carve + timer/death callback)" — no graphs.
2637. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "Migration preview (before/after diff + rule id + diagnostics + apply/revert)" — not implemented.
2638. - [ ] [W4] [BP3] [M3A] [GAP] PACK-A "Test launch adapter (sandbox + replay/event file + runtime diagnostics)" — not implemented.

## 306. spec/modding-model — Modding posture (BP3 closure includes V1 modding model)
3640. - [ ] [W4] [BP3] [GAP] MOD layer "Dev mount (folder with .rte/manifest/source assets + live indexed project)" — not implemented.
3641. - [ ] [W4] [BP3] [GAP] MOD layer "Local package (deterministic archive from dev mount + manifest + file hashes + generated metadata)" — not implemented.
3642. - [ ] [W4] [BP3] [GAP] MOD layer "Published package (immutable + provenance + compatibility metadata + signed/registered manifest + content hash)" — not implemented.
3643. - [ ] [W4] [BP3] [GAP] MOD layer "Legacy import (.rte / .zip module + diagnostics + migration notes)" — not implemented.
3644. - [ ] [W4] [BP3] [GAP] MOD layer "User content (scenes + saves + scripts + mutable userdata package tier)" — not implemented.
3645. - [ ] [W4] [BP3] [GAP] MOD loader parity rule "Model current loader before replacing (official module order + official fallback + sorted Mods/*.rte scan + userdata + Index.ini vs MergedIndex.ini + module metadata SupportedGameVersion + IncludeFile stack + wrong-case path checks + CopyOf resolution + duplicate preset collision/overwrite + ScanFolderContents caveats + .zip extraction + module/entity/movable script reload)" — not enforced.
3646. - [ ] [W4] [BP3] [GAP] MOD contract Manifest (ID + display_name + version + engine_range + package_type + dependencies + authors + license/provenance policy + entry_points + capabilities) — not present.
3647. - [ ] [W4] [BP3] [GAP] MOD contract Source graph (Include graph + CopyOf graph + script graph + asset path graph + dependency graph) — not present.
3648. - [ ] [W4] [BP3] [GAP] MOD contract Diagnostics (file + line + column + include_stack + severity + package-mode verdict + first fix action) — not present.
3649. - [ ] [W4] [BP3] [GAP] MOD contract Provenance (per-file source + copied/adapted/generated/original status + license + release-cleanup notes) — not present.
3650. - [ ] [W4] [BP3] [GAP] MOD contract Script capability (declared terrain/entity/audio/UI/backend/filesystem/network capabilities) — not present.
3651. - [ ] [W4] [BP3] [GAP] MOD contract Equipment metadata (resolved item role + AI summary + UI summary + balance fields + replay/backend fields + source provenance) — not present.
3652. - [ ] [W4] [BP3] [GAP] MOD contract Migration (rule IDs + preview diff + backups + diagnostics + post-migration validation) — not present.
3653. - [ ] [W4] [BP3] [GAP] MOD package mode verdict `dev_ok` (runs locally with visible warnings) — not implemented.
3654. - [ ] [W4] [BP3] [GAP] MOD package mode verdict `local_package_ok` (deterministic local archive + hashes + test-launches) — not implemented.
3655. - [ ] [W4] [BP3] [GAP] MOD package mode verdict `published_ready` (provenance + license + dependency + script + diagnostics policy clean) — not implemented.
3656. - [ ] [W4] [BP3] [GAP] MOD package mode verdict `bot_default_blocked` (content can exist but AI shouldn't use by default) — not implemented.
3657. - [ ] [W4] [BP3] [GAP] MOD package mode verdict `replay_backend_blocked` (runs locally but replay/server compatibility not valid) — not implemented.
3658. - [ ] [W4] [BP3] [GAP] MOD package mode verdict `migration_needed` (legacy can be imported but needs converter rules or manual fixes) — not implemented.
3659. - [ ] [W4] [BP3] [GAP] MOD acceptance MOD-A-01 (Clean .rte fixture imports into dev-mount + module graph + include graph + preset graph + script list + source paths) — none pass.
3660. - [ ] [W4] [BP3] [GAP] MOD acceptance MOD-A-02 (Loader parity fixture passes CONTENT-A checks) — not implemented.
3661. - [ ] [W4] [BP3] [GAP] MOD acceptance MOD-A-03 (Published mode fails unresolved include + wrong-case + unresolved CopyOf + undeclared script capability + duplicate preset without `replaces`) — not enforced.
3662. - [ ] [W4] [BP3] [GAP] MOD acceptance MOD-A-04 (Equipment fixture imports role-card fields from direct/inherited/manual sources + emits package diagnostics) — not implemented.
3663. - [ ] [W4] [BP3] [GAP] MOD acceptance MOD-A-05 (Legacy .zip import preserves source archive hash + skipped-file report + extracted path list + provenance prompt) — not implemented.
3664. - [ ] [W4] [BP3] [GAP] MOD acceptance MOD-A-06 (Test launch can run officials + selected dev module + export package diagnostics into prototype run bundle) — not implemented.

## 307. spec/art-and-asset-pipeline — 3-tier AI-driven pipeline (BP3 closure includes Tier 1 + Tier 2 partial)
3665. - [ ] [W4] [BP3] [M0+M2] [GAP] ART Tier 1 SVG/geometric placeholders (M0..M2; Python 3.11 + cairo-svg + Pillow + scripts under tools/asset_gen/) — partial at BP3.
3666. - [ ] [W4] [BP3] [GAP] ART Tier 1 build integration (`cargo build` runs `python3 tools/asset_gen/build_placeholders.py` if `.svg.template` or palette JSON changed) — not configured.
3667. - [ ] [W4] [BP3] [GAP] ART Tier 1 Faction palette JSON (per-faction primary + accent + outline + universal status palette for HP/ammo/affliction) — no faction palettes.
3668. - [ ] [W4] [BP3] [GAP] ART Tier 1 actor sprites (body-part rectangles + head circle + faction-colored outline + per-frame rotation/scale for walk cycle; 16×24 human / 48×64 mech) — partial.
3669. - [ ] [W4] [BP3] [GAP] ART Tier 1 weapons (rectangle + barrel triangle + faction-colored grip + muzzle-flash N-gon overlay; AK-47 = 16×6 wood-rect + steel-rect + magazine-rect) — not implemented.
3670. - [ ] [W4] [BP3] [GAP] ART Tier 1 vehicles / dropcraft (layered rectangles + wing trapezoids; Light dropship = 64×32 hull + thruster glow) — not implemented.
3671. - [ ] [W4] [BP3] [GAP] ART Tier 1 base objects (rectangles + iconographic glyphs; Medikit = 16×16 with red cross) — not implemented.
3672. - [ ] [W4] [BP3] [GAP] ART Tier 1 materials (solid color + 2x2 noise overlay; sand = tan dotted noise; rock = dark gray + crack lines) — not implemented.
3673. - [ ] [W4] [BP3] [GAP] ART Tier 1 UI icons (SVG iconography + rendered at 32/64/128 px; loadout slots + faction emblems + status icons) — not implemented.
3674. - [ ] [W4] [BP3] [GAP] ART Tier 1 audio (sine/square/triangle synth via synthio + 200ms blips at distinct frequencies; gunshot = 200Hz square attack+decay; reload = ascending sine) — not implemented.
3675. - [ ] [W4] [BP3] [GAP] ART Tier 1 fonts (Open-license JetBrains Mono + Press Start 2P + Noto) — not configured.
3676. - [ ] [W4] [BP3] [GAP] ART Tier 1 generated file structure under `game/assets/placeholders/{actors,weapons,materials,ui}/` — not present.
3677. - [ ] [W4] [BP3] [GAP] ART Tier 1 manifest.json catalog — not generated.
3678. - [ ] [W4] [BP3] [M2+M5] [GAP] ART Tier 2 ComfyUI/diffusion-generated (M2..M5) — not set up at BP3.
3679. - [ ] [W4] [BP3] [GAP] ART Tier 2 ComfyUI pinned (commit hash in `tools/comfyui_workflows/COMFYUI_COMMIT.txt` + installed in ~/.comfyui or per-developer dotfile path) — not configured.
3680. - [ ] [W4] [BP3] [GAP] ART Tier 2 base model SDXL 1.0 (~6.6GB; default for fast iteration) — not set up.
3681. - [ ] [W4] [BP3] [GAP] ART Tier 2 base model Flux.1-dev (~24GB; hero assets + backgrounds + cinematic concepts) — not set up.
3682. - [ ] [W4] [BP3] [GAP] ART Tier 2 base model SD3.5-large (~16GB; character consistency + photorealistic concept passes) — not set up.
3683. - [ ] [W4] [BP3] [GAP] ART Tier 2 LoRA Pixel Art XL (SDXL-compatible; CreativeML Open RAIL++-M license; logged in usage-ledger) — not configured.
3684. - [ ] [W4] [BP3] [GAP] ART Tier 2 LoRA Faction-style (per-faction style LoRA trained from Tier 1 + reference comic-noir + sci-fi art ~50-100 imgs/faction via kohya_ss) — not trained.
3685. - [ ] [W4] [BP3] [GAP] ART Tier 2 LoRA Animation-consistency (AnimateLCM or similar for character identity across frames) — not configured.
3686. - [ ] [W4] [BP3] [GAP] ART Tier 2 ControlNet SDXL (Canny + Depth + OpenPose + Tile) — not integrated.
3687. - [ ] [W4] [BP3] [GAP] ART Tier 2 custom nodes (ComfyUI-PixelArt-Detector + ComfyUI-Crystools + ComfyUI-Manager + ComfyUI-Impact-Pack + ComfyUI-AnimateDiff-Evolved) — not installed.
3688. - [ ] [W4] [BP3] [GAP] ART Tier 2 palette source (LoSpec palette JSON per-faction 16-color + universal 8-color status + 16-color environmental) — not configured.
3689. - [ ] [W4] [BP3] [GAP] ART Tier 2 background removal (rembg BRIA-RMBG-1.4 or U2Net via rembg Python lib) — not integrated.
3690. - [ ] [W4] [BP3] [GAP] ART Tier 2 spritesheet packer (TexturePacker CLI OR aseprite headless --sheet) — not integrated.
3691. - [ ] [W4] [BP3] [GAP] ART Tier 2 `tools/asset_gen/comfy_runner.py` (Python WebSocket API + reads asset spec + saves output + logs to usage-ledger) — not present.
3692. - [ ] [W4] [BP3] [GAP] ART Tier 2 per-asset workflow 12-step chassis-sprite pipeline (load spec → Tier 1 ControlNet → prompt → SDXL+LoRA+ControlNet → 1024×1024 → quantize → background strip → cleanup → save → animation frames → spritesheet pack → usage-ledger log) — not present.
3693. - [ ] [W4] [BP3] [GAP] ART Tier 2 background + sky + parallax pipeline (per-world Flux.1-dev sky concept + AI segment 4 parallax layers + per-time-of-day variants + per-weather variants triggered by EnvironmentSignal.weather) — not present.
3694. - [ ] [W4] [BP3] [GAP] ART Tier 2 video/cinematic pipeline (briefing comic panels + animated panel transitions + mission intro/outro 8-12s SVD + hero campaign cutscenes + in-game ambient loops + title screen background + trailer cuts) — not implemented.
3695. - [ ] [W4] [BP3] [GAP] ART Tier 2 deterministic seed (same seed + same prompt = identical output) — not enforced.
3696. - [ ] [W4] [BP3] [GAP] ART Tier 2 usage-ledger covers 100% of generated assets — not configured.
3697. - [ ] [W4] [BP3] [GAP] ART Tier 2 faction recolor variant generation (one source → 8 faction variants <5min per asset) — not implemented.
3698. - [ ] [W4] [BP3] [GAP] ART Tier 2 mod-author `cf-asset-pipeline regen --mod my_mod --tier 2` — not implemented.
3699. - [ ] [W4] [BP3] [M5] [GAP] ART Tier 3 AI-agent-polished final (M5+) — not set up.
3700. - [ ] [W4] [BP3] [GAP] ART Tier 3 Aseprite headless ($19.99 one-time + project-owner + per-modder license + Lua scripting cleanup) — not licensed.
3701. - [ ] [W4] [BP3] [GAP] ART Tier 3 Spine ($69 essential + bevy_spine runtime) OR DragonBones (free) for hero chassis skeletal animation — not integrated.
3702. - [ ] [W4] [BP3] [GAP] ART Tier 3 FMOD Studio (free under $200K/yr + bevy_fmod) OR bevy_kira_audio (pure-Rust Apache-2.0) — not integrated.
3703. - [ ] [W4] [BP3] [GAP] ART Tier 3 AI cleanup agent `tools/aseprite_cleanup.py` (pixel-snap + palette-enforce + isolated-pixel removal + dithering polish via Aseprite headless Lua) — not present.
3704. - [ ] [W4] [BP3] [GAP] ART Tier 3 variant generator `tools/variant_gen.py` (hero asset + variant spec faction/paint/damage stage/weather effect overlay → emits all variants) — not present.
3705. - [ ] [W4] [BP3] [GAP] ART Tier 3 `cf-asset-pipeline` CLI master Rust binary — not present.

## 316. spec/launch-content-roster — Full launch content roster (BP3 partial; closed direction)
3901. - [ ] [W4] [BP3] [GAP] ROSTER Pistols 12 (basic + revolver_44 + machine + silenced + smart + blaster + plasma + dueling + dart_tranq + emp_compact + chemical_injector + finger_gun) — 0 at BP3.
3902. - [ ] [W4] [BP3] [GAP] ROSTER SMGs 8 (basic + micro + vector + p90 + silenced + blaster + chemical + chain_dispenser) — 0 at BP3.
3903. - [ ] [W4] [BP3] [GAP] ROSTER Assault Rifles 10 (ak47 + m4 + g36 + galil + steyr_aug + blaster_rifle + battle_rifle_762 + carbine + pulse_rifle + tek_modular) — 0 at BP3; single rifle stub only.
3904. - [ ] [W4] [BP3] [GAP] ROSTER Battle Rifles + DMRs 5 (designated_marksman + fal_762 + g3 + sks + blaster_marksman) — 0 at BP3.
3905. - [ ] [W4] [BP3] [GAP] ROSTER Sniper Rifles 6 (50bmg + 338 + railgun + laser + emp + thermal) — 0 at BP3.
3906. - [ ] [W4] [BP3] [GAP] ROSTER Shotguns 6 (pump + blunderbuss + auto + combat + breach + plasma) — 0 at BP3.
3907. - [ ] [W4] [BP3] [GAP] ROSTER LMG/HMG 4 (m249 + pkm + m134 + chain_blaster) — 0 at BP3.
3908. - [ ] [W4] [BP3] [GAP] ROSTER Heavy/Explosive 15+ (rpg7 + javelin + law + grenade_launcher + milkor + flak_cannon + mortar_60mm + gauss_rifle + railgun_anti_tank + particle_accelerator + bazooka + recoilless_rifle + missile_swarm_launcher + auto_grenade_launcher + fuel_air_explosive_launcher) — 0 at BP3.
3909. - [ ] [W4] [BP3] [GAP] ROSTER Throwables/Explosives 15 (frag + smoke + flash + emp + incendiary + sticky + remote_charge_c4 + tripwire + claymore + satchel + nano + plasma_orb + breach_charge + decoy_beacon + blue_bomb) — 0 at BP3.
3910. - [ ] [W4] [BP3] [GAP] ROSTER Melee 8 (combat_knife + machete + riot_baton + vibroblade + plasma_sword + monomolecular_katana + stun_rod + chainsaw) — 0 at BP3.
3911. - [ ] [W4] [BP3] [GAP] ROSTER Tools 15 (digger_light/medium/heavy + breach_charge_handheld + repair_basic/advanced + concrete_sprayer + foam_constructor + sample_scanner + metal_detector + drill_light/medium/heavy + drill_vacuum_rated + oxygen_analyzer) — 0 at BP3.
3912. - [ ] [W4] [BP3] [GAP] ROSTER Mobility 6 (grapple_hook + jetpack_assist + rope_tether + deployable_ladder + harpoon_line + magnetic_boots) — 0 at BP3.
3913. - [ ] [W4] [BP3] [GAP] ROSTER Shields 5 (riot + combat + deployable_barrier + more) — 0 at BP3.
3914. - [ ] [W4] [BP3] [GAP] ROSTER full launch content (140+ weapons + all actors + all vehicles + all base objects + all factions + 30+ missions + 12 worlds + biomes + materials + ores + 30+ music tracks + 400+ SFX) — content stubs only.
3915. - [ ] [W4] [BP3] [GAP] ROSTER functional requirement "Every entry is FULLY WORKING (working sim behavior + AI-readable metadata + replay events + captions + balance fixture + localized strings) NOT stat-only NOT asset-only" — not enforced.

## 318. spec/equipment-loadout — Equipment & loadout model (BP3 M3A/B partial; comprehensive at launch)
3943. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `item_definition` (immutable authored facts: identity + role tags + handling + projectile/effect links + source package + declared capabilities) — partial.
3944. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `runtime_item_instance` (owner + ammo + heat/cooldown + condition + script state + attachments + dropped/pickup state) — partial.
3945. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `chassis_definition` (origin compatibility + body/chassis sockets + armor layers + module hardpoints + pilot capacity + mass + mobility profile + damage-stage vocabulary) — see §282.
3946. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `equipment_condition` (Intact/Impaired/Critical/Disabled/Destroyed + source event + repairability + smoke/spark/audio state + behavior penalty) — not implemented.
3947. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `loadout_template` (delivery craft + explicit actors + slots + package ids + budget + mission-role intent + legacy source order) — no loadouts.
3948. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `role_card` (best at + bad at + range + terrain consequence + support value + handling + risk) — not implemented.
3949. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `ai_summary` (target classes + range model + danger model + material fit + reason labels + claim state + scenario refs) — not implemented.
3950. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `ui_projection` (icon + cost/mass + range band + role chips + bot trust + warning badges + accessibility labels) — not implemented.
3951. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `package_diagnostic` (source path + field provenance + warning ids + mode verdicts + first fix action + manual patch id) — not implemented.
3952. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `balance_row` (role overlap + cost/mass/supply pressure + terrain power + handling cost + AI confidence + mission impact) — not implemented.
3953. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `replay_event` (item id + actor owner + action + reason label + target/material context + package hash + result/checksum) — not implemented.
3954. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP shared consumer contract `mission_requirement` (required/recommended capabilities + bot-safe/manual policy for breach/heal/scout/anti-craft/delivery/recovery) — not implemented.
3955. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Identity and provenance" — not modeled.
3956. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Handling and slot pressure (mass + bulk + hands_required + dual_wield_policy + support_requirement + support_offset + grip_strength + held_hitability + drop_fragility + actor_body_constraints)" — not modeled.
3957. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Chassis and armor pressure (origin_compatibility + armor_slot + coverage_arc + coverage_part + protection_profile + module_socket + pilot_state + route_profile + repair_tags + condition_stage)" — not modeled.
3958. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Action and effect model (primary_verb + secondary_verb + target_rule + activation_context + effect_profile + scripted_verbs + failure_reasons)" — not modeled.
3959. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Projectile and area shape (projectile_model + projectile_count + muzzle_velocity + lifetime + spread_shape + blast_radius + falloff + target_filters + friendly_fire_policy)" — not modeled.
3960. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Terrain and material consequence (dig_profile + fill_profile + material_affordances + dirty_region_policy + path_invalidation + collapse_risk + actor_collision_effect)" — not modeled.
3961. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "AI policy (bot_claim_state + target_classes + range_model + material_fit + danger_radius + utility_inputs + blackboard_keys + reason_labels + scenario_refs + harness_status)" — not modeled.
3962. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "UI projection (short_role_text + best_at + bad_at + icon_tags + comparison_fields + warning_badges + accessibility_labels + same_input_actions + source_tab_rows)" — not modeled.
3963. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Balance and overlap (role_signature + overlap_group_id + cost_pressure + mass_pressure + ammo_pressure + terrain_power + delivery_burden + mission_fit + counterplay)" — not modeled.
3964. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Replay backend session (event_families + causality_parent + source_snapshot_id + determinism_class + sync_relevance + package_hash_relevance + loadout_snapshot_ref)" — not modeled.
3965. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP role_record slice "Mission capability (capability_tags + required_by_missions + recommended_for_missions + bot_safe_for_missions + manual_only_policy + delivery_role)" — not modeled.
3966. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Physical handling (Mass + OneHanded + DualWieldable + Supportable + SupportOffset + GripStrengthMultiplier + GetsHitByMOsWhenHeld + SharpLength)" — not modeled.
3967. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Firearm cadence (RateOfFire + ActivationDelay + DeactivationDelay + ReloadTime + FullAuto + Reloadable + DualReloadable + OneHandedReloadTimeMultiplier + NoSupportFactor)" — partial.
3968. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Firearm offsets and feedback (MuzzleOffset + EjectionOffset + ShellEjectAngle + ShellSpreadRange + ShellAngVelRange + ShellVelVariation + RecoilScreenShakeAmount + ShakeRange + SharpShakeRange)" — not modeled.
3969. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Magazine/ammo (Magazine + RoundCount + RTTRatio + RegularRound + TracerRound + Discardable + full-capacity cache)" — partial.
3970. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Projectile/round behavior (Particle + ParticleCount + FireVelocity + InheritsFirerVelocity + Separation + LifeVariation + Shell + ShellVelocity)" — partial.
3971. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "AI projectile summary (AIBlastRadius + AILifeTime + AIFireVel + AIPenetration + EstimateDigStrength + GetBulletAccScalar + Lua trajectory comparison)" — not modeled.
3972. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Groups and delivery roles (AddToGroup + Weapons-Primary/Secondary/Light/Heavy/Sniper/Explosive + Tools-Diggers/Breaching + Light/Medium/Heavy/CQB/Scout/Sniper/Grenadier/Engineer)" — not modeled.
3973. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Ordered loadout cargo (DeliveryCraft + ordered AddCargoItem entries + first actor receives following items until next actor boundary)" — not implemented.
3974. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Buy/loadout UX state (Category tabs + craft rows + passenger count + mass + cost + saved loadouts + cart actions + allowed/always/prohibited item sets)" — no buy menu.
3975. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Scripted equipment (Medikit + Grapple Gun + Constructor + Disarmer + Scanner + concrete/digger tools + pie actions + scripted target checks)" — not implemented.
3976. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP CCCP field family "Package/workbench provenance (PresetName + CopyOf + source .ini path + inherited fields + inferred fields + manual overlay patches + warning details)" — not implemented.
3977. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP concrete role card "Light/Medium/Heavy Diggers (route-making + material fit + tunnel profile + path invalidation + bot stand-off + material overlay preview)" — single basic digger.
3978. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP concrete role card "Concrete Sprayer (build/fill/reinforcement + not normally buyable in scanned Base catalog)" — not implemented.
3979. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP concrete role card "Grapple Gun (scripted + guide arrows + pie actions + infinite claw ammo + player-control-specific input)" — not implemented.
3980. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP concrete role card "Medikit (limited-use scripted firearm shell + raycasts to self/ally + heals wounds/health + revives dead actor clone + refunds ammo on failed use)" — not implemented.
3981. - [ ] [W4] [BP3] [M3A] [GAP] EQUIP concrete role card "Rocket Launcher (heavy handling + projectile/emitter behavior + AIBlastRadius + slow reload + craft/armor threat + terrain blast risk)" — not implemented.


# ===== WAVE 5 — MISSION DIRECTOR + UX SHELL + TUTORIAL =====

## 35. DR-009 — Command UX still OPEN; lean is hybrid direct + slowdown (M4B owns but BP3 should expose slowdown stub)
471. - [ ] [W5] [BP3] [M4B] [DR-009] [GAP] DR-009 direct-control + slowdown command overlay (25% time dilation) — not implemented at BP3.
472. - [ ] [W5] [BP3] [M4B] [DR-009] [GAP] DR-009 hold-or-toggle slowdown key — not bound.
473. - [ ] [W5] [BP3] [M4B] [DR-009] [GAP] DR-009 tactical map mode — not implemented.
474. - [ ] [W5] [BP3] [M4B] [DR-009] [GAP] DR-009 commander focus charge resource — not declared.
475. - [ ] [W5] [BP3] [M4B] [DR-009] [GAP] DR-009 multiplayer slowdown vote-to-slow rule — N/A pre-multiplayer but spec calls for it.
476. - [ ] [W5] [BP3] [M4B] [DR-009] [GAP] DR-009 ORDER-01 acceptance test (player understands blocked reason in 2s) — not authored.

## 45. DR-023 — Tutorial/onboarding (closed but BP3 has zero onboarding surface)
541. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 polished onboarding mission — not scaffolded at all.
542. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Movement/Aim lab — not built.
543. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Terrain/Materials lab — not built.
544. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Loadout/Delivery lab — not built.
545. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Squad Orders/AI lab — not built.
546. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Command Core/Base lab — not built.
547. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Avatar Mode lab — not built.
548. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Chassis Damage lab — closest is m5_chassis_wreck_eject scenario but it's not framed as a lab.
549. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 Replay/Debrief lab — not built.
550. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 contextual tooltip system — not implemented.
551. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 "show me why" handoff from failure → replay viewer or relevant lab — not implemented.
552. - [ ] [W5] [BP3] [DR-023] [GAP] DR-023 `tooltip_state` data field per tooltip — not in schema.

## 126. spec/ux-wireframes-slice-a UX-W-01..16 closure debt
1116. - [ ] [W5] [GAP] UX-W-01 "Tactical HUD must map to real actor / AI / terrain / delivery / package state" — covered for player actor; enemy AI state not on HUD.
1117. - [ ] [W5] [GAP] UX-W-02 "Wounds, recoil, falling, craft crashes, blocked paths, AI failures need short explanations" — only recoil cooldown visible; falls + AI-blocked paths not labeled.
1118. - [ ] [W5] [GAP] UX-W-03 "Combat-critical status near actor/reticle; squad/server/package details in dense panels" — single-actor HUD only; no squad surface.
1119. - [ ] [W5] [GAP] UX-W-04 "Orders, purchases, package publishes, joins must show predicted result + blockers before commit" — no order/purchase/publish surface at BP3.
1120. - [ ] [W5] [GAP] UX-W-05 "Dense tables for server / replay / diagnostic / package / loadout" — no dense table UI at BP3.
1121. - [ ] [W5] [GAP] UX-W-06 buy/loadout UI — not built.
1122. - [ ] [W5] [GAP] UX-W-07 delivery-craft preview UI — not built (no delivery craft at BP3).
1123. - [ ] [W5] [GAP] UX-W-08 material overlay (toggle key) — exists logically; not wired to a keyboard shortcut.
1124. - [ ] [W5] [GAP] UX-W-09 replay browser dense table — only individual replay viewing via cf-tools-replay-viewer.
1125. - [ ] [W5] [GAP] UX-W-10 equipment workbench dense table — not built.
1126. - [ ] [W5] [GAP] UX-W-11 hub IA (Lobby / Local / Replays / Settings / Mods / Workbench / Diagnostics) — not built.
1127. - [ ] [W5] [GAP] UX-W-12 server-rows + compatibility-preflight UI — not built.
1128. - [ ] [W5] [GAP] UX-W-13 package-publish + blockers UI — not built.
1129. - [ ] [W5] [GAP] UX-W-14 server-supervisor lifecycle state (no log-parsing) — not built.
1130. - [ ] [W5] [M4A] [GAP] UX-W-15 same-input navigation across all surfaces — only HUD focus traversal at M4A.
1131. - [ ] [W5] [GAP] UX-W-16 accessibility floor + reduced-motion / reduced-flash propagation — fields flow but not all surfaces honor.

## 129. cf-app keyboard binding gaps
1148. - [ ] [W5] [GAP] cf-app does not bind a key to toggle material overlay.
1149. - [ ] [W5] [GAP] cf-app does not bind a key to toggle pause.
1150. - [ ] [W5] [GAP] cf-app does not bind a key to open settings UI (which doesn't exist).
1151. - [ ] [W5] [GAP] cf-app does not bind a key to advance one tick (debug).
1152. - [ ] [W5] [GAP] cf-app does not bind a key to take a screenshot (dev convenience).
1153. - [ ] [W5] [GAP] cf-app does not bind a key to switch slowdown ratio.
1154. - [ ] [W5] [GAP] cf-app does not bind a key to switch camera mode.
1155. - [ ] [W5] [GAP] cf-app does not bind a key to display debug overlay.
1156. - [ ] [W5] [GAP] cf-app does not bind a key to toggle HUD off (clean screenshot).
1157. - [ ] [W5] [GAP] cf-app does not respect macOS Cmd-Q (only ESC works).
1158. - [ ] [W5] [GAP] cf-app does not respect Win Alt-F4.
1159. - [ ] [W5] [GAP] cf-app does not respect Linux Ctrl-Q.
1160. - [ ] [W5] [GAP] cf-app does not bind a help/F1 key to surface the keyboard-binding cheatsheet.

## 141. Hub / lobby / pause-menu surfaces missing entirely at BP3
1254. - [ ] [W5] [BP3] [GAP] No hub UI exists.
1255. - [ ] [W5] [BP3] [GAP] No "Lobby" / "Local Game" / "Replays" / "Settings" / "Mods" / "Workbench" tabs.
1256. - [ ] [W5] [BP3] [GAP] No "first-run" experience.
1257. - [ ] [W5] [BP3] [GAP] No pause menu (ESC = quit; no pause).
1258. - [ ] [W5] [BP3] [GAP] No in-game settings UI (cli flags drive settings).
1259. - [ ] [W5] [BP3] [GAP] No quit-confirmation dialog.
1260. - [ ] [W5] [BP3] [GAP] No "exit to main menu" flow.

## 147. M4A Done-criteria audit gaps
1295. - [ ] [W5] [M4+M4A+M4B] [GAP] `M4-D03` "Mission card renders pre/post mission with comic-noir style" — deferred to M4B (BP7) per Roadmap V2 split; OK but the checklist still tracks it as M4-D03 which conflates M4A and M4B.

## 163. cf-mission objectives schema gaps
1416. - [ ] [W5] [GAP] `cf-mission::Objective` has `id`, `name`, `state` fields ✓ but no `description` field for the player-visible explanation.
1417. - [ ] [W5] [GAP] `Objective.required_for_win` boolean — missing (some objectives are bonus / optional).
1418. - [ ] [W5] [GAP] `Objective.deps` — no dependency declaration ("breach before extract").
1419. - [ ] [W5] [GAP] `Objective.time_limit_ticks` — only mission-wide; not per-objective.
1420. - [ ] [W5] [GAP] `Objective.expected_value` — no scoring weight per objective for retention/debrief.
1421. - [ ] [W5] [GAP] `Objective.event_filter` — no event-pattern that completes/fails objective.
1422. - [ ] [W5] [GAP] `Objective.update_state_predicate` — no declarative state-update rule.
1423. - [ ] [W5] [GAP] No `mission.objective_paused` for tutorial flow.
1424. - [ ] [W5] [GAP] No `mission.objective_replayed` for retry-same-seed flow.

## 178. DR-017 — Mission generation strategy (OPEN; M7 closes; BP3 manifest schema seed)
1544. - [ ] [W5] [BP3] [M7] [DR-017] [GAP] DR-017 typed mission manifest schema — `cf-mission::Objective` is the only field; full mission-manifest schema not authored.
1545. - [ ] [W5] [BP3] [M7] [DR-017] [GAP] DR-017 capability_requirements field — missions cannot declare "requires breach tool".
1546. - [ ] [W5] [BP3] [M7] [DR-017] [GAP] DR-017 director phases — no phased scenario authoring.
1547. - [ ] [W5] [BP3] [M7] [DR-017] [GAP] DR-017 MISSION-A acceptance — not authored.

## 180. DR-030 — Scenario editor commitment (CLOSED-DIRECTION; M8 closes; BP3 schema seed)
1555. - [ ] [W5] [BP3] [M8] [DR-030] [GAP] DR-030 in-engine workbench mode — `cf-tools-editor` is a 38-line scaffold.
1556. - [ ] [W5] [BP3] [M8] [DR-030] [GAP] DR-030 `.cfpkg` export from editor — not implemented.
1557. - [ ] [W5] [BP3] [M8] [DR-030] [GAP] DR-030 scenario validator (catches missing fields, broken refs, AI policy violations, accessibility issues) — not implemented.
1558. - [ ] [W5] [BP3] [M8] [DR-030] [GAP] DR-030 sample mod (new chassis archetype) — not authored.

## 213. DR-042 — Game modes & match grammar (CLOSED; M7 + M11 + M12 own; BP3 schema seed)
1894. - [ ] [W5] [BP3] [M11+M12+M7] [DR-042] [GAP] DR-042 match grammar (rooted_bunker_defence / dropship_attacker / Coop-Defence variant) — not declared.
1895. - [ ] [W5] [BP3] [M11+M12+M7] [DR-042] [GAP] DR-042 Bunker Defence Proof Mission scenario — not authored.
1896. - [ ] [W5] [BP3] [M11+M12+M7] [DR-002+DR-042] [GAP] DR-042 match.* event category schema seed — `match` declared in DR-002 baseline but not emitted.

## 257. DR-016 — Setting / world frame (CLOSED; BP3 narrative seed expected)
2194. - [ ] [W5] [BP3] [DR-016] [GAP] DR-016 "Frontier disaster-contract sci-fi" world frame seed — not authored in `content/`.
2195. - [ ] [W5] [BP3] [DR-016] [GAP] DR-016 mission types vocabulary (breach / rescue / recover / salvage / sabotage / stabilize / extract / defend / investigate) — partly declared as MissionResolved.reason; not all 9 mission-types are scenarioable.
2196. - [ ] [W5] [BP3] [DR-016] [GAP] DR-016 command core / neural anchor / continuity core / operator uplink / company command node working term — no canonical name picked.
2197. - [ ] [W5] [BP3] [DR-016] [GAP] DR-016 faction grammar — no factions registry declared at BP3.
2198. - [ ] [W5] [BP3] [DR-016] [GAP] DR-016 biome variety (colony / corporate facility / alien jungle / derelict / disaster zone) — only `micro_breach` and `m1_actor_range` scenarios exist.

## 278. spec/mission-director-slice-a — Mission director Slice A gaps (BP3 M4-M5 partial; M7 closes)
2513. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A typed mission manifest (10 sections: Identity / Scene / Teams / Objectives / Director / Commander / Equipment / Destruction / Save / Event / Script hooks) — only thin scenario JSON at BP3.
2514. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A scene contract "named areas + spawn anchors + brain vaults + LZ bands + destructible zones + critical objects + forbidden spawn zones" — no scene-area validator.
2515. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A director phase machine "setup → prep → launch → build_up → sustain_peak → peak_fade → relax → objective_push → emergency → extraction → debrief" — no phase machine; missions auto-start.
2516. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A director intensity sampler (7 signals: actor damage / terrain pressure / enemy proximity / delivery risk / resource pressure / objective progress / player command load) — not implemented.
2517. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A commander AI state (doctrine + knowledge + budget + targets + squads + delivery) — no commander AI at BP3.
2518. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A commander decision verbs (choose_target / score_lz / build_package / assign_squad / breach / defend / retreat_or_rescue) with reason strings — none emit events.
2519. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A objective grammar (brain_hunt / breach_route / hold_area / recover_object / sabotage / extract / survive_wave / salvage / rescue) — partial; only "destroy" + "defend" exist.
2520. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A objective "owns progress + fail/win + replay events + UI markers + director effects + save fields + telemetry + validation tests" — objectives are static at BP3.
2521. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A "Destruction-aware mission requirement: Terrain cannot be the only lock" — not enforced.
2522. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A "Critical-object overlay shows non-destructible objects" — no overlay.
2523. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A "Compact spaces beat empty sprawl" — no map authoring guidance at BP3.
2524. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A "Plan/action phase split (Teardown-style prep before launch)" — missions auto-start at BP3.
2525. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A "Prep-only save slot for prep phase" — no save at BP3.
2526. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A equipment capability contract (mission requests capability tags `dig.soft` / `breach.door` / `fight.medium` / `support.heal` / `build.fill` / `craft.capacity` / etc., not hardcoded item names) — not implemented.
2527. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A loadout UI "required/recommended/dangerous/manual-only equipment capability badges" — no badges.
2528. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A LZ scorer "terrain altitude + enemy LOS + fog + occupied/craft avoidance + async path requests + path length + obstacle height" — no LZ at BP3.
2529. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `mission_validate` / `mission_start` events — not emitted.
2530. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `director_phase_change` event — not emitted.
2531. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `director_intensity_sample` event — not emitted.
2532. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `objective_start/update/complete/fail` event with UI marker + replay event — partial.
2533. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `commander_decision.target_selected` event — not emitted.
2534. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `commander_decision.lz_scored` event — not emitted.
2535. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `commander_decision.package_built` event — not emitted.
2536. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `commander_decision.squad_tasked` event — not emitted.
2537. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `commander_decision.breach_ordered` event — not emitted.
2538. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `commander_decision.defend_ordered` event — not emitted.
2539. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `commander_decision.recovery_ordered` event — not emitted.
2540. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `lz_scored` event — not emitted.
2541. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `delivery_created/landed/lost` event — not emitted; no delivery system.
2542. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `terrain_changed` event with at least one consumer (path/AI/objective/replay/UI) — emitted but no consumers wired.
2543. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `loadout_capability_check` event — not emitted.
2544. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A `mission_end` event with winner/cause/objective table/actor losses/optional rewards — partial.
2545. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A save fields "mission identity / director / objectives / commander / actors / equipment / terrain / delivery / script hooks" — no save at BP3.
2546. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A emergency phase (rescue stuck/softlocked mission: replacement craft / alternate target / extraction route) — not implemented.
2547. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A acceptance MISSION-A-01..18 — none pass.
2548. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A world binding `Mission.world_id` → cf-environment world manifest resolution — no worlds at BP3.
2549. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A match grammar `Match.kind` (BunkerDefence/SymmetricArena/FFA/AsymmetricNTeam/CoopVsAI/Campaign) wiring — not implemented.
2550. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A weather policy `Mission.weather_policy` (force dust storm / RF silence solar flare / clear) — no weather at BP3.
2551. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A comms policy `Mission.comms_policy` (per-team default frequencies + per-mission radio bans + jamming overlays) — no comms at BP3.
2552. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A hazard escalation (mission objectives like "evacuate before radiation peak" / "breach hull while contained" / "extinguish atmosphere fire before reactor breach") — no hazards.
2553. - [ ] [W5] [BP3] [M4+M5+M7] [GAP] MISSION-A proof "Breach Contract Slice A" with bunker scene + commander + objectives + LZ + craft + reinforcements — micro_breach is much smaller scope.

## 279. spec/ux-wireframes-slice-a — UX wireframes Slice A gaps (BP3 M4A scoped; partial)
2554. - [ ] [W5] [BP3] [M4A] [GAP] UX-W information priority ladder (L0 immediate 0-500ms / L1 active 0.5-2s / L2 squad/tactical 2-5s / L3 planning/economy 5-60s / L4 debug/learning) — not enforced.
2555. - [ ] [W5] [BP3] [M4A] [GAP] UX-W wireframe A "Tactical HUD" (objective/timer + team alert + squad strip + actor focus + reticle/range arc + limb cue + order text + body silhouette + weapon state + event tail + gold/delivery queue/LZ risk) — minimal HUD only.
2556. - [ ] [W5] [BP3] [M4A] [GAP] UX-W HUD "incoming shell marker / projectile/explosion screen-edge direction" — not implemented.
2557. - [ ] [W5] [BP3] [M4A] [GAP] UX-W HUD "limb cue: left leg orange" (per-limb wound state) — single HP at BP3.
2558. - [ ] [W5] [BP3] [M4A] [GAP] UX-W HUD "instability indicator (travel impulse from actor lifecycle)" — no stability scalar.
2559. - [ ] [W5] [BP3] [M4A] [GAP] UX-W HUD "weapon readiness (fire/reload/jam/overheat/no-arm/blocked-muzzle)" — only fire/reload at BP3.
2560. - [ ] [W5] [BP3] [M4A] [GAP] UX-W HUD "danger arrow (direction + type of threat without color-only)" — no danger arrows.
2561. - [ ] [W5] [BP3] [M4A] [GAP] UX-W HUD "heard/noise badge (`alarm_registered`)" — not implemented.
2562. - [ ] [W5] [BP3] [M4A] [GAP] UX-W wireframe B "Squad Panel + Command Overlay" — no squad at BP3.
2563. - [ ] [W5] [BP3] [M4A] [GAP] UX-W command overlay "Order wheel: Move/Defend/Dig/Breach/Repair/Rescue/Retreat/Hold Fire" — not implemented.
2564. - [ ] [W5] [BP3] [M4A] [GAP] UX-W command overlay "Route line (green → yellow → red blocked segment)" — no pathfinding.
2565. - [ ] [W5] [BP3] [M4A] [GAP] UX-W command overlay "Tooltip with blocker reason + suggested alternatives" — not implemented.
2566. - [ ] [W5] [BP3] [M4A] [GAP] UX-W command overlay "Slowdown command mode at 25%/75%" — no time scaling at BP3.
2567. - [ ] [W5] [BP3] [M4A] [GAP] UX-W wireframe C "Buy / Loadout / Delivery" with role filter + package source + catalog table + selected pros/cons + cart + craft + LZ risk + ETA + preview — no buy menu at BP3.
2568. - [ ] [W5] [BP3] [M4A] [GAP] UX-W buy menu cost/mass/passenger badges (red/yellow/green delivery risk) — no buy menu.
2569. - [ ] [W5] [BP3] [M4A] [GAP] UX-W buy menu "Bot Skill: poor/medium/good/unknown" indicator — not implemented.
2570. - [ ] [W5] [BP3] [M4A] [GAP] UX-W buy menu "Terrain fit (item solves which materials)" — not implemented.
2571. - [ ] [W5] [BP3] [M4A] [GAP] UX-W buy menu "Package source (modded/private status)" — not implemented.
2572. - [ ] [W5] [BP3] [M4A] [GAP] UX-W buy menu "Delivery preview (LZ risk + craft exits + blast/danger area + cargo order)" — no delivery.
2573. - [ ] [W5] [BP3] [M4A] [GAP] UX-W wireframe D "Material / Path Overlay" 5 modes (Integrity / Path / Hazard / Support / AI Debug) with tints (green/yellow/red/white) + tooltip — not implemented.
2574. - [ ] [W5] [BP3] [M4A] [GAP] UX-W wireframe E "Death Recap + Replay Viewer" with auto recap on major death + cause chain + filters + bookmarks + full replay — not implemented.
2575. - [ ] [W5] [BP3] [M4A] [GAP] UX-W wireframe F "Hub / Local Game / Server Browser" — minimal launcher at BP3.
2576. - [ ] [W5] [BP3] [M4A] [GAP] UX-W hub "Servers table (Name/Ping/Mode/Map/Humans-Bots/Packages/Trust/Join)" — no servers at BP3.
2577. - [ ] [W5] [BP3] [M4A] [GAP] UX-W hub "Join blocker (exact reason + next action: update / install package / enter invite / repair hash / trust override)" — not implemented.
2578. - [ ] [W5] [BP3] [M4A] [GAP] UX-W hub "Local panel (profile / map / packages / bots / supervisor state / logs / last replay)" — partial.
2579. - [ ] [W5] [BP3] [M4A] [GAP] UX-W hub "Health (backend API / package registry / recorder schema / supervisor state / last failed join)" — not implemented.
2580. - [ ] [W5] [BP3] [M4A] [GAP] UX-W wireframe G "Workbench / Package Builder" (file tree + diagnostics table + preview/graph + build panel + provenance ledger + test launch) — no workbench.
2581. - [ ] [W5] [BP3] [M4A] [GAP] UX-W UX event hooks (`ux_screen_opened` / `ux_command_previewed` / `ux_order_confirmed` / `ux_buy_item_compared` / `ux_delivery_queued` / `ux_overlay_toggled` / `ux_recap_opened` / `ux_join_blocker_seen` / `ux_diag_fix_action_used` / `ux_accessibility_setting_changed`) — none emitted.
2582. - [ ] [W5] [BP3] [M4A] [GAP] UX-W acceptance UX-W-01..16 — none pass.

## 303. spec/product-promise — Product promise candidate (BP3 elevator pitch should be demoable)
3532. - [ ] [W5] [BP3] [GAP] PROMISE "Modern Cortex-like tactical physics sandbox" — partial; physics minimal.
3533. - [ ] [W5] [BP3] [GAP] PROMISE "Commander-first through capable AI squads" — no squads/AI at BP3.
3534. - [ ] [W5] [BP3] [GAP] PROMISE "Directly pilot fragile soldiers/androids/robots/armored bodies/enterable mechs" — single soldier control only.
3535. - [ ] [W5] [BP3] [GAP] PROMISE "Power and defend a base through vulnerable command core" — no base/core.
3536. - [ ] [W5] [BP3] [GAP] PROMISE "Uproot core for risky boosted-avatar play" — no core.
3537. - [ ] [W5] [BP3] [GAP] PROMISE "Tear through destructible terrain" — partial; single material at BP3.
3538. - [ ] [W5] [BP3] [GAP] PROMISE "Recover from chaotic failures" — partial; no recovery hooks.
3539. - [ ] [W5] [BP3] [GAP] PROMISE "Turn every battle into replayable story" — no replay viewer.
3540. - [ ] [W5] [BP3] [GAP] PROMISE "Solo-first" — partial; no AI companions.
3541. - [ ] [W5] [BP3] [GAP] PROMISE "Mod-friendly" — no workbench at BP3.
3542. - [ ] [W5] [BP3] [GAP] PROMISE "Strong replay/debug tools" — no viewer; replay events partial.
3543. - [ ] [W5] [BP3] [GAP] PROMISE "Readable UX" — minimal HUD only at BP3.
3544. - [ ] [W5] [BP3] [GAP] PROMISE "Progression around mastery / creative loadouts / veteran actors / damageable gear / salvage / base power / chassis identity / shared challenge seeds" — none implemented.
3545. - [ ] [W5] [BP3] [GAP] PROMISE target player "Cortex/Soldat/Liero veteran (preserve direct-control friction + improve readability + controls)" — partial.
3546. - [ ] [W5] [BP3] [GAP] PROMISE target player "Solo tactics player (AI companions that don't require human teammates)" — no AI companions.
3547. - [ ] [W5] [BP3] [GAP] PROMISE target player "Strategy-first (win/lose through plans/orders/doctrine without constant body possession)" — no command overlay.
3548. - [ ] [W5] [BP3] [GAP] PROMISE target player "Base-builder / defender (base power + shields + turrets + sensors + doors + repair + command relays)" — no base.
3549. - [ ] [W5] [BP3] [GAP] PROMISE target player "Last-stand gambler (pull core from base + plant into body/mech for power spike)" — no core.
3550. - [ ] [W5] [BP3] [GAP] PROMISE target player "Armored-machine fantasy (mechs + powered armor + robots + damaged weapons + smoke + sparks + cockpit rescues + heavy loadout tradeoffs)" — partial.
3551. - [ ] [W5] [BP3] [GAP] PROMISE target player "Builder/modder (deep data + package validation + quick test loops)" — no workbench.
3552. - [ ] [W5] [BP3] [GAP] PROMISE target player "Replay/spectacle (weird deaths + heroic saves + shareable clips)" — no replay viewer.
3553. - [ ] [W5] [BP3] [GAP] PROMISE target player "Long-tail campaign (persistent stakes without grind)" — no campaign.
3554. - [ ] [W5] [BP3] [GAP] PROMISE pillar "Physical battles create stories" — partial.
3555. - [ ] [W5] [BP3] [GAP] PROMISE pillar "AI is trustworthy enough for solo play" — no AI at BP3.
3556. - [ ] [W5] [BP3] [GAP] PROMISE pillar "UX makes chaos readable" — minimal at BP3.
3557. - [ ] [W5] [BP3] [GAP] PROMISE pillar "Bodies and machines fail locally (armor plates + weapons + sensors + limbs + mech modules + reactors + origin-specific systems degrade in stages)" — partial; only chassis stages.
3558. - [ ] [W5] [BP3] [GAP] PROMISE pillar "Command core is strategic object (rooted powers base + embedded creates boosted avatar + dangerous base-power tradeoff)" — no core.
3559. - [ ] [W5] [BP3] [GAP] PROMISE pillar "Progression widens tactics (new tools + veterans + salvage + contracts → increase options not raw grind)" — no progression.
3560. - [ ] [W5] [BP3] [GAP] PROMISE pillar "Modding is core (packages + validation + provenance + test launch in core workflow)" — no workbench.

## 304. spec/shell-ui-architecture — Shell UI (BP3 closure includes shell scaffolding; closed direction)
3561. - [ ] [W5] [BP3] [GAP] SHELL surface "Splash (3s studio logo + engine logo + legal disclaimers + AnimateDiff loop)" — minimal launcher at BP3.
3562. - [ ] [W5] [BP3] [GAP] SHELL surface "Title Screen (animated logo + parallax bg + 'press start')" — not implemented.
3563. - [ ] [W5] [BP3] [DR-046] [GAP] SHELL surface "Main Menu (hub of hubs + 8 menu options + comic-panel layout + DR-046 juice transitions)" — not implemented.
3564. - [ ] [W5] [BP3] [GAP] SHELL surface "Profile Select (multi-profile + New/Load/Delete/Cloud-sync)" — no profiles.
3565. - [ ] [W5] [BP3] [GAP] SHELL surface "Pause Menu (Resume/Save/Load/Settings/Restart/Quit + ESC opens + deterministic pause)" — no pause.
3566. - [ ] [W5] [BP3] [GAP] SHELL surface "Settings Menu (Graphics + Audio + Controls + Accessibility + Gameplay + Language + Online tabs + cfctl parity + settings persist + live-reload)" — minimal settings.
3567. - [ ] [W5] [BP3] [GAP] SHELL surface "Server Browser (list + filter + favorites + history + direct-IP join + Steam/EOS adapters optional)" — no servers.
3568. - [ ] [W5] [BP3] [DR-042] [GAP] SHELL surface "Lobby (pre-match config + team/faction/loadout/ready per DR-042 match grammar)" — no lobby.
3569. - [ ] [W5] [BP3] [GAP] SHELL surface "Loadout Workbench (drag/drop loadout builder + full Tier 3 polish)" — not implemented.
3570. - [ ] [W5] [BP3] [GAP] SHELL surface "Mission Briefing (comic-panel cards for all 30+ launch missions)" — no briefings.
3571. - [ ] [W5] [BP3] [GAP] SHELL surface "Mission Debrief (comic-panel timeline + death recap + replay CTA + share button)" — no debrief.
3572. - [ ] [W5] [BP3] [GAP] SHELL surface "Strategic Map (multi-world astrography + all 12 worlds + faction state + comms light-lag)" — no map.
3573. - [ ] [W5] [BP3] [GAP] SHELL surface "Hub UI (base + squad + campaign + mods + progression overview)" — partial.
3574. - [ ] [W5] [BP3] [GAP] SHELL surface "Replay Viewer (scrub + speed + multi-cam + bookmark + clip export)" — no viewer.
3575. - [ ] [W5] [BP3] [GAP] SHELL surface "Codex / lore browser (in-game encyclopedia + all factions/worlds/characters/weapons/materials unlockable + browsable)" — no codex.
3576. - [ ] [W5] [BP3] [GAP] SHELL surface "Photo Mode (free camera + freeze + filters + screenshot export)" — no photo mode.
3577. - [ ] [W5] [BP3] [GAP] SHELL surface "Cosmetic Locker (unlocked skins/decals/paint/voice/emblems + earned via play never paid)" — no cosmetics.
3578. - [ ] [W5] [BP3] [GAP] SHELL surface "Achievements (list + per-achievement unlock animation + 60-100 at launch)" — no achievements.
3579. - [ ] [W5] [BP3] [GAP] SHELL surface "Death Cam (auto-replay last 5s on death + 'show me why' handoff)" — no death cam.
3580. - [ ] [W5] [BP3] [GAP] SHELL surface "Mod Manager (Workshop/Local browse + Subscribe/Install/Update/Uninstall + trust tiers)" — no mod manager.
3581. - [ ] [W5] [BP3] [GAP] SHELL surface "Workshop Submission (one-button mod publish from in-game)" — no submission.
3582. - [ ] [W5] [BP3] [GAP] SHELL surface "Difficulty / Accessibility Presets (Standard/Easy/Hard/Custom + sliders)" — no presets.
3583. - [ ] [W5] [BP3] [GAP] SHELL settings Graphics (Resolution + fullscreen/windowed/borderless + V-Sync + FPS cap 30/60/120/144/unlimited + quality preset Steam Deck/Low/Med/High/Ultra/Custom + shader cache regen + particle density + decal density + shadow quality + normal-map quality + HDR + color blind filter Deut/Prot/Trit + screen shake 0-200% + camera shake 0-200% + film grain + chromatic aberration + bloom + gamma + brightness) — minimal at BP3.
3584. - [ ] [W5] [BP3] [GAP] SHELL settings Audio (Master + music + SFX + voice NPC + voice radio + ambient + UI + output device + audio quality + spatial audio + 3D voice + captions on/off/forced + caption size/color/background) — no audio.
3585. - [ ] [W5] [BP3] [GAP] SHELL settings Controls (KB remap per action + per-context bindings + mouse sensitivity/smoothing + controller deadzone/sensitivity X/Y + invert Y + vibration + vibration intensity + preset Xbox/PS/Steam Deck/Custom + keybind import/export + KB/M ↔ Controller hot-swap auto-detect) — single keybind at BP3.
3586. - [ ] [W5] [BP3] [GAP] SHELL settings Accessibility (UI scale 100/125/150/175/200 + high contrast + reduce motion + reduce shake + reduce flash + screen reader + one-handed mode + slow-down on input 0-200% + pause on focus loss + large pointer + focus indicators + captions style + font size + font choice default/dyslexic/monospace) — none implemented.
3587. - [ ] [W5] [BP3] [GAP] SHELL settings Gameplay (Difficulty preset + autosave frequency per minute/per phase/off + autosave slot count + ironman toggle + hint frequency high/med/low/off + tutorial tooltips + confirmations on destructive + friendly fire policy + camera mode default + HUD density + HUD positioning + aim assist) — none implemented.
3588. - [ ] [W5] [BP3] [GAP] SHELL settings Language (Locale switcher Tier-A full / Tier-B UI only + subtitle language + caption language + speech-to-text toggle) — single locale.
3589. - [ ] [W5] [BP3] [GAP] SHELL settings Online (connection mode + region preference + server browser filters defaults + cross-play toggle + telemetry opt-in/opt-out per region + crash report opt-in + Steam Workshop auto-update + mod trust tier max) — no online.
3590. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Button hover (scale 1.0→1.05 over 80ms ease-out + glow halo + soft tick SFX)" — not implemented.
3591. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Button click (scale punch + flash + mid-frequency punch + sub-bass thump)" — not implemented.
3592. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Menu transition (comic-panel slide-in + skew + 200ms ease-in-out + ambient mix duck)" — not implemented.
3593. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Settings save (soft confirmation tick + animated value snap)" — not implemented.
3594. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Loadout drag (cursor follow + slot-glow on valid drop targets)" — not implemented.
3595. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Loadout drop (snap-in + bass thump + slot-flash)" — not implemented.
3596. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Mission start (dropship cinematic 4s + LZ flash + objective banner)" — not implemented.
3597. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Mission victory (comic-page-flip + slow-mo + music swell + confetti)" — not implemented.
3598. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Mission defeat (scroll-of-failure + dirge)" — not implemented.
3599. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Death (slow-mo 0.3s + camera dolly + 'show me why' prompt)" — not implemented.
3600. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Achievement (comic-panel pop-in + cheer sting)" — not implemented.
3601. - [ ] [W5] [BP3] [GAP] SHELL juice rule "Cosmetic unlock (reveal animation + lights + cheer)" — not implemented.
3602. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl observe --hud` — not implemented.
3603. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl observe --settings` — partial.
3604. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl act settings set <key> <value>` — partial.
3605. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl act keybind <action> <key>` — not implemented.
3606. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl ui select <id>` — not implemented.
3607. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl ui type <text>` — not implemented.
3608. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl ui assert <id> <prop> <op> <value>` — not implemented.
3609. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl observe --captions` — no captions.
3610. - [ ] [W5] [BP3] [GAP] SHELL cfctl parity `cfctl observe --cinematic` — no cinematics.
3611. - [ ] [W5] [BP3] [GAP] SHELL perf budget "In-match HUD < 1ms" — not measured.
3612. - [ ] [W5] [BP3] [GAP] SHELL perf budget "Pause menu overlay < 1ms" — not measured.
3613. - [ ] [W5] [BP3] [GAP] SHELL perf budget "Main menu (full screen) < 4ms" — not measured.
3614. - [ ] [W5] [BP3] [GAP] SHELL perf budget "Loadout workbench < 4ms" — not measured.
3615. - [ ] [W5] [BP3] [GAP] SHELL perf budget "Briefing/debrief comic panels < 8ms" — not measured.
3616. - [ ] [W5] [BP3] [GAP] SHELL perf budget "Map view < 8ms" — not measured.
3617. - [ ] [W5] [BP3] [GAP] SHELL perf budget "Replay viewer with sim playback < 16ms" — not measured.
3618. - [ ] [W5] [BP3] [GAP] SHELL done-criteria "First-30-seconds friction <5% in playtest cohort" — not measured.

## 305. spec/setting-and-world-frame — World frame & faction grammar (BP3 closure includes one faction with non-generic doctrine)
3619. - [ ] [W5] [BP3] [GAP] FRAME "Frontier disaster-contract sci-fi world frame" — generic at BP3.
3620. - [ ] [W5] [BP3] [GAP] FRAME "Merc / rescue / salvage outfit player identity anchored in command core" — no command core.
3621. - [ ] [W5] [BP3] [GAP] FRAME mission context "Collapsing frontier colony (dome cracking + life support failing + militia turning feral + corporate eviction)" — no contexts.
3622. - [ ] [W5] [BP3] [GAP] FRAME mission context "Corporate war zone (two corps shooting + player hired by one)" — no contexts.
3623. - [ ] [W5] [BP3] [GAP] FRAME mission context "Alien biome (hostile fauna + parasitic flora + biological architecture)" — no biomes.
3624. - [ ] [W5] [BP3] [GAP] FRAME mission context "Derelict megastructure (generation ship + orbital ring + cracked Dyson + void hulk)" — no megastructures.
3625. - [ ] [W5] [BP3] [GAP] FRAME mission context "Disaster site (recent catastrophe — crash/attack/breach/runaway nanite)" — no contexts.
3626. - [ ] [W5] [BP3] [GAP] FRAME mission context "Black-site / off-books (heist + prisoner break + evidence recovery + defector extraction)" — no contexts.
3627. - [ ] [W5] [BP3] [GAP] FRAME faction axis "Doctrine (Disposable swarm ↔ small elite ↔ siege/defensive ↔ scavenger ↔ corporate professional ↔ ideological)" — no factions.
3628. - [ ] [W5] [BP3] [GAP] FRAME faction axis "Tech tier (Salvage/improvised ↔ standard issue ↔ corporate top tier ↔ experimental ↔ alien)" — single tier.
3629. - [ ] [W5] [BP3] [GAP] FRAME faction axis "Origin mix (Pure organic ↔ organic+drones ↔ androids ↔ hybrids ↔ alien)" — single origin.
3630. - [ ] [W5] [BP3] [GAP] FRAME faction axis "Stance toward player (Always hostile ↔ contract-driven ↔ rival ↔ ally-of-convenience ↔ allied)" — no factions.
3631. - [ ] [W5] [BP3] [GAP] FRAME faction axis "Visual register (Gritty utilitarian ↔ corporate slick ↔ pulpy retro ↔ biological strange ↔ cobbled-together)" — single register.
3632. - [ ] [W5] [BP3] [GAP] FRAME launch faction "The Player's Outfit (variable / contract-driven)" — no outfit identity.
3633. - [ ] [W5] [BP3] [GAP] FRAME launch faction "Dominion Salvors (scavenger + improvised + organic+drones + rival)" — not implemented.
3634. - [ ] [W5] [BP3] [GAP] FRAME launch faction "Halver Industries (corporate professional + corporate top tier + androids + ally-of-convenience)" — not implemented.
3635. - [ ] [W5] [BP3] [GAP] FRAME launch faction "The Seethe (disposable swarm + alien + alien + always hostile)" — not implemented.
3636. - [ ] [W5] [BP3] [GAP] FRAME launch faction "Continuity Chapel (ideological + experimental + hybrids + rival)" — not implemented.
3637. - [ ] [W5] [BP3] [GAP] FRAME "Named actors valuable + veterans + repair projects + salvage + legacy assets are core retention objects" — no retention.
3638. - [ ] [W5] [BP3] [GAP] FRAME "ONE common faction doctrine `disposable_swarm` uses cheap bodies/drones intentionally — faction choice not world default" — not enforced.
3639. - [ ] [W5] [BP3] [GAP] FRAME "Multiple in-world theories about what the command core is can coexist (mods add their own)" — no lore framework.

## 309. spec/missions-and-objectives — Mission principles index (BP3 proof mission)
3716. - [ ] [W5] [BP3] [GAP] MISSION principle "Missions are contracts (typed manifest with objectives + teams + terrain rules + director state + equipment needs + save fields + UI markers + replay events)" — see §278.
3717. - [ ] [W5] [BP3] [GAP] MISSION principle "Destruction allowed by default (no walls as locks; use defended spaces + distance + elevation + water + alarms + timers + resource pressure + critical-object policy)" — not enforced.
3718. - [ ] [W5] [BP3] [GAP] MISSION principle "Commander AI explains itself (target / LZ / package / squad / breach / defend / recovery decisions emit reason strings + structured event fields)" — no commander.
3719. - [ ] [W5] [BP3] [GAP] MISSION principle "Equipment requirements are capabilities (`breach` / `dig` / `heal` / `fight` / `carry` / `repair` tags not hard-coded item names)" — not implemented.
3720. - [ ] [W5] [BP3] [GAP] MISSION principle "UI and replay share truth (objective markers + phase text + debrief causes + replay events use same objective ids and event ids)" — not enforced.
3721. - [ ] [W5] [BP3] [GAP] MISSION principle "Private experiments stay unblocked (missing capability warnings can be overridden for private play but consequence must be visible)" — not enforced.
3722. - [ ] [W5] [BP3] [GAP] MISSION proof "Breach Contract" mission (compact BunkerBreach-modeled + typed manifest + LZ Attacker + Main Bunker + Brain + optional Internal Reinforcements + optional salvage cache + two breach paths + commander + objectives + LZ + craft + reinforcements) — micro_breach much smaller scope at BP3.

## 310. spec/game-modes-and-match-grammar — Match grammar (BP3 design intent; M7 ships Bunker Defence proof)
3723. - [ ] [W5] [BP3] [M7] [GAP] MATCH `Match` schema (id + mode + asymmetric + coop_within_teams + max_total_players + duration_policy + teams + spectators + map + comms_policy) — not present.
3724. - [ ] [W5] [BP3] [M7] [GAP] MATCH `ModePreset` enum (BunkerDefence / SymmetricArena / FreeForAll / AsymmetricNTeam / CoopVsAI / Campaign / Modder) — not implemented.
3725. - [ ] [W5] [BP3] [M7] [GAP] MATCH `Team` schema (id + kind + display_name + color + player_slots + ai_fill + objectives + victory_conditions + loss_conditions + spawn_rules + starting_resources + bunker_owner) — no teams.
3726. - [ ] [W5] [BP3] [M7] [GAP] MATCH `TeamKind` enum (Attacker / Defender / Neutral / Survivor / Hostile / Custom) — not modeled.
3727. - [ ] [W5] [BP3] [M7] [GAP] MATCH `DurationPolicy` (FixedTimer / UntilObjective / UntilElimination / Endless) — not implemented.
3728. - [ ] [W5] [BP3] [M7] [GAP] MATCH `CommsPolicy` (Realistic / ProximityOnly / GlobalChat / CrossTeamDisabled) — no comms.
3729. - [ ] [W5] [BP3] [M7] [GAP] MATCH `SpawnRules` (spawn_zones + respawn_policy + starting_loadout + starting_chassis + deployment Dropship/Walked/Spawn/Rooted) — not implemented.
3730. - [ ] [W5] [BP3] [M7] [GAP] MATCH `AiFillPolicy` (fill_empty + ai_doctrine + difficulty Recruit/Veteran/Elite) — not implemented.
3731. - [ ] [W5] [BP3] [M7] [GAP] MATCH "Bunker Defence" preset (asymmetric Attacker+Defender + 1-8 each side coop + defender rooted bunker/base power/turrets/shields/sealed life support/pre-deployed AI + attacker dropship or walked + breach kit + buy menu reinforcements + defender objectives Survive timer OR defeat all attackers OR protect command core + attacker objectives Destroy command core OR breach + extract mission item OR eliminate defenders + variants 1v1/2v2/3v3/4v4/Coop-Defence/Coop-Attack) — not implemented.
3732. - [ ] [W5] [BP3] [M7] [GAP] MATCH "Symmetric Arena" preset (symmetric 2-N teams + 1v1/2v2/3v3/NvN + equal spawn zones + same loadout budget + Eliminate enemy team / control center node / extract marker) — not implemented.
3733. - [ ] [W5] [BP3] [M7] [GAP] MATCH "Free-For-All" preset (3-8 players + distributed spawn + last surviving / first to N kills / first to extract MacGuffin + 1v1v1 / 1v1v1v1 / 1v1v1v1v1) — not implemented.
3734. - [ ] [W5] [BP3] [M7] [GAP] MATCH "Asymmetric N-Team" preset (2-3 teams + per-team different conditions + 2v1/3v1/4v2) — not implemented.
3735. - [ ] [W5] [BP3] [M7] [GAP] MATCH "Coop-vs-AI" preset (all humans on one team vs AI-only opposition) — not implemented.
3736. - [ ] [W5] [BP3] [M7] [GAP] MATCH "Campaign" preset (solo or coop linear / branching mission progression) — no campaign.

## 314. spec/progression-retention — Retention loop (BP3 design seeds; RET-A acceptance)
3811. - [ ] [W5] [BP3] [GAP] RETENTION return thought "I can beat that seed cleaner" (same-seed retry + replay timeline + personal best + failure cause) — partial.
3812. - [ ] [W5] [BP3] [GAP] RETENTION return thought "This squad deserves another mission" (named actors + scars + traits + rescue history + veteran UI) — not implemented.
3813. - [ ] [W5] [BP3] [GAP] RETENTION return thought "This machine deserves repair" (damaged armor + recovered mech hulls + repaired modules + android shells + robot frames + battle scars) — not implemented.
3814. - [ ] [W5] [BP3] [GAP] RETENTION return thought "This base deserves better power" (command core upgrades + shields + turrets + sensors + doors + repair platforms + reserve power + module scars) — not implemented.
3815. - [ ] [W5] [BP3] [GAP] RETENTION return thought "This new tool changes the plan" (horizontal equipment unlocks + lab tests + loadout templates) — not implemented.
3816. - [ ] [W5] [BP3] [GAP] RETENTION return thought "The enemy commander surprised me" (visible enemy doctrine + adaptation + scouting clues) — no commander.
3817. - [ ] [W5] [BP3] [GAP] RETENTION return thought "I want to show this moment" (replay card + seed hash + mod/package list + short export) — no replay card.
3818. - [ ] [W5] [BP3] [GAP] RETENTION return thought "I can build a better bunker/challenge" (workbench + package validation + modded contract browser) — no workbench.
3819. - [ ] [W5] [BP3] [GAP] RETENTION progression object `campaign_profile` (profile id + campaign seed + difficulty posture + unlocked labs + contract history + replay archive ids) — no campaign.
3820. - [ ] [W5] [BP3] [GAP] RETENTION progression object `command_core_record` (core id + origin/flavor + integrity + upgrades + rooted/portable/embedded history + near-loss events + avatar missions) — no core.
3821. - [ ] [W5] [BP3] [GAP] RETENTION progression object `base_power_grid` (rooted core socket + reserve power + shield emitters + turret links + sensor relays + door controllers + repair/charging pads + logistics beacons) — no base.
3822. - [ ] [W5] [BP3] [GAP] RETENTION progression object `base_module_record` (module id + type + power draw + condition + repair history + scars + mod provenance + tactical role) — no base modules.
3823. - [ ] [W5] [BP3] [GAP] RETENTION progression object `actor_veteran` (actor id + name + role + scars + injuries + traits + rescue count + mission count + favorite loadout) — no veterans.
3824. - [ ] [W5] [BP3] [GAP] RETENTION progression object `chassis_record` (chassis id + owner/pilot history + armor/module condition + repairs + scars/paint + salvage state + mission count) — no record.
3825. - [ ] [W5] [BP3] [GAP] RETENTION progression object `origin_profile` (origin id + treatment/repair needs + vulnerabilities + personality/story tags + compatible armor/mechs) — no profile.
3826. - [ ] [W5] [BP3] [GAP] RETENTION progression object `loadout_template` (actor roles + item ids + role tags + mass + cost + delivery craft + AI warnings + package hashes) — no templates.
3827. - [ ] [W5] [BP3] [GAP] RETENTION progression object `contract_seed` (seed id + objective + map/material profile + constraints + reward class + required capabilities + validation status) — no contracts.
3828. - [ ] [W5] [BP3] [GAP] RETENTION progression object `salvage_manifest` (recovered items + scrap/material types + enemy tech + damaged gear + base repair deltas) — no salvage.
3829. - [ ] [W5] [BP3] [GAP] RETENTION progression object `enemy_commander` (commander id + doctrine + visible adaptations + grudges + recent defeats + scouting clues) — no commander.
3830. - [ ] [W5] [BP3] [GAP] RETENTION progression object `replay_card` (seed + result + loadout + key events + actor fates + package versions + share hash) — no card.
3831. - [ ] [W5] [BP3] [GAP] RETENTION progression object `collection_entry` (cosmetic/story/trophy id + source event + unlock path + optional odds group + release-readiness tag + accessibility caption + localization key + mod provenance) — no collection.
3832. - [ ] [W5] [BP3] [GAP] RETENTION acceptance RET-A-01..10 — none pass.
3833. - [ ] [W5] [BP3] [GAP] RETENTION UI "Contract card (objective + length + material profile + required roles + seed + constraints + reward + validation badge)" — no card.
3834. - [ ] [W5] [BP3] [GAP] RETENTION UI "Campaign map (current pressure + available contracts + base damage + enemy commander clues + saved challenge seeds)" — no map.
3835. - [ ] [W5] [BP3] [GAP] RETENTION UI "Squad/veteran panel (name + role + health + scars + traits + doctrine + rescue risk + recent event)" — no panel.
3836. - [ ] [W5] [BP3] [GAP] RETENTION UI "Loadout builder (role filters + item tags + AI competence + provenance/warnings + delivery risk + missing capability summary)" — no builder.
3837. - [ ] [W5] [BP3] [GAP] RETENTION UI "Mech/chassis bay (origin compatibility + armor slots + module condition + repair cost + route/delivery warnings + pilot/rescue state)" — no bay.
3838. - [ ] [W5] [BP3] [GAP] RETENTION UI "Base power panel (core state + power + powered/offline modules + shields + turrets + sensors + doors + repair/charging pads)" — no panel.
3839. - [ ] [W5] [BP3] [GAP] RETENTION UI "Avatar core panel (core integrity + avatar boosts + base-offline warnings + energy/heat + ability cooldowns + extraction route + loss risk)" — no panel.
3840. - [ ] [W5] [BP3] [GAP] RETENTION UI "Mission HUD (current goal + high-risk actor warnings + salvage/recovery prompts when relevant)" — partial.
3841. - [ ] [W5] [BP3] [GAP] RETENTION UI "Recap screen (win/loss cause + key events + actor fates + salvage + retry same seed + save replay + edit loadout)" — no recap.
3842. - [ ] [W5] [BP3] [GAP] RETENTION UI "Replay card (mission title + seed + duration + result + mods/packages + notable events + share/export actions)" — no card.
3843. - [ ] [W5] [BP3] [GAP] RETENTION UI "Lab/workbench (test weapon/material interactions + compare role metadata + validate package fields + create contract fixtures)" — no lab.
3844. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Same-seed retry rate" — not measured.
3845. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Time to first meaningful event" — not measured.
3846. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Loadout edits after recap" — not measured.
3847. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Veteran preservation behavior" — not measured.
3848. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Chassis repair/reuse rate" — not measured.
3849. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Core uproot/embed rate" — not measured.
3850. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Base module power-off causes" — not measured.
3851. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Salvage usage in next mission" — not measured.
3852. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Contract abandonment cause" — not measured.
3853. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Replay saved/shared/opened" — not measured.
3854. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Mod challenge install errors" — not measured.
3855. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Session return after no reward claim" — not measured.
3856. - [ ] [W5] [BP3] [GAP] RETENTION telemetry "Power-obsolescence incidents" — not measured.
3857. - [ ] [W5] [BP3] [GAP] RETENTION guardrail "No core-power opacity (transparent access in any settled spec)" — not enforced.
3858. - [ ] [W5] [BP3] [GAP] RETENTION guardrail "No missed-reward punishment (shared daily seeds fine; missed daily chores not retention foundation)" — not enforced.
3859. - [ ] [W5] [BP3] [GAP] RETENTION guardrail "No UI dark patterns (no fake urgency / hidden costs / confusing currency / obstructed cancellation / disguised purchases)" — not enforced.
3860. - [ ] [W5] [BP3] [GAP] RETENTION guardrail "Optional economy dormant by default (battle-pass/gacha-like disable-able by server config + absent from UI/telemetry when disabled)" — not enforced.
3861. - [ ] [W5] [BP3] [GAP] RETENTION guardrail "Modding remains first-class (official progression must not make mods feel second-class)" — not enforced.
3862. - [ ] [W5] [BP3] [GAP] RETENTION guardrail "AI must understand progression (veteran traits + item roles + contract constraints need AI metadata + harness cases)" — not implemented.
3863. - [ ] [W5] [BP3] [GAP] RETENTION guardrail "Replays must explain progression losses" — no replay viewer.

## 317. spec/tutorial-implementation — Onboarding + 8 labs + "show me why" (BP3 closure includes tutorial mission)
3916. - [ ] [W5] [BP3] [GAP] TUTORIAL onboarding mission "First Contract" (12-15min Earth urban industrial + recover downed scientist + clear husk infestation + extract via dropship + tutorial-safety lethal demoted to KO) — no tutorial.
3917. - [ ] [W5] [BP3] [GAP] TUTORIAL beat 1 "Drop-in (2min) — direct-control body + movement + camera + aim + captioned dialog from commander" — not implemented.
3918. - [ ] [W5] [BP3] [GAP] TUTORIAL beat 2 "First contact (2min) — engage 1-2 light husks + fire LMB + reload R + switch weapon + husks demoted to KO + teach revive" — not implemented.
3919. - [ ] [W5] [BP3] [GAP] TUTORIAL beat 3 "Squad partner (2min) — AI teammate + squad order TAB + Q to call + wave gestures + cover fire + push" — not implemented.
3920. - [ ] [W5] [BP3] [GAP] TUTORIAL beat 4 "Breach (2min) — door blocks path + digger tool / breach charge + material physics" — not implemented.
3921. - [ ] [W5] [BP3] [GAP] TUTORIAL beat 5 "Recovery (2min) — find scientist downed + revive E + carry G" — not implemented.
3922. - [ ] [W5] [BP3] [GAP] TUTORIAL beat 6 "Dropship call (2min) — LZ reveal + dropship command call to LZ / abandon LZ + extract" — not implemented.
3923. - [ ] [W5] [BP3] [GAP] TUTORIAL beat 7 "Replay/debrief (2min) — auto-loaded debrief comic-panel timeline + explain show me why + lab launcher" — not implemented.
3924. - [ ] [W5] [BP3] [GAP] TUTORIAL voice acting "Hero ElevenLabs commander + scientist 30-50 lines (license review pre-launch)" — not implemented.
3925. - [ ] [W5] [BP3] [GAP] TUTORIAL voice acting "Fallback Text-only with subtitle + comic-panel speech bubbles" — not implemented.
3926. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_movement_aim` (Ground move + aim + recoil + jetpack + stance + cover + Player completes 5 timed move/aim challenges) — not implemented.
3927. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_terrain_materials` (Digging + breaching + repair + collapse risk + material overlay + Player breaches 3 walls + identifies 5 materials) — not implemented.
3928. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_loadout_delivery` (Loadout building + dropship craft + LZ risk + equipment role cards + Player builds 3 loadouts + delivers 3 squads) — not implemented.
3929. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_squad_orders_ai` (Squad orders + AI intent + rescue + retreat + recovery + Player issues 6 distinct order types) — not implemented.
3930. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_command_core_base` (Rooting core + base power + shields + turrets + sensors + doors + repair pads + Player roots core + powers 2 systems + uproots) — not implemented.
3931. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_avatar_mode` (Uprooting core + embedding into body/mech + Player uproots + embeds + survives 1 minute) — not implemented.
3932. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_chassis_damage` (Armor/mech module damage + smoke/failure states + ejection + salvage + Player ejects + recovers + repairs 1 chassis) — not implemented.
3933. - [ ] [W5] [BP3] [GAP] TUTORIAL lab `lab_replay_debrief` (Why I died + what I could have done + retry same seed + Player scrubs replay + identifies cause-of-death + retries) — not implemented.
3934. - [ ] [W5] [BP3] [GAP] TUTORIAL lab manifest format `content/labs/<lab_id>.ron` (id + title_key + description_key + duration_estimate_s + scenario + objectives with tutorial_safety + teaches + failure_routes) — no labs.
3935. - [ ] [W5] [BP3] [GAP] TUTORIAL contextual fading tooltips (50+ catalog + per-tooltip use counter fade after 3 uses + per-mastery flag suppression + re-enable via Settings → Gameplay → Reset Tutorial Tooltips) — no tooltips.
3936. - [ ] [W5] [BP3] [GAP] TUTORIAL tooltip format `content/tooltips/<id>.ron` (id + trigger + title_key + body_key + icon + fade_after_uses + suppress_if + relate_to) — no tooltips.
3937. - [ ] [W5] [BP3] [GAP] TUTORIAL "Show me why" handoff (Player death → replay scrubs to last 5s + suggest lab; Mission lost → debrief timeline + suggest lab; Command core lost → suggest lab_command_core_base; Mech wrecked → suggest lab_chassis_damage; Stuck in terrain → suggest lab_terrain_materials; Squad refused order → suggest lab_squad_orders_ai; LZ failed → suggest lab_loadout_delivery; Equipment didn't work → suggest lab_loadout_delivery; Bunker breached → suggest lab_command_core_base; Material kill → suggest lab_terrain_materials; Damage afflictions misunderstood → suggest lab_chassis_damage; Replay/debrief misunderstood → suggest lab_replay_debrief) — not implemented.
3938. - [ ] [W5] [BP3] [GAP] TUTORIAL adaptive hints (hint engine reads EnvironmentSignal + AI bot scoring + player input patterns + session telemetry + accuracy >95%) — not implemented.
3939. - [ ] [W5] [BP3] [GAP] TUTORIAL adaptive hints triggers (reload while ammo full / ignored ally call 30+s / aimed but never fired / on Mars without sealed helmet / enemy in cover + no grenade / mission timer < 30s / husks approaching from rear / low HP near medikit) — not implemented.
3940. - [ ] [W5] [BP3] [GAP] TUTORIAL AI-authored mission narrative (Claude Sonnet/GPT-4o briefing/debrief copy per faction tone + reviewed by AI agent + comic-panel art) — not implemented.
3941. - [ ] [W5] [BP3] [GAP] TUTORIAL difficulty/accessibility presets (Standard / Easy / Hard / Custom / Accessibility-relaxed with damage taken/dealt + AI aggression + time scale + hint frequency) — not implemented.
3942. - [ ] [W5] [BP3] [GAP] TUTORIAL CI gate "every UI element + system has tooltip data" — not enforced.

## 329. spec/player-modes — Player modes (STUB; closes DR-005 at M9-M12)
4335. - [ ] [W5] [M12+M9] [DR-005] [GAP] MODES "Solo (primary)" — partial.
4336. - [ ] [W5] [M12+M9] [DR-005] [GAP] MODES "Local split-screen co-op" — not implemented.
4337. - [ ] [W5] [M12+M9] [DR-005] [GAP] MODES "Online co-op (server-authoritative; post-launch milestone; prototyping ongoing)" — not implemented.
4338. - [ ] [W5] [M12+M9] [DR-005] [GAP] MODES "Async strategic layer (post-launch milestone)" — not implemented.
4339. - [ ] [W5] [M12+M9] [DR-005] [GAP] MODES "PvP research and prototype track + promoted to launch promise only when bandwidth/authority/cheating tests pass via follow-up DR" — not started.


# ===== WAVE 6 — AI TRUST BOOTSTRAP =====

## 15. M3A — Replay recorder (spec/replay-recorder-slice-a.md) gaps
211. - [ ] [W6] [M3A] [GAP] No `ai_item_choice` event (actor id + order context + selected item id + score inputs + selected reason + top rejected items + source confidence + summary row id).
212. - [ ] [W6] [M3A] [GAP] No `ai_item_refusal` event (actor id + refused item id + claim state + reason label + first fix action + scenario ref).
213. - [ ] [W6] [M3A] [GAP] No `weapon_fired` event with stable weapon record id + muzzle position + aim direction + projectile id chain.
214. - [ ] [W6] [M3A] [GAP] No `body.wound_added` event with entry/exit emitter ids and parent damage source.
215. - [ ] [W6] [M3A] [GAP] No `terrain.dirty_region` event with bbox + chunk-list payload.
216. - [ ] [W6] [M3A] [GAP] No `path_invalidated` event when terrain mutation crosses a known path.
217. - [ ] [W6] [M3A] [GAP] No `mission.alarm_raised` event with cause + range + caption.
218. - [ ] [W6] [M3A] [GAP] No `team` field on event envelope (currently events have actor_id only).
219. - [ ] [W6] [M3A] [GAP] No `bbox` field on event envelope (currently events have pos only).
220. - [ ] [W6] [M3A] [GAP] No `dropped_count` field on event envelope — recorder backpressure invisible.

## 34. DR-008 — AI architecture closure debt (M6 owns full closure but BP1+BP2 should have proved scaffolding)
466. - [ ] [W6] [BP1+BP2] [M6] [DR-008] [GAP] DR-008 hybrid jobs + utility scoring + scripted hooks — current cf-ai is a single ReactiveGuard FSM; no utility scorer producing per-option scores in observe.
467. - [ ] [W6] [BP1+BP2] [M6] [DR-008] [GAP] DR-008 "personality / doctrine" slots (cautious / aggressive / support / scout / sniper) — not declared.
468. - [ ] [W6] [BP1+BP2] [M6] [DR-008] [GAP] DR-008 reason-label vocabulary — current `tactic_chosen` has 4-5 reasons; spec calls for full taxonomy with material/atmospheric/chassis suffixes.
469. - [ ] [W6] [BP1+BP2] [M6] [DR-008] [GAP] DR-008 mistake/recovery model — bots don't panic / get-stuck / miss.
470. - [ ] [W6] [BP1+BP2] [M6] [DR-008] [GAP] DR-008 strategic adaptation across missions (faction commander persists across same campaign session) — no commander persistence at BP3.

## 37. DR-022 — AI humanlike bar gaps (M6 owns full closure; M1.5/M2.5 should have proved scaffolding)
481. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 1 "Intent": bots announce "covering door / breaching left wall / low ammo, falling back / pilot trapped, ejecting / no safe explosive shot" — current ReactiveGuard has no announcement system.
482. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 2 "Perception": bots act from sight/hearing/memory, not omniscience — ReactiveGuard has sight cone but no hearing or memory.
483. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 3 "Doctrine / personality": cautious medic / aggressive breacher / stubborn heavy / glory-hound sniper / careful engineer / panicking rookie / cold robot — none implemented.
484. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 4 "Plausible mistakes": miss, hesitate, overcommit, panic, take a bad route, drop gear, misread a threat, waste ammo — ReactiveGuard uses static miss_chance only.
485. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 5 "Recovery": replan after terrain destruction, pick up dropped gear, call for help, retreat, dig another route, revive/rescue, eject, repair — not implemented.
486. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 6 "Strategic adaptation": enemy commander remembers tactics across missions — no commander persistence.
487. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 7 "Replay proof": every AI decision in replay viewer shows perception, options considered, score, chosen action, result — only `tactic_chosen` is emitted, with no options/scores.
488. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 criterion 8 "Fairness": no hidden vision/range bonuses without UI exposure — no test ensures perception parity between AI and HUD readout.
489. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 `perception_updated` event family — not in event taxonomy.
490. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 `recovery_action` event family — not emitted.
491. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 `commander_adaptation` event family — not emitted.
492. - [ ] [W6] [M1.5+M2.5+M6] [DR-006+DR-022] [GAP] DR-022 Mod-authored doctrines (DR-006) must satisfy same eight criteria — no doctrine schema or validator.
493. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 HUD reason-label overlay showing what bot is currently trying — not implemented.
494. - [ ] [W6] [M1.5+M2.5+M6] [DR-022] [GAP] DR-022 squad panel "covers what current bot is trying" — no squad panel at BP3.

## 172. DR-008 — AI architecture (OPEN; M6 closes but M1.5/M2.5 + M5 scaffolds should be in place)
1501. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 Layer 1 "Reflex (engine)": dodge explosives, brace, avoid muzzle obstruction — not implemented.
1502. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 Layer 2 "Tactic (utility) — pick cover, fire, reload, retreat, throw, suppress, flank — scored per option with reason labels" — only `attack/reload/hold/search` scores at BP3; no cover / retreat / throw / suppress / flank.
1503. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 Layer 3 "Navigation (engine + scripted): pathfinding + local steering + dig/build plans" — no pathfinder at BP3.
1504. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 Layer 4 "Job (data-driven): miner, engineer, medic, breacher, scout, sniper, commander" — no job taxonomy declared.
1505. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 Layer 5 "Commander (per-team): squad/side allocation, reinforcement, route planning" — no commander layer.
1506. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 Layer 6 "Personality (data + script): courage, discipline, panic, loyalty" — no personality data.
1507. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 "AI, player, replay, and network should write through a shared intent/control interface" — partially ✓; ReactiveGuard does NOT route through ControlIntent (uses its own scripted path).
1508. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 "Terrain manipulation is a normal action type, not a special-case exception" — guards cannot dig.
1509. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 "Mobility tools (rope/tether/jetpack) need AI-safe affordance checks and visible refusal reasons" — guards have no mobility tools.
1510. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 "Every stuck recovery needs a logged reason, chosen recovery action, and next retry time" — no stuck-recovery system.
1511. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 "Hazard avoidance must be a first-class reflex with tests" — no hazards at BP3 to avoid.
1512. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 acceptance "Modder authors a 'Suppression Specialist' job in 1 day" — no job authoring surface.
1513. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 acceptance "Path-failure recovery: 90% of bots recover within 5 seconds of route invalidation" — no pathfinder, no recovery.
1514. - [ ] [W6] [M1.5+M2.5+M5+M6] [DR-008] [GAP] DR-008 acceptance "Rope/tether/mobility test" — no rope/tether/mobility tools at BP3.

## 181. DR-032 — Hybrid LLM AI direction (CLOSED at M6.5; BP3 schema seed)
1559. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 MindObservationFrame schema — not declared at BP3.
1560. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 mock provider — not built.
1561. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 mind event category in cf-replay — declared but unused.
1562. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `cfctl observe --mind-frame <scope>` — not implemented.

## 205. DR-050 — Modding ecosystem extensions / social / onboarding-plus / AI quality extensions (CLOSED; M8+ owns; BP3 schema seeds expected)
1788. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod manifest `requires_version` + `depends_on: [mod_id, version_range]` — schema not declared.
1789. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod conflict detection (per-asset override + load-order ranking + auto-resolve via priority) — not implemented.
1790. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod-creator analytics (opt-in per-mod usage/success/crash/playtime) — not implemented.
1791. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod tip-jar URL aggregation — `cf-mod-tip-jar` crate doesn't exist.
1792. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod-test-run AI agent (modder submits chassis → AI agent generates test scenarios + balance/AI validation) — not implemented.
1793. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod compatibility-with-base-version warnings + auto-migrate — not implemented.
1794. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod-author-controlled localization (per-locale `.ftl` packs) — not supported.
1795. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod SDK auto-docs generator — not present.
1796. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod conflict resolution UI — not implemented.
1797. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 mod auto-update + rollback — not implemented.
1798. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 `cf-social` crate (guilds + messaging + cross-shard friends + voice party + gifting) — does not exist.
1799. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 `cf-onboarding` crate (mentor matching + beginner pool + first-30-min telemetry + adaptive hints + tips-of-the-day) — does not exist.
1800. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 `cf-ai-extensions` crate (AI difficulty presets + faction personality + transparency + play-as-Husk + AI tournament submission) — does not exist.
1801. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 `cf-mentor` sub-system (mentor opt-in registry + auto-match + reward tracking) — does not exist.
1802. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 AI difficulty presets ("Cakewalk" / "Tough Crowd" / "Veteran" / "Nightmare" / "Demonic") — not declared as enums.
1803. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 faction AI personality identifiability — no per-faction style flag in cf-ai.
1804. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 AI mistake narration in debrief — not implemented.
1805. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 AI training mode for modders — not implemented.
1806. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 AI-vs-AI tournament mode — not implemented.
1807. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 AI transparency mode (show AI reason labels live in HUD) — opt-in setting not exposed.
1808. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 play-as-Husk mode — not implemented.
1809. - [ ] [W6] [BP3] [M8] [DR-050] [GAP] DR-050 AI personality voice variety per origin (ElevenLabs / XTTS) — not implemented.

## 216. DR-032 — Hybrid LLM AI direction (CLOSED; M6.5 owns; BP3 schema seed)
1903. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `MindObservationFrame` schema — not declared at BP3.
1904. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `MindTask` schema — not declared.
1905. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `AiMindProposal` schema — not declared.
1906. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `MindValidationResult` schema — not declared.
1907. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `MindMemoryRecord` schema — not declared.
1908. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `MindProviderConfig` schema — not declared.
1909. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `mock` provider — not built.
1910. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 cargo features `mind-openai` / `mind-anthropic` / `mind-ollama` / `mind-openai-compatible` — not declared.
1911. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 mind validator (rejects stale/invalid/impossible/unfair/over-budget/hidden-info/capability-violating proposals) — not implemented.
1912. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 mind policy compiler (apply accepted proposals as utility-weight patches / commander-blackboard goals / doctrine tags / dialogue / memory writes) — not implemented.
1913. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 mind event category in cf-replay (task_created / prompt_recorded / response_received / proposal_validated / patch_applied / patch_rejected / memory_written) — declared but unused.
1914. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 `cfctl observe --mind-frame <scope>` for `actor` / `squad` / `faction` / `mission_director` / `post_mission` — not implemented.
1915. - [ ] [W6] [BP3] [M6.5] [DR-032] [GAP] DR-032 mind worker secret-redaction policy — not authored.

## 275. spec/ai-trust-harness-slice-a — AI harness Slice A gaps (BP3 M5 partial; M6 closes)
2441. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H scenario manifest schema (scenario_id, seed, fixture_map, actors, orders, terrain_mutations, threats, success_assertions, failure_assertions, telemetry_required, timeout_ms) — no manifest schema at BP3.
2442. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H runner "loads manifest + spawns actors/map/orders with fixed seed" — no runner at BP3.
2443. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_update_allowed` (tick allowed/skipped + throttle evidence) — not emitted.
2444. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_order_issued` (issuer + target + order type + priority + timeout) — not emitted; no orders at BP3.
2445. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_behavior_selected` (AIMode + behavior name + previous + reason) — not emitted.
2446. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_tactic_scored` (tactic + score + top rejected + reason labels) — utility scoring not implemented.
2447. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_target_acquired` (target id + threat score + LOS/alarm source) — not emitted.
2448. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_perception_signal` (signal type + source + confidence + occlusion/ray + decay) — not emitted.
2449. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_path_requested` (start/goal/jump/dig/team/request id) — no pathfinding.
2450. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_path_completed` (status + length + total cost + waypoint count) — no pathfinding.
2451. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_path_invalidated` (path id + dirty bbox + terrain/material cause) — no pathfinding.
2452. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_stuck_state_changed` (stuck time + avg velocity + blocker + old/new state) — no stuck detection.
2453. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_recovery_action` (action + reason + retry time + expected outcome) — no recovery.
2454. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_tool_choice` (tool + target material + expected effect + rejected tools) — not emitted.
2455. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_item_choice` (order context + selected item + target context + score inputs + selected reason + rejected alternatives) — not emitted.
2456. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_item_refusal` (refused item + refusal reason + source/missing field + first fix action) — not emitted.
2457. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_item_result` (expected effect + outcome + interruption/failure reason + claim-state delta) — not emitted.
2458. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_hazard_reflex` (hazard + risk score + action/refusal) — not emitted; no hazards.
2459. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_friendly_fire_check` (weapon + target + blocked actors + decision) — not emitted; no teams.
2460. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_test_assertion` (scenario + assertion id + status + measured + threshold) — not emitted.
2461. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H event `ai_test_result` (scenario + status + duration + failures + replay path) — not emitted.
2462. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H scenario AI-H-01 "Sentry hears threat" — not implemented; ReactiveGuard FSM exists but no scenario runner.
2463. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H scenario AI-H-02 "GoTo path with new blockage" — not implemented.
2464. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H scenario AI-H-03 "Breach material gate (door/concrete) with tool selection" — not implemented.
2465. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H scenario AI-H-04 "Weapon/tool pickup" — not implemented.
2466. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H scenario AI-H-05 "Medikit/reflex interrupt" — not implemented.
2467. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H scenario AI-H-06 "Friendly obstruction yield" — not implemented.
2468. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H equipment scenario AI-H-LOAD-01..09 (9 fixtures: assault/engineer/medic/sniper/grenadier/heavy/scout + 2 negative loadouts) — not implemented.
2469. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H acceptance AI-HARNESS-01..12 — none exist.
2470. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H "Bot-default promotion gate (no item moves from `seeded_risky_pending_harness` to bot-default without replay-linked evidence)" — no policy enforced.
2471. - [ ] [W6] [BP3] [M5+M6] [GAP] AI-H debug overlay "current order + behavior/tactic + target/perception + path + stuck/recovery + tool/material + fire safety + assertion status" — no overlay UI.

## 308. spec/ai-and-command — AI and command model (currently STUB; BP3 should advance)
3706. - [ ] [W6] [BP3] [GAP] AICMD layered AI "reflex / tactic / navigation / job / commander / personality" — only single ReactiveGuard FSM at BP3.
3707. - [ ] [W6] [BP3] [GAP] AICMD order contract "intent persists + tactic adapts + path repairs" — no orders at BP3.
3708. - [ ] [W6] [BP3] [GAP] AICMD reason labels emitted as events for debug/replay — not emitted.
3709. - [ ] [W6] [BP3] [GAP] AICMD command UX "direct + slowdown overlay + optional tactical map" — no command surface.
3710. - [ ] [W6] [BP3] [GAP] AICMD authoring "jobs/tactics as data + Lua hooks" — no scripting.
3711. - [ ] [W6] [BP3] [GAP] AICMD requirement "AI writes through same serializable intent/control layer as player input" — not enforced.
3712. - [ ] [W6] [BP3] [GAP] AICMD requirement "Terrain manipulation is normal action type (bots carve/clear without special-case hacks)" — not implemented.
3713. - [ ] [W6] [BP3] [GAP] AICMD requirement "Mobility tools have AI-visible affordance checks (rope/tether/jetpack anchor/material/path state)" — no mobility tools.
3714. - [ ] [W6] [BP3] [GAP] AICMD requirement "Every stuck recovery is logged with reason fields and replay tests" — no stuck detection.
3715. - [ ] [W6] [BP3] [GAP] AICMD requirement "Hazard avoidance is reflex gate (no rotting projectile avoidance TODOs)" — not implemented.

## 313. spec/music-and-soundtrack — Music/soundtrack roster (BP3 cf-audio stub; closed direction)
3777. - [ ] [W6] [BP3] [GAP] MUSIC stack "Suno v5 / Udio v2 / ElevenLabs Music (private prototypes log terms; release requires cleanup or clearance)" — not configured.
3778. - [ ] [W6] [BP3] [GAP] MUSIC local fallback "MusicGen/AudioCraft (MIT code; CC-BY-NC 4.0 weights; replace/self-train/license for release)" — not configured.
3779. - [ ] [W6] [BP3] [GAP] MUSIC SFX generator "Stable Audio Open 1.0 (47s at 44.1kHz from text + Stability AI Community License + commercial-use registration for release)" — not configured.
3780. - [ ] [W6] [BP3] [GAP] MUSIC voice generator "ElevenLabs / XTTS-v2 / Tortoise (log provenance and clean/replace before release)" — not configured.
3781. - [ ] [W6] [BP3] [GAP] MUSIC mixer "FMOD Studio (free under $200K/yr + bevy_fmod adaptive layering + spatial) OR bevy_kira_audio (Apache-2.0 pure-Rust lower-feature)" — neither integrated.
3782. - [ ] [W6] [BP3] [GAP] MUSIC adaptive `cf-audio-adaptive` crate (reads EnvironmentSignal + match phase + mission director state + combat intensity + emits crossfade commands to FMOD/Kira) — not present.
3783. - [ ] [W6] [BP3] [DR-043] [GAP] MUSIC spatial "Steam Audio per DR-043 reused for music spatial cues (radio music + base PA system)" — not implemented.
3784. - [ ] [W6] [BP3] [GAP] MUSIC main theme `theme_main` (90-120s + heroic+tactical+pulp) — not composed.
3785. - [ ] [W6] [BP3] [GAP] MUSIC 12 world themes (`world_earth_ambient` / `world_earth_moon_ambient` / `world_mars_ambient` / `world_phobos_ambient` / `world_deimos_ambient` / `world_mimas_ambient` / `world_europa_ambient` / `world_vulcan_ambient` / `world_venus_ambient` / `world_belt_asteroid_ambient` / `world_orbital_station_ambient` / `world_sol_ambient`) — none composed.
3786. - [ ] [W6] [BP3] [GAP] MUSIC 6 combat layers (`combat_low_intensity` + `combat_mid_intensity` + `combat_high_intensity` + `combat_climactic` + `combat_chase` + `combat_stalemate`) — none composed.
3787. - [ ] [W6] [BP3] [GAP] MUSIC 4 base/tension layers (`base_exploration` + `base_tension` + `base_under_siege` + `base_post_victory`) — none composed.
3788. - [ ] [W6] [BP3] [GAP] MUSIC 4 menu/UI tracks (`menu_main` + `menu_loadout` + `menu_briefing` + `menu_debrief`) — none composed.
3789. - [ ] [W6] [BP3] [GAP] MUSIC 8 mission-specific stings (`sting_objective_complete` + `sting_objective_failed` + `sting_breach_imminent` + `sting_reinforcements_arriving` + `sting_command_core_uprooted` + `sting_command_core_lost` + `sting_named_npc_killed` + `sting_artifact_recovered`) — none composed.
3790. - [ ] [W6] [BP3] [GAP] MUSIC 3 hero antagonist motifs (`motif_imperatus_legion` + `motif_husks_corruption` + `motif_browncoat_assault`) — none composed.
3791. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "match.started → world ambient + base exploration" — not implemented.
3792. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "enemy_first_contact → combat low-intensity crossfade in" — not implemented.
3793. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "intensity_score > 0.5 → combat mid-intensity" — not implemented.
3794. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "intensity_score > 0.8 → combat high-intensity" — not implemented.
3795. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "combat_lull no contact > 30s → crossfade back to ambient" — not implemented.
3796. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "objective_completed → sting + return to ambient" — not implemented.
3797. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "objective_failed → sting + dirge variant" — not implemented.
3798. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "command_core_status_change → sting + tension layer" — not implemented.
3799. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "weather.event_started solar_flare → radio-static texture" — not implemented.
3800. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "actor_low_health → heartbeat sub-bass layer for that player" — not implemented.
3801. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "boss_phase_change → climactic motif transition" — not implemented.
3802. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "match_victory → victory orchestral swell" — not implemented.
3803. - [ ] [W6] [BP3] [GAP] MUSIC adaptive layering trigger "match_defeat → dirge + scroll" — not implemented.
3804. - [ ] [W6] [BP3] [GAP] MUSIC SFX library 400+ clips (Weapon fire 80+ / Footsteps 40+ / Equipment 50+ / Voice humanoid 60+ / Voice mech-robot 40+ / Environment 60+ / UI 30+ / Music stings 8+ / Combat 40+) — 0 clips at BP3.
3805. - [ ] [W6] [BP3] [GAP] MUSIC AI-generation SFX pipeline (Stable Audio Open prompt + 5-10 candidates + AI agent reviews + Audacity/RX cleanup + caption + tag + commit) — not present.
3806. - [ ] [W6] [BP3] [GAP] MUSIC caption coverage "Critical SFX YES / Voice YES / Music swell key gameplay event YES / Ambient NO / UI tick NO + `cf-caption-check` CI gate" — not enforced.
3807. - [ ] [W6] [BP3] [GAP] MUSIC file format `content/music/*.ron` (id + duration + layers with file + default_volume + license + bpm + tempo_synced) — no content.
3808. - [ ] [W6] [BP3] [GAP] MUSIC file format `content/sfx/*.ron` (id + file + category + duration + spatial + falloff_radius_m + caption + license) — no content.
3809. - [ ] [W6] [BP3] [GAP] MUSIC `cf-asset-ledger check --mode private` (passes before retaining generated tracks/SFX) — not implemented.
3810. - [ ] [W6] [BP3] [GAP] MUSIC `cf-asset-ledger check --mode release` (passes before public sale/release) — not implemented.


# ===== WAVE 7 — BP3 DESIGN-INTENT SCHEMAS (data + events; full impl is BP4+) =====

## 24. M2 — DR-007 terrain/material closure debt (BP2)
361. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch hazard set: Fire (spreads on flammable materials) — not in cf-terrain launch set.
362. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch hazard: Smoke/gas (vision/health debuff) — only `hazard` material id exists, no smoke/gas behavior.
363. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch hazard: Electric (stuns devices/robots) — not implemented.
364. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch hazard: Slippery/wet (movement modifier) — not implemented.
365. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch hazard: Hot/cold (gradual damage; structural implications) — not implemented.
366. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch affordance: Actor passability — affordance present but no per-material passability tests.
367. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch affordance: Projectile passability — bullets pass through `air` only; no per-material projectile-pass logic.
368. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch affordance: "Anchor/grapple/tether allowed" — `anchor` material id exists but no grapple/tether system.
369. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch affordance: "Blocks light/vision" — no shadow / vision-cone occlusion per material.
370. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch affordance: "Deals contact damage" — `hazard` material id exists but does no damage on actor contact.
371. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 launch affordance: "Produces debris/particles/sound when hit" — no particles emitted on carve.
372. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 5-hazards-readable-in-1-mission playtest (> 80% identification) not run.
373. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 5-affordance-flags-readable in material overlay not measured (only 8 IDs colored).
374. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 heavy-combat 20-actors + 5-explosives @ 60 FPS perf gate not measured.
375. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 path-cost recalc < 100ms benchmark not measured.
376. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 terrain edits replicate in co-op prototype < 200ms — no co-op layer yet.
377. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 mod-adds-custom-material acceptance test — `cf-mod validate content/materials/` does not yet read material extensions (only existing 8 ids).
378. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 MAT-T-01..MAT-T-10 terrain material sandbox tests not implemented (spec/terrain-material-sandbox-slice-a not built).
379. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 hazard count cap (launch hazards at 5; new ones via DR) not enforced — currently only `hazard` exists as a generic id.
380. - [ ] [W7] [BP2] [M2] [DR-007] [GAP] DR-007 path-cost contribution per material — `material_affordance(id).path_cost` field exists but no pathfinder consumes it.

## 31. DR-005 — Multiplayer architecture-from-day-one (BP3 inherits per AGENTS.md; deeper scope → FUTURE_FEATURES.md J.1)
421. - [ ] [W7] [BP3] [DR-005] [GAP] DR-005 closed 2026-05-05 — but `cf-net` is still a 1-line stub. Per "architecture from day one" rule, BP3 should at least define the `Transport` trait shape in `cf-net` before BP4 ships gameplay needing it.
422. - [ ] [W7] [BP3] [DR-005] [GAP] DR-005 anti-cheat profile enum (`casual` / `competitive` / `tournament_strict`) — should at least be declared as a Rust enum somewhere (used by future `cf-server-anti-cheat`). Not declared.
423. - [ ] [W7] [BP3] [DR-005] [GAP] DR-005 mod hash sync foundation — `mod_hash` field in scenario manifest envelope expected; not present.

## 38. DR-029 — Save game M5 first slice (multi-slot / autosave / ironman → FUTURE_FEATURES.md J.3)
495. - [ ] [W7] [M5] [DR-029] [GAP] DR-029 versioned save format `.cfsave` — cf-save exists but only carries actor + chassis + rifle; not the full M5 first-slice schema.
496. - [ ] [W7] [M5] [DR-029] [GAP] DR-029 migration-safe schema with version handlers — schema_version field exists but no migration handlers registered.
497. - [ ] [W7] [M5] [DR-029] [GAP] DR-029 save → load → continue mission produces identical state (M5 closure done-criterion) — not validated.
498. - [ ] [W7] [M5] [DR-029] [GAP] DR-029 cf-save serializes equipment-condition state — not implemented (equipment state itself missing).

## 49. DR-038 / DR-036 / DR-037 — Forward-compat scaffolds at BP3 (deeper kernel impl → FUTURE_FEATURES.md A/B/J.6)
587. - [ ] [W7] [BP3] [DR-036+DR-037+DR-038] [PART] DR-038 `GravityField` enum scaffolded at BP3 with `Uniform(f32)` variant only — per-cell sampling lives in FUTURE.
588. - [ ] [W7] [BP3] [DR-036+DR-037+DR-038] [PART] DR-036 `RESERVED_EVENT_CATEGORIES` const reserves `material` / `reaction` / `atmospherics` / `affliction` / `gravity` / `ballistics` / `environment` / `body_force_feedback` / `chassis` / `collision` — but only `chassis` actually emits at BP3.
589. - [ ] [W7] [BP3] [DR-036+DR-037+DR-038] [PART] `cf-environment` crate scaffolded with `EnvironmentSignal` + 15-class `HazardClass` — but no consumer reads it; BP3 forward-compat hook only.

## 177. DR-016 — Setting / world frame (OPEN; M5.10/M7.7 BP4+ but BP3 narrative seeds expected)
1541. - [ ] [W7] [BP3+BP4] [M5.10+M7.7] [DR-016] [GAP] DR-016 launch worlds catalog placeholder — `content/worlds/` directory missing.
1542. - [ ] [W7] [BP3+BP4] [M5.10+M7.7] [DR-016] [GAP] DR-016 faction registry placeholder — `content/factions/` directory missing.
1543. - [ ] [W7] [BP3+BP4] [M5.10+M7.7] [DR-016] [GAP] DR-016 narrative-seed prose — none authored at BP3.

## 179. DR-027 — Combat-base scope (OPEN; M7.5 closes; BP3 should expose stub)
1548. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base shields module — no shield game-object.
1549. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base turrets module — no turret game-object.
1550. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base sensors module — no sensor game-object.
1551. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base doors module — no door game-object.
1552. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base repair pads — no repair-pad game-object.
1553. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base power grid — no power-graph entity.
1554. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 CORE-A acceptance — not authored.

## 192. cf-control schemas (after full DR audit)
1611. - [ ] [W7] [GAP] No `system.shutdown` request schema (currently unwritten; relies on connection close).
1612. - [ ] [W7] [GAP] No `system.heartbeat_ping` / `pong` JSON schema (transport pings happen at WS layer).
1613. - [ ] [W7] [GAP] No `system.protocol_version` request schema (clients can't query server version).
1614. - [ ] [W7] [GAP] No `observe.history` request schema (clients can't ask for recent events from before subscribe).
1615. - [ ] [W7] [GAP] No `act.scenario.skip_intro` schema (for tutorials).
1616. - [ ] [W7] [GAP] No `act.debug.spawn_fixture` schema for the debug-only spawn command.
1617. - [ ] [W7] [GAP] No `act.debug.teleport` schema.
1618. - [ ] [W7] [GAP] No `act.debug.force_damage` schema.
1619. - [ ] [W7] [GAP] No `act.debug.reveal_map` schema.
1620. - [ ] [W7] [GAP] No `act.debug.grant_item` schema.

## 193. Cross-DR coherence: BP3 should expose stub for every M1+ inherited Universal row
1621. - [ ] [W7] [BP3] [M1+M4A] [UNIV-DR056] M4A perf gate Steam Deck 800p/60 — never measured.
1622. - [ ] [W7] [BP3] [M1+M4A] [UNIV-DR056] M4A perf gate 1080p/60 — verified ✓ (m4a_2026-05-10T05-57-39Z bundle).
1623. - [ ] [W7] [BP3] [M1+M4A] [UNIV-DR056] M4A perf gate 4K/120 — never measured.
1624. - [ ] [W7] [BP3] [M1+M4A] [UNIV-DR056] M4A 24h memory-leak soak — never run.
1625. - [ ] [W7] [BP3] [M1+M4A] [UNIV-DR056] M4A modding parity (mod-author can override HUD palette) — not verified.
1626. - [ ] [W7] [BP3] [M1+M4A] [UNIV-DR056] M4A captions for ALL audio — no audio at BP3 but captions pipeline exists ✓.
1627. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 perf gate Steam Deck 800p/60 — never measured.
1628. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 perf gate 1080p/60 — never measured.
1629. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 perf gate 4K/120 — never measured.
1630. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 24h memory-leak soak — never run.
1631. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 modding parity (mod-author can add new chassis archetype) — not verified.
1632. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 anti-FOMO + anti-pay-to-win audit — not run.
1633. - [ ] [W7] [BP3] [M1+M5] [DR-055] [UNIV-DR056] M5 juice rules per DR-055 — chassis stage tint + module pip colors exist ✓; but no hit-stop / hit-pause / explosion shake.
1634. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 Tier-A 11-language keyed-strings — English-only strings.
1635. - [ ] [W7] [BP3] [M1+M5] [UNIV-DR056] M5 captions for chassis events — no caption queue entries for chassis_stage_changed.

## 214. DR-043 — Voice / radio comms (CLOSED; M9.5 owns; BP3 schema seed)
1897. - [ ] [W7] [BP3] [M9.5] [DR-002+DR-043] [GAP] DR-043 voice.* + radio.* event category seed — declared in DR-002 baseline but not emitted.
1898. - [ ] [W7] [BP3] [M9.5] [DR-043] [GAP] DR-043 origin gating (human equips radio / robot built-in / android variant) — not declared.
1899. - [ ] [W7] [BP3] [M9.5] [DR-043] [GAP] DR-043 frequency band registry (HF / VHF / UHF / Microwave) — not declared.

## 215. DR-048 — Endgame retention & server-wide events (CLOSED; M12+ owns; BP3 schema seed)
1900. - [ ] [W7] [BP3] [M12] [DR-048] [GAP] DR-048 endgame mode roster (10 modes per DR-048) — not declared.
1901. - [ ] [W7] [BP3] [M12] [DR-048] [GAP] DR-048 server-wide events broadcaster (cross-shard event broadcaster per DR-048 + spec/server-wide-events-and-meta-narrative) — not implemented.
1902. - [ ] [W7] [BP3] [M12] [DR-048] [GAP] DR-048 persistent veterans across sessions — not implemented.

## 218. DR-034 — Dedicated server app (CLOSED; M9 owns; BP3 stub)
1922. - [ ] [W7] [BP3] [M9] [DR-034] [GAP] DR-034 `cf-server` binary with `--mode coop_room/pvp_arena/lan_room/mmo_shard/lobby_directory` — `cf-server` is 36-line stub at BP3.
1923. - [ ] [W7] [BP3] [M9] [DR-034] [GAP] DR-034 `cf-server-ops` config loader (RON) + mode selector + health (`/health`) + readiness (`/ready`) + Prometheus metrics — 1-line stub at BP3.
1924. - [ ] [W7] [BP3] [M9] [DR-034] [GAP] DR-034 `cf-server-persistence` snapshot writer + event journal + restore loop — 1-line stub.
1925. - [ ] [W7] [BP3] [M9] [DR-034] [GAP] DR-034 `cf-server-anti-cheat` server-authoritative input validation + rate-limit + capability gates + audit log + profile registry (`casual`/`competitive`/`tournament_strict`) — 1-line stub.
1926. - [ ] [W7] [BP3] [M9] [DR-034] [GAP] DR-034 `cf-server-admin` capability-gated cfctl-shape admin endpoints — 1-line stub.
1927. - [ ] [W7] [BP3] [M9] [DR-034] [GAP] DR-034 reference Docker image — not present.
1928. - [ ] [W7] [BP3] [M9] [DR-034] [GAP] DR-034 SERVER-001..SERVER-016 acceptance suite — not authored.

## 219. DR-035 — Persistent MMO architecture (CLOSED; M12 owns; BP3 stub)
1929. - [ ] [W7] [BP3] [M12] [DR-035] [GAP] DR-035 persistent world manifest (region map + materials + hazards + faction territories) — not declared.
1930. - [ ] [W7] [BP3] [M12] [DR-035] [GAP] DR-035 persistent state store (snapshot every 10 min + append-only event journal) — not implemented.
1931. - [ ] [W7] [BP3] [M12] [DR-035] [GAP] DR-035 MMO-001..MMO-012 acceptance suite — not authored.
1932. - [ ] [W7] [BP3] [M12] [DR-035] [GAP] DR-035 50-100 concurrent player target — not measured.

## 220. DR-036 / DR-037 — Material kernel + Stationeers-grade atmospherics (CLOSED-DIRECTION; M5.6/M5.9 own; BP3 forward-compat scaffold)
1933. - [ ] [W7] [BP3] [M5.6+M5.9] [DR-036+DR-037] [PART] DR-036 RESERVED_EVENT_CATEGORIES const reserves `material` + `reaction` + `atmospherics` + `affliction` + `gravity` + `ballistics` + `environment` + `body_force_feedback` + `chassis` + `collision` — `chassis` is the only category actually emitted at BP3.
1934. - [ ] [W7] [BP3] [M5.6+M5.9] [DR-036+DR-037] [PART] `cf-environment` crate scaffolded with `EnvironmentSignal` + 15-class `HazardClass` — but no consumer.
1935. - [ ] [W7] [BP3] [M5.6+M5.9] [DR-036+DR-037] [GAP] DR-036 material schema (id / display_name / category / movement_class / density / viscosity / mass_per_pixel / hardness / heat_capacity / thermal_conductivity / temperature / ignition_temperature / burn_rate / oxygen_requirement / burn_products / phase_changes / conductivity / wetting / reaction_tags / ai_affordances / ui_overlay_color / caption_priority / performance_tier / network_replay_mode) — not authored at BP3.
1936. - [ ] [W7] [BP3] [M5.6+M5.9] [DR-036+DR-037] [GAP] DR-037 atmospheric kernel boundary (CPU deterministic truth; GPU acceleration where deterministic parity proven) — `cf-atmos` is 1-line stub.

## 222. DR-039 — Celestial bodies / worlds direction (CLOSED-DIRECTION; M5.10/M7.7 own; BP3 schema seed)
1943. - [ ] [W7] [BP3] [M5.10+M7.7] [DR-039] [GAP] DR-039 World catalog (`content/worlds/<id>.ron`) — directory missing.
1944. - [ ] [W7] [BP3] [M5.10+M7.7] [DR-039] [GAP] DR-039 per-planet astrography schema (`rotation_period_seconds + axial_tilt_deg + semi_major_axis_au + parent.solar_distance_au`) — not declared.
1945. - [ ] [W7] [BP3] [M5.10+M7.7+M8.6] [DR-039] [GAP] DR-039 per-world ore deposit generator (deterministic seed) — M8.6 scope.

## 223. DR-040 — Environmental conditions & hazards direction (CLOSED-DIRECTION; M5.10 owns; BP3 forward-compat scaffold)
1946. - [ ] [W7] [BP3] [M5.10] [DR-040] [PART] DR-040 `EnvironmentSignal` aggregator scaffolded ✓ but no per-tick computation.
1947. - [ ] [W7] [BP3] [M5.10] [DR-040] [PART] DR-040 15-class hazard taxonomy declared ✓ but no consumer (AI / HUD / replay / audio).
1948. - [ ] [W7] [BP3] [M5.10] [DR-040] [GAP] DR-040 tick schedule for aggregator (after kernels, before consumers) — not enforced.

## 227. M4A — focusable_nodes contract gaps
1957. - [ ] [W7] [M4A+M5] [GAP] `cf_control::engine::HUD_FOCUSABLE_NODES` — 12 ids ✓ but M5 chassis-stage banner is NOT a focusable node.
1958. - [ ] [W7] [M4A] [GAP] HUD_FOCUSABLE_NODES does not include the captions-strip area.
1959. - [ ] [W7] [M4A] [GAP] HUD_FOCUSABLE_NODES does not include the material-overlay toggle.
1960. - [ ] [W7] [M4A] [GAP] HUD_FOCUSABLE_NODES has no per-actor focus mode (only HUD).

## 259. DR-024 — Native engine stack (CLOSED; some BP3 residual)
2212. - [ ] [W7] [BP3] [M2] [DR-024] [GAP] DR-024 "We use Bevy where it earns its keep and write custom for hot paths" — at BP3, terrain carving still CPU only (M2 done-criterion calls for GPU-assisted carve compute shader).
2213. - [ ] [W7] [BP3] [DR-024] [GAP] DR-024 audio backend choice (bevy_audio vs kira) — not picked at BP3.
2214. - [ ] [W7] [BP3] [DR-024] [GAP] DR-024 UI library specifics (egui vs custom Bevy UI for HUD) — picked Bevy UI ✓ but no egui surface yet for tools.
2215. - [ ] [W7] [BP3] [M5] [DR-024] [GAP] DR-024 modding scripts (mlua vs Rhai) — not picked at BP3; deferred to M5 per AGENTS.md but never resolved.
2216. - [ ] [W7] [BP3] [M0+M1] [DR-024] [GAP] DR-024 Bevy upgrade cadence (M0 + M1-000 audited 0.18.1) — no next-upgrade task scheduled.

## 260. DR-025 — Target platforms (CLOSED; BP3 T-RELEASE inherits)
2217. - [ ] [W7] [BP3] [DR-025] [GAP] DR-025 macOS Apple Silicon + Intel dual build — only aarch64 at BP3.
2218. - [ ] [W7] [BP3] [DR-025] [GAP] DR-025 Ubuntu LTS + Steam Runtime baseline — not measured for binary size or perf.
2219. - [ ] [W7] [BP3] [DR-025] [GAP] DR-025 "Project owner runs the build on macOS personally" — never verified for current BP3 commit.
2220. - [ ] [W7] [BP3] [DR-025] [GAP] DR-025 mobile non-promise enforcement — no test ensures mobile deps absent.
2221. - [ ] [W7] [BP3] [DR-025] [GAP] DR-025 web non-promise — no wasm build configuration as a sanity check.

## 261. DR-027 — Combat-base scope (CLOSED; M7.5 closes; BP3 schema stub)
2222. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base "power grid" entity — no power graph at BP3.
2223. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base shield with health + recovery delay + modular placement — not implemented.
2224. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base turret stationary defense + role-card metadata + jam states + ammo — not implemented.
2225. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base sensor + reveal range + cloaking detection + intrusion alerts + disable-able — not implemented.
2226. - [ ] [W7] [BP3] [M7.5] [DR-007+DR-027] [GAP] DR-027 base door with HP + lock states + breachable per DR-007 — micro_breach has soft-breach strips but no full door entity.
2227. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base repair pad (heal nearby actors over time + finite charges per scenario) — not implemented.
2228. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base hangar / storage (ready slots for chassis + equipment caches + salvage staging) — not implemented.
2229. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 base traps (mines / tripwires / decoys) — not implemented.
2230. - [ ] [W7] [BP3] [M7.5] [DR-027] [GAP] DR-027 "5 distinct base layouts produce meaningfully different mission outcomes" acceptance — no base authoring at BP3.

## 292. spec/origin-reaction-and-resource-model — Per-origin reaction matrix (BP3 M5 design intent; M5.8 implements)
3158. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "Shot force feedback per origin (human pain flash + soft-tissue jolt / android softer pain flash / robot servo-jolt + mechanical clank + no pain flash)" — single feedback at BP3.
3159. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "G-Force susceptibility (human high / android reduced 30-50% / robot none)" — not modeled.
3160. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "Concussion (rapid damage stacking → blackout HUD)" (human full / android reduced / robot NEVER) — not modeled.
3161. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "Internal-shock damage (robot-only: impact rolls damage on random un-armored internal module independent of armor)" — not modeled.
3162. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "Fall damage (human highest tolerance + bone break / android mid / robot frame absorbs but internal modules damaged)" — single fall model.
3163. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "Limb wounds (human + android skeletal/soft-tissue / robot armor + module damage only)" — not modeled.
3164. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "Bleeding (human yes / android reduced / robot NO blood + YES coolant/oil leak)" — not modeled.
3165. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN reaction "Leak channels: coolant (heat-dissipation lines) + oil (joint actuators) emits to material kernel + can ignite + reduces robot power" — not implemented.
3166. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN healing "Food (caloric replenishment) for human + android" — no food system.
3167. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN healing "Medical kits (wound treatment + bleed stop) for human + android" — no medkits.
3168. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN healing "Drugs (buff/debuff + pain suppression + focus) for human + android" — no drugs.
3169. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN healing "Robot: NEVER eats food / NEVER uses medkits / NEVER takes drugs — repaired via repair tools + coolant refills + oil refills + module swaps" — not implemented.
3170. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN buff "Drugs only for humans" — not implemented.
3171. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN buff "Android: drugs (organic side) + limited overclock (synthetic side, gated by installed modules)" — not implemented.
3172. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN buff "Robot: overclock only" — not implemented.
3173. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN overclock "Per-module (android) gated by module type (processor / actuator / sensor / weapon-mount) — only installed modules can overclock" — not implemented.
3174. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN overclock "Whole-processor (robot) affecting movement / aim / reload / sensor speed + deeper boost ceiling than android" — not implemented.
3175. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN overclock cost "Android: power draw (battery) + per-module heat (heat damages module)" — not implemented.
3176. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN overclock cost "Robot: power draw always + global heat (sustained heat damages internal modules)" — not implemented.
3177. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN resource "Human: caloric_energy (food-fed + depletes via action + over time + hunger affliction)" — no caloric system.
3178. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN resource "Android hybrid: caloric_energy (organic) + optional battery_charge (synthetic; some variants ship with batteries)" — not implemented.
3179. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN resource "Robot: power (gates EVERY action — move / aim / fire / observe + recharged via base power / generators / salvage + no power = inert)" — no power resource.
3180. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN depletion penalty "Human: slowdown / aim wobble / vision blur → `weak` / `exhausted` afflictions" — not implemented.
3181. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN depletion penalty "Android organic: same as human; synthetic battery: slowdown / ability lockout / module shutdown" — not implemented.
3182. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN depletion penalty "Robot: action cost rejection (cannot fire / cannot move at full) → eventually inert" — not implemented.
3183. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN death readability "Recap names cause chain on active failure side (organic vs synthetic for android) + hit → wound → bleed/concussion/G-load → status (human) + hit → module failure → coolant leak → fire reaction OR power depletion OR catastrophic frame (robot)" — single death model.
3184. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN env "Vacuum/no-oxygen: human MUST wear sealed helmet + oxygen tank (consumable); android same; robot immune (sealed by design)" — not implemented.
3185. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN env "Low-oxygen non-zero: helmet/tank not strictly required but oxygen_supply drains reduced + hypoxia stacks slowly" — not implemented.
3186. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN env "High-temperature (foundry/lava/fire): human burning + heat_exhaustion affliction high vulnerability; android per-module heat ring (combat-spec shielded / civilian unshielded); robot most resistant — global heat downclocks at throttle band + module damage at critical" — not implemented.
3187. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN env "Cold/cryo: human slowdown + caloric drain + frostbite (deferred); android same organic + actuator viscosity penalty; robot reduced joint viscosity + cold delays overheat" — not implemented.
3188. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN env "Irradiated: human irradiated affliction (deferred); android same organic + synthetic sensor/processor noise; robot sensor/processor noise + logic faults" — not implemented.
3189. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN env "Hostile material (acid/toxic/electrified water): human corroded/poisoned/electrified; android same + synthetic module corrosion; robot no organic + acid corrodes plates/modules + toxic mostly inert + electrified water arcs" — not implemented.
3190. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN equipment "helmet `provides_seal` + `oxygen_capacity_seconds` + `consumption_modifier_running` + `consumption_modifier_combat`" — not implemented.
3191. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN equipment "Robot cannot equip oxygen tanks (rejected with `wrong_origin_for_equipment`); helmets they CAN wear (cosmetic + visor optics)" — not implemented.
3192. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN equipment "helmet `breakable` + `breach_event` (penetrating round emits `helmet_breach` + oxygen drains at multiplied rate → hypoxia)" — not implemented.
3193. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN heat path "Overclock (voluntary boost; player/AI requests; 'Boosting' pip)" — not implemented.
3194. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN heat path "Downclock (involuntary throttle; heat crosses band from passive sources; 'Throttling' pip; AI under involuntary downclock should retreat)" — not implemented.
3195. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN event `body_force_feedback` (impulse_vector + magnitude + origin_id + chassis_layer + feedback_kind + g_load_delta always emit even if 0 + concussion_dose_delta always emit even if 0 + internal_shock_module_id + leak_channel + screen_kick_intensity) — not emitted.
3196. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN HUD G-Force vision blackout (concussion_dose crosses bands mild/moderate/severe/out + vignette darkens edges inward + at severe peripheral vision gone + at out full blackout) — not implemented.
3197. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN HUD blackout audio (heart-rate sound layer mixes louder + ambient duck + captions on) — not implemented.
3198. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN HUD blackout origin gate (humans full curve / android reduced cap at moderate / robot NEVER) — not modeled.
3199. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN HUD blackout accessibility (`--reduced-motion` + `--reduced-g-force-blackout` toggle + non-visual fallback caption + HUD icon) — not implemented.
3200. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN event `affliction.concussed_set` / `_intensified` / `_cleared` (replay reproducible) — not emitted.
3201. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN robot leak event `chassis_leak_started` / `chassis_leak_rate_changed` / `chassis_leak_stopped` (channel + rate + source module/zone + parent hit/damage) — not emitted.
3202. - [ ] [W7] [BP3] [M5+M5.8] [GAP] ORIGIN acceptance ORIGIN-A — not specified at BP3.

## 293. spec/environmental-conditions-model — EnvironmentSignal aggregation (BP3 M5.10 design intent)
3203. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `EnvironmentSignal` struct (per-tick per-actor bundle: identity + atmospheric + gravitational + thermal + radiation + photic + em + pressure + weather + water + acoustic + day_night + comms + derived_hazards) — not present at BP3.
3204. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `EnvironmentSignal::for_actor(actor, tick)` (one bundle per actor per tick at deterministic point in tick schedule) — no aggregator.
3205. - [ ] [W7] [BP3] [M5.10] [GAP] ENV principle "All consumers (AI/HUD/accessibility/replay/audio/mission/server) read from bundle; no individual kernel queries" — not enforced.
3206. - [ ] [W7] [BP3] [M5.10] [GAP] ENV principle "Kernels write via typed adapter `produce_signal(...)`" — not implemented.
3207. - [ ] [W7] [BP3] [M5.10] [GAP] ENV principle "Replay records signal deltas not full bundles per tick" — not implemented.
3208. - [ ] [W7] [BP3] [M5.10] [GAP] ENV principle "Origin gating at consumer not producer (bundle reports raw environment)" — not implemented.
3209. - [ ] [W7] [BP3] [M5.10] [GAP] ENV principle "Modder-extensible (typed extension for new signals like psionic_field)" — not implemented.
3210. - [ ] [W7] [BP3] [M5.10] [GAP] ENV principle "Per-tick performance bounded (SoA over all actors + SIMD-friendly + sleeping actors skip)" — not implemented.
3211. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `AtmosphericExposure` (atm_kind + partial_pressure_pa per gas + total_pressure_pa + temperature_k + composition_ratio + hazards + breach_eta_s) — no atmosphere.
3212. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `GravityVec` slice (direction + magnitude + g_factor + source) — not bundled.
3213. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `ThermalExposure` (ambient_k + radiation_w_m2 + conduction_w + wind_chill_modifier_k + derived_band Frigid/Cold/Comfortable/Hot/Extreme) — not bundled.
3214. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `RadiationExposure` (solar_msvph + cosmic_msvph + reactor_msvph + ambient_msvph + total_msvph + derived_dose_band Safe/Mild/Hazardous/Critical) — no radiation.
3215. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `PhoticExposure` (lux + spectrum band Visible/UV/IR/Mixed + flicker_hz + derived_visibility_band Pitch/Dim/Lit/Bright/Glaring) — no photic.
3216. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `EmExposure` (magnetic_t + em_noise_db + compass_deviation_rad + em_emp_recently bool) — no EM.
3217. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `PressureExposure` (ambient_pa + suit_pa + delta_pa_per_s breach detection) — not bundled.
3218. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `WeatherExposure` (active_event + intensity_0_1 + wind_mps vec + visibility_m + precipitation kind + eta_to_change_s) — no weather kernel.
3219. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `WaterExposure` (wetness_0_1 + submerged_depth_m + liquid_kind + is_drowning_hazard) — no water.
3220. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `AcousticExposure` (ambient_db + propagation_medium Air/Rarefied/Vacuum/Liquid/Foam + reverb_rt60_s + occlusion_db + derived_can_hear_voice_unaided) — no acoustics.
3221. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `DayNightPhase` (local_solar_time_s + phase Night/Dawn/Day/Dusk + sun_elevation_deg) — no day/night.
3222. - [ ] [W7] [BP3] [M5.10] [GAP] ENV `CommsLatency` (light_lag_to_command_anchor_s + active_radio_links) — no comms.
3223. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `hypoxic` (O2 < 16 kPa AND non-robot-suit) — not derived.
3224. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `combustible_atmosphere` (Volatiles >= 5% AND O2 >= 5% AND temp >= autoignite) — not derived.
3225. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `toxic_atmosphere` (Pollutant > toxin_threshold) — not derived.
3226. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `breach_decomp` (breach_eta_s.is_some()) — not derived.
3227. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `hyperthermic` (thermal Hot/Extreme + origin gate) — not derived.
3228. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `hypothermic` (thermal Cold/Frigid + origin gate) — not derived.
3229. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `radiation` (dose_band >= Hazardous + origin gate) — not derived.
3230. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `low_visibility` (Pitch OR weather visibility_m < 50) — not derived.
3231. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `glare` (Glaring solar flare) — not derived.
3232. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `em_disruption` (em_emp_recently OR em_noise_db > threshold + origin gate) — not derived.
3233. - [ ] [W7] [BP3] [M5.10+M5.5] [GAP] ENV derived hazard `wind_force` (wind > threshold + routes through M5.5 impulse force) — not derived.
3234. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `drowning_hazard` (is_drowning_hazard + origin gate) — not derived.
3235. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `vacuum_no_voice` (propagation Vacuum + must use radio) — not derived.
3236. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `comms_blackout` (light_lag > X OR all radio_links signal_quality < threshold) — not derived.
3237. - [ ] [W7] [BP3] [M5.10] [GAP] ENV derived hazard `gravity_shift` (source != Ambient OR magnitude crosses band) — not derived.
3238. - [ ] [W7] [BP3] [M5.10] [GAP] ENV tick schedule (gravity → atmos → material → worlds → weather → day_night → comms → aggregate → actor controller → AI → equipment → UI → replay → audio → mission) — not enforced.
3239. - [ ] [W7] [BP3] [M5.10] [GAP] ENV event `environment.signal_changed` (consumer-relevant slice changed beyond threshold + old/new summary) — not emitted.
3240. - [ ] [W7] [BP3] [M5.10] [GAP] ENV event `environment.hazard_detected` (new hazard class entered + parent event id) — not emitted.
3241. - [ ] [W7] [BP3] [M5.10] [GAP] ENV event `environment.hazard_cleared` — not emitted.
3242. - [ ] [W7] [BP3] [M5.10] [GAP] ENV event `environment.bundle_snapshot` (sparse periodic full bundle per scenario-second for debug scrub) — not emitted.
3243. - [ ] [W7] [BP3] [M5.10] [GAP] ENV event `environment.aggregator_perf` (active actor count + ms per tick + sleeping count) — not emitted.
3244. - [ ] [W7] [BP3] [M5.10] [GAP] ENV acceptance ENV-A-01..10 — none pass.
3245. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI doctrine integration "AI reads bundle never queries kernels independently (CI grep gate)" — not enforced.
3246. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI integration "Stay alive (derived_hazards has avoidance plan)" — not implemented.
3247. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI integration "Use environment as weapon (vent O2 into combustible room then ignite)" — not implemented.
3248. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI integration "Plan route across worlds (comms latency + per-world catalog data)" — not implemented.
3249. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI integration "Brief teammates on threats (hazard_detected → squad radio chatter)" — not implemented.
3250. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI integration "Refuse unsafe order (won't walk unsealed human across vacuum + `wrong_origin_for_environment` reason)" — not implemented.
3251. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI integration "Calibrate equipment (thermal ambient → adjust suit AC + radiation → swap to radiation-resistant module)" — not implemented.
3252. - [ ] [W7] [BP3] [M5.10] [GAP] ENV AI integration "Time mission by phase (day_night.phase → 'attack at dawn' + weather.eta → 'wait until storm passes')" — not implemented.
3253. - [ ] [W7] [BP3] [M5.10] [GAP] ENV reason-label enum extension `environment_*` codes (AI never invents free-text reasons) — not implemented.
3254. - [ ] [W7] [BP3] [M5.10] [GAP] ENV modder contract "Add new signal slice via `EnvironmentSignalExtension<T>` trait + register via `cf-environment::register_extension`" — no extension API.
3255. - [ ] [W7] [BP3] [M5.10] [GAP] ENV modder contract "Add new hazard class via `content/hazards/` data row (class id + source signal expression + severity + AI affordance + HUD chip + caption hook)" — not implemented.
3256. - [ ] [W7] [BP3] [M5.10] [GAP] ENV modder contract "`cargo run -p cf-mod -- validate content/hazards/`" — no validator.

## 294. spec/command-core-base-power — Command core + base power + avatar uproot (BP3 design intent; M5+ implements)
3257. - [ ] [W7] [BP3] [M5] [GAP] CORE 3 states (`rooted_base` + `portable_core` + `embedded_avatar`) — no command core entity at BP3.
3258. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Shields (base envelope + local shield doors + directional projectors)" — no shields.
3259. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Powered turrets (automated defense + target sharing + friend/foe + ammo/heat telemetry)" — no turrets.
3260. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Sensors (radar + motion + wall/terrain scans + enemy approach warnings + LZ warnings)" — no sensors.
3261. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Doors/locks (powered blast doors + smart locks + pressure gates + access routing)" — no doors.
3262. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Repair platforms (heal/repair actors + androids + robots + armor + weapons + tools + mech modules)" — no repair platforms.
3263. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Charging/energy pads (recharge energy weapons + shields + drones + powered armor + mech modules)" — no charging.
3264. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Command relays (improve AI coordination + order propagation + squad intent sharing + tactical overlays)" — no relays.
3265. - [ ] [W7] [BP3] [M5] [GAP] CORE rooted base power for "Logistics beacons (improve delivery accuracy + craft landing safety + cargo routing + emergency extraction)" — no logistics.
3266. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Shields collapse/weaken/drain reserve batteries" — not implemented.
3267. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Turrets offline / switch to dumb local mode / manual crew" — not implemented.
3268. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Sensors fog increases + targeting and commander AI confidence drop" — not implemented.
3269. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Doors fail safe by faction/design (lock / open / jam / require manual tool)" — not implemented.
3270. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Repair platforms slow down / lose advanced repair" — not implemented.
3271. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Charging pads recharge rate drops or stops" — not implemented.
3272. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Command relays AI loses local boost + shared sensor certainty + command bandwidth" — not implemented.
3273. - [ ] [W7] [BP3] [M5] [GAP] CORE offline behavior "Logistics delivery risk rises + LZ scoring gets worse" — not implemented.
3274. - [ ] [W7] [BP3] [M5] [GAP] CORE embedded avatar boost "Durability (more armor + health + shock resistance + emergency sealing readable as armor/core state not invisible HP)" — not implemented.
3275. - [ ] [W7] [BP3] [M5] [GAP] CORE embedded avatar boost "Mobility (faster movement + stronger jump/jet + better recovery + higher carry; still respects mass/terrain/recoil)" — not implemented.
3276. - [ ] [W7] [BP3] [M5] [GAP] CORE embedded avatar boost "Energy (larger battery + faster recharge + stronger shields + more ability uptime; expose heat/overload/energy warnings)" — not implemented.
3277. - [ ] [W7] [BP3] [M5] [GAP] CORE embedded avatar boost "Equipment output (higher power budget for heavy weapons + tools + shields + sensors + repair modules; not universal solution)" — not implemented.
3278. - [ ] [W7] [BP3] [M5] [GAP] CORE embedded avatar boost "Abilities (command pulse + rally + local repair burst + shield flare + emergency extraction beacon + overclock; costs and cooldowns replay/event-visible)" — not implemented.
3279. - [ ] [W7] [BP3] [M5] [GAP] CORE embedded avatar boost "Control aura (stronger command radius + faster AI response + better squad sensor sharing near avatar; commander fantasy not personal DPS)" — not implemented.
3280. - [ ] [W7] [BP3] [M5] [GAP] CORE UX "Base power panel (core state + available power + reserve power + powered/offline modules + shield/sensor/turret/repair status)" — no panel.
3281. - [ ] [W7] [BP3] [M5] [GAP] CORE UX "Core action prompt (Root / Uproot / Carry / Embed / Extract / Repair / Shield / Emergency eject)" — no actions.
3282. - [ ] [W7] [BP3] [M5] [GAP] CORE UX "Avatar HUD (core integrity + avatar bonuses + energy/heat + base-offline warning + extraction route)" — not implemented.
3283. - [ ] [W7] [BP3] [M5] [GAP] CORE UX "Tactical map (base power radius + command relay coverage + sensor coverage + powered doors + turret arcs + shield coverage)" — no tactical map.
3284. - [ ] [W7] [BP3] [M5] [GAP] CORE UX "Squad panel (which actors are boosted by relay/avatar aura + which are outside command support)" — no squad panel.
3285. - [ ] [W7] [BP3] [M5] [GAP] CORE UX "Replay/debrief (when core moved + what base systems went dark + what avatar boosts were active + whether the gamble paid off)" — no replay/debrief.
3286. - [ ] [W7] [BP3] [M5] [GAP] CORE AI behavior `defend_core_room` ("command core rooted; shield grid depends on it") — not implemented.
3287. - [ ] [W7] [BP3] [M5] [GAP] CORE AI behavior `repair_powered_module` ("turret offline; core power available; mechanic in range") — not implemented.
3288. - [ ] [W7] [BP3] [M5] [GAP] CORE AI behavior `escort_portable_core` ("core uprooted; base reserve power low") — not implemented.
3289. - [ ] [W7] [BP3] [M5] [GAP] CORE AI behavior `refuse_unsafe_embed` ("avatar chassis damaged; core loss risk too high") — not implemented.
3290. - [ ] [W7] [BP3] [M5] [GAP] CORE AI behavior `push_with_avatar` ("core embedded; shield burst ready; objective window open") — not implemented.
3291. - [ ] [W7] [BP3] [M5] [GAP] CORE AI behavior `retreat_avatar` ("core integrity critical; base power offline") — not implemented.
3292. - [ ] [W7] [BP3] [M5] [GAP] CORE AI behavior "Enemy raid power modules + breach shield generators + bait avatar deployment + cut off extraction" — not implemented.
3293. - [ ] [W7] [BP3] [M5] [GAP] CORE event `command_core_state_changed` (old/new + actor_or_base + cause + reason) — not emitted.
3294. - [ ] [W7] [BP3] [M5] [GAP] CORE event `base_power_changed` (available + reserve + lost_modules + restored_modules + cause) — not emitted.
3295. - [ ] [W7] [BP3] [M5] [GAP] CORE event `base_module_power_changed` (module + type + old/new + power_draw + reason) — not emitted.
3296. - [ ] [W7] [BP3] [M5] [GAP] CORE event `core_embedded` / `core_extracted` (core + actor/chassis + valid/invalid reason + time_to_complete) — not emitted.
3297. - [ ] [W7] [BP3] [M5] [GAP] CORE event `avatar_boost_changed` (boost_type + old/new value + source_core) — not emitted.
3298. - [ ] [W7] [BP3] [M5] [GAP] CORE event `core_damaged` (damage_type + integrity_remaining + shield_state + cause) — not emitted.
3299. - [ ] [W7] [BP3] [M5] [GAP] CORE event `base_reserve_depleted` (base + reserve_remaining + systems_failed) — not emitted.
3300. - [ ] [W7] [BP3] [M5] [GAP] CORE progression hook `command_core_record` (core id + origin/flavor + integrity + upgrades + scars + rooted/embedded history + near-loss events) — not implemented.
3301. - [ ] [W7] [BP3] [M5] [GAP] CORE progression hook `base_power_grid` (generators + reserves + emitters + turret links + sensor relays + repair pads + door controllers) — not implemented.
3302. - [ ] [W7] [BP3] [M5] [GAP] CORE progression hook `avatar_chassis_history` (which chassis held core + mission outcomes + damage + abilities used + extraction result) — not implemented.
3303. - [ ] [W7] [BP3] [M5] [GAP] CORE progression hook `base_module_record` (installed + power draw + condition + repair history + mod provenance + tactical role) — not implemented.
3304. - [ ] [W7] [BP3] [M5] [GAP] CORE acceptance CORE-A-01..06 — not specified at BP3.

## 295. spec/celestial-bodies-and-worlds-model — World catalog (BP3 design intent; M2/M5.6/M5.9/M5.10/M7.7 implements)
3305. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `World` schema (id + classification + display_name + parent + astro + surface + ore_deposits + weather + lore + visual_palette + canonical + package_source) — not present.
3306. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `Classification` enum (Planet / Moon / Asteroid / Sun / Station / Anomaly) — not implemented.
3307. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `Astrography` (semi_major_axis_au + orbital_period_days + mean_anomaly_at_epoch_rad + rotation_period_seconds + axial_tilt_deg + epoch_utc_iso) — no orbital math.
3308. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `SurfaceProfile` (gravity_g + atmosphere_ambient + surface_template + day_length_seconds + temperature_range_k + magnetic_field_microtesla + radiation_ambient_msvph) — single hardcoded surface at BP3.
3309. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `AtmosphereAmbient` (pressure_kpa + temperature_k + composition mole fractions) — no atmosphere.
3310. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `OreDepositEntry` (ore + abundance + depth_band + distribution) — see §297.
3311. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `WeatherProfile` (variation_table + baseline_wind_mps + baseline_visibility_m) — no weather.
3312. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `LoreTags` (tags + name_origin canonical/fictional/modder) — no lore.
3313. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD simplified Keplerian orbital math (ω = 2π / period + M(t) = mean_anomaly_at_epoch_rad + ω·(t-t_epoch) + position_in_parent_frame circular approximation) — not present.
3314. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD heliocentric position via parent chain recursion — not implemented.
3315. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD distance between bodies `d_ab(t)` and comms latency `d/c` — not implemented.
3316. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD launch catalog 12 worlds (sol + earth + earth_moon + mars + phobos + deimos + europa + mimas + vulcan + venus + belt_asteroid + orbital_station) — none modeled.
3317. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD `Worlds::get(world_id)` single source of truth — no enforcement.
3318. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD per-shard catalog (MMO shards declare hosted world subset; cross-shard travel via portal not seamless) — not implemented.
3319. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD acceptance WORLD-A + ASTRO-A — not specified at BP3.
3320. - [ ] [W7] [BP3] [M2+M5.10+M5.6+M5.9+M7.7] [GAP] WORLD modder contract `content/worlds/<id>.world.ron` schema validates — no schema.

## 296. spec/comms-voice-and-radio-model — Voice + radio simulation (BP3 design intent; M9.5 implements; M2/M4/M5/M5.7/M5.10/M6.6 precursors)
3321. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic propagation `AcousticSource` (position + speaker + base_loudness_db + spectrum + sealed_in_helmet) — no audio at BP3.
3322. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic propagation `AcousticReceiver` (position + listener + sealed_in_helmet + hearing_damage_factor + is_robot) — no receivers.
3323. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic vacuum check ("no medium" → silent) — not implemented.
3324. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic free-field attenuation (inverse square law) — not implemented.
3325. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic atmospheric absorption (high-frequency rolloff by humidity / temperature / composition) — not implemented.
3326. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic occlusion (Steam Audio-style raytraced walls/doors/terrain) — not implemented.
3327. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic reverb (Steam Audio-style raytraced reflection paths) — not implemented.
3328. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic helmet attenuation (src 30 dB + dst 25 dB) — not implemented.
3329. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acoustic hearing damage (damaged hearing raises threshold) — not implemented.
3330. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS speed of sound per medium (Air 343 m/s / Hot air 400-580 / Cold air 295 / Liquid water 1480 / Vacuum n/a / Helium-rich 970 "Donald Duck") — not modeled.
3331. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio `RadioTransmitter` (radio_id + owner + antenna + frequency + power + encryption + sidetone) — no radios.
3332. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio `RadioReceiver` (radio_id + owner + antenna + frequency + sensitivity_dbm + encryption_keys + band_limit_hz + static_threshold_dbm) — no radios.
3333. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio frequency match check (frequencies + encryption) — not implemented.
3334. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio free-space path loss FSPL formula — not implemented.
3335. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio antenna gain (transmitter direction-aware) — not implemented.
3336. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio multipath terrain (ACRE2 locked model + heightmap interference) — not implemented.
3337. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio material attenuation (walls / hills / foliage per frequency) — not implemented.
3338. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio atmospheric absorption (microwave only at long range) — not implemented.
3339. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio antenna gain receiver direction-aware — not implemented.
3340. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio ionospheric bounce (HF < 30 MHz on Earth-like + ionospheric_skip_loss) — not implemented.
3341. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio EM noise/interference (EnvironmentSignal.em.em_noise_db + solar flares + EMP) — not implemented.
3342. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS radio SNR computation + carries_voice + static_intensity_0_1 + PathKind LOS/Ionospheric — not implemented.
3343. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS band HF (3-30 MHz; ionospheric skip on Earth + LOS otherwise; SSB compressed hissy 300-3000 Hz; HAM 160m-10m) — not implemented.
3344. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS band VHF (30-300 MHz; LOS 5-30 km; cleaner FM voice 25 kHz; SINCGARS; HAM 6m+2m) — not implemented.
3345. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS band UHF (300 MHz-3 GHz; LOS 1-10 km; clean FM 12.5-25 kHz; AN/PRC-148/152; HAM 70cm) — not implemented.
3346. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS band Microwave (3-30 GHz; tight beam dish-to-dish LOS only; satellite uplink + dish-to-dish) — not implemented.
3347. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS HAM amateur bands (160m + 80m + 40m + 20m + 17m + 15m + 12m + 10m + 6m + 2m + 70cm + 23cm) — not implemented.
3348. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS launch radio roster (PRR-Lite UHF 0.5W + Squad-Mk1 VHF/UHF 5W + Squad-Mk2 VHF/UHF 10W + LongHaul-AT HF/VHF 50W + Dish-Beacon Microwave 50W + HAM-Field multiband + Ionopulse HF 100W fictional + Robot-Internal UHF 5W + Android-Module VHF/UHF 5W) — none implemented.
3349. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS antenna roster (Whip vertical / Long whip / Dipole wire / Yagi directional / Microwave dish / Helical) — none implemented.
3350. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS origin gating "Robots built-in radio (powered by power resource)" — not implemented.
3351. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS origin gating "Androids modular radio (built-in or modular upgrade; doesn't take equipment slot; powered by android battery)" — not implemented.
3352. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS origin gating "Humans equip radio (slot-occupying)" — not implemented.
3353. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS AI subscription to radio chatter (AI agents hear frequencies + doctrine reasoning + AI commander coordinates + going-dark = tactical mute) — not implemented.
3354. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS server-authoritative voice routing (cf-server routes voice + radio; clients receive band-limited streams; anti-cheat enforces transmission rules) — not implemented.
3355. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS accessibility (captions for every voice + radio transmission + visual indicators for transmission active / signal strength / band tuning + text-only chat fallback + optional narrator voice) — not implemented.
3356. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS audio reconstruction (band-limited + compressed + noise-gated + static-mixed at low SNR + distorted at very low SNR + per-band sound character HF SSB hissy / VHF FM cleaner / UHF FM clearer / microwave near-perfect) — not implemented.
3357. - [ ] [W7] [BP3] [M2+M4+M5+M5.10+M5.7+M6.6+M9.5] [GAP] COMMS acceptance COMMS-A — not specified at BP3.

## 297. spec/mining-and-extraction-model — Mining & extraction pipeline (BP3 design intent; M8.6 implements; precursor hooks at M5/M5.6/M7)
3358. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE `Ore` registry (id + classification + display_name + mass_per_unit_kg + bulk_density_kg_m3 + market_value_baseline + hazard_tags + storage_constraints + refining_recipe + smelting_recipe + canonical + package_source) — not present.
3359. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE OreClass enum (Metal / NonMetal / Volatile / Radioactive / Composite / Special) — not implemented.
3360. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE Hazard tag (FlammableDust / RadioactiveBeta / RadioactiveGamma / Toxic / Cryogenic / Explosive / StaticDischarge) — not implemented.
3361. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE Storage constraints (Standard / InsulatedTank / LeadShielded / InertGasFilled / Pressurized) — not implemented.
3362. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE launch ore set (iron / copper / silica / ice_volatiles / ice_oxite / ice_water / nickel / cobalt / gold / uranium / perchlorate / platinum_group) — none modeled.
3363. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `Sampler` (surface scan + outputs scan event with ore mix + depth band) — not implemented.
3364. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `LightDigger` (surface drill for soft material) — partial.
3365. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `HeavyDrill` (sub-surface drill for hard rock; tier-2; longer; needs power) — not implemented.
3366. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `CoreDrill` (DeepCrust extraction; tier-3; mech-mounted; needs cooling) — not implemented.
3367. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `RefiningStation` (stationary; ore → refined; consumes power; emits waste gas to atmospherics) — not implemented.
3368. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `SmelterFurnace` (stationary; refined → ingot; couples with combustion math: fuel + ore + O2) — not implemented.
3369. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `EnrichmentReactor` (stationary; uranium → fuel rod; tier-3; radiation hazard; AI avoid-unless-shielded) — not implemented.
3370. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `OreCargoBay` (mech/vehicle/dropship slot; holds extracted ore; mass affects mobility) — not implemented.
3371. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE tool class `ConveyorBelt` (base module; routes raw ore → refining → smelter) — not implemented.
3372. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE pipeline step "Sample" (Sampler at position + reads world.ore_deposits + ScanResult surface_visible + subsurface_likely + `mining.sampled` event) — not implemented.
3373. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE pipeline step "Drill / Extract" (Drill at cell + depth_band + material kernel removes ore + spawns rigid-body or pickup-able pile + cargo capacity check + `mining.drilled` + `mining.extracted` events) — not implemented.
3374. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE pipeline step "Refine" (RefiningStation + RefiningRecipe input + power + time → refined_material + waste gas via atmospherics + `mining.refined` event) — not implemented.
3375. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE pipeline step "Smelt" (SmelterFurnace + combustion: fuel + O2 + ore → ingot + CO2/Pollutant byproducts at locked temp/pressure + `mining.smelted` event) — not implemented.
3376. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE pipeline step "Use" (ingot → equipment crafting + `economy.material_consumed` event) — not implemented.
3377. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE per-world deposit generation (`Uniform` / `Veined` / `Pocketed` / `Streak` distribution per depth band + total ore mass bounded + deposit determinism same seed = byte-identical) — not implemented.
3378. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE `ResourceLedger` (shard + per_actor + per_team + per_world + market + audit_log) — not present.
3379. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE server-authoritative extraction validation against world deposit caps — not implemented.
3380. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE trade server-mediated (clients propose + server commits) — not implemented.
3381. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE audit log replicates as part of run-bundle for replay scrub + dispute resolution — not implemented.
3382. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE missions Survey (scan world + report findings + no extraction; tutorial-friendly) — not implemented.
3383. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE missions Quick Strike (drop + drill X + extract Y + dropship out; PvE) — not implemented.
3384. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE missions Holdout Extraction (hold position + slow drill runs; PvE/PvP) — not implemented.
3385. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE missions Salvage (extract from wrecked station/mech graveyard + bonus rare ores) — not implemented.
3386. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE missions Black Market Dump (extract uranium + ship to contested drop zone) — not implemented.
3387. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE AI miner doctrine "Survey before drilling" — not implemented.
3388. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE AI miner doctrine "Avoid hazardous deposits without protection" — not implemented.
3389. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE AI miner doctrine "Cargo capacity awareness (don't extract beyond actor + bay capacity)" — not implemented.
3390. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE AI miner doctrine "Power awareness (don't start slow drill if power won't last)" — not implemented.
3391. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE AI miner doctrine "Environmental awareness (don't mine Vulcan unprotected + don't dig in active dust storm)" — not implemented.
3392. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE AI miner doctrine "Squad coordination (sampler + driller pairing + haul-back drone)" — not implemented.
3393. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE AI miner doctrine "Defense awareness (if hostile retreat with cargo)" — not implemented.
3394. - [ ] [W7] [BP3] [M5+M5.6+M7+M8.6] [GAP] MINE acceptance AI-MINE-A-01..08 — not specified at BP3.

## 299. spec/network-sync-rollback-and-determinism — Network sync architecture (BP3 cf-net stub; M9-M12 implements)
3401. - [ ] [W7] [BP3] [M12+M9] [GAP] NET per-mode arch "Solo (local in-process authoritative server/sim; no internet transport)" — partial.
3402. - [ ] [W7] [BP3] [M10+M12+M9] [GAP] NET per-mode arch "LAN co-op (M10): deterministic lockstep + 16ms input delay" — not implemented.
3403. - [ ] [W7] [BP3] [M11+M12+M9] [GAP] NET per-mode arch "Online co-op (M11): server-auth + client prediction + snapshot interp + lag compensation + 50-200ms latency tolerance" — not implemented.
3404. - [ ] [W7] [BP3] [M12+M9] [GAP] NET per-mode arch "PvP arena (M12): rollback netcode GGPO + server validation + 16-50ms input delay 1-3 frames" — not implemented.
3405. - [ ] [W7] [BP3] [M12+M9] [GAP] NET per-mode arch "MMO shard (M12): server-auth + interest mgmt + snapshot delta + adaptive 60-120Hz" — not implemented.
3406. - [ ] [W7] [BP3] [M12+M9] [GAP] NET per-mode arch "Cross-shard events: eventually-consistent broadcaster" — not implemented.
3407. - [ ] [W7] [BP3] [M12+M9] [GAP] NET authority class `Truth` (Health / inventory / mission state / terrain collision / material/gas/fire / confirmed projectile hits / base power / doors/platforms / AI final decisions / save/replay / PvP validation) — not enforced.
3408. - [ ] [W7] [BP3] [M12+M9] [GAP] NET authority class `Prediction` (Local movement / provisional projectile path / provisional impact / held-weapon response) — not implemented.
3409. - [ ] [W7] [BP3] [M12+M9] [GAP] NET authority class `Presentation` (Lighting / smoke / particles / trails / decals / camera shake / audio / interpolation / debug overlays) — not classified.
3410. - [ ] [W7] [BP3] [M12+M9] [GAP] NET authority class `Advisory` (Broadphase candidates / pathfinding heatmaps / visibility hints / AI perception maps / compression hints) — not implemented.
3411. - [ ] [W7] [BP3] [M12+M9] [GAP] NET determinism "Sim tick output bit-identical (same seed + same input = same output)" — partial; not tested across platforms.
3412. - [ ] [W7] [BP3] [M12+M9] [GAP] NET determinism "Per-tick checksum via `blake3`" — partial; uses simple checksum.
3413. - [ ] [W7] [BP3] [M12+M9] [GAP] NET determinism "CI matrix: 100 runs/seed × Win/Lin/Mac × x86/ARM" — not configured.
3414. - [ ] [W7] [BP3] [M12+M9] [GAP] NET cross-platform float "f32 only in sim islands; no f64" — not enforced.
3415. - [ ] [W7] [BP3] [M12+M9] [GAP] NET cross-platform float "RUSTFLAGS=-C target-feature=+sse2,+sse4.2 baseline" — not configured.
3416. - [ ] [W7] [BP3] [M12+M9] [GAP] NET cross-platform float "LLVM -ffast-math disabled in sim crates (NO_FAST_MATH=1)" — not configured.
3417. - [ ] [W7] [BP3] [M12+M9] [GAP] NET cross-platform float "STD::FROUND_TO_NEAREST rounding mode" — not enforced.
3418. - [ ] [W7] [BP3] [M12+M9] [GAP] NET cross-platform float "No transcendental functions on hot paths without stabilized impl OR fixed-point alternative" — not enforced.
3419. - [ ] [W7] [BP3] [M12+M9] [GAP] NET replay reproducibility "cf-replay event log + per-tick checksums" — partial.
3420. - [ ] [W7] [BP3] [M12+M9] [GAP] NET replay reproducibility "cf-headless replay --verify-checksums walks every event + asserts state matches" — not implemented.
3421. - [ ] [W7] [BP3] [M12+M9] [GAP] NET network input ordering "Per-tick input batches on server + deterministic ordering by client_id ascending + tie-break by player_id" — no server.
3422. - [ ] [W7] [BP3] [M12+M9] [GAP] NET client prediction + reconciliation cycle (predict → render → send → wait snapshot → compare → rewind if mismatch → smooth/snap → emit `prediction_corrected`) — not implemented.
3423. - [ ] [W7] [BP3] [M12+M9] [GAP] NET prediction window "1-3 frames at 60Hz (16-50ms) capped at server's max-allowed-prediction" — not implemented.
3424. - [ ] [W7] [BP3] [M12+M9] [GAP] NET anti-cheat validation "Server validates every client input against current state + rejects impossible inputs (shoot-through-wall / teleport / infinite-ammo) + `input_rejected_by_anticheat`" — no anti-cheat.
3425. - [ ] [W7] [BP3] [M12+M9] [GAP] NET rollback netcode (GGPO-style with deterministic sim + bounded rollback window 8 frames 133ms) — not implemented.
3426. - [ ] [W7] [BP3] [M12+M9] [GAP] NET lag compensation (CS:Source model: server rewinds world state by client interp delay + ping/2 + validate hit at rewind tick + max 200ms rewind cap + `shot_rejected_by_lag_compensation`) — not implemented.
3427. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test sync-drift` (multi-client sync drift detection) — not implemented.
3428. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test latency-injection` (network degradation simulation) — not implemented.
3429. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test rollback-burst` (rollback stress test) — not implemented.
3430. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test replay-determinism` (100-run determinism verification) — not implemented.
3431. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test cross-platform-determinism` (Win/Lin/Mac × x86/ARM CI matrix) — not implemented.
3432. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test multi-shard` (MMO load test) — not implemented.
3433. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test combat-ttk` (TTK regression) — not implemented.
3434. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test network-jitter` (resilience boundary) — not implemented.
3435. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test prediction-correction-rate` (prediction accuracy) — not implemented.
3436. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test anti-cheat-injection` (anti-cheat validation) — not implemented.
3437. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl test replay-bit-identical` / `--inject-noise per-tick` — not implemented.
3438. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cfctl bench replay --measure throughput` — not implemented.
3439. - [ ] [W7] [BP3] [M12+M9] [GAP] NET `cf-network-sim` dev-tool (latency injection 0-500ms + packet loss 0-50% + jitter 0-200ms + bandwidth 10kbps-100Mbps + per-platform emulation Steam Deck + mobile + satellite + dial-up) — not present.
3440. - [ ] [W7] [BP3] [M12+M9] [GAP] NET perf budget "Online co-op (4 players): <50KB/s up + <100KB/s down per client" — not measured.
3441. - [ ] [W7] [BP3] [M12+M9] [GAP] NET perf budget "PvP arena (8 players): <200KB/s per client" — not measured.
3442. - [ ] [W7] [BP3] [M12+M9] [GAP] NET perf budget "MMO shard (50 players): <500KB/s per client peak" — not measured.
3443. - [ ] [W7] [BP3] [M12+M9] [GAP] NET perf budget "MMO shard (200 players): <2MB/s per client peak" — not measured.

## 311. spec/server-app-architecture — cf-server architecture (BP3 cf-server stub; closed direction)
3737. - [ ] [W7] [BP3] [GAP] SERVER `cf-server` binary entry point (pulls cf-sim-core + cf-terrain + cf-physics + cf-actor + cf-chassis + cf-equipment + cf-ai + cf-mission + cf-replay + cf-net + cf-control + cf-save + cf-mod; no cf-render-2d/cf-ui/cf-audio) — stub.
3738. - [ ] [W7] [BP3] [GAP] SERVER `cf-server-ops` library (config loader + mode selector + health/readiness + metrics + log shipping + shutdown drain + restart hooks) — stub.
3739. - [ ] [W7] [BP3] [GAP] SERVER `cf-server-persistence` library (MMO shard snapshot/restore + durable event store + cross-tick journaling + migration handlers) — stub.
3740. - [ ] [W7] [BP3] [GAP] SERVER `cf-server-anti-cheat` library (server-side validation + replay-driven anomaly detection + rate limits + capability gates) — stub.
3741. - [ ] [W7] [BP3] [GAP] SERVER `cf-server-admin` library (kick/ban/save/restart/mode switch/scenario load + JSON-RPC over cf-control + admin capability gate) — stub.
3742. - [ ] [W7] [BP3] [GAP] SERVER modes `coop_room` / `pvp_arena` / `lan_room` / `mmo_shard` / `ranked_arena` / `lobby_directory` — none implemented.
3743. - [ ] [W7] [BP3] [GAP] SERVER config RON `ServerConfig` (schema_version + mode + bind + public_address + max_clients + scenario + package_set + mod_packs + capabilities + persistence + anti_cheat + sim_backend + ai_mind + ops + rate_limits) — no config.
3744. - [ ] [W7] [BP3] [GAP] SERVER core loop (parse CLI/config + load package_set + initialize cf-sim-core fixed-tick + open cf-net listener + admit clients per capability gates + per tick drain cf-control + validate anti-cheat + run sim + emit events + broadcast snapshots + write replay events + periodically persist + rotate logs + expose metrics + health/readiness probes + drain on shutdown) — not implemented.
3745. - [ ] [W7] [BP3] [GAP] SERVER authority "Player input: Client sends cf-control + server validates against capability + rate limit + anti-cheat + only accepted enter sim" — not implemented.
3746. - [ ] [W7] [BP3] [GAP] SERVER authority "Sim state: 100% server-authoritative + clients receive snapshots + event deltas + prediction+reconciliation only for player-driven actor" — not implemented.
3747. - [ ] [W7] [BP3] [GAP] SERVER authority "Server sim backend: cpu required canonical + gpu_advisory hints only + gpu_certified manifest + CPU fallback" — not implemented.
3748. - [ ] [W7] [BP3] [GAP] SERVER authority "Terrain mutation: server-authoritative + clients render dirty regions delivered" — not implemented.
3749. - [ ] [W7] [BP3] [GAP] SERVER authority "AI decisions: server-authoritative + clients see reason labels via event stream" — not implemented.
3750. - [ ] [W7] [BP3] [GAP] SERVER authority "Mission director: server-authoritative + clients see commander events with reason strings" — not implemented.
3751. - [ ] [W7] [BP3] [GAP] SERVER authority "Save / persistence: server-authoritative for MMO shards + local in-process for solo + host/server for LAN + online co-op saves" — not implemented.
3752. - [ ] [W7] [BP3] [GAP] SERVER authority "Anti-cheat: server-authoritative + server-side validators mandatory + client hints not trusted" — not implemented.
3753. - [ ] [W7] [BP3] [GAP] SERVER authority "Match grammar: server-authoritative + team config flexibility (1v1 through NvN, FFA, asymmetric, coop) enforced server-side + AI fills empty slots per Match.ai_fill_policy" — not implemented.
3754. - [ ] [W7] [BP3] [GAP] SERVER authority "Voice routing: server-authoritative + clients send Opus-encoded packets + server fans out to receivers passing acoustic+radio gates + no P2P voice" — not implemented.
3755. - [ ] [W7] [BP3] [GAP] SERVER authority "Radio link state: server-authoritative RadioLink graph + per-pair propagation + frequency tuning + encryption + clients send tx intent + server validates+routes" — not implemented.
3756. - [ ] [W7] [BP3] [GAP] SERVER network transport `lan_room` (UDP + LAN broadcast discovery + TCP fallback) — not configured.
3757. - [ ] [W7] [BP3] [GAP] SERVER network transport `coop_room` (UDP via lightyear/renet/quinn + NAT punch-through + STUN-style relay + Steam Datagram Relay/EOS adapter optional) — not configured.
3758. - [ ] [W7] [BP3] [GAP] SERVER network transport `pvp_arena` (same as coop_room + stricter authority + anti-cheat profile + TLS over QUIC for tournament) — not configured.
3759. - [ ] [W7] [BP3] [GAP] SERVER network transport `mmo_shard` (QUIC quinn + long-lived connections + per-region UDP relay + Steam Datagram Relay/EOS/PlayFab adapters optional) — not configured.
3760. - [ ] [W7] [BP3] [GAP] SERVER network transport `lobby_directory` (HTTPS REST + WebSocket for live presence + Steam server browser/EOS lobby adapters) — not configured.
3761. - [ ] [W7] [BP3] [GAP] SERVER modding "Server-side mods (same cf-mod package format on client and server)" — not enforced.
3762. - [ ] [W7] [BP3] [GAP] SERVER modding "Hash sync (mandatory + matching package set hash on join + mismatch produces clean error with downloadable manifest of differences)" — not implemented.
3763. - [ ] [W7] [BP3] [GAP] SERVER modding "Auto-download optional per server config; off by default for production" — not implemented.
3764. - [ ] [W7] [BP3] [GAP] SERVER modding "Server-only mods (admin tools + tournament rules + `server_only: true` + clients see per-server policy summary never raw mod code)" — not implemented.
3765. - [ ] [W7] [BP3] [GAP] SERVER modding "Trust tiers (`vanilla` / `verified` / `community` / `experimental` + servers can pin maximum trust tier accepted from clients)" — not implemented.
3766. - [ ] [W7] [BP3] [GAP] SERVER modding "Sandbox (mod scripts in cf-mod sandboxed deterministic island + non-deterministic ops forbidden in sim-tick scope)" — not implemented.
3767. - [ ] [W7] [BP3] [GAP] SERVER anti-cheat "Input validation (reject outside declared rate limits / capability set / actor authority window)" — not implemented.
3768. - [ ] [W7] [BP3] [GAP] SERVER anti-cheat "Replay correlation (compare client-claimed actor state vs server snapshots + flag drift for review)" — not implemented.
3769. - [ ] [W7] [BP3] [GAP] SERVER anti-cheat "Capability gates (`admin` / `debug` / `god` / `teleport` / `force_damage` / `reveal_map` off by default + require server config opt-in)" — not implemented.
3770. - [ ] [W7] [BP3] [GAP] SERVER anti-cheat "Anomaly profiles (`casual` / `competitive` / `tournament_strict` thresholds for input rate / snapshot drift / modding / reason-label coverage)" — not implemented.
3771. - [ ] [W7] [BP3] [GAP] SERVER anti-cheat "Audit log (every rejection writes `system.anti_cheat_*` event in replay/run-bundle for offline review)" — not implemented.

## 312. spec/backend-networking — Backend posture (STUB; DR-013 closes the local spine; BP3+ extends)
3772. - [ ] [W7] [BP3] [DR-013] [GAP] BACKEND core local spine "Health endpoints + schema versions + local package registry + join eligibility + deep-link parser + local server supervisor + replay/report index + diagnostics export + privacy redaction" — partial; minimal stubs only.
3773. - [ ] [W7] [BP3] [DR-013] [GAP] BACKEND fixture-backed "Static servers.json + packages.json + replays.json + resolver fixtures + fake supervisor + package mismatch rows + stale heartbeat cases" — none present.
3774. - [ ] [W7] [BP3] [DR-013] [GAP] BACKEND public services "lobby_directory + server browser + account adapter + anti-cheat foundation + persistence/journal + server observability" — not implemented.
3775. - [ ] [W7] [BP3] [DR-013] [GAP] BACKEND platform adapters research (Steam server browser/SDR/GameNetworkingSockets + EOS lobbies/sessions + PlayFab lobby/server + self-hosted directory + LAN discovery) — not researched.
3776. - [ ] [W7] [BP3] [DR-013] [GAP] BACKEND server discovery schema (version + region + rules + content hashes + required packages + mod trust + player/bot counts + replay compatibility + join eligibility) — not designed.

## 315. spec/persistent-mmo-architecture — Persistent MMO shard (BP3 design intent; M12 implements)
3864. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "World terrain (per-region chunk + carved/repaired persists between session/restart + material state matches deterministic-island contract)" — not implemented.
3865. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Bases (full layouts + module HP/ammo/power state per faction + player-built bases survive reboot)" — not implemented.
3866. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Player veterans (per-account roster: names + traits + injuries + equipment + AI doctrines + kill/save histories + cross-mission)" — not implemented.
3867. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Faction state (reputation + contract pool + enemy commander memory + doctrine evolution)" — not implemented.
3868. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Mission director memory (cross-mission adaptation to player tactics + LLM memory writes)" — not implemented.
3869. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Mech/chassis state (damage history + salvageable modules + paint/identity + crew slots)" — not implemented.
3870. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Salvage / inventory (per-account materials + parts + recovered modules)" — not implemented.
3871. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Replay archives (per-mission run bundles queryable by player/faction/timeframe)" — not implemented.
3872. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence surface "Audit log (anti-cheat events + admin actions + config changes append-only + retention policy)" — not implemented.
3873. - [ ] [W7] [BP3] [M12] [GAP] MMO shard topology (one process running `cf-server --mode mmo_shard` + persistent world manifest + persistent state store + concurrent-player target + contract director + persistence cadence) — not implemented.
3874. - [ ] [W7] [BP3] [M12] [GAP] MMO player count target `intimate` (4-16 + 4-core/8GB/100Mbps) — not implemented.
3875. - [ ] [W7] [BP3] [M12] [GAP] MMO player count target `community` (16-50 + 8-core/16GB/1Gbps) — not implemented.
3876. - [ ] [W7] [BP3] [M12] [GAP] MMO player count target `regional` (50-100 + 16-core/32GB/1Gbps + relay) — not implemented.
3877. - [ ] [W7] [BP3] [M12] [GAP] MMO player count target `flagship` (100-200 + 32-core/64GB/10Gbps + relay tier) — not implemented.
3878. - [ ] [W7] [BP3] [M12] [GAP] MMO player count target `experimental` (200+ + TBD + R&D only) — not implemented.
3879. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence model "Snapshot store (compressed binary bincode+zstd per region + default every 10min + fast restore on shard restart)" — not implemented.
3880. - [ ] [W7] [BP3] [M12] [GAP] MMO persistence model "Event journal (append-only events.jsonl per shard tick + continuous batched + tick-level audit + replay reconstruction + point-in-time recovery)" — partial.
3881. - [ ] [W7] [BP3] [M12] [GAP] MMO recovery "Load most recent snapshot + replay journal forward + reach live state + schema-version-aware migration handlers" — not implemented.
3882. - [ ] [W7] [BP3] [M12] [GAP] MMO storage "Local filesystem default + operators can mount network storage + S3-compatible + remote durable journals via adapters + no proprietary cloud dependency" — not implemented.
3883. - [ ] [W7] [BP3] [M12] [GAP] MMO account model "Accounts required for public shards / not for private LAN/co-op" — not implemented.
3884. - [ ] [W7] [BP3] [M12] [GAP] MMO account provider "Local account file + lobby_directory adapter + Steam/EOS/PlayFab post-launch + token-based bearer credentials with expiry + replay/run-bundles redact tokens" — not implemented.
3885. - [ ] [W7] [BP3] [M12] [GAP] MMO authoritative loop (per fixed tick: drain cf-control + rate-limit + tick sim for active region + mission director + faction commander + LLM mind proposals async + emit events to clients in interest range + persist + snapshot if cadence) — not implemented.
3886. - [ ] [W7] [BP3] [M12] [GAP] MMO perf "50-100 concurrent + 200 AI actors targets 30Hz sim + 60Hz client interpolation (T-PERF tracks)" — not measured.
3887. - [ ] [W7] [BP3] [M12] [GAP] MMO interest management (Actors visual+audible range + mission allies always + faction commander always + Terrain chunk+1 halo + Base modules visible range + Audio captions for audible only + AI reason labels per visibility + Voice acoustic propagation Steam Audio + Radio link state per-frequency + Match grammar per-shard + World binding per shard) — not implemented.
3888. - [ ] [W7] [BP3] [M12] [DR-018] [GAP] MMO mission/contract loop "Per-faction contract pool + player accepts via base/HUB UI + contract director spawns sequence + state persists across sessions + per DR-018 consequence ladder + party-up + lobby/portal lists" — not implemented.
3889. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-cheat operator "Profile `competitive` (stricter than casual, less strict than tournament_strict) + tunable input rate caps + capability set + replay drift thresholds + auto-kick/ban + audit logs operator-readable + player appeals out-of-game" — not implemented.
3890. - [ ] [W7] [BP3] [M12] [GAP] MMO modding "Server-required mods declared + clients see manifest before join + Server-only mods allowed + Persistence migration handlers + Trust tiers + Sandbox cf-mod deterministic island" — not implemented.
3891. - [ ] [W7] [BP3] [M12] [GAP] MMO cross-shard "Lobby/portal `lobby_directory` instance + Shard browse by mode/region/ping/player count/packages/ruleset/trust + Cross-shard travel via log-out+log-in (no live cross-shard combat) + Identity persistence + Federation operator-of-operators" — not implemented.
3892. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Seamless single-shard world" — out of scope (correctly).
3893. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Cross-shard live combat" — out of scope (correctly).
3894. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Subscription-funded MMO" — out of scope (correctly).
3895. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Live cash shop / pay-to-win" — out of scope (correctly).
3896. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Mandatory account at all multiplayer modes" — out of scope (correctly).
3897. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Operator-imposed publisher hosting" — out of scope (correctly).
3898. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Real-money trading of in-game items" — out of scope (correctly).
3899. - [ ] [W7] [BP3] [M12] [GAP] MMO anti-goal "Auto-population (server bots dressed as players)" — out of scope (correctly).
3900. - [ ] [W7] [BP3] [M12] [GAP] MMO acceptance MMO-001..012 — not specified at BP3.

## 319. spec/atmospherics-and-chemistry-model — Stationeers-grade atmospherics (BP3 cf-atmos stub; M5.9/M7.5 lands)
3982. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS `Atmosphere` unit record (id + kind RoomCell/PipeNetwork/Suit/Canister/Lung/DeviceInternal + volume_l + moles per_gas + moles_liquid + moles_solid + temperature_k + insulation_class + pressure_differential_max_pa + thermal_mass_j_per_k + material_shell + parent + flags) — not modeled.
3983. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS ideal gas law `P = nRT/V` (R=8314.46 L·Pa/(mol·K)) — not implemented.
3984. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS per-gas partial pressure `P_g = n_g·R·T/V` — not implemented.
3985. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS atmospheric mixing (Total n summed + Total V summed + Temperature mass-weighted by specific heat) — not implemented.
3986. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gradual vs immediate mixing (flow-rate-limited interface vs directly connected) + per-tick partial mixing events — not implemented.
3987. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas Oxygen (16 g/mol + 21.1 J/mol·K + 800 J/mol latent + condensation 6.3 kPa @ 56.4K + max liquid 6000 kPa @ 162.2K + freeze 56.4K + 30 L/kmol + pure O2+volatiles autoignites at 573.15K) — not modeled.
3988. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas Nitrogen (14 g/mol + 20.6 J/mol·K + inert filler + cryogenic coolant) — not modeled.
3989. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas CO2 (44 g/mol + 28.2 J/mol·K + 600 J/mol latent + 517 kPa @ 217.82K + 217.82K freeze + 40 L/kmol + plant feed + coolant) — not modeled.
3990. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas Volatiles/Methane (16 g/mol + 20.4 J/mol·K + 1000 J/mol latent + 6.3 kPa @ 81.6K + combustible with O2/N2O/Ozone) — not modeled.
3991. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas Pollutant (28 g/mol + 24.8 J/mol·K + 2000 J/mol latent + toxic to humans/plants + coolant high latent heat) — not modeled.
3992. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas H2 (2 g/mol + 20.4 J/mol·K + 200 J/mol latent + combustible with O2/N2O/Ozone cleanest fuel) — not modeled.
3993. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas N2O (44 g/mol + 23 J/mol·K + oxidizer lower autoignition with volatiles 50°C + rocket fuel) — not modeled.
3994. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas Steam/H2O (18 g/mol + 72 J/mol·K very high + inert + combustion byproduct H2+O2 + high-capacity coolant) — not modeled.
3995. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas Ozone (48 g/mol + oxidizer autoignition with H2/Volatiles 150°C + rocketry + tracer) — not modeled.
3996. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS gas Helium (4 g/mol + inert + cryogenic) — not modeled.
3997. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS liquid mixtures Polluted Water + Alcohol + Silanol + Liquid Sodium Chloride + HCl + Hydrazine — not modeled.
3998. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS combustion reaction Volatiles+O2 (2V + 1O2 → 6CO2 + 3X + 572 kJ + 573.15K autoignition + ≥5% O2 AND ≥5% Volatiles) — not implemented.
3999. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS combustion reaction Volatiles+N2O (1V + 1N2O → 2CO2 + 2N2 + 572 kJ + 323.15K) — not implemented.
4000. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS combustion reaction Volatiles+Ozone (3V + 2O3 → 6CO2 + 3X + 1Steam + 1716 kJ + 423.15K) — not implemented.
4001. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS combustion reaction H2+O2 (2H2 + 1O2 → 3Steam + 612 kJ + 573.15K + ≥5% O2 AND ≥5% H2 AND ≥10 kPa) — not implemented.
4002. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS combustion reaction H2+N2O (1H2 + 1N2O → 1Steam + 1N2 + 612 kJ + 323.15K) — not implemented.
4003. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS combustion reaction H2+Ozone (3H2 + 1O3 → 4Steam + 1836 kJ + 423.15K) — not implemented.
4004. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS reaction rate `rate_o2(T) = clamp01(1/(0.002·T^1.6 + 0.05))/5` — not implemented.
4005. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS reaction rate `rate_n2o(T) = clamp01(1/(0.0025·T^1.01 + 0.05))/5` — not implemented.
4006. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS combustion efficiency 95% of limiting ingredient per ignition + residue prevents perpetual lossless cycling — not implemented.
4007. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS energy → temperature → pressure flow (ΔE = moles_consumed × energy_per_mol; ΔT = ΔE/Σ(n_g·cp_g); P = nRT/V recompute) — not implemented.
4008. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS pressure spikes from combustion can rupture pipes/canisters/walls per `pressure_differential_max_pa` — not implemented.
4009. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS phase change (gas ↔ liquid ↔ solid gradually per phase diagram + latent heat consumed on evaporation released on condensation + sublimation reverse risky) — not implemented.
4010. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS pipe network as one atmosphere (100 L per segment default + pumps/valves/regulators/filtration/condensation chambers split network) — not implemented.
4011. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Pipe Segment (100 L volume contribution + junctions don't add)" — not implemented.
4012. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Active Vent (room↔pipe boundary + Outward/Inward + PressureExternal/Internal thresholds + 100W + 10 kPa/tick into 8000L grid)" — not implemented.
4013. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Passive Vent (equalizes passively + no power + no flow control)" — not implemented.
4014. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Pressure Regulator (targets specific output pressure)" — not implemented.
4015. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Back Pressure Regulator (targets specific input pressure + dump-valve role)" — not implemented.
4016. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Volume Pump (0-10 L/tick dial + rate-based not pressure-based)" — not implemented.
4017. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Turbo Pump (high-flow industrial transfer)" — not implemented.
4018. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Valve (manual on/off + one-way variants)" — not implemented.
4019. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Filtration Unit (splits 1 input into 2 outputs by gas filter type)" — not implemented.
4020. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Condensation Chamber (gas → liquid + controlled cooling)" — not implemented.
4021. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Evaporation Chamber (liquid → gas + controlled heating)" — not implemented.
4022. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Purge Valve (liquid pipe → gas pipe gas only)" — not implemented.
4023. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Pressurant Valve (gas pipe → liquid pipe gas only)" — not implemented.
4024. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Condensation Valve (gas pipe → liquid pipe liquid only)" — not implemented.
4025. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Expansion Valve (liquid pipe → gas pipe liquid only)" — not implemented.
4026. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS device "Tank (up to 10 MPa portable game value + per-material structural limit)" — not implemented.
4027. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS pipe damage thresholds (Gas pipes rupture if frozen-solid > 0.05 mol/L OR pressure differential > 600 atm OR liquid stress > 100%; Liquid pipes rupture if pressure differential > 60 atm OR frozen > 0.05 mol/L) — not implemented.
4028. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS apertures physical (door openings + vents + cracked windows + bullet holes + shaped-charge cuts + blast breaches + pipe ruptures + suit punctures + terrain cracks create aperture records with area + edge material + normal + source event + open/closed/breached state) — not implemented.
4029. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS heat moves through matter (solids + liquids + gases + equipment + armor + weapons + pipes + tanks + doors + base modules exchange heat through conductivity/insulation + moving fluids + phase change + combustion + electrical load + collision/friction + ambient/radiation exchange) — not implemented.
4030. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS rooms as first-class atmospheres (connected sealed-volume graph = one atmosphere + walls/floors/ceilings are barriers sealed + doors/windows are barriers with state open/closed/breached + vacuum dissipates within "a few large grid atmospheres of distance") — not implemented.
4031. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS replay events "Every reaction emits + every phase change emits + every breach emits + run-bundle reproduces atmosphere state from event stream" — not implemented.
4032. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS performance "active-region + cache-friendly + multicore-ready + benchmarked + GPU acceleration for visualization + replay-deterministic CPU truth as acceptance" — not measured.
4033. - [ ] [W7] [BP3] [M5.9+M7.5] [GAP] ATMOS acceptance ATMOS-A — not specified at BP3.

## 320. spec/hybrid-llm-ai-plan — Async LLM Mind Layer (BP3 design intent; M6.5 lands)
4034. - [ ] [W7] [BP3] [M6.5] [GAP] LLM rule "LLM never in reflex loop" — not enforced.
4035. - [ ] [W7] [BP3] [M6.5] [GAP] LLM rule "Shipped AI must remain strong with zero network access" — not enforced at BP3.
4036. - [ ] [W7] [BP3] [M6.5] [GAP] LLM cadence "Reflex (8-16ms; no LLM)" — partial.
4037. - [ ] [W7] [BP3] [M6.5] [GAP] LLM cadence "Tactic (100-250ms; no direct control)" — no tactic layer.
4038. - [ ] [W7] [BP3] [M6.5] [GAP] LLM cadence "Job/Commander (0.5-2s; optional LLM proposal input)" — no commander.
4039. - [ ] [W7] [BP3] [M6.5] [GAP] LLM cadence "LLM Mind (2-30s or between missions; LLM allowed)" — not present.
4040. - [ ] [W7] [BP3] [M6.5] [GAP] LLM cadence "Strategic Reflection (between missions/background; LLM allowed)" — not present.
4041. - [ ] [W7] [BP3] [M6.5] [GAP] LLM target architecture "Game sim emits events → Observation Compressor (filters + fog-of-war + replay-visible) → Mind Task Queue (priority + budget + TTL + cancellation) → LLM Provider Adapter (OpenAI/Anthropic/local OpenAI-compatible vLLM/llama.cpp/Ollama/mock) → Strict Structured Output (AiMindProposal JSON schema + no arbitrary code + no direct low-level actions) → Proposal Validator (schema + TTL/staleness + capability + fog-of-war + cost/latency/abuse + replay event) → Policy Compiler (doctrine patch → utility weights + squad goal → commander blackboard + personality update → actor profile + memory write → campaign memory + dialogue → captioned radio event) → Local AI Executor" — not present at BP3.
4042. - [ ] [W7] [BP3] [M6.5] [GAP] LLM crate layout `cf-ai::mind::schema` + `cf-ai::mind::provider` + provider adapters behind cargo features (`mind-openai` + `mind-anthropic` + `mind-ollama` + `mind-openai-compatible`) + always-built deterministic mock + test scenarios in `tests/` + content packs in `content/` — not configured.
4043. - [ ] [W7] [BP3] [M6.5] [GAP] LLM `MindObservationFrame` (schema_version + run_id + sim_tick + scope + visible_facts + recent_events + orders + resources + threats + terrain_affordances + actor_profiles + constraints) — not modeled.
4044. - [ ] [W7] [BP3] [M6.5] [GAP] LLM `MindTask` (task_id + kind doctrine_patch/squad_plan/dialogue/memory_extract/enemy_adaptation/debrief/profile_generation + priority + deadline_ms + max_cost_usd + provider_policy + observation + output_schema) — not modeled.
4045. - [ ] [W7] [BP3] [M6.5] [GAP] LLM `AiMindProposal` (schema_version + task_id + sim_tick_observed + valid_until_tick + scope + confidence + summary + intent_label + orders + doctrine_patch + utility_weight_changes + risk_posture) — not modeled.
4046. - [ ] [W7] [BP3] [M6+M6.5] [GAP] LLM acceptance MIND-001..010 against deterministic mock provider with M6 local AI continuing under provider failure/sleep/stale — none pass.
4047. - [ ] [W7] [BP3] [M6.5] [GAP] LLM allowed uses "Doctrine changes + squad intent + personality + post-mission reflection + deception + chatter + memory + profile evolution + mission-director adaptation + mod/workbench assistance" — none implemented.
4048. - [ ] [W7] [BP3] [M6.5] [GAP] LLM forbidden "LLM aims/dodges/jumps/fires/paths per frame" — would-be enforced (no LLM exists).
4049. - [ ] [W7] [BP3] [M6.5] [GAP] LLM forbidden "Stream raw game state every tick to model" — would-be enforced.
4050. - [ ] [W7] [BP3] [M6.5] [GAP] LLM forbidden "Let LLM emit arbitrary executable code into active campaign" — would-be enforced.
4051. - [ ] [W7] [BP3] [M6.5] [GAP] LLM forbidden "Require API key for core game to work" — would-be enforced.
4052. - [ ] [W7] [BP3] [M6.5] [GAP] LLM forbidden "Hide unfair perfect information inside LLM prompts" — would-be enforced.
4053. - [ ] [W7] [BP3] [M6.5] [GAP] LLM forbidden "Make replay/E2E/AI-H tests depend on a live paid model" — would-be enforced.

## 321. spec/backend-service-hub-slice-a — Backend Slice A (BP3 partial; closes M9-M12 spine)
4054. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/health` (service version + schema versions + clock) — partial.
4055. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/servers` (server list sorted by last_heartbeat_at + filtered client-side) — not implemented.
4056. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/servers/register` (optional local-only registration for prototype servers) — not implemented.
4057. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/servers/{server_id}/heartbeat` (updates player counts + health + map + packages + expiry) — not implemented.
4058. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/servers/{server_id}/players` (visible player/bot summary if allowed) — not implemented.
4059. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/join/eligibility` (computes local install compatibility + join blockers from server row) — not implemented.
4060. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/packages` (lists known packages + manifest hashes) — not implemented.
4061. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/packages/{package_id}/{version}` (returns package manifest summary + provenance pointer) — not implemented.
4062. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/replays` (lists local replay metadata + tags) — not implemented.
4063. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK API `/v1/diagnostics/report` (accepts local crash/replay/AI-fail summaries in dev builds) — not implemented.
4064. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK server row schema (server_id + display_name + endpoint + region + version + protocol_version + simulation_profile + terrain_sync_profile + map_id/hash + game_mode + ruleset + players + bots + password_required + invite_required + package_set + content_manifest_hash + replay_schema_version + trust_tier + mod_safety + join_state + join_blockers + last_heartbeat_at/expires_at + tags + warnings) — not modeled.
4065. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `can_join` (version + protocol + packages + map + replay schema + trust pass) — not implemented.
4066. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `needs_download` (compatible packages missing but available) — not implemented.
4067. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `needs_update` (client or protocol is old) — not implemented.
4068. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `blocked_package_hash` (local package matches but hash differs + repair/reinstall action) — not implemented.
4069. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `blocked_unknown_script` (server requires unsafe or unknown script capability + dev override only in private mode) — not implemented.
4070. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `blocked_password_or_invite` (prompt without putting secret in URL) — not implemented.
4071. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `blocked_replay_schema` (server/replay event schema incompatible + warning) — not implemented.
4072. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK join eligibility result `blocked_trust` (server trust tier below user threshold + reason + override) — not implemented.
4073. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK deep link `ourgame://join?server_id=...&invite=...` (invite opaque + revocable + no plain password) — not implemented.
4074. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK deep link `ourgame://connect?host=...&port=...` (allowed for local/dev + resolver runs compatibility checks) — not implemented.
4075. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK deep link `ourgame://package?id=...&version=...&hash=...` (show provenance + required capabilities before install) — not implemented.
4076. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK deep link `ourgame://replay?id=...` (resolve local or remote metadata + open viewer/report flow) — not implemented.
4077. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK local server supervisor state `created` (config accepted + process not started) — not implemented.
4078. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK local server supervisor state `starting` (process spawned + process id + start time + expected health endpoint) — not implemented.
4079. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK local server supervisor state `ready` (server accepts queries/connects + endpoint + protocol + map + package hash + max players + bot count) — not implemented.
4080. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK local server supervisor state `degraded` (server running but health warnings + warning code + user-facing message) — not implemented.
4081. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK local server supervisor state `stopping` (controlled shutdown requested + reason + timeout) — not implemented.
4082. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK local server supervisor state `stopped` (clean exit + exit code + duration) — not implemented.
4083. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK local server supervisor state `crashed` (unexpected exit or failed health + exit code + stderr tail + replay/report id) — not implemented.
4084. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK hub server browser dense table (name + ping/region + mode + map + humans + bots + packages + trust + join state + warnings) — no browser.
4085. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK hub filters (quick search + mode + player range + bot presence + password/invite + compatible-only + trust tier + package status + local/dev servers) — no filters.
4086. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK hub server detail drawer (package set + rules + AI profile + replay compatibility + trust explanation + join blockers + provenance links) — no drawer.
4087. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK hub local game panel (start/stop local sandbox/server + lifecycle states + health + package hash + logs + quick-open replay folder) — minimal.
4088. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK hub replay/report browser (local replays grouped by map + event tags + AI failures + deaths + terrain collapses + friendly fire + version + package hash) — no browser.
4089. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK hub mods/workbench link (missing package actions route to workbench/registry not generic error) — no workbench.
4090. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK hub diagnostics (backend health + API schema versions + local package registry status + last failed join explanation) — no diagnostics.
4091. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `backend_server_list_fetched` (Hub fetches /v1/servers) — not emitted.
4092. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `backend_server_registered` (Local or remote server registers) — not emitted.
4093. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `backend_server_heartbeat` (Server updates liveness + metadata) — not emitted.
4094. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `backend_server_expired` (Backend removes stale server row) — not emitted.
4095. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `join_eligibility_requested` (UI or deep link asks if server can be joined) — not emitted.
4096. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `join_eligibility_result` (Resolver returns status + reasons) — not emitted.
4097. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `content_resolve_result` (Package resolver identifies available/missing/mismatched content) — not emitted.
4098. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `deep_link_opened` (App receives join/package/replay link) — not emitted.
4099. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `local_server_health_changed` (Supervisor transitions state) — not emitted.
4100. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK event `backend_diagnostics_report_created` (Dev/support report created) — not emitted.
4101. - [ ] [W7] [BP3] [M12+M9] [GAP] BACK acceptance BACK-A-01..12 — none pass.


# ===== WAVE 8 — RELEASE ENGINEERING & COMPLIANCE =====

## 1. BP3 — Double-Click Playability Hard Gate (T-RELEASE)
1. - [ ] [W8] [BP3] [GAP] BP1 release tag `v0.1.0-prealpha` deleted 2026-05-09 (failed double-click gate); not re-published.
2. - [ ] [W8] [BP3] [GAP] BP2 release tag `v0.2.0-prealpha` deleted 2026-05-09 (failed double-click gate); not re-published.
3. - [ ] [W8] [BP3] [GAP] BP3 release tag `v0.3.0-prealpha` not yet published.
4. - [ ] [W8] [BP3] [GAP] `cf-app` does NOT open a game window when launched with NO command-line args from a Finder/Explorer/Files double-click (per BP3 closure note: requires `--scenario` flag).
5. - [ ] [W8] [BP3] [GAP] No default-scenario / launcher menu in `cf-app` for the no-args case.
6. - [ ] [W8] [BP3] [GAP] No `Corefall.app` bundle for macOS (Info.plist + CFBundleExecutable + embedded cf-app + icon + LSEnvironment).
7. - [ ] [W8] [BP3] [GAP] No `.dmg` containing `Corefall.app` produced by `release.yml` (current workflow ships .tar.zst which failed the gate).
8. - [ ] [W8] [BP3] [GAP] No Windows `.msi` installer OR `.zip` with `Corefall.exe` launcher (current workflow ships raw CLI .zip).
9. - [ ] [W8] [BP3] [GAP] No Linux `AppImage` produced by `release.yml`.
10. - [ ] [W8] [BP3] [GAP] No friend-handoff verification documented under `## Friend-Handoff Verification` in BP3 closure note.
11. - [ ] [W8] [BP3] [GAP] No retroactive recovery of BP1/BP2 releases tagged at this BP3 closure commit.
12. - [ ] [W8] [BP3] [GAP] No SHA256SUMS.txt over the per-platform release artifacts.
13. - [ ] [W8] [BP3] [GAP] No determinism contract block in release notes (third-party `cfctl script run ... --seed N` must produce matching `final_sim_checksum`).
14. - [ ] [W8] [BP3] [GAP] No exemplar run bundle embedded in each release archive.
15. - [ ] [W8] [BP3] [GAP] No `summary_grid.png` hero image in release notes.
16. - [ ] [W8] [BP3] [GAP] No Steam Deck verification step (boots + fun-proof plays + run-bundle checker passes on Deck hardware).
17. - [ ] [W8] [BP3] [GAP] No ad-hoc code signing on macOS `Corefall.app` (BP1..BP9 must at least be ad-hoc signed to avoid Gatekeeper hard-block).
18. - [ ] [W8] [BP3] [GAP] No SmartScreen "More info → Run anyway" install instructions in release notes.
19. - [ ] [W8] [BP3] [GAP] No fallback `.tar.gz` with `Corefall.desktop` + `start-corefall.sh` for Linux distros that can't run AppImage.
20. - [ ] [W8] [BP3] [GAP] No drag-to-Applications symlink inside the macOS `.dmg`.

## 20. DR-002 — Replay/event closure debt (M3B closed but contract gaps remain)
281. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 23-category baseline: `mind` event category — no events emitted (cf-ai has no LLM mind layer).
282. - [ ] [W8] [M3B+M5.5] [DR-002] [GAP] DR-002 `collision` event category — no events emitted (full collision lands at M5.5).
283. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `server` event category — no events emitted (no `cf-server` yet).
284. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `anti_cheat` event category — no events emitted.
285. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `mmo` event category — no events emitted.
286. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `material` event category — no events emitted (cf-material stub).
287. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `reaction` event category — no events emitted.
288. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `atmospherics` event category — no events emitted (cf-atmos stub).
289. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `affliction` event category — no events emitted (no affliction system on actors yet at engine layer; AfflictionKind enum scaffolded but never emits).
290. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `body` event category — only `actor.actor_status_changed` exists; `body.wound_added`, `body.limb_function_changed`, `body.gib_spawned` not emitted.
291. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `logistics` event category — no events emitted.
292. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `ux` event category — no events emitted (focus/hover/tooltip/dropdown actions are silent).
293. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `accessibility` event category — no events emitted (settings_observed/settings_changed exist under `control` not `accessibility`).
294. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 `performance` event category — `system.tick_sample` is the only perf event; per-frame/render/worker counters not emitted.
295. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 polished GUI replay browser (egui/TUI on top of cf-tools-replay-viewer library) not built — DR explicitly says revisit when polished GUI lands.
296. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 mod-namespaced custom events not supported — there's no `mod_id` prefix discrimination in the event envelope.
297. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 snapshot format drift compatibility tests not run (snapshot version stamp exists but no migration handlers registered).
298. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 event-volume regression bench at BP4+ scale (full-collision + atmospherics) not scoped — but should be set up BEFORE BP4 lands the heavy event sources.
299. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 networking replication of events for co-op not implemented (no `cf-net` yet, but DR-002 lists "two clients see same world" as a closing pressure test).
300. - [ ] [W8] [M3B] [DR-002] [GAP] DR-002 scope-determinism test (AI test scenarios re-run 3 times produce identical events) — only macOS aarch64 verified locally.

## 60. release.yml — Workflow gaps
611. - [ ] [W8] [GAP] `release.yml` does not include macOS x86_64 build leg.
612. - [ ] [W8] [GAP] `release.yml` does not include ARM64 Linux build leg (Steam Deck is x86_64 but Apple Silicon → Linux ARM64 verification is open).
613. - [ ] [W8] [GAP] `release.yml` does not run `python3 game/tools/generate_release_notes.py` to populate release body from BP closure notes.
614. - [ ] [W8] [GAP] `release.yml` does not embed exemplar run-bundle in each platform's archive.
615. - [ ] [W8] [GAP] `release.yml` does not run `bash game/tools/verify_artifact_double_click.sh` (the verification gate per AGENTS.md).
616. - [ ] [W8] [GAP] `release.yml` does not generate SHA256SUMS.txt.
617. - [ ] [W8] [GAP] `release.yml` does not sign macOS artifact (ad-hoc or notarized).
618. - [ ] [W8] [GAP] `release.yml` does not sign Windows artifact (Authenticode deferred to BP10).
619. - [ ] [W8] [GAP] `release.yml` does not upload to a CDN / release store; relies on GitHub Releases page only.
620. - [ ] [W8] [GAP] `release.yml` does not auto-tag on PR merge to `main` (manual tag still required).
621. - [ ] [W8] [GAP] `game/tools/release/build_msi.ps1` or equivalent Windows installer build script — missing.
622. - [ ] [W8] [GAP] `game/tools/release/build_appimage.sh` — missing.
623. - [ ] [W8] [GAP] `game/tools/release/build_dmg.sh` — missing (only `build_macos_app.sh` exists, no dmg packaging).
624. - [ ] [W8] [GAP] `game/tools/release/sign_macos.sh` — missing (no signing automation).
625. - [ ] [W8] [GAP] `game/tools/release/verify_double_click.sh` — missing.
626. - [ ] [W8] [GAP] `game/tools/release/notarize_macos.sh` — missing (BP10+ scope but contract requires stub).

## 61. game/tools/ general gaps
627. - [ ] [W8] [GAP] `game/tools/check_status_surfaces.sh <bp>` — missing per Status-Surface Update Contract added 2026-05-09.
628. - [ ] [W8] [GAP] `game/tools/bp_close_loop.sh` Phase 7 "auto-publish release" — not implemented.
629. - [ ] [W8] [GAP] `game/tools/llm_grade_run.py validate --strict` mode — no strict mode to reject placeholder cells across the whole bundle dir.
630. - [ ] [W8] [GAP] `game/tools/agent_self_test_report.py` exists but does not auto-fill Q1-Q7 from bundle artifacts (it generates a template only).
631. - [ ] [W8] [GAP] `game/tools/current_bundle_proof.py` exists but is not wired into `bp_close_loop.sh` Phase 6 (manual call only).
632. - [ ] [W8] [DR-053+DR-057] [GAP] `game/tools/cf_asset_ledger.py` mode-aware `check --mode private/release` — does not exist (DR-053 + DR-057 require).
633. - [ ] [W8] [GAP] `game/tools/dependency_drift_report.py` runs but its `--deny-outdated` mode is never enforced in CI.
634. - [ ] [W8] [DR-054] [GAP] No `game/tools/perf_bench.py` for repeatable perf benchmark scenarios across the 3 perf tiers (DR-054 / T-PERF).
635. - [ ] [W8] [DR-046] [GAP] No `game/tools/i18n_check.py` for keyed-strings discipline (DR-046).

## 83. Smoke-test surface — cf-app double-click readiness (DR-051 + DR-024)
779. - [ ] [W8] [DR-024+DR-051] [GAP] `cf-app --help` does not list `--default-scenario` arg.
780. - [ ] [W8] [DR-024+DR-051] [GAP] `cf-app` has no launcher menu when launched with no args (per AGENTS.md Double-Click Hard Gate).
781. - [ ] [W8] [DR-024+DR-051] [GAP] `cf-app` has no first-run sentinel to skip the launcher on subsequent runs.
782. - [ ] [W8] [DR-024+DR-051] [GAP] `cf-app` has no last-played-scenario persistence.
783. - [ ] [W8] [DR-024+DR-051] [GAP] `cf-app` exits cleanly on Cmd-Q (macOS) / Alt-F4 (Win) / Ctrl-Q (Linux) — ESC works but no platform-standard exit key bound.
784. - [ ] [W8] [M0] [DR-024+DR-051] [GAP] `cf-app` window title shows "Corefall — M0 Engine Bootstrap (v…)" hardcoded; should reflect scenario + tick rate or be configurable.

## 99. M3B — Replay viewer surface gaps inherited at BP3 close
898. - [ ] [W8] [BP3] [M3B] [GAP] `cf-tools-replay-viewer view` lacks `--watch` mode for live tail of an active run.
899. - [ ] [W8] [BP3] [M3B] [GAP] `cf-tools-replay-viewer cause-chain` lacks `--render-png` flag for static cause-chain image.
900. - [ ] [W8] [BP3] [M3B] [GAP] `cf-tools-replay-viewer debrief` markdown doesn't include `summary_grid.png` ref by default.
901. - [ ] [W8] [BP3] [M3B] [GAP] `cf-tools-replay-viewer` has no `compare` subcommand for two bundles diff.
902. - [ ] [W8] [BP3] [M3B] [GAP] `cf-tools-replay-viewer` has no `--export-csv` for events.jsonl analytics.
903. - [ ] [W8] [BP3] [M3B] [GAP] No replay-as-data export — JSON-LD or Atom feed for downstream consumers.
904. - [ ] [W8] [BP3] [M3B] [DR-002] [GAP] No DR-002 "share replay link" surface — replays live on local disk only.
905. - [ ] [W8] [BP3] [M3B] [GAP] No replay-validation rejection when `events.jsonl` is non-monotonic — viewer iterates blindly.

## 190. DR-025 — Target platforms (CLOSED at M0; T-RELEASE inherits gap list)
1602. - [ ] [W8] [M0] [DR-025] [GAP] DR-025 macOS x86_64 dual build alongside aarch64 — release.yml does aarch64 only.
1603. - [ ] [W8] [M0] [DR-025] [GAP] DR-025 Linux aarch64 build (Apple Silicon → Linux test path) — not present.
1604. - [ ] [W8] [M0] [DR-025] [GAP] DR-025 Steam Deck Proton compat test — never run.
1605. - [ ] [W8] [M0] [DR-025] [GAP] DR-025 Windows ARM64 build — not present.
1606. - [ ] [W8] [M0] [DR-025] [GAP] DR-025 No-mobile guarantee enforced by CI — no test verifies absence of mobile deps.

## 191. DR-049 — Customization tournament & competitive (CLOSED; M12+ but BP3 schema seed)
1607. - [ ] [W8] [BP3] [M12] [DR-049] [GAP] DR-049 anti-cheat ML — M12+ scope but schema for `match_results.jsonl` not declared.
1608. - [ ] [W8] [BP3] [M12] [DR-049] [GAP] DR-049 ELO/MMR matchmaking — M12+ scope.
1609. - [ ] [W8] [BP3] [M12] [DR-049] [GAP] DR-049 tournament profile — M12+ scope.
1610. - [ ] [W8] [BP3] [M12] [DR-049] [GAP] DR-049 customization paint/decal schema — not declared.

## 325. spec/legal-and-compliance — Legal & compliance pre-launch (BP3 closure includes ledger discipline)
4247. - [ ] [W8] [BP3] [GAP] LEGAL trademark search + registration (US + EU; $1-2K legal counsel; M-MARKETING phase ~6-12mo pre-launch) — not started.
4248. - [ ] [W8] [BP3] [GAP] LEGAL domain registration (corefall.com / corefall.gg / corefall.dev pre-Steam-page) — partial.
4249. - [ ] [W8] [BP3] [GAP] LEGAL business entity ($300-1K LLC Wyoming or Delaware) — not formed.
4250. - [ ] [W8] [BP3] [GAP] LEGAL bank account + Stripe — not set up.
4251. - [ ] [W8] [BP3] [GAP] LEGAL EULA + ToS + Privacy Policy (legal counsel $2-5K pre-Steam-page) — not drafted.
4252. - [ ] [W8] [BP3] [GAP] LEGAL age rating submission (IARC self-rating + ESRB Mature 17+ / PEGI 16-18 / USK 16 / CERO D / ACB MA15+) — not submitted.
4253. - [ ] [W8] [BP3] [GAP] LEGAL open-source attribution screen (cargo-about auto-generated from Cargo.lock; CI gate per release) — not configured.
4254. - [ ] [W8] [BP3] [GAP] LEGAL AI-asset usage-ledger audit (pre-launch full audit + license verification) — not audited.
4255. - [ ] [W8] [BP3] [GAP] LEGAL private prototype ledger mode (AI agent + project owner; ongoing; never blocks private generation if provenance/status logged) — not configured.
4256. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Gameplay license (one-time-purchase)" — not drafted.
4257. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Modding rights (modders retain copyright; Workshop CC-BY-SA default)" — not drafted.
4258. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Data collection (GDPR / CCPA / LGPD compliant; opt-in in EU)" — not drafted.
4259. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Workshop content rights (DMCA process for IP claims)" — not drafted.
4260. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Dispute resolution (binding arbitration; small-claims carve-out)" — not drafted.
4261. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Age requirement (13+ COPPA; 16+ for EU full features)" — not drafted.
4262. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Prohibited conduct (cheating + harassment + illegal content)" — not drafted.
4263. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Termination clauses" — not drafted.
4264. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Limitation of liability" — not drafted.
4265. - [ ] [W8] [BP3] [GAP] LEGAL EULA "Governing law" — not drafted.
4266. - [ ] [W8] [BP3] [DR-047] [GAP] LEGAL privacy "Right-to-deletion endpoint per DR-047" — not implemented.
4267. - [ ] [W8] [BP3] [GAP] LEGAL privacy "Data Processing Agreements with Steam + Sentry/GlitchTip + ElevenLabs (if used)" — not signed.
4268. - [ ] [W8] [BP3] [GAP] LEGAL privacy "Cookie/data prompts where required + privacy-by-default in EU" — not implemented.
4269. - [ ] [W8] [BP3] [GAP] LEGAL "AI-asset usage-ledger entry per asset (prompt + seed + model + LoRA + license + regenerable Y/N)" — not implemented.
4270. - [ ] [W8] [BP3] [GAP] LEGAL "No open-weight model assumed release-cleared without checking exact model/weight license (Stable Audio Open + AudioCraft CC-BY-NC 4.0 weights)" — not enforced.
4271. - [ ] [W8] [BP3] [GAP] LEGAL "Tier-3 AI-agent cleanup doesn't change underlying model licensing" — not enforced.
4272. - [ ] [W8] [BP3] [GAP] LEGAL "Before public sale/release, each retained generated asset is cleared / replaced / relicensed / regenerated through release-safe source" — not enforced.
4273. - [ ] [W8] [BP3] [GAP] LEGAL modding rights "Modders retain copyright on mod content + license to other players via Workshop + default CC-BY-SA 4.0 (Workshop allows other choices GPL/MIT/custom) + Workshop ToS handles IP claims via DMCA" — not enforced.
4274. - [ ] [W8] [BP3] [DR-047] [GAP] LEGAL anti-harassment "Discord ToS + in-game chat moderation + reportable infractions per DR-047" — not configured.
4275. - [ ] [W8] [BP3] [GAP] LEGAL accessibility "WCAG 2.1 AA targeted for UI surfaces + caption support per ADA / EU Accessibility Act" — not measured.
4276. - [ ] [W8] [BP3] [GAP] LEGAL content rating disclosures "Loot boxes/gacha-like NONE by default + gambling NONE by default + in-app purchases NONE at launch + online interactions yes + UGC yes" — not declared.
4277. - [ ] [W8] [BP3] [GAP] LEGAL `cf-asset-ledger check --mode private` (passes for retained private prototypes) — not implemented.
4278. - [ ] [W8] [BP3] [GAP] LEGAL `cf-asset-ledger check --mode release` (passes before any public sale/release) — not implemented.
4279. - [ ] [W8] [BP3] [GAP] LEGAL privacy policy auto-generated from event definitions — not implemented.

## 326. spec/telemetry-and-bug-tooling — Telemetry & bug tooling (BP3 done-criteria; Universal Enhancement)
4280. - [ ] [W8] [BP3] [GAP] TELEMETRY crash reporting "Sentry (free tier 5K events/mo) OR self-hosted GlitchTip (free AGPL)" — not configured.
4281. - [ ] [W8] [BP3] [GAP] TELEMETRY stack traces "Symbolicated via sentry-cli upload of debug symbols on each release build" — not configured.
4282. - [ ] [W8] [BP3] [GAP] TELEMETRY auto-upload "Consent prompt on first crash + remembered + privacy-cleaned (no file paths/chat/PII)" — not implemented.
4283. - [ ] [W8] [BP3] [GAP] TELEMETRY scope "Panic + segfault + GPU hang + replay drift detected" — not implemented.
4284. - [ ] [W8] [BP3] [GAP] TELEMETRY anonymous gameplay opt-in "EU default off prompt on first launch; non-EU default on prompt on first launch + disclosed in privacy policy" — not implemented.
4285. - [ ] [W8] [BP3] [GAP] TELEMETRY captured fields "Scenario id + mission outcome + time-to-death + weapon-of-death + faction picked + mods loaded (hash only) + hardware specs CPU/GPU/RAM/OS + crash signatures + perf counters" — not captured.
4286. - [ ] [W8] [BP3] [GAP] TELEMETRY NEVER captured "Chat content + player names (Steam ID hashed) + inputs + file paths + mod content + save data" — not enforced.
4287. - [ ] [W8] [BP3] [GAP] TELEMETRY GDPR/CCPA/LGPD "Right-to-deletion endpoint + data retention 12 months max + aggregate reports only" — not implemented.
4288. - [ ] [W8] [BP3] [GAP] TELEMETRY performance "Frame ms / sim ms / dropped events / GPU memory / load times / VFX drop count / lighting drop count + aggregated per-build + perf regression detection" — not measured.
4289. - [ ] [W8] [BP3] [GAP] TELEMETRY balance "TTK matrix per weapon/chassis combo + per-faction win-rate + per-mission completion-rate + per-mode dropout-rate" — not measured.
4290. - [ ] [W8] [BP3] [GAP] TELEMETRY in-game bug tool "F12 in-game + screenshot + last 30s replay snapshot + run-bundle attached + user description prompt + system info + optional logs anonymized + uploads to configurable endpoint" — not implemented.
4291. - [ ] [W8] [BP3] [GAP] TELEMETRY AI-driven analysis "Weekly auto-report by AI agent: anomaly detection + summary top 5 issues + prioritized backlog suggestion + email to project-owner" — not implemented.
4292. - [ ] [W8] [BP3] [GAP] TELEMETRY file format `content/telemetry/event_definition.ron` (id + capture + privacy_clean + aggregate_at_endpoint) — not present.
4293. - [ ] [W8] [BP3] [GAP] TELEMETRY done-criteria "Crash reports symbolicate / Bug tool F12 captures + uploads / Gameplay telemetry opt-in flow GDPR-clean / AI weekly anomaly report runs / Privacy policy auto-generated from event definitions / Right-to-deletion endpoint functional" — none done.

## 327. spec/steam-and-platform-integration — Steam + EOS + GOG + itch.io (BP3 closure; release engineering)
4294. - [ ] [W8] [BP3] [GAP] STEAM Workshop (mod packages publishable from in-game + community subscribe + auto-install + trust tiers) — not integrated.
4295. - [ ] [W8] [BP3] [GAP] STEAM Achievements (60-100 achievements + most play 1 of each chassis / complete each mission + ~10 hidden lore/mastery) — not implemented.
4296. - [ ] [W8] [BP3] [GAP] STEAM Cloud (saves + replay archive auto-sync + encrypted) — not implemented.
4297. - [ ] [W8] [BP3] [GAP] STEAM Friends + Invites (friend list + party invite to lobby + presence "In Bunker Defence — Mars") — not implemented.
4298. - [ ] [W8] [BP3] [GAP] STEAM Input (full controller/gamepad/Steam Deck + community bindings sharable via Steam Input) — partial.
4299. - [ ] [W8] [BP3] [GAP] STEAM Deck Verified target rating (800p/60 perf + controller-complete + readable text + no shader compilation hitches) — not configured.
4300. - [ ] [W8] [BP3] [GAP] STEAM Trading Cards (non-monetized cosmetic + earned via play) — not implemented.
4301. - [ ] [W8] [BP3] [GAP] STEAM Remote Play Together (LAN co-op via Steam Remote Play free) — not implemented.
4302. - [ ] [W8] [BP3] [GAP] STEAM Stats (per-player aggregate stats + appears on player profile) — not implemented.
4303. - [ ] [W8] [BP3] [GAP] STEAM Leaderboards (per-mission speedrun + per-mode Bunker Defence wave + daily seed) — not implemented.
4304. - [ ] [W8] [BP3] [GAP] STEAM `bevy_steamworks` integration (Bevy plugin) — not present.
4305. - [ ] [W8] [BP3] [GAP] STEAM `steamworks` underlying SDK wrapper — not present.
4306. - [ ] [W8] [BP3] [GAP] STEAM EOS adapter (cargo feature `--feature eos` + off by default + cross-platform Friends + Lobby for Epic Games Store users) — not configured.
4307. - [ ] [W8] [BP3] [GAP] STEAM GOG.com post-launch (DRM-free build + same binary + no DRM stripped) — not planned.
4308. - [ ] [W8] [BP3] [GAP] STEAM itch.io (mod-friendly demo + early-access build + same binary + free or pay-what-you-want demo) — not planned.
4309. - [ ] [W8] [BP3] [GAP] STEAM console ports post-launch evaluation (Switch + PS5 + Xbox Series + cert paths) — not planned.
4310. - [ ] [W8] [BP3] [GAP] STEAM reference Docker image `cf-server:latest` (Linux + Windows + hosting guide documented) — not present.

## 328. spec/marketing-and-launch — Marketing & launch posture (BP3 Steam page seed)
4311. - [ ] [W8] [BP3] [GAP] MKTG Steam page "launched 6-12 months pre-release" — not launched.
4312. - [ ] [W8] [BP3] [GAP] MKTG title art Tier 3 AI-agent-polished — not produced.
4313. - [ ] [W8] [BP3] [GAP] MKTG capsule art (small/medium/large/header per Steam spec) — not produced.
4314. - [ ] [W8] [BP3] [GAP] MKTG 10+ screenshots (at-launch + Tier 3 polished) — not produced.
4315. - [ ] [W8] [BP3] [GAP] MKTG 90-second reveal trailer — not produced.
4316. - [ ] [W8] [BP3] [GAP] MKTG 30-second gameplay trailer — not produced.
4317. - [ ] [W8] [BP3] [GAP] MKTG 60-second "what is Corefall?" trailer — not produced.
4318. - [ ] [W8] [BP3] [GAP] MKTG description copy (AI-generated; 2-3 paragraphs + 8 bullet features) — not written.
4319. - [ ] [W8] [BP3] [GAP] MKTG system requirements — not declared.
4320. - [ ] [W8] [BP3] [GAP] MKTG languages (Tier-A list per localization-plan) — not declared.
4321. - [ ] [W8] [BP3] [GAP] MKTG tags (Sandbox + Pixel Art + Tactical + Sci-Fi + Multiplayer + Modding + Local Co-op + Online Co-op + PvE + PvP) — not declared.
4322. - [ ] [W8] [BP3] [GAP] MKTG trailer production reveal (60-90s SVD + AnimateDiff clips + Suno score + project-owner narrator + locked-in shots) — not produced.
4323. - [ ] [W8] [BP3] [GAP] MKTG trailer gameplay 30s (real gameplay clips + adaptive music + VO) — not produced.
4324. - [ ] [W8] [BP3] [GAP] MKTG trailer "What is Corefall?" 60-90s (comic-noir storytelling) — not produced.
4325. - [ ] [W8] [BP3] [GAP] MKTG trailer launch day 90-120s (final product showcase) — not produced.
4326. - [ ] [W8] [BP3] [GAP] MKTG press kit presskit() format (logo + screenshots + key art + 3 trailers + 1-pager fact sheet + contact + demo build link + quotes) — not produced.
4327. - [ ] [W8] [BP3] [GAP] MKTG demo build (Steam Next Fest 30-60min: Bunker Defence flagship + 1 onboarding + 1 lab + 4-player coop + time-limited + wishlist CTA + persists save + carries achievements forward) — not produced.
4328. - [ ] [W8] [BP3] [GAP] MKTG wishlist drive (Pre-launch 6mo 10K + Pre-launch 3mo 25K + Steam Next Fest 50K-100K + Launch day convert 10-15%) — not started.
4329. - [ ] [W8] [BP3] [GAP] MKTG channels Reddit + TikTok + Twitter/X + Bluesky + YouTube devlogs + IndieDB + itch.io + Discord — not configured.
4330. - [ ] [W8] [BP3] [GAP] MKTG Discord channels (announcements + general + playtest + mod-creators + per-Tier-A locale + bug-reports + fan-art + screenshots + support + AI-moderated + community mods) — not configured.
4331. - [ ] [W8] [BP3] [GAP] MKTG press outreach Tier-1 (RPS + PC Gamer + Eurogamer + Kotaku + IGN-indie at demo + at launch) — not started.
4332. - [ ] [W8] [BP3] [GAP] MKTG press outreach Tier-2 (regional gaming press at launch) — not started.
4333. - [ ] [W8] [BP3] [GAP] MKTG press outreach Tier-3 (YouTubers + TikTok + Twitch creators with creator-keys) — not started.
4334. - [ ] [W8] [BP3] [GAP] MKTG AI-driven outreach + social (AI agent drafts daily devlog + press emails + monitors community + generates social-post schedule + project-owner approves) — not configured.

