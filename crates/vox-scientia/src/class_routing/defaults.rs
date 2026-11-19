//! Per-class defaults file shape + loader + built-in defaults.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

/// Closed enum of candidate classes the micro-track config knows about.
/// Mirrors the values in `FindingCandidateClass` in
/// `vox-research-events` and the schema in
/// `contracts/scientia/finding-candidate.v1.schema.json`, plus the
/// Atlas-specific classes from Finalization Phase 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingClass {
    AlgorithmicImprovement,
    ReproducibilityInfra,
    PolicyGovernance,
    TelemetryTrust,
    EmpiricalScience,
    Other,
    /// Atlas track — provider behavior epidemiology.
    ModelCapabilityAtlas,
    /// Atlas track — provider reliability epidemiology.
    ProviderReliabilityAtlas,
}

impl FindingClass {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AlgorithmicImprovement => "algorithmic_improvement",
            Self::ReproducibilityInfra => "reproducibility_infra",
            Self::PolicyGovernance => "policy_governance",
            Self::TelemetryTrust => "telemetry_trust",
            Self::EmpiricalScience => "empirical_science",
            Self::Other => "other",
            Self::ModelCapabilityAtlas => "model_capability_atlas",
            Self::ProviderReliabilityAtlas => "provider_reliability_atlas",
        }
    }

    // Inherent parser returning Option (not the FromStr trait's Result); callers use
    // `FindingClass::from_str(...)` directly. Accepts canonical snake_case variants
    // as well as content-type / scholarly aliases (`paper`, `scientia`,
    // `benchmark_report`, `negative_result`, etc.).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        let normalized = s.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "algorithmic_improvement"
            | "algorithmicimprovement"
            | "algorithmic"
            | "benchmark_report"
            | "benchmark"
            | "perf"
            | "performance" => Some(Self::AlgorithmicImprovement),
            "reproducibility_infra"
            | "reproducibilityinfra"
            | "reproducibility"
            | "replication" => Some(Self::ReproducibilityInfra),
            "policy_governance" | "policygovernance" | "policy" | "governance" => {
                Some(Self::PolicyGovernance)
            }
            "telemetry_trust" | "telemetrytrust" | "telemetry" | "trust" => {
                Some(Self::TelemetryTrust)
            }
            "empirical_science" | "empiricalscience" | "empirical" | "paper" | "scientia"
            | "preprint" | "arxiv" | "research" | "negative_result" | "negative"
            | "null_result" => Some(Self::EmpiricalScience),
            "other" => Some(Self::Other),
            "model_capability_atlas"
            | "modelcapabilityatlas"
            | "model_capability"
            | "capability_atlas" => Some(Self::ModelCapabilityAtlas),
            "provider_reliability_atlas"
            | "providerreliabilityatlas"
            | "provider_reliability"
            | "reliability_atlas" => Some(Self::ProviderReliabilityAtlas),
            _ => None,
        }
    }

    /// Canonical list of all finding classes in display order.
    pub const fn all() -> &'static [Self] {
        &[
            Self::EmpiricalScience,
            Self::AlgorithmicImprovement,
            Self::ReproducibilityInfra,
            Self::PolicyGovernance,
            Self::TelemetryTrust,
            Self::ModelCapabilityAtlas,
            Self::ProviderReliabilityAtlas,
            Self::Other,
        ]
    }

    /// Human-readable label for GUI and checklist display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::AlgorithmicImprovement => "Algorithmic Improvement",
            Self::ReproducibilityInfra => "Reproducibility Infra",
            Self::PolicyGovernance => "Policy & Governance",
            Self::TelemetryTrust => "Telemetry & Trust",
            Self::EmpiricalScience => "Empirical Science",
            Self::Other => "Other Scholarly Artifact",
            Self::ModelCapabilityAtlas => "Model Capability Atlas",
            Self::ProviderReliabilityAtlas => "Provider Reliability Atlas",
        }
    }

    /// Atlas-class predicate; drives `atlas_gate_applies_to`.
    pub fn is_atlas(&self) -> bool {
        matches!(
            self,
            Self::ModelCapabilityAtlas | Self::ProviderReliabilityAtlas
        )
    }
}

