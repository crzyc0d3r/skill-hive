# n1 core

hive-core providers exit 0
hive-core testing exit 0
hive-cli contract exit 0
exit 0

   Compiling hive-core v0.0.1 (/workspace/crates/hive-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.56s
     Running unittests src/lib.rs (target/debug/deps/hive_core-36029881e13a7d4c)

running 14 tests
test providers::secrets::tests::linux_without_dbus_uses_labelled_fallback_and_hides_the_secret ... ok
test providers::tests::missing_recording_does_not_call_the_provider ... ok
test providers::tests::noop_sleeper_records_retry_after ... ok
test providers::tests::http_429_and_a_truncated_stream_are_retried ... ok
test providers::tests::anthropic_openai_and_ollama_answer_on_localhost ... ok
test providers::tests::public_provider_json_never_contains_the_secret ... ok
test providers::tests::record_then_replay_is_stable_and_costs_zero ... ok
test providers::tests::pricing_uses_the_published_table_and_a_user_override ... ok
test providers::tests::retry_honours_retry_after_and_stops_after_success ... ok
test providers::tests::scrub_removes_secrets_from_logs_payloads_and_debug ... ok
test providers::tests::store_round_trip_keeps_last_test_and_reference_state ... ok
test providers::tests::seven_stage_probe_covers_anthropic_ollama_and_claude_cli ... ok
test providers::tests::limiter_peak_stays_at_the_cap ... ok
test providers::tests::process_group_kill_stops_the_grandchild ... ok

test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 176 filtered out; finished in 0.16s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.13s
     Running unittests src/lib.rs (target/debug/deps/hive_core-36029881e13a7d4c)

running 8 tests
test testing::tests::aggregate_matches_the_python_fixture ... ok
test testing::tests::local_http_endpoint_can_drive_one_live_cell ... ok
test testing::tests::malformed_grader_output_is_a_failed_expectation ... ok
test testing::tests::suite_round_trip_rejects_empty_names_and_duplicate_ids ... ok
test testing::tests::trigger_verdicts_agree_with_python_across_three_runs ... ok
test testing::tests::sandbox_refuses_parent_paths_and_aliases_personal_collisions ... ok
test testing::tests::budget_pause_cancel_and_crash_resume ... ok
test testing::tests::fixture_run_writes_the_skill_creator_layout_live_and_replay ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 182 filtered out; finished in 0.36s

   Compiling hive-core v0.0.1 (/workspace/crates/hive-core)
   Compiling hive-cli v0.0.1 (/workspace/crates/hive-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 5.22s
     Running tests/contract.rs (target/debug/deps/contract-2e6dae055e9fb7ed)

running 3 tests
test usage_error_is_exit_2_with_invalid_argument ... ok
test deploy_into_an_occupied_target_returns_target_conflict ... ok
test contract_suite_exercises_every_subcommand ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.30s

