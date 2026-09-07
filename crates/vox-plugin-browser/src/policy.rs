pub fn parse_profile_id(raw: &str) -> Result<String, String> {
    let valid = !raw.is_empty()
        && raw.len() <= 63
        && raw
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && raw
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if !valid {
        return Err("profile_id must be 1-63 lowercase kebab-case characters".to_string());
    }

    let lower = raw.to_ascii_lowercase();
    let device_name = matches!(lower.as_str(), "con" | "prn" | "aux" | "nul")
        || lower
            .strip_prefix("com")
            .or_else(|| lower.strip_prefix("lpt"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'));
    if device_name {
        return Err("profile_id is a reserved Windows device name".to_string());
    }

    Ok(raw.to_string())
}

pub fn host_allowed(url: &str, allow_csv: Option<&str>) -> bool {
    if url
        .get(..5)
        .is_some_and(|scheme| scheme.eq_ignore_ascii_case("data:"))
        || url.eq_ignore_ascii_case("about:blank")
    {
        return true;
    }

    let host = url_host(url);
    if matches!(host.as_str(), "localhost" | "127.0.0.1" | "[::1]") {
        return true;
    }
    if host.is_empty() {
        return false;
    }

    let Some(allow_csv) = allow_csv.filter(|csv| !csv.trim().is_empty()) else {
        return true;
    };
    allow_csv
        .split(',')
        .map(str::trim)
        .filter(|allowed| !allowed.is_empty())
        .any(|allowed| {
            let allowed = allowed.to_ascii_lowercase();
            if let Some(suffix) = allowed.strip_prefix("*.") {
                host.len() > suffix.len()
                    && host.ends_with(suffix)
                    && host.as_bytes()[host.len() - suffix.len() - 1] == b'.'
            } else {
                host == allowed
            }
        })
}

pub(crate) fn url_host(url: &str) -> String {
    let authority = url
        .split_once("://")
        .map_or(url, |(_, remainder)| remainder)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if authority.contains('\\') {
        return String::new();
    }
    let authority = authority.rsplit('@').next().unwrap_or_default();

    let host = if authority.starts_with('[') {
        authority
            .find(']')
            .map_or(authority, |end| &authority[..=end])
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    host.trim_end_matches('.').to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_id_rejects_paths_and_device_names() {
        assert!(parse_profile_id("..").is_err());
        assert!(parse_profile_id("a/b").is_err());
        assert!(parse_profile_id("Work").is_err());
        assert!(parse_profile_id("con").is_err());
        assert!(parse_profile_id("COM1").is_err());
        assert_eq!(parse_profile_id("staging-1").unwrap(), "staging-1");
    }

    #[test]
    fn allowlist_empty_allows_all() {
        assert!(host_allowed("https://evil.test/x", None));
        assert!(host_allowed("https://evil.test/x", Some("")));
    }

    #[test]
    fn allowlist_suffix_localhost_and_data() {
        assert!(host_allowed(
            "https://app.example.com/",
            Some("*.example.com")
        ));
        assert!(!host_allowed("https://example.com/", Some("*.example.com")));
        assert!(host_allowed("http://localhost:5173/", Some("example.com")));
        assert!(host_allowed(
            "data:text/html,<h1>x</h1>",
            Some("example.com")
        ));
        assert!(host_allowed("about:blank", Some("example.com")));
        assert!(!host_allowed("https://evil.test/", Some("example.com")));
    }

    #[test]
    fn allowlist_ignores_empty_csv_entries() {
        assert!(!host_allowed("file:///tmp/page.html", Some("example.com,")));
    }

    #[test]
    fn allowlist_rejects_backslash_authority() {
        assert!(!host_allowed(
            r"https://evil.test\@example.com/",
            Some("example.com")
        ));
    }

    #[test]
    fn data_scheme_is_case_insensitive() {
        assert!(host_allowed(
            "DATA:text/html,<h1>x</h1>",
            Some("example.com")
        ));
    }
}
