use crate::spa_fallback::is_bot_challenge_page;
use ::html2text::from_read;
use ::scraper::{Html, Selector};
use std::time::Duration;
use tracing::debug;

#[derive(Debug, Clone)]
pub struct ScrapedDocument {
    pub url: String,
    pub title: String,
    pub markdown: String,
    pub text_density: f64,
    pub raw_html: Option<String>,
}

pub fn extract_document_from_html(url: &str, html_content: &str) -> ScrapedDocument {
    let document = Html::parse_document(html_content);

    // 1. Extract Title
    let title_selector = Selector::parse("title").unwrap();
    let title = document
        .select(&title_selector)
        .next()
        .map(|el| el.text().collect::<String>())
        .unwrap_or_else(|| url.to_string());

    // 2. Prune boilerplate (nav, footer, script, style, aside, header)
    // Note: scraper doesn't make it easy to mutate the tree for removal.
    // We'll use a selection-based approach or just rely on the converted text quality.
    // However, html2text often converts everything.

    // A simplified "readability" attempt: target the <main> or <article> if they exist.
    let main_selector =
        Selector::parse("main, article, #content, .content, .post-content").unwrap();
    let target_html = if let Some(main_el) = document.select(&main_selector).next() {
        main_el.html()
    } else {
        // Fallback to body
        let body_selector = Selector::parse("body").unwrap();
        if let Some(body_el) = document.select(&body_selector).next() {
            body_el.html()
        } else {
            html_content.to_string()
        }
    };

    // 3. Convert to Markdown
    let markdown = from_read(target_html.as_bytes(), 80);

    // 4. Text Density Heuristic
    let total_chars = html_content.len() as f64;
    let text_chars = markdown.len() as f64;
    let text_density = if total_chars > 0.0 {
        text_chars / total_chars
    } else {
        0.0
    };

    debug!(
        url = url,
        title = %title,
        density = text_density,
        "Scraped document"
    );

    ScrapedDocument {
        url: url.to_string(),
        title,
        markdown,
        text_density,
        raw_html: Some(html_content.to_string()),
    }
}

pub async fn fetch_and_extract_with_client(
    client: &reqwest::Client,
    url: &str,
) -> anyhow::Result<ScrapedDocument> {
    let resp = client.get(url).send().await?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        if is_bot_challenge_page("", &body) {
            return Err(anyhow::anyhow!(
                "Bot challenge detected on URL {}: HTTP {}",
                url,
                status
            ));
        }
        return Err(anyhow::anyhow!("Failed to fetch URL {}: {}", url, status));
    }

    let html_content = resp.text().await?;
    let doc = extract_document_from_html(url, &html_content);
    if is_bot_challenge_page(&doc.title, &html_content) {
        return Err(anyhow::anyhow!(
            "Bot challenge detected on URL {}: {}",
            url,
            doc.title
        ));
    }
    Ok(doc)
}

pub async fn fetch_and_extract(url: &str, timeout_ms: u64) -> anyhow::Result<ScrapedDocument> {
    let client = vox_http_client::client_builder()
        .timeout(Duration::from_millis(timeout_ms))
        .user_agent("VoxResearchBot/1.0 (+https://vox.dev/research-bot)")
        .build()?;

    fetch_and_extract_with_client(&client, url).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_fetch_and_extract_bot_challenge() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/blocked"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "<html><head><title>Just a moment...</title></head><body>Cloudflare Ray ID: 89f4</body></html>",
            ))
            .mount(&mock_server)
            .await;

        let client = vox_http_client::client_builder().build().unwrap();
        let result =
            fetch_and_extract_with_client(&client, &format!("{}/blocked", mock_server.uri())).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Bot challenge detected on URL"));
        assert!(err_msg.contains("Just a moment..."));
    }

    #[tokio::test]
    async fn test_fetch_and_extract_403_bot_challenge() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/cf-block"))
            .respond_with(
                ResponseTemplate::new(403)
                    .set_body_string("<html><body><p>Cloudflare Ray ID: 89f4abc</p></body></html>"),
            )
            .mount(&mock_server)
            .await;

        let client = vox_http_client::client_builder().build().unwrap();
        let result =
            fetch_and_extract_with_client(&client, &format!("{}/cf-block", mock_server.uri()))
                .await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Bot challenge detected on URL"));
        assert!(err_msg.contains("HTTP 403"));
    }

    #[tokio::test]
    async fn test_fetch_and_extract_404_normal_error() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let client = vox_http_client::client_builder().build().unwrap();
        let result =
            fetch_and_extract_with_client(&client, &format!("{}/missing", mock_server.uri())).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to fetch URL"));
        assert!(err_msg.contains("404"));
        assert!(!err_msg.contains("Bot challenge detected"));
    }

    #[tokio::test]
    async fn test_fetch_and_extract_clean_page() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/ok"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                "<html><head><title>Valid Page</title></head><body><article><p>Clean content</p></article></body></html>",
            ))
            .mount(&mock_server)
            .await;

        let client = vox_http_client::client_builder().build().unwrap();
        let doc = fetch_and_extract_with_client(&client, &format!("{}/ok", mock_server.uri()))
            .await
            .unwrap();

        assert_eq!(doc.title, "Valid Page");
        assert!(doc.markdown.contains("Clean content"));
    }
}
