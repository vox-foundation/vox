//! Strongly-typed `CommunityIntelligenceBundleV1` (`#[serde(deny_unknown_fields)]`),
//! `Ed25519` signature verification via `vox_crypto::verify_signature_hex`, non-blocking
//! client cache hydration, and bounded Empirical Bayes prior blending (`INTEL-01..05`, `TYPING-04`).

use anyhow::{Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Domain 1a: Model routing prior row (`intelligence_public.v_model_routing_priors_7d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelRoutingPriorRowV1 {
    pub model_id_hash: String,
    pub provider: String,
    pub task_category: String,
    pub distinct_clients: i64,
    pub total_events: i64,
    pub empirical_success_rate: f64,
    pub modal_duration_bucket: String,
}

/// Domain 1b: 5-spoke model benchmark prior row (`intelligence_public.v_model_bench_spoke_priors_14d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelBenchSpokePriorRowV1 {
    pub model_id_hash: String,
    pub spoke: String,
    pub distinct_clients: i64,
    pub total_events: i64,
    pub empirical_pass_rate: f64,
}

/// Domain 2: Error surface prior row (`intelligence_public.v_error_surface_7d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorSurfacePriorRowV1 {
    pub error_class: String,
    pub subsystem: String,
    pub distinct_clients: i64,
    pub total_events: i64,
    pub recoverable_rate: f64,
}

/// Domain 3: Default decision prior row (`intelligence_public.v_default_decision_priors_30d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultDecisionPriorRowV1 {
    pub decision_id: String,
    pub chosen_value: String,
    pub outcome: String,
    pub distinct_clients: i64,
    pub total_events: i64,
}

/// Domain 4: Prompt profile prior row (`intelligence_public.v_prompt_profile_priors_14d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptProfilePriorRowV1 {
    pub canonical_model_id: String,
    pub profile_variant_id: String,
    pub task_category: String,
    pub quality_bucket: String,
    pub distinct_clients: i64,
    pub total_events: i64,
}

/// Domain 5: Skill reliability prior row (`intelligence_public.v_skill_reliability_14d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SkillReliabilityPriorRowV1 {
    pub skill_id_hash: String,
    pub distinct_clients: i64,
    pub total_events: i64,
    pub acceptance_rate: f64,
}

/// Domain 6: Training throughput prior row (`intelligence_public.v_training_throughput_priors_30d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrainingThroughputPriorRowV1 {
    pub gpu_vendor: String,
    pub vram_bucket: String,
    pub seq_len_bucket: String,
    pub modal_tok_per_sec_bucket: String,
    pub distinct_clients: i64,
    pub total_events: i64,
}

/// Domain 7: Domain misguidance hotspot row (`intelligence_public.v_domain_misguidance_hotspots_30d`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainMisguidanceHotspotRowV1 {
    pub domain_hash: String,
    pub domain_category: String,
    pub defect_class: String,
    pub distinct_clients: i64,
    pub total_events: i64,
}

/// Lightweight manifest served at `GET /v1/intelligence/manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommunityIntelligenceManifestV1 {
    pub schema_version: u32,
    pub bundle_version: String,
    pub generated_at_ms: i64,
    pub expires_at_ms: i64,
    pub etag: String,
    pub bundle_blake3_hex: String,
    pub signer_public_key_hex: String,
}

/// Strongly-typed 7-domain `CommunityIntelligenceBundleV1` (`#[serde(deny_unknown_fields)]`) (`INTEL-01`, `TYPING-04`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CommunityIntelligenceBundleV1 {
    pub schema_version: u32,
    pub bundle_version: String,
    pub generated_at_ms: i64,
    pub expires_at_ms: i64,
    pub k_anonymity_floor: u32,
    pub min_events_floor: u32,
    pub laplace_epsilon: f64,
    pub signer_public_key_hex: String,
    pub ed25519_signature_hex: String,
    pub model_routing_priors: Vec<ModelRoutingPriorRowV1>,
    pub model_bench_spoke_priors: Vec<ModelBenchSpokePriorRowV1>,
    pub error_surface_priors: Vec<ErrorSurfacePriorRowV1>,
    pub default_decision_priors: Vec<DefaultDecisionPriorRowV1>,
    pub prompt_profile_priors: Vec<PromptProfilePriorRowV1>,
    pub skill_reliability_priors: Vec<SkillReliabilityPriorRowV1>,
    pub training_throughput_priors: Vec<TrainingThroughputPriorRowV1>,
    pub domain_misguidance_hotspots: Vec<DomainMisguidanceHotspotRowV1>,
}

