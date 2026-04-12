//! Pushdown automaton (PDA) backend for grammar-constrained token sampling.
//!
//! This backend uses a stack-based PDA driven by the Vox EBNF grammar.
//! It implements **context-independent token caching** (XGrammar-2 strategy):
//! tokens whose validity depends only on the top-of-stack symbol — not the full
//! stack — are cached in a lookup table for O(1) amortised masking.
//!
//! **Research rationale (Grammar Constraints §4.2):** PDA backends achieve
//! <40µs/token overhead with native recursion support, compared to >200µs for
//! naive Earley on large vocabularies.

use std::collections::{HashMap, HashSet};

use tracing::debug;

use crate::earley::{Grammar, Symbol};
use crate::{ConstrainedGenError, ConstrainedSampler, Result, SamplerState};

// ── PDA state ────────────────────────────────────────────────────────────────

/// Stack-based PDA state persisted between generation steps.
#[derive(Debug, Clone)]
pub struct PdaState {
    /// The PDA stack. Each entry is a non-terminal name waiting to be expanded
    /// or a terminal expectation that must be matched next.
    stack: Vec<StackEntry>,
    /// Number of tokens consumed.
    position: usize,
    /// Accumulated text for diagnostics.
    partial: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum StackEntry {
    /// Expect this non-terminal to be expanded.
    NonTerminal(String),
    /// Expect this exact terminal string.
    Terminal(String),
    /// Expect an identifier character class.
    IdentClass,
    /// Expect a digit class.
    DigitClass,
}

impl PdaState {
    fn new(start: &str) -> Self {
        Self {
            stack: vec![StackEntry::NonTerminal(start.to_string())],
            position: 0,
            partial: String::new(),
        }
    }

    /// Attempt to consume `token_text` from the current PDA state.
    ///
    /// Returns `Some(new_states)` — a *set* of possible successor states
    /// (because a non-terminal may have multiple productions). Returns an
    /// empty vec if no transition is valid.
    fn try_consume(&self, grammar: &Grammar, token_text: &str) -> Vec<PdaState> {
        if token_text.is_empty() {
            return vec![self.clone()];
        }

        let mut results = Vec::new();
        self.try_consume_inner(grammar, token_text, &mut results);
        results
    }

    fn try_consume_inner(&self, grammar: &Grammar, token_text: &str, out: &mut Vec<PdaState>) {
        if self.stack.is_empty() {
            // Stack empty — only valid if token is also consumed.
            // Empty stack + non-empty token means this path is dead.
            return;
        }

        let top = self.stack.last().unwrap().clone();

        match &top {
            StackEntry::Terminal(lit) => {
                // Check if token_text starts with or is a prefix of the literal.
                if lit.starts_with(token_text) {
                    let remainder = &lit[token_text.len()..];
                    let mut new = self.clone();
                    new.stack.pop();
                    if !remainder.is_empty() {
                        new.stack.push(StackEntry::Terminal(remainder.to_string()));
                    }
                    new.position += 1;
                    new.partial.push_str(token_text);
                    out.push(new);
                } else if token_text.starts_with(lit.as_str()) {
                    // Token overshoots the literal — consume the literal part,
                    // then try to consume the remainder against the next stack entry.
                    let remainder = &token_text[lit.len()..];
                    let mut new = self.clone();
                    new.stack.pop();
                    new.partial.push_str(lit);
                    if !remainder.is_empty() {
                        new.try_consume_inner(grammar, remainder, out);
                    } else {
                        new.position += 1;
                        out.push(new);
                    }
                }
            }
            StackEntry::IdentClass => {
                if token_text.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    let mut new = self.clone();
                    new.stack.pop();
                    new.position += 1;
                    new.partial.push_str(token_text);
                    out.push(new);
                }
            }
            StackEntry::DigitClass => {
                if token_text.chars().all(|c| c.is_ascii_digit()) {
                    let mut new = self.clone();
                    new.stack.pop();
                    new.position += 1;
                    new.partial.push_str(token_text);
                    out.push(new);
                }
            }
            StackEntry::NonTerminal(nt) => {
                // Expand: try every production for this NT.
                if let Some(indices) = grammar.index.get(nt) {
                    for &idx in indices {
                        let prod = &grammar.productions[idx];
                        let mut new = self.clone();
                        new.stack.pop();
                        // Push RHS symbols in reverse (so first symbol is on top).
                        for sym in prod.symbols.iter().rev() {
                            new.stack.push(match sym {
                                Symbol::Terminal(t) => StackEntry::Terminal(t.clone()),
                                Symbol::NonTerminal(n) => StackEntry::NonTerminal(n.clone()),
                                Symbol::IdentClass => StackEntry::IdentClass,
                                Symbol::DigitClass => StackEntry::DigitClass,
                            });
                        }
                        // Now try consuming with the expanded stack.
                        new.try_consume_inner(grammar, token_text, out);
                    }
                }
            }
        }
    }
}

