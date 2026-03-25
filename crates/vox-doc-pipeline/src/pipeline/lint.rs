//! Markdown lint passes for docs under `docs/src/`.

use std::fs;
use std::path::Path;

use super::types::{LintError, LintKind};

/// Recursively walk `dir` and collect lint errors for every `.md` file.
pub(crate) fn collect_lint_errors(dir: &Path, errors: &mut Vec<LintError>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_lint_errors(&path, errors);
            } else if path.extension().map(|e| e == "md").unwrap_or(false) {
                let rel = path.to_str().unwrap_or_default();
                if rel.contains("SUMMARY.md") {
                    continue;
                }
                let content = fs::read_to_string(&path).unwrap_or_default();
                lint_file(&path, &content, errors);
            }
        }
    }
}

/// Run all lint checks on a single file's content.
fn lint_file(path: &Path, content: &str, errors: &mut Vec<LintError>) {
    let mut fence_open = false;
    let mut fence_start_line = 0_usize;

    if !content.trim_start().starts_with("---") {
        errors.push(LintError {
            file: path.to_owned(),
            line: 1,
            kind: LintKind::MissingFrontmatter,
        });
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

        if backtick_count >= 1
            && trimmed
                .chars()
                .all(|c| c == '`' || c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
            && (trimmed == "`".repeat(backtick_count)
                || trimmed.starts_with(&"`".repeat(backtick_count)))
        {
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
                }
            }
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
