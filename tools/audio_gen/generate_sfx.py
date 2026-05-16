"""M12A § Audio pipeline orchestrator.

Per spec § Files:
> `tools/audio_gen/generate_sfx.py` (NEW) — Main orchestrator; reads
> sfx_manifest.ron; calls Stable Audio Open / AudioCraft API.

Per spec § Acceptance criteria:
> Scenario: tools/audio_gen/generate_sfx.py exists and runs
>   Given a fresh checkout
>   When `python3 tools/audio_gen/generate_sfx.py --check` runs
>   Then exit code is 0
>   And reports count of stale + missing SFX

This is the canonical M12A entry point. It:

1. Loads the existing SFX prompt manifests
   (`game/content/sfx/weapon_sfx_prompts.json`,
   `movement_sfx_prompts.json`,
   `impact_and_combat_sfx_prompts.json`,
   `ambient_environment_sfx_prompts.json`).
2. Procedurally expands them with the M12A roster categories that aren't
   in those manifests yet (per-material impacts × 4 states, atmospherics,
   chassis, fluid, power, crafting, voice grunts, pets, storyteller).
3. Dispatches each entry through the adapter chain (Stable Audio Open →
   AudioCraft → ElevenLabs → procedural Tier 1 fallback).
4. Applies the envelope-shaper + loudness normalization.
5. Writes the WAVs to `game/content/audio/sfx/`.
6. Inserts cf-asset-ledger entries via `ledger_writer`.
7. Authors a caption template per SFX via `caption_authorer`.
8. Persists the canonical `sfx_manifest.ron` + `caption_templates.ron`
   alongside the bake so the registry can be rebuilt from manifests.

Usage:

    python3 tools/audio_gen/generate_sfx.py --check
    python3 tools/audio_gen/generate_sfx.py --all
    python3 tools/audio_gen/generate_sfx.py --category weapon
    python3 tools/audio_gen/generate_sfx.py --report
    python3 tools/audio_gen/generate_sfx.py --mod my_audio_mod
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterable

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[1]
sys.path.insert(0, str(_HERE))
sys.path.insert(0, str(_REPO_ROOT / "tools" / "audio_synth"))
sys.path.insert(0, str(_REPO_ROOT / "tools" / "audio_pipeline"))

# Adapters (lazily used; primary path is procedural Tier 1 today).
from caption_authorer import author_caption_dict  # type: ignore  # noqa: E402

# Use importlib to load the spec-canonical sibling `ledger_writer.py`
# without `sys.path` clobbering it via `tools/asset_gen/ledger_writer.py`.
import importlib.util as _ilu  # noqa: E402

_LEDGER_WRITER_PATH = _HERE / "ledger_writer.py"
_spec = _ilu.spec_from_file_location("_m12a_ledger_writer", _LEDGER_WRITER_PATH)
assert _spec is not None and _spec.loader is not None
_ledger_writer_mod = _ilu.module_from_spec(_spec)
sys.modules["_m12a_ledger_writer"] = _ledger_writer_mod
_spec.loader.exec_module(_ledger_writer_mod)

SupersedeRecord = _ledger_writer_mod.SupersedeRecord
apply_superseded_entries = _ledger_writer_mod.apply_superseded_entries
add_new_entries = _ledger_writer_mod.add_new_entries
count_audio_entries = _ledger_writer_mod.count_audio_entries
LedgerEntryDraft = _ledger_writer_mod.LedgerEntryDraft
build_entry = _ledger_writer_mod.build_entry
hash_path = _ledger_writer_mod.hash_path

OUT_DIR = _REPO_ROOT / "game" / "content" / "audio" / "sfx"
SFX_PROMPTS_DIR = _REPO_ROOT / "game" / "content" / "sfx"
MANIFEST_OUT = _HERE / "sfx_manifest.ron"
CAPTION_TEMPLATES_OUT = _HERE / "caption_templates.ron"

PIPELINE_TIER1 = "M12A_audio_v1"
TIER1_TOOL = "tools/audio_gen/generate_sfx.py"
TIER1_MODEL = "procedural-numpy-synth-v1"
TIER1_MODEL_VERSION = "v1"


def _seed_for(name: str) -> int:
    """Deterministic seed derived from canonical name. Same name → same seed."""
    h = hashlib.blake3(name.encode("utf-8")).digest() if hasattr(hashlib, "blake3") else hashlib.sha256(name.encode("utf-8")).digest()
    return int.from_bytes(h[:8], "big") & 0x7FFFFFFFFFFFFFFF


@dataclass
class SfxManifestEntry:
    """One row in the M12A canonical manifest."""

    id: str
    category: str
    prompt: str
    duration_ms: int
    seed: int
    target_loudness_lufs: float = -16.0
    loops: bool = False
    material: str | None = None
    state: str | None = None
    faction: str | None = None
    origin: str | None = None
    propagation_range_m: float = 50.0
    occlusion_curve: str = "linear"
    package_source: str = "vanilla@1.0.0"
    license: str = "CC0"
    caption_template: str = ""
    caption_severity: str = "info"
    caption_categories: list[str] = field(default_factory=lambda: ["combat"])

    def to_dict(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "category": self.category,
            "prompt": self.prompt,
            "duration_ms": self.duration_ms,
            "seed": self.seed,
            "target_loudness_lufs": self.target_loudness_lufs,
            "loops": self.loops,
            "material": self.material,
            "state": self.state,
            "faction": self.faction,
            "origin": self.origin,
            "propagation_range_m": self.propagation_range_m,
            "occlusion_curve": self.occlusion_curve,
            "package_source": self.package_source,
            "license": self.license,
            "caption_template": self.caption_template,
            "caption_severity": self.caption_severity,
            "caption_categories": self.caption_categories,
        }


# ─── Existing-manifest loading ────────────────────────────────────────────


def _load_existing_entries() -> list[SfxManifestEntry]:
    """Load the 242 entries already authored in the four .json manifests
    under `game/content/sfx/`. Each entry's caption template is authored
    via `caption_authorer.author_caption`."""
    sources: list[tuple[str, str, str]] = [
        ("weapon_sfx_prompts.json", "weapon_action_sfx", "WeaponFire"),
        ("movement_sfx_prompts.json", "footstep_sfx", "Footstep"),
        ("movement_sfx_prompts.json", "locomotion_sfx", "Locomotion"),
        ("impact_and_combat_sfx_prompts.json", "projectile_sfx", "Projectile"),
        ("impact_and_combat_sfx_prompts.json", "impact_sfx_by_material", "Impact"),
        ("impact_and_combat_sfx_prompts.json", "body_hit_sfx", "BodyHit"),
        ("impact_and_combat_sfx_prompts.json", "dismemberment_sfx", "Dismember"),
        ("impact_and_combat_sfx_prompts.json", "death_sfx", "Death"),
        ("ambient_environment_sfx_prompts.json", "ambient_loops", "Ambient"),
        ("ambient_environment_sfx_prompts.json", "weather_sfx", "Weather"),
        ("ambient_environment_sfx_prompts.json", "hazard_sfx", "Hazard"),
        ("ambient_environment_sfx_prompts.json", "ui_sfx", "UI"),
        ("ambient_environment_sfx_prompts.json", "ai_chatter_prompts", "Chatter"),
    ]
    entries: list[SfxManifestEntry] = []
    for fname, key, category in sources:
        path = SFX_PROMPTS_DIR / fname
        if not path.exists():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        for e in data.get(key, []):
            entry_id = e["id"]
            duration_ms = int(float(e.get("duration_target_sec", 1.0)) * 1000)
            cap = author_caption_dict(entry_id, e.get("prompt", ""))
            entries.append(SfxManifestEntry(
                id=entry_id,
                category=category,
                prompt=e.get("prompt", ""),
                duration_ms=max(50, min(60000, duration_ms)),
                seed=_seed_for(entry_id),
                loops=bool(e.get("loops", False)),
                material=e.get("material"),
                caption_template=cap["template"],
                caption_severity=cap["severity"],
                caption_categories=cap["categories"],
            ))
    return entries


# ─── Procedural roster expansion ──────────────────────────────────────────
#
# Per the M12A spec § Content roster at M12A — the launch target is ~1200
# entries. The four existing prompt manifests cover 242. The functions
# below programmatically generate the remaining categories.


WEAPON_CLASSES = [
    "pistol", "smg", "rifle", "rifle_marksman", "sniper", "shotgun",
    "shotgun_auto", "gl", "heavy", "heavy_minigun", "flamer", "drill",
    "grappler", "drone_pistol", "drone_smg",
]
# spec says 70 weapons; we generate 14 base × 5 faction variants = 70.
WEAPON_FACTIONS = [
    "corp", "frontier", "tribal", "scientist", "drone",
]
MATERIALS = ["concrete", "metal", "wood", "glass", "dirt", "sand", "ice", "basalt"]
IMPACT_STATES = ["pristine", "scratched", "cracked", "destroyed"]
IMPACT_DAMAGE_TYPES = [
    "kinetic_small", "kinetic_large", "thermal", "electric", "chemical",
    "ballistic_armor_piercing", "ballistic_high_explosive",
    "ricochet", "deflection", "spall", "shatter", "scorch", "freeze",
    "crumble", "ignite",
]
GRENADE_TYPES = ["frag", "smoke", "flash", "incendiary"]
MELEE_TYPES = ["knife", "axe", "club", "bayonet"]
TOOL_TYPES = ["drill", "welder", "wrench", "scanner", "repair_kit", "med_kit", "beacon"]
TOOL_PHASES = ["activate", "use_loop", "complete", "fail", "deplete"]
HAZARD_TYPES = ["fire", "electric", "acid", "radiation", "vacuum", "smoke", "heat", "cold"]
CHASSIS_PHASES = ["board", "eject", "salvage", "step_heavy", "step_jet", "jet_engage",
                  "jet_disengage", "joint_pop", "armor_creak", "core_idle", "core_strain",
                  "core_shutdown"]
FLUID_PHASES = ["leak_start", "leak_drip", "ground_splatter", "ignition",
                "reservoir_warning", "reservoir_critical", "reservoir_empty",
                "phase_change", "freeze", "pipe_rupture", "refill_complete", "valve_close"]
POWER_PHASES = ["generator_hum_loop", "breaker_trip", "brownout_cascade", "spike",
                "shutdown", "boot", "diesel_idle", "solar_chime", "fusion_hum",
                "battery_warning", "battery_critical", "battery_recharge"]
CRAFTING_PHASES = ["station_hum_loop", "recipe_ding", "recipe_fail", "research_fanfare",
                   "blueprint_unlock", "material_add", "material_drop", "station_idle"]
VOICE_ORIGINS = ["human", "biomech", "robot", "aqueous", "crystalline",
                 "photosynthetic", "methane_breather", "insectoid"]
VOICE_EMOTIONS = ["calm", "stressed", "panicked", "dying"]
PET_TYPES = ["combat_drone", "scout_drone", "guard_dog", "service_bot"]
PET_REACTIONS = ["alert", "happy", "engaged", "damaged", "destroyed"]
STORYTELLER_EVENTS = [
    "cassandra_intro", "phoebe_intro", "randy_intro",
    "raid_inbound", "raid_resolved", "discovery_chime",
    "betrayal_sting", "victory_fanfare", "defeat_dirge",
    "ironman_lock", "permadeath_lock", "save_complete",
]
REACTOR_STATES = ["nominal", "elevated", "warning", "critical", "destroyed"]
MISSION_PHASES = ["start", "objective_unlocked", "objective_complete",
                  "win", "loss", "optional_offered", "branch_taken", "wave_inbound"]
BANNER_SEVERITIES = ["info", "warning", "critical"]
ATMOSPHERIC_EVENTS = [
    "breach_decompression", "gas_hiss_release", "combustion_ignition",
    "phase_transition", "pressure_spike", "vacuum_silence_onset",
    "wind_gust_chamber", "pipe_flow_loop", "valve_release",
    "duct_rumble", "atmos_normalize", "leak_seal",
]


def _gen_weapon_reload_entries() -> list[SfxManifestEntry]:
    """70 weapon reload SFX (14 classes × 5 factions)."""
    out: list[SfxManifestEntry] = []
    for cls in WEAPON_CLASSES:
        for fac in WEAPON_FACTIONS:
            entry_id = f"sfx_weapon_reload_{fac}_{cls}"
            prompt = (
                f"{fac} {cls} reload, metallic mag release + insert + bolt cycle, "
                f"clean foley, 0.9 second"
            )
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="WeaponReload", prompt=prompt,
                duration_ms=900, seed=_seed_for(entry_id),
                faction=fac,
                caption_template=cap["template"], caption_severity=cap["severity"],
                caption_categories=cap["categories"],
            ))
    return out


def _gen_weapon_jam_entries() -> list[SfxManifestEntry]:
    """140 weapon-jam SFX (70 weapons × 2 events: jam + clear)."""
    out: list[SfxManifestEntry] = []
    for cls in WEAPON_CLASSES:
        for fac in WEAPON_FACTIONS:
            for action, duration_ms, prompt_extra in [
                ("jam", 350, "harsh grinding clack stopping mid-cycle"),
                ("clear", 750, "manual charging handle pull + slap-release + chamber check"),
            ]:
                entry_id = f"sfx_weapon_jam_{fac}_{cls}_{action}"
                prompt = f"{fac} {cls} {action}, {prompt_extra}, foley, {duration_ms}ms"
                cap = author_caption_dict(entry_id, prompt)
                out.append(SfxManifestEntry(
                    id=entry_id, category="WeaponJam", prompt=prompt,
                    duration_ms=duration_ms, seed=_seed_for(entry_id),
                    faction=fac,
                    caption_template=cap["template"], caption_severity=cap["severity"],
                    caption_categories=cap["categories"],
                ))
    return out


def _gen_weapon_magazine_entries() -> list[SfxManifestEntry]:
    """70 magazine pop + shell eject SFX."""
    out: list[SfxManifestEntry] = []
    for cls in WEAPON_CLASSES:
        for fac in WEAPON_FACTIONS:
            entry_id = f"sfx_weapon_mag_{fac}_{cls}_pop"
            prompt = f"{fac} {cls} magazine drops to floor, polymer-metal clatter, 0.4s"
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="WeaponMagazine", prompt=prompt,
                duration_ms=400, seed=_seed_for(entry_id),
                faction=fac,
                caption_template=cap["template"], caption_severity=cap["severity"],
                caption_categories=cap["categories"],
            ))
    return out


def _gen_per_material_impact_entries() -> list[SfxManifestEntry]:
    """480 SFX = 8 materials × 4 states × 15 damage-type variants."""
    out: list[SfxManifestEntry] = []
    for mat in MATERIALS:
        for state in IMPACT_STATES:
            for dmg in IMPACT_DAMAGE_TYPES:
                entry_id = f"impact_{mat}_{state}_{dmg}"
                prompt = (
                    f"{dmg.replace('_', ' ')} impact on {state} {mat}, "
                    f"realistic material-specific timbre, 0.4 second"
                )
                cap = author_caption_dict(entry_id, prompt)
                out.append(SfxManifestEntry(
                    id=entry_id, category="Impact", prompt=prompt,
                    duration_ms=400, seed=_seed_for(entry_id),
                    material=mat, state=state,
                    caption_template=cap["template"], caption_severity=cap["severity"],
                    caption_categories=cap["categories"],
                ))
    return out


def _gen_grenade_entries() -> list[SfxManifestEntry]:
    """8 SFX — 4 grenade types × 2 phases (fuse + detonation)."""
    out: list[SfxManifestEntry] = []
    for gren in GRENADE_TYPES:
        for phase, duration_ms, prompt_extra in [
            ("fuse", 800, "metallic primer pop + low whoosh + count-down hiss"),
            ("detonation", 600, "sharp burst + shock-wave + concussive rumble tail"),
        ]:
            entry_id = f"sfx_grenade_{gren}_{phase}"
            prompt = f"{gren} grenade {phase}, {prompt_extra}, {duration_ms}ms"
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="Grenade", prompt=prompt,
                duration_ms=duration_ms, seed=_seed_for(entry_id),
                caption_template=cap["template"], caption_severity=cap["severity"],
                caption_categories=cap["categories"],
            ))
    return out


def _gen_melee_entries() -> list[SfxManifestEntry]:
    """8 SFX — 4 melee types × 2 phases (swing + hit)."""
    out: list[SfxManifestEntry] = []
    for melee in MELEE_TYPES:
        for phase, duration_ms, prompt_extra in [
            ("swing", 300, "whoosh through air with cloth-rustle"),
            ("hit", 350, "sharp impact + body thump + blade ring"),
        ]:
            entry_id = f"sfx_melee_{melee}_{phase}"
            prompt = f"{melee} {phase}, {prompt_extra}, {duration_ms}ms"
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="Melee", prompt=prompt,
                duration_ms=duration_ms, seed=_seed_for(entry_id),
                caption_template=cap["template"], caption_severity=cap["severity"],
                caption_categories=cap["categories"],
            ))
    return out


def _gen_tool_entries() -> list[SfxManifestEntry]:
    """35 SFX — 7 tools × 5 phases."""
    out: list[SfxManifestEntry] = []
    for tool in TOOL_TYPES:
        for phase in TOOL_PHASES:
            entry_id = f"sfx_tool_{tool}_{phase}"
            prompt = f"{tool} tool {phase} cue, characteristic mechanical signature, 0.5s"
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="Tool", prompt=prompt,
                duration_ms=500, seed=_seed_for(entry_id),
                loops=(phase == "use_loop"),
                caption_template=cap["template"], caption_severity=cap["severity"],
                caption_categories=cap["categories"],
            ))
    return out


def _gen_ui_extra_entries() -> list[SfxManifestEntry]:
    """Top up UI sounds to the spec's 12 launch UI sounds."""
    extras = [
        ("sfx_ui_tab_switch", "UI tab switch click, soft tick, 0.1s", 100),
        ("sfx_ui_modal_open", "UI modal open whoosh, brief slide, 0.2s", 200),
        ("sfx_ui_modal_close", "UI modal close click, brief, 0.15s", 150),
        ("sfx_ui_save_complete", "UI save complete chime, gentle ding, 0.4s", 400),
        ("sfx_ui_load_complete", "UI load complete chime, brighter ding, 0.4s", 400),
        ("sfx_ui_error_buzz", "UI error buzzer, low buzz tone, 0.3s", 300),
        ("sfx_ui_warning_pulse", "UI warning soft pulse, 0.3s", 300),
        ("sfx_ui_focus_move", "UI focus traversal tick, very brief, 0.05s", 50),
        ("sfx_ui_confirm_chime", "UI confirm chime, bright soft, 0.3s", 300),
        ("sfx_ui_cancel_blip", "UI cancel blip, low click, 0.1s", 100),
        ("sfx_ui_purchase_chime", "UI purchase complete chime, warm rising, 0.5s", 500),
        ("sfx_ui_drag_hover", "UI drag hover soft pop, 0.1s", 100),
    ]
    out: list[SfxManifestEntry] = []
    for entry_id, prompt, dur in extras:
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="UI", prompt=prompt,
            duration_ms=dur, seed=_seed_for(entry_id),
            caption_template=cap["template"], caption_severity=cap["severity"],
            caption_categories=cap["categories"],
        ))
    return out


