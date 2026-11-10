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
