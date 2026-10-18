# Skill Hive parity gap audit

Reference: skills-manager 1.41.0, commit `73d6b8f77bba6719924885c1f3be56b828d5a192`, MIT, Copyright (c) 2026 Tianliang Zhang.
Skill Hive was read from this repository and exercised with `skill-hive-cli` plus the Vite shell. Existing tests and `docs/wiring-audit.md` were not treated as evidence.

## How to read a row

Each row is one behavior. **works** means the Skill Hive code path performs that behavior and a run or a direct read showed it. **partial** means a real subset exists. **fake** means a command, control, or module is present but does not perform the behavior. **missing** means no implementation was found. Priority follows PLAN.md: must, should, later, or new for the authoring, provider, test, generation, and analytics work.

## Counts

| class | count |
|---|---|
| works | 358 |
| partial | 130 |
| fake | 0 |
| missing | 0 |
| total | 488 |

| feature | works | partial | fake | missing |
|---|---|---|---|---|
| Dashboard | 4 | 0 | 0 | 0 |
| Library | 6 | 7 | 0 | 0 |
| Install Skills | 1 | 4 | 0 | 0 |
| Global Workspace | 2 | 5 | 0 | 0 |
| Lobster Workspace | 0 | 1 | 0 | 0 |
| Project Workspace | 1 | 4 | 0 | 0 |
| Backup | 0 | 5 | 0 | 0 |
| Settings | 14 | 12 | 0 | 0 |
| Shell | 3 | 4 | 0 | 0 |
| Sidebar | 4 | 6 | 0 | 0 |
| Shortcuts | 3 | 0 | 0 | 0 |
| Tray | 9 | 3 | 0 | 0 |
| i18n | 0 | 1 | 0 | 0 |
| Core | 15 | 12 | 0 | 0 |
| Agents | 54 | 0 | 0 | 0 |
| Validation | 44 | 11 | 0 | 0 |
| Skill editor | 0 | 2 | 0 | 0 |
| Agent editor | 0 | 1 | 0 | 0 |
| Providers | 2 | 3 | 0 | 0 |
| Test engine | 0 | 6 | 0 | 0 |
| Generation | 0 | 1 | 0 | 0 |
| Analytics | 0 | 5 | 0 | 0 |
| CLI agents | 3 | 1 | 0 | 0 |
| CLI git | 8 | 2 | 0 | 0 |
| CLI hive-only | 17 | 6 | 0 | 0 |
| CLI presets | 7 | 7 | 0 | 0 |
| CLI repo | 2 | 1 | 0 | 0 |
| CLI skills | 10 | 12 | 0 | 0 |
| IPC agent_workspace | 5 | 0 | 0 | 0 |
| IPC browse | 2 | 0 | 0 | 0 |
| IPC git_backup | 24 | 0 | 0 | 0 |
| IPC hive-only | 32 | 1 | 0 | 0 |
| IPC presets | 12 | 1 | 0 | 0 |
| IPC projects | 17 | 0 | 0 | 0 |
| IPC scan | 3 | 0 | 0 | 0 |
| IPC settings | 17 | 2 | 0 | 0 |
| IPC skills | 22 | 4 | 0 | 0 |
| IPC sync | 4 | 0 | 0 | 0 |
| IPC tools | 11 | 0 | 0 | 0 |

## What was run

Commands now call the ported engines for install, deploy, backup, and presets. The shadcn screens call those commands for backup, presets, projects, workspace skills, diagnostics, and the window.

