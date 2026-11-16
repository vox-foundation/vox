//! Translate GSD execution waves into validated `vox-orchestrator` `PlanNode` DAGs.

use crate::types::FileAffinity;
use std::path::PathBuf;
use vox_orchestrator_types::gsd::GsdPlanWave;

use crate::orchestrator::OrchestratorError;
use crate::planning::{ExecutionPolicy, PlanNode, PlanStatus, quality_gate};

/// Convert wave-grouped GSD tasks into `PlanNode` entries where wave `k` depends on every
/// node in wave `k - 1`, preserving parallel execution inside each wave.
pub fn waves_to_plan_nodes(
    phase_number: u32,
    waves: &[GsdPlanWave],
) -> Result<Vec<PlanNode>, OrchestratorError> {
    let mut nodes = Vec::new();
    let mut prev_wave_ids: Vec<String> = Vec::new();
    let mut sorted = waves.to_vec();
    sorted.sort_by_key(|w| w.wave_number);

    for wave in sorted {
        let mut current_wave_ids = Vec::with_capacity(wave.tasks.len());
        for task in wave.tasks {
            let node_id = format!("p{}-w{}-{}", phase_number, wave.wave_number, task.task_id);
            let file_manifest = if task.files.is_empty() {
                vec![FileAffinity::read("Cargo.toml")]
            } else {
                task.files
                    .iter()
                    .map(|p| FileAffinity::write(PathBuf::from(p)))
                    .collect()
            };
            let description = format!(
                "[GSD Phase {} Wave {}] {} (verify: `{}`)",
                phase_number, wave.wave_number, task.title, task.verification_cmd
            );
            let node = PlanNode {
                node_id: node_id.clone(),
                description,
                depends_on: prev_wave_ids.clone(),
                status: PlanStatus::Pending,
                execution_policy: ExecutionPolicy {
                    allowed_skills: vec!["gsd-execute-phase".to_string()],
                    file_manifest,
                    ..ExecutionPolicy::default()
                },
                workflow_invocation: Some("gsd-execute-phase".to_string()),
            };
            current_wave_ids.push(node_id);
            nodes.push(node);
        }
        if !current_wave_ids.is_empty() {
            prev_wave_ids = current_wave_ids;
        }
    }

    quality_gate::validate_plan_nodes(&nodes)?;
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::gsd::parser::parse_phase_plan_waves;

    #[test]
    fn waves_translate_into_layered_plan_dag() {
        let md = "# Phase 408: Plan\n\n## Wave 1\n\n- [ ] `t1` Add types | files: crates/vox-orchestrator-types/src/gsd.rs | verify: cargo test -p vox-orchestrator-types\n- [ ] `t2` Add docs | files: docs/src/architecture/gsd-integration-ssot-2026.md | verify: cargo check\n\n## Wave 2\n\n- [ ] `t3` Add DB ops | files: crates/vox-db/src/store/ops_gsd.rs | verify: cargo test -p vox-db ops_gsd\n";
        let waves = parse_phase_plan_waves(md);
        assert_eq!(waves.len(), 2);
        let nodes = waves_to_plan_nodes(408, &waves).expect("valid DAG");
        assert_eq!(nodes.len(), 3);
        assert!(nodes[0].depends_on.is_empty());
        assert!(nodes[1].depends_on.is_empty());
        assert_eq!(nodes[2].depends_on.len(), 2);
        assert!(nodes[2].depends_on.contains(&nodes[0].node_id));
        assert!(nodes[2].depends_on.contains(&nodes[1].node_id));
    }
}
