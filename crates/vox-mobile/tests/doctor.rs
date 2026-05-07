use assert_cmd::Command;

#[test]
fn doctor_prints_check_table() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let output = cmd.arg("doctor").output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("cargo-ndk"), "doctor should check cargo-ndk; got: {stdout}");
    assert!(stdout.contains("ANDROID_NDK_HOME"), "doctor should check ANDROID_NDK_HOME; got: {stdout}");
    assert!(stdout.contains("aarch64-linux-android"), "doctor should check the rustup target; got: {stdout}");
    #[cfg(target_os = "macos")]
    {
        assert!(stdout.contains("xcodebuild"), "doctor should check xcodebuild on macOS; got: {stdout}");
    }
}

#[test]
fn doctor_exits_with_documented_code() {
    let output = Command::cargo_bin("vox-mobile").unwrap().arg("doctor").output().unwrap();
    let code = output.status.code().expect("doctor should have an exit code");
    assert!(matches!(code, 0 | 1), "doctor should exit 0 or 1; got: {code}");
}
