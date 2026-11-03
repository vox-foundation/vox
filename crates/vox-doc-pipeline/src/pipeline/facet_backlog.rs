//! Backlog ratchet tools (KB Phase 4, D-04): seed once, then only shrink.
//!
//! `--seed-facet-backlog --epoch-note <why>` writes the file from live violations (re-seeding
//! is the only way to add and is user-authorized-only); `--tighten-facet-backlog` only removes.
//! `facet_backlog_gate_failures` is the base-ref comparison `vox ci pre-push` runs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use vox_kb_facets::{BACKLOG_REL, Backlog, TAXONOMY_REL, Taxonomy};

use super::facets::{BACKLOGGABLE_RULE_IDS, facet_issues, facet_lint_rel_path};

pub(crate) const BACKLOG_HEAL: &str = "\
HEAL: the facet backlog can only shrink. Legal moves:
  1. Add the missing facets to the doc, then run
       cargo run -p vox-doc-pipeline -- --tighten-facet-backlog
  2. Adding a row or rule id is USER-AUTHORIZED-ONLY. Propose it in the PR description and stop;
     the user re-seeds with
       cargo run -p vox-doc-pipeline -- --seed-facet-backlog --epoch-note \"<reason>\"
Never hand-edit contracts/documentation/kb-facet-backlog.v1.yaml.";

/// Live backloggable violations of every in-scope doc under `docs/src`: path -> rule ids.
pub(crate) fn live_violations(
    repo_root: &Path,
    taxonomy: &Taxonomy,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut files = Vec::new();
    super::collect_md_files(&repo_root.join("docs/src"), &mut files);
    let mut out = BTreeMap::new();
    for f in files {
        let Some(rel) = facet_lint_rel_path(&f, repo_root) else {
            continue;
        };
        let Ok(text) = vox_bounded_fs::read_utf8_path_capped(&f) else {
            continue;
        };
        let rules: BTreeSet<String> = facet_issues(&text, taxonomy)
            .iter()
            .map(|(_, i)| i.rule_id())
            .filter(|id| BACKLOGGABLE_RULE_IDS.contains(id))
            .map(str::to_string)
            .collect();
        if !rules.is_empty() {
            out.insert(rel, rules);
        }
    }
    out
}

fn load_taxonomy(repo_root: &Path) -> Result<Taxonomy, String> {
    let text = vox_bounded_fs::read_utf8_path_capped(&repo_root.join(TAXONOMY_REL))
        .map_err(|e| format!("cannot read {TAXONOMY_REL}: {e}"))?;
    Taxonomy::from_yaml_str(&text).map_err(|e| format!("{TAXONOMY_REL} invalid: {e}"))
}

fn load_backlog(repo_root: &Path) -> Result<Option<Backlog>, String> {
    let p = repo_root.join(BACKLOG_REL);
    if !p.exists() {
        return Ok(None);
    }
    let text = vox_bounded_fs::read_utf8_path_capped(&p)
        .map_err(|e| format!("cannot read {BACKLOG_REL}: {e}"))?;
    Backlog::from_yaml_str(&text)
        .map(Some)
        .map_err(|e| format!("{BACKLOG_REL} invalid: {e}"))
}

fn write_backlog(repo_root: &Path, b: &Backlog) -> Result<(), String> {
    let p = repo_root.join(BACKLOG_REL);
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, b.to_yaml_string()?).map_err(|e| format!("cannot write {BACKLOG_REL}: {e}"))
}

/// Write the backlog from live violations. Epoch is prior + 1 when a file exists, else 1.
pub(crate) fn seed(repo_root: &Path, epoch_note: &str) -> Result<String, String> {
    if epoch_note.trim().is_empty() {
        return Err("--seed-facet-backlog requires a non-empty --epoch-note <reason>".into());
    }
    let taxonomy = load_taxonomy(repo_root)?;
    let epoch = load_backlog(repo_root)?.map_or(1, |b| b.epoch + 1);
    let live = live_violations(repo_root, &taxonomy);
    let b = Backlog::from_violations(epoch, epoch_note.trim(), &live);
    write_backlog(repo_root, &b)?;
    let rule_ids: usize = b.docs.iter().map(|r| r.rules.len()).sum();
    Ok(format!(
        "seeded {} docs, {rule_ids} rule ids (epoch {epoch})",
        b.docs.len()
    ))
}

