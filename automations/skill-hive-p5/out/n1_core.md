# n1 core

hive-core generation exit 0
hive-core analytics exit 0
hive-cli contract exit 0
exit 0

   Compiling hive-core v0.0.1 (/workspace/crates/hive-core)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4.13s
     Running unittests src/lib.rs (target/debug/deps/hive_core-36029881e13a7d4c)

running 9 tests
test generation::tests::envelope_parser_rejects_escape_and_reports_the_path ... ok
test generation::tests::drafted_trigger_set_has_at_least_sixteen_queries ... ok
test generation::tests::cancel_removes_staging_and_leaves_the_library_untouched ... ok
test generation::tests::malformed_reply_is_retried_once_then_surfaced ... ok
test generation::tests::rendered_p2_matches_the_golden_file ... ok
test generation::tests::generated_scripts_are_not_executable_and_safety_lists_flagged_lines ... ok
test generation::tests::revise_changes_only_files_named_in_changes_md ... ok
test generation::tests::generation_session_row_lands_in_schema_v12 ... ok
test generation::tests::nine_of_ten_recorded_goals_reach_review_and_the_tenth_is_not_saved ... ok

test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 197 filtered out; finished in 0.02s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.12s
     Running unittests src/lib.rs (target/debug/deps/hive_core-36029881e13a7d4c)

running 7 tests
test analytics::tests::each_regression_rule_has_a_positive_and_a_negative_case ... ok
test analytics::tests::held_out_winner_beats_a_failing_candidate_for_any_split ... ok
test analytics::tests::history_keeps_a_single_current_best_and_a_parent_chain ... ok
test analytics::tests::apply_optimized_is_a_separate_click ... ok
test analytics::tests::comparator_request_hides_every_side_revealing_string ... ok
test analytics::tests::static_viewer_junit_and_csv_match_the_slice ... ok
test analytics::tests::workflows_d1_d2_and_d3_run_on_recorded_fixtures ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 199 filtered out; finished in 0.10s

   Compiling hive-core v0.0.1 (/workspace/crates/hive-core)
   Compiling hive-cli v0.0.1 (/workspace/crates/hive-cli)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.78s
     Running tests/contract.rs (target/debug/deps/contract-2e6dae055e9fb7ed)

running 3 tests
test usage_error_is_exit_2_with_invalid_argument ... ok
test deploy_into_an_occupied_target_returns_target_conflict ... ok
test contract_suite_exercises_every_subcommand ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.35s

