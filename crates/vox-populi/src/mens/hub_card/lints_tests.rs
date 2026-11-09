use super::*;
use crate::mens::hub_card::tests::support::ladder_cards;
use crate::mens::hub_repo::SIZES;
use serde_yaml::Value;

fn card(body: &str) -> String {
    format!("---\nlicense: x\n---\n{body}\n{FOOTER_MARKER}\n")
}

#[test]
fn sentence_units_follow_docs_rule() {
    let text = "---\na: b. c. d. e. f.\n---\n# Title here now\n\n\
First line of a paragraph\nsecond line. Third word!\n\n\
- item one. two three\n* other item here\n\n| a b | c d. |\n|---|---|\n\n\
````text\nSkip. Skip. Skip. Skip.\n```\nstill inside. Skip.\n````\n\
After the fence A b c. D e f g? H\n\
See [the docs page](https://voxlang.org/x/#y) for 0.6B and v1.2 here.\n";
    assert_eq!(
        sentences(text),
        [
            "# title here now",
            "first line of a paragraph second line",
            "third word",
            "- item one",
            "two three",
            "other item here",
            "a b c d",
            "after the fence a b c",
            "d e f g",
            "h see the docs page for 0.6b and v1.2 here",
        ]
    );
}

#[test]
fn normalize_and_count() {
    assert_eq!(
        normalize("**Bold** `code` | a_b | _em_ [t](http://u/v.x) Text."),
        "bold code a_b em t text"
    );
    assert_eq!(word_count("--- — a b"), 2);
    assert_eq!(word_count("---"), 0);
}

#[test]
fn duplicates_catches_repeat_on_one_card_and_across_cards() {
    let one = card(
        "Four words right here. Four words right here.\n\nThree words here. Three words here.",
    );
    assert_eq!(
        duplicates(&[("a".into(), one)]),
        [("four words right here".into(), vec!["a".into(), "a".into()])]
    );
    let across = [
        ("a".to_string(), card("The size is stated here.")),
        ("b".to_string(), card("The size is stated here!")),
    ];
    assert_eq!(duplicates(&across).len(), 1);
    // The footer is the one exception; the same line anywhere else is not.
    let footers = [
        ("a".to_string(), card("One.")),
        ("b".to_string(), card("Two.")),
    ];
    assert!(duplicates(&footers).is_empty());
    let midway = format!("{FOOTER_MARKER} and more words here.\n\nTail text.");
    let moved = [("a".to_string(), midway.clone()), ("b".to_string(), midway)];
    assert_eq!(duplicates(&moved).len(), 1);
}

