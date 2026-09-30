//! One severity per engine event, decided where the event is defined, so no surface classifies events by
//! string matching. The match below has no wildcard arm on purpose: a new `AgentEventKind` variant does
//! not compile until someone decides how loud it is.

use serde::{Deserialize, Serialize};

use crate::budget::BudgetSignal;
use crate::events::AgentEventKind;

/// How much an engine event matters to a person. `Debug` is kept out of every surface except the
/// activity log and the verbose trace (docs/src/architecture/gui-observability-ssot-2026.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

impl AgentEventKind {
    /// The event's severity.
    #[must_use]
    pub fn severity(&self) -> EventSeverity {
        use AgentEventKind as K;
        use EventSeverity::{Debug, Error, Info, Warning};
        match self {
            K::TaskFailed { .. }
            | K::WorkflowFailed { .. }
            | K::EmergencyStop { .. }
            | K::InjectionDetected { .. }
            | K::ScopeViolation { .. }
            | K::TaskExpired { .. } => Error,

            K::ToolTimedOut { .. }
            | K::TaskDoubted { .. }
            | K::DoubtReported { .. }
            | K::ConflictDetected { .. }
            | K::PromptConflictDetected { .. }
            | K::AgentHandoffRejected { .. }
            | K::UrgentRebalanceTriggered { .. }
            | K::ReplanTriggered { .. }
            | K::ActivityRetried { .. }
            | K::AutoHealSuggested { .. }
            | K::AttentionBudgetAlert { .. }
            | K::TrustOverride { .. }
            | K::ContextTruncated { .. }
            | K::SemanticDriftDetected { .. } => Warning,

            K::GroundingCheckCompleted { flagged, .. } => {
                if *flagged {
                    Warning
                } else {
                    Info
                }
            }
            K::BudgetAlert { signal, .. } => match signal {
                BudgetSignal::Normal { .. } | BudgetSignal::ToolLatencyUnknown { .. } => Info,
                BudgetSignal::HighLoad { .. }
                | BudgetSignal::AttentionHigh { .. }
                | BudgetSignal::ToolLatencyHigh { .. }
                | BudgetSignal::DoomLoopSuspect { .. } => Warning,
                BudgetSignal::Critical { .. }
                | BudgetSignal::CostExceeded { .. }
                | BudgetSignal::AttentionCritical { .. }
                | BudgetSignal::HaltAgent { .. } => Error,
            },

            K::TokenStreamed { .. }
            | K::AgentHeartbeat { .. }
            | K::ActivityChanged { .. }
            | K::ToolCallDispatched { .. }
            | K::MessageSent { .. }
            | K::CostIncurred { .. }
            | K::AgentIdle { .. }
            | K::AgentBusy { .. }
            | K::LlmCallCompleted { .. }
            | K::ObservationRecorded { .. }
            | K::ThroughputTick { .. }
            | K::CostTick { .. }
            | K::FileDiagChanged { .. }
            | K::BuildStage { .. }
            | K::MensObserverObservation { .. }
            | K::EndpointReliabilityObservation { .. }
            | K::MeshNodeBudget { .. } => Debug,

            K::AgentSpawned { .. }
            | K::AgentRetired { .. }
            | K::OperatingModeChanged { .. }
            | K::FeedbackRequested { .. }
            | K::FeedbackResolved { .. }
            | K::TaskSubmitted { .. }
            | K::TaskStarted { .. }
            | K::TaskPhaseChanged { .. }
            | K::TaskCompleted { .. }
            | K::TaskDelegated { .. }
            | K::TaskResolved { .. }
            | K::LockAcquired { .. }
            | K::LockReleased { .. }
            | K::LockWaiting { .. }
            | K::ContinuationTriggered { .. }
            | K::PlanHandoff { .. }
            | K::CompactionTriggered { .. }
            | K::MemoryFlushed { .. }
            | K::SessionCreated { .. }
            | K::SessionReset { .. }
            | K::SnapshotCaptured { .. }
            | K::OperationUndone { .. }
            | K::OperationRedone { .. }
            | K::AgentHandoffAccepted { .. }
            | K::PlanningRouted { .. }
            | K::PlanSessionCreated { .. }
            | K::PlanVersionCreated { .. }
            | K::WorkflowHandoffRequested { .. }
            | K::WorkflowHandoffCompleted { .. }
            | K::WorkflowStarted { .. }
            | K::WorkflowCompleted { .. }
            | K::ActivityStarted { .. }
            | K::ActivityCompleted { .. }
            | K::ConflictResolved { .. }
            | K::WorkspaceCreated { .. }
            | K::OrchestratorIdle { .. }
            | K::AutoHealApplied { .. }
            | K::AttentionBudgetReset { .. }
            | K::AttentionConfigReloaded
            | K::OrientCompleted { .. }
            | K::ResearchExecuted { .. }
            | K::ResearchSynthesisExecuted { .. }
            | K::MeshTopologyChanged { .. }
            | K::TaskReprioritized { .. }
            | K::HopperItemAdmitted { .. }
            | K::HopperItemOverridden { .. }
            | K::HopperItemCancelled { .. }
            | K::MeshActionCommitted { .. }
            | K::PavPhaseChanged { .. } => Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetSignal;
    use crate::events::AgentEventKind;
    use crate::types::{AgentId, TaskId};

    fn budget(signal: BudgetSignal) -> AgentEventKind {
        AgentEventKind::BudgetAlert {
            agent_id: AgentId(1),
            signal,
        }
    }

    #[test]
    fn failures_and_safety_events_are_errors() {
        assert_eq!(
            AgentEventKind::InjectionDetected { detail: "x".into() }.severity(),
            EventSeverity::Error
        );
        assert_eq!(
            budget(BudgetSignal::CostExceeded {
                cost_usd: 2.0,
                limit_usd: 1.0
            })
            .severity(),
            EventSeverity::Error
        );
        let failed = AgentEventKind::TaskFailed {
            task_id: TaskId(7),
            agent_id: AgentId(1),
            error: "boom".into(),
            session_id: None,
            audit_report: None,
        };
        assert_eq!(failed.severity(), EventSeverity::Error);
    }

    #[test]
    fn budget_severity_follows_the_signal() {
        assert_eq!(
            budget(BudgetSignal::Normal { usage_ratio: 0.1 }).severity(),
            EventSeverity::Info
        );
        assert_eq!(
            budget(BudgetSignal::HighLoad {
                usage_ratio: 0.8,
                tokens_remaining: 100
            })
            .severity(),
            EventSeverity::Warning
        );
        assert_eq!(
            budget(BudgetSignal::Critical {
                usage_ratio: 0.99,
                tokens_remaining: 1
            })
            .severity(),
            EventSeverity::Error
        );
    }

    #[test]
    fn a_flagged_grounding_check_is_a_warning() {
        let check = |flagged| AgentEventKind::GroundingCheckCompleted {
            agent_id: AgentId(1),
            task_id: TaskId(1),
            confidence: 0.4,
            flagged,
        };
        assert_eq!(check(true).severity(), EventSeverity::Warning);
        assert_eq!(check(false).severity(), EventSeverity::Info);
    }

    #[test]
    fn routine_events_are_info() {
        assert_eq!(
            AgentEventKind::AttentionConfigReloaded.severity(),
            EventSeverity::Info
        );
        assert_eq!(
            AgentEventKind::LockWaiting {
                resource_id: "db://x".into(),
                task_id: TaskId(1),
                session_id: None
            }
            .severity(),
            EventSeverity::Info
        );
    }

    #[test]
    fn severity_serialises_lowercase_for_the_gui() {
        assert_eq!(
            serde_json::to_value(EventSeverity::Warning).unwrap(),
            serde_json::json!("warning")
        );
        assert_eq!(
            serde_json::to_value(EventSeverity::Debug).unwrap(),
            serde_json::json!("debug")
        );
    }
}
