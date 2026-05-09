//! AcademicCoauthor: record for a human co-author with ORCID and CRediT taxonomy roles.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CreditRole {
    Conceptualization,
    DataCuration,
    FormalAnalysis,
    FundingAcquisition,
    Investigation,
    Methodology,
    ProjectAdministration,
    Resources,
    Software,
    Supervision,
    Validation,
    Visualization,
    WritingOriginalDraft,
    WritingReviewEditing,
}

impl CreditRole {
    pub fn as_uri(&self) -> &'static str {
        match self {
            Self::Conceptualization => {
                "https://credit.niso.org/contributor-roles/conceptualization/"
            }
            Self::DataCuration => "https://credit.niso.org/contributor-roles/data-curation/",
            Self::FormalAnalysis => "https://credit.niso.org/contributor-roles/formal-analysis/",
            Self::FundingAcquisition => {
                "https://credit.niso.org/contributor-roles/funding-acquisition/"
            }
            Self::Investigation => "https://credit.niso.org/contributor-roles/investigation/",
            Self::Methodology => "https://credit.niso.org/contributor-roles/methodology/",
            Self::ProjectAdministration => {
                "https://credit.niso.org/contributor-roles/project-administration/"
            }
            Self::Resources => "https://credit.niso.org/contributor-roles/resources/",
            Self::Software => "https://credit.niso.org/contributor-roles/software/",
            Self::Supervision => "https://credit.niso.org/contributor-roles/supervision/",
            Self::Validation => "https://credit.niso.org/contributor-roles/validation/",
            Self::Visualization => "https://credit.niso.org/contributor-roles/visualization/",
            Self::WritingOriginalDraft => {
                "https://credit.niso.org/contributor-roles/writing-original-draft/"
            }
            Self::WritingReviewEditing => {
                "https://credit.niso.org/contributor-roles/writing-review-editing/"
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcademicCoauthor {
    pub name: String,
    pub orcid: Option<String>,
    pub affiliation: String,
    pub credit_roles: Vec<CreditRole>,
}

impl AcademicCoauthor {
    /// Build a minimal RO-Crate 1.2 Person entity for this co-author.
    pub fn to_ro_crate_entity(&self) -> serde_json::Value {
        let roles: Vec<serde_json::Value> = self
            .credit_roles
            .iter()
            .map(|r| serde_json::json!({ "@id": r.as_uri() }))
            .collect();
        serde_json::json!({
            "@type": "Person",
            "name": self.name,
            "identifier": self.orcid.as_deref().unwrap_or(""),
            "affiliation": { "@type": "Organization", "name": self.affiliation },
            "creditRoles": roles,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coauthor_to_ro_crate_entity() {
        let author = AcademicCoauthor {
            name: "Jane Researcher".into(),
            orcid: Some("https://orcid.org/0000-0000-0000-0001".into()),
            affiliation: "Stanford CRFM".into(),
            credit_roles: vec![CreditRole::Conceptualization, CreditRole::Methodology],
        };
        let entity = author.to_ro_crate_entity();
        assert_eq!(entity["@type"], "Person");
        assert_eq!(entity["name"], "Jane Researcher");
        assert_eq!(entity["affiliation"]["name"], "Stanford CRFM");
        assert!(entity["identifier"].as_str().unwrap().contains("orcid.org"));
    }

    #[test]
    fn credit_role_uris_are_niso() {
        assert!(
            CreditRole::Conceptualization
                .as_uri()
                .contains("credit.niso.org")
        );
        assert!(CreditRole::Software.as_uri().contains("software"));
    }
}
