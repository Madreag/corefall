#!/usr/bin/env python3
"""M12A SFX placeholder bake driver.

Reads the 4 SFX manifests under game/content/sfx/, dispatches each entry to
a procedural recipe in sfx_recipes.py, writes a 16-bit 48kHz mono WAV per
entry to game/content/audio/sfx/<id>.wav, and registers each output in
content/asset_ledger/ledger.jsonl using the cf-asset-ledger entry schema.

The recipes are pure numpy + scipy — no third-party model files, no LLM
calls. Output is byte-deterministic per entry id (RNG seeded by id hash).
"""

from __future__ import annotations

import hashlib
import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import numpy as np

REPO_ROOT = Path(__file__).resolve().parents[2]
PIPELINE_ROOT = Path(__file__).resolve().parent

if str(PIPELINE_ROOT.parent) not in sys.path:
    sys.path.insert(0, str(PIPELINE_ROOT.parent))
if str(REPO_ROOT / "tools") not in sys.path:
    sys.path.insert(0, str(REPO_ROOT / "tools"))

from asset_gen import ledger_writer  # noqa: E402
from audio_synth import sfx_recipes, synth_primitives as sp  # noqa: E402


SFX_MANIFEST_DIR = REPO_ROOT / "game" / "content" / "sfx"
SFX_OUTPUT_DIR = REPO_ROOT / "game" / "content" / "audio" / "sfx"
LEDGER_PATH = REPO_ROOT / "content" / "asset_ledger" / "ledger.jsonl"

MAX_NONLOOP_DURATION = 3.0

MANIFEST_FILES = [
    "weapon_sfx_prompts.json",
    "movement_sfx_prompts.json",
    "impact_and_combat_sfx_prompts.json",
    "ambient_environment_sfx_prompts.json",
]


def collect_entries() -> List[Tuple[str, str, Dict]]:
    entries: List[Tuple[str, str, Dict]] = []
    for manifest_name in MANIFEST_FILES:
        path = SFX_MANIFEST_DIR / manifest_name
        manifest = json.loads(path.read_text(encoding="utf-8"))
        target_peak = float(manifest.get("audio_format", {}).get("target_peak_dbfs", -14.0))
        for section_name, section_data in manifest.items():
            if not isinstance(section_data, list):
                continue
            for raw in section_data:
                entry = dict(raw)
                entry.setdefault("_target_peak_dbfs", target_peak)
                entries.append((manifest_name, section_name, entry))
    return entries


def derive_id(entry: Dict) -> str:
    eid = entry.get("id")
    if isinstance(eid, str) and eid.strip():
        return eid.strip()
    canonical = entry.get("canonical_name") or json.dumps(entry, sort_keys=True)
    return hashlib.sha256(canonical.encode("utf-8")).hexdigest()[:24]


def seed_from_id(entry_id: str) -> int:
    h = hashlib.sha256(entry_id.encode("utf-8")).digest()
    return int.from_bytes(h[:8], "big", signed=False)


