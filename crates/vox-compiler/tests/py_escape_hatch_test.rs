//! Phase 329 (PYX-01..04): Tier 2 Python module escape hatch tests.
//!
//! Verifies:
//! - Contextual `import py <alias> from "<module>"` parsing, formatting, and lowering.
//! - Compile-time `Denied` tier blocking (`pickle`, `marshal`, `subprocess`, `torch.load`,
//!   `eval`, `exec`, `importlib`, `os.system`, `os.popen`, `os.spawn*`, `os.exec*`).
//! - Typed curated `.vox.pyi` sidecar resolution (`scipy.signal`, `pandas`, etc.).
//! - Workspace `.vox/python/bindings/<module>.vox.pyi` resolution + `uv.lock` BLAKE3
//!   staleness compile error.
//! - Unbound `PyObject` opacity (no field access, indexing, operators, or crossing
//!   `pub fn` / `@activity` / `workflow` boundaries; requires `.decode(...)` + `uses python_unverified`).
//! - Warning diagnostic `vox/python/unverified-call`.
//! - Hashed telemetry payload (`module_hash`, `symbol_hash`, `tier`) + runtime worker `escape_call`.

use std::path::Path;
use vox_compiler::eval::Interpreter;
use vox_compiler::eval::caps::CapabilitySet;
use vox_compiler::eval::python::{PythonProject, set_python_project};
use vox_compiler::eval::value::VoxValue;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_compiler::python_escape_support::{
    format_escape_telemetry_event, is_denied_python_target, load_sidecar, sanitize_sidecar_filename,
};
use vox_compiler::typeck::diagnostics::{Diagnostic, TypeckSeverity, codes};
use vox_compiler::typeck::typecheck_hir_module_with_path;

fn check_with_path(src: &str, path: Option<&Path>) -> Vec<Diagnostic> {
    let module = parse(lex(src)).unwrap_or_else(|e| panic!("parse failed: {e:?}"));
    let mut hir = lower_module(&module);
    typecheck_hir_module_with_path(src, &mut hir, path)
}

fn errors_of(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.severity == TypeckSeverity::Error)
        .collect()
}

fn warnings_of(diags: &[Diagnostic]) -> Vec<&Diagnostic> {
    diags
        .iter()
        .filter(|d| d.severity == TypeckSeverity::Warning)
        .collect()
}

#[test]
fn py_module_import_parses_formats_and_preserves_symbol_py_imports() {
    let src = "import py sig from \"scipy.signal\"\n\nfn helper() to int {\n    return 1\n}\n";
    let ast = parse(lex(src)).expect("parse py import");
    let formatted = vox_compiler::format(src);
    assert!(
        formatted.contains("import py sig from \"scipy.signal\""),
        "formatted should round-trip import py: {formatted}"
    );
    let hir = lower_module(&ast);
    assert_eq!(hir.py_imports.len(), 1);
    assert_eq!(hir.py_imports[0].alias, "sig");
    assert_eq!(hir.py_imports[0].module, "scipy.signal");
}

#[test]
fn denied_tier_modules_and_symbols_fail_typecheck() {
    for src in [
        "import py pkl from \"pickle\"\nfn main() { }",
        "import py sub from \"subprocess\"\nfn main() { }",
        "import py imp from \"importlib\"\nfn main() { }",
        "import py m from \"marshal\"\nfn main() { }",
        "import py sh from \"shelve\"\nfn main() { }",
        "import py o from \"os\"\n@uses(python_unverified)\nfn main() { let _ = o.system(\"echo hi\") }",
        "import py t from \"torch\"\n@uses(python_unverified)\nfn main() { let _ = t.load(\"x.pt\") }",
        "import py b from \"builtins\"\n@uses(python_unverified)\nfn main() { let _ = b.eval(\"1+1\") }",
    ] {
        let diags = check_with_path(src, None);
        let errs = errors_of(&diags);
        assert!(
            errs.iter().any(|e| e.message.contains("Denied tier")),
            "expected Denied tier error for `{src}`, got: {errs:?}"
        );
    }
}

