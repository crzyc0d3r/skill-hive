# First hour

These steps reach a running window and `skill-hive-cli --version` without touching the real `~/.skill-hive` directory.

1. Install Node 22 (`nvm use` reads `.nvmrc`) and Rust stable (`rustup show` should report the channel in `rust-toolchain.toml`).
2. Install the platform packages in `docs/dev/toolchain.md`.
3. `npm ci`
4. `cargo test --workspace`
5. `npm test`
6. `npm run cli -- --version` prints `skill-hive-cli 0.0.1`.
7. `npm run tauri:dev` sets `SKILL_HIVE_BASE` to `.dev-base` in the repository and opens the shell at 1440 by 860. The database is `.dev-base/hive.db`.

`cargo watch` is optional and is not a Phase 0 dependency. The recipe is:

```bash
cargo install cargo-watch --locked
cargo watch -x 'test -p hive-core'
```

`npm run tauri:dev` is the GUI loop. `npm run dev` is the Vite shell alone on `127.0.0.1:1420`, with IPC mocked only in vitest.

Fixtures under `fixtures/skills/` are samples for the later validator. Phase 0 does not execute them.
