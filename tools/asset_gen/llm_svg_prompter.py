"""SVG path composer for M9A Tier-1 assets.

The spec describes two modes:

1. **LLM-prompted mode** — call an LLM with the asset prompt + palette + style
   descriptor and parse the returned SVG. In the M9A bake context the LLM
   "is the worker" (per the corefall-impl skill's M9A guidance): the LLM
   directly authored the procedural composer below by inspecting each
   archetype's prompt and writing palette-aware shape compositions. There is
   no external LLM API call. The composer is therefore named after the
   spec-required `llm_svg_prompter.py` filename but executes pure-procedural
   generation per asset kind.

2. **Procedural fallback** — composed of palette-sourced rectangles, ellipses,
   circles, lines, and polygons per the spec's explicit fallback path. Output
   is byte-deterministic given (palette, seed, kind, dimensions).

Both modes route through `compose_svg(...)` so the build_placeholders.py
orchestrator can call one function. When an exotic asset trips the
deterministic composer (e.g. an unrecognized kind), the fallback gracefully
emits a generic faction-coherent silhouette so no asset is skipped.

Determinism contract:
- Identical (kind, palette, seed, size) → byte-identical SVG output.
- No `random` calls without an explicit `random.Random(seed)` instance.
- All numeric coordinates rounded to 2 decimal places to avoid float drift.
"""

from __future__ import annotations

import random
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

from .palette_loader import Palette
from .style_enforcer import StyleDescriptor


SVG_HEADER = (
    '<?xml version="1.0" encoding="UTF-8" standalone="no"?>\n'
    '<svg xmlns="http://www.w3.org/2000/svg" '
    'viewBox="0 0 {w} {h}" width="{w}" height="{h}" shape-rendering="crispEdges">\n'
)
SVG_FOOTER = "</svg>\n"


@dataclass
class AssetSpec:
    canonical_name: str
    kind: str
    category: str
    width: int
    height: int
    seed: int
    palette: Palette
    style: Optional[StyleDescriptor] = None
    origin_palette: Optional[Palette] = None
    extra: Dict[str, str] = None  # type: ignore[assignment]


# ─── Low-level shape helpers ────────────────────────────────────────────────


def _r(value: float) -> str:
    return f"{value:.2f}"


def _rect(x: float, y: float, w: float, h: float, fill: str, stroke: Optional[str] = None,
          stroke_w: float = 0.0) -> str:
    s = f'<rect x="{_r(x)}" y="{_r(y)}" width="{_r(w)}" height="{_r(h)}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _circle(cx: float, cy: float, r: float, fill: str, stroke: Optional[str] = None,
            stroke_w: float = 0.0) -> str:
    s = f'<circle cx="{_r(cx)}" cy="{_r(cy)}" r="{_r(r)}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _ellipse(cx: float, cy: float, rx: float, ry: float, fill: str,
             stroke: Optional[str] = None, stroke_w: float = 0.0) -> str:
    s = f'<ellipse cx="{_r(cx)}" cy="{_r(cy)}" rx="{_r(rx)}" ry="{_r(ry)}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _line(x1: float, y1: float, x2: float, y2: float, stroke: str, stroke_w: float = 1.0) -> str:
    return (
        f'<line x1="{_r(x1)}" y1="{_r(y1)}" x2="{_r(x2)}" y2="{_r(y2)}" '
        f'stroke="{stroke}" stroke-width="{_r(stroke_w)}"/>'
    )


def _polygon(points: List[Tuple[float, float]], fill: str,
             stroke: Optional[str] = None, stroke_w: float = 0.0) -> str:
    pts = " ".join(f"{_r(x)},{_r(y)}" for x, y in points)
    s = f'<polygon points="{pts}" fill="{fill}"'
    if stroke:
        s += f' stroke="{stroke}" stroke-width="{_r(stroke_w)}"'
    s += "/>"
    return s


def _polyline(points: List[Tuple[float, float]], stroke: str, stroke_w: float = 1.0) -> str:
    pts = " ".join(f"{_r(x)},{_r(y)}" for x, y in points)
    return f'<polyline points="{pts}" fill="none" stroke="{stroke}" stroke-width="{_r(stroke_w)}"/>'


# ─── Composers by kind ──────────────────────────────────────────────────────


def _compose_weapon(spec: AssetSpec, rng: random.Random) -> str:
    """M12 polish-pass: weapon class-aware silhouettes.

    Dispatches on the canonical weapon class (rifle / sniper / shotgun / smg /
    pistol / gl / heavy / spec) parsed from the asset name. Each class has a
    distinct silhouette + proportions + detail layer (sight, scope, drum,
    bipod, etc.). Color richness preserved via primary + accent + metal +
    highlight + glow palette slots.
    """
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    metal = p.metal()
    dark = p.dark()
    highlight = p.highlight()
    glow = p.glow()

    name = spec.canonical_name.lower()
    if "rifle" in name:
        wclass = "rifle"
    elif "sniper" in name:
        wclass = "sniper"
    elif "shotgun" in name:
        wclass = "shotgun"
    elif "smg" in name:
        wclass = "smg"
    elif "pistol" in name:
        wclass = "pistol"
    elif "gl_" in name or name.startswith("gl_") or "_gl_" in name or "grenade_launcher" in name:
        wclass = "gl"
    elif "heavy" in name:
        wclass = "heavy"
    elif "spec_flamer" in name:
        wclass = "flamer"
    elif "spec_drill" in name or "drill_lance" in name:
        wclass = "drill"
    elif "spec_grapp" in name or "grappler" in name:
        wclass = "grappler"
    elif "spec_drone" in name or "sticky_array" in name:
        wclass = "drone"
    elif "spec_" in name:
        wclass = "spec"
    else:
        wclass = "rifle"

    parts: List[str] = []
    W = spec.width
    H = spec.height
    cx, cy = W / 2, H / 2

    if wclass == "pistol":
        # Compact: short barrel + no stock + tilted grip
        barrel_y = cy - H * 0.04
        parts.append(_rect(cx - W * 0.02, barrel_y, W * 0.28, H * 0.08, metal, dark, 0.5))  # short barrel
        parts.append(_rect(cx + W * 0.24, barrel_y - H * 0.01, W * 0.04, H * 0.10, dark))  # muzzle ring
        parts.append(_rect(cx - W * 0.12, barrel_y - H * 0.06, W * 0.18, H * 0.20, body, dark, 1.0))  # slide
        parts.append(_rect(cx - W * 0.10, barrel_y - H * 0.10, W * 0.06, H * 0.04, metal, dark, 0.5))  # iron sight rear
        parts.append(_rect(cx + W * 0.02, barrel_y - H * 0.10, W * 0.02, H * 0.04, metal, dark, 0.5))  # iron sight front
        # Trigger guard + grip
        parts.append(_polygon([
            (cx - W * 0.08, cy + H * 0.10),
            (cx - W * 0.04, cy + H * 0.10),
            (cx - W * 0.04, cy + H * 0.20),
            (cx - W * 0.08, cy + H * 0.20),
        ], dark))
        parts.append(_polygon([
            (cx - W * 0.06, cy + H * 0.08),
            (cx + W * 0.02, cy + H * 0.08),
            (cx + W * 0.05, cy + H * 0.30),
            (cx - W * 0.03, cy + H * 0.30),
        ], body, dark, 1.0))  # grip with checkering implied
        parts.append(_rect(cx - W * 0.04, cy + H * 0.08, W * 0.06, H * 0.20, dark))  # mag well
        parts.append(_circle(cx + W * 0.06, cy + H * 0.04, W * 0.01, accent))  # safety pin
    elif wclass == "smg":
        # Folding stock + compact barrel + vertical magazine + tactical rail
        barrel_y = cy - H * 0.04
        parts.append(_rect(cx + W * 0.08, barrel_y, W * 0.30, H * 0.08, metal, dark, 0.5))  # barrel
        parts.append(_rect(cx + W * 0.36, barrel_y - H * 0.01, W * 0.04, H * 0.10, dark))  # muzzle device
        parts.append(_rect(cx - W * 0.20, barrel_y - H * 0.08, W * 0.32, H * 0.22, body, dark, 1.0))  # boxy receiver
        parts.append(_rect(cx - W * 0.15, barrel_y - H * 0.13, W * 0.25, H * 0.04, metal, dark, 0.5))  # picatinny rail
        parts.append(_rect(cx - W * 0.10, barrel_y - H * 0.17, W * 0.08, H * 0.05, dark))  # red dot sight
        parts.append(_circle(cx - W * 0.07, barrel_y - H * 0.145, W * 0.012, glow))  # sight glow
        # Vertical magazine (long)
        parts.append(_rect(cx - W * 0.05, cy + H * 0.14, W * 0.07, H * 0.26, accent, dark, 0.5))
        # Pistol grip
        parts.append(_polygon([
            (cx + W * 0.04, cy + H * 0.14),
            (cx + W * 0.11, cy + H * 0.14),
            (cx + W * 0.08, cy + H * 0.34),
            (cx + W * 0.02, cy + H * 0.34),
        ], dark, body, 0.5))
        # Folding stock (collapsed shape)
        parts.append(_rect(cx - W * 0.36, cy - H * 0.02, W * 0.18, H * 0.03, dark))
        parts.append(_rect(cx - W * 0.36, cy + H * 0.02, W * 0.04, H * 0.06, body, dark, 0.5))
    elif wclass == "rifle":
        # Classic assault rifle: long barrel + full stock + magazine + scope rail
        barrel_y = cy - H * 0.04
        parts.append(_rect(cx + W * 0.08, barrel_y, W * 0.36, H * 0.08, metal, dark, 0.5))  # barrel
        parts.append(_rect(cx + W * 0.42, barrel_y - H * 0.01, W * 0.04, H * 0.10, dark))  # muzzle device
        parts.append(_rect(cx + W * 0.30, barrel_y - H * 0.04, W * 0.10, H * 0.04, dark))  # foregrip/handguard
        parts.append(_rect(cx - W * 0.16, barrel_y - H * 0.10, W * 0.32, H * 0.24, body, dark, 1.0))  # receiver
        parts.append(_rect(cx - W * 0.10, barrel_y - H * 0.14, W * 0.20, H * 0.04, metal, dark, 0.5))  # picatinny rail
        parts.append(_rect(cx - W * 0.05, barrel_y - H * 0.20, W * 0.10, H * 0.06, dark, metal, 0.5))  # red-dot housing
        parts.append(_circle(cx, barrel_y - H * 0.17, W * 0.015, glow))  # reticle glow
        # Magazine (curved)
        parts.append(_polygon([
            (cx - W * 0.04, cy + H * 0.14),
            (cx + W * 0.04, cy + H * 0.14),
            (cx + W * 0.06, cy + H * 0.30),
            (cx - W * 0.02, cy + H * 0.30),
        ], accent, dark, 0.8))
        # Pistol grip
        parts.append(_polygon([
            (cx + W * 0.06, cy + H * 0.14),
            (cx + W * 0.12, cy + H * 0.14),
            (cx + W * 0.10, cy + H * 0.30),
            (cx + W * 0.04, cy + H * 0.30),
        ], dark, body, 0.5))
        # Full stock (heavy buttplate)
        parts.append(_polygon([
            (cx - W * 0.32, cy - H * 0.04),
            (cx - W * 0.16, cy - H * 0.10),
            (cx - W * 0.16, cy + H * 0.14),
            (cx - W * 0.32, cy + H * 0.10),
        ], body, dark, 1.0))
        parts.append(_rect(cx - W * 0.36, cy - H * 0.04, W * 0.04, H * 0.16, dark))  # buttplate
    elif wclass == "sniper":
        # Very long barrel + heavy scope + bipod + bolt-action receiver
        barrel_y = cy - H * 0.04
        parts.append(_rect(cx + W * 0.05, barrel_y, W * 0.42, H * 0.06, metal, dark, 0.5))  # long barrel
        parts.append(_rect(cx + W * 0.45, barrel_y - H * 0.02, W * 0.03, H * 0.10, dark))  # muzzle device
        parts.append(_rect(cx - W * 0.05, barrel_y, W * 0.10, H * 0.06, dark))  # muzzle brake band
        parts.append(_rect(cx - W * 0.20, barrel_y - H * 0.08, W * 0.30, H * 0.20, body, dark, 1.0))  # receiver
        # Large scope
        parts.append(_rect(cx - W * 0.12, barrel_y - H * 0.18, W * 0.24, H * 0.06, dark, metal, 0.8))
        parts.append(_circle(cx - W * 0.12, barrel_y - H * 0.15, W * 0.03, dark))  # ocular
        parts.append(_circle(cx + W * 0.10, barrel_y - H * 0.15, W * 0.03, dark))  # objective
        parts.append(_circle(cx + W * 0.10, barrel_y - H * 0.15, W * 0.015, glow))  # scope glow
        parts.append(_rect(cx - W * 0.05, barrel_y - H * 0.16, W * 0.10, H * 0.04, metal))  # scope ring
        # Bolt
        parts.append(_rect(cx - W * 0.02, barrel_y - H * 0.10, W * 0.06, H * 0.04, metal, dark, 0.5))
        parts.append(_circle(cx + W * 0.05, barrel_y - H * 0.08, W * 0.01, accent))  # bolt knob
        # Magazine (single-stack box)
        parts.append(_rect(cx - W * 0.06, cy + H * 0.14, W * 0.06, H * 0.16, accent, dark, 0.5))
        # Pistol grip
        parts.append(_polygon([
            (cx + W * 0.02, cy + H * 0.14),
            (cx + W * 0.08, cy + H * 0.14),
            (cx + W * 0.06, cy + H * 0.30),
            (cx, cy + H * 0.30),
        ], dark, body, 0.5))
        # Long full stock with cheek-rest
        parts.append(_polygon([
            (cx - W * 0.40, cy - H * 0.05),
            (cx - W * 0.20, cy - H * 0.12),
            (cx - W * 0.20, cy + H * 0.12),
            (cx - W * 0.40, cy + H * 0.08),
        ], body, dark, 1.0))
        parts.append(_rect(cx - W * 0.36, cy - H * 0.16, W * 0.10, H * 0.06, body, dark, 0.5))  # cheek riser
        # Bipod
        parts.append(_line(cx + W * 0.18, cy + H * 0.02, cx + W * 0.12, cy + H * 0.30, dark, 1.5))
        parts.append(_line(cx + W * 0.18, cy + H * 0.02, cx + W * 0.24, cy + H * 0.30, dark, 1.5))
    elif wclass == "shotgun":
        # Thick barrel + pump action + tube magazine OR shell loop + heavy stock
        barrel_y = cy - H * 0.04
        parts.append(_rect(cx + W * 0.06, barrel_y, W * 0.38, H * 0.11, metal, dark, 0.5))  # thick barrel
        parts.append(_rect(cx + W * 0.42, barrel_y - H * 0.01, W * 0.04, H * 0.13, dark))  # choke
        # Tube magazine under barrel
        parts.append(_rect(cx + W * 0.06, barrel_y + H * 0.13, W * 0.32, H * 0.06, accent, dark, 0.5))
        # Pump-action grip (forward)
        parts.append(_rect(cx + W * 0.16, barrel_y + H * 0.19, W * 0.10, H * 0.06, dark, body, 0.5))
        # Receiver
        parts.append(_rect(cx - W * 0.18, barrel_y - H * 0.10, W * 0.24, H * 0.24, body, dark, 1.0))
        # Bead sight (no scope)
        parts.append(_rect(cx + W * 0.40, barrel_y - H * 0.04, W * 0.02, H * 0.05, dark))
        parts.append(_circle(cx + W * 0.41, barrel_y - H * 0.05, W * 0.01, glow))
        # Pistol grip
        parts.append(_polygon([
            (cx, cy + H * 0.14),
            (cx + W * 0.06, cy + H * 0.14),
            (cx + W * 0.04, cy + H * 0.34),
            (cx - W * 0.02, cy + H * 0.34),
        ], dark, body, 0.5))
        # Heavy stock with recoil pad
        parts.append(_polygon([
            (cx - W * 0.32, cy - H * 0.04),
            (cx - W * 0.18, cy - H * 0.10),
            (cx - W * 0.18, cy + H * 0.14),
            (cx - W * 0.32, cy + H * 0.12),
        ], body, dark, 1.0))
        parts.append(_rect(cx - W * 0.36, cy - H * 0.04, W * 0.04, H * 0.18, dark))
        # Shell loop on side
        for i in range(4):
            sx = cx - W * 0.14 + i * W * 0.04
            parts.append(_circle(sx, cy + H * 0.06, W * 0.012, accent, dark, 0.3))
    elif wclass == "gl":
        # Stubby thick barrel (40mm) + drum or tube + wide receiver + heavy stock
        barrel_y = cy - H * 0.06
        parts.append(_rect(cx + W * 0.04, barrel_y, W * 0.28, H * 0.16, metal, dark, 0.5))  # thick stubby barrel
        parts.append(_circle(cx + W * 0.32, barrel_y + H * 0.08, W * 0.04, dark, metal, 0.5))  # muzzle ring
        parts.append(_circle(cx + W * 0.32, barrel_y + H * 0.08, W * 0.02, body))  # bore
        # Receiver
        parts.append(_rect(cx - W * 0.16, cy - H * 0.10, W * 0.20, H * 0.24, body, dark, 1.0))
        # 6-shot drum/cylinder magazine
        parts.append(_circle(cx - W * 0.06, cy + H * 0.02, W * 0.08, accent, dark, 1.0))
        for i in range(6):
            ang = i * 1.047  # 60 degrees
            import math as _m
            sx = cx - W * 0.06 + _m.cos(ang) * W * 0.05
            sy = cy + H * 0.02 + _m.sin(ang) * W * 0.05
            parts.append(_circle(sx, sy, W * 0.012, dark))
        # Optical sight
        parts.append(_rect(cx - W * 0.04, cy - H * 0.16, W * 0.08, H * 0.04, dark, metal, 0.5))
        # Pistol grip + stock
        parts.append(_polygon([
            (cx + W * 0.04, cy + H * 0.14),
            (cx + W * 0.10, cy + H * 0.14),
            (cx + W * 0.08, cy + H * 0.32),
            (cx + W * 0.02, cy + H * 0.32),
        ], dark, body, 0.5))
        parts.append(_polygon([
            (cx - W * 0.30, cy - H * 0.04),
            (cx - W * 0.16, cy - H * 0.10),
            (cx - W * 0.16, cy + H * 0.14),
            (cx - W * 0.30, cy + H * 0.10),
        ], body, dark, 1.0))
    elif wclass == "heavy":
        # LMG / minigun: very wide receiver + box mag or belt + bipod + heat shroud
        barrel_y = cy - H * 0.04
        # Heat shroud over barrel
        parts.append(_rect(cx + W * 0.04, barrel_y - H * 0.02, W * 0.36, H * 0.14, metal, dark, 0.5))
        # Barrel through shroud
        parts.append(_rect(cx + W * 0.04, barrel_y + H * 0.02, W * 0.40, H * 0.04, dark))
        parts.append(_rect(cx + W * 0.42, barrel_y - H * 0.02, W * 0.04, H * 0.14, dark))  # muzzle
        # Shroud vent slots
        for i in range(5):
            vx = cx + W * 0.08 + i * W * 0.06
            parts.append(_rect(vx, barrel_y - H * 0.005, W * 0.04, H * 0.012, dark))
        # Wide receiver
        parts.append(_rect(cx - W * 0.22, cy - H * 0.10, W * 0.30, H * 0.26, body, dark, 1.0))
        # Top-mounted optic + carry handle
        parts.append(_rect(cx - W * 0.12, cy - H * 0.18, W * 0.20, H * 0.06, dark, metal, 0.5))
        parts.append(_circle(cx + W * 0.04, cy - H * 0.15, W * 0.02, glow))
        # Belt feed (drum on side)
        parts.append(_circle(cx - W * 0.20, cy + H * 0.18, W * 0.10, accent, dark, 1.0))
        parts.append(_circle(cx - W * 0.20, cy + H * 0.18, W * 0.05, dark))
        # Pistol grip
        parts.append(_polygon([
            (cx + W * 0.02, cy + H * 0.16),
            (cx + W * 0.10, cy + H * 0.16),
            (cx + W * 0.08, cy + H * 0.32),
            (cx, cy + H * 0.32),
        ], dark, body, 0.5))
        # Bipod (deployed)
        parts.append(_line(cx + W * 0.18, cy + H * 0.04, cx + W * 0.10, cy + H * 0.32, dark, 2.0))
        parts.append(_line(cx + W * 0.18, cy + H * 0.04, cx + W * 0.26, cy + H * 0.32, dark, 2.0))
    elif wclass == "flamer":
        # Fuel tank + nozzle + igniter
        # Tank on back/side
        parts.append(_rect(cx - W * 0.36, cy - H * 0.18, W * 0.20, H * 0.36, accent, dark, 1.0))
        parts.append(_rect(cx - W * 0.32, cy - H * 0.22, W * 0.12, H * 0.04, dark))  # tank cap
        # Fuel hose
        parts.append(_line(cx - W * 0.16, cy - H * 0.04, cx - W * 0.04, cy - H * 0.04, dark, 3.0))
        # Receiver
        parts.append(_rect(cx - W * 0.04, cy - H * 0.08, W * 0.18, H * 0.16, body, dark, 1.0))
        # Long thin nozzle
        parts.append(_rect(cx + W * 0.14, cy - H * 0.02, W * 0.30, H * 0.04, metal, dark, 0.5))
        parts.append(_rect(cx + W * 0.42, cy - H * 0.04, W * 0.04, H * 0.08, dark))  # flame collar
        # Pilot light glow
        parts.append(_circle(cx + W * 0.40, cy, W * 0.018, glow))
        # Grip
        parts.append(_polygon([
            (cx, cy + H * 0.08),
            (cx + W * 0.06, cy + H * 0.08),
            (cx + W * 0.04, cy + H * 0.26),
            (cx - W * 0.02, cy + H * 0.26),
        ], dark, body, 0.5))
    elif wclass == "drill":
        # Lance: long shaft + drill bit cone + grip + power coupler
        # Long drill shaft
        parts.append(_rect(cx - W * 0.10, cy - H * 0.03, W * 0.40, H * 0.06, metal, dark, 0.5))
        # Spiral cone bit (3 triangles)
        for i in range(3):
            ti = i * 0.06
            parts.append(_polygon([
                (cx + W * (0.30 + ti), cy - H * 0.05),
                (cx + W * (0.36 + ti), cy),
                (cx + W * (0.30 + ti), cy + H * 0.05),
            ], dark, metal, 0.5))
        parts.append(_polygon([
            (cx + W * 0.42, cy - H * 0.07),
            (cx + W * 0.48, cy),
            (cx + W * 0.42, cy + H * 0.07),
        ], accent, dark, 1.0))  # final tip
        # Mid coupler
        parts.append(_rect(cx + W * 0.06, cy - H * 0.06, W * 0.04, H * 0.12, dark))
        # Receiver
        parts.append(_rect(cx - W * 0.18, cy - H * 0.08, W * 0.10, H * 0.18, body, dark, 1.0))
        # Power coupler with glow
        parts.append(_circle(cx - W * 0.20, cy + H * 0.02, W * 0.02, glow))
        # Grip
        parts.append(_polygon([
            (cx - W * 0.16, cy + H * 0.10),
            (cx - W * 0.10, cy + H * 0.10),
            (cx - W * 0.12, cy + H * 0.28),
            (cx - W * 0.18, cy + H * 0.28),
        ], dark, body, 0.5))
    elif wclass == "grappler":
        # Crossbow/grappling hook launcher
        # Frame with cocked tension
        parts.append(_rect(cx - W * 0.20, cy - H * 0.04, W * 0.40, H * 0.06, metal, dark, 0.5))
        # Hook (front)
        parts.append(_polygon([
            (cx + W * 0.20, cy - H * 0.06),
            (cx + W * 0.40, cy - H * 0.04),
            (cx + W * 0.36, cy),
            (cx + W * 0.40, cy + H * 0.04),
            (cx + W * 0.30, cy + H * 0.04),
        ], accent, dark, 1.0))
        # 3-pronged hook tips
        for i in range(3):
            parts.append(_polygon([
                (cx + W * 0.40, cy - H * 0.04 + i * H * 0.04),
                (cx + W * 0.46, cy - H * 0.06 + i * H * 0.04),
                (cx + W * 0.42, cy + H * 0.02 + i * H * 0.04),
            ], dark, accent, 0.5))
        # Tension cable / arms
        parts.append(_line(cx - W * 0.10, cy - H * 0.14, cx + W * 0.20, cy - H * 0.04, dark, 1.5))
        parts.append(_line(cx - W * 0.10, cy + H * 0.06, cx + W * 0.20, cy + H * 0.02, dark, 1.5))
        # Reel housing
        parts.append(_circle(cx - W * 0.04, cy + H * 0.10, W * 0.06, body, dark, 0.5))
        parts.append(_circle(cx - W * 0.04, cy + H * 0.10, W * 0.03, dark))
        # Receiver + grip
        parts.append(_rect(cx - W * 0.20, cy - H * 0.10, W * 0.14, H * 0.18, body, dark, 1.0))
        parts.append(_polygon([
            (cx - W * 0.20, cy + H * 0.08),
            (cx - W * 0.10, cy + H * 0.08),
            (cx - W * 0.14, cy + H * 0.26),
            (cx - W * 0.22, cy + H * 0.26),
        ], dark, body, 0.5))
    elif wclass == "drone":
        # Drone deployer: sphere on rail + launcher tube
        parts.append(_rect(cx - W * 0.10, cy - H * 0.04, W * 0.36, H * 0.10, metal, dark, 0.5))
        # 4 small drones queued in tube
        for i in range(4):
            parts.append(_circle(cx + W * (0.04 + i * 0.06), cy + H * 0.01, W * 0.02, accent, dark, 0.4))
            parts.append(_circle(cx + W * (0.04 + i * 0.06), cy + H * 0.01, W * 0.008, glow))
        # Lead drone exiting
        parts.append(_circle(cx + W * 0.36, cy + H * 0.01, W * 0.04, accent, dark, 0.8))
        parts.append(_circle(cx + W * 0.36, cy + H * 0.01, W * 0.018, glow))
        # 4 rotor stubs on lead drone
        for ang in [0.7853, 2.3562, 3.927, 5.4978]:
            import math as _m2
            dx = cx + W * 0.36 + _m2.cos(ang) * W * 0.05
            dy = cy + H * 0.01 + _m2.sin(ang) * W * 0.05
            parts.append(_circle(dx, dy, W * 0.012, dark))
        # Receiver
        parts.append(_rect(cx - W * 0.18, cy - H * 0.10, W * 0.10, H * 0.20, body, dark, 1.0))
        # Antenna
        parts.append(_line(cx - W * 0.14, cy - H * 0.10, cx - W * 0.14, cy - H * 0.22, dark, 1.0))
        parts.append(_circle(cx - W * 0.14, cy - H * 0.22, W * 0.01, glow))
        # Grip
        parts.append(_polygon([
            (cx - W * 0.16, cy + H * 0.08),
            (cx - W * 0.08, cy + H * 0.08),
            (cx - W * 0.10, cy + H * 0.26),
            (cx - W * 0.18, cy + H * 0.26),
        ], dark, body, 0.5))
    else:  # spec / unknown
        # Generic compact special weapon
        parts.append(_rect(cx - W * 0.20, cy - H * 0.08, W * 0.40, H * 0.18, body, dark, 1.0))
        parts.append(_rect(cx + W * 0.18, cy - H * 0.04, W * 0.18, H * 0.08, metal, dark, 0.5))
        parts.append(_circle(cx, cy, W * 0.02, glow))

    # Variant: muzzle-flash overlay (every weapon kind supports it)
    if spec.extra and spec.extra.get("variant") == "muzzle-flash":
        # Position flash at the appropriate "front" for this class
        flash_x_off = {
            "pistol": 0.30, "smg": 0.40, "rifle": 0.46, "sniper": 0.48,
            "shotgun": 0.46, "gl": 0.36, "heavy": 0.46, "flamer": 0.48,
            "drill": 0.48, "grappler": 0.46, "drone": 0.40, "spec": 0.40,
        }.get(wclass, 0.46)
        flash_y = cy
        parts.append(_polygon([
            (cx + W * flash_x_off, flash_y - H * 0.06),
            (cx + W * (flash_x_off + 0.10), flash_y - H * 0.10),
            (cx + W * (flash_x_off + 0.14), flash_y),
            (cx + W * (flash_x_off + 0.10), flash_y + H * 0.10),
            (cx + W * flash_x_off, flash_y + H * 0.06),
        ], highlight, glow, 1.0))
        parts.append(_circle(cx + W * (flash_x_off + 0.06), flash_y, W * 0.03, glow))
    elif spec.extra and spec.extra.get("variant") == "magazine-attached":
        # Extended mag for relevant classes
        mag_x_off = {
            "pistol": 0.0, "smg": -0.05, "rifle": 0.0, "sniper": -0.06,
            "shotgun": 0.0, "gl": -0.04, "heavy": -0.20,
        }.get(wclass, 0.0)
        if wclass in ("pistol", "smg", "rifle", "sniper"):
            parts.append(_rect(cx + W * mag_x_off - W * 0.03, cy + H * 0.30,
                               W * 0.10, H * 0.10, accent, dark, 0.5))
    return "".join(parts)


def _compose_actor(spec: AssetSpec, rng: random.Random) -> str:
    body_palette = spec.palette
    origin_palette = spec.origin_palette or body_palette
    skin = origin_palette.primary()
    skin_shadow = origin_palette.dark()
    skin_light = origin_palette.highlight()
    armor = body_palette.primary()
    armor_dark = body_palette.dark()
    accent = body_palette.accent()
    metal = body_palette.metal()
    glow = body_palette.glow()

    cx = spec.width / 2
    head_r = spec.width * 0.10
    head_cy = spec.height * 0.18

    stance = (spec.extra or {}).get("stance", "idle")
    facing = (spec.extra or {}).get("facing", "right")
    facing_sign = 1.0 if facing == "right" else -1.0

    # Stance offsets for limbs.
    leg_spread = {
        "idle": 0.04,
        "walking": 0.10,
        "running": 0.18,
        "crouching": 0.02,
        "prone": 0.20,
        "jetting": 0.06,
        "climbing": 0.08,
    }.get(stance, 0.04)
    body_lean = {
        "idle": 0.0,
        "walking": 0.02,
        "running": 0.05,
        "crouching": -0.05,
        "prone": 0.20,
        "jetting": -0.04,
        "climbing": -0.02,
    }.get(stance, 0.0)

    body_yofs = body_lean * spec.height

    parts: List[str] = []

    if stance == "prone":
        # Prone: horizontal silhouette.
        torso_x = spec.width * 0.10
        torso_y = spec.height * 0.55
        parts.append(_rect(torso_x, torso_y, spec.width * 0.65, spec.height * 0.16, armor, armor_dark, 1.0))
        parts.append(_circle(torso_x + spec.width * 0.7 * facing_sign, torso_y + spec.height * 0.07,
                             head_r, skin, skin_shadow, 0.5))
        parts.append(_rect(torso_x - spec.width * 0.08, torso_y + spec.height * 0.16,
                           spec.width * 0.18, spec.height * 0.06, armor_dark))
        parts.append(_rect(torso_x + spec.width * 0.40, torso_y + spec.height * 0.16,
                           spec.width * 0.40, spec.height * 0.06, armor_dark))
        return "".join(parts)

    # Standing-style silhouette.
    head_cx = cx + facing_sign * spec.width * 0.02
    parts.append(_circle(head_cx, head_cy + body_yofs, head_r, skin, skin_shadow, 0.5))
    # Helmet rim
    parts.append(_rect(head_cx - head_r * 1.05, head_cy - head_r * 0.4 + body_yofs,
                       head_r * 2.10, head_r * 0.5, armor, armor_dark, 0.5))
    # Visor
    parts.append(_rect(head_cx - head_r * 0.8 + facing_sign * head_r * 0.2,
                       head_cy - head_r * 0.1 + body_yofs,
                       head_r * 1.2, head_r * 0.3, accent))

    # Torso
    torso_w = spec.width * 0.34
    torso_h = spec.height * 0.34
    torso_x = cx - torso_w / 2
    torso_y = spec.height * 0.28 + body_yofs
    parts.append(_rect(torso_x, torso_y, torso_w, torso_h, armor, armor_dark, 1.0))
    # Pauldrons
    parts.append(_polygon(
        [
            (torso_x, torso_y),
            (torso_x + torso_w * 0.25, torso_y - torso_h * 0.15),
            (torso_x + torso_w * 0.05, torso_y + torso_h * 0.10),
        ],
        armor_dark, accent, 0.5,
    ))
    parts.append(_polygon(
        [
            (torso_x + torso_w, torso_y),
            (torso_x + torso_w * 0.75, torso_y - torso_h * 0.15),
            (torso_x + torso_w * 0.95, torso_y + torso_h * 0.10),
        ],
        armor_dark, accent, 0.5,
    ))
    # Chest emblem
    parts.append(_circle(cx, torso_y + torso_h * 0.45, torso_w * 0.10, accent, glow, 0.5))

    # Arms
    arm_w = spec.width * 0.07
    arm_h = spec.height * 0.30
    arm_y = torso_y + torso_h * 0.05
    parts.append(_rect(torso_x - arm_w * 0.9, arm_y, arm_w, arm_h, armor, armor_dark, 0.5))
    parts.append(_rect(torso_x + torso_w - arm_w * 0.1 + facing_sign * arm_w * 0.2, arm_y,
                       arm_w, arm_h, armor, armor_dark, 0.5))

    # Hands
    parts.append(_circle(torso_x - arm_w * 0.4, arm_y + arm_h, arm_w * 0.5, skin_light))
    parts.append(_circle(torso_x + torso_w + arm_w * 0.4 + facing_sign * arm_w * 0.2,
                         arm_y + arm_h, arm_w * 0.5, skin_light))

    # Role accessories — temporary placement before legs (use only torso-scope vars)
    role_name = spec.canonical_name.lower()
    if "medic" in role_name:
        # Red cross emblem on chest (larger than default), medkit on back
        parts.append(_rect(cx - torso_w * 0.10, torso_y + torso_h * 0.30,
                           torso_w * 0.20, torso_w * 0.20, "#FFFFFF", "#990000", 1.0))
        parts.append(_rect(cx - torso_w * 0.03, torso_y + torso_h * 0.30,
                           torso_w * 0.06, torso_w * 0.20, "#CC0000"))
        parts.append(_rect(cx - torso_w * 0.10, torso_y + torso_h * 0.30 + torso_w * 0.07,
                           torso_w * 0.20, torso_w * 0.06, "#CC0000"))
        # Medkit on back
        parts.append(_rect(torso_x - spec.width * 0.05, torso_y + torso_h * 0.20,
                           spec.width * 0.06, torso_h * 0.30, "#DDDDDD", "#666666", 0.8))
        parts.append(_rect(torso_x - spec.width * 0.05, torso_y + torso_h * 0.30,
                           spec.width * 0.06, torso_h * 0.04, "#990000"))
    elif "engineer" in role_name:
        # Wrench on belt + tool pack on back
        parts.append(_polygon([
            (torso_x + torso_w * 0.15, torso_y + torso_h * 0.85),
            (torso_x + torso_w * 0.30, torso_y + torso_h * 0.85),
            (torso_x + torso_w * 0.32, torso_y + torso_h * 0.90),
            (torso_x + torso_w * 0.13, torso_y + torso_h * 0.90),
        ], "#999999", "#444444", 0.5))
        # Tool pack on back (square)
        parts.append(_rect(torso_x - spec.width * 0.06, torso_y + torso_h * 0.18,
                           spec.width * 0.07, torso_h * 0.45, accent, armor_dark, 0.8))
        # Bolt detail on tool pack
        for i in range(2):
            parts.append(_circle(torso_x - spec.width * 0.025, torso_y + torso_h * (0.28 + i * 0.20),
                                 spec.width * 0.008, metal))
    elif "marksman" in role_name or "sniper" in role_name:
        # Scope on back + long-rifle hint
        parts.append(_rect(torso_x - spec.width * 0.06, torso_y + torso_h * 0.10,
                           spec.width * 0.06, spec.height * 0.30, armor_dark, metal, 0.5))
        parts.append(_rect(torso_x - spec.width * 0.07, torso_y + torso_h * 0.15,
                           spec.width * 0.08, spec.height * 0.04, "#222222"))
        parts.append(_circle(torso_x - spec.width * 0.06, torso_y + torso_h * 0.18,
                             spec.width * 0.015, glow))
        # Range pip on shoulder
        parts.append(_circle(torso_x + torso_w * 0.10, torso_y, spec.width * 0.018, accent))
    elif "heavy" in role_name:
        # Extra armor plates + bigger pauldrons + chest ammo belt
        # Bigger pauldron shoulder caps
        parts.append(_polygon([
            (torso_x - torso_w * 0.05, torso_y),
            (torso_x + torso_w * 0.20, torso_y - torso_h * 0.15),
            (torso_x + torso_w * 0.10, torso_y + torso_h * 0.20),
        ], armor, armor_dark, 0.8))
        parts.append(_polygon([
            (torso_x + torso_w * 1.05, torso_y),
            (torso_x + torso_w * 0.80, torso_y - torso_h * 0.15),
            (torso_x + torso_w * 0.90, torso_y + torso_h * 0.20),
        ], armor, armor_dark, 0.8))
        # Ammo belt across chest
        parts.append(_rect(torso_x + torso_w * 0.05, torso_y + torso_h * 0.40,
                           torso_w * 0.90, torso_h * 0.06, accent, armor_dark, 0.5))
        for i in range(6):
            parts.append(_rect(torso_x + torso_w * (0.10 + i * 0.13), torso_y + torso_h * 0.41,
                               torso_w * 0.04, torso_h * 0.04, "#CC8800"))
        # Backpack ammo case
        parts.append(_rect(torso_x - spec.width * 0.07, torso_y + torso_h * 0.20,
                           spec.width * 0.08, torso_h * 0.50, armor, armor_dark, 0.8))
    elif "assault" in role_name or "berserker" in role_name:
        # Chest grenade strap (3 grenades) + waist pouch
        for i in range(3):
            gx = cx - torso_w * 0.15 + i * torso_w * 0.15
            parts.append(_circle(gx, torso_y + torso_h * 0.50, torso_w * 0.05, "#444444", "#222222", 0.5))
            parts.append(_rect(gx - torso_w * 0.015, torso_y + torso_h * 0.46,
                               torso_w * 0.03, torso_w * 0.02, "#666666"))
        # Waist pouch (use torso coords, not leg)
        parts.append(_rect(torso_x + torso_w * 0.15, torso_y + torso_h * 0.85,
                           torso_w * 0.70, torso_h * 0.10, accent, armor_dark, 0.5))
    elif "hunter" in role_name or "scout" in role_name or "spotter" in role_name:
        # Binocular goggles on head + light back pack
        parts.append(_rect(head_cx - head_r * 0.7, head_cy - head_r * 0.4 + body_yofs - head_r * 0.05,
                           head_r * 1.4, head_r * 0.2, armor_dark, dark_or(armor_dark, "#000000"), 0.5))
        parts.append(_circle(head_cx - head_r * 0.35, head_cy - head_r * 0.30 + body_yofs,
                             head_r * 0.18, glow, armor_dark, 0.5))
        parts.append(_circle(head_cx + head_r * 0.35, head_cy - head_r * 0.30 + body_yofs,
                             head_r * 0.18, glow, armor_dark, 0.5))
        # Light pack on back
        parts.append(_rect(torso_x - spec.width * 0.04, torso_y + torso_h * 0.18,
                           spec.width * 0.05, torso_h * 0.35, armor, armor_dark, 0.5))
    elif "shaman" in role_name or "diplomat" in role_name or "chieftain" in role_name or "commander" in role_name:
        # Ritual ornament / shoulder banner + neck adornment
        parts.append(_polygon([
            (torso_x + torso_w + spec.width * 0.04, torso_y - torso_h * 0.10),
            (torso_x + torso_w + spec.width * 0.10, torso_y - torso_h * 0.18),
            (torso_x + torso_w + spec.width * 0.08, torso_y + torso_h * 0.10),
            (torso_x + torso_w + spec.width * 0.02, torso_y + torso_h * 0.18),
        ], accent, armor_dark, 0.8))
        # Neck adornment
        parts.append(_circle(cx, torso_y - torso_h * 0.02, torso_w * 0.05, glow, accent, 0.5))
        # Crown or ritual hat (chieftain/commander)
        if "chieftain" in role_name or "commander" in role_name:
            parts.append(_polygon([
                (head_cx - head_r * 0.6, head_cy - head_r * 0.4 + body_yofs - head_r * 0.10),
                (head_cx - head_r * 0.3, head_cy - head_r * 0.4 + body_yofs - head_r * 0.4),
                (head_cx, head_cy - head_r * 0.4 + body_yofs - head_r * 0.20),
                (head_cx + head_r * 0.3, head_cy - head_r * 0.4 + body_yofs - head_r * 0.4),
                (head_cx + head_r * 0.6, head_cy - head_r * 0.4 + body_yofs - head_r * 0.10),
            ], accent, armor_dark, 0.5))
    elif "scientist" in role_name:
        # Glasses + clipboard + clean lab attire
        # Eye-glasses
        parts.append(_circle(head_cx - head_r * 0.35, head_cy - head_r * 0.10 + body_yofs,
                             head_r * 0.12, "none", "#222222", 1.0))
        parts.append(_circle(head_cx + head_r * 0.35, head_cy - head_r * 0.10 + body_yofs,
                             head_r * 0.12, "none", "#222222", 1.0))
        parts.append(_line(head_cx - head_r * 0.23, head_cy - head_r * 0.10 + body_yofs,
                           head_cx + head_r * 0.23, head_cy - head_r * 0.10 + body_yofs,
                           "#222222", 1.0))
        # Clipboard at side
        parts.append(_rect(torso_x + torso_w * 1.05, torso_y + torso_h * 0.40,
                           spec.width * 0.06, torso_h * 0.35, "#FFFFEE", "#666666", 0.5))
        parts.append(_rect(torso_x + torso_w * 1.07, torso_y + torso_h * 0.45,
                           spec.width * 0.04, torso_h * 0.02, "#222222"))
        parts.append(_rect(torso_x + torso_w * 1.07, torso_y + torso_h * 0.50,
                           spec.width * 0.04, torso_h * 0.02, "#222222"))
    elif "worker" in role_name:
        # Hard hat + tool belt
        parts.append(_polygon([
            (head_cx - head_r * 0.9, head_cy - head_r * 0.3 + body_yofs),
            (head_cx + head_r * 0.9, head_cy - head_r * 0.3 + body_yofs),
            (head_cx + head_r * 0.7, head_cy - head_r * 0.7 + body_yofs),
            (head_cx - head_r * 0.7, head_cy - head_r * 0.7 + body_yofs),
        ], "#FFAA22", "#995500", 1.0))
        parts.append(_rect(head_cx - head_r * 1.0, head_cy - head_r * 0.3 + body_yofs,
                           head_r * 2.0, head_r * 0.08, "#995500"))
        # Tool belt across hip
        parts.append(_rect(torso_x, torso_y + torso_h * 0.85,
                           torso_w, torso_h * 0.10, "#886633", "#553311", 0.5))
        # 2 tool holsters on belt
        parts.append(_rect(torso_x + torso_w * 0.10, torso_y + torso_h * 0.95,
                           torso_w * 0.10, torso_h * 0.10, "#553311"))
        parts.append(_rect(torso_x + torso_w * 0.78, torso_y + torso_h * 0.95,
                           torso_w * 0.10, torso_h * 0.10, "#553311"))

    # Legs
    leg_w = spec.width * 0.10
    leg_h = spec.height * 0.30
    leg_y = torso_y + torso_h
    left_x = cx - leg_w * 1.2 - leg_spread * spec.width / 2
    right_x = cx + leg_w * 0.2 + leg_spread * spec.width / 2
    parts.append(_rect(left_x, leg_y, leg_w, leg_h, armor_dark, dark_or(armor_dark, "#000000"), 0.5))
    parts.append(_rect(right_x, leg_y, leg_w, leg_h, armor_dark, dark_or(armor_dark, "#000000"), 0.5))
    # Boots
    parts.append(_rect(left_x - leg_w * 0.1, leg_y + leg_h, leg_w * 1.2, spec.height * 0.04, metal))
    parts.append(_rect(right_x - leg_w * 0.1, leg_y + leg_h, leg_w * 1.2, spec.height * 0.04, metal))

    # Stance-specific decorations
    if stance == "running":
        parts.append(_line(cx - leg_w, leg_y + leg_h * 0.5, cx - leg_w * 2.0, leg_y + leg_h * 0.6,
                           glow, 1.0))
    elif stance == "jetting":
        # Jet flame trail
        parts.append(_polygon(
            [
                (cx, spec.height * 0.94),
                (cx - leg_w * 0.6, spec.height * 0.99),
                (cx, spec.height * 0.96),
                (cx + leg_w * 0.6, spec.height * 0.99),
            ],
            glow, accent, 0.5,
        ))
    elif stance == "climbing":
        parts.append(_line(left_x - leg_w * 0.4, leg_y, left_x - leg_w * 0.8, leg_y - leg_h * 0.4,
                           metal, 1.5))
        parts.append(_line(right_x + leg_w * 1.2, leg_y, right_x + leg_w * 1.6, leg_y - leg_h * 0.4,
                           metal, 1.5))
    return "".join(parts)


