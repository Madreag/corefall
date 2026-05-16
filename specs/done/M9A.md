# M9A — Tier 1 SVG Asset Pipeline Bootstrap

## Status

`done` (re-sealed 2026-05-15 with expanded composer + manifest count)

## Closure state (2026-05-15)

Initial bake at commit `67a710a` produced 4735 entries. Subsequent polish-pass commits added enriched composers + new categories. As of 2026-05-15 commit b8c53e1, the ledger holds **6428 fresh entries** across 23 composers:

| Composer | Dispatch | Ledger Entries | Output Dir |
|---|---|---|---|
| `_compose_weapon` | WeaponSprite | 210 | weapons/ |
| `_compose_actor` | ActorSprite | 3080 | actors/ |
| `_compose_vehicle` | VehicleSprite | 18 | vehicles/ |
| `_compose_chassis` | ChassisSprite | 40 | chassis/ |
| `_compose_base_module` | BaseModuleSprite | 240 | base_modules/ |
| `_compose_ui_icon` | UiIcon | 640 | ui_icons/ |
| `_compose_material` | MaterialSwatch | 50 | materials/ |
| `_compose_particle` | Particle | 120 | particles/ |
| `_compose_terrain_tile` | TerrainTile | 85 | terrain_tiles/ |
| `_compose_cosmetic_stub` | Cosmetic | 256 | cosmetics/ |
| `_compose_emblem` | FactionEmblem | 16 | faction_emblems/ |
| `_compose_overlay` | CaptureGridOverlay | 24 | capture_overlays/ |
| `_compose_shell_ui` | ShellUi | 178 | shell_ui/ |
| `_compose_banner` | Banner | 264 | banners/ |
| `_compose_hud_widget` | HudWidget | 400 | hud_widgets/ |
| `_compose_vfx_decal` | VfxDecal | 96 | vfx_decals/ |
| `_compose_animation_frame` | AnimationFrame | 144 | animation_frames/ |
| `_compose_portrait` | Portrait | 44 | portraits/ |
| `_compose_ui_screen` | UiScreen | 20 | ui_screens/ |
| `_compose_vfx_frame` | VfxFrame | 64 | vfx_frames/ |
| `_compose_loading_bg` | LoadingBg | 20 | loading_backgrounds/ |
| `_compose_boss_splash` | BossSplash | 23 | boss_splashes/ |
| `_compose_key_art` | KeyArt | 19 | key_art/ |

Plus 242 SFX WAVs (Audio_SFX / Tier1_LLM_Audio / M12A_sfx_v1) + 120 music WAVs (Audio_Music / Tier1_LLM_Audio / M37A_music_v1) = **6428 total ledger entries**.

cf-mod ledger verify --strict clean: `verify total=6428 fresh=6428 stale=0 drifted=0 missing=0 failed=0`.

cf-mod validate content/ clean: `scanned=85 pass=1 warn=84 fail=0`.

## Intent

**M9A is the Tier 1 visual-asset pipeline** — every roster entry (weapons / actors / chassis / vehicles / base modules / materials / UI icons / particles / terrain tiles / cosmetic-placeholder) has a visually-coherent SVG / geometric placeholder generated at build time via Python + cairo-svg + Pillow + LLM-prompted SVG shape generation. The game LOOKS intentional at every milestone from M9A forward — never "MS Paint", never literal colored rectangles, always faction-coherent stylized 2D art at the floor quality.

**Why before M10:** by the time M10 ships the replay viewer + death recap UI, every cause-chain template needs to reference an asset (weapon icon, actor portrait, faction emblem) — text-only templates are insufficient for the player-facing recap UX. M9A ships before M10 so the recap modal has icons to render.

**Why this is the BACKBONE:** the human stops authoring art FOREVER at M9A. Every pixel of art in the game from M9A through M49 is regenerable from prompts + seeds + palette JSONs via this pipeline (or its Tier 2 successor at M32A, or Tier 3 polish at M48A). Modders use the same pipeline. CI re-bakes the entire roster on demand. M48A polishes; M9A bootstraps.

M9A promise: **"every weapon, actor, vehicle, chassis, base module, UI icon, material swatch in the game is data-driven SVG that regenerates byte-identically on any machine — and looks like real art, not placeholder rectangles."**

## Player-facing behavior

After M9A, the player sees:

