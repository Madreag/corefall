"""Re-ledger every existing audio WAV on disk.

When `tools/asset_gen/build_placeholders.py --all` overwrites
`content/asset_ledger/ledger.jsonl`, the audio (voice/sfx/music) entries that
were never part of the placeholder pipeline get dropped. This helper walks
the three audio directories under `game/content/audio/` and re-inserts an
`Audio_*` ledger entry for every WAV that has a matching prompt manifest +
progress record.

Idempotent: running it twice produces the same ledger.
"""

from __future__ import annotations

import json
import sys
import tomllib
from pathlib import Path

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from ledger_supersede import SupersedeRecord, apply_superseded_entries  # noqa: E402

REPO_ROOT = _HERE.parents[1]
SFX_PROMPTS = REPO_ROOT / "game" / "content" / "sfx"
VOICE_DIR = REPO_ROOT / "game" / "content" / "audio" / "voice"
SFX_DIR = REPO_ROOT / "game" / "content" / "audio" / "sfx"
MUSIC_DIR = REPO_ROOT / "game" / "content" / "audio" / "music"

VOICE_PIPELINE = "M37A_eleven_voice_v1"
VOICE_TOOL = "tools/audio_pipeline/eleven_voice_lines.py"
VOICE_MODEL_HQ = "eleven_v3"
VOICE_MODEL_FLASH = "eleven_flash_v2_5"

SFX_PIPELINE = "M12A_eleven_sfx_v1"
SFX_TOOL = "tools/audio_pipeline/eleven_sfx.py"
SFX_MODEL = "eleven_text_to_sound_v2"

MUSIC_PIPELINE = "M37A_eleven_music_v1"
MUSIC_TOOL = "tools/audio_pipeline/eleven_music.py"
MUSIC_MODEL = "music_v1"

MUSIC_PROCEDURAL_PIPELINE = "M37A_procedural_music_v1"
MUSIC_PROCEDURAL_TOOL = "tools/audio_synth/music_bake.py"
MUSIC_PROCEDURAL_MODEL = "procedural-numpy-synth-v1"


def _load_sfx_prompts() -> dict[str, dict[str, str]]:
    """Returns id → {prompt, kind, manifest, manifest_section, group}."""
    out: dict[str, dict[str, str]] = {}
    sources = [
        ("weapon_sfx_prompts.json", "weapon_action_sfx", "weapon", "weapon"),
        ("movement_sfx_prompts.json", "footstep_sfx", "footstep", "footstep"),
        ("movement_sfx_prompts.json", "locomotion_sfx", "locomotion", "locomotion"),
        ("impact_and_combat_sfx_prompts.json", "projectile_sfx", "projectile", "projectile"),
        ("impact_and_combat_sfx_prompts.json", "impact_sfx_by_material", "impact", "impact"),
        ("impact_and_combat_sfx_prompts.json", "body_hit_sfx", "body_hit", "body_hit"),
        ("impact_and_combat_sfx_prompts.json", "dismemberment_sfx", "dismemberment", "dismemberment"),
        ("impact_and_combat_sfx_prompts.json", "death_sfx", "death", "death"),
        ("ambient_environment_sfx_prompts.json", "ambient_loops", "ambient", "ambient_loop"),
        ("ambient_environment_sfx_prompts.json", "weather_sfx", "weather", "weather"),
        ("ambient_environment_sfx_prompts.json", "hazard_sfx", "hazard", "hazard"),
        ("ambient_environment_sfx_prompts.json", "ui_sfx", "ui", "ui"),
        ("ambient_environment_sfx_prompts.json", "ai_chatter_prompts", "ai_chatter", "ai_chatter"),
    ]
    for fname, key, group, default_kind in sources:
        path = SFX_PROMPTS / fname
        if not path.exists():
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        for e in data.get(key, []):
            entry_id = e.get("id")
            if not entry_id:
                continue
            out[entry_id] = {
                "prompt": e.get("prompt", ""),
                "kind": default_kind,
                "manifest": fname,
                "manifest_section": key,
                "group": group,
            }
    return out


def _load_voice_prompts() -> dict[str, dict[str, str]]:
    out: dict[str, dict[str, str]] = {}
    manifests = [
        ("voice_npc_prompts.json", "npc_voice_prompts", "npc", VOICE_MODEL_HQ, "voice_line"),
        ("voice_storyteller_boss_prompts.json", "storyteller_voice_prompts", "storyteller", VOICE_MODEL_HQ, "voice_line"),
        ("voice_storyteller_boss_prompts.json", "boss_voice_prompts", "boss", VOICE_MODEL_HQ, "voice_line"),
        ("voice_mission_tutorial_prompts.json", "mission_voice_prompts", "mission", VOICE_MODEL_HQ, "voice_line"),
        ("voice_mission_tutorial_prompts.json", "tutorial_voice_prompts", "tutorial", VOICE_MODEL_HQ, "voice_line"),
        ("voice_chatter_prompts.json", "per_faction_chatter_pool", "chatter", VOICE_MODEL_FLASH, "voice_chatter"),
    ]
    for fname, key, group, model, kind in manifests:
        path = SFX_PROMPTS / fname
        if not path.exists():
            continue
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        for e in data.get(key, []):
            entry_id = e.get("id")
            if not entry_id:
                continue
            out[entry_id] = {
                "prompt": e.get("line") or e.get("prompt") or "",
                "kind": kind,
                "model": model,
                "manifest": fname,
                "manifest_section": key,
                "group": group,
            }
    return out


