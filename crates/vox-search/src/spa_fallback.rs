const BOT_CHALLENGE_TITLE_SUBSTRINGS: &[&str] = &[
    "just a moment...",
    "attention required!",
    "security check",
    "access denied",
    "ddos-guard",
    "cloudflare",
    "please enable javascript and cookies",
];

const BOT_CHALLENGE_BODY_SUBSTRINGS: &[&str] = &[
    "just a moment...",
    "attention required!",
    "security check",
    "please enable javascript and cookies",
    "cloudflare ray id",
    "ray id:",
    "ddos-guard",
    "cf-browser-verification",
    "cf_chl_opt",
];

const SPA_SHELL_PATTERNS: &[&str] = &[
    "id=\"root\"",
    "id=\"__next\"",
    "id=\"app\"",
    "id=\"__nuxt\"",
    "id=\"docusaurus\"",
    "<noscript>you need to enable javascript",
    "<noscript>please enable javascript",
];

const STRUCTURAL_CONTAINERS: &[&str] = &[
    "banner",
    "main",
    "navigation",
    "paragraph",
    "RootWebArea",
    "list",
    "listItem",
    "none",
    "contentinfo",
];

pub fn is_bot_challenge_page(title: &str, raw_html: &str) -> bool {
    let lower_title = title.to_lowercase();
    let lower_html = raw_html.to_lowercase();

    for sub in BOT_CHALLENGE_TITLE_SUBSTRINGS {
        if lower_title.contains(sub) {
            return true;
        }
    }
    for sub in BOT_CHALLENGE_BODY_SUBSTRINGS {
        if lower_html.contains(sub) {
            return true;
        }
    }
    false
}

pub fn should_fallback_to_headless(
    raw_html: &str,
    extracted_chars: usize,
    text_density: f64,
) -> bool {
    if extracted_chars < 120 || text_density < 0.06 {
        return true;
    }
    let lower = raw_html.to_lowercase();
    for pattern in SPA_SHELL_PATTERNS {
        if lower.contains(pattern) && extracted_chars < 400 {
            return true;
        }
    }
    false
}

pub fn sanitize_extracted_accessibility_text(ax_text: &str) -> String {
    let mut out = Vec::new();
    for line in ax_text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let is_structural = STRUCTURAL_CONTAINERS.iter().any(|&c| {
            if trimmed == c {
                return true;
            }
            if let Some(rest) = trimmed.strip_prefix(c) {
                if rest.starts_with(' ') || rest.starts_with('\t') || rest.starts_with('[') {
                    return true;
                }
            }
            false
        });
        if is_structural {
            continue;
        }

        if let Some(pos) = trimmed.find("heading \"") {
            let start = pos + 9;
            if let Some(end) = trimmed[start..].find('"') {
                let heading_text = &trimmed[start..start + end];
                let level = if trimmed.contains("level=1") {
                    "# "
                } else if trimmed.contains("level=2") {
                    "## "
                } else if trimmed.contains("level=3") {
                    "### "
                } else {
                    "#### "
                };
                out.push(format!("{level}{heading_text}"));
                continue;
            }
        }
        if let Some(pos) = trimmed.find("text \"") {
            let start = pos + 6;
            if let Some(end) = trimmed[start..].find('"') {
                let text = &trimmed[start..start + end];
                out.push(text.to_string());
                continue;
            }
        }
        if !trimmed.starts_with("generic") && !trimmed.starts_with("group") {
            out.push(trimmed.to_string());
        }
    }
    out.join("\n\n")
}
