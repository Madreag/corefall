#!/usr/bin/env python3
"""M9A Tier-1 SVG placeholder pipeline — orchestrator.

Usage (from repo root):

    tools/asset_gen/.venv/bin/python tools/asset_gen/build_placeholders.py --all
    tools/asset_gen/.venv/bin/python tools/asset_gen/build_placeholders.py --check
    tools/asset_gen/.venv/bin/python tools/asset_gen/build_placeholders.py --category WeaponSprite
    tools/asset_gen/.venv/bin/python tools/asset_gen/build_placeholders.py --report

Modes:

- `--all` — full bake. Reads every asset manifest under
  `tools/asset_gen/asset_manifests/`, expands across origin / stance /
  faction / size axes, writes SVG + PNG to `content/assets/placeholders/`,
  emits one cf-asset-ledger entry per asset to `content/asset_ledger/ledger.jsonl`.
- `--check` — dry-run; counts stale + missing entries vs the existing
  ledger and reports without writing.
- `--category <X>` — bake only entries in category X.
- `--report` — print a summary table of (category, planned, on-disk, in-ledger).

The pipeline is deterministic: same manifests + same palettes + same seed
salt produce byte-identical output. Determinism is the cf-asset-ledger
contract — the freeze-then-store fallback at M9A is procedural composition
in `llm_svg_prompter.py`.
"""

from __future__ import annotations

import argparse
import io
import json
import multiprocessing as mp
import os
import re
import sys
import time
from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Iterable, List, Optional, Tuple

REPO_ROOT = Path(__file__).resolve().parents[2]
PIPELINE_ROOT = Path(__file__).resolve().parent
MANIFEST_ROOT = PIPELINE_ROOT / "asset_manifests"
CONTENT_ROOT = REPO_ROOT / "content"
PLACEHOLDER_ROOT = CONTENT_ROOT / "assets" / "placeholders"
LEDGER_PATH = CONTENT_ROOT / "asset_ledger" / "ledger.jsonl"

# Make the tools/asset_gen package importable without installing it.
if str(PIPELINE_ROOT.parent) not in sys.path:
    sys.path.insert(0, str(PIPELINE_ROOT.parent))

from asset_gen import (  # noqa: E402
    cairo_renderer,
    ledger_writer,
    llm_svg_prompter,
    palette_loader,
    style_enforcer,
)


# ─── Manifest parser (RON-lite for declarative asset entries) ──────────────


# The manifests are RON files but only use scalar primitives + lists + tuples
# + nested structs. We hand-roll a tiny parser instead of pulling in a RON
# Python package — this keeps the runtime dep list to (cairosvg, Pillow,
# blake3).
_TOKEN_RE = re.compile(
    r'\s*(?P<lp>\()|'
    r'\s*(?P<rp>\))|'
    r'\s*(?P<lb>\[)|'
    r'\s*(?P<rb>\])|'
    r'\s*,|'
    r'\s*(?P<colon>:)|'
    r'\s*(?P<str>"(?:[^"\\]|\\.)*")|'
    r'\s*(?P<num>-?\d+(?:\.\d+)?)|'
    r'\s*(?P<bool>true|false)|'
    r'\s*(?P<ident>[A-Za-z_][A-Za-z_0-9]*)|'
    r'\s*(?P<comment>//[^\n]*\n)',
    re.MULTILINE,
)


def _ron_strip_comments(text: str) -> str:
    out: List[str] = []
    for line in text.split("\n"):
        if "//" in line:
            line = line.split("//", 1)[0]
        out.append(line)
    return "\n".join(out)


def _ron_tokenize(text: str) -> List[Tuple[str, str]]:
    tokens: List[Tuple[str, str]] = []
    i = 0
    n = len(text)
    while i < n:
        ch = text[i]
        if ch.isspace() or ch == ",":
            i += 1
            continue
        if ch in "()[]:":
            tokens.append((ch, ch))
            i += 1
            continue
        if ch == '"':
            j = i + 1
            while j < n:
                if text[j] == "\\" and j + 1 < n:
                    j += 2
                    continue
                if text[j] == '"':
                    break
                j += 1
            tokens.append(("str", text[i + 1:j]))
            i = j + 1
            continue
        if ch.isdigit() or ch == "-":
            j = i
            while j < n and (text[j].isdigit() or text[j] in ".-"):
                j += 1
            tokens.append(("num", text[i:j]))
            i = j
            continue
        if ch.isalpha() or ch == "_":
            j = i
            while j < n and (text[j].isalnum() or text[j] == "_"):
                j += 1
            ident = text[i:j]
            if ident == "true":
                tokens.append(("bool", "true"))
            elif ident == "false":
                tokens.append(("bool", "false"))
            else:
                tokens.append(("ident", ident))
            i = j
            continue
        raise ValueError(f"manifest parser: unexpected char {ch!r} at offset {i}")
    return tokens


