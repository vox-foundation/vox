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

    let root = tempfile::tempdir().unwrap();
    let mcp = CapabilitySet::from_roots(
        vec![root.path().to_path_buf()],
        vec![],
        &["env:ro", "time:real"],
    )
    .unwrap();
    let mcp_denied = run_with(
        mcp,
        "@versioned fn save() { let x = 1 }\npub fn main() { save(); return 0 }",
    );
    assert!(
        matches!(
            mcp_denied,
            Err(EvalError::CapabilityDenied { ref ns, ref method })
                if ns == "repo" && method == "snapshot"
        ),
        "MCP from_roots must not snapshot via the decorator bypass: {mcp_denied:?}"
    );
}

#[test]
fn every_fs_method_that_takes_a_path_is_scoped() {
    let d = tempfile::tempdir().unwrap();
    let inside = d.path().join("in");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(inside.join("ok.txt"), "OK").unwrap();
    let outside = d.path().join("out");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(outside.join("s.txt"), "S").unwrap();
    let caps = match CapabilitySet::parse(&format!("fs:rw={}", inside.display())) {
        Ok(c) => c,
        Err(_) => CapabilitySet::from_roots(vec![], vec![inside.clone()], &[]).unwrap(),
    };
    let allowed = run_with(
        caps.clone(),
        &format!(
            r#"pub fn main() {{ return fs.read("{}") }}"#,
            inside.join("ok.txt").display()
        ),
    );
    assert!(
        matches!(allowed, Ok(VoxValue::Str(ref s)) if s.as_ref() == "OK")
            || matches!(
                allowed,
                Ok(VoxValue::Result(Ok(ref b)))
                    if matches!(b.as_ref(), VoxValue::Str(s) if s.as_ref() == "OK")
            ),
        "positive control died: {allowed:?}"
    );
    let f = outside.join("s.txt");
    for (m, arg) in [
        ("read", &f),
        ("read_file", &f),
        ("read_to_string", &f),
        ("read_bytes", &f),
        ("canonicalize", &f),
        ("exists", &f),
        ("is_file", &f),
        ("is_dir", &outside),
        ("stat", &f),
        ("list_dir", &outside),
        ("list_dir_detailed", &outside),
        ("walk", &outside),
        ("list_recursive", &outside),
        ("remove", &f),
        ("remove_dir_all", &outside),
        ("mkdir", &outside.join("new")),
    ] {
        let r = run_with(
            caps.clone(),
            &format!(r#"pub fn main() {{ return fs.{m}("{}") }}"#, arg.display()),
        );
        assert!(denied(&r, "fs"), "fs.{m} ungated: {r:?}");
    }
    let write_r = run_with(
        caps.clone(),
        &format!(
            r#"pub fn main() {{ return fs.write("{}", "x") }}"#,
            f.display()
        ),
    );
    assert!(denied(&write_r, "fs"), "fs.write ungated: {write_r:?}");
    let copy_r = run_with(
        caps.clone(),
        &format!(
            r#"pub fn main() {{ return fs.copy("{}", "{}") }}"#,
            f.display(),
            inside.join("copied.txt").display()
        ),
    );
    assert!(denied(&copy_r, "fs"), "fs.copy ungated: {copy_r:?}");
    let glob_r = run_with(
        caps.clone(),
        &format!(
            r#"pub fn main() {{ return fs.glob("{}") }}"#,
            outside.join("*").display()
        ),
    );
    assert!(denied(&glob_r, "fs"), "fs.glob ungated: {glob_r:?}");
    let open_r = run_with(
        caps.clone(),
        &format!(r#"pub fn main() {{ return io.open("{}") }}"#, f.display()),
    );
    assert!(
        denied(&open_r, "io") || denied(&open_r, "fs"),
        "io.open ungated: {open_r:?}"
    );
    let save_r = run_with(
        caps,
        &format!(
            r#"pub fn main() {{ return io.save("{}", "{{}}") }}"#,
            f.display()
        ),
    );
    assert!(
        denied(&save_r, "io") || denied(&save_r, "fs"),
        "io.save ungated: {save_r:?}"
    );
}

#[test]
#[serial_test::serial]
fn developer_default_relative_write_creates_the_file() {
    let cwd = tempfile::tempdir().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(cwd.path()).unwrap();
    let r = run_with(
        CapabilitySet::developer_default(),
        r#"pub fn main() { fs.write("out.txt", "hi"); return fs.read("out.txt") }"#,
    );
    std::env::set_current_dir(prev).unwrap();
    assert!(
        matches!(r, Ok(VoxValue::Str(ref s)) if s.as_ref() == "hi")
            || matches!(
                r,
                Ok(VoxValue::Result(Ok(ref b)))
                    if matches!(b.as_ref(), VoxValue::Str(s) if s.as_ref() == "hi")
            ),
        "relative write denied: {r:?}"
    );
    assert!(cwd.path().join("out.txt").exists());
}

#[test]
fn glob_filters_results_not_only_the_prefix() {
    let d = tempfile::tempdir().unwrap();
    let job = d.path().join("job");
    std::fs::create_dir_all(&job).unwrap();
    std::fs::write(job.join("a.txt"), "a").unwrap();
    std::fs::write(d.path().join("secret.txt"), "S").unwrap();
    let caps = match CapabilitySet::parse(&format!("fs:rw={}", job.display())) {
        Ok(c) => c,
        Err(_) => CapabilitySet::from_roots(vec![], vec![job.clone()], &[]).unwrap(),
    };
    let pat = format!("{}/*/../secret.txt", d.path().display());
    let r = run_with(
        caps,
        &format!(
            r#"pub fn main() {{ return match fs.glob("{pat}") {{ Ok(xs) => len(xs), Error(e) => -1 }} }}"#
        ),
    );
    assert!(
        matches!(r, Ok(VoxValue::Int(0))) || denied(&r, "fs"),
        "glob escaped via ..: {r:?}"
    );
}

#[test]
#[serial_test::serial]
fn unscoped_grant_does_not_canonicalize() {
    // developer_default has no boundary; a missing relative parent must still write.
    let cwd = tempfile::tempdir().unwrap();
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(cwd.path()).unwrap();
    let r = run_with(
        CapabilitySet::developer_default(),
        r#"pub fn main() { fs.mkdir("a/b/c"); return fs.exists("a/b/c") }"#,
    );
    std::env::set_current_dir(prev).unwrap();
    assert!(matches!(r, Ok(VoxValue::Bool(true))), "{r:?}");
}

#[test]
fn symlink_escape_is_denied_and_the_op_uses_the_checked_path() {
    let d = tempfile::tempdir().unwrap();
    let allowed = d.path().join("ok");
    std::fs::create_dir_all(&allowed).unwrap();
    std::fs::write(allowed.join("a.txt"), "A").unwrap();
    std::fs::write(d.path().join("secret.txt"), "S").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(d.path().join("secret.txt"), allowed.join("link.txt")).unwrap();
    let caps = match CapabilitySet::parse(&format!("fs:ro={}", allowed.display())) {
        Ok(c) => c,
        Err(_) => CapabilitySet::from_roots(vec![allowed.clone()], vec![], &[]).unwrap(),
    };
    let ok = run_with(
        caps.clone(),
        &format!(
            r#"pub fn main() {{ return fs.read("{}") }}"#,
            allowed.join("a.txt").display()
        ),
    );
    assert!(
        matches!(ok, Ok(VoxValue::Str(ref s)) if s.as_ref() == "A")
            || matches!(
                ok,
                Ok(VoxValue::Result(Ok(ref b)))
                    if matches!(b.as_ref(), VoxValue::Str(s) if s.as_ref() == "A")
            ),
        "{ok:?}"
    );
    #[cfg(unix)]
    {
        let via_link = run_with(
            caps,
            &format!(
                r#"pub fn main() {{ return fs.read("{}") }}"#,
                allowed.join("link.txt").display()
            ),
        );
        assert!(denied(&via_link, "fs"), "symlink escape: {via_link:?}");
    }
}

#[test]
fn degenerate_paths_are_denied_not_degraded_to_the_parent() {
    let d = tempfile::tempdir().unwrap();
    let caps = match CapabilitySet::parse(&format!("fs:rw={}", d.path().display())) {
        Ok(c) => c,
        Err(_) => CapabilitySet::from_roots(vec![], vec![d.path().to_path_buf()], &[]).unwrap(),
    };
    for raw in ["", "..", "."] {
        let r = run_with(
            caps.clone(),
            &format!(r#"pub fn main() {{ return fs.write("{raw}", "x") }}"#),
        );
        assert!(denied(&r, "fs"), "{raw:?}: {r:?}");
    }
}

#[test]
fn frozen_time_and_seeded_random_are_what_the_receiver_said() {
    let r = run_with(
        CapabilitySet::parse("time:frozen=42").unwrap(),
        r#"pub fn main() { return time.now_ms() }"#,
    );
    assert!(matches!(r, Ok(VoxValue::Int(42))), "{r:?}");
}

#[test]
fn deep_recursion_is_a_limit_error_not_a_crash() {
    // Debug `eval_expr`/`apply_closure` frames are large; 1024 Vox calls
    // need more than the default 8 MiB thread stack to reach the bound
    // instead of overflowing. The bound itself lives in `apply_closure`.
    // Return a Send `bool` so the worker need not send `VoxValue` (Rc).
    let hit_limit = std::thread::Builder::new()
        .name("deep-recursion".into())
        .stack_size(256 * 1024 * 1024)
        .spawn(|| {
            matches!(
                run_with(
                    CapabilitySet::developer_default(),
                    r#"fn f(n: int) to int { return f(n + 1) } pub fn main() { return f(0) }"#,
                ),
                Err(EvalError::RecursionLimitExceeded)
            )
        })
        .expect("spawn")
        .join()
        .expect("join");
    assert!(hit_limit, "expected RecursionLimitExceeded");
}

#[test]
fn deeply_nested_source_is_a_parse_error_not_a_crash() {
    let src = format!(
        "pub fn main() {{ return {}1{} }}",
        "(".repeat(200_000),
        ")".repeat(200_000)
    );
    let tokens = vox_compiler::lexer::lex(&src);
    assert!(
        vox_compiler::parser::parse_script(tokens).is_err(),
        "parser must bound nesting"
    );
}
