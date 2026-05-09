#!/usr/bin/env bash
# bp_close_loop.sh — AI-Agent Test-Improvement Loop driver for BP closure.
#
# Per `.claude/skills/corefall-review/SKILL.md` §AI-Agent Test-Improvement Loop
# and `corefall/AGENTS.md` Build Point Closure Gate, every BP closes through a
# self-correcting loop driven by the AI agent (Droid). This script orchestrates
# the loop's mechanical phases; the agent (you, reading this output) makes the
# semantic decisions between iterations.
#
# Loop:
#
#   while not done:
#       1. coverage check  (bp_test_coverage.py)
#          → 0 gaps?
#            yes: continue
#            no:  agent reads gap report, scaffolds missing tests / extends
#                 engine to emit missing events / fixes broken cargo tests,
#                 then loops back to (1)
#
#       2. cargo build + clippy + workspace tests
#          → all green?
#            yes: continue
#            no:  agent diagnoses + fixes, loops back to (1) so any test
#                 surface change is re-coverage-checked
#
#       3. self_play_sweep
#          → 13/13 PASS?
#            yes: continue
#            no:  agent reads the failing sweep row's stdout/stderr,
#                 diagnoses code vs test bug, fixes, loops to (1)
#
#       4. LLM grading scaffold for fun-proof bundles produced this iteration
#          → all bundles have grading.json?
#            yes: continue
#            no:  scaffold is automatic, this should always be yes
#
#       5. Agent fills in grading.json prose for each fun-proof bundle
#          (this is the LLM-graded test step; the agent reads
#          summary_grid.png + events.jsonl + observe.once and writes
#          per-dimension scores + prose + verdicts)
#          → bp_close_loop.sh PAUSES here and prints instructions
#            for the agent to run llm_grade_run.py validate; the agent
#            edits the grading.json + re-runs the validator until PASS
#
#       6. validate filled grading.json
#          → aggregate >= minimum_aggregate AND every per-dim >= min OR
#            FUTURE_OWNED?
#            yes: continue
#            no:  agent improves the LOWEST scoring dim:
#              - if it's a code gap (e.g. missing visual feedback): fix code
#              - if it's a test gap (e.g. wrong scenario / wrong dim): fix test
#              - if it's FUTURE_OWNED legitimately: classify it as such with
#                an owning milestone in `future_owners_if_blocked` field
#              loop back to (1)
#
#       7. /corefall-review BP<N> verdict = Accept?
#          → yes: DONE; print PR URL hint
#            no:  agent fixes findings + loops back to (1)
#
#   When done: branch is ready for `git push` + PR; the script prints the
#   one-line gh pr command.
#
# The script is INFRASTRUCTURE; the agent does the semantic work between
# iterations. This script makes each iteration's mechanical phases run in a
# defined order with a stable verdict report so the agent can decide what to
# fix next without re-deriving the loop from scratch every time.
#
# Usage:
#   bash game/tools/bp_close_loop.sh bp2
#
# Optional env:
#   AGENT_ID            agent identity recorded in grading.json (default: $AGENT_ID or "Droid (model unspecified)")
#   SKIP_SWEEP          if set, skip the self_play_sweep run (faster local
#                        coverage check; CI / pre-PR run should NOT skip)
#   SKIP_GRADE          if set, skip the LLM grading scaffold + validation
#                        (use only for the very first iteration when no
#                        bundles exist yet; subsequent iterations must run
#                        grading)
#   MAX_ITERATIONS      cap on loop attempts (default: read from
#                        bp<N>.test_manifest.json loop_thresholds, fallback 10)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
GAME_DIR="$REPO_ROOT/game"
BP="${1:-}"
if [[ -z "$BP" ]]; then
    echo "usage: bp_close_loop.sh <bp> (e.g. bp2)" >&2
    exit 2
fi

MANIFEST="$REPO_ROOT/game/content/build_points/${BP}.test_manifest.json"
if [[ ! -f "$MANIFEST" ]]; then
    echo "bp_close_loop: manifest not found: $MANIFEST" >&2
    exit 2
fi

UTC="$(date -u +%Y-%m-%dT%H-%M-%SZ)"
HASH="$(printf '%s' "$UTC$$" | shasum | head -c 8)"
LOOP_DIR="$REPO_ROOT/prototype_runs/native/${BP}_loop_${UTC}_${HASH}"
mkdir -p "$LOOP_DIR"

LOG="$LOOP_DIR/loop.log"
VERDICT="$LOOP_DIR/verdict.json"

