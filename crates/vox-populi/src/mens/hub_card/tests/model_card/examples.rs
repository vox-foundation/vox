//! The Examples section: rule clause, fences, verdict lines and the serving-path fallback.

use serde_json::{Value, json};
use vox_eval::qwen_reasoning::answer_region;

use super::super::support::{
    Case, LEGACY_CKPT, NEEDS_RE_GATE, PASSED, STALE_SERVING, cx_for, fixture_root, render_case,
};
use super::{load, trained_cases};
use crate::mens::hub_card::card_facts::{CardFacts, card_facts};
use crate::mens::hub_card::examples::{Verdict, pick_examples, shown_code};
use crate::mens::hub_card::{RunRecords, render_model_card};

/// One piece of a card body: a prose line, or a fenced block with its info string.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Piece {
    Line(String),
    Block { info: String, text: String },
}

/// Split markdown into lines and fenced blocks. A block closes on a line of exactly as many
/// backticks as opened it, which is the rule a Markdown reader applies.
pub(super) fn parse(text: &str) -> Vec<Piece> {
    let mut out = Vec::new();
    let mut open: Option<(usize, String, Vec<&str>)> = None;
    for line in text.lines() {
        let ticks = line.chars().take_while(|c| *c == '`').count();
        let closes = open
            .as_ref()
            .map(|(n, _, _)| ticks == *n && line.len() == *n);
        match closes {
            Some(true) => {
                let (_, info, body) = open.take().expect("open block");
                out.push(Piece::Block {
                    info,
                    text: body.join("\n"),
                });
            }
            Some(false) => open.as_mut().expect("open block").2.push(line),
            None if ticks >= 3 => open = Some((ticks, line[ticks..].to_string(), Vec::new())),
            None => out.push(Piece::Line(line.to_string())),
        }
    }
    assert!(open.is_none(), "a fence is never closed");
    out
}

/// The pieces of the `## name` section, heading excluded.
pub(super) fn section(card: &str, name: &str) -> Vec<Piece> {
    let all = parse(super::body(card));
    let start = all
        .iter()
        .position(|p| *p == Piece::Line(format!("## {name}")))
        .unwrap_or_else(|| panic!("no `## {name}` in\n{card}"));
    all[start + 1..]
        .iter()
        .take_while(|p| !matches!(p, Piece::Line(l) if l.starts_with("## ")))
        .cloned()
        .collect()
}

/// Non-empty prose lines of a section.
pub(super) fn prose(sec: &[Piece]) -> Vec<&str> {
    sec.iter()
        .filter_map(|p| match p {
            Piece::Line(l) if !l.is_empty() => Some(l.as_str()),
            _ => None,
        })
        .collect()
}

/// `(info, text)` of every fenced block of a section.
pub(super) fn blocks(sec: &[Piece]) -> Vec<(&str, &str)> {
    sec.iter()
        .filter_map(|p| match p {
            Piece::Block { info, text } => Some((info.as_str(), text.as_str())),
            _ => None,
        })
        .collect()
}

pub(super) fn facts(case: &Case) -> CardFacts {
    card_facts(&cx_for(case), &load(case, |_| {}))
}

/// Render `case` after `edit` changed its loaded records.
pub(super) fn render_edited(case: &Case, edit: impl FnOnce(&mut RunRecords)) -> String {
    let mut rec = load(case, |_| {});
    edit(&mut rec);
    render_model_card(&cx_for(case), &rec)
}

pub(super) fn passes(report: &Value) -> Vec<(String, bool)> {
    report["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["id"].as_str().unwrap().to_string(),
                r["samples"][0]["pass"].as_bool().unwrap(),
            )
        })
        .collect()
}

/// Overwrite each item's stored `pass` in `report` with `f(id, current)`.
pub(super) fn set_pass(report: &mut Option<Value>, f: impl Fn(&str, bool) -> bool) {
    for r in report.as_mut().unwrap()["results"].as_array_mut().unwrap() {
        let id = r["id"].as_str().unwrap().to_string();
        let now = r["samples"][0]["pass"].as_bool().unwrap();
        r["samples"][0]["pass"] = json!(f(&id, now));
    }
}

fn first_prose(card: &str) -> String {
    let sec = section(card, "Examples");
    prose(&sec)[0].to_string()
}

