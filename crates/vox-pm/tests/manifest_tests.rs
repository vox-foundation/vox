use vox_pm::manifest::{validate_mobile, VoxManifest};

#[test]
fn parses_minimal_mobile_manifest() {
    let toml_src = r#"
[package]
name = "hello-mobile"
kind = "application"

[build]
target = "mobile"

[mobile]
platforms = ["android", "ios"]

[mobile.android]
min_sdk = 26
target_sdk = 35
abis = ["arm64-v8a", "armeabi-v7a", "x86_64"]
ndk_version = "27.0.11902837"

[mobile.ios]
min_version = "15.0"
archs = ["aarch64-apple-ios", "aarch64-apple-ios-sim", "x86_64-apple-ios"]
"#;

    let manifest: VoxManifest = toml::from_str(toml_src).expect("parse failed");
    let build = manifest.build.expect("missing [build]");
    assert_eq!(build.target.as_deref(), Some("mobile"));

    let mobile = manifest.mobile.expect("missing [mobile]");
    assert_eq!(mobile.platforms, vec!["android".to_string(), "ios".to_string()]);

    let android = mobile.android.expect("missing [mobile.android]");
    assert_eq!(android.min_sdk, Some(26));
    assert_eq!(android.target_sdk, Some(35));
    assert_eq!(android.abis, vec!["arm64-v8a", "armeabi-v7a", "x86_64"]);
    assert_eq!(android.ndk_version.as_deref(), Some("27.0.11902837"));

    let ios = mobile.ios.expect("missing [mobile.ios]");
    assert_eq!(ios.min_version.as_deref(), Some("15.0"));
    assert_eq!(ios.archs.len(), 3);
}

#[test]
fn rejects_unknown_platform() {
    let toml_src = r#"
[package]
name = "x"

[build]
target = "mobile"

[mobile]
platforms = ["windows-mobile"]
"#;
    let manifest: VoxManifest = toml::from_str(toml_src).unwrap();
    let result = validate_mobile(&manifest);
    assert!(result.is_err(), "unknown platform should be rejected");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("windows-mobile"), "error should name the offending platform; got: {msg}");
}
