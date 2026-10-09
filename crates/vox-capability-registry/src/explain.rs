//! Exact-then-fuzzy resolution of a free-text query against the capability index (D-10).
//!
//! Pure and transport-independent: `vox explain` / `vox features` and the MCP twin all call
//! [`resolve`] and print [`explain_rows`], so both transports stay byte-comparable (D-12).
//! The query is only ever compared as a string: no regex, no paths, no shell.

use anyhow::{Result, bail};

use crate::index::{CapabilityIndex, DomainDef, IndexRow, RowKind};

/// Longest accepted query, in characters (not bytes).
pub const MAX_QUERY_CHARS: usize = 256;
/// Most rows a fuzzy shortlist returns.
pub const SHORTLIST_LIMIT: usize = 8;
/// Hard cap on any result set (exact primary + `also`).
pub const MAX_RESULTS: usize = 20;
/// Minimum score for a row to enter the shortlist.
const FUZZY_THRESHOLD: f64 = 0.75;

/// A fuzzy candidate with its score (1.0 is best).
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate<'a> {
    pub row: &'a IndexRow,
    pub score: f64,
}

/// Outcome of [`resolve`].
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution<'a> {
    /// The query equals an alias of `primary`; `also` are the other exact hits.
    Exact {
        primary: &'a IndexRow,
        also: Vec<&'a IndexRow>,
    },
    /// No exact hit; best fuzzy rows, score desc then id asc.
    Shortlist(Vec<Candidate<'a>>),
    NoMatch,
}

/// Exact-match aliases of a row, lowercased, with their priority
/// (0 id, 1 cli forms, 2 mcp name, 3 surface view key, 4 crate, 5 skill).
fn aliases(row: &IndexRow) -> Vec<(u8, String)> {
    let mut a = vec![(0, row.id.to_lowercase())];
    if let Some(c) = &row.cli {
        let path = c.path.join(" ").to_lowercase();
        a.push((1, c.invocation.to_lowercase()));
        a.push((1, path.replace(' ', ".")));
        a.push((1, path));
    }
    if let Some(m) = &row.mcp {
        let name = m.name.to_lowercase();
        if let Some(short) = name.strip_prefix("vox_") {
            a.push((2, short.to_string()));
        }
        a.push((2, name));
    }
    let tail = |prefix: &str| row.id.strip_prefix(prefix).map(str::to_lowercase);
    match row.kind {
        RowKind::Surface => {
            if let Some(k) = row.gui.as_ref().and_then(|g| g.view_key.as_ref()) {
                a.push((3, k.to_lowercase()));
            }
        }
        RowKind::Crate => {
            if let Some(name) = tail("crate.") {
                if let Some(short) = name.strip_prefix("vox-") {
                    a.push((4, short.to_string()));
                }
                a.push((4, name));
            }
        }
        RowKind::Skill => a.extend(tail("skill.").map(|s| (5, s))),
        _ => {}
    }
    a
}

fn tokens(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|t| !t.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Fuzzy score: best of Jaro-Winkler against any alias/title, and 0.9 x the share of query
/// tokens found as whole tokens of the row's id, title, cli path, mcp name or view key.
fn score(query: &str, query_tokens: &[String], row: &IndexRow) -> f64 {
    let title = row.title.to_lowercase();
    let jw = aliases(row)
        .iter()
        .map(|(_, a)| a)
        .chain(std::iter::once(&title))
        .map(|a| strsim::jaro_winkler(query, a))
        .fold(0.0, f64::max);
    if query_tokens.is_empty() {
        return jw;
    }
    let mut row_tokens = tokens(&row.id);
    row_tokens.extend(tokens(&row.title));
    row_tokens.extend(
        row.cli
            .iter()
            .flat_map(|c| c.path.iter().flat_map(|p| tokens(p))),
    );
    row_tokens.extend(row.mcp.iter().flat_map(|m| tokens(&m.name)));
    row_tokens.extend(
        row.gui
            .iter()
            .flat_map(|g| g.view_key.iter().flat_map(|k| tokens(k))),
    );
    let hit = query_tokens
        .iter()
        .filter(|t| row_tokens.contains(t))
        .count();
    jw.max(0.9 * hit as f64 / query_tokens.len() as f64)
}

/// Resolve `query` against `index`: exact alias first, then a ranked shortlist.
pub fn resolve<'a>(index: &'a CapabilityIndex, query: &str) -> Result<Resolution<'a>> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        bail!("empty query: pass a command, tool, view, crate or skill name");
    }
    if q.chars().count() > MAX_QUERY_CHARS {
        bail!("query too long (max {MAX_QUERY_CHARS} characters)");
    }
    let mut exact: Vec<(u8, &IndexRow)> = index
        .rows
        .iter()
        .filter_map(|r| {
            aliases(r)
                .iter()
                .filter(|(_, a)| *a == q)
                .map(|(p, _)| *p)
                .min()
                .map(|p| (p, r))
        })
        .collect();
    if !exact.is_empty() {
        exact.sort_by(|(pa, a), (pb, b)| (pa, a.kind, &a.id).cmp(&(pb, b.kind, &b.id)));
        let mut rows = exact.into_iter().map(|(_, r)| r);
        let primary = rows.next().expect("exact is non-empty");
        let also = rows.take(MAX_RESULTS - 1).collect();
        return Ok(Resolution::Exact { primary, also });
    }
    let qt = tokens(&q);
    let mut cands: Vec<Candidate> = index
        .rows
        .iter()
        .map(|row| Candidate {
            row,
            score: score(&q, &qt, row),
        })
        .filter(|c| c.score >= FUZZY_THRESHOLD)
        .collect();
    if cands.is_empty() {
        return Ok(Resolution::NoMatch);
    }
    cands.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.row.id.cmp(&b.row.id))
    });
    cands.truncate(SHORTLIST_LIMIT);
    Ok(Resolution::Shortlist(cands))
}

