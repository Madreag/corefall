#!/usr/bin/env python3
"""generate_release_notes.py — T-RELEASE release-notes payload generator.

Reads a Build Point tag (channel-based `v0.<N>.0-prealpha|alpha|beta|rc`,
GA `v1.0.0`, or legacy `v0.<N>.0-bp<N>`), the staging directory holding
the release archives + SHA256SUMS, and the most recent run bundle in
`prototype_runs/native/`. Emits a Markdown release notes payload to the
path given by `--output` for the `softprops/action-gh-release` step.

Channel boundaries: prealpha BP0-BP3 (engine + first fun slices),
alpha BP4-BP6 (full collision + atmospherics + AI combat),
beta BP7-BP9 (mission director + creator alpha + server/LAN),
rc BP10-BP11 (online + public systems beta), GA BP12 (launch).

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
        --tag v0.1.0-prealpha \\
        --staging release-staging \\
        --output release-staging/RELEASE_NOTES.md
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

GENERATOR_VERSION = "0.4.0"

# Static BP scope table. Matches the canonical Build Points table in
# docs/plan/spec/prototype-roadmap.md. Update both in lockstep.
BP_SCOPE = {
    "bp0": (
        "Foundation Build",
        ["M0 — Engine Bootstrap"],
        "(kickoff smoke)",
    ),
    "bp1": (
        "Micro Breach Build",
        [
            "M1 — Actor Controller And Sim Core",
            "M1.5 — Micro Breach Fun Slice",
            "T-CAPTURE infrastructure",
        ],
        "M1.5 Micro Breach Fun Slice",
    ),
    "bp2": (
        "Terrain & Replay Build",
        [
            "M2 — Pixel Terrain And Materials",
            "M2.5 — Micro Reactor Defense",
            "M3A — Event Recorder Core",
        ],
        "M2.5 Micro Reactor Defense",
    ),
    "bp3": (
        "Combat Readability Build",
        [
            "M3B — Replay Viewer And Debrief",
            "M4A — Readability And ACC-A Floor",
            "M5 — Equipment, Chassis, And Damage Grammar",
        ],
        "M5 Chassis Wreck/Eject",
    ),
    "bp4": (
        "Physics Sandbox Alpha",
        [
            "M5.5 — Full Collision Gauntlet",
            "M5.5.5 — Micro Sabotage",
            "M5.6 — Material Kernel",
            "M5.7 — Hazard Package",
            "M5.8 — Origin Resource & Overclock Pass",
        ],
        "M5.5.5 Micro Sabotage",
    ),
    "bp5": (
        "Atmospherics & Worlds Alpha",
        [
            "M5.9 — Atmospherics-Grade Kernel",
            "M5.9.5 — Micro Pressure Hold",
            "M5.10 — Environmental Conditions Aggregation",
        ],
        "M5.9.5 Micro Pressure Hold",
    ),
    "bp6": (
        "AI Combat Alpha",
        [
            "M6 — AI Core And Trust Harness",
            "M6.5 — LLM Mind Lab",
            "M6.6 — AI Material Competence",
        ],
        "AI-H + MIND + AI-MAT acceptance suites",
    ),
    "bp7": (
        "Vertical Slice Alpha",
        [
            "M7 — Mission Director And Breach Contract",
            "M7.5 — Base Atmospherics",
            "M7.7 — Weather And Day/Night Kernel",
            "M4B — Comic-Noir Polish",
        ],
        "Breach Contract + Bunker Defence proof",
    ),
    "bp8": (
        "Creator Alpha",
        [
            "M8 — Scenario Editor And Mod Tools",
            "M8.5 — Material Lab",
            "M8.6 — Mining, Refining, And Material Economy",
        ],
        "Modder parity smoke",
    ),
    "bp9": (
        "Server / LAN Alpha",
        ["M9 — Dedicated Server App", "M10 — LAN Co-op"],
        "LAN co-op smoke",
    ),
    "bp10": (
        "Online Beta",
        ["M11 — Online Co-op", "M9.5 — Voice And Radio Comms"],
        "Self-hosted online co-op + comms",
    ),
    "bp11": (
        "Public Systems Beta",
        ["M12 — Public PvP Arenas + Persistent MMO Shards"],
        "PvP arena + MMO shard proof",
    ),
    "bp12": (
        "Release Candidate",
        [
            "T-CONTENT-ART finalization",
            "T-CONTENT-NARRATIVE finalization",
            "T-LOCALIZATION finalization",
            "T-LIVEOPS finalization",
        ],
        "Launch GA build",
    ),
}

# Preferred exemplar bundle prefixes per BP. A BP-tagged bundle wins first, then
# these prefixes are tried in order. This keeps release archives centered on
# the BP's fun-proof slice even when the BP also includes infrastructure
# milestones such as M3A.
BP_ANCHOR_PREFIXES = {
    "bp0": ["m0"],
    "bp1": ["m1.5", "m1"],
    "bp2": ["m2.5", "m3a", "m2"],
    "bp3": ["m5", "m4a", "m3b"],
    "bp4": ["m5.5.5", "m5.8", "m5.7", "m5.6", "m5.5"],
    "bp5": ["m5.9.5", "m5.10", "m5.9"],
    "bp6": ["m6.6", "m6.5", "m6"],
    "bp7": ["m7", "m7.5", "m7.7", "m4b"],
    "bp8": ["m8.6", "m8.5", "m8"],
    "bp9": ["m10", "m9"],
    "bp10": ["m11", "m9.5"],
    "bp11": ["m12"],
    "bp12": ["bp12"],
}

BP_SMOKE_SCENARIOS = {
    "bp0": "m0_blank",
    "bp1": "micro_breach",
    "bp2": "micro_reactor_defense",
    "bp3": "m5_chassis_wreck_eject",
    "bp4": "micro_sabotage",
    "bp5": "micro_pressure_hold",
    "bp6": "ai_trust_harness",
    "bp7": "breach_contract",
    "bp8": "sample_mod_breach",
    "bp9": "breach_contract",
    "bp10": "breach_contract",
    "bp11": "pvp_arena_smoke",
    "bp12": "breach_contract",
}

BP_VAULT_NOTES = {
    "bp0": [
        "spec/prototype-roadmap.md#BP0",
        "spec/native-implementation-backlog.md#M0",
        "spec/feature-completion-checklist.md#M0",
    ],
    "bp1": [
        "spec/prototype-roadmap.md#BP1",
        "spec/prototype-roadmap.md#M1.5",
        "spec/feature-completion-checklist.md#BP1",
        "prototypes/native-m1-5-micro-breach.md",
    ],
    "bp2": [
        "spec/prototype-roadmap.md#BP2",
        "spec/prototype-roadmap.md#M2.5",
        "spec/prototype-roadmap.md#M3A",
        "spec/native-implementation-backlog.md#M2.5",
        "spec/native-implementation-backlog.md#M3A",
        "spec/feature-completion-checklist.md#BP2",
    ],
    "bp3": [
        "spec/prototype-roadmap.md#BP3",
        "spec/prototype-roadmap.md#M3B",
        "spec/prototype-roadmap.md#M4A",
        "spec/prototype-roadmap.md#M5",
        "spec/feature-completion-checklist.md#BP3",
    ],
    "bp4": [
        "spec/prototype-roadmap.md#BP4",
        "spec/prototype-roadmap.md#M5.5.5",
        "spec/full-collision-physics-plan.md",
        "spec/feature-completion-checklist.md#BP4",
    ],
    "bp5": [
        "spec/prototype-roadmap.md#BP5",
        "spec/atmospherics-and-chemistry-model.md",
        "decisions/dr-037-stationeers-grade-atmospherics-direction.md",
        "spec/feature-completion-checklist.md#BP5",
    ],
    "bp6": [
        "spec/prototype-roadmap.md#BP6",
        "spec/ai-trust-harness-slice-a.md",
        "spec/hybrid-llm-ai-plan.md",
        "spec/feature-completion-checklist.md#BP6",
    ],
    "bp7": [
        "spec/prototype-roadmap.md#BP7",
        "spec/mission-director-slice-a.md",
        "spec/command-core-base-power.md",
        "spec/feature-completion-checklist.md#BP7",
    ],
    "bp8": [
        "spec/prototype-roadmap.md#BP8",
        "spec/modding-model.md",
        "spec/package-builder-workbench-slice-a.md",
        "spec/feature-completion-checklist.md#BP8",
    ],
    "bp9": [
        "spec/prototype-roadmap.md#BP9",
        "spec/server-app-architecture.md",
        "spec/feature-completion-checklist.md#BP9",
    ],
    "bp10": [
        "spec/prototype-roadmap.md#BP10",
        "spec/server-app-architecture.md",
        "spec/feature-completion-checklist.md#BP10",
    ],
    "bp11": [
        "spec/prototype-roadmap.md#BP11",
        "spec/persistent-mmo-architecture.md",
        "spec/feature-completion-checklist.md#BP11",
    ],
    "bp12": [
        "spec/prototype-roadmap.md#BP12",
        "spec/feature-completion-checklist.md#BP12",
        "spec/authoritative-game-spec-v0.md",
    ],
}


@dataclass
class TagInfo:
    raw: str
    version: str
    bp: str
    bp_label: str
    is_launch_ga: bool
    channel: str  # "prealpha" | "alpha" | "beta" | "rc" | "ga" | "bp" (legacy)


# T-RELEASE channel boundaries. Tags whose BP falls outside the
# channel's allowed range are rejected by parse_tag (with the legacy
# `-bp<N>` form excluded from the boundary check for backward compat).
CHANNEL_BP_RANGES = {
    "prealpha": range(0, 4),   # BP0..BP3
    "alpha": range(4, 7),      # BP4..BP6
    "beta": range(7, 10),      # BP7..BP9
    "rc": range(10, 12),       # BP10..BP11
    "ga": range(12, 13),       # BP12
}

CHANNEL_LABEL = {
    "prealpha": "Prealpha",
    "alpha": "Alpha",
    "beta": "Beta",
    "rc": "Release Candidate",
    "ga": "Launch GA",
    "bp": "BP",  # legacy
}


@dataclass
class PullRequestEvidence:
    number: int
    title: str
    url: str
    merged_at: str
    body_excerpt: str
    vault_refs: list[str]
    source: str


def parse_tag(raw: str) -> TagInfo:
    """Parse a T-RELEASE tag into structured TagInfo.

    Accepted shapes:
    - `v1.0.0` (BP12 launch GA)
    - `v0.<N>.0-prealpha` (BP0..BP3)
    - `v0.<N>.0-alpha` (BP4..BP6)
    - `v0.<N>.0-beta` (BP7..BP9)
    - `v0.<N>.0-rc` (BP10..BP11)
    - `v0.<N>.0-bp<N>` (legacy; backward compat with already-published
      releases — new tags should use the channel-based form)
    Anything else fails fast.
    """
    if raw == "v1.0.0":
        return TagInfo(
            raw=raw,
            version="1.0.0",
            bp="bp12",
            bp_label="BP12",
            is_launch_ga=True,
            channel="ga",
        )
    # Channel-based form: v0.<N>.0-<channel>
    m = re.fullmatch(r"v(\d+\.\d+\.\d+)-(prealpha|alpha|beta|rc)", raw)
    if m:
        version, channel = m.group(1), m.group(2)
        major, minor, patch = (int(part) for part in version.split("."))
        if major != 0 or patch != 0:
            raise SystemExit(
                f"generate_release_notes: tag '{raw}' does not match the T-RELEASE "
                f"version axis. Pre-1.0 tags must use `v0.<N>.0-<channel>`."
            )
        bp_num = minor
        allowed = CHANNEL_BP_RANGES[channel]
        if bp_num not in allowed:
            raise SystemExit(
                f"generate_release_notes: tag '{raw}' uses channel '{channel}' "
                f"but BP{bp_num} is outside its allowed range "
                f"BP{allowed.start}..BP{allowed.stop - 1}. Channel boundaries: "
                f"prealpha BP0-BP3, alpha BP4-BP6, beta BP7-BP9, rc BP10-BP11, "
                f"GA BP12."
            )
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
            channel=channel,
        )
    # Legacy form: v0.<N>.0-bp<N>
    m = re.fullmatch(r"v(\d+\.\d+\.\d+)-bp(\d+)", raw)
    if not m:
        raise SystemExit(
            f"generate_release_notes: tag '{raw}' does not match T-RELEASE versioning "
            f"axis. Expected `v0.<N>.0-(prealpha|alpha|beta|rc)` (BP0..BP11), "
            f"`v1.0.0` (BP12 launch GA), or legacy `v0.<N>.0-bp<N>`."
        )
    version, bp_num = m.group(1), int(m.group(2))
    major, minor, patch = (int(part) for part in version.split("."))
    if major != 0 or patch != 0 or minor != bp_num:
        raise SystemExit(
            f"generate_release_notes: tag '{raw}' does not match the T-RELEASE "
            f"version axis. BP{bp_num} must use `v0.{bp_num}.0-bp{bp_num}`."
        )
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
        channel="bp",
    )


def channel_for_bp(bp_num: int) -> str:
    """Return the canonical channel name for a given BP number.

    Used by previous_bp_tag to construct the predecessor's expected tag
    when callers don't know whether the previous BP shipped under
    prealpha/alpha/beta/rc/legacy.
    """
    for channel, allowed in CHANNEL_BP_RANGES.items():
        if bp_num in allowed:
            return channel
    raise ValueError(f"BP{bp_num} has no known channel")


def run_text(cmd: list[str], cwd: Path, timeout: int = 15) -> Optional[str]:
    try:
        proc = subprocess.run(
            cmd,
            cwd=cwd,
            check=False,
            text=True,
            capture_output=True,
            timeout=timeout,
        )
    except Exception:
        return None
    if proc.returncode != 0:
        return None
    return proc.stdout.strip()


def previous_bp_tag(tag: TagInfo, repo_root: Path) -> Optional[str]:
    if tag.bp == "bp0":
        return None
    bp_num = int(tag.bp.removeprefix("bp"))
    if tag.is_launch_ga:
        previous_bp = 11
    else:
        if bp_num <= 1:
            return None
        previous_bp = bp_num - 1
    # Try every channel form the predecessor could have shipped under.
    # The first existing tag wins. Channel order matches the BP boundaries
    # so the "correct" channel for that BP is checked first, with legacy
    # `-bp<N>` last for backward compat.
    canonical_channel = channel_for_bp(previous_bp)
    candidates = [f"v0.{previous_bp}.0-{canonical_channel}"]
    for channel in ("prealpha", "alpha", "beta", "rc"):
        if channel == canonical_channel:
            continue
        candidates.append(f"v0.{previous_bp}.0-{channel}")
    candidates.append(f"v0.{previous_bp}.0-bp{previous_bp}")
    for candidate in candidates:
        existing = run_text(["git", "tag", "--list", candidate], repo_root)
        if existing == candidate:
            return candidate
    return None


def commit_range_for_tag(tag: TagInfo, repo_root: Path) -> str:
    previous = previous_bp_tag(tag, repo_root)
    if previous:
        return f"{previous}..HEAD"
    root = run_text(["git", "rev-list", "--max-parents=0", "HEAD"], repo_root)
    if root:
        return f"{root}..HEAD"
    return "HEAD"


def pr_numbers_from_git(tag: TagInfo, repo_root: Path) -> list[int]:
    commit_range = commit_range_for_tag(tag, repo_root)
    log = run_text(
        ["git", "log", "--format=%B%n---END-COMMIT---", commit_range],
        repo_root,
        timeout=30,
    )
    if not log:
        return []
    found: list[int] = []
    for match in re.finditer(
        r"(?:Merge\s+pull\s+request\s+#|Merge\s+PR\s+#|PR\s+#|pull/)(\d+)",
        log,
        re.IGNORECASE,
    ):
        number = int(match.group(1))
        if number not in found:
            found.append(number)
    return found


def excerpt(text: str, limit: int = 900) -> str:
    text = re.sub(r"\s+", " ", text or "").strip()
    if not text:
        return "-"
    if len(text) <= limit:
        return text
    return text[: limit - 1].rstrip() + "…"


def extract_vault_refs(text: str) -> list[str]:
    refs: list[str] = []
    patterns = [
        r"(?:docs/plan|cortext_command_vault)/[A-Za-z0-9_./#-]+\.md(?:#[A-Za-z0-9_.%+-]+)?",
        r"(?:spec|decisions|dashboards|prototypes|research-log|references|systems|comparables)/[A-Za-z0-9_./#-]+\.md(?:#[A-Za-z0-9_.%+-]+)?",
        r"\[\[([A-Za-z0-9_./# -]+)\]\]",
    ]
    for pattern in patterns:
        for match in re.finditer(pattern, text or ""):
            ref = match.group(1) if match.groups() else match.group(0)
            ref = ref.strip()
            if ref.startswith("docs/plan/"):
                ref = ref.removeprefix("docs/plan/")
            elif ref.startswith("cortext_command_vault/"):
                ref = ref.removeprefix("cortext_command_vault/")
            if ref and ref not in refs:
                refs.append(ref)
    return refs[:10]


def load_pr_with_gh(number: int, repo_root: Path) -> Optional[PullRequestEvidence]:
    repo = run_text(["git", "config", "--get", "remote.origin.url"], repo_root) or "Madreag/corefall"
    if repo.startswith("git@github.com:"):
        repo = repo.removeprefix("git@github.com:").removesuffix(".git")
    elif repo.startswith("https://github.com/"):
        repo = repo.removeprefix("https://github.com/").removesuffix(".git")
    raw = run_text(
        [
            "gh",
            "pr",
            "view",
            str(number),
            "--repo",
            repo,
            "--json",
            "number,title,url,body,mergedAt",
        ],
        repo_root,
        timeout=20,
    )
    if not raw:
        return None
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        return None
    body = data.get("body") or ""
    return PullRequestEvidence(
        number=int(data.get("number") or number),
        title=data.get("title") or f"PR #{number}",
        url=data.get("url") or f"https://github.com/{repo}/pull/{number}",
        merged_at=data.get("mergedAt") or "-",
        body_excerpt=excerpt(body),
        vault_refs=extract_vault_refs(body),
        source="gh",
    )


def fallback_pr_from_git(number: int, repo_root: Path) -> PullRequestEvidence:
    log = run_text(
        [
            "git",
            "log",
            "--format=%s%n%b",
            "--extended-regexp",
            "--grep",
            rf"#{number}([^0-9]|$)",
            "--all-match",
            "-n",
            "1",
        ],
        repo_root,
    )
    title = f"PR #{number}"
    if log:
        first = next((line.strip() for line in log.splitlines() if line.strip()), "")
        if first:
            title = first
    return PullRequestEvidence(
        number=number,
        title=title,
        url=f"https://github.com/Madreag/corefall/pull/{number}",
        merged_at="-",
        body_excerpt="PR body unavailable in this environment; generated from local git commit references.",
        vault_refs=extract_vault_refs(log or ""),
        source="git",
    )


def merged_pr_evidence(tag: TagInfo, repo_root: Path) -> list[PullRequestEvidence]:
    evidence: list[PullRequestEvidence] = []
    for number in pr_numbers_from_git(tag, repo_root):
        item = load_pr_with_gh(number, repo_root) or fallback_pr_from_git(number, repo_root)
        evidence.append(item)
    return evidence


def _bundle_is_fun_proof(bundle_dir: Path, bp: str) -> bool:
    """Return True when the bundle looks like a real fun-proof BP closure run.

    Filters out stale `--headless-smoke` bundles (M0 boilerplate manifests +
    no captures), bundles missing a manifest entirely, and bundles whose
    scene.id does not match the BP's expected fun-proof scenario.

    Without this, the BP-anchor sort picks the most-recent `m2.5_*` directory
    by ISO timestamp regardless of whether it's the actual fun-proof bundle
    or a developer's manual `cf-app --headless-smoke` smoke run from later
    that day. The release archive then ships a stale-template bundle as the
    "BP exemplar".
    """
    manifest_path = bundle_dir / "run_manifest.json"
    if not manifest_path.is_file():
        return False
    try:
        manifest = json.loads(manifest_path.read_text())
    except (OSError, json.JSONDecodeError):
        return False
    run_mode = (manifest.get("run_mode") or "").lower()
    if run_mode == "headless-smoke":
        return False
    expected_scene = BP_SMOKE_SCENARIOS.get(bp)
    if expected_scene:
        scene_id = (manifest.get("scene") or {}).get("id")
        if scene_id and scene_id != expected_scene:
            # Allow milestone-anchor bundles whose scenario differs from the
            # BP's headline fun-proof scenario (M2 dig path is BP2 scope but
            # not the fun-proof slice). Only reject when scene.id matches a
            # different BP's headline scenario.
            other_bp_scenes = set(BP_SMOKE_SCENARIOS.values()) - {expected_scene}
            if scene_id in other_bp_scenes:
                return False
    return True


def find_bp_run_bundle(repo_root: Path, bp: str) -> Optional[Path]:
    """Find the most recent run bundle that corresponds to the BP closure.

    Search order:
    1. `prototype_runs/native/<bp>_*` (when a BP-tagged bundle is available)
    2. The most recent milestone-tagged bundle that anchors the BP per
       BP_ANCHOR_PREFIXES (e.g., BP1 → newest m1.5_* bundle; BP2 → newest
       m2.5_* before M3A because M2.5 is the fun-proof slice).
       Stale headless-smoke bundles + cross-BP scenarios are filtered out
       via `_bundle_is_fun_proof` so a developer's later `cf-app
       --headless-smoke` smoke run doesn't shadow the actual fun-proof
       evidence.
    3. Else None — the run-bundle section is skipped.
    """
    bundles_root = repo_root / "prototype_runs" / "native"
    if not bundles_root.is_dir():
        return None
    # 1) BP-tagged bundle wins if present (still filter to keep contract).
    bp_bundles = [
        b for b in sorted(bundles_root.glob(f"{bp}_*"))
        if b.is_dir() and _bundle_is_fun_proof(b, bp)
    ]
    if bp_bundles:
        return bp_bundles[-1]
    # 2) Fall back to the BP's anchor milestone bundle, newest fun-proof first.
    for prefix in BP_ANCHOR_PREFIXES.get(bp, []):
        candidates = [
            c for c in sorted(bundles_root.glob(f"{prefix}_*"))
            if c.is_dir() and _bundle_is_fun_proof(c, bp)
        ]
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
    # Use explicit `is not None` checks because numeric fields like ticks_run
    # and event totals can legitimately be 0, which is falsy in Python `or`
    # chains and would otherwise be silently replaced by the "-" placeholder.
    ticks_run = summary.get("ticks_run")
    if ticks_run is None:
        ticks_run = perf.get("ticks_run")
    total_events = counts.get("total")
    rows = [
        ("Bundle", bundle_rel),
        ("Run id", summary.get("manifest_run_id") or manifest.get("run_id") or "-"),
        ("Tick rate", f"{perf.get('tick_rate_hz', '-')} Hz"),
        ("Ticks", str(ticks_run) if ticks_run is not None else "-"),
        ("Wall seconds", f"{perf.get('wall_seconds', '-')}"),
        ("Events", str(total_events) if total_events is not None else "-"),
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


def _install_quality_blurb(tag: TagInfo, bp_num: int) -> str:
    """Return the leading "this is a <channel> release" sentence for the
    install section, sourced from the tag's channel so the prose tracks
    the release-notes title instead of hardcoding "pre-alpha".
    """
    if tag.is_launch_ga:
        return "This is the **launch GA** release (`v1.0.0`)."
    if tag.channel == "prealpha":
        return "This is a **prealpha** release (engine + early fun slices; major systems still missing)."
    if tag.channel == "alpha":
        return "This is an **alpha** release (full collision + atmospherics + AI combat shipped; polish ongoing)."
    if tag.channel == "beta":
        return "This is a **beta** release (mission director + creator alpha + server/LAN; feature-complete-ish)."
    if tag.channel == "rc":
        return "This is a **release candidate** (online + public PvP/MMO; shippable to public-facing playtests)."
    # Legacy bp<N> tags fall back to the BP-derived channel description.
    canonical = channel_for_bp(bp_num)
    return f"This is a **{canonical}** release (legacy `-bp{bp_num}` tag form)."


def _install_signing_blurb(bp_num: int, is_launch_ga: bool) -> str:
    """Return the signing-status sentence for the install section.

    Per docs/plan/spec/prototype-roadmap.md §T-RELEASE: ad-hoc/unsigned
    through BP9; T-LIVEOPS activates Apple notarization + Windows
    Authenticode at BP10+; full code signing at BP12 (v1.0.0 GA).
    """
    if is_launch_ga:
        return "Builds are fully code-signed (Apple notarized + Windows Authenticode); no platform warnings expected."
    if bp_num >= 10:
        return "Builds are code-signed via T-LIVEOPS (Apple notarized + Windows Authenticode); minimal platform warnings."
    return "Builds are unsigned through BP9; expect platform warnings. Code signing activates at BP10+ via T-LIVEOPS pre-launch wiring."


def _macos_signing_paragraph(bp_num: int, is_launch_ga: bool) -> str:
    if is_launch_ga or bp_num >= 10:
        return (
            "macOS builds are notarized + stapled, so Gatekeeper opens them without a "
            "warning. If a download arrives quarantined for any reason, "
            "`xattr -dr com.apple.quarantine .` clears it."
        )
    return (
        "Gatekeeper will warn that the binary is unsigned. Right-click `cf-app` → Open the "
        "first time, or run `xattr -dr com.apple.quarantine .` (above) to clear the "
        "quarantine flag. Code signing arrives at BP10+ via T-LIVEOPS."
    )


def _windows_signing_paragraph(bp_num: int, is_launch_ga: bool) -> str:
    if is_launch_ga or bp_num >= 10:
        return (
            "Windows builds are signed with an Authenticode certificate, so SmartScreen "
            "should accept them without prompting."
        )
    return (
        "SmartScreen will warn that the binary is unrecognized. Click **More info → Run "
        "anyway**. Code signing arrives at BP10+ via T-LIVEOPS."
    )


def install_section(tag: TagInfo) -> str:
    scenario = BP_SMOKE_SCENARIOS.get(tag.bp, "m1_actor_range")
    bp_num = int(tag.bp.removeprefix("bp"))
    quality = _install_quality_blurb(tag, bp_num)
    signing = _install_signing_blurb(bp_num, tag.is_launch_ga)
    macos_paragraph = _macos_signing_paragraph(bp_num, tag.is_launch_ga)
    windows_paragraph = _windows_signing_paragraph(bp_num, tag.is_launch_ga)
    return (
        "## Install\n"
        "\n"
        f"{quality} {signing}\n"
        "\n"
        "### Linux (`x86_64-unknown-linux-gnu`)\n"
        "\n"
        "```bash\n"
        "tar --use-compress-program='zstd -d' -xf corefall-linux-x86_64-<tag>.tar.zst\n"
        "cd corefall-linux-x86_64-<tag>\n"
        f"./cf-app --scenario {scenario}\n"
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
        f"./cf-app --scenario {scenario}\n"
        "```\n"
        "\n"
        f"{macos_paragraph}\n"
        "\n"
        "### Windows (`x86_64-pc-windows-msvc`)\n"
        "\n"
        "```pwsh\n"
        "Expand-Archive corefall-windows-x86_64-<tag>.zip\n"
        "cd corefall-windows-x86_64-<tag>\n"
        f".\\cf-app.exe --scenario {scenario}\n"
        "```\n"
        "\n"
        f"{windows_paragraph}\n"
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
    # Use explicit `is not None` checks because numeric fields can legitimately
    # be 0 (falsy in Python `or` chains). Silently substituting the hardcoded
    # default would publish a verification command that cannot reproduce the
    # recorded final_sim_checksum, violating the determinism contract.
    tick_rate = manifest.get("tick_rate_hz")
    if tick_rate is None:
        tick_rate = perf.get("tick_rate_hz")
    if tick_rate is None:
        tick_rate = 60
    # ticks_run can live at summary top level OR nested under
    # summary["performance"] depending on the run-bundle version.
    ticks_run = summary.get("ticks_run")
    if ticks_run is None:
        ticks_run = perf.get("ticks_run")
    if ticks_run is None:
        ticks_run = 300
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


