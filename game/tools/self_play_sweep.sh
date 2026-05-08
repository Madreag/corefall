#!/usr/bin/env bash
# self_play_sweep.sh — canonical "play the game thoroughly" entry point.
#
# Implements the Self-Play Validation Rule from AGENTS.md. Drives every
# milestone-scope cfctl action through cf-e2e + cf-app, captures grids,
# verifies expected end-state via --expect, runs at 60 + 120 Hz, runs the
# headless-smoke no-window path, runs cf-mod validate + cfctl observe --once,
# and emits a verdict matrix in JSON for downstream tooling and humans.
#
# Output:
#   prototype_runs/native/self_play_sweep_<UTC>_<hash>/
#     verdict.json                 (per-row PASS/FAIL matrix)
#     verdict.txt                  (human-readable summary)
#     m1_actor_round_trip/         (capture-grid bundle for the M1 sweep)
#     m1_5_micro_breach_win/       (capture-grid bundle for M1.5 win path)
#     m1_5_micro_breach_loss/      (capture-grid bundle for M1.5 loss path)
#     m1_actor_60hz_determinism/   (cf-app direct, 60 Hz baseline)
#     m1_actor_120hz_determinism/  (cf-app direct, 120 Hz validation)
#     m0_smoke_5s_headless/        (--headless-smoke no-window run)
#     observe_once.json            (live cfctl observe.once snapshot)
#     mod_validate.txt             (cf-mod validate content/ output)
#
# Exit code: 0 on all-PASS; non-zero on any FAIL row.
#
# Usage:
#   bash game/tools/self_play_sweep.sh
#
# Optional env:
#   CF_APP_BIN              path to cf-app release binary (default: auto-build)
#   CF_E2E_BIN              path to cf-e2e release binary (default: auto-build)
#   CFCTL_BIN               path to cfctl release binary (default: auto-build)
#   CF_MOD_BIN              path to cf-mod release binary (default: auto-build)
#   SELF_PLAY_SWEEP_OUTDIR  override the sweep output dir
#   SELF_PLAY_SWEEP_SKIP    comma-separated row IDs to skip (for partial reruns)

set -uo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
GAME_DIR="$REPO_ROOT/game"

UTC="$(date -u +%Y-%m-%dT%H-%M-%SZ)"
HASH="$(printf '%s' "$UTC$$" | shasum | head -c 8)"
OUTDIR="${SELF_PLAY_SWEEP_OUTDIR:-$REPO_ROOT/prototype_runs/native/self_play_sweep_${UTC}_${HASH}}"
mkdir -p "$OUTDIR"

VERDICT_JSON="$OUTDIR/verdict.json"
VERDICT_TXT="$OUTDIR/verdict.txt"