impl CommunityIntelligenceBundleV1 {
    /// Deterministic byte payload covered by `ed25519_signature_hex` (matches `vox-ops-intelligence`).
    #[must_use]
    pub fn canonical_signing_bytes(&self) -> Vec<u8> {
        let mut unsigned = self.clone();
        unsigned.ed25519_signature_hex = String::new();
        serde_json::to_vec(&unsigned).expect("serialize canonical CommunityIntelligenceBundleV1")
    }

    /// Sign this bundle using `vox_crypto::sign` (`Ed25519`).
    pub fn sign_with_seed(&mut self, seed: &[u8; 32]) {
        let sk = vox_crypto::signing_key_from_bytes(seed);
        let vk = vox_crypto::to_verifying_key(&sk);
        self.signer_public_key_hex =
            vox_crypto::hex_encode(&vox_crypto::verifying_key_to_bytes(&vk));
        let sig = vox_crypto::sign(&sk, &self.canonical_signing_bytes());
        self.ed25519_signature_hex = vox_crypto::hex_encode(&sig);
    }

    /// Verify schema version, $k \ge 20$ / $n \ge 50$ floors, and `Ed25519` signature via `vox_crypto::verify_signature_hex`.
    pub fn verify_bundle_signature(&self) -> Result<()> {
        ensure!(
            self.schema_version == 1,
            "unsupported CommunityIntelligenceBundleV1 schema_version: {}",
            self.schema_version
        );
        ensure!(
            self.expires_at_ms > self.generated_at_ms,
            "CommunityIntelligenceBundleV1 expires_at_ms must be greater than generated_at_ms"
        );
        ensure!(
            self.k_anonymity_floor >= 20 && self.min_events_floor >= 50,
            "bundle k-anonymity floor ({}/{}) violates k>=20 / n>=50 invariant",
            self.k_anonymity_floor,
            self.min_events_floor
        );
        let k_min = i64::from(self.k_anonymity_floor);
        let n_min = i64::from(self.min_events_floor);
        let check_cohort = |domain: &str, k: i64, n: i64| -> Result<()> {
            ensure!(
                k >= k_min && n >= n_min,
                "{domain} row ({k}/{n}) below k>={k_min} / n>={n_min} threshold"
            );
            Ok(())
        };
        for r in &self.model_routing_priors {
            check_cohort("model_routing_prior", r.distinct_clients, r.total_events)?;
        }
        for r in &self.model_bench_spoke_priors {
            check_cohort(
                "model_bench_spoke_prior",
                r.distinct_clients,
                r.total_events,
            )?;
        }
        for r in &self.error_surface_priors {
            check_cohort("error_surface_prior", r.distinct_clients, r.total_events)?;
        }
        for r in &self.default_decision_priors {
            check_cohort("default_decision_prior", r.distinct_clients, r.total_events)?;
        }
        for r in &self.prompt_profile_priors {
            check_cohort("prompt_profile_prior", r.distinct_clients, r.total_events)?;
        }
        for r in &self.skill_reliability_priors {
            check_cohort(
                "skill_reliability_prior",
                r.distinct_clients,
                r.total_events,
            )?;
        }
        for r in &self.training_throughput_priors {
            check_cohort(
                "training_throughput_prior",
                r.distinct_clients,
                r.total_events,
            )?;
        }
        for r in &self.domain_misguidance_hotspots {
            check_cohort(
                "domain_misguidance_hotspot",
                r.distinct_clients,
                r.total_events,
            )?;
        }
        let ok = vox_crypto::verify_signature_hex(
            &self.signer_public_key_hex,
            &self.canonical_signing_bytes(),
            &self.ed25519_signature_hex,
        )
        .map_err(|e| anyhow::anyhow!("Ed25519 verification error: {e}"))?;
        if !ok {
            bail!("CommunityIntelligenceBundleV1 Ed25519 signature verification failed");
        }
        Ok(())
    }
}

