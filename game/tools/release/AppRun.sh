#!/bin/sh
# Corefall AppImage launcher (AppRun).
# Sets up the runtime so cf-app can find its content/ + scripts/cfctl/
# bundles regardless of where the AppImage was double-clicked from.

set -eu

HERE="$(dirname "$(readlink -f "${0}")")"
export APPDIR="${HERE}"
export PATH="${HERE}/usr/bin:${PATH:-}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${LD_LIBRARY_PATH:-}"

# cf-app resolves scenarios relative to the working dir's content/ tree.
# Run from the bundled content root so the default scenario loads cleanly.
cd "${HERE}/usr/share/corefall"

exec "${HERE}/usr/bin/cf-app" "$@"
