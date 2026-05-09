//! Atlas submission pipeline: gate checks + adapter dispatch.

use serde::{Deserialize, Serialize};

use crate::atlas::manifest::AtlasManifest;

/// Configuration for Atlas submission gate checks.
#[derive(Debug, Clone)]
pub struct AtlasSubmissionConfig {
    /// Trusty URI of the signed preregistration (None = not preregistered).
    pub prereg_id: Option<String>,
    /// True once all providers have cleared or the 14-day window has expired.
    pub reply_window_cleared: bool,
    /// If true, require at least one `supported: false` finding in the manifest.
    pub require_negative_result: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum AtlasGateError {
    MissingPreregistration,
    ReplyWindowNotCleared,
    NegativeResultQuotaNotMet,
}

impl std::fmt::Display for AtlasGateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingPreregistration => write!(f, "atlas requires a signed preregistration"),
            Self::ReplyWindowNotCleared => {
                write!(f, "14-day provider right-of-reply window not cleared")
            }
            Self::NegativeResultQuotaNotMet => {
                write!(f, "atlas must include at least one null-result finding")
            }
        }
    }
}

impl std::error::Error for AtlasGateError {}

/// Gate enforcing all publication preconditions before Atlas release.
pub struct AtlasSubmissionGate;

impl AtlasSubmissionGate {
    /// Returns `Ok(())` if the Atlas is clear for submission, else the first blocking error.
    pub fn check(
        manifest: &AtlasManifest,
        config: &AtlasSubmissionConfig,
    ) -> Result<(), AtlasGateError> {
        if config.prereg_id.is_none() {
            return Err(AtlasGateError::MissingPreregistration);
        }
        if !config.reply_window_cleared {
            return Err(AtlasGateError::ReplyWindowNotCleared);
        }
        if config.require_negative_result && manifest.negative_result_count() == 0 {
            return Err(AtlasGateError::NegativeResultQuotaNotMet);
        }
        Ok(())
    }
}

/// Summary of adapter receipts after Atlas submission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasSubmissionReceipt {
    pub zenodo_external_id: Option<String>,
    pub arxiv_external_id: Option<String>,
    pub osf_external_id: Option<String>,
    pub submission_date: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::atlas::manifest::{AtlasFinding, AtlasManifestBuilder};

    fn sample_manifest() -> AtlasManifest {
        let mut builder =
            AtlasManifestBuilder::new("Provider Atlas Q2-2026".into(), "provider-atlas".into());
        builder.add_finding(AtlasFinding {
            id: "f-001".into(),
            claim_text: "Latency increased.".into(),
            nanopub_uri: "https://vox.scientia/np/RAtest001".into(),
            supported: true,
        });
        builder.add_finding(AtlasFinding {
            id: "f-002".into(),
            claim_text: "Hypothesis B: null not rejected.".into(),
            nanopub_uri: "https://vox.scientia/np/RAtest002".into(),
            supported: false,
        });
        builder.build("2026-05-09")
    }

    #[test]
    fn submission_gate_passes_with_prereg_and_negative_result() {
        let manifest = sample_manifest();
        let config = AtlasSubmissionConfig {
            prereg_id: Some("RA_abc123".into()),
            reply_window_cleared: true,
            require_negative_result: true,
        };
        let result = AtlasSubmissionGate::check(&manifest, &config);
        assert!(result.is_ok(), "expected Ok, got {:?}", result);
    }

    #[test]
    fn submission_gate_fails_without_prereg() {
        let manifest = sample_manifest();
        let config = AtlasSubmissionConfig {
            prereg_id: None,
            reply_window_cleared: true,
            require_negative_result: false,
        };
        let result = AtlasSubmissionGate::check(&manifest, &config);
        assert!(matches!(
            result,
            Err(AtlasGateError::MissingPreregistration)
        ));
    }

    #[test]
    fn submission_gate_fails_without_reply_window() {
        let manifest = sample_manifest();
        let config = AtlasSubmissionConfig {
            prereg_id: Some("RA_abc".into()),
            reply_window_cleared: false,
            require_negative_result: false,
        };
        let result = AtlasSubmissionGate::check(&manifest, &config);
        assert!(matches!(result, Err(AtlasGateError::ReplyWindowNotCleared)));
    }

    #[test]
    fn submission_gate_fails_without_negative_result_when_required() {
        let mut builder = AtlasManifestBuilder::new("Atlas".into(), "provider-atlas".into());
        builder.add_finding(AtlasFinding {
            id: "f-001".into(),
            claim_text: "All positive.".into(),
            nanopub_uri: "uri".into(),
            supported: true,
        });
        let manifest = builder.build("2026-05-09");
        let config = AtlasSubmissionConfig {
            prereg_id: Some("RA_abc".into()),
            reply_window_cleared: true,
            require_negative_result: true,
        };
        let result = AtlasSubmissionGate::check(&manifest, &config);
        assert!(matches!(
            result,
            Err(AtlasGateError::NegativeResultQuotaNotMet)
        ));
    }
}
