//! Opt-in webhook -> hopper intake (Phase 3 D-03/D-04/D-10/D-13).
//!
//! vox-plugin-webhook accepts authenticated webhook deliveries and queues them;
//! the host drains them through the plugin's `WebhookInbox` poll extension as
//! JSON strings. This module turns each string into a `HopperIntake` item with
//! `IntakeSource::Webhook`. The JSON field names (`id`, `source`, `event_type`)
//! are the plugin's `WebhookEvent` serde names and are the whole contract across
//! the boundary — no crate here depends on the plugin.

use vox_orchestrator::hopper::{HopperIntake, IntakeSource, PriorityHint};

/// Longest slice of any untrusted event field that reaches the hopper intent.
const MAX_FIELD_CHARS: usize = 64;

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
}
