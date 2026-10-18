//! Test engine: suites, sandboxes, executors, transcripts, grading, and replay.

mod aggregate;
mod checks;
mod effectiveness;
mod engine;
mod suite;
mod trigger;

pub use aggregate::{aggregate_benchmark, delta_string, Stat};
pub use checks::{run_check, CheckResult, DeterministicCheck};
pub use effectiveness::{
    apply_description, benchmark_notes, bisect_distractors, blind_compare, calibrate_gold,
    classify_activation, conflict_matrix, export_snapshot, import_snapshot, influence_map,
    install_warnings, interview_goal, letter_grade, optimize_descriptions, radar_axes,
    rank_leaderboard, run_cli_trigger, run_measured, send_prompt, send_templates, snapshot_skill,
    static_overlap, strong_model_only, suggest_description, write_default_suite, ActivationRow,
    Axis, BlindRequest, GoldLabel, LibrarySkill, MeasureRequest, ModelChoice, OverlapHit, RankRow,
    SkillScore,
};
pub use engine::{
    materialize_fixture, persist_step_metrics, resume_run, run_plan, run_plan_with_clock,
    run_version_comparison, select_fixture_file, CellReport, EngineClock, Plan, RunReport,
};
pub use suite::{
    fix_skill_name, load_suite, save_suite, CheckSpec, Expectation, Suite, SuiteCase, TriggerRow,
};
pub use trigger::{
    agree_with_python, cleanup_command_file, judge_stream, run_trigger_set, spawn_trigger_queries,
    TriggerQuery,
};

#[cfg(test)]
mod tests;

pub fn python_bin() -> std::process::Command {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
    {
        std::process::Command::new("python3")
    } else {
        std::process::Command::new("python")
    }
}
