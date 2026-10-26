//! `vox ci knip-gui` — unused-export gate for `crates/vox-gui/ui` via knip.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub fn run(root: &Path, check: bool) -> Result<()> {
    let ui = root.join("crates/vox-gui/ui");
    let knip_cfg = ui.join("knip.jsonc");
    if !knip_cfg.is_file() {
        bail!("missing {}", knip_cfg.display());
    }

    let pnpm = which_pnpm();
    let Some(pnpm) = pnpm else {
        if check {
            bail!("pnpm not found; cannot run knip-gui --check");
        }
        eprintln!("knip-gui: pnpm not on PATH — skip (install pnpm to enable)");
        return Ok(());
    };

    // Prefer local knip from ui node_modules; fall back to pnpm dlx.
    let local_knip = ui.join("node_modules/.bin/knip");
    let status = if local_knip.is_file() {
        Command::new(&local_knip)
            .current_dir(&ui)
            .arg("--config")
            .arg("knip.jsonc")
            .status()
            .context("run local knip")?
    } else {
        eprintln!("knip-gui: local knip missing — trying `pnpm dlx knip@5`");
        Command::new(&pnpm)
            .current_dir(&ui)
            .args(["dlx", "knip@5", "--config", "knip.jsonc"])
            .status()
            .context("pnpm dlx knip")?
    };

    if !status.success() {
        bail!("knip reported unused exports / dependency issues in crates/vox-gui/ui");
    }
    println!("knip-gui: ok");
    Ok(())
}

fn which_pnpm() -> Option<std::path::PathBuf> {
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
