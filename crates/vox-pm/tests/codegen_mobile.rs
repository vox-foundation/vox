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

    let parsed: toml::Value =
        toml::from_str(&out).expect("emitted Cargo.toml should parse as valid TOML");
    assert_eq!(parsed["package"]["name"].as_str(), Some("my-mobile-app"));
    assert_eq!(parsed["lib"]["crate-type"].as_array().unwrap().len(), 2);
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

    let parsed: toml::Value =
        toml::from_str(&out).expect("emitted Cargo.toml should parse as valid TOML");
    assert_eq!(parsed["package"]["name"].as_str(), Some("my-server"));
}

#[test]
fn no_build_section_uses_baseline() {
    // Tests the unwrap_or("fullstack") path when manifest.build is None.
    let manifest = VoxManifest {
        package: PackageSection {
            name: "hello-default".into(),
            version: "0.1.0".into(),
            kind: "application".into(),
            ..Default::default()
        },
        build: None,
        ..Default::default()
    };

    let cargo_toml = cargo_toml_for_manifest(&manifest);
    assert!(
        !cargo_toml.contains("cdylib"),
        "no [build] should not emit cdylib; got:\n{cargo_toml}"
    );
    assert!(
        cargo_toml.contains("vox-runtime"),
        "baseline should still pull in vox-runtime"
    );
    let parsed: toml::Value = toml::from_str(&cargo_toml)
        .expect("emitted Cargo.toml should parse as valid TOML");
    assert_eq!(parsed["package"]["name"].as_str(), Some("hello-default"));
}
