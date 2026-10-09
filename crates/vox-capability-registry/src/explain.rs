//! Exact-then-fuzzy resolution of a free-text query against the capability index (D-10).
//!
//! Pure and transport-independent: `vox explain` / `vox features` and the MCP twin all call
//! [`resolve`] and print [`explain_rows`], so both transports stay byte-comparable (D-12).
//! The query is only ever compared as a string: no regex, no paths, no shell.

use anyhow::{Result, bail};

use crate::index::{CapabilityIndex, DomainDef, IndexRow};

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

/// Resolve `query` against `index`: exact alias first, then a ranked shortlist.
pub fn resolve<'a>(index: &'a CapabilityIndex, query: &str) -> Result<Resolution<'a>> {
    let _ = (index, query, FUZZY_THRESHOLD);
    bail!("RED stub: resolve not implemented")
}

/// The rows both transports print for a resolution (D-12).
#[must_use]
pub fn explain_rows<'a>(r: &Resolution<'a>) -> Vec<&'a IndexRow> {
    let _ = r;
    vec![]
}

/// Rows of one domain, sorted by id; an unknown domain is an error listing the valid ids.
pub fn rows_in_domain<'a>(index: &'a CapabilityIndex, domain: &str) -> Result<Vec<&'a IndexRow>> {
    let _ = (index, domain);
    bail!("RED stub: rows_in_domain not implemented")
}

/// Per domain (vocabulary order): number of cli, mcp_tool and surface rows.
#[must_use]
pub fn domain_counts(index: &CapabilityIndex) -> Vec<(&DomainDef, usize, usize, usize)> {
    let _ = index;
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::RowKind;
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
}
