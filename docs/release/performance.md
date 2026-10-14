# Performance

| Budget | Evidence |
|---|---|
| Library of 1,000 skills | `windowRows` keeps 100 rows. `src/test/phase6.test.tsx` and `LibraryView` |
| Transcript of 5,000 steps | The same window. `src/test/phase6.test.tsx` |
| Warm startup under 1.5 s | `warmStartWithinBudget` rejects 1500 ms and accepts 1499 ms. `log_startup_event` records the live timing |
| No horizontal page scroll at 1100x640 | Window `minWidth` 1100 and `min-w-0` on the results page |
| Editor typing | Validation stays on a debounce in the editor. This pass did not add a new editor path |

The budgets are unit-checked. A traced run on the reference laptops is still a manual measurement; the assertion the suite can make is the window and the 1.5 s predicate.
