//! Stable `diagram/*` diagnostic codes. Codes are flat, never renamed, only added.

/// A token sequence the grammar does not allow ("expected X, found Y").
pub const UNEXPECTED_TOKEN: &str = "diagram/unexpected-token";
/// An edge endpoint names a node that is not declared.
pub const UNDECLARED_NODE: &str = "diagram/undeclared-node";
/// The source exceeds `limits::MAX_SOURCE_BYTES`.
pub const LIMIT_SOURCE: &str = "diagram/limit-source";

/// A dotted reference passes through a node (only containers have children).
pub const NOT_A_CONTAINER: &str = "diagram/not-a-container";
/// An edge endpoint resolves to a container; edges connect nodes.
pub const CONTAINER_ENDPOINT: &str = "diagram/container-endpoint";

/// A string reached the end of its line without a closing quote.
pub const UNTERMINATED_STRING: &str = "diagram/unterminated-string";
/// A backslash escape other than `\"`, `\\` or `\n`.
pub const INVALID_ESCAPE: &str = "diagram/invalid-escape";
/// A control or bidirectional-override character inside a label.
pub const FORBIDDEN_CHAR: &str = "diagram/forbidden-char";
/// A word that is not a valid identifier.
pub const INVALID_IDENTIFIER: &str = "diagram/invalid-identifier";

/// Every code the crate can emit.
pub const ALL_CODES: &[&str] = &[
    UNEXPECTED_TOKEN,
    UNDECLARED_NODE,
    LIMIT_SOURCE,
    NOT_A_CONTAINER,
    CONTAINER_ENDPOINT,
    UNTERMINATED_STRING,
    INVALID_ESCAPE,
    FORBIDDEN_CHAR,
    INVALID_IDENTIFIER,
];

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
        assert_eq!(ALL_CODES.len(), 9);
    }
}
