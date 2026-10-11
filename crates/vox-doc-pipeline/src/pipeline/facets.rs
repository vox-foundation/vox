//! KB facet lint (Phase 4): rule ids are stable `facet-*` strings and double as backlog keys.

use std::path::Path;

use vox_kb_facets::{
    Authorship, BACKLOG_REL, Backlog, Facets, LabelMatch, MAX_PAGE_QUOTE_WORDS, MAX_QUOTE_WORDS,
    Origin, SUMMARY_MAX_WORDS, SUMMARY_MIN_WORDS, SummaryStatus, TAXONOMY_REL, Taxonomy,
    forced_training_ineligible, inspect_summary, is_blake3_content_hash, is_iso_date,
    is_valid_attestation, parse_facets, quote_word_counts,
};

use super::lint::suggest;
use super::types::{FacetIssue, LintError, LintKind};

/// Every rule id a backlog row may excuse (all but the two meta ids).
pub(crate) const BACKLOGGABLE_RULE_IDS: &[&str] = &[
    "facet-parse-error",
    "facet-missing-topic",
    "facet-missing-domain",
    "facet-unknown-topic",
    "facet-unknown-domain",
    "facet-deprecated-label",
    "facet-domain-mismatch",
    "facet-missing-summary",
    "facet-summary-placement",
    "facet-summary-word-count",
    "facet-missing-authorship",
    "facet-missing-authorship-model",
    "facet-training-eligible-contradiction",
    "facet-missing-sources",
    "facet-source-field",
    "facet-quote-over-cap",
    "facet-invalid-date",
    "facet-invalid-attestation",
];

fn hint(h: &Option<String>) -> String {
    h.as_ref()
        .map_or_else(String::new, |h| format!("; use prefLabel {h:?}"))
}

