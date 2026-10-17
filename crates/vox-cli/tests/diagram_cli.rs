//! `vox diagram` binary tests: every subcommand over files and stdin, exit codes and `--json` parity
//! with the `vox_validate_diagram` envelope.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use vox_doc_pipeline::diagram::{Emit, ValidateOpts, validate};

const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";
const VALID: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";
const SEQUENCE: &str =
    include_str!("../../vox-diagram/tests/fixtures/valid/sequence_all_constructs.vdg");
const FLOWCHART: &str =
    include_str!("../../vox-diagram/tests/fixtures/import-cases/00-minimal.mmd");
const GANTT: &str = "gantt\n    title Plan\n";
const NON_CANONICAL: &str = "graph {\nnode a:\"A\"\n   node b: \"B\"\n a -> b\n}\n";
const SHADOWED: &str = include_str!("../../vox-diagram/tests/fixtures/broken/18-shadowed-id.vdg");

struct Out {
    code: i32,
    stdout: String,
    stderr: String,
}

fn vox(args: &[&str], stdin: Option<&str>) -> Out {
    let mut child = Command::new(env!("CARGO_BIN_EXE_vox"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("vox should spawn");
    let mut pipe = child.stdin.take().expect("stdin pipe");
    if let Some(s) = stdin {
        pipe.write_all(s.as_bytes()).expect("write stdin");
    }
    drop(pipe);
    let o = child.wait_with_output().expect("vox should run");
    Out {
        code: o.status.code().expect("exit code"),
        stdout: String::from_utf8(o.stdout).expect("utf8 stdout"),
        stderr: String::from_utf8(o.stderr).expect("utf8 stderr"),
    }
}

fn write_tmp(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).expect("write fixture");
    p.display().to_string()
}

fn envelope(src: &str, opts: ValidateOpts) -> serde_json::Value {
    serde_json::to_value(validate(src, opts)).expect("envelope serializes")
}

#[test]
fn check_file_reports_every_error_and_exits_1() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_tmp(dir.path(), "broken.vdg", BROKEN);
    let o = vox(&["diagram", "check", &path], None);
    let want: Vec<String> = validate(BROKEN, ValidateOpts::default())
        .diagnostics
        .iter()
        .map(|d| {
            format!(
                "{path}:{}:{}: {}: {}",
                d.span.line, d.span.col, d.code, d.message
            )
        })
        .collect();
    assert!(
        want.len() >= 2,
        "tracer source must have two errors: {want:?}"
    );
    assert_eq!(
        o.code, 1,
        "exit code of a broken diagram; stderr: {}",
        o.stderr
    );
    assert_eq!(
        o.stderr.lines().map(str::to_owned).collect::<Vec<_>>(),
        want
    );
    assert!(o.stdout.is_empty(), "stdout: {}", o.stdout);
}

#[test]
fn check_stdin_valid_exits_0_silently() {
    let o = vox(&["diagram", "check", "-"], Some(VALID));
    assert_eq!((o.code, o.stdout.as_str(), o.stderr.as_str()), (0, "", ""));
}

#[test]
fn check_warning_only_exits_0() {
    let o = vox(&["diagram", "check", "-"], Some(SHADOWED));
    assert_eq!(o.code, 0, "warnings must not fail: {}", o.stderr);
    assert!(o.stderr.starts_with("<stdin>:"), "{}", o.stderr);
    assert!(o.stderr.contains(": diagram/shadowed-id: "), "{}", o.stderr);
}

#[test]
fn check_json_prints_the_mcp_envelope() {
    let want = envelope(BROKEN, ValidateOpts::default());
    for args in [
        &["--json", "diagram", "check", "-"][..],
        &["diagram", "check", "--json", "-"][..],
    ] {
        let o = vox(args, Some(BROKEN));
        assert_eq!(o.code, 1, "{args:?}: {}", o.stderr);
        let got: serde_json::Value = serde_json::from_str(&o.stdout).expect("stdout is JSON");
        assert_eq!(got, want, "{args:?}");
    }
}

#[test]
fn missing_file_exits_2() {
    let o = vox(&["diagram", "check", "/nonexistent/x.vdg"], None);
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(o.stderr.contains("/nonexistent/x.vdg"), "{}", o.stderr);
}

#[test]
fn no_subcommand_exits_2() {
    assert_eq!(vox(&["diagram"], None).code, 2);
}

fn normalized(src: &str) -> String {
    validate(src, ValidateOpts::default())
        .normalized_source
        .expect("valid source")
}

fn with(emit: Option<Emit>, from_mermaid: bool) -> ValidateOpts {
    ValidateOpts { from_mermaid, emit }
}

#[test]
fn fmt_prints_canonical() {
    let o = vox(&["diagram", "fmt", "-"], Some(NON_CANONICAL));
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_ne!(NON_CANONICAL, normalized(NON_CANONICAL));
    assert_eq!(o.stdout, normalized(NON_CANONICAL));
}

#[test]
fn fmt_check_fails_on_non_canonical() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_tmp(dir.path(), "a.vdg", NON_CANONICAL);
    let o = vox(&["diagram", "fmt", "--check", &path], None);
    assert_eq!(o.code, 1);
    assert!(
        o.stderr.contains(&path) && o.stderr.contains("vox diagram fmt --write"),
        "{}",
        o.stderr
    );
}

