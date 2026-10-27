//! Rust half of the diagram-authoring offline re-score: from the committed recording and the
//! committed Node oracle outputs, re-derive `reference-inputs.jsonl`, every run's
//! `oracle-inputs.jsonl`, `tallies.json` and `tables.md`, byte-for-byte. It never spawns `node`: the
//! Node half (`docs-astro/tests/unit/diagram-authoring-rescore.test.mjs`) re-derives the
//! `*-outputs.jsonl` files. `EVAL_DIAGRAM_BLESS=1` writes the Rust-derived files instead of
//! comparing (never `attempts.jsonl`, never a Node output).

use std::path::{Path, PathBuf};

use vox_cli::commands::model::eval_diagram::render::render_tables;
use vox_cli::commands::model::eval_diagram::score::oracle_inputs;
use vox_cli::commands::model::eval_diagram::tally::tally;
use vox_cli::commands::model::eval_diagram::{
    Line, OracleOutput, Task, parse_manifest, read_jsonl, reference_inputs,
};

fn bless() -> bool {
    std::env::var("EVAL_DIAGRAM_BLESS").as_deref() == Ok("1")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Compare a derived file with its committed bytes, or write it in bless mode.
fn check(path: &Path, derived: &str) {
    if bless() {
        std::fs::write(path, derived).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
        return;
    }
    assert_eq!(
        derived,
        read(path),
        "{} differs from its re-derivation (re-bless with EVAL_DIAGRAM_BLESS=1 and read the diff)",
        path.display()
    );
}

/// Present, or missing only while blessing (the Node side has not produced it yet).
fn node_output(path: &Path) -> Option<String> {
    if path.exists() {
        return Some(read(path));
    }
    assert!(bless(), "missing Node-derived {}", path.display());
    None
}

/// Re-derive (or bless) every derived file of `dataset`; returns the number of runs re-scored. A
/// dataset without `runs/` (the task set before its live run) checks the reference chain only. The
/// manifest's pre-registered decision rule must parse either way.
fn rescore(dataset: &Path) -> usize {
    let manifest =
        parse_manifest(&read(&dataset.join("manifest.v1.yaml"))).expect("manifest decision rule");
    let tasks: Vec<Task> = read_jsonl(&read(&dataset.join("tasks.v1.jsonl"))).expect("tasks");
    assert_eq!(
        manifest.corpus_size,
        tasks.len(),
        "corpus_size vs tasks.v1.jsonl"
    );
    check(
        &dataset.join("reference-inputs.jsonl"),
        &reference_inputs(&tasks).expect("reference inputs"),
    );
    let Some(refs) = node_output(&dataset.join("reference-outputs.jsonl")) else {
        return 0;
    };
    let refs: Vec<OracleOutput> = read_jsonl(&refs).expect("reference outputs");

    let mut runs: Vec<PathBuf> = match std::fs::read_dir(dataset.join("runs")) {
        Ok(dir) => dir.map(|e| e.expect("dir entry").path()).collect(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => panic!("{}: runs/: {e}", dataset.display()),
    };
    runs.sort();
    let mut scored = 0;
    for run in runs {
        let lines: Vec<Line> = read_jsonl(&read(&run.join("attempts.jsonl"))).expect("attempts");
        check(
            &run.join("oracle-inputs.jsonl"),
            &oracle_inputs(&tasks, &refs, &lines).expect("oracle inputs"),
        );
        let Some(outs) = node_output(&run.join("oracle-outputs.jsonl")) else {
            continue;
        };
        let outs: Vec<OracleOutput> = read_jsonl(&outs).expect("oracle outputs");
        let t = tally(&manifest.decision_rule, &tasks, &lines, &outs)
            .unwrap_or_else(|e| panic!("{}: {e:#}", run.display()));
        check(
            &run.join("tallies.json"),
            &(serde_json::to_string_pretty(&t).expect("json") + "\n"),
        );
        check(&run.join("tables.md"), &render_tables(&t));
        scored += 1;
    }
    scored
}

#[test]
fn synthetic_dataset_rescores() {
    let scored = rescore(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/eval_diagram_synthetic"),
    );
    assert!(scored > 0 || bless(), "the synthetic dataset scored no run");
}

#[test]
fn diagram_authoring_dataset_rescores() {
    rescore(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/eval/diagram-authoring"));
}

/// The findings report quotes the latest live run: its results block is that run's `tables.md`, it
/// names every model id the run used, states the tallied primary verdict once, and quotes no task.
#[test]
fn diagram_authoring_report_matches_run() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dataset = root.join("contracts/eval/diagram-authoring");
    let report =
        read(&root.join("docs/src/architecture/voxdiagram-authoring-reliability-findings-2026.md"));
    let mut runs: Vec<PathBuf> = std::fs::read_dir(dataset.join("runs"))
        .expect("runs/")
        .map(|e| e.expect("dir entry").path())
        .collect();
    runs.sort();
    let mut ids = Vec::new();
    for run in &runs {
        let lines: Vec<Line> = read_jsonl(&read(&run.join("attempts.jsonl"))).expect("attempts");
        ids.clear();
        for line in lines {
            match line {
                Line::Run(h) => {
                    assert!(!h.synthetic, "{}: not a live run", run.display());
                    ids.extend(h.models.into_iter().map(|m| m.requested_id));
                }
                Line::Attempt(a) => ids.extend(a.resolved_model),
                Line::Stop(_) => {}
            }
        }
    }
    let run = runs.last().expect("at least one live run");

    let (begin, end) = (
        "<!-- eval-diagram:tables:begin -->",
        "<!-- eval-diagram:tables:end -->",
    );
    let start = report.find(begin).expect("tables begin marker") + begin.len();
    let stop = report.find(end).expect("tables end marker");
    assert_eq!(
        report[start..stop].trim(),
        read(&run.join("tables.md")).trim(),
        "the report's results block differs from {}/tables.md",
        run.display()
    );

    ids.sort();
    ids.dedup();
    for id in &ids {
        assert!(
            report.contains(id.as_str()),
            "the report does not name model id {id}"
        );
    }

    let tallies: serde_json::Value =
        serde_json::from_str(&read(&run.join("tallies.json"))).expect("tallies.json");
    let key = tallies["primary"]["verdict"]
        .as_str()
        .expect("primary verdict");
    let verdict = format!(
        "Primary verdict: {}.",
        key.replace("non_inferior", "non-inferior")
            .replace('_', " ")
    );
    assert_eq!(
        report.lines().filter(|l| *l == verdict).count(),
        1,
        "the report must state `{verdict}` once, on its own line"
    );

    let tasks: Vec<Task> = read_jsonl(&read(&dataset.join("tasks.v1.jsonl"))).expect("tasks");
    for t in &tasks {
        assert!(
            !report.contains(t.prompt.as_str()),
            "the report quotes {}'s prompt",
            t.id
        );
        assert!(
            !report.contains(t.reference.trim()),
            "the report quotes {}'s reference",
            t.id
        );
    }
}
