//! Design-token registry — compiled from `vox.tokens.json` into typed Rust structures.
//!
//! Flattens `{"category": {"name": "value"}}` into `"category.name"` → `"value"`.
//! Skip the top-level `"version"` key. For nested objects (e.g. typography), store
//! the raw JSON string. For arrays (e.g. `surface.pairs`), index by position and by
//! the item's `"name"` field if present.

pub mod validate;

use std::collections::HashMap;

use serde_json::Value;

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Errors that may occur when loading a token registry.
#[derive(Debug)]
pub enum TokenLoadError {
    IoError(String),
    ParseFailed(String),
}

impl std::fmt::Display for TokenLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenLoadError::IoError(msg) => write!(f, "token io error: {msg}"),
            TokenLoadError::ParseFailed(msg) => write!(f, "token parse failed: {msg}"),
        }
    }
}

impl std::error::Error for TokenLoadError {}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Flattened design-token registry.
///
/// Keys use dotted notation: `"color.primary"`, `"spacing.md"`, etc.
#[derive(Debug, Default, Clone)]
pub struct TokenRegistry {
    map: HashMap<String, String>,
}

impl TokenRegistry {
    /// Parse and flatten a `vox.tokens.json` string into a [`TokenRegistry`].
    ///
    /// - Top-level `"version"` key is skipped.
    /// - Nested object values are stored as their raw JSON string.
    /// - Array entries are indexed by their position (`"<cat>.0"`) and also by
    ///   `"<cat>.<name-field>"` when an item has a `"name"` field.
    pub fn load_from_str(json: &str) -> Result<Self, TokenLoadError> {
        let root: Value =
            serde_json::from_str(json).map_err(|e| TokenLoadError::ParseFailed(e.to_string()))?;

        let obj = root
            .as_object()
            .ok_or_else(|| TokenLoadError::ParseFailed("root must be a JSON object".to_string()))?;

        let mut map = HashMap::new();

        for (category, value) in obj {
            if category == "version" {
                continue;
            }
            flatten_value(category, value, &mut map);
        }

        Ok(TokenRegistry { map })
    }

    /// Return `true` if `key` exists in the registry.
    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    /// Return the value for `key`, or `None` if not found.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.map.get(key).map(String::as_str)
    }

    /// Return all keys sorted alphabetically.
    pub fn all_keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = self.map.keys().map(String::as_str).collect();
        keys.sort_unstable();
        keys
    }

    /// Return keys whose Levenshtein distance from `unknown` is ≤ 2, sorted.
    pub fn suggest(&self, unknown: &str) -> Vec<String> {
        let mut matches: Vec<String> = self
            .map
            .keys()
            .filter(|k| edit_distance(k.as_str(), unknown) <= 2)
            .cloned()
            .collect();
        matches.sort();
        matches
    }

    /// Number of tokens in the registry.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// `true` when the registry holds no tokens.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn flatten_value(prefix: &str, value: &Value, out: &mut HashMap<String, String>) {
    match value {
        Value::Object(obj) => {
            for (k, v) in obj {
                let child_key = format!("{prefix}.{k}");
                match v {
                    // Leaf string → store directly.
                    Value::String(s) => {
                        out.insert(child_key, s.clone());
                    }
                    // Nested object → store as JSON string (e.g. typography groups).
                    Value::Object(_) => {
                        out.insert(child_key.clone(), v.to_string());
                        // Also recurse so deep keys are individually accessible.
                        flatten_value(&child_key, v, out);
                    }
                    // Array → recurse as array.
                    Value::Array(_) => {
                        flatten_array(&child_key, v.as_array().unwrap(), out);
                    }
                    other => {
                        out.insert(child_key, other.to_string());
                    }
                }
            }
        }
        Value::Array(arr) => {
            flatten_array(prefix, arr, out);
        }
        Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

fn flatten_array(prefix: &str, arr: &[Value], out: &mut HashMap<String, String>) {
    for (i, item) in arr.iter().enumerate() {
        let pos_key = format!("{prefix}.{i}");
        // Store the whole item as JSON.
        out.insert(pos_key.clone(), item.to_string());
        // Also index by name field if present.
        if let Some(name) = item.get("name").and_then(Value::as_str) {
            let name_key = format!("{prefix}.{name}");
            out.insert(name_key, item.to_string());
        }
    }
}

/// Levenshtein edit distance between two strings.
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let m = a.len();
    let n = b.len();

    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 0..=m {
        dp[i][0] = i;
    }
    for j in 0..=n {
        dp[0][j] = j;
    }
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1]
            } else {
                1 + dp[i - 1][j - 1].min(dp[i - 1][j]).min(dp[i][j - 1])
            };
        }
    }
    dp[m][n]
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_minimal_tokens() {
        let json = r##"{"color":{"primary":"#3a86ff"},"spacing":{"md":"16px"}}"##;
        let reg = TokenRegistry::load_from_str(json).unwrap();
        assert!(reg.contains("color.primary"));
        assert!(reg.contains("spacing.md"));
        assert!(!reg.contains("color.nonexistent"));
    }

    #[test]
    fn suggest_close_match() {
        let json = r##"{"color":{"primary":"#3a86ff","secondary":"#ff006e"}}"##;
        let reg = TokenRegistry::load_from_str(json).unwrap();
        let suggestions = reg.suggest("color.primry"); // typo
        assert!(
            suggestions.iter().any(|s| s == "color.primary"),
            "got: {suggestions:?}"
        );
    }

    #[test]
    fn skip_version_key() {
        let json = r##"{"version":"1.0","color":{"bg":"#fff"}}"##;
        let reg = TokenRegistry::load_from_str(json).unwrap();
        assert!(!reg.contains("version"), "version should not be a token");
        assert!(reg.contains("color.bg"));
    }
}
