# n1 rust

exit 101
test util::path_guard::tests::sanitize_strips_path_separators ... ok
test util::path_guard::tests::sanitize_strips_windows_forbidden ... ok
test util::path_guard::tests::sanitize_trims_dots_and_whitespace ... ok
test util::path_guard::tests::sanitize_truncates_long_names ... ok
test util::file_watcher::tests::non_linked_project_watches_non_empty_skill_dirs ... ok
test library::marketplace::tests::zip_entry_count_over_4096_is_refused ... ok
test util::repo_lock::tests::blocking_acquire_waits_then_times_out_and_recovers ... ok
test util::timing::tests::first_call_always_logs ... ok
test util::repo_lock::tests::lock_file_lives_outside_skills_dir ... ok
test util::tray_menu::tests::hidden_tray_refuses_minimize ... ok
test util::tray_menu::tests::tray_menu_lists_status_presets_and_quit ... ok
test util::timing::tests::slow_calls_log_even_after_first ... ok
test util::repo_lock::tests::stamp_is_readable_by_another_handle_while_held ... ok
test library::marketplace::tests::zip_over_100_mib_is_refused ... ok

failures:

---- library::marketplace::tests::recorded_http_fixture_serves_search stdout ----

thread 'library::marketplace::tests::recorded_http_fixture_serves_search' (22085) panicked at crates/hive-core/src/library/marketplace.rs:279:85:
called `Result::unwrap()` on an `Err` value: Connection reset by peer (os error 104)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    library::marketplace::tests::recorded_http_fixture_serves_search

test result: FAILED. 156 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.40s

error: test failed, to rerun pass `-p hive-core --lib`
