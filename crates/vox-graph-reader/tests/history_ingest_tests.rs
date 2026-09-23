mod common;
use common::Repo;
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::Duration;
use vox_graph_reader::history::git::{BlobReader, Git};
use vox_graph_reader::history::ingest::{CatchUp, catch_up, ingest_commit};
use vox_graph_reader::history::model::{LineageKind, LineageMethod, Reason};
use vox_graph_reader::history::store::HistoryStore;

fn store() -> (tempfile::TempDir, HistoryStore) {
    let d = tempfile::tempdir().unwrap();
    let s = HistoryStore::open(d.path()).unwrap();
    (d, s)
}

fn run(r: &Repo, s: &HistoryStore, budget: Option<Duration>) -> CatchUp {
    let lock = s.try_lock().unwrap().expect("lock");
    catch_up(r.path(), s, &lock, "main", budget, &mut |_, _| {}).unwrap()
}

#[test]
fn mixed_commit_flags_only_the_fmt_drift_files() {
    let r = Repo::new();
    r.write("real.rs", "fn a() -> u8 { 1 }\n");
    r.write("drift1.rs", "fn b() {\n    1;\n}\n");
    r.write("drift2.rs", "use x::y;\nuse z::w;\nfn c() {}\n");
    r.commit("init");
    r.write("real.rs", "fn a() -> u8 { 2 }\n");
    r.write("drift1.rs", "fn b() {\n  1;\n}\n");
    r.write("drift2.rs", "use z::w;\nuse x::y;\nfn c() {}\n");
    let c = r.commit("feat: real change\n\nCo-Authored-By: Claude Opus 5 <noreply@anthropic.com>");
    let g = Git::new(r.path());
    let (commit, changes, _) =
        ingest_commit(&g, &mut BlobReader::spawn(r.path()).unwrap(), &c).unwrap();
    let flag = |p: &str| changes.iter().find(|x| x.path == p).unwrap();
    assert!(!flag("real.rs").mechanical);
    assert!(flag("drift1.rs").reasons.contains(&Reason::Whitespace));
    assert!(flag("drift2.rs").reasons.contains(&Reason::SymbolNeutral));
    assert!(!commit.mechanical);
    assert_eq!(commit.agent.as_deref(), Some("Claude Opus 5"));
}

#[test]
fn catch_up_is_incremental_and_merges_carry_inner_subjects() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    r.commit("feat: one");
    let (_d, s) = store();
    assert_eq!(
        run(&r, &s, None),
        CatchUp::Done {
            ingested: 1,
            rebuilt: false
        }
    );
    r.git(&["checkout", "-q", "-b", "side"]);
    r.write("b.txt", "2\n");
    r.commit("wip: inner");
    r.git(&["checkout", "-q", "main"]);
    r.git(&["merge", "-q", "--no-ff", "side", "-m", "merge side"]);
    assert_eq!(
        run(&r, &s, None),
        CatchUp::Done {
            ingested: 1,
            rebuilt: false
        }
    );
    let d = s.load().unwrap();
    assert_eq!(
        d.commits.len(),
        2,
        "inner branch commit gets no row of its own"
    );
    assert!(d.commits[1].is_merge);
    assert_eq!(d.commits[1].inner_subjects, vec!["wip: inner".to_string()]);
    assert_eq!(
        run(&r, &s, None),
        CatchUp::Done {
            ingested: 0,
            rebuilt: false
        }
    );
}

#[test]
fn rewritten_tip_rolls_back_instead_of_rebuilding() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c1 = r.commit("one");
    r.write("a.txt", "2\n");
    r.commit("two (dropped by the rewrite)");
    let (_d, s) = store();
    assert_eq!(
        run(&r, &s, None),
        CatchUp::Done {
            ingested: 2,
            rebuilt: false
        }
    );
    r.git(&["reset", "-q", "--hard", &c1]);
    r.write("b.txt", "3\n");
    let c3 = r.commit("three");
    assert_eq!(
        run(&r, &s, None),
        CatchUp::Done {
            ingested: 1,
            rebuilt: false
        }
    );
    let shas: Vec<String> = s
        .load()
        .unwrap()
        .commits
        .into_iter()
        .map(|c| c.sha)
        .collect();
    assert_eq!(shas, vec![c1, c3]);
}