class _RonParser:
    def __init__(self, tokens: List[Tuple[str, str]]):
        self.tokens = tokens
        self.pos = 0

    def peek(self) -> Optional[Tuple[str, str]]:
        return self.tokens[self.pos] if self.pos < len(self.tokens) else None

    def eat(self, expected_kind: Optional[str] = None) -> Tuple[str, str]:
        if self.pos >= len(self.tokens):
            raise ValueError("manifest parser: unexpected EOF")
        tok = self.tokens[self.pos]
        if expected_kind and tok[0] != expected_kind:
            raise ValueError(f"manifest parser: expected {expected_kind}, got {tok}")
        self.pos += 1
        return tok

    def parse_value(self) -> object:
        tok = self.peek()
        if tok is None:
            raise ValueError("manifest parser: unexpected EOF in value")
        kind, _val = tok
        if kind == "(":
            return self._parse_struct()
        if kind == "[":
            return self._parse_list()
        if kind == "str":
            self.eat()
            return _val
        if kind == "num":
            self.eat()
            if "." in _val:
                return float(_val)
            return int(_val)
        if kind == "bool":
            self.eat()
            return _val == "true"
        if kind == "ident":
            # Treat as bare identifier (e.g. enum-like). Convert to string.
            self.eat()
            return _val
        raise ValueError(f"manifest parser: bad value start {tok}")

    def _parse_struct(self) -> Dict[str, object]:
        self.eat("(")
        out: Dict[str, object] = {}
        while True:
            tok = self.peek()
            if tok is None:
                raise ValueError("manifest parser: unexpected EOF in struct")
            if tok[0] == ")":
                self.eat()
                return out
            if tok[0] != "ident":
                raise ValueError(f"manifest parser: expected ident in struct, got {tok}")
            name = self.eat("ident")[1]
            self.eat(":")
            value = self.parse_value()
            out[name] = value

    def _parse_list(self) -> List[object]:
        self.eat("[")
        items: List[object] = []
        while True:
            tok = self.peek()
            if tok is None:
                raise ValueError("manifest parser: unexpected EOF in list")
            if tok[0] == "]":
                self.eat()
                return items
            items.append(self.parse_value())


def load_manifest(path: Path) -> Dict[str, object]:
    text = path.read_text(encoding="utf-8")
    text = _ron_strip_comments(text)
    tokens = _ron_tokenize(text)
    return _RonParser(tokens)._parse_struct()  # type: ignore[return-value]


# ─── Per-entry job ──────────────────────────────────────────────────────────


@dataclass(frozen=True)
class BakeJob:
    category: str
    canonical_name: str
    kind: str
    prompt: str
    faction: Optional[str]
    origin: Optional[str]
    stance: Optional[str]
    facing: Optional[str]
    variant: Optional[str]
    weight_class: Optional[str]
    module_state: Optional[str]
    integrity_band: Optional[str]
    overlay_mode: Optional[str]
    size: int
    palette_ref: str
    seed: int


def _seed_for(name: str, salt: int = 0xC0FE_FA11) -> int:
    h = 0
    for ch in name:
        h = (h * 131 + ord(ch)) & 0xFFFFFFFFFFFFFFFF
    return (h ^ salt) & 0xFFFFFFFFFFFFFFFF


# ─── Job factory per category ──────────────────────────────────────────────


def _weapon_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    variants = manifest.get("variants", ["side", "muzzle-flash", "magazine-attached"])
    if not isinstance(variants, list):
        variants = ["side", "muzzle-flash", "magazine-attached"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "weapon-side"))
        faction = str(entry.get("faction", "hostile_corp"))
        for variant in ["side", "muzzle-flash", "magazine-attached"]:
            canonical = f"{name}_{variant}".replace("-", "_")
            jobs.append(BakeJob(
                category="WeaponSprite",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({variant} variant)",
                faction=faction,
                origin=None,
                stance=None,
                facing="right",
                variant=variant,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=256,
                palette_ref=faction,
                seed=_seed_for(canonical),
            ))
    return jobs


def _actor_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    origins = manifest.get("origins")
    if not isinstance(origins, list) or not origins:
        origins = ["human"]
    stances = manifest.get("stances")
    if not isinstance(stances, list) or not stances:
        stances = ["idle", "walking", "running", "crouching", "prone", "jetting", "climbing"]
    facings = manifest.get("facings")
    if not isinstance(facings, list) or not facings:
        facings = ["right"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "actor-stance"))
        faction = str(entry.get("faction", "hostile_corp"))
        for origin in origins:
            origin_s = str(origin)
            for stance in stances:
                stance_s = str(stance)
                for facing in facings:
                    facing_s = str(facing)
                    canonical = f"{name}_{origin_s}_{stance_s}_{facing_s}"
                    jobs.append(BakeJob(
                        category="ActorSprite",
                        canonical_name=canonical,
                        kind=kind,
                        prompt=f"{prompt} ({origin_s}, {stance_s})",
                        faction=faction,
                        origin=origin_s,
                        stance=stance_s,
                        facing=facing_s,
                        variant=None,
                        weight_class=None,
                        module_state=None,
                        integrity_band=None,
                        overlay_mode=None,
                        size=128,
                        palette_ref=faction,
                        seed=_seed_for(canonical),
                    ))
    return jobs


def _vehicle_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    variants = manifest.get("variants")
    if not isinstance(variants, list) or not variants:
        variants = ["side", "boarding", "boarded"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "vehicle-side"))
        faction = str(entry.get("faction", "hostile_corp"))
        for variant in variants:
            v_s = str(variant)
            canonical = f"{name}_{v_s}"
            jobs.append(BakeJob(
                category="VehicleSprite",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({v_s})",
                faction=faction,
                origin=None,
                stance=None,
                facing="right",
                variant=v_s,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=256,
                palette_ref=faction,
                seed=_seed_for(canonical),
            ))
    return jobs


