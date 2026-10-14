//! Boot-report plumbing between `vox-orchestrator-d`'s boot plan and the MCP gate.
//!
//! The daemon publishes its [`BootReport`] here while booting and once more when boot
//! finishes. [`vox_mcp_registry::authorize_tool`] reads it to return
//! `subsystem_not_booted`.

use vox_mcp_registry::{BootReport, ProfileDenial, Subsystem};

use super::ServerState;

impl ServerState {
    /// Replace the shared boot report. Every clone of this `ServerState` (HTTP gateway,
    /// `McpExtraDispatch`, agent-fleet dispatcher) sees the new value.
    pub fn set_boot_report(&self, report: BootReport) {
        *self.boot_report.write() = report;
    }

    pub fn boot_report_snapshot(&self) -> BootReport {
        self.boot_report.read().clone()
    }

    pub fn subsystem_up(&self, s: Subsystem) -> bool {
        self.boot_report.read().is_up(s)
    }
}

/// `{"error":{"code":"subsystem_not_booted",...}}` for a tool or daemon RPC method.
pub fn subsystem_not_booted_json(kind: &str, name: &str, s: Subsystem) -> String {
    serde_json::json!({
        "error": {
            "code": ProfileDenial::SubsystemNotBooted(s).code(),
            "message": format!(
                "{kind} {name} requires subsystem {}, which is not running",
                s.as_str()
            ),
            "subsystem": s.as_str(),
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(booted: &[Subsystem], failed: &[(Subsystem, &str)]) -> BootReport {
        BootReport {
            booted: booted.iter().copied().collect(),
            failed: failed.iter().map(|(s, e)| (*s, (*e).to_string())).collect(),
        }
    }

    #[tokio::test]
    async fn set_boot_report_is_visible_through_clones() {
        let state = ServerState::new_test().await;
        let clone = state.clone();
        let r = report(&[Subsystem::ModelRegistry], &[(Subsystem::Db, "boom")]);
        state.set_boot_report(r.clone());
        assert_eq!(clone.boot_report_snapshot(), r);
    }

    #[tokio::test]
    async fn subsystem_up_reflects_booted_set_only() {
        let state = ServerState::new_test().await;
        state.set_boot_report(report(&[Subsystem::Attention], &[(Subsystem::Db, "x")]));
        assert!(state.subsystem_up(Subsystem::Attention));
        assert!(!state.subsystem_up(Subsystem::Db));
        assert!(!state.subsystem_up(Subsystem::Mesh));
    }

    #[test]
    fn subsystem_not_booted_json_matches_wire_shape() {
        let v: serde_json::Value = serde_json::from_str(&subsystem_not_booted_json(
            "tool",
            "vox_kb_search",
            Subsystem::SearchIndex,
        ))
        .expect("valid json");
        assert_eq!(v["error"]["code"], "subsystem_not_booted");
        assert_eq!(v["error"]["subsystem"], "search_index");
        assert_eq!(
            v["error"]["message"],
            "tool vox_kb_search requires subsystem search_index, which is not running"
        );
    }

    #[test]
    fn tool_rendering_matches_plan_02_denial_error_json() {
        let s = Subsystem::SearchIndex;
        let ours: serde_json::Value =
            serde_json::from_str(&subsystem_not_booted_json("tool", "vox_kb_search", s))
                .expect("json");
        let theirs = crate::profiles::denial_error_json(
            "vox_kb_search",
            &ProfileDenial::SubsystemNotBooted(s),
            &crate::profiles::ActiveProfile::dev(),
        );
        assert_eq!(ours, theirs);
    }

    #[test]
    fn boot_report_round_trips_through_json() {
        let r = report(&[Subsystem::LlmFacade], &[(Subsystem::Db, "no store")]);
        let back: BootReport =
            serde_json::from_value(serde_json::to_value(&r).expect("ser")).expect("de");
        assert_eq!(back, r);
    }

    fn daemon_parts() -> (
        std::sync::Arc<vox_orchestrator::Orchestrator>,
        vox_orchestrator::OrchestratorConfig,
        vox_repository::RepositoryContext,
        std::sync::Arc<tokio::sync::Mutex<vox_orchestrator::SessionManager>>,
        std::sync::Arc<vox_skills::SkillRegistry>,
    ) {
        let build = vox_orchestrator::bootstrap::build_repo_scoped_orchestrator(
            vox_orchestrator::OrchestratorConfig::for_testing(),
            None,
        );
        let sm = vox_orchestrator::SessionManager::new(vox_orchestrator::SessionConfig {
            persist: false,
            ..vox_orchestrator::SessionConfig::default()
        })
        .expect("in-memory session manager");
        (
            std::sync::Arc::new(build.orchestrator),
            build.config,
            build.repository,
            std::sync::Arc::new(tokio::sync::Mutex::new(sm)),
            vox_skills::new_registry_arc(),
        )
    }

    #[tokio::test]
    async fn gated_constructor_skips_research_subscriber_when_told() {
        let (o, c, r, s, k) = daemon_parts();
        let state = ServerState::new_for_daemon_gated(o, c, r, s, k, false);
        assert_eq!(state.research_events.receiver_count(), 0);
    }

    #[tokio::test]
    async fn new_for_daemon_still_spawns_research_subscriber() {
        let (o, c, r, s, k) = daemon_parts();
        let state = ServerState::new_for_daemon(o, c, r, s, k);
        assert!(state.research_events.receiver_count() >= 1);
    }
}