/// Compute `BLAKE3(model_id)` lowercase hex (64 chars) via `vox_crypto::secure_hash`.
#[must_use]
pub fn model_id_blake3_hex(model_id: &str) -> String {
    vox_crypto::hex_encode(&vox_crypto::secure_hash(model_id.as_bytes()))
}

/// Bounded Empirical Bayes blend of a local rate with a community prior (`INTEL-03`, `INTEL-04`):
///
/// $$w_{\text{local}} = \frac{n_{\text{local}}}{n_{\text{local}} + 10}$$
///
/// The blended result is clamped so the community prior can shift the base/local score by at most $\pm 15\%$ (`0.15`).
#[must_use]
pub fn blend_empirical_bayes_prior(local_rate: f64, local_n: u64, community_rate: f64) -> f64 {
    let w_local = (local_n as f64) / ((local_n as f64) + 10.0);
    let raw_blend = (w_local * local_rate) + ((1.0 - w_local) * community_rate);
    let delta = (raw_blend - local_rate).clamp(-0.15, 0.15);
    (local_rate + delta).clamp(0.0, 1.0)
}

/// Check whether community intelligence hydration is enabled (`VOX_COMMUNITY_INTELLIGENCE != off/0/false`).
#[must_use]
pub fn is_community_intelligence_enabled() -> bool {
    match std::env::var("VOX_COMMUNITY_INTELLIGENCE") {
        Ok(val) => !matches!(
            val.trim().to_ascii_lowercase().as_str(),
            "0" | "off" | "false" | "no" | "disabled"
        ),
        Err(_) => true,
    }
}

/// Canonical local cache path (`~/.vox/cache/community-intelligence.v1.json`).
#[must_use]
pub fn default_bundle_cache_path() -> PathBuf {
    vox_config::paths::dot_vox_user_dir()
        .join("cache")
        .join("community-intelligence.v1.json")
}

/// Load and verify a cached `CommunityIntelligenceBundleV1` from disk if enabled and valid.
/// Never blocks on network and never logs warnings when offline or absent (`INTEL-05`).
#[must_use]
pub fn load_cached_bundle_if_valid(
    path: Option<&std::path::Path>,
) -> Option<CommunityIntelligenceBundleV1> {
    if !is_community_intelligence_enabled() {
        return None;
    }
    let cache_path = path
        .map(PathBuf::from)
        .unwrap_or_else(default_bundle_cache_path);
    let raw = std::fs::read_to_string(cache_path).ok()?;
    let bundle: CommunityIntelligenceBundleV1 = serde_json::from_str(&raw).ok()?;
    bundle.verify_bundle_signature().ok()?;
    Some(bundle)
}

/// Resolve the remote intelligence bundle URL from an explicit override or
/// `SecretId::VoxTelemetryUploadUrl` (`ROUTE-LEARN-02`).
#[must_use]
pub fn resolve_intelligence_bundle_url(endpoint_override: Option<&str>) -> Option<String> {
    let raw = endpoint_override.map(str::to_owned).or_else(|| {
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxTelemetryUploadUrl)
            .expose()
            .map(str::to_owned)
    })?;
    let trimmed = raw.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with(".json") {
        Some(trimmed.to_owned())
    } else if let Some(base) = trimmed.strip_suffix("/v1/logs") {
        Some(format!("{base}/v1/intelligence/bundle.json"))
    } else {
        Some(format!("{trimmed}/v1/intelligence/bundle.json"))
    }
}

