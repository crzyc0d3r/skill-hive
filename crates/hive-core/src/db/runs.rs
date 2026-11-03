//! Evaluation run persistence.

use rusqlite::{OptionalExtension, Row, params};

use crate::error::{HiveError, Result};
use crate::model::{
    Benchmark, CaseResult, EvalRun, HistoryPoint, RunConfig, RunProgress, RunStatus,
};

use super::Database;

const COLUMNS: &str = "id, item_id, skill_name, skill_hash, config_json, status, progress_json, workspace, summary_json, error, created_at, started_at, finished_at";

fn row_to_run(row: &Row<'_>) -> rusqlite::Result<EvalRun> {
    let config: String = row.get(4)?;
    let status: String = row.get(5)?;
    let progress: String = row.get(6)?;
    let summary: Option<String> = row.get(8)?;
    Ok(EvalRun {
        id: row.get(0)?,
        item_id: row.get(1)?,
        skill_name: row.get(2)?,
        skill_hash: row.get(3)?,
        config: serde_json::from_str::<RunConfig>(&config).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?,
        status: RunStatus::parse(&status),
        progress: serde_json::from_str::<RunProgress>(&progress).unwrap_or_default(),
        workspace: row.get(7)?,
        summary: summary.and_then(|s| serde_json::from_str::<Benchmark>(&s).ok()),
        error: row.get(9)?,
        created_at: row.get(10)?,
        started_at: row.get(11)?,
        finished_at: row.get(12)?,
    })
}

