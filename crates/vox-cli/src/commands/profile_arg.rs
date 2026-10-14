//! Shared `--profile <ids>` resolution for `vox mcp` and `vox serve`.

use anyhow::Result;
use vox_mcp_registry::ActiveProfile;

const PROFILE_KEY: &str = "VOX_PROFILE";

/// `--profile` > `VOX_PROFILE` env > `VOX_PROFILE` in `~/.vox/config.toml` > `dev`.
pub fn resolve_cli_profile(flag: Option<&str>) -> Result<ActiveProfile> {
    let configured = vox_config::env_parse::resolve_config_opt_str(PROFILE_KEY);
    resolve_profile_from(flag, configured.as_deref())
}

fn resolve_profile_from(flag: Option<&str>, configured: Option<&str>) -> Result<ActiveProfile> {
    let raw = flag
        .filter(|s| !s.trim().is_empty())
        .or(configured)
        .unwrap_or("");
    let ids: Vec<&str> = raw.split(',').collect();
    Ok(ActiveProfile::resolve(&ids)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_wins_over_configured() {
        let p = resolve_profile_from(Some("models"), Some("mesh")).expect("resolve");
        assert_eq!(p.id_string(), "models");
    }

    #[test]
    fn configured_is_used_without_flag() {
        let p = resolve_profile_from(None, Some("knowledge, models")).expect("resolve");
        assert_eq!(p.id_string(), "knowledge,models");
    }

    #[test]
    fn blank_flag_falls_through_and_nothing_means_dev() {
        assert_eq!(
            resolve_profile_from(Some("  "), None)
                .expect("dev")
                .id_string(),
            "dev"
        );
        assert_eq!(
            resolve_profile_from(None, None).expect("dev").id_string(),
            "dev"
        );
    }

    #[test]
    fn unknown_id_is_profile_unknown() {
        let err = resolve_profile_from(Some("nope"), None).expect_err("unknown");
        assert_eq!(err.to_string(), "profile_unknown: nope");
    }

    #[test]
    fn resolve_cli_profile_with_flag_ignores_env_and_config() {
        assert_eq!(
            resolve_cli_profile(Some("mesh")).expect("mesh").id_string(),
            "mesh"
        );
    }
}
