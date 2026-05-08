use assert_cmd::Command;
use std::path::PathBuf;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/hello_mobile")
}

#[test]
fn build_all_runs_all_listed_platforms() {
    let mut cmd = Command::cargo_bin("vox-mobile").unwrap();
    let output = cmd
        .current_dir(fixture_dir())
        .arg("build")
        .arg("--platform=all")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Must surface the per-platform attempt for each requested platform
    // (the fixture lists both "android" and "ios" in [mobile.platforms]).
    assert!(
        stderr.contains("Android") || stderr.contains("android"),
        "should mention Android attempt; got stderr: {stderr}"
    );

    // On non-macOS, iOS must be skipped with a clear "skipping iOS" line.
    #[cfg(not(target_os = "macos"))]
    {
        assert!(
            stderr.contains("skipping iOS"),
            "iOS should be skipped on non-macOS; got stderr: {stderr}"
        );
    }
}
