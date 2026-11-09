//! "Use it": the pull, serve and request blocks, and the recorded output that ends them.

use serde_json::Value;

use super::super::support::{
    Case, FAILED, LEGACY_CKPT, NEEDS_RE_GATE, NO_EXAMPLES, NOT_EVALUATED, PASSED, fixture_root,
    render_case, vox_block_ok,
};
use super::examples::{
    Piece, blocks, ckpt_texts, facts, passes, prose, render_edited, section, set_pass,
};
use crate::mens::hub_card::card_facts::EVAL_USER_TURN_PREFIX;
use crate::mens::hub_card::examples::shown_code;
use crate::mens::hub_card::tests::support::CURRENT_CKPT;

const EVAL_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../vox-ml-cli/src/commands/mens/eval_local_prompt.rs"
));

fn report() -> Value {
    let text =
        std::fs::read_to_string(fixture_root().join("evidence/heldout_adapter_nothink.json"));
    serde_json::from_str(&text.unwrap()).unwrap()
}

#[test]
fn use_it_request_uses_eval_user_turn() {
    // The prefix is the format string the eval builds its user turn from.
    assert!(
        EVAL_SRC.contains(&format!("\"{EVAL_USER_TURN_PREFIX}{{}}\"")),
        "eval_local_prompt.rs no longer builds `{EVAL_USER_TURN_PREFIX}<description>`"
    );
    let f = facts(&PASSED);
    let u = f.use_it().expect("a use-it item");
    let req: Value = serde_json::from_str(f.request_json.as_ref().unwrap()).unwrap();
    assert_eq!(
        req["messages"][0]["content"],
        format!("{EVAL_USER_TURN_PREFIX}{}", u.description)
    );
    assert_eq!(req["messages"][0]["role"], "user");
    assert_eq!(req["lane"], "codegen");
}

#[test]
fn passed_card_ends_use_it_with_recorded_output() {
    let f = facts(&PASSED);
    let ex = f.examples.as_ref().unwrap();
    let u = f.use_it().expect("a use-it item");
    assert!(ex.wins.iter().all(|w| w.id != u.id), "use-it repeats a win");
    let code = u.adapter.code.as_deref().expect("recorded adapter code");
    vox_block_ok(code).unwrap_or_else(|e| panic!("{}: {e}", u.id));
    // The code is what the checkpoint recorded for this item.
    let recorded: Vec<String> = ckpt_texts(CURRENT_CKPT)
        .iter()
        .map(|t| shown_code(t))
        .collect();
    assert!(recorded.iter().any(|c| c == code));

    let card = render_case(&PASSED);
    let sec = section(&card, "Use it");
    let bl = blocks(&sec);
    assert_eq!(bl.len(), 4, "{bl:?}");
    assert!(
        bl[0].1.starts_with("vox mens hub pull --size 1.7B"),
        "{bl:?}"
    );
    assert_eq!(
        bl[1],
        (
            "bash",
            "vox mens serve --model mens/runs/qwen3-1.7b --port 11435"
        )
    );
    let req = f.request_json.as_ref().unwrap();
    assert_eq!(bl[2].0, "bash");
    assert!(
        bl[2].1.ends_with(&format!("<<'JSON'\n{req}\nJSON")),
        "{}",
        bl[2].1
    );
    assert_eq!(bl[3], ("vox", code));
    // The block order ends on the recorded code, after one label sentence.
    let last = sec
        .iter()
        .rev()
        .find(|p| !matches!(p, Piece::Line(l) if l.is_empty()));
    assert!(matches!(last, Some(Piece::Block { info, .. }) if info == "vox"));
    let label = prose(&sec)
        .into_iter()
        .rev()
        .find(|l| l.starts_with("The code below"))
        .unwrap();
    for part in [
        "1.7B",
        "evidence/heldout_adapter_nothink.ckpt.jsonl",
        u.id.as_str(),
        "not a transcript",
    ] {
        assert!(label.contains(part), "`{part}` not in {label}");
    }
    assert!(!label.contains("does not record"));

    // The request carries the decode settings the report recorded.
    let v: Value = serde_json::from_str(req).unwrap();
    let r = report();
    assert!(f.decode_recorded);
    assert_eq!(v["max_tokens"], r["max_tokens"]);
    assert_eq!(v["temperature"].as_f64(), r["temperature"].as_f64());

    // Without recorded settings the request leaves them out and the label says so.
    let card = render_edited(&PASSED, |rec| {
        let r = rec.heldout_adapter.as_mut().unwrap();
        r.as_object_mut().unwrap().remove("max_tokens");
        r.as_object_mut().unwrap().remove("temperature");
    });
    let sec = section(&card, "Use it");
    let req = blocks(&sec)[2].1;
    assert!(
        !req.contains("max_tokens") && !req.contains("temperature"),
        "{req}"
    );
    assert!(
        prose(&sec)
            .iter()
            .any(|l| l.contains("1.7B report does not record the evaluation's decode settings"))
    );
}

