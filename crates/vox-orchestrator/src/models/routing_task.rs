pub use vox_orchestrator_models::RoutingTask;

#[cfg(test)]
mod tests {
    use crate::config::CostPreference;
    use crate::models::spec::PricingSource;
    use crate::models::{
        ModelCapabilities, ModelRegistry, ModelSpec, ModelTier, ProviderType, RoutingTask,
        StrengthTag,
    };
    use crate::types::{AgentTask, TaskCategory, TaskId, TaskPriority};

    #[test]
    fn estimated_token_count_scales_with_category_and_squared_complexity() {
        let t = |category, complexity| RoutingTask {
            category,
            complexity,
            ..Default::default()
        };
        assert_eq!(t(TaskCategory::CodeGen, 5).estimated_token_count(), 2000);
        assert_eq!(t(TaskCategory::CodeGen, 10).estimated_token_count(), 8000);
        assert_eq!(t(TaskCategory::Research, 5).estimated_token_count(), 4000);
    }

    #[test]
    fn routing_task_from_agent_task_carries_category_and_complexity() {
        let mut task = AgentTask::new(TaskId(1), "test task", TaskPriority::Normal, vec![]);
        task.task_category = TaskCategory::CodeGen;
        task.estimated_complexity = 8;
        let rt = RoutingTask::from(&task);
        assert_eq!(rt.category, TaskCategory::CodeGen);
        assert_eq!(rt.complexity, 8);
    }

    #[test]
    fn best_for_task_with_routing_task_matches_agent_task() {
        let mut task = AgentTask::new(TaskId(1), "hard codegen", TaskPriority::Normal, vec![]);
        task.task_category = TaskCategory::CodeGen;
        task.estimated_complexity = 10;

        let mut r = ModelRegistry::default();
        let spec = |id: &str, tier: ModelTier, cost: f64| ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "test".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 200_000,
            cost_per_1k: cost,
            cost_per_1k_input: cost,
            cost_per_1k_output: cost,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Codegen, StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                tier,
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        };
        r.register(spec("acme/flagship-9", ModelTier::Elite, 0.0005));
        r.register(spec("acme/workhorse-9", ModelTier::Pro, 0.02));
        r.register(spec("acme/quick-9", ModelTier::Fast, 0.02));

        let rt: RoutingTask = (&task).into();
        let pick = r.best_for_task(&rt, CostPreference::Performance);
        assert_eq!(
            pick.as_ref().map(|m| m.id.as_str()),
            Some("acme/flagship-9")
        );
    }
}