def dark_or(color: str, fallback: str) -> str:
    """Tiny helper used inside _compose_actor when the palette doesn't expose `dark`."""
    return color or fallback


def _compose_vehicle(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    dark = p.dark()
    accent = p.accent()
    metal = p.metal()
    glow = p.glow()
    highlight = p.highlight()
    variant = (spec.extra or {}).get("variant", "side")
    name = spec.canonical_name.lower()
    W = spec.width
    H = spec.height
    parts: List[str] = []

    # ── Per-archetype silhouettes ──────────────────────────────────────────
    if "apc_treaded" in name:
        # Tracked hull + riveted plates + side hatch + boxy turret
        hull_y = H * 0.36
        hull_h = H * 0.30
        parts.append(_rect(W * 0.08, hull_y, W * 0.84, hull_h, body, dark, 1.2))
        # Rivets
        for ix in [0.12, 0.30, 0.50, 0.70, 0.88]:
            for iy in [0.40, 0.60]:
                parts.append(_circle(W * ix, H * iy, 2.0, dark, metal, 0.5))
        # Side hatch
        parts.append(_rect(W * 0.40, hull_y + H * 0.08, W * 0.12, H * 0.16, dark, accent, 1.0))
        # Boxy turret
        parts.append(_rect(W * 0.45, hull_y - H * 0.10, W * 0.20, H * 0.10, body, dark, 1.0))
        parts.append(_rect(W * 0.62, hull_y - H * 0.05, W * 0.18, H * 0.04, metal, dark, 0.5))
        # Viewport
        parts.append(_rect(W * 0.66, hull_y + H * 0.04, W * 0.16, H * 0.08, accent, dark, 0.5))
        # Treads
        parts.append(_rect(W * 0.04, hull_y + hull_h, W * 0.92, H * 0.14, dark, metal, 0.5))
        for i in range(10):
            tx = W * (0.06 + i * 0.09)
            parts.append(_rect(tx, hull_y + hull_h + H * 0.02, W * 0.06, H * 0.10, metal, dark, 0.5))
        # Tread road wheels
        for i in range(5):
            wx = W * (0.14 + i * 0.18)
            parts.append(_circle(wx, hull_y + hull_h + H * 0.07, H * 0.04, metal, dark, 0.5))
    elif "supply_truck" in name or "recovery_truck" in name:
        # Cab + flat bed + slatted side armor + 6 wheels
        cab_x = W * 0.06
        cab_y = H * 0.30
        # Cab
        parts.append(_rect(cab_x, cab_y, W * 0.24, H * 0.30, body, dark, 1.2))
        parts.append(_polygon([
            (cab_x, cab_y),
            (cab_x + W * 0.24, cab_y),
            (cab_x + W * 0.24, cab_y + H * 0.10),
            (cab_x + W * 0.04, cab_y + H * 0.04),
        ], dark))
        # Cab window
        parts.append(_rect(cab_x + W * 0.04, cab_y + H * 0.06, W * 0.18, H * 0.10, accent, dark, 0.5))
        # Flat bed
        parts.append(_rect(W * 0.30, H * 0.40, W * 0.62, H * 0.24, _lighten_hex(body, 0.05), dark, 1.0))
        # Slatted side armor
        for i in range(8):
            parts.append(_rect(W * 0.32 + i * W * 0.07, H * 0.42,
                               W * 0.04, H * 0.18, _darken_hex(body, 0.10), dark, 0.3))
        # 6 wheels
        for i in range(6):
            wx = W * (0.12 + i * 0.14)
            parts.append(_circle(wx, H * 0.76, H * 0.07, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.76, H * 0.04, metal))
        # Winch (recovery_truck variant)
        if "recovery" in name:
            parts.append(_rect(W * 0.84, H * 0.32, W * 0.10, H * 0.10, metal, dark, 1.0))
            parts.append(_line(W * 0.88, H * 0.36, W * 0.96, H * 0.50, dark, 2.0))
            parts.append(_polygon([
                (W * 0.94, H * 0.48),
                (W * 0.98, H * 0.46),
                (W * 0.96, H * 0.54),
            ], metal, dark, 0.5))
    elif "light_tank" in name:
        # Sloped front + main gun + 2 tracks + commander hatch
        hull_y = H * 0.40
        hull_h = H * 0.24
        # Sloped front armor
        parts.append(_polygon([
            (W * 0.08, hull_y + hull_h),
            (W * 0.08, hull_y + H * 0.10),
            (W * 0.20, hull_y),
            (W * 0.85, hull_y),
            (W * 0.92, hull_y + hull_h),
        ], body, dark, 1.2))
        # Plate seams
        for sx in [0.30, 0.45, 0.60, 0.75]:
            parts.append(_line(W * sx, hull_y, W * sx, hull_y + hull_h, dark, 0.6))
        # Turret (round)
        parts.append(_polygon([
            (W * 0.30, hull_y),
            (W * 0.65, hull_y),
            (W * 0.65, hull_y - H * 0.12),
            (W * 0.30, hull_y - H * 0.12),
        ], body, dark, 1.0))
        # Commander hatch
        parts.append(_circle(W * 0.42, hull_y - H * 0.10, W * 0.025, dark, metal, 0.5))
        # Main gun
        parts.append(_rect(W * 0.55, hull_y - H * 0.07, W * 0.40, H * 0.04, metal, dark, 0.5))
        parts.append(_rect(W * 0.92, hull_y - H * 0.09, W * 0.03, H * 0.08, dark))
        # 2 tracks
        parts.append(_rect(W * 0.06, hull_y + hull_h, W * 0.86, H * 0.12, dark, metal, 0.5))
        for i in range(8):
            tx = W * (0.09 + i * 0.10)
            parts.append(_rect(tx, hull_y + hull_h + H * 0.02, W * 0.04, H * 0.08, metal, dark, 0.3))
    elif "pickup" in name:
        # Cabin + bed + roll bar + 4 wheels + mounted weapon ring
        # Cabin
        parts.append(_rect(W * 0.08, H * 0.40, W * 0.34, H * 0.24, body, dark, 1.2))
        parts.append(_polygon([
            (W * 0.08, H * 0.40),
            (W * 0.42, H * 0.40),
            (W * 0.36, H * 0.30),
            (W * 0.12, H * 0.30),
        ], _darken_hex(body, 0.15), dark, 0.8))
        # Cabin windows
        parts.append(_rect(W * 0.14, H * 0.32, W * 0.20, H * 0.08, accent, dark, 0.5))
        # Bed
        parts.append(_rect(W * 0.42, H * 0.46, W * 0.50, H * 0.18, _lighten_hex(body, 0.05), dark, 1.0))
        # Roll bar
        parts.append(_polygon([
            (W * 0.50, H * 0.46),
            (W * 0.50, H * 0.24),
            (W * 0.54, H * 0.22),
            (W * 0.86, H * 0.22),
            (W * 0.90, H * 0.24),
            (W * 0.90, H * 0.46),
            (W * 0.86, H * 0.46),
            (W * 0.86, H * 0.28),
            (W * 0.54, H * 0.28),
            (W * 0.54, H * 0.46),
        ], dark, metal, 1.0))
        # Mounted weapon
        parts.append(_circle(W * 0.70, H * 0.30, W * 0.05, dark, metal, 1.0))
        parts.append(_rect(W * 0.72, H * 0.28, W * 0.20, H * 0.02, metal, dark, 0.5))
        # 4 wheels
        for i in range(4):
            wx = W * (0.15 + i * 0.22)
            parts.append(_circle(wx, H * 0.76, H * 0.07, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.76, H * 0.04, metal))
    elif "buggy" in name and "war" not in name:
        # Open frame + roll cage + 4 wheels + driver figure
        # Roll cage frame
        parts.append(_polygon([
            (W * 0.14, H * 0.62),
            (W * 0.14, H * 0.32),
            (W * 0.26, H * 0.24),
            (W * 0.76, H * 0.24),
            (W * 0.88, H * 0.32),
            (W * 0.88, H * 0.62),
        ], "none", dark, max(2.0, W * 0.012)))
        # Floor
        parts.append(_rect(W * 0.14, H * 0.54, W * 0.74, H * 0.10, body, dark, 1.0))
        # Internal cross-bracing
        parts.append(_line(W * 0.14, H * 0.32, W * 0.88, H * 0.32, dark, 1.5))
        parts.append(_line(W * 0.14, H * 0.44, W * 0.88, H * 0.44, dark, 1.0))
        parts.append(_line(W * 0.50, H * 0.24, W * 0.50, H * 0.62, dark, 1.0))
        # Driver figure outline
        parts.append(_circle(W * 0.34, H * 0.40, H * 0.04, _darken_hex(body, 0.20), dark, 0.5))
        parts.append(_rect(W * 0.30, H * 0.44, W * 0.08, H * 0.10, _darken_hex(body, 0.20), dark, 0.5))
        # Engine in back
        parts.append(_rect(W * 0.70, H * 0.34, W * 0.16, H * 0.18, metal, dark, 0.8))
        for i in range(3):
            parts.append(_circle(W * 0.78, H * 0.36 + i * H * 0.04, W * 0.012, accent))
        # 4 wheels (knobby)
        for i in range(4):
            wx = W * (0.16 + i * 0.22)
            parts.append(_circle(wx, H * 0.76, H * 0.08, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.76, H * 0.05, metal))
            # Tread knobs
            import math as _mvb
            for j in range(8):
                ang = j * 0.7854
                tx = wx + H * 0.07 * _mvb.cos(ang)
                ty = H * 0.76 + H * 0.07 * _mvb.sin(ang)
                parts.append(_circle(tx, ty, 1.5, dark))
    elif "motorcycle" in name:
        # 2 wheels + fuel tank + handlebars + rider seat
        # Front wheel
        parts.append(_circle(W * 0.20, H * 0.70, H * 0.14, dark, metal, 1.5))
        parts.append(_circle(W * 0.20, H * 0.70, H * 0.08, metal, dark, 0.5))
        parts.append(_circle(W * 0.20, H * 0.70, H * 0.03, dark))
        # Spokes
        import math as _mmc
        for i in range(8):
            ang = i * 0.7854
            sx = W * 0.20 + H * 0.12 * _mmc.cos(ang)
            sy = H * 0.70 + H * 0.12 * _mmc.sin(ang)
            parts.append(_line(W * 0.20, H * 0.70, sx, sy, dark, 0.8))
        # Rear wheel
        parts.append(_circle(W * 0.74, H * 0.70, H * 0.14, dark, metal, 1.5))
        parts.append(_circle(W * 0.74, H * 0.70, H * 0.08, metal, dark, 0.5))
        parts.append(_circle(W * 0.74, H * 0.70, H * 0.03, dark))
        for i in range(8):
            ang = i * 0.7854
            sx = W * 0.74 + H * 0.12 * _mmc.cos(ang)
            sy = H * 0.70 + H * 0.12 * _mmc.sin(ang)
            parts.append(_line(W * 0.74, H * 0.70, sx, sy, dark, 0.8))
        # Frame
        parts.append(_line(W * 0.20, H * 0.70, W * 0.40, H * 0.50, dark, 3.0))
        parts.append(_line(W * 0.74, H * 0.70, W * 0.55, H * 0.50, dark, 3.0))
        parts.append(_line(W * 0.40, H * 0.50, W * 0.55, H * 0.50, dark, 3.0))
        # Fuel tank
        parts.append(_polygon([
            (W * 0.40, H * 0.48),
            (W * 0.56, H * 0.48),
            (W * 0.58, H * 0.40),
            (W * 0.38, H * 0.40),
        ], body, dark, 1.0))
        # Seat
        parts.append(_polygon([
            (W * 0.56, H * 0.48),
            (W * 0.66, H * 0.50),
            (W * 0.66, H * 0.46),
            (W * 0.56, H * 0.44),
        ], dark, metal, 0.5))
        # Handlebars
        parts.append(_line(W * 0.30, H * 0.36, W * 0.40, H * 0.42, dark, 2.5))
        parts.append(_line(W * 0.30, H * 0.36, W * 0.34, H * 0.30, dark, 2.5))
        parts.append(_line(W * 0.30, H * 0.36, W * 0.26, H * 0.30, dark, 2.5))
        # Headlight
        parts.append(_circle(W * 0.30, H * 0.42, H * 0.04, glow, dark, 0.5))
    elif "war_buggy" in name:
        # Bone-spiked frame + skull totem + 4 wheels
        # Frame
        parts.append(_polygon([
            (W * 0.14, H * 0.62),
            (W * 0.14, H * 0.32),
            (W * 0.26, H * 0.24),
            (W * 0.76, H * 0.24),
            (W * 0.88, H * 0.32),
            (W * 0.88, H * 0.62),
        ], "#553311", dark, max(2.0, W * 0.012)))
        # Floor
        parts.append(_rect(W * 0.14, H * 0.54, W * 0.74, H * 0.10, "#3a2a13", dark, 1.0))
        # Skull totem (top center)
        parts.append(_circle(W * 0.50, H * 0.18, W * 0.05, "#eeddcc", dark, 1.0))
        parts.append(_circle(W * 0.48, H * 0.18, W * 0.012, dark))
        parts.append(_circle(W * 0.52, H * 0.18, W * 0.012, dark))
        parts.append(_polygon([
            (W * 0.50, H * 0.20),
            (W * 0.48, H * 0.22),
            (W * 0.52, H * 0.22),
        ], dark))
        # Bone spikes (4)
        for sx_off in [0.18, 0.38, 0.62, 0.82]:
            parts.append(_polygon([
                (W * sx_off, H * 0.24),
                (W * sx_off + W * 0.02, H * 0.16),
                (W * sx_off + W * 0.04, H * 0.24),
            ], "#eeddcc", dark, 0.5))
        # Engine
        parts.append(_rect(W * 0.74, H * 0.34, W * 0.14, H * 0.18, metal, dark, 0.8))
        # 4 wheels
        for i in range(4):
            wx = W * (0.16 + i * 0.22)
            parts.append(_circle(wx, H * 0.76, H * 0.08, dark, "#cc8822", 0.8))
            parts.append(_circle(wx, H * 0.76, H * 0.04, "#553311"))
    elif "war_wagon" in name:
        # Ornate sled + rune carving + 4 wheels + driver
        parts.append(_polygon([
            (W * 0.08, H * 0.36),
            (W * 0.92, H * 0.36),
            (W * 0.96, H * 0.64),
            (W * 0.04, H * 0.64),
        ], "#553311", dark, 1.2))
        # Ornate front
        parts.append(_polygon([
            (W * 0.92, H * 0.36),
            (W * 0.98, H * 0.50),
            (W * 0.96, H * 0.64),
        ], "#cc8822", dark, 0.8))
        # Rune carvings (5 rectangles)
        for i in range(5):
            parts.append(_rect(W * (0.14 + i * 0.16), H * 0.42, W * 0.10, H * 0.16,
                               _darken_hex("#553311", 0.20), accent, 0.5))
            # Carved rune (simple cross)
            parts.append(_line(W * (0.16 + i * 0.16), H * 0.46,
                               W * (0.22 + i * 0.16), H * 0.46, accent, 1.0))
            parts.append(_line(W * (0.19 + i * 0.16), H * 0.44,
                               W * (0.19 + i * 0.16), H * 0.54, accent, 1.0))
        # Driver figure
        parts.append(_circle(W * 0.16, H * 0.30, H * 0.04, _darken_hex(body, 0.20), dark, 0.5))
        # 4 wheels
        for i in range(4):
            wx = W * (0.18 + i * 0.22)
            parts.append(_circle(wx, H * 0.76, H * 0.08, dark, "#cc8822", 0.5))
            parts.append(_circle(wx, H * 0.76, H * 0.04, "#cc8822"))
    elif "cathedral_drone" in name:
        # Ornate hover-shape with gold trim + cross emblem
        parts.append(_polygon([
            (W * 0.20, H * 0.50),
            (W * 0.30, H * 0.32),
            (W * 0.70, H * 0.32),
            (W * 0.80, H * 0.50),
            (W * 0.70, H * 0.68),
            (W * 0.30, H * 0.68),
        ], "#5a2266", dark, max(2.0, W * 0.012)))
        # Gold trim
        parts.append(_polygon([
            (W * 0.24, H * 0.48),
            (W * 0.32, H * 0.36),
            (W * 0.68, H * 0.36),
            (W * 0.76, H * 0.48),
            (W * 0.68, H * 0.64),
            (W * 0.32, H * 0.64),
        ], "none", "#ddaa22", 2.5))
        # Cross emblem center
        parts.append(_rect(W * 0.48, H * 0.40, W * 0.04, H * 0.24, "#ddaa22", dark, 0.5))
        parts.append(_rect(W * 0.40, H * 0.46, W * 0.20, H * 0.04, "#ddaa22", dark, 0.5))
        # Hover glow underneath
        parts.append(_polygon([
            (W * 0.30, H * 0.68),
            (W * 0.70, H * 0.68),
            (W * 0.60, H * 0.84),
            (W * 0.40, H * 0.84),
        ], glow, "#ddaa22", 0.5))
        # Ornate antennae
        parts.append(_line(W * 0.30, H * 0.32, W * 0.26, H * 0.18, "#ddaa22", 1.5))
        parts.append(_line(W * 0.70, H * 0.32, W * 0.74, H * 0.18, "#ddaa22", 1.5))
        parts.append(_circle(W * 0.26, H * 0.18, W * 0.012, "#ddaa22"))
        parts.append(_circle(W * 0.74, H * 0.18, W * 0.012, "#ddaa22"))
    elif "relic_carrier" in name:
        # Flat bed with banner mast + relic crate
        # Cab
        parts.append(_rect(W * 0.06, H * 0.34, W * 0.20, H * 0.28, "#5a2266", dark, 1.0))
        parts.append(_rect(W * 0.10, H * 0.38, W * 0.12, H * 0.10, accent, dark, 0.5))
        # Flat bed
        parts.append(_rect(W * 0.26, H * 0.50, W * 0.66, H * 0.14, _darken_hex("#5a2266", 0.10), dark, 1.0))
        # Banner mast
        parts.append(_rect(W * 0.50, H * 0.10, W * 0.012, H * 0.40, "#ddaa22", dark, 0.5))
        # Banner flag
        parts.append(_polygon([
            (W * 0.512, H * 0.12),
            (W * 0.66, H * 0.16),
            (W * 0.66, H * 0.28),
            (W * 0.512, H * 0.32),
        ], accent, "#ddaa22", 1.0))
        # Relic crate
        parts.append(_rect(W * 0.30, H * 0.40, W * 0.30, H * 0.20, "#cc8844", dark, 1.5))
        # Cross on crate
        parts.append(_rect(W * 0.44, H * 0.42, W * 0.02, H * 0.16, "#ddaa22"))
        parts.append(_rect(W * 0.38, H * 0.48, W * 0.14, H * 0.04, "#ddaa22"))
        # Wheels
        for i in range(4):
            wx = W * (0.14 + i * 0.22)
            parts.append(_circle(wx, H * 0.76, H * 0.07, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.76, H * 0.04, metal))
    elif "research_apc" in name or "modular_apc" in name:
        # Clean rectangular hull + dish + strut roof + sensor mast
        hull_y = H * 0.36
        # Hull
        parts.append(_rect(W * 0.08, hull_y, W * 0.84, H * 0.30, body, dark, 1.2))
        # Strut roof
        parts.append(_rect(W * 0.10, hull_y - H * 0.04, W * 0.80, H * 0.04, metal, dark, 0.5))
        # Plate seams
        if "modular" in name:
            for sx in [0.25, 0.40, 0.55, 0.70]:
                parts.append(_rect(W * sx, hull_y + H * 0.02, W * 0.012, H * 0.26, accent))
        # Dish
        parts.append(_polygon([
            (W * 0.30, hull_y - H * 0.18),
            (W * 0.50, hull_y - H * 0.18),
            (W * 0.46, hull_y - H * 0.04),
            (W * 0.34, hull_y - H * 0.04),
        ], metal, dark, 0.8))
        parts.append(_circle(W * 0.40, hull_y - H * 0.11, W * 0.018, accent))
        # Sensor mast
        parts.append(_rect(W * 0.60, hull_y - H * 0.20, W * 0.012, H * 0.18, dark))
        parts.append(_circle(W * 0.606, hull_y - H * 0.20, W * 0.018, glow, accent, 0.5))
        # Side viewport
        parts.append(_rect(W * 0.62, hull_y + H * 0.08, W * 0.20, H * 0.08, accent, dark, 0.5))
        # Wheels
        for i in range(4):
            wx = W * (0.16 + i * 0.20)
            parts.append(_circle(wx, H * 0.78, H * 0.07, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.78, H * 0.04, metal))
    elif "data_rover" in name:
        # Wheeled buggy + holo-emitter mast
        # Compact body
        parts.append(_rect(W * 0.16, H * 0.40, W * 0.50, H * 0.22, body, dark, 1.0))
        parts.append(_polygon([
            (W * 0.16, H * 0.40),
            (W * 0.66, H * 0.40),
            (W * 0.60, H * 0.32),
            (W * 0.22, H * 0.32),
        ], _darken_hex(body, 0.15), dark, 0.8))
        # Viewport
        parts.append(_rect(W * 0.22, H * 0.34, W * 0.36, H * 0.06, accent, dark, 0.5))
        # Holo-emitter mast
        parts.append(_rect(W * 0.66, H * 0.30, W * 0.012, H * 0.16, dark))
        # Holographic emitter
        parts.append(_circle(W * 0.666, H * 0.30, W * 0.04, glow, "#00FFAA", 1.0))
        # Hologram projection (3 rings)
        for i in range(3):
            r = W * (0.06 + i * 0.04)
            parts.append(_circle(W * 0.666, H * 0.20, r, "none", "#00FFAA", 1.0))
        # 4 wheels
        for i in range(4):
            wx = W * (0.22 + i * 0.13)
            parts.append(_circle(wx, H * 0.74, H * 0.06, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.74, H * 0.03, metal))
    elif "pirate_skiff" in name or "armored_boat" in name:
        # Boat hull + mast + sail + figurehead
        # Hull (boat shape)
        parts.append(_polygon([
            (W * 0.06, H * 0.58),
            (W * 0.94, H * 0.58),
            (W * 0.88, H * 0.74),
            (W * 0.12, H * 0.74),
        ], body, dark, 1.2))
        # Hull planks
        for i in range(4):
            parts.append(_line(W * 0.06, H * (0.60 + i * 0.035),
                               W * 0.94, H * (0.60 + i * 0.035), dark, 0.5))
        # Mast
        parts.append(_rect(W * 0.50, H * 0.10, W * 0.014, H * 0.48, "#553311", dark, 0.5))
        # Sail
        parts.append(_polygon([
            (W * 0.50, H * 0.14),
            (W * 0.30, H * 0.30),
            (W * 0.50, H * 0.40),
        ], "#dddddd" if "pirate" in name else metal, dark, 1.0))
        # Pirate skull on sail
        if "pirate" in name:
            parts.append(_circle(W * 0.40, H * 0.26, W * 0.025, "#eeddcc", dark, 0.5))
            parts.append(_circle(W * 0.39, H * 0.25, W * 0.005, dark))
            parts.append(_circle(W * 0.41, H * 0.25, W * 0.005, dark))
        # Figurehead
        parts.append(_polygon([
            (W * 0.94, H * 0.58),
            (W * 0.98, H * 0.54),
            (W * 0.96, H * 0.62),
        ], accent, dark, 0.5))
        # Armored variant adds plate armor on side
        if "armored" in name:
            for i in range(4):
                parts.append(_rect(W * (0.16 + i * 0.18), H * 0.60,
                                   W * 0.14, H * 0.10, metal, dark, 1.0))
                # Bolts
                parts.append(_circle(W * (0.18 + i * 0.18), H * 0.62, 1.5, dark))
                parts.append(_circle(W * (0.28 + i * 0.18), H * 0.68, 1.5, dark))
        # Flag mast (armored_boat)
        if "armored" in name:
            parts.append(_polygon([
                (W * 0.508, H * 0.16),
                (W * 0.60, H * 0.16),
                (W * 0.60, H * 0.24),
                (W * 0.508, H * 0.24),
            ], accent, dark, 0.5))
    elif "hex_carrier" in name or "hex_assault" in name:
        # Faceted hexagonal hull + no-wheel hover indicator + rotating dome
        # Hex hull (top-down hex shape rendered as silhouette)
        import math as _mhex
        hex_pts = []
        for i in range(6):
            ang = i * 1.047
            r_x = W * 0.36
            r_y = H * 0.22
            hex_pts.append((W * 0.50 + r_x * _mhex.cos(ang),
                           H * 0.50 + r_y * _mhex.sin(ang)))
        parts.append(_polygon(hex_pts, body, dark, max(2.0, W * 0.012)))
        # Inner hex
        inner_pts = []
        for i in range(6):
            ang = i * 1.047
            r_x = W * 0.28
            r_y = H * 0.16
            inner_pts.append((W * 0.50 + r_x * _mhex.cos(ang),
                             H * 0.50 + r_y * _mhex.sin(ang)))
        parts.append(_polygon(inner_pts, _lighten_hex(body, 0.10), dark, 1.0))
        # Rotating dome (center turret)
        parts.append(_circle(W * 0.50, H * 0.50, W * 0.10, dark, metal, 1.5))
        parts.append(_circle(W * 0.50, H * 0.50, W * 0.06, body, accent, 1.0))
        parts.append(_circle(W * 0.50, H * 0.50, W * 0.025, glow))
        # Hex_assault: 4 mounted weapon barrels
        if "assault" in name:
            for i in range(4):
                ang = i * 1.5708
                wx = W * 0.50 + W * 0.10 * _mhex.cos(ang)
                wy = H * 0.50 + H * 0.10 * _mhex.sin(ang)
                wx2 = W * 0.50 + W * 0.20 * _mhex.cos(ang)
                wy2 = H * 0.50 + H * 0.20 * _mhex.sin(ang)
                parts.append(_line(wx, wy, wx2, wy2, metal, 3.0))
        # Hover glow underneath (no wheels)
        parts.append(_ellipse(W * 0.50, H * 0.82, W * 0.30, H * 0.05, glow, accent, 0.5))
        parts.append(_ellipse(W * 0.50, H * 0.86, W * 0.36, H * 0.04, _lighten_hex(glow, 0.20)))
        # Hex panel seams (3 visible)
        for i in range(3):
            ang = i * 2.094
            x1 = W * 0.50
            y1 = H * 0.50
            r_x = W * 0.36
            r_y = H * 0.22
            x2 = W * 0.50 + r_x * _mhex.cos(ang)
            y2 = H * 0.50 + r_y * _mhex.sin(ang)
            parts.append(_line(x1, y1, x2, y2, dark, 0.6))
    else:
        # Default: generic APC silhouette
        hull_y = H * 0.40
        hull_h = H * 0.32
        parts.append(_rect(W * 0.08, hull_y, W * 0.84, hull_h, body, dark, 1.0))
        parts.append(_polygon([
            (W * 0.92, hull_y),
            (W * 0.98, hull_y + hull_h * 0.4),
            (W * 0.92, hull_y + hull_h),
        ], metal, dark, 0.5))
        parts.append(_rect(W * 0.62, hull_y + hull_h * 0.18, W * 0.18, hull_h * 0.32,
                           accent, dark, 0.5))
        for i in range(4):
            wx = W * (0.16 + 0.20 * i)
            parts.append(_circle(wx, H * 0.78, H * 0.07, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.78, H * 0.04, metal))
        parts.append(_rect(W * 0.32, hull_y - H * 0.08, W * 0.18, H * 0.08, body, dark, 1.0))

    # Variant overlays apply to all archetypes
    if variant == "boarding":
        # Ramp open from rear
        parts.append(_polygon([
            (W * 0.08, H * 0.66),
            (W * 0.08, H * 0.78),
            (W * 0.30, H * 0.66),
        ], metal, dark, 0.5))
        parts.append(_rect(W * 0.10, H * 0.62, W * 0.06, H * 0.04, accent, dark, 0.5))
    elif variant == "boarded":
        # Door closed + green indicator
        parts.append(_circle(W * 0.18, H * 0.50, W * 0.025, glow, "#5bd078", 1.0))
        # Door outline
        parts.append(_rect(W * 0.08, H * 0.44, W * 0.06, H * 0.16, "none", "#5bd078", 1.0))
    return "".join(parts)


def _compose_chassis(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    dark = p.dark()
    metal = p.metal()
    accent = p.accent()
    parts: List[str] = []
    canonical = spec.canonical_name
    cx = spec.width / 2

    # M12 polish-pass: each chassis archetype gets a detailed mech silhouette
    # with cockpit, weapon mount, plating seams, joint articulation, faction
    # accent decal, and weight-class scaling (light = compact; super_heavy =
    # bulky with extra plates).
    highlight = p.highlight()
    glow = p.glow()
    W = spec.width
    H = spec.height
    weight_class = (spec.extra or {}).get("weight_class", "medium")
    # Weight class drives size + plating density
    plate_density = {"light": 1, "medium": 2, "heavy": 3, "super_heavy": 4}.get(weight_class, 2)
    bulk_mul = {"light": 0.85, "medium": 1.0, "heavy": 1.10, "super_heavy": 1.20}.get(weight_class, 1.0)

    if "bipedal" in canonical:
        # Head / cockpit (visor with glowing eye)
        head_r = W * 0.09 * bulk_mul
        parts.append(_rect(cx - head_r, H * 0.14, head_r * 2, head_r * 1.4, body, dark, 1.0))  # blocky head
        parts.append(_rect(cx - head_r * 0.85, H * 0.16, head_r * 1.7, head_r * 0.5, dark))  # visor band
        parts.append(_rect(cx - head_r * 0.45, H * 0.18, head_r * 0.9, head_r * 0.25, glow))  # visor glow
        parts.append(_circle(cx + head_r * 0.6, H * 0.16 + head_r * 0.6, W * 0.012, accent))  # sensor pip
        # Antenna / sensor mast for medium+
        if plate_density >= 2:
            parts.append(_line(cx + head_r * 0.4, H * 0.14, cx + head_r * 0.4 + W * 0.04, H * 0.06, dark, 1.5))
            parts.append(_circle(cx + head_r * 0.4 + W * 0.04, H * 0.06, W * 0.012, glow))
        # Torso (broader for heavier classes)
        torso_w = W * 0.36 * bulk_mul
        torso_x = cx - torso_w / 2
        parts.append(_rect(torso_x, H * 0.30, torso_w, H * 0.30, body, dark, 1.2))
        # Plating seams on torso
        for i in range(plate_density):
            sy = H * (0.34 + i * 0.06)
            parts.append(_line(torso_x + W * 0.02, sy, torso_x + torso_w - W * 0.02, sy, dark, 0.7))
        # Chest core (faction-colored)
        core_w = torso_w * 0.30
        parts.append(_rect(cx - core_w / 2, H * 0.36, core_w, H * 0.12, accent, dark, 0.8))
        parts.append(_circle(cx, H * 0.42, W * 0.018, glow))  # power core glow
        # Shoulder pauldrons
        parts.append(_polygon([
            (torso_x - W * 0.04, H * 0.30),
            (torso_x + W * 0.02, H * 0.30),
            (torso_x + W * 0.06, H * 0.42),
            (torso_x - W * 0.02, H * 0.42),
        ], dark, metal, 0.5))
        parts.append(_polygon([
            (torso_x + torso_w - W * 0.02, H * 0.30),
            (torso_x + torso_w + W * 0.04, H * 0.30),
            (torso_x + torso_w + W * 0.02, H * 0.42),
            (torso_x + torso_w - W * 0.06, H * 0.42),
        ], dark, metal, 0.5))
        # Weapon mount (right arm) — heavier weight = bigger weapon
        weap_w = W * 0.08 * bulk_mul
        parts.append(_rect(torso_x + torso_w, H * 0.36, weap_w, H * 0.06, metal, dark, 0.5))
        parts.append(_rect(torso_x + torso_w + weap_w * 0.6, H * 0.34, W * 0.04, H * 0.10, dark))
        # Arm (left, more compact)
        parts.append(_rect(torso_x - W * 0.02, H * 0.42, W * 0.05, H * 0.18, body, dark, 0.8))
        parts.append(_circle(torso_x - W * 0.005, H * 0.43, W * 0.018, dark))  # shoulder joint
        # Hip plates
        parts.append(_rect(cx - torso_w * 0.45, H * 0.58, torso_w * 0.90, H * 0.06, dark, metal, 0.5))
        # Legs — hydraulic detail
        leg_w = W * 0.11 * bulk_mul
        leg_top_y = H * 0.62
        for sx, name in [(cx - W * 0.16 * bulk_mul, "L"), (cx + W * 0.05 * bulk_mul, "R")]:
            # Upper leg
            parts.append(_rect(sx, leg_top_y, leg_w, H * 0.14, body, dark, 0.8))
            # Knee joint (cylinder)
            parts.append(_circle(sx + leg_w / 2, leg_top_y + H * 0.14, leg_w * 0.45, dark, metal, 0.4))
            # Lower leg (slightly tapered)
            parts.append(_rect(sx + leg_w * 0.05, leg_top_y + H * 0.16, leg_w * 0.9, H * 0.12, body, dark, 0.8))
            # Foot / piston
            parts.append(_rect(sx - leg_w * 0.10, leg_top_y + H * 0.28, leg_w * 1.20, H * 0.04, dark, metal, 0.5))
            # Hydraulic line
            parts.append(_line(sx + leg_w * 0.5, leg_top_y + H * 0.02,
                               sx + leg_w * 0.5, leg_top_y + H * 0.12, accent, 1.2))
        # Faction decal: stripe on chest
        parts.append(_rect(torso_x + torso_w * 0.10, H * 0.50, torso_w * 0.80, H * 0.02, accent, dark, 0.3))
    elif "quadruped" in canonical:
        # Body chassis
        body_w = W * 0.78 * bulk_mul
        body_x = cx - body_w / 2
        parts.append(_rect(body_x, H * 0.32, body_w, H * 0.20 + H * 0.02 * plate_density, body, dark, 1.2))
        # Plating seams
        for i in range(plate_density):
            sx = body_x + body_w * (0.20 + 0.20 * i)
            parts.append(_line(sx, H * 0.32, sx, H * 0.52, dark, 0.7))
        # Front sensor head (turret-like)
        parts.append(_rect(body_x + body_w * 0.78, H * 0.26, body_w * 0.22, H * 0.10, body, dark, 1.0))
        parts.append(_rect(body_x + body_w * 0.82, H * 0.28, body_w * 0.16, H * 0.04, dark))  # visor band
        parts.append(_rect(body_x + body_w * 0.86, H * 0.29, body_w * 0.10, H * 0.02, glow))  # eye glow
        # Weapon mount on top
        parts.append(_rect(body_x + body_w * 0.30, H * 0.22, body_w * 0.30, H * 0.10, metal, dark, 0.5))
        parts.append(_rect(body_x + body_w * 0.55, H * 0.24, body_w * 0.10, H * 0.06, dark))
        # Power core
        parts.append(_circle(cx, H * 0.42, W * 0.02, glow))
        # 4 legs with knees + claws
        leg_xs = [body_x + body_w * 0.10, body_x + body_w * 0.32, body_x + body_w * 0.55, body_x + body_w * 0.78]
        leg_w = W * 0.06 * bulk_mul
        for lx in leg_xs:
            # Upper leg (angled back)
            parts.append(_polygon([
                (lx - leg_w * 0.2, H * 0.52),
                (lx + leg_w * 1.2, H * 0.52),
                (lx + leg_w * 0.9, H * 0.66),
                (lx + leg_w * 0.1, H * 0.66),
            ], body, dark, 0.8))
            # Knee
            parts.append(_circle(lx + leg_w * 0.5, H * 0.66, leg_w * 0.4, dark, metal, 0.4))
            # Lower leg
            parts.append(_polygon([
                (lx + leg_w * 0.2, H * 0.68),
                (lx + leg_w * 0.9, H * 0.68),
                (lx + leg_w * 0.7, H * 0.86),
                (lx + leg_w * 0.3, H * 0.86),
            ], body, dark, 0.8))
            # Claw / foot
            parts.append(_polygon([
                (lx + leg_w * 0.2, H * 0.86),
                (lx + leg_w * 0.8, H * 0.86),
                (lx + leg_w * 0.5, H * 0.94),
            ], dark, metal, 0.5))
            # Hydraulic on knee
            parts.append(_circle(lx + leg_w * 0.5, H * 0.66, leg_w * 0.15, accent))
        # Faction decal stripe along side
        parts.append(_rect(body_x + body_w * 0.10, H * 0.46, body_w * 0.50, H * 0.02, accent, dark, 0.3))
    elif "treaded" in canonical:
        # Main hull
        hull_w = W * 0.80 * bulk_mul
        hull_x = cx - hull_w / 2
        parts.append(_rect(hull_x, H * 0.34, hull_w, H * 0.20, body, dark, 1.2))
        # Hull plating seams
        for i in range(plate_density + 1):
            sx = hull_x + hull_w * (0.10 + 0.18 * i)
            parts.append(_line(sx, H * 0.34, sx, H * 0.54, dark, 0.7))
        # Turret on top
        turret_w = hull_w * 0.45
        turret_x = hull_x + (hull_w - turret_w) / 2
        parts.append(_rect(turret_x, H * 0.20, turret_w, H * 0.14, body, dark, 1.0))
        # Cupola
        parts.append(_circle(turret_x + turret_w * 0.30, H * 0.20, W * 0.018, dark, metal, 0.5))
        parts.append(_circle(turret_x + turret_w * 0.30, H * 0.20, W * 0.008, glow))
        # Main gun barrel
        barrel_len = hull_w * 0.45
        parts.append(_rect(turret_x + turret_w * 0.85, H * 0.25, barrel_len, H * 0.04, metal, dark, 0.5))
        parts.append(_rect(turret_x + turret_w * 0.85 + barrel_len, H * 0.23, W * 0.04, H * 0.08, dark))  # muzzle brake
        # Secondary weapon (mounted on turret) for heavy+
        if plate_density >= 3:
            parts.append(_rect(turret_x + turret_w * 0.10, H * 0.18, turret_w * 0.20, H * 0.04, dark))
            parts.append(_rect(turret_x + turret_w * 0.12, H * 0.16, turret_w * 0.05, H * 0.02, metal))
        # Tread skirt
        parts.append(_rect(hull_x - W * 0.02, H * 0.62, hull_w + W * 0.04, H * 0.16, dark, metal, 0.5))
        # Drive sprocket (left)
        parts.append(_circle(hull_x + W * 0.04, H * 0.78, H * 0.08, dark, metal, 0.5))
        parts.append(_circle(hull_x + W * 0.04, H * 0.78, H * 0.04, metal))
        for ang_i in range(6):
            import math as _m3
            ang = ang_i * 1.047
            sx = hull_x + W * 0.04 + _m3.cos(ang) * H * 0.05
            sy = H * 0.78 + _m3.sin(ang) * H * 0.05
            parts.append(_circle(sx, sy, W * 0.008, dark))
        # Idler sprocket (right)
        parts.append(_circle(hull_x + hull_w - W * 0.04, H * 0.78, H * 0.07, dark, metal, 0.5))
        parts.append(_circle(hull_x + hull_w - W * 0.04, H * 0.78, H * 0.035, metal))
        # Road wheels
        for i in range(5):
            wx = hull_x + W * 0.10 + (hull_w - W * 0.20) * i / 4
            parts.append(_circle(wx, H * 0.78, H * 0.045, metal, dark, 0.5))
        # Track teeth
        for i in range(plate_density * 4 + 8):
            tx = hull_x + (hull_w + W * 0.04) * (i / (plate_density * 4 + 8))
            parts.append(_rect(tx, H * 0.64, W * 0.01, W * 0.01, dark))
        # Faction decal stripe on hull side
        parts.append(_rect(hull_x + hull_w * 0.20, H * 0.48, hull_w * 0.50, H * 0.025, accent, dark, 0.3))
    elif "hovering" in canonical:
        # Main body (lens shape)
        parts.append(_polygon([
            (W * 0.12, H * 0.40),
            (W * 0.88, H * 0.40),
            (W * 0.96, H * 0.50),
            (W * 0.88, H * 0.60),
            (W * 0.12, H * 0.60),
            (W * 0.04, H * 0.50),
        ], body, dark, 1.2))
        # Plating seams (lens segments)
        for i in range(plate_density + 1):
            sx = W * (0.20 + 0.16 * i)
            parts.append(_line(sx, H * 0.42, sx, H * 0.58, dark, 0.5))
        # Cockpit canopy (top)
        parts.append(_polygon([
            (W * 0.40, H * 0.30),
            (W * 0.60, H * 0.30),
            (W * 0.55, H * 0.42),
            (W * 0.45, H * 0.42),
        ], dark, metal, 0.5))
        parts.append(_polygon([
            (W * 0.42, H * 0.32),
            (W * 0.58, H * 0.32),
            (W * 0.54, H * 0.40),
            (W * 0.46, H * 0.40),
        ], glow))
        # Twin wing-tip weapon pods
        parts.append(_rect(W * 0.06, H * 0.46, W * 0.10, H * 0.06, metal, dark, 0.5))
        parts.append(_rect(W * 0.84, H * 0.46, W * 0.10, H * 0.06, metal, dark, 0.5))
        parts.append(_rect(W * 0.86, H * 0.43, W * 0.08, H * 0.04, dark))
        parts.append(_rect(W * 0.04, H * 0.43, W * 0.08, H * 0.04, dark))
        # Hover thrust glow (under-body)
        parts.append(_polygon([
            (W * 0.20, H * 0.62),
            (W * 0.80, H * 0.62),
            (W * 0.70, H * 0.72),
            (W * 0.30, H * 0.72),
        ], accent, dark, 0.4))
        parts.append(_polygon([
            (W * 0.28, H * 0.66),
            (W * 0.72, H * 0.66),
            (W * 0.62, H * 0.78),
            (W * 0.38, H * 0.78),
        ], glow))
        # Antenna mast
        parts.append(_line(W * 0.50, H * 0.30, W * 0.50, H * 0.18, dark, 1.5))
        parts.append(_circle(W * 0.50, H * 0.18, W * 0.012, glow))
        # Faction decal on top
        parts.append(_rect(W * 0.30, H * 0.50, W * 0.40, H * 0.02, accent, dark, 0.3))
    elif "wheeled" in canonical:
        # Hull
        hull_w = W * 0.78 * bulk_mul
        hull_x = cx - hull_w / 2
        parts.append(_rect(hull_x, H * 0.36, hull_w, H * 0.24, body, dark, 1.2))
        # Plating seams
        for i in range(plate_density + 1):
            sx = hull_x + hull_w * (0.15 + 0.20 * i)
            parts.append(_line(sx, H * 0.36, sx, H * 0.60, dark, 0.7))
        # Cockpit / windshield
        parts.append(_polygon([
            (hull_x + hull_w * 0.60, H * 0.36),
            (hull_x + hull_w * 0.90, H * 0.36),
            (hull_x + hull_w * 0.88, H * 0.28),
            (hull_x + hull_w * 0.62, H * 0.28),
        ], dark, metal, 0.5))
        parts.append(_polygon([
            (hull_x + hull_w * 0.62, H * 0.30),
            (hull_x + hull_w * 0.88, H * 0.30),
            (hull_x + hull_w * 0.86, H * 0.34),
            (hull_x + hull_w * 0.64, H * 0.34),
        ], glow))
        # Turret on top
        parts.append(_rect(hull_x + hull_w * 0.20, H * 0.30, hull_w * 0.30, H * 0.10, body, dark, 0.8))
        # Main gun
        parts.append(_rect(hull_x + hull_w * 0.45, H * 0.34, hull_w * 0.35, H * 0.04, metal, dark, 0.5))
        parts.append(_rect(hull_x + hull_w * 0.78, H * 0.32, W * 0.03, H * 0.08, dark))  # muzzle
        # Sensor cluster
        parts.append(_circle(hull_x + hull_w * 0.30, H * 0.32, W * 0.018, dark, metal, 0.5))
        parts.append(_circle(hull_x + hull_w * 0.30, H * 0.32, W * 0.008, glow))
        # 4 wheels with hubcaps
        wheel_r = H * 0.08 * bulk_mul
        for i in range(4):
            wx = hull_x + hull_w * (0.10 + 0.27 * i)
            parts.append(_circle(wx, H * 0.78, wheel_r, dark, metal, 0.5))
            parts.append(_circle(wx, H * 0.78, wheel_r * 0.5, metal))
            parts.append(_circle(wx, H * 0.78, wheel_r * 0.18, dark))
            # Wheel spokes
            for ang_i in range(4):
                import math as _m4
                ang = ang_i * 1.5708
                spx = wx + _m4.cos(ang) * wheel_r * 0.35
                spy = H * 0.78 + _m4.sin(ang) * wheel_r * 0.35
                parts.append(_circle(spx, spy, W * 0.005, dark))
        # Fender / wheel arch
        parts.append(_rect(hull_x - W * 0.01, H * 0.58, hull_w + W * 0.02, H * 0.04, dark))
        # Faction decal stripe on hull
        parts.append(_rect(hull_x + hull_w * 0.10, H * 0.50, hull_w * 0.50, H * 0.025, accent, dark, 0.3))
    else:
        # Fallback (generic chassis)
        parts.append(_rect(W * 0.10, H * 0.30, W * 0.80, H * 0.50, body, dark, 1.0))
        parts.append(_rect(W * 0.40, H * 0.20, W * 0.20, H * 0.12, body, dark, 1.0))
        parts.append(_circle(W * 0.50, H * 0.42, W * 0.04, glow))

    weight_class = (spec.extra or {}).get("weight_class", "medium")
    pip_count = {"light": 1, "medium": 2, "heavy": 3, "super_heavy": 4}.get(weight_class, 2)
    for i in range(pip_count):
        parts.append(_circle(spec.width * 0.08 + i * spec.width * 0.04, spec.height * 0.10,
                             spec.width * 0.012, accent))
    return "".join(parts)


def _compose_base_module(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    dark = p.dark()
    accent = p.accent()
    metal = p.metal()
    glow = p.glow()
    parts: List[str] = []
    canonical = spec.canonical_name
    cx = spec.width / 2

    base_w = spec.width * 0.70
    base_h = spec.height * 0.62
    base_x = (spec.width - base_w) / 2
    base_y = spec.height * 0.20
    parts.append(_rect(base_x, base_y, base_w, base_h, body, dark, 1.0))

    if "turret" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.3, base_w * 0.20, metal, dark, 0.5))
        parts.append(_rect(cx, base_y + base_h * 0.25, base_w * 0.60, base_h * 0.08, metal, dark, 0.5))
    elif "wall" in canonical or "gate" in canonical:
        for i in range(3):
            parts.append(_line(base_x, base_y + base_h * (0.25 + i * 0.25),
                               base_x + base_w, base_y + base_h * (0.25 + i * 0.25), dark, 0.5))
    elif "solar" in canonical:
        # Solar panel array (must come before "generator" match since "base_generator_solar")
        parts.append(_polygon([
            (base_x + base_w * 0.10, base_y + base_h * 0.20),
            (base_x + base_w * 0.90, base_y + base_h * 0.20),
            (base_x + base_w * 0.95, base_y + base_h * 0.40),
            (base_x + base_w * 0.05, base_y + base_h * 0.40),
        ], "#2244AA", dark, 1.0))
        for col in range(8):
            for row in range(3):
                cell_x = base_x + base_w * (0.12 + col * 0.10)
                cell_y = base_y + base_h * (0.22 + row * 0.06)
                parts.append(_rect(cell_x, cell_y, base_w * 0.08, base_h * 0.04,
                                   "#3355CC", dark, 0.3))
        parts.append(_rect(base_x + base_w * 0.48, base_y + base_h * 0.40,
                           base_w * 0.04, base_h * 0.40, metal, dark, 0.5))
        parts.append(_rect(base_x + base_w * 0.20, base_y + base_h * 0.80,
                           base_w * 0.60, base_h * 0.08, dark))
    elif "geothermal" in canonical:
        # Geothermal vent (must come before "generator" match)
        parts.append(_rect(base_x + base_w * 0.40, base_y + base_h * 0.05,
                           base_w * 0.20, base_h * 0.75, metal, dark, 1.0))
        for i in range(3):
            cx_steam = base_x + base_w * (0.42 + i * 0.07)
            for j in range(2):
                parts.append(_circle(cx_steam, base_y + base_h * (0.02 - j * 0.08),
                                     base_w * 0.04, "#dddddd", "#aaaaaa", 0.3))
        parts.append(_circle(base_x + base_w * 0.50, base_y + base_h * 0.75, base_w * 0.10,
                             accent, glow, 0.8))
        parts.append(_circle(base_x + base_w * 0.50, base_y + base_h * 0.75, base_w * 0.06, glow))
    elif "fusion" in canonical:
        # Fusion: 3 concentric rings + central toroid glow
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.32, "none", accent, max(1.0, base_w * 0.012)))
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.24, "none", glow, max(1.0, base_w * 0.012)))
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.14, accent, glow, 1.0))
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.08, glow))
        # Magnetic bottle struts
        for ang_i in range(6):
            import math as _mfu
            ang = ang_i * 1.047
            x1 = cx + _mfu.cos(ang) * base_h * 0.14
            y1 = base_y + base_h * 0.5 + _mfu.sin(ang) * base_h * 0.14
            x2 = cx + _mfu.cos(ang) * base_h * 0.32
            y2 = base_y + base_h * 0.5 + _mfu.sin(ang) * base_h * 0.32
            parts.append(_line(x1, y1, x2, y2, dark, 0.5))
    elif "generator_diesel" in canonical or ("diesel" in canonical and "generator" in canonical):
        # Diesel generator: piston housing + exhaust pipes
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.30,
                           base_w * 0.60, base_h * 0.45, body, dark, 1.0))
        # Exhaust stack
        parts.append(_rect(base_x + base_w * 0.72, base_y + base_h * 0.10,
                           base_w * 0.06, base_h * 0.30, metal, dark, 0.5))
        # Cooling vents
        for i in range(4):
            parts.append(_rect(base_x + base_w * 0.15, base_y + base_h * (0.36 + i * 0.10),
                               base_w * 0.50, base_h * 0.05, dark))
        # Output gauge
        parts.append(_circle(base_x + base_w * 0.20, base_y + base_h * 0.85, base_w * 0.05, "#FFFFFF", dark, 0.5))
        parts.append(_line(base_x + base_w * 0.20, base_y + base_h * 0.85,
                           base_x + base_w * 0.22, base_y + base_h * 0.82, accent, 1.0))
    elif "generator" in canonical or "reactor" in canonical or "battery" in canonical or "capacitor" in canonical:
        # Generic reactor torus + glow core
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.22, accent, glow, 1.0))
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.12, glow))
        # Battery cells (4 stacked) for battery_array variant
        if "battery" in canonical or "array" in canonical:
            for i in range(4):
                bx = base_x + base_w * (0.10 + i * 0.20)
                parts.append(_rect(bx, base_y + base_h * 0.10, base_w * 0.16, base_h * 0.30, metal, dark, 0.5))
                parts.append(_rect(bx + base_w * 0.04, base_y + base_h * 0.05, base_w * 0.08, base_h * 0.06, accent))
    elif "antenna" in canonical or "radar" in canonical or "sensor" in canonical:
        parts.append(_line(cx, base_y + base_h, cx, base_y - base_h * 0.4, metal, 1.5))
        parts.append(_circle(cx, base_y - base_h * 0.4, base_w * 0.15, metal))
    elif "pump" in canonical or "valve" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.30, metal, dark, 1.0))
        parts.append(_line(cx, base_y + base_h * 0.2, cx, base_y + base_h * 0.8, dark, 1.0))
        parts.append(_line(cx - base_h * 0.3, base_y + base_h * 0.5,
                           cx + base_h * 0.3, base_y + base_h * 0.5, dark, 1.0))
    elif "pipe" in canonical:
        parts.append(_rect(base_x - base_w * 0.05, base_y + base_h * 0.35,
                           base_w * 1.10, base_h * 0.30, metal, dark, 1.0))
    elif "silo" in canonical or "tank" in canonical:
        parts.append(_polygon(
            [
                (base_x, base_y + base_h * 0.20),
                (base_x + base_w / 2, base_y),
                (base_x + base_w, base_y + base_h * 0.20),
            ],
            metal, dark, 0.5,
        ))
    elif "beacon" in canonical or "spotlight" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.20, base_w * 0.15, glow, accent, 1.0))
        parts.append(_polygon(
            [
                (cx - base_w * 0.20, base_y + base_h * 0.20),
                (cx + base_w * 0.20, base_y + base_h * 0.20),
                (cx, base_y - base_h * 0.20),
            ],
            glow, accent, 0.5,
        ))
    elif "pad" in canonical or "platform" in canonical or "dock" in canonical:
        parts.append(_rect(base_x, base_y + base_h * 0.85, base_w, base_h * 0.15, dark, accent, 0.5))
    elif "console" in canonical or "terminal" in canonical:
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.10,
                           base_w * 0.80, base_h * 0.40, glow, dark, 0.5))
    elif "med" in canonical:
        parts.append(_rect(cx - base_w * 0.04, base_y + base_h * 0.20, base_w * 0.08, base_h * 0.30,
                           "#ffffff", dark, 0.5))
        parts.append(_rect(cx - base_w * 0.16, base_y + base_h * 0.31, base_w * 0.32, base_h * 0.08,
                           "#ffffff", dark, 0.5))
    elif "objective" in canonical or "extraction" in canonical or "lift" in canonical or "emplacement" in canonical:
        parts.append(_polygon(
            [
                (cx, base_y - base_h * 0.10),
                (cx - base_w * 0.10, base_y + base_h * 0.10),
                (cx + base_w * 0.10, base_y + base_h * 0.10),
            ],
            accent, dark, 0.5,
        ))
        # Add platform stripe pattern
        parts.append(_rect(base_x + base_w * 0.05, base_y + base_h * 0.75,
                           base_w * 0.90, base_h * 0.05, accent, dark, 0.5))
        for i in range(4):
            parts.append(_rect(base_x + base_w * (0.10 + i * 0.20), base_y + base_h * 0.78,
                               base_w * 0.10, base_h * 0.02, dark))
    elif "ammo" in canonical or "depot" in canonical:
        # Stack of crates / ammo boxes
        for row in range(2):
            for col in range(3):
                cx_box = base_x + base_w * (0.15 + col * 0.25)
                cy_box = base_y + base_h * (0.20 + row * 0.30)
                parts.append(_rect(cx_box, cy_box, base_w * 0.20, base_h * 0.22, metal, dark, 0.8))
                # Ammo label stripe
                parts.append(_rect(cx_box + base_w * 0.02, cy_box + base_h * 0.04,
                                   base_w * 0.16, base_h * 0.04, accent))
                parts.append(_rect(cx_box + base_w * 0.02, cy_box + base_h * 0.12,
                                   base_w * 0.16, base_h * 0.06, dark))
    elif "assembler" in canonical or "fabricator" in canonical or "forge" in canonical:
        # Industrial machine with screen + lever + output chute
        # Screen
        parts.append(_rect(base_x + base_w * 0.05, base_y + base_h * 0.10,
                           base_w * 0.45, base_h * 0.35, glow, dark, 0.5))
        # Screen grid pattern
        for i in range(4):
            parts.append(_line(base_x + base_w * 0.05, base_y + base_h * (0.18 + i * 0.07),
                               base_x + base_w * 0.50, base_y + base_h * (0.18 + i * 0.07),
                               dark, 0.5))
        # Lever
        parts.append(_rect(base_x + base_w * 0.55, base_y + base_h * 0.10,
                           base_w * 0.06, base_h * 0.20, dark))
        parts.append(_circle(base_x + base_w * 0.58, base_y + base_h * 0.10, base_w * 0.04, accent))
        # Output chute
        parts.append(_rect(base_x + base_w * 0.65, base_y + base_h * 0.40,
                           base_w * 0.25, base_h * 0.10, accent, dark, 0.5))
        # Output crate
        parts.append(_rect(base_x + base_w * 0.70, base_y + base_h * 0.55,
                           base_w * 0.20, base_h * 0.20, metal, dark, 0.5))
    elif "conveyor" in canonical or "belt" in canonical:
        # Belt with rollers + items
        parts.append(_rect(base_x, base_y + base_h * 0.40,
                           base_w, base_h * 0.20, dark))
        # Rollers
        for i in range(6):
            parts.append(_circle(base_x + base_w * (0.10 + i * 0.16), base_y + base_h * 0.50,
                                 base_h * 0.05, metal, dark, 0.5))
        # Items being moved
        for i in range(3):
            parts.append(_rect(base_x + base_w * (0.15 + i * 0.25), base_y + base_h * 0.36,
                               base_w * 0.08, base_h * 0.06, accent, dark, 0.5))
    elif "crate" in canonical or "rack" in canonical or "storage" in canonical:
        # Stacked crates
        for row in range(3):
            for col in range(3):
                cx_box = base_x + base_w * (0.10 + col * 0.27)
                cy_box = base_y + base_h * (0.10 + row * 0.27)
                parts.append(_rect(cx_box, cy_box, base_w * 0.22, base_h * 0.22, metal, dark, 0.8))
                # Cross-strap detail
                parts.append(_line(cx_box, cy_box + base_h * 0.11,
                                   cx_box + base_w * 0.22, cy_box + base_h * 0.11, dark, 0.5))
                parts.append(_line(cx_box + base_w * 0.11, cy_box,
                                   cx_box + base_w * 0.11, cy_box + base_h * 0.22, dark, 0.5))
    elif "cryosleep" in canonical or "pod" in canonical:
        # Cryosleep pod: rounded chamber with glass viewport
        parts.append(_polygon([
            (base_x + base_w * 0.20, base_y + base_h * 0.10),
            (base_x + base_w * 0.80, base_y + base_h * 0.10),
            (base_x + base_w * 0.85, base_y + base_h * 0.40),
            (base_x + base_w * 0.85, base_y + base_h * 0.60),
            (base_x + base_w * 0.80, base_y + base_h * 0.90),
            (base_x + base_w * 0.20, base_y + base_h * 0.90),
            (base_x + base_w * 0.15, base_y + base_h * 0.60),
            (base_x + base_w * 0.15, base_y + base_h * 0.40),
        ], dark, metal, 1.0))
        # Frosted glass viewport
        parts.append(_rect(base_x + base_w * 0.25, base_y + base_h * 0.25,
                           base_w * 0.50, base_h * 0.45, "#aaccee", dark, 0.5))
        # Cooling vent stripes
        for i in range(3):
            parts.append(_rect(base_x + base_w * (0.05 + i * 0.10), base_y + base_h * 0.45,
                               base_w * 0.04, base_h * 0.10, dark))
        # Status light
        parts.append(_circle(base_x + base_w * 0.50, base_y + base_h * 0.80, base_w * 0.03, glow))
    elif "food" in canonical or "processor" in canonical:
        # Vat + arm + crate
        # Main vat
        parts.append(_circle(base_x + base_w * 0.35, base_y + base_h * 0.50, base_h * 0.30,
                             metal, dark, 1.0))
        parts.append(_circle(base_x + base_w * 0.35, base_y + base_h * 0.50, base_h * 0.24,
                             "#88AA77", dark, 0.5))
        # Stir arm
        parts.append(_line(base_x + base_w * 0.35, base_y + base_h * 0.25,
                           base_x + base_w * 0.35, base_y + base_h * 0.45, metal, 2.0))
        # Output chute
        parts.append(_rect(base_x + base_w * 0.65, base_y + base_h * 0.55,
                           base_w * 0.30, base_h * 0.08, metal, dark, 0.5))
        # Receiver crate
        parts.append(_rect(base_x + base_w * 0.75, base_y + base_h * 0.65,
                           base_w * 0.20, base_h * 0.20, "#AA8855", dark, 0.5))
    elif "fuel" in canonical:
        # Fuel tank cluster with pressure gauges
        for tank_x in [0.20, 0.50, 0.80]:
            parts.append(_rect(base_x + base_w * (tank_x - 0.08), base_y + base_h * 0.15,
                               base_w * 0.16, base_h * 0.65, metal, dark, 1.0))
            # Pressure gauge
            parts.append(_circle(base_x + base_w * tank_x, base_y + base_h * 0.25,
                                 base_w * 0.04, "#FFFFFF", dark, 0.5))
            parts.append(_line(base_x + base_w * tank_x, base_y + base_h * 0.25,
                               base_x + base_w * (tank_x + 0.02), base_y + base_h * 0.23,
                               accent, 1.0))
            # Warning stripe
            parts.append(_rect(base_x + base_w * (tank_x - 0.08), base_y + base_h * 0.65,
                               base_w * 0.16, base_h * 0.05, accent))
    elif "hab" in canonical or "bunker" in canonical:
        # Habitat with windows + door + roof
        # Roof
        parts.append(_polygon([
            (base_x, base_y + base_h * 0.10),
            (base_x + base_w / 2, base_y),
            (base_x + base_w, base_y + base_h * 0.10),
        ], dark))
        # Main building
        parts.append(_rect(base_x, base_y + base_h * 0.10, base_w, base_h * 0.65, body, dark, 1.0))
        # Door
        parts.append(_rect(base_x + base_w * 0.42, base_y + base_h * 0.45,
                           base_w * 0.16, base_h * 0.30, dark))
        parts.append(_circle(base_x + base_w * 0.55, base_y + base_h * 0.60, base_w * 0.01, accent))
        # Windows
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.20,
                           base_w * 0.20, base_h * 0.15, glow, dark, 0.5))
        parts.append(_rect(base_x + base_w * 0.70, base_y + base_h * 0.20,
                           base_w * 0.20, base_h * 0.15, glow, dark, 0.5))
        # Window grids
        for win_x in [0.10, 0.70]:
            parts.append(_line(base_x + base_w * (win_x + 0.10), base_y + base_h * 0.20,
                               base_x + base_w * (win_x + 0.10), base_y + base_h * 0.35,
                               dark, 0.5))
            parts.append(_line(base_x + base_w * win_x, base_y + base_h * 0.275,
                               base_x + base_w * (win_x + 0.20), base_y + base_h * 0.275,
                               dark, 0.5))
    elif "jammer" in canonical or "dish" in canonical:
        # Satellite dish
        parts.append(_polygon([
            (base_x + base_w * 0.10, base_y + base_h * 0.40),
            (base_x + base_w * 0.90, base_y + base_h * 0.40),
            (base_x + base_w * 0.70, base_y + base_h * 0.05),
            (base_x + base_w * 0.30, base_y + base_h * 0.05),
        ], metal, dark, 1.0))
        # Dish ribs
        for i in range(4):
            ang = 0.39 + i * 0.78
            import math as _mbd
            x = (base_x + base_w * 0.50) + _mbd.cos(ang) * base_w * 0.30
            y = (base_y + base_h * 0.22) + _mbd.sin(ang) * base_h * 0.18
            parts.append(_line(base_x + base_w * 0.50, base_y + base_h * 0.22, x, y, dark, 0.7))
        # Receiver in center
        parts.append(_circle(base_x + base_w * 0.50, base_y + base_h * 0.22, base_w * 0.04, accent, dark, 0.5))
        # Support pole
        parts.append(_rect(base_x + base_w * 0.46, base_y + base_h * 0.40,
                           base_w * 0.08, base_h * 0.45, metal, dark, 0.5))
        # Base
        parts.append(_rect(base_x + base_w * 0.30, base_y + base_h * 0.80,
                           base_w * 0.40, base_h * 0.10, dark))
    elif "intercept" in canonical or "array" in canonical:
        # Array of small dishes/sensors
        for arr_x in [0.20, 0.40, 0.60, 0.80]:
            parts.append(_circle(base_x + base_w * arr_x, base_y + base_h * 0.30,
                                 base_w * 0.07, metal, dark, 0.5))
            parts.append(_line(base_x + base_w * arr_x, base_y + base_h * 0.36,
                               base_x + base_w * arr_x, base_y + base_h * 0.70,
                               metal, 1.5))
        # Base mounting bar
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.70,
                           base_w * 0.80, base_h * 0.06, dark))
    elif "solar" in canonical:
        # Solar panel array
        parts.append(_polygon([
            (base_x + base_w * 0.10, base_y + base_h * 0.20),
            (base_x + base_w * 0.90, base_y + base_h * 0.20),
            (base_x + base_w * 0.95, base_y + base_h * 0.40),
            (base_x + base_w * 0.05, base_y + base_h * 0.40),
        ], "#2244AA", dark, 1.0))
        # Solar cells (grid pattern on panel)
        for col in range(8):
            for row in range(3):
                cell_x = base_x + base_w * (0.12 + col * 0.10)
                cell_y = base_y + base_h * (0.22 + row * 0.06)
                parts.append(_rect(cell_x, cell_y, base_w * 0.08, base_h * 0.04,
                                   "#3355CC", dark, 0.3))
        # Mounting pole
        parts.append(_rect(base_x + base_w * 0.48, base_y + base_h * 0.40,
                           base_w * 0.04, base_h * 0.40, metal, dark, 0.5))
        # Base plate
        parts.append(_rect(base_x + base_w * 0.20, base_y + base_h * 0.80,
                           base_w * 0.60, base_h * 0.08, dark))
    elif "geothermal" in canonical:
        # Steam vent with pipe + glow
        parts.append(_rect(base_x + base_w * 0.40, base_y + base_h * 0.05,
                           base_w * 0.20, base_h * 0.75, metal, dark, 1.0))
        # Steam coming out top
        for i in range(3):
            cx_steam = base_x + base_w * (0.42 + i * 0.07)
            for j in range(2):
                parts.append(_circle(cx_steam, base_y + base_h * (0.02 - j * 0.08),
                                     base_w * 0.04, "#dddddd", "#aaaaaa", 0.3))
        # Heat glow at base
        parts.append(_circle(base_x + base_w * 0.50, base_y + base_h * 0.75, base_w * 0.10,
                             accent, glow, 0.8))
        parts.append(_circle(base_x + base_w * 0.50, base_y + base_h * 0.75, base_w * 0.06, glow))
    else:
        # Industrial / habitat enhanced fallback
        # Main body with multiple plate sections
        parts.append(_rect(base_x + base_w * 0.05, base_y + base_h * 0.10,
                           base_w * 0.90, base_h * 0.55, metal, dark, 0.8))
        # Plate seams
        parts.append(_line(base_x + base_w * 0.35, base_y + base_h * 0.10,
                           base_x + base_w * 0.35, base_y + base_h * 0.65, dark, 0.7))
        parts.append(_line(base_x + base_w * 0.65, base_y + base_h * 0.10,
                           base_x + base_w * 0.65, base_y + base_h * 0.65, dark, 0.7))
        # Vent grilles (3)
        for i in range(3):
            vx = base_x + base_w * (0.10 + i * 0.30)
            for j in range(3):
                parts.append(_rect(vx + base_w * 0.04, base_y + base_h * (0.20 + j * 0.05),
                                   base_w * 0.12, base_h * 0.02, dark))
        # Control panel
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.45,
                           base_w * 0.25, base_h * 0.18, glow, dark, 0.5))
        # Indicator lights
        for i in range(3):
            parts.append(_circle(base_x + base_w * (0.42 + i * 0.05), base_y + base_h * 0.50,
                                 base_w * 0.012, accent if i % 2 == 0 else glow))
        # Bottom plate
        parts.append(_rect(base_x, base_y + base_h * 0.85, base_w, base_h * 0.05, dark))

    # Module state pip in top-right
    state = (spec.extra or {}).get("module_state", "nominal")
    state_color = {
        "nominal": "#5bd078",
        "degraded": "#dab438",
        "warning": "#e87826",
        "failed": "#c93030",
    }.get(state, body)
    parts.append(_circle(base_x + base_w - spec.width * 0.04, base_y + spec.height * 0.04,
                         spec.width * 0.025, state_color, dark, 0.5))
    return "".join(parts)


