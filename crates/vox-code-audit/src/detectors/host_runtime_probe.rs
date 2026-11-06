//! Detector `vox/host/runtime-probe` — direct OS/ARCH probes outside allowlisted host crates.

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

const ALLOW_SUFFIXES: &[&str] = &[
    "/host/",
    "/platform/",
    "build.rs",
    "/vox-cli-ci/",
    "/vox-doctor/",
    "/scripts/",
];

pub struct HostRuntimeProbeDetector {
    langs: Vec<Language>,
}

impl Default for HostRuntimeProbeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl HostRuntimeProbeDetector {
    pub fn new() -> Self {
        Self {
            langs: vec![Language::Rust, Language::TypeScript],
        }
    }
}

fn allowlisted(path: &str) -> bool {
    let p = path.replace('\\', "/");
    ALLOW_SUFFIXES.iter().any(|s| p.contains(s))
}

impl DetectionRule for HostRuntimeProbeDetector {
    fn id(&self) -> &'static str {
        "vox/host/runtime-probe"
    }
    fn name(&self) -> &'static str {
        "Host Runtime Platform/Arch Probe Detector"
    }
    fn description(&self) -> &'static str {
        "Flags std::env::consts::{OS,ARCH} and process.platform/arch outside host boundaries."
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn languages(&self) -> &[Language] {
        &self.langs
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut out = Vec::new();
        let path = file.path.to_string_lossy().replace('\\', "/");
        if allowlisted(&path) {
            return out;
        }
        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            let hit = trimmed.contains("env::consts::OS")
                || trimmed.contains("env::consts::ARCH")
                || trimmed.contains("std::env::consts::OS")
                || trimmed.contains("std::env::consts::ARCH")
                || trimmed.contains("process.platform")
                || trimmed.contains("process.arch");
            if hit {
                out.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::HOST_RUNTIME_PROBE.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: "Direct host platform/arch probe outside allowlisted host boundary.".to_string(),
                    suggestion: Some(
                        "Route through a shared host/platform abstraction, or add toestub-ignore with reason.".to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: Some("t3code no-global-process-runtime.".to_string()),
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::Medium),
                    evidence: None,
                });
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flags_outside_allowlist() {
        let d = HostRuntimeProbeDetector::new();
        let f = SourceFile::new(
            PathBuf::from("crates/vox-gui/src/commands/chat.rs"),
            "let os = std::env::consts::OS;\n".to_string(),
        );
        assert!(!d.detect(&f, None).is_empty());
    }

    #[test]
    fn allowlists_build_rs() {
        let d = HostRuntimeProbeDetector::new();
        let f = SourceFile::new(
            PathBuf::from("crates/vox-gui/build.rs"),
            "let os = std::env::consts::OS;\n".to_string(),
        );
        assert!(d.detect(&f, None).is_empty());
    }
}
