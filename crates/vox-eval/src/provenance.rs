//! Run provenance (design §5): the facts that must travel with every benchmark
//! report for its numbers to be reproducible and comparable.
//!
//! Pure data plus one rule, [`RunProvenance::missing_required`]. Nothing here
//! defaults or backfills a required field: an absent JSON key deserializes to
//! `None`, and `None` is reported missing. Hashing and gathering of the values
//! is the CLI's job (12-07); this crate has no crypto edge.
//!
//! `LlmConfig` (which holds the provider credential) is never serialized into a report;
//! [`SamplingRecord`] is the explicit replacement.

use serde::{Deserialize, Serialize};

/// Integrity group: who this run is within a campaign (filled by 12-09).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunIdentity {
    /// Campaign this run belongs to.
    pub campaign_id: String,
    /// Unique id of this run.
    pub run_id: String,
    /// Replicate index within the campaign.
    pub replicate_id: u32,
    /// Attempt index (0 for the first try).
    pub attempt_index: u32,
    /// Run this one retries, if any.
    pub retry_of_run_id: Option<String>,
}

/// Sampling group: the generation settings actually used (filled by 12-07).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SamplingRecord {
    /// Sampling temperature.
    pub temperature: f32,
    /// Nucleus sampling cutoff.
    pub top_p: f32,
    /// Samples drawn per problem.
    pub n: usize,
    /// The k that pass@k was computed at.
    pub k: usize,
    /// Base sampling seed.
    pub seed: u64,
    /// Max tokens per completion.
    pub max_tokens: u64,
    /// Whether the provider is known to honor `seed`.
    pub seed_enforced: bool,
    /// Compiler-feedback repair turns per failed sample (0 = none). Reports written before
    /// this field existed had no feedback turns, so 0 is the true value, not a guess.
    #[serde(default)]
    pub feedback_turns: u32,
}

/// Model-MENS group: lineage of a `mens/*` adapter (filled by 12-07 from the AdapterCard).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MensLineage {
    /// `<hf id>@<revision>` of the base model.
    pub base_hf_id_at_revision: Option<String>,
    /// SHA-256 of the adapter weights.
    pub adapter_sha256: Option<String>,
    /// Digest of the training manifest.
    pub training_manifest_digest: Option<String>,
    /// SHA-256 of the training data.
    pub train_data_sha256: Option<String>,
    /// Syntax fingerprint the adapter was trained against.
    pub training_syntax_fingerprint: Option<String>,
    /// Which fingerprint scheme `training_syntax_fingerprint` uses.
    pub training_syntax_fingerprint_kind: Option<String>,
    /// Whether training syntax differs from the evaluated syntax.
    pub syntax_drift: Option<bool>,
}

/// The §5 provenance record. Every field is `Option` (or an empty-able vec) so
/// that absence is representable and reportable rather than defaulted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunProvenance {
    // Language
    /// Language group: fingerprint of the Vox syntax surface under test.
    pub syntax_fingerprint: Option<String>,
    /// Language group: fingerprint scheme name.
    pub syntax_fingerprint_kind: Option<String>,
    /// Language group: `cargo --version` of the build.
    pub cargo_version: Option<String>,
    /// Language group: git SHA of the vox checkout.
    pub vox_git_sha: Option<String>,
    /// Language group: SHA-256 of the verifier `vox` binary.
    pub vox_binary_sha256: Option<String>,
    /// Language group: verifier version string.
    pub verifier_version: Option<String>,
    /// Language group: verifier binary matches the harness build.
    pub verifier_matches_harness: Option<bool>,

    // Benchmark
    /// Benchmark group: tier name (e.g. smoke, full).
    pub tier: Option<String>,
    /// Benchmark group: hash of the problem set.
    pub problem_set_hash: Option<String>,
    /// Benchmark group: corpus manifest hash. Informational, never required.
    pub corpus_manifest_hash: Option<String>,
    /// Benchmark group: hash of the prompt template.
    pub prompt_template_hash: Option<String>,
    /// Benchmark group: prompting condition (e.g. zero-shot, rag).
    pub condition: Option<String>,
    /// Benchmark group: hash of the context supplied to the model.
    pub context_hash: Option<String>,
    /// Benchmark group: harness version string.
    pub harness_version: Option<String>,

    // Model
    /// Model group: model id (`mens/*` for MENS adapters).
    pub model_id: Option<String>,
    /// Model group (external): provider name.
    pub provider: Option<String>,
    /// Model group (external): date the requests were made.
    pub request_date: Option<String>,
    /// Model group (external): model strings the provider reported back.
    #[serde(default)]
    pub provider_reported_models: Vec<String>,
    /// Model group (MENS): adapter lineage.
    pub mens: Option<MensLineage>,

    // Sampling
    /// Sampling group: the settings actually used.
    pub sampling: Option<SamplingRecord>,

    // Runner
    /// Runner group: runner name or host class.
    pub runner: Option<String>,
    /// Runner group: operating system.
    pub os: Option<String>,
    /// Runner group: CPU architecture.
    pub arch: Option<String>,
    /// Runner group: per-candidate verifier timeout in seconds.
    pub verifier_timeout_secs: Option<u64>,

    // Integrity
    /// Integrity group: campaign/run identity (12-09).
    pub identity: Option<RunIdentity>,
    /// Integrity group: preregistration id the run executes.
    pub preregistration_id: Option<String>,
}

