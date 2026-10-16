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
