---
title: "Capability Profiles plan 04 — network bind policy, TLS/tunnel, per-caller exposure, GET /v1/profile"
description: "Implementation plan for the MCP HTTP gateway's bind policy (loopback default, tokens + TLS-or-tunnel off loopback), rustls termination on the existing ring provider, peer-derived CallContext for every tool path, the GET /v1/profile discovery endpoint, the vox doctor bind check, vox serve network flags, and the 'Running Vox as a server' how-to."
category: "architecture"
status: "proposed"
date: 2026-10-08
---

# Network Policy and Profile Discovery Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The MCP HTTP gateway binds to loopback by default, refuses any network-reachable bind
that lacks a write token, a read token, and TLS-or-tunnel, derives each caller's `CallContext`
from the TCP peer address and the matched token, never serves `internal` tools to non-loopback
callers, and advertises what it serves through `GET /v1/profile`.

**Architecture:** A pure `evaluate_bind_policy` (new `http_gateway/bind_policy.rs`) decides the
bind before anything listens; `spawn_http_gateway` refuses on `Err` and the daemon exits non-zero
on that error type. TLS termination (new `http_gateway/tls.rs`) wraps the tokio `TcpListener` in
an `axum::serve::Listener` that performs rustls handshakes with the explicitly selected `ring`
provider. Plan 02 left every gateway call to `crate::profiles::guard_tool` with
`CallContext::LOCAL_WRITE`; this plan replaces each with a context built by one helper,
`call_context(state, peer, role)`, where `loopback = peer.ip().to_canonical().is_loopback() && !behind_tunnel`.
`/v1/profile` reuses the same visibility function as `/v1/tools`, so the two cannot disagree.

**Tech Stack:** Rust 2024, axum 0.8.9 (`serve::Listener`, `ListenerExt::tap_io`), rustls 0.23.45 +
tokio-rustls 0.26.4 (workspace-pinned to `ring` 0.17.14), rustls-pki-types 1.14.1 (`PemObject`),
rcgen 0.14.8 (dev only), reqwest 0.12 (tests), clap 4, `vox_secrets::resolve_secret`.

## Global Constraints

Copied verbatim from [`00-contracts.md`](00-contracts.md) §Global constraints:

- TDD: failing test first; every new `pub fn` has a test in the same file (`skeleton/untested-pub-api`).
- Security guards are mutation-verified: break the guard, see the test fail, restore, with a
  before/after presence check (AGENTS.md §PR & Review Discipline).
- No new workspace crate edges in plans 01–04, 06–07. Plan 05's new edges are **proposed for user
  authorization** and are not added to `contracts/ci/crate-edges.allow.v1.json` by an agent.
- Secrets via `vox_secrets::resolve_secret`; no `std::env::var` for tokens. Non-secret `VOX_*`
  config via existing `vox-config` patterns.
- Never `cargo fmt --all`; format dirty files with `vox run scripts/fmt.vox`.
- Generated files (`tool-registry.canonical.yaml`, `*.generated.md`) are regenerated, never hand-edited.
- New `.md` under `docs/src/` has frontmatter; lint with `cargo run -p vox-doc-pipeline -- --lint-only --paths <path relative to docs/src>`.
- Per-crate checks before each commit: `cargo check -p <crate>`, `cargo clippy -p <crate> --all-targets -- -D warnings`, targeted `cargo nextest run -p <crate> <filter>`.
- `dev` profile must reproduce today's behaviour exactly (back-compat tests in plans 02 and 03).

Plan-04-specific constraints:

- **The transport-crypto ledger is USER-AUTHORIZED-ONLY.** This plan adds no TLS provider and
  bumps no provider version. `rustls`/`tokio-rustls` are already pinned in the root `Cargo.toml`
  (`default-features = false`, `features = ["ring"]`) and resolved in `Cargo.lock`. Task 3 has a
  hard stop: if `Cargo.lock` gains any `[[package]]` or any `version =` line changes, stop and ask
  the user; never edit `contracts/crypto/transport-providers.v1.json`.
- **Never call `rustls::ServerConfig::builder()`.** Unified workspace builds enable both `ring` and
  `aws_lc_rs` on rustls (reqwest 0.13 via chromiumoxide / gix), and `builder()` panics when it
  cannot choose. Always `builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))`.
- **`loopback` comes from the socket peer (`ConnectInfo<SocketAddr>`), never from `Host`,
  `Origin`, `X-Forwarded-For`, or any other header.**
- Every other `VOX_MCP_HTTP_*` setting is resolved through `vox_secrets::SecretId` in
  `spawn_http_gateway_if_enabled`; the three new ones follow that surrounding pattern so one code
  path reads all gateway settings. Recorded as a deviation in the Self-Review.

---

## File Structure

| File | Action | Responsibility |
|---|---|---|
| `crates/vox-orchestrator-mcp/src/http_gateway/bind_policy.rs` | Create | Pure bind-policy types + `evaluate_bind_policy`, `parse_bind_ip`, `is_loopback_ip`. |
| `crates/vox-orchestrator-mcp/src/http_gateway/tls.rs` | Create | `tls_material_from_paths`, `load_tls_acceptor` (ring provider), `TlsListener` (axum `Listener`). |
| `crates/vox-orchestrator-mcp/src/http_gateway/profile_info.rs` | Create | `GET /v1/profile` handler + `ProfileInfo` wire struct. |
| `crates/vox-orchestrator-mcp/src/http_gateway/network_policy_tests.rs` | Create | Gateway-level tests: startup refusal, TLS e2e, CallContext, WS, `/api/v2`, `/v1/profile`. |
| `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs` | Modify | `behind_tunnel` state, `resolve_bind_policy_input`, `spawn_http_gateway`, `serve_gateway`, `caller_is_loopback`, `call_context`, `visible_tools_for`, `/v1/profile` route, test builders. |
| `crates/vox-orchestrator-mcp/src/http_gateway/rpc_tools.rs` | Modify | `CallContext` on `/v1/tools`, `/v1/tools/call`; `gateway_denial`, `gated_tool_json`. |
| `crates/vox-orchestrator-mcp/src/http_gateway/ws.rs` | Modify | Per-message `CallContext`; dashboard-token loopback via `caller_is_loopback`. |
| `crates/vox-orchestrator-mcp/src/http_gateway/status.rs` | Modify | `/v1/info` read list and `/v1/mobile/status` use the caller's context. |
| `crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs` | Modify | Route classification doc; `/mesh/nodes/{id}/kill` loopback-only. |
| `crates/vox-orchestrator-mcp/Cargo.toml` | Modify | `rustls`, `tokio-rustls` (workspace pins); dev `rcgen`. |
| Daemon gateway call site (`crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs` today; `daemon_steps.rs` after plan 03) | Modify | Exit non-zero on `BindPolicyError`. |
| `crates/vox-secrets/src/spec/ids.rs`, `crates/vox-secrets/src/spec/registry/missing.rs` | Modify | `VoxMcpHttpTlsCert`, `VoxMcpHttpTlsKey`, `VoxMcpHttpBehindTunnel`. |
| `crates/vox-secrets/tests/mcp_http_network_specs.rs` | Create | Spec presence test. |
| `contracts/secrets/managed-env-names.v1.json`, `contracts/secrets/secret-capabilities.v1.json`, `contracts/secrets/managed-env-names.md` | Regenerate | `vox ci secrets-contracts`. |
| `contracts/config/env-vars.v1.yaml`, `docs/src/reference/env-vars.md` | Modify | Three new vars; real description for `VOX_MCP_HTTP_HOST`. |
| `contracts/mcp/http-gateway.openapi.yaml` | Modify | `/v1/profile`; bind-policy note; denial statuses. |
| `crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/gateway_bind.rs` | Create | `vox doctor` "MCP HTTP gateway bind policy" check. |
| `crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/mod.rs` | Modify | Register the check. |
| `crates/vox-cli/src/commands/serve_network.rs` | Create | `ServeNetworkArgs` (`--bind`, `--tls-cert`, `--tls-key`, `--behind-tunnel`) → env. |
| `crates/vox-cli/src/commands/mod.rs`, `crates/vox-cli/src/lib.rs`, `crates/vox-cli/src/cli_dispatch/mod.rs`, `crates/vox-cli/src/commands/serve.rs` (plan 02) | Modify | Flatten `ServeNetworkArgs` into `Cli::Serve`; pass env to the spawned daemon. |
| `vox-eval.compose.yml`, `docker/vox-eval.compose.yml`, `docs/src/architecture/eval-sandbox-deployment.md` | Modify | Read token + `VOX_MCP_HTTP_BEHIND_TUNNEL=1` (D2: operator adds the Coolify read-token secret; no exemption). |
| `docs/src/how-to/how-to-run-vox-as-a-server.md` | Create | Operator how-to. |
| `docs/src/reference/mcp-http-gateway-contract.md` | Modify | Bind policy + `/v1/profile` section. |

---

### Task 0: Verify plans 01–03 landed and pin the names this plan builds on

**Files:** none modified.

**Interfaces consumed** (from `00-contracts.md`, `02-profiles.md`, `03-subsystem-boot.md`):

- `vox_orchestrator_mcp::profiles::{ActiveProfile, BootReport, CallContext, CallerRole, ProfileDenial, Exposure, Subsystem, authorize_tool, registry_entry}` (re-exports).
- `crate::profiles::guard_tool(state: &ServerState, ctx: CallContext, canonical: &str) -> Result<(), ProfileDenial>` (plan 02; non-registry names pass only under `dev`).
- `crate::profiles::denial_error_json(tool: &str, denial: &ProfileDenial, profile: &ActiveProfile) -> serde_json::Value` (plan 02; contract wire shape).
- `ServerState { pub active_profile: Arc<ActiveProfile>, pub boot_report: Arc<parking_lot::RwLock<BootReport>> }` (plan 02) and `ServerState::boot_report_snapshot(&self) -> BootReport` (plan 03).
- `crate::commands::serve::run(profile_flag: Option<&str>)`, `serve_child_env(profile_id, socket)`, `Cli::Serve { profile: Option<String> }` (plan 02).

- [ ] **Step 1: Confirm the types and helpers exist**

```bash
cd /Users/brbrainerd/dev/vox/.worktrees/capability-profiles
rg -n "pub fn authorize_tool|pub struct CallContext|pub enum CallerRole|pub enum ProfileDenial" crates/vox-mcp-registry/src/
rg -n "pub fn guard_tool|pub fn denial_error_json" crates/vox-orchestrator-mcp/src/profiles.rs
rg -n "pub active_profile|pub boot_report|fn boot_report_snapshot" crates/vox-orchestrator-mcp/src/server_state.rs crates/vox-orchestrator-mcp/src/server_state/
rg -n "pub async fn run|fn serve_child_env" crates/vox-cli/src/commands/serve.rs
```

Expected: one hit for each name. If any is missing, STOP: the plan that owns it (02 or 03) has not landed. These names are fixed by `00-contracts.md` and plans 02/03; do not substitute alternatives.

- [ ] **Step 2: List plan 02's hard-coded loopback contexts in the gateway**

```bash
rg -n "LOCAL_WRITE|loopback: true" crates/vox-orchestrator-mcp/src/http_gateway/
```

Expected (from plan 02 Task "HTTP/WS filtering"): non-test hits in `mod.rs`
`visible_tools_for_role` (a `retain(... guard_tool(..., LOCAL_WRITE, ...))`), and in
`rpc_tools.rs` `http_call_tool` and `call_tool_response_value`. Task 5 removes every non-test hit.

- [ ] **Step 3: Baseline green**

```bash
cargo nextest run -p vox-orchestrator-mcp http_gateway
```

Expected: all pass. No commit for this task.

---

### Task 1: Pure bind policy

**Parallel-safe with:** Task 2, Task 3 (Task 3 also adds one `mod` line to `http_gateway/mod.rs`; an append-only module list, so union-merge is safe).

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/http_gateway/bind_policy.rs`
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs` (module declaration only)

**Interfaces:**

```rust
pub enum TlsMaterial { Absent, Ready { cert: PathBuf, key: PathBuf }, Invalid(String) }
pub struct BindPolicyInput { pub host: String, pub port: u16, pub has_write_token: bool, pub has_read_token: bool, pub tls: TlsMaterial, pub behind_tunnel: bool, pub allow_unauthenticated: bool }
pub enum BindExposure { Loopback, Network }
pub enum GatewayTransport { PlainHttp, Tls { cert: PathBuf, key: PathBuf } }
pub struct BindPolicy { pub addr: SocketAddr, pub exposure: BindExposure, pub transport: GatewayTransport, pub behind_tunnel: bool }
pub enum BindRequirement { WriteToken, ReadToken, TlsOrTunnel, AllowUnauthenticatedUnset }
pub enum BindPolicyError { InvalidHost { host }, Tls(String), NetworkBindRequirements { addr, reason, missing } }
pub fn parse_bind_ip(host: &str) -> Option<IpAddr>;
pub fn is_loopback_ip(ip: IpAddr) -> bool;
pub fn evaluate_bind_policy(input: BindPolicyInput) -> Result<BindPolicy, BindPolicyError>;
impl BindPolicy { pub fn summary(&self) -> String; }
impl BindRequirement { pub const fn describe(self) -> &'static str; }
```

