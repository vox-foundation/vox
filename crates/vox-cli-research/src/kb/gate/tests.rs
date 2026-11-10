use super::super::eval::{run_query, run_report};
use super::*;
use vox_search::kb_eval::baseline::BaselineBlock;
use vox_search::kb_eval::golden::Split;
use vox_search::kb_eval::golden::read_canary_guid;

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for e in std::fs::read_dir(from).unwrap().flatten() {
        let dst = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &dst);
        } else {
            std::fs::copy(e.path(), dst).unwrap();
        }
    }
}

/// Tempdir repo whose `contracts/kb/eval/` is a copy of the vox-search fixture.
pub(crate) fn temp_repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../vox-search/tests/fixtures/kb_eval");
    copy_dir(&fixture, &tmp.path().join(EVAL_DIR));
    tmp
}

/// Write a baseline whose every metric is `v`.
fn baseline_with(root: &Path, v: f64) -> std::path::PathBuf {
    let mut b = new_epoch(None, &dev_report(&root.join(EVAL_DIR)).unwrap(), "t").unwrap();
    let l = serde_json::json!({"recall@5": v, "recall@10": v, "mrr@10": v, "ndcg@10": v});
    let all = serde_json::json!({"doc": l, "chunk": l, "answer_in_budget": v});
    let blk: BaselineBlock = serde_json::from_value(all).unwrap();
    b.metrics.values_mut().for_each(|m| *m = blk);
    let path = root.join(BASELINE_REL);
    std::fs::write(&path, b.to_json_string().unwrap()).unwrap();
    path
}

#[test]
fn tighten_never_lowers_file() {
    let tmp = temp_repo();
    let report = dev_report(&tmp.path().join(EVAL_DIR)).unwrap();
    // Baseline below the computed values: raised to exactly the computed ones.
    let path = baseline_with(tmp.path(), 0.0);
    run_tighten(tmp.path()).unwrap();
    let got = read_baseline(&path).unwrap().unwrap();
    for (k, m) in &got.metrics {
        assert_eq!(m, &BaselineBlock::from(&report.keys[k]), "{k}");
    }
    // Baseline above the computed values: file left byte-identical.
    let path = baseline_with(tmp.path(), 2.0);
    let before = std::fs::read(&path).unwrap();
    run_tighten(tmp.path()).unwrap();
    assert_eq!(before, std::fs::read(&path).unwrap());
    // No baseline at all is an error, not a silent bootstrap.
    std::fs::remove_file(&path).unwrap();
    assert!(run_tighten(tmp.path()).is_err());
}

#[test]
fn write_baseline_needs_note_then_bootstraps_epoch_one() {
    let tmp = temp_repo();
    assert!(run_write_baseline(tmp.path(), "  ").is_err());
    assert!(!tmp.path().join(BASELINE_REL).exists());
    run_write_baseline(tmp.path(), "fixture bootstrap").unwrap();
    let load = || {
        read_baseline(&tmp.path().join(BASELINE_REL))
            .unwrap()
            .unwrap()
    };
    let b = load();
    assert_eq!((b.epoch, b.tolerance), (1, 0.01));
    assert!(b.metrics.contains_key("dev/all") && b.metrics.contains_key("dev/seed"));
    run_write_baseline(tmp.path(), "second").unwrap();
    assert_eq!(load().epoch, 2);
}

#[test]
fn index_build_rejects_corpus_with_guid() {
    let tmp = temp_repo();
    let dir = tmp.path().join(EVAL_DIR);
    let guid = read_canary_guid(&dir).unwrap();
    let p = dir.join("corpus.jsonl");
    let mut lines: Vec<serde_json::Value> = std::fs::read_to_string(&p)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let id = lines[0]["_id"].as_str().unwrap().to_string();
    let text = format!("{} {guid}", lines[0]["text"].as_str().unwrap());
    lines[0]["text"] = text.into();
    let body: String = lines.iter().map(|v| format!("{v}\n")).collect();
    std::fs::write(&p, body).unwrap();
    let mut out = Vec::new();
    for err in [
        run_report(&dir, Split::Dev, false, &mut out).unwrap_err(),
        run_query(&dir, "zephyrine", 3, &mut out).unwrap_err(),
        dev_report(&dir).unwrap_err(),
        run_write_baseline(tmp.path(), "x").unwrap_err(),
    ] {
        let msg = format!("{err:#}");
        assert!(msg.contains(&id), "{msg}");
        assert!(!msg.contains(&guid), "GUID must never be printed: {msg}");
    }
    assert!(out.is_empty(), "nothing scored before the assert");
}

