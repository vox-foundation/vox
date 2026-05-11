//! Markdown lint passes for docs under `docs/src/`.
//!
// SCIENTIA — contracts/scientia/*.schema.json
// Stable serde names live in vox-research-events::schema_types; typify exhaust in schema_types::generated.
// Regenerate: cargo run -p vox-scientia-jsonschema-codegen
// Hand-maintained name map (lint reminder): discovery-signal → DiscoverySignal; finding-candidate.v1 → FindingCandidateV1;
// novelty-evidence-bundle.v1 → NoveltyEvidenceBundle; evidence-pack.v1 → EvidencePackV1; worthiness-signals.v2 → WorthinessSignalsV2.

use std::fs;
use std::path::Path;

use super::types::{LintError, LintKind};

const VALID_CATEGORIES: &[&str] = &[
    "getting-started",
    "journey",
    "journeys",
    "tutorial",
    "tutorials",
    "how-to",
    "ref",
    "reference",
    "lang-ref",
    "language-reference",
    "api-keyword",
    "api-decorator",
    "api-crate",
    "example",
    "explanation",
    "adr",
    "architecture",
    "ssot",
    "ci",
    "quality",
    "contributor",
    "contributors",
    "operations",
];

const VALID_STATUS: &[&str] = &[
    "approved",
    "current",
    "experimental",
    "legacy",
    "research",
    "roadmap",
    "deprecated",
];

/// Recursively walk `dir` and collect lint errors for every `.md` file.
pub(crate) fn collect_lint_errors(dir: &Path, errors: &mut Vec<LintError>) {
    collect_lint_errors_target(dir, errors);
}

/// Collect lint errors from either a markdown file or a directory tree.
pub(crate) fn collect_lint_errors_target(target: &Path, errors: &mut Vec<LintError>) {
    if target.is_file() {
        if target.extension().map(|e| e == "md").unwrap_or(false) {
            let rel = target.to_str().unwrap_or_default();
            if rel.contains("SUMMARY.md") {
                return;
            }
            let content =
                vox_bounded_fs::read_utf8_path_capped(target).unwrap_or_else(|_| String::new());
            lint_file(target, &content, errors);
            crate::pipeline::doctest::check_doctests(target, &content, errors);
        }
        return;
    }

    if !target.is_dir() {
        return;
    }

    if let Ok(entries) = fs::read_dir(target) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_lint_errors_target(&path, errors);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let rel = path.to_str().unwrap_or_default();
                if rel.contains("SUMMARY.md") {
                    continue;
                }
                let content =
                    vox_bounded_fs::read_utf8_path_capped(&path).unwrap_or_else(|_| String::new());
                lint_file(&path, &content, errors);
                crate::pipeline::doctest::check_doctests(&path, &content, errors);
            }
        }
    }
}

/// Run all lint checks on a single file's content.
pub(crate) fn lint_file(path: &Path, content: &str, errors: &mut Vec<LintError>) {
    let mut fence_open = false;
    let mut fence_start_line = 0_usize;
    let mut fence_is_vox = false;
    if !content.trim_start().starts_with("---") {
        errors.push(LintError {
            file: path.to_owned(),
            line: 1,
            kind: LintKind::MissingFrontmatter,
        });
    } else {
        lint_frontmatter(path, content, errors);
    }

    if content.contains("Official documentation for ")
        && content.contains("in the Vox programming language ecosystem.")
    {
        errors.push(LintError {
            file: path.to_owned(),
            line: 0,
            kind: LintKind::GenericDescription,
        });
    }

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();
        let backtick_count = trimmed.chars().take_while(|&c| c == '`').count();

        // A code fence marker is N backticks followed by an optional language tag (no spaces).
        // Inline code like `identifier` or `identifier` is prose text is NOT a fence.
        let after_backticks = &trimmed[backtick_count..];
        let rest_is_fence_like = after_backticks.trim().is_empty()
            || after_backticks
                .trim()
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_');

        if backtick_count >= 1 && rest_is_fence_like {
            if (1..3).contains(&backtick_count) {
                if !fence_open {
                    errors.push(LintError {
                        file: path.to_owned(),
                        line: line_no,
                        kind: LintKind::ShortCodeFence {
                            backticks: backtick_count,
                            at_line: line_no,
                        },
                    });
                }
            } else if backtick_count >= 3 {
                if fence_open {
                    fence_open = false;
                } else {
                    fence_open = true;
                    fence_start_line = line_no;
                    let lang = trimmed[backtick_count..].trim();
                    fence_is_vox = lang == "vox";
                    if lang.is_empty() {
                        errors.push(LintError {
                            file: path.to_owned(),
                            line: line_no,
                            kind: LintKind::UnlabeledCodeFence { at_line: line_no },
                        });
                    }
                }
            }
        } else if fence_open && fence_is_vox {
        }

        // Also check for naked includes everywhere
        if !fence_open && trimmed.starts_with("{{#include ") {
            // Naked include check handles parsing anchors
            check_include_anchor(path, trimmed, line_no, errors);
        }
        // Fenced includes
        if fence_open && trimmed.starts_with("{{#include ") {
            check_include_anchor(path, trimmed, line_no, errors);
        }
    }

    if fence_open {
        errors.push(LintError {
            file: path.to_owned(),
            line: fence_start_line,
            kind: LintKind::UnclosedCodeFence,
        });
    }
}

