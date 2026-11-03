//! Pre-registration checks of the live run, pure over their inputs: the freeze commit (judged from
//! read-only git outputs), the manifest's decision rule and pinned hashes, the report's prereg block
//! and the installed Mermaid version. The live runner (`live.rs`) gathers the inputs.

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};

use super::{file_hash, parse_manifest};

/// The Mermaid version the oracle and the pre-registered rule name.
pub const MERMAID_VERSION: &str = "11.17.2";

/// Refuse anything but a full 40-character lowercase hex commit id.
pub fn check_sha(sha: &str) -> Result<()> {
    ensure!(
        sha.len() == 40
            && sha
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "freeze commit {sha:?} is not a 40-character lowercase hex commit id"
    );
    Ok(())
}

/// Judge the freeze commit from three read-only git outputs: its subject (`Err` = unknown to git),
/// the commit ids of `<freeze>^..HEAD`, and the files changed since it under the dataset (runs and
/// probe ledger excluded) or the report.
pub fn verify_freeze(
    freeze: &str,
    prefix: &str,
    subject: Result<&str, &str>,
    ancestry: &str,
    changed: &str,
) -> Result<()> {
    let subject = match subject {
        Ok(s) => s.trim(),
        Err(e) => bail!("freeze commit {freeze} is unknown to git: {e}"),
    };
    ensure!(
        subject.starts_with(prefix),
        "freeze commit {freeze} is not the pre-registration commit: its subject {subject:?} does not start with {prefix:?}"
    );
    ensure!(
        ancestry.lines().any(|l| l.trim() == freeze),
        "freeze commit {freeze} is not an ancestor of HEAD"
    );
    let changed: Vec<&str> = changed.lines().filter(|l| !l.trim().is_empty()).collect();
    ensure!(
        changed.is_empty(),
        "frozen files changed since freeze commit {freeze}: {}",
        changed.join(", ")
    );
    Ok(())
}

/// The manifest carries the pre-registered rule (margin 0.10, infra ceiling 0.05 and the keys
/// `parse_manifest` checks) and every pinned file still hashes as pinned.
pub fn check_manifest(yaml: &str, dataset_dir: &Path) -> Result<BTreeMap<String, String>> {
    let rule = parse_manifest(yaml)
        .context("manifest lacks the pre-registered decision_rule")?
        .decision_rule;
    ensure!(
        (rule.margin - 0.10).abs() < 1e-12 && (rule.max_infra_rate - 0.05).abs() < 1e-12,
        "decision_rule margin {} / max_infra_rate {} differ from the pre-registered 0.10 / 0.05",
        rule.margin,
        rule.max_infra_rate
    );
    #[derive(serde::Deserialize)]
    struct Files {
        files: BTreeMap<String, String>,
    }
    let files = serde_yaml::from_str::<Files>(yaml)
        .context("manifest files")?
        .files;
    ensure!(!files.is_empty(), "manifest pins no file");
    for (rel, want) in &files {
        let path = rel
            .split('/')
            .fold(dataset_dir.to_path_buf(), |p, s| p.join(s));
        let got = file_hash(&path)?;
        ensure!(
            &got == want,
            "{rel}: hash {got} differs from the manifest's {want}"
        );
    }
    Ok(files)
}

/// The report holds a non-empty block between the pre-registration markers.
pub fn check_report(text: &str) -> Result<()> {
    let block = text
        .split_once("<!-- prereg:begin -->")
        .and_then(|(_, rest)| rest.split_once("<!-- prereg:end -->"))
        .map(|(block, _)| block.trim());
    ensure!(
        block.is_some_and(|b| !b.is_empty()),
        "report lacks a pre-registration block between <!-- prereg:begin --> and <!-- prereg:end -->"
    );
    Ok(())
}

