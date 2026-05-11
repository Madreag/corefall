# cf-chassis — AGENTS.md

## Owns
- M5 chassis grammar (DR-014 + DR-021): body graph, layered armor, modules, pilot binding, damage stages, salvage + repair pipeline, tutorial-safety policy.
- **Body graph**: 15-zone `BodyZone` enum (Head, Torso, ArmLeft, ArmRight, ForearmLeft, ForearmRight, HandLeft, HandRight, LegLeft, LegRight, ShinLeft, ShinRight, FootLeft, FootRight, Backpack) with `parent()` kinematic chain + side-chain predicates; `BodyGraph` carries zones[] + 14 joints + 5 equipment sockets + 15 movement-contribution records per spec.
- **Layered armor**: 3-layer `ArmorLayer` (External → Internal → Core) per zone with `hp`/`hp_max`/`hardness`/`integrity`; `ZoneState` holds layers[] + wound container + destroyed flag.
- **Modules**: `ChassisModule` with 5 `ModuleKind` variants (WeaponMount, Jet, Shield, Sensor, RepairDrone), 5 `ModuleStateKind` rungs (Nominal → Degraded → Warning → Failed → NotPresent), `bound_zone` propagation so module health follows zone health.
- **Damage pipeline**: 11-stage `ChassisStage` enum (Nominal → Degraded → ModuleWarning → ModuleFailed → WeaponJammed → ArmorCracked → Disabled → PilotInjured → Eject → BailedTooLate → Wreck → Gibbed); `recompute_stage` advances monotonically based on layer/module/pilot signals.
- **Pilot lifecycle**: `PilotState` (Bound → Injured → Ejecting → Ejected → Extracted / BailedTooLate / Lost) + `EjectWindow` with tick-rate-stable duration; `attempt_eject` / `tick_eject` / `mark_extracted` mutators.
- **Tutorial-safety policy**: when `tutorial_safety = true`, lethal damage caps at `PilotInjured` and `attempt_eject` cannot transition to `BailedTooLate` during the tutorial window.
- **3 chassis archetypes**: `infantry_spec()`, `powered_armor_spec()`, `light_mech_spec()` returning canonical `ChassisSpec` presets; `chassis_specs()` registry + `chassis_spec(id)` lookup resolves `INFANTRY_ID`, `POWERED_ARMOR_ID`, `LIGHT_MECH_ID`.
- Determinism contract: every public mutator is pure (`&mut self`), no clock reads, no `rand::thread_rng()`; the engine seeds any RNG it needs for jam rolls and feeds it in explicitly.

## Public API Boundary
- Types: `ChassisKind`, `ChassisSpec`, `ChassisState`, `BodyGraph`, `BodyZone`, `ArmorLayer`, `ArmorLayerKind`, `ZoneState`, `ChassisModule`, `ModuleKind`, `ModuleStateKind`, `ChassisStage`, `PilotState`, `EjectWindow`, `EjectAccepted`, `EjectProgress`, `Joint`, `EquipmentSocket`, `MovementContribution`, `ZoneDamageOutcome`, `RepairOutcome`, `SalvageOutcome`, `LayerDamage`, `LayerGlance`, `ModuleTransition`.
- Functions: `infantry_spec()`, `powered_armor_spec()`, `light_mech_spec()`, `chassis_specs()`, `chassis_spec(id)`, `ChassisState::force_stage(stage)` (test-only stage override).
- Constants: `INFANTRY_ID`, `POWERED_ARMOR_ID`, `LIGHT_MECH_ID`, `SOCKET_HAND_RIGHT`, `SOCKET_HAND_LEFT`, `SOCKET_BACK_MOUNT`, `SOCKET_HEAD_MOUNT`, `SOCKET_TORSO_HARDPOINT`.

## Does NOT Own
- Per-zone capsule collision proxies + impulse-to-damage routing → `cf-physics` collision pipeline at M5.5 (DR-033).
- CA material chemistry on chassis (acid eating armor, fire propagation through joints) → `cf-material` at M5.6 (DR-036).
- Pipe networks / suit life-support / atmospherics interaction → `cf-atmos` at M5.9 (DR-037).
- AI doctrine / utility scoring over chassis stage → `cf-ai` at M6.
- Rendering / sprite atlases / animation rigs → `cf-render-2d`.
- HUD presentation of module strip / body silhouette → `cf-ui` (formatters read `ChassisView` projections from `cf-control`).
- Equipment role records / firing profiles → `cf-equipment` (chassis depends on it for `FiringProfile` lookup during module binding).