log() { printf '%s %s\n' "[$(date -u +%H:%M:%S)]" "$*" | tee -a "$LOG"; }
fail_with() { log "FAIL: $*"; exit 1; }

log "BP $BP closure loop starting"
log "  manifest: $MANIFEST"
log "  loop dir: $LOOP_DIR"

# ---------------------------------------------------------------------------
# Phase 1 — Coverage check
# ---------------------------------------------------------------------------
log "Phase 1: bp_test_coverage.py $BP"
COVERAGE_JSON="$LOOP_DIR/coverage.json"
if python3 "$REPO_ROOT/game/tools/bp_test_coverage.py" "$BP" --json > "$COVERAGE_JSON" 2>>"$LOG"; then
    GAPS=0
else
    GAPS=$(python3 -c "import json,sys; d=json.load(open('$COVERAGE_JSON')); print(d['summary']['total_gaps'])" 2>/dev/null || echo "?")
fi
log "  → gaps: $GAPS"
if [[ "$GAPS" != "0" ]]; then
    log "  COVERAGE GAPS DETECTED — agent must fix before continuing the loop:"
    python3 "$REPO_ROOT/game/tools/bp_test_coverage.py" "$BP" 2>&1 | sed 's/^/    /' | tee -a "$LOG"
    log ""
    log "  Next iteration: read $COVERAGE_JSON, scaffold/fix the gaps, re-run this loop."
    PHASE1=FAIL
else
    PHASE1=PASS
fi

# ---------------------------------------------------------------------------
# Phase 2 — cargo build + clippy + workspace tests
# ---------------------------------------------------------------------------
log "Phase 2: cargo fmt --check + clippy -D warnings + cargo test --workspace"
PHASE2=PASS
(cd "$GAME_DIR" && cargo fmt --all --check) >>"$LOG" 2>&1 || { PHASE2=FAIL; log "  → fmt drift"; }
if [[ "$PHASE2" == "PASS" ]]; then
    (cd "$GAME_DIR" && cargo clippy --workspace --all-targets -- -D warnings) >>"$LOG" 2>&1 \
        || { PHASE2=FAIL; log "  → clippy/build error (see $LOG)"; }
fi
if [[ "$PHASE2" == "PASS" ]]; then
    (cd "$GAME_DIR" && cargo test --workspace) >>"$LOG" 2>&1 \
        || { PHASE2=FAIL; log "  → cargo test failure (see $LOG)"; }
fi
log "  → $PHASE2"

# ---------------------------------------------------------------------------
# Phase 3 — self_play_sweep
# ---------------------------------------------------------------------------
PHASE3="SKIP"
if [[ -z "${SKIP_SWEEP:-}" ]]; then
    log "Phase 3: bash game/tools/self_play_sweep.sh"
    if bash "$REPO_ROOT/game/tools/self_play_sweep.sh" >>"$LOG" 2>&1; then
        PHASE3=PASS
        log "  → 13/13 PASS"
    else
        PHASE3=FAIL
        log "  → sweep failed (see $LOG)"
    fi
else
    log "Phase 3: SKIPPED (SKIP_SWEEP set)"
fi

