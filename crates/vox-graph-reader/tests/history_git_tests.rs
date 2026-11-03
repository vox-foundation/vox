mod common;
use common::Repo;
use std::collections::HashSet;
use vox_graph_reader::history::git::{BlobReader, Git};

#[test]
fn first_parent_range_skips_branch_commits_and_reads_inner_subjects() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c1 = r.commit("feat: one");
    r.git(&["checkout", "-q", "-b", "side"]);
    r.write("b.txt", "2\n");
    r.commit("wip: inner");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--no-ff", "side", "-m", "merge side"]);
    let m = r.git(&["rev-parse", "HEAD"]).trim().to_string();
    let g = Git::new(r.path());
    assert_eq!(
        g.first_parent_range(None, "main").unwrap(),
        vec![c1.clone(), m.clone()]
    );
    assert_eq!(
        g.first_parent_range(Some(&c1), "main").unwrap(),
        vec![m.clone()]
    );
    assert_eq!(
        g.inner_subjects(&m, None).unwrap(),
        vec!["wip: inner".to_string()]
    );
    assert_eq!(
        g.inner_subjects(&m, Some("b.txt")).unwrap(),
        vec!["wip: inner".to_string()]
    );
    assert!(g.inner_subjects(&m, Some("a.txt")).unwrap().is_empty());
    assert!(g.is_ancestor(&c1, &m));
}

#[test]
fn tip_count_tree_and_common_dir() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c1 = r.commit("one");
    r.write("b.txt", "2\n");
    r.commit("two");
    let g = Git::new(r.path());
    assert_eq!(g.default_tip(), "main", "no origin remote -> local main");
    r.git(&["update-ref", "refs/remotes/origin/main", &c1]);
    assert_eq!(g.default_tip(), "origin/main");
    assert_eq!(g.first_parent_range(None, "main").unwrap().len(), 2);
    assert_eq!(g.first_parent_range(Some(&c1), "main").unwrap().len(), 1);
    assert_eq!(
        g.ls_tree(&c1).unwrap(),
        HashSet::from(["a.txt".to_string()])
    );
    let other = tempfile::tempdir().unwrap();
    let wt = other.path().join("wt");
    r.git(&["worktree", "add", "-q", "-b", "side", wt.to_str().unwrap()]);
    let canon = |p: std::path::PathBuf| p.canonicalize().unwrap();
    assert_eq!(
        canon(Git::new(&wt).common_dir().unwrap()),
        canon(g.common_dir().unwrap()),
        "one store for every worktree"
    );
}

#[test]
fn commit_meta_reads_co_author_trailer() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c =
        r.commit("feat: x\n\nbody line\n\nCo-Authored-By: Claude Opus 5 <noreply@anthropic.com>");
    let m = Git::new(r.path()).commit_meta(&c).unwrap();
    assert_eq!(m.subject, "feat: x");
    assert!(m.parents.is_empty());
    assert_eq!(
        m.co_authors,
        vec!["Claude Opus 5 <noreply@anthropic.com>".to_string()]
    );
}

#[test]
fn file_deltas_report_rename_score_counts_and_whitespace_only() {
    let r = Repo::new();
    r.write("ws.rs", "fn a() {\n    1\n}\n");
    r.write("old.txt", "a\nb\nc\nd\ne\n");
    r.write("real.rs", "fn b() {}\n");
    let c1 = r.commit("init");
    r.write("ws.rs", "fn a() {\n  1\n}\n");
    r.git(&["mv", "old.txt", "new.txt"]);
    r.write("new.txt", "a\nb\nc\nd\ne\nf\n");
    r.write("real.rs", "fn b() { 2 }\n");
    let c2 = r.commit("two");
    let d = Git::new(r.path()).file_deltas(Some(&c1), &c2).unwrap();
    let by = |p: &str| d.iter().find(|f| f.path == p).unwrap().clone();
    let ren = by("new.txt");
    assert_eq!(
        (ren.status.as_str(), ren.old_path.as_deref()),
        ("R", Some("old.txt"))
    );
    assert!(ren.score.unwrap() >= 50);
    assert_eq!((ren.added, ren.deleted), (1, 0));
    assert!(by("ws.rs").whitespace_only);
    assert!(!by("real.rs").whitespace_only);
}

#[test]
fn root_commit_diffs_against_empty_tree() {
    let r = Repo::new();
    r.write("a.txt", "1\n2\n");
    let c = r.commit("init");
    let d = Git::new(r.path()).file_deltas(None, &c).unwrap();
    assert_eq!((d[0].status.as_str(), d[0].added), ("A", 2));
}

#[test]
fn generated_reads_the_commits_own_gitattributes() {
    let r = Repo::new();
    r.write(
        ".gitattributes",
        "*.gen.md linguist-generated=true\nCargo.lock linguist-generated\n",
    );
    let c = r.commit("attrs");
    // An uncommitted worktree edit must not change how commit `c` is classified.
    r.write(".gitattributes", "src/*.rs linguist-generated\n");
    let paths = [
        "x.gen.md".to_string(),
        "Cargo.lock".to_string(),
        "src/a.rs".to_string(),
    ];
    let g = Git::new(r.path()).generated(&c, &paths).unwrap();
    assert!(
        g.contains("x.gen.md") && g.contains("Cargo.lock") && !g.contains("src/a.rs"),
        "{g:?}"
    );
}

#[test]
fn blob_reader_reads_text_and_reports_missing() {
    let r = Repo::new();
    r.write("a.txt", "hello\n");
    let c = r.commit("init");
    let mut b = BlobReader::spawn(r.path()).unwrap();
    assert_eq!(
        b.read(&c, "a.txt", 1 << 20).unwrap().as_deref(),
        Some("hello\n")
    );
    assert_eq!(b.read(&c, "nope.txt", 1 << 20).unwrap(), None);
    assert_eq!(b.read(&c, "a.txt", 2).unwrap(), None, "over max_bytes");
    assert_eq!(
        b.read(&c, "a.txt", 1 << 20).unwrap().as_deref(),
        Some("hello\n"),
        "stream still in sync"
    );
}

#[test]
fn blob_reader_drains_oversized_blob_without_buffering_it() {
    // A genuinely large blob (well past a trivial header/body) must be drained
    // rather than allocated, and must leave the `cat-file --batch` stream in
    // sync for the next read.
    let r = Repo::new();
    let big = "x".repeat(5_000_000);
    r.write("big.txt", &big);
    r.write("small.txt", "hi\n");
    let c = r.commit("init");
    let mut b = BlobReader::spawn(r.path()).unwrap();
    assert_eq!(b.read(&c, "big.txt", 1024).unwrap(), None, "over max_bytes");
    assert_eq!(
        b.read(&c, "small.txt", 1 << 20).unwrap().as_deref(),
        Some("hi\n"),
        "stream still in sync after draining the big blob"
    );
}
