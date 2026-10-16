//! Stable `diagram/*` diagnostic codes. Codes are flat, never renamed, only added.

/// A token sequence the grammar does not allow ("expected X, found Y").
pub const UNEXPECTED_TOKEN: &str = "diagram/unexpected-token";
/// An edge endpoint names a node that is not declared.
pub const UNDECLARED_NODE: &str = "diagram/undeclared-node";
/// The source exceeds `limits::MAX_SOURCE_BYTES`.
pub const LIMIT_SOURCE: &str = "diagram/limit-source";

/// Every code the crate can emit.
pub const ALL_CODES: &[&str] = &[UNEXPECTED_TOKEN, UNDECLARED_NODE, LIMIT_SOURCE];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_prefixed_kebab_and_unique() {
        for (i, code) in ALL_CODES.iter().enumerate() {
            let name = code.strip_prefix("diagram/");
            assert!(name.is_some(), "{code} lacks the diagram/ prefix");
            let name = name.unwrap_or_default();
            assert!(!name.is_empty());
            assert!(
                name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
                "{code} is not lowercase kebab"
            );
            assert!(!ALL_CODES[..i].contains(code), "{code} is duplicated");
        }
        assert_eq!(ALL_CODES.len(), 3);
    }
}