## Test Surface
- Unit tests: `cargo test -p cf-chassis` — 20 tests covering body graph (parent chain, hand-right disables rifle, shin-left reduces speed), armor layers (glance, External → Internal → Core breach ladder), modules (backpack destruction fails jet, repair restores modules), damage stages (50% torso → ArmorCracked, full torso → Disabled), eject window (start, complete to Ejected, BailedTooLate after wreck), tutorial safety (caps at PilotInjured, blocks eject to Lost), salvage (pulls surviving modules), jam (round-trip jam + clear), checksum stability + zone-damage differentiation, registry round-trip for canonical ids.
- BODY-A + CHASSIS-A acceptance: exercised end-to-end via `cf-control` engine + `cf-actor::ChassisState` attachment + cfctl scripts `m5_chassis_wreck_eject` (win + loss) + `m5_chassis_salvage_roundtrip`.

## Cross-Crate Contracts
- Depends on: `cf-equipment` (for `FiringProfile`/`Role` lookup when modules bind to weapons).
- Depended on by:
  - `cf-actor` — `ActorState.chassis: Option<ChassisState>`; `apply_zone_damage` routes through chassis layers when attached; `Stance::from_chassis` derives Crouching/Climbing/Jetting/Ejecting; `BodySilhouette` reads chassis when `placeholder=false`.
  - `cf-save` — `SaveBlob` serializes the full `ChassisState` (zones + modules + pilot + eject_window + weapon_jammed) through blake3-checksummed canonical JSON.
  - `cf-control` — `emit_chassis_events` emits `chassis.armor_layer_damaged`, `armor_layer_glanced`, `armor_zone_destroyed`, `joint_severed`, `module_state_changed`, `stage_changed`, `pilot_ejected`, `pilot_separated`, `pilot_bailed_too_late`, `pilot_extracted`, `repaired`, `salvaged`, `weapon_cleared`; `tick_chassis_eject_for_all` runs every tick; `refresh_hud_chassis_banners` raises severity-tagged HUD banners on stage transitions; `observe.once` exposes the full `ChassisView` projection.
  - `cf-ui` — `module_strip` + `silhouette_line` formatters consume `ChassisView` projections.

## Common Pitfalls
- Do NOT call `recompute_stage` without first calling `apply_zone_damage` (or another state mutator) — stage advancement is monotonic and reads the current layer/module/pilot state; calling it standalone is a no-op but masks logic errors.
- `tutorial_safety` caps `recompute_stage` at `PilotInjured` and blocks `attempt_eject` from transitioning to `BailedTooLate`. Setting it true in non-tutorial scenarios hides real wreck/gib outcomes from replay.
- `salvage` requires the chassis to be in `Wreck`, `Disabled`, or `Gibbed` stage; calling it on a `Nominal` chassis returns `SalvageOutcome` with `accepted = false` and a `chassis_not_wreck_or_disabled` reason — this is intentional, not a bug.
- The 15 `BodyZone` discriminants are `repr(u8)` with stable IDs; granular variants (Forearm / Hand / Shin / Foot) are appended after the legacy 7 primary zones to preserve cross-milestone checksum + serialization stability. Inserting new variants in the middle of the enum breaks replay determinism.
- `attempt_eject` returns an `EjectAccepted { ticks_total }` only on first call; subsequent calls during an already-running eject window return `None` so the engine doesn't restart the timer.
- `EjectWindow` duration is computed from real-time seconds × `tick_rate_hz`; the same scenario produces a 60-tick eject at 60 Hz and a 120-tick eject at 120 Hz so real-time pacing is preserved.

## Source Trail
- spec/prototype-roadmap §M5 — Equipment, Chassis, And Damage Grammar.
- spec/chassis-armor-mechs-and-origins.
- spec/body-damage-model.
- DR-014 (chassis grammar; CLOSED at M5).
- DR-021 (origin compatibility; CLOSED at M5).
- DR-029 (save-game model; M5 slice via cf-save).
- docs/implementation-log/2026-05-10-m5-chassis-grammar.md.
