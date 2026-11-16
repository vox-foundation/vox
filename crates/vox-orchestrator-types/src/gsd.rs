//! Pure-data L0 domain types for the GSD (Get Shit Done) planning, execution,
//! verification, milestone lifecycle, and CAS-backed document framework.

use serde::{Deserialize, Serialize};

/// Lifecycle state of a GSD milestone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GsdMilestoneStatus {
    #[default]
    Active,
    Completed,
    Archived,
}

impl GsdMilestoneStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Completed => "completed",
            Self::Archived => "archived",
        }
    }

    #[must_use]
    pub fn from_db_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "completed" => Self::Completed,
            "archived" => Self::Archived,
            _ => Self::Active,
        }
    }
}

/// Lifecycle state of a single GSD roadmap phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GsdPhaseStatus {
    #[default]
    Pending,
    Discussing,
    Planned,
    Executing,
    Verifying,
    Completed,
    Blocked,
}

impl GsdPhaseStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Discussing => "discussing",
            Self::Planned => "planned",
            Self::Executing => "executing",
            Self::Verifying => "verifying",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
        }
    }

    #[must_use]
    pub fn from_db_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "discussing" => Self::Discussing,
            "planned" => Self::Planned,
            "executing" | "in_progress" => Self::Executing,
            "verifying" => Self::Verifying,
            "completed" | "complete" => Self::Completed,
            "blocked" => Self::Blocked,
            _ => Self::Pending,
        }
    }
}

/// Kind of markdown or state artifact stored under `.planning/` and indexed in VoxDb CAS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum GsdDocumentKind {
    Project,
    Roadmap,
    Requirements,
    #[default]
    State,
    Spec,
    Context,
    Plan,
    Summary,
    Verification,
    Review,
    Debug,
    Codebase,
}

impl GsdDocumentKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::Roadmap => "roadmap",
            Self::Requirements => "requirements",
            Self::State => "state",
            Self::Spec => "spec",
            Self::Context => "context",
            Self::Plan => "plan",
            Self::Summary => "summary",
            Self::Verification => "verification",
            Self::Review => "review",
            Self::Debug => "debug",
            Self::Codebase => "codebase",
        }
    }

    #[must_use]
    pub fn from_db_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "project" => Self::Project,
            "roadmap" => Self::Roadmap,
            "requirements" => Self::Requirements,
            "spec" => Self::Spec,
            "context" => Self::Context,
            "plan" => Self::Plan,
            "summary" => Self::Summary,
            "verification" => Self::Verification,
            "review" => Self::Review,
            "debug" => Self::Debug,
            "codebase" => Self::Codebase,
            _ => Self::State,
        }
    }
}

/// Milestone metadata persisted across `.planning/` and `gsd_milestones`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GsdMilestone {
    pub milestone_id: String,
    pub title: String,
    pub status: GsdMilestoneStatus,
    pub start_phase: u32,
    pub end_phase: u32,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at_ms: Option<i64>,
}

/// Roadmap phase entry linked to a milestone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GsdPhase {
    pub milestone_id: String,
    pub phase_number: u32,
    pub title: String,
    pub status: GsdPhaseStatus,
    pub wave_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_session_id: Option<String>,
    pub updated_at_ms: i64,
}

/// Traceable requirement item bound to a GSD milestone and target phase.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GsdRequirement {
    pub req_id: String,
    pub milestone_id: String,
    pub phase_number: u32,
    pub description: String,
    pub satisfied: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_ref: Option<String>,
    pub updated_at_ms: i64,
}

/// Content-addressed `.planning/` document header backed by Arca CAS (`objects` table).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GsdDocument {
    pub doc_id: String,
    pub milestone_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_number: Option<u32>,
    pub kind: GsdDocumentKind,
    pub rel_path: String,
    pub cas_hash: String,
    pub byte_len: u64,
    pub updated_at_ms: i64,
}

/// Atomic task inside a GSD execution wave.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GsdWaveTask {
    pub task_id: String,
    pub title: String,
    #[serde(default)]
    pub files: Vec<String>,
    pub verification_cmd: String,
}

/// Wave group inside a phase `PLAN.md` (tasks in the same wave are disjoint and parallel-safe).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GsdPlanWave {
    pub wave_number: u32,
    #[serde(default)]
    pub tasks: Vec<GsdWaveTask>,
}

/// Aggregated GSD workspace/DB snapshot for CLI, MCP, GUI, and `/gsd-progress`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct GsdProgressSnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_milestone: Option<GsdMilestone>,
    #[serde(default)]
    pub phases: Vec<GsdPhase>,
    #[serde(default)]
    pub requirements: Vec<GsdRequirement>,
    #[serde(default)]
    pub documents: Vec<GsdDocument>,
    pub next_recommended_skill: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gsd_status_and_kind_roundtrip() {
        for status in [
            GsdPhaseStatus::Pending,
            GsdPhaseStatus::Discussing,
            GsdPhaseStatus::Planned,
            GsdPhaseStatus::Executing,
            GsdPhaseStatus::Verifying,
            GsdPhaseStatus::Completed,
            GsdPhaseStatus::Blocked,
        ] {
            assert_eq!(GsdPhaseStatus::from_db_str(status.as_str()), status);
        }

        for kind in [
            GsdDocumentKind::Project,
            GsdDocumentKind::Roadmap,
            GsdDocumentKind::Requirements,
            GsdDocumentKind::State,
            GsdDocumentKind::Spec,
            GsdDocumentKind::Context,
            GsdDocumentKind::Plan,
            GsdDocumentKind::Summary,
            GsdDocumentKind::Verification,
            GsdDocumentKind::Review,
            GsdDocumentKind::Debug,
            GsdDocumentKind::Codebase,
        ] {
            assert_eq!(GsdDocumentKind::from_db_str(kind.as_str()), kind);
        }

        let snap = GsdProgressSnapshot {
            active_milestone: Some(GsdMilestone {
                milestone_id: "v7.0".to_string(),
                title: "Deep GSD Integration".to_string(),
                status: GsdMilestoneStatus::Active,
                start_phase: 407,
                end_phase: 412,
                created_at_ms: 1000,
                updated_at_ms: 2000,
                completed_at_ms: None,
            }),
            phases: vec![],
            requirements: vec![],
            documents: vec![],
            next_recommended_skill: "gsd-execute-phase".to_string(),
        };
        let encoded = serde_json::to_string(&snap).expect("serialize");
        let decoded: GsdProgressSnapshot = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(snap, decoded);
    }
}
