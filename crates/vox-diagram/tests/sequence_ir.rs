//! Serde round-trip for the sequence IR types (no parser until Phase 86).
use vox_diagram::sequence_ir::{Actor, SeqEvent, SequenceIr};
use vox_diagram::span::Location;

fn loc(line: u32) -> Location {
    Location {
        line,
        end_line: line,
        col: 1,
        end_col: 2,
        ..Location::default()
    }
}

fn actor(id: &str, line: u32) -> Actor {
    Actor {
        id: id.into(),
        label: id.to_uppercase(),
        span: loc(line),
    }
}

#[test]
fn sequence_ir_serde_round_trip() {
    let ir = SequenceIr {
        actors: vec![actor("a", 1), actor("b", 2)],
        events: vec![
            SeqEvent::Message {
                from: "a".into(),
                to: "b".into(),
                text: "hi".into(),
                dashed: false,
                span: loc(3),
            },
            SeqEvent::Message {
                from: "b".into(),
                to: "a".into(),
                text: "ok".into(),
                dashed: true,
                span: loc(4),
            },
            SeqEvent::Note {
                over: vec!["a".into(), "b".into()],
                text: "n".into(),
                span: loc(5),
            },
        ],
    };
    let json = serde_json::to_string(&ir).expect("serialize");
    assert!(json.contains("\"dashed\":true"));
    assert!(json.contains("\"type\":\"note\""));
    let back: SequenceIr = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back, ir);
}

#[test]
fn sequence_ir_note_over_single_actor() {
    let ir = SequenceIr {
        actors: vec![actor("a", 1)],
        events: vec![SeqEvent::Note {
            over: vec!["a".into()],
            text: "solo".into(),
            span: loc(2),
        }],
    };
    let back: SequenceIr =
        serde_json::from_str(&serde_json::to_string(&ir).expect("serialize")).expect("deserialize");
    assert_eq!(back, ir);
}
