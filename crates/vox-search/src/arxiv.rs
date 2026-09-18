use std::borrow::Cow;

use quick_xml::Reader;
use quick_xml::events::Event;
use tracing::debug;

use crate::searxng::SearxngResult;

const DEFAULT_ARXIV_BASE_URL: &str = "https://export.arxiv.org";

#[derive(Debug, PartialEq, Eq)]
enum CurrentTag {
    None,
    Title,
    Summary,
    Id,
}

/// Normalizes arXiv identifiers or URLs into canonical `https://arxiv.org/abs/{id}` format,
/// stripping any version suffix like `v1`, `v2`, etc.
pub fn normalize_arxiv_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let id_part = if let Some((_, after)) = trimmed.split_once("/abs/") {
        after
    } else if let Some((_, after)) = trimmed.split_once("/pdf/") {
        after.strip_suffix(".pdf").unwrap_or(after)
    } else {
        trimmed
    };

    let id_part = id_part.split('?').next().unwrap_or(id_part);
    let id_part = id_part.split('#').next().unwrap_or(id_part);
    let id_part = id_part.trim_matches('/');

    let clean_id = if let Some(pos) = id_part.rfind('v') {
        let (base, ver) = id_part.split_at(pos);
        if ver.len() > 1 && ver[1..].chars().all(|c| c.is_ascii_digit()) {
            base
        } else {
            id_part
        }
    } else {
        id_part
    };

    format!("https://arxiv.org/abs/{}", clean_id)
}

