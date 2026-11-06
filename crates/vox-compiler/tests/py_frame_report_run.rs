//! `vox run` of the frame golden through the Phase 325 worker with polars==2.1.0.
//!
//! Fails closed when the worker cannot run. A skip or `#[ignore]` is not a pass.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn vox_bin() -> PathBuf {
    let path = repo_root().join("target/debug/vox");
    assert!(
        path.is_file(),
        "vox debug binary missing at {}; py_frame_report_run fails closed",
        path.display()
    );
    path
}

fn output_text(output: &std::process::Output) -> String {
    format!(
        "status: {:?}\n--- stdout ---\n{}\n--- stderr ---\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn write_vox_toml(project: &Path) {
    fs::write(
        project.join("Vox.toml"),
        "\
[package]
name = \"py_frame_report\"

[python]
version = \"3.12\"
packages = [\"numpy==2.5.4\", \"pyarrow==26.0.1\", \"polars==2.1.0\"]
",
    )
    .expect("Vox.toml");
}

fn sync_python(project: &Path, vox: &Path) {
    let sync = Command::new(vox)
        .args(["py", "sync"])
        .current_dir(project)
        .env("UV_PYTHON", "3.12")
        .output()
        .expect("spawn vox py sync");
    assert!(
        sync.status.success(),
        "vox py sync failed:\n{}",
        output_text(&sync)
    );

    let pyproject =
        fs::read_to_string(project.join(".vox/python/pyproject.toml")).expect("pyproject.toml");
    assert!(
        pyproject.contains("polars==2.1.0"),
        "temp project must pin polars==2.1.0, got:\n{pyproject}"
    );
    assert!(
        pyproject.contains("numpy==2.5.4"),
        "temp project must pin numpy==2.5.4, got:\n{pyproject}"
    );
    assert!(
        pyproject.contains("pyarrow==26.0.1"),
        "temp project must pin pyarrow==26.0.1, got:\n{pyproject}"
    );

    let venv_python = project.join(".vox/python/.venv/bin/python");
    assert!(
        venv_python.is_file(),
        "uv venv python missing at {}",
        venv_python.display()
    );
    let version = Command::new(&venv_python)
        .args([
            "-c",
            "import sys; print(f'{sys.version_info[0]}.{sys.version_info[1]}')",
        ])
        .output()
        .expect("spawn venv python");
    let version_text = String::from_utf8_lossy(&version.stdout);
    assert!(
        version.status.success() && version_text.trim() == "3.12",
        "worker must be uv CPython 3.12, got {version_text:?}\n{}",
        output_text(&version)
    );
}

fn copy_fixture_csv(project: &Path) {
    let fixtures = project.join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures dir");
    let src = repo_root().join("examples/golden/fixtures/py_frame_report.csv");
    fs::copy(&src, fixtures.join("py_frame_report.csv")).expect("copy csv fixture");
}

#[test]
fn py_frame_report_run() {
    let vox = vox_bin();
    let root = tempfile::tempdir().expect("tempdir");
    let project = root.path();
    write_vox_toml(project);
    copy_fixture_csv(project);

    let golden = repo_root().join("examples/golden/py_frame_report.vox");
    let script = project.join("py_frame_report.vox");
    fs::copy(&golden, &script).expect("copy golden");
    let source = fs::read_to_string(&golden).expect("read golden");
    assert!(
        source.contains("assert(rows.num_rows() is 2)"),
        "golden must assert num_rows is 2"
    );
    assert!(
        source.contains("assert(rows.column(\"revenue\") is [10, 5])"),
        "golden must assert revenue is [10, 5]"
    );

    sync_python(project, &vox);

    let run = Command::new(&vox)
        .args(["run", script.to_str().expect("script path")])
        .current_dir(project)
        .env("VOX_BRIDGE_DEBUG_OPS", "1")
        .output()
        .expect("spawn vox run");
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        run.status.success(),
        "vox run failed:\n{}",
        output_text(&run)
    );

    let execute_plan_lines = log
        .lines()
        .filter(|line| line.contains("execute_plan"))
        .count();
    assert_eq!(
        execute_plan_lines, 1,
        "expected one execute_plan line in:\n{log}"
    );
}

#[test]
fn frame_null_csv_is_py_plan() {
    let vox = vox_bin();
    let root = tempfile::tempdir().expect("tempdir");
    let project = root.path();
    write_vox_toml(project);

    let fixtures = project.join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures dir");
    fs::write(
        fixtures.join("null_amount.csv"),
        "month,region,amount\n2024-01,west,\n",
    )
    .expect("null csv");

    fs::write(
        project.join("null_frame.vox"),
        r#"import std.frame

@uses(python, fs)
fn main() {
    let rows = frame.scan_csv("fixtures/null_amount.csv").collect().unwrap()
    assert(rows.num_rows() is 1)
}
"#,
    )
    .expect("null_frame.vox");

    sync_python(project, &vox);

    let run = Command::new(&vox)
        .args(["run", "null_frame.vox"])
        .current_dir(project)
        .output()
        .expect("spawn vox run");
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        !run.status.success(),
        "null cell must fail closed:\n{}",
        output_text(&run)
    );
    assert!(
        log.contains("py-plan:"),
        "null cell must report py-plan:, got:\n{log}"
    );
    assert!(
        log.contains("amount"),
        "null cell must name the column, got:\n{log}"
    );
    assert!(
        log.contains("scan_csv"),
        "null cell must name the step scan_csv, got:\n{log}"
    );
}

#[test]
fn frame_missing_column_runtime_is_py_plan() {
    let vox = vox_bin();
    let root = tempfile::tempdir().expect("tempdir");
    let project = root.path();
    write_vox_toml(project);
    copy_fixture_csv(project);

    fs::write(
        project.join("missing_col.vox"),
        r#"import std.frame

@uses(python, fs)
fn main() {
    let path = "fixtures/py_frame_report.csv"
    let rows = frame.scan_csv(path).filter(frame.col("no_such_col").eq("x")).collect().unwrap()
    assert(rows.num_rows() is 0)
}
"#,
    )
    .expect("missing_col.vox");

    sync_python(project, &vox);

    let run = Command::new(&vox)
        .args(["run", "missing_col.vox"])
        .current_dir(project)
        .output()
        .expect("spawn vox run");
    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        !run.status.success(),
        "missing runtime column must fail closed:\n{}",
        output_text(&run)
    );
    assert!(
        log.contains("py-plan:"),
        "missing runtime column must report py-plan:, got:\n{log}"
    );
}
