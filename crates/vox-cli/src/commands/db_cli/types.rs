//! Shared CLI argument types for `vox db` publication flows.

use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// Shared `publication-prepare` / `publication-prepare-validated` fields (no `content_type`).
#[derive(Args, Clone, Debug)]
pub struct PublicationPrepareBodyCli {
    /// Stable publication id.
    #[arg(long)]
    pub publication_id: String,
    /// Author identity (should match `scientific_publication.authors[0].name` when that list is set).
    #[arg(long)]
    pub author: String,
    /// Human title.
    #[arg(long)]
    pub title: String,
    /// Path to markdown body.
    #[arg(required = true)]
    pub path: PathBuf,
    /// Optional abstract text.
    #[arg(long)]
    pub abstract_text: Option<String>,
    /// Optional citations JSON file path.
    #[arg(long)]
    pub citations_json: Option<PathBuf>,
    /// Optional structured scholarly metadata JSON.
    #[arg(long)]
    pub scholarly_metadata_json: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum DbPreflightProfileCli {
    #[default]
    Default,
    DoubleBlind,
}

impl From<DbPreflightProfileCli> for vox_publisher::publication_preflight::PreflightProfile {
    fn from(v: DbPreflightProfileCli) -> Self {
        match v {
            DbPreflightProfileCli::Default => Self::Default,
            DbPreflightProfileCli::DoubleBlind => Self::DoubleBlind,
        }
    }
}
