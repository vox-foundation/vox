//! Zenodo REST [`metadata`](https://developers.zenodo.org/) object builder from a [`crate::publication::PublicationManifest`].
//!
//! Suitable for `POST /api/deposit/depositions` (new draft) or for `.zenodo.json` communities/workflows.
//! License ids follow Zenodo’s vocabulary where possible; unknown SPDX values are passed lowercased.

use crate::publication::PublicationManifest;
use crate::publication_preflight::parse_scientific_from_metadata_json;

/// Build the `metadata` object for a new Zenodo deposit draft.
#[must_use]
pub fn zenodo_deposition_metadata(manifest: &PublicationManifest) -> serde_json::Value {
    let scientific = match parse_scientific_from_metadata_json(manifest.metadata_json.as_deref()) {
        Ok(s) => s,
        Err(_) => None,
    };

    let creators: Vec<serde_json::Value> = if let Some(ref sci) = scientific {
        if sci.authors.is_empty() {
            vec![serde_json::json!({ "name": manifest.author })]
        } else {
            sci.authors
                .iter()
                .map(|a| {
                    let mut o = serde_json::json!({ "name": a.name });
                    if let Some(ref aff) = a.affiliation {
                        if !aff.trim().is_empty() {
                            o["affiliation"] = serde_json::Value::String(aff.clone());
                        }
                    }
                    if let Some(ref oid) = a.orcid {
                        let t = oid.trim();
                        if !t.is_empty() {
                            let uri = if t.starts_with("http") {
                                t.to_string()
                            } else {
                                format!("https://orcid.org/{t}")
                            };
                            o["orcid"] = serde_json::Value::String(uri);
                        }
                    }
                    o
                })
                .collect()
        }
    } else {
        vec![serde_json::json!({ "name": manifest.author })]
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

    serde_json::json!({
        "metadata": {
            "title": manifest.title,
            "upload_type": "publication",
            "publication_type": "article",
            "description": description,
            "creators": creators,
            "access_right": "open",
            "license": license,
        }
    })
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
            license_spdx: Some("MIT".to_string()),
            ..Default::default()
        };
        let meta = crate::scientific_metadata::build_scientia_metadata_json("x", None, Some(&sci))
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
        assert_eq!(v["metadata"]["license"], "mit");
        assert!(v["metadata"]["creators"][0]["orcid"]
            .as_str()
            .unwrap()
            .contains("orcid.org"));
    }
}
