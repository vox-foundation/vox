//! Speech-to-code “constrained decoding” integration (grammar **hints** only).
//!
//! **Not** tokenizer-level masking: there is no logits processor or CFG-enforced beam today. When
//! `contracts/speech-to-code/vox_grammar_artifact.json` is present we inject a short **controlled
//! vocabulary paragraph** into the system prompt so models favor legal surface tokens.
//!
//! **Hard gate:** MCP `vox_generate_code` still depends on `validate_document_with_hir` and bounded
//! diagnostic repair retries. Setting `VOX_MCP_GRAMMAR_MASK=1` records intent for a future bridge but
//! does not change generation semantics until a mask backend is wired.

use std::path::Path;

/// Suggested cap for bounded repair loops (speech-origin); MCP clamps user `max_retries` to 5.
pub const SPEECH_CODE_MAX_REPAIR_ATTEMPTS: u32 = 5;

/// Placeholder hook for type-aware hints during generation (future).
#[derive(Debug, Clone, Default)]
pub struct TypeHintStub;

impl TypeHintStub {
    /// No-op until compiler symbol tables are wired into MCP generation.
    #[must_use]
    pub fn system_prompt_addon(&self) -> &'static str {
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

/// Stub for future grammar-masked decoding backends (Outlines / custom logits processor).
#[derive(Debug, Clone, Copy, Default)]
pub struct ConstrainedDecodePolicy {
    /// When true, attempt mask-backed generation once a bridge exists.
    pub enabled: bool,
}

impl ConstrainedDecodePolicy {
    /// Resolve from env `VOX_MCP_GRAMMAR_MASK=1` (default off).
    #[must_use]
    pub fn from_env() -> Self {
        let enabled = matches!(
            std::env::var("VOX_MCP_GRAMMAR_MASK").as_deref(),
            Ok("1") | Ok("true")
        );
        Self { enabled }
    }

    /// No-op placeholder (logs intent only at call sites).
    pub fn note_delegation_target(&self) {
        tracing::debug!(
            target: "vox_mcp_speech",
            grammar_mask_enabled = self.enabled,
            mode = "prompt_hint_and_hir_validator",
            "constrained decode: no token-mask backend; grammar artifact is prompt-only; HIR validator is the gate"
        );
    }
}
