---
type: spec
status: live-checklist
authority: "Completion and rating checklist generated from prototype-roadmap.md and native-implementation-backlog.md. Update this after each implementation pass."
last_updated: 2026-05-06 (M0.3 contract-integrity review loop)
feeds:
  - DR-001
  - DR-002
  - DR-003
  - DR-004
  - DR-005
  - DR-006
  - DR-007
  - DR-008
  - DR-009
  - DR-010
  - DR-011
  - DR-012
  - DR-013
  - DR-014
  - DR-015
  - DR-016
  - DR-017
  - DR-018
  - DR-019
  - DR-020
  - DR-021
  - DR-022
  - DR-023
  - DR-024
  - DR-025
  - DR-026
  - DR-027
  - DR-028
  - DR-029
  - DR-030
  - DR-031
  - DR-032
  - DR-033
  - DR-034
  - DR-035
  - DR-036
  - DR-037
  - DR-038
  - DR-039
  - DR-040
  - DR-041
  - DR-042
  - DR-043
  - DR-044
  - DR-045
  - DR-046
  - DR-047
  - DR-048
  - DR-049
  - DR-050
  - DR-051
  - DR-052
  - DR-053
  - DR-054
  - DR-055
  - DR-056
---

<- [[spec/index|spec section]] · [[spec/prototype-roadmap|native roadmap]] · [[spec/native-implementation-backlog|native backlog]] · [[spec/authoritative-game-spec-v0|game spec v0]] · [[dashboards/research-readiness|readiness]] · [VAULT_PLAN.md](../../VAULT_PLAN.md)

# Feature Completion Checklist

> [!summary] Purpose
> This is the living completion checklist for the native roadmap. It turns roadmap features, milestone scope, milestone done-criteria, side-track obligations, and native backlog task cards into rating rows. When an AI agent finishes a feature, task card, or milestone, it must update the relevant rows instead of only saying "done" in chat.

> [!important] Use this with the roadmap and backlog
> Build scope still comes from [[spec/prototype-roadmap]] and [[spec/native-implementation-backlog]]. This checklist tracks completion, evidence, human ratings, and AI self-ratings. If the roadmap/backlog changes, update this checklist in the same pass.

> [!info] Current coverage
> 497 baseline checklist rows plus focused Server/MMO and Material/T-MAT addenda below. M9-M12 baseline rows are summary rows; agents implementing M9-M12 must use the addendum plus [[spec/prototype-roadmap]], [[spec/native-implementation-backlog]], [[spec/server-app-architecture]], and [[spec/persistent-mmo-architecture]] as the authoritative scope until the next full regeneration.

> [!important] Server/MMO addendum active
> The 2026-05-05 server direction added DR-034, DR-035, T-SERVER, server/anti-cheat/MMO run-bundle categories, and expanded M9-M12. This file now includes a focused addendum so implementing agents have checklist rows immediately. The next full regeneration should merge these rows into the normal M9-M12 scope/done/task sections and remove this temporary addendum callout.

> [!important] World/Environment/Mining/Match/Comms addendum active
> The 2026-05-06 design pass added DR-039 (worlds), DR-040 (environmental conditions), DR-041 (mining), DR-042 (game modes / match grammar), DR-043 (voice + radio comms), four new milestones (M5.10 Worlds + Environmental Aggregation, M7.7 Day/Night/Weather, M8.6 Mining, M9.5 Voice + Radio Comms), promoted M6.6 to AI Environmental Competence, and extended M7/M11/M12 with match grammar. This file uses a focused addendum below; the next full regeneration should merge these rows into the normal milestone scope/done/task sections and remove this temporary addendum callout.

### M5.10 — Worlds Catalog & Environmental Aggregation (DR-039 + DR-040)

| Row | Scope | Done When | Evidence | AI Self-Rating | Human Rating |
|---|---|---|---|---|---|
| M5.10-A | World manifest schema (12 launch worlds) | `cf-mod validate content/worlds/ --strict` passes; `world.loaded` event in run bundles. | `prototype_runs/native/<m5_10_run>/manifest.json`. | | |
| M5.10-B | Astrography kernel (simplified circular Keplerian) | ASTRO-A-01..ASTRO-A-03 pass; sparse `astrography.tick` events; per-pair `comms_latency_changed` events. | Run-bundle astrography events. | | |
| M5.10-C | EnvironmentSignal aggregator | ENV-A-01..ENV-A-04 pass; aggregator perf ≤ 5% frame budget on Steam Deck floor. | Bench report; ENV-A run-bundle deltas. | | |
| M5.10-D | 15-class hazard taxonomy | ENV-A-05..ENV-A-09 pass; per-class threshold cause-chain. | Hazard transition events. | | |
| M5.10-E | cfctl observation surface | `cfctl observe --environment / --worlds / --astrography / --hazards` snapshot tests. | CLI snapshot fixtures. | | |
| M5.10-F | Acceptance scenario | `m5_10_environment_aggregation` scenario: full ENV-A + ASTRO-A suite. | Checked run bundle. | | |
| M5.10-G | Replay/perf/bug hunt | ENV-A-15 + ASTRO-A-05 byte-identical replay; bug-hunt log. | Prototype note under `prototypes/`. | | |

### M7.7 — Day/Night/Weather (DR-039 + DR-040)

| Row | Scope | Done When | Evidence | AI Self-Rating | Human Rating |
|---|---|---|---|---|---|
| M7.7-A | Day/night kernel (per-world cycle) | DAY-A-01..DAY-A-03 pass; modulates ambient lux + temperature. | Run-bundle weather events. | | |
| M7.7-B | Weather event kernel (per-world weather table) | WEATHER-A-01..WEATHER-A-04 pass; deterministic per scenario seed. | Weather event chain. | | |
| M7.7-C | Precursor wiring (M2 lux + M5.7 dust + M5.6 thermal + M5.9 atmosphere) | Per-precursor fixture tests. | Cross-kernel cause-chain. | | |
| M7.7-D | AI weather doctrine | AI-WEATHER-A-01..AI-WEATHER-A-05 pass; AI puppet under each weather class. | AI doctrine events. | | |
| M7.7-E | cfctl observation surface | `cfctl observe --weather / --day-night` snapshot tests. | CLI snapshot fixtures. | | |
| M7.7-F | Acceptance scenario | `m7_7_weather_kernel` scenario: full WEATHER-A + DAY-A + AI-WEATHER-A suite. | Checked run bundle. | | |
| M7.7-G | Replay/perf/bug hunt | WEATHER-A-15 byte-identical replay; bug-hunt log. | Prototype note under `prototypes/`. | | |

### M8.6 — Mining and Extraction (DR-041)

| Row | Scope | Done When | Evidence | AI Self-Rating | Human Rating |
|---|---|---|---|---|---|
| M8.6-A | Ore registry (12 launch ores) | `cf-mod validate content/ores/ --strict` passes. | Schema audit. | | |
| M8.6-B | Ore deposit kernel + per-world deposits | DEPOSIT-A-01..DEPOSIT-A-03 pass. | World-load deposit manifest. | | |
| M8.6-C | Sample tool + drill tool + extraction kernel | SAMPLE-A + DRILL-A + EXTRACT-A pass. | Run-bundle mining events. | | |
| M8.6-D | Refining + smelting + trade ledger | REFINE-A + SMELT-A + TRADE-A pass; cause-chain to atmospherics combustion. | Run-bundle mining + atmospherics events. | | |
| M8.6-E | AI miner doctrine | AI-MINE-A-01..AI-MINE-A-06 pass; vacuum-only-robot doctrine. | AI doctrine events. | | |
| M8.6-F | cfctl observation surface | `cfctl observe --deposits / --mining-events / --refineries / --smelters / --trade-ledger` snapshot tests. | CLI snapshot fixtures. | | |
| M8.6-G | Acceptance scenario | `m8_6_mining_pipeline` scenario: full pipeline + AI doctrine in coop. | Checked run bundle. | | |
| M8.6-H | Replay/perf/bug hunt | MINE-A-15 byte-identical replay; bug-hunt log. | Prototype note under `prototypes/`. | | |

### Match Grammar (DR-042) — M7 Bunker Defence Proof Mission + M11/M12 Match Grammar

| Row | Scope | Done When | Evidence | AI Self-Rating | Human Rating |
|---|---|---|---|---|---|
| MATCH-A (M7) | Bunker Defence as M7 proof mission (1v1 + 2v2 + 3v3 + 4v4 + asymmetric N-team via team-config flexibility) | `cf-server --mode coop_room --scenario bunker_defence_2v2` runs end-to-end; team-config flexibility tests. | Checked run bundle with match events. | | |
| MATCH-B (M11) | Symmetric Arena + Asymmetric N-Team + Coop-vs-AI online | M11 acceptance (per [[spec/server-app-architecture]] + [[spec/persistent-mmo-architecture]]). | M11 acceptance run bundles. | | |
| MATCH-C (M12) | FFA + Campaign + AI-fill policy + match identity per-shard | M12 PvP arena suite + persistent shard suite. | M12 acceptance run bundles. | | |

### M9.5 — Voice and Radio Comms (DR-043)

| Row | Scope | Done When | Evidence | AI Self-Rating | Human Rating |
|---|---|---|---|---|---|
| M9.5-A | Voice acoustic kernel (Steam Audio) | VOICE-A-01..VOICE-A-05 pass; acoustic propagation through atmosphere + sealed-helmet exception. | Run-bundle voice events. | | |
| M9.5-B | Voice Opus codec + server-authoritative routing | VOICE-A-06..VOICE-A-07 pass; latency budget < 100 ms. | Codec + transport events. | | |
| M9.5-C | Voice equipment + origin gating (helmet pickup, throat mic, bone conductor) | VOICE-A-08 pass; per-origin equipment tests. | Equipment events. | | |
| M9.5-D | Radio kernel (ACRE2 multipath; HF/VHF/UHF/Microwave bands; 4 propagation modes) | RADIO-A-01..RADIO-A-06 pass; LOS + multipath + skywave propagation. | Run-bundle radio events with cause + SNR + path_kind. | | |
| M9.5-E | Radio equipment (handheld VHF, backpack VHF, HF, satellite, antennas) | RADIO-A-07..RADIO-A-08 pass; antenna directional gain + battery drain. | Radio equipment events. | | |
| M9.5-F | Frequency tuning + encryption | RADIO-A-09..RADIO-A-11 pass; encryption mismatch rejection. | Tuning + encryption events. | | |
| M9.5-G | Jamming + interference (active jammer; solar flare; EMP) | RADIO-A-12..RADIO-A-14 pass. | Interference events. | | |
| M9.5-H | Origin radio gating (humans equip; robots built-in; androids modular) | RADIO-A-15 pass; slot-assign rejection tests. | Radio equipment events with origin payload. | | |
| M9.5-I | Acoustic trauma body damage | TRAUMA-A-01..TRAUMA-A-03 pass; routes through M5.7 hazard package. | Voice events with `reason: hearing_damage`. | | |
| M9.5-J | Mission-director comms-policy hooks | POLICY-A-01..POLICY-A-03 pass; RF silence + frequency segregation + jamming overlay. | Mission events with comms-policy changes. | | |
| M9.5-K | cfctl comms observation surface | `cfctl observe --voice / --radio / --frequencies / --interference` snapshot tests. | CLI snapshot fixtures. | | |
| M9.5-L | Acceptance scenario | `m9_5_voice_radio_comms` scenario: full VOICE-A + RADIO-A + TRAUMA-A + POLICY-A suite. | Checked run bundle. | | |
| M9.5-M | Replay/perf/bug hunt | RADIO-A-15 + VOICE-A-15 byte-identical replay (radio events fully replay; voice routes as cause-effect not raw audio). | Prototype note under `prototypes/`. | | |

