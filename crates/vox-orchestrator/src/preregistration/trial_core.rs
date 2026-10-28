//! Pure L2 domain logic for the Research Trial Flywheel (Phases 07, 08, 09, 10, 11, 42).
//!
//! Covers:
//! - **Phase 07 (`TRIAL-01..05`)**: [`CampaignPreregV1`], [`SignedCampaignPrereg`],
//!   [`VerifiedCampaign`] (sealed type-system witness constructible only via [`verify_campaign`]),
//!   [`SignerAllowlist`], [`SpendReservationLedger`], [`GroupSequentialGate`], and
//!   [`EnvironmentFingerprint`].
//! - **Phase 08 (`BENCH-01`, `BENCH-02`)**: [`PairedItemScore`], [`paired_bootstrap_ci`],
//!   [`paired_sign_test_p_value`], [`benjamini_hochberg_adjust`], and
//!   [`validate_paired_arm_assignment`].
//! - **Phase 09 (`MEM-01..03`)**: [`FrozenKbSnapshotManifest`], [`verify_snapshot_immutability`],
//!   [`MemoryAblationCondition`], and [`compute_memory_utility_delta`].
//! - **Phase 10 (`TRAIN-01..03`)**: [`audit_preference_pair`], [`validate_training_provenance`],
//!   and [`evaluate_two_campaign_promotion`].
//! - **Phase 11 (`VERIF-01..03`)**: [`SignedEvidenceBundle`], [`verify_evidence_bundle_offline`],
//!   [`detect_prompt_contamination`], and [`calibrate_type_i_error`].
//! - **Phase 42 (`GRAD-01`, `GRAD-02`)**: [`evaluate_flywheel_graduation`].

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use vox_crypto::facades::{
    SigningKey, secure_hash, sign, to_verifying_key, verify, verifying_key_from_bytes,
    verifying_key_to_bytes,
};

use super::trusty_uri::{CanonicalError, canonical_json_bytes, compute_value_trusty_uri};

fn blake3_hex(bytes: &[u8]) -> String {
    hex::encode(secure_hash(bytes))
}

/// Errors returned by Research Trial Flywheel gates.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TrialGateError {
    #[error("canonical serialization error: {0}")]
    Canonical(#[from] CanonicalError),
    #[error("invalid hex encoding for {field}: {detail}")]
    InvalidHex { field: &'static str, detail: String },
    #[error("cryptographic error: {0}")]
    Crypto(String),
    #[error("signer public key '{pubkey_hex}' is not in the campaign allowlist")]
    SignerNotAllowlisted { pubkey_hex: String },
    #[error("trusty URI mismatch: expected '{expected}', got '{actual}'")]
    TrustyUriMismatch { expected: String, actual: String },
    #[error("signature verification failed for campaign '{campaign_id}'")]
    SignatureInvalid { campaign_id: String },
    #[error("invalid campaign specification: {0}")]
    InvalidCampaignSpec(String),
    #[error(
        "budget cap exceeded for campaign '{campaign_id}': requested {requested_usd:.4} USD, remaining {remaining_usd:.4} USD"
    )]
    BudgetExceeded {
        campaign_id: String,
        requested_usd: f64,
        remaining_usd: f64,
    },
    #[error("environment fingerprint incomplete: missing {field}")]
    IncompleteFingerprint { field: &'static str },
    #[error("KB snapshot mutated during sealed campaign: {detail}")]
    KbSnapshotMutated { detail: String },
    #[error("paired arm assignment violation: {0}")]
    PairedAssignmentViolation(String),
    #[error("evidence bundle sample digest mismatch on item '{item_id}'")]
    SampleDigestMismatch { item_id: String },
}

// ============================================================================
// Phase 07: Pre-Registration Gate, Spend Reservation, Group-Sequential & Env
// ============================================================================

/// Pinned execution environment fingerprint (`TRIAL-04`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvironmentFingerprint {
    pub git_commit: String,
    pub cargo_lock_blake3: String,
    pub uv_lock_blake3: Option<String>,
    pub rustc_version: String,
    pub host_triple: String,
}

impl EnvironmentFingerprint {
    /// Validate that all required environment pins are non-empty and well-formed.
    pub fn validate_pinned(&self) -> Result<(), TrialGateError> {
        if self.git_commit.trim().len() < 7 {
            return Err(TrialGateError::IncompleteFingerprint {
                field: "git_commit",
            });
        }
        if self.cargo_lock_blake3.trim().is_empty() {
            return Err(TrialGateError::IncompleteFingerprint {
                field: "cargo_lock_blake3",
            });
        }
        if self.rustc_version.trim().is_empty() {
            return Err(TrialGateError::IncompleteFingerprint {
                field: "rustc_version",
            });
        }
        if self.host_triple.trim().is_empty() {
            return Err(TrialGateError::IncompleteFingerprint {
                field: "host_triple",
            });
        }
        Ok(())
    }
}

/// Group-sequential or Bayesian stopping rule kind (`TRIAL-03`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GroupSequentialBoundaryKind {
    FixedN,
    GroupSequentialObrienFleming,
    GroupSequentialPocock,
    BayesianRope,
}

impl GroupSequentialBoundaryKind {
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::FixedN => "fixed_n",
            Self::GroupSequentialObrienFleming => "group_sequential_obrien_fleming",
            Self::GroupSequentialPocock => "group_sequential_pocock",
            Self::BayesianRope => "bayesian_rope",
        }
    }
}

/// Canonical pre-registration payload for a Tier-A trial campaign (`TRIAL-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CampaignPreregV1 {
    pub campaign_id: String,
    pub hypothesis: String,
    pub arms: Vec<String>,
    pub primary_metric_name: String,
    pub target_sample_size_per_arm: usize,
    pub max_budget_usd: f64,
    pub alpha: f64,
    pub max_interim_looks: usize,
    pub stopping_rule_kind: GroupSequentialBoundaryKind,
    pub kb_snapshot_manifest_digest: Option<String>,
    pub environment: EnvironmentFingerprint,
}

impl CampaignPreregV1 {
    /// Validate invariants before signing or verifying.
    pub fn validate(&self) -> Result<(), TrialGateError> {
        if self.campaign_id.trim().is_empty() {
            return Err(TrialGateError::InvalidCampaignSpec(
                "campaign_id must not be empty".to_string(),
            ));
        }
        if self.hypothesis.trim().is_empty() {
            return Err(TrialGateError::InvalidCampaignSpec(
                "hypothesis must not be empty".to_string(),
            ));
        }
        if self.arms.len() < 2 {
            return Err(TrialGateError::InvalidCampaignSpec(
                "campaign must declare at least 2 arms".to_string(),
            ));
        }
        if self.target_sample_size_per_arm == 0 {
            return Err(TrialGateError::InvalidCampaignSpec(
                "target_sample_size_per_arm must be > 0".to_string(),
            ));
        }
        if !self.max_budget_usd.is_finite() || self.max_budget_usd <= 0.0 {
            return Err(TrialGateError::InvalidCampaignSpec(
                "max_budget_usd must be finite and > 0.0".to_string(),
            ));
        }
        if !self.alpha.is_finite() || self.alpha <= 0.0 || self.alpha >= 0.5 {
            return Err(TrialGateError::InvalidCampaignSpec(
                "alpha must be in (0.0, 0.5)".to_string(),
            ));
        }
        if self.max_interim_looks == 0 {
            return Err(TrialGateError::InvalidCampaignSpec(
                "max_interim_looks must be >= 1".to_string(),
            ));
        }
        self.environment.validate_pinned()?;
        Ok(())
    }
}

