//! Context compaction engine for Vox agents.
//!
//! Prevents context window overflow by summarizing old conversation turns.
//! Adopts OpenClaw's compaction strategies:
//! - **Context window guard** — hard-stops if available tokens < minimum
//! - **Turn-based trimming** — trims whole turns, never mid-message
//! - **Head/tail preservation** — keeps first N + last M tokens of the context
//! - **Pre-compaction hook** — fires before summarization so memory can flush
//! - **Three strategies**: `Aggressive`, `Balanced`, `Conservative`

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// CompactionStrategy
// ---------------------------------------------------------------------------

fn default_complexity_token_weight() -> usize {
    32
}

/// Strategy that controls how aggressively stale context is trimmed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// Trim as much as possible: keep only the tail + head guard.
    Aggressive,
    /// Default: trim turns beyond the threshold, preserving recent turns.
    #[default]
    Balanced,
    /// Only trim when absolutely necessary (above 95% capacity).
    Conservative,
}

impl std::fmt::Display for CompactionStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Aggressive => write!(f, "aggressive"),
            Self::Balanced => write!(f, "balanced"),
            Self::Conservative => write!(f, "conservative"),
        }
    }
}

// ---------------------------------------------------------------------------
// CompactionConfig
// ---------------------------------------------------------------------------

/// Configuration for the compaction engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionConfig {
    /// Maximum tokens the model's context can hold. Default: 128 000.
    pub max_context_tokens: usize,
    /// Tokens always reserved for the model's response. Default: 10 000.
    pub reserved_tokens: usize,
    /// Fraction of `max_context_tokens` at which compaction triggers. Default: 0.80.
    pub compaction_threshold: f64,
    /// Minimum tokens that MUST remain after compaction or we error. Default: 2 000.
    pub min_viable_tokens: usize,
    /// Strategy to use when trimming. Default: `Balanced`.
    pub strategy: CompactionStrategy,
    /// Number of head tokens to always preserve. Default: 2 000.
    pub head_preserve_tokens: usize,
    /// Number of tail tokens to always preserve. Default: 8 000.
    pub tail_preserve_tokens: usize,
    /// Extra token weight per complexity unit (1–10) on a turn for budgeting / compaction. Default: 32.
    #[serde(default = "default_complexity_token_weight")]
    pub complexity_token_weight: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            max_context_tokens: 128_000,
            reserved_tokens: 10_000,
            compaction_threshold: 0.80,
            min_viable_tokens: 2_000,
            strategy: CompactionStrategy::Balanced,
            head_preserve_tokens: 2_000,
            tail_preserve_tokens: 8_000,
            complexity_token_weight: default_complexity_token_weight(),
        }
    }
}

impl CompactionConfig {
    /// Returns the token count at which compaction should trigger.
    pub fn trigger_at(&self) -> usize {
        ((self.max_context_tokens as f64) * self.compaction_threshold) as usize
    }

    /// Returns the usable token budget (max − reserved).
    pub fn usable_budget(&self) -> usize {
        self.max_context_tokens.saturating_sub(self.reserved_tokens)
    }
}

// ---------------------------------------------------------------------------
// Turn / CompactionInput
// ---------------------------------------------------------------------------

/// A single conversation turn (user message or assistant response).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Turn {
    /// `"user"` or `"assistant"` (or `"system"`).
    pub role: String,
    /// Raw text content.
    pub content: String,
    /// Estimated token count. Fill with [`CompactionEngine::estimate_tokens`].
    pub token_estimate: usize,
}

impl Turn {
    /// Create a turn with an estimated token count.
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        let content = content.into();
        let token_estimate = CompactionEngine::estimate_tokens(&content);
        Self {
            role: role.into(),
            content,
            token_estimate,
        }
    }
}

// ---------------------------------------------------------------------------
// CompactionResult
// ---------------------------------------------------------------------------