def _load_music_prompts() -> dict[str, dict[str, str]]:
    out: dict[str, dict[str, str]] = {}
    path = SFX_PROMPTS / "music_tracks_prompts.json"
    if not path.exists():
        return out
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return out
    for section_key in ("music_tracks", "ambient_tracks", "world_ambient_tracks", "intro_tracks"):
        for e in data.get(section_key, []) or []:
            entry_id = e.get("id") or e.get("canonical_name")
            if not entry_id:
                continue
            out[entry_id] = {
                "prompt": e.get("prompt", ""),
                "kind": e.get("kind", "music_loop"),
                "manifest": "music_tracks_prompts.json",
                "manifest_section": section_key,
                "group": e.get("group", "music"),
            }
    return out


def _is_tier2_music(wav_path: Path) -> bool:
    """Heuristic: ElevenLabs music tracks are >2 MB (PCM 48k stereo ~60s).
    Tier 1 procedural numpy tracks are typically <2 MB.
    """
    try:
        size = wav_path.stat().st_size
    except OSError:
        return False
    return size > 2_000_000


def build_records() -> list[SupersedeRecord]:
    records: list[SupersedeRecord] = []

    sfx_prompts = _load_sfx_prompts()
    if SFX_DIR.exists():
        for wav in sorted(SFX_DIR.glob("*.wav")):
            entry_id = wav.stem
            meta = sfx_prompts.get(entry_id)
            if not meta:
                continue
            records.append(SupersedeRecord(
                category="Audio_SFX",
                kind=meta["kind"],
                canonical_name=entry_id,
                output_path=wav.resolve(),
                new_pipeline=SFX_PIPELINE,
                new_tool=SFX_TOOL,
                new_model=SFX_MODEL,
                new_model_version="v2",
                new_workflow=f"{meta['manifest']}::{meta['manifest_section']}",
                prompt=meta["prompt"],
                seed=abs(hash(entry_id)) & 0x7FFFFFFF,
                old_tier="Tier1_LLM_Audio",
                new_tier="Tier2_Audio_Production",
            ))

    voice_prompts = _load_voice_prompts()
    if VOICE_DIR.exists():
        for wav in sorted(VOICE_DIR.glob("*.wav")):
            entry_id = wav.stem
            meta = voice_prompts.get(entry_id)
            if not meta:
                continue
            records.append(SupersedeRecord(
                category="Audio_Voice",
                kind=meta["kind"],
                canonical_name=entry_id,
                output_path=wav.resolve(),
                new_pipeline=VOICE_PIPELINE,
                new_tool=VOICE_TOOL,
                new_model=meta["model"],
                new_model_version="v2.5" if meta["model"] == VOICE_MODEL_FLASH else "v3",
                new_workflow=f"{meta['manifest']}::{meta['manifest_section']}",
                prompt=meta["prompt"],
                seed=abs(hash(entry_id)) & 0x7FFFFFFF,
                old_tier="Tier1_LLM_Audio",
                new_tier="Tier2_Audio_Production",
            ))

    music_prompts = _load_music_prompts()
    if MUSIC_DIR.exists():
        for wav in sorted(MUSIC_DIR.glob("*.wav")):
            entry_id = wav.stem
            meta = music_prompts.get(entry_id, {})
            tier2 = _is_tier2_music(wav)
            if tier2:
                records.append(SupersedeRecord(
                    category="Audio_Music",
                    kind=meta.get("kind", "music_loop"),
                    canonical_name=entry_id,
                    output_path=wav.resolve(),
                    new_pipeline=MUSIC_PIPELINE,
                    new_tool=MUSIC_TOOL,
                    new_model=MUSIC_MODEL,
                    new_model_version="v1",
                    new_workflow=f"{meta.get('manifest', 'music_tracks_prompts.json')}::{meta.get('manifest_section', 'music_tracks')}",
                    prompt=meta.get("prompt", ""),
                    seed=abs(hash(entry_id)) & 0x7FFFFFFF,
                    old_tier="Tier1_LLM_Audio",
                    new_tier="Tier2_Audio_Production",
                ))
            else:
                records.append(SupersedeRecord(
                    category="Audio_Music",
                    kind=meta.get("kind", "music_loop"),
                    canonical_name=entry_id,
                    output_path=wav.resolve(),
                    new_pipeline=MUSIC_PROCEDURAL_PIPELINE,
                    new_tool=MUSIC_PROCEDURAL_TOOL,
                    new_model=MUSIC_PROCEDURAL_MODEL,
                    new_model_version="v1",
                    new_workflow=f"{meta.get('manifest', 'music_tracks_prompts.json')}::{meta.get('manifest_section', 'music_tracks')}",
                    prompt=meta.get("prompt", ""),
                    seed=abs(hash(entry_id)) & 0x7FFFFFFF,
                    old_tier="Tier1_LLM_Audio",
                    new_tier="Tier1_LLM_Audio",
                ))

    return records


def main() -> int:
    records = build_records()
    if not records:
        print("[reledger] no audio files found")
        return 0
    print(f"[reledger] planning {len(records)} audio entries:")
    by_cat: dict[str, int] = {}
    for r in records:
        by_cat[r.category] = by_cat.get(r.category, 0) + 1
    for cat, count in sorted(by_cat.items()):
        print(f"  {cat}: {count}")
    replaced, inserted, total = apply_superseded_entries(records)
    print(f"[reledger] done: replaced={replaced} inserted={inserted} total={total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
