mod common;
use common::Repo;
use std::time::Duration;
use vox_graph_reader::history::api::{HistoryQuery, Paths, answer, store_dir};
use vox_graph_reader::history::git::Git;
use vox_graph_reader::history::ingest::CatchUp;
use vox_graph_reader::history::query::AreaBy;
use vox_graph_reader::history::store::HistoryStore;

const FOCUS: HistoryQuery = HistoryQuery::Focus {
    since_days: 30,
    by: AreaBy::Crate,
    include_mechanical: false,
    limit: 5,
};

fn repo() -> Repo {
    let r = Repo::new();
    r.write("crates/k/src/lib.rs", "fn a() {}\n");
    r.commit("feat: k");
    r
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn store_of(r: &Repo) -> HistoryStore {
    HistoryStore::open(&store_dir(&Git::new(r.path())).unwrap()).unwrap()
}

#[test]
fn answer_catches_up_then_reports_complete_rows() {
    let r = repo();
    let p = Paths {
        repo_root: r.path(),
        code_graph: None,
    };
    let a = answer(&p, &FOCUS, now(), None, &mut |_, _| {}).unwrap();
    assert_eq!(
        a.catch_up,
        CatchUp::Done {
            ingested: 1,
            rebuilt: false
        }
    );
    assert!(a.complete && a.value["complete"] == true && a.value["behind"] == 0);
    assert_eq!(a.value["rows"][0]["area"], "crates/k");
    assert!(
        store_dir(&Git::new(r.path()))
            .unwrap()
            .starts_with(r.path().canonicalize().unwrap().join(".git")),
        "store lives under the repo's git common dir"
    );
}

#[test]
fn brief_never_ingests_and_says_how_far_behind() {
    let r = repo();
    let p = Paths {
        repo_root: r.path(),
        code_graph: None,
    };
    let a = answer(&p, &HistoryQuery::Brief, now(), None, &mut |_, _| {}).unwrap();
    assert_eq!(a.catch_up, CatchUp::Skipped);
    assert!(
        !a.complete && a.text.contains("1 commits behind"),
        "{}",
        a.text
    );
    assert!(store_of(&r).load().unwrap().commits.is_empty());
}

#[test]
fn over_budget_answer_is_marked_partial() {
    let r = repo();
    let p = Paths {
        repo_root: r.path(),
        code_graph: None,
    };
    let a = answer(&p, &FOCUS, now(), Some(Duration::ZERO), &mut |_, _| {}).unwrap();
    assert!(!a.complete && a.value["complete"] == false);
    assert!(
        a.text
            .starts_with("partial: 1 commits not yet ingested; call again."),
        "{}",
        a.text
    );
}

#[test]
fn busy_store_answers_from_disk_and_is_incomplete() {
    let r = repo();
    let s = store_of(&r);
    let _held = s.try_lock().unwrap().expect("lock");
    let a = answer(
        &Paths {
            repo_root: r.path(),
            code_graph: None,
        },
        &FOCUS,
        now(),
        None,
        &mut |_, _| {},
    )
    .unwrap();
    assert_eq!(a.catch_up, CatchUp::Busy);
    assert!(!a.complete);
}