Rules (spec §6): a loopback bind needs nothing new (today's behaviour). A bind is
**network-exposed** when the IP is not loopback **or** `behind_tunnel` is set: a same-host tunnel
such as `cloudflared` or `tailscale serve` forwards remote callers to 127.0.0.1, so a loopback bind
behind a tunnel is reachable from the network. A network-exposed bind requires a write token, a
read token, TLS-or-tunnel, and `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` unset, and the error lists
every missing item. Invalid TLS material is an error on any bind, because the operator asked for TLS.

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-orchestrator-mcp/src/http_gateway/bind_policy.rs` containing only this test
module (no stub implementations); the implementation is added above it in Step 3:

```rust
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
        let msg = evaluate_bind_policy(input("0.0.0.0")).unwrap_err().to_string();
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
        assert_eq!(missing_of(evaluate_bind_policy(i)), vec![BindRequirement::ReadToken]);
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
        assert!(matches!(evaluate_bind_policy(i), Err(BindPolicyError::Tls(_))));
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
        assert_eq!(parse_bind_ip(" 10.1.2.3 "), Some("10.1.2.3".parse().unwrap()));
        assert_eq!(parse_bind_ip("[fe80::1]"), Some("fe80::1".parse().unwrap()));
        assert_eq!(parse_bind_ip("LOCALHOST"), Some("127.0.0.1".parse().unwrap()));
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
        assert!(BindRequirement::WriteToken.describe().contains("VOX_MCP_HTTP_BEARER_TOKEN"));
        assert!(BindRequirement::ReadToken.describe().contains("VOX_MCP_HTTP_READ_BEARER_TOKEN"));
        assert!(BindRequirement::TlsOrTunnel.describe().contains("--behind-tunnel"));
        assert!(
            BindRequirement::AllowUnauthenticatedUnset
                .describe()
                .contains("VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED")
        );
    }
}
```

In `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs`, next to `mod token;`, add:

```rust
pub mod bind_policy;
```

- [ ] **Step 2: Run the tests to see them fail**

```bash
cargo nextest run -p vox-orchestrator-mcp bind_policy
```

Expected: compile error `cannot find type BindPolicyInput in this scope` (and the other items).

- [ ] **Step 3: Implement**

Put this above the test module in `bind_policy.rs`:

```rust
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
```

- [ ] **Step 4: Run the tests to see them pass**

```bash
cargo nextest run -p vox-orchestrator-mcp bind_policy
```

Expected: `20 tests run: 20 passed`.

- [ ] **Step 5: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
git add crates/vox-orchestrator-mcp/src/http_gateway/bind_policy.rs crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
git commit -m "feat(mcp-http): pure bind policy for loopback/network gateway binds"
```

---

### Task 2: Register the new settings (Clavis spec + env contract)

**Parallel-safe with:** Task 1, Task 3.

**Files:**
- Create: `crates/vox-secrets/tests/mcp_http_network_specs.rs`
- Modify: `crates/vox-secrets/src/spec/ids.rs`, `crates/vox-secrets/src/spec/registry/missing.rs`
- Modify: `contracts/config/env-vars.v1.yaml`, `docs/src/reference/env-vars.md`
- Regenerate: `contracts/secrets/managed-env-names.v1.json`, `contracts/secrets/secret-capabilities.v1.json`, `contracts/secrets/managed-env-names.md`

**Interfaces:** `SecretId::VoxMcpHttpTlsCert`, `SecretId::VoxMcpHttpTlsKey`, `SecretId::VoxMcpHttpBehindTunnel` (canonical envs `VOX_MCP_HTTP_TLS_CERT`, `VOX_MCP_HTTP_TLS_KEY`, `VOX_MCP_HTTP_BEHIND_TUNNEL`).

- [ ] **Step 1: Write the failing test**

`crates/vox-secrets/tests/mcp_http_network_specs.rs`:

```rust
//! Capability-profiles plan 04: the gateway's network settings are resolvable through Clavis.

#[test]
fn mcp_http_network_settings_are_registered() {
    for env in [
        "VOX_MCP_HTTP_TLS_CERT",
        "VOX_MCP_HTTP_TLS_KEY",
        "VOX_MCP_HTTP_BEHIND_TUNNEL",
    ] {
        assert!(
            vox_secrets::all_specs().iter().any(|s| s.canonical_env == env),
            "{env} has no SecretSpec"
        );
    }
}
```

- [ ] **Step 2: Run it to see it fail**

```bash
cargo nextest run -p vox-secrets mcp_http_network_settings_are_registered
```

Expected: FAIL with `VOX_MCP_HTTP_TLS_CERT has no SecretSpec`.

- [ ] **Step 3: Add the ids and specs**

In `crates/vox-secrets/src/spec/ids.rs`, directly after `VoxMcpHttpPublicEvalEnabled,`:

```rust
    VoxMcpHttpTlsCert,
    VoxMcpHttpTlsKey,
    VoxMcpHttpBehindTunnel,
```

In `crates/vox-secrets/src/spec/registry/missing.rs`, directly after the
`SecretId::VoxMcpHttpPublicEvalEnabled` `SecretSpec { ... },` entry:

```rust
    SecretSpec {
        id: SecretId::VoxMcpHttpTlsCert,
        canonical_env: "VOX_MCP_HTTP_TLS_CERT",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "PEM certificate path for TLS on the MCP HTTP gateway; set together with VOX_MCP_HTTP_TLS_KEY (or `vox serve --tls-cert`).",
        scope_description: "",
    },
    SecretSpec {
        id: SecretId::VoxMcpHttpTlsKey,
        canonical_env: "VOX_MCP_HTTP_TLS_KEY",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "PEM private-key path for TLS on the MCP HTTP gateway; set together with VOX_MCP_HTTP_TLS_CERT (or `vox serve --tls-key`).",
        scope_description: "",
    },
    SecretSpec {
        id: SecretId::VoxMcpHttpBehindTunnel,
        canonical_env: "VOX_MCP_HTTP_BEHIND_TUNNEL",
        aliases: &[],
        deprecated_aliases: &[],
        backend_key: None,
        auth_registry: None,
        policy: SecretPolicy::optional_skip(),
        remediation: "Set to 1 when a TLS-terminating tunnel or reverse proxy (Tailscale, Cloudflare Tunnel, Traefik) fronts the gateway; every caller is then treated as non-loopback.",
        scope_description: "",
    },
```

- [ ] **Step 4: Run the test to see it pass**

```bash
cargo nextest run -p vox-secrets mcp_http_network_settings_are_registered
```

Expected: `1 test run: 1 passed`.

- [ ] **Step 5: Env-var contract and prose SSOT**

In `contracts/config/env-vars.v1.yaml`, replace the `VOX_MCP_HTTP_HOST` entry's
`description: "TODO"` with:

```yaml
    description: "Bind host for the MCP HTTP gateway (IP literal or localhost; default 127.0.0.1). A non-loopback host requires write + read tokens and TLS or VOX_MCP_HTTP_BEHIND_TUNNEL."
```

Insert directly after the `VOX_MCP_HTTP_PUBLIC_EVAL_ENABLED` entry:

```yaml
  - name: "VOX_MCP_HTTP_TLS_CERT"
    owner_crate: "vox-orchestrator-mcp"
    kind: "string"
    required: false
    introduced_in: "0.6.0"
    description: "PEM certificate path for TLS termination on the MCP HTTP gateway; requires VOX_MCP_HTTP_TLS_KEY. Satisfies the network bind policy's TLS-or-tunnel requirement."
  - name: "VOX_MCP_HTTP_TLS_KEY"
    owner_crate: "vox-orchestrator-mcp"
    kind: "string"
    required: false
    introduced_in: "0.6.0"
    description: "PEM private-key path for TLS termination on the MCP HTTP gateway; requires VOX_MCP_HTTP_TLS_CERT."
  - name: "VOX_MCP_HTTP_BEHIND_TUNNEL"
    owner_crate: "vox-orchestrator-mcp"
    kind: "bool"
    required: false
    introduced_in: "0.6.0"
    description: "Set to 1 when a TLS-terminating tunnel/proxy fronts the MCP HTTP gateway. Satisfies TLS-or-tunnel, forces token auth even on a loopback bind, and treats every caller as non-loopback."
```

In `docs/src/reference/env-vars.md`, directly after the `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR` row:

```markdown
| `VOX_MCP_HTTP_TLS_CERT` / `VOX_MCP_HTTP_TLS_KEY` | PEM certificate and private-key paths; when both are set the MCP HTTP gateway terminates TLS itself (rustls, `ring` provider). Same as `vox serve --tls-cert/--tls-key`. |
| `VOX_MCP_HTTP_BEHIND_TUNNEL` | `1` = the operator attests a TLS-terminating tunnel or proxy (Tailscale, Cloudflare Tunnel, Traefik) fronts the gateway. Satisfies the TLS-or-tunnel bind requirement, requires both tokens even on a loopback bind, and treats every caller as non-loopback (no `internal` tools). Same as `vox serve --behind-tunnel`. |
```

Replace the `VOX_MCP_HTTP_HOST` / `VOX_MCP_HTTP_PORT` row with:

```markdown
| `VOX_MCP_HTTP_HOST` / `VOX_MCP_HTTP_PORT` | Bind address for the optional MCP HTTP gateway (defaults: `127.0.0.1:3921`). A non-loopback host (anything outside `127.0.0.0/8` and `::1`, including `0.0.0.0` and `::`) is refused unless both bearer tokens are set and TLS or `VOX_MCP_HTTP_BEHIND_TUNNEL=1` is configured; `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` is rejected there. See [Running Vox as a server](../how-to/how-to-run-vox-as-a-server.md). |
```

- [ ] **Step 6: Regenerate and run the contract gates**

```bash
cargo run -q -p vox-cli -- ci secrets-contracts
cargo run -q -p vox-cli -- ci secrets-parity
cargo run -q -p vox-cli -- ci secret-env-guard
cargo run -q -p vox-cli -- ci command-compliance
cargo run -q -p vox-cli -- ci data-storage-guard
git status --short contracts/secrets
```

Expected: every command exits 0; `git status` lists the three regenerated `contracts/secrets/*`
files. The link to `how-to-run-vox-as-a-server.md` is dangling until Task 11; links are checked in
Task 11 Step 3.

- [ ] **Step 7: Commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-secrets --all-targets -- -D warnings
git add crates/vox-secrets contracts/secrets contracts/config/env-vars.v1.yaml docs/src/reference/env-vars.md
git commit -m "feat(secrets): register MCP HTTP TLS cert/key and behind-tunnel settings"
```

---

### Task 3: TLS termination on the existing rustls/ring stack

**Parallel-safe with:** Task 1, Task 2.

**TLS dependency finding (measured 2026-10-08 in this worktree):** `Cargo.lock` already contains
`rustls 0.23.45`, `tokio-rustls 0.26.4`, `rustls-pki-types 1.14.1`, `rustls-pemfile 2.2.0`,
`ring 0.17.14`, `rcgen 0.14.8`, `hyper-util 0.1.20`, `axum 0.8.9`; there is **no** `axum-server`.
The root manifest pins `rustls = { version = "0.23", default-features = false, features = ["ring"] }`
and `tokio-rustls = { version = "0.26", default-features = false, features = ["ring"] }`, and
`vox-populi` already terminates TLS with them (`crates/vox-populi/src/tls.rs`, test
`tests/tls_smoke.rs` with `rcgen 0.14` `ring`+`pem`). axum 0.8's `serve::Listener` trait lets a TLS
listener plug into `axum::serve` without `axum-server` or `hyper-util` server features. No new
package and no provider change is needed. `rustls-pemfile` is not used (unmaintained upstream);
`rustls_pki_types::pem::PemObject` replaces it.

Why not reuse `vox_populi::tls::build_acceptor`: it calls `ServerConfig::builder()`, which panics in
builds where rustls has both `ring` and `aws_lc_rs` enabled (true for any binary that also links
`vox-vcs`/chromiumoxide), and it sits behind populi's optional `tls` feature.

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/http_gateway/tls.rs`
- Modify: `crates/vox-orchestrator-mcp/Cargo.toml`, `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs` (module line), `Cargo.lock` (dependency list of `vox-orchestrator-mcp` only)

**Interfaces:**

```rust
pub fn tls_material_from_paths(cert: Option<PathBuf>, key: Option<PathBuf>) -> TlsMaterial;
pub(super) fn load_tls_acceptor(cert: &Path, key: &Path) -> anyhow::Result<TlsAcceptor>;
pub(super) struct TlsListener; // impl axum::serve::Listener<Io = TlsStream<TcpStream>, Addr = SocketAddr>
impl TlsListener { pub(super) fn spawn(tcp: TcpListener, acceptor: TlsAcceptor) -> std::io::Result<Self>; }
#[cfg(test)] pub(super) fn write_self_signed(dir: &Path) -> (PathBuf, PathBuf);
```

- [ ] **Step 1: Record the provider baseline**

```bash
cargo tree -p vox-orchestrator-mcp -e features -i rustls --target all 2>/dev/null | rg -c "aws_lc_rs|aws-lc-rs" > /tmp/vox-plan04-awslc.before || true
cat /tmp/vox-plan04-awslc.before
```

Expected: a number (possibly `0`). Record it.

- [ ] **Step 2: Add the dependencies**

In `crates/vox-orchestrator-mcp/Cargo.toml` `[dependencies]`, after `tokio = { workspace = true }`:

```toml
# MCP HTTP gateway TLS termination (capability-profiles plan 04). The workspace pins select
# the `ring` provider with default features off; never enable rustls/tokio-rustls defaults
# here (they select `aws_lc_rs`). Provider changes need a transport-providers ledger entry.
rustls = { workspace = true, features = ["std", "logging", "tls12"] }
tokio-rustls = { workspace = true }
```

In `[dev-dependencies]`, after `wiremock.workspace = true`:

```toml
# Self-signed certs for gateway TLS tests; same pin/features as vox-populi's tls_smoke test.
rcgen = { version = "0.14", default-features = false, features = ["ring", "pem"] }
```

In `http_gateway/mod.rs`, next to `pub mod bind_policy;`:

```rust
mod tls;
```

- [ ] **Step 3: Verify no new package or provider (HARD STOP)**

```bash
cargo check -p vox-orchestrator-mcp
git diff Cargo.lock | rg '^[+-](name|version) = '; echo "exit=$?"
cargo tree -p vox-orchestrator-mcp -e features -i rustls --target all 2>/dev/null | rg -c "aws_lc_rs|aws-lc-rs" || true
cat /tmp/vox-plan04-awslc.before
```

Expected: `cargo check` succeeds; the lockfile `rg` prints nothing and `exit=1` (only the
`dependencies = [...]` list of `vox-orchestrator-mcp` changes); the aws-lc count equals the baseline.

**If any `name =` or `version =` line appears, or the aws-lc count increases: STOP. Revert with
`git checkout -- Cargo.lock crates/vox-orchestrator-mcp/Cargo.toml` and ask the user for
authorization, naming the exact crate and version. Transport-provider ledger entries are
user-authorized-only. Do not continue the TLS parts of Tasks 3–4 until the user answers.
`--behind-tunnel` (Tasks 1, 2, 4–13) works without TLS.**

If `cargo hakari --version` succeeds, also run `cargo hakari generate --diff`. If it reports a
diff, run `cargo hakari generate` and include `crates/workspace-hack/Cargo.toml` in this commit.

- [ ] **Step 4: Write the failing tests**

Create `crates/vox-orchestrator-mcp/src/http_gateway/tls.rs` with this test module (the
implementation goes above it in Step 6):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_material_absent_when_unset() {
        assert_eq!(tls_material_from_paths(None, None), TlsMaterial::Absent);
    }

    #[test]
    fn tls_material_rejects_half_configuration() {
        let only_cert = tls_material_from_paths(Some("c.pem".into()), None);
        assert!(matches!(only_cert, TlsMaterial::Invalid(ref m) if m.contains("VOX_MCP_HTTP_TLS_KEY")));
        let only_key = tls_material_from_paths(None, Some("k.pem".into()));
        assert!(matches!(only_key, TlsMaterial::Invalid(ref m) if m.contains("VOX_MCP_HTTP_TLS_CERT")));
    }

    #[test]
    fn tls_material_rejects_unreadable_paths() {
        let dir = tempfile::tempdir().unwrap();
        let m = tls_material_from_paths(
            Some(dir.path().join("missing-cert.pem")),
            Some(dir.path().join("missing-key.pem")),
        );
        assert!(matches!(m, TlsMaterial::Invalid(ref msg) if msg.contains("missing-cert.pem")));
    }

    #[test]
    fn tls_material_ready_for_readable_files() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_self_signed(dir.path());
        assert_eq!(
            tls_material_from_paths(Some(cert.clone()), Some(key.clone())),
            TlsMaterial::Ready { cert, key }
        );
    }

    #[test]
    fn load_tls_acceptor_accepts_self_signed_pair() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_self_signed(dir.path());
        load_tls_acceptor(&cert, &key).expect("acceptor from rcgen pair");
    }

    #[test]
    fn load_tls_acceptor_rejects_non_pem() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, b"not a certificate").unwrap();
        std::fs::write(&key, b"not a key").unwrap();
        let err = format!("{:#}", load_tls_acceptor(&cert, &key).unwrap_err());
        assert!(err.contains("cert.pem"), "{err}");
    }
}
```

- [ ] **Step 5: Run them to see them fail**

```bash
cargo nextest run -p vox-orchestrator-mcp http_gateway::tls
```

Expected: compile errors `cannot find function tls_material_from_paths` / `load_tls_acceptor` / `write_self_signed`.

- [ ] **Step 6: Implement**

Above the test module in `tls.rs`:

```rust
//! TLS termination for the MCP HTTP gateway (capability-profiles plan 04).
//!
//! The provider is selected explicitly: unified workspace builds compile rustls with both
//! `ring` and `aws_lc_rs`, and `ServerConfig::builder()` panics when it cannot pick one.
//! `ring` is the workspace's pinned provider (contracts/crypto/transport-providers.v1.json).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::bind_policy::TlsMaterial;

/// A client that stalls mid-handshake is dropped after this long.
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// Completed handshakes waiting for axum to pick them up.
const PENDING_CONNECTIONS: usize = 64;

/// Resolve cert/key paths into [`TlsMaterial`]. Checks readability only; PEM parsing happens in
/// [`load_tls_acceptor`] at startup.
pub fn tls_material_from_paths(cert: Option<PathBuf>, key: Option<PathBuf>) -> TlsMaterial {
    match (cert, key) {
        (None, None) => TlsMaterial::Absent,
        (Some(_), None) => TlsMaterial::Invalid(
            "VOX_MCP_HTTP_TLS_CERT is set but VOX_MCP_HTTP_TLS_KEY is not".to_string(),
        ),
        (None, Some(_)) => TlsMaterial::Invalid(
            "VOX_MCP_HTTP_TLS_KEY is set but VOX_MCP_HTTP_TLS_CERT is not".to_string(),
        ),
        (Some(cert), Some(key)) => {
            for (label, path) in [("certificate", &cert), ("private key", &key)] {
                if let Err(e) = std::fs::File::open(path) {
                    return TlsMaterial::Invalid(format!(
                        "TLS {label} {} is not readable: {e}",
                        path.display()
                    ));
                }
            }
            TlsMaterial::Ready { cert, key }
        }
    }
}

pub(super) fn load_tls_acceptor(cert: &Path, key: &Path) -> anyhow::Result<TlsAcceptor> {
    let certs = CertificateDer::pem_file_iter(cert)
        .with_context(|| format!("read TLS certificate {}", cert.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("parse TLS certificate {}", cert.display()))?;
    anyhow::ensure!(
        !certs.is_empty(),
        "TLS certificate {} contains no PEM certificate",
        cert.display()
    );
    let private_key = PrivateKeyDer::from_pem_file(key)
        .with_context(|| format!("read TLS private key {}", key.display()))?;
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .context("rustls protocol versions")?
    .with_no_client_auth()
    .with_single_cert(certs, private_key)
    .with_context(|| {
        format!(
            "rustls rejected certificate {} / key {}",
            cert.display(),
            key.display()
        )
    })?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// `axum::serve::Listener` that yields only connections whose TLS handshake completed.
/// Handshakes run in their own tasks so one slow client cannot stall `accept`.
pub(super) struct TlsListener {
    rx: tokio::sync::mpsc::Receiver<(TlsStream<TcpStream>, SocketAddr)>,
    local_addr: SocketAddr,
}

impl TlsListener {
    pub(super) fn spawn(tcp: TcpListener, acceptor: TlsAcceptor) -> std::io::Result<Self> {
        let local_addr = tcp.local_addr()?;
        let (tx, rx) = tokio::sync::mpsc::channel(PENDING_CONNECTIONS);
        tokio::spawn(async move {
            loop {
                let (sock, peer) = match tcp.accept().await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, "mcp-http tcp accept failed");
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }
                };
                if tx.is_closed() {
                    break;
                }
                let acceptor = acceptor.clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    match tokio::time::timeout(TLS_HANDSHAKE_TIMEOUT, acceptor.accept(sock)).await {
                        Ok(Ok(stream)) => {
                            let _ = tx.send((stream, peer)).await;
                        }
                        Ok(Err(e)) => {
                            tracing::debug!(%peer, error = %e, "mcp-http tls handshake failed")
                        }
                        Err(_) => tracing::debug!(%peer, "mcp-http tls handshake timed out"),
                    }
                });
            }
        });
        Ok(Self { rx, local_addr })
    }
}

impl axum::serve::Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(conn) => conn,
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(self.local_addr)
    }
}

/// Write an rcgen self-signed `localhost` certificate + key into `dir`.
#[cfg(test)]
pub(super) fn write_self_signed(dir: &Path) -> (PathBuf, PathBuf) {
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    std::fs::write(&cert, ck.cert.pem()).unwrap();
    std::fs::write(&key, ck.signing_key.serialize_pem()).unwrap();
    (cert, key)
}
```

- [ ] **Step 7: Run the tests to see them pass**

```bash
cargo nextest run -p vox-orchestrator-mcp http_gateway::tls
```

Expected: `6 tests run: 6 passed`. `TlsListener` is exercised end-to-end in Task 4.

- [ ] **Step 8: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
git add Cargo.lock crates/vox-orchestrator-mcp/Cargo.toml crates/vox-orchestrator-mcp/src/http_gateway/tls.rs crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
git commit -m "feat(mcp-http): rustls termination on the pinned ring provider"
```

If clippy reports `TlsListener` as dead code (it has no caller until Task 4), do not commit here;
fold this task's changes into Task 4's commit.

---

### Task 4: Enforce the policy at gateway startup and serve TLS

Depends on Tasks 1–3.

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs`
- Create: `crates/vox-orchestrator-mcp/src/http_gateway/network_policy_tests.rs`
- Modify: the daemon's `spawn_http_gateway_if_enabled` call site

**Interfaces:**

```rust
pub fn resolve_bind_policy_input() -> bind_policy::BindPolicyInput;          // Clavis/env reads; shared with vox doctor
pub fn spawn_http_gateway_if_enabled(state: ServerState) -> Result<Option<JoinHandle<()>>>; // unchanged signature
pub(crate) fn spawn_http_gateway(state: ServerState, bind_input: BindPolicyInput) -> Result<JoinHandle<()>>;
pub(super) fn serve_gateway(listener: std::net::TcpListener, app: Router, transport: &GatewayTransport) -> Result<JoinHandle<()>>;
// GatewayState gains: behind_tunnel: bool
```

- [ ] **Step 1: Write the failing tests**

Create `crates/vox-orchestrator-mcp/src/http_gateway/network_policy_tests.rs`:

```rust
//! Capability-profiles plan 04: bind policy, TLS transport, and per-caller exposure.

use super::bind_policy::{BindPolicyError, BindPolicyInput, GatewayTransport, TlsMaterial};
use super::*;

fn network_input_without_credentials() -> BindPolicyInput {
    BindPolicyInput {
        host: "0.0.0.0".to_string(),
        port: 0,
        has_write_token: false,
        has_read_token: false,
        tls: TlsMaterial::Absent,
        behind_tunnel: false,
        allow_unauthenticated: false,
    }
}

#[tokio::test]
async fn spawn_refuses_network_bind_without_tokens() {
    let state = ServerState::new_test().await;
    let err = spawn_http_gateway(state, network_input_without_credentials())
        .expect_err("a network bind without tokens or TLS must refuse to start");
    let policy_err = err
        .downcast_ref::<BindPolicyError>()
        .expect("the refusal must come from the bind policy, not a later check");
    assert!(
        matches!(policy_err, BindPolicyError::NetworkBindRequirements { .. }),
        "{policy_err:?}"
    );
}

#[tokio::test]
async fn tls_transport_serves_health_over_https() {
    let dir = tempfile::tempdir().unwrap();
    let (cert, key) = tls::write_self_signed(dir.path());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = build_app(GatewayState::for_test().await);
    let _server = serve_gateway(listener, app, &GatewayTransport::Tls { cert, key }).unwrap();

    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap();
    let resp = client
        .get(format!("https://127.0.0.1:{port}/health"))
        .send()
        .await
        .expect("https request");
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
    assert_eq!(resp.text().await.unwrap(), "ok");

    let plain = reqwest::get(format!("http://127.0.0.1:{port}/health")).await;
    assert!(
        plain.map(|r| !r.status().is_success()).unwrap_or(true),
        "plain HTTP must not be served on the TLS port"
    );
}

#[tokio::test]
async fn plain_transport_still_serves_health() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = build_app(GatewayState::for_test().await);
    let _server = serve_gateway(listener, app, &GatewayTransport::PlainHttp).unwrap();
    let resp = reqwest::get(format!("http://127.0.0.1:{port}/health"))
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::OK);
}
```

`/health` extracts `ConnectInfo<SocketAddr>`; a 500 from the HTTPS request means the TLS path lost
connect info.

In `http_gateway/mod.rs`, after the existing `#[cfg(test)] mod tests { ... }` block:

```rust
#[cfg(test)]
mod network_policy_tests;
```

- [ ] **Step 2: Run them to see them fail**

```bash
cargo nextest run -p vox-orchestrator-mcp network_policy_tests
```

Expected: compile errors `cannot find function spawn_http_gateway` / `serve_gateway`.

- [ ] **Step 3: Add `behind_tunnel` to `GatewayState`**

In the `GatewayState` struct, after `trust_forwarded_for: bool,`:

```rust
    /// `VOX_MCP_HTTP_BEHIND_TUNNEL`: a same-host tunnel/proxy makes remote callers arrive from
    /// 127.0.0.1, so no peer is treated as loopback.
    behind_tunnel: bool,
```

In `for_test_with_state`, after `trust_forwarded_for: false,` add `behind_tunnel: false,`.

- [ ] **Step 4: Split startup into resolve → evaluate → bind → serve**

Add next to `read_bool_env`:

```rust
fn resolved_string(id: vox_secrets::SecretId) -> Option<String> {
    vox_secrets::resolve_secret(id)
        .expose()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// Gather bind-policy inputs from Clavis/env. Shared by gateway startup and `vox doctor`.
// toestub-ignore(skeleton/untested-pub-api) — thin Clavis reads; the policy it feeds is unit-tested in bind_policy.rs and startup refusal in network_policy_tests.rs
pub fn resolve_bind_policy_input() -> bind_policy::BindPolicyInput {
    use vox_secrets::SecretId;
    bind_policy::BindPolicyInput {
        host: resolved_string(SecretId::VoxMcpHttpHost)
            .unwrap_or_else(|| DEFAULT_BIND_HOST.to_string()),
        port: resolved_string(SecretId::VoxMcpHttpPort)
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(DEFAULT_BIND_PORT),
        has_write_token: resolved_string(SecretId::VoxMcpHttpBearerToken).is_some(),
        has_read_token: resolved_string(SecretId::VoxMcpHttpReadBearerToken).is_some(),
        tls: tls::tls_material_from_paths(
            resolved_string(SecretId::VoxMcpHttpTlsCert).map(std::path::PathBuf::from),
            resolved_string(SecretId::VoxMcpHttpTlsKey).map(std::path::PathBuf::from),
        ),
        behind_tunnel: read_bool_env(SecretId::VoxMcpHttpBehindTunnel).unwrap_or(false),
        allow_unauthenticated: read_bool_env(SecretId::VoxMcpHttpAllowUnauthenticated)
            .unwrap_or(false),
    }
}
```

Replace the head of `spawn_http_gateway_if_enabled` — from its doc comment and signature through
the `let bind_port = ... .unwrap_or(DEFAULT_BIND_PORT);` statement — with:

```rust
/// Start the optional HTTP+WebSocket gateway in a background task.
pub fn spawn_http_gateway_if_enabled(
    state: ServerState,
) -> Result<Option<tokio::task::JoinHandle<()>>> {
    if !http_gateway_enabled() {
        return Ok(None);
    }
    spawn_http_gateway(state, resolve_bind_policy_input()).map(Some)
}

/// Evaluate the bind policy first; nothing listens unless it passes.
pub(crate) fn spawn_http_gateway(
    state: ServerState,
    bind_input: bind_policy::BindPolicyInput,
) -> Result<tokio::task::JoinHandle<()>> {
    let policy = bind_policy::evaluate_bind_policy(bind_input)?;
    let bind_host = policy.addr.ip().to_string();
    let bind_port = policy.addr.port();
```

Keep everything from `#[allow(unused_mut)] let mut bearer_token =` through
`let app = build_app(gateway_state.clone());` unchanged (including plan 02's
`parse_allowed_tools(&state.active_profile)`), except add `behind_tunnel: policy.behind_tunnel,`
after `trust_forwarded_for,` in the `GatewayState { ... }` literal.

Replace the block from `let addr: SocketAddr = format!("{bind_host}:{bind_port}")` through
`let listener = tokio::net::TcpListener::from_std(listener) ... ?;` with:

```rust
    let listener = std::net::TcpListener::bind(policy.addr)
        .with_context(|| format!("failed to bind MCP HTTP gateway at {}", policy.addr))?;
```

(This also fixes IPv6 hosts: `format!("::1:3921")` never parsed as a `SocketAddr`.)

In the `tracing::info!(...)` call add the field `bind = %policy.summary(),` before
`"VOX_MCP_HTTP_ENABLED: gateway started"`.

Replace the trailing `let handle = tokio::spawn(async move { ... }); Ok(Some(handle))` with:

```rust
    serve_gateway(listener, app, &policy.transport)
}

/// Serve `app` on an already-bound listener, terminating TLS when the policy says so.
pub(super) fn serve_gateway(
    listener: std::net::TcpListener,
    app: Router,
    transport: &bind_policy::GatewayTransport,
) -> Result<tokio::task::JoinHandle<()>> {
    use axum::serve::ListenerExt;
    listener
        .set_nonblocking(true)
        .context("failed to set nonblocking gateway listener")?;
    let listener = tokio::net::TcpListener::from_std(listener)
        .context("failed to convert listener to tokio")?;
    let make_service = app.into_make_service_with_connect_info::<SocketAddr>();
    let handle = match transport {
        bind_policy::GatewayTransport::PlainHttp => tokio::spawn(async move {
            if let Err(e) = serve(listener, make_service).await {
                tracing::error!("MCP HTTP gateway exited with error: {e}");
            }
        }),
        bind_policy::GatewayTransport::Tls { cert, key } => {
            let acceptor = tls::load_tls_acceptor(cert, key)?;
            let tls_listener = tls::TlsListener::spawn(listener, acceptor)
                .context("failed to start TLS listener")?;
            // `tap_io` routes ConnectInfo<SocketAddr> through axum's generic Listener impl.
            let tls_listener = tls_listener
                .tap_io(|_io: &mut tokio_rustls::server::TlsStream<tokio::net::TcpStream>| {});
            tokio::spawn(async move {
                if let Err(e) = serve(tls_listener, make_service).await {
                    tracing::error!("MCP HTTP gateway (TLS) exited with error: {e}");
                }
            })
        }
    };
    Ok(handle)
}
```

The existing token-presence bail (`VOX_MCP_HTTP_ENABLED=1 requires VOX_MCP_HTTP_BEARER_TOKEN or ...`)
stays: it still governs loopback binds.

- [ ] **Step 5: Run the tests to see them pass**

```bash
cargo nextest run -p vox-orchestrator-mcp network_policy_tests http_gateway::tests
```

Expected: `network_policy_tests`: 3 passed; existing `http_gateway::tests` all pass.

- [ ] **Step 6: Daemon exits non-zero on a bind-policy refusal**

```bash
rg -n "spawn_http_gateway_if_enabled" crates/vox-orchestrator-d/src
```

Expected: one call site — `vox_orchestrator_d.rs` today, or `daemon_steps.rs` if plan 03 moved
the gateway into a boot step. Today's code is:

```rust
    if let Err(e) = vox_orchestrator_mcp::http_gateway::spawn_http_gateway_if_enabled(state) {
        tracing::error!(error = %e, "Failed to spawn HTTP gateway");
    }
```

Replace it (keeping whatever `state` expression plan 03 passes) with:

```rust
    if let Err(e) = vox_orchestrator_mcp::http_gateway::spawn_http_gateway_if_enabled(state) {
        if e
            .downcast_ref::<vox_orchestrator_mcp::http_gateway::bind_policy::BindPolicyError>()
            .is_some()
        {
            return Err(e.context("MCP HTTP gateway bind policy refused to start"));
        }
        tracing::error!(error = %e, "Failed to spawn HTTP gateway");
    }
```

If plan 03 made the gateway a boot step whose errors are recorded as degraded rather than
returned, the `BindPolicyError` branch must still end the process: return the error from the step
runner's caller (the function that decides the exit code). Step 7 verifies the exit status.
`vox mcp` in gateway-only mode already propagates the error through `lifecycle.rs`'s `?`.

```bash
cargo check -p vox-orchestrator-d
rg -c "BindPolicyError" crates/vox-orchestrator-d/src
```

Expected: check succeeds; exactly one file with count `1`.

- [ ] **Step 7: Manual refusal smoke**

```bash
cargo build -q -p vox-orchestrator-d
VOX_MCP_HTTP_ENABLED=1 VOX_MCP_HTTP_HOST=0.0.0.0 VOX_MCP_HTTP_BEARER_TOKEN=w ./target/debug/vox-orchestrator-d 2>&1 | rg -m1 "refusing to start"; echo "exit=${PIPESTATUS[0]}"
```

Expected: one line containing `refusing to start the MCP HTTP gateway on 0.0.0.0:3921 (non-loopback bind); missing: read token VOX_MCP_HTTP_READ_BEARER_TOKEN ...; TLS (...)`, then a non-zero `exit=`. If plan 03 changed the daemon's required arguments, use the invocation from plan 03's boot tests.

- [ ] **Step 8: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
cargo clippy -p vox-orchestrator-d --all-targets -- -D warnings
git add crates/vox-orchestrator-mcp/src/http_gateway crates/vox-orchestrator-d/src
git commit -m "feat(mcp-http): refuse unsafe network binds at startup; serve TLS"
```

---

### Task 5: Peer-derived `CallContext` on every tool path

Depends on Task 4.

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs`, `rpc_tools.rs`, `ws.rs`, `status.rs`
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/network_policy_tests.rs`

**Interfaces:**

```rust
pub(super) fn caller_is_loopback(state: &GatewayState, peer: &SocketAddr) -> bool;
pub(super) fn call_context(state: &GatewayState, peer: &SocketAddr, role: AccessRole) -> CallContext;
pub(super) fn visible_tools_for(state: &GatewayState, ctx: CallContext) -> HashSet<String>;
pub(super) fn gateway_denial(state: &GatewayState, ctx: CallContext, tool: &str) -> Option<ProfileDenial>;
pub(super) async fn gated_tool_json(state: &GatewayState, ctx: CallContext, tool: &str) -> Value;
// call_tool_response / call_tool_response_value / ws_handle_message take `ctx: CallContext` instead of `role: AccessRole`
```

- [ ] **Step 1: Write the failing tests**

Append to `network_policy_tests.rs`:

```rust
use crate::profiles::{CallContext, CallerRole, Exposure};
use axum::body::to_bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::HeaderValue;
use vox_mcp_registry::McpToolRegistryEntry;

const LAN_PEER: &str = "192.168.1.20:50123";
const LOOPBACK_PEER: &str = "127.0.0.1:50123";

fn peer(s: &str) -> SocketAddr {
    s.parse().unwrap()
}

/// First registry tool matching `pred` that needs no subsystem, so the result does not depend
/// on which subsystems the test ServerState booted.
fn tool_where(pred: impl Fn(&McpToolRegistryEntry) -> bool) -> &'static str {
    TOOL_REGISTRY
        .iter()
        .find(|e| e.requires.is_empty() && pred(e))
        .map(|e| e.name)
        .expect("registry has a matching subsystem-free tool")
}

fn internal_tool() -> &'static str {
    tool_where(|e| e.exposure == Exposure::Internal)
}
fn operator_tool() -> &'static str {
    tool_where(|e| e.exposure == Exposure::Operator)
}
fn public_read_tool() -> &'static str {
    tool_where(|e| e.exposure == Exposure::Public && e.http_read_role_eligible)
}

async fn dev_state_all_tools() -> GatewayState {
    let all: HashSet<String> = TOOL_REGISTRY.iter().map(|e| e.name.to_string()).collect();
    GatewayState::for_test_with_state(ServerState::new_test().await, all).await
}

