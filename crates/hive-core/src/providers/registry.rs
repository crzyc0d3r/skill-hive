//! Turn stored provider configurations into providers, resolve their API keys and probe them.

use std::sync::Arc;

use super::{
    AnthropicProvider, ClaudeCodeProvider, CodexProvider, OpenAiCompatProvider, Provider,
    ScriptedProvider,
};
use crate::error::Result;
use crate::model::{ProviderConfig, ProviderKind, ProviderProbe};
use crate::secrets::SecretStore;

/// Name of a provider's entry in the secret store.
pub fn secret_name(provider_id: &str) -> String {
    format!("provider:{provider_id}")
}

/// The API key for a provider: the secret store entry, else the environment variable named by
/// `api_key_env`, else the conventional variable for the kind (`ANTHROPIC_API_KEY`,
/// `OPENAI_API_KEY`). `None` when nothing is set (fine for local OpenAI-compatible servers).
pub fn resolve_api_key(config: &ProviderConfig, secrets: &SecretStore) -> Option<String> {
    if let Ok(Some(key)) = secrets.get(&secret_name(&config.id)) {
        if !key.trim().is_empty() {
            return Some(key.trim().to_string());
        }
    }
    let from_env = |var: &str| {
        std::env::var(var.trim())
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    if let Some(var) = config
        .api_key_env
        .as_deref()
        .filter(|v| !v.trim().is_empty())
    {
        if let Some(key) = from_env(var) {
            return Some(key);
        }
    }
    match config.kind {
        ProviderKind::Anthropic => from_env("ANTHROPIC_API_KEY"),
        ProviderKind::OpenaiCompatible => from_env("OPENAI_API_KEY"),
        _ => None,
    }
}

/// Build a provider for a configuration. CLI providers never fail here (a missing binary shows
/// up when they run); HTTP providers fail only on an unusable client; the scripted provider
/// fails when its script is missing or invalid.
pub fn build_provider(config: &ProviderConfig, secrets: &SecretStore) -> Result<Arc<dyn Provider>> {
    Ok(match config.kind {
        ProviderKind::ClaudeCode => Arc::new(ClaudeCodeProvider::new(config.clone())),
        ProviderKind::Codex => Arc::new(CodexProvider::new(config.clone())),
        ProviderKind::Anthropic => Arc::new(AnthropicProvider::new(
            config.clone(),
            resolve_api_key(config, secrets),
        )?),
        ProviderKind::OpenaiCompatible => Arc::new(OpenAiCompatProvider::new(
            config.clone(),
            resolve_api_key(config, secrets),
        )?),
        ProviderKind::Scripted => Arc::new(ScriptedProvider::new(config.clone())?),
    })
}

/// Check that a provider answers: `--version` for CLIs, the model list for HTTP APIs, the
/// script file for the scripted provider. Never fails; problems are reported in the probe.
pub async fn probe(config: &ProviderConfig, secrets: &SecretStore) -> ProviderProbe {
    match build_provider(config, secrets) {
        Ok(provider) => provider.probe().await,
        Err(e) => ProviderProbe {
            ok: false,
            latency_ms: 0,
            message: e.to_string(),
            version: None,
            checked_at: crate::util::now_ms(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::HivePaths;

    #[test]
    fn api_key_resolution_order() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path());
        let secrets = SecretStore::file_only(&paths);
        let mut cfg = super::super::scripted::test_support::config("k", tmp.path());
        cfg.kind = ProviderKind::OpenaiCompatible;
        cfg.api_key_env = Some("HIVE_TEST_KEY_VAR_THAT_IS_NOT_SET".into());
        // Nothing anywhere (the conventional variable may be set on a developer machine, so only
        // assert the store wins once it is populated).
        secrets.set(&secret_name("k"), " from-store ").unwrap();
        assert_eq!(
            resolve_api_key(&cfg, &secrets).as_deref(),
            Some("from-store")
        );
        secrets.delete(&secret_name("k")).unwrap();
        let resolved = resolve_api_key(&cfg, &secrets);
        assert_eq!(
            resolved,
            std::env::var("OPENAI_API_KEY")
                .ok()
                .filter(|v| !v.trim().is_empty())
        );
    }

    #[tokio::test]
    async fn probe_reports_missing_script() {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path());
        let secrets = SecretStore::file_only(&paths);
        let cfg =
            super::super::scripted::test_support::config("s", &tmp.path().join("missing.json"));
        let probe = probe(&cfg, &secrets).await;
        assert!(!probe.ok);
        assert!(probe.message.contains("missing.json"), "{}", probe.message);
    }
}
