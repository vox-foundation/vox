//! Library corpus walker: the Markdown files the KB indexes.
//!
//! The only filter is [`crate::kb_ids::is_kb_corpus_path`] — the same predicate the Phase 1
//! eval snapshot uses — plus "never follow or index symlinks".

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::kb_ids::{KB_CORPUS_ROOTS, KB_EXCLUDED_DIR_NAMES, is_kb_corpus_path};

/// One corpus file: absolute path plus repo-relative, forward-slash path (the doc id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusFile {
    pub abs: PathBuf,
    pub rel_path: String,
}

/// Walk every KB corpus root under `repo_root`; result is sorted by `rel_path`.
#[must_use]
pub fn walk_corpus(repo_root: &Path) -> Vec<CorpusFile> {
    let mut out = Vec::new();
    for root in KB_CORPUS_ROOTS {
        let walker = WalkDir::new(repo_root.join(root))
            .follow_links(false)
            .into_iter()
            // Pruning only skips work; `is_kb_corpus_path` below stays the authority.
            .filter_entry(|e| {
                !(e.file_type().is_dir()
                    && e.file_name()
                        .to_str()
                        .is_some_and(|n| KB_EXCLUDED_DIR_NAMES.contains(&n)))
            });
        for entry in walker.flatten() {
            if entry.path_is_symlink() || !entry.file_type().is_file() {
                continue;
            }
            let Ok(rel) = entry.path().strip_prefix(repo_root) else {
                continue;
            };
            let Some(rel_path) = rel
                .components()
                .map(|c| c.as_os_str().to_str())
                .collect::<Option<Vec<_>>>()
                .map(|parts| parts.join("/"))
            else {
                continue;
            };
            if is_kb_corpus_path(&rel_path) {
                out.push(CorpusFile {
                    abs: entry.into_path(),
                    rel_path,
                });
            }
        }
    }
    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, "# T\n").unwrap();
    }

    #[test]
    fn walk_corpus_applies_the_shared_filter_sorted() {
        let dir = tempfile::tempdir().unwrap();
        for rel in [
            "docs/src/b.md",
            "docs/src/a/z.md",
            "docs/src/archive/old.md",
            "docs/src/a/archive/deep.md",
            "docs/src/notes.txt",
            "docs/other/x.md",
            "docs/src/c#-notes.md",
        ] {
            write(dir.path(), rel);
        }
        let got: Vec<String> = walk_corpus(dir.path())
            .into_iter()
            .map(|f| f.rel_path)
            .collect();
        assert_eq!(got, ["docs/src/a/z.md", "docs/src/b.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn walk_corpus_skips_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docs/src/real.md");
        std::os::unix::fs::symlink(
            dir.path().join("docs/src/real.md"),
            dir.path().join("docs/src/link.md"),
        )
        .unwrap();
        let got: Vec<String> = walk_corpus(dir.path())
            .into_iter()
            .map(|f| f.rel_path)
            .collect();
        assert_eq!(got, ["docs/src/real.md"]);
    }
}
