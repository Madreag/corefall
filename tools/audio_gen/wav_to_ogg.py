"""M12A § WAV → OGG Vorbis conversion.

Per spec § Architecture rules:
> OGG Vorbis as game format — small file size, broad codec support,
> deterministic decode. Master WAV stored offline for re-baking.

Per spec § Acceptance criteria:
> Full SFX bake from scratch / Then 1200+ OGG Vorbis files generated.

This module converts every WAV under `game/content/audio/sfx/` to a
sibling `.ogg` (libvorbis encoded via libsndfile). The WAV stays on
disk as the master archive; the ledger entry's `output_path` switches
to the OGG and the WAV is registered as an `additional_outputs` row
with label `wav_master`.

cf-audio's runtime loader (via `bevy_audio` + the `vorbis` feature
already enabled in `game/Cargo.toml`) consumes OGG Vorbis directly.
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import soundfile as sf

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[1]
sys.path.insert(0, str(_HERE))

# Use the spec-canonical sibling without sys.path pollution.
import importlib.util as _ilu

_LEDGER_WRITER_PATH = _HERE / "ledger_writer.py"
_spec = _ilu.spec_from_file_location("_m12a_ledger_writer_for_ogg", _LEDGER_WRITER_PATH)
assert _spec is not None and _spec.loader is not None
_ledger_writer = _ilu.module_from_spec(_spec)
sys.modules["_m12a_ledger_writer_for_ogg"] = _ledger_writer
_spec.loader.exec_module(_ledger_writer)

SFX_DIR = _REPO_ROOT / "game" / "content" / "audio" / "sfx"
VOICE_DIR = _REPO_ROOT / "game" / "content" / "audio" / "voice"
MUSIC_DIR = _REPO_ROOT / "game" / "content" / "audio" / "music"


@dataclass
class OggConversionStats:
    converted: int = 0
    skipped_existing: int = 0
    failed: list[tuple[str, str]] = None  # type: ignore[assignment]

    def __post_init__(self) -> None:
        if self.failed is None:
            self.failed = []


def convert_wav_to_ogg(wav_path: Path, ogg_path: Path, force: bool = False) -> bool:
    """Convert one WAV to OGG Vorbis. Returns True on success.

    Idempotent: when `ogg_path` already exists and is newer than the WAV,
    skip unless `force=True`. The encoder is deterministic for fixed
    `subtype='VORBIS'`.

    Long stereo WAVs (multi-MB music) are encoded in 1-second chunks
    via the streaming `SoundFile` API to avoid libsndfile's internal
    vorbis_analysis_wrote stack overflow on large single-call submits.
    """
    try:
        if ogg_path.exists() and not force:
            wav_mtime = wav_path.stat().st_mtime
            ogg_mtime = ogg_path.stat().st_mtime
            if ogg_mtime >= wav_mtime:
                return False
        ogg_path.parent.mkdir(parents=True, exist_ok=True)
        chunk_seconds = 1.0
        with sf.SoundFile(str(wav_path), mode="r") as src:
            sr = src.samplerate
            channels = src.channels
            chunk_frames = max(1024, int(sr * chunk_seconds))
            with sf.SoundFile(
                str(ogg_path),
                mode="w",
                samplerate=sr,
                channels=channels,
                format="OGG",
                subtype="VORBIS",
            ) as dst:
                while True:
                    block = src.read(chunk_frames, dtype="int16", always_2d=False)
                    if len(block) == 0:
                        break
                    floats = (block.astype(np.float32) / 32768.0).clip(-1.0, 1.0)
                    dst.write(floats)
        return True
    except Exception as exc:
        # Clean up partial output on failure so callers don't load a
        # truncated OGG.
        try:
            if ogg_path.exists() and ogg_path.stat().st_size < 200:
                ogg_path.unlink(missing_ok=True)
        except OSError:
            pass
        raise RuntimeError(f"convert failed: {exc}") from exc


def convert_directory(directory: Path, force: bool = False, verbose: bool = False) -> OggConversionStats:
    stats = OggConversionStats()
    if not directory.exists():
        return stats
    wavs = sorted(directory.glob("*.wav"))
    for i, wav in enumerate(wavs):
        ogg = wav.with_suffix(".ogg")
        try:
            wrote = convert_wav_to_ogg(wav, ogg, force=force)
            if wrote:
                stats.converted += 1
            else:
                stats.skipped_existing += 1
        except Exception as exc:
            stats.failed.append((wav.name, str(exc)))
        if verbose and (i + 1) % 100 == 0:
            print(f"[wav_to_ogg] {i + 1}/{len(wavs)} processed", file=sys.stderr)
    return stats


def update_ledger_to_ogg() -> int:
    """Rewrite Audio_* ledger rows so:
    - `output_path` points to the OGG file (canonical runtime format).
    - `output_format` is `ogg`.
    - `output_size_bytes` + `output_blake3` reflect the OGG file.
    - The original WAV is recorded as an `additional_outputs` row with
      label `wav_master` so re-baking remains possible.

    Idempotent: rows whose `output_path` already ends in `.ogg` are
    left untouched.
    """
    rows = _ledger_writer.read_existing_entries()
    if not rows:
        return 0
    updated = 0
    out: list[dict] = []
    for row in rows:
        category = row.get("category", "")
        if not category.startswith("Audio_"):
            out.append(row)
            continue
        path = Path(row.get("output_path", ""))
        if path.suffix.lower() == ".ogg":
            out.append(row)
            continue
        wav_path = path
        ogg_path = wav_path.with_suffix(".ogg")
        if not ogg_path.exists():
            # No OGG to point at; keep the WAV row.
            out.append(row)
            continue
        size, blake = _ledger_writer.hash_path(ogg_path)
        wav_size, wav_blake = (None, None)
        if wav_path.exists():
            wav_size, wav_blake = _ledger_writer.hash_path(wav_path)
        new_row = dict(row)
        new_row["output_path"] = str(ogg_path.resolve())
        new_row["output_format"] = "ogg"
        new_row["output_size_bytes"] = size
        new_row["output_blake3"] = blake
        # Preserve the WAV master in additional_outputs.
        additional = list(new_row.get("additional_outputs") or [])
        if wav_size is not None and wav_blake is not None:
            additional = [
                a for a in additional
                if a.get("label") != "wav_master"
            ]
            additional.append({
                "label": "wav_master",
                "output_path": str(wav_path.resolve()),
                "blake3": wav_blake,
                "size_bytes": wav_size,
            })
        new_row["additional_outputs"] = additional
        # Recompute the id since (category, name, tier) is unchanged but
        # the ledger's identity is by the canonical_name + category +
        # tier triple; we keep the existing id stable to preserve replay
        # parity.
        out.append(new_row)
        updated += 1
    # Re-derive the ledger by overwriting.
    _ledger_writer.overwrite_ledger(_ledger_writer.LEDGER_PATH, out)
    return updated


def main() -> int:
    ap = argparse.ArgumentParser(description="M12A WAV → OGG Vorbis conversion.")
    ap.add_argument("--force", action="store_true", help="Re-encode even when OGG is newer than WAV.")
    ap.add_argument("--skip-ledger-update", action="store_true", help="Convert files only; don't rewrite the ledger.")
    ap.add_argument("--only", choices=("sfx", "voice", "music", "all"), default="all")
    args = ap.parse_args()

    targets: list[Path] = []
    if args.only in ("all", "sfx"):
        targets.append(SFX_DIR)
    if args.only in ("all", "voice"):
        targets.append(VOICE_DIR)
    if args.only in ("all", "music"):
        targets.append(MUSIC_DIR)

    total_converted = 0
    total_skipped = 0
    total_failed: list[tuple[str, str]] = []
    for d in targets:
        print(f"[wav_to_ogg] converting {d}", file=sys.stderr)
        stats = convert_directory(d, force=args.force, verbose=True)
        total_converted += stats.converted
        total_skipped += stats.skipped_existing
        total_failed.extend(stats.failed)
    print(
        f"[wav_to_ogg] converted={total_converted} skipped={total_skipped} failed={len(total_failed)}",
        file=sys.stderr,
    )
    if total_failed:
        for name, err in total_failed[:10]:
            print(f"  FAIL {name}: {err}", file=sys.stderr)

    if not args.skip_ledger_update:
        print("[wav_to_ogg] rewriting ledger Audio_* rows to point at OGG…", file=sys.stderr)
        updated = update_ledger_to_ogg()
        print(f"[wav_to_ogg] ledger rows updated: {updated}", file=sys.stderr)

    return 0 if not total_failed else 1


if __name__ == "__main__":
    raise SystemExit(main())
