//! Free AI client with multi-provider fallback.
//!
//! Supports a cascade of providers so Vox is fully redistributable:
//! 1. **Ollama** (local) — zero auth, best quality, no network
//! 2. **Pollinations.ai** — zero API key, zero signup, HTTP GET
//! 3. **Gemini Flash** — free tier, requires env var `GEMINI_API_KEY`
//! 4. **Deterministic** — always works, no AI, pattern-based responses

mod client;
mod constants;
mod error;
mod fallback;
mod keys;
mod provider;
mod validate;

pub use client::{AiReportFn, FreeAiClient, LudusStreamBackend, StreamRoute};
pub use error::AiError;
pub use fallback::deterministic_response;
pub use provider::FreeAiProvider;
pub use validate::{validate_hint, validate_svg};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_sprite_response() {
        let resp = deterministic_response("Generate an ASCII sprite for a happy robot");
        assert!(resp.contains("/\\_/\\"));
    }

    #[test]
    fn deterministic_name_response() {
        let resp = deterministic_response("Generate a creative name");
        assert_eq!(resp, "Code Companion");
    }

    #[test]
    fn deterministic_clean_response() {
        let resp = deterministic_response("Analyze code quality");
        assert_eq!(resp, "CLEAN");
    }

    #[test]
    fn deterministic_generic_response() {
        let resp = deterministic_response("Hello world");
        assert!(resp.contains("offline mode"));
    }

    #[test]
    fn provider_names() {
        assert_eq!(
            FreeAiProvider::Pollinations.name(),
            "Pollinations.ai (free)"
        );
    }

    #[test]
    fn url_encoding_basics() {
        let enc = crate::ai::validate::urlencode("a b");
        assert!(enc.contains("%20") || enc.contains('+'));
    }

    #[tokio::test]
    async fn client_has_deterministic_last() {
        let c = FreeAiClient::new(vec![FreeAiProvider::Deterministic]);
        let out = c.generate("hello").await.unwrap();
        assert!(!out.is_empty());
    }
}
