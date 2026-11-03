//! Opt-in webhook -> hopper intake (Phase 3 D-03/D-04/D-10/D-13).
//!
//! vox-plugin-webhook accepts authenticated webhook deliveries and queues them;
//! the host drains them through the plugin's `WebhookInbox` poll extension as
//! JSON strings. This module turns each string into a `HopperIntake` item with
//! `IntakeSource::Webhook`. The JSON field names (`id`, `source`, `event_type`)
//! are the plugin's `WebhookEvent` serde names and are the whole contract across
//! the boundary — no crate here depends on the plugin.

use std::sync::Arc;
use vox_orchestrator::config::WebhookIntakeConfig;
use vox_orchestrator::hopper::{HopperIntake, IntakeSource, PriorityHint};

/// Longest slice of any untrusted event field that reaches the hopper intent.
const MAX_FIELD_CHARS: usize = 64;
/// Floor for `poll_interval_ms` (bounds intake per second).
const MIN_POLL_INTERVAL_MS: u64 = 250;
/// Ceiling for `max_events_per_poll` (bounds intake per tick).
const MAX_EVENTS_PER_POLL: u32 = 1024;

// Copied instead of depending on the plugin: vox-plugin-webhook is an L4
// plugin, and D-10 forbids a crate edge to it — the host sees only the JSON
// the `WebhookInbox` extension returns. Keep this table in step with
// `crates/vox-plugin-webhook/src/webhook/bridge.rs`.
// vox:defactored-from vox-plugin-webhook 2026-09-25 (webhook::bridge::OrchestratorInboxItem::from_webhook, ~15 lines)
fn route_kind(source: &str, event_type: &str) -> &'static str {
    if source == "gitlab" {
        tracing::warn!(
            "received a GitLab webhook event; GitLab support is DEPRECATED (2026-06-03) \
             and no longer supported — this routing will be removed in a future release"
        );
    }
    match (source, event_type) {
        ("github" | "gitlab", "push" | "tag_push") => "git_push",
        ("github" | "gitlab", "pull_request" | "merge_request") => "pull_request",
        ("discord" | "slack", _) => "channel_message",
        _ => "external_event",
    }
}

/// Untrusted field -> at most [`MAX_FIELD_CHARS`] chars, control characters removed.
fn bounded(field: &str) -> String {
    field
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_FIELD_CHARS)
        .collect()
}

/// The subset of the plugin's `WebhookEvent` the host reads. `payload` is
/// deliberately absent: the body is attacker-controlled and never reaches an
/// intent that agents later read (prompt-injection surface).
#[derive(serde::Deserialize)]
struct PolledEvent {
    id: String,
    source: String,
    event_type: String,
}

/// One polled event JSON string -> `(intent, affinity_hints)`.
pub(crate) fn intake_from_event_json(json: &str) -> Result<(String, Vec<String>), String> {
    let ev: PolledEvent =
        serde_json::from_str(json).map_err(|e| format!("invalid webhook event JSON: {e}"))?;
    let kind = route_kind(&ev.source, &ev.event_type);
    let (id, source, event_type) = (
        bounded(&ev.id),
        bounded(&ev.source),
        bounded(&ev.event_type),
    );
    let intent = format!("webhook {kind}: {source}/{event_type} (delivery {id})");
    let hints = vec![format!("webhook:{source}"), format!("webhook-kind:{kind}")];
    Ok((intent, hints))
}

/// Submit each polled event to the hopper as `IntakeSource::Webhook`; malformed
/// events are logged and skipped. Returns the number submitted.
pub(crate) async fn submit_webhook_events(hopper: &dyn HopperIntake, events: Vec<String>) -> usize {
    let mut submitted = 0;
    for json in events {
        match intake_from_event_json(&json) {
            Ok((intent, hints)) => {
                hopper
                    .submit(
                        intent,
                        hints,
                        PriorityHint::Unspecified,
                        IntakeSource::Webhook,
                        None,
                    )
                    .await;
                submitted += 1;
            }
            // The raw string is not echoed: it is untrusted input.
            Err(e) => tracing::warn!(error = %e, "skipping malformed webhook event"),
        }
    }
    submitted
}

/// What [`spawn_webhook_intake_poller`] does once the opt-in checks pass.
/// Deliberately not `Debug`: `config_json` carries the ingress token.
pub(crate) struct IntakePlan {
    config_json: String,
    interval: std::time::Duration,
    max_events: u32,
}