// ── Token cache ──────────────────────────────────────────────────────────────

/// Context-independent token validity cache.
///
/// For each top-of-stack symbol, caches which token strings are valid
/// transitions. This avoids re-computing PDA transitions for the common case
/// where the top symbol is the same across multiple generation steps.
#[derive(Debug, Default)]
struct TokenCache {
    /// Map from top-of-stack entry → set of valid token indices.
    cache: HashMap<StackEntry, HashSet<usize>>,
}

impl TokenCache {
    /// Check or compute whether `token_idx` with text `token_text` is valid
    /// when the PDA top-of-stack is `top`.
    fn is_valid(
        &mut self,
        top: &StackEntry,
        token_idx: usize,
        token_text: &str,
        grammar: &Grammar,
    ) -> bool {
        // Only cache for terminal and character-class entries (truly context-independent).
        match top {
            StackEntry::Terminal(_) | StackEntry::IdentClass | StackEntry::DigitClass => {}
            StackEntry::NonTerminal(_) => {
                // NT validity depends on which production is chosen — not cacheable
                // at the single-symbol level.
                return self.compute_nt_validity(top, token_text, grammar);
            }
        }

        if let Some(valid_set) = self.cache.get(top) {
            return valid_set.contains(&token_idx);
        }

        // Compute and cache.
        let valid = self.compute_terminal_validity(top, token_text);
        let set = self.cache.entry(top.clone()).or_default();
        if valid {
            set.insert(token_idx);
        }
        valid
    }

    fn compute_terminal_validity(&self, top: &StackEntry, token_text: &str) -> bool {
        match top {
            StackEntry::Terminal(lit) => {
                lit.starts_with(token_text) || token_text.starts_with(lit.as_str())
            }
            StackEntry::IdentClass => token_text.chars().all(|c| c.is_alphanumeric() || c == '_'),
            StackEntry::DigitClass => token_text.chars().all(|c| c.is_ascii_digit()),
            _ => false,
        }
    }

    fn compute_nt_validity(
        &self,
        top: &StackEntry,
        token_text: &str,
        grammar: &Grammar,
    ) -> bool {
        // For NT entries, we fall through to the full PDA simulation.
        // Returning true here means "don't mask via cache — let full PDA decide".
        let _ = (
            std::hint::black_box(top as *const _ as usize),
            std::hint::black_box(token_text.len()),
            std::hint::black_box(grammar.productions.len()),
        );
        true
    }
}

// ── PdaSampler ───────────────────────────────────────────────────────────────

/// PDA-backed constrained sampler with context-independent token caching.
pub struct PdaSampler {
    grammar: Grammar,
    token_cache: std::sync::Mutex<TokenCache>,
}

impl PdaSampler {
    /// Build from an explicit EBNF string.
    pub fn from_ebnf(ebnf: &str) -> Result<Self> {
        let grammar = Grammar::from_ebnf(ebnf)?;
        debug!(
            productions = grammar.productions.len(),
            start = %grammar.start,
            "PdaSampler loaded grammar"
        );
        Ok(Self {
            grammar,
            token_cache: std::sync::Mutex::new(TokenCache::default()),
        })
    }

