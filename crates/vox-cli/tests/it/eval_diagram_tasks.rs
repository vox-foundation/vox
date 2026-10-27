//! Offline gates over the frozen diagram-authoring dataset (`contracts/eval/diagram-authoring/`):
//! every task follows the authoring rules (labels quoted verbatim from the real Mermaid parser's
//! canonical sets of the reference, direction stated in words, no syntax named outside quoted
//! labels), the two grammar cards are of comparable size, the shared template differs between the
//! arms only in its slots, and the manifest pins every frozen file by hash. The count and quota
//! gates are added with the rest of the set (91-03).

use std::path::{Path, PathBuf};

use serde_json::Value;
use vox_cli::commands::model::eval_diagram::prompt::{arm_slots, parse_template, render_messages};
use vox_cli::commands::model::eval_diagram::{Arm, OracleOutput, Task, file_hash, read_jsonl};
use vox_doc_pipeline::diagram::ir::Ir;
use vox_doc_pipeline::diagram::{GRAMMAR_CARD, check};

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

fn dataset() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/eval/diagram-authoring")
}

fn read(name: &str) -> String {
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

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn within_ten_percent(a: usize, b: usize) -> bool {
    a.abs_diff(b) * 10 <= b
}

#[test]
fn grammar_cards_are_comparable() {
    let vd = read("cards/voxdiagram.txt");
    let mm = read("cards/mermaid.txt");
    assert_eq!(
        vd, GRAMMAR_CARD,
        "cards/voxdiagram.txt is not the VoxDiagram grammar card"
    );
    assert!(
        within_ten_percent(mm.len(), vd.len()),
        "bytes: mermaid {} vs voxdiagram {}",
        mm.len(),
        vd.len()
    );
    assert!(
        within_ten_percent(words(&mm), words(&vd)),
        "words: mermaid {} vs voxdiagram {}",
        words(&mm),
        words(&vd)
    );
    let first = mm.lines().next().unwrap_or_default();
    assert!(
        first.contains("flowchart") && first.contains("sequenceDiagram"),
        "{first}"
    );
    assert!(first.contains("mermaid fenced block"), "{first}");
    let paragraphs: Vec<&str> = mm.split("\n\n").collect();
    for head in ["flowchart ", "sequenceDiagram"] {
        assert_eq!(
            paragraphs.iter().filter(|p| p.starts_with(head)).count(),
            1,
            "one {head:?} example paragraph"
        );
    }
    assert_eq!(
        mm.lines().last(),
        Some("Run the parser; fix the reported line; repeat until it parses.")
    );
    assert!(within_ten_percent(105, 100) && !within_ten_percent(111, 100));
}

#[test]
fn template_differs_between_arms_only_in_its_slots() {
    let template = parse_template(&read("prompt-template.txt")).expect("prompt-template.txt");
    let tasks: Vec<Task> = read_jsonl(&read("tasks.v1.jsonl")).expect("tasks");
    let history = vec![
        (
            "```\nfirst try\n```".to_string(),
            "line 3: unexpected token".to_string(),
        ),
        ("second try".to_string(), "[{\"code\":\"x\"}]".to_string()),
    ];
    for task in &tasks {
        let masked = |arm: Arm, card_file: &str| -> Vec<(String, String)> {
            let card = read(card_file);
            let s = arm_slots(arm);
            let messages = render_messages(&template, &card, arm, &task.prompt, &history);
            assert_eq!(messages[0].content, card);
            assert!(
                messages[1].content.contains(&task.prompt),
                "{}: prompt not verbatim",
                task.id
            );
            messages
                .into_iter()
                .map(|m| {
                    let content = m
                        .content
                        .replace(&card, "<CARD>")
                        .replace(&task.prompt, "<PROMPT>")
                        .replace(s.fence, "<FENCE>")
                        .replace(s.lang, "<LANG>")
                        .replace(s.validator, "<VALIDATOR>");
                    (m.role, content)
                })
                .collect()
        };
        let vd = masked(Arm::Voxdiagram, "cards/voxdiagram.txt");
        assert_eq!(vd, masked(Arm::Mermaid, "cards/mermaid.txt"), "{}", task.id);
        assert!(vd[1].1.contains("<PROMPT>"), "{}", task.id);
    }
}

#[test]
fn manifest_pins_the_frozen_files() {
    let manifest: serde_yaml::Value =
        serde_yaml::from_str(&read("manifest.v1.yaml")).expect("manifest.v1.yaml");
    let tasks: Vec<Task> = read_jsonl(&read("tasks.v1.jsonl")).expect("tasks");
    assert_eq!(manifest["corpus"].as_str(), Some("diagram-authoring"));
    assert_eq!(manifest["corpus_size"].as_u64(), Some(tasks.len() as u64));
    assert_eq!(manifest["mens"]["ingested"].as_bool(), Some(false));
    let files = manifest["files"].as_mapping().expect("files mapping");
    let pinned = [
        "tasks.v1.jsonl",
        "cards/voxdiagram.txt",
        "cards/mermaid.txt",
        "prompt-template.txt",
    ];
    assert_eq!(files.len(), pinned.len(), "files: {files:?}");
    let mut stale = Vec::new();
    for name in pinned {
        let recorded = files.get(name).and_then(|v| v.as_str());
        let actual = file_hash(&dataset().join(name)).expect("hash");
        if recorded != Some(actual.as_str()) {
            stale.push(format!("  {name}: {actual}"));
        }
    }
    assert!(
        stale.is_empty(),
        "frozen files changed without the manifest; after reviewing the change, set files to:\n{}",
        stale.join("\n")
    );
}