def _compose_ui_icon(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    metal = p.metal()
    glow = p.glow()
    name = spec.canonical_name
    cx, cy = spec.width / 2, spec.height / 2

    parts: List[str] = []
    # Frame
    parts.append(_rect(spec.width * 0.06, spec.height * 0.06, spec.width * 0.88, spec.height * 0.88,
                       dark, body, max(1.0, spec.width * 0.02)))
    parts.append(_rect(spec.width * 0.10, spec.height * 0.10, spec.width * 0.80, spec.height * 0.80,
                       body, accent, max(1.0, spec.width * 0.015)))

    # Inner glyph: bias by name
    if "slot_primary" in name or "weapon_rifle" in name:
        parts.append(_rect(spec.width * 0.18, cy - spec.height * 0.05, spec.width * 0.64, spec.height * 0.10,
                           metal, dark, 0.5))
        parts.append(_rect(spec.width * 0.20, cy - spec.height * 0.08, spec.width * 0.20, spec.height * 0.06,
                           accent))
        parts.append(_rect(spec.width * 0.42, cy + spec.height * 0.05, spec.width * 0.08, spec.height * 0.10,
                           metal))
    elif "slot_secondary" in name or "weapon_pistol" in name:
        parts.append(_rect(spec.width * 0.32, cy - spec.height * 0.08, spec.width * 0.40, spec.height * 0.10,
                           metal, dark, 0.5))
        parts.append(_rect(spec.width * 0.48, cy + spec.height * 0.02, spec.width * 0.08, spec.height * 0.12,
                           dark))
    elif "weapon_smg" in name:
        parts.append(_rect(spec.width * 0.22, cy - spec.height * 0.04, spec.width * 0.50, spec.height * 0.08,
                           metal, dark, 0.5))
        parts.append(_rect(spec.width * 0.40, cy + spec.height * 0.04, spec.width * 0.06, spec.height * 0.10,
                           accent))
    elif "weapon_shotgun" in name:
        parts.append(_rect(spec.width * 0.18, cy - spec.height * 0.06, spec.width * 0.66, spec.height * 0.12,
                           metal, dark, 0.5))
    elif "weapon_sniper" in name:
        parts.append(_rect(spec.width * 0.12, cy - spec.height * 0.03, spec.width * 0.76, spec.height * 0.06,
                           metal, dark, 0.5))
        parts.append(_circle(spec.width * 0.74, cy, spec.height * 0.08, accent, dark, 0.5))
    elif "weapon_gl" in name or "weapon_grenade" in name:
        parts.append(_circle(cx, cy, spec.height * 0.18, metal, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.24, spec.width * 0.08, spec.height * 0.08,
                           dark))
    elif "weapon_melee" in name or "slot_melee" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.04, cy - spec.height * 0.20),
                (cx + spec.width * 0.04, cy - spec.height * 0.20),
                (cx + spec.width * 0.02, cy + spec.height * 0.20),
                (cx - spec.width * 0.02, cy + spec.height * 0.20),
            ],
            metal, dark, 0.5,
        ))
    elif "weapon_tool" in name or "slot_tool" in name:
        parts.append(_circle(cx, cy, spec.height * 0.16, metal, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.02, cy, spec.width * 0.04, spec.height * 0.18, dark))
    elif "weapon_heavy" in name:
        parts.append(_rect(spec.width * 0.14, cy - spec.height * 0.08, spec.width * 0.72, spec.height * 0.16,
                           metal, dark, 0.5))
    elif "slot_helmet" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.20, cy + spec.height * 0.08),
                (cx, cy - spec.height * 0.22),
                (cx + spec.width * 0.20, cy + spec.height * 0.08),
            ],
            metal, dark, 0.5,
        ))
    elif "slot_torso" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.18, spec.width * 0.36, spec.height * 0.36,
                           metal, dark, 0.5))
    elif "slot_legs" in name:
        parts.append(_rect(cx - spec.width * 0.16, cy - spec.height * 0.20, spec.width * 0.10, spec.height * 0.40,
                           metal, dark, 0.5))
        parts.append(_rect(cx + spec.width * 0.06, cy - spec.height * 0.20, spec.width * 0.10, spec.height * 0.40,
                           metal, dark, 0.5))
    elif "slot_backpack" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.16, spec.width * 0.36, spec.height * 0.32,
                           accent, dark, 0.5))
    elif "status_health" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.18, cy - spec.height * 0.05),
                (cx, cy - spec.height * 0.20),
                (cx + spec.width * 0.18, cy - spec.height * 0.05),
                (cx, cy + spec.height * 0.20),
            ],
            "#c93030", dark, 0.5,
        ))
    elif "status_stamina" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.12, cy + spec.height * 0.18),
                (cx, cy - spec.height * 0.18),
                (cx + spec.width * 0.06, cy),
                (cx + spec.width * 0.14, cy + spec.height * 0.18),
            ],
            "#dab438", dark, 0.5,
        ))
    elif "status_armor" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.18, cy - spec.height * 0.10),
                (cx, cy - spec.height * 0.20),
                (cx + spec.width * 0.18, cy - spec.height * 0.10),
                (cx + spec.width * 0.10, cy + spec.height * 0.18),
                (cx - spec.width * 0.10, cy + spec.height * 0.18),
            ],
            metal, dark, 0.5,
        ))
    elif "status_oxygen" in name:
        parts.append(_polygon(
            [
                (cx, cy - spec.height * 0.20),
                (cx - spec.width * 0.14, cy + spec.height * 0.12),
                (cx + spec.width * 0.14, cy + spec.height * 0.12),
            ],
            "#8ac6ff", dark, 0.5,
        ))
    elif "status_temperature" in name:
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.20, spec.width * 0.08, spec.height * 0.30,
                           metal, dark, 0.5))
        parts.append(_circle(cx, cy + spec.height * 0.10, spec.height * 0.08, accent))
    elif "status_radiation" in name:
        for i in range(3):
            ang = (i * 120) - 90
            import math
            x = cx + spec.width * 0.18 * math.cos(math.radians(ang))
            y = cy + spec.height * 0.18 * math.sin(math.radians(ang))
            parts.append(_circle(x, y, spec.width * 0.06, accent))
        parts.append(_circle(cx, cy, spec.width * 0.05, dark))
    elif "status_bleed" in name:
        parts.append(_polygon(
            [
                (cx, cy - spec.height * 0.18),
                (cx - spec.width * 0.10, cy + spec.height * 0.12),
                (cx + spec.width * 0.10, cy + spec.height * 0.12),
            ],
            "#c93030", dark, 0.5,
        ))
    elif "status_stun" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.05, cy - spec.height * 0.18),
                (cx + spec.width * 0.10, cy - spec.height * 0.04),
                (cx, cy + spec.height * 0.00),
                (cx + spec.width * 0.10, cy + spec.height * 0.18),
                (cx - spec.width * 0.10, cy + spec.height * 0.02),
                (cx, cy - spec.height * 0.02),
            ],
            glow, dark, 0.5,
        ))
    elif "action_move" in name:
        # Direction arrows (4-way movement)
        parts.append(_polygon([
            (cx, cy - spec.height * 0.20),
            (cx - spec.width * 0.06, cy - spec.height * 0.08),
            (cx + spec.width * 0.06, cy - spec.height * 0.08),
        ], accent, dark, 0.5))
        parts.append(_polygon([
            (cx, cy + spec.height * 0.20),
            (cx - spec.width * 0.06, cy + spec.height * 0.08),
            (cx + spec.width * 0.06, cy + spec.height * 0.08),
        ], accent, dark, 0.5))
        parts.append(_polygon([
            (cx - spec.width * 0.20, cy),
            (cx - spec.width * 0.08, cy - spec.height * 0.06),
            (cx - spec.width * 0.08, cy + spec.height * 0.06),
        ], accent, dark, 0.5))
        parts.append(_polygon([
            (cx + spec.width * 0.20, cy),
            (cx + spec.width * 0.08, cy - spec.height * 0.06),
            (cx + spec.width * 0.08, cy + spec.height * 0.06),
        ], accent, dark, 0.5))
        parts.append(_circle(cx, cy, spec.width * 0.04, highlight))
    elif "action_aim" in name:
        # Crosshair
        parts.append(_circle(cx, cy, spec.width * 0.16, "none", accent, max(1.5, spec.width * 0.012)))
        parts.append(_circle(cx, cy, spec.width * 0.04, "none", accent, max(1.0, spec.width * 0.008)))
        parts.append(_line(cx - spec.width * 0.22, cy, cx - spec.width * 0.10, cy, accent, 1.5))
        parts.append(_line(cx + spec.width * 0.22, cy, cx + spec.width * 0.10, cy, accent, 1.5))
        parts.append(_line(cx, cy - spec.height * 0.22, cx, cy - spec.height * 0.10, accent, 1.5))
        parts.append(_line(cx, cy + spec.height * 0.22, cx, cy + spec.height * 0.10, accent, 1.5))
        parts.append(_circle(cx, cy, spec.width * 0.015, "#c93030"))
    elif "action_fire" in name:
        # Muzzle flash + bullet
        parts.append(_polygon([
            (cx - spec.width * 0.20, cy),
            (cx - spec.width * 0.08, cy - spec.height * 0.10),
            (cx + spec.width * 0.10, cy - spec.height * 0.06),
            (cx + spec.width * 0.18, cy),
            (cx + spec.width * 0.10, cy + spec.height * 0.06),
            (cx - spec.width * 0.08, cy + spec.height * 0.10),
        ], "#FFAA22", "#FFFFAA", 0.5))
        parts.append(_circle(cx, cy, spec.width * 0.06, "#FFFFFF"))
        # Spent shell trail
        for i in range(3):
            parts.append(_rect(cx + spec.width * (0.20 + i * 0.04), cy - spec.height * 0.01,
                               spec.width * 0.02, spec.height * 0.04, "#FFAA22", dark, 0.3))
    elif "action_reload" in name:
        # Magazine + circular reload arrow
        parts.append(_rect(cx - spec.width * 0.06, cy - spec.height * 0.16, spec.width * 0.12, spec.height * 0.20,
                           metal, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.14, spec.width * 0.08, spec.height * 0.06,
                           accent))
        # Circular reload arrow around magazine
        import math as _m_reload
        for i in range(6):
            ang = i * 1.047
            x1 = cx + spec.width * 0.20 * _m_reload.cos(ang)
            y1 = cy + spec.height * 0.20 * _m_reload.sin(ang)
            x2 = cx + spec.width * 0.20 * _m_reload.cos(ang + 1.0)
            y2 = cy + spec.height * 0.20 * _m_reload.sin(ang + 1.0)
            parts.append(_line(x1, y1, x2, y2, accent, 2.0))
        parts.append(_polygon([
            (cx + spec.width * 0.20, cy + spec.height * 0.05),
            (cx + spec.width * 0.25, cy + spec.height * 0.08),
            (cx + spec.width * 0.16, cy + spec.height * 0.16),
        ], accent, dark, 0.5))
    elif "action_use" in name or "action_pickup" in name:
        # Hand-grip / item-pickup
        parts.append(_polygon([
            (cx - spec.width * 0.12, cy + spec.height * 0.12),
            (cx, cy - spec.height * 0.18),
            (cx + spec.width * 0.12, cy + spec.height * 0.12),
        ], accent, dark, 0.5))
        # Hand fingers
        for i in range(3):
            parts.append(_rect(cx - spec.width * 0.10 + i * spec.width * 0.07,
                               cy + spec.height * 0.10,
                               spec.width * 0.04, spec.height * 0.06,
                               accent, dark, 0.3))
    elif "action_swap" in name:
        # Two arrows (swap weapon slots)
        parts.append(_polygon([
            (cx - spec.width * 0.20, cy - spec.height * 0.06),
            (cx, cy - spec.height * 0.06),
            (cx - spec.width * 0.04, cy - spec.height * 0.14),
            (cx + spec.width * 0.16, cy),
            (cx - spec.width * 0.04, cy + spec.height * 0.14),
            (cx, cy + spec.height * 0.06),
            (cx - spec.width * 0.20, cy + spec.height * 0.06),
        ], accent, dark, 0.5))
        parts.append(_polygon([
            (cx + spec.width * 0.20, cy - spec.height * 0.16),
            (cx + spec.width * 0.20, cy - spec.height * 0.04),
            (cx + spec.width * 0.10, cy - spec.height * 0.10),
            (cx + spec.width * 0.24, cy - spec.height * 0.20),
        ], accent, dark, 0.5))
    elif "action_jet" in name:
        # Jetpack burst
        parts.append(_rect(cx - spec.width * 0.10, cy - spec.height * 0.18, spec.width * 0.20, spec.height * 0.20,
                           metal, dark, 0.5))
        parts.append(_polygon([
            (cx - spec.width * 0.12, cy + spec.height * 0.02),
            (cx + spec.width * 0.12, cy + spec.height * 0.02),
            (cx + spec.width * 0.08, cy + spec.height * 0.16),
            (cx, cy + spec.height * 0.22),
            (cx - spec.width * 0.08, cy + spec.height * 0.16),
        ], "#FFAA22", "#FFFFAA", 0.5))
        parts.append(_circle(cx, cy + spec.height * 0.10, spec.width * 0.03, "#FFFFFF"))
    elif "action_heal" in name:
        # Medkit / plus sign
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.12, spec.width * 0.36, spec.height * 0.24,
                           "#FFFFFF", dark, 1.0))
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.10, spec.width * 0.08, spec.height * 0.20,
                           "#c93030"))
        parts.append(_rect(cx - spec.width * 0.14, cy - spec.height * 0.04, spec.width * 0.28, spec.height * 0.08,
                           "#c93030"))
    elif "action_order" in name:
        # Megaphone + soundwaves
        parts.append(_polygon([
            (cx - spec.width * 0.16, cy - spec.height * 0.08),
            (cx, cy - spec.height * 0.16),
            (cx + spec.width * 0.06, cy - spec.height * 0.14),
            (cx + spec.width * 0.06, cy + spec.height * 0.14),
            (cx, cy + spec.height * 0.16),
            (cx - spec.width * 0.16, cy + spec.height * 0.08),
        ], metal, dark, 1.0))
        # Sound waves
        for i in range(3):
            r = spec.width * (0.16 + i * 0.04)
            parts.append(_circle(cx + spec.width * 0.06, cy, r, "none", accent, 1.5))
    elif "action_tag" in name:
        # Tag/label
        parts.append(_polygon([
            (cx - spec.width * 0.20, cy - spec.height * 0.10),
            (cx + spec.width * 0.10, cy - spec.height * 0.10),
            (cx + spec.width * 0.18, cy),
            (cx + spec.width * 0.10, cy + spec.height * 0.10),
            (cx - spec.width * 0.20, cy + spec.height * 0.10),
        ], accent, dark, 0.5))
        parts.append(_circle(cx + spec.width * 0.06, cy, spec.width * 0.03, "#FFFFFF"))
    elif "action_mark" in name:
        # Diamond + center dot (highlight mark)
        parts.append(_polygon([
            (cx, cy - spec.height * 0.20),
            (cx + spec.width * 0.16, cy),
            (cx, cy + spec.height * 0.20),
            (cx - spec.width * 0.16, cy),
        ], accent, dark, 0.5))
        parts.append(_circle(cx, cy, spec.width * 0.06, highlight, dark, 0.5))
    elif "action_pause" in name:
        # Two vertical bars
        parts.append(_rect(cx - spec.width * 0.08, cy - spec.height * 0.16, spec.width * 0.06, spec.height * 0.32,
                           accent, dark, 0.5))
        parts.append(_rect(cx + spec.width * 0.02, cy - spec.height * 0.16, spec.width * 0.06, spec.height * 0.32,
                           accent, dark, 0.5))
    elif "action_open" in name or "action_close" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.20,
                           spec.width * 0.36, spec.height * 0.40, metal, dark, 0.5))
        parts.append(_line(cx, cy - spec.height * 0.20, cx, cy + spec.height * 0.20, dark, 1.0))
    elif "action_repair" in name:
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.20,
                           spec.width * 0.08, spec.height * 0.40, metal, dark, 0.5))
        parts.append(_circle(cx, cy, spec.width * 0.08, accent))
    elif "action_hack" in name:
        parts.append(_rect(cx - spec.width * 0.20, cy - spec.height * 0.16,
                           spec.width * 0.40, spec.height * 0.32, dark, accent, 0.5))
        parts.append(_rect(cx - spec.width * 0.06, cy + spec.height * 0.05,
                           spec.width * 0.04, spec.height * 0.10, accent))
    elif "action_revive" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.04, cy - spec.height * 0.18),
                (cx + spec.width * 0.04, cy - spec.height * 0.18),
                (cx + spec.width * 0.04, cy - spec.height * 0.04),
                (cx + spec.width * 0.18, cy - spec.height * 0.04),
                (cx + spec.width * 0.18, cy + spec.height * 0.04),
                (cx + spec.width * 0.04, cy + spec.height * 0.04),
                (cx + spec.width * 0.04, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy + spec.height * 0.04),
                (cx - spec.width * 0.18, cy + spec.height * 0.04),
                (cx - spec.width * 0.18, cy - spec.height * 0.04),
                (cx - spec.width * 0.04, cy - spec.height * 0.04),
            ],
            "#5bd078", dark, 0.5,
        ))
    elif "action_board" in name:
        parts.append(_rect(spec.width * 0.40, cy - spec.height * 0.18,
                           spec.width * 0.40, spec.height * 0.36, metal, dark, 0.5))
        parts.append(_polygon(
            [
                (spec.width * 0.20, cy),
                (spec.width * 0.36, cy - spec.height * 0.08),
                (spec.width * 0.36, cy + spec.height * 0.08),
            ],
            accent, dark, 0.5,
        ))
    elif "action_drop" in name:
        parts.append(_rect(cx - spec.width * 0.10, cy - spec.height * 0.20,
                           spec.width * 0.20, spec.height * 0.20, metal, dark, 0.5))
        parts.append(_polygon(
            [
                (cx, cy + spec.height * 0.20),
                (cx - spec.width * 0.10, cy + spec.height * 0.04),
                (cx + spec.width * 0.10, cy + spec.height * 0.04),
            ],
            accent, dark, 0.5,
        ))
    elif "action_signal" in name:
        parts.append(_line(cx, cy + spec.height * 0.18, cx, cy - spec.height * 0.18, metal, 1.5))
        for i in range(3):
            r = spec.width * (0.06 + 0.06 * i)
            parts.append(_circle(cx, cy - spec.height * 0.18, r, "none", accent, 1.0))
    elif "acca_" in name:
        # Generic ACC-A glyph
        parts.append(_circle(cx, cy, spec.width * 0.16, accent, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.02, cy - spec.height * 0.08, spec.width * 0.04, spec.height * 0.16,
                           highlight))
    elif "material_" in name:
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.10,
                           spec.width * 0.36, spec.height * 0.20, metal, dark, 0.5))
    elif "hud_compass" in name:
        parts.append(_circle(cx, cy, spec.width * 0.22, dark, metal, 1.0))
        parts.append(_polygon(
            [
                (cx, cy - spec.height * 0.18),
                (cx - spec.width * 0.04, cy),
                (cx + spec.width * 0.04, cy),
            ],
            accent, dark, 0.5,
        ))
        parts.append(_polygon(
            [
                (cx, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy),
                (cx + spec.width * 0.04, cy),
            ],
            highlight, dark, 0.5,
        ))
    elif "hud_objective_complete" in name:
        parts.append(_polyline(
            [
                (cx - spec.width * 0.18, cy),
                (cx - spec.width * 0.04, cy + spec.height * 0.14),
                (cx + spec.width * 0.18, cy - spec.height * 0.14),
            ],
            "#5bd078", stroke_w=max(2.0, spec.width * 0.05),
        ))
    elif "hud_objective" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.18, cy + spec.height * 0.04),
                (cx, cy - spec.height * 0.20),
                (cx + spec.width * 0.18, cy + spec.height * 0.04),
                (cx + spec.width * 0.10, cy + spec.height * 0.20),
                (cx - spec.width * 0.10, cy + spec.height * 0.20),
            ],
            accent, dark, 0.5,
        ))
    elif "hud_waypoint" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.04, cy + spec.height * 0.18),
                (cx - spec.width * 0.04, cy - spec.height * 0.18),
                (cx + spec.width * 0.18, cy - spec.height * 0.10),
                (cx - spec.width * 0.04, cy - spec.height * 0.04),
            ],
            accent, dark, 0.5,
        ))
    elif "hud_extraction" in name:
        parts.append(_rect(cx - spec.width * 0.16, cy - spec.height * 0.04, spec.width * 0.32, spec.height * 0.08,
                           metal, dark, 0.5))
        parts.append(_rect(cx + spec.width * 0.10, cy - spec.height * 0.10, spec.width * 0.10, spec.height * 0.20,
                           dark))
    elif "hud_alert" in name:
        bars = {"low": 1, "med": 2, "high": 3}
        n = 1
        for k, v in bars.items():
            if k in name:
                n = v
                break
        for i in range(3):
            color = accent if i < n else metal
            parts.append(_rect(cx - spec.width * 0.18 + i * spec.width * 0.13,
                               cy + spec.height * 0.04 - i * spec.height * 0.04,
                               spec.width * 0.10, spec.height * 0.14, color, dark, 0.5))
    else:
        # Generic
        parts.append(_circle(cx, cy, spec.width * 0.16, accent, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.04, cy - spec.height * 0.04, spec.width * 0.08, spec.height * 0.08,
                           highlight))
    return "".join(parts)