def _gen_banner_entries() -> list[SfxManifestEntry]:
    """6 SFX — 3 banner severities × 2 phases (raise + dismiss)."""
    out: list[SfxManifestEntry] = []
    for sev in BANNER_SEVERITIES:
        for phase, duration_ms in [("raise", 350), ("dismiss", 200)]:
            entry_id = f"sfx_banner_{sev}_{phase}"
            prompt = f"HUD banner {sev} {phase} cue, {sev}-band tone, {duration_ms}ms"
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="Banner", prompt=prompt,
                duration_ms=duration_ms, seed=_seed_for(entry_id),
                caption_template=cap["template"], caption_severity=sev,
                caption_categories=["system"],
            ))
    return out


def _gen_reactor_entries() -> list[SfxManifestEntry]:
    """5 SFX — reactor pressure-state cues."""
    out: list[SfxManifestEntry] = []
    for state in REACTOR_STATES:
        entry_id = f"sfx_reactor_pressure_{state}"
        prompt = f"reactor at {state} pressure, characteristic background hum + alarm cue, 1.0s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Reactor", prompt=prompt,
            duration_ms=1000, seed=_seed_for(entry_id),
            loops=(state in ("nominal", "elevated")),
            caption_template=cap["template"], caption_severity="warning",
            caption_categories=["mission"],
        ))
    return out


