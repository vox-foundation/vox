use vox_pm::codegen::cargo_toml_for_manifest;
use vox_pm::manifest::{BuildSection, MobileSection, PackageSection, VoxManifest};

#[test]
fn mobile_target_emits_cdylib_and_staticlib() {
    let manifest = VoxManifest {
        package: PackageSection {
            name: "my-mobile-app".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
        build: Some(BuildSection {
            target: Some("mobile".to_string()),
        }),
        mobile: Some(MobileSection {
            platforms: vec!["android".to_string()],
            ..Default::default()
        }),
        ..Default::default()
    };

    let out = cargo_toml_for_manifest(&manifest);
    assert!(out.contains("[lib]"), "expected [lib] section, got:\n{out}");
    assert!(
        out.contains(r#"crate-type = ["cdylib", "staticlib"]"#),
        "expected cdylib+staticlib crate-type, got:\n{out}"
    );
    assert!(out.contains("vox-runtime"), "expected vox-runtime dep");
    assert!(out.contains("vox-oratio"), "expected vox-oratio dep");
    assert!(
        out.contains("stt-sherpa"),
        "expected stt-sherpa feature on vox-oratio"
    );
    assert!(out.contains("vox-crypto"), "expected vox-crypto dep");
    assert!(out.contains("vox-db"), "expected vox-db dep");
    assert!(
        out.contains(r#"[target.'cfg(target_os = "android")'.dependencies]"#),
        "expected android cfg target deps section"
    );
    assert!(out.contains("jni"), "expected jni dep under android cfg");
    assert!(out.contains("my-mobile-app"));
}

#[test]
fn server_target_does_not_emit_cdylib() {
    let manifest = VoxManifest {
        package: PackageSection {
            name: "my-server".to_string(),
            version: "0.1.0".to_string(),
            ..Default::default()
        },
        build: Some(BuildSection {
            target: Some("server".to_string()),
        }),
        ..Default::default()
    };

    let out = cargo_toml_for_manifest(&manifest);
    assert!(
        !out.contains("cdylib"),
        "server target should not emit cdylib, got:\n{out}"
    );
    assert!(
        !out.contains("staticlib"),
        "server target should not emit staticlib, got:\n{out}"
    );
    assert!(out.contains("vox-runtime"));
    assert!(out.contains("my-server"));
}
