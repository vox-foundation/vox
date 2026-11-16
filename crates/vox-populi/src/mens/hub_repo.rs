//! Hugging Face repo naming for published adapters (HFNAME-01, HFNAME-02).
//!
//! One private repo per ladder size, `<namespace>/vox-mens-<family>-<size>` (for example
//! `vox-mens-qwen3-0.6b`, `vox-mens-qwen3.5-4b`, `vox-mens-qwen3.8-27b`), plus one index repo
//! `<namespace>/vox-mens-index` and one playground Space `<namespace>/vox-mens-playground`.
//! The family and size come from the pinned base in `gpu-specs.yaml`, so a repo id never drifts
//! from the base it was trained on. The namespace is operator config ([`NAMESPACE_ENV`]), never
//! source: this repo is public.

use anyhow::{Result, bail};

/// Env var that names the HF account or org the adapters publish under.
pub const NAMESPACE_ENV: &str = "VOX_MENS_HF_NAMESPACE";

/// Active Nov 2026 5-rung Qwen3.5 / Qwen3.8 ladder sizes that can have a published adapter, smallest first.
pub const SIZES: [&str; 5] = ["0.8B", "2B", "4B", "9B", "27B"];

/// Legacy Qwen3 3.0 ladder sizes retained strictly as fallback / pull compatibility.
pub const LEGACY_SIZES: [&str; 5] = ["0.6B", "1.7B", "8B", "14B", "32B"];

/// All known ladder sizes across active Qwen3.5 / Qwen3.8 rungs and legacy Qwen3 3.0 fallbacks.
pub const ALL_KNOWN_SIZES: [&str; 10] = [
    "0.6B", "0.8B", "1.7B", "2B", "4B", "8B", "9B", "14B", "27B", "32B",
];

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    s.get(..prefix.len())
        .filter(|head| head.eq_ignore_ascii_case(prefix))
        .map(|_| &s[prefix.len()..])
}

/// Resolve the model family (`"qwen3"`, `"qwen3.5"`, or `"qwen3.8"`) from a HF pin, repo slug,
/// model label, or bare size.
///
/// Explicit `Qwen3-4B` / `qwen3-4b` resolves to `"qwen3"` for backwards compatibility, whereas
/// bare `"4B"` or `"Qwen/Qwen3.5-4B@..."` resolves to `"qwen3.5"`.
#[must_use]
pub fn family_of_model(pin_or_model_or_size: &str) -> &'static str {
    let repo = pin_or_model_or_size
        .split('@')
        .next()
        .unwrap_or(pin_or_model_or_size);
    let name = repo.rsplit(['/', '\\']).next().unwrap_or(repo).trim();
    let lower = name.to_ascii_lowercase();
    let stripped = lower.strip_prefix("vox-mens-").unwrap_or(&lower);
    if stripped.contains("qwen3.8-") {
        return "qwen3.8";
    }
    if stripped.contains("qwen3.5-") {
        return "qwen3.5";
    }
    if stripped.contains("qwen3-") {
        return "qwen3";
    }
    match size_of_pin(stripped).as_deref() {
        Some("27B") => "qwen3.8",
        Some("0.8B" | "2B" | "4B" | "9B") => "qwen3.5",
        Some("0.6B" | "1.7B" | "8B" | "14B" | "32B") => "qwen3",
        _ => "qwen3",
    }
}

/// `4B` for `Qwen/Qwen3.5-4B@<sha>`, `Qwen3-4B`, `vox-mens-qwen3.5-4b`, or `4B`;
/// `None` when the id is not a known ladder size.
#[must_use]
pub fn size_of_pin(pin: &str) -> Option<String> {
    let repo = pin.split('@').next().unwrap_or(pin);
    let name = repo.rsplit(['/', '\\']).next().unwrap_or(repo).trim();
    let after_vox = strip_prefix_ci(name, "vox-mens-").unwrap_or(name);
    let size = strip_prefix_ci(after_vox, "Qwen3.8-")
        .or_else(|| strip_prefix_ci(after_vox, "Qwen3.5-"))
        .or_else(|| strip_prefix_ci(after_vox, "Qwen3-"))
        .unwrap_or(after_vox);
    ALL_KNOWN_SIZES
        .iter()
        .find(|s| s.eq_ignore_ascii_case(size))
        .map(|s| (*s).to_string())
}