/// The installed docs-astro mermaid package is the pinned version.
pub fn check_mermaid(package_json: &str) -> Result<()> {
    let v: serde_json::Value =
        serde_json::from_str(package_json).context("mermaid package.json")?;
    ensure!(
        v["version"] == MERMAID_VERSION,
        "docs-astro mermaid is {}, not {MERMAID_VERSION}",
        v["version"]
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const F: &str = "0123456789abcdef0123456789abcdef01234567";
    const P: &str = "docs(91-04): pre-register";

    #[test]
    fn check_sha_wants_40_lowercase_hex() {
        check_sha(F).unwrap();
        for bad in [
            String::new(),
            F[..39].to_string(),
            F.to_uppercase(),
            "g".repeat(40),
        ] {
            let err = check_sha(&bad).unwrap_err();
            assert!(err.to_string().contains("freeze commit"), "{err}");
        }
    }

    #[test]
    fn verify_freeze_refuses_each_failure_and_names_the_commit() {
        let ancestry = format!("{}\n{F}\n", "f".repeat(40));
        let subject = "docs(91-04): pre-register the decision rule\n";
        verify_freeze(F, P, Ok(subject), &ancestry, "\n").unwrap();
        let cases = [
            (
                Err("fatal: bad object"),
                ancestry.as_str(),
                "",
                "unknown to git",
            ),
            (
                Ok("feat(91-04): add a loop"),
                ancestry.as_str(),
                "",
                "pre-registration",
            ),
            (Ok(subject), "ffff\n", "", "not an ancestor of HEAD"),
            (
                Ok(subject),
                ancestry.as_str(),
                "tasks.v1.jsonl\n",
                "tasks.v1.jsonl",
            ),
        ];
        for (subject, ancestry, changed, want) in cases {
            let err = verify_freeze(F, P, subject, ancestry, changed)
                .unwrap_err()
                .to_string();
            assert!(err.contains(want) && err.contains(F), "{err}");
        }
    }

    const RULE: &str = "corpus_size: 1\ndecision_rule:\n  primary_endpoint: first_pass_validity_slot_a\n  interval: newcombe_method_10\n  confidence: 0.95\n  margin: 0.10\n  max_repair_turns: 2\n  max_infra_rate: 0.05\n";

    #[test]
    fn check_manifest_wants_the_rule_and_matching_hashes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("cards")).unwrap();
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        std::fs::write(dir.path().join("cards").join("c.txt"), "y").unwrap();
        let ha = file_hash(&dir.path().join("a.txt")).unwrap();
        let hc = file_hash(&dir.path().join("cards").join("c.txt")).unwrap();
        let yaml = format!("{RULE}files:\n  a.txt: {ha}\n  cards/c.txt: {hc}\n");
        assert_eq!(check_manifest(&yaml, dir.path()).unwrap().len(), 2);
        let no_rule = format!("corpus_size: 1\nfiles:\n  a.txt: {ha}\n");
        let err = check_manifest(&no_rule, dir.path()).unwrap_err();
        assert!(format!("{err:#}").contains("decision_rule"), "{err:#}");
        assert!(check_manifest(&yaml.replace("margin: 0.10", "margin: 0.20"), dir.path()).is_err());
        assert!(
            check_manifest(
                &yaml.replace("max_infra_rate: 0.05", "max_infra_rate: 0.10"),
                dir.path()
            )
            .is_err()
        );
        assert!(check_manifest(&format!("{RULE}files: {{}}\n"), dir.path()).is_err());
        std::fs::write(dir.path().join("a.txt"), "z").unwrap();
        let err = check_manifest(&yaml, dir.path()).unwrap_err();
        assert!(err.to_string().contains("a.txt"), "{err}");
    }

    #[test]
    fn check_report_wants_a_non_empty_prereg_block() {
        check_report("x\n<!-- prereg:begin -->\n> rule\n<!-- prereg:end -->\n").unwrap();
        for bad in [
            "no markers",
            "<!-- prereg:begin -->\n> rule\n",
            "<!-- prereg:end -->\n<!-- prereg:begin -->\n",
            "<!-- prereg:begin -->\n \n<!-- prereg:end -->",
        ] {
            assert!(check_report(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn check_mermaid_wants_the_pinned_version() {
        check_mermaid(r#"{"name":"mermaid","version":"11.17.2"}"#).unwrap();
        let err = check_mermaid(r#"{"version":"11.17.1"}"#).unwrap_err();
        assert!(err.to_string().contains("11.17.1"), "{err}");
        assert!(check_mermaid("not json").is_err());
    }
}
