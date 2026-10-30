//! DGM-03: every independent mistake is reported exactly once, with a stable code and an exact
//! span. Each `tests/fixtures/broken/*.vdg` states its expected codes on the first line as
//! `// expect: code, code`; the harness compares the sorted code lists exactly.

use std::fs;
use std::path::PathBuf;

use vox_diagram::check;
use vox_diagram::diag::Diagnostic;

fn broken_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/broken")
}

fn fixtures() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let entries = fs::read_dir(broken_dir()).unwrap_or_else(|e| panic!("broken dir: {e}"));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("vdg") {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let name = entry.file_name().to_string_lossy().into_owned();
        out.push((name, src));
    }
    out.sort();
    out
}

fn expected(name: &str, src: &str) -> Vec<String> {
    let first = src.lines().next().unwrap_or_default();
    let Some(list) = first.strip_prefix("// expect:") else {
        panic!("{name}: first line must be `// expect: <codes>`");
    };
    let mut codes: Vec<String> = list
        .split(',')
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    codes.sort();
    codes
}

fn assert_spans_in_range(name: &str, src: &str, diags: &[Diagnostic]) {
    for d in diags {
        let s = &d.span;
        assert!(
            s.start <= s.end && s.end as usize <= src.len(),
            "{name}: {d:?}"
        );
        assert!(src.is_char_boundary(s.start as usize), "{name}: {d:?}");
        assert!(src.is_char_boundary(s.end as usize), "{name}: {d:?}");
        assert!(
            s.line >= 1 && s.col >= 1 && s.end_line >= 1 && s.end_col >= 1,
            "{name}: {d:?}"
        );
    }
}

#[test]
fn every_broken_fixture_emits_exactly_its_expected_codes() {
    let all = fixtures();
    assert!(!all.is_empty(), "no broken fixtures found");
    for (name, src) in &all {
        let report = check(src);
        let mut got: Vec<String> = report
            .diagnostics
            .iter()
            .map(|d| d.code.to_string())
            .collect();
        got.sort();
        assert_eq!(
            got,
            expected(name, src),
            "{name}: {:#?}",
            report.diagnostics
        );
        assert_spans_in_range(name, src, &report.diagnostics);
        let sorted = report
            .diagnostics
            .windows(2)
            .all(|w| w[0].span.start <= w[1].span.start);
        assert!(sorted, "{name}: diagnostics not sorted by span.start");
    }
}

#[test]
fn declare_then_diagnose_keeps_a_broken_declaration() {
    let src = fixtures()
        .into_iter()
        .find(|(n, _)| n.starts_with("01-"))
        .map(|(_, s)| s)
        .unwrap_or_default();
    let report = check(&src);
    let undeclared: Vec<_> = report
        .diagnostics
        .iter()
        .filter(|d| d.code == "diagram/undeclared-node")
        .collect();
    assert_eq!(
        undeclared.len(),
        1,
        "only `c` is undeclared, not the broken `a`"
    );
    assert_eq!(undeclared[0].span.line, 6);
}

fn one(src: &str) -> Vec<Diagnostic> {
    check(src).diagnostics
}

#[test]
fn diagnostics_all_errors_six_independent() {
    let src = fixtures()
        .into_iter()
        .find(|(n, _)| n.starts_with("02-"))
        .map(|(_, s)| s)
        .unwrap_or_default();
    let report = check(&src);
    assert_eq!(report.diagnostics.len(), 6, "{:#?}", report.diagnostics);
    let lines: Vec<u32> = report.diagnostics.iter().map(|d| d.span.line).collect();
    assert_eq!(
        lines,
        [3, 4, 5, 6, 7, 8],
        "one diagnostic per mistaken line"
    );
    assert!(report.ir.is_none() && !report.valid);
    insta::assert_json_snapshot!("six_independent_mistakes", report.diagnostics);
}

#[test]
fn codes_registry_covers_emitted_codes() {
    for (name, src) in fixtures() {
        for d in check(&src).diagnostics {
            assert!(
                vox_diagram::codes::ALL_CODES.contains(&d.code),
                "{name}: {} is not in ALL_CODES",
                d.code
            );
        }
    }
}

#[test]
fn codes_golden_is_subset() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/codes.golden.txt");
    let golden = fs::read_to_string(path).unwrap_or_else(|e| panic!("codes.golden.txt: {e}"));
    let mut seen = 0;
    for line in golden.lines().filter(|l| !l.is_empty()) {
        assert!(line.starts_with("diagram/"), "{line}");
        assert!(
            vox_diagram::codes::ALL_CODES.contains(&line),
            "{line} was renamed or removed; codes are only ever added"
        );
        seen += 1;
    }
    assert!(seen >= 22, "golden lists only {seen} codes");
}

#[test]
fn shadowed_id_is_a_warning_and_the_report_stays_valid() {
    let r = check("graph {\n node x: \"X\"\n container c: \"C\" {\n  node x: \"I\"\n }\n}\n");
    assert!(r.valid && r.ir.is_some());
    assert_eq!(r.diagnostics.len(), 1);
    assert_eq!(r.diagnostics[0].code, "diagram/shadowed-id");
    assert_eq!(
        r.diagnostics[0].severity,
        vox_diagram::diag::Severity::Warning
    );
}

#[test]
fn duplicate_id_points_back_at_the_first_declaration() {
    let ds = one("graph {\n node a: \"A\"\n node a: \"B\"\n}\n");
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].span.line, 3);
    assert_eq!(ds[0].related.len(), 1);
    assert_eq!(ds[0].related[0].span.line, 2);
}

#[test]
fn invalid_direction_help_names_the_closest_word() {
    let ds = one("graph {\n direction: rigth\n}\n");
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].code, "diagram/invalid-direction");
    assert!(ds[0].help.as_deref().is_some_and(|h| h.contains("`right`")));
}

#[test]
fn unclosed_braces_point_at_their_opening_brace() {
    let src = "graph {\n container c: \"C\" {\n  node x: \"X\"\n";
    let ds = one(src);
    let spans: Vec<(u32, u32)> = ds.iter().map(|d| (d.span.line, d.span.col)).collect();
    assert_eq!(spans, [(1, 7), (2, 19)]);
}

#[test]
fn echoed_user_text_is_cut_at_40_chars() {
    let q = "Q".repeat(100);
    let sources = [
        format!("graph {{\n node a: \"A\"\n a -> {q}\n}}\n"),
        format!("graph {{\n direction: {q}\n}}\n"),
        format!("graph {{\n node a: \"A\" {{ shape: {q} }}\n}}\n"),
        format!("graph {{\n node a: \"A\" {{ {q}: red }}\n}}\n"),
        format!("{q} {{\n}}\n"),
        format!(
            "graph {{\n container {q}: \"C\" {{\n node x: \"X\"\n }}\n node a: \"A\"\n a -> {q}\n}}\n"
        ),
        format!("graph {{\n node a: \"A\"\n a -> {q}.{q}\n}}\n"),
        format!("graph {{\n {q}\n}}\n"),
        format!("graph {{\n node a: {q} {q}\n}}\n"),
    ];
    let needle = "Q".repeat(41);
    for src in &sources {
        let ds = one(src);
        assert!(!ds.is_empty(), "{src}");
        for d in ds {
            let help = d.help.clone().unwrap_or_default();
            assert!(!d.message.contains(&needle), "{}", d.message);
            assert!(!help.contains(&needle), "{help}");
        }
    }
}
