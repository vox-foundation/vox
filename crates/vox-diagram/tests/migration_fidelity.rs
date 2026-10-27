//! Per-fence fidelity report for the Phase 90 corpus migration. Ignored one-shot reader of
//! `docs/src` (the second one next to `freeze_corpus`; read-only, no git). Every frozen fence of
//! `tests/fixtures/corpus/MANIFEST.json` must still be, in its doc, either
//! - `pending`/`kept`: a Mermaid fence byte-identical to the frozen block (`DRIFT` otherwise), or
//! - `migrated`: a `vox-diagram` fence that checks, whose id-free canonical sets equal the golden
//!   the Node oracle proves equal to the real Mermaid parse of the original (`MISMATCH`), and
//!   that survives Mermaid export then re-import (`ROUNDTRIP`).
//!
//! A change in a doc's fence count is a `COUNT` failure. Run by hand:
//! `FIDELITY_OUT=<abs>/fidelity.md cargo nextest run -p vox-diagram --retries 0 --run-ignored only -E 'binary(migration_fidelity)'`.

mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde_json::Value;
use support::roundtrip::{canon_graph, canon_sequence};
use vox_diagram::check;
use vox_diagram::fence::{extract, normalize};
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, export, import};

/// Stem to fixture-relative golden path for blocks without a `corpus-roundtrip/<stem>.sets.json`.
const HAND: &[(&str, &str)] = &[];

struct Entry {
    stem: String,
    source: String,
    header: String,
}

fn manifest(corpus: &Path) -> Vec<(u64, Entry)> {
    let text = fs::read_to_string(corpus.join("MANIFEST.json")).unwrap_or_default();
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(&text) else {
        panic!("corpus/MANIFEST.json unreadable");
    };
    let s = |v: &Value, k: &str| v[k].as_str().unwrap_or_default().to_owned();
    items
        .iter()
        .map(|e| {
            let stem = s(e, "fixture").trim_end_matches(".mmd").to_owned();
            let entry = Entry {
                stem,
                source: s(e, "source"),
                header: s(e, "header"),
            };
            (e["open_line"].as_u64().unwrap_or(0), entry)
        })
        .collect()
}

/// Keys of two canonical-set objects whose values differ.
fn differing(a: &Value, b: &Value) -> String {
    let (Some(x), Some(y)) = (a.as_object(), b.as_object()) else {
        return "<not objects>".to_owned();
    };
    let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
    keys.sort();
    keys.dedup();
    let bad: Vec<&str> = keys
        .into_iter()
        .filter(|k| x.get(*k) != y.get(*k))
        .map(String::as_str)
        .collect();
    bad.join(",")
}

fn canon(ir: &Ir) -> Value {
    match ir {
        Ir::Graph(g) => canon_graph(g),
        Ir::Sequence(s) => canon_sequence(s),
    }
}

/// Counts `[kind, nodes-or-actors, edges-or-events, containers]` of an IR.
fn counts(ir: &Ir) -> [String; 4] {
    match ir {
        Ir::Graph(g) => [
            "graph".into(),
            g.nodes.len().to_string(),
            g.edges.len().to_string(),
            g.containers.len().to_string(),
        ],
        Ir::Sequence(s) => [
            "sequence".into(),
            s.actors.len().to_string(),
            s.events.len().to_string(),
            "-".into(),
        ],
    }
}

/// Verify a `vox-diagram` fence body against the stem's golden; the IR on success.
fn verify_migrated(stem: &str, body: &str, fixtures: &Path) -> Result<Ir, String> {
    let report = check(body);
    let Some(ir) = report.ir.filter(|_| report.valid) else {
        let code = report.diagnostics.first().map_or("none", |d| d.code);
        return Err(format!("INVALID {stem}: {code}"));
    };
    let rel = HAND.iter().find(|(s, _)| *s == stem).map_or_else(
        || format!("corpus-roundtrip/{stem}.sets.json"),
        |(_, p)| (*p).to_owned(),
    );
    let golden = fs::read_to_string(fixtures.join(&rel))
        .map_err(|e| format!("MISMATCH {stem}: golden {rel} unreadable: {e}"))?;
    let want: Value =
        serde_json::from_str(&golden).map_err(|e| format!("MISMATCH {stem}: golden {rel}: {e}"))?;
    let have = canon(&ir);
    if have != want {
        return Err(format!(
            "MISMATCH {stem}: differs in {}",
            differing(&have, &want)
        ));
    }
    let back = import(&export(&ir), ImportOpts::default());
    if !back.unsupported.is_empty() {
        return Err(format!("ROUNDTRIP {stem}: re-import refused"));
    }
    let again = back.source.as_deref().map(check).and_then(|r| r.ir);
    match again {
        Some(a) if canon(&a) == have => Ok(ir),
        Some(a) => Err(format!(
            "ROUNDTRIP {stem}: differs in {}",
            differing(&canon(&a), &have)
        )),
        None => Err(format!(
            "ROUNDTRIP {stem}: re-imported source does not check"
        )),
    }
}

