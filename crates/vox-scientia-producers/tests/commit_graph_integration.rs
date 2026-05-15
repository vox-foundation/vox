//! Integration: commit-graph producer against a synthetic temp git repo.

// `std::process::Command` is referenced via fully-qualified path in `git()`
// below so the inline `vox-arch-check: allow git-exec` marker lands on the
// same line as the path segment, as the lint expects.
use tempfile::tempdir;
use vox_research_events::ResearchEvent;
use vox_scientia_producers::{CommitGraphProducer, Producer, ProducerContext};

fn git(dir: &std::path::Path, args: &[&str]) {
    // Test-only fixture for a synthetic throw-away repo; not production
    // behavior. The producer under test exclusively uses `gix` via
    // `gix::open` — the line below uses the inline allow marker
    // (same line as the path segment) that `vox-arch-check` recognizes.
    let status = std::process::// vox-arch-check: allow git-exec
    Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .expect("git on PATH");
    assert!(status.success(), "git {args:?} failed");
}

fn make_synthetic_repo() -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    let p = tmp.path();
    git(p, &["init", "-q", "-b", "main"]);
    git(p, &["config", "user.email", "test@vox-scientia-producers"]);
    git(p, &["config", "user.name", "Test"]);
    // Initial commit.
    std::fs::write(p.join("README.md"), "hello\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "chore: initial"]);
    // A perf commit — should emit one candidate.
    std::fs::write(p.join("a.txt"), "fast\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "perf: shave p95 by 20%"]);
    // A non-matching commit — should not emit.
    std::fs::write(p.join("b.txt"), "x\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "fix: typo"]);
    // A refactor commit — should emit one candidate.
    std::fs::write(p.join("c.txt"), "y\n").unwrap();
    git(p, &["add", "."]);
    git(p, &["commit", "-q", "-m", "refactor: extract producer"]);
    tmp
}

#[tokio::test]
async fn commit_graph_emits_for_perf_and_refactor_subjects() {
    let repo = make_synthetic_repo();
    let producer = CommitGraphProducer::new();
    let ctx = ProducerContext {
        repo_root: repo.path().to_path_buf(),
        commit_window: 50,
        days_window: 30,
        now_ms: 1_747_000_000_000,
        session_id: "integration".into(),
        repository_id: Some("synthetic".into()),
    };
    let events = producer.observe(&ctx).await;
    let ids: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            ResearchEvent::FindingCandidateProposed { finding_id, .. } => Some(finding_id.as_str()),
            _ => None,
        })
        .collect();
    let has_algimp = ids.iter().any(|id| id.starts_with("algimp-"));
    let has_repinf = ids.iter().any(|id| id.starts_with("repinf-"));
    assert!(has_algimp, "expected an algimp candidate; got {ids:?}");
    assert!(has_repinf, "expected a repinf candidate; got {ids:?}");
    // No matched candidate for "fix: typo" or "chore: initial".
    assert_eq!(
        events.len(),
        2,
        "exactly two matched commits expected; got {ids:?}"
    );
}

#[tokio::test]
async fn commit_graph_finding_id_stable_across_runs_on_fixed_now_ms() {
    let repo = make_synthetic_repo();
    let producer = CommitGraphProducer::new();
    let ctx = ProducerContext {
        repo_root: repo.path().to_path_buf(),
        commit_window: 50,
        days_window: 30,
        now_ms: 1_747_000_000_000,
        session_id: "integration".into(),
        repository_id: Some("synthetic".into()),
    };
    let a = producer.observe(&ctx).await;
    let b = producer.observe(&ctx).await;
    assert_eq!(
        a, b,
        "commit-graph producer must be deterministic across runs"
    );
}
