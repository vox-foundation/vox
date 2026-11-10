//! Signed benchmark evidence bundle (`Phase 07.1` / `12-09`, `BENCH-08`).
//!
//! Wraps [`vox_eval::report::EvalCorpusReport`] arms and their [`vox_eval::compare::Comparison`]
//! in a canonical-JSON Trusty URI + Ed25519 signed envelope that can be verified offline with zero
//! network or database access.

use std::path::Path;

use anyhow::{Context, bail};
use serde::{Deserialize, Serialize};
use vox_crypto::facades::{
    SigningKey, sign, to_verifying_key, verify, verifying_key_from_bytes, verifying_key_to_bytes,
};
use vox_eval::compare::Comparison;
use vox_eval::report::EvalCorpusReport;
use vox_orchestrator::preregistration::{canonical_json_bytes, compute_value_trusty_uri};

/// Schema version for [`BenchBundle`] and [`SignedBenchBundle`].
pub const BENCH_BUNDLE_SCHEMA_VERSION: u32 = 1;

/// Unsigned benchmark evidence bundle payload (`BENCH-08`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchBundle {
    pub schema_version: u32,
    pub campaign_id: String,
    pub git_commit: String,
    pub cargo_lock_blake3: String,
    pub reports: Vec<EvalCorpusReport>,
    pub comparison: Option<Comparison>,
}

/// Signed benchmark evidence bundle envelope (`BENCH-08`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedBenchBundle {
    pub bundle: BenchBundle,
    pub trusty_uri: String,
    pub signature_hex: String,
    pub signer_pubkey_hex: String,
}

/// Sign a [`BenchBundle`] with an Ed25519 [`SigningKey`] and attach its self-verifying
/// `RA...` Trusty URI (`BENCH-08`).
pub fn sign_bundle(
    bundle: BenchBundle,
    signing_key: &SigningKey,
) -> anyhow::Result<SignedBenchBundle> {
    if bundle.schema_version != BENCH_BUNDLE_SCHEMA_VERSION {
        bail!(
            "unsupported BenchBundle schema_version {} (expected {BENCH_BUNDLE_SCHEMA_VERSION})",
            bundle.schema_version
        );
    }
    if bundle.campaign_id.trim().is_empty() {
        bail!("BenchBundle campaign_id must not be empty");
    }
    if bundle.reports.is_empty() {
        bail!("BenchBundle must contain at least one EvalCorpusReport");
    }
    let value = bundle_value(&bundle)?;
    let bytes = canonical_json_bytes(&value).context("canonicalize BenchBundle")?;
    let trusty_uri = compute_value_trusty_uri(&value).context("compute BenchBundle Trusty URI")?;
    let sig = sign(signing_key, &bytes);
    let vk = to_verifying_key(signing_key);
    Ok(SignedBenchBundle {
        bundle,
        trusty_uri,
        signature_hex: hex::encode(sig),
        signer_pubkey_hex: hex::encode(verifying_key_to_bytes(&vk)),
    })
}

/// Project a bundle to a `serde_json::Value` (which is `PartialEq`) for canonicalization; the
/// nested `vox-eval` report types do not derive `PartialEq`.
fn bundle_value(bundle: &BenchBundle) -> anyhow::Result<serde_json::Value> {
    serde_json::to_value(bundle).context("project BenchBundle to JSON value")
}

/// Verify a [`SignedBenchBundle`] offline against its embedded Trusty URI, Ed25519 signature,
/// and optional allowlist of hex public keys (`BENCH-08`).
pub fn verify_bundle(
    signed: &SignedBenchBundle,
    allowed_pubkeys_hex: Option<&[String]>,
) -> anyhow::Result<()> {
    if signed.bundle.schema_version != BENCH_BUNDLE_SCHEMA_VERSION {
        bail!(
            "unsupported BenchBundle schema_version {} (expected {BENCH_BUNDLE_SCHEMA_VERSION})",
            signed.bundle.schema_version
        );
    }
    let norm_pub = signed.signer_pubkey_hex.trim().to_ascii_lowercase();
    if let Some(allowlist) = allowed_pubkeys_hex
        && !allowlist
            .iter()
            .any(|k| k.trim().eq_ignore_ascii_case(&norm_pub))
    {
        bail!("signer public key '{norm_pub}' is not in the allowed public key list");
    }

    let value = bundle_value(&signed.bundle)?;
    let expected_uri =
        compute_value_trusty_uri(&value).context("recompute BenchBundle Trusty URI")?;
    if signed.trusty_uri != expected_uri {
        bail!(
            "BenchBundle Trusty URI mismatch: expected '{expected_uri}', got '{}'",
            signed.trusty_uri
        );
    }

    let pub_bytes: [u8; 32] = hex::decode(&norm_pub)
        .context("decode signer_pubkey_hex")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signer_pubkey_hex must decode to 32 bytes"))?;
    let verifying_key = verifying_key_from_bytes(&pub_bytes).map_err(|e| anyhow::anyhow!("{e}"))?;

    let sig_bytes: [u8; 64] = hex::decode(signed.signature_hex.trim())
        .context("decode signature_hex")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("signature_hex must decode to 64 bytes"))?;

    let canonical = canonical_json_bytes(&value).context("canonicalize BenchBundle")?;
    if !verify(&verifying_key, &canonical, &sig_bytes) {
        bail!("Ed25519 signature verification failed for BenchBundle");
    }

    Ok(())
}

