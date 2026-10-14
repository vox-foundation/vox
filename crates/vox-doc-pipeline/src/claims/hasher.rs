use regex::Regex;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn is_punct(c: char) -> bool {
    matches!(
        c,
        '(' | ')'
            | '{'
            | '}'
            | '['
            | ']'
            | ','
            | ':'
            | ';'
            | '='
            | '+'
            | '-'
            | '*'
            | '<'
            | '>'
            | '&'
            | '|'
            | '!'
            | '?'
            | '.'
            | '/'
    )
}

/// Compute a normalized SHA-256 hex digest for a code string.
/// Strips comments and collapses whitespace around punctuation so that
/// rustfmt reformatting, indentation, and comment edits do not cause hash churn.
pub fn hash_normalized_code(code: &str) -> String {
    let stripped = strip_comments(code);
    let chars: Vec<char> = stripped.chars().collect();
    let mut normalized = String::with_capacity(chars.len());
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if ch.is_whitespace() {
            // Check preceding non-whitespace character in `normalized`
            let prev_non_ws = normalized.chars().rev().find(|c| !c.is_whitespace());
            // Look ahead to next non-whitespace character in `chars`
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let next_non_ws = chars.get(j).copied();

            if let (Some(prev), Some(next)) = (prev_non_ws, next_non_ws) {
                // Only insert space if neither adjacent char is a punctuation token
                if !is_punct(prev) && !is_punct(next) && !normalized.ends_with(' ') {
                    normalized.push(' ');
                }
            }
            i = j;
        } else {
            normalized.push(ch);
            i += 1;
        }
    }

    let trimmed = normalized.trim();
    let mut hasher = Sha256::new();
    hasher.update(trimmed.as_bytes());
    let hash = hasher.finalize();
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Strip both line (`//`, `#`) and block (`/* ... */`) comments.
fn strip_comments(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut chars = code.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '/' {
            if let Some(&next_ch) = chars.peek() {
                if next_ch == '/' {
                    // Line comment: skip until newline
                    chars.next();
                    for c in chars.by_ref() {
                        if c == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                    continue;
                } else if next_ch == '*' {
                    // Block comment: skip until */
                    chars.next();
                    let mut prev = ' ';
                    for c in chars.by_ref() {
                        if prev == '*' && c == '/' {
                            break;
                        }
                        prev = c;
                    }
                    continue;
                }
            }
        } else if ch == '#' {
            // Shell / Python / YAML line comment
            for c in chars.by_ref() {
                if c == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        out.push(ch);
    }

    out
}

/// Recursively canonicalize JSON object keys into sorted BTreeMaps.
fn canonicalize_json(val: serde_json::Value) -> serde_json::Value {
    match val {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> = map
                .into_iter()
                .map(|(k, v)| (k, canonicalize_json(v)))
                .collect();
            serde_json::to_value(sorted).unwrap_or(serde_json::Value::Null)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(canonicalize_json).collect())
        }
        other => other,
    }
}

/// Normalize JSON / YAML by parsing and formatting with sorted keys.
pub fn hash_normalized_contract(content: &str) -> Result<String, String> {
    // Try JSON first
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(content) {
        let canonical_val = canonicalize_json(val);
        let canonical_str = serde_json::to_string(&canonical_val).map_err(|e| e.to_string())?;
        return Ok(hash_normalized_code(&canonical_str));
    }
    // Try YAML
    if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(content) {
        let json_val: serde_json::Value = serde_json::to_value(val).map_err(|e| e.to_string())?;
        let canonical_val = canonicalize_json(json_val);
        let canonical_str = serde_json::to_string(&canonical_val).map_err(|e| e.to_string())?;
        return Ok(hash_normalized_code(&canonical_str));
    }

    // Fallback to normalized text
    Ok(hash_normalized_code(content))
}

