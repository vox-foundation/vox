//! Plugin.toml is hand-maintained, not Cargo-generated, so nothing else
//! enforces it staying in sync with the crate's own `version.workspace =
//! true`. This is the drift gate: a workspace version bump that forgets to
//! touch Plugin.toml fails here, loudly, instead of shipping a plugin whose
//! declared version disagrees with the release it ships in.

#[test]
fn plugin_toml_version_matches_crate_version() {
    let manifest = include_str!("../Plugin.toml");
    let parsed: toml::Value = manifest.parse().expect("Plugin.toml must be valid TOML");
    let declared = parsed["plugin"]["version"]
        .as_str()
        .expect("Plugin.toml must have [plugin] version as a string");
    assert_eq!(
        declared,
        env!("CARGO_PKG_VERSION"),
        "Plugin.toml's version ({declared}) does not match this crate's \
         Cargo.toml version ({}). Update Plugin.toml's [plugin] version to \
         match -- it is not derived automatically.",
        env!("CARGO_PKG_VERSION")
    );
}

/// The CPU build ships with Plugin.cpu.toml; the installer derives the asset
/// name from its version, so it must track the crate and Plugin.toml too.
#[test]
fn plugin_cpu_toml_matches_plugin_toml() {
    let get = |src: &str| -> (String, i64) {
        let v: toml::Value = src.parse().unwrap();
        (
            v["plugin"]["version"].as_str().unwrap().to_string(),
            v["plugin"]["payload"]["abi-version"].as_integer().unwrap(),
        )
    };
    let main = get(include_str!("../Plugin.toml"));
    let cpu = get(include_str!("../Plugin.cpu.toml"));
    assert_eq!(
        cpu, main,
        "Plugin.cpu.toml version/abi-version must match Plugin.toml"
    );
    assert_eq!(cpu.0, env!("CARGO_PKG_VERSION"));
}
