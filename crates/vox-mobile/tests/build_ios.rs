use assert_cmd::Command;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_mobile")
}

#[cfg(target_os = "macos")]
#[test]
fn build_ios_produces_xcframework() {
    if which::which("xcodebuild").is_err() {
        eprintln!("skipping: xcodebuild not installed");
        return;
    }

    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=ios")
        .arg("--release")
        .assert()
        .success();

    let xcf = fixture_dir().join("target/mobile/ios/hello_mobile.xcframework");
    assert!(xcf.exists(), "expected {} to exist", xcf.display());
    let info = xcf.join("Info.plist");
    assert!(info.exists(), "expected XCFramework Info.plist");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn build_ios_fails_clearly_on_non_macos() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let output = cmd
        .current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=ios")
        .output()
        .unwrap();
    assert!(!output.status.success(), "should fail on non-macOS");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("macOS"),
        "expected macOS gate error; got: {stderr}"
    );
}
