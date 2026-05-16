"""ElevenLabs SFX v2 bake — upgrade 242 SFX placeholders to Tier 2.

Reads the four SFX prompt manifests:
  - weapon_sfx_prompts.json
  - movement_sfx_prompts.json
  - impact_and_combat_sfx_prompts.json
  - ambient_environment_sfx_prompts.json

Calls `client.text_to_sound_effects.convert` with the `eleven_text_to_sound_v2`
model. Writes WAVs to `game/content/audio/sfx/<entry_id>.wav` (16-bit PCM
48 kHz mono). Replaces the existing Tier 1 ledger entries (same path, new
content hash) with Tier 2 entries.

Usage:
    python eleven_sfx.py --dry-run
    python eleven_sfx.py
    python eleven_sfx.py --filter weapon
    python eleven_sfx.py --filter ambient
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path

from elevenlabs.client import ElevenLabs

_HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(_HERE))

from keys import load_elevenlabs_key  # noqa: E402
from ledger_supersede import (  # noqa: E402
    SupersedeRecord,
    apply_superseded_entries,
)
from post_process import cleanup_wav  # noqa: E402

REPO_ROOT = _HERE.parents[1]
SFX_PROMPTS = REPO_ROOT / "game" / "content" / "sfx"
OUT_DIR = REPO_ROOT / "game" / "content" / "audio" / "sfx"
PROGRESS_PATH = _HERE / "_state" / "eleven_sfx_progress.json"

PIPELINE = "M12A_eleven_sfx_v1"
TOOL = "tools/audio_pipeline/eleven_sfx.py"
MODEL = "eleven_text_to_sound_v2"
MODEL_VERSION = "v2"

ELEVEN_SFX_MAX_SEC = 30.0


SOURCES: list[tuple[str, str, str, str]] = [
    # (filename, key, group, default_kind)
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


def _load_entries() -> list[dict]:
    out: list[dict] = []
    for fname, key, group, default_kind in SOURCES:
        path = SFX_PROMPTS / fname
        if not path.exists():
            continue
        data = json.loads(path.read_text(encoding="utf-8"))
        for e in data.get(key, []):
            dur = float(e.get("duration_target_sec", 1.0))
            dur = max(0.5, min(dur, ELEVEN_SFX_MAX_SEC))
            out.append({
                "id": e["id"],
                "group": group,
                "kind": default_kind,
                "prompt": e.get("prompt", ""),
                "duration_sec": dur,
                "loops": bool(e.get("loops", False)),
                "manifest": fname,
                "manifest_section": key,
            })
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


def _stream_to_pcm(byte_iter, dest: Path) -> int:
    dest.parent.mkdir(parents=True, exist_ok=True)
    written = 0
    with dest.open("wb") as f:
        for chunk in byte_iter:
            if not chunk:
                continue
            f.write(chunk)
            written += len(chunk)
    return written


def _wrap_pcm_to_wav(pcm_path: Path, wav_path: Path, *, sample_rate: int) -> None:
    import numpy as np
    import soundfile as sf
    raw = pcm_path.read_bytes()
    if not raw:
        raise ValueError("empty PCM stream")
    data = np.frombuffer(raw, dtype="<i2")
    sf.write(str(wav_path), data, sample_rate, subtype="PCM_16", format="WAV")


def _bake_one(client: ElevenLabs, entry: dict, *, dry_run: bool) -> tuple[bool, Path | None]:
    out_path = OUT_DIR / f"{entry['id']}.wav"
    pcm_path = out_path.with_suffix(".pcm")

    if dry_run:
        preview = entry["prompt"][:80].replace("\n", " ")
        print(f"[dry] {entry['id']:<48s} {entry['duration_sec']:5.2f}s loop={int(entry['loops'])} :: {preview}…")
        return True, None

    print(f"[sfx] {entry['id']:<48s} {entry['duration_sec']:.2f}s loop={int(entry['loops'])}")
    try:
        stream = client.text_to_sound_effects.convert(
            text=entry["prompt"],
            duration_seconds=float(entry["duration_sec"]),
            loop=bool(entry["loops"]),
            prompt_influence=0.45,
            model_id=MODEL,
            output_format="pcm_48000",
        )
    except Exception as exc:
        print(f"[sfx] FAIL request {entry['id']}: {exc}")
        return False, None

    try:
        size = _stream_to_pcm(stream, pcm_path)
        if size < 4096:
            print(f"[sfx] FAIL {entry['id']} — short PCM ({size}B)")
            pcm_path.unlink(missing_ok=True)
            return False, None
        _wrap_pcm_to_wav(pcm_path, out_path, sample_rate=48000)
        pcm_path.unlink(missing_ok=True)
        cleanup_wav(out_path, category="sfx", loop=bool(entry["loops"]), skip_trim=False)
        return True, out_path
    except Exception as exc:
        print(f"[sfx] FAIL stream/wrap {entry['id']}: {exc}")
        try:
            pcm_path.unlink(missing_ok=True)
            if out_path.exists() and out_path.stat().st_size < 8192:
                out_path.unlink(missing_ok=True)
        except OSError:
            pass
        return False, None


def _record_for(entry: dict, path: Path) -> SupersedeRecord:
    return SupersedeRecord(
        category="Audio_SFX",
        kind=entry["kind"],
        canonical_name=entry["id"],
        output_path=path,
        new_pipeline=PIPELINE,
        new_tool=TOOL,
        new_model=MODEL,
        new_model_version=MODEL_VERSION,
        new_workflow=f"{entry['manifest']}::{entry['manifest_section']}",
        prompt=entry["prompt"],
        seed=abs(hash(entry["id"])) & 0x7FFFFFFF,
    )


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--filter",
                    choices=tuple(set(s[2] for s in SOURCES)),
                    default=None)
    ap.add_argument("--ids", default=None,
                    help="Comma-separated id list to bake.")
    ap.add_argument("--limit", type=int, default=None)
    ap.add_argument("--resume", action="store_true")
    ap.add_argument("--reset-progress", action="store_true")
    ap.add_argument("--inter-call-sleep", type=float, default=0.3)
    args = ap.parse_args()

    entries = _load_entries()
    if args.filter:
        entries = [e for e in entries if e["group"] == args.filter]
    if args.ids:
        wanted = {x.strip() for x in args.ids.split(",") if x.strip()}
        entries = [e for e in entries if e["id"] in wanted]
    if args.limit:
        entries = entries[: args.limit]

    if args.reset_progress and PROGRESS_PATH.exists():
        PROGRESS_PATH.unlink(missing_ok=True)
    progress = _load_progress()
    if args.resume:
        completed = set(progress["completed"])
        before = len(entries)
        entries = [e for e in entries if e["id"] not in completed]
        print(f"[sfx] resume — skipping {before - len(entries)} already-done")

    print(f"[sfx] entries = {len(entries)}  dry_run={args.dry_run}")
    if args.dry_run:
        for e in entries:
            _bake_one(client=None, entry=e, dry_run=True)  # type: ignore[arg-type]
        return 0

    key = load_elevenlabs_key()
    client = ElevenLabs(api_key=key.value)
    print(f"[sfx] client ready ({key!r})")

    baked: list[tuple[dict, Path]] = []
    for i, entry in enumerate(entries, start=1):
        ok, path = _bake_one(client, entry, dry_run=False)
        if ok and path is not None:
            baked.append((entry, path))
            progress["completed"].append(entry["id"])
        else:
            progress["failed"].append(entry["id"])
        _save_progress(progress)
        if i < len(entries):
            time.sleep(args.inter_call_sleep)
        if baked and i % 25 == 0:
            recs = [_record_for(e, p) for e, p in baked]
            replaced, inserted, total = apply_superseded_entries(recs)
            print(f"[sfx] checkpoint ledger: replaced={replaced} inserted={inserted} total={total}")
            baked = []

    if baked:
        recs = [_record_for(e, p) for e, p in baked]
        replaced, inserted, total = apply_superseded_entries(recs)
        print(f"[sfx] final ledger: replaced={replaced} inserted={inserted} total={total}")

    print(
        f"[sfx] DONE — completed={len(progress['completed'])}  "
        f"failed={len(progress['failed'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
