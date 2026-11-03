//! `workflow_policy_guard` — the hosted-CI contract for workflow YAML
//! (docs/superpowers/specs/2026-09-21-hosted-primary-ci-design.md):
//!
//! 1. Every job declares a literal integer `timeout-minutes`.
//! 2. Jobs in a *fast* workflow — triggered by `pull_request`,
//!    `pull_request_target`, `merge_group`, or a branch `push` — cap at
//!    [`FAST_CAP_MINS`]; everything else (schedule, dispatch, tag-only push,
//!    workflow_run) caps at [`SLOW_CAP_MINS`]. A job that needs more belongs in
//!    nightly, or needs caching/sharding — not a higher cap.
//! 3. Every `schedule`-triggered workflow's `name:` is listed in
//!    [`REPORT_WORKFLOW`]'s `on.workflow_run.workflows`, so a failing nightly
//!    becomes an open `nightly-failure` issue agents are shown.
//! 4. No job declares a literal `runs-on: self-hosted...` outside
//!    [`SELF_HOSTED_ALLOWLIST`] — the hosted-primary-CI migration's whole
//!    point is zero self-hosted runners with runners actually registered;
//!    the one allowlisted workflow is disabled at the repo level and stays
//!    that way until a GPU runner exists.
//!
//! Always fails on a violation. Runs inside `ssot-drift` (pre-push fast tier + ci.yml gate).

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde_yaml::Value;

pub const FAST_CAP_MINS: u64 = 30;
pub const SLOW_CAP_MINS: u64 = 180;
pub const REPORT_WORKFLOW: &str = "nightly-report.yml";
/// Disabled at the repo level (`gh workflow disable`) — need a GPU runner
/// that doesn't exist yet. `runs-on` here is dead until one is registered.
pub const SELF_HOSTED_ALLOWLIST: &[&str] = &["ml_data_extraction.yml"];

/// `serde_yaml_ng` parses `on:` as a string key; the `Bool(true)` fallback
/// covers YAML-1.1 parsers (mirrors workflow_concurrency_guard).
fn triggers(doc: &Value) -> Option<&Value> {
    let m = doc.as_mapping()?;
    m.get(Value::String("on".into()))
        .or_else(|| m.get(Value::Bool(true)))
}

pub(crate) fn trigger_keys(doc: &Value) -> Vec<(&str, Option<&Value>)> {
    match triggers(doc) {
        Some(Value::String(s)) => vec![(s.as_str(), None)],
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(Value::as_str)
            .map(|s| (s, None))
            .collect(),
        Some(Value::Mapping(m)) => m
            .iter()
            .filter_map(|(k, v)| k.as_str().map(|k| (k, Some(v))))
            .collect(),
        _ => Vec::new(),
    }
}

/// A push filtered to tags only is a release, not the dev loop.
fn is_tag_only_push(filters: Option<&Value>) -> bool {
    let Some(m) = filters.and_then(Value::as_mapping) else {
        return false;
    };
    let has = |k: &str| m.contains_key(Value::String(k.into()));
    (has("tags") || has("tags-ignore")) && !has("branches") && !has("branches-ignore")
}

fn is_fast(doc: &Value) -> bool {
    trigger_keys(doc).into_iter().any(|(k, v)| match k {
        "pull_request" | "pull_request_target" | "merge_group" => true,
        "push" => !is_tag_only_push(v),
        _ => false,
    })
}

fn has_schedule(doc: &Value) -> bool {
    trigger_keys(doc).iter().any(|(k, _)| *k == "schedule")
}

fn workflow_name(file: &str, doc: &Value) -> String {
    doc.get("name")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| file.to_string())
}