/// D-04/D-14: no `[orchestrator.webhook]` section -> `None` before any secret
/// read; a missing or blank ingress token -> `None` (fail closed).
pub(crate) fn plan_webhook_intake(
    cfg: Option<&WebhookIntakeConfig>,
    resolve_token: impl FnOnce() -> Option<String>,
) -> Option<IntakePlan> {
    let cfg = cfg?;
    let Some(ingress) = resolve_token().filter(|t| !t.trim().is_empty()) else {
        tracing::error!(
            "[orchestrator.webhook] is configured but VOX_WEBHOOK_INGRESS_TOKEN \
             (WebhookIngressToken) is not set; webhook intake stays off (fail closed)"
        );
        return None;
    };
    let mut start = serde_json::json!({ "ingress_token": ingress });
    if let Some(addr) = &cfg.bind_addr {
        start["addr"] = serde_json::Value::String(addr.clone());
    }
    Some(IntakePlan {
        config_json: start.to_string(),
        interval: std::time::Duration::from_millis(cfg.poll_interval_ms.max(MIN_POLL_INTERVAL_MS)),
        max_events: cfg.max_events_per_poll.clamp(1, MAX_EVENTS_PER_POLL),
    })
}

/// Start the webhook plugin's listener and poll its inbox into `hopper`.
/// Does nothing (returns `None`) unless [`plan_webhook_intake`] yields a plan and
/// a tokio runtime is running.
pub(crate) fn spawn_webhook_intake_poller(
    cfg: Option<&WebhookIntakeConfig>,
    hopper: Arc<dyn HopperIntake>,
) -> Option<tokio::task::JoinHandle<()>> {
    let plan = plan_webhook_intake(cfg, || {
        vox_secrets::resolve_secret(vox_secrets::SecretId::WebhookIngressToken)
            .expose()
            .map(str::to_owned)
    })?;
    let Ok(rt) = tokio::runtime::Handle::try_current() else {
        tracing::error!("webhook intake: no tokio runtime, poller not started");
        return None;
    };
    Some(rt.spawn(run_poller(plan, hopper)))
}

async fn run_poller(plan: IntakePlan, hopper: Arc<dyn HopperIntake>) {
    let IntakePlan {
        config_json,
        interval,
        max_events,
    } = plan;
    let loaded = match tokio::task::spawn_blocking(move || start_listener(&config_json)).await {
        Ok(Ok(loaded)) => loaded,
        Ok(Err(e)) => {
            tracing::error!(error = %e, "webhook intake disabled");
            return;
        }
        Err(e) => {
            tracing::error!(error = %e, "webhook intake: plugin start task failed");
            return;
        }
    };
    let mut tick = tokio::time::interval(interval);
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        tick.tick().await;
        // Owned Strings only: no FFI value is held across the await below.
        let polled: Result<Vec<String>, String> =
            match loaded.plugin.as_webhook_inbox().into_option() {
                Some(inbox) => inbox
                    .poll_events(max_events)
                    .into_result()
                    .map(|v| v.into_iter().map(|s| s.into_string()).collect())
                    .map_err(|e| e.to_string()),
                None => Err("webhook plugin has no WebhookInbox extension".into()),
            };
        match polled {
            Ok(events) => {
                submit_webhook_events(hopper.as_ref(), events).await;
            }
            Err(e) => tracing::warn!(error = %e, "webhook inbox poll failed"),
        }
    }
}

