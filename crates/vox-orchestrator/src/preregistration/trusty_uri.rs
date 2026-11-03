//! Trusty URI computation for [`PreregistrationV1`].
//!
//! A Trusty URI embeds a base64url-encoded content hash in the URI so the identifier
//! is self-verifying. Per the nanopublication spec, the hash covers the canonical
//! serialization of the artifact.
//!
//! Algorithm:
//! 1. Serialize the prereg to canonical JSON (BTreeMap-ordered keys via serde_json).
//! 2. SHA-256 hash the UTF-8 bytes.
//! 3. Base64url-encode (no padding).
//! 4. Prepend "RA" (the Trusty URI type code for nanopublication artifacts).

use std::collections::BTreeMap;
use vox_research_events::preregistration::PreregistrationV1;

/// Why a value has no canonical form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CanonicalError {
    /// JSON cannot carry the value exactly; non-finite numbers serialize as `null`.
    #[error("value does not survive a JSON round trip (non-finite number?)")]
    Lossy,
    #[error("serialization failed: {0}")]
    Serialization(String),
}

/// Compute the Trusty URI for `prereg`.
///
/// The `id` field of the prereg is excluded from the hash (it is the output of this function).
/// All other fields are included in canonical key-sorted JSON order.
pub fn compute_trusty_uri(prereg: &PreregistrationV1) -> Result<String, CanonicalError> {
    let canonical = canonical_json(prereg)?;
    let hash = vox_crypto::facades::interop_sha256(canonical.as_bytes());
    let b64 = base64url_encode_no_pad(&hash);
    Ok(format!("RA{b64}"))
}

/// Serialize any `T: Serialize + DeserializeOwned + PartialEq` to canonical UTF-8 JSON bytes
/// with keys sorted lexicographically at every depth, rejecting lossy values (`NaN`, `Infinity`).
pub fn canonical_json_bytes<T>(val: &T) -> Result<Vec<u8>, CanonicalError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    let value =
        serde_json::to_value(val).map_err(|e| CanonicalError::Serialization(e.to_string()))?;
    match serde_json::from_value::<T>(value.clone()) {
        Ok(back) if &back == val => {}
        _ => return Err(CanonicalError::Lossy),
    }
    serde_json::to_vec(&sort_keys(value)).map_err(|e| CanonicalError::Serialization(e.to_string()))
}

/// Compute a `RA<base64url(sha256(canonical_json_bytes(val)))>` Trusty URI for any value `T`.
pub fn compute_value_trusty_uri<T>(val: &T) -> Result<String, CanonicalError>
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq,
{
    let bytes = canonical_json_bytes(val)?;
    let hash = vox_crypto::facades::interop_sha256(&bytes);
    let b64 = base64url_encode_no_pad(&hash);
    Ok(format!("RA{b64}"))
}

/// Serialize `prereg` to canonical JSON with keys sorted at every depth.
///
/// The `id` field is set to an empty string in the canonical form so that the
/// Trusty URI can be computed before the field is written back. Values JSON
/// cannot carry exactly (NaN, infinities) are rejected rather than signed as
/// `null`.
pub(crate) fn canonical_json(prereg: &PreregistrationV1) -> Result<String, CanonicalError> {
    let mut with_empty_id = prereg.clone();
    with_empty_id.id = String::new();
    let bytes = canonical_json_bytes(&with_empty_id)?;
    String::from_utf8(bytes).map_err(|e| CanonicalError::Serialization(e.to_string()))
}

/// `serde_json` keeps insertion order workspace-wide (`preserve_order`), so
/// object keys are re-inserted in sorted order explicitly.
fn sort_keys(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> =
                map.into_iter().map(|(k, v)| (k, sort_keys(v))).collect();
            serde_json::Value::Object(sorted.into_iter().collect())
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sort_keys).collect())
        }
        other => other,
    }
}

