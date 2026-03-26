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
    /// Require structured scientific metadata, license, abstract, and at least one author.
    MetadataComplete,
}

impl From<DbPreflightProfileCli> for vox_publisher::publication_preflight::PreflightProfile {
    fn from(v: DbPreflightProfileCli) -> Self {
        match v {
            DbPreflightProfileCli::Default => Self::Default,
            DbPreflightProfileCli::DoubleBlind => Self::DoubleBlind,
            DbPreflightProfileCli::MetadataComplete => Self::MetadataComplete,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ScholarlyVenueCli {
    Zenodo,
    OpenReview,
    ArxivAssist,
}

impl ScholarlyVenueCli {
    #[must_use]
    pub fn to_venue(self) -> vox_publisher::submission_package::ScholarlyVenue {
        match self {
            ScholarlyVenueCli::Zenodo => vox_publisher::submission_package::ScholarlyVenue::Zenodo,
            ScholarlyVenueCli::OpenReview => {
                vox_publisher::submission_package::ScholarlyVenue::OpenReview
            }
            ScholarlyVenueCli::ArxivAssist => {
                vox_publisher::submission_package::ScholarlyVenue::ArxivAssist
            }
        }
    }
}

/// Operator milestone for the arXiv-assist handoff (recorded under `arxiv_handoff:*` status).
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ArxivHandoffStageCli {
    /// Staging bundle (`arxiv_handoff.json` + `arxiv_bundle.tar.gz`) was produced.
    StagingExported,
    /// Operator took custody of the bundle for manual upload.
    OperatorAck,
    /// Operator validated bundle contents (checksums, file layout).
    BundleValidated,
    /// Submitted via arXiv UI (identifier may still be pending).
    Submitted,
    /// Live on arXiv; requires `--arxiv-id`.
    Published,
}

impl ArxivHandoffStageCli {
    #[must_use]
    pub fn slug(self) -> &'static str {
        match self {
            Self::StagingExported => "staging_exported",
            Self::OperatorAck => "operator_ack",
            Self::BundleValidated => "bundle_validated",
            Self::Submitted => "submitted",
            Self::Published => "published",
        }
    }
}
