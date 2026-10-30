//! Offline gates over the frozen non-task inputs of the diagram-authoring dataset
//! (`contracts/eval/diagram-authoring/`): the two grammar cards are of comparable size, the shared
//! template differs between the arms only in its slots, and the manifest pins every frozen file by
//! hash. The task gates live in `eval_diagram_tasks`.

use vox_cli::commands::model::eval_diagram::prompt::{arm_slots, parse_template, render_messages};
use vox_cli::commands::model::eval_diagram::{Arm, Task, file_hash, read_jsonl};
use vox_doc_pipeline::diagram::GRAMMAR_CARD;

use crate::eval_diagram_tasks::{dataset, read};

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
