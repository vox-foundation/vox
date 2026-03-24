//! Routing service: file-affinity and group-based task routing.
//!
//! Decides which agent (existing or to be spawned) should receive a task
//! based on file manifest, affinity map, affinity groups, and load.

use std::collections::HashMap;
use dashmap::DashMap;

use crate::affinity::FileAffinityMap;
use crate::config::OrchestratorConfig;
use crate::contract::TaskCapabilityHints;
use crate::groups::AffinityGroupRegistry;
use crate::mesh_federation::RemoteMeshRoutingHint;
use crate::queue::AgentQueue;
use crate::types::{AgentId, FileAffinity};

/// Result of a routing decision: either use an existing agent or spawn one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteResult {
    /// Route to this existing agent.
    Existing(AgentId),
    /// Spawn a new agent with this name and route the task to it.
    SpawnAgent(String),
}

/// Stateless routing service implementing file-affinity and group voting.
pub struct RoutingService;

impl RoutingService {
    /// Route a task to the best agent based on file affinity and group voting.
    ///
    /// Returns either an existing agent ID or the name to use when spawning
    /// a new agent. The caller is responsible for spawning when `SpawnAgent` is returned.
    pub fn route(
        manifest: &[FileAffinity],
        affinity_map: &FileAffinityMap,
        groups: &AffinityGroupRegistry,
        agents: &DashMap<AgentId, AgentQueue>,
        config: &OrchestratorConfig,
        agent_reliability: Option<&HashMap<AgentId, f64>>,
        task_capability_requirements: Option<&TaskCapabilityHints>,
        remote_mesh_hints: Option<&[RemoteMeshRoutingHint]>,
    ) -> RouteResult {
        if manifest.is_empty() {
            return Self::least_loaded_or_spawn(agents, config);
        }

        let mut scores: HashMap<AgentId, f64> = HashMap::new();

        // 1. Direct file affinity (strongest signal)
        for fa in manifest {
            if let Some(owner) = affinity_map.lookup(&fa.path) {
                *scores.entry(owner).or_insert(0.0) += 10.0;
            }
        }

        // 2. Group affinity voting
        for fa in manifest {
            if let Some(group) = groups.resolve(&fa.path) {
                if let Some(default_agent) = group.default_agent {
                    if agents.contains_key(&default_agent) {
                        *scores.entry(default_agent).or_insert(0.0) += 15.0;
                    }
                }
                for pair in agents.iter() {
                    let agent_id = pair.key();
                    let queue = pair.value();
                    if queue.name == group.name {
                        *scores.entry(*agent_id).or_insert(0.0) += 5.0;
                    }
                }
            }
        }

        // 3. Weight by load (prefer emptier agents on ties)
        for (agent_id, score) in scores.iter_mut() {
            if let Some(queue) = agents.get(agent_id) {
                *score -= queue.weighted_load() * 0.1;
            }
        }

        // 3b. GPU / capability fit (penalize agents that cannot satisfy task requirements)
        if let Some(req) = task_capability_requirements {
            Self::apply_capability_penalties(&mut scores, agents, req);
        }

        // 3c. Experimental mesh visibility (read-only federation); never routes off-process.
        if config.mesh_routing_experimental {
            Self::apply_experimental_mesh_routing_signals(
                &mut scores,
                agents,
                task_capability_requirements,
                remote_mesh_hints,
            );
        }

        // 4. Optional reliability blend (Arca `agent_reliability`, schema V10)
        let rep_w = config.socrates_reputation_weight;
        if config.socrates_reputation_routing {
            for (agent_id, score) in scores.iter_mut() {
                let mut agent_base = 0.5;
                if let Some(rel) = agent_reliability {
                    if let Some(r) = rel.get(agent_id) {
                        agent_base = *r;
                    }
                }
                *score += agent_base * rep_w;

                // Blend in skill & workflow EWMA scores if present
                if let Some(queue) = agents.get(agent_id) {
                    let mut skill_rel_sum = 0.0;
                    let mut skill_count = 0;
                    for rel in queue.active_skills.values() {
                        skill_rel_sum += *rel;
                        skill_count += 1;
                    }
                    if skill_count > 0 {
                        // Blend average skill reliability at 50% reputation weight
                        *score += (skill_rel_sum / skill_count as f64) * rep_w * 0.5;
                    }
                    
                    // Small contextual boost if this agent is dedicated to a workflow
                    if queue.workflow_context.is_some() {
                        *score += rep_w * 0.1;
                    }
                }
            }
        }

        if let Some((&best_agent, _)) = scores
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            return RouteResult::Existing(best_agent);
        }

