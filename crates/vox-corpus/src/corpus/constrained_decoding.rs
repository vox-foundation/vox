//! Modes for constrained generation (wired to inference servers).

/// How tightly to constrain token generation for structured outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstrainedDecodingMode {
    /// No extra constraints beyond sampling.
    #[default]
    None,
    /// Prefer valid JSON prefixes (logit processing in GPU path).
    JsonPrefix,
    /// Enforce strict JSON object shape post-hoc.
    StrictJson,
}

impl ConstrainedDecodingMode {
    /// Parse a snake-case or kebab-case label; unknown values map to [`Self::None`].
    pub fn parse(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "json_prefix" | "json-prefix" => ConstrainedDecodingMode::JsonPrefix,
            "strict_json" | "strict-json" => ConstrainedDecodingMode::StrictJson,
            _ => ConstrainedDecodingMode::None,
        }
    }
}