/// Signed envelope wrapping [`CampaignPreregV1`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedCampaignPrereg {
    pub prereg: CampaignPreregV1,
    pub trusty_uri: String,
    pub signature_hex: String,
    pub signer_pubkey_hex: String,
}

/// Allowlist of Ed25519 verifying keys (lowercase hex, 64 chars) authorized to seal campaigns.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignerAllowlist {
    allowed_pubkeys_hex: BTreeSet<String>,
}

impl SignerAllowlist {
    #[must_use]
    pub fn new<I, S>(keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        Self {
            allowed_pubkeys_hex: keys
                .into_iter()
                .map(|s| s.as_ref().trim().to_ascii_lowercase())
                .collect(),
        }
    }

    #[must_use]
    pub fn is_allowed(&self, pubkey_hex: &str) -> bool {
        self.allowed_pubkeys_hex
            .contains(&pubkey_hex.trim().to_ascii_lowercase())
    }
}

/// Unforgeable witness proving a [`CampaignPreregV1`] has passed allowlist, Trusty URI,
/// and Ed25519 signature verification (`TRIAL-01`).
///
/// The inner field is private to this module; external callers can only obtain a
/// [`VerifiedCampaign`] via [`verify_campaign`].
#[derive(Debug, Clone, PartialEq)]
pub struct VerifiedCampaign {
    signed: SignedCampaignPrereg,
}

impl VerifiedCampaign {
    #[must_use]
    pub fn prereg(&self) -> &CampaignPreregV1 {
        &self.signed.prereg
    }

    #[must_use]
    pub fn trusty_uri(&self) -> &str {
        &self.signed.trusty_uri
    }

    #[must_use]
    pub fn signer_pubkey_hex(&self) -> &str {
        &self.signed.signer_pubkey_hex
    }

    #[must_use]
    pub fn signed_envelope(&self) -> &SignedCampaignPrereg {
        &self.signed
    }
}

/// Sign a [`CampaignPreregV1`] with an Ed25519 [`SigningKey`] and compute its self-verifying
/// Trusty URI (`RA...`).
pub fn sign_campaign(
    prereg: CampaignPreregV1,
    signing_key: &SigningKey,
) -> Result<SignedCampaignPrereg, TrialGateError> {
    prereg.validate()?;
    let canonical_bytes = canonical_json_bytes(&prereg)?;
    let trusty_uri = compute_value_trusty_uri(&prereg)?;
    let sig_bytes = sign(signing_key, &canonical_bytes);
    let vk = to_verifying_key(signing_key);
    let pubkey_bytes = verifying_key_to_bytes(&vk);
    Ok(SignedCampaignPrereg {
        prereg,
        trusty_uri,
        signature_hex: hex::encode(sig_bytes),
        signer_pubkey_hex: hex::encode(pubkey_bytes),
    })
}

/// Verify a [`SignedCampaignPrereg`] against a [`SignerAllowlist`], returning the sealed
/// [`VerifiedCampaign`] witness on success (`TRIAL-01`).
pub fn verify_campaign(
    signed: &SignedCampaignPrereg,
    allowlist: &SignerAllowlist,
) -> Result<VerifiedCampaign, TrialGateError> {
    signed.prereg.validate()?;
    let norm_pub = signed.signer_pubkey_hex.trim().to_ascii_lowercase();
    if !allowlist.is_allowed(&norm_pub) {
        return Err(TrialGateError::SignerNotAllowlisted {
            pubkey_hex: norm_pub,
        });
    }

    let expected_uri = compute_value_trusty_uri(&signed.prereg)?;
    if signed.trusty_uri != expected_uri {
        return Err(TrialGateError::TrustyUriMismatch {
            expected: expected_uri,
            actual: signed.trusty_uri.clone(),
        });
    }

    let pubkey_vec = hex::decode(&norm_pub).map_err(|e| TrialGateError::InvalidHex {
        field: "signer_pubkey_hex",
        detail: e.to_string(),
    })?;
    let pubkey_arr: [u8; 32] = pubkey_vec
        .try_into()
        .map_err(|_| TrialGateError::InvalidHex {
            field: "signer_pubkey_hex",
            detail: "expected 32-byte Ed25519 public key".to_string(),
        })?;
    let verifying_key = verifying_key_from_bytes(&pubkey_arr).map_err(TrialGateError::Crypto)?;

    let sig_vec =
        hex::decode(signed.signature_hex.trim()).map_err(|e| TrialGateError::InvalidHex {
            field: "signature_hex",
            detail: e.to_string(),
        })?;
    let sig_arr: [u8; 64] = sig_vec.try_into().map_err(|_| TrialGateError::InvalidHex {
        field: "signature_hex",
        detail: "expected 64-byte Ed25519 signature".to_string(),
    })?;

    let canonical_bytes = canonical_json_bytes(&signed.prereg)?;
    if !verify(&verifying_key, &canonical_bytes, &sig_arr) {
        return Err(TrialGateError::SignatureInvalid {
            campaign_id: signed.prereg.campaign_id.clone(),
        });
    }

    Ok(VerifiedCampaign {
        signed: signed.clone(),
    })
}

/// Pre-dispatch spend reservation ledger (`TRIAL-02`).
///
/// Guarantees that concurrent or sequential trial calls cannot exceed `max_budget_usd` by
/// reserving estimated spend before dispatch and settling/releasing on completion.
#[derive(Debug, Clone, PartialEq)]
pub struct SpendReservationLedger {
    campaign_id: String,
    max_budget_usd: f64,
    settled_usd: f64,
    active_reservations: BTreeMap<String, f64>,
}

impl SpendReservationLedger {
    #[must_use]
    pub fn new(campaign: &VerifiedCampaign) -> Self {
        Self {
            campaign_id: campaign.prereg().campaign_id.clone(),
            max_budget_usd: campaign.prereg().max_budget_usd,
            settled_usd: 0.0,
            active_reservations: BTreeMap::new(),
        }
    }

    #[must_use]
    pub fn reserved_in_flight_usd(&self) -> f64 {
        self.active_reservations.values().sum()
    }

    #[must_use]
    pub fn settled_usd(&self) -> f64 {
        self.settled_usd
    }

    #[must_use]
    pub fn remaining_budget_usd(&self) -> f64 {
        (self.max_budget_usd - self.settled_usd - self.reserved_in_flight_usd()).max(0.0)
    }

    /// Reserve `estimated_usd` for `call_id` before dispatching an LLM call.
    pub fn reserve(&mut self, call_id: &str, estimated_usd: f64) -> Result<(), TrialGateError> {
        if !estimated_usd.is_finite() || estimated_usd < 0.0 {
            return Err(TrialGateError::InvalidCampaignSpec(
                "estimated_usd must be finite and >= 0".to_string(),
            ));
        }
        let committed = self.settled_usd + self.reserved_in_flight_usd();
        if committed + estimated_usd > self.max_budget_usd + 1e-12 {
            return Err(TrialGateError::BudgetExceeded {
                campaign_id: self.campaign_id.clone(),
                requested_usd: estimated_usd,
                remaining_usd: (self.max_budget_usd - committed).max(0.0),
            });
        }
        self.active_reservations
            .insert(call_id.to_string(), estimated_usd);
        Ok(())
    }

