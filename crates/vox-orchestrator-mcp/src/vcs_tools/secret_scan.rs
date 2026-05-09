//! Secret-pattern scanner for staged VCS content.
//!
//! Detects common secret patterns (API keys, tokens, private keys) in text
//! content before it is committed to version control.

use std::sync::OnceLock;

use regex::Regex;

/// The kind of secret that was detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretKind {
    AwsAccessKey,
    AwsSecretKey,
    GitHubToken,
    OpenAiKey,
    AnthropicKey,
    SlackToken,
    GoogleApiKey,
    PemPrivateKey,
}

/// A single secret match found in scanned content.
#[derive(Debug)]
pub struct SecretMatch {
    /// The kind of secret detected.
    pub kind: SecretKind,
    /// 1-based line number where the match was found.
    pub line: usize,
    /// First 6 chars of the matched string + "..." to avoid storing the actual secret.
    pub redacted: String,
}

struct Pattern {
    kind: SecretKind,
    re: Regex,
}

fn patterns() -> &'static [Pattern] {
    static PATTERNS: OnceLock<Vec<Pattern>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        vec![
            Pattern {
                kind: SecretKind::AwsAccessKey,
                re: Regex::new(r"AKIA[0-9A-Z]{16}").unwrap(),
            },
            Pattern {
                kind: SecretKind::AwsSecretKey,
                re: Regex::new(
                    "(?i)aws.{0,20}secret.{0,20}[=:]\\s*[\"']?([A-Za-z0-9/+=]{40})",
                )
                .unwrap(),
            },
            Pattern {
                kind: SecretKind::GitHubToken,
                re: Regex::new(r"gh[pousr]_[A-Za-z0-9_]{36,}").unwrap(),
            },
            Pattern {
                kind: SecretKind::OpenAiKey,
                re: Regex::new(r"sk-[A-Za-z0-9]{20,}T3BlbkFJ[A-Za-z0-9]{20,}").unwrap(),
            },
            Pattern {
                kind: SecretKind::AnthropicKey,
                re: Regex::new(r"sk-ant-[A-Za-z0-9\-_]{90,}").unwrap(),
            },
            Pattern {
                kind: SecretKind::SlackToken,
                re: Regex::new(r"xox[baprs]-[A-Za-z0-9\-]{10,}").unwrap(),
            },
            Pattern {
                kind: SecretKind::GoogleApiKey,
                re: Regex::new(r"AIza[0-9A-Za-z\-_]{35}").unwrap(),
            },
            Pattern {
                kind: SecretKind::PemPrivateKey,
                re: Regex::new(r"-----BEGIN (RSA |EC |OPENSSH )?PRIVATE KEY-----").unwrap(),
            },
        ]
    })
}

/// Scan `content` for common secret patterns.
///
/// Returns a list of [`SecretMatch`] values, one per pattern match found.
/// Line numbers are 1-based. A single line may produce multiple matches.
/// The `redacted` field contains only the first 6 characters of the matched
/// text followed by `"..."` so the actual secret is never stored.
pub fn scan_for_secrets(content: &str) -> Vec<SecretMatch> {
    let patterns = patterns();
    let mut matches = Vec::new();

    for (line_idx, line) in content.lines().enumerate() {
        let line_number = line_idx + 1;
        for pattern in patterns {
            for mat in pattern.re.find_iter(line) {
                let matched = mat.as_str();
                let prefix: String = matched.chars().take(6).collect();
                let redacted = format!("{}...", prefix);
                matches.push(SecretMatch {
                    kind: pattern.kind.clone(),
                    line: line_number,
                    redacted,
                });
            }
        }
    }

    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_aws_access_key() {
        let content = "AKIAIOSFODNN7EXAMPLE";
        let matches = scan_for_secrets(content);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, SecretKind::AwsAccessKey);
        assert_eq!(matches[0].line, 1);
    }

    #[test]
    fn detects_github_token() {
        let content = "ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        let matches = scan_for_secrets(content);
        assert!(
            matches.iter().any(|m| m.kind == SecretKind::GitHubToken),
            "expected GitHubToken match, got: {:?}",
            matches.iter().map(|m| &m.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn detects_pem_header() {
        let content = "-----BEGIN RSA PRIVATE KEY-----";
        let matches = scan_for_secrets(content);
        assert!(
            matches.iter().any(|m| m.kind == SecretKind::PemPrivateKey),
            "expected PemPrivateKey match"
        );
    }

    #[test]
    fn detects_anthropic_key() {
        // "sk-ant-" + 90 'A' chars = 97 chars total, satisfies {90,}
        let key = format!("sk-ant-{}", "A".repeat(90));
        let matches = scan_for_secrets(&key);
        assert!(
            matches.iter().any(|m| m.kind == SecretKind::AnthropicKey),
            "expected AnthropicKey match, got: {:?}",
            matches.iter().map(|m| &m.kind).collect::<Vec<_>>()
        );
    }

    #[test]
    fn clean_content_returns_empty() {
        let content = "This file has no secrets, just normal text.\nAnother line here.";
        let matches = scan_for_secrets(content);
        assert!(matches.is_empty(), "expected no matches, got: {:?}", matches.len());
    }

    #[test]
    fn line_numbers_are_correct() {
        // Secret is on line 3 (two blank lines, then the key)
        let content = "\n\nAKIAIOSFODNN7EXAMPLE";
        let matches = scan_for_secrets(content);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].kind, SecretKind::AwsAccessKey);
        assert_eq!(matches[0].line, 3);
    }

    #[test]
    fn redacted_shows_prefix_only() {
        let content = "AKIAIOSFODNN7EXAMPLE";
        let matches = scan_for_secrets(content);
        assert_eq!(matches.len(), 1);
        // 6 chars + "..." = 9 chars total
        assert_eq!(matches[0].redacted.len(), 9, "redacted: {:?}", matches[0].redacted);
        assert!(matches[0].redacted.ends_with("..."));
        assert_eq!(&matches[0].redacted[..6], "AKIAIO");
    }
}