- **Faction-coherent stylized art** for every entity (8 factions × per-entity variants; each faction has a distinct palette + silhouette language)
- **64+ UI iconography variants** at 16/32/64/128/256 px (loadout slots, faction emblems, status pips, ACC-A glyphs, material affordance icons, weapon-type icons, action prompt glyphs)
- **Hand-drawn-look placeholders** with SVG-line variation + LLM-generated organic shapes (NOT raster rectangles)
- **Material swatches** with integrity-band variations (Pristine → Scratched → Cracked → Critical → Destroyed) per the 17 launch materials × 5 bands = 85 swatches
- **Per-stance actor frames** (idle / walking / running / crouching / prone / jetting / climbing) for 10 origins × 4 facing = 40 sprite sheets
- **Per-weapon side-view sprites** for 70+ weapons with muzzle-flash placeholder + magazine-attached state
- **Per-chassis silhouette templates** for 5 archetypes × 4 weight classes = 20 chassis silhouettes
- **Material overlay tints** for the 5-mode M3 overlay (integrity / pathability / mobility / hazard / build-repair)
- **Capture-grid screenshots** in run bundles now show coherent art (not blank tiles)
- **Modder authoring**: modders can author their own SVGs by editing palette JSONs + dropping into the pipeline

## Crates / modules touched

| Crate / dir | Status | What |
|---|---|---|
| `tools/asset_gen/` | NEW | Python tooling for SVG generation |
| `tools/asset_gen/build_placeholders.py` | NEW | Main entry point; orchestrator; reads palette JSONs + asset manifests; emits SVG + rendered PNG |
| `tools/asset_gen/llm_svg_prompter.py` | NEW | LLM (Claude/GPT-4/local Llama) generates SVG path strings from prompts; verifies via cairo render; falls back to procedural geometric primitives if LLM output invalid |
| `tools/asset_gen/palette_loader.py` | NEW | Loads per-faction / per-material / per-origin palette JSONs |
| `tools/asset_gen/style_enforcer.py` | NEW | Per-faction style consistency: verifies generated SVG follows faction silhouette language via shape descriptors |
| `tools/asset_gen/cairo_renderer.py` | NEW | SVG → PNG render at multiple sizes via cairo-svg + Pillow |
| `tools/asset_gen/normal_map_baker.py` | NEW | SVG depth approximation → normal map for cf-render-2d (Tier 1 quality; refined at M32A) |
| `tools/asset_gen/ledger_writer.py` | NEW | Writes entries via cf-asset-ledger CLI for every generated asset |
| `cf-mod` | MODIFY | Add `cf-mod asset-gen run` subcommand invoking the Python pipeline |
| `cf-render-2d` | MODIFY | Loads SVG → texture atlases at startup; per-asset texture-id; respects palette swaps for material overlay modes |
| `game/build.rs` | NEW (root) | Build-step hook: invokes `python3 tools/asset_gen/build_placeholders.py --check` if `.svg.template` or `palette.json` changed; regenerates only stale entries |

## Files

Tooling:
- `tools/asset_gen/build_placeholders.py` (NEW)
- `tools/asset_gen/llm_svg_prompter.py` (NEW)
- `tools/asset_gen/palette_loader.py` (NEW)
- `tools/asset_gen/style_enforcer.py` (NEW)
- `tools/asset_gen/cairo_renderer.py` (NEW)
- `tools/asset_gen/normal_map_baker.py` (NEW)
- `tools/asset_gen/ledger_writer.py` (NEW)
- `tools/asset_gen/asset_manifests/weapons.ron` (NEW: 70+ weapon entries)
- `tools/asset_gen/asset_manifests/actors.ron` (NEW: 44+ actor entries × 10 origins × stances)
- `tools/asset_gen/asset_manifests/vehicles.ron` (NEW: 18+ vehicle entries)
- `tools/asset_gen/asset_manifests/chassis.ron` (NEW: 5 archetypes × 4 weight classes)
- `tools/asset_gen/asset_manifests/base_modules.ron` (NEW: 60+ base objects)
- `tools/asset_gen/asset_manifests/ui_icons.ron` (NEW: 64+ UI icons)
- `tools/asset_gen/asset_manifests/materials.ron` (NEW: 17 materials × 5 integrity bands)
- `tools/asset_gen/asset_manifests/particles.ron` (NEW: 30+ impact + spark + smoke + ember)
- `tools/asset_gen/asset_manifests/terrain_tiles.ron` (NEW: per-material tile variations)
- `tools/asset_gen/asset_manifests/cosmetic_placeholders.ron` (NEW: per-faction cosmetic stub)
- `tools/asset_gen/palettes/factions/{hostile_corp,allied_resistance,marauder_tribes,religious_order,scientist_order,mercenary_guild,pirates,drone_collective}.palette.json` (NEW: 8 faction palettes)
- `tools/asset_gen/palettes/materials.palette.json` (NEW)
- `tools/asset_gen/palettes/origins/{human,android,robot,powered_organic,heavy_biomech,insectoid,crystalline,photosynthetic,aqueous,methane_breather}.palette.json` (NEW: 10 origin palettes)
- `tools/asset_gen/palettes/factions_emblems.palette.json` (NEW)
- `tools/asset_gen/style_descriptors/factions/*.style.json` (NEW: per-faction silhouette language descriptors)
- `tools/asset_gen/style_descriptors/origins/*.style.json` (NEW)