impl RunProvenance {
    /// Names of required fields that are absent or blank, in fixed declaration order.
    ///
    /// The single field-completeness rule: the report gate and the read-side
    /// recheck both call this and nothing else decides it.
    #[must_use]
    pub fn missing_required(&self) -> Vec<&'static str> {
        let mut m = Vec::new();
        let mut req = |name: &'static str, missing: bool| {
            if missing {
                m.push(name);
            }
        };
        req("syntax_fingerprint", blank(&self.syntax_fingerprint));
        req(
            "syntax_fingerprint_kind",
            blank(&self.syntax_fingerprint_kind),
        );
        req("cargo_version", blank(&self.cargo_version));
        req("vox_git_sha", blank(&self.vox_git_sha));
        req("vox_binary_sha256", blank(&self.vox_binary_sha256));
        req("verifier_version", blank(&self.verifier_version));
        req(
            "verifier_matches_harness",
            self.verifier_matches_harness.is_none(),
        );
        req("tier", blank(&self.tier));
        req("problem_set_hash", blank(&self.problem_set_hash));
        // corpus_manifest_hash is informational and never required.
        req("prompt_template_hash", blank(&self.prompt_template_hash));
        req("condition", blank(&self.condition));
        req("context_hash", blank(&self.context_hash));
        req("harness_version", blank(&self.harness_version));
        req("model_id", blank(&self.model_id));
        req("sampling", self.sampling.is_none());
        req("runner", blank(&self.runner));
        req("os", blank(&self.os));
        req("arch", blank(&self.arch));
        req(
            "verifier_timeout_secs",
            self.verifier_timeout_secs.is_none(),
        );
        match &self.identity {
            None => req("identity", true),
            Some(i) => {
                req("identity.campaign_id", i.campaign_id.trim().is_empty());
                req("identity.run_id", i.run_id.trim().is_empty());
            }
        }
        req("preregistration_id", blank(&self.preregistration_id));
        if self
            .model_id
            .as_deref()
            .is_some_and(|id| id.starts_with("mens/"))
        {
            match &self.mens {
                None => req("mens", true),
                Some(l) => {
                    req(
                        "mens.base_hf_id_at_revision",
                        blank(&l.base_hf_id_at_revision),
                    );
                    req("mens.adapter_sha256", blank(&l.adapter_sha256));
                    req(
                        "mens.training_manifest_digest",
                        blank(&l.training_manifest_digest),
                    );
                    req("mens.train_data_sha256", blank(&l.train_data_sha256));
                    req(
                        "mens.training_syntax_fingerprint",
                        blank(&l.training_syntax_fingerprint),
                    );
                    req(
                        "mens.training_syntax_fingerprint_kind",
                        blank(&l.training_syntax_fingerprint_kind),
                    );
                    req("mens.syntax_drift", l.syntax_drift.is_none());
                }
            }
        } else {
            req("provider", blank(&self.provider));
            req("request_date", blank(&self.request_date));
            req(
                "provider_reported_models",
                self.provider_reported_models
                    .iter()
                    .all(|s| s.trim().is_empty()),
            );
        }
        m
    }
}

