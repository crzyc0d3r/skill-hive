//! Generation analytics: optimization, blind comparison, regression, history, and exports.
//! Recorded fixtures only. This module does not call a live model.

mod compare;
mod export;
mod history;
mod optimize;
mod regression;
mod workflow;

pub use compare::{
    record_verdict, start_analysis, start_comparison, store_verdict, Comparison, Side,
};
pub use export::{
    export_workspace, CheckExport, ExpectationExport, ExportCell, ExportCounts, ExportSlice,
};
pub use history::{write_history, HistoryEntry};
pub use optimize::{apply_optimized, optimize, Candidate, OptimizeOutcome, QueryScore};
pub use regression::{
    evaluate, set_baseline, Alert, ExpectationOutcome, RegressionReport, RunSlice, SampleCell,
};
pub use workflow::{demonstrate_d1, demonstrate_d2, demonstrate_d3, WorkflowReport};

#[cfg(test)]
mod tests;
