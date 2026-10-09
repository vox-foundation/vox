//! Profile-gated boot runner.
//!
//! A boot plan is an ordered table of [`BootStep`]s. A step with `subsystem: Some(s)` runs
//! only when `s` is in the active profile's closure; its failure *degrades* boot (recorded in
//! the [`BootReport`], boot continues). A step with `subsystem: None` is core: it always runs
//! and its failure aborts boot, like the `?`s in the pre-profile `async_main`.

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;

use vox_orchestrator_mcp::profiles::{
    ActiveProfile, BootReport, DEV_PROFILE_ID, PROFILES, Subsystem,
};

pub type StepFuture<'a> = Pin<Box<dyn Future<Output = Result<(), String>> + 'a>>;
pub type StepFn<C> = for<'a> fn(&'a mut C) -> StepFuture<'a>;

pub struct BootStep<C> {
    pub name: &'static str,
    pub subsystem: Option<Subsystem>,
    /// Existed in `async_main` before capability profiles; `dev` must run these in their
    /// original relative order.
    pub legacy: bool,
    pub run: StepFn<C>,
}

pub trait BootContext {
    /// Called after every step that changed the report, so the live MCP gate never serves
    /// from a stale default while boot is still running.
    fn publish(&mut self, _report: &BootReport) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyMode {
    /// A step runs only if every direct dependency of its subsystem is booted.
    Strict,
    /// Steps run even when a dependency failed (pre-profile behaviour; `dev` only).
    Lenient,
}

impl DependencyMode {
    pub fn for_profile(profile: &ActiveProfile) -> Self {
        if profile.ids.contains(&DEV_PROFILE_ID) {
            Self::Lenient
        } else {
            Self::Strict
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BootOutcome {
    pub report: BootReport,
    pub ran: Vec<&'static str>,
    pub skipped: Vec<&'static str>,
}

/// Names of the steps a closure selects, in table order, without running anything.
pub fn planned_step_names<C>(
    steps: &[BootStep<C>],
    closure: &BTreeSet<Subsystem>,
) -> Vec<&'static str> {
    steps
        .iter()
        .filter(|s| s.subsystem.is_none_or(|sub| closure.contains(&sub)))
        .map(|s| s.name)
        .collect()
}

pub async fn run_boot_plan<C: BootContext>(
    steps: &[BootStep<C>],
    ctx: &mut C,
    closure: &BTreeSet<Subsystem>,
    mode: DependencyMode,
) -> anyhow::Result<BootOutcome> {
    let mut out = BootOutcome::default();
    for step in steps {
        let Some(sub) = step.subsystem else {
            (step.run)(ctx)
                .await
                .map_err(|e| anyhow::anyhow!("core boot step `{}` failed: {e}", step.name))?;
            out.ran.push(step.name);
            ctx.publish(&out.report);
            continue;
        };
        if !closure.contains(&sub) {
            out.skipped.push(step.name);
            continue;
        }
        if out.report.failed.contains_key(&sub) {
            out.skipped.push(step.name);
            continue;
        }
        // Nested so the strict-mode guard stays a single mutation site.
        #[allow(clippy::collapsible_if)]
        if mode == DependencyMode::Strict {
            if let Some(dep) = sub.deps().iter().copied().find(|d| !out.report.is_up(*d)) {
                tracing::warn!(
                    step = step.name,
                    subsystem = sub.as_str(),
                    dependency = dep.as_str(),
                    "boot step skipped: dependency not booted"
                );
                out.report
                    .failed
                    .insert(sub, format!("dependency {} failed", dep.as_str()));
                out.skipped.push(step.name);
                ctx.publish(&out.report);
                continue;
            }
        }
        match (step.run)(ctx).await {
            Ok(()) => {
                tracing::info!(step = step.name, subsystem = sub.as_str(), "boot step ok");
                out.report.booted.insert(sub);
            }
            Err(e) => {
                tracing::warn!(
                    step = step.name,
                    subsystem = sub.as_str(),
                    error = %e,
                    "boot step failed; continuing degraded"
                );
                out.report.booted.remove(&sub);
                out.report.failed.insert(sub, e);
            }
        }
        out.ran.push(step.name);
        ctx.publish(&out.report);
    }
    Ok(out)
}

/// Union of `requires` over the manifests named by `profile.ids` (the exit-rule roots).
pub fn profile_roots(profile: &ActiveProfile) -> BTreeSet<Subsystem> {
    PROFILES
        .iter()
        .filter(|m| profile.ids.contains(&m.id))
        .flat_map(|m| m.requires.iter().copied())
        .collect()
}

/// True when the profile required something and every required subsystem failed.
pub fn every_required_failed(roots: &BTreeSet<Subsystem>, report: &BootReport) -> bool {
    !roots.is_empty() && roots.iter().all(|s| report.failed.contains_key(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Fake {
        publishes: usize,
    }
    impl BootContext for Fake {
        fn publish(&mut self, _report: &BootReport) {
            self.publishes += 1;
        }
    }

    fn ok(_: &mut Fake) -> StepFuture<'_> {
        Box::pin(async { Ok(()) })
    }
    fn boom(_: &mut Fake) -> StepFuture<'_> {
        Box::pin(async { Err("boom".to_string()) })
    }
    fn st(name: &'static str, sub: Option<Subsystem>, run: StepFn<Fake>) -> BootStep<Fake> {
        BootStep {
            name,
            subsystem: sub,
            legacy: true,
            run,
        }
    }
    fn set(xs: &[Subsystem]) -> BTreeSet<Subsystem> {
        xs.iter().copied().collect()
    }

    #[tokio::test]
    async fn skips_steps_outside_closure() {
        let steps = [
            st("db", Some(Subsystem::Db), ok),
            st("models", Some(Subsystem::ModelRegistry), ok),
            st("core", None, ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::ModelRegistry]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["models", "core"]);
        assert_eq!(out.skipped, vec!["db"]);
        assert_eq!(out.report.booted, set(&[Subsystem::ModelRegistry]));
        assert!(out.report.failed.is_empty());
    }

    #[tokio::test]
    async fn runs_in_table_order_and_publishes_after_each_executed_step() {
        let steps = [
            st("a", Some(Subsystem::Attention), ok),
            st("core", None, ok),
            st("c", Some(Subsystem::Compiler), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Attention, Subsystem::Compiler]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["a", "core", "c"]);
        assert_eq!(c.publishes, 3);
    }

    #[tokio::test]
    async fn failed_step_degrades_and_boot_continues() {
        let steps = [
            st("db", Some(Subsystem::Db), boom),
            st("attention", Some(Subsystem::Attention), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Db, Subsystem::Attention]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["db", "attention"]);
        assert_eq!(
            out.report.failed.get(&Subsystem::Db).map(String::as_str),
            Some("boom")
        );
        assert_eq!(out.report.booted, set(&[Subsystem::Attention]));
    }

    #[tokio::test]
    async fn strict_mode_fails_dependents_with_dependency_message() {
        let steps = [
            st("db", Some(Subsystem::Db), boom),
            st("sessions", Some(Subsystem::Sessions), ok),
            st("agent_fleet", Some(Subsystem::AgentFleet), ok),
        ];
        let closure = set(&[Subsystem::Db, Subsystem::Sessions, Subsystem::AgentFleet]);
        let mut c = Fake::default();
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["db"]);
        assert_eq!(out.skipped, vec!["sessions", "agent_fleet"]);
        assert_eq!(
            out.report.failed[&Subsystem::Sessions],
            "dependency db failed"
        );
        assert_eq!(
            out.report.failed[&Subsystem::AgentFleet],
            "dependency sessions failed"
        );
        assert!(out.report.booted.is_empty());
    }

    #[tokio::test]
    async fn lenient_mode_still_runs_dependents_after_failure() {
        let steps = [
            st("db", Some(Subsystem::Db), boom),
            st("sessions", Some(Subsystem::Sessions), ok),
            st("agent_fleet", Some(Subsystem::AgentFleet), ok),
        ];
        let closure = set(&[Subsystem::Db, Subsystem::Sessions, Subsystem::AgentFleet]);
        let mut c = Fake::default();
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Lenient)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["db", "sessions", "agent_fleet"]);
        assert_eq!(
            out.report.failed.keys().copied().collect::<Vec<_>>(),
            vec![Subsystem::Db]
        );
        assert_eq!(
            out.report.booted,
            set(&[Subsystem::Sessions, Subsystem::AgentFleet])
        );
    }