def _chassis_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    weight_classes = manifest.get("weight_classes")
    if not isinstance(weight_classes, list) or not weight_classes:
        weight_classes = ["light", "medium", "heavy", "super_heavy"]
    facings = manifest.get("facings")
    if not isinstance(facings, list) or not facings:
        facings = ["right", "left"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "chassis-silhouette"))
        for weight in weight_classes:
            w_s = str(weight)
            for facing in facings:
                f_s = str(facing)
                canonical = f"{name}_{w_s}_{f_s}"
                jobs.append(BakeJob(
                    category="ChassisSprite",
                    canonical_name=canonical,
                    kind=kind,
                    prompt=f"{prompt} ({w_s}, facing {f_s})",
                    faction="mercenary_guild",
                    origin=None,
                    stance=None,
                    facing=f_s,
                    variant=None,
                    weight_class=w_s,
                    module_state=None,
                    integrity_band=None,
                    overlay_mode=None,
                    size=256,
                    palette_ref="mercenary_guild",
                    seed=_seed_for(canonical),
                ))
    return jobs


def _base_module_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    module_states = manifest.get("module_states")
    if not isinstance(module_states, list) or not module_states:
        module_states = ["nominal", "degraded", "warning", "failed"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "base-module"))
        faction = str(entry.get("faction", "mercenary_guild"))
        for state in module_states:
            s_s = str(state)
            canonical = f"{name}_{s_s}"
            jobs.append(BakeJob(
                category="BaseModuleSprite",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({s_s} state)",
                faction=faction,
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=s_s,
                integrity_band=None,
                overlay_mode=None,
                size=192,
                palette_ref=faction,
                seed=_seed_for(canonical),
            ))
    return jobs


def _ui_icon_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    sizes = manifest.get("sizes")
    if not isinstance(sizes, list) or not sizes:
        sizes = [16, 32, 64, 128, 256]
    sizes_int = [int(s) for s in sizes]
    faction_variants = manifest.get("faction_variants")
    if not isinstance(faction_variants, list):
        faction_variants = []
    faction_set = {str(x) for x in faction_variants}
    factions = ["hostile_corp", "allied_resistance", "marauder_tribes", "religious_order",
                "scientist_order", "mercenary_guild", "pirates", "drone_collective"]

    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "ui-icon"))
        for sz in sizes_int:
            canonical = f"{name}_{sz}px"
            jobs.append(BakeJob(
                category="UiIcon",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({sz}px)",
                faction="scientist_order",  # default UI palette
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=sz,
                palette_ref="scientist_order",
                seed=_seed_for(canonical),
            ))
        if name in faction_set:
            for faction in factions:
                for sz in sizes_int:
                    canonical = f"{name}_{faction}_{sz}px"
                    jobs.append(BakeJob(
                        category="UiIcon",
                        canonical_name=canonical,
                        kind=kind,
                        prompt=f"{prompt} ({faction} variant, {sz}px)",
                        faction=faction,
                        origin=None,
                        stance=None,
                        facing="right",
                        variant=None,
                        weight_class=None,
                        module_state=None,
                        integrity_band=None,
                        overlay_mode=None,
                        size=sz,
                        palette_ref=faction,
                        seed=_seed_for(canonical),
                    ))
    return jobs


def _material_jobs(manifest: Dict[str, object], palettes: Dict[str, palette_loader.Palette]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    bands = manifest.get("integrity_bands")
    if not isinstance(bands, list) or not bands:
        bands = ["pristine", "scratched", "cracked", "critical", "destroyed"]
    modes = manifest.get("overlay_modes")
    if not isinstance(modes, list) or not modes:
        modes = ["integrity", "pathability", "mobility", "hazard", "build_repair"]
    mat_bands = palette_loader.material_bands(palettes)
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "material-swatch"))
        band_colors = mat_bands.get(name, ["#888888", "#777777", "#666666", "#555555", "#444444"])
        for idx, band in enumerate(bands):
            b_s = str(band)
            canonical = f"{name}_{b_s}"
            jobs.append(BakeJob(
                category="MaterialSwatch",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({b_s} band)",
                faction="mercenary_guild",
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band=b_s,
                overlay_mode=None,
                size=128,
                palette_ref="materials",
                seed=_seed_for(canonical),
            ))
        for mode in modes:
            m_s = str(mode)
            canonical = f"{name}_overlay_{m_s}"
            jobs.append(BakeJob(
                category="MaterialSwatch",
                canonical_name=canonical,
                kind=kind + "-overlay",
                prompt=f"{prompt} (overlay tint: {m_s})",
                faction="mercenary_guild",
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band="pristine",
                overlay_mode=m_s,
                size=128,
                palette_ref="materials",
                seed=_seed_for(canonical),
            ))
    return jobs


