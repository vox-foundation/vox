//! Offline gates over the tasks of the frozen diagram-authoring dataset
//! (`contracts/eval/diagram-authoring/`): every task follows the authoring rules (labels quoted
//! verbatim from the real Mermaid parser's canonical sets of the reference, direction stated in
//! words, no syntax named outside quoted labels), the task counts equal the manifest's, and the
//! hard-case quotas, measured from the oracle's canonical sets, hold. Cards, template and manifest
//! hashes are gated in `eval_diagram_frozen`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use vox_cli::commands::model::eval_diagram::{OracleOutput, Task, read_jsonl};
use vox_doc_pipeline::diagram::check;
use vox_doc_pipeline::diagram::ir::Ir;

/// Direction of the canonical sets and the only phrase a prompt may use for it.
const DIRECTIONS: [(&str, &str); 4] = [
    ("down", "top to bottom"),
    ("right", "left to right"),
    ("up", "bottom to top"),
    ("left", "right to left"),
];

/// Words naming the syntax of either language (also in the plural), case-insensitive.
const BANNED_WORDS: [&str; 8] = [
    "subgraph",
    "container",
    "flowchart",
    "mermaid",
    "vox",
    "node",
    "actor",
    "participant",
];

/// Arrow and bracket tokens of either language.
const BANNED_TOKENS: [&str; 11] = ["->", "<-", "=>", "--", "|", "{", "}", "[", "]", "%%", "//"];

pub(crate) fn dataset() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/eval/diagram-authoring")
}

pub(crate) fn read(name: &str) -> String {
    let path = dataset().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn str_of(v: &Value) -> &str {
    v.as_str().unwrap_or_default()
}

/// Every non-empty text of the canonical sets: node, edge and container labels of a graph; actor,
/// message and note texts of a sequence. Longest first, so removing quoted spans is unambiguous.
fn set_texts(sets: &Value) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let list = |key: &str| sets[key].as_array().cloned().unwrap_or_default();
    if str_of(&sets["kind"]) == "graph" {
        out.extend(list("nodes").iter().map(|n| str_of(&n[0]).to_string()));
        out.extend(list("edges").iter().map(|e| str_of(&e[2]).to_string()));
        for path in list("containers") {
            out.extend(
                path.as_array()
                    .into_iter()
                    .flatten()
                    .map(|t| str_of(t).to_string()),
            );
        }
    } else {
        out.extend(list("actors").iter().map(|a| str_of(a).to_string()));
        for event in list("events") {
            let items = event.as_array().cloned().unwrap_or_default();
            out.extend(items.last().map(|t| str_of(t).to_string()));
        }
    }
    out.retain(|t| !t.is_empty());
    out.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    out.dedup();
    out
}

/// The authoring-rule violations of one prompt against its expected sets (empty when it passes).
fn prompt_errors(prompt: &str, sets: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let mut outside = prompt.to_string();
    for text in set_texts(sets) {
        if text.contains('\n') || text.contains('\r') {
            errors.push(format!("label {text:?} has a line break"));
        }
        let quoted = format!("\"{text}\"");
        if !prompt.contains(&quoted) {
            errors.push(format!("{text:?} is not double-quoted verbatim"));
        }
        outside = outside.replace(&quoted, " ");
    }
    if outside.contains('"') {
        errors.push(format!(
            "a quoted span is not a text of the sets: {outside:?}"
        ));
    }
    if str_of(&sets["kind"]) == "graph" {
        let want = DIRECTIONS
            .iter()
            .find(|(d, _)| *d == str_of(&sets["direction"]))
            .map_or("?", |(_, phrase)| *phrase);
        if !outside.contains(want) {
            errors.push(format!("direction {want:?} is not stated"));
        }
        for (_, phrase) in DIRECTIONS.iter().filter(|(_, p)| *p != want) {
            if outside.contains(phrase) {
                errors.push(format!("states the wrong direction {phrase:?}"));
            }
        }
    }
    let lower = outside.to_lowercase();
    for word in lower.split(|c: char| !c.is_ascii_alphabetic()) {
        let singular = word.strip_suffix('s').unwrap_or(word);
        if BANNED_WORDS.contains(&word) || BANNED_WORDS.contains(&singular) {
            errors.push(format!("names syntax: {word:?} outside quotes"));
        }
    }
    for token in BANNED_TOKENS {
        if outside.contains(token) {
            errors.push(format!("names syntax: {token:?} outside quotes"));
        }
    }
    errors
}

