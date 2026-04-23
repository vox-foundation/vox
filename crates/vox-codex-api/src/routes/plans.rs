// use axum::response::sse::{Event, Sse};
// use tokio_stream::wrappers::BroadcastStream;

/// Subscribe to planning orchestrator state transitions (e.g. BlockedOnApproval events)
/// Returns a Server-Sent Events stream.
pub async fn subscribe_planning_events(// State(state): State<AppState>,
) -> String /* Sse<impl Stream<Item = Result<Event, Infallible>>> */ {
    // Scaffold implementation for bridging vox-orchestrator events out to telemetry/visualizer clients
    // A channel receiver should map `AgentEventKind` into an SSE `Event`.
    "SSE stream placeholder".to_string()
}
