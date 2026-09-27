// Correct Clavis call sites (the shapes landed in d70ea497a). `secret-env-guard` must pass this
// file: calling vox_secrets, naming a SecretId, or saying "secret" in a message is not a leak.
use vox_secrets::{SecretId, resolve_secret, store_secret};

fn research_repo_root() -> std::path::PathBuf {
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxRepositoryRoot)
        .expose()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
}

fn save_key(provider: &str, value: &str) -> Result<(), String> {
    store_secret(SecretId::TavilyApiKey, value, None)
        .map_err(|e| format!("Failed to write secret for {provider}: {e}"))
}

fn tavily_configured() -> bool {
    resolve_secret(SecretId::TavilyApiKey).is_present()
}

fn describe(user: &str) -> String {
    let body = serde_json::json!({ "user": user, "lane": "deep" });
    tracing::info!("saved engine config for {user}");
    format!("system context for {user}: {body}")
}
