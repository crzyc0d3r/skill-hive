import fs from "node:fs";
import path from "node:path";

const root = path.resolve("src");
const locales = ["en", "zh-CN", "zh-TW"];
const tables = Object.fromEntries(
  locales.map((id) => [id, JSON.parse(fs.readFileSync(`src/locales/${id}.json`, "utf8"))]),
);
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
const english = tables.en;
const keys = new Set(Object.keys(english));
const missing = [...used].filter((key) => !keys.has(key)).sort();
const unused = [...keys].filter((key) => !used.has(key)).sort();
const problems = [];
if (missing.length) problems.push(`missing English keys:\n${missing.join("\n")}`);
if (unused.length) problems.push(`unused English keys:\n${unused.join("\n")}`);

for (const id of locales) {
  const table = tables[id];
  const absent = [...keys].filter((key) => typeof table[key] !== "string" || table[key].trim() === "");
  const extra = Object.keys(table).filter((key) => !keys.has(key));
  if (absent.length) problems.push(`${id} missing or empty keys:\n${absent.join("\n")}`);
  if (extra.length) problems.push(`${id} extra keys:\n${extra.join("\n")}`);
}

if (problems.length) {
  console.error(problems.join("\n\n"));
  process.exit(1);
}
console.log(`i18n lint: ${keys.size} keys in ${locales.join(", ")}, zero missing`);