fn strings(s: &str) -> Vec<String> {
    s.split_whitespace().map(String::from).collect()
}

fn write(root: &Path, rel: &str, body: &str) {
    let p = root.join(rel);
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn section(guid: &str) -> String {
    format!(
        "# Planted\n\n## Leaky section\n\n{}\nmarker {guid}\n",
        "This section body is deliberately long enough to be extracted as a pair. ".repeat(4)
    )
}

#[test]
fn mens_extract_scan_flags_planted_guid_then_clears() {
    let tmp = temp_repo();
    let root = tmp.path();
    let guid = read_canary_guid(&root.join(EVAL_DIR)).unwrap();
    write(root, "docs/src/planted.md", &section(&guid));
    assert!(root.join("docs/src/planted.md").exists());
    let hits = mens_extract_guid_hits(root, &guid).unwrap();
    assert!(hits.iter().any(|h| h.ends_with("planted.md")), "{hits:?}");
    std::fs::remove_file(root.join("docs/src/planted.md")).unwrap();
    assert!(!root.join("docs/src/planted.md").exists());
    assert!(mens_extract_guid_hits(root, &guid).unwrap().is_empty());
}

#[test]
fn fingerprint_paths_cover_all_tracked_files_but_eval_and_archive() {
    let ls = strings(
        "docs/src/a.md crates/x/README.md docs/src/archive/old.md crates/x/docs/archive/old.md \
         contracts/kb/eval/README.md docs/src/data.json scripts/notes.md",
    );
    assert_eq!(
        fingerprint_paths(&ls),
        strings("docs/src/a.md crates/x/README.md docs/src/data.json scripts/notes.md")
    );
}

#[test]
fn resolve_base_rejects_missing_ref() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let err = resolve_base(&repo, "refs/heads/definitely-missing-kb-eval").unwrap_err();
    assert!(err.to_string().contains("cannot resolve base ref"), "{err}");
    assert!(resolve_base(&repo, "--output=x").is_err());
    assert!(resolve_base(&repo, "HEAD").is_ok());
}

#[test]
fn gate_failures_sorted() {
    let got = sorted_unique(strings(
        "[tighten-only]_b [canary]_z [regression]_a [canary]_z [canary]_a",
    ));
    assert_eq!(
        got,
        strings("[canary]_a [canary]_z [regression]_a [tighten-only]_b")
    );
}

#[test]
fn canary_sweep_names_every_source_and_never_the_guid() {
    let tmp = temp_repo();
    let root = tmp.path();
    let eval = root.join(EVAL_DIR);
    let guid = read_canary_guid(&eval).unwrap();
    let pasted = "zephyrine installer setup walkthrough for the quokka daemon beta channel";
    let q = std::fs::read_to_string(eval.join("queries.jsonl")).unwrap();
    std::fs::write(
        eval.join("queries.jsonl"),
        q.replacen("zephyrine installer setup", pasted, 1),
    )
    .unwrap();
    write(root, "notes/leak.txt", &format!("oops {guid}\n"));
    write(root, "docs/src/planted.md", &section(&guid));
    write(root, "docs/src/pasted.md", &format!("# P\n\n{pasted}\n"));
    write(
        root,
        "crates/x/src/lib.rs",
        &format!("// {}\nfn f() {{}}\n", pasted.to_uppercase()),
    );
    write(root, ".planning/p/PLAN.md", &format!("Step: {pasted}.\n"));
    write(root, "assets/blob.bin", &format!("\0\0{pasted}"));
    write(
        root,
        "mens/data/train.jsonl",
        &format!("{{\"t\":\"{guid}\"}}\n"),
    );
    let tracked = strings(
        "contracts/kb/eval/README.md notes/leak.txt docs/src/planted.md docs/src/pasted.md \
         crates/x/src/lib.rs .planning/p/PLAN.md assets/blob.bin",
    );
    let out = canary_failures(root, &tracked).join("\n");
    for want in [
        "repo-scan: notes/leak.txt",
        "repo-scan: docs/src/planted.md",
        "mens-extractor: docs pair from docs/src/planted.md",
        "mens-jsonl: mens/data/train.jsonl",
        "query-fingerprint: q001 in docs/src/pasted.md",
        "query-fingerprint: q001 in crates/x/src/lib.rs",
        "query-fingerprint: q001 in .planning/p/PLAN.md",
    ] {
        assert!(out.contains(want), "missing {want:?} in:\n{out}");
    }
    assert!(
        !out.contains("assets/blob.bin"),
        "binary files are skipped: {out}"
    );
    assert!(!out.contains("contracts/kb/eval/README.md"), "{out}");
    assert!(!out.contains(&guid), "GUID must never be printed");
    for rel in "notes/leak.txt docs/src/planted.md docs/src/pasted.md mens/data/train.jsonl \
                crates/x/src/lib.rs .planning/p/PLAN.md assets/blob.bin"
        .split_whitespace()
    {
        std::fs::remove_file(root.join(rel)).unwrap();
        assert!(!root.join(rel).exists());
    }
    let clean = canary_failures(root, &["contracts/kb/eval/README.md".to_string()]);
    assert!(clean.is_empty(), "{clean:?}");
}