def linked_evidence_section(tag: TagInfo, repo_root: Path) -> str:
    prs = merged_pr_evidence(tag, repo_root)
    vault_refs = list(BP_VAULT_NOTES.get(tag.bp, []))
    for pr in prs:
        for ref in pr.vault_refs:
            if ref not in vault_refs:
                vault_refs.append(ref)

    lines = ["## Linked PRs and vault evidence", ""]
    if prs:
        lines.extend(
            [
                "### Merged PRs in this release range",
                "",
                "| PR | Title | Merged | Evidence source | Body excerpt |",
                "|---|---|---|---|---|",
            ]
        )
        for pr in prs:
            title = pr.title.replace("|", "\\|")
            body = pr.body_excerpt.replace("|", "\\|")
            lines.append(
                f"| [#{pr.number}]({pr.url}) | {title} | `{pr.merged_at}` | `{pr.source}` | {body} |"
            )
        lines.append("")
    else:
        lines.extend(
            [
                "### Merged PRs in this release range",
                "",
                "_No merged PR numbers were found in the local release range. If this was a squash-only or manually-tagged release, add the PR links before publishing._",
                "",
            ]
        )

    lines.extend(["### Canonical vault notes", ""])
    if vault_refs:
        for ref in vault_refs:
            lines.append(f"- `{ref}`")
    else:
        lines.append("_No BP-specific vault note mapping exists yet; update `BP_VAULT_NOTES` before publishing this release._")
    lines.append("")
    return "\n".join(lines)


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
    channel_label = CHANNEL_LABEL[tag.channel]
    if tag.channel == "bp":
        title = f"# Corefall {tag.raw} ({tag.bp_label})\n"
    elif tag.is_launch_ga:
        title = f"# Corefall {tag.raw} — Launch GA ({tag.bp_label})\n"
    else:
        title = f"# Corefall {tag.raw} ({tag.bp_label} — {channel_label})\n"
    sections = [
        title,
        hero_image_section(bundle_dir),
        scope_section(tag),
        run_bundle_section(bundle_dir),
        playtest_section(bundle_dir),
        install_section(tag),
        determinism_section(tag, bundle_dir),
        linked_evidence_section(tag, repo_root),
        sha256sums_section(staging),
        footer_section(tag),
    ]
    return "\n".join(s for s in sections if s).rstrip() + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Generate corefall release notes")
    parser.add_argument(
        "--tag",
        required=True,
        help=(
            "Release tag. Channel-based: v0.<N>.0-prealpha (BP0-BP3), "
            "v0.<N>.0-alpha (BP4-BP6), v0.<N>.0-beta (BP7-BP9), "
            "v0.<N>.0-rc (BP10-BP11), v1.0.0 (BP12 GA). "
            "Legacy v0.<N>.0-bp<N> still accepted for backward compat."
        ),
    )
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
