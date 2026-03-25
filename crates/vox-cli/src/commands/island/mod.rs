//! `vox island` — generate, upgrade, list, and cache v0.dev React islands.
//!
//! Entry point: [`run`]. Dispatches to the four action handlers:
//! * [`actions::generate`] — call v0 API, write TSX, emit Vox stub, optionally build.
//! * [`actions::upgrade`] — re-generate with existing code as context.
//! * [`list_cache::list_islands`] — scan `islands/src/` and `Vox.toml`.
//! * [`list_cache::handle_cache`] — list / clear / remove cache entries.

mod actions;
mod build;
mod list_cache;
mod stub_shadcn;

use anyhow::{Context, Result};

use crate::cli_actions::IslandCli;

pub use build::build_islands;

/// Dispatch `vox island <subcommand>`.
pub async fn run(cmd: IslandCli) -> Result<()> {
    let project_root = std::env::current_dir()
        .context("Cannot determine project root — is the current directory accessible?")?;

    match cmd {
        IslandCli::Generate {
            name,
            prompt,
            target,
            force,
            no_build,
            image,
        } => {
            actions::generate(
                &name,
                &prompt,
                &project_root,
                target.as_deref(),
                force,
                no_build,
                image.as_deref(),
            )
            .await
        }
        IslandCli::Upgrade {
            name,
            prompt,
            no_build,
        } => actions::upgrade(&name, &prompt, &project_root, no_build).await,
        IslandCli::List { json } => list_cache::list_islands(&project_root, json),
        IslandCli::Add { component, from } => {
            stub_shadcn::add_shadcn(&component, &project_root, from.as_deref()).await
        }
        IslandCli::Cache { action } => list_cache::handle_cache(action),
    }
}
