//! MCP tool access policy matrix (`T3-HANDOFF-MCP-02`), MCP App UI `16 KiB` model context + `20 KiB`
//! DOM snapshot shedding (`T3-HANDOFF-MCP-03`), and stateless BLAKE3 `client_id` + DPoP replay defense (`T3-HANDOFF-MCP-04`).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Maximum byte size for an MCP App UI model context update (`16 KiB = 16_384` bytes, `T3-HANDOFF-MCP-03`).
pub const MAX_MCP_APP_MODEL_CONTEXT_BYTES: usize = 16_384;
/// Maximum byte size for an interactive DOM snapshot before shedding (`20 KiB = 20_480` bytes, `T3-HANDOFF-MCP-03`).
pub const MAX_DOM_SNAPSHOT_BYTES: usize = 20_480;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAccessMode {
    ReadOnly,
    WorkspaceWrite,
    FullAccess,
}

#[must_use]
pub fn runtime_mode_rank(mode: RuntimeAccessMode) -> u8 {
    match mode {
        RuntimeAccessMode::ReadOnly => 0,
        RuntimeAccessMode::WorkspaceWrite => 1,
        RuntimeAccessMode::FullAccess => 2,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionAccessMode {
    Chat,
    Plan,
    Agent,
}

#[must_use]
pub fn interaction_mode_rank(mode: InteractionAccessMode) -> u8 {
    match mode {
        InteractionAccessMode::Chat => 0,
        InteractionAccessMode::Plan => 1,
        InteractionAccessMode::Agent => 2,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpToolAccessRequirement {
    pub min_runtime_mode: RuntimeAccessMode,
    pub min_interaction_mode: InteractionAccessMode,
    pub required_capability: Option<String>,
    pub starts_threads: bool,
}

impl McpToolAccessRequirement {
    #[must_use]
    pub fn reads() -> Self {
        Self {
            min_runtime_mode: RuntimeAccessMode::ReadOnly,
            min_interaction_mode: InteractionAccessMode::Chat,
            required_capability: None,
            starts_threads: false,
        }
    }

    #[must_use]
    pub fn writes() -> Self {
        Self {
            min_runtime_mode: RuntimeAccessMode::WorkspaceWrite,
            min_interaction_mode: InteractionAccessMode::Agent,
            required_capability: None,
            starts_threads: false,
        }
    }

    #[must_use]
    pub fn writes_with_capability(capability: impl Into<String>) -> Self {
        Self {
            min_runtime_mode: RuntimeAccessMode::WorkspaceWrite,
            min_interaction_mode: InteractionAccessMode::Agent,
            required_capability: Some(capability.into()),
            starts_threads: false,
        }
    }

    #[must_use]
    pub fn starts_threads() -> Self {
        Self {
            min_runtime_mode: RuntimeAccessMode::FullAccess,
            min_interaction_mode: InteractionAccessMode::Agent,
            required_capability: Some("spawn_threads".to_string()),
            starts_threads: true,
        }
    }

    pub fn authorize(
        &self,
        runtime_mode: RuntimeAccessMode,
        interaction_mode: InteractionAccessMode,
        granted_capabilities: &[String],
    ) -> Result<(), String> {
        if runtime_mode_rank(runtime_mode) < runtime_mode_rank(self.min_runtime_mode) {
            return Err(format!(
                "insufficient runtime_mode {:?} (requires {:?})",
                runtime_mode, self.min_runtime_mode
            ));
        }
        if interaction_mode_rank(interaction_mode)
            < interaction_mode_rank(self.min_interaction_mode)
        {
            return Err(format!(
                "insufficient interaction_mode {:?} (requires {:?})",
                interaction_mode, self.min_interaction_mode
            ));
        }
        if let Some(cap) = &self.required_capability {
            if !granted_capabilities.iter().any(|c| c == cap) {
                return Err(format!("missing required capability '{cap}'"));
            }
        }
        Ok(())
    }
}

/// Enforces the `16 KiB` MCP App UI model context ceiling (`T3-HANDOFF-MCP-03`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpAppModelContext {
    pub app_id: String,
    pub context_json: String,
}

impl McpAppModelContext {
    pub fn new_checked(app_id: impl Into<String>, context_json: String) -> Result<Self, String> {
        if context_json.len() > MAX_MCP_APP_MODEL_CONTEXT_BYTES {
            return Err(format!(
                "MCP App model context exceeds 16 KiB limit ({} > {})",
                context_json.len(),
                MAX_MCP_APP_MODEL_CONTEXT_BYTES
            ));
        }
        Ok(Self {
            app_id: app_id.into(),
            context_json,
        })
    }
}

/// Progressive 3-pass DOM snapshot shedding to `<= 20 KiB` (`T3-HANDOFF-MCP-03`).
#[must_use]
pub fn bound_snapshot_metadata(raw_snapshot: &str) -> String {
    if raw_snapshot.len() <= MAX_DOM_SNAPSHOT_BYTES {
        return raw_snapshot.to_string();
    }
    // Pass 1: drop lines containing data URIs or inline style blobs
    let pass1: String = raw_snapshot
        .lines()
        .filter(|l| !l.contains("data:image/") && !l.contains("style=\""))
        .collect::<Vec<_>>()
        .join("\n");
    if pass1.len() <= MAX_DOM_SNAPSHOT_BYTES {
        return pass1;
    }
    // Pass 2: trim each line to 160 chars on valid UTF-8 boundary
    let pass2: String = pass1
        .lines()
        .map(|l| {
            if l.len() > 160 {
                let cut = l.floor_char_boundary(160);
                &l[..cut]
            } else {
                l
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if pass2.len() <= MAX_DOM_SNAPSHOT_BYTES {
        return pass2;
    }
    // Pass 3: hard byte truncation on valid UTF-8 boundary with marker
    let safe_cut = pass2.floor_char_boundary(MAX_DOM_SNAPSHOT_BYTES - 24);
    let mut out = pass2[..safe_cut].to_string();
    out.push_str("\n...[dom_shed_20kib]");
    out
}

/// Stateless BLAKE3-MAC client registration token (`vox_<hex_payload>.<hex_mac>`, `T3-HANDOFF-MCP-04`).
#[must_use]
pub fn issue_stateless_client_id(signing_key: &[u8; 32], redirect_uri: &str) -> String {
    let payload_hex = vox_crypto::hex_encode(redirect_uri.as_bytes());
    let mac = vox_crypto::keyed_hash(signing_key, payload_hex.as_bytes());
    let mac_hex = vox_crypto::hex_encode(&mac[..16]);
    format!("vox_{payload_hex}.{mac_hex}")
}

/// Verifies a stateless `client_id` in constant time (`T3-HANDOFF-MCP-04`).
#[must_use]
pub fn verify_stateless_client_id(
    signing_key: &[u8; 32],
    client_id: &str,
    expected_redirect_uri: &str,
) -> bool {
    let expected = issue_stateless_client_id(signing_key, expected_redirect_uri);
    if expected.len() != client_id.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in expected.bytes().zip(client_id.bytes()) {
        diff |= a ^ b;
    }
    diff == 0
}

/// Bounded DPoP `jti` replay cache with TTL eviction (`T3-HANDOFF-MCP-04`).
#[derive(Debug, Clone, Default)]
pub struct DpopReplayCache {
    seen_jti_expiry_ms: HashMap<String, u64>,
}

impl DpopReplayCache {
    pub fn check_and_record_jti(&mut self, jti: &str, now_ms: u64, ttl_ms: u64) -> bool {
        self.seen_jti_expiry_ms.retain(|_, &mut exp| exp > now_ms);
        if self.seen_jti_expiry_ms.contains_key(jti) {
            return false;
        }
        self.seen_jti_expiry_ms
            .insert(jti.to_string(), now_ms.saturating_add(ttl_ms));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_tool_access_matrix_enforces_runtime_and_interaction_modes() {
        let write_req = McpToolAccessRequirement::writes_with_capability("fs_write");
        assert!(
            write_req
                .authorize(
                    RuntimeAccessMode::ReadOnly,
                    InteractionAccessMode::Agent,
                    &["fs_write".into()]
                )
                .is_err()
        );
        assert!(
            write_req
                .authorize(
                    RuntimeAccessMode::WorkspaceWrite,
                    InteractionAccessMode::Plan,
                    &["fs_write".into()]
                )
                .is_err()
        );
        assert!(
            write_req
                .authorize(
                    RuntimeAccessMode::WorkspaceWrite,
                    InteractionAccessMode::Agent,
                    &["fs_write".into()]
                )
                .is_ok()
        );
    }

    #[test]
    fn mcp_app_context_and_dom_shedding_respect_byte_budgets() {
        assert!(McpAppModelContext::new_checked("app1", "x".repeat(16_384)).is_ok());
        assert!(McpAppModelContext::new_checked("app1", "x".repeat(16_385)).is_err());

        let huge_dom = "<div>".repeat(10_000);
        let shed = bound_snapshot_metadata(&huge_dom);
        assert!(shed.len() <= MAX_DOM_SNAPSHOT_BYTES);
    }

    #[test]
    fn stateless_client_id_and_dpop_replay_cache_verify_correctly() {
        let key = [9u8; 32];
        let cid = issue_stateless_client_id(&key, "http://127.0.0.1:4123/cb");
        assert!(verify_stateless_client_id(
            &key,
            &cid,
            "http://127.0.0.1:4123/cb"
        ));
        assert!(!verify_stateless_client_id(
            &key,
            &cid,
            "http://evil.example/cb"
        ));

        let mut dpop = DpopReplayCache::default();
        assert!(dpop.check_and_record_jti("jti-1", 1_000, 60_000));
        assert!(!dpop.check_and_record_jti("jti-1", 2_000, 60_000));
        assert!(dpop.check_and_record_jti("jti-1", 65_000, 60_000));
    }
}
