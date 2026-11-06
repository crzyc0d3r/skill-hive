# Instruction files

Besides skills, every agent loads a few standing files into each session: `CLAUDE.md`,
`AGENTS.md`, `GEMINI.md`, a rules folder. They live in a different place per tool, several tools
read the same `AGENTS.md`, and an edit is easy to lose. Skill Hive lists them per agent, edits
them in place and keeps a copy of every version it overwrites. The code is
`crates/hive-core/src/instructions.rs`; the app reaches it through `instructions_list`,
`instruction_read`, `instruction_write`, `instruction_delete` and `instruction_backups`, the CLI
through `hive instructions`.

## Where each agent looks

The built-in table (`INSTRUCTION_SPECS`), in the order the agents appear in Hive. User-scope
paths are relative to your home directory, project-scope paths to the project root. A folder
entry is a rules folder; its members are the files directly inside it whose names end with the
suffix in the last column.

| Agent | Scope | Path | What it is | Members |
| --- | --- | --- | --- | --- |
| Claude Code | user | `.claude/CLAUDE.md` | User memory, loaded in every project | |
| Claude Code | project | `CLAUDE.md` | Project instructions, committed with the project | |
| Claude Code | project | `.claude/CLAUDE.md` | Project instructions, alternative location | |
| Claude Code | project | `CLAUDE.local.md` | Local instructions, usually kept out of git | |
| Codex CLI | user | `.codex/AGENTS.md` | Global instructions | |
| Codex CLI | project | `AGENTS.md` | Project instructions (shared) | |
| Gemini CLI | user | `.gemini/GEMINI.md` | Global context | |
| Gemini CLI | project | `GEMINI.md` | Project context | |
| Cursor | project | `.cursor/rules/` | Rules, one per file; frontmatter sets when it applies | `.mdc` |
| Cursor | project | `.cursorrules` | Rules (legacy single file) | |
| Cursor | project | `AGENTS.md` | Project instructions (shared) | |
| GitHub Copilot | project | `.github/copilot-instructions.md` | Repository instructions | |
| GitHub Copilot | project | `.github/instructions/` | Path-specific instructions; frontmatter names the paths | `.instructions.md` |
| GitHub Copilot | project | `AGENTS.md` | Project instructions (shared) | |
| Windsurf | user | `.codeium/windsurf/memories/global_rules.md` | Global rules | |
| Windsurf | project | `.windsurf/rules/` | Rules | `.md` |
| Windsurf | project | `.windsurfrules` | Rules (legacy single file) | |
| OpenCode | user | `.config/opencode/AGENTS.md` | Global instructions | |
| OpenCode | project | `AGENTS.md` | Project instructions (shared) | |
| Amp | user | `.config/amp/AGENTS.md` | Global instructions | |
| Amp | project | `AGENTS.md` | Project instructions (shared) | |
| Goose | user | `.config/goose/.goosehints` | Global hints | |
| Goose | project | `.goosehints` | Project hints | |
| Kiro | project | `.kiro/steering/` | Steering files | `.md` |
| Cline | user | `Documents/Cline/Rules/` | Global rules | `.md` |
| Cline | project | `.clinerules` | Rules: a single file, or a folder of `.md` files | |
| Roo Code | user | `.roo/rules/` | Global rules | `.md` |
| Roo Code | project | `.roo/rules/` | Rules | `.md` |
| Roo Code | project | `.roorules` | Rules (legacy single file) | |
| Shared (`.agents`) | project | `AGENTS.md` | Project instructions (shared) | |

The project `AGENTS.md` is the shared convention: Codex CLI, Cursor, GitHub Copilot, OpenCode,
Amp and the Shared target read the same file, so it is one entry naming all of them, and an edit
for one is an edit for all. Paths are shown as `~/.claude/CLAUDE.md` at user scope and as
`CLAUDE.md` or `.cursor/rules/style.mdc` at project scope.

## Listing

`hive instructions list` (user scope) or `hive instructions list --project <root>` walks the
table for the scope and keeps a location when its agent is detected on this machine, or when the
file is on disk even though the agent is not. Locations are merged by path, in table order, so
one entry names every agent that reads it. Each entry says whether it exists, its size,
modification time, line count and estimated tokens (zero when missing or over 2 MB). A rules
folder is listed as a folder, followed by its members sorted by name; files with another suffix
or a leading dot and subfolders are ignored. A single-file location that is a folder on disk
(`.clinerules`) is listed as a folder with `.md` members. Project scope needs an existing
directory; it does not have to be a registered project.

## Reading

`hive instructions show <path>` prints the file. A known location that does not exist yet reads
as empty text with `exists: false` (the CLI warns instead of printing). A backup under the Hive
home reads the same way, labelled `Backup`. Files over 2 MB or not UTF-8 are refused.

## Writing

`hive instructions write <path> --from file.md` (or from standard input) writes the file. The
path must be a location from the table, or a new member of a rules folder: a file directly inside
it (no separators), not starting with a dot, whose name ends with the folder's suffix and is
longer than it (`.cursor/rules/style.mdc`, not `.cursor/rules/style.md` or `.cursor/rules/.mdc`).
Anything else is refused as "not an instruction file of a known agent"; the folder itself is
refused with "write one of its files instead"; text over 2 MB is refused. An existing file is
backed up first. Missing parent folders are created, so a project's first
`.cursor/rules/style.mdc` creates `.cursor/rules/`. The write is atomic: the text goes to a
temporary file next to the target, which then replaces it, so a crash leaves the old file or the
new one, never a half-written one. Each write is recorded in the audit log and emits
`instructions_changed`, which refreshes the app.

## Backups

Every overwrite and delete first copies the file to `~/.skill-hive/backups/instructions/<key>/`,
where `<key>` is the absolute path with separators and unusual characters replaced by `_`, cut to
its last 80 characters, plus a short hash so two paths never share a folder. The copy is named
`<ms>.<ext>`: the save time in milliseconds since the epoch and the file's extension
(`1730000000000.md`; a file without one, such as `.cursorrules`, gets `.txt`). Copies in the same
millisecond become `<ms>-1.<ext>`, `<ms>-2.<ext>`, and so on. Twenty are kept per file, the
oldest removed beyond that. Creating a file leaves no backup. `hive instructions backups <path>`
lists them oldest first with time, size and path. To restore one, print it and write it back:

```bash
hive instructions show ~/.skill-hive/backups/instructions/<key>/1730000000000.md \
  | hive instructions write ~/.claude/CLAUDE.md
```

The restore is a write, so the version it replaces becomes the newest backup. In the app the
**Backups** menu loads a copy into the editor and **Save** writes it back.

## Deleting

`hive instructions delete <path>` takes a backup, removes the file and records the deletion in
the audit log. A path that does not exist is an error. Rules folders are never deleted, only
their members. A deleted known location stays in the listing as missing and can be created again.

## Which project a path belongs to

A path is matched against the known locations in this order: user scope under your home
directory, then the project passed with `--project` (the project chosen in the app), then every
project registered under **Settings > Projects** (`hive project list`). The first match wins. A
file in a project that is neither passed nor registered is refused with "add the project under
Agents first, or pass its path".

## Not covered

- Files larger than 2 MB are neither read nor written; the listing still shows them.
- Agents whose instruction location Skill Hive does not know (Antigravity, OpenClaw, Hermes,
  custom agents) have no entries, and only the table's paths can be written. Open an issue with
  the location if a tool reads instructions from somewhere else.
- Settings files that are not instructions (`settings.json`, `config.toml`, MCP configuration,
  hooks) are out of scope; the table holds only the natural-language files read into the prompt.
