#!/bin/sh
# Corefall macOS .app launcher.
#
# Finder/launchd launches Corefall.app/Contents/MacOS/Corefall with cwd
# set to the user's home (not the bundle), so cf-app's relative scenario
# lookup (`content/scenarios/<id>.ron`) would fail. This launcher resolves
# the bundle's own Resources dir and `cd`s there before exec'ing cf-app.
#
# Keeping the launcher as the CFBundleExecutable instead of patching
# cf-app keeps cf-app's behavior identical for CI, cfctl, and scripted
# E2E callers (which all set cwd themselves).

set -eu

HERE="$(cd "$(dirname "${0}")" && pwd)"
# Contents/MacOS -> .. -> Contents -> Resources
RESOURCES="${HERE}/../Resources"

cd "${RESOURCES}"
exec "${HERE}/cf-app" "$@"
