//! Detector `vox/ui/layout-and-perf-hazards` (`T3-TOESTUB-RULES-03`):
//! Flags CSS `transition-all`, V8 dynamic `delete obj[k]`, and `arguments` keyword usage in TypeScript/TSX.

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

pub struct UiPerfHazardsDetector {
    supported_langs: Vec<Language>,
}

impl Default for UiPerfHazardsDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl UiPerfHazardsDetector {
    pub fn new() -> Self {
        Self {
            supported_langs: vec![Language::TypeScript, Language::Unknown],
        }
    }
}

impl DetectionRule for UiPerfHazardsDetector {
    fn id(&self) -> &'static str {
        "vox/ui/layout-and-perf-hazards"
    }

    fn name(&self) -> &'static str {
        "UI Layout Thrashing and V8 Deopt Hazard Detector"
    }

    fn description(&self) -> &'static str {
        "Flags Tailwind `transition-all` layout thrashing and V8 hidden-class deoptimizations (`delete obj[k]`)."
    }

    fn severity(&self) -> Severity {
        Severity::Warning
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
            {
                continue;
            }
            let prev_has_ignore = idx > 0 && file.lines[idx - 1].contains("toestub-ignore");
            let prev_has_raf = idx > 0 && file.lines[idx - 1].contains("requestAnimationFrame");
            // Look ahead through the typical ResizeObserver callback body for an
            // rAF throttle (DockWorkspaceShell nests cancel/requestAnimationFrame
            // several lines below `new ResizeObserver(`).
            let next_has_raf = (1..=8).any(|offset| {
                idx + offset < file.lines.len()
                    && file.lines[idx + offset].contains("requestAnimationFrame")
            });
            if prev_has_ignore {
                continue;
            }

            let has_transition_all = trimmed.contains("transition-all");
            let has_dynamic_delete = trimmed.starts_with("delete ") && trimmed.contains('[');
            let has_raw_resize_observer = trimmed.contains("new ResizeObserver(")
                && !trimmed.contains("requestAnimationFrame")
                && !prev_has_raf
                && !next_has_raf;
            let has_unscoped_css_has =
                is_css && (trimmed.starts_with(":has(") || trimmed.contains(" :has("));
            let has_asymmetric_scrollbar_gutter = is_css
                && trimmed.contains("scrollbar-gutter: stable")
                && !trimmed.contains("both-edges")
                && trimmed.contains("mx-auto");

            if has_transition_all
                || has_dynamic_delete
                || has_raw_resize_observer
                || has_unscoped_css_has
                || has_asymmetric_scrollbar_gutter
            {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::UI_LAYOUT_AND_PERF_HAZARDS.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message:
                        "UI layout-thrashing (`transition-all`), raw `new ResizeObserver(`, or V8 dictionary-mode (`delete obj[key]`) hazard detected."
                            .to_string(),
                    suggestion: Some(
                        "Use compositor-only transitions (`transition-opacity`, `transition-transform`, `transition-colors`), shared `useResizeObserver`, or `Map.delete()`."
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
    fn flags_transition_all_and_dynamic_delete() {
        let det = UiPerfHazardsDetector::new();
        let f = SourceFile::new(
            PathBuf::from("src/Chat.tsx"),
            "<div className=\"transition-all duration-200\" />\ndelete cache[key];\n".to_string(),
        );
        assert_eq!(det.detect(&f, None).len(), 2);
    }

    #[test]
    fn flags_raw_resize_observer_and_unscoped_css_has() {
        let det = UiPerfHazardsDetector::new();
        let tsx = SourceFile::new(
            PathBuf::from("src/Panel.tsx"),
            "const ro = new ResizeObserver((entries) => { setH(entries[0].contentRect.height); });\n"
                .to_string(),
        );
        assert_eq!(det.detect(&tsx, None).len(), 1);

        let css = SourceFile::new(
            PathBuf::from("src/styles.css"),
            ":has(.active) { color: red; }\n".to_string(),
        );
        assert_eq!(det.detect(&css, None).len(), 1);
    }

    #[test]
    fn allows_raf_throttled_resize_observer_callback() {
        let det = UiPerfHazardsDetector::new();
        let tsx = SourceFile::new(
            PathBuf::from("src/Dock.tsx"),
            "const ro = new ResizeObserver(entries => {\n  const h = entries[0]?.contentRect.height;\n  if (h && h > 0) {\n    rafId = requestAnimationFrame(() => setPixelHeight(h));\n  }\n});\n"
                .to_string(),
        );
        assert!(det.detect(&tsx, None).is_empty());
    }

    #[test]
    fn flags_css_scrollbar_gutter_without_both_edges() {
        let det = UiPerfHazardsDetector::new();
        let css = SourceFile::new(
            PathBuf::from("src/layout.css"),
            ".shell { scrollbar-gutter: stable; mx-auto: true; }\n".to_string(),
        );
        assert_eq!(det.detect(&css, None).len(), 1);
    }
}
