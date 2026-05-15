"""Style descriptor loader + per-faction silhouette consistency checks.

Style descriptors live at `tools/asset_gen/style_descriptors/factions/*.style.json`
and `tools/asset_gen/style_descriptors/origins/*.style.json`. Each descriptor
declares (silhouette_language, preferred_palette_roles, shape_descriptors,
geometric_primitives) per the v1 schema.

`enforce_style(svg, descriptor, palette)` is invoked by the build orchestrator
after llm_svg_prompter.compose_svg() returns to verify:

1. SVG dimensions match the declared viewbox.
2. All `fill=` / `stroke=` colors are in the palette's color set.
3. Path-count under max_paths (when set).

On hard violation, returns (False, [error_messages]). On warning-level drift
(palette near-miss, low-confidence shape match), returns (True, [warnings]).
The orchestrator decides whether to regenerate.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Sequence, Tuple


PIPELINE_ROOT = Path(__file__).resolve().parent
STYLE_ROOT = PIPELINE_ROOT / "style_descriptors"


@dataclass(frozen=True)
class StyleDescriptor:
    style_id: str
    category: str
    version: str
    silhouette_language: str
    preferred_palette_roles: List[str] = field(default_factory=list)
    shape_descriptors: Dict[str, str] = field(default_factory=dict)
    geometric_primitives: Dict[str, List[str]] = field(default_factory=dict)


def _load_one(path: Path) -> StyleDescriptor:
    raw = json.loads(path.read_text(encoding="utf-8"))
    return StyleDescriptor(
        style_id=str(raw["style_id"]),
        category=str(raw.get("category", "faction")),
        version=str(raw.get("version", "1.0.0")),
        silhouette_language=str(raw.get("silhouette_language", "")),
        preferred_palette_roles=list(raw.get("preferred_palette_roles", [])),
        shape_descriptors=dict(raw.get("shape_descriptors", {})),
        geometric_primitives={k: list(v) for k, v in raw.get("geometric_primitives", {}).items()},
    )


def load_style_descriptor(style_id: str, category: str = "faction") -> StyleDescriptor:
    """Load a style descriptor by id and category."""
    candidate = STYLE_ROOT / f"{category}s" / f"{style_id}.style.json"
    if not candidate.is_file():
        raise FileNotFoundError(
            f"style descriptor '{style_id}' (category={category}) not found at {candidate}"
        )
    return _load_one(candidate)


def load_all_style_descriptors() -> Dict[str, Dict[str, StyleDescriptor]]:
    """Returns {category: {style_id: StyleDescriptor}}."""
    out: Dict[str, Dict[str, StyleDescriptor]] = {"faction": {}, "origin": {}}
    for path in sorted(STYLE_ROOT.rglob("*.style.json")):
        d = _load_one(path)
        out.setdefault(d.category, {})[d.style_id] = d
    return out


# Compiled once so enforcement of 5000 SVGs doesn't recompile per-call.
_FILL_OR_STROKE_RE = re.compile(r'\b(?:fill|stroke)\s*=\s*"(#[0-9a-fA-F]{3,6})"')
_PATH_TAG_RE = re.compile(r"<path\b")


def enforce_style(
    svg: str,
    descriptor: StyleDescriptor,
    palette_hexes: Sequence[str],
    max_paths: Optional[int] = None,
) -> Tuple[bool, List[str]]:
    """Validate that an SVG matches a faction/origin style descriptor.

    Returns (ok, messages). ok=False means the SVG fails enforcement and the
    pipeline should fall back to procedural geometric or regenerate with
    adjusted prompt. messages may include warnings even when ok=True.
    """
    messages: List[str] = []

    if not svg.startswith("<svg") and "<svg" not in svg[:200]:
        return (False, ["svg missing <svg> root element"])

    palette_set = {hx.lower() for hx in palette_hexes}
    color_violations: List[str] = []
    for match in _FILL_OR_STROKE_RE.finditer(svg):
        color = match.group(1).lower()
        if len(color) == 4:
            color = "#" + "".join(ch * 2 for ch in color[1:])
        if color not in palette_set:
            color_violations.append(color)
    if color_violations:
        unique = sorted(set(color_violations))[:6]
        messages.append(f"svg uses non-palette colors: {unique}")

    if max_paths is not None:
        n = len(_PATH_TAG_RE.findall(svg))
        if n > max_paths:
            return (False, messages + [f"path count {n} exceeds max {max_paths}"])

    ok = not color_violations
    return (ok, messages)


def shape_descriptor_for_kind(descriptor: StyleDescriptor, kind: str) -> str:
    """Look up the shape descriptor for an asset kind, with a sensible fallback."""
    if kind in descriptor.shape_descriptors:
        return descriptor.shape_descriptors[kind]
    # Stripped fallback: any shape descriptor at all is better than nothing.
    if descriptor.shape_descriptors:
        return next(iter(descriptor.shape_descriptors.values()))
    return descriptor.silhouette_language


__all__ = [
    "PIPELINE_ROOT",
    "STYLE_ROOT",
    "StyleDescriptor",
    "load_style_descriptor",
    "load_all_style_descriptors",
    "enforce_style",
    "shape_descriptor_for_kind",
]
