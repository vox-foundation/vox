//! Library corpus walker: the Markdown files the KB indexes.
//!
//! The only filter is [`crate::kb_ids::is_kb_corpus_path`] — the same predicate the Phase 1
//! eval snapshot uses — plus "never follow or index symlinks".

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::FileError;
use crate::kb_ids::KbCorpusManifest;

/// One corpus file: absolute path plus repo-relative, forward-slash path (the doc id).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusFile {
    pub abs: PathBuf,
    pub rel_path: String,
}

/// Walk every directory and root file allowed by `manifest` under `repo_root`; sorted & deduplicated.
#[must_use]
pub fn walk_corpus_with_manifest(
    repo_root: &Path,
    manifest: &KbCorpusManifest,
) -> (Vec<CorpusFile>, Vec<FileError>) {
    let mut out = Vec::new();
    let mut seen_paths = BTreeSet::new();
    let mut errors = Vec::new();
    let excluded = &manifest.excluded_dir_names;

    for root in &manifest.root_dirs {
        let root_path = if root == "." {
            repo_root.to_path_buf()
        } else {
            repo_root.join(root)
        };
        let walker = WalkDir::new(&root_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                !(e.file_type().is_dir()
                    && e.file_name()
                        .to_str()
                        .is_some_and(|n| excluded.iter().any(|ex| ex == n)))
            });
        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    errors.push(FileError {
                        path: e
                            .path()
                            .and_then(|p| rel_slash_path(repo_root, p))
                            .unwrap_or_else(|| root.clone()),
                        error: e
                            .io_error()
                            .map_or_else(|| e.to_string(), ToString::to_string),
                    });
                    continue;
                }
            };
            if entry.path_is_symlink() || !entry.file_type().is_file() {
                continue;
            }
            let Some(rel_path) = rel_slash_path(repo_root, entry.path()) else {
                continue;
            };
            if manifest.includes_rel_path(&rel_path) && seen_paths.insert(rel_path.clone()) {
                out.push(CorpusFile {
                    abs: entry.into_path(),
                    rel_path,
                });
            }
        }
    }

    for rel in &manifest.root_files {
        let abs = repo_root.join(rel);
        match std::fs::symlink_metadata(&abs) {
            Ok(meta) => {
                if !meta.file_type().is_symlink()
                    && meta.is_file()
                    && manifest.includes_rel_path(rel)
                    && seen_paths.insert(rel.clone())
                {
                    out.push(CorpusFile {
                        abs,
                        rel_path: rel.clone(),
                    });
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => errors.push(FileError {
                path: rel.clone(),
                error: e.to_string(),
            }),
        }
    }

    out.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    (out, errors)
}

/// Walk every KB corpus root under `repo_root`; files are sorted by `rel_path`.
///
/// The second half holds every walk error (missing root, unreadable or vanished directory) with
/// a repo-relative path and no host path in the text. A caller that deletes data must treat a
/// non-empty error list as an incomplete walk.
#[must_use]
pub fn walk_corpus(repo_root: &Path) -> (Vec<CorpusFile>, Vec<FileError>) {
    walk_corpus_with_manifest(repo_root, &KbCorpusManifest::vox_default("vox"))
}

/// `path` relative to `repo_root`, forward-slash joined; `None` when outside it or non-UTF-8.
fn rel_slash_path(repo_root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(repo_root)
        .ok()?
        .components()
        .map(|c| c.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("/"))
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
        let (files, errors) = walk_corpus(dir.path());
        let got: Vec<String> = files.into_iter().map(|f| f.rel_path).collect();
        assert_eq!(got, ["docs/src/a/z.md", "docs/src/b.md"]);
        assert!(errors.is_empty(), "{errors:?}");
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
        let (files, errors) = walk_corpus(dir.path());
        let got: Vec<String> = files.into_iter().map(|f| f.rel_path).collect();
        assert_eq!(got, ["docs/src/real.md"]);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn walk_corpus_reports_missing_root() {
        let dir = tempfile::tempdir().unwrap();
        let (files, errors) = walk_corpus(dir.path());
        assert!(files.is_empty());
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].path, "docs/src");
        let host = dir.path().to_string_lossy();
        assert!(!errors[0].error.contains(host.as_ref()), "{errors:?}");
    }

    #[cfg(unix)]
    #[test]
    fn walk_corpus_reports_unreadable_dir() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "docs/src/a.md");
        write(dir.path(), "docs/src/locked/b.md");
        let locked = dir.path().join("docs/src/locked");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
        let bypassed = std::fs::read_dir(&locked).is_ok(); // e.g. running as root
        let (files, errors) = walk_corpus(dir.path());
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        if bypassed {
            return;
        }
        let got: Vec<String> = files.into_iter().map(|f| f.rel_path).collect();
        assert_eq!(got, ["docs/src/a.md"]);
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!(errors[0].path, "docs/src/locked");
        let host = dir.path().to_string_lossy();
        assert!(!errors[0].error.contains(host.as_ref()), "{errors:?}");
    }
}