def _compose_material(spec: AssetSpec, rng: random.Random) -> str:
    # spec.extra carries 'base_color' (the integrity-banded hex). Pipeline
    # passes the material id via canonical_name.
    base = (spec.extra or {}).get("base_color", spec.palette.primary())
    band = (spec.extra or {}).get("integrity_band", "pristine")
    mode = (spec.extra or {}).get("overlay_mode")
    name = spec.canonical_name
    parts: List[str] = []

    parts.append(_rect(0, 0, spec.width, spec.height, base))
    # Add per-material texture flavor
    if "dirt" in name or "sand" in name or "mud" in name:
        for _ in range(40):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            r = rng.uniform(0.5, 1.4)
            parts.append(_circle(x, y, r, _darken_hex(base, 0.2)))
    elif "concrete" in name or "loose_fill" in name:
        for _ in range(18):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            r = rng.uniform(0.6, 2.0)
            parts.append(_rect(x, y, r * 2, r * 2, _darken_hex(base, 0.15)))
    elif "metal" in name or "alloy" in name:
        for i in range(0, spec.height, max(1, spec.height // 8)):
            parts.append(_line(0, i, spec.width, i, _lighten_hex(base, 0.05), 0.3))
    elif "hazard" in name:
        for i in range(0, spec.width + spec.height, max(1, spec.width // 6)):
            parts.append(_polyline(
                [(i, 0), (i - spec.height, spec.height)],
                _darken_hex(base, 0.2), max(1.0, spec.width / 24),
            ))
    elif "lava" in name:
        for _ in range(8):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            parts.append(_polyline(
                [(x, y), (x + rng.uniform(-spec.width / 4, spec.width / 4),
                          y + rng.uniform(-spec.height / 4, spec.height / 4))],
                _lighten_hex(base, 0.30), 1.5,
            ))
    elif "ice" in name or "glass" in name:
        for _ in range(8):
            x1 = rng.uniform(0, spec.width)
            x2 = x1 + rng.uniform(-spec.width / 3, spec.width / 3)
            y1 = rng.uniform(0, spec.height)
            y2 = y1 + rng.uniform(-spec.height / 3, spec.height / 3)
            parts.append(_line(x1, y1, x2, y2, _lighten_hex(base, 0.2), 0.5))
    elif "wood" in name:
        for i in range(0, spec.height, max(2, spec.height // 6)):
            parts.append(_line(0, i + rng.randint(-1, 1), spec.width, i + rng.randint(-1, 1),
                               _darken_hex(base, 0.2), 0.3))
    elif "snow" in name:
        for _ in range(80):
            x = rng.uniform(0, spec.width)
            y = rng.uniform(0, spec.height)
            parts.append(_circle(x, y, 0.5, _lighten_hex(base, 0.15)))
    elif "circuit" in name:
        for i in range(0, spec.height, max(2, spec.height // 8)):
            parts.append(_line(0, i, spec.width, i, _lighten_hex(base, 0.1), 0.4))
            parts.append(_line(i, 0, i, spec.height, _lighten_hex(base, 0.1), 0.4))
    elif "biomatter" in name:
        for _ in range(14):
            cx = rng.uniform(0, spec.width)
            cy = rng.uniform(0, spec.height)
            parts.append(_ellipse(cx, cy, rng.uniform(2, 6), rng.uniform(1, 3),
                                  _lighten_hex(base, 0.15)))
    # Integrity-band cracks
    if band in ("cracked", "critical", "destroyed"):
        count = {"cracked": 3, "critical": 6, "destroyed": 10}[band]
        for _ in range(count):
            x1 = rng.uniform(0, spec.width)
            y1 = rng.uniform(0, spec.height)
            x2 = x1 + rng.uniform(-spec.width / 3, spec.width / 3)
            y2 = y1 + rng.uniform(-spec.height / 3, spec.height / 3)
            parts.append(_line(x1, y1, x2, y2, _darken_hex(base, 0.35), 1.0))
    # Overlay mode tint (used by the 5-mode M3 overlay).
    if mode is not None:
        tint = {
            "integrity": "#5bd078",
            "pathability": "#3a8cff",
            "mobility": "#dab438",
            "hazard": "#c93030",
            "build_repair": "#8a78ff",
        }.get(mode, "#888888")
        parts.append(_rect(0, 0, spec.width, spec.height, tint))
    return "".join(parts)


def _compose_particle(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    highlight = p.highlight()
    glow = p.glow()
    name = spec.canonical_name
    phase = (spec.extra or {}).get("phase", "spawn")
    cx, cy = spec.width / 2, spec.height / 2
    W = spec.width
    H = spec.height
    import math as _m

    scale = {"spawn": 0.25, "mid": 0.55, "late": 0.80, "dissipate": 1.0}.get(phase, 0.5)
    parts: List[str] = []

    if "impact_dirt" in name:
        # 5-8 small brown splatter dots radiating outward
        count = 5 + int(scale * 6)
        for i in range(count):
            ang = i * (6.28 / count) + rng.uniform(-0.2, 0.2)
            r = W * 0.30 * scale * rng.uniform(0.6, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            sz = W * 0.04 * scale * rng.uniform(0.8, 1.4)
            parts.append(_circle(x, y, sz, "#7a5a33", "#3a2a13", 0.3))
        # Central impact crater
        parts.append(_circle(cx, cy, W * 0.06 * scale, "#3a2a13"))
        return "".join(parts)

    if "impact_concrete" in name:
        # 6-10 concrete spall chunks (irregular polygons)
        count = 6 + int(scale * 4)
        for i in range(count):
            ang = rng.uniform(0, 6.28)
            r = W * 0.32 * scale * rng.uniform(0.4, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            sz = W * 0.04 * scale * rng.uniform(0.6, 1.4)
            parts.append(_polygon([
                (x, y),
                (x + sz, y - sz * 0.5),
                (x + sz * 0.5, y + sz),
                (x - sz * 0.5, y + sz * 0.3),
            ], "#888888", "#444444", 0.3))
        # Dust cloud center
        for _ in range(6):
            dx = cx + rng.uniform(-W * 0.10, W * 0.10) * scale
            dy = cy + rng.uniform(-H * 0.10, H * 0.10) * scale
            parts.append(_circle(dx, dy, W * 0.05 * scale, "#bbbbbb"))
        return "".join(parts)

    if "impact_metal" in name:
        # 3 sharp yellow-white spark rays
        for i in range(3):
            ang = (i * 2.094) + rng.uniform(-0.3, 0.3)
            x1 = cx + W * 0.04 * _m.cos(ang)
            y1 = cy + H * 0.04 * _m.sin(ang)
            x2 = cx + W * 0.35 * scale * _m.cos(ang)
            y2 = cy + H * 0.35 * scale * _m.sin(ang)
            parts.append(_polygon([
                (x1, y1),
                (x2 + 1.5 * _m.cos(ang + 1.57), y2 + 1.5 * _m.sin(ang + 1.57)),
                (x2 - 1.5 * _m.cos(ang + 1.57), y2 - 1.5 * _m.sin(ang + 1.57)),
            ], "#FFE068", "#FFFFAA", 0.5))
        # Central white-hot core
        parts.append(_circle(cx, cy, W * 0.04 * scale, "#FFFFFF"))
        parts.append(_circle(cx, cy, W * 0.08 * scale, "#FFE068", "#FFAA22", 0.3))
        return "".join(parts)

    if "impact_glass" in name:
        # 8 angular crystal shards radiating
        for i in range(8):
            ang = i * 0.7854
            x1 = cx + W * 0.04 * _m.cos(ang)
            y1 = cy + H * 0.04 * _m.sin(ang)
            x2 = cx + W * 0.32 * scale * _m.cos(ang)
            y2 = cy + H * 0.32 * scale * _m.sin(ang)
            x3 = cx + W * 0.28 * scale * _m.cos(ang + 0.20)
            y3 = cy + H * 0.28 * scale * _m.sin(ang + 0.20)
            parts.append(_polygon([(x1, y1), (x2, y2), (x3, y3)],
                                  "#aaccee", "#5588aa", 0.5))
        parts.append(_circle(cx, cy, W * 0.04 * scale, "#FFFFFF"))
        return "".join(parts)

    if "impact_wood" in name:
        # 8-12 splinter shards
        count = 8 + int(scale * 4)
        for i in range(count):
            ang = i * (6.28 / count)
            r = W * 0.28 * scale * rng.uniform(0.5, 1.0)
            x2 = cx + r * _m.cos(ang)
            y2 = cy + r * _m.sin(ang)
            x3 = cx + r * 0.5 * _m.cos(ang + 0.15)
            y3 = cy + r * 0.5 * _m.sin(ang + 0.15)
            parts.append(_polygon([(cx, cy), (x2, y2), (x3, y3)],
                                  "#886633", "#553311", 0.4))
        return "".join(parts)

    if "impact_flesh" in name:
        # Blood spray
        for _ in range(10):
            ang = rng.uniform(0, 6.28)
            r = W * 0.28 * scale * rng.uniform(0.5, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            parts.append(_circle(x, y, rng.uniform(1, 3), "#a52a2a", "#7a0a14", 0.3))
        parts.append(_circle(cx, cy, W * 0.08 * scale, "#7a0a14"))
        return "".join(parts)

    if "spark_muzzle" in name:
        # 5-7 yellow-orange star-burst rays from center
        count = 5 + (1 if scale > 0.5 else 0)
        for i in range(count):
            ang = i * (6.28 / count)
            x1 = cx + W * 0.02 * _m.cos(ang)
            y1 = cy + H * 0.02 * _m.sin(ang)
            x2 = cx + W * 0.40 * scale * _m.cos(ang)
            y2 = cy + H * 0.40 * scale * _m.sin(ang)
            parts.append(_polygon([
                (x1, y1),
                (x2 + 2 * _m.cos(ang + 1.57), y2 + 2 * _m.sin(ang + 1.57)),
                (x2 - 2 * _m.cos(ang + 1.57), y2 - 2 * _m.sin(ang + 1.57)),
            ], "#FFAA22", "#FFFFAA", 0.5))
        # Central bright orb
        parts.append(_circle(cx, cy, W * 0.10 * scale, "#FFFFFF"))
        parts.append(_circle(cx, cy, W * 0.16 * scale, "#FFE068", "#FFAA22", 0.3))
        return "".join(parts)

    if "spark_ricochet" in name:
        # 3 short yellow trails
        for i in range(3):
            ang = (i * 2.094) + rng.uniform(-0.3, 0.3)
            x1 = cx
            y1 = cy
            x2 = cx + W * 0.35 * scale * _m.cos(ang)
            y2 = cy + H * 0.35 * scale * _m.sin(ang)
            parts.append(_line(x1, y1, x2, y2, "#FFE068", 2.0))
            parts.append(_circle(x2, y2, W * 0.025 * scale, "#FFFFAA"))
        parts.append(_circle(cx, cy, W * 0.06 * scale, "#FFFFFF"))
        return "".join(parts)

    if "spark_electric" in name:
        # Zigzag blue lightning bolt + 4-6 satellite dots
        zig_pts = [(cx, cy)]
        x_off = -W * 0.30 * scale
        y_off = -H * 0.30 * scale
        for i in range(6):
            zig_pts.append((cx + x_off + i * W * 0.10 * scale + rng.uniform(-3, 3),
                            cy + y_off + i * H * 0.10 * scale + rng.uniform(-3, 3)))
        parts.append(_polyline(zig_pts, "#3a8cff", 2.0))
        parts.append(_polyline(zig_pts, "#88CCFF", 0.8))
        # Satellite dots
        for _ in range(5):
            x = cx + rng.uniform(-W * 0.25, W * 0.25) * scale
            y = cy + rng.uniform(-H * 0.25, H * 0.25) * scale
            parts.append(_circle(x, y, rng.uniform(0.8, 2.0), "#88CCFF"))
        return "".join(parts)

    if "spark_welding" in name:
        # Welding spark fountain (upward bias)
        for _ in range(20):
            ang = -1.57 + rng.uniform(-0.8, 0.8)  # upward bias
            r = W * 0.40 * scale * rng.uniform(0.4, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            parts.append(_circle(x, y, rng.uniform(0.6, 1.8), "#FFE068"))
        parts.append(_circle(cx, cy, W * 0.06 * scale, "#FFFFFF"))
        parts.append(_circle(cx, cy + 2, W * 0.04 * scale, "#FFAA22"))
        return "".join(parts)

    if "spark" in name:
        # Generic spark fan
        for i in range(8):
            ang = i * 0.7854
            r = W * 0.35 * scale
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            parts.append(_line(cx, cy, x, y, accent if i % 2 == 0 else highlight, 1.5))
        return "".join(parts)

    if "smoke_white" in name:
        # 3-5 overlapping pale circles with low opacity (we use light gray hex)
        for _ in range(5):
            x = cx + rng.uniform(-W * 0.20, W * 0.20) * scale
            y = cy + rng.uniform(-H * 0.20, H * 0.20) * scale
            r = W * 0.16 * scale * rng.uniform(0.7, 1.3)
            parts.append(_circle(x, y, r, "#eeeeee", "#bbbbbb", 0.3))
        return "".join(parts)

    if "smoke_black" in name:
        # 3-5 overlapping dark circles
        for _ in range(5):
            x = cx + rng.uniform(-W * 0.20, W * 0.20) * scale
            y = cy + rng.uniform(-H * 0.20, H * 0.20) * scale
            r = W * 0.16 * scale * rng.uniform(0.7, 1.3)
            parts.append(_circle(x, y, r, "#1a1a1a", "#000000", 0.3))
        return "".join(parts)

    if "smoke_chem" in name:
        # Chemical green/purple plume
        for i, col in enumerate(["#88cc44", "#bb44dd", "#aaff44"]):
            x = cx + rng.uniform(-W * 0.15, W * 0.15) * scale
            y = cy + rng.uniform(-H * 0.15, H * 0.15) * scale
            r = W * 0.18 * scale * rng.uniform(0.7, 1.2)
            parts.append(_circle(x, y, r, col, "#225522", 0.3))
        return "".join(parts)

    if "smoke_steam" in name:
        # Steam vent puff (upward bias)
        for i in range(5):
            x = cx + rng.uniform(-W * 0.15, W * 0.15) * scale
            y = cy - i * H * 0.05 * scale + rng.uniform(-H * 0.05, H * 0.05) * scale
            r = W * (0.10 + i * 0.04) * scale
            parts.append(_circle(x, y, r, "#eeeeff", "#aabbcc", 0.3))
        return "".join(parts)

    if "smoke_fire" in name:
        # Fire-flame smoke plume (orange-red)
        for i, col in enumerate(["#aa3322", "#dd6633", "#FFAA22", "#FFFFFF"]):
            x = cx + rng.uniform(-W * 0.12, W * 0.12) * scale
            y = cy - i * H * 0.05 * scale
            r = W * (0.20 - i * 0.04) * scale
            parts.append(_circle(x, y, r, col, "#330000", 0.3))
        return "".join(parts)

    if "smoke" in name:
        # Generic smoke
        for _ in range(5):
            x = cx + rng.uniform(-W * 0.20, W * 0.20) * scale
            y = cy + rng.uniform(-H * 0.20, H * 0.20) * scale
            r = W * 0.14 * scale * rng.uniform(0.7, 1.3)
            parts.append(_circle(x, y, r, body, _darken_hex(body, 0.20), 0.3))
        return "".join(parts)

    if "ember_glow" in name:
        # Small bright dot with glow halo
        parts.append(_circle(cx, cy, W * 0.16 * scale, "#FFAA22", "#FFE068", 0.5))
        parts.append(_circle(cx, cy, W * 0.08 * scale, "#FFE068"))
        parts.append(_circle(cx, cy, W * 0.04 * scale, "#FFFFFF"))
        return "".join(parts)

    if "ember_fall" in name:
        # Falling ember trail
        for i in range(6):
            tx = cx + i * W * 0.04 * scale
            ty = cy + i * H * 0.06 * scale
            r = W * (0.06 - i * 0.008) * scale
            parts.append(_circle(tx, ty, max(0.5, r), "#FFAA22"))
            parts.append(_circle(tx, ty, max(0.3, r * 0.5), "#FFFFFF"))
        return "".join(parts)

    if "ember_swarm" in name:
        # Ember swarm rising
        for _ in range(15):
            x = cx + rng.uniform(-W * 0.30, W * 0.30) * scale
            y = cy + rng.uniform(-H * 0.30, 0) * scale
            r = rng.uniform(0.8, 2.5)
            parts.append(_circle(x, y, r, "#FFAA22"))
            if rng.random() < 0.5:
                parts.append(_circle(x, y, r * 0.5, "#FFFFFF"))
        return "".join(parts)

    if "ember" in name:
        for _ in range(8):
            x = cx + rng.uniform(-W * 0.20, W * 0.20) * scale
            y = cy + rng.uniform(-H * 0.20, H * 0.20) * scale
            r = W * 0.05 * scale * rng.uniform(0.7, 1.3)
            parts.append(_circle(x, y, r, accent))
        return "".join(parts)

    if "dust_kick" in name:
        # 4 brown wisps with rotational motion suggestion
        for i in range(4):
            ang = i * 1.57 + 0.7
            x = cx + W * 0.10 * scale * _m.cos(ang)
            y = cy + H * 0.10 * scale * _m.sin(ang)
            parts.append(_ellipse(x, y, W * 0.08 * scale, H * 0.04 * scale, "#a08866", "#553311", 0.3))
        # Footprint base shadow
        parts.append(_ellipse(cx, cy + H * 0.10, W * 0.18 * scale, H * 0.04, "#7a5a33", "#3a2a13", 0.3))
        return "".join(parts)

    if "dust_explosion" in name:
        # Expanding dust cloud — concentric rings of clumps
        for ring in range(3):
            ring_r = W * (0.10 + ring * 0.10) * scale
            ring_count = 6 + ring * 3
            for i in range(ring_count):
                ang = i * (6.28 / ring_count)
                x = cx + ring_r * _m.cos(ang)
                y = cy + ring_r * _m.sin(ang)
                shade = "#bbbbbb" if ring == 0 else ("#999999" if ring == 1 else "#777777")
                parts.append(_circle(x, y, rng.uniform(2, 4) * scale, shade,
                                     "#555555", 0.3))
        return "".join(parts)

    if "dust" in name or "dust_smoke_combo" in name:
        # Generic dust
        for _ in range(10):
            x = cx + rng.uniform(-W * 0.20, W * 0.20) * scale
            y = cy + rng.uniform(-H * 0.20, H * 0.20) * scale
            r = W * 0.08 * scale * rng.uniform(0.6, 1.4)
            parts.append(_circle(x, y, r, "#a08866", "#553311", 0.3))
        return "".join(parts)

    if "debris_chunk" in name:
        # Chunk silhouettes
        for _ in range(6):
            x = cx + rng.uniform(-W * 0.25, W * 0.25) * scale
            y = cy + rng.uniform(-H * 0.25, H * 0.25) * scale
            sz = W * 0.06 * scale * rng.uniform(0.7, 1.3)
            parts.append(_polygon([
                (x, y),
                (x + sz, y + sz * 0.3),
                (x + sz * 0.7, y + sz),
                (x - sz * 0.2, y + sz * 0.7),
            ], _darken_hex(body, 0.30), dark_or(p.dark(), "#000000"), 0.4))
        return "".join(parts)

    if "debris_shrapnel" in name:
        # Shrapnel shard cluster
        for _ in range(10):
            ang = rng.uniform(0, 6.28)
            r = W * 0.20 * scale * rng.uniform(0.5, 1.2)
            x1 = cx
            y1 = cy
            x2 = cx + r * _m.cos(ang)
            y2 = cy + r * _m.sin(ang)
            parts.append(_polygon([
                (x1, y1),
                (x2, y2),
                (x2 + 2 * _m.cos(ang + 1.57), y2 + 2 * _m.sin(ang + 1.57)),
            ], "#666666", "#222222", 0.3))
        return "".join(parts)

    if "fluid_blood" in name:
        # 4-6 red droplet polygons
        for i in range(6):
            ang = i * 1.047
            r = W * 0.20 * scale * rng.uniform(0.6, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            sz = W * 0.04 * scale
            parts.append(_polygon([
                (x, y - sz),
                (x + sz * 0.7, y + sz * 0.2),
                (x, y + sz),
                (x - sz * 0.7, y + sz * 0.2),
            ], "#a52a2a", "#7a0a14", 0.4))
        parts.append(_circle(cx, cy, W * 0.05 * scale, "#7a0a14"))
        return "".join(parts)

    if "fluid_oil" in name:
        # 4-6 dark teardrop polygons
        for i in range(6):
            ang = i * 1.047
            r = W * 0.20 * scale * rng.uniform(0.6, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            sz = W * 0.04 * scale
            parts.append(_polygon([
                (x, y - sz),
                (x + sz * 0.7, y + sz * 0.2),
                (x, y + sz),
                (x - sz * 0.7, y + sz * 0.2),
            ], "#1a1a22", "#0d0d11", 0.4))
        parts.append(_circle(cx, cy, W * 0.05 * scale, "#000000"))
        return "".join(parts)

    if "fluid_water" in name:
        # Water splash droplets
        for i in range(8):
            ang = rng.uniform(0, 6.28)
            r = W * 0.22 * scale * rng.uniform(0.5, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            parts.append(_circle(x, y, rng.uniform(1.5, 3), "#3a8cff", "#1a4a88", 0.4))
            parts.append(_circle(x, y, rng.uniform(0.5, 1.5), "#88CCFF"))
        return "".join(parts)

    if "fluid_coolant" in name:
        # Coolant cyan spray
        for i in range(8):
            ang = rng.uniform(0, 6.28)
            r = W * 0.22 * scale * rng.uniform(0.5, 1.0)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            parts.append(_circle(x, y, rng.uniform(1.5, 3), "#22ddee", "#0a5588", 0.4))
            parts.append(_circle(x, y, rng.uniform(0.5, 1.5), "#aaffff"))
        return "".join(parts)

    if "fluid" in name:
        for _ in range(8):
            x = cx + rng.uniform(-W * 0.20, W * 0.20) * scale
            y = cy + rng.uniform(-H * 0.20, H * 0.20) * scale
            r = W * 0.05 * scale * rng.uniform(0.6, 1.4)
            parts.append(_circle(x, y, r, accent))
        return "".join(parts)

    if "glow_orb" in name:
        # Soft halo orb
        parts.append(_circle(cx, cy, W * 0.36 * scale, glow, accent, 0.3))
        parts.append(_circle(cx, cy, W * 0.22 * scale, _lighten_hex(glow, 0.20)))
        parts.append(_circle(cx, cy, W * 0.10 * scale, "#FFFFFF"))
        return "".join(parts)

    if "glow_lens_flare" in name:
        # Lens flare star pattern
        parts.append(_circle(cx, cy, W * 0.14 * scale, "#FFFFFF"))
        for i in range(4):
            ang = i * 1.57
            x1 = cx + W * 0.04 * _m.cos(ang)
            y1 = cy + H * 0.04 * _m.sin(ang)
            x2 = cx + W * 0.38 * scale * _m.cos(ang)
            y2 = cy + H * 0.38 * scale * _m.sin(ang)
            parts.append(_polygon([
                (x1, y1),
                (x2 + 1 * _m.cos(ang + 1.57), y2 + 1 * _m.sin(ang + 1.57)),
                (x2 - 1 * _m.cos(ang + 1.57), y2 - 1 * _m.sin(ang + 1.57)),
            ], glow, accent, 0.5))
        parts.append(_circle(cx, cy, W * 0.20 * scale, "none", glow, 1.0))
        return "".join(parts)

    if "glow_arcane" in name:
        # Arcane sigil
        parts.append(_circle(cx, cy, W * 0.30 * scale, "none", "#aa44ff", 2.0))
        parts.append(_circle(cx, cy, W * 0.20 * scale, "none", "#dd88ff", 1.5))
        for i in range(6):
            ang = i * 1.047
            x = cx + W * 0.22 * scale * _m.cos(ang)
            y = cy + H * 0.22 * scale * _m.sin(ang)
            parts.append(_circle(x, y, W * 0.04 * scale, "#dd88ff", "#aa44ff", 0.5))
        parts.append(_circle(cx, cy, W * 0.06 * scale, "#FFFFFF"))
        return "".join(parts)

    # Generic glow orb
    parts.append(_circle(cx, cy, W * 0.30 * scale, glow))
    parts.append(_circle(cx, cy, W * 0.18 * scale, highlight))
    parts.append(_circle(cx, cy, W * 0.10 * scale, accent))
    return "".join(parts)


def _compose_terrain_tile(spec: AssetSpec, rng: random.Random) -> str:
    """M12 polish-pass: terrain tiles with visible 64x64-scale texture.

    Each material type gets a distinct visible pattern at 64x64 tile resolution:
    proper rivets / plates / grain / cracks / facets / bubbles / circuit traces.
    Stroke widths thickened to 1.5-2.0px (was 0.4-0.5px = sub-pixel invisible).
    Pattern density tuned for tileable + recognizable.
    """
    base = (spec.extra or {}).get("base_color", spec.palette.primary())
    variant = (spec.extra or {}).get("variant", "a")
    seed_offset = sum(ord(c) for c in variant)
    rng2 = random.Random(spec.seed + seed_offset)
    parts: List[str] = [_rect(0, 0, spec.width, spec.height, base)]
    name = spec.canonical_name
    W = spec.width
    H = spec.height
    dark1 = _darken_hex(base, 0.20)
    dark2 = _darken_hex(base, 0.35)
    light1 = _lighten_hex(base, 0.18)
    light2 = _lighten_hex(base, 0.30)

    if "dirt" in name:
        # Heavy granular speckles + scattered pebbles + soil clumps
        for _ in range(60):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(0.8, 2.2), dark1))
        for _ in range(15):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(2.5, 4.5), dark2))
        for _ in range(8):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(1.2, 2.0), light1))
    elif "sand" in name:
        # Fine grain + occasional ripple line
        for _ in range(80):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(0.6, 1.6), dark1))
        for _ in range(4):
            y = rng2.uniform(8, H - 8)
            parts.append(_polyline([
                (0, y),
                (W * 0.25, y + rng2.uniform(-3, 3)),
                (W * 0.50, y + rng2.uniform(-3, 3)),
                (W * 0.75, y + rng2.uniform(-3, 3)),
                (W, y + rng2.uniform(-3, 3)),
            ], light1, 1.2))
    elif "mud" in name:
        # Wet brown with reflective highlights + occasional dark patches
        for _ in range(20):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_ellipse(x, y, rng2.uniform(3, 6), rng2.uniform(2, 3), dark1))
        for _ in range(8):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_ellipse(x, y, rng2.uniform(2, 4), rng2.uniform(1, 2), light2))
    elif "loose" in name:
        # Loose rubble: visible chunks + edge fragments
        for _ in range(18):
            cx = rng2.uniform(4, W - 4)
            cy = rng2.uniform(4, H - 4)
            angle = rng2.uniform(0, 6.28)
            import math as _ms
            r = rng2.uniform(2, 4)
            pts = []
            for j in range(5):
                a = angle + j * 1.256 + rng2.uniform(-0.3, 0.3)
                rr = r * rng2.uniform(0.6, 1.2)
                pts.append((cx + rr * _ms.cos(a), cy + rr * _ms.sin(a)))
            parts.append(_polygon(pts, dark1, dark2, 0.8))
    elif "concrete" in name:
        # Concrete: visible plate seams + corner rivets + texture noise
        # Main horizontal seam
        parts.append(_line(0, H * 0.5, W, H * 0.5, dark2, 1.8))
        # Main vertical seam
        parts.append(_line(W * 0.5, 0, W * 0.5, H, dark2, 1.8))
        # Rivets at quad corners
        for ix in [0.0, 0.5, 1.0]:
            for iy in [0.0, 0.5, 1.0]:
                if ix in (0.5,) and iy in (0.5,):
                    continue  # skip center
                rx = W * ix
                ry = H * iy
                if 0 <= rx <= W and 0 <= ry <= H:
                    parts.append(_circle(rx, ry, 1.5, dark2))
                    parts.append(_circle(rx, ry, 0.8, light1))
        # Texture noise speckles
        for _ in range(35):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(0.5, 1.2), dark1))
    elif "anchor" in name:
        # Anchor rock: granite-style mineral grain + stratified bands
        for i in range(4):
            y = H * (0.1 + 0.25 * i) + rng2.uniform(-3, 3)
            parts.append(_polyline([
                (0, y), (W * 0.3, y + rng2.uniform(-2, 2)),
                (W * 0.7, y + rng2.uniform(-2, 2)), (W, y),
            ], dark2, 1.2))
        # Mineral specks
        for _ in range(40):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(0.4, 1.2), dark1 if rng2.random() < 0.6 else light1))
        for _ in range(8):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(1.5, 2.5), light2))
    elif "metal" in name or "alloy" in name:
        # Brushed metal: horizontal grain + corner rivets + central plate seam
        for i in range(0, H, 4):
            parts.append(_line(0, i, W, i, light1, 1.0))
        # 4 corner rivets
        for ix in [0.10, 0.90]:
            for iy in [0.10, 0.90]:
                parts.append(_circle(W * ix, H * iy, 2.0, dark2))
                parts.append(_circle(W * ix, H * iy, 1.2, light2))
        # Central horizontal plate seam
        parts.append(_line(0, H * 0.5, W, H * 0.5, dark2, 1.5))
        # Small diagonal highlights
        for _ in range(6):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_line(x, y, x + 4, y - 2, light2, 0.8))
    elif "hazard" in name:
        # Yellow + black diagonal hazard stripes (industrial caution)
        stripe_w = max(4, W // 6)
        for i in range(-H, W + H, stripe_w * 2):
            parts.append(_polygon([
                (i, 0), (i + stripe_w, 0), (i + stripe_w + H, H), (i + H, H),
            ], dark2))
    elif "lava" in name:
        # Glowing molten with bright spots + flowing cracks
        # Cracks (light flows)
        for _ in range(4):
            x1 = rng2.uniform(0, W)
            y1 = rng2.uniform(0, H)
            x2 = x1 + rng2.uniform(-W * 0.4, W * 0.4)
            y2 = y1 + rng2.uniform(-H * 0.4, H * 0.4)
            parts.append(_line(x1, y1, x2, y2, light2, 1.5))
        # Hot bright dots
        for _ in range(12):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            r = rng2.uniform(1.5, 3.5)
            parts.append(_circle(x, y, r, light2))
            parts.append(_circle(x, y, r * 0.5, "#FFFFAA"))
        # Cooler dark patches
        for _ in range(8):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(2, 4), dark1))
    elif "ice" in name:
        # Crystalline facets: angular line clusters + bright corner reflections
        # Polygonal facet boundaries
        for _ in range(8):
            cx = rng2.uniform(W * 0.2, W * 0.8)
            cy = rng2.uniform(H * 0.2, H * 0.8)
            import math as _mi
            for j in range(3):
                a1 = rng2.uniform(0, 6.28)
                a2 = a1 + 2.094 + rng2.uniform(-0.3, 0.3)
                r1 = rng2.uniform(3, 7)
                r2 = rng2.uniform(3, 7)
                parts.append(_line(
                    cx + r1 * _mi.cos(a1), cy + r1 * _mi.sin(a1),
                    cx + r2 * _mi.cos(a2), cy + r2 * _mi.sin(a2),
                    light2, 1.2,
                ))
        # Reflective highlights
        for _ in range(5):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(1.2, 2.0), "#FFFFFF"))
    elif "glass" in name:
        # Faint crystalline structure + corner light reflections
        for i in range(4):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_line(x, y, x + rng2.uniform(5, 12), y + rng2.uniform(5, 12), light2, 1.0))
        # Corner reflections
        for ix, iy in [(0.10, 0.10), (0.90, 0.10), (0.10, 0.90), (0.90, 0.90)]:
            parts.append(_circle(W * ix, H * iy, 2.0, light2))
            parts.append(_circle(W * ix, H * iy, 1.2, "#FFFFFF"))
    elif "wood" in name:
        # Wood: visible plank seams + grain lines + occasional knots
        # Plank seams (horizontal)
        for i in range(1, 4):
            y = H * (i / 4.0)
            parts.append(_line(0, y, W, y, dark2, 1.5))
        # Grain lines per plank
        for plank in range(4):
            y_base = H * (plank / 4.0)
            for _ in range(5):
                y = y_base + rng2.uniform(2, H / 4 - 2)
                parts.append(_line(0, y, W, y + rng2.uniform(-1, 1), dark1, 0.8))
        # Knots
        for _ in range(2):
            kx = rng2.uniform(W * 0.2, W * 0.8)
            ky = rng2.uniform(H * 0.1, H * 0.9)
            parts.append(_circle(kx, ky, 2.5, dark2))
            parts.append(_circle(kx, ky, 1.5, dark1))
            parts.append(_circle(kx, ky, 0.8, light1))
    elif "snow" in name:
        # Fluffy snow: many tiny bright dots + faint drift shadows
        # Drift shadow base
        for _ in range(4):
            y = rng2.uniform(H * 0.3, H * 0.7)
            parts.append(_polyline([
                (0, y), (W * 0.25, y + rng2.uniform(-2, 2)),
                (W * 0.50, y + rng2.uniform(-2, 2)),
                (W * 0.75, y + rng2.uniform(-2, 2)),
                (W, y + rng2.uniform(-2, 2)),
            ], dark1, 1.2))
        # Snow grain
        for _ in range(80):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(0.6, 1.4), "#FFFFFF"))
        # Sparkle reflections
        for _ in range(6):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(0.4, 1.0), light2))
    elif "circuit" in name:
        # Visible circuit traces + small chip squares + LED dots
        # Main grid
        for i in range(0, H, 8):
            parts.append(_line(0, i, W, i, light1, 1.0))
            parts.append(_line(i, 0, i, H, light1, 1.0))
        # Branch traces
        for _ in range(6):
            x1 = rng2.choice([8, 16, 24, 32, 40, 48, 56])
            y1 = rng2.choice([8, 16, 24, 32, 40, 48, 56])
            x2 = x1 + rng2.choice([-8, 8])
            y2 = y1 + rng2.choice([-8, 8])
            parts.append(_line(x1, y1, x2, y2, light2, 1.5))
        # Chip pads + LEDs
        for _ in range(4):
            x = rng2.choice([16, 24, 40, 48])
            y = rng2.choice([16, 24, 40, 48])
            parts.append(_rect(x - 3, y - 3, 6, 6, dark2))
            parts.append(_circle(x, y, 1.0, "#00FFAA"))
    elif "repair" in name:
        # Repair foam: cellular bubble pattern with rim highlights
        for _ in range(20):
            x = rng2.uniform(2, W - 2)
            y = rng2.uniform(2, H - 2)
            r = rng2.uniform(2, 4.5)
            parts.append(_circle(x, y, r, dark1, light1, 0.8))
            parts.append(_circle(x, y, r * 0.6, light2))
        # Edge cohesion outline
        parts.append(_rect(0.5, 0.5, W - 1, H - 1, "none", dark2, 0.8))
    elif "biomatter" in name:
        # Organic streaks + veins + occasional spore dots
        for _ in range(12):
            cx = rng2.uniform(0, W)
            cy = rng2.uniform(0, H)
            parts.append(_ellipse(cx, cy, rng2.uniform(4, 7), rng2.uniform(2, 3), light2))
        # Vein tracery
        for _ in range(6):
            x1 = rng2.uniform(0, W)
            y1 = rng2.uniform(0, H)
            for _ in range(4):
                x2 = x1 + rng2.uniform(-8, 8)
                y2 = y1 + rng2.uniform(-8, 8)
                parts.append(_line(x1, y1, x2, y2, dark2, 1.2))
                x1, y1 = x2, y2
        # Spore dots
        for _ in range(8):
            x = rng2.uniform(0, W)
            y = rng2.uniform(0, H)
            parts.append(_circle(x, y, rng2.uniform(1.2, 2.0), light1))
    elif "metal_nohook" in name:
        # Reinforced metal: heavy rivets + plate boundaries + warning chevron
        # Plate boundaries
        for i in range(1, 3):
            x = W * (i / 3.0)
            parts.append(_line(x, 0, x, H, dark2, 2.0))
        parts.append(_line(0, H * 0.5, W, H * 0.5, dark2, 2.0))
        # Heavy rivets
        for ix in [0.10, 0.45, 0.55, 0.90]:
            for iy in [0.10, 0.45, 0.55, 0.90]:
                parts.append(_circle(W * ix, H * iy, 2.5, dark2))
                parts.append(_circle(W * ix, H * iy, 1.8, light1))
                parts.append(_circle(W * ix, H * iy, 0.6, dark2))
        # Warning chevron
        parts.append(_polygon([
            (W * 0.4, H * 0.32), (W * 0.6, H * 0.32), (W * 0.5, H * 0.40),
        ], dark2))
    return "".join(parts)