/// Blocking: discover + dlopen the webhook plugin and start its listener.
fn start_listener(config_json: &str) -> Result<&'static vox_plugin_host::LoadedCodePlugin, String> {
    // LoadError's Display already carries the install hint (D-02).
    let loaded = vox_plugin_host::cached_code_plugin("webhook").map_err(|e| e.to_string())?;
    if loaded.plugin.as_webhook_inbox().is_none() {
        return Err(
            "the installed webhook plugin predates ABI 13 (no WebhookInbox extension); \
             reinstall it"
                .into(),
        );
    }
    let listener = loaded
        .plugin
        .as_http_listener()
        .into_option()
        .ok_or("webhook plugin has no HttpListener extension")?;
    listener
        .start_listening(config_json.into())
        .into_result()
        .map_err(|e| format!("webhook listener refused to start: {e}"))?;
    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator::hopper::{HopperIntake, InMemoryHopper, IntakeSource};

    const GITHUB_PUSH: &str = r#"{"id":"d-1","source":"github","event_type":"push","payload":{"ref":"refs/heads/main"},"received_at":1}"#;

    #[tokio::test]
    async fn github_push_event_lands_in_hopper_as_webhook_intake() {
        let hopper = InMemoryHopper::headless();
        let n = submit_webhook_events(&hopper, vec![GITHUB_PUSH.to_string()]).await;
        assert_eq!(n, 1);
        let inbox = hopper.inbox().await;
        assert_eq!(inbox.len(), 1);
        let item = &inbox[0];
        assert_eq!(item.source, IntakeSource::Webhook);
        assert!(item.intent.contains("git_push"), "{}", item.intent);
        assert!(item.intent.contains("github/push"), "{}", item.intent);
        assert!(item.intent.contains("d-1"), "{}", item.intent);
        assert!(!item.intent.contains("refs/heads/main"), "payload leaked");
        assert!(item.affinity_hints.contains(&"webhook:github".to_string()));
        assert!(
            item.affinity_hints
                .contains(&"webhook-kind:git_push".to_string())
        );
    }

    #[test]
    fn route_kind_matches_the_plugin_bridge_table() {
        assert_eq!(route_kind("github", "push"), "git_push");
        assert_eq!(route_kind("gitlab", "tag_push"), "git_push");
        assert_eq!(route_kind("github", "pull_request"), "pull_request");
        assert_eq!(route_kind("gitlab", "merge_request"), "pull_request");
        assert_eq!(route_kind("discord", "anything"), "channel_message");
        assert_eq!(route_kind("slack", "anything"), "channel_message");
        assert_eq!(route_kind("stripe", "charge.succeeded"), "external_event");
    }

    #[tokio::test]
    async fn malformed_events_are_skipped_not_submitted() {
        let hopper = InMemoryHopper::headless();
        let events = vec![
            "not json".to_string(),
            "{\"id\":\"x\"}".to_string(),
            GITHUB_PUSH.to_string(),
        ];
        assert_eq!(submit_webhook_events(&hopper, events).await, 1);
        assert_eq!(hopper.inbox().await.len(), 1);
    }

    #[test]
    fn untrusted_fields_are_bounded_and_control_free() {
        let source = format!("{}\n\u{0}", "a".repeat(500));
        let json = serde_json::json!({
            "id": "d-2",
            "source": source,
            "event_type": "push",
            "payload": {},
            "received_at": 1
        })
        .to_string();
        let (intent, hints) = intake_from_event_json(&json).expect("valid event");
        assert!(!intent.chars().any(char::is_control), "{intent:?}");
        assert!(intent.chars().count() < 3 * 64 + 64, "{}", intent.len());
        assert!(!intent.contains(&"a".repeat(65)));
        assert!(hints.iter().all(|h| !h.chars().any(char::is_control)));
        assert!(hints.iter().all(|h| !h.contains(&"a".repeat(65))));
    }

    #[test]
    fn no_config_section_means_no_plan_and_no_token_read() {
        let plan = plan_webhook_intake(None, || {
            panic!("the token must not be resolved without an [orchestrator.webhook] section")
        });
        assert!(plan.is_none());
    }

    #[test]
    fn missing_or_blank_token_fails_closed() {
        let cfg = WebhookIntakeConfig::default();
        assert!(plan_webhook_intake(Some(&cfg), || None).is_none());
        assert!(plan_webhook_intake(Some(&cfg), || Some(String::new())).is_none());
        assert!(plan_webhook_intake(Some(&cfg), || Some("   ".into())).is_none());
    }

    #[test]
    fn configured_section_with_token_plans_listener_start() {
        let cfg = WebhookIntakeConfig {
            poll_interval_ms: 10,
            max_events_per_poll: 0,
            ..WebhookIntakeConfig::default()
        };
        let plan = plan_webhook_intake(Some(&cfg), || Some("tok".into())).expect("plan");
        let json: serde_json::Value = serde_json::from_str(&plan.config_json).expect("json");
        assert_eq!(json["ingress_token"], "tok");
        assert!(
            json.get("addr").is_none(),
            "no addr unless bind_addr is set"
        );
        assert_eq!(plan.interval, std::time::Duration::from_millis(250));
        assert_eq!(plan.max_events, 1);

        let cfg = WebhookIntakeConfig {
            bind_addr: Some("127.0.0.1:9080".into()),
            max_events_per_poll: 5000,
            ..WebhookIntakeConfig::default()
        };
        let plan = plan_webhook_intake(Some(&cfg), || Some("tok".into())).expect("plan");
        let json: serde_json::Value = serde_json::from_str(&plan.config_json).expect("json");
        assert_eq!(json["addr"], "127.0.0.1:9080");
        assert_eq!(plan.max_events, 1024);
    }

    #[tokio::test]
    async fn spawn_without_config_is_a_no_op() {
        let hopper: std::sync::Arc<dyn HopperIntake> =
            std::sync::Arc::new(InMemoryHopper::headless());
        assert!(spawn_webhook_intake_poller(None, hopper).is_none());
    }
}
