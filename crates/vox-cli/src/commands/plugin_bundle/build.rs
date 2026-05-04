//! `vox bundle build <id>` — assemble a distribution tarball for a bundle.
//!
//! # MVP status
//! This implementation performs a **dry-run** listing of what would be included.
//! Actual tarball assembly (finding installed dylibs and packing them with the
//! `tar` + `flate2` crates) is deferred to a follow-up commit.
//! TODO: implement tarball assembly when plugin install artifacts are in place.

use anyhow::{Context, Result};
use std::path::Path;

pub fn run(id: &str, target: Option<&str>, out: Option<&Path>) -> Result<()> {
    let triple = target.unwrap_or_else(|| vox_plugin_host::current_target_triple_key());

    let plugins = vox_plugin_catalog::bundle_resolved(id)
        .with_context(|| format!("resolving bundle '{}'", id))?;

    let out_name = match out {
        Some(p) => p.display().to_string(),
        None => format!("vox-{}-latest-{}.tar.gz", id, triple),
    };

    println!("Bundle : {}", id);
    println!("Target : {}", triple);
    println!("Output : {}", out_name);
    println!();
    println!("Plugins to include ({}):", plugins.len());

    for p in &plugins {
        println!("  - {} ({:?})", p.id, p.payload_kind);
    }

    println!();
    println!(
        "NOTE: Tarball assembly is not yet implemented (MVP dry-run).\n\
         Install the listed plugins with `vox bundle apply {}` first,\n\
         then re-run once tarball assembly is shipped.",
        id
    );

    Ok(())
}
