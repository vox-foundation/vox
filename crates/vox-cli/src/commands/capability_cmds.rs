//! `vox explain <thing>` / `vox features [--domain <d>]`: thin wrappers over `vox-capability-registry`.
//!
//! All resolution and rendering lives in the L2 crate; the MCP twins call the same functions, so
//! `--json` here equals the MCP `data` for the same input (D-12).

use anyhow::Result;
use vox_capability_registry::CapabilityIndex;

/// Text for `vox explain <query>`; `json` prints the raw index rows instead of the card.
pub fn explain_output(_index: &CapabilityIndex, _query: &str, _json: bool) -> Result<String> {
    anyhow::bail!("RED stub: explain_output")
}

/// Text for `vox features [--domain <d>]`; `json` prints raw rows (domain) or the counts array.
pub fn features_output(
    _index: &CapabilityIndex,
    _domain: Option<&str>,
    _json: bool,
) -> Result<String> {
    anyhow::bail!("RED stub: features_output")
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_capability_registry::{bundled_index, explain_rows, resolve};

    #[test]
    fn explain_exact_prints_card() {
        let out = explain_output(bundled_index(), "check", false).unwrap();
        for needle in ["Equivalents", "Location", "Status", "Related"] {
            assert!(out.contains(needle), "card lacks {needle}:\n{out}");
        }
    }

    #[test]
    fn explain_json_is_explain_rows() {
        let out = explain_output(bundled_index(), "check", true).unwrap();
        let got: serde_json::Value = serde_json::from_str(&out).unwrap();
        let r = resolve(bundled_index(), "check").unwrap();
        let want = serde_json::to_value(explain_rows(&r)).unwrap();
        assert!(got.is_array() && !got.as_array().unwrap().is_empty());
        assert_eq!(got, want);
    }

    #[test]
    fn explain_no_match_is_err() {
        let e = explain_output(bundled_index(), "zzqqxx-no-such", false).unwrap_err();
        assert!(e.to_string().contains("vox features"), "{e}");
    }

    #[test]
    fn features_unknown_domain_is_err() {
        assert!(features_output(bundled_index(), Some("nope"), false).is_err());
    }

    #[test]
    fn features_json_shapes() {
        let rows = features_output(bundled_index(), Some("secrets"), true).unwrap();
        let rows: serde_json::Value = serde_json::from_str(&rows).unwrap();
        assert!(!rows.as_array().unwrap().is_empty());
        let all = features_output(bundled_index(), None, true).unwrap();
        let all: serde_json::Value = serde_json::from_str(&all).unwrap();
        let first = &all.as_array().unwrap()[0];
        for k in ["id", "label", "cli", "mcp", "surface"] {
            assert!(first.get(k).is_some(), "domain count lacks {k}: {first}");
        }
    }

    #[test]
    fn features_text_lists_secrets_get() {
        let out = features_output(bundled_index(), Some("secrets"), false).unwrap();
        assert!(out.contains("vox secrets get"), "{out}");
    }
}
