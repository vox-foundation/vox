//! Clap surface for `vox share`.

use clap::Parser;

use anyhow::Result;

/// Subcommands for `vox share`.
#[derive(Parser)]
pub enum ShareCli {
    /// Publish an artifact (stub).
    Publish {
        #[arg(long, default_value = "package")]
        r#type: String,
        #[arg(required = true)]
        name: String,
        #[arg(long, default_value = "")]
        hash: String,
        #[arg(required = true)]
        version: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        tags: Option<String>,
    },
    /// Search the local package index.
    Search {
        #[arg(required = true)]
        query: String,
    },
    /// List artifacts (`package` or `all`).
    List {
        #[arg(default_value = "all")]
        artifact_type: String,
    },
    /// Submit a review (stub).
    Review {
        #[arg(required = true)]
        artifact_id: String,
        #[arg(required = true)]
        rating: i64,
        #[arg(long)]
        comment: Option<String>,
    },
}

/// Dispatch `vox share …`.
pub async fn run(cmd: ShareCli) -> Result<()> {
    use super::share;
    match cmd {
        ShareCli::Publish {
            r#type,
            name,
            hash,
            version,
            description,
            tags,
        } => {
            share::publish(
                &r#type,
                &name,
                &hash,
                &version,
                description.as_deref(),
                tags.as_deref(),
            )
            .await
        }
        ShareCli::Search { query } => share::search(&query).await,
        ShareCli::List { artifact_type } => share::list(&artifact_type).await,
        ShareCli::Review {
            artifact_id,
            rating,
            comment,
        } => share::review(&artifact_id, rating, comment.as_deref()).await,
    }
}
