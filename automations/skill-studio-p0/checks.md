# checks: skill-studio-p0

Run `r1` · updated 2026-10-13T19:11:31+00:00 · verdicts: pass, fail, unresolved

## Node gates

| node | requirement | verdict | evidence |
|---|---|---|---|
| n1_inventory | a1 check.cmd | pass | `python3 -c "import json; p=json.load(open('automations/skill-studio-p0/out/n1_inventory.json')); assert p"` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n1_inventory.a1.md` |
| n1_inventory | a1 node.report | pass | exit 0: wrote automations/skill-studio-p0/out/n1_inventory.json · log `automations/skill-studio-p0/runs/r1/gates/n1_inventory.a1.md` |
| n2_port_util | a1 check.cmd | pass | `grep -q 'ported util' automations/skill-studio-p0/out/n2_port_util.md` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n2_port_util.a1.md` |
| n2_port_util | a1 node.report | pass | exit 0: wrote crates/studio-core/src/util/error.rs<br>wrote crates/studio-core/src/util/path_guard.rs<br>wrote crates/studio-core/src/util/crypto.rs<br>wrote crates/studio-core/src/util/log_sanitize.rs<br>wrote crates/studio-core/src/util/panic_log.rs<br>wrote crates/studio-core/src/util/timing.rs<… · log `automations/skill-studio-p0/runs/r1/gates/n2_port_util.a1.md` |
| n3_port_db | a1 check.cmd | pass | `grep -q 'studio.db' automations/skill-studio-p0/out/n3_port_db.md` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n3_port_db.a1.md` |
| n3_port_db | a1 node.report | pass | exit 0: wrote crates/studio-core/src/db/migrations.rs<br>wrote crates/studio-core/src/db/skill_store.rs<br>wrote crates/studio-core/src/db/mod.rs<br>wrote automations/skill-studio-p0/out/n3_port_db.md · log `automations/skill-studio-p0/runs/r1/gates/n3_port_db.a1.md` |
| n4_port_repo | a1 check.cmd | pass | `grep -q '.skill-studio' automations/skill-studio-p0/out/n4_port_repo.md` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n4_port_repo.a1.md` |
| n4_port_repo | a1 node.report | pass | exit 0: wrote crates/studio-core/src/library/central_repo.rs<br>wrote crates/studio-core/src/util/repo_lock.rs<br>wrote crates/studio-core/src/util/cli_bridge.rs<br>wrote crates/studio-core/src/library/sync_metadata.rs<br>wrote crates/studio-core/src/util/mod.rs<br>wrote crates/studio-core/src/libr… · log `automations/skill-studio-p0/runs/r1/gates/n4_port_repo.a1.md` |
| n5_port_watch | a1 check.cmd | pass | `grep -q 'app-files-changed' crates/studio-core/src/util/file_watcher.rs && grep -q Notifier automations/skill-studio-p0/out/n5_port_watch.md` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n5_port_watch.a1.md` |
| n5_port_watch | a1 node.report | pass | exit 0: wrote crates/studio-core/src/deploy/tool_adapters.rs<br>wrote crates/studio-core/src/util/file_watcher.rs<br>wrote crates/studio-core/src/deploy/mod.rs<br>wrote crates/studio-core/src/util/mod.rs<br>wrote crates/studio-core/src/lib.rs<br>wrote automations/skill-studio-p0/out/n5_port_watch.md · log `automations/skill-studio-p0/runs/r1/gates/n5_port_watch.a1.md` |
| n6_shell | a1 check.cmd | pass | `grep -q 'command palette' automations/skill-studio-p0/out/n6_shell.md` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n6_shell.a1.md` |
| n6_shell | a1 node.report | pass | exit 0: /workspace/automations/skill-studio-p0/out/n6_shell.md · log `automations/skill-studio-p0/runs/r1/gates/n6_shell.a1.md` |
| n7_ci | a1 check.cmd | pass | `grep -q 'release workflow' automations/skill-studio-p0/out/n7_ci.md` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n7_ci.a1.md` |
| n7_ci | a1 node.report | pass | exit 0: /workspace/automations/skill-studio-p0/out/n7_ci.md · log `automations/skill-studio-p0/runs/r1/gates/n7_ci.a1.md` |
| n8_verify | a1 check.cmd | pass | `grep -q 'schema v8' automations/skill-studio-p0/out/result.md && grep -q '\| cargo test \| 0 \|' automations/skill-studio-p0/out/result.md && grep -q '\| npm test \| 0 \|' automations/skill-studio-p0/out/result.md` → exit 0: (no output) · log `automations/skill-studio-p0/runs/r1/gates/n8_verify.a1.md` |
| n8_verify | a1 node.report | pass | exit 0: /workspace/automations/skill-studio-p0/out/result.md · log `automations/skill-studio-p0/runs/r1/gates/n8_verify.a1.md` |

## Acceptance checks (the deliverable)

| check | verdict | evidence |
|---|---|---|
| 0. result records schema v8 and local test exits | pass | `grep -q "schema v8" automations/skill-studio-p0/out/result.md && grep -q "\| cargo test \| 0 \|" automations/skill-studio-p0/out/result.md && grep -q "\| npm test \| 0 \|" automations/skill-studio-p0/out/result.md` → exit 0: (no output) |
| 1. MIT attribution file exists | pass | `test -s THIRD_PARTY_LICENSES/skills-manager.MIT.txt` → exit 0: (no output) |
| 2. release workflow lists the arm runner | pass | `grep -q ubuntu-24.04-arm .github/workflows/release.yml` → exit 0: (no output) |
| 3. product identifier is com.skillstudio.desktop | pass | `grep -q com.skillstudio.desktop src-tauri/tauri.conf.json` → exit 0: (no output) |
| 4. contrast helper passes | pass | `node scripts/contrast.mjs` → exit 0: pass light foreground/background: 17.85:1 (min 4.5:1)<br>pass light muted-foreground/background: 7.58:1 (min 4.5:1)<br>pass light primary/primary-foreground: 13.98:1 (min 4.5:1)<br>pass light success text: 5.02:1 (min 4.5:1)<br>pass light warning text: 5.02:1 (… |
| 5. derived-file headers pass | pass | `node scripts/check-headers.mjs` → exit 0: crates/studio-core/src/deploy/mod.rs<br>crates/studio-core/src/deploy/tool_adapters.rs<br>crates/studio-core/src/library/central_repo.rs<br>crates/studio-core/src/library/mod.rs<br>crates/studio-core/src/library/sync_metadata.rs<br>5 derived files carry th… |

## Baseline before the run (2026-10-13T19:10:58+00:00)

| item | verdict | evidence |
|---|---|---|
| deliverable exists before the run | fail | automations/skill-studio-p0/out/result.md absent |
| acceptance 0: result records schema v8 and local test exits | fail | `grep -q "schema v8" automations/skill-studio-p0/out/result.md && grep -q "\| cargo test \| 0 \|" automations/skill-studio-p0/out/result.md && grep -q "\| npm test \| 0 \|" automations/skill-studio-p0/out/result.md` → exit 2: [stderr]<br>grep: automations/skill-studio-p0/out/result.md: No such file… |
| acceptance 1: MIT attribution file exists | pass | `test -s THIRD_PARTY_LICENSES/skills-manager.MIT.txt` → exit 0: (no output) |
| acceptance 2: release workflow lists the arm runner | pass | `grep -q ubuntu-24.04-arm .github/workflows/release.yml` → exit 0: (no output) |
| acceptance 3: product identifier is com.skillstudio.desktop | pass | `grep -q com.skillstudio.desktop src-tauri/tauri.conf.json` → exit 0: (no output) |
| acceptance 4: contrast helper passes | pass | `node scripts/contrast.mjs` → exit 0: pass light foreground/background: 17.85:1 (min 4.5:1)<br>pass light muted-foreground/background: 7.58:1 (min 4.5:1)<br>pass light primary/primary-foreground: 13.98:1 (min 4.5:1)<br>pass light success text: 5.02:1 (min 4.5:1)<br>pass light warning text: 5.02:1 (… |
| acceptance 5: derived-file headers pass | pass | `node scripts/check-headers.mjs` → exit 0: crates/studio-core/src/deploy/mod.rs<br>crates/studio-core/src/deploy/tool_adapters.rs<br>crates/studio-core/src/library/central_repo.rs<br>crates/studio-core/src/library/mod.rs<br>crates/studio-core/src/library/sync_metadata.rs<br>5 derived files carry th… |

Observed before the run:
- studio-core tests were already green on this machine before the graph run

## Completion (2026-10-13T19:11:31+00:00) — COMPLETE

| item | verdict | evidence |
|---|---|---|
| graph.md is a valid spec | pass | validate ok |
| spec approved before execution | pass | hash 9f911c6dd5af7623 |
| n1_inventory: output exists and can be opened | pass | automations/skill-studio-p0/out/n1_inventory.json (639 bytes) |
| n1_inventory: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:03+00:00 |
| n2_port_util: output exists and can be opened | pass | automations/skill-studio-p0/out/n2_port_util.md (101 bytes) |
| n2_port_util: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:03+00:00 |
| n3_port_db: output exists and can be opened | pass | automations/skill-studio-p0/out/n3_port_db.md (98 bytes) |
| n3_port_db: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:03+00:00 |
| n4_port_repo: output exists and can be opened | pass | automations/skill-studio-p0/out/n4_port_repo.md (101 bytes) |
| n4_port_repo: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:03+00:00 |
| n5_port_watch: output exists and can be opened | pass | automations/skill-studio-p0/out/n5_port_watch.md (109 bytes) |
| n5_port_watch: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:03+00:00 |
| n6_shell: output exists and can be opened | pass | automations/skill-studio-p0/out/n6_shell.md (219 bytes) |
| n6_shell: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:03+00:00 |
| n7_ci: output exists and can be opened | pass | automations/skill-studio-p0/out/n7_ci.md (155 bytes) |
| n7_ci: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:03+00:00 |
| n8_verify: output exists and can be opened | pass | automations/skill-studio-p0/out/result.md (3057 bytes) |
| n8_verify: gate ran against the saved version | pass | gate pass at 2026-10-13T19:11:20+00:00 |
| graph.md matches the nodes that actually ran | pass | ran 8 of 8 declared nodes |
| acceptance 0: result records schema v8 and local test exits | pass | `grep -q "schema v8" automations/skill-studio-p0/out/result.md && grep -q "\| cargo test \| 0 \|" automations/skill-studio-p0/out/result.md && grep -q "\| npm test \| 0 \|" automations/skill-studio-p0/out/result.md` → exit 0: (no output) |
| acceptance 1: MIT attribution file exists | pass | `test -s THIRD_PARTY_LICENSES/skills-manager.MIT.txt` → exit 0: (no output) |
| acceptance 2: release workflow lists the arm runner | pass | `grep -q ubuntu-24.04-arm .github/workflows/release.yml` → exit 0: (no output) |
| acceptance 3: product identifier is com.skillstudio.desktop | pass | `grep -q com.skillstudio.desktop src-tauri/tauri.conf.json` → exit 0: (no output) |
| acceptance 4: contrast helper passes | pass | `node scripts/contrast.mjs` → exit 0: pass light foreground/background: 17.85:1 (min 4.5:1)<br>pass light muted-foreground/background: 7.58:1 (min 4.5:1)<br>pass light primary/primary-foreground: 13.98:1 (min 4.5:1)<br>pass light success text: 5.02:1 (min 4.5:1)<br>pass light warning text: 5.02:1 (… |
| acceptance 5: derived-file headers pass | pass | `node scripts/check-headers.mjs` → exit 0: crates/studio-core/src/deploy/mod.rs<br>crates/studio-core/src/deploy/tool_adapters.rs<br>crates/studio-core/src/library/central_repo.rs<br>crates/studio-core/src/library/mod.rs<br>crates/studio-core/src/library/sync_metadata.rs<br>5 derived files carry th… |
| deliverable exists | pass | automations/skill-studio-p0/out/result.md |
| run limit respected | pass | 0/3 model dispatches |
