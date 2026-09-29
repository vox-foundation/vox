//! Model families: the version-free identity of a model line (e.g. every
//! `anthropic/claude-opus-*`), so routing can prefer the newest member.

use std::collections::HashMap;

use crate::models::ModelSpec;

/// Qualifier tokens that mark a release stage, not a different family.
const QUALIFIERS: &[&str] = &["preview", "beta", "exp", "experimental", "latest"];

fn is_size_token(tok: &str) -> bool {
    let body = tok.strip_suffix('b').unwrap_or("").trim_start_matches('a');
    !body.is_empty()
        && body.chars().all(|c| c.is_ascii_digit() || c == '.')
        && body.chars().any(|c| c.is_ascii_digit())
}

/// Version-free family id: `org/name-words[:free]`, lowercased.
///
/// Per `-`-separated token of the name: pure numbers/dots (`4.6`, `20260101`, `0731`) and
/// `v4`-style markers are dropped; release-stage words (`preview`, `beta`, `exp`, `latest`)
/// are dropped; parameter sizes (`8b`, `70b`, `a22b`) are KEPT, because a smaller size is a
/// different cost point, not an older version; other mixed tokens keep only their letters
/// (`qwen3` → `qwen`, `k2.6` → `k`). A `:free` variant is its own family (a different price
/// pool); other `:variant` suffixes are dropped.
// ponytail: heuristic slug parse; OpenRouter publishes `~vendor/family-latest` alias entries
// (with `alias_target`) for ~18 families — prefer those if a family key ever needs to be authoritative.
#[must_use]
pub fn family_key(slug: &str) -> String {
    let slug = slug.to_ascii_lowercase();
    let (base, variant) = slug.split_once(':').unwrap_or((slug.as_str(), ""));
    let (org, name) = base.split_once('/').unwrap_or(("", base));
    let words: Vec<String> = name
        .split('-')
        .filter_map(|tok| {
            if tok.is_empty() || QUALIFIERS.contains(&tok) {
                return None;
            }
            if !tok.chars().any(|c| c.is_ascii_digit()) || is_size_token(tok) {
                return Some(tok.to_string());
            }
            let is_version = tok.chars().all(|c| c.is_ascii_digit() || c == '.');
            let is_v_marker = tok.strip_prefix('v').is_some_and(|r| {
                !r.is_empty() && r.chars().all(|c| c.is_ascii_digit() || c == '.')
            });
            if is_version || is_v_marker {
                return None;
            }
            let letters: String = tok.chars().filter(|c| c.is_ascii_alphabetic()).collect();
            (!letters.is_empty()).then_some(letters)
        })
        .collect();
    let mut key = if org.is_empty() {
        words.join("-")
    } else {
        format!("{org}/{}", words.join("-"))
    };
    if variant == "free" {
        key.push_str(":free");
    }
    key
}

/// Every run of digits in the slug, in order (`claude-opus-4.8` → `[4, 8]`).
#[must_use]
pub fn version_tuple(slug: &str) -> Vec<u32> {
    slug.split(|c: char| !c.is_ascii_digit())
        .filter(|s| !s.is_empty())
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Newest `(released_at, version)` per family, over dated specs only.
#[must_use]
pub fn newest_per_family<'a>(
    specs: impl IntoIterator<Item = &'a ModelSpec>,
) -> HashMap<String, (u64, Vec<u32>)> {
    let mut newest: HashMap<String, (u64, Vec<u32>)> = HashMap::new();
    for m in specs {
        let Some(at) = m.capabilities.released_at else {
            continue;
        };
        let key = family_key(&m.id);
        let cand = (at, version_tuple(&m.id));
        match newest.get(&key) {
            Some(cur) if *cur >= cand => {}
            _ => {
                newest.insert(key, cand);
            }
        }
    }
    newest
}

/// True when a newer dated member of the same family is in `newest`. Undated
/// specs (bootstrap, local MENS) are never superseded.
#[must_use]
pub fn is_superseded(m: &ModelSpec, newest: &HashMap<String, (u64, Vec<u32>)>) -> bool {
    if m.capabilities.is_alias_target {
        return false;
    }
    let Some(at) = m.capabilities.released_at else {
        return false;
    };
    newest
        .get(&family_key(&m.id))
        .is_some_and(|best| *best > (at, version_tuple(&m.id)))
}

