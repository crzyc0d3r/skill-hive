# Skill Hive user guide

Skill Hive is the desktop app for creating, generating, and testing agent skills. This guide covers every screen. A colleague who has not used the app can follow [Workflow D.2](#workflow-d2) from this page alone.

## Dashboard

Open `/`. The page shows how many skills, agents, and projects you have, plus a Tested card and a regressions block when a baseline exists. Scan and import local skills, install new skills, or generate a skill. An empty library omits the activity list.

## Library

Open `/my-skills`. Search, filter by enabled or available, and filter by source and tag. Cards show Draft, Tested, and Regressed badges. Open a card for the local, diff, and source tabs. The overflow menu checks, updates, edits, runs tests, generates a variant, or deletes. Select mode batches enable, disable, sync, tags, update, and delete. Lists longer than 100 cards stay windowed so a 1,000-skill library does not render every row at once.

## Install Skills

Open `/install`. Three tabs:

- Leaderboard searches the marketplace.
- Local install selects a folder or archive, scans tool directories, and imports.
- Git install clones a repository, previews skills, and imports the ones you keep.

For workflow D.2, use Local install, choose the skill folder, and confirm it appears in the Library without a Tested badge.

## Global Workspace

Open `/global-workspace`. Each detected agent is a card. Preset pills apply a preset to every agent. Open an agent to see in-sync, local-only, and center-changed skills, then pull, upload, or remove.

## Lobster Workspace

Open `/lobster-workspace` when a lobster-class assistant is installed. The layout matches the global workspace. The empty state names OpenClaw and Hermes Agent.

## Project Workspace

Open a linked project from the sidebar. Compare the project's agent folders with the library, enable or disable skills, and update the center or the project. Add skill remembers the agents you used last time.

## Backup

Open `/backup`. Connect a remote, back up, and restore a snapshot. The history keeps a safety tag on restore. Needs attention lists skills that changed on two devices; v1.0 shows that state and leaves the skill-aware merge for a later release. The device-flow code is selectable.

## Settings

Open `/settings`. Tabs are Agents, General, Appearance, Keyboard, Endpoints, Backup, and About.

- Agents: paths, enable, reorder, and custom agents.
- General: central repository path, sync mode, close behaviour, tray, and proxy. A proxy must be an `http` or `https` URL. The updater receives that proxy when you install an update.
- Appearance: light, dark, system, text size, and reduce motion.
- Keyboard: the editor shortcut table.
- Endpoints: provider keys stay in the keyring or the labelled disk fallback.
- Backup: remote URL and the libgit2 note.
- About: version, Check for Updates, diagnostics, logs, and acknowledgements.

## Agents

Open `/agents`. Import or create an agent, then open its editor. Badges show tested and draft state.

## Skill editor

Open a skill and choose Edit. The editor shows frontmatter, the body, problems, preview, and versions. Save, snapshot, and publish. Portable and Claude Code profiles change which findings appear.

## Agent editor

Open an agent and choose Edit. Fields use the documented names. Export writes the agent into a project or copies the `--agents` JSON.

## Endpoints

Settings, Endpoints tab, or `/endpoints`. Add an endpoint, probe it, and choose grader defaults. Replay mode needs no network.

## Test suite

Open a skill's test suite. Draft cases, expectations, and a trigger set, or import `evals.json`. Try a check against the fixture before a full run.

## Run results

Open the latest run. The summary shows with-skill and without-skill rates. The heatmap and transcript stay inside the page. A long transcript is windowed so 5,000 steps do not render at once. Leave feedback, then improve from results.

## Generate wizard

Choose Generate skill or Generate agent. The steps are goal, interview, drafting, review, and save. The prompt inspector copies the prompt. Escape asks before discarding answers.

## Compare

Select two cells. The verdict stays blinded until the comparison finishes. Apply through revision opens the wizard with the suggestion.

## History

Pin a baseline per endpoint and model. Regression rules R1 to R6 raise alerts. The Library and Dashboard show the same badges.

## Tray

The tray lists presets and Quit. Closing the window can minimize to the tray. That choice is remembered from the close dialog.

## Updates

Check for Updates only tells you whether a version is available. It does not install.

- On macOS and Windows, Install Update downloads the signed update and leaves a Restart Now control until you click it.
- On Linux, Download opens the release that contains the AppImage. That AppImage is the Linux update channel.
- An active test run blocks install.

Startup shows a notification toast when an update was already detected. The toast does not install by itself.

## CLI

`skill-hive-cli` uses the same library as the window. See the [CLI reference](cli/reference.md). `skill-hive-cli --json` prints a JSON envelope. Global flags are `--json` and `--skills-root`.

## Workflow D.2

1. On Install Skills, choose Local install and select the existing skill folder. The Library card has no Tested badge.
2. Open the skill. The detail panel's tests start empty. The editor's problems list shows profile findings, including a Claude Code-only field when one is present.
3. Draft the suite from the skill. Remove a case you cannot feed and add a real fixture file.
4. Run the suite in replay mode so the run costs nothing. Open Run results and read the with-skill and without-skill rates.
5. Pin the run as the baseline on History.
6. Return to the Library. The card shows Tested when the run passes, or Regressed when a later run trips a regression rule.

## Acknowledgements

Portions of Skill Hive are adapted from skills-manager by Tianliang Zhang and are used under the MIT License. Copyright (c) 2026 Tianliang Zhang. The About screen repeats this notice. Skill Hive does not reuse that project's name, identifier, icons, or updater key.
