//! Hugging Face repo naming for published adapters (HFNAME-01, HFNAME-02).
//!
//! One private repo per ladder size, `<namespace>/vox-mens-qwen3-<size>`, plus one index repo
//! `<namespace>/vox-mens-index`. The size comes from the pinned base in `gpu-specs.yaml`
//! (`Qwen/Qwen3-4B@<sha>` is `4B`), so a repo id never drifts from the base it was trained on.
//! The namespace is operator config ([`NAMESPACE_ENV`]), never source: this repo is public.

use anyhow::{Result, bail};

/// Env var that names the HF account or org the adapters publish under.
pub const NAMESPACE_ENV: &str = "VOX_MENS_HF_NAMESPACE";

/// The ladder sizes that can have a published adapter, smallest first.
pub const SIZES: [&str; 6] = ["0.6B", "1.7B", "4B", "8B", "14B", "32B"];

/// `4B` for `Qwen/Qwen3-4B@<sha>`, `Qwen3-4B` or `4B`; `None` when the id is not a ladder size.
pub fn size_of_pin(pin: &str) -> Option<String> {
    let repo = pin.split('@').next().unwrap_or(pin);
    let name = repo.rsplit(['/', '\\']).next().unwrap_or(repo);
    let size = name.strip_prefix("Qwen3-").unwrap_or(name);
    SIZES
        .iter()
        .find(|s| s.eq_ignore_ascii_case(size))
        .map(|s| (*s).to_string())
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

/// `<namespace>/vox-mens-qwen3-<size>` with the size lower-cased (`vox-mens-qwen3-0.6b`).
pub fn repo_id_for_size(namespace: &str, size: &str) -> Result<String> {
    let Some(size) = size_of_pin(size) else {
        bail!(
            "`{size}` is not a ladder size; known sizes: {}",
            SIZES.join(", ")
        );
    };
    let ns = resolve_namespace(Some(namespace))?;
    Ok(format!("{ns}/vox-mens-qwen3-{}", size.to_ascii_lowercase()))
}

/// `<namespace>/vox-mens-index`: the repo that lists every size with its status and pin.
pub fn index_repo_id(namespace: &str) -> Result<String> {
    Ok(format!(
        "{}/vox-mens-index",
        resolve_namespace(Some(namespace))?
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ladder_size_maps_to_its_repo() {
        let want = [
            ("0.6B", "me/vox-mens-qwen3-0.6b"),
            ("1.7B", "me/vox-mens-qwen3-1.7b"),
            ("4B", "me/vox-mens-qwen3-4b"),
            ("8B", "me/vox-mens-qwen3-8b"),
            ("14B", "me/vox-mens-qwen3-14b"),
            ("32B", "me/vox-mens-qwen3-32b"),
        ];
        for (size, repo) in want {
            assert_eq!(repo_id_for_size("me", size).unwrap(), repo);
        }
    }

    #[test]
    fn the_repo_follows_the_ladder_pin_not_a_hand_typed_size() {
        let overlay = crate::mens::tensor::size_select::load_ladder_overlay(None).unwrap();
        let mut seen = Vec::new();
        for rung in &overlay["qwen3_code"] {
            if !rung.methods.iter().any(|m| m == "qlora") {
                continue;
            }
            let size = size_of_pin(&rung.hf_id).expect("ladder pin names a ladder size");
            repo_id_for_size("me", &rung.hf_id).unwrap();
            seen.push(size);
        }
        for s in SIZES {
            assert!(seen.iter().any(|x| x == s), "ladder has no {s} rung");
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
}