/// Non-blocking remote sync of `CommunityIntelligenceBundleV1` (`ROUTE-LEARN-02`).
///
/// Fetches the signed bundle from the centralized server (`GET /v1/intelligence/bundle.json`),
/// enforces `Ed25519` signature + $k \ge 20$ / $n \ge 50$ verification before persisting to
/// `~/.vox/cache/community-intelligence.v1.json`, and returns `Ok(None)` when disabled or
/// unconfigured so offline operation is never disrupted.
pub async fn sync_community_intelligence_from_remote(
    endpoint_override: Option<&str>,
    cache_path: Option<&std::path::Path>,
) -> Result<Option<CommunityIntelligenceBundleV1>> {
    if !is_community_intelligence_enabled() {
        return Ok(None);
    }
    let Some(url) = resolve_intelligence_bundle_url(endpoint_override) else {
        return Ok(None);
    };
    let client = vox_http_client::client_builder()
        .timeout(vox_config::timeouts::D_10S)
        .build()
        .unwrap_or_else(|_| vox_http_client::client());

    let mut req = client.get(&url);
    if let Some(tok) = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxTelemetryUploadToken)
        .expose()
        .filter(|s| !s.trim().is_empty())
    {
        req = req.header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", tok.trim()),
        );
    }
    let resp = req.send().await?;
    ensure!(
        resp.status().is_success(),
        "intelligence bundle fetch failed with HTTP {}",
        resp.status()
    );
    let body = resp.text().await?;
    let bundle: CommunityIntelligenceBundleV1 = serde_json::from_str(&body)?;
    bundle.verify_bundle_signature()?;

    let dest = cache_path
        .map(PathBuf::from)
        .unwrap_or_else(default_bundle_cache_path);
    if let Some(parent) = dest.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(&dest, &body)?;
    Ok(Some(bundle))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ModelRegistry, ModelSpec, PricingSource, ProviderType, StrengthTag};

    fn test_spec(id: &str) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "ollama".into(),
            provider_type: ProviderType::Ollama,
            max_tokens: 32_768,
            cost_per_1k: 0.0,
            cost_per_1k_input: 0.0,
            cost_per_1k_output: 0.0,
            is_free: true,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: Default::default(),
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    fn sample_signed_bundle(model_id: &str) -> CommunityIntelligenceBundleV1 {
        let mut bundle = CommunityIntelligenceBundleV1 {
            schema_version: 1,
            bundle_version: "2026.10.19.1".into(),
            generated_at_ms: 1_729_380_000_000,
            expires_at_ms: 1_760_916_000_000,
            k_anonymity_floor: 20,
            min_events_floor: 50,
            laplace_epsilon: 1.0,
            signer_public_key_hex: String::new(),
            ed25519_signature_hex: String::new(),
            model_routing_priors: vec![ModelRoutingPriorRowV1 {
                model_id_hash: model_id_blake3_hex(model_id),
                provider: "ollama".into(),
                task_category: "codegen".into(),
                distinct_clients: 24,
                total_events: 82,
                empirical_success_rate: 0.92,
                modal_duration_bucket: "lt1s".into(),
            }],
            model_bench_spoke_priors: vec![],
            error_surface_priors: vec![],
            default_decision_priors: vec![],
            prompt_profile_priors: vec![],
            skill_reliability_priors: vec![],
            training_throughput_priors: vec![],
            domain_misguidance_hotspots: vec![],
        };
        bundle.sign_with_seed(&[7u8; 32]);
        bundle
    }

    #[test]
    fn intelligence_bundle_verifies_ed25519_signature_and_rejects_unknown_fields_or_under_k20() {
        let mut bundle = sample_signed_bundle("mens/qwen3.5-27b:five-spoke-qlora-r2");
        bundle
            .verify_bundle_signature()
            .expect("valid signed bundle must verify");

        // Tampering invalidates Ed25519 signature
        let mut tampered = bundle.clone();
        tampered.model_routing_priors[0].empirical_success_rate = 0.99;
        assert!(tampered.verify_bundle_signature().is_err());

        // k < 20 cohort is rejected even if re-signed
        bundle.model_routing_priors[0].distinct_clients = 19;
        bundle.sign_with_seed(&[7u8; 32]);
        assert!(bundle.verify_bundle_signature().is_err());

        // Unknown fields are rejected by #[serde(deny_unknown_fields)]
        let mut val = serde_json::to_value(sample_signed_bundle("m1")).unwrap();
        val.as_object_mut()
            .unwrap()
            .insert("unexpected_field".into(), serde_json::json!(true));
        assert!(serde_json::from_value::<CommunityIntelligenceBundleV1>(val).is_err());
    }

    #[test]
    fn community_intelligence_empirical_bayes_clamps_to_15_pct_and_preserves_offline_equivalence() {
        // Cold start (n_local = 0): local_rate = 0.50, community = 0.95 -> clamped to 0.50 + 0.15 = 0.65
        let cold = blend_empirical_bayes_prior(0.50, 0, 0.95);
        assert!((cold - 0.65).abs() < 1e-9, "cold={cold}");

        // Mature local history (n_local = 90): w_local = 0.90, local_rate = 0.80, community = 0.90 -> 0.81
        let mature = blend_empirical_bayes_prior(0.80, 90, 0.90);
        assert!((mature - 0.81).abs() < 1e-9, "mature={mature}");

        // Registry application & tampered / offline no-op equivalence
        let model_id = "mens/qwen3.5-27b:five-spoke-qlora-r2";
        let mut reg = ModelRegistry::default();
        reg.register(test_spec(model_id));

        let valid_bundle = sample_signed_bundle(model_id);
        let mut tampered_bundle = valid_bundle.clone();
        tampered_bundle.ed25519_signature_hex = "00".repeat(64);
        assert_eq!(reg.apply_community_intelligence_bundle(&tampered_bundle), 0);

        let updated = reg.apply_community_intelligence_bundle(&valid_bundle);
        assert_eq!(updated, 1);
    }

    #[test]
    fn resolve_intelligence_bundle_url_normalizes_logs_and_json_endpoints() {
        assert_eq!(
            resolve_intelligence_bundle_url(Some("https://ops.voxlang.org/v1/logs")),
            Some("https://ops.voxlang.org/v1/intelligence/bundle.json".into())
        );
        assert_eq!(
            resolve_intelligence_bundle_url(Some("https://ops.voxlang.org/")),
            Some("https://ops.voxlang.org/v1/intelligence/bundle.json".into())
        );
        assert_eq!(
            resolve_intelligence_bundle_url(Some("https://ops.voxlang.org/custom/bundle.json")),
            Some("https://ops.voxlang.org/custom/bundle.json".into())
        );
    }

    #[test]
    fn arena_preference_outcome_updates_bradley_terry_quality_bandit_arms_and_routing_rank() {
        use crate::config::CostPreference;
        use crate::models::scoring::auto_score_model;
        use crate::routing::ModelSelectionEngine;
        use crate::types::TaskCategory;

        let mut reg = ModelRegistry::default();
        reg.clear();
        let m_a = test_spec("ollama/candidate-a");
        let m_b = test_spec("ollama/candidate-b");
        reg.register(m_a.clone());
        reg.register(m_b.clone());

        // Initially equal scores
        let s_a0 = auto_score_model(
            &m_a,
            5,
            false,
            None,
            CostPreference::Performance,
            None,
            reg.get_score(&m_a.id),
        );
        let s_b0 = auto_score_model(
            &m_b,
            5,
            false,
            None,
            CostPreference::Performance,
            None,
            reg.get_score(&m_b.id),
        );
        assert!((s_a0 - s_b0).abs() < 1e-9);

        // Candidate B wins 3 consecutive Arena comparisons against Candidate A
        for _ in 0..3 {
            let (qa, qb) = reg.record_arena_preference_outcome(&m_a.id, &m_b.id, 0.0);
            assert!(
                qb > qa,
                "winner quality {qb} must exceed loser quality {qa}"
            );
        }

        let s_a1 = auto_score_model(
            &m_a,
            5,
            false,
            None,
            CostPreference::Performance,
            None,
            reg.get_score(&m_a.id),
        );
        let s_b1 = auto_score_model(
            &m_b,
            5,
            false,
            None,
            CostPreference::Performance,
            None,
            reg.get_score(&m_b.id),
        );
        assert!(
            s_b1 > s_a1,
            "Arena winner must outscore loser in auto_score_model ({s_b1} vs {s_a1})"
        );

        let arm_a = reg.arm_stats_snapshot().get(&m_a.id).copied().unwrap();
        let arm_b = reg.arm_stats_snapshot().get(&m_b.id).copied().unwrap();
        assert!(arm_b.0 > arm_a.0, "winner arm successes must exceed loser");
        assert!(arm_a.1 > arm_b.1, "loser arm failures must exceed winner");

        let mut engine = ModelSelectionEngine::new(Some(42));
        let picked = engine
            .pick_with_auto_score_thompson(
                &[m_a, m_b.clone()],
                TaskCategory::CodeGen,
                5,
                false,
                None,
                CostPreference::Performance,
                None,
                reg.arm_stats_snapshot(),
                reg.scoreboard_snapshot(),
                0,
            )
            .expect("must pick a candidate");
        assert_eq!(picked.id, m_b.id);
    }
}
