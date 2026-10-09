//! Dev-only gamify ops for Axis (plan 07 Task 11).
use axum::Router;

use crate::http_gateway::GatewayState;

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
