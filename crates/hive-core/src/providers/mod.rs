//! Provider layer: one interface for Anthropic, OpenAI-compatible endpoints, and local agent CLIs.

mod cli_agent;
mod http;
mod middleware;
mod pricing;
mod probe;
mod scripted;
mod secrets;
mod store;
mod types;

pub use cli_agent::{
    detect_local_agent_clis, help_flags, kill_tree, spawn_grouped, CliEvent, LocalCli,
};
pub use http::{AnthropicProvider, OpenAiProvider};
pub use middleware::{
    recording_diff, recording_exists, request_key, Limiter, Middleware, NoopSleeper, RealSleeper,
    Recording, RunMode, Sleeper,
};
pub use pricing::{cost_usd, lookup, seed_models, Pricing};
pub use probe::{probe_offline, probe_provider, ProbeStage};
pub use scripted::ScriptedProvider;
pub use secrets::{
    clear_git_token, clear_provider_secret, has_git_token, has_provider_secret, load_env_secret,
    load_provider_secret, store_git_token, store_provider_secret, SecretBackend,
};
pub use store::{
    get_provider, list_named_providers, list_providers, provider_public_json, referenced_by_preset,
    save_named_provider, set_last_test, upsert_provider, ProviderConfig, ProviderPublic,
};
pub use types::*;

#[cfg(test)]
mod tests;

pub fn scrub(input: &str, secrets: &[String]) -> String {
    let mut out = crate::util::log_sanitize::sanitize(input);
    for secret in secrets {
        if secret.len() >= 4 && out.contains(secret.as_str()) {
            out = out.replace(secret.as_str(), "<redacted>");
        }
    }
    out
}
