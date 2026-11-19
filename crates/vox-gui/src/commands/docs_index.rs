//! Docs index for omni-search: walks `docs/src/**/*.md` frontmatter.
//!
//! Only **help** categories (how-to, tutorial, reference, contributor) are indexed
//! for the Omnibar — architecture/SSOT docs are excluded unless opened by path.

use std::path::{Component, Path, PathBuf};
use std::sync::OnceLock;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DocEntry {
    pub title: String,
    pub description: String,
    pub path: String, // absolute path for open_locator
}

pub(crate) struct Frontmatter {
    pub title: String,
    pub description: String,
    pub category: Option<String>,
}

const HELP_CATEGORIES: &[&str] = &["how-to", "tutorial", "reference", "contributor"];

pub(crate) fn parse_frontmatter(content: &str) -> Option<Frontmatter> {
    let rest = content.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let block = &rest[..end];
    let mut title = None;
    let mut description = None;
    let mut category = None;
    for line in block.lines() {
        if let Some(v) = line.strip_prefix("title:") {
            title = Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(v) = line.strip_prefix("description:") {
            description = Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
        } else if let Some(v) = line.strip_prefix("category:") {
            category = Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
        }
    }
    Some(Frontmatter {
        title: title?,
        description: description.unwrap_or_default(),
        category,
    })
}

pub(crate) fn is_help_doc(fm: &Frontmatter, path: &Path) -> bool {
    let path_str = path.to_string_lossy().replace('\\', "/");
    if path_str.contains("docs/src/archive") {
        return false;
    }
    fm.category
        .as_deref()
        .is_some_and(|c| HELP_CATEGORIES.contains(&c))
}

fn walk_docs(root: &Path, out: &mut Vec<DocEntry>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_docs(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md")
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Some(fm) = parse_frontmatter(&content)
        {
            if !is_help_doc(&fm, &path) {
                continue;
            }
            out.push(DocEntry {
                title: fm.title,
                description: fm.description,
                path: path.to_string_lossy().to_string(),
            });
        }
    }
}

fn repo_root_and_lib() -> (PathBuf, Option<PathBuf>) {
    let hint = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let repo = vox_repository::discover_repository_or_fallback(&hint);
    let lib_root = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|h| PathBuf::from(h).join(".vox").join("library"));
    (repo.root, lib_root)
}

fn normalize_path_components(path: &Path) -> Result<PathBuf, String> {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return Err("doc path escapes allowed workspace root".into());
                }
            }
            Component::Normal(c) => out.push(c),
        }
    }
    Ok(out)
}

const ALLOWED_DOC_EXTENSIONS: &[&str] = &[
    "md", "mdx", "markdown", "txt", "adoc", "asciidoc", "json", "jsonl", "yaml", "yml", "toml",
    "csv", "tsv", "vox", "rs", "ts", "tsx", "sql",
];

fn validate_doc_leaf_and_ext(resolved: &Path) -> Result<(), String> {
    if let Some(name) = resolved.file_name().and_then(|n| n.to_str()) {
        let lower_name = name.to_ascii_lowercase();
        if lower_name.starts_with(".env") && lower_name != ".env.example" {
            return Err("access to .env secret files is blocked".into());
        }
    }
    let ext = resolved
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default();
    if !ALLOWED_DOC_EXTENSIONS.contains(&ext.as_str()) {
        return Err("doc path must have a supported documentation or source extension".into());
    }
    Ok(())
}

/// Resolve `path` strictly under `docs_root`, rejecting `..` escapes (preserved for tests).
#[cfg(test)]
pub(crate) fn resolve_path_under_root(docs_root: &Path, path: &str) -> Result<PathBuf, String> {
    let docs_root = normalize_path_components(docs_root)?;
    let candidate = PathBuf::from(path);
    let joined = if candidate.is_absolute() {
        candidate
    } else {
        docs_root.join(candidate)
    };
    let resolved = normalize_path_components(&joined)?;
    if !resolved.starts_with(&docs_root) {
        return Err("doc path must be under docs/src".into());
    }
    validate_doc_leaf_and_ext(&resolved)?;
    Ok(resolved)
}