/// The rows both transports print for a resolution (D-12).
#[must_use]
pub fn explain_rows<'a>(r: &Resolution<'a>) -> Vec<&'a IndexRow> {
    match r {
        Resolution::Exact { primary, also } => std::iter::once(*primary)
            .chain(also.iter().copied())
            .collect(),
        Resolution::Shortlist(c) => c.iter().map(|c| c.row).collect(),
        Resolution::NoMatch => vec![],
    }
}

/// Rows of one domain, sorted by id; an unknown domain is an error listing the valid ids.
pub fn rows_in_domain<'a>(index: &'a CapabilityIndex, domain: &str) -> Result<Vec<&'a IndexRow>> {
    let d = domain.trim().to_lowercase();
    if !index.domains.iter().any(|x| x.id == d) {
        let valid: Vec<&str> = index.domains.iter().map(|x| x.id.as_str()).collect();
        bail!(
            "unknown domain `{domain}`; valid domains: {}",
            valid.join(", ")
        );
    }
    let mut rows: Vec<&IndexRow> = index
        .rows
        .iter()
        .filter(|r| r.domain.as_deref() == Some(d.as_str()))
        .collect();
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(rows)
}

/// Per domain (vocabulary order): number of cli, mcp_tool and surface rows.
#[must_use]
pub fn domain_counts(index: &CapabilityIndex) -> Vec<(&DomainDef, usize, usize, usize)> {
    let count = |d: &DomainDef, k: RowKind| {
        index
            .rows
            .iter()
            .filter(|r| r.kind == k && r.domain.as_deref() == Some(d.id.as_str()))
            .count()
    };
    index
        .domains
        .iter()
        .map(|d| {
            (
                d,
                count(d, RowKind::Cli),
                count(d, RowKind::McpTool),
                count(d, RowKind::Surface),
            )
        })
        .collect()
}

