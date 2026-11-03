//! `vox explain <thing>` / `vox features [--domain <d>]`: thin wrappers over `vox-capability-registry`.
//!
//! All resolution and rendering lives in the L2 crate; the MCP twins call the same functions, so
//! `--json` here equals the MCP `data` for the same input (D-12).

use anyhow::{Result, bail};
use vox_capability_registry::{
    CapabilityIndex, Resolution, domain_counts_json, explain_rows, render_features,
    render_resolution, resolve, rows_in_domain,
};

/// Text for `vox explain <query>`; `json` prints the raw index rows instead of the card.
/// No match is an error (non-zero exit) carrying the `vox features` hint.
pub fn explain_output(index: &CapabilityIndex, query: &str, json: bool) -> Result<String> {
    let r = resolve(index, query)?;
    if matches!(r, Resolution::NoMatch) {
        bail!("{}", render_resolution(index, query, &r).trim_end());
    }
    if json {
        return Ok(serde_json::to_string_pretty(&explain_rows(&r))?);
    }
    Ok(render_resolution(index, query, &r))
}

/// Text for `vox features [--domain <d>]`; `json` prints raw rows (domain) or the counts array.
pub fn features_output(
    index: &CapabilityIndex,
    domain: Option<&str>,
    json: bool,
) -> Result<String> {
    if !json {
        return render_features(index, domain);
    }
    match domain {
        Some(d) => Ok(serde_json::to_string_pretty(&rows_in_domain(index, d)?)?),
        None => Ok(serde_json::to_string_pretty(&domain_counts_json(index))?),
    }
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

    /// Success criterion 3: `vox explain mesh` prints layer, GUI view, MCP tool and docs.
    #[test]
    fn explain_mesh_success_criterion() {
        let out = explain_output(bundled_index(), "mesh", false).unwrap();
        for needle in [
            "view mesh",
            "vox_mesh_nodes",
            "Crate:",
            "(L",
            "gui-navigation.md",
        ] {
            assert!(out.contains(needle), "card lacks {needle}:\n{out}");
        }
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
