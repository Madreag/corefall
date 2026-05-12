#!/usr/bin/env bash
# **M1.5 / CI**: drive every M1.5 cfctl script (micro_breach_{win,loss,abort,
# stealth} + ai_h_01_sentry_hears_threat) through cf-e2e at 60Hz AND 120Hz;
# verify each bundle is well-formed (`prototype_run_check.py` errors=0) AND
# contains the events that prove the script's contract.
#
# Usage:
#   game/scripts/ci/m1_5_determinism_matrix.sh         # full matrix
#   game/scripts/ci/m1_5_determinism_matrix.sh --keep  # retain bundles
#
# Exit codes:
#   0  -> every script * tick_rate combination passes.
#   1+ -> at least one combination failed; failing id printed to stderr.

set -euo pipefail

cd "$(dirname "$0")/../.."

KEEP=0
for arg in "$@"; do
  case "$arg" in
    --keep) KEEP=1 ;;
    -h|--help)
      sed -n '1,15p' "$0"
      exit 0
      ;;
    *)
      echo "unknown flag: $arg" >&2
      exit 2
      ;;
  esac
done

# Each entry: "<script>:<grep_pattern>:<rates>". The grep pattern is a
# literal JSON substring that must appear in events.jsonl. `rates` is a
# comma-separated list of tick rates this script supports — most M1.5
# scripts are written with movement budgets tuned for 60Hz; rate-aware
# rewrites land at M5+ when the cfctl grammar gains seconds-based
# duration primitives.
SCRIPTS=(
  "micro_breach_win:\"event_type\":\"mission_resolved\":60"
  "micro_breach_loss:\"event_type\":\"mission_resolved\":60"
  "micro_breach_abort:\"result\":\"aborted\":60,120"
  "micro_breach_stealth:\"event_type\":\"terrain_carved\":60"
  "ai_h_01_sentry_hears_threat:\"kind\":\"hearing\":60,120"
  "m1_5_difficulty_cakewalk:\"ai_difficulty\":\"cakewalk\":60"
  "m1_5_difficulty_veteran:\"ai_difficulty\":\"veteran\":60"
  "m1_5_pause_resume:\"event_type\":\"objective_paused\":60"
  "m1_5_ai_debug:\"ai_debug\":true:60"
)

OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cf-m1-5-determinism-XXXXXX")"
echo "M1.5 determinism matrix: writing bundles to $OUT_DIR" >&2

declare -i FAILURES=0
FAILED_LIST=()

# Pre-build to avoid the cf-e2e first-call cold-cache flake.
echo "M1.5 determinism matrix: pre-building cf-e2e and cf-app" >&2
cargo build --quiet -p cf-e2e -p cf-app

for entry in "${SCRIPTS[@]}"; do
  # Parse "<script>:<grep>:<rates>". Note grep_pattern itself contains
  # colons (the JSON-escaped `":"`), so use python to split on the LAST
  # two colon-separated fields.
  script="${entry%%:*}"
  remainder="${entry#${script}:}"
  rates="${remainder##*:}"
  expect_grep="${remainder%:${rates}}"
  scenario_for_script=$(python3 - "scripts/cfctl/${script}.cfctl.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
print(data.get("scenario", "micro_breach"))
PY
)
  IFS=',' read -ra RATE_LIST <<< "$rates"
  for hz in "${RATE_LIST[@]}"; do
    bundle_root="$OUT_DIR/${script}-${hz}hz"
    mkdir -p "$bundle_root"
    echo "  run @${hz}Hz: $script (scenario=$scenario_for_script)" >&2
    # **M1.5**: micro_breach scripts depend on positional progress
    # (player must move N units to reach the breach / extraction zone).
    # Under --unpaced the engine races faster than cf-e2e can dispatch
    # commands, so move-x intents land mid-race AND only a few hundred
    # ticks of move are observed. Paced mode (no --unpaced) holds
    # wall-clock cadence so the dispatcher → sim handshake is honored
    # tick-by-tick. Endurance-style M1.5 scripts (none today) would need
    # --unpaced; the basic micro_breach contract is paced.
    if ! cargo run --quiet -p cf-e2e -- \
        --scenario "$scenario_for_script" \
        --script "$script" \
        --tick-rate-hz "$hz" \
        --write-run-bundle \
        --timeout-seconds 180 \
        > "$bundle_root/cf-e2e.stdout" 2> "$bundle_root/cf-e2e.stderr"; then
      echo "    cf-e2e FAILED: $script @ ${hz}Hz" >&2
      echo "    last stderr lines:" >&2
      tail -n 8 "$bundle_root/cf-e2e.stderr" | sed 's/^/      /' >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: cf-e2e exit non-zero")
      continue
    fi
    strip_ansi='s/\x1B\[[0-9;]*[A-Za-z]//g'
    bundle_dir=$( (sed -e "$strip_ansi" "$bundle_root/cf-e2e.stdout" \
        | grep -oE 'bundle=[^[:space:]]+' || true) \
        | tail -n 1 | sed 's/^bundle=//')
    if [[ -z "$bundle_dir" ]]; then
      bundle_dir=$( (sed -e "$strip_ansi" "$bundle_root/cf-e2e.stderr" \
          | grep -oE 'bundle=[^[:space:]]+' || true) \
          | tail -n 1 | sed 's/^bundle=//')
    fi
    if [[ -z "$bundle_dir" ]]; then
      echo "    could not resolve bundle dir for $script @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: bundle dir missing")
      continue
    fi
    if [[ ! -d "$bundle_dir" ]]; then
      echo "    bundle dir does not exist: $bundle_dir" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: bundle dir missing on disk")
      continue
    fi
    if ! python3 tools/prototype_run_check.py "$bundle_dir" \
        > "$bundle_root/run_check.stdout" 2> "$bundle_root/run_check.stderr"; then
      echo "    prototype_run_check FAILED: $script @ ${hz}Hz" >&2
      cat "$bundle_root/run_check.stdout" | sed 's/^/      /' >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: run_check non-zero exit")
      continue
    fi
    errors=$( (grep -E '^errors ' "$bundle_root/run_check.stdout" || true) | awk '{print $2}')
    if [[ "$errors" != "0" ]]; then
      echo "    run_check reported $errors error(s) for $script @ ${hz}Hz" >&2
      cat "$bundle_root/run_check.stdout" | sed 's/^/      /' >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: errors=$errors")
      continue
    fi
    if ! grep -F -q "$expect_grep" "$bundle_dir/events.jsonl"; then
      echo "    expect pattern not found in events.jsonl for $script @ ${hz}Hz" >&2
      echo "    pattern: $expect_grep" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: missing event pattern $expect_grep")
      continue
    fi
  done
done

if (( KEEP == 0 )); then
  rm -rf "$OUT_DIR" 2>/dev/null || true
fi

if (( FAILURES > 0 )); then
  echo "M1.5 determinism matrix: ${FAILURES} failure(s)" >&2
  for f in "${FAILED_LIST[@]}"; do
    echo "  - $f" >&2
  done
  exit 1
fi

echo "M1.5 determinism matrix: all combinations PASS" >&2
exit 0
