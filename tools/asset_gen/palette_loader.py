"""Palette JSON loader for the M9A Tier-1 SVG pipeline.

Reads palette files under `tools/asset_gen/palettes/` and exposes a uniform
dict-style API to the rest of the pipeline. Palettes are immutable per session;
re-call `load_all_palettes` to pick up edits between bakes.

Determinism contract: returned palette dicts are sorted by `palette_id`; color
arrays preserve the on-disk order so role-index references in
`llm_svg_prompter` stay stable across machines.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional


PIPELINE_ROOT = Path(__file__).resolve().parent
PALETTE_ROOT = PIPELINE_ROOT / "palettes"


@dataclass(frozen=True)
class PaletteColor:
    role: str
    hex: str
    comment: str = ""


@dataclass(frozen=True)
class Palette:
    palette_id: str
    display_name: str
    category: str
    description: str
    colors: List[PaletteColor] = field(default_factory=list)
    accent_pairs: List[Dict[str, str]] = field(default_factory=list)
    material_subpalettes: List[Dict[str, object]] = field(default_factory=list)

    def hex_list(self) -> List[str]:
        return [c.hex for c in self.colors]

    def color_by_role(self, role: str) -> Optional[str]:
        for c in self.colors:
            if c.role == role:
                return c.hex
        return None

    def color_or_default(self, role: str, default: str) -> str:
        return self.color_by_role(role) or default

    def primary(self) -> str:
        return self.color_or_default("primary", "#888888")

    def secondary(self) -> str:
        return self.color_or_default("secondary", "#444444")

    def accent(self) -> str:
        return self.color_or_default("accent", "#cc4444")

    def highlight(self) -> str:
        return self.color_or_default("highlight", "#dddddd")

    def light(self) -> str:
        return self.color_or_default("light", "#aaaaaa")

    def dark(self) -> str:
        return self.color_or_default("dark", "#111111")

    def metal(self) -> str:
        return self.color_or_default("metal", self.secondary())

    def glow(self) -> str:
        return self.color_or_default("glow", self.highlight())

    def cloth(self) -> str:
        return self.color_or_default("cloth", self.secondary())


def _hex_normalize(hex_value: str) -> str:
    h = hex_value.strip().lower()
    if not h.startswith("#"):
        h = "#" + h
    return h


def _load_one(path: Path) -> Palette:
    raw = json.loads(path.read_text(encoding="utf-8"))
    colors = [
        PaletteColor(
            role=str(c["role"]),
            hex=_hex_normalize(str(c["hex"])),
            comment=str(c.get("comment", "")),
        )
        for c in raw.get("colors", [])
    ]
    return Palette(
        palette_id=str(raw["palette_id"]),
        display_name=str(raw.get("display_name", raw["palette_id"])),
        category=str(raw.get("category", "system")),
        description=str(raw.get("description", "")),
        colors=colors,
        accent_pairs=list(raw.get("accent_pairs", [])),
        material_subpalettes=list(raw.get("material_subpalettes", [])),
    )


def load_palette(palette_id: str) -> Palette:
    """Load a palette by id. Searches the standard palette tree."""
    for candidate in (
        PALETTE_ROOT / "factions" / f"{palette_id}.palette.json",
        PALETTE_ROOT / "origins" / f"{palette_id}.palette.json",
        PALETTE_ROOT / f"{palette_id}.palette.json",
    ):
        if candidate.is_file():
            return _load_one(candidate)
    raise FileNotFoundError(f"palette '{palette_id}' not found under {PALETTE_ROOT}")


def load_all_palettes() -> Dict[str, Palette]:
    """Load every palette under PALETTE_ROOT into a dict keyed by palette_id."""
    palettes: Dict[str, Palette] = {}
    for path in sorted(PALETTE_ROOT.rglob("*.palette.json")):
        p = _load_one(path)
        palettes[p.palette_id] = p
    return palettes


def material_bands(palettes: Dict[str, Palette]) -> Dict[str, List[str]]:
    """Build a {material_id: [pristine..destroyed]} mapping from the materials palette."""
    mat = palettes.get("materials")
    if mat is None:
        return {}
    out: Dict[str, List[str]] = {}
    for sub in mat.material_subpalettes:
        bands = sub.get("bands")
        material_id = sub.get("material_id")
        if not isinstance(bands, list) or not isinstance(material_id, str):
            continue
        out[material_id] = [_hex_normalize(str(b)) for b in bands]
    return out


def faction_emblem_pair(palettes: Dict[str, Palette], faction_id: str) -> Optional[Dict[str, str]]:
    """Return {'fg': '#xxx', 'bg': '#yyy'} for a faction id from the emblems palette."""
    emblems = palettes.get("factions_emblems")
    if emblems is None:
        return None
    fg = emblems.color_by_role(f"{faction_id}_fg")
    bg = emblems.color_by_role(f"{faction_id}_bg")
    if fg is None or bg is None:
        return None
    return {"fg": fg, "bg": bg}


__all__ = [
    "PIPELINE_ROOT",
    "PALETTE_ROOT",
    "Palette",
    "PaletteColor",
    "load_palette",
    "load_all_palettes",
    "material_bands",
    "faction_emblem_pair",
]