Build integration:
- `game/build.rs` (NEW: top-level build hook)
- `game/scripts/asset_audit.sh` (NEW: nightly CI script verifies all assets via cf-asset-ledger)
- `game/scripts/regen_all_assets.sh` (NEW: clean-checkout full re-bake)

Source:
- `game/crates/cf-render-2d/src/asset_loader.rs` (MODIFY: load SVG → texture atlas via resvg or cairo-rs)
- `game/crates/cf-render-2d/src/palette_swap.rs` (NEW: runtime palette swap for material overlay modes + faction color-shift)
- `game/crates/cf-mod/src/cli.rs` (MODIFY: add asset-gen subcommand)

Schemas:
- `tools/asset_gen/schemas/v1/asset_manifest.schema.json` (NEW)
- `tools/asset_gen/schemas/v1/palette.schema.json` (NEW)
- `tools/asset_gen/schemas/v1/style_descriptor.schema.json` (NEW)

## Pipeline algorithm

```text
For each asset_manifest entry:
  1. Lookup palette (per-faction / per-material / per-origin)
  2. Lookup style_descriptor (per-faction silhouette language)
  3. Compose prompt = manifest.prompt_template + palette + style + seed
  4. Call LLM with deterministic temperature=0 + seed
     → LLM produces SVG path string + simple shape primitives
  5. Validate SVG via cairo render attempt
     → If invalid: fallback to procedural geometric (rectangles + circles + lines via palette)
  6. Apply palette substitution (palette tokens → hex colors)
  7. Apply style enforcement (silhouette descriptor: width range, height range, key shape requirements)
  8. Render SVG → PNG at sizes [16, 32, 64, 128, 256] via cairo + Pillow
  9. Optional: bake normal map via Tier 1 depth approximation (per material)
  10. Write to content/assets/placeholders/<category>/<canonical_name>.{svg,png,_normal.png}
  11. Compute blake3 hash
  12. Call cf-asset-ledger add via Python subprocess
  13. Build manifest gets the asset_id back
```

## SVG style language per faction (LLM-prompted)

Each faction has a `style.json` descriptor that the LLM-prompter reads:

```json
{
  "faction_id": "hostile_corp",
  "silhouette_language": "blocky, industrial, riveted plates, sharp 90° angles, dark steel palette, glowing red accents on sensors",
  "preferred_palette_indices": [0, 1, 2, 5, 7],
  "shape_descriptors": {
    "weapon": "boxy receiver, rectangular magazine, picatinny rails, vertical foregrip",
    "actor": "wide-shoulder armor pauldrons, full-face helmet with red visor, exoskeleton joints visible",
    "vehicle": "tracked or 6-wheeled, riveted hull, smokestack exhaust, slanted armor plates",
    "base_module": "concrete + steel beams, industrial conduits, hazard stripe accents"
  }
}
```

```json
{
  "faction_id": "religious_order",
  "silhouette_language": "ornate, gothic, draped fabric, organic curves, gold + ivory + deep red palette, ceremonial inscriptions",
  "shape_descriptors": {
    "weapon": "engraved barrel, draped cloth wrapping, prayer-script etched receiver, golden trim",
    "actor": "long-coat silhouette, peaked hood, side-cape, religious icons on chest, golden gauntlets"
  }
}
```

LLM prompt template:

```
Generate an SVG path string for a {asset.kind} ({asset.canonical_name}) for faction {faction_id}.

Faction style: {style.silhouette_language}

Specific shape: {style.shape_descriptors[asset.kind]}

Palette (use these hex colors only): {palette.hex_list}

Constraints:
- Dimensions: {asset.dimensions}
- Total path count: max {asset.max_paths}
- Must be deterministic (no random)
- Side-profile (faces right by default; will be flipped via sprite-flip for facing-left)
- Output: ONLY the <svg>...</svg> XML; no commentary

Seed: {seed}
```