fn report_listed(report: &Value) -> Vec<String> {
    trigger_keys(report)
        .into_iter()
        .find(|(k, _)| *k == "workflow_run")
        .and_then(|(_, v)| v?.get("workflows")?.as_sequence())
        .map(|s| {
            s.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// `runs-on` as a literal string or sequence naming `self-hosted` — an
/// expression (`${{ fromJson(matrix.runs_on) }}`) isn't statically knowable
/// here and is intentionally out of scope: the one existing case is already
/// gated by a job-level `if:` (nightly-artifacts.yml's GPU matrix leg).
fn is_self_hosted(job: &Value) -> bool {
    match job.get("runs-on") {
        Some(Value::String(s)) => s.contains("self-hosted"),
        Some(Value::Sequence(seq)) => seq
            .iter()
            .filter_map(Value::as_str)
            .any(|s| s == "self-hosted"),
        _ => false,
    }
}

/// Timeout and runner-policy violations for one parsed workflow, each
/// prefixed with `file`.
pub fn check_doc(file: &str, doc: &Value) -> Vec<String> {
    let cap = if is_fast(doc) {
        FAST_CAP_MINS
    } else {
        SLOW_CAP_MINS
    };
    let jobs = doc.get("jobs").and_then(Value::as_mapping);
    let mut out = Vec::new();
    for (name, job) in jobs.into_iter().flatten() {
        let name = name.as_str().unwrap_or("?");
        // Reusable-workflow calls inherit the callee's job timeouts.
        if job.get("uses").is_some() {
            continue;
        }
        match job.get("timeout-minutes") {
            None => out.push(format!(
                "{file}: job `{name}` has no timeout-minutes (cap {cap})"
            )),
            Some(t) => match t.as_u64() {
                Some(m) if m <= cap => {}
                Some(m) => out.push(format!(
                    "{file}: job `{name}` timeout-minutes {m} exceeds cap {cap}"
                )),
                None => out.push(format!(
                    "{file}: job `{name}` timeout-minutes must be a literal integer (got {t:?})"
                )),
            },
        }
        if is_self_hosted(job) && !SELF_HOSTED_ALLOWLIST.contains(&file) {
            out.push(format!(
                "{file}: job `{name}` declares runs-on: self-hosted — not in SELF_HOSTED_ALLOWLIST \
                 (the hosted-primary-CI migration means zero self-hosted runners outside the \
                 disabled GPU workflows; move this job to a hosted runner)"
            ));
        }
    }
    out
}

/// Scheduled workflow names absent from the report workflow's list.
pub fn missing_from_report(scheduled: &[String], listed: &[String]) -> Vec<String> {
    scheduled
        .iter()
        .filter(|n| !listed.contains(n))
        .map(|n| format!("scheduled workflow `{n}` is not listed in {REPORT_WORKFLOW} on.workflow_run.workflows"))
        .collect()
}

pub fn run(repo_root: &Path) -> Result<()> {
    let dir = repo_root.join(".github").join("workflows");
    if !dir.is_dir() {
        return Ok(());
    }
    let mut paths: Vec<_> = std::fs::read_dir(&dir)
        .with_context(|| format!("read {}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    paths.sort();
    let mut violations = Vec::new();
    let mut scheduled = Vec::new();
    let mut listed = Vec::new();
    for path in paths {
        let file = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let text =
            std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let doc: Value =
            serde_yaml::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
        violations.extend(check_doc(&file, &doc));
        if has_schedule(&doc) {
            scheduled.push(workflow_name(&file, &doc));
        }
        if file == REPORT_WORKFLOW {
            listed = report_listed(&doc);
        }
    }
    violations.extend(missing_from_report(&scheduled, &listed));
    if violations.is_empty() {
        println!("workflow-policy-guard OK");
        return Ok(());
    }
    Err(anyhow!(
        "workflow-policy-guard: {} violation(s):\n  {}\n\
         Fast workflows (PR / merge_group / branch push) cap jobs at {FAST_CAP_MINS} min; \
         others at {SLOW_CAP_MINS}. Over budget? Cache, shard, or move the job to nightly.yml. \
         Every scheduled workflow must be listed in {REPORT_WORKFLOW}.",
        violations.len(),
        violations.join("\n  ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(y: &str) -> serde_yaml::Value {
        serde_yaml::from_str(y).unwrap()
    }

    #[test]
    fn pr_workflow_job_over_30_is_flagged() {
        let v = check_doc(
            "ci.yml",
            &doc(
                "on: { pull_request: {} }\njobs:\n  a:\n    runs-on: ubuntu-latest\n    timeout-minutes: 31\n    steps: []",
            ),
        );
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(v[0].contains("exceeds cap 30"), "{v:?}");
    }

    #[test]
    fn sequence_trigger_caps_at_30() {
        let at = doc("on: [pull_request]\njobs:\n  a:\n    timeout-minutes: 30\n    steps: []");
        assert!(check_doc("ci.yml", &at).is_empty());
        let over = doc("on: [pull_request]\njobs:\n  a:\n    timeout-minutes: 31\n    steps: []");
        assert_eq!(check_doc("ci.yml", &over).len(), 1);
    }

    #[test]
    fn missing_expression_or_quoted_timeout_is_flagged() {
        let v = check_doc(
            "x.yml",
            &doc(
                "on: workflow_dispatch\njobs:\n  a:\n    steps: []\n  b:\n    timeout-minutes: ${{ matrix.t }}\n    steps: []\n  c:\n    timeout-minutes: \"30\"\n    steps: []",
            ),
        );
        assert_eq!(v.len(), 3, "{v:?}");
        assert!(v.iter().any(|m| m.contains("`a` has no timeout-minutes")));
        assert!(
            v.iter()
                .any(|m| m.contains("`b` timeout-minutes must be a literal integer"))
        );
        assert!(
            v.iter()
                .any(|m| m.contains("`c` timeout-minutes must be a literal integer"))
        );
    }

    #[test]
    fn trigger_classes() {
        let job = |on: &str, mins: u32| {
            doc(&format!(
                "on: {on}\njobs:\n  a:\n    timeout-minutes: {mins}\n    steps: []"
            ))
        };
        // fast (cap 30)
        assert_eq!(check_doc("w", &job("push", 31)).len(), 1);
        assert_eq!(
            check_doc("w", &job("{ pull_request_target: {} }", 31)).len(),
            1
        );
        assert_eq!(check_doc("w", &job("{ merge_group: {} }", 31)).len(), 1);
        assert_eq!(
            check_doc(
                "w",
                &job("{ push: { branches: [main], tags: ['v*'] } }", 31)
            )
            .len(),
            1
        );
        // slow (cap 180)
        assert!(check_doc("w", &job("{ push: { tags: ['v*'] } }", 180)).is_empty());
        assert!(check_doc("w", &job("{ workflow_run: { workflows: [CI] } }", 180)).is_empty());
        assert!(check_doc("w", &job("{ schedule: [ { cron: '0 6 * * *' } ] }", 180)).is_empty());
        assert_eq!(
            check_doc("w", &job("{ schedule: [ { cron: '0 6 * * *' } ] }", 181)).len(),
            1
        );
    }

    #[test]
    fn self_hosted_runs_on_is_flagged_outside_the_allowlist() {
        let literal = doc(
            "on: [pull_request]\njobs:\n  a:\n    runs-on: self-hosted\n    timeout-minutes: 10\n    steps: []",
        );
        let v = check_doc("new-workflow.yml", &literal);
        assert!(
            v.iter()
                .any(|m| m.contains("declares runs-on: self-hosted")),
            "{v:?}"
        );

        let sequence = doc(
            "on: [pull_request]\njobs:\n  a:\n    runs-on: [self-hosted, linux]\n    timeout-minutes: 10\n    steps: []",
        );
        let v = check_doc("new-workflow.yml", &sequence);
        assert!(
            v.iter()
                .any(|m| m.contains("declares runs-on: self-hosted")),
            "{v:?}"
        );

        // Allowlisted (disabled, GPU-only) workflows are exempt.
        let v = check_doc("ml_data_extraction.yml", &sequence);
        assert!(!v.iter().any(|m| m.contains("self-hosted")), "{v:?}");

        // A hosted runner never trips it.
        let hosted = doc(
            "on: [pull_request]\njobs:\n  a:\n    runs-on: ubuntu-latest\n    timeout-minutes: 10\n    steps: []",
        );
        assert!(check_doc("new-workflow.yml", &hosted).is_empty());

        // An unresolvable expression (matrix-driven) is out of scope, not flagged.
        let expr = doc(
            "on: [schedule]\njobs:\n  a:\n    runs-on: ${{ fromJson(matrix.runs_on) }}\n    timeout-minutes: 10\n    steps: []",
        );
        assert!(check_doc("new-workflow.yml", &expr).is_empty());
    }

    #[test]
    fn reusable_workflow_call_jobs_are_skipped() {
        let v = check_doc(
            "x.yml",
            &doc("on: [pull_request]\njobs:\n  a:\n    uses: ./.github/workflows/y.yml"),
        );
        assert!(v.is_empty(), "{v:?}");
    }

    #[test]
    fn scheduled_names_missing_from_report_are_named() {
        let scheduled = vec!["Nightly".to_string(), "CodeQL".to_string()];
        let listed = vec!["Nightly".to_string()];
        let v = missing_from_report(&scheduled, &listed);
        assert_eq!(v.len(), 1, "{v:?}");
        assert!(
            v[0].contains("CodeQL") && v[0].contains("nightly-report.yml"),
            "{v:?}"
        );
        assert!(missing_from_report(&scheduled, &scheduled).is_empty());
    }

    #[test]
    fn repo_workflows_satisfy_policy() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        run(&root).unwrap();
    }
}
