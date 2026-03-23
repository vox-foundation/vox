//! Clap surface for `vox share`.

use clap::Parser;

use anyhow::Result;

/// Subcommands for `vox share`.
#[derive(Parser)]
pub enum ShareCli {
    /// Publish an artifact (stub).
    Publish {
        /// The type of artifact to publish (e.g. `package`, `model`).
        #[arg(long, default_value = "package")]
        r#type: String,
        /// The canonical name of the artifact.
        #[arg(required = true)]
        name: String,
        /// Content hash for integrity verification.
        #[arg(long, default_value = "")]
        hash: String,
        /// Semantic version string.
        #[arg(required = true)]
        version: String,
        /// Optional longer description string.
        #[arg(long)]
        description: Option<String>,
        /// Optional comma-separated tags for index discovery.
        #[arg(long)]
        tags: Option<String>,
    },
    /// Search the local package index.
    Search {
        /// The query substring.
        #[arg(required = true)]
        query: String,
    },
    /// List artifacts (`package` or `all`).
    List {
        /// The artifact type filter.
        #[arg(default_value = "all")]
        artifact_type: String,
    },
    /// Submit a review (stub).
    Review {
        /// Stable artifact ID.
        #[arg(required = true)]
        artifact_id: String,
        /// Numeric rating out of 10 or 5.
        #[arg(required = true)]
        rating: i64,
        /// Optional free-text feedback.
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
