//! Spec §3.2 items 2–3: denial is fatal; every side-effecting entry is gated.
use vox_compiler::eval::caps::CapabilitySet;
use vox_compiler::eval::value::VoxValue;
use vox_compiler::eval::{EvalError, Interpreter};

fn run_with(caps: CapabilitySet, src: &str) -> Result<VoxValue, EvalError> {
    run_with_path(caps, src, None)
}

fn run_with_path(
    caps: CapabilitySet,
    src: &str,
    source_path: Option<std::path::PathBuf>,
) -> Result<VoxValue, EvalError> {
    let tokens = vox_compiler::lexer::lex(src);
    let module = vox_compiler::parser::descent::parse(tokens).expect("parse");
    let lowered = vox_compiler::hir::lower::lower_module(&module);
    let mut interp = Interpreter::new(1_000_000);
    interp.caps = caps;
    if let Some(p) = source_path {
        interp.set_source_path(p);
    }
    interp.run_module(&lowered)?;
    interp.call("main", vec![])
}

fn denied(r: &Result<VoxValue, EvalError>, ns: &str) -> bool {
    matches!(r, Err(EvalError::CapabilityDenied { ns: n, .. }) if n == ns)
}

#[test]
fn denied_fs_read_is_fatal_and_nothing_after_it_runs() {
    let r = run_with(
        CapabilitySet::parse("env:ro").unwrap(),
        r#"pub fn main() { let s = fs.read("/etc/hosts"); print("MUST NOT PRINT"); return 1 }"#,
    );
    match r {
        Err(EvalError::CapabilityDenied { ns, method }) => {
            assert_eq!(ns, "fs");
            assert_eq!(method, "read");
        }
        other => panic!("expected CapabilityDenied, got {other:?}"),
    }
}

#[test]
fn http_is_gated_under_std() {
    let r = run_with(
        CapabilitySet::parse("").unwrap(),
        r#"pub fn main() { return std.http.get_text("http://127.0.0.1:9/") }"#,
    );
    assert!(
        matches!(r, Err(EvalError::CapabilityDenied { ref ns, .. }) if ns == "http"),
        "{r:?}"
    );
}

#[test]
fn repo_is_pure_and_allowed_without_caps() {
    let r = run_with(
        CapabilitySet::parse("").unwrap(),
        r#"pub fn main() { return len(repo.changes()) }"#,
    );
    assert!(matches!(r, Ok(VoxValue::Int(0))), "repo is PURE: {r:?}");
}

#[test]
fn path_resolve_is_an_fs_read() {
    let r = run_with(
        CapabilitySet::parse("").unwrap(),
        r#"pub fn main() { return path.resolve(".") }"#,
    );
    assert!(
        matches!(r, Err(EvalError::CapabilityDenied { ref ns, .. }) if ns == "fs"),
        "{r:?}"
    );
}

#[test]
fn env_set_needs_rw() {
    let ro = run_with(
        CapabilitySet::parse("env:ro").unwrap(),
        r#"pub fn main() { env.set("VOX_CAPS_TEST_KEY", "1"); return 1 }"#,
    );
    assert!(denied(&ro, "env"), "env:ro must deny set: {ro:?}");
    let rw = run_with(
        CapabilitySet::parse("env:rw").unwrap(),
        r#"pub fn main() { env.set("VOX_CAPS_TEST_KEY", "1"); return env.get("VOX_CAPS_TEST_KEY").is_some() }"#,
    );
    assert!(matches!(rw, Ok(VoxValue::Bool(true))), "{rw:?}");
}

#[test]
fn register_exit_command_is_gated_at_queue_time() {
    let r = run_with(
        CapabilitySet::parse("").unwrap(),
        r#"pub fn main() { process.register_exit_command("true", []); return 1 }"#,
    );
    assert!(denied(&r, "process"), "{r:?}");
}

#[test]
fn allowed_register_exit_command_queues_on_the_interpreter() {
    let src = r#"pub fn main() { process.register_exit_command("true", []); return 1 }"#;
    let tokens = vox_compiler::lexer::lex(src);
    let module = vox_compiler::parser::descent::parse(tokens).expect("parse");
    let lowered = vox_compiler::hir::lower::lower_module(&module);
    let mut interp = Interpreter::new(1_000_000);
    interp.caps = CapabilitySet::developer_default();
    interp.run_module(&lowered).expect("run_module");
    let r = interp.call("main", vec![]);
    assert!(matches!(r, Ok(VoxValue::Int(1))), "{r:?}");
    assert_eq!(
        interp.exit_commands,
        vec![("true".into(), Vec::<String>::new())]
    );
}

#[test]
fn import_outside_the_fs_roots_is_denied_before_main_runs() {
    let root = tempfile::tempdir().unwrap();
    let inside = root.path().join("in");
    let outside = root.path().join("out");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("lib.vox"), "pub fn leak() { return 1 }\n").unwrap();
    let main_path = inside.join("main.vox");
    let src = r#"import "../out/lib.vox"
pub fn main() { return leak() }
"#;
    std::fs::write(&main_path, src).unwrap();
    let caps = CapabilitySet::from_roots(vec![inside.clone()], vec![], &[]).unwrap();
    let r = run_with_path(caps, src, Some(main_path));
    assert!(
        matches!(
            r,
            Err(EvalError::CapabilityDenied { ref ns, ref method })
                if ns == "fs" && method == "import"
        ),
        "{r:?}"
    );
}

#[test]
fn a_default_interpreter_is_developer_default_not_ungated() {
    let r = run_with(
        CapabilitySet::developer_default(),
        r#"pub fn main() { return env.get("PATH").is_some() }"#,
    );
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn every_seeded_namespace_is_classified() {
    let src = include_str!("../src/eval/mod.rs");
    let mut names = std::collections::BTreeSet::new();
    let lines: Vec<&str> = src.lines().collect();
    for (i, l) in lines.iter().enumerate() {
        if l.contains("\"__namespace__\"")
            && let Some(next) = lines.get(i + 1)
            && let Some(start) = next.find("Str(\"")
        {
            let rest = &next[start + 5..];
            if let Some(end) = rest.find('"') {
                names.insert(rest[..end].to_string());
            }
        }
    }
    assert!(
        !names.is_empty(),
        "no namespaces parsed from eval/mod.rs — test is broken"
    );
    for ns in &names {
        assert!(
            vox_compiler::eval::caps::is_classified(ns),
            "namespace `{ns}` is seeded in Interpreter::new but is in neither caps::GATED nor caps::PURE"
        );
    }
}

#[test]
fn versioned_snapshot_is_gated_when_repo_write_is_denied_by_policy() {
    let ok = run_with(
        CapabilitySet::developer_default(),
        "@versioned fn save() { let x = 1 }\npub fn main() { save(); return len(repo.changes()) }",
    );
    assert!(
        matches!(ok, Ok(VoxValue::Int(n)) if n >= 1),
        "developer_default snapshots: {ok:?}"
    );

    let denied = run_with(
        CapabilitySet::parse("").unwrap(),
        "@versioned fn save() { let x = 1 }\npub fn main() { save(); return 0 }",
    );
    assert!(
        matches!(
            denied,
            Err(EvalError::CapabilityDenied { ref ns, ref method })
                if ns == "repo" && method == "snapshot"
        ),
        "restrictive embedder must not snapshot via the decorator bypass: {denied:?}"
    );
}
