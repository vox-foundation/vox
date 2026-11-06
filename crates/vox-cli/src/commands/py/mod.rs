//! `vox py sync` and `vox py doctor`.

mod env;

pub use env::{PythonConfig, discover_python, venv_python};

use anyhow::Result;
use clap::Subcommand;

#[derive(Debug, Subcommand)]
pub enum PyCmd {
    /// Write `.vox/python` from `[python]` and install it with uv.
    Sync,
    /// Report uv, Python, packages, the uv.lock hash, and sandbox status.
    Doctor,
}

pub async fn run(cmd: PyCmd) -> Result<()> {
    let cwd = std::env::current_dir()?;
    match cmd {
        PyCmd::Sync => {
            let hash = env::sync_project(&cwd)?;
            println!("vox py sync: recorded uv.lock {hash}");
        }
        PyCmd::Doctor => {
            let report = env::doctor_report(&cwd)?;
            print!("{report}");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::PyCmd;
    use crate::VoxCliRoot;

    #[test]
    fn py_sync_and_doctor_parse() {
        let sync = VoxCliRoot::try_parse_from(["vox", "py", "sync"]).unwrap();
        assert!(matches!(sync.cmd, crate::Cli::Py { cmd: PyCmd::Sync }));
        let doctor = VoxCliRoot::try_parse_from(["vox", "py", "doctor"]).unwrap();
        assert!(matches!(doctor.cmd, crate::Cli::Py { cmd: PyCmd::Doctor }));
    }

    /// `vox run` of the golden in a temp project that has `[python]`.
    #[test]
    fn python_bridge_vox_run_prints_version() {
        if which::which("uv").is_err() || which::which("python3").is_err() {
            eprintln!("skip: uv or python3 is absent");
            return;
        }
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Vox.toml"),
            "[python]\nversion = \"3.12\"\n",
        )
        .expect("vox.toml");
        let golden = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/golden/py_bridge_hello.vox");
        let script = dir.path().join("hello.vox");
        std::fs::copy(&golden, &script).expect("copy golden");
        super::env::sync_project(dir.path()).expect("vox py sync");

        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime");
        vox_compiler::eval::python::start_print_capture();
        let ran = rt.block_on(async {
            crate::commands::run::run(
                &script,
                &[],
                crate::commands::run::RunMode::Interp,
                &[],
                None,
                None,
                None,
                None,
                None,
            )
            .await
        });
        let lines = vox_compiler::eval::python::stop_print_capture();
        ran.expect("vox run");
        assert!(
            lines.iter().any(|line| {
                line.contains('.')
                    && line
                        .split('.')
                        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
            }),
            "expected the worker Python version, got {lines:?}"
        );
    }
}