/// Display/model label matching resolved family + canonical size (e.g. `"Qwen3.8-27B"`,
/// `"Qwen3.5-4B"`, `"Qwen3-0.6B"`, `"Qwen3-4B"`).
#[must_use]
pub fn model_label_for(size_or_pin: &str) -> String {
    let family = match family_of_model(size_or_pin) {
        "qwen3.8" => "Qwen3.8",
        "qwen3.5" => "Qwen3.5",
        _ => "Qwen3",
    };
    match size_of_pin(size_or_pin) {
        Some(size) => format!("{family}-{size}"),
        None => size_or_pin.trim().to_string(),
    }
}

/// Canonical repo slug without namespace, e.g. `vox-mens-qwen3-0.6b`, `vox-mens-qwen3.5-4b`,
/// `vox-mens-qwen3.8-27b`, or `vox-mens-qwen3-4b` for explicit `"Qwen3-4B"`.
pub fn repo_slug_for_model(size_or_pin: &str) -> Result<String> {
    let Some(size) = size_of_pin(size_or_pin) else {
        bail!(
            "`{size_or_pin}` is not a ladder size; known sizes: {}",
            ALL_KNOWN_SIZES.join(", ")
        );
    };
    let family = family_of_model(size_or_pin);
    Ok(format!("vox-mens-{family}-{}", size.to_ascii_lowercase()))
}

/// Canonical local training run directory, e.g. `mens/runs/qwen3.5-4b`, `mens/runs/qwen3.8-27b`,
/// or `mens/runs/qwen3-4b` when `"Qwen3-4B"` is given.
pub fn local_run_dir_for_size(size_or_pin: &str) -> Result<String> {
    let Some(size) = size_of_pin(size_or_pin) else {
        bail!(
            "`{size_or_pin}` is not a ladder size; known sizes: {}",
            ALL_KNOWN_SIZES.join(", ")
        );
    };
    let family = family_of_model(size_or_pin);
    Ok(format!("mens/runs/{family}-{}", size.to_ascii_lowercase()))
}

/// A namespace from config: trimmed, non-empty, HF-legal characters only.
pub fn resolve_namespace(raw: Option<&str>) -> Result<String> {
    let ns = raw.map(str::trim).unwrap_or("");
    if ns.is_empty() {
        bail!(
            "no Hugging Face namespace; set {NAMESPACE_ENV} to the HF account or org to publish under"
        );
    }
    if !ns
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        bail!("{NAMESPACE_ENV} `{ns}` is not a valid Hugging Face namespace");
    }
    Ok(ns.to_string())
}

/// `<namespace>/vox-mens-<family>-<size>` with the size lower-cased (e.g.
/// `me/vox-mens-qwen3-0.6b`, `me/vox-mens-qwen3.5-4b`, `me/vox-mens-qwen3.8-27b`).
pub fn repo_id_for_size(namespace: &str, size_or_pin: &str) -> Result<String> {
    let slug = repo_slug_for_model(size_or_pin)?;
    let ns = resolve_namespace(Some(namespace))?;
    Ok(format!("{ns}/{slug}"))
}

/// `<namespace>/vox-mens-index`: the repo that lists every size with its status and pin.
pub fn index_repo_id(namespace: &str) -> Result<String> {
    Ok(format!(
        "{}/vox-mens-index",
        resolve_namespace(Some(namespace))?
    ))
}

/// `<namespace>/vox-mens-playground`: the interactive HF Space showcasing published ladder weights.
pub fn space_repo_id(namespace: &str) -> Result<String> {
    Ok(format!(
        "{}/vox-mens-playground",
        resolve_namespace(Some(namespace))?
    ))
}

