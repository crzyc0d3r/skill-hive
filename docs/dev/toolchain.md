# Toolchain

Phase 0 pins the engines the three CI operating systems share.

| Tool | Pin | Where |
|---|---|---|
| Node.js | 22 | `.nvmrc`, `actions/setup-node` |
| Rust | stable channel | `rust-toolchain.toml` |
| MSRV | 1.97 (current stable minus two) | `Cargo.toml` `rust-version` |
| Tauri CLI | 2.x (`@tauri-apps/cli`) | `package.json` |
| Rustfmt, Clippy | toolchain components | `rust-toolchain.toml` |

`rust-toolchain.toml` tracks the stable channel so a security release of rustc is picked up without editing the file. The workspace `rust-version` stays two stable releases behind that channel. When stable moves, raise `rust-version` by the same step and note it in the pull request.

Do not reuse the skills-manager identifier `com.agentskills.desktop`, its icons, its updater public key, or its product name. Skill Hive's identifier is `com.skillhive.desktop`.

## Platform packages

Ubuntu (22.04 and 24.04):

```bash
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev librsvg2-dev patchelf build-essential curl pkg-config
sudo apt-get install -y libayatana-appindicator3-dev || sudo apt-get install -y libappindicator3-dev
```

macOS: Xcode command line tools. The overlay title bar is configured only in `src-tauri/tauri.macos.conf.json`.

Windows: the Visual Studio Build Tools C++ workload and the WebView2 runtime (present on GitHub-hosted runners).

## Checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
cargo audit
npm ci
npm run lint
npm run contrast
npm run check:headers
npm run check:palette
node scripts/gen-ipc-types.mjs --check
node scripts/gen-ipc-types.mjs --self-test
npx tsc -b
npm test
```