impl FacetIssue {
    pub(crate) fn rule_id(&self) -> &'static str {
        match self {
            Self::ContractUnreadable { .. } => "facet-contract-unreadable",
            Self::ParseError { .. } => "facet-parse-error",
            Self::MissingTopic => "facet-missing-topic",
            Self::MissingDomain => "facet-missing-domain",
            Self::UnknownTopic { .. } => "facet-unknown-topic",
            Self::UnknownDomain { .. } => "facet-unknown-domain",
            Self::DeprecatedLabel { .. } => "facet-deprecated-label",
            Self::DomainMismatch { .. } => "facet-domain-mismatch",
            Self::MissingSummary => "facet-missing-summary",
            Self::SummaryPlacement { .. } => "facet-summary-placement",
            Self::SummaryWordCount { .. } => "facet-summary-word-count",
            Self::MissingAuthorship => "facet-missing-authorship",
            Self::MissingAuthorshipModel => "facet-missing-authorship-model",
            Self::TrainingEligibleContradiction => "facet-training-eligible-contradiction",
            Self::MissingSources => "facet-missing-sources",
            Self::SourceField { .. } => "facet-source-field",
            Self::QuoteOverCap { .. } => "facet-quote-over-cap",
            Self::InvalidDate { .. } => "facet-invalid-date",
            Self::InvalidAttestation { .. } => "facet-invalid-attestation",
            Self::BacklogStale { .. } => "facet-backlog-stale",
        }
    }

    pub(crate) fn message(&self) -> String {
        match self {
            Self::ContractUnreadable { path, error } => {
                format!("facet contract {path} is missing or invalid: {error}")
            }
            Self::ParseError { msg } => format!("facet frontmatter does not parse: {msg}"),
            Self::MissingTopic => "frontmatter is missing `topic:`".into(),
            Self::MissingDomain => "frontmatter is missing `domain:`".into(),
            Self::UnknownTopic { value, hint: h } => {
                format!("topic {value:?} is not a taxonomy prefLabel{}", hint(h))
            }
            Self::UnknownDomain { value, hint: h } => {
                format!("domain {value:?} is not a taxonomy prefLabel{}", hint(h))
            }
            Self::DeprecatedLabel {
                field,
                value,
                replaced_by,
            } => format!("{field} {value:?} is a retired alias; use prefLabel {replaced_by:?}"),
            Self::DomainMismatch {
                topic,
                domain,
                expected,
            } => format!("topic {topic:?} belongs to domain {expected:?}, not {domain:?}"),
            Self::MissingSummary => "no complete <!-- kb:summary --> block".into(),
            Self::SummaryPlacement { reason } => format!("summary block misplaced: {reason}"),
            Self::SummaryWordCount { words } => {
                format!("summary is {words} words; must be {SUMMARY_MIN_WORDS}-{SUMMARY_MAX_WORDS}")
            }
            Self::MissingAuthorship => "frontmatter is missing `authorship:`".into(),
            Self::MissingAuthorshipModel => {
                "LLM authorship requires a non-blank `authorship_model:`".into()
            }
            Self::TrainingEligibleContradiction => {
                "`training_eligible: true` contradicts the page: it is forced MENS-ineligible \
                 (LLM-authored or web-promoted with no human attestation)"
                    .into()
            }
            Self::MissingSources => "web-promoted page has no `sources:` entries".into(),
            Self::SourceField {
                index,
                field,
                problem,
            } => {
                format!("sources[{index}].{field}: {problem}")
            }
            Self::QuoteOverCap { words, cap, scope } => {
                format!("verbatim quoting over the {scope} cap: {words} words > {cap}")
            }
            Self::InvalidDate { field, value } => {
                format!("{field} {value:?} is not a real YYYY-MM-DD date")
            }
            Self::InvalidAttestation { value } => format!(
                "human_attested_by {value:?} is not a human attestation (blank, a placeholder or an AI name)"
            ),
            Self::BacklogStale { rule } => format!(
                "backlog excuses {rule} but the doc no longer violates it; run --tighten-facet-backlog"
            ),
        }
    }

    pub(crate) fn workflow(&self) -> &'static str {
        match self {
            Self::ContractUnreadable { .. } => {
                "restore the taxonomy and facet backlog under contracts/documentation/"
            }
            Self::ParseError { .. } => "fix the facet value named in the message",
            Self::MissingTopic
            | Self::MissingDomain
            | Self::UnknownTopic { .. }
            | Self::UnknownDomain { .. }
            | Self::DeprecatedLabel { .. }
            | Self::DomainMismatch { .. } => {
                "use a prefLabel from contracts/documentation/kb-taxonomy.v1.yaml"
            }
            Self::MissingSummary
            | Self::SummaryPlacement { .. }
            | Self::SummaryWordCount { .. } => {
                "put a 40-120 word <!-- kb:summary --> block directly under the H1"
            }
            Self::MissingAuthorship => "add `authorship: human | llm-assisted | llm-generated`",
            Self::MissingAuthorshipModel => "add authorship_model",
            Self::TrainingEligibleContradiction => {
                "remove training_eligible: true or add human_attested_by after human review"
            }
            Self::MissingSources | Self::SourceField { .. } => {
                "add url, date (YYYY-MM-DD) and hash (blake3:<hex>) to every source"
            }
            Self::QuoteOverCap { .. } => "shorten or paraphrase the quote",
            Self::InvalidDate { .. } => "use YYYY-MM-DD",
            Self::InvalidAttestation { .. } => {
                "name the human reviewer in human_attested_by, or remove the key"
            }
            Self::BacklogStale { .. } => "run --tighten-facet-backlog to drop the stale excuse",
        }
    }

    pub(crate) fn is_hard(&self) -> bool {
        !matches!(self, Self::BacklogStale { .. })
    }
}

/// Taxonomy and backlog, loaded once per run (before the parallel file pass).
pub(crate) struct FacetCtx {
    pub(crate) taxonomy: Taxonomy,
    pub(crate) backlog: Backlog,
}

impl FacetCtx {
    /// A missing or invalid contract is an error, never "no rules apply" (fail closed).
    pub(crate) fn load(repo_root: &Path) -> Result<FacetCtx, FacetIssue> {
        let read = |rel: &str| {
            vox_bounded_fs::read_utf8_path_capped(&repo_root.join(rel)).map_err(|e| {
                FacetIssue::ContractUnreadable {
                    path: rel.into(),
                    error: e.to_string(),
                }
            })
        };
        let bad = |rel: &str, error: String| FacetIssue::ContractUnreadable {
            path: rel.into(),
            error,
        };
        let taxonomy =
            Taxonomy::from_yaml_str(&read(TAXONOMY_REL)?).map_err(|e| bad(TAXONOMY_REL, e))?;
        let backlog =
            Backlog::from_yaml_str(&read(BACKLOG_REL)?).map_err(|e| bad(BACKLOG_REL, e))?;
        for r in &backlog.docs {
            if let Some(id) = r
                .rules
                .iter()
                .find(|id| !BACKLOGGABLE_RULE_IDS.contains(&id.as_str()))
            {
                return Err(bad(
                    BACKLOG_REL,
                    format!("{}: unknown rule id {id:?}", r.path),
                ));
            }
        }
        Ok(FacetCtx { taxonomy, backlog })
    }
}

