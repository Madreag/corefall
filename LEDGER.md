# Corefall — Refactor Ledger

**Purpose**: persistent cross-session memory. Read this FIRST when resuming work.
Last updated: 5/20/2026 11:55 PM MST

---

## User's Standing Directives

1. **No compromise** — if code has hardcoded shortcuts or simulation/perf/tech blockers, FIX them. Refactor as needed.
2. **Don't be lazy** — loop on this until ALL compromises are fixed.
3. **Make all hardcoded tuning values configurable** via JSON/RON content files.
4. **Maintain this ledger** so context-compaction doesn't lose progress.
5. **Make no mistakes** — don't break determinism. 4281+ tests must keep passing.
6. **Commit cleanly** — each logical chunk gets its own commit so reverts are easy.

---

## Session History

### Session 1 (5/20/2026): M15B implementation
- Implemented M15B (GPU material kernel + precipitation cycle) from scratch
- Created cf-material-gpu crate, precipitation module, liquid_flow module
- 3 new event schemas, 2 scenarios, JSON loaders

### Session 2 (5/20/2026): M15B audit + chemistry pass
- User caught: "for paper/fabric/wood... the 2-output limit"
- Lifted 2-output limit: added `emissions: Vec<MaterialId>` to MaterialReaction
- Refactored kernel orchestrator to spawn emissions in adjacent air cells (NESW order)
- Chemistry audit fixed 7 incorrect reactions:
  - water+fire → smoke (not air) + water-gas shift hydrogen variant at >973K
  - rain+fire → smoke + water-gas shift hydrogen variant
  - concrete+acid → CO2 (not steam) [`CaCO3 + 2HCl → CaCl2 + H2O + CO2`]
  - ore_copper+acid → hydrogen (not ammonia) [`Cu + H2SO4 → CuSO4 + H2`]
  - alcohol+fire emits CO2 (ethanol combustion)
  - H2+O2 → steam+steam (both reactants → water)
  - acid+alkali emits steam (exothermic)
- Cascade reactions (wood/paper/fabric/oil/fuel/coal/gunpowder) now emit smoke+CO2 via emissions
- Added pressure to precipitation: PrecipitationInputs.ambient_pressure_kpa + pressure_rate_multiplier

### Session 3 (5/20/2026): Projectile mass/sharpness wiring
- Removed compromise: engine.rs had hardcoded mass=0.05, sharpness=0.8
- Added `bullet_mass_kg` + `bullet_sharpness` to RifleSpec, FiringProfile, Projectile
- Wired through 17 weapon presets with real-world calibers:
  - 5.56 NATO 4g, 9mm pistol 8g, 7.62 NATO 9.8g, .50 BMG 45g, 20mm anti-materiel 850g
  - HEAT shaped-charge 10kg/sharpness 0.7
  - APFSDS DU long-rod 8kg/sharpness 0.98

### Session 4 (5/20/2026): Content-driven extraction
- User: "should these hardcoded values be in a settings file"
- Made reaction registry loadable from JSON (`content/materials/reaction_registry.json`)
- Made phase registry loadable from JSON (`content/materials/phase_registry.json`)
- Made precipitation tuning loadable from JSON (`content/materials/precipitation_config.json`)
- Tank rounds (HEAT, APFSDS) now carry bullet_mass + sharpness in their RON files
- Added 10 content-driven registry tests
- Committed as 165a79f7

### Session 5 (5/20/2026): M15 kernel engine wiring
- User: "commit and continue working on all, maintain a ledger md files"
- Created LEDGER.md (this file)
- Wired M15 kernel_step into cf-control engine's drive_tick
- Added MaterialKernel, ReactionRegistry, PhaseRegistry, HeatField, prev_heat_field,
  PrecipitationCycle, PrecipitationConfig fields to EngineMutable
