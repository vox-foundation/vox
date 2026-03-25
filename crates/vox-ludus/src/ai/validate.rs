//! Output validation helpers.

/// Validate AI-generated SVG output.
///
/// Returns `Ok(svg)` if the SVG is plausibly valid, or `Err` with a reason.
/// Used to gate AI-generated sprites before persisting them.
pub fn validate_svg(svg: &str) -> Result<&str, &'static str> {
    let trimmed = svg.trim();
    if trimmed.is_empty() {
        return Err("SVG is empty");
    }
    if !trimmed.contains("<svg") {
        return Err("SVG missing <svg> element");
    }
    if !trimmed.contains("</svg>") {
        return Err("SVG missing closing </svg>");
    }
    // Basic safety: no script tags
    if trimmed.to_ascii_lowercase().contains("<script") {
        return Err("SVG contains <script> — rejected for safety");
    }
    Ok(trimmed)
}

/// Validate AI-generated hint text.
///
/// Returns `Ok(hint)` if the hint is plausibly useful, or `Err` with a reason.
pub fn validate_hint(hint: &str) -> Result<String, &'static str> {
    let trimmed = hint.trim().to_string();
    if trimmed.is_empty() {
        return Err("hint is empty");
    }
    if trimmed.len() < 10 {
        return Err("hint is too short");
    }
    if trimmed.len() > 2000 {
        return Err("hint is too long");
    }
    // Reject obvious garbage
    if trimmed.chars().all(|c| !c.is_alphabetic()) {
        return Err("hint contains no alphabetic content");
    }
    Ok(trimmed)
}

/// Minimal URL encoding for the Pollinations GET endpoint.
pub(crate) fn urlencode(s: &str) -> String {
    s.chars().map(urlencode_char).collect()
}

fn urlencode_char(c: char) -> String {
    match c {
        ' ' => "%20".to_string(),
        '\n' => "%0A".to_string(),
        '\r' => String::new(),
        '"' => "%22".to_string(),
        '#' => "%23".to_string(),
        '%' => "%25".to_string(),
        '&' => "%26".to_string(),
        '+' => "%2B".to_string(),
        '?' => "%3F".to_string(),
        _ if c.is_ascii_alphanumeric() || "-._~:/!$'()*,;=@".contains(c) => c.to_string(),
        _ => format!("%{:02X}", c as u32),
    }
}