fn clean_whitespace(s: &str) -> String {
    let unescaped = quick_xml::escape::unescape(s).unwrap_or(Cow::Borrowed(s));
    unescaped.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub struct ArXivClient;

impl ArXivClient {
    /// Parse Atom XML feed from arXiv, extracting entry nodes into `SearxngResult` up to `limit`.
    pub fn parse_atom_xml(xml: &str, limit: usize) -> anyhow::Result<Vec<SearxngResult>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut reader = Reader::from_str(xml);

        let mut results = Vec::new();
        let mut buf = Vec::new();

        let mut in_entry = false;
        let mut current_tag = CurrentTag::None;

        let mut entry_id = String::new();
        let mut entry_alternate_url: Option<String> = None;
        let mut entry_title = String::new();
        let mut entry_summary = String::new();

        loop {
            let event = match reader.read_event_into(&mut buf) {
                Ok(ev) => ev,
                Err(e) => {
                    return Err(anyhow::anyhow!(
                        "Error parsing arXiv Atom XML at position {}: {:?}",
                        reader.buffer_position(),
                        e
                    ));
                }
            };
            match event {
                Event::Start(ref e) => {
                    let local = e.local_name();
                    match local.as_ref() {
                        b"entry" => {
                            in_entry = true;
                            current_tag = CurrentTag::None;
                            entry_id.clear();
                            entry_alternate_url = None;
                            entry_title.clear();
                            entry_summary.clear();
                        }
                        b"title" if in_entry => current_tag = CurrentTag::Title,
                        b"summary" if in_entry => current_tag = CurrentTag::Summary,
                        b"id" if in_entry => current_tag = CurrentTag::Id,
                        b"link" if in_entry => {
                            Self::extract_link(e, &mut entry_alternate_url);
                        }
                        _ => {
                            if in_entry {
                                current_tag = CurrentTag::None;
                            }
                        }
                    }
                }
                Event::Empty(ref e) => {
                    let local = e.local_name();
                    if in_entry && local.as_ref() == b"link" {
                        Self::extract_link(e, &mut entry_alternate_url);
                    }
                }
                Event::Text(ref e) => {
                    if in_entry {
                        if let Ok(text) = std::str::from_utf8(e.as_ref()) {
                            match current_tag {
                                CurrentTag::Title => entry_title.push_str(text),
                                CurrentTag::Summary => entry_summary.push_str(text),
                                CurrentTag::Id => entry_id.push_str(text),
                                CurrentTag::None => {}
                            }
                        }
                    }
                }
                Event::GeneralRef(ref e) => {
                    if in_entry {
                        let name = std::str::from_utf8(e.as_ref()).unwrap_or("");
                        let entity_str = format!("&{};", name);
                        let decoded = quick_xml::escape::unescape(&entity_str)
                            .unwrap_or(Cow::Borrowed(&entity_str));
                        match current_tag {
                            CurrentTag::Title => entry_title.push_str(&decoded),
                            CurrentTag::Summary => entry_summary.push_str(&decoded),
                            CurrentTag::Id => entry_id.push_str(&decoded),
                            CurrentTag::None => {}
                        }
                    }
                }
                Event::CData(ref e) => {
                    if in_entry {
                        if let Ok(text) = std::str::from_utf8(e.as_ref()) {
                            match current_tag {
                                CurrentTag::Title => entry_title.push_str(text),
                                CurrentTag::Summary => entry_summary.push_str(text),
                                CurrentTag::Id => entry_id.push_str(text),
                                CurrentTag::None => {}
                            }
                        }
                    }
                }
                Event::End(ref e) => {
                    let local = e.local_name();
                    match local.as_ref() {
                        b"entry" => {
                            if in_entry {
                                in_entry = false;
                                current_tag = CurrentTag::None;

                                let raw_url = if let Some(ref alt) = entry_alternate_url {
                                    alt.clone()
                                } else {
                                    entry_id.clone()
                                };

                                let url = normalize_arxiv_url(&raw_url);
                                let title = clean_whitespace(&entry_title);
                                let content = clean_whitespace(&entry_summary);

                                results.push(SearxngResult {
                                    url,
                                    title,
                                    content,
                                    engine: Some("arxiv".to_string()),
                                    score: Some(0.85),
                                });

                                if results.len() >= limit {
                                    break;
                                }
                            }
                        }
                        b"title" if in_entry => current_tag = CurrentTag::None,
                        b"summary" if in_entry => current_tag = CurrentTag::None,
                        b"id" if in_entry => current_tag = CurrentTag::None,
                        _ => {}
                    }
                }
                Event::Eof => break,
                _ => {}
            }
            buf.clear();
        }

        Ok(results)
    }

    fn extract_link(e: &quick_xml::events::BytesStart<'_>, alternate_url: &mut Option<String>) {
        let mut href = None;
        let mut rel = None;

        for attr in e.attributes().flatten() {
            let key = attr.key.local_name();
            match key.as_ref() {
                b"href" => {
                    if let Ok(raw) = std::str::from_utf8(attr.value.as_ref()) {
                        let val = quick_xml::escape::unescape(raw).unwrap_or(Cow::Borrowed(raw));
                        href = Some(val.to_string());
                    }
                }
                b"rel" => {
                    if let Ok(raw) = std::str::from_utf8(attr.value.as_ref()) {
                        let val = quick_xml::escape::unescape(raw).unwrap_or(Cow::Borrowed(raw));
                        rel = Some(val.to_string());
                    }
                }
                _ => {}
            }
        }

        if rel.as_deref() == Some("alternate") || (rel.is_none() && href.is_some()) {
            if let Some(h) = href {
                *alternate_url = Some(h);
            }
        }
    }

    /// Query arXiv API with query string, limit, and optional base URL override.
    pub async fn search(
        query: &str,
        limit: usize,
        base_url: Option<&str>,
    ) -> anyhow::Result<Vec<SearxngResult>> {
        if query.trim().is_empty() || limit == 0 {
            return Ok(Vec::new());
        }

        let base = base_url.unwrap_or(DEFAULT_ARXIV_BASE_URL);
        let base_clean = base.trim_end_matches('/');
        let base_clean = base_clean.strip_suffix("/api/query").unwrap_or(base_clean);
        let url = format!(
            "{base_clean}/api/query?search_query=all:{}&max_results={}",
            urlencoding::encode(query.trim()),
            limit
        );

        debug!(url = %url, query = query, "Firing arXiv search");

        let client = vox_http_client::client();
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "arXiv API returned status {}",
                resp.status()
            ));
        }

        let xml = resp.text().await?;
        Self::parse_atom_xml(&xml, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_arxiv_url_basic() {
        assert_eq!(
            normalize_arxiv_url("http://arxiv.org/abs/2206.05503v1"),
            "https://arxiv.org/abs/2206.05503"
        );
    }

    #[test]
    fn test_parse_atom_xml_basic() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <entry>
    <id>http://arxiv.org/abs/2206.05503v1</id>
    <title>Rust Safety</title>
    <summary>Memory safety study.</summary>
  </entry>
</feed>"#;
        let hits = ArXivClient::parse_atom_xml(xml, 5).expect("parse xml");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].title, "Rust Safety");
    }

    #[tokio::test]
    async fn test_search_short_circuit_empty() {
        let hits = ArXivClient::search("   ", 5, None).await.expect("search");
        assert!(hits.is_empty());
    }
}