Output is parsed as XML; if parse fails → fallback to procedural geometric (composed of rectangles + ellipses + lines per palette).

## Content roster at M9A

| Category | Target count | Tier 1 SVG ships |
|---|---|---|
| UI icons | 64+ | ALL launch icons at 16/32/64/128/256 px |
| Weapons | 70+ | side-view + muzzle-flash variant + magazine-attached |
| Actors | 44+ × 10 origins | per-stance × per-facing = ~700 sprite sheets |
| Vehicles | 18+ | side-view + boarding-state variant |
| Chassis | 5 archetypes × 4 weight classes | silhouette templates per facing |
| Base modules | 60+ | side-view per module-state (Nominal / Degraded / Warning / Failed) |
| Material swatches | 17 × 5 integrity bands | 85 swatches |
| Particles | 30+ | impact / spark / smoke / ember / fluid splatter |
| Terrain tiles | 17 materials × variations | per-material tile sets (3-5 variants each) |
| Cosmetic placeholders | per-faction stub | stubs that M45A fills with Tier 2 |
| Faction emblems | 8 | full + simplified silhouette variants |
| Capture-grid overlays | per-bundle UI chrome | screenshot frames + watermarks for capture grids |

**Total launch assets:** ~5000 entries across all categories.

## Acceptance criteria

```gherkin
Scenario: tools/asset_gen/build_placeholders.py exists and runs
  Given a fresh checkout
  When `python3 tools/asset_gen/build_placeholders.py --check` runs
  Then exit code is 0
  And the script reports the count of stale + missing assets to regenerate

Scenario: Full re-bake from scratch on fresh checkout
  Given a fresh checkout with content/assets/ deleted
  When `python3 tools/asset_gen/build_placeholders.py --all` runs
  Then ~5000 SVG + PNG files are generated under content/assets/placeholders/
  And each has a corresponding ledger entry
  And `cf-mod ledger verify --strict --all` exits 0
  And the build completes in <10 minutes on a modern dev machine (single CPU; LLM API or local Llama 3 8B)

Scenario: 64+ UI icons at all sizes
  Given M9A closure
  Then content/assets/placeholders/ui_icons/ contains 64+ icons
  Each icon has variants at [16, 32, 64, 128, 256] px
  Total ui_icons = 64+ × 5 = 320+ PNG files + 64+ SVG sources

Scenario: 70+ weapon side-view sprites
  Given M9A closure
  Then 70+ weapons in content/assets/placeholders/weapons/
  Each with side-view + muzzle-flash variant + magazine-attached variant = 210+ SVGs
  Faction-coherent: each weapon's style matches its issuing faction

Scenario: Per-stance actor frames
  Given M9A closure
  Then 44+ actors × 10 origins × per-stance frames exist
  Stances: idle, walking, running, crouching, prone, jetting, climbing
  Per actor: 7 stances × 4 facing = 28 frames per actor
  Total actor frames: 44 × 10 × 28 = ~12,320 frames (NOT all generated at M9A — M9A covers TEMPLATE generation; per-origin variants at M9A only for human; other origins use procedural color-shift at M9A close)

Scenario: Material swatches with integrity bands
  Given the 17 launch materials (per M19)
  Then 17 × 5 = 85 swatches exist
  Each swatch is the material's base color at the integrity band's saturation (per M9 5-tier signature)
  And the material overlay 5-mode rendering (per M3) uses these swatches

Scenario: Faction silhouette language enforced
  Given two weapons: one from Hostile Corp + one from Religious Order
  When inspected side-by-side at 64 px
  Then they have visually distinct silhouettes
  And both follow their respective faction style.json descriptors
  And LLM-grader validates "is this weapon in faction style?" returns Accept for >95% of generated weapons

Scenario: Deterministic regen with same seed
  Given an SVG generated with seed=1234
  When regenerated on a different machine with seed=1234
  Then output is byte-identical (blake3 matches)
  (Deterministic: LLM with temperature=0 + same seed + cairo deterministic render)
  Exception: LLM-API non-determinism — pipeline freezes-then-stores; regen verifies against stored output

Scenario: Build-step hook regenerates only stale entries
  Given a developer edits a faction palette JSON
  When `cargo build` runs
  Then build.rs detects the palette change
  And invokes Python pipeline only for assets in that faction
  And only stale entries regenerate (not the full 5000)
  Total regen time: <60s for a single palette change

Scenario: LLM-invalid output falls back to procedural
  Given an LLM response that fails SVG parsing
  When build_placeholders.py processes it
  Then it logs "LLM output invalid for <asset>; falling back to procedural"
  And produces a procedural geometric SVG (rectangles + ellipses + lines per palette)
  And asset still ships (no missing asset)
  And the ledger entry's generator field reflects the fallback (`generator: {tool: "procedural_fallback"}`)

Scenario: Normal map baking
  Given a Tier 1 SVG sprite
  When normal_map_baker.py runs
  Then a _normal.png is produced at the same size
  And cf-render-2d uses it for lighting (per M48A Tier 3 lighting pass)
  Tier 1 quality is approximate; M32A bakes proper normal maps from depth

Scenario: All Tier 1 assets ledgered
  Given M9A closure
  Then content/asset_ledger/ledger.jsonl contains 5000+ entries
  Each entry has tier=Tier1_SVG, pipeline=M9A_svg_v1, prompt + seed + output_path + blake3
  All entries verify Fresh

Scenario: Modder authoring
  Given a mod author writes a custom faction palette JSON + style.json
  When they run `python3 tools/asset_gen/build_placeholders.py --mod my_faction_mod`
  Then assets for their faction generate via the same pipeline
  And entries register with category=Mod_Custom in the ledger
  Hot-reload supported via M33

Scenario: cf-render-2d loads SVG at startup
  Given M9A assets generated
  When cf-app starts
  Then SVG textures load via resvg or cairo-rs into Bevy texture atlases
  And per-asset texture-id available for sprite rendering
  Per-faction palette swap available at runtime (no re-bake needed for color variations)

Scenario: Per-tier readability gate
  Given M9A art at Tier 1 (placeholder) quality
  When a human reviewer sees a capture-grid screenshot from M9 reactor scenario
  Then assets look intentional (not "MS Paint"): faction palettes coherent + silhouettes recognizable
  And the player can identify weapons / actors / factions at-a-glance
  This is the "looks intentional, not placeholder" gate per the original art-pipeline brief

Scenario: Cosmetic placeholders stubbed
  Given 8 factions × per-actor cosmetic stubs
  Then content/assets/placeholders/cosmetics/ contains stub entries
  M45A fills these with Tier 2 ComfyUI production-quality variants

Scenario: Capture-grid overlay
  Given M4 run bundles produce capture-grid screenshots
  Then M9A ships a capture-grid frame overlay (logo + version + tick + seed)
  And cf-capture composites the overlay onto every grid screenshot
```

