"""ElevenLabs Music v1 bake — PRIMARY music backbone for Corefall.

Generates 120 music tracks from `game/content/sfx/music_tracks_prompts.json`
using the ElevenLabs Music API (`client.music.compose`). Each variant becomes
a single WAV at `game/content/audio/music/<track_id>_<variant>.wav` and gets
a Tier 2 ledger entry (replacing the Tier 1 procedural placeholder).

The AIVA path is offloaded to a separate agent (see HANDOFF_AIVA_MUSIC.md).
When that agent delivers AIVA WAVs, the `ingest_aiva` flow can supersede
these ElevenLabs tracks with the AIVA equivalents at the same ledger rows.

Usage:
    python eleven_music.py --dry-run
    python eleven_music.py                       # full bake (resumable)
    python eleven_music.py --filter world        # only world ambient
    python eleven_music.py --filter faction
    python eleven_music.py --filter storyteller
    python eleven_music.py --filter boss
    python eleven_music.py --variants calm,debrief
"""

from __future__ import annotations

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Iterable

import soundfile as sf  # noqa: F401 — keep available for downstream io
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
PROMPTS_PATH = REPO_ROOT / "game" / "content" / "sfx" / "music_tracks_prompts.json"
OUT_DIR = REPO_ROOT / "game" / "content" / "audio" / "music"
PROGRESS_PATH = _HERE / "_state" / "eleven_music_progress.json"

PIPELINE = "M37A_eleven_music_v1"
TOOL = "tools/audio_pipeline/eleven_music.py"
MODEL = "music_v1"
MODEL_VERSION = "v1"

# ElevenLabs hard caps Music compose at 5 minutes (300 s); keep our requests
# inside that envelope. Loop logic at runtime extends short loops by repetition.
ELEVEN_MAX_LEN_SEC = 300


def _load_prompts() -> list[dict]:
    raw = json.loads(PROMPTS_PATH.read_text(encoding="utf-8"))
    out: list[dict] = []
    for group_key, group_label in (
        ("world_ambient_tracks", "world"),
        ("faction_theme_tracks", "faction"),
        ("storyteller_theme_tracks", "storyteller"),
        ("boss_theme_tracks", "boss"),
    ):
        for entry in raw.get(group_key, []):
            for variant_name, vbody in entry.get("variants", {}).items():
                out.append({
                    "group": group_label,
                    "track_id": entry["id"],
                    "canonical_name": f"{entry['canonical_name']} ({variant_name})",
                    "file_id": f"{entry['id']}_{variant_name}",
                    "variant": variant_name,
                    "duration_seconds": min(int(entry["duration_seconds"]), ELEVEN_MAX_LEN_SEC),
                    "tempo_bpm": entry.get("tempo_bpm"),
                    "key": entry.get("key"),
                    "musicgen_prompt": vbody["musicgen_prompt"],
                    "seed": int(vbody["seed"]),
                })
    return out


