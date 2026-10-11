//! L0 KB facet contract (KB Phase 4, D-01).
//!
//! Pure: no I/O and no hashing. Lint, indexer, MENS builder and the generated schema all read
//! facets through this crate, so a doc cannot be eligible in one consumer and ineligible in
//! another.

pub mod backlog;
pub mod frontmatter;
pub mod provenance;
pub mod summary;
pub mod taxonomy;

pub use backlog::{BACKLOG_REL, Backlog, BacklogRow};
pub use taxonomy::{Concept, LabelMatch, TAXONOMY_REL, Taxonomy, facet_card_lines};

pub use frontmatter::{frontmatter_range, frontmatter_yaml};
pub use provenance::{
    MAX_PAGE_QUOTE_WORDS, MAX_QUOTE_WORDS, is_blake3_content_hash, is_iso_date, quote_word_counts,
};
pub use summary::{
    SUMMARY_CLOSE, SUMMARY_MAX_WORDS, SUMMARY_MIN_WORDS, SUMMARY_OPEN, SummaryStatus,
    inspect_summary, is_summary_marker_line, strip_summary_blocks,
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Who wrote the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Authorship {
    Human,
    LlmAssisted,
    LlmGenerated,
}

/// How the page entered the KB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Origin {
    Authored,
    WebPromoted,
}

/// Which collection the page belongs to: Vox docs or the general library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum KbTier {
    Vox,
    Library,
}

/// Trust level of the page; same wire strings as `ContextTrustTier`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    Untrusted,
    Advisory,
    Trusted,
    SystemVerified,
}

/// Whether the page is about Vox internals or external subject matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Scope {
    VoxInternal,
    External,
}

/// Egress sensitivity; `private` pages must not leave the machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Sensitivity {
    Public,
    Private,
}

/// One entry of `sources[]`; every field is optional so a gap is a lint error, not a parse error.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Source {
    /// Source URL (`source_url` of the promoted finding).
    pub url: Option<String>,
    /// UTC fetch date, `YYYY-MM-DD`.
    pub date: Option<String>,
    /// Content hash, `blake3:` plus 64 lowercase hex characters.
    pub hash: Option<String>,
    /// Human-readable source title.
    pub title: Option<String>,
    /// Verbatim quote taken from the source (capped by the lint).
    pub quote: Option<String>,
}

/// Typed KB facets read from page frontmatter. Unknown frontmatter keys are ignored.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Facets {
    /// Taxonomy topic id.
    pub topic: Option<String>,
    /// Taxonomy domain id.
    pub domain: Option<String>,
    /// Collection the page belongs to.
    pub kb_tier: Option<KbTier>,
    /// Trust level of the page.
    pub trust_tier: Option<TrustTier>,
    /// Vox-internal or external subject matter.
    pub scope: Option<Scope>,
    /// Date (`YYYY-MM-DD`) by which the page must be re-reviewed.
    pub review_by: Option<String>,
    /// Date (`YYYY-MM-DD`) the content was last verified.
    pub verified_at: Option<String>,
    /// Provenance for web-derived content.
    pub sources: Vec<Source>,
    /// Who wrote the page.
    pub authorship: Option<Authorship>,
    /// Free-string model name when authorship involves an LLM.
    pub authorship_model: Option<String>,
    /// How the page entered the KB.
    pub origin: Option<Origin>,
    /// Person who reviewed and attests LLM or web-promoted content.
    pub human_attested_by: Option<String>,
    /// Page paths this page replaces.
    pub supersedes: Vec<String>,
    /// Page path that replaces this page.
    pub superseded_by: Option<String>,
    /// Egress sensitivity.
    pub sensitivity: Option<Sensitivity>,
    /// Explicit MENS training opt-out or opt-in; defaults to true.
    pub training_eligible: Option<bool>,
}

/// Parse facets from a whole document. No frontmatter, an unterminated block or an empty block
/// is `Facets::default()` (grandfathered, D-05); an invalid facet value is an `Err` naming it.
pub fn parse_facets(doc_text: &str) -> Result<Facets, String> {
    match frontmatter_yaml(doc_text) {
        Some(y) if !y.trim().is_empty() => {
            serde_yaml::from_str::<Facets>(y).map_err(|e| e.to_string())
        }
        _ => Ok(Facets::default()),
    }
}

/// True when the page is forced training-ineligible: web-promoted or LLM-authored content with
/// no (non-blank) `human_attested_by`.
#[must_use]
pub fn forced_training_ineligible(f: &Facets) -> bool {
    let risky = f.origin == Some(Origin::WebPromoted)
        || matches!(
            f.authorship,
            Some(Authorship::LlmAssisted | Authorship::LlmGenerated)
        );
    let attested = f
        .human_attested_by
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());
    risky && !attested
}

/// The one shared MENS eligibility rule (D-05, D-06): not forced-ineligible and the flag, which
/// defaults to true.
#[must_use]
pub fn effective_training_eligible(f: &Facets) -> bool {
    !forced_training_ineligible(f) && f.training_eligible.unwrap_or(true)
}

