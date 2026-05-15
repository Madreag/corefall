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
    variant = (spec.extra or {}).get("variant", "side")

    parts: List[str] = []
    hull_y = spec.height * 0.40
    hull_h = spec.height * 0.32
    parts.append(_rect(spec.width * 0.08, hull_y, spec.width * 0.84, hull_h, body, dark, 1.0))
    # Slanted front armor
    parts.append(_polygon(
        [
            (spec.width * 0.92, hull_y),
            (spec.width * 0.98, hull_y + hull_h * 0.4),
            (spec.width * 0.92, hull_y + hull_h),
        ],
        metal, dark, 0.5,
    ))
    # Viewport
    parts.append(_rect(spec.width * 0.62, hull_y + hull_h * 0.18, spec.width * 0.18, hull_h * 0.32,
                       accent, dark, 0.5))
    # Wheels / treads
    for i in range(4):
        wx = spec.width * (0.16 + 0.20 * i)
        parts.append(_circle(wx, spec.height * 0.78, spec.height * 0.07, dark, metal, 0.5))
        parts.append(_circle(wx, spec.height * 0.78, spec.height * 0.04, metal))
    # Top hatch
    parts.append(_rect(spec.width * 0.32, hull_y - spec.height * 0.08, spec.width * 0.18, spec.height * 0.08,
                       body, dark, 1.0))
    if variant == "boarding":
        # ramp open
        parts.append(_polygon(
            [
                (spec.width * 0.08, hull_y + hull_h),
                (spec.width * 0.08, hull_y + hull_h + spec.height * 0.06),
                (spec.width * 0.32, hull_y + hull_h),
            ],
            metal, dark, 0.5,
        ))
    elif variant == "boarded":
        # door closed + indicator
        parts.append(_circle(spec.width * 0.18, hull_y + hull_h * 0.5, spec.width * 0.02, glow))
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
    elif "generator" in canonical or "reactor" in canonical or "battery" in canonical or "capacitor" in canonical:
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.22, accent, glow, 1.0))
        parts.append(_circle(cx, base_y + base_h * 0.5, base_h * 0.12, glow))
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
    else:
        # Industrial / habitat generic
        parts.append(_rect(base_x + base_w * 0.10, base_y + base_h * 0.20,
                           base_w * 0.80, base_h * 0.40, metal, dark, 0.5))

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
    elif "action_use" in name or "action_pickup" in name:
        parts.append(_polygon(
            [
                (cx - spec.width * 0.12, cy + spec.height * 0.12),
                (cx, cy - spec.height * 0.18),
                (cx + spec.width * 0.12, cy + spec.height * 0.12),
            ],
            accent, dark, 0.5,
        ))
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

    # Phase scales the particle's footprint.
    scale = {"spawn": 0.25, "mid": 0.55, "late": 0.80, "dissipate": 1.0}.get(phase, 0.5)
    parts: List[str] = []

    if "spark" in name:
        # Spark fan
        for i in range(8):
            ang = (i / 8.0) * 360.0
            import math
            r = spec.width * 0.35 * scale
            x = cx + r * math.cos(math.radians(ang))
            y = cy + r * math.sin(math.radians(ang))
            parts.append(_line(cx, cy, x, y, accent if i % 2 == 0 else highlight, 1.2))
    elif "smoke" in name or "dust" in name:
        for _ in range(12):
            x = cx + rng.uniform(-spec.width / 4, spec.width / 4) * scale
            y = cy + rng.uniform(-spec.height / 4, spec.height / 4) * scale
            r = spec.width * 0.08 * scale * rng.uniform(0.6, 1.6)
            parts.append(_circle(x, y, r, body))
    elif "ember" in name:
        for _ in range(6):
            x = cx + rng.uniform(-spec.width / 4, spec.width / 4) * scale
            y = cy + rng.uniform(-spec.height / 4, spec.height / 4) * scale
            r = spec.width * 0.04 * scale
            parts.append(_circle(x, y, r, accent))
    elif "impact" in name or "debris" in name:
        for _ in range(8):
            x = cx + rng.uniform(-spec.width / 3, spec.width / 3) * scale
            y = cy + rng.uniform(-spec.height / 3, spec.height / 3) * scale
            parts.append(_polygon(
                [(x, y),
                 (x + 1.5 * scale * spec.width / 32, y + 1.5 * scale * spec.height / 32),
                 (x + 1.5 * scale * spec.width / 32, y - 0.5 * scale * spec.height / 32)],
                body, accent, 0.5,
            ))
    elif "fluid" in name:
        for _ in range(8):
            x = cx + rng.uniform(-spec.width / 3, spec.width / 3) * scale
            y = cy + rng.uniform(-spec.height / 3, spec.height / 3) * scale
            r = spec.width * 0.05 * scale * rng.uniform(0.6, 1.4)
            parts.append(_circle(x, y, r, accent))
    else:
        # Generic glow orb
        parts.append(_circle(cx, cy, spec.width * 0.30 * scale, glow))
        parts.append(_circle(cx, cy, spec.width * 0.18 * scale, highlight))
        parts.append(_circle(cx, cy, spec.width * 0.10 * scale, accent))
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