def _compose_cosmetic_stub(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    parts: List[str] = []
    name = spec.canonical_name
    cx, cy = spec.width / 2, spec.height / 2

    parts.append(_rect(spec.width * 0.10, spec.height * 0.10, spec.width * 0.80, spec.height * 0.80,
                       body, dark, max(0.5, spec.width * 0.01)))
    if "helmet" in name or "decal" in name or "emblem" in name:
        parts.append(_circle(cx, cy, spec.width * 0.22, accent, dark, 1.0))
        parts.append(_circle(cx, cy, spec.width * 0.10, highlight))
    elif "pauldron" in name or "stripe" in name or "band" in name or "trim" in name:
        parts.append(_rect(spec.width * 0.14, spec.height * 0.45, spec.width * 0.72, spec.height * 0.10,
                           accent, dark, 0.5))
    elif "banner" in name:
        parts.append(_polygon(
            [
                (spec.width * 0.30, spec.height * 0.15),
                (spec.width * 0.70, spec.height * 0.15),
                (spec.width * 0.65, spec.height * 0.85),
                (cx, spec.height * 0.75),
                (spec.width * 0.35, spec.height * 0.85),
            ],
            accent, dark, 1.0,
        ))
    elif "skin" in name:
        for i in range(4):
            parts.append(_rect(spec.width * 0.14, spec.height * (0.18 + 0.18 * i),
                               spec.width * 0.72, spec.height * 0.08,
                               accent if i % 2 == 0 else highlight, dark, 0.5))
    return "".join(parts)


def _compose_emblem(spec: AssetSpec, rng: random.Random) -> str:
    """M12 polish-pass: faction emblems as heraldic crests.

    Each faction gets a faction-specific shield shape + layered border treatment
    + iconic mark + motto banner (full variant) instead of a generic
    circle+icon. Per Coalition / Frontier / Ronin / Synth / Crystalfold / Husks /
    Collegium / Starlight visual identity from factions_full.json.
    """
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    glow = p.glow()
    metal = p.metal()
    name = spec.canonical_name
    cx, cy = spec.width / 2, spec.height / 2
    simple = "_simple" in name
    W = spec.width
    H = spec.height
    import math as _math

    parts: List[str] = []

    # ─── Faction-specific shield shapes ──────────────────────────────────
    if "hostile_corp" in name:
        # COALITION-style heater shield (rounded top, pointed bottom)
        shield_pts = [
            (cx - W * 0.34, cy - H * 0.32),
            (cx + W * 0.34, cy - H * 0.32),
            (cx + W * 0.34, cy + H * 0.04),
            (cx, cy + H * 0.42),
            (cx - W * 0.34, cy + H * 0.04),
        ]
        parts.append(_polygon(shield_pts, dark, accent, max(2.0, W * 0.012)))
        # Inner field
        parts.append(_polygon([
            (cx - W * 0.28, cy - H * 0.26),
            (cx + W * 0.28, cy - H * 0.26),
            (cx + W * 0.28, cy + H * 0.02),
            (cx, cy + H * 0.34),
            (cx - W * 0.28, cy + H * 0.02),
        ], body, dark, 1.0))
        # Tower-anchor motif
        parts.append(_rect(cx - W * 0.04, cy - H * 0.18, W * 0.08, H * 0.30, accent, dark, 0.8))
        parts.append(_rect(cx - W * 0.10, cy - H * 0.22, W * 0.20, H * 0.06, accent, dark, 0.8))
        parts.append(_rect(cx - W * 0.14, cy - H * 0.26, W * 0.28, H * 0.04, highlight))
        # Crenellation
        for i in range(3):
            xx = cx - W * 0.10 + i * W * 0.08
            parts.append(_rect(xx, cy - H * 0.30, W * 0.05, H * 0.04, accent))
    elif "allied_resistance" in name:
        # COALITION-alt or FRONTIER-style: spearpoint shield
        shield_pts = [
            (cx - W * 0.32, cy - H * 0.34),
            (cx + W * 0.32, cy - H * 0.34),
            (cx + W * 0.18, cy + H * 0.10),
            (cx, cy + H * 0.42),
            (cx - W * 0.18, cy + H * 0.10),
        ]
        parts.append(_polygon(shield_pts, dark, accent, max(2.0, W * 0.012)))
        parts.append(_polygon([
            (cx - W * 0.26, cy - H * 0.28),
            (cx + W * 0.26, cy - H * 0.28),
            (cx + W * 0.14, cy + H * 0.08),
            (cx, cy + H * 0.34),
            (cx - W * 0.14, cy + H * 0.08),
        ], body, dark, 1.0))
        # Crossed wrench-and-rifle, larger + detailed
        parts.append(_polygon([
            (cx - W * 0.22, cy - H * 0.20),
            (cx + W * 0.20, cy + H * 0.18),
            (cx + W * 0.22, cy + H * 0.16),
            (cx - W * 0.20, cy - H * 0.22),
        ], accent, dark, 0.8))
        parts.append(_polygon([
            (cx - W * 0.20, cy + H * 0.18),
            (cx + W * 0.22, cy - H * 0.20),
            (cx + W * 0.20, cy - H * 0.22),
            (cx - W * 0.22, cy + H * 0.16),
        ], accent, dark, 0.8))
        parts.append(_circle(cx, cy, W * 0.05, glow, dark, 0.5))
    elif "marauder" in name:
        # FRONTIER / Husks-style irregular star-burst
        # Outer jagged shield
        burst_pts = []
        for i in range(10):
            ang = _math.radians(36 * i - 90)
            r = W * (0.36 if i % 2 == 0 else 0.24)
            burst_pts.append((cx + r * _math.cos(ang), cy + r * _math.sin(ang)))
        parts.append(_polygon(burst_pts, dark, accent, max(2.0, W * 0.010)))
        # Inner pentagon
        pent_pts = []
        for i in range(5):
            ang = _math.radians(72 * i - 90)
            r = W * 0.22
            pent_pts.append((cx + r * _math.cos(ang), cy + r * _math.sin(ang)))
        parts.append(_polygon(pent_pts, body, dark, 1.0))
        # Tribal mark — drill-and-thread cross
        parts.append(_polygon([
            (cx - W * 0.10, cy - H * 0.18),
            (cx + W * 0.10, cy - H * 0.18),
            (cx + W * 0.06, cy + H * 0.18),
            (cx - W * 0.06, cy + H * 0.18),
        ], accent, dark, 0.8))
        # Drill-tip
        parts.append(_polygon([
            (cx - W * 0.06, cy + H * 0.18),
            (cx + W * 0.06, cy + H * 0.18),
            (cx, cy + H * 0.24),
        ], highlight, dark, 0.5))
        # Thread/bind dots
        for i in range(3):
            parts.append(_circle(cx, cy - H * 0.10 + i * H * 0.08, W * 0.015, glow))
    elif "religious_order" in name:
        # RONIN-style circular medallion with crossed katanas + thread loop
        # Outer ring
        parts.append(_circle(cx, cy, W * 0.40, dark, accent, max(2.0, W * 0.012)))
        parts.append(_circle(cx, cy, W * 0.36, body, dark, 1.0))
        # Crossed katanas
        sword_a_pts = [
            (cx - W * 0.26, cy - H * 0.20),
            (cx + W * 0.26, cy + H * 0.20),
            (cx + W * 0.28, cy + H * 0.16),
            (cx - W * 0.24, cy - H * 0.24),
        ]
        parts.append(_polygon(sword_a_pts, metal, dark, 0.8))
        sword_b_pts = [
            (cx - W * 0.26, cy + H * 0.20),
            (cx + W * 0.26, cy - H * 0.20),
            (cx + W * 0.24, cy - H * 0.24),
            (cx - W * 0.28, cy + H * 0.16),
        ]
        parts.append(_polygon(sword_b_pts, metal, dark, 0.8))
        # Handle wraps
        parts.append(_rect(cx - W * 0.28, cy - H * 0.22, W * 0.08, H * 0.04, accent))
        parts.append(_rect(cx + W * 0.20, cy + H * 0.18, W * 0.08, H * 0.04, accent))
        # Center binding ring
        parts.append(_circle(cx, cy, W * 0.06, accent, dark, 0.8))
        parts.append(_circle(cx, cy, W * 0.03, glow))
        # Decorative inner ring lines
        parts.append(_circle(cx, cy, W * 0.30, "none", accent, max(1.0, W * 0.005)))
    elif "scientist_order" in name:
        # SYNTH / Collegium-style: hexagonal frame with central data-mandala
        hex_pts = []
        for i in range(6):
            ang = _math.radians(60 * i - 30)
            r = W * 0.40
            hex_pts.append((cx + r * _math.cos(ang), cy + r * _math.sin(ang)))
        parts.append(_polygon(hex_pts, dark, accent, max(2.0, W * 0.012)))
        # Inner hex
        inner_hex = []
        for i in range(6):
            ang = _math.radians(60 * i - 30)
            r = W * 0.32
            inner_hex.append((cx + r * _math.cos(ang), cy + r * _math.sin(ang)))
        parts.append(_polygon(inner_hex, body, dark, 1.0))
        # Central atom / circuit mandala
        parts.append(_circle(cx, cy, W * 0.08, glow, dark, 0.5))
        parts.append(_circle(cx, cy, W * 0.04, accent))
        # 3 orbit ellipses
        for rot in range(3):
            ang = _math.radians(60 * rot)
            # Generate ellipse approx via 16-point polygon (since _ellipse doesn't rotate)
            ell_pts = []
            for j in range(16):
                p_ang = _math.radians(22.5 * j)
                x_local = W * 0.18 * _math.cos(p_ang)
                y_local = H * 0.06 * _math.sin(p_ang)
                x_rot = x_local * _math.cos(ang) - y_local * _math.sin(ang)
                y_rot = x_local * _math.sin(ang) + y_local * _math.cos(ang)
                ell_pts.append((cx + x_rot, cy + y_rot))
            # Approximate ellipse by drawing edges between adjacent points
            for i in range(len(ell_pts)):
                a = ell_pts[i]
                b = ell_pts[(i + 1) % len(ell_pts)]
                parts.append(_line(a[0], a[1], b[0], b[1], accent, max(1.0, W * 0.005)))
        # Corner runes
        for i in range(6):
            ang = _math.radians(60 * i - 30)
            r = W * 0.28
            px = cx + r * _math.cos(ang)
            py = cy + r * _math.sin(ang)
            parts.append(_circle(px, py, W * 0.012, highlight))
    elif "mercenary_guild" in name:
        # MERC-style: shield-shape (Norman shield: rounded top, pointed bottom, flared)
        shield_pts = [
            (cx - W * 0.34, cy - H * 0.30),
            (cx + W * 0.34, cy - H * 0.30),
            (cx + W * 0.30, cy + H * 0.16),
            (cx, cy + H * 0.40),
            (cx - W * 0.30, cy + H * 0.16),
        ]
        parts.append(_polygon(shield_pts, dark, accent, max(2.0, W * 0.012)))
        parts.append(_polygon([
            (cx - W * 0.28, cy - H * 0.24),
            (cx + W * 0.28, cy - H * 0.24),
            (cx + W * 0.24, cy + H * 0.14),
            (cx, cy + H * 0.32),
            (cx - W * 0.24, cy + H * 0.14),
        ], body, dark, 1.0))
        # Horizontal coin/bar with chevron
        parts.append(_rect(cx - W * 0.20, cy - H * 0.04, W * 0.40, H * 0.10, accent, dark, 0.8))
        parts.append(_rect(cx - W * 0.18, cy - H * 0.02, W * 0.36, H * 0.06, highlight))
        # Chevron above
        parts.append(_polygon([
            (cx - W * 0.18, cy - H * 0.08),
            (cx, cy - H * 0.18),
            (cx + W * 0.18, cy - H * 0.08),
            (cx + W * 0.14, cy - H * 0.06),
            (cx, cy - H * 0.12),
            (cx - W * 0.14, cy - H * 0.06),
        ], accent, dark, 0.5))
        # Crossed coin underneath
        parts.append(_circle(cx, cy + H * 0.18, W * 0.06, accent, dark, 0.5))
        parts.append(_circle(cx, cy + H * 0.18, W * 0.04, highlight))
    elif "pirates" in name:
        # PIRATES-style: skull with crossbones on flagpoint shield
        # Flag/banner shape
        parts.append(_polygon([
            (cx - W * 0.32, cy - H * 0.36),
            (cx + W * 0.32, cy - H * 0.36),
            (cx + W * 0.28, cy + H * 0.28),
            (cx, cy + H * 0.38),
            (cx - W * 0.28, cy + H * 0.28),
        ], dark, accent, max(2.0, W * 0.012)))
        parts.append(_polygon([
            (cx - W * 0.28, cy - H * 0.30),
            (cx + W * 0.28, cy - H * 0.30),
            (cx + W * 0.24, cy + H * 0.24),
            (cx, cy + H * 0.32),
            (cx - W * 0.24, cy + H * 0.24),
        ], body, dark, 1.0))
        # Crossbones X
        parts.append(_polygon([
            (cx - W * 0.20, cy + H * 0.08),
            (cx + W * 0.20, cy - H * 0.20),
            (cx + W * 0.22, cy - H * 0.16),
            (cx - W * 0.18, cy + H * 0.12),
        ], accent, dark, 0.8))
        parts.append(_polygon([
            (cx - W * 0.20, cy - H * 0.20),
            (cx + W * 0.20, cy + H * 0.08),
            (cx + W * 0.18, cy + H * 0.12),
            (cx - W * 0.22, cy - H * 0.16),
        ], accent, dark, 0.8))
        # Bone ends
        for x_off in [-0.20, 0.20]:
            for y_off in [-0.20, 0.08]:
                parts.append(_circle(cx + W * x_off, cy + H * y_off, W * 0.03, highlight, dark, 0.5))
        # Skull silhouette on top
        parts.append(_circle(cx, cy - H * 0.10, W * 0.13, highlight, dark, 0.8))
        # Skull eyes
        parts.append(_circle(cx - W * 0.05, cy - H * 0.10, W * 0.025, dark))
        parts.append(_circle(cx + W * 0.05, cy - H * 0.10, W * 0.025, dark))
        # Skull nose
        parts.append(_polygon([
            (cx, cy - H * 0.06),
            (cx - W * 0.015, cy - H * 0.02),
            (cx + W * 0.015, cy - H * 0.02),
        ], dark))
        # Skull teeth
        for i in range(4):
            tx = cx - W * 0.05 + i * W * 0.027
            parts.append(_rect(tx, cy + H * 0.00, W * 0.02, H * 0.03, dark))
    elif "drone_collective" in name:
        # SYNTH-style: faceted hex + circuit-mandala
        hex_pts = []
        for i in range(6):
            ang = _math.radians(60 * i)
            r = W * 0.40
            hex_pts.append((cx + r * _math.cos(ang), cy + r * _math.sin(ang)))
        parts.append(_polygon(hex_pts, dark, accent, max(2.0, W * 0.012)))
        # Inner hex
        inner_hex = []
        for i in range(6):
            ang = _math.radians(60 * i)
            r = W * 0.32
            inner_hex.append((cx + r * _math.cos(ang), cy + r * _math.sin(ang)))
        parts.append(_polygon(inner_hex, body, dark, 1.0))
        # 6-armed circuit star
        for i in range(6):
            ang = _math.radians(60 * i)
            x = cx + W * 0.26 * _math.cos(ang)
            y = cy + H * 0.26 * _math.sin(ang)
            parts.append(_line(cx, cy, x, y, accent, max(2.0, W * 0.008)))
            parts.append(_circle(x, y, W * 0.03, glow, dark, 0.5))
        # Central drone-eye
        parts.append(_circle(cx, cy, W * 0.10, dark))
        parts.append(_circle(cx, cy, W * 0.06, glow))
        parts.append(_circle(cx, cy, W * 0.03, highlight))
        # Hex faces — 6 inner triangle highlights
        for i in range(6):
            a1 = _math.radians(60 * i)
            a2 = _math.radians(60 * (i + 1))
            r1 = W * 0.20
            r2 = W * 0.18
            x1 = cx + r1 * _math.cos(a1)
            y1 = cy + r1 * _math.sin(a1)
            x2 = cx + r1 * _math.cos(a2)
            y2 = cy + r1 * _math.sin(a2)
            xm = cx + r2 * _math.cos(_math.radians(60 * i + 30))
            ym = cy + r2 * _math.sin(_math.radians(60 * i + 30))
            parts.append(_polygon([(x1, y1), (x2, y2), (xm, ym)], highlight, dark, 0.3))
    else:
        # Generic — render a star-burst
        parts.append(_circle(cx, cy, min(W, H) * 0.40, dark, accent, max(2.0, W * 0.012)))
        parts.append(_circle(cx, cy, min(W, H) * 0.32, body, dark, 1.0))
        burst_pts = []
        for i in range(10):
            ang = _math.radians(36 * i - 90)
            r = W * (0.24 if i % 2 == 0 else 0.10)
            burst_pts.append((cx + r * _math.cos(ang), cy + r * _math.sin(ang)))
        parts.append(_polygon(burst_pts, accent, dark, 0.8))

    # Full variant adds motto banner + outer wreath ornament
    if not simple:
        # Outer wreath dots
        for i in range(12):
            ang = _math.radians(30 * i)
            r = W * 0.48
            x = cx + r * _math.cos(ang)
            y = cy + r * _math.sin(ang)
            parts.append(_circle(x, y, W * 0.012, accent, dark, 0.3))
        # Motto banner ribbon at bottom
        parts.append(_polygon([
            (cx - W * 0.36, cy + H * 0.44),
            (cx + W * 0.36, cy + H * 0.44),
            (cx + W * 0.32, cy + H * 0.49),
            (cx, cy + H * 0.46),
            (cx - W * 0.32, cy + H * 0.49),
        ], accent, dark, 1.0))
        parts.append(_polygon([
            (cx - W * 0.34, cy + H * 0.452),
            (cx + W * 0.34, cy + H * 0.452),
            (cx + W * 0.30, cy + H * 0.48),
            (cx, cy + H * 0.47),
            (cx - W * 0.30, cy + H * 0.48),
        ], highlight, dark, 0.5))
        # 3 stitching dots on banner
        for i in range(3):
            parts.append(_circle(cx - W * 0.18 + i * W * 0.18, cy + H * 0.465, W * 0.005, dark))

    return "".join(parts)


def _compose_emblem_old(spec: AssetSpec, rng: random.Random) -> str:
    """OLD emblem composer kept for reference. Not registered. Will be removed
    after _compose_emblem is validated."""
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    glow = p.glow()
    name = spec.canonical_name
    cx, cy = spec.width / 2, spec.height / 2
    simple = "_simple" in name

    parts: List[str] = []
    # Backdrop circle
    parts.append(_circle(cx, cy, min(spec.width, spec.height) * 0.46, dark, accent,
                         max(1.0, spec.width * 0.02)))
    if not simple:
        parts.append(_circle(cx, cy, min(spec.width, spec.height) * 0.38, body, dark, 1.0))

    # Faction-symbolic inner mark
    if "hostile_corp" in name:
        # Hex-and-spike
        import math
        pts = []
        for i in range(6):
            ang = math.radians(60 * i - 30)
            r = spec.width * 0.22
            pts.append((cx + r * math.cos(ang), cy + r * math.sin(ang)))
        parts.append(_polygon(pts, accent, dark, 1.0))
        parts.append(_rect(cx - spec.width * 0.02, cy - spec.height * 0.10,
                           spec.width * 0.04, spec.height * 0.20, highlight))
    elif "allied_resistance" in name:
        # Crossed wrench-and-rifle
        parts.append(_polyline(
            [(cx - spec.width * 0.18, cy - spec.height * 0.18),
             (cx + spec.width * 0.18, cy + spec.height * 0.18)],
            accent, max(2.0, spec.width * 0.04),
        ))
        parts.append(_polyline(
            [(cx - spec.width * 0.18, cy + spec.height * 0.18),
             (cx + spec.width * 0.18, cy - spec.height * 0.18)],
            accent, max(2.0, spec.width * 0.04),
        ))
    elif "marauder" in name:
        # Asymmetric tribal mark
        parts.append(_polygon(
            [(cx, cy - spec.height * 0.20),
             (cx + spec.width * 0.16, cy),
             (cx + spec.width * 0.06, cy + spec.height * 0.20),
             (cx - spec.width * 0.12, cy + spec.height * 0.06)],
            accent, dark, 0.5,
        ))
    elif "religious_order" in name:
        # Vertical with horizontal bar
        parts.append(_rect(cx - spec.width * 0.03, cy - spec.height * 0.22,
                           spec.width * 0.06, spec.height * 0.44, accent, dark, 0.5))
        parts.append(_rect(cx - spec.width * 0.18, cy - spec.height * 0.04,
                           spec.width * 0.36, spec.height * 0.08, accent, dark, 0.5))
    elif "scientist_order" in name:
        # Atom
        parts.append(_circle(cx, cy, spec.width * 0.05, glow))
        parts.append(_ellipse(cx, cy, spec.width * 0.22, spec.height * 0.10, "none",
                              accent, max(1.0, spec.width * 0.02)))
        parts.append(_ellipse(cx, cy, spec.width * 0.10, spec.height * 0.22, "none",
                              accent, max(1.0, spec.width * 0.02)))
    elif "mercenary_guild" in name:
        # Shield-and-bar
        parts.append(_polygon(
            [(cx - spec.width * 0.16, cy - spec.height * 0.18),
             (cx + spec.width * 0.16, cy - spec.height * 0.18),
             (cx + spec.width * 0.10, cy + spec.height * 0.20),
             (cx - spec.width * 0.10, cy + spec.height * 0.20)],
            accent, dark, 0.5,
        ))
        parts.append(_rect(cx - spec.width * 0.16, cy - spec.height * 0.06,
                           spec.width * 0.32, spec.height * 0.04, highlight))
    elif "pirates" in name:
        # Skull silhouette
        parts.append(_circle(cx, cy - spec.height * 0.04, spec.width * 0.16, highlight, dark, 0.5))
        parts.append(_circle(cx - spec.width * 0.06, cy - spec.height * 0.02,
                             spec.width * 0.03, dark))
        parts.append(_circle(cx + spec.width * 0.06, cy - spec.height * 0.02,
                             spec.width * 0.03, dark))
        parts.append(_rect(cx - spec.width * 0.04, cy + spec.height * 0.06,
                           spec.width * 0.08, spec.height * 0.04, dark))
    elif "drone_collective" in name:
        # Faceted hex
        import math
        pts = []
        for i in range(6):
            ang = math.radians(60 * i)
            r = spec.width * 0.20
            pts.append((cx + r * math.cos(ang), cy + r * math.sin(ang)))
        parts.append(_polygon(pts, accent, dark, 1.0))
        parts.append(_circle(cx, cy, spec.width * 0.04, glow))
    else:
        parts.append(_circle(cx, cy, spec.width * 0.08, accent))
    return "".join(parts)


def _compose_overlay(spec: AssetSpec, rng: random.Random) -> str:
    p = spec.palette
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    name = spec.canonical_name
    parts: List[str] = []

    # Frame
    parts.append(_rect(0, 0, spec.width, spec.height, "none", accent, max(2.0, spec.width * 0.01)))
    parts.append(_rect(spec.width * 0.005, spec.height * 0.005,
                       spec.width * 0.99, spec.height * 0.99,
                       "none", highlight, max(1.0, spec.width * 0.005)))
    if "logo_watermark_corner" in name:
        parts.append(_rect(spec.width * 0.02, spec.height * 0.02, spec.width * 0.10, spec.height * 0.06,
                           accent, dark, 1.0))
    elif "seed_watermark_topright" in name:
        parts.append(_rect(spec.width * 0.88, spec.height * 0.02, spec.width * 0.10, spec.height * 0.06,
                           highlight, dark, 1.0))
    elif "tick_watermark_bottom" in name:
        parts.append(_rect(spec.width * 0.45, spec.height * 0.92, spec.width * 0.10, spec.height * 0.06,
                           accent, dark, 1.0))
    elif "version_watermark_bottomleft" in name:
        parts.append(_rect(spec.width * 0.02, spec.height * 0.92, spec.width * 0.14, spec.height * 0.06,
                           highlight, dark, 1.0))
    elif "bp" in name or "m6" in name or "m7" in name or "m8" in name or "m9" in name or "m10" in name or "m11" in name:
        parts.append(_rect(spec.width * 0.02, spec.height * 0.02, spec.width * 0.20, spec.height * 0.08,
                           accent, dark, 1.0))
        parts.append(_rect(spec.width * 0.78, spec.height * 0.90, spec.width * 0.20, spec.height * 0.08,
                           highlight, dark, 1.0))
    else:
        parts.append(_rect(spec.width * 0.40, spec.height * 0.05, spec.width * 0.20, spec.height * 0.06,
                           accent, dark, 1.0))
    return "".join(parts)


# ─── Color utility ──────────────────────────────────────────────────────────


def _hex_to_rgb(hx: str) -> Tuple[int, int, int]:
    s = hx.lstrip("#")
    if len(s) == 3:
        s = "".join(c * 2 for c in s)
    return (int(s[0:2], 16), int(s[2:4], 16), int(s[4:6], 16))


def _rgb_to_hex(r: int, g: int, b: int) -> str:
    return f"#{r:02x}{g:02x}{b:02x}"


def _darken_hex(hx: str, factor: float) -> str:
    r, g, b = _hex_to_rgb(hx)
    f = max(0.0, 1.0 - factor)
    return _rgb_to_hex(int(r * f), int(g * f), int(b * f))


def _lighten_hex(hx: str, factor: float) -> str:
    r, g, b = _hex_to_rgb(hx)
    return _rgb_to_hex(
        min(255, int(r + (255 - r) * factor)),
        min(255, int(g + (255 - g) * factor)),
        min(255, int(b + (255 - b) * factor)),
    )


# ─── M11/M11A/M12 mega-expansion composers ──────────────────────────────────


def _compose_shell_ui(spec: AssetSpec, rng: random.Random) -> str:
    """M11A shell UI composer — vivid illustrated comic style.

    Dispatches by canonical_name prefix to produce title splashes, menu panels,
    buttons, tabs, sliders, toggles, cursors, hud widgets, comic panels, FRE
    cards, save-slot frames, and faction emblem cards.
    """
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    metal = p.metal()
    light = p.light()
    highlight = p.highlight()
    glow = p.glow()
    name = spec.canonical_name
    W = spec.width
    H = spec.height
    cx, cy = W / 2, H / 2
    parts: List[str] = []

    if name.startswith("title_splash_") or name.startswith("menu_bg_") or name.startswith("loading_"):
        # Cinematic wide-format background composition.
        # Sky band (top 30%) — gradient: light at top to primary at horizon
        for i in range(8):
            shade = _lighten_hex(body, 0.4 - i * 0.05)
            band_h = H * 0.30 / 8.0
            parts.append(_rect(0, i * band_h, W, band_h, shade))
        # Atmospheric distant particle dots (in sky band)
        for _ in range(40):
            x = rng.uniform(0, W)
            y = rng.uniform(0, H * 0.28)
            r = rng.uniform(0.5, 1.5)
            parts.append(_circle(x, y, r, highlight))
        # Horizon silhouette mid-band (dark layered)
        horizon_y = H * 0.30
        parts.append(_rect(0, horizon_y, W, H * 0.30, _darken_hex(body, 0.30)))
        # Distant silhouette mountains/structures
        peaks = []
        peak_count = 7
        peaks.append((0, horizon_y + H * 0.30))
        for i in range(peak_count):
            px = W * (i + 0.5) / peak_count
            py = horizon_y + rng.uniform(H * 0.05, H * 0.22)
            peaks.append((px, py))
        peaks.append((W, horizon_y + H * 0.30))
        peaks.append((W, horizon_y + H * 0.60))
        peaks.append((0, horizon_y + H * 0.60))
        parts.append(_polygon(peaks, dark, accent, 1.0))
        # Foreground silhouettes (accent layer)
        fg_y = H * 0.55
        fg_peaks = [(0, H)]
        for i in range(5):
            px = W * (i + 0.5) / 5
            py = fg_y + rng.uniform(H * 0.05, H * 0.30)
            fg_peaks.append((px, py))
        fg_peaks.append((W, H))
        parts.append(_polygon(fg_peaks, _darken_hex(dark, 0.10)))
        # Foreground actor / element silhouettes if title_splash
        if name.startswith("title_splash_"):
            # 2 actor silhouettes mid-foreground for scale
            for ax_off, scale in [(W * 0.35, 1.0), (W * 0.62, 0.85)]:
                ay = H * 0.78
                # Body
                parts.append(_rect(ax_off - W * 0.012 * scale, ay - H * 0.06 * scale,
                                   W * 0.024 * scale, H * 0.08 * scale, dark))
                # Head
                parts.append(_circle(ax_off, ay - H * 0.08 * scale,
                                     W * 0.012 * scale, dark))
                # Legs
                parts.append(_rect(ax_off - W * 0.012 * scale, ay + H * 0.02 * scale,
                                   W * 0.012 * scale, H * 0.06 * scale, dark))
                parts.append(_rect(ax_off, ay + H * 0.02 * scale,
                                   W * 0.012 * scale, H * 0.06 * scale, dark))
        # Reserved wordmark zone
        if name.startswith("title_splash_"):
            wm_y = H * 0.08
            wm_h = H * 0.16
            parts.append(_rect(W * 0.20, wm_y, W * 0.60, wm_h, "none", accent, 2.0))
        elif name.startswith("loading_"):
            # Loading progress bar reserved at bottom
            bar_y = H * 0.85
            parts.append(_rect(W * 0.15, bar_y, W * 0.70, H * 0.04, dark, accent, 1.5))
            # Loading-text zone above bar
            parts.append(_rect(W * 0.20, bar_y - H * 0.08, W * 0.60, H * 0.06,
                               "none", accent, 1.0))
        # Atmospheric haze accent at horizon line
        parts.append(_rect(0, horizon_y - 2, W, 4, accent))
        return "".join(parts)

    if name.startswith("menu_panel_") or name.startswith("fre_step_"):
        # 3-layer panel: ink-outline border, inner fill, divider lines + rivets.
        # Outer ink-outline border
        parts.append(_rect(W * 0.02, H * 0.02, W * 0.96, H * 0.96,
                           dark, accent, max(2.0, W * 0.012)))
        # Inner fill (gradient suggestion via 3 bands)
        for i in range(3):
            shade = _lighten_hex(body, i * 0.05)
            parts.append(_rect(W * 0.04, H * (0.04 + i * 0.30), W * 0.92, H * 0.30, shade))
        # Inner border
        parts.append(_rect(W * 0.05, H * 0.05, W * 0.90, H * 0.90,
                           "none", _darken_hex(body, 0.25), 1.0))
        # 4 corner rivets
        for cx_off, cy_off in [(0.06, 0.06), (0.94, 0.06), (0.06, 0.94), (0.94, 0.94)]:
            parts.append(_circle(W * cx_off, H * cy_off, W * 0.015, dark, metal, 1.0))
            parts.append(_circle(W * cx_off, H * cy_off, W * 0.008, metal))
        # 2 divider lines
        for div_y in [0.35, 0.70]:
            parts.append(_line(W * 0.10, H * div_y, W * 0.90, H * div_y,
                               _darken_hex(body, 0.30), 1.2))
        # FRE wizard variants add header zone marker
        if name.startswith("fre_step_"):
            parts.append(_rect(W * 0.10, H * 0.08, W * 0.80, H * 0.10,
                               accent, dark, 1.0))
            parts.append(_rect(W * 0.12, H * 0.10, W * 0.76, H * 0.06, highlight))
        return "".join(parts)

    if name.startswith("menu_button_") or name.startswith("btn_"):
        # Button with gradient fill + ink outline + inner glyph + corner notches.
        # Background gradient: primary at top → darker primary at bottom (8 bands)
        for i in range(8):
            shade = _lighten_hex(body, 0.20 - i * 0.04)
            parts.append(_rect(W * 0.06, H * (0.10 + i * 0.10), W * 0.88, H * 0.10, shade))
        # Ink outline
        parts.append(_rect(W * 0.06, H * 0.10, W * 0.88, H * 0.80,
                           "none", dark, max(1.5, W * 0.010)))
        # Corner notches (decorative)
        for cx_off, cy_off in [(0.06, 0.10), (0.94, 0.10), (0.06, 0.90), (0.94, 0.90)]:
            sign_x = 1 if cx_off < 0.5 else -1
            sign_y = 1 if cy_off < 0.5 else -1
            parts.append(_polygon([
                (W * cx_off, H * cy_off),
                (W * cx_off + sign_x * W * 0.04, H * cy_off),
                (W * cx_off, H * cy_off + sign_y * H * 0.04),
            ], dark))
        # Inner glyph by suffix
        if "play" in name:
            parts.append(_polygon([
                (cx - W * 0.10, cy - H * 0.16),
                (cx + W * 0.16, cy),
                (cx - W * 0.10, cy + H * 0.16),
            ], accent, dark, 1.0))
        elif "settings" in name or "gear" in name:
            import math as _m
            # Gear: 8 teeth around a central circle
            for i in range(8):
                ang = i * 0.7854
                x1 = cx + W * 0.18 * _m.cos(ang)
                y1 = cy + H * 0.18 * _m.sin(ang)
                parts.append(_rect(x1 - W * 0.025, y1 - H * 0.025, W * 0.05, H * 0.05, accent, dark, 0.5))
            parts.append(_circle(cx, cy, W * 0.13, dark, metal, 1.0))
            parts.append(_circle(cx, cy, W * 0.07, metal))
            parts.append(_circle(cx, cy, W * 0.04, dark))
        elif "save" in name:
            # Floppy disk
            parts.append(_rect(cx - W * 0.16, cy - H * 0.16, W * 0.32, H * 0.32, accent, dark, 1.0))
            parts.append(_rect(cx - W * 0.10, cy - H * 0.16, W * 0.20, H * 0.10, dark))
            parts.append(_rect(cx - W * 0.04, cy - H * 0.14, W * 0.02, H * 0.04, metal))
            parts.append(_rect(cx - W * 0.12, cy + H * 0.02, W * 0.24, H * 0.10, light))
        elif "quit" in name or "exit" in name or "_x_" in name or "close" in name:
            # X icon
            parts.append(_polygon([
                (cx - W * 0.14, cy - H * 0.10),
                (cx - W * 0.10, cy - H * 0.14),
                (cx + W * 0.14, cy + H * 0.10),
                (cx + W * 0.10, cy + H * 0.14),
            ], accent, dark, 0.8))
            parts.append(_polygon([
                (cx + W * 0.14, cy - H * 0.10),
                (cx + W * 0.10, cy - H * 0.14),
                (cx - W * 0.14, cy + H * 0.10),
                (cx - W * 0.10, cy + H * 0.14),
            ], accent, dark, 0.8))
        elif "continue" in name or "load" in name or "next" in name or "_chevron" in name:
            # Chevron arrow
            parts.append(_polygon([
                (cx - W * 0.08, cy - H * 0.14),
                (cx + W * 0.10, cy),
                (cx - W * 0.08, cy + H * 0.14),
                (cx - W * 0.04, cy + H * 0.14),
                (cx + W * 0.14, cy),
                (cx - W * 0.04, cy - H * 0.14),
            ], accent, dark, 0.8))
        elif "credits" in name or "scroll" in name:
            # Scroll
            parts.append(_rect(cx - W * 0.16, cy - H * 0.14, W * 0.32, H * 0.28, light, dark, 0.8))
            for i in range(3):
                parts.append(_line(cx - W * 0.10, cy - H * 0.06 + i * H * 0.06,
                                   cx + W * 0.10, cy - H * 0.06 + i * H * 0.06,
                                   dark, 1.0))
        elif "help" in name or "_q_" in name or "question" in name:
            # Question mark
            parts.append(_circle(cx, cy - H * 0.06, W * 0.10, "none", accent, max(2.0, W * 0.012)))
            parts.append(_rect(cx - W * 0.02, cy + H * 0.02, W * 0.04, H * 0.10, accent))
            parts.append(_circle(cx, cy + H * 0.16, W * 0.025, accent))
        elif "replay" in name or "rewind" in name:
            # Rewind double-triangle
            parts.append(_polygon([
                (cx - W * 0.02, cy - H * 0.12),
                (cx - W * 0.14, cy),
                (cx - W * 0.02, cy + H * 0.12),
            ], accent, dark, 0.8))
            parts.append(_polygon([
                (cx + W * 0.10, cy - H * 0.12),
                (cx - W * 0.02, cy),
                (cx + W * 0.10, cy + H * 0.12),
            ], accent, dark, 0.8))
        elif "photo" in name or "camera" in name:
            # Camera
            parts.append(_rect(cx - W * 0.16, cy - H * 0.10, W * 0.32, H * 0.20, dark, accent, 1.0))
            parts.append(_circle(cx, cy, W * 0.06, metal, dark, 0.8))
            parts.append(_circle(cx, cy, W * 0.03, accent))
            parts.append(_rect(cx + W * 0.10, cy - H * 0.12, W * 0.04, H * 0.02, accent))
        elif "mods" in name or "puzzle" in name:
            # Puzzle piece
            parts.append(_polygon([
                (cx - W * 0.10, cy - H * 0.10),
                (cx + W * 0.04, cy - H * 0.10),
                (cx + W * 0.04, cy - H * 0.04),
                (cx + W * 0.10, cy - H * 0.04),
                (cx + W * 0.10, cy + H * 0.10),
                (cx - W * 0.10, cy + H * 0.10),
            ], accent, dark, 0.8))
        elif "workshop" in name or "anvil" in name:
            # Anvil & hammer
            parts.append(_polygon([
                (cx - W * 0.16, cy - H * 0.02),
                (cx + W * 0.16, cy - H * 0.02),
                (cx + W * 0.10, cy + H * 0.10),
                (cx - W * 0.10, cy + H * 0.10),
            ], dark, metal, 1.0))
            parts.append(_rect(cx - W * 0.04, cy + H * 0.10, W * 0.08, H * 0.06, dark))
            # Hammer head
            parts.append(_rect(cx - W * 0.14, cy - H * 0.16, W * 0.12, H * 0.06, accent, dark, 0.8))
            parts.append(_rect(cx - W * 0.04, cy - H * 0.16, W * 0.02, H * 0.16, dark))
        else:
            # Generic centered glyph
            parts.append(_circle(cx, cy, W * 0.12, accent, dark, 1.0))
            parts.append(_circle(cx, cy, W * 0.06, highlight))
        return "".join(parts)

    if name.startswith("menu_tab_") or name.startswith("settings_tab_"):
        # Trapezoidal tab with accent fill + ink outline + glyph zone.
        parts.append(_polygon([
            (W * 0.10, H * 0.20),
            (W * 0.90, H * 0.20),
            (W * 0.84, H * 0.80),
            (W * 0.16, H * 0.80),
        ], accent, dark, max(2.0, W * 0.012)))
        parts.append(_polygon([
            (W * 0.14, H * 0.24),
            (W * 0.86, H * 0.24),
            (W * 0.80, H * 0.76),
            (W * 0.20, H * 0.76),
        ], _lighten_hex(accent, 0.15), dark, 0.8))
        # Tab icon glyph by suffix
        if "graphics" in name:
            # Monitor + slider
            parts.append(_rect(cx - W * 0.16, cy - H * 0.10, W * 0.32, H * 0.20, dark, metal, 1.0))
            parts.append(_rect(cx - W * 0.14, cy - H * 0.08, W * 0.28, H * 0.16, glow))
            parts.append(_rect(cx - W * 0.04, cy + H * 0.14, W * 0.08, H * 0.04, metal))
        elif "audio" in name or "speaker" in name:
            # Speaker + sound wave
            parts.append(_polygon([
                (cx - W * 0.10, cy - H * 0.08),
                (cx - W * 0.02, cy - H * 0.08),
                (cx + W * 0.06, cy - H * 0.16),
                (cx + W * 0.06, cy + H * 0.16),
                (cx - W * 0.02, cy + H * 0.08),
                (cx - W * 0.10, cy + H * 0.08),
            ], dark, metal, 1.0))
            for i in range(3):
                r = W * (0.10 + i * 0.04)
                parts.append(_circle(cx + W * 0.06, cy, r, "none", glow, 1.5))
        elif "controls" in name or "keyboard" in name:
            # Keyboard + mouse
            parts.append(_rect(cx - W * 0.18, cy - H * 0.04, W * 0.28, H * 0.12, dark, metal, 1.0))
            for col in range(5):
                for row in range(2):
                    parts.append(_rect(cx - W * 0.17 + col * W * 0.05,
                                       cy - H * 0.02 + row * H * 0.05,
                                       W * 0.04, H * 0.04, metal))
            parts.append(_circle(cx + W * 0.14, cy, W * 0.06, dark, accent, 0.8))
        elif "accessibility" in name or "eye" in name:
            # Eye glyph
            parts.append(_ellipse(cx, cy, W * 0.16, H * 0.10, light, dark, 1.0))
            parts.append(_circle(cx, cy, H * 0.06, dark))
            parts.append(_circle(cx, cy, H * 0.03, glow))
        elif "gameplay" in name or "dice" in name:
            # Dice
            parts.append(_rect(cx - W * 0.14, cy - H * 0.14, W * 0.28, H * 0.28, light, dark, 1.0))
            for off_x, off_y in [(-0.06, -0.06), (0.06, 0.06), (-0.06, 0.06), (0.06, -0.06)]:
                parts.append(_circle(cx + W * off_x, cy + H * off_y, W * 0.018, dark))
            parts.append(_circle(cx, cy, W * 0.018, dark))
        elif "language" in name or "globe" in name:
            # Globe
            parts.append(_circle(cx, cy, W * 0.14, glow, dark, 1.0))
            parts.append(_ellipse(cx, cy, W * 0.14, H * 0.05, "none", dark, 1.0))
            parts.append(_line(cx, cy - H * 0.14, cx, cy + H * 0.14, dark, 1.0))
            parts.append(_ellipse(cx, cy, W * 0.07, H * 0.14, "none", dark, 1.0))
        else:
            parts.append(_circle(cx, cy, W * 0.10, light, dark, 1.0))
            parts.append(_rect(cx - W * 0.02, cy - H * 0.10, W * 0.04, H * 0.20, dark))
        return "".join(parts)

    if name.startswith("menu_slider_"):
        # Slider: track + knob + tick marks
        track_y = H * 0.46
        track_h = H * 0.08
        parts.append(_rect(W * 0.10, track_y, W * 0.80, track_h, dark, accent, 1.0))
        # 5 tick marks
        for i in range(5):
            tx = W * (0.15 + i * 0.175)
            parts.append(_line(tx, track_y - 4, tx, track_y + track_h + 4, metal, 1.5))
        # Knob (positioned at 60% by default)
        knob_x = W * 0.60
        parts.append(_circle(knob_x, track_y + track_h / 2, H * 0.12, glow, dark, 1.5))
        parts.append(_circle(knob_x, track_y + track_h / 2, H * 0.06, accent))
        # Fill portion of track up to knob
        parts.append(_rect(W * 0.10, track_y, knob_x - W * 0.10, track_h, accent))
        return "".join(parts)

    if name.startswith("menu_toggle_"):
        # Switch: oval track + sliding knob + on/off indicator
        track_h = H * 0.30
        track_y = (H - track_h) / 2
        # Oval track approximation
        parts.append(_rect(W * 0.20, track_y, W * 0.60, track_h, dark, accent, 1.5))
        parts.append(_circle(W * 0.20, track_y + track_h / 2, track_h / 2, dark))
        parts.append(_circle(W * 0.80, track_y + track_h / 2, track_h / 2, dark))
        # Knob (on right = enabled)
        is_on = "_on" in name or "_enabled" in name or "_true" in name
        knob_cx = W * 0.74 if is_on else W * 0.26
        parts.append(_circle(knob_cx, track_y + track_h / 2, track_h * 0.50, glow, dark, 1.0))
        parts.append(_circle(knob_cx, track_y + track_h / 2, track_h * 0.30,
                             accent if is_on else metal))
        # Indicator dots
        if is_on:
            parts.append(_polyline([
                (W * 0.30, track_y + track_h * 0.50),
                (W * 0.38, track_y + track_h * 0.70),
                (W * 0.46, track_y + track_h * 0.30),
            ], glow, 2.5))
        else:
            parts.append(_polygon([
                (W * 0.62, track_y + track_h * 0.30),
                (W * 0.74, track_y + track_h * 0.70),
                (W * 0.78, track_y + track_h * 0.66),
                (W * 0.66, track_y + track_h * 0.26),
            ], "#cc4444"))
        return "".join(parts)

    if name.startswith("loading_bar_"):
        # 10-segment progress bar + outer frame + faction-colored fill gradient
        frame_y = H * 0.30
        frame_h = H * 0.40
        parts.append(_rect(W * 0.06, frame_y, W * 0.88, frame_h, dark, accent, max(2.0, W * 0.008)))
        seg_w = W * 0.084
        for i in range(10):
            seg_x = W * 0.08 + i * seg_w
            seg_fill = accent if i < 6 else _darken_hex(body, 0.30)
            parts.append(_rect(seg_x, frame_y + H * 0.05, seg_w * 0.92, frame_h - H * 0.10,
                               seg_fill, dark, 1.0))
            if i < 6:
                # Gradient highlight
                parts.append(_rect(seg_x, frame_y + H * 0.05, seg_w * 0.92, frame_h * 0.20,
                                   _lighten_hex(seg_fill, 0.20)))
        return "".join(parts)

    if name.startswith("cursor_"):
        # Reticle: crosshair + center dot + corner brackets + variant
        # 4 corner brackets
        for cx_off, cy_off, sign_x, sign_y in [
            (0.20, 0.20, 1, 1), (0.80, 0.20, -1, 1), (0.20, 0.80, 1, -1), (0.80, 0.80, -1, -1)
        ]:
            parts.append(_line(W * cx_off, H * cy_off,
                               W * (cx_off + sign_x * 0.08), H * cy_off, accent, 2.0))
            parts.append(_line(W * cx_off, H * cy_off,
                               W * cx_off, H * (cy_off + sign_y * 0.08), accent, 2.0))
        # Crosshair (lines from center out)
        parts.append(_line(cx, H * 0.30, cx, H * 0.45, accent, 1.5))
        parts.append(_line(cx, H * 0.55, cx, H * 0.70, accent, 1.5))
        parts.append(_line(W * 0.30, cy, W * 0.45, cy, accent, 1.5))
        parts.append(_line(W * 0.55, cy, W * 0.70, cy, accent, 1.5))
        # Center dot
        parts.append(_circle(cx, cy, W * 0.015, glow))
        # Variant: sniper has range pip
        if "sniper" in name:
            parts.append(_line(cx - W * 0.04, cy + H * 0.18, cx + W * 0.04, cy + H * 0.18, accent, 1.0))
            parts.append(_line(cx - W * 0.03, cy + H * 0.22, cx + W * 0.03, cy + H * 0.22, accent, 1.0))
        # Variant: weapon has firing-arc
        if "weapon" in name or "firing" in name:
            parts.append(_circle(cx, cy, W * 0.30, "none", _darken_hex(accent, 0.20), 1.0))
        return "".join(parts)

    if name.startswith("hud_widget_"):
        # HUD background frame: rectangle with corner clipping + ink outline.
        # Notched corners
        parts.append(_polygon([
            (W * 0.04, H * 0.10),
            (W * 0.10, H * 0.04),
            (W * 0.90, H * 0.04),
            (W * 0.96, H * 0.10),
            (W * 0.96, H * 0.90),
            (W * 0.90, H * 0.96),
            (W * 0.10, H * 0.96),
            (W * 0.04, H * 0.90),
        ], dark, accent, max(2.0, W * 0.010)))
        # Inner fill (gradient)
        for i in range(4):
            shade = _lighten_hex(body, 0.10 - i * 0.04)
            parts.append(_rect(W * 0.08, H * (0.10 + i * 0.20), W * 0.84, H * 0.20, shade))
        # Reserved status-text zone
        parts.append(_rect(W * 0.12, H * 0.20, W * 0.76, H * 0.60, "none",
                           _darken_hex(body, 0.20), 1.0))
        return "".join(parts)

    if name.startswith("comic_panel_") or "panel" in name:
        # Comic-book panel: thick black outline + inner color fill + ink scribble accents.
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92,
                           body, dark, max(3.0, W * 0.018)))
        # 2-3 internal panel lines (hand-drawn jitter)
        for div_y in [0.28, 0.50, 0.72]:
            jitter = [(W * 0.08, H * div_y),
                      (W * 0.30, H * (div_y + 0.01)),
                      (W * 0.55, H * (div_y - 0.005)),
                      (W * 0.92, H * div_y)]
            parts.append(_polyline(jitter, dark, 1.5))
        # Ink scribble accents (3-4 short slashes)
        for _ in range(8):
            x1 = rng.uniform(W * 0.10, W * 0.90)
            y1 = rng.uniform(H * 0.10, H * 0.90)
            x2 = x1 + rng.uniform(W * 0.02, W * 0.06)
            y2 = y1 + rng.uniform(H * 0.02, H * 0.06)
            parts.append(_line(x1, y1, x2, y2, dark, 1.2))
        return "".join(parts)

    if name.startswith("emblem_"):
        # Faction emblem card: shield + crest + motto banner.
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, body, dark, max(2.0, W * 0.012)))
        # Outer rim ornament
        for i in range(8):
            ang = i * 0.7854
            import math as _m
            x = cx + W * 0.42 * _m.cos(ang)
            y = cy + H * 0.42 * _m.sin(ang)
            parts.append(_circle(x, y, W * 0.01, accent))
        # Crest shield
        parts.append(_polygon([
            (cx - W * 0.24, cy - H * 0.30),
            (cx + W * 0.24, cy - H * 0.30),
            (cx + W * 0.20, cy + H * 0.10),
            (cx, cy + H * 0.32),
            (cx - W * 0.20, cy + H * 0.10),
        ], dark, accent, max(2.0, W * 0.012)))
        parts.append(_polygon([
            (cx - W * 0.20, cy - H * 0.26),
            (cx + W * 0.20, cy - H * 0.26),
            (cx + W * 0.18, cy + H * 0.08),
            (cx, cy + H * 0.28),
            (cx - W * 0.18, cy + H * 0.08),
        ], accent, dark, 1.0))
        # Central insignia
        parts.append(_circle(cx, cy - H * 0.04, W * 0.10, glow, dark, 1.0))
        parts.append(_circle(cx, cy - H * 0.04, W * 0.06, highlight))
        # Motto banner
        parts.append(_polygon([
            (cx - W * 0.30, cy + H * 0.34),
            (cx + W * 0.30, cy + H * 0.34),
            (cx + W * 0.26, cy + H * 0.40),
            (cx, cy + H * 0.36),
            (cx - W * 0.26, cy + H * 0.40),
        ], accent, dark, 1.0))
        return "".join(parts)

    if name.startswith("save_slot_"):
        # Save-slot frame: dashed or solid border + content zones
        is_empty = "empty" in name
        is_corrupted = "corrupted" in name
        border_color = "#c93030" if is_corrupted else (
            _darken_hex(body, 0.30) if is_empty else accent)
        # Frame
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, body, border_color,
                           max(2.5, W * 0.012)))
        if is_empty:
            # Dashed corner markers (simulated by 4 short segments)
            for dash_x, dash_y in [(0.04, 0.04), (0.96, 0.04), (0.04, 0.96), (0.96, 0.96)]:
                parts.append(_rect(W * (dash_x - 0.02), H * (dash_y - 0.02),
                                   W * 0.04, H * 0.04, "none", border_color, 1.5))
            # Placeholder text-region center
            parts.append(_rect(W * 0.30, H * 0.45, W * 0.40, H * 0.10, "none",
                               _darken_hex(body, 0.20), 1.0))
        else:
            # Mission-thumb zone
            parts.append(_rect(W * 0.08, H * 0.10, W * 0.84, H * 0.50, dark, border_color, 1.0))
            parts.append(_rect(W * 0.10, H * 0.12, W * 0.80, H * 0.46, _lighten_hex(body, 0.1)))
            # Metadata strip below
            parts.append(_rect(W * 0.08, H * 0.66, W * 0.84, H * 0.24, dark, border_color, 1.0))
            for i in range(3):
                parts.append(_line(W * 0.12, H * (0.72 + i * 0.06),
                                   W * 0.88, H * (0.72 + i * 0.06), accent, 0.8))
        if is_corrupted:
            # Warning glyph
            parts.append(_polygon([
                (cx, H * 0.20),
                (cx - W * 0.10, H * 0.40),
                (cx + W * 0.10, H * 0.40),
            ], "#c93030", dark, 1.5))
            parts.append(_rect(cx - W * 0.012, H * 0.26, W * 0.024, H * 0.08, dark))
            parts.append(_circle(cx, H * 0.37, W * 0.014, dark))
        return "".join(parts)

    # Generic fallback: panel with corner rivets + center accent
    parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, body, dark, max(2.0, W * 0.012)))
    parts.append(_rect(W * 0.08, H * 0.08, W * 0.84, H * 0.84, "none",
                       _darken_hex(body, 0.20), 1.0))
    for cx_off, cy_off in [(0.06, 0.06), (0.94, 0.06), (0.06, 0.94), (0.94, 0.94)]:
        parts.append(_circle(W * cx_off, H * cy_off, W * 0.015, dark, metal, 1.0))
    parts.append(_circle(cx, cy, W * 0.10, accent, dark, 1.0))
    parts.append(_circle(cx, cy, W * 0.06, highlight))
    return "".join(parts)


