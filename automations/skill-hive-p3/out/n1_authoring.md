# n1 authoring

hive-core authoring exit 0
hive-cli contract exit 0
exit 0

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running unittests src/lib.rs (target/debug/deps/hive_core-5fd5f2215bf3f991)

running 11 tests
test authoring::frontmatter::tests::editing_description_touches_only_that_key ... ok
test authoring::frontmatter::tests::round_trip_preserves_block_scalars_and_quotes ... ok
test authoring::tests::description_edit_changes_only_that_key_and_block_scalars_round_trip ... ok
test authoring::tests::agent_rules_export_and_conflict ... ok
test authoring::tests::publish_confirms_symlinks_and_marks_copies_center_changed ... ok
test authoring::tests::rename_refuses_unsafe_names_and_updates_evals ... ok
test authoring::tests::body_gauge_references_and_silent_skip_reasons ... ok
test authoring::tests::templates_have_zero_errors_and_paths_stay_inside ... ok
test authoring::tests::versions_dedup_pin_restore_and_history_schema ... ok
test authoring::tests::command_rules_and_agent_rows ... ok
test authoring::tests::every_rule_id_has_a_function_path_and_the_suite_agrees_with_quick_validate ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 157 filtered out; finished in 0.43s

    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.09s
     Running tests/contract.rs (target/debug/deps/contract-17b2a9ab6d47cc6e)

running 3 tests
test usage_error_is_exit_2_with_invalid_argument ... ok
test deploy_into_an_occupied_target_returns_target_conflict ... ok
test contract_suite_exercises_every_subcommand ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s

