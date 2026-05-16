#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import sys
from pathlib import Path

import numpy as np

THIS_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(THIS_DIR))

from music_primitives import (
    SAMPLE_RATE,
    adsr,
    bass_line,
    bell_note,
    chord_freqs,
    drum_pattern,
    fade_in_out,
    melody_line,
    noise_layer,
    normalize,
    organ_note,
    pad_chord,
    parse_key,
    reverb,
    saw_note,
    scale_notes,
    sine_note,
    stereo_pan,
    triangle_note,
    write_stereo,
)

REPO_ROOT = Path("/Users/erol/projects/corefall")
MANIFEST_PATH = REPO_ROOT / "game" / "content" / "sfx" / "music_tracks_prompts.json"
OUTPUT_DIR = REPO_ROOT / "game" / "content" / "audio" / "music"
LEDGER_PATH = REPO_ROOT / "content" / "asset_ledger" / "ledger.jsonl"
LOOP_DURATION_SEC = 60.0
TARGET_PEAK_DBFS = -8.0
FADE_MS = 50.0
PIPELINE = "M37A_music_v1"
GENERATOR_TOOL = "tools/audio_synth/music_bake.py"
GENERATOR_MODEL = "procedural-music-synth-v1"
GENERATOR_VERSION = "1.0.0"

CHORD_PROGRESSIONS = {
    "minor": [0, 5, 2, 6],
    "major": [0, 4, 5, 3],
}

MELODY_PATTERNS = [
    [0, 2, 4, 6, 0, 4, 2, 7, 0, 2, 5, 4, 2, 0, 7, 0],
    [0, 4, 2, 7, 4, 2, 0, 5, 0, 2, 4, 7, 5, 2, 0, 4],
    [7, 5, 4, 2, 0, 2, 4, 5, 7, 5, 4, 2, 0, 4, 2, 0],
    [0, 0, 4, 4, 5, 5, 4, 2, 0, 0, 7, 7, 5, 5, 4, 0],
]

WORLD_FLAVORS = {
    "earth": {"pad_voice": "triangle", "lead_voice": "sine", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "mars": {"pad_voice": "triangle", "lead_voice": "saw", "bass_voice": "triangle", "noise_amp": 0.03, "bass_octave_drop": 2},
    "moon": {"pad_voice": "sine", "lead_voice": "sine", "bass_voice": "triangle", "noise_amp": 0.01, "bass_octave_drop": 2},
    "phobos": {"pad_voice": "sine", "lead_voice": "saw", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "deimos": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.04, "bass_octave_drop": 2},
    "mimas": {"pad_voice": "sine", "lead_voice": "bell", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "europa": {"pad_voice": "sine", "lead_voice": "bell", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 3},
    "vulcan": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.05, "bass_octave_drop": 2},
    "venus": {"pad_voice": "triangle", "lead_voice": "saw", "bass_voice": "triangle", "noise_amp": 0.04, "bass_octave_drop": 2},
    "belt": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.05, "bass_octave_drop": 2},
    "orbital": {"pad_voice": "sine", "lead_voice": "sine", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "sol_zone": {"pad_voice": "organ", "lead_voice": "sine", "bass_voice": "organ", "noise_amp": 0.02, "bass_octave_drop": 2},
}

FACTION_FLAVORS = {
    "coalition": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.03, "bass_octave_drop": 2},
    "frontier": {"pad_voice": "triangle", "lead_voice": "triangle", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "ronin": {"pad_voice": "triangle", "lead_voice": "bell", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "synth": {"pad_voice": "sine", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.03, "bass_octave_drop": 2},
    "collective": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.05, "bass_octave_drop": 2},
    "husks": {"pad_voice": "saw", "lead_voice": "bell", "bass_voice": "saw", "noise_amp": 0.05, "bass_octave_drop": 2},
    "collegium": {"pad_voice": "organ", "lead_voice": "bell", "bass_voice": "organ", "noise_amp": 0.02, "bass_octave_drop": 2},
    "starlight": {"pad_voice": "organ", "lead_voice": "bell", "bass_voice": "organ", "noise_amp": 0.02, "bass_octave_drop": 2},
}

STORYTELLER_FLAVORS = {
    "cassandra_classic": {"pad_voice": "triangle", "lead_voice": "sine", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "phoebe_chillax": {"pad_voice": "sine", "lead_voice": "sine", "bass_voice": "triangle", "noise_amp": 0.01, "bass_octave_drop": 2},
    "randy_random": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.04, "bass_octave_drop": 2},
    "ironman": {"pad_voice": "triangle", "lead_voice": "triangle", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2},
    "sandbox": {"pad_voice": "sine", "lead_voice": "triangle", "bass_voice": "triangle", "noise_amp": 0.01, "bass_octave_drop": 2},
}

BOSS_FLAVORS = {
    "hollow_king": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.05, "bass_octave_drop": 2},
    "frozen_heart": {"pad_voice": "sine", "lead_voice": "bell", "bass_voice": "triangle", "noise_amp": 0.03, "bass_octave_drop": 3},
    "crimson_tide": {"pad_voice": "saw", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.06, "bass_octave_drop": 2},
    "eclipse_walker": {"pad_voice": "sine", "lead_voice": "saw", "bass_voice": "saw", "noise_amp": 0.04, "bass_octave_drop": 2},
    "last_star": {"pad_voice": "organ", "lead_voice": "sine", "bass_voice": "organ", "noise_amp": 0.03, "bass_octave_drop": 2},
}

