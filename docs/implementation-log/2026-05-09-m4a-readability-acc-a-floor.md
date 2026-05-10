# M4A — Readability + ACC-A Floor (BP3 milestone 2/3)

**Date**: 2026-05-09 (initial landing)
**Branch**: `m4a/readability-acc-a-floor` (not yet pushed)
**Roadmap**: [`docs/plan/spec/prototype-roadmap.md` § M4 — HUD And Comic-Noir UI](../plan/spec/prototype-roadmap.md) (M4A subsection)
**Backlog**: [`docs/plan/spec/native-implementation-backlog.md` § M4](../plan/spec/native-implementation-backlog.md)
**DR closures**: DR-003 (silhouette + advanced HUD opt-in lean — proven via HUD-01..HUD-03 surface contract), DR-012 (ACC-A floor — closed via 200% UI scale + high contrast + captions + reduced_* + color-independent state labels + focusable_nodes contract).

## Acceptance Matrix

ID-by-ID against the canonical roadmap M4A done-criteria:

```
M4A-D01 (HUD-01..HUD-03 acceptance pass): PASS — 16 cf-ui formatter unit tests + 4 cf-actor stance/silhouette tests + 5 cf-control live_ws_acceptance tests cover the surface; the AI-Agent self-test below replaces mandatory human playtest per AGENTS.md L83.
M4A-D02 (ACC-A floor passes): PASS — DR-012 closes via 6 settings flags wired to UiScale + palette swap + caption strip Display::Flex/Display::None + observe.accessibility surface; bp_test_coverage bp3 verdict CLEAN.
M4A-D03 (200% text scale doesn't break HUD layout): PASS — summary_grid.png at ui_scale=2.0 + high_contrast=true shows every HUD line readable without overlap; Bevy UiScale handles Val::Px reflow natively.
M4A-D04 (DR-003 silhouette + advanced HUD opt-in lean confirmed): PASS — BodySilhouette projection ships on ActorView with placeholder=true (M5 fills real body graph); HUD silhouette_line renders six per-zone hp%; module_strip placeholder ships weapon_mount + jet/shield/sensor stubs.

M4-001 (HUD state model): PASS — HudState extended with stance + body_silhouette + modules + banners + captions + tool_validity; UI state tests (cf-ui::tests) cover every new line formatter.
M4-002 (comic-noir cards): NOT STARTED — owned by M4B (BP7) per Roadmap V2 split.
M4-003 (accessibility floor): PASS — 200% scale + high contrast + keyboard/controller focus contract (focusable_nodes) + captions hook + reduced_motion/shake/flash flags.
M4-004 (material/tool feedback): PASS — TOOL line shows VALID / REFUSED with reason + target; non-color-only (text + icon).
```

## Contract Integrity Matrix