## Out of scope

- **Tier 2 production-quality ComfyUI art** — M32A (SDXL/Flux/AnimateDiff/ControlNet)
- **Animation frames beyond static stances** — M18A (walk cycles, hit reactions, death animations)
- **VFX particle textures beyond static sprites** — M24A (animated particle systems)
- **Cinematic VFX + lighting passes** — M48A Tier 3 polish
- **Per-faction LoRA training** — M32A (Tier 2 production)
- **Hand-tweaked Aseprite finalization** — M48A
- **Voice + audio assets** — M12A
- **Narrative content** — M25A
- **Animation rigging (Spine bones)** — M48A
- **Tier 3 color grading** — M48A
- **GPU-based SVG rendering** — Tier 1 uses CPU cairo; GPU baking optional at M32A
- **AI-generated 3D models** — Corefall is 2D side-view per DR-019; no 3D scope
- **Asset CDN / streaming downloads** — Steam Workshop handles for mods; M9A assets ship in-base-game
- **License compliance per AI-generated asset** — DR-053 handles via ledger metadata; M9A just records

## Dependencies

- **M4A asset ledger (must close OR concurrent)** — M9A writes to the cf-asset-ledger registered by M4A
- **M0 engine bootstrap (closed)** — cargo + build.rs + Python toolchain available
- **No gameplay dependencies** — M9A is pure production-track; ships in BP3 alongside M9

## Notes for the implementer

### Architecture rules

