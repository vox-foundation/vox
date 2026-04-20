//! Shared CLI argument types for database-related publication flows.

use clap::{Args, ValueEnum};
use std::path::PathBuf;

/// Shared `publication-prepare` fields.
#[derive(Args, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PublicationPrepareBodyCli {
    /// Stable publication id.
    #[arg(long)]
    pub publication_id: String,
    /// Author identity.
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
    /// Optional repo-local eval-gate JSON report.
    #[arg(long)]
    pub eval_gate_report_json: Option<PathBuf>,
    /// Optional repo-local benchmark pair JSON report.
    #[arg(long)]
    pub benchmark_pair_report_json: Option<PathBuf>,
    /// Human attestation that the candidate reflects a meaningful advance.
    #[arg(long, default_value_t = false)]
    pub human_meaningful_advance: bool,
    /// Human attestation that AI/generative disclosure is complete.
    #[arg(long, default_value_t = false)]
    pub human_ai_disclosure_complete: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum DbPreflightProfileCli {
    #[default]
    Default,
    DoubleBlind,
    MetadataComplete,
    ArxivAssist,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum DiscoveryIntakeGateCli {
    #[default]
    None,
    StrongSignalsOnly,
    AllowReviewSuggested,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum ScholarlyVenueCli {
    Zenodo,
    OpenReview,
    ArxivAssist,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, serde::Serialize, serde::Deserialize)]
pub enum ArxivHandoffStageCli {
    StagingExported,
    OperatorAck,
    BundleValidated,
    Submitted,
    Published,
}
