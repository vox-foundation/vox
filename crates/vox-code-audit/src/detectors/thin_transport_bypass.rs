//! Detector `vox/transport/thin-transport-bypass` (`T3-TOESTUB-ENF-01`):
//! Flags raw RPC/client guard installs and thin-transport bypasses outside
//! allowlisted boundary files (t3code `no-rpc-permission-bypass` analog).

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

/// Paths (suffix match) where raw client / permission-guard installation is allowed.
const ALLOWLIST_SUFFIXES: &[&str] = &[
    "/remote/gate.rs",
    "/remote/calls.rs",
    "/remote/mod.rs",
    "/rpc/permission.rs",
    "/rpc/client.rs",
    "/rpc/mod.rs",
    "/commands/mod.rs",
    "t3Wave3Rpc.ts",
    "/t3_wave3_rpc.ts",
    "/t3_wave3_rpc.rs",
];

fn is_allowlisted(path: &str) -> bool {
    let p = path.replace('\\', "/");
    ALLOWLIST_SUFFIXES
        .iter()
        .any(|s| p.ends_with(s) || p.contains(s))
}

pub struct ThinTransportBypassDetector {
    supported_langs: Vec<Language>,
}

impl Default for ThinTransportBypassDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ThinTransportBypassDetector {
    pub fn new() -> Self {
        Self {
            supported_langs: vec![Language::Rust, Language::TypeScript],
        }
    }
}

impl DetectionRule for ThinTransportBypassDetector {
    fn id(&self) -> &'static str {
        "vox/transport/thin-transport-bypass"
    }

    fn name(&self) -> &'static str {
        "Thin Transport / Raw RPC Permission Bypass Detector"
    }

    fn description(&self) -> &'static str {
        "Flags raw RPC client access and permission-guard installation outside allowlisted boundary files."
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn languages(&self) -> &[Language] {
        &self.supported_langs
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        let path_str = file.path.to_string_lossy().replace('\\', "/");
        if is_allowlisted(&path_str) {
            return findings;
        }
        let in_scope = path_str.contains("vox-gui")
            || path_str.contains("vox-orchestrator-mcp")
            || path_str.contains("/rpc/")
            || path_str.contains("t3_wave3")
            || path_str.contains("t3Wave3");
        if !in_scope {
            return findings;
        }

        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.contains("toestub-ignore")
            {
                continue;
            }
            let hit = trimmed.contains("RpcPermissionGuard")
                || trimmed.contains("install_permission_guard")
                || trimmed.contains("installPermissionGuard")
                || (trimmed.contains(".client")
                    && (trimmed.contains("rpc")
                        || trimmed.contains("Rpc")
                        || trimmed.contains("RPC")))
                || trimmed.contains("bypass_permission")
                || trimmed.contains("bypassPermission")
                || trimmed.contains("raw_rpc_client")
                || trimmed.contains("rawRpcClient");
            if hit {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::TRANSPORT_THIN_BYPASS.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: format!(
                        "raw RPC/client or permission-guard install outside allowlisted boundary ({path_str}:{line_num})"
                    ),
                    suggestion: Some(
                        "Move guard installation / raw client access into remote/gate.rs or rpc/*; or add // toestub-ignore(vox/transport/thin-transport-bypass) — <reason>."
                            .to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: Some(
                        "t3code no-rpc-permission-bypass: thin transport must stay behind shared permission-aware boundaries."
                            .to_string(),
                    ),
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::Medium),
                    evidence: None,
                });
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flags_bypass_outside_allowlist() {
        let d = ThinTransportBypassDetector::new();
        let f = SourceFile::new(
            PathBuf::from("crates/vox-gui/src/commands/chat.rs"),
            "let _ = raw_rpc_client.connect();\n".to_string(),
        );
        assert!(!d.detect(&f, None).is_empty());
    }

    #[test]
    fn allowlists_remote_gate() {
        let d = ThinTransportBypassDetector::new();
        let f = SourceFile::new(
            PathBuf::from("crates/vox-gui/src/remote/gate.rs"),
            "fn install_permission_guard() {}\n".to_string(),
        );
        assert!(d.detect(&f, None).is_empty());
    }
}