        // No overlap: spawn by group or fallback
        let group_name = manifest
            .first()
            .and_then(|fa| groups.resolve(&fa.path))
            .map(|g| g.name.clone());
        if let Some(name) = group_name {
            return RouteResult::SpawnAgent(name);
        }
        if config.fallback_to_single_agent {
            Self::least_loaded_or_spawn(agents, config)
        } else {
            RouteResult::SpawnAgent("general".to_string())
        }
    }

    fn apply_capability_penalties(
        scores: &mut HashMap<AgentId, f64>,
        agents: &DashMap<AgentId, AgentQueue>,
        req: &TaskCapabilityHints,
    ) {
        const PENALTY: f64 = 10_000.0;
        if req.gpu_cuda {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    if !q.capabilities.gpu_cuda {
                        *score -= PENALTY;
                    }
                }
            }
        }
        if req.gpu_metal {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    if !q.capabilities.gpu_metal {
                        *score -= PENALTY;
                    }
                }
            }
        }
        if req.gpu_vulkan {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    if !q.capabilities.gpu_vulkan {
                        *score -= PENALTY;
                    }
                }
            }
        }
        if req.gpu_webgpu {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    if !q.capabilities.gpu_webgpu {
                        *score -= PENALTY;
                    }
                }
            }
        }
        if req.npu {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    if !q.capabilities.npu {
                        *score -= PENALTY;
                    }
                }
            }
        }
        if let Some(min_v) = req.min_vram_mb {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    let ok = q
                        .capabilities
                        .min_vram_mb
                        .map(|v| v >= min_v)
                        .unwrap_or(false);
                    if !ok {
                        *score -= PENALTY * 0.5;
                    }
                }
            }
        }
        if let Some(min_c) = req.min_cpu_cores {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    let have = q.capabilities.cpu_cores.unwrap_or(0);
                    if have < min_c {
                        *score -= PENALTY * 0.5;
                    }
                }
            }
        }
        if req.prefer_gpu_compute {
            for (agent_id, score) in scores.iter_mut() {
                if let Some(q) = agents.get(agent_id) {
                    let has_gpu = q.capabilities.gpu_cuda
                        || q.capabilities.gpu_metal
                        || q.capabilities.gpu_vulkan
                        || q.capabilities.gpu_webgpu
                        || q.capabilities.npu;
                    if !has_gpu {
                        *score -= PENALTY * 0.15;
                    }
                }
            }
        }
    }

    fn labels_cover(have: &[String], need: &[String]) -> bool {
        need.iter()
            .all(|n| have.iter().any(|h| h.as_str() == n.as_str()))
    }

    fn remote_hint_matches_task(r: &RemoteMeshRoutingHint, req: &TaskCapabilityHints) -> bool {
        if req.labels.is_empty() {
            return false;
        }
        Self::labels_cover(&r.labels, &req.labels)
            && (!req.gpu_cuda || r.gpu_cuda)
            && (!req.gpu_metal || r.gpu_metal)
    }

    /// Soft score bump + tracing when cached remote mesh nodes align with task labels (no remote execute).
    fn apply_experimental_mesh_routing_signals(
        scores: &mut HashMap<AgentId, f64>,
        agents: &DashMap<AgentId, AgentQueue>,
        task_capability_requirements: Option<&TaskCapabilityHints>,
        remote_mesh_hints: Option<&[RemoteMeshRoutingHint]>,
    ) {
        const LABEL_BUMP: f64 = 0.25;
        let Some(req) = task_capability_requirements else {
            return;
        };
        let Some(remote) = remote_mesh_hints.filter(|s| !s.is_empty()) else {
            return;
        };
        if !req.labels.is_empty() {
            let local_matches = agents
                .iter()
                .any(|pair| Self::labels_cover(&pair.value().capabilities.labels, &req.labels));
            let remote_candidates = remote
                .iter()
                .filter(|r| Self::remote_hint_matches_task(r, req))
                .count();
            if !local_matches && remote_candidates > 0 {
                tracing::info!(
                    target: "vox.orchestrator.routing",
                    decision = "remote_label_match_only",
                    remote_candidates,
                    "mesh_routing_experimental: no local agent matches task labels; mesh lists remote candidates (no remote execute)"
                );
            } else if local_matches && remote_candidates > 0 {
                tracing::debug!(
                    target: "vox.orchestrator.routing",
                    decision = "local_and_remote_label_match",
                    remote_candidates,
                    "mesh_routing_experimental: preferring local placement"
                );
            }
            if local_matches {
                for (agent_id, score) in scores.iter_mut() {
                    if let Some(q) = agents.get(agent_id) {
                        if Self::labels_cover(&q.capabilities.labels, &req.labels) {
                            *score += LABEL_BUMP;
                        }
                    }
                }
            }
        }
        if req.prefer_gpu_compute || req.gpu_cuda || req.gpu_metal {
            let remote_gpu = remote.iter().filter(|r| r.gpu_cuda || r.gpu_metal).count();
            if remote_gpu > 0 {
                tracing::trace!(
                    target: "vox.orchestrator.routing",
                    remote_gpu_candidates = remote_gpu,
                    "mesh federation GPU visibility (experimental)"
                );
            }
        }
    }

    /// Choose least-loaded existing agent or request spawn of "default".
    pub fn least_loaded_or_spawn(
        agents: &DashMap<AgentId, AgentQueue>,
        _config: &OrchestratorConfig,
    ) -> RouteResult {
        if agents.is_empty() {
            return RouteResult::SpawnAgent("default".to_string());
        }
        let least_loaded = agents
            .iter()
            .min_by(|pair_a, pair_b| {
                pair_a.value().weighted_load()
                    .partial_cmp(&pair_b.value().weighted_load())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|pair| *pair.key());
        match least_loaded {
            Some(id) => RouteResult::Existing(id),
            None => RouteResult::SpawnAgent("default".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::groups::AffinityGroup;
    use crate::mesh_federation::RemoteMeshRoutingHint;
    use crate::types::{TaskId, TaskPriority};

    #[test]
    fn reliability_blend_prefers_higher_reliability_when_enabled() {
        let manifest = vec![FileAffinity::write("src/lib.rs")];
        let affinity = FileAffinityMap::new();
        let groups = AffinityGroupRegistry::new(vec![AffinityGroup {
            name: "core-group".to_string(),
            patterns: vec!["**/src/**".to_string()],
            default_agent: None,
        }]);

        let agents = DashMap::new();
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        agents.insert(a1, AgentQueue::new(a1, "core-group"));
        agents.insert(a2, AgentQueue::new(a2, "core-group"));

        let mut config = OrchestratorConfig::for_testing();
        config.socrates_reputation_routing = true;

        let mut rel = HashMap::new();
        rel.insert(a1, 0.15);
        rel.insert(a2, 0.95);

        let route = RoutingService::route(
            &manifest,
            &affinity,
            &groups,
            &agents,
            &config,
            Some(&rel),
            None,
            None,
        );
        assert_eq!(route, RouteResult::Existing(a2));
    }

    #[test]
    fn least_loaded_fallback_prefers_lower_weighted_queue() {
        let agents = DashMap::new();
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        let mut q1 = AgentQueue::new(a1, "one");
        let mut q2 = AgentQueue::new(a2, "two");
        q1.enqueue(crate::types::AgentTask::new(
            TaskId(1),
            "urgent",
            TaskPriority::Urgent,
            vec![],
        ));
        q2.enqueue(crate::types::AgentTask::new(
            TaskId(2),
            "normal",
            TaskPriority::Normal,
            vec![],
        ));
        agents.insert(a1, q1);
        agents.insert(a2, q2);

        let route = RoutingService::least_loaded_or_spawn(&agents, &OrchestratorConfig::default());
        assert_eq!(route, RouteResult::Existing(a2));
    }

    #[test]
    fn prefer_gpu_compute_soft_penalty_favors_gpu_agent() {
        let manifest = vec![FileAffinity::write("src/lib.rs")];
        let affinity = FileAffinityMap::new();
        let groups = AffinityGroupRegistry::new(vec![AffinityGroup {
            name: "core-group".to_string(),
            patterns: vec!["**/src/**".to_string()],
            default_agent: None,
        }]);

        let agents = DashMap::new();
        let cpu = AgentId(1);
        let gpu = AgentId(2);
        let mut q_cpu = AgentQueue::new(cpu, "core-group");
        q_cpu.capabilities.gpu_cuda = false;
        let mut q_gpu = AgentQueue::new(gpu, "core-group");
        q_gpu.capabilities.gpu_cuda = true;
        agents.insert(cpu, q_cpu);
        agents.insert(gpu, q_gpu);

        let hints = TaskCapabilityHints {
            prefer_gpu_compute: true,
            ..Default::default()
        };
        let route = RoutingService::route(
            &manifest,
            &affinity,
            &groups,
            &agents,
            &OrchestratorConfig::default(),
            None,
            Some(&hints),
            None,
        );
        assert_eq!(route, RouteResult::Existing(gpu));
    }

    #[test]
    fn experimental_mesh_bumps_local_when_labels_match() {
        let manifest = vec![FileAffinity::write("src/lib.rs")];
        let affinity = FileAffinityMap::new();
        let groups = AffinityGroupRegistry::new(vec![AffinityGroup {
            name: "core-group".to_string(),
            patterns: vec!["**/src/**".to_string()],
            default_agent: None,
        }]);

        let agents = DashMap::new();
        let a1 = AgentId(1);
        let a2 = AgentId(2);
        let mut q1 = AgentQueue::new(a1, "core-group");
        q1.capabilities.labels = vec!["pool=a".to_string()];
        let mut q2 = AgentQueue::new(a2, "core-group");
        q2.capabilities.labels = vec!["pool=b".to_string()];
        agents.insert(a1, q1);
        agents.insert(a2, q2);

        let mut config = OrchestratorConfig::for_testing();
        config.mesh_routing_experimental = true;

        let hints = TaskCapabilityHints {
            labels: vec!["pool=a".to_string()],
            ..Default::default()
        };
        let remote = vec![RemoteMeshRoutingHint {
            node_id: "remote-1".into(),
            labels: vec!["pool=a".to_string()],
            gpu_cuda: false,
            gpu_metal: false,
        }];
        let route = RoutingService::route(
            &manifest,
            &affinity,
            &groups,
            &agents,
            &config,
            None,
            Some(&hints),
            Some(remote.as_slice()),
        );
        assert_eq!(route, RouteResult::Existing(a1));
    }
}
