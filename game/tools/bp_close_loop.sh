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
#   SKIP_SWEEP          if set, skip the self_play_sweep run for local
#                        diagnosis only. Any skipped proof phase forces the
#                        aggregate verdict false.
#   SKIP_GRADE          if set, skip LLM grading for local diagnosis only.
#                        Any skipped proof phase forces the aggregate verdict
#                        false.
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

# Audit fix round-5 (2026-05-10): Phase 4/5/6 must bind to bundles created
# DURING THIS loop iteration, not historical bundles. We capture the loop
# start epoch so subsequent phases can filter `prototype_runs/native/*` down
# to the fresh bundles produced by Phase 3's sweep + any in-loop cf-app
# invocations. Historical bundles' gradings cannot launder a fresh bundle's
# closure verdict.
LOOP_START_EPOCH=$(date -u +%s)
FRESH_BUNDLES_FILE="$LOOP_DIR/fresh_bundles.txt"
REDUNDANT_BUNDLES_FILE="$LOOP_DIR/redundant_fresh_bundles.txt"
: > "$FRESH_BUNDLES_FILE"
: > "$REDUNDANT_BUNDLES_FILE"
log "  loop start epoch: $LOOP_START_EPOCH (Unix UTC seconds)"

HEAD_SHA_FULL=$(git -C "$REPO_ROOT" rev-parse HEAD 2>/dev/null || echo "")
HEAD_SHA12=${HEAD_SHA_FULL:0:12}
CURRENT_DIRTY=false
CURRENT_FINGERPRINT=""
if [[ -n "$(git -C "$REPO_ROOT" status --porcelain=v1 2>/dev/null || true)" ]]; then
    CURRENT_DIRTY=true
    CURRENT_FINGERPRINT=$(cd "$GAME_DIR" && cargo run -q -p cf-control --example worktree_fingerprint 2>>"$LOG" || true)
    if [[ -z "$CURRENT_FINGERPRINT" ]]; then
        fail_with "current checkout is dirty but worktree fingerprint could not be computed"
    fi
fi
log "  HEAD commit (12-char): $HEAD_SHA12"
log "  current worktree dirty: $CURRENT_DIRTY"
if [[ "$CURRENT_DIRTY" == "true" ]]; then
    log "  current worktree fingerprint: $CURRENT_FINGERPRINT"
fi

