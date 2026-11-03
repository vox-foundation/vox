//! Opt-in webhook intake (Phase 3 D-04).
//!
//! An `[orchestrator.webhook]` section present in `Vox.toml` means: vox-orchestrator-mcp
//! loads the webhook plugin, starts its listener and polls its events into the hopper.
//! Absent means nothing happens — no plugin load, no listener. The ingress token is the
//! `WebhookIngressToken` secret (`VOX_WEBHOOK_INGRESS_TOKEN`), never a config value.

use serde::{Deserialize, Serialize};

fn default_poll_interval_ms() -> u64 {
    2_000
}

fn default_max_events_per_poll() -> u32 {
    64
}

/// `[orchestrator.webhook]`: presence opts in to webhook -> hopper intake.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WebhookIntakeConfig {
    /// Listener bind address passed to the plugin (`addr`); `None` keeps the plugin's default.
    pub bind_addr: Option<String>,
    /// Interval between inbox polls (milliseconds). Clamped to ≥ 250 at the spawn site.
    #[serde(default = "default_poll_interval_ms")]
    pub poll_interval_ms: u64,
    /// Most events drained per poll. Clamped to 1..=1024 at the spawn site.
    #[serde(default = "default_max_events_per_poll")]
    pub max_events_per_poll: u32,
}

impl Default for WebhookIntakeConfig {
    fn default() -> Self {
        Self {
            bind_addr: None,
            poll_interval_ms: default_poll_interval_ms(),
            max_events_per_poll: default_max_events_per_poll(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::OrchestratorConfig;

    #[test]
    fn webhook_section_absent_means_none() {
        assert!(OrchestratorConfig::default().webhook.is_none());
        let parsed: OrchestratorConfig = toml::from_str("").expect("empty config");
        assert!(parsed.webhook.is_none());
    }

    #[test]
    fn empty_webhook_section_opts_in_with_defaults() {
        let parsed: OrchestratorConfig = toml::from_str("[webhook]\n").expect("parse");
        let w = parsed.webhook.expect("section present => Some");
        assert_eq!(w.bind_addr, None);
        assert_eq!(w.poll_interval_ms, 2000);
        assert_eq!(w.max_events_per_poll, 64);
    }

    #[test]
    fn webhook_section_fields_parse() {
        let parsed: OrchestratorConfig = toml::from_str(
            "[webhook]\nbind_addr = \"127.0.0.1:9080\"\npoll_interval_ms = 500\nmax_events_per_poll = 8\n",
        )
        .expect("parse");
        let w = parsed.webhook.expect("Some");
        assert_eq!(w.bind_addr.as_deref(), Some("127.0.0.1:9080"));
        assert_eq!(w.poll_interval_ms, 500);
        assert_eq!(w.max_events_per_poll, 8);
    }
}