/// Multi-dimensional scholarly publication tier determining IMRaD depth,
/// citation floor, and `arXiv.org` vs venue packaging expectations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationTier {
    Nanopublication,
    TechnicalReport,
    WorkshopPaper,
    ConferencePaper,
    ArxivPreprint,
    JournalArticle,
}

impl PublicationTier {
    pub const fn all() -> &'static [Self] {
        &[
            Self::Nanopublication,
            Self::TechnicalReport,
            Self::WorkshopPaper,
            Self::ConferencePaper,
            Self::ArxivPreprint,
            Self::JournalArticle,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nanopublication => "nanopublication",
            Self::TechnicalReport => "technical_report",
            Self::WorkshopPaper => "workshop_paper",
            Self::ConferencePaper => "conference_paper",
            Self::ArxivPreprint => "arxiv_preprint",
            Self::JournalArticle => "journal_article",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Nanopublication => "Nanopublication / Claim Brief",
            Self::TechnicalReport => "Technical Report",
            Self::WorkshopPaper => "Workshop Paper",
            Self::ConferencePaper => "Conference Paper",
            Self::ArxivPreprint => "arXiv.org Preprint",
            Self::JournalArticle => "Archival Journal Article",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        let normalized = s.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "nanopublication" | "nanopub" | "brief" => Some(Self::Nanopublication),
            "technical_report" | "tech_report" | "report" => Some(Self::TechnicalReport),
            "workshop_paper" | "workshop" => Some(Self::WorkshopPaper),
            "conference_paper" | "conference" => Some(Self::ConferencePaper),
            "arxiv_preprint" | "arxiv" | "preprint" => Some(Self::ArxivPreprint),
            "journal_article" | "journal" => Some(Self::JournalArticle),
            _ => None,
        }
    }

    /// Whether full IMRaD narrative sections (`Introduction`, `Related Work`,
    /// `Methods`, `Results`, `Discussion`, `Limitations`, `Conclusion`) are required.
    pub fn requires_full_imrad(&self) -> bool {
        !matches!(self, Self::Nanopublication)
    }

    /// Minimum BibTeX citation count expected before archival readiness.
    pub fn minimum_citations(&self) -> usize {
        match self {
            Self::Nanopublication => 1,
            Self::TechnicalReport => 1,
            Self::WorkshopPaper => 2,
            Self::ConferencePaper | Self::ArxivPreprint => 3,
            Self::JournalArticle => 5,
        }
    }

    /// Recommended tier for a given [`FindingClass`].
    pub fn recommended_for_class(class: FindingClass) -> Self {
        match class {
            FindingClass::EmpiricalScience => Self::ArxivPreprint,
            FindingClass::AlgorithmicImprovement => Self::ConferencePaper,
            FindingClass::ReproducibilityInfra => Self::TechnicalReport,
            FindingClass::PolicyGovernance => Self::WorkshopPaper,
            FindingClass::TelemetryTrust => Self::TechnicalReport,
            FindingClass::ModelCapabilityAtlas | FindingClass::ProviderReliabilityAtlas => {
                Self::ArxivPreprint
            }
            FindingClass::Other => Self::Nanopublication,
        }
    }
}

/// Canonical `arXiv.org` subject category metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ArxivSubjectEntry {
    pub code: &'static str,
    pub archive: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

