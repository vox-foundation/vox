//! Sequence IR to Mermaid `sequenceDiagram` text, one statement per line.

use super::escape::seq_text;
use crate::sequence_ir::{SeqEvent, SequenceIr};

/// Display text that may not be empty (alias, note body): blank prints as a numeric space.
fn filled(s: &str) -> String {
    let t = seq_text(s);
    if t.trim().is_empty() {
        "#32;".to_owned()
    } else {
        t
    }
}

/// Index of `id` among the participants, appending it when no actor declared it.
fn at<'a>(ids: &mut Vec<&'a str>, id: &'a str) -> usize {
    if let Some(i) = ids.iter().position(|x| *x == id) {
        return i;
    }
    ids.push(id);
    ids.len() - 1
}

/// One resolved event: participant indexes instead of ids.
enum Step<'a> {
    Message(usize, usize, bool, &'a str),
    Note(Vec<usize>, &'a str),
}

/// Export `ir` as Mermaid text with one line per statement: `sequenceDiagram`, one
/// `participant p<i> as <label>` per actor, one more per id that events use but no actor
/// declares (first-appearance order, labelled with the id), then the events in order.
///
/// Participant ids are always opaque `p<index>` because sequence keywords such as `end`,
/// `loop` and `opt` break as actor ids. A repeated actor id resolves to its first occurrence.
/// Total over any `SequenceIr`. A note whose `over` is empty or longer than two is degenerate
/// (neither the importer nor the parser produces one): it exports with at most its first two
/// ids and is outside the acceptance guarantee. `export` routes `Ir::Sequence` here.
pub fn export_sequence(ir: &SequenceIr) -> String {
    let mut ids: Vec<&str> = ir.actors.iter().map(|a| a.id.as_str()).collect();
    let steps: Vec<Step<'_>> = ir
        .events
        .iter()
        .map(|e| match e {
            SeqEvent::Message {
                from,
                to,
                text,
                dashed,
                ..
            } => Step::Message(at(&mut ids, from), at(&mut ids, to), *dashed, text),
            SeqEvent::Note { over, text, .. } => Step::Note(
                over.iter().take(2).map(|id| at(&mut ids, id)).collect(),
                text,
            ),
        })
        .collect();
    let mut out = String::from("sequenceDiagram\n");
    for (i, id) in ids.iter().enumerate() {
        let label = ir.actors.get(i).map_or(*id, |a| a.label.as_str());
        out.push_str(&format!("    participant p{i} as {}\n", filled(label)));
    }
    for step in &steps {
        match step {
            Step::Message(f, t, dashed, text) => {
                let arrow = if *dashed { "-->>" } else { "->>" };
                let body = seq_text(text);
                out.push_str(&format!("    p{f}{arrow}p{t}:"));
                if !body.trim().is_empty() {
                    out.push(' ');
                    out.push_str(&body);
                }
            }
            Step::Note(over, text) => {
                let who: Vec<String> = over.iter().map(|i| format!("p{i}")).collect();
                out.push_str(&format!(
                    "    Note over {}: {}",
                    who.join(","),
                    filled(text)
                ));
            }
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence_ir::Actor;

    fn actor(id: &str, label: &str) -> Actor {
        Actor {
            id: id.into(),
            label: label.into(),
            span: Default::default(),
        }
    }

    #[test]
    fn keyword_ids_become_opaque_participants() {
        let ir = SequenceIr {
            actors: vec![actor("end", "E"), actor("loop", "L")],
            events: vec![SeqEvent::Message {
                from: "end".into(),
                to: "loop".into(),
                text: "go".into(),
                dashed: false,
                span: Default::default(),
            }],
        };
        assert_eq!(
            export_sequence(&ir),
            "sequenceDiagram\n    participant p0 as E\n    participant p1 as L\n    p0->>p1: go\n"
        );
    }

    #[test]
    fn empty_diagram_is_just_the_header() {
        let ir = SequenceIr {
            actors: vec![],
            events: vec![],
        };
        assert_eq!(export_sequence(&ir), "sequenceDiagram\n");
    }
}
