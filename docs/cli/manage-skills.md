# manage-skills

`skill-hive-cli` is the command-line client for the Skill Hive library. The desktop app copies that binary into `~/.skill-hive/bin/` and writes a `.version` stamp beside it. The dashboard card that would install this document into agents is deferred until the skill package is published.

## Envelope

Pass `--json` on every command. Success:

```json
{"ok":true,"data":{}}
```

Failure:

```json
{"ok":false,"error":{"code":"INVALID_ARGUMENT","message":"...","details":{}}}
```

Exit codes:

| Code | Meaning |
|---|---|
| 0 | Success |
| 1 | The command ran and failed |
| 2 | Usage error. The envelope code is `INVALID_ARGUMENT` |

Deploying into a directory that already has foreign files returns `TARGET_CONFLICT`. `details.conflicts` is a list of `{path, reason}` and the directory is left unchanged.

## Root

`--skills-root <path>` stores command state under `external/<sanitized-name>-<10 hex chars of sha256>/` inside the Skill Hive base. `SKILL_HIVE_BASE` and `SKILL_HIVE_CONFIG_DIR` redirect the library and the config file for tests.

## Commands

- `repo status|set-path|reset-path`
- `agents list|enable|disable|add-custom`
- `skills list|show|export|install|update|check|remove|enable|disable|deploy|undeploy|status|sync|search|set-source|adopt`
- `skills tag add|remove|list`
- `presets list|current|show|create|update|delete|preview|apply|deactivate|deploy|undeploy|status|add-skill|remove-skill`
- `git status|init|clone|set-remote|pull|push|commit|versions|restore|prune-sync-refs`

`crates/hive-cli/tests/contract.rs` runs every subcommand with `--json` against a temporary library and checks the payload against `crates/hive-cli/schemas/commands.json`.