    /// Settle an in-flight reservation with actual post-call spend.
    pub fn settle(&mut self, call_id: &str, actual_usd: f64) -> Result<(), TrialGateError> {
        if !actual_usd.is_finite() || actual_usd < 0.0 {
            return Err(TrialGateError::InvalidCampaignSpec(
                "actual_usd must be finite and >= 0".to_string(),
            ));
        }
        let _ = self.active_reservations.remove(call_id);
        self.settled_usd += actual_usd;
        if self.settled_usd + self.reserved_in_flight_usd() > self.max_budget_usd + 1e-12 {
            return Err(TrialGateError::BudgetExceeded {
                campaign_id: self.campaign_id.clone(),
                requested_usd: actual_usd,
                remaining_usd: 0.0,
            });
        }
        Ok(())
    }

    /// Release an unused reservation (e.g. when a call fails before billing).
    pub fn release(&mut self, call_id: &str) {
        let _ = self.active_reservations.remove(call_id);
    }
}

/// Group-sequential interim decision (`TRIAL-03`).
#[derive(Debug, Clone, PartialEq)]
pub enum InterimStopDecision {
    /// Continue collecting samples; critical boundary not crossed.
    Continue {
        look_index: usize,
        information_fraction: f64,
        critical_z: f64,
        observed_z: f64,
    },
    /// Stop early for efficacy ($|Z| \ge z_{\text{crit}}$ at interim or final look).
    StopForEfficacy {
        look_index: usize,
        information_fraction: f64,
        critical_z: f64,
        observed_z: f64,
    },
    /// Stop at final look without crossing efficacy boundary (null retained).
    StopFinalNull {
        look_index: usize,
        critical_z: f64,
        observed_z: f64,
    },
}

/// Group-sequential alpha-spending gate (`TRIAL-03`).
///
/// Supports O'Brien-Fleming ($z_k = c_{\text{OBF}}(K,\alpha) / \sqrt{t_k}$),
/// Pocock ($z_k = c_{\text{Pocock}}(K,\alpha)$), and Fixed-N (only stops at look $K$).
#[derive(Debug, Clone, PartialEq)]
pub struct GroupSequentialGate {
    pub boundary_kind: GroupSequentialBoundaryKind,
    pub max_looks: usize,
    pub alpha: f64,
}

impl GroupSequentialGate {
    /// Standard normal two-sided critical value $z_{1 - \alpha/2}$ via Abramowitz & Stegun 26.2.23.
    #[must_use]
    pub fn normal_two_sided_z(alpha: f64) -> f64 {
        let half_alpha = (alpha.clamp(1e-9, 0.4999)) * 0.5;
        let t = (-2.0 * half_alpha.ln()).sqrt();
        let num = 2.515_517 + 0.802_853 * t + 0.010_328 * t * t;
        let den = 1.0 + 1.432_788 * t + 0.189_269 * t * t + 0.001_308 * t * t * t;
        t - (num / den)
    }

    /// Critical $|Z|$ threshold at 1-based look `look_index` (`1..=max_looks`).
    #[must_use]
    pub fn critical_z_at_look(&self, look_index: usize) -> f64 {
        let k_max = self.max_looks.max(1);
        let k = look_index.clamp(1, k_max);
        let t = (k as f64) / (k_max as f64);
        let z_base = Self::normal_two_sided_z(self.alpha);
        match self.boundary_kind {
            GroupSequentialBoundaryKind::FixedN => {
                if k < k_max {
                    f64::INFINITY
                } else {
                    z_base
                }
            }
            GroupSequentialBoundaryKind::GroupSequentialObrienFleming => {
                // O'Brien-Fleming inflation factor C_OBF(K) * z_{1-alpha/2} / sqrt(t)
                let c_obf = 1.0 + 0.018 * ((k_max.saturating_sub(1)) as f64).sqrt();
                (z_base * c_obf) / t.sqrt()
            }
            GroupSequentialBoundaryKind::GroupSequentialPocock => {
                // Pocock constant boundary across all K looks
                let c_pocock = 1.0 + 0.115 * (k_max as f64).ln();
                z_base * c_pocock
            }
            GroupSequentialBoundaryKind::BayesianRope => z_base,
        }
    }

    /// Evaluate an interim or final look with `observed_z`.
    #[must_use]
    pub fn evaluate_interim_look(&self, look_index: usize, observed_z: f64) -> InterimStopDecision {
        let k_max = self.max_looks.max(1);
        let k = look_index.clamp(1, k_max);
        let information_fraction = (k as f64) / (k_max as f64);
        let critical_z = self.critical_z_at_look(k);
        if observed_z.abs() >= critical_z {
            InterimStopDecision::StopForEfficacy {
                look_index: k,
                information_fraction,
                critical_z,
                observed_z,
            }
        } else if k >= k_max {
            InterimStopDecision::StopFinalNull {
                look_index: k,
                critical_z,
                observed_z,
            }
        } else {
            InterimStopDecision::Continue {
                look_index: k,
                information_fraction,
                critical_z,
                observed_z,
            }
        }
    }
}

// ============================================================================
// Phase 08: Paired Race Harness & Benjamini-Hochberg FDR (BENCH-01, BENCH-02)
// ============================================================================

/// Single paired item observation across two competing arms (`BENCH-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairedItemScore {
    pub item_id: String,
    pub paired_group_id: String,
    pub seed: i64,
    pub arm_a_score: f64,
    pub arm_b_score: f64,
}

impl PairedItemScore {
    #[must_use]
    pub fn diff(&self) -> f64 {
        self.arm_a_score - self.arm_b_score
    }
}

/// Summary of paired bootstrap and sign-test evaluation (`BENCH-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PairedBootstrapResult {
    pub n_items: usize,
    pub mean_diff: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub sign_test_p_value: f64,
}

/// Validate that all paired item observations share non-empty item IDs, matching seeds,
/// and finite scores (`BENCH-01`).
pub fn validate_paired_arm_assignment(items: &[PairedItemScore]) -> Result<(), TrialGateError> {
    if items.is_empty() {
        return Err(TrialGateError::PairedAssignmentViolation(
            "paired item slice must not be empty".to_string(),
        ));
    }
    let mut seen_items = BTreeSet::new();
    for item in items {
        if item.item_id.trim().is_empty() || item.paired_group_id.trim().is_empty() {
            return Err(TrialGateError::PairedAssignmentViolation(
                "item_id and paired_group_id must be non-empty".to_string(),
            ));
        }
        if !item.arm_a_score.is_finite() || !item.arm_b_score.is_finite() {
            return Err(TrialGateError::PairedAssignmentViolation(format!(
                "non-finite score on item '{}'",
                item.item_id
            )));
        }
        if !seen_items.insert(item.item_id.clone()) {
            return Err(TrialGateError::PairedAssignmentViolation(format!(
                "duplicate item_id '{}' in paired group",
                item.item_id
            )));
        }
    }
    Ok(())
}