def _particle_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    phases = manifest.get("phases")
    if not isinstance(phases, list) or not phases:
        phases = ["spawn", "mid", "late", "dissipate"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "particle"))
        for phase in phases:
            p_s = str(phase)
            canonical = f"{name}_{p_s}"
            jobs.append(BakeJob(
                category="Particle",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({p_s})",
                faction="hostile_corp",
                origin=None,
                stance=None,
                facing="right",
                variant=p_s,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=96,
                palette_ref="hostile_corp",
                seed=_seed_for(canonical),
            ))
    return jobs


def _terrain_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    variants = manifest.get("variants")
    if not isinstance(variants, list) or not variants:
        variants = ["a", "b", "c", "d", "e"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "terrain-tile"))
        for variant in variants:
            v_s = str(variant)
            canonical = f"{name}_{v_s}"
            jobs.append(BakeJob(
                category="TerrainTile",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} (variant {v_s})",
                faction="mercenary_guild",
                origin=None,
                stance=None,
                facing="right",
                variant=v_s,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=64,
                palette_ref="materials",
                seed=_seed_for(canonical),
            ))
    return jobs


def _cosmetic_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    factions = manifest.get("factions")
    if not isinstance(factions, list) or not factions:
        factions = ["hostile_corp", "allied_resistance", "marauder_tribes", "religious_order",
                    "scientist_order", "mercenary_guild", "pirates", "drone_collective"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "cosmetic-stub"))
        for faction in factions:
            f_s = str(faction)
            canonical = f"{name}_{f_s}"
            jobs.append(BakeJob(
                category="Cosmetic",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({f_s})",
                faction=f_s,
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=128,
                palette_ref=f_s,
                seed=_seed_for(canonical),
            ))
    emblems = manifest.get("faction_emblems")
    if isinstance(emblems, list):
        for emb in emblems:
            e_s = str(emb)
            # Pick faction from name prefix
            faction = "_".join(e_s.split("_")[:-1])
            jobs.append(BakeJob(
                category="FactionEmblem",
                canonical_name=e_s,
                kind="faction-emblem",
                prompt=f"faction emblem {e_s}",
                faction=faction,
                origin=None,
                stance=None,
                facing="right",
                variant="full" if e_s.endswith("_full") else "simple",
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=256,
                palette_ref=faction,
                seed=_seed_for(e_s),
            ))
    overlays = manifest.get("capture_grid_overlays")
    if isinstance(overlays, list):
        for ov in overlays:
            o_s = str(ov)
            jobs.append(BakeJob(
                category="CaptureGridOverlay",
                canonical_name=o_s,
                kind="capture-overlay",
                prompt=f"capture grid overlay {o_s}",
                faction="mercenary_guild",
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=256,
                palette_ref="mercenary_guild",
                seed=_seed_for(o_s),
            ))
    return jobs


def _shell_ui_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """M11A shell-ui manifest expander.

    Iterates entries × sizes × (optional faction variants for selected entries).
    Default size set: [256, 512, 1024]. faction_variants list (if present in
    manifest) gets the 8-faction per-entry recolor expansion.
    """
    jobs: List[BakeJob] = []
    sizes = manifest.get("sizes")
    if not isinstance(sizes, list) or not sizes:
        sizes = [512]
    sizes_int = [int(s) for s in sizes]
    # Only bake the smaller two sizes by default so the bake stays under a
    # few thousand entries; 1024+ are big and slow + rarely needed.
    sizes_int = [s for s in sizes_int if s <= 512]
    if not sizes_int:
        sizes_int = [512]
    faction_variants = manifest.get("faction_variants")
    if not isinstance(faction_variants, list):
        faction_variants = []
    fset = {str(x) for x in faction_variants}
    factions = ["hostile_corp", "allied_resistance", "marauder_tribes", "religious_order",
                "scientist_order", "mercenary_guild", "pirates", "drone_collective"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        if "canonical_name" not in entry:
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "shell-ui asset")))
        kind = str(entry.get("kind", "shell-ui"))
        # Default palette for shell UI: scientist_order (clean UI palette)
        for sz in sizes_int:
            canonical = f"{name}_{sz}px"
            jobs.append(BakeJob(
                category="ShellUi",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({sz}px)",
                faction="scientist_order",
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=sz,
                palette_ref="scientist_order",
                seed=_seed_for(canonical),
            ))
        if name in fset:
            for faction in factions:
                for sz in sizes_int:
                    canonical = f"{name}_{faction}_{sz}px"
                    jobs.append(BakeJob(
                        category="ShellUi",
                        canonical_name=canonical,
                        kind=kind,
                        prompt=f"{prompt} ({faction} variant, {sz}px)",
                        faction=faction,
                        origin=None,
                        stance=None,
                        facing="right",
                        variant=None,
                        weight_class=None,
                        module_state=None,
                        integrity_band=None,
                        overlay_mode=None,
                        size=sz,
                        palette_ref=faction,
                        seed=_seed_for(canonical),
                    ))
    return jobs


def _hud_widget_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """M11 HUD widget expander — 1 entry × 8 factions × 1 size."""
    jobs: List[BakeJob] = []
    factions = ["hostile_corp", "allied_resistance", "marauder_tribes", "religious_order",
                "scientist_order", "mercenary_guild", "pirates", "drone_collective"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "hud-widget"))
        for faction in factions:
            canonical = f"{name}_{faction}"
            jobs.append(BakeJob(
                category="HudWidget",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({faction})",
                faction=faction,
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=192,
                palette_ref=faction,
                seed=_seed_for(canonical),
            ))
    return jobs