#[test]
fn examples_section_renders_wins_and_loss() {
    let case = NEEDS_RE_GATE;
    let ex = facts(&case)
        .examples
        .expect("current-serving evidence has examples");
    assert_eq!((ex.wins.len(), ex.loss.is_some()), (2, true), "{ex:?}");
    let card = render_case(&case);
    let sec = section(&card, "Examples");
    let lines = prose(&sec);
    let first = lines[0];
    assert!(first.starts_with("For 1.7B, the examples are"), "{first}");
    let totals = format!(
        "of {} items it fixes {} and breaks {}.",
        ex.total, ex.fixes, ex.breaks
    );
    assert!(first.contains(&totals), "{first}");
    let heads: Vec<&&str> = lines.iter().filter(|l| l.starts_with("### ")).collect();
    assert_eq!(heads.len(), ex.wins.len() + 1);
    for ((win, e), head) in ex.blocks().iter().zip(heads) {
        assert!(head.contains(&format!("`{}`", e.id)), "{head}");
        assert_eq!(head.contains("Fixed"), *win, "{head}");
    }
    // The prompt, then base and adapter output, per block: three fences each.
    assert_eq!(blocks(&sec).len(), 3 * (ex.wins.len() + 1));
    let text = |i: usize| blocks(&sec)[i].1.to_string();
    assert_eq!(text(0), ex.wins[0].description);
}

#[test]
fn rule_and_totals_on_every_trained_card_with_examples() {
    let mut seen: Vec<(&str, String)> = Vec::new();
    for (name, case) in trained_cases() {
        let Some(ex) = facts(&case).examples else {
            continue;
        };
        let card = render_case(&case);
        let first = first_prose(&card);
        let lead = format!("For {}, the examples are", case.size);
        assert!(first.starts_with(&lead), "{name}: {first}");
        assert!(first.contains("#examples-on-the-cards"), "{name}: {first}");
        if ex.wins.len() == 2 {
            assert!(
                first.contains("the 2 shortest items the adapter fixes"),
                "{name}"
            );
        }
        assert_eq!(
            first.contains("and the shortest it still fails"),
            ex.loss.is_some(),
            "{name}: {first}"
        );
        for part in [
            format!("of {} items", ex.total),
            format!("it fixes {} and", ex.fixes),
            format!("breaks {}.", ex.breaks),
        ] {
            assert!(first.contains(&part), "{name}: `{part}` not in {first}");
        }
        seen.push((case.size, first));
    }
    assert!(seen.len() >= 3, "{seen:?}");
    for (i, (size_a, a)) in seen.iter().enumerate() {
        for (size_b, b) in &seen[i + 1..] {
            assert!(
                size_a == size_b || a != b,
                "{size_a} and {size_b} share `{a}`"
            );
        }
    }
}

#[test]
fn totals_sentence_forms() {
    let base_fail = |rec: &RunRecords| {
        passes(rec.heldout_base.as_ref().unwrap())
            .into_iter()
            .find(|(_, p)| !p)
            .map(|(id, _)| id)
            .unwrap()
    };
    // Exactly one fixed item: the singular form.
    let card = render_edited(&NEEDS_RE_GATE, |rec| {
        let only = base_fail(rec);
        set_pass(&mut rec.heldout_adapter, |id, _| id == only);
    });
    let first = first_prose(&card);
    assert!(first.contains("the one item the adapter fixes"), "{first}");
    assert!(first.contains("it fixes 1 and"), "{first}");
    // No item fixed, one still failing: the zero form still names the loss.
    let card = render_edited(&NEEDS_RE_GATE, |rec| {
        let base = rec.heldout_base.clone();
        set_pass(&mut rec.heldout_adapter, |id, _| {
            passes(base.as_ref().unwrap())
                .iter()
                .any(|(i, p)| i == id && *p)
        });
    });
    let first = first_prose(&card);
    assert!(
        first.contains("none that the adapter fixes and the shortest it still fails"),
        "{first}"
    );
    assert!(first.contains("it fixes 0 and breaks 0."), "{first}");
    // Everything passes on both sides: no fix and no loss.
    let card = render_edited(&NEEDS_RE_GATE, |rec| {
        set_pass(&mut rec.heldout_base, |_, _| true);
        set_pass(&mut rec.heldout_adapter, |_, _| true);
    });
    let first = first_prose(&card);
    assert!(
        first.contains("none that the adapter fixes and no item it still fails"),
        "{first}"
    );
    assert!(!card.contains("### "), "{card}");
    // The adapter fails nothing but the base fails some: wins and an explicit no-loss clause.
    let card = render_edited(&NEEDS_RE_GATE, |rec| {
        set_pass(&mut rec.heldout_adapter, |_, _| true);
    });
    let first = first_prose(&card);
    assert!(
        first.contains("the 2 shortest items the adapter fixes and no item it still fails"),
        "{first}"
    );
    // An id in one report only is counted apart and said once.
    let card = render_edited(&NEEDS_RE_GATE, |rec| {
        rec.heldout_adapter.as_mut().unwrap()["results"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "only_adapter", "description": "d", "samples": [{"pass": true}]}));
    });
    let first = first_prose(&card);
    assert!(first.contains("found in only one of them is 1"), "{first}");
}

