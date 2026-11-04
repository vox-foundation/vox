//! `vox run` of the z-score golden through the Phase 325 worker.
//!
//! Fails closed when the worker cannot run. A skip or `#[ignore]` is not a pass.

use std::fs;
use std::path::PathBuf;
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
        "vox debug binary missing at {}; py_array_stats_run fails closed",
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

/// A wire method is an RPC method name, not an op inside the plan JSON.
fn wire_method_is_mean_or_std(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed == "mean" || trimmed == "std" {
        return true;
    }
    const NEEDLES: &[&str] = &[
        "\"method\":\"mean\"",
        "\"method\": \"mean\"",
        "\"method\":\"std\"",
        "\"method\": \"std\"",
        "method=mean",
        "method=std",
    ];
    NEEDLES.iter().any(|needle| line.contains(needle))
}

#[test]
fn py_array_stats_run() {
    let vox = vox_bin();
    let root = tempfile::tempdir().expect("tempdir");
    let project = root.path();
    fs::write(
        project.join("Vox.toml"),
        "\
[package]
name = \"py_array_stats\"

[python]
version = \"3.12\"
packages = [\"numpy>=2.1\", \"pyarrow>=17\"]
",
    )
    .expect("Vox.toml");

    let golden = repo_root().join("examples/golden/py_array_stats.vox");
    let script = project.join("py_array_stats.vox");
    fs::copy(&golden, &script).expect("copy golden");
    let source = fs::read_to_string(&golden).expect("read golden");
    assert!(
        source.contains("assert(z is [-1.0, 1.0])"),
        "golden assert must accept [-1.0, 1.0]"
    );
    assert!(
        source.contains("9223372036854775807"),
        "golden must assert i64::MAX with the max i64 literal"
    );
    assert!(
        !source.contains("9223372036854775808"),
        "golden must not spell the minimum i64 magnitude"
    );
    assert!(source.contains("isnt"), "NaN must be compared with isnt");

    let sync = Command::new(&vox)
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
        pyproject.contains("requires-python = \">=3.12\""),
        "requires-python must include an operator uv accepts, got:\n{pyproject}"
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
        execute_plan_lines, 3,
        "expected three execute_plan lines in:\n{log}"
    );
    assert!(
        !log.lines().any(wire_method_is_mean_or_std),
        "debug log has a wire method named mean or std:\n{log}"
    );
}
