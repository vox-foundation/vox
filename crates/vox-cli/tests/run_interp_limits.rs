use std::process::Command;
fn vox() -> String {
    env!("CARGO_BIN_EXE_vox").to_string()
}
fn write(name: &str, src: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!("vox-limits-{name}-{}.vox", std::process::id()));
    std::fs::write(&p, src).unwrap();
    p
}

#[test]
fn capability_denial_exits_77_with_a_marker_on_stderr_and_nothing_on_stdout() {
    let f = write(
        "caps",
        r#"pub fn main() { let s = fs.read("/etc/hosts"); print("LEAK") }"#,
    );
    let out = Command::new(vox())
        .args(["run", "--mode", "interp", "--caps", "env:ro"])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(77),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!String::from_utf8_lossy(&out.stdout).contains("LEAK"));
    assert!(String::from_utf8_lossy(&out.stderr).contains("vox: capability denied: fs.read"));
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--caps"),
        "error must name the flag that grants it"
    );
}

#[test]
fn caps_is_repeatable() {
    let d = tempfile::tempdir().unwrap();
    std::fs::write(d.path().join("a.txt"), "A").unwrap();
    let f = write(
        "rep",
        &format!(
            r#"pub fn main() {{ print(fs.read("{}/a.txt")) }}"#,
            d.path().display()
        ),
    );
    let out = Command::new(vox())
        .args([
            "run",
            "--mode",
            "interp",
            "--caps",
            &format!("fs:ro={}", d.path().display()),
            "--caps",
            "time:real",
        ])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stdout).contains('A'));
}

#[test]
fn caps_flag_overrides_script_directive() {
    let f = write(
        "override",
        "// vox:caps fs\npub fn main() { let s = fs.read(\"/etc/hosts\"); print(\"LEAK\") }",
    );
    let out = Command::new(vox())
        .args(["run", "--mode", "interp", "--caps", "env:ro"])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(77));
}

#[test]
fn step_and_depth_limits_exit_78() {
    let f = write(
        "steps",
        "pub fn main() { let mut i = 0; while true { i = i + 1 } }",
    );
    let out = Command::new(vox())
        .args(["run", "--mode", "interp", "--max-steps", "10000"])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(78),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let g = write(
        "depth",
        "fn f(n: int) to int { return f(n + 1) } pub fn main() { return f(0) }",
    );
    let out = Command::new(vox())
        .args(["run", "--mode", "interp", "--max-depth", "32"])
        .arg(&g)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(78),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn memory_limit_exits_79() {
    // Doubling crosses 64 MiB in 26 iterations. Do not use list.push.
    let f = write(
        "mem",
        r#"pub fn main() { let mut s = "x"; let mut i = 0; while i < 40 { s = s + s; i = i + 1 } }"#,
    );
    let out = Command::new(vox())
        .args([
            "run",
            "--mode",
            "interp",
            "--max-memory",
            "67108864",
            "--max-steps",
            "10000",
        ])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(79),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("memory limit exceeded"));
}

#[test]
fn composite_result_rendering_cannot_escape_memory_accounting() {
    let f = write(
        "render-mem",
        r#"pub fn main() { let s = "x".repeat(4194304); return [s, s, s, s, s, s, s, s] }"#,
    );
    let out = Command::new(vox())
        .args([
            "run",
            "--mode",
            "interp",
            "--max-memory",
            "16777216",
            "--max-steps",
            "10000",
        ])
        .arg(&f)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(79),
        "rendering escaped the allocator ceiling: status={:?}, stdout={} bytes, stderr={}",
        out.status.code(),
        out.stdout.len(),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("memory limit exceeded"));
}

#[test]
fn argv_is_the_scripts_own() {
    let f = write(
        "argv",
        r#"pub fn main() { let a = env.args(); print(str(len(a))); print(a[1]) }"#,
    );
    let out = Command::new(vox())
        .args(["run", "--mode", "interp"])
        .arg(&f)
        .args(["--", "hello"])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(
        s.contains("hello"),
        "{s:?} stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
