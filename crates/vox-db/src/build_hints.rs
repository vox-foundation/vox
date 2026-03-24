//! Static hint table for build observability — maps crate-name prefixes and
//! warning codes to actionable developer suggestions.
//!
//! Zero DB overhead: all hints live here as `&'static str` slices. The lookup
//! is O(n) over a small table; fast enough for per-run reporting.

/// One entry in the static hint table.
pub struct BuildHint {
    /// Substring matched against the crate name (case-insensitive prefix check).
    pub crate_pattern: &'static str,
    /// Optional `code` field from `compiler-message` (e.g. `"dead_code"`, `"unused_imports"`).
    pub warning_code: Option<&'static str>,
    /// Short actionable suggestion surfaced to the developer.
    pub suggestion: &'static str,
}

/// Static hint table. Entries are matched top-to-bottom; the first match wins.
pub static BUILD_HINTS: &[BuildHint] = &[
    // ── Crate-level latency hints ─────────────────────────────────────────
    BuildHint {
        crate_pattern: "tokio",
        warning_code: None,
        suggestion: "Slim tokio features: replace `features=[\"full\"]` with `[\"rt\",\"macros\",\"sync\",\"time\"]` in this crate's Cargo.toml",
    },
    BuildHint {
        crate_pattern: "schemars",
        warning_code: None,
        suggestion: "Make schemars optional: add `#[cfg_attr(feature=\"schemars\", derive(schemars::JsonSchema))]` and gate with `optional = true`",
    },
    BuildHint {
        crate_pattern: "jsonschema",
        warning_code: None,
        suggestion: "Consider replacing jsonschema with schemars validation; jsonschema pulls ahash/fancy-regex adding ~4s to clean builds",
    },
    BuildHint {
        crate_pattern: "syn",
        warning_code: None,
        suggestion: "syn appears in your critical path — check for proc-macro crates that can be replaced with simpler alternatives",
    },
    BuildHint {
        crate_pattern: "turso_core",
        warning_code: None,
        suggestion: "turso_core is always slow (large C++ backend). Ensure crates that don't need DB access do not transitively pull vox-db",
    },
    BuildHint {
        crate_pattern: "ring",
        warning_code: None,
        suggestion: "ring compiles C code; consider rustls with aws-lc-rs or native-tls for faster non-FIPS builds",
    },
    BuildHint {
        crate_pattern: "reqwest",
        warning_code: None,
        suggestion: "reqwest re-links TLS on every crate that depends on it. Use a single shared HTTP client in vox-runtime rather than multiple reqwest instances",
    },
    // ── Warning-code hints ────────────────────────────────────────────────
    BuildHint {
        crate_pattern: "",
        warning_code: Some("dead_code"),
        suggestion: "Mark item `#[allow(dead_code)]` if intentional, or remove it to reduce codegen overhead",
    },
    BuildHint {
        crate_pattern: "",
        warning_code: Some("unexpected_cfgs"),
        suggestion: "Add missing feature to `[features]` in Cargo.toml or remove the #[cfg(feature=...)] guard",
    },
    BuildHint {
        crate_pattern: "",
        warning_code: Some("unused_imports"),
        suggestion: "Remove unused imports — they cause unnecessary monomorphization in generic crates",
    },
    BuildHint {
        crate_pattern: "",
        warning_code: Some("unused_variables"),
        suggestion: "Prefix with `_` (e.g. `_var`) or remove; unused variables prevent optimizer from eliminating allocations",
    },
];

/// Return the first matching suggestion for a crate + optional warning code.
/// Returns `None` when no pattern matches.
pub fn lookup_hint(crate_name: &str, warning_code: Option<&str>) -> Option<&'static str> {
    let name_lc = crate_name.to_lowercase();
    for hint in BUILD_HINTS {
        let code_matches = match (hint.warning_code, warning_code) {
            (Some(hc), Some(wc)) => hc == wc,
            (None, _) => true,
            (Some(_), None) => false,
        };
        let name_matches = hint.crate_pattern.is_empty() || {
            let p = hint.crate_pattern.to_lowercase();
            name_lc == p
                || name_lc.starts_with(&format!("{p}-"))
                || name_lc.starts_with(&format!("{p}_"))
                || name_lc.contains(&format!("-{p}"))
                || name_lc.contains(&format!("_{p}"))
        };
        if name_matches && code_matches {
            return Some(hint.suggestion);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokio_hint_matches() {
        let h = lookup_hint("tokio", None).unwrap();
        assert!(h.contains("Slim"));
    }

    #[test]
    fn warning_code_hint_matches() {
        let h = lookup_hint("some-crate", Some("unexpected_cfgs")).unwrap();
        assert!(h.contains("[features]"));
    }

    #[test]
    fn unknown_crate_no_code_returns_none() {
        assert!(lookup_hint("obscure-dep-xyz", None).is_none());
    }
}
