use regex::Regex;
use std::sync::OnceLock;

/// Simple PII filter that redacts common sensitive patterns.
pub struct PiiFilter;

impl PiiFilter {
    /// Redact potential PII (emails, IP addresses, typical tokens) from a string.
    pub fn redact(text: &str) -> String {
        let mut result = text.to_string();
        
        // Email pattern
        static EMAIL_RE: OnceLock<Regex> = OnceLock::new();
        let email_re = EMAIL_RE.get_or_init(|| Regex::new(r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}").unwrap());
        result = email_re.replace_all(&result, "[REDACTED_EMAIL]").to_string();

        // IPv4 pattern
        static IP_RE: OnceLock<Regex> = OnceLock::new();
        let ip_re = IP_RE.get_or_init(|| Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap());
        result = ip_re.replace_all(&result, "[REDACTED_IP]").to_string();

        // Potential secret/token pattern (e.g. sk-..., gsk-...)
        static TOKEN_RE: OnceLock<Regex> = OnceLock::new();
        let token_re = TOKEN_RE.get_or_init(|| Regex::new(r"\b(?:sk|gsk|pk|ak)-[a-zA-Z0-9]{20,}\b").unwrap());
        result = token_re.replace_all(&result, "[REDACTED_TOKEN]").to_string();

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_email() {
        let input = "Contact me at alice@example.com for info.";
        let output = PiiFilter::redact(input);
        assert_eq!(output, "Contact me at [REDACTED_EMAIL] for info.");
    }

    #[test]
    fn redacts_ip() {
        let input = "Connecting to 192.168.1.42 now.";
        let output = PiiFilter::redact(input);
        assert_eq!(output, "Connecting to [REDACTED_IP] now.");
    }
}