/// Resolve workspace-wide documentation, wiki, contract, source, or `~/.vox/library/` paths.
pub(crate) fn resolve_workspace_doc_path(
    repo_root: &Path,
    user_lib_root: Option<&Path>,
    path: &str,
) -> Result<PathBuf, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("doc path must not be empty".into());
    }
    let slash_norm = trimmed.replace('\\', "/");

    if slash_norm.starts_with("~/") {
        let lib_rel = slash_norm
            .strip_prefix("~/.vox/library/")
            .ok_or_else(|| "home doc path must reside under ~/.vox/library/".to_string())?;
        let lib_root =
            user_lib_root.ok_or_else(|| "user library directory unavailable".to_string())?;
        let norm_lib = normalize_path_components(lib_root)?;
        let resolved = normalize_path_components(&norm_lib.join(lib_rel))?;
        if !resolved.starts_with(&norm_lib) {
            return Err("doc path escapes ~/.vox/library".into());
        }
        validate_doc_leaf_and_ext(&resolved)?;
        return Ok(resolved);
    }

    let norm_repo = normalize_path_components(repo_root)?;
    let candidate = PathBuf::from(trimmed);
    let resolved = if candidate.is_absolute() {
        let r = normalize_path_components(&candidate)?;
        let in_lib = user_lib_root
            .and_then(|l| normalize_path_components(l).ok())
            .is_some_and(|nl| r.starts_with(&nl));
        if !r.starts_with(&norm_repo) && !in_lib {
            return Err(
                "doc path must reside inside the active workspace or ~/.vox/library".into(),
            );
        }
        r
    } else if slash_norm.starts_with("docs/")
        || slash_norm.starts_with("contracts/")
        || slash_norm.starts_with("crates/")
        || slash_norm.starts_with("scripts/")
        || norm_repo.join(trimmed).exists()
    {
        let r = normalize_path_components(&norm_repo.join(trimmed))?;
        if !r.starts_with(&norm_repo) {
            return Err("doc path escapes workspace root".into());
        }
        r
    } else {
        let docs_src = norm_repo.join("docs").join("src");
        let r = normalize_path_components(&docs_src.join(trimmed))?;
        if !r.starts_with(&norm_repo) {
            return Err("doc path escapes workspace root".into());
        }
        r
    };

    validate_doc_leaf_and_ext(&resolved)?;
    Ok(resolved)
}

fn resolve_doc_path(path: &str) -> Result<PathBuf, String> {
    let (repo_root, lib_root) = repo_root_and_lib();
    resolve_workspace_doc_path(&repo_root, lib_root.as_deref(), path)
}

static DOCS_CACHE: OnceLock<Vec<DocEntry>> = OnceLock::new();

#[tauri::command]
pub async fn vox_docs_index() -> Result<Vec<DocEntry>, String> {
    Ok(DOCS_CACHE
        .get_or_init(|| {
            let mut out = Vec::new();
            let (repo_root, _) = repo_root_and_lib();
            walk_docs(&repo_root.join("docs").join("src"), &mut out);
            walk_docs(&repo_root.join("docs").join("wiki"), &mut out);
            out.sort_by(|a, b| a.title.cmp(&b.title).then_with(|| a.path.cmp(&b.path)));
            out.dedup_by(|a, b| a.path == b.path);
            out
        })
        .clone())
}

