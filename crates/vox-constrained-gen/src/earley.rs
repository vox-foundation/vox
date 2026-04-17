//! Earley parser backend for grammar-constrained token sampling.
//!
//! This backend maintains an incremental Earley chart that tracks all viable parse
//! prefixes. On each generation step it tests every candidate token against the
//! chart and masks out tokens that would leave no viable continuation.
//!
//! **Research rationale (Grammar Constraints §2.1):** Earley parsing handles the
//! full class of context-free grammars — including left-recursion, which GBNF/FSA
//! backends cannot represent without depth caps. Overhead is O(n) for unambiguous
//! grammars (which Vox is) and O(n³) worst-case for ambiguous grammars.

use tracing::debug;
use vox_grammar_export::grammar_ir::{Grammar, Symbol};

use crate::{ConstrainedGenError, ConstrainedSampler, Result, SamplerState};

// ── Earley items & chart ─────────────────────────────────────────────────────

/// A single Earley item: a dotted rule with an origin position.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct EarleyItem {
    /// Index into `Grammar::productions`.
    prod_idx: usize,
    /// Position of the dot within `productions[prod_idx].symbols`.
    dot: usize,
    /// Chart column where this item was predicted/started.
    origin: usize,
}

impl EarleyItem {
    fn is_complete(&self, grammar: &Grammar) -> bool {
        self.dot >= grammar.productions[self.prod_idx].symbols.len()
    }

    fn next_symbol<'g>(&self, grammar: &'g Grammar) -> Option<&'g Symbol> {
        grammar.productions[self.prod_idx].symbols.get(self.dot)
    }
}

/// Incremental Earley chart state persisted between generation steps.
#[derive(Debug, Clone)]
pub struct EarleyState {
    /// The current chart column (set of Earley items).
    items: Vec<EarleyItem>,
    /// Number of tokens consumed so far.
    position: usize,
    /// Accumulated generated text (for deadlock diagnostics).
    partial: String,
}

impl EarleyState {
    fn new(grammar: &Grammar) -> Self {
        let mut items = Vec::new();
        // Predict from start symbol.
        if let Some(indices) = grammar.index.get(&grammar.start) {
            for &idx in indices {
                items.push(EarleyItem {
                    prod_idx: idx,
                    dot: 0,
                    origin: 0,
                });
            }
        }
        let mut state = EarleyState {
            items,
            position: 0,
            partial: String::new(),
        };
        state.closure(grammar);
        state
    }

    /// Earley prediction + completion closure on the current item set.
    fn closure(&mut self, grammar: &Grammar) {
        let mut i = 0;
        while i < self.items.len() {
            let item = self.items[i].clone();

            if item.is_complete(grammar) {
                // Completion: advance items from `origin` that were waiting for this NT.
                let completed_name = &grammar.productions[item.prod_idx].name;
                let advances: Vec<EarleyItem> = self
                    .items
                    .iter()
                    .filter(|it| it.origin <= item.origin || it.dot > 0)
                    .filter(|it| {
                        if let Some(Symbol::NonTerminal(nt)) = it.next_symbol(grammar) {
                            nt == completed_name
                        } else {
                            false
                        }
                    })
                    .map(|it| EarleyItem {
                        prod_idx: it.prod_idx,
                        dot: it.dot + 1,
                        origin: it.origin,
                    })
                    .collect();
                for a in advances {
                    if !self.items.contains(&a) {
                        self.items.push(a);
                    }
                }
            } else if let Some(Symbol::NonTerminal(nt)) = item.next_symbol(grammar) {
                // Prediction: add items for all productions of this NT.
                if let Some(indices) = grammar.index.get(nt) {
                    for &idx in indices {
                        let new_item = EarleyItem {
                            prod_idx: idx,
                            dot: 0,
                            origin: self.position,
                        };
                        if !self.items.contains(&new_item) {
                            self.items.push(new_item);
                        }
                    }
                }
            }
            i += 1;
        }
    }

    /// Attempt to scan `token_text` against the current chart.
    /// Returns `Some(new_state)` if the token is a valid continuation.
    fn try_scan(&self, grammar: &Grammar, token_text: &str) -> Option<EarleyState> {
        if token_text.is_empty() {
            return Some(self.clone());
        }

        let mut next_items = Vec::new();
        let next_pos = self.position + 1;

        for item in &self.items {
            if item.is_complete(grammar) {
                continue;
            }
            let Some(sym) = item.next_symbol(grammar) else {
                continue;
            };
            let matches = match sym {
                Symbol::Terminal(lit) => {
                    token_text.contains(lit.as_str()) || lit.contains(token_text)
                }
                Symbol::IdentClass => token_text.chars().all(|c| c.is_alphanumeric() || c == '_'),
                Symbol::DigitClass => token_text.chars().all(|c| c.is_ascii_digit()),
                Symbol::NonTerminal(_) => continue, // handled by prediction
            };
            if matches {
                let advanced = EarleyItem {
                    prod_idx: item.prod_idx,
                    dot: item.dot + 1,
                    origin: item.origin,
                };
                if !next_items.contains(&advanced) {
                    next_items.push(advanced);
                }
            }
        }

        if next_items.is_empty() {
            return None;
        }

        let mut new_state = EarleyState {
            items: next_items,
            position: next_pos,
            partial: format!("{}{}", self.partial, token_text),
        };
        new_state.closure(grammar);
        Some(new_state)
    }
}