# ---------------------------------------------------------------------------
# Phase 4-6 — LLM grading
# ---------------------------------------------------------------------------
PHASE4="SKIP"
PHASE5="PENDING_AGENT"
PHASE6="SKIP"
if [[ -z "${SKIP_GRADE:-}" ]]; then
    log "Phase 4: LLM grading scaffolds for fun-proof bundles produced this hour"
    SCAFFOLD_COUNT=0
    for bundle in "$REPO_ROOT/prototype_runs/native"/m*_$(date -u +%Y-%m-%dT%H)*; do
        if [[ -d "$bundle" ]] && [[ -f "$bundle/run_manifest.json" ]] && [[ ! -f "$bundle/grading.json" ]]; then
            python3 "$REPO_ROOT/game/tools/llm_grade_run.py" scaffold \
                --bundle "$bundle" \
                --agent "${AGENT_ID:-Droid (loop-scaffolded)}" \
                >>"$LOG" 2>&1 && SCAFFOLD_COUNT=$((SCAFFOLD_COUNT + 1))
        fi
    done
    log "  → scaffolded $SCAFFOLD_COUNT new grading.json files"
    PHASE4=PASS

    log "Phase 5: agent fills grading.json prose for each fun-proof bundle"
    log "  AGENT ACTION REQUIRED:"
    log "    1. Read each new grading.json under prototype_runs/native/m*_$(date -u +%Y-%m-%d)*"
    log "    2. For each dimension, read the evidence_required (frames, events, observe fields)"
    log "    3. Fill in score (0-10) + evidence_read (audit trail) + prose (>=30 chars) + verdict"
    log "    4. Run: python3 game/tools/llm_grade_run.py validate --bundle <dir> --write"
    log "    5. When validate exits 0, this phase is complete"
    log ""

    log "Phase 6: at least one grading.json per fun_proof_scenario must validate PASS"
    # Read the fun_proof_scenarios from the manifest + check that for each
    # scenario id, at least one bundle anywhere under prototype_runs/native/
    # has a passing grading.json. Empty scaffolds are skipped (they're
    # instructions to the agent, not failures).
    PHASE6=PASS
    SCENARIOS_JSON=$(python3 -c "
import json
m=json.load(open('$MANIFEST'))
ids=[s['id'] for s in m.get('fun_proof_scenarios',[])]
print('\n'.join(ids))
")
    while IFS= read -r SCEN; do
        [[ -z "$SCEN" ]] && continue
        FOUND_PASS=0
        FOUND_ANY=0
        for bundle in $(ls -1dt "$REPO_ROOT/prototype_runs/native"/*_*/ 2>/dev/null); do
            [[ -d "$bundle" ]] || continue
            [[ -f "$bundle/grading.json" ]] || continue
            G_SCEN=$(python3 -c "import json; print(json.load(open('${bundle%/}/grading.json')).get('scenario_id',''))" 2>/dev/null || echo "")
            [[ "$G_SCEN" == "$SCEN" ]] || continue
            FOUND_ANY=$((FOUND_ANY + 1))
            if python3 "$REPO_ROOT/game/tools/llm_grade_run.py" validate --bundle "${bundle%/}" --write >/dev/null 2>>"$LOG"; then
                FOUND_PASS=$((FOUND_PASS + 1))
                log "  → $SCEN: PASS via ${bundle%/}/grading.json"
                break
            fi
        done
        if [[ "$FOUND_PASS" == "0" ]]; then
            PHASE6=FAIL
            if [[ "$FOUND_ANY" == "0" ]]; then
                log "  → $SCEN: FAIL — no grading.json scaffolded yet for any bundle of this scenario"
            else
                log "  → $SCEN: FAIL — $FOUND_ANY scaffold(s) exist but none filled in by an agent yet"
                log "     AGENT ACTION: open one of those scaffolds, fill score/prose/evidence_read/verdict per dimension,"
                log "                   then run \`python3 game/tools/llm_grade_run.py validate --bundle <dir> --write\`"
            fi
        fi
    done <<< "$SCENARIOS_JSON"
    log "  → $PHASE6"
else
    log "Phase 4-6: SKIPPED (SKIP_GRADE set)"
fi

# ---------------------------------------------------------------------------
# Aggregate verdict
# ---------------------------------------------------------------------------
ALL_PASS="false"
if [[ "$PHASE1" == "PASS" && "$PHASE2" == "PASS" && ( "$PHASE3" == "PASS" || "$PHASE3" == "SKIP" ) && ( "$PHASE6" == "PASS" || "$PHASE6" == "SKIP" ) ]]; then
    ALL_PASS="true"
fi

cat > "$VERDICT" <<EOF
{
  "schema_version": "cf-bp-close-loop.v1",
  "bp": "$BP",
  "utc": "$UTC",
  "loop_dir": "$LOOP_DIR",
  "phases": {
    "coverage": "$PHASE1",
    "build_lint_test": "$PHASE2",
    "self_play_sweep": "$PHASE3",
    "grading_scaffold": "$PHASE4",
    "grading_filled": "$PHASE5",
    "grading_validate": "$PHASE6"
  },
  "all_phases_pass_or_skipped": $ALL_PASS,
  "next_step": "$([[ "$ALL_PASS" == "true" ]] && echo "Run /corefall-review $BP; if Accept, push branch + open PR" || echo "Agent reads $LOG, fixes findings, re-runs this loop")"
}
EOF

log "═══════════════════════════════════════════════"
log "Verdict written to $VERDICT"
log ""
cat "$VERDICT" | tee -a "$LOG"
log ""
if [[ "$ALL_PASS" == "true" ]]; then
    log "ALL PHASES PASSED OR SKIPPED — agent next: /corefall-review $BP"
    log "  When the review verdict is Accept, push branch + open PR:"
    log "    git push -u origin \$(git symbolic-ref --short HEAD)"
    log "    gh pr create --title 'BP$BP: ...' --body '...'"
    exit 0
else
    log "LOOP NOT COMPLETE — agent must fix findings + re-run this loop."
    exit 1
fi
