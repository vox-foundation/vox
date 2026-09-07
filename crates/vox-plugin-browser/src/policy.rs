use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserLaunchMode {
    Ephemeral,
    Named,
    Attach,
}

impl Default for BrowserLaunchMode {
    fn default() -> Self {
        Self::Ephemeral
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserLaunchOptions {
    pub url: String,
    #[serde(default = "default_true")]
    pub headless: bool,
    #[serde(default)]
    pub mode: BrowserLaunchMode,
    pub profile_id: Option<String>,
    pub cdp_url: Option<String>,
}

impl Default for BrowserLaunchOptions {
    fn default() -> Self {
        Self {
            url: String::new(),
            headless: true,
            mode: BrowserLaunchMode::Ephemeral,
            profile_id: None,
            cdp_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConsent {
    pub profile_id: String,
    pub save_cookies: bool,
    pub created_unix_s: u64,
}

const CONSENTS_FILE: &str = "consents.json";

fn consents_path(profiles_root: &Path) -> PathBuf {
    profiles_root.join(CONSENTS_FILE)
}

fn load_consents(profiles_root: &Path) -> Vec<ProfileConsent> {
    let Ok(bytes) = std::fs::read(consents_path(profiles_root)) else {
        return Vec::new();
    };
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn save_consents(profiles_root: &Path, consents: &[ProfileConsent]) -> Result<(), String> {
    std::fs::create_dir_all(profiles_root).map_err(|e| e.to_string())?;
    let json = serde_json::to_vec_pretty(consents).map_err(|e| e.to_string())?;
    std::fs::write(consents_path(profiles_root), json).map_err(|e| e.to_string())
}

pub fn has_save_consent(profiles_root: &Path, profile_id: &str) -> bool {
    load_consents(profiles_root)
        .iter()
        .any(|consent| consent.profile_id == profile_id && consent.save_cookies)
}

pub fn record_save_consent(profiles_root: &Path, profile_id: &str) -> Result<(), String> {
    let mut consents = load_consents(profiles_root);
    let created_unix_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Some(existing) = consents
        .iter_mut()
        .find(|consent| consent.profile_id == profile_id)
    {
        existing.save_cookies = true;
    } else {
        consents.push(ProfileConsent {
            profile_id: profile_id.to_string(),
            save_cookies: true,
            created_unix_s,
        });
    }
    save_consents(profiles_root, &consents)
}

pub fn require_named_consent(
    profiles_root: &Path,
    profile_id: &str,
    save_profile: bool,
) -> Result<(), String> {
    if save_profile || has_save_consent(profiles_root, profile_id) {
        Ok(())
    } else {
        Err("consent_required".to_string())
    }
}

pub(crate) fn validate_navigation_url(url: &str) -> Result<(), String> {
    let allow_csv = std::env::var("VOX_BROWSER_ALLOWED_HOSTS").ok();
    if host_allowed(url, allow_csv.as_deref()) {
        Ok(())
    } else {
        Err(format!("host_not_allowed:{}", url_host(url)))
    }
}

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

    #[test]
    fn named_without_consent_errors() {
        let root = tempfile_or_std_temp("vox-consent");
        let id = parse_profile_id("staging-1").unwrap();
        assert!(!has_save_consent(&root, &id));
        let err = require_named_consent(&root, &id, false).unwrap_err();
        assert_eq!(err, "consent_required");
        record_save_consent(&root, &id).unwrap();
        assert!(require_named_consent(&root, &id, false).is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    fn tempfile_or_std_temp(prefix: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let root = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&root).expect("temp consent dir");
        root
    }
}
