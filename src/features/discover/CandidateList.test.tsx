import * as React from "react";
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type * as T from "@/lib/api/bindings";
import { TooltipProvider } from "@/components/ui";
import { CandidateList } from "./CandidateList";
import { defaultSelection, isImportable, toggleId } from "./candidates";

let n = 0;
function cand(name: string, status: T.CandidateStatus, targetId: string | null = "claude-code"): T.Candidate {
  n += 1;
  return {
    id: `cand-${n}`,
    kind: "skill",
    name,
    description: `${name} description`,
    path: `/home/u/.claude/skills/${name}`,
    contentHash: "abc",
    sizeBytes: 100,
    tokenEstimate: 500,
    source: { kind: "agent_dir", locator: `/home/u/.claude/skills/${name}`, targetId, fetchedAt: 0, subpath: null, reference: null, revision: null },
    validation: { findings: status === "invalid" ? [{ code: "name.missing", severity: "error", message: "`name` is required", field: "name" }] : [] },
    status,
    libraryItemId: status === "in_library" ? "item-1" : null,
  };
}

const candidates = [cand("alpha", "new"), cand("beta", "new", "codex"), cand("pdf", "in_library"), cand("broken", "invalid", null)];

function Harness({ grouped }: { grouped?: boolean }) {
  const [selected, setSelected] = React.useState(() => defaultSelection(candidates));
  return (
    <TooltipProvider>
      <CandidateList
        candidates={candidates}
        selected={selected}
        onSelectedChange={setSelected}
        groupOf={grouped ? (c) => ({ key: c.source.targetId ?? "other", label: c.source.targetId ?? "Other" }) : undefined}
      />
    </TooltipProvider>
  );
}

describe("candidate selection helpers", () => {
  it("selects every new candidate by default and never invalid ones", () => {
    expect(defaultSelection(candidates)).toEqual(["cand-1", "cand-2"]);
    expect(candidates.filter(isImportable).map((c) => c.name)).toEqual(["alpha", "beta", "pdf"]);
    expect(toggleId(["a"], "b")).toEqual(["a", "b"]);
    expect(toggleId(["a", "b"], "a")).toEqual(["b"]);
  });
});

describe("CandidateList", () => {
  it("toggles rows, skips invalid candidates and supports bulk actions", async () => {
    const user = userEvent.setup();
    render(<Harness />);
    expect(screen.getByText("2 of 3 selected")).toBeInTheDocument();
    expect(screen.getByText("2 new · 1 in library · 1 invalid")).toBeInTheDocument();

    await user.click(screen.getByLabelText("Select pdf"));
    expect(screen.getByText("3 of 3 selected")).toBeInTheDocument();

    expect(screen.getByLabelText("Select broken")).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "None" }));
    expect(screen.getByText("0 of 3 selected")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "All" }));
    expect(screen.getByText("3 of 3 selected")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "New only" }));
    expect(screen.getByText("2 of 3 selected")).toBeInTheDocument();
    expect(screen.getByLabelText("Select alpha")).toHaveAttribute("data-state", "checked");
    expect(screen.getByLabelText("Select pdf")).toHaveAttribute("data-state", "unchecked");
  });

  it("groups candidates and toggles a whole group", async () => {
    const user = userEvent.setup();
    render(<Harness grouped />);
    expect(screen.getByText("claude-code")).toBeInTheDocument();
    expect(screen.getByText("codex")).toBeInTheDocument();

    // claude-code group: alpha (selected) + pdf (not) → indeterminate; clicking selects both.
    const group = screen.getByLabelText("Select all in claude-code");
    expect(group).toHaveAttribute("data-state", "indeterminate");
    await user.click(group);
    expect(screen.getByText("3 of 3 selected")).toBeInTheDocument();
    await user.click(group);
    expect(screen.getByText("1 of 3 selected")).toBeInTheDocument();
  });
});
