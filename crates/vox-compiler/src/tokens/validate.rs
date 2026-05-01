//! Registry-level consistency checks for the design-token system.
//!
//! These checks run against a [`TokenRegistry`] independently of any WebIR
//! module; per-node token checks live in `web_ir::validate`.

use crate::tokens::TokenRegistry;

// ---------------------------------------------------------------------------
// Diagnostic type
// ---------------------------------------------------------------------------

/// A diagnostic emitted by [`validate_token_registry`].
#[derive(Debug, Clone)]
pub struct TokenValidationDiagnostic {
    pub code: String,
    pub message: String,
}

impl std::fmt::Display for TokenValidationDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

// ---------------------------------------------------------------------------
// Validator
// ---------------------------------------------------------------------------

/// Run consistency checks on a [`TokenRegistry`].
///
/// # Checks
///
/// - `token.registry.empty` — the registry contains no tokens at all.
/// - `token.registry.invalid_key` — a token key contains whitespace characters.
///
/// # Future work (TODO)
///
/// - Contrast ratio checks between foreground/background color pairs require
///   a full CSS color parser; defer to a dedicated audit pass.
pub fn validate_token_registry(registry: &TokenRegistry) -> Vec<TokenValidationDiagnostic> {
    let mut out = Vec::new();

    if registry.is_empty() {
        out.push(TokenValidationDiagnostic {
            code: "token.registry.empty".to_string(),
            message: "token registry is empty — no design tokens were loaded".to_string(),
        });
        // No point checking keys if there are none.
        return out;
    }

    for key in registry.all_keys() {
        if key.chars().any(|c| c.is_whitespace()) {
            out.push(TokenValidationDiagnostic {
                code: "token.registry.invalid_key".to_string(),
                message: format!("token key {key:?} contains whitespace"),
            });
        }
    }

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::TokenRegistry;

    #[test]
    fn empty_registry_warns() {
        let reg = TokenRegistry::load_from_str("{}").unwrap();
        let diags = validate_token_registry(&reg);
        assert!(diags.iter().any(|d| d.code == "token.registry.empty"));
    }

    #[test]
    fn valid_registry_no_warnings() {
        let reg =
            TokenRegistry::load_from_str(r##"{"color":{"primary":"#fff"}}"##).unwrap();
        let diags = validate_token_registry(&reg);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
    }

    #[test]
    fn whitespace_key_emits_invalid_key() {
        let reg =
            TokenRegistry::load_from_str(r##"{"color":{"bad key":"#fff"}}"##).unwrap();
        let diags = validate_token_registry(&reg);
        assert!(
            diags.iter().any(|d| d.code == "token.registry.invalid_key"),
            "expected invalid_key diagnostic, got: {diags:?}"
        );
        assert!(
            diags.iter().any(|d| d.message.contains("bad key")),
            "diagnostic should name the offending key"
        );
    }
}
