//! `vox plugin` — install, remove, list, and inspect Vox plugins.

pub mod doctor;
pub mod info;
pub mod install;
pub mod list;
pub mod remove;

use clap::Subcommand;
use std::path::PathBuf;

/// Subcommands for `vox plugin`.
#[derive(Subcommand)]
pub enum PluginCmd {
    /// List all catalog entries with install status.
    List,
    /// Show manifest and install details for a plugin.
    Info {
        /// Plugin id, e.g. `noop-skill` or `mens-candle-cuda`.
        id: String,
    },
    /// Install a plugin from the catalog, a local directory, or a URL.
    Install {
        /// Plugin id (catalog install) — omit when using `--path` or `--url`.
        id: Option<String>,
        /// Install from a local directory containing Plugin.toml.
        #[arg(long, value_name = "DIR")]
        path: Option<PathBuf>,
        /// Install from an HTTPS URL pointing to a `.zip` archive.
        #[arg(long, value_name = "URL")]
        url: Option<String>,
        /// Skip the confirmation prompt.
        #[arg(long)]
        yes: bool,
    },
    /// Remove an installed plugin.
    Remove {
        /// Plugin id to remove.
        id: String,
    },
    /// Check installed plugins for ABI version drift and missing native libs.
    Doctor,
}

pub async fn run(cmd: PluginCmd) -> anyhow::Result<()> {
    match cmd {
        PluginCmd::List => list::run(),
        PluginCmd::Info { id } => info::run(&id),
        PluginCmd::Install {
            id,
            path,
            url,
            yes,
        } => install::run(id.as_deref(), path.as_deref(), url.as_deref(), yes).await,
        PluginCmd::Remove { id } => remove::run(&id),
        PluginCmd::Doctor => doctor::run(),
    }
}
