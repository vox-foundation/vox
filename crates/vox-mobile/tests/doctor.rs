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
fn doctor_succeeds_when_at_least_one_platform_is_complete() {
    // Cannot reliably guarantee any platform is fully installed in CI;
    // this test only asserts the binary runs without panicking.
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    cmd.arg("doctor").assert();
}
