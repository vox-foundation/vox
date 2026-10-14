//! `vox doctor`: effective MCP HTTP gateway bind policy (capability-profiles plan 04).

use super::super::common::Check;
use vox_orchestrator_mcp::http_gateway::bind_policy::{BindPolicy, BindPolicyError};

const CHECK_NAME: &str = "MCP HTTP gateway bind policy";

pub(super) fn run(checks: &mut Vec<Check>) {
    let enabled = vox_orchestrator_mcp::http_gateway::http_gateway_enabled();
    let result = vox_orchestrator_mcp::http_gateway::bind_policy::evaluate_bind_policy(
        vox_orchestrator_mcp::http_gateway::resolve_bind_policy_input(),
    );
    checks.push(bind_policy_check(enabled, &result));
}

fn bind_policy_check(enabled: bool, result: &Result<BindPolicy, BindPolicyError>) -> Check {
    match (enabled, result) {
        (true, Ok(p)) => Check::pass(CHECK_NAME, format!("enabled; {}", p.summary())),
        (false, Ok(p)) => Check::pass(
            CHECK_NAME,
            format!(
                "disabled (VOX_MCP_HTTP_ENABLED unset); would bind {}",
                p.summary()
            ),
        ),
        (true, Err(e)) => Check::fail(CHECK_NAME, e.to_string()),
        (false, Err(e)) => Check::pass(
            CHECK_NAME,
            format!("disabled (VOX_MCP_HTTP_ENABLED unset); would refuse to start if enabled: {e}"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator_mcp::http_gateway::bind_policy::{
        BindPolicyInput, TlsMaterial, evaluate_bind_policy,
    };

    fn eval(host: &str) -> Result<BindPolicy, BindPolicyError> {
        evaluate_bind_policy(BindPolicyInput {
            host: host.to_string(),
            port: 3921,
            has_write_token: false,
            has_read_token: false,
            tls: TlsMaterial::Absent,
            behind_tunnel: false,
            allow_unauthenticated: false,
        })
    }

    #[test]
    fn enabled_loopback_passes_with_summary() {
        let c = bind_policy_check(true, &eval("127.0.0.1"));
        assert!(c.pass);
        assert_eq!(c.name, CHECK_NAME);
        assert!(
            c.detail
                .contains("127.0.0.1:3921 — loopback only, plain HTTP"),
            "{}",
            c.detail
        );
    }

    #[test]
    fn enabled_unsafe_network_bind_fails_naming_what_is_missing() {
        let c = bind_policy_check(true, &eval("0.0.0.0"));
        assert!(!c.pass);
        assert!(
            c.detail.contains("VOX_MCP_HTTP_READ_BEARER_TOKEN"),
            "{}",
            c.detail
        );
    }

    #[test]
    fn disabled_gateway_passes_but_warns_about_a_bad_config() {
        let c = bind_policy_check(false, &eval("0.0.0.0"));
        assert!(c.pass);
        assert!(c.detail.contains("would refuse to start"), "{}", c.detail);
    }
}
