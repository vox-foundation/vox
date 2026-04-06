//! Zenodo REST [`metadata`](https://developers.zenodo.org/) object builder from a [`crate::publication::PublicationManifest`].
//!
//! Suitable for `POST /api/deposit/depositions` (new draft) or for `.zenodo.json` communities/workflows.
//! License ids follow Zenodo’s vocabulary where possible; unknown SPDX values are passed lowercased.

use crate::publication::PublicationManifest;
use crate::publication_preflight::parse_scientific_from_metadata_json;
use crate::zenodo_api_types::{
    ZenodoCreator, ZenodoDepositionCreateBody, ZenodoDepositionMetadata,
};

/// Build the typed POST body for a new Zenodo deposit draft.
#[must_use]
pub fn zenodo_deposition_create_body(manifest: &PublicationManifest) -> ZenodoDepositionCreateBody {
    let scientific = parse_scientific_from_metadata_json(manifest.metadata_json.as_deref())
        .ok()
        .flatten();

    let creators: Vec<ZenodoCreator> = if let Some(ref sci) = scientific {
        if sci.authors.is_empty() {
            vec![ZenodoCreator {
                name: manifest.author.clone(),
                affiliation: None,
                orcid: None,
            }]
        } else {
            sci.authors
                .iter()
                .map(|a| {
                    let affiliation = a
                        .affiliation
                        .as_deref()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(std::string::ToString::to_string);
                    let orcid = a.orcid.as_deref().and_then(|oid| {
                        let t = oid.trim();
                        if t.is_empty() {
                            return None;
                        }
                        let uri = if t.starts_with("http") {
                            t.to_string()
                        } else {
                            format!("https://orcid.org/{t}")
                        };
                        Some(uri)
                    });
                    ZenodoCreator {
                        name: a.name.clone(),
                        affiliation,
                        orcid,
                    }
                })
                .collect()
        }
    } else {
        vec![ZenodoCreator {
            name: manifest.author.clone(),
            affiliation: None,
            orcid: None,
        }]
    };

    let description = manifest
        .abstract_text
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| {
            let body = manifest.body_markdown.trim();
            if body.len() <= 4000 {
                body.to_string()
            } else {
                format!("{}…", body.chars().take(3990).collect::<String>())
            }
        });

    let license = scientific
        .as_ref()
        .and_then(|s| s.license_spdx.as_deref())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "notspecified".to_string());

    ZenodoDepositionCreateBody {
        metadata: ZenodoDepositionMetadata {
            title: manifest.title.clone(),
            upload_type: "publication".to_string(),
            publication_type: "article".to_string(),
            description,
            creators,
            access_right: "open".to_string(),
            license,
        },
    }
}

/// Build the JSON envelope for a new Zenodo deposit draft (compat for callers expecting [`serde_json::Value`]).
#[must_use]
pub fn zenodo_deposition_metadata(manifest: &PublicationManifest) -> serde_json::Value {
    serde_json::to_value(zenodo_deposition_create_body(manifest)).unwrap_or_else(|_| {
        serde_json::json!({ "metadata": { "title": manifest.title, "upload_type": "publication" } })
    })
}

/// Pretty JSON for a sidecar `.zenodo.json` file (same envelope as [`zenodo_deposition_metadata`]).
pub fn zenodo_json_pretty(manifest: &PublicationManifest) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(&zenodo_deposition_create_body(manifest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scientific_metadata::{ScientificAuthor, ScientificPublicationMetadata};

    #[test]
    fn zenodo_includes_creators_and_license() {
        let sci = ScientificPublicationMetadata {
            authors: vec![ScientificAuthor {
                name: "A".to_string(),
                orcid: Some("0000-0001-2345-6789".to_string()),
                affiliation: Some("U".to_string()),
            }],
            license_spdx: Some("Apache-2.0".to_string()),
            ..Default::default()
        };
        let meta =
            crate::scientific_metadata::build_scientia_metadata_json("x", None, Some(&sci), None)
                .unwrap();
        let m = PublicationManifest {
            publication_id: "p".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "T".to_string(),
            author: "A".to_string(),
            abstract_text: Some("Abs".to_string()),
            body_markdown: "b".to_string(),
            citations_json: None,
            metadata_json: Some(meta),
        };
        let v = zenodo_deposition_metadata(&m);
        assert_eq!(v["metadata"]["title"], "T");
        assert_eq!(v["metadata"]["license"], "apache-2.0");
        assert!(
            v["metadata"]["creators"][0]["orcid"]
                .as_str()
                .unwrap()
                .contains("orcid.org")
        );
    }

    #[test]
    fn zenodo_json_pretty_round_trips() {
        let m = PublicationManifest {
            publication_id: "p".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "T2".to_string(),
            author: "B".to_string(),
            abstract_text: None,
            body_markdown: "body".to_string(),
            citations_json: None,
            metadata_json: None,
        };
        let s = zenodo_json_pretty(&m).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["metadata"]["title"], "T2");
    }
}
