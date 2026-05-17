#!/usr/bin/env bash
# **M4B § "Delta compression hits its target ratio"** — bench harness.
#
# Synthesizes 1-minute, 5-minute, and 30-minute scenarios as JSON-blob pairs
# (per-tick world state) and measures the delta-encoded chain size against
# the equivalent full-snapshot total. M4B target: >= 4.0x compression on the
# canonical 30-min / 200-actor / 500-projectile / 1000-hazard-pixel
# scenario.
#
# Implementation: defers the compute to `cargo run -p cf-save --example
# delta_compression_bench` (built ad-hoc). The script prints the ratio per
# duration and exits non-zero when the 30-min ratio is below 4.0.

set -eu

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
cd "${ROOT_DIR}"

cargo run --release -p cf-save --example delta_compression_bench
