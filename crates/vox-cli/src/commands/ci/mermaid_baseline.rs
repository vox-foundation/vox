//! Base-ref gate for the legacy-Mermaid baseline (VoxDiagram Phase 87, R4): a PR may not raise any
//! per-file `mermaid` fence count, even when it edited its own baseline upward to match its docs.
//! Runs in the fast pre-push tier, hence in the hosted-CI `linux` job. The rule lives in
//! `vox_doc_pipeline::pipeline::mermaid_baseline_gate_failures`; this only gathers the two texts.

use std::path::Path;

use anyhow::{Result, bail};
use vox_doc_pipeline::pipeline::{MERMAID_BASELINE_REL, mermaid_baseline_gate_failures};
use vox_git::read_cmd::read_only;

use crate::commands::kb::facet_backlog::file_at;

pub(crate) fn run_mermaid_baseline_gate(root: &Path, base: &str) -> Result<()> {
    crate::commands::kb::gate::resolve_base(root, base)?;
    let base_text = file_at(|a| read_only(root, a), base, MERMAID_BASELINE_REL)
        .map_err(|e| anyhow::anyhow!("[legacy-mermaid] {e}"))?;
    let head_path = root.join(MERMAID_BASELINE_REL);
    let head_text = if head_path.exists() {
        Some(std::fs::read_to_string(&head_path)?)
    } else {
        None
    };
    let failures = mermaid_baseline_gate_failures(base_text.as_deref(), head_text.as_deref());
    if failures.is_empty() {
        println!("legacy-Mermaid baseline gate: OK");
        return Ok(());
    }
    for f in &failures {
        eprintln!("{f}");
    }
    eprintln!(
        "HEAL: the legacy-Mermaid baseline only shrinks. Author new diagrams as ```vox-diagram and run \
         `cargo run -p vox-doc-pipeline -- --tighten-mermaid-baseline`; raising a count or adding a file \
         is USER-AUTHORIZED-ONLY (re-seeding with --seed-mermaid-baseline)."
    );
    bail!(
        "legacy-Mermaid baseline gate: {} failure(s)",
        failures.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

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

    fn baseline(files: &[(&str, usize)]) -> String {
        let rows: String = files.iter().map(|(p, n)| format!("\n  {p}: {n}")).collect();
        let body = if rows.is_empty() { " {}".into() } else { rows };
        format!("schema_version: 1\nfiles:{body}\n")
    }

    fn put(root: &Path, rel: &str, text: &str) {
        let p = root.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, text).unwrap();
    }

    #[test]
    fn gate_fails_on_upward_edit_against_the_base_ref() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        git(root, &["init", "-q", "-b", "main"]);
        put(root, "README.md", "x");
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "base"]);
        let run = |base: &str| run_mermaid_baseline_gate(root, base);

        // pre-seed passes; an unresolvable base fails closed
        run("HEAD").unwrap();
        assert!(run("refs/heads/definitely-missing").is_err());

        // the seed lands on a second commit: bootstrap against HEAD~1, identical against HEAD
        let fence = "```mermaid\ngraph TD\n```\n";
        put(root, "docs/src/a.md", fence);
        put(
            root,
            MERMAID_BASELINE_REL,
            &baseline(&[("docs/src/a.md", 1)]),
        );
        git(root, &["add", "."]);
        git(root, &["commit", "-q", "-m", "seed"]);
        run("HEAD~1").unwrap();
        run("HEAD").unwrap();

        // a PR that adds a second fence AND edits its own baseline to match still fails
        put(root, "docs/src/a.md", &format!("{fence}\n{fence}"));
        put(
            root,
            MERMAID_BASELINE_REL,
            &baseline(&[("docs/src/a.md", 2)]),
        );
        let err = run("HEAD").unwrap_err().to_string();
        assert!(err.contains("1 failure(s)"), "{err}");

        // a new file with a fence fails
        put(
            root,
            MERMAID_BASELINE_REL,
            &baseline(&[("docs/src/a.md", 1), ("docs/src/b.md", 1)]),
        );
        let err = run("HEAD").unwrap_err().to_string();
        assert!(err.contains("1 failure(s)"), "{err}");

        // shrinking passes
        put(root, MERMAID_BASELINE_REL, &baseline(&[]));
        run("HEAD").unwrap();

        // deleting the baseline after it existed at base fails
        std::fs::remove_file(root.join(MERMAID_BASELINE_REL)).unwrap();
        assert!(run("HEAD").is_err());
    }
}