#[test]
fn curated_sidecar_checks_signature_and_emits_unverified_warning() {
    let golden =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/golden/py_escape_psd.vox");
    let src = std::fs::read_to_string(&golden).expect("read py_escape_psd.vox");
    let diags = check_with_path(&src, Some(&golden));
    let errs = errors_of(&diags);
    assert!(
        errs.is_empty(),
        "curated golden must typecheck with no errors, got: {errs:?}"
    );
    let warns = warnings_of(&diags);
    assert!(
        warns
            .iter()
            .any(|w| w.code.as_deref() == Some(codes::PYTHON_UNVERIFIED_CALL)),
        "expected PYTHON_UNVERIFIED_CALL warning, got: {warns:?}"
    );

    let bad_args = r#"
import py sig from "scipy.signal"
@uses(python)
fn compute_psd(samples: list[float]) to list[float] {
    return sig.periodogram("not-a-list")
}
"#;
    let bad_diags = check_with_path(bad_args, Some(&golden));
    assert!(
        !errors_of(&bad_diags).is_empty(),
        "curated sidecar must reject mismatched arg type"
    );
}

#[test]
fn unbound_py_object_opacity_and_boundary_guards() {
    let dir = tempfile::tempdir().unwrap();
    let vox_file = dir.path().join("app.vox");

    // Unbound call without `python_unverified` fails typeck.
    let missing_cap = r#"
import py math from "math"
@uses(python)
fn f() to Result[float, str] {
    return math.sqrt(9.0).decode(float)
}
"#;
    let diags = check_with_path(missing_cap, Some(&vox_file));
    assert!(
        errors_of(&diags)
            .iter()
            .any(|e| e.message.contains("python_unverified")),
        "missing python_unverified must fail: {diags:?}"
    );

    // Valid `.decode(float)` typechecks with warning.
    let valid_decode = r#"
import py math from "math"
@uses(python_unverified)
fn f() to Result[float, str] {
    return math.sqrt(9.0).decode(float)
}
"#;
    let ok_diags = check_with_path(valid_decode, Some(&vox_file));
    assert!(
        errors_of(&ok_diags).is_empty(),
        "unexpected errors: {ok_diags:?}"
    );
    assert!(
        warnings_of(&ok_diags)
            .iter()
            .any(|w| w.code.as_deref() == Some(codes::PYTHON_UNVERIFIED_CALL))
    );

    // PyObject field access, indexing, operator, and pub fn / @activity return are all rejected.
    for (label, bad_src) in [
        (
            "field_access",
            "import py math from \"math\"\n@uses(python_unverified)\nfn f() { let _ = math.sqrt(4.0).foo }",
        ),
        (
            "indexing",
            "import py math from \"math\"\n@uses(python_unverified)\nfn f() { let _ = math.sqrt(4.0)[0] }",
        ),
        (
            "binary_op",
            "import py math from \"math\"\n@uses(python_unverified)\nfn f() { let _ = math.sqrt(4.0) + 1.0 }",
        ),
        (
            "pub_fn_boundary",
            "import py math from \"math\"\npub @uses(python_unverified)\nfn leak() { return math.sqrt(4.0) }",
        ),
    ] {
        let diags = check_with_path(bad_src, Some(&vox_file));
        let errs = errors_of(&diags);
        assert!(
            errs.iter().any(|e| e.message.contains("PyObject")),
            "{label}: expected PyObject opacity/boundary error, got: {errs:?}"
        );
    }
}

#[test]
fn stale_uv_lock_sidecar_is_compile_error() {
    let dir = tempfile::tempdir().unwrap();
    let py_dir = dir.path().join(".vox/python");
    let bindings_dir = py_dir.join("bindings");
    std::fs::create_dir_all(&bindings_dir).unwrap();
    std::fs::write(py_dir.join("uv.lock"), b"current-lock-contents").unwrap();
    let stale_hash = "0".repeat(64);
    std::fs::write(
        bindings_dir.join("math.vox.pyi"),
        format!("# vox-lock-blake3: {stale_hash}\nfn sqrt(x: float) -> float\n"),
    )
    .unwrap();
    let app_path = dir.path().join("main.vox");
    let src = r#"
import py m from "math"
@uses(python)
fn run() to float {
    return m.sqrt(9.0)
}
"#;
    let diags = check_with_path(src, Some(&app_path));
    assert!(
        errors_of(&diags)
            .iter()
            .any(|e| e.message.contains("stale Python sidecar")
                && e.message.contains("vox py bind math")),
        "expected stale sidecar compile error, got: {diags:?}"
    );
}