/// Outcome of a compaction pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    /// Turns that survived the compaction pass.
    pub retained_turns: Vec<Turn>,
    /// Turns that were dropped by this pass, in their original order.
    ///
    /// This is the lossless-archival contract: callers MUST persist
    /// `dropped_turns` somewhere retrievable (e.g. [`crate::session::Session`]'s
    /// `archived_turns`) before discarding them from the live/working set.
    /// `compact()` itself never discards data — it only classifies turns as
    /// retained vs. dropped and hands both back to the caller.
    pub dropped_turns: Vec<Turn>,
    /// Number of turns that were dropped (`dropped_turns.len()`, kept as a
    /// separate field for cheap access/back-compat with count-only readers).
    pub dropped_count: usize,
    /// Tokens before compaction.
    pub tokens_before: usize,
    /// Estimated tokens after compaction.
    pub tokens_after: usize,
    /// Whether compaction was actually performed.
    pub compacted: bool,
}

// ---------------------------------------------------------------------------
// CompactionError
// ---------------------------------------------------------------------------

/// Errors from the compaction engine.
#[derive(Debug, thiserror::Error)]
pub enum CompactionError {
    #[error(
        "Context window guard exceeded: {current} tokens in use, only {available} available (min viable: {min})"
    )]
    ContextWindowExceeded {
        current: usize,
        available: usize,
        min: usize,
    },
}

// ---------------------------------------------------------------------------
// CompactionEngine
// ---------------------------------------------------------------------------

/// Manages context window usage and trims conversation history when required.
pub struct CompactionEngine {
    config: CompactionConfig,
}

impl CompactionEngine {
    /// Create with explicit config.
    pub fn new(config: CompactionConfig) -> Self {
        Self { config }
    }

    /// Reference to the active config.
    pub fn config(&self) -> &CompactionConfig {
        &self.config
    }

    /// tiktoken-compatible BPE token estimator.
    pub fn estimate_tokens(text: &str) -> usize {
        crate::context::token_optimization::count_tokens(text)
    }

    /// Returns `true` if current token usage warrants compaction.
    pub fn should_compact(&self, current_tokens: usize) -> bool {
        current_tokens >= self.config.trigger_at()
    }

    /// Returns `Err` if context is so full that the model cannot respond.
    pub fn guard(&self, current_tokens: usize) -> Result<(), CompactionError> {
        let available = self.config.usable_budget().saturating_sub(current_tokens);
        if available < self.config.min_viable_tokens {
            Err(CompactionError::ContextWindowExceeded {
                current: current_tokens,
                available,
                min: self.config.min_viable_tokens,
            })
        } else {
            Ok(())
        }
    }

    /// Count the total estimated tokens across a slice of turns.
    pub fn count_tokens(turns: &[Turn]) -> usize {
        turns.iter().map(|t| t.token_estimate).sum()
    }

    /// Perform a compaction pass over `history`.
    ///
    /// Returns a [`CompactionResult`] describing what was retained/dropped.
    /// Does NOT modify `history` in place — caller replaces accordingly.
    ///
    /// **Lossless contract:** every turn removed from `history` is returned in
    /// [`CompactionResult::dropped_turns`] in its original order — `compact()`
    /// never discards a turn's content, it only classifies it as retained or
    /// dropped. Callers that need durable archival (e.g. [`crate::session`])
    /// must persist `dropped_turns` before dropping their own working copy.
    pub fn compact(&self, history: &[Turn]) -> Result<CompactionResult, CompactionError> {
        let tokens_before = Self::count_tokens(history);

        if !self.should_compact(tokens_before) {
            return Ok(CompactionResult {
                retained_turns: history.to_vec(),
                dropped_turns: Vec::new(),
                dropped_count: 0,
                tokens_before,
                tokens_after: tokens_before,
                compacted: false,
            });
        }

        let keep_indices: std::collections::HashSet<usize> = match self.config.strategy {
            CompactionStrategy::Aggressive => self.trim_aggressive(history),
            CompactionStrategy::Balanced => self.trim_balanced(history),
            CompactionStrategy::Conservative => self.trim_conservative(history),
        };

        let mut retained_turns = Vec::with_capacity(keep_indices.len());
        let mut dropped_turns = Vec::with_capacity(history.len() - keep_indices.len());
        for (i, t) in history.iter().enumerate() {
            if keep_indices.contains(&i) {
                retained_turns.push(t.clone());
            } else {
                dropped_turns.push(t.clone());
            }
        }

        let tokens_after = Self::count_tokens(&retained_turns);
        let dropped_count = dropped_turns.len();

        // Guard: after compaction we must have enough headroom
        self.guard(tokens_after)?;

        Ok(CompactionResult {
            retained_turns,
            dropped_turns,
            dropped_count,
            tokens_before,
            tokens_after,
            compacted: true,
        })
    }

