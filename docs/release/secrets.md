# Secrets for signed Skill Hive releases

No certificate, notarization credential, or updater private key is stored in this repository. Unsigned builds succeed when every secret below is unset. If any secret in a set is present and the set is incomplete, the release workflow fails before `tauri-action` can ad-hoc sign.

Add these in the GitHub repository settings, Actions secrets. Names are exact.

## Apple notarization

Use one complete set.

App Store Connect API key (matches the procurement note):

- `APPLE_CERTIFICATE` — base64 of the Developer ID Application `.p12`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY` — `Developer ID Application: <name> (<TEAMID>)`
- `APPLE_API_ISSUER` — App Store Connect issuer id
- `APPLE_API_KEY` — the key id
- `APPLE_API_PRIVATE_KEY` — contents of the AuthKey `.p8`

Apple ID app-specific password, if you are not using the API key:

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`

When the set is complete, macOS jobs sign with the hardened runtime, notarize with `notarytool` (30 minute wait inside the 60 minute job), and run `codesign --verify`, `spctl --assess`, and `xcrun stapler validate` on the `.app` and on the `.app` inside `.app.tar.gz`. The CLI is signed in a step-local keychain. A bare executable is not stapled; the gate is the verified signature and `notarytool --wait`.

## Windows signing

Use one complete set.

Azure Trusted Signing:

- `AZURE_TENANT_ID`
- `AZURE_CLIENT_ID`
- `AZURE_CLIENT_SECRET`
- `AZURE_TRUSTED_SIGNING_ENDPOINT`
- `AZURE_CODE_SIGNING_ACCOUNT_NAME`
- `AZURE_CERTIFICATE_PROFILE_NAME`

The bundler `signCommand` hook runs `scripts/windows-sign.ps1`. The runner also needs `signtool` and `Azure.CodeSigning.Dlib.dll` in `RUNNER_TEMP`.

OV or EV certificate:

- `WINDOWS_CERTIFICATE` — base64 of the `.pfx`
- `WINDOWS_CERTIFICATE_PASSWORD`

## Updater

Skill Hive's minisign key. The public key is `plugins.updater.pubkey` in `src-tauri/tauri.conf.json` (key id `8FC6E5C30C36CA69`). Do not substitute another product's key.

- `TAURI_SIGNING_PRIVATE_KEY` — the minisign secret key file, including the comment line
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — optional. The key generated for this release is unencrypted, so leave this unset

When `TAURI_SIGNING_PRIVATE_KEY` is set, the workflow turns on `createUpdaterArtifacts` and `verify-updater-assets` requires `latest.json` plus a signature for `darwin-aarch64`, `darwin-x86_64`, `windows-x86_64`, `linux-x86_64`, and `linux-aarch64`. A missing signature fails the job, and `publish-release` does not run.

`publish-release` also requires the Apple set and the Windows set. Until then the tag stays a draft.

## What you do

1. Put the private key into `TAURI_SIGNING_PRIVATE_KEY`.
2. Add one complete Apple set and one complete Windows set.
3. Tag `v*` again. The workflow signs, verifies, and publishes.
