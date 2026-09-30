//! Pure sidecar build-plan logic for `build.rs`, kept in its own file so it can
//! be unit-tested (`tests/build_sidecar_test.rs` includes it via `#[path]`);
//! build scripts themselves never run `#[cfg(test)]` code.

/// `tauri.conf.json` `bundle.externalBin` binary name -> the workspace package
/// that owns that `[[bin]]` target. `cargo build -p <pkg> --bin <bin>` fails
/// with "no bin target named ..." if these disagree, so a new sidecar must be
/// added here (the test below fails until it is).
const SIDECAR_PACKAGES: &[(&str, &str)] = &[("vox", "vox-cli"), ("vox-ml-cli", "vox-ml-cli")];

/// Argv (after `cargo`) that builds the release binary for sidecar `bin`.
pub fn sidecar_build_args(bin: &str) -> Result<Vec<String>, String> {
    let (_, pkg) = SIDECAR_PACKAGES
        .iter()
        .find(|(b, _)| *b == bin)
        .ok_or_else(|| {
            format!("no owning package known for sidecar `{bin}`; add it to SIDECAR_PACKAGES in crates/vox-gui/build_sidecar.rs")
        })?;
    Ok(["build", "-p", pkg, "--release", "--bin", bin]
        .map(String::from)
        .to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_sidecar_builds_from_its_owning_package() {
        assert_eq!(
            sidecar_build_args("vox").unwrap(),
            ["build", "-p", "vox-cli", "--release", "--bin", "vox"]
        );
        // Regression: this was `-p vox-cli`, which has no `vox-ml-cli` bin.
        assert_eq!(
            sidecar_build_args("vox-ml-cli").unwrap(),
            [
                "build",
                "-p",
                "vox-ml-cli",
                "--release",
                "--bin",
                "vox-ml-cli"
            ]
        );
        assert!(sidecar_build_args("not-a-sidecar").is_err());
    }

    #[test]
    fn every_external_bin_in_tauri_conf_has_an_owning_package() {
        let conf = include_str!("tauri.conf.json");
        let bins: Vec<&str> = conf
            .split('"')
            .filter_map(|s| s.strip_prefix("../../target/release/"))
            .collect();
        assert!(
            !bins.is_empty(),
            "no externalBin entries found in tauri.conf.json"
        );
        for bin in bins {
            assert!(
                sidecar_build_args(bin).is_ok(),
                "sidecar `{bin}` has no owning package"
            );
        }
    }
}
