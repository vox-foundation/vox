//! Build-time validation of catalog.toml. Runs the same structural checks the
//! integration tests in tests/catalog_validation.rs do, but at compile time so
//! a malformed catalog fails the build instead of a runtime test.

use serde::Deserialize;
use std::collections::HashSet;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct PluginEntry {
    id: String,
    payload_kind: String,
    #[serde(default)]
    extension_points: Option<Vec<String>>,
    #[serde(default)]
    exposes_tools: Option<Vec<String>>,
    default_source: String,
    #[serde(default)]
    bundled_in: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
struct BundleEntry {
    id: String,
    #[serde(default)]
    extends: Option<String>,
    #[serde(default)]
    plugins: Vec<String>,
}

#[derive(Deserialize)]
struct CatalogFile {
    #[serde(default, rename = "plugin")]
    plugins: Vec<PluginEntry>,
    #[serde(default, rename = "bundle")]
    bundles: Vec<BundleEntry>,
}

fn main() {
    println!("cargo:rerun-if-changed=catalog.toml");
    let src = std::fs::read_to_string("catalog.toml")
        .expect("catalog.toml not found");
    let cat: CatalogFile = match toml::from_str(&src) {
        Ok(v) => v,
        Err(e) => {
            panic!("catalog.toml failed to parse: {e}");
        }
    };

    let mut errors: Vec<String> = Vec::new();

    // Unique ids
    let mut plugin_ids = HashSet::new();
    for p in &cat.plugins {
        if !plugin_ids.insert(p.id.clone()) {
            errors.push(format!("duplicate plugin id: {}", p.id));
        }
    }
    let mut bundle_ids = HashSet::new();
    for b in &cat.bundles {
        if !bundle_ids.insert(b.id.clone()) {
            errors.push(format!("duplicate bundle id: {}", b.id));
        }
    }

    // Cross-references
    for p in &cat.plugins {
        for b in &p.bundled_in {
            if !bundle_ids.contains(b) {
                errors.push(format!(
                    "plugin '{}' lists bundled-in='{}', but no such bundle exists",
                    p.id, b
                ));
            }
        }
    }
    for b in &cat.bundles {
        for p in &b.plugins {
            if !plugin_ids.contains(p) {
                errors.push(format!(
                    "bundle '{}' lists plugin '{}', but no such plugin exists",
                    b.id, p
                ));
            }
        }
        if let Some(parent) = &b.extends {
            if !bundle_ids.contains(parent) {
                errors.push(format!(
                    "bundle '{}' extends '{}', but no such bundle exists",
                    b.id, parent
                ));
            }
        }
    }

    // Per-payload-kind requirements
    for p in &cat.plugins {
        match p.payload_kind.as_str() {
            "code" => {
                if p.extension_points.as_ref().is_none_or(|v| v.is_empty()) {
                    errors.push(format!(
                        "code plugin '{}' must declare extension-points",
                        p.id
                    ));
                }
            }
            "skill" => {
                if p.exposes_tools.as_ref().is_none_or(|v| v.is_empty()) {
                    errors.push(format!(
                        "skill plugin '{}' must declare exposes-tools",
                        p.id
                    ));
                }
            }
            "composite" => {
                if p.extension_points.as_ref().is_none_or(|v| v.is_empty()) {
                    errors.push(format!(
                        "composite plugin '{}' must declare extension-points",
                        p.id
                    ));
                }
                if p.exposes_tools.as_ref().is_none_or(|v| v.is_empty()) {
                    errors.push(format!(
                        "composite plugin '{}' must declare exposes-tools",
                        p.id
                    ));
                }
            }
            other => {
                errors.push(format!(
                    "plugin '{}' has unknown payload-kind '{}' (must be code|skill|composite)",
                    p.id, other
                ));
            }
        }
        if p.default_source.is_empty() {
            errors.push(format!("plugin '{}' has empty default-source", p.id));
        }
    }

    if !errors.is_empty() {
        for e in &errors {
            println!("cargo:warning={e}");
        }
        panic!(
            "catalog.toml validation failed with {} error(s); see warnings above",
            errors.len()
        );
    }
}
