# Skill Hive

Desktop studio for creating, generating, and testing agent skills. The repository is `skill-hive`. Screens are grouped by feature under `src/features/`. The delivery plan remains in `PLAN.md`.

Identifier: `com.skillhive.desktop`. Database: `hive.db`. Default base directory: `~/.skill-hive`. Development base: `.dev-base` via `npm run tauri:dev`.

## Module map

```
crates/hive-core/     library, no Tauri dependency
  src/util/             errors, path guard, lock, crypto, log sanitiser, panic log, timing, audit, watcher, notifier
  src/db/               migrations v1–v8 and skill store (hive.db)
  src/library/          central repository layout and the adapter table the watcher tests need
  src/deploy/           ported deploy helpers kept so the reference tests still compile
crates/hive-cli/      skill-hive-cli
src-tauri/              Tauri 2 shell, commands, capabilities, tightened CSP
src/features/           screens, models, and tests grouped by feature
src/                    React 19 shell: sidebar, header, status bar, command palette, tokens
src/components/ui/      shadcn/ui new-york primitives
src/lib/ipc/            typed invoke wrappers and generated ipc.d.ts
fixtures/skills/        valid, warnings, and errors samples
```

`hive-core` talks to the window through the `Notifier` trait. The Tauri app implements that trait and emits `app-files-changed`.

## Attribution

Modules under `library/`, `deploy/`, and `backup/` carry the derived-file header for skills-manager commit `73d6b8f77bba6719924885c1f3be56b828d5a192`, MIT, Copyright (c) 2026 Tianliang Zhang. See `NOTICE` and `THIRD_PARTY_LICENSES/skills-manager.MIT.txt`.

## Develop

`docs/dev/first-hour.md` and `docs/dev/toolchain.md`.