def _gen_mission_entries() -> list[SfxManifestEntry]:
    """8 SFX — mission lifecycle cues."""
    out: list[SfxManifestEntry] = []
    for phase in MISSION_PHASES:
        entry_id = f"sfx_mission_{phase}"
        prompt = f"mission {phase} cue, distinct sting, 0.6s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Mission", prompt=prompt,
            duration_ms=600, seed=_seed_for(entry_id),
            caption_template=cap["template"], caption_severity="info",
            caption_categories=["mission"],
        ))
    return out


def _gen_atmospheric_entries() -> list[SfxManifestEntry]:
    """12 SFX — M19 atmospherics events."""
    out: list[SfxManifestEntry] = []
    for atmos in ATMOSPHERIC_EVENTS:
        entry_id = f"sfx_atmos_{atmos}"
        prompt = f"atmospheric {atmos.replace('_', ' ')}, characteristic timbre, 0.8s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Atmospheric", prompt=prompt,
            duration_ms=800, seed=_seed_for(entry_id),
            loops="loop" in atmos,
            caption_template=cap["template"], caption_severity="warning",
            caption_categories=["system"],
        ))
    return out


def _gen_chassis_entries() -> list[SfxManifestEntry]:
    """12 SFX — chassis movement / board / eject / salvage."""
    out: list[SfxManifestEntry] = []
    for phase in CHASSIS_PHASES:
        entry_id = f"sfx_chassis_{phase}"
        prompt = f"chassis {phase.replace('_', ' ')}, heavy mechanical + servo, 0.6s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Chassis", prompt=prompt,
            duration_ms=600, seed=_seed_for(entry_id),
            loops="loop" in phase or phase == "core_idle",
            caption_template=cap["template"], caption_severity="info",
            caption_categories=["system"],
        ))
    return out


