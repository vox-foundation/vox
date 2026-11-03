//! `GET /v1/profile` — active capability profile and the tools served to the calling token.

use super::*;
use crate::profiles::{Exposure, Subsystem};

#[derive(Debug, Serialize)]
pub(super) struct ProfileInfo {
    profiles: Vec<&'static str>,
    max_exposure: Exposure,
    subsystems: ProfileSubsystems,
    tools: Vec<String>,
    axis_surfaces: Vec<&'static str>,
    server_version: &'static str,
}

#[derive(Debug, Serialize)]
struct ProfileSubsystems {
    booted: Vec<Subsystem>,
    failed: Vec<FailedSubsystem>,
}

#[derive(Debug, Serialize)]
struct FailedSubsystem {
    id: Subsystem,
    error: String,
}

pub(super) fn profile_info(state: &GatewayState, ctx: CallContext) -> ProfileInfo {
    let profile = &state.server_state.active_profile;
    let mut profiles = profile.ids.clone();
    profiles.sort_unstable();
    let (mut booted, mut failed) = {
        let boot = state.server_state.boot_report.read();
        let booted: Vec<Subsystem> = boot.booted.iter().copied().collect();
        let failed: Vec<FailedSubsystem> = boot
            .failed
            .iter()
            .map(|(s, e)| FailedSubsystem {
                id: *s,
                error: e.clone(),
            })
            .collect();
        (booted, failed)
    };
    booted.sort_by_key(|s| s.as_str());
    failed.sort_by_key(|f| f.id.as_str());
    // Same set `/v1/tools` advertises: it lists only compiled registry tools.
    let mut tools: Vec<String> = visible_tools_for(state, ctx)
        .into_iter()
        .filter(|n| {
            crate::profiles::registry_entry(n).is_some()
                && crate::registry::dispatchable_under_features(n)
        })
        .collect();
    tools.sort();
    let mut axis_surfaces: Vec<&'static str> = profile.axis_surfaces.iter().copied().collect();
    axis_surfaces.sort_unstable();
    ProfileInfo {
        profiles,
        max_exposure: profile.max_exposure,
        subsystems: ProfileSubsystems { booted, failed },
        tools,
        axis_surfaces,
        server_version: env!("CARGO_PKG_VERSION"),
    }
}

pub(super) async fn http_profile(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return *resp;
    }
    let role = match resolve_access_role(&state, &headers, Some(&connect.0)) {
        Ok(r) => r,
        Err(msg) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response();
        }
    };
    let ctx = call_context(&state, &connect.0, role);
    Json(profile_info(&state, ctx)).into_response()
}