/// The generated texts of an eval checkpoint file.
pub(super) fn ckpt_texts(rel: &str) -> Vec<String> {
    let text = std::fs::read_to_string(fixture_root().join(rel)).unwrap();
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let item: Value = serde_json::from_str(l).ok()?;
            let resp: Value = serde_json::from_str(item["response"].as_str()?).ok()?;
            Some(resp["generated_text"].as_str()?.to_string())
        })
        .collect()
}

#[test]
fn stale_serving_card_shows_no_adapter_output() {
    let card = render_case(&STALE_SERVING);
    let sec = section(&card, "Examples");
    let lines = prose(&sec);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(
        lines[0].starts_with("No 1.7B example is shown"),
        "{}",
        lines[0]
    );
    assert!(lines[0].contains("has no serving field"), "{}", lines[0]);
    assert_eq!(blocks(&sec), [("text", "no recorded output")]);
    assert!(!card.contains("### ") && !card.contains("the examples are"));
    assert!(!card.contains("it fixes "), "{card}");
    // None of the older serving path's adapter code is anywhere on the card.
    let legacy = ckpt_texts(LEGACY_CKPT);
    assert!(!legacy.is_empty());
    for t in legacy
        .iter()
        .map(|t| shown_code(t))
        .filter(|c| c.len() > 20)
    {
        assert!(!card.contains(&t), "legacy adapter code on the card: {t}");
    }
    // Another serving path is named; a missing checkpoint says so.
    let card = render_edited(&NEEDS_RE_GATE, |rec| {
        rec.adapter_serving = Some("folded_into_base v1".to_string());
    });
    assert!(
        card.contains("records the serving path `folded_into_base v1`"),
        "{card}"
    );
    assert!(!card.contains("### "));
    let card = render_edited(&NEEDS_RE_GATE, |rec| rec.heldout_adapter_ckpt = None);
    let sec = section(&card, "Examples");
    assert!(prose(&sec)[0].contains("no adapter checkpoint records its serving path"));
    assert_eq!(blocks(&sec), [("text", "no recorded output")]);
}

const DOCS_PAGE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/src/reference/mens-models.md"
));

