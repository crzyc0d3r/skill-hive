// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::{bail, Context, Result};
use rusqlite::Connection;

/// Current schema version. Bump this when adding a new migration.
const LATEST_VERSION: u32 = 11;

/// Run all pending migrations on the database.
///
/// - New databases: creates full schema and sets version to LATEST_VERSION.
/// - Existing databases (user_version == 0): runs incremental migrations
///   to bring them up to date.
/// - Databases newer than this app version: returns an error.
pub fn run_migrations(conn: &Connection) -> Result<()> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current > LATEST_VERSION {
        bail!(
            "Database schema version ({current}) is newer than this app supports ({LATEST_VERSION}). \
             Please upgrade the application."
        );
    }

    if current == LATEST_VERSION {
        return Ok(());
    }

    // Run each migration step in a transaction
    for version in current..LATEST_VERSION {
        conn.execute_batch("BEGIN EXCLUSIVE")?;
        match migrate_step(conn, version) {
            Ok(()) => {
                conn.pragma_update(None, "user_version", version + 1)?;
                conn.execute_batch("COMMIT")?;
            }
            Err(e) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(e).with_context(|| {
                    format!("migration from version {version} to {} failed", version + 1)
                });
            }
        }
    }

    Ok(())
}

/// Execute a single migration step: version N → N+1.
fn migrate_step(conn: &Connection, from_version: u32) -> Result<()> {
    match from_version {
        0 => migrate_v0_to_v1(conn),
        1 => migrate_v1_to_v2(conn),
        2 => migrate_v2_to_v3(conn),
        3 => migrate_v3_to_v4(conn),
        4 => migrate_v4_to_v5(conn),
        5 => migrate_v5_to_v6(conn),
        6 => migrate_v6_to_v7(conn),
        7 => migrate_v7_to_v8(conn),
        8 => migrate_v8_to_v9(conn),
        9 => migrate_v9_to_v10(conn),
        10 => migrate_v10_to_v11(conn),
        _ => bail!("unknown migration version: {from_version}"),
    }
}