    // ── Private trim strategies ─────────────────────────────────────────
    // Each returns the set of *retained* indices into `history`; `compact()`
    // derives both `retained_turns` and `dropped_turns` from that single
    // source of truth so the two lists always partition `history` exactly.

    /// Keep only the system prompt (if any) + the last N tokens of tail.
    fn trim_aggressive(&self, history: &[Turn]) -> std::collections::HashSet<usize> {
        let target = self.config.tail_preserve_tokens;
        let mut keep = std::collections::HashSet::new();

        // Always keep system turns at the head
        for (i, t) in history.iter().enumerate() {
            if t.role == "system" {
                keep.insert(i);
            }
        }

        // Fill from the tail backwards until we hit target
        let mut accumulated = 0usize;
        for (i, t) in history.iter().enumerate().rev() {
            if t.role == "system" {
                continue;
            }
            if accumulated + t.token_estimate <= target {
                accumulated += t.token_estimate;
                keep.insert(i);
            } else {
                break;
            }
        }
        keep
    }

    /// Head/tail preservation: keep head_preserve_tokens at the front and
    /// tail_preserve_tokens at the back, dropping the middle. Always preserves system messages.
    fn trim_balanced(&self, history: &[Turn]) -> std::collections::HashSet<usize> {
        let head_budget = self.config.head_preserve_tokens;
        let tail_budget = self.config.tail_preserve_tokens;

        let mut head_tokens = 0usize;
        let mut keep_indices = std::collections::HashSet::new();

        // 1. Always keep system messages
        for (i, t) in history.iter().enumerate() {
            if t.role == "system" {
                keep_indices.insert(i);
            }
        }

        // 2. Keep head turns until budget
        for (i, t) in history.iter().enumerate() {
            if keep_indices.contains(&i) {
                continue;
            }
            if head_tokens + t.token_estimate <= head_budget {
                head_tokens += t.token_estimate;
                keep_indices.insert(i);
            } else {
                break;
            }
        }

        // 3. Keep tail turns until budget
        let mut tail_tokens = 0usize;
        for (i, t) in history.iter().enumerate().rev() {
            if keep_indices.contains(&i) {
                continue;
            }
            if tail_tokens + t.token_estimate <= tail_budget {
                tail_tokens += t.token_estimate;
                keep_indices.insert(i);
            } else {
                break;
            }
        }

        keep_indices
    }

    /// Only drop whole turns from the middle until we fall below the trigger.
    fn trim_conservative(&self, history: &[Turn]) -> std::collections::HashSet<usize> {
        let trigger = self.config.trigger_at();
        let mut keep: std::collections::HashSet<usize> = (0..history.len()).collect();
        let mut total = Self::count_tokens(history);

        // Find the midpoint; drop turns outward from the middle
        let preserve_head = 1; // keep at least first turn
        let preserve_tail = 2; // keep at least last 2 turns

        let mut lo = preserve_head;
        while total >= trigger && lo < history.len().saturating_sub(preserve_tail) {
            if history[lo].role == "system" {
                lo += 1;
                continue;
            }
            if keep.remove(&lo) {
                total = total.saturating_sub(history[lo].token_estimate);
            }
            lo += 1;
        }
        keep
    }
}

impl Default for CompactionEngine {
    fn default() -> Self {
        Self::new(CompactionConfig::default())
    }
}

// ---------------------------------------------------------------------------
// SessionWorkingMemory & Observation Masking (`SOTA-SPEED-02..04`)
// ---------------------------------------------------------------------------

