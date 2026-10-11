//! KB facet lint (Phase 4): rule ids are stable `facet-*` strings and double as backlog keys.

use std::path::Path;

use vox_kb_facets::{
    BACKLOG_REL, Backlog, SummaryStatus, TAXONOMY_REL, Taxonomy, inspect_summary, parse_facets,
};

use super::types::{FacetIssue, LintError, LintKind};

/// Every rule id a backlog row may excuse (all but the two meta ids).
pub(crate) const BACKLOGGABLE_RULE_IDS: &[&str] = &[
    "facet-parse-error",
    "facet-missing-topic",
    "facet-missing-domain",
    "facet-missing-summary",
    "facet-missing-authorship",
];

impl FacetIssue {
    pub(crate) fn rule_id(&self) -> &'static str {
        match self {
            Self::ContractUnreadable { .. } => "facet-contract-unreadable",
            Self::ParseError { .. } => "facet-parse-error",
            Self::MissingTopic => "facet-missing-topic",
            Self::MissingDomain => "facet-missing-domain",
            Self::MissingAuthorship => "facet-missing-authorship",
            Self::MissingSummary => "facet-missing-summary",
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
            Self::MissingAuthorship => "frontmatter is missing `authorship:`".into(),
            Self::MissingSummary => "no complete <!-- kb:summary --> block".into(),
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
            Self::MissingTopic | Self::MissingDomain => {
                "use a prefLabel from contracts/documentation/kb-taxonomy.v1.yaml"
            }
            Self::MissingAuthorship => "add `authorship: human | llm-assisted | llm-generated`",
            Self::MissingSummary => {
                "put a 40-120 word <!-- kb:summary --> block directly under the H1"
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

fn blank(v: &Option<String>) -> bool {
    v.as_deref().is_none_or(|s| s.trim().is_empty())
}

/// Pure facet check of one document: `(1-based line, issue)` pairs.
pub(crate) fn facet_issues(doc_text: &str, _taxonomy: &Taxonomy) -> Vec<(usize, FacetIssue)> {
    let f = match parse_facets(doc_text) {
        Ok(f) => f,
        Err(msg) => return vec![(1, FacetIssue::ParseError { msg })],
    };
    let mut out = Vec::new();
    if blank(&f.topic) {
        out.push((1, FacetIssue::MissingTopic));
    }
    if blank(&f.domain) {
        out.push((1, FacetIssue::MissingDomain));
    }
    if f.authorship.is_none() {
        out.push((1, FacetIssue::MissingAuthorship));
    }
    if matches!(
        inspect_summary(doc_text),
        SummaryStatus::Missing | SummaryStatus::Unterminated
    ) {
        out.push((1, FacetIssue::MissingSummary));
    }
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