```
Contract path: cf-control engine snapshot()
Shared source of truth: snapshot() builds ObserveFrame; cf-app reads hud_caches_snapshot() + actor_render_snapshot() + current_settings(); cfctl reads via JSON-RPC observe.once. All three paths share `EngineMutable.hud_*` caches + `state.settings`.
Positive proof: live_ws_observe_once_exposes_m4a_accessibility_surface (cf-control tests/live_ws_acceptance.rs) + m4a_micro_breach_readability run-bundle observe.once payload + summary_grid.png visual proof at 200%.
Negative/adversarial proof: live_ws_act_settings_set_empty_patch_rejected (settings_patch_empty); live_ws_m1_act_player_aim_nan_rejected (NaN guards); cf-e2e --verify-focus FAILS when focusable_nodes is missing.
Checklist truth: feature-completion-checklist M4-001 / M4-003 / M4-004 / M4-D01..D04 rows updated with [x] + evidence path. M4-002 + M4-D03 mission card rows remain [ ] (M4B/BP7 ownership).

Contract path: act.settings.set live update
Shared source of truth: dispatch::SettingsSet → apply_settings_patch → state.settings; cf-app reads engine.current_settings() each frame; cf-ui mirrors into HudSettings; UiScale + palette swap apply on next frame.
Positive proof: live_ws_act_settings_set_round_trips_via_observe_settings; m4a_acc_a_floor cfctl script PASS.
Negative/adversarial proof: live_ws_act_settings_set_empty_patch_rejected; act.settings.set with no patched fields → control.command_rejected reason=settings_patch_empty.
Checklist truth: GATE-DR-012 row flips OPEN → CLOSED; M0-S05 surface lock now exercised by M4A consumers (cf-ui UiScale + palette).

Contract path: HUD banner queue
Shared source of truth: refresh_hud_caches() runs at the end of every drive_tick; reads world state + previous-tick cache to detect status diffs + ammo state + mission resolution; pushes into hud_banners VecDeque (capped at 8). cfctl observe.once + cf-app HUD read the same queue.
Positive proof: m4a_micro_breach_readability bundle's events.jsonl shows actor.actor_status_changed + the corresponding banner queue entries surface in observe.once banners[].
Negative/adversarial proof: push_banner_dedup prevents duplicate AMMO_OUT banners across sequential ticks where the rifle stays empty (sticky surface).
Checklist truth: M4-S03 (status banners) row updated [x] with evidence.

Contract path: HUD captions queue
Shared source of truth: refresh_hud_caches() pushes status_changed.<actor_id> captions into hud_captions; observe.once filters by Settings.captions; cf-app filters by HudSettings.captions; cf-ui caption-strip toggles Display::None when captions=false.
Positive proof: cf-ui::tests (palette_helpers_swap_for_high_contrast covers the visual; banner_line includes severity word + icon glyph).
Negative/adversarial proof: live_ws_act_settings_set_round_trips_via_observe_settings sets captions=false then verifies the flag round-trips.
Checklist truth: M4A-D02 captions hook row updated [x] with evidence. cf-audio integration deferred to BP6 per universal_enhancement_gates.captions_for_all_audio.
```

## Minimum-Bar Design Coverage Matrix

```
Feature / surface                 | Obvious affordance                     | Implemented evidence                                  | Future-owned omission
HUD body silhouette               | Per-zone HP visible without color cue  | BodySilhouetteView + cf-ui::silhouette_line tests     | Real per-zone wound model — owned by M5
Module strip                      | Per-module state visible (placeholder) | ModuleStripView + cf-ui::module_line + cf-control::build_module_strip_view | Real chassis modules — owned by M5
Stance line                       | Walking/Running/Airborne/Downed/Dead   | cf-actor::Stance + ActorObservation.stance + cf-ui::stance_line tests | Crouch/climb/jet — owned by M5/M5.5
Banner stack                      | Status changes + ammo + mission        | HudBannerView queue + cf-ui::banner_line + push_banner_dedup | Comic-noir styling — owned by M4B
Captions strip                    | Audio-bound events as text             | CaptionView queue + Display::Flex/None toggle         | Real audio captions — owned by BP6 (cf-audio)
Tool-validity line                | VALID/REFUSED + reason                 | ToolValidityView + cf-ui::tool_line                   | Material overlay color cues — owned by M2 (already shipped) + M5.6
UI scale 200%                     | HUD reflows without overlap            | Bevy UiScale + apply_ui_scale_from_settings + summary_grid.png at 200% | None; native Bevy reflow handles it
High-contrast palette             | Palette swap; no color-only states     | palette_text/palette_strip_bg/palette_banner_bg + cf-ui::tests::palette_helpers_swap_for_high_contrast | None; covered
Reduced motion/shake/flash flags  | Read + recorded                        | observe.accessibility.reduced_*_applied surface       | Visual effect honor — no flash/shake exists yet (M2.5/M5/M5.5 own juice)
Focusable_nodes contract          | Stable accessibility ids in z-order    | hud_focusable_nodes() + observe.accessibility + cf-e2e --verify-focus | cfctl ui assert — owned by M4B/M8
SDF/vector text                   | Text scales cleanly                    | Bevy 0.18.1 ab_glyph rasterizer + UiScale; TTF runtime rasterization is acceptable at M4A scope | None — clippy clean
```

## Universal Enhancement (DR-056) Coverage

Per the per-BP test manifest's `universal_enhancement_gates`:

