use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::cargo_bin;

fn resolve_or_build_vox_bin(root: &Path) -> Result<PathBuf> {
    if let Some(p) = std::env::var_os("VOX_BIN") {
        let pb = PathBuf::from(p);
        if pb.is_file() {
            return Ok(pb);
        }
    }
    let exe_name = if cfg!(windows) { "vox.exe" } else { "vox" };
    let debug_bin = root.join("target/debug").join(exe_name);
    if debug_bin.is_file() {
        return Ok(debug_bin);
    }
    let release_bin = root.join("target/release").join(exe_name);
    if release_bin.is_file() {
        return Ok(release_bin);
    }
    let status = Command::new(cargo_bin())
        .current_dir(root)
        .args(["build", "-p", "vox-cli"])
        .status()?;
    if !status.success() {
        return Err(anyhow!("cargo build -p vox-cli failed"));
    }
    if debug_bin.is_file() {
        Ok(debug_bin)
    } else {
        Err(anyhow!("vox binary not found at {}", debug_bin.display()))
    }
}

pub fn run(root: &Path) -> Result<()> {
    println!("Running determinism audit on examples/golden...");
    let vox_bin = resolve_or_build_vox_bin(root)?;
    let golden_dir = root.join("examples/golden");

    if !golden_dir.is_dir() {
        return Err(anyhow!("examples/golden directory not found"));
    }

    let mut entries: Vec<_> = fs::read_dir(golden_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "vox"))
        .collect();
    entries.sort_by_key(|e| e.path());

    let temp_dir1 = root.join("target/determinism-audit-1");
    let temp_dir2 = root.join("target/determinism-audit-2");

    if temp_dir1.exists() {
        fs::remove_dir_all(&temp_dir1)?;
    }
    if temp_dir2.exists() {
        fs::remove_dir_all(&temp_dir2)?;
    }
    fs::create_dir_all(&temp_dir1)?;
    fs::create_dir_all(&temp_dir2)?;

    for entry in entries {
        let path = entry.path();
        let stem = path.file_stem().unwrap().to_str().unwrap();
        println!("  Checking {}...", stem);

        // Each example gets its own subdirectory per run — `vox build` writes a
        // fixed bundle of files (types.ts, schemas.ts, etc., not a single
        // <stem>.ts), so sharing one flat directory across every golden example
        // would let later examples silently overwrite earlier ones' output.
        let dir1 = temp_dir1.join(stem);
        let dir2 = temp_dir2.join(stem);
        fs::create_dir_all(&dir1)?;
        fs::create_dir_all(&dir2)?;

        // Run build 1 directly against the compiled `vox` binary (avoids 140 `cargo run` lock rounds)
        let status1 = Command::new(&vox_bin)
            .current_dir(root)
            .args([
                "build",
                path.to_str().unwrap(),
                "-o",
                dir1.to_str().unwrap(),
            ])
            .status()?;
        if !status1.success() {
            return Err(anyhow!("Build 1 failed for {}", stem));
        }

        // Run build 2
        let status2 = Command::new(&vox_bin)
            .current_dir(root)
            .args([
                "build",
                path.to_str().unwrap(),
                "-o",
                dir2.to_str().unwrap(),
            ])
            .status()?;
        if !status2.success() {
            return Err(anyhow!("Build 2 failed for {}", stem));
        }

        // Compare every file written to dir1 against its counterpart in dir2.
        let mut files1: Vec<_> = fs::read_dir(&dir1)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();
        files1.sort();
        if files1.is_empty() {
            return Err(anyhow!("Build produced no output files for {}", stem));
        }
        for f1 in &files1 {
            let fname = f1.file_name().unwrap();
            let f2 = dir2.join(fname);
            let content1 = fs::read(f1)?;
            let content2 =
                fs::read(&f2).map_err(|e| anyhow!("second run missing {}: {e}", f2.display()))?;
            if content1 != content2 {
                return Err(anyhow!(
                    "Nondeterministic output detected for {} ({}). Outputs differ between runs.",
                    stem,
                    fname.to_string_lossy()
                ));
            }
        }
    }

    println!(
        "Determinism audit passed: all golden examples produce byte-identical output across runs."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_or_build_vox_bin_prefers_existing_target_debug_binary() {
        let tmp = tempfile::tempdir().unwrap();
        let debug_dir = tmp.path().join("target/debug");
        fs::create_dir_all(&debug_dir).unwrap();
        let exe_name = if cfg!(windows) { "vox.exe" } else { "vox" };
        let fake_bin = debug_dir.join(exe_name);
        fs::write(&fake_bin, b"#!/bin/sh\n").unwrap();
        let resolved = resolve_or_build_vox_bin(tmp.path()).unwrap();
        assert_eq!(resolved, fake_bin);
    }
}