def _banner_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """M11 banner expander — 1 entry × 8 factions, fixed wide aspect."""
    jobs: List[BakeJob] = []
    factions = ["hostile_corp", "allied_resistance", "marauder_tribes", "religious_order",
                "scientist_order", "mercenary_guild", "pirates", "drone_collective"]
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "banner"))
        for faction in factions:
            canonical = f"{name}_{faction}"
            jobs.append(BakeJob(
                category="Banner",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} ({faction})",
                faction=faction,
                origin=None,
                stance=None,
                facing="right",
                variant=None,
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=256,
                palette_ref=faction,
                seed=_seed_for(canonical),
            ))
    return jobs


def _vfx_decal_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """M12 VFX decal expander — single 128px size per entry (no faction recolor)."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry["prompt"])
        kind = str(entry.get("kind", "decal"))
        for variant_idx in range(3):
            canonical = f"{name}_v{variant_idx}"
            jobs.append(BakeJob(
                category="VfxDecal",
                canonical_name=canonical,
                kind=kind,
                prompt=f"{prompt} (variant {variant_idx})",
                faction="mercenary_guild",
                origin=None,
                stance=None,
                facing="right",
                variant=f"v{variant_idx}",
                weight_class=None,
                module_state=None,
                integrity_band=None,
                overlay_mode=None,
                size=128,
                palette_ref="mercenary_guild",
                seed=_seed_for(canonical),
            ))
    return jobs


def _portrait_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """NPC / storyteller / boss / faction-generic portraits — 1 entry, 1 size 384px."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "portrait")))
        kind = str(entry.get("kind", "portrait"))
        faction = str(entry.get("faction", "scientist_order"))
        origin = entry.get("origin")
        canonical = f"{name}"
        jobs.append(BakeJob(
            category="Portrait",
            canonical_name=canonical,
            kind=kind,
            prompt=prompt,
            faction=faction,
            origin=str(origin) if origin else None,
            stance=None,
            facing="right",
            variant=None,
            weight_class=None,
            module_state=None,
            integrity_band=None,
            overlay_mode=None,
            size=384,
            palette_ref=faction,
            seed=_seed_for(canonical),
        ))
    return jobs


def _ui_screen_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """Assembled UI screen mockups — 1 entry, 1 size 1024px."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "ui-screen")))
        kind = str(entry.get("kind", "ui-screen"))
        faction = str(entry.get("faction", "scientist_order"))
        canonical = f"{name}"
        jobs.append(BakeJob(
            category="UiScreen",
            canonical_name=canonical,
            kind=kind,
            prompt=prompt,
            faction=faction,
            origin=None,
            stance=None,
            facing="right",
            variant=None,
            weight_class=None,
            module_state=None,
            integrity_band=None,
            overlay_mode=None,
            size=1024,
            palette_ref=faction,
            seed=_seed_for(canonical),
        ))
    return jobs


def _vfx_frame_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """VFX animation frames — 1 entry per (effect × frame_index), 192px."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "vfx-frame")))
        kind = str(entry.get("kind", "vfx-frame"))
        faction = str(entry.get("faction", "scientist_order"))
        canonical = f"{name}"
        jobs.append(BakeJob(
            category="VfxFrame",
            canonical_name=canonical,
            kind=kind,
            prompt=prompt,
            faction=faction,
            origin=None,
            stance=None,
            facing="right",
            variant=None,
            weight_class=None,
            module_state=None,
            integrity_band=None,
            overlay_mode=None,
            size=192,
            palette_ref=faction,
            seed=_seed_for(canonical),
        ))
    return jobs


def _loading_bg_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """Loading-screen atmospheric backgrounds — 1 entry per scene, 1024px."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "loading-bg")))
        kind = str(entry.get("kind", "loading-bg"))
        faction = str(entry.get("faction", "scientist_order"))
        canonical = f"{name}"
        jobs.append(BakeJob(
            category="LoadingBg",
            canonical_name=canonical,
            kind=kind,
            prompt=prompt,
            faction=faction,
            origin=None,
            stance=None,
            facing="right",
            variant=None,
            weight_class=None,
            module_state=None,
            integrity_band=None,
            overlay_mode=None,
            size=1024,
            palette_ref=faction,
            seed=_seed_for(canonical),
        ))
    return jobs


def _boss_splash_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """Boss intro cinematic key frames — 1 entry per boss, 768px."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "boss-splash")))
        kind = str(entry.get("kind", "boss-splash"))
        faction = str(entry.get("faction", "scientist_order"))
        canonical = f"{name}"
        jobs.append(BakeJob(
            category="BossSplash",
            canonical_name=canonical,
            kind=kind,
            prompt=prompt,
            faction=faction,
            origin=None,
            stance=None,
            facing="right",
            variant=None,
            weight_class=None,
            module_state=None,
            integrity_band=None,
            overlay_mode=None,
            size=768,
            palette_ref=faction,
            seed=_seed_for(canonical),
        ))
    return jobs


