//! Shared [`reqwest::Client`] / [`reqwest::ClientBuilder`] presets for Vox outbound HTTP.
//!
//! **Policy:** see `docs/src/architecture/outbound-http-policy.md` in the repo for when to use this crate, migration order, and exceptions.

use std::time::Duration;

fn default_user_agent() -> String {
    format!(
        "vox-reqwest-defaults/{}",
        option_env!("CARGO_PKG_VERSION").unwrap_or("0.0.0")
    )
}

/// Builder with user-agent, connect timeout, and idle pool cap suitable for CLI and services.
pub fn client_builder() -> reqwest::ClientBuilder {
    reqwest::Client::builder()
        .user_agent(default_user_agent())
        .connect_timeout(Duration::from_secs(15))
        .pool_idle_timeout(Duration::from_secs(90))
}

/// Fall-back client when a custom builder chain fails to [`build`](reqwest::ClientBuilder::build).
pub fn client() -> reqwest::Client {
    client_builder()
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