fn sanitize_slug(raw: &str) -> Result<String> {
    let trimmed = raw.trim().to_ascii_lowercase().replace('_', "-");
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
    {
        bail!("invalid dataset slug `{raw}`");
    }
    Ok(trimmed)
}

/// `<namespace>/vox-mens-eval-<suite>`: evaluation dataset repo on HF Hub.
pub fn eval_dataset_repo_id(namespace: &str, suite: &str) -> Result<String> {
    let ns = resolve_namespace(Some(namespace))?;
    let slug = sanitize_slug(suite)?;
    Ok(format!("{ns}/vox-mens-eval-{slug}"))
}

/// Known Hub spoke slugs supported under `spokes/<spoke>/`.
pub const KNOWN_SPOKES: &[&str] = &[
    "vox-lang",
    "rust",
    "harness",
    "tool-selection",
    "argument-generation",
    "agents",
    "safety",
    "populi",
    "harry",
];

/// Relative repository path `spokes/<spoke>` for a known spoke slug.
pub fn spoke_rel_dir(spoke: &str) -> Result<std::path::PathBuf> {
    let trimmed = spoke.trim();
    if trimmed.is_empty() || !KNOWN_SPOKES.contains(&trimmed) {
        bail!(
            "unknown spoke `{spoke}`; known spokes: {}",
            KNOWN_SPOKES.join(", ")
        );
    }
    Ok(std::path::PathBuf::from("spokes").join(trimmed))
}