#[test]
fn verdict_phrases_match_docs_page() {
    let table: Vec<&str> = DOCS_PAGE
        .lines()
        .skip_while(|l| !l.starts_with("| Phrase on a card |"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
        .collect();
    let on_page: Vec<&str> = table
        .iter()
        .map(|row| row.split('|').nth(1).unwrap().trim().trim_matches('`'))
        .collect();
    let in_code: Vec<&str> = [
        Verdict::Passes,
        Verdict::Empty,
        Verdict::ParseErrors(None),
        Verdict::TypeErrors(None),
        Verdict::Stub,
        Verdict::Rejected(String::new()),
    ]
    .iter()
    .map(Verdict::phrase)
    .collect();
    assert_eq!(on_page, in_code);
}

#[test]
fn failing_outputs_are_never_in_vox_fences() {
    let mut checked = 0;
    for (name, case) in trained_cases() {
        let Some(ex) = facts(&case).examples else {
            continue;
        };
        let card = render_case(&case);
        let sec = section(&card, "Examples");
        let mut want: Vec<(&str, String)> = Vec::new();
        for (_, e) in ex.blocks() {
            want.push(("text", e.description.clone()));
            for (_, s) in e.sides() {
                want.push(match (&s.code, s.passed) {
                    (Some(code), true) => ("vox", code.clone()),
                    (Some(code), false) => ("text", code.clone()),
                    (None, _) => ("text", "no recorded output".to_string()),
                });
            }
        }
        let got: Vec<(&str, String)> = blocks(&sec)
            .into_iter()
            .map(|(i, t)| (i, t.to_string()))
            .collect();
        assert_eq!(got, want, "{name}");
        checked += got.iter().filter(|(i, _)| *i == "vox").count();
    }
    assert!(checked > 0, "no passing output was exercised");
}

#[test]
fn long_backtick_runs_in_a_prompt_cannot_close_the_fence() {
    let card = render_edited(&NEEDS_RE_GATE, |rec| {
        for report in [&mut rec.heldout_base, &mut rec.heldout_adapter] {
            for r in report.as_mut().unwrap()["results"].as_array_mut().unwrap() {
                r["description"] = json!(format!(
                    "{} ````` ```` end",
                    r["description"].as_str().unwrap()
                ));
            }
        }
    });
    let sec = section(&card, "Examples");
    let first = &blocks(&sec)[0];
    assert!(first.1.contains("````` ```` end"), "{first:?}");
    assert!(
        card.contains("``````text\n"),
        "the prompt fence is six backticks"
    );
}

#[test]
fn verdict_lines_name_size_and_item() {
    let phrases = [
        "passes",
        "no code",
        "does not parse",
        "type errors",
        "stub",
        "rejected by",
    ];
    for (name, case) in trained_cases() {
        let Some(ex) = facts(&case).examples else {
            continue;
        };
        let card = render_case(&case);
        let sec = section(&card, "Examples");
        let lines = prose(&sec);
        for (_, e) in ex.blocks() {
            for (is_base, side) in e.sides() {
                let who = if is_base {
                    format!("Qwen3-{} base on `{}`: ", case.size, e.id)
                } else {
                    format!("{} adapter on `{}`: ", case.size, e.id)
                };
                let line = lines
                    .iter()
                    .find(|l| l.starts_with(&who))
                    .unwrap_or_else(|| panic!("{name}: no `{who}` line in {lines:?}"));
                let tail = &line[who.len()..];
                assert!(tail.starts_with(side.verdict.phrase()), "{name}: {line}");
                let hits = phrases.iter().filter(|p| tail.contains(*p)).count();
                assert_eq!(hits, 1, "{name}: not exactly one phrase in {line}");
            }
        }
    }
}

#[test]
fn vox_symbols_are_not_escaped() {
    // A recorded win whose code holds the symbols an HTML escaper would rewrite, and a backtick run.
    let code = "fn f(a: int, b: int) to bool {\n    // `` quoted\n    a < b && b > a -> a\n}";
    let recorded = format!("```vox\n{code}\n```");
    let card = render_edited(&PASSED, |rec| {
        let id = pick_examples(
            rec.heldout_base.as_ref().unwrap(),
            rec.heldout_adapter.as_ref().unwrap(),
            None,
            None,
        )
        .unwrap()
        .wins[0]
            .id
            .clone();
        let row = rec.heldout_adapter.as_mut().unwrap()["results"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["id"] == id.as_str())
            .unwrap();
        let old = row["samples"][0]["completion_preview"]
            .as_str()
            .unwrap()
            .to_string();
        let texts = rec.heldout_adapter_ckpt.as_mut().unwrap();
        let i = texts
            .iter()
            .position(|t| answer_region(t).chars().take(240).eq(old.chars()))
            .expect("the win's checkpoint text");
        texts[i] = recorded.clone();
        let preview: String = answer_region(&recorded).chars().take(240).collect();
        row["samples"][0]["completion_preview"] = preview.into();
    });
    let sec = section(&card, "Examples");
    assert!(blocks(&sec).contains(&("vox", code)), "{card}");
    for escaped in ["&lt;", "&gt;", "&amp;", "&#"] {
        assert!(!card.contains(escaped), "`{escaped}` on the card\n{card}");
    }
}