/// `--json` / MCP `data` shape of the domain overview: `[{id, label, cli, mcp, surface}]`.
#[must_use]
pub fn domain_counts_json(index: &CapabilityIndex) -> serde_json::Value {
    domain_counts(index)
        .into_iter()
        .map(|(d, cli, mcp, surface)| {
            serde_json::json!({"id": d.id, "label": d.label, "cli": cli, "mcp": mcp, "surface": surface})
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_fixture::{cli_ref, fixture_index, row};

    fn exact_id<'a>(r: &Resolution<'a>) -> &'a str {
        match r {
            Resolution::Exact { primary, .. } => &primary.id,
            other => panic!("expected Exact, got {other:?}"),
        }
    }

    #[test]
    fn exact_on_id() {
        let idx = fixture_index();
        let r = resolve(&idx, "cli.secrets.get").unwrap();
        assert_eq!(exact_id(&r), "cli.secrets.get");
    }

    #[test]
    fn exact_on_cli_invocation() {
        let idx = fixture_index();
        for q in ["vox secrets get", "secrets get", "secrets.get"] {
            assert_eq!(
                exact_id(&resolve(&idx, q).unwrap()),
                "cli.secrets.get",
                "{q}"
            );
        }
    }

    #[test]
    fn exact_on_mcp_name() {
        let idx = fixture_index();
        for q in ["vox_secrets_get", "secrets_get"] {
            assert_eq!(
                exact_id(&resolve(&idx, q).unwrap()),
                "mcp.vox_secrets_get",
                "{q}"
            );
        }
    }

    #[test]
    fn exact_on_view_key() {
        let idx = fixture_index();
        assert_eq!(exact_id(&resolve(&idx, "  MESH ").unwrap()), "surface.mesh");
    }

    #[test]
    fn exact_on_crate_and_skill() {
        let idx = fixture_index();
        assert_eq!(
            exact_id(&resolve(&idx, "vox-secrets").unwrap()),
            "crate.vox-secrets"
        );
        assert_eq!(
            exact_id(&resolve(&idx, "demo-skill").unwrap()),
            "skill.demo-skill"
        );
    }

    #[test]
    fn multiple_exact_prioritized() {
        let idx = fixture_index();
        match resolve(&idx, "secrets").unwrap() {
            Resolution::Exact { primary, also } => {
                assert_eq!(primary.id, "cli.secrets");
                let ids: Vec<_> = also.iter().map(|r| r.id.as_str()).collect();
                assert_eq!(ids, ["crate.vox-secrets"]);
            }
            other => panic!("expected Exact, got {other:?}"),
        }
    }

    #[test]
    fn typo_gives_shortlist() {
        let idx = fixture_index();
        match resolve(&idx, "secrts get").unwrap() {
            Resolution::Shortlist(c) => {
                assert_eq!(c[0].row.id, "cli.secrets.get");
                assert!(c.len() <= SHORTLIST_LIMIT);
            }
            other => panic!("expected Shortlist, got {other:?}"),
        }
    }

    #[test]
    fn shortlist_ties_break_on_id() {
        let mut idx = fixture_index();
        // Inserted mcp-first; identical aliases give identical scores, so id order must win.
        let mut m = row("mcp.frobnicate", RowKind::McpTool, "frobnicate");
        m.mcp = Some(crate::index::McpRef {
            name: "frobnicate".into(),
        });
        let mut c = row("cli.frobnicate", RowKind::Cli, "frobnicate");
        c.cli = cli_ref(&["frobnicate"]);
        idx.rows = vec![m, c];
        match resolve(&idx, "frobnicat").unwrap() {
            Resolution::Shortlist(c) => {
                assert_eq!(c[0].score, c[1].score);
                let ids: Vec<_> = c.iter().map(|x| x.row.id.as_str()).collect();
                assert_eq!(ids, ["cli.frobnicate", "mcp.frobnicate"]);
            }
            other => panic!("expected Shortlist, got {other:?}"),
        }
    }

    #[test]
    fn empty_and_long_queries_err() {
        let idx = fixture_index();
        assert!(resolve(&idx, "").is_err());
        assert!(resolve(&idx, "   ").is_err());
        let e = resolve(&idx, &"a".repeat(257)).unwrap_err().to_string();
        assert!(e.contains("256"), "{e}");
        // 256 multibyte chars are within the bound (chars, not bytes).
        assert!(resolve(&idx, &"é".repeat(256)).is_ok());
    }

    #[test]
    fn no_match_and_empty_index() {
        let idx = fixture_index();
        assert_eq!(resolve(&idx, "zzqqxx").unwrap(), Resolution::NoMatch);
        let mut empty = fixture_index();
        empty.rows = vec![];
        assert_eq!(resolve(&empty, "secrets").unwrap(), Resolution::NoMatch);
    }

    #[test]
    fn explain_rows_parity() {
        let idx = fixture_index();
        let r = resolve(&idx, "secrets").unwrap();
        let ids: Vec<_> = explain_rows(&r).iter().map(|x| x.id.as_str()).collect();
        assert_eq!(ids, ["cli.secrets", "crate.vox-secrets"]);
        let s = resolve(&idx, "secrts get").unwrap();
        let Resolution::Shortlist(c) = &s else {
            panic!("expected Shortlist")
        };
        assert_eq!(explain_rows(&s).len(), c.len());
        assert!(explain_rows(&Resolution::NoMatch).is_empty());
    }

    #[test]
    fn rows_in_domain_and_unknown_domain() {
        let idx = fixture_index();
        let ids: Vec<_> = rows_in_domain(&idx, "secrets")
            .unwrap()
            .iter()
            .map(|r| r.id.as_str())
            .collect();
        assert_eq!(
            ids,
            [
                "cli.secrets",
                "cli.secrets.get",
                "crate.vox-secrets",
                "mcp.vox_secrets_get"
            ]
        );
        let e = rows_in_domain(&idx, "nope").unwrap_err().to_string();
        assert!(e.contains("secrets, mesh, platform"), "{e}");
        let counts = domain_counts(&idx);
        let (d, cli, mcp, gui) = counts[0];
        assert_eq!((d.id.as_str(), cli, mcp, gui), ("secrets", 2, 1, 0));
    }

    #[test]
    fn domain_counts_json_shape() {
        let v = domain_counts_json(&fixture_index());
        let first = &v.as_array().expect("array")[0];
        assert_eq!(
            first,
            &serde_json::json!({"id": "secrets", "label": first["label"], "cli": 2, "mcp": 1, "surface": 0})
        );
        assert_eq!(v.as_array().unwrap().len(), 3);
    }
}
