//! `vox ci gui-smoke` — deterministic WebIR lowering tests plus opt-in Vite / Playwright integration lanes.

use std::path::Path;
use std::process::Command;

use anyhow::{Result, anyhow};

use super::cargo_bin;

/// Run the GUI smoke bundle from repo `root`.
pub fn run(root: &Path) -> Result<()> {
    let cargo = cargo_bin();

    let st = Command::new(&cargo)
        .current_dir(root)
        .args(["test", "-p", "vox-compiler", "--test", "web_ir_lower_emit"])
        .status()?;
    if !st.success() {
        return Err(anyhow!(
            "gui-smoke: `cargo test -p vox-compiler --test web_ir_lower_emit` failed"
        ));
    }
    println!("gui-smoke: web_ir_lower_emit OK");

    if std::env::var("VOX_WEB_VITE_SMOKE").ok().as_deref() == Some("1") {
        let st = Command::new(&cargo)
            .current_dir(root)
            .env("VOX_WEB_VITE_SMOKE", "1")
            .args([
                "test",
                "-p",
                "vox-integration-tests",
                "--test",
                "web_vite_smoke",
                "--",
                "--ignored",
                "--nocapture",
            ])
            .status()?;
        if !st.success() {
            return Err(anyhow!(
                "gui-smoke: Vite `web_vite_smoke` failed (see `VOX_WEB_VITE_SMOKE`)"
            ));
        }
        println!("gui-smoke: Vite web_vite_smoke OK");
    } else {
        println!("gui-smoke: skip Vite lane (set VOX_WEB_VITE_SMOKE=1 to run `web_vite_smoke`)");
    }

    if std::env::var("VOX_GUI_PLAYWRIGHT").ok().as_deref() == Some("1") {
        let st = Command::new(&cargo)
            .current_dir(root)
            .env("VOX_GUI_PLAYWRIGHT", "1")
            .args([
                "test",
                "-p",
                "vox-integration-tests",
                "--test",
                "playwright_golden_route",
                "--",
                "--ignored",
                "--nocapture",
            ])
            .status()?;
        if !st.success() {
            return Err(anyhow!(
                "gui-smoke: Playwright `playwright_golden_route` failed (set browsers + `pnpm install` under `crates/vox-integration-tests`)"
            ));
        }
        println!("gui-smoke: Playwright golden_route OK");
    } else {
        println!(
            "gui-smoke: skip Playwright lane (set VOX_GUI_PLAYWRIGHT=1 on browser-capable runners)"
        );
    }

    Ok(())
}