/// Compute a deterministic seeded paired bootstrap confidence interval and two-sided sign test
/// $p$-value over `items` (`BENCH-02`).
pub fn paired_bootstrap_ci(
    items: &[PairedItemScore],
    resamples: usize,
    alpha: f64,
    seed: u64,
) -> Result<PairedBootstrapResult, TrialGateError> {
    validate_paired_arm_assignment(items)?;
    let n = items.len();
    let diffs: Vec<f64> = items.iter().map(PairedItemScore::diff).collect();
    let mean_diff = diffs.iter().sum::<f64>() / (n as f64);

    let b_count = resamples.max(200);
    let mut boot_means = Vec::with_capacity(b_count);
    // SplitMix64 PRNG for deterministic, reproducible resampling without external crate deps.
    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut next_u64 = || -> u64 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };

    for _ in 0..b_count {
        let mut acc = 0.0;
        for _ in 0..n {
            let idx = (next_u64() as usize) % n;
            acc += diffs[idx];
        }
        boot_means.push(acc / (n as f64));
    }
    boot_means.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let lower_idx = (((alpha * 0.5) * (b_count as f64)).floor() as usize).min(b_count - 1);
    let upper_idx = ((((1.0 - alpha * 0.5) * (b_count as f64)).ceil() as usize).saturating_sub(1))
        .min(b_count - 1);

    let sign_test_p_value = paired_sign_test_p_value(items);
    Ok(PairedBootstrapResult {
        n_items: n,
        mean_diff,
        ci_lower: boot_means[lower_idx],
        ci_upper: boot_means[upper_idx],
        sign_test_p_value,
    })
}

/// Exact two-sided binomial sign-test $p$-value on non-zero paired differences.
#[must_use]
pub fn paired_sign_test_p_value(items: &[PairedItemScore]) -> f64 {
    let mut pos = 0usize;
    let mut neg = 0usize;
    for item in items {
        let d = item.diff();
        if d > 1e-12 {
            pos += 1;
        } else if d < -1e-12 {
            neg += 1;
        }
    }
    let n = pos + neg;
    if n == 0 {
        return 1.0;
    }
    let k = pos.min(neg);
    // Sum_{i=0..=k} C(n, i) / 2^n in log/iterative space to avoid overflow
    let mut term = 0.5_f64.powi(n as i32);
    let mut tail = term;
    for i in 0..k {
        term *= ((n - i) as f64) / ((i + 1) as f64);
        tail += term;
    }
    (2.0 * tail).min(1.0)
}

/// Benjamini-Hochberg step-up false-discovery-rate adjustment (`BENCH-02`).
///
/// Returns adjusted $q$-values in the original input order, preserving monotonicity.
#[must_use]
pub fn benjamini_hochberg_adjust(raw_p_values: &[f64]) -> Vec<f64> {
    let m = raw_p_values.len();
    if m == 0 {
        return Vec::new();
    }
    let mut indexed: Vec<(usize, f64)> = raw_p_values
        .iter()
        .copied()
        .map(|p| p.clamp(0.0, 1.0))
        .enumerate()
        .collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let mut q_sorted = vec![1.0; m];
    let mut running_min: f64 = 1.0;
    for rank_zero in (0..m).rev() {
        let rank = rank_zero + 1;
        let p = indexed[rank_zero].1;
        let candidate = (p * (m as f64) / (rank as f64)).min(1.0);
        running_min = running_min.min(candidate);
        q_sorted[rank_zero] = running_min;
    }

    let mut out = vec![1.0; m];
    for (rank_zero, (orig_idx, _)) in indexed.into_iter().enumerate() {
        out[orig_idx] = q_sorted[rank_zero];
    }
    out
}

// ============================================================================
// Phase 09: Frozen KB Snapshot & 4-Condition Memory Ablation (MEM-01..03)
// ============================================================================

/// Single KB document digest entry inside a [`FrozenKbSnapshotManifest`] (`MEM-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KbDocDigestEntry {
    pub doc_uri: String,
    pub content_blake3: String,
    pub trust_tier: String,
}

/// Frozen KB snapshot manifest pinned before sealing a campaign (`MEM-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FrozenKbSnapshotManifest {
    pub entries: Vec<KbDocDigestEntry>,
    pub manifest_digest_blake3: String,
}

impl FrozenKbSnapshotManifest {
    /// Build a canonical sorted snapshot manifest and compute its BLAKE3 digest.
    #[must_use]
    pub fn from_entries(mut entries: Vec<KbDocDigestEntry>) -> Self {
        entries.sort_by(|a, b| a.doc_uri.cmp(&b.doc_uri));
        let mut canonical = String::new();
        for e in &entries {
            canonical.push_str(&e.doc_uri);
            canonical.push('\t');
            canonical.push_str(&e.content_blake3);
            canonical.push('\t');
            canonical.push_str(&e.trust_tier);
            canonical.push('\n');
        }
        let digest = blake3_hex(canonical.as_bytes());
        Self {
            entries,
            manifest_digest_blake3: digest,
        }
    }
}

/// Verify that a live KB state matches the frozen manifest pinned at campaign seal time (`MEM-01`).
pub fn verify_snapshot_immutability(
    frozen: &FrozenKbSnapshotManifest,
    live_entries: Vec<KbDocDigestEntry>,
) -> Result<(), TrialGateError> {
    let live_manifest = FrozenKbSnapshotManifest::from_entries(live_entries);
    if live_manifest.manifest_digest_blake3 != frozen.manifest_digest_blake3 {
        return Err(TrialGateError::KbSnapshotMutated {
            detail: format!(
                "expected manifest digest '{}', got '{}' (frozen={} rows, live={} rows)",
                frozen.manifest_digest_blake3,
                live_manifest.manifest_digest_blake3,
                frozen.entries.len(),
                live_manifest.entries.len()
            ),
        });
    }
    Ok(())
}

/// 4-condition memory ablation protocol (`MEM-02`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAblationCondition {
    /// Zero memory or retrieval context.
    Off,
    /// Ephemeral turn-local session buffer only; no cross-session persistence or KB retrieval.
    SessionOnly,
    /// Includes unverified persistent agent notes alongside session buffer, without curated RAG verification.
    PersistentUnverified,
    /// Full curated hybrid RAG + verified epistemic graph.
    FullRagHybrid,
}

impl MemoryAblationCondition {
    #[must_use]
    pub const fn as_db_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::SessionOnly => "session_only",
            Self::PersistentUnverified => "persistent_unverified",
            Self::FullRagHybrid => "full_rag_hybrid",
        }
    }

    #[must_use]
    pub const fn allows_session_read(self) -> bool {
        !matches!(self, Self::Off)
    }

    #[must_use]
    pub const fn allows_persistent_unverified(self) -> bool {
        matches!(self, Self::PersistentUnverified)
    }

    #[must_use]
    pub const fn allows_curated_rag(self) -> bool {
        matches!(self, Self::FullRagHybrid)
    }
}

/// Marginal utility breakdown across the 4 memory ablation conditions (`MEM-03`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct MemoryUtilityAttribution {
    pub session_gain_over_off: f64,
    pub unverified_drift_over_session: f64,
    pub verified_rag_gain_over_unverified: f64,
    pub total_rag_gain_over_off: f64,
}

/// Compute the marginal utility attribution across the 4 ablation arm mean scores (`MEM-03`).
#[must_use]
pub fn compute_memory_utility_delta(
    off_mean: f64,
    session_only_mean: f64,
    persistent_unverified_mean: f64,
    full_rag_hybrid_mean: f64,
) -> MemoryUtilityAttribution {
    MemoryUtilityAttribution {
        session_gain_over_off: session_only_mean - off_mean,
        unverified_drift_over_session: persistent_unverified_mean - session_only_mean,
        verified_rag_gain_over_unverified: full_rag_hybrid_mean - persistent_unverified_mean,
        total_rag_gain_over_off: full_rag_hybrid_mean - off_mean,
    }
}

// ============================================================================
// Phase 10: Guarded Feedback Loop & Two-Campaign Promotion (TRAIN-01..03)
// ============================================================================