/// Remove fixed rules and rows. Refuses when any live violation is not already listed.
pub(crate) fn tighten(repo_root: &Path) -> Result<String, String> {
    let prior = load_backlog(repo_root)?
        .ok_or_else(|| format!("{BACKLOG_REL} is missing; nothing to tighten"))?;
    let live = live_violations(repo_root, &load_taxonomy(repo_root)?);
    let next = prior.tightened(&live).map_err(|adds| {
        let mut msg = format!(
            "refusing to tighten: {} live violation(s) are not in the backlog:\n",
            adds.len()
        );
        for (p, r) in adds.iter().take(20) {
            msg.push_str(&format!("  {p}: {r}\n"));
        }
        if adds.len() > 20 {
            msg.push_str(&format!("  ... and {} more\n", adds.len() - 20));
        }
        msg.push_str(BACKLOG_HEAL);
        msg
    })?;
    if next == prior {
        return Ok("facet backlog already tight".into());
    }
    write_backlog(repo_root, &next)?;
    let n = |b: &Backlog| b.docs.iter().map(|r| r.rules.len()).sum::<usize>();
    Ok(format!(
        "tightened: {} -> {} docs, {} -> {} rule ids",
        prior.docs.len(),
        next.docs.len(),
        n(&prior),
        n(&next)
    ))
}

/// Gate failures for the head backlog text against the base-ref text (`None` = file absent).
/// `base_docs` is every `docs/src` path in the base tree; a re-epoch may only list those.
#[must_use]
pub fn facet_backlog_gate_failures(
    base_text: Option<&str>,
    head_text: Option<&str>,
    base_docs: &BTreeSet<String>,
) -> Vec<String> {
    let parse = |which: &str, t: &str| {
        Backlog::from_yaml_str(t)
            .map_err(|e| format!("[facet-backlog] {which} backlog invalid: {e}"))
    };
    let base = match base_text.map(|t| parse("base", t)).transpose() {
        Ok(b) => b,
        Err(e) => return vec![e],
    };
    let head = match head_text {
        Some(t) => match parse("head", t) {
            Ok(h) => h,
            Err(e) => return vec![e],
        },
        None if base.is_some() => return vec!["[facet-backlog] backlog deleted".into()],
        None => return Vec::new(),
    };
    Backlog::tighten_only_violations(base.as_ref(), &head, base_docs)
}

#[cfg(test)]
mod tests {
    use super::super::lint_in;
    use super::*;

    const TAXONOMY: &str = include_str!("../../../../contracts/documentation/kb-taxonomy.v1.yaml");
    const BARE: &str = "---\ntitle: \"T\"\n---\n# T\n";