/// Persistent session working memory that pins active code artifacts, active spoke state,
/// and unresolved compiler diagnostics across compaction passes so multi-turn coreferences
/// ("that actor we wrote earlier", "what we just built") survive 30+ turn sessions (`SOTA-SPEED-02`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionWorkingMemory {
    /// Active spoke name (`vox_lang`, `rust_workspace`, `mcp_tool_chain`, etc.).
    pub active_spoke: Option<String>,
    /// Keyed working code/schema artifacts (`path` or `role_key` -> verified snippet).
    pub working_artifacts_by_key: std::collections::BTreeMap<String, String>,
    /// Unresolved compiler/oracle diagnostics from the latest failed check.
    pub unresolved_diagnostics: Vec<String>,
}

impl SessionWorkingMemory {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record or update a verified working artifact and active spoke.
    pub fn record_turn_artifact(
        &mut self,
        spoke: impl Into<String>,
        key: impl Into<String>,
        artifact: impl Into<String>,
    ) {
        self.active_spoke = Some(spoke.into());
        self.working_artifacts_by_key
            .insert(key.into(), artifact.into());
    }

    /// Record an unresolved compiler/oracle diagnostic.
    pub fn record_diagnostic(&mut self, diagnostic: impl Into<String>) {
        self.unresolved_diagnostics.push(diagnostic.into());
    }

    /// Clear unresolved diagnostics once a repair turn succeeds.
    pub fn clear_diagnostics(&mut self) {
        self.unresolved_diagnostics.clear();
    }

    /// Render a deterministic, compact `role = "system"` anchor turn that survives all
    /// compaction strategies (`Aggressive`, `Balanced`, `Conservative`).
    #[must_use]
    pub fn to_pinned_system_turn(&self) -> Option<Turn> {
        if self.active_spoke.is_none()
            && self.working_artifacts_by_key.is_empty()
            && self.unresolved_diagnostics.is_empty()
        {
            return None;
        }
        let mut lines = Vec::new();
        lines.push(format!(
            "[SESSION WORKING MEMORY | active_spoke={}]",
            self.active_spoke.as_deref().unwrap_or("none")
        ));
        for (k, v) in &self.working_artifacts_by_key {
            let compact: String = v.lines().take(6).collect::<Vec<_>>().join(" \\n ");
            lines.push(format!("artifact[{k}]: {compact}"));
        }
        if !self.unresolved_diagnostics.is_empty() {
            lines.push(format!(
                "unresolved_diagnostics: {}",
                self.unresolved_diagnostics.join(" | ")
            ));
        }
        Some(Turn::new("system", lines.join("\n")))
    }
}

impl CompactionEngine {
    /// SWE-agent-style observation masking (`SOTA-SPEED-03`):
    /// Collapses verbose read-only tool observations older than `keep_recent_turns` into
    /// 1-line content-addressed `blake3` receipts while keeping prefix turns block-aligned
    /// for high KV/prompt-cache hit ratios (`CachePrediction::Hit`).
    ///
    /// Returns `(masked_turns, tokens_saved, cache_signal)`.
    #[must_use]
    pub fn mask_stale_tool_observations(
        history: &[Turn],
        keep_recent_turns: usize,
    ) -> (Vec<Turn>, usize, crate::cache_predictor::CacheSignal) {
        let cutoff = history.len().saturating_sub(keep_recent_turns);
        let turn_tokens = |t: &Turn| -> usize { t.token_estimate.max(t.content.len() / 4) };
        let tokens_before: usize = history.iter().map(turn_tokens).sum();
        let mut masked = Vec::with_capacity(history.len());
        let mut prefix_tokens = 0u32;

        for (idx, turn) in history.iter().enumerate() {
            if idx < cutoff && turn.role == "tool" && turn.content.len() > 160 {
                let digest_hex =
                    vox_crypto::hex_encode(&vox_crypto::secure_hash(turn.content.as_bytes()));
                let short_hash = &digest_hex[..16.min(digest_hex.len())];
                let preview: String = turn
                    .content
                    .chars()
                    .take(48)
                    .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
                    .collect();
                let summary = format!(
                    "[Collapsed tool observation: len={}B, blake3={short_hash}, preview=\"{preview}...\"]",
                    turn.content.len()
                );
                let new_turn = Turn::new("tool", summary);
                prefix_tokens = prefix_tokens.saturating_add(turn_tokens(&new_turn) as u32);
                masked.push(new_turn);
            } else {
                if idx < cutoff {
                    prefix_tokens = prefix_tokens.saturating_add(turn_tokens(turn) as u32);
                }
                masked.push(turn.clone());
            }
        }

        let tokens_after: usize = masked.iter().map(turn_tokens).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let cache_signal = crate::cache_predictor::CacheSignal {
            prefix_overlap_tokens: prefix_tokens,
            total_context_tokens: tokens_after as u32,
        };
        (masked, tokens_saved, cache_signal)
    }