/// Candidate preference/training pair audited before admission to MENS training (`TRAIN-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PreferencePairSample {
    pub sample_id: String,
    pub chosen_text: String,
    pub rejected_text: String,
    pub chosen_passed_deterministic_gate: bool,
    pub rejected_passed_deterministic_gate: bool,
}

/// Result of auditing a candidate training pair for reward hacking (`TRAIN-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RewardAuditVerdict {
    Accepted,
    Rejected { reason: String },
}

/// Audit a candidate preference pair for reward-hacking signatures (`TRAIN-01`).
#[must_use]
pub fn audit_preference_pair(sample: &PreferencePairSample) -> RewardAuditVerdict {
    let chosen = sample.chosen_text.trim();
    let rejected = sample.rejected_text.trim();
    if chosen.is_empty() || chosen == rejected {
        return RewardAuditVerdict::Rejected {
            reason: "empty_or_identical_pair".to_string(),
        };
    }
    if !sample.chosen_passed_deterministic_gate {
        return RewardAuditVerdict::Rejected {
            reason: "chosen_failed_deterministic_verifier".to_string(),
        };
    }
    const CHEAT_MARKERS: &[&str] = &[
        "#[ignore]",
        "#[allow(unreachable_code)]",
        "todo!()",
        "unimplemented!()",
        ".skip(",
        "vox-deprecated-since",
    ];
    for marker in CHEAT_MARKERS {
        if chosen.contains(marker) && !rejected.contains(marker) {
            return RewardAuditVerdict::Rejected {
                reason: "chosen_introduces_test_bypass_or_stub_marker".to_string(),
            };
        }
    }
    // Length-inflation guard: if both pass deterministic gates and chosen is >3x longer without
    // fixing a failure, reject verbosity gaming.
    if sample.rejected_passed_deterministic_gate && chosen.len() > rejected.len().saturating_mul(3)
    {
        return RewardAuditVerdict::Rejected {
            reason: "verbosity_inflation_without_correctness_gain".to_string(),
        };
    }
    RewardAuditVerdict::Accepted
}

/// Provenance lineage tag attached to every trial-derived training example (`TRAIN-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrainingProvenanceTag {
    pub campaign_id: String,
    pub trusty_uri: String,
    pub run_id: String,
    pub sample_item_id: String,
    pub kb_snapshot_manifest_digest: String,
}

/// Validate that a [`TrainingProvenanceTag`] has complete, non-empty lineage links (`TRAIN-02`).
pub fn validate_training_provenance(tag: &TrainingProvenanceTag) -> Result<(), TrialGateError> {
    if tag.campaign_id.trim().is_empty()
        || !tag.trusty_uri.starts_with("RA")
        || tag.run_id.trim().is_empty()
        || tag.sample_item_id.trim().is_empty()
        || tag.kb_snapshot_manifest_digest.trim().is_empty()
    {
        return Err(TrialGateError::InvalidCampaignSpec(
            "incomplete training provenance tag (requires campaign_id, RA trusty_uri, run_id, sample_item_id, kb_snapshot_manifest_digest)".to_string(),
        ));
    }
    Ok(())
}

/// Evidence from a single completed campaign submitted to the two-campaign promotion gate (`TRAIN-03`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CampaignPromotionEvidence {
    pub campaign_id: String,
    pub trusty_uri: String,
    pub seeds: Vec<i64>,
    pub lower_ci_95: f64,
    pub bh_fdr_q: f64,
    pub guard_regressions: usize,
}

/// Decision returned by [`evaluate_two_campaign_promotion`] (`TRAIN-03`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PromotionVerdict {
    Promote,
    Hold { reason: String },
}

/// Require at least two independent sealed campaigns with disjoint seeds, positive 95% lower CI,
/// BH-FDR $q \le 0.05$, and zero regressions on held-out guard tasks (`TRAIN-03`).
#[must_use]
pub fn evaluate_two_campaign_promotion(
    campaigns: &[CampaignPromotionEvidence],
) -> PromotionVerdict {
    if campaigns.len() < 2 {
        return PromotionVerdict::Hold {
            reason: format!(
                "requires >= 2 independent campaigns, got {}",
                campaigns.len()
            ),
        };
    }
    let mut seen_ids = BTreeSet::new();
    let mut seen_uris = BTreeSet::new();
    let mut seen_seeds = BTreeSet::new();
    for c in campaigns {
        if !c.trusty_uri.starts_with("RA") {
            return PromotionVerdict::Hold {
                reason: format!("campaign '{}' has invalid trusty_uri", c.campaign_id),
            };
        }
        if !seen_ids.insert(c.campaign_id.clone()) {
            return PromotionVerdict::Hold {
                reason: format!("duplicate campaign_id '{}'", c.campaign_id),
            };
        }
        if !seen_uris.insert(c.trusty_uri.clone()) {
            return PromotionVerdict::Hold {
                reason: format!("duplicate trusty_uri '{}'", c.trusty_uri),
            };
        }
        if c.seeds.is_empty() {
            return PromotionVerdict::Hold {
                reason: format!("campaign '{}' has no recorded seeds", c.campaign_id),
            };
        }
        for &seed in &c.seeds {
            if !seen_seeds.insert(seed) {
                return PromotionVerdict::Hold {
                    reason: format!("non-disjoint seed {seed} reused across promotion campaigns"),
                };
            }
        }
        if c.lower_ci_95 <= 0.0 {
            return PromotionVerdict::Hold {
                reason: format!(
                    "campaign '{}' 95% lower CI ({:.4}) is not > 0.0",
                    c.campaign_id, c.lower_ci_95
                ),
            };
        }
        if c.bh_fdr_q > 0.05 {
            return PromotionVerdict::Hold {
                reason: format!(
                    "campaign '{}' BH-FDR q-value ({:.4}) exceeds 0.05",
                    c.campaign_id, c.bh_fdr_q
                ),
            };
        }
        if c.guard_regressions > 0 {
            return PromotionVerdict::Hold {
                reason: format!(
                    "campaign '{}' had {} guard regressions",
                    c.campaign_id, c.guard_regressions
                ),
            };
        }
    }
    PromotionVerdict::Promote
}

// ============================================================================
// Phase 11: Offline Verifier, Contamination Audit & Null Calibration (VERIF-01..03)
// ============================================================================

/// Digest record for a single sample inside an offline evidence bundle (`VERIF-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSampleDigest {
    pub item_id: String,
    pub prompt_bytes_utf8: String,
    pub output_bytes_utf8: String,
    pub prompt_blake3: String,
    pub output_blake3: String,
}

/// Offline-verifiable evidence bundle payload (`VERIF-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceBundlePayload {
    pub signed_prereg: SignedCampaignPrereg,
    pub samples: Vec<EvidenceSampleDigest>,
    pub paired_summary: PairedBootstrapResult,
}

/// Signed evidence bundle verifiable without network or database access (`VERIF-01`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedEvidenceBundle {
    pub payload: EvidenceBundlePayload,
    pub bundle_trusty_uri: String,
    pub bundle_signature_hex: String,
    pub signer_pubkey_hex: String,
}