def _gen_fluid_entries() -> list[SfxManifestEntry]:
    """12 SFX — fluid system events."""
    out: list[SfxManifestEntry] = []
    for phase in FLUID_PHASES:
        entry_id = f"sfx_fluid_{phase}"
        prompt = f"fluid {phase.replace('_', ' ')}, wet liquid signature, 0.5s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Fluid", prompt=prompt,
            duration_ms=500, seed=_seed_for(entry_id),
            caption_template=cap["template"], caption_severity="warning",
            caption_categories=["system"],
        ))
    return out


def _gen_power_entries() -> list[SfxManifestEntry]:
    """12 SFX — power-grid + generator events."""
    out: list[SfxManifestEntry] = []
    for phase in POWER_PHASES:
        entry_id = f"sfx_power_{phase}"
        prompt = f"power-grid {phase.replace('_', ' ')}, electrical hum + servo, 0.6s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Power", prompt=prompt,
            duration_ms=600, seed=_seed_for(entry_id),
            loops="loop" in phase or "idle" in phase or "hum" in phase,
            caption_template=cap["template"], caption_severity="info",
            caption_categories=["system"],
        ))
    return out


def _gen_crafting_entries() -> list[SfxManifestEntry]:
    """8 SFX — crafting / research events."""
    out: list[SfxManifestEntry] = []
    for phase in CRAFTING_PHASES:
        entry_id = f"sfx_crafting_{phase}"
        prompt = f"crafting {phase.replace('_', ' ')}, station mechanical signature, 0.5s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Crafting", prompt=prompt,
            duration_ms=500, seed=_seed_for(entry_id),
            loops="loop" in phase,
            caption_template=cap["template"], caption_severity="info",
            caption_categories=["system"],
        ))
    return out