def _compose_banner(spec: AssetSpec, rng: random.Random) -> str:
    """M11 status banner composer: framed warning ribbon by severity.

    Suffix dispatch:
      *_critical → red-pulse frame with warning chevrons.
      *_warning  → amber frame, thinner outline.
      *_info     → blue/neutral frame, ribbon style.
      *_objective → green gradient frame with checkmark.
    """
    name = spec.canonical_name.lower()
    W = spec.width
    H = spec.height
    cx, cy = W / 2, H / 2
    p = spec.palette
    metal = p.metal()
    dark = p.dark()
    parts: List[str] = []

    if "critical" in name or "dying" in name or "lost" in name or "venting" in name or "now" in name:
        # Red-pulse critical banner.
        ink = "#220404"
        fill = "#380a0a"
        flash = "#FF3030"
        accent = "#FFAA22"
        # Outer thick red ink outline
        parts.append(_rect(W * 0.02, H * 0.02, W * 0.96, H * 0.96, fill, flash,
                           max(3.0, W * 0.016)))
        # Inner dark band
        parts.append(_rect(W * 0.06, H * 0.06, W * 0.88, H * 0.88, ink, flash, 1.5))
        # Inner gradient highlight
        parts.append(_rect(W * 0.08, H * 0.08, W * 0.84, H * 0.20, _lighten_hex(fill, 0.10)))
        # Warning chevrons left + right
        for sign_x, x_anchor in [(1, 0.04), (-1, 0.92)]:
            parts.append(_polygon([
                (W * x_anchor, H * 0.30),
                (W * (x_anchor + sign_x * 0.06), H * 0.50),
                (W * x_anchor, H * 0.70),
            ], flash, ink, 1.0))
        # [!!] zone center
        parts.append(_polygon([
            (cx - W * 0.10, cy - H * 0.18),
            (cx - W * 0.04, cy - H * 0.18),
            (cx - W * 0.06, cy + H * 0.06),
            (cx - W * 0.08, cy + H * 0.06),
        ], accent, ink, 1.0))
        parts.append(_circle(cx - W * 0.07, cy + H * 0.16, W * 0.020, accent, ink, 0.5))
        parts.append(_polygon([
            (cx + W * 0.04, cy - H * 0.18),
            (cx + W * 0.10, cy - H * 0.18),
            (cx + W * 0.08, cy + H * 0.06),
            (cx + W * 0.06, cy + H * 0.06),
        ], accent, ink, 1.0))
        parts.append(_circle(cx + W * 0.07, cy + H * 0.16, W * 0.020, accent, ink, 0.5))
        return "".join(parts)

    if "warning" in name or "low" in name or "jammed" in name or "overheat" in name or "mild" in name or "concussed" in name or "leaking" in name or "degraded" in name or "knocked" in name or "stimmed" in name or "unstable" in name:
        # Amber warning banner.
        ink = "#332200"
        fill = "#5a3a08"
        amber = "#FFAA22"
        accent = "#FFE068"
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, fill, amber,
                           max(2.0, W * 0.010)))
        parts.append(_rect(W * 0.08, H * 0.08, W * 0.84, H * 0.84, ink, amber, 1.0))
        # Gradient inner band
        parts.append(_rect(W * 0.10, H * 0.10, W * 0.80, H * 0.20, _lighten_hex(fill, 0.10)))
        # [!] zone center
        parts.append(_polygon([
            (cx - W * 0.04, cy - H * 0.18),
            (cx + W * 0.04, cy - H * 0.18),
            (cx + W * 0.02, cy + H * 0.06),
            (cx - W * 0.02, cy + H * 0.06),
        ], accent, ink, 1.0))
        parts.append(_circle(cx, cy + H * 0.16, W * 0.022, accent, ink, 0.5))
        # Triangle-warning frames left + right
        for x_off in [0.10, 0.86]:
            parts.append(_polygon([
                (W * x_off, H * 0.30),
                (W * (x_off + 0.04), H * 0.50),
                (W * x_off, H * 0.70),
            ], amber, ink, 0.8))
        return "".join(parts)

    if "info" in name or "reloading" in name:
        # Blue/neutral info ribbon.
        ink = "#001b33"
        fill = "#0d3766"
        blue = "#3a8cff"
        accent = "#aaccff"
        # Ribbon style: trapezoidal frame
        parts.append(_polygon([
            (W * 0.04, H * 0.10),
            (W * 0.96, H * 0.10),
            (W * 0.92, H * 0.90),
            (W * 0.08, H * 0.90),
        ], fill, blue, max(2.0, W * 0.010)))
        parts.append(_polygon([
            (W * 0.08, H * 0.14),
            (W * 0.92, H * 0.14),
            (W * 0.88, H * 0.86),
            (W * 0.12, H * 0.86),
        ], ink, blue, 1.0))
        # [*] glyph (star/dot center)
        parts.append(_circle(cx, cy, W * 0.08, blue, ink, 1.0))
        parts.append(_circle(cx, cy, W * 0.04, accent))
        import math as _m
        for i in range(5):
            ang = _m.radians(72 * i - 90)
            x1 = cx + W * 0.10 * _m.cos(ang)
            y1 = cy + H * 0.10 * _m.sin(ang)
            parts.append(_line(cx, cy, x1, y1, accent, 1.5))
        return "".join(parts)

    if "objective" in name or "complete" in name or "active" in name or "win" in name:
        # Green objective banner with checkmark.
        ink = "#003311"
        fill = "#0d4422"
        green = "#5bd078"
        accent = "#a8f0c0"
        # Gradient frame (4 bands)
        for i in range(4):
            shade = _lighten_hex(fill, 0.05 * (3 - i))
            parts.append(_rect(W * 0.04, H * (0.04 + i * 0.23), W * 0.92, H * 0.23, shade))
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, "none", green,
                           max(2.0, W * 0.010)))
        parts.append(_rect(W * 0.08, H * 0.08, W * 0.84, H * 0.84, "none", green, 1.0))
        # Checkmark center
        parts.append(_polyline([
            (cx - W * 0.14, cy),
            (cx - W * 0.04, cy + H * 0.12),
            (cx + W * 0.14, cy - H * 0.12),
        ], accent, max(3.0, W * 0.018)))
        return "".join(parts)

    # Fallback: neutral frame
    parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, _darken_hex(metal, 0.30),
                       metal, max(2.0, W * 0.010)))
    parts.append(_rect(W * 0.08, H * 0.08, W * 0.84, H * 0.84, dark, metal, 1.0))
    return "".join(parts)


