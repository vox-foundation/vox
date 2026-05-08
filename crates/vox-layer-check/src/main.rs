//! Layer check: enforces the 6-layer workspace architecture.
//!
//! Reads `docs/src/architecture/layers.toml`, runs `cargo metadata`, and verifies
//! that every workspace dep edge respects layer ordering (a crate at layer N may
//! depend only on crates at layer ≤ N, except for entries listed in
//! `known_inversions` — transitional debt scheduled for fix-up).
//!
//! Exit codes:
//!   0 — clean (no unknown inversions)
//!   1 — inversions found OR config error
//!
//! Modes:
//!   default        — fail (exit 1) on any unknown inversion
//!   --warn-only    — print but exit 0 (used during refactor phases 0–8)
//!
//! See `docs/src/architecture/2026-05-08-workspace-reorg-design.md` Phase 9 for
//! the schedule that flips this from warn to error.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result, anyhow};
use cargo_metadata::MetadataCommand;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct LayersConfig {
    /// Map from crate name to its assigned layer (0..=5).
    crates: HashMap<String, CrateEntry>,
    /// Inversions explicitly accepted as transitional debt.
    #[serde(default)]
    known_inversions: Vec<KnownInversion>,
}

#[derive(Debug, Deserialize)]
struct CrateEntry {
    layer: u8,
}

#[derive(Debug, Deserialize)]
struct KnownInversion {
    from: String,
    to: String,
    #[allow(dead_code)]
    reason: String,
}

fn main() -> ExitCode {
    let warn_only = std::env::args().any(|a| a == "--warn-only");

    match run(warn_only) {
        Ok(0) => {
            println!("vox-layer-check: clean ✓");
            ExitCode::SUCCESS
        }
        Ok(n) => {
            if warn_only {
                eprintln!(
                    "vox-layer-check: {n} unknown inversion(s) found (warn-only — not failing)"
                );
                ExitCode::SUCCESS
            } else {
                eprintln!("vox-layer-check: {n} unknown inversion(s) — failing");
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("vox-layer-check: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run(_warn_only: bool) -> Result<usize> {
    // Locate workspace root via cargo_metadata.
    let metadata = MetadataCommand::new()
        .no_deps()
        .exec()
        .context("cargo metadata failed")?;

    let workspace_root: PathBuf = metadata.workspace_root.clone().into();
    let layers_path = workspace_root.join("docs/src/architecture/layers.toml");

    let layers_text = std::fs::read_to_string(&layers_path)
        .with_context(|| format!("reading {}", layers_path.display()))?;
    let layers: LayersConfig = toml::from_str(&layers_text)
        .with_context(|| format!("parsing {}", layers_path.display()))?;

    // Workspace member name set, so we ignore deps to non-workspace (external) crates.
    let workspace_members: std::collections::HashSet<&str> = metadata
        .workspace_packages()
        .iter()
        .map(|p| p.name.as_str())
        .collect();

    // Build full metadata (with deps) for edges.
    let metadata_full = MetadataCommand::new()
        .exec()
        .context("cargo metadata (with deps) failed")?;

    let mut unlisted: Vec<String> = Vec::new();
    let mut inversions_found: Vec<(String, String, u8, u8)> = Vec::new();

    for pkg in metadata_full.workspace_packages() {
        let from_name = pkg.name.as_str();
        let from_layer = match layers.crates.get(from_name) {
            Some(e) => e.layer,
            None => {
                unlisted.push(from_name.to_string());
                continue;
            }
        };
        for dep in &pkg.dependencies {
            let to_name = dep.name.as_str();
            if !workspace_members.contains(to_name) {
                continue;
            }
            let to_layer = match layers.crates.get(to_name) {
                Some(e) => e.layer,
                None => continue, // unlisted target: reported separately
            };
            if to_layer > from_layer {
                let is_known = layers
                    .known_inversions
                    .iter()
                    .any(|k| k.from == from_name && k.to == to_name);
                if !is_known {
                    inversions_found.push((
                        from_name.to_string(),
                        to_name.to_string(),
                        from_layer,
                        to_layer,
                    ));
                }
            }
        }
    }

    if !unlisted.is_empty() {
        unlisted.sort();
        unlisted.dedup();
        return Err(anyhow!(
            "{} workspace crate(s) missing from layers.toml: {}",
            unlisted.len(),
            unlisted.join(", ")
        ));
    }

    if !inversions_found.is_empty() {
        eprintln!("Unknown layer inversions:");
        for (from, to, fl, tl) in &inversions_found {
            eprintln!("  {from} (L{fl}) → {to} (L{tl})");
        }
    }
    Ok(inversions_found.len())
}