/// Family key for joining a model to its family's benchmark: a direct-provider id has no `org/`
/// prefix (`widget-5-5-…`), so its dated `canonical_slug` is used; a `:free` variant is the same
/// weights as its paid sibling, so it shares the paid family's benchmark.
#[must_use]
pub fn join_key(m: &ModelSpec) -> String {
    let slug = if m.id.contains('/') || m.canonical_slug.is_empty() {
        &m.id
    } else {
        &m.canonical_slug
    };
    family_key(slug).trim_end_matches(":free").to_string()
}

/// The newest benchmarked member of each family (by [`join_key`]): `(index, id)`.
#[must_use]
pub fn family_benchmarks<'a>(
    specs: impl IntoIterator<Item = &'a ModelSpec>,
) -> HashMap<String, (f32, String)> {
    let mut best: HashMap<String, ((u64, Vec<u32>), f32, String)> = HashMap::new();
    for m in specs {
        let Some(index) = m.capabilities.intelligence_index.filter(|i| i.is_finite()) else {
            continue;
        };
        let key = join_key(m);
        let rank = (
            m.capabilities.released_at.unwrap_or(0),
            version_tuple(&m.id),
        );
        match best.get(&key) {
            Some((cur, _, _)) if *cur >= rank => {}
            _ => {
                best.insert(key, (rank, index, m.id.clone()));
            }
        }
    }
    best.into_iter()
        .map(|(k, (_, index, id))| (k, (index, id)))
        .collect()
}