/// Expected id of the `i`-th task (0-based) given the number of graph tasks.
fn expected_id(i: usize, graphs: usize) -> String {
    if i < graphs {
        format!("g{:03}", i + 1)
    } else {
        format!("s{:03}", i - graphs + 1)
    }
}

#[test]
fn per_task_gates() {
    let raw = read("tasks.v1.jsonl");
    let tasks: Vec<Task> = read_jsonl(&raw).expect("tasks.v1.jsonl");
    let outputs: Vec<OracleOutput> =
        read_jsonl(&read("reference-outputs.jsonl")).expect("reference-outputs.jsonl");
    assert!(!tasks.is_empty(), "no tasks");
    assert_eq!(outputs.len(), tasks.len(), "one reference output per task");
    let graphs = tasks.iter().filter(|t| t.kind == "graph").count();
    let mut failures = Vec::new();
    let mut prompts = std::collections::BTreeSet::new();
    for (i, (line, task)) in raw.lines().zip(&tasks).enumerate() {
        let mut fail = |why: String| failures.push(format!("{}: {why}", task.id));
        if serde_json::to_string(task).ok().as_deref() != Some(line) {
            fail("line is not the compact serialisation of its task (key order id, kind, prompt, reference)".into());
        }
        if task.id != expected_id(i, graphs) {
            fail(format!(
                "expected id {} at line {}",
                expected_id(i, graphs),
                i + 1
            ));
        }
        if !matches!(task.kind.as_str(), "graph" | "sequence") {
            fail(format!("unknown kind {:?}", task.kind));
        }
        if !prompts.insert(task.prompt.clone()) {
            fail("duplicate prompt".into());
        }
        let report = check(&task.reference);
        if !report.valid || !report.diagnostics.is_empty() {
            fail(format!(
                "reference does not check clean: {:?}",
                report.diagnostics
            ));
        }
        let ir_kind = match report.ir {
            Some(Ir::Graph(_)) => "graph",
            Some(Ir::Sequence(_)) => "sequence",
            None => "none",
        };
        if ir_kind != task.kind {
            fail(format!(
                "kind {:?} but the reference is {ir_kind}",
                task.kind
            ));
        }
        let out = &outputs[i];
        let supported = out.sets.is_object() && out.sets.get("unsupported").is_none();
        if out.id != task.id || !out.ok || !supported {
            fail(format!(
                "reference output {} is not ok with supported sets",
                out.id
            ));
            continue;
        }
        if str_of(&out.sets["kind"]) != task.kind {
            fail("reference output kind differs".into());
        }
        for why in prompt_errors(&task.prompt, &out.sets) {
            fail(why);
        }
    }
    assert!(
        failures.is_empty(),
        "task gate failures:\n{}",
        failures.join("\n")
    );
}

