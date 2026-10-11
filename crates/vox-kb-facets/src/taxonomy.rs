//! SKOS-lite controlled vocabulary for the KB `topic` / `domain` facets (TAX-01, D-07).
//!
//! The prefLabel is the exact value a doc writes in frontmatter. altLabels and deprecated aliases
//! are search/lint synonyms. Pure: the caller reads the YAML file and passes the text in.

use std::collections::HashMap;

use serde::Deserialize;

use crate::Facets;

/// Repo-relative path of the committed vocabulary.
pub const TAXONOMY_REL: &str = "contracts/documentation/kb-taxonomy.v1.yaml";

/// One vocabulary entry (a domain or a topic).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Concept {
    #[serde(rename = "prefLabel")]
    pub pref_label: String,
    #[serde(rename = "altLabel", default)]
    pub alt_labels: Vec<String>,
    /// Topics name their domain here; domains must not set it.
    #[serde(default)]
    pub broader: Option<String>,
    #[serde(default)]
    pub related: Vec<String>,
    #[serde(rename = "scopeNote", default)]
    pub scope_note: Option<String>,
    #[serde(rename = "deprecatedAliases", default)]
    pub deprecated_aliases: Vec<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TaxonomyFile {
    #[serde(rename = "x-vox-version")]
    _x_vox_version: u32,
    schema_version: u32,
    domains: Vec<Concept>,
    topics: Vec<Concept>,
}

/// Validated vocabulary.
#[derive(Debug, Clone, PartialEq)]
pub struct Taxonomy {
    pub domains: Vec<Concept>,
    pub topics: Vec<Concept>,
}