    /// Build from the canonical Vox EBNF.
    pub fn from_vox_grammar() -> Result<Self> {
        let ebnf = vox_grammar_export::ebnf::emit_ebnf();
        Self::from_ebnf(&ebnf)
    }
}

impl ConstrainedSampler for PdaSampler {
    fn mask_logits(
        &self,
        logits: &[f32],
        state: &SamplerState,
        token_strings: &[String],
    ) -> Result<(Vec<f32>, SamplerState)> {
        let pda_state = match state {
            SamplerState::Pda(s) => s,
            SamplerState::Empty => {
                let init = PdaState::new(&self.grammar.start);
                return self.mask_logits(logits, &SamplerState::Pda(init), token_strings);
            }
            _ => {
                return Err(ConstrainedGenError::Internal(
                    "PdaSampler received non-PDA state".into(),
                ));
            }
        };

        let mut masked = logits.to_vec();
        let mut any_valid = false;
        let mut best_states: Option<Vec<PdaState>> = None;
        let mut best_logit = f32::NEG_INFINITY;

        let mut cache = self.token_cache.lock().unwrap_or_else(|e| e.into_inner());

        for (i, token_str) in token_strings.iter().enumerate() {
            if i >= masked.len() {
                break;
            }

            // Fast-path: use cache for context-independent top-of-stack checks.
            let top = pda_state.stack.last();
            let cache_says_invalid = if let Some(top_entry) = top {
                !cache.is_valid(top_entry, i, token_str, &self.grammar)
            } else {
                // Empty stack — no more input accepted.
                true
            };

            if cache_says_invalid {
                masked[i] = f32::NEG_INFINITY;
                continue;
            }

            // Full PDA simulation for NT expansions.
            let next_states = pda_state.try_consume(&self.grammar, token_str);
            if next_states.is_empty() {
                masked[i] = f32::NEG_INFINITY;
            } else {
                any_valid = true;
                if masked[i] > best_logit {
                    best_logit = masked[i];
                    best_states = Some(next_states);
                }
            }
        }

        drop(cache);

        if !any_valid {
            return Err(ConstrainedGenError::Deadlock {
                position: pda_state.position,
                partial_output: pda_state.partial.clone(),
            });
        }

        // Take the first successor from the best-logit token's state set.
        let next = best_states
            .and_then(|mut ss| {
                if ss.is_empty() {
                    None
                } else {
                    Some(ss.swap_remove(0))
                }
            })
            .unwrap_or_else(|| pda_state.clone());

        Ok((masked, SamplerState::Pda(next)))
    }

    fn initial_state(&self) -> SamplerState {
        SamplerState::Pda(PdaState::new(&self.grammar.start))
    }

    fn name(&self) -> &'static str {
        "pda"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pda_sampler_builds_from_vox() {
        let sampler = PdaSampler::from_vox_grammar().expect("build PdaSampler");
        assert_eq!(sampler.name(), "pda");
    }

    #[test]
    fn pda_initial_state_is_pda_variant() {
        let sampler = PdaSampler::from_vox_grammar().unwrap();
        let state = sampler.initial_state();
        assert!(matches!(state, SamplerState::Pda(_)));
    }

    #[test]
    fn pda_state_new_starts_with_module() {
        let state = PdaState::new("module");
        assert_eq!(state.stack.len(), 1);
        assert_eq!(state.position, 0);
    }

    #[test]
    fn pda_masks_logits() {
        let sampler = PdaSampler::from_vox_grammar().unwrap();
        let state = sampler.initial_state();
        let logits = vec![1.0, 2.0, 0.5];
        let tokens = vec![
            "fn".to_string(),
            "???invalid!!!".to_string(),
            "let".to_string(),
        ];
        let result = sampler.mask_logits(&logits, &state, &tokens);
        // Should succeed (at least some tokens valid for module-level)
        assert!(result.is_ok());
        let (masked, _) = result.unwrap();
        assert_eq!(
            masked[1],
            f32::NEG_INFINITY,
            "invalid token should be masked"
        );
    }
}