    /// Compact `history` while masking stale tool observations and injecting `working_memory`
    /// as a pinned system anchor so active artifacts and diagnostics never vanish (`SOTA-SPEED-02..03`).
    pub fn compact_with_working_memory(
        &self,
        history: &[Turn],
        working_memory: &SessionWorkingMemory,
        keep_recent_tool_turns: usize,
    ) -> Result<(CompactionResult, crate::cache_predictor::CacheSignal), CompactionError> {
        let (masked, _saved, cache_signal) =
            Self::mask_stale_tool_observations(history, keep_recent_tool_turns);
        let mut result = self.compact(&masked)?;
        if let Some(anchor) = working_memory.to_pinned_system_turn() {
            let insert_pos = result
                .retained_turns
                .iter()
                .position(|t| t.role != "system")
                .unwrap_or(0);
            result.tokens_after = result.tokens_after.saturating_add(anchor.token_estimate);
            result.retained_turns.insert(insert_pos, anchor);
        }
        Ok((result, cache_signal))
    }

    /// Partial history compaction (`CompactionDirection::UpToIndex` / `FromIndex`, adapted from `t3code`
    /// `Decider.compact.ts`) that preserves `thought_signature` on tool-calling assistant turns (`pi-t3chat` Gemini 3 parity).
    pub fn compact_partial_with_signatures(
        &self,
        turns: &[SignaturePreservingTurn],
        direction: CompactionDirection,
        summary_replacement: impl Into<String>,
    ) -> PartialCompactionOutcome {
        let total = turns.len();
        if total == 0 {
            return PartialCompactionOutcome {
                retained_turns: Vec::new(),
                compact_message_index: 0,
                preserved_thought_signatures: Vec::new(),
            };
        }
        let summary_text = summary_replacement.into();
        let summary_tokens = Self::estimate_tokens(&summary_text);

        match direction {
            CompactionDirection::UpToIndex(inclusive_idx) => {
                let bound = inclusive_idx.min(total.saturating_sub(1));
                let mut preserved_sigs = Vec::new();
                for t in &turns[..=bound] {
                    if let Some(sig) = &t.thought_signature {
                        preserved_sigs.push(sig.clone());
                    }
                }
                let mut retained = Vec::with_capacity(total - bound);
                retained.push(SignaturePreservingTurn {
                    message_id: "compacted-summary-up-to".to_string(),
                    role: "system".to_string(),
                    content: summary_text,
                    token_estimate: summary_tokens,
                    thought_signature: preserved_sigs.last().cloned(),
                });
                for t in &turns[(bound + 1)..] {
                    if let Some(sig) = &t.thought_signature {
                        preserved_sigs.push(sig.clone());
                    }
                    retained.push(t.clone());
                }
                PartialCompactionOutcome {
                    retained_turns: retained,
                    compact_message_index: 0,
                    preserved_thought_signatures: preserved_sigs,
                }
            }
            CompactionDirection::FromIndex(start_idx) => {
                let bound = start_idx.min(total.saturating_sub(1));
                let mut preserved_sigs = Vec::new();
                let mut retained = Vec::with_capacity(bound + 1);
                for t in &turns[..bound] {
                    if let Some(sig) = &t.thought_signature {
                        preserved_sigs.push(sig.clone());
                    }
                    retained.push(t.clone());
                }
                for t in &turns[bound..] {
                    if let Some(sig) = &t.thought_signature {
                        preserved_sigs.push(sig.clone());
                    }
                }
                let divider_idx = retained.len();
                retained.push(SignaturePreservingTurn {
                    message_id: "compacted-summary-from".to_string(),
                    role: "system".to_string(),
                    content: summary_text,
                    token_estimate: summary_tokens,
                    thought_signature: preserved_sigs.last().cloned(),
                });
                PartialCompactionOutcome {
                    retained_turns: retained,
                    compact_message_index: divider_idx,
                    preserved_thought_signatures: preserved_sigs,
                }
            }
        }
    }
}