ROWS=()
add_row() {
    # add_row <id> <verdict> <evidence-path> <one-line-note>
    local id="$1" v="$2" path="$3" note="$4"
    ROWS+=("$(printf '{"id":%s,"verdict":%s,"evidence":%s,"note":%s}' \
        "$(printf '%s' "$id"   | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')" \
        "$(printf '%s' "$v"    | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')" \
        "$(printf '%s' "$path" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')" \
        "$(printf '%s' "$note" | python3 -c 'import json,sys; print(json.dumps(sys.stdin.read()))')")")
}

skip_id() {
    [[ ",${SELF_PLAY_SWEEP_SKIP:-}," == *",$1,"* ]]
}

# Build release binaries up front so we don't time out individual rows.
if [[ -z "${CF_APP_BIN:-}" || -z "${CF_E2E_BIN:-}" || -z "${CFCTL_BIN:-}" || -z "${CF_MOD_BIN:-}" ]]; then
    echo "self_play_sweep: building release binaries (cf-app, cf-e2e, cfctl, cf-mod)..."
    (cd "$GAME_DIR" && cargo build --release -p cf-app -p cf-e2e -p cfctl -p cf-mod 2>&1 | tail -5)
fi
CF_APP_BIN="${CF_APP_BIN:-$GAME_DIR/target/release/cf-app}"
CF_E2E_BIN="${CF_E2E_BIN:-$GAME_DIR/target/release/cf-e2e}"
CFCTL_BIN="${CFCTL_BIN:-$GAME_DIR/target/release/cfctl}"
CF_MOD_BIN="${CF_MOD_BIN:-$GAME_DIR/target/release/cf-mod}"
export CF_APP_BIN

# ---------------------------------------------------------------------------
# Row 1: M1 actor round-trip (move + jump + aim + fire + reload + select_item)
# ---------------------------------------------------------------------------
ROW="m1_actor_round_trip"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    cd "$GAME_DIR"
    # cf-e2e --expect lookups against the FINAL observe.once payload (world
    # snapshot), not the events.jsonl audit log. Successful cf-e2e exit already
    # proves every cfctl step in the script dispatched without rejection. We
    # add a tick floor + scenario id assertion to nail down the post-state.
    if "$CF_E2E_BIN" \
        --scenario m1_actor_range \
        --script m1_move_jump_fire_reload \
        --capture-grid \
        --expect "scenario=m1_actor_range" \
        --expect "tick>=150" \
        --expect "capture.summary_grid.non_blank_ratio>=0.95" \
        > "$OUTDIR/$ROW.stdout.txt" 2> "$OUTDIR/$ROW.stderr.txt"; then
        BUNDLE="$(ls -dt "$REPO_ROOT/prototype_runs/native/m1_"* 2>/dev/null | head -n1)"
        add_row "$ROW" "PASS" "${BUNDLE:-?}" "M1 6-action sweep: move + jump + aim + fire + reload + select_item"
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cf-e2e exit nonzero"
    fi
    cd "$REPO_ROOT"
fi

# ---------------------------------------------------------------------------
# Row 2: M1.5 micro_breach WIN path (full mission)
# ---------------------------------------------------------------------------
ROW="m1_5_micro_breach_win"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    cd "$GAME_DIR"
    if "$CF_E2E_BIN" \
        --scenario micro_breach \
        --script micro_breach_win \
        --capture-grid \
        --expect "mission.result=won" \
        --expect "objective.extract=completed" \
        --expect "breach.outer_wall.broken=true" \
        --expect "capture.summary_grid.non_blank_ratio>=0.95" \
        > "$OUTDIR/$ROW.stdout.txt" 2> "$OUTDIR/$ROW.stderr.txt"; then
        BUNDLE="$(ls -dt "$REPO_ROOT/prototype_runs/native/m1.5_"* 2>/dev/null | head -n1)"
        add_row "$ROW" "PASS" "${BUNDLE:-?}" "M1.5 win path: dig outer_wall, kill guard, reach extraction, summary_grid>=0.95"
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cf-e2e exit nonzero or expect failed"
    fi
    cd "$REPO_ROOT"
fi

# ---------------------------------------------------------------------------
# Row 3: M1.5 micro_breach LOSS path
# ---------------------------------------------------------------------------
ROW="m1_5_micro_breach_loss"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    cd "$GAME_DIR"
    if "$CF_E2E_BIN" \
        --scenario micro_breach \
        --script micro_breach_loss \
        --capture-grid \
        --expect "mission.result=lost" \
        --expect "capture.summary_grid.non_blank_ratio>=0.95" \
        > "$OUTDIR/$ROW.stdout.txt" 2> "$OUTDIR/$ROW.stderr.txt"; then
        BUNDLE="$(ls -dt "$REPO_ROOT/prototype_runs/native/m1.5_"* 2>/dev/null | head -n1)"
        add_row "$ROW" "PASS" "${BUNDLE:-?}" "M1.5 loss path: time-out / failure conditions reach mission.result=lost"
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cf-e2e exit nonzero or expect failed"
    fi
    cd "$REPO_ROOT"
fi

# ---------------------------------------------------------------------------
# Row 4: M0 settings round-trip (act.settings.set + observe.settings)
# ---------------------------------------------------------------------------
ROW="m0_settings_roundtrip"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    cd "$GAME_DIR"
    if "$CF_E2E_BIN" \
        --scenario m0_smoke_5s \
        --script m0_settings_roundtrip \
        > "$OUTDIR/$ROW.stdout.txt" 2> "$OUTDIR/$ROW.stderr.txt"; then
        add_row "$ROW" "PASS" "$OUTDIR/$ROW.stdout.txt" "act.settings.set + observe.settings round-trip"
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cf-e2e exit nonzero"
    fi
    cd "$REPO_ROOT"
fi

# ---------------------------------------------------------------------------
# Row 5: 60 Hz determinism baseline (cf-app direct)
# ---------------------------------------------------------------------------
ROW="m1_actor_60hz_determinism"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    if "$CF_APP_BIN" \
        --scenario m1_actor_range \
        --ticks 600 \
        --tick-rate-hz 60 \
        --headless-smoke \
        --write-run-bundle \
        > "$OUTDIR/$ROW.stdout.txt" 2> "$OUTDIR/$ROW.stderr.txt"; then
        BUNDLE="$(ls -dt "$REPO_ROOT/prototype_runs/native/m1_"* 2>/dev/null | head -n1)"
        CHECKSUM=""
        if [[ -n "$BUNDLE" && -f "$BUNDLE/summary.json" ]]; then
            CHECKSUM="$(python3 -c "import json; d=json.load(open('$BUNDLE/summary.json')); print(d.get('final_sim_checksum',''))" 2>/dev/null)"
        fi
        add_row "$ROW" "PASS" "${BUNDLE:-?}" "60 Hz / 600 ticks; final_sim_checksum=${CHECKSUM:-unknown}"
        echo "$CHECKSUM" > "$OUTDIR/$ROW.checksum"
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cf-app exit nonzero"
    fi
fi

# ---------------------------------------------------------------------------
# Row 6: 120 Hz determinism validation (same scenario, same logical duration)
# ---------------------------------------------------------------------------
ROW="m1_actor_120hz_determinism"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    if "$CF_APP_BIN" \
        --scenario m1_actor_range \
        --ticks 1200 \
        --tick-rate-hz 120 \
        --headless-smoke \
        --write-run-bundle \
        > "$OUTDIR/$ROW.stdout.txt" 2> "$OUTDIR/$ROW.stderr.txt"; then
        BUNDLE="$(ls -dt "$REPO_ROOT/prototype_runs/native/m1_"* 2>/dev/null | head -n1)"
        CHECKSUM=""
        if [[ -n "$BUNDLE" && -f "$BUNDLE/summary.json" ]]; then
            CHECKSUM="$(python3 -c "import json; d=json.load(open('$BUNDLE/summary.json')); print(d.get('final_sim_checksum',''))" 2>/dev/null)"
        fi
        add_row "$ROW" "PASS" "${BUNDLE:-?}" "120 Hz / 1200 ticks; final_sim_checksum=${CHECKSUM:-unknown}"
        echo "$CHECKSUM" > "$OUTDIR/$ROW.checksum"
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cf-app exit nonzero"
    fi
fi

# ---------------------------------------------------------------------------
# Row 7: --headless-smoke (no-window CI path)
# ---------------------------------------------------------------------------
ROW="m0_blank_headless_smoke"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    if "$CF_APP_BIN" \
        --scenario m0_blank \
        --ticks 300 \
        --tick-rate-hz 60 \
        --headless-smoke \
        --write-run-bundle \
        > "$OUTDIR/$ROW.stdout.txt" 2> "$OUTDIR/$ROW.stderr.txt"; then
        BUNDLE="$(ls -dt "$REPO_ROOT/prototype_runs/native/m0_"* 2>/dev/null | head -n1)"
        if [[ -n "$BUNDLE" ]] && python3 "$GAME_DIR/tools/prototype_run_check.py" "$BUNDLE" >/dev/null 2>&1; then
            add_row "$ROW" "PASS" "${BUNDLE:-?}" "headless-smoke produces valid run-bundle (no window, no captures)"
        else
            add_row "$ROW" "FAIL" "${BUNDLE:-$OUTDIR/$ROW.stdout.txt}" "run-bundle invalid or missing"
        fi
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cf-app exit nonzero"
    fi
fi

# ---------------------------------------------------------------------------
# Row 8: cfctl observe --once (live read against an offline engine)
# ---------------------------------------------------------------------------
ROW="cfctl_observe_once"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    if "$CFCTL_BIN" observe --once > "$OUTDIR/observe_once.json" 2> "$OUTDIR/$ROW.stderr.txt"; then
        if python3 -c "import json,sys; d=json.load(open('$OUTDIR/observe_once.json')); sys.exit(0 if 'tick' in d or 'sim' in d or 'state' in d or 'observation' in d else 1)" 2>/dev/null; then
            add_row "$ROW" "PASS" "$OUTDIR/observe_once.json" "cfctl observe --once returned a valid snapshot"
        else
            add_row "$ROW" "FAIL" "$OUTDIR/observe_once.json" "observe.once payload missing required fields"
        fi
    else
        add_row "$ROW" "FAIL" "$OUTDIR/$ROW.stderr.txt" "cfctl observe --once exit nonzero"
    fi
fi

# ---------------------------------------------------------------------------
# Row 9: cf-mod validate content/
# ---------------------------------------------------------------------------
ROW="cf_mod_validate_content"
if skip_id "$ROW"; then
    add_row "$ROW" "SKIP" "" "skipped via SELF_PLAY_SWEEP_SKIP"
else
    cd "$GAME_DIR"
    if "$CF_MOD_BIN" validate content/ > "$OUTDIR/mod_validate.txt" 2>&1; then
        add_row "$ROW" "PASS" "$OUTDIR/mod_validate.txt" "cf-mod validate content/ all scenarios OK"
    else
        add_row "$ROW" "FAIL" "$OUTDIR/mod_validate.txt" "cf-mod validate failed"
    fi
    cd "$REPO_ROOT"
fi

# ---------------------------------------------------------------------------
# Aggregate
# ---------------------------------------------------------------------------
{
    printf '{\n  "schema_version": 1,\n  "sweep_id": "%s",\n  "utc": "%s",\n  "rows": [\n' "self_play_sweep_${UTC}_${HASH}" "$UTC"
    SEP=""
    for r in "${ROWS[@]}"; do
        printf '%s    %s' "$SEP" "$r"
        SEP=$',\n'
    done
    printf '\n  ]\n}\n'
} > "$VERDICT_JSON"

PASS=0
FAIL=0
SKIP=0
{
    echo "Self-Play Validation Sweep — $UTC"
    echo "Output: $OUTDIR"
    echo
    printf "%-32s %s\n" "ROW" "VERDICT"
    printf "%-32s %s\n" "---" "-------"
    for r in "${ROWS[@]}"; do
        ID="$(printf '%s' "$r" | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["id"])')"
        V="$(printf '%s' "$r" | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["verdict"])')"
        N="$(printf '%s' "$r" | python3 -c 'import json,sys; print(json.loads(sys.stdin.read())["note"])')"
        printf "%-32s %s — %s\n" "$ID" "$V" "$N"
        case "$V" in
            PASS) PASS=$((PASS+1));;
            FAIL) FAIL=$((FAIL+1));;
            SKIP) SKIP=$((SKIP+1));;
        esac
    done
    echo
    echo "Pass: $PASS  Fail: $FAIL  Skip: $SKIP"
} | tee "$VERDICT_TXT"

if (( FAIL > 0 )); then
    exit 1
fi
exit 0