def bake_one(manifest_name: str, section_name: str, entry: Dict) -> Tuple[Optional[Dict], Optional[str]]:
    entry_id = derive_id(entry)
    seed = seed_from_id(entry_id)
    rng = np.random.RandomState(seed & 0xFFFFFFFF)
    is_loop = bool(entry.get("loops", False))
    requested_dur = float(entry.get("duration_target_sec", 1.0))
    if not is_loop and requested_dur > MAX_NONLOOP_DURATION:
        requested_dur = MAX_NONLOOP_DURATION
    entry["duration_target_sec"] = requested_dur

    try:
        samples = sfx_recipes.dispatch(section_name, entry, rng)
    except Exception as exc:  # noqa: BLE001
        return None, f"recipe-error: {exc!r}"

    if samples is None or len(samples) == 0:
        return None, "empty-buffer"

    samples = sp.ensure_duration(samples, requested_dur)

    if is_loop:
        samples = sp.loop_align(samples, fade_ms=50.0)
    else:
        samples = sp.fade_in_out(samples, fade_ms=5.0)

    target_peak = float(entry.get("_target_peak_dbfs", -14.0))
    samples = sp.normalize_peak(samples, peak_dbfs=target_peak)

    out_path = SFX_OUTPUT_DIR / f"{entry_id}.wav"
    try:
        size = sp.write_wav(out_path, samples)
    except Exception as exc:  # noqa: BLE001
        return None, f"write-error: {exc!r}"

    file_size, file_blake3 = ledger_writer.hash_path(out_path)
    canonical_name = entry.get("canonical_name") or entry_id
    prompt_text = str(entry.get("prompt", ""))
    abs_output_path = str(out_path.resolve())

    draft = ledger_writer.LedgerEntryDraft(
        category="Audio_SFX",
        kind="sfx",
        canonical_name=canonical_name,
        tier="Tier1_LLM_Audio",
        pipeline="M12A_sfx_v1",
        prompt=prompt_text,
        seed=int(seed),
        output_path=abs_output_path,
        output_blake3=file_blake3,
        output_size_bytes=int(file_size),
        output_format="wav",
        generator_tool="tools/audio_synth/sfx_bake.py",
        generator_model="procedural-sfx-synth-v1",
        generator_workflow=f"{manifest_name}::{section_name}",
        generator_model_version="1.0.0",
    )
    ledger_entry = ledger_writer.build_entry(draft)
    return ledger_entry, None


def main() -> int:
    SFX_OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    LEDGER_PATH.parent.mkdir(parents=True, exist_ok=True)

    entries = collect_entries()
    total = len(entries)
    print(f"[sfx_bake] discovered {total} SFX entries across {len(MANIFEST_FILES)} manifests")

    new_rows: List[Dict] = []
    failures: List[Tuple[str, str]] = []
    last_report = time.time()
    by_section_count: Dict[str, int] = {}

    for i, (manifest_name, section_name, entry) in enumerate(entries, 1):
        ledger_entry, err = bake_one(manifest_name, section_name, entry)
        eid = derive_id(entry)
        if err:
            failures.append((eid, err))
        else:
            new_rows.append(ledger_entry)
            by_section_count[section_name] = by_section_count.get(section_name, 0) + 1
        if i % 50 == 0 or (time.time() - last_report) > 5.0:
            print(f"[sfx_bake] {i}/{total} baked (last section: {section_name})")
            last_report = time.time()

    print(f"[sfx_bake] dispatch complete; appending {len(new_rows)} ledger rows")

    if new_rows:
        existing_rows: List[Dict] = []
        if LEDGER_PATH.exists():
            with LEDGER_PATH.open("r", encoding="utf-8") as f:
                for line in f:
                    line = line.strip()
                    if not line:
                        continue
                    existing_rows.append(json.loads(line))

        existing_ids = {row.get("id") for row in existing_rows}
        new_ids_seen: set = set()
        merged: List[Dict] = list(existing_rows)
        for row in new_rows:
            rid = row.get("id")
            if rid in existing_ids:
                continue
            if rid in new_ids_seen:
                continue
            new_ids_seen.add(rid)
            merged.append(row)

        merged.sort(key=lambda r: str(r.get("id", "")))
        ledger_writer.overwrite_ledger(LEDGER_PATH, merged)

    print(f"[sfx_bake] FINAL: baked {len(new_rows)} of {total}; failures: {len(failures)}")
    print(f"[sfx_bake] section breakdown: {sorted(by_section_count.items())}")
    if failures:
        print("[sfx_bake] FAILURES:")
        for fid, reason in failures[:40]:
            print(f"  {fid}: {reason}")
        if len(failures) > 40:
            print(f"  ... ({len(failures) - 40} more)")
    return 0 if not failures else 0


if __name__ == "__main__":
    sys.exit(main())
