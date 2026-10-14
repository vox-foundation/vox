//! Tighten-only gate for the KB facet backlog (D-04): the backlog may only shrink against the
//! base ref unless the file was re-seeded with a new epoch and note. Runs in the fast pre-push
//! tier, hence in the hosted-CI `linux` job. Decisions live in
//! `vox_doc_pipeline::pipeline::facet_backlog_gate_failures`; this only gathers the two texts.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Result, bail};
use vox_doc_pipeline::pipeline::{FACET_BACKLOG_REL, facet_backlog_gate_failures};
use vox_git::read_cmd::{GitReadError, read_only};

/// `rel` at `base`. `Ok(None)` (bootstrap) only when `ls-tree` succeeds and lists nothing; any
/// git failure is an error, never a bootstrap (fail closed).
pub(crate) fn file_at(
    git: impl Fn(&[&str]) -> Result<String, GitReadError>,
    base: &str,
    rel: &str,
) -> Result<Option<String>, String> {
    let listed = git(&["ls-tree", "--name-only", base, "--", rel])
        .map_err(|e| format!("cannot list {rel} at {base}: {e}"))?;
    if listed.trim().is_empty() {
        return Ok(None);
    }
    git(&["show", &format!("{base}:{rel}")])
        .map(Some)
        .map_err(|e| format!("cannot read {rel} at {base}: {e}"))
}

