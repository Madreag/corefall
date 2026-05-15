#!/usr/bin/env python3
"""Restore SFX + music ledger entries clobbered by concurrent visual worker.

The SFX (M12A_sfx_v1) and music (M37A_music_v1) WAV files exist on disk at
game/content/audio/{sfx,music}/ but their ledger rows were overwritten by the
visual worker's overwrite_ledger() call. This script rescans the wav dirs,
rebuilds the entries from manifests, and merges them into the ledger.
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

THIS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(THIS_DIR))
sys.path.insert(0, str(THIS_DIR.parent))

from asset_gen import ledger_writer
from sfx_bake import (
    LEDGER_PATH,
    SFX_OUTPUT_DIR,
    collect_entries as collect_sfx_entries,
    derive_id as derive_sfx_id,
)
from music_bake import (
    MANIFEST_PATH as MUSIC_MANIFEST,
    OUTPUT_DIR as MUSIC_OUT,
    build_ledger_entry as build_music_entry,
)


def rebuild_sfx_entries():
    entries = collect_sfx_entries()
    rows = []
    for manifest_name, section_name, entry in entries:
        eid = derive_sfx_id(entry)
        out_path = SFX_OUTPUT_DIR / f"{eid}.wav"
        if not out_path.exists():
            continue
        canonical_name = entry.get("canonical_name") or eid
        prompt_text = str(entry.get("prompt", ""))
        seed = int.from_bytes(eid.encode()[:8], "big") if not entry.get("seed") else int(entry["seed"])
        file_size, file_blake3 = ledger_writer.hash_path(out_path)
        draft = ledger_writer.LedgerEntryDraft(
            category="Audio_SFX",
            kind="sfx",
            canonical_name=canonical_name,
            tier="Tier1_LLM_Audio",
            pipeline="M12A_sfx_v1",
            prompt=prompt_text,
            seed=int(seed),
            output_path=str(out_path.resolve()),
            output_blake3=file_blake3,
            output_size_bytes=int(file_size),
            output_format="wav",
            generator_tool="tools/audio_synth/sfx_bake.py",
            generator_model="procedural-sfx-synth-v1",
            generator_workflow=f"{manifest_name}::{section_name}",
            generator_model_version="1.0.0",
        )
        rows.append(ledger_writer.build_entry(draft))
    return rows


def rebuild_music_entries():
    with open(MUSIC_MANIFEST) as f:
        manifest = json.load(f)
    all_tracks = []
    for cat in ("world_ambient_tracks", "faction_theme_tracks", "storyteller_theme_tracks", "boss_theme_tracks"):
        all_tracks.extend(manifest.get(cat, []))
    rows = []
    for track in all_tracks:
        tid = track["id"]
        for variant_name in ("calm", "buildup", "climax", "debrief"):
            if variant_name not in track["variants"]:
                continue
            canonical_name = f"{tid}_{variant_name}"
            out_path = MUSIC_OUT / f"{canonical_name}.wav"
            if not out_path.exists():
                continue
            seed = int(track["variants"][variant_name]["seed"])
            prompt = track["variants"][variant_name]["musicgen_prompt"]
            row = build_music_entry(canonical_name, out_path, seed, prompt)
            row["category"] = "Audio_Music"
            row["kind"] = "music"
            rows.append(row)
    return rows


def main() -> int:
    print("[restore] rebuilding SFX entries...")
    sfx_rows = rebuild_sfx_entries()
    print(f"[restore] {len(sfx_rows)} SFX entries rebuilt")

    print("[restore] rebuilding music entries...")
    music_rows = rebuild_music_entries()
    print(f"[restore] {len(music_rows)} music entries rebuilt")

    existing = []
    if LEDGER_PATH.exists():
        with LEDGER_PATH.open("r") as f:
            for line in f:
                line = line.strip()
                if line:
                    existing.append(json.loads(line))
    print(f"[restore] existing ledger: {len(existing)} rows")

    existing_ids = {r.get("id") for r in existing}
    merged = list(existing)
    added = 0
    for row in sfx_rows + music_rows:
        if row.get("id") in existing_ids:
            continue
        existing_ids.add(row.get("id"))
        merged.append(row)
        added += 1

    merged.sort(key=lambda r: str(r.get("id", "")))
    ledger_writer.overwrite_ledger(LEDGER_PATH, merged)
    print(f"[restore] merged ledger: {len(merged)} rows (+{added} added)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
