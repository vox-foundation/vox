//! MCP HTTP gateway bind policy (capability-profiles spec §6).
//!
//! Pure: no environment reads, no filesystem access. `resolve_bind_policy_input` in the parent
//! module gathers the inputs; gateway startup and `vox doctor` both call
//! [`evaluate_bind_policy`] so they can never disagree.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

/// TLS certificate/key material as resolved from `VOX_MCP_HTTP_TLS_CERT` / `_KEY`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TlsMaterial {
    Absent,
    Ready { cert: PathBuf, key: PathBuf },
    Invalid(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindPolicyInput {
    pub host: String,
    pub port: u16,
    pub has_write_token: bool,
    pub has_read_token: bool,
    pub tls: TlsMaterial,
    pub behind_tunnel: bool,
    pub allow_unauthenticated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindExposure {
    Loopback,
    Network,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GatewayTransport {
    PlainHttp,
    Tls { cert: PathBuf, key: PathBuf },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BindPolicy {
    pub addr: SocketAddr,
    pub exposure: BindExposure,
    pub transport: GatewayTransport,
    pub behind_tunnel: bool,
}

impl BindPolicy {
    /// One-line description used by the startup log and `vox doctor`.
    pub fn summary(&self) -> String {
        let scope = match self.exposure {
            BindExposure::Loopback => "loopback only",
            BindExposure::Network => "network-reachable",
        };
        let transport = match &self.transport {
            GatewayTransport::PlainHttp if self.behind_tunnel => {
                "plain HTTP behind an attested TLS tunnel".to_string()
            }
            GatewayTransport::PlainHttp => "plain HTTP".to_string(),
            GatewayTransport::Tls { cert, .. } => format!("TLS (cert {})", cert.display()),
        };
        format!("{} — {scope}, {transport}", self.addr)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindRequirement {
    WriteToken,
    ReadToken,
    TlsOrTunnel,
    AllowUnauthenticatedUnset,
}

impl BindRequirement {
    pub const fn describe(self) -> &'static str {
        match self {
            Self::WriteToken => {
                "write token VOX_MCP_HTTP_BEARER_TOKEN (`vox secrets set VOX_MCP_HTTP_BEARER_TOKEN --stdin`)"
            }
            Self::ReadToken => {
                "read token VOX_MCP_HTTP_READ_BEARER_TOKEN (`vox secrets set VOX_MCP_HTTP_READ_BEARER_TOKEN --stdin`)"
            }
            Self::TlsOrTunnel => {
                "TLS (VOX_MCP_HTTP_TLS_CERT + VOX_MCP_HTTP_TLS_KEY, or `--tls-cert`/`--tls-key`) or a TLS-terminating tunnel attestation (VOX_MCP_HTTP_BEHIND_TUNNEL=1, or `--behind-tunnel`)"
            }
            Self::AllowUnauthenticatedUnset => {
                "VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED must be unset (it is only honoured on loopback binds)"
            }
        }
    }
}

fn describe_missing(missing: &[BindRequirement]) -> String {
    missing
        .iter()
        .map(|r| r.describe())
        .collect::<Vec<_>>()
        .join("; ")
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BindPolicyError {
    #[error("invalid VOX_MCP_HTTP_HOST {host:?}: expected an IP address literal or `localhost`")]
    InvalidHost { host: String },
    #[error("invalid MCP HTTP gateway TLS configuration: {0}")]
    Tls(String),
    #[error(
        "refusing to start the MCP HTTP gateway on {addr} ({reason}); missing: {}",
        describe_missing(.missing)
    )]
    NetworkBindRequirements {
        addr: SocketAddr,
        reason: &'static str,
        missing: Vec<BindRequirement>,
    },
}

/// Parse a bind host: IP literal (IPv6 optionally bracketed) or `localhost`.
pub fn parse_bind_ip(host: &str) -> Option<IpAddr> {
    let h = host.trim();
    let h = h
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(h);
    if h.eq_ignore_ascii_case("localhost") {
        return Some(IpAddr::V4(Ipv4Addr::LOCALHOST));
    }
    h.parse().ok()
}

/// `127.0.0.0/8`, `::1`, and IPv4-mapped loopback (`::ffff:127.x.y.z`). `0.0.0.0` / `::` are not.
pub fn is_loopback_ip(ip: IpAddr) -> bool {
    ip.to_canonical().is_loopback()
}

pub fn evaluate_bind_policy(input: BindPolicyInput) -> Result<BindPolicy, BindPolicyError> {
    let ip = parse_bind_ip(&input.host).ok_or_else(|| BindPolicyError::InvalidHost {
        host: input.host.clone(),
    })?;
    let addr = SocketAddr::new(ip, input.port);
    let transport = match input.tls {
        TlsMaterial::Invalid(msg) => return Err(BindPolicyError::Tls(msg)),
        TlsMaterial::Absent => GatewayTransport::PlainHttp,
        TlsMaterial::Ready { cert, key } => GatewayTransport::Tls { cert, key },
    };
    let loopback = is_loopback_ip(ip);
    if loopback && !input.behind_tunnel {
        return Ok(BindPolicy {
            addr,
            exposure: BindExposure::Loopback,
            transport,
            behind_tunnel: false,
        });
    }

    let mut missing = Vec::new();
    if !input.has_write_token {
        missing.push(BindRequirement::WriteToken);
    }
    if !input.has_read_token {
        missing.push(BindRequirement::ReadToken);
    }
    if transport == GatewayTransport::PlainHttp && !input.behind_tunnel {
        missing.push(BindRequirement::TlsOrTunnel);
    }
    if input.allow_unauthenticated {
        missing.push(BindRequirement::AllowUnauthenticatedUnset);
    }
    if !missing.is_empty() {
        let reason = if loopback {
            "loopback bind behind a tunnel is reachable from the network"
        } else {
            "non-loopback bind"
        };
        return Err(BindPolicyError::NetworkBindRequirements {
            addr,
            reason,
            missing,
        });
    }
    Ok(BindPolicy {
        addr,
        exposure: BindExposure::Network,
        transport,
        behind_tunnel: input.behind_tunnel,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(host: &str) -> BindPolicyInput {
        BindPolicyInput {
            host: host.to_string(),
            port: 3921,
            has_write_token: false,
            has_read_token: false,
            tls: TlsMaterial::Absent,
            behind_tunnel: false,
            allow_unauthenticated: false,
        }
    }

    fn tls() -> TlsMaterial {
        TlsMaterial::Ready {
            cert: PathBuf::from("/etc/vox/cert.pem"),
            key: PathBuf::from("/etc/vox/key.pem"),
        }
    }

    fn missing_of(r: Result<BindPolicy, BindPolicyError>) -> Vec<BindRequirement> {
        match r {
            Err(BindPolicyError::NetworkBindRequirements { missing, .. }) => missing,
            other => panic!("expected NetworkBindRequirements, got {other:?}"),
        }
    }

    #[test]
    fn loopback_default_needs_no_tokens_or_tls() {
        let p = evaluate_bind_policy(input("127.0.0.1")).unwrap();
        assert_eq!(p.addr, "127.0.0.1:3921".parse::<SocketAddr>().unwrap());
        assert_eq!(p.exposure, BindExposure::Loopback);
        assert_eq!(p.transport, GatewayTransport::PlainHttp);
        assert!(!p.behind_tunnel);
    }

    #[test]
    fn whole_127_slash_8_is_loopback() {
        let p = evaluate_bind_policy(input("127.8.9.10")).unwrap();
        assert_eq!(p.exposure, BindExposure::Loopback);
    }

    #[test]
    fn ipv6_loopback_forms_are_loopback() {
        for host in ["::1", "[::1]", "::ffff:127.0.0.1"] {
            let p = evaluate_bind_policy(input(host)).unwrap_or_else(|e| panic!("{host}: {e}"));
            assert_eq!(p.exposure, BindExposure::Loopback, "{host}");
        }
    }

    #[test]
    fn localhost_maps_to_ipv4_loopback() {
        let p = evaluate_bind_policy(input("localhost")).unwrap();
        assert_eq!(p.addr, "127.0.0.1:3921".parse::<SocketAddr>().unwrap());
    }

    #[test]
    fn unspecified_v4_without_anything_lists_every_missing_item() {
        let missing = missing_of(evaluate_bind_policy(input("0.0.0.0")));
        assert_eq!(
            missing,
            vec![
                BindRequirement::WriteToken,
                BindRequirement::ReadToken,
                BindRequirement::TlsOrTunnel
            ]
        );
    }

    #[test]
    fn unspecified_v6_is_not_loopback() {
        let missing = missing_of(evaluate_bind_policy(input("::")));
        assert_eq!(missing.len(), 3);
    }

    #[test]
    fn refusal_message_names_every_missing_setting() {
        let msg = evaluate_bind_policy(input("0.0.0.0"))
            .unwrap_err()
            .to_string();
        for needle in [
            "0.0.0.0:3921",
            "VOX_MCP_HTTP_BEARER_TOKEN",
            "VOX_MCP_HTTP_READ_BEARER_TOKEN",
            "VOX_MCP_HTTP_TLS_CERT",
            "VOX_MCP_HTTP_BEHIND_TUNNEL",
        ] {
            assert!(msg.contains(needle), "missing {needle:?} in: {msg}");
        }
    }

    #[test]
    fn lan_with_both_tokens_and_tls_is_allowed() {
        let mut i = input("192.168.1.5");
        i.has_write_token = true;
        i.has_read_token = true;
        i.tls = tls();
        let p = evaluate_bind_policy(i).unwrap();
        assert_eq!(p.exposure, BindExposure::Network);
        assert!(matches!(p.transport, GatewayTransport::Tls { .. }));
    }

    #[test]
    fn lan_with_both_tokens_behind_tunnel_is_allowed_without_tls() {
        let mut i = input("100.101.102.103");
        i.has_write_token = true;
        i.has_read_token = true;
        i.behind_tunnel = true;
        let p = evaluate_bind_policy(i).unwrap();
        assert_eq!(p.exposure, BindExposure::Network);
        assert_eq!(p.transport, GatewayTransport::PlainHttp);
        assert!(p.behind_tunnel);
    }

    #[test]
    fn lan_with_only_write_token_needs_read_token() {
        let mut i = input("10.0.0.2");
        i.has_write_token = true;
        i.tls = tls();
        assert_eq!(
            missing_of(evaluate_bind_policy(i)),
            vec![BindRequirement::ReadToken]
        );
    }

    #[test]
    fn allow_unauthenticated_is_rejected_off_loopback() {
        let mut i = input("10.0.0.2");
        i.has_write_token = true;
        i.has_read_token = true;
        i.tls = tls();
        i.allow_unauthenticated = true;
        assert_eq!(
            missing_of(evaluate_bind_policy(i)),
            vec![BindRequirement::AllowUnauthenticatedUnset]
        );
    }

    #[test]
    fn allow_unauthenticated_still_works_on_loopback() {
        let mut i = input("127.0.0.1");
        i.allow_unauthenticated = true;
        assert!(evaluate_bind_policy(i).is_ok());
    }

    #[test]
    fn loopback_behind_tunnel_is_network_exposed() {
        let mut i = input("127.0.0.1");
        i.behind_tunnel = true;
        assert_eq!(
            missing_of(evaluate_bind_policy(i)),
            vec![BindRequirement::WriteToken, BindRequirement::ReadToken]
        );
    }

    #[test]
    fn loopback_behind_tunnel_with_tokens_reports_network_exposure() {
        let mut i = input("127.0.0.1");
        i.behind_tunnel = true;
        i.has_write_token = true;
        i.has_read_token = true;
        let p = evaluate_bind_policy(i).unwrap();
        assert_eq!(p.exposure, BindExposure::Network);
    }

    #[test]
    fn invalid_tls_material_is_an_error_even_on_loopback() {
        let mut i = input("127.0.0.1");
        i.tls = TlsMaterial::Invalid(
            "VOX_MCP_HTTP_TLS_CERT is set but VOX_MCP_HTTP_TLS_KEY is not".into(),
        );
        assert!(matches!(
            evaluate_bind_policy(i),
            Err(BindPolicyError::Tls(_))
        ));
    }

    #[test]
    fn hostname_is_rejected() {
        assert!(matches!(
            evaluate_bind_policy(input("vox.example.com")),
            Err(BindPolicyError::InvalidHost { .. })
        ));
    }

    #[test]
    fn parse_bind_ip_accepts_literals_brackets_and_localhost() {
        assert_eq!(
            parse_bind_ip(" 10.1.2.3 "),
            Some("10.1.2.3".parse().unwrap())
        );
        assert_eq!(parse_bind_ip("[fe80::1]"), Some("fe80::1".parse().unwrap()));
        assert_eq!(
            parse_bind_ip("LOCALHOST"),
            Some("127.0.0.1".parse().unwrap())
        );
        assert_eq!(parse_bind_ip("nope"), None);
    }

    #[test]
    fn is_loopback_ip_canonicalizes_v4_mapped() {
        assert!(is_loopback_ip("::ffff:127.0.0.1".parse().unwrap()));
        assert!(!is_loopback_ip("::ffff:10.0.0.1".parse().unwrap()));
        assert!(!is_loopback_ip("0.0.0.0".parse().unwrap()));
    }

    #[test]
    fn summary_names_address_scope_and_transport() {
        let p = evaluate_bind_policy(input("127.0.0.1")).unwrap();
        assert_eq!(p.summary(), "127.0.0.1:3921 — loopback only, plain HTTP");
        let mut i = input("10.0.0.2");
        i.has_write_token = true;
        i.has_read_token = true;
        i.behind_tunnel = true;
        let p = evaluate_bind_policy(i).unwrap();
        assert_eq!(
            p.summary(),
            "10.0.0.2:3921 — network-reachable, plain HTTP behind an attested TLS tunnel"
        );
    }

    #[test]
    fn every_requirement_has_a_description_naming_its_setting() {
        assert!(
            BindRequirement::WriteToken
                .describe()
                .contains("VOX_MCP_HTTP_BEARER_TOKEN")
        );
        assert!(
            BindRequirement::ReadToken
                .describe()
                .contains("VOX_MCP_HTTP_READ_BEARER_TOKEN")
        );
        assert!(
            BindRequirement::TlsOrTunnel
                .describe()
                .contains("--behind-tunnel")
        );
        assert!(
            BindRequirement::AllowUnauthenticatedUnset
                .describe()
                .contains("VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED")
        );
    }
}
