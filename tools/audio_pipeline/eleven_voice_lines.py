"""ElevenLabs voice line bake — 242 lines across 6 manifests.

Reads the four voice prompt manifests:
  - voice_npc_prompts.json                  → Multilingual v2 (high quality)
  - voice_storyteller_boss_prompts.json     → Multilingual v2
  - voice_mission_tutorial_prompts.json     → Multilingual v2
  - voice_chatter_prompts.json              → Flash v2.5 (cheap volume work)

Looks each line's `voice_id` up in `per_npc_voice_registry.toml` and resolves
to the real ElevenLabs voice_id. Writes WAVs to
`game/content/audio/voice/<entry_id>.wav` (16-bit PCM 48 kHz mono). Inserts a
fresh `Audio_Voice` ledger entry per file (no Tier 1 to supersede; voice was
manifests-only at Tier 1).

Usage:
    python eleven_voice_lines.py --dry-run
    python eleven_voice_lines.py
    python eleven_voice_lines.py --filter npc
    python eleven_voice_lines.py --filter chatter
    python eleven_voice_lines.py --voice coalition_marcus_authoritative_male
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import tomllib
from pathlib import Path

from elevenlabs.client import ElevenLabs
from elevenlabs.types.voice_settings import VoiceSettings

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from keys import load_elevenlabs_key  # noqa: E402
from ledger_supersede import SupersedeRecord, add_new_entries  # noqa: E402
from post_process import cleanup_wav  # noqa: E402

REPO_ROOT = _HERE.parents[1]
SFX_DIR = REPO_ROOT / "game" / "content" / "sfx"
OUT_DIR = REPO_ROOT / "game" / "content" / "audio" / "voice"
REGISTRY_PATH = _HERE / "voice_synthesis" / "per_npc_voice_registry.toml"
ALIASES_PATH = _HERE / "voice_synthesis" / "voice_aliases.toml"
PROGRESS_PATH = _HERE / "_state" / "eleven_voice_lines_progress.json"

PIPELINE = "M37A_eleven_voice_v1"
TOOL = "tools/audio_pipeline/eleven_voice_lines.py"

# Model selection (all latest as of 2026):
# - eleven_v3              — most expressive, supports audio tags, best for named characters
# - eleven_multilingual_v2 — production stable fallback (kept for reference)
# - eleven_flash_v2_5      — fast + cheap, ideal for short faction chatter callouts
MODEL_HQ = "eleven_v3"
MODEL_HQ_FALLBACK = "eleven_multilingual_v2"
MODEL_FLASH = "eleven_flash_v2_5"

VOICE_SETTINGS = VoiceSettings(
    stability=0.55,
    similarity_boost=0.85,
    style=0.20,
    use_speaker_boost=True,
)

# Manifest definitions: (filename, key, group_label, model, kind)
MANIFESTS: list[tuple[str, str, str, str, str]] = [
    ("voice_npc_prompts.json", "npc_voice_prompts", "npc", MODEL_HQ, "voice_line"),
    ("voice_storyteller_boss_prompts.json", "storyteller_voice_prompts", "storyteller", MODEL_HQ, "voice_line"),
    ("voice_storyteller_boss_prompts.json", "boss_voice_prompts", "boss", MODEL_HQ, "voice_line"),
    ("voice_mission_tutorial_prompts.json", "mission_voice_prompts", "mission", MODEL_HQ, "voice_line"),
    ("voice_mission_tutorial_prompts.json", "tutorial_voice_prompts", "tutorial", MODEL_HQ, "voice_line"),
    ("voice_chatter_prompts.json", "per_faction_chatter_pool", "chatter", MODEL_FLASH, "voice_chatter"),
]


def _load_registry() -> dict[str, dict[str, str]]:
    if not REGISTRY_PATH.exists():
        return {}
    with REGISTRY_PATH.open("rb") as f:
        return dict(tomllib.load(f))


def _load_aliases() -> dict[str, str]:
    """Read alias map (internal_voice_id → aliased_internal_voice_id)."""
    if not ALIASES_PATH.exists():
        return {}
    with ALIASES_PATH.open("rb") as f:
        raw = tomllib.load(f)
    out: dict[str, str] = {}
    for k, v in raw.items():
        if isinstance(v, dict) and v.get("alias_of"):
            out[k] = v["alias_of"]
    return out


def _resolve_real_voice(voice_id: str, registry: dict, aliases: dict[str, str]) -> tuple[str | None, str | None]:
    """Returns (real_elevenlabs_voice_id, resolved_internal_id)."""
    direct = registry.get(voice_id, {}).get("elevenlabs_voice_id")
    if direct:
        return direct, voice_id
    alias = aliases.get(voice_id)
    if alias:
        real = registry.get(alias, {}).get("elevenlabs_voice_id")
        if real:
            return real, alias
    return None, None


def _load_chatter_voice_map() -> dict[str, str]:
    """For chatter, look up per-faction voice_id from the manifest's `factions` section."""
    path = SFX_DIR / "voice_chatter_prompts.json"
    if not path.exists():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    out: dict[str, str] = {}
    for fac, cfg in data.get("factions", {}).items():
        if isinstance(cfg, dict) and cfg.get("voice_id"):
            out[fac] = cfg["voice_id"]
    return out