- **Python pipeline, not Rust pipeline**: Python ecosystem has cairo-svg + Pillow + LLM SDKs + better string templating. Rust core engine consumes the OUTPUT (SVGs + PNGs); Python tooling is a build-time dependency.
- **Build determinism**: pipeline reproduces byte-identical output given pinned LLM model + seed + palette. Cairo is deterministic; PIL is deterministic; LLM with temp=0 + seed is mostly deterministic; for non-determinism handle via freeze-then-store.
- **LLM provider abstraction**: support Claude / GPT-4 / local Llama / Mistral via adapter pattern. Default: local Llama 3 8B (no API cost, fully deterministic, offline-friendly).
- **Palette JSON is source of truth**: every color comes from a palette JSON; no hardcoded hex in SVG templates. Faction color shift = swap palette JSON.
- **Style descriptors are LLM prompts**: `style.json` files are crafted prompts that LLM reads; modders edit these to author new factions.

### Style enforcement

- Generated SVG is validated: width/height in bounds, path count within limits, only palette colors used
- LLM-grader pass: a separate LLM call reads the generated SVG + style descriptor and answers "does this match the faction style?" — score < 0.7 triggers regen with adjusted prompt
- Auto-fix loop: max 3 regen attempts per asset; final attempt accepts whatever the LLM produces with a warning

### Faction silhouette consistency

The 8 launch factions need DISTINCT visual identities. The style.json descriptors enforce:

| Faction | Silhouette language |
|---|---|
| Hostile Corp | Blocky, industrial, rivets, sharp angles, dark steel + red accents |
| Allied Resistance | Mismatched scavenged gear, improvised armor, rust + olive + tan palette |
| Marauder Tribes | Bone + leather + tribal patterns, asymmetric, scavenged tech |
| Religious Order | Ornate, gothic, draped fabric, gold + ivory + deep red |
| Scientist Order | Clean lines, lab coats, holographic visors, white + cyan + chrome |
| Mercenary Guild | Tactical, modular, swappable plates, black + gold + neutral grays |
| Pirates | Asymmetric, scavenged, sea-themed, flag motifs, blue + red + tarnished gold |
| Drone Collective | Geometric, swarm-aesthetic, blue glow accents, hard edges, no flesh |

Each origin (human / android / robot / etc.) gets a style.json too — robots look chrome+joints; insectoids look chitin+exoskeleton; crystalline looks lattice+facets.

### Decision-record alignment

- **DR-019 (visual direction)**: M9A ships the Tier 1 readability + faction-coherent floor. M12 comic-noir styling layers on top. M32A Tier 2 + M48A Tier 3 produce final art.
- **DR-044 (audiovisual production pipeline)**: M9A is the first tier; M32A is the second; M48A is the third.
- **DR-045 (launch roster)**: M9A covers placeholder for the FULL launch roster.
- **DR-053 (asset ledger)**: M9A writes every entry to cf-asset-ledger; closes the DR jointly with M4A.
- **DR-006 (mod parity)**: mods author via same Python pipeline.
- **DR-024 (native engine stack)**: cf-render-2d loads SVG at startup; runtime is pure Rust.

### Pitfalls

- **LLM non-determinism across providers**: Claude vs GPT-4 vs Llama produce different outputs. Pipeline freezes-then-stores per asset; regen verifies against stored output, not re-call to LLM. CI verifies the stored output is reproducible from the pipeline.
- **Faction style drift over time**: as new factions ship in mods, the silhouette-language vocabulary needs versioning. Per `style.json` has a `version` field; old assets retain old version.
- **Cairo rendering platform differences**: Linux/macOS/Windows cairo versions may render subtly differently. Pin cairo-svg + Pillow versions in `requirements.txt`; CI matrix verifies cross-platform byte-identical output.
- **Palette JSON in git LFS**: small text files; commit normally
- **Generated PNG/SVG in git**: ~50-100 MB at full launch roster; use git LFS for content/assets/placeholders/ directory
- **Mod assets shipped via Workshop**: mods that author their own factions ship their style.json + palette + generated assets; pipeline runs at install-time on Workshop subscriber's machine? Or at mod build time? Decision: build-time (mod author runs the pipeline; ships generated assets). Steam Workshop hosts the generated assets, not the source.

### Closure procedure

1. Reference bundle: `prototype_runs/native/m9a_<UTC>_<hash>/` (proves M9 scenario screenshots use real M9A art for all entities)
2. Self-play sweep rows: `m9a_pipeline_full_bake`, `m9a_faction_silhouette_coherence`, `m9a_palette_swap`, `m9a_modder_authoring`, `m9a_build_step_hook`, `m9a_universal_done_criteria`. All PASS.
3. Update DR-053 (jointly with M4A) → CLOSED-DIRECTION-WITH-EVIDENCE.
4. Move M9A → done/.
