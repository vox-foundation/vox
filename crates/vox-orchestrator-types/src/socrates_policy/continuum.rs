use serde::{Deserialize, Serialize};

/// Operator-selected or policy-resolved research continuum depth mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResearchContinuumMode {
    /// Parametric / active-file only; zero external web calls.
    Off,
    /// Adaptive Socrates + CRAG escalation (`Direct -> LocalRag -> QuickWeb -> DeepMultiWave`).
    #[default]
    Auto,
    /// Pinned 1-wave fast retrieval cap; never auto-escalates to multi-wave deep research.
    Fast,
    /// Direct promotion to multi-wave deep research.
    Deep,
}

impl ResearchContinuumMode {
    /// Canonical snake_case wire label.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Fast => "fast",
            Self::Deep => "deep",
        }
    }
}

impl std::fmt::Display for ResearchContinuumMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for ResearchContinuumMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "auto" => Ok(Self::Auto),
            "fast" | "quick" => Ok(Self::Fast),
            "deep" => Ok(Self::Deep),
            _ => Err(format!(
                "Unknown research continuum mode: {s}. Expected off|auto|fast|deep"
            )),
        }
    }
}

/// Ordered execution tier along the adaptive research continuum.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchContinuumTier {
    /// Zero-retrieval parametric / active-file turn.
    Direct,
    /// Local workspace / Codex hybrid RAG.
    LocalRag,
    /// Single-wave federated web retrieval.
    QuickWeb,
    /// Multi-wave adversarial deep research.
    DeepMultiWave,
}

impl ResearchContinuumTier {
    /// Canonical snake_case wire label.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::LocalRag => "local_rag",
            Self::QuickWeb => "quick_web",
            Self::DeepMultiWave => "deep_multi_wave",
        }
    }

    /// Next monotonic step in `Direct -> LocalRag -> QuickWeb -> DeepMultiWave`.
    #[must_use]
    pub fn next_tier(self) -> Option<Self> {
        match self {
            Self::Direct => Some(Self::LocalRag),
            Self::LocalRag => Some(Self::QuickWeb),
            Self::QuickWeb => Some(Self::DeepMultiWave),
            Self::DeepMultiWave => None,
        }
    }
}

/// Structured diagnostic reason for escalating between continuum tiers.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationReason {
    /// Local retrieval confidence fell below `local_confidence_floor`.
    LowLocalConfidence,
    /// Evidence contradiction ratio met or exceeded `contradiction_ceiling`.
    HighContradiction,
    /// Corrective-RAG evidence quality fell below `crag_quality_floor`.
    LowCragQuality,
    /// Operator or intent classifier requested deep multi-wave synthesis.
    ExplicitDeepIntent,
    /// Prompt requires fresh external / time-sensitive evidence unavailable locally.
    RecencySignal,
    /// High surface confidence despite fewer than `coverage_paradox_min_sources` sources.
    CoverageParadox,
}

impl EscalationReason {
    /// Canonical snake_case wire label.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LowLocalConfidence => "low_local_confidence",
            Self::HighContradiction => "high_contradiction",
            Self::LowCragQuality => "low_crag_quality",
            Self::ExplicitDeepIntent => "explicit_deep_intent",
            Self::RecencySignal => "recency_signal",
            Self::CoverageParadox => "coverage_paradox",
        }
    }
}

/// Deterministic Socrates + CRAG thresholds governing continuum tier transitions.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CragEscalationPolicy {
    /// Minimum local retrieval confidence required to stay at local tiers (`0.68`).
    pub local_confidence_floor: f64,
    /// Contradiction ratio at or above which tier escalation triggers (`0.25`).
    pub contradiction_ceiling: f64,
    /// Minimum CRAG evidence quality score required to avoid deeper retrieval (`0.55`).
    pub crag_quality_floor: f64,
    /// Minimum distinct sources expected before trusting high confidence (`3`).
    pub coverage_paradox_min_sources: usize,
    /// Maximum multi-wave iterations permitted under `Auto` escalation (`3`).
    pub max_auto_waves: u8,
}

impl Default for CragEscalationPolicy {
    fn default() -> Self {
        Self {
            local_confidence_floor: 0.68,
            contradiction_ceiling: 0.25,
            crag_quality_floor: 0.55,
            coverage_paradox_min_sources: 3,
            max_auto_waves: 3,
        }
    }
}

