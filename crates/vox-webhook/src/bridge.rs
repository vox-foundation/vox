//! Bridge between the webhook broadcast channel and the Orchestrator Scheduler.
//!
//! Subscribes to the `broadcast::Receiver<WebhookEvent>` emitted by the Axum
//! webhook router and converts each event into an `OrchestratorInboxItem` that is
//! forwarded to the orchestrator's task queue.
//!
//! ## Wiring
//!
//! ```text
//! WebhookState.event_sink  →  WebhookOrchestratorBridge::run()
//!                          →  mpsc::Sender<OrchestratorInboxItem>
//!                          →  Orchestrator Scheduler
//! ```
//!
//! Start the bridge with [`WebhookOrchestratorBridge::spawn`] to drive it on a
//! dedicated tokio task.

use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};
use vox_orchestrator::Orchestrator;

use crate::handler::WebhookEvent;

/// A task dispatched from an inbound webhook event.
///
/// Callers that hold a `mpsc::Receiver<OrchestratorInboxItem>` (typically the
/// Orchestrator Scheduler) consume these and translate them into concrete tasks.
#[derive(Debug, Clone)]
pub struct OrchestratorInboxItem {
    /// The originating webhook event.
    pub event: WebhookEvent,
    /// High-level task kind inferred from source + event_type.
    pub kind: InboxItemKind,
}

/// Coarse task kind used for routing in the Orchestrator Scheduler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InboxItemKind {
    /// A git push/tag/branch event (e.g. GitHub, GitLab).
    GitPush,
    /// A pull-request lifecycle event.
    PullRequest,
    /// A message from a chat channel (Discord, Slack).
    ChannelMessage,
    /// Any other external event that doesn't match a known kind.
    ExternalEvent,
}

impl OrchestratorInboxItem {
    /// Construct an `OrchestratorInboxItem` from a [`WebhookEvent`].
    ///
    /// Routing rules (source → kind):
    /// - `github` or `gitlab` + event_type `push` → `GitPush`
    /// - `github` or `gitlab` + event_type `pull_request` / `merge_request` → `PullRequest`
    /// - `discord` or `slack` → `ChannelMessage`
    /// - anything else → `ExternalEvent`
    pub fn from_webhook(event: WebhookEvent) -> Self {
        let kind = match (event.source.as_ref(), event.event_type.as_ref()) {
            ("github" | "gitlab", "push" | "tag_push") => InboxItemKind::GitPush,
            ("github" | "gitlab", "pull_request" | "merge_request") => InboxItemKind::PullRequest,
            ("discord" | "slack", _) => InboxItemKind::ChannelMessage,
            _ => InboxItemKind::ExternalEvent,
        };
        Self { event, kind }
    }
}

/// Bridges the webhook broadcast channel into the orchestrator's task queue.
///
/// Drives a `broadcast::Receiver` loop that converts each [`WebhookEvent`] into
/// an agent task directly via [`Orchestrator::submit_task_with_agent`].
pub struct WebhookOrchestratorBridge {
    rx: broadcast::Receiver<WebhookEvent>,
    orch: Arc<Orchestrator>,
}

impl WebhookOrchestratorBridge {
    /// Create a bridge that subscribes to `event_sink` and delegates to `orch`.
    pub fn new(event_sink: &broadcast::Sender<WebhookEvent>, orch: Arc<Orchestrator>) -> Self {
        Self {
            rx: event_sink.subscribe(),
            orch,
        }
    }

    /// Spawn the bridge on a dedicated tokio task.
    ///
    /// Returns the task handle. The bridge runs until the broadcast channel is closed.
    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(self.run())
    }

    /// Drive the bridge loop (consumes `self`).
    ///
    /// Exits cleanly when the broadcast channel closes.
    pub async fn run(mut self) {
        loop {
            match self.rx.recv().await {
                Ok(event) => {
                    let item = OrchestratorInboxItem::from_webhook(event.clone());
                    debug!(kind = ?item.kind, source = %event.source, "Forwarding webhook event to orchestrator");

                    let prompt = format!(
                        "Process incoming webhook event from source {}. Event type: {}. Payload: {}",
                        event.source,
                        event.event_type,
                        serde_json::to_string(&event.payload).unwrap_or_default()
                    );

                    if let Err(e) = self
                        .orch
                        .submit_task_with_agent(
                            prompt,
                            vec![],
                            Some(vox_orchestrator::types::TaskPriority::Normal),
                            None,
                            None,
                            None,
                            None,
                        )
                        .await
                    {
                        error!("Failed to submit webhook task to orchestrator: {}", e);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        n,
                        "Webhook bridge lagged — {} events dropped; consider increasing channel capacity",
                        n
                    );
                    // Continue — do not exit on lag.
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("Webhook broadcast channel closed; bridge shutting down");
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::{broadcast, mpsc};

    use crate::handler::{InboundPayload, WebhookEvent};

    use super::*;

    fn make_event(source: &str, event_type: &str) -> WebhookEvent {
        let payload = InboundPayload {
            source: source.to_string(),
            event_type: event_type.to_string(),
            body: serde_json::json!({"ref": "refs/heads/main"}),
            signature: None,
            timestamp: None,
        };
        WebhookEvent::new(&payload)
    }

    #[test]
    fn github_push_maps_to_git_push() {
        let event = make_event("github", "push");
        let item = OrchestratorInboxItem::from_webhook(event);
        assert_eq!(item.kind, InboxItemKind::GitPush);
    }

    #[test]
    fn gitlab_merge_request_maps_to_pull_request() {
        let event = make_event("gitlab", "merge_request");
        let item = OrchestratorInboxItem::from_webhook(event);
        assert_eq!(item.kind, InboxItemKind::PullRequest);
    }

    #[test]
    fn discord_maps_to_channel_message() {
        let event = make_event("discord", "interaction_create");
        let item = OrchestratorInboxItem::from_webhook(event);
        assert_eq!(item.kind, InboxItemKind::ChannelMessage);
    }

    #[test]
    fn unknown_source_maps_to_external_event() {
        let event = make_event("zapier", "trigger");
        let item = OrchestratorInboxItem::from_webhook(event);
        assert_eq!(item.kind, InboxItemKind::ExternalEvent);
    }

    // Removed integration test since it requires spinning up a full orchestrator now.
}
