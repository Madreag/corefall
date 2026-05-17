#!/usr/bin/env bash
# **M4B § "Migration corpus matrix passes for every fixture"** — CI gate that
# loads every fixture under game/content/save_corpus/v1_*.cfsave, migrates it
# to the current build's schema, and asserts the resulting canonical-JSON
# BLAKE3 matches the v(N)_minimal.cfsave / v(N)_full_squad.cfsave golden file
# for the target version.
#
# Runs `cargo run -p cf-save --example dump_save_corpus -- --check` which
# does the byte-for-byte drift check; on drift the example exits non-zero
# with the list of files that drifted.
#
# Also runs `cf-mod save validate <path>` on every fixture (asserts schema +
# migration + checksum integrity in isolation).
#
# Usage:
#   game/scripts/m4b_migration_matrix.sh            # all fixtures, abort on first failure
#   game/scripts/m4b_migration_matrix.sh --json     # JSON envelope per fixture

set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
CORPUS_DIR="${ROOT_DIR}/content/save_corpus"
JSON=0

for arg in "$@"; do
  case "$arg" in
    --json) JSON=1 ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

if [ ! -d "${CORPUS_DIR}" ]; then
  echo "save corpus missing at ${CORPUS_DIR}" >&2
  exit 2
fi

cd "${ROOT_DIR}"

# Step 1: regenerate-or-check the corpus. The --check path fails on drift.
if [ "${JSON}" = "1" ]; then
  cargo run -p cf-save --example dump_save_corpus -- --check 2>&1 | tee /tmp/m4b_corpus_check.log
else
  echo "[m4b] corpus drift check..."
  cargo run -p cf-save --example dump_save_corpus -- --check
fi

# Step 2: run `cf-mod save validate` on every fixture. The tampered fixture
# MUST exit non-zero (validating the corruption rejection path).
declare -a results=()
total=0
ok=0
expected_fail=0
unexpected_fail=0
for fixture in "${CORPUS_DIR}"/*.cfsave; do
  total=$((total+1))
  basename="$(basename "${fixture}")"
  set +e
  output="$(cargo run -p cf-mod -- --json save validate "${fixture}" 2>&1)"
  rc=$?
  set -e
  if [[ "${basename}" == tampered_chain* ]]; then
    if [ ${rc} -ne 0 ]; then
      expected_fail=$((expected_fail+1))
      results+=("${basename}: REJECTED (expected) rc=${rc}")
    else
      unexpected_fail=$((unexpected_fail+1))
      results+=("${basename}: UNEXPECTED PASS (tampered fixture should reject)")
    fi
  else
    if [ ${rc} -eq 0 ]; then
      ok=$((ok+1))
      results+=("${basename}: OK")
    else
      unexpected_fail=$((unexpected_fail+1))
      results+=("${basename}: UNEXPECTED FAIL rc=${rc}: ${output}")
    fi
  fi
done

echo
echo "M4B migration matrix:"
for r in "${results[@]}"; do
  echo "  ${r}"
done
echo "  total=${total} ok=${ok} expected_reject=${expected_fail} unexpected=${unexpected_fail}"

if [ ${unexpected_fail} -ne 0 ]; then
  exit 1
fi

# Step 3: byte-for-byte canonical-JSON BLAKE3 golden hash check.
# Per spec: "the migrated blob's canonical-JSON blake3 matches the
# v(N)_minimal.cfsave or v(N)_full_squad.cfsave golden file for the
# target version".
#
# Implementation: migrate v1_minimal.cfsave to v2 in-memory, recompute
# its canonical-JSON blake3, and compare to v2_minimal.cfsave.checksum.
echo
echo "[m4b] golden hash check (v1_minimal migrated to v2 must match v2_minimal)..."
cargo run -p cf-save --example migration_golden_check 2>&1 | tee /tmp/m4b_golden_check.log
GOLDEN_RC=${PIPESTATUS[0]}
if [ ${GOLDEN_RC} -ne 0 ]; then
  echo "[m4b] golden hash check FAILED" >&2
  exit 1
fi

echo "[m4b] migration matrix PASS"
