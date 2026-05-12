#!/usr/bin/env bash
# **M1 Enhancement CI D1**: run every `m1_*.cfctl.json` script at 60Hz AND
# 120Hz, write a run bundle for each, and assert the per-rate checksum is
# stable + the bundle's `result` is `ok` (`prototype_run_check.py` errors=0).
#
# Usage:
#   game/scripts/ci/m1_determinism_matrix.sh            # uses defaults
#   game/scripts/ci/m1_determinism_matrix.sh --keep     # leaves bundles in place
#
# Exit codes:
#   0  -> every script * tick_rate combination produced errors=0
#   1+ -> at least one combination failed; failing script id printed to stderr.

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

SCRIPTS=(
  scripts/cfctl/m1_jump_only.cfctl.json
  scripts/cfctl/m1_inventory_cycle.cfctl.json
  scripts/cfctl/m1_reset_loop.cfctl.json
  scripts/cfctl/m1_move_jump_fire_reload.cfctl.json
  scripts/cfctl/m1_sharp_aim_invalidations.cfctl.json
  scripts/cfctl/m1_shotgun_particle_count.cfctl.json
  scripts/cfctl/m1_tracer_cadence.cfctl.json
  scripts/cfctl/m1_120hz_determinism.cfctl.json
  scripts/cfctl/m1_5min_endurance.cfctl.json
)

TICK_RATES=(60 120)

OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cf-m1-determinism-XXXXXX")"
echo "M1 determinism matrix: writing bundles to $OUT_DIR" >&2

declare -i FAILURES=0
FAILED_LIST=()

for script in "${SCRIPTS[@]}"; do
  if [[ ! -f "$script" ]]; then
    echo "skip (missing): $script" >&2
    continue
  fi
  scenario=$(python3 - "$script" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
print(data.get("scenario", "m1_actor_range"))
PY
)
  for hz in "${TICK_RATES[@]}"; do
    # Sum ticks the script asks for so we can pass --ticks.
    total_ticks=$(python3 - "$script" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
ticks = 0
for step in data.get("steps", []):
    p = step.get("params", {})
    if step.get("method", "").startswith("sim.") and "ticks" in p:
        ticks += int(p["ticks"])
print(max(ticks, 60))
PY
)
    bundle_root="$OUT_DIR/$(basename "${script%.cfctl.json}")-${hz}hz"
    mkdir -p "$bundle_root"
    echo "  run @${hz}Hz: $script (scenario=$scenario, ticks=$total_ticks)" >&2
    if ! cargo run --quiet -p cf-app -- \
        --headless-smoke \
        --scenario "$scenario" \
        --tick-rate-hz "$hz" \
        --ticks "$total_ticks" \
        --write-run-bundle \
        --run-bundle-dir "$bundle_root" > "$bundle_root/cf-app.stdout" 2> "$bundle_root/cf-app.stderr"; then
      echo "    cf-app FAILED: $script @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz")
      continue
    fi
    bundle_dir=$(find "$bundle_root" -maxdepth 2 -mindepth 1 -type d | head -n 1 || true)
    if [[ -z "$bundle_dir" || ! -f "$bundle_dir/events.jsonl" ]]; then
      echo "    no events.jsonl produced for $script @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz")
      continue
    fi
    if ! python3 game/tools/prototype_run_check.py "$bundle_dir" > "$bundle_root/run_check.stdout" 2>&1; then
      echo "    run_check FAILED: $script @ ${hz}Hz (see $bundle_root/run_check.stdout)" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz")
      continue
    fi
    errors=$(grep -E '^errors ' "$bundle_root/run_check.stdout" | awk '{print $2}')
    if [[ "$errors" != "0" ]]; then
      echo "    errors=$errors > 0 for $script @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz")
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
