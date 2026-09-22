//! `vox ci status` — GitHub CI state pushed into agent context by hooks
//! (git pre-commit/pre-push via lefthook; Claude Code SessionStart and
//! UserPromptSubmit), so no agent has to remember to ask. Two signals:
//!
//! 1. failed or timed-out jobs on the current branch's latest head commit,
//!    with the failing step (or the step running when the timeout hit) and the
//!    three slowest steps — enough to cache, shard, or move a job to nightly;
//! 2. open `nightly-failure` issues (opened by `.github/workflows/nightly-report.yml`).
//!
//! Hook modes read a per-branch cache and refresh it in a detached background
//! process, so they never block and never fail the hook.
//! Spec: docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use vox_cli_ci::constants::REPO_SLUG;
use vox_cli_ci::job_timings::run_seconds;

const CACHE_FRESH_SECS: i64 = 120;
/// Bound on the live fetch in pre-push so an offline push never stalls.
const PUSH_FETCH_TIMEOUT: Duration = Duration::from_secs(10);
/// GitHub's annotation text on a job killed by `timeout-minutes`.
const TIMEOUT_MARKER: &str = "exceeded the maximum execution time";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProblemKind {
    Failed,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobProblem {
    pub workflow: String,
    pub job: String,
    pub job_id: u64,
    pub kind: ProblemKind,
    /// The failed step, or the step still running when the timeout hit.
    pub step: Option<String>,
    /// Up to three `(step, seconds)`, slowest first.
    pub slowest: Vec<(String, i64)>,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NightlyIssue {
    pub number: u64,
    pub title: String,
    pub url: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CiStatus {
    pub generated_at: i64,
    pub branch: String,
    pub head_sha: Option<String>,
    pub problems: Vec<JobProblem>,
    pub nightly: Vec<NightlyIssue>,
}

#[derive(Debug, Deserialize)]
struct JobsResponse {
    jobs: Vec<Job>,
}

#[derive(Debug, Deserialize)]
struct Job {
    id: u64,
    name: String,
    #[serde(default)]
    conclusion: Option<String>,
    /// The API schema allows `null`.
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
struct Step {
    name: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    conclusion: Option<String>,
    #[serde(default)]
    started_at: Option<String>,
    #[serde(default)]
    completed_at: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunRow {
    database_id: u64,
    workflow_name: String,
    status: String,
    /// `""` while a run is in progress.
    conclusion: Option<String>,
    head_sha: String,
}

/// `cancelled` without a timeout annotation is a concurrency cancel (a newer
/// push superseded the run) — not a problem worth an agent's attention.
fn classify(conclusion: Option<&str>, annotations: &[String]) -> Option<ProblemKind> {
    let timed_out = annotations.iter().any(|a| a.contains(TIMEOUT_MARKER));
    match conclusion {
        Some("timed_out") => Some(ProblemKind::TimedOut),
        Some("failure" | "cancelled") if timed_out => Some(ProblemKind::TimedOut),
        Some("failure" | "startup_failure") => Some(ProblemKind::Failed),
        _ => None,
    }
}

fn job_problem(workflow: &str, job: &Job, kind: ProblemKind) -> JobProblem {
    let step = job
        .steps
        .iter()
        // Last, not first: the Jobs API reports a `continue-on-error: true`
        // step's own conclusion as "failure" even though it doesn't fail the
        // job. GitHub skips every step after a genuinely blocking failure, so
        // the last "failure" in array order is the real one; an earlier
        // "failure" followed by more steps that actually ran is a soft step.
        .rev()
        .find(|s| s.conclusion.as_deref() == Some("failure"))
        .or_else(|| {
            job.steps
                .iter()
                .find(|s| s.status == "in_progress" || s.conclusion.as_deref() == Some("cancelled"))
        })
        .map(|s| s.name.clone());
    let mut slowest: Vec<(String, i64)> = job
        .steps
        .iter()
        .filter_map(|s| {
            let secs = run_seconds(s.started_at.as_deref(), s.completed_at.as_deref())?;
            Some((s.name.clone(), secs))
        })
        .collect();
    slowest.sort_by_key(|(_, secs)| std::cmp::Reverse(*secs));
    slowest.truncate(3);
    JobProblem {
        workflow: workflow.into(),
        job: job.name.clone(),
        job_id: job.id,
        kind,
        step,
        slowest,
        url: job.html_url.clone().unwrap_or_default(),
    }
}

/// Empty string when there is nothing to report (hooks then print nothing).
fn render(s: &CiStatus) -> String {
    let mut out = Vec::new();
    for p in &s.problems {
        let what = match p.kind {
            ProblemKind::TimedOut => "TIMED OUT",
            ProblemKind::Failed => "FAILED",
        };
        out.push(format!(
            "{what} on {}: {} / {} at step `{}` -> {}",
            s.branch,
            p.workflow,
            p.job,
            p.step.as_deref().unwrap_or("?"),
            p.url
        ));
        match p.kind {
            ProblemKind::TimedOut => {
                let slow: Vec<String> = p
                    .slowest
                    .iter()
                    .map(|(n, secs)| format!("{n} {}m{}s", secs / 60, secs % 60))
                    .collect();
                let slow = if slow.is_empty() {
                    "unknown".to_string()
                } else {
                    slow.join(", ")
                };
                out.push(format!(
                    "  slowest steps: {slow}. Fix: cache or shard the slow step, or move the job to \
                     nightly.yml — do not raise the cap (workflow-policy-guard)."
                ));
            }
            ProblemKind::Failed => {
                out.push(format!(
                    "  log: gh run view --job {} --log-failed",
                    p.job_id
                ));
            }
        }
    }
    for i in &s.nightly {
        // The issue title already starts with "Nightly failing:" — strip it so the
        // rendered line doesn't repeat "failing".
        let title = i
            .title
            .strip_prefix("Nightly failing: ")
            .unwrap_or(&i.title);
        out.push(format!(
            "NIGHTLY FAILING: {} (#{}) -> {}",
            title, i.number, i.url
        ));
    }
    if out.is_empty() {
        return String::new();
    }
    out.insert(0, "GitHub CI (auto-injected by vox hooks):".into());
    out.join("\n")
}

/// What the per-prompt hook prints, given what this session last saw.
fn change_message(last_shown: Option<&str>, current: &str) -> Option<String> {
    let prev = last_shown.unwrap_or("");
    if prev == current {
        None
    } else if current.is_empty() {
        Some("GitHub CI: previously reported problems are resolved.".into())
    } else {
        Some(current.to_string())
    }
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Claude Code hooks receive JSON on stdin with a `session_id`.
fn session_id(stdin: &str) -> String {
    serde_json::from_str::<serde_json::Value>(stdin)
        .ok()
        .and_then(|v| v.get("session_id")?.as_str().map(|s| s.to_string()))
        .map(|s| sanitize(s.rsplit('/').next().unwrap_or("")))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "default".into())
}

fn cache_dir() -> PathBuf {
    vox_config::paths::dot_vox_user_dir().join("ci-status")
}

fn cache_path(branch: &str) -> PathBuf {
    cache_dir().join(format!("{}.json", sanitize(branch)))
}

fn write_cache(s: &CiStatus) -> Result<()> {
    let path = cache_path(&s.branch);
    std::fs::create_dir_all(cache_dir())?;
    // Temp + rename: parallel sessions refresh concurrently; no torn reads.
    let tmp = path.with_extension(format!("json.{}", std::process::id()));
    std::fs::write(&tmp, serde_json::to_vec(s)?)?;
    if let Err(e) = std::fs::rename(&tmp, &path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e.into());
    }
    Ok(())
}

fn read_cache(branch: &str) -> Option<CiStatus> {
    serde_json::from_slice(&std::fs::read(cache_path(branch)).ok()?).ok()
}

fn gh(args: &[&str]) -> Result<String> {
    let out = Command::new("gh")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .context("spawn gh")?;
    if !out.status.success() {
        return Err(anyhow!(
            "gh {}: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn annotations(job_id: u64) -> Vec<String> {
    // --paginate: unpaginated, `gh api` returns only the first 30 — enough
    // steps of noise ahead of a timeout-kill annotation and the marker
    // `classify()` looks for silently falls off the page.
    gh(&[
        "api",
        "--paginate",
        &format!("repos/{REPO_SLUG}/check-runs/{job_id}/annotations"),
        "--jq",
        ".[].message",
    ])
    .map(|o| o.lines().map(str::to_string).collect())
    .unwrap_or_default()
}

fn fetch(branch: &str, now: i64) -> Result<CiStatus> {
    let runs: Vec<RunRow> = serde_json::from_str(&gh(&[
        "run",
        "list",
        "--repo",
        REPO_SLUG,
        "--branch",
        branch,
        "--limit",
        "30",
        "--json",
        "databaseId,workflowName,status,conclusion,headSha",
    ])?)?;
    let head_sha = runs.first().map(|r| r.head_sha.clone());
    let mut problems = Vec::new();
    for r in runs
        .iter()
        .filter(|r| Some(&r.head_sha) == head_sha.as_ref() && r.status == "completed")
    {
        if matches!(
            r.conclusion.as_deref(),
            Some("success" | "skipped" | "neutral")
        ) {
            continue;
        }
        // One unreadable run must not hide the others.
        let Ok(text) = gh(&[
            "api",
            &format!(
                "repos/{REPO_SLUG}/actions/runs/{}/jobs?per_page=100",
                r.database_id
            ),
        ]) else {
            continue;
        };
        let Ok(jobs) = serde_json::from_str::<JobsResponse>(&text) else {
            continue;
        };
        for job in &jobs.jobs {
            let ann = match job.conclusion.as_deref() {
                Some("failure" | "cancelled") => annotations(job.id),
                _ => Vec::new(),
            };
            if let Some(kind) = classify(job.conclusion.as_deref(), &ann) {
                problems.push(job_problem(&r.workflow_name, job, kind));
            }
        }
    }
    let nightly: Vec<NightlyIssue> = serde_json::from_str(&gh(&[
        "issue",
        "list",
        "--repo",
        REPO_SLUG,
        "--label",
        "nightly-failure",
        "--state",
        "open",
        "--json",
        "number,title,url",
    ])?)?;
    Ok(CiStatus {
        generated_at: now,
        branch: branch.into(),
        head_sha,
        problems,
        nightly,
    })
}

fn current_branch() -> Option<String> {
    // vox-arch-check: allow git-exec
    let out = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let branch = String::from_utf8_lossy(&out.stdout).trim().to_string();
    // `--abbrev-ref HEAD` on a detached checkout prints "HEAD" literally —
    // not a real branch `gh run list --branch` should ever query.
    (!branch.is_empty() && branch != "HEAD").then_some(branch)
}

/// Detached `vox ci status` (stdout discarded). A marker file suppresses a
/// stampede of refreshes from parallel sessions while one is in flight.
// ponytail: on Windows the child holds target/debug/vox.exe open for a few
// seconds; add creation_flags(DETACHED_PROCESS) if that blocks rebuilds.
fn spawn_refresh(branch: &str) {
    let marker = cache_dir().join(format!("refreshing-{}", sanitize(branch)));
    let in_flight = std::fs::metadata(&marker)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.elapsed().ok())
        .is_some_and(|age| age.as_secs() < CACHE_FRESH_SECS as u64);
    if in_flight {
        return;
    }
    let _ = std::fs::create_dir_all(cache_dir());
    let _ = std::fs::write(&marker, b"");
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(exe)
            .args(["ci", "status"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
    }
}

pub struct StatusArgs {
    pub hook: bool,
    pub changed_only: bool,
}

pub fn run(args: StatusArgs) -> Result<()> {
    // "DETACHED", not "HEAD" — matches status_writer::current_branch's
    // fallback, and never queries `gh run list --branch` with the literal
    // string a detached checkout's own git prints.
    let branch = current_branch().unwrap_or_else(|| "DETACHED".into());
    let now = chrono::Utc::now().timestamp();
    if !(args.hook || args.changed_only) {
        let s = fetch(&branch, now)?;
        write_cache(&s)?;
        let _ = std::fs::remove_file(cache_dir().join(format!("refreshing-{}", sanitize(&branch))));
        let r = render(&s);
        if r.is_empty() {
            println!("GitHub CI: nothing failing on {branch}; no open nightly failures.");
        } else {
            println!("{r}");
        }
        return Ok(());
    }
    // Hook modes: never block, never fail the hook.
    let cached = read_cache(&branch);
    if cached
        .as_ref()
        .is_none_or(|c| now - c.generated_at > CACHE_FRESH_SECS)
    {
        spawn_refresh(&branch);
    }
    let current = cached.as_ref().map(render).unwrap_or_default();
    if args.changed_only {
        let mut stdin = String::new();
        if !std::io::stdin().is_terminal() {
            let _ = std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin);
        }
        let shown = cache_dir().join(format!("shown-{}.txt", session_id(&stdin)));
        if let Some(msg) = change_message(std::fs::read_to_string(&shown).ok().as_deref(), &current)
        {
            println!("{msg}");
        }
        let _ = std::fs::create_dir_all(cache_dir());
        let _ = std::fs::write(&shown, &current);
    } else if !current.is_empty() {
        println!("{current}");
    }
    Ok(())
}

/// Live fetch for `vox ci pre-push`, bounded by [`PUSH_FETCH_TIMEOUT`]; falls
/// back to the cached block. Silent inside GitHub Actions.
pub(crate) fn print_live_for_push() {
    if std::env::var_os("GITHUB_ACTIONS").is_some() {
        return;
    }
    let Some(branch) = current_branch() else {
        return;
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let b = branch.clone();
    std::thread::spawn(move || {
        let _ = tx.send(fetch(&b, chrono::Utc::now().timestamp()));
    });
    let s = match rx.recv_timeout(PUSH_FETCH_TIMEOUT) {
        Ok(Ok(s)) => {
            let _ = write_cache(&s);
            s
        }
        Ok(Err(e)) => {
            println!("pre-push: GitHub CI status unavailable ({e}); showing cached");
            match read_cache(&branch) {
                Some(c) => c,
                None => return,
            }
        }
        Err(_) => match read_cache(&branch) {
            Some(c) => c,
            None => return,
        },
    };
    let r = render(&s);
    if !r.is_empty() {
        println!("{r}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn step(
        name: &str,
        status: &str,
        conclusion: Option<&str>,
        start: &str,
        end: Option<&str>,
    ) -> Step {
        Step {
            name: name.into(),
            status: status.into(),
            conclusion: conclusion.map(Into::into),
            started_at: Some(start.into()),
            completed_at: end.map(Into::into),
        }
    }

    #[test]
    fn classify_distinguishes_timeout_failure_and_supersede() {
        let timeout = vec![
            "The job running on runner X has exceeded the maximum execution time of 30 minutes."
                .to_string(),
        ];
        assert_eq!(
            classify(Some("cancelled"), &timeout),
            Some(ProblemKind::TimedOut)
        );
        assert_eq!(
            classify(Some("failure"), &timeout),
            Some(ProblemKind::TimedOut)
        );
        assert_eq!(
            classify(Some("timed_out"), &[]),
            Some(ProblemKind::TimedOut)
        );
        assert_eq!(classify(Some("failure"), &[]), Some(ProblemKind::Failed));
        // A concurrency cancel (newer push) is not a problem.
        assert_eq!(classify(Some("cancelled"), &[]), None);
        assert_eq!(classify(Some("success"), &[]), None);
        assert_eq!(classify(None, &[]), None);
    }

    #[test]
    fn job_problem_names_running_step_and_slowest_three() {
        let job = Job {
            id: 42,
            name: "Check, Build, and Test (Rust)".into(),
            conclusion: Some("cancelled".into()),
            html_url: Some("https://github.com/x/y/actions/runs/1/job/42".into()),
            steps: vec![
                step(
                    "Checkout",
                    "completed",
                    Some("success"),
                    "2026-09-21T10:00:00Z",
                    Some("2026-09-21T10:00:10Z"),
                ),
                step(
                    "Build vox CLI",
                    "completed",
                    Some("success"),
                    "2026-09-21T10:00:10Z",
                    Some("2026-09-21T10:12:10Z"),
                ),
                step(
                    "Clippy (affected)",
                    "completed",
                    Some("success"),
                    "2026-09-21T10:12:10Z",
                    Some("2026-09-21T10:17:10Z"),
                ),
                step(
                    "Tests (affected)",
                    "completed",
                    Some("cancelled"),
                    "2026-09-21T10:17:10Z",
                    Some("2026-09-21T10:30:00Z"),
                ),
            ],
        };
        let p = job_problem("CI", &job, ProblemKind::TimedOut);
        assert_eq!(p.step.as_deref(), Some("Tests (affected)"));
        assert_eq!(p.job_id, 42);
        let names: Vec<&str> = p.slowest.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            ["Tests (affected)", "Build vox CLI", "Clippy (affected)"]
        );
        assert_eq!(p.slowest[1].1, 720);
    }

    #[test]
    fn job_problem_prefers_the_failed_step() {
        let job = Job {
            id: 7,
            name: "gate".into(),
            conclusion: Some("failure".into()),
            html_url: None,
            steps: vec![
                step(
                    "Clippy (affected)",
                    "completed",
                    Some("failure"),
                    "2026-09-21T10:00:00Z",
                    Some("2026-09-21T10:01:00Z"),
                ),
                step(
                    "Tests (affected)",
                    "completed",
                    Some("skipped"),
                    "2026-09-21T10:01:00Z",
                    Some("2026-09-21T10:01:00Z"),
                ),
            ],
        };
        let p = job_problem("CI", &job, ProblemKind::Failed);
        assert_eq!(p.step.as_deref(), Some("Clippy (affected)"));
        assert_eq!(p.url, "");
    }

    /// A `continue-on-error: true` step reports its own conclusion as
    /// "failure" too, but the job keeps running past it — the real blocking
    /// step is whichever "failure" comes LAST, since nothing runs after it.
    #[test]
    fn job_problem_prefers_the_last_failed_step_over_a_soft_earlier_one() {
        let job = Job {
            id: 9,
            name: "tests".into(),
            conclusion: Some("failure".into()),
            html_url: None,
            steps: vec![
                step(
                    "Unused dependency scan (non-blocking)",
                    "completed",
                    Some("failure"),
                    "2026-09-21T10:00:00Z",
                    Some("2026-09-21T10:01:00Z"),
                ),
                step(
                    "Serving path unit tests",
                    "completed",
                    Some("failure"),
                    "2026-09-21T10:01:00Z",
                    Some("2026-09-21T10:05:00Z"),
                ),
            ],
        };
        let p = job_problem("CI", &job, ProblemKind::Failed);
        assert_eq!(p.step.as_deref(), Some("Serving path unit tests"));
    }

    #[test]
    fn real_gh_json_shapes_parse() {
        let jobs: JobsResponse = serde_json::from_str(
            r#"{"total_count":1,"jobs":[{"id":1,"name":"g","conclusion":null,"html_url":null,
                "steps":[{"name":"s","status":"in_progress","conclusion":null,"number":1,
                "started_at":"2026-09-21T10:00:00Z","completed_at":null}]}]}"#,
        )
        .unwrap();
        assert_eq!(jobs.jobs[0].steps.len(), 1);
        let runs: Vec<RunRow> = serde_json::from_str(
            r#"[{"databaseId":5,"workflowName":"CI","status":"in_progress","conclusion":"","headSha":"abc"}]"#,
        )
        .unwrap();
        assert_eq!(runs[0].database_id, 5);
        let issues: Vec<NightlyIssue> = serde_json::from_str(
            r#"[{"number":9,"title":"Nightly failing: Nightly","url":"https://i"}]"#,
        )
        .unwrap();
        assert_eq!(issues[0].number, 9);
    }

    #[test]
    fn render_is_empty_when_all_clear_and_explains_timeouts() {
        let clear = CiStatus {
            branch: "b".into(),
            ..Default::default()
        };
        assert_eq!(render(&clear), "");

        let s = CiStatus {
            branch: "feat/x".into(),
            problems: vec![JobProblem {
                workflow: "CI".into(),
                job: "gate".into(),
                job_id: 42,
                kind: ProblemKind::TimedOut,
                step: Some("Tests (affected)".into()),
                slowest: vec![
                    ("Tests (affected)".into(), 773),
                    ("Build vox CLI".into(), 720),
                ],
                url: "https://u".into(),
            }],
            nightly: vec![NightlyIssue {
                number: 9,
                title: "Nightly failing: Nightly".into(),
                url: "https://i".into(),
            }],
            ..Default::default()
        };
        let r = render(&s);
        assert!(
            r.contains("TIMED OUT on feat/x: CI / gate at step `Tests (affected)`"),
            "{r}"
        );
        assert!(
            r.contains("Tests (affected) 12m53s, Build vox CLI 12m0s"),
            "{r}"
        );
        assert!(r.contains("move the job to nightly"), "{r}");
        assert!(
            r.contains("NIGHTLY FAILING: Nightly (#9) -> https://i"),
            "{r}"
        );
        assert!(r.lines().next().unwrap().starts_with("GitHub CI"), "{r}");
    }

    #[test]
    fn timeout_without_step_timings_says_unknown() {
        let s = CiStatus {
            branch: "b".into(),
            problems: vec![JobProblem {
                workflow: "CI".into(),
                job: "gate".into(),
                job_id: 1,
                kind: ProblemKind::TimedOut,
                step: None,
                slowest: vec![],
                url: String::new(),
            }],
            ..Default::default()
        };
        assert!(
            render(&s).contains("slowest steps: unknown"),
            "{}",
            render(&s)
        );
    }

    #[test]
    fn failed_job_render_points_at_the_log_command() {
        let s = CiStatus {
            branch: "b".into(),
            problems: vec![JobProblem {
                workflow: "CI".into(),
                job: "gate".into(),
                job_id: 7,
                kind: ProblemKind::Failed,
                step: Some("Clippy (affected)".into()),
                slowest: vec![],
                url: "https://u".into(),
            }],
            ..Default::default()
        };
        assert!(
            render(&s).contains("gh run view --job 7 --log-failed"),
            "{}",
            render(&s)
        );
    }

    #[test]
    fn change_message_only_on_change_and_announces_recovery() {
        assert_eq!(change_message(None, ""), None);
        assert_eq!(change_message(Some("X"), "X"), None);
        assert_eq!(change_message(None, "X").as_deref(), Some("X"));
        assert_eq!(change_message(Some("X"), "Y").as_deref(), Some("Y"));
        assert!(change_message(Some("X"), "").unwrap().contains("resolved"));
    }

    #[test]
    fn session_id_is_read_from_hook_json_and_sanitized() {
        assert_eq!(
            session_id(r#"{"session_id":"abc-123","prompt":"hi"}"#),
            "abc-123"
        );
        assert_eq!(session_id(r#"{"session_id":"../../etc"}"#), "etc");
        assert_eq!(session_id("not json"), "default");
        assert_eq!(sanitize("feat/x y"), "feat_x_y");
    }
}
