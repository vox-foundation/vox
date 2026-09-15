use vox_search::spa_fallback::{
    is_bot_challenge_page, sanitize_extracted_accessibility_text, should_fallback_to_headless,
};

#[test]
fn test_is_bot_challenge_page_detection() {
    assert!(is_bot_challenge_page(
        "Just a moment...",
        "<html><head><title>Just a moment...</title></head><body>Cloudflare Ray ID: 89f4</body></html>"
    ));
    assert!(is_bot_challenge_page(
        "Security Check",
        "<html><body>Please enable JavaScript and cookies to continue.</body></html>"
    ));
    // Check detection via HTML body with empty/generic title (e.g. HTTP 403 response)
    assert!(is_bot_challenge_page(
        "",
        "<html><head><title>403 Forbidden</title></head><body>Cloudflare Ray ID: 89f4</body></html>"
    ));
    assert!(is_bot_challenge_page(
        "Access Issue",
        "<html><body><noscript>Please enable JavaScript and cookies to proceed.</noscript></body></html>"
    ));
    assert!(!is_bot_challenge_page(
        "Tokio Documentation",
        "<html><article>Async runtime for Rust</article></html>"
    ));
}

#[test]
fn test_should_fallback_to_headless_on_spa_shells() {
    let empty_root =
        "<html><body><div id=\"root\"></div><script src=\"bundle.js\"></script></body></html>";
    assert!(should_fallback_to_headless(empty_root, 15, 0.02));

    let next_app =
        "<html><body><div id=\"__next\"></div><script>var __NEXT_DATA__={};</script></body></html>";
    assert!(should_fallback_to_headless(next_app, 30, 0.03));

    // Meaningful branch test: extracted_chars >= 120 (e.g. 250) and text_density >= 0.06 (e.g. 0.15)
    // When SPA pattern is present, fallback triggers if extracted_chars < 400
    let spa_with_some_text = "<html><body><div id=\"root\">Loading app...</div><script>window.INIT=1;</script></body></html>";
    assert!(should_fallback_to_headless(spa_with_some_text, 250, 0.15));

    // Non-SPA with same 250 chars and 0.15 text density should NOT trigger fallback
    let non_spa_with_some_text = "<html><body><article><p>Some intermediate documentation snippet without SPA root.</p></article></body></html>";
    assert!(!should_fallback_to_headless(
        non_spa_with_some_text,
        250,
        0.15
    ));

    let dense_content = "<html><body><article><p>Comprehensive technical documentation about Rust type systems.</p></article></body></html>";
    assert!(!should_fallback_to_headless(dense_content, 500, 0.35));
}

#[test]
fn test_sanitize_extracted_accessibility_text() {
    let raw_ax = r#"RootWebArea [url="https://vox.dev"]
banner
  heading "Vox Documentation" [level=1] description="Vox Homepage"
main
  paragraph
  heading "Overview" [level=2]
  list
    listItem
      text "Vox is an agent-first programming language."
  contentinfo
  none"#;
    let cleaned = sanitize_extracted_accessibility_text(raw_ax);
    assert!(cleaned.contains("# Vox Documentation"));
    assert!(!cleaned.contains("Vox Homepage")); // Verifies forward quote parsing didn't swallow description
    assert!(cleaned.contains("## Overview"));
    assert!(cleaned.contains("Vox is an agent-first programming language."));
    // Verifies structural noise lines were filtered out
    assert!(!cleaned.contains("RootWebArea"));
    assert!(!cleaned.contains("paragraph"));
    assert!(!cleaned.contains("listItem"));
    assert!(!cleaned.contains("contentinfo"));
}
