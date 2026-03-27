//! Shared helpers for `@island` mount points (`data-vox-island` + `data-prop-*`).
//!
//! Hydration runtime: `islands/src/island-mount.tsx` (see `vox-cli` templates).
//!
//! # V1 contract (OP-0212)
//!
//! - Mount tag: HTML attribute `data-vox-island` with the island **name** (escaped for double quotes).
//! - Props: `data-prop-{kebab}` where `_` in the Vox/JSX attr becomes `-` ([`island_data_prop_attr`]).
//! - `bind` is omitted on the mount `div` (handled elsewhere for controlled inputs).
//!
//! **Compatibility:** AST JSX ([`super::jsx`]), HIR emit ([`super::hir_emit`]), and Web IR lowering must
//! keep the same string shapes until a V2 contract exists (OP-0214 — reserve extension points here).
//!
//! **Legacy-shrink:** bump [`ISLAND_MOUNT_FORMAT_VERSION`] only with a coordinated runtime + codegen
//! migration (hydrator in `vox-cli` templates).
//!
//! **V1 lock notes (OP-S039):** until `island_mount_format_version()` increments, every producer (AST JSX,
//! HIR `hir_emit`, Web IR `emit_tsx`) and the `vox-cli` `propsFromElement` decoder must agree on
//! `data-vox-island` + `data-prop-*` bytes—add regression tests before any optional-attribute or encoding change.
//!
//! **Island contract A/B/C (OP-S119 / S165 / S197):** V1 helpers remain shared across AST/HIR/Web IR until
//! [`ISLAND_MOUNT_FORMAT_VERSION`] bumps.

/// Monotonic island mount wire-format version for future V2 adapters (OP-0214).
pub const ISLAND_MOUNT_FORMAT_VERSION: u32 = 1;

#[must_use]
pub const fn island_mount_format_version() -> u32 {
    ISLAND_MOUNT_FORMAT_VERSION
}

use crate::hir::HirModule;
use std::collections::HashSet;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

static ISLAND_MOUNT_AST_FORMAT_COUNT: AtomicU64 = AtomicU64::new(0);
static ISLAND_MOUNT_HIR_FRAGMENT_COUNT: AtomicU64 = AtomicU64::new(0);

/// Counters for island mount helper usage (OP-0218); diagnostics only, best-effort relaxed ordering.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IslandCompatMetrics {
    pub ast_mount_formats: u64,
    pub hir_mount_fragments: u64,
}

#[must_use]
pub fn island_compat_metrics() -> IslandCompatMetrics {
    IslandCompatMetrics {
        ast_mount_formats: ISLAND_MOUNT_AST_FORMAT_COUNT.load(Ordering::Relaxed),
        hir_mount_fragments: ISLAND_MOUNT_HIR_FRAGMENT_COUNT.load(Ordering::Relaxed),
    }
}

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

/// Validate a Vox/JSX attribute name before mapping to `data-prop-*` (OP-0216).
pub fn validate_island_prop_attr_name(vox_attr: &str) -> Result<(), String> {
    if vox_attr.trim().is_empty() {
        return Err("island mount prop attribute name is empty".to_string());
    }
    Ok(())
}

/// Vox `snake_case` / JSX attr → `data-prop-foo-bar` (matches island-mount prop parsing).
#[must_use]
pub fn island_data_prop_attr(vox_attr: &str) -> String {
    format!("data-prop-{}", vox_attr.replace('_', "-"))
}

/// Like [`island_data_prop_attr`] but fails on empty / whitespace-only names (serializer diagnostic).
pub fn try_island_data_prop_attr(vox_attr: &str) -> Result<String, String> {
    validate_island_prop_attr_name(vox_attr)?;
    Ok(island_data_prop_attr(vox_attr))
}

/// Build the first mount chunk: `data-vox-island="..."` (HTML-escaped).
#[must_use]
pub fn island_mount_opening_part(tag: &str) -> String {
    format!("data-vox-island=\"{}\"", escape_html_attr(tag))
}

/// Sort `data-prop-*={...}` segments after [`island_mount_opening_part`] lexicographically.
///
/// [`crate::web_ir::emit_tsx`] does the same for [`crate::web_ir::DomNode::IslandMount`] so legacy
/// `hir_emit` / AST JSX stays whitespace-normalization–parity with the Web IR preview path.
pub fn sort_island_mount_data_prop_parts(parts: &mut [String]) {
    if parts.len() > 1 {
        parts[1..].sort();
    }
}

/// JSX block comment for islands used with non-empty JSX children (runtime ignores children).
#[must_use]
pub fn island_ignored_children_jsx_block(tag: &str, child_count: usize) -> String {
    format!("{{/* vox: @island `{tag}` ignores {child_count} JSX child(ren); use `<{tag} />` */}}")
}

/// Self-closing `<div ... />` with optional leading indent (`indent` × two spaces).
#[must_use]
pub fn island_mount_div_self_closing(indent: usize, parts: &[String]) -> String {
    let pad = "  ".repeat(indent);
    format!("{pad}<div {} />", parts.join(" "))
}

/// AST / classic JSX: newline-terminated mount line(s) for an `@island` tag.
///
/// `parts` must begin with [`island_mount_opening_part`]; remaining entries are `data-prop-*={expr}`.
#[must_use]
pub fn format_island_mount_ast(
    tag: &str,
    parts: &[String],
    indent: usize,
    child_count: usize,
) -> String {
    ISLAND_MOUNT_AST_FORMAT_COUNT.fetch_add(1, Ordering::Relaxed);
    if child_count == 0 {
        format!("{}\n", island_mount_div_self_closing(indent, parts))
    } else {
        let pad = "  ".repeat(indent);
        format!(
            "{pad}<>{}{}<div {} /></>\n",
            island_ignored_children_jsx_block(tag, child_count),
            "",
            parts.join(" ")
        )
    }
}

/// HIR / inline JSX: compact `<div />` or fragment-wrapped variant (caller verifies island tag).
#[must_use]
pub fn island_mount_hir_fragment(tag: &str, parts: &[String], child_count: usize) -> String {
    ISLAND_MOUNT_HIR_FRAGMENT_COUNT.fetch_add(1, Ordering::Relaxed);
    let inner = format!("<div {} />", parts.join(" "));
    if child_count == 0 {
        inner
    } else {
        format!(
            "<>{}{}</>",
            island_ignored_children_jsx_block(tag, child_count),
            inner
        )
    }
}
