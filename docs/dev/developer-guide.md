# Skill Hive developer guide

## Toolchain

Node 22 (`.nvmrc`) and Rust stable (`rust-toolchain.toml`, MSRV in `Cargo.toml`). The desktop shell uses Tauri 2. Platform packages are the apt list in `.github/workflows/release.yml` on Linux, Xcode command line tools on macOS, and the Visual Studio C++ build tools plus WebView2 on Windows.

Details: [toolchain.md](toolchain.md).

## First hour

[first-hour.md](first-hour.md) gets a window open and `npm run cli -- --version` printing. `npm run tauri:dev` uses a development base directory so a personal library is left alone.

## Fixtures

`fixtures/skills/` holds valid, warning, and error skills. `fixtures/generation/` holds the golden generation prompt. `fixtures/recordings/` holds replay captures. A cache miss in replay fails the test. Do not point tests at a live vendor API.

## Recordings

Provider adapters replay captured responses. The test engine's regression suite runs the fixture skill in replay mode. Re-record only when a model or prompt change is being reviewed, and keep the budget capped.

## Loop

- `npm run tauri:dev` for the window.
- `cargo test -p hive-core` for the core.
- `npm test` for the interface.
- `npm run cli -- --help` for the command tree. The checked reference is [../cli/reference.md](../cli/reference.md).

## Release

Version comes from `package.json`. `node scripts/prepare-release.mjs` copies it into `src-tauri/tauri.conf.json`, the workspace `Cargo.toml`, and the Homebrew and winget manifests.

Tag `v*` to run `.github/workflows/release.yml`. The workflow builds five targets, uploads a draft, and runs `verify-updater-assets`. It publishes the draft only when Apple signing, Windows signing, and the updater key are all configured. A nightly schedule builds an unsigned prerelease from `main`.

Secrets, and the reason unsigned builds stay unsigned, are in [../release/secrets.md](../release/secrets.md).

## Attribution

Derived files under `crates/hive-core/src/library`, `deploy`, and `backup` keep the header in `docs/dev/derived-file-header.txt`. `npm run check:headers` enforces it. The About screen carries the MIT notice.
