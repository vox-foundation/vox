//! [Citation File Format](https://citation-file-format.github.io/) (1.2.0) export from a
//! [`crate::publication::PublicationManifest`] and optional embedded scientific metadata.

use serde::Serialize;

use crate::publication::PublicationManifest;
use crate::publication_preflight::parse_scientific_from_metadata_json;

#[derive(Serialize)]
struct CitationCffAuthor {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    orcid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    affiliation: Option<String>,
}

#[derive(Serialize)]
struct CitationCffRoot {
    #[serde(rename = "cff-version")]
    cff_version: &'static str,
    title: String,
    message: &'static str,
    authors: Vec<CitationCffAuthor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license: Option<String>,
    #[serde(rename = "repository-code", skip_serializing_if = "Option::is_none")]
    repository_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    r#abstract: Option<String>,
}

/// Render `CITATION.cff` YAML (version 1.2.0) for the manifest.
///
/// Authors are taken from `metadata_json.scientific_publication.authors` when present; otherwise a
/// single author uses [`PublicationManifest::author`].
#[must_use]
pub fn render_citation_cff(manifest: &PublicationManifest) -> Result<String, serde_yaml::Error> {
    let scientific = parse_scientific_from_metadata_json(manifest.metadata_json.as_deref())
        .ok()
        .flatten();

    let authors: Vec<CitationCffAuthor> = if let Some(ref sci) = scientific {
        if sci.authors.is_empty() {
            vec![CitationCffAuthor {
                name: manifest_author_name(&manifest.author),
                orcid: None,
                affiliation: None,
            }]
        } else {
            sci.authors
                .iter()
                .map(|a| CitationCffAuthor {
                    name: manifest_author_name(&a.name),
                    orcid: a
                        .orcid
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(std::string::ToString::to_string),
                    affiliation: a
                        .affiliation
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(std::string::ToString::to_string),
                })
                .collect()
        }
    } else {
        vec![CitationCffAuthor {
            name: manifest_author_name(&manifest.author),
            orcid: None,
            affiliation: None,
        }]
    };

    let license = scientific
        .as_ref()
        .and_then(|s| s.license_spdx.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    let repository_code = scientific
        .as_ref()
        .and_then(|s| s.reproducibility.as_ref())
        .and_then(|r| r.code_repository_url.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    let r#abstract = manifest
        .abstract_text
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    let root = CitationCffRoot {
        cff_version: "1.2.0",
        title: manifest.title.trim().to_string(),
        message: "If you use this work, please cite it using the metadata from this file.",
        authors,
        license,
        repository_code,
        r#abstract,
    };

    serde_yaml::to_string(&root)
}

fn manifest_author_name(s: &str) -> String {
    let t = s.trim();
    if t.is_empty() {
        "Unknown".to_string()
    } else {
        t.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scientific_metadata::{ScientificAuthor, ScientificPublicationMetadata};

    #[test]
    fn citation_cff_contains_title_and_spdx_license() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "A. Person".to_string(),
                orcid: Some("0000-0002-1825-0097".to_string()),
                affiliation: Some("Example Univ".to_string()),
            }],
            license_spdx: Some("MIT".to_string()),
            funding_statement: None,
            competing_interests_statement: None,
            reproducibility: None,
            ethics_and_impact: None,
        };
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("t", None, Some(&sci), None)
                .unwrap();
        let m = PublicationManifest {
            publication_id: "pub1".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "On Examples".to_string(),
            author: "Ignored When Scientific".to_string(),
            abstract_text: Some("Short abstract.".to_string()),
            body_markdown: String::new(),
            citations_json: None,
            metadata_json: Some(meta),
        };
        let y = render_citation_cff(&m).unwrap();
        assert!(y.contains("cff-version: 1.2.0"));
        assert!(y.contains("title: On Examples"));
        assert!(y.contains("name: A. Person"));
        assert!(y.contains("MIT"));
        assert!(y.contains("Short abstract."));
    }
}