#[test]
fn per_task_gate_rejects_bad_prompts() {
    let sets: Value = serde_json::from_str(
        r#"{"kind":"graph","direction":"right","nodes":[["Web App","rect",["Edge Zone"]],["DB","cylinder",[]]],"edges":[[["Web App","rect",["Edge Zone"]],["DB","cylinder",[]],"query"]],"containers":[["Edge Zone"]]}"#,
    )
    .unwrap();
    let good = "Lay it out left to right. A group titled \"Edge Zone\" holds \"Web App\", which sends to \"DB\", drawn as a cylinder, with an arrow labelled \"query\".";
    assert_eq!(prompt_errors(good, &sets), Vec::<String>::new());
    let cases = [
        (good.replace("\"Web App\"", "Web App"), "not double-quoted"),
        (good.replace("left to right", "sideways"), "is not stated"),
        (format!("{good} Also top to bottom."), "wrong direction"),
        (good.replace("A group", "A subgraph"), "\"subgraph\""),
        (good.replace("A group", "Two Containers"), "\"containers\""),
        (good.replace("which sends", "which -> sends"), "\"->\""),
        (good.replace("labelled", "labelled |x|"), "\"|\""),
        (format!("{good} Add \"Cache\"."), "not a text of the sets"),
    ];
    for (prompt, expected) in cases {
        let errors = prompt_errors(&prompt, &sets);
        assert!(
            errors.iter().any(|e| e.contains(expected)),
            "{prompt:?} should fail with {expected:?}: {errors:?}"
        );
    }
    let seq: Value = serde_json::from_str(
        r#"{"kind":"sequence","actors":["Web","API"],"events":[["message",0,1,false,"GET /a"],["note",[0,1],"both"]]}"#,
    )
    .unwrap();
    let ok = "Two parties, \"Web\" then \"API\". \"Web\" sends \"GET /a\" to \"API\"; a note over both reads \"both\".";
    assert_eq!(prompt_errors(ok, &seq), Vec::<String>::new());
    let bad = ok.replace("Two parties", "Two actors");
    assert!(
        prompt_errors(&bad, &seq)
            .iter()
            .any(|e| e.contains("\"actors\""))
    );
    let broken: Value =
        serde_json::from_str(r#"{"kind":"sequence","actors":["a\nb"],"events":[]}"#).unwrap();
    assert!(
        prompt_errors("\"a\nb\"", &broken)
            .iter()
            .any(|e| e.contains("line break"))
    );
}

/// Hard-case quotas, in `features` order: the first seven are graph cases, the last three
/// sequence cases (CONTEXT: Task set). `QUOTA_MINIMA` is the number of tasks each needs.
const QUOTAS: [&str; 10] = [
    "punctuation",
    "nesting",
    "edge_labels",
    "self_loops",
    "fan",
    "shapes",
    "hostile",
    "notes",
    "replies",
    "self_messages",
];
const QUOTA_MINIMA: [usize; 10] = [60, 30, 80, 20, 30, 30, 15, 10, 10, 4];

/// The hard cases one canonical-set document contains. Punctuation: any label with one of
/// `()"':/#&<>;[]{}|` or a non-ASCII character; nesting: a group path of depth 2 or more; fan: a
/// node with three or more outgoing or incoming edges; shapes: not `rect`; hostile: a node label
/// `end`, or one alphanumeric word starting with o or x (case-insensitive).
fn features(sets: &Value) -> [bool; 10] {
    let list = |key: &str| sets[key].as_array().cloned().unwrap_or_default();
    let (nodes, edges, events) = (list("nodes"), list("edges"), list("events"));
    let punct = |t: &String| {
        t.chars()
            .any(|c| !c.is_ascii() || "()\"':/#&<>;[]{}|".contains(c))
    };
    let hostile = |l: String| {
        l == "end" || l.starts_with(['o', 'x']) && l.chars().all(|c| c.is_ascii_alphanumeric())
    };
    let fan = |n: &Value| (0..2).any(|k| edges.iter().filter(|e| e[k] == *n).count() >= 3);
    let message = |e: &Value| str_of(&e[0]) == "message";
    [
        str_of(&sets["kind"]) == "graph" && set_texts(sets).iter().any(punct),
        list("containers")
            .iter()
            .any(|p| p.as_array().is_some_and(|p| p.len() >= 2)),
        edges.iter().any(|e| !str_of(&e[2]).is_empty()),
        edges.iter().any(|e| e[0] == e[1]),
        nodes.iter().any(fan),
        nodes.iter().any(|n| str_of(&n[1]) != "rect"),
        nodes.iter().any(|n| hostile(str_of(&n[0]).to_lowercase())),
        events.iter().any(|e| str_of(&e[0]) == "note"),
        events.iter().any(|e| message(e) && e[3] == true),
        events.iter().any(|e| message(e) && e[1] == e[2]),
    ]
}

/// Per-quota task counts over canonical-set documents (the oracle's, never self-declared tags),
/// and how many documents were counted: one of neither kind contributes nothing, so a caller can
/// prove no task was skipped.
fn quota_counts(sets: &[Value]) -> (usize, [usize; 10]) {
    let mut totals = [0; 10];
    let known = sets
        .iter()
        .filter(|s| matches!(str_of(&s["kind"]), "graph" | "sequence"));
    let counted = known.inspect(|s| {
        for (total, has) in totals.iter_mut().zip(features(s)) {
            *total += usize::from(has);
        }
    });
    (counted.count(), totals)
}

#[test]
fn quota_counts_measure_the_canonical_sets() {
    let docs = [
        r#"{"kind":"graph","direction":"down","nodes":[["Start","start",[]],["B","rect",["Outer","Inner"]],["C","rect",[]],["x-ray","rect",[]]],"edges":[[["Start","start",[]],["B","rect",["Outer","Inner"]],"go"],[["Start","start",[]],["C","rect",[]],""],[["Start","start",[]],["x-ray","rect",[]],""],[["B","rect",["Outer","Inner"]],["B","rect",["Outer","Inner"]],""]],"containers":[["Outer"],["Outer","Inner"]]}"#,
        r#"{"kind":"graph","direction":"left","nodes":[["Parse: main.rs","rect",[]],["Order","rounded",["Box"]],["Endpoint","rect",[]]],"edges":[[["Parse: main.rs","rect",[]],["Order","rounded",["Box"]],""],[["Order","rounded",["Box"]],["Endpoint","rect",[]],""]],"containers":[["Box"]]}"#,
        r#"{"kind":"sequence","actors":["A","B"],"events":[["message",0,1,false,"hi"],["message",1,0,true,"ok"],["note",[0],"n"]]}"#,
        r#"{"kind":"sequence","actors":["A"],"events":[["message",0,0,false,"tick"],["note",[0],"t"]]}"#,
    ];
    let sets: Vec<Value> = docs
        .iter()
        .map(|d| serde_json::from_str(d).unwrap())
        .collect();
    assert_eq!(quota_counts(&sets), (4, [1, 1, 1, 1, 1, 2, 1, 2, 1, 1]));
    let other = [serde_json::json!({"kind": "unsupported"})];
    assert_eq!(quota_counts(&other), (0, [0; 10]));
}

#[test]
fn quotas() {
    let tasks: Vec<Task> = read_jsonl(&read("tasks.v1.jsonl")).expect("tasks");
    let outputs: Vec<OracleOutput> =
        read_jsonl(&read("reference-outputs.jsonl")).expect("reference-outputs.jsonl");
    let sets: Vec<Value> = outputs.into_iter().map(|o| o.sets).collect();
    let (counted, totals) = quota_counts(&sets);
    assert_eq!(counted, tasks.len(), "every task contributes a count row");
    let mut short = Vec::new();
    for ((name, min), n) in QUOTAS.iter().zip(QUOTA_MINIMA).zip(totals) {
        println!("quota {name}: {n} (minimum {min})");
        if n < min {
            short.push(format!("{name}: {n} < {min}"));
        }
    }
    assert!(short.is_empty(), "quotas short:\n{}", short.join("\n"));
}

#[test]
fn counts_match_the_manifest() {
    let manifest: serde_yaml::Value =
        serde_yaml::from_str(&read("manifest.v1.yaml")).expect("manifest.v1.yaml");
    let tasks: Vec<Task> = read_jsonl(&read("tasks.v1.jsonl")).expect("tasks");
    let kind = |k: &str| tasks.iter().filter(|t| t.kind == k).count() as u64;
    let counts = (tasks.len() as u64, kind("graph"), kind("sequence"));
    assert_eq!(counts, (200, 184, 16), "tasks (total, graph, sequence)");
    let recorded = |v: &serde_yaml::Value| v.as_u64().unwrap_or_default();
    let m = &manifest;
    let pinned = (
        recorded(&m["corpus_size"]),
        recorded(&m["kinds"]["graph"]),
        recorded(&m["kinds"]["sequence"]),
    );
    assert_eq!(
        pinned, counts,
        "manifest corpus_size and kinds vs tasks.v1.jsonl"
    );
}
