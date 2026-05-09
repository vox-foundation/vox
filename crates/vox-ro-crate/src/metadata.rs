use serde_json::{Value, json};

pub struct RoCrateMetadata {
    pub name: String,
    pub description: String,
    pub doi: Option<String>,
    pub author_orcid: Option<String>, // e.g. "https://orcid.org/0000-0001-2345-6789"
    pub author_ror: Option<String>,   // e.g. "https://ror.org/03yrm5c26"
    pub license_spdx: String,         // e.g. "CC-BY-4.0"
    pub published_at: i64,            // Unix timestamp
    pub keywords: Vec<String>,
}

pub fn build_ro_crate_json(metadata: &RoCrateMetadata) -> Value {
    // Format ISO date from Unix timestamp (seconds since epoch, UTC)
    let iso_date = format_iso_date(metadata.published_at);

    let license_url = format!("https://spdx.org/licenses/{}", metadata.license_spdx);

    let author_id = metadata
        .author_orcid
        .clone()
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
fn format_iso_date(unix_secs: i64) -> String {
    // Days since Unix epoch
    let days_since_epoch = unix_secs / 86_400;
    let mut year = 1970u32;
    let mut remaining = days_since_epoch as u32;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }

    let month_days: [u32; 12] = [
        31,
        if is_leap_year(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];

    let mut month = 1u32;
    for &days in &month_days {
        if remaining < days {
            break;
        }
        remaining -= days;
        month += 1;
    }

    let day = remaining + 1; // 1-based day
    format!("{year:04}-{month:02}-{day:02}")
}

fn is_leap_year(year: u32) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
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
        let has_dataset = graph
            .iter()
            .any(|node| node["@type"].as_str() == Some("Dataset"));
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

    #[test]
    fn iso_date_epoch_zero_is_1970_01_01() {
        assert_eq!(format_iso_date(0), "1970-01-01");
    }

    #[test]
    fn iso_date_one_day_is_1970_01_02() {
        assert_eq!(format_iso_date(86_400), "1970-01-02");
    }

    #[test]
    fn iso_date_known_date_2023_11_15() {
        // 1700000000 Unix = 2023-11-14 22:13:20 UTC → date = 2023-11-14
        // 1700006400 Unix = 2023-11-15 00:00:00 UTC → date = 2023-11-15
        assert_eq!(format_iso_date(1_700_006_400), "2023-11-15");
    }
}