/// True when the value is absent or whitespace-only.
fn blank(v: &Option<String>) -> bool {
    v.as_deref().is_none_or(|s| s.trim().is_empty())
}

/// A fully populated external-model provenance for tests in this crate.
#[cfg(test)]
pub(crate) fn test_complete_external() -> RunProvenance {
    let s = |v: &str| Some(v.to_string());
    RunProvenance {
        syntax_fingerprint: s("fp"),
        syntax_fingerprint_kind: s("grammar-v1"),
        cargo_version: s("cargo 1.0"),
        vox_git_sha: s("abc123"),
        vox_binary_sha256: s("bin-sha"),
        verifier_version: s("0.6.0"),
        verifier_matches_harness: Some(true),
        tier: s("smoke"),
        problem_set_hash: s("ps-hash"),
        corpus_manifest_hash: s("cm-hash"),
        prompt_template_hash: s("pt-hash"),
        condition: s("zero-shot"),
        context_hash: s("ctx-hash"),
        harness_version: s("h1"),
        model_id: s("openai/gpt-x"),
        provider: s("openai"),
        request_date: s("2026-10-11"),
        provider_reported_models: vec!["gpt-x-2026-10-01".to_string()],
        mens: None,
        sampling: Some(SamplingRecord {
            temperature: 0.2,
            top_p: 0.95,
            n: 5,
            k: 5,
            seed: 7,
            max_tokens: 1024,
            seed_enforced: false,
            feedback_turns: 0,
        }),
        runner: s("local"),
        os: s("macos"),
        arch: s("aarch64"),
        verifier_timeout_secs: Some(30),
        identity: Some(RunIdentity {
            campaign_id: "camp".to_string(),
            run_id: "run-1".to_string(),
            replicate_id: 0,
            attempt_index: 0,
            retry_of_run_id: None,
        }),
        preregistration_id: s("prereg-1"),
    }
}