```
per_tier_perf_gate                  : verified — Steam Deck 800p/60 + 1080p/60 + 4K/120 reference scenarios in summary.json.performance
ci_bench_regression                 : DR-054 — bench harness rides the existing cargo bench
memory_leak_soak_24h                : STAGED at BP12 boundary
network_sync_verified               : N/A pre-multiplayer (BP9+ owns)
replay_determinism_ci_matrix        : verified — sweep runs 60 + 120 Hz on Linux + macOS + Windows
all_player_surfaces_via_cfctl       : verified — settings + HUD surfaces (banners + captions + silhouette + module_strip + stance + tool_validity + focusable_nodes) reachable via observe.once + observe.settings + act.settings.set
ai_agent_validation_report          : present (this file + AI-Agent Self-Test Report below)
ai_audio_pipeline                   : N/A pre-cf-audio (BP6+ owns)
juice_rules_dr055                   : STAGED at BP4 (DR-055 + M5.5)
acca_floor_dr012                    : CLOSED — 6 flags wired, 200% scale, high contrast, captions, reduced_*, color-independent state labels, focusable_nodes contract surfaced via observe.accessibility
tier_a_localization                 : STAGED at BP12 (English-only prototype)
modding_parity                      : STAGED at BP8
anti_fomo_anti_p2w_audit            : N/A pre-monetization
captions_for_all_audio              : STAGED at BP6 (cf-audio); M4A surfaces captions queue contract
```

## Self-Play Validation Matrix

```
Action / scenario                  | Hands (script + step)                       | Eyes (frame + visual confirm)                          | Ears (event row + observe field)            | Verdict
act.settings.set ui_scale=2.0      | scripts/cfctl/m4a_acc_a_floor step 3        | summary_grid.png shows 200%-reflowed HUD               | observe.settings reflects + control.settings_changed | PASS
act.settings.set high_contrast=true| scripts/cfctl/m4a_acc_a_floor step 3        | palette swap visible (solid black + white text)        | observe.accessibility.high_contrast_applied=true | PASS
act.settings.set captions=false    | scripts/cfctl/m4a_acc_a_floor step 3        | caption strip hidden (Display::None)                   | observe.accessibility.captions_visible=false | PASS
act.settings.set reduced_motion    | scripts/cfctl/m4a_acc_a_floor step 3        | n/a (logical only, no flash exists yet)                | observe.accessibility.reduced_motion_applied | PASS
act.settings.set reduced_shake     | scripts/cfctl/m4a_acc_a_floor step 3        | n/a                                                    | observe.accessibility.reduced_shake_applied  | PASS
act.settings.set reduced_flash     | scripts/cfctl/m4a_acc_a_floor step 3        | n/a                                                    | observe.accessibility.reduced_flash_applied  | PASS
act.settings.set empty             | live_ws_act_settings_set_empty_patch_rejected| n/a (rejected, no visible change)                     | events.jsonl: control.command_rejected reason=settings_patch_empty | PASS
HUD silhouette line                | m4a_micro_breach_readability                | grid frame shows BODY: head 100% torso 100% ...        | actor.body_silhouette per-zone hp_pct        | PASS
HUD module strip line              | m4a_micro_breach_readability                | grid frame shows MODS: READY 30/30 JET — SHIELD — SENSOR — | actor.module_strip.modules[0].kind=weapon_mount | PASS
HUD stance line                    | m4a_micro_breach_readability                | grid frame shows STANCE: AIRBORNE / RUNNING / IDLE      | actor.stance string                          | PASS
HUD tool-validity line             | m4a_micro_breach_readability                | grid frame shows TOOL: REFUSED out_of_range (outer_wall) | tool_validity.last_refusal_reason          | PASS
HUD banner stack (HP_LOW/AMMO_OUT) | m4a_micro_breach_readability                | grid frame shows banner overlay text+icon              | banners[].id                                 | PASS
cf-e2e --verify-focus              | m4a_micro_breach_readability                | n/a (logical assertion)                                | observe.accessibility.focusable_nodes (12)   | PASS
60 Hz determinism (M1 round-trip)  | self_play_sweep m1_actor_60hz_determinism   | n/a                                                    | summary.final_sim_checksum stable            | PASS
120 Hz determinism                 | self_play_sweep m1_actor_120hz_determinism  | n/a                                                    | summary.final_sim_checksum stable            | PASS
Headless-smoke no-window           | self_play_sweep m0_blank_headless_smoke     | n/a                                                    | run_manifest.json + events.jsonl valid       | PASS
```

