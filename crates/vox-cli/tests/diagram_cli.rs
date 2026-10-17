//! `vox diagram` binary tests: every subcommand over files and stdin, exit codes and `--json` parity
//! with the `vox_validate_diagram` envelope.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use vox_doc_pipeline::diagram::{ValidateOpts, validate};

const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";
const VALID: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";
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