find_valid_current_proof() {
    local scenario="$1"
    local reference="${2:-}"
    local args=(
        "$REPO_ROOT/game/tools/current_bundle_proof.py" find
        --root "$REPO_ROOT/prototype_runs/native"
        --head-sha12 "$HEAD_SHA12"
        --current-dirty "$CURRENT_DIRTY"
        --current-fingerprint "$CURRENT_FINGERPRINT"
        --scenario "$scenario"
    )
    if [[ -n "$reference" ]]; then
        args+=(--reference "$reference")
    fi
    local candidate
    while IFS= read -r candidate; do
        [[ -d "$candidate" ]] || continue
        if python3 "$REPO_ROOT/game/tools/llm_grade_run.py" validate --bundle "$candidate" --write >/dev/null 2>>"$LOG"; then
            printf '%s\n' "$candidate"
            return 0
        fi
    done < <(python3 "${args[@]}" 2>>"$LOG")
    return 1
}

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
        log "  → PASS"
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
    # Audit fix round-5 (2026-05-10): rebuild the fresh-bundle set from
    # filesystem mtime against $LOOP_START_EPOCH so subsequent phases never
    # mistake a historical bundle's grading for proof of THIS iteration's
    # behavior. The reviewer caught a real failure mode: previously Phase 4
    # scanned `m*_<hour>*` and Phase 6 scanned every bundle ever produced —
    # so an old hand-graded bundle could pass Phase 6 even after the loop's
    # current sweep produced a fresh bundle that NOBODY graded. The fix:
    # bind every grading phase to the bundles whose run_manifest.json was
    # written AT OR AFTER LOOP_START_EPOCH.
    log "Phase 4: identifying fresh fun-proof bundles produced by THIS loop iteration"
    : > "$FRESH_BUNDLES_FILE"
    SCEN_LIST=$(python3 -c "
import json
m=json.load(open('$MANIFEST'))
print(' '.join(s['id'] for s in m.get('fun_proof_scenarios',[])))
")
    for bundle in "$REPO_ROOT/prototype_runs/native"/m*_*/; do
        [[ -d "$bundle" ]] || continue
        manifest_file="${bundle%/}/run_manifest.json"
        [[ -f "$manifest_file" ]] || continue
        BUNDLE_EPOCH=$(date -r "$manifest_file" -u +%s 2>/dev/null || stat -f %m "$manifest_file" 2>/dev/null || echo "0")
        if [[ "$BUNDLE_EPOCH" -ge "$LOOP_START_EPOCH" ]]; then
            BUNDLE_SCEN=$(python3 -c "import json; print(json.load(open('$manifest_file')).get('scene',{}).get('id',''))" 2>/dev/null || echo "")
            if [[ -n "$BUNDLE_SCEN" ]] && [[ " $SCEN_LIST " == *" $BUNDLE_SCEN "* ]]; then
                echo "${bundle%/}" >> "$FRESH_BUNDLES_FILE"
            fi
        fi
    done
    FRESH_COUNT=$(wc -l < "$FRESH_BUNDLES_FILE" | tr -d ' ')
    log "  → identified $FRESH_COUNT fresh fun-proof bundle(s) for grading"

    if [[ "$FRESH_COUNT" == "0" ]]; then
        PHASE4=FAIL
        PHASE5=FAIL
        log "  → FAIL — no fresh fun-proof bundles were produced by this loop"
        log "     Closure-quality BP runs must execute the sweep and produce current evidence."
    else
        PHASE4=PASS
    fi

    log "Phase 4: LLM grading scaffolds for THIS loop's fresh fun-proof bundles"
    SCAFFOLD_COUNT=0
    REDUNDANT_COUNT=0
    if [[ "$PHASE4" == "PASS" ]]; then
        while IFS= read -r bundle; do
            [[ -d "$bundle" ]] || continue
            BUNDLE_SCEN=$(python3 -c "import json; print(json.load(open('$bundle/run_manifest.json')).get('scene',{}).get('id',''))" 2>/dev/null || echo "")
            CANONICAL_PROOF=$(find_valid_current_proof "$BUNDLE_SCEN" "$bundle" || true)
            if [[ -n "$CANONICAL_PROOF" && "$CANONICAL_PROOF" != "$bundle" ]]; then
                echo "$bundle|$CANONICAL_PROOF" >> "$REDUNDANT_BUNDLES_FILE"
                REDUNDANT_COUNT=$((REDUNDANT_COUNT + 1))
                continue
            fi
            if [[ ! -f "$bundle/grading.json" ]]; then
                python3 "$REPO_ROOT/game/tools/llm_grade_run.py" scaffold \
                    --bundle "$bundle" \
                    --agent "${AGENT_ID:-Droid (loop-scaffolded)}" \
                    >>"$LOG" 2>&1 && SCAFFOLD_COUNT=$((SCAFFOLD_COUNT + 1))
            fi
        done < "$FRESH_BUNDLES_FILE"
    fi
    log "  → scaffolded $SCAFFOLD_COUNT new grading.json files for THIS loop's fresh bundles"
    log "  → recognized $REDUNDANT_COUNT fresh bundle(s) already covered by a current-code graded equivalent"

    log "Phase 5: agent fills grading.json prose for THIS loop's fresh bundles"
    # Phase 5 = every fresh bundle from THIS loop has either (a) its own
    # filled+valid grading.json, or (b) a same-scenario/same-settings bundle
    # whose build fingerprint matches the current checkout and whose grading
    # validates. Case (b) is not laundering: it is exact current-code reuse for
    # deterministic duplicate sweep rows and avoids creating cloned prose files.
    PENDING_BUNDLES=()
    if [[ "$PHASE4" == "PASS" ]]; then
        PHASE5=PASS
        while IFS= read -r bundle; do
            [[ -d "$bundle" ]] || continue
            if [[ -f "$bundle/grading.json" ]] && python3 "$REPO_ROOT/game/tools/llm_grade_run.py" validate --bundle "$bundle" --write >/dev/null 2>>"$LOG"; then
                continue
            fi
            BUNDLE_SCEN=$(python3 -c "import json; print(json.load(open('$bundle/run_manifest.json')).get('scene',{}).get('id',''))" 2>/dev/null || echo "")
            CANONICAL_PROOF=$(awk -F'|' -v b="$bundle" '$1 == b {print $2; exit}' "$REDUNDANT_BUNDLES_FILE")
            if [[ -z "$CANONICAL_PROOF" ]]; then
                CANONICAL_PROOF=$(find_valid_current_proof "$BUNDLE_SCEN" "$bundle" || true)
            fi
            if [[ -n "$CANONICAL_PROOF" ]]; then
                log "  → $bundle: PASS via current-code equivalent $CANONICAL_PROOF/grading.json"
                continue
            fi
            PENDING_BUNDLES+=("$bundle")
        done < "$FRESH_BUNDLES_FILE"
    fi
    if [[ "$PHASE5" == "FAIL" ]]; then
        log "  → FAIL (no fresh fun-proof bundles to grade)"
    elif [[ ${#PENDING_BUNDLES[@]} -gt 0 ]]; then
        PHASE5=PENDING_AGENT
        log "  → PENDING_AGENT — ${#PENDING_BUNDLES[@]} fun-proof grading.json files in this iteration are unfilled or invalid:"
        for b in "${PENDING_BUNDLES[@]}"; do
            log "      $b/grading.json"
        done
        log "  AGENT ACTION REQUIRED for each PENDING bundle:"
        log "    1. Read each pending grading.json"
        log "    2. For each dimension, read the evidence_required (frames, events, observe fields)"
        log "    3. Fill in score (0-10) + evidence_read (audit trail) + prose (>=30 chars) + verdict"
        log "    4. Run: python3 game/tools/llm_grade_run.py validate --bundle <dir> --write"
        log "    5. Re-run this loop until all bundles validate"
    else
        log "  → PASS (every fun-proof grading.json from this iteration validates)"
    fi
    log ""

    log "Phase 6: at least one CURRENT-CODE bundle per fun_proof_scenario must validate PASS"
    # Current proof is either a fresh validating bundle from this loop or an
    # earlier validating bundle whose build metadata proves the same current
    # source state. Clean checkouts match HEAD exactly. Dirty checkouts must
    # match the worktree fingerprint; commit_sha[:12] alone is rejected.
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
        FRESH_FOR_SCEN=0

        # First try: any FRESH bundle with a filled grading.
        while IFS= read -r bundle; do
            [[ -d "$bundle" ]] || continue
            [[ -f "$bundle/grading.json" ]] || { FRESH_FOR_SCEN=$((FRESH_FOR_SCEN + 1)); continue; }
            G_SCEN=$(python3 -c "import json; print(json.load(open('$bundle/grading.json')).get('scenario_id',''))" 2>/dev/null || echo "")
            [[ "$G_SCEN" == "$SCEN" ]] || continue
            FRESH_FOR_SCEN=$((FRESH_FOR_SCEN + 1))
            if python3 "$REPO_ROOT/game/tools/llm_grade_run.py" validate --bundle "$bundle" --write >/dev/null 2>>"$LOG"; then
                FOUND_PASS=$((FOUND_PASS + 1))
                log "  → $SCEN: PASS via $bundle/grading.json (fresh, this loop)"
                break
            fi
        done < "$FRESH_BUNDLES_FILE"

        if [[ "$FOUND_PASS" == "0" ]]; then
            CANONICAL_PROOF=$(find_valid_current_proof "$SCEN" || true)
            if [[ -n "$CANONICAL_PROOF" ]]; then
                FOUND_PASS=$((FOUND_PASS + 1))
                if [[ "$CURRENT_DIRTY" == "true" ]]; then
                    log "  → $SCEN: PASS via $CANONICAL_PROOF/grading.json (worktree fingerprint matches current checkout)"
                else
                    log "  → $SCEN: PASS via $CANONICAL_PROOF/grading.json (clean HEAD $HEAD_SHA12)"
                fi
            fi
        fi

        if [[ "$FOUND_PASS" == "0" ]]; then
            PHASE6=FAIL
            if [[ "$FRESH_FOR_SCEN" == "0" ]]; then
                log "  → $SCEN: FAIL — no fresh OR current-source graded bundle for this scenario"
                log "     AGENT ACTION: run cf-e2e for this scenario, then fill the freshly-scaffolded grading.json"
                log "                   from THIS bundle's actual events.jsonl + summary.json + captures (no laundering),"
                log "                   then run \`python3 game/tools/llm_grade_run.py validate --bundle <dir> --write\`"
            else
                log "  → $SCEN: FAIL — $FRESH_FOR_SCEN fresh bundle(s) for this scenario but none have a filled+validating grading"
                log "     AGENT ACTION: open the fresh bundle's grading.json, fill score/prose/evidence_read/verdict per dimension"
                log "                   from THIS bundle's actual events.jsonl + summary.json + captures (do NOT clone older grading),"
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
# Closure quality has no waiver aggregate: SKIP_SWEEP/SKIP_GRADE are useful
# while iterating, but a BP closeout verdict requires coverage, build/lint/test,
# sweep, grading scaffold, grading filled, and grading validate all to PASS.
# This deliberately keeps the legacy JSON key name for downstream parsers while
# making skipped proof phases non-closing.
ALL_PASS="false"
if [[ "$PHASE1" == "PASS" && "$PHASE2" == "PASS" \
      && "$PHASE3" == "PASS" && "$PHASE4" == "PASS" \
      && "$PHASE5" == "PASS" && "$PHASE6" == "PASS" ]]; then
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
  "next_step": "$([[ "$ALL_PASS" == "true" ]] && echo "Run /corefall-review $BP; if Accept, push branch + open PR" || echo "Agent reads $LOG, fixes findings, re-runs this loop without skipped proof phases")"
}
EOF

log "═══════════════════════════════════════════════"
log "Verdict written to $VERDICT"
log ""
cat "$VERDICT" | tee -a "$LOG"
log ""
if [[ "$ALL_PASS" == "true" ]]; then
    log "ALL REQUIRED PHASES PASSED — agent next: /corefall-review $BP"
    log "  When the review verdict is Accept, push branch + open PR:"
    log "    git push -u origin \$(git symbolic-ref --short HEAD)"
    log "    gh pr create --title 'BP$BP: ...' --body '...'"
    exit 0
else
    log "LOOP NOT COMPLETE — agent must fix findings + re-run this loop."
    exit 1
fi