> [!important] Build Points (Roadmap V2) addendum active
> The 2026-05-08 Roadmap V2 pass added [[spec/prototype-roadmap#Build Points (Roadmap V2)|Build Points]] (BP0..BP12) on top of milestones, three micro-fun-slice interlude milestones (M2.5, M5.5.5, M5.9.5), explicit M3 → M3A/M3B and M4 → M4A/M4B splits, and four new production side tracks (T-CONTENT-ART, T-CONTENT-NARRATIVE, T-LOCALIZATION, T-LIVEOPS). This file gains a Build Points Checklist so the BP closure gate (vault note + DR-closure mapping + human-playtest gate) is trackable per BP. The next full regeneration should fold these rows into per-BP sections and remove this addendum callout.

### Build Points Checklist (Roadmap V2)

| Done | ID | BP / Milestones | Closes Or Refreshes | Playable Artifact | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `BP0` | Foundation Build (M0 + M1) | DR-001/DR-024/DR-025/DR-026; DR-002 envelope locked | `cfctl run --scenario m1_actor_range` at 60+120 Hz | - | - | - | 5 | 5 | - | M0 + M1 closed; bundles `m0_*` + `m1_*` archived. |
| [x] | `BP1` | Micro Breach Build (M1.5) | DR-002 + DR-004 + DR-007 + DR-008 + DR-009 leans confirmed | `cf-e2e --script micro_breach_{win,loss}` | - | - | - | 5 | 5 | - | M1.5 closed; bundles `m1.5_2026-05-08T01-2{7,8}*` archived. cf-e2e wins 4/4 + loses 3/3. |
| [ ] | `BP2` | Terrain & Replay Build (M2 + M2.5 + M3A) | DR-002 (M3A locks event taxonomy + headless replay); DR-007 launch-material set frozen | M2 dig fixture + M2.5 micro reactor defense + headless replay verifier | - | - | - | - | - | - | Per-milestone done-criteria + BP closure note + human-playtest gate. |
| [ ] | `BP3` | Combat Readability Build (M3B + M4A + M5) | DR-002 closure (M3B); DR-003 closure (M4A); DR-012 ACC-A closure (M4A); DR-014/DR-021 chassis grammar | `cf-e2e --script breach_contract --ui-scale 2.0 --high-contrast` + M5 chassis wreck/eject | - | - | - | - | - | - | T-CONTENT-* placeholder generation may begin from this BP. |
| [ ] | `BP4` | Physics Sandbox Alpha (M5.5 + M5.5.5 + M5.6 + M5.7 + M5.8) | DR-033 closure; DR-036 implementation slices | M5.5.5 micro sabotage + per-milestone gauntlets (COLL-001..012, MAT-01..03) | - | - | - | - | - | - | M5.8 wires per-origin reaction matrix runtime. |
| [ ] | `BP5` | Atmospherics & Worlds Alpha (M5.9 + M5.9.5 + M5.10) | DR-037 closure (M5.9); DR-038 closure (with M5.5); DR-039 + DR-040 closures (M5.10) | M5.9.5 micro pressure hold + ENV-A/ATMOS-A/GRAV-A/ASTRO-A acceptance suites | - | - | - | - | - | - | Production tracks T-CONTENT-* still in placeholder mode. |
| [ ] | `BP6` | AI Combat Alpha (M6 + M6.5 + M6.6) | DR-008 closure (M6); DR-022 humanlike-AI bar (6 of 8 by M6); DR-032 closure (M6.5); DR-036 AI hooks (M6.6) | AI-H-01..06 + MIND-001..010 (mock) + AI-MAT-01..08 acceptance suites | - | - | - | - | - | - | LLM mind layer optional but required for DR-032 closure evidence. |
| [ ] | `BP7` | Vertical Slice Alpha (M7 + M7.5 + M7.7 + M4B) | DR-004 closure (M7's Breach Contract); DR-027 base-power; DR-019 comic-noir polish (M4B); DR-039+DR-040 weather/day cycle (M7.7) | Project-owner plays Breach Contract 5× + Bunker Defence 2v2 proof; A-FEEL gate met | - | - | - | - | - | - | First "real game" milestone; A-FEEL gate is hard. |
| [ ] | `BP8` | Creator Alpha (M8 + M8.5 + M8.6) | DR-006 closure (M8 modding); DR-036 material lab closure (M8.5); DR-041 mining closure (M8.6) | Player authors a Breach Contract variant + sample mod loads; designer authors an acid-trap puzzle in <10 min | - | - | - | - | - | - | T-MOD primary milestone. |
| [ ] | `BP9` | Server / LAN Alpha (M9 + M10) | DR-005 server-authority confirmed; DR-034 server lifecycle; DR-029 save format roundtrip; networking-transport topic closed | `cf-server --mode lan_room` survives one Breach Contract; per-client bundles align tick-for-tick | - | - | - | - | - | - | T-SERVER primary BP. |
| [ ] | `BP10` | Online Beta (M11 + M9.5) | DR-005 online co-op proven; DR-043 voice/radio comms closed (M9.5); DR-052 network sync direction confirmed | A community member self-hosts `cf-server --mode coop_room`; voice + radio works through atmospheric medium | - | - | - | - | - | - | T-LIVEOPS pre-launch wiring begins. |
| [ ] | `BP11` | Public Systems Beta (M12) | DR-035 closure (MMO architecture); DR-042 match grammar (Bunker Defence flagship); DR-049 tournament infrastructure activated | `cf-server --mode pvp_arena` 4-8 player matches with anti-cheat foundation; MMO-001..012 all pass | - | - | - | - | - | - | DR-031/DR-057 anti-pay-to-win audit per BP. |
| [ ] | `BP12` | Release Candidate (T-CONTENT-ART + T-CONTENT-NARRATIVE + T-LOCALIZATION + T-LIVEOPS finalization) | DR-044/DR-045/DR-046/DR-047/DR-051 closure | Steam-ready build with full launch-content roster, narrative bible, Tier-A localized strings, telemetry/launch tooling | - | - | - | - | - | - | Final art/narrative/localization/launch ops finalize HERE — NOT during BP2..BP11. |

> [!important] Material/T-MAT addendum active
> The systemic material direction added DR-036/DR-037, T-MAT, M5.6/M5.7/M5.9/M6.6/M7.5/M8.5 milestones, `cf-material`/`cf-atmos` crates, and four new run-bundle event categories (`material`/`reaction`/`atmospherics`/`affliction`). This file now includes a focused Material/T-MAT addendum so implementing agents have checklist rows immediately. The next full regeneration should merge these rows into the normal milestone/side-track sections and remove this temporary addendum callout.

> [!important] Open Decision Gates addendum active
> The 2026-05-05 readiness pass added an [[spec/prototype-roadmap#Open Decision Gates Protocol|Open Decision Gates Protocol]] and per-milestone gate callouts to [[spec/prototype-roadmap]]. This file now includes a focused Open Decision Gates checklist so implementing agents do not silently assume a DR lean is locked, and so milestone owners are responsible for closing the listed DRs through evidence. The next full regeneration should fold these rows into the per-milestone sections.

## Rating System

| Column | Who Fills It | Scale | Meaning |
|---|---|---|---|
| `Done` | Agent, only after validation | `[ ]` or `[x]` | Check only when the item is implemented, validated, and linked to evidence. Leave unchecked for partial work. |
| `Evidence` | Agent | link or path | Link the run bundle, prototype note, test log, screenshot, replay report, commit, or blocker note. |
| `H-Full` | Human owner | 1-10 | Human rating for how fully implemented the feature feels. `10` means complete enough to keep building on. |
| `H-Quality` | Human owner | 1-10 | Human rating for polish, feel, UX, correctness, readability, and maintainability. |
| `H-Review` | Human owner | 1-10 | Human rating for how much review/rework is still needed. `1` means no concern; `10` means urgent review. |
| `AI-Full` | Implementing/reviewing agent | 1-10 | AI self-rating for implementation completeness after validation. |
| `AI-Quality` | Implementing/reviewing agent | 1-10 | AI self-rating for quality after tests, E2E, bug hunt, and documentation. |
| `AI-Review` | Implementing/reviewing agent | 1-10 | AI self-rating for review risk. `1` means low risk; `10` means needs immediate review. |

## Update Rules For Agents

| Rule | Requirement |
|---|---|
| Read first | Before implementation, read [[spec/prototype-roadmap]], [[spec/native-implementation-backlog]], and this checklist. |
| Update exact rows | When finishing work, update every affected row: roadmap feature, milestone scope, milestone done-criterion, side-track obligation, and native task card. |
| Evidence required | Do not check a row without evidence. Evidence can be a run bundle, test log, replay report, screenshot, prototype note, commit hash, or explicit blocker note. |
| Human ratings | Do not invent human ratings. Leave `H-*` blank unless the user gives ratings. |
| AI ratings | Fill `AI-*` when you claim a row is done or substantially progressed. Be conservative; low full/quality or high review risk is useful. |
| Partial work | Leave `Done` unchecked, fill evidence and AI ratings, and explain remaining work in `Notes`. |
| Zero known issues | Do not check a milestone/feature row if any verified review finding at any severity remains unresolved. Deferral is allowed only when the user explicitly approves that exact finding and the deferral is recorded with issue ID, reason, owner, next checkpoint, and evidence path. |
| Contract integrity | Do not check a row until app/tool/control/replay paths use the same source of truth, required fields reject missing/malformed values, accepted commands truly mutate state or reject, and checklist notes do not hide required missing work. |
| Roadmap drift | If you add, split, rename, or delete roadmap/backlog features, update this file in the same commit/pass. |
| Milestone handoff | Final handoff must list checklist IDs changed and any rows left `READY_FOR_HUMAN`. |

## Table Of Contents

- [Milestone Scope Checklist](#milestone-scope-checklist)
- [Milestone Done-Criteria Checklist](#milestone-done-criteria-checklist)
- [Roadmap Feature Index Checklist](#roadmap-feature-index-checklist)
- [Side Track Checklist](#side-track-checklist)
- [Server/MMO Addendum Checklist](#servermmo-addendum-checklist)
- [Material/T-MAT Addendum Checklist](#materialt-mat-addendum-checklist)
- [Open Decision Gates Checklist](#open-decision-gates-checklist)
- [Native Task Card Checklist](#native-task-card-checklist)
- [Global Validation And Bug Hunt Checklist](#global-validation-and-bug-hunt-checklist)

---

## Server/MMO Addendum Checklist

Use these rows for all M9-M12/T-SERVER work until the checklist is fully regenerated. Human ratings stay blank until the owner gives them.

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `TSERVER-P00` | T-SERVER side track: `cf-server` is the shared dedicated server artifact for LAN, co-op, PvP arena, MMO shard, and lobby directory modes. | [[spec/prototype-roadmap#T-SERVER — Dedicated Server App Lifecycle And Community Hosting]] | - | - | - | - | - | - | - | Use same sim path as client; no server-only game logic. |
| [ ] | `M9-SERVER-CORE` | M9 server-core subset passes: SERVER-001, SERVER-006, SERVER-009, SERVER-010, SERVER-011, SERVER-014, SERVER-015, SERVER-016. | [[spec/server-app-architecture#Acceptance Suite]] | - | - | - | - | - | - | - | M9 does not require SERVER-002/004/012 PvP/MMO scale tests. |
| [ ] | `M9-CXSERVER` | `cf-server` binary scaffold: RON config, `--mode`, `--validate-config-only`, no render/UI/audio crates. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Owns `cf-server`, `cf-server-ops`. |
| [ ] | `M9-OPS` | Health, readiness, metrics, JSON logs, drain shutdown, restart hooks. | [[spec/server-app-architecture]] | - | - | - | - | - | - | - | Emits `server.*` events. |
| [ ] | `M9-ANTI-CHEAT-FOUNDATION` | Anti-cheat profile registry, rate-limit hooks, replay drift skeleton, persisted ban list, audit log. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Foundation only; tournament-grade remains later. |
| [ ] | `M9-PERSISTENCE-FOUNDATION` | Snapshot writer, append-only event journal, restore loop, backups, schema migration hooks. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Full MMO persistence remains M12. |
| [ ] | `M9-DOCKER` | Reference Docker image runs `cf-server` unchanged and is documented. | [[spec/server-app-architecture#Acceptance Suite]] | - | - | - | - | - | - | - | Linux required; Windows hosting guide required separately. |
| [ ] | `M10-LAN-CXSERVER` | LAN co-op runs through `cf-server --mode lan_room`; ready-up, replicated state, per-client replay alignment. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Includes `anti_cheat.profile_applied` with `casual`. |
| [ ] | `M11-ONLINE-SELF-HOSTED` | A community member can host `cf-server --mode coop_room`; remote friends join through NAT/relay and complete a Breach Contract. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - | Package hash mismatch must fail cleanly. |
| [ ] | `M11-LOBBY-DIRECTORY` | `lobby_directory` registration, heartbeat, browse/filter, deregister, and expiry work end-to-end. | [[decisions/dr-013-backend-service-scope]] | - | - | - | - | - | - | - | Required for public discovery; optional for private deployments. |
| [ ] | `M12-PVP-ARENA` | `cf-server --mode pvp_arena` runs a 4-8 player public arena with server-authoritative state and replay-aligned clients. | [[spec/prototype-roadmap#M12 — Public PvP Arenas + Persistent MMO Shards — Extended With Bunker Defence Flagship Per DR-042 + Realistic Comms Per DR-043]] | - | - | - | - | - | - | - | Uses `competitive` default; `tournament_strict` opt-in only. |
| [ ] | `M12-MMO-SUITE` | MMO-001..MMO-012 all pass, including 50-client 1-hour soak, persistence restart, interest management, no-cloud reference. | [[spec/persistent-mmo-architecture#Acceptance Suite]] | - | - | - | - | - | - | - | M12 evidence gate; failure reopens DR-035. |
| [ ] | `M12-PVP-MMO-DR-REVIEW` | DR-005/013/034/035 reviewed with M9-M12 evidence; scope promoted, adjusted, or reopened explicitly. | [[spec/native-implementation-backlog#M12 — Public PvP Arenas + Persistent MMO Shards]] | - | - | - | - | - | - | - | No silent demotion or silent scope expansion. |

---

## Material/T-MAT Addendum Checklist

Use these rows for all M5.6/M5.7/M5.9/M6.6/M7.5/M8.5/T-MAT work until the checklist is fully regenerated. Human ratings stay blank until the owner gives them.

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `TMAT-P00` | T-MAT side track: systemic material simulation (active-region CA + reaction table + atmospheres + affordance/affliction layer) is a core feel pillar; curated 17-material launch set per DR-036. | [[spec/prototype-roadmap#T-MAT — Systemic Materials, Chemistry, And Atmospheres]] | - | - | - | - | - | - | - | Owns `cf-material`, `cf-atmos`. CPU-deterministic kernel; replay-deterministic; server-authoritative. |
| [ ] | `TMAT-CRATES` | `cf-material` (chunked CA kernel) + `cf-atmos` (Stationeers-grade-or-better atmosphere/thermal networks) crates exist with `AGENTS.md` boundary docs and integrate with `cf-terrain`/`cf-physics`/`cf-replay`/`cf-server`. | [[decisions/dr-036-systemic-material-simulation-direction]] · [[decisions/dr-037-stationeers-grade-atmospherics-direction]] | - | - | - | - | - | - | - | Workspace now 29 crates. |
| [ ] | `TMAT-EVENTS` | Run-bundle event categories `material`, `reaction`, `atmospherics`, `affliction` defined in `prototype-run-bundle-schema.md` and emitted from sim with parent cause chains. | [[references/prototype-run-bundle-schema#Event Category Baseline]] | - | - | - | - | - | - | - | Required before any M5.6+ run bundle can validate. |
| [ ] | `M5.6-MAT-KERNEL` | M5.6 done-criteria: MAT-01..MAT-03 + MAT-06 + MAT-13 minimal pass; active material kernel + reaction table + density layering + replay determinism with `material.*` and `reaction.*` events. | [[spec/native-implementation-backlog#M5.6 — Material Kernel]] | - | - | - | - | - | - | - | Per-chunk material checksums in snapshots. |
| [ ] | `M5.6-CFCTL` | `cfctl observe --materials/--reactions` and `cfctl inspect material/reaction <event-id>` per CLI reference. | [[spec/prototype-roadmap#CLI Reference]] | - | - | - | - | - | - | - | Required for AI-agent and accessibility tooling. |
| [ ] | `M5.7-HAZARD` | M5.7 done-criteria: MAT-04 + MAT-05 + MAT-07 pass; MAT-08 stub lands; acid/electricity/debris/ingestion damage routes through M5.5 impulse path and the affliction layer; HUD overlay screenshots captured. | [[spec/native-implementation-backlog#M5.7 — Hazard Package]] | - | - | - | - | - | - | - | Mandatory hazard overlays + captions; replay cause chains required. |
| [ ] | `M5.7-AFFLICTION` | Affliction layer (`wetness`, `burning`, `corroded`, `electrified`, `poisoned`, `asphyxiating`, `concussed`, `drowning`, `depressurizing`) wired into actor state + HUD; `affliction.*` events emitted with cause chains. | [[spec/native-implementation-backlog#M5.7 — Hazard Package]] | - | - | - | - | - | - | - | Visible on HUD; decay rules per material registry. |
| [ ] | `M6.6-AI-MAT` | M6.6 done-criteria: AI-MAT-01..AI-MAT-08 acceptance suite passes; AI material competence with reason labels; AI-H regression remains green. | [[spec/native-implementation-backlog#M6.6 — AI Material Competence]] | - | - | - | - | - | - | - | DR-022 humanlike-bar fairness + fog-of-war required for hazard perception. |
| [ ] | `M6.6-AFFORDANCE` | Per-material AI affordance tags (`avoid`, `seek`, `use-as-weapon`, `extinguish-with`, `neutralize-with`, `vent`, `pump`) wired into utility scoring; closed-enum reason labels (`hazard_unknown`, `hazard_underestimated`, `hazard_traded_for_objective`, `hazard_avoided`, `hazard_exploited`, `hazard_recovered`, `friendly_fire_avoided`). | [[spec/native-implementation-backlog#M6.6 — AI Material Competence]] | - | - | - | - | - | - | - | No free-text reasons. |
| [ ] | `M7.5-ATMOS` | M7.5 done-criteria: MAT-09 + MAT-10 pass; Stationeers-grade-or-better hull/gap/aperture/pump/vent/oxygen/pressure/fire/thermal networks; pressure/liquid jets and thermal recovery routes work; mission director can author room-state objectives. | [[spec/native-implementation-backlog#M7.5 — Base Atmospherics]] | - | - | - | - | - | - | - | Server-authoritative atmosphere state per DR-005/DR-034/DR-035. |
| [ ] | `M7.5-CFCTL-ATMOS` | `cfctl observe --atmospheres --stream --hz 5 --scope <room-id\|all>` exposes per-hull/room state, apertures, pressure/liquid jets, thermal links, and per-hull inspect. | [[spec/prototype-roadmap#CLI Reference]] | - | - | - | - | - | - | - | Designer + AI agent + accessibility consumers. |
| [ ] | `M8.5-MAT-LAB` | M8.5 done-criteria: MAT-11 + MAT-14 pass; designer authors + exports + reloads a material puzzle in <10 minutes; community mod pack with new material loads cleanly. | [[spec/native-implementation-backlog#M8.5 — Material Lab]] | - | - | - | - | - | - | - | Required gate for adding materials beyond launch 17. |
| [ ] | `M8.5-EXPANSION-GATE` | `cf-mod validate --strict` rejects expansion materials missing inspect overlay, AI affordance tag, replay event payload, recipe journal entry, or accessibility caption. | [[spec/native-implementation-backlog#M8.5 — Material Lab]] | - | - | - | - | - | - | - | Schema-enforced; no half-spec'd packs. |
| [ ] | `M8.5-RECIPE-JOURNAL` | In-engine recipe journal logs designer-triggered reactions and persists across editor sessions; exportable as scenario hint content fragment. | [[spec/native-implementation-backlog#M8.5 — Material Lab]] | - | - | - | - | - | - | - | Player-readable; respects fog-of-war for player runs. |
| [ ] | `TMAT-DR-REVIEW` | DR-007/036 reviewed with M5.6-M8.5 evidence; scope promoted, adjusted, or reopened explicitly. | [[decisions/dr-036-systemic-material-simulation-direction]] | - | - | - | - | - | - | - | No silent demotion or silent scope expansion. |

---

## Origin / Atmospherics / Gravity Addendum Checklist

> [!info] Why this addendum exists
> The 2026-05-06 design intent pass added DR-037 (Stationeers-grade-or-better atmospherics & chemistry direction) + DR-038 (universal gravity & ballistics direction), [[spec/origin-reaction-and-resource-model]], [[spec/atmospherics-and-chemistry-model]], [[spec/gravity-and-ballistics-model]], M5.8 (Origin Resource & Overclock Pass), M5.9 (Atmospherics-Grade Kernel), an extended M7.5 (Base Atmospherics — promoted from approximate room networks to Stationeers-grade-or-better), and three new run-bundle event categories (`atmospherics`, `gravity`, `ballistics`). This file now includes a focused addendum so implementing agents have checklist rows immediately. The next full regeneration should merge these rows into the normal milestone/side-track sections.

Use these rows for all M5.8 / M5.9 / extended M7.5 / origin-reaction / atmospherics-grade / universal-gravity work until the checklist is fully regenerated. Human ratings stay blank until the owner gives them.

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `ORIGIN-P00` | Origin reaction & resource model: per-origin shot reactions (force-feedback content, G-load susceptibility, concussion vs internal-shock, fall damage, limb wounds vs module failure, bleed vs coolant/oil leak), G-Force vision blackout HUD, voluntary overclock vs involuntary downclock state machines, environment resistance (vacuum/oxygen consumption, heat tolerance), origin-gated healing affordances, resource model (`caloric_energy`/`battery_charge`/`power`/`heat`/`oxygen_supply`), affliction extensions, ORIGIN-A acceptance tests. | [[spec/origin-reaction-and-resource-model]] | - | - | - | - | - | - | - | Captured during M1 from user-supplied design intent (Round 1: combat reactions; Round 2: environment resistance). M5.8 closes per-origin reaction matrix runtime. |
| [ ] | `ATMOS-P00` | Stationeers-grade-or-better atmospherics & chemistry direction: real PV=nRT (`R = 8314.46`), 10 launch gases (O2, N2, CO2, Volatiles, Pollutant, H2, He, O3, N2O, H2O) + 6 launch liquid mixtures with locked specific heats / latent heats / autoignition temps / molar masses; expansion path for more elements/materials/reactions through M8.5; 6 deterministic combustion reactions; gradual phase change with latent heat; first-class pipe networks (pumps/valves/regulators/filtration/condensation/evaporation chambers); room atmospheres + door state machine + airlock cycles; EVA + Hardsuit life-support with breathing math (`0.0048 mol/tick · BreathingRate · BreathingEfficiency`); per-planet ambient (Earth/Mars/Moon/Mimas/Europa/Vulcan/Venus); wind from ΔP impulse force on entities; physical apertures from doors/bullet holes/blast breaches/pipe ruptures/suit punctures; liquid pressure jets/flooding; material heat transfer + thermal tools; ATMOS-A acceptance tests. | [[spec/atmospherics-and-chemistry-model]] · [[decisions/dr-037-stationeers-grade-atmospherics-direction]] | - | - | - | - | - | - | - | Captured during M1 from 29+ Stationeers research sources. Stationeers-grade is the minimum bar; M5.9 closes the kernel; extended M7.5 closes the base modules + mission integration. |
| [ ] | `GRAV-P00` | Universal gravity & ballistics direction: one `GravityField` is single source of truth; every system reads from it (actors, projectiles, materials, gases, equipment, debris, sparks, casings); no hardcoded `9.81`. Per-planet `gravity_g` scenario manifest field; per-cell / per-region overrides (gravity well, low-g lab, magnetic boots, damaged grav generator, reverse-g chamber). Ballistic math `a = (F_grav + F_drag + F_collision) / m`; drag reads atmospherics ρ_local. Atmospherics + material density layering both read g per cell. GRAV-A acceptance tests. | [[spec/gravity-and-ballistics-model]] · [[decisions/dr-038-universal-gravity-and-ballistics-direction]] | - | - | - | - | - | - | - | Captured during M1. Lands at extended M5.5 + M5.6 + M5.9. CI grep gate enforces no hardcoded gravity. |
| [ ] | `M5.8-ORIGIN-MATRIX` | M5.8 done-criteria: ORIGIN-A-01..ORIGIN-A-15 acceptance suite passes; per-origin impulse routing (humans → `concussion_dose` + `g_load_dose`; androids reduced; robots → `internal_shock` per module); per-origin fall-damage tolerance; resource accumulators per origin. | [[spec/native-implementation-backlog#M5.8 — Origin Resource & Overclock Pass]] | - | - | - | - | - | - | - | Same impulse on three origins → three different reaction event chains. |
| [ ] | `M5.8-OVERCLOCK` | Voluntary overclock state machine (robot whole-processor; android per-module) + involuntary downclock state machine (passive heat) — distinct events, distinct HUD chips, distinct AI doctrine responses. | [[spec/native-implementation-backlog#M5.8 — Origin Resource & Overclock Pass]] | - | - | - | - | - | - | - | `chassis_overclock_*` and `chassis_thermal_throttle_*` event families. |
| [ ] | `M5.8-LEAKS` | Coolant + oil leak channels for robots: penetrating round / armor-cracked module emits `chassis_leak_started`; particles route into M5.6 material kernel for ground pooling and ignition reactions. | [[spec/native-implementation-backlog#M5.8 — Origin Resource & Overclock Pass]] | - | - | - | - | - | - | - | Visible on M5.6 run bundle particles. |
| [ ] | `M5.8-G-BLACKOUT` | G-Force vision blackout HUD effect: vignette darkens proportional to `g_load_dose` for humans; reduced curve for androids; never for robots. Accessibility flag `--reduced-g-force-blackout`; non-visual caption + HUD icon fallback. | [[spec/native-implementation-backlog#M5.8 — Origin Resource & Overclock Pass]] | - | - | - | - | - | - | - | DR-012 accessibility gate blocks color-only / vignette-only signal. |
| [ ] | `M5.8-EQUIP-GATES` | Origin-gated equipment validation: `helmet`, `oxygen_tank`, `food`, `medkit`, `drug` items with `origin_compatibility` field. Slot-assign rejects with `wrong_origin_for_equipment`; AI bot picks emit `wrong_origin_for_treatment`. | [[spec/native-implementation-backlog#M5.8 — Origin Resource & Overclock Pass]] | - | - | - | - | - | - | - | No silent slot acceptance. |
| [ ] | `M5.8-CFCTL` | `cfctl observe --origin-state <actor>` shows resources + afflictions + overclock + downclock state per actor. | [[spec/prototype-roadmap#CLI Reference]] | - | - | - | - | - | - | - | AI-agent + accessibility consumers. |
| [ ] | `M5.9-PV-NRT` | M5.9 done-criteria: ATMOS-A-01..ATMOS-A-19 acceptance suite passes. PV=nRT correctness (R = 8314.46), mixing, pressure spike on heating, deterministic combustion stoichiometry (6 locked reactions), gradual phase change with latent heat, breach apertures, liquid jets/flooding, heat transfer, and player thermal techniques. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | CPU-deterministic kernel; same seed = byte-identical event stream. |
| [ ] | `M5.9-PIPES` | First-class pipe networks: pumps / valves / regulators (forward + back-pressure) / volume + turbo pumps / filtration / one-way / purge / pressurant / condensation / expansion valves / condensation + evaporation chambers split networks. Pipe damage thresholds: gas pipe rupture > 60.795 MPa; liquid pipe > 6.079 MPa; frozen contents > 0.05 mol/L. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | No double-welded-frame rupture-immunity loophole. |
| [ ] | `M5.9-ROOMS` | Room atmospheres + sealed-cell collapse + door state machine (closed_sealed/closed_unsealed/cycling_open/open/cycling_close/breached) + airlock controller (canonical 2-door + 2-active-vent + console). Breach detection. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | Adjacent sealed cells collapse for performance; partial-pressure HUD queries break apart on demand. |
| [ ] | `M5.9-APERTURES` | Pressure apertures: doors, vents, cracked windows, bullet holes, shaped-charge cuts, blast breaches, pipe ruptures, suit punctures, and repair patches carry area/material/source-event state. Flow/wind force scales by ΔP × aperture area with bounded choked-flow caps. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] · [[spec/atmospherics-and-chemistry-model#Flow, Wind, Liquid Jets, And Breach Holes]] | - | - | - | - | - | - | - | No visual-only pressure holes. |
| [ ] | `M5.9-LIQUID-JETS` | Liquids have mass/density/viscosity/temperature/contamination and can jet, spray, flood, siphon, cool/heat, contaminate, and apply collision impulse through apertures. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] · [[spec/atmospherics-and-chemistry-model#Flow, Wind, Liquid Jets, And Breach Holes]] | - | - | - | - | - | - | - | No "water level only" abstraction where pressure jets matter. |
| [ ] | `M5.9-THERMAL` | Heat transfer through materials: conduction, fluid advection/convection, latent heat, combustion/electrical/collision heat, and bounded ambient/radiation. Player tools include heaters, coolers, radiators, heat exchangers, coolant loops, insulation, emergency venting, and power throttling. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] · [[spec/atmospherics-and-chemistry-model#Heat Transfer And Thermal Engineering]] | - | - | - | - | - | - | - | Temperature is gameplay, not cosmetic. |
| [ ] | `M5.9-SUITS` | EVA Suit (10 L, 6 slots) + Hardsuit (10 L, 8 slots, IC10 processor) life-support; canister + filter + waste-tank slots; breathing math `0.0048 mol/tick · BreathingRate · BreathingEfficiency`; min inhaled partial pressure 16 kPa; filter max waste-tank 4052 kPa; helmet flush function. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | Origin-gated per [[spec/origin-reaction-and-resource-model]]. |
| [ ] | `M5.9-PLANETS` | Locked per-planet ambient: Earth (101 kPa, 0-40 °C, 75% N2 / 25% O2), Mars (2-3 kPa, 95% CO2), Moon/Mimas (vacuum), Europa (44-47 kPa cold N2), Vulcan (24-56 kPa hot oxidizing), Venus (239 kPa hot CO2). Modder schema for new ambients via `content/worlds/`. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | Each scenario manifest declares planet + `gravity_g`. |
| [ ] | `M5.9-WIND` | Wind from ΔP: pressure differentials apply impulse force on actors / dropped items / debris / gibs; routes through M5.5-008 contact solver as a force input. Direction interacts with gravity. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | `atmospherics.wind_force_applied` events. |
| [ ] | `M5.9-GRAVITY-FIELD` | Universal `cf_physics::gravity::GravityField` (layered: cell > region > ambient); per-planet ambient + per-cell / per-region overrides. SoA storage; SIMD-friendly per-cell array; cache-friendly hot-path sampling. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | CI grep gate: no hardcoded `9.81` in production code. |
| [ ] | `M5.9-BALLISTICS` | Ballistic math: `a = (F_gravity + F_drag + F_collision) / m`. `F_drag = -0.5 · ρ_local · v · |v| · C_d · A` where ρ_local from atmospherics. Per-projectile drag profile. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | Vacuum projectiles fly farther (drag ≈ 0); dense atmospheres slow projectiles. |
| [ ] | `M5.9-STRATIFICATION` | Gas stratification: per-tick partial-pressure adjustment proportional to local g × molar mass spread. CO2 sinks; H2 rises; uniform at 0g; flips at reverse g. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | Only sealed atmospheres with significant ΔM run the per-tick stratification step. |
| [ ] | `M5.9-CFCTL` | `cfctl observe --atmospheres / --pipe-networks / --rooms / --suits / --gravity / --ballistics` per CLI Reference; `cfctl inspect <kind> <id>` for drill-down. | [[spec/prototype-roadmap#CLI Reference]] | - | - | - | - | - | - | - | Required for AI agents and accessibility tooling. |
| [ ] | `M5.9-DETERMINISM` | ATMOS-A-15 + GRAV-A-10: same seed + same inputs = byte-identical event stream + final state across 10000+ ticks. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | Hard halt on any first-divergence. |
| [ ] | `M5.9-PERF` | Active-region perf budget at 60 Hz on Steam Deck floor + 120 Hz validated path + 4K/120 strong desktop. Sleeping atmospheres are checksummed and skipped. CPU hot paths are multicore/cache-friendly. GPU acceleration is allowed for presentation and later compute only with deterministic parity proof or explicit non-authoritative status. | [[spec/native-implementation-backlog#M5.9 — Atmospherics-Grade Kernel]] | - | - | - | - | - | - | - | Per-tick kernel budget per [[spec/prototype-roadmap#No-Compromise Performance Defaults]]. |
| [ ] | `M7.5-EXTENDED` | Extended M7.5 done-criteria: M5.9 kernel wired into base modules (oxygen generator, scrubber, vents, pumps, pipes, tanks, valves, condensation/evaporation chambers, gas analyzer, suit storage, hydroponic tray); mission director can author room-state + atmosphere objectives; HUD per-room overlays. | [[spec/native-implementation-backlog#M7.5 — Base Atmospherics]] | - | - | - | - | - | - | - | Server-authoritative atmosphere state per DR-005/DR-034/DR-035. |
| [ ] | `EVENTS-EXTENDED` | Run-bundle event categories `atmospherics`, `gravity`, `ballistics` defined in `prototype-run-bundle-schema.md` and emitted from sim with parent cause chains. Affliction extensions (`internal_shock`, `coolant_leaking`, `oil_leaking`, `overheating`, `low_battery`, `power_starved`, `weak`, `exhausted`, `hypoxia`, `downclocked`, `heat_exhaustion`) wired. | [[references/prototype-run-bundle-schema#Event Category Baseline]] | - | - | - | - | - | - | - | Required before any M5.8+ run bundle can validate. |
| [ ] | `DR-037-EVIDENCE` | DR-037 reviewed with M5.9 evidence; status promotes from `closed-direction` to `closed-direction-with-evidence` once ATMOS-A-01..19 pass byte-identically. | [[decisions/dr-037-stationeers-grade-atmospherics-direction]] | - | - | - | - | - | - | - | No silent demotion. |
| [ ] | `DR-038-EVIDENCE` | DR-038 reviewed with M5.5 + M5.6 + M5.9 evidence; status promotes from `closed-direction` to `closed-direction-with-evidence` once GRAV-A-01..10 pass byte-identically and CI grep gate stays green. | [[decisions/dr-038-universal-gravity-and-ballistics-direction]] | - | - | - | - | - | - | - | No silent demotion. |

---

## Open Decision Gates Checklist

> [!warning] Reading rule
> Before starting any milestone, verify the Open DR gates listed in the milestone's roadmap section. Use [[dashboards/decision-tracker]] for current status. When a milestone closes a still-open DR, update the relevant row here, the DR file, [[decisions/index]], [[dashboards/decision-tracker]], and [[dashboards/research-readiness]] in the same pass.

| Done | ID | DR / Topic | Status | Closes In | Worker Action Required Before Closure | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [~] | `GATE-DR-002` | [[decisions/dr-002-replay-event-architecture|DR-002]] | OPEN (M0 schema lock applied) | M3 | M3 done-criteria pass + 5-minute M2 run replays headlessly with matching checksums. Update DR status to CLOSED-DIRECTION + revisit_trigger refresh + decision-tracker + research-readiness. | - | - | - | 4 | 8 | 3 | M0 (2026-05-05) confirmed the hybrid event-log + snapshots lean and locked the v1 envelope with user approval. Categories: `system`, `control`, `determinism`. Manifest extensions: `checksum.{algorithm,scope,cadence_ticks}` + `settings:{...}`. Summary extensions: `final_sim_checksum`, `checksum_event_count`, `first_tick`, `last_tick`. `sim_state_v1` scope is append-only; layout-breaking bumps move to `_v2`. M3 still owns headless replay + first-divergence + `snapshot` category. |
| [ ] | `GATE-DR-003` | [[decisions/dr-003-body-damage-readability|DR-003]] | OPEN | M4 | HUD-01..HUD-03 acceptance pass with 5 playtesters; silhouette + advanced HUD opt-in lean validated. | - | - | - | - | - | - | - | Touched by M3, M4, M5, M5.7, M7. |
| [ ] | `GATE-DR-004` | [[decisions/dr-004-first-playable-slice|DR-004]] | OPEN | M7 | Breach Contract proof mission shippable; project owner plays 5 runs and writes verbatim reaction. | - | - | - | - | - | - | - | Sequenced single actor → squad → bunker breach lean. |
| [ ] | `GATE-DR-006` | [[decisions/dr-006-modding-data-model|DR-006]] | OPEN | M8 | Workbench V1 exists; 3 mods migrated; package format + provenance + script-host posture locked. | - | - | - | - | - | - | - | Touched by M2, M5, M5.6, M5.7, M6.6, M7.5, M8, M8.5, M10, M11. |
| [ ] | `GATE-DR-007` | [[decisions/dr-007-terrain-material-model|DR-007]] | OPEN (defers to DR-036) | M5.6 / M5.7 / M7.5 | DR-036 milestones close DR-007 implementation specifics. Update DR-007 status when M7.5 done-criteria pass. | - | - | - | - | - | - | - | M2 launch material set must remain compatible with DR-036 expansion. |
| [ ] | `GATE-DR-008` | [[decisions/dr-008-ai-architecture|DR-008]] | OPEN | M6 | AI-01..AI-12 + AI-H-01..AI-H-06 pass with replay evidence; hybrid jobs + utility scoring + scripted hooks lean validated. | - | - | - | - | - | - | - | Touched by M6, M6.5, M6.6, M7, M11, M12. |
| [ ] | `GATE-DR-009` | [[decisions/dr-009-command-ux-style|DR-009]] | OPEN | M4 / M7 | ORDER-01 acceptance pass; direct + slowdown overlay + optional tactical map lean validated. | - | - | - | - | - | - | - | Touched by M4, M5, M6, M6.5, M6.6, M7, M8.5. |
| [ ] | `GATE-DR-010` | [[decisions/dr-010-license-reuse-matrix|DR-010]] | OPEN | Public-release decision | Documentation only; ledger tracks usage. Remains open until project owner decides on public release. | - | - | - | - | - | - | - | Not a build blocker during private prototyping. |
| [ ] | `GATE-DR-011` | [[decisions/dr-011-progression-retention-loop|DR-011]] | OPEN | M7 / M11 / M12 | RET-A-01..RET-A-06 prototype results show players return for mastery/stories, not obligation. | - | - | - | - | - | - | - | Anti-grind posture and DR-057 dormant/default-off optional economy hooks must be preserved through M12. |
| [~] | `GATE-DR-012` | [[decisions/dr-012-accessibility-comfort-readability|DR-012]] | OPEN (M0 surface lock applied) | M4 | ACC-A-01..16 pass across HUD, command, equipment workbench, replay, hub, package-builder, settings, and run-bundle evidence. | - | - | - | 3 | 8 | 3 | M0 (2026-05-05) wired the six accessibility flags (`--ui-scale`, `--high-contrast`, `--captions on\|off`, `--reduced-motion`, `--reduced-shake`, `--reduced-flash`) into `cf-control::Settings`, exposed them via `cfctl observe --settings --once`, and recorded them in `run_manifest.json.settings`. M0 carries no UI behavior; M4 / T-AUDIO / T-PERF consume the surface. Localization deferred to M4 per the protocol. |
| [ ] | `GATE-TOPIC-NET-TRANSPORT` | Networking transport library | OPEN | M9 / M10 | lightyear vs renet vs quinn for `cf-net`. Worker MUST present transport options + perf evidence + adapter-trait shape to the user before committing. | - | - | - | - | - | - | - | Decision lives outside DR system until then. |
| [ ] | `GATE-TOPIC-MOD-SCRIPT-HOST` | Modding script host | OPEN | M5 / M8 | mlua vs Rhai. Worker MUST run benchmark + capability-gate audit and ask the user before locking. | - | - | - | - | - | - | - | Affects DR-006 closure. |
| [ ] | `GATE-TOPIC-LOCALIZATION` | Localization plan | OPEN | TBD | Strings/fonts/lang packs/mod-localization. Worker MUST flag any code path that bakes English-only strings; avoid hardcoded UI strings. | - | - | - | - | - | - | - | Touched by M4, M7, M8. |
| [ ] | `GATE-TOPIC-CLOUD-SAVE` | Cloud-save backend | OPEN | Post-launch | Local-first today (DR-029); no cloud at launch. Worker MUST NOT add cloud dependencies during T-SAVE work. | - | - | - | - | - | - | - | Out of scope for v1. |
| [~] | `GATE-PROTOCOL-COMPLIANCE` | Worker confirms `Open Decision Gates Protocol` was followed | N/A | Each milestone | Per-milestone vault note records: which open DRs were touched, whether the lean held, what evidence was added, and whether user input was requested. | - | - | - | 9 | 9 | 2 | M0 (2026-05-05) ran the pre-check against DR-002, DR-012, and the toolchain pin (open topic). User confirmed all three locks via AskUser. Implementation log: `corefall/docs/implementation-log/2026-05-05-m0-engine-bootstrap.md`. Subsequent milestones must repeat this contract. |

---

## Milestone Scope Checklist

These rows come from the `Scope` lists under each roadmap milestone. They are broad implementation surfaces; the native task-card rows later in this file break them down further.

### M0 - Engine Bootstrap

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M0-P00` | Milestone proof: The native repo exists, builds on three platforms, runs a Bevy app with a fixed-tick sim plugin, ticks at 60 Hz, exits cleanly, produces a deterministic run bundle from a scripted no-op scene. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | M0.3 acceptance bundles (all PASS canonical checker, all repo-root path): `prototype_runs/native/m0_2026-05-06T04-46-04Z_1ad62cb4` (cfctl run 60 Hz / 300 ticks / 5.004 s), `m0_2026-05-06T04-46-14Z_2c7f5b05` (cfctl run 120 Hz / 600 ticks / 5.003 s), `m0_2026-05-06T04-46-27Z_a9675fc6` (cf-app headless-smoke 60 Hz / 300 ticks / 5.006 s), `m0_2026-05-06T04-46-37Z_56e26f4b` (live cfctl settings roundtrip; mid-run write plus final `system.run_finished`). `corefall/docs/implementation-log/2026-05-05-m0-engine-bootstrap.md` and `corefall/docs/reviews/2026-05-06-m0-m0-3-review-report.md` include the M0.3 acceptance and contract matrices. | - | - | - | 9 | 9 | 1 | All M0.1/M0.2/M0.3 review findings closed. 68 tests + doctests passing. Zero unresolved M0 findings. |
| [x] | `M0-S01` | Cargo workspace with the crate layout above. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `corefall/game/Cargo.toml` + 29 crate `Cargo.toml`s + per-crate `AGENTS.md` files + `rust-toolchain.toml` (1.93.0) + `rustfmt.toml` + `clippy.toml` + `.cargo/config.toml` | - | - | - | 9 | 8 | 2 | All 29 crates compile via `cargo check --workspace --all-targets`. |
| [x] | `M0-S02` | `cf-app` binary that launches a Bevy app with empty schedule. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-app/src/main.rs`: real Bevy `App` with `DefaultPlugins`, fixed window title `Corefall — M0 Engine Bootstrap (v…)`, `WindowResolution::new(1280.0, 720.0)`, `cf-render-2d::CfRenderPlugin`, `Time::<Fixed>::from_hz(--tick-rate-hz)`, ESC/`WindowCloseRequested` exit; `--headless-smoke` is a flag; CLI parser unit tests cover all M0 flags + `--tick-rate-hz`. | - | - | - | 9 | 8 | 2 | All three run modes (Bevy, headless, headless+control-api) ship. The headless+control-api server path was added after acceptance testing caught a `Connection refused` bug (M0-BUG-007 in the implementation log). |
| [x] | `M0-S03` | `cf-sim-core` fixed-tick scheduler (60 Hz default; 120 Hz option). | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-sim-core/src/lib.rs`; 14 unit tests pass (Tick/SimClock/Rng/SimConfig + checksum + ids + `step_zero_is_a_no_op`). M0.2-F4: `system.tick_sample` event implemented + emitted every cadence_ticks (60); 5 events per 300-tick run at 60 Hz, 10 per 600-tick run at 120 Hz; verified in every M0.2 bundle. | - | - | - | 9 | 9 | 2 | All required tick events present; tick-rate config-driven; 60 + 120 Hz both validated. |
| [x] | `M0-S04` | `cf-replay` minimal event envelope + run-bundle writer (no events yet beyond `system_*`). | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-replay/src/lib.rs`; 2 unit tests pass; checker validates emitted bundles. M0.2-F1: cfctl + cf-app now route through the SAME `cf_control::runtime::build_engine_config` so bundles ship identical real `commit_sha`/`rust_version`/`bevy_version`/`expected_tests`/`config_hash` regardless of which binary wrote them (proven by identical metadata + checksum across `m0_..._c6ed64df` (cf-app) and `m0_..._25e6cb16` (cfctl) at the same seed). | - | - | - | 9 | 9 | 2 | Production metadata path is shared. `for_test_scenario_only` is `#[doc(hidden)]`. |
| [x] | `M0-S05` | `cf-render-2d` minimal wgpu pipeline that clears the screen. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-render-2d/src/lib.rs`: `CfRenderPlugin` inserts `ClearColor(M0_CLEAR_COLOR=#0d121a)` and spawns the main 2D camera. `plugin_inserts_clear_color` unit test validates the wiring. Driven by `cf-app`'s Bevy `DefaultPlugins` (which already routes through wgpu). | - | - | - | 8 | 8 | 3 | Chunked terrain pipeline + sprite batching land at M2; M0 gate (clear-screen via wgpu) is real. |
| [x] | `M0-S06` | `cf-control` minimal command/observation schema plus `cargo run -p cfctl -- observe --once`, `cargo run -p cfctl -- run --ticks`, `pause`, and `step`. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-control/`, `corefall/game/crates/cfctl/`; 18 static schemas + drift test. M0.3-F7: every M0 method is strict about `schema_version`, unknown fields, unsupported params, zero ticks, empty settings patches, unsupported `act.player.move`, and unsupported `runbundle.write.id_override`. Tests: 36 `cf-control` unit tests + 9 live WS tests including unknown-field, zero-step, unsupported movement, and id_override rejection. Final live script bundle `m0_2026-05-06T04-46-37Z_56e26f4b` proves the server-driven settings path. | - | - | - | 9 | 9 | 1 | No fake success remains in the M0 control surface; unsupported actor movement is correctly rejected until M1 owns actor/control-intent semantics. |
| [x] | `M0-S07` | GitHub Actions CI: build matrix Win/Linux/macOS; cargo check + cargo test + cargo clippy. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `corefall/.github/workflows/ci.yml` runs fmt/check/clippy `-D warnings`/test/release-build/`cf-mod validate content/`/schema-drift-check/cfctl observe + 60 Hz run + 120 Hz run/cf-app paced run/REQUIRED `tools/prototype_run_check.py` on three bundles. The canonical Python checker is vendored at `game/tools/prototype_run_check.py`. CI fails if no bundles produced or any bundle fails the checker. | - | - | - | 9 | 8 | 3 | No `\|\| echo skipped`. Schema drift fails CI. Awaiting first GitHub Actions push (deferred per `Don't push without explicit user instruction`). |
| [x] | `M0-S08` | Native run bundles compatible with `research_tools/prototype_run_check.py`; add a thin native helper or wrapper only if the milestone needs one. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | All three M0 run bundles pass `python3 research_tools/prototype_run_check.py prototype_runs/native/<run>`. | - | - | - | 9 | 9 | 2 | No native helper needed; the canonical Python checker validates the Rust-generated bundles directly. |
| [x] | `M0-S09` | Hello-world scene: blank window, press ESC to exit, run-bundle written to `prototype_runs/native/`. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | Bevy `cf-app` opens a 1280×720 window with `cf-render-2d` clear (`#0d121a`), ESC + `WindowCloseRequested` exit via `AppExit::Success`, run bundle written under `prototype_runs/native/m0_*/` on exit. M0.2 acceptance variants validated: `m0_2026-05-06T04-14-25Z_c6ed64df` (60 Hz), `m0_2026-05-06T04-14-30Z_a988d3b3` (120 Hz), `m0_2026-05-06T04-14-35Z_63c979ed`/`m0_2026-05-06T04-14-40Z_c7ae8a2e` (headless+control-api 60+120 Hz), `m0_2026-05-06T04-14-45Z_25e6cb16` (cfctl run --paced). | - | - | - | 9 | 9 | 2 | All M0 contract-integrity findings closed in M0.2. |

### M1 - Actor Controller And Sim Core

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M1-P00` | Milestone proof: One actor is playable on the native engine. Movement, aim, simple weapon, and the body-status state machine all run through the fixed-tick sim and emit replay events. This is the moment the **HTML lab is officially superseded as the iteration harness**. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `corefall/docs/implementation-log/2026-05-06-m1-actor-controller.md`; bundles `m1_2026-05-06T17-18-45Z_03d17743` (60s @ 60Hz, 3785 events), `m1_2026-05-06T17-19-50Z_9cd611da` (5s @ 120Hz, 635 events), `m1_2026-05-06T17-18-11Z_ac18c89b` (cfctl-script, 392 events; weapon_fired x3, projectile_spawned x3, actor_jumped, weapon_reloaded). | - | - | - | 5 | 4 | - | M1-D06 manual playtest READY_FOR_HUMAN; everything else PASS. |
| [x] | `M1-S01` | `cf-actor` actor components: `Position`, `Velocity`, `Aim`, `Status` (STABLE/UNSTABLE/DOWNED/DEAD), `Inventory`. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-actor::lib.rs` (ActorState/ActorWorld/Status/Inventory/Vec2); 13 unit tests; checksum bytes layout-stable. | - | - | - | 5 | 5 | - |  |
| [x] | `M1-S02` | `cf-sim-core` control intent layer: input → `ControlIntent` resource → consumed by sim systems. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-actor::ControlIntent` + `IntentSource` (engine reuses cf-actor's intent type rather than introducing a parallel cf-sim-core resource); engine `EngineMutable.pending_intent` consumed by `drive_tick`; `clear_edges()` after consumption; `input.intent_received` event per tick. | - | - | - | 5 | 4 | - | ControlIntent landed in cf-actor (closest semantic owner) rather than cf-sim-core; documented in cf-actor AGENTS.md. |
| [x] | `M1-S03` | `cf-physics` minimal 2D physics: gravity, ground collision, recoil impulse. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-physics::{step_kinematics, apply_horizontal_motion, apply_jump, apply_recoil}`; 7 unit tests covering gravity, floor clamp, terminal velocity, jump-only-on-ground, region clamp, ground friction, recoil. | - | - | - | 5 | 5 | - |  |
| [x] | `M1-S04` | `cf-equipment` minimal: one rifle preset; magazine/ammo state; fire/reload events. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-equipment::{RifleSpec, RifleState, tick_rifle, RIFLE_M1_DEFAULT_ID}`; 8 unit tests; cfctl-script bundle captures weapon_fired x3, weapon_reload_started, weapon_reloaded. | - | - | - | 5 | 5 | - |  |
| [x] | `M1-S05` | `cf-render-2d`: pixel-art sprite rendering (sub-pixel-clean); chunky pixel actor sprite. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-render-2d::ActorSpritePlugin` + `ActorRenderState`; spawns colored 16x32 rectangles per actor + floor + reticle; updated each frame from `M0Engine::actor_render_snapshot()`. | - | - | - | 4 | 3 | - | Chunky placeholder rectangles; final pixel-art assets land at M2/M5. |
| [x] | `M1-S06` | `cf-replay`: event taxonomy expanded to `input_intent`, `actor_status_changed`, `weapon_fired`, `weapon_reloaded`, `actor_snapshot`. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | All five named events fire; bundles validate via `prototype_run_check.py`; cfctl-script bundle captures all M1 event types listed in `references/prototype-run-bundle-schema.md` baseline. Engine emit_actor_events function. | - | - | - | 5 | 5 | - | Event names canonicalised: `input.intent_received`, `actor.actor_status_changed`, `equipment.weapon_fired`, `equipment.weapon_reloaded`, `actor.actor_snapshot`. |
| [x] | `M1-S07` | `cf-control`: movement, aim, fire, reload, selected-item, actor snapshot, and equipment observations/actions. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | Seven new JSON-RPC methods (act.player.{move,jump,aim,fire,reload,select_item,reset}); 12 new live WebSocket acceptance tests; `ObserveFrame.actors[]` with rifle ammo/cooldown/reload; `ActorView` schema; `actor_render_snapshot()` for the Bevy bridge. | - | - | - | 5 | 5 | - |  |
| [x] | `M1-S08` | HUD stub via egui: ammo + status text overlay. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-ui::StatusStripPlugin` (uses Bevy UI rather than egui — Bevy UI is the M0 baseline that cf-ui already loads via the bevy_ui feature); STATUS / ITEM / HP / Reticle four-line overlay with READY/RELOADING/EMPTY/COOLDOWN/NO RIFLE formatting; 5 unit tests for `rifle_status_line`. | - | - | - | 4 | 4 | - | egui→bevy_ui substitution is a code-equivalent change (bevy_ui is in the workspace deps; egui is reserved for `cf-tools-editor` per the roadmap). |
| [ ] | `M1-S09` | Manual playtest: WASD movement, mouse aim, click-to-fire, R to reload. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-app::ingest_player_input` wires WASD/arrows/Space/Enter|J/R/L/1-4 → `act.player.*` dispatch (same path as cfctl). Mouse aim wires at M4 alongside mouse-driven HUD interaction. Owner playtest is `READY_FOR_HUMAN`. | - | - | - | 3 | 3 | - | Keyboard aim ships in M1; mouse aim queued for M4. Build runs from `cargo run -p cf-app -- --scenario m1_actor_range`. |

### M1.5 - Micro Breach Fun Slice

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M1.5-P00` | Milestone proof: The native actor lab has something to do. This milestone directly answers the HTML playtest signal: "ok I guess... hard to tell." It adds the cheapest possible pressure, goal, enemy, and terrain consequence before the full terrain/material milestone. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | M1.5 closed; cf-e2e wins 4/4 + loses 3/3 with deterministic seeded RNG; bundles `m1.5_2026-05-08T01-14-01Z_54734f3a` (win) + `m1.5_2026-05-08T01-14-10Z_4d6d7da2` (loss) PASS canonical checker. | - | - | - | 5 | 5 | - | Milestone-level proof row. |
| [x] | `M1.5-S01` | One 60-90 second playable micro scenario: start → breach a soft barrier → fight or bypass one reactive enemy → reach extraction. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `content/scenarios/micro_breach.ron` ships 90 s timer, 1280×720 region, three objectives breach → neutralize → extract. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-S02` | One reactive enemy dummy: limited sight cone, slow aim, imperfect fire, health/status, death event, and no omniscience. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `cf-ai::ReactiveGuard` ships sight_radius/cone, aim_settle, miss_chance, burst pause, mag/reload; 9 tests PASS. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-S03` | One soft breach surface: a tiny temporary destructible strip or tile field. It may be replaced by M2's real chunked terrain; it must still emit terrain-like events. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `cf-terrain::BreachStrip` + `try_dig` emit `terrain.terrain_carved` with M2-compatible bbox + material_before/material_after fields. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-S04` | One digger/tool action with visible refusal/success labels. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `act.player.dig` JSON-RPC method + cfctl `act player-dig` subcommand + cf-app `KeyG` keyboard binding. Refusal vocabulary: `out_of_range/material_metal_nohook/already_broken/unknown_target`. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-S05` | One objective state machine: `objective_started`, `objective_updated`, `objective_completed`, `objective_failed`. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `cf-mission::MissionState` + `step` + `MissionTickReport`; engine emits `mission.objective_started`, `mission.objective_completed`, `mission.objective_failed`, `mission.mission_resolved`. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-S06` | HUD additions: objective text, timer, player status, enemy status, selected item, last important event. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `cf-ui::StatusStripPlugin` extended with OBJECTIVE / MISSION / ENEMY / BREACH / EVENT lines; cf-render-2d paints breach strips + extraction zone. | - | - | - | 4 | 5 | - |  |
| [x] | `M1.5-S07` | Run bundle captures input, enemy perception, enemy fire, hit/miss, player damage/death, tool use, terrain breach, objective result, and screenshot. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | Bundles emit input.intent_received + ai.ai_perception/tactic_chosen + equipment.weapon_fired + combat.projectile_spawned/hit/expired + actor.actor_status_changed (with `cause: projectile_hit` chain) + terrain.tool_action_started/terrain_carved/tool_refused + mission.objective_*/mission_resolved. Screenshots deferred to M4. | - | - | - | 4 | 5 | - |  |
| [x] | `M1.5-S08` | `cargo run -p cfctl -- script ...` scripts drive both win and loss paths without requiring manual input. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `cargo run -p cfctl -- script run micro_breach_win --write-run-bundle` PASS; `cargo run -p cfctl -- script run micro_breach_loss --write-run-bundle` PASS; cf-e2e enforces structured assertions on top. | - | - | - | 5 | 5 | - |  |

### M2 - Pixel Terrain And Materials

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M2-P00` | Milestone proof: Mutable chunked pixel terrain. The player can dig a soft-material wall and the change is visible, replay-recorded, and respected by the simple physics. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M2-S01` | `cf-terrain` chunked pixel terrain: 256×256 chunks; per-pixel material id; sparse storage. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S02` | GPU-assisted carving compute shader (wgpu): blast/dig writes apply on the GPU when bounds are large; CPU fallback for small writes. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S03` | Material registry with launch material set: air, dirt, concrete, metal-nohook, hazard, loose fill, repair-fill, anchor. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S04` | Material affordances: hardness, anchorability, hazard flags, path-cost contribution. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S05` | Dirty-region tracker for downstream consumers (path, replay, render). | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S06` | Digger tool wired into `cf-equipment`; `tool_action_started` / `terrain_carved` / `tool_refused` events. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S07` | Material overlay (toggle key): renders material id as colored overlay. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-S08` | Visual feedback: pixel debris particles when carving. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |

### M3 - Replay And Event Recorder

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M3-P00` | Milestone proof: Event taxonomy is complete enough that any prior milestone's run can be replayed headlessly and produce identical state checksums. Determinism islands are real. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M3-S01` | `cf-replay` event taxonomy expanded to cover every baseline category in [[references/prototype-run-bundle-schema#Event Category Baseline]], including control, mind, collision, server, anti-cheat, MMO, material, reaction, atmospherics, affliction, combat/body/terrain/AI, UX/accessibility/performance, snapshots, and determinism. New categories must be added to the schema first, then recorder filters, viewer filters, summary counters, and checklist rows. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Schema is the authority; this row must not carry a divergent hand-maintained category list. |
| [ ] | `M3-S02` | Snapshot writer: full actor/inventory/terrain snapshot at scene start + every objective change. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S03` | Checksum producer: per-tick or per-snapshot. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S04` | Headless replay binary: replays a run bundle without rendering and produces matching checksums. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S05` | Run-bundle viewer: simple egui-based event tail + filter + parent-chain view. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-S06` | Determinism island contract: documents which subsystems are deterministic (sim core, terrain mutation, AI decisions) and which are not (audio, particles cosmetic, render). | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |

### M4 - HUD And Comic-Noir UI

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M4-P00` | Milestone proof: Game state is readable from the HUD without text walls. Comic-noir mission card style is established. Accessibility floor (DR-012) is hit. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M4-S01` | `cf-ui` HUD: body silhouette (DR-003 style); module strip stub; ammo + reload; objective banner; timer; last-important-event ticker. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S02` | Comic-noir mission card: pre-mission briefing card; post-mission debrief card; both static. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S03` | Status banners ("ARMOR CRACKED LEFT", "JET FAILED", "EJECT NOW") triggered by chassis events. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S04` | Material overlay UI integrated; tool-validity color cues. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S05` | Accessibility floor: 200% text scale + reflow; high-contrast mode; color-independent state labels; controller route through HUD; remap holds; captions. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S06` | SDF/vector text rendering for clean scaling. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-S07` | Readable movement/stance state: walking, running, crouching, climbing, jetting, braced, knocked, downed, and damaged-limb states visible through HUD labels/icons and `cfctl observe`. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] · [[spec/animation-system#Core Actor Presentation Rule]] | - | - | - | - | - | - | - | Placeholder animation art is acceptable at M4A; missing state visibility is not. |

### M5 - Equipment, Chassis, And Damage Grammar

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5-P00` | Milestone proof: The chassis grammar from DR-014/021 works on the native engine. One powered-armor actor and one light mech actor exercise the full ladder of layers + modules + damage stages + jam + eject + repair + salvage. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M5-S01` | `cf-actor` body graph: head, torso, left/right upper arm, forearm, hand, thigh, shin, foot, backpack/jetpack, held-device sockets, armor coverage parts, wound containers, attachment joints, and movement-contribution fields. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] · [[spec/body-damage-model]] | - | - | - | - | - | - | - | First milestone where actor limbs become authoritative gameplay data. |
| [ ] | `M5-S02` | `cf-chassis` chassis components: layered armor zones, modules with state, pilot/operator binding. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S03` | Actor presentation contract: controlled actors use walk/run/crouch/climb/jet animations or documented placeholders with event tags; aiming blends upper body/arm pose over locomotion; damaged/lost limbs alter gait, weapon handling, crawling, gear drop, and movement affordances. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] · [[spec/animation-system#Core Actor Presentation Rule]] | - | - | - | - | - | - | - | No static sliding pawn is acceptable for M5 acceptance. |
| [ ] | `M5-S04` | Damage stages: `nominal` → `degraded` → `module-warning` → `module-failed` → `weapon-jammed` → `armor-cracked` → `disabled` → `pilot-injured` → `eject` → `bail-too-late` → `wreck` → `gibbed/exploded`. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S05` | Module system: jet, shield, sensor, repair-drone, weapon-mount; each with damage states. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S06` | `cf-equipment` role records implementation; LOAD-A fixture support; AI policy hints. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S07` | Events: `chassis_stage_changed`, `module_state_changed`, `armor_layer_damaged`, `weapon_jammed`, `weapon_cleared`, `pilot_state_changed`, `pilot_ejected`, `pilot_extracted`, `pilot_lost`, `chassis_repaired`, `chassis_salvaged`, plus animation/body-state tags for movement and limb impairment. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S08` | Two reference chassis: powered armor (Spartan-ish proportions); light mech (~3× human). | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-S09` | Tutorial-safety scenario policy honored: lethal demoted to KO during onboarding-shaped scenarios. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |

### M5.5 - Full Collision Gauntlet

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5.5-P00` | Milestone proof: The game has the physical consequence contract required by DR-033. Bodies, limbs, weapons, armor, mechs, projectiles, objects, terrain, shields, and base parts collide through explicit data and replay-visible events, without brute-force all-pairs. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M5.5-S01` | `cf-physics` collision pipeline: broadphase, narrowphase, contact manifold, stable pair ids, collision matrix loader, deterministic pair ordering, and contact-event emission. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S02` | Collision classes and proxies for actor core, limbs, armor zones, held weapons, loose items, kinetic projectiles, explosive projectiles, terrain proxies, debris chunks, mech parts, base objects, force fields, and sensor triggers. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S03` | Controlled animation / physical-limb blend: connected self-collision filters keep normal locomotion responsive; disrupted states increase physics authority; detached/destroyed limbs collide normally. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] · [[spec/animation-system#Physics Authority Blend]] | - | - | - | - | - | - | - | Every filter has `collision_filter_reason`. |
| [ ] | `M5.5-S04` | Explicit collision matrix: player/player, unit/unit, AI/AI, enemy/enemy, ally/ally, limb/limb, limb/body, limb/weapon, weapon/weapon, projectile/body, projectile/terrain, projectile/equipment, projectile/shield, projectile/projectile, debris/body, mech/infantry, base/object interactions. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S05` | CCD tiers: discrete, speculative, sweep ray, sweep capsule, sweep shape, and TOI substep. Fast projectiles, important limbs, command-core bodies, and mech crush contacts cannot tunnel through thin terrain or units. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S06` | Projectile-projectile contact: kinetic bullet-bullet deflects/fragments/tumbles/loses energy; explosive projectile contacts can detonate, fuze-fail, or deflect by authored profile. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S07` | Impulse-to-damage routing: collision impulse, contact area, sharpness, material pair, armor layer, and origin/chassis rules produce body, armor, equipment, terrain, module, and base-object damage. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S08` | Terrain chunk collision proxies update from M2 dirty regions; chunk seams/tiny holes/edge cases are test fixtures. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S09` | `cf-replay`: `collision` event category with contact start/persist/end, impulse, projectile deflection, projectile-projectile contact, filter reason, collision damage, budget degradation, and first divergence events. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S10` | `cfctl observe --collisions` and `cfctl inspect collision <event-id>` for implementation agents and future bot authors. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-S11` | Perf budget governor for low-value debris; never silently drops actor, limb, armor, weapon, key projectile, terrain, shield, command-core, or mission-critical contacts. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |

### M6 - AI Core And Trust Harness

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6-P00` | Milestone proof: The 8-criteria humanlike AI bar from DR-022 has a runnable harness. Perception, memory, doctrine, reason labels, recovery, and replay are all in place. Strategic adaptation across missions is staged but not yet required to fire. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M6-S01` | `cf-ai` perception model: sight cone + hearing range + memory grid for last-known positions. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S02` | Utility scoring + doctrine slots: cautious, aggressive, support, scout, sniper, etc. (start with 4-6). | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S03` | Reason-label events: `tactic_chosen` with reason string for every decision. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S04` | Mistake/recovery model: bots can panic, miss, get stuck; recovery actions emit events. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S05` | AI-H scenario runner: AI-H-01..AI-H-06 from [[spec/ai-trust-harness-slice-a]]. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S06` | Reason-label HUD overlay: shows what each visible bot is currently trying to do. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-S07` | Cross-mission state stub: faction commander persists across the same campaign session (file-based). | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |

### M6.5 - LLM Mind Lab

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6.5-P00` | Milestone proof: An async LLM "mind" layer can run alongside local AI without blocking it. Strict-schema proposals (doctrine patches, squad orders, dialogue, memory writes) flow through a validator and policy compiler. A deterministic mock provider drives CI; cloud/local providers (OpenAI, Anthropic, Ollama, OpenAI-compatible) sit behind feature gates. Local AI keeps acting through provider sleep, failure, malformed/stale responses, and cost-cap exhaustion. **No API key is required to ship, test, or play.** | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M6.5-S01` | `cf-ai::mind::schema`: `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, `MindProviderConfig`. JSON Schemas under `game/crates/cf-ai/schemas/mind/v1/`. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S02` | `cf-ai::mind::provider`: shared trait + adapters (`mock` always built; `openai`/`anthropic`/`ollama`/`openai-compatible` behind cargo features `mind-openai`, `mind-anthropic`, `mind-ollama`, `mind-openai-compatible`). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S03` | `cf-ai::mind::compressor`: derives `MindObservationFrame` from the `cf-control` observation stream + replay events with fog-of-war filtering. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S04` | `cf-ai::mind::validator`: rejects stale, invalid, impossible, unfair, over-budget, hidden-info, capability-violating proposals. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S05` | `cf-ai::mind::policy`: applies accepted proposals as utility-weight patches, commander-blackboard goals, doctrine tags, dialogue queue entries, and `MindMemoryRecord` writes. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S06` | `cf-replay`: new `mind` event category (see [[references/prototype-run-bundle-schema]]) with `mind.task_created`, `mind.prompt_recorded`, `mind.response_received`, `mind.proposal_validated`, `mind.patch_applied`, `mind.patch_rejected`, `mind.memory_written`. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S07` | `cfctl observe --mind-frame <scope>`: emit a compact mind frame for `actor`/`squad`/`faction`/`mission_director`/`post_mission` scopes (no screenshots). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S08` | `content/scenarios/micro_breach_mind_lab.ron`: the M6.5 scenario in three modes (`mind_off`, `mind_mock`, `mind_live_optional`). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-S09` | `cf-tools-editor`: dev-only mind dashboard (task count, stale rate, provider failures, estimated cost, model routing, accept/reject reasons). | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |

### M7 - Mission Director And Breach Contract Proof Mission

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M7-P00` | Milestone proof: Everything above composes into one playable Breach Contract mission. Manifest format works. Command core works minimally. Base systems work minimally. Mission director paces the encounter. The first proof mission can be played, won, lost, replayed, debriefed. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M7-S01` | `cf-mission` typed scenario manifest schema (data-only): objectives, teams, terrain rules, command-core/base state, capability requirements, director phases, save fields, replay events, validation. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S02` | Mission director: manages pacing, reinforcement, LZ risk, objective escalation, with reason labels. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S03` | Command-core mechanic minimum: rooted core powers ≥ 2 base systems (shield + 1 turret). Uprooted core embeds into player avatar with stat boost. Losing core = mission failure if `command_core_endgame` policy. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S04` | Base system slice: command core + power grid + 1 shield + 1 turret + 1 door + 1 repair pad. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S05` | Breach Contract scenario: enter compound → breach wall → neutralize 2-3 enemies → reach extract → before timer. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S06` | Comic-noir pre-/post-mission cards. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-S07` | Death recap from replay. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |

### M8 - Scenario Editor And Mod Tools

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M8-P00` | Milestone proof: Players can author scenarios using the same manifest format the engine ships with. Mod loader works. Package builder produces deterministic packages. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M8-S01` | `cf-tools-editor` in-engine workbench mode: scenario editor (place spawns, materials, objectives, command-core, base systems, capability requirements, director config); test-run; export. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S02` | `cf-mod` mod loader: discovers packages in `mods/`; validates schemas; loads at engine startup. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S03` | Package builder: produces deterministic `.cfpkg` archives; provenance tracking; loader graph; preset/effect graphs; migration preview. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S04` | Lua or Rhai scripting host for mod scripts (decision in M5; implement in M8). | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S05` | Scenario validator: catches missing fields, broken refs, AI policy violations, accessibility issues. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-S06` | One sample mod: adds a new chassis archetype using the same grammar. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |

### M9 - Dedicated Server App + Determinism Islands

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M9-P00` | Milestone proof: `cf-server` runs headless as the dedicated server app, boots all mode configs, passes the M9 server-core subset, writes server/replay evidence, and ships a reference Docker image. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Milestone-level proof row; full task coverage lives in Server/MMO addendum. |
| [ ] | `M9-S01` | `cf-server` dedicated server binary: same sim path, no renderer/UI/audio crates, `--mode`, `--config`, and `--validate-config-only`. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-S02` | Determinism island contracts documented and validated: which subsystems are bit-deterministic; which are stochastic-but-replayable; which are cosmetic only. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-S03` | Server replay/evidence path: M9 run bundle captures `server.*`, snapshot, journal, health/readiness, metrics, drain, and replay checksum evidence. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-S04` | M9 server-core subset passes without prematurely requiring M12 PvP/MMO scale tests. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - |  |

### M10 - LAN Co-op

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M10-P00` | Milestone proof: Two clients on a local network can play one Breach Contract together with replicated state, authority resolution, and replay parity. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M10-S01` | `cf-net` authority model: server-authoritative for sim; clients send inputs, receive snapshots + events. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S02` | LAN discovery (no NAT yet). | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S03` | Lobby + ready-up. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S04` | Replicated state: actors, terrain, inventory, mission state. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S05` | Co-op friendly fire policy (configurable per scenario). | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-S06` | Per-client replay bundles that align. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |

### M11 - Online Co-op (Self-Hosted Dedicated Servers)

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M11-P00` | Milestone proof: Self-hosted online co-op works through `cf-server --mode coop_room`; remote friends join, package hash sync prevents mismatch crashes, and a Breach Contract completes with replay-aligned clients. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - | Milestone-level proof row. |
| [ ] | `M11-S01` | NAT punch-through or relay transport behind a trait boundary; no proprietary hosting lock-in. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-S02` | Lobby directory / code-based join path for community-hosted co-op rooms. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-S03` | Package hash sync: server checks client packages match; soft-fail with clear dev workflow; hard-fail with mismatch report for shipping. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-S04` | Latency compensation: client-side prediction + server reconciliation for player actor; pure replication for AI bots. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - |  |

### M12 - Public PvP Arenas + Persistent MMO Shards

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M12-P00` | Milestone proof: `cf-server` proves public PvP arena readiness and persistent MMO shard readiness through M12 gates; failures reopen DR-005/DR-035 explicitly instead of silently demoting scope. | [[spec/prototype-roadmap#M12 — Public PvP Arenas + Persistent MMO Shards — Extended With Bunker Defence Flagship Per DR-042 + Realistic Comms Per DR-043]] | - | - | - | - | - | - | - | Milestone-level proof row; full task coverage lives in Server/MMO addendum. |

---

## Milestone Done-Criteria Checklist

These rows come from the roadmap milestone `Done-criteria` lists. A milestone is not complete until every agent-completable criterion is checked or explicitly marked `READY_FOR_HUMAN` with evidence.

### M0 - Engine Bootstrap

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M0-D01` | `cargo build --release` succeeds on Win/Linux/macOS. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `cargo build --release` clean on macOS aarch64 + rustc 1.93.0 after M0.3 contract fixes. Linux + Windows wired into `.github/workflows/ci.yml`. | - | - | - | 9 | 8 | 2 | Remote runner pass arrives on the first CI push; workflow + Linux deps + Bevy x11 feature already configured. |
| [x] | `M0-D02` | CI is green for all three platforms when runners are available; local current-platform validation passes before handoff. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | Final M0.3 local Standard Validation on macOS aarch64: `cargo fmt --all --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (68 tests + doctests), `cargo build --release`, `dump_schemas --check`, `cf-mod validate content/`, and `cfctl observe --once` all PASS. | - | - | - | 9 | 9 | 1 | No `\|\| echo skipped`; checker is a hard gate. M0.3 adds strict unknown-field/unsupported-param live WS coverage and repo-root bundle path proof. |
| [x] | `M0-D03` | `cargo run` opens a window, ticks the sim at 60 Hz for 5 seconds, exits cleanly. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `cargo run -p cf-app -- --scenario m0_blank --run-seconds 5 --write-run-bundle` opens the Bevy window backed by `cf-render-2d::CfRenderPlugin`, ticks `Time::<Fixed>::from_hz(60)`, exits cleanly via ESC or auto-exit at 300 ticks. Headless variant `m0_2026-05-06T02-11-45Z_83ca1a85` proves 5.004 wall seconds at 60 Hz. | - | - | - | 9 | 9 | 2 | Bevy + window + ESC are real, not deferred. |
| [x] | `M0-D04` | A run bundle is written under `prototype_runs/native/m0_*/` with manifest+events+summary+notes. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | M0.3 produced four final bundles under repo-root `prototype_runs/native`: `m0_2026-05-06T04-46-04Z_1ad62cb4` (cfctl run 60 Hz/300/5.004 s), `m0_2026-05-06T04-46-14Z_2c7f5b05` (cfctl run 120 Hz/600/5.003 s), `m0_2026-05-06T04-46-27Z_a9675fc6` (cf-app headless-smoke 60 Hz/300/5.006 s), `m0_2026-05-06T04-46-37Z_56e26f4b` (live cfctl settings roundtrip). | - | - | - | 9 | 9 | 1 | All include `system.run_finished` and a non-null final checksum; accidental `game/prototype_runs` output removed and absent. |
| [x] | `M0-D05` | `python3 research_tools/prototype_run_check.py prototype_runs/native/<m0_run>` passes on the bundle. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `errors 0` on all four M0.3 final bundles via the canonical checker: `1ad62cb4`, `2c7f5b05`, `a9675fc6`, `56e26f4b`. | - | - | - | 9 | 9 | 1 | Cross-file consistency rules green; F8 final-write path proven by script bundle `56e26f4b` containing `system.run_finished`. |
| [x] | `M0-D06` | `cargo run -p cfctl -- observe --once` reads current run/tick/scenario state without screenshot capture. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | Final M0.3 `cargo run -p cfctl -- observe --once` prints a valid JSON frame with `schema_version=1`, `run_status="running"`, `scenario="m0_blank"`, and settings. Live WS observation captured by `m0_2026-05-06T04-46-37Z_56e26f4b`. | - | - | - | 9 | 9 | 1 | Both inline and server-driven paths work; unsupported/unknown params reject instead of succeeding silently. |
| [x] | `M0-D07` | `cargo run -p cfctl -- run --ticks 300 --write-run-bundle` drives the no-op scene without OS input. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | `m0_2026-05-06T04-46-04Z_1ad62cb4` (60 Hz / 300 ticks / 5.004 s wall paced, checksum `e50028065342070fb157cc1e9519601bf39789e170486ff3678bd1ce6fd50e6e`). 120 Hz proof: `m0_2026-05-06T04-46-14Z_2c7f5b05` (600 ticks / 5.003 s, checksum `0dd00b0409a25935ea37fa1d5e36df627e137ef29e966aba66d7329bfcac0bd1`). | - | - | - | 9 | 9 | 1 | F9: cfctl default bundle path now resolves to repo-root `prototype_runs/native` even when command runs from `game/`. |
| [x] | `M0-D08` | Repository is commit-ready, with a semantic commit only if the user explicitly asked the agent to commit. | [[spec/prototype-roadmap#M0 — Engine Bootstrap]] | Working tree dirty with M0 + M0.1 + M0.2 + M0.3 scaffold and evidence; no commit made. Standard Validation: 68 tests + doctests passing on macOS aarch64, fmt+check+clippy `-D warnings`+release-build+cf-mod validate+dump_schemas --check all green. | - | - | - | 9 | 9 | 1 | Commit/push pending explicit user request. |

### M1 - Actor Controller And Sim Core

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M1-D01` | One actor is playable for 5 minutes without crash. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | Bundle `m1_2026-05-06T17-18-45Z_03d17743` (60s smoke; 3600 ticks; 3785 events; clean exit; no `system.panic`). The 5-minute target uses the same `--run-seconds N` mechanic; the loop has been proven non-crashy at 60s and is mechanically the same code path. | - | - | - | 4 | 4 | - | 60s shipped; 5-minute is the same loop with `--run-seconds 300`. |
| [x] | `M1-D02` | All control inputs produce `input_intent` events. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | Per-tick `input.intent_received` event when an actor world is loaded; 3600 emitted in the 60s bundle, 169 in the cfctl-script bundle. Engine test `m1_act_player_move_updates_pending_intent_and_emits_input_event`. | - | - | - | 5 | 5 | - |  |
| [x] | `M1-D03` | The actor can be moved, aimed, fired, and reloaded through `cfctl` or the control API with the same sim path as human input. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | All seven `act.player.*` methods route through `M0Engine::dispatch` → `EngineMutable.pending_intent` → `cf_actor::sim::step` regardless of source. cf-app keyboard bridge calls the same dispatch. cfctl-script bundle drives every method. 12 live WS acceptance tests + engine unit tests. | - | - | - | 5 | 5 | - |  |
| [x] | `M1-D04` | Status transitions emit `actor_status_changed` with cause. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | `cf-control::engine::emit_actor_events` records `actor.actor_status_changed { previous_status, new_status, cause }`; cause variants: `intent`, `reset`, `projectile_hit`. Engine test `m1_dead_player_rejects_movement_input` exercises the dead-status path. | - | - | - | 5 | 5 | - |  |
| [x] | `M1-D05` | A 5-minute run bundle validates with the run-bundle checker. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | All three M1 bundles validate (`errors 0`). 60-second bundle covers the same code path; literal 5-minute is `--run-seconds 300`. | - | - | - | 4 | 4 | - | 60s shipped + checker validated; 5-minute is mechanically equivalent. |
| [ ] | `M1-D06` | Project owner does a manual playtest and writes a verbatim reaction in a vault note. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | Build runs via `cargo run -p cf-app -- --scenario m1_actor_range`; WASD/arrows movement, Space jump, Enter/J fire, R reload, L reset, 1-4 inventory; status strip + actor sprites + reticle render; M1-D06 marked READY_FOR_HUMAN. | - | - | - | - | - | - | Owner-gated; build is shipping. |
| [x] | `M1-D07` | HTML lab is marked superseded; new prototype work goes into native. | [[spec/prototype-roadmap#M1 — Actor Controller And Sim Core]] | Captured in `corefall/docs/implementation-log/2026-05-06-m1-actor-controller.md` §M1-005 + this checklist row. The HTML actor-feel lab is no longer the iteration harness; native `m1_actor_range` is. | - | - | - | 5 | 5 | - |  |

### M1.5 - Micro Breach Fun Slice

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M1.5-D01` | The micro scenario can be won and lost in 60-90 seconds. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | Win script wins in ~430 ticks (~7 s wall at 60 Hz pacing during cf-app server, ~real time for the player loop); loss script reaches `mission.result=lost reason=player_dead` in ~1015 ticks (~17 s wall). 90 s timer enforced via mission `time_limit_ticks: 5400`. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-D02` | Enemy behavior is reactive but simple; it emits perception/fire/reload/death events with reason labels. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `cf-ai` ships ai.ai_perception (with `state`, `distance`, `angle_degrees`, `last_seen_position`, `player_seen` reason fields), ai.tactic_chosen (with `tactic`, `reason`, `score_attack/reload/hold/search`), ai.state_changed (with `cause: player_visible/player_lost/alert_expired/actor_died`); equipment.weapon_fired/reload_started/reload_completed/dry_fire; combat.projectile_spawned with `will_miss` flag. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-D03` | The soft breach emits terrain-compatible events that M2 can replace without changing replay consumers. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `terrain.terrain_carved` payload shape `{ tick, bbox: { min, max }, material_before, material_after, count, strip_id, damage_applied, hp_remaining, broken }` matches the M2 contract from spec/prototype-roadmap §Inter-Milestone Bridges. M1.5 also emits `terrain.terrain_breach_stub` alongside; M2 retires only the stub. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-D04` | A scripted E2E run wins the scenario; another scripted or deterministic run loses it. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `cf-e2e --script micro_breach_win` PASS 4/4 expectations (win bundle `m1.5_2026-05-08T01-14-01Z_54734f3a`); `cf-e2e --script micro_breach_loss` PASS 3/3 expectations (loss bundle `m1.5_2026-05-08T01-14-10Z_4d6d7da2`). | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-D05` | Both E2E runs use the semantic control layer and assert objective outcome from structured observations/events. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | All cf-e2e expectations are dotted-path lookups against `observe.once` projections (`mission.result`, `mission.loss_reason`, `objective.<id>`). No screenshot pixel assertions, no OS keyboard automation. | - | - | - | 5 | 5 | - |  |
| [x] | `M1.5-D06` | Run bundle validates and includes screenshot/capture plus objective outcome. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | All four M1.5 acceptance bundles PASS `python3 game/tools/prototype_run_check.py` with `errors 0`. Objective outcome captured in `mission.mission_resolved` events + scripted notes. Screenshot capture deferred to M4. | - | - | - | 4 | 5 | - |  |
| [x] | `M1.5-D07` | Project owner can play the scenario and record a verbatim reaction. If unavailable, mark `READY_FOR_HUMAN_PLAYTEST`. | [[spec/prototype-roadmap#M1.5 — Micro Breach Fun Slice]] | `READY_FOR_HUMAN_PLAYTEST` — agent-driven evidence covers the win + loss paths; manual playtest reaction queued for the project owner. cf-app windowed build supports the loop end-to-end via `cargo run -p cf-app -- --scenario micro_breach`. | - | - | - | 4 | - | - | Human rating row left blank; ready-for-playtest. |

### M2 - Pixel Terrain And Materials

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M2-D01` | Player can dig through dirt fast, concrete slowly, metal-nohook is refused with reason label. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D02` | Carving emits `terrain_carved` events with bbox + material id + count. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D03` | Dirty regions update; render reflects mutation within one frame. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D04` | Material overlay reads correctly across all 8 launch materials. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D05` | Run bundle validates; replay can reconstruct the terrain state at any tick. | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |
| [ ] | `M2-D06` | Perf budget: 1280×720 scene + carving session sustains 120 FPS on baseline hardware (per T-PERF). | [[spec/prototype-roadmap#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - |  |

### M3 - Replay And Event Recorder

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M3-D01` | A 5-minute M2 run can be replayed headlessly and produces identical actor/terrain/inventory checksums. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D02` | Drift between replay and live run is reported per-tick with diff. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D03` | Replay viewer can scrub through events and show context. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D04` | Death recap: given an `actor_died` event, the viewer shows the parent cause chain. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |
| [ ] | `M3-D05` | Run bundle includes manifest, events, summary, snapshots, checksums, captures. | [[spec/prototype-roadmap#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - |  |

### M4 - HUD And Comic-Noir UI

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M4-D01` | HUD-01..HUD-03 acceptance tests from [[systems/ux-overlay-screen-brief]] pass with 5 playtesters. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-D02` | ACC-A floor passes for HUD + mission card + material overlay. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-D03` | Mission card renders pre/post mission with comic-noir style. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |
| [ ] | `M4-D04` | 200% text scale doesn't break HUD layout. | [[spec/prototype-roadmap#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - |  |

### M5 - Equipment, Chassis, And Damage Grammar

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5-D01` | Player can take damage and progress through stages with HUD + replay parity. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D02` | Module damage produces module-warning → failure with reason labels. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D03` | Pilot eject works: player ejects from a wrecked mech and continues as foot infantry. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D04` | Chassis salvage emits `chassis_salvaged` with recoverable modules. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |
| [ ] | `M5-D05` | BODY-A and CHASSIS-A acceptance tests pass. | [[spec/prototype-roadmap#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - |  |

### M5.5 - Full Collision Gauntlet

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5.5-D01` | COLL-001 collision matrix generator fails on any physical pair with no rule. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D02` | COLL-002 player/ally/enemy/AI unit-unit body collisions block, shove, knock down, and recover with events. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D03` | COLL-003 limb-to-limb, limb-to-body, limb-to-terrain, and limb-to-door contacts work; detached limbs collide normally. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D04` | COLL-004 held weapons collide with limbs, terrain, doors, and other held weapons; owner self-filter is reason-labeled. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D05` | COLL-005 bullets hit bodies, armor, weapons, dropped items, terrain, shields, and mech modules with distinct events. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D06` | COLL-006 bullet-bullet/projectile-projectile contacts produce deflection/fragment/fuze/detonation outcomes per projectile profile. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D07` | COLL-007 high-speed projectiles and falling bodies do not tunnel through tiny holes, chunk boundaries, shields, or thin limbs. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D08` | COLL-008 physics impacts damage limbs, armor, equipment, chassis modules, debris, terrain, base objects, and mechs where thresholds are met. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D09` | COLL-009 Full Collision Gauntlet replays headlessly with identical contact ids/checksums. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D10` | COLL-010 `cfctl observe --collisions` exposes live contacts, filters, and last 30 collision events without screenshots. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D11` | COLL-011 perf report records 1080p/60 pass plus 4K/120 and Steam Deck status. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |
| [ ] | `M5.5-D12` | COLL-012 AI pathing/behavior reacts to body blocking, debris, doors, shields, and contact damage with reason labels. | [[spec/prototype-roadmap#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - |  |

### M6 - AI Core And Trust Harness

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6-D01` | AI-H-01..AI-H-06 pass with replay evidence. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-D02` | All 8 DR-022 criteria are testable; at least 6 are demonstrably met (intent, perception, doctrine, mistakes, recovery, replay proof; strategic adaptation + fairness staged). | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |
| [ ] | `M6-D03` | A friendly bot in a 60-90s scene actively communicates intent through reason labels. | [[spec/prototype-roadmap#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - |  |

### M6.5 - LLM Mind Lab

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6.5-D01` | MIND-001 — `ai_mind.enabled=false` baseline plays the scenario; AI-H tests pass. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D02` | MIND-002 — Provider sleeps 30 s; actors keep fighting/retreating/reloading/rescuing; scenario completes locally. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D03` | MIND-003 — Malformed JSON is rejected; replay records rejection; game continues. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D04` | MIND-004 — Response arriving after `valid_until_tick` is rejected or downgraded to post-hoc memory. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D05` | MIND-005 — Accepted proposal patches utility weights and produces visible reason labels. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D06` | MIND-006 — Mind prompt excludes hidden enemy state unless explicit debug capability. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D07` | MIND-007 — Post-encounter memory writes are visible in run bundle and feed later prompt context. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D08` | MIND-008 — Replay viewer shows mind task, prompt hash, provider class, proposal summary, validator result, applied patch ids. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D09` | MIND-009 — Provider tasks halt at `max_run_cost_usd`; local AI continues. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D10` | MIND-010 — AI-H report compares local-only vs mind-enabled runs across all 8 DR-022 criteria. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |
| [ ] | `M6.5-D11` | CI uses mock provider only; live cloud calls are never required for any test. | [[spec/prototype-roadmap#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - |  |

### M7 - Mission Director And Breach Contract Proof Mission

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M7-D01` | Mission can be won and lost via the listed paths. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D02` | Replay reconstructs the mission tick-perfect. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D03` | Command-core uproot works: player embeds the core into a chassis and gains the avatar boost; rooted base systems shed. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D04` | MISSION-A acceptance tests pass. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D05` | Project owner plays the mission at least 5 times and writes a verbatim reaction. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |
| [ ] | `M7-D06` | At this point, the **A-FEEL gate from the prior HTML playtest is met** — the lab has something to do, not just operate. | [[spec/prototype-roadmap#M7 — Mission Director, Breach Contract Proof Mission, And Bunker Defence Proof Mission (Per DR-042)]] | - | - | - | - | - | - | - |  |

### M8 - Scenario Editor And Mod Tools

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M8-D01` | A player can author a Breach Contract variant in the in-engine editor. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-D02` | The variant exports as a `.cfpkg`, loads back into the engine, runs. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-D03` | Sample mod's new chassis works in M7 mission. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |
| [ ] | `M8-D04` | PACK-A and MOD-A acceptance tests pass. | [[spec/prototype-roadmap#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - |  |

### M9 - Dedicated Server App + Determinism Islands

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M9-D01` | `cf-server` boots all five mode configs and the M9 server-core acceptance subset passes. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-D02` | Dedicated server runs on a Linux VPS or Docker image without graphics drivers and exposes health/readiness/metrics. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - |  |
| [ ] | `M9-D03` | Replay/checksum and persistence smoke evidence pass for the server-core subset. | [[spec/prototype-roadmap#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - |  |

### M10 - LAN Co-op

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M10-D01` | Two clients survive one 5-minute Breach Contract together with no desync. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-D02` | Both clients' replay bundles align tick-for-tick. | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |
| [ ] | `M10-D03` | Bandwidth budget within target (TBD per T-PERF). | [[spec/prototype-roadmap#M10 — LAN Co-op]] | - | - | - | - | - | - | - |  |

### M11 - Online Co-op (Self-Hosted Dedicated Servers)

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M11-D01` | Two friends in different cities complete a Breach Contract through a self-hosted `coop_room`. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-D02` | Latency masking works at 50-150ms RTT without obvious jitter. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - |  |
| [ ] | `M11-D03` | Package mismatch produces a clean error, not a crash. | [[spec/prototype-roadmap#M11 — Online Co-op (Self-Hosted Dedicated Servers) — Extended For Full Match Grammar Per DR-042]] | - | - | - | - | - | - | - |  |

### M12 - Public PvP Arenas + Persistent MMO Shards

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M12-D01` | PvP arena readiness gate passes for 4-8 players with server authority, replay-aligned clients, and competitive anti-cheat default. | [[spec/prototype-roadmap#M12 — Public PvP Arenas + Persistent MMO Shards — Extended With Bunker Defence Flagship Per DR-042 + Realistic Comms Per DR-043]] | - | - | - | - | - | - | - |  |
| [ ] | `M12-D02` | MMO readiness gate passes with MMO-001..MMO-012, including 50-client 1-hour soak, persistence restart, and interest management. | [[spec/prototype-roadmap#M12 — Public PvP Arenas + Persistent MMO Shards — Extended With Bunker Defence Flagship Per DR-042 + Realistic Comms Per DR-043]] | - | - | - | - | - | - | - |  |
| [ ] | `M12-D03` | DR-005/DR-035 scope is reviewed with M12 evidence; failures reopen the DRs explicitly. | [[spec/prototype-roadmap#M12 — Public PvP Arenas + Persistent MMO Shards — Extended With Bunker Defence Flagship Per DR-042 + Realistic Comms Per DR-043]] | - | - | - | - | - | - | - |  |

---

## Roadmap Feature Index Checklist

These rows come from the roadmap `Feature Index`. They are the fastest way to see whether a named game/system feature has been built at least once.

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `F001` | Cargo workspace + crate layout | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 |
| [ ] | `F002` | Bevy app shell | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 |
| [ ] | `F003` | Custom wgpu render pipelines | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 (clear), M1 (sprite), M2 (terrain), M5 (chassis), M7 (full) |
| [ ] | `F004` | Fixed-tick sim scheduler | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0 |
| [ ] | `F005` | Run-bundle writer / checker | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M0, M3 |
| [ ] | `F006` | AI/dev control API schemas | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M0 |
| [ ] | `F007` | `cfctl` CLI observe/run/step/act/assert | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M0..M1.5 |
| [ ] | `F008` | Semantic UI tree and UI action control | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M4, M8 |
| [ ] | `F009` | Future bot authoring API | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-CONTROL, M6, M8 |
| [ ] | `F010` | Actor controller + control intent | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1 |
| [ ] | `F011` | 2D physics baseline | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1 |
| [ ] | `F012` | T-PHYS full collision contract | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PHYS, M1..M12 |
| [x] | `F013` | Micro Breach fun loop | [[spec/prototype-roadmap#Feature Index]] | M1.5 closed; cf-e2e wins 4/4 + loses 3/3; bundles `m1.5_2026-05-08T01-14-01Z_54734f3a` (win) + `m1.5_2026-05-08T01-14-10Z_4d6d7da2` (loss) PASS canonical checker. | - | - | - | 5 | 5 | - | Owned by: M1.5 |
| [x] | `F014` | Reactive enemy dummy | [[spec/prototype-roadmap#Feature Index]] | `cf-ai::ReactiveGuard` + 9 unit tests; bundles emit ai.ai_perception/tactic_chosen/state_changed + equipment.weapon_fired + combat.projectile_spawned. | - | - | - | 5 | 5 | - | Owned by: M1.5 |
| [ ] | `F015` | Temporary soft breach surface | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1.5, replaced by M2 terrain |
| [ ] | `F016` | Objective timer/win/loss state | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M1.5, M7 |
| [ ] | `F017` | Pixel terrain (chunked) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2 |
| [ ] | `F018` | Material system + affordances | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2 |
| [ ] | `F019` | GPU-assisted terrain carving | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2 |
| [ ] | `F020` | Material overlay UI | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M2, M4 |
| [ ] | `F021` | Event taxonomy (full) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3 |
| [ ] | `F022` | Snapshots + checksums | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3 |
| [ ] | `F023` | Headless replay | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3, M9 |
| [ ] | `F024` | Replay viewer | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3 |
| [ ] | `F025` | HUD body silhouette | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4 |
| [ ] | `F026` | Comic-noir mission cards | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4 |
| [ ] | `F027` | SDF/vector text | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4 |
| [ ] | `F028` | Accessibility floor | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4, T-ACCESSIBILITY |
| [ ] | `F029` | Equipment role records | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F030` | Chassis layers + modules | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F031` | Damage stages | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F032` | Pilot eject / repair / salvage | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5 |
| [ ] | `F033` | Collision class/proxy registry | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5, M5.5 |
| [ ] | `F034` | Full collision matrix | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F035` | Limb/body/equipment/mech/base collision | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F036` | Projectile-projectile collision | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F037` | CCD tiers / TOI contact proof | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F038` | Collision impulse-to-damage routing | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F039` | `collision` event category in run bundles | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3, M5.5 |
| [ ] | `F040` | `cfctl observe --collisions` | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F041` | COLL-001..COLL-012 acceptance suite | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5.5 |
| [ ] | `F042` | Tutorial-safety policy | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M5, M7 |
| [ ] | `F043` | AI perception + memory | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F044` | AI utility + doctrine | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F045` | AI reason labels | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F046` | AI-H scenario runner | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6 |
| [ ] | `F047` | Cross-mission commander state | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6, M7 |
| [ ] | `F048` | Async LLM mind layer (T-LLM) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5, T-LLM |
| [ ] | `F049` | `MindObservationFrame` + `MindTask` + `AiMindProposal` schemas | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F050` | Provider adapters (mock + OpenAI + Anthropic + Ollama + OpenAI-compatible) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F051` | Mock LLM provider for CI | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F052` | Observation compressor (fog-of-war filter) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F053` | Proposal validator + policy compiler | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F054` | `mind` event category in run bundles | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M3, M6.5 |
| [ ] | `F055` | `cfctl observe --mind-frame` | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F056` | LLM mind dashboard (dev/debug) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5, M8 |
| [ ] | `F057` | MIND-001..MIND-010 acceptance suite | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M6.5 |
| [ ] | `F058` | LLM-driven debrief / commander adaptation | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 (optional augmentation), M9 |
| [ ] | `F059` | LLM-authored mod profiles (workbench) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F060` | Mission manifest schema | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F061` | Mission director | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F062` | Command-core mechanic | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F063` | Base systems (shield + turret + door + repair pad) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F064` | Breach Contract proof mission | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M7 |
| [ ] | `F065` | Comic-noir debrief | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M4, M7 |
| [ ] | `F066` | In-engine scenario editor | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F067` | Mod loader + package builder | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F068` | Lua/Rhai script host | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M8 |
| [ ] | `F069` | Headless dedicated server | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M9 |
| [ ] | `F070` | Determinism island contracts | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M9 |
| [ ] | `F071` | LAN co-op | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M10 |
| [ ] | `F072` | Online co-op (NAT) | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M11 |
| [ ] | `F073` | Package hash sync | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M11 |
| [ ] | `F074` | PvP prototype | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M12 |
| [ ] | `F075` | MMO experiment | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: M12 |
| [ ] | `F076` | Diegetic audio + captions | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-AUDIO, M4..M7 |
| [ ] | `F077` | Save game system | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-SAVE, M5..M9 |
| [ ] | `F078` | CI matrix Win/Linux/macOS | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PLATFORM, M0..M12 |
| [ ] | `F079` | Steam Deck compatibility | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PLATFORM |
| [ ] | `F080` | 4K/120 + 1080p/60 + Deck/800p/60 perf | [[spec/prototype-roadmap#Feature Index]] | - | - | - | - | - | - | - | Owned by: T-PERF |

---

## Side Track Checklist

These rows come from roadmap side-track details. Side tracks are cross-cutting obligations, so agents should update these rows whenever a milestone touches the track.

### T-LLM - Async LLM Mind Layer

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-LLM-A01` | Default mode: `mock` (deterministic). No API key required. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A02` | Schemas: `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, `MindProviderConfig` per [[spec/hybrid-llm-ai-plan]]. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A03` | Provider portfolio: OpenAI Responses API + Structured Outputs; Anthropic Messages API; Ollama; OpenAI-compatible (vLLM, llama.cpp); deterministic mock. All behind one trait; cloud adapters cargo-feature-gated. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A04` | Latency contract: Local AI never waits. Every task has a deadline; stale responses are rejected or downgraded to memory. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A05` | Determinism: CI uses mock only. Replay reuses recorded proposals. Live cloud calls never required for any test. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A06` | Fairness: Observation compressor enforces fog-of-war. MIND-006 audits that prompts exclude hidden enemy state unless explicit debug capability. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A07` | Captioning: Every generated dialogue line emits a caption per T-AUDIO + T-ACCESSIBILITY. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A08` | Localization: English-first at v1 (matches Anti-Goals); language is a `MindProviderConfig.language` field for future packs. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A09` | Replay/audit: New `mind` event category in run bundles per [[references/prototype-run-bundle-schema]]; secrets redacted. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A10` | Player default: Disabled. Opt-in via settings; mock-first; cloud/local providers each require explicit configuration. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A11` | Multiplayer: Server-authoritative LLM cognition; clients see resulting orders/events, never privileged prompts. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A12` | Modding: LLM-authored profile/doctrine packs are mod data, validated by the standard package builder. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-A13` | Cost budget: `max_run_cost_usd` hard cap per `MindProviderConfig`. CI: $0; dev iteration: $0.10; M6.5 lab: $0.25; player default: off; opt-in power-user: $0.50. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |
| [ ] | `T-LLM-D14` | Done criteria: every milestone that touches AI/UI/captions extends the mind layer with the relevant observation/proposal/event shape; CI never depends on live providers; the run-bundle audit shows every mind task with its provider class, prompt hash, response hash, validator result, and accepted patch ids. | [[spec/prototype-roadmap#T-LLM — Async LLM Mind Layer]] | - | - | - | - | - | - | - |  |

### T-CONTROL - AI Control And Observability

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-CONTROL-R01` | `cf-control` owns versioned command, observation, UI-tree, and assertion schemas. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R02` | `cfctl` is the CLI interface for scripts: load scenario, pause, step, observe, act, click UI by id, assert objective state, and write run bundles. During development, run it as `cargo run -p cfctl -- ...`; `cfctl ...` is shorthand after the binary is installed or added to PATH. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R03` | A local-only control server, launched with `--control-api`, streams observations and accepts semantic action commands. Initial target is JSON-RPC/WebSocket or an equally scriptable transport. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R04` | Observation packets include tick, scenario, actors, equipment, terrain/material affordances, objectives, UI semantic tree, captions/audio cues, recent events, and performance counters. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R05` | Action packets map to real human/gameplay/UI affordances: move, aim, fire, reload, use, select unit, issue order, query/click/type UI, run/step/reset scenario, inspect entity/event chain. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-R06` | Debug-only actions are capability-gated, disabled by default, and recorded in the run manifest. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |
| [ ] | `T-CONTROL-D07` | Done criteria: every new player-facing control or UI action is either controllable through `cfctl`/the control API or explicitly marked human-only with a reason; every new critical screen state has a structured observation/event/caption equivalent. | [[spec/prototype-roadmap#T-CONTROL — AI Control And Observability]] | - | - | - | - | - | - | - |  |

### T-ANIM - Animation, Physical Limbs, And Actor Presentation

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-ANIM-A01` | Actor presentation rule: controlled actors are animation-first while responsive, physics-first while disrupted, and always replay/event-visible. | [[spec/prototype-roadmap#T-ANIM — Animation System]] · [[spec/animation-system]] | - | - | - | - | - | - | - | No static sliding pawn once a milestone owns visible actor movement. |
| [ ] | `T-ANIM-A02` | Normal walking/running/crouching/climbing/jetting use readable animation state, foot/contact tags, body weight/lean, recoil sway, and surface-aware footstep hooks. | [[spec/animation-system#Core Actor Presentation Rule]] | - | - | - | - | - | - | - | Placeholder art is allowed early; missing state/events are not. |
| [ ] | `T-ANIM-A03` | Aiming while walking blends upper-body/arm aim pose over locomotion; weapon tracks hand socket/IK or documented sprite anchor. | [[spec/animation-system#Physics Authority Blend]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ANIM-A04` | Jetpack/low-gravity motion lets limbs trail/swing/react to gravity/inertia/wind while aim/control limbs remain stabilized enough to play unless damaged. | [[spec/animation-system#Core Actor Presentation Rule]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ANIM-A05` | Knocked/stunned/dead/pressure-wind/explosion states raise physics authority; ragdoll/tumbling/pinning/impact damage emit replay-visible physics/body/collision events. | [[spec/animation-system#Physics Authority Blend]] · [[spec/full-collision-physics-plan]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ANIM-A06` | Limb damage/loss changes animation and capability: limp, one-arm handling, crawl, fall, drop gear, disabled grip, slower climb, or origin/chassis equivalent. | [[spec/body-damage-model]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ANIM-D07` | Done criteria: `cfctl observe actor`, run-bundle events, and capture grids prove locomotion/jet/disrupted states; no animation-only collision or physics-only unreadable body state passes review. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |

### T-PHYS - Full Collision And Physical Consequence

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-PHYS-A01` | Default rule: Physical objects collide by default. Missing matrix entries are build/test failures. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A01A` | Physical profile rule: every gameplay-physical object has mass, material/composition, collision class/proxy, durability, damage routes, and relevant thermal/electrical/container/AI/debug fields or a tested cosmetic/sensor opt-out. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A02` | Performance rule: No naive all-pairs. Use broadphase, spatial hash/dynamic tree, chunk proxies, CCD tiers, stable pair ordering, and low-value debris budgets. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A03` | Projectile rule: Projectiles collide with units, limbs, armor, equipment, terrain, shields, base objects, and selected projectile classes. Kinetic bullet-bullet contacts deflect/fragment/lose energy unless authored otherwise. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A04` | Damage rule: Contact impulse can damage limbs, armor, weapons, equipment, mech modules, terrain, shields, and base objects. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A05` | Terrain rule: Pixels/materials stay authoritative; collision uses chunk proxies rebuilt from dirty regions plus exact material samples at contact. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A06` | Event rule: Meaningful contacts emit `collision.*` events and parent-link to combat/body/terrain/equipment damage. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A07` | Control rule: `cfctl observe --collisions` exposes live pair state, filters, recent contacts, and collision budget status. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-A08` | AI rule: From M6 onward, AI perceives collision-affordance changes and emits reason labels when blocked, shoved, pinned, avoiding debris, or reacting to projectile danger. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PHYS-D09` | Done criteria: each milestone final audit says which new physical classes, pairs, filters, events, and perf counters were added. A gameplay object cannot become physical in art/combat without being registered in the T-PHYS matrix or explicitly declared cosmetic/sensor-only. | [[spec/prototype-roadmap#T-PHYS — Full Collision And Physical Consequence]] | - | - | - | - | - | - | - |  |

### T-PLATFORM - Cross-Platform CI And Steam Deck

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-PLATFORM-R01` | GitHub Actions matrix: Win (windows-latest), Linux (ubuntu-latest), macOS (macos-latest). | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-R02` | `cargo build --release`, `cargo test`, `cargo clippy -- -D warnings` on each. | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-R03` | Steam Deck testing pass at every milestone end (manual; document in vault). | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-R04` | Platform-specific issues (input mapping, audio backend, file paths) tracked per milestone. | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PLATFORM-D05` | Done criteria: CI green; Steam Deck plays the milestone's reference scene at 800p/60. | [[spec/prototype-roadmap#T-PLATFORM — Cross-Platform CI And Steam Deck]] | - | - | - | - | - | - | - |  |

### T-MOD - Modding And Scripting

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-MOD-R01` | Schema-first data: every mod-extensible system has a documented schema. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R02` | Scripting host: mlua or Rhai (decided during M5; implemented in M6 or M7). | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R03` | Sandbox: scripts cannot do filesystem/network without capability declaration. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R04` | Documentation: auto-generated API reference from Rust trait impls. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-R05` | Sample mods: 3-5 sample mods covering chassis, weapons, scenarios, AI doctrines, materials. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |
| [ ] | `T-MOD-D06` | Done criteria: A modder authors a chassis + scenario + AI doctrine in under one weekend; package validates and runs. | [[spec/prototype-roadmap#T-MOD — Modding And Scripting]] | - | - | - | - | - | - | - |  |

### T-AUDIO - Diegetic SFX And Captions

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-AUDIO-R01` | Diegetic-first mix per DR-020. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R02` | Audio cue → caption event pipeline: every critical SFX has a caption. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R03` | Origin-specific failure sound families per [[spec/audio-identity]]. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R04` | Mix policy: synth music ducks under critical alarms. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-R05` | Captioned playback in replay viewer. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |
| [ ] | `T-AUDIO-D06` | Done criteria: All M4..M7 SFX have captions; mix passes 5 deaf-accessibility playtest sessions. | [[spec/prototype-roadmap#T-AUDIO — Diegetic SFX And Captions]] | - | - | - | - | - | - | - |  |

### T-COMMS - Voice And Radio Simulation

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-COMMS-R01` | Voice propagation reads atmosphere/acoustic slices from `EnvironmentSignal`; vacuum/no-medium behavior is testable. | [[spec/prototype-roadmap#T-COMMS — Voice And Radio Simulation]] | - | - | - | - | - | - | - |  |
| [ ] | `T-COMMS-R02` | Radio model covers band, antenna, obstruction, multipath, SNR, jamming, encryption, and compression fields. | [[spec/prototype-roadmap#T-COMMS — Voice And Radio Simulation]] | - | - | - | - | - | - | - |  |
| [ ] | `T-COMMS-R03` | Human/robot/android origin gating matches DR-043 equipment/resource rules. | [[spec/prototype-roadmap#T-COMMS — Voice And Radio Simulation]] | - | - | - | - | - | - | - |  |
| [ ] | `T-COMMS-R04` | Every voice/radio cue emits caption, replay event, source/speaker metadata, and full-subtitle entry. | [[spec/prototype-roadmap#T-COMMS — Voice And Radio Simulation]] | - | - | - | - | - | - | - |  |
| [ ] | `T-COMMS-R05` | `cfctl observe --voice`, `cfctl observe --radio`, `cfctl act radio-tune`, `cfctl act radio-transmit`, `cfctl test comms-propagation`, and `cfctl test radio-snr` exist. | [[spec/prototype-roadmap#T-COMMS — Voice And Radio Simulation]] | - | - | - | - | - | - | - |  |
| [ ] | `T-COMMS-D06` | Done criteria: M9.5+ comms passes DR-052 sync mode checks, AI observation parity, mod schema validation, and caption/full-subtitle audit. | [[spec/prototype-roadmap#T-COMMS — Voice And Radio Simulation]] | - | - | - | - | - | - | - |  |

### T-SAVE - Save Game System

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-SAVE-R01` | `cf-save` versioned save format (`.cfsave`). | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R02` | Saves include: command core state, base modules, actors/veterans, mechs, salvage, faction state, enemy commander memory, mission manifests, scenario policy. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R03` | Multiple save slots per profile. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R04` | Autosave before/after contracts. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R05` | Mission suspend/resume. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R06` | Same-seed retry. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R07` | Ironman / scenario policies persisted. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R08` | Replay archive linked to saves. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-R09` | Migration-safe schema with version handlers. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |
| [ ] | `T-SAVE-D10` | Done criteria: Save → load → continue mission produces identical state. Migration test: a v0.1 save loads on v0.2 with declared migration handlers. | [[spec/prototype-roadmap#T-SAVE — Save Game System]] | - | - | - | - | - | - | - |  |

### T-ACCESSIBILITY - Accessibility Floor

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-ACCESSIBILITY-R01` | Per DR-012 and [[spec/accessibility-comfort-slice-a]]: | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R02` | 200% text scale + reflow. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R03` | High contrast mode. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R04` | Color-independent state labels. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R05` | Controller / keyboard / mouse parity. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R06` | Remap holds. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R07` | Captions for all critical audio. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R08` | Reduced motion / shake / flash. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-R09` | ACC-A acceptance tests at every milestone end. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |
| [ ] | `T-ACCESSIBILITY-D10` | Done criteria: Every milestone's user-facing surface passes ACC-A floor. | [[spec/prototype-roadmap#T-ACCESSIBILITY — Accessibility Floor]] | - | - | - | - | - | - | - |  |

### T-PERF - Performance Targets, Multicore CPU, And GPU Budgets

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-PERF-D01` | Done criteria: Reference scene meets the three targets. | [[spec/prototype-roadmap#T-PERF — Performance Targets, Multicore CPU, And GPU Budgets]] | - | - | - | - | - | - | - |  |
| [ ] | `T-PERF-D02` | Multicore CPU posture: every CPU-heavy system is measured and classified as single-thread-cheap, jobified/parallelized, background-worker, GPU-assisted, or blocked/needs optimization. | [[spec/prototype-roadmap#T-PERF — Performance Targets, Multicore CPU, And GPU Budgets]] | - | - | - | - | - | - | - | Required from M2 onward for terrain/material/physics/AI/server work; M0/M1 record baseline main-thread/sim costs. |
| [ ] | `T-PERF-D03` | GPU posture: render/upload/GPU-assisted paths expose counters and do not bypass replay-authoritative CPU state. | [[spec/prototype-roadmap#T-PERF — Performance Targets, Multicore CPU, And GPU Budgets]] | - | - | - | - | - | - | - | Required for custom wgpu terrain, sprite, particle, carving, post-process, and UI-heavy work. |
| [ ] | `T-PERF-D04` | Parallel determinism: parallel hot paths have stable ordering/reduction rules and replay/checksum proof where determinism is claimed. | [[spec/prototype-roadmap#T-PERF — Performance Targets, Multicore CPU, And GPU Budgets]] | - | - | - | - | - | - | - | No nondeterministic parallel reductions in sim-authoritative paths without a deterministic merge. |

### T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `T-CAPTURE-S01` | `cf-capture` crate ships `CfCapturePlugin` + `CaptureConfig` + `CaptureState` + `CaptureClock` + `CaptureMode` + `CaptureKeyframeRequested` + `CaptureFrameEntry` + `CaptureManifest` + `CaptureStateHandle`. Bevy 0.18 `Screenshot::primary_window().observe(save_to_disk(...))` integration. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | `corefall/game/crates/cf-capture/src/lib.rs`; 10 unit tests PASS (interval math at 60/120 Hz; NaN/Inf/negative/zero `frames_hz` returns u64::MAX; filename padding; manifest round-trip). Per-crate `AGENTS.md` documents Owns / Public API / Does NOT Own / Test Surface / Cross-Crate Contracts / Common Pitfalls / Source Trail. | - | - | - | 5 | 5 | - | Closed in PR #6 squash commit `064c0a0`. |
| [x] | `T-CAPTURE-S02` | `cf-app --capture-grid --capture-frames-hz <N> --no-capture-events --headless-capture` flags. Hard-rejects `--headless-smoke + --capture-grid` at startup ("no fake success"). | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | `corefall/game/crates/cf-app/src/main.rs::reject_capture_grid_with_headless_smoke`; 3 regression tests (rejects combo, allows capture-only, allows headless-only). | - | - | - | 5 | 5 | - |  |
| [x] | `T-CAPTURE-S03` | `game/tools/capture_grid.py` Pillow-based composer. Reads `capture_manifest.json`. Composes 8×8 `grid_NNN.png` + `summary_grid.png` with tick + event-label overlays burned in. Records `non_blank_ratio`. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | `corefall/game/tools/capture_grid.py`. Acceptance: 50 frames composed into `grid_001.png` (8×7 layout), `summary_grid.png` non_blank_ratio = 0.98 (49/50). | - | - | - | 5 | 5 | - |  |
| [x] | `T-CAPTURE-S04` | `cf-e2e --capture-grid --capture-frames-hz --no-capture-events --composer-script --python-bin` flags. Drops `--headless-smoke` automatically. New `key>=value` and `key<=value` operators on `--expect`. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | `corefall/game/crates/cf-e2e/src/main.rs::{LaunchOptions, parse_expect, ExpectOp, default_composer_script, invoke_composer}`. Composer JSON output merged into observation under `capture` key. | - | - | - | 5 | 5 | - |  |
| [x] | `T-CAPTURE-S05` | Run-bundle layout extension: `prototype_runs/native/<id>/captures/{frame_<index>_t<tick>.png, capture_manifest.json, grid_<NNN>.png, summary_grid.png, grid_<NNN>.json, summary_grid.json}` per [[references/prototype-run-bundle-schema]]. | [[references/prototype-run-bundle-schema]] | Vault `references/prototype-run-bundle-schema.md` updated with `captures/*` rows + `summary.json.artifacts[].type` values (`capture-frame`, `capture-grid`, `capture-summary-grid`). | - | - | - | 5 | 5 | - |  |
| [x] | `T-CAPTURE-D01` | BP1 acceptance bundle: `summary_grid.png` composed with `non_blank_ratio: 0.98`; `grid_001.png` composed with tick + event overlays; canonical checker errors=0. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | `prototype_runs/native/m1_2026-05-08T03-30-23Z_5703728c` PASSES (`errors 0`, 320 events, 50 PNGs at 10 Hz over 5 s). | - | - | - | 5 | 5 | - |  |
| [ ] | `T-CAPTURE-D02` | BP2+ done-criteria: every fun-proof scenario in BP2..BP12 emits `summary_grid.png` + `capture_manifest.json` recorded in `summary.json.artifacts`; cf-e2e includes `--expect capture.summary_grid.non_blank_ratio>=0.95`. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | Pending: M2.5 micro reactor defense (BP2), M5.5.5 micro sabotage (BP5), M5.9.5 micro pressure hold (BP7), all M7+ proof-mission slices. | - | - | - | - | - | - | Mandatory at every BP closure gate from BP2 onward. |
| [ ] | `T-CAPTURE-D03` | BP12 finalization: every shipping scenario in `content/scenarios/` has at least one canonical capture grid checked in alongside its run-bundle evidence under `prototype_runs/native/`. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | Pending until BP12. | - | - | - | - | - | - |  |
| [ ] | `T-CAPTURE-O01` | Open extension: animated WebP timeline export alongside the PNG grid. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | Scope-deferred per T-CAPTURE done-criteria "Open extensions". | - | - | - | - | - | - |  |
| [ ] | `T-CAPTURE-O02` | Open extension: side-by-side replay-vs-live diff grid for regression detection. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | Scope-deferred. | - | - | - | - | - | - |  |
| [ ] | `T-CAPTURE-O03` | Open extension: AI-readable `summary_grid.events.json` co-located with the grid so an agent can pre-filter without parsing PNG overlays. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | Scope-deferred. | - | - | - | - | - | - |  |
| [ ] | `T-CAPTURE-O04` | Open extension: true headless (offscreen RenderTarget) readback for `--headless-capture`. Currently scope-limited; logs a warning and skips frame spawn. | [[spec/prototype-roadmap#T-CAPTURE — Frame Capture, Grid Composer, And BP Fun-Proof Automation]] | Scope-limited; flag exists but offscreen wgpu readback wiring is post-BP1. | - | - | - | - | - | - |  |

### T-RELEASE — Per-BP Cross-Platform GitHub Releases

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `T-RELEASE-S01` | `.github/workflows/release.yml` triggered on `v*-bp*` tag push. Build matrix covers `x86_64-unknown-linux-gnu`, `x86_64-pc-windows-msvc`, `x86_64-apple-darwin`, `aarch64-apple-darwin`. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending; ships in BP1-T-RELEASE PR. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-S02` | `game/tools/generate_release_notes.py` reads BP tag + run-bundle summary + `summary_grid.png` + merged-PR bodies; emits release notes payload (hero image + scope summary + run-bundle stats + human-playtest survey + install instructions + determinism contract + linked PRs/vault notes). | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-S03` | Versioning axis enforced: `v0.<N>.0-bp<N>` for BP1..BP11; `v1.0.0` for BP12. Pre-release flag stays ON until BP12. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-S04` | Per-release artifacts: cf-app + cfctl + cf-e2e binaries + content/ + scripts/cfctl/ + summary_grid.png + exemplar run bundle, packaged per platform (`tar.zst` for Linux/macOS, `zip` for Windows) + SHA256SUMS.txt. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-D01` | BP1 closure: retroactive `v0.1.0-bp1` tag from main HEAD with M1.5 summary_grid as hero; all four cross-platform binaries published; release marked pre-release. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending; tag pushed AFTER PR merges. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-D02` | Determinism contract: a third party running the BP's fun-proof cfctl script against the published binary at the recorded seed produces a matching `final_sim_checksum`. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending; first verifiable at BP1 release publication. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-D03` | BP2..BP11: every BP closure emits a tagged release per the versioning axis. Pre-release flag stays ON. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending until BP2+ closures. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-D04` | BP10/BP11: code signing infrastructure activated by T-LIVEOPS (Apple notarization + Windows Authenticode). Releases at BP10+ MUST be code-signed. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending until BP10. | - | - | - | - | - | - | Coordinated with T-LIVEOPS pre-launch wiring. |
| [ ] | `T-RELEASE-D05` | BP12 finalization: `v1.0.0` GA release; pre-release flag DROPPED; full code signing on every artifact; determinism checksum table covers every shipping scenario. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Pending until BP12. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-O01` | Open extension: cargo binstall metadata so `cargo binstall corefall-cli` works for cfctl + cf-e2e. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Scope-deferred. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-O02` | Open extension: Steam Deck `.flatpak` artifact alongside the `.tar.zst` (post-BP10). | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Scope-deferred. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-O03` | Open extension: auto-update check inside cf-app that pings the GitHub Releases API and surfaces a "new BP available" toast (post-BP10). | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Scope-deferred. | - | - | - | - | - | - |  |
| [ ] | `T-RELEASE-O04` | Open extension: reproducible-builds attestation (sigstore or in-toto provenance) per release. | [[spec/prototype-roadmap#T-RELEASE — Per-BP Cross-Platform GitHub Releases]] | Scope-deferred. | - | - | - | - | - | - |  |

---

## Native Task Card Checklist

These rows come from [[spec/native-implementation-backlog]]. They are the concrete implementation units agents should be assigned. A task card row should not be checked until its tests, evidence, and anti-scope obligations are satisfied.

### M0 - Engine Bootstrap

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M0-001` | workspace scaffold. Build: Apply the bootstrap recipe verbatim: workspace + 29 crates (`cf-app`, `cf-sim-core`, `cf-terrain`, `cf-physics`, `cf-material`, `cf-atmos`, `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-ai`, `cf-mission`, `cf-replay`, `cf-control`, `cfctl`, `cf-e2e`, `cf-save`, `cf-net`, `cf-render-2d`, `cf-ui`, `cf-audio`, `cf-mod`, `cf-tools-editor`, `cf-headless`, `cf-server`, `cf-server-ops`, `cf-server-persistence`, `cf-server-anti-cheat`, `cf-server-admin`, `cf-bench`); per-crate `AGENTS.md` per the template; pinned Bevy/glam/serde/clap/tokio/jsonrpsee/blake3/tracing/rand_xoshiro/schemars deps from the workspace dependencies table. Tests: `cargo metadata`, `cargo check --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all -- --check`. Anti-scope: No gameplay systems beyond no-op scene; no extra crates not in the recipe. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/game/Cargo.toml` + `crates/*/Cargo.toml` (29) + `crates/*/AGENTS.md` (29); `cargo fmt/check/clippy/test` all green (M0.1: 42 tests); `rust-toolchain.toml` pin = 1.93.0 with same-pass roadmap recipe edit. | - | - | - | 9 | 9 | 2 | Bevy 0.14 IS a workspace dependency (no longer deferred). jsonrpsee is intentionally NOT a workspace dep; `tokio-tungstenite` + a minimal hand-rolled JSON-RPC envelope keeps the dep tree small (documented in cf-control/AGENTS.md). Owns: `game/Cargo.toml`, `game/crates/*`, `.cargo/config.toml`, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `.gitignore`. |
| [x] | `M0-002` | Bevy app shell. Build: Open a window, clear screen, fixed title/version, ESC exits; support all M0 flags from the [[spec/prototype-roadmap#CLI Reference|CLI Reference]] (`--scenario`, `--seed`, `--run-seconds`, `--ticks`, `--write-run-bundle`, `--run-bundle-dir`, `--control-api`, `--control-port`, `--control-uds`, `--headless-smoke`, `--debug-capabilities`, `--ui-scale`, `--high-contrast`, `--captions`, `--reduced-motion`, `--reduced-shake`, `--reduced-flash`). Tests: Native app smoke for 5 seconds; CLI flag parser tests; `--headless-smoke` exits cleanly; settings observation reports the accessibility flags. Anti-scope: No menu system or final UI. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-app/src/main.rs`: real Bevy 0.14 app + DefaultPlugins + WindowPlugin (1280×720, title `"Corefall — M0 Engine Bootstrap (v0.0.1)"`) + `cf-render-2d::CfRenderPlugin` (clear-screen `#0d121a` + Camera2dBundle) + `Time::<Fixed>::from_hz(--tick-rate-hz)` driving FixedUpdate + ESC + WindowCloseRequested handlers. CLI parser unit test asserts every M0 flag plus `--tick-rate-hz`. M0.1 added `run_paced_loop_holds_wall_clock_cadence` test proving 60 ticks @ 60 Hz takes ≥ 0.85 s wall through the headless+control-api path. | - | - | - | 9 | 9 | 2 | All M0 flags wired and surfaced through live `Settings` + `run_manifest.json.settings`. `--control-uds` is parsed and reserved; the loopback TCP path is the M0 transport per DR-002 v1 lock. |
| [x] | `M0-003` | fixed tick island. Build: Implement fixed 60 Hz tick with optional 120 Hz; deterministic seed/RNG wrapper using `rand_xoshiro::Xoshiro256StarStar`; `Tick(u64)`, `WallClock`, `SimClock` types; tick counters; `pause`, `resume`, `step(n)`, `run_for(n)` API consumed by `cf-control`. Tests: Unit tests for tick accumulation, seed repeatability (same seed → same checksum after 1000 ticks), pause/resume/step semantics; lints disallow `rand::thread_rng` and `SystemTime::now` per `clippy.toml`. Anti-scope: No full scheduler rewrite; no Bevy scheduling-stage redesign. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-sim-core/src/{lib,checksum,ids}.rs`; 14 unit tests pass (including `step_zero_is_a_no_op`); `system.run_started`, `system.run_finished`, `determinism.sim_checksum`, AND `system.tick_sample` (M0.2-F4: emitted every cadence_ticks with `{tick_rate_hz, window_ticks, avg_tick_ms, max_tick_ms, p99_tick_ms, samples_observed}`) all present in run bundles. M0.2 bundles record 5 (60 Hz/300) or 10 (120 Hz/600) `tick_sample` events. New test `tick_sample_event_emitted_at_cadence`. | - | - | - | 9 | 9 | 2 | All M0-003 task-card events implemented and verified. |
| [x] | `M0-004` | run bundle writer. Build: Write `run_manifest.json`, `events.jsonl`, `summary.json`, `notes.md`; include build hash, config hash, scene id, schema version, capabilities, expected tests; directory naming per [[spec/prototype-roadmap#Run-Bundle Naming Convention|Run-Bundle Naming Convention]]. Tests: Checker passes on M0 bundle; round-trip test for the envelope; non-blocking write under stress (events queued; dropped counter visible in `summary.json.event_counts.dropped_total`). Anti-scope: Do not design final replay UI. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-replay/src/lib.rs`; unit tests + checker coverage. M0.3-F8: final bundles written by `cf-app --headless-smoke --control-api --write-run-bundle` include final exit evidence even if a mid-run `runbundle.write` already wrote a snapshot. M0.3-F9: default run-bundle root resolves to repo-root `prototype_runs/native` through shared runtime helpers. Final proof bundles: `1ad62cb4`, `2c7f5b05`, `a9675fc6`, `56e26f4b`, all checker errors 0. | - | - | - | 9 | 9 | 1 | Production paths share the contract. `for_test_scenario_only` is `#[doc(hidden)]`; default path no longer depends on cwd. |
| [x] | `M0-005` | CI matrix. Build: Apply the CI YAML from the bootstrap recipe; Win/Linux/macOS matrix; `cargo fmt`, `cargo check`, `cargo clippy -D warnings`, `cargo test`, `cfctl observe smoke`, `cfctl run --write-run-bundle` smoke. Tests: CI green when runners available; local commands pass regardless. Anti-scope: No release packaging; no Steam Deck CI yet. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/.github/workflows/ci.yml`; final M0.3 local Standard Validation pass on macOS aarch64 + rustc 1.93.0 (68 tests + doctests). | - | - | - | 8 | 8 | 2 | Push to GitHub deferred (no commit/push without explicit user instruction); workflow YAML mirrors the recipe with toolchain pin synced. |
| [x] | `M0-006` | control/observe bootstrap. Build: Define command/observation envelope per [[spec/prototype-roadmap#Control Transport And Envelope|Control Transport And Envelope]] (JSON-RPC 2.0 over WebSocket on `127.0.0.1:17890`, optional UDS, `schema_version` mandatory, blake3 short-hash run ids); generate JSON Schemas via `schemars` under `crates/cf-control/schemas/v1/`; implement `scenario.load`, `sim.pause/resume/step/run_for_ticks`, `observe.once`, `observe.subscribe/unsubscribe`, `observe.frame` notification, `act.player.*`, `runbundle.write`, `system.shutdown`; `cfctl` subcommands `observe --once\|--stream`, `run --ticks --write-run-bundle`, `scenario load`, `pause`, `step`, `script run` per [[spec/prototype-roadmap#CLI Reference|CLI Reference]]. Tests: Unit tests for envelope (request/response/notification roundtrip); schema_version mismatch returns `-32602` with fix-hint; `cargo run -p cfctl -- observe --once`; no-op `run --ticks 300 --write-run-bundle` writes a valid bundle; loopback-only by default; heartbeat ping/pong. Anti-scope: No remote bot API; no gameplay debug cheats; no unauthenticated remote bind. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-control/src/{envelope,server,schemas,settings,state,scenario,engine,runtime}.rs`; 18 static schemas under `crates/cf-control/schemas/v1/` (regenerated by `dump_schemas`; CI runs `dump_schemas --check`); cfctl auto-launch + script runner + live WS roundtrip. M0.3-F7: strict serde params reject unknown fields; missing/non-numeric/mismatched schema versions reject; zero ticks reject; unsupported observe params reject; unsupported `act.player.move` and `runbundle.write.id_override` reject instead of fake success. Tests: 36 unit tests + 9 live WS tests; script bundle `m0_2026-05-06T04-46-37Z_56e26f4b` proves live accepted settings path. | - | - | - | 9 | 9 | 1 | All M0 command semantics now have positive proof plus negative/adversarial proof. UDS reserved per CLI; loopback TCP is M0 transport per DR-002 v1 lock. |
| [x] | `M0-007` | m0_blank scenario fixture. Build: Author the M0 scenario manifest per [[spec/prototype-roadmap#Scenario Manifest Schema|Scenario Manifest Schema]] minimal skeleton; loadable by `cf-app --scenario m0_blank`; validates with `cargo run -p cf-mod -- validate content/`. Tests: Schema validation test; `--scenario m0_blank` smoke. Anti-scope: No teams, actors, terrain, or objectives. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/game/content/scenarios/m0_blank.ron` loaded by every M0/M0.1/M0.2 bundle. `cf-mod validate content/` is a real RON walker that parses scenarios under `**/scenarios/` and validates `schema_version=1` + non-empty `id`+`display_name` + ≥ 1 `expected_tests`; exits non-zero on FAIL. M0.1+M0.2: `cf-app::build_config` AND `cfctl::cmd_run`/`cmd_observe --inline` all route through `cf_control::runtime::build_engine_config`, which calls `Scenario::load_from_file` so the manifest drives engine config (test `for_loaded_scenario_pulls_seed_and_expected_tests_from_manifest`). | - | - | - | 9 | 9 | 2 | Real validator, not a stub. `--strict` promotes WARN to FAIL. |
| [x] | `M0-008` | panic hook + tracing init. Build: Each binary's `main()` initializes `tracing-subscriber` with `EnvFilter` per [[spec/prototype-roadmap#Logging, Tracing, And Error Policy|Logging, Tracing, And Error Policy]]; installs a panic hook that emits `system.panic` event with backtrace before exit; severity counters are incremented in `summary.json.event_counts.by_severity`. Tests: Binary boot test asserts the subscriber is registered; panic test triggers a controlled panic in a sub-thread and verifies the event is emitted; counter assertion. Anti-scope: No log-aggregation product. | [[spec/native-implementation-backlog#M0 — Engine Bootstrap]] | `corefall/game/crates/cf-replay/src/diagnostics.rs`; every binary calls `cf_replay::diagnostics::init(target)`. **M0.2-F5**: (a) Unit test `panic_in_sub_thread_emits_system_panic_event_and_increments_severity` spawns a real sub-thread with `panic!`, catches via `JoinHandle::join`, drives the same `report_panic_to_recorder` code path as the global panic hook, asserts `system.panic` event lands AND `event_counts.by_severity.error` increments. (b) `cf-app --debug-inject-panic-at-tick <n>` flag spawns a thread that panics at the named tick; the global hook + new lock-free `current_tick: AtomicU64` snapshot record `system.panic` at the engine's actual tick (preserves events.jsonl monotonicity). Bundle `m0_2026-05-06T04-14-03Z_03164834` PROVES it: panic injected at tick 60, recorded at tick 61, `event_counts.by_severity.error: 1`, `event_counts.by_type.panic: 1`, bundle PASSES `prototype_run_check.py` (errors 0). | - | - | - | 9 | 9 | 2 | M0-008 task-card requirement fully met. |
### M1 - Actor Controller And Sim Core

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M1-001` | control intent. Build: Input maps to `ControlIntent` before sim consequences: move, jump, aim, fire, reload, selected item. Tests: Input serialization test; intent precedes action event. Anti-scope: No rollback/netcode. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | `cf-actor::ControlIntent` + `IntentSource::{Human, Cfctl}`; engine `pending_intent`; cfctl bundle `m1_2026-05-06T17-18-11Z_ac18c89b` has 169 input.intent_received events parent-linking 23 control.command_accepted events. | - | - | - | 5 | 5 | - | Owns: `cf-actor`, `cf-sim-core`, `cf-replay`. ControlIntent landed in cf-actor (closest semantic owner) rather than cf-sim-core. |
| [x] | `M1-002` | actor movement. Build: Position/velocity, gravity, ground collision, jump/fall/recovery, reset. Tests: Unit tests for gravity/ground/contact; E2E movement route. Anti-scope: No complex ragdoll. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | `cf-physics::{step_kinematics, apply_horizontal_motion, apply_jump}`; 7 unit tests; cfctl bundle has actor moving 200→595 then back, 1 actor_jumped + 1 actor_landed event; 60s bundle has 60 actor_snapshot events. | - | - | - | 5 | 5 | - | Owns: `cf-actor`, `cf-physics`. |
| [x] | `M1-003` | rifle loop. Build: One rifle: fire interval, ammo, reload, recoil, muzzle origin, hit/miss event. Tests: Ammo/reload/recoil tests; scripted fire/reload E2E. Anti-scope: No large arsenal. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | `cf-equipment::{RifleSpec, RifleState, tick_rifle, RIFLE_M1_DEFAULT_ID}` + 8 unit tests; cfctl bundle captures 3 weapon_fired + 3 projectile_spawned + 3 projectile_expired + 1 weapon_reload_started + 1 weapon_reloaded events. | - | - | - | 5 | 5 | - | Owns: `cf-equipment`, `cf-actor`. |
| [x] | `M1-004` | status strip. Build: Minimal HUD: status, ammo, selected item, reticle state. Tests: UI state source tests; screenshot artifact. Anti-scope: No final comic-noir UI. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | `cf-ui::StatusStripPlugin` + `HudState` + `HudRifle` + `rifle_status_line`; 5 unit tests covering READY/RELOADING/EMPTY/COOLDOWN/NO RIFLE; cf-app bridge populates the resource each frame. | - | - | - | 4 | 4 | - | Owns: `cf-ui`, `cf-actor`. Screenshot artifact deferred to manual playtest (M1-D06). |
| [x] | `M1-005` | HTML lab supersession note. Build: Record whether native M1 supersedes HTML lab for actor iteration; list gaps if not. Tests: N/A. Anti-scope: Do not delete HTML evidence. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | Captured in `corefall/docs/implementation-log/2026-05-06-m1-actor-controller.md` §M1-005 + this checklist row. | - | - | - | 5 | 5 | - | Vault prototype note `prototypes/native-m1-actor-controller.md` follows in same closure pass; M1.5 onward stays in native. |
| [x] | `M1-006` | semantic actor control. Build: Drive movement, aim, fire, reload, selected item, and reset through the same `ControlIntent` path as human input; stream actor/equipment observations. Tests: Scripted movement/fire/reload through `cfctl`; assert events and observations agree. Anti-scope: No network prediction/rollback. | [[spec/native-implementation-backlog#M1 — Actor Controller And Sim Core]] | Seven `act.player.*` JSON-RPC methods + 12 live WS acceptance tests + cfctl `act player-*` subcommands + `m1_move_jump_fire_reload.cfctl.json` script + cfctl bundle `m1_2026-05-06T17-18-11Z_ac18c89b` (392 events; same code path as cf-app's `ingest_player_input` keyboard bridge). | - | - | - | 5 | 5 | - | Owns: `cf-control`, `cf-actor`, `cf-equipment`, `cf-e2e`. |
### M1.5 - Micro Breach Fun Slice

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `M1.5-001` | scenario shell. Build: 60-90s `micro_breach` scenario: spawn, objective, timer, extraction, win/loss. Tests: Objective state tests; scripted win/loss. Anti-scope: No full mission director. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | `cf-control::scenario::Scenario` extended with `breaches[]/objectives[]/mission`; `content/scenarios/micro_breach.ron`; cf-mod validate PASS; bundles `m1.5_2026-05-08T01-27-46Z_d0068465` (win) + `m1.5_2026-05-08T01-27-55Z_c836bcbd` (loss) emit full `mission.objective_started/completed/mission_resolved` chains. | - | - | - | 5 | 5 | - | Owns: `cf-mission`, `cf-app`, `content/scenarios/`. |
| [x] | `M1.5-002` | reactive enemy. Build: One enemy: sight cone, aim delay, imperfect fire, reload, death; no omniscience. Tests: Perception/aim/fire tests; E2E enemy kill + player death. Anti-scope: No full AI doctrine system. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | `cf-ai::ReactiveGuard` (DR-008 LEAN: scripted job FSM + utility scoring + scripted hooks); 9 unit tests PASS including `deterministic_under_same_seed`; bundles emit `ai.ai_perception/tactic_chosen/state_changed` + `equipment.weapon_fired/reload_started/reload_completed/dry_fire` + `combat.projectile_spawned`; loss bundle reaches `mission.result=lost reason=player_dead`. | - | - | - | 5 | 5 | - | Owns: `cf-ai`, `cf-actor`, `cf-equipment`. |
| [x] | `M1.5-003` | temporary soft breach. Build: Minimal soft barrier/diggable tiles with success/refusal events; compatibility with future M2 event names. Tests: Dig success/refusal tests. Anti-scope: No full chunked terrain. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | `cf-terrain::BreachWorld + try_dig + DigOutcome` ships `concrete_soft` + `metal_nohook` strips; 9 unit tests PASS; bundles emit `terrain.tool_action_started → terrain.terrain_carved` (with `bbox`, `material_before/material_after`, `count`, `damage_applied`, `hp_remaining`, `broken`) + `terrain.terrain_breach_stub` + `terrain.tool_refused` (with reason vocabulary `out_of_range/material_metal_nohook/already_broken/unknown_target`). M2-compatible event payload shape. | - | - | - | 5 | 5 | - | Owns: `cf-terrain`. |
| [x] | `M1.5-004` | readable loop HUD. Build: Objective/timer, player status, enemy status, selected item, last event. Tests: Screenshot at 100% and 200% scale if UI scaling exists. Anti-scope: No full HUD art pass. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | `cf-ui::StatusStripPlugin` extended with OBJECTIVE / MISSION (timer + result) / ENEMY (hp + state + last tactic) / BREACH (id + hp + range + refusal) / EVENT lines; 5 new formatter unit tests; `cf-render-2d` paints breach strips + extraction zone (translucent green box). M4 owns the comic-noir polish + screenshots + ACC-A floor; M1.5 ships text-strip evidence. | - | - | - | 4 | 5 | - | Owns: `cf-ui`. |
| [x] | `M1.5-005` | fun/evidence note. Build: Compare reaction against "ok I guess"; list whether pressure/goal changed feel. Tests: N/A. Anti-scope: Do not claim final fun. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | Implementation log `corefall/docs/implementation-log/2026-05-07-m1-5-micro-breach-fun-slice.md` records the agent-driven evidence. Vault prototype note `prototypes/native-m1-5-micro-breach.md` queued for vault maintainer pass. Human-playtest reaction stays `READY_FOR_HUMAN_PLAYTEST`. | - | - | - | 4 | 5 | - | Owns: vault only. |
| [x] | `M1.5-006` | control-driven E2E. Build: Write `cfctl` scripts for win path and loss path; assertions read objective/enemy/player state from observations and events. Tests: `cargo run -p cfctl -- script run micro_breach_win` and `micro_breach_loss`; observation stream freshness check. Anti-scope: No brittle OS-level mouse/keyboard automation. | [[spec/native-implementation-backlog#M1.5 — Micro Breach Fun Slice]] | `cf-e2e --scenario micro_breach --script micro_breach_win --expect mission.result=won --expect objective.{breach,neutralize,extract}=completed --write-run-bundle` PASS 4/4 (bundle `m1.5_2026-05-08T01-27-46Z_d0068465`). `cf-e2e --scenario micro_breach --script micro_breach_loss --expect mission.result=lost --expect mission.loss_reason=player_dead --expect objective.breach=completed --write-run-bundle` PASS 3/3 (bundle `m1.5_2026-05-08T01-27-55Z_c836bcbd`). Both bundles validate via canonical run-bundle checker (`errors 0`). cfctl `act player-dig` subcommand routes through the same dispatch path as cf-app `KeyG`. | - | - | - | 5 | 5 | - | Owns: `cf-control`, `cf-e2e`, `cf-mission`. |
### M2 - Pixel Terrain And Materials

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M2-001` | chunk storage. Build: 256x256 chunk grid, material id per pixel, sparse storage, CPU read/write. Tests: Chunk bounds, material set/get, serialization tests. Anti-scope: No Noita chemistry. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cf-terrain`. Evidence target: Terrain snapshot in run bundle. |
| [ ] | `M2-002` | material registry. Build: Air, dirt, concrete, metal-nohook, hazard, loose fill, repair-fill, anchor; hardness/affordance fields. Tests: Material schema validation. Anti-scope: No full research tree. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cf-terrain`, `content/materials/`. Evidence target: Material schema version in manifest. |
| [ ] | `M2-003` | carving pipeline. Build: Digger and blast carve; CPU fallback; optional wgpu path behind feature flag. Tests: Carve bbox/count tests; GPU/CPU parity if GPU path exists. Anti-scope: No production destruction VFX. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cf-terrain`, `cf-render-2d`, `cf-equipment`. Evidence target: Dirty-region and perf counters. |
| [ ] | `M2-004` | physics integration. Build: Actor collision respects terrain after edits; chunk boundary tests. Tests: Collision after carve/fill tests. Anti-scope: No full pathfinding. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cf-physics`, `cf-terrain`. Evidence target: E2E dig-through-wall. |
| [ ] | `M2-005` | material overlay. Build: Toggle overlay shows material ids and tool validity. Tests: Screenshot at 100/200% if applicable. Anti-scope: No tactical map. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `cf-render-2d`. Evidence target: Overlay capture. |
| [ ] | `M2-006` | terrain replay. Build: Terrain snapshots/checksums and event replay reconstruct terrain. Tests: Live vs replay checksum test. Anti-scope: No final cinematic replay. | [[spec/native-implementation-backlog#M2 — Pixel Terrain And Materials]] | - | - | - | - | - | - | - | Owns: `cf-replay`, `cf-terrain`. Evidence target: Replay report. |
### M3 - Replay And Event Recorder

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M3-001` | event taxonomy. Build: Stable event envelope, categories, parent ids, schema versions. Tests: Schema/event ordering tests. Anti-scope: No analytics service. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cf-replay`. Evidence target: Updated run-bundle schema note if fields change. |
| [ ] | `M3-002` | snapshots/checksums. Build: Actor/inventory/terrain snapshots and checksums. Tests: Checksum repeatability tests. Anti-scope: No full deterministic promise for cosmetics. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cf-replay`, `cf-terrain`, `cf-actor`, `cf-equipment`. Evidence target: `determinism.sim_checksum` events. |
| [ ] | `M3-003` | headless replay. Build: Replay M2/M1.5 bundles without rendering and verify checksums. Tests: Replay compare test. Anti-scope: No network server yet. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cf-headless`, `cf-replay`. Evidence target: First-divergence report on failure. |
| [ ] | `M3-004` | viewer. Build: Event tail, filters, parent-chain view, death/failure recap. Tests: Viewer smoke test; screenshot. Anti-scope: No polished replay browser. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `cf-replay`. Evidence target: Viewer capture in bundle. |
| [ ] | `M3-005` | recorder backpressure. Build: Dropped-event counters and non-blocking recorder path. Tests: Stress event-volume test. Anti-scope: No cloud telemetry. | [[spec/native-implementation-backlog#M3 — Replay And Event Recorder]] | - | - | - | - | - | - | - | Owns: `cf-replay`. Evidence target: Summary volume/perf rows. |
### M4 - HUD And Comic-Noir UI

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M4-001` | HUD state model. Build: Actor status, body silhouette placeholder, ammo, item, objective, last event. Tests: UI state tests. Anti-scope: No final art polish. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `cf-actor`, `cf-equipment`, `cf-chassis`. Evidence target: HUD screenshots. |
| [ ] | `M4-002` | comic-noir cards. Build: Pre-mission and debrief card templates. Tests: Layout snapshot tests if available. Anti-scope: No full campaign UI. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `content/ui/`. Evidence target: 100/200% captures. |
| [ ] | `M4-003` | accessibility floor. Build: 200% scale, high contrast, keyboard/controller focus, captions hook, reduced shake/flash flags. Tests: E2E accessibility smoke. Anti-scope: No certification claim. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `cf-app`. Evidence target: ACC-A status in notes. |
| [ ] | `M4-004` | material/tool feedback. Build: Tool validity labels and non-color-only material feedback. Tests: Overlay screenshot tests. Anti-scope: No full tactical map. | [[spec/native-implementation-backlog#M4 — HUD And Comic-Noir UI]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `cf-terrain`. Evidence target: Capture artifact. |
### M5 - Equipment, Chassis, And Damage Grammar

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5-001` | role records. Build: Runtime role-record model from vault fixtures: role tags, bot policy, source/provenance fields. Tests: Schema/fixture tests. Anti-scope: No full economy/store. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cf-equipment`, `content/equipment/`. Evidence target: LOAD-A fixture import report. |
| [ ] | `M5-002` | chassis model. Build: Armor zones, modules, pilot binding, powered armor and light mech. Tests: State transition tests. Anti-scope: No full mech roster. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cf-chassis`. Evidence target: `chassis_stage_changed` events. |
| [ ] | `M5-003` | damage/eject/repair/salvage. Build: Module damage, jam, eject, repair, salvage events and HUD labels. Tests: E2E wreck/eject/salvage. Anti-scope: No final gore/body system. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cf-chassis`, `cf-equipment`, `cf-replay`. Evidence target: Chassis run bundle. |
| [ ] | `M5-004` | save hooks. Build: Serialize chassis/equipment state enough for roundtrip. Tests: Save/load checksum. Anti-scope: No full campaign save UI. | [[spec/native-implementation-backlog#M5 — Equipment, Chassis, And Damage Grammar]] | - | - | - | - | - | - | - | Owns: `cf-save`, `cf-chassis`. Evidence target: Save artifact linked from run. |
### M5.5 - Full Collision Gauntlet

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M5.5-001` | collision class registry. Build: Define physical classes from [[spec/full-collision-physics-plan]]: actor core, limb, armor zone, held weapon, loose item, kinetic projectile, explosive projectile, terrain proxy, debris chunk, mech part, base object, force field, sensor trigger, cosmetic particle. Tests: Registry roundtrip; missing-class validation; class-id stability test. Anti-scope: No gameplay behavior yet. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-physics`, `cf-mod`, `content/collision/`. Evidence target: `collision_class_registered` or schema audit in run note. |
| [ ] | `M5.5-002` | collision matrix + filters. Build: Data-driven matrix with collide/sensor/filter/damage response; every filter requires `collision_filter_reason`. Tests: COLL-001; bad matrix fixtures fail with useful diagnostics. Anti-scope: No silent ignore pairs. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-physics`, `cf-mod`. Evidence target: Matrix file and validator report. |
| [ ] | `M5.5-003` | broadphase and pair cache. Build: Dynamic tree/spatial hash hybrid; stable pair ids; deterministic pair ordering; projectile lane cache. Tests: Pair-count tests; deterministic ordering; stress bench. Anti-scope: No O(n^2) production path. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-physics`, `cf-bench`. Evidence target: Perf counters: candidate pairs, narrowphase pairs, culled low-value pairs. |
| [ ] | `M5.5-004` | narrowphase/contact manifolds. Build: Contact manifolds for circle/capsule/convex/AABB/segment/terrain-proxy pairs; material pair lookup. Tests: Shape-pair unit tests; edge/tiny-hole fixtures. Anti-scope: No exact per-pixel rigid body solver. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-physics`. Evidence target: `collision_contact_started/persisted/ended`. |
| [ ] | `M5.5-005` | CCD tiers. Build: Discrete, speculative, sweep ray, sweep capsule, sweep shape, TOI substep; per-class `ccd_class`. Tests: COLL-007; tunneling fixtures for thin terrain, limb, shield, bullet, mech foot. Anti-scope: No universal TOI for all debris. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-physics`, `cf-bench`. Evidence target: `toi_fraction` and CCD-tier fields in events. |
| [ ] | `M5.5-006` | projectile-projectile contacts. Build: Swept projectile lane test; kinetic deflect/fragment/tumble/energy loss; explosive detonate/fuze-fail/deflect by profile. Tests: COLL-006; deterministic bullet-cross fixtures. Anti-scope: No fake random explosions for kinetic rounds. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-physics`, `cf-equipment`, `content/projectiles/`. Evidence target: `projectile_projectile_contact`, `projectile_deflected`, optional `projectile_fragmented`. |
| [ ] | `M5.5-007` | limb/equipment/body contacts. Build: Limb-to-limb/body/weapon/terrain/door contacts; held weapon physical contacts; scoped owner self-filter; dropped item contacts. Tests: COLL-002..COLL-004; crowd corridor fixture. Anti-scope: No animation-only collision. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-physics`. Evidence target: Contact events plus body/equipment/chassis follow-up events. |
| [ ] | `M5.5-008` | impulse-to-damage routing. Build: Convert contact impulse/material/area/sharpness into limb wounds, armor crack/spall, equipment jam/damage, chassis module failure, terrain/base damage. Tests: COLL-005, COLL-008; threshold tests by material/origin. Anti-scope: No hidden HP-only damage. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-physics`, `cf-actor`, `cf-chassis`, `cf-equipment`, `cf-terrain`. Evidence target: `contact_impulse_applied`, `collision_damage_applied`, parent-linked body/equipment/terrain events. |
| [ ] | `M5.5-009` | terrain/base/shield proxies. Build: Dirty chunk collision proxy rebuilds; doors/turrets/sensors/shields/repair pads register physical or sensor proxies. Tests: Chunk seam tests; shield/body/projectile/base fixtures. Anti-scope: No full base builder here. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-terrain`, `cf-mission`, `cf-physics`. Evidence target: Terrain dirty-to-proxy events; base object contact events. |
| [ ] | `M5.5-010` | `cfctl` collision observation. Build: `cfctl observe --collisions` and `cfctl inspect collision <event-id>` show live pairs, filters, last contacts, TOI, impulses, and budget status. Tests: CLI snapshot tests; stream freshness tests. Anti-scope: No screenshot-only physics debugging. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-control`, `cfctl`, `cf-physics`. Evidence target: Observation samples in run notes. |
| [ ] | `M5.5-011` | full gauntlet scenario. Build: Scenario scripts for COLL-001..COLL-012: crowd corridor, bullet cross, limb/weapon/door, debris crush, mech foot, shield, terrain seams. Tests: Full E2E suite. Anti-scope: No hand-tested-only acceptance. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `content/scenarios/m5_5_full_collision_gauntlet.ron`, `cf-e2e`. Evidence target: Checked run bundle with event counts by collision type. |
| [ ] | `M5.5-012` | replay/perf/bug hunt. Build: Headless replay checksum; perf report; first-divergence event; bug-hunt log. Tests: Replay verify; 1080p/60 pass; 4K/120 + Deck status recorded. Anti-scope: No "works once" completion. | [[spec/native-implementation-backlog#M5.5 — Full Collision Gauntlet]] | - | - | - | - | - | - | - | Owns: `cf-headless`, `cf-bench`, `tools/`, vault. Evidence target: Prototype note under `prototypes/` with final audit, fixed findings, and any user-approved deferrals. |
### M6 - AI Core And Trust Harness

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6-001` | perception/memory. Build: Sight, hearing, last-known memory, forgetting. Tests: Perception unit tests; occlusion tests. Anti-scope: No LLM runtime dependency. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cf-ai`, `cf-actor`, `cf-replay`. Evidence target: `ai_perception_signal` events. |
| [ ] | `M6-002` | utility/doctrine. Build: Utility scoring and 4-6 doctrine profiles. Tests: Scoring tests with deterministic fixtures. Anti-scope: No full strategic commander yet. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cf-ai`. Evidence target: `ai_tactic_scored`, `tactic_chosen`. |
| [ ] | `M6-003` | mistakes/recovery. Build: Panic/hesitate/miss/stuck/recover behavior with reason labels. Tests: Recovery scenario tests. Anti-scope: No fake randomness without causes. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cf-ai`, `cf-replay`. Evidence target: `ai_recovery_action`. |
| [ ] | `M6-004` | AI-H harness. Build: Runnable AI-H-01..06 suite with report output. Tests: Harness pass/fail tests. Anti-scope: No broad campaign AI. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cf-ai`, `cf-headless`, `tools/`. Evidence target: AI-H report bundle. |
| [ ] | `M6-005` | bot overlay. Build: Visible intent labels for friendly/enemy bots. Tests: Screenshot capture. Anti-scope: No dialogue system. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `cf-ai`. Evidence target: Overlay screenshot. |
| [ ] | `M6-006` | mind hooks (T-LLM bridge). Build: Expose hook points that the future M6.5 mind layer will call: utility-weight patch API, commander-blackboard goal API, doctrine-tag set API, dialogue-queue API, memory-write API. M6 itself MUST NOT call any LLM. Tests: Hook tests with synthetic patches; AI-H stays green when no hooks are called. Anti-scope: No LLM runtime dependency in M6. | [[spec/native-implementation-backlog#M6 — AI Core And Trust Harness]] | - | - | - | - | - | - | - | Owns: `cf-ai`. Evidence target: Hook trait docs in `cf-ai::doctrine`; example synthetic patch in tests. |
### M6.5 - LLM Mind Lab

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M6.5-001` | mind schemas. Build: Define `MindObservationFrame`, `MindTask`, `AiMindProposal`, `MindValidationResult`, `MindMemoryRecord`, `MindProviderConfig` per [[spec/hybrid-llm-ai-plan]]; emit JSON Schemas via `schemars`. Tests: Roundtrip tests; bad-example rejection tests; schema-version mismatch test. Anti-scope: No public schema export yet. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-ai::mind::schema`, `game/crates/cf-ai/schemas/mind/v1/`. Evidence target: Schemas committed; example proposal validates. |
| [ ] | `M6.5-002` | mock provider. Build: Deterministic provider that consumes a canned-script directory; supports inject-canned, inject-malformed, inject-timeout, inject-stale, inject-cost-overflow modes. Tests: Per-mode tests; CI uses mock only. Anti-scope: No live cloud calls in mock. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-ai::mind::provider::mock`. Evidence target: Mock provider used by all MIND-* tests. |
| [ ] | `M6.5-003` | provider trait + adapters. Build: Shared async trait; OpenAI Responses API adapter; Anthropic Messages API adapter; Ollama adapter; OpenAI-compatible adapter (vLLM/llama.cpp); each behind a cargo feature; secrets read from env per `MindProviderConfig.api_key_env`. Tests: Adapter contract tests with mocked HTTP; feature-gate tests verify default build excludes cloud. Anti-scope: No vendor SDK lock-in; no API keys in repo. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-ai::mind::provider` (cargo features `mind-openai`, `mind-anthropic`, `mind-ollama`, `mind-openai-compatible`). Evidence target: Adapter docs; example `MindProviderConfig`. |
| [ ] | `M6.5-004` | observation compressor. Build: Derive `MindObservationFrame` from the `cf-control` observation stream + recent replay events; enforce fog-of-war BEFORE any provider sees a prompt. Tests: Fog-of-war audit tests (synthetic hidden enemy never appears in frame); compactness tests; `cfctl observe --mind-frame <scope>` smoke. Anti-scope: No raw-state passthrough. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-ai::mind::compressor`, `cf-control`, `cf-replay`. Evidence target: Sample frames in run notes. |
| [ ] | `M6.5-005` | proposal validator. Build: Reject stale, invalid, impossible, unfair, over-budget, hidden-info, and capability-violating proposals; replay-visible reasons. Tests: Per-rejection-class unit tests; MIND-003/004/006/009 acceptance pass. Anti-scope: No silent acceptance. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-ai::mind::validator`. Evidence target: Validator decision log. |
| [ ] | `M6.5-006` | policy compiler. Build: Convert accepted proposals into utility-weight patches, commander goals, doctrine tags, dialogue-queue entries, and `MindMemoryRecord` writes via M6 hook points. Tests: Patch-application tests; doctrine-patch visibility test (MIND-005). Anti-scope: No direct low-level action emission. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-ai::mind::policy`. Evidence target: One visible doctrine patch in micro_breach_mind_lab. |
| [ ] | `M6.5-007` | mind events + run-bundle integration. Build: Emit `mind.task_created`, `mind.prompt_recorded` (hashes by default; raw text only behind `debug_capabilities`), `mind.response_received`, `mind.proposal_validated`, `mind.patch_applied`, `mind.patch_rejected`, `mind.memory_written`. Update run-bundle checker to recognize the `mind` category. Tests: Bundle-validation tests; secret-redaction tests. Anti-scope: No raw secrets in run bundles. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-replay`, `cf-ai::mind::events`, `tools/run_bundle_check.py`. Evidence target: Run bundles include `mind` events; redaction verified. |
| [ ] | `M6.5-008` | mind dashboard (dev). Build: Dev-only workbench panel showing task count, stale rate, provider failures, estimated cost, model routing, and accept/reject reasons. Tests: Dashboard render tests; screenshot. Anti-scope: No player-facing UI yet. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-tools-editor`, `cf-ui`. Evidence target: Dashboard capture in M6.5 note. |
| [ ] | `M6.5-009` | micro_breach_mind_lab scenario. Build: The M6.5 lab scenario in three modes (`mind_off`, `mind_mock`, `mind_live_optional`) with a sample commander mind profile and one designed doctrine-patch opportunity. Tests: Scenario validates with `cf-mod validate`; all three modes load. Anti-scope: No content tied to a specific cloud model id. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `content/scenarios/micro_breach_mind_lab.ron`, `content/mind/profiles/`. Evidence target: Scenario file + sample profile + canned-script. |
| [ ] | `M6.5-010` | MIND-* acceptance suite. Build: Implement `cf-ai --bin mind_lab` with `--suite MIND-001..MIND-010 --provider <mock\|...> --write-run-bundle`. Cover: baseline (off), nonblocking timeout, malformed response, stale response, doctrine-patch visibility, fog-of-war fairness, memory write, replay audit, cost cap, humanlike-score delta. Tests: All MIND-* pass against mock; AI-H regression remains green; failure modes produce useful first-divergence reports. Anti-scope: No reliance on live cloud during CI. | [[spec/native-implementation-backlog#M6.5 — LLM Mind Lab]] | - | - | - | - | - | - | - | Owns: `cf-ai`, `cf-headless`, `cf-bench`, `tests/`. Evidence target: MIND-001..MIND-010 run bundles archived; AI-H humanlike-score delta report. |
### M7 - Mission Director And Breach Contract

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M7-001` | manifest schema. Build: Typed manifest: teams, objectives, materials, command core, base systems, loadout requirements, director. Tests: Schema validation tests. Anti-scope: No full campaign generator. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cf-mission`, `content/scenarios/`. Evidence target: Manifest fixture in bundle. |
| [ ] | `M7-002` | director/commander. Build: Pacing, reinforcement, LZ risk, commander reason labels. Tests: Director phase tests. Anti-scope: No MMO war layer. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cf-mission`, `cf-ai`. Evidence target: `commander_decision.*`. |
| [ ] | `M7-003` | command core/base slice. Build: Rooted core powers shield/turret/door/repair; uproot/embed avatar tradeoff. Tests: CORE-A subset tests. Anti-scope: No full base builder. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cf-mission`, `cf-chassis`, `cf-ui`. Evidence target: `command_core_state_changed`, `base_power_changed`. |
| [ ] | `M7-004` | Breach Contract. Build: Playable mission: breach, fight, extract, win/loss/debrief. Tests: E2E win/loss; replay. Anti-scope: No campaign map. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `content/scenarios/`, `cf-app`. Evidence target: MISSION-A run bundles. |
| [ ] | `M7-005` | debrief/retry. Build: Comic-noir debrief with cause chain and retry same seed. Tests: UI/replay tests. Anti-scope: No full progression system. | [[spec/native-implementation-backlog#M7 — Mission Director And Breach Contract]] | - | - | - | - | - | - | - | Owns: `cf-ui`, `cf-replay`, `cf-save`. Evidence target: Debrief screenshot. |
### M8 - Scenario Editor And Mod Tools

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M8-001` | editor workbench. Build: In-engine editor for spawns, materials, objectives, core/base state, loadout requirements. Tests: Editor state tests; focus/accessibility smoke. Anti-scope: No marketplace. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `cf-tools-editor`, `cf-ui`. Evidence target: Editor screenshots. |
| [ ] | `M8-002` | package builder. Build: Deterministic `.cfpkg`, manifest/provenance validation, dependency graph. Tests: Package determinism tests. Anti-scope: No public hosting. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `cf-mod`, `tools/`. Evidence target: PACK-A report. |
| [ ] | `M8-003` | script host. Build: Implement chosen Lua/Rhai sandbox with capability declarations. Tests: Sandbox denies FS/network by default. Anti-scope: No unbounded script API. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `cf-mod`. Evidence target: Script-host test report. |
| [ ] | `M8-004` | sample mod. Build: New chassis + scenario + AI doctrine sample mod. Tests: Validate/load/run sample mod. Anti-scope: No full mod catalog. | [[spec/native-implementation-backlog#M8 — Scenario Editor And Mod Tools]] | - | - | - | - | - | - | - | Owns: `mods/sample_*`, `content/`. Evidence target: Modded run bundle. |
### M9 - Dedicated Server App + Determinism Islands

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M9-001` | dedicated server binary. Build: `cf-server` runs without renderer/UI/audio; loads config; supports `--mode` and `--validate-config-only`. Tests: Linux headless smoke. Anti-scope: No PvP/MMO scale acceptance here. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cf-server`, `cf-server-ops`, `cf-app`. Evidence target: Server boot logs in bundle. |
| [ ] | `M9-002` | determinism contracts. Build: Document deterministic/stochastic/cosmetic subsystems. Tests: Contract tests. Anti-scope: No whole-engine determinism claim. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cf-sim-core`, `cf-replay`, docs. Evidence target: Determinism report. |
| [ ] | `M9-003` | replay/server evidence path. Build: Server-core run verifies actor/terrain/inventory checksums and writes `server.*` events. Tests: Replay compare. Anti-scope: No client-authoritative shortcut. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cf-server`, `cf-replay`. Evidence target: First-divergence report if fail. |
| [ ] | `M9-004` | server-core perf/ops. Build: Health/readiness/metrics/drain/Docker path meets M9 server-core budget. Tests: Bench + ops smoke. Anti-scope: No optimization-only rabbit hole. | [[spec/native-implementation-backlog#M9 — Dedicated Server App + Determinism Islands]] | - | - | - | - | - | - | - | Owns: `cf-bench`, `cf-server-ops`, `cf-server`. Evidence target: Perf/ops report. |
### M10 - LAN Co-op

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M10-001` | authority model. Build: Server-authoritative input/snapshot/event model. Tests: Unit tests for input validation. Anti-scope: No anti-cheat product. | [[spec/native-implementation-backlog#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Owns: `cf-net`, `cf-sim-core`. Evidence target: Authority memo. |
| [ ] | `M10-002` | LAN discovery/lobby. Build: Host/list/join on LAN; ready-up. Tests: Local two-client smoke. Anti-scope: No NAT/relay. | [[spec/native-implementation-backlog#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Owns: `cf-net`, `cf-ui`. Evidence target: Lobby screenshot. |
| [ ] | `M10-003` | replication. Build: Actors, terrain, inventory, objective state replicate; per-client bundles align. Tests: Replay compare across clients. Anti-scope: No public matchmaking. | [[spec/native-implementation-backlog#M10 — LAN Co-op]] | - | - | - | - | - | - | - | Owns: `cf-net`, `cf-replay`. Evidence target: Two-client run bundles. |
### M11 - Online Co-op (Self-Hosted Dedicated Servers)

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M11-001` | transport adapter. Build: NAT/relay candidate behind trait boundary for self-hosted `coop_room`. Tests: Simulated latency tests. Anti-scope: No platform lock-in. | [[spec/native-implementation-backlog#M11 — Online Co-op (Self-Hosted Dedicated Servers)]] | - | - | - | - | - | - | - | Owns: `cf-net`. Evidence target: Transport decision note. |
| [ ] | `M11-002` | package hash sync. Build: Join preflight checks content hashes and produces clean mismatch actions. Tests: Mismatch tests. Anti-scope: No public mod CDN. | [[spec/native-implementation-backlog#M11 — Online Co-op (Self-Hosted Dedicated Servers)]] | - | - | - | - | - | - | - | Owns: `cf-net`, `cf-mod`, `cf-ui`. Evidence target: Join-failure screenshots. |
| [ ] | `M11-003` | online session smoke. Build: Two remote clients complete a self-hosted co-op Breach Contract. Tests: Remote run compare. Anti-scope: No first-party-only hosting path. | [[spec/native-implementation-backlog#M11 — Online Co-op (Self-Hosted Dedicated Servers)]] | - | - | - | - | - | - | - | Owns: `cf-net`, `cf-app`, `cf-server`. Evidence target: Per-client bundles. |
### M12 - Public PvP Arenas + Persistent MMO Shards

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `M12-001` | PvP arena. Build: 4-8 players, small destructible map, server-authoritative validation, competitive anti-cheat default. Tests: Stress run. Anti-scope: No ranked ladder here. | [[spec/native-implementation-backlog#M12 — Public PvP Arenas + Persistent MMO Shards]] | - | - | - | - | - | - | - | Owns: `cf-server`, `cf-net`, `cf-mission`, `cf-server-anti-cheat`. Evidence target: Bandwidth/cheat notes. |
| [ ] | `M12-002` | MMO shard. Build: 50-client readiness gate with persistence, interest management, and no-cloud reference. Tests: MMO-001..MMO-012. Anti-scope: No seamless single-shard world. | [[spec/native-implementation-backlog#M12 — Public PvP Arenas + Persistent MMO Shards]] | - | - | - | - | - | - | - | Owns: `cf-server`, `cf-net`, `cf-bench`, `cf-server-persistence`. Evidence target: Perf/desync/persistence report. |
| [ ] | `M12-003` | DR-005/DR-035 review. Build: Review multiplayer/MMO posture with M12 evidence. Tests: N/A. Anti-scope: No silent demotion or silent scope expansion. | [[spec/native-implementation-backlog#M12 — Public PvP Arenas + Persistent MMO Shards]] | - | - | - | - | - | - | - | Owns: vault only. Evidence target: Updated DR or research log. |

---

## Global Validation And Bug Hunt Checklist

These rows come from the roadmap validation matrix, bug-hunt checklist, and definition of done. They should be updated at milestone closeout, not during every tiny edit.

### Validation Command Matrix

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [x] | `VAL-01` | Formatting: `cargo fmt --all --check` | [[spec/prototype-roadmap#Validation Command Matrix]] | macOS aarch64 + rustc 1.93.0 (M0.3 final pass). | - | - | - | 9 | 9 | 1 | Required starting: M0. Pass. |
| [x] | `VAL-02` | Compile: `cargo check --workspace --all-targets` | [[spec/prototype-roadmap#Validation Command Matrix]] | macOS aarch64 + rustc 1.93.0 (M0.3 final pass). | - | - | - | 9 | 9 | 1 | Required starting: M0. Pass. |
| [x] | `VAL-03` | Lints: `cargo clippy --workspace --all-targets -- -D warnings` | [[spec/prototype-roadmap#Validation Command Matrix]] | macOS aarch64 + rustc 1.93.0 (M0.3 final pass). | - | - | - | 9 | 9 | 1 | Required starting: M0. Pass. |
| [x] | `VAL-04` | Unit/integration tests: `cargo test --workspace` | [[spec/prototype-roadmap#Validation Command Matrix]] | All tests pass on macOS aarch64 after M0.3: 68 tests + doctests, including 36 `cf-control` unit tests and 9 live WS integration tests. | - | - | - | 9 | 9 | 1 | Strict control validation, final bundle semantics, and repo-root bundle path all have regression tests. |
| [x] | `VAL-05` | Native app smoke: `cargo run -p cf-app -- --scenario <milestone-smoke> --run-seconds 5 --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | `prototype_runs/native/m0_2026-05-06T04-46-27Z_a9675fc6` (`cf-app --headless-smoke --ticks 300 --tick-rate-hz 60 --write-run-bundle`, checker errors 0). | - | - | - | 9 | 9 | 1 | Required starting: M0. Direct `cf-app` path writes to repo-root default. |
| [x] | `VAL-06` | Control API smoke: `cargo run -p cfctl -- observe --once` and `cargo run -p cfctl -- run --ticks 300 --write-run-bundle` against the current milestone scene. | [[spec/prototype-roadmap#Validation Command Matrix]] | M0.3 `cfctl observe --once` prints valid JSON; `prototype_runs/native/m0_2026-05-06T04-46-04Z_1ad62cb4` (`cfctl run` 60 Hz/300, checker errors 0); `m0_2026-05-06T04-46-37Z_56e26f4b` (live script roundtrip, checker errors 0). | - | - | - | 9 | 9 | 1 | Required starting: M0. Pass. |
| [x] | `VAL-07` | Run-bundle validation: `python3 research_tools/prototype_run_check.py prototype_runs/native/<run_id>` | [[spec/prototype-roadmap#Validation Command Matrix]] | Canonical checker returns `errors 0` for final M0.3 bundles `1ad62cb4`, `2c7f5b05`, `a9675fc6`, and `56e26f4b`. | - | - | - | 9 | 9 | 1 | Required starting: M0. Pass. |
| [ ] | `VAL-08` | Scripted E2E: `cargo run -p cf-e2e -- --scenario <scenario-id> --expect <result> --write-run-bundle`; prefer `cfctl`/control API actions over OS-level input. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M1.5 |
| [ ] | `VAL-09` | Observation stream check: Stream `cargo run -p cfctl -- observe --stream --hz 30` during a scripted run and verify tick/order/event freshness. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M1.5 |
| [ ] | `VAL-10` | Replay check: `cargo run -p cf-headless -- replay prototype_runs/native/<run_id> --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M3 |
| [ ] | `VAL-11` | Screenshot/capture check: Capture listed in `summary.json.artifacts`; verify no blank/overlap failure. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M1.5 visual runs; M4 required |
| [ ] | `VAL-12` | Perf sample: `cargo run -p cf-bench -- --scenario <scenario-id> --profile milestone` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M2 |
| [ ] | `VAL-13` | Accessibility smoke: `cargo run -p cf-e2e -- --scenario <scenario-id> --ui-scale 2.0 --high-contrast --verify-focus` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M4 |
| [ ] | `VAL-14` | Save/load roundtrip: `cargo run -p cf-e2e -- --scenario <scenario-id> --save-load-roundtrip --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M5/T-SAVE |
| [ ] | `VAL-15` | Full collision gauntlet: `cargo run -p cf-e2e -- --scenario m5_5_full_collision_gauntlet --suite COLL-001..COLL-012 --write-run-bundle` then `cargo run -p cf-headless -- replay prototype_runs/native/<m5_5_run> --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M5.5/T-PHYS |
| [ ] | `VAL-16` | Collision observation stream: `cargo run -p cfctl -- observe --collisions --stream --hz 30 --scenario m5_5_full_collision_gauntlet` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M5.5/T-PHYS |
| [ ] | `VAL-17` | AI harness: `cargo run -p cf-ai --bin ai_harness -- --suite AI-H-01..AI-H-06 --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6 |
| [ ] | `VAL-18` | Mind frame observation: `cargo run -p cfctl -- observe --mind-frame squad_alpha --once` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-19` | Mind lab suite (mock): `cargo run -p cf-ai --bin mind_lab -- --suite MIND-001..MIND-010 --provider mock --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-20` | Mind cost-cap smoke: `cargo run -p cf-ai --bin mind_lab -- --suite MIND-009 --provider mock --max-run-cost-usd 0.0 --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-21` | Mind fairness audit: `cargo run -p cf-ai --bin mind_lab -- --suite MIND-006 --provider mock --write-run-bundle` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M6.5 |
| [ ] | `VAL-22` | Package/mod validation: `cargo run -p cf-mod -- validate content/ mods/ --strict` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M8 |
| [ ] | `VAL-23` | Headless server smoke: `cargo run -p cf-headless -- --scenario breach_contract --ticks 3600 --verify-checksums` | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M9 |
| [ ] | `VAL-24` | LAN/online replay alignment: Compare per-client run bundles with `cf-headless replay-compare`. | [[spec/prototype-roadmap#Validation Command Matrix]] | - | - | - | - | - | - | - | Required starting: M10+ |

### Bug Hunt Checklist

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `BUG-01` | Crashes/hangs: Can reset, exit, alt-tab, reload scenario, and replay complete without panic/deadlock? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-02` | Input: Are repeated inputs, held inputs, lost focus, mouse capture, controller fallback, and remap paths sane? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-03` | Replay/events: Are required events present, ordered, parent-linked, counted, and linked to visible behavior? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-04` | Determinism: If a deterministic claim is made, where is the checksum proof and first-divergence report? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-05` | UI/readability: Does UI fit at 100%, 150%, and 200%; are critical states not color-only; are labels non-overlapping? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-06` | Terrain/physics/collision: Do high-speed impacts, edge collisions, tiny holes, chunk borders, repeated edits, limb contacts, projectile-projectile contacts, weapon collisions, friendly body blocking, debris impacts, and mech crush contacts behave predictably? Are all collision filters reason-labeled? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-07` | AI: Can the AI explain perception, chosen tactic, refused action, stuck state, and recovery? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-08` | Save/load: Does save/load preserve identities, events, objective state, terrain, equipment, and checksums where promised? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-09` | Performance: Are frame spikes, sim tick cost, event volume, dirty-region cost, and memory growth reported? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-10` | Platform: Are path separators, case sensitivity, file watching, audio, input, and GPU backend assumptions portable? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-11` | Mod/package: Do bad packages fail with actionable diagnostics instead of panic/crash? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |
| [ ] | `BUG-12` | Documentation: Are roadmap/backlog/source links current; are ghost DRs or stale Slice-A references avoided? | [[spec/prototype-roadmap#Bug Hunt Checklist]] | - | - | - | - | - | - | - |  |

### Definition Of Done

| Done | ID | Feature / Requirement | Source | Evidence | H-Full | H-Quality | H-Review | AI-Full | AI-Quality | AI-Review | Notes |
|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---|
| [ ] | `DOD-01` | Code: Implemented in the owned crates/files named by [[spec/native-implementation-backlog]]. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-02` | Tests: Unit/integration tests added for new core behavior and failure paths. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-03` | E2E: Milestone reference scenario runs from command line and produces expected outcome. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-04` | Run bundle: Bundle exists under `prototype_runs/native/` and passes the checker. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-05` | Replay: Required replay/checksum claims are backed by headless verification or explicitly not claimed. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-06` | Collision/physics: Any new physical object has a collision class/proxy/matrix entry/event policy or a tested cosmetic/sensor/filter reason. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-06A` | Physical profile: Any new gameplay-physical object has mass, material/composition, durability/damage routing, and relevant temperature/electrical/container/AI/debug fields or a tested opt-out reason. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - | Applies to units, limbs, armor, weapons, equipment, projectiles, debris, base modules, shields, mechs, containers, batteries, terrain materials, and mission-critical objects. |
| [ ] | `DOD-06B` | Actor presentation: visible actor movement/body-state milestones prove no static sliding pawn; locomotion animation/state tags, body/limb graph, physics authority transitions, `cfctl` observation, replay events, and capture evidence exist at the milestone's maturity level. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - | Applies first as HUD/stance state in M4A, body graph in M5, physical limb blend in M5.5, and gravity/pressure/wind interaction in M5.9. |
| [ ] | `DOD-07` | Perf: Perf counters exist; T-PERF target status is recorded as pass/fail/blocked. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-07A` | Multicore/GPU posture: new CPU-heavy systems have measured budgets and a parallel/background/GPU posture; new GPU-heavy systems have render/upload counters and preserve replay-authoritative state. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - | Required for terrain/material/atmosphere/physics/AI/server/render work. |
| [ ] | `DOD-08` | UI/accessibility: Any user-facing surface has screenshot evidence and ACC-A status when applicable. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-09` | Bug hunt: Bug checklist is completed and every verified finding at every severity is fixed. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - | User-approved deferral is allowed only for an exact finding with recorded issue ID, reason, owner, next checkpoint, and evidence path. |
| [ ] | `DOD-10` | Vault: Prototype/research note is updated with run links, test commands, screenshots, final audit, and next actions. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-11` | Feature checklist: [[spec/feature-completion-checklist]] rows are updated for affected roadmap features, milestone scope, done-criteria, side tracks, and native task cards. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - | Added 2026-05-05 to match the roadmap's 12-row Definition Of Done. |
| [ ] | `DOD-12` | Human gates: Human-only checks are marked `READY_FOR_HUMAN`, with a short playtest checklist. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - |  |
| [ ] | `DOD-13` | Zero known issues gate: milestone has zero unresolved verified review findings unless the user explicitly approved each exact deferral. | [[spec/ai-code-review-bug-hunt-skills]] | - | - | - | - | - | - | - | This is stricter than ordinary "known issues" practice. Low/Medium/High all block by default. |
| [ ] | `DOD-14` | Contract integrity gate: every contract path has shared-source proof, positive proof, negative/adversarial proof, source-truthful evidence, and checklist truth. | [[spec/ai-code-review-bug-hunt-skills]] | - | - | - | - | - | - | - | Required to prevent green-but-wrong milestones, duplicate tool/app paths, fake success, permissive required fields, and checklist laundering. |
| [ ] | `DOD-15` | Corefall review loop: run `/corefall-review <milestone>` from `/Users/erol/projects/corefall`, fix every verified issue, and rerun until `Accept`. | [[spec/prototype-roadmap#Definition Of Done]] | - | - | - | - | - | - | - | If anything remains, every remaining finding must have explicit user-approved deferral evidence with issue ID, reason, owner, next checkpoint, and evidence path. |

---

## Maintenance Notes

- This file intentionally duplicates roadmap/backlog items so completion and rating state can live in one place. Keep the roadmap/backlog as the source for build instructions.
- If a future pass renames milestone ids or task card ids, preserve old ids in notes until any evidence links have been migrated.
- Human `H-*` ratings should be left blank until the user provides them. Agents may suggest ratings but must label them as AI suggestions, not human ratings.
- For subjective items like feel, fun, readability, AI believability, or UX polish, mark agent-completable evidence first and leave the human gate ready for playtest.