/// Fake `git`: `ls` answers `ls-tree`, `show` answers `show`; `None` is a non-zero exit.
fn fake_git(
    ls: Option<&'static str>,
    show: Option<String>,
) -> impl Fn(&[&str]) -> Result<String, GitReadError> {
    move |a: &[&str]| {
        let r = if a[0] == "ls-tree" {
            ls.map(str::to_owned)
        } else {
            show.clone()
        };
        r.ok_or(GitReadError::NonZero {
            code: 128,
            stderr: "fatal: simulated".into(),
        })
    }
}

#[test]
fn baseline_at_bootstraps_only_on_proven_absence() {
    let tmp = temp_repo();
    let json = new_epoch(None, &dev_report(&tmp.path().join(EVAL_DIR)).unwrap(), "t")
        .unwrap()
        .to_json_string()
        .unwrap();
    // ls-tree succeeds and lists nothing: absent at base, bootstrap.
    assert!(
        baseline_at(fake_git(Some(""), None), "b")
            .unwrap()
            .is_none()
    );
    // Present and readable.
    assert!(
        baseline_at(fake_git(Some(BASELINE_REL), Some(json)), "b")
            .unwrap()
            .is_some()
    );
    // Listed but `git show` fails (corrupt/missing blob, offline promisor): fail closed.
    let err = baseline_at(fake_git(Some(BASELINE_REL), None), "b").unwrap_err();
    assert!(err.contains("simulated"), "{err}");
    // ls-tree itself fails: fail closed, never a bootstrap.
    assert!(baseline_at(fake_git(None, None), "b").is_err());
    // Listed but unparseable.
    assert!(baseline_at(fake_git(Some(BASELINE_REL), Some("{".into())), "b").is_err());
}

#[test]
fn baseline_at_reads_real_git() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let git = |a: &[&str]| read_only(&repo, a);
    assert!(baseline_at(git, "HEAD").unwrap().is_some());
    // A tree-ish without the path: provably absent.
    assert!(baseline_at(git, "HEAD:docs").unwrap().is_none());
    assert!(baseline_at(git, "refs/heads/definitely-missing-kb-eval").is_err());
}

#[test]
fn ls_files_z_paths_are_split_on_nul_unquoted() {
    let got = split_nul("docs/src/caf\u{e9}.md\0docs/src/a \"q\".md\0docs/src/b\\c.md\0");
    assert_eq!(
        got,
        [
            "docs/src/caf\u{e9}.md",
            "docs/src/a \"q\".md",
            "docs/src/b\\c.md"
        ]
    );
    assert!(split_nul("").is_empty());
    // Real git: `-z` output parses to plain repo-relative paths.
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = read_only(&repo, &["ls-files", "-z", "--", "contracts/kb"]).unwrap();
    assert!(split_nul(&out).contains(&BASELINE_REL.to_string()));
}