fn bearer(token: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    h.insert(
        "authorization",
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    h
}

async fn body_json(resp: Response) -> (StatusCode, Value) {
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn loopback_comes_from_the_peer_not_headers() {
    let state = dev_state_all_tools().await;
    assert!(caller_is_loopback(&state, &peer(LOOPBACK_PEER)));
    assert!(caller_is_loopback(&state, &peer("[::1]:9")));
    assert!(!caller_is_loopback(&state, &peer(LAN_PEER)));
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    assert_eq!(ctx, CallContext { role: CallerRole::Write, loopback: false });
}

#[tokio::test]
async fn tunnel_callers_are_never_loopback() {
    let state = dev_state_all_tools().await.with_behind_tunnel(true);
    assert!(!caller_is_loopback(&state, &peer(LOOPBACK_PEER)));
}

#[tokio::test]
async fn internal_tools_are_visible_only_to_loopback() {
    let state = dev_state_all_tools().await;
    let internal = internal_tool();
    let local = visible_tools_for(&state, call_context(&state, &peer(LOOPBACK_PEER), AccessRole::Write));
    let remote = visible_tools_for(&state, call_context(&state, &peer(LAN_PEER), AccessRole::Write));
    assert!(local.contains(internal), "dev profile on loopback serves {internal}");
    assert!(!remote.contains(internal), "{internal} must not be served off loopback");
}

#[tokio::test]
async fn non_loopback_peer_cannot_call_internal_tool() {
    let state = dev_state_all_tools().await;
    let internal = internal_tool();
    let req: ToolCallRequest =
        serde_json::from_value(serde_json::json!({ "name": internal, "args": {} })).unwrap();
    let resp = http_call_tool(
        State(state),
        ConnectInfo(peer(LAN_PEER)),
        HeaderMap::new(),
        Json(req),
    )
    .await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn read_token_off_loopback_cannot_call_operator_tool() {
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    let op = operator_tool();
    let req: ToolCallRequest =
        serde_json::from_value(serde_json::json!({ "name": op, "args": {} })).unwrap();
    let resp = http_call_tool(State(state), ConnectInfo(peer(LAN_PEER)), bearer("r"), Json(req)).await;
    let (status, body) = body_json(resp).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn v1_tools_off_loopback_hides_internal_and_respects_role() {
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    let names = |v: &Value| -> Vec<String> {
        v["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap().to_string())
            .collect()
    };
    let (_, read) =
        body_json(http_tools(State(state.clone()), ConnectInfo(peer(LAN_PEER)), bearer("r")).await).await;
    let (_, write) =
        body_json(http_tools(State(state), ConnectInfo(peer(LAN_PEER)), bearer("w")).await).await;
    let (read, write) = (names(&read), names(&write));
    assert!(read.iter().any(|n| n == public_read_tool()));
    assert!(!read.iter().any(|n| n == operator_tool()));
    assert!(write.iter().any(|n| n == operator_tool()));
    assert!(!read.iter().chain(write.iter()).any(|n| n == internal_tool()));
}

#[tokio::test]
async fn ws_list_tools_hides_internal_off_loopback() {
    let state = dev_state_all_tools().await;
    let req: WsMessageIn =
        serde_json::from_value(serde_json::json!({ "type": "list_tools" })).unwrap();
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    let out = ws_handle_message(&state, req, peer(LAN_PEER), ctx).await;
    let tools: Vec<&str> = out.data["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert!(!tools.contains(&internal_tool()));
}

#[tokio::test]
async fn ws_call_internal_off_loopback_is_denied() {
    let state = dev_state_all_tools().await;
    let req: WsMessageIn = serde_json::from_value(
        serde_json::json!({ "type": "call_tool", "name": internal_tool(), "args": {} }),
    )
    .unwrap();
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    let out = ws_handle_message(&state, req, peer(LAN_PEER), ctx).await;
    assert!(out.is_error);
    assert_eq!(out.data["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn gated_tool_json_denies_without_executing() {
    let state = dev_state_all_tools().await;
    let ctx = call_context(&state, &peer(LAN_PEER), AccessRole::Write);
    let v = gated_tool_json(&state, ctx, internal_tool()).await;
    assert_eq!(v["error"]["code"], "exposure_denied");
}

#[tokio::test]
async fn dashboard_token_is_not_honoured_behind_a_tunnel() {
    let tunnelled = dev_state_all_tools()
        .await
        .with_tokens(Some("w"), Some("r"))
        .with_dashboard_token("dt")
        .with_behind_tunnel(true);
    assert!(resolve_access_role(&tunnelled, &bearer("dt"), Some(&peer(LOOPBACK_PEER))).is_err());
    let direct = dev_state_all_tools()
        .await
        .with_tokens(Some("w"), Some("r"))
        .with_dashboard_token("dt");
    assert_eq!(
        resolve_access_role(&direct, &bearer("dt"), Some(&peer(LOOPBACK_PEER))),
        Ok(AccessRole::Write)
    );
}
```

- [ ] **Step 2: Run them to see them fail**

```bash
cargo nextest run -p vox-orchestrator-mcp network_policy_tests
```

Expected: compile errors for `caller_is_loopback`, `call_context`, `visible_tools_for`,
`with_tokens`, `with_behind_tunnel`, `with_dashboard_token`, `gated_tool_json`, and the
`ws_handle_message` argument type.

- [ ] **Step 3: Context helpers and test builders in `mod.rs`**

Add to the imports:

```rust
use crate::profiles::{CallContext, CallerRole, ProfileDenial};
```

Add after `resolve_access_role`:

```rust
/// Peer-derived, never header-derived. Behind a same-host tunnel every remote caller arrives
/// from 127.0.0.1, so `behind_tunnel` disables loopback trust entirely.
pub(super) fn caller_is_loopback(state: &GatewayState, peer: &SocketAddr) -> bool {
    !state.behind_tunnel && bind_policy::is_loopback_ip(peer.ip())
}

pub(super) fn call_context(state: &GatewayState, peer: &SocketAddr, role: AccessRole) -> CallContext {
    CallContext {
        role: match role {
            AccessRole::Read => CallerRole::Read,
            AccessRole::Write => CallerRole::Write,
        },
        loopback: caller_is_loopback(state, peer),
    }
}

/// Advertised set = gateway allowlist for the role, filtered by `guard_tool` for `ctx`.
/// `/v1/tools`, `/v1/info`, WS `list_tools`, and `/v1/profile` all use this.
pub(super) fn visible_tools_for(state: &GatewayState, ctx: CallContext) -> HashSet<String> {
    let role = match ctx.role {
        CallerRole::Read => AccessRole::Read,
        CallerRole::Write => AccessRole::Write,
    };
    let mut visible = visible_tools_for_role(state, role);
    visible.retain(|name| crate::profiles::guard_tool(&state.server_state, ctx, name).is_ok());
    visible
}
```

In `visible_tools_for_role`, delete plan 02's trailing
`visible.retain(|name| { crate::profiles::guard_tool(&state.server_state, crate::profiles::CallContext::LOCAL_WRITE, name).is_ok() });`
so the function returns only the env-allowlist/read-role set again (the `match` expression becomes
the function body). Profile filtering now happens once, in `visible_tools_for`.

In `resolve_access_role`, replace
`let is_loopback = peer.map(|p| p.ip().is_loopback()).unwrap_or(false);` with:

```rust
        let is_loopback = peer.map(|p| caller_is_loopback(state, p)).unwrap_or(false);
```

In `impl GatewayState`, after `for_test_with_state`:

```rust
    #[cfg(test)]
    pub(crate) fn with_tokens(mut self, write: Option<&str>, read: Option<&str>) -> Self {
        self.bearer_token = write.map(str::to_string);
        self.read_bearer_token = read.map(str::to_string);
        self.allow_unauthenticated = false;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_behind_tunnel(mut self, on: bool) -> Self {
        self.behind_tunnel = on;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_dashboard_token(mut self, token: &str) -> Self {
        self.dashboard_token = Some(DashboardToken(token.to_string()));
        self
    }
```

`AccessRole` already derives `Debug, Clone, Copy, PartialEq, Eq`, which the tests' `assert_eq!` needs.

- [ ] **Step 4: `rpc_tools.rs` — context plumbing and denials**

Add:

```rust
pub(super) fn gateway_denial(
    state: &GatewayState,
    ctx: CallContext,
    tool: &str,
) -> Option<ProfileDenial> {
    crate::profiles::guard_tool(&state.server_state, ctx, tool).err()
}

/// Call a fixed tool for an aggregate endpoint, still subject to the caller's context.
pub(super) async fn gated_tool_json(state: &GatewayState, ctx: CallContext, tool: &str) -> Value {
    match gateway_denial(state, ctx, tool) {
        Some(d) => crate::profiles::denial_error_json(tool, &d, &state.server_state.active_profile),
        None => {
            call_tool_json(&state.server_state, tool, Value::Object(serde_json::Map::new())).await
        }
    }
}
```

In `http_tools`, replace `let visible_tools = visible_tools_for_role(&state, role);` with:

```rust
    let visible_tools = visible_tools_for(&state, call_context(&state, &connect.0, role));
```

In `http_call_tool`, delete plan 02's `let canonical = ...; if let Err(denial) = crate::profiles::guard_tool(&state.server_state, crate::profiles::CallContext::LOCAL_WRITE, &canonical) { ... }`
block and replace the final call with:

```rust
    let ctx = call_context(&state, &connect.0, role);
    call_tool_response(&state, req.name, req.args, Some(connect.0), ctx).await
```

Replace `call_tool_response` with:

```rust
pub(super) async fn call_tool_response(
    state: &GatewayState,
    name: String,
    args: Value,
    peer: Option<SocketAddr>,
    ctx: CallContext,
) -> Response {
    let canonical = canonical_tool_name(name.as_str()).to_string();
    if let Some(d) = gateway_denial(state, ctx, &canonical) {
        let status = StatusCode::from_u16(d.http_status()).unwrap_or(StatusCode::FORBIDDEN);
        let body = crate::profiles::denial_error_json(&canonical, &d, &state.server_state.active_profile);
        return (status, Json(body)).into_response();
    }
    Json(call_tool_response_value(state, canonical, args, peer, ctx).await).into_response()
}
```

In `call_tool_response_value`: change the parameter `role: AccessRole` to `ctx: CallContext`;
in plan 02's guard block replace `crate::profiles::CallContext::LOCAL_WRITE` with `ctx`; and
replace `if !visible_tools_for_role(state, role).contains(&canonical) {` with:

```rust
    let role = match ctx.role {
        CallerRole::Read => AccessRole::Read,
        CallerRole::Write => AccessRole::Write,
    };
    if !visible_tools_for_role(state, role).contains(&canonical) {
```

(The guard block, now using `ctx`, stays first, so profile/exposure denials carry their typed
code; the env-allowlist message is unchanged.)

- [ ] **Step 5: `ws.rs` and `status.rs`**

`ws.rs` `handle_ws`: in the dashboard-token branch replace `if peer.ip().is_loopback() {` with
`if caller_is_loopback(&state, &peer) {`. Replace
`Ok(req) => ws_handle_message(&state, req, peer, role).await,` with:

```rust
                            Ok(req) => {
                                let ctx = call_context(&state, &peer, role);
                                ws_handle_message(&state, req, peer, ctx).await
                            }
```

`ws_handle_message`: change the parameter `role: AccessRole` to `ctx: CallContext`; replace
`visible_tools_for_role(state, role)` with `visible_tools_for(state, ctx)`; replace
`call_tool_response_value(state, name, args, Some(peer), role)` with
`call_tool_response_value(state, name, args, Some(peer), ctx)`.

`status.rs` `http_info`: replace

```rust
    let mut read_role_allowed_tools: Vec<String> = visible_tools_for_role(&state, AccessRole::Read)
```

with

```rust
    let mut read_role_allowed_tools: Vec<String> =
        visible_tools_for(&state, call_context(&state, &connect.0, AccessRole::Read))
```

`status.rs` `http_mobile_status`: after the guard, resolve the role and gate both tools,
replacing the two direct `call_tool_json(...)` calls:

```rust
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
    let git = gated_tool_json(&state, ctx, "vox_git_status").await;
    let orch = gated_tool_json(&state, ctx, "vox_orchestrator_status").await;
```

The startup `GatewayInfo` log in `spawn_http_gateway` still calls
`visible_tools_for_role(&gateway_state, AccessRole::Read)`; that is a config description with no
caller, so it stays.

- [ ] **Step 6: Run the tests, and prove there is one construction point**

```bash
cargo nextest run -p vox-orchestrator-mcp network_policy_tests http_gateway::tests
rg -n "LOCAL_WRITE|loopback: true" crates/vox-orchestrator-mcp/src/http_gateway/
rg -n "\.is_loopback\(\)" crates/vox-orchestrator-mcp/src/http_gateway/
```

Expected: all tests pass. The first `rg` has hits only inside `#[cfg(test)]` modules (inspect each).
The second `rg` has exactly one hit, `ip.to_canonical().is_loopback()` in `bind_policy.rs`.

- [ ] **Step 7: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
git add crates/vox-orchestrator-mcp/src/http_gateway
git commit -m "feat(mcp-http): derive CallContext from the TCP peer and matched token"
```

---

### Task 6: `/api/v2` dashboard exposure rules

Depends on Task 5. **Parallel-safe with:** Task 7 (both append to `network_policy_tests.rs`; append-only).

`/api/v2` is the dashboard REST surface that Axis remote (plan 06) will use. This plan's decision:

| Route | Method | Required |
|---|---|---|
| `/health` (stateless, `services::routes`) | GET | none |
| `/mesh/nodes`, `/runs/recent`, `/dashboard/layout`, `/routing/summary`, `/routing/manual-ssot`, `/models/catalog`, `/scientia/queue`, `/scientia/cost`, `/vcs/isolation`, `/hopper/inbox`, `/hopper/assigned`, `/hopper/history` | GET | read or write token (`enforce_dashboard_read`, unchanged) |
| `/dashboard/layout` | PUT | write token (`enforce_dashboard_write`, unchanged) |
| `/vcs/isolation/strategy` | POST | write token (unchanged) |
| `/hopper/submit` | POST | write token (unchanged) |
| `/mesh/nodes/{id}/kill` | POST | write token **and a loopback caller** (new): it is a mesh emergency control, which spec §3 classes `internal` |

`/api/v2` routes are not filtered by profile in this plan; per-surface gating is plans 06/07.

**Files:**
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs`
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/network_policy_tests.rs`

**Interfaces:** `fn enforce_dashboard_loopback_write(gs: &GatewayState, peer: &SocketAddr, headers: &HeaderMap) -> GuardResult` (private).

- [ ] **Step 1: Write the failing tests**

Append to `network_policy_tests.rs`:

```rust
#[tokio::test]
async fn mesh_kill_is_loopback_only() {
    use axum::extract::Path;
    let state = dev_state_all_tools().await;
    let remote = dashboard_api::post_mesh_node_kill(
        State(state.clone()),
        ConnectInfo(peer(LAN_PEER)),
        HeaderMap::new(),
        Path("node-1".to_string()),
    )
    .await;
    assert_eq!(remote.0["error"]["code"], "exposure_denied");

    let local = dashboard_api::post_mesh_node_kill(
        State(state),
        ConnectInfo(peer(LOOPBACK_PEER)),
        HeaderMap::new(),
        Path("node-1".to_string()),
    )
    .await;
    assert_eq!(local.0["data"]["node_id"], "node-1");
}

#[tokio::test]
async fn dashboard_write_routes_still_need_write_token_off_loopback() {
    use dashboard_api::{HopperSubmitBody, post_hopper_submit};
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    let body: HopperSubmitBody = serde_json::from_value(serde_json::json!({
        "intent": "fix the flaky http_gateway test",
        "affinity_hints": ["crates/vox-orchestrator-mcp"],
        "priority_hint": "urgent",
        "source": "developer"
    }))
    .unwrap();
    let resp = post_hopper_submit(State(state), ConnectInfo(peer(LAN_PEER)), bearer("r"), Json(body)).await;
    let (_, v) = body_json(resp.into_response()).await;
    assert_eq!(v["error"]["code"], "forbidden");
}
```

- [ ] **Step 2: Run them to see the new one fail**

```bash
cargo nextest run -p vox-orchestrator-mcp mesh_kill_is_loopback_only dashboard_write_routes_still_need_write_token_off_loopback
```

Expected: `mesh_kill_is_loopback_only` FAILS (the remote call returns the `data` envelope);
`dashboard_write_routes_still_need_write_token_off_loopback` passes (it pins existing behaviour).

- [ ] **Step 3: Implement**

In `dashboard_api.rs`, add `caller_is_loopback` to the `use super::{...}` list, and add after
`enforce_dashboard_write`:

```rust
/// Emergency/mesh controls: write token AND a loopback caller (spec §3 `internal`).
fn enforce_dashboard_loopback_write(
    gs: &GatewayState,
    peer: &SocketAddr,
    headers: &HeaderMap,
) -> GuardResult {
    enforce_dashboard_write(gs, peer, headers)?;
    if !caller_is_loopback(gs, peer) {
        return Err(err(
            "exposure_denied",
            "this operation is only available to loopback callers",
        ));
    }
    Ok(())
}
```

In `post_mesh_node_kill`, replace `enforce_dashboard_write(&gs, &connect.0, &headers)` with
`enforce_dashboard_loopback_write(&gs, &connect.0, &headers)`.

Append to the module doc comment at the top of `dashboard_api.rs`:

```rust
//! - **Exposure (capability-profiles plan 04):** every GET needs a read or write token; PUT
//!   `/dashboard/layout`, POST `/vcs/isolation/strategy`, POST `/hopper/submit` need the write
//!   token; POST `/mesh/nodes/{id}/kill` additionally needs a loopback caller. Loopback is the
//!   TCP peer (never a header) and is never granted when `VOX_MCP_HTTP_BEHIND_TUNNEL=1`.
```

- [ ] **Step 4: Run the tests to see them pass**

```bash
cargo nextest run -p vox-orchestrator-mcp network_policy_tests http_gateway::tests
```

Expected: all pass.

- [ ] **Step 5: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
git add crates/vox-orchestrator-mcp/src/http_gateway
git commit -m "feat(mcp-http): mesh kill is loopback-only; document /api/v2 roles"
```

---

### Task 7: `GET /v1/profile` and its OpenAPI contract

Depends on Task 5. **Parallel-safe with:** Task 6.

**Files:**
- Create: `crates/vox-orchestrator-mcp/src/http_gateway/profile_info.rs`
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs` (module + route)
- Modify: `contracts/mcp/http-gateway.openapi.yaml`
- Modify: `crates/vox-orchestrator-mcp/src/http_gateway/network_policy_tests.rs`

**Interfaces:**

```rust
pub(super) struct ProfileInfo { profiles, max_exposure: Exposure, subsystems: ProfileSubsystems, tools: Vec<String>, axis_surfaces, server_version: &'static str }
pub(super) fn profile_info(state: &GatewayState, ctx: CallContext) -> ProfileInfo;
pub(super) async fn http_profile(State<GatewayState>, ConnectInfo<SocketAddr>, HeaderMap) -> Response;
```

Wire shape (verbatim from `00-contracts.md`):

```json
{
  "profiles": ["models"],
  "max_exposure": "operator",
  "subsystems": { "booted": ["llm_facade", "model_registry"], "failed": [{"id": "db", "error": "..."}] },
  "tools": ["vox_list_models", "..."],
  "axis_surfaces": ["chat", "models"],
  "server_version": "0.x.y"
}
```

`profiles`, `booted`, `failed` (by `id`), `tools`, and `axis_surfaces` are sorted ascending as strings.

- [ ] **Step 1: Write the failing tests**

Append to `network_policy_tests.rs`:

```rust
async fn get_profile(state: GatewayState, from: &str, headers: HeaderMap) -> (StatusCode, Value) {
    body_json(profile_info::http_profile(State(state), ConnectInfo(peer(from)), headers).await).await
}

fn str_list(v: &Value) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

#[tokio::test]
async fn profile_has_the_contract_shape() {
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    let (status, v) = get_profile(state, LOOPBACK_PEER, bearer("w")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(str_list(&v["profiles"]), vec!["dev".to_string()]);
    assert_eq!(v["max_exposure"], "internal");
    assert!(v["subsystems"]["booted"].is_array());
    assert!(v["subsystems"]["failed"].is_array());
    assert!(v["axis_surfaces"].is_array());
    assert_eq!(v["server_version"], env!("CARGO_PKG_VERSION"));
    let tools = str_list(&v["tools"]);
    let mut sorted = tools.clone();
    sorted.sort();
    assert_eq!(tools, sorted, "tools are sorted");
    let booted = str_list(&v["subsystems"]["booted"]);
    let mut sorted = booted.clone();
    sorted.sort();
    assert_eq!(booted, sorted, "booted subsystems are sorted");
}

#[tokio::test]
async fn profile_read_token_off_loopback_lists_only_public_read_eligible_tools() {
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    let (status, v) = get_profile(state, LAN_PEER, bearer("r")).await;
    assert_eq!(status, StatusCode::OK);
    let tools = str_list(&v["tools"]);
    assert!(tools.contains(&public_read_tool().to_string()));
    for t in &tools {
        let e = crate::profiles::registry_entry(t).unwrap();
        assert_eq!(e.exposure, Exposure::Public, "{t}");
        assert!(e.http_read_role_eligible, "{t}");
    }
}

#[tokio::test]
async fn profile_write_token_off_loopback_includes_operator_never_internal() {
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    let (_, v) = get_profile(state, LAN_PEER, bearer("w")).await;
    let tools = str_list(&v["tools"]);
    assert!(tools.contains(&operator_tool().to_string()));
    assert!(!tools.contains(&internal_tool().to_string()));
}

#[tokio::test]
async fn profile_requires_a_token_off_loopback() {
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    let (status, _) = get_profile(state, LAN_PEER, HeaderMap::new()).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn profile_tools_equal_v1_tools_for_the_same_caller() {
    let state = dev_state_all_tools().await.with_tokens(Some("w"), Some("r"));
    for token in ["r", "w"] {
        let (_, p) = get_profile(state.clone(), LAN_PEER, bearer(token)).await;
        let (_, t) = body_json(
            http_tools(State(state.clone()), ConnectInfo(peer(LAN_PEER)), bearer(token)).await,
        )
        .await;
        let listed: Vec<String> = t["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| d["name"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(str_list(&p["tools"]), listed, "token {token}");
    }
}

#[test]
fn openapi_documents_v1_profile() {
    let spec = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/mcp/http-gateway.openapi.yaml"
    ));
    assert!(spec.contains("\n  /v1/profile:\n"), "path missing");
    for key in ["max_exposure", "axis_surfaces", "server_version", "booted", "failed"] {
        assert!(spec.contains(key), "{key} missing from /v1/profile schema");
    }
}
```

- [ ] **Step 2: Run them to see them fail**

```bash
cargo nextest run -p vox-orchestrator-mcp profile_ openapi_documents_v1_profile
```

Expected: compile error `could not find profile_info in super`.

- [ ] **Step 3: Implement the handler**

`crates/vox-orchestrator-mcp/src/http_gateway/profile_info.rs`:

```rust
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
    let boot = state.server_state.boot_report_snapshot();
    let mut profiles = profile.ids.clone();
    profiles.sort_unstable();
    let mut booted: Vec<Subsystem> = boot.booted.iter().copied().collect();
    booted.sort_by_key(|s| s.as_str());
    let mut failed: Vec<FailedSubsystem> = boot
        .failed
        .iter()
        .map(|(s, e)| FailedSubsystem {
            id: *s,
            error: e.clone(),
        })
        .collect();
    failed.sort_by_key(|f| f.id.as_str());
    let mut tools: Vec<String> = visible_tools_for(state, ctx).into_iter().collect();
    tools.sort();
    ProfileInfo {
        profiles,
        max_exposure: profile.max_exposure,
        subsystems: ProfileSubsystems { booted, failed },
        tools,
        axis_surfaces: profile.axis_surfaces.iter().copied().collect(),
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
```

In `mod.rs`: add `mod profile_info;` next to `mod rpc_tools;`, and in `build_app`, after
`.route("/v1/tools/call", post(http_call_tool))`, add:

```rust
        .route("/v1/profile", get(profile_info::http_profile))
```

- [ ] **Step 4: OpenAPI**

In `contracts/mcp/http-gateway.openapi.yaml`, append to `info.description` (keep the existing
text, add a blank line, then):

```yaml
    Bind policy: loopback (127.0.0.0/8, ::1) by default. Any other bind host, or any bind with
    `VOX_MCP_HTTP_BEHIND_TUNNEL=1`, requires both bearer tokens and TLS
    (`VOX_MCP_HTTP_TLS_CERT`/`VOX_MCP_HTTP_TLS_KEY`) or the tunnel attestation; the gateway
    refuses to start otherwise. Off loopback, `internal` tools are never served, the read token
    sees `public` read-eligible tools, and the write token sees `public` + `operator`.
```

Insert directly before the top-level `components:` line:

```yaml
  /v1/profile:
    get:
      summary: Active capability profile and the tools served to this caller
      description: |
        `tools` is exactly the set `/v1/tools` advertises to the same token from the same peer:
        gateway allowlist, then profile membership, exposure (internal is loopback-only;
        operator needs the write token), read-role eligibility, and booted subsystems.
        Arrays are sorted ascending.
      security:
        - bearerAuth: []
      responses:
        "200":
          description: Active profile snapshot
          content:
            application/json:
              schema:
                type: object
                required: [profiles, max_exposure, subsystems, tools, axis_surfaces, server_version]
                properties:
                  profiles:
                    type: array
                    items:
                      type: string
                    example: [models]
                  max_exposure:
                    type: string
                    enum: [public, operator, internal]
                  subsystems:
                    type: object
                    required: [booted, failed]
                    properties:
                      booted:
                        type: array
                        items:
                          type: string
                        example: [llm_facade, model_registry]
                      failed:
                        type: array
                        items:
                          type: object
                          required: [id, error]
                          properties:
                            id:
                              type: string
                            error:
                              type: string
                  tools:
                    type: array
                    items:
                      type: string
                  axis_surfaces:
                    type: array
                    items:
                      type: string
                    example: [chat, models]
                  server_version:
                    type: string
        "401":
          description: Missing or invalid bearer token
        "403":
          description: Forwarded HTTPS requirement failed
        "429":
          description: Rate limit exceeded
```

Under `/v1/tools/call` → `post` → `responses`, add these entries unless plan 02 already did:

```yaml
        "403":
          description: "`exposure_denied` or `insufficient_role` — body `{\"error\":{\"code\",\"message\"}}`"
        "404":
          description: "`tool_not_in_profile`"
        "503":
          description: "`subsystem_not_booted` — body carries `error.subsystem`"
```

- [ ] **Step 5: Run the tests to see them pass**

```bash
cargo nextest run -p vox-orchestrator-mcp network_policy_tests
```

Expected: all `network_policy_tests` pass (6 more than after Task 6).

- [ ] **Step 6: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-orchestrator-mcp --all-targets -- -D warnings
git add crates/vox-orchestrator-mcp/src/http_gateway contracts/mcp/http-gateway.openapi.yaml
git commit -m "feat(mcp-http): GET /v1/profile reports profile, subsystems, served tools"
```

---

### Task 8: `vox doctor` reports the effective bind policy

Depends on Task 4. **Parallel-safe with:** Tasks 5–7, 9.

**Files:**
- Create: `crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/gateway_bind.rs`
- Modify: `crates/vox-cli/src/commands/diagnostics/doctor/checks_standard/mod.rs`

**Interfaces:** `pub(super) fn run(checks: &mut Vec<Check>)`; `fn bind_policy_check(enabled: bool, result: &Result<BindPolicy, BindPolicyError>) -> Check`.

- [ ] **Step 1: Write the failing tests**

In `checks_standard/mod.rs`, add `mod gateway_bind;` after `mod freshness;`. Create
`gateway_bind.rs` with the tests:

```rust
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
        assert!(c.detail.contains("127.0.0.1:3921 — loopback only, plain HTTP"), "{}", c.detail);
    }

    #[test]
    fn enabled_unsafe_network_bind_fails_naming_what_is_missing() {
        let c = bind_policy_check(true, &eval("0.0.0.0"));
        assert!(!c.pass);
        assert!(c.detail.contains("VOX_MCP_HTTP_READ_BEARER_TOKEN"), "{}", c.detail);
    }

    #[test]
    fn disabled_gateway_passes_but_warns_about_a_bad_config() {
        let c = bind_policy_check(false, &eval("0.0.0.0"));
        assert!(c.pass);
        assert!(c.detail.contains("would refuse to start"), "{}", c.detail);
    }
}
```

- [ ] **Step 2: Run them to see them fail**

```bash
cargo nextest run -p vox-cli gateway_bind
```

Expected: compile error `cannot find function bind_policy_check` / `CHECK_NAME`.

- [ ] **Step 3: Implement**

Above the tests in `gateway_bind.rs`:

```rust
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
            format!("disabled (VOX_MCP_HTTP_ENABLED unset); would bind {}", p.summary()),
        ),
        (true, Err(e)) => Check::fail(CHECK_NAME, e.to_string()),
        (false, Err(e)) => Check::pass(
            CHECK_NAME,
            format!("disabled (VOX_MCP_HTTP_ENABLED unset); would refuse to start if enabled: {e}"),
        ),
    }
}
```

In `checks_standard/mod.rs` `run_checks`, after `secrets::run(auto_heal, checks).await;`:

```rust
    gateway_bind::run(checks);
```

- [ ] **Step 4: Run the tests to see them pass**

```bash
cargo nextest run -p vox-cli gateway_bind
cargo run -q -p vox-cli -- doctor 2>&1 | rg "MCP HTTP gateway bind policy"
```

Expected: `3 tests run: 3 passed`. On a machine with default settings the doctor line reads
`disabled (VOX_MCP_HTTP_ENABLED unset); would bind 127.0.0.1:3921 — loopback only, plain HTTP`.

- [ ] **Step 5: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-cli --all-targets -- -D warnings
git add crates/vox-cli/src/commands/diagnostics/doctor/checks_standard
git commit -m "feat(doctor): report the MCP HTTP gateway bind policy"
```

---

### Task 9: `vox serve` network flags

Depends on Task 2 and plan 02's `vox serve`. **Parallel-safe with:** Tasks 4–8.

**Files:**
- Create: `crates/vox-cli/src/commands/serve_network.rs`
- Modify: `crates/vox-cli/src/commands/mod.rs`, `crates/vox-cli/src/lib.rs` (`Cli::Serve`), `crates/vox-cli/src/cli_dispatch/mod.rs` (`Cli::Serve` arm), `crates/vox-cli/src/commands/serve.rs` (`run`)
- Regenerate: `docs/src/reference/cli-command-surface.generated.md` (if `command-sync` changes it)

**Interfaces:**

```rust
#[derive(clap::Args, Debug, Clone, Default)]
pub struct ServeNetworkArgs { pub bind: Option<SocketAddr>, pub tls_cert: Option<PathBuf>, pub tls_key: Option<PathBuf>, pub behind_tunnel: bool }
impl ServeNetworkArgs { pub fn gateway_env(&self) -> std::io::Result<Vec<(&'static str, String)>>; }
// plan 02's `serve::run(profile_flag: Option<&str>)` becomes `serve::run(profile_flag: Option<&str>, network: &ServeNetworkArgs)`
```

- [ ] **Step 1: Write the failing tests**

`crates/vox-cli/src/commands/serve_network.rs`, tests first:

```rust
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
        assert!(ServeNetworkArgs::default().gateway_env().unwrap().is_empty());
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
        assert!(t.n.gateway_env().unwrap().contains(&("VOX_MCP_HTTP_HOST", "::1".to_string())));
    }

    #[test]
    fn tls_paths_become_absolute() {
        let t = T::try_parse_from(["t", "--tls-cert", "c.pem", "--tls-key", "k.pem"]).unwrap();
        let env = t.n.gateway_env().unwrap();
        let cert = env.iter().find(|(k, _)| *k == "VOX_MCP_HTTP_TLS_CERT").unwrap();
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
```

In `crates/vox-cli/src/commands/mod.rs`, add `pub mod serve_network;` next to plan 02's `pub mod serve;`.

- [ ] **Step 2: Run them to see them fail**

```bash
cargo nextest run -p vox-cli serve_network
```

Expected: compile error `cannot find type ServeNetworkArgs`.

- [ ] **Step 3: Implement**

Above the tests in `serve_network.rs`:

```rust
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
    #[arg(long, value_name = "PEM", requires = "tls_key", conflicts_with = "behind_tunnel")]
    pub tls_cert: Option<PathBuf>,
    /// PEM private key for TLS termination (requires `--tls-cert`).
    #[arg(long, value_name = "PEM", requires = "tls_cert", conflicts_with = "behind_tunnel")]
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
```

Wire it into plan 02's command. In `crates/vox-cli/src/lib.rs`, extend the `Serve` variant:

```rust
    Serve {
        /// Capability profile ids (comma-separated), e.g. `orchestrator,knowledge,mesh`. Default: `VOX_PROFILE`, then `~/.vox/config.toml`, then `dev`.
        #[arg(long, value_name = "IDS")]
        profile: Option<String>,
        #[command(flatten)]
        network: crate::commands::serve_network::ServeNetworkArgs,
    },
```

In `crates/vox-cli/src/cli_dispatch/mod.rs`, replace plan 02's arm with:

```rust
        Cli::Serve { profile, network } => {
            crate::commands::serve::run(profile.as_deref(), network).await?;
        }
```

(If the dispatcher matches on a reference, pass `network` as-is; otherwise pass `&network`.)

In `crates/vox-cli/src/commands/serve.rs`, change the signature to
`pub async fn run(profile_flag: Option<&str>, network: &crate::commands::serve_network::ServeNetworkArgs) -> Result<()>`,
and directly after plan 02's loop

```rust
    for (key, value) in serve_child_env(&profile_id, socket) {
        cmd.env(key, value);
    }
```

add:

```rust
    for (key, value) in network.gateway_env()? {
        cmd.env(key, value);
    }
```

- [ ] **Step 4: Run the tests and regenerate the CLI surface**

```bash
cargo nextest run -p vox-cli serve_network serve::tests
cargo run -q -p vox-cli -- serve --help | rg -- "--bind|--tls-cert|--tls-key|--behind-tunnel"
cargo run -q -p vox-cli -- ci command-sync --write
cargo run -q -p vox-cli -- ci command-compliance
git status --short docs/src/reference contracts/cli crates/vox-cli/tests/fixtures
```

Expected: `serve_network` 7 passed and plan 02's `serve::tests` still pass; four help lines;
`command-sync`/`command-compliance` exit 0; commit any regenerated files listed.

- [ ] **Step 5: Checks and commit**

```bash
vox run scripts/fmt.vox
cargo clippy -p vox-cli --all-targets -- -D warnings
git add crates/vox-cli/src docs/src/reference contracts/cli crates/vox-cli/tests/fixtures
git commit -m "feat(cli): vox serve --bind/--tls-cert/--tls-key/--behind-tunnel"
```

---

### Task 10: Keep the public eval sandbox deployable (decision D2 — no exemption)

Depends on Task 4. **Parallel-safe with:** Tasks 5–9, 11. **Must merge in the same PR as Task 4.**

`vox-eval.compose.yml` (and its mirror `docker/vox-eval.compose.yml`) runs `vox mcp` with
`VOX_MCP_HTTP_HOST=0.0.0.0`, `VOX_MCP_HTTP_PUBLIC_EVAL_ENABLED=1`, and only the write token,
behind Coolify's Traefik TLS. After Task 4 that container **refuses to start** (no read token, no
TLS, no tunnel attestation), so `eval.voxlang.org` would go down on its next deploy.

Decision D2 (`00-contracts.md` Amendments): **no public-eval exemption** from the
"network-exposed requires auth" rule. The operator adds a `VOX_MCP_HTTP_READ_BEARER_TOKEN` Coolify
project secret (any random value; never handed out) before the first deploy that includes Task 4.
This task changes only the compose files' environment wiring/comments and the deployment doc — no
policy, code, or test changes. `/v1/eval` stays reachable without a token exactly as today
(`VOX_MCP_HTTP_PUBLIC_EVAL_ENABLED=1`); only gateway startup requirements change.

**Files:** `vox-eval.compose.yml`, `docker/vox-eval.compose.yml`, `docs/src/architecture/eval-sandbox-deployment.md`.

- [ ] **Step 1: Operator prerequisite (record, do not block on it in code)**

Add to the PR description, under "Deploy prerequisites": "Before deploying this PR to
`eval.voxlang.org`, add the Coolify project secret `VOX_MCP_HTTP_READ_BEARER_TOKEN` (random value,
e.g. `openssl rand -hex 32`). Without it the gateway refuses to start." Do not add any exemption,
flag, or skip path to the bind policy.

- [ ] **Step 2: Update both compose files identically**

In the `environment:` list of each file, after `- VOX_MCP_HTTP_BEARER_TOKEN=${VOX_MCP_HTTP_BEARER_TOKEN}`:

```yaml
      # Bind policy (capability-profiles plan 04): a non-loopback bind needs both tokens and
      # TLS or a tunnel attestation. Traefik terminates TLS for eval.voxlang.org.
      - VOX_MCP_HTTP_READ_BEARER_TOKEN=${VOX_MCP_HTTP_READ_BEARER_TOKEN}
      - VOX_MCP_HTTP_BEHIND_TUNNEL=1
```

```bash
diff <(tail -n +4 docker/vox-eval.compose.yml) vox-eval.compose.yml && echo in-sync
```

Expected: `in-sync` (the mirror differs only by its 3-line header).

- [ ] **Step 3: Document it and commit**

In `docs/src/architecture/eval-sandbox-deployment.md`, in the section that lists required Coolify
secrets, add `VOX_MCP_HTTP_READ_BEARER_TOKEN` with the note: "Required since the gateway bind
policy (capability-profiles plan 04): a `0.0.0.0` bind refuses to start without both tokens. Any
random value; not shared with the docs site."

```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/eval-sandbox-deployment.md
git add vox-eval.compose.yml docker/vox-eval.compose.yml docs/src/architecture/eval-sandbox-deployment.md
git commit -m "chore(eval): satisfy the gateway bind policy in the eval sandbox"
```

Expected: lint exits 0.

---

### Task 11: Docs — "Running Vox as a server"

Depends on Tasks 7 and 9. **Parallel-safe with:** Tasks 8, 10.

**Files:**
- Create: `docs/src/how-to/how-to-run-vox-as-a-server.md`
- Modify: `docs/src/reference/mcp-http-gateway-contract.md`

- [ ] **Step 1: Write the how-to**

`docs/src/how-to/how-to-run-vox-as-a-server.md`:

~~~~markdown
---
title: "Running Vox as a server"
description: "Run a scoped Vox capability profile as an HTTP/MCP server on loopback, on a LAN with TLS, or behind Tailscale or Cloudflare Tunnel, and discover what it serves with GET /v1/profile."
category: "How-To Guides"
status: "current"
---

# Running Vox as a server

`vox serve --profile <ids>` starts the Vox daemon with the MCP HTTP gateway and only the
capability slice you name (for example `models`, `knowledge`, `mesh`; `vox serve --help` lists
them). This page covers where it listens and who can call what.

## Threat model in one paragraph

A Vox server is **single tenant**: one operator, any number of clients, two shared bearer
tokens. The **write token** can call `public` and `operator` tools; the **read token** can call
only `public` tools marked read-eligible. Anyone holding a token acts as the operator — there are
no per-user identities, quotas, or audit separation. `internal` tools (agent gateways, approval
allowlists, emergency stop, secrets diagnostics) are served **only to loopback callers**, whatever
the profile says. "Loopback" is decided by the TCP peer address, never by a header.

## 1. Loopback (default)

```bash
vox serve --profile models
```

Binds `127.0.0.1:3921`. Nothing on the network can reach it.

## 2. LAN with TLS

Create both tokens in the Clavis vault (never put them in shell history or `.env`):

```bash
openssl rand -base64 32 | vox secrets set VOX_MCP_HTTP_BEARER_TOKEN --stdin
openssl rand -base64 32 | vox secrets set VOX_MCP_HTTP_READ_BEARER_TOKEN --stdin
```

Then bind a LAN address with a PEM certificate and key:

```bash
vox serve --profile models,knowledge \
  --bind 0.0.0.0:3921 \
  --tls-cert /etc/vox/tls/cert.pem --tls-key /etc/vox/tls/key.pem
```

The gateway terminates TLS itself (rustls). Clients must trust the certificate; for a
self-signed certificate, Axis pins it on first connect.

## 3. LAN behind Tailscale or Cloudflare Tunnel

If a tunnel already encrypts traffic, attest it instead of configuring TLS:

```bash
# Tailscale: bind the tailnet address; WireGuard encrypts the hop.
vox serve --profile mesh --bind 100.101.102.103:3921 --behind-tunnel

# Cloudflare Tunnel, `tailscale serve`, or a same-host reverse proxy: stay on loopback.
vox serve --profile research --behind-tunnel
```

`--behind-tunnel` makes the gateway treat **every** caller as remote. A same-host tunnel
delivers remote traffic from `127.0.0.1`, so loopback trust would otherwise hand `internal`
tools to the internet. Both tokens are therefore required even on a loopback bind. If the proxy
sets `X-Forwarded-For`, also set `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR=1` so rate limits apply per
client rather than to the tunnel.

> **Do not** put a same-host reverse proxy in front of a loopback bind without `--behind-tunnel`:
> the gateway cannot tell proxied callers from local ones.

## What the gateway refuses

A bind that is not loopback (anything outside `127.0.0.0/8` and `::1`, including `0.0.0.0` and
`::`), or any bind with `--behind-tunnel`, refuses to start unless **all** of these hold:

| Requirement | Set with |
|---|---|
| Write token | `vox secrets set VOX_MCP_HTTP_BEARER_TOKEN --stdin` |
| Read token | `vox secrets set VOX_MCP_HTTP_READ_BEARER_TOKEN --stdin` |
| TLS or tunnel | `--tls-cert`/`--tls-key` (`VOX_MCP_HTTP_TLS_CERT`/`_KEY`) or `--behind-tunnel` (`VOX_MCP_HTTP_BEHIND_TUNNEL=1`) |
| No unauthenticated override | `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` unset |

The error names every missing item. `vox doctor` shows the same verdict under
**MCP HTTP gateway bind policy** without starting anything.

## Discover what a server serves

```bash
curl -s https://vox.lan:3921/v1/profile -H "Authorization: Bearer $READ_TOKEN"
```

```json
{
  "profiles": ["models"],
  "max_exposure": "operator",
  "subsystems": { "booted": ["db", "llm_facade", "model_registry"], "failed": [] },
  "tools": ["vox_list_models"],
  "axis_surfaces": ["chat", "models"],
  "server_version": "0.6.0"
}
```

`tools` is what **your token from your address** may call — the same list `GET /v1/tools`
returns. A tool missing from it fails with `exposure_denied` (403), `insufficient_role` (403),
`tool_not_in_profile` (404), or `subsystem_not_booted` (503).

## Related

- [Environment variables](../reference/env-vars.md) — `VOX_MCP_HTTP_*`
- [MCP HTTP gateway contract](../reference/mcp-http-gateway-contract.md)
~~~~

- [ ] **Step 2: Gateway contract reference**

In `docs/src/reference/mcp-http-gateway-contract.md`, add this section:

```markdown
## Network bind policy and discovery

- Loopback (`127.0.0.0/8`, `::1`) is the default bind. Any other host, or
  `VOX_MCP_HTTP_BEHIND_TUNNEL=1`, requires `VOX_MCP_HTTP_BEARER_TOKEN`,
  `VOX_MCP_HTTP_READ_BEARER_TOKEN`, and TLS (`VOX_MCP_HTTP_TLS_CERT`/`_KEY`) or the tunnel
  attestation; `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` is rejected there. Startup refuses otherwise.
- Caller loopback is the TCP peer address; behind a tunnel no caller is loopback. Off loopback,
  `internal` tools are never served and `POST /api/v2/mesh/nodes/{id}/kill` is refused.
- `GET /v1/profile` returns the active profile, booted/failed subsystems, `axis_surfaces`, server
  version, and the tools the calling token may use. See
  [Running Vox as a server](../how-to/how-to-run-vox-as-a-server.md).
```

- [ ] **Step 3: Lint, check links, commit**

```bash
cargo run -p vox-doc-pipeline -- --lint-only --paths how-to/how-to-run-vox-as-a-server.md
cargo run -p vox-doc-pipeline -- --lint-only --paths reference/mcp-http-gateway-contract.md
cargo run -q -p vox-cli -- ci check-links
git add docs/src/how-to/how-to-run-vox-as-a-server.md docs/src/reference/mcp-http-gateway-contract.md
git commit -m "docs: running Vox as a server (bind policy, TLS, tunnels, /v1/profile)"
```

Expected: every command exits 0.

---

### Task 12: Mutation-verify the security guards

Depends on Tasks 1–7 being committed, so `git checkout -- <file>` restores only the mutation.

**Files:** temporary edits only; nothing committed.

- [ ] **Step 0: Clean tree**

```bash
git status --porcelain crates/ | wc -l
```

Expected: `0`.

- [ ] **Step 1 (a): Break the registry's off-loopback internal check**

```bash
F=crates/vox-mcp-registry/src/profile.rs
grep -c '!ctx.loopback' $F                    # expect 1
grep -c 'false && !ctx.loopback' $F           # expect 0
```

Edit `$F`: in `authorize_tool`, change the step-2 condition from `!ctx.loopback && ...` to
`false && !ctx.loopback && ...`.

```bash
grep -c 'false && !ctx.loopback' $F           # expect 1 (mutation present)
cargo nextest run -p vox-orchestrator-mcp non_loopback_peer_cannot_call_internal_tool internal_tools_are_visible_only_to_loopback profile_write_token_off_loopback_includes_operator_never_internal
```

Expected: all three FAIL (status 200 instead of 403, or the internal tool appears in the remote
set). If any passes, the gateway does not route through `authorize_tool`; stop and fix Task 5.

```bash
git checkout -- $F
grep -c 'false && !ctx.loopback' $F           # expect 0 (restored)
cargo nextest run -p vox-orchestrator-mcp non_loopback_peer_cannot_call_internal_tool
```

Expected: passes again.

- [ ] **Step 2 (a'): Break the gateway's peer-derived loopback**

```bash
F=crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
grep -c '!state.behind_tunnel && bind_policy::is_loopback_ip(peer.ip())' $F   # expect 1
```

Edit `caller_is_loopback` so its body is `let _ = (state, peer); true`.

```bash
grep -c '!state.behind_tunnel && bind_policy::is_loopback_ip(peer.ip())' $F   # expect 0
cargo nextest run -p vox-orchestrator-mcp network_policy_tests
```

Expected: at least `loopback_comes_from_the_peer_not_headers`, `tunnel_callers_are_never_loopback`,
`non_loopback_peer_cannot_call_internal_tool`, `mesh_kill_is_loopback_only`, and
`dashboard_token_is_not_honoured_behind_a_tunnel` FAIL.

```bash
git checkout -- $F
grep -c '!state.behind_tunnel && bind_policy::is_loopback_ip(peer.ip())' $F   # expect 1
```

- [ ] **Step 3 (b): Break the bind policy**

```bash
F=crates/vox-orchestrator-mcp/src/http_gateway/bind_policy.rs
grep -c 'if loopback && !input.behind_tunnel {' $F            # expect 1
grep -c 'if true || (loopback && !input.behind_tunnel) {' $F  # expect 0
```

Edit: change `if loopback && !input.behind_tunnel {` to `if true || (loopback && !input.behind_tunnel) {`.

```bash
grep -c 'if true || (loopback && !input.behind_tunnel) {' $F  # expect 1
cargo nextest run -p vox-orchestrator-mcp spawn_refuses_network_bind_without_tokens unspecified_v4_without_anything_lists_every_missing_item
```

Expected: both FAIL. `spawn_refuses...` fails on the `downcast_ref::<BindPolicyError>` expectation,
or on `expect_err` if the shell happens to have tokens set.

```bash
git checkout -- $F
grep -c 'if true || (loopback && !input.behind_tunnel) {' $F  # expect 0
cargo nextest run -p vox-orchestrator-mcp bind_policy spawn_refuses_network_bind_without_tokens
git status --porcelain crates/ | wc -l                        # expect 0
```

Expected: tests pass; tree clean. Paste each step's FAIL and PASS lines into the PR description.

---

### Task 13: Final verification

- [ ] **Step 1: Crate suites**

```bash
cargo nextest run -p vox-orchestrator-mcp http_gateway
cargo nextest run -p vox-secrets
cargo nextest run -p vox-cli gateway_bind serve_network serve::tests
cargo check -p vox-orchestrator-d
```

Expected: all green.

- [ ] **Step 2: Gates**

```bash
cargo run -q -p vox-cli -- ci secrets-parity
cargo run -q -p vox-cli -- ci secret-env-guard
cargo run -q -p vox-cli -- ci command-compliance
cargo run -q -p vox-cli -- ci data-storage-guard
cargo run -q -p vox-cli -- ci crate-edges
cargo run -q -p vox-arch-check
git diff origin/main -- Cargo.lock | rg '^[+-](name|version) = '; echo "lock-packages-changed=$?"
cargo run -q -p vox-cli -- ci pre-push --complete
```

Expected: every gate exits 0; `lock-packages-changed=1` (no package added or bumped by this plan;
if plans 01–03 changed packages, compare against the commit before Task 1 instead of `origin/main`);
`crate-edges` reports no new workspace edge.

- [ ] **Step 3: Self-review the branch diff** (AGENTS.md §PR & Review Discipline): read every hunk
of `git diff <commit-before-task-1>..HEAD -- . ':(exclude)Cargo.lock' ':(exclude)contracts/secrets/*.json'`.

---

## Self-Review

**Spec coverage (§6, §8, §9):**

| Requirement | Task |
|---|---|
| Default bind 127.0.0.1 | Task 1 (`loopback_default_needs_no_tokens_or_tls`), Task 4 (default unchanged) |
| Non-loopback needs write + read token + TLS-or-tunnel; error names every missing item | Task 1, Task 4 |
| `VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED` rejected off loopback | Task 1 |
| Startup refuses; reported by `vox doctor` | Task 4 (gateway + daemon exit), Task 8 |
| Off loopback: read → public ∩ read-eligible; write → public + operator; internal never | Task 5 (`guard_tool` with peer-derived ctx); Task 7 tests |
| `GET /v1/profile` + OpenAPI | Task 7 |
| `exposure_denied` / `insufficient_role` / `tool_not_in_profile` / `subsystem_not_booted` over HTTP + WS | Task 5 (plan 02's `denial_error_json`, status from `ProfileDenial::http_status`) |
| §9.2 mutation-verified guards (internal over LAN; bind without tokens/TLS) | Task 12 |
| Env vars + `vox serve` flags | Tasks 2, 9 |
| Docs: running Vox as a server, threat model | Task 11 |

**Placeholder scan:** all code is complete. Names from plans 02/03 (`guard_tool`,
`denial_error_json`, `active_profile`, `boot_report_snapshot`, `serve::run`, `serve_child_env`,
`Cli::Serve`) were read from `02-profiles.md` / `03-subsystem-boot.md` and are re-checked in
Task 0. The daemon call site is located by `rg` in Task 4 Step 6 because plan 03 may move it.

**Type consistency:** `CallContext { role: CallerRole, loopback }`, `ProfileDenial::{code, http_status}`,
`Exposure`, `Subsystem::as_str`, `ActiveProfile::{ids, max_exposure, axis_surfaces}`, and
`BootReport::{booted, failed}` are used exactly as in `00-contracts.md`. `/v1/profile` field names
and order match the contract JSON.

**Deviations recorded:**
1. The new settings are `vox_secrets::SecretId`s, matching every other `VOX_MCP_HTTP_*` read in
   `spawn_http_gateway_if_enabled`, rather than `vox-config` (the contract's general rule). One
   resolution path for all gateway settings keeps `vox secrets` output and the gateway in agreement.
2. `--behind-tunnel` on a loopback bind is treated as network exposure: tokens are required and no
   caller is loopback. The contract table only says "operator attests a TLS-terminating tunnel";
   without this rule, `cloudflared` → 127.0.0.1 would hand `internal` tools to the internet.
3. `/v1/profile.tools` is the `/v1/tools` set (env allowlist ∩ `guard_tool`), not `authorize_tool`
   alone, so discovery never advertises a tool that `/v1/tools/call` would refuse. Plan 02 already
   defaults the allowlist to the profile's tools, so the two coincide unless an operator narrows
   `VOX_MCP_HTTP_ALLOWED_TOOLS`.
4. `--bind` takes an IP-literal `SocketAddr` (`localhost:3921` is rejected by clap); the env
   variable still accepts `localhost`.

## Risks

1. **Plans 02/03 are plans, not code yet.** If their final code renames a helper or field, Task 0
   catches it; only the call sites listed there change.
2. **Eval sandbox outage.** Task 4 makes `eval.voxlang.org`'s current config refuse to start.
   Per D2 there is no exemption: Task 10 wires the read token and the operator must add the Coolify secret before deploy; merging Task 4 without Task 10 (or deploying without the secret) breaks the
   docs playground on its next deploy.
3. **Origin guard vs remote clients.** `origin_guard::is_origin_allowed` rejects a WebSocket
   upgrade without an `Origin` header, and any browser `Origin` that is not loopback. LAN browsers
   (the `/v1/mobile` page's POSTs) and a Rust `GatewayClient` WS connection will be refused until
   plan 06 sends an `Origin` or the guard is relaxed for bearer-authenticated requests. Out of
   scope here; flagged for plan 06.
4. **Same-host proxy without `--behind-tunnel`.** The gateway cannot detect it; every proxied
   caller looks loopback. Mitigated only by the docs warning.
5. **Dual TLS providers.** Any future `ServerConfig::builder()` call in this crate panics in
   unified builds; the module doc and Global Constraints say so. Provider additions remain a
   user-authorized ledger change (Task 3 Step 3 hard stop).
6. **`workspace-hack` drift.** Direct `rustls`/`tokio-rustls` edges may change hakari output;
   Task 3 Step 3 regenerates it when `cargo hakari` is installed, otherwise nightly flags it.
7. **TLS handshake flooding.** Each handshake has a 10 s timeout, but there is no per-IP limit, and
   rate limiting applies only after the handshake. Acceptable for a single-tenant LAN; a public
   deployment should sit behind a tunnel.
8. **Rate-limit identity behind a tunnel** collapses to `127.0.0.1` unless
   `VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR=1`; the how-to says so.
9. **WS read token.** The WS `auth` message accepts only the write token or the dashboard token;
   read-token WS clients must authenticate with the `Authorization` header on the upgrade.
   Unchanged by this plan.