#[test]
fn fmt_check_passes_on_canonical() {
    let o = vox(
        &["diagram", "fmt", "--check", "-"],
        Some(&normalized(NON_CANONICAL)),
    );
    assert_eq!((o.code, o.stdout.as_str(), o.stderr.as_str()), (0, "", ""));
}

#[test]
fn fmt_write_rewrites_then_check_passes() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_tmp(dir.path(), "a.vdg", NON_CANONICAL);
    let o = vox(&["diagram", "fmt", "--write", &path], None);
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        normalized(NON_CANONICAL)
    );
    assert_eq!(vox(&["diagram", "fmt", "--check", &path], None).code, 0);
    let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().collect();
    assert_eq!(names.len(), 1, "temp sibling left behind: {names:?}");
}

#[test]
fn fmt_write_leaves_invalid_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_tmp(dir.path(), "a.vdg", BROKEN);
    let o = vox(&["diagram", "fmt", "--write", &path], None);
    assert_eq!(o.code, 1);
    assert!(o.stderr.contains(&format!("{path}:")), "{}", o.stderr);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), BROKEN);
}

#[test]
fn fmt_write_stdin_is_usage_error() {
    assert_eq!(
        vox(&["diagram", "fmt", "--write", "-"], Some(VALID)).code,
        2
    );
}

#[test]
fn json_with_output_is_usage_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("x.svg");
    let o = vox(
        &[
            "--json",
            "diagram",
            "render",
            "-o",
            out.to_str().unwrap(),
            "-",
        ],
        Some(VALID),
    );
    assert_eq!(o.code, 2, "{}", o.stderr);
    assert!(!out.exists());
}

#[test]
fn render_emits_the_envelope_svg() {
    let want = validate(VALID, with(Some(Emit::Svg), false))
        .emitted
        .expect("svg");
    let o = vox(&["diagram", "render", "-"], Some(VALID));
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert!(o.stdout.starts_with("<svg"), "{}", o.stdout);
    assert_eq!(o.stdout, want);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("a.svg");
    let o = vox(
        &["diagram", "render", "-o", out.to_str().unwrap(), "-"],
        Some(VALID),
    );
    assert_eq!((o.code, o.stdout.as_str()), (0, ""));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), want);

    let bad = dir.path().join("b.svg");
    let o = vox(
        &["diagram", "render", "-o", bad.to_str().unwrap(), "-"],
        Some(BROKEN),
    );
    assert_eq!(o.code, 1);
    assert!(!bad.exists());
}

#[test]
fn render_failure_exits_1() {
    let mut big = String::from("graph {\n");
    for i in 0..101 {
        big.push_str(&format!("    node n{i}: \"N\"\n"));
    }
    big.push_str("}\n");
    let emit_error = validate(&big, with(Some(Emit::Svg), false))
        .emit_error
        .expect("over render cap");
    let o = vox(&["diagram", "render", "-"], Some(&big));
    assert_eq!(o.code, 1);
    assert!(o.stderr.contains(&emit_error), "{}", o.stderr);
    assert!(o.stdout.is_empty());
}

#[test]
fn export_emits_mermaid() {
    for src in [VALID, SEQUENCE] {
        let want = validate(src, with(Some(Emit::Mermaid), false))
            .emitted
            .expect("mermaid");
        let o = vox(&["diagram", "export", "-"], Some(src));
        assert_eq!((o.code, o.stdout), (0, want), "{}", o.stderr);
    }
}

#[test]
fn import_converts_mermaid() {
    let want = validate(FLOWCHART, with(None, true))
        .normalized_source
        .expect("importable");
    let o = vox(&["diagram", "import", "-"], Some(FLOWCHART));
    assert_eq!(o.code, 0, "{}", o.stderr);
    assert_eq!(o.stdout, want);
    assert_eq!(vox(&["diagram", "check", "-"], Some(&o.stdout)).code, 0);
}

#[test]
fn import_lists_unsupported_lines() {
    let u = validate(GANTT, with(None, true))
        .unsupported
        .expect("from_mermaid");
    assert!(!u.is_empty());
    let o = vox(&["diagram", "import", "-"], Some(GANTT));
    assert_eq!(o.code, 1);
    assert!(
        o.stderr.contains(&format!(
            "<stdin>:{}: unsupported: {}",
            u[0].line, u[0].construct
        )),
        "{}",
        o.stderr
    );
}

#[test]
fn json_matches_each_subcommands_opts() {
    let cases: [(&str, &str, ValidateOpts); 4] = [
        ("fmt", VALID, ValidateOpts::default()),
        ("render", VALID, with(Some(Emit::Svg), false)),
        ("export", VALID, with(Some(Emit::Mermaid), false)),
        ("import", FLOWCHART, with(None, true)),
    ];
    for (sub, src, opts) in cases {
        let o = vox(&["--json", "diagram", sub, "-"], Some(src));
        assert_eq!(o.code, 0, "{sub}: {}", o.stderr);
        let got: serde_json::Value = serde_json::from_str(&o.stdout).expect("stdout is JSON");
        assert_eq!(got, envelope(src, opts), "{sub}");
    }
}