#[test]
fn no_duplicate_sentences_across_ladder() {
    let cards = ladder_cards();
    assert_eq!(
        cards.len(),
        SIZES.len() + 1,
        "one card per size plus the index"
    );
    for (name, md) in &cards {
        assert!(
            md.trim_end()
                .lines()
                .last()
                .is_some_and(|l| l.contains(FOOTER_MARKER)),
            "{name} does not end with the footer line"
        );
        assert!(!sentences(md).is_empty(), "{name}: no sentence inspected");
    }
    // Size-specific clauses and the stale-serving reasons are inside the checked set (B1, B2).
    let with_examples = SIZES
        .iter()
        .filter(|size| {
            let want = format!("for {}, the examples are", size.to_lowercase());
            cards
                .iter()
                .any(|(_, md)| sentences(md).iter().any(|s| s.starts_with(&want)))
        })
        .count();
    assert!(
        with_examples >= 2,
        "{with_examples} cards carry the examples clause"
    );
    assert!(
        cards.iter().any(|(_, md)| sentences(md)
            .iter()
            .any(|s| s.contains("has no serving field"))),
        "no card carries a stale-serving reason"
    );
    let dup = duplicates(&cards);
    assert!(
        dup.is_empty(),
        "sentences found twice:\n{}",
        dup.iter()
            .map(|(s, on)| format!("  {s:?} on {on:?}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn named(cards: &[(String, String)], name: &str) -> String {
    cards
        .iter()
        .find(|(n, _)| n == name)
        .expect("card")
        .1
        .clone()
}

#[test]
fn slug_matches_site_ids() {
    // The ids on the built /reference/mens-models/ page, recorded in 339-02-SUMMARY.md.
    let site = [
        "what-mens-is",
        "hub-and-spoke",
        "the-harness",
        "gating",
        "held-out-pass-rate",
        "collateral-damage",
        "why-mcnemar",
        "pinned-slices",
        "state-legend",
        "examples-on-the-cards",
        "prompt-protocol-and-training-method",
        "shared-training-settings",
        "why-peft-cannot-load-the-adapters",
        "how-the-cards-are-made",
        "links",
    ];
    assert_eq!(
        doc_slugs(DOCS_MD),
        site.map(String::from).into_iter().collect()
    );
    assert_eq!(slug("Held-out pass rate"), "held-out-pass-rate");
    assert_eq!(
        doc_slugs("## A b\n\n```\n## skipped\n```\n### A b\n"),
        ["a-b", "a-b-1"].map(String::from).into_iter().collect()
    );
}

#[test]
fn every_card_anchor_exists_in_mens_models() {
    let slugs = doc_slugs(DOCS_MD);
    let (mut seen, mut missing) = (0, Vec::new());
    for (name, md) in ladder_cards() {
        for a in card_anchors(&md) {
            seen += 1;
            if !slugs.contains(&a) {
                missing.push(format!("{name} links #{a}"));
            }
        }
    }
    assert!(seen > 0, "no docs anchor was inspected");
    assert!(
        missing.is_empty(),
        "anchors with no heading on mens-models.md: {missing:?}"
    );
}

#[test]
fn frontmatter_matches_hf_rules() {
    let rules = hf_rules();
    let mut cards = ladder_cards();
    let golden = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/hub_cards");
    let mut goldens = 0;
    for f in std::fs::read_dir(golden).expect("golden dir") {
        let path = f.expect("entry").path();
        if path.extension().is_some_and(|x| x == "md") {
            let stem = path
                .file_stem()
                .expect("stem")
                .to_string_lossy()
                .into_owned();
            cards.push((stem, std::fs::read_to_string(&path).expect("golden")));
            goldens += 1;
        }
    }
    assert!(
        goldens >= 1 && cards.len() == SIZES.len() + 1 + goldens,
        "{goldens} goldens"
    );
    let mut errs = Vec::new();
    for (name, md) in &cards {
        match frontmatter_of(md) {
            Some(fm) => errs.extend(
                check_hf_metadata(&fm, &rules, kind_of(name, md))
                    .into_iter()
                    .map(|e| format!("{name}: {e}")),
            ),
            None => errs.push(format!("{name}: frontmatter does not parse")),
        }
    }
    assert!(errs.is_empty(), "{}", errs.join("\n"));
}

#[test]
fn hf_rules_reject_bad_metadata() {
    let rules = hf_rules();
    let md = named(&ladder_cards(), "vox-mens-qwen3-4b");
    let good = frontmatter_of(&md).expect("frontmatter");
    assert_eq!(kind_of("vox-mens-qwen3-4b", &md), Kind::Trained);
    assert_eq!(
        check_hf_metadata(&good, &rules, Kind::Trained),
        Vec::<String>::new()
    );
    let bad = |edit: &dyn Fn(&mut Value), want: &str| {
        let mut fm = good.clone();
        edit(&mut fm);
        let errs = check_hf_metadata(&fm, &rules, Kind::Trained);
        assert!(errs.iter().any(|e| e.contains(want)), "{want}: {errs:?}");
    };
    bad(
        &|fm| fm["base_model"] = "Qwen/Qwen3-4B@abc".into(),
        "base_model",
    );
    bad(
        &|fm| fm["base_model_relation"] = "lora".into(),
        "base_model_relation",
    );
    bad(&|fm| fm["extra"] = "x".into(), "unknown key `extra`");
    let metric = |fm: &mut Value| {
        let m = &mut fm["model-index"][0]["results"][0]["metrics"][0];
        assert!(m.get("value").is_some(), "fixture card has a metric value");
        m.clone()
    };
    bad(
        &|fm| {
            metric(fm);
            fm["model-index"][0]["results"][0]["metrics"][0]
                .as_mapping_mut()
                .unwrap()
                .remove("value");
        },
        "missing `value`",
    );
    bad(
        &|fm| {
            metric(fm);
            fm["model-index"][0]["results"][0]["metrics"][0]["value"] = "0.6".into();
        },
        "value must be a number",
    );
    bad(
        &|fm| {
            fm["tags"]
                .as_sequence_mut()
                .unwrap()
                .retain(|t| !t.as_str().unwrap().starts_with("gate-"))
        },
        "`gate-`",
    );
    bad(
        &|fm| {
            fm["tags"]
                .as_sequence_mut()
                .unwrap()
                .retain(|t| !t.as_str().unwrap().starts_with("spoke-"))
        },
        "`spoke-`",
    );
    let not_trained = check_hf_metadata(&good, &rules, Kind::NotTrained);
    assert!(
        not_trained.iter().any(|e| e.contains("not-trained")),
        "{not_trained:?}"
    );
}

#[test]
fn vox_blocks_compile() {
    let (mut checked, mut bad) = (0, Vec::new());
    for (name, md) in ladder_cards() {
        let (n, failures) = vox_failures(&md);
        checked += n;
        bad.extend(failures.into_iter().map(|e| format!("{name}: {e}")));
    }
    assert!(checked > 0, "no vox block was inspected");
    assert!(bad.is_empty(), "{bad:?}");
}

#[test]
fn lint_rejects_unparseable_vox_block() {
    let (n, bad) = vox_failures(&card(
        "```vox\nfn ((( not vox\n```\n\n```text\nfn ((( not checked\n```",
    ));
    assert_eq!((n, bad.len()), (1, 1), "{bad:?}");
}

#[test]
fn lint_rejects_failing_output_in_vox_fence() {
    use crate::mens::hub_card::RunRecords;
    use crate::mens::hub_card::examples::{Verdict, pick_examples};
    use crate::mens::hub_card::tests::support::{CURRENT_CKPT, evidence_run};
    let run = evidence_run(CURRENT_CKPT);
    let rec = RunRecords::load(
        run.path(),
        "4B",
        "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c",
    );
    let (base, adapter) = (
        rec.heldout_base.as_ref().unwrap(),
        rec.heldout_adapter.as_ref().unwrap(),
    );
    let facts = pick_examples(
        base,
        adapter,
        rec.heldout_base_ckpt.as_deref(),
        rec.heldout_adapter_ckpt.as_deref(),
    )
    .expect("examples");
    let ex = facts
        .wins
        .iter()
        .find(|e| matches!(e.base.verdict, Verdict::ParseErrors(_)))
        .expect("a win whose base output does not parse");
    let stored = base["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == ex.id.as_str())
        .unwrap();
    assert_eq!(
        stored["samples"][0]["checks"]["parse_ok"], false,
        "{}",
        ex.id
    );
    let code = ex.base.code.as_deref().expect("recorded base output");
    let fence = ex.base.fence();
    let (n, bad) = vox_failures(&card(&format!("{fence}vox\n{code}\n{fence}")));
    assert_eq!((n, bad.len()), (1, 1), "{}: {bad:?}", ex.id);
}

/// What the four lints looked at, so a caller can refuse a pass over nothing.
#[derive(Debug, Default)]
struct Inspected {
    sentences: usize,
    anchors: usize,
    vox_blocks: usize,
}

/// All four lints over one set of `(name, markdown)` cards.
fn violations(cards: &[(String, String)]) -> (Inspected, Vec<String>) {
    let (rules, slugs) = (hf_rules(), doc_slugs(DOCS_MD));
    let (mut seen, mut out) = (Inspected::default(), Vec::new());
    for (s, on) in duplicates(cards) {
        out.push(format!("sentence found twice: {s:?} on {on:?}"));
    }
    for (name, md) in cards {
        seen.sentences += sentences(md).len();
        for a in card_anchors(md) {
            seen.anchors += 1;
            if !slugs.contains(&a) {
                out.push(format!("{name}: no heading for #{a} on mens-models.md"));
            }
        }
        match frontmatter_of(md) {
            Some(fm) => out.extend(
                check_hf_metadata(&fm, &rules, kind_of(name, md))
                    .into_iter()
                    .map(|e| format!("{name}: {e}")),
            ),
            None => out.push(format!("{name}: frontmatter does not parse")),
        }
        let (n, bad) = vox_failures(md);
        seen.vox_blocks += n;
        out.extend(
            bad.into_iter()
                .map(|e| format!("{name}: vox block fails: {e}")),
        );
    }
    (seen, out)
}

#[test]
fn the_ladder_passes_all_four_lints_and_inspects_something() {
    let cards = ladder_cards();
    let (seen, out) = violations(&cards);
    assert!(out.is_empty(), "{out:#?}");
    assert!(
        seen.sentences > 0 && seen.anchors > 0 && seen.vox_blocks > 0,
        "{seen:?}"
    );
}

/// Run by 339-10 on the real rendered cards (`vox mens hub sync` output, never pushed):
/// `VOX_HUB_CARDS_DIR=<abs path to mens/runs/hub_cards> cargo test -p vox-populi --features
/// mens-hf-hub --lib lints_hold_on_rendered_cards_dir -- --ignored`.
#[test]
#[ignore = "needs VOX_HUB_CARDS_DIR, the directory of rendered cards"]
fn lints_hold_on_rendered_cards_dir() {
    use crate::mens::hub_card::repo_name;
    let dir = std::env::var_os("VOX_HUB_CARDS_DIR")
        .map(std::path::PathBuf::from)
        .expect("set VOX_HUB_CARDS_DIR to the directory `vox mens hub sync` renders cards into");
    let mut names: Vec<String> = SIZES.iter().map(|s| repo_name(s)).collect();
    names.push("vox-mens-index".into());
    let (mut cards, mut missing) = (Vec::new(), Vec::new());
    for name in names {
        match std::fs::read_to_string(dir.join(&name).join("README.md")) {
            Ok(md) => cards.push((name, md)),
            Err(e) => missing.push(format!("{name}/README.md: {e}")),
        }
    }
    assert!(
        missing.is_empty(),
        "{} lacks cards: {missing:?}",
        dir.display()
    );
    let (seen, out) = violations(&cards);
    eprintln!("inspected {seen:?}");
    assert!(seen.sentences > 0 && seen.anchors > 0, "{seen:?}");
    assert!(out.is_empty(), "{out:#?}");
}
