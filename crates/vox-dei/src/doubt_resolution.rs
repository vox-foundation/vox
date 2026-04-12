use std::sync::Arc;
use tracing::{error, info};
use vox_ludus::ai::FreeAiClient;
use vox_orchestrator::Orchestrator;
use vox_orchestrator::types::{AgentId, TaskId, TaskStatus};

/// The Resolution Agent acts as a rigorous arbiter of truth.
/// It picks up "Doubted" tasks and evaluates them against the user's concerns.
pub struct ResolutionAgent {
    orchestrator: Arc<Orchestrator>,
    ai: FreeAiClient,
}

impl ResolutionAgent {
    /// Create a new Resolution Agent.
    pub async fn new(orchestrator: Arc<Orchestrator>) -> Self {
        let budget_manager = orchestrator.budget_manager.clone();
        let ai = FreeAiClient::auto_discover()
            .await
            .with_cost_reporter(Arc::new(move |cost: f64| {
                if let Ok(bm) = budget_manager.write() {
                    bm.record_cost(AgentId(0), cost);
                }
            }));

        Self { orchestrator, ai }
    }

    /// Background loop to poll for Doubted tasks.
    pub async fn run(&self) {
        info!("Resolution Agent started; polling for Doubted tasks...");
        loop {
            let agent_ids = self.orchestrator.agent_ids();
            for aid in agent_ids {
                if let Some(mut task) = self.orchestrator.dequeue_doubted(aid) {
                    let task_id = task.id;
                    let reason = match &task.status {
                        TaskStatus::Doubted(r) => r
                            .clone()
                            .unwrap_or_else(|| "No reason provided".to_string()),
                        _ => "No reason provided".to_string(),
                    };

                    info!(
                        "Resolution Agent processing doubted task {} from agent {}",
                        task_id, aid
                    );
                    if let Err(e) = self.resolve_task(aid, &mut task, &reason).await {
                        error!("Resolution Agent failed to resolve task {}: {}", task_id, e);
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    async fn resolve_task(
        &self,
        agent_id: AgentId,
        task: &mut vox_orchestrator::types::AgentTask,
        doubt_reason: &str,
    ) -> anyhow::Result<()> {
        let prompt = self.build_prompt(task, doubt_reason);

        info!(
            "Resolution Agent interrogating AI output for task {}...",
            task.id
        );

        // Use a high-tier model for resolution if possible.
        let report: String = self.ai.generate(&prompt).await?;

        if report.contains("[VALIDATED]") {
            info!(
                "Resolution Agent VALIDATED task {}. AI was correct.",
                task.id
            );
            self.orchestrator
                .complete_task_with_audit(task.id, report.clone())
                .await?;
            self.emit_resolution_event(task.id, agent_id, true, &report)
                .await;
        } else if report.contains("[OVERRULED]") {
            info!(
                "Resolution Agent OVERRULED task {}. AI was incorrect.",
                task.id
            );
            self.orchestrator
                .fail_task_with_audit(
                    task.id,
                    format!("Resolution Agent Overruled: {}", report),
                    Some(report.clone()),
                )
                .await?;
            self.emit_resolution_event(task.id, agent_id, false, &report)
                .await;
        } else {
            // Default to fail if the agent is inconclusive but suspicious?
            // Or maybe just fail it for human review.
            info!(
                "Resolution Agent inconclusive for task {}. Defaulting to OVERRULED.",
                task.id
            );
            self.orchestrator
                .fail_task_with_audit(
                    task.id,
                    format!("Resolution Agent Inconclusive: {}", report),
                    Some(report.clone()),
                )
                .await?;
            self.emit_resolution_event(task.id, agent_id, false, &report)
                .await;
        }

        Ok(())
    }

    fn build_prompt(
        &self,
        task: &vox_orchestrator::types::AgentTask,
        doubt_reason: &str,
    ) -> String {
        format!(
            "SYSTEM: Persona: INTERROGATOR. You are a rigorous, non-obsequious arbiter of truth. 
Your goal is to evaluate if an AI agent's output is correct or if the user's doubt is justified.
DO NOT be polite to the user or the agent. Use technical truth as your only metric.
Pay special attention to 'obsequiousness' - when an agent agrees with a user just to be 'nice' or 'helpful' despite being technically wrong or incomplete.

CONTEXT:
Task Description: {description}
AI Output/State: {status:?}

USER DOUBT REASON:
{doubt_reason}

INSTRUCTIONS:
1. Analyze the AI output against the task description and the user's doubt.
2. If the AI was correct and the user's doubt is unjustified, respond with [VALIDATED] followed by a brief technical justification.
3. If the AI was incorrect (hallucination, incomplete, wrong approach, or being TOO OBSEQUIOUS) as the user suspected, respond with [OVERRULED] followed by a detailed explanation level. 
   If the agent was being obsequious (agreeing with wrong user premises), EXPLICITLY mention the word 'obsequious' in your report.
4. Be brief, blunt, and accurate.",
            description = task.description,
            status = task.status,
            doubt_reason = doubt_reason
        )
    }

    async fn emit_resolution_event(
        &self,
        task_id: TaskId,
        agent_id: AgentId,
        validated: bool,
        report: &str,
    ) {
        use vox_orchestrator::events::AgentEventKind;
        self.orchestrator
            .event_bus()
            .emit(AgentEventKind::TaskResolved {
                task_id,
                agent_id,
                validated,
                report: report.to_string(),
            });
    }
}