/// Base64url encoding without padding characters.
fn base64url_encode_no_pad(bytes: &[u8]) -> String {
    // Manual base64url — avoids adding a `base64` dep not in workspace.
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let combined = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((combined >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((combined >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(ALPHABET[((combined >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(ALPHABET[(combined & 0x3F) as usize] as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_research_events::preregistration::{
        DecisionRule, MetricSpec, PreregistrationV1, StatisticalTest, StopRule, SubstrateRef,
        TestSpec,
    };

    fn sample_prereg() -> PreregistrationV1 {
        PreregistrationV1 {
            id: String::new(),
            hypothesis: "p95 latency rises by >10ms after model update".to_string(),
            eval_substrate: SubstrateRef {
                repo_swhid: "swh:1:rev:abc123".to_string(),
                eval_set_swhid: "swh:1:dir:def456".to_string(),
                inspect_task_id: None,
            },
            metric: MetricSpec {
                name: "p95_latency_ms".to_string(),
                aggregation: "percentile_95".to_string(),
                units: "milliseconds".to_string(),
            },
            statistical_test: TestSpec {
                kind: StatisticalTest::Bayesian,
                prior: Some("Beta(1,1)".to_string()),
                threshold: Some(0.95),
                alpha: None,
            },
            stopping_rule: StopRule {
                max_n: 1000,
                alpha: None,
                threshold: Some(0.95),
            },
            decision_rule: DecisionRule {
                description: "if posterior P(direction) > 0.95, conclude hypothesis".to_string(),
            },
            cost_cap_usd: 50.0,
            signed_at: 1715299200,
            signing_key: "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"
                .to_string(),
            supersedes: None,
            analysis_tree_commit: None,
        }
    }

    #[test]
    fn canonical_json_sorts_keys_at_every_depth() {
        // Catches: nested objects keeping struct-field order (serde_json preserve_order)
        let canonical = canonical_json(&sample_prereg()).unwrap();
        assert!(
            canonical.contains(r#""eval_substrate":{"eval_set_swhid":"#),
            "{canonical}"
        );
        assert!(
            canonical.contains(r#""metric":{"aggregation":"#),
            "{canonical}"
        );
    }

    #[test]
    fn canonical_json_rejects_non_finite_numbers() {
        // Catches: NaN/inf silently signed as `null`
        let mut prereg = sample_prereg();
        prereg.cost_cap_usd = f64::NAN;
        assert_eq!(canonical_json(&prereg), Err(CanonicalError::Lossy));
        let mut prereg = sample_prereg();
        prereg.statistical_test.threshold = Some(f64::INFINITY);
        assert_eq!(canonical_json(&prereg), Err(CanonicalError::Lossy));
        assert!(compute_trusty_uri(&prereg).is_err());
    }

    #[test]
    fn trusty_uri_hashes_canonical_json_with_sha256() {
        let prereg = sample_prereg();
        let digest =
            vox_crypto::facades::interop_sha256(canonical_json(&prereg).unwrap().as_bytes());
        assert_eq!(
            compute_trusty_uri(&prereg).unwrap(),
            format!("RA{}", base64url_encode_no_pad(&digest))
        );
        assert_ne!(digest, [0u8; 32]);
    }

    #[test]
    fn trusty_uri_starts_with_ra() {
        let prereg = sample_prereg();
        let uri = compute_trusty_uri(&prereg).unwrap();
        assert!(
            uri.starts_with("RA"),
            "Trusty URI must start with 'RA', got: {uri}"
        );
    }

    #[test]
    fn trusty_uri_is_deterministic() {
        let prereg = sample_prereg();
        let uri1 = compute_trusty_uri(&prereg).unwrap();
        let uri2 = compute_trusty_uri(&prereg).unwrap();
        assert_eq!(uri1, uri2, "Trusty URI must be deterministic");
    }

    #[test]
    fn trusty_uri_changes_on_field_change() {
        let mut prereg = sample_prereg();
        let uri1 = compute_trusty_uri(&prereg).unwrap();
        prereg.hypothesis = "p95 latency rises by >20ms after model update".to_string();
        let uri2 = compute_trusty_uri(&prereg).unwrap();
        assert_ne!(uri1, uri2, "Trusty URI must change when content changes");
    }
}