const ARXIV_SUBJECT_CATALOG: &[ArxivSubjectEntry] = &[
    ArxivSubjectEntry {
        code: "cs.AI",
        archive: "Computer Science",
        name: "Artificial Intelligence",
        description: "Reasoning, planning, agent architectures, autonomous research systems.",
    },
    ArxivSubjectEntry {
        code: "cs.SE",
        archive: "Computer Science",
        name: "Software Engineering",
        description: "Empirical SE, static analysis, tool verification, developer workflows.",
    },
    ArxivSubjectEntry {
        code: "cs.LG",
        archive: "Computer Science",
        name: "Machine Learning",
        description: "QLoRA/fine-tuning, model evaluation, statistical learning algorithms.",
    },
    ArxivSubjectEntry {
        code: "cs.CL",
        archive: "Computer Science",
        name: "Computation and Language",
        description: "LLMs, NLI claim verification, structured generation, retrieval.",
    },
    ArxivSubjectEntry {
        code: "cs.PL",
        archive: "Computer Science",
        name: "Programming Languages",
        description: "Type systems, compilers, effect disciplines, language design.",
    },
    ArxivSubjectEntry {
        code: "cs.DC",
        archive: "Computer Science",
        name: "Distributed, Parallel, and Cluster Computing",
        description: "Swarm orchestration, local-first mesh, distributed consensus.",
    },
    ArxivSubjectEntry {
        code: "cs.CR",
        archive: "Computer Science",
        name: "Cryptography and Security",
        description: "Trust verification, provenance hashing, sandboxed execution.",
    },
    ArxivSubjectEntry {
        code: "cs.DB",
        archive: "Computer Science",
        name: "Databases",
        description: "Embedded storage engines, CAS ledgers, vector/graph indexing.",
    },
    ArxivSubjectEntry {
        code: "cs.IR",
        archive: "Computer Science",
        name: "Information Retrieval",
        description: "Hybrid BM25/semantic search, RRF fusion, RAG pipelines.",
    },
    ArxivSubjectEntry {
        code: "cs.HC",
        archive: "Computer Science",
        name: "Human-Computer Interaction",
        description: "IDE control planes, visual verification, human-in-the-loop review.",
    },
    ArxivSubjectEntry {
        code: "cs.MA",
        archive: "Computer Science",
        name: "Multiagent Systems",
        description: "Multi-agent coordination, bulletin boards, causal observability.",
    },
    ArxivSubjectEntry {
        code: "cs.RO",
        archive: "Computer Science",
        name: "Robotics",
        description: "Embodied agents, control loops, simulation-to-reality pipelines.",
    },
    ArxivSubjectEntry {
        code: "cs.CY",
        archive: "Computer Science",
        name: "Computers and Society",
        description: "AI governance, auditing policies, ethical & reproducibility standards.",
    },
    ArxivSubjectEntry {
        code: "stat.ML",
        archive: "Statistics",
        name: "Machine Learning (Statistics)",
        description: "Confidence intervals, statistical calibration, empirical methodology.",
    },
    ArxivSubjectEntry {
        code: "stat.AP",
        archive: "Statistics",
        name: "Applications (Statistics)",
        description: "Applied benchmark measurement and longitudinal provider telemetry.",
    },
    ArxivSubjectEntry {
        code: "stat.ME",
        archive: "Statistics",
        name: "Methodology (Statistics)",
        description: "Hypothesis testing, null-result power analysis, experimental design.",
    },
    ArxivSubjectEntry {
        code: "math.OC",
        archive: "Mathematics",
        name: "Optimization and Control",
        description: "Scheduling optimization, routing policies, constrained convergence.",
    },
    ArxivSubjectEntry {
        code: "q-bio.QM",
        archive: "Quantitative Biology",
        name: "Quantitative Methods",
        description: "Quantitative experimental reproducibility and data analysis.",
    },
    ArxivSubjectEntry {
        code: "physics.comp-ph",
        archive: "Physics",
        name: "Computational Physics",
        description: "High-precision numerical simulation and deterministic validation.",
    },
    ArxivSubjectEntry {
        code: "eess.SY",
        archive: "Electrical Engineering and Systems Science",
        name: "Systems and Control",
        description: "Feedback loops, autonomic schedulers, system telemetry.",
    },
    ArxivSubjectEntry {
        code: "econ.EM",
        archive: "Economics",
        name: "Econometrics",
        description: "Cost-quality Pareto frontiers and empirical model routing economics.",
    },
];

/// Valid `arXiv.org` subject catalog for scientific taxonomy assignment.
pub fn arxiv_subject_catalog() -> &'static [ArxivSubjectEntry] {
    ARXIV_SUBJECT_CATALOG
}

/// Find an `arXiv.org` subject entry by code (case-insensitive on archive prefix/suffix).
pub fn find_arxiv_subject(code: &str) -> Option<&'static ArxivSubjectEntry> {
    let trimmed = code.trim();
    ARXIV_SUBJECT_CATALOG
        .iter()
        .find(|entry| entry.code.eq_ignore_ascii_case(trimmed))
}

