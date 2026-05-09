#!/usr/bin/env python3
"""llm_grade_run.py — LLM-graded test verdict harness.

Per `.claude/skills/corefall-review/SKILL.md` §LLM-Graded Test Verdicts +
`corefall/AGENTS.md` Build Point Closure Gate, every BP fun-proof scenario's
run bundle is gradable along multiple dimensions (look / feel / goal /
agent), not just `--expect key=value` literal pass/fail. The grading is
performed by the AI agent driving the corefall-review session: this tool
scaffolds the grading.json artifact + validates a filled-in version against
the scenario's grading-criterion contract.

Workflow:

1. **Scaffold** — given a run bundle + a grading-criterion file, emit a
   `grading.json` skeleton inside the bundle dir. Each dimension row gets
   the criterion text, the evidence_required pointers, and empty
   {score, prose, verdict} cells the agent fills.

   ```
   python3 game/tools/llm_grade_run.py scaffold \\
       --bundle prototype_runs/native/m2.5_*/ \\
       --criteria game/content/scenarios/grading/micro_reactor_defense.grading.json
   ```

2. **Fill** — the agent (Droid) reads each dimension's evidence
   (summary_grid.png frames, events.jsonl rows, observe.once fields) and
   writes structured grades into `<bundle>/grading.json`. The agent edits
   the file directly via Edit/Create tools.

3. **Validate** — the harness validates the filled grading.json against the
   contract: every dimension has a non-empty score + non-empty prose, no
   placeholder text, scores are in [0..10], aggregate score >=
   minimum_aggregate_for_pass, every per-dimension score >=
   minimum_per_dimension_for_pass.

   ```
   python3 game/tools/llm_grade_run.py validate \\
       --bundle prototype_runs/native/m2.5_*/
   ```

   Exit code: 0 on PASS, 1 on FAIL. Emits a verdict line + per-dimension
   summary to stdout.

4. **Report** — emit a human-readable summary suitable for embedding in
   the AI-Agent Self-Test Report or the BP closure note.

   ```
   python3 game/tools/llm_grade_run.py report \\
       --bundle prototype_runs/native/m2.5_*/
   ```

The grading.json artifact is the durable evidence of the LLM grading,
auditable by future reviewers / Bugbot / Devin / human playtest.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import List, Optional, Tuple

PLACEHOLDER_TOKENS = {
    "",
    "TODO",
    "todo",
    "FILL_ME",
    "<agent prose>",
    "<score>",
    "PASS / FAIL",
    "looks correct",
    "looks fine",
    "n/a",
    "N/A",
}
# Bare "PASS" is a LEGITIMATE verdict (not a placeholder). Verdicts that ARE
# placeholders are the scaffold's literal slash-form ("PASS / FAIL") and any
# value that's empty or matches the no-content tokens above.


def load_criteria(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"llm_grade_run: criteria file not found: {path}")
    try:
        c = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        raise SystemExit(f"llm_grade_run: invalid JSON in {path}: {e}")
    if c.get("schema_version") != "cf-grading.v1":
        raise SystemExit(f"llm_grade_run: unsupported schema_version in {path}: {c.get('schema_version')}")
    if not isinstance(c.get("dimensions"), list) or not c["dimensions"]:
        raise SystemExit(f"llm_grade_run: criteria must declare a non-empty 'dimensions' list: {path}")
    return c


def find_criteria_for_bundle(repo_root: Path, bundle: Path) -> Optional[Path]:
    """Resolve the grading-criteria file for a bundle by reading its
    run_manifest.json scene.id and matching against
    `game/content/scenarios/grading/<scene_id>.grading.json`.
    """
    manifest_path = bundle / "run_manifest.json"
    if not manifest_path.is_file():
        return None
    try:
        manifest = json.loads(manifest_path.read_text())
    except (OSError, json.JSONDecodeError):
        return None
    scene_id = (manifest.get("scene") or {}).get("id")
    if not scene_id:
        return None
    candidate = repo_root / "game" / "content" / "scenarios" / "grading" / f"{scene_id}.grading.json"
    return candidate if candidate.is_file() else None


def scaffold(criteria: dict, bundle: Path, agent_id: str) -> dict:
    """Build a grading.json skeleton from the criteria contract. The agent
    fills in score/prose/verdict; everything else (scenario_id, criterion
    text, evidence pointers, weight) comes from the contract."""
    from datetime import datetime, timezone

    rows = []
    for dim in criteria["dimensions"]:
        rows.append({
            "id": dim["id"],
            "criterion": dim["criterion"],
            "evidence_required": dim.get("evidence_required", []),
            "weight": float(dim.get("weight", 1.0)),
            "future_owners_if_blocked": dim.get("future_owners_if_blocked", []),
            "score": None,
            "max_score": criteria.get("rubric", {}).get("score_range", [0, 10])[1],
            "evidence_read": [],
            "prose": "",
            "verdict": "",
        })
    return {
        "schema_version": "cf-grading.v1",
        "scenario_id": criteria["scenario_id"],
        "scope": criteria.get("scope"),
        "milestone": criteria.get("milestone"),
        "criteria_path": str((Path(__file__).parent.parent / "content" / "scenarios" / "grading" / f"{criteria['scenario_id']}.grading.json").relative_to(Path(__file__).parent.parent.parent)),
        "bundle": bundle.name,
        "agent": agent_id,
        "timestamp": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
        "rubric": criteria.get("rubric", {}),
        "dimensions": rows,
        "aggregate_score": None,
        "aggregate_max": None,
        "verdict": "",
        "summary_prose": "",
    }


def is_placeholder(text: str) -> bool:
    if text is None:
        return True
    s = str(text).strip()
    if not s:
        return True
    if s in PLACEHOLDER_TOKENS:
        return True
    if s.startswith("<") and s.endswith(">"):
        return True
    if s.startswith("_") and s.endswith("_") and len(s) > 2:
        return True
    return False


def validate_filled(grading: dict) -> Tuple[bool, List[str]]:
    """Validate a filled grading.json against the contract. Returns
    (passes, list_of_issues). Empty issues + True = clean PASS."""
    issues: List[str] = []
    rubric = grading.get("rubric") or {}
    # Devin 3212580462: defensive read of score_range. The cf-grading.v1
    # contract guarantees [0, 10] but a hand-edited grading.json could pass
    # a malformed `score_range` (single-element list, scalar, missing). Crashing
    # on tuple-unpack would mask the error as a stack trace; reporting it as
    # a structured validation issue is the same shape as every other rubric
    # error and lets `cmd_validate` surface it via the normal FAIL path.
    raw_range = rubric.get("score_range") or [0, 10]
    if isinstance(raw_range, (list, tuple)) and len(raw_range) >= 2:
        score_lo, score_hi = raw_range[0], raw_range[1]
    else:
        issues.append(
            f"rubric.score_range must be a 2-element list [low, high]; got {raw_range!r}. "
            "Falling back to [0, 10] for downstream checks."
        )
        score_lo, score_hi = 0, 10
    min_agg = float(rubric.get("minimum_aggregate_for_pass", 7.0))
    min_per = int(rubric.get("minimum_per_dimension_for_pass", 5))
    dims = grading.get("dimensions") or []
    if not dims:
        return False, ["grading has no dimensions"]
    weighted_sum = 0.0
    total_weight = 0.0
    weighted_max = 0.0
    for d in dims:
        did = d.get("id", "<unknown>")
        score = d.get("score")
        prose = d.get("prose")
        evidence_read = d.get("evidence_read") or []
        verdict = d.get("verdict")
        if score is None:
            issues.append(f"{did}: score is null — agent must fill")
            continue
        try:
            score = float(score)
        except (TypeError, ValueError):
            issues.append(f"{did}: score must be a number, got {score!r}")
            continue
        if score < score_lo or score > score_hi:
            issues.append(f"{did}: score {score} outside [{score_lo}..{score_hi}]")
        if is_placeholder(prose):
            issues.append(f"{did}: prose is empty / placeholder — agent must write prose justification")
        elif len(str(prose).strip()) < 30:
            issues.append(f"{did}: prose too short ({len(str(prose).strip())} chars) — must articulate the look/feel/goal observation")
        if not evidence_read:
            issues.append(f"{did}: evidence_read is empty — agent must list what it actually read (frames, event types, observe fields)")
        if is_placeholder(verdict):
            issues.append(f"{did}: verdict is empty / placeholder — agent must classify (PASS / PARTIAL / NEEDS_FIXES / FUTURE_OWNED)")
        if score < min_per:
            future_owners = d.get("future_owners_if_blocked") or []
            v = (verdict or "").upper()
            if not (future_owners and ("FUTURE" in v or "PARTIAL" in v)):
                issues.append(
                    f"{did}: score {score} below minimum_per_dimension_for_pass {min_per} "
                    f"and not classified as PARTIAL/FUTURE_OWNED — agent must either raise score with new evidence or flag as NEEDS_FIXES"
                )
        weight = float(d.get("weight", 1.0))
        weighted_sum += score * weight
        total_weight += weight
        weighted_max += score_hi * weight
    if total_weight > 0:
        agg = weighted_sum / total_weight
    else:
        agg = 0.0
    grading["aggregate_score"] = round(agg, 2)
    grading["aggregate_max"] = score_hi
    if agg < min_agg:
        issues.append(
            f"aggregate score {round(agg, 2)} (weighted) below minimum_aggregate_for_pass {min_agg} — "
            f"either raise specific dimensions with stronger evidence or flag the BP/milestone as NEEDS_FIXES"
        )
    if is_placeholder(grading.get("verdict")):
        issues.append("top-level verdict is empty / placeholder — agent must summarize (PASS / PARTIAL / NEEDS_FIXES / BLOCKER)")
    if is_placeholder(grading.get("summary_prose")):
        issues.append("top-level summary_prose is empty / placeholder — agent must write 2-4 sentences summarizing the run quality")
    return (len(issues) == 0), issues


def render_report(grading: dict) -> str:
    out = []
    out.append(f"# LLM-Graded Test Verdict — {grading.get('scenario_id', '?')} ({grading.get('scope', '?')})\n")
    out.append(f"- Bundle: `{grading.get('bundle', '?')}`\n")
    out.append(f"- Agent: `{grading.get('agent', '?')}`\n")
    out.append(f"- Timestamp: `{grading.get('timestamp', '?')}`\n")
    out.append(f"- Aggregate score: **{grading.get('aggregate_score', '?')} / {grading.get('aggregate_max', '?')}** (weighted)\n")
    out.append(f"- Top-level verdict: **{grading.get('verdict', '?')}**\n")
    out.append("\n")
    out.append(f"_{grading.get('summary_prose', '')}_\n\n")
    out.append("| Dimension | Score | Verdict | Prose (excerpt) |\n")
    out.append("|---|---|---|---|\n")
    for d in grading.get("dimensions") or []:
        prose = (d.get("prose") or "").replace("\n", " ").replace("|", "\\|")
        if len(prose) > 200:
            prose = prose[:197] + "..."
        out.append(f"| `{d.get('id', '?')}` | {d.get('score', '?')} / {d.get('max_score', '?')} | {d.get('verdict', '?')} | {prose} |\n")
    return "".join(out)


def cmd_scaffold(args: argparse.Namespace) -> int:
    bundle = args.bundle.resolve()
    if not bundle.is_dir():
        print(f"llm_grade_run: bundle dir not found: {bundle}", file=sys.stderr)
        return 2
    criteria_path = args.criteria
    if criteria_path is None:
        repo_root = args.repo_root or Path(__file__).resolve().parents[2]
        criteria_path = find_criteria_for_bundle(repo_root, bundle)
        if criteria_path is None:
            print(f"llm_grade_run: no grading-criteria file resolved for {bundle.name}; pass --criteria explicitly", file=sys.stderr)
            return 2
    criteria = load_criteria(criteria_path)
    out_path = args.output or (bundle / "grading.json")
    if out_path.is_file() and not args.force:
        print(f"llm_grade_run: {out_path} already exists; pass --force to overwrite", file=sys.stderr)
        return 2
    skeleton = scaffold(criteria, bundle, args.agent)
    out_path.write_text(json.dumps(skeleton, indent=2) + "\n")
    print(f"llm_grade_run: wrote scaffold to {out_path}")
    print(f"  scenario={skeleton['scenario_id']} scope={skeleton['scope']} dimensions={len(skeleton['dimensions'])}")
    print(f"  next: agent reads each dimension's evidence_required, fills score/prose/verdict, then run `validate`")
    return 0


def cmd_validate(args: argparse.Namespace) -> int:
    bundle = args.bundle.resolve()
    grading_path = bundle / "grading.json"
    if not grading_path.is_file():
        print(f"llm_grade_run: no grading.json in {bundle}; run `scaffold` first", file=sys.stderr)
        return 2
    try:
        grading = json.loads(grading_path.read_text())
    except json.JSONDecodeError as e:
        print(f"llm_grade_run: invalid JSON in {grading_path}: {e}", file=sys.stderr)
        return 2
    ok, issues = validate_filled(grading)
    if args.write:
        grading_path.write_text(json.dumps(grading, indent=2) + "\n")
    if ok:
        print(f"llm_grade_run: PASS  aggregate={grading.get('aggregate_score')}/{grading.get('aggregate_max')}  verdict={grading.get('verdict')}")
        return 0
    print(f"llm_grade_run: FAIL  {len(issues)} issue(s):")
    for iss in issues:
        print(f"  - {iss}")
    print(f"  aggregate={grading.get('aggregate_score')}/{grading.get('aggregate_max')}  verdict={grading.get('verdict')}")
    return 1


def cmd_report(args: argparse.Namespace) -> int:
    bundle = args.bundle.resolve()
    grading_path = bundle / "grading.json"
    if not grading_path.is_file():
        print(f"llm_grade_run: no grading.json in {bundle}; run `scaffold` first", file=sys.stderr)
        return 2
    grading = json.loads(grading_path.read_text())
    print(render_report(grading))
    return 0


def main() -> int:
    p = argparse.ArgumentParser(description="LLM-graded test verdict harness for corefall fun-proof scenarios")
    sub = p.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("scaffold", help="Emit a grading.json skeleton from a scenario's grading-criteria contract")
    sp.add_argument("--bundle", type=Path, required=True, help="Run bundle dir (will get grading.json written into it)")
    sp.add_argument("--criteria", type=Path, default=None, help="Grading-criteria file; auto-resolved from scene.id if omitted")
    sp.add_argument("--output", type=Path, default=None, help="Output path (default: <bundle>/grading.json)")
    sp.add_argument("--agent", default=os.environ.get("AGENT_ID", "Droid (model unspecified)"),
                    help="Agent identity string")
    sp.add_argument("--force", action="store_true", help="Overwrite existing grading.json")
    sp.add_argument("--repo-root", type=Path, default=None, help="Repo root (defaults to script's grandparent)")
    sp.set_defaults(func=cmd_scaffold)

    vp = sub.add_parser("validate", help="Validate a filled grading.json against the rubric")
    vp.add_argument("--bundle", type=Path, required=True, help="Run bundle dir containing grading.json")
    vp.add_argument("--write", action="store_true", help="Write computed aggregate_score back into grading.json")
    vp.set_defaults(func=cmd_validate)

    rp = sub.add_parser("report", help="Print a human-readable Markdown summary of grading.json")
    rp.add_argument("--bundle", type=Path, required=True, help="Run bundle dir containing grading.json")
    rp.set_defaults(func=cmd_report)

    args = p.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