/// `<namespace>/vox-mens-data-<spoke>`: curated training corpus repo on HF Hub.
pub fn data_dataset_repo_id(namespace: &str, spoke: &str) -> Result<String> {
    let ns = resolve_namespace(Some(namespace))?;
    let slug = sanitize_slug(spoke)?;
    Ok(format!("{ns}/vox-mens-data-{slug}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_active_and_legacy_size_maps_to_its_repo() {
        let want = [
            ("0.8B", "me/vox-mens-qwen3.5-0.8b"),
            ("2B", "me/vox-mens-qwen3.5-2b"),
            ("4B", "me/vox-mens-qwen3.5-4b"),
            ("9B", "me/vox-mens-qwen3.5-9b"),
            ("27B", "me/vox-mens-qwen3.8-27b"),
            ("0.6B", "me/vox-mens-qwen3-0.6b"),
            ("Qwen3-4B", "me/vox-mens-qwen3-4b"),
            ("1.7B", "me/vox-mens-qwen3-1.7b"),
            ("8B", "me/vox-mens-qwen3-8b"),
            ("14B", "me/vox-mens-qwen3-14b"),
            ("32B", "me/vox-mens-qwen3-32b"),
        ];
        for (size, repo) in want {
            assert_eq!(repo_id_for_size("me", size).unwrap(), repo);
        }
    }

    #[test]
    fn family_label_slug_and_run_dir_handle_multi_family_and_legacy_4b() {
        assert_eq!(family_of_model("27B"), "qwen3.8");
        assert_eq!(family_of_model("Qwen/Qwen3.8-27B@abc"), "qwen3.8");
        assert_eq!(family_of_model("0.8B"), "qwen3.5");
        assert_eq!(family_of_model("Qwen/Qwen3.5-0.8B@abc"), "qwen3.5");
        assert_eq!(family_of_model("4B"), "qwen3.5");
        assert_eq!(family_of_model("Qwen/Qwen3.5-4B@abc"), "qwen3.5");
        assert_eq!(family_of_model("Qwen3-4B"), "qwen3");
        assert_eq!(family_of_model("Qwen/Qwen3-4B@abc"), "qwen3");
        assert_eq!(family_of_model("qwen3-4b"), "qwen3");
        assert_eq!(family_of_model("0.6B"), "qwen3");

        assert_eq!(model_label_for("27B"), "Qwen3.8-27B");
        assert_eq!(model_label_for("0.8B"), "Qwen3.5-0.8B");
        assert_eq!(model_label_for("4B"), "Qwen3.5-4B");
        assert_eq!(model_label_for("0.6B"), "Qwen3-0.6B");
        assert_eq!(model_label_for("Qwen3-4B"), "Qwen3-4B");

        assert_eq!(
            repo_slug_for_model("0.8B").unwrap(),
            "vox-mens-qwen3.5-0.8b"
        );
        assert_eq!(repo_slug_for_model("0.6B").unwrap(), "vox-mens-qwen3-0.6b");
        assert_eq!(repo_slug_for_model("4B").unwrap(), "vox-mens-qwen3.5-4b");
        assert_eq!(
            repo_slug_for_model("Qwen3-4B").unwrap(),
            "vox-mens-qwen3-4b"
        );
        assert_eq!(repo_slug_for_model("27B").unwrap(), "vox-mens-qwen3.8-27b");

        assert_eq!(
            local_run_dir_for_size("4B").unwrap(),
            "mens/runs/qwen3.5-4b"
        );
        assert_eq!(
            local_run_dir_for_size("27B").unwrap(),
            "mens/runs/qwen3.8-27b"
        );
        assert_eq!(
            local_run_dir_for_size("Qwen3-4B").unwrap(),
            "mens/runs/qwen3-4b"
        );
        assert_eq!(space_repo_id("me").unwrap(), "me/vox-mens-playground");
    }

    #[test]
    fn the_repo_follows_the_ladder_pin_not_a_hand_typed_size() {
        let overlay = crate::mens::tensor::size_select::load_ladder_overlay(None).unwrap();
        let mut seen = Vec::new();
        for group in ["qwen3_5_code", "qwen3_code"] {
            for rung in &overlay[group] {
                if !rung.methods.iter().any(|m| m == "qlora") {
                    continue;
                }
                let size = size_of_pin(&rung.hf_id).expect("ladder pin names a ladder size");
                repo_id_for_size("me", &rung.hf_id).unwrap();
                seen.push((group, size));
            }
        }
        for s in SIZES {
            assert!(
                seen.iter().any(|(_, x)| x == s),
                "ladder has no {s} rung across qwen3_5_code + qwen3_code"
            );
        }
        for s in LEGACY_SIZES {
            assert!(
                seen.iter().any(|(g, x)| *g == "qwen3_code" && x == s),
                "qwen3_code overlay has no legacy {s} rung"
            );
        }
    }

    #[test]
    fn pins_revisions_and_prefixes_resolve_to_the_same_size() {
        let sha = "b968826d9c46dd6066d109eabc6255188de91218";
        assert_eq!(
            size_of_pin(&format!("Qwen/Qwen3-8B@{sha}")).as_deref(),
            Some("8B")
        );
        assert_eq!(size_of_pin("Qwen3-8B").as_deref(), Some("8B"));
        assert_eq!(size_of_pin("Qwen3.5-4B").as_deref(), Some("4B"));
        assert_eq!(size_of_pin("vox-mens-qwen3.8-27b").as_deref(), Some("27B"));
        assert_eq!(size_of_pin("8b").as_deref(), Some("8B"));
        assert_eq!(size_of_pin("Qwen/Qwen2.5-Coder-7B"), None);
    }

    #[test]
    fn a_missing_namespace_names_the_config_key() {
        for raw in [None, Some(""), Some("   ")] {
            let e = resolve_namespace(raw).unwrap_err().to_string();
            assert!(e.contains(NAMESPACE_ENV), "{e}");
        }
    }

    #[test]
    fn an_illegal_namespace_or_size_is_refused() {
        assert!(resolve_namespace(Some("a/b")).is_err());
        assert!(resolve_namespace(Some("a b")).is_err());
        assert!(repo_id_for_size("me", "7B").is_err());
        assert_eq!(index_repo_id("me").unwrap(), "me/vox-mens-index");
    }

    #[test]
    fn spoke_rel_dir_maps_known_spokes_and_rejects_unknown() {
        for &sp in KNOWN_SPOKES {
            assert_eq!(
                spoke_rel_dir(sp).unwrap(),
                std::path::PathBuf::from("spokes").join(sp)
            );
        }
        assert!(spoke_rel_dir("").is_err());
        assert!(spoke_rel_dir("nonexistent-spoke").is_err());
    }
}
