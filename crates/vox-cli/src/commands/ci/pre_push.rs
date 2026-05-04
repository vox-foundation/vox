//! `vox ci pre-push` — local aggregate that mirrors the merge-blocking CI subset.
//!
//! Runs in order: fmt --check, line-endings, ssot-drift, doc-inventory verify,
//! clippy (workspace, all-targets, -D warnings), scoped TOESTUB (changed paths).
//! `--quick` skips clippy + TOESTUB; `--full` also runs nextest on changed crates.

use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;
use std::time::Instant;

#[derive(Clone, Copy)]
pub struct PrePushOpts {
    pub quick: bool,
    pub full: bool,
    pub dry_run: bool,
}

pub fn run(root: &Path, opts: PrePushOpts) -> Result<()> {
    let _ = (root, opts);
    bail!("pre-push: not yet implemented")
}
