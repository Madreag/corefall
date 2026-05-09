#!/usr/bin/env python3
"""agent_self_test_report.py — scaffold the AI-Agent Self-Test Report.

Per `.claude/skills/corefall-review/SKILL.md` §AI-Agent Self-Test Report Gate +
`AGENTS.md` Build Point Closure Gate, every BP closure must include an
`## AI-Agent Self-Test Report` section in `prototype_runs/native/<bp>_*/notes.md`
answering Q1..Q7 with concrete evidence + the agent's prose articulation of
look + feel + juice. This tool reads a run bundle (or a sweep of bundles),
extracts the structured signal — manifest, summary, events.jsonl, observe.once,
captures/ dir, summary_grid.png path, capture_manifest.json — and emits a
markdown skeleton the agent fills with its own prose observations.

The scaffold does NOT auto-fill the prose cells. The point of the gate is that
the agent personally reads the summary_grid.png and writes what it saw;
auto-generated text would defeat the gate. The scaffold pulls every piece of
machine-readable evidence into the report so the agent has nothing to look up.

Usage:
    python3 game/tools/agent_self_test_report.py \\
        --bp bp2 \\
        --bundle prototype_runs/native/m2.5_2026-05-08T23-52-44Z_e5868b68 \\
        [--output prototype_runs/native/bp2_<UTC>_<hash>/notes.md] \\
        [--agent "Droid (claude-sonnet-4.5-20250522)"]

If `--output` is omitted the scaffold goes to stdout so the agent can review
before writing. If `--bundle` is omitted, the script auto-discovers the
fun-proof bundle via the same BP-anchor logic as `generate_release_notes.py`.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Optional

# Mirror the BP_SCOPE table from generate_release_notes.py. The two scripts
# share the canonical mapping; if they drift, BP closure docs disagree with
# release notes.
BP_GOALS = {
    "bp0": [
        "M0 — Engine Bootstrap: cargo workspace, cf-control + cfctl, scenario loader, observe/inspect/act surface, run-bundle writer, schema dump, cf-mod validate, 60+120 Hz determinism + headless smoke.",
    ],
    "bp1": [
        "M1 — Actor Controller And Sim Core: cf-actor + cf-physics + cf-equipment + cf-render-2d + cf-ui; act.player.{move,aim,fire,reload,jump,select_item,reset}.",
        "M1.5 — Micro Breach Fun Slice: cf-mission + cf-terrain BreachStrip + cf-ai ReactiveGuard; act.player.dig; mission state machine; reactive enemy with utility-scored tactics; 60-90 s win/loss scenario.",
        "T-CAPTURE: cf-capture frame readback + capture_grid.py composer + summary_grid.png self-test surface.",
    ],
    "bp2": [
        "M2 — Pixel Terrain And Materials: cf-terrain ChunkedTerrain + 8-material launch set (DR-007) + try_carve/try_blast/fill_aabb/fill_circle + projectile-vs-terrain collision; material_schema_version=cf-terrain-launch-v1.",
        "M2.5 — Micro Reactor Defense Fun Slice: cf-mission Reactor + DefendReactor objective + LossReason::ReactorDestroyed; dirt-shield strategic-choice scenario where the player chooses to preserve the shield (win) or breach it (loss).",
        "M3A — Event Recorder Core: snapshot.* events at run_started + ExpectedOutcome contract (clean/panic/abort) + cf-headless replay verifier with tick-for-tick checksum verification.",
    ],
    "bp3": [
        "M3B — Replay Viewer And Debrief.",
        "M4A — Readability And ACC-A Floor.",
        "M5 — Equipment, Chassis, And Damage Grammar.",
    ],
    "bp4": [
        "M5.5 — Full Collision Gauntlet.",
        "M5.5.5 — Micro Sabotage Fun Slice.",
        "M5.6 — Material Kernel.",
        "M5.7 — Hazard Package.",
        "M5.8 — Origin Resource & Overclock Pass.",
    ],
    "bp5": [
        "M5.9 — Atmospherics-Grade Kernel.",
        "M5.9.5 — Micro Pressure Hold Fun Slice.",
        "M5.10 — Environmental Conditions Aggregation.",
    ],
    "bp6": [
        "M6 — AI Core And Trust Harness.",
        "M6.5 — LLM Mind Lab.",
        "M6.6 — AI Material Competence.",
    ],
    "bp7": [
        "M7 — Mission Director And Breach Contract.",
        "M7.5 — Base Atmospherics.",
        "M7.7 — Weather And Day/Night Kernel.",
        "M4B — Comic-Noir Polish.",
    ],
    "bp8": [
        "M8 — Scenario Editor And Mod Tools.",
        "M8.5 — Material Lab.",
        "M8.6 — Mining, Refining, And Material Economy.",
    ],
    "bp9": [
        "M9 — Dedicated Server App.",
        "M10 — LAN Co-op.",
    ],
    "bp10": [
        "M11 — Online Co-op.",
        "M9.5 — Voice And Radio Comms.",
    ],
    "bp11": [
        "M12 — Public PvP Arenas + Persistent MMO Shards.",
    ],
    "bp12": [
        "T-CONTENT-ART finalization.",
        "T-CONTENT-NARRATIVE finalization.",
        "T-LOCALIZATION finalization.",
        "T-LIVEOPS finalization.",
        "Launch GA build.",
    ],
}

BP_FUN_PROOF_SCENARIOS = {
    "bp0": "m0_smoke_5s",
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


def find_bundle(repo_root: Path, bp: str) -> Optional[Path]:
    bundles_root = repo_root / "prototype_runs" / "native"
    if not bundles_root.is_dir():
        return None
    bp_bundles = sorted(bundles_root.glob(f"{bp}_*"))
    if bp_bundles:
        for b in reversed(bp_bundles):
            if (b / "summary.json").is_file():
                return b
    expected_scene = BP_FUN_PROOF_SCENARIOS.get(bp)
    for prefix in BP_ANCHOR_PREFIXES.get(bp, []):
        candidates = sorted(bundles_root.glob(f"{prefix}_*"))
        for c in reversed(candidates):
            manifest_path = c / "run_manifest.json"
            if not manifest_path.is_file():
                continue
            try:
                manifest = json.loads(manifest_path.read_text())
            except (OSError, json.JSONDecodeError):
                continue
            if (manifest.get("run_mode") or "").lower() == "headless-smoke":
                continue
            if expected_scene:
                scene_id = (manifest.get("scene") or {}).get("id")
                if scene_id and scene_id != expected_scene:
                    other_bp_scenes = set(BP_FUN_PROOF_SCENARIOS.values()) - {expected_scene}
                    if scene_id in other_bp_scenes:
                        continue
            return c
    return None


def load_json(path: Path) -> Optional[dict]:
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError):
        return None


def event_type_counts(bundle_dir: Path) -> dict:
    summary = load_json(bundle_dir / "summary.json") or {}
    return (summary.get("event_counts") or {}).get("by_type", {}) or {}


def collect_actions_invoked(bundle_dir: Path) -> list:
    """Scan events.jsonl for control.command_accepted entries and collect the
    distinct method names invoked through cfctl. Used to populate the Hands
    column scaffold so the agent can confirm each action against a frame."""
    events_path = bundle_dir / "events.jsonl"
    if not events_path.is_file():
        return []
    methods = []
    seen = set()
    try:
        with events_path.open() as f:
            for line in f:
                try:
                    ev = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if ev.get("category") != "control":
                    continue
                if ev.get("event_type") != "command_accepted":
                    continue
                method = (ev.get("payload") or {}).get("method")
                if method and method not in seen:
                    seen.add(method)
                    methods.append(method)
    except OSError:
        return methods
    return methods


def render_report(bp: str, bundle_dir: Path, agent_id: str) -> str:
    manifest = load_json(bundle_dir / "run_manifest.json") or {}
    summary = load_json(bundle_dir / "summary.json") or {}
    perf = summary.get("performance") or {}
    counts = (summary.get("event_counts") or {}).get("by_type", {}) or {}
    captures_dir = bundle_dir / "captures"
    summary_grid = captures_dir / "summary_grid.png"
    capture_manifest = captures_dir / "capture_manifest.json"
    capture_summary_meta = summary.get("capture") or {}
    grid_meta = (capture_summary_meta or {}).get("summary_grid") or {}
    actions_invoked = collect_actions_invoked(bundle_dir)

    bp_label = bp.upper()
    goals = BP_GOALS.get(bp, [])
    fun_proof_scene = BP_FUN_PROOF_SCENARIOS.get(bp, "(unknown)")
    timestamp = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

    out = []
    out.append(f"## AI-Agent Self-Test Report ({bp_label})\n")
    out.append(f"- **Agent:** `{agent_id}`\n")
    out.append(f"- **Timestamp:** `{timestamp}`\n")
    out.append(f"- **Source bundle:** `{bundle_dir}`\n")
    out.append(f"- **Run id:** `{manifest.get('run_id') or summary.get('manifest_run_id') or '?'}`\n")
    out.append(f"- **Scenario:** `{(manifest.get('scene') or {}).get('id') or '?'}`\n")
    out.append(f"- **Tick rate:** `{manifest.get('tick_rate_hz') or perf.get('tick_rate_hz') or '?'} Hz`\n")
    out.append(f"- **Final sim checksum:** `{summary.get('final_sim_checksum') or '?'}`\n")
    if summary_grid.is_file():
        rel = summary_grid.relative_to(bundle_dir.parent.parent.parent) if bundle_dir.is_absolute() else summary_grid
        out.append(f"- **Summary grid:** `{rel}` ({grid_meta.get('frame_count', '?')} frames; non_blank_ratio=`{grid_meta.get('non_blank_ratio', '?')}`)\n")
    else:
        out.append(f"- **Summary grid:** _not present at `{summary_grid}`_ — Eyes axis cannot be confirmed without it.\n")
    if capture_manifest.is_file():
        # Bugbot 3212416395 caught the missing is_absolute() guard here that
        # crashed when --bundle was passed as a relative path. Mirror the
        # guard from line 246 so the report renders against any path shape.
        cap_rel = (
            capture_manifest.relative_to(bundle_dir.parent.parent.parent)
            if bundle_dir.is_absolute()
            else capture_manifest
        )
        out.append(f"- **Capture manifest:** `{cap_rel}`\n")
    out.append("\n")

    # Q1
    out.append("### Q1. What does this BP claim to deliver, in the project owner's words?\n\n")
    if goals:
        for g in goals:
            out.append(f"- {g}\n")
    else:
        out.append(f"_Add the verbatim goal statements from `docs/plan/spec/prototype-roadmap.md` for {bp_label}._\n")
    out.append("\n")

    # Q2
    out.append("### Q2. Does the playable scenario deliver Q1 end-to-end through cfctl-driven inputs?\n\n")
    out.append("Per cfctl action exercised in this bundle (from `events.jsonl` `control.command_accepted` rows):\n\n")
    out.append("| cfctl method | Hands (action invoked) | Eyes (summary_grid.png frame + agent prose) | Ears (events.jsonl + observe.once) | Verdict |\n")
    out.append("|---|---|---|---|---|\n")
    if actions_invoked:
        for method in actions_invoked:
            out.append(f"| `{method}` | yes (events.jsonl `control.command_accepted` row present) | _Agent: open summary_grid.png and write 1-2 sentences describing the visible state at the tick this method fired._ | _Agent: cite the structured event row + the observe.once field that confirms post-action state._ | _PASS / FAIL_ |\n")
    else:
        out.append("| _no command_accepted rows found in events.jsonl — bundle may be a headless-smoke run_ | | | | |\n")
    out.append("\n")

    # Q3
    out.append("### Q3. Does the visual presentation match the maturity level the BP promises?\n\n")
    out.append("_Agent: open `captures/summary_grid.png` with the `Read` tool. Pick at least 4 frames spanning the run's tick range. For each, write 1-2 sentences describing what is visually present (sprite positions, terrain shape, projectile trails, HUD numbers, mission-state cards, lighting/effects). Tie each observation back to the BP's promised maturity (e.g. \"M2 chunked terrain visible as carved tunnel through dirt column\" or \"M2.5 dirt shield mound between guard and reactor\"). Replace this placeholder with the prose._\n\n")

    # Q4
    out.append("### Q4. Does the simulation behavior match the project owner's stated feel?\n\n")
    out.append("_Agent: enumerate each \"feel claim\" from the BP's roadmap entry (Q1) and write 1-2 sentences confirming or refuting it from the captures + events. Examples for BP2: \"actors move with weight\" — yes/no; \"terrain refuses metal_nohook with a refusal reason event\" — yes/no; \"reactor takes damage from projectile-vs-AABB hits\" — yes/no; \"guard utility-scores tactics\" — yes/no. Replace this placeholder with the prose._\n\n")

    # Q5
    out.append("### Q5. Are there obvious inside-scope affordances the BP's text implies but the implementation skipped?\n\n")
    out.append("_Agent: list any inside-scope player-facing affordance / feedback state / cfctl action / replay event / failure path the BP's roadmap entry implies but that is not in the captures + events + observe + tests. Empty list = clean. If items remain, each is a `Needs Fixes` finding; write a one-line description + which existing milestone owns the fix._\n\n")
    out.append("- (none identified) — _replace with bullet list if non-empty_\n\n")

    # Q6
    out.append("### Q6. Did the BP regress any prior-BP feel/feature?\n\n")
    out.append("_Agent: run the prior-BP fun-proof scenarios under the new build (e.g., for BP2 review: re-run BP1's `micro_breach_win` + `micro_breach_loss` cfctl scripts). Confirm `summary_grid.png` looks the same as the BP1 exemplar AND `final_sim_checksum` matches the BP1 exemplar (or document the intentional change with a roadmap-approved DR reference). Write 2-3 sentences summarizing the regression check._\n\n")

    # Q7
    out.append("### Q7. What would a human playtester see in the first 30 seconds that the AI agent missed?\n\n")
    out.append("_Agent: be honest. List anything the captures + events + observe surface cannot capture but a human eyeballing the running game might (input lag, camera judder, sprite alignment off by 1 px, audio-visual sync issues if audio shipped, accessibility-flag visual side-effects). If nothing comes to mind, write \"no candidate gaps identified by AI agent — human playtest still recommended for novelty signal but not gating\". This question is the optional human-playtest's escape valve._\n\n")

    # Event signal pulled from events.jsonl by event_type
    out.append("### Run signal (auto-extracted from events.jsonl)\n\n")
    if counts:
        out.append("| event_type | count |\n|---|---|\n")
        for k, v in sorted(counts.items()):
            out.append(f"| `{k}` | {v} |\n")
    else:
        out.append("_No event_counts.by_type in summary.json._\n")
    out.append("\n")

    # Optional Human Playtest
    out.append("## Human Playtest Survey (optional confirmation)\n\n")
    out.append("_This section is OPTIONAL per `corefall/AGENTS.md` Build Point Closure Gate. The AI-Agent Self-Test Report above is the gating contract. The project owner may add a row here after playing the BP._\n\n")
    out.append(f"- **Question:** Did {bp_label} make the game more fun than the previous BP?\n")
    out.append(f"- **Reference summary grid:** `{summary_grid.relative_to(bundle_dir.parent.parent.parent) if summary_grid.is_file() and bundle_dir.is_absolute() else 'see captures/summary_grid.png'}`\n")
    out.append("- **Owner's answer:** _(empty until played)_\n")
    out.append("- **Concrete observations:** _(empty until played)_\n")
    return "".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description="Scaffold the AI-Agent Self-Test Report for a BP closure")
    parser.add_argument("--bp", required=True, help="Build Point id (bp0..bp12)")
    parser.add_argument("--bundle", type=Path, default=None, help="Run bundle dir; auto-discovered if omitted")
    parser.add_argument("--output", type=Path, default=None, help="Output notes.md path; stdout if omitted")
    parser.add_argument("--agent", default=os.environ.get("AGENT_ID", "Droid (model unspecified)"),
                        help="Agent identity string (e.g. 'Droid (claude-sonnet-4.5-20250522)')")
    parser.add_argument("--repo-root", type=Path, default=None,
                        help="Corefall repo root (defaults to script's grandparent dir)")
    args = parser.parse_args()

    if args.repo_root is None:
        args.repo_root = Path(__file__).resolve().parents[2]

    bp = args.bp.lower()
    if bp not in BP_GOALS:
        print(f"agent_self_test_report: unknown BP '{bp}'. Known: {sorted(BP_GOALS)}", file=sys.stderr)
        return 2

    bundle = args.bundle if args.bundle else find_bundle(args.repo_root, bp)
    if bundle is None or not bundle.is_dir():
        print(f"agent_self_test_report: no fun-proof bundle found for {bp.upper()}.", file=sys.stderr)
        print(f"  Searched: prototype_runs/native/{bp}_*, then anchor prefixes {BP_ANCHOR_PREFIXES.get(bp, [])}", file=sys.stderr)
        print("  Run the BP's fun-proof cfctl script via cf-e2e --capture-grid first.", file=sys.stderr)
        return 2

    body = render_report(bp, bundle, args.agent)
    if args.output is None:
        sys.stdout.write(body)
        return 0
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(body)
    print(f"agent_self_test_report: wrote {len(body)} bytes to {args.output}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
