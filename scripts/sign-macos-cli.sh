#!/bin/bash
# Sign and notarize the CLI in a step-local keychain. A bare executable cannot
# carry a stapled ticket; the gate is the verified signature plus Apple's Accepted status.
set -euo pipefail

bin="target/${CLI_TARGET}/release/skill-hive-cli"
if [ "${SIGNING_APPLE:-unsigned}" != "sign" ]; then
  echo "unsigned CLI: step-local keychain and notarytool are skipped"
  exit 0
fi

keychain="${RUNNER_TEMP:-/tmp}/skill-hive-cli.keychain"
password=$(openssl rand -hex 16)
security create-keychain -p "$password" "$keychain"
security set-keychain-settings -lut 600 "$keychain"
security unlock-keychain -p "$password" "$keychain"
security default-keychain -s "$keychain"
security list-keychains -d user -s "$keychain" "$(security list-keychains -d user | sed 's/\"//g')"

cert="${RUNNER_TEMP:-/tmp}/developer-id.p12"
python3 -c 'import os,sys,base64; open(sys.argv[1],"wb").write(base64.b64decode(os.environ["APPLE_CERTIFICATE"]))' "$cert"
security import "$cert" -k "$keychain" -P "$APPLE_CERTIFICATE_PASSWORD" -T /usr/bin/codesign
security set-key-partition-list -S apple-tool:,apple: -s -k "$password" "$keychain"

codesign --force --options runtime --timestamp --sign "$APPLE_SIGNING_IDENTITY" "$bin"
codesign --verify --strict --verbose=2 "$bin"

zip="${RUNNER_TEMP:-/tmp}/skill-hive-cli.zip"
ditto -c -k --keepParent "$bin" "$zip"
if [ "${APPLE_PATH}" = "api-key" ]; then
  xcrun notarytool submit "$zip" --key "$APPLE_API_KEY_PATH" --key-id "$APPLE_API_KEY" --issuer "$APPLE_API_ISSUER" --wait --timeout 30m
else
  xcrun notarytool submit "$zip" --apple-id "$APPLE_ID" --password "$APPLE_PASSWORD" --team-id "$APPLE_TEAM_ID" --wait --timeout 30m
fi
echo "CLI signature verified; notarytool --wait returned Accepted for a bare executable (no staple)"
security delete-keychain "$keychain"
