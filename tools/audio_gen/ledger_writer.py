"""M12A § Spec-canonical ledger writer.

Per spec § Files:
> `tools/audio_gen/ledger_writer.py` (NEW) — Writes via cf-asset-ledger.

This module is the spec-canonical entry point for cf-asset-ledger writes
from the M12A audio pipeline. To avoid the Python import-system clash
between `tools/audio_gen/ledger_writer.py` and the pre-existing
`tools/asset_gen/ledger_writer.py`, we load the underlying asset-gen
implementation via `importlib.util.spec_from_file_location` (which
binds an absolute path) rather than via `sys.path` resolution.

The audio-pipeline supersede helpers from
`tools/audio_pipeline/ledger_supersede.py` are imported the same way.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

_HERE = Path(__file__).resolve().parent
_REPO_ROOT = _HERE.parents[1]
_ASSET_GEN_LEDGER_WRITER = _REPO_ROOT / "tools" / "asset_gen" / "ledger_writer.py"
_AUDIO_PIPELINE_SUPERSEDE = _REPO_ROOT / "tools" / "audio_pipeline" / "ledger_supersede.py"


def _load_module_by_path(name: str, path: Path):
    """Load a Python module from an absolute path without polluting
    `sys.path` (which would shadow this module's own name)."""
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise ImportError(f"failed to build spec for {name} at {path}")
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


_asset_ledger = _load_module_by_path("_cf_asset_ledger_writer", _ASSET_GEN_LEDGER_WRITER)
_audio_supersede = _load_module_by_path("_cf_audio_ledger_supersede", _AUDIO_PIPELINE_SUPERSEDE)


# Asset-ledger primitives.
LedgerEntryDraft = _asset_ledger.LedgerEntryDraft
build_entry = _asset_ledger.build_entry
overwrite_ledger = _asset_ledger.overwrite_ledger
hash_path = _asset_ledger.hash_path
compute_asset_id = _asset_ledger.compute_asset_id

# Audio-pipeline supersede primitives.
SupersedeRecord = _audio_supersede.SupersedeRecord
apply_superseded_entries = _audio_supersede.apply_superseded_entries
add_new_entries = _audio_supersede.add_new_entries
LEDGER_PATH = _audio_supersede.LEDGER_PATH


def read_existing_entries() -> list[dict]:
    """Read every entry currently in `ledger.jsonl`."""
    import json

    out: list[dict] = []
    if not LEDGER_PATH.exists():
        return out
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


def count_audio_entries() -> dict[str, int]:
    """Return per-Audio_* category counts (SFX / Voice / Music)."""
    counts = {"Audio_SFX": 0, "Audio_Voice": 0, "Audio_Music": 0}
    for entry in read_existing_entries():
        cat = entry.get("category", "")
        if cat in counts:
            counts[cat] += 1
    return counts


__all__ = [
    "LEDGER_PATH",
    "LedgerEntryDraft",
    "SupersedeRecord",
    "add_new_entries",
    "apply_superseded_entries",
    "build_entry",
    "compute_asset_id",
    "count_audio_entries",
    "hash_path",
    "overwrite_ledger",
    "read_existing_entries",
]
