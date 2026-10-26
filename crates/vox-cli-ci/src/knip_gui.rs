//! `vox ci knip-gui` — dual unused-export gate for `crates/vox-gui/ui` via knip.
//!
//! Pass 1: files + dependencies. Pass 2: exports. Requires a **local** knip binary
//! when `--check` is set (CI); no `pnpm dlx` pin drift.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

fn which_pnpm() -> Option<PathBuf> {
    let name = if cfg!(windows) { "pnpm.cmd" } else { "pnpm" };
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let cand = dir.join(name);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

fn run_knip(local: &Path, ui: &Path, include: &str) -> Result<()> {
    let status = Command::new(local)
        .current_dir(ui)
        .args(["--config", "knip.jsonc", "--include", include])
        .status()
        .with_context(|| format!("run knip --include {include}"))?;
    if !status.success() {
        bail!("knip --include {include} failed for crates/vox-gui/ui");
    }
    Ok(())
}

pub fn run(root: &Path, check: bool) -> Result<()> {
    let ui = root.join("crates/vox-gui/ui");
    let knip_cfg = ui.join("knip.jsonc");
    if !knip_cfg.is_file() {
        bail!("missing {}", knip_cfg.display());
    }

    let local_knip = ui.join("node_modules/.bin/knip");
    if !local_knip.is_file() {
        if check {
            bail!(
                "local knip missing at {} — run `pnpm install` in crates/vox-gui/ui (dlx forbidden under --check)",
                local_knip.display()
            );
        }
        let Some(pnpm) = which_pnpm() else {
            eprintln!("knip-gui: pnpm not on PATH — skip");
            return Ok(());
        };
        eprintln!("knip-gui: local knip missing — trying `pnpm dlx knip@5.50.0` (dev only)");
        let status = Command::new(&pnpm)
            .current_dir(&ui)
            .args(["dlx", "knip@5.50.0", "--config", "knip.jsonc"])
            .status()
            .context("pnpm dlx knip")?;
        if !status.success() {
            bail!("knip reported unused exports / dependency issues");
        }
        println!("knip-gui: ok (dlx single-pass)");
        return Ok(());
    }

    // Dual pass (t3code analog): files+dependencies, then exports.
    run_knip(&local_knip, &ui, "files,dependencies")?;
    run_knip(&local_knip, &ui, "exports")?;
    println!("knip-gui: ok (dual: files+dependencies, exports)");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knip_config_exists_in_repo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        assert!(root.join("crates/vox-gui/ui/knip.jsonc").is_file());
    }
}