// ── EarleySampler ────────────────────────────────────────────────────────────

/// Earley parser-backed constrained sampler.
pub struct EarleySampler {
    grammar: Grammar,
}

impl EarleySampler {
    /// Build from an explicit EBNF string.
    pub fn from_ebnf(ebnf: &str) -> Result<Self> {
        let grammar = Grammar::from_ebnf(ebnf).map_err(|e| ConstrainedGenError::GrammarError { reason: e })?;
        debug!(
            productions = grammar.productions.len(),
            start = %grammar.start,
            "EarleySampler loaded grammar"
        );
        Ok(Self { grammar })
    }

    /// Build from the canonical Vox EBNF via `vox-grammar-export`.
    pub fn from_vox_grammar() -> Result<Self> {
        let ebnf = vox_grammar_export::ebnf::emit_ebnf();
        Self::from_ebnf(&ebnf)
    }
}

impl ConstrainedSampler for EarleySampler {
    fn mask_logits(
        &self,
        logits: &[f32],
        state: &SamplerState,
        token_strings: &[String],
    ) -> Result<(Vec<f32>, SamplerState)> {
        let earley_state = match state {
            SamplerState::Earley(s) => s,
            SamplerState::Empty => {
                let init = EarleyState::new(&self.grammar);
                return self.mask_logits(logits, &SamplerState::Earley(init), token_strings);
            }
            _ => {
                return Err(ConstrainedGenError::Internal(
                    "EarleySampler received non-Earley state".into(),
                ));
            }
        };

        let mut masked = logits.to_vec();
        let mut any_valid = false;
        // We'll keep the best new state to return (from the highest-logit valid token).
        let mut best_state: Option<EarleyState> = None;
        let mut best_logit = f32::NEG_INFINITY;

        for (i, token_str) in token_strings.iter().enumerate() {
            if i >= masked.len() {
                break;
            }
            if let Some(new_state) = earley_state.try_scan(&self.grammar, token_str) {
                any_valid = true;
                if masked[i] > best_logit {
                    best_logit = masked[i];
                    best_state = Some(new_state);
                }
            } else {
                masked[i] = f32::NEG_INFINITY;
            }
        }

        if !any_valid {
            return Err(ConstrainedGenError::Deadlock {
                position: earley_state.position,
                partial_output: earley_state.partial.clone(),
            });
        }

        let next = best_state.unwrap_or_else(|| earley_state.clone());
        Ok((masked, SamplerState::Earley(next)))
    }

    fn initial_state(&self) -> SamplerState {
        SamplerState::Earley(EarleyState::new(&self.grammar))
    }

    fn name(&self) -> &'static str {
        "earley"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_from_vox_ebnf_parses() {
        let ebnf = vox_grammar_export::ebnf::emit_ebnf();
        let grammar = Grammar::from_ebnf(&ebnf).expect("parse Vox EBNF");
        assert!(
            grammar.productions.len() > 20,
            "should have many productions"
        );
        assert_eq!(grammar.start, "module");
    }

    #[test]
    fn earley_sampler_builds_from_vox() {
        let sampler = EarleySampler::from_vox_grammar().expect("build EarleySampler");
        assert_eq!(sampler.name(), "earley");
    }

    #[test]
    fn earley_initial_state_is_earley_variant() {
        let sampler = EarleySampler::from_vox_grammar().unwrap();
        let state = sampler.initial_state();
        assert!(matches!(state, SamplerState::Earley(_)));
    }

    #[test]
    fn earley_masks_logits() {
        let sampler = EarleySampler::from_vox_grammar().unwrap();
        let state = sampler.initial_state();
        let logits = vec![1.0, 2.0, 0.5, 3.0];
        let tokens = vec![
            "fn".to_string(),
            "???".to_string(),
            "let".to_string(),
            "import".to_string(),
        ];
        let (masked, _new_state) = sampler.mask_logits(&logits, &state, &tokens).unwrap();
        // "fn", "let", "import" should be valid at module level; "???" should be masked
        assert!(masked[0] > f32::NEG_INFINITY, "fn should be valid");
        assert_eq!(masked[1], f32::NEG_INFINITY, "??? should be masked");
    }
}
