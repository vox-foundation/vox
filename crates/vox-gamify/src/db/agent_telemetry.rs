//! Agent events, cost records, sessions, metrics.

use anyhow::Result;
use vox_db::Codex;

/// Persistent record of an agent lifecycle or state-change event.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentEventRecord {
    /// Monotonic database row ID.
    pub id: i64,
    /// Identifier of the agent that emitted the event.
    pub agent_id: String,
    /// Discriminant string for the event kind (e.g. `"task_completed"`).
    pub event_type: String,
    /// Optional JSON payload attached to the event.
    pub payload: Option<String>,
    /// CLI / crate version string stored with the event when present.
    #[serde(default)]
    pub cli_version: Option<String>,
    /// SQLite `datetime` string when the event was recorded.
    pub timestamp: String,
}

/// Load recent events for an agent.
pub async fn get_events(
    db: &Codex,
    agent_id: &str,
    limit: Option<i64>,
) -> Result<Vec<AgentEventRecord>> {
    let rows = db.list_gamify_events(agent_id, limit.unwrap_or(50)).await?;
    let mut events = Vec::new();
    for row in rows {
        events.push(AgentEventRecord {
            id: row.id,
            agent_id: row.agent_id.into_string(),
            event_type: row.event_type,
            payload: row.payload_json,
            cli_version: row.cli_version,
            timestamp: row.timestamp,
        });
    }
    Ok(events)
}

/// Insert a new agent event.
pub async fn insert_event(
    db: &Codex,
    agent_id: &str,
    event_type: &str,
    payload: Option<&str>,
) -> Result<()> {
    db.insert_gamify_event(agent_id, event_type, payload)
        .await?;
    Ok(())
}

/// A recorded LLM cost event for a single agent inference call.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CostRecord {
    /// Monotonic database row ID.
    pub id: i64,
    /// Identifier of the agent that incurred the cost.
    pub agent_id: String,
    /// Optional session correlation ID.
    pub session_id: Option<String>,
    /// Provider backend name (e.g. `"google"`, `"openai"`).
    pub provider: String,
    /// Model identifier used for the inference.
    pub model: Option<String>,
    /// Number of prompt (input) tokens consumed.
    pub input_tokens: i64,
    /// Number of completion (output) tokens produced.
    pub output_tokens: i64,
    /// Estimated cost in USD for this call.
    pub cost_usd: f64,
    /// SQLite `datetime` string when the record was inserted.
    pub timestamp: String,
}

impl CostRecord {
    /// Create a new cost record for in-memory tracking.
    pub fn new_ephemeral(
        agent_id: impl Into<String>,
        provider: impl Into<String>,
        model: Option<String>,
        input_tokens: i64,
        output_tokens: i64,
        cost_usd: f64,
    ) -> Self {
        Self {
            id: 0,
            agent_id: agent_id.into(),
            session_id: None,
            provider: provider.into(),
            model,
            input_tokens,
            output_tokens,
            cost_usd,
            timestamp: String::new(),
        }
    }

    /// Set the session ID.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// Insert a cost record.
#[allow(clippy::too_many_arguments)]
pub async fn insert_cost_record(
    db: &Codex,
    agent_id: &str,
    session_id: Option<&str>,
    provider: &str,
    model: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
    cost_usd: f64,
) -> Result<()> {
    db.insert_gamify_cost_record(
        agent_id,
        session_id,
        provider,
        model,
        input_tokens,
        output_tokens,
        cost_usd,
    )
    .await?;
    Ok(())
}

/// Get total cost for an agent.
pub async fn get_agent_cost_usd(db: &Codex, agent_id: &str) -> Result<f64> {
    Ok(db.get_gamify_agent_cost_usd(agent_id).await?)
}

/// Get cost records for an agent, most recent first.
pub async fn list_cost_records(db: &Codex, agent_id: &str, limit: i64) -> Result<Vec<CostRecord>> {
    let rows = db.list_gamify_cost_records(agent_id, limit).await?;
    let mut records = Vec::new();
    for (
        id,
        agent_id,
        session_id,
        provider,
        model,
        input_tokens,
        output_tokens,
        cost_usd,
        timestamp,
    ) in rows
    {
        records.push(CostRecord {
            id,
            agent_id,
            session_id,
            provider,
            model,
            input_tokens,
            output_tokens,
            cost_usd,
            timestamp,
        });
    }
    Ok(records)
}

/// Acknowledge an A2A message by ID.
pub async fn acknowledge_message(db: &Codex, id: i64) -> Result<()> {
    db.acknowledge_a2a_message_by_id(id).await?;
    Ok(())
}

/// Persisted snapshot of an agent's run session for lifecycle tracking.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct AgentSessionRecord {
    /// Unique session identifier (UUID string).
    pub id: String,
    /// Numeric or named agent identifier.
    pub agent_id: String,
    /// Human-readable name of the agent, if assigned.
    pub agent_name: Option<String>,
    /// SQLite `datetime` when the session began.
    pub started_at: String,
    /// SQLite `datetime` when the session ended, or `None` if still active.
    pub ended_at: Option<String>,
    /// Session lifecycle status: `"active"`, `"archived"`, etc.
    pub status: String,
    /// JSON snapshot of the task being processed at compaction time.
    pub task_snapshot: Option<String>,
    /// Summarized context carried across compaction boundaries.
    pub context_summary: Option<String>,
}

/// Insert a new agent session.
pub async fn insert_agent_session(
    db: &Codex,
    id: &str,
    agent_id: &str,
    agent_name: Option<&str>,
) -> Result<()> {
    db.insert_gamify_session(id, agent_id, agent_name).await?;
    Ok(())
}

/// Update session status and optional context.
pub async fn update_agent_session(
    db: &Codex,
    id: &str,
    status: &str,
    task_snapshot: Option<&str>,
    context_summary: Option<&str>,
) -> Result<()> {
    db.update_gamify_session(id, status, task_snapshot, context_summary)
        .await?;
    Ok(())
}

/// End a session by setting ended_at and status.
pub async fn end_agent_session(db: &Codex, id: &str, status: &str) -> Result<()> {
    db.end_gamify_session(id, status).await?;
    Ok(())
}

/// Get active sessions.
pub async fn list_active_sessions(db: &Codex) -> Result<Vec<AgentSessionRecord>> {
    let rows = db.list_gamify_active_sessions().await?;
    let mut sessions = Vec::new();
    for row in rows {
        sessions.push(AgentSessionRecord {
            id: row.0,
            agent_id: row.1,
            agent_name: row.2,
            started_at: row.3,
            ended_at: row.4,
            status: row.5,
            task_snapshot: row.6,
            context_summary: row.7,
        });
    }
    Ok(sessions)
}

/// Upsert an aggregated metric for an agent.
pub async fn upsert_agent_metric(
    db: &Codex,
    agent_id: &str,
    metric_name: &str,
    metric_value: f64,
    period: &str,
) -> Result<()> {
    db.upsert_gamify_agent_metric(agent_id, metric_name, metric_value, period)
        .await?;
    Ok(())
}

/// Get all metrics for an agent in a given period.
pub async fn get_agent_metrics(
    db: &Codex,
    agent_id: &str,
    period: &str,
) -> Result<std::collections::HashMap<String, f64>> {
    let metrics = db.get_gamify_agent_metrics(agent_id, period).await?;
    let mut map = std::collections::HashMap::new();
    for (name, val) in metrics {
        map.insert(name, val);
    }
    Ok(map)
}