#[test]
#[ignore = "one-shot report: reads docs/src; set FIDELITY_OUT"]
fn migration_fidelity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures = root.join("crates/vox-diagram/tests/fixtures");
    let corpus = fixtures.join("corpus");
    let mut by_doc: BTreeMap<String, Vec<(u64, Entry)>> = BTreeMap::new();
    for (line, e) in manifest(&corpus) {
        by_doc.entry(e.source.clone()).or_default().push((line, e));
    }
    let (mut migrated, mut pending, mut kept) = (0, 0, 0);
    let mut failures: Vec<String> = Vec::new();
    let mut rows: Vec<(String, String)> = Vec::new();
    for (source, mut entries) in by_doc {
        entries.sort_by_key(|(line, _)| *line);
        let text = fs::read_to_string(root.join(&source)).unwrap_or_default();
        let mut fences: Vec<(u32, bool, String)> = extract(&text, "mermaid")
            .into_iter()
            .map(|f| (f.open_line, false, f.body))
            .chain(
                extract(&text, "vox-diagram")
                    .into_iter()
                    .map(|f| (f.open_line, true, f.body)),
            )
            .collect();
        fences.sort_by_key(|f| f.0);
        if fences.len() != entries.len() {
            failures.push(format!(
                "COUNT {source}: manifest {}, doc {}",
                entries.len(),
                fences.len()
            ));
        }
        for (k, (_, e)) in entries.iter().enumerate() {
            let cell = |status: &str, line: u32, c: [String; 4]| {
                let at = format!("{source}:{line}");
                let row = format!(
                    "| {} | {at} | {status} | {} | {} | {} | {} |",
                    e.stem, c[0], c[1], c[2], c[3]
                );
                (e.stem.clone(), row)
            };
            let dash = || ["-".into(), "-".into(), "-".into(), "-".into()];
            let Some((line, is_vd, body)) = fences.get(k) else {
                rows.push(cell("missing", 0, dash()));
                continue;
            };
            if *is_vd {
                match verify_migrated(&e.stem, &normalize(body), &fixtures) {
                    Ok(ir) => {
                        migrated += 1;
                        rows.push(cell("migrated", *line, counts(&ir)));
                    }
                    Err(f) => {
                        failures.push(f);
                        rows.push(cell("FAILED", *line, dash()));
                    }
                }
                continue;
            }
            let frozen = fs::read_to_string(corpus.join(format!("{}.mmd", e.stem)));
            let now = format!("{}\n", normalize(body));
            if frozen.ok().as_deref() != Some(now.as_str()) {
                failures.push(format!(
                    "DRIFT {}: {source}:{line} differs from the frozen block",
                    e.stem
                ));
            }
            let status = if e.header == "gantt" {
                "kept"
            } else {
                "pending"
            };
            if status == "kept" {
                kept += 1;
            } else {
                pending += 1;
            }
            rows.push(cell(status, *line, dash()));
        }
    }
    rows.sort();
    let mut report = String::from(
        "| stem | fence | status | kind | nodes/actors | edges/events | containers |\n|---|---|---|---|---|---|---|\n",
    );
    for (_, row) in &rows {
        let _ = writeln!(report, "{row}");
    }
    let _ = writeln!(
        report,
        "\nfidelity: migrated {migrated}, pending {pending}, kept {kept}, failures {}",
        failures.len()
    );
    for f in &failures {
        let _ = writeln!(report, "{f}");
    }
    println!("{report}");
    if let Ok(out) = std::env::var("FIDELITY_OUT") {
        if let Some(parent) = Path::new(&out).parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Err(e) = fs::write(&out, &report) {
            panic!("cannot write {out}: {e}");
        }
    }
    assert!(
        failures.is_empty(),
        "fidelity failures:\n{}",
        failures.join("\n")
    );
}
