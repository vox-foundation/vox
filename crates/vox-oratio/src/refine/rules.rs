//! Deterministic refinement rules (no ML).

/// Collapse outer whitespace and trim ends — safe default before richer ITN ships.
#[must_use]
pub fn light_trim(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_trim_collapse() {
        assert_eq!(light_trim("  a   b  "), "a b");
    }
}
