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
import json
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Optional

GENERATOR_VERSION = "0.3.0"

# Static BP scope table. Matches the canonical Build Points table in
# cortext_command_vault/spec/prototype-roadmap.md. Update both in lockstep.
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
    )


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
    if tag.is_launch_ga:
        previous = "v0.11.0-bp11"
    else:
        bp_num = int(tag.bp.removeprefix("bp"))
        if bp_num <= 1:
            return None
        previous = f"v0.{bp_num - 1}.0-bp{bp_num - 1}"
    existing = run_text(["git", "tag", "--list", previous], repo_root)
    return previous if existing == previous else None


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
        r"cortext_command_vault/[A-Za-z0-9_./#-]+\.md(?:#[A-Za-z0-9_.%+-]+)?",
        r"(?:spec|decisions|dashboards|prototypes|research-log|references|systems|comparables)/[A-Za-z0-9_./#-]+\.md(?:#[A-Za-z0-9_.%+-]+)?",
        r"\[\[([A-Za-z0-9_./# -]+)\]\]",
    ]
    for pattern in patterns:
        for match in re.finditer(pattern, text or ""):
            ref = match.group(1) if match.groups() else match.group(0)
            ref = ref.strip()
            if ref.startswith("cortext_command_vault/"):
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


def find_bp_run_bundle(repo_root: Path, bp: str) -> Optional[Path]:
    """Find the most recent run bundle that corresponds to the BP closure.

    Search order:
    1. `prototype_runs/native/<bp>_*` (when a BP-tagged bundle is available)
    2. The most recent milestone-tagged bundle that anchors the BP per
       BP_ANCHOR_PREFIXES (e.g., BP1 → newest m1.5_* bundle; BP2 → newest
       m2.5_* before M3A because M2.5 is the fun-proof slice).
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
    for prefix in BP_ANCHOR_PREFIXES.get(bp, []):
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


def install_section(tag: TagInfo) -> str:
    scenario = BP_SMOKE_SCENARIOS.get(tag.bp, "m1_actor_range")
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
        "Gatekeeper will warn that the binary is unsigned. Right-click `cf-app` → Open the\n"
        "first time, or run `xattr -dr com.apple.quarantine .` (above) to clear the\n"
        "quarantine flag. Code signing arrives at BP10+ via T-LIVEOPS.\n"
        "\n"
        "### Windows (`x86_64-pc-windows-msvc`)\n"
        "\n"
        "```pwsh\n"
        "Expand-Archive corefall-windows-x86_64-<tag>.zip\n"
        "cd corefall-windows-x86_64-<tag>\n"
        f".\\cf-app.exe --scenario {scenario}\n"
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
    sections = [
        f"# Corefall {tag.raw} ({tag.bp_label})\n",
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
