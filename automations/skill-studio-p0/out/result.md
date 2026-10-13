# Skill Studio Phase 0

Local verification for milestone M0. schema v8 is asserted by `studio_db_migrates_to_v8_and_layout_is_created`.

| check | exit |
|---|---|
| cargo fmt | 0 |
| cargo test | 0 |
| npm test | 0 |
| tsc | 0 |
| contrast | 0 |
| headers | 0 |

## Evidence

### cargo fmt (exit 0)

```
(no output)
```

### cargo test (exit 0)

```
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 1 test
test second_process_waits_for_the_repo_lock ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Compiling studio-core v0.0.1 (/workspace/crates/studio-core)
   Compiling skill-studio v0.0.1 (/workspace/src-tauri)
   Compiling studio-cli v0.0.1 (/workspace/crates/studio-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.27s
     Running unittests src/lib.rs (target/debug/deps/skill_studio_lib-6dad3ad58e561408)
     Running unittests src/main.rs (target/debug/deps/skill_studio-714777b9fff99081)
     Running unittests src/main.rs (target/debug/deps/skill_studio_cli-9671df0de4a507ea)
     Running unittests src/lib.rs (target/debug/deps/studio_core-9f1abf2ba130a841)
     Running unittests src/bin/lock-holder.rs (target/debug/deps/lock_holder-e03beb06203bb7bf)
     Running tests/repo_lock_processes.rs (target/debug/deps/repo_lock_processes-c7751ac869086dfe)
   Doc-tests skill_studio_lib
   Doc-tests studio_core
```

### npm test (exit 0)

```
> skill-studio@0.0.1 test
> vitest run


 RUN  v5.0.4 /workspace


 Test Files  3 passed (3)
      Tests  4 passed (4)
   Start at  19:11:16
   Duration  967ms (environment 60%, import 16%, tests 14%, setup 6%, transform 3%, worker 1%)
```

### tsc (exit 0)

```
(no output)
```

### contrast (exit 0)

```
pass light foreground/background: 17.85:1 (min 4.5:1)
pass light muted-foreground/background: 7.58:1 (min 4.5:1)
pass light primary/primary-foreground: 13.98:1 (min 4.5:1)
pass light success text: 5.02:1 (min 4.5:1)
pass light warning text: 5.02:1 (min 4.5:1)
pass light destructive text: 6.47:1 (min 4.5:1)
pass light info text: 6.70:1 (min 4.5:1)
pass dark foreground/background: 17.06:1 (min 4.5:1)
pass dark muted-foreground/background: 6.96:1 (min 4.5:1)
pass dark success text: 10.25:1 (min 4.5:1)
pass dark warning text: 10.69:1 (min 4.5:1)
pass dark destructive text: 6.45:1 (min 4.5:1)
pass dark info text: 7.02:1 (min 4.5:1)
```

### headers (exit 0)

```
crates/studio-core/src/deploy/mod.rs
crates/studio-core/src/deploy/tool_adapters.rs
crates/studio-core/src/library/central_repo.rs
crates/studio-core/src/library/mod.rs
crates/studio-core/src/library/sync_metadata.rs
5 derived files carry the skills-manager header
```

pass: cargo test and npm test are green locally; schema v8 migration is in the studio-core suite.