#[test]
fn corrupt_state_triggers_full_rebuild() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    let c = r.commit("one");
    let (d, s) = store();
    run(&r, &s, None);
    let st = format!("{{\"schema_version\":1,\"extractor_version\":\"0\",\"last_sha\":\"{c}\"}}");
    std::fs::write(d.path().join("state.json"), st).unwrap();
    assert_eq!(
        run(&r, &s, None),
        CatchUp::Done {
            ingested: 1,
            rebuilt: true
        }
    );
    // AMENDED: #16 — an unparseable state.json used to fail every query forever.
    std::fs::write(d.path().join("state.json"), "not json").unwrap();
    assert_eq!(
        run(&r, &s, None),
        CatchUp::Done {
            ingested: 1,
            rebuilt: true
        }
    );
    assert_eq!(s.load().unwrap().commits.len(), 1);
}

// AMENDED: #20 — without `skip`, the generated source would become a lineage parent of src/b.rs.
#[test]
fn generated_files_never_take_part_in_lineage() {
    let r = Repo::new();
    let fns = |names: &[&str]| {
        names
            .iter()
            .map(|n| format!("fn {n}() {{\n    let v = \"{n}\";\n    drop(v);\n}}\n"))
            .collect::<String>()
    };
    r.write(".gitattributes", "gen/** linguist-generated\n");
    r.write(
        "gen/a.rs",
        &fns(&["g0", "g1", "g2", "g3", "g4", "g5", "g6", "g7", "g8", "g9"]),
    );
    r.commit("init");
    std::fs::remove_file(r.path().join("gen/a.rs")).unwrap();
    // Two of the generated fns plus plenty of new code: too dissimilar for a git rename.
    r.write(
        "src/b.rs",
        &fns(&[
            "g0", "g1", "n0", "n1", "n2", "n3", "n4", "n5", "n6", "n7", "n8", "n9", "n10", "n11",
        ]),
    );
    let c = r.commit("hand-copy two generated fns");
    let (_, _, lineage) = ingest_commit(
        &Git::new(r.path()),
        &mut BlobReader::spawn(r.path()).unwrap(),
        &c,
    )
    .unwrap();
    assert!(lineage.iter().all(|e| e.from != "gen/a.rs"), "{lineage:?}");
}

#[test]
fn zero_budget_stops_before_ingesting_and_reports_behind() {
    let r = Repo::new();
    r.write("a.txt", "1\n");
    r.commit("one");
    r.write("a.txt", "2\n");
    r.commit("two");
    let (_d, s) = store();
    assert_eq!(
        run(&r, &s, Some(Duration::ZERO)),
        CatchUp::OverBudget {
            ingested: 0,
            behind: 2
        }
    );
}

#[test]
fn split_commit_records_lineage() {
    let r = Repo::new();
    let fns = |names: &[&str]| {
        names
            .iter()
            .map(|n| format!("fn {n}() {{\n    let v = \"{n}\";\n    drop(v);\n}}\n"))
            .collect::<String>()
    };
    // 14 fns x 4 lines; keeping 2 deletes 48 lines, safely over MIN_CHURN (40).
    r.write(
        "big.rs",
        &fns(&[
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n",
        ]),
    );
    r.commit("init");
    r.write("big.rs", &fns(&["a", "b"]));
    r.write("part1.rs", &fns(&["c", "d", "e", "f", "g", "h"]));
    r.write("part2.rs", &fns(&["i", "j", "k", "l", "m", "n"]));
    let c = r.commit("refactor: split big.rs");
    let (_, _, lineage) = ingest_commit(
        &Git::new(r.path()),
        &mut BlobReader::spawn(r.path()).unwrap(),
        &c,
    )
    .unwrap();
    let to: Vec<_> = lineage
        .iter()
        .filter(|e| e.from == "big.rs" && e.kind == LineageKind::Split)
        .map(|e| e.to.as_str())
        .collect();
    assert!(
        to.contains(&"part1.rs") && to.contains(&"part2.rs"),
        "{lineage:?}"
    );
}

