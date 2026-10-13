import fs from "node:fs";
import path from "node:path";

const header = [
  "// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)",
  "// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.",
  "// Copyright (c) 2026 Tianliang Zhang",
].join("\n");

const roots = [
  "crates/studio-core/src/library",
  "crates/studio-core/src/deploy",
  "crates/studio-core/src/backup",
];

function walk(dir, out) {
  if (!fs.existsSync(dir)) return;
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full, out);
    else if (entry.name.endsWith(".rs")) out.push(full);
  }
}

const files = [];
for (const root of roots) walk(root, files);
files.sort();

let failed = 0;
for (const file of files) {
  const text = fs.readFileSync(file, "utf8");
  if (!text.startsWith(header)) {
    console.error(`missing derived-file header: ${file}`);
    failed += 1;
  } else {
    console.log(file);
  }
}

if (files.length === 0) {
  console.error("no Rust files found under library/, deploy/, or backup/");
  process.exit(1);
}

if (failed > 0) {
  console.error(`${failed} file(s) missing the derived-file header`);
  process.exit(1);
}

console.log(`${files.length} derived files carry the skills-manager header`);
