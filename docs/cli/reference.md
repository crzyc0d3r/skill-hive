# Skill Hive CLI reference

Generated from the clap command tree in `crates/hive-cli/src/lib.rs`. The binary name is `skill-hive-cli`.

Global flags:

- `--json` prints a JSON envelope on stdout.
- `--skills-root <PATH>` operates on an external skills directory.
- `--version` prints the version.
- `--help` prints a command's flags.

Exit codes: 0 success, 1 operational error, 2 usage error.

## repo

- `repo init`
- `repo path`
- `repo migrate`

## agents

- `agents list`
- `agents enable --key <KEY>`
- `agents disable --key <KEY>`
- `agents add-custom --key <KEY> --name <NAME> --path <PATH>`
- `agents def list`
- `agents def show`
- `agents def deploy`
- `agents def undeploy`
- `agents def validate`

## skills

- `skills list`
- `skills show --id <ID>`
- `skills export --id <ID> --to <PATH>`
- `skills install --path <PATH>`
- `skills update --id <ID>`
- `skills remove --id <ID>`
- `skills search`
- `skills deploy`
- `skills check`
- `skills adopt`
- `skills enable`
- `skills disable`
- `skills tag`

## presets

- `presets list`
- `presets show`
- `presets create`
- `presets delete`
- `presets current`
- `presets apply`
- `presets deactivate`
- `presets deploy`
- `presets add-skill`
- `presets remove-skill`
- `presets preview`

## git

- `git init`
- `git status`
- `git commit`
- `git set-remote`
- `git pull`
- `git push`
- `git clone`
- `git restore`
- `git versions`
- `git prune-sync-refs`

## providers

- `providers list`
- `providers add --id <ID> --name <NAME> --kind <KIND>`
- `providers set-secret --id <ID> --secret <SECRET>`
- `providers has-secret --id <ID>`
- `providers test`
- `providers models`

## test

- `test save-suite --skill <PATH>`
- `test run --skill <PATH> --mode <MODE>`
- `test show`
- `test suites`
- `test status`
- `test results`
- `test export`

`test run --mode replay` grades a recorded fixture and does not call a live model.

## generate

- `generate skill --name <NAME> [--replay]`
- `generate agent --name <NAME> [--replay]`
- `generate evals --name <NAME> [--replay]`

`--replay` answers from the committed fixtures. Omit it only on a machine that already has an endpoint configured.