/// v0 → v1: Initial schema.
///
/// For new databases this creates all tables from scratch.
/// For existing pre-migration databases, the `CREATE TABLE IF NOT EXISTS`
/// statements are no-ops, and the `add_column_if_missing` calls handle
/// columns that were added incrementally before the migration system existed.
fn migrate_v0_to_v1(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            source_type TEXT NOT NULL,
            source_ref TEXT,
            source_ref_resolved TEXT,
            source_subpath TEXT,
            source_branch TEXT,
            source_revision TEXT,
            remote_revision TEXT,
            central_path TEXT NOT NULL UNIQUE,
            content_hash TEXT,
            enabled INTEGER DEFAULT 1,
            created_at INTEGER,
            updated_at INTEGER,
            status TEXT DEFAULT 'ok',
            update_status TEXT DEFAULT 'unknown',
            last_checked_at INTEGER,
            last_check_error TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_skills_name ON skills(name);

        CREATE TABLE IF NOT EXISTS skill_targets (
            id TEXT PRIMARY KEY,
            skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
            tool TEXT NOT NULL,
            target_path TEXT NOT NULL,
            mode TEXT NOT NULL,
            status TEXT DEFAULT 'ok',
            synced_at INTEGER,
            last_error TEXT,
            source_hash TEXT,
            UNIQUE(skill_id, tool)
        );

        CREATE TABLE IF NOT EXISTS discovered_skills (
            id TEXT PRIMARY KEY,
            tool TEXT NOT NULL,
            found_path TEXT NOT NULL,
            name_guess TEXT,
            fingerprint TEXT,
            found_at INTEGER NOT NULL,
            imported_skill_id TEXT REFERENCES skills(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS skillssh_cache (
            cache_key TEXT PRIMARY KEY,
            data TEXT NOT NULL,
            fetched_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS scenarios (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT,
            icon TEXT,
            sort_order INTEGER DEFAULT 0,
            created_at INTEGER,
            updated_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS scenario_skills (
            scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
            skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
            added_at INTEGER,
            PRIMARY KEY(scenario_id, skill_id)
        );

        CREATE TABLE IF NOT EXISTS scenario_skill_tools (
            scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
            skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
            tool TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(scenario_id, skill_id, tool)
        );

        CREATE TABLE IF NOT EXISTS active_scenario (
            key TEXT PRIMARY KEY DEFAULT 'current',
            scenario_id TEXT REFERENCES scenarios(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            workspace_type TEXT NOT NULL DEFAULT 'project',
            linked_agent_key TEXT,
            linked_agent_name TEXT,
            disabled_path TEXT,
            sort_order INTEGER DEFAULT 0,
            created_at INTEGER,
            updated_at INTEGER
        );

        CREATE TABLE IF NOT EXISTS skill_tags (
            skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
            tag TEXT NOT NULL,
            PRIMARY KEY(skill_id, tag)
        );
        CREATE INDEX IF NOT EXISTS idx_skill_tags_tag ON skill_tags(tag);
        ",
    )?;

    // For pre-migration databases: add columns that didn't exist in the original schema.
    // For new databases these are already in the CREATE TABLE, so the calls are no-ops.
    add_column_if_missing(conn, "scenarios", "icon", "TEXT")?;
    add_column_if_missing(conn, "skills", "source_ref_resolved", "TEXT")?;
    add_column_if_missing(conn, "skills", "source_subpath", "TEXT")?;
    add_column_if_missing(conn, "skills", "source_branch", "TEXT")?;
    add_column_if_missing(conn, "skills", "remote_revision", "TEXT")?;
    add_column_if_missing(conn, "skills", "update_status", "TEXT DEFAULT 'unknown'")?;
    add_column_if_missing(conn, "skills", "last_checked_at", "INTEGER")?;
    add_column_if_missing(conn, "skills", "last_check_error", "TEXT")?;

    Ok(())
}

/// v1 → v2: Add per-scenario, per-skill tool toggle table.
fn migrate_v1_to_v2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS scenario_skill_tools (
            scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
            skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
            tool TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY(scenario_id, skill_id, tool)
        );
        ",
    )?;
    Ok(())
}

/// v2 → v3: Add sort_order to scenario_skills for drag-and-drop reordering.
fn migrate_v2_to_v3(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "scenario_skills", "sort_order", "INTEGER DEFAULT 0")?;
    Ok(())
}

/// v3 → v4: Expand projects into generic workspace records.
fn migrate_v3_to_v4(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL UNIQUE,
            workspace_type TEXT NOT NULL DEFAULT 'project',
            linked_agent_key TEXT,
            linked_agent_name TEXT,
            disabled_path TEXT,
            sort_order INTEGER DEFAULT 0,
            created_at INTEGER,
            updated_at INTEGER
        );
        ",
    )?;
    add_column_if_missing(
        conn,
        "projects",
        "workspace_type",
        "TEXT NOT NULL DEFAULT 'project'",
    )?;
    add_column_if_missing(conn, "projects", "linked_agent_key", "TEXT")?;
    add_column_if_missing(conn, "projects", "linked_agent_name", "TEXT")?;
    add_column_if_missing(conn, "projects", "disabled_path", "TEXT")?;
    Ok(())
}

/// v4 → v5: Add audit log table — append-only history of user/system actions.
fn migrate_v4_to_v5(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS audit_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts INTEGER NOT NULL,
            action TEXT NOT NULL,
            skill_id TEXT,
            skill_name TEXT,
            tool TEXT,
            success INTEGER NOT NULL,
            detail TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_audit_log_ts ON audit_log(ts);
        ",
    )?;
    Ok(())
}

/// v5 → v6: Add `source_hash` to `skill_targets`. Lets the sync engine
/// skip a Copy-mode resync when the central skill content has not
/// changed since the last successful sync, avoiding the per-startup
/// recursive copy that pinned Windows users on issue #153.
///
/// Existing rows get NULL, which is treated as "no recorded hash" and
/// forces one copy on the first post-upgrade sync. No backfill needed.
fn migrate_v5_to_v6(conn: &Connection) -> Result<()> {
    add_column_if_missing(conn, "skill_targets", "source_hash", "TEXT")?;
    Ok(())
}

/// v6 → v7: pending-conflict projection for the object merge engine
/// (merge-engine design §4). A local UI cache only — the source of truth is
/// the commit trailers plus `refs/skills-manager/conflict/*`, from which
/// this table is rebuilt at startup and after every merge.
fn migrate_v6_to_v7(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS pending_conflicts (
            skill_id TEXT PRIMARY KEY,
            theirs_commit TEXT NOT NULL,
            theirs_path TEXT,
            detected_at INTEGER NOT NULL
        );
        ",
    )?;
    Ok(())
}