def _key_art_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """Marketing key art — 1 entry per faction + main, 1280px wide aspect."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "key-art")))
        kind = str(entry.get("kind", "key-art"))
        faction = str(entry.get("faction", "scientist_order"))
        canonical = f"{name}"
        jobs.append(BakeJob(
            category="KeyArt",
            canonical_name=canonical,
            kind=kind,
            prompt=prompt,
            faction=faction,
            origin=None,
            stance=None,
            facing="right",
            variant=None,
            weight_class=None,
            module_state=None,
            integrity_band=None,
            overlay_mode=None,
            size=1280,
            palette_ref=faction,
            seed=_seed_for(canonical),
        ))
    return jobs


def _animation_frame_jobs(manifest: Dict[str, object]) -> List[BakeJob]:
    """Multi-direction animation frames — 1 entry per (archetype × frame × direction), 256px."""
    jobs: List[BakeJob] = []
    for entry in manifest["entries"]:  # type: ignore[index]
        if not isinstance(entry, dict):
            continue
        name = str(entry["canonical_name"])
        prompt = str(entry.get("prompt", entry.get("kind", "animation-frame")))
        kind = str(entry.get("kind", "animation-frame"))
        faction = str(entry.get("faction", "hostile_corp"))
        origin = entry.get("origin")
        canonical = f"{name}"
        jobs.append(BakeJob(
            category="AnimationFrame",
            canonical_name=canonical,
            kind=kind,
            prompt=prompt,
            faction=faction,
            origin=str(origin) if origin else None,
            stance=str(entry.get("stance", "walking")),
            facing="right",
            variant=None,
            weight_class=None,
            module_state=None,
            integrity_band=None,
            overlay_mode=None,
            size=256,
            palette_ref=faction,
            seed=_seed_for(canonical),
        ))
    return jobs


_MANIFEST_TO_BUILDER = {
    "weapons.ron": _weapon_jobs,
    "actors.ron": _actor_jobs,
    "vehicles.ron": _vehicle_jobs,
    "chassis.ron": _chassis_jobs,
    "base_modules.ron": _base_module_jobs,
    "ui_icons.ron": _ui_icon_jobs,
    "particles.ron": _particle_jobs,
    "terrain_tiles.ron": _terrain_jobs,
    "cosmetic_placeholders.ron": _cosmetic_jobs,
    "shell_ui.ron": _shell_ui_jobs,
    "shell_widgets.ron": _shell_ui_jobs,
    "hud_widgets.ron": _hud_widget_jobs,
    "banners.ron": _banner_jobs,
    "vfx_decals.ron": _vfx_decal_jobs,
    "portraits.ron": _portrait_jobs,
    "ui_screens.ron": _ui_screen_jobs,
    "vfx_frames.ron": _vfx_frame_jobs,
    "loading_backgrounds.ron": _loading_bg_jobs,
    "boss_splashes.ron": _boss_splash_jobs,
    "key_art.ron": _key_art_jobs,
    "animation_frames.ron": _animation_frame_jobs,
}


def build_all_jobs(palettes: Dict[str, palette_loader.Palette]) -> List[BakeJob]:
    jobs: List[BakeJob] = []
    for path in sorted(MANIFEST_ROOT.glob("*.ron")):
        manifest = load_manifest(path)
        builder = _MANIFEST_TO_BUILDER.get(path.name)
        if builder is None and path.name == "materials.ron":
            jobs.extend(_material_jobs(manifest, palettes))
            continue
        if builder is None:
            continue
        jobs.extend(builder(manifest))
    return jobs


# ─── Per-asset bake worker (runs in subprocess) ─────────────────────────────


_PALETTES_CACHE: Optional[Dict[str, palette_loader.Palette]] = None
_STYLES_CACHE: Optional[Dict[str, Dict[str, style_enforcer.StyleDescriptor]]] = None
_MAT_BANDS_CACHE: Optional[Dict[str, List[str]]] = None


def _load_caches() -> None:
    global _PALETTES_CACHE, _STYLES_CACHE, _MAT_BANDS_CACHE
    if _PALETTES_CACHE is None:
        _PALETTES_CACHE = palette_loader.load_all_palettes()
    if _STYLES_CACHE is None:
        _STYLES_CACHE = style_enforcer.load_all_style_descriptors()
    if _MAT_BANDS_CACHE is None:
        _MAT_BANDS_CACHE = palette_loader.material_bands(_PALETTES_CACHE or {})


def _output_dir_for(category: str) -> Path:
    sub = {
        "WeaponSprite": "weapons",
        "ActorSprite": "actors",
        "VehicleSprite": "vehicles",
        "ChassisSprite": "chassis",
        "BaseModuleSprite": "base_modules",
        "UiIcon": "ui_icons",
        "MaterialSwatch": "materials",
        "Particle": "particles",
        "TerrainTile": "terrain_tiles",
        "Cosmetic": "cosmetics",
        "FactionEmblem": "faction_emblems",
        "CaptureGridOverlay": "capture_overlays",
        "ShellUi": "shell_ui",
        "HudWidget": "hud_widgets",
        "Banner": "banners",
        "VfxDecal": "vfx_decals",
        "AnimationFrame": "animation_frames",
        "Portrait": "portraits",
        "UiScreen": "ui_screens",
        "VfxFrame": "vfx_frames",
        "LoadingBg": "loading_backgrounds",
        "BossSplash": "boss_splashes",
        "KeyArt": "key_art",
    }.get(category, "misc")
    return PLACEHOLDER_ROOT / sub


def _bake_one(job: BakeJob) -> Dict[str, object]:
    _load_caches()
    palettes = _PALETTES_CACHE or {}
    styles = _STYLES_CACHE or {"faction": {}, "origin": {}}
    mat_bands = _MAT_BANDS_CACHE or {}

    palette = palettes.get(job.palette_ref) or palettes.get(job.faction or "")
    if palette is None:
        # Generic fallback palette so no job dies.
        palette = palette_loader.Palette(
            palette_id="generic",
            display_name="Generic",
            category="system",
            description="fallback",
            colors=[palette_loader.PaletteColor(role="primary", hex="#888888"),
                    palette_loader.PaletteColor(role="dark", hex="#222222"),
                    palette_loader.PaletteColor(role="accent", hex="#cc4444"),
                    palette_loader.PaletteColor(role="highlight", hex="#dddddd")],
        )
    style = None
    if job.faction:
        style = styles.get("faction", {}).get(job.faction)
    origin_palette = palettes.get(job.origin or "") if job.origin else None

    extra: Dict[str, str] = {}
    if job.variant:
        extra["variant"] = job.variant
    if job.stance:
        extra["stance"] = job.stance
    if job.facing:
        extra["facing"] = job.facing
    if job.weight_class:
        extra["weight_class"] = job.weight_class
    if job.module_state:
        extra["module_state"] = job.module_state
    if job.integrity_band and job.category == "MaterialSwatch":
        # Look up the integrity-band color for this material.
        band_idx = {"pristine": 0, "scratched": 1, "cracked": 2, "critical": 3, "destroyed": 4}.get(
            job.integrity_band, 0)
        # The canonical_name carries the material id followed by band suffix.
        # Strip the band/overlay suffix to recover the material id.
        material_id = job.canonical_name
        for suffix in ("_pristine", "_scratched", "_cracked", "_critical", "_destroyed"):
            if material_id.endswith(suffix):
                material_id = material_id[: -len(suffix)]
                break
        for suffix in ("_overlay_integrity", "_overlay_pathability", "_overlay_mobility",
                       "_overlay_hazard", "_overlay_build_repair"):
            if material_id.endswith(suffix):
                material_id = material_id[: -len(suffix)]
                break
        bands = mat_bands.get(material_id)
        if bands and 0 <= band_idx < len(bands):
            extra["base_color"] = bands[band_idx]
        extra["integrity_band"] = job.integrity_band
    if job.integrity_band and job.category == "TerrainTile":
        extra["integrity_band"] = job.integrity_band
    if job.category == "TerrainTile":
        material_id = job.canonical_name.replace("tile_", "")
        for v in ["_a", "_b", "_c", "_d", "_e"]:
            if material_id.endswith(v):
                material_id = material_id[: -len(v)]
                break
        bands = mat_bands.get(material_id)
        if bands:
            extra["base_color"] = bands[0]
        if job.variant:
            extra["variant"] = job.variant
    if job.overlay_mode:
        extra["overlay_mode"] = job.overlay_mode

    spec = llm_svg_prompter.AssetSpec(
        canonical_name=job.canonical_name,
        kind=job.kind,
        category=job.category,
        width=job.size,
        height=job.size,
        seed=job.seed,
        palette=palette,
        style=style,
        origin_palette=origin_palette,
        extra=extra,
    )
    svg_text = llm_svg_prompter.compose_svg(spec)
    svg_bytes = svg_text.encode("utf-8")

    out_dir = _output_dir_for(job.category)
    svg_path = out_dir / f"{job.canonical_name}.svg"
    svg_path.parent.mkdir(parents=True, exist_ok=True)
    staging = svg_path.with_suffix(".svg.tmp")
    staging.write_bytes(svg_bytes)
    os.replace(staging, svg_path)
    png_path = out_dir / f"{job.canonical_name}.png"
    png_size = cairo_renderer.render_to_png(svg_bytes, png_path, job.size)
    svg_size, svg_blake3 = ledger_writer.hash_path(svg_path)
    png_size_actual, png_blake3 = ledger_writer.hash_path(png_path)

    rel_svg = str(svg_path.resolve())
    rel_png = str(png_path.resolve())
    # Map non-engine categories (FactionEmblem, ShellUi, HudWidget, Banner,
    # CaptureGridOverlay) to the closest first-class engine category. The
    # original category is preserved in the kind string for filterability.
    category_to_engine = {
        "FactionEmblem": "UiIcon",
        "CaptureGridOverlay": "UiIcon",
        "ShellUi": "UiIcon",
        "HudWidget": "UiIcon",
        "Banner": "UiIcon",
        "VfxDecal": "Particle",
        "AnimationFrame": "Animation",
        "Portrait": "UiIcon",
        "UiScreen": "UiIcon",
        "VfxFrame": "Particle",
        "LoadingBg": "UiIcon",
        "BossSplash": "Cosmetic",
        "KeyArt": "Cosmetic",
    }
    engine_category = category_to_engine.get(job.category, job.category)
    draft = ledger_writer.LedgerEntryDraft(
        category=engine_category,
        kind=job.kind,
        canonical_name=job.canonical_name,
        tier="Tier1_SVG",
        pipeline="M9A_svg_v1",
        prompt=job.prompt,
        seed=job.seed,
        output_path=rel_svg,
        output_blake3=svg_blake3,
        output_size_bytes=svg_size,
        output_format="svg",
        palette_ref=job.palette_ref,
        additional_outputs=[{
            "label": f"png_{job.size}",
            "output_path": rel_png,
            "blake3": png_blake3,
            "size_bytes": int(png_size_actual),
        }],
    )
    if job.category == "CaptureGridOverlay":
        draft.kind = "capture-overlay"

    entry = ledger_writer.build_entry(draft)
    return entry


# ─── Top-level pipeline ────────────────────────────────────────────────────


def report_existing() -> Dict[str, int]:
    """Quick filesystem scan: count SVG / PNG already on disk + ledger lines."""
    out = {"svg": 0, "png": 0, "ledger": 0}
    if PLACEHOLDER_ROOT.exists():
        for path in PLACEHOLDER_ROOT.rglob("*.svg"):
            out["svg"] += 1
        for path in PLACEHOLDER_ROOT.rglob("*.png"):
            out["png"] += 1
    if LEDGER_PATH.exists():
        with LEDGER_PATH.open("r", encoding="utf-8") as f:
            for line in f:
                if line.strip():
                    out["ledger"] += 1
    return out


def bake(jobs: List[BakeJob], parallel: int = 0) -> List[Dict[str, object]]:
    """Run the bake. Returns the list of ledger entry dicts."""
    entries: List[Dict[str, object]] = []
    started = time.time()
    n = len(jobs)
    if parallel <= 1:
        for i, job in enumerate(jobs):
            entry = _bake_one(job)
            entries.append(entry)
            if (i + 1) % 200 == 0 or i + 1 == n:
                elapsed = time.time() - started
                rate = (i + 1) / elapsed if elapsed > 0 else 0
                print(
                    f"[bake] {i + 1}/{n} ({rate:.1f}/s; "
                    f"elapsed {elapsed:.1f}s)",
                    file=sys.stderr,
                )
        return entries

    workers = max(2, parallel)
    print(f"[bake] running {n} jobs across {workers} workers", file=sys.stderr)
    with ProcessPoolExecutor(max_workers=workers) as ex:
        futures = [ex.submit(_bake_one, job) for job in jobs]
        completed = 0
        for fut in as_completed(futures):
            entries.append(fut.result())
            completed += 1
            if completed % 200 == 0 or completed == n:
                elapsed = time.time() - started
                rate = completed / elapsed if elapsed > 0 else 0
                print(
                    f"[bake] {completed}/{n} ({rate:.1f}/s; "
                    f"elapsed {elapsed:.1f}s)",
                    file=sys.stderr,
                )
    return entries


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="M9A Tier-1 SVG asset pipeline.")
    parser.add_argument("--all", action="store_true", help="Bake every asset.")
    parser.add_argument("--check", action="store_true",
                        help="Dry-run; report counts without writing.")
    parser.add_argument("--report", action="store_true",
                        help="Report on-disk + ledger counts.")
    parser.add_argument("--category", type=str, default=None,
                        help="Only bake one category (e.g. WeaponSprite).")
    parser.add_argument("--parallel", type=int, default=0,
                        help="Parallel workers (0 = serial; >=2 = multiprocessing).")
    parser.add_argument("--limit", type=int, default=0,
                        help="Limit total jobs (test mode).")
    args = parser.parse_args(argv)

    palettes = palette_loader.load_all_palettes()
    print(f"[build_placeholders] loaded {len(palettes)} palettes", file=sys.stderr)

    if args.report:
        counts = report_existing()
        print(f"on-disk: svg={counts['svg']} png={counts['png']} ledger_entries={counts['ledger']}")
        return 0

    jobs = build_all_jobs(palettes)
    if args.category:
        jobs = [j for j in jobs if j.category == args.category]
    if args.limit > 0:
        jobs = jobs[: args.limit]

    print(f"[build_placeholders] planned: {len(jobs)} bake jobs", file=sys.stderr)
    by_cat: Dict[str, int] = {}
    for j in jobs:
        by_cat[j.category] = by_cat.get(j.category, 0) + 1
    for cat in sorted(by_cat):
        print(f"  {cat}: {by_cat[cat]}", file=sys.stderr)

    if args.check:
        counts = report_existing()
        stale = max(0, len(jobs) - counts["ledger"])
        print(
            f"[check] {len(jobs)} planned; {counts['ledger']} in ledger; ~{stale} stale to bake"
        )
        return 0

    if not args.all and not args.category:
        print("specify --all or --category, or --check / --report", file=sys.stderr)
        return 2

    CONTENT_ROOT.mkdir(parents=True, exist_ok=True)
    PLACEHOLDER_ROOT.mkdir(parents=True, exist_ok=True)
    LEDGER_PATH.parent.mkdir(parents=True, exist_ok=True)

    entries = bake(jobs, parallel=args.parallel)
    n = ledger_writer.overwrite_ledger(LEDGER_PATH, entries)
    counts = report_existing()
    print(
        f"baked: {n} assets across {len(by_cat)} categories; "
        f"on-disk svg={counts['svg']} png={counts['png']} ledger_lines={counts['ledger']}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
