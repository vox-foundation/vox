pub mod publisher;
pub use publisher::*;

pub mod adapters;
pub mod citation_cff;
pub mod contract;
pub mod crossref_metadata;
pub mod gate;
pub mod openreview_api_types;
pub mod publication;
pub mod publication_preflight;
pub mod publication_worthiness;
pub mod scientia_contracts;
pub mod scientia_discovery;
pub mod scientia_evidence;
pub mod scientia_finding_ledger;
pub mod scientia_heuristics;
pub mod scientia_prior_art;

pub mod scholarly;
#[cfg(feature = "scholarly-external-jobs")]
pub use crate::scholarly::external as scholarly_external_jobs;
#[cfg(feature = "scholarly-external-jobs")]
pub mod scholarly_remote_status;
#[cfg(feature = "scholarly-external-jobs")]
pub mod scientia_worthiness_enrich;
pub mod scientific_metadata;
pub mod submission;
pub mod switching;
pub mod templates;
pub mod topic_packs;
pub mod distribution_compile;
pub mod types;
pub mod zenodo_api_types;
pub mod zenodo_metadata;

mod social_retry;
mod syndication_outcome;
pub mod adapter_health;

pub use syndication_outcome::{ChannelOutcome, SyndicationResult};
pub use topic_packs::{apply_topic_pack_from_metadata_json, hydrate_syndication_from_pack_id};
pub use distribution_compile::{
    compile_for_publish, ChannelPlan, DistributionCompileReport,
};

pub use contract::NewsSiteConfig;
