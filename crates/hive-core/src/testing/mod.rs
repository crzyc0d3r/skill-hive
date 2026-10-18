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
    grading_path, history_entries, interview_goal, load_effectiveness, run_effectiveness,
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