#[tauri::command]
pub async fn read_doc_markdown(path: String) -> Result<String, String> {
    let resolved = resolve_doc_path(&path)?;
    std::fs::read_to_string(&resolved).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn write_doc_markdown(path: String, content: String) -> Result<(), String> {
    let resolved = resolve_doc_path(&path)?;
    if let Some(parent) = resolved.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::write(&resolved, content).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parses_frontmatter_title_and_description() {
        let md = "---\ntitle: Mesh SSOT\ndescription: Seven phases of mesh work\ncategory: architecture\n---\n\n# Body\n";
        let fm = parse_frontmatter(md).expect("frontmatter");
        assert_eq!(fm.title, "Mesh SSOT");
        assert_eq!(fm.description, "Seven phases of mesh work");
        assert_eq!(fm.category.as_deref(), Some("architecture"));
    }

    #[test]
    fn missing_frontmatter_returns_none() {
        assert!(parse_frontmatter("# Just a heading\n").is_none());
    }

    #[test]
    fn strips_surrounding_quotes() {
        let md = "---\ntitle: \"Quoted Title\"\ndescription: 'single'\n---\nbody";
        let fm = parse_frontmatter(md).expect("frontmatter");
        assert_eq!(fm.title, "Quoted Title");
        assert_eq!(fm.description, "single");
    }

    #[test]
    fn excludes_architecture_category() {
        let fm = Frontmatter {
            title: "Arch".into(),
            description: String::new(),
            category: Some("architecture".into()),
        };
        assert!(!is_help_doc(&fm, Path::new("docs/src/architecture/foo.md"),));
    }

    #[test]
    fn includes_how_to() {
        let fm = Frontmatter {
            title: "How".into(),
            description: String::new(),
            category: Some("how-to".into()),
        };
        assert!(is_help_doc(&fm, Path::new("docs/src/how-to/foo.md"),));
    }

    #[test]
    fn rejects_path_traversal_outside_docs_root() {
        let root = Path::new("/repo/docs/src");
        assert!(resolve_path_under_root(root, "../../../etc/passwd").is_err());
        assert!(resolve_path_under_root(root, "reference/../../outside.md").is_err());
    }

    #[test]
    fn accepts_relative_help_doc_under_root() {
        let root = Path::new("/repo/docs/src");
        let resolved = resolve_path_under_root(root, "reference/cli.md").expect("resolve");
        assert!(resolved.ends_with("reference/cli.md"));
    }

    #[test]
    fn resolves_workspace_paths_without_double_docs_src_prefix() {
        let repo = Path::new("/repo");
        let lib = Path::new("/home/user/.vox/library");

        let src_doc =
            resolve_workspace_doc_path(repo, Some(lib), "docs/src/reference/cli.md").expect("src");
        assert_eq!(src_doc, PathBuf::from("/repo/docs/src/reference/cli.md"));

        let wiki_doc =
            resolve_workspace_doc_path(repo, Some(lib), "docs/wiki/interfaces-and-apis.md")
                .expect("wiki");
        assert_eq!(
            wiki_doc,
            PathBuf::from("/repo/docs/wiki/interfaces-and-apis.md")
        );

        let contract_yaml = resolve_workspace_doc_path(
            repo,
            Some(lib),
            "contracts/documentation/kb-taxonomy.v1.yaml",
        )
        .expect("contract");
        assert_eq!(
            contract_yaml,
            PathBuf::from("/repo/contracts/documentation/kb-taxonomy.v1.yaml")
        );

        let crate_rs =
            resolve_workspace_doc_path(repo, Some(lib), "crates/vox-gui/src/main.rs").expect("rs");
        assert_eq!(crate_rs, PathBuf::from("/repo/crates/vox-gui/src/main.rs"));

        let lib_doc =
            resolve_workspace_doc_path(repo, Some(lib), "~/.vox/library/notes.md").expect("lib");
        assert_eq!(lib_doc, PathBuf::from("/home/user/.vox/library/notes.md"));
    }

    #[test]
    fn rejects_env_secrets_and_traversal_in_workspace_doc_resolver() {
        let repo = Path::new("/repo");
        let lib = Path::new("/home/user/.vox/library");

        assert!(resolve_workspace_doc_path(repo, Some(lib), ".env").is_err());
        assert!(resolve_workspace_doc_path(repo, Some(lib), ".env.local").is_err());
        assert!(resolve_workspace_doc_path(repo, Some(lib), "../../../etc/passwd").is_err());
        assert!(resolve_workspace_doc_path(repo, Some(lib), "~/.ssh/id_rsa.txt").is_err());
    }
}