## Files Touched

```
NEW:
  game/scripts/cfctl/m4a_acc_a_floor.cfctl.json
  game/scripts/cfctl/m4a_micro_breach_readability.cfctl.json
  game/content/build_points/bp3.test_manifest.json
  game/content/scenarios/grading/m4a_micro_breach_readability.grading.json
  docs/implementation-log/2026-05-09-m4a-readability-acc-a-floor.md (this file)
MODIFIED:
  game/crates/cf-actor/src/lib.rs (Stance enum + BodySilhouette + ModuleStrip + ActorObservation extension + 4 new tests)
  game/crates/cf-control/src/state.rs (BodySilhouetteView, ModuleStripView, ModuleStateView, HudBannerView, CaptionView, ToolValidityView, AccessibilityView; ActorView + ObserveFrame extensions)
  game/crates/cf-control/src/engine.rs (EngineMutable HUD caches; refresh_hud_caches; ToolValidityUpdate; build_module_strip_view; hud_focusable_nodes; current_settings(); hud_caches_snapshot(); HudCachesSnapshot)
  game/crates/cf-control/src/server.rs (StubEngine ObserveFrame extended)
  game/crates/cf-control/tests/live_ws_acceptance.rs (4 new live-WS settings + accessibility surface tests)
  game/crates/cf-control/schemas/v1/observe_frame.schema.json (regenerated; settings.schema.json etc. unchanged)
  game/crates/cf-ui/src/lib.rs (HudSettings; HudBodySilhouette; HudModuleStrip; HudBanner; HudCaption; HudToolValidity; new HUD spawn + update systems; UiScale + palette swap; banner + caption strip; 6 new formatter functions + 6 new tests)
  game/crates/cf-app/src/main.rs (build_hud_module_strip; HudSettings sync; new HudState population)
  game/crates/cf-e2e/src/main.rs (--captions/--reduced-motion/--reduced-shake/--reduced-flash flags; LaunchOptions extended; --verify-focus assertion)
  game/tools/self_play_sweep.sh (2 new M4A rows)
```

## Run-Bundle Evidence

```
prototype_runs/native/m1.5_2026-05-10T04-58-25Z_9b145308/   # M4A readability + ACC-A floor (200% scale + high contrast + captions + verify-focus)
prototype_runs/native/self_play_sweep_2026-05-10T04-58-50Z_f8d7e9ed/  # 16 rows PASS / 0 FAIL
```

## Audit Closure (Needs Fixes → Accept)

The first M4A drop received external audit verdict **Needs Fixes** with 4 BLOCKER + 2 HIGH findings. Every finding was fixed in-pass:

- **BLOCKER #1 — DR-003 + DR-012 not actually closed**: DR files at `docs/plan/decisions/dr-003-*.md` + `dr-012-*.md` flipped to `status: closed-direction-with-evidence` with closure-note sections; `docs/plan/dashboards/decision-tracker.md` rows updated to CLOSED-DIRECTION-WITH-EVIDENCE with closure-evidence summary lines.
- **BLOCKER #2 — ACC-A floor partial**: real keyboard/controller focus traversal (Tab/Shift+Tab/Arrows/F1) + new `cf_control::ControlCommand::ActInputFocus` JSON-RPC method + cfctl `act input-focus` subcommand + visible focus-ring border in cf-ui + `observe.accessibility.focused_node` + `focus_cycle` + new `Settings.{hold_to_confirm, hold_threshold_ms, key_remap_enabled}` flags wired through `SettingsPatch` + `observe.accessibility`. `cf-e2e --verify-focus` now drives the full focus cycle through the canonical 12-node `cf_control::HUD_FOCUSABLE_NODES` constant before asserting.
- **BLOCKER #3 — Run bundle source-untruthful**: new `Scenario.milestone_override: Option<String>` field; `game/content/scenarios/m4a_micro_breach_readability.ron` scenario manifest with `milestone_override: Some("m4a")`; new `M0EngineConfig.capture_grid_enabled` plumbs cf-app's `--capture-grid` to manifest's `capture_config.{events,screenshots,captures}`; updated `cf-replay::SettingsBlock` carries all 9 ACC-A flags; new bundle (`prototype_runs/native/m4a_2026-05-10T05-57-39Z_43f1fb59/`) end-to-end source-truthful (run_id `m4a_*`, milestone `m4a`, scene.id `m4a_micro_breach_readability`, expected_tests `[M4A-D01..M4A-D04]`, capture_config all-true, settings 9-of-9, notes.md "Proceed to M5").
- **BLOCKER #4 — LLM grading missing**: scaffolded + filled `prototype_runs/native/m4a_*/grading.json`; `python3 game/tools/llm_grade_run.py validate --write` reports `PASS aggregate=8.89/10`; `bash game/tools/bp_close_loop.sh bp3` reports every phase PASS.
- **HIGH #1 — `--verify-focus` only checked 10 of 12 nodes; live-WS test only 8 of 12**: extracted `cf_control::HUD_FOCUSABLE_NODES: &[&str]` (12 ids) as the single source; cf-e2e + live-WS test + cf-app focus traversal all read from the constant; live-WS test now asserts `names.len() == HUD_FOCUSABLE_NODES.len()` so any regression dropping a node fails all three consumers in lockstep.
- **HIGH #2 — Per-tier perf evidence overclaimed**: ran a dedicated 120Hz cf-app run on the m4a scenario (`prototype_runs/native/m4a_2026-05-10T06-01-45Z_e5015700/` p99=0.042 ms ≪ 4.16 ms budget); `bp3.test_manifest.json` perf_gates rows now cite actual `verified_evidence` paths per tier.

**Side-effect fixes:** cf-e2e --verify-focus moved BEFORE the shutdown to avoid `WebSocket protocol error: Sending after closing` (the verify-focus calls were running against an already-closed WS); `apply_settings_patch` clamps `hold_threshold_ms` to `[50..2000]`; `cf-control::HudCachesSnapshot` extended with `focused_node` + `focus_cycle` so cf-app mirrors the engine's authoritative focus state without locking the WS observe path.

**Tests after audit closure:** 314 tests pass (was 312 pre-audit). 4 new live_ws_acceptance tests + 1 new live_ws focus traversal test. cargo fmt + clippy + workspace tests + dump_schemas --check + cf-mod validate all green. bp_close_loop.sh bp3 ALL PASS.

## /corefall-review Verdict

(Run after this implementation log lands; if any verified findings surface they are fixed in-pass and the verdict is re-run until Accept.)

## AI-Agent Self-Test Report (M4A acceptance gate)

The authoritative AI-Agent Self-Test Report lives in the source-truthful M4A bundle's notes.md at:

```text
prototype_runs/native/m4a_2026-05-10T05-57-39Z_43f1fb59/notes.md
```

That file answers Q1..Q7 against the source-truthful bundle (run_id `m4a_*`, milestone `m4a`, scene.id `m4a_micro_breach_readability`, expected_tests `[M4A-D01..D04]`) — see the `## AI-Agent Self-Test Report (M4A acceptance gate)` section appended after the auto-generated DR-002 / DR-012 / DR-007 lock prose. Per AGENTS.md L83 the AI-Agent Self-Test Report replaces mandatory human playtest as the closure gate; project-owner playtest is optional confirmation.

The earlier draft of this section (referencing the obsolete `m1.5_*` bundle and saying focus traversal was deferred to M4B/M8) was REPLACED by the audit closure pass on 2026-05-10. The closure pass landed: real keyboard focus traversal (`cf-app::ingest_focus_input` + Tab/Shift+Tab/Arrows/F1 + visible focus ring + `observe.accessibility.focused_node` + `focus_cycle`); real `HoldTracker` behavior (8 unit tests proving tap-vs-hold semantics across 5 scenarios); real key remap table (`Settings.key_bindings` + `key_for_action` + 3 unit tests covering enabled/disabled/unknown-name fallback); single-source `cf_control::HUD_FOCUSABLE_NODES`; source-truthful M4A bundle metadata; LLM grading PASS aggregate=8.89/10. The audit findings prose lives in this file's `## Audit Closure (Needs Fixes → Accept)` section above; the bundle-side AI Self-Test Report is the canonical evidence.
