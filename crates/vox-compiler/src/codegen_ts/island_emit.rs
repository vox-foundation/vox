//! Shared helpers for `@island` mount points (`data-vox-island` + `data-prop-*`).
//!
//! Hydration runtime: `islands/src/island-mount.tsx` (see `vox-cli` templates).

use crate::hir::HirModule;
use std::collections::HashSet;
use std::sync::OnceLock;

/// Names declared via `@island Name { ... }` in the current module.
#[must_use]
pub fn collect_island_names(hir: &HirModule) -> HashSet<String> {
    hir.islands.iter().map(|i| i.0.name.clone()).collect()
}

/// Static empty set for call sites with no islands (e.g. Express `routes.rs` emit).
#[must_use]
pub fn empty_island_set() -> &'static HashSet<String> {
    static EMPTY: OnceLock<HashSet<String>> = OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}

#[must_use]
pub fn escape_html_attr(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

/// Vox `snake_case` / JSX attr → `data-prop-foo-bar` (matches island-mount prop parsing).
#[must_use]
pub fn island_data_prop_attr(vox_attr: &str) -> String {
    format!("data-prop-{}", vox_attr.replace('_', "-"))
}
