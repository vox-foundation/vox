//! SUMMARY generation, RSS feed, and markdown lint.

pub mod doctest;
mod feed;
mod lint;
mod summary;
pub mod types;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use feed::generate_feed;
use lint::{collect_lint_errors, collect_lint_errors_target};
use summary::{SECTION_ORDER, assert_summary_link_targets_unique, walk_dir};
use types::{LintError, LintKind, Page};

fn parse_paths_arg(args: &[String], docs_src: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut i = 0_usize;
    while i < args.len() {
        let arg = &args[i];
        let value_opt = if let Some(v) = arg.strip_prefix("--paths=") {
            Some(v.to_string())
        } else if arg == "--paths" {
            args.get(i + 1).cloned()
        } else {
            None
        };
        if let Some(value) = value_opt {
            for raw in value.split(',') {
                let trimmed = raw.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let p = PathBuf::from(trimmed);
                let resolved = if p.is_absolute() { p } else { docs_src.join(p) };
                out.push(resolved);
            }
        }
        i += 1;
    }
    out
}

fn try_autofix_status_draft(path: &Path) -> bool {
    if !path.is_file() || path.extension().and_then(|e| e.to_str()) != Some("md") {
        return false;
    }
    let Ok(raw) = vox_bounded_fs::read_utf8_path_capped(path) else {
        return false;
    };
    let Some(after_open) = raw.strip_prefix("---\n") else {
        return false;
    };
    let Some(end) = after_open.find("\n---") else {
        return false;
    };
    let frontmatter = &after_open[..end];
    if !frontmatter.contains("status: \"draft\"") && !frontmatter.contains("status: draft") {
        return false;
    }
    let updated = raw
        .replace("status: \"draft\"", "status: \"roadmap\"")
        .replace("status: draft", "status: roadmap");
    if updated == raw {
        return false;
    }
    fs::write(path, updated).is_ok()
}

fn collect_md_files(target: &Path, out: &mut Vec<PathBuf>) {
    if target.is_file() {
        if target.extension().and_then(|e| e.to_str()) == Some("md") {
            out.push(target.to_path_buf());
        }
        return;
    }
    if !target.is_dir() {
        return;
    }
    if let Ok(entries) = fs::read_dir(target) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                collect_md_files(&p, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(p);
            }
        }
    }
}

