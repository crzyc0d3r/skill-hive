#!/bin/bash
# codesign, spctl, and stapler run only after Apple signing was activated.
set -euo pipefail

if [ "${SIGNING_APPLE:-unsigned}" != "sign" ]; then
  echo "unsigned macOS build: codesign, spctl, and stapler are skipped until the Apple secrets in docs/release/secrets.md are present"
  exit 0
fi

app=$(find src-tauri/target -type d -name 'Skill Hive.app' | head -n 1)
if [ -z "${app}" ]; then
  echo "signed macOS build did not produce Skill Hive.app"
  exit 1
fi

verify_app() {
  local target="$1"
  codesign --verify --deep --strict --verbose=2 "$target"
  codesign -dv --verbose=4 "$target" 2>&1 | grep -q 'Authority=Developer ID Application'
  codesign -dv --verbose=4 "$target" 2>&1 | grep -q 'runtime'
  spctl --assess --type execute --verbose "$target"
}

verify_app "$app"
archive=$(find src-tauri/target -name '*.app.tar.gz' | head -n 1)
if [ -z "${archive}" ]; then
  echo "updater archive .app.tar.gz is missing"
  exit 1
fi
workdir=$(mktemp -d)
tar -xzf "$archive" -C "$workdir"
inner=$(find "$workdir" -type d -name 'Skill Hive.app' | head -n 1)
verify_app "$inner"
xcrun stapler validate "$app"
echo "macOS signature and notarization verified for the app and the updater archive"
