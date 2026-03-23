//! Clap surface for `vox snippet`.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// Subcommands for `vox snippet`.
#[derive(Parser)]
pub enum SnippetCli {
    /// Save a snippet from a file (store integration in progress).
    Save {
        /// Input file path for the snippet source.
        #[arg(required = true)]
        file: PathBuf,
        /// Primary display title.
        #[arg(required = true)]
        title: String,
        /// Extended context or explanation.
        #[arg(long)]
        description: Option<String>,
        /// Comma-separated indexing strings.
        #[arg(long)]
        tags: Option<String>,
    },
    /// Search saved snippets.
    Search {
        /// Substring query for the search.
        #[arg(required = true)]
        query: String,
    },
    /// Export snippets as JSON (from local/remote Arca store).
    Export {
        /// Maximum number of snippets to export.
        #[arg(long, default_value_t = 100)]
        limit: i64,
    },
}

/// Dispatch `vox snippet …`.
pub async fn run(cmd: SnippetCli) -> Result<()> {
    use super::snippet;
    match cmd {
        SnippetCli::Save {
            file,
            title,
            description,
            tags,
        } => snippet::save(&file, &title, description.as_deref(), tags.as_deref()).await,
        SnippetCli::Search { query } => snippet::search(&query).await,
        SnippetCli::Export { limit } => snippet::export(limit).await,
    }
}
