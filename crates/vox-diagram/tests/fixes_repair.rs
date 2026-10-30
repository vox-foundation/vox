//! DGM-04: fix-bearing diagnostics flow check -> apply_fixes -> valid, every mechanically
//! fixable code carries the exact documented fix, and the seeded repairable set repairs to
//! zero errors.

use std::fs;
use std::path::PathBuf;

use vox_diagram::diag::Severity;
use vox_diagram::{apply_fixes, check, codes, repair};

fn fixture_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fixtures(dir: &str) -> Vec<(String, String)> {
    let entries = fs::read_dir(fixture_dir(dir)).unwrap_or_else(|e| panic!("{dir}: {e}"));
    let mut out: Vec<(String, String)> = entries
        .flatten()
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("vdg"))
        .map(|e| {
            let src =
                fs::read_to_string(e.path()).unwrap_or_else(|x| panic!("{:?}: {x}", e.path()));
            let stem = e
                .path()
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned());
            (stem.unwrap_or_default(), src)
        })
        .collect();
    out.sort();
    out
}

/// `(original text, replacement)` for every fix of every diagnostic with `code`.
fn fixes_for(src: &str, code: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for d in check(src).diagnostics.iter().filter(|d| d.code == code) {
        for f in &d.fixes {
            let orig = src.get(f.span.start as usize..f.span.end as usize);
            out.push((
                orig.unwrap_or("<bad span>").to_string(),
                f.replacement.clone(),
            ));
        }
    }
    out
}

fn pair(a: &str, b: &str) -> (String, String) {
    (a.to_string(), b.to_string())
}

#[test]
fn fixes_repair_unclosed_brace_end_to_end() {
    let src = "graph {\n    container c: \"C\" {\n        node x: \"X\"\n";
    let report = check(src);
    assert!(!report.valid);
    assert!(report.diagnostics.iter().all(|d| d.fixes.len() == 1));
    let out = apply_fixes(src, &report.diagnostics);
    assert_eq!((out.applied, out.skipped), (2, 0));
    assert!(check(&out.text).valid, "{}", out.text);
    let r = repair(src, 5);
    assert_eq!(r.rounds, 1);
    assert!(r.report.valid);
    assert_eq!(r.text, out.text);
}

#[test]
fn fixes_repair_seeded_set() {
    let set = fixtures("repairable");
    assert!(set.len() >= 10, "seeded set shrank to {}", set.len());
    for (name, src) in &set {
        assert!(!check(src).valid, "{name} is not broken");
        let r = repair(src, 5);
        assert!(
            r.report.valid,
            "{name} -> {}\n{:?}",
            r.text, r.report.diagnostics
        );
        assert_eq!(
            repair(&r.text, 5).rounds,
            0,
            "{name}: repaired text is not stable"
        );
        insta::assert_snapshot!(name.as_str(), r.text);
    }
}

#[test]
fn lexer_and_parser_fixes_have_the_documented_shape() {
    let unterminated = fixes_for(
        "graph {\n    node a: \"A  \n}\n",
        codes::UNTERMINATED_STRING,
    );
    assert_eq!(unterminated, [pair("", "\"")]);
    let escape = fixes_for("graph {\n    node a: \"x\\ty\"\n}\n", codes::INVALID_ESCAPE);
    assert_eq!(escape, [pair("\\", "\\\\")]);
    let forbidden = fixes_for(
        "graph {\n    node a: \"x\u{202e}y\"\n}\n",
        codes::FORBIDDEN_CHAR,
    );
    assert_eq!(forbidden, [pair("\u{202e}", "")]);
    let unquoted = fixes_for(
        "graph {\n    node a: Say \"hi\\\"\n}\n",
        codes::EXPECTED_QUOTED_LABEL,
    );
    assert_eq!(unquoted.len(), 1);
    let single = fixes_for(
        "graph {\n    node a: 'it \"x\" \\'\n}\n",
        codes::EXPECTED_QUOTED_LABEL,
    );
    assert_eq!(single, [pair("'it \"x\" \\'", "\"it \\\"x\\\" \\\\\"")]);
    let plain = fixes_for(
        "graph {\n    node a: Alpha service\n}\n",
        codes::EXPECTED_QUOTED_LABEL,
    );
    assert_eq!(plain, [pair("Alpha service", "\"Alpha service\"")]);
    let empty = fixes_for("graph {\n    node a: \"\"\n}\n", codes::EMPTY_LABEL);
    assert_eq!(empty, [pair("\"\"", "\"a\"")]);
    let container = fixes_for(
        "graph {\n    container c: \"\" {\n    }\n}\n",
        codes::EMPTY_LABEL,
    );
    assert_eq!(container, [pair("\"\"", "\"c\"")]);
    let edge = "graph {\n    node a: \"A\"\n    a -> a : \"\"\n}\n";
    assert_eq!(fixes_for(edge, codes::EMPTY_LABEL), [pair(" : \"\"", "")]);
    let shape = "graph {\n    node a: \"A\" { shape: Cylinder }\n}\n";
    assert_eq!(
        fixes_for(shape, codes::INVALID_SHAPE),
        [pair("Cylinder", "cylinder")]
    );
    let hex = "graph {\n    node a: \"A\" { shape: hexagon }\n}\n";
    assert!(fixes_for(hex, codes::INVALID_SHAPE).is_empty());
    let diags = check(hex).diagnostics;
    assert_eq!(diags.len(), 1);
}

