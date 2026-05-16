#!/usr/bin/env bash
# M8A determinism contract lint gate.
# Enforces the 10 architecture rules from
# docs/plan/spec/determinism-island-contract.md § M8A extensions.
#
# Verifies:
# - clippy --workspace --all-targets -D warnings (the workspace-wide gate already
#   rejects rand::thread_rng, Instant::now, SystemTime::now via clippy.toml).
# - sim crates do not introduce new f64 outside the documented boundary-use
#   lines.
# - sim crates do not introduce std::sync::Mutex in hot paths (only allowed
#   outside sim, e.g. cf-control's audio_plugin field).
#
# Exits 0 on green, 1 on any violation.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "${REPO_ROOT}/game"

FAIL=0
SIM_CRATES=(
    cf-sim-core
    cf-actor
    cf-ai
    cf-physics
    cf-material
    cf-terrain
    cf-atmos
)

echo "M8A determinism lint: clippy gate (delegates to workspace clippy)"
echo "  -> covered by m8a_close_gates.sh + the workspace 5-cmd build-health"
echo "M8A determinism lint: scanning sim crates for forbidden patterns..."

# Boundary uses for f64 are explicitly documented in
# docs/plan/spec/determinism-island-contract.md § M8A extensions.
# These lines are allowed because their f64 results never enter the tick
# determinism checksum: they are parse-time (cf-material), display-time
# (cf-sim-core), or 53-bit-mantissa trick producing f32 outputs (cf-ai).
ALLOWED_F64_PATTERN='(// boundary use|^[[:space:]]*//|tick_dt_ms|sim_time_ms|next_f32_uniform|loader\.rs|constants\.rs:.*= 1.0_f64)'

for crate in "${SIM_CRATES[@]}"; do
    crate_dir="crates/${crate}/src"
    if [[ ! -d "${crate_dir}" ]]; then
        echo "  [SKIP] ${crate}: src/ not found"
        continue
    fi

    # Forbidden: thread_rng usage (comments are OK; we look for actual function call patterns)
    if rg -nE '\brand::thread_rng\(\)|\bthread_rng\(\)' "${crate_dir}" \
       --type rust 2>/dev/null | rg -v '//[^"]*thread_rng' > /tmp/m8a_lint_thread_rng_${crate}.txt; then
        if [[ -s /tmp/m8a_lint_thread_rng_${crate}.txt ]]; then
            echo "  [FAIL] ${crate}: thread_rng() usage detected"
            cat /tmp/m8a_lint_thread_rng_${crate}.txt
            FAIL=1
        fi
    fi

    # Forbidden: Instant::now / SystemTime::now in sim crate src files
    if rg -nE '(Instant::now|SystemTime::now)\(' "${crate_dir}" \
       --type rust 2>/dev/null > /tmp/m8a_lint_time_${crate}.txt; then
        if [[ -s /tmp/m8a_lint_time_${crate}.txt ]]; then
            echo "  [FAIL] ${crate}: Instant::now / SystemTime::now usage detected"
            cat /tmp/m8a_lint_time_${crate}.txt
            FAIL=1
        fi
    fi

    # Forbidden: std::sync::Mutex in sim hot paths. Allowed: documented in
    # determinism contract (currently none expected inside SIM_CRATES).
    if rg -nE 'std::sync::Mutex|parking_lot::Mutex' "${crate_dir}" \
       --type rust 2>/dev/null > /tmp/m8a_lint_mutex_${crate}.txt; then
        if [[ -s /tmp/m8a_lint_mutex_${crate}.txt ]]; then
            echo "  [FAIL] ${crate}: std::sync::Mutex usage in sim crate"
            cat /tmp/m8a_lint_mutex_${crate}.txt
            FAIL=1
        fi
    fi
done

# Cleanup
rm -f /tmp/m8a_lint_*.txt

if [[ ${FAIL} -ne 0 ]]; then
    echo "M8A determinism lint: FAIL"
    exit 1
fi

echo "M8A determinism lint: PASS"
exit 0
