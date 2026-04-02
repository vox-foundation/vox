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
    pub title: Option<String>,
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
    /// Optional repo-local eval-gate JSON report to seed `scientia_evidence`.
    #[arg(long)]
    pub eval_gate_report_json: Option<PathBuf>,
    /// Optional repo-local benchmark pair JSON report to seed `scientia_evidence`.
    #[arg(long)]
    pub benchmark_pair_report_json: Option<PathBuf>,
    /// Human attestation that the candidate reflects a meaningful advance.
    #[arg(long, default_value_t = false)]
    pub human_meaningful_advance: bool,
    /// Human attestation that AI/generative disclosure is complete for the target venue.
    #[arg(long, default_value_t = false)]
    pub human_ai_disclosure_complete: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum DbPreflightProfileCli {
    #[default]
    Default,
    DoubleBlind,
    /// Require structured scientific metadata, license, abstract, and at least one author.
    MetadataComplete,
    /// arXiv-assist path: error if abstract is missing (other checks like default / warnings).
    ArxivAssist,
}

impl From<DbPreflightProfileCli> for vox_publisher::publication_preflight::PreflightProfile {
    fn from(v: DbPreflightProfileCli) -> Self {
        match v {
            DbPreflightProfileCli::Default => Self::Default,
            DbPreflightProfileCli::DoubleBlind => Self::DoubleBlind,
            DbPreflightProfileCli::MetadataComplete => Self::MetadataComplete,
            DbPreflightProfileCli::ArxivAssist => Self::ArxivAssist,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum)]
pub enum DiscoveryIntakeGateCli {
    /// No gate (default).
    #[default]
    None,
    /// Require at least one strong discovery signal and no structured conflicts.
    StrongSignalsOnly,
    /// Allow strong or review-suggested tiers only (block low-signal).
    AllowReviewSuggested,
}

impl From<DiscoveryIntakeGateCli> for vox_publisher::scientia_discovery::DiscoveryIntakeGate {
    fn from(v: DiscoveryIntakeGateCli) -> Self {
        match v {
            DiscoveryIntakeGateCli::None => Self::None,
            DiscoveryIntakeGateCli::StrongSignalsOnly => Self::StrongSignalsOnly,
            DiscoveryIntakeGateCli::AllowReviewSuggested => Self::AllowReviewSuggested,
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