/// A fully populated `mens/*` provenance for tests in this crate.
#[cfg(test)]
pub(crate) fn test_complete_mens() -> RunProvenance {
    let s = |v: &str| Some(v.to_string());
    RunProvenance {
        model_id: s("mens/demo"),
        provider: None,
        request_date: None,
        provider_reported_models: Vec::new(),
        mens: Some(MensLineage {
            base_hf_id_at_revision: s("Qwen/Qwen3-8B@b968826"),
            adapter_sha256: s("ad-sha"),
            training_manifest_digest: s("tm-digest"),
            train_data_sha256: s("td-sha"),
            training_syntax_fingerprint: s("fp"),
            training_syntax_fingerprint_kind: s("grammar-v1"),
            syntax_drift: Some(false),
        }),
        ..test_complete_external()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_external_provenance_has_nothing_missing() {
        assert!(test_complete_external().missing_required().is_empty());
    }

    #[test]
    fn blank_and_absent_keys_count_as_missing() {
        let mut p = test_complete_external();
        p.cargo_version = Some("  ".into());
        assert_eq!(p.missing_required(), ["cargo_version"]);

        let mut v = serde_json::to_value(test_complete_external()).unwrap();
        v.as_object_mut().unwrap().remove("vox_git_sha");
        let p: RunProvenance = serde_json::from_value(v).unwrap();
        assert_eq!(p.missing_required(), ["vox_git_sha"]);
    }

    type Clear = fn(&mut RunProvenance);

    #[test]
    fn each_required_field_is_reported_when_cleared() {
        let table: Vec<(&str, Clear)> = vec![
            ("syntax_fingerprint", |p| p.syntax_fingerprint = None),
            ("syntax_fingerprint_kind", |p| {
                p.syntax_fingerprint_kind = None
            }),
            ("cargo_version", |p| p.cargo_version = None),
            ("vox_git_sha", |p| p.vox_git_sha = None),
            ("vox_binary_sha256", |p| p.vox_binary_sha256 = None),
            ("verifier_version", |p| p.verifier_version = None),
            ("verifier_matches_harness", |p| {
                p.verifier_matches_harness = None
            }),
            ("tier", |p| p.tier = None),
            ("problem_set_hash", |p| p.problem_set_hash = None),
            ("prompt_template_hash", |p| p.prompt_template_hash = None),
            ("condition", |p| p.condition = None),
            ("context_hash", |p| p.context_hash = None),
            ("harness_version", |p| p.harness_version = None),
            ("model_id", |p| p.model_id = None),
            ("sampling", |p| p.sampling = None),
            ("runner", |p| p.runner = None),
            ("os", |p| p.os = None),
            ("arch", |p| p.arch = None),
            ("verifier_timeout_secs", |p| p.verifier_timeout_secs = None),
            ("identity", |p| p.identity = None),
            ("preregistration_id", |p| p.preregistration_id = None),
        ];
        for (name, clear) in table {
            let mut p = test_complete_external();
            clear(&mut p);
            assert_eq!(p.missing_required(), [name], "clearing {name}");
        }
        // Informational only: never required.
        let mut p = test_complete_external();
        p.corpus_manifest_hash = None;
        assert!(p.missing_required().is_empty());
    }

    #[test]
    fn blank_identity_ids_are_missing() {
        let mut p = test_complete_external();
        p.identity.as_mut().unwrap().campaign_id = " ".into();
        p.identity.as_mut().unwrap().run_id = String::new();
        assert_eq!(
            p.missing_required(),
            ["identity.campaign_id", "identity.run_id"]
        );
    }

    #[test]
    fn mens_ids_require_lineage() {
        assert!(test_complete_mens().missing_required().is_empty());
        let mut p = test_complete_mens();
        p.mens = None;
        assert_eq!(p.missing_required(), ["mens"]);

        let table: Vec<(&str, Clear)> = vec![
            ("mens.base_hf_id_at_revision", |p| {
                p.mens.as_mut().unwrap().base_hf_id_at_revision = None
            }),
            ("mens.adapter_sha256", |p| {
                p.mens.as_mut().unwrap().adapter_sha256 = None
            }),
            ("mens.training_manifest_digest", |p| {
                p.mens.as_mut().unwrap().training_manifest_digest = Some(String::new())
            }),
            ("mens.train_data_sha256", |p| {
                p.mens.as_mut().unwrap().train_data_sha256 = None
            }),
            ("mens.training_syntax_fingerprint", |p| {
                p.mens.as_mut().unwrap().training_syntax_fingerprint = None
            }),
            ("mens.training_syntax_fingerprint_kind", |p| {
                p.mens.as_mut().unwrap().training_syntax_fingerprint_kind = None
            }),
            ("mens.syntax_drift", |p| {
                p.mens.as_mut().unwrap().syntax_drift = None
            }),
        ];
        for (name, clear) in table {
            let mut p = test_complete_mens();
            clear(&mut p);
            assert_eq!(p.missing_required(), [name], "clearing {name}");
        }
    }

    #[test]
    fn external_ids_require_provider_snapshot() {
        let table: Vec<(&str, Clear)> = vec![
            ("provider", |p| p.provider = None),
            ("request_date", |p| p.request_date = Some("\t".into())),
            ("provider_reported_models", |p| {
                p.provider_reported_models.clear()
            }),
        ];
        for (name, clear) in table {
            let mut p = test_complete_external();
            clear(&mut p);
            assert_eq!(p.missing_required(), [name], "clearing {name}");
        }
    }

    #[test]
    fn missing_names_come_in_declaration_order() {
        let mut p = test_complete_external();
        p.preregistration_id = None;
        p.os = None;
        p.tier = None;
        p.cargo_version = None;
        p.syntax_fingerprint = None;
        p.provider = None;
        let expected = [
            "syntax_fingerprint",
            "cargo_version",
            "tier",
            "os",
            "preregistration_id",
            "provider",
        ];
        assert_eq!(p.missing_required(), expected);
        assert_eq!(p.clone().missing_required(), expected);
    }
}
