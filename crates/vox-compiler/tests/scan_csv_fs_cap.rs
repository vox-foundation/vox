//! Host `fs:ro` denial for `frame.scan_csv` before any worker request (FRM-03).

use std::path::PathBuf;
use std::time::Duration;

use vox_compiler::eval::caps::CapabilitySet;
use vox_compiler::eval::python::{PythonProject, set_python_project};
use vox_compiler::eval::{EvalError, Interpreter};

fn run_scan(
    caps: CapabilitySet,
    project_root: PathBuf,
    path_literal: &str,
) -> Result<vox_compiler::eval::value::VoxValue, EvalError> {
    let src = format!(
        r#"import std.frame
pub fn main() {{ return frame.scan_csv("{path_literal}") }}
"#
    );
    let tokens = vox_compiler::lexer::lex(&src);
    let module = vox_compiler::parser::descent::parse(tokens).expect("parse");
    let lowered = vox_compiler::hir::lower::lower_module(&module);
    let mut interp = Interpreter::new(1_000_000);
    interp.caps = caps;
    set_python_project(
        &mut interp,
        PythonProject {
            root: project_root,
            max_fetch_mb: 8,
            timeout: Duration::from_secs(5),
        },
    );
    interp.run_module(&lowered)?;
    interp.call("main", vec![])
}

#[test]
fn scan_csv_outside_fs_ro_never_calls_worker() {
    let allowed = tempfile::tempdir().expect("allowed root");
    let outside = tempfile::tempdir().expect("outside root");
    let project = tempfile::tempdir().expect("python project");

    // Point the worker command at a non-interpreter path. Host::start would
    // resolve `.vox/python/.venv/bin/python` here; a spawn attempt would not
    // produce CapabilityDenied for fs.scan_csv.
    let venv_bin = project
        .path()
        .join(".vox")
        .join("python")
        .join(".venv")
        .join("bin");
    std::fs::create_dir_all(&venv_bin).expect("venv bin");
    let worker_cmd = venv_bin.join("python");
    assert!(
        !worker_cmd.exists(),
        "worker command must start absent: {}",
        worker_cmd.display()
    );
    // Empty file so venv_python selects this path (not python3 on PATH).
    std::fs::write(&worker_cmd, []).expect("stub worker path");

    let caps = CapabilitySet::parse(&format!("py:allow,fs:ro={}", allowed.path().display()))
        .expect("scoped caps");

    let outside_csv = outside.path().join("leak.csv");
    std::fs::write(&outside_csv, "a,b\n1,2\n").expect("outside csv");

    let outside_result = run_scan(
        caps.clone(),
        project.path().to_path_buf(),
        &outside_csv.display().to_string(),
    );
    match outside_result {
        Err(EvalError::CapabilityDenied { ns, method }) => {
            assert_eq!(ns, "fs");
            assert_eq!(method, "scan_csv");
        }
        other => panic!("outside path must be CapabilityDenied, got {other:?}"),
    }

    let parent_seg = format!("{}/../outside.csv", allowed.path().display());
    let parent_result = run_scan(caps, project.path().to_path_buf(), &parent_seg);
    match parent_result {
        Err(EvalError::CapabilityDenied { ns, method }) => {
            assert_eq!(ns, "fs");
            assert_eq!(method, "scan_csv");
        }
        other => panic!("parent-dir path must be CapabilityDenied, got {other:?}"),
    }
}