/// Every path under `docs/src` in the base tree. A git failure is an error (fail closed): an
/// empty set would make the re-epoch check reject every row, never admit one.
pub(crate) fn docs_at(
    git: impl Fn(&[&str]) -> Result<String, GitReadError>,
    base: &str,
) -> Result<BTreeSet<String>, String> {
    git(&["ls-tree", "-r", "--name-only", "-z", base, "--", "docs/src"])
        .map(|s| {
            s.split('\0')
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .map_err(|e| format!("cannot list docs/src at {base}: {e}"))
}

pub(crate) fn run_facet_backlog_gate(root: &Path, base: &str) -> Result<()> {
    super::gate::resolve_base(root, base)?;
    let base_text = file_at(|a| read_only(root, a), base, FACET_BACKLOG_REL)
        .map_err(|e| anyhow::anyhow!("[facet-backlog] {e}"))?;
    let head_path = root.join(FACET_BACKLOG_REL);
    let head_text = if head_path.exists() {
        Some(std::fs::read_to_string(&head_path)?)
    } else {
        None
    };
    let base_docs = docs_at(|a| read_only(root, a), base)
        .map_err(|e| anyhow::anyhow!("[facet-backlog] {e}"))?;
    let failures =
        facet_backlog_gate_failures(base_text.as_deref(), head_text.as_deref(), &base_docs);
    if failures.is_empty() {
        println!("facet backlog gate: OK");
        return Ok(());
    }
    for f in &failures {
        eprintln!("{f}");
    }
    eprintln!(
        "HEAL: the facet backlog can only shrink. Fix docs and run `cargo run -p vox-doc-pipeline -- --tighten-facet-backlog`; \
         adding a row or rule id is USER-AUTHORIZED-ONLY (re-seed with --seed-facet-backlog --epoch-note)."
    );
    bail!("facet backlog gate: {} failure(s)", failures.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn fake_git(
        ls: Option<&'static str>,
        show: Option<&'static str>,
    ) -> impl Fn(&[&str]) -> Result<String, GitReadError> {
        move |a: &[&str]| {
            let r = if a[0] == "ls-tree" { ls } else { show };
            r.map(str::to_owned).ok_or(GitReadError::NonZero {
                code: 128,
                stderr: "fatal: simulated".into(),
            })
        }
    }

    #[test]
    fn file_at_bootstraps_only_on_proven_absence() {
        assert_eq!(file_at(fake_git(Some(""), None), "b", "p").unwrap(), None);
        assert_eq!(
            file_at(fake_git(Some("p"), Some("text")), "b", "p").unwrap(),
            Some("text".to_string())
        );
        let err = file_at(fake_git(Some("p"), None), "b", "p").unwrap_err();
        assert!(err.contains("simulated"), "{err}");
        assert!(file_at(fake_git(None, None), "b", "p").is_err());
    }

    #[test]
    fn docs_at_splits_nul_and_fails_closed() {
        let ok = docs_at(
            fake_git(Some("docs/src/a.md\0docs/src/b c.md\0"), None),
            "b",
        )
        .unwrap();
        assert_eq!(
            ok.into_iter().collect::<Vec<_>>(),
            ["docs/src/a.md", "docs/src/b c.md"]
        );
        let err = docs_at(fake_git(None, None), "b").unwrap_err();
        assert!(err.contains("simulated"), "{err}");
    }

    fn git(dir: &Path, args: &[&str]) {
        // vox-arch-check: allow git-exec
        let ok = Command::new("git")
            .current_dir(dir)
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap()
            .status
            .success();
        assert!(ok, "git {args:?} failed");
    }

    fn y(epoch: u32, note: &str, rules: &[(&str, &str)]) -> String {
        let rows: String = rules
            .iter()
            .map(|(p, r)| format!("\n  - path: {p}\n    rules: [{r}]"))
            .collect();
        format!(
            "schema_version: 1\nepoch: {epoch}\nepoch_note: {note}\ndocs:{}\n",
            if rows.is_empty() { " []".into() } else { rows }
        )
    }

    fn put(root: &Path, text: &str) {
        let p = root.join(FACET_BACKLOG_REL);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn gate_rejects_same_epoch_addition() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        std::fs::write(root.join("README.md"), "x").unwrap();
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "base"]);
        let run = |base: &str| run_facet_backlog_gate(root, base);

        // absent at base and head: pre-seed passes; unresolvable base fails closed
        run("HEAD").unwrap();
        assert!(run("refs/heads/definitely-missing").is_err());

        // seed lands on a second commit (with the docs it lists); the gate compares against the
        // first (absent: bootstrap)
        std::fs::create_dir_all(root.join("docs/src")).unwrap();
        for d in ["a.md", "b.md"] {
            std::fs::write(root.join("docs/src").join(d), "# d\n").unwrap();
        }
        put(
            root,
            &y(1, "seed", &[("docs/src/a.md", "facet-missing-topic")]),
        );
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "seed"]);
        run("HEAD~1").unwrap();
        run("HEAD").unwrap(); // identical to base

        // same-epoch addition fails and names the path
        put(
            root,
            &y(
                1,
                "seed",
                &[
                    ("docs/src/a.md", "facet-missing-topic"),
                    ("docs/src/b.md", "facet-missing-topic"),
                ],
            ),
        );
        let err = run("HEAD").unwrap_err().to_string();
        assert!(err.contains("1 failure"), "{err}");
        let failures = facet_backlog_gate_failures(
            Some(&y(1, "seed", &[("docs/src/a.md", "facet-missing-topic")])),
            Some(&std::fs::read_to_string(root.join(FACET_BACKLOG_REL)).unwrap()),
            &docs_at(|a| read_only(root, a), "HEAD").unwrap(),
        );
        assert!(failures[0].contains("docs/src/b.md"), "{failures:?}");

        // re-epoch with a new note passes
        put(
            root,
            &y(
                2,
                "reseed after import",
                &[
                    ("docs/src/a.md", "facet-missing-topic"),
                    ("docs/src/b.md", "facet-missing-topic"),
                ],
            ),
        );
        run("HEAD").unwrap();

        // WR-01: a re-epoch that lists a doc absent from the base tree is rejected
        put(
            root,
            &y(
                2,
                "reseed after import",
                &[
                    ("docs/src/a.md", "facet-missing-topic"),
                    ("docs/src/mine.md", "facet-missing-topic"),
                ],
            ),
        );
        let err = run("HEAD").unwrap_err().to_string();
        assert!(err.contains("1 failure"), "{err}");

        // deleting the file after it existed at base fails
        std::fs::remove_file(root.join(FACET_BACKLOG_REL)).unwrap();
        assert!(run("HEAD").is_err());
    }
}
