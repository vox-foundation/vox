//! Shared **Socrates** confidence policy for orchestrator, MCP, and TOESTUB review.
//!
//! Single source of truth for numeric thresholds so prompts, filters, and gates stay aligned.
//! See `docs/src/architecture/socrates-protocol-ssot.md`.

mod complexity;
mod confidence_override;
mod confidence_policy;
mod entropy;
mod policy_types;

pub use complexity::SocratesComplexityJudge;
pub use confidence_override::ConfidencePolicyOverride;
pub use entropy::{expected_information_gain_bits, shannon_entropy_bits};
pub use policy_types::{
    ClarificationStopReason, ComplexityBand, ConfidencePolicy, QuestionCandidate, QuestionKind, QuestioningPolicy,
    QuestionSelection, RiskBand, RiskDecision, SocratesResearchDecision,
    CLARIFICATION_INTERRUPT_COST_MS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_constants_match_struct_fields() {
        let p = ConfidencePolicy::workspace_default();
        assert_eq!(
            p.min_review_finding_confidence,
            ConfidencePolicy::DEFAULT_MIN_REVIEW_FINDING_CONFIDENCE
        );
        assert_eq!(
            p.min_persist_confidence,
            ConfidencePolicy::DEFAULT_MIN_PERSIST_CONFIDENCE
        );
        assert_eq!(
            p.min_training_pair_confidence,
            ConfidencePolicy::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE
        );
    }

    #[test]
    fn default_policy_symmetric_review_thresholds() {
        let p = ConfidencePolicy::default();
        assert_eq!(
            p.min_review_finding_confidence,
            p.min_prompt_report_confidence
        );
    }

    #[test]
    fn high_confidence_answers() {
        let p = ConfidencePolicy::default();
        assert_eq!(
            p.evaluate_risk_decision(0.9, 0.0, 1.0),
            RiskDecision::Answer
        );
    }

    #[test]
    fn contradiction_forces_abstain() {
        let p = ConfidencePolicy::default();
        assert_eq!(
            p.evaluate_risk_decision(0.99, 0.99, 1.0),
            RiskDecision::Abstain
        );
    }

    #[test]
    fn medium_confidence_requests_clarification() {
        let p = ConfidencePolicy::default();
        let c = (p.abstain_threshold + p.ask_for_help_threshold) / 2.0;
        assert_eq!(p.evaluate_risk_decision(c, 0.0, 1.0), RiskDecision::Ask);
    }

    #[test]
    fn confidence_monotonicity_for_fixed_contradiction() {
        let p = ConfidencePolicy::default();
        let low = p.evaluate_risk_decision(0.10, 0.0, 1.0);
        let mid = p.evaluate_risk_decision(0.50, 0.0, 1.0);
        let hi = p.evaluate_risk_decision(0.95, 0.0, 1.0);
        assert_eq!(low, RiskDecision::Abstain);
        assert!(matches!(mid, RiskDecision::Ask | RiskDecision::Answer));
        assert_eq!(hi, RiskDecision::Answer);
    }

    #[test]
    fn entropy_uniform_over_four_states_is_two_bits() {
        let h = shannon_entropy_bits(&[0.25, 0.25, 0.25, 0.25]);
        assert!((h - 2.0).abs() < 1e-9, "h={h}");
    }

    #[test]
    fn expected_information_gain_positive_when_posteriors_reduce_uncertainty() {
        let prior = [0.5, 0.5];
        let post = vec![vec![0.9, 0.1], vec![0.1, 0.9]];
        let outcome_probs = [0.5, 0.5];
        let gain = expected_information_gain_bits(&prior, &post, &outcome_probs);
        assert!(gain > 0.0, "gain={gain}");
    }

    #[test]
    fn select_clarification_uses_best_gain_per_cost() {
        let p = ConfidencePolicy::default();
        let qp = QuestioningPolicy::default();
        let candidates = vec![
            QuestionCandidate {
                prompt: "Do you want only docs or code changes?".into(),
                question_kind: QuestionKind::MultipleChoice,
                expected_information_gain_bits: 0.20,
                expected_user_cost: 0.20,
            },
            QuestionCandidate {
                prompt: "What is your broader goal?".into(),
                question_kind: QuestionKind::OpenEnded,
                expected_information_gain_bits: 0.28,
                expected_user_cost: 0.55,
            },
        ];
        let sel = p.select_clarification_question(0.45, 0.0, 1.0, 0, &candidates, qp, 0, 0);
        assert!(sel.question_needed);
        assert_eq!(sel.question_kind, Some(QuestionKind::MultipleChoice));
    }

    #[test]
    fn select_clarification_stops_when_max_turns_reached() {
        let p = ConfidencePolicy::default();
        let qp = QuestioningPolicy {
            max_clarification_turns: 1,
            ..QuestioningPolicy::default()
        };
        let candidates = vec![QuestionCandidate {
            prompt: "confirm scope".into(),
            question_kind: QuestionKind::Entry,
            expected_information_gain_bits: 0.5,
            expected_user_cost: 0.1,
        }];
        let sel = p.select_clarification_question(0.40, 0.0, 1.0, 1, &candidates, qp, 0, 0);
        assert!(!sel.question_needed);
        assert_eq!(
            sel.stop_reason,
            Some(ClarificationStopReason::MaxClarificationTurns)
        );
    }

    #[test]
    fn select_clarification_stops_when_attention_budget_spent() {
        let p = ConfidencePolicy::default();
        let qp = QuestioningPolicy::default();
        let candidates = vec![QuestionCandidate {
            prompt: "confirm scope".into(),
            question_kind: QuestionKind::Entry,
            expected_information_gain_bits: 0.5,
            expected_user_cost: 0.1,
        }];
        let sel =
            p.select_clarification_question(0.40, 0.0, 1.0, 0, &candidates, qp, 10_000, 10_000);
        assert!(!sel.question_needed);
        assert_eq!(
            sel.stop_reason,
            Some(ClarificationStopReason::AttentionBudgetExceeded)
        );
    }

    #[test]
    fn coverage_paradox_downgrades_to_ask() {
        let p = ConfidencePolicy::default();
        // High confidence (0.9), but extreme contradiction (0.9).
        // If coverage is low (0.1), we should NOT abstain, we should Ask/Medium.
        let decision = p.evaluate_risk_decision(0.9, 0.9, 0.1);
        assert_eq!(decision, RiskDecision::Ask);

        // However, if coverage is high (0.8), we MUST abstain.
        let decision_high_cov = p.evaluate_risk_decision(0.9, 0.9, 0.8);
        assert_eq!(decision_high_cov, RiskDecision::Abstain);
    }

    #[test]
    fn research_dispatch_logic() {
        let p = ConfidencePolicy::default();

        // Scenario A: Everything is fine.
        let res = p.evaluate_research_need(0.9, 0.0, 0.9, "test query");
        assert!(!res.should_research);

        // Scenario B: Low coverage contradiction.
        let res = p.evaluate_research_need(0.9, 0.9, 0.1, "test query");
        assert!(res.should_research);
        assert!(res.trigger.contains("Coverage Paradox"));

        // Scenario C: Pure Abstain band.
        let res = p.evaluate_research_need(0.2, 0.0, 0.1, "test query");
        assert!(res.should_research);
        assert!(res.trigger.contains("Insufficient Evidence"));
    }
}