/// Whether `code` is a curated canonical `arXiv.org` subject code (or valid `archive.XX` pattern).
pub fn is_valid_arxiv_category(code: &str) -> bool {
    let trimmed = code.trim();
    if find_arxiv_subject(trimmed).is_some() {
        return true;
    }
    let Some((prefix, suffix)) = trimmed.split_once('.') else {
        return false;
    };
    !prefix.is_empty()
        && !suffix.is_empty()
        && prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        && suffix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Default primary + secondary `arXiv.org` subject categories for a [`FindingClass`].
pub fn default_arxiv_categories_for_class(
    class: FindingClass,
) -> (&'static str, Vec<&'static str>) {
    match class {
        FindingClass::EmpiricalScience => ("cs.AI", vec!["cs.SE", "stat.AP"]),
        FindingClass::AlgorithmicImprovement => ("cs.LG", vec!["cs.PL", "stat.ML"]),
        FindingClass::ReproducibilityInfra => ("cs.SE", vec!["cs.DC", "stat.ME"]),
        FindingClass::PolicyGovernance => ("cs.CY", vec!["cs.AI"]),
        FindingClass::TelemetryTrust => ("cs.CR", vec!["cs.SE"]),
        FindingClass::ModelCapabilityAtlas => ("cs.CL", vec!["cs.LG", "stat.ML"]),
        FindingClass::ProviderReliabilityAtlas => ("cs.DC", vec!["stat.AP", "cs.SE"]),
        FindingClass::Other => ("cs.SE", vec!["cs.AI"]),
    }
}

/// SPDX license identifiers accepted by `arXiv.org` submissions.
pub fn valid_spdx_licenses() -> &'static [&'static str] {
    &[
        "CC-BY-4.0",
        "CC-BY-SA-4.0",
        "CC-BY-NC-SA-4.0",
        "CC0-1.0",
        "ArXiv-1.0",
    ]
}

/// One row of `finding-class-defaults.v1.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassPolicy {
    /// How many days the right-of-reply window stays open before the
    /// manifest can ship live.
    pub reply_window_days: u32,
    /// Minimum number of null-result publications required per N positive
    /// findings in the same quarterly window. `0` means the negative-result
    /// mandate (Finalization Phase 10) does NOT apply to this class.
    pub negative_result_quota: u32,
    /// Whether the venue catalog defaults to allowing an audited LLM critic
    /// to substitute for the second human approver (Phase D). Per-venue
    /// rows in `venue-catalog.v1.yaml` can override.
    pub critic_allowed: bool,
    /// Ordered list of venues to recommend for this class, primary first.
    /// Empty → no recommendation (caller falls back to a manual choice).
    #[serde(default)]
    pub recommended_venues: Vec<String>,
}

/// Top-level shape of `finding-class-defaults.v1.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassDefaults {
    #[serde(flatten)]
    pub by_class: HashMap<String, ClassPolicy>,
}

impl ClassDefaults {
    pub fn policy_for(&self, class: FindingClass) -> Option<&ClassPolicy> {
        self.by_class.get(class.as_str())
    }
}

#[derive(Debug, Error)]
pub enum ClassRoutingError {
    #[error("yaml parse: {0}")]
    Yaml(String),
}

/// Load class defaults from YAML matching the shape of
/// `contracts/scientia/finding-class-defaults.v1.yaml`.
pub fn load_class_defaults_from_yaml(yaml: &str) -> Result<ClassDefaults, ClassRoutingError> {
    let by_class: HashMap<String, ClassPolicy> =
        serde_yaml::from_str(yaml).map_err(|e| ClassRoutingError::Yaml(e.to_string()))?;
    Ok(ClassDefaults { by_class })
}

/// Load class defaults from the repo SSOT file, falling back to builtins.
pub fn load_class_defaults_from_repo(repo_root: &Path) -> ClassDefaults {
    let path = repo_root.join("contracts/scientia/finding-class-defaults.v1.yaml");
    if !path.is_file() {
        return builtin_class_defaults();
    }
    match std::fs::read_to_string(&path) {
        Ok(raw) => load_class_defaults_from_yaml(&raw).unwrap_or_else(|_| builtin_class_defaults()),
        Err(_) => builtin_class_defaults(),
    }
}

