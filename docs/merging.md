# Merging overlapping skills

Two skills whose descriptions overlap make an agent pick between them at random, and two bodies
that say nearly the same thing cost context twice. A merge replaces them with one skill. A model
does the writing; Skill Hive does everything around it: it decides which files travel, what the
frontmatter becomes, which evaluation cases survive, and it writes nothing until the draft is
accepted. The code is `crates/hive-core/src/merge.rs`; the app reaches it through
`merge_propose` and `merge_apply`, the CLI through `hive merge`.

## Propose

`propose_merge` takes two or more library ids (duplicates are collapsed; fewer than two distinct
ids or anything that is not a skill is an error), an optional name, optional free-form guidance
and an optional provider id.

**Provider.** The one asked for; otherwise the default provider with the recommender role, then
the grader, then the executor. With none configured the call fails and points at
Settings > Providers. The request uses the provider's timeout, raised to at least 60 seconds,
and no explicit temperature.

**Prompt.** The system prompt says what a skill is, that agents choose by name and description
alone, and that the body must keep every distinct procedure, rule, command, file reference and
caveat, say shared things once, resolve contradictions explicitly, stay under 500 lines and refer
to bundled files by their final paths. The user prompt holds, in order:

- the number of skills, the required name when one was given, and the guidance;
- "Why they overlap": every conflict the analyser finds among the sources, with its detail;
- one section per source: name, description, pre-approved tools, licence, the bundled files with
  their final path in the merged skill (and the original path when it changes), then the
  `SKILL.md` body in a code fence, capped at 24 000 characters with a `[… truncated …]` marker;
- up to 8 excerpts per source of its bundled text files (by extension: markdown, scripts,
  JSON, YAML, TOML, CSV and the like; files over 512 KiB are skipped), 1 200 characters each;
- the answer shape to use.

The output budget is 8 192 tokens.

**Answer.** Strict JSON: `{"name", "description", "body", "notes", "dropped": [...],
"keep_files": [...]}`. The first `{` to the last `}` of the reply is parsed, so a code fence or a
sentence around it does no harm, but the object must carry a body or a description. If there is
no usable JSON, a raw `SKILL.md` (frontmatter and body, optionally inside a `markdown` fence) is
accepted instead; then notes and the dropped list are empty and `keep_files` is unknown. Anything
else is an error ("the model did not return a merged skill").

**Name.** The requested name wins and is validated up front (1–64 lowercase letters, digits or
single hyphens). Otherwise the model's name is slugified and used when valid (with a warning when
it was changed, for example `PDF Tools` to `pdf-tools`). Otherwise the sources' names are joined
with hyphens, slugified and cut to 64 characters, with a warning; `merged-skill` if even that is
empty. The proposal is validated like any authored skill, and a skill outside the merge that
already has the name adds the error finding `name.taken` on the `name` field.

**Frontmatter.** Besides name and description the merged skill gets:

- `allowed-tools`: the union of the sources' pre-approved tools, in source order, when any;
- `license`: only when every source that has a licence has the same one; sources without one do
  not count. Different licences produce a warning and no licence;
- `metadata`: every key from every source, the first source's value winning on a clash, plus
  `merged-from` set to the sources' names, comma separated.

`compatibility` is not carried. Tags are the sorted union of the sources' tags.

**Bundled files.** Everything in a source directory except `SKILL.md` and the `evals/` suite,
in source order. A later source whose file has the same relative path as an earlier one gets it
under `<its slug>/<path>` when the content differs; a byte-identical file is carried once, from
the first source that had it. Each entry has `path` (in the source), `dest` (in the merged
skill), its size and a `keep` flag. `keep` starts true for every file; when the model returned
`keep_files`, only the files it lists (by final or original path, a leading `./` ignored) stay
true. A file whose final path appears in the body is kept whatever the model said. The user can
flip any flag before applying.

**Evaluation cases.** The union of the sources' suites, deduplicated: cases by prompt and
trigger cases by query, both compared trimmed and case-insensitively, the first source winning.
`eval_cases` in the proposal is that count. A suite that cannot be read produces a warning and
contributes nothing.

**Figures.** `body_tokens_before` is the sum of the sources' instruction token estimates,
`body_tokens_after` the estimate of the merged body; the description pair is the standing
context cost before and after. The proposal also carries the provider, the model, the token
counts and latency the provider reported, the model's notes, the `dropped` list, the warnings and
the validation report. An audit row `merge.propose` records the sources, name and provider.

## Apply

`apply_merge` takes the accepted proposal: source ids, name, description, body, the extra
frontmatter, the file list, tags, `retire_sources` and `carry_evals`. The CLI passes the proposal
unchanged; the app passes whatever the user edited. Step by step:

1. Validate the name and reload the sources (the same checks as propose). If the name is a
   source's name, that source becomes the one replaced in place; if it is any other skill's name
   the call fails with "already exists" and nothing changes.
2. Stage the skill in a temporary directory: render `SKILL.md` from name, description, the extra
   frontmatter and body, and parse it back to be sure it is valid.
3. Copy every file with `keep` set to its `dest`. A path that escapes the skill, or a file from a
   skill outside the merge, is an error.
4. With `carry_evals`, build the union suite, copy each case's input files from the source that
   owns the case (a path two sources use for different content is written as
   `<slug>-<file name>` next to the original and the case updated), renumber the cases and write
   `evals/`. Nothing is written when the union is empty.
5. Import the staged directory into the library as an authored skill with the tags. Replacing in
   place keeps the source's id, sidecar, deployments and run history; its copy deployments are
   re-synced so they carry the new content. Otherwise a new item is created.
6. With `retire_sources`, for every other source: note each deployment's target, scope, project
   and mode, undeploy it, and remove the skill from the library.
7. Deploy the merged skill to each noted place, skipping any where it already has a deployment
   for the same target, scope and project. A deployment that fails is reported in `failed` and
   does not undo the merge.
8. Write the audit row `merge.apply`, emit `LibraryChanged` and `DeploymentsChanged`, and return
   the item, the retired items, `replaced_id`, the deployments made, the failures and the number
   of eval cases written.

Without `retire_sources` the sources stay, keep their deployments, and the overlap remains.

## CLI

```
hive merge <SKILLS>... [--name N] [--guidance TEXT] [--provider P]
           [--keep-sources] [--no-evals] [--dry-run]
```

Skills are addressed by id, name or `skill:name`. The command prints the proposal (name,
description, token figures, files kept and dropped, eval cases, notes, what was left out,
warnings, validation findings, provider usage and the body) and applies it in one go. The
defaults retire the sources and carry evaluations; `--keep-sources` and `--no-evals` turn those
off. `--dry-run` stops after the proposal, and so does a proposal with a validation error (exit
code 1 with the messages). `--json` prints the proposal on a dry run, otherwise the result.

## Limitations

- The model can drop content. Read the "left out" list and the body before accepting; the
  system prompt asks it to keep everything distinct, but nothing checks that it did. The merged
  description is what agents will see, so a trigger evaluation afterwards is worth running.
- Bodies over 24 000 characters are truncated in the prompt, and only 8 text files per source
  are excerpted, 1 200 characters each; the model never sees binary files or the rest.
- Token figures are estimates from character and word counts (characters / 4, or 0.75 per word
  when larger), not a tokenizer's answer; only the provider's own input and output counts are
  reported figures.
- The licence and metadata rules are mechanical: check them when the sources came from
  different places. Only skills merge; subagents and commands are refused.