/// Draft-07 JSON Schema for [`Facets`] (the GUI type and the 04-03 golden read this).
#[must_use]
pub fn facet_json_schema() -> serde_json::Value {
    let settings = schemars::generate::SchemaSettings::draft07();
    let root = settings.into_generator().into_root_schema_for::<Facets>();
    let mut v = serde_json::to_value(&root).expect("Facets schema serializes");
    let obj = v.as_object_mut().expect("schema root is an object");
    obj.insert(
        "$id".into(),
        "https://voxlang.org/schemas/kb-facets.v1.schema.json".into(),
    );
    obj.insert("x-vox-version".into(), 1.into());
    obj.insert("title".into(), "KbFacets".into());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(fm: &str) -> String {
        format!("---\n{fm}\n---\n# T\n")
    }
    fn eff(fm: &str) -> bool {
        effective_training_eligible(&parse_facets(&doc(fm)).unwrap())
    }

    #[test]
    fn effective_training_web_promoted_unattested_is_false_even_if_flag_true() {
        assert!(!eff("origin: web-promoted\ntraining_eligible: true"));
        assert!(eff(
            "origin: web-promoted\nhuman_attested_by: alice\ntraining_eligible: true"
        ));
    }

    #[test]
    fn effective_training_llm_authorship_branches() {
        assert!(!eff("authorship: llm-assisted"));
        assert!(!eff("authorship: llm-generated"));
        assert!(eff("authorship: llm-assisted\nhuman_attested_by: alice"));
        assert!(!eff(
            "authorship: llm-assisted\nhuman_attested_by: alice\ntraining_eligible: false"
        ));
        assert!(eff("authorship: human"));
    }

    #[test]
    fn effective_training_defaults_grandfathered() {
        assert!(eff("title: X"));
        assert!(!eff("training_eligible: false"));
        assert!(effective_training_eligible(&Facets::default()));
    }

    #[test]
    fn effective_training_blank_attestation_counts_as_absent() {
        assert!(!eff("origin: web-promoted\nhuman_attested_by: \"\""));
        assert!(!eff(
            "authorship: llm-generated\nhuman_attested_by: \"   \""
        ));
    }

    #[test]
    fn parse_facets_empty_shapes_are_default() {
        assert_eq!(
            parse_facets("# no frontmatter\n").unwrap(),
            Facets::default()
        );
        assert_eq!(
            parse_facets("---\ntopic: x\n# unterminated\n").unwrap(),
            Facets::default()
        );
        assert_eq!(parse_facets("---\n---\n# T\n").unwrap(), Facets::default());
        assert_eq!(
            parse_facets("---\n  \n---\n# T\n").unwrap(),
            Facets::default()
        );
    }

    #[test]
    fn parse_facets_ignores_unknown_keys() {
        let t = doc(
            "title: T\ncategory: C\nstatus: s\nschema_type: x\nrelated: [a, b]\nsort_order: 3\ntraining_rationale: why\ntopic: t",
        );
        assert_eq!(parse_facets(&t).unwrap().topic.as_deref(), Some("t"));
    }

    #[test]
    fn parse_facets_invalid_enum_names_the_field() {
        let err = parse_facets(&doc("authorship: robot")).unwrap_err();
        assert!(err.contains("authorship"), "{err}");
    }

    #[test]
    fn parse_facets_reads_all_fields() {
        let t = doc(
            "topic: a\ndomain: b\nkb_tier: library\ntrust_tier: system_verified\nscope: vox-internal\nreview_by: 2026-10-10\nverified_at: 2026-10-10\nsources:\n  - url: https://x.y\n    hash: h\nauthorship: llm-assisted\nauthorship_model: m\norigin: web-promoted\nhuman_attested_by: al\nsupersedes: [p]\nsuperseded_by: q\nsensitivity: private\ntraining_eligible: false",
        );
        let f = parse_facets(&t).unwrap();
        assert_eq!(f.kb_tier, Some(KbTier::Library));
        assert_eq!(f.trust_tier, Some(TrustTier::SystemVerified));
        assert_eq!(f.scope, Some(Scope::VoxInternal));
        assert_eq!(f.sources.len(), 1);
        assert_eq!(f.sources[0].url.as_deref(), Some("https://x.y"));
        assert_eq!(f.sensitivity, Some(Sensitivity::Private));
        assert_eq!(f.supersedes, vec!["p".to_string()]);
    }

    #[test]
    fn facet_json_schema_shape() {
        let s = facet_json_schema();
        assert!(s["$schema"].as_str().unwrap().contains("draft-07"));
        assert_eq!(
            s["$id"],
            "https://voxlang.org/schemas/kb-facets.v1.schema.json"
        );
        assert_eq!(s["x-vox-version"], 1);
        assert_eq!(s["title"], "KbFacets");
        let mut keys: Vec<&str> = s["properties"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        let mut want = vec![
            "topic",
            "domain",
            "kb_tier",
            "trust_tier",
            "scope",
            "review_by",
            "verified_at",
            "sources",
            "authorship",
            "authorship_model",
            "origin",
            "human_attested_by",
            "supersedes",
            "superseded_by",
            "sensitivity",
            "training_eligible",
        ];
        want.sort_unstable();
        assert_eq!(keys, want);
    }
}
