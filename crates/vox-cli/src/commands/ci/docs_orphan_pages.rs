//! Gated check for docs pages with zero inbound links.
//!
//! Pages that nothing links to (neither any other docs page nor any root guide)
//! are flagged as orphans. Index pages, generated documentation, and archive
//! trees are exempt.

use crate::commands::ci::docs_cli_claims::Scope;
use anyhow::{Result, bail};
use std::path::{Path, PathBuf};

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            _ => out.push(comp),
        }
    }
    out
}

/// Find all pages in `pages` that have zero inbound links according to the
/// prune link graph. Exempts index pages, generated pages, `AGENTS.md`,
/// and `.well-known/` or `archive/` paths.
pub(crate) fn find_orphans(root: &Path, pages: &[PathBuf]) -> Result<Vec<String>> {
    let root_norm = normalize_path(root);
    let root = &root_norm;
    let docs_dir = root.join("docs/src");
    let root_guides = [
        "AGENTS.md",
        "README.md",
        "CONTRIBUTING.md",
        "CLAUDE.md",
        "GEMINI.md",
    ]
    .iter()
    .map(|p| root.join(p))
    .filter(|p| p.is_file())
    .collect::<Vec<_>>();

    let graph =
        vox_doc_pipeline::claims::prune::build_inbound_link_graph(root, &docs_dir, &root_guides);

    let mut orphans = Vec::new();

    for page in pages {
        let norm_page = normalize_path(page);
        let rel = norm_page
            .strip_prefix(root)
            .unwrap_or(&norm_page)
            .to_string_lossy()
            .replace('\\', "/");

        let file_name = page.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if file_name == "index.md" || file_name == "index.mdx" {
            continue;
        }
        if file_name.ends_with(".generated.md") {
            continue;
        }
        if rel == "docs/src/AGENTS.md" || rel == "AGENTS.md" {
            continue;
        }
        if page
            .components()
            .any(|c| c.as_os_str() == ".well-known" || c.as_os_str() == "archive")
        {
            continue;
        }

        let inbound = graph.get(&rel);
        if inbound.map_or(true, |set| set.is_empty()) {
            orphans.push(rel);
        }
    }

    orphans.sort();
    orphans.dedup();
    Ok(orphans)
}

/// Gated orphan check: gates reader-facing pages (tutorials, how-to, reference, explanation, etc.)
/// at zero orphans, and prints the whole-tree orphan count for reporting.
pub(crate) fn run(root: &Path) -> Result<()> {
    let root_norm = normalize_path(root);
    let root = &root_norm;

    let mut pages = crate::commands::ci::docs_cli_claims::scope_files(root, Scope::ReaderFacing)?;

    let tutorials_dir = root.join("docs/src/tutorials");
    if tutorials_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&tutorials_dir)
            .follow_links(false)
            .into_iter()
            .flatten()
        {
            let p = entry.path();
            if p.is_file() {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "md" || ext == "mdx" {
                    if !p.components().any(|c| c.as_os_str() == "archive") {
                        pages.push(p.to_path_buf());
                    }
                }
            }
        }
    }
    pages.sort();
    pages.dedup();

    let orphans = find_orphans(root, &pages)?;
    if !orphans.is_empty() {
        eprintln!(
            "docs_orphan_pages: found {} orphaned reader-facing page(s):",
            orphans.len()
        );
        for orphan in &orphans {
            eprintln!("  {orphan} (HEAL: link it from a related existing page)");
        }
        bail!(
            "docs_orphan_pages: {} orphaned page(s) found",
            orphans.len()
        );
    }

    // Whole-tree orphan report (informational, not gated)
    let docs_dir = root.join("docs/src");
    let mut all_pages = Vec::new();
    if docs_dir.is_dir() {
        for entry in walkdir::WalkDir::new(&docs_dir)
            .follow_links(false)
            .into_iter()
            .flatten()
        {
            let p = entry.path();
            if p.is_file() {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "md" || ext == "mdx" {
                    if !p.components().any(|c| c.as_os_str() == "archive") {
                        all_pages.push(p.to_path_buf());
                    }
                }
            }
        }
    }
    let whole_tree_orphans = find_orphans(root, &all_pages)?;
    println!(
        "docs_orphan_pages: {} whole-tree orphan(s) (architecture research)",
        whole_tree_orphans.len()
    );

    println!("docs_orphan_pages OK (zero reader-facing orphans)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::ci::docs_cli_claims::Scope;

    #[test]
    fn find_orphans_exemptions() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let docs_dir = root.join("docs/src");
        std::fs::create_dir_all(&docs_dir).unwrap();

        let a = docs_dir.join("a.md");
        let b = docs_dir.join("b.md");
        let c = docs_dir.join("c.md");
        let idx = docs_dir.join("index.md");
        let gen_file = docs_dir.join("x.generated.md");

        std::fs::write(&a, "See [b](b.md).\n").unwrap();
        std::fs::write(&b, "# B\n").unwrap();
        std::fs::write(&c, "# C\n").unwrap();
        std::fs::write(&idx, "# Index\n").unwrap();
        std::fs::write(&gen_file, "# Generated\n").unwrap();

        let orphans = find_orphans(root, &[b, c, idx, gen_file]).unwrap();
        assert_eq!(orphans, vec!["docs/src/c.md".to_string()]);
    }

    #[test]
    fn test_run_reader_facing_real_tree() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert!(run(&repo_root).is_ok());
    }

    #[test]
    fn test_run_fails_on_fixture_orphan() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let how_to = root.join("docs/src/how-to");
        std::fs::create_dir_all(&how_to).unwrap();
        let page = how_to.join("orphan.md");
        std::fs::write(
            &page,
            "---\ntitle: \"Orphan\"\ndescription: \"desc\"\ncategory: \"How-To Guides\"\nstatus: \"current\"\n---\n# Orphan\n",
        )
        .unwrap();

        let res = run(root);
        assert!(res.is_err());
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("orphaned page(s) found"));
    }

    #[test]
    fn real_tree_orphans_measure() {
        let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let reader_facing_pages =
            crate::commands::ci::docs_cli_claims::scope_files(&repo_root, Scope::ReaderFacing)
                .expect("scope reader facing");
        let reader_orphans = find_orphans(&repo_root, &reader_facing_pages).expect("find orphans");

        println!(
            "docs_orphan_pages: {} reader-facing orphan(s)",
            reader_orphans.len()
        );
        for ro in &reader_orphans {
            println!("  reader orphan: {ro}");
        }

        // Measure whole non-archive tree
        let docs_src = repo_root.join("docs/src");
        let mut whole_tree_pages = Vec::new();
        for entry in walkdir::WalkDir::new(&docs_src)
            .follow_links(false)
            .into_iter()
            .flatten()
        {
            let p = entry.path();
            if p.is_file() {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "md" || ext == "mdx" {
                    if !p.components().any(|c| c.as_os_str() == "archive") {
                        whole_tree_pages.push(p.to_path_buf());
                    }
                }
            }
        }
        let all_orphans = find_orphans(&repo_root, &whole_tree_pages).expect("all orphans");

        let mut by_dir: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for orphan in &all_orphans {
            let section = orphan
                .strip_prefix("docs/src/")
                .unwrap_or(orphan)
                .split('/')
                .next()
                .unwrap_or("root")
                .to_string();
            *by_dir.entry(section).or_default() += 1;
        }

        println!(
            "docs_orphan_pages: {} whole-tree orphan(s):",
            all_orphans.len()
        );
        for (sec, count) in &by_dir {
            println!("  {sec}: {count}");
        }
    }
}