/// Extract symbol span from code text (functions, structs, enums, traits).
pub fn extract_symbol_span<'a>(code: &'a str, symbol: &str) -> Option<&'a str> {
    let escaped = regex::escape(symbol);
    let pattern_str = format!(
        r"(?m)(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:fn|struct|enum|trait|type|const|actor|workflow|component)\s+{}\b",
        escaped
    );
    let re = Regex::new(&pattern_str).ok()?;
    let mat = re.find(code)?;
    let start = mat.start();

    // Locate the balanced curly brace block if present
    let remainder = &code[start..];
    if let Some(open_brace_rel) = remainder.find('{') {
        let mut depth = 0;
        let mut end_rel = None;
        for (i, c) in remainder[open_brace_rel..].char_indices() {
            if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if depth == 0 {
                    end_rel = Some(open_brace_rel + i + 1);
                    break;
                }
            }
        }
        if let Some(end) = end_rel {
            return Some(&remainder[..end]);
        }
    }

    // If no brace (e.g. type alias or single line declaration), take until semicolon or newline
    if let Some(semi) = remainder.find(';') {
        return Some(&remainder[..=semi]);
    }

    Some(remainder.lines().next().unwrap_or(remainder))
}

/// Hash target anchor (file path or file#symbol).
pub fn hash_anchor(repo_root: &Path, target: &str) -> Result<String, String> {
    let (file_rel, symbol_opt) = match target.split_once('#') {
        Some((file, sym)) => (file.trim(), Some(sym.trim())),
        None => (target.trim(), None),
    };

    let resolved_file_rel = if !repo_root.join(file_rel).exists() {
        if file_rel.starts_with("vox-") {
            let crate_manifest = format!("crates/{}/Cargo.toml", file_rel);
            if repo_root.join(&crate_manifest).exists() {
                crate_manifest
            } else {
                file_rel.to_string()
            }
        } else if file_rel.starts_with("VOX_") || file_rel.starts_with("RUST_") {
            "contracts/config/env-vars.v1.yaml".to_string()
        } else if file_rel.starts_with("vox ") || file_rel == "vox" {
            "contracts/cli/command-registry.yaml".to_string()
        } else {
            file_rel.to_string()
        }
    } else {
        file_rel.to_string()
    };

    let file_path = repo_root.join(&resolved_file_rel);
    if !file_path.exists() {
        return Err(format!("File does not exist: {}", file_rel));
    }

    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read {}: {}", resolved_file_rel, e))?;

    if let Some(symbol) = symbol_opt {
        if let Some(span) = extract_symbol_span(&content, symbol) {
            return Ok(hash_normalized_code(span));
        } else {
            return Err(format!("Symbol '{}' not found in {}", symbol, file_rel));
        }
    }

    // Whole file hashing
    if file_rel.ends_with(".yaml") || file_rel.ends_with(".yml") || file_rel.ends_with(".json") {
        hash_normalized_contract(&content)
    } else {
        Ok(hash_normalized_code(&content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rustfmt_invariance() {
        let unformatted = r#"
            // Leading comment
            fn calculate( x :  i32,  y: i32 ) ->  i32 {
                /* Inline comment */
                let z = x + y;
                z * 2
            }
        "#;

        let formatted = r#"
            fn calculate(x: i32, y: i32) -> i32 {
                let z = x + y;
                z * 2
            }
        "#;

        let modified = r#"
            fn calculate(x: i32, y: i32) -> i32 {
                let z = x + y;
                z * 3
            }
        "#;

        let h1 = hash_normalized_code(unformatted);
        let h2 = hash_normalized_code(formatted);
        let h3 = hash_normalized_code(modified);

        assert_eq!(
            h1, h2,
            "Formatting and comments must produce identical hash"
        );
        assert_ne!(h1, h3, "Code change must produce different hash");
    }

    #[test]
    fn test_contract_canonicalization() {
        let yaml1 = "b: 2\na: 1\n";
        let yaml2 = "a: 1\nb: 2\n";
        let yaml3 = "a: 1\nb: 3\n";

        let h1 = hash_normalized_contract(yaml1).unwrap();
        let h2 = hash_normalized_contract(yaml2).unwrap();
        let h3 = hash_normalized_contract(yaml3).unwrap();

        assert_eq!(h1, h2, "Key order in contracts must not affect hash");
        assert_ne!(h1, h3, "Value change must change hash");
    }

    #[test]
    fn test_symbol_extraction() {
        let code = r#"
            struct Other;

            pub fn my_target_fn(a: u32) -> u32 {
                a + 1
            }

            struct Another;
        "#;

        let span = extract_symbol_span(code, "my_target_fn").unwrap();
        assert!(span.contains("pub fn my_target_fn"));
        assert!(span.contains("a + 1"));
        assert!(!span.contains("Another"));
    }
}
