use vox_orchestrator_types::TaskCategory;

/// Narrow view of a task used for model routing and selection.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RoutingTask {
    pub category: TaskCategory,
    pub complexity: u8,
    pub research_hints: Vec<String>,
    pub tool_hints: Vec<String>,
    pub max_cost_usd: Option<f64>,
    pub context_fill_ratio: Option<f32>,
}

impl RoutingTask {
    /// Predict the number of tokens this task will consume based on its complexity and category.
    #[must_use]
    pub fn estimated_token_count(&self) -> u64 {
        let base = match self.category {
            TaskCategory::CodeGen => 2000,
            TaskCategory::Research => 4000,
            TaskCategory::Visus => 8000,
            _ => 1000,
        };
        let complexity_mult = f64::from(self.complexity).powi(2) / 25.0; // 5 is 1.0, 10 is 4.0
        (base as f64 * complexity_mult).round() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
