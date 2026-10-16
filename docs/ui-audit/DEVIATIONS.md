# UI audit

Before shots are the frontend that shipped in the debug bundle (`dist/`, built before this UI work), captured with demo data in light and dark at 1100x640 and 1440x860. After shots are the same routes after the screen rebuild. Both sets live under `docs/ui-audit/before` and `docs/ui-audit/after`.

The spec is PLAN.md Part 4 (tokens, shadcn new-york, Tailwind v4, OKLCH light and dark, JetBrains Mono and IBM Plex Sans, 28 px rows, 32 px controls, 6 px radius). Where a screen's layout is not drawn there, the shell still uses the sidebar, 44 px header, 24 px status bar, and a centered content column.

## What was wrong on every screen

- Controls were native `<button>` and `<input>` elements with a one-pixel border and default browser type, not shadcn `Button`, `Input`, `Tabs`, `Card`, `Dialog`, `Table`, or `Alert`.
- Type sat at the left edge of a mostly empty canvas. There was no stat grid, card grid, or panel hierarchy, so 1100 px and 1440 px windows looked the same: a short stack and a blank field.
- Headings used the mono face, but body, metadata, and buttons did not separate title, lede, and action. Secondary text was the same size and weight as actions.
- Nothing showed hover, focus, or selected state beyond the browser outline. Selected tabs, filters, and theme choices stayed outlined even when active.
- Empty, loading, and error states were a single line of text, or missing. Disabled actions did not use the button disabled treatment.
- Icons were absent. Status was color or a bare word.
- Spacing was ad hoc (`mt-2`, `ml-2` on inline controls) instead of the 8/16 px grid.
- Page content could sit in a narrow left column with no max width rhythm, and long labels risked a horizontal page scrollbar. Wide tables and diffs did not own their own scroller.

## Screen by screen

| Screen | Before | Spec |
|---|---|---|
| Shell | Flat page, text nav, no icon rail, no pinned settings treatment, command palette was a plain dialog list | Sidebar 240/56, header 44, status bar 24, command palette, help, close guard, toasts |
| Dashboard | Four bordered boxes, text links, a regressions line, then empty space | Four stat cards, primary Install, scan and generate, recent activity and regressions |
| Library | A search field and filter words, then a vertical list of names | Card or list grid, source and tag filters, batch bar, detail panel, draft/tested/regressed badges |
| Install | Four text tabs and stacked fields | Tab bar with a selected tab, folder and archive fields, scan rows, git preview, marketplace cards |
| Workspaces | A title and a few bordered buttons | Preset bar, agent cards, agent page with search, layout toggle, add sheet, confirm |
| Lobster | Title and one bordered button | Same shell as workspace, with the lobster empty copy |
| Project | Title and a link button | Skill row, agent dots, local/diff/center, batch actions |
| Backup | A column of bordered buttons and inline inputs | Status and scope, primary Back up now, history with Restore, first-run remote form |
| Settings | Equal outlined tabs, theme as bordered buttons, agents as a flat list | Selected tab, theme and text-size controls, agent rows, shortcut table, about actions |
| Agents | Title, one field, empty | Import field, budget line, agent cards, collision dialog |
| Skill editor | Three cramped columns of bordered buttons and a textarea | File tree, description, body, problems/preview/versions with a selected tab |
| Agent editor | A stack of buttons and a textarea | Save and export, tabbed problems and preview |
| Generate | A short form | Stepper, goal field, continue, review diff, discard dialog |
| Test suite | Three lines and two buttons | Title, with/without skill, optimize, run |
| Results | A small grid of bordered cells | Heatmap grid, timeline, transcript, evidence, budget dialog |
| History | A plain HTML table | Table with a baseline action |
| Compare | Two bordered boxes | Side-by-side cards and a verdict action |
| Endpoints | A word list | Stepper, endpoint rows, secret field, export |
| Command palette | A search box over the empty page | Grouped command dialog, empty-state copy, Escape to close |

## What this pass changes

Feature views now use `Button` and `Input` (checkboxes stay native so their accessible names stay stable), `Card` on the library, `Table` on history and the keyboard tab, `Screen` for the page title and lede, and a selected-tab treatment (`default` vs `outline`) on settings, install, and the editors. The shell already uses `Sidebar`, `Dialog`, `Command`, and `sonner`. Badge variants cover success, warning, and info. Reduced motion shortens transitions. The visual suite checks both themes at both window sizes for horizontal overflow and heading contrast.
