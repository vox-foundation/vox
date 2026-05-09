//! Agent Q&A and bulletin-board MCP tools built on the orchestrator QA router.
//!
//! Questions carry correlation ids; answers and broadcasts are published for other agents to observe.

use serde::{Deserialize, Serialize};

use crate::params::ToolResult;
use crate::server_state::ServerState;
use vox_actor_runtime::prompt_canonical;
use vox_orchestrator::{AgentId, MessageGateway};

const REM_QA_LOCK: &str = "Retry; persistent poisoned-lock errors usually need an MCP restart.";
const REM_QA_CORRELATION: &str = "Use the correlation id returned by `ask_agent`, or list `pending_questions` for the target agent.";

/// MCP arguments: direct question from `from_agent` to `to_agent` (canonicalized prompt text).
#[derive(Debug, Deserialize)]
pub struct AskAgentParams {
    /// Asking agent id.
    pub from_agent: u64,
    /// Target agent id.
    pub to_agent: u64,
    /// Question body (canonicalized before routing).
    pub question: String,
}

/// MCP arguments: satisfy a pending question by `correlation_id`.
#[derive(Debug, Deserialize)]
pub struct AnswerQuestionParams {
    /// Id returned by [`ask_agent`].
    pub correlation_id: u64,
    /// Answer text published to the bulletin board.
    pub answer: String,
    /// Answering agent id for bulletin / event metadata (default `0` = unknown).
    #[serde(default)]
    pub from_agent: u64,
}

/// MCP arguments: list outstanding questions addressed to this agent.
#[derive(Debug, Deserialize)]
pub struct PendingQuestionsParams {
    /// Agent whose inbox is listed.
    pub agent_id: u64,
}

/// MCP arguments: fan-out bulletin message from one agent to all subscribers.
#[derive(Debug, Deserialize)]
pub struct BroadcastParams {
    /// Originating agent id.
    pub from_agent: u64,
    /// Broadcast body (canonicalized).
    pub message: String,
}

/// JSON row for each pending question returned by [`pending_questions`].
#[derive(Debug, Serialize)]
pub struct PendingQuestionResponse {
    /// Correlation id for answering.
    pub correlation_id: u64,
    /// Original question text.
    pub question: String,
}

/// Post a canonicalized question, register it in the QA router, and emit a bulletin event.
pub async fn ask_agent(state: &ServerState, params: AskAgentParams) -> String {
    let orch = &state.orchestrator;

    let question = prompt_canonical::canonicalize_simple(&params.question);
    let q_router = orch.qa_router_handle();
    let q_guard = match crate::sync_poison::poison_rw_write(q_router.write(), "qa router") {
        Ok(g) => g,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e.to_string(), REM_QA_LOCK)
                .to_json();
        }
    };
    let corr_id = q_guard.ask(
        AgentId(params.from_agent),
        AgentId(params.to_agent),
        question.clone(),
    );

    let msg = vox_orchestrator::types::AgentMessage::Question {
        from: AgentId(params.from_agent),
        to: AgentId(params.to_agent),
        question,
        correlation_id: vox_orchestrator::types::CorrelationId(corr_id.0),
    };
    let bus = orch.event_bus().clone();
    MessageGateway::publish_bulletin_inter_agent(orch.bulletin(), &bus, msg);

    ToolResult::ok(format!(
        "Question posted with correlation ID: {}",
        corr_id.0
    ))
    .to_json()
}

/// Record an answer for `correlation_id` and publish an `AgentMessage::Answer` bulletin (best-effort answerer id).
pub async fn answer_question(state: &ServerState, params: AnswerQuestionParams) -> String {
    let orch = &state.orchestrator;

    let answer = params.answer.clone();
    let corr_id = vox_orchestrator::types::CorrelationId(params.correlation_id);
    let q_router = orch.qa_router_handle();
    let q_guard = match crate::sync_poison::poison_rw_write(q_router.write(), "qa router") {
        Ok(g) => g,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e.to_string(), REM_QA_LOCK)
                .to_json();
        }
    };
    match q_guard.answer(corr_id, &answer) {
        Some(original_asker) => {
            let answerer = AgentId(params.from_agent);
            let msg = vox_orchestrator::types::AgentMessage::Answer {
                from: answerer,
                to: original_asker,
                answer,
                correlation_id: corr_id,
            };
            let bus = orch.event_bus().clone();
            MessageGateway::publish_bulletin_inter_agent(orch.bulletin(), &bus, msg);
            ToolResult::ok(format!(
                "Answer posted for correlation ID: {}",
                params.correlation_id
            ))
            .to_json()
        }
        None => ToolResult::<String>::err_with_remediation(
            format!(
                "No pending question found for correlation ID: {}",
                params.correlation_id
            ),
            REM_QA_CORRELATION,
        )
        .to_json(),
    }
}

/// Return JSON array of `{ correlation_id, question }` tuples awaiting this agent.
pub async fn pending_questions(state: &ServerState, params: PendingQuestionsParams) -> String {
    let orch = &state.orchestrator;

    let q_router = orch.qa_router_handle();
    let read_guard = match crate::sync_poison::poison_rw_read(q_router.read(), "qa router") {
        Ok(g) => g,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(e.to_string(), REM_QA_LOCK)
                .to_json();
        }
    };
    let questions = read_guard.pending_questions(AgentId(params.agent_id));

    let result: Vec<PendingQuestionResponse> = questions
        .into_iter()
        .map(|(id, q)| PendingQuestionResponse {
            correlation_id: id.0,
            question: q,
        })
        .collect();

    ToolResult::ok(result).to_json()
}

/// Publish a canonicalized broadcast message on the orchestrator bulletin board.
pub async fn broadcast(state: &ServerState, params: BroadcastParams) -> String {
    let orch = &state.orchestrator;

    let message = prompt_canonical::canonicalize_simple(&params.message);
    let msg = vox_orchestrator::types::AgentMessage::Broadcast {
        from: AgentId(params.from_agent),
        message,
    };
    let bus = orch.event_bus().clone();
    MessageGateway::publish_bulletin_inter_agent(orch.bulletin(), &bus, msg);

    ToolResult::ok(format!(
        "Message broadcasted from agent: {}",
        params.from_agent
    ))
    .to_json()
}