#[test]
fn fixes_only_on_errors() {
    let mut sources: Vec<String> = Vec::new();
    for dir in ["repairable", "broken", "valid"] {
        sources.extend(fixtures(dir).into_iter().map(|(_, s)| s));
    }
    sources.push(
        "graph {\n    node a: \"A\"\n    container c: \"C\" {\n        node a: \"A2\"\n    }\n}\n"
            .into(),
    );
    let mut with_fixes = 0;
    for src in &sources {
        for d in check(src).diagnostics {
            if d.severity == Severity::Warning {
                assert!(d.fixes.is_empty(), "warning with fixes: {d:?}");
            }
            for f in &d.fixes {
                let in_range = f.span.start <= f.span.end && f.span.end as usize <= src.len();
                assert!(in_range, "{d:?}");
                assert!(src.is_char_boundary(f.span.start as usize), "{d:?}");
                assert!(src.is_char_boundary(f.span.end as usize), "{d:?}");
            }
            with_fixes += usize::from(!d.fixes.is_empty());
        }
    }
    assert!(with_fixes >= 10, "only {with_fixes} diagnostics had fixes");
}

#[test]
fn non_mechanical_codes_have_no_fixes() {
    let mut sources: Vec<String> = Vec::new();
    for dir in ["repairable", "broken"] {
        sources.extend(fixtures(dir).into_iter().map(|(_, s)| s));
    }
    sources.push("graph {\n    node a: \"A\"\n    container c: \"C\" {\n        node x: \"X\"\n    }\n    a -> c\n    a -> a.x\n}\n".into());
    sources.push(format!(
        "graph {{\n    node a: \"{}\"\n}}\n",
        "x".repeat(201)
    ));
    let nested = "container c: \"C\" {\n".repeat(5);
    sources.push(format!("graph {{\n{nested}{}}}\n", "}\n".repeat(5)));
    let nodes: String = (0..501)
        .map(|i| format!("    node n{i}: \"N\"\n"))
        .collect();
    sources.push(format!("graph {{\n{nodes}}}\n"));
    let non_mechanical = [
        codes::DUPLICATE_ID,
        codes::CONTAINER_ENDPOINT,
        codes::NOT_A_CONTAINER,
        codes::RESERVED_WORD,
        codes::INVALID_DIRECTION,
        codes::DUPLICATE_DIRECTION,
        codes::DIRECTION_IN_CONTAINER,
        codes::UNKNOWN_ATTRIBUTE,
        codes::UNKNOWN_DIAGRAM_KIND,
        codes::INVALID_IDENTIFIER,
        codes::UNEXPECTED_TOKEN,
        codes::TRAILING_CONTENT,
        codes::LIMIT_SOURCE,
        codes::LIMIT_NODES,
        codes::LIMIT_EDGES,
        codes::LIMIT_DEPTH,
        codes::LIMIT_LABEL,
        codes::LIMIT_DIAGNOSTICS,
    ];
    let mut seen = std::collections::BTreeSet::new();
    for src in &sources {
        for d in check(src).diagnostics {
            if non_mechanical.contains(&d.code) {
                assert!(d.fixes.is_empty(), "{} must not carry fixes: {d:?}", d.code);
                seen.insert(d.code);
            }
        }
    }
    for code in [
        codes::DUPLICATE_ID,
        codes::CONTAINER_ENDPOINT,
        codes::NOT_A_CONTAINER,
        codes::RESERVED_WORD,
        codes::INVALID_DIRECTION,
        codes::UNKNOWN_ATTRIBUTE,
        codes::TRAILING_CONTENT,
        codes::LIMIT_NODES,
        codes::LIMIT_DEPTH,
        codes::LIMIT_LABEL,
    ] {
        assert!(seen.contains(code), "{code} was never exercised");
    }
}
