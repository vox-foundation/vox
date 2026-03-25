use super::errors::ConfigValidationError;
use super::orchestrator_fields::OrchestratorConfig;

impl OrchestratorConfig {
    /// Validates the configuration against required invariants.
    pub fn validate(&self) -> Result<(), Vec<ConfigValidationError>> {
        let mut errors = Vec::new();

        if self.max_agents < 1 {
            errors.push(ConfigValidationError::InvalidMaxAgents(self.max_agents));
        }
        if self.lock_timeout_ms < 100 {
            errors.push(ConfigValidationError::InvalidLockTimeout(
                self.lock_timeout_ms,
            ));
        }
        if self.bulletin_capacity < 1 {
            errors.push(ConfigValidationError::InvalidBulletinCapacity(
                self.bulletin_capacity,
            ));
        }
        if self.min_agents > self.max_agents {
            errors.push(ConfigValidationError::InvalidScalingLimits(
                self.min_agents,
                self.max_agents,
            ));
        }
        if self.planning_router_enabled && !self.planning_enabled {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_router_enabled requires planning_enabled".to_string(),
            ));
        }
        if self.planning_replan_enabled && !self.planning_enabled {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_replan_enabled requires planning_enabled".to_string(),
            ));
        }
        if self.planning_workflow_handoff_enabled && !self.planning_enabled {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_workflow_handoff_enabled requires planning_enabled".to_string(),
            ));
        }
        if self.planning_rollout_percent > 100 {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_rollout_percent must be <= 100".to_string(),
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
