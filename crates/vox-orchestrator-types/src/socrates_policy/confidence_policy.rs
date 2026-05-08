use super::confidence_override::ConfidencePolicyOverride;
use super::policy_types::{
    ClarificationStopReason, ConfidencePolicy, QuestionCandidate, QuestionSelection,
    QuestioningPolicy, RiskBand, RiskDecision, SocratesResearchDecision,
};

impl ConfidencePolicy {
    /// Default TOESTUB / review finding floor (0–100); must match [`Self::default`].
    pub const DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE: u8 = 80;
    /// Default prompt report floor (0–100); must match [`Self::default`].
    pub const DEFAULT_MIN_PROMPT_REPORT_CONFIDENCE: u8 = 80;
    /// Default abstain cutoff on normalized confidence; must match [`Self::default`].
    pub const DEFAULT_ABSTAIN_THRESHOLD: f64 = 0.35;
    /// Default “ask” band lower bound; must match [`Self::default`].
    pub const DEFAULT_ASK_FOR_HELP_THRESHOLD: f64 = 0.55;
    /// Default contradiction ratio that forces abstain; must match [`Self::default`].
    pub const DEFAULT_MAX_CONTRADICTION_RATIO_FOR_ANSWER: f64 = 0.40;
    /// Default minimum normalized confidence to persist research-like artifacts; must match [`Self::default`].
    pub const DEFAULT_MIN_PERSIST_CONFIDENCE: f64 = 0.60;
    /// Default minimum normalized confidence to emit training pairs; must match [`Self::default`].
    pub const DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE: f64 = 0.75;

    /// Global default used across the workspace unless overridden.
    #[must_use]
    pub fn workspace_default() -> Self {
        Self::default()
    }

    /// Merge [`ConfidencePolicyOverride`] fields onto this policy (unset override fields keep base values).
    #[must_use]
    pub fn with_overrides(&self, o: &ConfidencePolicyOverride) -> Self {
        let mut out = *self;
        if let Some(v) = o.min_review_finding_confidence {
            out.min_review_finding_confidence = v;
        }
        if let Some(v) = o.min_prompt_report_confidence {
            out.min_prompt_report_confidence = v;
        }
        if let Some(v) = o.abstain_threshold {
            out.abstain_threshold = v;
        }
        if let Some(v) = o.ask_for_help_threshold {
            out.ask_for_help_threshold = v;
        }
        if let Some(v) = o.max_contradiction_ratio_for_answer {
            out.max_contradiction_ratio_for_answer = v;
        }
        if let Some(v) = o.min_persist_confidence {
            out.min_persist_confidence = v;
        }
        if let Some(v) = o.min_training_pair_confidence {
            out.min_training_pair_confidence = v;
        }
        out
    }

    /// Classify a normalized confidence in `[0,1]`, contradiction ratio in `[0,1]`, and citation coverage in `[0,1]`.
    /// 1. If confidence < abstain_threshold → RiskBand::Low.
    /// 2. If contradiction_ratio > max_contradiction_ratio_for_answer:
    ///    - If citation_coverage >= 0.3 → RiskBand::Low (Conflicting evidence).
    ///    - If citation_coverage < 0.3 → RiskBand::Medium (The "Coverage Paradox" — weak evidence causes false contradiction signals).
    /// 3. If confidence < ask_for_help_threshold → RiskBand::Medium.
    /// 4. Else → RiskBand::High.
    #[must_use]
    pub fn classify_risk(
        &self,
        confidence: f64,
        contradiction_ratio: f64,
        citation_coverage: f64,
    ) -> RiskBand {
        let c = confidence.clamp(0.0, 1.0);
        let cr = contradiction_ratio.clamp(0.0, 1.0);
        let cov = citation_coverage.clamp(0.0, 1.0);

        if c < self.abstain_threshold {
            return RiskBand::Low;
        }

        if cr > self.max_contradiction_ratio_for_answer {
            if cov >= 0.30 {
                // We have enough evidence to be sure it's a contradiction.
                return RiskBand::Low;
            } else {
                // Coverage Paradox: with <30% coverage, a contradiction signal is likely structural noise.
                // Downgrade to Medium so we ask or research instead of refusing.
                return RiskBand::Medium;
            }
        }

        if c < self.ask_for_help_threshold {
            RiskBand::Medium
        } else {
            RiskBand::High
        }
    }

    /// Map calibrated signal to answer / ask / abstain.
    #[must_use]
    pub fn evaluate_risk_decision(
        &self,
        confidence: f64,
        contradiction_ratio: f64,
        citation_coverage: f64,
    ) -> RiskDecision {
        let band = self.classify_risk(confidence, contradiction_ratio, citation_coverage);
        match band {
            RiskBand::High => RiskDecision::Answer,
            RiskBand::Medium => RiskDecision::Ask,
            RiskBand::Low => RiskDecision::Abstain,
        }
    }