/// Sign an [`EvidenceBundlePayload`] with an Ed25519 [`SigningKey`] (`VERIF-01`).
pub fn sign_evidence_bundle(
    payload: EvidenceBundlePayload,
    signing_key: &SigningKey,
) -> Result<SignedEvidenceBundle, TrialGateError> {
    let bytes = canonical_json_bytes(&payload)?;
    let bundle_trusty_uri = compute_value_trusty_uri(&payload)?;
    let sig = sign(signing_key, &bytes);
    let vk = to_verifying_key(signing_key);
    Ok(SignedEvidenceBundle {
        payload,
        bundle_trusty_uri,
        bundle_signature_hex: hex::encode(sig),
        signer_pubkey_hex: hex::encode(verifying_key_to_bytes(&vk)),
    })
}

/// Pure offline verification of a [`SignedEvidenceBundle`]: verifies the inner campaign prereg,
/// every sample's BLAKE3 prompt/output digest, the bundle's Trusty URI, and the outer Ed25519
/// signature (`VERIF-01`).
pub fn verify_evidence_bundle_offline(
    bundle: &SignedEvidenceBundle,
    allowlist: &SignerAllowlist,
) -> Result<VerifiedCampaign, TrialGateError> {
    let verified_campaign = verify_campaign(&bundle.payload.signed_prereg, allowlist)?;
    for sample in &bundle.payload.samples {
        let actual_prompt_hash = blake3_hex(sample.prompt_bytes_utf8.as_bytes());
        if actual_prompt_hash != sample.prompt_blake3 {
            return Err(TrialGateError::SampleDigestMismatch {
                item_id: sample.item_id.clone(),
            });
        }
        let actual_output_hash = blake3_hex(sample.output_bytes_utf8.as_bytes());
        if actual_output_hash != sample.output_blake3 {
            return Err(TrialGateError::SampleDigestMismatch {
                item_id: sample.item_id.clone(),
            });
        }
    }

    let expected_bundle_uri = compute_value_trusty_uri(&bundle.payload)?;
    if bundle.bundle_trusty_uri != expected_bundle_uri {
        return Err(TrialGateError::TrustyUriMismatch {
            expected: expected_bundle_uri,
            actual: bundle.bundle_trusty_uri.clone(),
        });
    }

    let norm_pub = bundle.signer_pubkey_hex.trim().to_ascii_lowercase();
    if !allowlist.is_allowed(&norm_pub) {
        return Err(TrialGateError::SignerNotAllowlisted {
            pubkey_hex: norm_pub,
        });
    }
    let pubkey_arr: [u8; 32] = hex::decode(&norm_pub)
        .map_err(|e| TrialGateError::InvalidHex {
            field: "signer_pubkey_hex",
            detail: e.to_string(),
        })?
        .try_into()
        .map_err(|_| TrialGateError::InvalidHex {
            field: "signer_pubkey_hex",
            detail: "expected 32-byte key".to_string(),
        })?;
    let verifying_key = verifying_key_from_bytes(&pubkey_arr).map_err(TrialGateError::Crypto)?;
    let sig_arr: [u8; 64] = hex::decode(bundle.bundle_signature_hex.trim())
        .map_err(|e| TrialGateError::InvalidHex {
            field: "bundle_signature_hex",
            detail: e.to_string(),
        })?
        .try_into()
        .map_err(|_| TrialGateError::InvalidHex {
            field: "bundle_signature_hex",
            detail: "expected 64-byte signature".to_string(),
        })?;
    let canonical = canonical_json_bytes(&bundle.payload)?;
    if !verify(&verifying_key, &canonical, &sig_arr) {
        return Err(TrialGateError::SignatureInvalid {
            campaign_id: bundle.payload.signed_prereg.prereg.campaign_id.clone(),
        });
    }

    Ok(verified_campaign)
}

/// Contamination audit report between evaluation prompts and training corpus (`VERIF-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContaminationReport {
    pub exact_hash_collisions: usize,
    pub max_4gram_jaccard: f64,
    pub contaminated_eval_indices: Vec<usize>,
}

fn word_4grams(text: &str) -> BTreeSet<String> {
    let tokens: Vec<String> = text
        .split_whitespace()
        .map(|t| {
            t.chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_ascii_lowercase()
        })
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.len() < 4 {
        return tokens.into_iter().collect();
    }
    tokens.windows(4).map(|w| w.join(" ")).collect()
}

/// Detect prompt-vs-corpus leakage via exact BLAKE3 hashes and normalized word 4-gram Jaccard
/// similarity (`VERIF-02`).
#[must_use]
pub fn detect_prompt_contamination(
    eval_prompts: &[&str],
    training_docs: &[&str],
    jaccard_threshold: f64,
) -> ContaminationReport {
    let train_hashes: BTreeSet<String> = training_docs
        .iter()
        .map(|d| blake3_hex(d.trim().as_bytes()))
        .collect();
    let train_grams: Vec<BTreeSet<String>> = training_docs.iter().map(|d| word_4grams(d)).collect();

    let mut exact_hash_collisions = 0usize;
    let mut max_4gram_jaccard = 0.0_f64;
    let mut contaminated_eval_indices = Vec::new();

    for (idx, prompt) in eval_prompts.iter().enumerate() {
        let h = blake3_hex(prompt.trim().as_bytes());
        let exact = train_hashes.contains(&h);
        if exact {
            exact_hash_collisions += 1;
        }
        let p_grams = word_4grams(prompt);
        let mut best_j = 0.0_f64;
        if !p_grams.is_empty() {
            for t_grams in &train_grams {
                if t_grams.is_empty() {
                    continue;
                }
                let inter = p_grams.intersection(t_grams).count() as f64;
                let union = p_grams.union(t_grams).count() as f64;
                if union > 0.0 {
                    best_j = best_j.max(inter / union);
                }
            }
        }
        max_4gram_jaccard = max_4gram_jaccard.max(best_j);
        if exact || best_j >= jaccard_threshold {
            contaminated_eval_indices.push(idx);
        }
    }

    ContaminationReport {
        exact_hash_collisions,
        max_4gram_jaccard,
        contaminated_eval_indices,
    }
}

/// Result of synthetic null ($A = B$) Type-I error calibration (`VERIF-03`).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct NullCalibrationReport {
    pub num_simulations: usize,
    pub nominal_alpha: f64,
    pub empirical_false_positive_rate: f64,
    pub calibrated: bool,
}

/// Run `num_simulations` synthetic null ($A = B$) paired trials and verify the empirical
/// false-positive rate stays within `nominal_alpha + tolerance` (`VERIF-03`).
#[must_use]
pub fn calibrate_type_i_error(
    num_simulations: usize,
    items_per_trial: usize,
    nominal_alpha: f64,
    tolerance: f64,
    seed: u64,
) -> NullCalibrationReport {
    let sims = num_simulations.max(100);
    let n_items = items_per_trial.max(20);
    let mut state = seed ^ 0xA5A5_5A5A_F0F0_0F0F;
    let mut next_f64 = || -> f64 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        let u = z ^ (z >> 31);
        ((u >> 11) as f64) * (1.0 / ((1u64 << 53) as f64)) - 0.5
    };

    let mut false_positives = 0usize;
    for sim_idx in 0..sims {
        let items: Vec<PairedItemScore> = (0..n_items)
            .map(|i| {
                let base = 0.5 + 0.1 * next_f64();
                let noise_a = 0.05 * next_f64();
                let noise_b = 0.05 * next_f64();
                PairedItemScore {
                    item_id: format!("item-{i}"),
                    paired_group_id: format!("sim-{sim_idx}"),
                    seed: sim_idx as i64,
                    arm_a_score: base + noise_a,
                    arm_b_score: base + noise_b,
                }
            })
            .collect();
        let p = paired_sign_test_p_value(&items);
        if p < nominal_alpha {
            false_positives += 1;
        }
    }

    let empirical_fpr = (false_positives as f64) / (sims as f64);
    NullCalibrationReport {
        num_simulations: sims,
        nominal_alpha,
        empirical_false_positive_rate: empirical_fpr,
        calibrated: empirical_fpr <= nominal_alpha + tolerance,
    }
}

