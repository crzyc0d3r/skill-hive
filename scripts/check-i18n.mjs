import fs from "node:fs";
import path from "node:path";

const root = path.resolve("src");
const dictionary = JSON.parse(fs.readFileSync("src/locales/en.json", "utf8"));
const used = new Set();

function walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) walk(full);
    else if (/\.(tsx|ts)$/.test(entry.name)) {
      const text = fs.readFileSync(full, "utf8");
      for (const match of text.matchAll(/\bt\(\s*["']([^"']+)["']/g)) used.add(match[1]);
    }
  }
}

walk(root);
const keys = new Set(Object.keys(dictionary));
const missing = [...used].filter((key) => !keys.has(key)).sort();
const unused = [...keys].filter((key) => !used.has(key)).sort();
if (missing.length || unused.length) {
  if (missing.length) console.error(`missing English keys:\n${missing.join("\n")}`);
  if (unused.length) console.error(`unused English keys:\n${unused.join("\n")}`);
  process.exit(1);
}
console.log(`i18n lint: ${keys.size} English keys, zero missing, zero unused`);
