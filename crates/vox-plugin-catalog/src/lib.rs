//! SSOT catalog of all first-party Vox plugins and distribution bundles.
//!
//! See `docs/src/architecture/plugin-system-redesign-2026.md`.

pub mod schema;

use schema::{BundleEntry, PluginCatalogEntry};
use serde::Deserialize;
use std::sync::OnceLock;

/// Embedded raw catalog source. Validated at build time by `build.rs`.
const CATALOG_SRC: &str = include_str!("../catalog.toml");

#[derive(Deserialize)]
struct CatalogFile {
    #[serde(default, rename = "plugin")]
    plugins: Vec<PluginCatalogEntry>,
    #[serde(default, rename = "bundle")]
    bundles: Vec<BundleEntry>,
}

fn parsed() -> &'static CatalogFile {
    static CACHED: OnceLock<CatalogFile> = OnceLock::new();
    CACHED.get_or_init(|| {
        toml::from_str::<CatalogFile>(CATALOG_SRC)
            .expect("catalog.toml should parse — build.rs validates this")
    })
}

/// All first-party plugins declared in `catalog.toml`.
pub fn all_plugins() -> &'static [PluginCatalogEntry] {
    &parsed().plugins
}

/// All distribution bundles declared in `catalog.toml`.
pub fn all_bundles() -> &'static [BundleEntry] {
    &parsed().bundles
}
