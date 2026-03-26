//! JSON export bundle aligned with Crossref-style **work** metadata (informative; not a full deposit).
//!
//! Use for operator review, auxiliary manifests, or future deposit tooling — Crossref production
//! deposits typically use validated XML. This object is versioned for stability in tests and
//! downstream consumers.

use crate::publication::PublicationManifest;
use crate::publication_preflight::parse_scientific_from_metadata_json;

/// Map common SPDX identifiers to a public license URL when known.
#[must_use]
pub fn spdx_license_url(spdx: &str) -> Option<&'static str> {
    if spdx.eq_ignore_ascii_case("MIT") {
        return Some("https://opensource.org/licenses/MIT");
    }
    if spdx.eq_ignore_ascii_case("Apache-2.0")
        || spdx.eq_ignore_ascii_case("Apache-2")
    {
        return Some("https://www.apache.org/licenses/LICENSE-2.0");
    }
    if spdx.eq_ignore_ascii_case("CC-BY-4.0") {
        return Some("https://creativecommons.org/licenses/by/4.0/legalcode");
    }
    if spdx.eq_ignore_ascii_case("CC-BY-SA-4.0") {
        return Some("https://creativecommons.org/licenses/by-sa/4.0/legalcode");
    }
    if spdx.eq_ignore_ascii_case("BSD-3-Clause") {
        return Some("https://opensource.org/licenses/BSD-3-Clause");
    }
    None
}

/// Crossref-oriented work metadata as JSON (`schema` is a Vox extension label).
#[must_use]
pub fn crossref_work_export_json(manifest: &PublicationManifest) -> serde_json::Value {
    let scientific = parse_scientific_from_metadata_json(manifest.metadata_json.as_deref())
        .ok()
        .flatten();

    let contributors: Vec<serde_json::Value> = if let Some(ref sci) = scientific {
        if sci.authors.is_empty() {
            vec![serde_json::json!({
                "role": "author",
                "name": manifest.author,
            })]
        } else {
            sci.authors
                .iter()
                .map(|a| {
                    let mut o = serde_json::json!({
                        "role": "author",
                        "name": a.name,
                    });
                    if let Some(ref aff) = a.affiliation {
                        let t = aff.trim();
                        if !t.is_empty() {
                            o["affiliation"] = serde_json::Value::String(t.to_string());
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
        vec![serde_json::json!({
            "role": "author",
            "name": manifest.author,
        })]
    };

    let license_spdx = scientific
        .as_ref()
        .and_then(|s| s.license_spdx.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    let license_url = license_spdx.as_deref().and_then(spdx_license_url);

    let funding = scientific
        .as_ref()
        .and_then(|s| s.funding_statement.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    let competing_interests = scientific
        .as_ref()
        .and_then(|s| s.competing_interests_statement.as_deref())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(std::string::ToString::to_string);

    serde_json::json!({
        "schema": "vox.crossref_work_metadata.v1",
        "work_type": "journal-article",
        "title": manifest.title,
        "abstract": manifest.abstract_text,
        "language": "en",
        "contributors": contributors,
        "license": {
            "spdx": license_spdx,
            "url": license_url,
        },
        "funding_statement": funding,
        "competing_interests_statement": competing_interests,
        "source_ref": manifest.source_ref,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crossref_export_includes_title_and_mit_license_url() {
        let sci = crate::scientific_metadata::ScientificPublicationMetadata {
            authors: vec![crate::scientific_metadata::ScientificAuthor {
                name: "A. Person".to_string(),
                orcid: None,
                affiliation: None,
            }],
            license_spdx: Some("MIT".to_string()),
            ..Default::default()
        };
        let meta = crate::scientific_metadata::build_scientia_metadata_json(
            "p",
            None,
            Some(&sci),
            None,
        )
        .unwrap();
        let m = PublicationManifest {
            publication_id: "x".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "Paper".to_string(),
            author: "A. Person".to_string(),
            abstract_text: Some("Abs".to_string()),
            body_markdown: String::new(),
            citations_json: None,
            metadata_json: Some(meta),
        };
        let v = crossref_work_export_json(&m);
        assert_eq!(v["schema"], "vox.crossref_work_metadata.v1");
        assert_eq!(v["title"], "Paper");
        assert_eq!(v["license"]["url"], "https://opensource.org/licenses/MIT");
        assert_eq!(v["contributors"][0]["name"], "A. Person");
    }
}