    #[tokio::test]
    async fn later_steps_of_a_failed_subsystem_are_skipped() {
        let steps = [
            st("db", Some(Subsystem::Db), ok),
            st("mesh_a", Some(Subsystem::Mesh), boom),
            st("mesh_b", Some(Subsystem::Mesh), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Db, Subsystem::Mesh]);
        let out = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Lenient)
            .await
            .unwrap();
        assert_eq!(out.ran, vec!["db", "mesh_a"]);
        assert_eq!(out.skipped, vec!["mesh_b"]);
        assert!(!out.report.booted.contains(&Subsystem::Mesh));
    }

    #[tokio::test]
    async fn core_step_failure_aborts_boot() {
        let steps = [
            st("server_state", None, boom),
            st("attention", Some(Subsystem::Attention), ok),
        ];
        let mut c = Fake::default();
        let closure = set(&[Subsystem::Attention]);
        let err = run_boot_plan(&steps, &mut c, &closure, DependencyMode::Strict)
            .await
            .expect_err("core failure must abort");
        assert!(
            err.to_string()
                .contains("core boot step `server_state` failed: boom"),
            "{err}"
        );
    }

    #[test]
    fn planned_step_names_filters_by_closure_and_keeps_core() {
        let steps = [
            st("db", Some(Subsystem::Db), ok),
            st("core", None, ok),
            st("speech", Some(Subsystem::Speech), ok),
        ];
        assert_eq!(
            planned_step_names(&steps, &set(&[Subsystem::Speech])),
            vec!["core", "speech"]
        );
    }

    #[test]
    fn every_required_failed_only_when_all_roots_failed() {
        let roots = set(&[Subsystem::ModelRegistry, Subsystem::LlmFacade]);
        let mut r = BootReport::default();
        r.failed.insert(Subsystem::ModelRegistry, "x".into());
        assert!(!every_required_failed(&roots, &r));
        r.failed.insert(
            Subsystem::LlmFacade,
            "dependency model_registry failed".into(),
        );
        assert!(every_required_failed(&roots, &r));
        assert!(!every_required_failed(&BTreeSet::new(), &r));
    }

    #[test]
    fn dependency_mode_is_lenient_only_for_dev() {
        assert_eq!(
            DependencyMode::for_profile(&ActiveProfile::dev()),
            DependencyMode::Lenient
        );
        let models = ActiveProfile::resolve(&["models"]).expect("models profile exists");
        assert_eq!(DependencyMode::for_profile(&models), DependencyMode::Strict);
    }

    #[test]
    fn profile_roots_of_dev_is_every_subsystem() {
        let all: BTreeSet<Subsystem> = Subsystem::ALL.iter().copied().collect();
        assert_eq!(profile_roots(&ActiveProfile::dev()), all);
    }

    #[test]
    fn profile_roots_of_models_is_its_manifest_requires() {
        let models = ActiveProfile::resolve(&["models"]).expect("models profile exists");
        let manifest = PROFILES
            .iter()
            .find(|m| m.id == "models")
            .expect("manifest");
        assert_eq!(
            profile_roots(&models),
            manifest.requires.iter().copied().collect::<BTreeSet<_>>()
        );
    }
}