def _load_progress() -> dict:
    if not PROGRESS_PATH.exists():
        return {"completed": [], "failed": [], "skipped": []}
    try:
        return json.loads(PROGRESS_PATH.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return {"completed": [], "failed": [], "skipped": []}


def _save_progress(state: dict) -> None:
    PROGRESS_PATH.parent.mkdir(parents=True, exist_ok=True)
    PROGRESS_PATH.write_text(json.dumps(state, indent=2), encoding="utf-8")


def _build_prompt_text(entry: dict) -> str:
    """Assemble a high-leverage prompt for ElevenLabs Music v1."""
    base = entry["musicgen_prompt"]
    suffix_bits: list[str] = ["instrumental", "no vocals", "loopable"]
    if entry["tempo_bpm"]:
        suffix_bits.append(f"{entry['tempo_bpm']} BPM")
    if entry["key"]:
        suffix_bits.append(f"key of {entry['key']}")
    suffix_bits.append("seamless intro and outro that bridge well")
    return base + ". " + ", ".join(suffix_bits) + "."


def _stream_to_wav(byte_iter: Iterable[bytes], dest: Path) -> int:
    """Write iterator-of-bytes (an ElevenLabs WAV stream) to `dest`."""
    dest.parent.mkdir(parents=True, exist_ok=True)
    written = 0
    with dest.open("wb") as f:
        for chunk in byte_iter:
            if not chunk:
                continue
            f.write(chunk)
            written += len(chunk)
    return written


def _bake_one(client: ElevenLabs, entry: dict, *, dry_run: bool) -> tuple[bool, Path | None]:
    file_id = entry["file_id"]
    out_path = OUT_DIR / f"{file_id}.wav"
    prompt_text = _build_prompt_text(entry)
    duration_ms = max(10_000, min(entry["duration_seconds"] * 1000, ELEVEN_MAX_LEN_SEC * 1000))

    if dry_run:
        prompt_preview = prompt_text[:90].replace("\n", " ")
        print(f"[dry] {file_id:<48s} → pcm_48000 {duration_ms/1000:5.1f}s :: {prompt_preview}…")
        return True, None

    print(f"[music] {file_id:<48s} bake → {out_path.name} ({duration_ms/1000:.1f}s)")
    pcm_path = out_path.with_suffix(".pcm")
    try:
        stream = client.music.compose(
            prompt=prompt_text,
            music_length_ms=duration_ms,
            model_id=MODEL,
            output_format="pcm_48000",
            force_instrumental=True,
            store_for_inpainting=False,
        )
        bytes_written = _stream_to_wav(stream, pcm_path)
        if bytes_written < 8192:
            print(f"[music] FAIL {file_id} — wrote only {bytes_written}B")
            pcm_path.unlink(missing_ok=True)
            return False, None
        _wrap_music_pcm_to_wav(pcm_path, out_path, sample_rate=48000)
        pcm_path.unlink(missing_ok=True)
        cleanup_wav(out_path, category="music", loop=True, skip_trim=False)
        return True, out_path
    except Exception as exc:
        print(f"[music] FAIL {file_id} — {exc}")
        try:
            pcm_path.unlink(missing_ok=True)
            if out_path.exists() and out_path.stat().st_size < 8192:
                out_path.unlink(missing_ok=True)
        except OSError:
            pass
        return False, None


def _wrap_music_pcm_to_wav(pcm_path: Path, wav_path: Path, *, sample_rate: int) -> None:
    """Wrap raw PCM_S16LE stereo (interleaved) into a stereo WAV file."""
    import numpy as np
    import soundfile as sf
    raw = pcm_path.read_bytes()
    if not raw:
        raise ValueError("empty PCM stream")
    samples = np.frombuffer(raw, dtype="<i2")
    if samples.size % 2 == 0:
        stereo = samples.reshape(-1, 2)
        sf.write(str(wav_path), stereo, sample_rate, subtype="PCM_16", format="WAV")
    else:
        sf.write(str(wav_path), samples, sample_rate, subtype="PCM_16", format="WAV")


def _supersede_records(baked: list[tuple[dict, Path]]) -> list[SupersedeRecord]:
    out: list[SupersedeRecord] = []
    for entry, path in baked:
        out.append(
            SupersedeRecord(
                category="Audio_Music",
                kind="music_loop",
                canonical_name=entry["file_id"],
                output_path=path,
                new_pipeline=PIPELINE,
                new_tool=TOOL,
                new_model=MODEL,
                new_model_version=MODEL_VERSION,
                new_workflow=f"music_tracks_prompts.json::{entry['group']}::{entry['variant']}",
                prompt=_build_prompt_text(entry),
                seed=entry["seed"] & 0x7FFFFFFF,
            )
        )
    return out


def _filter_entries(entries: list[dict], group_filter: str | None,
                    variant_filter: list[str] | None,
                    track_filter: list[str] | None) -> list[dict]:
    out = entries
    if group_filter:
        out = [e for e in out if e["group"] == group_filter]
    if variant_filter:
        wanted = {v.strip() for v in variant_filter if v.strip()}
        out = [e for e in out if e["variant"] in wanted]
    if track_filter:
        wanted = {t.strip() for t in track_filter if t.strip()}
        out = [e for e in out if e["track_id"] in wanted or e["file_id"] in wanted]
    return out


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true",
                    help="Print what would be baked; spend zero credits.")
    ap.add_argument("--filter", choices=("world", "faction", "storyteller", "boss"),
                    default=None, help="Limit to one group.")
    ap.add_argument("--variants", default=None,
                    help="Comma-separated variants (calm,buildup,climax,debrief).")
    ap.add_argument("--tracks", default=None,
                    help="Comma-separated track_ids or file_ids to bake.")
    ap.add_argument("--limit", type=int, default=None,
                    help="Bake at most this many entries (after filters).")
    ap.add_argument("--resume", action="store_true",
                    help="Skip entries already in progress.completed.")
    ap.add_argument("--reset-progress", action="store_true",
                    help="Wipe progress.json before starting.")
    ap.add_argument("--inter-track-sleep", type=float, default=0.5,
                    help="Seconds to sleep between API calls (default 0.5).")
    args = ap.parse_args()

    entries = _load_prompts()
    entries = _filter_entries(
        entries,
        group_filter=args.filter,
        variant_filter=(args.variants.split(",") if args.variants else None),
        track_filter=(args.tracks.split(",") if args.tracks else None),
    )
    if args.limit:
        entries = entries[: args.limit]

    if args.reset_progress and PROGRESS_PATH.exists():
        PROGRESS_PATH.unlink(missing_ok=True)
    progress = _load_progress()

    if args.resume:
        completed_ids = set(progress["completed"])
        before = len(entries)
        entries = [e for e in entries if e["file_id"] not in completed_ids]
        print(f"[music] resume — skipping {before - len(entries)} already-done")

    print(f"[music] entries to bake = {len(entries)}  dry_run={args.dry_run}")

    if args.dry_run:
        for e in entries:
            _bake_one(client=None, entry=e, dry_run=True)  # type: ignore[arg-type]
        print(f"[music] dry-run complete — {len(entries)} entries previewed")
        return 0

    key = load_elevenlabs_key()
    client = ElevenLabs(api_key=key.value)
    print(f"[music] client ready ({key!r})")

    baked: list[tuple[dict, Path]] = []
    for i, entry in enumerate(entries, start=1):
        ok, path = _bake_one(client, entry, dry_run=False)
        if ok and path is not None:
            baked.append((entry, path))
            progress["completed"].append(entry["file_id"])
        else:
            progress["failed"].append(entry["file_id"])
        _save_progress(progress)
        if i < len(entries):
            time.sleep(args.inter_track_sleep)
        # Periodic ledger flush every 10 tracks so the ledger never lags far
        # behind the on-disk WAVs.
        if baked and i % 10 == 0:
            replaced, inserted, total = apply_superseded_entries(_supersede_records(baked))
            print(
                f"[music] checkpoint ledger update: replaced={replaced} inserted={inserted} total={total}"
            )
            baked = []

    if baked:
        replaced, inserted, total = apply_superseded_entries(_supersede_records(baked))
        print(
            f"[music] final ledger update: replaced={replaced} inserted={inserted} total={total}"
        )

    print(
        f"[music] DONE — completed={len(progress['completed'])}  "
        f"failed={len(progress['failed'])}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