/// Repo-relative `docs/src/**/*.md` path (`/` separators) the facet rules apply to; `None` for
/// archive, generated, `.mdx` and out-of-tree paths.
pub(crate) fn facet_lint_rel_path(path: &Path, repo_root: &Path) -> Option<String> {
    let full = path.to_string_lossy().replace('\\', "/");
    let rel = match path.strip_prefix(repo_root) {
        Ok(r) => r.to_string_lossy().replace('\\', "/"),
        // Drive casing or extra prefix segments: anchor on the `docs/src/` segment.
        Err(_) => full[full.find("docs/src/")?..].to_string(),
    };
    let rel = rel.trim_start_matches("./");
    let ok = rel.starts_with("docs/src/")
        && rel.ends_with(".md")
        && !rel.ends_with(".generated.md")
        && !rel.split('/').any(|seg| seg == "archive");
    ok.then(|| rel.to_string())
}

fn nonblank(v: &Option<String>) -> Option<&str> {
    v.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

/// Topic and domain presence, prefLabel resolution (D-07) and topic-to-domain agreement.
fn label_issues(f: &Facets, tax: &Taxonomy, out: &mut Vec<(usize, FacetIssue)>) {
    let (topic, domain) = (nonblank(&f.topic), nonblank(&f.domain));
    let mut topic_ok = None;
    let mut domain_ok = None;
    match topic.map(|v| (v, tax.resolve_topic(v))) {
        None => out.push((1, FacetIssue::MissingTopic)),
        Some((_, LabelMatch::Pref(c))) => topic_ok = Some(c),
        Some((v, m)) => out.push((1, unresolved("topic", v, &m, &tax.topic_labels()))),
    }
    match domain.map(|v| (v, tax.resolve_domain(v))) {
        None => out.push((1, FacetIssue::MissingDomain)),
        Some((_, LabelMatch::Pref(c))) => domain_ok = Some(c),
        Some((v, m)) => out.push((1, unresolved("domain", v, &m, &tax.domain_labels()))),
    }
    if let (Some(t), Some(d)) = (topic_ok, domain_ok)
        && let Some(expected) = t.broader.as_deref().filter(|b| *b != d.pref_label)
    {
        out.push((
            1,
            FacetIssue::DomainMismatch {
                topic: t.pref_label.clone(),
                domain: d.pref_label.clone(),
                expected: expected.into(),
            },
        ));
    }
}

/// The issue for a topic/domain value that is not a prefLabel (D-07).
fn unresolved(field: &'static str, v: &str, m: &LabelMatch<'_>, labels: &[&str]) -> FacetIssue {
    let value = v.to_string();
    let hint = match m {
        LabelMatch::Alt(c) => Some(c.pref_label.clone()),
        _ => suggest(v, labels).map(str::to_string),
    };
    match (m, field) {
        (LabelMatch::Deprecated(c), _) => FacetIssue::DeprecatedLabel {
            field,
            value,
            replaced_by: c.pref_label.clone(),
        },
        (_, "topic") => FacetIssue::UnknownTopic { value, hint },
        _ => FacetIssue::UnknownDomain { value, hint },
    }
}

fn summary_issue(doc_text: &str) -> Option<(usize, FacetIssue)> {
    match inspect_summary(doc_text) {
        SummaryStatus::Missing | SummaryStatus::Unterminated => {
            Some((1, FacetIssue::MissingSummary))
        }
        SummaryStatus::Multiple => Some((
            1,
            FacetIssue::SummaryPlacement {
                reason: "more than one summary block".into(),
            },
        )),
        SummaryStatus::Misplaced { line } => Some((
            line,
            FacetIssue::SummaryPlacement {
                reason: "the block must be the first content after the H1".into(),
            },
        )),
        SummaryStatus::Present { words, line }
            if !(SUMMARY_MIN_WORDS..=SUMMARY_MAX_WORDS).contains(&words) =>
        {
            Some((line, FacetIssue::SummaryWordCount { words }))
        }
        SummaryStatus::Present { .. } => None,
    }
}

/// Authorship, eligibility, dates and provenance rules.
fn provenance_issues(doc_text: &str, f: &Facets, out: &mut Vec<(usize, FacetIssue)>) {
    if f.authorship.is_none() {
        out.push((1, FacetIssue::MissingAuthorship));
    }
    if matches!(
        f.authorship,
        Some(Authorship::LlmAssisted | Authorship::LlmGenerated)
    ) && nonblank(&f.authorship_model).is_none()
    {
        out.push((1, FacetIssue::MissingAuthorshipModel));
    }
    if let Some(v) = f
        .human_attested_by
        .as_deref()
        .filter(|v| !is_valid_attestation(v))
    {
        out.push((1, FacetIssue::InvalidAttestation { value: v.into() }));
    }
    if f.training_eligible == Some(true) && forced_training_ineligible(f) {
        out.push((1, FacetIssue::TrainingEligibleContradiction));
    }
    for (field, v) in [("review_by", &f.review_by), ("verified_at", &f.verified_at)] {
        if let Some(value) = nonblank(v).filter(|s| !is_iso_date(s)) {
            out.push((
                1,
                FacetIssue::InvalidDate {
                    field,
                    value: value.into(),
                },
            ));
        }
    }
    let promoted = f.origin == Some(Origin::WebPromoted);
    if promoted && f.sources.is_empty() {
        out.push((1, FacetIssue::MissingSources));
    }
    for (index, s) in f.sources.iter().enumerate() {
        let fields: [(&'static str, &Option<String>, fn(&str) -> bool, &str); 3] = [
            ("url", &s.url, |_| true, ""),
            ("date", &s.date, is_iso_date, "not a real YYYY-MM-DD date"),
            (
                "hash",
                &s.hash,
                is_blake3_content_hash,
                "not blake3:<64 hex>",
            ),
        ];
        for (field, v, valid, bad) in fields {
            let problem = match nonblank(v) {
                Some(x) if !valid(x) => bad,
                None if promoted => "missing",
                _ => continue,
            };
            out.push((
                1,
                FacetIssue::SourceField {
                    index,
                    field,
                    problem: problem.into(),
                },
            ));
        }
    }
    if promoted {
        let counts = quote_word_counts(doc_text, &f.sources);
        for &words in counts.iter().filter(|&&w| w > MAX_QUOTE_WORDS) {
            out.push((
                1,
                FacetIssue::QuoteOverCap {
                    words,
                    cap: MAX_QUOTE_WORDS,
                    scope: "quote",
                },
            ));
        }
        let total: usize = counts.iter().sum();
        if total > MAX_PAGE_QUOTE_WORDS {
            out.push((
                1,
                FacetIssue::QuoteOverCap {
                    words: total,
                    cap: MAX_PAGE_QUOTE_WORDS,
                    scope: "page",
                },
            ));
        }
    }
}

/// Pure facet check of one document: `(1-based line, issue)` pairs.
pub(crate) fn facet_issues(doc_text: &str, tax: &Taxonomy) -> Vec<(usize, FacetIssue)> {
    let f = match parse_facets(doc_text) {
        Ok(f) => f,
        Err(msg) => return vec![(1, FacetIssue::ParseError { msg })],
    };
    let mut out = Vec::new();
    label_issues(&f, tax, &mut out);
    out.extend(summary_issue(doc_text));
    provenance_issues(doc_text, &f, &mut out);
    out
}

/// Lint one file's facets: drop issues the backlog excuses for this doc, warn on stale excuses.
pub(crate) fn lint_facets(
    path: &Path,
    content: &str,
    repo_root: &Path,
    ctx: &FacetCtx,
    errors: &mut Vec<LintError>,
) {
    let Some(rel) = facet_lint_rel_path(path, repo_root) else {
        return;
    };
    let issues = facet_issues(content, &ctx.taxonomy);
    let excused = ctx.backlog.rules_for(&rel).unwrap_or(&[]);
    let mut push = |line: usize, issue: FacetIssue| {
        errors.push(LintError {
            file: path.to_owned(),
            line,
            kind: LintKind::Facet(issue),
        });
    };
    for rule in excused {
        if !issues.iter().any(|(_, i)| i.rule_id() == rule) {
            push(0, FacetIssue::BacklogStale { rule: rule.clone() });
        }
    }
    for (line, issue) in issues {
        if !excused.iter().any(|r| r == issue.rule_id()) {
            push(line, issue);
        }
    }
}

#[cfg(test)]
#[path = "facets_tests.rs"]
mod tests;
