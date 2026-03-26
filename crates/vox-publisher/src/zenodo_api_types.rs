//! Typed JSON for Zenodo Deposit API (subset used by scholarly adapter).

use serde::{Deserialize, Serialize};

/// Body for `POST /api/deposit/depositions`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenodoDepositionCreateBody {
    pub metadata: ZenodoDepositionMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenodoDepositionMetadata {
    pub title: String,
    pub upload_type: String,
    pub publication_type: String,
    pub description: String,
    pub creators: Vec<ZenodoCreator>,
    pub access_right: String,
    pub license: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenodoCreator {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affiliation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orcid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZenodoDepositionLinks {
    #[serde(default)]
    pub bucket: Option<String>,
}

/// Deposition resource returned by create, get, and publish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZenodoDeposition {
    pub id: u64,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub doi: Option<String>,
    #[serde(default)]
    pub links: Option<ZenodoDepositionLinks>,
}