    fn root(name: &str) -> std::path::PathBuf {
        let r =
            std::env::temp_dir().join(format!("vox-facet-backlog-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        std::fs::create_dir_all(r.join("docs/src/archive")).unwrap();
        std::fs::create_dir_all(r.join("contracts/documentation")).unwrap();
        std::fs::write(r.join(TAXONOMY_REL), TAXONOMY).unwrap();
        r
    }

    fn doc(r: &Path, rel: &str, text: &str) {
        std::fs::write(r.join(rel), text).unwrap();
    }

    fn run(r: &Path, args: &[&str]) -> i32 {
        let a: Vec<String> = args.iter().map(|s| (*s).to_string()).collect();
        lint_in(r, &a)
    }

    fn backlog(r: &Path) -> Backlog {
        Backlog::from_yaml_str(&std::fs::read_to_string(r.join(BACKLOG_REL)).unwrap()).unwrap()
    }

    #[test]
    fn facet_backlog_seed_then_tighten_then_refuse_addition() {
        let r = root("flow");
        doc(&r, "docs/src/a.md", BARE);
        doc(&r, "docs/src/b.md", BARE);
        doc(&r, "docs/src/archive/old.md", BARE);
        // no backlog file exists: seeding must not need one (returns before FacetCtx::load)
        assert!(!r.join(BACKLOG_REL).exists());
        assert_eq!(run(&r, &["--seed-facet-backlog"]), 1, "note is required");
        assert_eq!(
            run(&r, &["--seed-facet-backlog", "--epoch-note", "initial"]),
            0
        );
        let first = std::fs::read_to_string(r.join(BACKLOG_REL)).unwrap();
        assert!(first.starts_with("# GENERATED"));
        let b = backlog(&r);
        assert_eq!(b.epoch, 1);
        assert_eq!(b.docs.len(), 2, "archive doc is out of scope");
        assert!(b.docs.iter().all(|d| d.rules.len() == 4));
        assert_eq!(
            seed(&r, "second").unwrap(),
            "seeded 2 docs, 8 rule ids (epoch 2)"
        );
        assert_eq!(backlog(&r).epoch, 2);
        // the written text is the canonical rendering (stable bytes across runs)
        let again = std::fs::read_to_string(r.join(BACKLOG_REL)).unwrap();
        assert_eq!(backlog(&r).to_yaml_string().unwrap(), again);
        assert_eq!(
            again
                .replace("second", "initial")
                .replace("epoch: 2", "epoch: 1"),
            first
        );

        // fix topic+domain on a.md
        doc(
            &r,
            "docs/src/a.md",
            "---\ntitle: \"T\"\ntopic: search-retrieval\ndomain: knowledge\n---\n# T\n",
        );
        assert_eq!(run(&r, &["--tighten-facet-backlog"]), 0);
        let b = backlog(&r);
        assert_eq!(
            b.rules_for("docs/src/a.md").unwrap(),
            ["facet-missing-authorship", "facet-missing-summary"]
        );
        assert_eq!(b.rules_for("docs/src/b.md").unwrap().len(), 4);
        assert_eq!(b.epoch, 2, "tighten keeps the epoch");

        // a row whose rules are all clean, and a deleted file, are removed
        std::fs::remove_file(r.join("docs/src/b.md")).unwrap();
        assert_eq!(run(&r, &["--tighten-facet-backlog"]), 0);
        assert!(backlog(&r).rules_for("docs/src/b.md").is_none());

        // an addition is refused and the file is untouched
        doc(&r, "docs/src/c.md", BARE);
        let before = std::fs::read_to_string(r.join(BACKLOG_REL)).unwrap();
        let err = tighten(&r).unwrap_err();
        assert!(err.contains("docs/src/c.md"), "{err}");
        assert!(err.contains("USER-AUTHORIZED-ONLY"), "{err}");
        assert_eq!(run(&r, &["--tighten-facet-backlog"]), 1);
        assert_eq!(
            std::fs::read_to_string(r.join(BACKLOG_REL)).unwrap(),
            before
        );
    }

    #[test]
    fn facet_backlog_seed_summary_line_and_blank_note() {
        let r = root("line");
        doc(&r, "docs/src/a.md", BARE);
        doc(&r, "docs/src/b.md", BARE);
        assert_eq!(
            seed(&r, "initial").unwrap(),
            "seeded 2 docs, 8 rule ids (epoch 1)"
        );
        assert!(seed(&r, "  ").is_err());
        assert!(tighten(&root("missing")).unwrap_err().contains("missing"));
    }

    #[test]
    fn facet_backlog_gate_text_cases() {
        let y = |epoch: u32, note: &str, rows: &str| {
            format!("schema_version: 1\nepoch: {epoch}\nepoch_note: {note}\ndocs:{rows}\n")
        };
        let one = "\n  - path: docs/src/a.md\n    rules: [facet-missing-topic]";
        let two = "\n  - path: docs/src/a.md\n    rules: [facet-missing-topic]\n  - path: docs/src/b.md\n    rules: [facet-missing-topic]";
        let tree: BTreeSet<String> = ["docs/src/a.md", "docs/src/b.md"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        let g = |b: Option<&str>, h: Option<&str>| facet_backlog_gate_failures(b, h, &tree);
        assert!(g(None, None).is_empty(), "pre-seed passes");
        assert!(g(None, Some(&y(1, "s", two))).is_empty(), "bootstrap");
        assert!(g(Some(&y(1, "s", one)), Some(&y(1, "s", one))).is_empty());
        assert!(g(Some(&y(1, "s", two)), Some(&y(1, "s", one))).is_empty());
        assert_eq!(g(Some(&y(1, "s", one)), Some(&y(1, "s", two))).len(), 1);
        assert!(g(Some(&y(1, "s", one)), Some(&y(2, "reseed", two))).is_empty());
        // WR-01: the re-seed may not list a path that is absent from the base tree
        let own = "\n  - path: docs/src/a.md\n    rules: [facet-missing-topic]\n  - path: docs/src/mine.md\n    rules: [facet-missing-topic]";
        let f = g(Some(&y(1, "s", one)), Some(&y(2, "reseed", own)));
        assert_eq!(f.len(), 1, "{f:?}");
        assert!(f[0].contains("docs/src/mine.md"), "{f:?}");
        assert_eq!(
            g(Some(&y(1, "s", one)), None),
            ["[facet-backlog] backlog deleted"]
        );
        assert_eq!(g(Some(&y(1, "s", one)), Some("not: [valid")).len(), 1);
        assert_eq!(g(Some("junk: ["), Some(&y(1, "s", one))).len(), 1);
    }
}
