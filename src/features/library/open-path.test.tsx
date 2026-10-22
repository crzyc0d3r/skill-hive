import { Profiler, useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter } from "react-router-dom";
import { expect, it } from "vitest";
import { LibraryView } from "@/features/library/LibraryView";
import { groupSkillCards, type SkillCard } from "@/features/library/model";
import { writeLibraryIndex } from "@/lib/library-cache";
import { installIpc, ipcLog, renderAt, SKILLS } from "@/test/render-app";

function crowd(count: number): SkillCard[] {
  return Array.from({ length: count }, (_, index) => ({
    ...SKILLS[0],
    id: `skill-${index}`,
    name: `skill-${index}`,
    description: "Keep a long task moving across the library index.",
    tags: [],
    agents: [],
    body: "",
    sourceBody: "",
    files: [],
  }));
}

const noop = () => undefined;

it("paints a cached library on revisit without scanning again", async () => {
  const skills = crowd(500);
  installIpc(skills);
  writeLibraryIndex(skills);
  const user = userEvent.setup();
  renderAt("/");
  await screen.findByTestId("dashboard");
  await waitFor(() => expect(ipcLog.some((entry) => entry.cmd === "check_all_skill_updates")).toBe(true));
  await waitFor(() => expect(ipcLog.some((entry) => entry.cmd === "get_managed_skills")).toBe(true));
  const checksBefore = ipcLog.filter((entry) => entry.cmd === "check_all_skill_updates").length;
  const fetchesBefore = ipcLog.filter((entry) => entry.cmd === "get_managed_skills").length;

  await user.click(screen.getByRole("link", { name: "Library" }));
  await screen.findByTestId("library-cards");
  expect(document.querySelector("[data-testid='library-cards']")?.textContent ?? "").toContain("skill-0");
  await user.click(screen.getByRole("link", { name: "Dashboard" }));
  await screen.findByTestId("dashboard");
  const toolFetchesBefore = ipcLog.filter((entry) => entry.cmd === "get_tool_status").length;
  const library = screen.getByRole("link", { name: "Library" });
  const revisitStarted = performance.now();
  await act(async () => {
    fireEvent.click(library);
  });
  const cardsNode = document.querySelector("[data-testid='library-cards']");
  expect(cardsNode?.textContent ?? "").toContain("skill-0");
  const revisitMs = performance.now() - revisitStarted;
  const cards = cardsNode?.children.length ?? 0;

  const report = `library revisit: cached revisit ${revisitMs.toFixed(1)} ms, cards painted ${cards}, check_all ${ipcLog.filter((entry) => entry.cmd === "check_all_skill_updates").length}, get_managed_skills ${ipcLog.filter((entry) => entry.cmd === "get_managed_skills").length}, get_tool_status ${ipcLog.filter((entry) => entry.cmd === "get_tool_status").length}`;
  console.info(report);
  // This machine paints the cached click in about 92 ms. The same click in
  // GitHub's jsdom job measured 239 ms with no extra scan, so CI allows 500 ms.
  // A multi-second rescan still fails this budget.
  const budget = process.env.CI ? 500 : 200;
  expect(revisitMs, report).toBeLessThan(budget);
  expect(ipcLog.filter((entry) => entry.cmd === "check_all_skill_updates").length).toBe(checksBefore);
  expect(ipcLog.filter((entry) => entry.cmd === "get_managed_skills").length).toBe(fetchesBefore);
  expect(ipcLog.filter((entry) => entry.cmd === "get_tool_status").length).toBe(toolFetchesBefore);
  expect(cards).toBeGreaterThan(0);
  expect(cards).toBeLessThanOrEqual(24);
});

it("groups and paints 500 skills in one window without a measurement storm", async () => {
  const skills = crowd(500);
  const groupStarted = performance.now();
  const grouped = groupSkillCards(skills);
  const groupMs = performance.now() - groupStarted;
  expect(grouped).toHaveLength(500);

  let commits = 0;
  let renderMs = 0;
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  function Harness() {
    const [tick, setTick] = useState(0);
    return (
      <Profiler
        id="library"
        onRender={(_id, _phase, actual) => {
          commits += 1;
          renderMs += actual;
        }}
      >
        <button type="button" onClick={() => setTick((value) => value + 1)}>
          tick {tick}
        </button>
        <LibraryView
          skills={grouped}
          onRenameTag={noop}
          onAddTag={noop}
          onSync={noop}
          onToggleMember={noop}
          onReorderMembers={noop}
          onDeleteSkills={noop}
          onDeleteTag={noop}
          onRemoveTag={noop}
          onSetEnabled={noop}
          onBatchTag={noop}
          onToggleAgent={noop}
        />
      </Profiler>
    );
  }
  const started = performance.now();
  render(
    <QueryClientProvider client={client}>
      <MemoryRouter>
        <Harness />
      </MemoryRouter>
    </QueryClientProvider>,
  );
  const sample = document.querySelector("[data-testid='library-cards']");
  expect(sample?.textContent ?? "").toContain("skill-0");
  const paintMs = performance.now() - started;
  const painted = sample?.children.length ?? 0;
  const mountCommits = commits;
  const mountRender = renderMs;
  await act(async () => {
    screen.getByRole("button", { name: /tick/ }).click();
  });
  const report = `library paint: group ${groupMs.toFixed(1)} ms, first cards ${paintMs.toFixed(1)} ms, mount commits ${mountCommits}, render ${mountRender.toFixed(1)} ms, cards ${painted}`;
  console.info(report);
  expect(paintMs, report).toBeLessThan(200);
  expect(mountRender, report).toBeLessThan(200);
  expect(painted).toBeLessThanOrEqual(24);
  expect(mountCommits).toBeLessThanOrEqual(3);
});
