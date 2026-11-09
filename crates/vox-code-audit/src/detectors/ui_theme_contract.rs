//! Detector `vox/ui/theme-contract-drift` (`ROMA-V66-THEME-CONTRACT-03`):
//! Enforces `contracts/gui/theme-contract.v1.json` across `crates/vox-gui/ui/src/`:
//! flags hardcoded legacy dark-brown hexes (`#0A0805`..`#2A2011`), dark-brown RGBA surfaces,
//! and banned non-Romanesque font literals (`Courier`, `Arial`, `Consolas`, `Menlo`, `Monaco`)
//! outside allowlisted theme definition files (`index.css`, `tokens.slate.generated.css`, `themeContract.ts`).

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

const BANNED_DARK_BROWN_HEXES: &[&str] = &[
    "#0a0805", "#0d0a07", "#0e0b08", "#100c08", "#120e0a", "#14100b", "#16120c", "#18120c",
    "#1a140e", "#1c160f", "#1e1710", "#201810", "#221a11", "#241c14", "#261d13", "#281f14",
    "#2a2011",
];

const ALLOWED_EMERALD_STATUS_HEXES: &[&str] = &[
    "#10b981", "#1b6b3a", "#1e7e46", "#156336", "#0d4524", "#15803d", "#064e3b",
];

const BANNED_DARK_RGBA_PREFIXES: &[&str] = &[
    "rgba(10, 8, 5",
    "rgba(10,8,5",
    "rgba(14, 11, 8",
    "rgba(14,11,8",
    "rgba(18, 14, 10",
    "rgba(18,14,10",
    "rgba(19, 16, 12",
    "rgba(19,16,12",
    "rgba(20, 16, 11",
    "rgba(20,16,11",
    "rgba(24, 18, 12",
    "rgba(24,18,12",
    "rgba(28, 22, 15",
    "rgba(28,22,15",
];

const BANNED_FONT_LITERALS: &[&str] = &[
    "courier new",
    "consolas",
    "menlo",
    "monaco",
    "'courier'",
    "\"courier\"",
    "courier,",
    "'arial'",
    "\"arial\"",
    "arial,",
];

fn extract_dark_tailwind_bg_hex(lower_line: &str) -> Option<String> {
    if lower_line.contains("swatch:") {
        return None;
    }
    for prefix in ["bg-[#", "from-[#", "via-[#", "to-[#"] {
        let mut search_from = 0;
        while let Some(rel_idx) = lower_line[search_from..].find(prefix) {
            let start = search_from + rel_idx + prefix.len() - 1; // points to '#'
            if lower_line.len() >= start + 7 {
                let candidate = &lower_line[start..start + 7];
                let bytes = candidate.as_bytes();
                let first_digit = bytes[1];
                let all_hex = bytes[1..7].iter().all(|b| b.is_ascii_hexdigit());
                if all_hex
                    && matches!(first_digit, b'0' | b'1' | b'2')
                    && !ALLOWED_EMERALD_STATUS_HEXES.contains(&candidate)
                {
                    return Some(candidate.to_string());
                }
            }
            search_from = start + 1;
        }
    }
    None
}

pub struct UiThemeContractDetector {
    supported_langs: Vec<Language>,
}

impl Default for UiThemeContractDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl UiThemeContractDetector {
    pub fn new() -> Self {
        Self {
            supported_langs: vec![Language::TypeScript, Language::Unknown],
        }
    }
}

