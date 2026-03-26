//! On-disk layout for v0.dev **islands** (`islands/src/<Name>/…` or `packages/islands/src/…`).
//!
//! [`resolve_island_main_tsx`] backs `vox island upgrade` when the **`island`** Cargo feature is on.
//! Unit tests keep the path rules stable without enabling that feature.

use anyhow::Result;
use std::path::{Path, PathBuf};

/// Default **bootstrap** directory when creating a new islands Vite app (`<repo>/islands/`).
#[must_use]
pub fn island_root(root: &Path) -> PathBuf {
    root.join("islands")
}

/// Resolved islands package root: **`islands/`** or **`packages/islands/`** when `package.json` exists.
///
/// Falls back to [`island_root`] so callers can still construct paths before first bootstrap.
#[must_use]
pub fn island_package_root(repo_root: &Path) -> PathBuf {
    resolve_island_package_root(repo_root).unwrap_or_else(|| island_root(repo_root))
}

/// `Some(path)` when a workspace-style or repo-root islands app is present.
#[must_use]
pub fn resolve_island_package_root(repo_root: &Path) -> Option<PathBuf> {
    let candidates = [
        repo_root.join("islands"),
        repo_root.join("packages").join("islands"),
    ];
    for p in candidates {
        if p.join("package.json").is_file() {
            return Some(p);
        }
    }
    None
}

/// **`…/src/`** under [`island_package_root`].
#[must_use]
pub fn island_src_dir(root: &Path) -> PathBuf {
    island_package_root(root).join("src")
}

/// **`islands/src/<Name>/`** directory for one island.
#[must_use]
pub fn island_component_dir(root: &Path, name: &str) -> PathBuf {
    island_src_dir(root).join(name)
}

/// Primary generated file: **`islands/src/<Name>/<Name>.component.tsx`**.
#[must_use]
pub fn island_component_tsx_path(root: &Path, name: &str) -> PathBuf {
    island_component_dir(root, name).join(format!("{name}.component.tsx"))
}

/// Prefer `islands/src/<Name>/<Name>.component.tsx`, else `islands/src/<Name>/index.tsx`.
///
/// Used by `vox island upgrade` to locate the main TSX file for an island.
#[cfg_attr(all(not(feature = "island"), not(test)), allow(dead_code))]
pub fn resolve_island_main_tsx(root: &Path, name: &str) -> Result<PathBuf> {
    let base = island_component_dir(root, name);
    let component_tsx = island_component_tsx_path(root, name);
    let index_tsx = base.join("index.tsx");
    if component_tsx.exists() {
        Ok(component_tsx)
    } else if index_tsx.exists() {
        Ok(index_tsx)
    } else {
        anyhow::bail!(
            "Island '{name}' not found. Expected one of:\n  {}\n  {}\n\
             Use `vox island generate` to create it.",
            component_tsx.display(),
            index_tsx.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn paths_match_layout() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let name = "FooBar";
        assert_eq!(island_root(root), root.join("islands"));
        assert_eq!(island_src_dir(root), root.join("islands").join("src"));
        assert_eq!(
            island_component_tsx_path(root, name),
            root.join("islands")
                .join("src")
                .join(name)
                .join("FooBar.component.tsx")
        );
    }

    #[test]
    fn packages_islands_resolves_when_pkg_present() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let pkg_root = root.join("packages").join("islands");
        std::fs::create_dir_all(pkg_root.join("src")).unwrap();
        std::fs::write(pkg_root.join("package.json"), "{}").unwrap();
        assert_eq!(resolve_island_package_root(root), Some(pkg_root.clone()));
        assert_eq!(island_package_root(root), pkg_root);
    }

    #[test]
    fn resolve_prefers_component_file() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let name = "FooBar";
        let base = island_component_dir(root, name);
        fs::create_dir_all(&base).unwrap();
        let component = island_component_tsx_path(root, name);
        fs::write(&component, "//c").unwrap();
        fs::write(base.join("index.tsx"), "//i").unwrap();
        let p = resolve_island_main_tsx(root, name).unwrap();
        assert_eq!(p, component);
    }

    #[test]
    fn resolve_falls_back_to_index() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let name = "FooBar";
        let base = island_component_dir(root, name);
        fs::create_dir_all(&base).unwrap();
        let index = base.join("index.tsx");
        fs::write(&index, "//i").unwrap();
        let p = resolve_island_main_tsx(root, name).unwrap();
        assert_eq!(p, index);
    }

    #[test]
    fn resolve_errors_when_missing() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        let err = resolve_island_main_tsx(root, "Nope").unwrap_err();
        assert!(err.to_string().contains("Nope"));
    }
}
