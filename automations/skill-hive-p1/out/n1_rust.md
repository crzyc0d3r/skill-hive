# n1 rust

| cargo test | 0 |

test util::file_watcher::tests::non_linked_project_skips_empty_skill_dirs ... ok
test util::log_sanitize::redacts_provider_tokens_and_bearer ... ok
test util::log_sanitize::tests::redacts_email_and_token ... ok
test util::log_sanitize::tests::redacts_git_credentials ... ok
test util::log_sanitize::tests::replaces_generic_users_path ... ok
test util::path_guard::tests::path_safe_accepts_base_itself ... ok
test util::path_guard::tests::path_safe_allows_subpath ... ok
test util::path_guard::tests::path_safe_handles_non_existing_target ... ok
test util::path_guard::tests::path_safe_rejects_symlink_escape ... ok
test util::path_guard::tests::path_safe_rejects_sibling ... ok
test util::path_guard::tests::sanitize_blocks_traversal ... ok
test util::path_guard::tests::sanitize_collapses_consecutive_dots ... ok
test util::path_guard::tests::sanitize_empty_yields_fallback ... ok
test util::file_watcher::tests::non_linked_project_skips_missing_skill_dirs ... ok
test util::path_guard::tests::path_safe_rejects_traversal ... ok
test util::path_guard::tests::sanitize_preserves_unicode ... ok
test util::path_guard::tests::sanitize_strips_null_and_control ... ok
test util::path_guard::tests::sanitize_strips_windows_forbidden ... ok
test util::path_guard::tests::sanitize_strips_path_separators ... ok
test util::path_guard::tests::sanitize_trims_dots_and_whitespace ... ok
test util::path_guard::tests::sanitize_truncates_long_names ... ok
test util::file_watcher::tests::non_linked_project_watches_non_empty_skill_dirs ... ok
test util::timing::tests::first_call_always_logs ... ok
test util::timing::tests::slow_calls_log_even_after_first ... ok
test util::repo_lock::tests::blocking_acquire_waits_then_times_out_and_recovers ... ok
test util::repo_lock::tests::lock_file_lives_outside_skills_dir ... ok
test util::repo_lock::tests::stamp_is_readable_by_another_handle_while_held ... ok

test result: ok. 124 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.53s

