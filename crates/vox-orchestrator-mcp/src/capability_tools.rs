//! `vox_capability_explain` / `vox_capability_features`: read-only MCP twins of `vox explain` /
//! `vox features`. Pure and synchronous; they touch only the embedded capability index, so `data`
//! equals the CLI `--json` rows for the same input (D-12).

use crate::params::ToolResult;
use serde::Deserialize;
use serde_json::{Value, json};
use vox_capability_registry::{
    Resolution, bundled_index, domain_counts_json, explain_rows, render_features,
    render_resolution, resolve, rows_in_domain,
};

/// Input of `vox_capability_explain`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExplainArgs {
    pub query: String,
}

/// Input of `vox_capability_features`.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeaturesArgs {
    #[serde(default)]
    pub domain: Option<String>,
}

/// Rows for the best match (exact or shortlist); error envelope for no match or a bad query.
pub fn explain(_args: ExplainArgs) -> String {
    ToolResult::<Value>::err_with_remediation("RED stub: explain", "RED stub").to_json()
}

/// Rows of one domain, or the per-domain counts when `domain` is absent.
pub fn features(_args: FeaturesArgs) -> String {
    ToolResult::<Value>::err_with_remediation("RED stub: features", "RED stub").to_json()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Value {
        serde_json::from_str(s).unwrap_or_else(|e| panic!("not JSON ({e}): {s}"))
    }

    #[test]
    fn explain_returns_rows_for_check() {
        let v = parse(&explain(ExplainArgs {
            query: "check".into(),
        }));
        assert_eq!(v["success"], true, "{v}");
        assert_eq!(v["data"][0]["id"], "cli.check");
        assert!(v["meta"]["text"].as_str().unwrap().contains("Equivalents"));
        assert_eq!(v["meta"]["match"], "exact");
    }

    #[test]
    fn explain_bad_query_is_error_envelope() {
        for q in [String::new(), "x".repeat(257)] {
            let v = parse(&explain(ExplainArgs { query: q }));
            assert_eq!(v["success"], false, "{v}");
            assert!(v["remediation"].as_str().is_some(), "{v}");
        }
    }

    #[test]
    fn explain_no_match_points_at_features() {
        let v = parse(&explain(ExplainArgs {
            query: "zzqqxx-no-such".into(),
        }));
        assert_eq!(v["success"], false, "{v}");
        assert!(
            v["remediation"]
                .as_str()
                .unwrap()
                .contains("vox_capability_features")
        );
    }

    #[test]
    fn features_unknown_domain_is_error_envelope() {
        let v = parse(&features(FeaturesArgs {
            domain: Some("nope".into()),
        }));
        assert_eq!(v["success"], false, "{v}");
        assert!(v["remediation"].as_str().is_some(), "{v}");
    }

    #[test]
    fn features_without_domain_returns_counts() {
        let v = parse(&features(FeaturesArgs { domain: None }));
        assert_eq!(v["success"], true, "{v}");
        assert!(v["data"][0].get("label").is_some(), "{v}");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        assert!(serde_json::from_value::<ExplainArgs>(json!({"query": "a", "x": 1})).is_err());
        assert!(serde_json::from_value::<FeaturesArgs>(json!({"x": 1})).is_err());
    }
}
