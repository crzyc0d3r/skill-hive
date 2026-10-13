# Skill fixtures

Phase 0 samples only. The validator that scores these files arrives in Phase 3. Nothing in `npm test` or `cargo test` parses them yet.

| Directory | Intent |
|---|---|
| `valid/` | Name and description present, body under the frontmatter, no extra keys. |
| `warnings/` | Description longer than the listing budget, an unknown frontmatter key, and a body that is only a heading. |
| `errors/` | A name with spaces, plus `missing-name/` with no `name` key. Both bodies point at a `../` path. |

Each directory contains one `SKILL.md`.
