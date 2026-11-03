//! `check` / `repair` for `sequence`: every independent mistake at once, with a stable code,
//! an exact span and a machine-applicable fix where one exists (the same contract as `graph`).

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use vox_diagram::diag::{CheckReport, Diagnostic};
use vox_diagram::limits::{MAX_DIAGNOSTICS, MAX_SOURCE_BYTES};
use vox_diagram::{apply_fixes, check, repair};

fn codes(r: &CheckReport) -> Vec<&str> {
    r.diagnostics.iter().map(|d| d.code).collect()
}

fn lines_with(text: &str, prefix: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with(prefix))
        .map(str::to_string)
        .collect()
}

#[test]
fn tracer_undeclared_actors_repair_in_one_round() {
    let src = "sequence {\n    actor a: \"A\"\n    a -> b: \"hi\"\n    b -> c\n    c -> b\n}\n";
    let r = check(src);
    // Every use is reported (`b` on 3, 4 and 5, `c` on 4 and 5); only the first per id has a fix.
    let uses: Vec<(u32, bool)> = r
        .diagnostics
        .iter()
        .map(|d| (d.span.line, !d.fixes.is_empty()))
        .collect();
    assert!(
        r.diagnostics
            .iter()
            .all(|d| d.code == "diagram/undeclared-actor")
    );
    assert_eq!(
        uses,
        [(3, true), (4, false), (4, true), (5, false), (5, false)]
    );
    let fixed = repair(src, 5);
    assert_eq!(fixed.rounds, 1, "{}", fixed.text);
    assert!(fixed.report.valid, "{:?}", fixed.report.diagnostics);
    assert_eq!(
        lines_with(&fixed.text, "actor"),
        ["actor a: \"A\"", "actor b: \"b\"", "actor c: \"c\""]
    );
    let order: Vec<&str> = fixed.text.lines().take(5).collect();
    assert_eq!(
        order[1..4],
        [
            "    actor a: \"A\"",
            "    actor b: \"b\"",
            "    actor c: \"c\""
        ]
    );
    assert_eq!(repair(&fixed.text, 5).rounds, 0);
}

#[test]
fn declare_fix_goes_first_without_actors() {
    let r = repair("sequence {\n    a -> b\n}\n", 5);
    assert_eq!(r.rounds, 1, "{}", r.text);
    assert!(r.report.valid, "{:?}", r.report.diagnostics);
    let body: Vec<&str> = r.text.lines().skip(1).take(2).map(str::trim).collect();
    assert_eq!(body, ["actor a: \"a\"", "actor b: \"b\""]);
}

#[test]
fn declare_fix_respects_a_single_line_scope() {
    // The last actor line also holds the closing brace: an insert after that line would land
    // outside the block.
    let src = "sequence {\n    a -> b\n    actor a: \"A\" }\n";
    let r = check(src);
    let close = src.rfind('}').unwrap_or(0);
    for f in r.diagnostics.iter().flat_map(|d| &d.fixes) {
        assert!(f.span.end as usize <= close, "{f:?}");
    }
    let fixed = repair(src, 5);
    assert!(fixed.report.valid, "{}\n{:?}", fixed.text, fixed.report);
    let one = repair("sequence { a -> b }\n", 5);
    assert!(one.report.valid, "{}\n{:?}", one.text, one.report);
}

#[test]
fn no_declare_fix_without_both_braces() {
    let src = "sequence {\n    actor a: \"A\"\n    a -> b\n";
    let r = check(src);
    let undeclared: Vec<&Diagnostic> = r
        .diagnostics
        .iter()
        .filter(|d| d.code == "diagram/undeclared-actor")
        .collect();
    assert_eq!(undeclared.len(), 1);
    assert!(undeclared[0].fixes.is_empty());
    let fixed = repair(src, 5);
    assert!(fixed.report.valid, "{}\n{:?}", fixed.text, fixed.report);
    assert_eq!(fixed.rounds, 2);
}

