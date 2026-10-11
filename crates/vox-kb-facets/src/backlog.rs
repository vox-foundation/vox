//! Facet backlog (D-03, D-04): the docs excused from specific facet rules until they are
//! curated. Read by the facet lint; written only by the seed/tighten command.

use serde::{Deserialize, Serialize};

/// Repo-relative path of the committed backlog.
pub const BACKLOG_REL: &str = "contracts/documentation/kb-facet-backlog.v1.yaml";

/// One excused doc: exact repo-relative path (`/` separators) and the rule ids it is excused from.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BacklogRow {
    pub path: String,
    pub rules: Vec<String>,
}

/// The backlog file. Rows are sorted by path, rule ids sorted within a row.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Backlog {
    pub schema_version: u32,
    pub epoch: u32,
    pub epoch_note: String,
    pub docs: Vec<BacklogRow>,
}

fn strictly_sorted(v: &[&str]) -> bool {
    v.windows(2).all(|w| w[0] < w[1])
}

impl Backlog {
    /// Parse and validate. Every problem is reported, joined with newlines.
    pub fn from_yaml_str(s: &str) -> Result<Backlog, String> {
        let b: Backlog = serde_yaml::from_str(s).map_err(|e| e.to_string())?;
        let mut errs: Vec<String> = Vec::new();
        if b.schema_version != 1 {
            errs.push(format!("schema_version {} is not 1", b.schema_version));
        }
        if b.epoch == 0 {
            errs.push("epoch must be >= 1".into());
        }
        if b.epoch_note.trim().is_empty() {
            errs.push("epoch_note must not be blank".into());
        }
        let paths: Vec<&str> = b.docs.iter().map(|r| r.path.as_str()).collect();
        if !strictly_sorted(&paths) {
            errs.push("docs must be sorted by path with no duplicates".into());
        }
        for r in &b.docs {
            if !r.path.starts_with("docs/src/") || !r.path.ends_with(".md") {
                errs.push(format!("{}: path must be docs/src/**/*.md", r.path));
            }
            let rules: Vec<&str> = r.rules.iter().map(String::as_str).collect();
            if rules.is_empty() || rules.iter().any(|x| x.trim().is_empty()) {
                errs.push(format!("{}: rules must be non-empty ids", r.path));
            } else if !strictly_sorted(&rules) {
                errs.push(format!("{}: rules must be sorted and unique", r.path));
            }
        }
        if errs.is_empty() {
            Ok(b)
        } else {
            Err(errs.join("\n"))
        }
    }

    /// Rule ids excused for the exact repo-relative path, or `None` when the doc is not listed.
    #[must_use]
    pub fn rules_for(&self, rel_path: &str) -> Option<&[String]> {
        self.docs
            .binary_search_by(|r| r.path.as_str().cmp(rel_path))
            .ok()
            .map(|i| self.docs[i].rules.as_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn y(docs: &str) -> String {
        format!("schema_version: 1\nepoch: 1\nepoch_note: seed\ndocs:{docs}\n")
    }

    #[test]
    fn backlog_accepts_valid_and_looks_up_by_exact_path() {
        let b = Backlog::from_yaml_str(&y(
            "\n  - path: docs/src/a.md\n    rules: [facet-missing-domain, facet-missing-topic]\n  - path: docs/src/b.md\n    rules: [facet-missing-topic]",
        ))
        .unwrap();
        assert_eq!(b.rules_for("docs/src/a.md").unwrap().len(), 2);
        assert!(b.rules_for("docs/src/c.md").is_none());
        assert!(b.rules_for("docs/src/A.md").is_none());
        assert!(Backlog::from_yaml_str(&y(" []")).unwrap().docs.is_empty());
    }

    #[test]
    fn backlog_rejects_every_malformed_shape() {
        let bad = [
            "schema_version: 1\nepoch: 1\nepoch_note: n\ndocs: []\nextra: 1\n".to_string(),
            "schema_version: 2\nepoch: 1\nepoch_note: n\ndocs: []\n".to_string(),
            "schema_version: 1\nepoch: 0\nepoch_note: n\ndocs: []\n".to_string(),
            "schema_version: 1\nepoch: 1\nepoch_note: '  '\ndocs: []\n".to_string(),
            y("\n  - path: docs/src/b.md\n    rules: [x]\n  - path: docs/src/a.md\n    rules: [x]"),
            y("\n  - path: docs/src/a.md\n    rules: [x]\n  - path: docs/src/a.md\n    rules: [x]"),
            y("\n  - path: docs/src/a.md\n    rules: []"),
            y("\n  - path: docs/src/a.md\n    rules: [b, a]"),
            y("\n  - path: docs/src/a.md\n    rules: [a, a]"),
            y("\n  - path: other/a.md\n    rules: [a]"),
            y("\n  - path: docs/src/a.mdx\n    rules: [a]"),
        ];
        for s in &bad {
            assert!(Backlog::from_yaml_str(s).is_err(), "accepted: {s}");
        }
    }
}
