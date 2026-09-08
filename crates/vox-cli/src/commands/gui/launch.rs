use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

pub async fn run(args: crate::cli_args::GuiArgs) -> Result<()> {
    tracing::info!("Launching Vox Axis (Axis) — the native Vox GUI…");
    let mut cmd = gui_command()?;
    if let Some(cmd_val) = args.command {
        cmd.arg("--command").arg(cmd_val);
    }
    let mut child = cmd.spawn()?;
    child.wait()?;
    Ok(())
}

pub fn gui_command() -> Result<Command> {
    if cfg!(debug_assertions) {
        let mut c = Command::new("cargo");
        c.args(["run", "-p", "vox-gui"]);
        return Ok(c);
    }
    let exe = env::current_exe()?;
    let parent = exe.parent().context("Failed to get executable directory")?;
    let gui_bin_name = if cfg!(windows) {
        "vox-gui.exe"
    } else {
        "vox-gui"
    };
    let installed = parent.join(gui_bin_name);
    let launch_path = if installed.exists() {
        installed
    } else {
        resolve_or_build_gui(&installed, gui_bin_name)?
    };
    Ok(Command::new(launch_path))
}

pub fn spawn_gui_detached(mut cmd: Command) -> Result<Child> {
    cmd.stdin(std::process::Stdio::null());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x00000008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    cmd.spawn().context("spawn vox-gui")
}

fn resolve_or_build_gui(installed: &Path, gui_bin_name: &str) -> Result<PathBuf> {
    let component = vox_plugin_catalog::all_components()
        .iter()
        .find(|c| c.id == "gui")
        .context("no 'gui' component declared in the plugin catalog")?;

    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let os_ok = component.requires.os.is_empty() || component.requires.os.iter().any(|o| o == os);
    let arch_ok =
        component.requires.arch.is_empty() || component.requires.arch.iter().any(|a| a == arch);
    if !os_ok || !arch_ok {
        anyhow::bail!("the Vox GUI component is not available for this platform ({os}/{arch}).");
    }

    let Some(workspace_root) = locate_workspace_root() else {
        anyhow::bail!(gui_missing_no_checkout_message(
            installed,
            &component.default_source,
        ));
    };

    tracing::info!(
        "Vox GUI not installed; building from source (cargo build -p vox-gui --release)…"
    );
    let status = Command::new("cargo")
        .args(["build", "-p", "vox-gui", "--release"])
        .current_dir(&workspace_root)
        .status()
        .context("failed to invoke `cargo build -p vox-gui`")?;
    if !status.success() {
        anyhow::bail!(
            "`cargo build -p vox-gui --release` failed. The GUI is a Tauri app and needs its \
             frontend toolchain (Node + the platform webview deps) and a built `ui/dist`; see \
             crates/vox-gui for setup."
        );
    }

    let target_dir = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root.join("target"));
    let built = target_dir.join("release").join(gui_bin_name);
    if !built.exists() {
        anyhow::bail!(
            "GUI build reported success but no binary was found at {}.",
            built.display()
        );
    }
    if let Some(parent) = installed.parent() {
        let _ = std::fs::create_dir_all(parent);
        if std::fs::copy(&built, installed).is_ok() {
            tracing::info!("Installed Vox GUI to {}", installed.display());
            return Ok(installed.to_path_buf());
        }
    }
    Ok(built)
}

fn locate_workspace_root() -> Option<PathBuf> {
    crate::contributor_mode::locate_workspace_root()
}

fn gui_missing_no_checkout_message(installed: &Path, catalog_source: &str) -> String {
    format!(
        "the Vox GUI is an optional component and is not installed at {}.\n\
         Prebuilt GUI release assets don't ship yet, and this isn't a Vox source \
         checkout — so there's no way to obtain the GUI here. (Building it from \
         source with `cargo build -p vox-gui` is the contributor path, from inside \
         a checkout.)\n\
         Catalog source: {catalog_source}",
        installed.display(),
    )
}

#[cfg(test)]
mod gui_missing_message_tests {
    use super::*;

    #[test]
    fn leads_with_actual_status_not_installed_instruction() {
        let msg = gui_missing_no_checkout_message(
            Path::new("/opt/vox/bin/vox-gui"),
            "https://example.invalid/vox-gui",
        );
        assert!(msg.contains("is not installed at"));
        assert!(msg.contains("don't ship yet"));
        assert!(msg.contains("this isn't a Vox source checkout"));
        assert!(msg.contains("the contributor path"));
        assert!(!msg.contains("Clone the repo"));
        assert!(msg.contains("https://example.invalid/vox-gui"));
    }
}
