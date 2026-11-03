//! Stable `diagram/*` diagnostic codes. Codes are flat, never renamed, only added.

/// A token sequence the grammar does not allow ("expected X, found Y").
pub const UNEXPECTED_TOKEN: &str = "diagram/unexpected-token";
/// An edge endpoint names a node that is not declared.
pub const UNDECLARED_NODE: &str = "diagram/undeclared-node";
/// A message or note names an actor that is not declared.
pub const UNDECLARED_ACTOR: &str = "diagram/undeclared-actor";
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

/// A label that is unquoted or single-quoted; labels are double-quoted.
pub const EXPECTED_QUOTED_LABEL: &str = "diagram/expected-quoted-label";
/// A `{` that is never closed before the end of the input.
pub const UNCLOSED_BRACE: &str = "diagram/unclosed-brace";
/// Anything after the closing `}` of the graph block.
pub const TRAILING_CONTENT: &str = "diagram/trailing-content";
/// The header word is not a supported diagram kind.
pub const UNKNOWN_DIAGRAM_KIND: &str = "diagram/unknown-diagram-kind";
/// A declared id that equals a keyword.
pub const RESERVED_WORD: &str = "diagram/reserved-word";
/// A direction word other than down, right, up or left.
pub const INVALID_DIRECTION: &str = "diagram/invalid-direction";
/// A second `direction` statement in one graph.
pub const DUPLICATE_DIRECTION: &str = "diagram/duplicate-direction";
/// A `direction` statement inside a container.
pub const DIRECTION_IN_CONTAINER: &str = "diagram/direction-in-container";
/// A shape word that is not one of the six shapes.
pub const INVALID_SHAPE: &str = "diagram/invalid-shape";
/// An attribute key other than `shape`.
pub const UNKNOWN_ATTRIBUTE: &str = "diagram/unknown-attribute";
/// The same id declared twice in one scope.
pub const DUPLICATE_ID: &str = "diagram/duplicate-id";
/// An inner id hides an outer id (warning).
pub const SHADOWED_ID: &str = "diagram/shadowed-id";
/// A quoted label with no text.
pub const EMPTY_LABEL: &str = "diagram/empty-label";

/// More than `limits::MAX_NODES` nodes.
pub const LIMIT_NODES: &str = "diagram/limit-nodes";
/// More than `limits::MAX_EDGES` edges.
pub const LIMIT_EDGES: &str = "diagram/limit-edges";
/// Containers nested deeper than `limits::MAX_DEPTH`.
pub const LIMIT_DEPTH: &str = "diagram/limit-depth";
/// A label longer than `limits::MAX_LABEL_CHARS` characters.
pub const LIMIT_LABEL: &str = "diagram/limit-label";

/// Mermaid syntax used where VoxDiagram has a different canonical form.
pub const MERMAID_SYNTAX: &str = "diagram/mermaid-syntax";

/// `a -> b -> c`: several edges chained on one line.
pub const CHAINED_EDGE: &str = "diagram/chained-edge";

/// More diagnostics were found than one report can hold (sentinel, always a warning).
pub const LIMIT_DIAGNOSTICS: &str = "diagram/limit-diagnostics";

/// Every code the crate can emit.
pub const ALL_CODES: &[&str] = &[
    UNEXPECTED_TOKEN,
    UNDECLARED_NODE,
    UNDECLARED_ACTOR,
    LIMIT_SOURCE,
    NOT_A_CONTAINER,
    CONTAINER_ENDPOINT,
    UNTERMINATED_STRING,
    INVALID_ESCAPE,
    FORBIDDEN_CHAR,
    INVALID_IDENTIFIER,
    LIMIT_DIAGNOSTICS,
    EXPECTED_QUOTED_LABEL,
    UNCLOSED_BRACE,
    TRAILING_CONTENT,
    UNKNOWN_DIAGRAM_KIND,
    RESERVED_WORD,
    INVALID_DIRECTION,
    DUPLICATE_DIRECTION,
    DIRECTION_IN_CONTAINER,
    INVALID_SHAPE,
    UNKNOWN_ATTRIBUTE,
    DUPLICATE_ID,
    SHADOWED_ID,
    EMPTY_LABEL,
    LIMIT_NODES,
    LIMIT_EDGES,
    LIMIT_DEPTH,
    LIMIT_LABEL,
    MERMAID_SYNTAX,
    CHAINED_EDGE,
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
        assert_eq!(ALL_CODES.len(), 30);
    }
}
