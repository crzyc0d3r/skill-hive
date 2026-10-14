# Definition of done for v1.0

Each row of plan section 7.1, with the evidence this phase can show.

1. Parity. Phases 1 and 2 closed the must and should rows. The ledgers are `docs/parity/m1-ledger.md` and `docs/parity/m2-ledger.md`. Skill-aware merge and libgit2 stay deferred, and the Backup screen says so.
2. Authoring. Phase 3 validator, editors, and versions. `cargo test -p hive-core --lib authoring::` on CI.
3. Testing. Phase 4 engine and replay. `cargo test -p hive-core --lib testing::` on CI.
4. Generation. Phase 5 wizard, comparison, history, and exports on recorded fixtures. Live nine-of-ten measurement stays the frozen replay, because no vendor key is configured.
5. Platforms. `test.yml` runs on `ubuntu-22.04`, `macos-latest`, and `windows-latest`. The macOS smoke list is `macos-smoke.md`. Windows junction tests and the Linux keyring fallback stay in the core suite from earlier phases.
6. Release. The workflow builds five targets and verifies them. Signing, notarization, Homebrew install, and winget install activate when `secrets.md` is filled in. Until then the tag stays a draft and this row is unsigned.
7. Security. `security.md`.
8. Accessibility and performance. `accessibility.md` and `performance.md`.
9. Attribution. `THIRD_PARTY_LICENSES/skills-manager.MIT.txt`, derived-file headers, and the About notice. The updater key id is `8FC6E5C30C36CA69`.
10. Documentation. `docs/user-guide.md`, `docs/cli/reference.md`, `docs/dev/developer-guide.md`, `docs/release/v1.0.0.md`, and the Help cards.
11. Quality gate. No release blocker is open in this phase. Defects that need a certificate are listed in `secrets.md` instead of being hidden.

Signed off for the rows the automated suite can see. Rows that need Apple, Windows, or the updater private key stay listed in `secrets.md`.
