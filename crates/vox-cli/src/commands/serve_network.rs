//! `vox serve` network flags → MCP HTTP gateway env for the spawned daemon
//! (capability-profiles plan 04). The gateway's bind policy validates the result at startup.

use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(clap::Args, Debug, Clone, Default)]
pub struct ServeNetworkArgs {
    /// Gateway bind address as an IP literal `host:port` (default `127.0.0.1:3921`).
    /// A non-loopback address requires both bearer tokens and TLS or `--behind-tunnel`.
    #[arg(long, value_name = "HOST:PORT")]
    pub bind: Option<SocketAddr>,
    /// PEM certificate for TLS termination (requires `--tls-key`).
    #[arg(
        long,
        value_name = "PEM",
        requires = "tls_key",
        conflicts_with = "behind_tunnel"
    )]
    pub tls_cert: Option<PathBuf>,
    /// PEM private key for TLS termination (requires `--tls-cert`).
    #[arg(
        long,
        value_name = "PEM",
        requires = "tls_cert",
        conflicts_with = "behind_tunnel"
    )]
    pub tls_key: Option<PathBuf>,
    /// Attest that a TLS-terminating tunnel/proxy (Tailscale, Cloudflare Tunnel, Traefik)
    /// fronts the gateway. All callers are then treated as non-loopback.
    #[arg(long)]
    pub behind_tunnel: bool,
}

impl ServeNetworkArgs {
    pub fn gateway_env(&self) -> std::io::Result<Vec<(&'static str, String)>> {
        let mut env = Vec::new();
        if let Some(addr) = self.bind {
            env.push(("VOX_MCP_HTTP_HOST", addr.ip().to_string()));
            env.push(("VOX_MCP_HTTP_PORT", addr.port().to_string()));
        }
        if let Some(cert) = &self.tls_cert {
            env.push((
                "VOX_MCP_HTTP_TLS_CERT",
                std::path::absolute(cert)?.display().to_string(),
            ));
        }
        if let Some(key) = &self.tls_key {
            env.push((
                "VOX_MCP_HTTP_TLS_KEY",
                std::path::absolute(key)?.display().to_string(),
            ));
        }
        if self.behind_tunnel {
            env.push(("VOX_MCP_HTTP_BEHIND_TUNNEL", "1".to_string()));
        }
        Ok(env)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser, Debug)]
    struct T {
        #[command(flatten)]
        n: ServeNetworkArgs,
    }

    #[test]
    fn no_flags_sets_no_env() {
        assert!(
            ServeNetworkArgs::default()
                .gateway_env()
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn bind_maps_to_host_and_port() {
        let t = T::try_parse_from(["t", "--bind", "0.0.0.0:4000"]).unwrap();
        let env = t.n.gateway_env().unwrap();
        assert!(env.contains(&("VOX_MCP_HTTP_HOST", "0.0.0.0".to_string())));
        assert!(env.contains(&("VOX_MCP_HTTP_PORT", "4000".to_string())));
    }

    #[test]
    fn ipv6_bind_host_has_no_brackets() {
        let t = T::try_parse_from(["t", "--bind", "[::1]:3921"]).unwrap();
        assert!(
            t.n.gateway_env()
                .unwrap()
                .contains(&("VOX_MCP_HTTP_HOST", "::1".to_string()))
        );
    }

    #[test]
    fn tls_paths_become_absolute() {
        let t = T::try_parse_from(["t", "--tls-cert", "c.pem", "--tls-key", "k.pem"]).unwrap();
        let env = t.n.gateway_env().unwrap();
        let cert = env
            .iter()
            .find(|(k, _)| *k == "VOX_MCP_HTTP_TLS_CERT")
            .unwrap();
        assert!(std::path::Path::new(&cert.1).is_absolute(), "{}", cert.1);
        assert!(env.iter().any(|(k, _)| *k == "VOX_MCP_HTTP_TLS_KEY"));
    }

    #[test]
    fn behind_tunnel_sets_flag() {
        let t = T::try_parse_from(["t", "--behind-tunnel"]).unwrap();
        assert_eq!(
            t.n.gateway_env().unwrap(),
            vec![("VOX_MCP_HTTP_BEHIND_TUNNEL", "1".to_string())]
        );
    }

    #[test]
    fn tls_and_tunnel_are_mutually_exclusive() {
        assert!(
            T::try_parse_from(["t", "--tls-cert", "c", "--tls-key", "k", "--behind-tunnel"])
                .is_err()
        );
    }

    #[test]
    fn cert_requires_key_and_key_requires_cert() {
        assert!(T::try_parse_from(["t", "--tls-cert", "c"]).is_err());
        assert!(T::try_parse_from(["t", "--tls-key", "k"]).is_err());
    }
}