def _compose_hud_widget(spec: AssetSpec, rng: random.Random) -> str:
    """M11 HUD widget composer for 12-focusable-node HUD."""
    name = spec.canonical_name.lower()
    W = spec.width
    H = spec.height
    cx, cy = W / 2, H / 2
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    metal = p.metal()
    light = p.light()
    highlight = p.highlight()
    glow = p.glow()
    parts: List[str] = []

    if name.startswith("silhouette_body_"):
        # Body outline with 4 zones colored by zone HP (head/torso/arms/legs).
        green = "#5bd078"
        yellow = "#dab438"
        red = "#c93030"
        # Pick zone states by seed for variation across origins
        zone_states = [green, yellow, green, red] if (spec.seed & 0xF) % 3 == 0 else [green, green, yellow, green]
        # Head zone
        parts.append(_circle(cx, H * 0.20, W * 0.10, zone_states[0], dark, 1.5))
        # Torso (rectangle)
        parts.append(_rect(cx - W * 0.14, H * 0.30, W * 0.28, H * 0.32, zone_states[1], dark, 1.5))
        # Arms (left + right polygons)
        parts.append(_polygon([
            (cx - W * 0.14, H * 0.30),
            (cx - W * 0.24, H * 0.36),
            (cx - W * 0.24, H * 0.58),
            (cx - W * 0.14, H * 0.54),
        ], zone_states[2], dark, 1.2))
        parts.append(_polygon([
            (cx + W * 0.14, H * 0.30),
            (cx + W * 0.24, H * 0.36),
            (cx + W * 0.24, H * 0.58),
            (cx + W * 0.14, H * 0.54),
        ], zone_states[2], dark, 1.2))
        # Legs (2 polygons)
        parts.append(_polygon([
            (cx - W * 0.14, H * 0.62),
            (cx - W * 0.04, H * 0.62),
            (cx - W * 0.06, H * 0.92),
            (cx - W * 0.16, H * 0.92),
        ], zone_states[3], dark, 1.2))
        parts.append(_polygon([
            (cx + W * 0.14, H * 0.62),
            (cx + W * 0.04, H * 0.62),
            (cx + W * 0.06, H * 0.92),
            (cx + W * 0.16, H * 0.92),
        ], zone_states[3], dark, 1.2))
        # Chassis-specific decorations
        if "chassis_light" in name or "chassis_heavy" in name:
            # Add plating seams + mech detail
            parts.append(_line(cx, H * 0.30, cx, H * 0.62, dark, 0.8))
            parts.append(_circle(cx, H * 0.45, W * 0.025, glow, dark, 0.5))
        if "robot" in name or "biomech" in name:
            # Single eye on head
            parts.append(_circle(cx, H * 0.20, W * 0.03, dark))
            parts.append(_circle(cx, H * 0.20, W * 0.015, glow))
        return "".join(parts)

    if name.startswith("module_strip_slot_"):
        # Module bay icon: rectangular slot + inner glyph + state tag
        parts.append(_rect(W * 0.06, H * 0.06, W * 0.88, H * 0.88, dark, accent,
                           max(2.0, W * 0.010)))
        parts.append(_rect(W * 0.10, H * 0.10, W * 0.80, H * 0.80, body, dark, 1.0))
        # Glyph by suffix
        if "weapon" in name:
            parts.append(_rect(W * 0.20, cy - H * 0.04, W * 0.50, H * 0.08, metal, dark, 1.0))
            parts.append(_rect(W * 0.60, cy + H * 0.04, W * 0.06, H * 0.10, dark))
        elif "jet" in name:
            parts.append(_polygon([
                (cx - W * 0.10, cy - H * 0.10),
                (cx + W * 0.10, cy - H * 0.10),
                (cx + W * 0.06, cy + H * 0.06),
                (cx - W * 0.06, cy + H * 0.06),
            ], metal, dark, 1.0))
            parts.append(_polygon([
                (cx - W * 0.06, cy + H * 0.06),
                (cx + W * 0.06, cy + H * 0.06),
                (cx, cy + H * 0.20),
            ], glow, accent, 0.5))
        elif "shield" in name:
            parts.append(_polygon([
                (cx, cy - H * 0.20),
                (cx - W * 0.16, cy - H * 0.10),
                (cx - W * 0.12, cy + H * 0.18),
                (cx, cy + H * 0.24),
                (cx + W * 0.12, cy + H * 0.18),
                (cx + W * 0.16, cy - H * 0.10),
            ], metal, dark, 1.0))
            parts.append(_circle(cx, cy, W * 0.06, accent))
        elif "sensor" in name:
            parts.append(_polygon([
                (cx, cy + H * 0.16),
                (cx - W * 0.18, cy - H * 0.10),
                (cx + W * 0.18, cy - H * 0.10),
            ], metal, dark, 1.0))
            parts.append(_circle(cx, cy + H * 0.04, W * 0.04, glow, accent, 0.5))
        elif "repair" in name:
            parts.append(_rect(cx - W * 0.04, cy - H * 0.16, W * 0.08, H * 0.32, metal, dark, 1.0))
            parts.append(_rect(cx - W * 0.16, cy - H * 0.04, W * 0.32, H * 0.08, metal, dark, 1.0))
        # State tag (top-right corner pip — defaults to OK green)
        parts.append(_circle(W * 0.82, H * 0.18, W * 0.04, "#5bd078", dark, 1.0))
        return "".join(parts)

    if name.startswith("ammo_counter_"):
        # Digital 3-digit display + reload progress arc
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, dark, accent,
                           max(2.0, W * 0.010)))
        # 3 segments for digits (faux 7-seg display)
        for i in range(3):
            digit_x = W * (0.10 + i * 0.22)
            digit_w = W * 0.18
            digit_h = H * 0.36
            digit_y = H * 0.18
            # Background slot
            parts.append(_rect(digit_x, digit_y, digit_w, digit_h, _darken_hex(dark, 0.20),
                               accent, 1.0))
            # 7-seg style: top + middle + bottom + 2 verticals
            seg_color = "#1aff7a"
            parts.append(_rect(digit_x + digit_w * 0.10, digit_y + digit_h * 0.05,
                               digit_w * 0.80, digit_h * 0.08, seg_color))
            parts.append(_rect(digit_x + digit_w * 0.10, digit_y + digit_h * 0.47,
                               digit_w * 0.80, digit_h * 0.08, seg_color))
            parts.append(_rect(digit_x + digit_w * 0.10, digit_y + digit_h * 0.87,
                               digit_w * 0.80, digit_h * 0.08, seg_color))
            parts.append(_rect(digit_x + digit_w * 0.05, digit_y + digit_h * 0.10,
                               digit_w * 0.10, digit_h * 0.40, seg_color))
            parts.append(_rect(digit_x + digit_w * 0.85, digit_y + digit_h * 0.10,
                               digit_w * 0.10, digit_h * 0.40, seg_color))
        # Reload progress arc (bottom)
        parts.append(_rect(W * 0.10, H * 0.65, W * 0.80, H * 0.10, _darken_hex(dark, 0.20),
                           accent, 1.0))
        parts.append(_rect(W * 0.12, H * 0.67, W * 0.50, H * 0.06, "#1aff7a"))
        return "".join(parts)

    if name.startswith("objective_banner_"):
        # Wide top banner: title zone + sub-line zone + countdown digits.
        parts.append(_rect(W * 0.02, H * 0.04, W * 0.96, H * 0.92, dark, accent,
                           max(2.0, W * 0.010)))
        # Title zone (top)
        parts.append(_rect(W * 0.05, H * 0.10, W * 0.90, H * 0.30, body, accent, 1.0))
        # Title text marker (3 bars)
        for i in range(3):
            parts.append(_rect(W * 0.10 + i * W * 0.28, H * 0.20, W * 0.20, H * 0.10,
                               _darken_hex(body, 0.20)))
        # Sub-line zone
        parts.append(_rect(W * 0.05, H * 0.46, W * 0.90, H * 0.16, _darken_hex(body, 0.10),
                           accent, 0.8))
        # Countdown digits (right side) if timer variant
        if "timer" in name or "countdown" in name:
            # Digital clock
            parts.append(_rect(W * 0.65, H * 0.66, W * 0.30, H * 0.24, _darken_hex(dark, 0.20),
                               accent, 1.0))
            for i in range(4):
                parts.append(_rect(W * 0.67 + i * W * 0.07, H * 0.70, W * 0.06, H * 0.16,
                                   "#1aff7a"))
        # Status indicator on left
        parts.append(_circle(W * 0.10, H * 0.78, W * 0.04, "#5bd078", dark, 1.0))
        return "".join(parts)

    if name.startswith("event_ticker_"):
        # Narrow bottom-right strip: icon + text-zone
        parts.append(_rect(W * 0.04, H * 0.20, W * 0.92, H * 0.60, dark, accent,
                           max(2.0, W * 0.010)))
        # Icon slot
        parts.append(_rect(W * 0.06, H * 0.22, W * 0.18, H * 0.56, body, accent, 0.8))
        # Icon glyph by suffix
        if "combat" in name:
            parts.append(_circle(W * 0.15, cy, W * 0.06, "#c93030", dark, 0.5))
        elif "systems" in name:
            parts.append(_rect(W * 0.10, cy - H * 0.10, W * 0.10, H * 0.20, accent, dark, 0.5))
        elif "squad" in name:
            parts.append(_circle(W * 0.12, cy, W * 0.04, accent, dark, 0.5))
            parts.append(_circle(W * 0.18, cy, W * 0.04, accent, dark, 0.5))
        # Text zone (right)
        for i in range(2):
            parts.append(_rect(W * 0.28, H * (0.30 + i * 0.20), W * 0.66, H * 0.10,
                               _darken_hex(body, 0.15)))
        return "".join(parts)

    if name.startswith("tool_validity_"):
        # Small tool icon + valid/refused indicator
        parts.append(_rect(W * 0.06, H * 0.06, W * 0.88, H * 0.88, dark, accent,
                           max(1.5, W * 0.008)))
        # Tool glyph
        if "drill" in name:
            parts.append(_rect(cx - W * 0.20, cy - H * 0.06, W * 0.30, H * 0.12, metal, dark, 1.0))
            for i in range(3):
                parts.append(_polygon([
                    (cx + W * (0.06 + i * 0.06), cy - H * 0.08),
                    (cx + W * (0.10 + i * 0.06), cy),
                    (cx + W * (0.06 + i * 0.06), cy + H * 0.08),
                ], dark, metal, 0.5))
        elif "grappler" in name:
            parts.append(_polygon([
                (cx - W * 0.10, cy),
                (cx + W * 0.10, cy - H * 0.10),
                (cx + W * 0.16, cy),
                (cx + W * 0.10, cy + H * 0.10),
            ], metal, dark, 1.0))
        elif "medkit" in name:
            parts.append(_rect(cx - W * 0.16, cy - H * 0.10, W * 0.32, H * 0.20, "#FFFFFF", dark, 1.0))
            parts.append(_rect(cx - W * 0.04, cy - H * 0.08, W * 0.08, H * 0.16, "#c93030"))
            parts.append(_rect(cx - W * 0.12, cy - H * 0.02, W * 0.24, H * 0.04, "#c93030"))
        elif "wrench" in name:
            parts.append(_polygon([
                (cx - W * 0.18, cy + H * 0.06),
                (cx + W * 0.06, cy - H * 0.18),
                (cx + W * 0.18, cy - H * 0.06),
                (cx - W * 0.06, cy + H * 0.18),
            ], metal, dark, 1.0))
        # Validity indicator (corner)
        parts.append(_polyline([
            (W * 0.66, H * 0.20),
            (W * 0.74, H * 0.30),
            (W * 0.86, H * 0.14),
        ], "#5bd078", max(2.0, W * 0.014)))
        return "".join(parts)

    if name.startswith("reactor_") and "gauge" in name:
        # Vertical pressure gauge: scale 0-100 with red zone above 80
        parts.append(_rect(W * 0.30, H * 0.10, W * 0.40, H * 0.80, dark, accent,
                           max(2.0, W * 0.010)))
        # Inner scale background
        parts.append(_rect(W * 0.34, H * 0.14, W * 0.32, H * 0.72, _darken_hex(dark, 0.20)))
        # Green safe zone (bottom 60%)
        parts.append(_rect(W * 0.36, H * (0.14 + 0.72 * 0.40), W * 0.28, H * 0.72 * 0.40, "#3aa256"))
        # Yellow caution (20%)
        parts.append(_rect(W * 0.36, H * (0.14 + 0.72 * 0.20), W * 0.28, H * 0.72 * 0.20, "#dab438"))
        # Red danger (top 20%)
        parts.append(_rect(W * 0.36, H * 0.14, W * 0.28, H * 0.72 * 0.20, "#c93030"))
        # Tick marks
        for i in range(11):
            ty = H * (0.14 + 0.72 * i / 10)
            tw = W * 0.04 if i % 5 == 0 else W * 0.02
            parts.append(_rect(W * 0.30 - tw, ty - 1, tw, 2, accent))
            parts.append(_rect(W * 0.70, ty - 1, tw, 2, accent))
        # Current-level needle (around 70%)
        needle_y = H * (0.14 + 0.72 * (0.30 if "temp" in name else 0.40))
        parts.append(_polygon([
            (W * 0.30, needle_y),
            (W * 0.34, needle_y - 4),
            (W * 0.34, needle_y + 4),
        ], "#FFE068", dark, 1.0))
        parts.append(_polygon([
            (W * 0.70, needle_y),
            (W * 0.66, needle_y - 4),
            (W * 0.66, needle_y + 4),
        ], "#FFE068", dark, 1.0))
        return "".join(parts)

    if name.startswith("triage_window_"):
        # Red/green box frame + TTD counter + 3-line breakdown zone
        is_red = "red" in name
        box_color = "#c93030" if is_red else "#5bd078"
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, dark, box_color,
                           max(2.5, W * 0.012)))
        # TTD counter (large digital)
        parts.append(_rect(W * 0.10, H * 0.10, W * 0.80, H * 0.30, _darken_hex(dark, 0.30),
                           box_color, 1.0))
        for i in range(4):
            parts.append(_rect(W * 0.14 + i * W * 0.18, H * 0.14, W * 0.14, H * 0.22, box_color))
        # 3-line breakdown zone
        for i in range(3):
            parts.append(_rect(W * 0.10, H * (0.46 + i * 0.13), W * 0.80, H * 0.10,
                               body, _darken_hex(box_color, 0.20), 0.8))
        return "".join(parts)

    if name.startswith("squad_strip_row_"):
        # Per-bot row: portrait circle + HP bar + role badge + priority icon
        parts.append(_rect(W * 0.02, H * 0.10, W * 0.96, H * 0.80, dark, accent,
                           max(2.0, W * 0.008)))
        # Portrait circle (left)
        parts.append(_circle(W * 0.10, cy, H * 0.32, body, accent, 1.5))
        parts.append(_circle(W * 0.10, cy, H * 0.20, _lighten_hex(body, 0.15)))
        parts.append(_circle(W * 0.10, cy, H * 0.10, accent))
        # HP bar (center)
        parts.append(_rect(W * 0.24, H * 0.30, W * 0.50, H * 0.20, _darken_hex(dark, 0.20),
                           accent, 1.0))
        parts.append(_rect(W * 0.25, H * 0.32, W * 0.36, H * 0.16, "#5bd078"))
        # Role badge by suffix
        role_colors = {
            "medic": "#FFFFFF", "engineer": "#dab438", "rifleman": "#3a8cff",
            "sniper": "#aa44ff", "heavy": "#c93030",
        }
        role_color = "#888888"
        for k, v in role_colors.items():
            if k in name:
                role_color = v
                break
        parts.append(_rect(W * 0.76, H * 0.30, W * 0.10, H * 0.40, role_color, dark, 1.0))
        # Priority icon (right)
        parts.append(_polygon([
            (W * 0.90, H * 0.30),
            (W * 0.96, H * 0.50),
            (W * 0.90, H * 0.70),
            (W * 0.84, H * 0.50),
        ], accent, dark, 0.8))
        return "".join(parts)

    if name.startswith("chatter_caption_"):
        # 2-line caption strip with role-colored text zone
        role_colors = {
            "combat": "#c93030", "medical": "#FFFFFF",
            "engineering": "#dab438", "tactical": "#3a8cff",
        }
        role_color = "#888888"
        for k, v in role_colors.items():
            if k in name:
                role_color = v
                break
        parts.append(_rect(W * 0.02, H * 0.10, W * 0.96, H * 0.80, dark, role_color,
                           max(2.0, W * 0.008)))
        # 2 text lines
        for i in range(2):
            parts.append(_rect(W * 0.06, H * (0.20 + i * 0.32), W * 0.88, H * 0.16,
                               _darken_hex(dark, 0.20), role_color, 0.5))
            # Text bars
            for j in range(4):
                parts.append(_rect(W * 0.10 + j * W * 0.20, H * (0.22 + i * 0.32),
                                   W * 0.16, H * 0.12, _lighten_hex(role_color, 0.30)))
        # Speaker color tag (left)
        parts.append(_rect(W * 0.02, H * 0.10, W * 0.03, H * 0.80, role_color))
        return "".join(parts)

    if name.startswith("minimap_frame_"):
        if "round" in name:
            parts.append(_circle(cx, cy, W * 0.44, dark, accent, max(2.0, W * 0.012)))
            parts.append(_circle(cx, cy, W * 0.40, body, dark, 1.0))
            # Compass markers N/S/E/W
            for ang_i, mark in enumerate(["N", "E", "S", "W"]):
                import math as _m
                ang = _m.radians(ang_i * 90 - 90)
                mx = cx + W * 0.42 * _m.cos(ang)
                my = cy + H * 0.42 * _m.sin(ang)
                parts.append(_circle(mx, my, W * 0.02, accent, dark, 0.5))
        else:
            parts.append(_rect(W * 0.06, H * 0.06, W * 0.88, H * 0.88, dark, accent,
                               max(2.0, W * 0.012)))
            parts.append(_rect(W * 0.10, H * 0.10, W * 0.80, H * 0.80, body, dark, 1.0))
        # Player center dot
        parts.append(_circle(cx, cy, W * 0.025, "#1aff7a", dark, 0.5))
        return "".join(parts)

    if name.startswith("compass_strip"):
        # Horizontal compass strip
        parts.append(_rect(W * 0.04, H * 0.30, W * 0.92, H * 0.40, dark, accent,
                           max(2.0, W * 0.010)))
        for i in range(13):
            tx = W * (0.04 + i * 0.077)
            th = H * 0.20 if i % 3 == 0 else H * 0.10
            parts.append(_rect(tx, cy - th / 2, 1.5, th, accent))
        # Center marker
        parts.append(_polygon([
            (cx, H * 0.20),
            (cx - W * 0.02, H * 0.10),
            (cx + W * 0.02, H * 0.10),
        ], "#FFE068", dark, 0.5))
        return "".join(parts)

    if name.startswith("stance_indicator_"):
        # Stance icon
        parts.append(_rect(W * 0.06, H * 0.06, W * 0.88, H * 0.88, dark, accent,
                           max(2.0, W * 0.010)))
        if "standing" in name:
            parts.append(_circle(cx, H * 0.25, W * 0.06, body, dark, 1.0))
            parts.append(_rect(cx - W * 0.04, H * 0.32, W * 0.08, H * 0.30, body, dark, 1.0))
            parts.append(_rect(cx - W * 0.06, H * 0.62, W * 0.04, H * 0.24, body, dark, 1.0))
            parts.append(_rect(cx + W * 0.02, H * 0.62, W * 0.04, H * 0.24, body, dark, 1.0))
        elif "crouching" in name:
            parts.append(_circle(cx, H * 0.30, W * 0.06, body, dark, 1.0))
            parts.append(_rect(cx - W * 0.04, H * 0.36, W * 0.08, H * 0.20, body, dark, 1.0))
            parts.append(_polygon([
                (cx - W * 0.06, H * 0.56),
                (cx, H * 0.56),
                (cx + W * 0.10, H * 0.80),
                (cx + W * 0.06, H * 0.84),
            ], body, dark, 1.0))
        elif "prone" in name:
            parts.append(_circle(W * 0.25, cy, W * 0.06, body, dark, 1.0))
            parts.append(_rect(W * 0.30, cy - H * 0.04, W * 0.40, H * 0.08, body, dark, 1.0))
            parts.append(_rect(W * 0.70, cy - H * 0.05, W * 0.12, H * 0.04, body, dark, 1.0))
        return "".join(parts)

    if name.startswith("stamina_bar_") or "bar" in name and not name.startswith("loading_bar"):
        # Segmented bar
        parts.append(_rect(W * 0.04, H * 0.30, W * 0.92, H * 0.40, dark, accent,
                           max(2.0, W * 0.010)))
        fill_count = 0 if "empty" in name else (10 if "full" in name else 5)
        for i in range(10):
            seg_x = W * 0.06 + i * W * 0.088
            fill_c = accent if i < fill_count else _darken_hex(dark, 0.15)
            parts.append(_rect(seg_x, H * 0.35, W * 0.08, H * 0.30, fill_c, dark, 0.5))
        return "".join(parts)

    if "gauge" in name and (name.startswith("heat_") or name.startswith("oxygen_") or name.startswith("radiation_")):
        # Generic gauge: similar to reactor but with custom colors
        is_hot = "hot" in name
        is_low = "low" in name
        parts.append(_rect(W * 0.30, H * 0.10, W * 0.40, H * 0.80, dark, accent,
                           max(2.0, W * 0.010)))
        parts.append(_rect(W * 0.34, H * 0.14, W * 0.32, H * 0.72, _darken_hex(dark, 0.20)))
        if name.startswith("heat_"):
            band1, band2, band3 = "#3a8cff", "#dab438", "#c93030"
        elif name.startswith("oxygen_"):
            band1, band2, band3 = "#c93030", "#dab438", "#3a8cff"
        else:  # radiation
            band1, band2, band3 = "#5bd078", "#dab438", "#aa44ff"
        # Bottom band
        parts.append(_rect(W * 0.36, H * (0.14 + 0.72 * 0.66), W * 0.28, H * 0.72 * 0.34, band1))
        parts.append(_rect(W * 0.36, H * (0.14 + 0.72 * 0.33), W * 0.28, H * 0.72 * 0.33, band2))
        parts.append(_rect(W * 0.36, H * 0.14, W * 0.28, H * 0.72 * 0.33, band3))
        # Needle position based on state
        if is_hot:
            needle_frac = 0.15
        elif is_low:
            needle_frac = 0.85
        else:
            needle_frac = 0.50
        needle_y = H * (0.14 + 0.72 * needle_frac)
        parts.append(_polygon([
            (W * 0.30, needle_y),
            (W * 0.34, needle_y - 4),
            (W * 0.34, needle_y + 4),
        ], "#FFE068", dark, 1.0))
        return "".join(parts)

    # Fallback: generic HUD frame
    parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, dark, accent,
                       max(2.0, W * 0.010)))
    parts.append(_rect(W * 0.08, H * 0.08, W * 0.84, H * 0.84, body, dark, 1.0))
    parts.append(_circle(cx, cy, W * 0.10, accent))
    return "".join(parts)


def _compose_vfx_decal(spec: AssetSpec, rng: random.Random) -> str:
    """M12 VFX decal composer: ground/surface marks like bullet holes, blood, scorch."""
    name = spec.canonical_name.lower()
    W = spec.width
    H = spec.height
    cx, cy = W / 2, H / 2
    parts: List[str] = []
    import math as _m

    if "bullet_hole_concrete" in name:
        # Concentric grey-black circles + radial cracks
        parts.append(_circle(cx, cy, W * 0.18, "#1a1a1a", "#888888", 1.0))
        parts.append(_circle(cx, cy, W * 0.10, "#000000", "#444444", 0.8))
        parts.append(_circle(cx, cy, W * 0.05, "#000000"))
        for i in range(8):
            ang = i * 0.7854
            x1 = cx + W * 0.06 * _m.cos(ang)
            y1 = cy + H * 0.06 * _m.sin(ang)
            x2 = cx + W * 0.30 * _m.cos(ang)
            y2 = cy + H * 0.30 * _m.sin(ang)
            parts.append(_line(x1, y1, x2, y2, "#444444", 1.5))
        # Dust spray
        for _ in range(12):
            ang = rng.uniform(0, 6.28)
            r = rng.uniform(W * 0.20, W * 0.36)
            parts.append(_circle(cx + r * _m.cos(ang), cy + r * _m.sin(ang),
                                 rng.uniform(0.5, 1.5), "#888888"))
        return "".join(parts)

    if "bullet_hole_metal" in name:
        # Ringed pucker with paint scrape rays
        parts.append(_circle(cx, cy, W * 0.16, "#1a1a1a", "#888888", 1.5))
        parts.append(_circle(cx, cy, W * 0.10, "#000000", "#cccccc", 1.0))
        parts.append(_circle(cx, cy, W * 0.04, "#000000"))
        # Paint scrape rays (longer than concrete cracks, brighter)
        for i in range(6):
            ang = i * 1.047
            x1 = cx + W * 0.08 * _m.cos(ang)
            y1 = cy + H * 0.08 * _m.sin(ang)
            x2 = cx + W * 0.30 * _m.cos(ang)
            y2 = cy + H * 0.30 * _m.sin(ang)
            parts.append(_line(x1, y1, x2, y2, "#aaaaaa", 1.5))
            parts.append(_line(x1, y1, x2, y2, "#ffffff", 0.5))
        return "".join(parts)

    if "bullet_hole_wood" in name:
        # Splinter starburst
        parts.append(_circle(cx, cy, W * 0.10, "#000000", "#553311", 1.0))
        parts.append(_circle(cx, cy, W * 0.05, "#000000"))
        for i in range(12):
            ang = i * 0.5236
            x1 = cx + W * 0.06 * _m.cos(ang)
            y1 = cy + H * 0.06 * _m.sin(ang)
            x2 = cx + (W * rng.uniform(0.18, 0.34)) * _m.cos(ang)
            y2 = cy + (H * rng.uniform(0.18, 0.34)) * _m.sin(ang)
            parts.append(_polygon([
                (x1, y1),
                (x2 + 1.5 * _m.cos(ang + 1.57), y2 + 1.5 * _m.sin(ang + 1.57)),
                (x2 - 1.5 * _m.cos(ang + 1.57), y2 - 1.5 * _m.sin(ang + 1.57)),
            ], "#886633", "#553311", 0.5))
        return "".join(parts)

    if "bullet_hole_glass" in name:
        # 8 angular crystal shards radiating
        parts.append(_circle(cx, cy, W * 0.06, "#000000", "#aaccee", 1.0))
        for i in range(8):
            ang = i * 0.7854
            x1 = cx + W * 0.04 * _m.cos(ang)
            y1 = cy + H * 0.04 * _m.sin(ang)
            x2 = cx + W * 0.30 * _m.cos(ang)
            y2 = cy + H * 0.30 * _m.sin(ang)
            x3 = cx + W * 0.28 * _m.cos(ang + 0.20)
            y3 = cy + H * 0.28 * _m.sin(ang + 0.20)
            parts.append(_polygon([(x1, y1), (x2, y2), (x3, y3)], "#88aacc", "#5588aa", 0.8))
        return "".join(parts)

    if "scorch" in name:
        # Blackened irregular blob with charred edge fade
        size = {"small": 0.20, "medium": 0.30, "large": 0.40}.get(
            next((k for k in ["small", "medium", "large"] if k in name), "medium"), 0.30)
        # Outer charred fade
        pts = []
        for i in range(12):
            ang = i * 0.5236
            r = W * size * rng.uniform(0.8, 1.1)
            pts.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts, "#1a1a1a", "#553333", 1.0))
        # Inner darker core
        pts2 = []
        for i in range(10):
            ang = i * 0.6283
            r = W * size * 0.6 * rng.uniform(0.7, 1.1)
            pts2.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts2, "#000000"))
        # Ash specks
        for _ in range(20):
            x = cx + rng.uniform(-W * size, W * size)
            y = cy + rng.uniform(-H * size, H * size)
            parts.append(_circle(x, y, rng.uniform(0.3, 1.0), "#444444"))
        return "".join(parts)

    if "blood_pool" in name:
        # Red blob with shape variant
        size = {"small": 0.18, "medium": 0.28, "large": 0.38}.get(
            next((k for k in ["small", "medium", "large"] if k in name), "small"), 0.20)
        # Outer pool (irregular)
        pts = []
        for i in range(14):
            ang = i * 0.4488
            r = W * size * rng.uniform(0.7, 1.2)
            pts.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts, "#5a0810", "#3a0408", 1.0))
        # Inner darker core
        pts2 = []
        for i in range(10):
            ang = i * 0.6283
            r = W * size * 0.65 * rng.uniform(0.7, 1.1)
            pts2.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts2, "#7a0a14"))
        # Surface highlights (wet look)
        for _ in range(5):
            sx = cx + rng.uniform(-W * size * 0.6, W * size * 0.6)
            sy = cy + rng.uniform(-H * size * 0.6, H * size * 0.6)
            parts.append(_ellipse(sx, sy, rng.uniform(1.5, 3), rng.uniform(0.8, 1.5), "#a52a2a"))
        return "".join(parts)

    if "blood_splatter" in name:
        # Radial droplet pattern
        size = 0.40 if "large" in name else 0.25
        for _ in range(28):
            ang = rng.uniform(0, 6.28)
            r = rng.uniform(W * 0.05, W * size)
            x = cx + r * _m.cos(ang)
            y = cy + r * _m.sin(ang)
            droplet_r = rng.uniform(0.6, 2.5)
            parts.append(_circle(x, y, droplet_r, "#7a0a14", "#5a0408", 0.3))
        # Central splat
        parts.append(_circle(cx, cy, W * 0.06, "#5a0810"))
        return "".join(parts)

    if "blood_drag_trail" in name:
        # Drag trail across decal
        for i in range(8):
            tx = W * (0.10 + i * 0.10)
            ty = H * 0.50 + _m.sin(i * 0.5) * H * 0.08
            parts.append(_ellipse(tx, ty, W * 0.05, H * 0.08, "#5a0810", "#3a0408", 0.5))
            parts.append(_ellipse(tx, ty, W * 0.03, H * 0.05, "#7a0a14"))
        # Endpoint pool
        parts.append(_circle(W * 0.85, H * 0.50, W * 0.10, "#5a0810", "#3a0408", 1.0))
        return "".join(parts)

    if "oil_pool" in name:
        # Dark gradient pool with rainbow sheen edge
        size = 0.30 if "large" in name else 0.20
        pts = []
        for i in range(14):
            ang = i * 0.4488
            r = W * size * rng.uniform(0.8, 1.1)
            pts.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts, "#1a1a22", "#0d0d11", 1.0))
        # Inner darker
        pts2 = []
        for i in range(10):
            ang = i * 0.6283
            r = W * size * 0.6
            pts2.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts2, "#000000"))
        # Rainbow sheen (3 edge arcs in different colors)
        for color in ["#aa44ff", "#44aaff", "#44ffaa"]:
            for _ in range(4):
                ang = rng.uniform(0, 6.28)
                r = W * size * 0.85
                x = cx + r * _m.cos(ang)
                y = cy + r * _m.sin(ang)
                parts.append(_circle(x, y, rng.uniform(1.5, 3), color))
        return "".join(parts)

    if "acid_pool" in name:
        # Green-yellow pool with bubble dots
        size = 0.30 if "large" in name else 0.20
        pts = []
        for i in range(14):
            ang = i * 0.4488
            r = W * size * rng.uniform(0.8, 1.2)
            pts.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts, "#aacc11", "#668800", 1.0))
        # Inner brighter
        parts.append(_circle(cx, cy, W * size * 0.6, "#ccff22"))
        # Bubbles
        for _ in range(12):
            bx = cx + rng.uniform(-W * size * 0.7, W * size * 0.7)
            by = cy + rng.uniform(-H * size * 0.7, H * size * 0.7)
            br = rng.uniform(1.5, 4)
            parts.append(_circle(bx, by, br, "#FFFFAA", "#88AA22", 0.5))
        return "".join(parts)

    if "frost" in name:
        # White crystalline radial pattern
        size = 0.36 if "large" in name else 0.24
        # Central frost core
        parts.append(_circle(cx, cy, W * size * 0.4, "#ddeeff"))
        # Crystalline spikes
        for i in range(12):
            ang = i * 0.5236
            x1 = cx + W * size * 0.3 * _m.cos(ang)
            y1 = cy + H * size * 0.3 * _m.sin(ang)
            x2 = cx + W * size * _m.cos(ang)
            y2 = cy + H * size * _m.sin(ang)
            x3 = cx + W * size * 0.4 * _m.cos(ang + 0.15)
            y3 = cy + H * size * 0.4 * _m.sin(ang + 0.15)
            x4 = cx + W * size * 0.4 * _m.cos(ang - 0.15)
            y4 = cy + H * size * 0.4 * _m.sin(ang - 0.15)
            parts.append(_polygon([(x1, y1), (x3, y3), (x2, y2), (x4, y4)],
                                  "#aaddff", "#7799bb", 0.5))
        # Sparkle dots
        for _ in range(10):
            sx = cx + rng.uniform(-W * size, W * size)
            sy = cy + rng.uniform(-H * size, H * size)
            parts.append(_circle(sx, sy, rng.uniform(0.5, 1.5), "#FFFFFF"))
        return "".join(parts)

    if "lava" in name:
        # Orange-red glowing blob with darker crusted edge
        size = 0.32 if "large" in name else 0.22
        # Outer crust (dark)
        pts = []
        for i in range(14):
            ang = i * 0.4488
            r = W * size * rng.uniform(0.85, 1.15)
            pts.append((cx + r * _m.cos(ang), cy + r * _m.sin(ang)))
        parts.append(_polygon(pts, "#332211", "#1a1108", 1.0))
        # Mid layer
        parts.append(_circle(cx, cy, W * size * 0.7, "#aa4422"))
        # Bright hot core
        parts.append(_circle(cx, cy, W * size * 0.4, "#FFAA22"))
        parts.append(_circle(cx, cy, W * size * 0.2, "#FFFFAA"))
        # Glowing cracks
        for i in range(4):
            ang = rng.uniform(0, 6.28)
            x1 = cx + W * size * 0.3 * _m.cos(ang)
            y1 = cy + H * size * 0.3 * _m.sin(ang)
            x2 = cx + W * size * 0.9 * _m.cos(ang)
            y2 = cy + H * size * 0.9 * _m.sin(ang)
            parts.append(_line(x1, y1, x2, y2, "#FFAA22", 2.0))
            parts.append(_line(x1, y1, x2, y2, "#FFFFAA", 0.8))
        return "".join(parts)

    if "crater" in name:
        # Ringed depression with radial cracks
        size = {"small": 0.22, "medium": 0.32, "large": 0.42}.get(
            next((k for k in ["small", "medium", "large"] if k in name), "medium"), 0.30)
        # Outer ring (light rim)
        parts.append(_circle(cx, cy, W * size, "#666666", "#444444", 1.0))
        # Mid ring (darker)
        parts.append(_circle(cx, cy, W * size * 0.75, "#333333"))
        # Inner pit (darkest)
        parts.append(_circle(cx, cy, W * size * 0.45, "#000000"))
        # Radial cracks
        for i in range(8):
            ang = i * 0.7854
            x1 = cx + W * size * 0.5 * _m.cos(ang)
            y1 = cy + H * size * 0.5 * _m.sin(ang)
            x2 = cx + W * (size + 0.08) * _m.cos(ang)
            y2 = cy + H * (size + 0.08) * _m.sin(ang)
            parts.append(_line(x1, y1, x2, y2, "#222222", 1.5))
        # Debris specks
        for _ in range(20):
            ang = rng.uniform(0, 6.28)
            r = W * size * rng.uniform(1.0, 1.4)
            parts.append(_circle(cx + r * _m.cos(ang), cy + r * _m.sin(ang),
                                 rng.uniform(0.4, 1.3), "#555555"))
        return "".join(parts)

    if "footprint" in name:
        # Boot/claw/track imprint
        if "boot" in name:
            # Boot shape: heel + sole + toe
            parts.append(_ellipse(cx, cy - H * 0.10, W * 0.10, H * 0.06, "#332211", "#1a0d04", 1.0))
            parts.append(_ellipse(cx, cy + H * 0.08, W * 0.14, H * 0.10, "#332211", "#1a0d04", 1.0))
            # Boot tread (5 dots on sole)
            for i in range(5):
                parts.append(_circle(cx + (i - 2) * W * 0.04, cy + H * 0.08, W * 0.012, "#000000"))
        elif "claw" in name:
            # Claw imprint: 4 toe pads + central pad
            parts.append(_ellipse(cx, cy + H * 0.06, W * 0.10, H * 0.06, "#332211", "#1a0d04", 1.0))
            for i in range(4):
                ang = -1.0 + i * 0.5
                px = cx + W * 0.14 * _m.sin(ang)
                py = cy - H * 0.12 - 0.05 * abs(_m.cos(ang)) * H
                parts.append(_ellipse(px, py, W * 0.03, H * 0.05, "#332211", "#1a0d04", 1.0))
                # Claw point
                parts.append(_polygon([
                    (px, py - H * 0.05),
                    (px - W * 0.02, py - H * 0.10),
                    (px + W * 0.02, py - H * 0.10),
                ], "#000000"))
        elif "track" in name:
            # Tank track: parallel rectangles
            for i in range(6):
                ty = cy - H * 0.18 + i * H * 0.07
                parts.append(_rect(cx - W * 0.16, ty, W * 0.32, H * 0.04, "#332211", "#1a0d04", 1.0))
                # Tread detail
                for j in range(5):
                    parts.append(_rect(cx - W * 0.14 + j * W * 0.06, ty + H * 0.005,
                                       W * 0.04, H * 0.03, "#000000"))
        elif "mech" in name:
            # Big stomp imprint: ringed square depression
            parts.append(_rect(cx - W * 0.18, cy - H * 0.18, W * 0.36, H * 0.36,
                               "#332211", "#1a0d04", 1.5))
            parts.append(_rect(cx - W * 0.14, cy - H * 0.14, W * 0.28, H * 0.28,
                               "#000000", "#1a0d04", 1.0))
            # Internal grid (mech sole pattern)
            for i in range(3):
                parts.append(_line(cx - W * 0.14, cy - H * 0.04 + i * H * 0.04,
                                   cx + W * 0.14, cy - H * 0.04 + i * H * 0.04, "#444444", 0.8))
                parts.append(_line(cx - W * 0.04 + i * W * 0.04, cy - H * 0.14,
                                   cx - W * 0.04 + i * W * 0.04, cy + H * 0.14, "#444444", 0.8))
        return "".join(parts)

    if "chassis_oil_leak" in name or "chassis_coolant_leak" in name:
        # Streak + pool
        is_coolant = "coolant" in name
        col = "#3a8cff" if is_coolant else "#1a1a22"
        col_dark = "#1a4a88" if is_coolant else "#0d0d11"
        # Streak
        for i in range(8):
            sx = W * (0.20 + i * 0.08)
            sy = cy + _m.sin(i * 0.5) * H * 0.05
            parts.append(_ellipse(sx, sy, W * 0.04, H * 0.04, col, col_dark, 0.5))
        # End pool
        parts.append(_circle(W * 0.85, cy, W * 0.12, col, col_dark, 1.0))
        parts.append(_circle(W * 0.85, cy, W * 0.06, _lighten_hex(col, 0.10)))
        return "".join(parts)

    if "glass_shatter_pattern" in name:
        # Radial crack pattern
        for i in range(16):
            ang = i * 0.3927
            x1 = cx
            y1 = cy
            x2 = cx + W * rng.uniform(0.20, 0.40) * _m.cos(ang)
            y2 = cy + H * rng.uniform(0.20, 0.40) * _m.sin(ang)
            parts.append(_line(x1, y1, x2, y2, "#aaccee", 1.5))
            # Branch cracks
            if rng.random() < 0.5:
                mx = (x1 + x2) / 2
                my = (y1 + y2) / 2
                bx = mx + W * 0.05 * _m.cos(ang + 1.0)
                by = my + H * 0.05 * _m.sin(ang + 1.0)
                parts.append(_line(mx, my, bx, by, "#88aacc", 1.0))
        parts.append(_circle(cx, cy, W * 0.04, "#000000"))
        return "".join(parts)

    if "ricochet_scuff" in name:
        # Linear scuff with sparks
        parts.append(_polygon([
            (W * 0.20, H * 0.45),
            (W * 0.80, H * 0.55),
            (W * 0.80, H * 0.60),
            (W * 0.20, H * 0.50),
        ], "#888888", "#444444", 1.0))
        parts.append(_polygon([
            (W * 0.25, H * 0.47),
            (W * 0.78, H * 0.55),
            (W * 0.78, H * 0.58),
            (W * 0.25, H * 0.50),
        ], "#cccccc"))
        # Spark trail at end
        for _ in range(8):
            ang = rng.uniform(-0.5, 0.5)
            x = W * 0.80 + rng.uniform(0, W * 0.15) * _m.cos(ang)
            y = H * 0.55 + rng.uniform(-H * 0.05, H * 0.05)
            parts.append(_circle(x, y, rng.uniform(0.5, 1.5), "#FFE068"))
        return "".join(parts)

    # Generic decal fallback
    parts.append(_circle(cx, cy, W * 0.20, "#444444", "#222222", 1.0))
    parts.append(_circle(cx, cy, W * 0.10, "#000000"))
    return "".join(parts)


