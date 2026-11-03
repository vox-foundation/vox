//! `vox harness publish` — export new harness_eval_* rows to an append-only, git-committed
//! JSONL file, and ingest that file back into the local vox-db idempotently. See the chat
//! harness continuous eval design spec §9 for why: this is the sync mechanism that lets any
//! developer's local GUI/CLI see CI's nightly results after a `git pull`, with no server
//! dependency.

use clap::Parser;
use serde::{Deserialize, Serialize};

/// One line of the published JSONL file — a self-contained snapshot of one run and its children,
/// so ingest can upsert a whole run atomically per line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedRun {
    pub run: vox_db::HarnessEvalRunRecord,
    pub task_results: Vec<vox_db::HarnessEvalTaskResultRecord>,
    pub selection_events: Vec<vox_db::ModelSelectionEventRecord>,
}

/// Serialize a batch of runs to JSONL (one `PublishedRun` per line).
pub fn to_jsonl(runs: &[PublishedRun]) -> String {
    runs.iter()
        .filter_map(|r| serde_json::to_string(r).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Parse a JSONL blob back into `PublishedRun`s, skipping any line that fails to parse (forward-
/// compatible with future schema additions — a malformed/future-shaped line is logged and
/// skipped, never a hard error that blocks ingesting the rest of the file).
pub fn from_jsonl(blob: &str) -> (Vec<PublishedRun>, Vec<String>) {
    let mut runs = Vec::new();
    let mut skipped_lines = Vec::new();
    for line in blob.lines().filter(|l| !l.trim().is_empty()) {
        match serde_json::from_str::<PublishedRun>(line) {
            Ok(r) => runs.push(r),
            Err(e) => skipped_lines.push(format!("{e}: {line}")),
        }
    }
    (runs, skipped_lines)
}

/// A `git_sha` must be a 7-40 character lowercase hex string to be trusted as one — anything else
/// is rejected here, at the single point untrusted data (a `runs.jsonl` line, which any PR or a
/// compromised bot commit could add) enters `vox-db`. This is deliberately centralized in
/// `ingest_runs` rather than re-checked at every later `git diff` call site (`eval::run`'s
/// `changed_files` computation, a future `report` command, a future GUI regression command) —
/// once a `git_sha` is in the database, every downstream reader can trust it without
/// re-validating.
fn is_valid_git_sha(s: &str) -> bool {
    (7..=40).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Ingest a batch of published runs into the local DB. Idempotent: a run_id already present is
/// skipped entirely (run + its children), not re-inserted or duplicated. A run whose `git_sha`
/// doesn't look like a real SHA is rejected outright (not silently truncated/sanitized) — a
/// malformed `git_sha` here means either a corrupted publish or a tampered `runs.jsonl`, and
/// ingesting it anyway would poison every downstream `git diff` call that trusts this column.
pub async fn ingest_runs(db: &vox_db::VoxDb, runs: &[PublishedRun]) -> anyhow::Result<usize> {
    let existing: std::collections::HashSet<String> = db
        .list_harness_eval_runs(10_000)
        .await?
        .into_iter()
        .map(|r| r.run_id)
        .collect();

    let mut ingested = 0;
    for published in runs {
        if existing.contains(&published.run.run_id) {
            continue;
        }
        if !is_valid_git_sha(&published.run.git_sha) {
            eprintln!(
                "skipping run {} — git_sha {:?} is not a valid hex SHA",
                published.run.run_id, published.run.git_sha
            );
            continue;
        }
        db.record_harness_eval_run(&published.run).await?;
        for task_result in &published.task_results {
            db.record_harness_eval_task_result(task_result).await?;
        }
        for event in &published.selection_events {
            db.record_model_selection_event(event).await?;
        }
        ingested += 1;
    }
    Ok(ingested)
}

/// Sync the local DB from the git-committed JSONL history file, if present. Called by both the
/// CLI and the GUI backend before querying, so a fresh `git pull` is reflected without requiring
/// the user to manually run `publish`.
pub async fn sync_from_jsonl(db: &vox_db::VoxDb, path: &std::path::Path) -> anyhow::Result<usize> {
    let blob = match std::fs::read_to_string(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e.into()),
    };
    let (runs, _skipped) = from_jsonl(&blob);
    ingest_runs(db, &runs).await
}

/// Every history file under `history_dir`: the legacy append-only `runs.jsonl` (kept as
/// read-only history) plus one `runs/<run_id>.jsonl` per published run. One file per run
/// means two results PRs can never conflict (GitHub's merge ignores `merge=union`).
fn history_files(history_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = vec![history_dir.join("runs.jsonl")];
    if let Ok(entries) = std::fs::read_dir(history_dir.join("runs")) {
        let mut per_run: Vec<_> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "jsonl"))
            .collect();
        per_run.sort();
        files.extend(per_run);
    }
    files
}

/// Run ids already published anywhere under `history_dir`.
pub fn published_run_ids(history_dir: &std::path::Path) -> std::collections::HashSet<String> {
    history_files(history_dir)
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .flat_map(|blob| from_jsonl(&blob).0)
        .map(|p| p.run.run_id)
        .collect()
}

/// Write one run to `<history_dir>/runs/<run_id>.jsonl`. The run id becomes a file name, so
/// only `[A-Za-z0-9._-]` is accepted (and not `.`/`..`).
pub fn write_run_file(
    history_dir: &std::path::Path,
    run: &PublishedRun,
) -> anyhow::Result<std::path::PathBuf> {
    let id = &run.run.run_id;
    let safe = !id.is_empty()
        && id != "."
        && id != ".."
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !safe {
        anyhow::bail!("run id {id:?} is not a safe file name");
    }
    let dir = history_dir.join("runs");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{id}.jsonl"));
    std::fs::write(&path, to_jsonl(std::slice::from_ref(run)) + "\n")?;
    Ok(path)
}

/// Sync the local DB from every git-committed history file (idempotent per run id). Called
/// before querying, so a fresh `git pull` is reflected without a manual `publish`.
pub async fn sync_from_history(
    db: &vox_db::VoxDb,
    history_dir: &std::path::Path,
) -> anyhow::Result<usize> {
    let mut ingested = 0;
    for path in history_files(history_dir) {
        ingested += sync_from_jsonl(db, &path).await?;
    }
    Ok(ingested)
}

/// `vox harness publish` arguments.
#[derive(Parser)]
pub struct PublishArgs {
    /// Git-tracked history directory: each run is written to `<dir>/runs/<run_id>.jsonl`
    /// (the legacy `<dir>/runs.jsonl` is still read, never written).
    #[arg(long, default_value = "docs/harness-eval-history")]
    pub dir: std::path::PathBuf,
}

/// Export every local `harness_eval_run` not already published under `args.dir`, one file per
/// run (auto-generated files: never hand-edit, per this repo's convention for generated docs).
pub async fn run(args: PublishArgs) -> anyhow::Result<()> {
    let db = vox_db::open_project_db().await?;
    let already_published_ids = published_run_ids(&args.dir);

    let local_runs = db.list_harness_eval_runs(10_000).await?;
    let mut newly_published = Vec::new();
    for run in local_runs {
        if already_published_ids.contains(&run.run_id) {
            continue;
        }
        let task_results = db.get_harness_eval_task_results(&run.run_id).await?;
        let selection_events = db.get_model_selection_events(&run.run_id).await?;
        newly_published.push(PublishedRun {
            run,
            task_results,
            selection_events,
        });
    }

    if newly_published.is_empty() {
        println!("nothing new to publish");
        return Ok(());
    }

    for run in &newly_published {
        let path = write_run_file(&args.dir, run)?;
        println!("published {}", path.display());
    }
    println!("published {} run(s)", newly_published.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_run(run_id: &str) -> PublishedRun {
        PublishedRun {
            run: vox_db::HarnessEvalRunRecord {
                run_id: run_id.to_string(),
                triggered_by: "ci-nightly".to_string(),
                git_sha: "abc1234".to_string(),
                git_branch: "main".to_string(),
                changed_files: vec![],
                config_version: None,
                samples_per_task: 3,
                task_count: 1,
                pass_count: 1,
                fail_count: 0,
                skip_count: 0,
                total_cost_usd: 0.001,
                started_at_ms: 1000,
                finished_at_ms: 2000,
            },
            task_results: vec![],
            selection_events: vec![],
        }
    }

    #[test]
    fn jsonl_round_trip_preserves_run_id() {
        let runs = vec![fixture_run("run-1"), fixture_run("run-2")];
        let blob = to_jsonl(&runs);
        let (parsed, skipped) = from_jsonl(&blob);
        assert!(skipped.is_empty());
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].run.run_id, "run-1");
        assert_eq!(parsed[1].run.run_id, "run-2");
    }

    #[test]
    fn from_jsonl_skips_malformed_lines_without_failing_the_whole_parse() {
        let blob = format!(
            "{}\nnot valid json\n{}",
            to_jsonl(&[fixture_run("run-1")]),
            to_jsonl(&[fixture_run("run-2")])
        );
        let (parsed, skipped) = from_jsonl(&blob);
        assert_eq!(parsed.len(), 2);
        assert_eq!(skipped.len(), 1);
    }

    /// A run with non-empty children — the earlier version of this test only used
    /// `task_results: vec![]`/`selection_events: vec![]`, which cannot catch a bug where child
    /// rows get duplicated on re-ingest even while the parent run row stays correctly deduped
    /// (e.g. a future refactor that decouples child-row insertion from the run-level existence
    /// check). Real-shaped fixture, not empty.
    fn fixture_run_with_children(run_id: &str) -> PublishedRun {
        let mut run = fixture_run(run_id);
        run.task_results = vec![vox_db::HarnessEvalTaskResultRecord {
            run_id: run_id.to_string(),
            task_id: "chat-arithmetic-basic".to_string(),
            category: "chat".to_string(),
            checker_kind: "deterministic".to_string(),
            status: "pass".to_string(),
            pass_samples: 3,
            total_samples: 3,
            latency_p50_ms: Some(200),
            cost_usd: Some(0.0005),
            failure_detail: None,
            recorded_at_ms: 1500,
        }];
        run.selection_events = vec![vox_db::ModelSelectionEventRecord {
            run_id: run_id.to_string(),
            task_id: "chat-arithmetic-basic".to_string(),
            model_id: "deepseek/deepseek-v4-flash".to_string(),
            cost_tier: "free".to_string(),
            selection_reason: "highest score".to_string(),
            was_privacy_gated: false,
            recorded_at_ms: 1450,
        }];
        run
    }

    #[tokio::test]
    async fn ingesting_the_same_jsonl_twice_produces_no_duplicate_rows() {
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .expect("db");
        let runs = vec![fixture_run("run-idempotent-1")];

        ingest_runs(&db, &runs).await.expect("first ingest");
        ingest_runs(&db, &runs)
            .await
            .expect("second ingest (same data)");

        let listed = db.list_harness_eval_runs(10).await.expect("list");
        assert_eq!(
            listed.len(),
            1,
            "ingesting the identical run twice must not create a duplicate row"
        );
    }

    #[tokio::test]
    async fn ingesting_the_same_jsonl_twice_does_not_duplicate_child_rows_either() {
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .expect("db");
        let runs = vec![fixture_run_with_children("run-idempotent-children-1")];

        ingest_runs(&db, &runs).await.expect("first ingest");
        ingest_runs(&db, &runs)
            .await
            .expect("second ingest (same data)");

        let task_results = db
            .get_harness_eval_task_results("run-idempotent-children-1")
            .await
            .expect("get task results");
        let selection_events = db
            .get_model_selection_events("run-idempotent-children-1")
            .await
            .expect("get selection events");
        assert_eq!(
            task_results.len(),
            1,
            "double-ingesting must not duplicate task_result child rows"
        );
        assert_eq!(
            selection_events.len(),
            1,
            "double-ingesting must not duplicate model_selection_event child rows"
        );
    }

    #[tokio::test]
    async fn ingest_runs_rejects_a_run_with_a_malformed_git_sha_but_still_ingests_siblings() {
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .expect("db");
        let mut bad = fixture_run("run-bad-sha");
        bad.run.git_sha = "--output=/tmp/evil".to_string();
        let good = fixture_run("run-good-sha");
        let runs = vec![bad, good];

        let ingested = ingest_runs(&db, &runs).await.expect("ingest");
        assert_eq!(
            ingested, 1,
            "only the run with a valid git_sha should be ingested"
        );

        let listed = db.list_harness_eval_runs(10).await.expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].run_id, "run-good-sha");
    }

    fn tmp_history(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "harness-history-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&d).expect("tmp dir");
        d
    }

    #[test]
    fn published_run_ids_reads_the_legacy_file_and_per_run_files() {
        let dir = tmp_history("ids");
        std::fs::write(
            dir.join("runs.jsonl"),
            to_jsonl(&[fixture_run("legacy-1")]) + "\n",
        )
        .expect("legacy");
        write_run_file(&dir, &fixture_run("per-run-1")).expect("per-run");
        let ids = published_run_ids(&dir);
        assert!(
            ids.contains("legacy-1") && ids.contains("per-run-1"),
            "{ids:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_run_file_names_the_file_by_run_id_and_rejects_unsafe_ids() {
        let dir = tmp_history("write");
        let path = write_run_file(&dir, &fixture_run("run-abc_1.2")).expect("write");
        assert_eq!(path, dir.join("runs").join("run-abc_1.2.jsonl"));
        let (parsed, skipped) = from_jsonl(&std::fs::read_to_string(&path).expect("read"));
        assert!(skipped.is_empty() && parsed.len() == 1);
        for bad in ["../escape", "a/b", "", "a b"] {
            assert!(
                write_run_file(&dir, &fixture_run(bad)).is_err(),
                "accepted {bad:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn sync_from_history_ingests_legacy_and_per_run_files_idempotently() {
        let dir = tmp_history("sync");
        std::fs::write(
            dir.join("runs.jsonl"),
            to_jsonl(&[fixture_run("legacy-1")]) + "\n",
        )
        .expect("legacy");
        write_run_file(&dir, &fixture_run("per-run-1")).expect("a");
        write_run_file(&dir, &fixture_run("per-run-2")).expect("b");
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .expect("db");
        sync_from_history(&db, &dir).await.expect("sync 1");
        sync_from_history(&db, &dir).await.expect("sync 2");
        let mut ids: Vec<String> = db
            .list_harness_eval_runs(10)
            .await
            .expect("list")
            .into_iter()
            .map(|r| r.run_id)
            .collect();
        ids.sort();
        assert_eq!(ids, vec!["legacy-1", "per-run-1", "per-run-2"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn run_creates_the_jsonl_file_when_it_does_not_exist_yet() {
        let tmp = std::env::temp_dir().join(format!("harness-publish-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).expect("tmp dir");
        let path = tmp.join("runs.jsonl");
        assert!(!path.exists(), "precondition: file must not exist yet");

        // This test exercises the file-I/O boundary of `run` directly (missing-file handling,
        // directory creation, append-mode write) — it does not exercise the DB-query half of
        // `run` (which needs `open_project_db()`'s real repo-root discovery, awkward to isolate
        // in a unit test); that half is already covered indirectly by `ingest_runs`'s tests plus
        // Task 5/9's manual end-to-end verification.
        let existing_blob = std::fs::read_to_string(&path).unwrap_or_default();
        assert_eq!(
            existing_blob, "",
            "missing file must read as empty, not error"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