DEFAULT_FLAVOR = {"pad_voice": "sine", "lead_voice": "saw", "bass_voice": "triangle", "noise_amp": 0.02, "bass_octave_drop": 2}

VARIANT_DRUMS = {
    "calm": ".........k......",
    "buildup": "k...h...s...h...",
    "climax": "k.h.s.h.k.h.s.h.",
    "debrief": "..........k.....",
}

VARIANT_INTENSITY = {
    "calm": {"pad_amp": 0.28, "bass_amp": 0.0, "lead_amp": 0.10, "drum_k": 0.30, "drum_s": 0.20, "drum_h": 0.10, "noise_mul": 0.6, "counter_amp": 0.0, "lead_voice_pan": 0.0},
    "buildup": {"pad_amp": 0.30, "bass_amp": 0.30, "lead_amp": 0.18, "drum_k": 0.45, "drum_s": 0.25, "drum_h": 0.14, "noise_mul": 1.0, "counter_amp": 0.0, "lead_voice_pan": 0.15},
    "climax": {"pad_amp": 0.32, "bass_amp": 0.42, "lead_amp": 0.24, "drum_k": 0.55, "drum_s": 0.34, "drum_h": 0.18, "noise_mul": 1.3, "counter_amp": 0.15, "lead_voice_pan": 0.25},
    "debrief": {"pad_amp": 0.30, "bass_amp": 0.16, "lead_amp": 0.12, "drum_k": 0.25, "drum_s": 0.0, "drum_h": 0.0, "noise_mul": 0.4, "counter_amp": 0.0, "lead_voice_pan": 0.0},
}


def flavor_for_track(track):
    if "world_id" in track:
        wid = track["world_id"]
        if "boss_id" in track:
            return BOSS_FLAVORS.get(track["boss_id"], DEFAULT_FLAVOR)
        return WORLD_FLAVORS.get(wid, DEFAULT_FLAVOR)
    if "faction_id" in track:
        return FACTION_FLAVORS.get(track["faction_id"], DEFAULT_FLAVOR)
    if "storyteller_id" in track:
        return STORYTELLER_FLAVORS.get(track["storyteller_id"], DEFAULT_FLAVOR)
    if "boss_id" in track:
        return BOSS_FLAVORS.get(track["boss_id"], DEFAULT_FLAVOR)
    return DEFAULT_FLAVOR


def select_melody_pattern(seed, root_offset, scale_is_minor):
    rng = np.random.default_rng(int(seed) & 0xFFFFFFFF)
    base = MELODY_PATTERNS[rng.integers(0, len(MELODY_PATTERNS))].copy()
    if rng.random() < 0.4:
        for i in range(len(base)):
            if rng.random() < 0.2:
                base[i] = (base[i] + 1) % 7
    return base