/// The oldest first-parent commit of `tip` that contains `sha`: what `catch_up` really ingests.
fn carrier(g: &Git, tip: &str, sha: &str) -> String {
    let full = g
        .rev_parse(sha)
        .unwrap_or_else(|_| panic!("{sha} not in this clone: run in a full-history checkout"));
    g.first_parent_range(None, tip)
        .unwrap()
        .into_iter()
        .find(|x| g.is_ancestor(&full, x))
        .expect("carrier on the first-parent line")
}

fn crate_of(p: &str) -> String {
    p.split('/').take(2).collect::<Vec<_>>().join("/")
}

#[test]
#[ignore = "needs full history; run: cargo test -p vox-graph-reader --test history_ingest_tests -- --ignored real_history"]
fn real_history_split_and_merge_are_detected_on_the_carrier() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let g = Git::new(&root);
    let tip = g.default_tip();
    let c = carrier(&g, &tip, "05e42775a");
    assert_eq!(
        c,
        carrier(&g, &tip, "a7aaf48d4"),
        "both changes arrive via one merge"
    );
    let (_, _, lineage) = ingest_commit(&g, &mut BlobReader::spawn(&root).unwrap(), &c).unwrap();

    let engine: Vec<_> = lineage
        .iter()
        .filter(|e| {
            e.from.ends_with("vox-plugin-browser/src/engine.rs") && e.kind == LineageKind::Split
        })
        .collect();
    assert!(
        engine.len() >= 2,
        "engine.rs split into host/input/resolve: {engine:?}"
    );
    assert!(
        engine
            .iter()
            .all(|e| e.to.starts_with("crates/vox-plugin-browser/")),
        "{engine:?}"
    );
    assert!(
        lineage.iter().any(|e| e.kind == LineageKind::Rename),
        "candle fold has git renames"
    );
    assert!(
        lineage
            .iter()
            .any(|e| e.kind == LineageKind::Merge && e.to.contains("candle-core")),
        "cuda+metal duplicates fold into core"
    );

    // Precision: every crate pair of a non-rename edge was reviewed by a human once.
    let pairs: BTreeSet<String> = lineage
        .iter()
        .filter(|e| e.kind != LineageKind::Rename)
        .map(|e| format!("{} -> {}", crate_of(&e.from), crate_of(&e.to)))
        .collect();
    let mut per_target: HashMap<&str, usize> = HashMap::new();
    for e in lineage.iter().filter(|e| e.method == LineageMethod::Line) {
        *per_target.entry(e.to.as_str()).or_default() += 1;
    }
    assert!(
        per_target.values().all(|&n| n <= 3),
        "too many line edges into one target: {per_target:?}"
    );
    // AMENDED: #12 — the golden is seeded by hand; the code under test never writes it.
    // VOX_HISTORY_BLESS only writes a `.new` candidate for the owner to diff and review.
    let golden =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/history_carrier_pairs.txt");
    if std::env::var_os("VOX_HISTORY_BLESS").is_some() {
        let candidate = golden.with_extension("txt.new");
        std::fs::write(
            &candidate,
            pairs.iter().map(|p| format!("{p}\n")).collect::<String>(),
        )
        .unwrap();
        panic!(
            "wrote {candidate:?}: diff it against the golden and get owner review before replacing"
        );
    }
    let allowed: BTreeSet<String> = std::fs::read_to_string(&golden)
        .expect("hand-seeded golden missing")
        .lines()
        .map(str::to_string)
        .collect();
    let unreviewed: Vec<_> = pairs.difference(&allowed).collect();
    assert!(
        unreviewed.is_empty(),
        "unreviewed crate pairs: {unreviewed:?}"
    );
}