/// Drop the orphaned `project_default_export_agents` preference.
///
/// It was written behind a "save default agents" action that 688fc9b removed
/// along with the old Add Skills flow, leaving the reader behind. Users who
/// used that button still carry a frozen subset they can neither see nor
/// change, and it silently narrows which agents a project preset reaches —
/// exactly the failure #400 reported, but invisible and unfixable from the UI.
/// A preference with no way to inspect or edit it is a trap, not a preference.
fn migrate_v7_to_v8(conn: &Connection) -> Result<()> {
    // A database can reach this step without a settings table (older partial
    // schemas do), and a cleanup has no business failing an upgrade.
    let has_settings: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'settings')",
        [],
        |row| row.get(0),
    )?;
    if !has_settings {
        return Ok(());
    }
    conn.execute(
        "DELETE FROM settings WHERE key = 'project_default_export_agents'",
        [],
    )?;
    Ok(())
}

/// v8 → v9: agent definitions and the places they are exported.
fn migrate_v8_to_v9(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            tags TEXT NOT NULL DEFAULT '[]',
            library_path TEXT NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS agent_targets (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            path TEXT NOT NULL,
            mode TEXT NOT NULL,
            exported_hash TEXT,
            version_id TEXT,
            FOREIGN KEY(agent_id) REFERENCES agents(id)
        );
        ",
    )?;
    Ok(())
}

/// v9 → v10: provider configs, pricing, and the usage ledger.
fn migrate_v9_to_v10(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS provider_configs (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            kind TEXT NOT NULL,
            flavor TEXT,
            base_url TEXT,
            default_model TEXT,
            api_version TEXT,
            auth_mode TEXT NOT NULL DEFAULT 'keyring',
            auth_env_var TEXT,
            keyring_account TEXT,
            extra_headers_json TEXT NOT NULL DEFAULT '{}',
            cli_program TEXT,
            cli_args_json TEXT NOT NULL DEFAULT '[]',
            timeout_ms INTEGER NOT NULL DEFAULT 600000,
            max_retries INTEGER NOT NULL DEFAULT 3,
            max_concurrency INTEGER NOT NULL DEFAULT 4,
            rpm_limit INTEGER,
            tpm_limit INTEGER,
            default_effort TEXT,
            default_max_tokens INTEGER NOT NULL DEFAULT 16000,
            supports_tools INTEGER NOT NULL DEFAULT 1,
            supports_streaming INTEGER NOT NULL DEFAULT 1,
            proxy_mode TEXT NOT NULL DEFAULT 'inherit',
            role_executor INTEGER NOT NULL DEFAULT 0,
            role_grader INTEGER NOT NULL DEFAULT 0,
            role_generator INTEGER NOT NULL DEFAULT 0,
            last_test_at INTEGER,
            last_test_ok INTEGER,
            last_test_latency_ms INTEGER,
            last_test_error TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS model_pricing (
            id TEXT PRIMARY KEY,
            provider_config_id TEXT NOT NULL DEFAULT '*',
            model TEXT NOT NULL,
            input_per_mtok REAL NOT NULL,
            output_per_mtok REAL NOT NULL,
            cache_read_per_mtok REAL,
            cache_write_per_mtok REAL,
            source TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE (provider_config_id, model)
        );
        CREATE TABLE IF NOT EXISTS usage_ledger (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts INTEGER NOT NULL,
            provider_config_id TEXT REFERENCES provider_configs(id) ON DELETE SET NULL,
            model TEXT NOT NULL,
            purpose TEXT NOT NULL,
            ref_kind TEXT,
            ref_id TEXT,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cache_read_tokens INTEGER NOT NULL DEFAULT 0,
            cache_write_tokens INTEGER NOT NULL DEFAULT 0,
            cost_usd REAL,
            latency_ms INTEGER,
            status TEXT NOT NULL,
            request_id TEXT,
            error_kind TEXT
        );
        ",
    )?;
    let settings: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'settings'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    if settings > 0 {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES ('pricing_as_of', '2026-10-06')",
            [],
        )?;
    }
    for (model, input, output, cache_read, cache_write) in [
        ("claude-opus-5-5", 4.0, 20.0, 0.40, 5.0),
        ("claude-sonnet-5-5", 2.0, 10.0, 0.20, 2.5),
        ("claude-haiku-5-5", 0.10, 0.50, 0.01, 0.125),
        ("claude-fable-5-1", 10.0, 50.0, 1.0, 12.5),
    ] {
        conn.execute(
            "INSERT OR IGNORE INTO model_pricing
             (id, provider_config_id, model, input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, source, updated_at)
             VALUES (?1, '*', ?2, ?3, ?4, ?5, ?6, 'builtin', 0)",
            params_pricing(model, input, output, cache_read, cache_write),
        )?;
    }
    Ok(())
}

