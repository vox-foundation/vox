use std::path::Path;
use vox_compiler::pipeline::check_file;

use crate::pipeline::types::{LintError, LintKind};

#[derive(Debug, Clone)]
struct VoxFenceBlock {
    start_line: usize,
    source: String,
    had_skip_directive: bool,
}

pub fn check_doctests(path: &Path, content: &str, errors: &mut Vec<LintError>) {
    let path_str = path.to_string_lossy().replace('\\', "/");
    let is_archived = path_str.contains("/archive/");

    let mut in_fence = false;
    let mut fence_backticks = 0usize;
    let mut is_vox = false;
    let mut block_start_line = 1usize;
    let mut current_block = String::new();
    let mut had_skip = false;
    let mut blocks: Vec<VoxFenceBlock> = Vec::new();

    for (idx, line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            let count = trimmed.chars().take_while(|&c| c == '`').count();
            if in_fence {
                if count >= fence_backticks && trimmed[count..].trim().is_empty() {
                    in_fence = false;
                    if is_vox {
                        blocks.push(VoxFenceBlock {
                            start_line: block_start_line,
                            source: std::mem::take(&mut current_block),
                            had_skip_directive: had_skip,
                        });
                    }
                } else if is_vox {
                    current_block.push_str(line);
                    current_block.push('\n');
                }
            } else {
                in_fence = true;
                fence_backticks = count;
                let lang = trimmed[count..].trim();
                // Only ```vox is Vox source. Other fences (tsx, rust, text, …) are not compiled as Vox.
                is_vox = lang == "vox";
                block_start_line = line_no;
                current_block.clear();
                had_skip = false;
            }
        } else if in_fence && is_vox {
            if trimmed.contains("vox:skip") || trimmed.contains("Skip-Test") {
                had_skip = true;
            }
            if !trimmed.contains("{{#include") {
                current_block.push_str(line);
                current_block.push('\n');
            }
        }
    }

    if blocks.is_empty() || is_archived {
        return;
    }

    let mut found_skip = false;
    for block in &blocks {
        if block.had_skip_directive {
            found_skip = true;
            errors.push(LintError {
                file: path.to_owned(),
                line: block.start_line,
                kind: LintKind::DocTestFailed {
                    msg: format!(
                        "DocTest error in {} (line {}): `vox:skip` is prohibited; all ```vox blocks must be valid, current `.vox` code that compiles cleanly.",
                        path.display(),
                        block.start_line
                    ),
                },
            });
        }
    }
    if found_skip {
        return;
    }

    let combined: String = blocks
        .iter()
        .filter(|b| !b.source.trim().is_empty())
        .map(|b| b.source.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if combined.trim().is_empty() {
        return;
    }

    let combined_diags = check_file(&combined, &path_str);
    if combined_diags.is_empty() {
        return;
    }

    // Fallback for reference/guide pages with multiple independent examples that may redefine `main`, `Task`, etc.:
    // verify each non-empty `vox` fence compiles cleanly on its own (or with cumulative prior blocks).
    let mut cumulative = String::new();
    let mut per_block_errors = Vec::new();
    for block in &blocks {
        if block.source.trim().is_empty() {
            continue;
        }
        let standalone_diags = check_file(&block.source, &path_str);
        if standalone_diags.is_empty() {
            cumulative.push_str(&block.source);
            cumulative.push('\n');
            continue;
        }
        let mut with_prior = cumulative.clone();
        with_prior.push_str(&block.source);
        let prior_diags = check_file(&with_prior, &path_str);
        if prior_diags.is_empty() {
            cumulative = with_prior;
            cumulative.push('\n');
            continue;
        }
        for diag in standalone_diags {
            per_block_errors.push(format!(
                "  - [block at line {}, {:?}] {} at line {}\n",
                block.start_line, diag.severity, diag.message, diag.span.start_line
            ));
        }
    }

    if !per_block_errors.is_empty() {
        let mut err_msg = format!("DocTest error in {}:\n", path.display());
        for line in per_block_errors {
            err_msg.push_str(&line);
        }
        errors.push(LintError {
            file: path.to_owned(),
            line: blocks[0].start_line,
            kind: LintKind::DocTestFailed { msg: err_msg },
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_vox_fences_and_archive_paths_produce_no_errors() {
        let mut errors = Vec::new();
        check_doctests(
            Path::new("docs/src/a.md"),
            "```rust\nfn main() {}\n```\n",
            &mut errors,
        );
        check_doctests(
            Path::new("docs/src/archive/a.md"),
            "```vox\n// vox:skip\nnot vox\n```\n",
            &mut errors,
        );
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn vox_skip_directive_is_rejected_at_block_start_line() {
        let mut errors = Vec::new();
        let doc = "# T\n\n```vox\n// vox:skip — excerpt\nfn x() {}\n```\n";
        check_doctests(Path::new("docs/src/a.md"), doc, &mut errors);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].line, 3);
        assert!(
            matches!(&errors[0].kind, LintKind::DocTestFailed { msg } if msg.contains("`vox:skip` is prohibited"))
        );
    }
}