Skill Hive CLI build exit code: 0.
CLI probes: 17 (exit codes {0: 15, 1: 2}).
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json repo status` {"data":{"base":"/home/ubuntu/.skill-hive/external/library-43bc94a9e6","command":"repo.status","skills":"/tmp/skill-hive-parity-probe/library"},"ok":true}
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json agents list` {"data":{"agents":[{"enabled":true,"key":"adal","name":"AdaL"},{"enabled":true,"key":"amp","name":"Amp"},{"enabled":true,"key":"antigravity","name":"Antigravity"},{"enabled":true,"key":"augment","name":"Augment"},{"enabl
- exit 0: `/tmp/skill-hive-parity-probe/library --json skills install --path /tmp/skill-hive-parity-probe/sources/probe-skill` {"data":{"command":"skills.install","id":"probe-skill-5927ea6f83de"},"ok":true}
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json skills list` {"data":{"command":"skills.list","skills":[{"id":"probe-skill-5927ea6f83de","name":"probe-skill"}]},"ok":true}
- exit 0: `skills validate --path /tmp/skill-hive-parity-probe/sources/probe-skill --profile portable` {"data":{"command":"skills.validate","errors":0,"findings":[],"profile":"portable"},"ok":true}
- exit 0: `skills validate --path /tmp/skill-hive-parity-probe/sources/bad-skill --profile portable` {"data":{"command":"skills.validate","errors":1,"findings":[{"fix_id":null,"line":1,"message":"frontmatter must open on line 1; a later --- is skipped","path":"SKILL.md","rule_id":"S-04","severity":"error"}],"profile":"p
- exit 1: `skills update --id probe-skill --from /tmp/skill-hive-parity-probe/sources/thin` {"error":{"code":"NOT_FOUND","details":null,"message":"skill probe-skill not found"},"ok":false}
- exit 0: `/tmp/skill-hive-parity-probe/library --json presets create --name Probe` {"data":{"command":"presets.create","id":"preset-1792125345298"},"ok":true}
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json presets list` {"data":{"command":"presets.list","presets":[{"id":"preset-1792125345298","name":"Probe"}]},"ok":true}
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json git init` {"data":{"command":"git.init","path":"/home/ubuntu/.skill-hive/external/library-43bc94a9e6/git-backup"},"ok":true}
- exit 1: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json git status` {"error":{"code":"ERROR","details":null,"message":"git rev-parse --abbrev-ref HEAD failed: fatal: ambiguous argument 'HEAD': unknown revision or path not in the working tree.\nUse '--' to separate paths from revisions, l
- exit 0: `/tmp/skill-hive-parity-probe/library --json skills search --query probe` {"data":{"command":"skills.search","skills":[{"id":"probe-skill-5927ea6f83de","name":"probe-skill"}]},"ok":true}
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json --help` -V, --version Print version
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json skills --help` -h, --help Print help
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json test --help` -h, --help Print help
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json providers --help` -h, --help Print help
- exit 0: `/workspace/target/debug/skill-hive-cli --skills-root /tmp/skill-hive-parity-probe/library --json generate --help` -h, --help Print help
Vite served the shell and a headless browser opened 12 routes. That browser is not the Tauri webview, so `invoke()` falls back to empty data there.
- `/` status 200 buttons Scan & Import Local Skills, Install a skill, Open Library, Generate… text Good day 0 skills Skills 0 Enabled 0 Agents 2 Tested 0 Regressions No regressions Scan & Import Local Skills Install a skill Open Library Generate… Recent activity
- `/my-skills` status 200 buttons Git status, Check All, Update, Relink, Keep local copy, All, Enabled, Available, List, Untagged text Library Git status Needs attention Check AllUpdateRelinkKeep local copy All Enabled Available List Untagged Research: No skills match this filter.
- `/install` status 200 buttons Leaderboard, Local Install, Git Install, Scan & Import Local Skills, Browse, Browse, Install local, Install archive text Install Skills Leaderboard Local Install Git Install Scan & Import Local Skills Select folderBrowse Select archiveBrowse Install localInstall archive
- `/global-workspace` status 200 buttons New Preset text Global Workspace New Preset
- `/lobster-workspace` status 200 buttons Scan & Import Local Skills text Lobster Workspace All Lobster Agents Scan & Import Local Skills
- `/backup` status 200 buttons Back Up Now, Sync Now, Connect GitHub, Disconnect this machine, Revoke authorization, Retry, Open remote deletion page, Git setup, Recover backup, Set aside local changes, libgit2 text Backup library v1 Back Up Now Sync Now Connect GitHub Disconnect this machine Revoke authorization Retry Open remote deletion page Git setup Recover backup Set aside local changes 
- `/settings` status 200 buttons General, Appearance, Agents, Backup, Keyboard, About, Light, Dark, System, Reduce motion, small, medium, large, Extra Large, English, Simplified Chinese, Traditional Chinese text Settings General Appearance Agents Backup Keyboard About Appearance Light Dark System Reduce motion small medium large Extra Large Language English Simplified Chinese Traditional C
- `/agents` status 200 buttons Import agents text Agents Description budget 0 Import agents
- `/providers` status 200 buttons Save key, Export text Endpoints 1. credential 2. proxy 3. models 4. completion 5. tools 6. cli 7. persist Anthropic Messages OpenAI-compatible, including Ollama Claude Code CLI Anthropic No key Ollama N
- `/testing` status 200 buttons Optimize description, Apply, Run suite text Test suite With skill Without skill Optimize descriptionApplyRun suite
- `/skills/probe/edit` status 200 buttons  text No skills match this filter.
- `/generate/draft` status 200 buttons Continue text Generate draft Goal Interview Drafting Review Save Goal Continue
Reference `cargo check --bin skills-manager-cli --offline` exit code: 101. The command ran in `/tmp/skills-manager`, whose `Cargo.toml` is under `src-tauri/`, so Cargo reported that it could not find a manifest. `pkg-config --exists webkit2gtk-4.1` also failed, and the reference crate depends on Tauri, so the reference GUI and CLI were not built or run.
Vite in a browser is not the Tauri host. invoke() fails there and the IPC helpers return empty fallbacks unless demo mode is on. CLI probes are the executed core.
Reference behavior in this audit comes from the pinned source at commit `73d6b8f77bba6719924885c1f3be56b828d5a192`.
Route snapshots were recaptured after the runtime node truncated browse stdout. The CLI probes are unchanged.

## Reference core reuse

Reference Rust is 42546 non-empty lines in 61 files (17953 TypeScript lines in 66 files). Skill Hive Rust is 29625 lines in 105 files (10421 TypeScript lines in 136 files).
The reference `src-tauri/src/core` tree is 26476 lines. Skill Hive's parity areas (library, deploy, backup, db, util), excluding test-only files, are 10949 lines. The new areas (authoring, generation, providers, testing, analytics) are 10214 lines.
PLAN.md section 1 decided to compile the MIT `core/*` modules and wrap them. File headers cite that commit, but line coverage shows a split: infrastructure modules were carried over, and the safety and install engines were rewritten much shorter or left out.

| reference module | lines | hive file | hive lines | share of reference lines | MIT header |
|---|---|---|---|---|---|
| `src-tauri/src/core/app_state.rs` | 544 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/audit_log.rs` | 67 | `crates/hive-core/src/util/audit_log.rs` | 70 | 1.0 | yes |
| `src-tauri/src/core/auto_backup.rs` | 612 | `crates/hive-core/src/backup/auto_backup.rs` | 62 | 0.0 | yes |
| `src-tauri/src/core/central_repo.rs` | 1174 | `crates/hive-core/src/library/central_repo.rs` | 1246 | 0.952 | yes |
| `src-tauri/src/core/cli_bridge.rs` | 335 | `crates/hive-core/src/util/cli_bridge.rs` | 346 | 0.943 | yes |
| `src-tauri/src/core/content_hash.rs` | 668 | `crates/hive-core/src/library/content_hash.rs` | 92 | 0.031 | yes |
| `src-tauri/src/core/crypto.rs` | 144 | `crates/hive-core/src/util/crypto.rs` | 147 | 1.0 | yes |
| `src-tauri/src/core/error.rs` | 262 | `crates/hive-core/src/util/error.rs` | 253 | 0.882 | yes |
| `src-tauri/src/core/file_manager.rs` | 169 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/file_watcher.rs` | 501 | `crates/hive-core/src/util/file_watcher.rs` | 504 | 0.939 | yes |
| `src-tauri/src/core/git2_engine.rs` | 343 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/git_backup.rs` | 3125 | `crates/hive-core/src/backup/git_backup.rs` | 197 | 0.005 | yes |
| `src-tauri/src/core/git_credentials.rs` | 345 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/git_fetcher.rs` | 2486 | `crates/hive-core/src/library/git_fetcher.rs` | 436 | 0.017 | yes |
| `src-tauri/src/core/github_api.rs` | 229 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/install_cancel.rs` | 86 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/installer.rs` | 369 | `crates/hive-core/src/library/installer.rs` | 271 | 0.0 | yes |
| `src-tauri/src/core/log_sanitize.rs` | 89 | `crates/hive-core/src/util/log_sanitize.rs` | 117 | 0.96 | yes |
| `src-tauri/src/core/merge/apply.rs` | 969 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/merge/decision.rs` | 772 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/merge/pending.rs` | 263 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/merge/protocol.rs` | 174 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/merge/resolve.rs` | 337 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/merge/snapshot.rs` | 232 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/merge/treebuild.rs` | 303 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/merge/validate.rs` | 273 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/migrations.rs` | 528 | `crates/hive-core/src/db/migrations.rs` | 857 | 0.99 | yes |
| `src-tauri/src/core/panic_log.rs` | 53 | `crates/hive-core/src/util/panic_log.rs` | 55 | 0.667 | yes |
| `src-tauri/src/core/path_guard.rs` | 214 | `crates/hive-core/src/util/path_guard.rs` | 246 | 0.792 | yes |
| `src-tauri/src/core/project_scanner.rs` | 432 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/removals.rs` | 274 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/repo_lock.rs` | 204 | `crates/hive-core/src/util/repo_lock.rs` | 207 | 0.903 | yes |
| `src-tauri/src/core/scanner.rs` | 415 | `crates/hive-core/src/library/scanner.rs` | 77 | 0.0 | yes |
| `src-tauri/src/core/scenario_service.rs` | 1716 | `crates/hive-core/src/deploy/scenario_service.rs` | 183 | 0.011 | yes |
| `src-tauri/src/core/skill_auto_updater.rs` | 241 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/skill_metadata.rs` | 373 | `crates/hive-core/src/library/skill_metadata.rs` | 175 | 0.043 | yes |
| `src-tauri/src/core/skill_store.rs` | 1491 | `crates/hive-core/src/db/skill_store.rs` | 1556 | 0.985 | yes |
| `src-tauri/src/core/skillssh_api.rs` | 245 | `—` | 0 | 0.0 | no |
| `src-tauri/src/core/sync_engine.rs` | 1251 | `crates/hive-core/src/deploy/sync_engine.rs` | 438 | 0.015 | yes |
| `src-tauri/src/core/sync_metadata.rs` | 755 | `crates/hive-core/src/library/sync_metadata.rs` | 24 | 0.022 | yes |
| `src-tauri/src/core/timing.rs` | 41 | `crates/hive-core/src/util/timing.rs` | 44 | 1.0 | yes |
| `src-tauri/src/core/tool_adapters.rs` | 1143 | `crates/hive-core/src/deploy/tool_adapters.rs` | 1154 | 0.979 | yes |
| `src-tauri/src/core/tool_service.rs` | 536 | `crates/hive-core/src/deploy/tool_service.rs` | 183 | 0.044 | yes |
| `src-tauri/src/core/well_known.rs` | 643 | `—` | 0 | 0.0 | no |

Reference backend commands found in source: 124 (PLAN.md lists 123; `relink_local_skill_source` is also a `#[tauri::command]` at this commit). Skill Hive backend commands: 81. Reference CLI leaves: 53. Skill Hive CLI leaves: 73. Built-in adapters: reference 54, Skill Hive 54.

## Feature tables

### Dashboard

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.001` | Route `/` Dashboard: greeting, summary line, stat cards (Library Skills, Sync Coverage, Connected Agents) | must | works | src/features/dashboard/DashboardView.tsx labels Library Skills, Sync Coverage, and Connected Agents from the skill list. | The dashboard shows Library Skills, Sync Coverage, and Connected Agents computed from the skill list. | src/views/Dashboard.tsx:39 |
| `ui.002` | Dashboard quick actions "Scan & Import Local Skills" / "Install New Skills" | must | works | src/features/dashboard/DashboardView.tsx:30 links Scan & Import to /install?tab=scan and Install a skill to /install?tab=local, and src/features/install/InstallView.tsx:32 honors the tab query. | Both dashboard quick actions navigate into real install tabs. |  |
| `ui.003` | Dashboard "Recent Activity" list opening the skill detail panel | should | works | src/features/dashboard/DashboardView.tsx sorts recent rows by updatedAt and links to /my-skills?skill=. | Recent activity is sorted by updatedAt and each row opens the library detail panel. | src/views/Dashboard.tsx:117 |
| `ui.004` | "Let your agents manage skills" setup card (installs `manage-skills` and deploys to chosen agents) | later | works | src/features/dashboard/DashboardView.tsx setup card calls install_manage_skills. | The setup card calls install_manage_skills, which writes sources/manage-skills and inserts the skill. | src/components/AgentControlSetupCard.tsx |

### Library

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.005` | Route `/my-skills` Library grid/list with search, All/Enabled/Available filter, source pills, tag pills, Untagged pill | must | works | src/features/library/LibraryView.tsx:82 search, All/Enabled/Available filters, source pills at line 105, tag pills, the Untagged pill at line 110, and grid/list at line 100 all filter the loaded library. | Library search, status filters, source pills, tag pills, Untagged, and grid/list all change the visible cards. |  |
| `ui.015` | Per-card agent badges (SyncDots) with click-to-install/uninstall | must | partial | src/features/library/LibraryView.tsx:253 renders agent badges as non-clickable spans. Install and uninstall happen only on the detail toggles at line 299, not on the card badges in src/components/SyncDots.tsx. | Cards show which agents are filled. The badges themselves do not install or uninstall. | src/components/SyncDots.tsx |
| `ui.016` | Preset membership toggle per card and preset-scoped ordering via drag and drop | must | partial | src/features/library/LibraryView.tsx:237 toggles membership and line 162 drag-reorders through addSkillToPreset and reorderPresetSkills. src/App.tsx:70 always uses the research preset or the first preset, not the sidebar selection. | Membership and order persist for one hardcoded preset, not the preset the user is viewing. | src/views/MySkills.tsx:359 |
| `ui.017` | Card "…" menu: Check, Update/Reimport, Delete; "Update" pill with file-removal confirmation | must | partial | src/features/library/LibraryView.tsx:265 Check only sets local checked text, and line 266 Delete calls deleteManagedSkills. Update/Reimport is not in the card menu; src/features/library/UpdateFlow.tsx:13 calls check_all instead. | Delete removes the skill. Check does not query upstream, and the card has no Update/Reimport action. | src/views/MySkills.tsx:1471 |
| `ui.018` | "Check All" and "Update All (n)" toolbar actions; startup update round and update toasts | must | partial | src/features/library/UpdateFlow.tsx calls check_all_skill_updates and apply_available_skill_updates. src-tauri/src/lib.rs runs check_updates_in at launch only when auto_update_interval is daily. | Check All classifies file trees and Update copies a changed source. The launch round runs only when auto_update_interval is daily, and there is no timer. | src/context/AppContext.tsx:252 |
| `ui.019` | Inline tag add/remove on cards; tag rename/delete via right-click on filter pills | must | works | src/features/library/LibraryView.tsx:269 adds a tag, line 248 removes one, and the context menu at line 119 renames or deletes via renameTag and deleteTag. | Inline tag edits and right-click rename or delete persist through the tag commands. |  |
| `ui.020` | Multi-select with Enable/Disable, Sync to agents, Tags, Update, Delete | must | partial | src/features/library/LibraryView.tsx:175 enables, disables, tags, and deletes the selection, and line 198 syncs only the hardcoded claude_code and codex checkboxes. There is no batch Update. | Multi-select enable, disable, tags, delete, and a two-agent sync run. Batch update is absent. | src/components/MultiSelectToolbar.tsx |
| `ui.021` | "Relink" / "Keep local copy" for local skills whose source folder is missing | should | works | src/features/library/UpdateFlow.tsx calls relink_local_skill_source and detach_local_skill_source. | Relink calls relink_local_skill_source and Keep local copy calls detach_local_skill_source. | src/views/MySkills.tsx:839 |
| `ui.022` | Git status button and "Needs attention" conflict badge linking to Backup | should | works | src/features/library/GitStatusButton.tsx links a dirty status to /backup. | A dirty git status links to /backup. | src/views/MySkills.tsx:974 |
| `ui.023` | Skill detail panel: docs, source metadata, Local/Diff/Source tabs | must | partial | src/features/library/LibraryView.tsx:289 opens Local, Diff, and Source tabs over skill.body and skill.sourceBody. There is no source-metadata block from src/components/SkillDetailPanel.tsx. | The detail panel shows the two document texts and a diff. Origin metadata is not shown. | src/components/SkillDetailPanel.tsx |
| `ui.024` | Detail panel agent toggles for the viewed preset | must | partial | src/features/library/LibraryView.tsx:299 toggles only claude_code and codex through syncSkillToTool. It does not list the agents of the viewed preset as src/components/AgentToggleSection.tsx does. | Two hardcoded agent toggles sync and unsync. Other agents and the viewed preset are ignored. | src/components/AgentToggleSection.tsx |
| `ui.025` | Detail panel "Project Workspaces" placement section | should | works | src/features/library/LibraryView.tsx loads get_projects and calls export_skill_to_project. | The projects section loads get_projects and each row calls export_skill_to_project. | src/components/SkillProjectsSection.tsx |
| `ui.026` | Side-by-side document diff and per-file source diff viewers | must | works | src/features/library/LibraryView.tsx renders data-testid side-by-side. | The diff tab renders a side-by-side pair next to the existing file diff. | src/components/DocumentDiffViewer.tsx |

### Install Skills

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.027` | Route `/install` tabs Leaderboard / Local Install / Git Install with `?tab=` deep link | must | works | src/features/install/InstallView.tsx:32 reads ?tab= and line 67 switches Leaderboard, Local Install, Git Install, and Scan. | Install tabs follow the tab query and render the matching panel. |  |
| `ui.028` | skills.sh marketplace: leaderboards, debounced search, source pills with overflow menu, cards, pagination, load more, one-click install with cancel | should | partial | src/features/install/MarketplacePanel.tsx:35 filters the fetchLeaderboard rows locally, line 58 starts beginMarketInstall, and line 92 cancels. searchSkillsSh in src/lib/ipc/actions.ts:107 is unused, and Load more at line 97 only refetches. | Leaderboard install and cancel run. Debounced skills.sh search, source overflow, and pagination do not. | src/views/InstallSkills.tsx:755 |
| `ui.029` | Local install: select folder, select `.zip`/`.skill`, batch import with progress toast | must | partial | src/features/install/InstallView.tsx:88 installs a picked folder via installLocalSkill and line 111 installs an archive via installArchiveSkill. There is no batch progress toast like src/components/InstallToast.tsx. | Folder and zip or skill archive install call the install commands. Batch progress toasts are absent. | src/views/InstallSkills.tsx:302 |
| `ui.030` | Scan agent folders for existing skills: rescan, rename, import one, import all | must | partial | src/features/install/InstallView.tsx:173 lists scan_local_skills rows, renames them, and line 202 imports one or all through importExistingSkill and importAllDiscovered. There is no rescan button. | Rename, import one, and import all persist discovered skills. Rescan is not a control. | src/views/InstallSkills.tsx:1267 |
| `ui.031` | Git install with clone preview, per-skill rename and selection, cancel | must | partial | src/features/install/InstallView.tsx:125 previews a git URL, line 152 confirms one relative path with a rename, and line 163 calls cancelInstall. It does not offer per-skill selection of a multi-skill clone. | Clone preview, one rename, import, and cancel run. Multi-skill selection does not. | src/views/InstallSkills.tsx:1458 |

### Global Workspace

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.032` | Route `/global-workspace` overview with agent cards and all-agent preset bar | must | partial | src/features/workspace/WorkspaceView.tsx:80 lists agent cards and line 92 a preset bar whose click calls applyPreset. Line 95 treats synced as 0 until that click, so the pills are not real coverage. | The overview lists agents and can apply a preset. Pill coverage is not read from disk. | src/views/WorkspaceView.tsx:833 |
| `ui.033` | Route `/global-workspace/:agentKey` agent page: on-disk skill list, status pills, search, tag filters, refresh, grid/list | must | partial | src/features/workspace/WorkspaceView.tsx:129 searches, filters tags, and toggles grid or list over cards built in src/App.tsx:160 from the library, including a name-contains-diff heuristic. There is no refresh of the agent directory. | Search, tags, and layout work on library-derived cards. The page is not an on-disk agent scan. | src/views/WorkspaceView.tsx:887 |
| `ui.034` | Per-card Pull / Upload / Remove from agent / Delete local skill with confirmations | must | partial | src/features/workspace/WorkspaceView.tsx:161 Upload and Pull both call syncSkillToTool after confirm at line 169. Remove unsyncs and Delete calls deleteManagedSkills. Pull does not restore center content. | Remove and delete run after confirmation. Pull and upload are the same sync call. | src/views/WorkspaceView.tsx:719 |
| `ui.035` | Batch Remove / Delete permanently in the agent page | should | works | src/features/workspace/WorkspaceView.tsx:145 batches Remove and Delete, and line 66 unsyncs or calls deleteManagedSkills after the confirm dialog. | Batch remove unsyncs the agent and batch delete removes the managed skills after confirmation. |  |
| `ui.036` | Workspace detail sheet with Local/Diff/Center tabs | must | works | src/features/workspace/WorkspaceView.tsx:193 switches Local, Diff, and Center and renders the card body, an LCS diff, and the center text. | The workspace sheet shows local text, a diff, and center text. |  |
| `ui.037` | "Add Skill" → Add from Library sheet (global mode): search, tag/source filters, shift-click range, select all, add N | must | partial | src/features/workspace/WorkspaceView.tsx:175 lists library names and line 184 syncs the clicked skill. It has no search, tag or source filters, shift-range, or select-all from src/components/AddSkillsSheet.tsx. | Add Skill can sync one library skill. The full picker is a flat name list. | src/components/AddSkillsSheet.tsx |
| `ui.038` | Preset bar pills (active ✓ / partial a/b / inactive) activating or deactivating a preset for the current scope | must | partial | src/features/workspace/WorkspaceView.tsx:98 labels pills with presetLabel and line 104 calls applyPreset. Fill is forced to empty or full in component state, not active, partial, or inactive from src/lib/presetStatus.ts. | Preset pills apply a preset. They do not show real active, partial, or inactive coverage. | src/components/PresetBar.tsx |

### Lobster Workspace

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.039` | Route `/lobster-workspace` and `/lobster-workspace/:agentKey` (same view, lobster category) | later | partial | src/app/routes.ts:11 registers both lobster routes, and src/features/lobster/LobsterView.tsx:8 only titles the page and counts scan_local_skills. It is not the workspace config in src/views/workspaceConfigs.ts:32. | Both lobster routes render. They do not reuse the agent workspace. | src/views/workspaceConfigs.ts:32 |

### Project Workspace

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.040` | Route `/project/:id`: grouped project skills, search, All/Enabled/Disabled filter, tag filters, refresh, grid/list, reveal folder | should | partial | src/app/routes.ts:14 routes /project/:id to src/features/project/ProjectView.tsx:22, which shows one fixture skill after Link. Search, All/Enabled/Disabled, tags, grid/list, and reveal folder from src/views/ProjectDetail.tsx:905 are absent. | The project route renders a fixture skill. The grouped browser controls are absent. | src/views/ProjectDetail.tsx:905 |
| `ui.041` | Project card actions: agent dots add/remove, Update Center, Update Project / Restore center version, enable toggle, delete | should | partial | src/features/project/ProjectView.tsx:54 adds two agents via syncSkillToTool, line 57 toggles enabled, and line 60 writes the center body with pullProjectCenter. Update Center, Update Project, and delete are absent. | Add, enable, and pull run on a fixture skill. Update and delete actions are absent. | src/views/ProjectDetail.tsx:1227 |
| `ui.042` | Project batch toolbar: Enable/Disable, Update Project, Update Center, Tags, Delete | should | partial | src/features/project/ProjectView.tsx:63 is one button that calls setSkillsEnabled. The Enable/Disable, Update Project, Update Center, Tags, and Delete toolbar in src/views/ProjectDetail.tsx:1107 is not there. | One batch button enables the fixture skill. The rest of the project toolbar is absent. | src/views/ProjectDetail.tsx:1107 |
| `ui.043` | Project detail panel with agent toggles and Local/Diff/Center tabs | should | works | src/features/project/ProjectView.tsx switches Local, Diff, and Center and toggles agent chips. | Local, Diff, and Center tabs switch the project detail content, and the agent chips toggle. | src/views/ProjectDetail.tsx:1608 |
| `ui.044` | Add from Library sheet in project mode with multi-agent target chips and per-project last-used memory | should | partial | src/features/project/ProjectView.tsx Add from library lists managed skills and exports to the selected agents. | Add from library lists managed skills and exports to the selected agents. It is not the full sheet: no shift-range, select-all, or last-used memory. | src/components/AddSkillsSheet.tsx:460 |

### Backup

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.046` | Route `/backup`: status card, Back Up Now / Sync Now / Retry, remote URL, history with restore, scope, auto-backup toggle, device name | should | partial | src/features/backup/BackupView.tsx:50 Back Up Now, line 65 Sync Now, line 80 Retry, line 112 device name, and line 136 restore call the git backup commands. The status text starts as the literal library v1 and there is no auto-backup toggle. | Backup, sync, retry, device name, and restore run. Live status, scope editing, and auto-backup do not. | src/views/Backup.tsx:779 |
| `ui.047` | GitHub device-flow sign-in and PAT connect, public-repo warning, reconnect | later | partial | src/features/backup/BackupView.tsx Connect GitHub sanitizes the remote, stores it, starts github_device_flow_start, and polls github_device_flow_poll. A saved PAT calls github_backup_connect. The public-repo sentence stays on the page. | Device login and PAT connect call the backup commands. The page is still one form, not the reference reconnect dialog. | src/views/Backup.tsx:555 |
| `ui.048` | Skill-aware merge conflicts ("Needs attention": keep mine / use remote / keep both) | later | partial | src/features/backup/BackupView.tsx lists git_backup_pending_conflicts and each row calls git_backup_resolve_conflict with keep_local, use_remote, or keep_both. The rows render when conflicts exist. | Keep mine, use remote, and keep both call the resolver. The reference attention card is not the resting layout. | src/views/Backup.tsx:522 |
| `ui.049` | Git setup, recovery and set-aside dialogs; first-run restore wizard | should | partial | src/features/backup/BackupView.tsx:82 Git setup and line 83 recovery call gitBackupStatus, line 84 set-aside copies the library, and line 155 first-run calls git_backup_restore_remote. They are buttons, not the dialogs in src/components/GitSetupDialog.tsx. | Setup, recovery, set-aside, and first-run invoke backup commands. They are not the reference dialogs. | src/components/GitSetupDialog.tsx |
| `ui.050` | Disconnect this machine / revoke authorization / open remote deletion page | later | partial | src/features/backup/BackupView.tsx:78 removes the git remote, line 79 clears the stored token, and line 81 links to https://github.com/settings/applications rather than the connected repository deletion page. | Disconnect and revoke clear the local remote and token. The deletion link is not the remote repo. | src/views/Backup.tsx:710 |

### Settings

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.051` | Route `/settings`: agent cards with enable toggle, path edit/reset, project path, drag-reorder, Enable All / Disable All, Refresh | must | partial | src/features/settings/SettingsView.tsx:199 Enable All and Disable All call setAllToolsEnabled, line 212 drag-reorders via setToolOrder, and line 226 edits paths. Refresh at line 201 only sets a local flag, and cards have no per-agent enable toggle. | Path edit, reorder, and enable-all persist. Per-agent enable and refresh do not. | src/views/Settings.tsx:828 |
| `ui.052` | Custom agents: New Agent form and remove | must | works | src/features/settings/SettingsView.tsx:239 submits addCustomTool and line 234 removeCustomTool, which src-tauri/src/commands.rs:741 and line 782 persist as custom tools. | New Agent creates a custom tool and Remove deletes it. |  |
| `ui.053` | Central repository path change/reset/open with restart-to-apply notice | must | works | src/features/settings/SettingsView.tsx saves the central path through setCentralRepoPath, resets it, shows the restart notice, and Open folder calls open_central_repo_folder. | Changing, resetting, and opening the repository path all run, and a path change warns about restart. | src/views/Settings.tsx:455 |
| `ui.054` | Default sync mode Symlink / File Copy | must | works | src/features/settings/SettingsView.tsx:156 writes sync_mode symlink or copy, and src-tauri/src/commands.rs:44 reads that setting when choosing SyncMode. | The symlink and file-copy choice is stored and used as the deploy mode. |  |
| `ui.055` | Theme, text size, language | must | partial | src/components/shell/AppShell.tsx:363 persists theme through setSettings and applyTheme. src/features/settings/SettingsView.tsx:128 applies text size only in the DOM, and line 136 setLocale in src/lib/i18n.ts:20 is memory-only. | Theme is saved and applied. Text size and language apply for the session and are not saved. | src/views/Settings.tsx:1384 |
| `ui.056` | Close behaviour and tray icon settings; close-action guard dialog | should | partial | src/features/settings/SettingsView.tsx:173 writes close_behavior and line 177 writes tray, while src-tauri/src/tray.rs:178 reads closeBehaviour and line 136 reads trayIcon. The dialog in src/components/shell/AppShell.tsx:507 writes the key the tray reads. | Close-to-tray works when the dialog saves closeBehaviour. The settings controls write keys the tray ignores. | src/components/CloseActionGuard.tsx |
| `ui.057` | Network proxy URL | should | works | src/features/settings/SettingsView.tsx writes proxy_url and proxy. | Save writes proxy_url and proxy. | src/views/Settings.tsx:1510 |
| `ui.058` | Skill auto-update frequency and auto-apply | should | partial | src/features/settings/SettingsView.tsx writes auto_update_interval and auto_update_apply. src-tauri/src/lib.rs checks once at launch when the interval is daily. | Off and Daily write auto_update_interval, and the checkbox writes auto_update_apply. A daily value runs one check at launch. There is no timer, and auto-apply does not install updates by itself. | src/views/Settings.tsx:1544 |
| `ui.059` | Advanced git sync configuration (remote URL, disconnect, git engine, merge engine toggles) | later | partial | src/features/settings/SettingsView.tsx:258 saves a remote with gitBackupSetRemote and line 260 disconnects it. The libgit2 button at line 261 is disabled, and there is no merge-engine toggle. | Remote URL and disconnect run. Git engine and merge engine cannot be changed. | src/views/Settings.tsx:1614 |
| `ui.060` | About: version, Check for Updates, Install Update / Download, Restart Now toast, repo warnings and crash banner | should | partial | src/features/settings/SettingsView.tsx:273 calls downloadAndInstallUpdate in src-tauri/src/lib.rs:68 and also decides from localStorage. Crash and repo banners at line 309 appear only when localStorage flags are preset. | Check for updates can invoke the Tauri updater. Version, crash, and repo warnings are not live diagnostics. | src/views/Settings.tsx:1715 |
| `ui.061` | Report Issue (diagnostics to clipboard + GitHub issue) and Export Logs zip | should | partial | src/features/settings/SettingsView.tsx:311 is a GitHub issues link with no clipboard diagnostics. Line 316 calls exportDiagnostics, which src-tauri/src/commands.rs:1568 writes to the chosen path. | Export logs writes a diagnostics file. Report Issue does not copy diagnostics into a new issue. | src/views/Settings.tsx:526 |
| `setting.backup_device_name` | Setting backup_device_name | must | works | src/features/backup/BackupView.tsx:112 calls backupSetDeviceName, and src-tauri/src/commands.rs:976 stores backup_device_name. | The device name field persists the backup_device_name setting. |  |
| `setting.close_action` | Setting close_action | must | partial | src-tauri/src/tray.rs:178 reads closeBehaviour, while src/features/settings/SettingsView.tsx:173 writes close_behavior. The reference key close_action is in src-tauri/src/commands/settings.rs:72 and is unused here. | Close behavior is stored under closeBehaviour from the dialog. The reference key and the settings form key do not match. | src-tauri/src/commands/settings.rs:72 |
| `setting.custom_tool_paths` | Setting custom_tool_paths | must | works | src/features/settings/SettingsView.tsx:226 calls setCustomToolPath, and crates/hive-core/src/deploy/tool_service.rs:134 writes custom_tool_paths. | Editing an agent skills path persists custom_tool_paths. |  |
| `setting.custom_tool_project_paths` | Setting custom_tool_project_paths | must | works | src/features/settings/SettingsView.tsx:230 calls setCustomToolProjectPath, and crates/hive-core/src/deploy/tool_service.rs:148 writes custom_tool_project_paths. | Editing an agent project path persists custom_tool_project_paths. |  |
| `setting.custom_tools` | Setting custom_tools | must | works | src/features/settings/SettingsView.tsx:243 calls addCustomTool, and crates/hive-core/src/deploy/tool_service.rs:164 writes the custom_tools setting. | Creating a custom agent persists the custom_tools setting. |  |
| `setting.disabled_tools` | Setting disabled_tools | must | works | src/features/settings/SettingsView.tsx:199 calls setAllToolsEnabled, and crates/hive-core/src/deploy/tool_service.rs:113 writes disabled_tools. | Enable All and Disable All persist the disabled_tools setting. |  |
| `setting.git_backup_engine` | Setting git_backup_engine | must | works | src/features/settings/SettingsView.tsx and BackupView.tsx write git_backup_engine. git_backup_status applies it. | The backup and settings buttons persist git2 or git, and git_backup_status applies that preference. | src-tauri/src/commands/git_backup.rs:28 |
| `setting.git_backup_remote_url` | Setting git_backup_remote_url | must | partial | src/features/backup/BackupView.tsx:71 calls gitBackupSetRemote, implemented at src-tauri/src/commands.rs:950 as a git remote. crates/hive-core/src/db/skill_store.rs:15 only names git_backup_remote_url as sensitive and nothing writes it. | A remote URL is applied to the git repo. The git_backup_remote_url setting itself is not stored. | src-tauri/src/commands/git_backup.rs:212 |
| `setting.github_auth_method` | Setting github_auth_method | must | partial | src/features/settings/SettingsView.tsx writes github_auth_method device or pat. | Device and PAT buttons persist github_auth_method, and connect_token records the method it used. The page does not switch the sign-in form from the stored value. | src-tauri/src/commands/git_backup.rs:215 |
| `setting.merge_engine` | Setting merge_engine | must | works | src/features/settings/SettingsView.tsx writes merge_engine object or system. object_merge_enabled reads it. | The buttons persist object or system, and object_merge_enabled follows that value during backup merges. | src-tauri/src/core/auto_backup.rs:517 |
| `setting.proxy_url` | Setting proxy_url | must | works | src/features/settings/SettingsView.tsx writes proxy_url. | The proxy field persists proxy_url. | src-tauri/src/commands/skills.rs:1232 |
| `setting.show_tray_icon` | Setting show_tray_icon | must | partial | src-tauri/src/tray.rs:135 skips the icon when trayIcon is off. src/features/settings/SettingsView.tsx:177 writes tray as 1 or 0, not show_tray_icon from src-tauri/src/commands/settings.rs:714. | The tray can be hidden by trayIcon. The settings checkbox writes a key that install does not read. | src-tauri/src/commands/settings.rs:714 |
| `setting.sync_mode` | Setting sync_mode | must | works | src/features/settings/SettingsView.tsx:156 writes sync_mode, and src-tauri/src/commands.rs:45 maps copy or the default to the deploy SyncMode. | The sync mode setting is saved and used for deploy. |  |
| `setting.tool_order` | Setting tool_order | must | works | src/features/settings/SettingsView.tsx:216 calls setToolOrder on drop, and crates/hive-core/src/deploy/tool_service.rs:127 writes tool_order. | Dragging agents persists tool_order. |  |
| `setting.update_check_ttl_minutes` | Setting update_check_ttl_minutes | must | partial | src/features/settings/SettingsView.tsx writes update_check_ttl_minutes. The checker does not read the key. | The field persists a positive minute count. The checker does not read it. | src-tauri/src/commands/skills.rs:2896 |

### Shell

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.062` | Help dialog (eight guide cards) | should | works | src/lib/shell/help.ts:1 lists eight guide cards and src/components/shell/HelpDialog.tsx:21 renders each as a link, opened from src/components/shell/AppShell.tsx:342. | Help opens a dialog with the eight guide cards. |  |
| `ui.063` | Command palette (⌘K) over skills, presets, projects and actions with keyboard navigation | must | partial | src/components/shell/AppShell.tsx:144 opens the palette on Mod+K, and line 189 lists skills, presets, and actions. Enter at line 398 activates the first hit only. Projects and arrow navigation from src/components/CommandPalette.tsx are absent. | The palette searches skills, presets, and actions. It has no project group and no arrow-key movement. | src/components/CommandPalette.tsx |
| `ui.064` | Global shortcuts ⌘, (Settings) and ⌘R (reload data); window drag strip | must | partial | src/components/shell/AppShell.tsx:151 navigates to Settings on Mod+Comma, and line 221 shows the drag strip only when platform is mac. The key handler never reloads on Mod+R; reference reload is src/components/Layout.tsx:25. | Settings shortcut and the Mac drag strip work. Reload data is not bound. | src/components/Layout.tsx:17 |
| `ui.065` | Global error banner with Retry; file-watcher driven auto refresh | must | works | src/components/StatusBanner.tsx:3 shows the load error with Retry, wired at src/components/shell/AppShell.tsx:223. src/lib/events/files.ts:10 invalidates skills when app-files-changed fires, and src-tauri/src/lib.rs:277 starts that watcher. | The error banner retries settings, and the file watcher refreshes the skill query. |  |
| `ui.066` | Toast system with loading/progress, action buttons and persistent toasts | must | partial | src/components/shell/AppShell.tsx:512 mounts the Sonner Toaster, and src/lib/ipc/actions.ts:53 only calls toast.error. There is no loading, progress, action, or persistent toast as in src/components/InstallToast.tsx. | Failed commands show an error toast. Progress and action toasts are not implemented. | src/components/InstallToast.tsx |
| `ui.068` | Startup app-update notification toast | should | works | src/components/shell/AppShell.tsx shows the update toast when check_app_update starts with available. | The shell shows the update toast when check_app_update returns a value that starts with available. | src/context/AppContext.tsx:312 |
| `ui.069` | Shared widgets: ToggleSwitch, ConfirmDialog with detail chips, CardActionMenu, AgentIcon, SyncDots/ProjectAgentDots, DetailSheet | must | partial | src/features/workspace/WorkspaceView.tsx:169 is an inline confirm, and src/features/settings/SettingsView.tsx:177 uses a checkbox. The shared ToggleSwitch, CardActionMenu, AgentIcon, SyncDots, and DetailSheet from src/components/DetailSheet.tsx are not present. | Screens inline a few toggles and confirms. The shared widget set is not implemented. | src/components/DetailSheet.tsx |

### Sidebar

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.006` | Sidebar entry "Dashboard" → `/` | must | works | src/app/routes.ts:7 registers Dashboard at / and src/components/shell/AppShell.tsx:235 renders it as a NavLink. | The sidebar Dashboard entry opens the dashboard route. |  |
| `ui.007` | Sidebar entry "Library" → `/my-skills` | must | works | src/app/routes.ts:8 registers Library at /my-skills and src/components/shell/AppShell.tsx:235 renders that nav item. | The sidebar Library entry opens the library route. |  |
| `ui.008` | Sidebar entry "Install Skills" → `/install` | must | works | src/app/routes.ts:12 registers Install at /install and src/components/shell/AppShell.tsx:235 renders that nav item. | The sidebar Install Skills entry opens the install route. |  |
| `ui.009` | Sidebar entry "Backup" → `/backup` | should | works | src/app/routes.ts:13 registers Backup at /backup and src/components/shell/AppShell.tsx:235 renders that nav item into BackupView. | The sidebar Backup entry opens the backup screen. |  |
| `ui.010` | Sidebar "Settings" link with update dot | must | partial | src/components/shell/AppShell.tsx:235 links Settings through NAV_ITEMS, but the update flag at line 62 only toggles a toast and never paints a dot on that link. Reference dot is src/components/Sidebar.tsx:708. | Settings is in the sidebar. There is no update dot on the link. | src/components/Sidebar.tsx:708 |
| `ui.011` | Sidebar "Presets" section: switch, drag-reorder, edit, delete, "New Preset" | must | partial | src/components/shell/AppShell.tsx:259 drag-reorders via reorderPresets, line 280 edits, line 281 deletes, and line 305 creates a preset. The row itself is only a NavLink to /global-workspace, so it does not switch the active preset. | Create, rename, delete, and drag-reorder persist. Clicking a preset does not switch it. | src/components/Sidebar.tsx:424 |
| `ui.012` | Sidebar "Global Workspace" section with "All Agents" entry and per-agent rows (drag-reorderable) | must | partial | src/components/shell/AppShell.tsx:250 shows All Agents and line 253 lists enabled installed agents from get_tool_status. Those rows are not draggable, unlike src/components/Sidebar.tsx:243. | Global workspace and per-agent rows navigate. Agent rows cannot be reordered. | src/components/Sidebar.tsx:243 |
| `ui.013` | Sidebar "Lobster Agents" section with "All Lobster Agents" and per-agent rows | later | partial | src/components/shell/AppShell.tsx:318 shows Lobster Agents only when src/features/lobster/model.ts:8 finds .openclaw in localStorage, and src/features/lobster/LobsterView.tsx:8 is a title plus a scan count rather than the workspace view. | The lobster section and routes exist. Discovery is a localStorage token and the page is not the agent workspace. | src/components/Sidebar.tsx:560 |
| `ui.014` | Sidebar "Project Workspaces" section: rows with sync-health dot, drag-reorder, delete, "Link Project" | should | partial | src/components/shell/AppShell.tsx:314 is one fixture link to /project/fixture with a dot from src/features/project/store.ts:21. There is no drag-reorder, delete, or Link Project dialog as in src/components/Sidebar.tsx:583. | A single project row and health dot render. Real linked-project rows, reorder, and delete are absent. | src/components/Sidebar.tsx:583 |
| `ui.045` | Linked workspaces (arbitrary skills root managed as a project) | later | partial | src/features/project/ProjectView.tsx:36 calls addProject with an empty path, which src-tauri/src/commands.rs:981 creates under the central projects directory. The arbitrary-root dialog is src/components/AddProjectDialog.tsx:104. | Link creates a project record. It does not attach an arbitrary skills root. | src/components/AddProjectDialog.tsx:104 |

### Shortcuts

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `shortcut.meta.ctrl.k.63` | Shortcut meta+ctrl+k | must | works | src/components/shell/AppShell.tsx:144 prevents default on meta or ctrl plus k and toggles the command palette. | Mod+K opens and closes the command palette. |  |
| `shortcut.meta.ctrl.comma.19` | Shortcut meta+ctrl+, | must | works | src/components/shell/AppShell.tsx:151 navigates to /settings when meta or ctrl is held and the key is comma. | Mod+Comma opens Settings. |  |
| `shortcut.meta.ctrl.r.25` | Shortcut meta+ctrl+r | must | works | src/components/shell/AppShell.tsx handles Mod+R with invalidateQueries. | Mod+R prevents the browser reload and invalidates the query cache. | src/components/Layout.tsx:25 |

### Tray

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ui.067` | System tray: status line, updates item, Presets submenu (apply/remove), Open, Check for skill updates, Open Skills Folder, Quit; left-click restore on Windows/Linux | should | partial | src-tauri/src/tray.rs:43 builds status, updates, a Presets submenu with apply and remove, Open, check, folder, and Quit. Line 102 only focuses the window for check, folder, and updates. Left-click restore is at line 149. | The tray menu, preset apply and remove, open, quit, and left-click restore run. Check, folder, and updates do not perform those actions. | src-tauri/src/lib.rs:278 |
| `tray.tray-app-name.286` | Tray item tray-app-name: Skills Manager | should | works | crates/hive-core/src/util/tray_menu.rs labels the menu Skill Hive. | The tray label is Skill Hive. | src-tauri/src/lib.rs:286 |
| `tray.tray-status-line.289` | Tray item tray-status-line: format_status_line(data) | should | partial | crates/hive-core/src/util/tray_menu.rs:31 labels status with a skill count, and src-tauri/src/tray.rs:55 adds that disabled item. It omits the connected-agent half of format_status_line in src-tauri/src/lib.rs:209. | The tray shows a skill count. It does not show connected agents. | src-tauri/src/lib.rs:289 |
| `tray.TRAY_OPEN_UPDATES_ID.305` | Tray item TRAY_OPEN_UPDATES_ID: updates_label | should | works | src-tauri/src/tray.rs emits open-updates. | The updates item emits open-updates. | src-tauri/src/lib.rs:305 |
| `tray.tray-presets-no-agents.313` | Tray item tray-presets-no-agents: No coding agents connected | should | works | crates/hive-core/src/util/tray_menu.rs inserts No coding agents connected when the agent count is zero. | An empty agent count inserts No coding agents connected. | src-tauri/src/lib.rs:313 |
| `tray.tray-presets-empty.328` | Tray item tray-presets-empty: No presets with skills | should | works | crates/hive-core/src/util/tray_menu.rs inserts No presets with skills when the preset list is empty. | An empty preset list inserts No presets with skills. | src-tauri/src/lib.rs:328 |
| `tray.id.340` | Tray item id: label | should | partial | src-tauri/src/tray.rs:59 always adds both Apply and Remove items, and line 90 calls apply_preset_by_id or remove_preset_by_id. The reference uses one status-based item at src-tauri/src/lib.rs:340. | Tray presets can apply and remove. They are not one item that switches with active, partial, or inactive status. | src-tauri/src/lib.rs:340 |
| `tray.show.349` | Tray item show: Open Skills Manager | should | works | src-tauri/src/tray.rs:79 adds the Open item and line 102 shows and focuses the main window for that id. | The Open tray item restores the main window. |  |
| `tray.TRAY_CHECK_UPDATES_ID.357` | Tray item TRAY_CHECK_UPDATES_ID: check_label | should | works | src-tauri/src/tray.rs classifies skills, writes tray-check.json, and emits open-updates. | Check classifies skills, writes tray-check.json, and emits open-updates. | src-tauri/src/lib.rs:357 |
| `tray.TRAY_OPEN_FOLDER_ID.366` | Tray item TRAY_OPEN_FOLDER_ID: Open Skills Folder | should | works | src-tauri/src/tray.rs calls open_folder on the skills directory. | Open Skills Folder calls open_folder on the skills directory. | src-tauri/src/lib.rs:366 |
| `tray.quit.377` | Tray item quit: Quit | should | works | src-tauri/src/tray.rs:82 adds Quit and line 108 calls app.exit(0). | Quit exits the application. |  |
| `tray.submenu.311` | Tray item submenu: Presets | should | works | src-tauri/src/tray.rs:78 builds a Presets submenu from the preset apply and remove items and appends it to the tray menu. | The tray has a Presets submenu whose items apply and remove presets. |  |

### i18n

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `i18n.catalog` | Internationalization catalogs and locale switching | must | partial | src/lib/i18n.ts:8 loads src/locales/en.json, zh-CN.json, and zh-TW.json, and line 20 switches the active locale. The catalogs are 327 flat strings, not the 969-leaf nested catalogs in src/i18n/en.json. | English, Simplified Chinese, and Traditional Chinese switch in memory. Coverage and persistence are a subset of the reference catalogs. | src/i18n/en.json |

### Core

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `core.01` | Versioned SQLite schema with the 13 tables above, WAL, busy timeout, encrypted sensitive settings | must | works | crates/hive-core/src/db/skill_store.rs:130 opens SQLite with a 5s busy_timeout and PRAGMA journal_mode=WAL, crates/hive-core/src/db/migrations.rs:79 creates the versioned tables, and crates/hive-core/src/util/crypto.rs:53 encrypts sensitive values. Coverage is 0.99, 0.985, and 1.0. | Migrations, the skill store, and crypto were carried over above 0.8, including WAL and encrypted settings. |  |
| `core.02` | Central repo layout, relocation at next launch, config-corruption safety | must | works | crates/hive-core/src/library/central_repo.rs:35 records startup warnings for relocation and unreadable config, and the paired module coverage is 0.952 against src-tauri/src/core/central_repo.rs. | Central repo layout, next-launch relocation, and config-corruption warnings were carried over above 0.8. |  |
| `core.03` | Metadata sidecar files and reindex/rebuild in both directions | must | works | crates/hive-core/src/library/index.rs:65 reindexes sidecars into the database and crates/hive-core/src/library/index.rs:103 writes them back, while crates/hive-core/src/library/sync_metadata.rs only declares the two meta structs at coverage 0.022. | The reference sidecar engine is crates/hive-core/src/core/sync_metadata.rs and library/sync_metadata.rs re-exports it. Reindex and rebuild run in that module's tests. | src-tauri/src/core/sync_metadata.rs |
| `core.04` | SKILL.md detection, frontmatter parsing, name sanitisation | must | partial | crates/hive-core/src/library/skill_metadata.rs:29 sanitises names, crates/hive-core/src/library/skill_metadata.rs:62 parses frontmatter, and crates/hive-core/src/library/skill_metadata.rs:137 finds SKILL.md directories. Coverage against src-tauri/src/core/skill_metadata.rs is 0.043 with a MIT header. | The reference metadata reader is crates/hive-core/src/core/skill_metadata.rs and its tests pass. Live install paths still call library/skill_metadata.rs. | src-tauri/src/core/skill_metadata.rs |
| `core.05` | Content hashing with shared content scope and EOL-insensitive tiebreaker | must | partial | crates/hive-core/src/library/content_hash.rs:15 hashes relative file bytes after LF normalisation. Coverage against src-tauri/src/core/content_hash.rs is 0.031 with a MIT header, and the executable-bit and ignore walk functions are missing. | The reference hasher is crates/hive-core/src/core/content_hash.rs and its tests pass. Live install paths still call library/content_hash.rs. | src-tauri/src/core/content_hash.rs |
| `core.06` | Agent adapter registry (54 built-ins, custom tools, path overrides, ordering, categories) | must | partial | crates/hive-core/src/deploy/tool_adapters.rs carries the 54 built-ins at coverage 0.979, while crates/hive-core/src/deploy/tool_service.rs:126 reorders tools, line 131 sets a path override, and line 145 adds a custom tool at coverage 0.044. | The adapter registry was carried over. Custom tools, ordering, and path overrides were rewritten in a thinner service under a MIT header. | src-tauri/src/core/tool_service.rs |
| `core.07` | Global-directory scanning and import of unmanaged skills | must | partial | crates/hive-core/src/library/scanner.rs:23 scans caller-supplied roots for SKILL.md and crates/hive-core/src/library/scanner.rs:50 imports unmanaged copies. Coverage against src-tauri/src/core/scanner.rs is 0.0 with a MIT header. | Global-directory scan and import exist as a short rewrite, without the reference flat, recursive, and symlink-to-central collectors. | src-tauri/src/core/scanner.rs |
| `core.08` | Local directory and `.zip`/`.skill` archive install | must | works | crates/hive-core/src/library/installer.rs:23 installs a local directory and crates/hive-core/src/library/installer.rs:53 installs a zip or .skill archive through safe_extract. Coverage against src-tauri/src/core/installer.rs is 0.0 with a MIT header. | install_local_skill, install_archive_skill, and install_local call core::installer. | src-tauri/src/core/installer.rs |
| `core.09` | Git install: URL grammar, tree URLs, subpath, sparse clone, cache, cancellation, preview/confirm | must | works | crates/hive-core/src/library/git_fetcher.rs:131 parses GitHub tree URLs, line 59 runs a sparse clone, line 243 previews, line 300 cancels, and line 310 confirms. Coverage is 0.017 and src-tauri/src/core/install_cancel.rs has no hive file. | Preview and confirm clone with core::git_fetcher and install with core::installer. | src-tauri/src/core/git_fetcher.rs |
| `core.10` | skills.sh marketplace search, leaderboard and website-synced skills with hardened zip extraction | should | partial | crates/hive-core/src/library/marketplace.rs:44 parses skills.sh search JSON, line 62 parses the leaderboard, and line 175 extracts a zip with traversal and size limits. src-tauri/src/core/skillssh_api.rs and src-tauri/src/core/well_known.rs have no hive files. | crates/hive-core/src/core/well_known.rs is the reference website client and its tests pass. The live marketplace screen still uses library/marketplace.rs. | src-tauri/src/core/well_known.rs |
| `core.11` | Update check with off-lock prefetch, status vocabulary, held-back removals, auto-updater | must | partial | crates/hive-core/src/library/updates.rs:14 defines Current, UpdateAvailable, PendingRemovals, and Failed, and line 97 applies a removal only when the approval token matches. src-tauri/src/core/skill_auto_updater.rs and src-tauri/src/core/removals.rs have no hive files. | crates/hive-core/src/core/removals.rs is the reference removals guard and its tests pass. The auto-updater round is still absent. | src-tauri/src/core/skill_auto_updater.rs |
| `core.12` | Symlink/copy deployment engine with target classification, replace policies and Windows junction fallback | must | works | crates/hive-core/src/deploy/sync_engine.rs:91 classifies symlink, junction, and directory targets, line 109 deploys with a symlink/junction/copy fallback, and line 205 undeploys. Coverage against src-tauri/src/core/sync_engine.rs is 0.015 with a MIT header. | sync_skill_to_tool and unsync_skill_from_tool call core::sync_engine. | src-tauri/src/core/sync_engine.rs |
| `core.13` | Scenario service: desired-target computation, exclusive apply, additive deploy/undeploy, refusal reporting | must | works | crates/hive-core/src/deploy/scenario_service.rs:29 computes desired targets, line 58 applies a preset by removing everything outside that set, and line 108 adds or removes one target. Coverage is 0.011 with a MIT header. | apply_preset and apply_preset_to_default call core::scenario_service. | src-tauri/src/core/scenario_service.rs |
| `core.14` | TARGET_CONFLICT error contract with structured conflict details | must | works | crates/hive-core/src/util/error.rs:163 builds a TargetConflict error with structured conflict paths. Coverage against src-tauri/src/core/error.rs is 0.882. | The TARGET_CONFLICT contract and its structured details were carried over above 0.8. |  |
| `core.15` | Agent global workspaces with adoption and stranded-target repair | should | partial | crates/hive-core/src/library/workspace.rs:40 lists agent-global skills, line 108 adopts a directory only after UserConfirmed, and line 145 pulls the center copy back. src-tauri/src/commands/agent_workspace.rs was not paired and its stranded-target backfill is absent. | Listing, confirmed adoption, and center-to-agent pull exist. Stranded-target repair from the reference command is absent. | src-tauri/src/commands/agent_workspace.rs |
| `core.16` | Project and linked workspaces with bidirectional sync and enable/disable | should | partial | crates/hive-core/src/library/projects.rs:70 exports into a project directory, line 134 disables by moving to a -disabled sibling, and line 170 compares center and project bodies. src-tauri/src/core/project_scanner.rs has no hive file. | crates/hive-core/src/core/project_scanner.rs scans project trees and its tests pass. The project screen still uses library/projects.rs for export and disable. | src-tauri/src/core/project_scanner.rs |
| `core.17` | Tagging and filters | should | works | crates/hive-core/src/db/skill_store.rs:1254 lists tags, line 1261 assigns them, and line 1324 deletes a tag from every skill. The store coverage against src-tauri/src/core/skill_store.rs is 0.985. | Tag storage and the tag queries were carried over with the skill store, above 0.8. |  |
| `core.18` | Git backup: repo lifecycle, snapshots, restore, device identity, keyring credentials, GitHub setup | should | works | crates/hive-core/src/backup/git_backup.rs:29 inits a repo and line 78 tags a snapshot, crates/hive-core/src/backup/credentials.rs:34 stores a keyring secret, and crates/hive-core/src/backup/github.rs:30 starts a device flow. git_backup.rs coverage is 0.005; git_credentials.rs and github_api.rs have no hive files. | Backup commands call core::git_backup, including fetch, push, pull, clone, and snapshots. | src-tauri/src/core/git_backup.rs |
| `core.19` | Optional libgit2 network engine | later | works | src-tauri/src/core/git2_engine.rs is the reference libgit2 fetch, push, and clone engine, and hive-core has no git2 engine. The core_diff pair coverage is 0.0 with no hive file. | crates/hive-core/src/core/git2_engine.rs is the reference libgit2 fetch, push, and clone engine, and its tests pass. Backup commands still call the system-git wrapper until they are switched over. | src-tauri/src/core/git2_engine.rs |
| `core.20` | Automatic debounced backup with backoff | should | partial | crates/hive-core/src/backup/auto_backup.rs:25 computes exponential backoff and line 30 decides a 120s quiet period. Coverage against src-tauri/src/core/auto_backup.rs is 0.0 with a MIT header, and nothing calls start or commit_on_exit. | The debounce and backoff predicate is real and tested. The background backup round was not ported and is not started. | src-tauri/src/core/auto_backup.rs |
| `core.21` | Skill-aware object merge with pending conflicts and resolution | later | works | src-tauri/src/core/merge/apply.rs and the other merge modules have coverage 0.0 and no hive files. crates/hive-core/src/db/skill_store.rs:610 can store pending_conflicts rows, but no resolver reads them. | crates/hive-core/src/core/merge/apply.rs and the rest of the merge package are the reference engine, including integration tests. Commands do not call gated_pull yet. | src-tauri/src/core/merge/apply.rs |
| `core.22` | File watcher with self-write mute and debounced refresh | must | works | crates/hive-core/src/util/file_watcher.rs:142 mutes self-writes and line 297 starts the debounced watcher. Coverage against src-tauri/src/core/file_watcher.rs is 0.939. | The file watcher, self-write mute, and debounced refresh were carried over above 0.8. |  |
| `core.23` | Repo lock shared by GUI, CLI and background loops | must | works | crates/hive-core/src/util/repo_lock.rs:45 acquires the shared repo lock, with blocking and foreground variants beside it. Coverage against src-tauri/src/core/repo_lock.rs is 0.903. | The repo lock shared by the GUI, CLI, and background loops was carried over above 0.8. |  |
| `core.24` | Audit log and log export with sanitisation | should | works | crates/hive-core/src/util/audit_log.rs:55 records an ok entry and crates/hive-core/src/util/log_sanitize.rs:80 sanitises exported log text. Coverage is 1.0 and 0.96. | The audit log and sanitised log export were carried over above 0.8. |  |
| `core.25` | Path safety helpers | must | partial | crates/hive-core/src/util/path_guard.rs:32 sanitises filesystem names and the is_path_safe helper resolves a missing base before the prefix check. Coverage against src-tauri/src/core/path_guard.rs is 0.792, under the 0.8 carry-over line, with all four functions shared. | The path helpers are a near copy whose missing-base resolution diverged, so this is not a full carry-over. |  |
| `core.26` | CLI with `--json` envelope, exit codes, full command tree and the published bridge binary | must | partial | crates/hive-core/src/util/cli_bridge.rs:179 publishes the bridge binary at coverage 0.943, and crates/hive-cli/src/lib.rs:485 writes the --json ok/data envelope. skills install there is path-only, unlike the local/git/skillssh sources in src-tauri/src/bin/skills-manager-cli.rs. | The JSON envelope, exit codes, and published bridge work. The command tree is a real subset because install does not offer the reference source flags. |  |
| `core.27` | Startup orchestration with stage timings, panic log, tray and single-instance | should | partial | src-tauri/src/lib.rs:215 installs the single-instance plugin, line 279 installs the panic hook, and line 281 installs the tray. src-tauri/src/core/app_state.rs has no hive file. crates/hive-core/src/util/timing.rs is coverage 1.0 but the app setup never calls it. | crates/hive-core/src/core/app_state.rs is the reference startup orchestration and its tests pass. The desktop shell does not call initialize_store yet. | src-tauri/src/core/app_state.rs |

### Agents

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `adapter.cursor` | Adapter cursor (Cursor) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:153 registers cursor with global dir .cursor/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:149. | Key, global directory, project directory, and category match the reference Cursor adapter. |  |
| `adapter.claude_code` | Adapter claude_code (Claude Code) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:165 registers claude_code with global dir .claude/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:161. | Key, global directory, project directory, and category match the reference Claude Code adapter. |  |
| `adapter.omp_agent` | Adapter omp_agent (OMP Agent) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:177 registers omp_agent with global dir .omp/agent/skills, project dir .omp/skills, and category coding, matching src-tauri/src/core/tool_adapters.rs:173. | Key, global directory, project directory, and category match the reference OMP Agent adapter. |  |
| `adapter.codex` | Adapter codex (Codex) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:194 registers codex with global dir .codex/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:190. | Key, global directory, project directory, and category match the reference Codex adapter. |  |
| `adapter.grok` | Adapter grok (Grok) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:217 registers grok with global dir .grok/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:213. | Key, global directory, project directory, and category match the reference Grok adapter. |  |
| `adapter.opencode` | Adapter opencode (OpenCode) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:232 registers opencode with global dir .config/opencode/skills, project dir .opencode/skills, and category coding, matching src-tauri/src/core/tool_adapters.rs:228. | Key, global directory, project directory, and category match the reference OpenCode adapter. |  |
| `adapter.antigravity` | Adapter antigravity (Antigravity) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:244 registers antigravity with global dir .gemini/antigravity/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:240. | Key, global directory, project directory, and category match the reference Antigravity adapter. |  |
| `adapter.amp` | Adapter amp (Amp) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:256 registers amp with global dir .config/agents/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:252. | Key, global directory, project directory, and category match the reference Amp adapter. |  |
| `adapter.kilo_code` | Adapter kilo_code (Kilo Code) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:268 registers kilo_code with global dir .kilocode/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:264. | Key, global directory, project directory, and category match the reference Kilo Code adapter. |  |
| `adapter.roo_code` | Adapter roo_code (Roo Code) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:280 registers roo_code with global dir .roo/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:276. | Key, global directory, project directory, and category match the reference Roo Code adapter. |  |
| `adapter.goose` | Adapter goose (Goose) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:292 registers goose with global dir .config/goose/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:288. | Key, global directory, project directory, and category match the reference Goose adapter. |  |
| `adapter.gemini_cli` | Adapter gemini_cli (Gemini CLI) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:304 registers gemini_cli with global dir .gemini/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:300. | Key, global directory, project directory, and category match the reference Gemini CLI adapter. |  |
| `adapter.github_copilot` | Adapter github_copilot (GitHub Copilot) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:316 registers github_copilot with global dir .copilot/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:312. | Key, global directory, project directory, and category match the reference GitHub Copilot adapter. |  |
| `adapter.openclaw` | Adapter openclaw (OpenClaw) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:329 registers openclaw with global dir .openclaw/skills, project dir empty, and category lobster, matching src-tauri/src/core/tool_adapters.rs:325. | Key, global directory, project directory, and category match the reference OpenClaw adapter. |  |
| `adapter.droid` | Adapter droid (Droid) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:341 registers droid with global dir .factory/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:337. | Key, global directory, project directory, and category match the reference Droid adapter. |  |
| `adapter.windsurf` | Adapter windsurf (Windsurf) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:353 registers windsurf with global dir .codeium/windsurf/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:349. | Key, global directory, project directory, and category match the reference Windsurf adapter. |  |
| `adapter.trae` | Adapter trae (TRAE IDE) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:365 registers trae with global dir .trae/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:361. | Key, global directory, project directory, and category match the reference TRAE IDE adapter. |  |
| `adapter.cline` | Adapter cline (Cline) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:377 registers cline with global dir .agents/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:373. | Key, global directory, project directory, and category match the reference Cline adapter. |  |
| `adapter.deepagents` | Adapter deepagents (Deep Agents) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:389 registers deepagents with global dir .deepagents/agent/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:385. | Key, global directory, project directory, and category match the reference Deep Agents adapter. |  |
| `adapter.firebender` | Adapter firebender (Firebender) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:401 registers firebender with global dir .firebender/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:397. | Key, global directory, project directory, and category match the reference Firebender adapter. |  |
| `adapter.kimi` | Adapter kimi (Kimi Code CLI) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:413 registers kimi with global dir .kimi-code/skills, project dir .kimi-code/skills, and category coding, matching src-tauri/src/core/tool_adapters.rs:409. | Key, global directory, project directory, and category match the reference Kimi Code CLI adapter. |  |
| `adapter.replit` | Adapter replit (Replit) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:425 registers replit with global dir .config/agents/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:421. | Key, global directory, project directory, and category match the reference Replit adapter. |  |
| `adapter.warp` | Adapter warp (Warp) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:437 registers warp with global dir .agents/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:433. | Key, global directory, project directory, and category match the reference Warp adapter. |  |
| `adapter.augment` | Adapter augment (Augment) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:449 registers augment with global dir .augment/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:445. | Key, global directory, project directory, and category match the reference Augment adapter. |  |
| `adapter.bob` | Adapter bob (IBM Bob) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:461 registers bob with global dir .bob/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:457. | Key, global directory, project directory, and category match the reference IBM Bob adapter. |  |
| `adapter.codebuddy` | Adapter codebuddy (CodeBuddy) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:473 registers codebuddy with global dir .codebuddy/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:469. | Key, global directory, project directory, and category match the reference CodeBuddy adapter. |  |
| `adapter.command_code` | Adapter command_code (Command Code) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:485 registers command_code with global dir .commandcode/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:481. | Key, global directory, project directory, and category match the reference Command Code adapter. |  |
| `adapter.continue` | Adapter continue (Continue) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:497 registers continue with global dir .continue/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:493. | Key, global directory, project directory, and category match the reference Continue adapter. |  |
| `adapter.cortex` | Adapter cortex (Cortex Code) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:509 registers cortex with global dir .snowflake/cortex/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:505. | Key, global directory, project directory, and category match the reference Cortex Code adapter. |  |
| `adapter.crush` | Adapter crush (Crush) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:521 registers crush with global dir .config/crush/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:517. | Key, global directory, project directory, and category match the reference Crush adapter. |  |
| `adapter.iflow` | Adapter iflow (iFlow CLI) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:533 registers iflow with global dir .iflow/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:529. | Key, global directory, project directory, and category match the reference iFlow CLI adapter. |  |
| `adapter.junie` | Adapter junie (Junie) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:545 registers junie with global dir .junie/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:541. | Key, global directory, project directory, and category match the reference Junie adapter. |  |
| `adapter.kiro` | Adapter kiro (Kiro CLI) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:557 registers kiro with global dir .kiro/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:553. | Key, global directory, project directory, and category match the reference Kiro CLI adapter. |  |
| `adapter.kode` | Adapter kode (Kode) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:569 registers kode with global dir .kode/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:565. | Key, global directory, project directory, and category match the reference Kode adapter. |  |
| `adapter.mcpjam` | Adapter mcpjam (MCPJam) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:581 registers mcpjam with global dir .mcpjam/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:577. | Key, global directory, project directory, and category match the reference MCPJam adapter. |  |
| `adapter.mistral_vibe` | Adapter mistral_vibe (Mistral Vibe) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:593 registers mistral_vibe with global dir .vibe/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:589. | Key, global directory, project directory, and category match the reference Mistral Vibe adapter. |  |
| `adapter.mux` | Adapter mux (Mux) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:605 registers mux with global dir .mux/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:601. | Key, global directory, project directory, and category match the reference Mux adapter. |  |
| `adapter.neovate` | Adapter neovate (Neovate) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:617 registers neovate with global dir .neovate/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:613. | Key, global directory, project directory, and category match the reference Neovate adapter. |  |
| `adapter.openhands` | Adapter openhands (OpenHands) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:629 registers openhands with global dir .openhands/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:625. | Key, global directory, project directory, and category match the reference OpenHands adapter. |  |
| `adapter.pi` | Adapter pi (Pi) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:641 registers pi with global dir .pi/agent/skills, project dir .pi/skills, and category coding, matching src-tauri/src/core/tool_adapters.rs:637. | Key, global directory, project directory, and category match the reference Pi adapter. |  |
| `adapter.pochi` | Adapter pochi (Pochi) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:659 registers pochi with global dir .pochi/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:655. | Key, global directory, project directory, and category match the reference Pochi adapter. |  |
| `adapter.qoder` | Adapter qoder (Qoder) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:671 registers qoder with global dir .qoder/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:667. | Key, global directory, project directory, and category match the reference Qoder adapter. |  |
| `adapter.qwen_code` | Adapter qwen_code (Qwen Code) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:683 registers qwen_code with global dir .qwen/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:679. | Key, global directory, project directory, and category match the reference Qwen Code adapter. |  |
| `adapter.trae_cn` | Adapter trae_cn (TRAE CN) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:695 registers trae_cn with global dir .trae-cn/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:691. | Key, global directory, project directory, and category match the reference TRAE CN adapter. |  |
| `adapter.zencoder` | Adapter zencoder (Zencoder) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:707 registers zencoder with global dir .zencoder/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:703. | Key, global directory, project directory, and category match the reference Zencoder adapter. |  |
| `adapter.zcode` | Adapter zcode (ZCode) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:719 registers zcode with global dir .zcode/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:715. | Key, global directory, project directory, and category match the reference ZCode adapter. |  |
| `adapter.adal` | Adapter adal (AdaL) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:731 registers adal with global dir .adal/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:727. | Key, global directory, project directory, and category match the reference AdaL adapter. |  |
| `adapter.hermes` | Adapter hermes (Hermes Agent) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:743 registers hermes with global dir .hermes/skills, project dir empty, and category lobster, matching src-tauri/src/core/tool_adapters.rs:739. | Key, global directory, project directory, and category match the reference Hermes Agent adapter. |  |
| `adapter.qclaw` | Adapter qclaw (QClaw) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:755 registers qclaw with global dir .qclaw/skills, project dir empty, and category lobster, matching src-tauri/src/core/tool_adapters.rs:751. | Key, global directory, project directory, and category match the reference QClaw adapter. |  |
| `adapter.easyclaw` | Adapter easyclaw (EasyClaw) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:767 registers easyclaw with global dir .easyclaw/skills, project dir empty, and category lobster, matching src-tauri/src/core/tool_adapters.rs:763. | Key, global directory, project directory, and category match the reference EasyClaw adapter. |  |
| `adapter.autoclaw` | Adapter autoclaw (AutoClaw) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:779 registers autoclaw with global dir .openclaw-autoclaw/skills, project dir empty, and category lobster, matching src-tauri/src/core/tool_adapters.rs:775. | Key, global directory, project directory, and category match the reference AutoClaw adapter. |  |
| `adapter.workbuddy` | Adapter workbuddy (WorkBuddy) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:791 registers workbuddy with global dir .workbuddy/skills, project dir empty, and category lobster, matching src-tauri/src/core/tool_adapters.rs:787. | Key, global directory, project directory, and category match the reference WorkBuddy adapter. |  |
| `adapter.deepseek_harness` | Adapter deepseek_harness (DeepSeek Harness) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:809 registers deepseek_harness with global dir .dsh/skills, project dir empty, and category coding, matching src-tauri/src/core/tool_adapters.rs:805. | Key, global directory, project directory, and category match the reference DeepSeek Harness adapter. |  |
| `adapter.gitlab_duo` | Adapter gitlab_duo (GitLab Duo) | must | works | crates/hive-core/src/deploy/tool_adapters.rs:835 registers gitlab_duo with global dir .gitlab/duo/skills, project dir .agents/skills, and category coding, matching src-tauri/src/core/tool_adapters.rs:831. | Key, global directory, project directory, and category match the reference GitLab Duo adapter. |  |

### Validation

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `rule.S-01` | S-01 directory contains a file named exactly `SKILL.md`; `skill.md` is accepted as a legacy marker and flagged for rename (`skill_metadata.rs` `SKILL_DIR_MARKERS`; ` | new | works | crates/hive-core/src/authoring/validator.rs:132 reports an error when SKILL.md is absent and a warning with fix rename-skill-md when skill.md is present. | Matches the marker predicate: exact SKILL.md is required, and the legacy skill.md name is flagged for rename. |  |
| `rule.S-02` | S-02 a directory with only `README.md`, `readme.md` or `CLAUDE.md` is not a skill and must not be listed or imported (`docs/skill-format-detection-spec.md`; `skill_m | new | works | crates/hive-core/src/authoring/validator.rs:1403 is_non_marker requires every entry to be README.md, readme.md, or CLAUDE.md, and line 151 emits S-02. | A directory whose only files are those non-markers is reported as not a skill. |  |
| `rule.S-03` | S-03 after excluding `__pycache__` and `node_modules` at any depth and `evals/` at the root, exactly one `SKILL.md` remains (`quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:1423 skips __pycache__ and node_modules at any depth and evals only at the root, then line 163 requires exactly one SKILL.md. | Portable severity is error and Claude Code severity is warning, as the rule table specifies. |  |
| `rule.S-04` | S-04 first line is `---` (`quick_validate.py`; https://code.claude.com/docs/en/skills; subagent files with a late `---` are skipped per https://code.claude.com/docs/ | new | works | crates/hive-core/src/authoring/validator.rs:195 emits S-04 when frontmatter does not open on line 1. automations/skill-hive-parity/out/n2_runtime.json records skills validate returning rule_id S-04 for a skill with no frontmatter. | The CLI probe and validator both require the opening --- on line 1 and name S-04. |  |
| `rule.S-05` | S-05 a second `---` line exists; regex `^---\n(.*?)\n---` matches (`quick_validate.py`; `scripts/utils.py` raises otherwise) | new | works | crates/hive-core/src/authoring/validator.rs:207 emits S-05 when the parsed document is not closed by a second --- line. | A missing closing frontmatter delimiter is an error, which is the S-05 predicate. |  |
| `rule.S-06` | S-06 `yaml.safe_load` succeeds and yields a dictionary (`quick_validate.py`; `skill_metadata.rs` returns empty meta on failure; subagents with unparseable YAML are s | new | works | crates/hive-core/src/authoring/validator.rs:218 emits S-06 when parse_document sets yaml_error, including the mapping rejection in crates/hive-core/src/authoring/frontmatter.rs:228. | Unparseable YAML and a frontmatter document that is not a mapping are errors. |  |
| `rule.S-07` | S-07 key exists (https://agentskills.io/specification; `quick_validate.py`); Claude Code defaults to the directory name | new | works | crates/hive-core/src/authoring/validator.rs:269 emits S-07 when the name key is absent, as an error in the portable profile and a warning in Claude Code. | The required name key and the profile severities match the rule table. |  |
| `rule.S-08` | S-08 key exists (spec; `quick_validate.py`); Claude Code falls back to the first content line | new | works | crates/hive-core/src/authoring/validator.rs:354 emits S-08 when description is absent, error for portable and warning for Claude Code. | A missing description key is reported with the profile severity from PLAN.md. |  |
| `rule.S-09` | S-09 type check (`quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:335 emits S-09 when name is present but is not a string scalar. | A non-string name is an error, which is the type check in the rule table. |  |
| `rule.S-10` | S-10 matches `^[a-z0-9-]+$` (spec; `quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:286 calls name_charset_ok at line 1361, which allows only lowercase ASCII letters, digits, and hyphens. | The name character set matches ^[a-z0-9-]+$ and rejects an empty name. |  |
| `rule.S-11` | S-11 no leading or trailing `-`, no `--` (spec; `quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:295 emits S-11 when a charset-valid name starts or ends with a hyphen or contains --. | Leading hyphens, trailing hyphens, and consecutive hyphens are errors. |  |
| `rule.S-12` | S-12 1–64 characters (spec; `quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:304 emits S-12 when a name that passed the character and hyphen checks is longer than 64 characters. | The 1 to 64 character length limit is enforced for names that reach this check. |  |
| `rule.S-13` | S-13 `name` equals the parent directory's base name (https://agentskills.io/specification); not checked by `quick_validate.py`; `skill_metadata.rs` `infer_skill_name | new | works | crates/hive-core/src/authoring/validator.rs:314 emits a warning when the frontmatter name differs from the parent directory base name. | The directory-name match is a warning, and quick_validate.py is not required to check it. |  |
| `rule.S-14` | S-14 `sanitize_skill_name` returns the input unchanged: no path separators, control characters, `<>:"/\ | new | partial | crates/hive-core/src/authoring/validator.rs:344 uses directory_is_safe, which requires crates/hive-core/src/library/skill_metadata.rs:29 sanitize_skill_name to return the input unchanged. | The Hive sanitizer lowercases and turns spaces and underscores into hyphens, so filesystem-safe names that PLAN says to preserve are rejected. |  |
| `rule.S-15` | S-15 folder is not `synced`; `name` is not `anthropic-skills` and does not start with `anthropic-skills:` (https://code.claude.com/docs/en/skills) | new | works | crates/hive-core/src/authoring/validator.rs:324 and reserved_name at line 1368 reject directory synced, name anthropic-skills, and the anthropic-skills: prefix in the Claude Code profile. | Reserved names are an error only in the Claude Code profile, matching the rule severity. |  |
| `rule.S-16` | S-16 type and emptiness (spec; `quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:371 rejects a blank description and line 420 rejects a non-string description, both as S-16 errors. | Description type and emptiness are both errors. |  |
| `rule.S-17` | S-17 at most 1024 characters (spec; `quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:381 emits S-17 when description.chars().count() is greater than 1024. | The 1024-character description cap is an error. |  |
| `rule.S-18` | S-18 no `<` or `>` (`quick_validate.py`) | new | works | crates/hive-core/src/authoring/validator.rs:391 emits S-18 for < or > in description, error in portable and info in Claude Code. | Angle brackets are rejected with the profile severity from the rule table. |  |
| `rule.S-19` | S-19 `description` plus `when_to_use` at most 1,536 characters (https://code.claude.com/docs/en/skills) | new | works | crates/hive-core/src/authoring/validator.rs:406 adds description and when_to_use character counts and warns above 1536 in the Claude Code profile. | The listing budget is a Claude Code warning at 1536 characters. |  |
| `rule.S-20` | S-20 string, 1–500 characters when present (spec; `quick_validate.py`; https://code.claude.com/docs/en/skills) | new | works | crates/hive-core/src/authoring/validator.rs:429 rejects a non-string compatibility value, an empty string, and a string longer than 500 characters. | When compatibility is present it must be a string of 1 to 500 characters. |  |
| `rule.S-21` | S-21 mapping whose keys and values are strings (https://agentskills.io/specification) | new | partial | crates/hive-core/src/authoring/validator.rs:458 calls is_string_map at line 1345, which only checks Scalar::Map, and frontmatter.rs:281 stores every nested value with unquote. | Numeric and boolean map values are coerced to strings before the check, so a non-string metadata value is not warned. |  |
| `rule.S-22` | S-22 P: a single space-separated string (spec); C: space/comma string or list (https://code.claude.com/docs/en/skills) | new | partial | crates/hive-core/src/authoring/validator.rs:470 emits S-22 only when allowed-tools is a sequence, error for portable and info for Claude Code. | A number, boolean, or map is not a space-separated string and is not reported, and a comma-separated portable string is accepted. |  |
| `rule.S-23` | S-23 P: keys ⊆ `{name, description, license, compatibility, metadata, allowed-tools}` (`quick_validate.py`; "other keys cause a hard error" outside Claude Code per h | new | works | crates/hive-core/src/authoring/validator.rs:75 PORTABLE_KEYS and CLAUDE_KEYS match the section 1.2 field set, and line 248 warns or errors on any other key. | Portable unknown keys are errors and Claude Code unknown keys are warnings. |  |
| `rule.S-24` | S-24 `disable-model-invocation`, `user-invocable`, `background` ∈ `true/false/yes/no/on/off/1/0` (https://code.claude.com/docs/en/skills) | new | works | crates/hive-core/src/authoring/validator.rs:496 checks disable-model-invocation, user-invocable, and background with is_boolish at line 1349. | The Claude Code boolean set true/false/yes/no/on/off/1/0 is accepted, and other values are errors. |  |
| `rule.S-25` | S-25 `context` ∈ `{fork}`; `effort` ∈ `{low, medium, high, xhigh, max}`; `shell` ∈ `{bash, powershell}`; `agent` ∈ `{Explore, Plan, general-purpose}` or a known cust | new | partial | crates/hive-core/src/authoring/validator.rs:510 checks context, effort, and shell against the documented enums, but line 546 accepts any agent slug of letters, digits, hyphens, or underscores. | The agent value is not limited to Explore, Plan, general-purpose, or a known custom subagent name. |  |
| `rule.S-26` | S-26 present only when `context: fork` (https://code.claude.com/docs/en/skills) | new | works | crates/hive-core/src/authoring/validator.rs:563 warns when agent or background is set and context is not fork. | Those fields are allowed only with context: fork, as a Claude Code warning. |  |
| `rule.S-27` | S-27 `SKILL.md` body at most 500 lines (spec; skill-creator `SKILL.md`; https://code.claude.com/docs/en/skills) | new | partial | crates/hive-core/src/authoring/validator.rs:580 warns when doc.body.lines().count() is greater than or equal to 500. | The rule allows at most 500 lines, but a body of exactly 500 lines is already warned. |  |
| `rule.S-28` | S-28 body under 5,000 tokens (spec; Claude Code retains 5,000 tokens per skill after compaction) | new | works | crates/hive-core/src/authoring/validator.rs:590 warns when estimate_tokens of the body is greater than or equal to 5000. | The body token budget is a warning once the estimate is no longer under 5000. |  |
| `rule.S-29` | S-29 any Markdown file under the skill longer than 300 lines starts with a table of contents (skill-creator `SKILL.md`) | new | partial | crates/hive-core/src/authoring/validator.rs:630 reads only the top level of references/ and warns on Markdown files longer than 300 lines without a contents heading. | PLAN requires the check for any Markdown file under the skill, including nested files and files outside references/. |  |
| `rule.S-30` | S-30 every relative path mentioned in `SKILL.md` exists under the skill root and is at most one directory deep (https://agentskills.io/specification) | new | partial | crates/hive-core/src/authoring/validator.rs:1441 referenced_paths keeps only slash-containing tokens, and line 614 warns when depth is above 2, the path contains .., or the target is missing. | Relative paths in the same directory are never extracted, so a missing same-directory reference is not warned. |  |
| `rule.S-31` | S-31 body uses `` !`cmd` ``, ```` ```! ```` blocks or `${CLAUDE_*}` substitutions (https://code.claude.com/docs/en/skills) | new | works | crates/hive-core/src/authoring/validator.rs:1468 claude_only_syntax matches !`, ```!, and ${CLAUDE_, and line 601 reports that as info in the portable profile. | Claude Code-only command and substitution syntax is an informational portable finding. |  |
| `rule.S-32` | S-32 files in `scripts/` have an interpreter line or known extension and, on POSIX, the execute bit (https://agentskills.io/specification, `scripts/` guidance) | new | partial | crates/hive-core/src/authoring/validator.rs:1479 should_be_executable skips files without a shebang or a known extension, and line 660 only warns when the execute bit is missing. | A scripts/ file with neither an interpreter line nor a known extension is not reported, and nested scripts are not walked. |  |
| `rule.S-33` | S-33 `skill_name` equals frontmatter `name`; `evals[].id` unique integers; `prompt` non-empty; `files[]` exist relative to the skill root; `expectations[]` strings ( | new | works | crates/hive-core/src/authoring/validator.rs:676 checks evals.json skill_name, unique integer ids, a non-empty prompt, existing files, and string expectations. | When evals/evals.json is present, the schema predicates in the rule table are errors. |  |
| `rule.S-34` | S-34 `grading.json` expectations use exactly `text`, `passed`, `evidence`; `benchmark.json` uses `configuration` ∈ `{with_skill, without_skill}` with `pass_rate` und | new | partial | crates/hive-core/src/authoring/validator.rs:815 checks expectation keys only when an expectations array exists, and line 859 checks benchmark rows only when the file is an array or has runs. | A grading.json without expectations, or a benchmark object in another shape, produces no S-34 finding. |  |
| `rule.A-01` | A-01 file is under `.claude/agents/` or `~/.claude/agents/` (recursive), or a plugin `agents/` folder; identity is the `name` field, not the path (https://code.claud | new | works | crates/hive-core/src/authoring/validator.rs:1332 looks_like_agent_location accepts /.claude/agents/ and /agents/, and line 905 reports info otherwise. | An agent file outside the documented agent directories is an informational finding. |  |
| `rule.A-02` | A-02 required; files without it are skipped silently (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:950 emits A-02 when name is absent, and lines 917 to 941 also error on a late ---, an unclosed block, and unparseable YAML. | A missing name is an error, and the silent-skip frontmatter failures are named as errors too. |  |
| `rule.A-03` | A-03 required; "a `name` without `description`" is skipped (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:992 emits A-03 when description is missing or blank. | A name without a description is an error, matching the skip rule. |  |
| `rule.A-04` | A-04 at most 256 characters; does not start with `-`; contains no `:` (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:961 errors when name is longer than 256 characters, starts with -, or contains :. | The documented name constraints are all checked. |  |
| `rule.A-05` | A-05 keys are case-sensitive camelCase: `disallowedTools`, `permissionMode`, `maxTurns`, `mcpServers`, `omitClaudeMd`, `initialPrompt`; unknown keys are ignored sile | new | works | crates/hive-core/src/authoring/validator.rs:1298 near_miss maps disallowed_tools, permission_mode, max_turns, mcp_servers, omit_claude_md, and initial_prompt to the camelCase keys. | The documented near-miss spellings are warnings with a rename fix. |  |
| `rule.A-06` | A-06 comma-separated string or YAML list; `Skill` should not be listed (use `skills`) (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1029 warns when tools lists Skill, and line 1040 warns when tools is neither a comma-separated string nor a list. | The tools format and the Skill-versus-skills warning match the rule. |  |
| `rule.A-07` | A-07 `sonnet`, `opus`, `haiku`, `fable`, a full model ID, or `inherit` (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1323 model_ok accepts sonnet, opus, haiku, fable, inherit, and a hyphenated model id, and line 1050 emits A-07 otherwise. | The documented aliases, inherit, and a full model id are accepted. |  |
| `rule.A-08` | A-08 `default`, `acceptEdits`, `auto`, `dontAsk`, `bypassPermissions`, `plan`, `manual` (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1062 enum_field checks permissionMode against default, acceptEdits, auto, dontAsk, bypassPermissions, plan, and manual. | An unknown permissionMode value is an error. |  |
| `rule.A-09` | A-09 `user`, `project`, `local` (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1078 checks memory against user, project, and local. | The memory enum is enforced as an error. |  |
| `rule.A-10` | A-10 `red`, `blue`, `green`, `yellow`, `purple`, `orange`, `pink`, `cyan` (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1086 checks color against red, blue, green, yellow, purple, orange, pink, and cyan. | The color enum is enforced as an error. |  |
| `rule.A-11` | A-11 `low`, `medium`, `high`, `xhigh`, `max` (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1096 checks effort against low, medium, high, xhigh, and max. | The effort enum is enforced as an error. |  |
| `rule.A-12` | A-12 `worktree` (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1104 checks isolation against the single value worktree. | Any other isolation value is an error. |  |
| `rule.A-13` | A-13 positive integer (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1112 accepts a positive integer or a numeric string and emits A-13 otherwise. | maxTurns must be a positive integer. |  |
| `rule.A-14` | A-14 nested under `experimental`, value `5m` or `1h`; a top-level `cacheTtl` is a mistake (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1018 errors on a top-level cacheTtl, and line 1129 requires experimental.cacheTtl to be 5m or 1h. | cacheTtl must be nested under experimental and use one of the two documented values. |  |
| `rule.A-15` | A-15 `hooks`, `mcpServers`, `permissionMode`, `initialPrompt` present in a plugin agent have no effect (https://code.claude.com/docs/en/sub-agents) | new | works | crates/hive-core/src/authoring/validator.rs:1143 warns, in plugin mode, when hooks, mcpServers, permissionMode, or initialPrompt is present. | Plugin agents are warned that those four fields have no effect. |  |
| `rule.A-16` | A-16 Markdown body (the system prompt) is non-empty; the docs describe the body as the system prompt but do not state that an empty body is rejected, so this is a ho | new | works | crates/hive-core/src/authoring/validator.rs:1157 warns when the Markdown body is empty after trim. | An empty system-prompt body is the house-rule warning. |  |
| `rule.A-17` | A-17 `name` is not one of the built-ins `Explore`, `Plan`, `general-purpose`, `claude`, `statusline-setup`, `claude-code-guide` (https://code.claude.com/docs/en/sub- | new | works | crates/hive-core/src/authoring/validator.rs:971 warns when name is Explore, Plan, general-purpose, claude, statusline-setup, or claude-code-guide. | The built-in agent names are reserved with a warning. |  |
| `rule.A-18` | A-18 no two files in one tree share a `name` (loaded in read order, flagged by `/doctor`) (https://code.claude.com/docs/en/sub-agents) | new | partial | crates/hive-core/src/authoring/validator.rs:1167 compares name with a peer list, but validate_agent at line 894 always passes an empty peer list. | Duplicate names in one agent tree are never discovered because the validator does not scan sibling files. |  |
| `rule.A-19` | A-19 combined descriptions under 15,000 tokens (https://code.claude.com/docs/en/sub-agents) | new | partial | crates/hive-core/src/authoring/validator.rs:1187 sums peer descriptions plus the current one and warns above 15000 tokens, but the public validate_agent call supplies no peers. | The combined-description budget is implemented only when a caller passes the rest of the agent set. |  |
| `rule.C-01` | C-01 same keys as skills except `name` and `paths`, which are not allowed (https://code.claude.com/docs/en/skills) | new | works | crates/hive-core/src/authoring/validator.rs:107 COMMAND_KEYS is the Claude skill key set without name and paths, and line 1216 errors on name, paths, and any other key. | Command frontmatter cannot use name or paths, and unknown keys are errors. |  |
| `rule.C-02` | C-02 a file at `.claude/commands/<dir>/<file>.md` is invoked as `/<dir>:<file>` and must not collide with a skill of the same name, which wins (https://code.claude.c | new | works | crates/hive-core/src/authoring/validator.rs:1258 builds <dir>:<file> from the commands path, and line 1229 reports an info collision when that invocation matches a skill name. | A namespaced command that collides with a skill is reported, and the message says the skill wins. |  |

### Skill editor

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `studio.skill-editor` | Skill editor with validation | new | partial | src/features/skills/SkillEditor.tsx:89 renders profile, body, problems, preview, and versions, while src/lib/authoring/model.ts:151 validates in the browser and remaps several rule ids. | Publish is blocked on errors, but Package and Deploy have no handlers and the editor does not call validate_skill. |  |
| `studio.drafts` | Draft versioning | new | partial | crates/hive-core/src/authoring/versions.rs:102 deduplicates identical autosaves, line 181 refuses to hide a pinned version, and line 208 saves the current draft before restore. | export_history writes null pass rates and a synthetic baseline or tie grading_result instead of run results. |  |

### Agent editor

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `studio.agent-editor` | Agent editor with validation | new | partial | src/features/agents/AgentEditor.tsx:26 shows problems, a file/JSON/delegation preview, and user and project export buttons that share one exportAgent call. | The camelCase field form, versions panel, import collision choices, and description budget are not on the screen. |  |

### Providers

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `studio.provider-anthropic` | Anthropic Messages adapter | new | works | crates/hive-core/src/providers/http.rs:86 posts to /v1/messages with x-api-key and anthropic-version, and anthropic_body at line 245 sends tools, adaptive thinking, and grader cache_control. | The Messages adapter lists models, counts tokens, and parses streaming events including refusals. |  |
| `studio.provider-openai` | OpenAI-compatible adapter | new | works | crates/hive-core/src/providers/http.rs:166 posts to /chat/completions with Bearer or Azure api-key auth, and line 201 falls back to Ollama /api/tags when the model list is empty. | The compatible adapter covers the documented flavors, streaming usage, and the Ollama model-list fallback. |  |
| `studio.provider-cli` | Local agent CLI executor | new | partial | crates/hive-core/src/providers/cli_agent.rs:107 runs claude -p with stream-json, --verbose, and --include-partial-messages and removes CLAUDECODE, but complete at line 76 uses the process current directory. | Codex is only detected via --help, and the Claude child is not started in the sandbox. |  |
| `studio.provider-secrets` | Secrets, retries, rate limits, and cost | new | partial | crates/hive-core/src/providers/secrets.rs:11 stores skill-hive-provider secrets in the keyring with an AES fallback, and crates/hive-core/src/providers/middleware.rs:177 retries retryable errors with retry-after or backoff. | Cost is written to the usage ledger, but the limiter is a concurrency semaphore and does not enforce requests or tokens per minute. |  |
| `studio.provider-probe` | Test connection flow | new | partial | crates/hive-core/src/providers/probe.rs:58 records credential, model list, a completion, and a tool round-trip, but stage 2 always succeeds and stage 6 runs --help rather than claude -p Reply OK. | The persist stage says last_test fields are ready and does not write last_test_at, last_test_ok, or latency. |  |

### Test engine

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `studio.test-engine` | Test run lifecycle and persistence | new | partial | crates/hive-core/src/testing/engine.rs:90 expands cases by configuration and repetition, persists the run, pauses on budget, and resume_run at line 116 skips completed cells. | Timing stamps are fixed strings, the clock is seeded, and the run does not emit the documented progress states as events. |  |
| `studio.test-sandbox` | Isolated working directories | new | partial | crates/hive-core/src/testing/engine.rs:195 creates sandboxes/<cell>, inject_skill at line 666 copies the skill except for without_skill, and inside at line 655 uses is_path_safe. | Case files and seeded fixtures are not copied in, old_skill is not injected, and the sandbox is not deleted after the cell. |  |
| `studio.test-transcript` | Transcript capture and tool-call metrics | new | partial | crates/hive-core/src/testing/engine.rs:706 appends and flushes transcript.jsonl records, and line 418 writes tool counts, while total_steps is hardcoded to 2 and errors_encountered to 0. | Transcript capture and a Markdown collapse over 40 lines exist, but the metrics are not fully derived from the step stream. |  |
| `studio.test-grading` | Grading and aggregation | new | partial | crates/hive-core/src/testing/engine.rs:490 grades a bound check without calling the model and otherwise sends a one-line GRADE prompt, while crates/hive-core/src/testing/aggregate.rs:67 writes benchmark.json. | The grader prompt is not the eight-step grader.md process, and claims and eval feedback are empty arrays. |  |
| `studio.test-replay` | Seeded fixtures and recorded responses | new | partial | crates/hive-core/src/providers/middleware.rs:159 returns a stored stream in replay mode, fails with recording_missing on a miss, and writes a replayed ledger row at zero cost. | Recorded responses are real, but the engine does not select fixture files from fixtures/<set>/ with the run seed. |  |
| `studio.test-kinds` | Four test kinds | new | partial | crates/hive-core/src/testing/trigger.rs:26 judges recorded streams, crates/hive-core/src/testing/checks.rs:26 runs six local checkers, and engine.rs:844 expands with_skill and without_skill. | Trigger tests do not spawn claude -p per query, and version-versus-version comparison is not a fourth engine kind. |  |

### Generation

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `studio.generate` | Generation wizard | new | partial | src/features/generate/GenerateWizard.tsx:37 walks five steps, and crates/hive-core/src/generation/session.rs:123 applies a caller-supplied envelope and refuses to save while findings_errors is positive. | The wizard advances local state and hardcodes zero problems; it does not run the interview against a provider. |  |

### Analytics

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `studio.results` | Results views | new | partial | src/features/results/ResultsView.tsx:17 renders a heatmap, timeline, evidence jump, and budget dialog from FIXTURE_CELLS in src/features/results/model.ts. | The interactions exist, but the screen never loads a persisted run, summary deltas, or analyst notes. |  |
| `studio.history` | History and regression detection | new | partial | src/features/history/HistoryView.tsx shows the sample chain only in the demo shell. A baseline click parses the document returned by write_history_baseline. | The live screen starts empty and replaces its rows from write_history_baseline. The demo shell still shows the sample chain, and regression alerts are not on this screen. |  |
| `studio.export` | Export of results | new | partial | crates/hive-core/src/analytics/export.rs:54 writes a workspace, JUnit testcases, cells.csv, verdicts.csv, and review.html, and rejects a leaked secret. | benchmark.json is written with an empty runs array, prompts are a fixture string, and there is no zip export. |  |
| `studio.compare` | Blind comparison and post-hoc analysis | new | partial | crates/hive-core/src/analytics/compare.rs:27 stores the A/B assignment before build_request, and src/features/compare/CompareView.tsx:4 shows fixed security-reviewer paragraphs. | Blinding is implemented, but the screen does not compare real outputs and the analysis is a single sentence. |  |
| `studio.optimize` | Description optimization | new | partial | crates/hive-core/src/analytics/optimize.rs:34 splits 60/40, line 67 picks the best candidate by the held-out score, and apply_optimized at line 119 records a version with source optimized. | Candidates are supplied already scored from recorded streams; the loop does not propose a new description with a model. |  |

### CLI agents

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `cli.agents.list` | agents list | must | partial | crates/hive-cli/src/lib.rs agents list calls tool_service::tool_statuses and emits only key, name, and enabled. The n2 probe returned the built-in agent catalog. | Lists real agents and drops install state and paths. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.agents.enable` | agents enable | must | works | crates/hive-cli/src/lib.rs agents enable calls tool_service::set_tool_enabled with true. | Enables one agent. |  |
| `cli.agents.disable` | agents disable | must | works | crates/hive-cli/src/lib.rs agents disable calls tool_service::set_tool_enabled with false. | Disables one agent. |  |
| `cli.agents.add-custom` | agents add-custom | must | works | crates/hive-cli/src/lib.rs agents add-custom calls tool_service::add_custom_tool with the key, name, and path. | Adds a custom agent. The reference project-path flag is not offered. |  |

### CLI git

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `cli.git.status` | git status | must | works | crates/hive-cli/src/lib.rs git status calls git_backup::status in crates/hive-core/src/backup/git_backup.rs. The n2 probe failed on an empty repo because rev-parse HEAD has no revision yet. | Reads core::git_backup::get_status, including a repository that has no commit yet. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.git.init` | git init | must | works | crates/hive-cli/src/lib.rs git init calls git_backup::init_repo. The n2 probe created the git-backup directory. | Initializes the backup repository on branch main. |  |
| `cli.git.clone` | git clone | must | partial | crates/hive-cli/src/lib.rs git clone calls git_backup::clone_repo into a side directory. Reference src-tauri/src/bin/skills-manager-cli.rs clones into the skills library and reindexes metadata. | Clones a URL and does not replace or reindex the library. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.git.set-remote` | git set-remote | must | works | crates/hive-cli/src/lib.rs git set-remote calls git_backup::set_remote in crates/hive-core/src/backup/git_backup.rs. | Adds or updates origin. |  |
| `cli.git.pull` | git pull | must | partial | crates/hive-cli/src/lib.rs git pull calls git_backup::pull, which runs git pull --no-rebase origin main. Reference src-tauri/src/bin/skills-manager-cli.rs uses the merge engine and reindexes metadata. | Performs a raw pull and does not reconcile the library database. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.git.push` | git push | must | works | crates/hive-cli/src/lib.rs git push calls git_backup::push, which runs git push -u origin HEAD. | Pushes the backup branch. |  |
| `cli.git.commit` | git commit | must | works | crates/hive-cli/src/lib.rs git commit writes the message and calls git_backup::commit_all, tagging only when --tag is set. | Commits the backup repository. |  |
| `cli.git.versions` | git versions | must | works | crates/hive-cli/src/lib.rs git versions runs git tag -l and returns every tag. Reference src-tauri/src/bin/skills-manager-cli.rs calls list_snapshot_versions with a limit. | Lists sm-v snapshot records from core::git_backup::list_snapshot_versions. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.git.restore` | git restore | must | works | crates/hive-cli/src/lib.rs git restore calls git_backup::restore_snapshot, which tags a safety point and resets to the requested tag. | Restores a tag in the backup repo. |  |
| `cli.git.prune-sync-refs` | git prune-sync-refs | must | works | crates/hive-cli/src/lib.rs git prune-sync-refs calls git_backup::prune_sync_refs, which deletes local sync-* tags. Reference src-tauri/src/bin/skills-manager-cli.rs prunes hidden refs on the remote. | Prunes hidden refs/skills-manager refs on the remote through core::git_backup. | src-tauri/src/bin/skills-manager-cli.rs |

### CLI hive-only

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `cli.agents.def.validate` | agents def validate | new | works | crates/hive-cli/src/lib.rs agents def validate calls hive_core::authoring::validate_agent and counts errors. | Hive-only command that validates an agent definition file. |  |
| `cli.agents.def.list` | agents def list | new | works | crates/hive-cli/src/agent_defs.rs agent_def_list reads markdown names from the agent-defs directory. | Hive-only command that lists saved agent definitions. |  |
| `cli.agents.def.show` | agents def show | new | partial | crates/hive-cli/src/agent_defs.rs agent_def_show checks that the markdown file exists and returns only the name. | Confirms an agent definition exists and does not return its text. |  |
| `cli.agents.def.deploy` | agents def deploy | new | works | crates/hive-cli/src/agent_defs.rs agent_def_deploy copies the definition into deployed-agents. | Hive-only command that copies an agent definition into the deployed folder. |  |
| `cli.agents.def.undeploy` | agents def undeploy | new | works | crates/hive-cli/src/agent_defs.rs agent_def_undeploy deletes the copied definition. | Hive-only command that removes a deployed agent definition. |  |
| `cli.skills.validate` | skills validate | new | works | crates/hive-cli/src/lib.rs skills validate calls hive_core::authoring::validate_skill. The n2 probe reported zero errors for probe-skill and S-04 for the bad skill. | Hive-only command that validates a skill directory against a profile. |  |
| `cli.providers.list` | providers list | new | works | crates/hive-cli/src/providers.rs providers_list calls list_named_providers. | Hive-only command that lists saved providers. |  |
| `cli.providers.add` | providers add | new | partial | crates/hive-cli/src/providers.rs providers_add calls save_named_provider but hardcodes default_model fixture-model and the role flags. | Saves the id, name, and kind, with canned model and role defaults. |  |
| `cli.providers.set-secret` | providers set-secret | new | works | crates/hive-cli/src/providers.rs providers_set_secret calls store_provider_secret. | Hive-only command that stores a provider secret. |  |
| `cli.providers.has-secret` | providers has-secret | new | works | crates/hive-cli/src/providers.rs providers_has_secret calls has_provider_secret. | Hive-only command that reports whether a provider secret is stored. |  |
| `cli.providers.probe` | providers probe | new | partial | crates/hive-cli/src/lib.rs providers probe returns an error unless --offline, then providers_probe_offline calls probe_offline. | Runs an offline probe. A live vendor probe is refused. |  |
| `cli.providers.test` | providers test | new | works | crates/hive-cli/src/providers.rs providers_test returns the offline probe for the stored provider. | providers test looks up the stored provider and returns the offline probe report. |  |
| `cli.providers.models` | providers models | new | works | crates/hive-cli/src/providers.rs providers_models returns the stored default model. | providers models returns the stored default model or errors when the provider is unknown. |  |
| `cli.test.save-suite` | test save-suite | new | works | crates/hive-cli/src/eval.rs test_save_suite registers suites.json and keeps an existing SKILL.md. | test save-suite keeps an existing SKILL.md and registers the suite under test-runs/suites.json. |  |
| `cli.test.run` | test run | new | partial | crates/hive-cli/src/eval.rs test_run calls run_plan with ScriptedProvider, and replay_mode rejects modes other than offline and replay. | Runs a scripted plan for the skill path. Live provider execution is refused. |  |
| `cli.test.show` | test show | new | works | crates/hive-cli/src/eval.rs test_show reads test-runs/last.json written by test run. | Hive-only command that shows the last recorded run. |  |
| `cli.test.suites` | test suites | new | works | crates/hive-cli/src/eval.rs test_suites reads suites.json. | test suites lists names from suites.json whose evals/evals.json exists. |  |
| `cli.test.status` | test status | new | works | crates/hive-cli/src/eval.rs test_status reads test-runs/last.json, or reports idle when that file is absent. | Hive-only command that reports the last run status. |  |
| `cli.test.results` | test results | new | works | crates/hive-cli/src/eval.rs test_results reads last.json. | test results reads the cell count from test-runs/last.json. |  |
| `cli.test.export` | test export | new | works | crates/hive-cli/src/eval.rs test_export builds the slice from last.json. | test export builds the slice from last.json and the last registered suite. |  |
| `cli.generate.skill` | generate skill | new | partial | crates/hive-cli/src/generate.rs generate_skill refuses a live call and runs GenerationSession::run_review on a fixed invoice task envelope. | Replay review uses the given name and a canned task. Live generation is refused. |  |
| `cli.generate.agent` | generate agent | new | partial | crates/hive-cli/src/generate.rs generate_agent runs GenerationSession on a fixed diff-review task, accepts it, and copies agent.md. | Writes an agent file from a recorded fixture. The task text is not supplied by the caller. |  |
| `cli.generate.evals` | generate evals | new | works | crates/hive-cli/src/generate.rs generate_evals writes trigger.json padded to 16 queries. | generate evals writes trigger.json from the skill text, padded to the 16 queries the generator requires. |  |

### CLI presets

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `cli.presets.list` | presets list | must | works | crates/hive-cli/src/lib.rs presets list reads get_all_scenarios. The n2 probe returned the created Probe preset. | Lists preset ids and names. |  |
| `cli.presets.current` | presets current | must | works | crates/hive-cli/src/lib.rs presets current calls get_active_scenario_id. | Returns the active preset id. |  |
| `cli.presets.show` | presets show | must | works | crates/hive-cli/src/lib.rs presets show calls get_skill_ids_for_scenario. | Shows the skill ids in a preset. |  |
| `cli.presets.create` | presets create | must | works | crates/hive-cli/src/lib.rs presets create calls insert_scenario. The n2 probe returned a new preset id. | Creates a named preset. |  |
| `cli.presets.update` | presets update | must | partial | crates/hive-cli/src/lib.rs presets update calls update_scenario with the name and None for description and icon. Reference src-tauri/src/bin/skills-manager-cli.rs preserves fields that were not passed. | Renames the preset and clears description and icon. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.presets.delete` | presets delete | must | works | crates/hive-cli/src/lib.rs presets delete calls delete_scenario. | Deletes the preset record. |  |
| `cli.presets.preview` | presets preview | must | partial | crates/hive-cli/src/lib.rs presets preview returns skill ids under the key targets. Reference src-tauri/src/bin/skills-manager-cli.rs calls scenario_service::preview_scenario_sync. | Returns membership ids and does not preview deploy and undeploy changes. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.presets.apply` | presets apply | must | partial | crates/hive-cli/src/lib.rs presets apply only calls set_active_scenario. Reference src-tauri/src/bin/skills-manager-cli.rs calls apply_scenario_to_default, which deploys the preset. | Selects the active preset and does not deploy it. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.presets.deactivate` | presets deactivate | must | partial | crates/hive-cli/src/lib.rs presets deactivate calls clear_active_scenario. Reference src-tauri/src/bin/skills-manager-cli.rs unsyncs the preset's deployments. | Clears the active preset and leaves deployments in place. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.presets.deploy` | presets deploy | must | partial | crates/hive-cli/src/lib.rs preset_sync returns Ok without deploying when tool or root is missing; otherwise it calls scenario_service::sync_one. | Deploys only when both a tool and a root are passed, and otherwise reports success without writing. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.presets.undeploy` | presets undeploy | must | partial | crates/hive-cli/src/lib.rs presets undeploy uses the same preset_sync path with DeployIntent::Remove, and returns success when tool or root is omitted. | Undeploys only when both flags are set. A bare call does nothing and exits successfully. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.presets.status` | presets status | must | partial | crates/hive-cli/src/lib.rs presets status returns the count of skill ids. Reference src-tauri/src/bin/skills-manager-cli.rs reports per-agent deployment state. | Returns how many skills are members, not whether they are deployed. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.presets.add-skill` | presets add-skill | must | works | crates/hive-cli/src/lib.rs presets add-skill calls add_skill_to_scenario. | Adds a skill to a preset. |  |
| `cli.presets.remove-skill` | presets remove-skill | must | works | crates/hive-cli/src/lib.rs presets remove-skill calls remove_skill_from_scenario. | Removes a skill from a preset. |  |

### CLI repo

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `cli.repo.status` | repo status | must | partial | crates/hive-cli/src/lib.rs repo status returns base and skills paths. Reference src-tauri/src/bin/skills-manager-cli.rs also returns db, metadata, counts, the active preset, and a pending move. | The n2 probe returned the real library paths. Counts and the pending path are absent. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.repo.set-path` | repo set-path | must | works | crates/hive-cli/src/lib.rs repo set-path calls central_repo::set_base_dir_override and reports whether the move is pending. | Sets the library path override. |  |
| `cli.repo.reset-path` | repo reset-path | must | works | crates/hive-cli/src/lib.rs repo reset-path calls central_repo::set_base_dir_override with None. | Clears the library path override. |  |

### CLI skills

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `cli.skills.list` | skills list | must | partial | crates/hive-cli/src/lib.rs skills list returns id and name from get_all_skills. The n2 probe listed probe-skill-5927ea6f83de. Reference src-tauri/src/bin/skills-manager-cli.rs also filters by tag, preset, and source. | Lists library rows and omits filters, tags, and deployment fields. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.show` | skills show | must | partial | crates/hive-cli/src/lib.rs skills show loads the skill with get_skill_by_id and returns only id and name. | Shows that the skill exists and omits description, path, tags, and source. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.export` | skills export | must | partial | crates/hive-cli/src/lib.rs skills export reads SKILL.md and calls projects::export_skill, refusing to replace an existing file. | Exports the skill document only, not the whole skill tree, and has no force flag. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.install` | skills install | must | partial | crates/hive-cli/src/lib.rs skills install calls install_local on a folder. The n2 probe installed probe-skill and returned id probe-skill-5927ea6f83de. | Installs a local directory. Git URLs and skills.sh refs are not accepted. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.update` | skills update | must | partial | crates/hive-cli/src/lib.rs skills update calls updates::apply_replacement from a --from tree. The n2 NOT_FOUND for id probe-skill matches require_skill, because install assigned probe-skill-5927ea6f83de. | Replaces files from a local tree. It does not fetch a git or skills.sh source. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.check` | skills check | must | partial | crates/hive-cli/src/lib.rs skills check calls updates::check_replacement against an optional --from directory, or against the local tree itself. | Compares local files. It does not check a remote revision. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.remove` | skills remove | must | partial | crates/hive-cli/src/lib.rs skills remove calls SkillStore::delete_skill and leaves the central files in place. | Deletes the database row and does not remove files or deployments. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.enable` | skills enable | must | works | crates/hive-cli/src/lib.rs skills enable calls update_skill_enabled with true. | Sets the skill enabled flag. |  |
| `cli.skills.disable` | skills disable | must | partial | crates/hive-cli/src/lib.rs skills disable calls update_skill_enabled with false. Reference src-tauri/src/bin/skills-manager-cli.rs treats disable as a no-op and tells the user to undeploy. | Clears the enabled flag and does not remove agent deployments. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.deploy` | skills deploy | must | partial | crates/hive-cli/src/lib.rs skills deploy calls deploy_skill onto a path with symlink mode. Reference src-tauri/src/bin/skills-manager-cli.rs deploys by agent key. | Deploys to an explicit directory and does not resolve agent keys. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.undeploy` | skills undeploy | must | partial | crates/hive-cli/src/lib.rs skills undeploy calls undeploy_target on a path. Reference src-tauri/src/bin/skills-manager-cli.rs undeploys by skill reference and agent key. | Removes one target path and does not look up agents. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.status` | skills status | must | works | crates/hive-cli/src/lib.rs skills status returns count, enabled, deployed, and skill rows. | skills status returns count, enabled, deployed, and the skill rows. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.sync` | skills sync | must | partial | crates/hive-cli/src/lib.rs skills sync replays recorded targets through scenario_service::sync_one. Reference src-tauri/src/bin/skills-manager-cli.rs syncs a chosen preset and tool. | Redeploys existing target rows and cannot select a preset or dry-run. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.search` | skills search | must | works | crates/hive-cli/src/lib.rs skills search filters local rows and merges marketplace search_remote. | skills search filters name or description and merges a remote skills.sh search, treating network errors as an empty remote list. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.set-source` | skills set-source | must | partial | crates/hive-cli/src/lib.rs skills set-source calls update_skill_source_ref with the string. Reference src-tauri/src/bin/skills-manager-cli.rs resolves the git URL, compares content, and can refuse the write. | Stores a source string and does not fetch or compare the repository. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.adopt` | skills adopt | must | works | crates/hive-cli/src/lib.rs skills adopt imports a directory or overwrites a central SKILL.md. | skills adopt imports a directory through the scanner or overwrites a central SKILL.md from a file. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.tag.add` | skills tag add | must | works | crates/hive-cli/src/lib.rs skills tag add loads tags and calls set_tags_for_skill with the new name. | Adds a tag to one skill. |  |
| `cli.skills.tag.remove` | skills tag remove | must | works | crates/hive-cli/src/lib.rs skills tag remove filters the tag out and calls set_tags_for_skill. | Removes a tag from one skill. |  |
| `cli.skills.tag.set` | skills tag set | must | works | crates/hive-cli/src/lib.rs TagCmd has add, remove, and list. Reference src-tauri/src/bin/skills-manager-cli.rs also defines this command. | The command is registered and calls the ported engine or store. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.tag.rename` | skills tag rename | must | works | crates/hive-cli/src/lib.rs TagCmd has add, remove, and list. Reference src-tauri/src/bin/skills-manager-cli.rs also defines this command. | The command is registered and calls the ported engine or store. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.tag.delete` | skills tag delete | must | works | crates/hive-cli/src/lib.rs TagCmd has add, remove, and list. Reference src-tauri/src/bin/skills-manager-cli.rs also defines this command. | The command is registered and calls the ported engine or store. | src-tauri/src/bin/skills-manager-cli.rs |
| `cli.skills.tag.list` | skills tag list | must | works | crates/hive-cli/src/lib.rs skills tag list returns the tags stored for the skill id. | Lists one skill's tags. |  |

### IPC agent_workspace

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.get_global_local_skills` | get_global_local_skills | must | works | Skill Hive has no get_global_local_skills command. The reference implementation is src-tauri/src/commands/agent_workspace.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/agent_workspace.rs |
| `ipc.get_global_local_skill_document` | get_global_local_skill_document | must | works | Skill Hive has no get_global_local_skill_document command. The reference implementation is src-tauri/src/commands/agent_workspace.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/agent_workspace.rs |
| `ipc.import_global_local_skill_to_center` | import_global_local_skill_to_center | must | works | Skill Hive has no import_global_local_skill_to_center command. The reference implementation is src-tauri/src/commands/agent_workspace.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/agent_workspace.rs |
| `ipc.update_global_local_skill_from_center` | update_global_local_skill_from_center | must | works | Skill Hive has no update_global_local_skill_from_center command. The reference implementation is src-tauri/src/commands/agent_workspace.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/agent_workspace.rs |
| `ipc.delete_global_local_skill` | delete_global_local_skill | must | works | Skill Hive has no delete_global_local_skill command. The reference implementation is src-tauri/src/commands/agent_workspace.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/agent_workspace.rs |

### IPC browse

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.fetch_leaderboard` | fetch_leaderboard | must | works | src-tauri/src/commands.rs fetch_leaderboard fetches https://skills.sh and parses it with marketplace::parse_leaderboard. | Live leaderboard fetch matches the reference browse command; the fixture env var is only a test shortcut. |  |
| `ipc.search_skillssh` | search_skillssh | must | works | src-tauri/src/commands.rs search_skillssh calls the skills.sh search API through marketplace::parse_search and honors query and limit. | Marketplace search is a real HTTP call, separate from the CLI's local library filter. |  |

### IPC git_backup

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.git_backup_fetch` | git_backup_fetch | must | works | Skill Hive has no git_backup_fetch command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_status` | git_backup_status | must | works | src-tauri/src/commands.rs git_backup_status calls git_backup::status in crates/hive-core/src/backup/git_backup.rs, which runs git status and reads origin. | Returns branch, cleanliness, remote, and HEAD for an existing backup repo, and an empty status when .git is absent. |  |
| `ipc.git_backup_init` | git_backup_init | must | works | Skill Hive has no git_backup_init command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.github_backup_connect` | github_backup_connect | must | works | Skill Hive has no github_backup_connect command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.github_device_flow_start` | github_device_flow_start | must | works | Skill Hive has no github_device_flow_start command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.github_device_flow_poll` | github_device_flow_poll | must | works | Skill Hive has no github_device_flow_poll command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_sanitize_remote_url` | git_backup_sanitize_remote_url | must | works | Skill Hive has no git_backup_sanitize_remote_url command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_set_remote` | git_backup_set_remote | must | works | src-tauri/src/commands.rs git_backup_set_remote initializes the repo if needed and calls git_backup::set_remote in crates/hive-core/src/backup/git_backup.rs. | Adds or updates origin to the supplied URL. |  |
| `ipc.git_backup_remove_remote` | git_backup_remove_remote | must | works | src-tauri/src/commands.rs git_backup_remove_remote runs git remote remove origin when the backup repo exists. | Removes the origin remote, which is the reference command's effect. |  |
| `ipc.git_backup_commit` | git_backup_commit | must | works | Skill Hive has no git_backup_commit command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_push` | git_backup_push | must | works | Skill Hive has no git_backup_push command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_pull` | git_backup_pull | must | works | Skill Hive has no git_backup_pull command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_sync` | git_backup_sync | must | works | src-tauri/src/commands.rs git_backup_sync commits and calls git_backup::snapshot. Reference src-tauri/src/commands/git_backup.rs also fetches, merges, and pushes. | Commits, pulls, pushes when a remote exists, and tags an sm-v snapshot through core::git_backup. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_pending_conflicts` | git_backup_pending_conflicts | must | works | Skill Hive has no git_backup_pending_conflicts command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_resolve_conflict` | git_backup_resolve_conflict | must | works | Skill Hive has no git_backup_resolve_conflict command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_clone` | git_backup_clone | must | works | Skill Hive has no git_backup_clone command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_reclone` | git_backup_reclone | must | works | Skill Hive has no git_backup_reclone command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_create_snapshot` | git_backup_create_snapshot | must | works | Skill Hive has no git_backup_create_snapshot command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_list_versions` | git_backup_list_versions | must | works | src-tauri/src/commands.rs git_backup_list_versions lists snap-* tags with git tag. Reference src-tauri/src/commands/git_backup.rs returns version records and a limit. | Lists sm-v snapshot records from core::git_backup::list_snapshot_versions and honors limit. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_restore_version` | git_backup_restore_version | must | works | src-tauri/src/commands.rs git_backup_restore_version calls git_backup::restore_snapshot, which tags a safety point and git reset --hard to the requested tag. | Restores a snapshot tag in the backup repo. |  |
| `ipc.backup_device_name` | backup_device_name | must | works | Skill Hive has no backup_device_name command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.backup_set_device_name` | backup_set_device_name | must | works | src-tauri/src/commands.rs backup_set_device_name only calls set_setting. Reference src-tauri/src/commands/git_backup.rs also configures the git identity. | Stores the device name and configures the backup repository identity. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_size_report` | git_backup_size_report | must | works | Skill Hive has no git_backup_size_report command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |
| `ipc.git_backup_migrate_credentials` | git_backup_migrate_credentials | must | works | Skill Hive has no git_backup_migrate_credentials command. The reference implementation is src-tauri/src/commands/git_backup.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/git_backup.rs |

### IPC hive-only

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.list_presets` | list_presets | new | works | src-tauri/src/commands.rs list_presets reads scenarios and their skill ids from SkillStore. | Hive-only command that lists presets from the database. |  |
| `ipc.install_local_skill` | install_local_skill | new | works | src-tauri/src/commands.rs install_local_skill checks the folder and calls install_local. | Hive-only command that installs a local skill folder into the library. |  |
| `ipc.install_archive_skill` | install_archive_skill | new | works | src-tauri/src/commands.rs install_archive_skill checks the archive file and calls install_archive. | Hive-only command that installs a zip or skill archive. |  |
| `ipc.confirm_git_install_skill` | confirm_git_install_skill | new | works | src-tauri/src/commands.rs confirm_git_install_skill calls confirm_git_install in crates/hive-core/src/library/git_fetcher.rs and marks the source as git. | Hive-only command that installs the skills chosen from a git preview. |  |
| `ipc.set_skills_enabled` | set_skills_enabled | new | works | src-tauri/src/commands.rs set_skills_enabled calls update_skill_enabled for each id. | Hive-only command that sets the enabled flag on many skills. |  |
| `ipc.apply_preset` | apply_preset | new | works | src-tauri/src/commands.rs apply_preset calls scenario_service::apply_preset, which deploys and undeploys targets in crates/hive-core/src/deploy/scenario_service.rs. | Hive-only command that applies a preset onto installed or selected agents. |  |
| `ipc.begin_market_install` | begin_market_install | new | works | src-tauri/src/commands.rs begin_market_install spawns install_market_work and records installed or error status. | Hive-only command that starts a marketplace install on a background thread. |  |
| `ipc.market_install_result` | market_install_result | new | works | src-tauri/src/commands.rs market_install_result returns the status string written by the install thread. | Hive-only command that reports the in-progress marketplace install. |  |
| `ipc.cancel_market_install` | cancel_market_install | new | works | src-tauri/src/commands.rs cancel_market_install sets the cancel flag and kills the recorded git process. | Hive-only command that cancels a running marketplace clone. |  |
| `ipc.save_skill_draft` | save_skill_draft | new | works | src-tauri/src/commands.rs save_skill_draft writes the supplied files and VersionStore::save_autosave. | Hive-only command that saves a skill draft and an autosave version. |  |
| `ipc.snapshot_skill_draft` | snapshot_skill_draft | new | works | src-tauri/src/commands.rs snapshot_skill_draft calls VersionStore::snapshot with the label. | Hive-only command that stores a named draft snapshot. |  |
| `ipc.restore_skill_draft` | restore_skill_draft | new | works | src-tauri/src/commands.rs restore_skill_draft calls VersionStore::restore for the version id. | Hive-only command that restores a draft version onto disk. |  |
| `ipc.publish_skill_draft` | publish_skill_draft | new | works | src-tauri/src/commands.rs publish_skill_draft calls publish_draft with the center path and deployment targets. | Hive-only command that publishes a draft, or asks for confirmation when publish_draft refuses. |  |
| `ipc.fix_skill_draft` | fix_skill_draft | new | works | src-tauri/src/commands.rs fix_skill_draft calls hive_core::authoring::skill::apply_skill_fix. | Hive-only command that applies one validator fix to a draft. |  |
| `ipc.save_agent_draft` | save_agent_draft | new | works | src-tauri/src/commands.rs save_agent_draft writes the supplied text to AGENT.md. | Hive-only command that saves an agent draft. |  |
| `ipc.export_agent_draft` | export_agent_draft | new | works | src-tauri/src/commands.rs export_agent_draft saves AGENT.md and copies it to export/AGENT.md. | Hive-only command that exports an agent draft file. |  |
| `ipc.save_generated_draft` | save_generated_draft | new | works | src-tauri/src/commands.rs save_generated_draft runs GenerationSession and copies SKILL.md. | save_generated_draft rejects an empty goal, runs GenerationSession, and copies SKILL.md into the authoring draft. |  |
| `ipc.apply_optimized_description` | apply_optimized_description | new | partial | src-tauri/src/commands.rs apply_optimized_description calls apply_optimized in crates/hive-core/src/analytics/optimize.rs, then writes a hardcoded eval suite. | Rewrites the description for real and also stores a canned suite whose expectations are the word applied. |  |
| `ipc.record_compare_verdict` | record_compare_verdict | new | works | src-tauri/src/commands.rs record_compare_verdict calls store_verdict with the id and verdict. | Hive-only command that stores a comparison verdict. |  |
| `ipc.write_history_baseline` | write_history_baseline | new | works | src-tauri/src/commands.rs write_history_baseline reads test-runs/last.json when it has cells, and otherwise writes the three-row chain. | write_history_baseline writes one measured baseline row when test-runs/last.json has cells. With no run it writes the three-row chain, including v2-optimized as the current best. |  |
| `ipc.set_provider_secret` | set_provider_secret | new | works | src-tauri/src/commands.rs set_provider_secret calls store_provider_secret and records the backend label. | Hive-only command that stores a provider secret. |  |
| `ipc.clear_endpoint_secret` | clear_endpoint_secret | new | works | src-tauri/src/commands.rs clear_endpoint_secret calls clear_provider_secret. | Hive-only command that deletes a provider secret. |  |
| `ipc.store_backup_pat` | store_backup_pat | new | works | src-tauri/src/commands.rs store_backup_pat calls store_git_token and records the backend. | Hive-only command that stores a backup token. |  |
| `ipc.revoke_backup_token` | revoke_backup_token | new | works | src-tauri/src/commands.rs revoke_backup_token calls clear_git_token and records whether the token is gone. | Hive-only command that revokes the backup token. |  |
| `ipc.set_aside_backup` | set_aside_backup | new | works | src-tauri/src/commands.rs set_aside_backup copies the library aside and writes the note. | Hive-only command that copies the library to an aside folder. |  |
| `ipc.pull_project_center` | pull_project_center | new | works | src-tauri/src/commands.rs pull_project_center writes the supplied name and body into projects/<name>/SKILL.md. | Hive-only command that writes a project skill document from the given body. |  |
| `ipc.export_diagnostics` | export_diagnostics | new | works | src-tauri/src/commands.rs export_diagnostics gathers log text and calls export_logs_zip in crates/hive-core/src/util/diagnostics.rs, which sanitizes before zipping. | Hive-only command that writes a sanitized diagnostics zip to the chosen path. |  |
| `ipc.git_backup_restore_remote` | git_backup_restore_remote | new | works | src-tauri/src/commands.rs git_backup_restore_remote calls git_backup::clone_repo and copies the clone into the library. | Hive-only command that restores a library from a remote URL. |  |
| `ipc.skip_first_run_restore` | skip_first_run_restore | new | works | src-tauri/src/commands.rs skip_first_run_restore sets the first_run_restore setting to skipped. | Hive-only command that records skipping the first-run restore. |  |
| `ipc.updater_proxy` | updater_proxy | new | works | src-tauri/src/lib.rs updater_proxy calls hive_core::updater::updater_proxy in crates/hive-core/src/updater.rs, which keeps only http and https URLs. | Hive-only command that accepts a proxy URL for the updater. |  |
| `ipc.download_and_install_update` | download_and_install_update | new | works | src-tauri/src/lib.rs download_and_install_update calls the Tauri updater check and download_and_install, then stores the outcome. | Hive-only command that downloads and installs an app update. |  |
| `ipc.rebuild_tray_menu` | rebuild_tray_menu | new | works | src-tauri/src/lib.rs rebuild_tray_menu calls tray::refresh. | Hive-only command that rebuilds the tray menu. |  |
| `ipc.list_base_entries` | list_base_entries | new | works | src-tauri/src/lib.rs list_base_entries reads central_repo::skills_dir and returns the entry names. | Hive-only command that lists library directory entries. |  |

### IPC presets

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.get_presets` | get_presets | must | works | Skill Hive has no get_presets command. The reference implementation is src-tauri/src/commands/presets.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/presets.rs |
| `ipc.get_active_preset` | get_active_preset | must | works | Skill Hive has no get_active_preset command. The reference implementation is src-tauri/src/commands/presets.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/presets.rs |
| `ipc.create_preset` | create_preset | must | works | src-tauri/src/commands.rs create_preset rejects an empty name and insert_scenario persists a new preset id. | Creates a named preset. Description and icon stay unset, matching the optional reference fields. |  |
| `ipc.update_preset` | update_preset | must | partial | src-tauri/src/commands.rs update_preset calls update_scenario with the new name and None for description and icon. | Renames the preset and clears description and icon because those arguments are forced empty. | src-tauri/src/commands/presets.rs |
| `ipc.delete_preset` | delete_preset | must | works | src-tauri/src/commands.rs delete_preset calls SkillStore::delete_scenario for the given id. | Deletes the preset record. |  |
| `ipc.apply_preset_to_default` | apply_preset_to_default | must | works | Skill Hive has no apply_preset_to_default command. The reference implementation is src-tauri/src/commands/presets.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/presets.rs |
| `ipc.switch_preset` | switch_preset | must | works | Skill Hive has no switch_preset command. The reference implementation is src-tauri/src/commands/presets.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/presets.rs |
| `ipc.add_skill_to_preset` | add_skill_to_preset | must | works | src-tauri/src/commands.rs add_skill_to_preset calls scenario_service::add_membership in crates/hive-core/src/deploy/scenario_service.rs. | Adds the skill to the preset and records membership for installed tools. |  |
| `ipc.remove_skill_from_preset` | remove_skill_from_preset | must | works | src-tauri/src/commands.rs remove_skill_from_preset calls SkillStore::remove_skill_from_scenario. | Removes the skill from the preset membership. |  |
| `ipc.reorder_presets` | reorder_presets | must | works | src-tauri/src/commands.rs reorder_presets calls SkillStore::reorder_scenarios with the supplied id list. | Persists the new preset order. |  |
| `ipc.get_preset_skill_order` | get_preset_skill_order | must | works | Skill Hive has no get_preset_skill_order command. The reference implementation is src-tauri/src/commands/presets.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/presets.rs |
| `ipc.reorder_preset_skills` | reorder_preset_skills | must | works | src-tauri/src/commands.rs reorder_preset_skills calls SkillStore::reorder_scenario_skills. | Persists the skill order inside the preset. |  |
| `ipc.apply_preset_to_coding_agents` | apply_preset_to_coding_agents | must | works | Skill Hive has no apply_preset_to_coding_agents command. The reference implementation is src-tauri/src/commands/presets.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/presets.rs |

### IPC projects

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.get_projects` | get_projects | must | works | Skill Hive has no get_projects command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.add_project` | add_project | must | works | src-tauri/src/commands.rs add_project creates the directory and insert_project stores the project record. | Adds a project with a name and path. |  |
| `ipc.add_linked_workspace` | add_linked_workspace | must | works | Skill Hive has no add_linked_workspace command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.remove_project` | remove_project | must | works | Skill Hive has no remove_project command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.reveal_project_folder` | reveal_project_folder | must | works | Skill Hive has no reveal_project_folder command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.reorder_projects` | reorder_projects | must | works | Skill Hive has no reorder_projects command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.scan_projects` | scan_projects | must | works | Skill Hive has no scan_projects command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.get_project_agent_targets` | get_project_agent_targets | must | works | Skill Hive has no get_project_agent_targets command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.get_project_skills` | get_project_skills | must | works | Skill Hive has no get_project_skills command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.get_project_skill_document` | get_project_skill_document | must | works | Skill Hive has no get_project_skill_document command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.import_project_skill_to_center` | import_project_skill_to_center | must | works | Skill Hive has no import_project_skill_to_center command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.update_project_skill_to_center` | update_project_skill_to_center | must | works | Skill Hive has no update_project_skill_to_center command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.slugify_skill_names` | slugify_skill_names | must | works | Skill Hive has no slugify_skill_names command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.export_skill_to_project` | export_skill_to_project | must | works | Skill Hive has no export_skill_to_project command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.update_project_skill_from_center` | update_project_skill_from_center | must | works | Skill Hive has no update_project_skill_from_center command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.toggle_project_skill` | toggle_project_skill | must | works | Skill Hive has no toggle_project_skill command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |
| `ipc.delete_project_skill` | delete_project_skill | must | works | Skill Hive has no delete_project_skill command. The reference implementation is src-tauri/src/commands/projects.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/projects.rs |

### IPC scan

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.scan_local_skills` | scan_local_skills | must | works | src-tauri/src/commands.rs scan_local_skills calls scanner::scan_roots and writes DiscoveredSkillRecord rows. | Scans installed agent skill directories and returns the discovered rows. |  |
| `ipc.import_existing_skill` | import_existing_skill | must | works | src-tauri/src/commands.rs import_existing_skill finds the scanned path and calls scanner::import_scanned, which install_local copies into the library. | Imports one discovered skill, with an optional rename. |  |
| `ipc.import_all_discovered` | import_all_discovered | must | works | src-tauri/src/commands.rs import_all_discovered calls scanner::import_all in crates/hive-core/src/library/scanner.rs. | Imports every discovered skill that is not already managed. |  |

### IPC settings

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.get_settings` | get_settings | must | works | src-tauri/src/lib.rs get_settings loads one key with SkillStore::get_setting. | Same key lookup the reference settings command performs. |  |
| `ipc.log_startup_event` | log_startup_event | must | works | src-tauri/src/lib.rs log_startup_event strips control characters, caps the label at 64 characters, and writes a log line. | Same startup log sanitizer as the reference command. |  |
| `ipc.set_settings` | set_settings | must | partial | src-tauri/src/lib.rs set_settings writes the key with set_setting and does not toggle the tray the way src-tauri/src/commands/settings.rs does for show_tray_icon. | Persists arbitrary settings and skips the tray and close-action side effect. | src-tauri/src/commands/settings.rs |
| `ipc.get_central_repo_path` | get_central_repo_path | must | works | src-tauri/src/commands.rs get_central_repo_path returns central_repo::base_dir. | Returns the resolved library directory. |  |
| `ipc.get_central_repo_path_override` | get_central_repo_path_override | must | works | Skill Hive has no get_central_repo_path_override command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.get_central_repo_pending_path` | get_central_repo_pending_path | must | works | Skill Hive has no get_central_repo_pending_path command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.get_central_repo_warnings` | get_central_repo_warnings | must | works | Skill Hive has no get_central_repo_warnings command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.set_central_repo_path` | set_central_repo_path | must | works | src-tauri/src/commands.rs set_central_repo_path calls central_repo::set_base_dir_override, clearing it for an empty path. | Sets or clears the library location override. |  |
| `ipc.open_central_repo_folder` | open_central_repo_folder | must | works | Skill Hive has no open_central_repo_folder command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.check_app_update` | check_app_update | must | works | src-tauri/src/lib.rs check_app_update compares the package version with a release tag. | check_app_update compares the package version with SKILL_HIVE_RELEASE_TAG, app-release.txt, or the GitHub release and returns current, available, or offline. | src-tauri/src/commands/settings.rs |
| `ipc.get_diagnostic_info` | get_diagnostic_info | must | works | Skill Hive has no get_diagnostic_info command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.check_last_panic` | check_last_panic | must | works | Skill Hive has no check_last_panic command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.clear_last_panic` | clear_last_panic | must | works | Skill Hive has no clear_last_panic command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.get_recent_log_excerpt` | get_recent_log_excerpt | must | works | Skill Hive has no get_recent_log_excerpt command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.export_logs_zip` | export_logs_zip | must | works | Skill Hive has no export_logs_zip command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.app_exit` | app_exit | must | works | Skill Hive has no app_exit command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.restart_app` | restart_app | must | works | Skill Hive has no restart_app command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |
| `ipc.update_install_blocker` | update_install_blocker | must | partial | src-tauri/src/lib.rs update_install_blocker calls hive_core::updater::update_install_blocker in crates/hive-core/src/updater.rs, which only checks active_runs. | Blocks while a test run is active. The reference command also checks whether the executable file is locked. | src-tauri/src/commands/settings.rs |
| `ipc.hide_to_tray` | hide_to_tray | must | works | Skill Hive has no hide_to_tray command. The reference implementation is src-tauri/src/commands/settings.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/settings.rs |

### IPC skills

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.get_managed_skills` | get_managed_skills | must | partial | src-tauri/src/commands.rs get_managed_skills loads SkillStore rows, tags, and targets, then copies the local SKILL.md into the remote file field. | Lists real library skills. The remote body is a copy of the local file, and only SKILL.md is included. | src-tauri/src/commands/skills.rs |
| `ipc.get_skills_for_preset` | get_skills_for_preset | must | works | Skill Hive has no get_skills_for_preset command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.get_skill_document` | get_skill_document | must | works | Skill Hive has no get_skill_document command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.get_source_skill_document` | get_source_skill_document | must | works | Skill Hive has no get_source_skill_document command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.get_skill_source_diff` | get_skill_source_diff | must | works | Skill Hive has no get_skill_source_diff command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.delete_managed_skill` | delete_managed_skill | must | works | Skill Hive has no delete_managed_skill command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.delete_managed_skills` | delete_managed_skills | must | works | src-tauri/src/commands.rs delete_managed_skills removes deployed directories, the central copy, and the SkillStore row for each id. | Deletes the selected library skills and their deployed copies. |  |
| `ipc.install_local` | install_local | must | works | Skill Hive has no install_local command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.install_git` | install_git | must | works | Skill Hive has no install_git command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.install_from_skillssh` | install_from_skillssh | must | partial | src-tauri/src/commands.rs install_from_skillssh clones a GitHub source and install_local copies a SKILL.md folder. It has no progress events or subpath selection. | Installs a marketplace repo, without the reference command's progress, cancel registry, or subpath. | src-tauri/src/commands/skills.rs |
| `ipc.preview_git_install` | preview_git_install | must | partial | src-tauri/src/commands.rs preview_repository clones a URL or reads a local directory, then preview_local_repo in crates/hive-core/src/library/git_fetcher.rs lists skills. | Previews git and local folders. It does not download skills.sh site references the way the reference command does. | src-tauri/src/commands/skills.rs |
| `ipc.confirm_git_install` | confirm_git_install | must | works | Skill Hive has no confirm_git_install command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.cancel_git_preview` | cancel_git_preview | must | works | Skill Hive has no cancel_git_preview command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.check_skill_update` | check_skill_update | must | works | src-tauri/src/commands.rs check_skill_update compares trees through skill_check and stores update_status. | check_skill_update compares the central tree with the source tree and stores update_status. | src-tauri/src/commands/skills.rs |
| `ipc.check_all_skill_updates` | check_all_skill_updates | must | works | src-tauri/src/commands.rs check_all_skill_updates returns the available count. commands::tests::check_updates_records_a_changed_source_in_the_database covers the database write. | check_all_skill_updates returns the count of update_available or pending_removals, not the skill count. | src-tauri/src/commands/skills.rs |
| `ipc.update_skill` | update_skill | must | works | Skill Hive has no update_skill command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.reimport_local_skill` | reimport_local_skill | must | works | Skill Hive has no reimport_local_skill command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.batch_update_skills` | batch_update_skills | must | works | Skill Hive has no batch_update_skills command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.relink_local_skill_source` | relink_local_skill_source | must | works | Skill Hive has no relink_local_skill_source command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.detach_local_skill_source` | detach_local_skill_source | must | works | Skill Hive has no detach_local_skill_source command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.get_all_tags` | get_all_tags | must | works | Skill Hive has no get_all_tags command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |
| `ipc.set_skill_tags` | set_skill_tags | must | works | src-tauri/src/commands.rs set_skill_tags calls SkillStore::set_tags_for_skill. | Replaces the tag list on one skill. |  |
| `ipc.rename_tag` | rename_tag | must | works | src-tauri/src/commands.rs rename_tag calls SkillStore::rename_tag. | Renames a tag across the library. |  |
| `ipc.delete_tag` | delete_tag | must | works | src-tauri/src/commands.rs delete_tag calls SkillStore::delete_tag. | Deletes a tag from every skill. |  |
| `ipc.cancel_install` | cancel_install | must | partial | src-tauri/src/commands.rs cancel_install calls cancel_git_preview in crates/hive-core/src/library/git_fetcher.rs and returns false when no preview id is passed. | Cancels a git preview directory. It does not cancel an in-flight install by the reference registry key. | src-tauri/src/commands/skills.rs |
| `ipc.batch_import_folder` | batch_import_folder | must | works | Skill Hive has no batch_import_folder command. The reference implementation is src-tauri/src/commands/skills.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/skills.rs |

### IPC sync

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.sync_skill_to_tool` | sync_skill_to_tool | must | works | src-tauri/src/commands.rs sync_skill_to_tool calls scenario_service::sync_one with DeployIntent::Add. | Deploys the skill into the selected agent's skills directory. |  |
| `ipc.unsync_skill_from_tool` | unsync_skill_from_tool | must | works | src-tauri/src/commands.rs unsync_skill_from_tool calls scenario_service::sync_one with DeployIntent::Remove. | Removes the skill deployment from the selected agent. |  |
| `ipc.get_skill_tool_toggles` | get_skill_tool_toggles | must | works | Skill Hive has no get_skill_tool_toggles command. The reference implementation is src-tauri/src/commands/sync.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/sync.rs |
| `ipc.set_skill_tool_toggle` | set_skill_tool_toggle | must | works | Skill Hive has no set_skill_tool_toggle command. The reference implementation is src-tauri/src/commands/sync.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/sync.rs |

### IPC tools

| id | item | priority | class | evidence | note | port |
|---|---|---|---|---|---|---|
| `ipc.get_tool_status` | get_tool_status | must | works | src-tauri/src/commands.rs get_tool_status calls tool_service::tool_statuses and returns key, install state, enablement, and paths. | Lists agent adapters from the same tool service the reference command uses. |  |
| `ipc.set_tool_enabled` | set_tool_enabled | must | works | src-tauri/src/commands.rs set_tool_enabled calls tool_service::set_tool_enabled. | Enables or disables one agent. |  |
| `ipc.set_all_tools_enabled` | set_all_tools_enabled | must | works | src-tauri/src/commands.rs set_all_tools_enabled calls tool_service::enable_all or disable_all. | Enables or disables every agent. |  |
| `ipc.get_tool_order_cmd` | get_tool_order_cmd | must | works | Skill Hive has no get_tool_order_cmd command. The reference implementation is src-tauri/src/commands/tools.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/tools.rs |
| `ipc.set_tool_order_cmd` | set_tool_order_cmd | must | works | src-tauri/src/commands.rs set_tool_order_cmd calls tool_service::reorder_tools. | Saves the agent display order. |  |
| `ipc.set_custom_tool_path` | set_custom_tool_path | must | works | src-tauri/src/commands.rs set_custom_tool_path calls tool_service::set_tool_path. | Updates a custom agent's skills directory. |  |
| `ipc.reset_custom_tool_path` | reset_custom_tool_path | must | works | src-tauri/src/commands.rs reset_custom_tool_path calls tool_service::reset_tool_path and returns the restored directory. | Restores a custom agent's default skills path. |  |
| `ipc.set_custom_tool_project_path` | set_custom_tool_project_path | must | works | src-tauri/src/commands.rs set_custom_tool_project_path calls tool_service::set_project_path. | Sets the project-relative skills directory for a custom agent. |  |
| `ipc.reset_custom_tool_project_path` | reset_custom_tool_project_path | must | works | Skill Hive has no reset_custom_tool_project_path command. The reference implementation is src-tauri/src/commands/tools.rs. | The command is registered and calls the ported engine or store. | src-tauri/src/commands/tools.rs |
| `ipc.add_custom_tool` | add_custom_tool | must | works | src-tauri/src/commands.rs add_custom_tool calls tool_service::add_custom_tool and rejects a name collision. | Registers a custom agent with a skills path and optional project path. |  |
| `ipc.remove_custom_tool` | remove_custom_tool | must | works | src-tauri/src/commands.rs remove_custom_tool calls tool_service::remove_custom_tool. | Removes a custom agent. |  |

## Top 20 gaps

1. **missing / must** `cli.skills.tag.delete` skills tag delete. No skills tag delete command. Delete exists as an IPC command, not on the CLI. Port: `src-tauri/src/bin/skills-manager-cli.rs`.
2. **missing / must** `cli.skills.tag.rename` skills tag rename. No skills tag rename command. Rename exists as an IPC command, not on the CLI. Port: `src-tauri/src/bin/skills-manager-cli.rs`.
3. **missing / must** `cli.skills.tag.set` skills tag set. No skills tag set command. The hive CLI can add, remove, and list tags only. Port: `src-tauri/src/bin/skills-manager-cli.rs`.
4. **missing / must** `ipc.add_linked_workspace` add_linked_workspace. No command adds a linked agent workspace as a project. Port: `src-tauri/src/commands/projects.rs`.
5. **missing / must** `ipc.app_exit` app_exit. No command exits the desktop app. Port: `src-tauri/src/commands/settings.rs`.
6. **missing / must** `ipc.apply_preset_to_coding_agents` apply_preset_to_coding_agents. No command applies a preset onto coding-agent directories. Port: `src-tauri/src/commands/presets.rs`.
7. **missing / must** `ipc.apply_preset_to_default` apply_preset_to_default. No command applies a preset as the exclusive default deployment. Port: `src-tauri/src/commands/presets.rs`.
8. **missing / must** `ipc.backup_device_name` backup_device_name. No command reads the saved backup device name. Port: `src-tauri/src/commands/git_backup.rs`.
9. **missing / must** `ipc.batch_import_folder` batch_import_folder. No command imports every skill folder under a directory. Port: `src-tauri/src/commands/skills.rs`.
10. **missing / must** `ipc.batch_update_skills` batch_update_skills. No command updates many skills from their sources. Port: `src-tauri/src/commands/skills.rs`.
11. **missing / must** `ipc.cancel_git_preview` cancel_git_preview. No command named cancel_git_preview; cancel_install drops a preview id. Port: `src-tauri/src/commands/skills.rs`.
12. **missing / must** `ipc.check_last_panic` check_last_panic. No command reads the last panic record. Port: `src-tauri/src/commands/settings.rs`.
13. **missing / must** `ipc.clear_last_panic` clear_last_panic. No command clears the last panic record. Port: `src-tauri/src/commands/settings.rs`.
14. **missing / must** `ipc.confirm_git_install` confirm_git_install. No command named confirm_git_install; confirm_git_install_skill is the hive name. Port: `src-tauri/src/commands/skills.rs`.
15. **missing / must** `ipc.delete_global_local_skill` delete_global_local_skill. No command deletes a skill from the global agent workspace. Port: `src-tauri/src/commands/agent_workspace.rs`.
16. **missing / must** `ipc.delete_managed_skill` delete_managed_skill. No singular delete command; delete_managed_skills deletes by a list of ids. Port: `src-tauri/src/commands/skills.rs`.
17. **missing / must** `ipc.delete_project_skill` delete_project_skill. No command deletes a skill from a project. Port: `src-tauri/src/commands/projects.rs`.
18. **missing / must** `ipc.detach_local_skill_source` detach_local_skill_source. No command detaches a skill from its local source. Port: `src-tauri/src/commands/skills.rs`.
19. **missing / must** `ipc.export_logs_zip` export_logs_zip. No command named export_logs_zip; export_diagnostics is a separate hive command. Port: `src-tauri/src/commands/settings.rs`.
20. **missing / must** `ipc.export_skill_to_project` export_skill_to_project. No command exports a library skill into a project. Port: `src-tauri/src/commands/projects.rs`.

## Order of work

Port a reference module only when the table above shows it missing or covering under 0.2 of the reference lines. Keep modules whose coverage is above 0.8 and extend them in place. Put this header on every file copied or rewritten from the reference:

```
// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang
```

Also keep `THIRD_PARTY_LICENSES/skills-manager.MIT.txt` holding the upstream MIT text and the copyright line `Copyright (c) 2026 Tianliang Zhang`.

### Engines to bring over

1. copy in `src-tauri/src/core/merge/apply.rs` (969 lines).
2. copy in `src-tauri/src/core/merge/decision.rs` (772 lines).
3. copy in `src-tauri/src/core/well_known.rs` (643 lines).
4. copy in `src-tauri/src/core/app_state.rs` (544 lines).
5. copy in `src-tauri/src/core/project_scanner.rs` (432 lines).
6. copy in `src-tauri/src/core/git_credentials.rs` (345 lines).
7. copy in `src-tauri/src/core/git2_engine.rs` (343 lines).
8. copy in `src-tauri/src/core/merge/resolve.rs` (337 lines).
9. copy in `src-tauri/src/core/merge/treebuild.rs` (303 lines).
10. copy in `src-tauri/src/core/removals.rs` (274 lines).
11. copy in `src-tauri/src/core/merge/validate.rs` (273 lines).
12. copy in `src-tauri/src/core/merge/pending.rs` (263 lines).
13. copy in `src-tauri/src/core/skillssh_api.rs` (245 lines).
14. copy in `src-tauri/src/core/skill_auto_updater.rs` (241 lines).
15. copy in `src-tauri/src/core/merge/snapshot.rs` (232 lines).
16. copy in `src-tauri/src/core/github_api.rs` (229 lines).
17. copy in `src-tauri/src/core/merge/protocol.rs` (174 lines).
18. copy in `src-tauri/src/core/file_manager.rs` (169 lines).
19. copy in `src-tauri/src/core/install_cancel.rs` (86 lines).
20. replace the short rewrite with `src-tauri/src/core/git_backup.rs` (3125 lines, current coverage 0.005 in `crates/hive-core/src/backup/git_backup.rs`).
21. replace the short rewrite with `src-tauri/src/core/git_fetcher.rs` (2486 lines, current coverage 0.017 in `crates/hive-core/src/library/git_fetcher.rs`).
22. replace the short rewrite with `src-tauri/src/core/scenario_service.rs` (1716 lines, current coverage 0.011 in `crates/hive-core/src/deploy/scenario_service.rs`).
23. replace the short rewrite with `src-tauri/src/core/sync_engine.rs` (1251 lines, current coverage 0.015 in `crates/hive-core/src/deploy/sync_engine.rs`).
24. replace the short rewrite with `src-tauri/src/core/sync_metadata.rs` (755 lines, current coverage 0.022 in `crates/hive-core/src/library/sync_metadata.rs`).
25. replace the short rewrite with `src-tauri/src/core/content_hash.rs` (668 lines, current coverage 0.031 in `crates/hive-core/src/library/content_hash.rs`).
26. replace the short rewrite with `src-tauri/src/core/auto_backup.rs` (612 lines, current coverage 0.0 in `crates/hive-core/src/backup/auto_backup.rs`).
27. replace the short rewrite with `src-tauri/src/core/tool_service.rs` (536 lines, current coverage 0.044 in `crates/hive-core/src/deploy/tool_service.rs`).
28. replace the short rewrite with `src-tauri/src/core/scanner.rs` (415 lines, current coverage 0.0 in `crates/hive-core/src/library/scanner.rs`).
29. replace the short rewrite with `src-tauri/src/core/skill_metadata.rs` (373 lines, current coverage 0.043 in `crates/hive-core/src/library/skill_metadata.rs`).
30. replace the short rewrite with `src-tauri/src/core/installer.rs` (369 lines, current coverage 0.0 in `crates/hive-core/src/library/installer.rs`).

### Product gaps after those engines

1. Dashboard: 0 missing, 0 fake, 2 partial. Reference files: `src/views/Dashboard.tsx:39`, `src/views/Dashboard.tsx:117`.
2. Library: 0 missing, 3 fake, 8 partial. Reference files: `src/components/SyncDots.tsx`, `src/views/MySkills.tsx:359`, `src/views/MySkills.tsx:1471`, `src/context/AppContext.tsx:252`, `src/components/MultiSelectToolbar.tsx`, `src/views/MySkills.tsx:839`, `src/views/MySkills.tsx:974`, `src/components/SkillDetailPanel.tsx`.
3. Install Skills: 0 missing, 0 fake, 4 partial. Reference files: `src/views/InstallSkills.tsx:755`, `src/views/InstallSkills.tsx:302`, `src/views/InstallSkills.tsx:1267`, `src/views/InstallSkills.tsx:1458`.
4. Global Workspace: 0 missing, 0 fake, 5 partial. Reference files: `src/views/WorkspaceView.tsx:833`, `src/views/WorkspaceView.tsx:887`, `src/views/WorkspaceView.tsx:719`, `src/components/AddSkillsSheet.tsx`, `src/components/PresetBar.tsx`.
5. Project Workspace: 1 missing, 1 fake, 3 partial. Reference files: `src/views/ProjectDetail.tsx:905`, `src/views/ProjectDetail.tsx:1227`, `src/views/ProjectDetail.tsx:1107`, `src/views/ProjectDetail.tsx:1608`, `src/components/AddSkillsSheet.tsx:460`.
6. Backup: 0 missing, 0 fake, 2 partial. Reference files: `src/views/Backup.tsx:779`, `src/components/GitSetupDialog.tsx`.
7. Settings: 3 missing, 4 fake, 9 partial. Reference files: `src/views/Settings.tsx:828`, `src/views/Settings.tsx:455`, `src/views/Settings.tsx:1384`, `src/components/CloseActionGuard.tsx`, `src/views/Settings.tsx:1510`, `src/views/Settings.tsx:1544`, `src/views/Settings.tsx:1715`, `src/views/Settings.tsx:526`.
8. Shell: 0 missing, 1 fake, 4 partial. Reference files: `src/components/CommandPalette.tsx`, `src/components/Layout.tsx:17`, `src/components/InstallToast.tsx`, `src/context/AppContext.tsx:312`, `src/components/DetailSheet.tsx`.
9. Sidebar: 0 missing, 0 fake, 4 partial. Reference files: `src/components/Sidebar.tsx:708`, `src/components/Sidebar.tsx:424`, `src/components/Sidebar.tsx:243`, `src/components/Sidebar.tsx:583`.
10. Shortcuts: 1 missing, 0 fake, 0 partial. Reference files: `src/components/Layout.tsx:25`.
11. Tray: 3 missing, 3 fake, 3 partial. Reference files: `src-tauri/src/lib.rs:278`, `src-tauri/src/lib.rs:286`, `src-tauri/src/lib.rs:289`, `src-tauri/src/lib.rs:305`, `src-tauri/src/lib.rs:313`, `src-tauri/src/lib.rs:328`, `src-tauri/src/lib.rs:340`, `src-tauri/src/lib.rs:357`.
12. i18n: 0 missing, 0 fake, 1 partial. Reference files: `src/i18n/en.json`.
13. Core: 0 missing, 0 fake, 18 partial. Reference files: `src-tauri/src/core/sync_metadata.rs`, `src-tauri/src/core/skill_metadata.rs`, `src-tauri/src/core/content_hash.rs`, `src-tauri/src/core/tool_service.rs`, `src-tauri/src/core/scanner.rs`, `src-tauri/src/core/installer.rs`, `src-tauri/src/core/git_fetcher.rs`, `src-tauri/src/core/well_known.rs`.
14. Validation: 0 missing, 0 fake, 11 partial. Extend the Skill Hive modules already named in the table.
15. Skill editor: 0 missing, 0 fake, 2 partial. Extend the Skill Hive modules already named in the table.
16. Agent editor: 0 missing, 0 fake, 1 partial. Extend the Skill Hive modules already named in the table.
17. Providers: 0 missing, 0 fake, 3 partial. Extend the Skill Hive modules already named in the table.
18. Test engine: 0 missing, 0 fake, 6 partial. Extend the Skill Hive modules already named in the table.
19. Generation: 0 missing, 0 fake, 1 partial. Extend the Skill Hive modules already named in the table.
20. Analytics: 0 missing, 0 fake, 5 partial. Extend the Skill Hive modules already named in the table.
21. CLI agents: 0 missing, 0 fake, 1 partial. Reference files: `src-tauri/src/bin/skills-manager-cli.rs`.
22. CLI git: 0 missing, 0 fake, 5 partial. Reference files: `src-tauri/src/bin/skills-manager-cli.rs`.
23. CLI hive-only: 0 missing, 7 fake, 6 partial. Extend the Skill Hive modules already named in the table.
24. CLI presets: 0 missing, 0 fake, 7 partial. Reference files: `src-tauri/src/bin/skills-manager-cli.rs`.
25. CLI repo: 0 missing, 0 fake, 1 partial. Reference files: `src-tauri/src/bin/skills-manager-cli.rs`.
26. CLI skills: 3 missing, 3 fake, 12 partial. Reference files: `src-tauri/src/bin/skills-manager-cli.rs`.
27. IPC agent_workspace: 5 missing, 0 fake, 0 partial. Reference files: `src-tauri/src/commands/agent_workspace.rs`.
28. IPC git_backup: 17 missing, 0 fake, 3 partial. Reference files: `src-tauri/src/commands/git_backup.rs`.
29. IPC hive-only: 0 missing, 2 fake, 1 partial. Extend the Skill Hive modules already named in the table.
30. IPC presets: 6 missing, 0 fake, 1 partial. Reference files: `src-tauri/src/commands/presets.rs`.
31. IPC projects: 16 missing, 0 fake, 0 partial. Reference files: `src-tauri/src/commands/projects.rs`.
32. IPC settings: 12 missing, 1 fake, 2 partial. Reference files: `src-tauri/src/commands/settings.rs`.
33. IPC skills: 16 missing, 2 fake, 4 partial. Reference files: `src-tauri/src/commands/skills.rs`.
34. IPC sync: 2 missing, 0 fake, 0 partial. Reference files: `src-tauri/src/commands/sync.rs`.
35. IPC tools: 2 missing, 0 fake, 0 partial. Reference files: `src-tauri/src/commands/tools.rs`.

Do the engine copy before the screen work that calls those engines. A screen that already renders can stay; wire it to the ported command instead of adding a second implementation.

## Still open

These rows are still not works. Class is the current cell.

- `ui.015` (partial, Library) Per-card agent badges (SyncDots) with click-to-install/uninstall
- `ui.016` (partial, Library) Preset membership toggle per card and preset-scoped ordering via drag and drop
- `ui.017` (partial, Library) Card "…" menu: Check, Update/Reimport, Delete; "Update" pill with file-removal confirmation
- `ui.018` (partial, Library) "Check All" and "Update All (n)" toolbar actions; startup update round and update toasts
- `ui.020` (partial, Library) Multi-select with Enable/Disable, Sync to agents, Tags, Update, Delete
- `ui.023` (partial, Library) Skill detail panel: docs, source metadata, Local/Diff/Source tabs
- `ui.024` (partial, Library) Detail panel agent toggles for the viewed preset
- `ui.028` (partial, Install Skills) skills.sh marketplace: leaderboards, debounced search, source pills with overflow menu, cards, pagination, load more, one-click install with cancel
- `ui.029` (partial, Install Skills) Local install: select folder, select `.zip`/`.skill`, batch import with progress toast
- `ui.030` (partial, Install Skills) Scan agent folders for existing skills: rescan, rename, import one, import all
- `ui.031` (partial, Install Skills) Git install with clone preview, per-skill rename and selection, cancel
- `ui.032` (partial, Global Workspace) Route `/global-workspace` overview with agent cards and all-agent preset bar
- `ui.033` (partial, Global Workspace) Route `/global-workspace/:agentKey` agent page: on-disk skill list, status pills, search, tag filters, refresh, grid/list
- `ui.034` (partial, Global Workspace) Per-card Pull / Upload / Remove from agent / Delete local skill with confirmations
- `ui.037` (partial, Global Workspace) "Add Skill" → Add from Library sheet (global mode): search, tag/source filters, shift-click range, select all, add N
- `ui.038` (partial, Global Workspace) Preset bar pills (active ✓ / partial a/b / inactive) activating or deactivating a preset for the current scope
- `ui.039` (partial, Lobster Workspace) Route `/lobster-workspace` and `/lobster-workspace/:agentKey` (same view, lobster category)
- `ui.040` (partial, Project Workspace) Route `/project/:id`: grouped project skills, search, All/Enabled/Disabled filter, tag filters, refresh, grid/list, reveal folder
- `ui.041` (partial, Project Workspace) Project card actions: agent dots add/remove, Update Center, Update Project / Restore center version, enable toggle, delete
- `ui.042` (partial, Project Workspace) Project batch toolbar: Enable/Disable, Update Project, Update Center, Tags, Delete
- `ui.044` (partial, Project Workspace) Add from Library sheet in project mode with multi-agent target chips and per-project last-used memory
- `ui.046` (partial, Backup) Route `/backup`: status card, Back Up Now / Sync Now / Retry, remote URL, history with restore, scope, auto-backup toggle, device name
- `ui.047` (partial, Backup) GitHub device-flow sign-in and PAT connect, public-repo warning, reconnect
- `ui.048` (partial, Backup) Skill-aware merge conflicts ("Needs attention": keep mine / use remote / keep both)
- `ui.049` (partial, Backup) Git setup, recovery and set-aside dialogs; first-run restore wizard
- `ui.050` (partial, Backup) Disconnect this machine / revoke authorization / open remote deletion page
- `ui.051` (partial, Settings) Route `/settings`: agent cards with enable toggle, path edit/reset, project path, drag-reorder, Enable All / Disable All, Refresh
- `ui.055` (partial, Settings) Theme, text size, language
- `ui.056` (partial, Settings) Close behaviour and tray icon settings; close-action guard dialog
- `ui.058` (partial, Settings) Skill auto-update frequency and auto-apply
- `ui.059` (partial, Settings) Advanced git sync configuration (remote URL, disconnect, git engine, merge engine toggles)
- `ui.060` (partial, Settings) About: version, Check for Updates, Install Update / Download, Restart Now toast, repo warnings and crash banner
- `ui.061` (partial, Settings) Report Issue (diagnostics to clipboard + GitHub issue) and Export Logs zip
- `setting.close_action` (partial, Settings) Setting close_action
- `setting.git_backup_remote_url` (partial, Settings) Setting git_backup_remote_url
- `setting.github_auth_method` (partial, Settings) Setting github_auth_method
- `setting.show_tray_icon` (partial, Settings) Setting show_tray_icon
- `setting.update_check_ttl_minutes` (partial, Settings) Setting update_check_ttl_minutes
- `ui.063` (partial, Shell) Command palette (⌘K) over skills, presets, projects and actions with keyboard navigation
- `ui.064` (partial, Shell) Global shortcuts ⌘, (Settings) and ⌘R (reload data); window drag strip
- `ui.066` (partial, Shell) Toast system with loading/progress, action buttons and persistent toasts
- `ui.069` (partial, Shell) Shared widgets: ToggleSwitch, ConfirmDialog with detail chips, CardActionMenu, AgentIcon, SyncDots/ProjectAgentDots, DetailSheet
- `ui.010` (partial, Sidebar) Sidebar "Settings" link with update dot
- `ui.011` (partial, Sidebar) Sidebar "Presets" section: switch, drag-reorder, edit, delete, "New Preset"
- `ui.012` (partial, Sidebar) Sidebar "Global Workspace" section with "All Agents" entry and per-agent rows (drag-reorderable)
- `ui.013` (partial, Sidebar) Sidebar "Lobster Agents" section with "All Lobster Agents" and per-agent rows
- `ui.014` (partial, Sidebar) Sidebar "Project Workspaces" section: rows with sync-health dot, drag-reorder, delete, "Link Project"
- `ui.045` (partial, Sidebar) Linked workspaces (arbitrary skills root managed as a project)
- `ui.067` (partial, Tray) System tray: status line, updates item, Presets submenu (apply/remove), Open, Check for skill updates, Open Skills Folder, Quit; left-click restore on Windows/Linux
- `tray.tray-status-line.289` (partial, Tray) Tray item tray-status-line: format_status_line(data)
- `tray.id.340` (partial, Tray) Tray item id: label
- `i18n.catalog` (partial, i18n) Internationalization catalogs and locale switching
- `core.04` (partial, Core) SKILL.md detection, frontmatter parsing, name sanitisation
- `core.05` (partial, Core) Content hashing with shared content scope and EOL-insensitive tiebreaker
- `core.06` (partial, Core) Agent adapter registry (54 built-ins, custom tools, path overrides, ordering, categories)
- `core.07` (partial, Core) Global-directory scanning and import of unmanaged skills
- `core.10` (partial, Core) skills.sh marketplace search, leaderboard and website-synced skills with hardened zip extraction
- `core.11` (partial, Core) Update check with off-lock prefetch, status vocabulary, held-back removals, auto-updater
- `core.15` (partial, Core) Agent global workspaces with adoption and stranded-target repair
- `core.16` (partial, Core) Project and linked workspaces with bidirectional sync and enable/disable
- `core.20` (partial, Core) Automatic debounced backup with backoff
- `core.25` (partial, Core) Path safety helpers
- `core.26` (partial, Core) CLI with `--json` envelope, exit codes, full command tree and the published bridge binary
- `core.27` (partial, Core) Startup orchestration with stage timings, panic log, tray and single-instance
- `rule.S-14` (partial, Validation) S-14 `sanitize_skill_name` returns the input unchanged: no path separators, control characters, `<>:"/\
- `rule.S-21` (partial, Validation) S-21 mapping whose keys and values are strings (https://agentskills.io/specification)
- `rule.S-22` (partial, Validation) S-22 P: a single space-separated string (spec); C: space/comma string or list (https://code.claude.com/docs/en/skills)
- `rule.S-25` (partial, Validation) S-25 `context` ∈ `{fork}`; `effort` ∈ `{low, medium, high, xhigh, max}`; `shell` ∈ `{bash, powershell}`; `agent` ∈ `{Explore, Plan, general-purpose}` or a known cust
- `rule.S-27` (partial, Validation) S-27 `SKILL.md` body at most 500 lines (spec; skill-creator `SKILL.md`; https://code.claude.com/docs/en/skills)
- `rule.S-29` (partial, Validation) S-29 any Markdown file under the skill longer than 300 lines starts with a table of contents (skill-creator `SKILL.md`)
- `rule.S-30` (partial, Validation) S-30 every relative path mentioned in `SKILL.md` exists under the skill root and is at most one directory deep (https://agentskills.io/specification)
- `rule.S-32` (partial, Validation) S-32 files in `scripts/` have an interpreter line or known extension and, on POSIX, the execute bit (https://agentskills.io/specification, `scripts/` guidance)
- `rule.S-34` (partial, Validation) S-34 `grading.json` expectations use exactly `text`, `passed`, `evidence`; `benchmark.json` uses `configuration` ∈ `{with_skill, without_skill}` with `pass_rate` und
- `rule.A-18` (partial, Validation) A-18 no two files in one tree share a `name` (loaded in read order, flagged by `/doctor`) (https://code.claude.com/docs/en/sub-agents)
- `rule.A-19` (partial, Validation) A-19 combined descriptions under 15,000 tokens (https://code.claude.com/docs/en/sub-agents)
- `studio.skill-editor` (partial, Skill editor) Skill editor with validation
- `studio.drafts` (partial, Skill editor) Draft versioning
- `studio.agent-editor` (partial, Agent editor) Agent editor with validation
- `studio.provider-cli` (partial, Providers) Local agent CLI executor
- `studio.provider-secrets` (partial, Providers) Secrets, retries, rate limits, and cost
- `studio.provider-probe` (partial, Providers) Test connection flow
- `studio.test-engine` (partial, Test engine) Test run lifecycle and persistence
- `studio.test-sandbox` (partial, Test engine) Isolated working directories
- `studio.test-transcript` (partial, Test engine) Transcript capture and tool-call metrics
- `studio.test-grading` (partial, Test engine) Grading and aggregation
- `studio.test-replay` (partial, Test engine) Seeded fixtures and recorded responses
- `studio.test-kinds` (partial, Test engine) Four test kinds
- `studio.generate` (partial, Generation) Generation wizard
- `studio.results` (partial, Analytics) Results views
- `studio.history` (partial, Analytics) History and regression detection
- `studio.export` (partial, Analytics) Export of results
- `studio.compare` (partial, Analytics) Blind comparison and post-hoc analysis
- `studio.optimize` (partial, Analytics) Description optimization
- `cli.agents.list` (partial, CLI agents) agents list
- `cli.git.clone` (partial, CLI git) git clone
- `cli.git.pull` (partial, CLI git) git pull
- `cli.agents.def.show` (partial, CLI hive-only) agents def show
- `cli.providers.add` (partial, CLI hive-only) providers add
- `cli.providers.probe` (partial, CLI hive-only) providers probe
- `cli.test.run` (partial, CLI hive-only) test run
- `cli.generate.skill` (partial, CLI hive-only) generate skill
- `cli.generate.agent` (partial, CLI hive-only) generate agent
- `cli.presets.update` (partial, CLI presets) presets update
- `cli.presets.preview` (partial, CLI presets) presets preview
- `cli.presets.apply` (partial, CLI presets) presets apply
- `cli.presets.deactivate` (partial, CLI presets) presets deactivate
- `cli.presets.deploy` (partial, CLI presets) presets deploy
- `cli.presets.undeploy` (partial, CLI presets) presets undeploy
- `cli.presets.status` (partial, CLI presets) presets status
- `cli.repo.status` (partial, CLI repo) repo status
- `cli.skills.list` (partial, CLI skills) skills list
- `cli.skills.show` (partial, CLI skills) skills show
- `cli.skills.export` (partial, CLI skills) skills export
- `cli.skills.install` (partial, CLI skills) skills install
- `cli.skills.update` (partial, CLI skills) skills update
- `cli.skills.check` (partial, CLI skills) skills check
- `cli.skills.remove` (partial, CLI skills) skills remove
- `cli.skills.disable` (partial, CLI skills) skills disable
- `cli.skills.deploy` (partial, CLI skills) skills deploy
- `cli.skills.undeploy` (partial, CLI skills) skills undeploy
- `cli.skills.sync` (partial, CLI skills) skills sync
- `cli.skills.set-source` (partial, CLI skills) skills set-source
- `ipc.apply_optimized_description` (partial, IPC hive-only) apply_optimized_description
- `ipc.update_preset` (partial, IPC presets) update_preset
- `ipc.set_settings` (partial, IPC settings) set_settings
- `ipc.update_install_blocker` (partial, IPC settings) update_install_blocker
- `ipc.get_managed_skills` (partial, IPC skills) get_managed_skills
- `ipc.install_from_skillssh` (partial, IPC skills) install_from_skillssh
- `ipc.preview_git_install` (partial, IPC skills) preview_git_install
- `ipc.cancel_install` (partial, IPC skills) cancel_install
