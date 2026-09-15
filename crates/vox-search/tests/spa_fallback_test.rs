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

    let dense_content = "<html><body><article><p>Comprehensive technical documentation about Rust type systems.</p></article></body></html>";
    assert!(!should_fallback_to_headless(dense_content, 500, 0.35));
}

#[test]
fn test_sanitize_extracted_accessibility_text() {
    let raw_ax = "banner\n  heading \"Vox Documentation\" [level=1]\nmain\n  heading \"Overview\" [level=2]\n  text \"Vox is an agent-first programming language.\"";
    let cleaned = sanitize_extracted_accessibility_text(raw_ax);
    assert!(cleaned.contains("# Vox Documentation"));
    assert!(cleaned.contains("## Overview"));
    assert!(cleaned.contains("Vox is an agent-first programming language."));
}