// ============================================================================
// Phase 42: Research Flywheel Graduation Gate (GRAD-01, GRAD-02)
// ============================================================================

/// Input checklist for Phase 42 Research Flywheel Graduation (`GRAD-01`, `GRAD-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlywheelGraduationInput {
    pub verified_campaigns_count: usize,
    pub atomic_store_failpoints_verified: bool,
    pub paired_fdr_harness_active: bool,
    pub frozen_kb_and_ablation_verified: bool,
    pub promotion_verdict: PromotionVerdict,
    pub offline_bundle_verified: bool,
    pub null_calibration: NullCalibrationReport,
    pub contamination_report: ContaminationReport,
}

/// Final readiness verdict for Phase 42 Research Flywheel Graduation (`GRAD-01`, `GRAD-02`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlywheelGraduationReport {
    pub graduated: bool,
    pub unmet_gates: Vec<String>,
}

/// Evaluate whether all 6 Research Trial Flywheel gates are satisfied (`GRAD-01`, `GRAD-02`).
#[must_use]
pub fn evaluate_flywheel_graduation(input: &FlywheelGraduationInput) -> FlywheelGraduationReport {
    let mut unmet = Vec::new();
    if input.verified_campaigns_count < 2 {
        unmet.push("G1: requires >= 2 verified sealed campaigns".to_string());
    }
    if !input.atomic_store_failpoints_verified {
        unmet.push("G2: atomic Tier-A persistence failpoint suite not verified".to_string());
    }
    if !input.paired_fdr_harness_active {
        unmet.push("G3: paired race + BH-FDR harness not active".to_string());
    }
    if !input.frozen_kb_and_ablation_verified {
        unmet.push("G4: frozen KB snapshot + 4-condition memory ablation not verified".to_string());
    }
    if let PromotionVerdict::Hold { reason } = &input.promotion_verdict {
        unmet.push(format!("G5: two-campaign promotion gate held ({reason})"));
    }
    if !input.offline_bundle_verified {
        unmet.push("G6a: offline evidence bundle verification not passed".to_string());
    }
    if !input.null_calibration.calibrated {
        unmet.push(format!(
            "G6b: synthetic null Type-I error rate ({:.4}) exceeds calibrated bound",
            input.null_calibration.empirical_false_positive_rate
        ));
    }
    if !input
        .contamination_report
        .contaminated_eval_indices
        .is_empty()
    {
        unmet.push(format!(
            "G6c: {} contaminated evaluation prompts detected",
            input.contamination_report.contaminated_eval_indices.len()
        ));
    }
    FlywheelGraduationReport {
        graduated: unmet.is_empty(),
        unmet_gates: unmet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_campaign_prereg(campaign_id: &str) -> CampaignPreregV1 {
        CampaignPreregV1 {
            campaign_id: campaign_id.to_string(),
            hypothesis: "Full RAG hybrid improves nDCG@5 over session-only by >= 0.08".to_string(),
            arms: vec!["session_only".to_string(), "full_rag_hybrid".to_string()],
            primary_metric_name: "ndcg_at_5".to_string(),
            target_sample_size_per_arm: 50,
            max_budget_usd: 10.0,
            alpha: 0.05,
            max_interim_looks: 3,
            stopping_rule_kind: GroupSequentialBoundaryKind::GroupSequentialObrienFleming,
            kb_snapshot_manifest_digest: Some("b3_snapshot_01".to_string()),
            environment: EnvironmentFingerprint {
                git_commit: "0123456789abcdef0123456789abcdef01234567".to_string(),
                cargo_lock_blake3: "cargo_lock_hash".to_string(),
                uv_lock_blake3: None,
                rustc_version: "rustc 1.88.0".to_string(),
                host_triple: "aarch64-apple-darwin".to_string(),
            },
        }
    }

    #[test]
    fn sign_and_verify_campaign_enforces_allowlist_and_tamper_detection() {
        let (key, vk) = vox_crypto::facades::generate_signing_keypair();
        let pub_hex = hex::encode(verifying_key_to_bytes(&vk));
        let allowlist = SignerAllowlist::new([pub_hex.as_str()]);

        let prereg = sample_campaign_prereg("camp-alpha");
        let signed = sign_campaign(prereg, &key).expect("sign");
        let verified = verify_campaign(&signed, &allowlist).expect("verify");
        assert_eq!(verified.prereg().campaign_id, "camp-alpha");
        assert!(verified.trusty_uri().starts_with("RA"));

        // Reject non-allowlisted key
        let empty_allowlist = SignerAllowlist::new::<[&str; 0], &str>([]);
        assert!(matches!(
            verify_campaign(&signed, &empty_allowlist),
            Err(TrialGateError::SignerNotAllowlisted { .. })
        ));

        // Reject payload tampering (Trusty URI mismatch)
        let mut tampered = signed.clone();
        tampered.prereg.max_budget_usd = 999.0;
        assert!(matches!(
            verify_campaign(&tampered, &allowlist),
            Err(TrialGateError::TrustyUriMismatch { .. })
        ));
    }

    #[test]
    fn spend_reservation_ledger_enforces_pre_dispatch_budget_cap() {
        let (key, vk) = vox_crypto::facades::generate_signing_keypair();
        let allowlist = SignerAllowlist::new([hex::encode(verifying_key_to_bytes(&vk))]);
        let signed = sign_campaign(sample_campaign_prereg("camp-budget"), &key).expect("sign");
        let verified = verify_campaign(&signed, &allowlist).expect("verify");

        let mut ledger = SpendReservationLedger::new(&verified);
        ledger.reserve("call-1", 6.0).expect("reserve 6 USD");
        ledger.reserve("call-2", 3.5).expect("reserve 3.5 USD");
        // Third reservation of 1.0 USD would exceed 10.0 USD cap while call-1 and call-2 are in flight
        assert!(matches!(
            ledger.reserve("call-3", 1.0),
            Err(TrialGateError::BudgetExceeded { .. })
        ));

        // Settle call-1 at actual cost 4.0 USD -> frees 2.0 USD
        ledger.settle("call-1", 4.0).expect("settle call-1");
        ledger
            .reserve("call-3", 1.5)
            .expect("reserve 1.5 USD after settle");
        ledger.release("call-2");
        assert!((ledger.settled_usd() - 4.0).abs() < 1e-9);
    }

    #[test]
    fn group_sequential_obrien_fleming_is_strict_early_and_converges_at_final_look() {
        let gate = GroupSequentialGate {
            boundary_kind: GroupSequentialBoundaryKind::GroupSequentialObrienFleming,
            max_looks: 3,
            alpha: 0.05,
        };
        let z1 = gate.critical_z_at_look(1);
        let z2 = gate.critical_z_at_look(2);
        let z3 = gate.critical_z_at_look(3);
        assert!(
            z1 > z2 && z2 > z3,
            "OBF boundaries must decrease: {z1}, {z2}, {z3}"
        );
        assert!(
            z1 > 3.2,
            "look 1 critical z should be conservative (>3.2), got {z1}"
        );

        // Z=2.3 at look 1 continues, but at look 3 stops for efficacy
        assert!(matches!(
            gate.evaluate_interim_look(1, 2.3),
            InterimStopDecision::Continue { .. }
        ));
        assert!(matches!(
            gate.evaluate_interim_look(3, 2.3),
            InterimStopDecision::StopForEfficacy { .. }
        ));
    }

    #[test]
    fn paired_bootstrap_and_benjamini_hochberg_fdr_behave_correctly() {
        let items: Vec<PairedItemScore> = (0..20)
            .map(|i| PairedItemScore {
                item_id: format!("item-{i}"),
                paired_group_id: "grp-1".to_string(),
                seed: 7,
                arm_a_score: 0.85,
                arm_b_score: 0.65,
            })
            .collect();
        let boot = paired_bootstrap_ci(&items, 500, 0.05, 42).expect("bootstrap");
        assert!((boot.mean_diff - 0.20).abs() < 1e-9);
        assert!(boot.ci_lower > 0.15);
        assert!(boot.sign_test_p_value < 1e-4);

        let raw_p = vec![0.001, 0.02, 0.04, 0.50];
        let q = benjamini_hochberg_adjust(&raw_p);
        assert_eq!(q.len(), 4);
        assert!(q[0] < q[1] && q[1] <= q[2] && q[2] < q[3]);
    }

    #[test]
    fn frozen_kb_snapshot_and_memory_ablation_detect_drift_and_attribution() {
        let entries = vec![
            KbDocDigestEntry {
                doc_uri: "vox://kb/doc1.md".to_string(),
                content_blake3: "h1".to_string(),
                trust_tier: "curated".to_string(),
            },
            KbDocDigestEntry {
                doc_uri: "vox://kb/doc2.md".to_string(),
                content_blake3: "h2".to_string(),
                trust_tier: "curated".to_string(),
            },
        ];
        let frozen = FrozenKbSnapshotManifest::from_entries(entries.clone());
        verify_snapshot_immutability(&frozen, entries.clone()).expect("unchanged KB");

        let mut mutated = entries;
        mutated[1].content_blake3 = "h2_modified".to_string();
        assert!(matches!(
            verify_snapshot_immutability(&frozen, mutated),
            Err(TrialGateError::KbSnapshotMutated { .. })
        ));

        assert!(!MemoryAblationCondition::Off.allows_session_read());
        assert!(MemoryAblationCondition::SessionOnly.allows_session_read());
        assert!(!MemoryAblationCondition::SessionOnly.allows_curated_rag());
        assert!(MemoryAblationCondition::FullRagHybrid.allows_curated_rag());

        let attr = compute_memory_utility_delta(0.50, 0.60, 0.55, 0.82);
        assert!((attr.session_gain_over_off - 0.10).abs() < 1e-9);
        assert!((attr.unverified_drift_over_session - (-0.05)).abs() < 1e-9);
        assert!((attr.verified_rag_gain_over_unverified - 0.27).abs() < 1e-9);
    }

    #[test]
    fn reward_audit_and_two_campaign_promotion_gate_enforce_invariants() {
        let good_pair = PreferencePairSample {
            sample_id: "s1".to_string(),
            chosen_text: "fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            rejected_text: "fn add(a: i32, b: i32) -> i32 { a - b }".to_string(),
            chosen_passed_deterministic_gate: true,
            rejected_passed_deterministic_gate: false,
        };
        assert_eq!(
            audit_preference_pair(&good_pair),
            RewardAuditVerdict::Accepted
        );

        let cheat_pair = PreferencePairSample {
            sample_id: "s2".to_string(),
            chosen_text: "#[ignore]\nfn test_it() {}".to_string(),
            rejected_text: "fn test_it() { assert!(false); }".to_string(),
            chosen_passed_deterministic_gate: true,
            rejected_passed_deterministic_gate: false,
        };
        assert!(matches!(
            audit_preference_pair(&cheat_pair),
            RewardAuditVerdict::Rejected { .. }
        ));

        let c1 = CampaignPromotionEvidence {
            campaign_id: "camp-1".to_string(),
            trusty_uri: "RAabc111".to_string(),
            seeds: vec![101, 102],
            lower_ci_95: 0.06,
            bh_fdr_q: 0.012,
            guard_regressions: 0,
        };
        let c2 = CampaignPromotionEvidence {
            campaign_id: "camp-2".to_string(),
            trusty_uri: "RAdef222".to_string(),
            seeds: vec![201, 202],
            lower_ci_95: 0.04,
            bh_fdr_q: 0.018,
            guard_regressions: 0,
        };
        assert!(matches!(
            evaluate_two_campaign_promotion(std::slice::from_ref(&c1)),
            PromotionVerdict::Hold { .. }
        ));
        assert_eq!(
            evaluate_two_campaign_promotion(&[c1, c2]),
            PromotionVerdict::Promote
        );
    }

    #[test]
    fn offline_evidence_bundle_contamination_and_graduation_end_to_end() {
        let (key, vk) = vox_crypto::facades::generate_signing_keypair();
        let allowlist = SignerAllowlist::new([hex::encode(verifying_key_to_bytes(&vk))]);
        let signed_prereg =
            sign_campaign(sample_campaign_prereg("camp-bundle"), &key).expect("sign");

        let prompt = "Explain borrow checking in Vox actors.";
        let output = "Actors isolate state without shared mutable references.";
        let sample = EvidenceSampleDigest {
            item_id: "item-1".to_string(),
            prompt_bytes_utf8: prompt.to_string(),
            output_bytes_utf8: output.to_string(),
            prompt_blake3: blake3_hex(prompt.as_bytes()),
            output_blake3: blake3_hex(output.as_bytes()),
        };
        let payload = EvidenceBundlePayload {
            signed_prereg,
            samples: vec![sample],
            paired_summary: PairedBootstrapResult {
                n_items: 20,
                mean_diff: 0.12,
                ci_lower: 0.05,
                ci_upper: 0.19,
                sign_test_p_value: 0.004,
            },
        };
        let bundle = sign_evidence_bundle(payload, &key).expect("sign bundle");
        let verified = verify_evidence_bundle_offline(&bundle, &allowlist).expect("offline verify");
        assert_eq!(verified.prereg().campaign_id, "camp-bundle");

        // Contamination check
        let clean_contam = detect_prompt_contamination(
            &["Write a parser combinator for identifiers"],
            &["Gradient descent optimizes neural weights"],
            0.5,
        );
        assert!(clean_contam.contaminated_eval_indices.is_empty());

        let leaked_contam = detect_prompt_contamination(
            &["Write a parser combinator for identifiers in Vox"],
            &["Write a parser combinator for identifiers in Vox"],
            0.5,
        );
        assert_eq!(leaked_contam.exact_hash_collisions, 1);

        // Null calibration
        let null_cal = calibrate_type_i_error(200, 30, 0.05, 0.04, 12345);
        assert!(
            null_cal.calibrated,
            "empirical FPR {} exceeded bound",
            null_cal.empirical_false_positive_rate
        );

        // Graduation check
        let grad = evaluate_flywheel_graduation(&FlywheelGraduationInput {
            verified_campaigns_count: 2,
            atomic_store_failpoints_verified: true,
            paired_fdr_harness_active: true,
            frozen_kb_and_ablation_verified: true,
            promotion_verdict: PromotionVerdict::Promote,
            offline_bundle_verified: true,
            null_calibration: null_cal,
            contamination_report: clean_contam,
        });
        assert!(grad.graduated, "unmet: {:?}", grad.unmet_gates);
    }
}
