//! The daemon's boot steps as a [`BootStep`] table, in pre-profile order.
//!
//! Each step is the code that used to sit inline in `async_main`, moved verbatim apart from
//! reading/writing [`DaemonBootCtx`] instead of locals. Steps for subsystems that were lazy
//! before profiles (`model_registry`, `llm_facade`, `search_index`, `memory`, `compiler`,
//! `browser`, `speech`) are probes or markers: they do no eager IO, they give the subsystem a
//! slot in the closure and in the [`BootReport`] the MCP gate reads.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use vox_orchestrator::{
    Orchestrator, OrchestratorConfig, RemotePopuliSnapshot, SessionManager,
    build_repo_scoped_orchestrator,
};
use vox_orchestrator_mcp::profiles::{ActiveProfile, BootReport, Subsystem};
use vox_orchestrator_mcp::server_state::ServerState;

use crate::boot_plan::{BootContext, BootStep, StepFn};

pub struct DaemonBootCtx {
    pub orch: Arc<Orchestrator>,
    pub orch_config: OrchestratorConfig,
    pub repository: vox_repository::RepositoryContext,
    pub repository_id: String,
    /// Repository discovery hint for the store; `None` = process CWD (production).
    pub start_dir: Option<PathBuf>,
    pub profile: ActiveProfile,
    pub plugins_dir: PathBuf,
    pub db: Option<Arc<vox_db::VoxDb>>,
    pub session_manager: Option<SessionManager>,
    pub skill_registry: Option<Arc<vox_skills::SkillRegistry>>,
    pub state: Option<ServerState>,
    pub populi_remote_snapshot: Arc<RwLock<RemotePopuliSnapshot>>,
    pub populi_poll_join: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl DaemonBootCtx {
    pub fn new(
        cfg: OrchestratorConfig,
        start_dir: Option<&Path>,
        profile: ActiveProfile,
        plugins_dir: PathBuf,
    ) -> Self {
        let build = build_repo_scoped_orchestrator(cfg, start_dir);
        let orch_config = build.config.clone();
        let repository_id = build.repository.repository_id.clone();
        Self {
            orch: Arc::new(build.orchestrator),
            orch_config,
            repository: build.repository,
            repository_id,
            profile,
            start_dir: start_dir.map(Path::to_path_buf),
            plugins_dir,
            db: None,
            session_manager: None,
            skill_registry: None,
            state: None,
            populi_remote_snapshot: Arc::new(RwLock::new(RemotePopuliSnapshot::default())),
            populi_poll_join: Arc::new(Mutex::new(None)),
        }
    }
}

impl BootContext for DaemonBootCtx {
    fn publish(&mut self, report: &BootReport) {
        if let Some(state) = self.state.as_ref() {
            state.set_boot_report(report.clone());
        }
    }
}

/// `$VOX_PLUGINS_DIR` if set, else the platform-local default (moved verbatim from `async_main`).
pub fn plugins_install_dir(env_value: Option<String>) -> PathBuf {
    env_value.map(PathBuf::from).unwrap_or_else(|| {
        dirs::data_local_dir()
            .map(|p| p.join("vox").join("plugins"))
            .unwrap_or_else(|| PathBuf::from("./vox-plugins"))
    })
}

mod run;
use run::{
    step_a2a_result, step_a2a_worker, step_agent_fleet, step_attention, step_clarification,
    step_db, step_flywheel, step_lazy, step_model_registry, step_populi_federation,
    step_routing_preferences, step_server_state, step_sessions, step_skills, step_socrates,
};

const fn s(
    name: &'static str,
    subsystem: Option<Subsystem>,
    legacy: bool,
    run: StepFn<DaemonBootCtx>,
) -> BootStep<DaemonBootCtx> {
    BootStep {
        name,
        subsystem,
        legacy,
        run,
    }
}

/// Pre-profile order for `legacy` steps; new steps slot in topologically.
/// The HTTP gateway stays in `main`, after the exit decision.
pub static DAEMON_STEPS: &[BootStep<DaemonBootCtx>] = &[
    s("db", Some(Subsystem::Db), true, step_db),
    s(
        "model_registry",
        Some(Subsystem::ModelRegistry),
        false,
        step_model_registry,
    ),
    s(
        "routing_preferences",
        Some(Subsystem::ModelRegistry),
        true,
        step_routing_preferences,
    ),
    s("llm_facade", Some(Subsystem::LlmFacade), false, step_lazy),
    s(
        "search_index",
        Some(Subsystem::SearchIndex),
        false,
        step_lazy,
    ),
    s("memory", Some(Subsystem::Memory), false, step_lazy),
    s("sessions", Some(Subsystem::Sessions), true, step_sessions),
    s("skills", None, true, step_skills),
    s("server_state", None, true, step_server_state),
    s(
        "agent_fleet",
        Some(Subsystem::AgentFleet),
        true,
        step_agent_fleet,
    ),
    s(
        "populi_federation_poller",
        Some(Subsystem::Mesh),
        true,
        step_populi_federation,
    ),
    s(
        "a2a_remote_result_poller",
        Some(Subsystem::Mesh),
        true,
        step_a2a_result,
    ),
    s(
        "a2a_remote_worker_poller",
        Some(Subsystem::Mesh),
        true,
        step_a2a_worker,
    ),
    s(
        "clarification_inbox",
        Some(Subsystem::Clarification),
        true,
        step_clarification,
    ),
    s(
        "socrates_research_poller",
        Some(Subsystem::Research),
        true,
        step_socrates,
    ),
    s("flywheel", Some(Subsystem::Flywheel), true, step_flywheel),
    s(
        "attention_calibration",
        Some(Subsystem::Attention),
        true,
        step_attention,
    ),
    s("compiler", Some(Subsystem::Compiler), false, step_lazy),
    s("browser", Some(Subsystem::Browser), false, step_lazy),
    s("speech", Some(Subsystem::Speech), false, step_lazy),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boot_plan::planned_step_names;

    /// `async_main` before capability profiles, L218–373, in order.
    const TODAYS_ORDER: &[&str] = &[
        "db",
        "routing_preferences",
        "sessions",
        "skills",
        "server_state",
        "agent_fleet",
        "populi_federation_poller",
        "a2a_remote_result_poller",
        "a2a_remote_worker_poller",
        "clarification_inbox",
        "socrates_research_poller",
        "flywheel",
        "attention_calibration",
    ];

    #[test]
    fn dev_plan_runs_todays_steps_in_todays_order() {
        let planned = planned_step_names(DAEMON_STEPS, &ActiveProfile::dev().subsystems);
        let legacy: Vec<&str> = planned
            .iter()
            .copied()
            .filter(|n| DAEMON_STEPS.iter().any(|s| s.name == *n && s.legacy))
            .collect();
        assert_eq!(legacy, TODAYS_ORDER);
    }

    #[test]
    fn dev_plan_runs_every_step() {
        let planned = planned_step_names(DAEMON_STEPS, &ActiveProfile::dev().subsystems);
        let all: Vec<&str> = DAEMON_STEPS.iter().map(|s| s.name).collect();
        assert_eq!(planned, all);
    }

    #[test]
    fn every_subsystem_has_a_step() {
        for sub in Subsystem::ALL {
            assert!(
                DAEMON_STEPS.iter().any(|s| s.subsystem == Some(*sub)),
                "no boot step for {}",
                sub.as_str()
            );
        }
    }

    #[test]
    fn steps_are_topologically_ordered() {
        for (i, step) in DAEMON_STEPS.iter().enumerate() {
            let Some(sub) = step.subsystem else {
                continue;
            };
            for dep in sub.deps() {
                assert!(
                    DAEMON_STEPS[..i].iter().any(|p| p.subsystem == Some(*dep)),
                    "step {} ({}) runs before its dependency {}",
                    step.name,
                    sub.as_str(),
                    dep.as_str()
                );
            }
        }
    }

    const NON_DB_PROFILE_BANNED: [&str; 8] = [
        "sessions",
        "agent_fleet",
        "populi_federation_poller",
        "a2a_remote_result_poller",
        "a2a_remote_worker_poller",
        "clarification_inbox",
        "socrates_research_poller",
        "flywheel",
    ];

    /// D1: `attention` is DB-free.
    #[test]
    fn attention_plan_has_no_db_session_or_mesh_steps() {
        let attention = ActiveProfile::resolve(&["attention"]).expect("attention profile exists");
        let planned = planned_step_names(DAEMON_STEPS, &attention.subsystems);
        for banned in std::iter::once("db").chain(NON_DB_PROFILE_BANNED) {
            assert!(
                !planned.contains(&banned),
                "{banned} planned for attention: {planned:?}"
            );
        }
        assert!(planned.contains(&"attention_calibration"));
        assert!(planned.contains(&"skills") && planned.contains(&"server_state"));
    }

    /// D1: `models` plans `db` (and `routing_preferences`) but nothing that depends on sessions or mesh.
    #[test]
    fn models_plan_has_db_but_no_session_or_mesh_steps() {
        let models = ActiveProfile::resolve(&["models"]).expect("models profile exists");
        let planned = planned_step_names(DAEMON_STEPS, &models.subsystems);
        assert!(
            planned.contains(&"db") && planned.contains(&"routing_preferences"),
            "{planned:?}"
        );
        for banned in NON_DB_PROFILE_BANNED {
            assert!(
                !planned.contains(&banned),
                "{banned} planned for models: {planned:?}"
            );
        }
        assert!(planned.contains(&"skills") && planned.contains(&"server_state"));
    }

    #[test]
    fn plugins_install_dir_prefers_env_value() {
        assert_eq!(
            plugins_install_dir(Some("/opt/vox-plugins".into())),
            PathBuf::from("/opt/vox-plugins")
        );
        let fallback = plugins_install_dir(None);
        assert!(
            fallback.ends_with(Path::new("vox").join("plugins"))
                || fallback == Path::new("./vox-plugins"),
            "{}",
            fallback.display()
        );
    }

    #[test]
    fn ctx_new_builds_orchestrator_without_side_effects() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let ctx = DaemonBootCtx::new(
            OrchestratorConfig::for_testing(),
            Some(tmp.path()),
            ActiveProfile::dev(),
            tmp.path().join("plugins"),
        );
        assert!(ctx.db.is_none() && ctx.state.is_none() && ctx.session_manager.is_none());
        assert_eq!(ctx.start_dir.as_deref(), Some(tmp.path()));
        assert!(ctx.populi_poll_join.lock().unwrap().is_none());
    }
}