- Initialized them in EngineMutable construction (loaders fall back to hardcoded defaults)
- Per-tick kernel_step call after dig/projectile/mission, before checksum
- Routes reaction_triggered + phase_transition + cellular_step events to recorder
- Wired precipitation cycle: scans steam pixels per tick (cap 4096), observes them,
  applies cloud + rain pixel writes to terrain, drains nucleation + precipitation events
- Routes phase_nucleated + precipitation_started events to recorder
- 4 new engine-integration tests (VAL-M15-ENGINE-001..004)
- 4285 tests passing (was 4281 + 4 new)
- Committed as 0ae23952

### Session 6 (5/20/2026): Wire HeatField + AmbientWorld from scenario
- HeatField now populated from scenario atmosphere_cells (was stub-init at ambient)
- AmbientWorld inferred from scenario id (*_vulcan*, *_mimas*, *_mars*, else Earth)
- Phase transitions can now actually fire because temperature changes
- m15b_acid_rain_vulcan now actually produces acid_droplet pixels
- Committed as 6c3ed1d5

### Session 7 (5/20/2026): M15 perf - O(1) reaction lookup
- Added ReactionLookup struct with (a*256+b) → reaction index table
- Added [bool; 256] reactive_bitmap for O(1) is_reactive check
- Refactored dispatch_reactions_in_chunk + try_fire_reaction to use lookup
- Lookup built once per step, reused across all chunks
- Bench: m15-ca-burst @ 100K pixels still ~55ms p99 (dispatch not bottleneck;
  Margolus CA stepper + per-chunk material_at/set_material_pixel BTreeMap
  lookups are the real cost — need parallel + cache optimization)
- 4285 tests passing
- Committed as fd39e529

### Session 9 (5/20/2026): M15 material affordances + render visibility
- Extended cf-terrain MATERIAL_TABLE from launch-9 → 21 entries
- Added 12 M15 affordances: water(13), oil(16), acid(21), lava(26), iron(29),
  co2(43), steam(50), smoke(62), fire_intense(65), cloud(71), rain(87),
  acid_droplet(88)
- Includes physics-relevant fields: hardness, hazard+damage_per_tick,
  density, friction, stickiness, overlay_rgba (visible colors)
- Smoke blocks LOS=true (combat-relevant), others false
- Hazardous materials (acid 2 dpt, lava 12 dpt, fire_intense 8 dpt,
  acid_droplet 2 dpt) emit per-tick actor damage
- 2 new tests: val_m15b_material_affordances_cover_active_set + val_m15b_hazardous_materials_damage_actors
- Result: players now SEE fire spreading, steam rising, rain falling, etc.
  instead of invisible chemistry behind transparent pixels
- 4288 tests passing (was 4286, +2 new)
- Committed as 4a2243a2

### Session 8 (5/20/2026): Steam scan opt + dynamic heat field
- Steam pixel scan now iterates ONLY awake chunks (was O(W*H) full world scan)
  - For typical scene: 4 awake chunks × 4096 = 16K lookups (vs 1M)
  - Falls back to allocated chunks for tick-0 before wake/sleep gating settles
  - Committed as 3c6ca1ed
- HeatField now updates DYNAMICALLY each tick from hot materials
  - Sources: fire_intense(65)→1200K, lava(26)→1473K, lightning(64)→30000K, electric_arc(63)→6000K
  - inject_thermal_sources_and_diffuse helper runs BEFORE kernel_step
  - One diffuse() pass per tick spreads heat to 4 neighbors (mix=0.05)
  - Without this, phase transitions could only fire at scenarios with
    pre-heated cells — fire spreading mid-game wouldn't heat anything
  - VAL-M15-ENGINE-005 proves determinism is preserved (byte-identical
    reaction payloads across two independent runs)
  - 4286 tests passing

---

## Completed Work (committed)

