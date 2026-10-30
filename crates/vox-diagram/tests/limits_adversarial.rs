//! DGM-06: every cap accepts N and rejects N+1 with one span-precise `diagram/limit-*` error,
//! and hostile input terminates in bounded time without panicking. The 10 s receive timeout is
//! a loose hang detector, never a performance assertion.

use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use vox_diagram::check;
use vox_diagram::diag::CheckReport;
use vox_diagram::limits::{MAX_DIAGNOSTICS, MAX_SOURCE_BYTES};

/// Run `check` on a fresh thread with the default stack and wait at most 10 s for it.
fn bounded(src: String) -> CheckReport {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send((check(&src), src.len()));
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok((report, len)) => {
            assert_in_range(&report, len);
            report
        }
        Err(e) => panic!("check did not finish (hang or crash): {e}"),
    }
}

fn assert_in_range(report: &CheckReport, len: usize) {
    for d in &report.diagnostics {
        assert!(
            d.span.start <= d.span.end && d.span.end as usize <= len,
            "{d:?}"
        );
    }
}

fn codes_of(report: &CheckReport, code: &str) -> usize {
    report.diagnostics.iter().filter(|d| d.code == code).count()
}

fn graph_with_nodes(n: usize) -> String {
    let mut s = String::from("graph {\n");
    for i in 0..n {
        s.push_str(&format!("node n{i}: \"N\"\n"));
    }
    s.push_str("}\n");
    s
}

fn graph_with_edges(n: usize) -> String {
    let mut s = String::from("graph {\nnode a: \"A\"\nnode b: \"B\"\n");
    for _ in 0..n {
        s.push_str("a -> b\n");
    }
    s.push_str("}\n");
    s
}

fn nested(depth: usize) -> String {
    let mut s = String::from("graph {\n");
    for i in 0..depth {
        s.push_str(&format!("container c{i}: \"C\" {{\n"));
    }
    s.push_str("node x: \"X\"\n");
    s.push_str(&"}\n".repeat(depth));
    s.push_str("}\n");
    s
}

#[test]
fn nodes_500_accepted_501_rejected() {
    assert!(bounded(graph_with_nodes(500)).valid);
    let r = bounded(graph_with_nodes(501));
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
    let d = &r.diagnostics[0];
    assert_eq!(d.code, "diagram/limit-nodes");
    assert_eq!(d.span.line, 502, "node #501 is on line 502");
    assert!(d.message.contains("501"), "{}", d.message);
    assert!(r.ir.is_none());
}

#[test]
fn edges_1000_accepted_1001_rejected() {
    assert!(bounded(graph_with_edges(1000)).valid);
    let r = bounded(graph_with_edges(1001));
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
    assert_eq!(r.diagnostics[0].code, "diagram/limit-edges");
    assert_eq!(
        r.diagnostics[0].span.line, 1004,
        "edge #1001 is on line 1004"
    );
    assert!(r.diagnostics[0].message.contains("1001"));
}

#[test]
fn depth_4_accepted_5_rejected_on_the_fifth_header() {
    assert!(bounded(nested(4)).valid);
    let r = bounded(nested(5));
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
    assert_eq!(r.diagnostics[0].code, "diagram/limit-depth");
    assert_eq!(r.diagnostics[0].span.line, 6);
}

#[test]
fn four_thousand_nested_headers_give_one_depth_error_and_no_overflow() {
    let src = format!("graph {{\n{}", "container a:\"a\"{".repeat(4_000));
    assert!(src.len() <= MAX_SOURCE_BYTES);
    let r = bounded(src);
    assert_eq!(
        codes_of(&r, "diagram/limit-depth"),
        1,
        "{:?}",
        r.diagnostics
    );
    assert!(!r.valid);
}

#[test]
fn label_200_chars_accepted_201_rejected() {
    let ok = format!("graph {{\n node a: \"{}\"\n}}\n", "é".repeat(200));
    assert!(bounded(ok).valid);
    let bad = format!("graph {{\n node a: \"{}\"\n}}\n", "é".repeat(201));
    let r = bounded(bad);
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
    let d = &r.diagnostics[0];
    assert_eq!(d.code, "diagram/limit-label");
    assert_eq!(d.span.line, 2);
    assert_eq!(
        (d.span.end - d.span.start) as usize,
        201 * 2 + 2,
        "the string token"
    );
}

