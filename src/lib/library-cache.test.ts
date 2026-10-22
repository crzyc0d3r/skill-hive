import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { clearLibraryMemory, readLibraryIndex, slimSkill, writeLibraryIndex } from "@/lib/library-cache";
import type { SkillCard } from "@/features/library/model";

function skill(index: number, description: string): SkillCard {
  return {
    id: `skill-${index}`,
    name: `skill-${index}`,
    description,
    source: "local",
    tags: [],
    enabled: true,
    body: description,
    sourceBody: description,
    agents: ["claude_code"],
  };
}

function readMarkdown(files: string[]): { name: string; description: string }[] {
  return files.map((text) => {
    const description = text.match(/description:\s*(.*)/)?.[1] ?? "";
    const name = text.match(/name:\s*(.*)/)?.[1] ?? "";
    return { name, description };
  });
}

describe("library index", () => {
  it("serves 500 skills from memory faster than re-reading each file", () => {
    localStorage.clear();
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "hive-library-"));
    const body = `${"A long skill body that used to be copied onto every card. ".repeat(40)}\n`;
    const files = Array.from({ length: 500 }, (_, index) => {
      const folder = path.join(dir, `skill-${index}`);
      fs.mkdirSync(folder);
      const text = `---\nname: skill-${index}\ndescription: ${body}\n---\n${body}`;
      fs.writeFileSync(path.join(folder, "SKILL.md"), text);
      return text;
    });
    const coldStart = performance.now();
    const parsed = readMarkdown(
      fs.readdirSync(dir).map((name) => fs.readFileSync(path.join(dir, name, "SKILL.md"), "utf8")),
    );
    const coldMs = performance.now() - coldStart;
    expect(parsed).toHaveLength(500);

    const indexed = writeLibraryIndex(files.map((text, index) => skill(index, text)));
    expect(indexed[0].description.length).toBeLessThanOrEqual(180);
    expect(indexed[0].body).toBe("");
    clearLibraryMemory();

    const diskStart = performance.now();
    const fromDisk = readLibraryIndex();
    const diskMs = performance.now() - diskStart;
    expect(fromDisk).toHaveLength(500);

    const warmStart = performance.now();
    const cached = readLibraryIndex();
    const warmMs = performance.now() - warmStart;
    expect(cached).toHaveLength(500);
    expect(diskMs).toBeLessThan(coldMs);
    expect(warmMs).toBeLessThanOrEqual(diskMs);
    console.info(
      `library index 500 skills: read SKILL.md ${coldMs.toFixed(1)} ms, disk index ${diskMs.toFixed(1)} ms, memory index ${warmMs.toFixed(1)} ms`,
    );
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("collapses whitespace before a description is stored", () => {
    expect(slimSkill(skill(1, "line one\n\nline two")).description).toBe("line one line two");
  });
});