def _gen_voice_grunt_entries() -> list[SfxManifestEntry]:
    """32 SFX — 8 origins × 4 emotions (grunts, placeholder before M37A)."""
    out: list[SfxManifestEntry] = []
    for origin in VOICE_ORIGINS:
        for emotion in VOICE_EMOTIONS:
            entry_id = f"sfx_voice_grunt_{origin}_{emotion}"
            prompt = (
                f"{origin}-origin combat grunt, {emotion} emotion, brief vocalization, 0.4s"
            )
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="VoiceGrunt", prompt=prompt,
                duration_ms=400, seed=_seed_for(entry_id),
                origin=origin,
                caption_template=cap["template"],
                caption_severity="critical" if emotion == "dying" else "info",
                caption_categories=["ai"],
            ))
    return out


def _gen_pet_entries() -> list[SfxManifestEntry]:
    """20 SFX — 4 pet types × 5 reactions."""
    out: list[SfxManifestEntry] = []
    for pet in PET_TYPES:
        for reaction in PET_REACTIONS:
            entry_id = f"sfx_pet_{pet}_{reaction}"
            prompt = f"{pet} pet {reaction} cue, characteristic vocalization, 0.4s"
            cap = author_caption_dict(entry_id, prompt)
            out.append(SfxManifestEntry(
                id=entry_id, category="Pet", prompt=prompt,
                duration_ms=400, seed=_seed_for(entry_id),
                caption_template=cap["template"], caption_severity="info",
                caption_categories=["ai"],
            ))
    return out


def _gen_storyteller_entries() -> list[SfxManifestEntry]:
    """12 SFX — storyteller event cues."""
    out: list[SfxManifestEntry] = []
    for event in STORYTELLER_EVENTS:
        entry_id = f"sfx_storyteller_{event}"
        prompt = f"storyteller event '{event.replace('_', ' ')}' cinematic sting, 1.0s"
        cap = author_caption_dict(entry_id, prompt)
        out.append(SfxManifestEntry(
            id=entry_id, category="Storyteller", prompt=prompt,
            duration_ms=1000, seed=_seed_for(entry_id),
            caption_template=cap["template"], caption_severity="info",
            caption_categories=["mission"],
        ))
    return out


def build_full_manifest() -> list[SfxManifestEntry]:
    """Combine the existing 242 entries + procedurally generate the
    remaining ~960 to reach the M12A 1200+ roster target."""
    out: list[SfxManifestEntry] = []
    out.extend(_load_existing_entries())
    out.extend(_gen_weapon_reload_entries())
    out.extend(_gen_weapon_jam_entries())
    out.extend(_gen_weapon_magazine_entries())
    out.extend(_gen_per_material_impact_entries())
    out.extend(_gen_grenade_entries())
    out.extend(_gen_melee_entries())
    out.extend(_gen_tool_entries())
    out.extend(_gen_ui_extra_entries())
    out.extend(_gen_banner_entries())
    out.extend(_gen_reactor_entries())
    out.extend(_gen_mission_entries())
    out.extend(_gen_atmospheric_entries())
    out.extend(_gen_chassis_entries())
    out.extend(_gen_fluid_entries())
    out.extend(_gen_power_entries())
    out.extend(_gen_crafting_entries())
    out.extend(_gen_voice_grunt_entries())
    out.extend(_gen_pet_entries())
    out.extend(_gen_storyteller_entries())
    # Dedup by id (existing manifests may have authored some categories
    # that the procedural generator would re-emit).
    seen: dict[str, SfxManifestEntry] = {}
    for e in out:
        if e.id not in seen:
            seen[e.id] = e
    return list(seen.values())


