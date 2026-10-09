//! Capability-profile surface for MCP/HTTP hosts: re-exports the `vox-mcp-registry` profile types
//! and adds the `ServerState`-aware guard every advertise and dispatch path calls.

pub use vox_mcp_registry::{
    ActiveProfile, BootReport, CallContext, CallerRole, DEV_PROFILE_ID, Exposure,
    McpToolRegistryEntry, PROFILES, ProfileDenial, ProfileError, ProfileManifest, SideEffectClass,
    Subsystem, authorize_tool, registry_entry, subsystem_closure,
};

use crate::server_state::ServerState;

const DENIAL_CODES: &[&str] = &[
    "tool_not_in_profile",
    "subsystem_not_booted",
    "exposure_denied",
    "insufficient_role",
];

pub fn is_dev(profile: &ActiveProfile) -> bool {
    profile.ids.as_slice() == [DEV_PROFILE_ID]
}

/// Boot report used until plan 03's conditioned boot fills the real one.
pub fn full_boot_report() -> BootReport {
    BootReport {
        booted: Subsystem::ALL.iter().copied().collect(),
        failed: Default::default(),
    }
}

/// Profile gate for one canonical tool name. Names outside `TOOL_REGISTRY` (workspace `@tool`s,
/// skill macro tools, unknown names) keep today's handling under `dev` and are denied otherwise.
pub fn guard_tool(
    state: &ServerState,
    ctx: CallContext,
    canonical: &str,
) -> Result<(), ProfileDenial> {
    if registry_entry(canonical).is_none() {
        return if is_dev(&state.active_profile) {
            Ok(())
        } else {
            Err(ProfileDenial::ToolNotInProfile)
        };
    }
    let boot = state.boot_report.read();
    authorize_tool(&state.active_profile, &boot, ctx, canonical).map(|_| ())
}

pub fn denial_error_json(
    tool: &str,
    denial: &ProfileDenial,
    profile: &ActiveProfile,
) -> serde_json::Value {
    let message = match denial {
        ProfileDenial::ToolNotInProfile => {
            format!(
                "tool {tool} is not in the active profile ({})",
                profile.id_string()
            )
        }
        ProfileDenial::SubsystemNotBooted(s) => {
            format!(
                "tool {tool} requires subsystem {}, which is not running",
                s.as_str()
            )
        }
        ProfileDenial::ExposureDenied => format!("tool {tool} is not exposed to this caller"),
        ProfileDenial::InsufficientRole => format!("tool {tool} requires a write token"),
    };
    let mut error = serde_json::json!({ "code": denial.code(), "message": message });
    if let ProfileDenial::SubsystemNotBooted(s) = denial {
        error["subsystem"] = serde_json::Value::String(s.as_str().to_string());
    }
    serde_json::json!({ "error": error })
}

pub fn is_profile_denial_json(v: &serde_json::Value) -> bool {
    v.get("error")
        .and_then(|e| e.get("code"))
        .and_then(|c| c.as_str())
        .is_some_and(|c| DENIAL_CODES.contains(&c))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_state::ServerState;

    #[test]
    fn is_dev_only_for_the_dev_profile() {
        assert!(is_dev(&ActiveProfile::dev()));
        assert!(!is_dev(
            &ActiveProfile::resolve(&["models"]).expect("models")
        ));
    }

    #[test]
    fn full_boot_report_has_every_subsystem_up() {
        let r = full_boot_report();
        for s in Subsystem::ALL {
            assert!(r.is_up(*s));
        }
        assert!(r.failed.is_empty());
    }

    #[tokio::test]
    async fn default_state_is_dev_with_everything_booted() {
        let state = ServerState::new_test().await;
        assert!(is_dev(&state.active_profile));
        assert_eq!(*state.boot_report.read(), full_boot_report());
    }

    #[tokio::test]
    async fn guard_passes_non_registry_names_only_under_dev() {
        let dev = ServerState::new_test().await;
        assert_eq!(
            guard_tool(&dev, CallContext::LOCAL_WRITE, "read_file"),
            Ok(())
        );
        let models = dev
            .clone()
            .with_profile(ActiveProfile::resolve(&["models"]).expect("models"));
        assert_eq!(
            guard_tool(&models, CallContext::LOCAL_WRITE, "read_file"),
            Err(ProfileDenial::ToolNotInProfile)
        );
    }

    #[tokio::test]
    async fn guard_filters_registry_tools_by_profile() {
        let state = ServerState::new_test()
            .await
            .with_profile(ActiveProfile::resolve(&["models"]).expect("models"));
        assert_eq!(
            guard_tool(&state, CallContext::LOCAL_WRITE, "vox_list_models"),
            Ok(())
        );
        assert_eq!(
            guard_tool(&state, CallContext::LOCAL_WRITE, "vox_git_status"),
            Err(ProfileDenial::ToolNotInProfile)
        );
    }

    #[tokio::test]
    async fn guard_reports_subsystem_not_booted_from_state_boot_report() {
        let profile = ActiveProfile::resolve(&["knowledge"]).expect("knowledge");
        let tool = *profile
            .tools
            .iter()
            .find(|t| !registry_entry(t).expect("registry").requires.is_empty())
            .expect("knowledge has a tool with requires");
        let first = registry_entry(tool).expect("registry").requires[0];
        let state = ServerState::new_test().await.with_profile(profile);
        *state.boot_report.write() = BootReport::default();
        assert_eq!(
            guard_tool(&state, CallContext::LOCAL_WRITE, tool),
            Err(ProfileDenial::SubsystemNotBooted(first))
        );
    }

    #[test]
    fn denial_json_matches_the_contract_shape() {
        let v = denial_error_json(
            "vox_kb_query",
            &ProfileDenial::SubsystemNotBooted(Subsystem::SearchIndex),
            &ActiveProfile::dev(),
        );
        assert_eq!(
            v,
            serde_json::json!({"error": {
                "code": "subsystem_not_booted",
                "message": "tool vox_kb_query requires subsystem search_index, which is not running",
                "subsystem": "search_index"
            }})
        );
        let v = denial_error_json(
            "vox_git_status",
            &ProfileDenial::ToolNotInProfile,
            &ActiveProfile::resolve(&["models"]).expect("models"),
        );
        assert_eq!(v["error"]["code"], "tool_not_in_profile");
        assert_eq!(
            v["error"]["message"],
            "tool vox_git_status is not in the active profile (models)"
        );
        assert!(v["error"].get("subsystem").is_none());
    }

    #[test]
    fn is_profile_denial_json_recognizes_only_denial_codes() {
        let v = denial_error_json("x", &ProfileDenial::ExposureDenied, &ActiveProfile::dev());
        assert!(is_profile_denial_json(&v));
        assert!(!is_profile_denial_json(
            &serde_json::json!({"error": {"code": "other"}})
        ));
        assert!(!is_profile_denial_json(
            &serde_json::json!({"success": false, "error": "boom"})
        ));
    }

    #[test]
    fn every_bundle_profile_id_resolves() {
        for b in vox_plugin_catalog::all_bundles() {
            let ids: Vec<&str> = b.profiles.iter().map(String::as_str).collect();
            assert!(
                ActiveProfile::resolve(&ids).is_ok(),
                "bundle {} has unknown profile ids {ids:?}",
                b.id
            );
        }
    }
}