#[test]
fn gate_reports_split_and_paraphrase_violations_as_golden() {
    let tmp = temp_repo();
    let root = tmp.path();
    let eval = root.join(EVAL_DIR);
    let golden = |root: &Path| -> Vec<String> {
        gate_failures(root, "HEAD")
            .0
            .into_iter()
            .filter(|f| f.starts_with("[golden]"))
            .collect()
    };
    // Disjoint, covering split: q003 held out.
    let dev = std::fs::read_to_string(eval.join("qrels/dev.tsv")).unwrap();
    let kept: String = dev
        .lines()
        .filter(|l| !l.starts_with("q003"))
        .map(|l| format!("{l}\n"))
        .collect();
    write(root, "contracts/kb/eval/qrels/dev.tsv", &kept);
    let test_tsv = "query-id\tcorpus-id\tscore\nq003\tdocs/src/beta.md\t2\n";
    write(root, "contracts/kb/eval/qrels/test.tsv", test_tsv);
    assert!(golden(root).is_empty(), "{:?}", golden(root));
    // A held-out query also listed in dev.tsv: the dev-only load accepts it, the gate must not.
    write(
        root,
        "contracts/kb/eval/qrels/dev.tsv",
        &format!("{kept}q003\tdocs/src/beta.md\t2\n"),
    );
    let got = golden(root);
    assert!(
        got.iter().any(|f| f.contains("both the dev and test")),
        "{got:?}"
    );
    write(root, "contracts/kb/eval/qrels/dev.tsv", &kept);
    // A dev paraphrase of a held-out seed.
    let q = std::fs::read_to_string(eval.join("queries.jsonl")).unwrap();
    let q: String = q
        .lines()
        .map(|l| {
            let mut v: serde_json::Value = serde_json::from_str(l).unwrap();
            if v["_id"] == "q002" {
                v["metadata"]["paraphrase_of"] = "q003".into();
            }
            format!("{v}\n")
        })
        .collect();
    write(root, "contracts/kb/eval/queries.jsonl", &q);
    let got = golden(root);
    assert!(got.iter().any(|f| f.contains("same split")), "{got:?}");
}

#[test]
fn unreadable_head_baseline_is_not_also_reported_as_deleted() {
    let tmp = temp_repo();
    let root = tmp.path();
    write(root, BASELINE_REL, "{ not json");
    let (fails, _) = gate_failures(root, "HEAD");
    assert!(
        fails.iter().any(|f| f.contains("baseline unreadable")),
        "{fails:?}"
    );
    assert!(
        !fails.iter().any(|f| f.starts_with("[tighten-only]")),
        "{fails:?}"
    );
    // Absent (not unreadable) is still named, even alongside other failures.
    std::fs::remove_file(root.join(BASELINE_REL)).unwrap();
    let (fails, _) = gate_failures(root, "HEAD");
    assert!(
        fails.iter().any(|f| f.contains("no baseline at HEAD")),
        "{fails:?}"
    );
}

#[test]
fn write_baseline_warns_when_the_gate_would_reject_the_epoch() {
    let tmp = temp_repo();
    let prev = new_epoch(None, &dev_report(&tmp.path().join(EVAL_DIR)).unwrap(), "t").unwrap();
    let mut next = prev.clone();
    next.epoch += 1;
    let nz = || GitReadError::NonZero {
        code: 128,
        stderr: "fatal: simulated".into(),
    };
    assert!(
        reepoch_warning(Ok(false), None, &next).is_none(),
        "bootstrap"
    );
    let w = reepoch_warning(Ok(false), Some(&prev), &next).unwrap();
    assert!(w.contains("will reject"), "{w}");
    assert!(reepoch_warning(Ok(true), Some(&prev), &next).is_none());
    assert!(
        reepoch_warning(Err(nz()), Some(&prev), &next)
            .unwrap()
            .contains("simulated")
    );
    next.chunker = "md-h2-512-v2".into();
    assert!(reepoch_warning(Ok(false), Some(&prev), &next).is_none());
}

#[test]
fn canary_redteam_dir_is_outside_index_roots() {
    assert_eq!(redteam_roots_failure(KB_CORPUS_ROOTS), None);
    assert!(check_roots_disjoint(KB_CORPUS_ROOTS, REDTEAM_DIR).is_ok());
    let bad = redteam_roots_failure(&["contracts"]).expect("a covering root must fail");
    assert!(bad.starts_with("[canary] roots(redteam): "), "{bad}");
    assert!(check_roots_disjoint(&["contracts"], REDTEAM_DIR).is_err());
    assert!(redteam_roots_failure(&["contracts/kb/redteam/sub"]).is_some());
    // sibling dir with a shared prefix does not overlap
    assert_eq!(redteam_roots_failure(&["contracts/kb/redteamx"]), None);
}

#[test]
fn canary_failures_do_not_report_redteam_roots_for_default_roots() {
    let tmp = temp_repo();
    let out = canary_failures(tmp.path(), &[]).join("\n");
    assert!(!out.contains("roots(redteam)"), "{out}");
}