#[test]
fn reserved_actor_ids() {
    for word in ["node", "graph", "sequence"] {
        let r = check(&format!("sequence {{\n    actor {word}: \"X\"\n}}\n"));
        assert_eq!(codes(&r), ["diagram/reserved-word"], "{word}");
        assert!(r.diagnostics[0].fixes.is_empty(), "{word}");
        assert!(r.diagnostics[0].message.contains("keyword"), "{word}");
    }
    let r = check("sequence {\n    actor a: \"A\"\n    a -> node\n}\n");
    assert_eq!(codes(&r), ["diagram/undeclared-actor"]);
    assert!(r.diagnostics[0].fixes.is_empty());
}

#[test]
fn caps_wording() {
    let actors: String = (0..501)
        .map(|i| format!("    actor a{i}: \"A\"\n"))
        .collect();
    let r = check(&format!("sequence {{\n{actors}}}\n"));
    assert_eq!(codes(&r), ["diagram/limit-nodes"]);
    assert!(r.diagnostics[0].message.contains("actors"), "{r:?}");
    assert!(r.ir.is_none());
    let msgs = "    a -> b\n".repeat(1_001);
    let src = format!("sequence {{\n    actor a: \"A\"\n    actor b: \"B\"\n{msgs}}}\n");
    let r = check(&src);
    assert_eq!(codes(&r), ["diagram/limit-edges"]);
    assert!(r.diagnostics[0].message.contains("events"), "{r:?}");
    assert!(r.ir.is_none());
}

#[test]
fn all_errors_at_once() {
    let src = concat!(
        "sequence {\n",
        "    actor a: \"A\"\n",
        "    actor a: \"A2\"\n",
        "    actor node: \"N\"\n",
        "    a -> ghost\n",
        "    a -> a: hello world\n",
        "    a -> a: \"\"\n",
        "    a -> a: \"oops\n",
        "}\n",
    );
    let r = check(src);
    assert_eq!(
        codes(&r),
        [
            "diagram/duplicate-id",
            "diagram/reserved-word",
            "diagram/undeclared-actor",
            "diagram/expected-quoted-label",
            "diagram/empty-label",
            "diagram/unterminated-string",
        ]
    );
    let lines: Vec<u32> = r.diagnostics.iter().map(|d| d.span.line).collect();
    assert_eq!(lines, [3, 4, 5, 6, 7, 8]);
    let fixable: Vec<bool> = r.diagnostics.iter().map(|d| !d.fixes.is_empty()).collect();
    assert_eq!(fixable, [false, false, true, true, true, true]);
    let out = apply_fixes(src, &r.diagnostics);
    assert_eq!((out.applied, out.skipped), (4, 0), "{}", out.text);
    assert_eq!(
        codes(&check(&out.text)),
        ["diagram/duplicate-id", "diagram/reserved-word"]
    );
}

fn bounded(src: String) -> CheckReport {
    assert!(src.len() < MAX_SOURCE_BYTES, "{} bytes", src.len());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let t = Instant::now();
        let _ = tx.send((check(&src), t.elapsed()));
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok((report, _)) => report,
        Err(e) => panic!("check did not finish (hang or crash): {e}"),
    }
}

fn repeat_to_cap(unit: &str, head: &str, tail: &str) -> String {
    let room = MAX_SOURCE_BYTES - head.len() - tail.len() - 1;
    format!("{head}{}{tail}", unit.repeat(room / unit.len()))
}

#[test]
fn adversarial_sequence_inputs_are_bounded() {
    let inputs = [
        repeat_to_cap("    a -> b: \"x\"\n", "sequence {\n", "}\n"),
        repeat_to_cap("reply ", "sequence {\n", "\n}\n"),
        repeat_to_cap("a, ", "sequence {\nnote over ", "\n}\n"),
        repeat_to_cap("    actor a: \"A\"\n", "sequence {\n", "}\n"),
        repeat_to_cap("    a -> a: \"x\n", "sequence {\n", "}\n"),
    ];
    for (i, src) in inputs.into_iter().enumerate() {
        let r = bounded(src);
        if i < 4 {
            assert!(r.diagnostics.len() <= MAX_DIAGNOSTICS, "input {i}");
        }
        assert!(!r.valid, "input {i}");
    }
}