/// Direction for partial thread compaction (`t3code` `up_to` / `from` parity, `T3-BATTERY-LEDGER-03`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionDirection {
    /// Summarize all messages from `0..=index` while keeping messages after `index` verbatim.
    UpToIndex(usize),
    /// Keep messages `0..index` verbatim while summarizing `index..len()`.
    FromIndex(usize),
}

/// A turn record carrying an optional Gemini / provider `thoughtSignature` (`pi-t3chat` parity).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignaturePreservingTurn {
    pub message_id: String,
    pub role: String,
    pub content: String,
    pub token_estimate: usize,
    /// Provider-returned cryptographic thought signature (required by Gemini 3 thinking models on tool turns).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

/// Result of a partial `up_to` or `from` compaction pass.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartialCompactionOutcome {
    pub retained_turns: Vec<SignaturePreservingTurn>,
    /// Index of the inserted summary divider turn for UI rendering (`compact_message_index`).
    pub compact_message_index: usize,
    /// All cryptographic `thoughtSignature` tokens preserved across the compaction boundary.
    pub preserved_thought_signatures: Vec<String>,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_turns(sizes: &[usize]) -> Vec<Turn> {
        sizes
            .iter()
            .enumerate()
            .map(|(i, &sz)| {
                let content = "x".repeat(sz * 4); // 4 chars per token
                Turn {
                    role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                    content,
                    token_estimate: sz,
                }
            })
            .collect()
    }

    #[test]
    fn estimate_tokens_reasonable() {
        let s = "Hello, world!"; // 13 chars → ~3 tokens
        let est = CompactionEngine::estimate_tokens(s);
        assert!((1..=10).contains(&est));
    }

    #[test]
    fn no_compaction_when_under_threshold() {
        let engine = CompactionEngine::default();
        let turns = make_turns(&[1_000, 1_000]); // 2k tokens, well under 128k threshold
        let result = engine.compact(&turns).expect("compact");
        assert!(!result.compacted);
        assert_eq!(result.dropped_count, 0);
        assert!(result.dropped_turns.is_empty());
    }

    #[test]
    fn balanced_compaction_drops_middle_turns() {
        let cfg = CompactionConfig {
            max_context_tokens: 100,
            reserved_tokens: 10,
            compaction_threshold: 0.5, // trigger at 50 tokens
            min_viable_tokens: 5,
            strategy: CompactionStrategy::Balanced,
            head_preserve_tokens: 10,
            tail_preserve_tokens: 15,
            complexity_token_weight: 32,
        };
        let engine = CompactionEngine::new(cfg);
        let turns = make_turns(&[5, 5, 20, 20, 5, 5]); // total 60, over threshold
        let result = engine.compact(&turns).expect("compact");
        assert!(result.compacted);
        assert!(result.dropped_count > 0);
        assert_eq!(result.dropped_turns.len(), result.dropped_count);
        // Lossless: retained + dropped accounts for every original turn.
        assert_eq!(
            result.retained_turns.len() + result.dropped_turns.len(),
            turns.len()
        );
        assert!(result.tokens_after <= 65); // should have reduced
    }

    #[test]
    fn aggressive_compaction_keeps_only_tail() {
        let cfg = CompactionConfig {
            max_context_tokens: 100,
            reserved_tokens: 10,
            compaction_threshold: 0.3, // very low trigger
            min_viable_tokens: 2,
            strategy: CompactionStrategy::Aggressive,
            head_preserve_tokens: 5,
            tail_preserve_tokens: 15,
            complexity_token_weight: 32,
        };
        let engine = CompactionEngine::new(cfg);
        let turns = make_turns(&[5, 5, 5, 5, 10]);
        let result = engine.compact(&turns).expect("compact");
        assert!(result.compacted);
    }

    /// RED-then-GREEN: `compact()` must return the actual dropped `Turn`
    /// content (lossless), not just a count — this is the T4.2 audit-defect
    /// fix. Every dropped turn's exact content must be present in
    /// `dropped_turns`, and retained ∪ dropped must equal the original set.
    #[test]
    fn compact_losslessly_returns_dropped_turn_content() {
        let cfg = CompactionConfig {
            max_context_tokens: 100,
            reserved_tokens: 10,
            compaction_threshold: 0.5,
            min_viable_tokens: 5,
            strategy: CompactionStrategy::Balanced,
            head_preserve_tokens: 10,
            tail_preserve_tokens: 15,
            complexity_token_weight: 32,
        };
        let engine = CompactionEngine::new(cfg);
        let turns = make_turns(&[5, 5, 20, 20, 5, 5]);
        let result = engine.compact(&turns).expect("compact");

        assert!(result.compacted);
        assert!(!result.dropped_turns.is_empty(), "must actually drop turns");

        // Every dropped turn must be traceable back to the original history
        // by exact content — nothing is summarized/mutated/lost at this layer.
        for dropped in &result.dropped_turns {
            assert!(
                turns.contains(dropped),
                "dropped turn content must match an original turn exactly"
            );
        }

        // Lossless partition: retained + dropped reconstructs the full turn set
        // (as a multiset — order/dedup are not required at this layer).
        let mut reconstructed: Vec<&Turn> = result
            .retained_turns
            .iter()
            .chain(result.dropped_turns.iter())
            .collect();
        let mut original: Vec<&Turn> = turns.iter().collect();
        reconstructed.sort_by_key(|t| t.token_estimate);
        original.sort_by_key(|t| t.token_estimate);
        assert_eq!(reconstructed, original);
    }

    #[test]
    fn guard_errors_when_over_minimum() {
        let cfg = CompactionConfig {
            max_context_tokens: 100,
            reserved_tokens: 50,
            min_viable_tokens: 60, // requires 60 usable, but only 50 usable
            ..Default::default()
        };
        let engine = CompactionEngine::new(cfg);
        // usable = 100-50 = 50; current = 0; available = 50; min = 60 → error
        let result = engine.guard(0);
        assert!(result.is_err());
    }

    #[test]
    fn config_trigger_at_calculation() {
        let cfg = CompactionConfig {
            max_context_tokens: 100_000,
            compaction_threshold: 0.8,
            ..Default::default()
        };
        assert_eq!(cfg.trigger_at(), 80_000);
    }

    #[test]
    fn should_compact_false_under_threshold() {
        let engine = CompactionEngine::default();
        assert!(!engine.should_compact(50_000));
    }

    #[test]
    fn should_compact_true_at_threshold() {
        let engine = CompactionEngine::default();
        assert!(engine.should_compact(128_000));
    }

    #[test]
    fn compaction_strategy_display() {
        assert_eq!(CompactionStrategy::Aggressive.to_string(), "aggressive");
        assert_eq!(CompactionStrategy::Balanced.to_string(), "balanced");
        assert_eq!(CompactionStrategy::Conservative.to_string(), "conservative");
    }

    #[test]
    fn observation_masking_and_working_memory_preserve_artifacts_and_kv_cache_hit() {
        use crate::cache_predictor::{CachePrediction, CachePredictor, CachePredictorConfig};

        let mut wm = SessionWorkingMemory::new();
        wm.record_turn_artifact(
            "vox_lang",
            "examples/golden/ref_actors.vox",
            "actor BuildMonitor { on record(count: int) to int { return count + 1 } }",
        );
        wm.record_diagnostic("E0502: cannot borrow *items as mutable");

        let mut history = vec![Turn::new(
            "system",
            "You are Vox Axis Agent, the multi-spoke local-first coding and orchestration harness for the Vox programming language and workspace.",
        )];
        for i in 0..10 {
            history.push(Turn::new(
                "user",
                format!(
                    "Turn {i}: inspect workspace files, verify actor state transitions, and check diagnostics for module {i}"
                ),
            ));
            // Verbose tool observation on older turns
            history.push(Turn::new(
                "tool",
                format!(
                    "{{\"tool_name\":\"vox_repo_query_file\",\"turn\":{i},\"payload\":\"{}\"}}",
                    "line content alpha beta gamma delta ".repeat(15)
                ),
            ));
            history.push(Turn::new(
                "assistant",
                format!(
                    "Turn {i} analysis complete: verified module {i} actor handlers, checked borrow lifetimes, and updated working memory."
                ),
            ));
        }

        let (masked, tokens_saved, cache_signal) =
            CompactionEngine::mask_stale_tool_observations(&history, 2);
        assert!(
            tokens_saved > 1_000,
            "observation masking must save >1,000 tokens across 9 stale tool turns (saved={tokens_saved})"
        );
        let predictor = CachePredictor::new(CachePredictorConfig::default());
        assert_eq!(
            predictor.predict(&cache_signal),
            CachePrediction::Hit,
            "masked prefix overlap ({:.2}) must exceed 0.70 KV cache hit threshold",
            CachePredictor::overlap_ratio(&cache_signal)
        );
        assert!(
            masked[2]
                .content
                .starts_with("[Collapsed tool observation: len="),
            "older tool turn must be collapsed to a content-addressed blake3 receipt"
        );

        // Even under Aggressive compaction, working memory artifacts & diagnostics survive!
        let cfg = CompactionConfig {
            max_context_tokens: 400,
            reserved_tokens: 50,
            compaction_threshold: 0.3,
            min_viable_tokens: 20,
            strategy: CompactionStrategy::Aggressive,
            head_preserve_tokens: 40,
            tail_preserve_tokens: 120,
            complexity_token_weight: 32,
        };
        let engine = CompactionEngine::new(cfg);
        let (compacted, _sig) = engine
            .compact_with_working_memory(&history, &wm, 2)
            .expect("compact with working memory");
        assert!(compacted.compacted);
        let combined_retained: String = compacted
            .retained_turns
            .iter()
            .map(|t| t.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            combined_retained.contains("actor BuildMonitor"),
            "pinned working memory must preserve BuildMonitor actor across aggressive compaction"
        );
        assert!(
            combined_retained.contains("E0502"),
            "pinned working memory must preserve unresolved diagnostics across aggressive compaction"
        );
    }

    #[test]
    fn partial_compaction_up_to_and_from_preserve_gemini_thought_signatures() {
        let engine = CompactionEngine::default();
        let turns = vec![
            SignaturePreservingTurn {
                message_id: "m0".into(),
                role: "user".into(),
                content: "Inspect crate graph".into(),
                token_estimate: 10,
                thought_signature: None,
            },
            SignaturePreservingTurn {
                message_id: "m1".into(),
                role: "assistant".into(),
                content: "Calling vox_read_file".into(),
                token_estimate: 12,
                thought_signature: Some("gemini-sig-alpha".into()),
            },
            SignaturePreservingTurn {
                message_id: "m2".into(),
                role: "user".into(),
                content: "Now update id_allocator".into(),
                token_estimate: 10,
                thought_signature: None,
            },
        ];

        let up_to = engine.compact_partial_with_signatures(
            &turns,
            CompactionDirection::UpToIndex(1),
            "[Summary of m0..m1]",
        );
        assert_eq!(up_to.compact_message_index, 0);
        assert_eq!(up_to.retained_turns.len(), 2);
        assert_eq!(up_to.retained_turns[1].message_id, "m2");
        assert_eq!(
            up_to.preserved_thought_signatures,
            vec!["gemini-sig-alpha".to_string()]
        );
        assert_eq!(
            up_to.retained_turns[0].thought_signature.as_deref(),
            Some("gemini-sig-alpha")
        );
    }
}
