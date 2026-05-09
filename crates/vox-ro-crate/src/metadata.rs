use serde_json::{json, Value};

pub struct RoCrateMetadata {
    pub name: String,
    pub description: String,
    pub doi: Option<String>,
    pub author_orcid: Option<String>,    // e.g. "https://orcid.org/0000-0001-2345-6789"
    pub author_ror: Option<String>,      // e.g. "https://ror.org/03yrm5c26"
    pub license_spdx: String,            // e.g. "CC-BY-4.0"
    pub published_at: i64,               // Unix timestamp
    pub keywords: Vec<String>,
}

pub fn build_ro_crate_json(metadata: &RoCrateMetadata) -> Value {
    // Format ISO date from Unix timestamp (seconds since epoch, UTC)
    let iso_date = format_iso_date(metadata.published_at);

    let license_url = format!("https://spdx.org/licenses/{}", metadata.license_spdx);

    let author_id = metadata.author_orcid.clone()
        .unwrap_or_else(|| "#anonymous-author".to_string());

    // Build identifier array (include DOI if present)
    let mut identifiers: Vec<Value> = Vec::new();
    if let Some(doi) = &metadata.doi {
        let doi_url = if doi.starts_with("http") {
            doi.clone()
        } else {
            format!("https://doi.org/{}", doi)
        };
        identifiers.push(json!({ "@id": doi_url }));
    }

    // Build author node
    let mut author_node = json!({
        "@id": author_id,
        "@type": "Person"
    });
    if let Some(ror) = &metadata.author_ror {
        author_node["affiliation"] = json!({ "@id": ror });
    }

    // Descriptor node (ro-crate-metadata.json itself)
    let descriptor = json!({
        "@id": "ro-crate-metadata.json",
        "@type": "CreativeWork",
        "about": { "@id": "./" },
        "conformsTo": { "@id": "https://w3id.org/ro/crate/1.2" }
    });

    // Root dataset node
    let mut dataset = json!({
        "@id": "./",
        "@type": "Dataset",
        "name": metadata.name,
        "description": metadata.description,
        "license": { "@id": license_url },
        "author": [{ "@id": author_id }],
        "datePublished": iso_date,
        "keywords": metadata.keywords
    });

    if !identifiers.is_empty() {
        dataset["identifier"] = json!(identifiers);
    }

    let mut graph = vec![descriptor, dataset];
    // Only include author node if we have a real ORCID or ROR
    if metadata.author_orcid.is_some() || metadata.author_ror.is_some() {
        graph.push(author_node);
    }

    json!({
        "@context": "https://w3id.org/ro/crate/1.2/context",
        "@graph": graph
    })
}

/// Format a Unix timestamp as an ISO 8601 date string (YYYY-MM-DD).
fn format_iso_date(unix_ts: i64) -> String {
    // Simple implementation: days since epoch
    let days = unix_ts / 86400;
    let mut year = 1970i64;
    let mut remaining_days = days;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1usize;
    for &md in &month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        month += 1;
    }
    let day = remaining_days + 1;

    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_metadata() -> RoCrateMetadata {
        RoCrateMetadata {
            name: "Test Dataset".to_string(),
            description: "A test dataset".to_string(),
            doi: None,
            author_orcid: Some("https://orcid.org/0000-0001-2345-6789".to_string()),
            author_ror: None,
            license_spdx: "CC-BY-4.0".to_string(),
            published_at: 1_700_000_000,
            keywords: vec!["test".to_string()],
        }
    }

    #[test]
    fn ro_crate_has_required_context() {
        let json = build_ro_crate_json(&sample_metadata());
        assert_eq!(json["@context"], "https://w3id.org/ro/crate/1.2/context");
    }

    #[test]
    fn ro_crate_graph_contains_dataset() {
        let json = build_ro_crate_json(&sample_metadata());
        let graph = json["@graph"].as_array().unwrap();
        let has_dataset = graph.iter().any(|node| {
            node["@type"].as_str() == Some("Dataset")
        });
        assert!(has_dataset);
    }

    #[test]
    fn doi_included_when_present() {
        let mut meta = sample_metadata();
        meta.doi = Some("10.1234/test".to_string());
        let json = build_ro_crate_json(&meta);
        let graph = json["@graph"].as_array().unwrap();
        let dataset = graph.iter().find(|n| n["@type"] == "Dataset").unwrap();
        assert!(dataset.get("identifier").is_some());
    }
}