def _load_all_lines() -> list[dict]:
    chatter_map = _load_chatter_voice_map()
    out: list[dict] = []
    for fname, key, group, model, kind in MANIFESTS:
        path = SFX_DIR / fname
        if not path.exists():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        for entry in data.get(key, []):
            line = {
                "id": entry["id"],
                "group": group,
                "model": model,
                "kind": kind,
                "voice_id": entry.get("voice_id"),
                "text": entry.get("text", "").strip(),
                "manifest": fname,
                "manifest_section": key,
                "duration_hint_sec": entry.get("duration_target_sec"),
            }
            if group == "chatter":
                fac = entry.get("faction")
                if fac and fac in chatter_map:
                    line["voice_id"] = chatter_map[fac]
                line["faction"] = fac
            if not line["text"] or not line["voice_id"]:
                continue
            out.append(line)
    return out


def _load_progress() -> dict:
    if not PROGRESS_PATH.exists():
        return {"completed": [], "failed": []}
    try:
        return json.loads(PROGRESS_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {"completed": [], "failed": []}


def _save_progress(state: dict) -> None:
    PROGRESS_PATH.parent.mkdir(parents=True, exist_ok=True)
    PROGRESS_PATH.write_text(json.dumps(state, indent=2), encoding="utf-8")


def _stream_to_wav(byte_iter, dest: Path) -> int:
    dest.parent.mkdir(parents=True, exist_ok=True)
    written = 0
    with dest.open("wb") as f:
        for chunk in byte_iter:
            if not chunk:
                continue
            f.write(chunk)
            written += len(chunk)
    return written


def _bake_one(client: ElevenLabs, line: dict, registry: dict[str, dict[str, str]],
              aliases: dict[str, str], *, dry_run: bool) -> tuple[bool, Path | None]:
    real_voice, resolved = _resolve_real_voice(line["voice_id"], registry, aliases)
    if not real_voice:
        if dry_run:
            print(f"[dry] WARN no registry/alias for {line['voice_id']}; line {line['id']} skipped")
            return False, None
        print(f"[voice] SKIP {line['id']} — no registry+alias entry for {line['voice_id']}")
        return False, None
    if resolved != line["voice_id"]:
        line["voice_alias_of"] = resolved
        print(f"[voice]   alias: {line['voice_id']} → {resolved}")

    out_path = OUT_DIR / f"{line['id']}.wav"
    if dry_run:
        preview = line["text"][:70].replace("\n", " ")
        print(f"[dry] {line['id']:<48s} model={line['model']} voice={line['voice_id'][:30]} :: {preview!r}")
        return True, None

    print(f"[voice] {line['id']:<48s} model={line['model']} voice={line['voice_id'][:30]}")
    stream = None
    last_exc: Exception | None = None
    for model_attempt in [line["model"], MODEL_HQ_FALLBACK if line["model"] == MODEL_HQ else None]:
        if model_attempt is None:
            continue
        try:
            stream = client.text_to_speech.convert(
                voice_id=real_voice,
                text=line["text"],
                model_id=model_attempt,
                output_format="wav_48000",
                voice_settings=VOICE_SETTINGS,
                seed=abs(hash(line["id"])) & 0x7FFFFFFF,
                apply_text_normalization="auto",
            )
            line["actual_model"] = model_attempt
            break
        except Exception as exc:
            last_exc = exc
            if model_attempt == line["model"] and line["model"] == MODEL_HQ:
                print(f"[voice] WARN {model_attempt} failed; falling back to {MODEL_HQ_FALLBACK} ({exc})")
                continue
            print(f"[voice] FAIL request {line['id']}: {exc}")
            return False, None
    if stream is None:
        print(f"[voice] FAIL all models for {line['id']}: {last_exc}")
        return False, None

    try:
        bytes_written = _stream_to_wav(stream, out_path)
        if bytes_written < 4096:
            print(f"[voice] FAIL {line['id']} — short WAV ({bytes_written}B)")
            out_path.unlink(missing_ok=True)
            return False, None
        cleanup_wav(out_path, category="voice", loop=False, skip_trim=False)
        return True, out_path
    except Exception as exc:
        print(f"[voice] FAIL save {line['id']}: {exc}")
        try:
            if out_path.exists() and out_path.stat().st_size < 8192:
                out_path.unlink(missing_ok=True)
        except OSError:
            pass
        return False, None


def _records_for(line: dict, path: Path) -> SupersedeRecord:
    actual_model = line.get("actual_model", line["model"])
    if actual_model == "eleven_v3":
        version = "v3"
    elif actual_model == "eleven_flash_v2_5":
        version = "v2.5"
    elif actual_model == "eleven_multilingual_v2":
        version = "v2"
    else:
        version = "unknown"
    return SupersedeRecord(
        category="Audio_Voice",
        kind=line["kind"],
        canonical_name=line["id"],
        output_path=path,
        new_pipeline=PIPELINE,
        new_tool=TOOL,
        new_model=actual_model,
        new_model_version=version,
        new_workflow=f"{line['manifest']}::{line['manifest_section']}",
        prompt=line["text"],
        seed=abs(hash(line["id"])) & 0x7FFFFFFF,
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--filter", choices=("npc", "storyteller", "boss", "mission", "tutorial", "chatter"),
                    default=None, help="Only one group.")
    ap.add_argument("--voice", default=None,
                    help="Only lines for this voice_id (comma-separated allowed).")
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--reset-progress", action="store_true")
    ap.add_argument("--inter-call-sleep", type=float, default=0.4)
    args = ap.parse_args()

    registry = _load_registry()
    aliases = _load_aliases()
    print(f"[voice] registry size = {len(registry)}  aliases = {len(aliases)}")

    lines = _load_all_lines()
    if args.filter:
        lines = [l for l in lines if l["group"] == args.filter]
    if args.voice:
        wanted = {v.strip() for v in args.voice.split(",") if v.strip()}
        lines = [l for l in lines if l["voice_id"] in wanted]
    if args.limit:
        lines = lines[: args.limit]

    if args.reset_progress and PROGRESS_PATH.exists():
        PROGRESS_PATH.unlink(missing_ok=True)
    progress = _load_progress()
    if args.resume:
        completed = set(progress["completed"])
        before = len(lines)
        lines = [l for l in lines if l["id"] not in completed]
        print(f"[voice] resume — skipping {before - len(lines)} already-done")

    print(f"[voice] lines to bake = {len(lines)}  dry_run={args.dry_run}")

    if args.dry_run:
        for l in lines:
            _bake_one(client=None, line=l, registry=registry, aliases=aliases, dry_run=True)  # type: ignore[arg-type]
        print(f"[voice] dry-run complete — {len(lines)} previewed")
        return 0

    key = load_elevenlabs_key()
    client = ElevenLabs(api_key=key.value)
    print(f"[voice] client ready ({key!r})")

    baked: list[tuple[dict, Path]] = []
    for i, line in enumerate(lines, start=1):
        ok, path = _bake_one(client, line, registry, aliases, dry_run=False)
        if ok and path is not None:
            baked.append((line, path))
            progress["completed"].append(line["id"])
        else:
            progress["failed"].append(line["id"])
        _save_progress(progress)
        if i < len(lines):
            time.sleep(args.inter_call_sleep)
        if baked and i % 20 == 0:
            recs = [_records_for(l, p) for l, p in baked]
            total = add_new_entries(recs)
            print(f"[voice] checkpoint ledger total={total}  +{len(recs)}")
            baked = []

    if baked:
        recs = [_records_for(l, p) for l, p in baked]
        total = add_new_entries(recs)
        print(f"[voice] final ledger total={total}  +{len(recs)}")

    print(
        f"[voice] DONE — completed={len(progress['completed'])}  "
        f"failed={len(progress['failed'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