| # | Description | Files | Tests |
|---|---|---|---|
| 1 | M15B emissions extension lifting 2-output reaction limit | cf-material/src/reactions.rs, kernel.rs | 4 emissions tests |
| 2 | Chemistry audit: 7 reactions corrected for real-world accuracy | cf-material/src/reactions.rs | M15 acceptance tests |
| 3 | Pressure-aware precipitation per M19 dependency | cf-material/src/precipitation.rs | 9 pressure tests |
| 4 | Bullet mass/sharpness wired through 17 weapon presets | cf-equipment/* (18 files), cf-actor/sim.rs, cf-control/engine.rs | M14 acceptance tests |
| 5 | Reaction registry JSON-loadable | reaction_registry.json + reactions.rs loaders | VAL-CONTENT-001/004/008-010 |
| 6 | Phase registry JSON-loadable | phase_registry.json + phase.rs loaders | VAL-CONTENT-002/005 |
| 7 | Precipitation config JSON-loadable | precipitation_config.json + precipitation.rs | VAL-CONTENT-003/006/007 |
| 8 | rayon added to workspace + chunk_summary_entries parallelized | cf-terrain/Cargo.toml, chunked.rs | existing tests |
| 9 | M15B GPU dispatch (real wgpu pipeline with buffers + readback) | cf-material-gpu/* | 32 GPU-feature tests |
| 10 | ForensicsDump helper for GPU-CPU divergence | cf-physics/determinism.rs | 4 forensics tests |
| 11 | cf-bench m15b-gpu-vs-cpu performance benchmark | cf-bench/src/m15b_gpu_vs_cpu.rs | 3 bench tests |

**Current test totals**: 4281 workspace tests passing, 0 failures. 32 GPU-feature tests passing.

---

## Pending Work (NOT YET DONE)

### HIGH PRIORITY — true blockers to proper simulation

1. ~~**M15 kernel_step not wired into cf-control engine drive_tick**~~ ✅ DONE in Session 5

2. **No actual parallelism in sim hot path**
   - rayon added to workspace, chunk_summary parallelized (small win)
   - M15 kernel single-threaded — bench shows 56ms p99 at 100K pixels (14× over 4ms budget)
   - Add 4-color phase parallel mode to dispatch_reactions_in_chunk + dispatch_phase_in_chunk
   - Requires careful determinism handling (collect-then-apply pattern; within-chunk cascade via local override map)
   - `MaterialKernel::with_parallel(bool)` opt-in API exists; needs implementation

3. **CA stepper single-threaded**
   - cf-terrain/src/ca.rs step_ca_filtered iterates chunks sequentially
   - Margolus 2x2 pattern is data-parallel within parity but cross-chunk writes serialize it
   - Same parallelization approach as #2

### MEDIUM PRIORITY — performance / config / scale

4. **Material loader doesn't load FROM content registry JSON in production**
   - Engine doesn't call MaterialRegistry::load_from_file at scenario start
   - Hardcoded fallback only

5. **No SIMD in penetration math**
   - cf-physics::try_penetrate is per-projectile sequential
   - Could batch with SIMD (f32x4) for the impulse formula

6. **MaterialId = u8 caps at 256 materials**
   - Design tradeoff (1 byte per pixel) but modders will hit this
   - Currently 89/256 used

7. **No projectile pair pass parallelism**
   - cf-physics run_projectile_pair_pass iterates candidates serially
   - Narrowphase resolution per candidate could be parallel within projectile-disjoint groups

8. **Steam pixel scan in precipitation is O(width × height) per tick**
   - Each engine tick scans every pixel looking for steam (id=50)
   - For 1024x1024 world = 1M pixel lookups per tick
   - Should track steam pixels in a dedicated set maintained on writes (cf-terrain side)

9. **HeatField is stub-initialized at ambient and never updated**
   - The engine clones heat_field each tick for prev_heat_field but heat never changes
   - M19 atmospherics needs to drive per-cell heat updates
   - Phase transitions can't actually fire because temperature never changes

10. **PrecipitationCycle::world is hardcoded to AmbientWorld::Earth**
    - Scenarios should set this from their config (Vulcan ambient triggers acid rain)
    - Engine doesn't expose a way to set this currently

### LOW PRIORITY — designated future milestones (intentionally deferred)

- cf-server-ops, cf-server-persistence, cf-server-anti-cheat, cf-server-admin — M0 stubs for M9-M12
- Killcam playback (cf-killcam) — M8 stub for M41 polish
- cf-replay-scrub — M8 stub
- Splash damage routing (engine.rs line 4373 `let _ = hit.damage`) — M5.5

---

## Key Files / Locations

### Content (data-driven, editable without recompile)
- `game/content/materials/material_registry.json` — 89 materials
- `game/content/materials/reaction_registry.json` — 38 reactions ✓ NEW
- `game/content/materials/phase_registry.json` — 11 phase transitions ✓ NEW
- `game/content/materials/precipitation_config.json` — 9 tuning constants ✓ NEW
- `game/content/equipment/firearms/*.ron` — 12 weapon presets (mass/dims/durability)
- `game/content/equipment/weapons/{rpg_launcher_v1,tank_autocannon_t3}.ron` — tank rounds with bullet_mass/sharpness
- `game/content/scenarios/*.ron` — scenario manifests

### Source code (engine)
- `game/crates/cf-material/src/reactions.rs` — MaterialReaction + ReactionRegistry + loader
- `game/crates/cf-material/src/phase.rs` — PhaseTransition + PhaseRegistry + loader
- `game/crates/cf-material/src/precipitation.rs` — PrecipitationCycle + PrecipitationConfig + loader
- `game/crates/cf-material/src/kernel.rs` — MaterialKernel orchestrator (NOT WIRED TO ENGINE)
- `game/crates/cf-material-gpu/` — GPU compute pipeline + CPU fallback
- `game/crates/cf-control/src/engine.rs` — 29000-line engine (drive_tick at line 3650)
- `game/crates/cf-equipment/src/lib.rs` — RifleSpec + FiringProfile + bullet fields

### Specs
- `specs/done/M15B.md` — milestone spec (updated with emissions + chemistry + pressure)

---

## Build/Test Commands

```bash
# Workspace tests (4281 passing)
cd game && cargo test --workspace --no-fail-fast

# GPU-feature tests (32 passing on dev machine with GPU)
cd game && cargo test -p cf-material-gpu --features gpu

# M15 CA burst bench (currently 56ms p99 single-threaded)
cd game && cargo run -p cf-bench --release -- m15-ca-burst --ticks 200 --seed 42

# M15B GPU vs CPU bench
cd game && cargo run -p cf-bench --release -- m15b-gpu-vs-cpu --ticks 200 --seed 42

# Dump hardcoded registries to JSON for inspection
cd game && cargo run -p cf-material --example dump_registries reactions
cd game && cargo run -p cf-material --example dump_registries phase

# Clean target dir if disk full (it can grow to 143GB)
cd game && cargo clean
```

---

## How to resume this work

1. Read this ledger top-to-bottom
2. Run `cd game && cargo test --workspace 2>&1 | grep "test result" | awk ...` to confirm baseline pass count
3. Pick the highest-priority pending item
4. Update this ledger AS YOU GO — append to Session N, update Pending list
5. Commit each logical chunk with clear message
6. Push ONLY when explicitly asked by user

---

## Determinism Contract — DO NOT BREAK

- Every change must keep 4281 tests passing
- Material kernel must produce byte-identical sim_checksum per tick
- Adding parallelism requires either:
  - Collect-then-apply pattern with deterministic merge order
  - 4-color chunk phase pattern (chunks in same phase color don't share writes)
- No `thread_rng`, no `f64` in sim crates per AGENTS.md
- All RNG draws come from seeded `cf_sim_core::Rng`
