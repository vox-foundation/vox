use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use vox_compiler::eval::Interpreter;
use vox_compiler::eval::array_plan::{ArrayPlan, plan_worker_calls};
use vox_compiler::eval::caps::CapabilitySet;
use vox_compiler::eval::plot_plan::{PlotPlan, plot_worker_calls};
use vox_compiler::eval::python::{PythonProject, set_python_project};
use vox_compiler::eval::value::VoxValue;
use vox_compiler::hir::lower::lower_module;
use vox_compiler::lexer::lex;
use vox_compiler::parser::parse_script;
use vox_compiler::typeck::{TypeckSeverity, typecheck_hir_module_with_path};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates")
        .parent()
        .expect("repo")
        .to_path_buf()
}

fn eval_script(
    src: &str,
    caps: CapabilitySet,
    project: Option<PythonProject>,
) -> Result<(VoxValue, Interpreter), String> {
    let module = parse_script(lex(src)).map_err(|e| format!("parse: {e:?}"))?;
    let hir = lower_module(&module);
    let mut interp = Interpreter::new(1_000_000);
    interp.caps = caps;
    if let Some(proj) = project {
        set_python_project(&mut interp, proj);
    }
    interp.run_module(&hir).map_err(|e| format!("run: {e:?}"))?;
    let val = interp
        .call("main", vec![])
        .map_err(|e| format!("call: {e:?}"))?;
    Ok((val, interp))
}

#[test]
fn py_plot_and_tensor_goldens_typecheck() {
    for rel in [
        "examples/golden/py_plot_report.vox",
        "examples/golden/py_tensor_infer.vox",
    ] {
        let path = repo_root().join(rel);
        let src = fs::read_to_string(&path).expect("read golden");
        let module = parse_script(lex(&src)).expect("parse golden");
        let mut hir = lower_module(&module);
        let diags = typecheck_hir_module_with_path(&src, &mut hir, Some(&path));
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == TypeckSeverity::Error)
            .map(|d| d.message.clone())
            .collect();
        assert!(errors.is_empty(), "{rel} typecheck errors: {errors:?}");
    }
}

#[test]
fn plot_and_tensor_require_effects() {
    let plot_src = "import std.frame\nimport std.plot\nfn bad() {\n    let rows = frame.scan_csv(\"a.csv\")\n    let _ = plot.bar(rows).save(\"out.png\")\n}\n";
    let m1 = parse_script(lex(plot_src)).expect("parse");
    let mut h1 = lower_module(&m1);
    let errs1: Vec<_> = typecheck_hir_module_with_path(plot_src, &mut h1, None)
        .into_iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .map(|d| d.message)
        .collect();
    assert!(
        errs1
            .iter()
            .any(|m| m.contains("save requires `uses python` and `uses fs`")),
        "expected Plot.save capability error, got {errs1:?}"
    );

    let tensor_src =
        "import std.tensor\nfn bad() {\n    let _ = tensor.load_exported(\"m.pt2\")\n}\n";
    let m2 = parse_script(lex(tensor_src)).expect("parse");
    let mut h2 = lower_module(&m2);
    let errs2: Vec<_> = typecheck_hir_module_with_path(tensor_src, &mut h2, None)
        .into_iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .map(|d| d.message)
        .collect();
    assert!(
        errs2
            .iter()
            .any(|m| m.contains("load_exported requires `uses python` and `uses fs`")),
        "expected tensor.load_exported capability error, got {errs2:?}"
    );
}

#[test]
fn plot_and_tensor_plans_are_pure_and_pickle_extensions_denied() {
    let before_arr = plan_worker_calls();
    let reshaped = ArrayPlan::from_list(vec![1.0, 2.0])
        .astype("f32")
        .reshape(vec![1, 2])
        .softmax(-1);
    assert_eq!(reshaped.rank(), 2);
    assert_eq!(plan_worker_calls(), before_arr);

    let before_plot = plot_worker_calls();
    let frame = vox_compiler::eval::frame_plan::FramePlan::scan_csv("fixtures/py_frame_report.csv");
    let _p = PlotPlan::hist(frame).x("month").title("Hist").legend(true);
    assert_eq!(plot_worker_calls(), before_plot);

    for bad_ext in ["weights.pt", "weights.pth", "weights.pkl", "weights.pickle"] {
        let src = format!(
            "import std.tensor\n@uses(python, fs)\nfn main() {{\n    return tensor.load_exported(\"{bad_ext}\")\n}}\n"
        );
        let (val, _) = eval_script(
            &src,
            CapabilitySet::developer_default(),
            Some(PythonProject {
                root: std::env::temp_dir(),
                max_fetch_mb: 1,
                timeout: Duration::from_secs(5),
            }),
        )
        .expect("eval");
        match val {
            VoxValue::Result(Err(err)) => match err.as_ref() {
                VoxValue::Str(msg) => assert!(
                    msg.contains("pickle archives are denied"),
                    "unexpected error for {bad_ext}: {msg}"
                ),
                other => panic!("unexpected err payload: {other:?}"),
            },
            other => panic!("expected Err for {bad_ext}, got {other:?}"),
        }
    }
}