fn params_pricing(
    model: &str,
    input: f64,
    output: f64,
    cache_read: f64,
    cache_write: f64,
) -> (&str, &str, f64, f64, f64, f64) {
    (model, model, input, output, cache_read, cache_write)
}

/// v10 → v11: test suites, runs, steps, and grades.
fn migrate_v10_to_v11(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS test_suites (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            kind TEXT NOT NULL,
            skill_id TEXT REFERENCES skills(id) ON DELETE CASCADE,
            agent_id TEXT REFERENCES agents(id) ON DELETE CASCADE,
            source TEXT NOT NULL DEFAULT 'db',
            baseline TEXT NOT NULL DEFAULT 'without_skill',
            workspace_dir TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            CHECK (skill_id IS NOT NULL OR agent_id IS NOT NULL)
        );
        CREATE TABLE IF NOT EXISTS test_cases (
            id TEXT PRIMARY KEY,
            suite_id TEXT NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
            eval_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            prompt TEXT NOT NULL,
            expected_output TEXT,
            files_json TEXT NOT NULL DEFAULT '[]',
            expectations_json TEXT NOT NULL DEFAULT '[]',
            should_trigger INTEGER,
            enabled INTEGER NOT NULL DEFAULT 1,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            UNIQUE (suite_id, eval_id)
        );
        CREATE TABLE IF NOT EXISTS runs (
            id TEXT PRIMARY KEY,
            suite_id TEXT NOT NULL REFERENCES test_suites(id) ON DELETE CASCADE,
            iteration INTEGER NOT NULL,
            version_label TEXT,
            parent_run_id TEXT REFERENCES runs(id) ON DELETE SET NULL,
            skill_content_hash TEXT,
            executor_provider_id TEXT REFERENCES provider_configs(id) ON DELETE SET NULL,
            executor_model TEXT,
            grader_provider_id TEXT REFERENCES provider_configs(id) ON DELETE SET NULL,
            grader_model TEXT,
            configurations_json TEXT NOT NULL,
            runs_per_case INTEGER NOT NULL DEFAULT 1,
            concurrency INTEGER NOT NULL DEFAULT 2,
            mode TEXT NOT NULL DEFAULT 'live',
            seed INTEGER,
            budget_usd REAL,
            status TEXT NOT NULL,
            workspace_dir TEXT NOT NULL,
            benchmark_path TEXT,
            grading_result TEXT,
            is_current_best INTEGER NOT NULL DEFAULT 0,
            pass_rate_mean REAL,
            total_tokens INTEGER,
            total_cost_usd REAL,
            error TEXT,
            queued_at INTEGER NOT NULL,
            started_at INTEGER,
            finished_at INTEGER
        );
        CREATE TABLE IF NOT EXISTS run_cases (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL REFERENCES runs(id) ON DELETE CASCADE,
            test_case_id TEXT NOT NULL REFERENCES test_cases(id) ON DELETE CASCADE,
            configuration TEXT NOT NULL,
            run_number INTEGER NOT NULL,
            status TEXT NOT NULL,
            sandbox_dir TEXT,
            outputs_dir TEXT,
            transcript_path TEXT,
            command_line TEXT,
            exit_code INTEGER,
            started_at INTEGER,
            finished_at INTEGER,
            duration_ms INTEGER,
            input_tokens INTEGER,
            output_tokens INTEGER,
            cache_read_tokens INTEGER,
            cache_write_tokens INTEGER,
            cost_usd REAL,
            total_tool_calls INTEGER,
            total_steps INTEGER,
            errors_encountered INTEGER,
            output_chars INTEGER,
            transcript_chars INTEGER,
            tool_calls_json TEXT,
            files_created_json TEXT,
            triggered INTEGER,
            pass_count INTEGER,
            fail_count INTEGER,
            pass_rate REAL,
            recording_key TEXT,
            error TEXT,
            UNIQUE (run_id, test_case_id, configuration, run_number)
        );
        CREATE TABLE IF NOT EXISTS run_steps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            run_case_id TEXT NOT NULL REFERENCES run_cases(id) ON DELETE CASCADE,
            seq INTEGER NOT NULL,
            kind TEXT NOT NULL,
            tool_name TEXT,
            tool_use_id TEXT,
            is_error INTEGER NOT NULL DEFAULT 0,
            ts INTEGER NOT NULL,
            duration_ms INTEGER,
            input_tokens INTEGER,
            output_tokens INTEGER,
            preview TEXT,
            payload_offset INTEGER NOT NULL,
            payload_len INTEGER NOT NULL,
            UNIQUE (run_case_id, seq)
        );
        CREATE INDEX IF NOT EXISTS run_steps_tool ON run_steps(run_case_id, tool_name);
        CREATE TABLE IF NOT EXISTS grades (
            id TEXT PRIMARY KEY,
            run_case_id TEXT NOT NULL REFERENCES run_cases(id) ON DELETE CASCADE,
            expectation_index INTEGER NOT NULL,
            text TEXT NOT NULL,
            passed INTEGER NOT NULL,
            evidence TEXT NOT NULL,
            method TEXT NOT NULL DEFAULT 'llm',
            grader_provider_id TEXT REFERENCES provider_configs(id) ON DELETE SET NULL,
            grader_model TEXT,
            grader_duration_ms INTEGER,
            grader_cost_usd REAL,
            graded_at INTEGER NOT NULL,
            UNIQUE (run_case_id, expectation_index, method)
        );
        CREATE TABLE IF NOT EXISTS claims (
            id TEXT PRIMARY KEY,
            run_case_id TEXT NOT NULL REFERENCES run_cases(id) ON DELETE CASCADE,
            claim TEXT NOT NULL,
            type TEXT NOT NULL,
            verified INTEGER NOT NULL,
            evidence TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

// ── Helpers ──

fn add_column_if_missing(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<()> {
    // Validate identifiers to prevent SQL injection if call sites ever change.
    validate_identifier(table)?;
    validate_identifier(column)?;

    if !has_column(conn, table, column)? {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
            [],
        )?;
    }
    Ok(())
}

fn validate_identifier(name: &str) -> Result<()> {
    if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        anyhow::bail!("Invalid SQL identifier: {}", name);
    }
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns: Vec<String> = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(columns.iter().any(|name| name == column))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_database_migrates_to_latest() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        run_migrations(&conn).unwrap();

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, LATEST_VERSION);

        // Verify tables exist
        let tables: Vec<String> = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"skills".to_string()));
        assert!(tables.contains(&"skill_targets".to_string()));
        assert!(tables.contains(&"scenarios".to_string()));
        assert!(tables.contains(&"projects".to_string()));
        assert!(tables.contains(&"skill_tags".to_string()));
        assert!(tables.contains(&"scenario_skill_tools".to_string()));
        assert!(tables.contains(&"audit_log".to_string()));
        assert!(tables.contains(&"agents".to_string()));
        assert!(tables.contains(&"agent_targets".to_string()));
        assert!(tables.contains(&"provider_configs".to_string()));
        assert!(tables.contains(&"model_pricing".to_string()));
        assert!(tables.contains(&"usage_ledger".to_string()));
        assert!(tables.contains(&"test_suites".to_string()));
        assert!(tables.contains(&"runs".to_string()));
        assert!(tables.contains(&"grades".to_string()));
    }

    #[test]
    fn test_idempotent_migration() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        run_migrations(&conn).unwrap();
        // Running again should be a no-op
        run_migrations(&conn).unwrap();

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, LATEST_VERSION);
    }

    #[test]
    fn test_pre_migration_database_upgrades() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        // Simulate a pre-migration database: create skills table without newer columns
        conn.execute_batch(
            "
            CREATE TABLE skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                source_type TEXT NOT NULL,
                source_ref TEXT,
                source_revision TEXT,
                central_path TEXT NOT NULL UNIQUE,
                content_hash TEXT,
                enabled INTEGER DEFAULT 1,
                created_at INTEGER,
                updated_at INTEGER,
                status TEXT DEFAULT 'ok'
            );
            CREATE TABLE scenarios (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            );
            ",
        )
        .unwrap();

        // user_version is 0 (default), so migration should run
        run_migrations(&conn).unwrap();

        // Verify new columns were added
        assert!(has_column(&conn, "skills", "source_ref_resolved").unwrap());
        assert!(has_column(&conn, "skills", "source_subpath").unwrap());
        assert!(has_column(&conn, "skills", "source_branch").unwrap());
        assert!(has_column(&conn, "skills", "remote_revision").unwrap());
        assert!(has_column(&conn, "skills", "update_status").unwrap());
        assert!(has_column(&conn, "skills", "last_checked_at").unwrap());
        assert!(has_column(&conn, "skills", "last_check_error").unwrap());
        assert!(has_column(&conn, "scenarios", "icon").unwrap());

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, LATEST_VERSION);
    }

    #[test]
    fn test_v1_database_upgrades_to_v2() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();

        conn.execute_batch(
            "
            CREATE TABLE skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                source_type TEXT NOT NULL,
                source_ref TEXT,
                source_ref_resolved TEXT,
                source_subpath TEXT,
                source_branch TEXT,
                source_revision TEXT,
                remote_revision TEXT,
                central_path TEXT NOT NULL UNIQUE,
                content_hash TEXT,
                enabled INTEGER DEFAULT 1,
                created_at INTEGER,
                updated_at INTEGER,
                status TEXT DEFAULT 'ok',
                update_status TEXT DEFAULT 'unknown',
                last_checked_at INTEGER,
                last_check_error TEXT
            );
            CREATE TABLE scenarios (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                icon TEXT,
                sort_order INTEGER DEFAULT 0,
                created_at INTEGER,
                updated_at INTEGER
            );
            CREATE TABLE scenario_skills (
                scenario_id TEXT NOT NULL REFERENCES scenarios(id) ON DELETE CASCADE,
                skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
                added_at INTEGER,
                PRIMARY KEY(scenario_id, skill_id)
            );
            CREATE TABLE skill_targets (
                id TEXT PRIMARY KEY,
                skill_id TEXT NOT NULL REFERENCES skills(id) ON DELETE CASCADE,
                tool TEXT NOT NULL,
                target_path TEXT NOT NULL,
                mode TEXT NOT NULL,
                status TEXT DEFAULT 'ok',
                synced_at INTEGER,
                last_error TEXT,
                UNIQUE(skill_id, tool)
            );
            PRAGMA user_version = 1;
            ",
        )
        .unwrap();

        run_migrations(&conn).unwrap();
        assert!(has_column(&conn, "scenario_skill_tools", "enabled").unwrap());
        assert!(has_column(&conn, "skill_targets", "source_hash").unwrap());

        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, LATEST_VERSION);
    }

    #[test]
    fn orphaned_default_export_agents_setting_is_dropped() {
        let conn = Connection::open_in_memory().unwrap();
        // Arrive at v7 the way a real upgrading database does, then plant the
        // row that the removed UI used to write.
        conn.pragma_update(None, "user_version", 0).unwrap();
        run_migrations(&conn).unwrap();
        conn.pragma_update(None, "user_version", 7).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('project_default_export_agents', ?1)",
            ["[\"claude_code\",\"codex\"]"],
        )
        .unwrap();
        assert_eq!(
            count_setting(&conn),
            1,
            "precondition: the row must exist, or this test proves nothing"
        );

        run_migrations(&conn).unwrap();

        assert_eq!(
            count_setting(&conn),
            0,
            "v7→v8 must delete the orphaned preference"
        );
        // Unrelated settings must survive.
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('theme', 'dark')",
            [],
        )
        .unwrap();
        run_migrations(&conn).unwrap();
        let theme: String = conn
            .query_row("SELECT value FROM settings WHERE key = 'theme'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(theme, "dark");
    }

    fn count_setting(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT COUNT(*) FROM settings WHERE key = 'project_default_export_agents'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn test_newer_schema_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "user_version", LATEST_VERSION + 1)
            .unwrap();

        let err = run_migrations(&conn).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("newer than this app supports"),
            "unexpected error: {msg}"
        );
    }
}