    /// Pick the highest utility clarification question under risk/time/cost constraints.
    #[must_use]
    pub fn select_clarification_question(
        &self,
        confidence: f64,
        contradiction_ratio: f64,
        citation_coverage: f64,
        clarification_turn_index: u32,
        candidates: &[QuestionCandidate],
        questioning: QuestioningPolicy,
        spent_clarification_attention_ms: u64,
        max_clarification_attention_ms: u64,
    ) -> QuestionSelection {
        let decision =
            self.evaluate_risk_decision(confidence, contradiction_ratio, citation_coverage);
        if confidence >= questioning.target_confidence || decision == RiskDecision::Answer {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::ConfidenceSufficient),
            };
        }
        if decision == RiskDecision::Abstain {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::RiskGateBlocked),
            };
        }
        if max_clarification_attention_ms > 0
            && spent_clarification_attention_ms >= max_clarification_attention_ms
        {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::AttentionBudgetExceeded),
            };
        }
        if clarification_turn_index >= questioning.max_clarification_turns {
            return QuestionSelection {
                question_needed: false,
                question_kind: None,
                prompt: None,
                expected_information_gain_bits: 0.0,
                expected_user_cost: 0.0,
                utility_bits_per_cost: 0.0,
                stop_reason: Some(ClarificationStopReason::MaxClarificationTurns),
            };
        }

        let mut best_idx: Option<usize> = None;
        let mut best_utility = f64::MIN;
        for (idx, c) in candidates.iter().enumerate() {
            if c.expected_user_cost > questioning.max_expected_user_cost {
                continue;
            }
            if c.expected_information_gain_bits < questioning.min_information_gain_bits {
                continue;
            }
            let utility = c.expected_information_gain_bits / c.expected_user_cost.max(1e-6);
            if utility > best_utility {
                best_utility = utility;
                best_idx = Some(idx);
            }
        }

        if let Some(idx) = best_idx {
            let c = &candidates[idx];
            return QuestionSelection {
                question_needed: true,
                question_kind: Some(c.question_kind),
                prompt: Some(c.prompt.clone()),
                expected_information_gain_bits: c.expected_information_gain_bits,
                expected_user_cost: c.expected_user_cost,
                utility_bits_per_cost: best_utility,
                stop_reason: None,
            };
        }

        let cost_blocked = candidates
            .iter()
            .any(|c| c.expected_user_cost > questioning.max_expected_user_cost);
        QuestionSelection {
            question_needed: false,
            question_kind: None,
            prompt: None,
            expected_information_gain_bits: 0.0,
            expected_user_cost: 0.0,
            utility_bits_per_cost: 0.0,
            stop_reason: Some(if cost_blocked {
                ClarificationStopReason::UserCostTooHigh
            } else {
                ClarificationStopReason::MarginalGainTooLow
            }),
        }
    }

    /// Evaluates whether an external web search is required based on RAG signals.
    #[must_use]
    pub fn evaluate_research_need(
        &self,
        confidence: f64,
        contradiction_ratio: f64,
        citation_coverage: f64,
        query: &str,
    ) -> SocratesResearchDecision {
        let band = self.classify_risk(confidence, contradiction_ratio, citation_coverage);

        if band == RiskBand::Low {
            if citation_coverage < 0.35
                && contradiction_ratio > self.max_contradiction_ratio_for_answer
            {
                return SocratesResearchDecision {
                    should_research: true,
                    trigger: "Coverage Paradox (Conflicting signals with low coverage)".into(),
                    suggested_query: Some(format!(
                        "{} pros and cons comparison contradiction",
                        query
                    )),
                };
            }
            return SocratesResearchDecision {
                should_research: true,
                trigger: "Insufficient Evidence (Abstain Band)".into(),
                suggested_query: Some(format!("{} canonical technical details evidence", query)),
            };
        }

        if band == RiskBand::Medium {
            return SocratesResearchDecision {
                should_research: true,
                trigger: "Weak Evidence (Clarification Band)".into(),
                suggested_query: Some(format!("{} authoritative documentation overview", query)),
            };
        }

        SocratesResearchDecision {
            should_research: false,
            trigger: "Sufficient Confidence".into(),
            suggested_query: None,
        }
    }
}

impl Default for ConfidencePolicy {
    fn default() -> Self {
        Self {
            min_review_finding_confidence: Self::DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE,
            min_prompt_report_confidence: Self::DEFAULT_MIN_PROMPT_REPORT_CONFIDENCE,
            abstain_threshold: Self::DEFAULT_ABSTAIN_THRESHOLD,
            ask_for_help_threshold: Self::DEFAULT_ASK_FOR_HELP_THRESHOLD,
            max_contradiction_ratio_for_answer: Self::DEFAULT_MAX_CONTRADICTION_RATIO_FOR_ANSWER,
            min_persist_confidence: Self::DEFAULT_MIN_PERSIST_CONFIDENCE,
            min_training_pair_confidence: Self::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE,
        }
    }
}