def _compose_animation_frame(spec: AssetSpec, rng: random.Random) -> str:
    """Multi-direction walk-cycle frame composer.

    Reads canonical_name suffix `*_walk_frame_N_dir_X` where N is 0..3 and
    X is n/ne/e/se/s/sw/w/nw. Wraps _compose_actor with stance + facing offset.
    """
    name = spec.canonical_name.lower()
    frame_index = 0
    direction = "e"
    if "_frame_" in name:
        try:
            frame_part = name.split("_frame_")[1].split("_")[0]
            frame_index = int(frame_part)
        except (ValueError, IndexError):
            frame_index = 0
    if "_dir_" in name:
        direction = name.split("_dir_")[1].split("_")[0]
    # Direction → facing
    facing = "left" if direction in ("w", "nw", "sw") else "right"
    # Stance based on frame index — 4 distinct stances cycle for visible walk-cycle
    stance = {
        0: "walking",
        1: "running",
        2: "jetting",
        3: "crouching",
    }.get(frame_index, "walking")

    # Build a synthetic AssetSpec with stance + facing patched in
    extra = dict(spec.extra or {})
    extra["stance"] = stance
    extra["facing"] = facing
    patched_spec = AssetSpec(
        canonical_name=spec.canonical_name,
        kind=spec.kind,
        category="ActorSprite",
        width=spec.width,
        height=spec.height,
        seed=spec.seed,
        palette=spec.palette,
        style=spec.style,
        origin_palette=spec.origin_palette,
        extra=extra,
    )
    return _compose_actor(patched_spec, rng)


def _compose_portrait(spec: AssetSpec, rng: random.Random) -> str:
    """NPC / storyteller / boss / faction-generic portrait — face + bust.

    Routes by canonical_name prefix:
        portrait_npc_<faction>_<role>_<name>
        portrait_storyteller_<id>
        portrait_boss_<id>
        portrait_faction_<id>_generic
    """
    import math as _m
    name = spec.canonical_name.lower()
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    metal = p.metal()
    glow = p.glow()
    skin = (spec.origin_palette or p).primary()
    skin_dark = (spec.origin_palette or p).dark()
    W, H = spec.width, spec.height
    cx = W / 2

    parts: List[str] = []
    parts.append(_rect(0, 0, W, H, dark))
    for i in range(8):
        ring_r = W * (0.55 - i * 0.04)
        shade = _darken_hex(body, 0.10 + i * 0.04)
        parts.append(_circle(cx, H * 0.42, ring_r, shade, dark_or(dark, "#000000"), 0.3))
    parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.92, "none", accent, max(1.5, W * 0.012)))
    parts.append(_rect(W * 0.06, H * 0.06, W * 0.88, H * 0.88, "none", dark, 0.8))
    head_r = W * 0.16
    head_cx, head_cy = cx, H * 0.38
    parts.append(_circle(head_cx, head_cy, head_r * 1.05, skin_dark))
    parts.append(_circle(head_cx, head_cy, head_r, skin, skin_dark, 0.6))
    parts.append(_polygon([
        (head_cx - head_r * 0.95, head_cy - head_r * 0.05),
        (head_cx, head_cy - head_r * 0.95),
        (head_cx + head_r * 0.95, head_cy - head_r * 0.05),
        (head_cx + head_r * 0.85, head_cy + head_r * 0.30),
        (head_cx - head_r * 0.85, head_cy + head_r * 0.30),
    ], metal, dark, 1.0))
    parts.append(_rect(head_cx - head_r * 0.55, head_cy - head_r * 0.20,
                       head_r * 1.10, head_r * 0.40, accent, dark, 0.5))
    parts.append(_rect(head_cx - head_r * 0.50, head_cy - head_r * 0.15,
                       head_r * 0.40, head_r * 0.06, glow))
    parts.append(_rect(head_cx + head_r * 0.10, head_cy - head_r * 0.15,
                       head_r * 0.40, head_r * 0.06, glow))
    if "_boss_" in name:
        for j in range(6):
            ang = (j * 60) * 3.14159 / 180
            x1 = head_cx + head_r * 1.3 * _m.cos(ang)
            y1 = head_cy + head_r * 1.3 * _m.sin(ang)
            x2 = head_cx + head_r * 1.8 * _m.cos(ang)
            y2 = head_cy + head_r * 1.8 * _m.sin(ang)
            parts.append(_line(x1, y1, x2, y2, accent, 2.0))
    shoulder_y = head_cy + head_r * 1.2
    shoulder_w = W * 0.50
    parts.append(_polygon([
        (cx - shoulder_w * 0.55, H),
        (cx - shoulder_w * 0.55, shoulder_y + H * 0.05),
        (cx - shoulder_w * 0.40, shoulder_y - H * 0.02),
        (cx, shoulder_y - H * 0.06),
        (cx + shoulder_w * 0.40, shoulder_y - H * 0.02),
        (cx + shoulder_w * 0.55, shoulder_y + H * 0.05),
        (cx + shoulder_w * 0.55, H),
    ], body, dark, 1.2))
    parts.append(_polygon([
        (cx - shoulder_w * 0.55, shoulder_y + H * 0.05),
        (cx - shoulder_w * 0.30, shoulder_y - H * 0.04),
        (cx - shoulder_w * 0.32, shoulder_y + H * 0.10),
    ], metal, dark, 0.6))
    parts.append(_polygon([
        (cx + shoulder_w * 0.55, shoulder_y + H * 0.05),
        (cx + shoulder_w * 0.30, shoulder_y - H * 0.04),
        (cx + shoulder_w * 0.32, shoulder_y + H * 0.10),
    ], metal, dark, 0.6))
    parts.append(_circle(cx, shoulder_y + H * 0.10, W * 0.04, accent, dark, 0.5))
    parts.append(_rect(W * 0.10, H * 0.85, W * 0.80, H * 0.10, dark))
    parts.append(_rect(W * 0.10, H * 0.85, W * 0.80, H * 0.02, accent))
    parts.append(_rect(W * 0.10, H * 0.93, W * 0.80, H * 0.02, accent))
    parts.append(_rect(W * 0.74, H * 0.06, W * 0.20, W * 0.20, body, accent, 1.0))
    parts.append(_circle(W * 0.84, H * 0.06 + W * 0.10, W * 0.07, accent, dark, 0.8))
    return "".join(parts)


def _compose_ui_screen(spec: AssetSpec, rng: random.Random) -> str:
    """Assembled UI screen mockup (M11A shell + M11 in-mission HUD).

    Routes by canonical_name prefix to produce a recognizable layout:
        screen_title_main / screen_main_menu / screen_pause / screen_save_load
        screen_settings_<tab> / screen_credits / screen_loading
        screen_hud_combat / screen_briefing_comic
    """
    name = spec.canonical_name.lower()
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    metal = p.metal()
    glow = p.glow()
    W, H = spec.width, spec.height
    parts: List[str] = []
    parts.append(_rect(0, 0, W, H, dark))
    for i in range(5):
        parts.append(_rect(0, H * i / 5, W, H / 5, _darken_hex(body, 0.4 + i * 0.05)))

    if "title" in name or "main_menu" in name:
        parts.append(_rect(W * 0.30, H * 0.10, W * 0.40, H * 0.25, dark, accent, 2.0))
        parts.append(_rect(W * 0.32, H * 0.12, W * 0.36, H * 0.21, body, accent, 1.0))
        parts.append(_rect(W * 0.34, H * 0.16, W * 0.32, H * 0.05, accent))
        parts.append(_rect(W * 0.34, H * 0.24, W * 0.20, H * 0.03, glow))
        menu_items = ["NEW GAME", "CONTINUE", "MULTIPLAYER", "WORKSHOP", "SETTINGS", "QUIT"]
        for i, _ in enumerate(menu_items):
            y = H * (0.42 + i * 0.07)
            fill = accent if i == 0 else body
            parts.append(_rect(W * 0.35, y, W * 0.30, H * 0.05, fill, dark, 0.8))
            parts.append(_rect(W * 0.36, y + H * 0.018, W * 0.22, H * 0.012, dark))
        parts.append(_rect(W * 0.04, H * 0.94, W * 0.15, H * 0.04, body, accent, 0.5))
        parts.append(_rect(W * 0.81, H * 0.94, W * 0.15, H * 0.04, body, accent, 0.5))
    elif "pause" in name:
        parts.append(_rect(0, 0, W, H, dark, None, 0.0))
        parts.append(_rect(W * 0.25, H * 0.15, W * 0.50, H * 0.70, body, accent, 2.0))
        parts.append(_rect(W * 0.30, H * 0.20, W * 0.40, H * 0.06, accent))
        items = ["RESUME", "SETTINGS", "SAVE", "LOAD", "SHOW ME WHY", "QUIT"]
        for i, _ in enumerate(items):
            y = H * (0.32 + i * 0.08)
            parts.append(_rect(W * 0.30, y, W * 0.40, H * 0.06, _darken_hex(body, 0.2), dark, 0.5))
            parts.append(_rect(W * 0.32, y + H * 0.022, W * 0.28, H * 0.012, accent))
    elif "save" in name or "load" in name:
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.92, H * 0.10, body, accent, 1.5))
        parts.append(_rect(W * 0.06, H * 0.06, W * 0.30, H * 0.06, accent))
        for row in range(3):
            for col in range(3):
                sx = W * (0.06 + col * 0.32)
                sy = H * (0.18 + row * 0.26)
                parts.append(_rect(sx, sy, W * 0.28, H * 0.22, _darken_hex(body, 0.3), accent, 1.0))
                parts.append(_rect(sx + W * 0.01, sy + H * 0.01, W * 0.26, H * 0.10, _darken_hex(body, 0.5)))
                parts.append(_circle(sx + W * 0.14, sy + H * 0.06, W * 0.04, accent))
                parts.append(_rect(sx + W * 0.01, sy + H * 0.13, W * 0.26, H * 0.015, dark))
                parts.append(_rect(sx + W * 0.01, sy + H * 0.155, W * 0.20, H * 0.015, dark))
                parts.append(_rect(sx + W * 0.01, sy + H * 0.18, W * 0.16, H * 0.015, glow))
    elif "settings" in name:
        parts.append(_rect(0, 0, W, H * 0.08, body, accent, 1.0))
        tabs = ["DISPLAY", "AUDIO", "CONTROLS", "ACCESS", "GAMEPLAY", "LANG"]
        for i, _ in enumerate(tabs):
            tx = W * (0.04 + i * 0.16)
            fill = accent if i == 2 else _darken_hex(body, 0.2)
            parts.append(_rect(tx, H * 0.02, W * 0.14, H * 0.05, fill, dark, 0.8))
            parts.append(_rect(tx + W * 0.02, H * 0.038, W * 0.10, H * 0.012, dark))
        for row in range(8):
            ry = H * (0.13 + row * 0.10)
            parts.append(_rect(W * 0.06, ry, W * 0.88, H * 0.08, _darken_hex(body, 0.3), dark, 0.5))
            parts.append(_rect(W * 0.08, ry + H * 0.025, W * 0.30, H * 0.025, accent))
            parts.append(_rect(W * 0.50, ry + H * 0.030, W * 0.40, H * 0.015, dark))
            parts.append(_circle(W * 0.50 + W * 0.20, ry + H * 0.038, W * 0.012, glow))
    elif "credits" in name:
        for col in range(2):
            cxx = W * (0.10 + col * 0.45)
            parts.append(_rect(cxx, H * 0.06, W * 0.35, H * 0.04, accent))
            for j in range(10):
                py = H * (0.13 + j * 0.07)
                parts.append(_rect(cxx, py, W * 0.20, H * 0.015, _darken_hex(body, 0.4)))
                parts.append(_rect(cxx, py + H * 0.025, W * 0.32, H * 0.012, dark))
    elif "loading" in name:
        parts.append(_rect(0, 0, W, H * 0.65, _darken_hex(body, 0.4)))
        for i in range(8):
            parts.append(_circle(rng.uniform(0, W), rng.uniform(0, H * 0.5), rng.uniform(2, 6), glow))
        parts.append(_polygon([
            (0, H * 0.65),
            (W * 0.30, H * 0.50),
            (W * 0.50, H * 0.55),
            (W * 0.75, H * 0.45),
            (W, H * 0.60),
            (W, H * 0.65),
        ], dark))
        parts.append(_rect(W * 0.10, H * 0.78, W * 0.80, H * 0.04, body, accent, 1.0))
        parts.append(_rect(W * 0.10, H * 0.78, W * 0.55, H * 0.04, accent))
        parts.append(_rect(W * 0.10, H * 0.86, W * 0.80, H * 0.10, _darken_hex(body, 0.3), dark, 0.5))
        parts.append(_rect(W * 0.12, H * 0.88, W * 0.70, H * 0.012, accent))
        for j in range(3):
            parts.append(_rect(W * 0.12, H * (0.91 + j * 0.020), W * 0.50, H * 0.012, dark))
    elif "hud_combat" in name:
        parts.append(_rect(W * 0.02, H * 0.03, W * 0.20, H * 0.10, body, accent, 1.0))
        parts.append(_rect(W * 0.03, H * 0.06, W * 0.18, H * 0.02, _darken_hex(body, 0.4)))
        parts.append(_rect(W * 0.03, H * 0.06, W * 0.13, H * 0.02, "#c93030"))
        parts.append(_rect(W * 0.03, H * 0.09, W * 0.10, H * 0.015, "#dab438"))
        parts.append(_rect(W * 0.78, H * 0.03, W * 0.20, H * 0.10, body, accent, 1.0))
        parts.append(_rect(W * 0.80, H * 0.06, W * 0.10, H * 0.03, accent))
        parts.append(_rect(W * 0.32, H * 0.04, W * 0.36, H * 0.06, dark, accent, 1.0))
        parts.append(_rect(W * 0.33, H * 0.05, W * 0.34, H * 0.04, _darken_hex(body, 0.3)))
        parts.append(_rect(W * 0.02, H * 0.30, W * 0.10, H * 0.40, body, accent, 1.0))
        parts.append(_rect(W * 0.03, H * 0.32, W * 0.08, H * 0.08, accent))
        for i in range(4):
            parts.append(_rect(W * 0.03, H * (0.41 + i * 0.07), W * 0.08, H * 0.06, _darken_hex(body, 0.3)))
        parts.append(_circle(W / 2, H / 2, W * 0.012, accent))
        parts.append(_circle(W / 2, H / 2, W * 0.020, "none", accent, 1.5))
        parts.append(_line(W / 2 - W * 0.04, H / 2, W / 2 - W * 0.015, H / 2, accent, 1.5))
        parts.append(_line(W / 2 + W * 0.015, H / 2, W / 2 + W * 0.04, H / 2, accent, 1.5))
        parts.append(_line(W / 2, H / 2 - H * 0.04, W / 2, H / 2 - H * 0.015, accent, 1.5))
        parts.append(_line(W / 2, H / 2 + H * 0.015, W / 2, H / 2 + H * 0.04, accent, 1.5))
        parts.append(_rect(W * 0.62, H * 0.20, W * 0.34, H * 0.30, _darken_hex(body, 0.3), accent, 1.0))
        for i in range(4):
            parts.append(_rect(W * 0.63, H * (0.22 + i * 0.07), W * 0.32, H * 0.05, body, dark, 0.5))
            parts.append(_circle(W * 0.65, H * (0.245 + i * 0.07), W * 0.012, glow))
            parts.append(_rect(W * 0.68, H * (0.235 + i * 0.07), W * 0.15, H * 0.012, accent))
        parts.append(_rect(W * 0.62, H * 0.82, W * 0.34, H * 0.10, body, accent, 1.0))
        parts.append(_rect(W * 0.63, H * 0.84, W * 0.32, H * 0.06, _darken_hex(body, 0.3)))
    elif "briefing_comic" in name or "briefing" in name:
        parts.append(_rect(W * 0.04, H * 0.04, W * 0.46, H * 0.45, body, dark, 3.0))
        parts.append(_rect(W * 0.06, H * 0.06, W * 0.42, H * 0.41, _darken_hex(body, 0.3), dark, 1.0))
        parts.append(_circle(W * 0.20, H * 0.20, W * 0.08, accent))
        parts.append(_rect(W * 0.10, H * 0.30, W * 0.30, H * 0.10, dark))
        parts.append(_rect(W * 0.52, H * 0.04, W * 0.44, H * 0.45, body, dark, 3.0))
        parts.append(_rect(W * 0.54, H * 0.06, W * 0.40, H * 0.41, _darken_hex(body, 0.5), dark, 1.0))
        for i in range(4):
            parts.append(_rect(W * 0.56, H * (0.10 + i * 0.10), W * 0.36, H * 0.05, body))
        parts.append(_rect(W * 0.04, H * 0.52, W * 0.92, H * 0.43, body, dark, 3.0))
        parts.append(_rect(W * 0.06, H * 0.54, W * 0.88, H * 0.05, accent))
        for i in range(6):
            parts.append(_rect(W * 0.06, H * (0.62 + i * 0.05), W * 0.85, H * 0.015, _darken_hex(body, 0.3)))
    else:
        parts.append(_rect(W * 0.10, H * 0.10, W * 0.80, H * 0.80, body, accent, 2.0))
        parts.append(_rect(W * 0.12, H * 0.12, W * 0.76, H * 0.10, accent))
        for i in range(6):
            parts.append(_rect(W * 0.12, H * (0.26 + i * 0.10), W * 0.76, H * 0.06, _darken_hex(body, 0.3)))

    return "".join(parts)


def _compose_vfx_frame(spec: AssetSpec, rng: random.Random) -> str:
    """Animation frame composer for VFX (muzzle_flash / explosion / impact_burst / blood_splat).

    Reads frame_index from canonical_name suffix `_frame_N`. Total frames = 4 unless
    `_of_M` suffix specifies otherwise. Each frame shows progressive dissipation.
    """
    import math as _m
    name = spec.canonical_name.lower()
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    glow = p.glow()
    metal = p.metal()
    W, H = spec.width, spec.height
    cx, cy = W / 2, H / 2

    frame_index = 0
    if "_frame_" in name:
        try:
            frame_index = int(name.split("_frame_")[1].split("_")[0])
        except (ValueError, IndexError):
            frame_index = 0
    fp = frame_index / 4.0
    fade = 1.0 - fp

    parts: List[str] = []

    if "muzzle_flash" in name:
        burst_r = W * (0.10 + fp * 0.20)
        opacity_outer = 0.4 * fade
        outer_color = "#FFCC66" if frame_index < 2 else "#996644"
        parts.append(_circle(cx, cy, burst_r, outer_color, None, 0.0))
        if frame_index < 3:
            for i in range(7):
                ang = i * (3.14159 * 2 / 7) + rng.uniform(-0.1, 0.1)
                tip = W * (0.25 - fp * 0.05)
                x = cx + tip * _m.cos(ang)
                y = cy + tip * _m.sin(ang)
                parts.append(_polygon([
                    (cx + burst_r * 0.6 * _m.cos(ang - 0.15), cy + burst_r * 0.6 * _m.sin(ang - 0.15)),
                    (x, y),
                    (cx + burst_r * 0.6 * _m.cos(ang + 0.15), cy + burst_r * 0.6 * _m.sin(ang + 0.15)),
                ], "#FFE068"))
        if frame_index == 0:
            parts.append(_circle(cx, cy, W * 0.08, "#FFFFFF"))
        elif frame_index < 3:
            parts.append(_circle(cx, cy, W * 0.06 * fade, "#FFEEAA"))
    elif "explosion" in name:
        radius = W * (0.10 + fp * 0.30)
        parts.append(_circle(cx, cy, radius * 1.2, "#FF6633" if frame_index < 2 else "#664433"))
        parts.append(_circle(cx, cy, radius, "#FFCC44" if frame_index < 3 else "#886633"))
        if frame_index < 2:
            parts.append(_circle(cx, cy, radius * 0.6, "#FFFFFF"))
        for i in range(12):
            ang = i * (3.14159 * 2 / 12) + frame_index * 0.1
            dr = radius * (0.7 + rng.uniform(0, 0.5))
            x = cx + dr * _m.cos(ang)
            y = cy + dr * _m.sin(ang)
            chunk_color = "#888888" if frame_index >= 2 else "#FFAA44"
            parts.append(_circle(x, y, rng.uniform(2, 5), chunk_color))
        if frame_index >= 2:
            for j in range(6):
                px = cx + rng.uniform(-radius, radius)
                py = cy - radius * (1 - fade)
                parts.append(_circle(px, py, W * 0.04, "#666666"))
    elif "impact_burst" in name:
        for i in range(8):
            ang = i * (3.14159 * 2 / 8)
            spread = W * (0.05 + fp * 0.25)
            x = cx + spread * _m.cos(ang)
            y = cy + spread * _m.sin(ang)
            color = "#FFEE88" if frame_index < 2 else "#996644"
            parts.append(_circle(x, y, max(1.0, 4 - frame_index), color))
        if frame_index < 2:
            parts.append(_circle(cx, cy, W * 0.05, "#FFFFFF"))
    elif "blood_splat" in name:
        for i in range(12):
            ang = i * (3.14159 * 2 / 12) + rng.uniform(-0.2, 0.2)
            d = W * (0.05 + fp * 0.30 + rng.uniform(0, 0.05))
            x = cx + d * _m.cos(ang)
            y = cy + d * _m.sin(ang)
            r = max(1.0, 6 * fade * rng.uniform(0.5, 1.0))
            parts.append(_circle(x, y, r, "#aa0000"))
        if frame_index < 2:
            parts.append(_circle(cx, cy, W * 0.06, "#cc1111"))
    else:
        for i in range(6):
            ang = i * (3.14159 * 2 / 6)
            x = cx + W * 0.10 * fp * _m.cos(ang)
            y = cy + H * 0.10 * fp * _m.sin(ang)
            parts.append(_circle(x, y, max(1.0, 5 - frame_index), accent))

    return "".join(parts)


def _compose_loading_bg(spec: AssetSpec, rng: random.Random) -> str:
    """Atmospheric loading-screen background per world / scenario."""
    name = spec.canonical_name.lower()
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    metal = p.metal()
    glow = p.glow()
    W, H = spec.width, spec.height
    parts: List[str] = []

    for i in range(10):
        sky_color = _darken_hex(highlight, 0.10 + i * 0.06) if i < 5 else _darken_hex(body, 0.20 + (i - 5) * 0.05)
        parts.append(_rect(0, H * i / 10, W, H / 10, sky_color))

    if "mars" in name or "vulcan" in name:
        parts.append(_circle(W * 0.20, H * 0.18, W * 0.06, "#FFAA66"))
        for i in range(40):
            px = rng.uniform(0, W)
            py = rng.uniform(H * 0.30, H * 0.55)
            parts.append(_circle(px, py, rng.uniform(0.5, 2), "#FFE0AA"))
    elif "europa" in name or "moon" in name:
        parts.append(_circle(W * 0.78, H * 0.15, W * 0.05, "#FFFFCC"))
        for i in range(60):
            px = rng.uniform(0, W)
            py = rng.uniform(0, H * 0.30)
            parts.append(_circle(px, py, rng.uniform(0.5, 1.5), "#FFFFFF"))
    elif "venus" in name:
        for i in range(8):
            parts.append(_circle(rng.uniform(0, W), rng.uniform(0, H * 0.40), W * 0.10, "#cc7733"))
    elif "belt" in name or "orbital" in name:
        for i in range(80):
            px = rng.uniform(0, W)
            py = rng.uniform(0, H * 0.50)
            parts.append(_circle(px, py, rng.uniform(0.5, 1.2), "#FFFFFF"))
        for i in range(6):
            parts.append(_circle(rng.uniform(0, W), rng.uniform(H * 0.20, H * 0.45),
                                 rng.uniform(W * 0.02, W * 0.06), metal, dark, 0.5))

    horizon_y = H * 0.55
    pts = [(0, horizon_y)]
    for i in range(8):
        px = W * (i + 1) / 8
        py = horizon_y + rng.uniform(-H * 0.08, H * 0.04)
        pts.append((px, py))
    pts.append((W, horizon_y))
    pts.append((W, H * 0.65))
    pts.append((0, H * 0.65))
    parts.append(_polygon(pts, _darken_hex(dark, 0.10)))

    for i in range(3):
        sx = W * (0.20 + i * 0.30)
        sy = horizon_y - H * rng.uniform(0.06, 0.14)
        sh = H * 0.18
        parts.append(_polygon([
            (sx - W * 0.04, horizon_y),
            (sx, sy),
            (sx + W * 0.04, horizon_y),
        ], dark))
        parts.append(_rect(sx - W * 0.03, sy + H * 0.04, W * 0.012, H * 0.012, glow))

    fg_y = H * 0.78
    parts.append(_rect(0, fg_y, W, H - fg_y, _darken_hex(dark, 0.20)))
    for i in range(3):
        fx = W * (0.20 + i * 0.30)
        fy = fg_y - H * 0.08
        parts.append(_rect(fx - W * 0.04, fy, W * 0.04, H * 0.04, dark))
        parts.append(_circle(fx - W * 0.02, fy - H * 0.02, W * 0.012, _darken_hex(dark, 0.5)))
        parts.append(_rect(fx - W * 0.05, fy + H * 0.04, W * 0.02, H * 0.05, dark))
        parts.append(_rect(fx - W * 0.01, fy + H * 0.04, W * 0.02, H * 0.05, dark))
    parts.append(_rect(W * 0.04, H * 0.92, W * 0.92, H * 0.04, "none", accent, 1.0))
    return "".join(parts)


def _compose_boss_splash(spec: AssetSpec, rng: random.Random) -> str:
    """Boss intro cinematic key frame — heroic silhouette + threat aura + nameplate."""
    import math as _m
    name = spec.canonical_name.lower()
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    glow = p.glow()
    metal = p.metal()
    W, H = spec.width, spec.height
    cx, cy = W / 2, H * 0.55
    parts: List[str] = []
    for i in range(12):
        ring_r = W * (0.65 - i * 0.04)
        shade = _darken_hex(accent, 0.05 + i * 0.06)
        parts.append(_circle(cx, cy, ring_r, shade))
    parts.append(_rect(0, 0, W, H, dark, None, 0.0))
    parts.append(_circle(cx, cy, W * 0.50, _darken_hex(body, 0.6)))
    for j in range(20):
        ang = j * 18 * 3.14159 / 180
        x1 = cx + W * 0.40 * _m.cos(ang)
        y1 = cy + W * 0.40 * _m.sin(ang)
        x2 = cx + W * 0.55 * _m.cos(ang)
        y2 = cy + W * 0.55 * _m.sin(ang)
        parts.append(_line(x1, y1, x2, y2, accent, 1.5))

    head_r = W * 0.10
    parts.append(_circle(cx, cy - H * 0.20, head_r * 1.1, dark))
    parts.append(_circle(cx, cy - H * 0.20, head_r, _darken_hex(metal, 0.10), dark, 0.8))
    parts.append(_polygon([
        (cx - head_r, cy - H * 0.20),
        (cx + head_r, cy - H * 0.20),
        (cx + head_r * 1.4, cy - H * 0.20 + head_r * 0.4),
        (cx + head_r, cy - H * 0.20 + head_r * 1.2),
        (cx - head_r, cy - H * 0.20 + head_r * 1.2),
        (cx - head_r * 1.4, cy - H * 0.20 + head_r * 0.4),
    ], metal, dark, 0.8))
    parts.append(_rect(cx - head_r * 0.7, cy - H * 0.20 - head_r * 0.1,
                       head_r * 1.4, head_r * 0.4, accent))
    parts.append(_rect(cx - head_r * 0.5, cy - H * 0.20 + head_r * 0.0,
                       head_r * 1.0, head_r * 0.1, "#FF1100"))
    torso_w = W * 0.30
    torso_h = H * 0.25
    parts.append(_rect(cx - torso_w / 2, cy - H * 0.10, torso_w, torso_h, body, dark, 1.5))
    parts.append(_polygon([
        (cx - torso_w / 2, cy - H * 0.10),
        (cx - torso_w * 0.8, cy - H * 0.12),
        (cx - torso_w * 0.7, cy - H * 0.06),
    ], metal, dark, 0.8))
    parts.append(_polygon([
        (cx + torso_w / 2, cy - H * 0.10),
        (cx + torso_w * 0.8, cy - H * 0.12),
        (cx + torso_w * 0.7, cy - H * 0.06),
    ], metal, dark, 0.8))
    parts.append(_rect(cx - W * 0.01, cy - H * 0.05, W * 0.02, H * 0.05, glow))
    for j in range(6):
        sx = cx - torso_w * 0.4 + j * torso_w * 0.16
        parts.append(_rect(sx, cy - H * 0.04, W * 0.012, H * 0.012, metal))

    parts.append(_rect(W * 0.06, H * 0.83, W * 0.88, H * 0.10, dark))
    parts.append(_rect(W * 0.06, H * 0.83, W * 0.88, H * 0.005, accent))
    parts.append(_rect(W * 0.06, H * 0.925, W * 0.88, H * 0.005, accent))
    parts.append(_rect(W * 0.10, H * 0.86, W * 0.40, H * 0.04, accent))
    parts.append(_rect(W * 0.55, H * 0.88, W * 0.20, H * 0.02, "#aa1111"))
    for i in range(8):
        parts.append(_rect(W * (0.06 + i * 0.115), H * 0.96, W * 0.10, H * 0.01, _darken_hex(accent, 0.4)))
    return "".join(parts)


def _compose_key_art(spec: AssetSpec, rng: random.Random) -> str:
    """Marketing key art — cinematic 3-actor composition + faction logo."""
    import math as _m
    name = spec.canonical_name.lower()
    p = spec.palette
    body = p.primary()
    accent = p.accent()
    dark = p.dark()
    highlight = p.highlight()
    metal = p.metal()
    glow = p.glow()
    W, H = spec.width, spec.height
    parts: List[str] = []
    for i in range(10):
        gradient = _darken_hex(highlight, 0.05 + i * 0.06) if i < 5 else _darken_hex(body, 0.10 + (i - 5) * 0.08)
        parts.append(_rect(0, H * i / 10, W, H / 10, gradient))
    for j in range(8):
        ang = (j - 4) * 0.15
        x_start = W / 2
        y_start = H * 0.30
        x_end = W / 2 + W * 1.2 * _m.cos(ang)
        y_end = H * 0.30 + W * 1.2 * _m.sin(ang)
        parts.append(_line(x_start, y_start, x_end, y_end, _darken_hex(accent, 0.20), 0.5))
    horizon_y = H * 0.62
    pts = [(0, horizon_y)]
    for i in range(10):
        px = W * (i + 1) / 10
        py = horizon_y + rng.uniform(-H * 0.10, H * 0.04)
        pts.append((px, py))
    pts.append((W, horizon_y))
    pts.append((W, H * 0.72))
    pts.append((0, H * 0.72))
    parts.append(_polygon(pts, _darken_hex(dark, 0.10)))
    for k in range(4):
        bx = W * (0.20 + k * 0.20)
        by = horizon_y - H * rng.uniform(0.08, 0.20)
        parts.append(_rect(bx - W * 0.03, by, W * 0.06, horizon_y - by, dark))
        for w in range(4):
            wy = by + H * 0.03 + w * H * 0.025
            parts.append(_rect(bx - W * 0.015, wy, W * 0.012, H * 0.012, glow))
            parts.append(_rect(bx + W * 0.003, wy, W * 0.012, H * 0.012, glow))
    actor_positions = [
        (W * 0.30, H * 0.74, 1.0),
        (W * 0.50, H * 0.70, 1.3),
        (W * 0.70, H * 0.74, 1.0),
    ]
    for ax, ay, scale in actor_positions:
        head_r = W * 0.025 * scale
        parts.append(_circle(ax, ay - H * 0.18 * scale, head_r * 1.2, dark))
        parts.append(_circle(ax, ay - H * 0.18 * scale, head_r, _darken_hex(metal, 0.10), dark, 0.5))
        parts.append(_rect(ax - head_r * 0.5, ay - H * 0.18 * scale - head_r * 0.1,
                           head_r * 1.0, head_r * 0.4, accent))
        torso_w = W * 0.04 * scale
        torso_h = H * 0.12 * scale
        parts.append(_rect(ax - torso_w / 2, ay - H * 0.14 * scale, torso_w, torso_h, body, dark, 0.8))
        parts.append(_polygon([
            (ax - torso_w * 0.5, ay - H * 0.14 * scale),
            (ax - torso_w * 0.8, ay - H * 0.15 * scale),
            (ax - torso_w * 0.7, ay - H * 0.12 * scale),
        ], metal, dark, 0.5))
        parts.append(_rect(ax + torso_w * 0.6, ay - H * 0.13 * scale,
                           W * 0.07 * scale, H * 0.02 * scale, _darken_hex(metal, 0.20), dark, 0.5))
        leg_y = ay - H * 0.02 * scale
        parts.append(_rect(ax - torso_w * 0.4, leg_y, torso_w * 0.3, H * 0.08 * scale, _darken_hex(body, 0.3), dark, 0.5))
        parts.append(_rect(ax + torso_w * 0.1, leg_y, torso_w * 0.3, H * 0.08 * scale, _darken_hex(body, 0.3), dark, 0.5))
    parts.append(_rect(W * 0.04, H * 0.04, W * 0.10, H * 0.06, body, accent, 1.0))
    parts.append(_circle(W * 0.04 + W * 0.05, H * 0.04 + H * 0.03, H * 0.020, accent, dark, 0.5))
    parts.append(_rect(W * 0.20, H * 0.06, W * 0.60, H * 0.06, body, accent, 1.0))
    parts.append(_rect(W * 0.22, H * 0.075, W * 0.56, H * 0.03, accent))
    parts.append(_rect(W * 0.10, H * 0.88, W * 0.80, H * 0.06, body, accent, 1.0))
    parts.append(_rect(W * 0.12, H * 0.90, W * 0.76, H * 0.025, accent))
    return "".join(parts)


# ─── Dispatch ───────────────────────────────────────────────────────────────


_COMPOSERS = {
    "WeaponSprite": _compose_weapon,
    "ActorSprite": _compose_actor,
    "VehicleSprite": _compose_vehicle,
    "ChassisSprite": _compose_chassis,
    "BaseModuleSprite": _compose_base_module,
    "UiIcon": _compose_ui_icon,
    "MaterialSwatch": _compose_material,
    "Particle": _compose_particle,
    "TerrainTile": _compose_terrain_tile,
    "Cosmetic": _compose_cosmetic_stub,
    "FactionEmblem": _compose_emblem,
    "CaptureGridOverlay": _compose_overlay,
    "ShellUi": _compose_shell_ui,
    "Banner": _compose_banner,
    "HudWidget": _compose_hud_widget,
    "VfxDecal": _compose_vfx_decal,
    "AnimationFrame": _compose_animation_frame,
    "Portrait": _compose_portrait,
    "UiScreen": _compose_ui_screen,
    "VfxFrame": _compose_vfx_frame,
    "LoadingBg": _compose_loading_bg,
    "BossSplash": _compose_boss_splash,
    "KeyArt": _compose_key_art,
}


def compose_svg(spec: AssetSpec) -> str:
    """Compose an SVG string for the given asset spec.

    Routes by `spec.category`. Pre-pads the header + footer; the composer
    returns only the inner <g>-body.
    """
    rng = random.Random(int(spec.seed) & 0xFFFFFFFFFFFFFFFF)
    composer = _COMPOSERS.get(spec.category)
    if composer is None:
        # Procedural fallback: faction-coherent rectangle stack.
        p = spec.palette
        body = p.primary()
        accent = p.accent()
        dark = p.dark()
        inner = "".join([
            _rect(spec.width * 0.10, spec.height * 0.10, spec.width * 0.80, spec.height * 0.80,
                  body, dark, 1.0),
            _rect(spec.width * 0.30, spec.height * 0.30, spec.width * 0.40, spec.height * 0.40,
                  accent, dark, 0.5),
        ])
    else:
        inner = composer(spec, rng)
    return SVG_HEADER.format(w=spec.width, h=spec.height) + inner + SVG_FOOTER


__all__ = ["AssetSpec", "compose_svg"]
