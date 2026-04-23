#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScholarlyVenue {
    Zenodo,
    OpenReview,
    ArxivAssist,
}

impl ScholarlyVenue {
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "zenodo" => Some(Self::Zenodo),
            "openreview" => Some(Self::OpenReview),
            "arxiv" | "arxiv_assist" | "arxiv-assist" => Some(Self::ArxivAssist),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zenodo => "zenodo",
            Self::OpenReview => "openreview",
            Self::ArxivAssist => "arxiv_assist",
        }
    }
}

pub struct StagingArtifact {
    pub relative_path: String,
    pub require_non_empty_source: bool,
}

#[must_use]
pub fn staging_artifacts(venue: ScholarlyVenue) -> Vec<StagingArtifact> {
    let mut v = vec![
        StagingArtifact {
            relative_path: "body.md".to_string(),
            require_non_empty_source: true,
        },
        StagingArtifact {
            relative_path: "CITATION.cff".to_string(),
            require_non_empty_source: false,
        },
        StagingArtifact {
            relative_path: "crossref_work.json".to_string(),
            require_non_empty_source: false,
        },
        StagingArtifact {
            relative_path: "citations.json".to_string(),
            require_non_empty_source: false,
        },
    ];
    if matches!(venue, ScholarlyVenue::Zenodo) {
        v.push(StagingArtifact {
            relative_path: "zenodo.json".to_string(),
            require_non_empty_source: false,
        });
    }
    if matches!(venue, ScholarlyVenue::ArxivAssist) {
        v.push(StagingArtifact {
            relative_path: "main.tex".to_string(),
            require_non_empty_source: true,
        });
        v.push(StagingArtifact {
            relative_path: "arxiv_handoff.json".to_string(),
            require_non_empty_source: false,
        });
        v.push(StagingArtifact {
            relative_path: "arxiv_bundle.tar.gz".to_string(),
            require_non_empty_source: false,
        });
    }
    v
}

pub const MAX_STAGING_FILE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Debug)]
pub enum StagingExportError {
    Io(std::io::Error),
    Cff(serde_yaml::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for StagingExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StagingExportError::Io(e) => write!(f, "{e}"),
            StagingExportError::Cff(e) => write!(f, "{e}"),
            StagingExportError::Json(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for StagingExportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StagingExportError::Io(e) => Some(e),
            StagingExportError::Cff(e) => Some(e),
            StagingExportError::Json(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for StagingExportError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_yaml::Error> for StagingExportError {
    fn from(value: serde_yaml::Error) -> Self {
        Self::Cff(value)
    }
}

impl From<serde_json::Error> for StagingExportError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub mod arxiv;
pub mod staging;
pub mod validation;

pub use arxiv::*;
pub use staging::*;
pub use validation::*;
