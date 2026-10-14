//! Skill and agent authoring: frontmatter, validation, drafts, and versions.
//! Phase 4 (providers and the test engine) is intentionally not implemented here.

pub mod agents;
pub mod frontmatter;

pub mod skill;
#[cfg(test)]
mod tests;
pub mod validator;
pub mod versions;

pub use agents::{
    agent_description_budget, apply_agent_fix, export_agent, export_status, render_agent_file,
    render_agents_cli_json, validate_agent_file, AgentExportRequest,
};
pub use frontmatter::{estimate_tokens, parse_document, Document};
pub use skill::{
    apply_skill_fix, deployment_status, instantiate_template, plan_publish, publish_draft,
    template_ids, BuiltInTemplate, Deployment, PublishReport,
};
pub use validator::{
    validate_agent, validate_command_file, validate_skill, Finding, Profile, Severity,
};
pub use versions::{
    export_history_json, hide_version, label_version, restore_version, save_autosave,
    set_current_best, snapshot_version, VersionStore,
};
