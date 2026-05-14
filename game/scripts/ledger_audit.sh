#!/usr/bin/env bash
# M4A: nightly ledger audit. Verifies that every entry in the canonical
# `content/asset_ledger/ledger.jsonl` references an existing output_path
# whose blake3 matches the ledger record. Drift / missing / failed entries
# cause a non-zero exit.
#
# Usage:
#   game/scripts/ledger_audit.sh         # verify all entries (strict)
#   game/scripts/ledger_audit.sh --json  # emit JSON report on stdout

set -euo pipefail

cd "$(dirname "$0")/.."

JSON_MODE=0
for arg in "$@"; do
  case "$arg" in
    --json) JSON_MODE=1 ;;
    -h|--help)
      sed -n '1,18p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

LEDGER_PATH="content/asset_ledger/ledger.jsonl"
if [[ ! -f "$LEDGER_PATH" ]]; then
  echo "ERROR: missing ledger file $LEDGER_PATH" >&2
  exit 3
fi

CARGO_FLAGS=("--release" "-q")

# **M4A spec literal**: `cf-mod ledger verify --strict --all` is the CI-gate
# form. `--strict` is the cf-mod GLOBAL flag (consumed by run_ledger via
# global_strict OR'd into the per-verb strict-status). `--strict-status` is
# the local alias. We pass BOTH so the script is resilient to clap reordering
# and to milestone teams who switch between conventions.
if [[ "$JSON_MODE" == "1" ]]; then
  cargo run "${CARGO_FLAGS[@]}" -p cf-mod -- --strict --json ledger verify --strict-status --all
else
  cargo run "${CARGO_FLAGS[@]}" -p cf-mod -- --strict ledger verify --strict-status --all
fi