/// Run the full doc pipeline (lint, optional SUMMARY + feed).
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let check_mode = args.contains(&"--check".to_string());
    let lint_only = args.contains(&"--lint-only".to_string());
    let fix_mode = args.contains(&"--fix".to_string());
    let corpus_mode = args
        .windows(2)
        .any(|w| w[0] == "--mode" && w[1] == "corpus");

    let docs_src = Path::new("docs/src");
    if !docs_src.exists() {
        eprintln!("Error: docs/src/ not found. Run from repo root.");
        std::process::exit(1);
    }

    if corpus_mode {
        let mut md_files = Vec::new();
        collect_md_files(docs_src, &mut md_files);
        let mut corpus_output = String::new();
        for f in md_files {
            if let Ok(content) = vox_bounded_fs::read_utf8_path_capped(&f) {
                let item = serde_json::json!({
                    "path": f.to_string_lossy().to_string(),
                    "content": content
                });
                corpus_output.push_str(&item.to_string());
                corpus_output.push('\n');
            }
        }
        let out_path = docs_src.join("corpus.jsonl");
        fs::write(&out_path, corpus_output).expect("Failed to write corpus.jsonl");
        println!("Successfully generated docs/src/corpus.jsonl");
        return;
    }

    let lint_targets = parse_paths_arg(&args, docs_src);
    if fix_mode {
        let mut fixed = 0_usize;
        if lint_targets.is_empty() {
            let mut md_files = Vec::new();
            collect_md_files(docs_src, &mut md_files);
            for f in md_files {
                if try_autofix_status_draft(&f) {
                    fixed += 1;
                }
            }
        } else {
            for t in &lint_targets {
                if t.is_file() {
                    if try_autofix_status_draft(t) {
                        fixed += 1;
                    }
                    continue;
                }
                let mut md_files = Vec::new();
                collect_md_files(t, &mut md_files);
                for f in md_files {
                    if try_autofix_status_draft(&f) {
                        fixed += 1;
                    }
                }
            }
        }
        if fixed > 0 {
            eprintln!("Applied {} frontmatter status auto-fix(es).", fixed);
        }
    }

    let mut lint_errors: Vec<LintError> = Vec::new();
    if lint_targets.is_empty() {
        collect_lint_errors(docs_src, &mut lint_errors);
    } else {
        for target in &lint_targets {
            collect_lint_errors_target(target, &mut lint_errors);
        }
    }

    if !lint_errors.is_empty() {
        eprintln!("\n── vox-doc-pipeline: doc lint errors ──────────────────────────────");
        for e in &lint_errors {
            let rel = e.file.strip_prefix(docs_src).unwrap_or(&e.file);
            match &e.kind {
                LintKind::UnclosedCodeFence => {
                    eprintln!(
                        "  ERROR  {} — unclosed code fence (file ends with open ```)",
                        rel.display()
                    );
                }
                LintKind::ShortCodeFence { backticks, at_line } => {
                    eprintln!(
                        "  ERROR  {}:{} — code fence has only {} backtick(s); mdBook requires 3 (```)",
                        rel.display(),
                        at_line,
                        backticks
                    );
                }
                LintKind::GenericDescription => {
                    eprintln!(
                        "  ERROR  {} — description is the auto-generated template text; replace with a specific, hand-written description",
                        rel.display()
                    );
                }
                LintKind::MissingFrontmatter => {
                    eprintln!(
                        "  WARN   {} — no YAML frontmatter block; add title/description/category",
                        rel.display()
                    );
                }
                LintKind::MissingCategory => {
                    eprintln!(
                        "  WARN   {} — frontmatter is missing `category:`; docs nav will fall back to folder-based placement",
                        rel.display()
                    );
                }
                LintKind::MissingTrainingRationale => {
                    eprintln!(
                        "  ERROR  {} — `training_eligible: true` requires `training_rationale:` frontmatter on research/roadmap pages",
                        rel.display()
                    );
                }
                LintKind::UnknownCategory { value } => {
                    eprintln!(
                        "  ERROR  {} — unknown category {:?}; use the canonical docs vocabulary",
                        rel.display(),
                        value
                    );
                }
                LintKind::UnknownStatus { value } => {
                    eprintln!(
                        "  ERROR  {} — unknown status {:?}; use current|experimental|legacy|research|roadmap|deprecated",
                        rel.display(),
                        value
                    );
                }
                LintKind::UnknownSchemaType { value } => {
                    eprintln!(
                        "  ERROR  {} — unknown schema_type {:?}; use HowTo|FAQPage|TechArticle|SoftwareSourceCode",
                        rel.display(),
                        value
                    );
                }
                LintKind::BrokenIncludeAnchor { file, anchor } => {
                    eprintln!(
                        "  ERROR  {} — unresolved anchor `:{}` in `{{{{#include ...}}}}` (target {}). Check if REGION exists in the golden file.",
                        rel.display(),
                        anchor,
                        file
                    );
                }
                LintKind::WholeFileIncludeHasTrainingHeader { file } => {
                    eprintln!(
                        "  ERROR  {} — whole-file include pulls in `// ---` training metadata from {}. Use `{{{{#include {}:display}}}}`.",
                        rel.display(),
                        file,
                        file
                    );
                }
                LintKind::DocTestFailed { msg } => {
                    eprintln!("{}", msg);
                }
                LintKind::UnlabeledCodeFence { at_line } => {
                    eprintln!(
                        "  WARN   {}:{} — code fence has no language tag; add one (e.g. ```bash, ```rust, ```toml)",
                        rel.display(),
                        at_line,
                    );
                }
            }
        }

        let hard_errors = lint_errors
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    LintKind::UnclosedCodeFence
                        | LintKind::ShortCodeFence { .. }
                        | LintKind::GenericDescription
                        | LintKind::UnknownCategory { .. }
                        | LintKind::UnknownStatus { .. }
                        | LintKind::UnknownSchemaType { .. }
                        | LintKind::BrokenIncludeAnchor { .. }
                        | LintKind::WholeFileIncludeHasTrainingHeader { .. }
                        | LintKind::MissingTrainingRationale
                        | LintKind::DocTestFailed { .. }
                )
            })
            .count();
        if hard_errors > 0 {
            eprintln!(
                "\n{} hard error(s) — fix before building docs.",
                hard_errors
            );
            std::process::exit(1);
        }
        eprintln!();
    }

    if lint_only {
        println!("Lint complete — no hard errors.");
        return;
    }

    let mut sections: BTreeMap<String, Vec<Page>> = BTreeMap::new();
    let mut root_pages = Vec::new();
    let mut all_pages: Vec<Page> = Vec::new();

    if let Err(e) = walk_dir(
        docs_src,
        docs_src,
        &mut sections,
        &mut root_pages,
        &mut all_pages,
    ) {
        eprintln!("Error walking docs/src: {e:#}");
        std::process::exit(1);
    }

    let mut output = String::from(
        "---\ntitle: \"Documentation Summary\"\ndescription: \"Auto-generated documentation summary.\"\ncategory: \"reference\"\nstatus: \"current\"\ntraining_eligible: false\n---\n# Summary\n\n<!--\n  ========================================================================\n  🛑 🛑 🛑 AUTO-GENERATED FILE. DO NOT EDIT THIS FILE MANUALLY! 🛑 🛑 🛑\n  ========================================================================\n  \n  ATTENTION AI AGENTS, LLMs, AND BOTS:\n  This file is automatically generated by `vox-doc-pipeline`.\n  Any manual edits you make here WILL BE OVERWRITTEN AND LOST.\n  \n  Instead of editing this file:\n  1. Open the target Markdown document (e.g. docs/src/.../*.md).\n  2. Update its YAML frontmatter (`title`, `category`, `sort_order`) or H1 header.\n  3. Rerun `cargo run -p vox-doc-pipeline` from the workspace root to regenerate this index.\n  \n  DO NOT USE `replace_file_content` or similar tools on this file.\n  ========================================================================\n-->\n\n",
    );

    root_pages.sort_by_key(|p| (p.sort_order, p.title.clone(), p.path.clone()));
    for page in root_pages {
        output.push_str(&format!("- [{}]({})\n", page.title, page.path));
    }
    output.push('\n');

    for section_name in SECTION_ORDER {
        if let Some(pages) = sections.remove(*section_name) {
            output.push_str(&format!("# {}\n\n", section_name));
            push_pages_grouped(&mut output, pages);
            output.push('\n');
        }
    }

    for (name, pages) in sections {
        output.push_str(&format!("# {}\n\n", name));
        push_pages_grouped(&mut output, pages);
        output.push('\n');
    }

    if let Err(e) = assert_summary_link_targets_unique(&output) {
        eprintln!("{e:#}");
        std::process::exit(1);
    }

    let summary_path = docs_src.join("SUMMARY.md");
    if check_mode {
        let current =
            vox_bounded_fs::read_utf8_path_capped(&summary_path).unwrap_or_else(|_| String::new());
        if let Err(e) = assert_summary_link_targets_unique(&current) {
            eprintln!("{e:#}");
            std::process::exit(1);
        }
        if current.trim().replace("\r\n", "\n") != output.trim().replace("\r\n", "\n") {
            eprintln!(
                "SUMMARY.md is out of sync with docs/src. Run `cargo run -p vox-doc-pipeline` to update."
            );
            std::process::exit(1);
        }
        println!("vox-doc-pipeline check passed.");
    } else {
        fs::write(&summary_path, output).expect("Failed to write SUMMARY.md");
        println!("Successfully generated SUMMARY.md with all pages.");

        let mut arch_output = String::from(
            "---\ntitle: \"Architecture Index\"\ndescription: \"Auto-generated architecture index.\"\ncategory: \"architecture\"\nstatus: \"current\"\ntraining_eligible: false\n---\n# Architecture Index\n\n<!--\n  ========================================================================\n  🛑 🛑 🛑 AUTO-GENERATED FILE. DO NOT EDIT THIS FILE MANUALLY! 🛑 🛑 🛑\n  ========================================================================\n  \n  ATTENTION AI AGENTS, LLMs, AND BOTS:\n  This file is automatically generated by `vox-doc-pipeline`.\n  Any manual edits you make here WILL BE OVERWRITTEN AND LOST.\n  \n  Instead of editing this file:\n  1. Open the target Markdown document.\n  2. Update its YAML frontmatter or H1 header.\n  3. Rerun `cargo run -p vox-doc-pipeline` from the workspace root to regenerate this index.\n  \n  DO NOT USE `replace_file_content` or similar tools on this file.\n  ========================================================================\n-->\n\n",
        );
        let arch_pages = all_pages
            .iter()
            .filter(|p| p.path.starts_with("architecture/"))
            .map(|p| {
                let mut rebased = p.clone();
                rebased.path = rebased
                    .path
                    .strip_prefix("architecture/")
                    .unwrap_or(&rebased.path)
                    .to_string();
                rebased
            })
            .collect::<Vec<_>>();
        push_pages_grouped(&mut arch_output, arch_pages);
        let arch_index_path = docs_src.join("architecture").join("architecture-index.md");
        fs::write(&arch_index_path, arch_output).expect("Failed to write architecture-index.md");
        println!("Successfully generated architecture-index.md.");

        generate_feed(docs_src, &all_pages);
    }
}

