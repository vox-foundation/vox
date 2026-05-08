//! `vox bundle` subcommands — list, build, apply, and verify plugin distribution bundles.

pub mod apply;
pub mod build;
pub mod list;
pub mod verify;

use clap::Subcommand;
use std::path::PathBuf;

/// Subcommands for `vox bundle`.
#[derive(Subcommand)]
pub enum BundlePluginCmd {
    /// List all bundle definitions with resolved plugin counts.
    List,
    /// Assemble a distribution tarball for a bundle.
    Build {
        /// Bundle id to build (e.g. `vox-base`, `vox-ml`).
        id: String,
        /// Target triple to build for (default: current host triple).
        #[arg(long, value_name = "TRIPLE")]
        target: Option<String>,
        /// Output path for the generated tarball (default: `<id>-<version>-<triple>.tar.gz`).
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
    },
    /// Extract a bundle tarball to a tempdir and run `vox plugin doctor` against it.
    Verify {
        /// Path to the tarball produced by `vox bundle build`.
        tarball: PathBuf,
    },
    /// Install every plugin in a bundle (resolved through `extends` chain).
    Apply {
        /// Bundle id to apply.
        id: String,
        /// Skip confirmation prompts for each plugin.
        #[arg(long)]
        yes: bool,
    },
}

pub async fn run(cmd: BundlePluginCmd) -> anyhow::Result<()> {
    match cmd {
        BundlePluginCmd::List => list::run(),
        BundlePluginCmd::Build { id, target, out } => {
            build::run(&id, target.as_deref(), out.as_deref())
        }
        BundlePluginCmd::Verify { tarball } => verify::run(&tarball),
        BundlePluginCmd::Apply { id, yes } => apply::run(&id, yes).await,
    }
}
