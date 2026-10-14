import { describe, expect, it } from "vitest";
import {
  descriptionBudget,
  exportHistory,
  hideVersion,
  instantiateTemplate,
  planPublish,
  renderAgentsCliJson,
  renderLoaded,
  resolveInside,
  restoreVersion,
  saveAutosave,
  templateDraft,
  validateAgentText,
  validateSkill,
  warningsFixture,
} from "./model";

describe("authoring model", () => {
  it("gives every built-in template zero errors in both profiles", () => {
    for (const id of ["blank", "workflow", "reference", "script", "variant", "document"]) {
      for (const profile of ["portable", "claude-code"] as const) {
        const draft = templateDraft(id, "sample-skill");
        draft.profile = profile;
        expect(validateSkill(draft).filter((item) => item.severity === "error")).toEqual([]);
      }
      expect(Object.keys(instantiateTemplate(id, "sample-skill"))).toContain("SKILL.md");
    }
  });

  it("flags argument-hint on Portable and accepts it on Claude Code", () => {
    const draft = templateDraft("blank", "ui-styling");
    const skill = draft.files[0];
    skill.content = skill.content.replace("description:", "argument-hint: [file]\ndescription:");
    draft.profile = "portable";
    expect(validateSkill(draft).some((item) => item.ruleId === "S-23" && item.severity === "error")).toBe(true);
    draft.profile = "claude-code";
    expect(validateSkill(draft).filter((item) => item.severity === "error")).toEqual([]);
  });

  it("counts two errors and three warnings on the warnings fixture", () => {
    const findings = validateSkill(warningsFixture());
    expect(findings.filter((item) => item.severity === "error")).toHaveLength(2);
    expect(findings.filter((item) => item.severity === "warning")).toHaveLength(3);
    expect(findings.map((item) => item.ruleId)).toEqual(expect.arrayContaining(["S-18", "S-23", "S-30", "S-32", "S-33"]));
  });

  it("hides shell buttons until a click is allowed", () => {
    const draft = templateDraft("blank", "preview-skill");
    draft.dir = "/skills/preview-skill";
    draft.files[0].content += "Work in ${CLAUDE_SKILL_DIR}.\n!`echo hi`\n";
    const hidden = renderLoaded(draft);
    expect(hidden.text).toContain("/skills/preview-skill");
    expect(hidden.showRun).toBe(false);
    expect(renderLoaded({ ...draft, shellPreview: true }).showRun).toBe(true);
  });

  it("confirms symlinks and marks copies center changed", () => {
    const symlink = templateDraft("blank", "symlink-skill");
    symlink.deployments = [{ agent: "Claude Code", mode: "symlink" }];
    expect(planPublish(symlink, false)).toEqual({ kind: "confirm", agents: ["Claude Code"] });
    const copy = templateDraft("blank", "copy-skill");
    copy.deployments = [{ agent: "Claude Code", mode: "copy" }];
    expect(planPublish(copy, false)).toEqual({ kind: "published", centerChanged: ["Claude Code"] });
  });

  it("rejects a path that leaves the skill directory", () => {
    expect(resolveInside("/skills/demo", "../etc/passwd").ok).toBe(false);
    expect(resolveInside("/skills/demo", "scripts/run.py").ok).toBe(true);
  });

  it("deduplicates saves, refuses a pinned hide, restores without losing the draft, and exports one current best", () => {
    const first = { "SKILL.md": "one" };
    const second = { "SKILL.md": "two" };
    let versions = saveAutosave([], first);
    versions = saveAutosave(versions, first);
    expect(versions).toHaveLength(1);
    versions = saveAutosave(versions, second);
    expect(versions).toHaveLength(2);
    expect(versions[0].parentId).toBe(versions[1].id);
    versions[1].pinnedRunIds = ["run-9"];
    const hidden = hideVersion(versions, versions[1].id);
    expect(hidden.error).toContain("invalid_input");
    expect(hidden.error).toContain("run-9");
    const restored = restoreVersion(versions, versions[1].id, second);
    expect(restored.files["SKILL.md"]).toBe("one");
    expect(restored.versions.some((version) => version.hash.includes("two") || version.files["SKILL.md"] === "two")).toBe(true);
    for (const version of versions) version.isCurrentBest = false;
    versions[0].isCurrentBest = true;
    versions[0].gradingResult = "won";
    const history = exportHistory("sample-skill", versions);
    expect(history.iterations.filter((item) => item.is_current_best)).toHaveLength(1);
    expect(history.current_best).toBe(history.iterations.find((item) => item.is_current_best)?.version);
    expect(["baseline", "won", "lost", "tie", null]).toContain(history.iterations[0].grading_result);
  });

  it("renames a near-miss agent key in the JSON the CLI would receive", () => {
    const text = "---\nname: reviewer\ndescription: Reviews code.\ntools: Read\nmodel: sonnet\n---\nYou review.\n";
    const flagged = `---\nname: reviewer\ndescription: Reviews code.\ndisallowed_tools: Bash\n---\nYou review.\n`;
    expect(validateAgentText(flagged, false).some((item) => item.ruleId === "A-05")).toBe(true);
    const json = renderAgentsCliJson(text);
    expect(json.reviewer.prompt).toContain("You review.");
    expect(json.reviewer.model).toBe("sonnet");
    expect(descriptionBudget(["x".repeat(60004)]).amber).toBe(true);
  });
});
