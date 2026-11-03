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

fn err(msg: impl Into<String>, remediation: &str) -> String {
    ToolResult::<Value>::err_with_remediation(msg, remediation).to_json()
}

/// Rows for the best match (exact or shortlist); error envelope for no match or a bad query.
pub fn explain(args: ExplainArgs) -> String {
    let index = bundled_index();
    let r = match resolve(index, &args.query) {
        Ok(r) => r,
        Err(e) => return err(e.to_string(), "pass a query of 1-256 characters"),
    };
    let kind = match r {
        Resolution::Exact { .. } => "exact",
        Resolution::Shortlist(_) => "shortlist",
        Resolution::NoMatch => {
            return err(
                format!("no capability matches `{}`", args.query.trim()),
                "call vox_capability_features to browse by domain",
            );
        }
    };
    let text = render_resolution(index, &args.query, &r);
    match serde_json::to_value(explain_rows(&r)) {
        Ok(rows) => {
            ToolResult::ok_with_meta(rows, json!({ "text": text, "match": kind })).to_json()
        }
        Err(e) => err(format!("serialize rows: {e}"), "report this as a bug"),
    }
}

/// Rows of one domain, or the per-domain counts when `domain` is absent.
pub fn features(args: FeaturesArgs) -> String {
    let index = bundled_index();
    let Some(d) = args.domain.as_deref() else {
        let text = render_features(index, None).unwrap_or_default();
        return ToolResult::ok_with_meta(domain_counts_json(index), json!({ "text": text }))
            .to_json();
    };
    let rows = match rows_in_domain(index, d) {
        Ok(rows) => rows,
        Err(e) => {
            return err(
                e.to_string(),
                "call vox_capability_features without a domain to list valid domains",
            );
        }
    };
    let text = render_features(index, Some(d)).unwrap_or_default();
    match serde_json::to_value(rows) {
        Ok(v) => ToolResult::ok_with_meta(v, json!({ "text": text })).to_json(),
        Err(e) => err(format!("serialize rows: {e}"), "report this as a bug"),
    }
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
    fn features_overlong_domain_error_does_not_echo_it() {
        let v = parse(&features(FeaturesArgs {
            domain: Some("x".repeat(100_000)),
        }));
        assert_eq!(v["success"], false, "{}", v.to_string().len());
        assert!(v.to_string().len() < 1_000, "response not bounded");
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
