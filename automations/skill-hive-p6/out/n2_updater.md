# n2 updater

hive-core updater exit 0
path guard exit 0
log scrub exit 0
vitest exit 0
exit 0

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.74s
     Running unittests src/lib.rs (target/debug/deps/hive_core-836865bfe5244699)

running 2 tests
test updater::tests::an_active_run_blocks_install_and_a_proxy_must_be_http ... ok
test updater::tests::a_tampered_latest_json_signature_is_rejected ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 207 filtered out; finished in 0.01s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.11s
     Running unittests src/lib.rs (target/debug/deps/hive_core-836865bfe5244699)

running 1 test
test util::path_guard::tests::sanitize_rejects_traversal_payloads ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 208 filtered out; finished in 0.00s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running unittests src/lib.rs (target/debug/deps/hive_core-836865bfe5244699)

running 1 test
test util::log_sanitize::redacts_provider_tokens_and_bearer ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 208 filtered out; finished in 0.01s


 RUN  v5.0.4 /workspace


 Test Files  2 passed (2)
      Tests  10 passed (10)
   Start at  05:50:20
   Duration  2.82s (environment 41%, import 21%, tests 18%, transform 15%, setup 5%)

