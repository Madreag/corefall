#!/usr/bin/env bash
# **M2 / CI**: drive every M2 cfctl script (m2_dig_concrete_refuse_metal,
# m2_anchor_refuse_nohook, m2_hazard_contact, m2_overlay_cycle,
# m2_perf_burst) through cf-e2e at 60Hz AND 120Hz; verify each bundle is
# well-formed (`prototype_run_check.py` errors=0) AND contains the events
# that prove the script's contract.
#
# Usage:
#   game/scripts/ci/m2_determinism_matrix.sh         # full matrix
#   game/scripts/ci/m2_determinism_matrix.sh --keep  # retain bundles
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

# Each entry: "<script>:<patterns>:<rates>"
#   - <patterns>: one or more grep patterns separated by '||'. ALL must be
#                 present in events.jsonl for the script to pass.
#   - <rates>: comma-separated tick rates this script supports. M2 scripts
#              that depend on positional progress (perf_burst) are 60Hz-only
#              because walk budgets are tuned for 60Hz; rate-aware rewrites
#              land at the M5+ cfctl seconds-based duration grammar.
SCRIPTS=(
  "m2_dig_concrete_refuse_metal:\"event_type\":\"terrain_carved\"||\"material_metal_nohook\":60"
  "m2_anchor_refuse_nohook:\"material_metal_nohook\"||\"material_anchor\":60"
  "m2_hazard_contact:\"event_type\":\"hazard_contact_or_avoidance\"||\"damage_applied\":60"
  "m2_overlay_cycle:\"event_type\":\"overlay_mode_changed\"||\"off\":60,120"
  "m2_perf_burst:\"event_type\":\"terrain_dirty_region_batch\"||\"event_type\":\"terrain_carved\":60"
)

OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cf-m2-determinism-XXXXXX")"
echo "M2 determinism matrix: writing bundles to $OUT_DIR" >&2

declare -i FAILURES=0
FAILED_LIST=()

# Pre-build to avoid cf-e2e cold-cache flake.
echo "M2 determinism matrix: pre-building cf-e2e and cf-app (release)" >&2
cargo build --release --quiet -p cf-e2e -p cf-app

for entry in "${SCRIPTS[@]}"; do
  # Parse "<script>:<patterns>:<rates>". patterns may contain `:` (JSON
  # ":") so split on the FIRST `:` (script) and LAST `:` (rates).
  script="${entry%%:*}"
  remainder="${entry#${script}:}"
  rates="${remainder##*:}"
  patterns="${remainder%:${rates}}"
  scenario_for_script=$(python3 - "scripts/cfctl/${script}.cfctl.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
print(data.get("scenario", "m2_material_lane"))
PY
)
  IFS=',' read -ra RATE_LIST <<< "$rates"
  for hz in "${RATE_LIST[@]}"; do
    bundle_root="$OUT_DIR/${script}-${hz}hz"
    mkdir -p "$bundle_root"
    echo "  run @${hz}Hz: $script (scenario=$scenario_for_script)" >&2
    # **M2**: micro_breach-style positional scripts hold at paced mode so
    # move-x intents land tick-by-tick rather than racing the unpaced
    # driver. Engineering note: --unpaced is fast but desynchronises
    # dispatch <-> sim handshake on movement-positional scripts.
    if ! cargo run --release --quiet -p cf-e2e -- \
        --scenario "$scenario_for_script" \
        --script "$script" \
        --tick-rate-hz "$hz" \
        --write-run-bundle \
        --timeout-seconds 60 \
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
    # Verify EVERY listed pattern is in events.jsonl. Patterns separated
    # by '||'; ALL must match.
    IFS='||' read -ra PATTERN_LIST <<< "$patterns"
    missing_any=0
    for p in "${PATTERN_LIST[@]}"; do
      # Skip empty fragments produced by the '||' tokenizer.
      [[ -z "$p" ]] && continue
      if ! grep -F -q "$p" "$bundle_dir/events.jsonl"; then
        echo "    expect pattern not found in events.jsonl for $script @ ${hz}Hz" >&2
        echo "    pattern: $p" >&2
        missing_any=1
        FAILED_LIST+=("$script@${hz}hz: missing event pattern $p")
      fi
    done
    if (( missing_any > 0 )); then
      FAILURES=$((FAILURES + 1))
      continue
    fi
  done
done

if (( KEEP == 0 )); then
  rm -rf "$OUT_DIR" 2>/dev/null || true
fi

if (( FAILURES > 0 )); then
  echo "M2 determinism matrix: ${FAILURES} failure(s)" >&2
  for f in "${FAILED_LIST[@]}"; do
    echo "  - $f" >&2
  done
  exit 1
fi

echo "M2 determinism matrix: all combinations PASS" >&2
exit 0
