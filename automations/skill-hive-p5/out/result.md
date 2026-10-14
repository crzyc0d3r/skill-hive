# Phase 5 result

Product: Skill Hive.

The split is graph-informed. graphify extract --code-only wrote graphify-out/graph.json (graphifyy 0.9.90), then graphify update and cluster-only refreshed it to 308 communities. n1_core takes the Rust communities that compile into one hive-core or hive-cli test binary: 12 (generation session), 137 (envelope), 7 (analytics workflow), 118 (optimize), 131 (compare), 70 (regression), 95 (export), 51 (schema v12), 90 (cli phase5), and 107 (replay benchmark). n2_screens takes the UI communities: 14 (wizard, compare, suite), 43 (App, dashboard, history), 52 (library badges), 32 (shell), 3 (vitest), and 139 (playwright). Shadcn communities were not given nodes.

Criterion 1: pass. B.3 criteria 1 to 8 and C.7 criteria 10 to 15 pass as hive-core tests on recorded fixtures. Nine of ten replayed goal sessions reach Review with zero validation errors; the tenth surfaces findings and accept refuses to save. Limit: no vendor API key is configured, so the nine-of-ten measurement is the frozen replay fixture, not a live endpoint.
Criterion 2: pass. Workflows D.1, D.2, and D.3 run end to end in analytics::tests on the fixture skill and a generated agent, using recorded replies.
Criterion 3: pass. The exported skill-creator workspace opens with fixtures/eval-viewer/generate_review.py --static. JUnit testcase counts and CSV row counts match the cells, expectations, and checks.
Criterion 4: pass. The Library shows Tested and Regressed badges and the Dashboard shows the Tested stat and the regressions block from the regression report produced by rules R1 to R6.

Gate summary:
core exit 0
frontend npm test 0

auto-approved by --auto
graphify was installed and the split followed graphify-out/graph.json.
