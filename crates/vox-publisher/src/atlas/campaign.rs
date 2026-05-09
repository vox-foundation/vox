//! MeasurementCampaign: typed record for a pre-registered measurement campaign.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementCampaign {
    pub campaign_id: String,
    pub hypothesis: String,
    pub metric: String,
    /// Corresponds to `InspectTaskDescriptor.task_id` in `vox-inspect-bridge`.
    pub inspect_task_id: String,
    /// Trusty URI of the signed `PreregistrationV1` (set after OSF registration).
    pub prereg_id: Option<String>,
    pub started_at: i64,
}

impl MeasurementCampaign {
    pub fn is_preregistered(&self) -> bool {
        self.prereg_id.is_some()
    }

    /// Build the JSON descriptor passed to the vox-inspect-bridge layer.
    pub fn to_inspect_bridge_config(&self) -> serde_json::Value {
        serde_json::json!({
            "task_id": self.inspect_task_id,
            "campaign_id": self.campaign_id,
            "hypothesis": self.hypothesis,
            "metric": self.metric,
            "prereg_id": self.prereg_id,
            "started_at": self.started_at,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn campaign_inspect_descriptor_contains_task_id() {
        let campaign = MeasurementCampaign {
            campaign_id: "camp-latency-2026-q2".into(),
            hypothesis: "Provider latency has increased relative to Q1 2026 baseline.".into(),
            metric: "p95_latency_ms".into(),
            inspect_task_id: "vox-provider-latency-v1".into(),
            prereg_id: None,
            started_at: 1_746_748_800,
        };
        let descriptor = campaign.to_inspect_bridge_config();
        assert_eq!(descriptor["task_id"], "vox-provider-latency-v1");
        assert!(
            descriptor["hypothesis"]
                .as_str()
                .unwrap()
                .contains("latency")
        );
    }

    #[test]
    fn campaign_requires_prereg_check() {
        let campaign_no_prereg = MeasurementCampaign {
            campaign_id: "camp-001".into(),
            hypothesis: "Test".into(),
            metric: "m".into(),
            inspect_task_id: "t".into(),
            prereg_id: None,
            started_at: 0,
        };
        assert!(!campaign_no_prereg.is_preregistered());

        let campaign_with_prereg = MeasurementCampaign {
            prereg_id: Some("RA_abc123".into()),
            ..campaign_no_prereg
        };
        assert!(campaign_with_prereg.is_preregistered());
    }
}
