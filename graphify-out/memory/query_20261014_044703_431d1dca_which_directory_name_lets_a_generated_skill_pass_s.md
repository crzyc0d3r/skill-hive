---
type: "query"
date: "2026-10-14T04:47:03.311155+00:00"
question: "Which directory name lets a generated skill pass S-14?"
contributor: "graphify"
outcome: "useful"
source_nodes: ["GenerationSession"]
---

# Q: Which directory name lets a generated skill pass S-14?

## Answer

Stage under staging/<skill-name> so the leaf matches the frontmatter name. A leading-dot directory fails directory_is_safe.

## Outcome

- Signal: useful

## Source Nodes

- GenerationSession