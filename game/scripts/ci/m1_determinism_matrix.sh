#!/usr/bin/env bash
# **M1 R2 / CI D1 (real)**: drive every `m1_*.cfctl.json` script through cf-e2e
# at 60Hz AND 120Hz; verify the resulting run bundle is well-formed
# (`prototype_run_check.py` returns errors=0) AND contains the events the
# script actually exercises (weapon_fired / projectile_spawned / actor_*
# / equipment.alarm_registered etc).
#
# The previous version of this matrix only ran `cf-app --headless-smoke` for
# a raw tick budget, which does NOT dispatch the cfctl script steps — so a
# 120-tick smoke of m1_actor_range_shotgun produced zero weapon_fired events.
# That made the gate fake. This rewrite drives cf-e2e --script <name> per
# script, with --unpaced so 18000-tick endurance scripts complete in
# seconds rather than 5 minutes of wall-clock pacing.
#
# Usage:
#   game/scripts/ci/m1_determinism_matrix.sh         # full matrix incl endurance
#   game/scripts/ci/m1_determinism_matrix.sh --fast  # skip endurance (dev loop)
#   game/scripts/ci/m1_determinism_matrix.sh --keep  # retain bundles
#
# Exit codes:
#   0  -> every script * tick_rate combination produced errors=0 AND the
#         expected event types are present in events.jsonl.
#   1+ -> at least one combination failed; failing script id + reason
#         printed to stderr.

set -euo pipefail

cd "$(dirname "$0")/../.."

KEEP=0
FAST=0
for arg in "$@"; do
  case "$arg" in
    --keep) KEEP=1 ;;
    --fast) FAST=1 ;;
    -h|--help)
      sed -n '1,25p' "$0"
      exit 0
      ;;
    *)
      echo "unknown flag: $arg" >&2
      exit 2
      ;;
  esac
done

# Every m1_*.cfctl.json under scripts/cfctl/. Format:
#   "<script_name>:<event_grep_pattern>"
# Where the grep pattern is an event_type to require in events.jsonl (a
# minimum proof that the script's commands actually flowed through to the
# engine). Pattern is anchored with double quotes so we match the JSON
# field literally.
SCRIPTS=(
  "m1_move_jump_fire_reload:\"event_type\":\"weapon_fired\""
  "m1_jump_only:\"event_type\":\"actor_jumped\""
  "m1_reset_loop:\"event_type\":\"actor_reset\""
  "m1_inventory_cycle:\"event_type\":\"selected_item_changed\""
  "m1_sharp_aim_invalidations:\"method\":\"act.player.sharp_aim\""
  "m1_120hz_determinism:\"event_type\":\"weapon_fired\""
  "m1_shotgun_particle_count:\"event_type\":\"projectile_spawned\""
  "m1_tracer_cadence:\"is_tracer\":true"
)
# Endurance is 18000 ticks. Even unpaced it dominates wall clock; --fast
# skips it for dev iteration; default = full coverage.
ENDURANCE="m1_5min_endurance:\"event_type\":\"weapon_fired\""

if (( FAST == 0 )); then
  SCRIPTS+=("$ENDURANCE")
fi

TICK_RATES=(60 120)

OUT_DIR="$(mktemp -d "${TMPDIR:-/tmp}/cf-m1-determinism-XXXXXX")"
echo "M1 determinism matrix: writing bundles to $OUT_DIR" >&2

declare -i FAILURES=0
FAILED_LIST=()

# Build cf-e2e + cf-app once up front so the first invocation isn't wedged
# in a Bevy compile that exceeds the cf-e2e timeout.
echo "M1 determinism matrix: pre-building cf-e2e and cf-app" >&2
cargo build --quiet -p cf-e2e -p cf-app

# Endurance gets a generous timeout (the unpaced engine should finish in
# seconds, but giving it 400s avoids any chance of a cold-cache flake).
endurance_timeout=400

for entry in "${SCRIPTS[@]}"; do
  script="${entry%%:*}"
  expect_grep="${entry#*:}"
  scenario_for_script=$(python3 - "scripts/cfctl/${script}.cfctl.json" <<'PY'
import json, sys
with open(sys.argv[1]) as f:
    data = json.load(f)
print(data.get("scenario", "m1_actor_range"))
PY
)
  for hz in "${TICK_RATES[@]}"; do
    bundle_root="$OUT_DIR/${script}-${hz}hz"
    mkdir -p "$bundle_root"
    echo "  run @${hz}Hz: $script (scenario=$scenario_for_script)" >&2
    timeout_s=180
    if [[ "$script" == "m1_5min_endurance" ]]; then
      timeout_s=$endurance_timeout
    fi
    if ! cargo run --quiet -p cf-e2e -- \
        --scenario "$scenario_for_script" \
        --script "$script" \
        --tick-rate-hz "$hz" \
        --unpaced \
        --write-run-bundle \
        --timeout-seconds "$timeout_s" \
        > "$bundle_root/cf-e2e.stdout" 2> "$bundle_root/cf-e2e.stderr"; then
      echo "    cf-e2e FAILED: $script @ ${hz}Hz" >&2
      echo "    last stderr lines:" >&2
      tail -n 8 "$bundle_root/cf-e2e.stderr" | sed 's/^/      /' >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: cf-e2e exit non-zero")
      continue
    fi
    # cf-e2e writes the bundle into the configured run-bundle root, which
    # defaults to prototype_runs/native. Locate the most recent matching
    # bundle dir by reading the cf-app stdout (cf-e2e tee'd both streams).
    bundle_dir=$(grep -oE 'bundle=[^[:space:]]+' "$bundle_root/cf-e2e.stderr" 2>/dev/null \
        | tail -n 1 | sed 's/^bundle=//')
    if [[ -z "$bundle_dir" ]]; then
      bundle_dir=$(grep -oE 'bundle=[^[:space:]]+' "$bundle_root/cf-e2e.stdout" 2>/dev/null \
          | tail -n 1 | sed 's/^bundle=//')
    fi
    if [[ -z "$bundle_dir" || ! -f "$bundle_dir/events.jsonl" ]]; then
      echo "    could not resolve bundle dir for $script @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: bundle dir missing")
      continue
    fi
    cp -r "$bundle_dir" "$bundle_root/bundle"
    if ! python3 ../game/tools/prototype_run_check.py "$bundle_dir" \
        > "$bundle_root/run_check.stdout" 2>&1; then
      echo "    run_check FAILED: $script @ ${hz}Hz" >&2
      tail -n 6 "$bundle_root/run_check.stdout" | sed 's/^/      /' >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: run_check non-zero exit")
      continue
    fi
    errors=$(grep -E '^errors ' "$bundle_root/run_check.stdout" | awk '{print $2}')
    if [[ "$errors" != "0" ]]; then
      echo "    errors=$errors > 0 for $script @ ${hz}Hz" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: prototype_run_check errors=$errors")
      continue
    fi
    # Proof of life: events.jsonl must contain at least one event matching
    # the expected event_type / payload pattern. Without this the previous
    # gate could pass with zero script-driven events.
    if ! grep -qE "$expect_grep" "$bundle_dir/events.jsonl"; then
      echo "    expected pattern not found in events.jsonl for $script @ ${hz}Hz" >&2
      echo "      pattern: $expect_grep" >&2
      FAILURES=$((FAILURES + 1))
      FAILED_LIST+=("$script@${hz}hz: missing pattern $expect_grep")
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
