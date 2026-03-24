use vox_orchestrator::{
    services::{RoutingService, RouteResult},
    OrchestratorConfig, AgentTask, AgentId, TaskId, TaskPriority,
    affinity::FileAffinityMap,
    groups::AffinityGroupRegistry,
    queue::AgentQueue,
    contract::TaskCapabilityHints,
    types::FileAffinity
};
use dashmap::DashMap;

#[tokio::test]
async fn test_routing_service_affinity_assignment() {
    let mut config = OrchestratorConfig::default();
    config.default_agent_capabilities.labels.push("research".to_string());
    
    let affinity_map = FileAffinityMap::new();
    let groups = AffinityGroupRegistry::defaults();
    let agents = DashMap::new();
    
    let a1 = AgentId(1);
    let mut q1 = AgentQueue::new(a1, "pm-group");
    q1.capabilities.labels.push("research".to_string());
    agents.insert(a1, q1);

    let task_manifest = vec![FileAffinity::write("crates/vox-pm/src/lib.rs")];
    let caps = TaskCapabilityHints {
        labels: vec!["research".to_string()],
        ..Default::default()
    };

    // Route using the static method
    let route = RoutingService::route(
        &task_manifest,
        &affinity_map,
        &groups,
        &agents,
        &config,
        None,
        Some(&caps),
        None,
    );
    
    match route {
        RouteResult::Existing(id) => assert_eq!(id, a1),
        _ => panic!("Expected existing agent routing"),
    }
}

#[tokio::test]
async fn test_routing_service_load_balancing() {
    let config = OrchestratorConfig::default();
    let affinity_map = FileAffinityMap::new();
    let groups = AffinityGroupRegistry::defaults();
    let agents = DashMap::new();
    
    let a1 = AgentId(1);
    let a2 = AgentId(2);
    
    let q1 = AgentQueue::new(a1, "a1");
    let q2 = AgentQueue::new(a2, "a2");
    
    agents.insert(a1, q1);
    agents.insert(a2, q2);
    
    let task_manifest = vec![]; // No file affinity
    
    let route = RoutingService::route(
        &task_manifest,
        &affinity_map,
        &groups,
        &agents,
        &config,
        None,
        None,
        None,
    );
    
    match route {
        RouteResult::Existing(id) => assert!(id == a1 || id == a2),
        _ => panic!("Expected existing agent routing"),
    }
}