/// Built-in defaults — what the system uses when no YAML is supplied.
/// These match the design-doc recommendations.
pub fn builtin_class_defaults() -> ClassDefaults {
    let mut by_class = HashMap::new();
    by_class.insert(
        FindingClass::AlgorithmicImprovement.as_str().to_string(),
        ClassPolicy {
            reply_window_days: 7,
            negative_result_quota: 0,
            critic_allowed: true,
            recommended_venues: vec!["ICSE".into(), "FSE".into(), "OOPSLA".into(), "PLDI".into()],
        },
    );
    by_class.insert(
        FindingClass::ReproducibilityInfra.as_str().to_string(),
        ClassPolicy {
            reply_window_days: 7,
            negative_result_quota: 0,
            critic_allowed: true,
            recommended_venues: vec!["REP".into(), "MSR".into(), "ICSE-SEIP".into()],
        },
    );
    by_class.insert(
        FindingClass::TelemetryTrust.as_str().to_string(),
        ClassPolicy {
            reply_window_days: 14,
            negative_result_quota: 0,
            // Provider-behavior implications → human approver required.
            critic_allowed: false,
            recommended_venues: vec![
                "MLSys-workshop".into(),
                "SOSP-workshop".into(),
                "USENIX-ATC".into(),
            ],
        },
    );
    by_class.insert(
        FindingClass::PolicyGovernance.as_str().to_string(),
        ClassPolicy {
            reply_window_days: 14,
            negative_result_quota: 0,
            critic_allowed: false,
            recommended_venues: vec!["AIES".into(), "FAccT".into()],
        },
    );
    by_class.insert(
        FindingClass::EmpiricalScience.as_str().to_string(),
        ClassPolicy {
            reply_window_days: 7,
            negative_result_quota: 0,
            critic_allowed: true,
            recommended_venues: vec![
                "arXiv".into(),
                "Nature-Scientific-Data".into(),
                "ACS-Omega".into(),
            ],
        },
    );
    by_class.insert(
        FindingClass::Other.as_str().to_string(),
        ClassPolicy {
            reply_window_days: 14,
            negative_result_quota: 0,
            critic_allowed: false,
            recommended_venues: vec![],
        },
    );
    // Atlas classes: unchanged Atlas behavior (14-day window, 3:1 negative
    // quota, critic forbidden).
    for atlas in [
        FindingClass::ModelCapabilityAtlas,
        FindingClass::ProviderReliabilityAtlas,
    ] {
        by_class.insert(
            atlas.as_str().to_string(),
            ClassPolicy {
                reply_window_days: 14,
                negative_result_quota: 3,
                critic_allowed: false,
                recommended_venues: vec!["IMC".into(), "MLSys".into()],
            },
        );
    }
    ClassDefaults { by_class }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn finding_class_round_trips_through_string() {
        for c in [
            FindingClass::AlgorithmicImprovement,
            FindingClass::ReproducibilityInfra,
            FindingClass::PolicyGovernance,
            FindingClass::TelemetryTrust,
            FindingClass::EmpiricalScience,
            FindingClass::Other,
            FindingClass::ModelCapabilityAtlas,
            FindingClass::ProviderReliabilityAtlas,
        ] {
            assert_eq!(FindingClass::from_str(c.as_str()), Some(c));
        }
        assert_eq!(FindingClass::from_str("nope"), None);
    }

    #[test]
    fn atlas_predicate_matches_only_atlas_classes() {
        assert!(FindingClass::ModelCapabilityAtlas.is_atlas());
        assert!(FindingClass::ProviderReliabilityAtlas.is_atlas());
        assert!(!FindingClass::AlgorithmicImprovement.is_atlas());
        assert!(!FindingClass::TelemetryTrust.is_atlas());
        assert!(!FindingClass::EmpiricalScience.is_atlas());
    }

    #[test]
    fn builtin_defaults_cover_all_classes() {
        let d = builtin_class_defaults();
        for c in [
            FindingClass::AlgorithmicImprovement,
            FindingClass::ReproducibilityInfra,
            FindingClass::PolicyGovernance,
            FindingClass::TelemetryTrust,
            FindingClass::EmpiricalScience,
            FindingClass::Other,
            FindingClass::ModelCapabilityAtlas,
            FindingClass::ProviderReliabilityAtlas,
        ] {
            assert!(d.policy_for(c).is_some(), "missing policy for {:?}", c);
        }
    }

    #[test]
    fn builtin_micro_classes_have_zero_negative_result_quota() {
        let d = builtin_class_defaults();
        for c in [
            FindingClass::AlgorithmicImprovement,
            FindingClass::ReproducibilityInfra,
            FindingClass::PolicyGovernance,
            FindingClass::TelemetryTrust,
            FindingClass::EmpiricalScience,
        ] {
            assert_eq!(d.policy_for(c).unwrap().negative_result_quota, 0);
        }
    }

    #[test]
    fn builtin_atlas_classes_keep_3to1_negative_quota() {
        let d = builtin_class_defaults();
        for atlas in [
            FindingClass::ModelCapabilityAtlas,
            FindingClass::ProviderReliabilityAtlas,
        ] {
            assert_eq!(d.policy_for(atlas).unwrap().negative_result_quota, 3);
        }
    }

    #[test]
    fn yaml_loader_round_trip() {
        let y = r#"
algorithmic_improvement:
  reply_window_days: 7
  negative_result_quota: 0
  critic_allowed: true
  recommended_venues: [ICSE, FSE]
telemetry_trust:
  reply_window_days: 14
  negative_result_quota: 0
  critic_allowed: false
  recommended_venues: [MLSys-workshop]
"#;
        let d = load_class_defaults_from_yaml(y).unwrap();
        let algimp = d.policy_for(FindingClass::AlgorithmicImprovement).unwrap();
        assert_eq!(algimp.reply_window_days, 7);
        assert!(algimp.critic_allowed);
        assert_eq!(algimp.recommended_venues, vec!["ICSE", "FSE"]);
        let teltr = d.policy_for(FindingClass::TelemetryTrust).unwrap();
        assert!(!teltr.critic_allowed);
    }

    #[test]
    fn malformed_yaml_yields_error() {
        let res = load_class_defaults_from_yaml("not: valid: yaml: at: all");
        assert!(matches!(res, Err(ClassRoutingError::Yaml(_))));
    }

    #[test]
    fn contract_yaml_loads_from_repo() {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let d = load_class_defaults_from_repo(&repo);
        let algimp = d
            .policy_for(FindingClass::AlgorithmicImprovement)
            .expect("algorithmic_improvement in contract");
        assert!(algimp.critic_allowed);
        assert!(
            !d.policy_for(FindingClass::TelemetryTrust)
                .expect("telemetry_trust in contract")
                .critic_allowed
        );
    }

    #[test]
    fn finding_class_resolves_paper_and_benchmark_aliases() {
        assert_eq!(
            FindingClass::from_str("paper"),
            Some(FindingClass::EmpiricalScience)
        );
        assert_eq!(
            FindingClass::from_str("SCIENTIA"),
            Some(FindingClass::EmpiricalScience)
        );
        assert_eq!(
            FindingClass::from_str("benchmark-report"),
            Some(FindingClass::AlgorithmicImprovement)
        );
        assert_eq!(
            FindingClass::from_str("governance"),
            Some(FindingClass::PolicyGovernance)
        );
    }

    #[test]
    fn publication_tier_and_arxiv_catalog_contracts_hold() {
        for tier in PublicationTier::all() {
            assert_eq!(PublicationTier::from_str(tier.as_str()), Some(*tier));
            assert!(tier.minimum_citations() >= 1);
        }
        assert!(!PublicationTier::Nanopublication.requires_full_imrad());
        assert!(PublicationTier::ArxivPreprint.requires_full_imrad());
        assert!(is_valid_arxiv_category("cs.SE"));
        assert!(is_valid_arxiv_category("stat.ML"));
        assert!(!is_valid_arxiv_category("invalid_cat"));
        let (primary, secondaries) =
            default_arxiv_categories_for_class(FindingClass::AlgorithmicImprovement);
        assert!(is_valid_arxiv_category(primary));
        assert!(!secondaries.is_empty());
        assert!(valid_spdx_licenses().contains(&"CC-BY-4.0"));
    }
}