fn has_python_worker_deps(python_bin: &Path) -> bool {
    std::process::Command::new(python_bin)
        .args(["-c", "import numpy, polars, pyarrow"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[test]
fn plot_png_and_tensor_pt2_cpu_seeded_end_to_end() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let py_dir = root.join(".vox").join("python");
    fs::create_dir_all(&py_dir).expect("py dir");
    fs::write(py_dir.join("uv.lock"), b"version = 1\n").expect("write uv.lock");

    // Copy fixtures so fs read/write stay inside tempdir sandbox.
    let fixtures = root.join("fixtures");
    fs::create_dir_all(&fixtures).expect("fixtures dir");
    fs::copy(
        repo_root().join("examples/golden/fixtures/py_frame_report.csv"),
        fixtures.join("py_frame_report.csv"),
    )
    .expect("copy csv");
    fs::copy(
        repo_root().join("examples/golden/fixtures/tiny_linear.pt2"),
        fixtures.join("tiny_linear.pt2"),
    )
    .expect("copy pt2");

    // Write a fake `.pt2` containing pickle magic `\x80` to verify content check.
    fs::write(fixtures.join("smuggled.pt2"), b"\x80\x04\x95fake_pickle").expect("write smuggled");

    let venv_py = repo_root().join(".vox/python/.venv/bin/python");
    let python_exe = if venv_py.is_file() {
        venv_py
    } else {
        PathBuf::from("python3")
    };
    if !has_python_worker_deps(&python_exe) {
        eprintln!("skip: python worker deps (numpy/polars/pyarrow) absent");
        return;
    }

    // Point tempdir `.vox/python/.venv/bin/python` or fall back to python3 on PATH.
    if python_exe != PathBuf::from("python3") {
        let bin_dir = py_dir.join(".venv").join("bin");
        fs::create_dir_all(&bin_dir).expect("bin dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&python_exe, bin_dir.join("python")).expect("symlink python");
    }

    let caps = CapabilitySet::parse(&format!(
        "py:yes,fs:rw:{},random:seed=42",
        root.to_string_lossy()
    ))
    .expect("parse caps");
    let project = PythonProject {
        root: root.to_path_buf(),
        max_fetch_mb: 16,
        timeout: Duration::from_secs(15),
    };

    // 1) Plot save produces non-empty valid PNG inside root.
    let out_png = root.join("chart.png");
    let plot_src = format!(
        "import std.frame\nimport std.plot\n@uses(python, fs)\nfn main() {{\n    let rows = frame.scan_csv(\"{}\").filter(frame.col(\"region\").eq(\"west\")).group_by([\"month\"]).agg([frame.col(\"amount\").sum().alias(\"revenue\")]).sort(\"month\", nulls=\"last\")\n    return plot.bar(rows).x(\"month\").y(\"revenue\").title(\"West\").legend(false).save(\"{}\")\n}}\n",
        fixtures.join("py_frame_report.csv").display(),
        out_png.display()
    );
    let (res, _) = eval_script(&plot_src, caps.clone(), Some(project.clone())).expect("plot eval");
    assert!(
        matches!(res, VoxValue::Result(Ok(_))),
        "plot save failed: {res:?}"
    );
    let png = fs::read(&out_png).expect("read png");
    assert!(png.starts_with(b"\x89PNG\r\n\x1a\n"), "valid PNG signature");

    // 2) Tensor load_exported + run twice with seed=42 produces identical floats, and smuggled.pt2 is rejected.
    let smuggled_src = format!(
        "import std.tensor\n@uses(python, fs)\nfn main() {{\n    return tensor.load_exported(\"{}\")\n}}\n",
        fixtures.join("smuggled.pt2").display()
    );
    let (smug_res, _) =
        eval_script(&smuggled_src, caps.clone(), Some(project.clone())).expect("smuggled eval");
    assert!(
        matches!(smug_res, VoxValue::Result(Err(_))),
        "smuggled pickle must fail"
    );

    let infer_src = format!(
        "import std.array\nimport std.tensor\n@uses(python, fs)\nfn main() {{\n    let m = tensor.load_exported(\"{}\").unwrap()\n    let x = array.from_list([1.0, 2.0]).astype(f32).reshape([1, 2])\n    let y = m.run(x).unwrap()\n    return y.softmax(-1).to_list().unwrap()\n}}\n",
        fixtures.join("tiny_linear.pt2").display()
    );
    let (r1, interp1) = eval_script(&infer_src, caps.clone(), Some(project.clone())).expect("run1");
    let (r2, _) = eval_script(&infer_src, caps, Some(project)).expect("run2");
    assert_eq!(r1, r2, "seeded execution must be deterministic");
    let meta = vox_compiler::eval::python::python_env_metadata(&interp1)
        .expect("python_env_metadata cached");
    assert!(
        meta.get("uv_lock_blake3").is_some(),
        "uv_lock_blake3 recorded: {meta}"
    );
}