impl Database {
    pub fn insert_run(&self, run: &EvalRun) -> Result<()> {
        self.with(|c| {
            c.execute(
                "INSERT INTO eval_runs(id, item_id, skill_name, skill_hash, config_json, status, progress_json, workspace, summary_json, error, created_at, started_at, finished_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    run.id,
                    run.item_id,
                    run.skill_name,
                    run.skill_hash,
                    serde_json::to_string(&run.config)?,
                    run.status.as_str(),
                    serde_json::to_string(&run.progress)?,
                    run.workspace,
                    run.summary.as_ref().map(serde_json::to_string).transpose()?,
                    run.error,
                    run.created_at,
                    run.started_at,
                    run.finished_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn update_run(&self, run: &EvalRun) -> Result<()> {
        self.with(|c| {
            c.execute(
                "UPDATE eval_runs SET status = ?2, progress_json = ?3, summary_json = ?4, error = ?5, started_at = ?6, finished_at = ?7 WHERE id = ?1",
                params![
                    run.id,
                    run.status.as_str(),
                    serde_json::to_string(&run.progress)?,
                    run.summary.as_ref().map(serde_json::to_string).transpose()?,
                    run.error,
                    run.started_at,
                    run.finished_at,
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_run(&self, id: &str) -> Result<Option<EvalRun>> {
        self.with(|c| {
            let sql = format!("SELECT {COLUMNS} FROM eval_runs WHERE id = ?1");
            Ok(c.query_row(&sql, params![id], row_to_run).optional()?)
        })
    }

    pub fn require_run(&self, id: &str) -> Result<EvalRun> {
        self.get_run(id)?
            .ok_or_else(|| HiveError::not_found(format!("run {id}")))
    }

    pub fn list_runs(&self, item_id: Option<&str>, limit: u32) -> Result<Vec<EvalRun>> {
        self.with(|c| {
            let sql = format!(
                "SELECT {COLUMNS} FROM eval_runs WHERE (?1 IS NULL OR item_id = ?1) ORDER BY created_at DESC LIMIT ?2"
            );
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map(params![item_id, limit as i64], row_to_run)?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }

    /// Runs that were left non-terminal by a previous process (crash, force quit).
    pub fn stale_runs(&self) -> Result<Vec<EvalRun>> {
        self.with(|c| {
            let sql = format!(
                "SELECT {COLUMNS} FROM eval_runs WHERE status IN ('queued','running','grading')"
            );
            let mut stmt = c.prepare(&sql)?;
            let rows = stmt.query_map([], row_to_run)?;
            Ok(rows.collect::<std::result::Result<_, _>>()?)
        })
    }

    pub fn delete_run(&self, id: &str) -> Result<bool> {
        self.with(|c| Ok(c.execute("DELETE FROM eval_runs WHERE id = ?1", params![id])? > 0))
    }

    pub fn upsert_case_result(&self, result: &CaseResult) -> Result<()> {
        self.with(|c| {
            c.execute(
                "INSERT INTO eval_case_results(id, run_id, case_id, arm, repeat, result_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(id) DO UPDATE SET result_json = excluded.result_json",
                params![
                    result.id,
                    result.run_id,
                    result.case_id as i64,
                    result.arm,
                    result.repeat as i64,
                    serde_json::to_string(result)?
                ],
            )?;
            Ok(())
        })
    }

    pub fn case_results(&self, run_id: &str) -> Result<Vec<CaseResult>> {
        self.with(|c| {
            let mut stmt = c.prepare("SELECT result_json FROM eval_case_results WHERE run_id = ?1 ORDER BY case_id, arm, repeat")?;
            let rows = stmt.query_map(params![run_id], |r| r.get::<_, String>(0))?;
            let mut out = Vec::new();
            for row in rows {
                out.push(serde_json::from_str(&row?)?);
            }
            Ok(out)
        })
    }

    /// Effectiveness history for an item from finished runs with a benchmark.
    pub fn history_for_item(&self, item_id: &str, limit: u32) -> Result<Vec<HistoryPoint>> {
        let runs = self.list_runs(Some(item_id), limit)?;
        let mut points = Vec::new();
        for run in runs.into_iter().filter(|r| r.status == RunStatus::Done) {
            let Some(summary) = &run.summary else {
                continue;
            };
            let with = summary.arms.iter().find(|a| a.label == "with_skill");
            let base = summary.arms.iter().find(|a| a.label == summary.baseline);
            let (Some(with), Some(base)) = (with, base) else {
                continue;
            };
            points.push(HistoryPoint {
                run_id: run.id.clone(),
                finished_at: run.finished_at.unwrap_or(run.created_at),
                skill_hash: run.skill_hash.clone(),
                executor: summary.executor.clone(),
                verdict: summary.verdict,
                with_skill_pass_rate: with.pass_rate.mean,
                baseline_pass_rate: base.pass_rate.mean,
                delta: with.pass_rate.mean - base.pass_rate.mean,
                with_skill_tokens: with.tokens.mean,
                baseline_tokens: base.tokens.mean,
            });
        }
        points.sort_by_key(|p| p.finished_at);
        Ok(points)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::items::sample_item;
    use crate::model::{ArmKind, ArmSpec, CaseStatus};

    fn sample_run(id: &str, item: &str) -> EvalRun {
        EvalRun {
            id: id.into(),
            item_id: Some(item.into()),
            skill_name: "a".into(),
            skill_hash: "h".into(),
            config: RunConfig {
                item_id: item.into(),
                arms: vec![
                    ArmSpec {
                        label: "with_skill".into(),
                        kind: ArmKind::WithSkill,
                    },
                    ArmSpec {
                        label: "without_skill".into(),
                        kind: ArmKind::WithoutSkill,
                    },
                ],
                case_ids: vec![],
                repeats: 1,
                concurrency: 1,
                executor_provider_id: "p".into(),
                grader_provider_id: None,
                timeout_secs: 60,
                include_trigger: false,
                baseline_label: None,
            },
            status: RunStatus::Queued,
            progress: RunProgress::default(),
            workspace: "/w".into(),
            summary: None,
            error: None,
            created_at: 1,
            started_at: None,
            finished_at: None,
        }
    }

    #[test]
    fn run_round_trip() {
        let db = Database::open_memory().unwrap();
        db.insert_item(&sample_item(
            "1",
            "a",
            "alpha skill for testing things",
            "body",
            &[],
        ))
        .unwrap();
        let mut run = sample_run("r1", "1");
        db.insert_run(&run).unwrap();
        run.status = RunStatus::Running;
        run.progress.total = 4;
        db.update_run(&run).unwrap();
        let loaded = db.get_run("r1").unwrap().unwrap();
        assert_eq!(loaded.status, RunStatus::Running);
        assert_eq!(loaded.progress.total, 4);
        assert_eq!(loaded.config.arms.len(), 2);
        assert_eq!(db.stale_runs().unwrap().len(), 1);

        let result = CaseResult {
            id: "c1".into(),
            run_id: "r1".into(),
            case_id: 1,
            case_name: "n".into(),
            arm: "with_skill".into(),
            repeat: 1,
            status: CaseStatus::Graded,
            expectations: vec![],
            passed: 1,
            failed: 0,
            total: 1,
            pass_rate: 1.0,
            input_tokens: 10,
            output_tokens: 5,
            duration_ms: 100,
            tool_calls: 2,
            cost_usd: None,
            output_excerpt: "ok".into(),
            sandbox_path: "/s".into(),
            transcript_path: None,
            grader_notes: None,
            error: None,
        };
        db.upsert_case_result(&result).unwrap();
        db.upsert_case_result(&result).unwrap();
        assert_eq!(db.case_results("r1").unwrap().len(), 1);
        assert_eq!(db.list_runs(Some("1"), 10).unwrap().len(), 1);
        assert!(db.history_for_item("1", 10).unwrap().is_empty());
        assert!(db.delete_run("r1").unwrap());
        assert!(db.case_results("r1").unwrap().is_empty());
    }
}