impl crate::models::ModelRegistry {
    /// True when the registry holds a newer dated member of `m`'s family.
    #[must_use]
    pub fn is_superseded_in_registry(&self, m: &ModelSpec) -> bool {
        // ponytail: O(models) per call; only used on the premium-alias path.
        is_superseded(m, &newest_per_family(self.models_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ModelCapabilities, ModelSpec, ProviderType};

    fn dated(slug: &str, released_at: Option<u64>) -> ModelSpec {
        ModelSpec {
            id: slug.into(),
            canonical_slug: slug.into(),
            provider: "test".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 8192,
            cost_per_1k: 1.0,
            cost_per_1k_input: 1.0,
            cost_per_1k_output: 1.0,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![],
            capabilities: ModelCapabilities {
                released_at,
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: crate::models::spec::PricingSource::Bootstrap,
            supported_parameters: vec![],
        }
    }

    #[test]
    fn family_key_strips_versions_across_naming_shapes() {
        assert_eq!(
            family_key("anthropic/claude-opus-4.8"),
            "anthropic/claude-opus"
        );
        assert_eq!(
            family_key("anthropic/claude-sonnet-4.6"),
            "anthropic/claude-sonnet"
        );
        assert_eq!(
            family_key("anthropic/claude-3-7-sonnet"),
            "anthropic/claude-sonnet"
        );
        assert_eq!(
            family_key("deepseek/deepseek-v4-flash"),
            "deepseek/deepseek-flash"
        );
        assert_eq!(
            family_key("deepseek/deepseek-v4.1-flash"),
            "deepseek/deepseek-flash"
        );
        assert_eq!(
            family_key("deepseek/deepseek-v4-flash-0731"),
            "deepseek/deepseek-flash"
        );
        assert_eq!(family_key("openai/gpt-6-luna"), "openai/gpt-luna");
        assert_eq!(family_key("openai/gpt-5.6-luna"), "openai/gpt-luna");
        assert_eq!(family_key("google/gemini-3.8-flash"), "google/gemini-flash");
        assert_eq!(
            family_key("google/gemini-3.1-flash-lite-preview"),
            "google/gemini-flash-lite"
        );
        assert_eq!(
            family_key("Anthropic/Claude-Opus-5.5"),
            "anthropic/claude-opus"
        );
    }

    #[test]
    fn family_key_keeps_free_variants_and_sizes_apart() {
        // :free and parameter sizes are distinct families
        assert_eq!(family_key("qwen/qwen3.8-27b:free"), "qwen/qwen-27b:free");
        assert_eq!(family_key("qwen/qwen3.8-27b"), "qwen/qwen-27b");
        assert_eq!(family_key("qwen/qwen3.8-max-0902"), "qwen/qwen-max");
        assert_eq!(family_key("qwen/qwen3.8-max-prime"), "qwen/qwen-max-prime");
        assert_ne!(
            family_key("meta-llama/llama-3.1-8b"),
            family_key("meta-llama/llama-3.1-70b")
        );
        assert_eq!(family_key("qwen/qwen3-235b-a22b"), "qwen/qwen-235b-a22b");
        assert_eq!(family_key("z-ai/glm-5.3-flash"), "z-ai/glm-flash");
        assert_eq!(family_key("z-ai/glm-5.3"), "z-ai/glm");
    }

    #[test]
    fn version_tuple_orders_numeric_parts() {
        assert_eq!(version_tuple("anthropic/claude-opus-4.8"), vec![4, 8]);
        assert!(
            version_tuple("anthropic/claude-opus-5.5") > version_tuple("anthropic/claude-opus-4.8")
        );
        assert_eq!(version_tuple("acme/plain-model"), Vec::<u32>::new());
    }

    #[test]
    fn older_dated_member_is_superseded_newest_is_not() {
        let old = dated("anthropic/claude-opus-4.8", Some(1_700_000_000));
        let new = dated("anthropic/claude-opus-5.5", Some(1_760_000_000));
        let other = dated("deepseek/deepseek-v4-flash", Some(1_650_000_000));
        let newest = newest_per_family([&old, &new, &other]);
        assert!(is_superseded(&old, &newest));
        assert!(!is_superseded(&new, &newest));
        assert!(
            !is_superseded(&other, &newest),
            "sole member of its family is never superseded"
        );
    }

    #[test]
    fn free_variant_is_not_superseded_by_a_newer_paid_member() {
        let free = dated("qwen/qwen3.8-27b:free", Some(1_700_000_000));
        let paid = dated("qwen/qwen3.9-27b", Some(1_760_000_000));
        let newest = newest_per_family([&free, &paid]);
        assert!(!is_superseded(&free, &newest));
    }

    #[test]
    fn undated_specs_are_never_superseded_and_never_supersede() {
        let undated_old_name = dated("anthropic/claude-opus-4.8", None);
        let dated_new = dated("anthropic/claude-opus-5.5", Some(1_760_000_000));
        let newest = newest_per_family([&undated_old_name, &dated_new]);
        assert!(
            !is_superseded(&undated_old_name, &newest),
            "no date, no verdict"
        );
        assert!(newest_per_family([&undated_old_name]).is_empty());
    }

    #[test]
    fn same_date_tie_breaks_on_version() {
        let a = dated("google/gemini-3-flash", Some(1_750_000_000));
        let b = dated("google/gemini-3.1-flash", Some(1_750_000_000));
        let newest = newest_per_family([&a, &b]);
        assert!(is_superseded(&a, &newest));
        assert!(!is_superseded(&b, &newest));
    }

    #[test]
    fn registry_reports_a_superseded_member() {
        use crate::models::ModelRegistry;
        let mut r = ModelRegistry::default();
        r.register(dated("acme/widget-4.8", Some(1_700_000_000)));
        r.register(dated("acme/widget-5.5", Some(1_760_000_000)));
        assert!(r.is_superseded_in_registry(&dated("acme/widget-4.8", Some(1_700_000_000))));
        assert!(!r.is_superseded_in_registry(&dated("acme/widget-5.5", Some(1_760_000_000))));
    }

    #[test]
    fn an_alias_target_is_never_superseded() {
        let mut old = dated("acme/widget-4.8", Some(1_700_000_000));
        old.capabilities.is_alias_target = true;
        let new = dated("acme/widget-5.5", Some(1_760_000_000));
        let newest = newest_per_family([&old, &new]);
        assert!(!is_superseded(&old, &newest));
    }

    #[test]
    fn join_key_uses_the_canonical_slug_for_direct_ids_and_ignores_free() {
        let mut direct = dated("widget-5-5-20260101", Some(1));
        direct.canonical_slug = "acme/widget-5-5-20260101".into();
        assert_eq!(join_key(&direct), "acme/widget");
        assert_eq!(
            join_key(&dated("acme/widget-5.5:free", None)),
            "acme/widget"
        );
        assert_eq!(join_key(&dated("acme/widget-4.8", None)), "acme/widget");
    }

    #[test]
    fn family_benchmarks_pick_the_newest_benchmarked_member() {
        let mut a = dated("acme/widget-4.8", Some(1_700_000_000));
        a.capabilities.intelligence_index = Some(30.0);
        let mut b = dated("acme/widget-5.0", Some(1_750_000_000));
        b.capabilities.intelligence_index = Some(38.0);
        let c = dated("acme/widget-5.5", Some(1_760_000_000));
        let fb = family_benchmarks([&a, &b, &c]);
        assert_eq!(
            fb.get("acme/widget"),
            Some(&(38.0, "acme/widget-5.0".to_string()))
        );
    }
}