def synthesize_track(track, variant_name):
    variant_data = track["variants"][variant_name]
    seed = int(variant_data["seed"])
    musicgen_prompt = variant_data["musicgen_prompt"]
    bpm = float(track["tempo_bpm"])
    key_str = track["key"]
    root_offset, scale = parse_key(key_str)
    scale_is_minor = "major" not in key_str.lower()
    flavor = flavor_for_track(track)
    intensity = VARIANT_INTENSITY[variant_name]

    total_dur = LOOP_DURATION_SEC

    chord_octave = 3
    progression = CHORD_PROGRESSIONS["minor" if scale_is_minor else "major"]
    beats_per_bar = 4
    sec_per_beat = 60.0 / bpm
    chord_dur = beats_per_bar * sec_per_beat

    n_samples_total = int(total_dur * SAMPLE_RATE)
    mix = np.zeros(n_samples_total)

    cursor = 0
    chord_idx = 0
    while cursor < n_samples_total:
        deg = progression[chord_idx % len(progression)]
        freqs = chord_freqs(root_offset, deg, scale, octave=chord_octave)
        pad = pad_chord(freqs, chord_dur, amp=intensity["pad_amp"], voice=flavor["pad_voice"])
        end = min(cursor + len(pad), n_samples_total)
        mix[cursor:end] += pad[:end - cursor]
        cursor += int(chord_dur * SAMPLE_RATE)
        chord_idx += 1

    freqs_in_key = scale_notes(root_offset, scale, octave_low=3, octave_high=5)

    melody_pattern_idx = select_melody_pattern(seed, root_offset, scale_is_minor)
    note_dur = sec_per_beat
    if variant_name == "calm":
        note_dur = sec_per_beat * 2
    elif variant_name == "buildup":
        note_dur = sec_per_beat
    elif variant_name == "climax":
        note_dur = sec_per_beat * 0.5
    elif variant_name == "debrief":
        note_dur = sec_per_beat * 2

    if intensity["lead_amp"] > 0:
        lead = melody_line(melody_pattern_idx, freqs_in_key, note_dur=note_dur, total_dur=total_dur,
                           amp=intensity["lead_amp"], voice=flavor["lead_voice"])
        mix += lead

    if intensity["counter_amp"] > 0:
        counter_freqs = scale_notes(root_offset, scale, octave_low=4, octave_high=5)
        counter_pattern = list(reversed(melody_pattern_idx))
        counter = melody_line(counter_pattern, counter_freqs, note_dur=note_dur * 0.5,
                              total_dur=total_dur, amp=intensity["counter_amp"], voice=flavor["lead_voice"])
        mix += counter

    if intensity["bass_amp"] > 0:
        bass_pattern_indices = []
        for deg in progression:
            bass_pattern_indices.extend([deg] * 4)
        bass = bass_line(bass_pattern_indices, freqs_in_key, note_dur=sec_per_beat,
                         total_dur=total_dur, amp=intensity["bass_amp"],
                         octave_drop=flavor["bass_octave_drop"], voice=flavor["bass_voice"])
        mix += bass

    pattern = VARIANT_DRUMS[variant_name]
    rng_drums = np.random.default_rng(seed ^ 0xA5A5A5A5)
    drums = drum_pattern(pattern, bpm, total_dur, rng=rng_drums,
                         kick_amp=intensity["drum_k"],
                         snare_amp=intensity["drum_s"],
                         hihat_amp=intensity["drum_h"])
    mix += drums

    rng_noise = np.random.default_rng(seed ^ 0x5A5A5A5A)
    noise = noise_layer(total_dur, amp=flavor["noise_amp"] * intensity["noise_mul"],
                        hp_cutoff_hz=200.0, rng=rng_noise)
    mix += noise

    pan = intensity["lead_voice_pan"]
    left = mix.copy()
    right = mix.copy()

    if intensity["counter_amp"] > 0:
        counter_freqs = scale_notes(root_offset, scale, octave_low=4, octave_high=5)
        counter_pattern = list(reversed(melody_pattern_idx))
        counter_pan = melody_line(counter_pattern, counter_freqs, note_dur=note_dur * 0.5,
                                  total_dur=total_dur, amp=intensity["counter_amp"] * 0.5,
                                  voice=flavor["lead_voice"])
        left += counter_pan * (0.5 - pan * 0.5)
        right += counter_pan * (0.5 + pan * 0.5)

    if intensity["lead_amp"] > 0 and pan != 0.0:
        left = left * (1.0 - pan * 0.1)
        right = right * (1.0 + pan * 0.1)

    left = fade_in_out(left, fade_ms=FADE_MS)
    right = fade_in_out(right, fade_ms=FADE_MS)

    peak = max(np.max(np.abs(left)), np.max(np.abs(right))) if (left.size and right.size) else 0.0
    target = 10 ** (TARGET_PEAK_DBFS / 20.0)
    if peak > 0:
        scale_factor = target / peak
        left = left * scale_factor
        right = right * scale_factor

    return left, right, musicgen_prompt


def hex_id(canonical_name, seed):
    h = hashlib.sha256()
    h.update(canonical_name.encode("utf-8"))
    h.update(b":")
    h.update(str(seed).encode("utf-8"))
    h.update(b":")
    h.update(PIPELINE.encode("utf-8"))
    return h.hexdigest()


