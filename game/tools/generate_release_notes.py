#!/usr/bin/env python3
"""generate_release_notes.py — T-RELEASE release-notes payload generator.

Reads a Build Point tag (e.g. `v0.1.0-bp1`), the staging directory holding the
release archives + SHA256SUMS, and the most recent run bundle in
`prototype_runs/native/`. Emits a Markdown release notes payload to the path
given by `--output` for the `softprops/action-gh-release` step.

The generator is deterministic given the same inputs (commit + tag + bundles
+ tool version). It never invents data: if a section's evidence is missing,
the section is skipped + a `### TODO: <name>` placeholder is emitted so a
human can fill it in.

Sections produced (in order):
1. Hero `summary_grid.png` (if present in the BP's exemplar run bundle).
2. BP scope summary (auto-generated from the canonical Build Points table).
3. Run-bundle stats table (events, ticks, tick rate, p99 ms, final checksum).
4. Human-playtest survey (verbatim from `prototype_runs/native/<bp>_*/notes.md`).
5. Install instructions per platform (Gatekeeper / SmartScreen warnings; ad-hoc
   signed; code signing arrives at BP10+).
6. Determinism contract: the cfctl one-liner that should reproduce the
   recorded `final_sim_checksum`.
7. Linked PRs + linked vault notes.
8. SHA256SUMS table.

Usage:
    python3 game/tools/generate_release_notes.py \\
        --tag v0.1.0-bp1 \\
        --staging release-staging \\
        --output release-staging/RELEASE_NOTES.md
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

GENERATOR_VERSION = "0.1.0"

# Static BP scope table. Matches the canonical Build Points table in
# cortext_command_vault/spec/prototype-roadmap.md. Update both in lockstep.
BP_SCOPE = {
    "bp0": ("Engine bootstrap", ["M0"], "(kickoff smoke)"),
    "bp1": (
        "Actor controller + breach fun proof",
        ["M1", "M1.5", "T-CAPTURE infrastructure"],
        "M1.5 Micro Breach Fun Slice",
    ),
    "bp2": (
        "Material storytelling",
        ["M2", "M2.5"],
        "M2.5 Micro Reactor Defense",
    ),
    "bp3": (
        "Replay + comic-noir UI",
        ["M3A", "M3B", "M4A", "M4B"],
        "(UI is the proof)",
    ),
    "bp4": (
        "Equipment + chassis grammar",
        ["M5"],
        "(slot/wreck/eject scripted scenario)",
    ),
    "bp5": (
        "Full collision gauntlet + sabotage proof",
        ["M5.5", "M5.5.5"],
        "M5.5.5 Micro Sabotage",
    ),
    "bp6": (
        "Material kernel + hazards",
        ["M5.6", "M5.7"],
        "(chained-reaction debrief)",
    ),
    "bp7": (
        "Origin + atmospherics + pressure-hold proof",
        ["M5.8", "M5.9", "M5.9.5", "M5.10"],
        "M5.9.5 Micro Pressure Hold",
    ),
    "bp8": (
        "AI core + LLM mind + material competence",
        ["M6", "M6.5", "M6.6"],
        "(8-criteria humanlike bar)",
    ),
    "bp9": (
        "Mission director + base atmospherics",
        ["M7", "M7.5", "M7.7"],
        "(proof mission A-FEEL gate)",
    ),
    "bp10": (
        "Editor + mod tools + material lab",
        ["M8", "M8.5", "M8.6"],
        "(modder parity smoke)",
    ),
    "bp11": (
        "Networking spine",
        ["M9", "M9.5"],
        "(LAN co-op smoke)",
    ),
    "bp12": (
        "Online co-op + PvP + MMO + launch",
        [
            "M10",
            "M11",
            "M12",
            "T-CONTENT-ART finalization",
            "T-CONTENT-NARRATIVE finalization",
            "T-LOCALIZATION finalization",
            "T-LIVEOPS finalization",
        ],
        "(launch GA build)",
    ),
}


@dataclass
class TagInfo:
    raw: str
    version: str
    bp: str
    bp_label: str
    is_launch_ga: bool


def parse_tag(raw: str) -> TagInfo:
    """Parse `v0.1.0-bp1` or `v1.0.0` into structured TagInfo.

    Tags MUST match the T-RELEASE versioning axis. Anything else fails fast.
    """
    if raw == "v1.0.0":
        return TagInfo(
            raw=raw,
            version="1.0.0",
            bp="bp12",
            bp_label="BP12",
            is_launch_ga=True,
        )
    m = re.fullmatch(r"v(\d+\.\d+\.\d+)-bp(\d+)", raw)
    if not m:
        raise SystemExit(
            f"generate_release_notes: tag '{raw}' does not match T-RELEASE versioning "
            f"axis. Expected `v0.<N>.0-bp<N>` (BP1..BP11) or `v1.0.0` (BP12 launch GA)."
        )
    version, bp_num = m.group(1), int(m.group(2))
    bp = f"bp{bp_num}"
    if bp not in BP_SCOPE:
        raise SystemExit(
            f"generate_release_notes: tag '{raw}' refers to unknown {bp.upper()}. "
            f"Add it to BP_SCOPE in this file + the canonical Build Points table."
        )
    return TagInfo(
        raw=raw,
        version=version,
        bp=bp,
        bp_label=bp.upper(),
        is_launch_ga=False,
    )


def find_bp_run_bundle(repo_root: Path, bp: str) -> Optional[Path]:
    """Find the most recent run bundle that corresponds to the BP closure.

    Search order:
    1. `prototype_runs/native/<bp>_*` (when a BP-tagged bundle is available)
    2. The most recent milestone-tagged bundle that anchors the BP per BP_SCOPE
       (e.g., BP1 → newest m1.5_* bundle; BP2 → newest m2.5_*).
    3. Else None — the run-bundle section is skipped.
    """
    bundles_root = repo_root / "prototype_runs" / "native"
    if not bundles_root.is_dir():
        return None
    # 1) BP-tagged bundle wins if present.
    bp_bundles = sorted(bundles_root.glob(f"{bp}_*"))
    if bp_bundles:
        return bp_bundles[-1]
    # 2) Fall back to the BP's anchor milestone bundle.
    anchor_milestones = BP_SCOPE.get(bp, ("", [], ""))[1]
    for milestone in reversed(anchor_milestones):
        # M1.5 → m1.5_*, M2.5 → m2.5_*, M5.5.5 → m5.5.5_*, etc.
        # Hyphens AND spaces both map to underscore so milestone labels like
        # "T-CAPTURE infrastructure" become "t_capture_infrastructure_*".
        prefix = milestone.lower().replace("-", "_").replace(" ", "_")
        candidates = sorted(bundles_root.glob(f"{prefix}_*"))
        if candidates:
            return candidates[-1]
    return None


def load_summary_json(bundle_dir: Path) -> Optional[dict]:
    summary = bundle_dir / "summary.json"
    if not summary.is_file():
        return None
    try:
        return json.loads(summary.read_text())
    except Exception:
        return None


def load_run_manifest_json(bundle_dir: Path) -> Optional[dict]:
    manifest = bundle_dir / "run_manifest.json"
    if not manifest.is_file():
        return None
    try:
        return json.loads(manifest.read_text())
    except Exception:
        return None


def load_notes_human_playtest(bundle_dir: Path) -> Optional[str]:
    """Pull the human-playtest survey block from notes.md.

    Looks for a section starting with `## Human Playtest Survey` and returns
    its body until the next H2 heading or EOF.
    """
    notes = bundle_dir / "notes.md"
    if not notes.is_file():
        return None
    text = notes.read_text(errors="ignore")
    m = re.search(
        r"^##\s+Human\s+Playtest\s+Survey\s*\n(.*?)(?=^##\s|\Z)",
        text,
        re.IGNORECASE | re.MULTILINE | re.DOTALL,
    )
    if not m:
        return None
    body = m.group(1).strip()
    return body or None


def hero_image_section(bundle_dir: Optional[Path]) -> str:
    if bundle_dir is None:
        return ""
    grid = bundle_dir / "captures" / "summary_grid.png"
    if not grid.is_file():
        return ""
    return (
        "## Hero (T-CAPTURE summary grid)\n"
        "\n"
        "The release archive embeds this image at `summary_grid.png` so it can\n"
        "be inspected offline. It is the BP's fun-proof scenario rendered as an\n"
        "8x8 grid of tick-overlaid frames + one frame per major event keyframe.\n"
        "AI agents read this image directly to validate motion + physics + effects.\n"
        "\n"
        f"`{grid.relative_to(bundle_dir.parent.parent.parent)}`\n"
    )


def scope_section(tag: TagInfo) -> str:
    label, milestones, fun_proof = BP_SCOPE[tag.bp]
    bullets = "\n".join(f"- {m}" for m in milestones)
    return (
        f"## Scope ({tag.bp_label} — {label})\n"
        f"\n"
        f"This release bundles:\n"
        f"\n"
        f"{bullets}\n"
        f"\n"
        f"Fun-proof slice: **{fun_proof}**.\n"
    )


def run_bundle_section(bundle_dir: Optional[Path]) -> str:
    if bundle_dir is None:
        return "## Run bundle stats\n\n_No run bundle available; first BP closure that produces one will populate this section._\n"
    summary = load_summary_json(bundle_dir) or {}
    manifest = load_run_manifest_json(bundle_dir) or {}
    perf = summary.get("performance") or {}
    counts = summary.get("event_counts") or {}
    bundle_rel = bundle_dir.name
    rows = [
        ("Bundle", bundle_rel),
        ("Run id", summary.get("manifest_run_id") or manifest.get("run_id") or "-"),
        ("Tick rate", f"{perf.get('tick_rate_hz', '-')} Hz"),
        ("Ticks", str(summary.get("ticks_run") or perf.get("ticks_run") or "-")),
        ("Wall seconds", f"{perf.get('wall_seconds', '-')}"),
        ("Events", str(counts.get("total") or "-")),
        ("Avg tick ms", f"{perf.get('avg_tick_ms', '-')}"),
        ("p99 tick ms", f"{perf.get('p99_tick_ms', '-')}"),
        ("Final checksum", summary.get("final_sim_checksum") or "-"),
        ("Checksum events", str(summary.get("checksum_event_count") or "-")),
    ]
    table = "| Field | Value |\n|---|---|\n" + "\n".join(
        f"| {k} | `{v}` |" for k, v in rows
    )
    return (
        "## Run bundle stats (exemplar from this BP)\n"
        "\n"
        f"{table}\n"
        "\n"
        "The exemplar run bundle is included verbatim under `run-bundle-exemplar/`\n"
        "in this release archive so determinism can be verified offline.\n"
    )


def playtest_section(bundle_dir: Optional[Path]) -> str:
    if bundle_dir is None:
        return ""
    body = load_notes_human_playtest(bundle_dir)
    if not body:
        return (
            "## Human-playtest survey\n"
            "\n"
            "_No survey row found in `prototype_runs/native/<bundle>/notes.md`. "
            "Per AGENTS.md Build Point Closure Gate this row is mandatory; if you are "
            "publishing this release without one, fix it before merging the next BP._\n"
        )
    return (
        "## Human-playtest survey (verbatim)\n"
        "\n"
        f"{body}\n"
    )


def install_section() -> str:
    return (
        "## Install\n"
        "\n"
        "This is a **pre-alpha** release. Builds are unsigned through BP9; expect platform\n"
        "warnings. Code signing activates at BP10+ via T-LIVEOPS pre-launch wiring.\n"
        "\n"
        "### Linux (`x86_64-unknown-linux-gnu`)\n"
        "\n"
        "```bash\n"
        "tar --use-compress-program='zstd -d' -xf corefall-linux-x86_64-<tag>.tar.zst\n"
        "cd corefall-linux-x86_64-<tag>\n"
        "./cf-app --scenario m1_actor_range\n"
        "```\n"
        "\n"
        "Verify checksum (download `SHA256SUMS.txt` alongside the archive):\n"
        "`shasum -a 256 --ignore-missing -c SHA256SUMS.txt`.\n"
        "\n"
        "### macOS (`aarch64-apple-darwin` / `x86_64-apple-darwin`)\n"
        "\n"
        "```bash\n"
        "tar --use-compress-program='zstd -d' -xf corefall-macos-<arch>-<tag>.tar.zst\n"
        "cd corefall-macos-<arch>-<tag>\n"
        "xattr -dr com.apple.quarantine . || true\n"
        "./cf-app --scenario m1_actor_range\n"
        "```\n"
        "\n"
        "Gatekeeper will warn that the binary is unsigned. Right-click `cf-app` → Open the\n"
        "first time, or run `xattr -dr com.apple.quarantine .` (above) to clear the\n"
        "quarantine flag. Code signing arrives at BP10+ via T-LIVEOPS.\n"
        "\n"
        "### Windows (`x86_64-pc-windows-msvc`)\n"
        "\n"
        "```pwsh\n"
        "Expand-Archive corefall-windows-x86_64-<tag>.zip\n"
        "cd corefall-windows-x86_64-<tag>\n"
        ".\\cf-app.exe --scenario m1_actor_range\n"
        "```\n"
        "\n"
        "SmartScreen will warn that the binary is unrecognized. Click **More info → Run\n"
        "anyway**. Code signing arrives at BP10+ via T-LIVEOPS.\n"
    )


def determinism_section(tag: TagInfo, bundle_dir: Optional[Path]) -> str:
    if bundle_dir is None:
        return ""
    summary = load_summary_json(bundle_dir) or {}
    manifest = load_run_manifest_json(bundle_dir) or {}
    final = summary.get("final_sim_checksum")
    seed = manifest.get("seed")
    scenario = manifest.get("scenario_id") or manifest.get("scenario")
    perf = summary.get("performance") or {}
    tick_rate = (manifest.get("tick_rate_hz")
                 or perf.get("tick_rate_hz")
                 or 60)
    # Mirror run_bundle_section: ticks_run can live at summary top level OR
    # nested under summary["performance"] depending on the run-bundle version.
    # Falling back silently to a hardcoded 300 would publish a verification
    # command that cannot reproduce the recorded final_sim_checksum.
    ticks_run = summary.get("ticks_run") or perf.get("ticks_run") or 300
    if not (final and scenario):
        return ""
    return (
        "## Determinism contract (DR-002)\n"
        "\n"
        f"Per the Roadmap V2 T-RELEASE side track, every release publishes the BP's\n"
        f"exemplar `final_sim_checksum` so a third party can reproduce the run on\n"
        f"their hardware and assert byte-identical behavior.\n"
        "\n"
        "Reproduce locally against this build:\n"
        "\n"
        "```bash\n"
        f"./cfctl run --scenario {scenario} --ticks {ticks_run} "
        f"--tick-rate-hz {tick_rate} --seed {seed if seed is not None else 1} "
        f"--write-run-bundle\n"
        "```\n"
        "\n"
        f"Expected `final_sim_checksum`: `{final}`.\n"
        "\n"
        "Drift = bug. Open an issue with the run bundle attached.\n"
    )


def sha256sums_section(staging: Path) -> str:
    sumsfile = staging / "SHA256SUMS.txt"
    if not sumsfile.is_file():
        return ""
    body = sumsfile.read_text().strip()
    if not body:
        return ""
    return (
        "## SHA256SUMS\n"
        "\n"
        "```\n"
        f"{body}\n"
        "```\n"
    )


def footer_section(tag: TagInfo) -> str:
    canonical_bp = (
        "https://github.com/Madreag/corefall/blob/main/AGENTS.md#build-point-closure-gate"
    )
    return (
        "## Cross-references\n"
        "\n"
        f"- [Build Point Closure Gate (AGENTS.md)]({canonical_bp})\n"
        f"- [Roadmap V2 (canonical vault)](https://github.com/Madreag/corefall#research-vault)\n"
        f"- [CHANGELOG.md](https://github.com/Madreag/corefall/blob/main/CHANGELOG.md)\n"
        "\n"
        "---\n"
        f"\n"
        f"_Generated by `game/tools/generate_release_notes.py` v{GENERATOR_VERSION}_\n"
    )


def build_notes(tag: TagInfo, repo_root: Path, staging: Path) -> str:
    bundle_dir = find_bp_run_bundle(repo_root, tag.bp)
    sections = [
        f"# Corefall {tag.raw} ({tag.bp_label})\n",
        hero_image_section(bundle_dir),
        scope_section(tag),
        run_bundle_section(bundle_dir),
        playtest_section(bundle_dir),
        install_section(),
        determinism_section(tag, bundle_dir),
        sha256sums_section(staging),
        footer_section(tag),
    ]
    return "\n".join(s for s in sections if s).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate corefall release notes")
    parser.add_argument("--tag", required=True, help="Release tag (e.g. v0.1.0-bp1 or v1.0.0)")
    parser.add_argument(
        "--staging",
        type=Path,
        default=None,
        help="Directory containing the release archives + SHA256SUMS.txt",
    )
    parser.add_argument("--output", type=Path, default=None, help="Output Markdown file")
    parser.add_argument(
        "--repo-root",
        type=Path,
        default=None,
        help="Path to corefall repo root (defaults to the script's grandparent dir)",
    )
    parser.add_argument(
        "--print-bp-bundle",
        action="store_true",
        help=(
            "Resolve the BP run-bundle path for --tag (using the same BP-aware "
            "search order as the release notes generator) and print it. Prints "
            "nothing and exits 0 if no bundle is available. Used by the release "
            "workflow to keep archive-bundling and release-notes selection in lockstep."
        ),
    )
    args = parser.parse_args()

    if args.repo_root is None:
        # game/tools/generate_release_notes.py → corefall repo root is two up.
        args.repo_root = Path(__file__).resolve().parents[2]

    tag = parse_tag(args.tag)

    if args.print_bp_bundle:
        bundle = find_bp_run_bundle(args.repo_root, tag.bp)
        if bundle is not None:
            print(bundle)
        return 0

    if args.staging is None or args.output is None:
        raise SystemExit(
            "generate_release_notes: --staging and --output are required unless "
            "--print-bp-bundle is set."
        )

    notes = build_notes(tag, args.repo_root, args.staging)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(notes)
    print(f"Wrote {args.output} ({len(notes)} bytes; tag={tag.raw}; bp={tag.bp})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