impl CragEscalationPolicy {
    /// Resolve the starting continuum tier for a given [`ResearchContinuumMode`].
    #[must_use]
    pub fn initial_tier_for_mode(&self, mode: ResearchContinuumMode) -> ResearchContinuumTier {
        match mode {
            ResearchContinuumMode::Off => ResearchContinuumTier::Direct,
            ResearchContinuumMode::Auto => ResearchContinuumTier::LocalRag,
            ResearchContinuumMode::Fast => ResearchContinuumTier::QuickWeb,
            ResearchContinuumMode::Deep => ResearchContinuumTier::DeepMultiWave,
        }
    }

    /// Detect coverage paradox (high apparent confidence from too few independent sources).
    #[must_use]
    pub fn has_coverage_paradox(&self, source_count: usize, confidence: f64) -> bool {
        source_count < self.coverage_paradox_min_sources
            && confidence >= self.local_confidence_floor
    }

    /// Deterministically evaluate whether the turn should escalate from `current_tier`.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate_escalation(
        &self,
        mode: ResearchContinuumMode,
        current_tier: ResearchContinuumTier,
        local_confidence: f64,
        contradiction_ratio: f64,
        crag_quality: f64,
        recency_required: bool,
        explicit_deep_intent: bool,
    ) -> Option<(ResearchContinuumTier, EscalationReason)> {
        match mode {
            ResearchContinuumMode::Off | ResearchContinuumMode::Fast => None,
            ResearchContinuumMode::Deep => {
                if current_tier < ResearchContinuumTier::DeepMultiWave {
                    Some((
                        ResearchContinuumTier::DeepMultiWave,
                        EscalationReason::ExplicitDeepIntent,
                    ))
                } else {
                    None
                }
            }
            ResearchContinuumMode::Auto => {
                let next = current_tier.next_tier()?;
                if explicit_deep_intent {
                    return Some((next, EscalationReason::ExplicitDeepIntent));
                }
                if recency_required && current_tier < ResearchContinuumTier::QuickWeb {
                    return Some((next, EscalationReason::RecencySignal));
                }
                if contradiction_ratio >= self.contradiction_ceiling {
                    return Some((next, EscalationReason::HighContradiction));
                }
                if local_confidence < self.local_confidence_floor {
                    return Some((next, EscalationReason::LowLocalConfidence));
                }
                if crag_quality < self.crag_quality_floor {
                    return Some((next, EscalationReason::LowCragQuality));
                }
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn continuum_mode_serde_and_from_str_round_trip() {
        assert_eq!(
            ResearchContinuumMode::default(),
            ResearchContinuumMode::Auto
        );
        for (mode, wire) in [
            (ResearchContinuumMode::Off, "off"),
            (ResearchContinuumMode::Auto, "auto"),
            (ResearchContinuumMode::Fast, "fast"),
            (ResearchContinuumMode::Deep, "deep"),
        ] {
            assert_eq!(mode.as_str(), wire);
            let json = serde_json::to_string(&mode).expect("serialize mode");
            assert_eq!(json, format!("\"{wire}\""));
            let parsed: ResearchContinuumMode = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(parsed, mode);
            assert_eq!(
                wire.to_uppercase().parse::<ResearchContinuumMode>(),
                Ok(mode)
            );
        }
        assert_eq!(
            "quick".parse::<ResearchContinuumMode>(),
            Ok(ResearchContinuumMode::Fast)
        );
        assert!("invalid".parse::<ResearchContinuumMode>().is_err());
    }

    #[test]
    fn continuum_tier_ordering_and_serde() {
        assert!(ResearchContinuumTier::Direct < ResearchContinuumTier::LocalRag);
        assert!(ResearchContinuumTier::LocalRag < ResearchContinuumTier::QuickWeb);
        assert!(ResearchContinuumTier::QuickWeb < ResearchContinuumTier::DeepMultiWave);

        for (tier, wire) in [
            (ResearchContinuumTier::Direct, "direct"),
            (ResearchContinuumTier::LocalRag, "local_rag"),
            (ResearchContinuumTier::QuickWeb, "quick_web"),
            (ResearchContinuumTier::DeepMultiWave, "deep_multi_wave"),
        ] {
            assert_eq!(tier.as_str(), wire);
            let json = serde_json::to_string(&tier).expect("serialize tier");
            assert_eq!(json, format!("\"{wire}\""));
            let back: ResearchContinuumTier = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, tier);
        }
    }

    #[test]
    fn escalation_reason_serde_snake_case() {
        for (reason, wire) in [
            (
                EscalationReason::LowLocalConfidence,
                "\"low_local_confidence\"",
            ),
            (
                EscalationReason::HighContradiction,
                "\"high_contradiction\"",
            ),
            (EscalationReason::LowCragQuality, "\"low_crag_quality\""),
            (
                EscalationReason::ExplicitDeepIntent,
                "\"explicit_deep_intent\"",
            ),
            (EscalationReason::RecencySignal, "\"recency_signal\""),
            (EscalationReason::CoverageParadox, "\"coverage_paradox\""),
        ] {
            let json = serde_json::to_string(&reason).expect("serialize reason");
            assert_eq!(json, wire);
            let back: EscalationReason = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back, reason);
        }
    }

    #[test]
    fn policy_defaults_and_off_fast_caps() {
        let policy = CragEscalationPolicy::default();
        assert!((policy.local_confidence_floor - 0.68).abs() < f64::EPSILON);
        assert!((policy.contradiction_ceiling - 0.25).abs() < f64::EPSILON);
        assert!((policy.crag_quality_floor - 0.55).abs() < f64::EPSILON);
        assert_eq!(policy.coverage_paradox_min_sources, 3);
        assert_eq!(policy.max_auto_waves, 3);
        assert!(policy.has_coverage_paradox(2, 0.80));
        assert!(!policy.has_coverage_paradox(3, 0.80));

        for (mode, expected) in [
            (ResearchContinuumMode::Off, ResearchContinuumTier::Direct),
            (ResearchContinuumMode::Auto, ResearchContinuumTier::LocalRag),
            (ResearchContinuumMode::Fast, ResearchContinuumTier::QuickWeb),
            (
                ResearchContinuumMode::Deep,
                ResearchContinuumTier::DeepMultiWave,
            ),
        ] {
            assert_eq!(policy.initial_tier_for_mode(mode), expected);
        }

        for pinned in [ResearchContinuumMode::Off, ResearchContinuumMode::Fast] {
            for tier in [
                ResearchContinuumTier::Direct,
                ResearchContinuumTier::QuickWeb,
            ] {
                assert_eq!(
                    policy.evaluate_escalation(pinned, tier, 0.10, 0.90, 0.10, true, true),
                    None
                );
            }
        }
    }

    #[test]
    fn auto_progressive_escalation_and_deep_promotion() {
        use EscalationReason as R;
        use ResearchContinuumMode as M;
        use ResearchContinuumTier as T;

        let policy = CragEscalationPolicy::default();
        let eval = |m, t, c, cr, q, rec, d| policy.evaluate_escalation(m, t, c, cr, q, rec, d);
        assert_eq!(
            eval(M::Auto, T::Direct, 0.40, 0.05, 0.80, false, false),
            Some((T::LocalRag, R::LowLocalConfidence))
        );
        assert_eq!(
            eval(M::Auto, T::LocalRag, 0.85, 0.05, 0.80, true, false),
            Some((T::QuickWeb, R::RecencySignal))
        );
        assert_eq!(
            eval(M::Auto, T::QuickWeb, 0.85, 0.10, 0.75, true, false),
            None
        );
        assert_eq!(
            eval(M::Auto, T::QuickWeb, 0.85, 0.35, 0.75, false, false),
            Some((T::DeepMultiWave, R::HighContradiction))
        );
        assert_eq!(
            eval(M::Auto, T::QuickWeb, 0.85, 0.05, 0.40, false, false),
            Some((T::DeepMultiWave, R::LowCragQuality))
        );
        assert_eq!(
            eval(M::Auto, T::LocalRag, 0.90, 0.0, 0.90, false, true),
            Some((T::QuickWeb, R::ExplicitDeepIntent))
        );
        assert_eq!(
            eval(M::Deep, T::Direct, 0.95, 0.0, 0.95, false, false),
            Some((T::DeepMultiWave, R::ExplicitDeepIntent))
        );
        assert_eq!(
            eval(M::Deep, T::DeepMultiWave, 0.10, 0.90, 0.10, true, true),
            None
        );
    }
}
