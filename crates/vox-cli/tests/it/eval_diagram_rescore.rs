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