#[test]
fn source_cap_is_exact_and_lands_on_a_char_boundary() {
    let head = "graph {\n node a: \"A\"\n";
    let tail = "}\n";
    let pad = MAX_SOURCE_BYTES - head.len() - tail.len();
    let exact = format!("{head}{}{tail}", " ".repeat(pad));
    assert_eq!(exact.len(), MAX_SOURCE_BYTES);
    assert!(bounded(exact).valid);

    // The 2-byte `é` straddles byte 65_536: the cut moves back to 65_535.
    let straddle = format!("{}é", " ".repeat(MAX_SOURCE_BYTES - 1));
    assert_eq!(straddle.len(), MAX_SOURCE_BYTES + 1);
    let r = bounded(straddle.clone());
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].code, "diagram/limit-source");
    assert_eq!(r.diagnostics[0].span.start as usize, MAX_SOURCE_BYTES - 1);
    assert!(straddle.is_char_boundary(r.diagnostics[0].span.start as usize));
}

#[test]
fn container_endpoint_is_rejected() {
    let src = "graph {\n node c: \"C\"\n container b: \"B\" {\n  node g: \"G\"\n }\n c -> b\n}\n";
    let r = bounded(src.to_string());
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].code, "diagram/container-endpoint");
}

/// The input as-is and wrapped in a `graph {` block, so both the header path and the
/// statement loop see it.
fn adversarial(body: &str) {
    let r = bounded(body.to_string());
    assert!(!r.valid, "raw hostile input must be invalid");
    assert!(
        r.diagnostics.len() <= MAX_DIAGNOSTICS,
        "{}",
        r.diagnostics.len()
    );
    let wrapped = format!("graph {{\n{body}");
    if wrapped.len() <= MAX_SOURCE_BYTES {
        let r = bounded(wrapped);
        assert!(
            r.diagnostics.len() <= MAX_DIAGNOSTICS,
            "{}",
            r.diagnostics.len()
        );
    }
}

#[test]
fn adversarial_open_braces() {
    adversarial(&"{".repeat(65_000));
}

#[test]
fn adversarial_open_brackets() {
    adversarial(&"[".repeat(65_000));
}

#[test]
fn adversarial_quote_soup() {
    adversarial(&"\"".repeat(65_000));
}

#[test]
fn adversarial_huge_dotted_reference() {
    let dotted = vec!["a"; 30_000].join(".");
    let src = format!("graph {{\n node a: \"A\"\n {dotted} -> a\n}}\n");
    let r = bounded(src);
    assert!(!r.valid);
    assert!(r.diagnostics.len() <= MAX_DIAGNOSTICS);
}

#[test]
fn adversarial_keyword_soup() {
    // 20,000 repetitions would exceed the 64 KiB source cap, which would then be the only
    // thing exercised; use the most that fits.
    adversarial(&"node ".repeat(13_000));
    adversarial(&"container ".repeat(6_500));
    adversarial(&"-> ".repeat(21_000));
}

#[test]
fn adversarial_mermaid_tokens() {
    // Link-label scans (`-- text -->`, `-->|text|`) and bracket forms must stay linear when
    // nothing ever closes them, and fan-out, chains and nested frames must stay capped.
    adversarial(&"-- ".repeat(21_000));
    adversarial(&"-->| ".repeat(12_000));
    adversarial(&"a[ ".repeat(21_000));
    adversarial(&"a[(".repeat(21_000));
    adversarial(&"a & ".repeat(16_000));
    adversarial(&"a -> ".repeat(13_000));
    adversarial(&"a --> b ".repeat(8_000));
    adversarial(&"subgraph a\n".repeat(5_000));
    adversarial(&"%%".repeat(30_000));
    adversarial(&"style a\n".repeat(7_000));
    adversarial(&";".repeat(60_000));
    adversarial(&":::c ".repeat(12_000));
    adversarial(&"a>".repeat(30_000));
}

/// WR-06: `a>a>a>...` (an unclosed `>[`-form opener) once rescanned the line per `>`. A
/// coarse growth check, not an absolute time: 4x the input must cost well under 16x (linear
/// is about 4x, quadratic about 16x); best of three runs each to shed scheduler noise.
#[test]
fn unclosed_gt_openers_scale_linearly() {
    let best = |n: usize| {
        let src = "a>".repeat(n);
        (0..3)
            .map(|_| {
                let t = Instant::now();
                let _ = check(&src);
                t.elapsed()
            })
            .min()
            .unwrap_or_default()
    };
    let (small, large) = (best(8_000), best(32_000));
    assert!(
        large < small * 10 + Duration::from_millis(20),
        "{small:?} -> {large:?}"
    );
}

#[test]
fn adversarial_brace_pairs() {
    adversarial(&"{}".repeat(32_000));
    adversarial(&"{ node a: ".repeat(6_000));
}

#[test]
fn adversarial_control_characters_do_not_flood_the_report() {
    let src = format!("graph {{\n node a: \"{}\"\n}}\n", "\u{1}".repeat(60_000));
    let r = bounded(src);
    assert!(r.diagnostics.len() <= MAX_DIAGNOSTICS);
    assert_eq!(
        r.diagnostics.last().map(|d| d.code),
        Some("diagram/limit-diagnostics")
    );
}
