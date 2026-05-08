use assert_cmd::Command;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_mobile")
}

#[test]
fn build_android_produces_so_per_abi() {
    if which::which("cargo-ndk").is_err() {
        eprintln!("skipping: cargo-ndk not installed");
        return;
    }
    if std::env::var("ANDROID_NDK_HOME").is_err() {
        eprintln!("skipping: ANDROID_NDK_HOME not set");
        return;
    }

    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=android")
        .arg("--release")
        .assert()
        .success();

    let so = fixture_dir()
        .join("target/mobile/android/aarch64-linux-android/libhello_mobile.so");
    assert!(so.exists(), "expected {} to exist", so.display());
}

#[test]
fn build_android_fails_clearly_with_missing_section() {
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Vox.toml"),
        r#"[package]
name = "x"
kind = "application"

[build]
target = "mobile"

[mobile]
platforms = ["android"]
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let out = cmd
        .current_dir(dir.path())
        .arg("build")
        .arg("--platform=android")
        .output()
        .unwrap();

    assert!(
        !out.status.success(),
        "should fail with missing [mobile.android]"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("[mobile.android]") || stderr.contains("missing"),
        "expected helpful error; got: {stderr}"
    );
}

#[test]
fn build_android_fails_clearly_with_no_mobile_section() {
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    fs::write(
        dir.path().join("Vox.toml"),
        r#"[package]
name = "x"
kind = "application"

[build]
target = "mobile"
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let out = cmd
        .current_dir(dir.path())
        .arg("build")
        .arg("--platform=android")
        .output()
        .unwrap();

    assert!(!out.status.success(), "should fail with no [mobile] section");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("[mobile]") && stderr.contains("missing"),
        "expected error mentioning missing [mobile] section; got: {stderr}"
    );
    assert!(
        !stderr.contains("panicked") && !stderr.contains("RUST_BACKTRACE"),
        "should fail gracefully, not panic; got: {stderr}"
    );
}