#[test]
fn telemetry_payload_hashes_module_and_symbol_never_raw_strings() {
    let payload = format_escape_telemetry_event("scipy.signal", "periodogram", "curated");
    assert_eq!(payload["event"], "python.escape_call");
    assert_eq!(payload["tier"], "curated");
    let mh = payload["module_hash"].as_str().unwrap();
    let sh = payload["symbol_hash"].as_str().unwrap();
    assert_eq!(mh.len(), 64);
    assert_eq!(sh.len(), 64);
    assert!(!mh.contains("scipy") && !sh.contains("periodogram"));
}

#[test]
fn runtime_escape_call_executes_unbound_stdlib_math_when_python_available() {
    if which::which("python3").is_err() {
        eprintln!("skip: python3 absent");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    // Create a thin venv symlink to system python3 so the worker launches without network.
    let bin_dir = dir.path().join(".vox/python/.venv/bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    #[cfg(unix)]
    {
        let py3 = which::which("python3").unwrap();
        std::os::unix::fs::symlink(py3, bin_dir.join("python")).unwrap();
    }

    let src = r#"
import py math from "math"

@uses(python_unverified)
fn root_of(x: float) to Result[float, str] {
    let raw = math.sqrt(x)
    return raw.decode(float)
}
"#;
    let module = parse(lex(src)).expect("parse");
    let hir = lower_module(&module);
    let mut interp = Interpreter::new(10_000);
    interp.caps = CapabilitySet::parse("py:unverified").unwrap();
    interp.source_path = Some(dir.path().join("app.vox"));
    set_python_project(
        &mut interp,
        PythonProject {
            root: dir.path().to_path_buf(),
            max_fetch_mb: 16,
            timeout: std::time::Duration::from_secs(10),
        },
    );
    interp.run_module(&hir).expect("run_module");
    let res = interp
        .call("root_of", vec![VoxValue::Float(16.0)])
        .expect("call root_of");
    match res {
        VoxValue::Result(Ok(inner)) => match *inner {
            VoxValue::Float(v) => assert!((v - 4.0).abs() < 1e-9, "got {v}"),
            other => panic!("expected Float(4.0), got {other:?}"),
        },
        other => panic!("expected Ok(4.0), got {other:?}"),
    }
    assert_eq!(interp.python_escape_telemetry.len(), 1);
    assert_eq!(interp.python_escape_telemetry[0]["tier"], "unverified");
}

#[test]
fn denied_escape_aliases_and_dir_sidecar_resolution_hold() {
    assert!(is_denied_python_target("torch.serialization", Some("load")).is_some());
    assert!(is_denied_python_target("posix", Some("system")).is_some());
    assert!(is_denied_python_target("_posixsubprocess", None).is_some());
    assert!(is_denied_python_target("_imp", None).is_some());
    assert!(sanitize_sidecar_filename(".hidden").is_err());
    assert!(sanitize_sidecar_filename("bad name").is_err());

    let dir = tempfile::tempdir().unwrap();
    let py_dir = dir.path().join(".vox/python");
    let bindings_dir = py_dir.join("bindings");
    std::fs::create_dir_all(&bindings_dir).unwrap();
    let lock_bytes = b"dir-test-lock\n";
    std::fs::write(py_dir.join("uv.lock"), lock_bytes).unwrap();
    let hash = vox_crypto::hex_encode(&vox_crypto::secure_hash(lock_bytes));
    std::fs::write(
        bindings_dir.join("custom_mod.vox.pyi"),
        format!("# vox-lock-blake3: {hash}\nfn ping() -> int\n"),
    )
    .unwrap();

    let loaded = load_sidecar("custom_mod", Some(dir.path()))
        .expect("load_sidecar with directory")
        .expect("local binding resolved when directory path is passed");
    assert!(!loaded.is_curated);
    assert_eq!(loaded.fns[0].name, "ping");
}