/// How a label resolved.
#[derive(Debug, PartialEq)]
pub enum LabelMatch<'a> {
    Pref(&'a Concept),
    Alt(&'a Concept),
    Deprecated(&'a Concept),
    Unknown,
}

fn is_kebab(s: &str) -> bool {
    !s.is_empty()
        && s.split('-').all(|run| {
            !run.is_empty()
                && run
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

fn resolve<'a>(list: &'a [Concept], label: &str) -> LabelMatch<'a> {
    if let Some(c) = list.iter().find(|c| c.pref_label == label) {
        return LabelMatch::Pref(c);
    }
    let l = norm(label);
    if let Some(c) = list
        .iter()
        .find(|c| c.alt_labels.iter().any(|a| norm(a) == l))
    {
        return LabelMatch::Alt(c);
    }
    if let Some(c) = list
        .iter()
        .find(|c| c.deprecated_aliases.iter().any(|a| norm(a) == l))
    {
        return LabelMatch::Deprecated(c);
    }
    LabelMatch::Unknown
}

/// True when `needle` occurs in `hay` bounded by start/end or a char that is not alphanumeric,
/// `-` or `_`.
fn contains_token(hay: &str, needle: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '-' || c == '_';
    hay.match_indices(needle).any(|(i, m)| {
        !hay[..i].chars().next_back().is_some_and(word)
            && !hay[i + m.len()..].chars().next().is_some_and(word)
    })
}

impl Taxonomy {
    /// Parse and validate. Every curation problem is reported, joined with newlines.
    pub fn from_yaml_str(s: &str) -> Result<Taxonomy, String> {
        let f: TaxonomyFile = serde_yaml::from_str(s).map_err(|e| e.to_string())?;
        let mut errs: Vec<String> = Vec::new();
        if f.schema_version != 1 {
            errs.push(format!("schema_version {} is not 1", f.schema_version));
        }
        let all: Vec<(&str, &Concept)> = f
            .domains
            .iter()
            .map(|c| ("domain", c))
            .chain(f.topics.iter().map(|c| ("topic", c)))
            .collect();
        let mut prefs: HashMap<&str, usize> = HashMap::new();
        for (i, (kind, c)) in all.iter().enumerate() {
            if !is_kebab(&c.pref_label) {
                errs.push(format!(
                    "{kind} prefLabel `{}` is not lowercase kebab-case",
                    c.pref_label
                ));
            }
            if prefs.insert(&c.pref_label, i).is_some() {
                errs.push(format!("duplicate prefLabel `{}`", c.pref_label));
            }
        }
        let domain_names: Vec<&str> = f.domains.iter().map(|c| c.pref_label.as_str()).collect();
        for d in &f.domains {
            if d.broader.is_some() {
                errs.push(format!("domain `{}` must not have broader", d.pref_label));
            }
        }
        for t in &f.topics {
            match t.broader.as_deref() {
                Some(b) if domain_names.contains(&b) => {}
                Some(b) => errs.push(format!(
                    "topic `{}` broader `{b}` names no domain",
                    t.pref_label
                )),
                None => errs.push(format!("topic `{}` has no broader", t.pref_label)),
            }
        }
        // Every lowercased label (pref, alt, deprecated) must map to exactly one concept.
        let mut owner: HashMap<String, usize> = HashMap::new();
        for (i, (_, c)) in all.iter().enumerate() {
            for r in &c.related {
                if !prefs.contains_key(r.as_str()) {
                    errs.push(format!(
                        "`{}` related names unknown concept `{r}`",
                        c.pref_label
                    ));
                }
            }
            let labels = std::iter::once(&c.pref_label)
                .chain(&c.alt_labels)
                .chain(&c.deprecated_aliases);
            for label in labels {
                let l = norm(label);
                if l.is_empty() {
                    errs.push(format!("`{}` has a blank label", c.pref_label));
                    continue;
                }
                match owner.insert(l.clone(), i) {
                    Some(j) if j != i => errs.push(format!(
                        "label `{l}` resolves to both `{}` and `{}`",
                        all[j].1.pref_label, c.pref_label
                    )),
                    _ => {}
                }
            }
        }
        if errs.is_empty() {
            Ok(Taxonomy {
                domains: f.domains,
                topics: f.topics,
            })
        } else {
            Err(errs.join("\n"))
        }
    }

    pub fn resolve_topic(&self, label: &str) -> LabelMatch<'_> {
        resolve(&self.topics, label)
    }

    pub fn resolve_domain(&self, label: &str) -> LabelMatch<'_> {
        resolve(&self.domains, label)
    }

    /// Topic prefLabels in file order (did-you-mean candidates).
    pub fn topic_labels(&self) -> Vec<&str> {
        self.topics.iter().map(|c| c.pref_label.as_str()).collect()
    }

    /// Domain prefLabels in file order.
    pub fn domain_labels(&self) -> Vec<&str> {
        self.domains.iter().map(|c| c.pref_label.as_str()).collect()
    }

    /// Append one quoted facet phrase per concept whose altLabel or deprecated alias occurs in
    /// the query as a whole token. The phrase targets the card line written by
    /// [`facet_card_lines`], so only facet-tagged cards gain score. Only validated prefLabels are
    /// emitted; user text is never rewritten. An odd number of `"` returns the query unchanged.
    pub fn expand_query(&self, query: &str) -> String {
        if query.matches('"').count() % 2 == 1 {
            return query.to_string();
        }
        let q = query.to_lowercase();
        let mut out = query.to_string();
        let kinds = self
            .domains
            .iter()
            .map(|c| ("domain", c))
            .chain(self.topics.iter().map(|c| ("topic", c)));
        for (kind, c) in kinds {
            let hit = c
                .alt_labels
                .iter()
                .chain(&c.deprecated_aliases)
                .any(|l| contains_token(&q, &norm(l)));
            if hit {
                out.push_str(&format!(" \"{kind} {}\"", c.pref_label));
            }
        }
        out
    }
}

/// Card lines for the taxonomy facets: `topic: <v>` then `domain: <v>`, skipping blank values.
pub fn facet_card_lines(f: &Facets) -> Vec<String> {
    [("topic", &f.topic), ("domain", &f.domain)]
        .into_iter()
        .filter_map(|(k, v)| {
            let v = v.as_deref()?.trim();
            (!v.is_empty()).then(|| format!("{k}: {v}"))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = r#"
x-vox-version: 1
schema_version: 1
domains:
  - prefLabel: knowledge
  - prefLabel: platform
topics:
  - prefLabel: orchestration
    broader: platform
    altLabel:
      - vox-orchestrator
      - agent orchestration
    deprecatedAliases:
      - agent-dispatch
"#;

    fn fx() -> Taxonomy {
        Taxonomy::from_yaml_str(FIXTURE).unwrap()
    }

    fn err_of(yaml: &str) -> String {
        Taxonomy::from_yaml_str(yaml).unwrap_err()
    }

    #[test]
    fn resolve_topic_pref_alt_deprecated_unknown() {
        let t = fx();
        assert!(matches!(
            t.resolve_topic("orchestration"),
            LabelMatch::Pref(_)
        ));
        assert!(matches!(
            t.resolve_topic("Vox-Orchestrator"),
            LabelMatch::Alt(_)
        ));
        match t.resolve_topic("agent-dispatch") {
            LabelMatch::Deprecated(c) => assert_eq!(c.pref_label, "orchestration"),
            o => panic!("{o:?}"),
        }
        assert_eq!(t.resolve_topic("Orchestration"), LabelMatch::Unknown);
        assert_eq!(t.resolve_topic("nope"), LabelMatch::Unknown);
        assert!(matches!(t.resolve_domain("platform"), LabelMatch::Pref(_)));
        assert_eq!(t.topic_labels(), ["orchestration"]);
        assert_eq!(t.domain_labels(), ["knowledge", "platform"]);
    }

    #[test]
    fn expand_query_alt_label_appends_phrase() {
        assert_eq!(
            fx().expand_query("how does vox-orchestrator route"),
            "how does vox-orchestrator route \"topic orchestration\""
        );
    }

    #[test]
    fn expand_query_deprecated_alias_expands_to_replacement() {
        assert_eq!(
            fx().expand_query("agent-dispatch internals"),
            "agent-dispatch internals \"topic orchestration\""
        );
    }

    #[test]
    fn expand_query_multiword_label_appends_once() {
        let q = "agent orchestration vox-orchestrator";
        assert_eq!(fx().expand_query(q), format!("{q} \"topic orchestration\""));
    }

    #[test]
    fn expand_query_requires_whole_token() {
        for q in [
            "vox-orchestrator-types",
            "myvox-orchestrator",
            "vox-orchestrator_x",
            "vox-orchestrator2",
        ] {
            assert_eq!(fx().expand_query(q), q);
        }
    }

    #[test]
    fn expand_query_no_match_and_odd_quotes_are_unchanged() {
        assert_eq!(fx().expand_query("zebra"), "zebra");
        let odd = "vox-orchestrator \"open";
        assert_eq!(fx().expand_query(odd), odd);
    }

    #[test]
    fn expand_query_emits_only_pref_labels() {
        let q = "vox-orchestrator agent-dispatch \"x\" {y} OR z";
        let out = fx().expand_query(q);
        assert_eq!(out.strip_prefix(q).unwrap(), " \"topic orchestration\"");
    }

    #[test]
    fn from_yaml_rejects_unknown_key() {
        let y = FIXTURE.replace("altLabel:", "altLabels:");
        assert!(err_of(&y).contains("altLabels"));
    }

    #[test]
    fn from_yaml_rejects_non_kebab_pref_label() {
        let y = FIXTURE.replace("prefLabel: knowledge", "prefLabel: Build System");
        assert!(err_of(&y).contains("not lowercase kebab-case"));
    }

    #[test]
    fn from_yaml_rejects_duplicate_pref_label() {
        let y = FIXTURE.replace("prefLabel: platform", "prefLabel: knowledge");
        assert!(err_of(&y).contains("duplicate prefLabel"));
    }

    #[test]
    fn from_yaml_rejects_bad_or_missing_broader() {
        let y = FIXTURE.replace("broader: platform", "broader: nowhere");
        assert!(err_of(&y).contains("names no domain"));
        let y = FIXTURE.replace("    broader: platform\n", "");
        assert!(err_of(&y).contains("has no broader"));
    }

    #[test]
    fn from_yaml_rejects_unknown_related() {
        let y = FIXTURE.replace(
            "broader: platform",
            "broader: platform\n    related: [ghost]",
        );
        assert!(err_of(&y).contains("unknown concept `ghost`"));
    }

    #[test]
    fn from_yaml_rejects_ambiguous_label() {
        let y = FIXTURE.replace("- agent orchestration", "- Platform");
        assert!(err_of(&y).contains("resolves to both"));
        let y = FIXTURE.replace("- agent-dispatch", "- VOX-ORCHESTRATOR\n      - knowledge");
        assert!(err_of(&y).contains("`knowledge` resolves to both"));
    }

    #[test]
    fn from_yaml_rejects_schema_version_2() {
        let y = FIXTURE.replace("schema_version: 1", "schema_version: 2");
        assert!(err_of(&y).contains("schema_version 2"));
    }

    #[test]
    fn facet_card_lines_skips_blank_and_absent() {
        let f = Facets {
            topic: Some("orchestration".into()),
            domain: Some("platform".into()),
            ..Facets::default()
        };
        assert_eq!(
            facet_card_lines(&f),
            ["topic: orchestration", "domain: platform"]
        );
        let f = Facets {
            topic: Some("  ".into()),
            domain: Some(" x ".into()),
            ..Facets::default()
        };
        assert_eq!(facet_card_lines(&f), ["domain: x"]);
        assert!(facet_card_lines(&Facets::default()).is_empty());
    }
}