impl DetectionRule for UiThemeContractDetector {
    fn id(&self) -> &'static str {
        "vox/ui/theme-contract-drift"
    }

    fn name(&self) -> &'static str {
        "GUI Theme Contract Drift Detector"
    }

    fn description(&self) -> &'static str {
        "Flags hardcoded legacy dark-brown hex colors (#0A0805..#2A2011) and banned non-Romanesque font literals in vox-gui UI source files."
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
        if !path_str.contains("vox-gui/ui/src/") {
            return findings;
        }
        if path_str.ends_with(".test.ts")
            || path_str.ends_with(".test.tsx")
            || path_str.ends_with(".spec.ts")
            || path_str.ends_with("src/index.css")
            || path_str.ends_with("src/styles/tokens.slate.generated.css")
            || path_str.ends_with("src/lib/themeContract.ts")
        {
            return findings;
        }

        let is_css = file.path.extension().and_then(|e| e.to_str()) == Some("css");
        if file.language == Language::Unknown && !is_css {
            return findings;
        }

        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("//")
                || trimmed.starts_with("/*")
                || trimmed.starts_with('*')
                || trimmed.contains("toestub-ignore")
                || trimmed.contains("theme-contract-ignore")
            {
                continue;
            }
            let lower = trimmed.to_ascii_lowercase();

            let matched_hex = BANNED_DARK_BROWN_HEXES
                .iter()
                .copied()
                .find(|h| lower.contains(h))
                .map(str::to_string)
                .or_else(|| extract_dark_tailwind_bg_hex(&lower));

            if let Some(hex) = matched_hex {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::UI_THEME_CONTRACT_DRIFT.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: format!(
                        "Hardcoded dark background color `{hex}` violates GUI Theme Contract (`contracts/gui/theme-contract.v1.json`)."
                    ),
                    suggestion: Some(
                        "Use semantic theme tokens (`bg-bg-base`, `bg-bg-surface`, `bg-bg-elevated`, `var(--color-bg-*)`) so all 4 theme profiles (`arcane`, `slate`, `high-contrast`, `glacier`) stay in sync."
                            .to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: None,
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::High),
                    evidence: None,
                });
            }

            if let Some(rgba) = BANNED_DARK_RGBA_PREFIXES
                .iter()
                .copied()
                .find(|r| lower.contains(r))
            {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::UI_THEME_CONTRACT_DRIFT.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: format!(
                        "Hardcoded dark RGBA surface `{rgba}...` violates GUI Theme Contract (`contracts/gui/theme-contract.v1.json`)."
                    ),
                    suggestion: Some(
                        "Use semantic overlay/surface tokens (`bg-bg-elevated`, `var(--color-overlay-*)`) instead of hardcoded dark RGBA values."
                            .to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: None,
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::High),
                    evidence: None,
                });
            }

            if let Some(font) = BANNED_FONT_LITERALS
                .iter()
                .copied()
                .find(|f| lower.contains(f))
            {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::UI_THEME_CONTRACT_DRIFT.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: format!(
                        "Banned non-Romanesque font literal `{font}` violates GUI Theme Contract (`contracts/gui/theme-contract.v1.json`)."
                    ),
                    suggestion: Some(
                        "Use semantic font variables (`var(--font-sans)`, `var(--font-serif)`, `var(--font-display)`, `var(--font-mono)`) so typography adapts automatically across themes."
                            .to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: None,
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::High),
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
    fn flags_hardcoded_dark_brown_hex_and_banned_font_in_gui_component() {
        let det = UiThemeContractDetector::new();
        assert_eq!(det.severity(), Severity::Error);
        let file = SourceFile::new(
            PathBuf::from("crates/vox-gui/ui/src/components/NewWidget.tsx"),
            "export const NewWidget = () => <div className=\"bg-[#16120C]\" style={{ fontFamily: 'Courier New' }} />;".into(),
        );
        let findings = det.detect(&file, None);
        assert_eq!(findings.len(), 2);
        assert!(findings[0].message.contains("#16120c"));
        assert!(findings[1].message.contains("courier new"));
    }

    #[test]
    fn flags_arbitrary_dark_hex_and_inline_rgba_and_unquoted_font() {
        let det = UiThemeContractDetector::new();
        let file = SourceFile::new(
            PathBuf::from("crates/vox-gui/ui/src/components/AdversarialWidget.tsx"),
            "export const AdversarialWidget = () => <div className=\"bg-[#251C0E]\" style={{ background: 'rgba(19, 16, 12, 0.97)', fontFamily: 'Courier, monospace' }} />;".into(),
        );
        let findings = det.detect(&file, None);
        assert_eq!(findings.len(), 3);
        assert!(findings[0].message.contains("#251c0e"));
        assert!(findings[1].message.contains("rgba(19, 16, 12"));
        assert!(findings[2].message.contains("courier,"));
    }

    #[test]
    fn allows_semantic_tokens_and_ignores_allowlisted_files() {
        let det = UiThemeContractDetector::new();
        let clean = SourceFile::new(
            PathBuf::from("crates/vox-gui/ui/src/components/CleanWidget.tsx"),
            "export const CleanWidget = () => <div className=\"bg-bg-surface text-text-primary font-serif\" />;".into(),
        );
        assert!(det.detect(&clean, None).is_empty());

        let allowlisted = SourceFile::new(
            PathBuf::from("crates/vox-gui/ui/src/lib/themeContract.ts"),
            "export const HEX = '#16120c';".into(),
        );
        assert!(det.detect(&allowlisted, None).is_empty());
    }
}
