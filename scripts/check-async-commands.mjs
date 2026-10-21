#!/usr/bin/env node
/**
 * Fail when a synchronous Tauri command body touches disk, git, the database,
 * the network, or a process. Those commands must be `async` and offload the work.
 */
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const root = new URL("../src-tauri/src/", import.meta.url);
const files = readdirSync(root).filter((name) => name.endsWith(".rs"));
const markers = [
  "std::fs",
  "fs::",
  "read_to_string",
  "read_dir",
  "OpenOptions",
  "remove_file",
  "remove_dir",
  "create_dir",
  "Command::",
  "std::process",
  "store.as_ref()",
  "store.inner()",
  "SkillStore",
  "central_repo",
  "installer::",
  "scanner::",
  "fetch_https",
  "reqwest",
  "rusqlite",
  "git_backup::",
  "updater::",
  "hive_core::testing",
  "write_skill_file",
  "apply_fix",
  "clear_provider_secret",
  "begin_device_flow",
  "export_logs_zip",
  "secrets_dir",
];

function commands(text) {
  const found = [];
  const attr = /#\[tauri::command[^\]]*\]\n([\s\S]*?\n\{)/g;
  let match;
  while ((match = attr.exec(text))) {
    const header = match[1];
    const braceAt = match.index + match[0].length - 1;
    let depth = 0;
    let end = braceAt;
    for (let i = braceAt; i < text.length; i += 1) {
      if (text[i] === "{") depth += 1;
      else if (text[i] === "}") {
        depth -= 1;
        if (depth === 0) {
          end = i;
          break;
        }
      }
    }
    const name = header.match(/\bfn\s+([A-Za-z0-9_]+)/)?.[1] ?? "unknown";
    const asyncFn = /\basync\s+fn\b/.test(header);
    const body = text.slice(braceAt, end + 1);
    found.push({ name, asyncFn, body });
    attr.lastIndex = end + 1;
  }
  return found;
}

const failures = [];
for (const file of files) {
  const text = readFileSync(join(root.pathname, file), "utf8");
  for (const command of commands(text)) {
    if (command.asyncFn) continue;
    const hit = markers.find((marker) => command.body.includes(marker));
    if (hit) failures.push(`${file} ${command.name} is synchronous and uses ${hit}`);
  }
}

if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log(`async command check passed (${files.length} rust files)`);