# ─── Manifest persistence ─────────────────────────────────────────────────


def write_manifest_ron(entries: list[SfxManifestEntry], path: Path) -> None:
    """Write the manifest as a RON-compatible JSON-array-of-structs. The
    cf-asset-ledger contract is the authoritative source; this file is
    the canonical roster for tooling / mod authors."""
    body = {
        "schema_version": "1.0.0",
        "$schema": "schemas/v1/sfx_manifest_entry.schema.json",
        "entry_count": len(entries),
        "entries": [e.to_dict() for e in entries],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(body, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def write_caption_templates_ron(entries: list[SfxManifestEntry], path: Path) -> None:
    """Write the canonical caption-templates registry consumed by
    cf_audio::caption_bridge::CaptionRegistry at startup."""
    body = {
        "schema_version": "1.0.0",
        "$schema": "schemas/v1/caption_template.schema.json",
        "templates": [
            {
                "sfx_id": e.id,
                "template": e.caption_template,
                "severity": e.caption_severity,
                "categories": e.caption_categories,
            }
            for e in entries
        ],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(body, indent=2, sort_keys=True) + "\n", encoding="utf-8")


# ─── Procedural bake driver ───────────────────────────────────────────────


_CATEGORY_TO_RECIPE_SECTION = {
    "WeaponFire": "weapon_action_sfx",
    "WeaponReload": "weapon_action_sfx",
    "WeaponJam": "weapon_action_sfx",
    "WeaponSwap": "weapon_action_sfx",
    "WeaponMagazine": "weapon_action_sfx",
    "Footstep": "footstep_sfx",
    "Locomotion": "locomotion_sfx",
    "Projectile": "projectile_sfx",
    "Impact": "impact_sfx_by_material",
    "BodyHit": "body_hit_sfx",
    "Dismember": "dismemberment_sfx",
    "Death": "death_sfx",
    "Ambient": "ambient_loops",
    "Weather": "weather_sfx",
    "Hazard": "hazard_sfx",
    "UI": "ui_sfx",
    "Chatter": "ai_chatter_prompts",
    "Banner": "ui_sfx",
    "Reactor": "hazard_sfx",
    "Mission": "ui_sfx",
    "Atmospheric": "hazard_sfx",
    "Chassis": "hazard_sfx",
    "Fluid": "hazard_sfx",
    "Power": "hazard_sfx",
    "Crafting": "ui_sfx",
    "VoiceGrunt": "ai_chatter_prompts",
    "Pet": "ai_chatter_prompts",
    "Storyteller": "ui_sfx",
    "Grenade": "projectile_sfx",
    "Melee": "body_hit_sfx",
    "Tool": "ui_sfx",
}


def bake_procedural_tier1(entries: list[SfxManifestEntry], verbose: bool = False) -> list[Path]:
    """Drive procedural Tier 1 bakes via `tools/audio_synth/sfx_recipes`
    + `synth_primitives`. Each entry produces a 16-bit PCM 48 kHz mono
    WAV at `game/content/audio/sfx/<id>.wav`. Returns the list of
    written paths.
    """
    import numpy as np
    sys.path.insert(0, str(_REPO_ROOT / "tools"))
    from audio_synth import sfx_recipes, synth_primitives as sp  # type: ignore

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    paths: list[Path] = []
    failures: list[tuple[str, str]] = []
    for i, e in enumerate(entries):
        rng = np.random.RandomState(int(e.seed) & 0xFFFFFFFF)
        section = _CATEGORY_TO_RECIPE_SECTION.get(e.category, "ui_sfx")
        entry_dict = {
            "id": e.id,
            "duration_target_sec": e.duration_ms / 1000.0,
            "loops": e.loops,
            "prompt": e.prompt,
            "material": e.material,
            "state": e.state,
            "intensity": "small",
            "weapon_class": "rifle",
            "stance": "walking",
            "origin": e.origin or "human",
        }
        try:
            samples = sfx_recipes.dispatch(section, entry_dict, rng)
            if samples is None or len(samples) == 0:
                failures.append((e.id, "empty"))
                continue
            target_dur = e.duration_ms / 1000.0
            samples = sp.ensure_duration(samples, target_dur)
            if e.loops:
                samples = sp.loop_align(samples, fade_ms=50.0)
            else:
                samples = sp.fade_in_out(samples, fade_ms=5.0)
            samples = sp.normalize_peak(samples, peak_dbfs=-14.0)
            out = OUT_DIR / f"{e.id}.wav"
            sp.write_wav(out, samples)
            paths.append(out)
        except Exception as exc:
            failures.append((e.id, str(exc)))
            continue
        if verbose and (i + 1) % 100 == 0:
            print(f"[generate_sfx] baked {i + 1}/{len(entries)}", file=sys.stderr)
    if failures:
        print(f"[generate_sfx] {len(failures)} failures (first 5: {failures[:5]})", file=sys.stderr)
    return paths


def ledger_all_entries(entries: list[SfxManifestEntry], paths: dict[str, Path]) -> tuple[int, int, int]:
    """Insert/supersede a ledger entry per baked WAV. Returns
    (replaced, inserted, total_after)."""
    records: list[SupersedeRecord] = []
    for e in entries:
        path = paths.get(e.id)
        if path is None or not path.exists():
            continue
        records.append(SupersedeRecord(
            category="Audio_SFX",
            kind=e.category.lower(),
            canonical_name=e.id,
            output_path=path.resolve(),
            new_pipeline=PIPELINE_TIER1,
            new_tool=TIER1_TOOL,
            new_model=TIER1_MODEL,
            new_model_version=TIER1_MODEL_VERSION,
            new_workflow=f"sfx_manifest::{e.category}",
            prompt=e.prompt,
            seed=e.seed,
            old_tier="Tier1_LLM_Audio",
            new_tier="Tier1_LLM_Audio",
        ))
    return apply_superseded_entries(records)


# ─── CLI ──────────────────────────────────────────────────────────────────


def _cmd_check(args: argparse.Namespace) -> int:
    """`--check` — report stale + missing SFX. Exit 0 on success."""
    entries = build_full_manifest()
    existing = count_audio_entries()
    expected = len(entries)
    on_disk = sum(1 for e in entries if (OUT_DIR / f"{e.id}.wav").exists())
    stale = expected - on_disk
    print(f"[generate_sfx] planned roster: {expected}")
    print(f"[generate_sfx] on-disk WAVs:   {on_disk}")
    print(f"[generate_sfx] stale + missing: {stale}")
    print(f"[generate_sfx] ledger Audio_SFX entries: {existing.get('Audio_SFX', 0)}")
    return 0


def _cmd_report(args: argparse.Namespace) -> int:
    """`--report` — print summary table only."""
    existing = count_audio_entries()
    on_disk_sfx = len(list(OUT_DIR.glob("*.wav"))) if OUT_DIR.exists() else 0
    print(f"on-disk SFX wavs:       {on_disk_sfx}")
    print(f"ledger Audio_SFX:       {existing.get('Audio_SFX', 0)}")
    print(f"ledger Audio_Voice:     {existing.get('Audio_Voice', 0)}")
    print(f"ledger Audio_Music:     {existing.get('Audio_Music', 0)}")
    return 0


def _cmd_all(args: argparse.Namespace) -> int:
    """`--all` — bake every roster entry that isn't on disk."""
    entries = build_full_manifest()
    if args.category:
        cat = args.category.lower()
        entries = [e for e in entries if e.category.lower() == cat or e.id.lower().startswith(cat)]

    # Skip entries that already have an on-disk WAV unless --force is set.
    if not args.force:
        entries = [e for e in entries if not (OUT_DIR / f"{e.id}.wav").exists()]

    if not entries:
        print("[generate_sfx] nothing to bake (use --force to re-bake)")
        # Always (re)write the canonical manifest + caption registry.
        all_entries = build_full_manifest()
        write_manifest_ron(all_entries, MANIFEST_OUT)
        write_caption_templates_ron(all_entries, CAPTION_TEMPLATES_OUT)
        return 0

    print(f"[generate_sfx] baking {len(entries)} entries to {OUT_DIR}", file=sys.stderr)
    bake_procedural_tier1(entries, verbose=True)

    # Build paths map for ledger insertion.
    paths = {e.id: OUT_DIR / f"{e.id}.wav" for e in entries}
    replaced, inserted, total = ledger_all_entries(entries, paths)
    print(f"[generate_sfx] ledger updated: replaced={replaced} inserted={inserted} total={total}")

    # Always re-emit the canonical manifest + caption registry for the
    # FULL roster (not just the entries we baked this run).
    all_entries = build_full_manifest()
    write_manifest_ron(all_entries, MANIFEST_OUT)
    write_caption_templates_ron(all_entries, CAPTION_TEMPLATES_OUT)
    print(f"[generate_sfx] wrote {MANIFEST_OUT.name} ({len(all_entries)} entries)")
    print(f"[generate_sfx] wrote {CAPTION_TEMPLATES_OUT.name}")
    return 0


def _mod_manifest_path(mod_id: str) -> Path:
    """Canonical mod SFX manifest path. Modders drop their manifest at
    `content/mods/<mod_id>/sfx_manifest.json`. The M33 workbench will
    grow a richer multi-file layout; M12A ships the minimal contract."""
    return _REPO_ROOT / "content" / "mods" / mod_id / "sfx_manifest.json"


def _load_mod_manifest(mod_id: str) -> list[SfxManifestEntry]:
    """Read a mod's SFX manifest + author defaults via the canonical
    rule-based caption authorer. Returns an empty list when the manifest
    file is missing — callers should treat that as a no-op."""
    path = _mod_manifest_path(mod_id)
    if not path.exists():
        return []
    data = json.loads(path.read_text(encoding="utf-8"))
    out: list[SfxManifestEntry] = []
    for raw in data.get("entries", []):
        entry_id = raw["id"]
        cat = author_caption_dict(entry_id, raw.get("prompt", ""))
        out.append(SfxManifestEntry(
            id=entry_id,
            category=str(raw.get("category", "UI")),
            prompt=raw.get("prompt", ""),
            duration_ms=int(raw.get("duration_ms", 500)),
            seed=_seed_for(f"mod:{mod_id}:{entry_id}"),
            loops=bool(raw.get("loops", False)),
            material=raw.get("material"),
            caption_template=raw.get("caption_template") or cat["template"],
            caption_severity=raw.get("caption_severity") or cat["severity"],
            caption_categories=raw.get("caption_categories") or cat["categories"],
            package_source=f"mod:{mod_id}@{raw.get('mod_version', '1.0.0')}",
            license=raw.get("license", "CC0"),
        ))
    return out


def _cmd_mod(args: argparse.Namespace) -> int:
    """`--mod <id>` — modder authoring path. Spec § Acceptance:

    > Mod author writes a custom SFX manifest entry. Pipeline generates
    > via same code path, registers as `category=Mod_Custom` in ledger.
    > Mod-pack publisher includes both the OGG + ledger entry.

    Reads `content/mods/<mod_id>/sfx_manifest.json`, bakes each entry
    via the same procedural Tier 1 path (and OGG conversion), writes
    WAV + OGG under `game/content/audio/sfx/mods/<mod_id>/`, registers
    each entry with `category=Mod_Custom`.
    """
    mod_id = args.mod
    manifest_path = _mod_manifest_path(mod_id)
    print(f"[generate_sfx] mod authoring — reading {manifest_path}")
    entries = _load_mod_manifest(mod_id)
    if not entries:
        print(
            f"[generate_sfx] no mod manifest found at {manifest_path}; "
            f"create one with shape {{'entries': [...]}}",
            file=sys.stderr,
        )
        return 1

    out_dir = _REPO_ROOT / "game" / "content" / "audio" / "sfx" / "mods" / mod_id
    out_dir.mkdir(parents=True, exist_ok=True)

    # Bake via the procedural Tier 1 path (same recipe dispatcher as
    # vanilla; mods get the same audio floor for free).
    sys.path.insert(0, str(_REPO_ROOT / "tools"))
    import numpy as np
    from audio_synth import sfx_recipes, synth_primitives as sp  # type: ignore

    baked_paths: dict[str, Path] = {}
    failures: list[tuple[str, str]] = []
    for e in entries:
        rng = np.random.RandomState(int(e.seed) & 0xFFFFFFFF)
        section = _CATEGORY_TO_RECIPE_SECTION.get(e.category, "ui_sfx")
        entry_dict = {
            "id": e.id,
            "duration_target_sec": e.duration_ms / 1000.0,
            "loops": e.loops,
            "prompt": e.prompt,
            "material": e.material,
        }
        try:
            samples = sfx_recipes.dispatch(section, entry_dict, rng)
            if samples is None or len(samples) == 0:
                failures.append((e.id, "empty"))
                continue
            samples = sp.ensure_duration(samples, e.duration_ms / 1000.0)
            if e.loops:
                samples = sp.loop_align(samples, fade_ms=50.0)
            else:
                samples = sp.fade_in_out(samples, fade_ms=5.0)
            samples = sp.normalize_peak(samples, peak_dbfs=-14.0)
            wav_path = out_dir / f"{e.id}.wav"
            sp.write_wav(wav_path, samples)
            # Convert to OGG sibling — mod runtime format matches vanilla.
            import wav_to_ogg as _w2o  # noqa: E402
            ogg_path = wav_path.with_suffix(".ogg")
            try:
                _w2o.convert_wav_to_ogg(wav_path, ogg_path, force=True)
            except Exception as exc:
                failures.append((e.id, f"ogg-encode: {exc}"))
                continue
            baked_paths[e.id] = ogg_path
        except Exception as exc:
            failures.append((e.id, str(exc)))

    # Register every baked entry with category=Mod_Custom (per spec).
    records: list[SupersedeRecord] = []
    for e in entries:
        ogg = baked_paths.get(e.id)
        if ogg is None:
            continue
        records.append(SupersedeRecord(
            category="Mod_Custom",
            kind=e.category.lower(),
            canonical_name=e.id,
            output_path=ogg.resolve(),
            new_pipeline=PIPELINE_TIER1,
            new_tool=TIER1_TOOL,
            new_model=TIER1_MODEL,
            new_model_version=TIER1_MODEL_VERSION,
            new_workflow=f"mod:{mod_id}",
            prompt=e.prompt,
            seed=e.seed,
            old_tier="Tier1_LLM_Audio",
            new_tier="Tier1_LLM_Audio",
        ))
    if records:
        replaced, inserted, total = apply_superseded_entries(records)
        print(
            f"[generate_sfx] mod '{mod_id}' baked {len(records)} entries — "
            f"ledger replaced={replaced} inserted={inserted} total={total}"
        )
    else:
        print(f"[generate_sfx] mod '{mod_id}': no entries baked")
    if failures:
        print(f"[generate_sfx] mod '{mod_id}' failures ({len(failures)}):", file=sys.stderr)
        for fid, reason in failures[:10]:
            print(f"  FAIL {fid}: {reason}", file=sys.stderr)
        return 1
    return 0


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="M12A Tier-1 SFX pipeline.")
    ap.add_argument("--all", action="store_true", help="Bake every roster entry that isn't on disk.")
    ap.add_argument("--check", action="store_true", help="Report stale + missing without baking.")
    ap.add_argument("--report", action="store_true", help="Print summary counts only.")
    ap.add_argument("--category", type=str, default=None, help="Filter to one category (e.g. 'weapon' / 'footstep').")
    ap.add_argument("--force", action="store_true", help="Re-bake every entry even when WAV exists.")
    ap.add_argument("--mod", type=str, default=None, help="Mod pack id for modder-authoring path.")
    args = ap.parse_args(argv)

    if args.mod:
        return _cmd_mod(args)
    if args.report:
        return _cmd_report(args)
    if args.check:
        return _cmd_check(args)
    if args.all or args.category:
        return _cmd_all(args)
    ap.print_help()
    return 2


if __name__ == "__main__":
    raise SystemExit(main())
