#!/usr/bin/env bash
# **M1 Enhancement CI D1**: smoke every M1 scenario at 60Hz AND 120Hz to
# prove the engine is deterministic + crash-free across tick rates.
#
# Each `m1_*.cfctl.json` script names a `scenario`; this script runs
# `cf-app --headless-smoke` on that scenario at both tick rates with the
# script's nominal tick budget, then runs `prototype_run_check.py` on the
# resulting run bundle and asserts `errors=0`. Combined with the in-process
# cross-run determinism test in cf-control, this proves the engine
# (1) ticks cleanly at each rate, (2) produces a well-formed bundle, and
# (3) honors the deterministic checksum contract.
#
# Live cfctl-script execution (i.e. spawning cf-app with --control-api and
# replaying methods) lands as a separate CI workflow at M3A when the
# replay verifier ships its bundle-from-script path.
#
# Usage:
#   game/scripts/ci/m1_determinism_matrix.sh            # uses defaults
#   game/scripts/ci/m1_determinism_matrix.sh --keep     # leaves bundles in place
#
# Exit codes:
#   0  -> every scenario * tick_rate combination produced errors=0
#   1+ -> at least one combination failed; failing scenario id printed to stderr.

set -euo pipefail

cd "$(dirname "$0")/../.."

KEEP=0
for arg in "$@"; do
  case "$arg" in
    --keep) KEEP=1 ;;
    -h|--help)
      sed -n '1,20p' "$0"
      exit 0
      ;;
    *)
      echo "unknown flag: $arg" >&2
      exit 2
      ;;
  esac
done

# Unique scenarios referenced by m1_*.cfctl.json scripts.
SCENARIOS=(
  "m1_actor_range:600"
  "m1_actor_range_shotgun:120"
  "m1_actor_range_tracer:600"
)

TICK_RATES=(60 120)

OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cf-m1-determinism-XXXXXX")"
echo "M1 determinism matrix: writing bundles to $OUT_DIR" >&2

declare -i FAILURES=0
FAILED_LIST=()

for entry in "${SCENARIOS[@]}"; do
  scenario="${entry%%:*}"
  ticks="${entry##*:}"
  for hz in "${TICK_RATES[@]}"; do
    bundle_root="$OUT_DIR/${scenario}-${hz}hz"
    mkdir -p "$bundle_root"
    echo "  run @${hz}Hz: $scenario (ticks=$ticks)" >&2
    if ! cargo run --quiet -p cf-app -- \
        --headless-smoke \
        --scenario "$scenario" \
        --tick-rate-hz "$hz" \
        --ticks "$ticks" \
        --write-run-bundle \
        --run-bundle-dir "$bundle_root" > "$bundle_root/cf-app.stdout" 2> "$bundle_root/cf-app.stderr"; then
      echo "    cf-app FAILED: $scenario @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$scenario@${hz}hz")
      continue
    fi
    bundle_dir=$(find "$bundle_root" -maxdepth 2 -mindepth 1 -type d | head -n 1 || true)
    if [[ -z "$bundle_dir" || ! -f "$bundle_dir/events.jsonl" ]]; then
      echo "    no events.jsonl produced for $scenario @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$scenario@${hz}hz")
      continue
    fi
    if ! python3 ../game/tools/prototype_run_check.py "$bundle_dir" > "$bundle_root/run_check.stdout" 2>&1; then
      echo "    run_check FAILED: $scenario @ ${hz}Hz (see $bundle_root/run_check.stdout)" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$scenario@${hz}hz")
      continue
    fi
    errors=$(grep -E '^errors ' "$bundle_root/run_check.stdout" | awk '{print $2}')
    if [[ "$errors" != "0" ]]; then
      echo "    errors=$errors > 0 for $scenario @ ${hz}Hz (see $bundle_root/run_check.stdout)" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$scenario@${hz}hz")
      continue
    fi
  done
done

if (( FAILURES > 0 )); then
  echo "M1 determinism matrix: $FAILURES failure(s)" >&2
  printf '  - %s\n' "${FAILED_LIST[@]}" >&2
  if (( KEEP == 0 )); then
    rm -rf "$OUT_DIR"
  fi
  exit 1
fi

echo "M1 determinism matrix: all combinations PASS"
if (( KEEP == 0 )); then
  rm -rf "$OUT_DIR"
else
  echo "bundles retained at $OUT_DIR"
fi
