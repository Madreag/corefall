#!/usr/bin/env bash
# M8B § Notes — byte-pin CI gate.
#
# "Any change that flips a single byte in a v0.1 fixture vector MUST
# bump `PROTOCOL_SEMVER` minor and add a new fixture; the byte-pin CI
# gate enforces this."
#
# This script runs:
# 1. The cf-net `frame_v01_byte_pin` integration test which loads
#    `game/content/net/protocol/frame_v01_fixtures.json` and verifies
#    that the encoder produces byte-identical output for every locked
#    NetPayload variant.
# 2. A targeted inner-module test that re-runs the same check from
#    inside the cf-net protocol module (`byte_pinning_tests`).
#
# Exits 0 on green, 1 on any byte-layout drift.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "${REPO_ROOT}/game"

echo "M8B byte-pin gate: running cf-net frame_v01_byte_pin integration test..."
if ! cargo test --quiet -p cf-net --test frame_v01_byte_pin 2>&1 | tail -20; then
    echo "[FAIL] cf-net frame_v01_byte_pin"
    echo "       The v0.1 fixture vector(s) drifted from the encoder output."
    echo "       Either:"
    echo "       1) The byte change is intentional — bump PROTOCOL_SEMVER minor + regenerate"
    echo "          game/content/net/protocol/frame_v01_fixtures.json."
    echo "       2) The change is unintentional — revert the wire-shape change."
    exit 1
fi

echo "M8B byte-pin gate: running cf-net inner-module byte_pinning tests..."
if ! cargo test --quiet -p cf-net --lib byte_pinning 2>&1 | tail -10; then
    echo "[FAIL] cf-net inner byte_pinning_tests"
    exit 1
fi

echo "M8B byte-pin gate: PASS"
exit 0
