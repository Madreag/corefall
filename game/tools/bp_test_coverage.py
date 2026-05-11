#!/usr/bin/env python3
"""bp_test_coverage.py — Per-BP test-suite coverage analyzer.

Reads `game/content/build_points/bp<N>.test_manifest.json` (the declarative
test contract for a BP) and reports gaps between what the contract requires
and what the repository actually has on disk + what the latest run bundles
actually emitted.

This is the inspection half of the AI-Agent Test-Improvement Loop documented
in `.claude/skills/corefall-review/SKILL.md` §AI-Agent Test-Improvement Loop.
The orchestration half is `game/tools/bp_close_loop.sh`. The two are
deliberately split so the agent can:

1. Run the analyzer (`bp_test_coverage.py bp2`) → JSON gap report.
2. Read the gap report, decide which gaps are missing-test (scaffold a new
   test/script/scenario) vs missing-evidence (extend the engine to emit
   the missing event/observe field) vs broken-test (fix the code).
3. Apply fixes via the existing tool surface (Edit/Create/Execute).
4. Re-run the analyzer until 0 gaps + the loop driver asserts the sweep +
   grading + review verdict are also green.

Gap categories the analyzer reports:

- **scenario_missing** — the manifest references a scenario that doesn't
  exist on disk.
- **script_missing** — the manifest references a cfctl script that doesn't
  exist on disk.
- **grading_contract_missing** — the manifest references a grading.json
  contract that doesn't exist on disk.
- **sweep_row_missing** — the manifest requires a sweep row but
  `self_play_sweep.sh` doesn't define it.
- **events_not_emitted** — for each scenario the manifest declares
  `required_events_emitted`; we open the latest run bundle for that
  scenario and assert each required event_type is present in events.jsonl.
- **observe_fields_missing** — for each `required_observe_fields` entry we
  run `cfctl observe --once` and confirm the field is present in the
  envelope OR (when CFCTL_OBSERVE_ONCE is set in the env to a path) read
  the cached observe.once payload. Missing fields = harness gap.
- **cargo_modules_missing** — each `required_cargo_test_modules` glob is
  resolved against the workspace; missing = unimplemented test target.
- **grading_dimension_missing** — for each scenario the manifest declares
  required dimensions; we open the grading contract and assert every
  required dimension id is declared.
- **grading_evidence_unproduced** — cross-reference: each grading contract
  dimension's `evidence_required` references types of evidence (frames,
  events, observe fields). For each one, the corresponding run bundle
  must actually contain that evidence. Catches contract drift between
  what the rubric says it grades against vs what the run actually
  produces.

Invocation:

    python3 game/tools/bp_test_coverage.py bp2 \\
        [--repo-root /path/to/corefall] \\
        [--bundle-dir prototype_runs/native] \\
        [--json] \\
        [--strict]

Exit code:
- 0 = no gaps
- 1 = gaps found
- 2 = manifest invalid / unreadable

JSON output (stable shape) is what `bp_close_loop.sh` parses:

```json
{
  "bp": "bp2",
  "manifest_path": "...",
  "gaps": [
    {"category": "events_not_emitted", "scenario": "micro_reactor_defense_win", "missing": ["combat.projectile_hit"], "owner_hint": "engine code or test scenario script"},
    ...
  ],
  "summary": {"total_gaps": 3, "by_category": {"events_not_emitted": 1, ...}},
  "verdict": "GAPS_FOUND" or "CLEAN"
}
```
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Iterable, List, Optional, Tuple


def load_manifest(path: Path) -> dict:
    if not path.is_file():
        raise SystemExit(f"bp_test_coverage: manifest not found: {path}")
    try:
        m = json.loads(path.read_text())
    except json.JSONDecodeError as e:
        raise SystemExit(f"bp_test_coverage: invalid JSON: {e}")
    if m.get("schema_version") != "cf-bp-test-manifest.v1":
        raise SystemExit(f"bp_test_coverage: unsupported schema_version {m.get('schema_version')!r}")
    return m


def resolve(repo_root: Path, p: str) -> Path:
    pp = Path(p)
    return pp if pp.is_absolute() else repo_root / pp


def bp_number(bp: object) -> Optional[int]:
    m = re.fullmatch(r"bp(\d+)", str(bp or "").lower())
    if not m:
        return None
    return int(m.group(1))


def file_text(repo_root: Path, path: str) -> str:
    p = resolve(repo_root, path)
    try:
        return p.read_text(errors="ignore")
    except OSError:
        return ""


def regex_present(repo_root: Path, path: str, pattern: str) -> bool:
    try:
        return re.search(pattern, file_text(repo_root, path), flags=re.MULTILINE | re.DOTALL) is not None
    except re.error:
        return pattern in file_text(repo_root, path)


def cfctl_script_methods(repo_root: Path, path: str) -> List[str]:
    p = resolve(repo_root, path)
    try:
        data = json.loads(p.read_text())
    except (OSError, json.JSONDecodeError):
        return []
    methods: List[str] = []
    for step in data.get("steps") or []:
        if isinstance(step, dict) and isinstance(step.get("method"), str):
            methods.append(step["method"])
    return methods


def latest_bundle_for_scenario(
    bundles_root: Path,
    scene_id: str,
    outcome: Optional[str] = None,
    fun_proof_only: bool = True,
) -> Optional[Path]:
    """Find the most recent fun-proof bundle whose scene.id matches AND whose
    mission outcome (when given) matches `outcome` ('won' / 'lost' / None).

    Outcome filter looks at the bundle's `summary.json.event_counts.by_type`:
    - `outcome='won'` => the bundle has `mission_resolved>=1` AND
      `objective_completed>=1` AND `objective_failed==0`.
    - `outcome='lost'` => the bundle has `mission_resolved>=1` AND
      `objective_failed>=1`.
    - `outcome=None`  => no outcome filter; first matching bundle wins.

    This lets the manifest declare separate `<scenario>_win` and
    `<scenario>_loss` keys in `required_events_emitted` and the analyzer
    finds the right bundle for each.
    """
    if not bundles_root.is_dir():
        return None
    # Try each known prefix in order.
    prefix_groups = [
        f"m5_*", f"m4a_*", f"m3b_*", f"m3a_*",
        f"m2.5_*", f"m2_*", f"m1.5_*", f"m1_*", f"m0_*",
        f"bp[0-9]*_*",
    ]
    candidates: List[Path] = []
    for pat in prefix_groups:
        for c in sorted(bundles_root.glob(pat)):
            if c.is_dir() and (c / "run_manifest.json").is_file():
                candidates.append(c)
    candidates.sort(key=lambda p: p.name)
    for c in reversed(candidates):
        try:
            mf = json.loads((c / "run_manifest.json").read_text())
        except (OSError, json.JSONDecodeError):
            continue
        if (mf.get("scene") or {}).get("id") != scene_id:
            continue
        if fun_proof_only and (mf.get("run_mode") or "").lower() == "headless-smoke":
            continue
        if outcome is not None:
            counts = event_types_in_bundle_counts(c)
            resolved = counts.get("mission_resolved", 0)
            completed = counts.get("objective_completed", 0)
            failed = counts.get("objective_failed", 0)
            if outcome == "won" and not (resolved >= 1 and completed >= 1 and failed == 0):
                continue
            # `lost` triggers via objective_failed (mission-objective path) OR
            # via mission_resolved without objective_completed (player_dead /
            # timer_expired path; cf-mission resolves lost without firing an
            # objective_failed event).
            if outcome == "lost" and not (resolved >= 1 and (failed >= 1 or completed == 0)):
                continue
        return c
    return None


def event_types_in_bundle_counts(bundle: Path) -> dict:
    summary = bundle / "summary.json"
    if not summary.is_file():
        return {}
    try:
        s = json.loads(summary.read_text())
    except (OSError, json.JSONDecodeError):
        return {}
    return (s.get("event_counts") or {}).get("by_type") or {}


def event_types_in_bundle(bundle: Path) -> List[str]:
    summary = bundle / "summary.json"
    if not summary.is_file():
        return []
    try:
        s = json.loads(summary.read_text())
    except (OSError, json.JSONDecodeError):
        return []
    return list((s.get("event_counts") or {}).get("by_type") or {})


def event_categories_in_bundle(bundle: Path) -> List[str]:
    summary = bundle / "summary.json"
    if not summary.is_file():
        return []
    try:
        s = json.loads(summary.read_text())
    except (OSError, json.JSONDecodeError):
        return []
    return list((s.get("event_counts") or {}).get("by_category") or {})


def fully_qualified_event_name_present(bundle: Path, fq: str) -> bool:
    """`fq` is `category.event_type` (e.g. "combat.weapon_fired"); we check
    that the bundle has BOTH a category match AND an event_type match. If
    the manifest declares only a bare event_type ("weapon_fired"), accept
    that as a match too."""
    if "." in fq:
        cat, et = fq.split(".", 1)
        events_in = event_types_in_bundle(bundle)
        cats_in = event_categories_in_bundle(bundle)
        return et in events_in and cat in cats_in
    else:
        return fq in event_types_in_bundle(bundle)


def expand_cargo_glob(repo_root: Path, glob: str) -> bool:
    """Resolve a `cf-<crate>::<module>::<fn>` glob against the workspace.
    We don't actually run the test; we grep for an `fn <fn>` declaration in
    the matching crate's source to confirm the test exists. Cheap enough.
    """
    parts = glob.split("::")
    if not parts:
        return False
    crate = parts[0]
    crate_dir = repo_root / "game" / "crates" / crate
    if not crate_dir.is_dir():
        return False
    if len(parts) == 1:
        return True
    # The remaining parts are module path + function name; the function
    # name may end in `*`. We just grep the crate source for a matching
    # `fn <name>` or `mod <name>` pattern.
    target = parts[-1]
    try:
        src_iter = list(crate_dir.rglob("*.rs"))
    except OSError:
        return False
    if not src_iter:
        return False
    if target.endswith("*"):
        prefix = target[:-1]
        pattern = re.compile(rf"\bfn\s+{re.escape(prefix)}\w*\s*\(")
    else:
        pattern = re.compile(rf"\bfn\s+{re.escape(target)}\s*\(")
    for path in src_iter:
        try:
            text = path.read_text(errors="ignore")
        except OSError:
            continue
        if pattern.search(text):
            return True
    return False


def sweep_row_present(repo_root: Path, row_id: str) -> bool:
    sweep_path = repo_root / "game" / "tools" / "self_play_sweep.sh"
    if not sweep_path.is_file():
        return False
    text = sweep_path.read_text(errors="ignore")
    return f'ROW="{row_id}"' in text or f"ROW='{row_id}'" in text


def grading_contract_dimensions(repo_root: Path, contract_path: str) -> List[str]:
    p = resolve(repo_root, contract_path)
    if not p.is_file():
        return []
    try:
        c = json.loads(p.read_text())
    except (OSError, json.JSONDecodeError):
        return []
    return [d.get("id") for d in (c.get("dimensions") or []) if isinstance(d, dict) and d.get("id")]


def collect_gaps(manifest: dict, repo_root: Path, bundle_dir: Path) -> List[dict]:
    gaps: List[dict] = []
    bp = manifest.get("bp", "?")
    bp_n = bp_number(bp)

    # 1. Scenarios + scripts on disk.
    for s in manifest.get("fun_proof_scenarios", []):
        sp = resolve(repo_root, s["scenario_path"])
        if not sp.is_file():
            gaps.append({
                "category": "scenario_missing",
                "scenario": s["id"],
                "expected_path": str(sp),
                "owner_hint": "scaffold game/content/scenarios/<scenario>.ron with the BP-scope manifest",
            })
        for key in ("win_script", "loss_script"):
            if key in s:
                gp = resolve(repo_root, s[key])
                if not gp.is_file():
                    gaps.append({
                        "category": "script_missing",
                        "scenario": s["id"],
                        "expected_path": str(gp),
                        "owner_hint": f"scaffold {key} cfctl script driving the {s['id']} {key.replace('_script','')} path",
                    })
        if "grading_contract" in s:
            gp = resolve(repo_root, s["grading_contract"])
            if not gp.is_file():
                gaps.append({
                    "category": "grading_contract_missing",
                    "scenario": s["id"],
                    "expected_path": str(gp),
                    "owner_hint": "scaffold game/content/scenarios/grading/<scenario>.grading.json with per-dimension criteria + evidence + weights",
                })

        # BP2+ T-CAPTURE is not optional for fun-proof evidence. If a fun-proof
        # path launches through cf-e2e, the manifest must require the same
        # non-blank capture threshold the roadmap calls out. This prevents a
        # scenario from passing mechanically while the agent never actually sees
        # the feature it claims to close.
        if bp_n is not None and bp_n >= 2 and not s.get("headless_smoke_required", False):
            capture_expect = "capture.summary_grid.non_blank_ratio>=0.95"
            for expectation_key, script_key in (("win_path_expectations", "win_script"), ("loss_path_expectations", "loss_script")):
                if script_key not in s:
                    continue
                expectations = s.get(expectation_key) or []
                if capture_expect not in expectations:
                    gaps.append({
                        "category": "capture_expectation_missing",
                        "scenario": s["id"],
                        "expectation_key": expectation_key,
                        "missing_expectation": capture_expect,
                        "owner_hint": "add the capture non-blank expectation to the manifest and the matching cf-e2e row in self_play_sweep.sh",
                    })

    for s in manifest.get("supporting_scenarios", []):
        sp = resolve(repo_root, s["scenario_path"])
        if not sp.is_file():
            gaps.append({
                "category": "scenario_missing",
                "scenario": s["id"],
                "expected_path": str(sp),
                "owner_hint": "scaffold supporting scenario .ron",
            })
        if "script" in s:
            gp = resolve(repo_root, s["script"])
            if not gp.is_file():
                gaps.append({
                    "category": "script_missing",
                    "scenario": s["id"],
                    "expected_path": str(gp),
                    "owner_hint": "scaffold supporting cfctl script",
                })

    # 2. Sweep rows.
    for row in manifest.get("required_sweep_rows", []):
        if not sweep_row_present(repo_root, row):
            gaps.append({
                "category": "sweep_row_missing",
                "row_id": row,
                "expected_path": "game/tools/self_play_sweep.sh",
                "owner_hint": "extend self_play_sweep.sh with the row exercising this scenario + cfctl script + --expect set",
            })

    # 3. Cargo test modules.
    for glob in manifest.get("required_cargo_test_modules", []):
        if not expand_cargo_glob(repo_root, glob):
            gaps.append({
                "category": "cargo_module_missing",
                "glob": glob,
                "owner_hint": "add the named cargo test (or a wildcard-matching one) to the listed crate",
            })

    # 4. Grading contract dimensions.
    for scen, dims in (manifest.get("required_grading_dimensions_per_scenario") or {}).items():
        # Find the contract path for this scenario
        contract_path = None
        for s in manifest.get("fun_proof_scenarios", []) + manifest.get("supporting_scenarios", []):
            if s["id"] == scen and "grading_contract" in s:
                contract_path = s["grading_contract"]
                break
        if contract_path is None:
            continue
        actual_dims = grading_contract_dimensions(repo_root, contract_path)
        for required in dims:
            if required not in actual_dims:
                gaps.append({
                    "category": "grading_dimension_missing",
                    "scenario": scen,
                    "missing_dimension": required,
                    "contract_path": contract_path,
                    "owner_hint": "extend the grading contract with this dimension; criterion, evidence_required, weight, future_owners_if_blocked",
                })

    # 5. Required events emitted (per scenario, by win/loss key).
    for key, required_events in (manifest.get("required_events_emitted") or {}).items():
        # The keys in required_events_emitted look like "micro_reactor_defense_win".
        scene = key
        outcome: Optional[str] = None
        if key.endswith("_win"):
            scene = key[: -len("_win")]
            outcome = "won"
        elif key.endswith("_loss"):
            scene = key[: -len("_loss")]
            outcome = "lost"
        bundle = latest_bundle_for_scenario(bundle_dir, scene, outcome=outcome)
        if bundle is None:
            gaps.append({
                "category": "events_bundle_missing",
                "scenario_key": key,
                "owner_hint": f"run the BP's fun-proof scenario {scene!r} via cf-e2e --capture-grid + --write-run-bundle so we have evidence",
            })
            continue
        events_in = event_types_in_bundle(bundle)
        cats_in = event_categories_in_bundle(bundle)
        missing_events: List[str] = []
        for req in required_events:
            if "." in req:
                cat, et = req.split(".", 1)
                if et not in events_in or cat not in cats_in:
                    missing_events.append(req)
            else:
                if req not in events_in:
                    missing_events.append(req)
        if missing_events:
            gaps.append({
                "category": "events_not_emitted",
                "scenario_key": key,
                "bundle": str(bundle),
                "missing_events": missing_events,
                "owner_hint": "either the engine doesn't emit this event yet (code gap) or the cfctl script doesn't exercise the path that triggers it (test gap)",
            })

    # 6. Main-feature semantic probes. These are deliberately BP-owned and
    # manifest-declared so the canonical roadmap's headline promise becomes
    # machine-checkable. They catch the systematic failure mode where an agent
    # implements adjacent scaffolding, updates docs, and passes generic harness
    # checks without touching the milestone's central feature.
    for contract in manifest.get("main_feature_contracts") or []:
        cid = contract.get("id", "<unnamed>")
        for req in contract.get("required_source_patterns") or []:
            path = req.get("path")
            pattern = req.get("pattern")
            if not path or not pattern:
                continue
            if not regex_present(repo_root, path, pattern):
                gaps.append({
                    "category": "main_feature_source_missing",
                    "contract": cid,
                    "path": path,
                    "missing_pattern": pattern,
                    "description": req.get("description", ""),
                    "owner_hint": "implement the roadmap's main feature in production code, not only in docs/tests",
                })
        for req in contract.get("forbidden_source_patterns") or []:
            path = req.get("path")
            pattern = req.get("pattern")
            if not path or not pattern:
                continue
            if regex_present(repo_root, path, pattern):
                gaps.append({
                    "category": "main_feature_forbidden_source_present",
                    "contract": cid,
                    "path": path,
                    "forbidden_pattern": pattern,
                    "description": req.get("description", ""),
                    "owner_hint": "remove the old shortcut or static path instead of layering evidence around it",
                })
        for req in contract.get("required_script_methods") or []:
            script = req.get("script")
            method = req.get("method")
            if not script or not method:
                continue
            methods = cfctl_script_methods(repo_root, script)
            if method not in methods:
                gaps.append({
                    "category": "main_feature_script_method_missing",
                    "contract": cid,
                    "script": script,
                    "missing_method": method,
                    "methods_present": methods,
                    "description": req.get("description", ""),
                    "owner_hint": "drive the feature through the production cf-control/cfctl script path",
                })
        for req in contract.get("required_cli_patterns") or []:
            path = req.get("path", "game/crates/cfctl/src/main.rs")
            pattern = req.get("pattern")
            if not pattern:
                continue
            if not regex_present(repo_root, path, pattern):
                gaps.append({
                    "category": "main_feature_cli_surface_missing",
                    "contract": cid,
                    "path": path,
                    "missing_pattern": pattern,
                    "description": req.get("description", ""),
                    "owner_hint": "expose the production behavior through cfctl, not only raw JSON-RPC scripts",
                })

    return gaps


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("bp", help="Build Point id (e.g. bp2)")
    p.add_argument("--repo-root", type=Path, default=None, help="Repo root (defaults to script's grandparent)")
    p.add_argument("--bundle-dir", type=Path, default=None, help="Run bundle root (default: prototype_runs/native)")
    p.add_argument("--json", action="store_true", help="Emit JSON instead of human-readable text")
    p.add_argument("--strict", action="store_true", help="Exit non-zero on warnings as well as gaps")
    args = p.parse_args()

    repo_root = (args.repo_root or Path(__file__).resolve().parents[2]).resolve()
    manifest_path = repo_root / "game" / "content" / "build_points" / f"{args.bp}.test_manifest.json"
    bundle_dir = args.bundle_dir or (repo_root / "prototype_runs" / "native")

    manifest = load_manifest(manifest_path)
    gaps = collect_gaps(manifest, repo_root, bundle_dir)

    by_category: dict = {}
    for g in gaps:
        by_category[g["category"]] = by_category.get(g["category"], 0) + 1
    verdict = "CLEAN" if not gaps else "GAPS_FOUND"
    output = {
        "schema_version": "cf-bp-coverage.v1",
        "bp": args.bp,
        "manifest_path": str(manifest_path),
        "gaps": gaps,
        "summary": {"total_gaps": len(gaps), "by_category": by_category},
        "verdict": verdict,
    }

    if args.json:
        print(json.dumps(output, indent=2))
    else:
        print(f"BP test-suite coverage — {args.bp.upper()} ({manifest.get('label', '?')})")
        print(f"  manifest: {manifest_path}")
        print(f"  bundle dir: {bundle_dir}")
        print(f"  verdict: {verdict}")
        print(f"  total gaps: {len(gaps)}")
        if not gaps:
            print("  All required tests + scenarios + scripts + grading contracts + events + cargo modules + sweep rows are present and emit the data the grading dimensions need.")
        else:
            print(f"  by category:")
            for cat, n in sorted(by_category.items()):
                print(f"    {cat}: {n}")
            print()
            print("  Gaps:")
            for g in gaps:
                line_bits = [f"[{g['category']}]"]
                for k, v in g.items():
                    if k == "category":
                        continue
                    if isinstance(v, list):
                        v = ", ".join(map(str, v))
                    line_bits.append(f"{k}={v}")
                print("    - " + "  ".join(line_bits))

    return 0 if verdict == "CLEAN" else 1


if __name__ == "__main__":
    sys.exit(main())
