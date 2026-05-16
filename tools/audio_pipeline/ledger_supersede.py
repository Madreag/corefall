"""Tier 1 → Tier 2 ledger supersede helper.

When a Tier 2 bake (ElevenLabs / AIVA / etc.) overwrites a Tier 1 placeholder
WAV at the same path, the old Tier 1 entry must be REMOVED from the ledger and
the new Tier 2 entry inserted in its place. The Tier 1 entry's `id` was
`blake3(category|canonical_name|Tier1_LLM_Audio)`; the Tier 2 entry's `id`
will be `blake3(category|canonical_name|Tier2_Audio_Production)` — different
ID, so the supersede is a delete-old + insert-new operation, not an update.

Cross-bake safe: multiple bakes can call `apply_superseded_entries()` in any
order; each call merges into ledger.jsonl atomically.
"""

from __future__ import annotations

import contextlib
import fcntl
import json
import os
import sys
import threading
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable, Optional

# Reuse the canonical ledger primitives from asset_gen.
_REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(_REPO_ROOT / "tools" / "asset_gen"))

from ledger_writer import (  # type: ignore[import-not-found]
    LedgerEntryDraft,
    build_entry,
    compute_asset_id,
    hash_path,
)


LEDGER_PATH = _REPO_ROOT / "content" / "asset_ledger" / "ledger.jsonl"
LEDGER_LOCK_PATH = _REPO_ROOT / "content" / "asset_ledger" / ".ledger.lock"

_LEDGER_LOCK = threading.Lock()


@contextlib.contextmanager
def _crossproc_lock(path: Path):
    """Acquire an advisory exclusive flock on `path` (blocks)."""
    path.parent.mkdir(parents=True, exist_ok=True)
    f = open(path, "a+")
    deadline = time.time() + 60.0
    while True:
        try:
            fcntl.flock(f.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
            break
        except BlockingIOError:
            if time.time() > deadline:
                fcntl.flock(f.fileno(), fcntl.LOCK_EX)
                break
            time.sleep(0.1)
    try:
        yield
    finally:
        try:
            fcntl.flock(f.fileno(), fcntl.LOCK_UN)
        finally:
            f.close()


@dataclass
class SupersedeRecord:
    """One Tier 1 → Tier 2 supersede instruction."""

    category: str            # "Audio_SFX" | "Audio_Voice" | "Audio_Music"
    kind: str                # sub-category (e.g., "sfx", "voice_line", "music_loop")
    canonical_name: str
    output_path: Path        # absolute path to the new Tier 2 WAV (already written)
    new_pipeline: str        # e.g., "M12A_eleven_sfx_v1" / "M37A_aiva_playwright_v1"
    new_tool: str            # e.g., "tools/audio_pipeline/eleven_sfx.py"
    new_model: str           # e.g., "eleven_text_to_sound_v2"
    new_model_version: str   # e.g., "v2.0"
    new_workflow: Optional[str] = None
    prompt: str = ""
    seed: int = 0
    old_tier: str = "Tier1_LLM_Audio"
    new_tier: str = "Tier2_Audio_Production"


def _read_all_entries() -> list[dict]:
    if not LEDGER_PATH.exists():
        return []
    out: list[dict] = []
    with LEDGER_PATH.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                out.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    return out


def _atomic_overwrite(entries: Iterable[dict]) -> int:
    LEDGER_PATH.parent.mkdir(parents=True, exist_ok=True)
    sorted_entries = sorted(entries, key=lambda e: str(e.get("id", "")))
    body = "\n".join(
        json.dumps(e, separators=(",", ":"), sort_keys=True) for e in sorted_entries
    )
    if body:
        body += "\n"
    staging = LEDGER_PATH.with_suffix(LEDGER_PATH.suffix + ".tmp")
    staging.write_text(body, encoding="utf-8")
    os.replace(staging, LEDGER_PATH)
    return len(sorted_entries)


def apply_superseded_entries(records: list[SupersedeRecord]) -> tuple[int, int, int]:
    """Apply a batch of Tier-1→Tier-2 supersedes.

    Returns (replaced_count, inserted_count, total_after).
    """
    if not records:
        return (0, 0, 0)
    with _LEDGER_LOCK, _crossproc_lock(LEDGER_LOCK_PATH):
        existing = _read_all_entries()
        # Build (category, canonical_name) → entry index
        idx_by_key: dict[tuple[str, str], int] = {}
        for i, e in enumerate(existing):
            key = (str(e.get("category", "")), str(e.get("canonical_name", "")))
            idx_by_key[key] = i

        replaced = 0
        inserted = 0
        to_drop: set[int] = set()
        new_entries: list[dict] = []
        for r in records:
            key = (r.category, r.canonical_name)
            if key in idx_by_key:
                to_drop.add(idx_by_key[key])
                replaced += 1
            else:
                inserted += 1
            size, blake = hash_path(r.output_path)
            output_path_str = str(r.output_path.resolve())
            draft = LedgerEntryDraft(
                category=r.category,
                kind=r.kind,
                canonical_name=r.canonical_name,
                tier=r.new_tier,
                pipeline=r.new_pipeline,
                prompt=r.prompt,
                seed=int(r.seed),
                output_path=output_path_str,
                output_blake3=blake,
                output_size_bytes=size,
                output_format="wav",
                generator_tool=r.new_tool,
                generator_model=r.new_model,
                generator_model_version=r.new_model_version,
                generator_workflow=r.new_workflow,
            )
            new_entries.append(build_entry(draft))

        merged = [e for i, e in enumerate(existing) if i not in to_drop]
        # New entries can collide with each other if (cat,name) repeats — keep
        # the last one written deterministically.
        seen: dict[str, dict] = {}
        for e in merged + new_entries:
            seen[str(e.get("id", ""))] = e
        total = _atomic_overwrite(seen.values())
        return (replaced, inserted, total)


def add_new_entries(records: list[SupersedeRecord]) -> int:
    """Insert Tier 2 entries without expecting any Tier 1 counterpart.

    Used for voice line bakes (no Tier 1 placeholder existed).
    """
    if not records:
        return 0
    with _LEDGER_LOCK, _crossproc_lock(LEDGER_LOCK_PATH):
        existing = _read_all_entries()
        for r in records:
            size, blake = hash_path(r.output_path)
            output_path_str = str(r.output_path.resolve())
            draft = LedgerEntryDraft(
                category=r.category,
                kind=r.kind,
                canonical_name=r.canonical_name,
                tier=r.new_tier,
                pipeline=r.new_pipeline,
                prompt=r.prompt,
                seed=int(r.seed),
                output_path=output_path_str,
                output_blake3=blake,
                output_size_bytes=size,
                output_format="wav",
                generator_tool=r.new_tool,
                generator_model=r.new_model,
                generator_model_version=r.new_model_version,
                generator_workflow=r.new_workflow,
            )
            existing.append(build_entry(draft))
        seen: dict[str, dict] = {}
        for e in existing:
            seen[str(e.get("id", ""))] = e
        return _atomic_overwrite(seen.values())


__all__ = [
    "SupersedeRecord",
    "apply_superseded_entries",
    "add_new_entries",
    "LEDGER_PATH",
]