/// Write a [`SignedBenchBundle`] to disk as pretty-printed JSON.
pub fn write_bundle(path: &Path, signed: &SignedBenchBundle) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(signed).context("serialize SignedBenchBundle")?;
    std::fs::write(path, json)
        .with_context(|| format!("write SignedBenchBundle to {}", path.display()))
}

/// Read and deserialize a [`SignedBenchBundle`] from disk.
pub fn read_bundle(path: &Path) -> anyhow::Result<SignedBenchBundle> {
    let text = vox_bounded_fs::read_utf8_path_capped(path)?;
    serde_json::from_str(&text)
        .with_context(|| format!("deserialize SignedBenchBundle from {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_eval::corpus_score::{AttemptOutcome, FixtureOutcome};
    use vox_eval::provenance::RunProvenance;
    use vox_eval::report::{DEFAULT_MAX_INFRA_RATE, ReportInputs};

    fn provenance() -> RunProvenance {
        let s = |v: &str| Some(v.to_string());
        RunProvenance {
            syntax_fingerprint: s("fp"),
            syntax_fingerprint_kind: s("grammar-v1"),
            cargo_version: s("cargo 1.0"),
            vox_git_sha: s("0123456789abcdef"),
            vox_binary_sha256: s("bin-sha"),
            verifier_version: s(env!("CARGO_PKG_VERSION")),
            verifier_matches_harness: Some(true),
            tier: s("smoke"),
            problem_set_hash: s("ps-hash"),
            corpus_manifest_hash: s("cm-hash"),
            prompt_template_hash: s("pt-hash"),
            condition: s("zero-shot"),
            context_hash: s("ctx-hash"),
            harness_version: s("h1"),
            model_id: s("test-model-v1"),
            provider: s("local"),
            request_date: s("2026-10-15"),
            provider_reported_models: vec![],
            mens: None,
            sampling: None,
            runner: s("local"),
            os: s("macos"),
            arch: s("aarch64"),
            verifier_timeout_secs: Some(30),
            identity: None,
            preregistration_id: s("prereg-1"),
        }
    }

    fn sample_report() -> EvalCorpusReport {
        let outcomes = (0..4)
            .map(|i| FixtureOutcome {
                fixture_id: format!("f{i:02}"),
                n: 1,
                c: 1,
                attempts: vec![AttemptOutcome {
                    compiled: true,
                    tests_passed: true,
                    cheated: false,
                    total_tokens: 10,
                    latency_ms: 5,
                    cost_usd: None,
                }],
            })
            .collect();
        EvalCorpusReport::assemble(ReportInputs {
            model_id: "test-model-v1".into(),
            harness_id: "h".into(),
            config_digest: "d".into(),
            cutoff: None,
            held_out_only: true,
            measured: true,
            k: 1,
            outcomes,
            samples: vec![],
            n_infra_errors: 0,
            n_samples_planned: 4,
            max_infra_rate: DEFAULT_MAX_INFRA_RATE,
            spend_cap_hit: false,
            provenance: provenance(),
            bootstrap_reps: 100,
            analysis_seed: 1,
        })
    }

    #[test]
    fn sign_and_verify_bench_bundle_roundtrip_and_tamper_rejection() {
        let (key, vk) = vox_crypto::facades::generate_signing_keypair();
        let pub_hex = hex::encode(verifying_key_to_bytes(&vk));
        let bundle = BenchBundle {
            schema_version: BENCH_BUNDLE_SCHEMA_VERSION,
            campaign_id: "camp-bench-01".to_string(),
            git_commit: "0123456789abcdef".to_string(),
            cargo_lock_blake3: "blake3_lock".to_string(),
            reports: vec![sample_report()],
            comparison: None,
        };

        let signed = sign_bundle(bundle, &key).expect("sign_bundle");
        assert!(signed.trusty_uri.starts_with("RA"));
        verify_bundle(&signed, Some(std::slice::from_ref(&pub_hex))).expect("verify_bundle");

        // Tamper with model_id -> must fail verification
        let mut tampered = signed.clone();
        tampered.bundle.reports[0].model_id = "tampered-model".to_string();
        assert!(verify_bundle(&tampered, None).is_err());
    }
}
