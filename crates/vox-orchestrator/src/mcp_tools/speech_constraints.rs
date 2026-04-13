//! Speech-to-code “constrained decoding” integration (grammar **hints** only).
//!
//! **Not** tokenizer-level masking: there is no logits processor or CFG-enforced beam today. When
//! `contracts/speech-to-code/vox_grammar_artifact.json` is present we inject a short **controlled
//! vocabulary paragraph** into the system prompt so models favor legal surface tokens.
//!
//! **Hard gate:** MCP `vox_generate_code` still depends on `validate_document_with_hir` and bounded
//! diagnostic repair retries. Setting `VOX_MCP_GRAMMAR_MASK=1` records intent for a future bridge but
//! does not change generation semantics until a mask backend is wired.
//!
//! ## Staged roadmap (constrained generation)
//!
//! 1. **Today:** grammar artifact + output-surface guards + HIR repair loop (this module).
//! 2. **Next:** optional logits / mask adapter behind the same env flag, with automatic fallback to
//!    unmasked decode when the adapter is unavailable or times out.
//! 3. **Gate:** enable constrained decode by default only after compile\@k / latency benchmarks beat
//!    the validator-only baseline on the frozen speech benchmark manifest.
//! 
//! Refactored in April 2026 to separate policy logic and surface contracts.

use std::path::Path;

/// Suggested cap for bounded repair loops (speech-origin); MCP clamps user `max_retries` to 5.
pub const SPEECH_CODE_MAX_REPAIR_ATTEMPTS: u32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputSurfaceMode {
    #[default]
    RawCodeOnly,
    FencedTransport,
}

/// Placeholder hook for type-aware hints during generation (future).
#[derive(Debug, Clone, Default)]
pub struct TypeHintStub;

impl TypeHintStub {
    /// No-op until compiler symbol tables are wired into MCP generation.
    #[must_use]
    pub fn system_prompt_addon(&self) -> &'static str {
        let _ = std::hint::black_box(std::ptr::from_ref(self) as usize);
        ""
    }
}

/// Optional system-promptparagraph listing Vox lexer keywords (kept short; truncates if huge).
#[must_use]
pub fn grammar_artifact_prompt_addon(repo_root: &Path) -> String {
    let path = repo_root.join("contracts/speech-to-code/vox_grammar_artifact.json");
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return String::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return String::new();
    };
    let mut parts: Vec<String> = Vec::new();
    if let Some(arr) = v.get("keywords").and_then(|x| x.as_array()) {
        let kws: Vec<String> = arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .take(48)
            .collect();
        if !kws.is_empty() {
            parts.push(format!("Core Vox keywords: {}.", kws.join(", ")));
        }
    }
    if let Some(arr) = v.get("punctuators").and_then(|x| x.as_array()) {
        let p: Vec<String> = arr
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .take(24)
            .collect();
        if !p.is_empty() {
            parts.push(format!("Common punctuators: {}.", p.join(" ")));
        }
    }
    if parts.is_empty() {
        return String::new();
    }
    format!(
        "\nVox surface hints (prefer these spellings for syntax tokens):\n{}\n",
        parts.join("\n")
    )
}

/// Different tiers of constrained decoding enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstrainedDecodePolicy {
    /// No extra constraints beyond basic prompt.
    #[default]
    None,
    /// Soft hinting in prompt, but allows some variance.
    Soft,
    /// Rigid enforcement with prompt-guards and surface contract rejection.
    Rigid,
}

impl ConstrainedDecodePolicy {
    /// Resolve from env `VOX_MCP_GRAMMAR_MASK=1` or `VOX_MCP_DECODE_POLICY=rigid`.
    #[must_use]
    pub fn from_env() -> Self {
        let mask_env = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMcpGrammarMask);
        let policy_env = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxMcpDecodePolicy);
        
        let p = policy_env.expose().map(|s| s.to_ascii_lowercase());
        match p.as_deref() {
            Some("rigid") => Self::Rigid,
            Some("soft") => Self::Soft,
            Some("none") | Some("off") | Some("0") => Self::None,
            _ => {
                if matches!(mask_env.expose(), Some("1") | Some("true")) {
                    Self::Soft
                } else {
                    Self::None
                }
            }
        }
    }

    /// Whether any extra constraints (soft or rigid) are active.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// No-op placeholder (logs intent only at call sites).
    pub fn note_delegation_target(&self) {
        tracing::debug!(
            target: "vox_mcp_speech",
            policy = ?self,
            mode = "prompt_hint_and_hir_validator",
            "constrained decode status reported"
        );
    }

    /// Practical interim guard while token-mask backend is unavailable: reject common non-code wrappers.
    #[must_use]
    pub fn surface_contract_ok(&self, candidate: &str, mode: OutputSurfaceMode) -> bool {
        if matches!(self, Self::None) {
            return true;
        }
        let lower = candidate.to_ascii_lowercase();
        // Rigid policy is stricter on prose-like preamble
        let prose_ok = if matches!(self, Self::Rigid) {
            !lower.contains("here is")
                && !lower.contains("explanation")
                && !lower.contains("i can")
                && !lower.contains("of course")
        } else {
            true // Soft allows some preamble
        };

        match mode {
            OutputSurfaceMode::RawCodeOnly => prose_ok && !candidate.contains("```"),
            OutputSurfaceMode::FencedTransport => prose_ok,
        }
    }
}