def deterministic_iso(canonical_name, seed):
    h = hashlib.sha256()
    h.update(canonical_name.encode("utf-8"))
    h.update(b":")
    h.update(str(seed).encode("utf-8"))
    return "ledger-deterministic:" + h.hexdigest()[:16]


def blake3_of(path):
    try:
        import blake3 as b3
        h = b3.blake3()
        with open(path, "rb") as f:
            while True:
                chunk = f.read(1 << 20)
                if not chunk:
                    break
                h.update(chunk)
        return h.hexdigest()
    except ImportError:
        h = hashlib.sha256()
        with open(path, "rb") as f:
            while True:
                chunk = f.read(1 << 20)
                if not chunk:
                    break
                h.update(chunk)
        return h.hexdigest()


def build_ledger_entry(canonical_name, out_path, seed, prompt):
    file_size = os.path.getsize(out_path)
    out_blake3 = blake3_of(out_path)
    entry_id = hex_id(canonical_name, seed)
    return {
        "canonical_name": canonical_name,
        "category": "Music",
        "generated_at_iso": deterministic_iso(canonical_name, seed),
        "generated_by_human": False,
        "generated_on_machine": "deterministic",
        "generator": {
            "model": GENERATOR_MODEL,
            "model_version": GENERATOR_VERSION,
            "tool": GENERATOR_TOOL,
        },
        "id": entry_id,
        "kind": "music",
        "license": "CC0",
        "output_blake3": out_blake3,
        "output_format": "wav",
        "output_path": str(out_path),
        "output_size_bytes": file_size,
        "package_source": "Vanilla",
        "pipeline": PIPELINE,
        "prompt": prompt,
        "regen_command": f"cf-mod ledger regenerate {entry_id}",
        "regen_inputs": [],
        "schema_version": "1.0.0",
        "seed": seed,
        "tier": "Tier1_Audio_Placeholder",
        "upstream_assets": [],
    }


def main():
    parser = argparse.ArgumentParser(description="M37A music placeholder bake")
    parser.add_argument("--manifest", default=str(MANIFEST_PATH))
    parser.add_argument("--out-dir", default=str(OUTPUT_DIR))
    parser.add_argument("--ledger", default=str(LEDGER_PATH))
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--filter-id", default=None, help="Only bake tracks whose id contains this string")
    parser.add_argument("--filter-variant", default=None, help="Only bake one variant name")
    args = parser.parse_args()

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    with open(args.manifest) as f:
        manifest = json.load(f)

    all_tracks = []
    for cat in ("world_ambient_tracks", "faction_theme_tracks", "storyteller_theme_tracks", "boss_theme_tracks"):
        all_tracks.extend(manifest.get(cat, []))

    ledger_entries = []
    failures = []
    rendered_count = 0
    total_size = 0

    for track in all_tracks:
        tid = track["id"]
        if args.filter_id and args.filter_id not in tid:
            continue
        for variant_name in ("calm", "buildup", "climax", "debrief"):
            if variant_name not in track["variants"]:
                continue
            if args.filter_variant and args.filter_variant != variant_name:
                continue
            canonical_name = f"{tid}_{variant_name}"
            out_path = out_dir / f"{canonical_name}.wav"
            try:
                left, right, prompt = synthesize_track(track, variant_name)
                if args.dry_run:
                    print(f"[DRY] {canonical_name}: would write {len(left)} samples / channel")
                else:
                    write_stereo(str(out_path), left, right, sample_rate=SAMPLE_RATE)
                    seed = int(track["variants"][variant_name]["seed"])
                    entry = build_ledger_entry(canonical_name, out_path, seed, prompt)
                    ledger_entries.append(entry)
                    rendered_count += 1
                    total_size += os.path.getsize(out_path)
                    print(f"[OK] {canonical_name} ({os.path.getsize(out_path)} bytes)")
            except Exception as e:
                failures.append((canonical_name, repr(e)))
                print(f"[FAIL] {canonical_name}: {e!r}")

    if not args.dry_run and ledger_entries:
        ledger_path = Path(args.ledger)
        ledger_path.parent.mkdir(parents=True, exist_ok=True)
        with open(ledger_path, "a") as f:
            for entry in ledger_entries:
                f.write(json.dumps(entry, sort_keys=True) + "\n")

    print(f"\nRendered: {rendered_count} WAVs")
    print(f"Total size: {total_size / (1024*1024):.1f} MB")
    print(f"Failures: {len(failures)}")
    for fid, err in failures:
        print(f"  FAIL {fid}: {err}")
    return 0 if not failures else 1


if __name__ == "__main__":
    sys.exit(main())