#[test]
fn non_passed_card_shows_no_live_request() {
    for case in [NEEDS_RE_GATE, FAILED, NOT_EVALUATED] {
        assert!(facts(&case).request_json.is_none(), "{}", case.size);
        let card = render_case(&case);
        let sec = section(&card, "Use it");
        let bl = blocks(&sec);
        assert_eq!(bl.len(), 1, "{}: {bl:?}", case.size);
        assert!(bl[0].1.ends_with(" --allow-ungated"), "{bl:?}");
        assert!(!card.contains("curl") && !card.contains("mens serve --model"));
        let serve = prose(&sec)
            .into_iter()
            .find(|l| l.contains("`vox mens serve`"))
            .unwrap_or_else(|| panic!("{}: no serve sentence", case.size));
        for part in [
            case.size,
            "passing collateral report",
            "`gate_receipt.json`",
        ] {
            assert!(serve.contains(part), "`{part}` not in {serve}");
        }
    }
}

#[test]
fn passed_stale_serving_shows_no_recorded_output() {
    let stale = Case {
        ckpt: Some(LEGACY_CKPT),
        ..PASSED
    };
    let f = facts(&stale);
    assert!(f.request_json.is_none() && f.use_it().is_none());
    let card = render_case(&stale);
    let sec = section(&card, "Use it");
    let bl = blocks(&sec);
    assert_eq!(bl.len(), 3, "{bl:?}");
    assert!(bl[0].1.starts_with("vox mens hub pull") && bl[1].1.starts_with("vox mens serve"));
    assert_eq!(bl[2], ("text", "no recorded output"));
    let reason = prose(&sec)
        .into_iter()
        .find(|l| l.contains("has no serving field"))
        .unwrap();
    assert!(reason.contains("1.7B"), "{reason}");
    assert!(!card.contains("curl"));
    for t in ckpt_texts(LEGACY_CKPT)
        .iter()
        .map(|t| shown_code(t))
        .filter(|c| c.len() > 20)
    {
        assert!(!card.contains(&t), "legacy adapter code on the card: {t}");
    }
    // No checkpoint at all is a different sentence with the same fallback.
    let card = render_case(&NO_EXAMPLES);
    let sec = section(&card, "Use it");
    assert_eq!(blocks(&sec).last(), Some(&("text", "no recorded output")));
    assert!(
        prose(&sec)
            .iter()
            .any(|l| l.contains("no adapter checkpoint records its serving path"))
    );
    // Current evidence but every adapter-passing item is a shown win: one sentence, no request.
    let card = render_edited(&PASSED, |rec| {
        let only = passes(rec.heldout_base.as_ref().unwrap())
            .into_iter()
            .find(|(_, p)| !p)
            .map(|(id, _)| id)
            .unwrap();
        set_pass(&mut rec.heldout_adapter, |id, _| id == only);
    });
    let sec = section(&card, "Use it");
    assert_eq!(blocks(&sec).len(), 2, "pull and serve only");
    assert!(
        prose(&sec)
            .iter()
            .any(|l| l.starts_with("No recorded passing 1.7B output"))
    );
}
