//! Orchestrator-profile routes and ops for Axis (plan 07 Task 10).
use std::sync::Arc;

use axum::Router;

use crate::http_gateway::GatewayState;
use crate::server_state::ServerState;

/// The gateway's task hopper.
pub(crate) fn hopper_for(state: &ServerState) -> Arc<dyn vox_orchestrator::hopper::HopperIntake> {
    Arc::new(vox_orchestrator::hopper::InMemoryHopper::new(Arc::new(
        state.orchestrator.event_bus.clone(),
    )))
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
}

#[cfg(test)]
mod tests {
    #[test]
    fn router_builds() {
        let _ = super::api_v2_router();
    }
}
