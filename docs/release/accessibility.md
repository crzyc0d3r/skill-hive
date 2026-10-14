# Accessibility audit

Checklist from the product plan, section 6.1. This pass used the automated suite on Linux. VoiceOver, Narrator, and Orca still need a person on each operating system; the rows below say what the suite already locks.

| Item | Evidence |
|---|---|
| Text contrast 4.5:1 | `npm run contrast` |
| Reduce motion | Settings Appearance sets `data-reduce-motion`. `src/test/phase1.test.tsx` |
| Help dialog names and eight cards | `src/test/phase2.test.tsx`. Cards are links to the user guide |
| Status messages | Update notes use `role="status"`. Crash and repository warnings use `role="alert"` |
| Focus and names on settings controls | Testing Library queries by role and name in the phase 1 and phase 6 tests |
| No information by color alone | Tested and Regressed badges include words |
| Target size | Buttons use border padding at or above 24px in the shell |
| Keyboard alternative for drag | Alt and the menus remain on list reordering from earlier phases |

Screen-reader sessions on VoiceOver, Narrator, and Orca are the remaining manual pass. No release blocker was filed from the automated rows.