fn lint_frontmatter(path: &Path, content: &str, errors: &mut Vec<LintError>) {
    let Some(after_dash) = content.strip_prefix("---") else {
        return;
    };
    let Some(end) = after_dash.find("---") else {
        return;
    };
    let yaml = &after_dash[..end];
    let mut saw_category = false;
    let mut status: Option<String> = None;
    let mut training_eligible = false;
    let mut saw_training_rationale = false;

    for (idx, raw_line) in yaml.lines().enumerate() {
        let line_no = idx + 2;
        let line = raw_line.trim();
        if let Some(value) = line.strip_prefix("category:") {
            saw_category = true;
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            if !VALID_CATEGORIES.contains(&value) {
                errors.push(LintError {
                    file: path.to_owned(),
                    line: line_no,
                    kind: LintKind::UnknownCategory {
                        value: value.to_string(),
                    },
                });
            }
        } else if let Some(value) = line.strip_prefix("status:") {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            status = Some(value.to_string());
            if !VALID_STATUS.contains(&value) {
                errors.push(LintError {
                    file: path.to_owned(),
                    line: line_no,
                    kind: LintKind::UnknownStatus {
                        value: value.to_string(),
                    },
                });
            }
        } else if let Some(value) = line.strip_prefix("schema_type:") {
            let val = value.trim().trim_matches(|c| c == '"' || c == '\'');
            const VALID_SCHEMA_TYPES: &[&str] =
                &["HowTo", "FAQPage", "TechArticle", "SoftwareSourceCode"];
            if !VALID_SCHEMA_TYPES.contains(&val) {
                errors.push(LintError {
                    file: path.to_owned(),
                    line: line_no,
                    kind: LintKind::UnknownSchemaType {
                        value: val.to_string(),
                    },
                });
            }
        } else if let Some(value) = line.strip_prefix("training_eligible:") {
            let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
            if value == "true" {
                training_eligible = true;
            }
        } else if line.starts_with("training_rationale:") {
            saw_training_rationale = true;
        }
    }

    if !saw_category {
        errors.push(LintError {
            file: path.to_owned(),
            line: 1,
            kind: LintKind::MissingCategory,
        });
    }

    if training_eligible
        && !saw_training_rationale
        && let Some(st) = status
        && (st == "research" || st == "roadmap")
    {
        errors.push(LintError {
            file: path.to_owned(),
            line: 1,
            kind: LintKind::MissingTrainingRationale,
        });
    }
}

fn check_include_anchor(path: &Path, line: &str, line_no: usize, errors: &mut Vec<LintError>) {
    let Some(start) = line.find("{{#include ") else {
        return;
    };
    let Some(end) = line[start..].find("}}") else {
        return;
    };
    let include_body = &line[start + 11..start + end].trim();

    let parts: Vec<&str> = include_body.split(':').collect();
    let target_file = parts[0];
    let anchor = if parts.len() > 1 {
        Some(parts[1])
    } else {
        None
    };

    // Resolve target path relative to current file's dir
    let mut target_path = path.parent().unwrap_or(Path::new("")).to_path_buf();
    target_path.push(target_file);

    // Normalize path to some degree for reading, assuming docs/src as root of md files
    // But since target_file is usually `../../../examples/...` we just read it relative to cwd
    let content_res = vox_bounded_fs::read_utf8_path_capped(&target_path);
    if content_res.is_err() {
        errors.push(LintError {
            file: path.to_owned(),
            line: line_no,
            kind: LintKind::BrokenIncludeFile {
                file: target_file.to_string(),
            },
        });
        return;
    }
    if let Ok(content) = content_res {
        if let Some(anchor_name) = anchor {
            // Looking for `// ANCHOR: anchor_name`
            let needle = format!("ANCHOR: {}", anchor_name);
            if !content.contains(&needle) {
                errors.push(LintError {
                    file: path.to_owned(),
                    line: line_no,
                    kind: LintKind::BrokenIncludeAnchor {
                        file: target_file.to_string(),
                        anchor: anchor_name.to_string(),
                    },
                });
            }
        } else {
            // Whole file include. Warn if it has `// ---` at the top
            if content.starts_with("// ---") {
                errors.push(LintError {
                    file: path.to_owned(),
                    line: line_no,
                    kind: LintKind::WholeFileIncludeHasTrainingHeader {
                        file: target_file.to_string(),
                    },
                });
            }
        }
    }
}