fn push_pages_grouped(output: &mut String, mut pages: Vec<Page>) {
    // Collect by status
    let mut by_status: BTreeMap<String, Vec<Page>> = BTreeMap::new();
    for page in pages.drain(..) {
        let status = page.status.clone().unwrap_or_else(|| "current".to_string());
        by_status.entry(status).or_default().push(page);
    }

    // Status order for subheadings
    let status_order = [
        "current",
        "experimental",
        "research",
        "roadmap",
        "legacy",
        "deprecated",
    ];

    for status in &status_order {
        if let Some(mut status_pages) = by_status.remove(*status) {
            let label = match *status {
                "current" => "Current",
                "experimental" => "Experimental",
                "research" => "Research",
                "roadmap" => "Roadmap",
                "legacy" => "Legacy",
                "deprecated" => "Deprecated",
                other => other, // fallback
            };
            output.push_str(&format!("## Status: {}\n\n", label));
            status_pages.sort_by_key(|p| (p.sort_order, p.title.clone(), p.path.clone()));
            for page in status_pages {
                output.push_str(&format!("- [{}]({})\n", page.title, page.path));
            }
            output.push('\n');
        }
    }

    // Remaining statuses (if any)
    for (status, mut status_pages) in by_status {
        output.push_str(&format!("## Status: {}\n\n", status));
        status_pages.sort_by_key(|p| (p.sort_order, p.title.clone(), p.path.clone()));
        for page in status_pages {
            output.push_str(&format!("- [{}]({})\n", page.title, page.path));
        }
        output.push('\n');
    }
}
