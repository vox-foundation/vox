//! SUMMARY generation, RSS feed, and markdown lint.

mod feed;
mod lint;
mod summary;
mod types;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use feed::generate_feed;
use lint::collect_lint_errors;
use summary::{SECTION_ORDER, assert_summary_link_targets_unique, walk_dir};
use types::{LintError, LintKind, Page};

/// Run the full doc pipeline (lint, optional SUMMARY + feed).
pub fn run() {
    let args: Vec<String> = std::env::args().collect();
    let check_mode = args.contains(&"--check".to_string());
    let lint_only = args.contains(&"--lint-only".to_string());

    let docs_src = Path::new("docs/src");
    if !docs_src.exists() {
        eprintln!("Error: docs/src/ not found. Run from repo root.");
        std::process::exit(1);
    }

    let mut lint_errors: Vec<LintError> = Vec::new();
    collect_lint_errors(docs_src, &mut lint_errors);

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
                LintKind::RawVoxCodeBlock => {
                    eprintln!(
                        "  ERROR  {}:{} — raw vox/tsx code block detected; replace with `{{{{#include ...}}}}` from `examples/golden/` or add `// Skip-Test`",
                        rel.display(),
                        e.line
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
                        | LintKind::RawVoxCodeBlock
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

    let mut output = String::from("# Summary\n\n");

    root_pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
    for page in root_pages {
        output.push_str(&format!("- [{}]({})\n", page.title, page.path));
    }
    output.push('\n');

    for section_name in SECTION_ORDER {
        if let Some(mut pages) = sections.remove(*section_name) {
            output.push_str(&format!("# {}\n\n", section_name));
            pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
            for page in pages {
                output.push_str(&format!("- [{}]({})\n", page.title, page.path));
            }
            output.push('\n');
        }
    }

    for (name, mut pages) in sections {
        output.push_str(&format!("# {}\n\n", name));
        pages.sort_by_key(|p| (p.sort_order, p.title.clone()));
        for page in pages {
            output.push_str(&format!("- [{}]({})\n", page.title, page.path));
        }
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
        if current.trim() != output.trim() {
            eprintln!(
                "SUMMARY.md is out of sync with docs/src. Run `cargo run -p vox-doc-pipeline` to update."
            );
            std::process::exit(1);
        }
        println!("vox-doc-pipeline check passed.");
    } else {
        fs::write(&summary_path, output).expect("Failed to write SUMMARY.md");
        println!("Successfully generated SUMMARY.md with all pages.");
        generate_feed(docs_src, &all_pages);
    }
}
